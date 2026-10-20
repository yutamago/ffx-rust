# Menu module structures

The in-game menus (main menu, status, items, abilities, equipment, aeons,
shops, save/load, name entry) as the menu module (`crates/ffx-menu`,
`tk.rs`) reads and writes them, recovered from FFX.exe. Integers are
little-endian; addresses are the image's (`symbols/ffx.sym`). Names in
the `Tk*`/`TO*` families are the original symbols (from Fahrenheit's
Switch export, `fh` lines); `Menu*_` names are ours.

## Menu slots (`g_TkMenuSlots`, `0x01840888`)

25 pointers to menu records, one per main menu, set by
`TkMenuMainRegistMenu(slot, menu)` (`0x008aaab0`, which also clears the
flag bytes and calls `+8(menu, mode)` with `g_TkMn2MenuMode`
`0x018408f8`). In a menu record:

| Offset | Use |
|---|---|
| `+0` | close callback (`TkMenuCloseAll` `0x008aa200`) |
| `+4` | draw callback (`TkMenuDrawAll` `0x008aa290`) |
| `+8` | open callback (`TkMenuMainRegistMenu`) |
| `+0x10` | sleep callback (`TkMenuMainSleepMenu` `0x008aad70`) |
| `+0x18`..`+0x1b` | u8 state flags: `+0x18` active; close runs on `+0x18`/`+0x1a`, draw on `+0x19`/`+0x1b` |

`g_TkMenuMainActiveMask` (`0x018408ec`) has a bit per active slot
(`TkMenuMainActiveMenu` `0x008aa100` sets it, `TkMenuMainSleepMenu`
clears it); the next word, `g_TkMenuDrawSkipMask` (`0x018408f0`), skips
slots in both `TkMenuDrawAll` and `TkMenuCloseAll`. `MenuMain_Reset`
(`0x008aa570`) clears the 28 words `0x01840888..0x018408f4`.

## Windows (`g_TkMenuWindows`, `0x01840900`)

32 records of `0x98` bytes. `TkMenuMainAllocWindow` (`0x008aa1a0`) returns
the first with `+0x40` zero, reset by `MenuMain_ResetWindow`
(`0x008aa4b0`); `TkMenuMainRegistWindow` (`0x008aab00`) sets `+0x40` = 1,
`+0x28` = 0 and calls `+8(window)`. Fields seen so far:

| Offset | Type | Use |
|---|---|---|
| `+8` | fn | open callback (`TkMenuMainRegistWindow`) |
| `+0xc` | fn | control step (often `TkMn2WindowStepFade` `0x008d26d0`) |
| `+0x10` | fn | draw (`TkMenuDrawLayer` `0x008a9690` calls it for its layer) |
| `+0x14` | fn | "faded out" test (`TkMenuWindowFadedOut` `0x008d3200`: `+0x52` == 0) |
| `+0x1c` | fn | usability test (name entry, item windows) |
| `+0x28` | u32 | state (0 = idle) |
| `+0x2c`, `+0x2e` | u16 | ids, reset to -1 |
| `+0x30` | u16 | row count (`MenuSelList_CursorX` `0x008d1bc0`) |
| `+0x32` | i16 | first row shown (`MenuSelList_DrawCursor` `0x008d15a0`) |
| `+0x34` | u32 | reset to `0x01000000` |
| `+0x3e` | u8 | draw layer 0..8 (reset 1); `+0x3f` reset 1 |
| `+0x40` | u8 | in use |
| `+0x48` | i16 | cursor (row = `+0x48 >> 1` in two-column lists) |
| `+0x52` | i16 | fade 0..0x1000 |
| `+0x54` | i16 | fade step (`0x155` or `0xaa` in the openers) |
| `+0x58`..`+0x68` | words | the opener's arguments (kind, character, index) |

`TkMenuDrawLayer(layer)` draws the windows of one layer; `TkMenuDrawAll`
runs layer 0, each slot's draw, the main task, then layers 1..8.

## Draw cursor and packets

Menu drawing writes GS packets at a cursor. `g_TkMn2DrawY` (`0x0186a238`)
is the plate writer's cursor: each plate steps it by 16 and the writer's
return value becomes the new cursor (`MenuMn2_EmitPacket` `0x008c11b0`,
`TkMn2DrawCommonPlateL` `0x008bf9b0`, `MenuStatus_DrawLayoutRow`
`0x008c11e0`). `TkMn2DrawKickPacket` (`0x008c0c40`) sends what was
written; the status page draw ends with it.

