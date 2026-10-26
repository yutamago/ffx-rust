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
| Frames | `FFXApplication_animate` (`0x0042f520`), wrapped; `Phyre_PRendering_PRenderer_flip` (`0x005b6af0`) and `Renderer_PresentFlip` (`0x00593760`), wrapped; `IDXGISwapChain::Present` (vtable of the swap chain from `D3D11CreateDeviceAndSwapChain`, patched in the import table) | The frame boundary is the entry of the game's update, on the main thread: the trace samples there, and `call`, waits, `loadsave`, `snapshot` and `fpucw` run there, between two updates; lockstep holds the main thread there. Screenshots are taken in `Present` on Phyre's render thread, of the image of a known update ("Frames and threads" below) |
| Buttons | `Input_KeyHeld` (`0x00629d50`), wrapped with a trampoline | `Virtuos_InputManager_updateInput` (`0x00632a30`) builds the PS2 pad word at `0x00ccb170` (previous frame at `+4`) by asking it about every binding. For each held button the first keyboard binding paired with it reads as held, so the game sees the same as a key press: exactly that button's bit |
| Keys | `IDirectInput8::CreateDevice` → device `GetDeviceState` / `SetCooperativeLevel` | The PC menus (Esc) read keys directly. The keyboard is made a background device, and an unacquired one reads as all keys up |
| Focus | `ShowWindow` | Background mode shows the window without activating it |
| Isolation | `GetDeviceState`, `XInputGetState` | In background mode the real keyboard, mouse and pads are ignored: background devices would otherwise see what the user types elsewhere |
| Dialogs | `MessageBoxA`, `MessageBoxW` | In background mode a game dialog is logged (`ffx-hook.log`, and an event `messagebox <text>`) and answered OK instead of appearing on the user's desktop |
| Events | `Phyre_PSerialization_PStreamFileWin32_openFile` | Every file the game opens, and which came from a mod |
| Randomness | `Rng_ClockSeed` (`0x00798950`), wrapped | `MsRandMize` (`0x00798890`) seeds the global generator (`0x00c42200`) and the 68 at `0x01135ee0` from the system time (`GetSystemTime` bytes XORed, plus 1). With `--seed N` it gets `N`; `ffx-harness` uses seed 1 unless a test says otherwise |
| Job seeds | the `push <job>` of `JobSchedule_Ch_Cloth_PerCharacter` (`0x00a44b51`) and `JobSchedule_Ch_CalcMiddle_Batch` (`0x00a44ba4`) | The cloth's wind (`Sg_ClothStep` `0x00822de0`), the mouths (`Ch_Mouth`) and the texture animations, eyes included (`chr_texanim_main` `0x0077c8d0`, from `Ch_CalcMiddle_Batch` `0x00833530`), draw from the CRT `rand()`, whose seed is per thread (MSVCR110's `_getptd()` `+0x14`), in jobs that run on whichever JobWorker is free, so they differed between runs (the cloth in every draw capture). The harness queues its own jobs, which seed `rand` for the job from its kind, its first character's place in the character array and ctrl work 0's event frame count at the last frame boundary, run the game's job and give the worker its seed back (original code and Rust alike; `harness.rs` `seeded_job`) |
| Speed | `Phyre_PTimerWin32_GetTime` (`0x0042fd80`), wrapped; the main loop's `call [Sleep]` (`0x0062f3bb`) | The main loop (`Game_Main`, `0x0062ee50`) runs a frame (`0x0062e340`) once `DAT_00c30e88` × 1/59.94 s have passed on `Phyre_PTimerWin32_GetTime`, and sleeps away the rest. With `--fast` that one `Sleep` returns at once and adds its time to the clock: frames run back to back (461 a second instead of 30–60) while the game still sees one frame interval per frame. `ffx-harness` uses it by default. On the virtual clock (`--virtual-clock`) the clock starts at 2^20 s (`VIRTUAL_START`, not the wall clock's FILETIME reading of about 1.34e10 s, whose f64 rounding differed in every run) and the `Sleep` moves it to the next frame's time exactly (`g_AppStartTime` + `g_FrameLimiterLast` + one interval, plus 1e-9 s) instead of the whole ms the limiter chops it to, so every frame is one interval long and time differences repeat (`docs/drawdiff.md`, "Noise in cdsp0000") |
| Sandbox | `SHGetFolderPathW` | With `--home DIR`, the Documents folder is `DIR`: the game reads and writes `GameSetting.ini` and its saves (autosaves too) there. `ffx-harness` makes `DIR` a fresh copy of the user's `Square Enix/FINAL FANTASY X&X-2 HD Remaster` before each launch, so tests can load the user's saves and never change them, with the sandbox's own `GameSetting.ini` in place of the user's ("Sandbox settings" below) |
| Loader turn | `AsyncLoadManager_threadMain_00691360` (`0x00691360`), wrapped; `Phyre_PMutex_lockInner_00431850` (`0x00431850`): its port asks the turn first, or a wrapper does around the original when the port is off | In lockstep the asset loader takes the render lock only in a turn at each frame boundary, until its requests are done, instead of in the real-time gaps between the render thread's holds, so a load takes the same number of frames whatever the update's real time ("Load pacing", `loader_turn.rs`) |

## Frames and threads

The game updates on the main thread and draws on Phyre's render thread
(`Phyre_PRendering_PRenderThread_threadEntry`, `0x00616010`). Each pass of
`Game_Main`'s loop (`0x0062ee50`) runs `App_Frame` (input) and
`Phyre_PFramework_PApplication_frame` (`0x00627940`), which calls the
update, `FFXApplication_animate` (`0x0042f520`: `updateFFX` `0x00822840`,
`Sg_MainLoop` `0x00820ae0` with `Sg_FrameSync` `0x00821e80` inside, unless
the PC menu is up), then the frame's render commands and
`Phyre_PRendering_PRenderer_flip` (`0x005b6af0`), which queues a flip
command (`0x16`). The render thread runs the queued commands (a ring of 16
slots) and, for the flip, `Renderer_PresentFlip` (`0x00593760`), which
calls `IDXGISwapChain::Present`. So the render thread draws and presents
update `n` while the main thread already runs update `n + 1`.

Until 2026-10-24 the harness did everything in `Present`, on the render
thread: the trace caught the main thread's next update half-done
(one-frame in-between values, event-agency below), `call` ran game code
beside the main thread, and lockstep held only the render thread. Since
then (harness-main-thread):

- **Frame `n`** (`frame`, `wait`'s replies, the trace's first column) is
  the boundary after `n` updates: the entry of `FFXApplication_animate`,
  wrapped. There every update before has finished, the next has not
  begun, and the previous frame's flip is queued. The trace samples there,
  and the main thread's jobs (`call`, `wait`, `press`'s release,
  `waitfor`, `loadsave`, `snapshot`, `fpucw`, `pin`) run there.
- **Lockstep** holds the main thread at the boundary until the script
  allows more updates: one update per frame of `wait`, `press` or
  `waitfor`, as before, and now nothing of the next update runs while the
  script reads, writes or calls. The render thread is held too (below);
  the JobWorker threads have no work then (the update hands them theirs).
  The asset loader and the other threads keep running.
- **The virtual vsync** (`--virtual-clock`) runs at the boundary, just
  before the update, instead of in `Present`; before the first update (the
  start-up screens) it still runs from `Present`.
