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
  A stub pops like the original, logs every word the original passes, and
  writes what the callee writes through any pointer the port hands it
  (`docs/workflow.md`, "Test stubs").
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
`this`) or `"fastcall"` as the original needs, or `"fastcall-caller-pops"`
for an original that takes `ecx` and `edx` plus stack words its caller
pops (an LTCG custom convention; `docs/hooking.md`, "The hook table";
`Conv::FastcallCallerPops` in the oracle). The name must be the one in
`symbols/ffx.sym`; the function must be at least five bytes (the jump;
shorter ones go in a `host_only!` list, "Functions under five bytes" below).
`tools/check-ported.py` checks both, and with `FFX_GAME_DIR` set that the
shim pops what the original's `ret` pops. Sizes come from
`progress.functions()`: the dump's, except where `symbols/sizes.tsv` holds the
length measured in FFX.exe (Ghidra sizes some natives as one byte). After a
new dump, regenerate it with `tools/progress.py --measure-sizes`.

A cdecl `ret` says nothing about the argument count, so count the words
yourself, including the words the original only passes on. A function that
ends in `jmp Target` with its own frame popped hands `Target` its caller's
arguments: `MsMusicReadStart` (`0x0079fff0`) tail-jumps to `SndMusicRead`
with its music argument, and a port that declared no argument and called
`SndMusicRead` with none never loaded the victory fanfare (its oracle stub
logged no arguments, so the test passed). Declare every word the original
reads, pass the forwarded ones on, and make the test stubs log them.
`FFX_GAME_DIR=... python3 -I tools/check-tailargs.py` (about 10 s) checks
this statically and must report "0 problems". It measures the words each
hooked original reads (through its tail jumps, recursively; a word stored
just before the jump is the original's own; a pushed word no callee reads
does not count) and reports a shim declaring fewer, a shim ignoring one
(`_name` its body never uses), and a port call to a hooked original's
tail-jump target passing fewer words than that target reads.
`--explain NAME --code` shows the instruction behind each word; checked
exceptions go into `ALLOW` in the script with their reason.

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
- The static checks (`docs/workflow.md`, "The port checks"; `tools/verify-batch.py`
  runs all but check-argslots).
- A replay that exercises it (`docs/harness.md`): the trace with
  replacements must equal the original's recorded run
  (`tools/replays/run-all.sh`). Platform ports also pass the draw gates and
  the audio comparison where they can change them (`docs/workflow.md`,
  "Gates beyond the tests").
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
  `python3 -I tools/check-thisarg.py` (needs `FFX_GAME_DIR`, about 3 s)
  scans for this statically. It finds every original that reads `ecx`
  before writing it on some path from its entry, then every port call to
  one that passes no `ecx` (`Conv::Cdecl`/`Conv::Stdcall`,
  `ffx_bridge::Host::call`, `call_cdecl*`, `call0`, and per-file
  forwarding helpers). It must report "0 problems", like
  check-callwidth. Targets it can't resolve (pointers from memory) are
  counted, not checked.
- **Calling convention.** Call each original and import the way it
  returns: `ret n` means the callee pops `n` bytes (stdcall, thiscall,
  fastcall), so pass exactly `n`/4 stack words with that `Conv`; a plain
  `ret` means cdecl. An import thunk (`jmp [IAT]`) takes the DLL
  export's convention: Win32 APIs are stdcall (`ImmAssociateContext`
  at `0x009da254` pops 8), the CRT and steam_api are cdecl, and so are
  varargs exports (`IggyGDrawSendWarning`: the originals `add esp`
  after it). p6's createWindow called that thunk as cdecl; the argument
  words were popped twice and every replay died at startup.
  **Test stubs pop like the original**: give a stub the `pops` (words)
  the original's `ret n` pops, or for an import the API's argument
  bytes / 4 (`Conv::Stdcall` in render.rs `stub_sym`). The oracle checks
  `esp` after every call against the convention the port asked for, but
  a stub that pops nothing hides a port that calls a stdcall callee as
  cdecl. `python3 -I tools/check-callconv.py` (needs `FFX_GAME_DIR`,
  about 15 s) checks both statically: every port call it can resolve
  (directly, through forwarding helpers and through closures the shims
  hand the bridges) against the callee's `ret n` or the import's
  convention, and every test helper call with `sym` and `pops` (or
  `conv` and `n`) arguments against the original. It must report
  "0 problems"; `--self-test` checks its fixtures.
