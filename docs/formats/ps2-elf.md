# The PS2 executables (`SLPM_675.13`, `SCES_504.92`)

FFX.exe is a port of the PS2 game's code, and many of its functions still
have a PS2 twin that shows what the remaster changed. This page lists the
twins and the PS2-only facts ffx-demaster (the PS2-look mod) found while
rebuilding the PS2 menus, lighting and saves (sources: ffx-demaster
`docs/ui.md`, `docs/render.md`, `docs/compare.md`, `docs/debugger.md`).

Two ELFs are cited:

- `SLPM_675.13`: the Japanese International release. Most addresses below
  are from it (MIPS disassembly).
- `SCES_504.92`: the German PAL disc (`docs/formats/ps2-disc.md`). Used for
  the save files and the run-time reads in PCSX2 (PINE, DebugServer).

Addresses on this page are ELF virtual addresses unless they say FFX.exe.
FFX.exe names are those of `symbols/ffx.sym`. Where the two ELFs agree
(the menu layout, the save format), the facts hold for both.

## Threads

`WaitSema` is the syscall stub at `0x002f8460` (`SCES_504.92`;
`li v1, 0x44; syscall; jr ra`). All 8 game EE threads wait there at the
start of the game (thread PC `0x002f8468`). One of them is called from
`0x002d2068` (returns to `0x002d20b0`), which also calls `0x0011ad9c` (jal
at `0x002d20c8`). The EE BIOS idle thread's PC is `0x00081fc0` (PCSX2
DebugServer: thread list, a breakpoint on `0x002f8468`, backtrace).

## VU1 programs (`.vutext`)

`SLPM_675.13`'s `.vutext` (VA `0x00578e00`, file `0x479e00`, `0x3850`
bytes) holds two VU1 programs behind DMA ref tags and a VIF `MPG`:

| Program | VA | Instructions | VU address |
|---|---|---|---|
| 1 | `0x00578e10` | 412 | 0 |
| 2 | `0x00579b10` | 1376 | |

Program 1 draws characters. Its entry table at 0..`0x13` branches to
`0x14`, `0x2a`, `0xa0`, `0x3f` and `0x73`; the MSCAL entries in FFX.exe's
`g_ChrPacketDrawModes` (`0x00b5a6b0`) point into it. Its constants in VU
memory (qwords) are:

| qw | Register | Contents |
|---|---|---|
| 0-3 | vf1-4 | screen matrix |
| 4-7, 8-11 | | the shadow matrices |
| 12-15 | vf13-16 | clip matrix |
| 16-19 | vf9-12 | light colour matrix (colours 0-2, ambient) |
| 20-22 | vf5-7 | normal light matrix x bone |
| 35 | vf8 | material colour x shade x 2 |
| 38, 39, 40 | vf17 | and two GS words |

FFX.exe still builds these uploads: `Ch_DrawPs2Packets_0082e610` sends the
matrices (VIF UNPACK `0x6c230000` of `0x023c3ba0`, `0x023c3aa0`,
`0x023c3ae0`, `0x023c3b20`, `0x023c4400`, `0x023c4460`) and
`Ch_PacketCreateMain_008423c0` the mesh (MSCAL 2), but nothing sends the
packets on PC.

Vertex lighting, program 1 `0xcd`..`0xf4`:

```
vf29 = max(0, Lmat x N)
vf26 = Cmat x vf29 + ambient
vf26.xyz *= vf8.xyz
vertex RGBA = FTOI0(min(max(vf26, 0), 255))     (LOI 255.0)
```

The GS then modulates: texture x vertex / 128. Light colours are script
bytes / 255 (`Light_Set` `0x00836520`), so a light of 128 at N.L = 1 gives
254/128, about twice the texture. `Ch_PacketSetMaterialColor_008437f0`
fills vf8 (qw `0x23`): (material byte `+4` x `g_ChrEffectColor` x 2.0,
byte `+5` x `0x013033b0` x 2.0, byte `+6` x `0x013033b4` x 2.0,
`g_ChrEffectAlpha`), then MSCAL 0. The material table is the model
header's `+0x18` (`Ch_PacketSetName`, `0x013033a8`); PS2 player models
have one material, colour `7f 7f 7f 80`. The PC's formula is in
`docs/formats/render.md`, "Character light levels".