- **Screenshots** show the update the boundary follows. The flip wrapper
  notes, for each queued flip, the frame it was queued at (the update it
  draws); `Renderer_PresentFlip` tells `Present` which that is. `shot`
  asks for the image of frame `n` (the current one) or a later one, and in
  lockstep the present of update `n` waits until the main thread has left
  boundary `n`, so a shot taken while the main thread is held at `n` is
  that image. The reply says which: `ok 1280 720 update 435`. A shot thus
  shows the same update the trace and `waitfor` saw at that frame: a shot
  after `waitfor` on ctrl work 0's count `101` shows the update in which
  the count became 101. Before, the shot was the image of the update
  before (count 100, "PhyreEngine draws frame N while the main thread
  computes N + 1"), so every recorded screenshot moved by one update and
  the originals were recorded again. In real time (no lockstep) a shot is
  the first image of the current update or a later one.
- `Game::launch` is ready at the first present (`stats`' `presents`), not
  at the first update: the memochek logo, where `load direct` presses
  Start, starts with the first update and in fast mode lasts about 120
  frames.

Every flip and present is counted (`stats`: `presents`, `flips`,
`flips_run`); in every run so far the three agreed, the start-up screens
included, and the log says so if a present ever comes without a flip's
label. A present waits at most 5 s for the main thread to reach its
boundary (a main thread that waits for the render thread there would
deadlock otherwise; none is known), and logs it.

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
`COVERAGE_ALL=1` arms every function of the dump instead (55,871, the
statically linked `PhyreEngine` and CRT included; `coverage.py candidates
--all`) and leaves `symbols/coverage.txt` alone; `COVERAGE_REPLAYS` picks
the replays, `COVERAGE_DIR` the folder. `tools/coverage-phyre.py <hits>`
groups such a run's hits by module and `PhyreEngine` subsystem (the M4
pilot's measurement, `~/ffx-work/m4/pilot-report.md`). Every launch arms
all candidates again, so each run's own hits are the lines it appends:
`coverage-run.sh` keeps them as `<tag>.hits` in the folder too.
`tools/coverage-new.py DIR TAG... [--pilot HITS] [--out FILE]` lists the
platform functions (the `phyre` module, and the `glue` and `platform`
classes) those runs reached that no `ported!` line hooks and no M4 list
(`~/ffx-work/m4/port/*.list`) holds, by module, with the first run (in the
order given) that reached each. Each hit is one
write since 2026-10-25 (before, `writeln!` could split a line and other
threads' lines landed inside it).

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
  after. A replay gives the same events either way. (Until 2026-10-24
  menu-save's save and load copies showed an in-between state for a frame
  with the slower first calls, and `--hits` left those ranges out: the
  render-thread trace's doing, see menu-save below.)
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

The event-script natives are mostly invisible to this mode (rp-events2,
2026-10-24). The interpreter runs in Rust (`Script_Run`), and its native
dispatch (`run::script_call_native` / `_poll_` / `_finish_`) looks each
table entry up in `native::find` (and the per-module `find`s it chains to)
and calls the Rust version directly, so the native's own hook, and its
`int3`, is never entered. A native counts only when other code calls it
through its address: `Native_MsgOpen` and `Native_MsgSetPos` by
`AtelOpenChoiceWindow`, for instance. In a Rust run of
`event-agency.txt`, which calls `Native_MsgSetText` for every message, the
first byte of `Native_MsgSetText` (`0x00857870`) still held the unfired
`int3` after the book's text was on screen. The `Native_` line of the
report ("never ran") therefore lists natives the replays do run; coverage
mode on the original code (an `int3` on every hooked port's first byte
with `--hook-disable replacements`) counts them. Run that way, the ten
replays and the five coverage sessions reached 4,950 hooked ports, 432 of
them natives, where the hook-hit union of the same runs has 3,512; the
candidates file is every `ported!` address with its first byte
(`~/ffx-work/rp-events2/natcand.py`, `covrun.sh`, `covattr.py`).

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

Loading took a varying number of frames (the asset loader runs on its own
thread; see "Determinism" in `docs/rewrite-plan.md`). Since 2026-10-26 the
loader's turn makes a load take the same frames in lockstep ("Load
pacing" below), but outside lockstep it still varies, and scripts still
wait for game state where it matters (`waitfor <range> <hex> [mask] [frames]`:
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
--coarse field-walk`, `compare_coarse`). Until 2026-10-24 the deck's sway,
light and searchlight beams were not pinned down by the replay: the asset
loader took a varying number of frames ("Determinism" in
`docs/rewrite-plan.md`), so the map's animations were at another phase
when the shot was taken. The shots are now pinned to the event's clock
(below); the coarse comparison stays. The
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
lockstep) and pins its four shots to ctrl work 0's count (108 on the logo,
908 and 1208 in the campfire scene, 108 on the title). At the game's own
rate two original runs agree within 9,000-32,000 pixels (a frame of
jitter) and the Rust run matched both (0 pixels). It has no trace ranges;
Until 2026-10-26 each shot was `wait N` after the count entered a range,
then `shot`: without lockstep the game runs on while the replies go back
and forth, so the image came 3 or 4 updates after the wait's end, and
boot-2 failed by 67,860 pixels about once in 20 runs (the one measured: 3
updates, against 4 in the other 20). Now each shot is `shotwhen <range>
<value> <mask> <frames> <path>`: a `waitfor` whose job, at the first frame
boundary where the bytes match, queues a shot of exactly that update;
while one is pending the present of update `n` waits for the main thread
to leave boundary `n`, as in lockstep, so the image cannot be presented
before the shot is queued. The windows are 4 counts wide (mask
`fcffffff`), so a count a slow frame skips still matches (the next
count's picture). A `shotwhen` that matches at the first boundary it
checks fails, because that image may already be gone; start it while the
count is still well below its window.
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
the burst flag 6 starts (10 and 30 frames in). Since load pacing and
`pin` ("Pinned state", 2026-10-26) the shots are the same to the pixel in
every run, original and Rust, and `shot-compare.py` compares them exactly;
before, the idle swim pose, the bubbles and the burst's pattern moved with
the updates run before lockstep. Traces: the magic ID, the flags and the
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
are. About 25 s.

Summon timing (2026-10-24, valefor-timing). Measured from its magic ID,
Valefor's entry took 321-350 frames in original runs and exactly 319 in
every Rust run of `run-all.sh`. That is loading, not a port bug. The magic
file's read (`g_BtlEffectReadClear` `0x00c421e8`, counted up each frame by
`Sg_EffectReadPoll` from 0 and set to -1 when it ends) waits for the
file's textures (`SegmentList_AllReady`; the DLL itself is loaded at
once, `g_PrxState` 3 on the ID's frame). It took 38-67 frames in the
original and 10-38 in Rust (with the dense trace below, 140 and 223 in the
original, 23 and 25 in Rust). The effect then starts when Valefor's
effect state (`MsEffectStart`, actor 8 `+0xdfb`) moves from 3 to 4,
which also waits for the summoner's motion (state 3's conditions): never
before ID + 37. Original runs always load later than that, so they start at
load + 1. Rust runs mostly load sooner and start at ID + 37, so 37 + 282 =
319. Per-frame probes (`~/ffx-work/valefor-timing`: actor 1 and 8 records,
`0x0112a8e0`-`0x0112cae0`, the magic and read words, every frame in
lockstep), aligned at the load's end, gave the same change frames for
every word from load + 9 to the entry's end in two original and two Rust
runs, and the same values for Valefor's per-frame words (`+0xf34`-`+0xf4c`).
Before load + 9 only the idle pose differs. Two Rust probe runs that loaded
slowly (42 and 50 frames) took 325 and 333 frames, in the original's range.
Turning off any one module's hooks (`FFX_HOOK_DISABLE`, each
`ported/<module>.rs`, two runs each) left Rust's loads at 10-36 frames
and the entry at 319, so the faster load has no single port as its cause.

So the replay waits for state 4 (`waitfor *0x011334d4+0x8a7b:1 04`)
instead of the file handle. The shots after it (3, 4, 6, 7) show the same
moment of the effect in every run (the two recordings differ in about 350
pixels on shots 3 and 4). The entry's camera still varies: it looks at
Valefor's head from the left in some runs and at Yuna from behind Valefor's
wing in others (shot 4: the ground, or the forest path), with the original
code too (another original run at the same frame matched the Rust run's
angle to 558 pixels, and both differed from the recordings by 457,000).
The script also checks the effect's length
within a frame: the magic ID must still be set 280 frames after state 4
and -1 one or two frames later (281 and one frame until 2026-10-26, see
below); Energy Blast's must be set 257 frames after its state
4 and -1 one or two frames later. Four runs each side gave 282 and 259
(from the ID to state 4: original 41-55 and 66-80, Rust 37 and 49-59) when
`waitfor` still sampled on the render thread. Sampled between two updates
(2026-10-24, "Frames and threads"), the entry still takes 282 in every
run, but Energy Blast took 258 in two original runs (recorded side by side
with the other replays) and over 258 in three run alone (259 in the one
measured to the end): the frame its state 4
shows up at seems to follow the loader's real time (the effect waits for
its textures), and the old sampling hid that.

On 2026-10-26 (replay-flakes) the entry check failed in 5 of 6 Rust runs
beside two other games (0 of 6 alone): the entry took 281 frames from
state 4, not 282. Which one follows how many frames the magic file's
textures took to load (`g_BtlEffectReadClear` from the magic ID to -1):
original runs 24-28 frames (282 in all), Rust runs 13-17 (281, or 282 at
17), and with the frames slowed by a per-frame probe (400 ms a frame) the
load ended 8-10 frames in and every run took 281. The textures load on
`AsyncLoadManager`'s thread in real time. Holding the main thread at the
boundary until that loader is idle (`+0x20`/`+0x2c` clear, as
`AsyncLoadManager_waitTasks_00691d40` waits) stalls it: the loader needs
the main thread's next update to finish, and the replay timed out. Letting
the FMOD thread catch up at each boundary (its 33 ms sound loop waits in
real milliseconds) and waiting for the render thread's queued flips made
Rust runs give 281 in 5 of 5 but original runs still 282 (the load
length differs), so neither was kept. The script then checked 281 or 282
(set 280 frames after state 4, -1 within two frames), as Energy Blast's
258 or 259; six Rust runs three at a time passed (282, 282, 281, 282,
281, 281; Energy Blast 259 in all). A port that ends either effect a frame early or late stops the
replay at that `waitfor` (`timeout at N`), and its cut trace fails the
comparison. The trace ranges cannot show this: they compare each range's
sequence of distinct values, not their frames, and no traced word counts
the effect's frames. Valefor's per-frame words do, but the idle pose
before and after the entry makes them differ between original runs.
Since the loader's turn below (2026-10-26) the entry takes exactly 281
frames and Energy Blast 259 in every run, original and Rust, and the
script checks both to the frame again.

### Load pacing

How many frames a load took in lockstep depended on real time, and the
Rust replacements changed it (2026-10-26, load-pacing).
`examples/load_pacing.rs` (`ffx_harness::pacing`) measures it: it plays
battle-overdrive up to Yuna's summon, then steps one update at a time and
holds the main thread at each boundary for 250 ms (`load_pacing
[original] [ms]`), and counts the updates from the magic ID (`0x00c64ca0`
= `0x253`) to the end of the effect read (`g_BtlEffectReadClear`
`0x00c421e8` back at -1). Each frame it logs the read, whether the loader's
request lists are non-empty (`g_AsyncLoadManager` `+0x20`, `+0x2c`) and
the scene's VFX texture slots and how many are ready (`g_PhyreScene`
`+0x58`, `+0x60`); `stacks` adds every thread's stack (the harness's
`stacks` command: `eip` and the return addresses into `FFX.exe`). The game
test `magic_file_load_paced_like_the_original` runs it on both sides.

What the measurement found:

- `AsyncLoadManager_threadMain_00691360` runs `PhyreScene_loadMagicFileVFXTexture`,
  which queues one cluster per texture; `Phyre_PBinary_loadClusterTextures`
  creates each under the render lock (`0x00c94ee4`, a recursive
  `PCriticalSectionSimple`: `Phyre_PMutex_lockInner_00431850` spins
  on the owner word `+0x10` with 20 ms event waits). Valefor's file has
  26 textures, one top-level hold of the lock each.
- The render thread takes that lock three times a frame and holds the last
  through `Present`, and in lockstep the harness holds `Present` until the
  main thread leaves its boundary (sleeping 1 ms at a time). At every
  sampled boundary the loader waited for the lock and the render thread
  owned it; the main thread took it in none of the load's frames.
- So the loader only got the lock between the render thread's holds, after
  the present hold ended. In the original the main thread's update took
  about 0.8 ms and the flip was queued about 1.3 ms after the boundary,
  before the render thread woke (about 1.5 ms): the render thread went
  straight on and the loader got one texture in about every other frame.
  The Rust update took about 2.5 ms (flip at 3-3.7 ms), the render thread
  waited for it without the lock, and the loader loaded two or three
  textures a frame. Load frames with 250 ms frames: original 21-43, Rust
  9-18; with every M4 hook off, or the M3 ports alone (`FFX_HOOK_ONLY`),
  Rust-like; single modules gave in-between counts, because the cause is
  the update's real time, spread over many ports (in a sampled profile the
  Rust update spends its time under `Ch_CalcMain`, `Ch_DrawMain`, the
  battle window and message functions and waiting for the job workers),
  not one port's logic.

The loader's turn (`crates/ffx-hook/src/loader_turn.rs`) takes the race
out, in lockstep only. When the main thread is about to leave a boundary
(the script is done with that update and its screenshots) and the loader
has work, the harness lets the render thread out of its present hold, its
next take of the render lock waits, and the loader runs with the lock free
until its request lists are empty; then the main thread and the render
thread go on. While the main thread runs an update the loader waits before
each top-level take of the render lock, unless the main thread stays in one
update for 100 ms (it waits for the loader there, as during map loads), in
which case the loader goes on until that update ends. The loader's thread
is noted by a trampoline on `AsyncLoadManager_threadMain_00691360`. The
rule itself (`loader_turn::before_take`) runs at the top of the lock,
`Phyre_PMutex_lockInner_00431850`, which has one hook: p1-load's port
(`ported/p1_load.rs`) calls it before taking the lock, and when that port
is left out (`--hook-disable replacements` for the original code,
`FFX_HOOK_DISABLE`, `FFX_HOOK_ONLY`) `loader_turn::install_fallback`
puts a wrapper with a trampoline to the original at the same address
instead, so the original code runs under the same rule. Measured with it: Valefor's
load ends 2 frames after the magic ID in original, Rust and Rust with no
slowing alike, and battle-overdrive's entry takes 281 frames and Energy
Blast 259 in every run.

### Pinned state

`lockstep on` makes the frames after it repeat, but not what the frames
before it leave behind: boot, title and `load` run free in fast mode and
take 220 to 430 updates, depending on real time (2026-10-26,
magic-determinism). Three things carried that count into battle-magic:

- `Sg_FrameSync`'s (`0x00821e80`) carries (`f32` `0x012fb7f8`,
  `0x012fb7fc`). The virtual clock's frame is two NTSC fields
  (2 x 0.0166833 s), 0.1% over the 1/30 s logic tick, so the 30 Hz carry
  grows a little each update and about one update in a thousand runs two
  logic ticks (`g_VoiceHold` 2: twice the effect draws in one update).
  Where that double tick fell moved with the update count before
  lockstep, so the shots fell into a few clusters (battle-magic-1 the
  same in runs whose lockstep started at update 296 to 323, another
  picture from 377). The step itself also depended on it: the game time
  is an `f32` (`FFXApplication` `+0x20`, clock - `g_AppStartTime` -
  paused time `+0x28`) and the step (`+0x24`) the difference of two, so
  its last bits came out differently with the clock further on.
- `g_Vu0_R` (`0x00c0a000`) and its seeded flag (`0x00c8f728`): the
  effects' generator (`op_random`, the ppp rand programs), drawn by the
  field's effects before the battle.
- the main thread's CRT `rand()` seed (`op_loop_ffx_after` draws from it
  each update; the job threads' are seeded per job, "How it gets in").

The `pin` command (main thread, between two updates; on the virtual
clock) sets them: the clock to 4096 s past its start, `g_AppStartTime`
so that the game time is 64 s, the paused time 0, `g_FrameLimiterLast`,
`+0x20`/`+0x120`, the running update's step and the play-time stamp
(`0x01325b84`) to match (the next frame is one interval on, every step
one frame), the emulated vsyncs from the new time, both carries 0,
`g_Vu0_R` `0x3fb5a5c3` (seeded), the main thread's CRT seed 1. It
answers what it pinned. The game's own generators need nothing: loading
a save seeds them. A replay that needs it says `pin` right after
`lockstep on`. battle-magic, battle-attack, menu-save, menu-shop,
menu-browse and menu-grid do, event-besaid since audio-check (2026-10-26;
its FMOD capture differed between unpinned original runs, "Audio capture"
below), and the extra list and the draw-gate replays. The others
(field-walk, the realtime ones, battle-overdrive, event-agency,
event-arrival) don't.

Measured on battle-magic: without `pin`, original runs as well as Rust
runs landed in four or five pictures for shot 1 and in three for shots 2
and 3 (Rust always in other ones than the 2026-10-26 references, whose
two recordings had happened to start lockstep 6 updates apart); with
`pin`, three original and three Rust runs gave the same three pictures to
the pixel, whatever the update lockstep started at (269 to 370).

The same pre-lockstep state moved the small differences in battle-attack,
menu-save and menu-shop (pixel-regress, 2026-10-26). They had been blamed
on the p1 and p6 ports and on h3's `PhyreScene_setupSamplers`:
- the cursor glow's left edge, a 1-2 px column: battle-attack-1 x 73..74,
  y 549..561, menu-save-6 x 545..546, y 251..261, and menu-shop-2/-6/-7.
  It passed as a streak.
- battle-attack-1's idle pose (fish, Tidus, Rikku): 100 to 960 px on
  model edges, which failed the compare.
Unpinned, six original-code runs on main c28c3607 gave all of them: the
glow in 5 of 6 runs, a 165 px pose difference in 1. 25 Rust runs gave
them at about the same rates, with or without other games in the slots.
No port caused them. With `pin` in those three replays, three original and
three Rust runs gave every shot of the three replays the same to the
pixel, except battle-attack-2 (x 141..472, y 185..287) and menu-save-4
(the title, which loads in real time). Those two differ between original
runs either way. The three replays, with menu-browse and menu-grid,
were recorded again with `pin` on 2026-10-26.

`pin` cannot fix one more source of drift. `lockstep on` can take hold a
few updates late after `load direct`, with the map already running. Every
run so far had the same gap from `lockstep on` to the first `waitfor`
(battle-attack 88 updates, menu-save and menu-shop 66), except one
recording, made with three games starting at once: 87, 54 and 59. Its
shots differed everywhere. Check those gaps in the replay output (`.out`)
before trusting a recording. (Since 2026-10-26 the main thread itself
turns a requested lockstep on, at the top of the first boundary it enters
after the command; before, the hold could also fall on the boundary it was
already in, which changed battle-attack's audio in two of 42 runs: "Audio
capture" below. `lockstep on` now answers the frame it holds at, where it
answered the frame before.)

`tools/replays/event-besaid.txt` and `event-cliff.txt` (2026-10-24) play
scripted events from save slot 3 (`ffx_003`). The harness's five save
entries are `ffx_000` (the autosave: Besaid beach, map `0x46`, scenario
118), `ffx_001` (the salvage ship deck, `cdsp0000`, map `0x47`, scenario
100: the other replays), `ffx_002` (the sea off Besaid, map `0x46`,
scenario 111: the arrival cut scene plays on load), `ffx_003` (Besaid
beach at the save sphere, scenario 118, after Wakka's beach scene) and
`ffx_005` (the Thunder Plains agency, `kami0100`, map `0x107`, scenario
1470); `load direct N` loads `ffx_00N` (`load N` counts the title's list,
where entry 1 is the autosave). The map words are `g_SaveData` `+0`/`+2`
(`0x0112ca90`), the scenario `+0xbec` (`0x0112d67c`).

event-besaid (lockstep, fast mode, about 18 s) walks off the beach (the
exit line of `bsil0000`'s worker 32, `Native_MapChange(20)`) to map `0x14`
(`bsil0200`), walks up into Wakka's trigger (scenario 118 to 119: Wakka
waves, his voiced line "Hey! It's this way!" as a subtitle, then he runs
off), then right to map `0x29` (`bsil0300`), where the script has put
Wakka on the far bank, and up to the river's edge. Shots: the beach, the
subtitle, map `0x29`, Tidus at the river with Wakka across it. Each map's
inputs and shots are pinned to ctrl work 0's count, which each map's event
starts from 0. Traced: the map words, scenario, player position
(`g_SaveData_PlayerPos` `0x00ccb9a8`), field view matrix (`g_YiViewMatrix`
`0x018ded38`), message windows 0 and 1 (`g_AtelMsgWindows` + `0x14`, +
`0x40`: state and flags), the random generators, the party records.

Voiced lines wait for the voice stream, which plays in real time, so what
a script does after one lands on a different frame in every run, original
code included (Wakka's run: 7,700 pixels between two original runs at the
same count). event-besaid takes no shot then, and waits for the subtitle's
window to close (`waitfor` state 0) before the next input, after which the
script's frames are fixed again. Two more things vary between original
runs: a chocobo icon at the bottom right of maps `0x14` and `0x29` that
animates by the wall clock (masked, `shot-compare.py` `MASKS`), and the
player position range (`run-all.sh` passes `--ignore 2`). That was thought
to be a trace frame skipped; sampled between two updates (2026-10-24) the
three original runs still gave three different sequences (404-406 values,
parting at the 193rd to 262nd), with other values, not only fewer, so the
position itself differs.

event-cliff (`realtime`, about 72 s, the longest replay) makes the same
walk one step further, into the trigger of `bsil0300`'s worker 18 (a line
at the river, active at scenario 119; entry 6 disables the control and
starts worker 9's scene): the camera pans over the water, cuts to Wakka and
Tidus on the bank ("Tidus: Huh?"), Wakka shoves Tidus off it, jumps after
him, and Tidus comes up in the water. The scene keeps time with its voice
stream like the opening (`boot.txt`): in fast mode two original runs gave
another moment of it at the same event count in every shot (up to 600,000
pixels apart), at the game's own rate a few thousand (cloth, hair, water).
Without lockstep the walk's presses land a frame or two apart, so the shots
are pinned to the scene: the frame message window 0 opens for the first
subtitle (`waitfor 0x01326d84:4 02000000`), then fixed waits (the event
count read back with each shot lands in the replay's output; two original
runs read the same counts within one). The trigger is a fixed line
(-417.0, -34.9, -476.4) to (-483.1, -35.0, -359.5), each end extended by
20; Tidus stops against the river edge 0.24 units short of it, 23 inside
its end, and the ninth step is held 45 frames for slack. Traced from map
`0x29` on (worker 6 is Wakka only there): as event-besaid without the
player position and the view matrix, which follow the presses' frames, plus
Wakka's position and rotation (`*0x01325bbc + 0x4968`, `+0x4998`). The
shots are on `run-all.sh`'s `eye` list (checked by eye, like
battle-overdrive's): three original runs differed by 2,000-77,000 pixels
per shot (water, cloth, hair), more than the Rust run does (2026-10-24,
sampled between two updates: 0-2,100). Wakka's position and rotation
change every frame, and at the game's own rate a frame that falls behind
runs more ticks (`Sg_FrameSync`'s hold), so the trace, taken once a frame,
can miss values in between: a Rust run missed 4 of 197. `run-all.sh`
compares those two ranges by order (`replay-compare.py --sampled 6,7`).

On 2026-10-24 the Rust run of event-cliff did not reach the scene, and with
that fixed Wakka stood about 200 pixels out of place: `Atel_TurnUpdate`
and `Atel_MoveUpdate` made their 8-word `Math_YawPitchTo` call through
`Host::call`, which in the game passes at most 6 words and refuses longer
calls (the oracle hosts accepted any width, so the parity tests passed).
Bisected with `FFX_HOOK_DISABLE` lists (at most about 30 KB of names
per run: the variable goes through the Windows environment; for longer
lists use `FFX_HOOK_ONLY=@<file>`, `docs/hooking.md`). A replay that
stops early like this leaves its later shots missing and its trace's last
line cut: `shot-compare.py` reports a missing shot as a failure and goes on
with the other replays' shots, and `replay-compare.py` drops a cut line.

`tools/replays/event-agency.txt` (2026-10-24, lockstep, fast mode, about
20 s) plays the Thunder Plains travel agency from save slot 5 (`ffx_005`,
`kami0100`, map `0x107`): a book with a Read/Don't read choice and five
pages, out of the front door (worker 23's line, `Native_MapChange(256,
2)`) to the plains (map `0x100`), the chest by the door (the
lightning-dodge prize: "Amazing! You've dodged 200 lightning bolts in a
row!", then "Obtained Venus Sigil!", `Native_015b_GiveEventReward`, a key
item), back in (entrance 1), the counter (two messages and the four-way
choice Rest / Buy weapons / Buy items / Leave) and Rest (fade out, the inn
music, every member's HP and MP restored, fade in). Each map's inputs and
shots are pinned to ctrl work 0's count, as in event-besaid. Rest waits
for its music file and its sound (real time), and after it the plains'
rain and lightning and some agency animations were no longer at the same
phase (after a rest, shots of the plains differed by 47,000-67,000 pixels
between original runs), so Rest comes last and no shot follows it; the
script waits for the control to come back (ctrl work 0 `+0` bit 1,
`0x01325ba0`, which `AtelUcOn` sets). Traced: the map and previous map
(`0x0112ca90`, 16 bytes as in event-besaid), scenario, player position,
view matrix, message windows 0 and 1 (`+0x14`, 12 bytes each), the random
generators, the party records, the key items (`g_SaveImportant`
`0x01130f1c`) and ctrl work 0's first word (`0x01325ba0`: bit 1 of `+0`
the control, bit 2 of `+3` while an event reward is given). Until
2026-10-24 the map words were cut to 4 bytes, the windows to 8, ctrl work
0 to its byte `+3`, and the position was left out of the comparison: each
had shown a one-frame value in an original run, which the render-thread
trace had caught (below). Six original runs since agree on all of them
but the position, which skipped a value in one (a frame where it moved two
steps): `run-all.sh` compares the position by order (`--sampled 2`).
Seven shots: the agency, the book's choice, its first page, the plains by
the door, the chest's first message, "Obtained Venus Sigil!", the
counter's choice. The plains' shots (4-6) and the counter's (7) show the
chocobo icon at the bottom right (x 1227-1262, y 661-705), which animates
by the wall clock as on event-besaid's maps (about 800-1,000 pixels
between original runs), so `tools/shot-compare.py` masks it there as it
does for event-besaid. The plains' rain differs between original runs too
(about 1,000 pixels more, in some pairs of five runs, 2026-10-24), so
`run-all.sh` compares shots 4-6 coarsely, as field-walk's
(`--coarse`). Rin's eyes at the counter (x 663-673, y 345-347) take one of
two states in shot 1, menu-save-5 and menu-grid-9 (in one original run of
three before the middle-calc jobs were seeded, and in some Rust runs
after): 4-5 pixels, under `shot-compare.py`'s streak limit.

Until 2026-10-24 (harness-main-thread) the trace sampled in `Present`, on
Phyre's render thread (`Phyre_PRendering_PRenderThread_threadEntry`
`0x00616010`; the call log's callers `00593782,006159bb,0061608e`), while
the main thread ran the next frame's update (`Script_Run`, the natives)
beside it: lockstep held only the render thread, and the main thread went
on until it waited for the render thread. In the call log `Native_MsgOpen`
started before one `present` line and returned after it. A sample saw the
main thread's update up to wherever it had got to (agency-race). Two pairs
of writes were caught this way in message windows 0 and 1's
`+0x1c..+0x1f` (`+0x1d` the mode flags, `+0x1e`/`+0x1f` the choice
cursor), each made by one call on the main thread with no frame in between:
- the book's choice (window 1): `Msg_ShowSetup` (`0x008607a0`) calls
  `Native_MsgOpen`, which stores `Msg_ClampChoices`' flags (`02`) into
  `+0x1d` first and then sets up the text box (`TOInitMessageSlot_008ada30`,
  `TOSetWindowRGBA`, ... `TOOpenWindowAndMessage`), then
  `Native_MsgSelect`, which ORs in `0x20`. Split traces showed `02`, then
  `22` a frame later;
- Read confirmed (window 0, the book's message reopened): in one script run
  `Msg_CloseWindow` (`0x008640f0`) writes `+0x1e` = `ffff`, `Native_MsgOpen`
  writes `+0x1d` (`00`) and sets up the box, and the wait's start
  (`Native_MsgWaitEx` `0x0085abd0`, `wait_start`) ORs in `0x10`. Split
  traces showed `ffff` with `+0x1d` `00`, then `10` a frame later.
The longer the main thread spent between the two writes (the text box
setup in between), the likelier a sample landed there. Per-byte traces
(`~/ffx-work/agency-race/bytes.txt`, one-byte ranges decoded by
`decode.py`; `probe.sh`): the original 0 of 7 (and 0 of 25 before, 0 of 6
with coverage mode's first-call traps), plain Rust 1 of 6 (3 of 26
before), hook-hit mode 3 of 3 (its first-call traps fall in the text box
setup). The original split both pairs in 3 of 3 runs, exactly as Rust did,
with a debug build that wraps the original `Native_MsgOpen` with a
trampoline and sleeps 30 ms after it returns (`debug-probe.patch` there,
`FFX_DEBUG_MSGOPEN=30`; 0 of 2 with the same wrapper and no sleep). No game
state differed: the main thread reads its own writes in order, and the
game's code never looks at these bytes between the two writes. So no port
was wrong, and the old bisection could not find one: what it moved was the
main thread's time inside that window (any 757 hooks' worth of trap and
call cost), not one function or a pair of them. The trace now samples on
the main thread between two updates ("Frames and threads"), so the windows
are traced for 12 bytes again; three original runs, the Rust runs and a
hook-hit run agree on them.

`tools/replays/event-arrival.txt` (2026-10-24, `realtime`, about 90 s, the
longest replay) plays the cut scene save slot 2 (`ffx_002`, the sea off
Besaid, `bsil0000` entrance `0x41`, scenario 111) runs on load, into the
control: Tidus swims up and surfaces ("Tidus: Rikku!"), "Tidus:
Blitzball!!", the Aurochs on the beach ("Hey! You okay?"), the kick, the
ball over the beach, then Tidus treading water with the control back.
Voiced, so `realtime` as event-cliff, with the shots pinned to the first
subtitle (window 0 state 2) and fixed waits after it (two original runs
read the event count within 10 of each other at each shot), and the last
one to the control (ctrl work 0 `+0` bit 1). The first subtitle comes
about 640 counts (21 s) after the load, the control about 1,280 frames
after it. Traced from event count 256 on (in the event's first 40 frames
one original run of two showed a one-frame value in the map words and in
Tidus's position): the map words, scenario, message windows, the global
generator, the party records, Tidus's x, z and rotation (the player's
worker 0, `*0x01325bbc + 0x558`, `+0x560`, `+0x588`; not y, which bobs every
frame once the control is back) and the player chr's motion id
(`*0x01300788 + 0x72c`: `Sg_SetPlayer`'s pointer, 0 and so `-` until the
control, then `Ch_MotionGetId`'s field). On the `eye` list: two original
runs differ by 127,000-846,000 pixels per shot (water, the moving camera).

The control check answers a report (2026-10-24) that Tidus sometimes stays
in the upside-down pose of the swim when this scene ends. On the
`load direct 2` path no FMV plays (the movie number `0x00c40e28` stays -1;
the `Native_b000_MovieStart` of `bsil0000`'s worker 24 entry 5, which
worker 0 entry 17 requests, was not reached; after the load the screen is
black, then fades into the in-engine swim). In 22 runs to the control
(6 fast lockstep and 3 realtime each on the original code and the Rust
replacements, and 4 with the coverage session's presses during the
scene) Tidus came back upright every time: motion `0x000110b8`, worker 0 at
(-184.8, 3.5, -46.7), yaw -1.82, ctrl byte `0x8b`, and a press of Up moved
him (to about (-177, -75)). Not reproduced on these paths; the replay
guards it from now on (a wrong motion at the control changes the last
range's value).

`tools/replays/menu-browse.txt` opens the main menu and takes screenshots
at fixed frames. Some of the screen animates by the wall clock (the
character panels' shimmer), so two runs of the original already differ in
about 27,000 pixels; `tools/shot-compare.py ORIG ORIG2 RUST` (three
directories of the screenshots: the original twice, then the Rust run)
counts only the pixels the original draws the same both times, a few
pixels around each animated one left out.

`tools/replays/run-all.sh --record` records the original runs (twice),
`tools/replays/run-all.sh` replays everything with the Rust
replacements and compares. A recorded original run can itself be off (one
field-walk recording had other party records from the first frame on,
which no later run reproduced): when a comparison fails, record the
original again before suspecting a replacement. On 2026-10-16 the three
replays of the time gave the original's events and screenshots with every
replacement in (1593 functions; M3's 13 replays: `docs/rewrite-plan.md`);
the menu screenshots keep a few dozen pixels on the
animated cursor that the two original runs happen to agree on, so
`tools/shot-compare.py` leaves out 12 pixels around every animated one
(the cursor icon is about 25 pixels wide); with that, 0 pixels differ.

`tools/replays/menu-shop.txt`, `menu-grid.txt` and `menu-save.txt`
(2026-10-24) play the menus menu-browse leaves out, all from save slot 5
(`ffx_005`, the Thunder Plains agency `kami0100`, with all 11 main menu
entries), in lockstep and fast mode, the first input at ctrl work 0's
count 101 as in field-walk. Their shots passed `shot-compare.py` (0 pixels)
on the first Rust run; none of them shows the main menu, whose play-time
clock blinks (menu-browse masks it), or the PC save and load lists, whose
dates are the files' times.

- menu-shop (about 20 s): Tidus walks to the counter and talks to Rin
  twice. Buy weapons (the weapon shop, `MenuWShop_*`, menu slot `0xc`:
  bit `0x1000` of `g_TkMenuMainActiveMask` `0x018408ec`): a Baroque Sword
  bought ("Are you sure?" and "Sell ...?" start on No, so Up first;
  "Equip?" answered No), a Stunning Steel sold; Buy items (slot `0xd`, bit
  `0x2000`): a Mega Potion bought and sold. Gil 16563, 15738, 16500,
  16450, 16462. Each shop's inputs start a fixed number of frames after its
  slot turns active (`waitfor` on the mask), each dialogue's after message
  window 0 opens. Traced: gil (`g_SaveGil`), the item inventory
  (`g_SaveItemIds`, `g_SaveItemCounts`), the equipment inventory
  (`0x01130f2c`), the party records, the mask, message window 0. In one of
  about ten runs (an original one, beside other agents' games) Rin's
  dialogue never opened and the replay stopped at its first `waitfor`;
  the walk's positions were the same in every run read back.
- menu-grid (about 20 s): Sphere Grid, Tidus: Use, Power Sphere, the
  Strength node under the cursor (Power Spheres 99 to 98); Use again, Right
  to the Defense node (97); Move one node (S.Lv 30 to 29); Cross, "Return
  to Main Menu?" Yes. After a node's activation its message must run out
  (60 frames) before the next press, or the press goes elsewhere. Then
  Aeons (Valefor's status and abilities) and Formation (Tidus and Rikku
  swap). Traced: the party records, the inventory (the spheres),
  `g_SaveAbilityMap` (`0x0112ec7c`, `0x1320` bytes: the grid's save
  block), the party order (`g_SaveInParty`), the mask.
- menu-save (about 30 s): Equip (Tidus: Sonic Steel to Brotherhood, so the
  save differs from `ffx_005`), the save sphere beside Tidus, Save: the PC
  save list (Flash, keys: Enter, Backspace), New Save Data, Yes. It is
  written as `ffx_004` in the slot's sandboxed Documents folder
  (`Options::home`, `~/ffx-work/harness/<slot>/home`, copied afresh at each
  launch; the user's saves are never written). The PC menu's Quit ends the
  process and soft reset does nothing in this build, so the replay goes
  back to the title as a game over does: `call 0x0086da10`
  (`AtelJumpGameOver`: map `0x17`, which goes on to `memochek` and the
  opening, map `0x15d`; Start skips to the title proper). Then Load, list
  entry 1 (the newest manual save, the one just written; entry 0 is the
  autosave), Yes: the save's map opens again with Brotherhood equipped.
  Every key waits for the save/load machine's state (`0x00ce72e8`: save 1
  list, 2 its input, 3 Yes/No, 5 writing; load `0xb`, `0xc`, `0xd`;
  `docs/formats/save.md`) and 30 to 60 frames more, the title for the map
  words and then its event count (101, the menu shown, as boot's shot 4;
  since 2026-10-26: a fixed 120 frames after the map word left the title
  black in one Rust run beside two other games, and its Load key was lost,
  `waitfor 0x00ce72e8:4 0c000000` timing out at frame 2418). Traced: `g_SaveData` and the save file buffer without the play
  time (`g_SaveData +0xbc`, file `+0x10`, `+0xfc`) and the CRCs (file
  `+0x1a`, `+0x64f4`): the play time counts from `graphicGetTime` and
  ticks at another point of the script in every run, original code
  included; the machine's state, cursor and file counts, the slot list's
  present flags and its entry 1's slot, message window 0. Until
  2026-10-24 `run-all.sh` left three ranges out (`--ignore 2,11,12`), each
  of which had taken an in-between or other value for a frame in an
  original run: the save data word at `+0x28` (at the load frame `+0x29`
  held another value for one frame: 1 of 9 original runs, most Rust runs),
  the present flags, and entry 1 before the save; in hook-hit mode
  (`--hits`) three more (`0,4,7`: the loaded block's map words, the rest
  of the block and the packed save's header, a frame apart with the slower
  first calls). A bisect of the Rust runs' `+0x28` difference with
  `FFX_HOOK_DISABLE` found no single port, and the halves that fixed it
  held a harness hook (`Phyre_PTimerWin32_GetTime`, the virtual clock;
  `Input_KeyHeld`). They were the render-thread trace catching the main
  thread's copies half-done ("Frames and threads"): sampled between two
  updates, three original runs, three Rust runs and a hook-hit run agree on
  every range, and nothing is left out. The waits on the map words still
  check the maps.

### Coverage replays

The extra list (2026-10-26, m4-coverage2; `tools/replays/run-all.sh
--extra`, recordings in `~/ffx-work/replay-shots-extra`) reaches platform
code the main list does not (`~/ffx-work/m4/port/plan.md`, "M4 is done
when", item 2). All are pinned (`pin` after `lockstep on`) except the
real-time movie.

- `settings-hdao`, `-msaa`, `-fxaa`, `-smaa`, `-texhigh` (about 15 s
  each): field-walk's deck (save 1, `cdsp0000`) with one settings file
  each ("Sandbox settings"), shots at event count 101 and after a short
  walk of a fixed 230 frames. `settings-shadow` the same with `Shadow=3`,
  then Besaid beach (save 3, shadow solution 4) and the agency (save 5,
  solution 0) through `loadsave`, so the three shadow solutions the saves
  reach all draw their shadow maps.
- `movie-fmv` (`realtime`, about 40 s): movie 2 (`IWK_us.webm`, Wakka's
  kick). No save reaches an FMV by itself; the script requests
  `bsil0000`'s worker 24 entry 5, which waits for effect flag 5 and calls
  `Native_b000_MovieStart(2, 0x500)`, with a `call` of
  `AtelRegistSignalWithoutSameLevel` (`0x0086eba0`: from `0xffff`, worker
  24, kind 0, level 1, entry 5) on Besaid beach (save 3), then does what
  `Native_b009_MovieCameraWait`'s start does (`g_WaitFramesByTime`
  `0x0112a008` = 1, `graphicPlayVideo(0)` `0x006427c0`) and, after the
  shots, what `Native_b004`/`b001` do (`graphicDestroyFmv` `0x0063df30`,
  `g_WaitFramesByTime` 0, the movie number `0x00c40e28` -1). The player
  needs real time: in fast mode, lockstep or not, the movie never started
  (`g_FMVPlayerManager` `+0x6d8`, the frame, stayed 0 and the screen
  black), and calling `graphicInitFMVPlayer` with `call` while lockstep
  held the main thread hung the game. At the game's rate the frame
  advances about one per update; the shots are pinned to it (`shotwhen`
  on `*0x00cded3c+0x6d8`, windows 4 frames wide: 64, 192, 320): the
  three were the same to the pixel in three original and two Rust runs
  (updates 392, 520, 648 in each); the field after it differs between
  original runs (real time). event-arrival already runs the same movie
  code (coverage mode: libvpx, mkvparser, libVorbis, `PVideoPlaybackWin32`
  reached in event-arrival and the session on save 2, not on the other
  saves), although nothing of it shows there; movie-fmv reached no
  function those had not.
- `iggy-menus` (about 40 s): the PC menu (Esc) and every page it opens
  without changing anything (System setting: Audio, Video scrolled to its
  last row, Key bindings, Keyboard; Special features; Change parameters),
  then the title as after a game over (as menu-save) and the load list,
  left with Backspace (save/load state `0xc`, then `0xa`). The PC menu
  reads DirectInput keys; Backspace goes back one page and puts the cursor
  on its first entry; Esc closes the menu from any page; Enter on the
  Video page's first row opens the resolution list, so the script never
  presses it there. The Flash pages animate their transitions: each shot
  comes 240 frames after the key that opened the page. Those fades follow
  the wall clock: the pages with sliders (3-5) and the title (11, 12)
  differ between any two original runs, and a third original run differed
  from both recordings on page 6 (the background's fade, 21,746 pixels),
  so iggy-menus is on the extra list's `eye` list (traces compared). No
  shot of the load list (its dates are the files' times).
- `battle-ifrit` (about 20 s): the Thunder Plains (`battle 34 0 0`,
  `kami00_00`, battle map `0x412`) from save 5, set up as battle-overdrive
  (party, the monsters' HP 99999); Tidus attacks, Yuna summons Ifrit
  (magic ID `0x254`, actor 9). Ifrit's effect state (`+0xdfb`) goes from 1
  to 5 within a frame (never 4 at a boundary, unlike Valefor's), so the
  shots count from 5. Its shots were the same in every run but for a few
  cursor-glow pixels.

Measured on 2026-10-26: two recordings and a third original run gave the
same traces; the settings shots were the same to a few pixels (the
chocobo icon on Besaid beach and in the agency, which animates by the
wall clock, is masked in `settings-shadow-3`/`-4`). Two Rust runs with
every replacement passed every trace and shot. They stay on the extra list
because their recordings would show up as missing shots in every other
branch's main-list comparison until they are merged.

Coverage (`COVERAGE_ALL=1`, original code, the main list, the extra list
and the five sessions; per-run hits in `~/ffx-work/m4/coverage2/hits`):
10,596 functions. Beyond the main list and the sessions the extra list
reached 39: 20 game-logic functions in battle-ifrit (all ported), and 19
render glue and `PhyreEngine` functions of the settings replays (shadow
maps 11, FXAA 3, SMAA 3, MSAA 1) and iggy-menus (1). HDAO, `TQ_HIGH` and
the movie reached nothing new: their code also runs with the settings off
(HDAO's pass returns early) or in event-arrival (the movie). The open
ones are in `~/ffx-work/m4/port/coverage2.list` (`tools/coverage-new.py`).

`fpucw` answers the game thread's x87 control word: `0x027f` (53-bit
precision, round to nearest, all exceptions masked).

### Audio capture

`FFX_AUDIO_WAV=<file>` (a WSL path, best on `/mnt/c`; `tools/run-game.sh`
passes it with `/p`) writes FMOD's mix to a WAV file, original code and Rust
alike (`crates/ffx-hook/src/audio_wav.rs`). FMOD Ex is 4.44.32. The hook
wraps `FFX.exe`'s import of `EventSystem::init`, which
`FfxFmod_initEventSystem` (`0x00706820`, original or h4's port) calls, and
before the real `init` it sets on the `System`:
`FMOD_OUTPUTTYPE_WAVWRITER_NRT` (5: no audio device, so nothing reaches
the speakers), the software format with `PCMFLOAT` (the WAV holds the
mixer's 32-bit floats), DSP blocks of `rate / 30` (1600 at 48 kHz: one block
per FMOD tick), `FMOD_INIT_STREAM_FROM_UPDATE` (no stream thread), and the
file name as `extradriverdata`. Measured: 48 kHz stereo float, 1600 x 4
buffers, one block per `EventSystem::update`; at `FFX_TEST_VOLUME` 5 the
battle-attack capture peaks at 0.039 (not silent).

Lockstep for FMOD: with the virtual clock, once the main thread has reached
its first frame boundary, `FmodManager_tick` (`0x006fa700`, a trampoline)
runs only in the turn the main thread gives it at each boundary, just
before the update, and the main thread waits for it. Between turns the
FMOD thread only serves a `registCommandSync` the main thread waits on (it
runs that one handler, as the queue run does first). `audiomark <name>`
writes `mark <name> frame <n> sample <dsp clock> updates <n>` to
`<file>.marks`; event-besaid, battle-attack, menu-browse and battle-ifrit
have `audiomark start` and `audiomark end`. `tools/wav-compare.py A.wav
B.wav` compares the two between the marks, sample for sample (or
`--tolerance`), and prints the first differing sample, its time and FMOD
tick. `quit` (TerminateProcess) loses FMOD's last file buffer (about 450
frames); the tool leaves out what was never written and says so.
The gate (M4 "done when" item 4):

```bash
tools/replays/run-all.sh --audio                  # Rust against the references
tools/replays/run-all.sh --audio battle-attack    # one replay
tools/replays/run-all.sh --audio --record [replay...]   # (re-)record the references
```

The audio list is event-besaid (music, Wakka's voice, footsteps),
battle-attack (sound effects, the victory fanfare), menu-browse (menu
sounds) and battle-ifrit (a summon). The references are two original-code
captures per replay in `~/ffx-work/replay-audio` (`FFX_REPLAY_AUDIO`;
never in git, about 15 MB a capture). `--record` plays the original code
twice and stores the pair only when the two agree sample for sample;
otherwise it keeps the old references and fails. Re-record a replay after
changing its script, its save, or anything the harness does before its
`audiomark start`; the Rust run is compared with both references, and any
difference in a sample fails. A failure prints the first differing sample
(its time after the start mark and its FMOD tick: one tick is one update
of the window) and how many frames differ. To find the port behind one,
capture with `FFX_HOOK_ONLY=@<list>` (the `ported!` names of some
`crates/ffx-hook/src/ported/*.rs`, as `~/ffx-work/m4/main-capture.sh`
builds them) and split the list until one function is left; two lists
that differ apart mean two culprits (or one calling the other's Rust).

Determinism (2026-10-26, audio-check). With the turns above, two
original runs agree sample for sample on all four replays, given three
things:

- The fallback tick: the first recording (2 s stall fallback) gave
  captures exactly one block apart on all three replays (event-besaid: lag
  1600 frames, residue 6e-10). A map load holds one update for seconds in
  `CdReadSync` (`0x0076c490`, `MovieCdReadTaskLoop` waiting for the read
  thread, not for FMOD), and the free tick there ran the command queue
  mid-update, one soundLoop earlier than the next turn. It is now 30 s (a
  deadlock guard only, logged with every thread's stack, `free` in the
  marks and in wav-compare's output); no run since has needed it.
- `pin` (above, "Pinned state"): event-besaid had none, and two runs
  differed by about 1e-10 under the beach's sounds for 316 blocks (the
  `f32` game time's last bits) and by up to 0.0086 in the river walk's
  footsteps (a double logic tick: the event frame count e9 against e8 at
  frame 1029, another final player position). It pins now.
- Where `lockstep on` takes hold: set from the command thread, the hold
  fell on the boundary the main thread was in when the command came (if it
  had not reached its hold check yet) or on the next one. In two of 42
  battle-attack captures it was the first, and the mix differed from the
  first sample of the window (the same frames, the same waitfor gaps).
  The main thread now turns a requested lockstep on at the top of
  `animate`, so the hold is the first boundary entered after the command;
  18 of 18 runs matched after that.

One thing the harness does not pin: music that began before lockstep keeps
the phase real time gave it. battle-ifrit's Thunder Plains music differed
between original runs until it faded out for the battle (60 blocks), so its
`audiomark start` is after the battle's `waitfor`.

Ports (2026-10-26): with every port on, event-besaid, menu-browse and
battle-ifrit matched the references sample for sample; battle-attack went
near-silent from block 768 of 990 (the AP screen). Bisected with
`FFX_HOOK_ONLY` to `MsMusicReadStart` (`0x0079fff0`) and
`MsBattleProcess_Status_007917d0` (whose port calls the Rust
`MsMusicReadStart`): the original tail-jumps to `SndMusicRead` with its own
argument (`g_BtlEndMusic`, the fanfare 0x8a), and the port passed none, so
the fanfare never loaded. Fixed; all four now match.

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
(names, sizes, times, inodes) changes; `mods/` is made afresh at every
launch and holds only each `Options::mods` folder (`--mod DIR`), as
`mods/<name>`. Test games run without the user's mods (the game folder's
`mods/`, 16 character mods on this machine; user decision 2026-10-24):
replays, captures and game tests see the game as shipped plus what a test
passes. `tools/instance.sh N --user-mods` (or `FFX_INSTANCE_USER_MODS=1`)
links the game folder's `mods/` in as well. Files a launch installs or something writes in place are
never links: the instance's own `DINPUT8.dll` (the caller's build, replaced
at each launch), `ffx-hook.log` (`Game::log_path`, `Game::new_log` for this
launch's lines), `ffx-launch.exe`, the renderer's files (`ffx-render.toml`
copied at each launch). Tests that change mods pass them in `Options::mods`
instead of writing into the game folder, so games side by side never see
each other's.

`Options::home` (default `Home::Instance`) is made afresh at each launch:
a copy of the user's saves (`copy_user_data`), then `Options::saves`,
`(name, file)` pairs put into its `FINAL FANTASY X` folder (`ffx_001` is
load-screen entry 1), so a test can start from a given save with
`copy_user_data: false`, and the sandbox's settings. A home inside the
user's real Documents folder is refused.

### Sandbox settings

`tools/sandbox/GameSetting.ini` is every test game's `GameSetting.ini`
(`ffx_harness::SANDBOX_SETTINGS`): the user's own is never read. Until
2026-10-24 the sandbox copied the user's file; the repository's file holds
the values that copy had, which the replays were recorded with: English,
1280x720 in a window, vsync off, `Quality=VQ_LOW`, `Shadow=0`, no MSAA or
post AA, the key bindings, and `TextureQuality=TQ_LOW` (the user,
2026-10-24: tests don't need high-quality textures; it drops the top 2 mip
levels of every texture). Change a value there, in one line, and record
the replays again. `ffx-harness` drops the file's `#` comment lines when
it writes the sandbox's copy (the game's parser, `Settings_Load`
`0x00401cb0`, knows only `key=value` lines), and sets `Resolution`,
`ScreenMode`, `Mute` and `MasterVolume` (`FFX_TEST_VOLUME`, default 5).

Other settings for one game (2026-10-26, m4-coverage2): a replay line
`settings NAME` (read before the launch, like `realtime`), or
`FFX_SETTINGS=NAME` for any harness game (`Options::settings`), merges
`tools/sandbox/NAME.ini` over that file: each of its `key=value` lines
replaces the default's line in place, keys the default lacks are appended,
and its `#` lines are dropped too (a value with a `/` or ending in `.ini`
is a path from the repository instead). Keys 11-18 of the settings table
(`Anisotropic` to `HDAO`, `docs/formats/platform.md`) take the `Quality`
preset whatever the file says, so every such file says
`Quality=VQ_CUSTOM`; the default file's values of those keys are the
`VQ_LOW` preset, so only the keys the file names change. The files:
`hdao.ini` (`HDAO=High`), `msaa4.ini` (`MSAA=4`), `fxaa.ini`
(`PostAA=PAA_FXAA`, `UnSharpMask=On`), `smaa.ini` (`PAA_SMAA4`,
`UnSharpMask=On`), `shadow.ini` (`Shadow=3`), `texhigh.ini`
(`TextureQuality=TQ_HIGH`). The `settings-*` replays read the values back
(`readp 0x022fb504 0x30 0x1c`, `g_Settings` `+0x30..+0x4b`) and trace
them. Read back in the recordings: `Shadow` 3, `TextureQuality` 0 (high;
2 elsewhere), `MSAA` 4, `PostAA` 1 (FXAA) and 5 (SMAA4) with
`UnSharpMask` 1, `HDAO` 3, each as its file says. The harness writes the
file with LF line ends (the repository's file has none either; CRLF, the
platform census's open question 1, is untested). Editing
`TextureQuality` alone in `GameSetting.ini` changes nothing while
`Quality=VQ_LOW`: the preset gives index 2 (`TQ_LOW`) whatever the line
says, which is why draw-field captures with `TQ_MEDIUM` and `TQ_HIGH`
written there gave the same textures (2026-10-26); the references were
`TQ_LOW` as recorded.

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
instance folders nor the pool; `run-all.sh` left the pad word out
(`replay-compare.py --ignore`) until 2026-10-24, when the trace moved to
the main thread's frame boundary and the release of a press to the same
boundary, and three original runs agreed on it; `shot-compare.py` allows
24 levels per channel. With those, three more runs (two side by side, one
serial) gave the same events everywhere; one side-by-side run still failed
on screenshots that follow the wall clock: the field's searchlight block
at y 120 (38 levels, as a serial run in the game folder once measured) and
5 pixels of the menu cursor. Games side by side run slower in real time,
so wall-clock animations drift a little further.

The cursor glow and battle-attack-1's idle pose in the lockstep replays
came from the frames before `lockstep on`, not from the wall clock
("Pinned state"). boot runs in real time, so `pin` doesn't apply there.
boot-3 (the opening) had 39 px, single pixels on model edges, in 1 of 25
Rust runs. The two original runs differ there by 35,000 px. Rerun such a
boot-3 failure rather than bisect it. Measured with
`~/ffx-work/pixel-regress/measure.sh` (`ORIGINAL=1` for the original
code).

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
tools/game-tests.sh --all      # all five
tools/game-tests.sh HEAD~3     # compare with another commit
```

| Test | Checks | Selected by changes to |
|---|---|---|
| `start_up_input_and_kernel_mod` | the keyboard is read without focus; a mod's `item.bin` (Potion power 77) is what `g_KernelItem` points at, not a later mod's switched off by `mod.toml`; New Game to the Sphere Grid choice, where Down moves the cursor and the pad word holds exactly Circle | `ffx-kernel`, `ffx-text` |
| `first_scene_with_script_model_and_motion_mods` | into `znkd0600` with four mods: the event script grown by `pushimm 0x1234; popv v1` (its byte global at `0xa3c` reads `0x34`), its English dialogue with a changed first line (the game's message table, `*(*0x01326b28 + 0x34)`, holds the mod's bytes), Tidus's model 30 % wider with a segment doubled and a new segment (new skeleton matrix, new material on Yuna's texture, his asset list naming it), his motions turned; all read from the mod, the scene runs 10 s | `ffx-script`, `ffx-text`, `ffx-phyre`, `ffx-motion` |
| `save_and_battle_with_monster_and_formation_mods` | slot 1 of the sandboxed saves with three mods: the encounter table's salvage ship group (rate 61, weights 3 and 1) is what the game holds once the map is entered (current weights copied into the high nibbles); a battle started directly (field 3, `cdsp00_00`) has the modded formation's monsters (m068, m070, m070), and m068 the mod's 12345 HP; Tidus's and Rikku's worn weapon and armor (kernel equipment layout) give their actors' formula, power and crit bonus; the user's own settings and saves are byte for byte unchanged | `ffx-monster`, `ffx-battle`, `ffx-kernel` |
| `random_generators_repeat_with_a_seed` | three save loads: seed 1 twice gives identical generators, seed 2 others | — |
| `magic_file_load_paced_like_the_original` | Valefor's magic file loads in as many frames on the original code as with the Rust replacements, frames slowed to 250 ms ("Load pacing") | — |

Changes to the hook, the harness, `ffx-core`, `ffx-vbf`, the launcher or
the workspace select all five; other changes (docs, the CLI, symbols)
none.

Screens are recognised by mean colours of fixed rectangles; a failing
check saves its screenshot to `~/ffx-work/harness/`.
