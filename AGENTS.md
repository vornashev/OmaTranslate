# Notes for AI agents

OmaTranslate is an Omarchy 4 plugin: QML in `plugin/`, a Rust backend in `src/`, a Python installer in `bin/`, the offline worker in `worker/`. See `CONTRIBUTING.md` for tests and `docs/architecture.md` for the IPC protocol.

## Finishing a task: install it so the user can check

The user tests changes in their live Omarchy session. A task isn't done until the change is installed there.

- **Changed only `plugin/` (QML, `plugin/i18n/*.json`) or `manifest.json`:** run `./install.sh --plugin`. It copies the plugin into Omarchy without a build or `sudo` and restarts the shell only if the interface changed. Run it yourself, then tell the user the task is done and what to look at.
- **Changed `src/`, `worker/`, `Cargo.*` or packaging:** the backend must be rebuilt with `./install.sh`, which asks for the `sudo` password in a terminal. Don't run it yourself; tell the user to run `./install.sh` and what to check afterwards.
- If `./install.sh --plugin` fails, read `~/.local/state/omatranslate/last-integration.json`, fix the cause and run it again. Don't report the task as done while installation fails.

Before installing, run the checks that cover the change: `python3 bin/i18n-check --strict` for strings, `python3 tests/ui_smoke.py` and `python3 tests/visual_regression.py` for QML, `cargo test` and `cargo clippy --all-targets -- -D warnings` for Rust.

## Rules

- Interface strings live in `plugin/i18n/*.json`. Add every new key to all catalogs; `en.json` is the source.
- Keep defaults language-neutral: no hard-coded language pair. The target language follows the system locale.
- The user may be editing files at the same time. Stage commits with explicit paths, never `git commit -a` or `git add -A`, and leave files you didn't change alone.
- Commit, push, tag, release or submit to the Omarchy Marketplace only when the user asks.
