# Security

Please report vulnerabilities privately through [GitHub security advisories](https://github.com/vornashev/OmaTranslate/security/advisories/new) instead of public issues.

What's in scope: the backend's IPC socket (`$XDG_RUNTIME_DIR/omatranslate-*.sock`, mode 0600), handling of untrusted text, images and service responses, model download and integrity checks, and the installer's changes to Omarchy and Hyprland configuration.

Known limitation: the Argos catalog doesn't publish digests, so the first model download is not independently authenticated (see [docs/installation.md](docs/installation.md#offline-engine)).
