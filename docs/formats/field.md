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
`+4` the guide object. The map's `+8` is its scene (below), which also
holds the minimap models: `+0x18` count (`i16`), `+0x1a` the one shown
(`i16`, `Yn_GuideMapGetMapNo`, `0x00919c70`), `+0x1c` the models
(`Yn_GuideMapSetMapNo`, `0x00919da0`), `+0xf4` world units per minimap
pixel (10.0).

Guide object: `+0` the map's own minimap model, else `+8` the one made
from the walk mesh (`Guide_MeshObject`, `0x0091d460`); `+0x6c` the view
limit, a float: `rcbgGuideUpdateEx` clamps the minimap's offset to minus
it (`rcbgGuideGetLimit`/`SetLimit`, `0x0091d450`/`0x0091d580`; script
native `0x0091b740`), kept in `0x0193bf7c` over battles
(`rcbgGuideBattleBegin`/`End`, `0x0091ccc0`/`0x0091cce0`); `+0x70` fog of
war on; `+0x74` blink counter. The guide's setters write globals and
ignore the object: `g_GuideFlags` (`0x00c5e350`; bit 0 may show,
`rcbgGuideSetDispProg`; bit 3 `rcbgGuideSetFlag3`, no reader found; bit 8
read by the update), `g_GuideTarget` (`0x00c5e354`), `g_GuideSize`
(`0x00c5e358`) and `g_GuidePos` (`0x00c5e35c`, x then y).
`rcbgGuideSetTarget` (`0x0091d930`) maps a target to size and centre:

| Target | Size | Centre |
|---|---|---|
| 0 top left | `0x80` | `0x50, 0x40` |
| 1 top right | `0x80` | `0x1b0, 0x40` |
| 2 bottom left | `0x80` | `0x50, 0x160` |
| 3 bottom right | `0x80` | `0x1b0, 0x160` |
| 4 centre | `0x100` | `0x100, 0xd0` |

The `Yn_GuideMap*` wrappers (`0x00919c00`..`0x00919e80`) call these on
`g_YnMapSystem`'s guide when it exists.

## Scene (`rcbgScene`)

`rcbgSceneInit` (`0x009209e0`, `0x110` bytes): `+0` 1, `+4` layer count
and `+8` the layers (the script natives' "map parts", `0xc0` bytes each,
`rcbgLayerInit` `0x0092ad20`: animation at `+0x90`, palette at `+0xb0`),
`+0xc` object count and `+0x10` the objects (pointers; an object's first
word is its type: `0x20002` transfer, `0x20004` particle simulation
`rcbgPSimInit`, `0x20008` ...; `rcbgSceneGetObject`/`CountObjects`,
`0x00920950`/`0x009209a0`), then the minimap models above.
`Yn_MapPartCount` (`0x0091a3d0`) returns the layer count;
`Yn_MapPartAnimeOn`/`Off` switch bit 1 of a layer's animation word. On
the HD deck snapshot the scene has no layers or objects (the remaster
draws backgrounds through PhyreEngine); the battle snapshot's has one
object.

Yn data chunks are tagged `YNDT YNPR YNSC YNTX YNTM YNSM YNST YNSP YNTT
YNSE YNFT YNGM YNED` (`rcbgChunkTag`, `0x00921b90`, table `0x00c5e480`).

## Background matrices

`rcbgUpdateMatrix` (`0x0092b160`): `g_RcbgViewWorldMatrix` (`0x025d5960`)
= world (`0x025d59a0`, `rcbgSetWorldMatrix`) through `g_CameraMatrix`;
then that through the clip matrix (`0x025d58e0`, `rcbgSetClipMatrix`)
into `0x025d58a0` and through `g_ScreenMatrix` into `0x025d5920`.
`rcCalcPersMatrix(dst, m)` (`0x00920020`) is `m` through camera then
screen. Both use `cdc_Mul4x4_007059b0` (`ffx_system::math`).

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
  for `0x00..0x20` (`rcdtParseInitSeparators`, `0x0092c000`; more with
  `rcdtParseAddSeparators`, `0x0092c0f0`), `+8` the mode
  (`rcdtParseSetMode`).
