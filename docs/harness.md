# Driving the game from tests

`ffx-hook` has a test harness: with `FFX_HARNESS=<port>` it listens on
`127.0.0.1:<port>` and takes one command per line (see
`crates/ffx-hook/src/harness.rs` for the list). WSL reaches it through the
mirrored network.

```bash
tools/run-game.sh --harness 27100 --background   # no focus stealing
tools/harness.py stats 'press cross 8' 'wait 60' "shot $(wslpath -w /tmp/a.bmp)"
tools/harness.py quit
```

## How it gets in

| Piece | Hook | Why |
|---|---|---|
| Frames | `IDXGISwapChain::Present` (vtable of the swap chain from `D3D11CreateDeviceAndSwapChain`, patched in the import table) | Counts frames; commands that touch game state or the GPU (`call`, `shot`, waits) run here, on the game thread, between frames |
| Buttons | `Input_KeyHeld` (`0x00629d50`), wrapped with a trampoline | `Virtuos_InputManager_updateInput` (`0x00632a30`) builds the PS2 pad word at `0x00ccb170` (previous frame at `+4`) by asking it about every binding. For each held button the first keyboard binding paired with it reads as held, so the game sees the same as a key press: exactly that button's bit |
| Keys | `IDirectInput8::CreateDevice` → device `GetDeviceState` / `SetCooperativeLevel` | The PC menus (Esc) read keys directly. The keyboard is made a background device, and an unacquired one reads as all keys up |
| Focus | `ShowWindow` | Background mode shows the window without activating it |
| Isolation | `GetDeviceState`, `XInputGetState` | In background mode the real keyboard, mouse and pads are ignored: background devices would otherwise see what the user types elsewhere |
| Dialogs | `MessageBoxA`, `MessageBoxW` | In background mode a game dialog is logged (`ffx-hook.log`, and an event `messagebox <text>`) and answered OK instead of appearing on the user's desktop |
| Events | `Phyre_PSerialization_PStreamFileWin32_openFile` | Every file the game opens, and which came from a mod |
| Randomness | `Rng_ClockSeed` (`0x00798950`), wrapped | `MsRandMize` (`0x00798890`) seeds the global generator (`0x00c42200`) and the 68 at `0x01135ee0` from the system time (`GetSystemTime` bytes XORed, plus 1). With `--seed N` it gets `N`; `ffx-harness` uses seed 1 unless a test says otherwise |
| Speed | `Phyre_PTimerWin32_GetTime` (`0x0042fd80`), wrapped; the main loop's `call [Sleep]` (`0x0062f3bb`) | The main loop (`Game_Main`, `0x0062ee50`) runs a frame (`0x0062e340`) once `DAT_00c30e88` × 1/59.94 s have passed on `Phyre_PTimerWin32_GetTime`, and sleeps away the rest. With `--fast` that one `Sleep` returns at once and adds its time to the clock: frames run back to back (461 a second instead of 30–60) while the game still sees one frame interval per frame. `ffx-harness` uses it by default |
| Sandbox | `SHGetFolderPathW` | With `--home DIR`, the Documents folder is `DIR`: the game reads and writes `GameSetting.ini` and its saves (autosaves too) there. `ffx-harness` makes `DIR` a fresh copy of the user's `Square Enix/FINAL FANTASY X&X-2 HD Remaster` before each launch, so tests can load the user's saves and never change them |

XInput is not used: the game creates XInput devices only for controllers
found while enumerating DirectInput devices at start-up
(`0x006240e0`), so without one it never calls `XInputGetState`.

## Input facts

`Input_BuildPadMap` (`0x006315c0`) pairs each of the 81 bindings
(`GameSetting.ini` `ffxKeyBinding`, defaults at `0x00c8a538`) with a pad
bit; `Input_KeyHeld` (`0x00629d50`) treats a binding ≥ 28 as Phyre key
`binding − 28` (keys 0–25 A–Z, 26–35 digits, 36 Esc, 49 Enter, 63–66
arrows; scan codes at `0x00c30e90`); smaller ones are pad buttons.

The pad word is the PS2 pad data with its two bytes swapped:

| Bit | Button | Default key |
|---|---|---|
| `0x1` / `0x2` / `0x4` / `0x8` | L2 / R2 / L1 / R1 | PgUp / PgDn / S / D |
| `0x10` | Triangle (menu) | V |
| `0x20` | Circle (confirm: the Japanese layout) | C, Enter |
| `0x40` | Cross (cancel) | X, Backspace |
| `0x80` | Square | Z |
| `0x100` / `0x800` | Select / Start | N / B |
| `0x1000` … `0x8000` | Up, Right, Down, Left | arrows |

