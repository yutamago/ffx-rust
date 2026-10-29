# Rewrite plan

Goal: all of FFX's game code in Rust. The result runs without `FFX.exe`,
builds for 64-bit and for other platforms, and can be changed anywhere by
mods. It reads the player's own `FFX_Data.vbf` and `metamenu.vbf`; we ship
nothing of the game.

## Where we stand (2026-10-16)

This is the baseline the plan started from. Since then M0 to M4 are done
(M4's result is under "M4" below). Tracked numbers per
module are in `tools/progress.py` and the status page (`tools/status.sh`).

Ghidra finds 55,707 functions in 6.27 MB of code in `FFX.exe`. Sorting them
by name, and the unnamed ones by their named neighbours, gives the split
below. The neighbour guess is crude, so read the module sizes as ±30%.

| Part | Code | Functions | Named |
|---|---|---|---|
| PhyreEngine (statically linked) | 2,206 KB | 24,800 | 5% |
| MSVC runtime and STL | 874 KB | 13,000 | 10% |
| Lua 5.2.1, zlib, Bullet | 177 KB | 1,000 | 3% |
| **Game code** | **2,862 KB** | **~17,000** | **~17%** |

The Switch build's symbols carry the original PS2 module prefixes, which
split the game code like this:

| Module | Prefix | Code | Named | What it is |
|---|---|---|---|---|
| Map and background | `rc*`, `Yn`, `Gu`, `Ke`, `Sqs`, `Nkz`, `yi` | 430 KB | 12% | field maps, walk meshes, guide map, map effects |
| Battle | `Ms` | 414 KB | 26% | battle flow, actors, commands, damage, AI calls, rewards |
| System | `BigFile*`, `Save*`, `CD*`, `fios`, `sce*`, `TO*` | 401 KB | 31% | archives, PS2 file layer, saves, language, settings |
| Menus and text | `Tk` | 299 KB | 13% | windows, text, the PS2-era menus |
| Particle effects | `ppp`, `ClassVFX` | 274 KB | 16% | effects of spells, summons, maps |
| Event scripts | `Atel`, `CT_` | 238 KB | 14% | the script VM and its natives |
| Remaster renderer | `graphic*`, `PhyreScene`, managers | 221 KB | 11% | the HD layer over PhyreEngine |
| PC user interface | `Menu*`, `FFEscMenu`, `Iggy*` | 215 KB | 9% | title, config, save list, Esc menu (Flash through Iggy) |
| Main loop | `Sg`, `FFXApplication` | 122 KB | 26% | frame, fades, scene changes, menu requests |
| Characters | `Ch` | 100 KB | 30% | models, motions, attachments |
| Movies | `movie`, `PhyFMV` | 70 KB | 5% | `.webm` playback |
| Sound | `Snd`, `Fmod*` | 63 KB | 7% | music, sound effects, voice through FMOD Ex |
| PS2 emulation | `AsmEmu`, `FFXVu` | 15 KB | 11% | VU instructions emulated on the CPU |
| Spell overlays | `magic_*.dll` | ≤3.8 MB unique | 0% | spell and summon effects, one DLL each (below) |

Next to the exe are the spell and summon effects: 581 DLLs in
`magicFiles/FFX/` (`magic_0003.dll` … `magic_0710.dll`), the PS2's effect
overlays compiled for Windows. Together they have 10 MB of code; with
relocations masked and pieces deduplicated by hash, at most 3.8 MB of it is
unique, in about 8,100 pieces. That's an upper bound: copies differ wherever
their relative calls point elsewhere. Each DLL exports `InitMagicPRX` and
`GetEffectOverlayTable` and imports only the C runtime and KERNEL32. FFX.exe
loads one (`MagicFile_Start`, `onMagicDLLLoaded` at `0x009db110`) and passes
`InitMagicPRX` a table of its own functions (`g_MagicImports`, `0x00c64ce8`:
744 slots, 658 of them `FFX.exe` functions: effects 278, battle 114, map
104, characters 83, main 24, system 21, PS2 emulation 20), so the overlays
call back into the game only through that table. Once those functions are
Rust, the overlays already run on them (the slots point at the patched
originals). That makes them a module with one
clean boundary: in the hybrid phase a Rust overlay can stand in for one DLL
behind the same two exports.

Outside the exe, the game also loads `fmodex.dll` and `fmod_event.dll` (FMOD Ex),
`iggy_w32.dll` (Flash UI), `steam_api.dll`, `d3d11.dll`, XInput and DInput.

In Rust so far:
- **Data formats.** VBF archives (read and write), kernel tables, text,
  monsters, formations and encounters, script bytecode with 354 named
  natives, PhyreEngine clusters, models, textures and motions.
- **Running in the game.** 2 functions replaced (`zlib_uncompress`,
  `Path_ToLowerAscii`) and 8 wrapped. Mod loading, the test harness and the
  external 64-bit renderer (wgpu) all run through these hooks.
- **Infrastructure.** The hook table and patcher (`ffx-hook`), parity tests
  against the original machine code in Unicorn (`ffx-oracle`), in-game tests
  (`ffx-harness`), and the scene stream that feeds the renderer.

## What "100%" means

- **All game code.** We rewrite the ~17,000 functions of the PS2-derived
  modules and the remaster layer one at a time and check each against the
  original.
- **Libraries.** We replace them instead of decompiling them, because their
  code isn't the game's. We rewrite only the part the game uses, behind our
  own interfaces:
  - PhyreEngine: our asset loaders (`ffx-phyre`), our scene graph and the
    wgpu renderer (`ffx-render`). Step one is a map of every PhyreEngine call
    the game code makes; that list is the whole job, not the engine's 24,800
    functions.
  - MSVC runtime and STL: Rust `std`. zlib: `miniz_oxide` (already).
  - Lua and Bullet: first find what calls them. Lua looks like PhyreEngine's
    script component; drop whatever the game never uses.
  - FMOD Ex: our own mixer over the `.fsb` sound banks (1,260) and `.fev`
    event files (1,260). Both formats need decoding first.
  - Iggy (19 `.swf` screens): played by Ruffle (pure Rust, MIT OR
    Apache-2.0), tried on three screens first; if its quality isn't good
    enough, the screens are rebuilt natively (user, 2026-10-27).
  - `.webm` movies: an open VP8/VP9 decoder with Vorbis/Opus audio.
  - Steam: achievements through `steam_api64.dll`, shipped with the
    release package.

## What gets ported

Not every game function needs an exact port. `tools/classify.py` sorts the
17,986 functions of the game modules into six categories and writes
`symbols/classes.tsv` (address, category, one line of evidence);
`tools/progress.py` counts what is left in its "to port" columns (logic
not replaced yet).

| category | what | what happens to it |
|---|---|---|
| logic | decides game state or the original look: battle, field, events, the script VM and natives, characters, menu logic, effect simulation, the PS2 Snd layer | ported exactly, with oracle parity |
| glue | only passes data to a replaced library (state setters on its objects, render targets, shaders, buffers, packet writers): PhyreEngine/D3D11 (render), Iggy (pc-ui), FMOD (FfxFmod, FmodMusic/Sfx/Voice, iMusic/iSfx), the movie player | replaced in M4 at the library boundary, not ported |
| platform | file layer (fios, CD/DVD emulation, VBF), allocators (Sg_Mem, user heap), threads and locks, settings, saves on disk, Steam, window/input setup, std:: copies | replaced by the host and Rust `std` |
| debug | debug consoles, debug drawing, host0: readers, Deci2, tweak overlays | not ported now; kept as mod-tool candidates (below) |
| dead | nothing reaches it (no call, jump or pointer anywhere in the image), or PS2-only (memory card, IOP) | not ported |
| stub | logic under five bytes, too short for the hook's 5-byte jmp: a bare `ret` (115, such as `dbgPrintf`), `ret 4`/`ret 8`, `xor eax, eax; ret`, `mov eax, ecx`, a `lea`/`mov` field return off `ecx` (158 on 2026-10-24); the evidence names the bytes' behaviour | not ported or hooked: the original bytes keep running, and callers' ports reproduce them or call them; counted apart from "to port" (`stub`, `stub_bytes`) |

Numbers (2026-10-17, with runtime coverage; functions and size):

| module | logic | glue | platform | debug | dead |
|---|---|---|---|---|---|
| vfx | 1490 (491K) | 189 (62K) | 41 (4K) | 19 (4K) | 673 (74K) |
| system | 1302 (266K) | 145 (82K) | 445 (56K) | 25 (3K) | 1247 (116K) |
| render | 469 (63K) | 662 (212K) | 53 (7K) | 14 (2K) | 782 (49K) |
| map | 972 (194K) | 30 (10K) | 2 (0K) | 219 (60K) | 451 (60K) |
| battle | 1151 (214K) | 1 (1K) | 0 | 141 (96K) | 143 (14K) |
| menu | 777 (183K) | 3 (0K) | 1 (0K) | 172 (22K) | 237 (20K) |
| script | 2042 (182K) | 0 | 13 (1K) | 24 (9K) | 241 (18K) |
| pc-ui | 461 (83K) | 205 (58K) | 63 (8K) | 62 (26K) | 417 (31K) |
| main | 560 (103K) | 6 (2K) | 84 (6K) | 127 (25K) | 96 (13K) |
| chara | 443 (66K) | 20 (4K) | 2 (0K) | 39 (3K) | 106 (10K) |
| sound | 134 (10K) | 305 (29K) | 47 (2K) | 1 (0K) | 118 (8K) |
| movie | 49 (6K) | 68 (9K) | 35 (5K) | 25 (7K) | 215 (18K) |
| ps2-emu | 54 (8K) | 0 | 3 (2K) | 0 | 65 (4K) |
| **GAME** | **9904 (1870K)** | **1634 (469K)** | **789 (90K)** | **868 (257K)** | **4791 (433K)** |

So about 60% of the game code by size is logic and needs an exact port.
Of the 5,199 functions ported so far, 3,614 are logic; 657 are dead, 381
glue, 327 debug and 220 platform.

How the evidence works, in order (first rule that fires): reachability
from the PE entry point and from every function address found in the
image's data (vtables, native and overlay tables, exception tables) or as
an immediate in code, plus Ghidra's calls and objdump's direct
call/jmp/jcc edges (Ghidra leaves out tail jumps), with the 676 blocks
those reach that Ghidra made no function for (thunks, tail-call targets)
as nodes of their own; then name rules (the
PC-only rules stop at 0x00760000, where the PS2 game code starts and
porters reuse words like Menu*, Save*; scene state the new renderer
needs as input is logic even under a glue prefix: camera view and
projection, fog/ambient/background colours, screen fades, mesh and
skeleton matrices, character lighting, 2D-layer and mesh visibility, the
minimap and area-map camera); then Iggy/PhyreEngine calls and
Win32 imports; then, in the PC port's part of the image (below
0x00760000), copies of std:: and PhyreEngine container code are platform:
functions whose every call goes to allocators, trees, free lists, vectors,
std:: copies or platform code, static initialisers (atexit), destructors
only exception unwinding reaches, and the unnamed stubs between the PS2 SDK
stand-ins; then pure GS/DMA packet writers are glue (the packet API,
`TOAdp*`, `rcPacketData*`, the `sceDma*`/`sceGs*` kicks, the 2D shape
writers `TOMkp*`/`TODraw*`/`TOMakePkt*` and the PC port's `graphicDrawUI*`
that replaced their GIF primitives, and functions that only call it,
read-only getters, the menu scissor clip and VU0 scratch, and write no
global any logic reads apart from the packet API's own cursor; a writer
found this way counts as packet API for the next round when it appends,
never when it only kicks a DMA); then propagation (a function only debug code calls
is debug, callers outside the game modules included, and an unnamed function
between two debug methods of one class, such as the free camera's, is
debug; glue and platform pass one hop to unnamed callees, and any number of
hops inside the PC part when the callee calls no game logic: allocator
internals, container copies, renderer set-up; what only exception unwinding
reaches is platform). Static initialisers (atexit) are platform anywhere. Last, a thin wrapper
(at most 64 bytes, no global write, every call to platform code, or in the
PC part to glue or platform code) is that boundary even
under a scene-state name: `graphicAddScreenFadeTask` only registers a
renderer delegation, so ffx-render takes its arguments. Auto-port shape
names (`SetFields_*`, `Compare_*`) count as unnamed. Anything left is
logic: skipping something necessary breaks parity, porting something
unnecessary only costs time. A debug-sounding name that logic calls (an
"is debug" flag the battle code reads, an rc math helper) is logic.

