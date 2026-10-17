# PC UI (`ffx-pcui`)

The PC port's own screens are Iggy (Flash) movies driven from C++
(`IggyMenu`, `IggyMenuManager`, `FFEscMenu`); the PS2 sphere grid screen
(`Abmap*`, data at `g_Abmap`) sits in the same part of `FFX.exe`. What is
ported is the state and data logic; calls into Iggy, PhyreEngine and
Direct3D stay original (glue).

## Iggy data values

`IggyDataValue`, 16 bytes, filled before `IggyMenu_callIggyAS3External`
passes arguments to ActionScript:

| Offset | Size | |
|---|---|---|
| 0x00 | 4 | type: 3 boolean, 4 number, 5 UTF-8 string |
| 0x08 | 4 | boolean: the byte, zero-extended |
| 0x08 | 8 | number: `f64` |
| 0x08 | 4 | string: pointer (not copied) |
| 0x0c | 4 | string: length without the terminator |

Setters, each a no-op on a null value:

- `iggyDataValueSet(v, int)` (`0x00a28d30`): number; the integer goes
  through an `f32` (`fild`, `fstp dword`, `fld dword`, `fstp qword`), so
  integers past 2^24 lose their low bits.
- `iggyDataValueSet(v, const char *)` (`0x00a28d70`): string, `strlen`.
- `iggyDataValueSet_00a28db0(v, bool)` (`0x00a28db0`): boolean.

## Iggy menu

| Offset | Size | |
|---|---|---|
| 0x04 | 5 × 0x14 | handler slots |
| 0x70 | 1 | enabled (`IggyMenu_setEnable`, `0x00a28ef0`) |

A handler slot: used byte at `+0`, then four words at `+4..+0x14`; the
fourth is the function called with the other three.
`IggyMenuHandlers_add(this, a, b, c, handler)` (`0x00a28e40`) takes the
first slot whose used byte is 0, stores the words and sets the byte to 1;
`IggyMenuHandlers_addCall(this, f, arg)` (`0x00a28df0`) stores
`(0, f, arg, 0x00a28c00)`, a trampoline that calls `f(arg)` (the key
handlers such as `onDefaultKey`, `0x0068f6b0`). Both return `al` = 1
over the upper bytes of their last argument, or `eax` = 0 when all five
slots are used.

## Sphere grid helpers

`g_Abmap` (`0x02305834`): nodes of 0x28 bytes at `+0x808` (kind `u16` at
node `+6`), kind sizes `i16` at `+0xf834` (0x30 each), zoom level byte at
`+0x115cc`.

- `AbmapZoomScale()` (`0x00a56820`): 1, 0.5, 0.25, 0.125 for zoom levels
  0..3 (jump table), 1 otherwise; in `ST0`.
- `AbmapNodeRadius(node)` (`0x00a56f30`): `(size >> 1) + 3.0`, the shift
  arithmetic on 16 bits, the sum in `f64` stored as `f32`; in `ST0`.
- `AbmapRangesOverlap(a, b, c, d)` (`0x00a57000`): 1 when `[a, b]` and
  `[c, d]` overlap: `c > a` needs `b >= c`, `c < a` needs `d >= a`, equal
  or unordered starts give 1. Its caller `0x00a49270` passes two boxes' y
  ranges.
- `AbmapResetVectors()` (`0x00a57120`): the eight vec4 at
  `g_Abmap + 0x11520..0x115a0` from the emulated `r0` (zero, `g_Ee_Gpr`),
  then `w` = 1.0 in the first four; `eax` = `g_Abmap`.
- `AbmapReadPad(no_stick, pad)` (`0x00a57520`): the sphere grid's pad
  state, `eax` = 0:

  | Offset | Size | |
  |---|---|---|
  | 0x00 | 2 | held now (`TOMenuGetControlPad2`; with `no_stick` 0 the stick, `TOMenuGetControlPadAnalogHist(0, 0, 3, -1)` + 0x80 as a signed byte, adds 0x1000 below -0x40, 0x4000 above 0x40) |
  | 0x02 | 2 | pressed this frame (`~held & now`) |
  | 0x04 | 2 | held last frame |
  | 0x06 | 2 | repeat: pressed, or held 6 frames or more |
  | 0x08 | 2 | first: pressed, or every third repeat frame |
  | 0x0a | 16 | frames held per button, up to 6 |
  | 0x1a | 16 | repeat cycle per button, 0..2 |
