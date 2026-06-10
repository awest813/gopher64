# Third-Party Notices

This project incorporates code and libraries from third parties. The notices
below apply to the listed components.

## rasky/small64 (MIT License)

RDRAM register initialization in `src/device/rdram_init.rs` is adapted from
[rasky/small64](https://github.com/rasky/small64), specifically the compact
RDRAM init sequence in `stage0.S` (`rdram_init` / `rdram_init_values`) and
register helpers in `minidragon.h`.

Copyright (c) 2025 Giovanni Bajo

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

## Git submodules

The following submodules are included as dependencies. See each repository for
license terms:

- [Themaister/parallel-rdp-standalone](https://github.com/Themaister/parallel-rdp-standalone)
- [DLTcollab/sse2neon](https://github.com/DLTcollab/sse2neon) (MIT)
- [RetroAchievements/rcheevos](https://github.com/RetroAchievements/rcheevos) (MIT)
