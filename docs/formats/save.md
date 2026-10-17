# Save files and loading a save

How the PC build reads a save and starts the game from it, recovered from
FFX.exe (`symbols/ffx.sym` has the names) and checked on the running game
(`crates/ffx-harness/examples/load_trace.rs`, `load_compare.rs`). The
harness command `loadsave` (`crates/ffx-hook/src/loadsave.rs`) runs the
same path without the title menu.

## Files

`SetUpDefaultSaveFolder` (`0x006f0330`) builds the folder from Documents
(`SHGetFolderPathW`, `CSIDL_PERSONAL`) and `Square Enix\FINAL FANTASY
X&X-2 HD Remaster\FINAL FANTASY X\` into `g_SaveFolder` (`0x00ce6ee0`,
UTF-16) and sets `g_SaveFolderSet` (`0x00ce72e0`). A slot `n` (0..199) is
the file `%s%s%03d` of that folder, `ffx_` (`*0x00c3577c`) and `n`:
`ffx_000` is the autosave, `ffx_001` up the manual saves.

Every file is `0x6900` bytes, the size the save manager was made with
(`SaveData_Init`, `0x00647fb0`: buffer `g_SaveFileBuffer` `0x01597f70`,
size `0x6900`, kept at manager `+0x350`/`+0x354`):

| Offset | Size | |
|---|---|---|
| `0x00` | `0x40` | header; `+0x1a` `u16` CRC |
| `0x40` | `0x68c0` | the game state block, copied to and from `g_SaveData` (`0x0112ca90`): `+0` map, `+2` previous map, `+0xc` entrance, `+0xb8`/`+0xba` the map and entrance to return to (`u16` each), `+0xbec` story progress (`g_SaveScenario`), the party records at `0x0113205c` (block `+0x55cc`) |
| `0x64f4` | 4 | the CRC again |

`SaveDataCheckCrc` (`0x00647d70`) passes a buffer when the `u16` at `+0x1a`
equals the word at `+0x64f4` and `TkCRC(0x64b8, buffer + 0x40)` gives it
(the word is zeroed first), or always with `g_SaveData_CrcCheckOff`
(`0x00ccb9a4`).

## The load machine

`SaveData_Update` (`0x00648480`, every frame from `Sg_MainLoop`) runs the
save/load state machine `SaveDataMachine_Update` (`0x006f0950`) on
`g_SaveMachineState` (`0x00ce72e8`). Its Load states, as the title's Load
menu goes through them (`load_trace`):

| State | Function | Does |
|---|---|---|
| `0xb` | `SaveDataMachine_BuildList` (`0x006f0220`) | `SaveDataMachine_ScanFiles` (`0x006f0a70`: opens `ffx_000`..`ffx_199`, notes each one's modification date and time), sorts entries 1..199 newest first (`SortSaveTable_006f0fb0`, `g_SortedSaveTable` `0x00ce7318`, 3 words an entry), fills the Flash list; then `0xc` |
| `0xc` | `0x006f0140` | the list's input (`0x006ef320`): a pick opens the Yes/No (`0xd`), cancel ends (`0x11` with no file) |
| `0xd` | `0x006effa0` | Yes/No; Yes: `0xe` |
| `0xe` | `SaveDataMachine_ReadFile` (`0x006f0070`) | `_wfopen` + `fread` of the picked entry into the buffer (list entry `top + cursor`, `0x00ce72ec` + `0x00ce72f0`, through `g_SortedSaveTable`; one more when there is no autosave); a 30-frame timer (`0x00ce72e4`); then `0xf` |
| `0xf` | `SaveDataMachine_CheckFile` (`0x006efbd0`) | after the timer: `SaveDataCheckCrc`; a bad file is zeroed and reported (`0x10`) |
| `0x11` | `SaveDataMachine_LoadDone` (`0x006f0020`) | closes the Flash screen, `SaveDataSaveLoadSucceed(3)` (`0x00648540`: manager state 3 -> 4, result 3; queues the save's music, block `+0xe8`, in `0x00c526d0` and sets the texture video mode from `+0xea`), `SaveDataSetDialogState(0)`; then `0xa` (idle) |

So list entry 1 is the newest manual save, not `ffx_001`. The harness
sandbox copies the saves with new modification times, all in about the
same second, which makes the order of the copies (and the sort's ties)
decide it: in the sandbox, entry 1 was `ffx_001` (2026-10-17).

## From the loaded buffer into the game

`HandleSaveDataScreen` (`0x0081ff80`, every frame from `Sg_MainLoop`)
follows the screen state `g_SaveData_ScreenState` (`0x00ccb994`): 1 starts
the load (`SaveDataToLoad`, `0x00648760` -> `SaveDataManager_doLoad`
`0x006f1ce0`: manager `+0xc` = -1, state 3, `SaveDataMachine_StartLoad`
`0x006f01d0` sets the machine's buffer and size, `0x00ce7308`/`0x00ce7304`,
and state `0xb`), then 2. In 2, once loading finished and succeeded, the Al
Bhed check (`SaveDataIsAlbhedLoaded` -1: `g_SaveLoadedPending`
`0x012fb8c0` = 1 and `0x00647e40`, manager `+0` = 1), state 3:
`SaveDataResetToIdle`, `AtelClearForcePlaceId`, state 4: with the pending
flag, `TkMenuJumpToLoadedScene` (`0x008b4ec0`) and state 0.

`TkMenuJumpToLoadedScene` copies the block (`TkMenuCopyLoadedSave`,
`0x008b54a0`: `memcpy(g_SaveData, buffer + 0x40, 0x68c0)`, then
`setDefaultPlayerName`), refreshes the music, sound and HDD settings from
it, sets `g_TkMenuSaveLoadActive` (`0x018663c4`) and the menu help's load
result (`g_MenuHelpExtArg`, `0x00c5a088`) to 1 and puts the save menu to
sleep. It does not change the map.

The title script does. The title is event `test20` (map `0x17`, which
`Sg_MainInit` `0x00820860` jumps to at boot), on map `titl00`. Its Load
choice:

```
0x00fa(1) 0x00e2(1) 0x00aa(1) 0xc02c(0) AtelSetEventMode(2) FadeOut(15)
Native_MenuOpen_Result(0x40100000)      ; 0x401d: the save list (Sg_MenuSetReserve)
if 0x01fa() != 0:                       ; the load result, g_MenuHelpExtArg
    Native_MusicFadeOut(60)             ; the title music out, the save's queued music in
    Native_FadeOut(1)
    0x01c2()                            ; 0x00873d30: block +0xe0 of 2..5 -> 1
    Native_SetMapFadeFrames(0) Native_SetMapFadeType(0)
    Native_MapJump(-1, 0)
