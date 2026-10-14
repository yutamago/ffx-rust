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
| Buttons | `Input_KeyHeld` (`0x00629d50`), wrapped with a trampoline | `Input_UpdatePad` (`0x00632a30`) builds the PS2 pad word at `0x00ccb170` (previous frame at `+4`) by asking it about every binding. For each held button the first keyboard binding paired with it reads as held, so the game sees the same as a key press: exactly that button's bit |
| Keys | `IDirectInput8::CreateDevice` → device `GetDeviceState` / `SetCooperativeLevel` | The PC menus (Esc) read keys directly. The keyboard is made a background device, and an unacquired one reads as all keys up |
| Focus | `ShowWindow` | Background mode shows the window without activating it |
| Isolation | `GetDeviceState`, `XInputGetState` | In background mode the real keyboard, mouse and pads are ignored: background devices would otherwise see what the user types elsewhere |
| Events | `File_Open` | Every file the game opens, and which came from a mod |
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
- `examples/mod_probe.rs` plays into the first scene with any mod file and
  reports whether the game read it and survived; it found that grown PS2
  files were cut at their original size (`docs/hooking.md`).

## Tests

`crates/ffx-harness` launches the game (`Game::launch`, background mode)
and wraps the commands; a launched game is ended when the `Game` is
dropped, so a failing test does not leave it running. The game tests are
ignored by default because they open a window on the Windows desktop:

```bash
FFX_GAME_DIR=~/steam-games/FFX-X2-HD cargo test --release -p ffx-harness --test game -- --ignored --test-threads 1
```

| Test | Checks | Time |
|---|---|---|
| `runs_in_the_background_and_takes_input` | frames advance, the keyboard is read without focus, the pad word holds exactly the held button, 1280×720 screenshots | 10 s |
| `a_kernel_mod_reaches_game_memory` | a mod's `item.bin` (Potion power 4 → 77) is what `g_KernelItem` points at | 6 s |
| `new_game_reaches_the_sphere_grid_choice` | the opening skipped with Start (B, a 3-frame tap), New Game by keys, the Sphere Grid cursor moved by the pad | 30 s |
| `a_grown_event_script_runs_in_the_game` | `znkd0600.ebp` with `pushimm 0x1234; popv v1` in front of worker 0's first entry (the worker grows and moves): the global byte at `0xa3c` is `0x34` (0 without the mod), and the scene keeps running | 45 s |
| `a_resized_model_loads_in_the_game` | Tidus's `c001.dae.phyre`, 30 % wider and with segment 0 doubled (resized buffers), is read from the mod and the scene runs 10 s | 40 s |
| `a_save_loads_from_the_sandbox_and_the_users_files_stay_untouched` | title → Load → slot 1 (the first manual save; entry 0 is the autosave, taken mid-cutscene) → Yes; the save's event starts, the game runs, and the user's files are byte for byte unchanged | 35 s |
| `an_edited_motion_group_loads_in_the_game` | Tidus's `resident0.mgrp`, root bone turned a quarter, is read from the mod and the scene runs 10 s | 40 s |

The first scene of a new game is `screens::first_scene`: New Game,
Standard Sphere Grid, Yes, Arranged soundtrack, then event `znkd0600` (the
Zanarkand ruins), which loads Tidus (`c001`) and his resident motions.
Script globals (storage 0, `Script_VarAddress` `0x0086c2e0`) are at
`*(*0x01326b28 + 0x2c) + offset`.

Screens are recognised by mean colours of fixed rectangles; a failing
check saves its screenshot to `<temp>/ffx-harness/`.