- `AbmapTryChangePlayer()` (`0x00a58ec0`, Fahrenheit's name): unless a
  state is stacked (`+0x115b0`), pad edge bit 4 (`TkMenuSetPrevPlayer`,
  direction 1) or 8 (`TkMenuSetNextPlayer`, direction 2) at `+0x11672`
  changes character: direction `+0x115be`, previous `+0x115bd`, current
  `+0x115bc` (`TkMenuGetCurrentPlayer`, a byte; same character clears the
  direction, another sets `+0x116ac` = 1, `+0x116a8` = -1 and plays
  0x80000004), then `AbmapInitHoming(node at +0x110cc + 0x50 * current,
  0.25)`, `AbmapState_HomeOnNode` pushed over `+0x115a8` (into `+0x115b0`)
  when nothing is stacked by then, `AbmapCheckAchievements`,
  `AbmapMarkEmptyMenus`. Always ends with a tail call of
  `AbmapResetCursorCamera`, whose `eax` it returns.
- `AbmapFlagConnectedLinks(flag)` (`0x00a58df0`, Fahrenheit's name): for
  each node (count `i16` at `+2`) of a kind other than 0xffff whose record
  `MsGetExcelData(kind, [0x01a860e0])` has bit 0x8000 at `+0x10`, ORs the
  `flag` byte into `+0xe` of every link (`+0xa808`, 0x14 bytes: node `u16`
  at both ends, count `i16` at `+4`) touching it. `eax`: the node pointer
  past the last, 0 with no nodes.

### Name tags

Seven party members' name tags on the grid, 0x50 bytes each at
`g_Abmap + 0x11088 + 0x50 * chr`:

| Offset | Size | |
|---|---|---|
| 0x00 | 16 | position (the node's `i16` x, y as `f32`), 0, 1 |
| 0x10 | 16 | anchor, 0, 1 |
| 0x20 | 12 | three words from the caller |
| 0x2c | 4 | name (`TOGetSaveChrName`) |
| 0x30 | 2 | name width: `ftol(max(TOGetTextWidth(name, 1), [0x00b41d84]))`, a NaN or equal width kept |
| 0x32 | 2 | 16 |
| 0x3c | 4 | radius (`AbmapNodeRadius` of the node) |
| 0x40 | 4 | scale |
| 0x44 | 2 | node |
| 0x46 | 8 | box offsets (`i16`): horizontal, vertical, text, height |
| 0x4e | 1 | corner: bits 0-1 which, bit 2 the wide box |

- `AbmapInitPlyTag(chr, node, scale, a, b, c)` (`0x00a57f80`, name ours):
  fills the table's fields; the name and width only for `chr <= 6`
  (signed). Returns the tag.
- `AbmapPositionPlyTag(chr)` (`0x00a58080`): `AbmapSetupPlyTag(tag)`
  (which leaves box words at `0x02305804..0x02305834`), then the anchor:
  `x + cos(a) * r`, `y - sin(a) * r` (x87, stored `f32`) with
  `a = corner * 0x4000 + 0x2000` in `u16` turns (`g_pppSinTable`, cosine
  at `a + 0x4000`), the corner read before the call; the box offsets from
  the corner read after it: the words at `0x02305814`/`0x02305818` (bit
  2) or `0x0230581c`/`0x02305820` give vertical/horizontal, the vertical
  negated for corners 0 and 1, the horizontal for 1 and 2; the text
  offset is `[0x02305810]` for corners 0 and 3, else
  `-([0x02305830] + [0x0230580c])`; the height `[0x02305808]` for corners
  0 and 1, else `[0x02305804]`. Returns the tag.

## Pointer zones

Mouse and touch input on the PC screens (`GuiTouch_*`, names ours). The
update at `0x0068a3a0` resets the buttons, then for each pointer event
of the input device calls `GuiTouch_hover` (moves: kinds 0, 4, 5) and
`GuiTouch_handleEvent`. Counts mask off their top bit.

| Offset | Size | |
|---|---|---|
| 0x150 | 4 | hover regions |
| 0x154 | 4 | → regions, 0x150 bytes each |
| 0x158 | 4 | button groups |
| 0x15c | 4 | → groups, 0x1c8 bytes each |

A region: active byte `+1`, the hit rectangle index `+4` (-1 none), 32
rectangles of 10 bytes from `+0xe` (`i16` left, top, right, bottom). A
group: active byte `+1`, 8 buttons of 0x38 bytes from `+8`:

| Offset | Size | |
|---|---|---|
| 0x00 | 16 | left, top, right, bottom (`f32`) |
| 0x10 | 1 | this frame's event: 1 hover, 2 press, 3, 4 release |
| 0x14 | 1 | pointer id that pressed it |
| 0x15 | 1 | copy of `+0x10` |
| 0x16 | 1 | phase: 1 held, 2 released outside, 3 moved while held inside, 4 off |
| 0x17 | 1 | released this frame |
| 0x18 | 4 | press time (`graphicGetTime`) |
| 0x1c | 8 | release minus press position |
| 0x24 | 8 | press position |
| 0x2c | 8 | release position |

Event: `+8` x, `+0xc` y (`f32`), `+0x18` kind, `+0x1c` pointer id.
Tests are strict (`<`) on all four edges; any NaN misses.

- `GuiTouch_resetButtons` (`0x0068ab30`): clears `+0x10`, `+0x15`,
  `+0x17`; phase 3 or 4 becomes 0; phase 0 zeroes the time.
- `GuiTouch_hover(x, y)` (`0x0068b6d0`): `+4` of each active region.
- `GuiTouch_handleEvent(event)` (`0x0068b470`): first each button under
  the pointer: kinds 0/4/5 set `+0x10`/`+0x15` to 1; kind 1 records the
  pointer, phase 1, press position, sets 2 and the time; kind 2 sets 3.
  Then each button whose pointer id matches: kinds 0/4/5 set phase 3 if
  inside and held, else 4; kind 3 (release) moves phase 1 to 2 when
  outside, sets 4, `+0x17`, the end position and the drag.

`GuiTouch_initGroup(group)` (`0x0068a420`, fastcall) empties a button
group: `+0` (`u16`), `+4`, `+0x1c4` (`u32`) zero; in each button the
rectangle, `+0x10`, `+0x16`, `+0x18` and `+0x34` zero and the `u16` at
`+0x14` 0xff (no pointer); the drag fields stay.

## Speed-up booster

`SpdCtrl_ScaleElapsedTime` (`0x006f7340`, Fahrenheit's name) scales the
frame time while the PC booster is on, except where these say no (names
ours):

- `SpdCtrl_IsMapExcluded(map)` (`0x006f6fa0`): 1 for the maps at
  `g_SpdCtrlExcludedMaps` (`0x00b4bf74`: 0x3e, 0x163, 0x181), and for map
  0x118 outside battle (`MsBattleGetStatus` not 1) while
  `getScenerioFlag` is 0x71c..0x721 (asked only then).
- `SpdCtrl_IsMagicExcluded()` (`0x006f6ff0`): `al` = 1 when a magic file
  is loaded (`MagicFile_GetHandle` not -1) and its magic
  (`MagicFile_GetMagicId`) is 0x15b, 0x186..0x18d, 0x19b..0x19e, 0x1a9,
  0x1b7..0x1bd, 0x1bf..0x1c2, 0x1eb..0x1ef, 0x1f4, 0x2b9 or 0x2ba (two
  jump tables; signed range checks), unless `g_BtlWindowOpen` is set and
  `TOGetBtlWindowControlEnable()` returns 1. The upper bytes of `eax`
  are what the switch left (-1 with no magic file, the id less the range
  start when outside both tables, the id for 0x1b7..0x1bd, the control
  call's result), reproduced by the port.

## Item menu pieces

Three item menu functions the PC build placed among its UI code (names
ours; `ffx_pcui::item`):

- `MenuItem_ParseTargetKeys(t)` (`0x008c5320`): nothing for `t < 0`;
  else the message id `u16` at `0x00c5a0d8 + 4t` (Japanese,
  `TOGetFFXLangJmp` 0) or `0x00c5a120 + 4t` (the other languages),
  `TOMenuGetNameMes` of it, and `Menu_ParseItemKeys` on the string (a
  tail call; both void).
- `MenuItem_DrawRow(window, x, y, index)` (`0x008c9a70`): for `index` in
  `0..u16 window+0x30`: `TODrawMenuPlateXYWHType(x, y, rx2(740),
  ry2(60), 5)` (`rx2`/`ry2` are `graphicUiRemapX2`/`Y2`); then, when the
  signed byte `+0x45` is positive and `index` equals the `i16` at `+0x2c`,
  `TkMn2DrawCrossCmdCursor(x + rx2(160), y + ry2(4), 1)` (each sum stored
  as a float); then, when `g_TkMenuItemList[index]` byte `+2` is set,
  `DrawCrossMenuIconWeaponName2(MsGetSaveWeapon(id, &local), x, y, 0)`.
  The window bytes are read after the plate is drawn.
- `MenuItem_GetLinePos()` (`0x008c9b60`): `TkMn2SeqGetPos("p_linmline",
  0x0186a400)` and `("p_rinmline", 0x0186a404)`.

## rcui sea speed

`rcuiSeaAddUvSpd(axis, step)` (`0x00930c00`, name ours; no callers in the
image, among the `rcui` debug screens): the first time (`g_RcuiWork + 4`
is 0) it sets that word to 1 and the two floats at `g_RcuiWork + 0x2c`
and `+0x30` to 0; then `speed[axis] = speed[axis] + step / 10.0` (x87:
`fdiv` by the double, `fadd` of the float, one store; `axis` is not
checked) and `Yn_SeaSetUvSpd(speed[0], speed[1])`.

## Sphere grid cursor

The cursor's move and the node searches of the sphere grid's cursor
update (`0x00a58ff0`; names ours, `ffx_pcui::cursor`). Floats follow the
x87 code (53-bit arithmetic, rounded where stored).

- `AbmapReadCursorStick(move)` (`0x00a56bd0`): `move` is 4 floats;
  `move[1]`, `move[0]` = 0 first. The stick:
  `TOMenuGetControlPadAnalogHist(0, 0, 2, -1)` and `(0, 0, 3, -1)`, the
  low bytes less `0x80` as `x`, `y`. When either byte is outside
  `0x41..=0xbf` (`(b - 0x41) as u8 > 0x7e`): the acceleration (`f32` at
  `g_Abmap + 0x115e4`) goes up by the double 0.1 (stored, then at most 2)
  when `x² + y² > 0x3840`, else down 0.1 (at least 1); each axis past its
  dead zone (`> 0x40` less 0x40, `< -0x40` plus 0x40, else 0, byte
  arithmetic) gives `move[0] = x * 0.125 * accel`, `move[1] = 0.125 * y *
  accel` (the acceleration read back after its store); returns 1.
  Otherwise the d-pad (`g_Abmap + 0x1166c`): `0x8000` gives `move[0] =
  -1`, else `0x2000` 1; `0x1000` gives `move[1] = -1`, else `0x4000` 1.
  Nothing pressed clears the counter byte `g_Abmap + 0x115ca`; one axis
  alone increments it and, while it is below 2, zeroes the move (from 2
  it stays 2); both axes pass at once. `move[2]`, `move[3]` = 0, 1;
  returns 0.
- `AbmapFindNearestNode(dist, pos, filter)` (`0x00a56a40`) and
  `AbmapFindNodeInDirection(pos, dir, filter)` (`0x00a56860`, returns the
  node in `eax` though Ghidra says void) walk the nodes (`0x28` bytes from
  `g_Abmap + 0x808`, `i16` count at `+2`, the end `g_Abmap + (5n + 0x101)
  * 8`), skipping those without a kind (`+6` = `0xffff`) or refused by
  `filter(node)` (cdecl, none when 0). The offset is `(f32)(x - pos.x)`,
  `(f32)(y - pos.y)` from the node's `i16` position. Its length goes
  through the emulated VU0 as the PS2 code did: `vf10` (`0x00c8f830`)
  gets `(x², x² + y², x² + y² + 0², 1)` in place, `g_Vu0_Q` =
  `sqrt|sum|` (`_CIsqrt`), `vf10.x = vf00.x + Q` is the length; the
  scratch keeps the last node's values.
  - The nearest: the first node strictly below the best so far (from
    1e9, `0x00b7b70c`); `*dist` = the best; returns the node or 0.
  - In a direction: `dot = FFXVu0InnerProduct((dx, dy, 0, 1), dir)` must
    be above 0.5 and the length above 0; the score `dot / (len * len)`
    (the square kept at 53 bits) must be strictly above the best (from 0);
    returns the node or 0.

## Sphere grid panels

`AbmapPadCtrl()` (`0x00a583e0`, name ours, `ffx_pcui::panel`; only
caller `eiAbmCalc`, which reads no `eax`) drives the open list panel of
`g_AbmapPad` (`0x01a86108`): panels of `0x340` bytes, the open one's
index at `+0x2700` (`i16`, negative for none), the handler at `+0x2704`.
A panel's list: `+0x16` rows shown, `+0x18` cursor, `+0x1a` first shown,
`+0x1c` the first shown before a page flip, `+0x1e` count, `+0x23`
shown, `+0x24` flags, `+0x26` scroll animation (`u16`; the panel takes
no input while its low byte is set), entries of 12 bytes at `+0x40`
(byte `+4` locked).

- Repeating buttons (`g_Abmap + 0x11674`): when the list is longer than
  a page, bit 0 flips a page up while the first shown is above 0
  (first shown less a page, at least 0; the cursor moves as much;
  animation 3), else bit 1 a page down while a page fits below
  (`first < count - page`; first plus a page, at most `count - page`;
  animation 4); a flip ends the frame. Otherwise `0x1000` moves the
  cursor up (sound only when above 0; at least 0; scrolls with animation
  1), else `0x4000` down (sound only when below `count - 1`; at most
  `count - 1`; scrolls with animation 2 when `first + page <= cursor`).
- Pressed buttons (`g_Abmap + 0x1166e`): `0x20` chooses: a locked entry
  buzzes (`0x80000003`); else the cursor sound unless flag 8, the panel
  hidden (`+0x23` = 0), none open (`+0x2700` = -1) and
  `handler(panel, entry, cursor, 1)`. Else `0x40` cancels: flag 4 puts
  the cursor on entry 1 (cursor sound), else flag 2 closes the same way
  with `0x80000004` and `handler(..., 0)`, else the buzz.
- Sounds are `SndSepPlaySimple(0x80000001)` unless stated. `i16`
  arithmetic wraps; the comparisons are signed, some in 32 bits as the
  original does them.

## Sphere grid links

The outlines of the links between nodes (Fahrenheit's names;
`ffx_pcui::link`). Both normalise through the emulated VU0 as the PS2
code did: `vf09` (`0x00c8f810`) = offset + `vf00` (`vf00.z + 0` third),
`vf10` (`0x00c8f830`) gets the squares in place (`x²`, `y²`, `0·0`,
`x² + y²` over `vf10.y`... as in the node searches), `sqrt|sum|`
(`_CIsqrt`) + `vf00.x` over `vf10.x` is the length, `Q = vf00.w /
length` (`g_Vu0_Q`), the unit `vf09 * Q` over `0x00c8f820`. The scratch
keeps the last normalisation.

- `AbmapCalcLinkPointsStraight(out, a, b, ra, rb, kind)` (`0x00a581f0`):
  `*kind` = 2; `u` the unit of `b - a`; `a += ra u`, `b -= rb u` (both
  written back); `out` gets `a, (uy, -ux), (-uy, ux)` then the same at
  `b` (12 floats); returns `out + 0x30`.
- `AbmapCalcLinkPointsCurved(out, a, b, c, ra, rb, kind)` (`0x00a57710`):
  around the centre `c`: `ang = _CIatan2` of `a - c` and `b - c` (stored
  as floats), `R1 = |a - c|`, `R2 = |b - c|` (VU0 lengths). `diff = ang2 -
  ang1` (float) brought within ±pi when `ang1 <= ang2` and `diff > pi`
  (less 2pi) or otherwise `diff < -pi` (plus 2pi), pi being `f32` pi as a
  double (`0x00b92288`, `0x00b39378`, `0x00b5c008`). For `diff > 0`:
  `start = ang1 + ra/R1`, `sweep = diff - ra/R1 - rb/R2`, else `start =
  ang1 - ra/R1`, `sweep = ra/R1 + diff + rb/R2` (stored once each). `n =
  clamp(i16(ftol(|(R2 + R1) sweep| / 20)), 2, 6)`, `step = 1/(n + 1)`,
  `dang = sweep step`, `drad = (R2 - R1) step`. Points: `a`, then `n + 2`
  points at radius `R1 + k drad` and angle `start + k dang` (each stored)
  from `g_pppSinTable` (index `ftol(angle * 32768 / pi) & 0xffff`, sine
  `>> 4`, cosine `+ 0x4000` first), then `b`. For each of the `n + 2`
  inner points: the units from the previous point and to the next one
  (normalised without storing `Q`), summed, normalised; the row is
  `(point, (uy, -ux), (-uy, ux))`. `*kind = n + 2`; returns the end of the
  rows (the caller `0x00a5a8e0` returns it).

`AbmapPadActivateHandler` (`0x00a56160`, name ours) is the handler of the
panel that asks whether to keep nodes just activated: see its `ffx.sym`
line and `ffx_pcui::panel::bridge::activate_handler`.

## Left original

PhyreEngine library code in the same address range (verdicts, not
ported): `0x00a29320` (a `PArray` search by first word, called by
`Phyre_PText_PBitmapFontText_createMeshGeometry`), `0x00a29030` (the
first character of a string not in a set, used by the `PText` parser at
`0x00a290d0`, `0x00a29140`).
