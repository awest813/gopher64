# Contributing to Gopher64

Thank you for your interest in contributing! Please open a GitHub issue or reach out on [Discord](https://discord.gg/9RGXq8W8JQ) before starting substantial work.

## Getting started

1. Clone with submodules:

   ```bash
   git clone --recursive https://github.com/gopher64/gopher64.git
   ```

   If you already cloned without `--recursive`:

   ```bash
   git submodule update --init --recursive
   ```

2. Install [Rust](https://www.rust-lang.org/tools/install) (the pinned toolchain in `rust-toolchain.toml` is used automatically).

3. On Linux, install [SDL3 build dependencies](https://wiki.libsdl.org/SDL3/README-linux#build-dependencies).

4. Build:

   ```bash
   cargo build --release
   ```

## Before submitting a PR

Run the same checks as CI:

```bash
cargo fmt --all
cargo clippy -- -Dwarnings
cargo clippy --no-default-features -- -Dwarnings
cargo test
```

If you change Slint UI strings, sync translations:

```bash
cargo install slint-tr-extractor
find -name \*.slint | sort | xargs slint-tr-extractor -o /tmp/translations.pot
msgcmp --use-untranslated /tmp/translations.pot data/translations/gopher64.pot
```

See `data/translations/README.md` for adding new locales.

## Project layout

| Path | Purpose |
|------|---------|
| `src/device/` | N64 hardware emulation (CPU, RSP, carts, controllers) |
| `src/ui/` | SDL3 rendering, audio, input, and Slint GUI |
| `parallel-rdp/` | Vulkan RDP renderer (C++ submodule) |
| `retroachievements/` | RetroAchievements integration (C submodule) |
| `data/` | Shaders, translations, cheats database, icons |

## License

By contributing, you agree that your contributions will be licensed under the GPLv3 license.