Read back from the pad word with the keys held; Esc gives `0x1` too but
opens the PC menu.

## Two kinds of input

The game proper (field, battle, the PS2 menus) reads the PS2 pad word:
drive it with `press`/`pad` and button names. The Flash (Iggy) screens,
the title menu and the PC menu (Esc), read raw keys instead: drive them
with `key` (arrows `c8`/`d0`/`cb`/`cd`, Enter `1c`, Esc `01`).

## Observed

- Frames keep coming and input keeps working with the window in the
  background; screenshots are 1280×720 BGRA.
- Esc opens the PC menu; the arrows move its cursor and X closes it.
- The opening (campfire, credits) is the title event's attract scene; it
  runs about 4,800 frames, then the title menu (Load selected) appears. The
  title event opens `titl00` when the opening starts. Start (B) skips it, even
  as a 2-frame tap, once the opening is a few seconds in (title about 280
  frames later; `examples/skip_probe.rs`). Inputs are taps of a few frames:
  menus repeat a held direction.
- Title: Up arrow, Enter starts a new game; the Sphere Grid choice takes
  `press down` and `press circle` (confirm), then the Soundtrack choice
  (Arranged or Original).
- Load: the title's cursor starts on Load; the save list and its Yes/No
  are Flash screens (Enter, arrows). Slot 1 of the user's saves starts on
  the salvage ship deck (`cdsp0000`), in the field: the D-pad turns Tidus.
- Loading without the menus: `loadsave <slot|windows path>` runs what the
  Load menu runs once a save is picked (`docs/formats/save.md`);
  `screens::load_save_direct` (replay line `load direct <slot|path>
  [script]`) taps Start until the title proper runs and sends it there.
  The save's event opens about 3 s sooner (1.6-2.2 s after the first frame against
  4.3-5.2 s), with no key presses to lose, and the state matches the menu
  path's (the replays give the recorded originals' events and
  screenshots). The replays use it; `load N` (list entry N) stays.
- Battles start from the field without walking into one:
  `screens::start_battle(field, group, formation)` writes the request a
  random encounter makes (`MsBattleEncountExe`, `0x00780d10`) and raises its flag;
  `MsBattleProcess` (`0x00790c10`) loads it on the next frame. While the
  field is still loading, `MsBattleExeClear` (`0x00781530`, called by the
  event set-up and `AtelUpdateBattleStatus`) drops the request; with games
  side by side the field loads slower in frames than the replays wait, so
  `start_battle` raises the flag again every 30 frames while it is down
  and no battle has begun. The field table
  (`*0x0112a9c8`, count `0x0112a9d0`, 14-byte entries: map number, group
  offset, first formation, name) lists all 96 battle fields; entry 3 is
  `cdsp00`. Actors are `*0x011334d4 + index × 0xf90`; a monster's max HP is
  at `+0x594`, its HP at `+0x5d0`.
- `examples/mod_probe.rs` plays into the first scene with any mod file and
  reports whether the game read it and survived; it found that grown PS2
  files were cut at their original size (`docs/hooking.md`).

## Coverage mode

`FFX_COVERAGE=<file>` (passed on by `tools/run-game.sh`, `/p` in `WSLENV`)
makes `ffx-hook` put a one-time `int3` on the first byte of every function
listed in `<file>` (`<address> <first byte>` lines, from
`tools/coverage.py candidates`) whose byte no hook or patch owns. The
vectored handler (`log.rs`) restores the byte, appends the address to
`<file>.hits` and resumes the function. Keep `<file>` on the Windows drive:
the hits file is opened once per first hit. `tools/coverage-run.sh` runs
the replays and `tools/replays/coverage-session.txt` on every save in the
game slot and merges the hits into `symbols/coverage.txt`; the last run's
logs and outputs are in `~/ffx-work/agents/coverage/`.

## Hook-hit mode

