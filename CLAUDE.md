# ffx-decomp

Rust reimplementation of Final Fantasy X (Steam HD Remaster), recovered from
`FFX.exe` with REA + Ghidra. Workflow details: `docs/workflow.md`.

## Rules

- Never commit game binaries, `.vbf` archives, extracted files or the
  decompilation dump. Tests find the game through `FFX_GAME_DIR` and skip
  when it is unset.
- Fahrenheit and VBFExtract are references for facts only. Do not copy or
  port their code; derive implementations from FFX.exe and the data. The
  license stays MIT OR Apache-2.0.
- Fahrenheit's names, symbols and findings may be used (written permission
  from peppy, their lead, 2026-10-16). `tools/fh-import.py` merges their
  names into `symbols/ffx.sym` as `fh` lines; prefer their name when naming
  something new, and cite Fahrenheit when a fact comes from them.
- Every recovered name goes into `symbols/ffx.sym` (sorted by address, with
  the evidence in the comment). REA sessions lose names on close; reapply
  them with `tools/sym2json.py` → `set_addresses_names`.
- Format docs in `docs/formats/` cite the FFX.exe functions they come from.
- A function replacement needs a parity test in `ffx-oracle` against the
  original machine code before it goes into the hook table. The recipe and
  its checks: `docs/porting.md`; the hook table: `docs/hooking.md`; where
  the project stands: `docs/rewrite-plan.md`.

## Environment

- Game: `~/steam-games/FFX-X2-HD` (`FFX_GAME_DIR`). Target hash in
  `symbols/ffx.sym`.
- Ghidra: `GHIDRA_INSTALL_DIR=~/tools/ghidra_12.1.4_PUBLIC`. Non-interactive
  shells may not have it; export it before running `rea` or `tools/ghidra/`.
- Decompilation dump: `~/ffx-work/decomp` (`tools/ghidra/dump.sh`). Grep it
  before asking REA; use REA for focused questions.
- Windows tools run from WSL with `.exe`. x64dbg is in
  `%USERPROFILE%\x64dbg`; frida-server listens on
  `127.0.0.1:27042` when started.

## Commands

```bash
cargo clippy --all-targets      # pedantic lints, keep it clean
cargo fmt
FFX_GAME_DIR=~/steam-games/FFX-X2-HD cargo test
FFX_GAME_DIR=~/steam-games/FFX-X2-HD cargo test --release -p ffx-vbf --test game_data -- --ignored
FFX_GAME_DIR=~/steam-games/FFX-X2-HD cargo test --release -p ffx-oracle   # parity with the original
tools/build-hook.sh                                                        # 32-bit dinput8.dll
tools/verify-batch.py [BASE]                                               # what a merge batch affects: tests, port checks, hook build, replays
FFX_GAME_DIR=~/steam-games/FFX-X2-HD python3 -I tools/check-ported.py      # and check-clobber, -callwidth, -thisarg, -callconv, -tailargs, -argslots (docs/porting.md)
tools/replays/run-all.sh [--extra] [--record [replay...]]                  # replays against the original's recorded runs (docs/harness.md)
tools/replays/run-all.sh --audio [--record] [replay...]                    # FMOD's mix, sample for sample
tools/scene-gate-capture.sh <replay> <run> [original|only-m4|all]          # draw-stream capture of a gate replay
tools/scene-gate.sh <scene> <reference run> <run>                          # draw-stream gate (docs/drawdiff.md, "Scene gates")
tools/game-tests.sh [--list|--all]                                         # in-game tests for what changed (docs/harness.md)
tools/run-game.sh [--no-hooks] [--lang en]                                 # start FFX.exe on Windows, no Steam needed
tools/run-game.sh --renderer                                               # with the external renderer (parked; docs/renderer.md)
tools/build-renderer.sh                                                    # 64-bit ffx-render.exe (parked)
uv run --with frida==17.23.2 tools/frida/peek.py <addr> <len> [--deref]     # read the running game's memory
target/i686-pc-windows-msvc/release/examples/hook-smoke.exe "$(wslpath -w ~/steam-games/FFX-X2-HD/FFX.exe)"
```

`FFX_HOOK_DISABLE=<names>` leaves those hooks out; `FFX_HOOK_ONLY=<names>` or
`FFX_HOOK_ONLY=@<file>` (a Windows path; names comma- or line-separated)
installs only those replacements (`docs/hooking.md`).
