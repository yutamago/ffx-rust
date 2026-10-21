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
slots in both `TkMenuDrawAll` and `TkMenuCloseAll`.

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
| `+0x30` | u16 | row count (`MenuSelList_CursorX` `0x008d1bc0`) |
| `+0x32` | i16 | first row shown (`MenuSelList_DrawCursor` `0x008d15a0`) |
| `+0x3e` | u8 | draw layer 0..8 (reset 1); `+0x3f` reset 1 |
| `+0x40` | u8 | in use |
| `+0x48` | i16 | cursor (row = `+0x48 >> 1` in two-column lists) |
| `+0x52` | i16 | fade 0..0x1000 |
| `+0x54` | i16 | fade step (`0x155` or `0xaa` in the openers) |
| `+0x58`..`+0x68` | words | the opener's arguments (kind, character, index) |

`TkMenuDrawLayer(layer)` draws the windows of one layer; `TkMenuDrawAll`
runs layer 0, each slot's draw, the main task, then layers 1..8.

## Draw cursor and packets

`g_TkMn2DrawY` (`0x0186a238`) is the plate packet writer's cursor: each
plate steps it by 16 and the writer's return value becomes the new
cursor (`MenuMn2_EmitPacket` `0x008c11b0`, `TkMn2DrawCommonPlateL`
`0x008bf9b0`, `MenuStatus_DrawLayoutRow` `0x008c11e0`). `TkMn2DrawKickPacket` (`0x008c0c40`) sends what was
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

`DrawLimitPageOverDrive(chr)` (`0x008d2270`) draws the overdrive page: a
type-0xd plate, two gauge frames and three coloured pieces cut from the
menu sheet (`TOMkpShapeXYWHUV` / `TOMkpShapeXYWHUVC2`, the uv words given
as floats: texel coordinates for some pieces, fractions for others), the
gauge (`TkMn2DrawCrossLimitParam(602, 371, 450, 8, TkMenuGetLimit(chr),
TkMenuGetLimitMax(chr))`) and, for party members (below 8), the mode's
name `TkMenuMenu1Mes(TkMn2GetOverDriveId(chr) + 0x1033)` centred on x 1550.

The sub-windows draw with `MenuStatus_WindowDraw_008d2790` (`+0x10`): a
command page (`TkMenuGetDispCommand(+0x60, +0x58, +0x5c, who, list)`,
at most 22 commands) for the current member (`+0x64` 0) or aeon,
sliding in by `MenuMn2_Ease(+0x52, +0x54)`: down by `ease * 416 / 2 /
4096` while `i16 +0x54` is positive, else across by `ease * 512 / 2 /
4096`. Command `k` is in row `k >> 1`, column `k & 1` (x `210 -
Menu_Scale32(dx, row)` or `970 + ..`, y from 316 (aeons 375) in steps of
63 (58) plus `MenuHelp_Ratio(dy, row)`); only plates within y 416 and x
-512..512 (remapped units) are drawn, each a type-0xe plate with the
command's name (`+0x58` 1: `MenuDrawChrNameOrCommand`, else the
ability (`+0x60` 0) or command icon name). The title goes through
`0x008f9280(+0x68, ..)` in the font shade `i16 +0x52 >> 5`.

`TkMn2MainStatusPlayerStatus` (`0x008d5180`) draws the stats of the
character page from the table at `0x00c5a790`: `(i32 mes, u32 field)`
pairs (messages 7..14 over save-record offsets 47..54), ended by a
negative `mes`. Four go in a column from `remapX2(210)`, the rest from
`remapX2(970)`, `remapY2(81)` apart from `remapY2(552)`; each row slides
in by `MenuHelp_Ratio(ease * 0xd0 / 4096, row % 4)`. For ids outside
14..17 a plate at `remapY2(925)` against the right edge (`remapX2(1806)`)
holds shape `0xf3` and `TOMenuGetStatusMes(0x1a)`.

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

## Main menu options

`g_TkMenuMainOptions` (`0x01871568`) holds 8-byte entries (`+0` menu,
`+4` i16 enabled; `TkMenuMainGetOptionMenu` `0x008e2270`,
`TkMenuMainGetOptionEnabled` `0x008e2290`). `MenuMain_DrawOptionRow`
(`0x008e22a0`) draws option `i` as a bar from the texture rows
`74.7 i .. 74.7 i + 67`, sliding from `y = 3` (PS2 pixels) to its place
as the fade goes 0..0x1000, with alpha `fade / 32`; a disabled option's
name is drawn dimmed. The highlighted option (`0x01871548`) gets the cross
cursor once fully shown.

`MenuMain_DrawCmdWindow` (`0x008e20f0`) is the option window's draw: it
does nothing while the active-menu mask has a bit of `0x80800`; otherwise
it draws the backing (`TOMkpShapeXYWHUVC2` shape -1, UV `1879, 542` to
`1881, 662`, colour `0x80808080`) from `(1280, s * 194 / 4096)` with width
2 and height `s * 805 / 4096`, `s` the slide `0x01871558` as an `f32`,
and, while the slide is non-zero, every option row
(`MenuMain_DrawOptionRow(i, 0, 0, -20, slide)`) with the highlighted one
last. The original keeps the slide in `ecx` across
`TkMenuMainGetMaxOptions` (a bare `mov eax, [0x01871688]`), so that call
must leave `ecx` alone.

`TkMenuAppearMainCmdWindow` (`0x008e1c60`, `(slide, mode)`) fills that
table. Modes `0xc`-`0x10` and `0x18` open a single option (menus `0xd`,
`0xc`, `0xe`, `0xa`, `0xb`, `0x37`) and clear `0x01871554` (the main
menu control waits for it to come back before freeing its window
groups). Any other mode builds an option bit mask from the scenario flag
(`AtelGetScenarioFlag`; 0 counts as "after everything"): bits 0, 1, 4,
8, 9 and 15 always, 2 from flag `0xd2`, 3 from `0x43`, 7
from `0x448`, 6 from `0xd9` when the menu mask lacks bit `0x40` and the
party has more than one member, 5 from `0xd0` when an aeon is enabled.
Option bit `i` goes to slot `u8 0x00c5b2d4[i]` (01 02 04 00 05 06 07 08
09 0a ..., so the slots are below 32) with menu `i`, enabled 1 and a
`u16` kind at `+6` (1 for menu 1, 2 for menus 2-4 and 15, else 0); the
filled slots in slot order become the options. The window
(`0x01871540`: callbacks `0x008e2040`/`0x008e20f0`, kind 2, layer 7)
records the widest option text in `0x01871550`; `0x01871558` is its
slide (`0x3000` when `slide` is set), and the u16 rows at `0x01871668`
(delay `4 * row`) and `0x01871678` (slide, 0) time the rows.

## Item, shop and aeon lists

