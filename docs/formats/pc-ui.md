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

## Left original

PhyreEngine library code in the same address range (verdicts, not
ported): `0x00a29320` (a `PArray` search by first word, called by
`Phyre_PText_PBitmapFontText_createMeshGeometry`), `0x00a29030` (the
first character of a string not in a set, used by the `PText` parser at
`0x00a290d0`, `0x00a29140`).