**Runtime coverage** checks the static verdict. `ffx-hook`'s coverage mode
(`FFX_COVERAGE=<list>`, `crates/ffx-hook/src/coverage.rs`) puts a one-time
`int3` on the first byte of every dead and debug candidate; the vectored
handler restores the byte, logs the address and resumes. `tools/coverage-run.sh`
runs it on the original code (`--hook-disable replacements`, so ported
functions count too) through the three replays and the coverage session
(`tools/replays/coverage-session.txt`: boot, load, walk and talk, a battle
with attacks, skills, a summon attempt and items, every main menu screen,
the PC menu) on all five saves; `tools/coverage.py merge` keeps the union in
`symbols/coverage.txt`, which `tools/classify.py` reads: a function that ran
is a root, and one no static reference reaches is logic (debug when it
looks like debug); a debug-named function that ran and that logic calls is
logic. Of 5,844 armed candidates, 63 ran. Of the 34 that were dead in the
first classification, the better static graph (blocks Ghidra misses,
fall-through) already explained 32; the last two (`MenuTuto_GetChoice` and
its neighbour, reached only at run time) moved to logic. Of the 29 debug
functions that ran, 5 moved to logic (two VU-emulation helpers with an
assert, `rcuiCopyMatrix`/`rcuiMulMatrix` that the movie camera uses, one
battle helper); the other 24 are debug code the release build calls every
frame or at start-up without doing anything visible (the PC tweak overlays'
init and render, `Sg_MainDebug`, `Native_DebugPrint`, `dbgPrintf`).
The saves load on Besaid (bsil0000, three) and in Kilika (kami0100, two).
Not covered yet: saving (not scripted), whether the blind summon attempt
actually summoned (the battle menu is driven without checking the screen),
and later content; dead stays a static verdict there.