The menu list at `0x01597770` has `0x200` 4-byte entries, count in
`0x0186a384`: u16 id, then a byte that is the count in the item list
(`TkMenuItemRefresh`) and the text kind in a selection list
(`MenuSelList_CursorX`), then a usable/enabled byte. A selection list
row (`MenuSelList_DrawRow` `0x008d1820`) draws by that kind: 1 a command
icon and name, 2 an aeon name, 3 `TkMenuMenu1Mes(id + 0x1033)`; the
window's i16 `+0x46` scrolls it by `+0x46 * 32 / 4096` pixels.
`MenuItem_DrawKeyUseCells` (`0x008c4770`) draws the key-use row of the
item menu: `MenuItem_CellTable()`'s u16 `+2` cells (at most 8 fit), each
a 4-byte text at `0x0186a280` (an empty one ends the names), centred by
`(8 - n) * i32 0x0186a31c / 2`. The frame of the cell to fill next (the
first empty one, or the last when all are filled) blinks: drawn
`0x80e1f8ff` while the fraction of `graphicGetTime()` is above 0.5, the
others `0x8000c6ff`.
`MenuItem_DrawKeyCursor(_, x, y)` (`0x008c4b60`) puts the command cursor
on the key cell `0x0186a268` (`15 row + col`; keys in groups of five)
unless `0x0186a270` is set: `cx = f32(remapX2(53) * f32(col % 5) +
f32(x))`, `cy = f32(f32(row) * step + f32(y))`, `step =
f32(remapY2(471) / f32(rows))` with `rows` `MenuItem_KeyRowCount()`, at
least 6; groups 0, 1, 2 add `remapX2(684)`, `remapX2(1041)`,
`remapX2(1398)` and `remapY2(423)`.
`TkMn2DrawStatusPlate` (`0x008c1490`) draws one stat: a caption and the
byte at an offset into the character's save record
(`MsGetSavePlayerPtr`). The shop row drawers (`MenuShop_DrawRows` `0x008dabf0`, ten
rows; `MenuWShop_DrawBuyRows` `0x008d8c30` and `MenuWShop_DrawSellRows`
`0x008d8ee0`, nine) and `MenuSummon_DrawGrowRows` (`0x008ce520`, eleven)
draw a fixed number of rows from a first index. The buy/sell panels are drawn
between the packet kicks (`TkMenuDrawKick` twice, `0x008c1790`,
`TkMn2DrawInitPacket` before, `TkMn2DrawKickSyncPacket(w)` after):
`MenuWShop_DrawBuyPanel` (`0x008d7a40`) calls
`TkMn2WeaponBuySellWindow(remapX2(680), remapY2(594), +0x5c, +0x60,
+0x68)`, `MenuWShop_DrawSellPanel` (`0x008d7cc0`) the same at
`remapX2(656)` with `-(+0x60)`, and `MenuShop_DrawBuySellPanel`
(`0x008d9fe0`) `TkMn2BuySellWindow(remapX2(680), remapY2(435), +0x5c,
row, +0x64 + row, +0x60, +0x68)`. `MenuWShop_StockWindowCtrl`
(`0x008d77f0`) keeps byte `+4` of the stock record under the cursor in
`0x0186aba0` (-1 for none). `MenuWShop_InitList` (`0x008d94a0`) fills the
weapon shop's stock from `TkMn2GetExcelData(0x0186ab1c, 0x0186aba8)`: up
to 16 u16 ids from `+2`, each record (`MenuWShop_GetRecord`) getting byte
`+6` = `0xff` and its name; the count goes to `0x0186ab10`, and
`0x0186ab14` says whether an owned weapon has byte `+2` zero.

`MenuItem_KeyRowCount` (`0x008c5180`) counts the key layout's rows of
three: u16 lists ended by 0 (`0x00c5a2f0[i32 0x0186a25c]` for Japanese,
`0x00c5a2c8` English, `0x00c5a260` for languages 9, 10, 11, `0x00c5a288`
otherwise); 9, 10 and 11 without an item target count
`PasswordGetTotalIndex()` instead. `MenuItem_TypeKey` (`0x008c4c90`) copies the character
under the key cursor (`i32 0x0186a268`, five per row) into the first empty
key-use cell: one byte, two for codes `0x06..0x2f`, three for `04` with a
byte below `0x30`, then a zero and an empty next cell.

`TkMenuGetDispCommand` (`0x008d2e10`) lists a menu's commands into a word
array: the entries of `MsMenuStatusNum(0, ..)` from
`TkMenuGetCommandOffset` (4 bytes each, u16 id at `+2`), filtered by
`MsGetSaveCommand` (packed; for party members a missing command with a
menu panel leaves a gap), by membership in place for sub-menu 1, or the
weapon's and armour's abilities for kind 0.
`MenuSelList_DrawRows` (`0x008d1620`) draws a two-column selection list:
nine row pairs from row `first * 2 - 2`, left column at `remapX2(210)`,
right at `remapX2(970)`, from `y + remapY2(403)` down by `remapY2(77)`.
`MenuSelList_DrawList_008d1400` (`0x008d1400`, the overdrive window's
list draw through `0x008d16f0`) puts those rows in the scissor
`TkMn2DrawMenuWindowScissor(ftol(remapY2(475)), ftol(remapY2(545)), part,
i16 +0x46)`: one pass (part 0) at rest, the old rows (part 1) and the
new ones (part 2, `y = ftol(remapY2(545) * +0x46 * -1/4096)`) while the
list scrolls. The scroll bar is `DrawCrossMenuScrollParts(remapX2(955),
remapY2(480), remapX2(10), remapY2(530), +0x32, +0x3a, (+0x30 + 1) >> 1)`
(row pairs), the cursor `MenuSelList_DrawCursor(w, 0xc, 1)` while `i8
+0x45` is not negative.

Gear sorting: `0x008c9bf0` calls `MenuItem_SortGear_008c9c40(list,
start, n)` for each owner's weapons, then armour
(`g_TkMenuItemGearCounts`, the runs follow each other in the list). Each
entry gets a 32-byte ability bit set (abilities `u16` at `+0xe` of its
`MsGetSaveWeapon` record, count byte `+0xb`, `0xff` skipped: bit
`31 - (a & 31)` of word `(a & 0xff) >> 5`), then a selection sort puts
the largest set first (`MenuItem_CompareEntries`, word by word,
unsigned). Equal sets go to the record with more abilities, then to the
first larger `u16` ability (so `0x105` ranks above `0x05`, same bit);
each move swaps the save data too (`SwapSaveGear`). The bit table is
`0x1900` bytes of the frame: 200 entries.

The item list window draws a row with `MenuItem_DrawListRow`
(`0x008c7c30`) by the window's list kind (`+0x43`): 0 items (count right
aligned on `remapX2(636)` at scale 0.68, dimmed when the usable byte is
0), 1 key items (`g_TkMenuImportantList` ids, shown when
`TkMsImportantGet`), 2 commands (byte `+2` is 1 for a command with its
icon and MP cost `+0x25`, 2 for a packet at the integer position). The
cursor blinks with bit 1 of `TkMenuGetTimer()` on the row
`+0x58 + 2 * i16 +0x2c`; for key items it is compared with the key id,
not the row. In gear mode (`0x0186a393`) `MenuItem_DrawGearInfo`
(`0x008c9570`) draws the owner (record byte `+4` when byte `+2` is set)
and the piece it wears of the selected row's kind (byte `+5`: armour,
else weapon) above the selected piece.

`TkMn2BuildItemList_008c2390(kind, arg)` (`0x008c2390`, callers
`TkMn2RefreshList` and two menu screens) clears the item list and its
counts and fills `{u16 id, u8 kind, u8 value}` entries by `kind` (byte
table `0x008c2aa4` into the jump table `0x008c2a7c`; 4, 8..0x12 and
anything above `0x15` list nothing):

