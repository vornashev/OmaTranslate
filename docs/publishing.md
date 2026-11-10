# Publishing OmaTranslate

The permanent id is `vornashev.omatranslate`. The canonical manifest is at the repository root and loads `plugin/Translate.qml`. Keep the manifest version and Cargo package version in sync. PKGBUILD's package release distinguishes packaging-only fixes.

The public repository is `https://github.com/vornashev/OmaTranslate`. Until it exists and has a release, don't advertise download URLs.

## Repository and validation

The [Marketplace publishing guide](https://plugins.omarchy.org/publish.html) requires a public GitHub repository, a root manifest, README, license, and safe installation/removal. The [development guide](https://plugins.omarchy.org/develop.html) describes the runtime lifecycle and validation. The native Arch package is an additional dependency of this plugin.

Before submission, run:

```bash
bin/test --offline
python3 bin/package_app.py --validate
bin/package
```

The validation command stages only files shipped in the source release and runs the real `omarchy plugin validate` on that directory. This avoids passing local build caches or other unshipped files to the validator. `bin/package` produces the native package, a source archive containing the root manifest and installer, and a SHA-256 cache record. No build artifacts should be committed to the Git repository.

Test installation, selection, clipboard, screen region, close/reopen, disable/re-enable, shell restart, update and removal in a real Omarchy session. Offscreen QML tests cannot replace those checks.

## User installation

For a downloaded or cloned repository, the single entry point is:

```bash
./install.sh
```

The script resolves dependencies, uses a verified current local package or builds one, installs it with pacman, and enables the plugin. It requests privilege only for native packages; plugin integration runs as the user. A caller without a terminal is handed off to an Omarchy terminal for the password prompt. There is no unauthenticated download-and-execute bootstrap or invented release URL.

Integration persists the plugin files and bar entry before asking the shell to rescan. It tolerates transient IPC failures and attempts one stock shell restart if needed. An activation timeout retains the saved integration and returns a failure with `activation-pending` diagnostics. `./install.sh --repair` retries integration using the already installed backend, without a package build, sudo, or terminal handoff. This is not a backend update.

The installed Omarchy plugin manager clones and updates repositories; it does not run arbitrary installation hooks or compile this Rust backend. After `omarchy plugin add <public-repository-url>` without `--enable`, run:

```bash
~/.config/omarchy/plugins/vornashev.omatranslate/install.sh
```

Repeat that command after `omarchy plugin update vornashev.omatranslate`. Git-managed plugin files are retained in place; native build files go into XDG_CACHE_HOME. The regular Arch package holds the binary, offline worker, desktop entry and a fallback copy of QML. Updating the native package does not replace the Marketplace git checkout.

Removal is `./install.sh --uninstall`. Settings, history and downloaded models are retained. The previous plugin directory is backed up before removal, including a Marketplace checkout. Dependencies are not recursively removed because other apps may use them.

Submit the real public repository URL through the Marketplace's linked issue form only after the desktop checks pass. Submission and publication are separate user actions; the installer does neither.

## Releases and AUR

1. Bump the version in `Cargo.toml`, `manifest.json`, `pkgbuild/PKGBUILD` and `pkgbuild/aur/PKGBUILD` (reset `pkgrel=1`), then run `cargo update -p omatranslate --offline`.
2. Move the `CHANGELOG.md` section from "unreleased" to the date.
3. Tag and push: `git tag v0.2.0 && git push origin v0.2.0`.

The `Release` workflow checks that the versions match the tag. It then builds `pkgbuild/aur/PKGBUILD` against the tag archive in an Arch container and publishes a GitHub release with the package, `SHA256SUMS`, the AUR `PKGBUILD` with real checksums, and `.SRCINFO`.

To publish on the AUR (once, then for every release):

```bash
git clone ssh://aur@aur.archlinux.org/omatranslate.git aur-omatranslate
cd aur-omatranslate
# copy PKGBUILD, .SRCINFO and omatranslate.install from the GitHub release
git add PKGBUILD .SRCINFO omatranslate.install && git commit -m "Update to 0.2.0" && git push
```

AUR users then run `omatranslate-integrate` as themselves to add the bar button and shortcuts; the package's install script prints this.