- Texture transfer entries: `+0x30` count, `+0x34` entry size, `+0x38`
  the entries, each a 16-byte header then data (`rcbgTrxGetEntry`,
  `rcbgTrxGetEntryData`, `0x0092d4d0`/`0x0092d4f0`).
- `Ke` resource heap (`KeThResHd_Init`, `0x00764bb0`): `+0` block count
  (`u8`), `+1` size class (`u8`), `+2` stride = `(class − 1) * 48 + 0x80`
  (`u16`), `+4` the first free block; blocks from `+0x10`, each one's
  first word pointing at the next, the last 0.
- `rcui` work (`g_RcuiWork`, `0x0193c0f0`): `+0xbe8` the ID dragged,
  `+0xbec` `rcuiMouseMoveGetID`, `+0xbf0` dragging. UI values carry a
  type: 1 `i8`, 2 `u8`, 3 `i16`, 4 `u16`, 5 and 6 32 bits
  (`rcuiValueGet`/`Set`, `0x0093c210`/`0x0093c270`).
- Display object colours: two RGBA byte colours at `+0x90`
  (`yiGetColorDObj`/`yiSetColorDObj`, `0x00908e90`/`0x0090a5c0`).
- Effect targets (`g_RcefTargets`, `0x025d5c60`, up to 16, `0x24` bytes,
  filled by `0x00924a20`): `u16` flags (bit 0 reached, bit 1 done), `+4`
  character, `+8` horizontal distance, `+0xc` direction angle, `+0x10`
  height difference, `+0x14` direction (`rcefTarget*`, `0x009249c0`..).
- Field renderer packets: the DMA write position at render `+0xba3f0`
  and the tag count at `+0xba3f8` (`yiRender_addRef`, `0x0090c1f0`: DMA
  `ref` tags, ID 3); matrices go to VU memory as VIF `UNPACK V4-32` of 4
  quadwords (`yiPkUnpackMatrix`, `0x0090cd10`); GS registers as A+D
  quadwords (data, 0, register, 0; `yiPkAddGsAD`, `0x0090ae20`).
- Image VRAM: a bump allocator in 64-byte blocks, next at `0x025d17bc`,
  end `(0x38 − pages) << 14` at `0x025d17ac` (`yiVramAlloc`,
  `yiVramSetLimit`).
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

## Renderer state and textures (round 4)

- Render state save (`yiPushRenderState` `0x00909c60`,
  `yiPopRenderState` `0x00909b90`): the area at `0x01931230` holds the
  render status, colour scale (`0x018df00c`), colour offset
  (`0x018df01c`), a word (`0x018df140`), three floats (`0x018df144`),
  the colour shift (`0x018df150`), then 128 display objects as 100-byte
  records: the object's `+0x88` float, rotation, translation and matrix.
  Floats move through `fld`/`fstp`, vectors through `FFXVu0CopyVector`,
  matrices with `rep movsd`.
- Background colour (`yiGetBackColor` `0x009086d0`): RGBA bytes at
  `0x018df178`. With render status bits 0 and 2 each channel is
  `channel * scale + offset` (each step stored as `f32`), clamped to
  `0..=255` and converted with `_ftol2_sse`; else the raw bytes.
- Battle centre (`0x018df02c`) is kept divided by the four scale factor
  words (`yiSetBattleCenter`, `yiGetBattleCenter`).
- The renderer object `g_YiRender` (`0x01876e00`): `+0x68788` the object
  count animations may name, `+0x9cabc` animations in use, `+0x9cac0` 64
  animation slots of `0x28` bytes (`+0` flags, bit 0 in use; `+4` kind,
  4 default and 1 keyframe; `+6` object id; `+0x14`/`+0x18` start time
  from `yiGetFCount` or `yiGetVCount` by `0x0193bb08`; `+0x24` keyframe
  data), `+0x9d4c4` keyframe data per index, `+0x9d9c4` texture swaps in
  use, `+0x9d9c8` 64 texture swaps of `0x720` bytes (eight words, then
  64 slots of `0x1c`; `+4` the frame shown, `+0x1c` the frame count)
  (`yiAnimAttachDefault_009143c0`, `yiKeyFrame_attachChr` `0x00914630`,
  `yiTexSwap_init` `0x009152d0`, `yiTexSwapSetFrame` `0x00909e30`).