| kind | entries |
|---|---|
| 0 | the current character's commands: `TkMenuGetCommandMax/Offset(MsMenuStatusNum(0), 5, chr, 0)`, owned ones (`MsGetSaveCommand`) `0x101`; a missing one with a menu panel (`MsGetSaveCommandMenuPanel`) leaves an empty entry, except for characters 0, 2 and 4. Character 1 instead lists the aeons of `TkMenuSummonEnableMask` (ids 8..15, `0x102`); jump table `0x008c2abc` on the character |
| 1 | the overdrive modes: each byte of `g_TkMn2OverDriveOrder` (`0x00c8765c`, 20) whose bit is set in `TkMn2GetOverDriveType(chr)`, `0x103` |
| 2 | kind 0 for the current aeon (`TkMenuGetCurrentSummon`), never with gaps |
| 3 | the aeon's grow list (`TkMn2GetSummonGrow`, `TkMn2GetSummonGrowMax` 8-byte records: mask byte, `u16` command, `u16` item, needed count byte): kind 5 learnt, 7 fewer items than needed, 4 when the mask has bit `aeon - 8` or key item `0xa022` is owned, else 6; value = record index. No items, no entry, but its `+3` keeps the `0xff` written before the test |
| 5 | weapon customisations (`TkMn2GetKaizou`/`Max`, same record shape; mask bit 2 when `arg + 5` is set, else 1) against the piece at `arg`: its four `u16` abilities at `+0xe` (0 and `0xff` skipped) give `MsGetRomAbility` bytes `+0x69` group, `+0x6a` level, `+0x6b` kind. A row starts at `0xb`; a same-group ability with a higher level makes `0xf`, the same level `0xc` (same kind) or `0xf`; with an ability of kind `0xff` every row of kind `0xfe` is `0xf`; `0xb` with fewer items than needed is `0xe`; no empty slot (`MenuList_IsEmptySlot(arg)`) makes `0x10`. Rows without items follow as `0xff11_0000`, counted in the list but not shown |
| 6, 7 | the inventory (`MsSaveItemTypeBuffer`/`NumBuffer`), slot by slot (empty slots stay zero): usable items (as in `TkMenuItemRefresh`; all for 7) get kind `0x12`, value 1 |
| `0x13`, `0x15` | every weapon (`MsGetSaveWeaponList(-1, -1, 0)`): owned ones without flag 2 kind 1, value 1 when flags `+3 & 0xd` are 0 and `+6` is `0xff`; the others zero (`0x13`) or id and 0 (`0x15`) |
| `0x14` | the current character's weapons of kind `arg` that are owned, `0x101`; for `arg` 1 then an empty `0x0100_0000` entry |

`g_TkMn2ListMax` gets the entries written and `g_TkMn2NameModeMax` the
entries shown, at least 1: for `0x13`/`0x15` up to the last owned
weapon (MsGetSaveWeapon `+2`), for 5 without the missing rows.

`TkMn2RefreshList(kind)` (`0x008c2360`) is `TkMn2BuildItemList(kind,
0)`; `MenuWeapon_BuildKindList(_, kind)` (`0x008cfd20`) builds kind
`0x14` for weapon kind `kind` and keeps the count in `0x0186a620`;
`MenuKaizou_BuildList(k)` (`0x008d6dd0`) builds kind 5 for the record of
weapon `k` of the customise list (`u16` ids at `[0x0186aa38]`). Both go
through the thunk `0x008c2380` (a `jmp`).
`TkMn2RefreshWeaponShopSell`/`Sell2` (`0x008d9550`/`0x008d9560`) refresh
kinds `0x13`/`0x15`. The list windows over these lists are all opened
the same way (`TkMenuMainAllocWindow`, list, callbacks, rows,
`TkMenuMainRegistWindow`):

| opener | list | rows, visible | kept at |
|---|---|---|---|
| `MenuWShop_OpenSellWindow` `0x008da950` | kind 7 | `TkMn2GetListMax`, 10 | `0x0186ac38` |
| `MenuKaizou_OpenListWindow(k)` `0x008d6ce0` | `MenuKaizou_BuildList(k)` | `TkMn2GetNameModeMax`, 9 | `0x0186aa34` |
| `MenuEquip_OpenWeaponWindow(a, kind)` `0x008cfc90` | `MenuWeapon_BuildKindList` | `u16 0x0186a620`, 5 | `0x0186a61c` |
| `MenuLimit_OpenModeWindow(chr)` `0x008d0dd0` | kind 1 | `TkMn2GetListMax`, 7 | `0x0186a718` |

The overdrive mode window restores its scroll (`+0x32`, `+0x34`) and
cursor (`+0x48`) from `chr`'s `u16` in the 4-byte entries at
`0x0186a7a0` and `0x0186a7e0`.
The list windows' `+0xc` controllers pass on to the generic ones by a
row byte `+0x45`: `MenuItem_GearWindowCtrl` (`0x008c7630`) runs
`TkMenuListWindowCtrl_008b44b0` while the window's is not negative,
`MenuLimit_ModeWindowCtrl` (`0x008d0e90`) `TkMn2CtrlSelItemsWindow` while
it is 0, `MenuWShop_SellWindowCtrl` (`0x008d9df0`) the same while the
open sub-command window's (`0x023cc128`) is above 0;
`MenuKaizou_ListWindowCtrl` (`0x008d5810`) runs the list controller,
then `TkMn2DrawSelItemsHelp`.
The customise window draws with `MenuKaizou_Draw_008d6860`: the menu
background, the rows (`MenuKaizou_DrawRows`) between y 315 and 680
(`TkMn2DrawScissorRect` at rest, `TkMn2DrawMenuWindowScissor` parts 1/2
while scrolling, the new rows shifted by `remapY2(675) * +0x46 / 4096`
rounded to `f32`), `0x008d65f0(w, 389, 327)`, the scroll bar `(955, 319,
8, 675)` over `u16 +0x30` rows, the plates `(209, 252)` and `(970, 659)`
(`740 x 60`) with sheet cells 6 at `(364, 264)` and 7 at `(1125, 671)`,
and the character plate at `(210, 165)` for `MenuKaizou_WeaponByte4(w)`.
The list window draws with `DrawGearCustomizationMenu` (`0x008d5f60`):
the same rows (`0x008d5d50` instead of `MenuKaizou_DrawRows`), and for a
row whose `g_TkMenuItemList` entry names a customisation (byte `+3` not
`0xff`) the required item from `TkMn2GetKaizou()` (8-byte entries: `u16
+4` item, `u8 +6` count) in `TkMn2ItemRequireWindow(970, 319, item,
count, 0)`. The weapon title comes from the other window's (`0x0186aa30`)
row.
`MenuItem_OpenGearWindow` (`0x008c9330`) opens the item menu's gear list
over `TkMn2RefreshWeaponShopSell2` (kept at `0x0186a37c`, gear mode off,
rows `TkMn2GetNameModeMax`, 9 visible). `MenuItem_PackGear`
(`0x008ca120`) closes the gaps in that list: each empty entry takes the
next filled one after it (`Menu_FindFlagged`), swapping the save's gear
(`SwapSaveGear`) and the kind bytes, then the list is built again.

Scrolling list windows share one draw pattern (`MenuWShop_DrawBuyList`
`0x008d7aa0`, `MenuWShop_DrawSellList` `0x008d7d20`,
`MenuItem_DrawGearList` `0x008c87e0`): at rest (`u16 +0x32` == `+0x34`)
the rows are drawn once from row `+0x32` at `y = 317`; while the list
scrolls the old rows (part 1) and the new ones (part 2, from row `+0x34`,
`y = 317 - 77 * 9 * i16 +0x46 / 4096`, so `+0x46` is the scroll offset
in 1/4096 of nine rows) are drawn inside `TkMn2DrawMenuWindowScissor(
ftol(312), ftol(10 + 77 * 9), part, +0x46)`, then the scissor is cleared.
The scroll bar is `DrawCrossMenuScrollParts(955, 317, 10, 683, first row,
rows shown, row count)` (1920x1080 units). The equip menu's weapon list
(`MenuWeapon_DrawList_008cf9f0` `0x008cf9f0`) uses the same pattern with
rows `79` high, five a page, the scissor `(8 + 79 * 5, 595)`, part 0 at
rest, and integer row offsets (`ftol(f32(-79 * 5 * +0x46 / 4096))`); its
bar is `(956, 599, 8, 395)`. The weapon shop lists pass
`TkMn2GetExcelData([0x0186ab1c], [0x0186aba8])` to their row draws and
draw their cursor while the sub-command window is open.

