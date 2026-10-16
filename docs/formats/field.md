# Field structures in memory

The field's runtime structures, as the ported `FFX.exe` functions read
and write them. Code: [`crates/ffx-field`](../../crates/ffx-field) (one
module per family, each with a `bridge`); parity tests:
`crates/ffx-oracle/tests/field.rs`. All values little-endian, offsets in
bytes. Minimap rendering: [renderer.md](../renderer.md) "Minimap".

## Minimap mesh (`rcbgModel` and its OMD blob)

A model keeps its blob at `+0x14` (`rcbgModelCreateFromOMD`, `0x00927d70`)
and, for the minimap mesh, its vertex-to-world matrix at `+0xc0`.

| Blob offset | Type | Meaning |
|---|---|---|
| `0x04` | `u32` | Offset of the first packet (`rcbgModelGetPrimHeader`, `0x009282c0`) |
| `0x08` | `u32` | Offset of the vertices, three `i16` each |
| `0x10` | `i16` | Primitives in all packets (`rcbgModelSetAlpha`, `0x00928460`) |
| `0x12` | `u16` | Vertex count |
| `0x20` | | Where `rcbgModelSetAlpha` starts the packets (it does not read `+0x04`) |

A packet is a 16-byte header, then `count` records of the type's stride
(`g_GuidePacketStrides`, `0x00c5e790`: `0x14 0x1c 0x20 0x28 0x18 0x20 0x28
0x30` for types 0..7). Header: `+1` type (`i8`), `+2` count (`i16`);
`0xffff` in the first two bytes ends the list. `rcbgModelGetPrimHeader(m, n)`
skips `n` packets (`0x10 + stride * count` each, signed); `rcbgModelGetPrim`
(`0x009282a0`) is that `+ 0x10`. A record starts with one RGBA colour per
vertex: three for types 0..3, four for 4..7 (`rcbgModelSetAlpha` writes
the alpha bytes `+3 +7 +0xb [+0xf]`). Type 0 records keep three vertex
indices at `+0xc`.

On the salvage ship deck (`field.snap`) the walk-mesh minimap is one
type-0 packet of 156 triangles at `+0x20`; the end marker follows it, the
140 vertices start at `+0xc70`, right after the marker. The battle
snapshot's is 2 triangles over 4 vertices.

## Guide map (`rcbgGuide*`, `Guide_*`, `Yn_GuideMap*`)

`g_YnMapSystem` (`0x0193bf80`) points at Yn's field state: `+0` the map,
`+4` the guide object. The map's `+8` is the guide map list: `+0x18`
count (`i16`), `+0x1a` the one shown (`i16`, `Yn_GuideMapGetMapNo`,
`0x00919c70`), `+0x1c` the models (`Yn_GuideMapSetMapNo`, `0x00919da0`).

Guide object: `+0` the map's own minimap model, else `+8` the one made
from the walk mesh (`Guide_MeshObject`, `0x0091d460`); `+0x6c` a float
kept in `0x0193bf7c` over battles (`rcbgGuideBattleBegin`/`End`,
`0x0091ccc0`/`0x0091cce0`, from `Yn_BattleBegin`/`End`); `+0x70` fog of
war on; `+0x74` blink counter. `g_GuideFlags` (`0x00c5e350`) bit 0: may
show (`rcbgGuideSetDispProg`, `0x0091d560`).

The fade (`Guide_FadeStep`, `0x0091d280`) is a state machine:

| `0x0193bf6c` | Shown | Hidden |
|---|---|---|
| 0 waiting | wait + 1; at `g_GuideFadeDelay` (2): wait 0, state 1 | — |
| 1 fading in | level + 1 up to `g_GuideFadeSteps` (6), then state 2 | state 3 |
| 2 shown | — | wait + 1; past the delay: wait 0, state 3 |
| 3 fading out | state 1 | level − 1 down to 0, then state 0 |

Every call then sets `g_GuideFade` = `level * 128 / steps` (signed
division). `Guide_FadeOff` (`0x0091d3a0`) zeroes state, wait, level and
fade.

## Effect tasks (`rcef`)

A task (`rcefTaskInit`, `0x0092ed10`, `0xbc` bytes):

