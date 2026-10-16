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

Outside the exe, the game loads `fmodex.dll` and `fmod_event.dll` (FMOD Ex),
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

### M3: field, events, characters, menus

Map loading, walk meshes and collision, guide map (`rc*`, `Yn`, `Gu`);
characters and motions (`Ch`; motion formats are done); the remaining Atel
natives; menus and text (`Tk`). It is done when walking, events and the
menus of a save run on Rust logic and replay identically.

### M4: the 64-bit side takes the platform

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

### M6: ports

Linux and the Steam Deck first (wgpu and our platform layer already cover
them), then macOS, then whatever else. Controller mappings and the PC UI
screens need a per-platform pass.

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