- **Real states.** `tools/harvest-render.sh` harvests the render chain's
  calls by running `graphicRender` (`0x00642bb0`) on the snapshots: every
  world of the frame, with the culling jobs run by the waiting thread.
  The game-update drivers of `tools/harvest.sh` never render.
  `tests/harvested_phyre_render.rs` checks the ports on those calls.
- **One hook per address.** Where a wrapper already owns an address
  (`renderWorldCulledSorted`'s capture wrapper in `world.rs`, the mod
  wrapper of `openFile`, the harness's clock, key and flip wrappers), the
  wrapper calls the port instead of its trampoline, behind its own
  `FFX_HOOK_DISABLE` name (`docs/hooking.md`, "Wrapper-owned
  addresses").
- **Gate on the original code.** `FFX_HOOK_ONLY=<names>` or
  `FFX_HOOK_ONLY=@<file>` (`ffx-dinput8`, passed on by
  `tools/run-game.sh`; `docs/hooking.md`) installs only the named replacements,
  so a capture holds the original code plus the slice. Compare it with
  captures of the same build run `original`, not older ones: the harness
  changes what a capture sees (`orig3`/`orig4` of 2026-10-24 no longer
  match a fresh original capture of `tools/replays/draw-field.txt`).

## Run-time frames and varargs

`StackFrame::<W>` is a constant size. Some originals size a buffer at run
time with `_alloca_probe_16` (`0x00949890`; `_alloca_probe_8` and
`_alloca_probe` alike) and hand it to callees: a string with `.lua`
added, the UTF-16 copy of a path, the display modes of an output. Others
hand a callee the address of their own variable arguments (a `va_list`).
Both are ported without new machinery in the bridges (M5 round
"design-frames"; the ports in `ffx_prender::m5p_render_b_df`,
`m5p_scene_df`, `ffx_pother_a::m5p_scripting_df`,
`ffx_pother_b::m5p_framework_input_df`, `ffx_system::m5a_df`).

- **Split the bridge at the alloca.** The part before it does what the
  original does up to `call _alloca_probe_16` (its calls included, in
  order) and returns the size it computes, in the original's arithmetic
  (`count * 28`, `strlen + 5`, `2 * (len + 1)`, wrapping); `None` where
  the original skips the alloca. The part after takes the buffer's
  address. An alloca has no effect a callee can see, so a second alloca
  (`File_Probe`, `openFile`: the UTF-16 path, then the archive name) can
  be carved together with the first.
- **The shim carves the frame:** `super::with_frame(bytes, |at| ...)`
  (`ported/mod.rs`) runs the closure with `bytes` zeroed bytes, 16-byte
  aligned (what `_alloca_probe_16` leaves, so at least what the other two
  do), alive until it returns. Nested calls nest like allocas: the inner
  frame lies below the outer. In the hook the bytes come from the
  thread's stack (the smallest of five fixed sizes up to 16 KB, each in
  its own call; beyond that the heap, which the original's 32 KB threads
  could not have given anyway); in `ffx-host` from the router's frame on
  the game thread's stack in the arena (`Router::frame_alloc`, already
  16-byte aligned), so the originals in Unicorn reach it.

  ```rust
  match ad::probe_sizes(&mem(), path) {
      None => ad::file_probe(&mut mem(), path, None),
      Some((len, wide, narrow)) => with_frame(wide, |w| {
          with_frame(narrow, |n| ad::file_probe(&mut mem(), path, Some((len, w, n))))
      }),
  }
  ```
- **Locals at fixed offsets** next to the alloca (the adapter, output and
  count `Renderer_SetDisplayMode` hands the DXGI calls, the screenshot's
  writer object) stay a `StackFrame` laid out at the original's `ebp`
  offsets, as before.
- **A `va_list`** is a pointer into the caller's argument words: declare
  the named arguments and take `slots` (`fn shim(a, b; slots: u32)`,
  `docs/workflow.md`, "Argument slots"); the first variable word is at
  `slots + 4 * named`. The `va_list` word itself, when the original keeps
  it in its frame and passes its address, is a `StackFrame::<1>`
  (`0x0044e9e0`); `Phyre_PAnimationTargetBlenderController_log_00530b20`
  passes it the same way. An aligned scratch buffer the original rounds
  up inside its frame (`lea eax, [ebp-0x1c]; neg eax; and eax, 0xf`) is a
  `with_frame` of its size.