## The menu's GS set-up

- Frame buffer 512 x 416 (`0x001182e0`: `0x11e308(0x200, 0x1a0)`). Menu
  vertices are `((x + 0x700) << 4, (y + 0x730) << 4)` (`0x001fbbb8`,
  `0x001fbbe8`), so menu y 0 is the frame's first line.
- The menu's GS init packet `0x001efb10`: `ALPHA_1` (A0 B1 C0 D1, FIX
  0x80), `TEST_1` (ATE 1, ATST NOTEQUAL, AREF 0, AFAIL KEEP), `TEX1_1`
  bilinear, `CLAMP`. A+D builders: `0x001fb908` TEXFLUSH, `0x001fb920`
  ALPHA, `0x001fb988` TEST, `0x001fb9d8` TEX0, `0x001fba58` TEX1,
  `0x001fbaa0` CLAMP. No menu code writes `TEXA`.
- `0x001f0330(x, y, w, h, c0, c1)` and `0x00225720` / `0x001ff3a0` /
  `0x00245a88` (same arguments) draw an untextured alpha-blended gouraud
  quad (GIF PRIM `0x4c` / `0x14c`), c0 left, c1 right: FFX.exe's
  `TODrawBoxXYWHC2` (`0x008f3db0`, same order and blend on screen).
  `0x001eff10` = `TOMakePktBoxXYWHRGBA`.
- `menu.clp` is at block `0x2ea0` (meswin CBP `0x2ea0`, battle `0x2ea4`).
  The texture data of the menu sheets is in `docs/formats/ps2-textures.md`,
  "Menu sheets".

## Shapes and windows

The shape lookup `0x001fd770`: ids 0..198 and 200..398 (id - 200) index
the meswin table (`0x00321328`), 400.. the battle table (`0x00322190`),
600..649 the icon table (`0x00323440`, context `0x001fd980`, dispatcher
`0x001fda78`). The three tables equal FFX.exe's `g_TOLayoutMeswin`,
`g_TOLayoutBattle` and `g_TOLayoutIcon` entry by entry (85, 110 and 25
entries); FFX.exe's `TOMkpShapePartIndexXYWHC2` does the same lookup.
Layout `0x001fd6f8(id)` gives a shape's size (`+8` the width).

| PS2 | FFX.exe | What |
|---|---|---|
| `0x001fa550` | `TOMkpWindowPlain_008f9a40` | the message window (meswin 200-203), same structure |
| `0x001fa328` | `TOAdpWindowFrame_008f1e40` | the common window, meswin `0xc8..0xcb` (see below) |
| `0x001ff2b8(A, B, x, y, w, c0, c1)` | `TOMkpShapeLeftRight_008f5e90` | A stretched to w - width(B) in c0 -> c1, B in c1 |
| `0x001ff1e0` | `TOMkpShapePairStretchB_008f5e20` | A in c0, B stretched c0 -> c1 |
| `0x001ff028` | `TOAdpShapePair_008f0990` | `0x001ff1e0` with an explicit packet (plate L) |
| `0x001ff100` | `TOAdpShapePairR_008f09f0` | plate R |
| `0x001fe800` | `TOMakePktShapeCursor` (`0x008feb00`) | the cursor (below) |
| `0x001fe580(pkt, kind, x, y)` | | the cursor with a packet; `0x00227100(x, y, blink)` wraps it |
| `0x001f4c60` | `TOAdpShapeByKind_008f1b50` | plate by kind; builders `0x001f4820`, `0x001f48d0`, `0x001f49a0`, `0x001f4a70` |
| `0x001f4820` | `TOAdpShapeDaDb_008f1a90` | the list plate: meswin `0xda` at x, `0xdb` at x + width(`0xda`), `0xd1` - width(`0xda`) wide: 209 in all. The PC's builder lacks shape `0xd7` |
| `0x002014e8` | (like `TOMenuDrawChrLayoutPlate_008f0a70`) | the party plate with a box |
| `0x00200c60(chr, x, y)` | `TOMenuDrawChrLayoutPlate_008f0a70` | the party plate with meswin `0xe7` stretched to 512 at y; aeons `0xcc`/`0xcd`, `0xe9`, `0xea` |
| `0x00201768`, `0x002018e4` | `TOMkpShape2ECCCell_008f8d50` / `TODrawSheetCell_008f8bb0` | a `strtex` cell (170 x 14; column n / 9, row n % 9); the notes pair it with both PC cell draws |
| `0x00201a70(label, x, y)` | `TODrawSheetCell_008f8bb0` | black plate `0xda`/`0xdb` 213 wide in `0x80000000` (`0x001ff028`), then `strtex` cell `label` at (+21, +5); called from the menu too (`0x00226218` <- `0x00230708`), so `strtex` is not only a battle sheet |
| `0x00200a60`, `0x00200878` | | scroll bars (`0x00225e90` -> `0x00200878`) |
| `0x00200048` | | face sprite 91 x 98, cells 4 a row (UV `(k % 4) 0x5b, (k / 4) 0x62`) |

