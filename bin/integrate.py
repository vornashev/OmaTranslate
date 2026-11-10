#!/usr/bin/env python3
"""Install/remove only OmaTranslate's user-owned Omarchy integration."""
import argparse
import copy
import hashlib
from contextlib import contextmanager
import json
import os
from pathlib import Path
import re
import shlex
import shutil
import signal
import socket
import struct
import subprocess
import sys
import tempfile
import time

PLUGIN_ID = "vornashev.omatranslate"
LEGACY_ID = "tema.omatranslate"
START = "-- BEGIN OMATRANSLATE"
END = "-- END OMATRANSLATE"
DEFAULTS = dict(selectionKey="SUPER + U", clipboardKey="SUPER + SHIFT + U", screenshotKey="SUPER + ALT + U",
                replaceKey="SUPER + CTRL + U")
DIAGNOSTICS = []
PLUGIN_IDS = (PLUGIN_ID, LEGACY_ID)
BAR_SECTIONS = ("left", "centerBegin", "center", "centerEnd", "right", "tray")


def without_block(text):
    return re.sub(r"(?m)^" + START + r"\n.*?^" + END + r"\n?", "", text, flags=re.S)


def binding_id(value):
    bits = dict(SUPER=64, SHIFT=1, CTRL=4, CONTROL=4, ALT=8)
    parts = [p.strip().upper() for p in value.split("+")]
    if len(parts) < 2 or any(p not in bits for p in parts[:-1]) or not re.fullmatch(r"[A-Z0-9_:]+", parts[-1]):
        raise ValueError("Unsupported shortcut: " + value)
    return sum(set(bits[p] for p in parts[:-1])), parts[-1]


def managed_block(settings):
    lines = [START]
    for key, mode, label in [("selectionKey", "selection", "Translate selection"),
                             ("clipboardKey", "clipboard", "Translate clipboard"),
                             ("screenshotKey", "screenshot", "Translate screen region"),
                             ("replaceKey", "replace", "Translate and replace selection")]:
        value = settings.get(key, DEFAULTS[key])
        binding_id(value)
        lines.append(f'o.bind({json.dumps(value)}, {json.dumps(label)}, "omatranslate {mode}")')
    return "\n".join(lines + [END, ""])


def owned_bindings(text):
    """(modmask, key, label) of the binds in our managed block of bindings.lua.

    Hyprland's Lua config reports these as dispatcher "__lua" with a numeric arg,
    so the command alone no longer tells our binds from someone else's."""
    match = re.search(r"(?m)^" + START + r"\n(.*?)^" + END, text, flags=re.S)
    owned = set()
    for value, label in re.findall(r'^o\.bind\((".*?"), (".*?"), "omatranslate [a-z]+"\)$', match.group(1) if match else "", flags=re.M):
        try:
            owned.add((*binding_id(json.loads(value)), json.loads(label)))
        except ValueError:
            pass
    return owned


def conflicts(settings, binds, owned=frozenset()):
    wanted = [binding_id(settings.get(k, v)) for k, v in DEFAULTS.items()]
    if len(set(wanted)) != len(wanted):
        return ["OmaTranslate shortcuts must be distinct"]
    result = []
    for bind in binds:
        key = str(bind.get("key", "")).upper()
        if not key and bind.get("keycode"):
            key = "CODE:" + str(bind["keycode"])
        ours = str(bind.get("arg", "")).startswith("omatranslate ") or (bind.get("modmask"), key, bind.get("description")) in owned
        if (bind.get("modmask"), key) in wanted and not ours:
            result.append(str(bind.get("description") or bind.get("arg") or key))
    return result


def atomic(path, text):
    path.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile("w", dir=path.parent, delete=False) as f:
        f.write(text)
        name = f.name
    os.replace(name, path)


class IntegrationError(RuntimeError):
    pass


class ShellUnavailable(IntegrationError):
    pass


class ActivationPending(IntegrationError):
    """Files are installed; only the running shell has not confirmed activation."""


class CommandError(IntegrationError):
    def __init__(self, args, status, output):
        self.output = output.strip()
        super().__init__(f"{shlex.join(args)} failed (exit {status}):\n{self.output or 'No diagnostic output'}")


