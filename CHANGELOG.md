# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-08-14

### Added

- Per-user Inno Setup installers for x64 and ARM64 Windows.
- A tag-driven GitHub release workflow that publishes standalone executables,
  installers, and SHA-256 checksums for both architectures.
- Background and manual GitHub Releases update checks, architecture-specific
  in-place updates, download integrity verification, and restart prompts.

## [0.1.2] - 2026-08-14

### Fixed

- End the active EWTS composition when a mouse button is pressed, preventing
  subsequent input from deleting text after the caret is moved with the mouse.

## [0.1.1] - 2026-08-13

### Added

- Support for the active Windows keyboard layout, including AZERTY and QWERTZ.

### Fixed

- Track left and right modifier state reliably in the low-level keyboard hook.
- Treat AltGr combinations as text input while preserving regular Control,
  Alt, and Windows shortcut chords.
- Suppress key-release events for keys consumed as EWTS input.

## [0.1.0] - 2026-08-13

### Added

- System-wide conversion from incremental EWTS input to Unicode Tibetan.
- Recomposition of ambiguous stacks and source-aware backspace handling.
- Configurable global hotkeys for enabling and disabling Tibetan input.
- A system tray menu with state icons, settings access, reload, and exit actions.
- Portable x64 and ARM64 Windows builds.
- Unit, CI, and Windows end-to-end test coverage.