The two pairings of `0x001fa328` in the demaster's notes differ: one
gives `TOMkpWindowFrame_008f9b60`, the other `0x008f1e40`. The call
structure decides for `0x008f1e40`: the PS2's `TkMn2DrawCommonWindow`
(`0x00226fe8`) calls `0x001fa328` as FFX.exe's `TkMn2DrawCommonWindow`
calls `0x008f1e40`; `TOMkpWindowFrame_008f9b60` draws the same four
shapes through the packet cursor.

The cursor `0x001fe800`: kind 0 is meswin shape 45, part 0 a ball (80,
144) 40 x 40 at (-42, -11) in `dum75`'s colour (`0x003241cc`, FFX.exe
`g_TOColDum75`, `73 55 0b 80`), part 1 a hand (0, 168) 40 x 24 at (-34,
-3) in grey; sprites cover (u, v)..(u + w - 1, v + h - 1). Kinds 4 and 5
clamp x and y and use shapes 8 and 9. FFX.exe draws kind 0 as the HD hand
(shape 400 on the battle sheet).

Message window kinds: the handler table `0x00323710` holds 6 words a kind
(FFX.exe `g_YiWindowHandlers`). The kind-0 draw `0x00208758` is scissor,
`TOMkpWindowPlain` and the text scissor (x0 + 12, y0 + 4, x1 - 12,
y1 - 4): FFX.exe's `yiWindowDrawPlate`. FFX.exe's kind 0 draws
`yiWindowDrawMenu` instead (the HD plate). Kinds 1 and 2 (`0x00208870`)
draw no plate. Callers of `0x001fa550`: the five kind draws `0x00208758`,
`0x00208a68`, `0x00208ce8`, `0x00208f60`, `0x002091e0`; the yes/no window
`0x002362c0` (FFX.exe `TOMenuDrawYesNoWindow`); `0x00223880`,
`0x002238e0`, `0x002239b0` (the last two through a table at `0x00223cd8`)
and `0x0023d758`, not identified. `0x001fa0f0`, `0x001fa210` and
`0x001fa448` read the same colours and have no reference.

## Fonts and numbers

| PS2 | FFX.exe | What |
|---|---|---|
| `0x001f43c8` | `TOAddPktBaseMesFontLClutTypeRGBA` (`0x008e92a0`) | 16x12 text |
| `0x001f3810(text, x, y, colour)` | `TOMkpBaseMesFont_00901cb0` | 16x12 text |
| `0x001f3858(text, x, y, colour, alpha)` | `TOMkpBaseMesFontAlpha_00901ce0` | with alpha |
| `0x001f37d0` | | the help bar's message |
| `0x001f4268` | | text for `TkMn2DrawMesL` (`0x00226358`) |
| `0x001f5e88`, `0x001f5f58(v, x, y, c)` | `TOMkpValueFontMClutRight` (`0x009042e0`) | `"%d"` right-aligned in 12-pixel cells ending at x + 2 (font M) |
| `0x001f63c0`, `0x001f6408` | `TOMkpValueFont3d80_00904310` | the same in font S |
| `0x001f6500(s, x, y, c)` | ~ `TOMkpCrossEasyStrFontSClut` | font S text |

The 16x12 font: glyph info `0x00204cb0` (`+0x10` width, `+0x12` height
18, `+0xe` y offset 0), face `0x001f0ec8` (a sprite w x 18 at the pen),
shadow `0x001f11e8` at (+2, +2), advance width + 2 (edge kind + 4), line
feed + 20 (+ 22 for kind 1). The PC draws the same text smaller
(`docs/formats/system.md`, "The 16x12 font's size").

