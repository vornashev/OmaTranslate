# Порт на Windows: заметки

Ветка `windows-port`. Это задел, а не рабочая версия: Rust-бэкенд компилируется под
`x86_64-pc-windows-gnu`, но на Windows ни разу не запускался, а окна пока нет совсем.

## Что сделано

- `src/ipc/` — транспорт IPC и блокировка от второго демона.
  - `mod.rs`: блокировка на `File::try_lock` из std (Rust ≥ 1.89), общая для обеих платформ.
  - `unix.rs`: прежнее поведение — Unix-сокет и lock-файл с правами 0600.
  - `windows.rs`: named pipe `\\.\pipe\omatranslate-<хеш пути сокета>`,
    `first_pipe_instance` (никто не займёт имя раньше демона), `reject_remote_clients`,
    повтор при `ERROR_PIPE_BUSY`.
- `config.rs`: без `XDG_RUNTIME_DIR` сокет лежит в `%TEMP%\omatranslate`; `private_dir`
  ставит 0700 только на Unix.
- `libc` подключается только на Unix. `tests/daemon.rs` помечен `#![cfg(unix)]`.

## Проверка с Linux

```sh
rustup target add x86_64-pc-windows-gnu
# системный /usr/bin/cargo не видит таргеты rustup — берём cargo из toolchain
T=~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin
PATH=$T:$PATH RUSTC=$T/rustc \
SYSTEM_DEPS_CAIRO_NO_PKG_CONFIG=1 SYSTEM_DEPS_CAIRO_LIB=cairo \
  $T/cargo check --target x86_64-pc-windows-gnu --all-targets
```

Без mingw-gcc падает сборка `aws-lc-sys` (криптография rustls). Для проверки только
нашего кода можно временно заменить в `Cargo.toml` фичу reqwest `rustls` на `native-tls`.

## Сборка на самой Windows

- Toolchain MSVC (`x86_64-pc-windows-msvc`): с ним `aws-lc-sys` собирается штатно
  (нужны Visual Studio Build Tools, CMake и, возможно, NASM).
- `cairo-rs` требует cairo для Windows (gvsbuild или vcpkg). Проще перевести рендеринг
  превью и экспорта в `src/images.rs` на `tiny-skia`.

## Чего нет: окно

Интерфейс — плагин оболочки Omarchy. `plugin/Translate.qml` использует `Quickshell`,
`Quickshell.Io` (`Socket`, `Process`), `Quickshell.Wayland` (`OverlayWindow`),
`qs.Ui` и `qs.Commons`. Под Windows этого нет.

Переиспользовать можно `TranslatorView.qml` и `SettingsView.qml`: они на чистом Qt Quick.
`tests/ui_smoke.py` уже запускает их через PySide6 с заглушками `qs.Ui` и `qs.Commons`.

Нужен хост вместо `Translate.qml`:

1. Qt-приложение (C++ с `QQmlApplicationEngine` или PySide6) с обычным `ApplicationWindow`.
2. Замены `qs.Commons` (тема, переводы из `plugin/i18n`) и `qs.Ui`.
3. Подключение к демону через `QLocalSocket`: на Windows он работает поверх named pipe.
   Имя трубы считается в `src/ipc/windows.rs::pipe_name`, хосту его нужно знать
   (например, команда `omatranslate pipe-name` или общий алгоритм).
4. Иконка в трее и глобальная горячая клавиша (`RegisterHotKey`).

## Что ещё не портировано (компилируется, но не работает)

| Сейчас | Замена под Windows |
|---|---|
| `wl-copy` / `wl-paste` (`server.rs`) | Win32 Clipboard (`arboard` / `clipboard-win`) |
| `grim` + `slurp` (скриншот области) | свой оверлей выбора области + захват GDI/DXGI |
| xdg-portal SaveFile через `zbus` (`portal.rs`) | `rfd` (нативный диалог) |
| перевод выделенного (primary selection) | нет аналога, обычно имитируют Ctrl+C |
| `omarchy-pkg-add`, `omatranslate-setup-offline` | свой установщик Tesseract и Python-движка |
| `/usr/share/omatranslate` в `config::resource_path` | путь рядом с `.exe` |
| `XDG_CONFIG_HOME` / `XDG_DATA_HOME` / `home()` | `%APPDATA%` / `%LOCALAPPDATA%` (крейт `dirs`) |
| хоткеи через Hyprland | `RegisterHotKey` |
| установка через PKGBUILD | MSI или Inno Setup |

## Безопасность named pipe

По умолчанию у трубы DACL с правом чтения для Everyone: чужой процесс может открыть её
на чтение, но не сможет отправить запрос. Для полной закрытости задать DACL «только
текущий пользователь» через `ServerOptions::create_with_security_attributes_raw`.

## Первый минимальный шаг

Окно на Windows, которое подключается к демону через pipe и переводит введённый текст
через онлайн-провайдера. Буфер обмена, скриншоты и офлайн-движок — потом.
