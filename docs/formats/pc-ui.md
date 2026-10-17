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

## Left original

PhyreEngine library code in the same address range (verdicts, not
ported): `0x00a29320` (a `PArray` search by first word, called by
`Phyre_PText_PBitmapFontText_createMeshGeometry`), `0x00a29030` (the
first character of a string not in a set, used by the `PText` parser at
`0x00a290d0`, `0x00a29140`).