Fonts M and S are on sheet `0x3d80` (`xfont1208.fmt`; layout in
`docs/formats/ps2-textures.md`, "Menu fonts M and S"): M glyphs
`0x001f6110`, text `0x001f5ff8`; S glyphs `0x001f66f0`, text `0x001f6500`,
`0x001f6560`, `0x001f6630`. The play-time clock `0x0024ad28` (t seconds)
draws `%02d` at x - 18, `:` at x - 28 and `%3d` hours at x - 58 in font S.

Icons: `TOAddPktMenuIconRGBA` `0x001f9798(cursor, x, y, id, r, g, b, a)`
draws a GIF sprite (PRIM `0x15e`) with `TEX0` set twice through
`0x001fb3c8`: PSMT8 with CBP `0x2ec0 + 4 (id >> 4)` and CLD 2 (a CLUT
load), then PSMT4 `0x3dc0` TBW 4 with CSA `id & 15`; UV
`((id % 17) 15, (id / 17) 18)` .. +15/+18. Icon ids 45..50 and the
buttons are shapes 600+ with CLUT 13 row 2.

`0x001f5298(id, x, y, c)` = `DrawCrossMenuIconWeaponName` (`0x008e67f0`):
gear icon `rec[4] * 2 + rec[5] + 1` at (+12, +1), the name (`0x001c0040`)
at (+29, +1), icon `0x31` when `rec[4] == rec[6]`. `0x001f5388` =
`TODrawCrossMenuIconAbilityName` (`0x008f4f10`): ability icon (record
`+0x68`, `0x001bc2e8` = `MsGetRomAbility`) and name. `0x001f5100` =
`DrawCrossMenuIconItemName` (`0x008e6700`).

## Menu screens

Menu records (FFX.exe layout in `docs/formats/menu.md`, "Menu records"):

| Screen | PS2 record | Words | FFX.exe record |
|---|---|---|---|
| Main | `0x00326d40` | `+0x10` ctrl `0x00249b78`, `+0x14` draw `0x0024a4a8` | `g_TkMenuMainRecord` (`0x00c5b2b0`) |
| Items | `0x00326dd8` | ctrl `0x0024f540`, draw `0x00250048` | `g_TkMenuItemRecord` (`0x00c5a2fc`) |
| Abilities | `0x00326e00` | ctrl `0x00250438`, draw `0x00250978` | `g_TkMenuAbilityRecord` (`0x00c5a320`) |
| Status | `0x00325850` | ctrl `0x00229300`, draw `0x0022abd0`, ... | |
| Equip | `0x003258d8` | ctrl `0x0022c438`, draw `0x0022cc28`, `0x0022cce8`, `0x0022ccf0`, close `0x0022ce38`, `0x0022ce70` | `g_TkMenuEquipRecord` (`0x00c5a744`) |
| Overdrive | `0x00325a78` | ctrl `0x00230aa0`, draw `0x002312d0`, `0x00231348`, `0x00231350`, close `0x002315a8`, `0x002315e8` | `g_TkMenuLimitRecord` (`0x00c5a768`) |
| Customize | `0x00325aa0` | ctrl `0x00232a60`, draw `0x00232b30`, `0x00232b58`, `0x00232b60`, close `0x00232c18` (stops scene0, scene21), `0x00232c80`; init `0x002323a0`, layout init `0x002315f0` | |
| Config | `0x00326008` | ctrl `0x002353f8`, draw `0x00235a68`, page `0x00235ab0`, open `0x00235b50`, close `0x00235c10`, `0x00235c38` | `g_TkMenuConfigRecord` (`0x00c5a5e4`) |

The words were read from the ELF (`+0` control, `+4` draw, `+8` page or
layout, `+0xc` open, `+0x10` close, `+0x14`). The demaster's scene notes
cite `0x00325a80`, `0x00325aa8` and `0x00326010` for Overdrive, Customize
and Config; those are the records' `+8`. More records at `0x00325d40`,
`0x00326048`, `0x00326150`, `0x00326de0` are not identified.

