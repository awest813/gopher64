# Native Apple Silicon macOS port: audit and plan

**Status:** planning only. Do not start a “fresh” Mac app rewrite until Phase 0 is done.

This replaces the earlier Dreamcast-port plan. Gopher64 already ships `gopher64-macos-aarch64.zip` and a Homebrew cask, but that build is a Linux-shaped process model wrapped in a sandboxed `.app`, assembled without a Mac. A Silicon port here means a **native arm64 macOS app**, not a new console core and not a Rosetta/Intel binary.

Upstream maintainer note ([issue #1163](https://github.com/gopher64/gopher64/issues/1163)): security-scoped bookmarks were never implemented because there was no Mac. The sandbox was done “blind.”

## Verdict

Keep the N64 core. Rebuild the **macOS host**: sandbox file access, process model, MoltenVK packaging, Retina, and the app bundle.

Do not:

- Add Intel (`x86_64-apple-darwin`) or a universal binary unless that is a later, explicit product choice
- Drop App Sandbox (it is a documented security goal: [wiki/Security](https://github.com/gopher64/gopher64/wiki/Security))
- Vendor a Metal RDP in the first milestones (parallel-rdp is Vulkan; MoltenVK stays)
- Start this work on a fork that is still 24 commits behind upstream

---

## Current Mac Silicon state (2026-09-19)

| Item | State |
|------|--------|
| This repo | `awest813/gopher64`, **0 ahead / 24 behind** `gopher64/gopher64` |
| CI target | `aarch64-apple-darwin` on `macos-15` only |
| CPU flags | `target-cpu=apple-m1`, C/C++ `-march=armv8.4-a` |
| Min OS | **15.0** (`Cargo.toml` + `MACOSX_DEPLOYMENT_TARGET` + Homebrew `macos >= 15`) |
| GPU | Vulkan → MoltenVK. Framework path is **hardcoded Homebrew ARM**: `/opt/homebrew/opt/molten-vk/lib/libMoltenVK.dylib` |
| SIMD | RSP uses sse2neon (`src/compat/`), not native NEON helpers |
| GUI | Slint + winit; game window is a **second** SDL3 Vulkan window |
| Process model | GUI binary spawns `Contents/MacOS/gopher64-cli` for games and input-profile setup |
| Sandbox | Production: `com.apple.security.app-sandbox` on the app **and** the CLI (`inherit`) |
| Signing | Ad-hoc on PR builds; Developer ID + notarytool + stapler on tags |
| Homebrew | Cask `gopher64`, arm64 Sequoia+, zaps `~/Library/Containers/io.github.gopher64.gopher64` |
| Info.plist | Not in the repo. `cargo-bundle` generates it. No document types, no high-res keys, no Game Mode |
| Known bug | [#1163](https://github.com/gopher64/gopher64/issues/1163) Recent ROMs + CLI denied by sandbox (`file-read-data`). Reproduced on M1 |

### What the 24 missing upstream commits change for Mac

Pull these **before** any Mac-specific work. The important ones:

- [#1215](https://github.com/gopher64/gopher64/pull/1215) High-DPI: `SDL_GetWindowSizeInPixels` in `parallel-rdp/wsi_platform.cpp`. Current `main` still uses `SDL_GetWindowSize`, which is **points**, not pixels — Retina frames are the wrong size.
- Slint update, SDL update, pause-on-minimize, Android/NDK (unrelated but keeps the tree mergeable).

This fork’s `wsi_platform.cpp` still has:

```35:44:parallel-rdp/wsi_platform.cpp
uint32_t SDL_WSIPlatform::get_surface_width() {
  int w, h;
  SDL_GetWindowSize(window, &w, &h);
  return w;
}
```

### How the current `.app` is built

From `.github/workflows/build.yml` `build-macos`:

1. `brew install molten-vk`
2. `cargo build --no-default-features` → rename to `gopher64-cli`
3. `cargo bundle --format osx` (GUI app, pulls MoltenVK from the Homebrew path)
4. Copy `gopher64-cli` next to the GUI executable
5. Codesign: MoltenVK, then CLI with `entitlements_child.plist`, then app with `entitlements_prod.plist`

Production entitlements (`data/macos/entitlements_prod.plist`): sandbox, network client/server, USB, Bluetooth, **user-selected read-write**. Missing: security-scoped bookmarks, microphone (VRU), and any folder-access story.

Child CLI only inherits the sandbox. It never saw the NSOpenPanel, so it cannot read the ROM path the GUI puts on `argv`.

`get_rom_contents` then does `std::fs::File::open(file_path).unwrap()` — that is the exact panic in #1163.

---

## Phase 0 — must complete before a fresh Mac app

Work in this order. Separate PRs.

### 0.1 Sync with upstream

Fast-forward this fork to `gopher64/gopher64` `main`.

Exit: High-DPI pixel sizing is in tree; Mac CI still builds `aarch64-apple-darwin`.

### 0.2 Policy that is already decided

Do not re-litigate these unless the user changes them:

| Policy | Decision |
|--------|----------|
| Sandbox | **Keep.** Wiki + #1163: bookmarks, not “turn it off.” |
| Arch | **Apple Silicon only.** `apple-m1` is the right baseline for M1–M4. |
| GPU | **Vulkan + bundled MoltenVK** until a later Metal RDP exists. |
| Mac-specific code | Lives in `src/ui/macos.rs`, same idea as `src/ui/android.rs` (upstream request in #1163). |
| This fork | Fine for implementation. Upstream still forbids AI-authored PRs; a human would need to re-home any patch they want on `gopher64/gopher64`. |

### 0.3 ROM load must stop panicking

Even before bookmarks: `get_rom_contents` must return `None` on `PermissionDenied` / truncated files instead of `unwrap()`. That is also N64 hardening, and it is the Mac CLI crash.

Same class of bugs: zip/7z unwraps, AF-RTC panics. Those are not Mac-only, but a “fresh” app that still aborts on a denied file is not shippable.

### 0.4 Decide the Mac process model (blocking design choice)

Today desktop GUI **always** spawns a sibling binary. Android runs the emulator **in-process**.

For a native sandboxed Mac app, spawning `gopher64-cli` is the bug.

**Recommended:** on `target_os = "macos"`, run the emulator in-process (Android pattern). One `Gopher64` executable inside the bundle. The process that showed `NSOpenPanel` / `rfd` keeps the sandbox grant for that file.

**Alternative:** keep the CLI child, pass a security-scoped bookmark (or an inherited file descriptor) across `exec`, and resolve it in the child. More moving parts; still needed if you want a double-clickable CLI tool.

Pick one in Phase 0 and do not mix them. In-process is the smaller native app.

CLI-from-Terminal (`Gopher64.app/Contents/MacOS/gopher64 /path/to.z64`) still needs either:

- a user-granted ROM **folder** bookmark, or
- a non-GUI helper that is **not** sandboxed (conflicts with the wiki), or
- documenting that Terminal CLI is unsupported in the sandboxed app and pointing Homebrew users at `open -a Gopher64 file.z64` after a folder grant.

### 0.5 What we will not wait on

These are real, but they are not gates for Phase 1:

- Remaining N64 game bugs (Ridge Racer lights, ISS64, Knife Edge)
- Lost June “Tiers 1–8” stack on stale branches — cherry-pick panic fixes as they touch Mac paths; do not rebase the whole stack
- Metal renderer
- Mac App Store

---

## Architecture audit: what is already Silicon-native vs what is a Linux wrap

### Already native enough

- arm64 codegen (`apple-m1` / armv8.4-a)
- sse2neon for RSP VU
- SDL3 Vulkan window + MoltenVK *in principle*
- Notarization pipeline on upstream tags
- Slint GUI compiles for `aarch64-apple-darwin`
- `dirs` crate → containerized `Application Support` under the sandbox (Homebrew zap path confirms this)

### Linux-shaped and wrong on Mac

| Piece | Problem |
|-------|---------|
| GUI → `gopher64-cli` | Child does not receive powerbox access. Recent ROMs, CLI, Transfer Pak GB files, cheat files, savestate paths all fail the same way |
| No bookmarks | `rfd` grant dies when the GUI process exits |
| MoltenVK path | Absolute `/opt/homebrew/...` in `cargo-bundle` metadata. CI happens to have that path; any other machine does not. No pinned MoltenVK version |
| Surface size | Window **points** vs Retina **pixels** (fixed upstream, missing here) |
| No `SDL_WINDOW_HIGH_PIXEL_DENSITY` | Window creation in `src/ui/video.rs` never asks for a Retina backing store |
| Global `SDKROOT` | `.cargo/config.toml` `[env]` sets `/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk` for **every** OS |
| `linker = rust-lld` | Applied to macOS. Mach-O / dylib / install_name work is ld64’s job; audit whether `headerpad_max_install_names` is papering over rpath issues |
| `portable.txt` | Resolves relative to `Contents/MacOS/`, inside the bundle, which the sandbox cannot use as a user data dir |
| Open Saves Folder | `open::that` on a container path; may work, but “Recent ROMs” stores raw paths with no bookmark |
| USB / SC64 | Sandbox USB entitlement ≠ IOKit device access. SummerCart64 on Mac is unproven |
| VRU | No microphone entitlement |
| Dual window stacks | Slint/winit GUI + SDL game window. Menu bar / focus / fullscreen / Spaces behave like two apps |
| Min OS 15 | Cuts off M1 machines still on 14. A native Silicon port should **choose** this, not inherit it from “whatever CI image” |

---

## Phase 1+ — native Apple Silicon app (after Phase 0)

### Phase 1 — Sandbox that actually opens games

**Goal:** Open ROM, quit, relaunch, click Recent, game starts. No kernel `deny file-read-data`.

- `src/ui/macos.rs`: create/resolve/start/stop security-scoped bookmarks (`NSURL` bookmark data, `startAccessingSecurityScopedResource`)
- Persist bookmark blobs next to config (container), not only the display path
- `rfd` / document picker: after pick, store bookmark for the file **and** optionally the parent folder
- Recent ROMs list stores `{ path, bookmark }`
- Transfer Pak GB ROM/RAM paths get the same treatment
- ROM load: `Result`/`Option`, never `unwrap` on IO

If in-process (recommended): GUI calls `device::run_game` on a worker thread/process-internal runtime instead of `Command::new(gopher64-cli)`.

If child CLI: pass bookmark bytes via a private fd or argv file in the container; child must `startAccessing` before `open`.

**Exit:** #1163 Recent ROMs flow works on a sandboxed arm64 build.

### Phase 2 — App bundle that looks like a Mac app

**Goal:** A real `Gopher64.app` for Apple Silicon, not a cargo-bundle default.

- Check `Info.plist` into the repo (or generate from a template we own):
  - `LSMinimumSystemVersion`
  - `NSHighResolutionCapable = true`
  - Document types / UTIs for `z64` / `n64` / `v64` / `zip` / `7z`
  - `CFBundleIdentifier` `io.github.gopher64.gopher64`
  - `GCSupportsGameMode` / Game Controller if applicable
- Drag-and-drop and `open -a Gopher64 rom.z64` (Apple Events) go through the same bookmark/grant path
- Stop requiring a sibling `gopher64-cli` in the bundle if Phase 0 chose in-process
- Vendor **a pinned MoltenVK** dylib into `Contents/Frameworks/` with `@rpath`; delete the Homebrew absolute path from `Cargo.toml`
- Codesign order: dylibs → helpers → app. Hardened runtime on all. Dev entitlements should not need `disable-library-validation` once MoltenVK is signed as part of the bundle
- `get_dirs()` on macOS: ignore `portable.txt` inside the bundle; use the container. Optional: user-selected data folder via bookmark

**Exit:** `codesign --verify --deep --strict` and a local run with no Homebrew MoltenVK installed.

### Phase 3 — Retina, windowing, input

**Goal:** Sharp output and one coherent window story on M-series.

- Confirm #1215 pixel size is present
- Set `SDL_WINDOW_HIGH_PIXEL_DENSITY` (and integer-scale / upscale math in **pixels**)
- Slint GUI DPI vs SDL game window DPI: test 1x / 2x / 3x (built-in vs 5K)
- Fullscreen / native vs borderless, Spaces, Stage Manager
- Controller: SDL3 on macOS is fine; verify rumble and that background joystick events still work under sandbox
- Keyboard: Game Mode / capture

**Exit:** 320×240 and 640×480 integer scales look sharp on a 2x display; input profile window is usable (upstream also made it resizable in #1215).

### Phase 4 — Packaging and distribution (this fork)

**Goal:** Repeatable arm64 artifacts.

- Pin MoltenVK version in CI (cached tarball or submodule), not `brew install` floating HEAD
- Keep `macos-15` runners for Sequoia; if min OS drops, add a compile with that `MACOSX_DEPLOYMENT_TARGET`
- This fork likely **cannot** notarize (no Apple cert secrets). Ship ad-hoc/signed-dev artifacts and document Gatekeeper. Do not pretend tag notarization works here
- Homebrew: out of scope unless this fork is what users install. Note cask currently tracks **upstream** releases

### Phase 5 — Optional later

- Lower min OS to 14 (or 13) after measuring MoltenVK + SDL3 + Slint
- Microphone entitlement + VRU
- SC64 USB: real IOKit access vs “entitlement theater”
- Metal backend for parallel-rdp (large; not a packaging task)
- Universal binary / Intel (explicit non-goal)
- Mac App Store (sandbox already on, but MAS needs privacy nutrition labels, iCloud, review)

---

## Suggested PR sequence

| # | PR | Depends on |
|---|----|------------|
| 1 | Sync fork with `gopher64/gopher64` (gets Retina pixel size) | — |
| 2 | ROM/zip/7z load returns errors instead of panicking | 1 |
| 3 | `src/ui/macos.rs` bookmarks + Recent ROMs persist grants | 1 |
| 4 | macOS in-process run (drop GUI→CLI spawn) **or** bookmark handoff to CLI | 3 |
| 5 | Vendor MoltenVK + owned Info.plist + rpath; remove Homebrew absolute framework | 1 |
| 6 | `SDL_WINDOW_HIGH_PIXEL_DENSITY` + scale math; verify against #1215 | 1, 5 |
| 7 | Document types, drag-drop, `open -a` | 3, 4 |
| 8 | CI: pin MoltenVK, `codesign --verify` on the artifact | 5 |

PRs 1–2 are “what to do before.” PRs 3–4 are the actual Silicon sandbox port. PRs 5–8 make it a native app.

---

## Risks

- **In-process emu crash kills the GUI.** Acceptable for v1; optional XPC later.
- **MoltenVK / `VK_KHR_portability_subset`.** SDL’s Vulkan extensions usually include it; verify instance creation on Apple GPU after vendoring. Do not assume the Homebrew dylib CI used is identical to what we vendor.
- **Fork has no Apple signing secrets.** Local/ad-hoc only unless the user adds certs.
- **Min OS 15** is already in Homebrew. Lowering it is a product change, not a bugfix.
- **Upstream Mac was written blind.** Test on a real M-series machine. This cloud environment is Linux; Phase 3 cannot be signed off here.
- **AI PRs vs upstream.** Keep this on the fork unless a human rewrites for `gopher64/gopher64`.

---

## Out of scope

- Dreamcast (or any second console)
- Running the emulator *on* vintage Mac hardware
- Intel Macs / Rosetta as a supported target
- Replacing parallel-rdp with a Metal RDP in the first milestones
- Disabling the sandbox to “fix” #1163

---

## How to use this document

1. Land PR 1 (sync). Confirm `wsi_platform.cpp` uses `SDL_GetWindowSizeInPixels`.
2. Land PR 2 so a sandbox deny is a UI error, not a process abort.
3. Implement `src/ui/macos.rs` bookmarks and the in-process (or bookmark-passing) run loop.
4. Only then vendor MoltenVK and replace cargo-bundle defaults.
5. Verify on a physical Apple Silicon Mac: Open ROM → quit → Recent ROMs → Retina fullscreen → controller.