Coverage mode skips every byte a hook owns, so it cannot say which ported
functions run. `FFX_HOOK_HITS=<file>` (a WSL path, best on `/mnt/c`;
`tools/run-game.sh` passes it on with `/p`, and the harness and replays
inherit it) does: after the hook table is installed, `ffx-hook`
(`crates/ffx-hook/src/hook_hits.rs`) puts a one-time `int3` over the `0xe9`
of every installed hook's `jmp`. The first call through a hook traps into
the vectored handler (`log.rs`), which puts the `0xe9` back, appends
`0x<address> <name>` to `<file>` and resumes at the `jmp`, so the shim runs
as usual. Hooks turned off with `FFX_HOOK_DISABLE` keep their original code
and are not armed.

- Cost: unset, nothing is armed and the shims, thunks and patch sites are
  those of a normal run (no flag or branch on any path). Set, each hook
  costs one exception and one append the first time it runs, nothing
  after. A replay gives the same events either way.
- Crash-safe: each hit is one `WriteFile` in append mode with no user-space
  buffer, so a game that dies leaves every hit it had, and lines from
  several threads never interleave.
- It counts hooks the game enters. A port that a module crate calls as Rust
  (not through the original's address) is not counted on that path.
- Don't take snapshots in this mode: the image holds the `int3`s that have
  not fired yet.

`tools/replays/run-all.sh --hits` runs and compares the replays with the
mode on (one hits file per replay in the run folder's `hits/`), then plays
`tools/replays/coverage-session.txt` on the Rust side for each save in
`FFX_HITS_SAVES` (default `0 1 2 3 5`), merges the hits into
`symbols/hook-hits.txt` (the union of every run, addresses only) and writes
the report (`FFX_HITS_REPORT`, default
`~/ffx-work/agents/coverage/hook-hits-report.txt`). `tools/hook-hits.py`
does the merging and reporting on its own:

```bash
python3 -I tools/hook-hits.py merge FILE.hits...
python3 -I tools/hook-hits.py report [FILE.hits...] [--only] [--out REPORT]
```

The report gives, per hook file (`crates/ffx-hook/src/ported/*.rs`) and per
module (`tools/progress.py`), the ported hooks that ran against those
hooked, in functions and bytes (body sizes from the Ghidra dump), the
biggest areas (name prefixes) that never ran, and every port that never
ran, biggest first.

## Snapshots and replays

Two tools for checking Rust replacements against the original code on real
game state (`docs/rewrite-plan.md`):

- `snapshot <windows path>` writes the game's memory (private regions and
  `FFX.exe`'s image, in 1 MiB LZ4 blocks; `ffx_bridge::snapshot`) on the
  game thread between frames. `ffx-oracle` loads it (`Oracle::from_snapshot`)
  and runs original functions on it; a field snapshot is 461 regions, 395 MB,
  61 MB on disk.

  ```bash
  cargo run --release -p ffx-harness --example snapshot -- 'C:\Users\…\Temp\field.snap' 1 120
  FFX_SNAPSHOT=~/ffx-work/snapshots/field.snap cargo test --release -p ffx-oracle --test snapshot
  ```
- Replays (`ffx_harness::replay`): a script of harness commands played
  twice, once with `--hook-disable replacements` (the original code) and
  once with the Rust replacements, each tracing hashes of chosen memory
  ranges every frame (`trace`). With `--virtual-clock` the game's clock
  never follows real time, and after `lockstep on` frames advance only as
  far as `wait` and `press` say, so the same inputs land on the same frames:
  two runs of `tools/replays/field-walk.txt` on the original code agree on
  every frame.

  ```bash
  cargo run --release -p ffx-harness --example replay -- tools/replays/field-walk.txt 'C:\…\orig.trace' original
  cargo run --release -p ffx-harness --example replay -- tools/replays/field-walk.txt 'C:\…\rust.trace'
  cargo run --release -p ffx-harness --example replay_diff -- /mnt/c/…/orig.trace /mnt/c/…/rust.trace
  ```

Loading takes a varying number of frames (the asset loader runs on its own
thread; see "Determinism" in `docs/rewrite-plan.md`), so scripts wait for
game state where it matters (`waitfor <range> <hex> [mask] [frames]`:
advances frame by frame until the bytes match; a battle is under way when
`0x0112a8e0` is 1), and `tools/replay-compare.py` compares two runs by
their events: each trace range as its sequence of distinct values, and the
call logs (`FFX_HARNESS_CALL_LOG`, written by Rust replacements) per
stream. `tools/replays/battle-attack.txt` gives the same events on the
original and with the Rust random numbers and kernel lookups:

```bash
cargo run --release -p ffx-harness --example replay -- tools/replays/battle-attack.txt 'C:\…\orig.trace' original
cargo run --release -p ffx-harness --example replay -- tools/replays/battle-attack.txt 'C:\…\rust.trace'
tools/replay-compare.py /mnt/c/…/orig.trace /mnt/c/…/rust.trace
```

The field replay's screenshots are compared coarsely (`shot-compare.py
--coarse field-walk`, `compare_coarse`). The deck's sway, light and
searchlight beams are not pinned down by the replay: the asset loader
takes a varying number of frames ("Determinism" in `docs/rewrite-plan.md`),
so the map's animations are at another phase when the shot is taken. The
sway is not a sub-pixel shift: over 47 runs (2026-10-17 to 2026-10-20) the
scene sat 0 pixels across and -9 to +6 down from the recording, in a few
discrete steps, with the near railing moving further (parallax, roll).
Runs that swayed 4-5 pixels scored 25-26 on the old median-shifted block
check (limit 25; effects rounds 32 and 36) although the geometry was the
same. The comparison therefore:

1. finds the one shift (up to 4 across, 16 down) that best lines up the
   scene below the top 240 rows, on gradients (light does not move them);
   a shift at the edge of that range fails as a wrong camera. The scene's
   four quarters are then lined up on their own (8 pixels around that):
   the right half sitting lower than the left (roll) or wider apart
   (zoom) by more than 4.5 pixels fails. The sway reached 3 (roll) and 2.5
   (zoom); a camera rotated by 1 degree gives 6.5 to 12.5, zoomed by 2 %
   6.5 to 11, both with the minimap left in place (0.5 degree, 4 to 5,
   mostly passes);
2. compares 20x20 blocks' mean colour per channel, after the median
   difference (the light step), each block allowed to match within 3
   across and 6 down of that shift;
3. gives each block the limit 13 + 1.4 x its mean gradient in the original
   (levels per pixel): edges turn a leftover shift into a difference,
   flat deck does not, so a missing model on the deck shows against a low
   limit. A block's limit is at least twice what the two original runs
   differ by there;
4. leaves out the searchlight beams: the top 240 rows, the sky behind the
   crane (field-walk-1 x 880-1280, y 240-400), the beam's end
   (field-walk-2 x 600-700, y 240-320 and x 1000-1120, y 240-280) and the
   save sphere's glow (field-walk-1);
5. compares the minimap (x 100-320, y 60-220) unshifted, limit 6: it does
   not sway, so a picture shifted as a whole (2 pixels: 6.9, 4: 14) or a
   missing UI layer (90) fails there.

On the field-walk shots of 47 past runs (94 shots, `~/ffx-work/fwcheck`)
every run since the 2026-10-17 recording passes, the worst at 0.73 of its
block limit (17.2 of 23.5: the round 36 and round 32 runs, 26.0 and 25.7
on the old check), roll and zoom at most 1.0, the minimap at most 1.0;
the 16 runs from before that recording, which all failed the old check
(31-58), pass too (0.92 of the limit, roll 3, minimap 5.4). A real run with
Tidus not drawn fails (48.7, limit 29.2). Synthetic breakage made from
real shots with pillow fails: a character removed (Tidus 43, limit 29;
Rikku 29, limit 26), garbled geometry and dark spikes on the deck (16-24
against 14-17), the camera zoomed or rotated or shifted with the minimap
kept, the minimap removed, the whole picture shifted by 2 pixels or more.
Not caught: a 0.5 degree camera rotation, and a camera shift within the
sway range (20 down on a run already swayed 5 up). Battle and menu screenshots stay
exact.

Since 2026-10-24 the field-walk shots are taken at a fixed point of the
event's own clock instead of a fixed frame count after the load: ctrl
work 0's frame count (`0x01325d94`, `g_AtelCtrlWorks` + 0x1f4, counted by
`Atel_ExecCtrlWorks`, zeroed by `AtelRequireCtrlWork`). The searchlight
sweep and the deck's sway start with the event, and the event starts
once the asset loader is done, so 120 frames after the load the count was
105-111 on an idle machine but 40-75 under load, and those runs scored
17-20 (scene shifted 0, -4 to -9) against a limit of 23.5. At the same
count every run gives the same picture (three runs, 13 counts: identical
scores). The script waits for the count to enter 64..127 (`waitfor` with
the mask `c0ffffff`) and then 37 frames, so the shot is at count 101,
which matches the recording (0.1; 98: 0.6, 110: 2.2); a count that skips
a value cannot be missed, and an event that never runs fails the replay
after 1500 frames (`replay::play` names the line and what the range
held). The walk takes a fixed 358 frames in lockstep, so the second shot
is at count 459. field-walk-1 before: 0.5-20.6 (median 2.2, 8 of 42 runs
above 15, the worst at 0.88 of its limit); after: 0.0-0.1 in all 18 runs
(9 of them beside other games and busy loops), field-walk-2 0.1. With the void
`ClassDynamicGeometry_SetShaderGl` shim of 2026-10-23 (no searchlight
beams, no light orb) field-walk-2 scores 103 against 34.4.
`examples/counter_probe.rs` lists the image's counters that count once a
frame, with how late they started after a load, for pinning other shots.

`tools/replays/boot.txt` (2026-10-24) checks what the other replays skip:
the boot before `load direct`. With no input the game shows the FINAL
FANTASY X PROJECT logo (map `0x15c`, `memochek`), then the opening (map
`0x15d`): credits on black, then the in-engine campfire scene (Zanarkand
ruins, the party, Tidus and Yuna close-ups) with credits, no FMV, about
4,800 frames; Start goes to the title menu (map `0x17`, previous `0x15d`).
The opening keeps time with its music stream, which plays in real time:
in fast mode the same event frame (`0x01325d94`) showed another camera
cut, credit and pose in every run, original code included (three original
runs at count 1200: three different pictures). The script therefore says
`realtime` (`replay::realtime`: no fast mode, no virtual clock, no
lockstep) and pins its four shots to ctrl work 0's count (101 on the logo,
900 and 1200 in the campfire scene, 101 on the title). At the game's own
rate two original runs agree within 9,000-32,000 pixels (a frame of
jitter) and the Rust run matched both (0 pixels). It has no trace ranges;
`run-all.sh --record boot` records it alone (`--record NAME...` leaves the
other replays' recordings alone). About 75 s, the longest replay.

`tools/replays/battle-magic.txt` (2026-10-24) plays the one spell
overlay that calls `op_shape_draw` (`0x007ed0c0`): `magic_0481.dll`, its
only caller (`g_MagicImports` slot 242). No command, item or monster
ability plays magic 481: it is an event effect that waits for the event
script's effect flags (`AtelGetEffectFlag` 0..11 and 15, `g_AtelEffectFlags`
`0x01326b70`). The replay starts battle-attack's battle, points the first
item record's effect IDs (`+0x10`, `+0x12` underwater; `MsGetMagicEffNum`)
at 481 with `writep` (`*addr + offset`, like `readp`), has Rikku use it
and sets the flags itself, one more every 60 frames, from the frame the
magic file is loaded (`g_MagicFile_Handle` `0x00c64cac`). Coverage mode
(`FFX_COVERAGE`, `0x007ed0c0`) gives no hit up to flag 5 + 90 frames and a
hit once flag 6 is set: shot 1 is the stage before (flag 5), shots 2 and 3
the burst flag 6 starts (10 and 30 frames in). The effect looks the same in
every run; the characters' idle swim pose under it does not (the battle's
load time moves its phase), so `shot-compare.py` is blind there like on
battle-attack-1: look at the shots. Traces: the magic ID, the flags and the
actors' HP (the item heals Rikku at the end). About 17 s.

`tools/replays/battle-overdrive.txt` (2026-10-24) plays an overdrive with
its minigame and a summon: Tidus's Spiral Cut with the timing bar hit,
Yuna summons Valefor (entry sequence, party swap), Valefor's overdrive
Energy Blast, Valefor dismissed (the party comes back), and Wakka's menu.
It starts from save slot 5 (story 1470, Valefor, Ifrit and Ixion joined)
in a land battle, `battle 33 0 3` (`genk16_03`, the Djose highroad: Ochu
and Garm), and forces the rest with `write`/`writep` before the first
turn: the battle party (`g_SaveInParty` `0x011307e8`, `MsGetSaveInParty`:
Yuna, Tidus, Wakka), the overdrive gauges (actor `+0x5bc`,
`MsGetChrLimitGauge`; `+0x5bd` the max) of Tidus (100) and Valefor (actor 8,
20), and the Ochu's HP and max (99999), so that Energy Blast leaves the
battle running for the dismissal. The script waits on game state, not
frame counts, wherever the loader's timing shows: whose menu is open
(`g_BtlWindowOpen` `0x023cc08c`, the window character's bit), and each
effect's magic ID and file (`0x00c64ca0`, `g_MagicFile_Handle`
`0x00c64cac`). The timing bar (`TOBtlCtrlTidusLimitWindow`) is hit the
same way every run: in its state 4 the marker (`0x0133d742`) moves by 11
from 118 over 275, the hit zone is 243..267 (at 1280x720), and the script
presses circle once the marker reads 250, so the check sees 250 (or 261 a
frame later). With a longer script (dismiss, two attacks, summon again)
the Ochu KO'd Yuna before her second summon (in a shorter trial it only
hurt her): the monsters' choices follow the frames between the events, so
after changing a wait, check the outcome in the shots again.
Traces, from Tidus's first menu on (it opens on the frame the phase byte
turns 1 in two of five original runs, a frame later in the others): the
magic ID and the file handle's low half (the high half is the module
base), the bar's result (`0x0133d6f3`; its states 3 and 5 last a frame and
one original run skipped 3), the window
bits, `g_BtlSummoner`, the two gauges, the HP of Tidus, Yuna, Wakka,
Valefor, the Ochu and the Garm, and the battle phase. Its nine shots (the
bar, Spiral Cut, Valefor's entry twice, Valefor's menu, Energy Blast twice,
the dismissal, the party back) are checked by eye only: the overdrive's
lightning, the fireflies and the idle poses differ in a few hundred to a
few thousand pixels between any two runs, original or Rust (shots 1 and 5
are only 30% noise, so the exact comparison counted the rest and failed;
the coarse field comparison failed on two original runs too).
`run-all.sh` records them into `orig/eye` and `orig2/eye` (its `eye`
list), out of `shot-compare.py`'s sight, and prints where the Rust run's
are. The effects' phase at a shot varies by a few
frames even between original runs (Valefor's entry took 327-348 frames
from its magic ID to its end in three original runs, 319 in three Rust
runs; Energy Blast's damage came 268-284 frames after its ID, 264-275 in
Rust), so shot 6 shows the dark summoning circle in some runs and the
white flash after it in others. About 25 s.

`tools/replays/menu-browse.txt` opens the main menu and takes screenshots
at fixed frames. Some of the screen animates by the wall clock (the
character panels' shimmer), so two runs of the original already differ in
about 27,000 pixels; `tools/shot-compare.py ORIG ORIG2 RUST` (three
directories of the screenshots: the original twice, then the Rust run)
counts only the pixels the original draws the same both times, a few
pixels around each animated one left out.

`tools/replays/run-all.sh --record` records the original runs (the menu's
twice), `tools/replays/run-all.sh` replays everything with the Rust
replacements and compares. A recorded original run can itself be off (one
field-walk recording had other party records from the first frame on,
which no later run reproduced): when a comparison fails, record the
original again before suspecting a replacement. All three replays give the
original's events and screenshots with every replacement in (1593
functions, 2026-10-16); the menu screenshots keep a few dozen pixels on the
animated cursor that the two original runs happen to agree on, so
`tools/shot-compare.py` leaves out 12 pixels around every animated one
(the cursor icon is about 25 pixels wide); with that, 0 pixels differ.

`fpucw` answers the game thread's x87 control word: `0x027f` (53-bit
precision, round to nearest, all exceptions masked).

## Parallel games

`Game_Main` (`0x0062ee50`) takes the named mutex
`9B8F7468-6DE0-4C5A-9209-89D363F8FCC9`; when it already exists the game
shows "Same program is already running." and ends. With the harness on, the
hook replaces the `CreateMutexW` import and gives that one mutex a name per
port (`…-harness-<port>`), so harness games run side by side; without the
harness the game is unchanged. `quit` ends the process with
`TerminateProcess`: running the game's own shutdown from the harness thread
once deadlocked, and that process, stuck while exiting, kept its mutex (and
its harness port) until it finally went away.

Each game runs in a slot of a pool (`crates/ffx-harness/src/slot.rs`),
shared by all worktrees in `~/ffx-work/game-slot`: `FFX_GAME_SLOTS` slots
(default 3, see the measurements below). Slot `n` selects

| | |
|---|---|
| game folder | `~/ffx-work/instances/<n>` (`FFX_INSTANCES_DIR`) |
| harness port | `27100 + n` (`FFX_HARNESS_PORT` is the base) |
| Documents folder | `~/ffx-work/harness/<n>/home` (`Options::home`, `Home::Instance`) |
| output folder | `~/ffx-work/harness/<n>` (`FFX_HARNESS_DIR`; `Game::out_dir`, screenshots) |
| lock | `~/ffx-work/game-slot/lock-<n>` |

An instance folder (`tools/instance.sh <n>`, called by `tools/run-game.sh
--instance <n>`) is the game folder made of hard links (`cp -al`: same
filesystem, no space; about 3,200 files, 0.2 s). Windows loads `FFX.exe`,
its DLLs and the archives from such a tree over the WSL share like from the
game folder. The links are rebuilt only when the game folder's listing
(names, sizes, times, inodes) changes; `mods/` is linked afresh at every
launch, with each `Options::mods` folder (`--mod DIR`) added as
`mods/<name>`. Files a launch installs or something writes in place are
never links: the instance's own `DINPUT8.dll` (the caller's build, replaced
at each launch), `ffx-hook.log` (`Game::log_path`, `Game::new_log` for this
launch's lines), `ffx-launch.exe`, the renderer's files (`ffx-render.toml`
copied at each launch). Tests that change mods pass them in `Options::mods`
instead of writing into the game folder, so games side by side never see
each other's.

`Options::home` (default `Home::Instance`) is made afresh at each launch:
a copy of the user's settings and saves (`copy_user_data`), then
`Options::saves`, `(name, file)` pairs put into its `FINAL FANTASY X`
folder (`ffx_001` is load-screen entry 1), so a test can start from a
given save with `copy_user_data: false`. A home inside the user's real
Documents folder is refused.

`Game::launch` takes a ticket, waits until a slot is free (saying so on
stderr), holds that slot's lock while the game runs and frees it when the
`Game` is dropped; a process that dies frees its ticket and lock with it.
The oldest `N` waiters may take any free slot. Every slot holder also holds
`~/ffx-work/game-slot/lock` shared, and whoever wants the whole game (a
checkout from before the pool, `tools/run-game.sh` without `--instance`)
takes it exclusively: they wait for each other. `FFX_GAME_SLOTS=1` gives
back one game at a time.

`tools/run-game.sh --instance N` refuses to start when a game runs from the
same folder, or when someone else holds that slot. Without the harness it
refuses while any game outside the instances runs (or one with no path:
stuck while exiting), since both would want the real mutex.

From the shell:

```bash
cargo build --release -p ffx-harness --bin game-slot
target/release/game-slot status              # slots in use, queue
target/release/game-slot run -- <command>    # waits, runs it in a slot
```

`game-slot run` sets `FFX_GAME_SLOT_HELD=1` and `FFX_GAME_SLOT=<n>`; the
games the command launches then use slot `n`, one after the other (scripts
that want that source `tools/in-game-slot.sh`). `tools/replays/run-all.sh`
and `tools/game-tests.sh` no longer take one slot for everything: each
replay or test takes its own, so they run side by side (`run-all.sh` with
`FFX_REPLAY_SERIAL=1`, `game-tests.sh` inside `game-slot run`: one after
the other).

A test stopped mid-run leaves its game running; the next `Game::launch` in
that slot ends it with the harness's `quit` (only games that answer on the
slot's port: a game the user plays is never touched). A detached game
(`Game::detach`) outlives its slot. Run long game tests as background
commands: they wait for their turn and finish later.

Replays write their screenshots to `$OUT`, the trace's folder (the
scripts say `shot $OUT\field-walk-1.bmp`). `run-all.sh` gives each call
its own folder, `%TEMP%\ffx-snap\runs\<id>\<tag>\` (traces, screenshots,
each replay's output with the hook's faults), and keeps the recorded
originals where the comparisons read them, outside Temp (which Windows may
clean): `~/ffx-work/replay-shots/traces/<replay>-orig.trace`, `-orig2.trace`
and `~/ffx-work/replay-shots/orig`, `orig2` (`FFX_REPLAY_SHOTS`). It keeps
only the newest `KEEP_RUNS` (20) run folders, so `%TEMP%\ffx-snap` can be
deleted at any time.

Test games play their sound at 5% master volume (`FFX_TEST_VOLUME`,
0-100), set in each slot's own `GameSetting.ini`, never the user's.

Measured 2026-10-17 (16 threads; the host also ran other agents' builds,
so CPU is the whole machine's load, sampled every 2 s):

| Games side by side | Wall | Host CPU | `FFX.exe` memory | Free memory (min) |
|---|---|---|---|---|
| 1 | 26 s | 81 % | 0.6 GB | 9.4 GB |
| 2 | 24–25 s | 87–88 % | 1.0–1.1 GB | 8.4 GB |
| 3 | 33 s | 89 % | 1.6 GB | 7.1 GB |
| 4 | 27–40 s | 82–91 % | 1.9–2.4 GB | 6.0 GB |

One replay takes about 22 s alone. Three side by side take 33 s, four 33
to 40 s, but with four one of three runs lost a key press on the load
screen (a Flash screen, timed by the wall clock) and loaded the autosave
instead: hence 3 slots by default. `run-all.sh` took 52–100 s for its
replays one after the other and 24–36 s side by side (whole script with
the builds: 73–124 s against 45–61 s).

Side by side or not, the comparisons give the same results: every
difference seen, in 9 serial and parallel runs, was one known to come and
go in the game folder too: the pad word range (`0x00ccb170`) of
`field-walk` or `menu-browse` missing one frame's transition value (a
frame where only the previous pad word still holds the button), and a few
dozen pixels on the menu's animated cursor. They come from neither the
instance folders nor the pool; `run-all.sh` now leaves the pad word out
(`replay-compare.py --ignore`) and `shot-compare.py` allows 24 levels per
channel. With those, three more runs (two side by side, one
serial) gave the same events everywhere; one side-by-side run still failed
on screenshots that follow the wall clock: the field's searchlight block
at y 120 (38 levels, as a serial run in the game folder once measured) and
5 pixels of the menu cursor. Games side by side run slower in real time,
so wall-clock animations drift a little further.

## Tests

`crates/ffx-harness` launches the game (`Game::launch`, background mode)
and wraps the commands; a launched game is ended when the `Game` is
dropped, so a failing test does not leave it running. The game tests are
ignored by default because they open a window on the Windows desktop.
Starting and loading cost more than the checks, so each test shares one
launch between several checks; the tests run side by side, each in its own
slot, and `tools/game-tests.sh` runs only the
tests the changed files concern (since the merge base with `origin/main`,
plus uncommitted and untracked files):

```bash
tools/game-tests.sh --list     # which tests, and the file that selected each
tools/game-tests.sh            # run them
tools/game-tests.sh --all      # all four (about 2.7 minutes)
tools/game-tests.sh HEAD~3     # compare with another commit
```

| Test | Checks | Selected by changes to |
|---|---|---|
| `start_up_input_and_kernel_mod` | the keyboard is read without focus; a mod's `item.bin` (Potion power 77) is what `g_KernelItem` points at, not a later mod's switched off by `mod.toml`; New Game to the Sphere Grid choice, where Down moves the cursor and the pad word holds exactly Circle | `ffx-kernel`, `ffx-text` |
| `first_scene_with_script_model_and_motion_mods` | into `znkd0600` with four mods: the event script grown by `pushimm 0x1234; popv v1` (its byte global at `0xa3c` reads `0x34`), its English dialogue with a changed first line (the game's message table, `*(*0x01326b28 + 0x34)`, holds the mod's bytes), Tidus's model 30 % wider with a segment doubled and a new segment (new skeleton matrix, new material on Yuna's texture, his asset list naming it), his motions turned; all read from the mod, the scene runs 10 s | `ffx-script`, `ffx-text`, `ffx-phyre`, `ffx-motion` |
| `save_and_battle_with_monster_and_formation_mods` | slot 1 of the sandboxed saves with three mods: the encounter table's salvage ship group (rate 61, weights 3 and 1) is what the game holds once the map is entered (current weights copied into the high nibbles); a battle started directly (field 3, `cdsp00_00`) has the modded formation's monsters (m068, m070, m070), and m068 the mod's 12345 HP; Tidus's and Rikku's worn weapon and armor (kernel equipment layout) give their actors' formula, power and crit bonus; the user's own settings and saves are byte for byte unchanged | `ffx-monster`, `ffx-battle`, `ffx-kernel` |
| `random_generators_repeat_with_a_seed` | three save loads: seed 1 twice gives identical generators, seed 2 others | — |

Changes to the hook, the harness, `ffx-core`, `ffx-vbf`, the launcher or
the workspace select all four; other changes (docs, the CLI, symbols)
none.

Screens are recognised by mean colours of fixed rectangles; a failing
check saves its screenshot to `~/ffx-work/harness/`.
