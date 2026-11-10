<div align="center">

# OmaTranslate

**Select. Translate. Keep going.**

Instant translation of selected text, the clipboard and screenshots for [Omarchy](https://omarchy.org): a native Rust backend with a QML panel that follows your desktop theme.

[![CI](https://github.com/vornashev/OmaTranslate/actions/workflows/ci.yml/badge.svg)](https://github.com/vornashev/OmaTranslate/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Omarchy 4](https://img.shields.io/badge/Omarchy-4-8fbcbb.svg)](https://omarchy.org)

**English** · [Русский](README.ru.md)

![OmaTranslate translating English into German](docs/text-panel-en.png)

</div>

## Why OmaTranslate

- **One keystroke.** Select text in any app and press <kbd>Super</kbd>+<kbd>U</kbd>. No copy, no browser tab.
- **Translate in place.** <kbd>Super</kbd>+<kbd>Ctrl</kbd>+<kbd>U</kbd> replaces the selection with its translation, right in the app you're typing in.
- **Screenshots, translated on the image.** Pick a screen region. Text is recognized locally with Tesseract and the translation is drawn over the original, cell by cell and paragraph by paragraph.
- **55 languages, detected offline.** Your two languages (say German and English) are pinned. Text already in your language goes to the other one automatically.
- **Names stay names.** CamelCase, product names and versions (`GPT-6`, `FrontierCode`, "Name 5.2") are not translated. Add your own terms to a do-not-translate list.
- **Private by default.** Images never leave your machine; online services receive only the recognized text. History is off unless you turn it on. There's also a fully offline engine.
- **Feels like Omarchy.** Theme colors and fonts, a bar button, keyboard-first, and an interface in your language.

## Install

On Omarchy 4, from a clone of this repository:

```bash
git clone https://github.com/vornashev/OmaTranslate.git
cd OmaTranslate
./install.sh
```

Run it as your user, not with `sudo`; the password is asked only for system packages. The script installs dependencies, builds and installs an Arch package, enables the plugin, adds the bar button and the shortcuts. It also adds Tesseract data for your system language. Run the same command to update.

A translate button (文A) appears on the right of the bar: left click opens the translator, right click opens settings, middle click translates the clipboard.

More options, the Omarchy Marketplace route and troubleshooting: [docs/installation.md](docs/installation.md).

## Shortcuts

| Shortcut | Action |
|---|---|
| <kbd>Super</kbd>+<kbd>U</kbd> | Translate the selected text (Wayland primary selection) |
| <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>U</kbd> | Translate the clipboard: text or image |
| <kbd>Super</kbd>+<kbd>Alt</kbd>+<kbd>U</kbd> | Select a screen region and translate it |
| <kbd>Super</kbd>+<kbd>Ctrl</kbd>+<kbd>U</kbd> | Replace the selection with its translation |
| <kbd>Ctrl</kbd>+<kbd>Enter</kbd> | Translate again |
| <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>C</kbd> | Copy the translation |
| <kbd>Esc</kbd> | Close settings or the panel |

Shortcuts can be changed in settings. Existing bindings are never overwritten.

## Languages

<img src="docs/language-picker.png" alt="Language picker with native names and search" width="360" align="right">

- **Translation:** 55 languages, from Arabic to Vietnamese. On first run the target language and OCR languages follow your system locale.
- **Detection:** offline and instant ([whatlang](https://github.com/greyblake/whatlang-rs) plus script rules). Short or ambiguous phrases resolve to one of your two languages.
- **Text on images:** any installed Tesseract language. Install more with `sudo pacman -S tesseract-data-<code>` (for example `tesseract-data-jpn`) and toggle them in Settings → Images.
- **Interface:** English, Русский, Deutsch, Español, Français, Português, Українська, 简体中文, 日本語. It follows Omarchy's Setup → Language. [Add yours](CONTRIBUTING.md#translating-the-interface): it's one JSON file.

<br clear="right">

## Translation services

| Service | Key | Notes |
|---|---|---|
| Google | none | Unofficial web endpoint; may be rate limited or change |
| [MyMemory](https://mymemory.translated.net/doc/usagelimits.php) | none | 5,000 characters/day anonymously |
| [LibreTranslate](https://docs.libretranslate.com/) | optional | Your own server; HTTPS required except on localhost |
| Offline (Argos + CTranslate2) | none | On-device, CPU only; models for English ↔ Russian today |

The service never switches on its own when one fails.

### Offline mode

```bash
omatranslate-setup-offline   # or ./install.sh --offline-engine
```

This creates a private Python environment with CTranslate2 and SentencePiece; no PyTorch and no GPU are needed. Download models in Settings → Offline models. [Details](docs/installation.md#offline-engine).

## Privacy and data

- Settings: `$XDG_CONFIG_HOME/omatranslate/settings.json`, mode 0600.
- Models and optional history (up to 100 entries; turning it off deletes it): `$XDG_DATA_HOME/omatranslate/`.
- The translation cache is in memory only. Temporary images are deleted when the panel closes; your files are never modified.
- The clipboard is read only when you ask. Nothing watches it in the background.

## Limitations

- Built for Omarchy 4 (Quickshell, Hyprland). Other desktops aren't supported yet.
- OCR targets printed text on screenshots: up to 32 MP and 250 text blocks.
- Drawing translations on images doesn't shape right-to-left scripts yet. The full translation is always shown in the text panel.
- Primary selection depends on the app. If it's empty, copy and use <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>U</kbd>.

## Contributing

Bug reports, translations and new providers are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for the development setup and tests. The architecture and IPC protocol are described in [docs/architecture.md](docs/architecture.md).

## Uninstall

```bash
./install.sh --uninstall
```

This removes the integration and the package. Settings, history and models are kept.

## License

[MIT](LICENSE)
