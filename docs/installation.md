# Installation and troubleshooting

## One-command install

From a clone of the repository on Omarchy 4:

```bash
./install.sh
```

The script:

1. installs missing dependencies (Rust, CMake and base-devel only when a build is needed);
2. builds and installs the Arch package. If a package matching the current sources already exists and its SHA-256 matches, the build is skipped;
3. adds the Tesseract data package for your system language (for example `tesseract-data-deu` under `de_DE.UTF-8`);
4. enables the plugin, adds the bar button and the shortcuts.

Use the same command to update, or to finish an interrupted install. `./bin/install` is kept as a compatible entry point.

Run it as your user, without `sudo`; the password is asked only for system packages. Started without an interactive terminal, it opens an Omarchy terminal. The first build needs internet and can take a few minutes.

Runtime dependencies: Cairo, Python 3, Tesseract with English data, wl-clipboard, wtype, grim, slurp, Quickshell and xdg-desktop-portal.

Conflicting shortcuts are never overwritten. Backups go to `~/.local/state/omatranslate/backups/`.

### Options

```bash
./install.sh --offline-engine   # also install the offline translation engine
./install.sh --rebuild          # force a package rebuild
./install.sh --plugin           # update only the plugin QML, bar button and shortcuts: no build, no sudo
./install.sh --uninstall        # remove the app and the integration
```

## How integration works

The plugin id is `vornashev.omatranslate`. The installer:

1. saves the QML;
2. adds the button to `shell.json`;
3. waits for the shell to confirm.

When it upgrades from `tema.omatranslate`, the old copy goes to the backup folder and its entry is replaced. Settings, history and models keep using the same directories. A backend already running in this Wayland session is restarted, so that it talks to the new id.

When an update changes the plugin's interface, the installer restarts the shell (`omarchy restart shell`) and the bar disappears for a couple of seconds. A loaded plugin keeps its QML types cached, and rescanning plugins isn't enough to replace them.

If writing files or checking shortcuts fails, the integration changes are rolled back. A temporary shell IPC failure does not remove the installed plugin: the installer waits for the shell and, if needed, runs `omarchy restart shell` once. If the shell is still unavailable, the command fails with status `activation-pending` and keeps the files and the bar entry for the next shell start.

### The button didn't appear

If the package is installed but the button is missing, restore only the integration, without building and without `sudo`:

```bash
./install.sh --plugin
```

`--plugin` (formerly `--repair`, which still works) copies the QML from your checkout and keeps the installed backend; the shell restarts only if the interface changed. If a change also touches the Rust backend, run `./install.sh`.

### Diagnostics

- `~/.local/state/omatranslate/last-install.json`: result of the last install.
- `~/.local/state/omatranslate/last-integration.json`: integration errors and Hyprland check results.
- `omatranslate doctor`: tools, OCR languages, offline engine and models.

Success is confirmed by `OmaTranslate installation completed successfully`, `OmaTranslate is installed` or `OmaTranslate plugin updated`. A message that a terminal was opened doesn't mean the install itself succeeded.

## Omarchy Marketplace

```bash
omarchy plugin add https://github.com/vornashev/OmaTranslate --enable
~/.config/omarchy/plugins/vornashev.omatranslate/install.sh
```

`omarchy plugin add` clones the QML plugin; it doesn't build the Rust backend, so run the installer once. Until then the panel shows the command with a copy button. The installer keeps the git copy and builds in `~/.cache/omatranslate/`, so compilation doesn't trigger shell reloads.

- Update: `omarchy plugin update vornashev.omatranslate`, then run `install.sh` again.
- Remove: `~/.config/omarchy/plugins/vornashev.omatranslate/install.sh --uninstall`. This removes the plugin directory, the bar button, the shortcuts and the package. `omarchy plugin remove` alone would leave the package and the shortcuts behind.

## Offline engine

```bash
omatranslate-setup-offline
# from sources:
bin/setup-offline
```

This creates a private environment in `$XDG_DATA_HOME/omatranslate/venv` with CTranslate2 and SentencePiece; no PyTorch and no GPU are needed. Then download models in Settings → Offline models. Restart the backend after the first engine install so its availability indicator updates.

Inference runs in a separate persistent Python process; networking, IPC, settings, the queue, OCR and image composition are handled by Rust. Cancelling an active offline job restarts the process so the next job doesn't wait for the old one.

Argos 1.9 models are downloaded from the official `argos-net.com`. Without "keep warm" they are unloaded after 10 minutes of inactivity. The Argos catalog doesn't publish SHA-256 digests. The app records the digest of the archive and of every unpacked file at install and verifies them before loading a model. This detects later corruption; **it doesn't independently verify the authenticity of the first download**. `bin/lock-models` pins archives for a release; its output is installed as `/usr/share/omatranslate/models.lock.json`.

## Uninstall

```bash
./install.sh --uninstall
```

This removes the integration and the package. Settings, history and models are kept; delete them from the directories above if you want. Without the sources: `omatranslate-integrate --remove` and `sudo pacman -R omatranslate`.
