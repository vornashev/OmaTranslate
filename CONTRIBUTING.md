# Contributing to OmaTranslate

Thanks for helping! Bug reports, interface translations, new translation services and fixes are all welcome.

## Development setup

You need Omarchy 4 (or Arch with Quickshell) and:

```bash
sudo pacman -S --needed rust cmake pkgconf cairo tesseract tesseract-data-eng pyside6
```

Build and run the backend from the checkout:

```bash
cargo build
export PATH="$PWD/target/debug:$PATH"
python3 bin/integrate.py        # install the QML from this checkout and enable the plugin
omatranslate doctor             # tools, OCR languages, models
```

After changing QML, run `python3 bin/integrate.py` again. If the interface changed, the shell restarts so it loads the new QML.

## Tests

```bash
bin/test --offline                                        # everything below plus plugin validation
cargo clippy --locked --offline --all-targets -- -D warnings
python3 bin/i18n-check                                    # interface catalogs
python3 tests/visual_regression.py                        # screenshots vs tests/visual/
```

The visual regression renders every panel state in dark and light themes. After an intended interface change, update the baselines with `python3 tests/visual_regression.py --update` and look at the new pictures before committing them.

End-to-end tests use the real Rust backend, Tesseract and the PNG renderer with a fake translation process. Real APIs, model downloads and live shortcuts need a manual check in a Wayland session.

## Translating the interface

Interface strings live in `plugin/i18n/<code>.json`, one flat JSON file per language. `en.json` is the source.

1. Copy `plugin/i18n/en.json` to `plugin/i18n/<code>.json`. Use the code from Omarchy's Setup → Language: `de`, `pt-BR`, `zh-CN`, `es-MX`… A base file such as `es.json` also covers `es-MX`.
2. Translate the values; keep the keys. Keep `%1`, `%2` placeholders and keyboard names (`Ctrl`, `Super`) as they are.
3. Keep it short: the panel is narrow. Check long strings with `python3 tests/visual_regression.py`.
4. Run `python3 bin/i18n-check`. Missing keys fall back to English, so a partial translation is fine to start with.
5. Add the language to the "Interface" line in README.md and open a pull request.

Some catalogs started as machine translations. If a phrase sounds wrong in your language, a fix is very welcome.

## Adding a translation service

Providers live in `src/providers.rs`. Requests are bounded (1 MiB responses, three concurrent segments), and errors map to stable codes (`networkError`, `rateLimited`, `providerError`) that the interface translates. Keep a new provider to that contract and add a parser test next to the existing ones. If the service has its own spelling of language codes, add it to `languages::provider_code`.

## Pull requests

- One topic per pull request; describe what changed and how you tested it.
- `bin/test --offline` and clippy must pass.
- Don't commit build output (`target/`, `build/`, packages).
- By contributing you agree that your work is released under the MIT license.
