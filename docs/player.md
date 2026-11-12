# ffx-rust: playing FINAL FANTASY X with it

ffx-rust is a new program that plays FINAL FANTASY X from your own copy of
FINAL FANTASY X/X-2 HD Remaster (Steam, Windows). It is written from
scratch to behave exactly like the game's own program, `FFX.exe`, which it
replaces while it runs. It reads the game's files from your game folder
and changes none of them. `FFX.exe`, the game data and your Steam
installation stay as they are, and you can go back to the normal game at
any time.

The package holds no part of the game. You need the game, installed
through Steam.

**This is an early test version.** Read "What works and what doesn't yet"
below before you start, and keep a copy of your saves.

## What you need

- FINAL FANTASY X/X-2 HD Remaster, installed through Steam.
- Windows 10 or 11, 64-bit.
- For achievements: Steam running and signed in.

## Installing

1. Unpack the zip file. Any folder works, for example your Desktop or
   the game folder itself. Keep all the files of the package together.
2. Find your game folder. In Steam, right-click FINAL FANTASY X/X-2 HD
   Remaster, then Properties, Installed Files, Browse. It is the folder
   with `FFX.exe` in it, usually
   `C:\Program Files (x86)\Steam\steamapps\common\FINAL FANTASY FFX&FFX-2 HD Remaster`.

## Starting the game

Pick one of these:

- **Package in the game folder.** If you unpacked the package into the
  game folder (next to `FFX.exe`), double-click `play-ffx-rust.cmd`.
- **Package anywhere else.** Drag the game folder onto
  `play-ffx-rust.cmd`. To skip that every time, make a shortcut to
  `ffx-launch.exe` and add the game folder to its target, for example:

  ```text
  "D:\ffx-rust\ffx-launch.exe" --rust "C:\Program Files (x86)\Steam\steamapps\common\FINAL FANTASY FFX&FFX-2 HD Remaster"
  ```

- **From Steam.** In Steam, right-click the game, Properties, General,
  and put this under Launch Options (with the folder where you unpacked
  the package):

  ```text
  "D:\ffx-rust\ffx-launch.exe" --rust %command%
  ```

  Pressing Play then starts ffx-rust instead of the usual launcher.
  Remove the line to go back to the normal game.

Double-clicking `ffx-launch.exe` itself starts the original `FFX.exe`,
not ffx-rust. Use `play-ffx-rust.cmd`, or `--rust` as above.

If the game folder, a game file or a file of the package is missing,
a message says what and where, and nothing starts. If the game
folder is the right one and a game file is missing, let Steam repair it:
right-click the game, Properties, Installed Files, Verify integrity of
game files.

## Saves and settings

ffx-rust uses the same saves and settings as the normal game, in your
Documents folder under `Square Enix\FINAL FANTASY X&X-2 HD Remaster`.
ffx-rust writes saves and settings in the same format as the normal
game, so a game saved in one loads in the other. A test that checks this
byte for byte (on copies of saves, both ways) exists. Its first full run
is still to come.

Because this is a test version, **copy that folder somewhere safe before
you play**, and again from time to time. To undo anything, put your copy
back.

## Steam and achievements

The package comes with Steam's own `steam_api64.dll`, which ffx-rust uses
to unlock achievements on your account, as the normal game does. Steam
has to be running and signed in. Started from the package (not through
Steam), ffx-rust tells Steam which game it is, so achievements still
count.

Without Steam, or without `steam_api64.dll`, the game runs anyway, with
no achievements (if the file is missing, a message says so). ffx-rust
never resets your achievements or statistics.

## What works and what doesn't yet

ffx-rust is being finished. In the project's tests it plays the game's
start, the title screen, walking around, events, cutscene movies, battles
with attacks, magic, summons and overdrives, and saving and loading, with
the same pictures and results as the normal game. The PC menus (settings,
save and load) are drawn by a different player for their animations and
look very close to the normal game's, but not identical.

Not yet, in this version:

- **Speed.** Many scenes run far too slowly (a few frames per second),
  because parts of the game still go through a slow stand-in that is
  being replaced.
- **Controls.** The keyboard, the mouse and controllers do not reach the
  game yet. You can watch it start, but you can't play it. This is the next
  thing being worked on.
- **Sound** may crackle or drop out while the game runs slowly.
- Some thin lines in the PC menus (the volume bars, a pop-up's frame)
  look slightly different.
- **Full Screen** mode does not change your screen's resolution. The game
  stays in a window.
- **Closing the window** ends the game straight away, without the usual
  question, and doesn't open the Steam launcher.
- A game started outside the test tools can stop with an "error" message
  while loading. This is known and being looked into.
- Only FINAL FANTASY X. X-2 and Last Mission aren't part of it.
- Mods for the normal game don't work in ffx-rust.

## Reporting a problem

Each run writes a report, `ffx-rust.log`, next to `ffx-rust.exe` (the run
before is kept as `ffx-rust.previous.log`). If the folder can't be
written to, the report goes to your Windows temporary folder (type
`%TEMP%` into the Explorer address bar).

When something goes wrong, report it at
<https://github.com/yutamago/ffx-rust/issues> with:

- `ffx-rust.log` of that run (attach the file),
- what you were doing and how long into the game it happened,
- a screenshot of any message.

The report contains the folders of your game and of your Documents, which
can show your Windows user name. Look through it before you post it
publicly.

## Removing it

Delete the package's folder (and the Launch Options line in Steam, if
you added one). The game, your saves and Steam stay as they are.

## Licences

ffx-rust is free software under the MIT or Apache-2.0 licence, at your
choice (`licences\LICENSE-MIT`, `licences\LICENSE-APACHE`). It contains
other people's free software, each under its own licence, all listed in
`licences\THIRD-PARTY.txt` (the flash player Ruffle, the video decoder
zvidlib-vp8, the libVorbis sound decoder and more). `steam_api64.dll` is
Valve's and shipped under Valve's terms (`licences\steam_api64-NOTICE.txt`).
FINAL FANTASY X is a trademark of Square Enix. ffx-rust is not made or
endorsed by Square Enix or Valve.