Main menu: party rows and footer `0x0024a4a8` (`rect_nameplt`,
`col_nameplt`, `rect_placeplt`; it sets `num_namepltspc` = 44 and
`num_compltspc` = 32), option list `0x0024af70` (`rect_complt`,
`col_complt`), option window `0x0024b148`, help bar `0x00243888` (FFX.exe
`TkMenuMain_DrawHelpBar_008a9350`), `0x00249a20` = `TkMn2GetMainPos`. The
background is `0x00245a88(0, 0, 512, 416, c0, c1)` (an untextured quad,
PRIM `0x14c`, c0 left); FFX.exe's `TODrawMenuBGC2` (`0x008f5c60`, only
caller `0x008aa338`) keeps only its alpha over `ffx_bg`. `0x00243f98`
packs `col_menubg0/1` from `0x003241fc..ff`.

Items and abilities: list window draw / row / cursor / opener
`0x0024f0e8` / `0x0024ecf0` / `0x0024efd0` / `0x002500a8` = FFX.exe
`MenuItem_ListWindowDraw_008c7750` / `MenuItem_DrawListRow` /
`MenuItem_DrawListCursor_008c7af0`; target window `0x0024dcd8` =
`MenuItem_TargetWindowDraw_008c80b0`; sub-command bars `0x00236e68` /
`0x00237188` = `MenuSubCmd_DrawRow_008e3cf0` / `MenuSubCmd_DrawItems_008e4140`,
openers `0x002373e0`, `0x00237600` (item and ability, minimum width 381),
`0x002376b8` = `TkMenuOpenSubCmdWindowMain_008e4790`,
`TkMenuOpenSubCmdWindowRect`, `TkMenuOpenSubCmdWindowBeside`; brush
`0x002020a8` (shapes `0xe2`-`0xe4`).

Status and Equip:

| PS2 | FFX.exe |
|---|---|
| `0x0022ad00` (Status main window; opener `0x0022adb8` sets `+0x10`) | `MenuStatus_DrawMainWindow` |
| `0x0022a770` (called by Status and by Equip `0x0022cc28`, page 0) | `MenuStatus_DrawPage` |
| `0x0022a9a8` | `TkMn2MainStatusPlayerStatus` |
| `0x002297a8` | `TkMn2DrawStatusBG` |
| `0x00228b88` | `TkMn2GetStatusBG` |
| `0x0022b5a0` | `MenuStatus_WindowDraw_008d2790` |
| `0x0022b280` | `TkMenuGetDispCommand` (same arguments) |
| `0x00228c48` (called from `0x002346e0`) | `TkMn2GetStatusPos` |
| `0x00228b98` | `MenuStatus_GetPos` |
| `0x002299b0` | `MenuPlayer_DrawFaceSlide` |
| `0x00229fa0` | `MenuStatus_DrawEquipPlate` |
| `0x00229d60(x, y, chr)` | `TkMn2DrawOverDrive` |
| `0x00229828(x, y, g, max)` | `TkMn2DrawLimitParam` |
| `0x00225f38` | `MenuStatus_DrawLayoutRow` |
| `0x002271b8` | `TkMn2DrawStatusPlate` |
| `0x00325808` (stat table, (7, 0x2f) .. (0xe, 0x36), -1) | `g_TkMenuStatusStats` (`0x00c5a790`), the same values |
| `0x00224958` | `MenuHelp_Ratio` |
| `0x00224980` | `Menu_Scale32` |
| `0x002292c0` | `MenuMn2_Ease` |
| `0x0022ba00` | `MenuEquip_InitLayout_008cfc40` |
| `0x0022bdc0` weapon list, rows `0x0022bc20` / `0x0022bd20` (first - 1 .. first + 5), cursor `0x0022bcf8`, abilities `0x0022baa8`, cursor `0x0022bbd0` | |

The Overdrive plate `0x00229d60` moves its pieces by (x, y) -
`rect_lgplt`; Status passes `rect_lgplt` (`0x0022a8f8`), Overdrive
`rect_2lgplt` (`0x0023131c`). `0x00229cf0` copies a common-plate record
moved by (x, y). The gauge `0x00229828` draws the `rect_lg` box, width v /
max, colour between `col_lmtg0` and `col_lmtg1`, full `col_lmtgf0`.
`0x00226298` -> `0x00201ae8` draws the title brush (`0xda`/`0xdb`, 213
wide, black at the shade alpha) and a `strtex` cell at (+21, +5).