The weapon shop's sell window (`MenuWShop_DrawSellWindow_008da470`,
`0x008da470`) is a two-column list (`MenuItem_DrawRowPairs`): rows start at
y 240, 77 high, ten a page, scissor `(10 + 770, 235)`, and the scroll bar
`(955, 240, 10, 760)` counts `(u16 +0x30 + 1) / 2` rows. Its cursor shows
while the sub-command window is open and not on its third entry (`u16
+0x48` 2).
`MenuItem_ListWindowDraw_008c7750` (`0x008c7750`) is the item menu's
list: row pairs at x 210/978 from y 284 (list kind 2, `u8 +0x43`) or 240,
77 apart, `u16 +0x3a` pairs; the scroll bar `(955, 240, 10, 760)` with
`0x0186a408` 1, else `(955, 284, 10, 687)`. While it scrolls it draws two
passes in the scissor from `y0 - 5` to `10 + 77 * pairs`: part 1 with row
mode 2 from `y0`, part 2 with mode 1 from `y0 - (413 - (y0 - 2)) * +0x46 /
4096`. The cursor (`0x008c7af0(w, 210, y0)`) shows while `+0x94` is set.

`TkMn2GetWShopPos` (`0x008d9190`) loads the weapon shop's layout from the
menu sequence (7 rectangles, 12 positions, 5 colours, 3 common plates)
and then grows the `u16` at `0x023cc2fa` (the third rectangle's) by
`0x30`.
`TkMn2GetIShopPos` (`0x008db080`) does the same for the item shop in one
interleaved run: 10 rectangles, 21 positions and 11 colours (the colour
words at `0x0186abec..0x0186ac14`), then five common plates
(`0x023cc240`, `0x023cc220`, `0x023cc1e0`, `0x023cc1c0`, `0x023cc1a0`; the
second and fifth with the extra pair `0xda`/`0xdb`), with no size fix-up.

The weapon shop's first sub-window (`MenuWShop_OpenSubWindow1`) steps
with `MenuWShop_SubWindow1Ctrl` (`0x008d79c0`): like
`MenuWShop_StockWindowCtrl` it clears the stock byte `0x0186aba0` to -1
and, while the sub-command window allows it, runs the list controller;
the byte is then `+4` of the weapon record `MsGetSaveWeapon(u16
g_TkMenuItemList[2 row])` when that record's byte `+2` is set. The item
shop's sell window draws with `MenuShop_DrawSellPanel` (`0x008da730`):
`TkMn2BuySellWindow(remapX2(656), remapY2(436), id, row, owned - row,
-price, gil)`, the buy panel's call with the row taken off the owned count
and the price negated.

The item menu opens with `MenuItem_Open_008c5f20` (`0x008c5f20`): the
lists are rebuilt (`TkMenuItemRefresh(0)`, the key-item list), the list
window goes to `g_TkMenuItemWindow`, and a four-row sub-command window
(`TkMenuItemMes`/`TkMenuItemHelp` of 0, 1, 9, 2, usability
`MenuItem_IsUsable`) opens at `remapX2(210)`, `remapY2(155)`, size
`remapX2(1500)` x `remapY2(54)`. `MenuItem_DrawTargetNames_008c4030` (`0x008c4030`) draws the target
list: from `(165, 496)`, `420 / MenuItem_CursorCount()` apart, a plate
`500 x 60` per `(name, slot)` pair of `MenuItem_CursorTable()` with the
name (`TOMenuGetNameMes`) centred at scale 0.78, and a marker bar under
the slot in `0x0186a260`. Its target face is drawn by
`MenuItem_DrawTargetFace` (`0x008c3ec0`): for `g_TkMenuItemTarget` below 8
the character face slide, from 8 the aeon's, at `(315, 263)` size
`200 x 200` (1920x1080 units), eased by `Menu_EaseCounter` over the words
`0x023cc3c8..0x023cc3bc`.

`MenuItem_SortGearByKind_008ca1b0` (`0x008ca1b0`) runs after
`MenuItem_SortGearByOwner`: inside each owner's group it moves armour
(`MsGetSaveWeapon` byte `+5` set) out of the first `weapons` entries,
swapping each with the next weapon after the weapon part
(`Menu_FindByte(flags, 0, from, n)` over a 0x400-byte frame buffer of
armour flags), so a group is its weapons, then its armour. It returns 0
when no weapon is left to swap with.

`MenuItem_TargetWindowCtrl_008c7430` (jump table `0x008c75fc`, states
0..12 at `+0x28`) confirms a target with circle: a command for all
(`g_TkMenuAbilitySingle` 0) needs a non-zero mask `u16 0x0186a3a4`, a
single-target one the mask's bit for the cursor; the chosen cursor byte
goes to `0x0186a3ac[+0x58]` (`+0x58` 0 party, else aeons). The cursor
wraps over `TkMenuGetPlayerListMax`/`TkMenuGetSummonListMax`.
`MenuSummon_CmdWindowCtrl_008cced0` (jump table `0x008cd05c`) is the aeon
menu's window: circle asks the window's `+0x1c` callback, and the
repeat bits `0x1000`/`0x4000` (or the d-pad's `4`/`8` held alone) switch
to the previous/next aeon.
`MenuSummon_StatusWindowCtrl_008ccb60` (jump table `0x008cce84`) is the
aeon status window (four pages): state 4 waits for input (circle opens
page 1 unless the aeon is `0xe..=0x11`, the magus sisters' range; cross
leaves through 15/16; the d-pad flips aeons as above), then 7/9/11/13
open the pages (`MenuStatus_OpenWindow(1, page, 0x42/0x43/0x47/0x46, 0)`)
and 8/10/12/14 wait on each: circle closes it and opens the next (page 4
back to 3), cross closes it and returns to 3. Entering (0..2) skips
straight to 4; 3 fades the background out (`TMn2SetStatusBGDiff(-0x155)`)
and 7 back in (`0x155`).
The growth window's draw (`MenuSummon_Window2Draw_008ce750`, the
`+0x10` of `MenuSummon_OpenWindow2`) asks `MenuSummon_GrowInfo(+0x48,
aeon, &cost, &gain)`; a cost (not negative) shows the required-items
window `TkMn2ItemRequireWindow(970, 380, cost, gain, 0)` and the right
plate (sheet cell 0x10); the rows (`MenuSummon_DrawGrowRows`) sit in
the scissor from y 365, `remapY2(70) * 9` high, the scroll bar at `(955,
370, 8, 630)`.

`TkMenuCtrlAbility` (`0x008c60b0`, jump table `0x008c6454`, states
`0..=0x12` at `+0x1c`) runs the ability menu: the list window
(`g_TkMenuItemWindow`) and the target window (`0x0186a39c`) answer
through `i8 +0x45` (positive picked, negative cancelled). A cast (state
12) pays `MsGetRomPlyCommand([0x0186a3a8]) +0x25` MP, heals the target
(`MenuAbility_ApplyHeal_008c6d00`, sound `0x80000062`) for target kinds
0 and 2 (`u8 0x0186a392`) and goes back to the target window while the
MP still covers the cost, else closes it. Flipping members with the
d-pad keeps each member's row, column and top row in `u16 0x0186a3b4`,
`0x0186a3c4`, `0x0186a3d4` (indexed by character) and restores the new
member's unless its row is negative.

The equip menu opens with `MenuEquip_Open_008cf830` (`0x008cf830`): it
saves every character's (0..=6) armour, weapon, HP and MP in the word
tables `0x0186a674`, `0x0186a654`, `0x0186a694` and `0x0186a6b4`, seeds the
face words `0x023cc3c4..0x023cc3bc` with `TkMenuGetCurrentPlayerPos()`,
builds the weapon kind list and starts scenes `scene0`, `scene11` and
`scene20`.

The overdrive menu opens with `MenuLimit_Open_008d0a20` (`0x008d0a20`):
for every party member it finds the member's current mode in the
overdrive list (`TkMn2RefreshList(1)`) and remembers its row in
`g_TkMenuLimitCursors` and a scroll in `g_TkMenuLimitScrolls`
(`row / 2 - 3`, clamped to `0..=(rows + 1) / 2 - 7`, 0 below row 7: the
list has two columns and shows seven lines), then opens a two-entry
sub-command window (status messages 2 and 3) and the mode window.

The aeon growth window (`MenuSummon_GrowWindowCtrl_008cd460`, jump table
`0x008cd6a0`, state `0x0186a610`) pages through four status windows with
the start button: each start opens the next (`MenuStatus_OpenWindow(1, k,
page, 0)` with pages `0x42`, `0x43`, `0x47`, `0x46`), cancel closes the
open one and returns to the list (state 1, which slides the status
background back by `-0x155`).

`MenuSubCmd_FlipWindowCtrl_008e3880` (`0x008e3880`, jump table
`0x008e3b34`, states 0..16 at `+0x28`) is a sub-command window that also
flips the shown party member (`+0x44` 0) or aeon (1): its fade `+0x46`
grows by `+0x38` up to `0x1000`, its rows carry their help ids in 12-byte
entries from `+0x5c`, the cursor wraps with the repeat bits `0x8000`
(up) and `0x2000` (down), and with one row (or none) the d-pad's `4`/`8`
held alone flip to the previous/next member while
`TkMenuGetHelpRatio2()` is set.

`TkMenuCtrlKaizou` (`0x008d5540`, jump table `0x008d55ac` on the menu's
state `+0x1c`) opens the customise window (state 0 -> 1), waits for its
`+0x45` to go negative (-> 2), then sleeps menu 9 and wakes the main menu
(states 2..4, stored as 4).

## Config menu

`MenuConfig_DrawOptionRow` (`0x008cbae0`) draws row `row` of the config
menu from the record `GetDerefU32_008cbaa0(row)`: option count `+4`,
current choice `+8`, enabled `+0xc` (0 dims the options and hides the
brush), title `u16 +0x18`, option text ids every 4 bytes from `+0x1c`
(`TkMenuConfigMes`; help `TkMenuConfigHelp`). The `i32 0x0186a448` rows
share 740 PS2 pixels: row `r` sits at `f32(r) * f32(740 / rows)` (kept at
53 bits) below 220. Options are measured at scale 0.68 but drawn at 0.78,
centred on `remapX2(750) + k * remapX2(420) + remapX2(210)`. One row
(`0x0186a438`) is the PC's own (the HD remaster's settings): its title
and options come from `SaveDataGetLoc` (`0x1cd`, options `0x1d0`,
`0x1ce`, .., help `0x1d1`, `0x1cf`, .. into `0x0186a4a0`). The cursor row
is `i8 0x0186a444`.

