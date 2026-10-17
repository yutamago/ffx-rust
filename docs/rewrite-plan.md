# Rewrite plan

Goal: all of FFX's game code in Rust. The result runs without `FFX.exe`,
builds for 64-bit and for other platforms, and can be changed anywhere by
mods. It reads the player's own `FFX_Data.vbf` and `metamenu.vbf`; we ship
nothing of the game.

## Where we stand (2026-10-16)

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
  - Iggy (19 `.swf` screens): rebuild these screens natively. A Flash player
    for 19 screens would cost more than the screens.
  - `.webm` movies: an open VP8/VP9 decoder with Vorbis/Opus audio.
  - Steam: optional (achievements) behind a feature flag.

## What gets ported

Not every game function needs an exact port. `tools/classify.py` sorts the
17,986 functions of the game modules into five categories and writes
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
global any logic reads); then propagation (a function only debug code calls
is debug, callers outside the game modules included, and an unnamed function
between two debug methods of one class, such as the free camera's, is
debug; glue and platform pass one hop to unnamed callees, and any number of
hops inside the PC part when the callee calls no game logic: allocator
internals, container copies, renderer set-up; what only exception unwinding
reaches is platform). Last, a thin wrapper in the PC part (at most 64 bytes,
no global write, every call to glue or platform code) is that boundary even
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

### Two sides that grow toward each other

The renderer shows the pattern. It runs as a separate 64-bit process
(`ffx-render`), fed by the scene stream from the 32-bit game. Parts with
narrow, per-frame interfaces can move to that 64-bit side early:
- rendering (started);
- audio;
- input and window;
- the PC UI screens;
- movies.

Game logic stays in-process in the 32-bit game, because it calls and is
called thousands of times a frame, and shrinks there function by function.
At the end the logic crates move into the 64-bit host, and `FFX.exe` is no
longer loaded.

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

### M4: the 64-bit side takes the platform

The original look is preserved first (user's decision, 2026-10-17): the
renderer's original mode (the game's own shaders and draw state, no
enhancements) is the default and must match the original game; the
enhanced mode (PBR materials, new lighting, upscaling) is always an option
the player turns on. Each part the renderer takes over (scene, effects,
glow and post-processing, the UI, movies) switches from the old engine only
when its original mode passes:

- a draw-stream comparison: the draws the Rust scene builder makes (meshes,
  order, shaders, constants, state) against the ones PhyreEngine makes for
  the same frame, draw by draw;
- screenshot comparisons against the original game (`tools/shot-compare.py`
  and the replays), within the rounding that different graphics APIs bring.

- Effects (`ppp`) and the remaster renderer (`graphic*`) are rewritten into
  the scene stream: the Rust side builds the scene instead of reading it out
  of PhyreEngine's.
- Audio (FSB/FEV decoding, mixer), input and the window, movies, and the PC
  UI screens move into the 64-bit host. FMOD, Iggy and D3D11 are no longer
  used.

### M5: inversion

The main loop (`Sg`, `FFXApplication`) and the system module (archives,
saves, settings, language) move to Rust. The 64-bit host runs the whole game
from the Rust crates and no longer starts `FFX.exe`. Saves stay
byte-compatible with the original's.

Alongside M4 and M5: the Luau stages L1 and L2 (Modding, below).

### M6: ports

Linux and the Steam Deck first (wgpu and our platform layer already cover
them), then macOS, then whatever else. Controller mappings and the PC UI
screens need a per-platform pass.

## Determinism

Replays need the same inputs to reach the same state. Found so far:
- The game's clock follows real time; `--virtual-clock` makes it one frame
  interval per frame.
- Archive reads run on the thread pool (`FileSys_SubmitRead`); on the
  virtual clock they run at once on the game thread.
- The PS2 vsync interrupt is a thread calling its handler every 16 ms of
  real time (`createVsyncEmuThread`, `VsyncThreadImpl_run`); on the virtual
  clock the handler runs from `Present`, once per 1/59.94 s of game time.
- Heap addresses change between runs (Windows' bottom-up ASLR, and other
  threads allocating); `ffx-launch` turns ASLR off for the game, but
  allocation order still varies, so traces compare values, not pointers.
- The asset loader (`AsyncLoadManager`, its own thread) takes a varying
  number of frames. It cannot be awaited at frame boundaries: some of its
  tasks need frames to pass. Logic is not affected (a battle makes the same
  random calls in the same order either way), only timing, so replays
  wait for game state (`waitfor`) instead of counting frames, and battle
  logic is compared by its call sequence (the call log).
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
- **L1, Luau beside Atel (after M3, once the natives are in Rust).** Mods
  ship Luau (or TypeScript) scripts that hook game events (map entered,
  worker started, signal sent, battle started, command chosen) and call the
  natives through a typed API (`chr.move_to`, `msg.show`, `wait.frames`).
  The original scripts stay as they are; a mod adds behaviour or replaces
  one worker's. Runs in the hook first and in the 64-bit host after M5
  unchanged.
- **L2, Atel to Luau (with M4/M5).** A decompiler turns every shipped event
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
