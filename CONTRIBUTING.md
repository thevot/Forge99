# Contributing to Forge99

Thank you for your interest in contributing to **Forge99**!

## Development Setup

### Prerequisites
- **Rust 1.80+** (2021 edition)
- A GPU or driver supporting Vulkan 1.2+, DirectX 12, or Metal
- On Ubuntu/Debian Linux, install the windowing and audio development headers:
  ```bash
  sudo apt-get update && sudo apt-get install -y libasound2-dev libudev-dev libx11-xcb-dev libxkbcommon-dev libwayland-dev libgtk-3-dev pkg-config
  ```

### Building & Testing
Forge99 is organized as a 3-crate Cargo workspace (`ut99-format`, `ut99-bsp`, `ut99-editor`):

```bash
cargo check --workspace --all-targets
cargo test --workspace
cargo run --release --bin forge99
```

Note that tests referencing retail Unreal Tournament 99 packages skip automatically when no local `UnrealTournament/` installation is present, so the entire workspace test suite passes standalone.

## Pull Request Guidelines

- Keep pull requests focused on a single bug fix or feature.
- Add or update unit/integration tests for behavioral changes and verify `cargo test --workspace` passes cleanly before submitting.
- Update `CHANGELOG.md` under `## [Unreleased]` if your change affects user-visible behavior.
- Never include proprietary Unreal Tournament 99 assets (`.unr`, `.utx`, `.uax`, `.umx`, `.u`, executables, or extracted game data) in commits or test fixtures.

## Translations

See [docs/LOCALIZATION.md](docs/LOCALIZATION.md) for instructions on updating built-in dictionaries or testing custom JSON language packs.

## License

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in Forge99 by you, as defined in the Apache-2.0 license, shall be dual licensed under [MIT](LICENSE-MIT) and [Apache-2.0](LICENSE-APACHE), without any additional terms or conditions.