def run(args, *, timeout=15):
    environment = dict(os.environ)
    if args[0] == "omarchy-shell":
        environment["OMARCHY_SHELL_IPC_TIMEOUT"] = "8s"
    try:
        result = subprocess.run(args, text=True, capture_output=True, timeout=timeout, env=environment)
    except subprocess.TimeoutExpired as error:
        DIAGNOSTICS.append({"command": args, "timeoutSeconds": timeout})
        failure = ShellUnavailable if args[0] == "omarchy-shell" else IntegrationError
        raise failure(f"{shlex.join(args)} timed out after {timeout} seconds") from error
    except OSError as error:
        raise IntegrationError(f"Cannot run {shlex.join(args)}: {error}") from error
    event = {"command": args, "exitCode": result.returncode}
    # Keep errors and config diagnostics, not the full plugin catalog or clipboard data.
    if result.returncode or "configerrors" in args:
        event.update(stdout=result.stdout, stderr=result.stderr)
    DIAGNOSTICS.append(event)
    del DIAGNOSTICS[:-200]
    if result.returncode:
        error = CommandError(args, result.returncode, "\n".join(filter(None, [result.stderr, result.stdout])))
        if args[0] == "omarchy-shell" and any(text in error.output for text in
                ("omarchy-shell is not responding", "omarchy-shell is not running", "omarchy-shell is not ready")):
            raise ShellUnavailable(str(error)) from error
        raise error
    return result.stdout


def plugins():
    result = json.loads(run(["omarchy-shell", "shell", "listPlugins"]))
    if not isinstance(result, list) or any(not isinstance(p, dict) or "id" not in p for p in result):
        raise IntegrationError("Omarchy returned an invalid plugin list")
    return {p["id"]: p for p in result}


def wait_for_plugin(plugin_id, *, enabled=True, timeout=30):
    deadline = time.monotonic() + timeout
    last_error = "The plugin has not appeared in the shell registry."
    while True:
        try:
            entry = plugins().get(plugin_id)
            if (enabled and entry and entry.get("enabled") is True and "bar-widget" in entry.get("kinds", [])) or (
                    not enabled and (entry is None or not entry.get("enabled"))):
                return
        except ShellUnavailable as error:
            # Read-only calls may be retried while the shell reloads QML.
            last_error = str(error)
        if time.monotonic() >= deadline:
            raise ShellUnavailable(f"Timed out waiting for {plugin_id} to be {'enabled' if enabled else 'removed'}. {last_error}")
        time.sleep(0.5)


def qml_digest(plugin):
    """Digest of a plugin copy's QML and catalogs, or None when it is absent."""
    if not (plugin / "plugin").is_dir() or plugin.is_symlink():
        return None
    digest = hashlib.sha256()
    for path in sorted((plugin / "plugin").rglob("*")):
        if path.is_file() and not path.is_symlink():
            digest.update(str(path.relative_to(plugin)).encode() + b"\0" + path.read_bytes())
    return digest.hexdigest()


def refresh_shell(remove=False, restart=False):
    if restart:
        # A loaded plugin keeps its QML types cached (the bar widget holds them),
        # and rescanPlugins does not drop them. Only a fresh shell shows new QML.
        print("Restarting the Omarchy shell to load the updated interface. The bar will briefly disappear.", flush=True)
        try:
            run(["omarchy", "restart", "shell"], timeout=180)
        except (CommandError, IntegrationError):
            # Its readiness check can fail while the shell is in fact up; the
            # registry wait below decides.
            pass
        wait_for_plugin(PLUGIN_ID, enabled=True)
        return
    print("Waiting for the Omarchy shell to load the saved configuration…", flush=True)
    try:
        run(["omarchy-shell", "shell", "rescanPlugins"])
    except ShellUnavailable:
        # The request may have run already. Do not resubmit a mutating call.
        pass
    try:
        wait_for_plugin(PLUGIN_ID, enabled=not remove)
    except ShellUnavailable:
        print("The shell is still unavailable; restarting it once. The bar will briefly disappear.", flush=True)
        # The stock command preserves the session's environment and refuses an
        # unsafe restart while locked. Its own readiness wait can take a minute.
        run(["omarchy", "restart", "shell"], timeout=180)
        wait_for_plugin(PLUGIN_ID, enabled=not remove)


