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

XInput is not used: the game creates XInput devices only for controllers
found while enumerating DirectInput devices at start-up
(`0x006240e0`), so without one it never calls `XInputGetState`.

## Input facts

`Input_BuildPadMap` (`0x006315c0`) pairs each of the 81 bindings
(`GameSetting.ini` `ffxKeyBinding`, defaults at `0x00c8a538`) with a pad
bit; `Input_KeyHeld` (`0x00629d50`) treats a binding ≥ 28 as Phyre key
`binding − 28` (keys 0–25 A–Z, 26–35 digits, 36 Esc, 49 Enter, 63–66
arrows; scan codes at `0x00c30e90`). By the defaults, bindings 0–13 are
Triangle, Cross, Square, Circle, Right, Down, Up, Left, R3, Select, Start,
L3, R1, L2 on the keyboard and 14–27 the same on a pad.

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
  title event opens `titl00` when the opening starts. Pressing Circle,
  Cross, Start, Triangle, Square, Select, Enter, Space or Backspace during
  it does not skip it (`examples/skip_probe.rs`).
- Title: Up arrow, Enter starts a new game; then the Sphere Grid choice
  takes `press down`, `press cross`.

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
| `new_game_reaches_the_sphere_grid_choice` | title menu after the opening, New Game by keys, the Sphere Grid cursor moved by the pad | about 2 min (the opening is not skipped) |

Screens are recognised by mean colours of fixed rectangles; a failing
check saves its screenshot to `<temp>/ffx-harness/`.