`TkMn2CtrlSelItemsWindow` (`0x008d0eb0`) runs a two-column selection
window: the cursor `i16 +0x48` counts entries, two per row (bit 0 the
column; left/right `0x8000`/`0x2000` flip it, the last odd entry steps
back one), `u16 +0x30` entries, `i16 +0x32` the top row, `u16 +0x3a`
visible rows. Up/down move two entries and scroll a row past the view
(timer `+0x46` from `0x1000`/`0xf000` by `0x333`, cut short while the
direction is held); page up/down (repeat bits 1/2 with the pad on 1/2)
scroll by a page (by `0x111`). Confirm (`0x20`) asks the `+0x1c` callback
`(w, cursor, picks)` first (negative: wait, 0: buzzer `0x80000003`).

`MenuConfig_DrawRows()` (`0x008cbab0`) draws the page: `TODrawMenuBG`,
then `MenuConfig_DrawOptionRow(0, 0, row)` and
`TkMn2DrawKickSyncPacket2()` for each of `g_TkMenuConfigRows`.
Option callbacks get the option record, whose `+8` is the value:
`MenuConfig_SetBgmType` (`0x008cb0b0`) stores `value != 0` in the save
event block's byte `+0x5ec` and calls `iMusicRefreshBgmType`;
`MenuConfig_SetVibration` (`0x008cafa0`) sets the menu settings' bit 4
(`MsGetSaveMenu()`) for value 0, clears it otherwise, and calls
`TkRefreshActuator`; `MenuConfig_GetSoundMode` (`0x008cb430`) sets
`+0xc` to 1 and the value to 1 when the settings' bit 0 is clear, then
calls `TkRefreshSoundMode`.
`MenuConfig_SetSoundMode` (`0x008cb460`): value 1 clears bits 0 and 7,
anything else clears 7 and sets 0. `MenuConfig_GetVibration`
(`0x008caf60`) sets the value from bit 4 (clear: 1) and `+0xc` to
whether `TkGetControlerActuatorNumber(0, 0)` is non-zero.
`MenuConfig_SetHdd` (`0x008cb060`) sets bit 13 for value 0 and
`MenuConfig_GetHdd` (`0x008cb020`) reads it back (clear: 1), `+0xc` = 1
when `get_hdd_word_518()` is not 3 and `get_hdd_status()` is 1.
`MenuConfig_SetOption1` (`0x008cb4c0`) sets bit 8 and enables row 1
(`GetDerefU32_008cbaa0(1)` `+0xc`) for value 0, clears both otherwise.
The page draw `MenuConfig_Draw(w)` (`0x008cb320`) kicks the packets
(`TkMenuDrawKick` twice, `0x008c1790`, `TkMn2DrawInitPacket`), draws
the rows and ends with `TkMn2DrawKickSyncPacket(w)`.
## Characters and aeons

`g_TkMenuPlayers` (`0x01841c14`) and `g_TkMenuSummons` (`0x01841c38`) are
the u8 ids the menu cycles through, indexed by `g_TkMenuCurrentPlayerPos`
(`0x01841c28`) and `g_TkMenuCurrentSummonPos` (`0x01841c54`)
(`TkMenuGetCurrentPlayer` `0x008a9860`, `TkMenuGetCurrentSummon`
`0x008a9880`). Character ids below 8 are party members; 8 and up are
aeons (`TkMn2DrawOverDrive` draws the Overdrive mode only below 8).
`DrawCrossMenuSummonPlateClut` (`0x008e6e10`) draws a name with HP and MP
as `"%5d/%5d"` and `"%4d/%4d"` (current/max); language 11 widens the
labels. `TkMn2DrawSummonPlateCheckHP` (`0x008ceb00`) picks it while HP is
above 0, else the plate without HP/MP plus the battle count plate. Its
colour is `MenuPlayer_DamageClass`, or 1 for ids above 13 whose row in
the open sub-command window (`0x023cc128`, row `i16 +0x48`) is marked 1
in the bytes at `0x0186a5b0`; a dead character without that mark gets
`0x53`.