Overdrive: `0x00230708` = `MenuLimit_WindowDraw_008d16f0` (both PS2
openers `0x002308c0`, `0x002309a0` set `+0x10` to it, as
`MenuLimit_OpenWindow` and `MenuLimit_OpenModeWindow` do); list
`0x002305c0`, 9 rows x 2 columns `0x00230488`, row `0x00230328`, cursor
`0x00230568`; layout `0x0022fb38` = `MenuLimit_InitLayout_008d0cd0`;
windows `0x00693c50` / `0x00693c54` = `g_TkMenuLimitWindow` /
`g_TkMenuLimitModeWindow`.

Customize: equipment list window `0x00693d58` (draw `0x00232188` =
`MenuKaizou_Draw_008d6860`), abilities-to-add window `0x00693d5c` (opened
by `0x00232948` = `MenuKaizou_OpenListWindow`, draw `0x00232730` =
`DrawGearCustomizationMenu`), state `0x00693d9c` =
`g_TkMenuKaizouState`, and:

| PS2 | FFX.exe |
|---|---|
| `0x00231e70` | `MenuKaizou_DrawRow` |
| `0x00231f90` | `MenuKaizou_DrawRows` |
| `0x00232160`, `0x00232668` | `MenuKaizou_DrawCursor_008d65f0`, `MenuKaizou_DrawCursor_008d5df0` |
| `0x00232568` | `MenuKaizou_DrawRow_008d5e60` |
| `0x00232690` | `MenuKaizou_DrawRows_008d5d50` |
| `0x00232030` | `MenuKaizou_DrawWeaponAbilities_008d63f0` |
| `0x00231e18` | `MenuKaizou_DrawFadeBand_008d6340` (PS2: a `col_menubg0/1` box; PC: shape -2) |
| `0x002324d8` | `MenuKaizou_DrawWeaponTitle` |
| `0x00231dd0` | `MenuKaizou_WeaponByte4` |
| `0x002317c8` | `MenuEquip_IsSelectable` |
| `0x00227800` | `TkMn2ItemRequireWindow` |
| `0x00227ed8` | `TkMn2GetKaizou` |
| `0x002315f0` | `MenuKaizou_InitLayout_008d6be0` |

Config: `0x00235a00` draws the page, `0x00235720(0, 0, row)` +
`0x00225b28` per row; `0x00235720` is one row (`MenuConfig_DrawOptionRow`
`0x008cbae0`), `0x00235718` jumps to the cursor `0x00227100`. Option
records `0x00325d60`.. (0x30 apart, 11 records) and row lists
`0x00325f70`, `0x00325f98`, `0x00325fb8`, `0x00325fe0` (9, 8, 10, 9 rows;
chosen by `0x0020b858` and `0x002d92d0`). Layout init `0x00234a38` =
`MenuConfig_InitLayout_008cc050`. Globals: `0x00693f60` open state,
`0x00693f68` i8 cursor row, `0x00693f6c` row count, `0x00693f70` row list.
The open and close (`0x00235b50`, `0x00235c10`) start and stop the menu
scenes `scene4` and `scene17` (`0x00227488` / `0x00227498` =
`TOSeqStartScene` / `TOSeqStopScene(1, name)`); FFX.exe's do not, and
started on PC they draw a dark blue band sliding up at the right edge.

Shared menu helpers:

| PS2 | FFX.exe |
|---|---|
| `0x00226358` | `TkMn2DrawMesL` (text with the font shade colours `0x003257e8..0x00325800`) |
| `0x00226f30` | `TkMn2DrawCommonPlateR` |
| `0x00226e98(rect)` | `TkMn2DrawCommonPlateL` (A fixed in c0, B stretched c0..c1) |
| `0x00226fe8` | `TkMn2DrawCommonWindow` |
| `0x00226fc8(rect)` | a record box (the gradient quad `0x00225720` from a common-plate record) |
| `0x00225808`, `0x00225910` | scissor (x, y, w, h), scissor reset |
| `0x00225b08`, `0x00225b28` | kicks |
| `0x00225bf0(slot, x, y, w)` | the brush: `0x00201d50` (restart on a new x/y: w, p 0, steps w / 14 + 2) + `0x00201f58` (p++, width (p / steps)^2 by `0x00201c20`; shape `0xe2` at x - 5, `0xe3` from x + 11, `0xe4` at the end); slots at `0x006672d8` (`g_TOBrushSlots`) |

