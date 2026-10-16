# Porting a function

How one `FFX.exe` function becomes Rust (`docs/rewrite-plan.md`). The
RNG (`ffx_battle::rng`) and the kernel lookups (`ffx_kernel::bridge`) are
the worked examples.

## 1. Understand it

- Read it in the dump (`$FFX_WORK/decomp/code/0x00NN0000.c`, search
  `// ==== 0x<addr>`) and in the machine code
  (`objdump -d --no-show-raw-insn --start-address=0x… --stop-address=0x… FFX.exe`).
  Ghidra's pseudocode is a guide, not the truth: check signedness (`sar`
  vs `shr`, `movsx` vs `movzx`), whether a "void" function returns `eax`
  from a tail call, and the calling convention (who pops, `ecx`/`edx`).
- Name what it touches: callees and globals go into `symbols/ffx.sym`
  (Fahrenheit's name when they have one), with the evidence in the
  comment. `tools/ownership.py function <name>` lists the globals and who
  else touches them.

## 2. Write the core and the bridge

In the module crate (`ffx-battle`, `ffx-kernel`, `ffx-script`, ...):

- **Core:** the logic on its own types, no game memory: pure functions
  where possible (`rng::step_global`), any target. Game data (tables,
  constants beyond a handful of literals) is an input, never a constant
  in the repo: the bridge reads it from game memory, and a host without
  `FFX.exe` running reads it from the player's `FFX.exe`.
- **Bridge:** `pub mod bridge` with one function per original, taking
  `&mut impl ffx_bridge::Mem` and the original's arguments, returning what
  it returns. Globals are `ffx_bridge::Global<T>` at their `ffx.sym`
  address (rebased for you); pointers are `u32` or `Ptr<T>`. The bridge
  reads the state, calls the core, writes the state back, byte for byte
  where the original writes.

## 3. Prove it against the original

A test in `crates/ffx-oracle/tests/<module>.rs`, two oracles started alike:
one runs the original machine code (`Oracle::call`), the other the bridge
(`Oracle` implements `Mem`); results and `a.diff(&mut b)` (every byte
written) must agree.

- Synthetic inputs on `Oracle::load` (the image alone): edge cases,
  random cases (a fixed-seed xorshift), negative and out-of-range values.
- Real state on `Oracle::from_snapshot` (`FFX_SNAPSHOT`, a snapshot of the
  running game: `cargo run -p ffx-harness --example snapshot`): the
  game's own tables and objects. Two are kept in `~/ffx-work/snapshots/`:
  `field.snap` (the first manual save, on the salvage ship deck) and
  `battle.snap` (600 frames into that save's first battle, `cdsp00_00`).
  A test that needs battle state should skip on a snapshot without it.

  ```bash
  FFX_GAME_DIR=~/steam-games/FFX-X2-HD FFX_SNAPSHOT=~/ffx-work/snapshots/battle.snap \
      cargo test --release -p ffx-oracle --test <module>
  ```
- `Oracle::stub_return(addr, value)` fixes a callee (a clock) for the test.
- Floats: the game thread runs x87 at 53-bit precision (`0x027f`); the
  snapshot oracle sets that control word. Results compare exactly unless
  a tolerance is stated and justified. `Ret::st0_raw` has `ST0`'s 80 bits
  (`Ret::st0` makes every NaN the same). The emulator differs from the
  hardware on one point: a signalling NaN loaded with `flds` stays
  signalling there, while the CPU quiets it; replacements follow the
  hardware, and their tests leave signalling NaNs out.

## 4. Install it

One line in `crates/ffx-hook/src/ported/<module>.rs`:

```rust
/// `MsGetRomItem(id, strings)`.
MsGetRomItem @ 0x0079_09f0 => extern "C" fn ms_get_rom_item(id: u32, strings: u32) -> u32 {
    ffx_kernel::bridge::rom(&mut mem(), ffx_kernel::bridge::ITEM, id, strings)
}
```

`extern "C"` is cdecl; use `"stdcall"`, `"thiscall"` (first argument
`this`) or `"fastcall"` as the original needs. The name must be the one in
`symbols/ffx.sym`; the function must be at least five bytes (the jump).
`tools/check-ported.py` checks both.

## 5. Check it in the game

- `tools/build-hook.sh` (and clippy for the 32-bit target:
  `PATH=tools/shims:$PATH XWIN_ARCH=x86,x86_64 cargo xwin clippy --release --target i686-pc-windows-msvc -p ffx-hook`).
- A replay that exercises it (`docs/harness.md`): the trace with
  replacements must equal the trace with `--hook-disable replacements`.
- `tools/progress.py` counts it as replaced.

## Choosing what to port

`tools/worklist.py --module <m> --named` lists the functions of a module
whose callees are all ported already (or library code, left original for
now): the ones that can be ported next, most-called first.
`tools/progress.py` shows where each module stands.

## What not to do

- No raw pointers in module crates: `Mem` only, so the same code runs in
  the oracle and later in the 64-bit host.
- No Fahrenheit or PhyreEngine SDK code; no game data in the repo.
- No replacement without its oracle test.