Coordinates are given for a 1920x1080 screen and mapped to the PS2's
512x416 by `graphicUiRemapX2`/`graphicUiRemapY2` (`0x006447e0`,
`0x00644820`, results in `ST0`). The draw functions remap the height,
width, y and x of each element in that order and add their own offset to
x (and sometimes y) before rounding to `f32`.

## Status menu

`MenuStatus_OpenMainWindow` (`0x008d3100`) opens the main status window
(draw `0x008d2710`), and `MenuStatus_OpenWindow` (`0x008d3140`) the
sub-windows in `g_TkMenuStatusWindows` (`0x0186a820`). The main draw
calls `MenuStatus_DrawPage` (`0x008d4ee0`), which picks the page with a
jump table at `0x008d5134`:

| Page | Draws |
|---|---|
| 0 | character: face slide, `MenuStatus_DrawEquipPlate` (`0x008d4540`), `TkMn2DrawOverDrive` (`0x008d3d70`), `TOMenuDrawChrStatusPlate` |
| 1 | aeon: face slide, `MenuSummon_DrawCommandRows` (`0x008d4280`) |
| 2 | `MenuStatus_DrawLayoutRow` (`0x008c11e0`) |
| 3 | aeon Overdrive: `TkMn2DrawOverDrive` |

The page slides in by `MenuMn2_Ease(t, 1) * 0x155 / 4096` pixels.

Status rectangles are 24 bytes (i16 x, y, then sizes);
`MenuStatus_OffsetRect` (`0x008d3220`) copies one, moves it and can take
the size `g_TkMenuStatusRectSize` (`0x0186a8a4`, the w, h of the
rectangle at `0x0186a8a0`). `TkMn2DrawOverDrive` works relative to the
rectangle at `0x0186a888`; `MenuStatus_DrawPage` places pages 2 and 3 at
the x, y of `0x0186a83c` and `0x0186a8a0`.
`g_TkMenuStatusOrigin` (`0x0186a86c`) is added by `MenuStatus_GetPos`
(`0x008d4e30`).

Aeon commands: `MenuSummon_BuildCommandList` (`0x008d2b40`) fills the
u16 ids at `0x0186aa04`, counted in u16 `0x0186aa24` (groups 1, 3, 4, 5)
and u16 `0x0186aa28` (group 2); `MenuSummon_DrawCommandRow`
(`0x008d4350`) captions the first group `TOMenuGetStatusMes(0x15)` and
the second `(0x16)`. `g_TkMn2StatusBG` (`0x0186a9fc`) holds the status
background's step and value (`TkMn2StepStatusBG` `0x008d3280`).

## Item, shop and aeon lists

The menu list at `0x01597770` has `0x200` 4-byte entries, count in
`0x0186a384`: u16 id, then a byte that is the count in the item list
(`TkMenuItemRefresh`) and the text kind in a selection list
(`MenuSelList_CursorX`), then a usable/enabled byte. `MenuItem_SortGearByOwner` (`0x008c9fb0`) groups
gear by owner using the u16 [weapon, armour] counts per owner at
`0x0186a3e4`. The shop row drawers (`MenuShop_DrawRows` `0x008dabf0`, ten
rows; `MenuWShop_DrawBuyRows` `0x008d8c30` and `MenuWShop_DrawSellRows`
`0x008d8ee0`, nine) and `MenuSummon_DrawGrowRows` (`0x008ce520`, eleven)
draw a fixed number of rows from a first index.

## Characters and aeons

`g_TkMenuPlayers` (`0x01841c14`) and `g_TkMenuSummons` (`0x01841c38`) are
the u8 ids the menu cycles through, indexed by `g_TkMenuCurrentPlayerPos`
(`0x01841c28`) and `g_TkMenuCurrentSummonPos` (`0x01841c54`)
(`TkMenuGetCurrentPlayer` `0x008a9860`, `TkMenuGetCurrentSummon`
`0x008a9880`). Character ids below 8 are party members; 8 and up are
aeons (`TkMn2DrawOverDrive` draws the Overdrive mode only below 8).
