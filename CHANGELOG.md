# Changelog

## [Unreleased]

### Added
- "My language" and the last language: foreign text goes into your language, and text already in your language goes to the language you worked with last (the last foreign text you translated, or a target you picked by hand). The panel marks it with a "last" badge; Settings → General can pin one language instead.

### Changed
- Picking a source or target language and the ⇄ button in the panel apply to the text at hand only. The swap button no longer switches automatic mode off for good.
- Older settings move to the new mode once: in a fixed language pair, the target becomes your language and the source the last language. Change "My language" in Settings → General if that guessed wrong.
- When language detection is unsure, Google and LibreTranslate get "auto" instead of a guessed source language.

## [0.2.0] - 2026-11-10

### Added
- 55 translation languages with offline detection (whatlang plus script rules); the language picker searches native and English names.
- "Second language": text already in your target language is translated into it. This replaces the Russian-only reverse option, and existing settings are migrated.
- The target language and OCR languages follow the system locale on first run.
- OCR language toggles for every installed Tesseract language, with a hint for the missing package.
- Interface translations: Deutsch, Español, Français, Português, Українська, 简体中文, 日本語.
- A plugin added from the Omarchy Marketplace without its backend shows the install command with a copy button instead of waiting forever.
- AUR recipe, GitHub CI and release workflow, `bin/i18n-check`; end-to-end daemon tests in Rust (`tests/daemon.rs`).

### Changed
- `tesseract-data-rus` is optional; the installer adds Tesseract data for the system language.
- "Translate and replace" notifications use the interface catalogs and the shell's language.
- Updating a loaded plugin restarts the shell so the new interface is actually shown.
- Text on images is recognised in two Tesseract passes, sparse text for interfaces and automatic layout, merged without duplicates. Labels on dark buttons and bold headings are no longer lost or split.
- Translations drawn on images match the original's font size (accounting for ascenders and descenders) and weight. Headings no longer merge with the text under them, and the background patch covers only what the translation needs.

## [0.1.0]

First version: selection, clipboard and screen-region translation for Omarchy 4 with Google, MyMemory, LibreTranslate and offline English ↔ Russian.