### Debug code worth keeping (mod-tool candidates)

| family | functions | what it is | could become |
|---|---|---|---|
| `Ms_Gui_*` (battle) | 11 + ~120 helpers | in-battle debug consoles: config, chr 1/2, info, camera, map data, shake, reference position | a battle inspector overlay: actor stats, ATB/CTB, camera and shake tuning, live |
| `MsDebug*` | 13 | full gil/items/weapons, add item/weapon, debug info read, out camera, env save | a cheat/test panel for modders (give items, set HP) |
| `rcui*`, `rcbgGuide*` (debug-only parts) | 51 + ~160 | the rc map tool's windows: camera, layers, alpha, anime, FP memory, guide map converter | a map viewer/editor: layers, camera paths, guide map export |
| `Gu_*`, `Gui_*`, `gui_*` | 140 | the PS2 debug GUI toolkit: windows, buttons, selectors, mouse, printf | the widget layer for the other tools, or replaced by our own overlay UI |
| `Sg_Db*` | 14 | debug primitives: boxes, circles, 2D vertices | collision and trigger visualisation in the field |
| `Sg_Capture*` | 7 | capture mode with filters, selection, GUI hide, capture to file | a photo mode |
| `TkPutDebugFont*`, `TkDebug*`, `TkGetDebugControler*` | 24 | debug text on screen, pad history | an on-screen log and input display |
| `AtelDebug*`, `AtelPs2DebugString*`, `AtelSetUpSourceDebug`, `AtelLoadDebugData` | 6 | the event script's debug draw and source debug data | an event script debugger (step, breakpoints, worker state) |
| `Ch_DrawGet/Set*`, `Ch_GuiGet*`, `Ch_PacketSetSpec*` | 16 | character draw parameters: specular, shadow colour/direction, parameter edit | a character look editor |
| `*TweakUtil*`, `DebugMenuManager` | 23 | the PC port's tweak overlays: post process, lighting, animation, battle params | renderer and lighting tuning panels |
| `yiGui_setfogparam*` | 2 | fog parameter window | fog editing in the map editor |
| `debugLoadForViewer`, `debugSavedataLoadAll`, `SaveDataPreDebugSave` | 3 | save viewer and debug saves | a save-file viewer/editor |
| `sceDeci2*`, `TkInitDeci2` | 3 | the PS2 host debugger link | nothing (PS2 only) |

## Strategy

### Rules for every rewritten function