- Map data sections (`yiMapGetSection` `0x00907f00`): data tagged `MAP1`
  has section offsets at `+0x10 + 4 * index`, relative to the data.
- VRAM regions (`yiVramAllocPsm` `0x00918090`): four bump regions
  (`0x025d17a8`: limits at `+0..+0xc`, next addresses at `+0x10..+0x1c`;
  region 0 is `yiVramAlloc`'s). PSMT8 and PSMT4 images that fit neither
  of the first two take four or eight times their size from the last
  two.

## Scene parts, TIM2 and the sphere grid (round 4)

- Fog of war (`Guide_RevealFog` `0x0091f420`): the guide's 64 x 64 bit
  map at `+0x70`; the player's cell is `((x - ox * 10) / 40,
  (oz * 10 - z) / 40)` through `_ftol2_sse` (`ox`, `oz` the minimap
  model's `+0x90`, `+0xa8`), and the 13 cells of a radius-2 diamond
  around it (clamped to the map) are marked seen.

- Per-map FP parts (`rcbgFP.c`, chosen by map in `0x00921a50`): type
  `0x20008` and two functions at `+4`, `+8`: map `0x61`
  (`rcbgFPm097Init`), `0xad` (`rcbgFPm173Init`), `0x235`/`0x236`
  (`rcbgFPm565Init`). `rcfxDampInit` (`0x009293d0`): type `0x30003`,
  factor 0.0001 at `+0x1c`.
- `rcbgFpGetWork(id, part, index)` (`0x00923100`): the FP run record's
  data (`+0x30`), its part list (`+8`), the part table (`+0`), entry
  `0x28 + 16 * part`, its work offsets (`+0xc`): `offset + 0xa0 + data`.
  `Yn_PSimGetHandle(n)` reads the word at work `(n + 0x5b, 6, 2)`.
- Scene waves: `Yn_SceneSetDC` starts one (`+0xdc` mode) that
  `rcbgSceneWaveDCStop` (`0x00920650`) ends; `Yn_SceneSetC0` another
  (`+0xc0`) ended by `rcbgSceneWaveC0Stop` (`0x00921200`).
- TIM2: the picture header's image type (`+0x13`) maps to a GS pixel
  format through `0x00c5e754` (`rcdtTm2GetPsm`); `rcdtTm2ClutAlphaToPs2`
  scales CLUT alphas from 0..255 to 0..128 (`a * 128 / 255`).
- Map part language overrides (`MapPart_InitLangOverrides` `0x0067c620`,
  `MapPart_LangOverride` `0x0067fb30`): two entries of ten words at
  `+0x2c00` of the field object, a map id and nine parts by language
  slot (game languages 0..5, then 9 and 10).
- Sphere grid (`g_Abmap`, `0x02305834`): `+4` link count, nodes of
  `0x28` bytes from `+0x808` (x, y `i16`, kind at `+6`, panel bits at
  `+0x21`), link flags every `0x14` bytes from `+0xa816`, node kind
  sizes every `0x30` bytes from `+0xf834` (`AbmapGetNodePosRadius`
  returns half of it).
- Lens flare shapes (`KeLnsShp_Update` `0x0076abe0`): the state's clock
  (`u16`) moves `0x200` a frame through frames of 8 bytes (duration at
  `+0x12`, bit 7 of `+0x14` loops); `KeLnsClm_SetShp` seeds element
  phases with `s = s * 0x80d + 7` (16 bits) over 65535.0.

## Map parts, sphere grid moves, render set-up (round 5)

- Map part (scene layer, `0xc0` bytes) fields the script natives read and
  write through `Yn_MapPart*` (`0x00919fe0`..`0x0091acc0`; each checks the
  part against `Yn_MapPartCount`): `+4`/`+8` width and height, `+0xc`,
  `+0x10`, `+0x14` (float), `+0x24`, `+0x28`, colour alpha `+0x43`, scale
  `+0x44`/`+0x48`, position `+0x4c`/`+0x50`; a scale change from
  `+0x54`/`+0x5c` to `+0x58`/`+0x60` over `+0x74` frames (counter
  `+0x78`; `Yn_MapPartScaleTo`, a 0.0 target keeps the scale), a move from
  `+0x64`/`+0x6c` to `+0x68`/`+0x70` over `+0x7c` frames (counter `+0x80`;
  `Yn_MapPartMoveTo`); the animation word `+0x90` (bit 0, bit 1, mode bits
  8..11), current frame `+0x96`, frame count `+0x98`, rate `+0xa4`.
  Natives `0x8012`..`0x8056` (entries of 16 bytes from `0x00c5dc9c`;
  setters at slot 0, getters at slot `0xc`).
- Sphere grid (`g_Abmap`): node links at `+0x814` (five pointers per
  node), the cheapest cost at `+0x82c` (`AbmapCalcMoveCost` `0x00a47560`:
  a depth-first walk, step 1 when the link's `+0xc` byte matches the mask
  `0x01a86104`, else 4; limit `0x01a860fc`); the camera at `+0x112b8`
  (copied to `+0x11308`), zoom `+0x112f4`; homing state at
  `+0x11620`..`+0x11648` (`AbmapInitHoming`, `AbmapState_HomeOnNode`:
  `FFXVu0InterVectorXYZ` from the start toward the node); state functions
  at `+0x115a8` (saved at `+0x115b0`); tilt level `+0x115cb` (0, -3640,
  -7281) eased over 30 frames through the sine table `0x00c44bf0` into
  `+0x11340` (`AbmapCtrlTilt`); zoom level `+0x115cc` (1, 0.5, 0.25,
  0.125); the characters' nodes every `0x50` bytes from `+0x110cc`. The
  save (`MsGetSaveAbilityMap`): node kinds and panel bits (2 bytes per
  node), link flags from `+0xa00`, nodes from `+0xf00`, tilt and zoom at
  `+0xf18`, `+0xf19` (`LoadSaveAbmap`).
- GS set-up packets of the renderer (`yiRender_setupparamGS` `0x0090d260`,
  `+0x677a8` and `+0x678a8`): DMA 12 quadwords, VIF `FLUSH`, `DIRECT` 11,
  an A+D tag of 10 registers; the background colour is `b << 16 | g << 8
  | r` from `+0x68340`, through the renderer's own colour scale and offset
  (`+0x6820c`, `+0x6821c`) when flag bit 2 is set (clamped to 0..255,
  `_ftol2`); the frame register from `+0x9cab4 >> 11` unless flag bit 25.
- CLUT upload packet (`yiClutAlloc_makepacket` `0x00917050`): 72 transfers
  of `0x460` bytes (A+D `BITBLTBUF`/`TRXPOS`/`TRXREG`/`TRXDIR` left 0 by
  the PC build, an `IMAGE` tag, 256 colours), double-buffered at
  `+0x13b14`.
- `KeVu0RndI` (`0x0075f920`): two VU0 random steps through the emulated
  EE/VU0 registers (`0x00c8f5xx`, `g_Vu0_R`), each `srand`ing the CRT.
- `NkzSetLuluLimitTbl` (`0x00a5cc70`): one of 17 Lulu digit tables
  (`0x14` bytes from `0x00b7b7f4`) by battle group and her magic.
- `0x00441010` is `Phyre::PCluster::create(count, descriptor, flags)`, one
  body (not folded); `0x0064b7a0`, `0x0064b7e0`, `0x0064b800` are its
  typed wrappers for `PMaterialSet`, `PTexture2D` and `PWorldMatrix`
  (descriptors `0x00c998e8`, `0x00ca37d0`, `0x00c92090`), PhyreEngine
  code; Fahrenheit's Switch BSim names for them (`yiGetRootMatrix`,
  `yiSetRootMatrix`, `SqsEeSoundCreateTime`) are wrong matches.

## Sprite packets, sphere grid helpers, map file records (round 6)

- `KeDttSpr_MkGifPk_Pre` (`0x00768d30`): the sprite pre-packet, a chain
  of VU0 float products kept on the x87 stack (each step rounded to 53
  bits, stored as `f32` only where the original stores), the constants
  at `0x00b51b9c`, `0x00b921e0`, `0x00b50224`, `0x00b515c8`; its packet
  space from `op_spr_alloc(0x90)`.
- Sphere grid: node visitors `AbmapForEachNode` (`0x00a59710`, over the
  `+2` node count, kind -1 skipped) and `AbmapForNodeAndLinked`
  (`0x00a59760`: the node, then the other end of each link, two `u16`s at
  the start of the `0x14`-byte links from `+0xa808`, link count `+4`),
  both stopping at the first non-zero result; node and link matrices
  (`0x00a5ad30`, `0x00a5a360`) built from the emulated zero register
  (`g_Ee_Gpr`) with the scale on the diagonal and the `i16` position as
  translation; character markers (`0x50` bytes from `+0x11088`, node
  `+0x44`); the cursor pulse `+0x115c5` from the sine table
  (`AbmapPulseCursor` `0x00a47440`, truncating `fistp`); the help state
  (`AbmapResetHelp`); `AbmapVu0LoadMatrix1`/`2` load 64 bytes into the
  emulated `vf01`..`vf04` / `vf05`..`vf08` (`0x00c8f7a0`, `0x00c8f7e0`).
- Map file records (reader `0x0090f900`, magic `0x214365`, version
  `0x110`): object records of `0x40` bytes (`yiObjSave`/`yiObjLoad`,
  object `+0x80..+0x96` and the floats `+0x50..+0x6c`), camera records
  (`yiCameraLoad`), the fog record (`yiFogLoad`: `+0x68340..+0x6834c`,
  `+0xba400`, colour `+0x68378`, `+0x6822c`), the matrix chunk
  (`yiLoadMatrixChunk`). The fog parameters for VU1
  (`yiRender_setupFogParam` `0x0090d1b0`): `+0x68350 = -255 / (far -
  near)` (0 when equal), `+0x68354 = 255 - +0x68344` (255 with
  `+0x67f08` bit 3), `+0x68358 = -near`.
- `rcfx` initialisers: kinds `0x30002` gravity, `0x30004` nail, `0x30006`
  wind, each with its callbacks; `rcfxParticleInit` (0.03, 0.001, 0.1).
- `rcefAnimeStep` (`0x0092f400`): time `+0x40` += speed `+0x44`; past
  the key's length (`+0x12` of the 8-byte keys at `+4`) to the next key;
  past the last (table `+6`) loops with key flag 0x80, else stops (bit 0
  of `+0` cleared).
- `rcuiClearSelect`/`rcuiSetSelect` (`0x00934af0`, `0x00934b50`): the
  layer selection bits (`+0xb0` bit 0 of the selected layer in mode 1,
  bit 1 of each layer's first word in mode 2).
- Functions that do not return a value often leave something in `eax`
  that a caller could read; the ports return the same (`check-clobber`
  checks the callers). `yiRender_setupFogParam` and `KeAddScaledColor`
  end with `fnstsw ax` over an untouched upper half (the caller's `eax`),
  so they are `void`.
- Skipped: `0x00915bb0` stores uninitialised stack words (`local_24`
  etc.), so its output is not deterministic; `0x00919a60` is a guard
  jumping into `rcbgFini`; `0x0083bdb0` (a free-block helper beside the
  `Sg_`/`Ch_` code, writing `0x01301968`) is left to its module.

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