def configured_shell(config, remove=False):
    """Change only our entries, preserving widget placement and user settings."""
    result = copy.deepcopy(config)
    if not isinstance(result, dict) or result.get("version") != 1:
        raise IntegrationError("shell.json must be an object with version: 1; it was not changed")
    bar = result.setdefault("bar", {})
    if not isinstance(bar, dict) or not isinstance(bar.setdefault("layout", {}), dict):
        raise IntegrationError("Invalid bar layout in shell.json; it was not changed")
    layout = bar["layout"]
    for section in BAR_SECTIONS:
        if section in layout and not isinstance(layout[section], list):
            raise IntegrationError(f"Invalid bar section {section} in shell.json; it was not changed")
    for key in ("plugins", "disabledPlugins"):
        if key in result and not isinstance(result[key], list):
            raise IntegrationError(f"Invalid {key} in shell.json; it was not changed")

    def own(entry):
        return isinstance(entry, dict) and entry.get("id") in PLUGIN_IDS

    # Prefer the current id's placement, otherwise migrate the legacy widget.
    chosen = next(((section, index) for plugin_id in PLUGIN_IDS for section in BAR_SECTIONS
                   for index, entry in enumerate(layout.get(section, []))
                   if isinstance(entry, dict) and entry.get("id") == plugin_id), None)
    for section in BAR_SECTIONS:
        if section not in layout:
            continue
        entries = []
        for index, entry in enumerate(layout[section]):
            if not own(entry):
                entries.append(entry)
            elif not remove and chosen == (section, index):
                entries.append(dict(entry, id=PLUGIN_ID, enabled=True) if "enabled" in entry else dict(entry, id=PLUGIN_ID))
        layout[section] = entries
    if not remove and chosen is None:
        settings = next((entry for entry in result.get("plugins", []) if own(entry)), {})
        entry = dict(settings, id=PLUGIN_ID)
        if "enabled" in entry:
            entry["enabled"] = True
        layout.setdefault("right", []).append(entry)
    if "plugins" in result:
        result["plugins"] = [entry for entry in result["plugins"] if not own(entry)]
    if "disabledPlugins" in result:
        result["disabledPlugins"] = [entry for entry in result["disabledPlugins"] if entry not in PLUGIN_IDS]
    return result


def save_shell_config(config, backup, remove=False):
    path = config / "omarchy/shell.json"
    original = path.read_text() if path.exists() else ""
    if original.strip():
        source = json.loads(original)
    else:
        defaults = Path(os.environ.get("OMARCHY_PATH", "/usr/share/omarchy")) / "config/omarchy/shell.json"
        source = json.loads(defaults.read_text())
    updated = configured_shell(source, remove)
    if path.exists():
        atomic(backup / "shell.json", original)
    if updated != source or not original.strip():
        atomic(path, json.dumps(updated, ensure_ascii=False, indent=2) + "\n")


def checked_plugin(path, plugin_id):
    if path.is_symlink():
        raise IntegrationError(f"Refusing to replace a plugin symlink: {path}")
    if path.exists() and json.loads((path / "manifest.json").read_text()).get("id") != plugin_id:
        raise IntegrationError(f"Refusing to replace an unrelated plugin: {path}")


def source_dir():
    source = Path(__file__).resolve().parents[1]
    return source if (source / "manifest.json").is_file() else Path("/usr/share/omatranslate/plugin")


def copy_payload(source, target):
    target.mkdir()
    shutil.copy2(source / "manifest.json", target / "manifest.json", follow_symlinks=False)
    shutil.copytree(source / "plugin", target / "plugin", symlinks=True)


def validate_payload(source):
    # makepkg creates symlinks in build/. Only the manifest and runtime QML
    # enter the plugin directory; keep validation scoped to that same payload.
    # Preserve runtime symlinks in the staging copy so Omarchy can reject them.
    with tempfile.TemporaryDirectory(prefix="omatranslate-payload-") as temporary:
        stage = Path(temporary) / "plugin"
        copy_payload(source, stage)
        run(["omarchy", "plugin", "validate", str(stage)])