```

`Native_MapJump` (`0x00858460`) calls `AtelSetEventJump2(-1, 0, 0)`
(`0x0086ff40`), which needs the running event to hold control (ctrl work
byte 0 bit 7) and calls `TkSetEventJump` (`0x0088ea60`): a negative map
means the one the block keeps to return to, so map `+0` = `+0xba`, entrance
`+0xc` = `+0xb8` (then 0), previous map `+2` = `+0xc2`, `+0xd` = `+0xce`,
and the request `g_TkEventJumpRequest` (`0x0133084c`) = 1. The scene step
`TkUpdateEventJump` (`0x0088d3d0`, every frame) waits out the fade and calls
`TkInitScene(map)` (`0x0088e070`), which opens the map's event.

The writes the script makes before the menu to the block (`0x00fa`
`TkSetEventNewGame`, `0x00e2` `+0x13` bit 1) are overwritten by the copy;
`0x00aa(1)` sets bit 0 of ctrl work 0 `+0x210`, `AtelSetEventMode(2)` the
event mode in `g_AtelMapIdPage` `+3`.

A developer path does the same in one step: `TkDebugLoadViewerSave`
(`0x008b55e0`) reads a file with `debugLoadForViewer`, copies the block and
jumps with `0x0086fec0` to the map and entrance in `+0xb8`.

## `loadsave` (harness)

`loadsave <slot|path>` runs, between two frames on the game thread: the
file into the buffer, manager `+0xc` = -1 and state 3 (what `doLoad` sets),
`SaveDataCheckCrc`, `SaveDataSaveLoadSucceed(3)`, `SaveDataSetDialogState(0)`,
the Al Bhed check, `SaveDataResetToIdle`, `AtelClearForcePlaceId`,
`TkMenuJumpToLoadedScene`, `SaveDataSetScreenState(0)`, then the title
script's natives above (`0x0086f3d0(1)`, `AtelSetEventMode(2)` and the
post-menu ones, called with a script stack made for them) with ctrl work 0
as the current one. `screens::load_save_direct` sends it as soon as the
title event (`test20.ebp`) has opened, about 20 frames after boot; the
opening and the title never show.

Compared with the menu path on the same file (`ffx_001`, snapshots once
the event opened and 120 frames later, `load_compare`): `g_SaveData` (and
with it the party records), both random generators, the save/load
globals, the scene variables and the save buffer are the same 120 frames
in. What differs:

- the Flash list's machine leftovers: state `0xa` vs 0, the timer, the list
  cursor and file count, the Yes/No choice, the machine's buffer pointer
  and size (set again by every save, `SaveDataManager_doSave`), the sorted
  table: the direct load never builds the list;
- heap pointers in the ctrl works (`+0x1c`, `+0x34`, `+0x38`, `+0x48`,
  `+0x204`) and the script caches: the two runs allocate differently (the
  menu path loaded the Flash screens and the opening);
- ctrl work 0 `+0x1f4` (`0x6b` vs `0x6d`) and, just after the event opens,
  block `+0xe` and two message-window words: the snapshots were taken a few
  frames apart (the menu path polls every 30 frames for the event, the
  direct one every 5), and they agree 120 frames in.
