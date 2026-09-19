# Dreamcast port: audit and plan

**Status:** planning only. Do not start Dreamcast core code until Phase 0 is done.

Gopher64 is a single-system N64 emulator. A Dreamcast core is a new machine: new CPU ISA, new bus, new GPU, a second CPU for audio, and disc media instead of cartridges. Almost none of the current `src/device/` tree is reusable. The frontend (SDL3, Slint, config, storage, input plumbing) is the only realistic shared surface, and even that is N64-shaped today.

This document is the audit of what must happen *before* a Dreamcast port, then a phased plan for the port itself.

## Verdict

Do not add SH4 / HOLLY / PowerVR / AICA code yet.

The fork is not in a state to host a second system:

1. `main` is **24 commits behind** `gopher64/gopher64` (as of 2026-09-19).
2. The June 2026 N64 hardening work (Tiers 1–8) was merged on this fork, then **lost** when `main` was reset/synced to upstream. Current `main` still panics on AF-RTC, ED save types, malformed ROMs, SC64, flash, VRU, and several RCP paths.
3. The codebase is a **monolithic N64 `Device`**. CPU, RCP, events, savestates, GUI, RA, and netplay all assume one VR4300 + RDP machine.
4. There are **zero unit tests** and CI does not run `cargo test`.

A Dreamcast core dropped into this tree would freeze the N64 backend in a panic-prone, untested, N64-only shape. Extract a frontend/backend split and recover the outstanding N64 work first.

---

## Current repo state (2026-09-19)

| Item | State |
|------|--------|
| This repo | `awest813/gopher64`, fork of `gopher64/gopher64` |
| Default branch | Matches upstream `efe78e5` (`fix some clippy warnings (#1181)`, 2026-08-06) |
| vs upstream | **0 ahead, 24 behind** |
| Dreamcast code | None |
| Unit tests | None (`#[test]` / `#[cfg(test)]` absent) |
| CI | `cargo fmt`, clippy, Slint translation check. No `cargo test` |
| Issues | Disabled on the fork. Upstream has 10 open N64 bugs |

Upstream commits this fork is missing include rumble/VRU fixes, a libdragon interrupt fix, rustls, Android AAB/SDK 37/NDK 30, High-DPI scaling, Slint update, and rcheevos v12.5.0. See `gopher64/gopher64` compare `awest813:main...gopher64:main`.

### What happened to Tiers 1–8

Between 2026-06-08 and 2026-06-10 this fork merged PRs #1–#12: ROM-load hardening, AF-RTC, ED save types, AI bitrate, panic elimination across SI/PI/PIF/RSP/SC64/flash/VRU/netplay, small64 RDRAM init, n64-systemtest ISViewer notes, and a `CONTRIBUTING.md`.

Those commits still exist on stale branches (`cursor/tier-8-compatibility-af7c` and siblings) but they are **not ancestors of current `main`**. The merge base with `origin/main` is `19dc966` (pre-June snapshot). Rebasing that stack onto today's `main` will conflict with months of upstream work. Treat it as a **source of intent**, not a merge candidate.

Do not replay the old branches blindly.

---

## Phase 0 — must complete before Dreamcast code

Work in this order. Each item is a separate PR on this fork.

### 0.1 Sync with upstream

Fast-forward (or merge) `gopher64/gopher64` `main` into this fork.

- Do this before any other code change.
- The 24 missing commits include Android, input, and renderer behavior a later frontend split will touch.
- After sync, re-run this audit’s panic list; a few items may already be gone.

### 0.2 Re-apply N64 hardening on current `main`

Re-implement the still-valid June work against the synced tree. Use `origin/cursor/tier-8-compatibility-af7c` as a reference, not as a patch.

**Still present on current `main` (highest impact first):**

