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
  a tolerance is stated and justified. `Ret::st0` is `ST0` as `fst qword`
  stores it (`ffx_oracle::f80_to_f64`): a NaN keeps its sign and top 52
  fraction bits, so compare bit for bit (`ffx_oracle::same_float`,
  `Ret::st0_is(port)`), NaNs included. `Ret::st0_f32()` is `ST0` as
  `fst dword` stores it (`ffx_oracle::f80_to_f32`, one rounding from the
  80 bits), `Ret::st0_raw` the 80 bits themselves. For a port that calls
  an original float function, hand it `|o, f, args|
  o.call_f64(f, args).unwrap()` (`Oracle::call_f64`): payloads reach the
  port as they do in the game, no per-test NaN helpers. The emulator
  differs from the hardware on one point: a signalling NaN loaded with
  `flds` stays signalling there, while the CPU quiets it; replacements
  follow the hardware, and their tests leave signalling NaNs out.

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
`tools/check-ported.py` checks both, and with `FFX_GAME_DIR` set that the
shim pops what the original's `ret` pops. Sizes come from
`progress.functions()`: the dump's, except where `symbols/sizes.tsv` holds the
length measured in FFX.exe (Ghidra sizes some natives as one byte). After a
new dump, regenerate it with `tools/progress.py --measure-sizes`.

The game reaches the shim through a thunk the macro generates, which keeps
`eax`, `ecx` and `edx` for the caller except where the return type puts
the result (`eax`, and `edx` for 64 bits). FFX.exe was built with
whole-program optimisation: a caller may keep a scratch register live
across a call to a function the compiler knew leaves it alone
(`op_spline_xyz_table` returns the `eax` it had before calling
`op_spr_free`). `tools/check-clobber.py` lists those call sites (it
caches the disassembly per FFX.exe hash in `~/ffx-work/cache` and
disassembles what is missing in parallel, `--jobs N`; `--no-cache` skips
the cache; same output either way); with the
thunk they need nothing more, but a shim returning `u8`, `u16` or `bool`
must still match what the original leaves in the rest of `eax` when a
caller reads all of it.

Callers through a pointer (a vtable, a table in `.data`, a callback
stored by an immediate) are not in the dump's call graph. check-clobber
also reports a shim returning nothing whose original is stored that way
(the base relocations, `tools/refs.py`) and leaves a result in `eax`:
a value it computes at the end, or a direct callee's (followed through
`__security_check_cookie`, which keeps `eax`). `ClassDynamicGeometry_SetShader_006a4800`
returns `0x6a46e0`'s `eax` (0) to its virtual callers; a void shim lost
the searchlight beams (pc round 2). Entries of `g_MagicImports` are
decided by the magic DLLs' own call sites (`magicFiles/FFX/*.dll`,
cached in `~/ffx-work/cache/magic-*.pickle`, about a minute to build
once), particle program records by their dispatchers (which never read
`eax`). Pointer callers checked by hand to drop `eax` go into `VOID_OK`
in the script, with the dispatcher addresses as the reason.

## 5. Check it in the game

- `tools/build-hook.sh` (and clippy for the 32-bit target:
  `PATH=tools/shims:$PATH XWIN_ARCH=x86,x86_64 cargo xwin clippy --release --target i686-pc-windows-msvc -p ffx-hook`).
- A replay that exercises it (`docs/harness.md`): the trace with
  replacements must equal the trace with `--hook-disable replacements`.
- `tools/progress.py` counts it as replaced.

## Library code: PhyreEngine and Direct3D 11 (M4 pilot)

The M4 pilot (`crates/ffx-prender`, `crates/ffx-oracle/tests/phyre_render.rs`,
`~/ffx-work/m4/pilot-report.md`) ported part of `PhyreEngine`'s map render
chain with this recipe. What it adds:

