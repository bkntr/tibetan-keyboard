# Tibetan Wylie/EWTS Keyboard

A lightweight Windows tray application for typing Unicode Tibetan with
[Extended Wylie Transliteration (EWTS)](https://en.wikipedia.org/wiki/Wylie_transliteration#Extensions).
Type familiar Latin characters in any application and the keyboard converts them to Tibetan as you type.

## Features

- Types Unicode Tibetan system-wide in Windows applications
- Converts EWTS incrementally, including stacked letters and backspace corrections
- Follows the active Windows keyboard layout, including AZERTY and QWERTZ
- Toggles on and off with a global, customizable hotkey
- Shows the current input state in the Windows system tray
- Offers both a per-user installer and a portable standalone executable
- Provides native builds for x64 and ARM64 Windows devices
- Verifies and installs architecture-matched updates from GitHub Releases

## Install

1. Open the [latest release](../../releases/latest).
2. Download the installer for your device:
   - `windows-x64-setup.exe` for most Windows PCs
   - `windows-arm64-setup.exe` for ARM-based Windows devices
3. Run the installer. It installs for the current user without administrator access.

For a portable copy, download the matching `standalone.exe` asset instead. The
`x86_64-pc-windows-msvc` build is for x64 and the
`aarch64-pc-windows-msvc` build is for ARM64. Move it to a writable location
before running it so the built-in updater can replace it.

The Tibetan OM icon appears in the Windows system tray after launch.

Windows is currently the only supported operating system.

## Quick start

The keyboard starts disabled so it does not change your typing unexpectedly.

1. Open any application where you can type.
2. Hold `Shift` and press `Space` twice to enable Tibetan input.
3. Type EWTS. For example, `sangs rgyas` becomes `སངས་རྒྱས`.
4. Use the same hotkey again to return to normal input.

You can also right-click the tray icon and select **Enable Tibetan** or **Disable Tibetan**.

| Tray icon | State |
| --- | --- |
| <img src="assets/preview/om-enabled-32.png" width="24" alt="Green Tibetan OM icon"> | Tibetan input enabled |
| <img src="assets/preview/om-disabled-32.png" width="24" alt="Gray Tibetan OM icon"> | Tibetan input disabled |

## Typing with EWTS

The keyboard accepts standard EWTS input and replaces it with Unicode Tibetan in the active application.

| Type | Result |
| --- | --- |
| `sangs rgyas` | `སངས་རྒྱས` |
| `oM` | `ཨོཾ` |
| `bka' brgyud` | `བཀའ་བརྒྱུད` |

A space ends the current syllable and produces a Tibetan tsheg (`་`). Use `x` or an underscore (`_`) when you want a regular word space instead. Latin letters that are not part of EWTS are ignored while Tibetan input is enabled.

## Tray menu

Right-click the tray icon to:

- Enable or disable Tibetan input
- Open the settings file in Notepad
- Reload settings after editing them
- Check for and install updates
- Exit the application

If the icon is not visible, check the system tray's hidden-icons menu.

## Configuration

Choose **Open settings…** from the tray menu. The application stores its settings at:

```text
%APPDATA%\Tibetan Keyboard\Tibetan EWTS Keyboard\config\settings.toml
```

The default settings are:

```toml
hotkey = "Shift+Space+Space"
enabled_on_start = false
check_for_updates_on_startup = true
```

After changing the file, save it and choose **Reload settings** from the tray menu. A changed `enabled_on_start` value takes effect the next time the application starts.

### Custom hotkeys

A hotkey must include at least one modifier and one main key. Supported modifiers are `Ctrl`, `Alt`, `Shift`, and `Win`. The main key can be:

- A letter or digit
- `F1` through `F24`
- `Space`, `Tab`, `Enter`, or `Escape`

To require repeated presses, repeat the main key in the setting. For example:

```toml
hotkey = "Ctrl+Alt+T"
# or
hotkey = "Shift+Space+Space"
```

For a repeated hotkey, keep the modifiers held while pressing and releasing the main key the required number of times.

## Updates

The app checks the latest GitHub Release in the background after startup when
`check_for_updates_on_startup` is enabled. It asks before downloading or
installing anything and asks again before restarting. Startup network errors are
silent; choose **Check for updates...** in the tray menu to run a visible check.

Updates only select the standalone release asset for the app's current Rust
target, so x64 and ARM64 builds cannot be mixed. Downloads are performed over
HTTPS and are checked against the SHA-256 digest published by GitHub when that
digest is available.

## Development

### Prerequisites

- Windows
- A current stable [Rust toolchain](https://www.rust-lang.org/tools/install) with Cargo
- [Inno Setup 6](https://jrsoftware.org/isdl.php) to create installers locally

Build and run a debug version:

```powershell
cargo run
```

Create an optimized executable:

```powershell
cargo build --release
```

The release executable is written to `target\release\tibetan-ewts-keyboard.exe`.

Create both the standalone asset and installer for the native ARM64 target:

```powershell
.\scripts\package.ps1 `
  -Target aarch64-pc-windows-msvc `
  -Architecture arm64
```

Use `x86_64-pc-windows-msvc` and `x64` for the x64 packages. Outputs are
written to `dist`. Pushing a tag such as `v0.2.0` whose version exactly matches
`Cargo.toml` runs the release workflow, builds all four Windows assets, creates
`SHA256SUMS.txt`, and publishes them to a GitHub Release.

### Checks

Run the unit tests and lints:

```powershell
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
```

Run the Windows end-to-end check:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\e2e.ps1
```

The end-to-end check launches the debug keyboard, sends input through its low-level Windows keyboard hook into a WinForms text box, and verifies the resulting Tibetan Unicode. Release builds deliberately ignore injected keyboard events.

### Project layout

```text
src/config.rs       Settings and hotkey parsing
src/composition.rs  Incremental EWTS composition
src/windows_app.rs  Windows keyboard hook and tray application
src/updater.rs      GitHub Releases update client
scripts/e2e.ps1     End-to-end Windows input test
scripts/package.ps1 Standalone and installer packaging
installer/          Inno Setup definition
assets/             Tray icon sources and generated sizes
```

The EWTS conversion engine is provided by the Apache-2.0/MIT-licensed [`ewts`](https://crates.io/crates/ewts) Rust library. Its dictionaries derive from [`ewts-js`](https://github.com/rogerespel/ewts-js).

## License

This project is licensed under either the [MIT License](LICENSE-MIT) or the
[Apache License 2.0](LICENSE-APACHE), at your option. See
[third-party notices](THIRD_PARTY_NOTICES.md) for dependency attribution.
