# n64-systemtest integration

[gopher64](https://github.com/gopher64/gopher64) supports the ISViewer debug
interface used by [lemmy-64/n64-systemtest](https://github.com/lemmy-64/n64-systemtest)
(MIT License). When the test ROM writes to `0xB3FF0014`, gopher64 prints the
512-byte buffer at `0xB3FF0020` to stdout.

## Building the test ROM

Install the [cargo-n64](https://github.com/rust-console/cargo-n64) toolchain,
then build from a checkout of n64-systemtest:

```bash
git clone https://github.com/lemmy-64/n64-systemtest.git
cd n64-systemtest
cargo build --release
```

The resulting ROM is written to `target/mips-nintendo64-none/release/n64-systemtest`.

CI builds are also published as artifacts from the upstream
[build-rom workflow](https://github.com/lemmy-64/n64-systemtest/actions).

## Running under gopher64

Use a small ROM image (development cartridge or flashcart save type) so the
ISViewer region at `0x13FF0000` is mapped instead of cart ROM:

```bash
cargo run --release -- path/to/n64-systemtest.z64
```

Test output appears on stdout via ISViewer. SC64 USB logging is also supported
when running on real SummerCart64 hardware.

## Emulator bring-up notes

From the n64-systemtest README:

- RDRAM init can be skipped when `RI_SELECT` is pre-set (gopher64 initializes
  RI/RDRAM via `src/device/rdram_init.rs`, adapted from rasky/small64).
- RSP DMA reads beyond IMEM wrap within IMEM (`mem_addr & 0xFFF`).
- RDRAM reads beyond installed RAM return zero (no mirroring).

To disable individual tests during bring-up, edit `tests/testlist.rs` in the
n64-systemtest source tree before rebuilding the ROM.