@contextmanager
def change_plugin(config, backup, remove=False):
    root = config / "omarchy/plugins"
    root.mkdir(parents=True, exist_ok=True)
    ids = PLUGIN_IDS
    for plugin_id in ids:
        checked_plugin(root / plugin_id, plugin_id)
    source = source_dir()
    # Marketplace updates own their git checkout. Installing the native backend
    # must not replace that checkout with the small packaged QML payload.
    in_place = source.resolve() == (root / PLUGIN_ID).resolve() or (root / PLUGIN_ID / ".git").exists()
    if in_place:
        source = root / PLUGIN_ID
    for plugin_id in ids:
        if (root / plugin_id).exists():
            if plugin_id == PLUGIN_ID and in_place and not remove:
                copy_payload(root / plugin_id, backup / plugin_id)
            else:
                shutil.copytree(root / plugin_id, backup / plugin_id)
    if not remove:
        checked_plugin(source, PLUGIN_ID)
        validate_payload(source)
    # Publish a complete directory so the watcher never sees partially copied QML.
    # Stage beside plugins/, outside its recursively watched tree. Publishing
    # and retiring complete directories are the only plugin watcher events.
    with tempfile.TemporaryDirectory(prefix=f".{PLUGIN_ID}-", dir=root.parent) as temporary:
        stage = Path(temporary)
        moved = []
        published = False
        try:
            if remove:
                for plugin_id in ids:
                    if (root / plugin_id).exists():
                        (root / plugin_id).rename(stage / plugin_id)
                        moved.append(plugin_id)
            else:
                if not in_place:
                    copy_payload(source, stage / "new")
                    if (root / PLUGIN_ID).exists():
                        (root / PLUGIN_ID).rename(stage / PLUGIN_ID)
                        moved.append(PLUGIN_ID)
                    (stage / "new").rename(root / PLUGIN_ID)
                    published = True
                if (root / LEGACY_ID).exists():
                    (root / LEGACY_ID).rename(stage / LEGACY_ID)
                    moved.append(LEGACY_ID)
            yield
        except BaseException:
            if published:
                shutil.rmtree(root / PLUGIN_ID)
            for plugin_id in moved:
                (stage / plugin_id).rename(root / plugin_id)
            raise


def config_errors():
    output = run(["hyprctl", "-j", "configerrors"])
    try:
        errors = json.loads(output)
    except ValueError as error:
        raise IntegrationError(f"Hyprland returned invalid JSON for configerrors: {output.strip()}") from error
    if not isinstance(errors, list) or any(not isinstance(item, str) for item in errors):
        raise IntegrationError(f"Hyprland returned an unexpected configerrors response: {output.strip()}")
    return [item for item in errors if item.strip()]


def update_bindings(bindings, original, updated):
    atomic(bindings, updated)
    try:
        run(["hyprctl", "reload"])
        # An empty plain-text reply is returned as "unknown request" by some
        # Hyprland revisions. JSON is always an array, including when empty.
        errors = config_errors()
        if errors:
            raise IntegrationError("Hyprland configuration errors:\n" + "\n".join(errors))
    except BaseException as error:
        atomic(bindings, original)
        try:
            run(["hyprctl", "reload"])
        except IntegrationError as rollback_error:
            raise IntegrationError(f"{error}\nBindings restored, but reload failed: {rollback_error}") from error
        raise