1. **Named and understood first.** The function's callees and globals are
   named in `symbols/ffx.sym` (Fahrenheit's names where they have one).
2. **Parity before it ships.** An `ffx-oracle` test runs the original and the
   Rust code on the same inputs; stateful functions run on memory snapshots
   taken from the running game (below).
3. **Written against our own types, not the game's memory.** In the game a
   Rust function has to read and write the original structs: 32-bit
   pointers, fixed offsets, fixed array sizes. If the logic were written
   against those, it would all need rewriting for 64-bit. Instead, each
   module gets:
   - a **core crate** with its own types (indices or handles instead of
     pointers, explicit layouts only where data is saved), built for any
     target;
   - a thin **bridge** in `ffx-hook` (i686 only) that maps game memory to and
     from the core's types while original code still shares that state.

   Once every function that touches a piece of state is in Rust, the state
   moves into the core and the bridge shrinks to the views the remaining
   original callers need. When the last caller goes, the bridge goes too.

### One process, then 64-bit

Until 2026-10-24 the plan grew a separate 64-bit process (`ffx-render`,
fed by the scene stream) toward the 32-bit game, because enhancements
needed more than the 32-bit process allows. Enhancements are now a
separate project after the rewrite (M4), so the platform is rewritten
in-process instead (user, 2026-10-24): the Rust renderer, audio, input,
window, movies and PC UI run inside the 32-bit game, the way the logic
does, on the game's own D3D11 device and with its own shaders. Every crate
is built for any target, so at M5 the whole game becomes one 64-bit Rust
program and `FFX.exe` is no longer loaded. The external renderer is parked
as it is (with its enhanced mode), as a starting point for the
enhancement project.

### Verification at scale

Single-function parity is not enough for 17,000 functions. Three levels:

1. **Function parity.** `ffx-oracle` runs synthetic and real inputs and
   compares results, outputs and memory writes.
2. **Snapshot parity.** Dump the game's memory from the running game
   (Frida), load it into the Unicorn oracle, run the original and the Rust
   function on the same state, and compare every byte written. This covers
   functions whose inputs are whole game states (battle turns, worker
   updates, menus).
3. **Replay parity.** The harness already seeds randomness and runs frames
   back to back. Record input sessions (a battle, a map walk, an event) and
   play them on the original and on the hybrid build. Compare per-frame
   hashes of the state that matters (save block, battle actors, script
   workers, camera). A replaced module is done when its replays match.

## Order

We port leaves before the code that calls them, then whole modules in the
order their interfaces get narrower.

### M0: tooling (before porting at scale)

Status (2026-10-16): done except the naming push, which goes on with
every module. `tools/progress.py`, `tools/ownership.py`,
`tools/cg-match.py`, `ffx-bridge`, snapshot parity (`ffx-oracle`) and replay
parity (`ffx_harness::replay`) work; the control word is `0x027f`.

- `tools/progress.py`: named % and ported % per module, from `ffx.sym`, the
  hook table and the dump. Replaces the estimates above with tracked numbers.
- **Ownership map.** A Ghidra script over xrefs lists, for each global and
  each known struct, the functions that read or write it. That list draws
  the module boundaries and says how much each bridge has to cover.
- **Naming push.** More Switch→Steam matches (their version-tracking sessions
  have 37,000 candidates; we use the 1:1 ones), struct layouts from the
  Fahrenheit findings, our own analysis. Target: ≥60% of a module named
  before porting it.
- Snapshot oracle and replay recording in `ffx-harness`.
- **Float model.** The game's float code is x87: `objdump` counts about
  68,000 `flds` and `fstps` against about 5,000 `movss`, the SSE part
  probably PhyreEngine's vector math. x87 keeps intermediates in 80-bit
  registers, rounded to the precision the control word sets (MSVC's default
  is 53 bits), and Rust's f32 on SSE2 rounds every step. Read the game's
  control word at run time, then decide per function: bit-exact where
  results feed saved or compared state (damage, RNG-adjacent math, anything
  replays hash), a stated tolerance where they only reach the screen.

### M1: leaves and the script VM core

- **Pure functions.** Random numbers (`rnd`, `brnd`, `MsRandMize`), damage
  (`MsCalcDamageCommand`), kernel and ROM lookups (`MsGetExcelData`,
  `MsGetRom*`), text decoding (`DecodingGamecodeJPCHKR`), the VU-emulation
  math. They're stateless or nearly, so oracle-testable now.
- **Atel VM core.** Stack, workers, opcode loop, variable storage.
  `ffx-script` already decodes the bytecode. Natives at first call the
  original through the bridge; each moves to Rust together with its module.
  The VM drives fields, events and monster AI, so every later milestone
  depends on it.

### M2: battle

`Ms`: the battle loop, actors, command selection, targeting, status,
rewards, plus their natives. The formats are done and the battle is
deterministic under a seeded RNG, so replay parity fits it best. It is done
when a recorded battle replays identically with no original `Ms` function
left.

The spell overlays go with M2 as far as battle logic needs them (their
calls into the game), and with M4 for what they draw: first the shared
runtime that most DLLs contain a copy of, then the overlays one by one,
each checked by recording its spell in a battle replay.

### M3: field, events, characters, menus

Map loading, walk meshes and collision, guide map (`rc*`, `Yn`, `Gu`);
characters and motions (`Ch`; motion formats are done); the remaining Atel
natives; menus and text (`Tk`). It is done when walking, events and the
menus of a save run on Rust logic and replay identically.

**Done 2026-10-24.**
- 10,286 functions ported and hooked. 5 are held on purpose: Sg_MainLoop, updateFFX and 0x822490 for M5; opu_v_u2 and pppKeLnsLpTOtDraw until they are observed in-game.
- 158 functions too short to hook are classified `stub`.
- All 13 replays in `tools/replays/run-all.sh` give the original's events on Rust logic:
  - walking (field-walk);
  - events (event-besaid, event-cliff, event-agency, event-arrival);
  - menus (menu-browse, menu-shop, menu-grid, menu-save);
  - battles (battle-attack, battle-magic, battle-overdrive);
  - the boot (boot).
- Their screenshots match, or are checked by eye where the original's own runs differ.
- The event replays found two port bugs, both an 8-word call through a 6-word host (now caught by `tools/check-callwidth.py`). Both are fixed.

### M4: Rust takes the platform, in-process

Every part of the platform (the scene renderer, effects and
post-processing, the UI, movies, audio, input and the window) moves to
Rust inside the game's process and must give the original's output
exactly. Improvements to graphics, sound and input are not part of this
rewrite: they are a separate project after it, backwards compatible with
the original behaviour (user, 2026-10-24). The external renderer
(`ffx-render`, its enhanced mode) is parked and gets no M4 work.

**Ported, not rebuilt (user, 2026-10-25).** M4 ports the platform code
the game reaches, function by function with the M3 pipeline: the
statically linked PhyreEngine and the game's glue over it (`graphic*`,
`PhyreScene`, the FMOD and Iggy wrappers, the movie manager). Output is
identical by construction, and every function has a parity test. The
m4-pilot measured it (`~/ffx-work/m4/pilot-report.md`):
- the game reaches 2,799 of PhyreEngine's 24,943 functions (514 of
  2,084 KB) and 1,494 glue functions;