## Save/load slots

`g_TkSaveSlots` (`0x01866290`) holds a state byte for each of the 99
slots (0 free, 1 used, 3 marked); `TkMenuGetSaveFileInfo` (`0x008b4e40`)
returns slot `i`'s 0x40-byte record at `0x0159ec70 + i * 0x40` (u32
`+0x10` play time in frames, u16 `+0x18` place id, u8 `+0x1f`).
`MenuSaveLoad_DrawSlotRow` (`0x008b4b70`) draws one row: the slot number
(`"%02d"`), `MenuSave_KindText` (`0x008b5560`) and, for a used slot, the
place name (`AtelGetSaveDicName`) and play time (`TOMenuDrawTimer`). A
slot is drawn dimmed by its state (when byte `0x0186628a` is 1 anything
but 0..2 and 4, else anything but 1).


`TkMenuPreSave()` (`0x008b50b0`) packs and describes the save of slot
`g_TkMenuSaveSlot` (`0x0186628c`), which it sets to 1. Two callers hand
it an already packed save instead: they set `g_TkMenuPreSavePacked`
(`0x01866244`) and the location `g_TkMenuPreSaveDic` (`0x0186624c`),
call it and clear the flag. Unpacked, it runs
`MenuSave_PackSave(g_SaveFileBuffer, TkGetSaveEventAddress(slot), slot)`
(the result, header `+0x10`, is the play time in seconds) with the
location `AtelGetSaveDic()`; packed, the time is the buffer's `+0x10`.
Then `MenuSave_BuildIconSys(slot, time, dic)`,
`g_SaveData_LoadResult` = 1, and the PC description
`SaveDataSetSaveDataInfo(title, subtitle, detail)`:

- title: `SaveDataGetLoc(0x4f)`, `strncpy` 64 bytes;
- subtitle: `SaveDataGetLoc(0x50)` through `SaveDataLocRemoveNumTag` and
  `ConvertGameCodeToUTF8`, 128 bytes;
- detail: `"%s %s\n%s %s"` of `SaveDataGetLoc(0x51)` in UTF-8, the
  location name (`AtelGetSaveDicName(id, 0)` in UTF-8; `id` is the
  location, or for a save packed here the buffer's `u16 +0x18`),
  `SaveDataGetLoc(0x52)` in UTF-8 and the play time
  `"%003d:%02d:%02d"` (from 1000 hours on `999:59:59`).

It returns `SaveDataUpdateIcon(dic)`. The frame is `0x16cc` bytes of
`0x200`/`0x400`-byte text buffers; the port keeps the original's layout,
so a text that fills its buffer runs on into the same neighbour.
## Pad work records

The Tk pad layer keeps a work record per pad (`Pad_UpdateFrame`
`0x00889790`). `Pad_UpdateAnalog` (`0x00889570`) fills its analog part
from a PS2-style packet: `+0x84..+0x87` the stick bytes (`0x80`
centred) from `inputGetCurrDeviceStickInfo`'s floats 2, 3, 0, 1,
`+0x90`/`+0x91` the d-pad pressure bytes, and direction bits of u16
`+0x9a` from bytes `+0x86` (`0x8000`/`0x2000`) and `+0x87`
(`0x4000`/`0x1000`) past `0x10`/`0xf0`.
`Pad_ReadAndUpdate` (`0x00888f90`) is one pad's frame: `Pad_ReadCurrDevice`,
`Pad_UpdateAnalog` with the buttons at u16 `+0x98`, and, when byte
`+0x9d` is set, `inputSetVibration(port, +0xff, +0xfe)` (bytes). `Pad_InitWork` (`0x00889360`)
resets a record: u16 `+0x9d` = `0x100`, byte `+0x9f` = 1, `+0x94` =
`0x5000c`, bytes `+0x9c`/`+0x83` = 0, and the identity button map.

`Pad_ScanPort_00889050(port, slot, pad)` (`0x00889050`, from
`TkUpdateScanControler`) walks a record's connection phase (byte
`+0x9f`). It switches first on `scePadGetState` (jump table
`0x008892fc`; the PC stub always returns 6): 0 resets the record (phase
1), 2 and 6 read the pad (`Pad_ReadAndUpdate`) and poll `0x0062fb20` in
phases 6 and 7, 7 reports a lost pad. Then on the phase (jump table
`0x0088931c`): 1 waits one frame (byte `+0x83`) and probes the pad kind
(`scePadInfoMode(.., 2, 0)`, else `(.., 1, 0)`: 4 goes to phase 2, 7 to
6); 2 sets the main mode (phase 6, or 1 on failure); 3 aligns the
actuators (`Pad_ActuatorIds`, `Pad_SetActuatorPair`, bytes `+0xf7`/`+0xf6`
= 8 - `0x00888a40(..)`, `scePadSetActAlign(.., +0xf0)`, phase 4); 4
becomes 5; 5 is the steady state: `Pad_CheckPressMode(.., 5, pad)`, the
actuators (`TkUpdateControlerActuator`, `scePadSetActDirect(.., +0xf8)`)
and a report. Byte `+0x82` marks the press-mode check. The report sets
byte `+0x80` to the pad's mode nibble (`pad[1] >> 4`) and `+0x81` to 1;
without one `u16 +0x80` = `0x1ff`. The original passes `ecx` (the port)
to `scePadGetState`, relying on `TkGetControlerWork` keeping it.

`MenuSaveLoad_DrawSlotList` (`0x008b4a50`) draws the save/load list
inside the scissor `(0, 0x5a)`..`(0x200, 0x146)` (PS2 pixels): with the
list at rest (`u16 +0x32` == `+0x34`) the rows -1..=5 from y `0x2a`, 0x30
apart (mode 0); while it scrolls, rows 0..5 from `0x5a` (mode 2) and
again `+0x46 * 0xec / 4096` higher (mode 1). Then the cursor at
`0x5a + 0x30 (+0x48 - +0x32)` and the scroll bar `0x00902700(0x1e3,
0x52, 0xa, 0xfc, top, shown, count)`.

## Help files

`TKMallocAndReadSPS2(index, page)` (`0x0088ce80`) loads a help `.sps2`
whole into a `user_malloc` buffer (64-byte alignment, size rounded up to
0x800, first word cleared) or returns 0 when the file is missing. The
path is `/ffx_ps2/ffx/master/new_XXpc` for the language (jump table
`0x0088d0f0` on `TOGetFFXLang()`: 0 jp, 2 fr, 3 sp, 4 de, 5 it, 9 kr, 10
ch, anything else us) plus entry `index` of `0x00c58e44` (page 0:
`/help/<name>/<name>.sps2` for test_proj, now_help, mon_boku, s_monitor,
dvdcopy) or `0x00c58e58` (the `_page.sps2` files).

`DrawMesFontLClutTypeRGBA_ForHelp(key, text, x, y, flags, r, g, b, a)`
(`0x008e75c0`) draws help text through the pen at `0x025d09a0` (bytes
r, g, b, a; f32 x `+4`, y `+8`; flags byte `+0x1b`). Codes by
`TOGetGroup` (jump table `0x008e788c`): group 1 is a line break (x back,
y + 16), group 2 a control code (`07 e1..ef` moves x by `p - 0xe0`, `0a
p` sets the flag nibbles to `(p & 0xf) - 1` and `(p & 0xf0) - 0x40`, `0b
p` draws button icon `p` at 0.65, 1 lower for icons `0x34..0x37`), group
3 a macro (`TOAddPktChangeMacro_help`, unless `key` is -1), group 4 a
glyph (shadow then face, 0.78).