def stop_backend():
    """Stop only this display's service so an upgrade cannot retain the old plugin id."""
    display = re.sub(r"[^A-Za-z0-9-]", "_", os.environ.get("WAYLAND_DISPLAY", "default"))[:32]
    runtime = Path(os.environ.get("XDG_RUNTIME_DIR", f"/tmp/omatranslate-{os.geteuid()}"))
    path = Path(os.environ.get("OMATRANSLATE_SOCKET", runtime / f"omatranslate-{display}.sock"))
    if not path.exists():
        return
    with socket.socket(socket.AF_UNIX) as connection:
        connection.settimeout(2)
        try:
            connection.connect(str(path))
        except (FileNotFoundError, ConnectionRefusedError):
            return
        pid, uid, _ = struct.unpack("3i", connection.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
    if uid != os.geteuid() or pid <= 1:
        raise IntegrationError("The backend socket belongs to another user or process")
    try:
        descriptor = os.pidfd_open(pid)
    except ProcessLookupError:
        return
    try:
        process = Path(f"/proc/{pid}")
        try:
            executable = os.readlink(process / "exe").removesuffix(" (deleted)")
            arguments = (process / "cmdline").read_bytes().rstrip(b"\0").split(b"\0")
        except FileNotFoundError:
            return
        if Path(executable).name != "omatranslate" or arguments[1:] != [b"serve"]:
            raise IntegrationError("The backend socket is not served by omatranslate serve")
        # pidfd avoids signalling a reused PID if the old service exits meanwhile.
        try:
            signal.pidfd_send_signal(descriptor, signal.SIGTERM)
        except ProcessLookupError:
            pass
    finally:
        os.close(descriptor)


def main(argv=None):
    parser = argparse.ArgumentParser()
    parser.add_argument("--bindings-only", action="store_true")
    parser.add_argument("--remove", action="store_true")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args(argv)
    config = Path(os.environ.get("XDG_CONFIG_HOME", Path.home() / ".config"))
    settings_path = config / "omatranslate/settings.json"
    settings = json.loads(settings_path.read_text()) if settings_path.exists() else DEFAULTS
    bindings = config / "hypr/bindings.lua"
    original = bindings.read_text()
    cleaned = without_block(original)
    if args.remove:
        updated = cleaned
    else:
        binds = json.loads(run(["hyprctl", "binds", "-j"]))
        found = conflicts(settings, binds, owned_bindings(original))
        if found:
            raise IntegrationError("Shortcut conflict; existing bindings were preserved: " + "; ".join(found))
        updated = cleaned.rstrip() + "\n\n" + managed_block(settings)
    if args.dry_run:
        print(json.dumps({"plugin": PLUGIN_ID, "migrateFrom": LEGACY_ID, "bindings": str(bindings), "block": "remove" if args.remove else managed_block(settings)}, indent=2))
        return
    stamp = time.strftime("%Y%m%d-%H%M%S")
    backups = Path(os.environ.get("XDG_STATE_HOME", Path.home() / ".local/state")) / "omatranslate/backups"
    backups.mkdir(parents=True, exist_ok=True)
    backup = Path(tempfile.mkdtemp(prefix=stamp + "-", dir=backups))
    shutil.copy2(bindings, backup / "bindings.lua")
    if args.bindings_only and not args.remove:
        update_bindings(bindings, original, updated)
    else:
        installed = config / "omarchy/plugins" / PLUGIN_ID
        previous = qml_digest(installed)
        with change_plugin(config, backup, args.remove):
            update_bindings(bindings, original, updated)
            try:
                # Read the latest shell.json after Hyprland validation so other
                # widgets' settings changed in the meantime are preserved.
                # This atomic write is the last step of the file transaction.
                save_shell_config(config, backup, args.remove)
            except BaseException:
                atomic(bindings, original)
                run(["hyprctl", "reload"])
                raise
        print(f"Integration files {'removed' if args.remove else 'saved'}. Backup: {backup}", flush=True)
        try:
            stop_backend()
            refresh_shell(args.remove, restart=not args.remove and previous is not None
                          and previous != qml_digest(installed))
        except (IntegrationError, OSError, ValueError) as error:
            raise ActivationPending(
                f"Integration files were {'removed' if args.remove else 'installed'}, but the shell has not confirmed the change.\n"
                f"{error}\nSaved configuration was kept. Retry ./install.sh --{'uninstall' if args.remove else 'repair'}.\nBackup: {backup}"
            ) from error
    print(f"OmaTranslate integration {'removed' if args.remove else 'installed'}: {PLUGIN_ID}\nBackup: {backup}")


def cli():
    status, message, pending = 0, "", False
    try:
        main()
    except (IntegrationError, OSError, ValueError) as error:
        status, message = 1, str(error)
        pending = isinstance(error, ActivationPending)
        print(f"OmaTranslate integration failed: {message}", file=sys.stderr)
    report = Path(os.environ.get("XDG_STATE_HOME", Path.home() / ".local/state")) / "omatranslate/last-integration.json"
    try:
        atomic(report, json.dumps({"plugin": PLUGIN_ID, "status": "activation-pending" if pending else "failed" if status else "success",
                                   "error": message, "commands": DIAGNOSTICS}, ensure_ascii=False, indent=2) + "\n")
        print(f"Integration diagnostics: {report}")
    except OSError as error:
        print(f"Could not save integration diagnostics: {error}", file=sys.stderr)
    return status


if __name__ == "__main__":
    sys.exit(cli())