- **Tests.** The oracle test computes where the original's buffer lies
  for its call: the arguments are pushed from `stack_range().end - 0x100`,
  then the return address, so `ebp` is that less `4 * words + 8`; the
  saved registers and the frame's own `sub esp` give `esp` at the alloca,
  and the buffer is `(esp - bytes) & !0xf`. It hands the part after the
  alloca that address in the other oracle, runs the port's calls below it
  (`Oracle::set_call_top`), and compares the buffer bytes after the call
  (a UTF-16 string up to its NUL: the rest is never written).
  `crates/ffx-oracle/tests/m5_design_frames.rs` has the helpers (stubs
  that write through their arguments and copy into buffers, a second
  answer for a callee called twice).

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

## Functions under five bytes

A bare `ret`, `ret 4`, `xor eax, eax; ret`, `mov al, 1; ret`,
`mov eax, ecx; ret` or a one-instruction field read off `ecx` is too short
for the hook's 5-byte `jmp`: patching it would overwrite the next function
or its padding. In the 32-bit game their bytes keep running (until M4 they
were only listed, `stub` in `symbols/classes.tsv` and
`symbols/m4-not-portable.tsv`). `ffx-rust` runs no `FFX.exe` code in the
end, so its function table needs them too: they get a port and a table
entry, and no hook.

- The lines go in a `host_only! { ... }` block instead of `ported!`
  (same syntax; `crates/ffx-hook/src/ported/mod.rs`). In the hook's build
  the shims are compiled and linted but the module's `hooks()` is empty;
  under `cfg(ffx_host)` (`crates/ffx-host-ports`) the block is a
  `ported!` list, one table entry per line. ffx-host traps an entry with an
  `int3` on its first byte, so the length doesn't matter there.
- `tools/check-ported.py` accepts an address under five bytes only inside
  `host_only!`, and refuses one of five bytes or more there (that one gets
  a hook). Name, convention and `ret n` are checked as for `ported!`.
- The shim declares what the original pops (`ret n`: thiscall with `n / 4`
  stack words when it is a method) and returns what the original leaves:
  `u8` for `xor al, al` / `mov al, 1` / `mov al, [ecx+n]` (the upper bytes
  of `eax` are the caller's there, as for any `u8` port), `f32` for
  `fld dword [ecx+n]`, nothing for a bare `ret` (the table keeps the
  caller's `eax`, `ecx` and `edx`).
- Tests: the oracle test as for any port (each copy against its own bytes,
  on the image and both snapshots), plus a run through the table, since no
  replay of the hooked game reaches the port: `crates/ffx-host/tests/m5p_stubs.rs`
  calls each copy from Unicorn code with the ported table and without it.
- List each in `symbols/m5-not-portable.tsv` with the reason (not hookable
  in-process; a host-only table entry), unless `m4-not-portable.tsv` has it.

The PhyreEngine ones are `ffx_pcore::m5p_stubs` (M5 list `stubs`): nine
shapes, one generic port each, and a generated table of the 179 copies.

## What not to do

- No raw pointers in module crates: `Mem` only, so the same code runs in
  the oracle and later in the 64-bit host.
- No Fahrenheit or PhyreEngine SDK code; no game data in the repo.
- No replacement without its oracle test.

## Compiler bug

rustc 1.99.0 (LLVM 23.1.1) miscompiles this shape whenever optimization is
on (`-O1` and up, so every release build, the hook and the oracle tests
included):

```rust
if v > 0x80 { 0x80 } else { v.max(0) }   // 129 comes out as 129
```

The range LLVM infers for `v` inside the else arm (at most 0x80) stays on
the operand of the `smax` it becomes, and once that `smax` is hoisted out
of the branch the bound check is folded away. The mirrored form
(`if v < k { k } else { v.min(h) }`) and `i64`/`u32` versions fail the
same way; whether a given site fails depends on what the optimizer can see
of its callers. rustc 1.100.0-beta.6 and beta.7 compile all of them correctly; the repo pins
beta.6 in `rust-toolchain.toml` until 1.100 is stable (user, 2026-10-28). Found
by the pc-ui parity test of `Gui_WindowSetGlobalTrans` (2026-10-28).

Until the toolchain moves past 1.99:
- write the explicit chain, `if v > hi { hi } else if v < lo { lo } else { v }`;
- use `.clamp(lo, hi)` only with constant bounds (`clamp` panics when
  `lo > hi`, and a panic in a hook ends the game);
- `tools/check-ported.py` rejects the shape anywhere in `crates/`.
