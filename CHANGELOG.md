# Changelog

## [0.2.0] - unreleased

### Added
- 55 translation languages with offline detection (whatlang plus script rules); the language picker searches native and English names.
- "Second language": text already in your target language is translated into it. This replaces the Russian-only reverse option, and existing settings are migrated.
- The target language and OCR languages follow the system locale on first run.
- OCR language toggles for every installed Tesseract language, with a hint for the missing package.
- Interface translations: Deutsch, Español, Français, Português, Українська, 简体中文, 日本語.
- AUR recipe, GitHub CI and release workflow, `bin/i18n-check`.

### Changed
- `tesseract-data-rus` is optional; the installer adds Tesseract data for the system language.
- "Translate and replace" notifications use the interface catalogs and the shell's language.
- Updating a loaded plugin restarts the shell so the new interface is actually shown.

## [0.1.0]

First version: selection, clipboard and screen-region translation for Omarchy 4 with Google, MyMemory, LibreTranslate and offline English ↔ Russian.