| Area | File | Current behavior |
|------|------|------------------|
| ROM load | `src/device.rs` `get_rom_contents` / `swap_rom` | `unwrap()` / `expect()` on zip, 7z, and raw files; short dumps can panic in `set_cic` |
| AF-RTC | `src/device/cart.rs` | Block 1 read/write panic; block 2 write is `TODO` then panic |
| ED save types | `src/ui/storage.rs` | Types 4 and 6 (96KB / 128KB SRAM) panic |
| AI bitrate | `src/device/ai.rs` | Hardcoded 16-bit stereo (`XXX` comment) |
| PIF | `src/device/pif.rs` | `XXX: check out of bounds accesses`; write-to-PIF-ROM panics |
| Flash | `src/device/cart/sram.rs` | Unknown command/DMA panics |
| SC64 | `src/device/cart/sc64.rs` | Unknown register / command panics |
| SI / PI | `src/device/si.rs`, `pi.rs` | Unknown DMA panics |
| RSP | `src/device/rsp_interface.rs` | `RSP DMA already full` panic |
| VRU | `src/device/controller/vru.rs` | Unknown word/command panics |
| GB cart | `src/device/controller/gbcart.rs` | Unsupported address/type panics |
| Netplay / USB | `src/ui/netplay.rs`, `usb.rs` | Lagged channel / “ROM upload not supported” panics |
| Cheats | `src/cheats.rs` | Unknown code type panics |

Reference implementations already exist on the stale tier-8 branch:

- AF-RTC block 1 storage + BCD-validated `data2time` for block 2
- Dynamic SRAM size for ED types 4 and 6
- AI duration scaled by `AI_BITRATE_REG`
- Bounds-checked PIF / SI / SP helpers
- `eprintln!` + status bits instead of `panic!` on unknown cart/flash/SC64 commands

**Also restore, as small follow-up PRs:**

- Unit tests for BCD, save types, SRAM masking, cheat parse, AI bitrate, SI DMA, PIF writes
- `cargo test` in `.github/workflows/lint.yml`
- `CONTRIBUTING.md` (layout + local check commands). Do not copy upstream’s “no AI PRs” rule into this fork unless that is the intent here
- Optional: small64 RDRAM init (`src/device/rdram_init.rs` on the stale branch) and the n64-systemtest ISViewer notes. ISViewer already exists on `main`; the stale branch mainly added bounds safety and docs

Related upstream N64 bugs that overlap this work (do not expect them to vanish from hardening alone):