## Globals that moved

PS2 addresses and their FFX.exe places (names from the menu sequence,
`docs/formats/menu.md`):

| PS2 | FFX.exe | What |
|---|---|---|
| `0x0069be58` | `0x0186a394` | `g_TkMenuItemWindow` |
| `0x0069bec8`, `0x0069bed0` | `0x0186a400`, `0x0186a404` | `p_linmline`, `p_rinmline` |
| `0x0069bed4`, `0x0069bed8` | `0x0186a408`, `0x0186a40c` | `g_TkMenuTargetFromItem`, `g_TkMenuItemTargetMode` |
| `0x0069bee0`, `0x0069bee8` | `0x0186a410`, `0x0186a418` | `rect_iscroll`, `rect_abscroll` |
| `0x0069be68`, `0x0069be6c` | `0x0186a3a4`, `0x0186a391` | `g_TkMenuTargetMask`, `g_TkMenuAbilitySingle` |
| `0x01d57300`, `0x01d56b00` | `0x01597770`, `0x01596f70` | `g_TkMenuItemList`, `g_TkMenuImportantList` |
| `0x00326030`, `0x00326034` | `0x023cc128`, `0x023cc124` | the sub-command windows |
| `0x006672d8` | `0x018762e0` | brush slots |
| `0x00326da8`, `0x00326da0`, `0x00326d98`, `0x00326da4` | `0x023cc138`, `0x023cc144`, `0x023cc148`, `0x023cc140` | `rect_noactive`, `pos_noactive`, `col_noactive0/1` |
| `0x0069be30`, `0x0069be38` | | `pos_nmblkline`, `rect_nmblkline` |
| `0x00693880..0x00693a70` | `0x0186a83c..0x0186a9f8` | the status layout (`TkMn2GetStatusPos`); common plates `0x006938e8` + 0x18k = `0x0186a888` + 0x18k |
| `0x00325878..0x003258b0` | `0x023cc3e0..0x023cc420` | the status layout's second part |
| `0x003241cc` | `0x025d16ac` | `dum75` |
| `0x00324304`, `0x00324308` | `0x025d0af4`, `0x025d0af0` | `dum91`, `dum92` |
| `0x00325022` | `0x023cc090` | `dumm79` |
| `0x003194d0` | `0x0112ca90` | `g_SaveData` |
| `0x00318c76` | `0x0112c256` | `g_BtlFieldIndex` |
| `0x0068f440` + 0x100 seq | `0x01865f20` | `g_TOSeqSlots` |

The layouts of Equip, Overdrive, Customize and Config are in
`docs/formats/menu.md`, "Screen layouts", with both addresses.

## Battle windows

`TOBtlSetCallback` `0x00210398` (jump table `0x00570ad0` on i16 `+6`, 23
kinds) stores open `+0x88`, wait `+0x8c`, `+0x90`, draw `+0x80` and
`+0x84`: FFX.exe's `+0x98`, `+0x9c`, `+0xa0`, `+0x90`, `+0x94`
(`g_BtlWindowRecords`). Draws by kind:

| Kind | Draw |
|---|---|
| 0 | `0x00210a40` |
| 1-4, 6, `0xe`, `0x11` | `0x00211cc0` |
| 5 | `0x002123c0` |
| 7, `0xf` | `0x00212580` |
| `0xa` | `0x00214328` |
| `0xc` | `0x00210da0` |
| `0xd` | `0x00211528` |
| `0x14` | `0x002126c8` |
| `0x15`, `0x16` | `0x00212c00` |

All but 0, `0xa` and `0x15`/`0x16` call the shared `0x00211e88` (the PS2
`TOBtlDrawCommandWindow`); the main window's `+0x78` hook is `0x0021bdc8`
(`jr ra`). The box is `0x001f0330` with `dum91` and `dum92` (run time
`42 88 cb 3c` / `42 00 85 6e`).

