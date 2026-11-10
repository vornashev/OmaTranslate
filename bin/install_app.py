#!/usr/bin/env python3
"""Install or update OmaTranslate with one command, from a checkout or Marketplace."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
import uuid

import package_app

ROOT = Path(__file__).resolve().parents[1]


def run(args, **kwargs):
    return subprocess.run([str(a) for a in args], check=True, **kwargs)


def terminal_available():
    return sys.stdin.isatty() and sys.stdout.isatty()


def report_path(run_id):
    if len(run_id) != 32 or any(c not in "0123456789abcdef" for c in run_id):
        raise RuntimeError("Invalid installation run id")
    state = Path(os.environ.get("XDG_STATE_HOME", Path.home() / ".local/state")) / "omatranslate"
    return state / "install-runs" / (run_id + ".json")


def write_report(run_id, status, error=""):
    path = report_path(run_id)
    record = {"runId": run_id, "status": status, "error": error, "pid": os.getpid(), "time": time.time()}
    for target in (path, path.parent.parent / "last-install.json"):
        target.parent.mkdir(parents=True, exist_ok=True)
        with tempfile.NamedTemporaryFile("w", dir=target.parent, delete=False) as stream:
            json.dump(record, stream, ensure_ascii=False, indent=2)
            stream.write("\n")
        os.replace(stream.name, target)


def wait_for_result(run_id, timeout=1800):
    path = report_path(run_id)
    deadline = time.monotonic() + timeout
    start_deadline = time.monotonic() + min(timeout, 10)
    while True:
        record = json.loads(path.read_text())
        if record.get("status") == "success":
            print("OmaTranslate installation completed successfully.")
            return
        if record.get("status") == "failed":
            raise RuntimeError(f"Installation in the terminal failed: {record.get('error', '')}\nResult: {path}")
        if record.get("status") == "starting" and time.monotonic() >= start_deadline:
            raise RuntimeError(f"The terminal did not start the installer. Result: {path}")
        if record.get("status") == "running":
            try:
                os.kill(record["pid"], 0)
            except ProcessLookupError as error:
                raise RuntimeError(f"The installer exited without completing. Diagnostics: {path.parent.parent}") from error
        # A launcher can return before its terminal process exits. Only the
        # child's result, written before the close prompt, proves completion.
        if time.monotonic() >= deadline:
            raise RuntimeError(f"The terminal has not reported installation completion. Result: {path}")
        time.sleep(0.2)


def ensure_terminal(arguments, run_id=None):
    if terminal_available():
        return True
    if os.environ.get("OMATRANSLATE_INSTALL_TERMINAL") or not shutil.which("omarchy-launch-terminal"):
        raise RuntimeError("Open a desktop terminal and run ./install.sh there; sudo needs an interactive password prompt.")
    print("Opening a terminal to install OmaTranslate and enter the sudo password…", flush=True)
    run_id = run_id or uuid.uuid4().hex
    write_report(run_id, "starting")
    run(["omarchy-launch-terminal", "env", "OMATRANSLATE_INSTALL_TERMINAL=1",
         "OMATRANSLATE_INSTALL_RUN=" + run_id, "bash", ROOT / "install.sh", *arguments])
    wait_for_result(run_id)
    return False


def missing_dependencies(dependencies):
    result = subprocess.run(["pacman", "-T", *dependencies], text=True, capture_output=True)
    if result.returncode not in (0, 127):
        raise RuntimeError(result.stderr.strip() or "Could not inspect installed packages")
    return result.stdout.splitlines()


def locale_ocr_package(environ=os.environ, root=ROOT):
    """Tesseract data package for the session language, e.g. tesseract-data-deu."""
    import re
    locale = next((environ[v] for v in ("LC_ALL", "LC_MESSAGES", "LANG") if environ.get(v)), "")
    locale = re.split(r"[.@]", locale)[0].replace("_", "-").lower()
    base = locale.split("-")[0]
    code = {"zh": "zh-tw" if locale in ("zh-tw", "zh-hk", "zh-mo") else "zh-cn", "nb": "no", "nn": "no"}.get(base, base)
    try:
        catalog = (root / "src/languages.rs").read_text()
    except OSError:
        return None
    for provider, ocr in re.findall(r'\("([A-Za-z-]+)", "[a-z]*", "([a-z_]+)"', catalog):
        if provider.lower() == code and ocr != "eng":
            return "tesseract-data-" + ocr
    return None


def package_exists(name):
    return subprocess.run(["pacman", "-Si", name], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode == 0


def integrate(*arguments):
    started = time.time_ns()
    try:
        run([sys.executable, ROOT / "bin/integrate.py", *arguments])
    except subprocess.CalledProcessError as error:
        report = Path(os.environ.get("XDG_STATE_HOME", Path.home() / ".local/state")) / "omatranslate/last-integration.json"
        try:
            if report.stat().st_mtime_ns >= started:
                details = json.loads(report.read_text()).get("error")
                if details:
                    raise RuntimeError(f"{details}\nIntegration diagnostics: {report}") from error
        except (OSError, ValueError):
            pass
        raise


def execute(args):
    if args.repair:
        print("Updating the plugin, bar button and shortcuts; the installed backend is kept…", flush=True)
        integrate()
        print("OmaTranslate plugin updated. Click 文A in the bar or run omatranslate open.")
        return
    if args.uninstall:
        print("Removing OmaTranslate integration and native package. Settings and models are kept.", flush=True)
        integrate("--remove")
        installed = subprocess.run(["pacman", "-Qq", "omatranslate"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        if installed.returncode == 0:
            run(["sudo", "pacman", "-R", "omatranslate"], cwd=Path.home())
        return
    cached = None if args.rebuild else package_app.cached_packages()
    requirements = package_app.dependencies()
    needed = list(requirements["depends"])
    if not cached:
        needed += ["base-devel", *requirements["makedepends"]]
    if args.offline_engine:
        needed.append("python-pip")
    ocr = locale_ocr_package()
    if ocr and package_exists(ocr):
        needed.append(ocr)
    missing = missing_dependencies(list(dict.fromkeys(needed)))
    print("Installing OmaTranslate: dependencies → native package → plugin and shortcuts.", flush=True)
    run(["sudo", "-v"])
    if missing:
        run(["sudo", "pacman", "-S", "--needed", *missing])
    packages = package_app.build(reuse=not args.rebuild)
    run(["sudo", "pacman", "-U", *packages])
    if args.offline_engine:
        run(["bash", ROOT / "bin/setup-offline"])
    integrate()
    print("OmaTranslate is installed. Click 文A in the bar or run omatranslate open.")


def main(argv=None, run_id=None):
    arguments = list(sys.argv[1:] if argv is None else argv)
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rebuild", action="store_true", help="rebuild even when a current package is available")
    parser.add_argument("--offline-engine", action="store_true", help="also install the optional CTranslate2 runtime")
    parser.add_argument("--uninstall", action="store_true", help="remove the package and integration; retain settings/models")
    parser.add_argument("--plugin", "--repair", dest="repair", action="store_true",
                        help="update only the plugin QML, bar button and shortcuts from this checkout, "
                             "using the installed backend; no build or sudo")
    args = parser.parse_args(arguments)
    if args.uninstall and (args.rebuild or args.offline_engine):
        parser.error("--uninstall cannot be combined with installation options")
    if args.repair and (args.uninstall or args.rebuild or args.offline_engine):
        parser.error("--plugin cannot be combined with package installation/removal options")
    if os.geteuid() == 0:
        raise RuntimeError("Run ./install.sh as your normal user, without sudo. It requests privileges only for packages.")
    commands = ("omarchy", "omarchy-shell", "hyprctl", "omatranslate") if args.repair else ("pacman", "makepkg", "omarchy", "omarchy-shell", "hyprctl")
    for command in commands:
        if not shutil.which(command):
            raise RuntimeError(f"Missing {command}; run ./install.sh on an Omarchy 4 / Arch desktop first.")
    if args.repair or ensure_terminal(arguments, run_id):
        if run_id:
            write_report(run_id, "running")
        execute(args)
        return True
    return False


if __name__ == "__main__":
    status = 0
    run_id = os.environ.get("OMATRANSLATE_INSTALL_RUN") or uuid.uuid4().hex
    message = ""
    completed_here = False
    try:
        completed_here = main(run_id=run_id)
    except (RuntimeError, OSError, subprocess.CalledProcessError) as error:
        message = str(error)
        print(f"OmaTranslate installation stopped: {message}", file=sys.stderr)
        print("Diagnostics were saved. Use ./install.sh --plugin if the package is already installed, or rerun ./install.sh.", file=sys.stderr)
        status = 1
    except KeyboardInterrupt:
        print("\nInstallation cancelled.", file=sys.stderr)
        status = 130
        message = "Installation cancelled"
    if completed_here or status:
        try:
            write_report(run_id, "failed" if status else "success", message)
            print(f"Installation result: {report_path(run_id).parent.parent / 'last-install.json'}")
        except (OSError, RuntimeError) as error:
            print(f"Could not save installation result: {error}", file=sys.stderr)
    if os.environ.get("OMATRANSLATE_INSTALL_TERMINAL") and terminal_available():
        try:
            input("Press Enter to close this terminal…")
        except (EOFError, KeyboardInterrupt):
            pass
    sys.exit(status)
