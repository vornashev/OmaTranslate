# Architecture

`plugin/Translate.qml` implements the Omarchy lifecycle (`open`, `close`, `toggle`) and delegates the UI to `TranslatorView.qml`. The latter uses only Qt Quick Controls and can be exercised without Quickshell. Production uses Omarchy theme and translation singletons, the active-monitor `OverlayWindow`, and a persistent socket subscription.

Rust owns the daemon, CLI, configuration, cancellation, providers, image composition, model management and the portal SaveFile call. `worker/offline.py` is an isolated, persistent CTranslate2/SentencePiece bridge. Tesseract runs as a cancellable subprocess producing TSV. PNG previews and exports use the same Cairo rendering result.

## Lifecycle

The keep-loaded plugin starts `omatranslate serve`. CLI actions also start it if missing. A runtime lock prevents duplicate daemons. The socket is per Wayland display, inside XDG_RUNTIME_DIR, mode 0600. UI restarts reconnect and subscribe; the daemon is independent of the shell's reload lifecycle.

Every translation/capture gets a monotonically increasing job id and cancellation token. A replacement cancels the previous token. Every completion checks both token and job id before publishing. Requests are bounded to 1 MiB, original text to 100 KB, HTTP responses to 1 MiB, image decoding to 32 MP. Network concurrency is three segments and inference is serialized. A single reqwest client reuses connections; the in-memory segment cache is bounded.

## IPC v1

One UTF-8 JSON object per line. Requests include `v: 1` and `op`. Responses are `ack`, `error` or `state` events. `subscribe` returns the current snapshot and streams subsequent ones. `status` returns one snapshot. Error messages are stable codes, translated by the UI; raw service replies, input text and secrets are not logged.

| Operation | Additional fields |
|---|---|
| `open`, `settings`, `selection`, `clipboard`, `screenshot` | none |
| `image` | `path` |
| `translate` | `text` |
| `retranslate` | none |
| `editBlock` | `index`, `text` |
| `setSettings` | `settings` object containing changed fields |
| `copy` | `kind`: `text` or `image` |
| `save` | no fields opens portal; explicit `path`, optional `overwrite` for CLI |
| `downloadModel`, `removeModel` | `pair`: `en_ru` or `ru_en` |
| `cancelDownload`, `clearHistory`, `applyShortcuts`, `dismiss` | none |

Snapshots include job/status, input/translation, resolved languages, image paths and dimensions, OCR blocks, settings, model/download state and optional history. OCR blocks carry x/y/width/height in original image pixels, source, translated, confidence and overflow. QML scales coordinates with the image.

`omatranslate request '{"v":1,"op":"status"}'` exposes this protocol to scripts. `omatranslate serve --stdio` uses the same dispatcher and framing without sockets, useful for restricted test environments. It does not imply that the production socket integration has been exercised.

## Adding a provider

Add its identifier to settings validation and UI, implement construction/parsing in `providers.rs`, and cover malformed responses, limits, Unicode and cancellation. Translation accepts ordered segments and returns one result per segment. Provider URLs cannot silently redirect text to another host. LibreTranslate endpoints are user-configured; cloud endpoints are fixed HTTPS URLs.

## Model trust

Argos packages are downloaded over HTTPS from their versioned official URLs. ZIP extraction rejects path escapes, links, unexpected layouts and excessive sizes. Install uses a staging directory and rollback backup. An optional release-generated models.lock.json pins archive SHA-256. Installed files always have a local integrity manifest verified by the inference worker. Without a release lock this is trust on first download, not a pinned upstream digest.
