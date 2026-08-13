# Contributing

Thanks for helping improve Tibetan EWTS Keyboard.

## Development setup

Development and end-to-end testing currently require Windows and a current
stable Rust toolchain. Build the application with:

```powershell
cargo build
```

Before opening a pull request, run:

```powershell
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets --locked
```

For changes to keyboard input handling, also run the Windows end-to-end check:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\e2e.ps1
```

Please keep changes focused and include tests for new parsing, composition, or
hotkey behavior where practical.