Two 16x12 siblings write into a packet (`g_TOPktCursor` `0x018716cc`,
returned) through the same pen, with byte `+0x1a` choosing an edge font
(1) over shadow and face: `MenuMes_AddPktFont1612` (`0x008e4af0`) and
`MenuMes_AddPktFont1612Icons` (`0x008e4e60`). Both draw `0b` icons at
0.73 and advance by the icon width plus 1. The first has English quirks
(at y `0x163` a leading `48 73 78 82 7f` is skipped; at `(0xd6, 0xbb)` a
text starting with `h` starts at x 200) and places icons at `y + 4.5 +
AdjustButtonIconY`; the second places them at `y + 4.0 +
AdjustButtonIconY` in Japanese and at `y + 5.0` otherwise, 2.5 higher for
the icons `0x20..0x22`, `0x34..0x37`, `0x42`, and `0x30`/`0x48` while
y is below 350. At y `0x163` both lift glyphs while drawing them (1 in Japanese,
else 2 or 3).

`TkMenuCtrlHelp` (`0x008bef50`, jump table `0x008befdc` on `+0x1c`)
drives the help menu through two `.data` pointers: once
`TOMenuGetFirstReadEndFlag()` is set it calls the start at `0x00c416b8`
(`sa_menu_start` `0x00770430` in the image) with the mode in `0x0186a234`
(1 when 0), copies the mode's low half to `u16 0x0186a22c` and moves to
state 2; states 2 and 3 tail-jump to the step at `0x00c416ac`
(`0x0076f470`).

Message scrolling has a language-driven pair beside the system module's
`TOTextScrollInit_008ded00`/`TOTextScrollStep_008ded60`:
`TOTextScrollInitLang_008de8c0` (which `TOTextScrollInit_008ded00`
installs at `+0x118`) sets the scroll speed `+0xac` from
`TOGetFFXLangJmp()` through the byte table `0x008de950` (jump table
`0x008de944`: languages 0, 1 and 16..18 get 30, 2..6 get 25, the rest keep
theirs) and `+0xa8` = `TOCountMessageWords_008b80b0(i16 +4, +0x94)`;
`TOTextScrollStepLang_008de970` is the step without the `+0xb4` test.

`TkMenuMessageSize_008de250` (`0x008de250`) measures a message line by
line with `TOGetMessageWH`: `u16 +0x40` takes the widest line (with the
lead measured first, grown by `TOGetChangeMacroWidth` when `u16 +0x22` is
1) and `u16 +0x42` the tallest.

`TkMenuMessageLayout_008de370` (`0x008de370`) lays a menu message out:
the text pointers `+0x94`, `+0x60`, `+0x78` move past the previous line's
ends `+0x98`, `+0x64`, `+0x7c` (`TOSkipCodesAbove2` finds the new ones),
the size comes from `TOGetMessageWH` into `u16 +0x44` (width) and `+0x46`
(height; windows flagged in `0x025d0b20[id]` wrap at width 300, the height
multiplied by the line count), the window is centred in `(+0x40, +0x42)`
from `(f32 +0x30, f32 +0x34)` (window mode 1 keeps y 1), and the
rectangle `+0x48` comes from `TOWideWindowRect_008b8d10`, except on map
`0x17` for message `(0x1c7, 0xc7)`, which is pinned at x `0x154` (language
3) or `0x160`. It ends with `+0x2c` = `0x10001`, `+0x114` = `+0x11c` and
the seven words at `+0xd4` copied to `+0xb8`.