- [#1199](https://github.com/gopher64/gopher64/issues/1199) Transfer Pak Pokémon RTC
- [#1161](https://github.com/gopher64/gopher64/issues/1161) Ridge Racer 64 replay lights
- [#1098](https://github.com/gopher64/gopher64/issues/1098) ISS64 penalty graphics
- [#562](https://github.com/gopher64/gopher64/issues/562) Knife Edge black screen
- [#1055](https://github.com/gopher64/gopher64/issues/1055) / [#1076](https://github.com/gopher64/gopher64/issues/1076) Android crash / black screen

Those are accuracy bugs, not panics. They are **not** blockers for a Dreamcast port. Panic-free ROM load, RTC, and save-type handling *are*.

### 0.3 Frontend / backend split

This is the real architectural prerequisite. Today every layer names N64 hardware.

**Monolith couplings to break:**

| Layer | Coupling |
|-------|----------|
| `Device` (`src/device.rs`) | One struct owns VR4300, TLB, RSP, RDP, RDRAM, MI/PI/SI/RI/VI/AI, PIF, cart, VRU, Transfer Paks |
| CPU run loop | `cpu::run` is a VR4300 interpreter; the event scheduler lives on `cpu.events` with N64-only event IDs (`EVENT_TYPE_AI` … `EVENT_TYPE_SP`) |
| Memory | 64KB-page function tables mapped to N64 physical ranges (`MM_RDRAM_DRAM`, `MM_CART_ROM`, `MM_PIF_MEM`, …) |
| Video | `ui/video.rs` builds `GFX_INFO` for parallel-rdp (RDRAM + DMEM + DPC regs) and opens an `SDL_WINDOW_VULKAN` window sized for 320×240 / 384×288 |
| Audio | N64 AI DMA → SDL3 stream |
| Input | Four N64 ports, mempak / rumble / Transfer Pak / VRU; profiles are N64 button maps |
| GUI | “Open ROM”, `N64_EXTENSIONS`, N64 cheats, VRU dialog, expansion pak, netplay cart CRC |
| Savestates | Postcard-serialize the entire `Device` |
| RA / netplay | Cart CRC, rcheevos game load, GGRS over N64 input |
| Process model | GUI spawns `gopher64-cli` with `--overclock` / `--disable-expansion-pak` / a `.z64` path |

**Target shape (keep N64 working at every step):**

```
src/
  frontend/          # SDL3 window, audio device, input, Slint, config, storage
  n64/               # today's src/device + parallel-rdp glue (moved, not rewritten)
  dc/                # empty until Phase 1
  lib.rs             # detect media → dispatch to n64::run or dc::run
```

Minimum interface the frontend should talk to, not `Device`:

- `open_media(path) -> System` (N64 cart vs DC disc vs unknown)
- `run_frame()` / cooperative `run()` that pumps SDL
- `submit_audio(i16 samples)`
- `present_frame(width, height, pixels or Vulkan image)`
- `poll_input(port) -> system-specific state`
- `save_state() / load_state()`
- `shutdown()`

Rules for this split:

- Move code first; do not “clean up” the N64 interpreter in the same PR.
- parallel-rdp stays an **N64-only** dependency. Dreamcast must not link it.
- `Device` may remain the N64 backend’s private type.
- GUI file picker grows a second filter later; do not add `.gdi`/`.cdi` until a DC core exists.
- Keep the GUI→CLI spawn model unless there is a concrete reason to change it. Add DC CLI flags beside the N64 ones, do not reuse `--disable-expansion-pak`.

Exit criterion: N64 still boots the same games, and `src/n64/` does not import Slint. A stub `src/dc/` crate/module that returns “not implemented” is enough to prove dispatch.

### 0.4 Legal, BIOS, and source policy

Decide these before writing SH4 code.

| Topic | Decision needed |
|-------|-----------------|
| BIOS | Require user-dumped `dc_boot.bin` + `dc_flash.bin`. Do not ship firmware. |
| Disc images | Support user dumps (GDI / CDI / CHD / CUE+BIN). No game files in the repo. |
| Flycast | Licensed **GPL-2.0-or-later**, so combining with gopher64’s GPLv3 is allowed. Prefer an original Rust core that uses public hardware manuals and Flycast only as a behavioral oracle. Do not vendor Flycast C++ into this tree. |
| ares | No Dreamcast core. Gopher64’s N64 CPU style (interpreter + function tables) is still a good template for SH4. |
| Hardware docs | SH7750 hardware manual, Sega Dreamcast System Architecture, Maple/GD-ROM/AICA public docs. Keep a `docs/dc-refs.md` of sources as they are used. |
| Upstream | `gopher64/gopher64` is N64-only and forbids AI-authored PRs. This port stays on the fork unless a human rewrites and upstream agrees to a multi-system scope. |

### 0.5 Host test strategy before a second core

Without this, the split and the DC core will regress N64 silently.

1. `cargo test` in CI (even a handful of N64 unit tests).
2. Keep n64-systemtest / ISViewer as the N64 bring-up ROM path (already mapped at `0x13FF0000`).
3. Before DC Phase 1, identify a **BIOS-only boot** log (SH4 reset vector → HOLLY register probes → GD-ROM IDENTIFY) as the first DC test, analogous to ISViewer.

Do not wait for a full commercial-game compatibility suite.

---

## Architecture audit: what can be reused

### Reusable (frontend)

- SDL3 window + audio stream + controller enumeration
- Slint shell (recent games, settings, input profiles) after Phase 0.3
- Config/dir layout (`config_dir` / `data_dir` / `cache_dir` / portable.txt)
- GGRS/netplay *framework* later (DC netplay is a different input size and timing model; do not enable it in v1)
- RetroAchievements client later (rcheevos supports Dreamcast; hook after games boot)
- Savestate *pattern* (serde + postcard), not the `Device` blob

### Not reusable (new cores)

| N64 (gopher64) | Dreamcast (needed) |
|----------------|--------------------|
| VR4300 MIPS III @ 93.75 MHz, 64-bit GPRs, COP0/COP1/TLB | Hitachi SH-4 (SH7750) @ 200 MHz, 16-bit ops, 32-bit GPRs, FPU, MMU, on-chip cache, delayed branches |
| RSP (MIPS + vector coprocessor) | No equivalent. Geometry is SH4 + TA |
| RDP via parallel-rdp (Vulkan, 8-bit blending, 4KB TMEM) | PowerVR2 CLX2: Tile Accelerator + ISP + TSP, 8 MB VRAM, 32×32 tiles, modifier volumes, punch-through, trilinear |
| RDRAM 4/8 MB | 16 MB system RAM + 8 MB VRAM + 2 MB AICA RAM |
| Cartridge + PIF + CIC | BIOS flash + GD-ROM (ATA-ish + Sega packet commands) |
| Audio Interface DMA | Yamaha AICA: ARM7DI + 64-channel DSP. Second CPU, not a DMA peripheral |
| SI / 4 controller ports / mempak | Maple bus, VMU (second tiny CPU + 128×64 LCD), rumble, light gun, keyboard, fishing controller |
| VI 240p/288p | 640×480 VGA / 720×480 NTSC / PAL encoder via HOLLY |

SH4 at 200 MHz interpreted in the current gopher64 style will not run full-speed. An interpreter is still the correct **first** CPU. A dynarec is Phase 5, not Phase 1.

---

## Dreamcast hardware map (for later phases)

Target the consumer NAOMI-less Dreamcast first. No arcade, no Atomiswave, no Windows CE special cases until commercial GD-ROM software boots.

```
SH-4 200 MHz ──┬── Area 0: BIOS 2 MB, flash 128–256 KB, HOLLY regs, GD-ROM, Maple, PVR regs, AICA
               ├── Area 1–3: 16 MB SDRAM (cached / uncached / store-queue mirrors)
               ├── Area 7: on-chip peripherals (TMU, SCI, INTC, CCN, UBC, …)
               └── SQ / OC index: store queues, operand-cache RAM mode

HOLLY 100 MHz
  SB   system bus, DMA, interrupts
  TA   tile accelerator (input to tile lists in VRAM)
  CORE PowerVR2 ISP/TSP → framebuffer in VRAM
  VO   video output

AICA
  ARM7DI @ ~2.8–45 MHz (typical 2.8/5.6)
  64 PCM/ADPCM channels + DSP
  2 MB sound RAM

Maple
  4 ports × 6 subdevices
  Controller, VMU, rumble, misc

GD-ROM
  1.2 MB/s class, 2048-byte Mode-1 / 2352-byte raw
  Images: GDI (preferred dump), CDI, CHD, CUE/BIN
```

Public references (use these; do not copy closed-source cores):

- Sega, *Dreamcast System Architecture*
- Hitachi, *SH7750 Hardware Manual*
- [Copetti, Dreamcast Architecture](https://www.copetti.org/writings/consoles/dreamcast/)
- Flycast `core/hw/{sh4,holly,pvr,aica,gdrom,maple}` as a **behavior reference only**

---

## Phase 1+ — Dreamcast core (only after Phase 0)

Each phase is one or more PRs. N64 must keep working.

### Phase 1 — SH4 interpreter + memory + BIOS

**Goal:** Load user BIOS, reset, execute until the boot ROM talks to HOLLY.

- SH4 register file, 16-bit decoder, delay slot, privileged vs user
- Subset of the ISA actually used by the boot ROM (integer, branch, load/store, FPU later)
- On-chip MMU/TLB can start as identity + a few boot-ROM mappings; full MMU is Phase 1b
- Area 0 map: BIOS ROM, flash, stub HOLLY registers that return documented power-on values
- Cycle counter + a generic scheduler (do not reuse `EVENT_TYPE_*` N64 IDs)
- Log unknown MMIO instead of panicking (apply the N64 lesson immediately)

**Exit:** BIOS runs, prints/logs the first GD-ROM or Maple probe, does not crash the process.

### Phase 2 — HOLLY, GD-ROM, Maple

**Goal:** BIOS reaches the disc menu or a homebrew IP.BIN.

- HOLLY interrupt controller + DMA (ch2 TA, GD-ROM, Maple, PVR, G2/AICA)
- GD-ROM: TOC, READ, GET SC, DMA into SH4 RAM. Start with GDI
- Maple: one controller, no VMU yet
- Flash partition read/write (region, language, timezone)

**Exit:** A known homebrew (KallistiOS “Hello”) or the official BIOS animation/menu with a mounted empty/homebrew image.

### Phase 3 — Tile Accelerator + software PVR

**Goal:** Visible 3D from a simple homebrew, then a first commercial title in software.

- TA input FIFO → display lists in VRAM
- ISP hidden-surface removal per 32×32 tile (software)
- TSP: opaque, punch-through, translucent; nearest/bilinear; ignore fancy PVR features at first
- Present via SDL texture (not Vulkan, not parallel-rdp)
- Framebuffer readback for RA later

**Exit:** A 3D homebrew cube / proto, then one commercial title that uses a small TA subset (candidates: *ChuChu Rocket!*, *Sonic Adventure* is too large — pick from “2D + simple 3D” first).

Do not start a Vulkan PVR until software can render something. Software is the oracle.

### Phase 4 — AICA + ARM7

**Goal:** Sound in the games that already display.

- ARM7DI interpreter (tiny vs SH4)
- AICA MMIO, 64 channels, basic ADPCM/PCM, no DSP effects at first
- G2 DMA into sound RAM
- Sample callback into the existing SDL audio stream

**Exit:** BIOS chime + one in-game BGM without hanging the SH4.

### Phase 5 — Performance and renderer

**Goal:** Playable frame rates on desktop.

- SH4 dynarec (x86_64 first, then aarch64). Interpreter remains the fallback and the test oracle
- Vulkan or wgpu TA/CORE renderer, **separate** from parallel-rdp
- Accurate-enough TMU / TA list timing so games stop flickering

Android is out of scope until desktop is playable. Vulkan-on-Android for two GPUs (RDP + PVR) is a later project.

### Phase 6 — Peripherals, then polish

Order:

1. VMU save + LCD
2. Rumble
3. VGA vs NTSC/PAL output flags
4. More disc formats (CDI, CHD)
5. Savestates for `dc::Device`
6. RetroAchievements
7. Netplay (hard; DC idle-loop and vsync models differ)
8. Naomi / Atomiswave: **not in the first year of this plan**

---

## Suggested PR sequence

Do not collapse these.

| # | PR | Depends on |
|---|----|------------|
| 1 | Sync fork with `gopher64/gopher64` | — |
| 2 | ROM-load + AF-RTC + ED save types 4/6: no panics | 1 |
| 3 | AI bitrate + PIF/SI/PI/RSP bounds | 1 |
| 4 | Remaining device panics → logs (flash, SC64, VRU, GB cart, netplay lag) | 1 |
| 5 | Unit tests + `cargo test` in CI | 2–4 |
| 6 | Frontend/backend split: move `src/device` → `src/n64`, keep behavior | 1 |
| 7 | Media dispatch + stub `src/dc` (“Dreamcast not implemented”) | 6 |
| 8 | SH4 interpreter + BIOS map (Phase 1) | 7 + legal/BIOS policy |
| 9 | HOLLY stubs + GD-ROM GDI + Maple pad (Phase 2) | 8 |
| 10 | Software PVR (Phase 3) | 9 |
| 11 | AICA/ARM7 (Phase 4) | 9 |

PRs 2–5 are the recovered N64 plan. They are the “what needs to be done before” answer. PRs 6–7 are the architectural gate. PRs 8+ are the actual port.

---

## Risks

- **Scope:** A playable Dreamcast core is a new emulator that happens to share a GUI. Budget it that way.
- **SH4 speed:** An interpreter will not run *Shenmue* at full speed. That is acceptable until Phase 5.
- **PVR:** Tile-based deferred rendering is the hard part. Software first, or the Vulkan path will be undebuggable.
- **AICA:** Games that poll the ARM7 can deadlock a wrong ARM/SH4 interleave. Schedule both CPUs from day one of Phase 4.
- **Windows CE games:** Different MMU/boot path. Ignore until MIL-CD / GD-ROM native software is solid.
- **Fork drift:** If this fork stays 24 commits behind, every later PR conflicts. Sync (PR 1) is not optional.
- **Stale tier branches:** Useful as a checklist. Dangerous as a merge.

---

## Out of scope (explicit)

- Running gopher64 *on* Dreamcast hardware
- Replacing the N64 core with Flycast, or embedding Flycast
- Naomi / Atomiswave / Triforce
- Dreamcast netplay in the first playable milestone
- Upstreaming this to `gopher64/gopher64` without the maintainer’s agreement

---

## How to use this document

1. Land PR 1 (sync).
2. Re-scan panics; update the table in §0.2 if upstream already fixed something.
3. Land PRs 2–5 until `cargo test` is in CI and opening a truncated ROM / AF-RTC game does not abort.
4. Land PRs 6–7. Stop and re-audit if N64 regressions appear.
5. Only then open Phase 1 SH4 work, with a BIOS dump in the developer’s local `data_dir` (never committed).