| Offset | Meaning |
|---|---|
| `0x00` | Flags: bit 0 alive, bit 1 run again this frame, bit 2 dead (killed after the frame by `rcefTaskCtrlProc`, after-function skipped) |
| `0x04`, `0x08` | Previous, next task |
| `0x0c` | Sequence functions, 8 levels |
| `0x2c` | 16 bytes of arguments per level (`rcefTaskWait`'s counter) |
| `0xac` | Level, 0..7 (`rcefTaskCallSeq` pushes while < 7, `rcefTaskRetSeqCont_Inner` pops while > 0) |
| `0xb0` | After-function (`rcefTaskProc`) |
| `0xb4` | Destructor (`rcefTaskKill`), default the empty `0x0092ed00` |
| `0xb8` | Priority (`i32`, default `0x8000`) |

`rcefTaskSetSeq` replaces the current level's function; `rcefTaskRetSeqCont`
and `rcefTaskSetSeqCont` (`0x0092ee40`) also set bit 1. A controller
(`rcefTaskCtrlInit`, `0x00930290`) is a head task (priority 0) and a tail
task at `+0xbc` (priority `0xffff`) with the others between them sorted
by priority: `rcefTaskCtrlAdd` (`0x00930220`) inserts before the first
with a greater one; `rcefTaskCtrlIsEnd` tests head.next == tail.

An effect object (`rcefObjInit`, `0x490` bytes) is a task plus: `+0xbc`
mode (bit 1 attached, set by `rcefObjAttachChr` with bit 2 cleared; bits
4..11 by `rcefObjSetMode`), `+0x130` scale, `+0x1b0`/`+0x1f0`/`+0x230`
matrices, `+0x270`/`+0x274` character and element, `+0x280` particle part
(`rcefParInit`, `0xb0` bytes), `+0x330` animation (`rcefAnimeInit`,
`0x90` bytes; `rcefAnimeStart`, `0x0092f470`). Bit 0 of the part's and the
animation's first byte is set while they run (`rcefObjIsParEnd`,
`rcefObjIsAnimeEnd`).

## Smaller objects

- Chunk (`rcdtChunkInit`, `0x0092b840`): `+0` data, `+4` size
  (`0x7fffffff` from `rcdtChunkSetBin`). Parser (`rcdtParseInit`,
  `0x0092bf50`): `+0` chunk, `+4` cursor, `+0xc` 128 separator flags, set
  for `0x00..0x20` (`rcdtParseInitSeparators`, `0x0092c000`).
- DMA packet (`rcPacketData*`): `+0` start, `+4` cursor; opening reserves
  a quadword for the DMA tag, closing appends a GIF tag with EOP
  (`0x8000, 0x10000000, 0, 0`) and writes the tag `0x70000000 | (qwords
  after it − 1)` (`rcPacketDataCloseAdr`, `0x009275b0`).
- Bounding box (`rcBBoxInit`, `0x009214e0`): min at `+0`, max at `+0x10`,
  w = 1; empty is ±`FLT_MAX`.
- Texture transfer (`rcbgTrxInit`, `0x0092d510`): type `0x20002` at `+0`,
  delete and draw functions at `+4`, `+0xc`, the pending transfer at
  `+0x3c` (−1 none). Palette: number at `+8` (`u16`). Layer: width and
  height at `+4`, `+8`; `rcbgLayerLoadCalcCenter` stores their halves at
  `+0x68 +0x64 +0x4c` and `+0x70 +0x6c +0x50`.
- Display objects (`g_YiDObjs`, `0x018df59c`, `0xb4` bytes, count at
  `0x018df588`): matrix at `+0`, rotation `+0x40`, translation `+0x50`,
  group `+0x74` (`u16`), rotation order `+0x76` (`yiGetMatrixDObj`,
  `yiGetMatrixGroup`).

## Floats

The game thread's x87 unit runs at 53-bit precision (`0x027f`). A single
operation on `f32` operands stored back to `f32` gives the correctly
rounded `f32` result; chains kept on the x87 stack (cross products, sums
of squares) are computed in `f64` and rounded once, which matches
(`ffx_field::x87`). `_CIsqrt` is the correctly rounded root at that
precision. Loading and storing a float through the x87 stack quiets a
signalling NaN on the CPU (masked #IA); the oracle's emulator keeps it
signalling, so those cases are not oracle-checked and the port follows
the CPU. NaN arithmetic operands are left out.
