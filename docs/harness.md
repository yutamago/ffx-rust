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
- Battles start from the field without walking into one:
  `screens::start_battle(field, group, formation)` writes the request a
  random encounter makes (`MsBattleEncountExe`, `0x00780d10`) and raises its flag;
  `MsBattleProcess` (`0x00790c10`) loads it on the next frame. The field table
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
--coarse field-walk`): the ship's sway runs on a timer the replay does not
pin down, so four runs of the original differ by 110,000 to 200,000 pixels
there, a sub-pixel shift of the whole picture. Blocks of 40x40 pixels then
move by up to about 14 brightness levels (the light beams at the top),
while a missing character model moves its blocks by about 50; the limit
is 25. Battle and menu screenshots
stay exact.

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

## One game at a time

`Game_Main` (`0x0062ee50`) takes the named mutex
`9B8F7468-6DE0-4C5A-9209-89D363F8FCC9`; when it already exists the game
shows "Same program is already running." and ends. `tools/run-game.sh`
therefore refuses to start while any `FFX.exe` is listed. `quit` ends the
process with `TerminateProcess`: running the game's own shutdown from the
harness thread once deadlocked, and that process, stuck while exiting,
kept the mutex (and the harness port) until it finally went away.

So every test and agent queues for the game: the game slot
(`crates/ffx-harness/src/slot.rs`), shared by all worktrees in
`~/ffx-work/game-slot`. `Game::launch` takes a ticket, waits until it is
first (saying so on stderr), holds an OS file lock while the game runs and
frees it when the `Game` is dropped; a process that dies frees its ticket
and lock with it. Scripts that start several games (`tools/replays/run-all.sh`,
`tools/game-tests.sh`) take the slot once for all of them
(`tools/in-game-slot.sh`). From the shell:

```bash
cargo build --release -p ffx-harness --bin game-slot
target/release/game-slot status              # holder and queue
target/release/game-slot run -- <command>    # waits, runs it in the slot
```

`tools/run-game.sh` refuses to start a game while someone else holds the
slot. A test stopped mid-run leaves its game running; the next
`Game::launch` in the slot ends it with the harness's `quit` (only games
that answer on the harness port: a game the user plays is never touched). A detached game (`Game::detach`) outlives its slot. Run long game
tests as background commands: they wait for their turn and finish later.

## Tests

`crates/ffx-harness` launches the game (`Game::launch`, background mode)
and wraps the commands; a launched game is ended when the `Game` is
dropped, so a failing test does not leave it running. The game tests are
ignored by default because they open a window on the Windows desktop.
Starting and loading cost more than the checks, so each test shares one
launch between several checks, and `tools/game-tests.sh` runs only the
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
check saves its screenshot to `<temp>/ffx-harness/`.