- **Frames callees see.** Engine code hands the addresses of its locals to
  callees (cursors, block lists, the blend factor), and recording stubs
  log them. The test computes where the original's frame is (the call
  pushes from `Oracle::stack_range().end - 0x100`: arguments, the return
  address, `ebp`, then the `ebp` offsets from the disassembly), gives the
  port a frame at the same address in the other oracle, and moves that
  oracle's own calls below it with `Oracle::set_call_top`. Both then log
  the same pointers, and after the call the two frames can be compared
  byte for byte (what a callee only reads through a pointer). The bridge
  takes the frame as an argument, laid out at the original's `ebp`
  offsets; the shim passes a `StackFrame`.
- **COM.** `ID3D11DeviceContext` methods stay calls through the object's
  vtable (`stdcall`, `this` first on the stack). The test builds fake
  objects whose vtables point at recording stubs that log and pop the
  method's words (`com_object`).
- **Hidden `this`.** Ghidra shows some engine methods as plain functions;
  check in the disassembly what the caller leaves in `ecx`
  (`Phyre_PGeometry_PDynamicMesh_getDataBlockCount` is a thiscall on the
  dynamic mesh). A stub that does not log `ecx` cannot catch a port that
  forgets it: give stubs of library callees `this = true`.
- **One hook per address.** Where a wrapper already owns an address
  (`renderWorldCulledSorted`'s capture wrapper in `world.rs`), the wrapper
  calls the port instead of its trampoline, behind its own
  `FFX_HOOK_DISABLE` name.
- **Gate on the original code.** `FFX_HOOK_ONLY=<names>` (`ffx-dinput8`,
  passed on by `tools/run-game.sh`) installs only the named replacements,
  so a capture holds the original code plus the slice. Compare it with
  captures of the same build run `original`, not older ones: the harness
  changes what a capture sees (`orig3`/`orig4` of 2026-10-24 no longer
  match a fresh original capture of `tools/replays/draw-field.txt`).

## Choosing what to port

`tools/worklist.py --module <m> --named` lists the functions of a module
whose callees are all ported already (or library code, left original for
now): the ones that can be ported next, most-called first.
`tools/progress.py` shows where each module stands.

## Trivial functions

Getters, setters, flag stores, constants and field copies are ported by
`tools/autoport.py`, not by hand. It evaluates every unported game
function symbolically over a small exact set of integer instructions
(moves and extensions, `lea`, add/sub/and/or/xor/not/neg/inc/dec, shifts
and multiplies by constants, `cmp`/`test` + `setcc`, push/pop and the MSVC
frame, `ret`). A function qualifies only when it is straight-line code of
those alone covering its whole body, reads nothing but its arguments
(`this`, fastcall's `ecx`/`edx`), touches the stack only for its frame,
restores every callee-saved register, is at least five bytes with nothing
jumping or pointing inside it, and either defines all 32 bits of `eax`
(a `mov al, [x]` getter does not) or leaves `eax` alone. Relocations tell
image addresses from constants.

Each match becomes a row of `ffx_auto::ROWS` (`crates/ffx-auto/src/table.rs`:
the loads and stores in program order, the `eax` expression), run by the
interpreter in `ffx_auto` from a generated shim in
`crates/ffx-hook/src/ported/auto.rs`. Only functions named in
`symbols/ffx.sym` are ported; `tools/autoport.py --unnamed` lists the rest
for naming, after which a rerun picks them up. `--verify` runs
`crates/ffx-oracle/tests/auto.rs` (every row against the original on random
arguments, pointers, indices and globals, and on both snapshots; `eax` in all
32 bits and every written byte) and drops a failing row for good
(`tools/autoport-dropped.tsv`), never patching it. Do not edit the
generated files; a function a module agent ports by hand simply leaves the
table on the next run. `--rejects` shows why the other candidates were
skipped.

## What not to do

- No raw pointers in module crates: `Mem` only, so the same code runs in
  the oracle and later in the 64-bit host.
- No Fahrenheit or PhyreEngine SDK code; no game data in the repo.
- No replacement without its oracle test.
