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
`TkMn2DrawStatusPlate` (`0x008c1490`) draws one stat: a caption and the
byte at an offset into the character's save record
(`MsGetSavePlayerPtr`). The shop row drawers (`MenuShop_DrawRows` `0x008dabf0`, ten
rows; `MenuWShop_DrawBuyRows` `0x008d8c30` and `MenuWShop_DrawSellRows`
`0x008d8ee0`, nine) and `MenuSummon_DrawGrowRows` (`0x008ce520`, eleven)
draw a fixed number of rows from a first index. `MenuWShop_InitList` (`0x008d94a0`) fills the
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