`0x002105d8` sets the window rectangle (FFX.exe rewrote it with HD
numbers): box x 0 .. max(`dum79`.x + W + 32, `dumm79`), y `dum79`.y ..
+ 20 rows + 2, text from (`dum79`.x + 16, `dum79`.y + 1), W = cols x
colw + (cols - 1) x 16. Parts per kind (`+0x28` count, `+0x2c` u16
kind << 8 / digits) are the PC's, except that the PC adds a kind-1 icon to
kinds 1-4 and `0xe`. `dum79` is (45, 339), `dumm79` 130. Column width
`0x00215bc0`: widest name + parts (1/7: 20, 2/4: 12 d + 5, 3: 12,
5/6/`0x12`: 12 d, `0x11`: widest of the `+0xa4` strings); it stores
`+0x62`, `+0x64` = 18, `+0x66` = widest name.

The battle sequence: `TOSeqDraw` `0x0020d7b8`; `TODrawBtlWindow`
`0x00214ae4` (`TOSeqDraw(2, 0)` before the window callbacks, `(2, 1)`
after); `TOOpenBtlWinMain` `0x002108a0` starts `bnormal{,e,l,t}` + `lay`
by `MsCheckLeftWindow` (`0x001dc368`, 0..3); `TOOpenBtlLimitBreak`
`0x00212f38` (`bevent`, `blimit`, `btwo`); `TOOpenBtlWinLeft` `0x00210d20`
(`bleftw`); `TOMakePktSeqPrimLayout` `0x001fd268` (no special cases);
`TOSeqStartScene` `0x0020d840`; `TOSeqStopScene` `0x0020db20`; scene
names `0x00570b90..0x00570d00`. FFX.exe's `TOOpenBtlWinMain` starts only
`bnormallay`/`bnormalllay` and `TOOpenBtlLimitBreak` only `blimitlay`;
its `TOMakePktSeqPrimLayout_008fe0b0` draws shape 413 only at y <= 300
and skips 411, 415 and 507.

`col_bsoplt`, `col_bitemplt`, `col_babplt` and `col_lmtplt` (PS2
`0x003241d8`..`0x003241f4`) are read only by `TOAdpShapeByKind`
(`0x001f4c60`), whose only caller `0x00226d90` is called from the menu
module (`0x00227218`..`0x0024ef3c`): they colour menu plates, not battle
rows.

Battle words: `TOBtlDrawDamageValue2` `0x0021f4b0` -> `0x001f30f0(kind, x,
y, alpha)` -> `0x001f9b50` draws from CBP `0x2ecc` (`menu.clp` CLUT 13),
CSA `13 - kind`, rectangles at `0x00570000` (OVERKILL (0,110)-(164,127),
IMMUNE (164,96)-(231,118), MISS (55,96)-(152,110), MP (0,96)-(45,106)) at
1:1. FFX.exe's `TOMakePktBtlIconA` (`0x008fbcc0`) uses HD-pixel UVs
((108,452)-(550,506), (632,382)-(952,474), (304,392)-(464,432),
(0,382)-(73,420)) at `X2(310)` x `Y2(38)` and so on.

`strtex` is loaded only in battle, by `0x0020ca40`, at TBP `0x3f00` TBW 8
(the menu loader `0x0020c898` puts `battle.fmt` there), and drawn by
`0x00201768` / `0x002018e4` with CBP `0x2ecc` CSA 7 (TEX0
`0x0705d985e5423f00`), cells 170 x 14 at `((n / 9) 170, (n % 9) 14)`.

Meswin layout entries 59 ((0,224) 136 x 16, the Overdrive label) and 64
(bar (120,88) 136 x 24 at (-16,5) + the label at (28,3)) are passed by no
code as an immediate; `jppc/menu/battle_script.bin` has u16 `0x0103` once
at `0x6a8c`.

## `TOSeqInitParam`

`TOSeqInitParam` is `0x00209bc8` (names at `0x00570370`..), with the same
92 reads in the same order as FFX.exe's (`0x008ae6f0`). The PC passes the
out pointers mirrored: `docs/formats/system.md`, "`TOSeqInitParam`'s
outputs".

## Save files

The memory card names, the file layout and the checksum are in
`docs/formats/save.md`, "The PS2 save file".
