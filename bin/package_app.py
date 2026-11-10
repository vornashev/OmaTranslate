#!/usr/bin/env python3
"""Build the Arch package and reuse it only while sources and artifact match."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import tarfile
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]
INPUTS = ("Cargo.toml", "Cargo.lock", "manifest.json", "install.sh", "src", "plugin", "worker",
          "bin", "pkgbuild", "docs", "tests", "README.md", "README.ru.md", "CHANGELOG.md", "LICENSE")


def source_files(root=ROOT):
    files = []
    for name in INPUTS + (("models.lock.json",) if (root / "models.lock.json").exists() else ()):
        path = root / name
        candidates = path.rglob("*") if path.is_dir() else [path]
        for item in candidates:
            if "__pycache__" in item.parts or item.suffix == ".pyc":
                continue
            if item.is_symlink():
                raise RuntimeError(f"Release sources cannot contain symlinks: {item}")
            if item.is_file():
                files.append(item)
    return sorted(files)


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def fingerprint(root=ROOT):
    value = hashlib.sha256(platform.machine().encode())
    for path in source_files(root):
        value.update(str(path.relative_to(root)).encode() + b"\0")
        value.update(str(path.stat().st_mode & 0o777).encode() + b"\0")
        value.update(digest(path).encode())
    return value.hexdigest()


def build_directory(root=ROOT):
    plugin_root = Path(os.environ.get("XDG_CONFIG_HOME", Path.home() / ".config")) / "omarchy/plugins"
    if root.resolve().is_relative_to(plugin_root.resolve()):
        # Building inside a watched plugin would trigger thousands of hot reloads.
        return Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache")) / "omatranslate/package"
    return root / "build/package"


def cached_packages(root=ROOT):
    directory = build_directory(root)
    try:
        cache = json.loads((directory / "artifact.json").read_text())
        if cache["source"] != fingerprint(root) or not cache["packages"]:
            return None
        result = []
        for item in cache["packages"]:
            if Path(item["name"]).name != item["name"]:
                return None
            path = directory / item["name"]
            if digest(path) != item["sha256"]:
                return None
            result.append(path)
        return result
    except (OSError, ValueError, KeyError, TypeError):
        return None


def record_packages(paths, source, root=ROOT):
    directory = build_directory(root)
    data = {"source": source, "packages": [{"name": p.name, "sha256": digest(p)} for p in paths]}
    (directory / "artifact.json").write_text(json.dumps(data, indent=2) + "\n")


def validate(root=ROOT):
    manifest = json.loads((root / "manifest.json").read_text())
    version = tomllib.loads((root / "Cargo.toml").read_text())["package"]["version"]
    if manifest["version"] != version:
        raise RuntimeError("Cargo.toml and manifest.json versions must match")
    # Validate only shipped files: build caches are not part of the repository payload.
    with tempfile.TemporaryDirectory(prefix="omatranslate-validate-") as temporary:
        stage = Path(temporary)
        for path in source_files(root):
            target = stage / path.relative_to(root)
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(path.read_bytes())
        subprocess.run(["omarchy", "plugin", "validate", str(stage)], check=True)


def dependencies(root=ROOT):
    result = subprocess.run(["makepkg", "--printsrcinfo"], cwd=root / "pkgbuild",
                            text=True, capture_output=True, check=True)
    values = {"depends": [], "makedepends": []}
    for line in result.stdout.splitlines():
        key, separator, value = line.strip().partition(" = ")
        if separator and key in values:
            values[key].append(value)
    if not values["depends"] or not values["makedepends"]:
        raise RuntimeError("PKGBUILD did not report its runtime/build dependencies")
    return values


def build(root=ROOT, *, reuse=False, options=()):
    cached = cached_packages(root) if reuse else None
    if cached:
        print("Using the verified package for the current sources.", flush=True)
        return cached
    validate(root)
    directory = build_directory(root)
    directory.mkdir(parents=True, exist_ok=True)
    version = tomllib.loads((root / "Cargo.toml").read_text())["package"]["version"]
    source = fingerprint(root)
    archive = directory / f"omatranslate-{version}.tar.gz"
    with tarfile.open(archive, "w:gz") as tar:
        for path in source_files(root):
            tar.add(path, arcname=f"omatranslate-{version}/{path.relative_to(root)}", recursive=False)
    recipe = (root / "pkgbuild/PKGBUILD").read_text()
    (directory / "PKGBUILD").write_text(recipe.replace("sha256sums=('SKIP')", f"sha256sums=('{digest(archive)}')"))
    env = dict(os.environ, PKGDEST=str(directory), SRCDEST=str(directory), BUILDDIR=str(directory))
    env.setdefault("CARGO_TARGET_DIR", str(directory.parent / "cargo-target"))
    # Extraction into an existing srcdir retains files removed from the archive
    # (for example the old nested manifest). Cargo's cache lives outside srcdir.
    subprocess.run(["makepkg", "-f", "--cleanbuild", *options], cwd=directory, env=env, check=True)
    listed = subprocess.run(["makepkg", "--packagelist"], cwd=directory, env=env,
                            text=True, capture_output=True, check=True)
    paths = [Path(p) for p in listed.stdout.splitlines() if p]
    if not paths or any(not p.is_file() or p.parent.resolve() != directory.resolve() for p in paths):
        raise RuntimeError("makepkg did not produce the expected package files")
    if fingerprint(root) != source:
        raise RuntimeError("Sources changed during the build; rerun the installer")
    record_packages(paths, source, root)
    return paths


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reuse", action="store_true", help="reuse a package verified against current sources")
    parser.add_argument("--validate", action="store_true", help="validate the Marketplace repository payload")
    args, options = parser.parse_known_args()
    if args.validate:
        validate()
    else:
        for path in build(reuse=args.reuse, options=options):
            print(path)


if __name__ == "__main__":
    main()