- a 21-function slice of the map's render chain ported at about 3.5
  minutes per function and passed drawdiff and the replays.

A first rebuild (round 2: `ffx-scene`'s map and character builders, the
in-game map path in `ffx-hook/src/gfx/scene.rs`) gives the same draws as
the original on the captured maps. It stays as a second check and as the
seam for M6's backends.

Order (user's decision, 2026-10-24): the renderer's scene first, then
effects and post-processing, the UI, movies, audio, input and the window.
Porting runs the subsystems in parallel, but merges and checks keep that
order of priority.

What stays as it is in M4:
- the Direct3D 11 calls (still calls, through the device's vtables);
- the external DLLs: FMOD (`fmodex.dll`, `fmod_event.dll`) and Iggy
  (`iggy_w32.dll`);
- the open-source libraries linked into FFX.exe (libvpx, vorbis,
  mkvparser);
- the CRT and the platform category.

M5 replaces them: Rust `std`, pure-Rust decoders, our own mixer for
FMOD and a Rust SWF player for the PC screens (decided 2026-10-27, M5
below).

Each part replaces the original only when it passes:

- a draw-stream comparison: the D3D11 calls of the ported code (draws,
  order, shaders, constants, state, resources) against the original's for
  the same frame, captured in the same process, call by call
  (`ffx-drawdiff`, references `~/ffx-work/m4/captures/ref-*`);
- the replays and screenshot comparisons against the original game
  (`tools/replays/run-all.sh`, `tools/shot-compare.py`);
- for audio, FMOD's mixed output written with its WAV writer, compared
  sample for sample (sound may be on, on the main machine only).

**Done 2026-10-27.**
- **Ports.** The pilot's coverage run split the reached code into 10 lists of 4,183 functions. A second coverage round added 18 more, with replays for the graphics settings, a movie, the Iggy screens and a summon battle.
  - 4,146 of the 4,201 are ported and hooked, each with a parity test against the original: 14,053 hooks in all with M1 to M3.
  - 54 can't be hooked: 52 are under the hook's 5 bytes, plus a tail jump forwarding an unknown argument count and the SetThreadName exception.
  - `Game_Main` (0x0062ee50) is held for M5 on purpose, like M3's `Sg_MainLoop` and `updateFFX`. It is the Win32 message pump and the frame limiter the harness's lockstep hooks into, and M5's inversion replaces it.
  - All 55 are listed with their reason and evidence in `symbols/m4-not-portable.tsv`.
- **Draw streams.** `tools/scene-gate.sh` gates six scenes with an allow file each (`crates/ffx-drawdiff/allow/gate/`): draw-field, bsil0000, bsil0300, kami0100, a menu and a battle. Each passes with only the M4 ports and with every hook on.
  - Every allow entry has a cause seen between two runs of the original, except one: `SgSpec_CameraVec[0].z`, a never-written stack read in `Ch_DrawMain`, which the junk-read policy sets to 0.
- **Audio.** `run-all.sh --audio` compares FMOD's mix sample for sample on event-besaid, battle-attack, menu-browse and battle-ifrit; all four are identical with every port on. It found one port bug: `MsMusicReadStart` dropped the victory fanfare's id.
- **Replays.** The main list (13 replays) and the extra list (9) give the original's trace events with every port on. What they still differ by is listed in `docs/harness.md`, "Accepted residue": eye blinks, cloth, and a few pixels of animation phase that also differ between two runs of the original.
  - Most of the flakes on the way were real-time dependencies in the harness or the game, not ports: the loader racing the render lock, FMOD and Iggy on the wall clock, `GlobalTime`, the title water counter, the save copies' file times, and the user's mouse reaching background games. Each one now runs on game time or is pinned.
- **Static checks.** Besides check-ported, check-clobber and check-callwidth:
  - check-thisarg: callees that read ecx;
  - check-argslots: argument slots an unhooked caller reads back;
  - check-callconv: `ret n` against the port's convention;
  - check-tailargs: stack words an original only forwards;
  - check-privacy: no user paths in tracked files, no-reply commit e-mails only.
  `tools/verify-batch.py` runs them all.

What stays for M5:
- the Direct3D 11 calls (still calls, through the device's vtables);
- FMOD (`fmodex.dll`, `fmod_event.dll`) and Iggy (`iggy_w32.dll`);
- libvpx, vorbis and mkvparser, linked into FFX.exe;
- the CRT and the platform category;
- `Game_Main` and M3's held main-loop functions;
- the functions the coverage runs never reached.

### M5: inversion

A 64-bit Rust program, `ffx-rust.exe`, runs the whole game without
loading `FFX.exe`. Output stays identical to the original's, saves stay
byte-compatible, and the original stays untouched next to it.

**Decisions (user, 2026-10-27).**
- **The 32-bit arena.** The 64-bit program reserves one 4 GB block and
  keeps every game address as a `u32` offset into it, so the 14,053 ports
  run unchanged through `Mem`. The game's heap allocates inside the arena,
  and objects outside it (COM interfaces, OS handles) get `u32` handles.
  At start-up the program maps `.data`, `.rdata` and the other data
  sections from the player's own `FFX.exe`, read as data the way the
  `.vbf` archives are (nothing of the game ships). Moving state into
  Rust types is a separate project after the rewrite.
- **One function table.** Every port registers under its original
  address. `call_sym`, `call_virtual` and every function pointer stored
  in game memory (vtables, callbacks, the script native tables) dispatch
  through that table. Later this is the modding seam: a mod replaces a
  function by its address, the way Fahrenheit mods do, without patching
  code.
- **Statically reachable code only.** A static call graph from the entry
  point, the vtables, the native tables and every stored function
  pointer decides what still has to be ported, with parity tests on
  synthetic inputs where no replay reaches it. Unreachable code is
  dropped and listed with the evidence (`symbols/m5-dropped.tsv`).
- **D3D11 at M5.** The 64-bit program issues the game's D3D11 calls
  itself, so the draw-stream gate still compares call for call. Proton
  runs it on Linux and the Steam Deck. The choice between wgpu and Vulkan
  is made at M6.
- **Our own mixer for FMOD.** Rust decoders for `.fsb` and `.fev` and a
  mixer, gated sample for sample against FMOD's WAV writer, as at M4.
  Where FMOD's resampler or reverb can't be matched exactly, the residue
  is documented and the user signs off on a tolerance.
- **Iggy: Ruffle at runtime (user, 2026-10-27).** The game's 19 `.swf`
  files (SWF 9, ActionScript 3: about 4,700 methods in `escmenu.swf`,
  3,000 in each `saveload_*.swf`) are played as they are by Ruffle, a
  pure-Rust Flash player under MIT OR Apache-2.0, used as a dependency.
  We write the game's side: the 41 Iggy calls `FFX.exe` makes (players,
  ticks, AS3 function calls, the AS3 callback, value paths, input events)
  and drawing through our D3D11. Considered and rejected: our own player
  (the AS3 VM is most of the work), converting to our own format or to
  Lottie (the logic would need porting, the conversion loses detail),
  Rive (C++ runtime, proprietary editor). The title menu, the settings
  and save/load come first, for the user to judge; if the quality isn't
  good enough, the screens are rebuilt natively.
- **Pure Rust.** No C libraries: pure-Rust VP8/VP9, Vorbis and Matroska
  decoders (existing crates, or our own port where the game's use is
  small). The CRT and STL become Rust `std`. One exception: Unicorn runs
  the original spell overlays until they are ported (stage 1, below).
- **Steam.** Achievements through `steam_api64.dll`, shipped with the
  release package (not in git).
- **Delivery.** `ffx-rust.exe` sits next to `FFX.exe` in the game folder
  and is started by `ffx-launch` or a Steam launch option. `FFX.exe`,
  the saves and the data stay untouched.
- **Luau after M5.** L1 and L2 start once the 64-bit program runs,
  against the function table.

**Stages.** Each one is merged and verified before the next depends on
it. Stages 1 and 2 run in the current 32-bit process, so every existing
gate still applies.

Stages 3 and 4 start alongside stage 1 (user, 2026-10-28): they are the
critical path and mostly sequential, while stage 1 spreads over many
agents. Until stage 1 is done, `ffx-host` runs any function without a
port in Unicorn from `FFX.exe`'s `.text`, read as data, so the 64-bit
program runs early and shows at run time what is still missing. That
fallback is for development only and is gone before the stage 4 gate;
the release runs no `FFX.exe` code.

A second machine (PC2) builds and runs tests and replays only, driven
over SSH; it has its own original recordings because its GPU renders
slightly differently.

1. **The rest of the game code, in-process.**
   - Build the static call graph and list what is reachable but not yet
     ported.
   - Port it with the M3/M4 pipeline: `Game_Main`, `Sg_MainLoop`,
     `updateFFX` and 0x822490 included (the lockstep hold point moves
     into the Rust main loop).
   - Port the CRT, STL and Lua functions the game reaches: memory, files,
     strings, math and `rand`, onto Rust `std`, keeping each function's
     observable results (`rand`'s sequence, `printf` formatting).
   - Port the PhyreEngine code the game reaches (m5-phyre-scope,
     2026-10-29). Scope: the 6,142 functions (517 KB) that the class-gated
     model (`tools/phyre-classes.py`) reaches and no hook covers yet,
     without the script layer. The script layer is the
     `PMethodCallerConcrete`/`PFunctionCallerConcrete` vtables and
     descriptor slots 3-6, and 2,950 functions only it reaches. A coverage
     run put a trap on each of its 3,032 entry points and 1,692 inner
     functions, ran the original code through the main list, the extra
     list and the five coverage sessions, and none of them fired; the three
     control traps fired in every run. The script layer is ported only if a
     later replay reaches it. More than half of the scope (3,205) is
     descriptor-template slot copies: one generic port per template slot,
     with a generated table of per-copy constants. The 136 movie-library
     functions (mkvparser, vorbis, libvpx) go to stage 2. The lists are in
     `~/ffx-work/m5/phyre/` (`tools/phyre-lists.py`, `plan.md`).
   - The spell and summon overlays (581 `magic_*.dll`, 10 MB of code, at
     most 3.8 MB unique; above) are not on M5's path (user, 2026-10-28).
     A 64-bit program can't load them, so from stage 3 on `ffx-host` runs
     the player's own DLLs in Unicorn inside the arena: it maps them,
     calls `InitMagicPRX` and `GetEffectOverlayTable`, and routes their
     `g_MagicImports` calls to the function table. That makes them
     identical by construction. Unicorn is C, the one exception to "pure
     Rust" (user: "just this once"), and only for the overlays.
   - Porting them (user, 2026-10-29: before the release; Unicorn,
     GPL-2.0, stays a development and test tool): each overlay becomes a
     Rust overlay behind the same interface (`crates/ffx-magic`,
     docs/formats/magic.md "Porting the overlays"), shared pieces are
     ported once and bind by code identity in every DLL that carries
     them, parity tests run the DLL's own code in Unicorn against the
     port, and `battle-magic`, `battle-ifrit` and `battle-overdrive` gate
     them in the game (`FFX_MAGIC_RUST`). The work list is
     `~/ffx-work/m5/magic/worklist.tsv` (`tools/magic-worklist.py`). A
     ported overlay replaces its Unicorn run; when the last one is
     ported, Unicorn goes.
   - Gate: a coverage run with a trap on every original `.text` byte
     except the libraries still to replace in stage 2 shows no game code
     running, across the main, extra and audio replay lists (the
     overlays' own code is outside `FFX.exe` and not part of the gate).
2. **The libraries, in-process.**
   - Movies: the pure-Rust decoders replace libvpx, vorbis and
     mkvparser. Gate: identical decoded frames and samples for every
     movie.
   - Sound: the mixer replaces FMOD. Gate: `run-all.sh --audio`, sample
     for sample.
   - Iggy: the Ruffle pilot on the title, settings and save/load
     screens, for the user to judge. Then the other screens, either on
     Ruffle or rebuilt natively.
   - Steam: our calls into `steam_api.dll`.
   - Gate: the in-process build runs no code from `FFX.exe` and loads none
     of the game's DLLs except `steam_api.dll`. Every replay, scene gate
     and audio check passes.
3. **The arena host, still 32-bit.**
   - A new crate, `ffx-host`, provides the arena `Mem`, the loader that
     maps the player's `FFX.exe` data sections, the arena heap, the handle
     table, the function table and the threads (render, loader, input and
     the mixer's).
   - `ffx-rust.exe` is built for i686 first. Floats, struct layouts and
     the replays behave as in the hooked game, so any difference is the
     host's.
   - Gate: replays and scene gates against the original, now run in a
     second process. The harness drives both with the same inputs and
     compares trace events, draw streams, the mix and screenshots
     (`docs/harness.md` gets a "two processes" section).
4. **64-bit.**
   - The same program built for x86_64: the arena's `u32` addresses, the
     handle table for 64-bit COM pointers, and the D3D11 and Win32 calls
     made from 64-bit.
   - Gate: the same as stage 3, plus a check that no code path builds a
     host pointer from an arena address without the arena base.
5. **Saves, settings and delivery.**
   - Gate: saves and settings written by `ffx-rust.exe` load in `FFX.exe`
     and back, byte for byte (on copies, never the user's saves).
   - Gate: the release links no GPL code. Unicorn (GPL-2.0) is behind
     the `dev-unicorn` feature of `ffx-hook`, `ffx-dinput8` and
     `ffx-host` (on by default for development builds);
     `tools/check-licences.py` lists the release trees (`cargo tree -e
     normal --no-default-features`) of `ffx-dinput8` (i686) and
     `ffx-host` (x86_64) and fails on any GPL-family licence, and both
     crates build with `--no-default-features`. Today `ffx-dinput8`
     builds that way (no overlay runner) and `ffx-host` refuses to (a
     `compile_error!`: its executor still needs Unicorn for code without
     a port, which goes at the stage 4 gate).
   - `ffx-launch` starts either program. The release package holds
     `ffx-rust.exe`, `steam_api64.dll` and the licences.
   - Docs: what a player needs and what stays the original's.

**Done when** `ffx-rust.exe` (64-bit) plays every replay with the
original's trace events, passes the scene gates and the audio checks, and
reads and writes the original's saves. Every function still in `FFX.exe`
is either ported or listed as unreachable with its evidence.

### M6: ports

Linux and the Steam Deck first, then macOS, then whatever else. The
native graphics backend (wgpu or Vulkan) is chosen at the start of M6;
until then Proton runs the 64-bit Windows build. FMOD is already gone
(M5's mixer). Controller mappings and the PC UI
screens need a per-platform pass.

## Determinism

Replays need the same inputs to reach the same state. Found so far:
- The game's clock follows real time; `--virtual-clock` makes it one frame
  interval per frame, from a fixed start.
- Archive reads run on the thread pool (`FileSys_SubmitRead`); on the
  virtual clock they run at once on the calling thread.
- The PS2 vsync interrupt is a thread calling its handler every 16 ms of
  real time (`createVsyncEmuThread`, `VsyncThreadImpl_run`); on the virtual
  clock the handler runs at the main thread's frame boundary, once per
  1/59.94 s of game time (`docs/harness.md`, "Frames and threads").
- The update runs on the main thread, the drawing on the render thread a
  frame behind; the harness samples and runs its jobs between two updates.
- The cloth's wind comes from the CRT `rand()`, seeded per thread, on
  whichever JobWorker runs the cloth job; the harness seeds it per job.
- Heap addresses change between runs (Windows' bottom-up ASLR, and other
  threads allocating); `ffx-launch` turns ASLR off for the game, but
  allocation order still varies, so traces compare values, not pointers.
- The asset loader (`AsyncLoadManager`, its own thread) took a varying
  number of frames: it raced the render thread for the render lock in real
  time, and the Rust update's different speed changed the count. In
  lockstep the harness now gives the loader its turn at each frame boundary
  (`loader_turn.rs`, `docs/harness.md` "Load pacing"), so a load takes the
  same frames on the original and the ports. Replays still wait for game
  state (`waitfor`) rather than counting frames, and battle logic is also
  compared by its call sequence (the call log).
- State from before lockstep (frame sync counters, game time, the VU0
  registers, the CRT seed) varies with how long boot and loading took; the
  replay command `pin` sets it to fixed values. `lockstep on` takes hold at
  the first boundary after the command, not at whichever one the main
  thread happens to be at.
- FMOD mixes on its own threads; with audio capture on, the harness runs
  `FmodManager`'s tick in a turn at each boundary, so the mix is the same
  from run to run (`docs/harness.md`, "Audio capture"). Music that starts
  before lockstep can't be pinned, only left out of the compared window.
- Threads of the game: the render thread (`PRenderThread`), the asset
  loader, an input task manager, two FMOD threads, the movie decoder's
  (libvpx, only with a movie), a Virtuos test-command thread.

## The PS2 emulation registers

Some of the PS2 code was hand-written R5900 and VU0 assembly; the PC port
turned it into C that keeps the registers in globals: `g_Ee_Gpr`
(`0x00c8f508`, 32 registers of 16 bytes, `sp` among them) and VU0's `vf00`
(`g_Vu0_Vf00`, `0x00c0a004`) and the rest of the VF file after the GPRs.
Callers pass arguments through them (`AsmEmuSetFuncParam0` and its siblings
write the argument registers), and 78 functions across battle, map, effects
and scripts write `t0`. For parity these registers are part of a function's
outputs: a Rust replacement must leave the same values in them as long as an
original caller may read one, and the bridge carries them until the last
emulated caller is gone. `tools/ownership.py global 0x00c8f508` lists who
touches them.

## Modding

Each module that moves into Rust gets extension points as it lands, not
afterwards:
- tables that can grow past the original sizes, keeping the original IDs;
- new script natives registered by mods;
- hooks around battle commands, damage and rewards;
- assets from mod folders (already).

The original's fixed array sizes stay only where save compatibility needs
them.

### Scripting for modders (Luau)

FFX's events, cutscenes, field logic and monster AI are Atel bytecode run by
the script VM (`Script_Run`, in Rust since M1), and everything a script does
goes through the VM's natives. Modders should not need Atel: the plan adds
Luau on top, in three stages (user's decision, 2026-10-17). Luau is
Roblox's open-source (MIT) dialect of Lua 5.1: the language most beginners
already meet (Roblox), with current docs and tutorials (luau.org), gradual
types for autocomplete and early errors, and a sandbox made for untrusted
scripts. It embeds from Rust (`mlua`, `luau` feature), and its coroutines
match Atel's model: workers that run a little each frame and wait on
signals, moves and messages. (FFX.exe links a Lua 5.2.1 of its own through
PhyreEngine, but never runs it and exposes nothing of the game to it:
`docs/formats/lua.md`. This is a new runtime, not that one.)

- **Prerequisite, from M1 on: a documented native API.** Every native that
  moves to Rust gets its name, its arguments and their meaning, and what it
  waits for, in `docs/formats/script.md`. One machine-readable API
  description is generated from that table, and from it the Luau type
  definitions and TypeScript declarations (below). This is part of each
  script round, not extra work.
- **TypeScript, optional.** Modders who prefer it write TypeScript and
  compile it to Luau with roblox-ts (MIT); the game still runs one runtime.
  It needs Node on the modder's machine, documented as a one-time setup.
  C# and Python are not offered: .NET is a large runtime that cannot
  sandbox untrusted code, and embedded Python is slow and hard to sandbox.
- **Getting started is part of the work.** A first-mod tutorial, small
  example mods to copy, the API reference with editor autocomplete, and
  errors that point at the mod's own line.
- **L1, Luau beside Atel (after M5, user 2026-10-27).** Mods
  ship Luau (or TypeScript) scripts that hook game events (map entered,
  worker started, signal sent, battle started, command chosen) and call the
  natives through a typed API (`chr.move_to`, `msg.show`, `wait.frames`).
  The original scripts stay as they are; a mod adds behaviour or replaces
  one worker's. Runs in the 64-bit program, on its function table.
- **L2, Atel to Luau (after M5).** A decompiler turns every shipped event
  and battle script into readable Luau (workers as coroutines, natives by
  name, variables by their descriptors), and the engine runs the Luau.
  Checked like everything else: a decompiled script must give the original's
  replay. This is what lets the community edit existing cutscenes and
  fights, not only add to them.
- **L3, Luau to Atel (optional, on demand).** A compiler back to bytecode,
  so an edited script also runs in the unmodified game. Only if there is
  demand; constructs without an Atel equivalent are refused.

## Risks

- **Float behaviour.** x87 intermediates and the VU-emulation semantics can
  make Rust results differ in the last bit. Replay parity shows where, and
  some functions may need x87-faithful arithmetic to match.
- **Timing.** Logic tied to the frame rate (30/60 fps paths) has to keep its
  exact frame behaviour.
- **Size.** About 17,000 functions. The tooling in M0 and the bulk names are
  what keep the cost per function low; without them this does not finish.
- **Facts only.** Fahrenheit, the PhyreEngine SDK and VBFExtract stay
  reference-only. We use Fahrenheit's names and findings by permission but
  none of its code, and confirm every SDK fact in `FFX.exe`. Every implementation comes from `FFX.exe`
  and the data.