Menu messages come in two kinds, each with a table of method words (two
copies each): `0x00c5b1b0`/`0x00c5b1d4` holds
`TkMenuMessageWidth_008de210` and `TkMenuMessageLayout_008de370`,
`0x00c5b200`/`0x00c5b224` holds `TkMenuMessageSize_008de250`,
`TkMenuMessageLayout2_008de5d0` and `TOTextScrollInit_008ded00`.
`TkMenuMessageLayout2_008de5d0` (`0x008de5d0`) counts the words above
the text (`TOCountMessageWordsAbove_008b8140(id, text, 3)` into `u16
+0x24` and `+0x28`, which also counts the pen `0x025d09bc`'s), measures
the lead (`TOGetMessageWH(.., flags | 3)`, width in `u16 +0x2a`, grown by
`TOGetChangeMacroWidth` for `u16 +0x22` 1) and then the rest from the
text it returned (`.., flags | 1`, height into `+0x46`); `u16 +0x44` is
the rest's width plus the lead's (`+0x22` 1) or the rest's alone. `+0xa8`
counts the words (plus the pen macro's, `TOGetChangeMacroAddress(0,
pen)`), and `u16 +2` is set when all of them are above (`+0xa8` equal to
`+0x28`, or to `+0x24` for the other lead kind), except in Eternal Calm
scenes (`isEternalCalmScene(g_GraphicMapId)`). The window is centred as
in the first kind (no wide or map `0x17` cases) through
`TOWideWindowRect_008b8d10`; `u16 +0xe` = 0 when words are above, and
the choices are laid out (`TOLayoutMessageChoices_008b9a60(m, 1)`) when
`u16 +0x1e` is 1 and `i16 +0x1c` positive.

`TkMenuCtrlWorldMap` (`0x008e2960`, jump table `0x008e2a84`) runs the
world map through states 0..10 at `+0x1c`: read the map image (CD file
`0x12`/`0x17` into `0x023cc480`), wait for the read (`0x008abb00()` 0),
upload it (`TOTransImageHL(.., 0x3800, 0x13, 0, 0, 0x200, 0x1a0)`), open
the window, wait for its `+0x64` to reach `0x80`, wait for the cancel
button (trigger bit `0x40`, sound `0x80000004`), stop scene `scene30` and
set `+0x68` to -6, wait for `+0x64` to return to 0, then close and sleep
menu `0x17`.

`TkCdReadWithMalloc2(file, size_of, min)` (`0x0088d240`) reads a CD file
through the `GetCD()` object's methods (`+0x10` open, `+0x14` size, `+0`
read, `+8` wait) into a `user_malloc` buffer (64-byte alignment, at least
`min` bytes, rounded up to 0x800, first word cleared). The two Sqs info
loaders `0x0088e3a0`/`0x0088e440` (CD files `0x2d`/`0x2c`, into
`0x0081d340`/`SqsInitWaveInfo`) run only when the `.rdata` byte
`g_GsDefDrawEnv2` (`0x00b5ec4f`) is 0; it is 1, so they never run.

## Splines (`TkMtCalcSpline*`)

`0x0088aa40` (the body of `TkMtCalcSpline3` and its tangent
`0x0088aba0`) clamps `t` to 0..1 and, for more than 3 points, picks the
segment index with an `ftol` (per Ghidra; not ported yet), moves it until
the local `t = (t - index / (n - 3)) * (n - 3)` is in 0..1, keeps it in
`0..n - 4` and calls `TkMtCalcSpline3Segment_0088a500(out, index,
t, points, position)`; up to 3 points go to `0x0088afd0`, which uses
`TkMtCalcSplineSegment_0088abd0`. Points are 16 bytes. Both are uniform
B-splines evaluated on the emulated VU0 (`g_Vu0Acc` `0x00c8f790` and the
registers after it):

| segment | position weights | tangent weights |
|---|---|---|
| cubic (`0x0088a500`) | `(1-t)^3/6`, `t^3/2 - t^2 + 2/3`, `-t^3/2 + t^2/2 + t/2 + 1/6`, `t^3/6` | `-(1-t)^2/2`, `1.5t^2 - 2t`, `-1.5t^2 + t + 1/2`, `t^2/2` |
| quadratic (`0x0088abd0`) | `(1-t)^2/2`, `t(1-t) + 1/2`, `t^2/2` | `t - 1`, `1 - 2t`, `t` |

Each weight is stored as `f32` (the cubic also stores `1 - t`, `t^2`,
`t/2` and `t^2 * t/2`). A nonzero `position` writes the point; 0 writes
the tangent normalised (`x^2 + y^2 + z^2`, `MsDivSqrt` into `g_Vu0_Q`,
`w` from `vf00.w`). The quadratic writes three weights into a 16-byte
frame buffer and copies all 16 bytes, so `0x00c8f7dc` gets a frame word
nobody wrote (no helper reads it).

## Battle end

`TkBtlEndCheckLegendWeapon()` (`0x008ba5c0`) counts the six weapon spoil
slots (`u16` at `0x02310f8e`) whose `MsGetSaveWeapon` record is in use
(`+2`) with any of flags `+3 & 0xd`. For exactly one, its name
(`MsGetSaveWeaponName`) becomes macro 7's value and the message is
`TkBtlEndGetText(0x401b)`, otherwise `0x401c`; the message goes to
`0x025cc590` and the count is returned.
`BtlEnd_AddAP_008bdaf0(chr, ap, mark)` (`0x008bdaf0`) marks a level-up
(`TOBtlEndSetFlagMark(chr, mark)`) when the AP saved
(`BtlEnd_GetSaveNextAP`) plus `ap` reaches `BtlEnd_GetNextAP` (signed);
at sphere level 98 (`BtlEnd_GetSP`) `ap` is cut to the AP still missing.
It then hands `(chr, ap)` to `0x008ba390`.
`BtlEnd_CountApStep_008ba4e0(chr, mark)` (`0x008ba4e0`) is one step of
the AP count-up: from sphere level 99 the rest (`0x02310f20[chr]`) is
dropped (0 returned); otherwise `BtlEnd_ApStep(chr)` AP, at most the
rest (signed), go to `BtlEnd_AddAP_008bdaf0` and the rest shrinks by
them; it returns 1 while AP were left over. `BtlEnd_CountApSteps`
(`0x008ba550`) runs it for every entry `k` of the battle-end list
(`0x01869f18`, `i16` character at entry `+6`, `mark` = `k`) and returns
1 while any had AP left.

`TkBtlEndInit(_, kind)` (`0x008bdb70`) sets the battle-end screen up:
texts `0x4004`, `0x4013`, `0x400a` into `0x025cc594`, `0x025cc590`,
`0x025cc58c`, the gear list (`MsGetSaveWeaponList(-1, -1, 0,
&0x0186a188)`, kept in `0x025cc580`) with its cursor on the first free
entry (record byte `+2` 0) shown three rows below the top (top in
`0x0186a190`/`0x0186a194`, at most `count - 7`; cursor `0x0186a1a0`,
0..5 from the top). The spoil slots past `i8 0x02310f72` are freed; when
the spoils don't fit (`MsGetSaveWeaponAmount`) `0x0186a184` is set,
otherwise each spoil is added (`MsAddEquipment_007ab910`). It ends with
four `Math_SemicircleTable`s of radius 12 (`0x01869f90` 0x15 entries,
`0x01869fc0` 0x79, `0x0186a0b8` 0x2c, `0x0186a118` 0x22).

Two battle-end pages slide in by an integer `dx` (as `f32`, added to the
remapped x): `BtlEnd_DrawGilPage_008bbb10` (`0x008bbb10`) draws the gil
plate (`(210, 700)` size `740 x 60`, sheet cell 12) and two
`TkMn2DrawGillPlate` rows, the gil won (`0x01869f14`) at y 786 and the
party's gil (`MsGetGIL`) at y 838; `BtlEnd_DrawWeaponPage_008bbed0`
(`0x008bbed0`) draws, from `(970 + dx, 270)`, a plate and sheet cell 6
above, then one plate and `DrawCrossMenuIconWeaponName` row per gear
spoil in use (the six `u16` at `0x02310f8e`), `remapY2(84)` apart;
`BtlEnd_DrawItemPage_008bbca0` (`0x008bbca0`) does the same for the items
won: `i8 0x02310f70` rows of `u16` item `0x02310f74[i]` and `u8` count
`0x02310f84[i]` (drawn right-aligned at x 637, scale 0.68). All
coordinates are 1920x1080 units through `graphicUiRemapX2/Y2`.

`BtlEnd_DrawExchangePage_008bd670` (`0x008bd670`, called with the
slide by `0x008bd940` between `0x008bd280` and `0x008bd020`) is the
weapon page's twin for exchanging gear: y from 250, the title plate 66
and sheet cell 0xe 54 above, rows 66 apart, the name lit when the
record's byte `+3` & 0xd is set, a second full scissor after the title.
Its cursor (`TOMakePktShapeCursor(0, ftol(remapX2(180) + left),
ftol(remapY2(66) * row + y0 + 3))`) follows `0x0186a19c`: in mode 1 the
row `[0x0186a1a0 + 4 * mode]` (the cursor), or with a pick
(`0x0186a1a8`) the picked slot `0x0186a1a4`, blinking on odd
`TkMenuGetTimer() / 2`; other modes show it only with a pick.

`TkInitSaveWork` (`0x0088df40`) resets the save event record
(`MsGetSaveEventAddress`) for a new game (`+4` = -1, `+0x20` =
`0xffffff00`, byte `+0x13` = bit 0 kept | 2, byte `+0xd0` = 2, the eight
bytes `+0xac` = 0..7, the rest of `+0..+0xec` and the 56 words at
`+0x10c` cleared), then runs `AtelInitSaveWork`, `TkRefreshSoundMode`
and, in Asia mode (`FFXGetAsiaMode_007826a0` through the jump
`0x0086a7f0`), `MsSetSaveConfigEnglish(1)`. `TkMenuCopyLoadedSave(dst,
buffer)` (`0x008b54a0`) copies the `0x68c0` bytes after a save file's
`0x40`-byte header and runs `setDefaultPlayerName`.

`BtlEnd_DrawSpoilListPage_008bd280` is the third page `0x008bd940`
slides (at `-dx`): the gear held, from x `210 + dx`, y 250, rows 66
apart drawn by `BtlEnd_DrawWeaponPlate_008be0d0` inside the scissor
`(left - 5, y0, 750, 66 * 6 + 6)`. At rest (`0x0186a190` ==
`0x0186a194`) it draws rows `top - 1 ..= top + 6`; while scrolling six
from the old top (fixed 2) and six from the new one shifted by `66 * 6 *
[0x0186a18c] / 4096` (fixed 1). In mode 0 (`0x0186a19c`) the cursor marks
row `[0x0186a1a0]` and blinks while a pick is held; in other modes it
shows only with a pick. With no slide it adds the scroll bar `(955, 250,
10, 396)` over `[0x0186a188]` rows, 6 shown.

`BtlEnd_DrawCaption_008b9f30` (`0x008b9f30`) draws the battle-end
caption at the bottom right: with `u16 0x0186a168` 0 the text
`0x025cc594` (`TkBtlEndGetText(0x4004)`) on one plate at y 925, else
`0x025cc58c` (`0x400a`) split at its first line code (the byte is 0 while
it draws, 1 after) into two lines on plates at y 870 and 925. A plate is
`remapX2(80)` wider than the widest line (scale 0.78), at least
`remapX2(740)`, right-aligned at `remapX2(1806)`, the text centred in it.
