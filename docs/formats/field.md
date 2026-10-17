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

## Free camera, map teardown, windows (round 7)

- Free camera (`FreeCamera`, the PC build's fly camera, `0x344` bytes,
  singleton `g_FreeCamera` `0x00ccc830`): two `Phyre::PCameraPerspective`
  at `+0xc` and `+0x164` (their buffers at `+0xdc`, `+0x234`; bit 0 marks a
  buffer not owned), a `PWorldMatrixFlyController` at `+0x2bc`, speed
  `+8` (60.0), flags `+0x338`, `+0x339`, `+0x33a`, `+0x33c`. The
  constructor (`0x0067fe70`) and destructor (`0x0067ff10`) link an
  exception frame at `fs:[0]`; the ports run the Phyre constructors and
  destructors (library code) through callbacks.
  `FreeCamera_destroyInstance` (`0x00680000`) is called by
  `graphicFinalize`.
- Map teardown (`Yn_MapSystemFini` `0x00919a60` → `rcbgFini`
  `0x00921aa0`): the guide's minimap VFX (`graphicVFXDestroy(0, model
  +0x14)`), `rcbgPreFini` (each scene, `0x110` bytes, `rcbgSceneFini`;
  the scene array, `+0xc` and the map freed; `MsSetCameraMatrix(0, 0, 0,
  0, 0, 1024.0)`), `sceDevVif0Reset` in debug builds, the guide
  (`rcbgGuideDelete`: its VFX instance, handle low word `0x0230fd20`,
  high word the guide; the walk model), the chunks at `+0x10`, `+0x14`,
  `+0x18`, then the system. Scenes (`rcbgSceneFini`): layers (`0xc0`
  bytes: transfers `+0xa0`, `+0xbc`, animation block `+0x90`), objects
  (destructor at `+4` of each), `+0x14`, models (`+0x1c`, `i16` count
  `+0x18`). Everything is freed through `rcFree` (`0x009258e0`, a jump
  to `user_free`).
- Window handlers (`g_YiWindowHandlers` `0x00c5d788`, 6 slots a kind:
  `+0x60` init, `+0x64` open, `+0x68` after opening, `+0x6c` close,
  `+0x70` after closing, `+0x78` draw; the current one at `+0x74`, the
  idle handler `0x00907e50`). Window fields: `+0` active, `+6` progress
  (1/4096), `+8` state (2 open, 4 closed), `+0xc`/`+0xe` frames and
  reload, `+0x10` height, `+0x18..+0x24` the rectangle animated from,
  `+0x28..+0x34` the rectangle (x0, y0, x1, y1), `+0x38`/`+0x3c` steps,
  `+0x48..+0x54` the rectangle drawn, `+0x58` colour. Kinds: 0 grows from
  the left edge (`+0x2aa` progress a frame), 1 and 2 pop, 3 slides down,
  4 and 5 up, 6 and 7 right, 8 and 9 left (6 frames over a 512x416 area;
  the "Here" variants start from the window's own rectangle), 10 is a
  bar `(0, 8, 512, 28)`. The draw handlers call the PS2 2D drawing and
  stay original.
- `rcCopyQword_*`: thirteen copies of `(dst, src)` moving a quadword
  through the emulated `vf31` (`0x00c8f980`) with `VFX_Copy16From` and
  `VFX_Copy16To`.
- Testing: the Phyre perspective calls `_CItan` (patched to `fptan`);
  `operator delete`, `rcFree`, the VFX calls, `MsSetCameraMatrix` and
  `sceDevVif0Reset` are replaced by logging stubs in both oracles (the
  CRT heap needs critical sections the oracle does not stub yet), so the
  tests compare the calls and their order.
- Not ported: the map file reader `0x0090f900` (it calls the
  `sceGs*`/`FlushCache` stubs, `Sg_MFIFOSync` and graphics objects, and
  `0x0090ff40`/`0x0090e5d0`, not ported yet); the window draw handlers;
  `0x006e9250` (a UI movie loader registering `getLocByStr` etc.).

## Map file reader, background loading, window drawing (round 8)

- Map render file (`yiMapFileRead` `0x0090f900`, through `yiArc_readmem`
  `0x0090f8e0`, which checks version `0x110`): magic `0x214365`, version
  `0x110`, the record count at `+0xc` (decremented as records are read),
  records from `+0x40`, each a `0x40`-byte header (`+0` kind, `+4` the
  data length) and its data:
  - 0 an object (`0xb4` bytes at `+0x6878c`, count `+0x68788`;
    `yiObjInit`, `yiObjLoad`, `graphicObjectSetTranslation` and
    `SetRotation`), the current object for the cameras that follow;
  - 1 a camera key list (`0x5c` bytes at `+0x6e190`, count `+0x6e18c`),
    linked to the previous camera (`+0x4c` next, `+0x50` previous; the
    first goes to the object's `+0xa8`), its keys after a further `0x40`
    bytes; a key count (`+0x48`) that disagrees with the length ends the
    file;
  - 2 a texture (`0x24`-byte entries from `+0x9c19c`, count `+0x9c198`):
    uploaded with `sceGsSetDefLoadImage`/`sceGsExecLoadImage` (format
    0x14: halved width and quartered height, PSM 0), or kept in the
    scene's VRAM copy from `+0x276a4` when its GS address is `0x3c00` or
    above (`yiLoadVramImage`; format 0x13 is swizzled by `0x0090e0b0`);
  - 3 the CLUTs (kinds 2..=4 at `+0x24`; `yiClutAlloc_makepacket`);
  - 4 the fog (`yiFogLoad`; the matrix `+0x68078` turned by the fog
    angles `+0xba400`/`+0xba404`, degrees);
  - 5 and 8 packet lists (`+0x9d4c4`, `+0x9d5c4`, sizes at `+0x9d6c4`,
    count `+0x9d4c0`; by index `+0x24` when `+0x20` is set, else
    appended with the length / 128 or / 32);
  - 6 animation tracks (`0x720`-byte sets from `+0x9d9d0`);
  - 7 the matrix chunk (`yiLoadMatrixChunk`).
  The original leaves the current object and camera uninitialised, so
  files start with an object. Its load-image packet is a 16-byte aligned
  stack buffer; the port takes it as a parameter.
- Background files (`rcbgLoadBin` `0x00921d60`, from `Yn_PreBgSetData`):
  a `YNDT` chunk, then chunks with a 16-byte header (the tag from the
  `rcbgChkId` table, `+4` the length in 16-byte units, `+0xa` a scene
  index): `YNPR` the map (`rcbgPreInit`, `rcbgPreLoad`), `YNSC` the next
  scene (`rcbgSceneLoad`: layers, colours, `rcbgSceneChkLayers` /
  `rcbgSceneChkSortLayer`, `rcbgSceneSetMatrix`), `YNSM`/`YNST` a model or
  transfer of that scene, `YNTT`/`YNFT` texture transfers of the current
  scene, `YNGM` a model replacing the last one of scene `+0xa`, `YNED`
  the end. Then `rcbgPreSetSceneNo(0)`.
- GS memory for image layers (`rcbgSceneChkSortLayer`): each layer with
  flag 0x10 gets its image page at `+0x34` (64-word units, the 128x64
  rounded size; past `0x68000` words the images move to page `0x2f00`
  and count a quarter), then its CLUT page at `+0x36` (`0x100` words each
  from `0xbc000`); the transfers' `BITBLTBUF` words follow, and pending
  transfers are drawn. Image layers always have their CLUT transfer at
  `+0xa0`.
- The `rc` allocator (`rcAlloc`/`rcCalloc`/`rcRealloc`, `yiMalloc`,
  `yiMalloc64`) sits on `user_malloc`; `rcRealloc` keeps a block of the
  same size (`Sg_MemGetSize`) and copies the smaller size otherwise. The
  `*New` constructors allocate and run the matching `*Init`.
- Map (re)set-up: `yiResetObj` (teardown, a new `rcbgInit`,
  `yiRender_resetobj`, graphics unloads), `yiObjReadCallback` (a `MAP1`
  file's background `+0x10`, render data `+0x14`, alarm, `ppp` data
  `+0x38`, two more backgrounds `+0x40`/`+0x3c`; then the draw mask and
  the default animations), `yiObjReadFinish`. Animation slots:
  `yiAnimInfo_init`, `yiAnimDetach`, `yiKeyFrame_attachDObj` (keyframe
  records from `+0x9d4c4`, kind 5 for packet lists), `yiAnimAttachDefault`.
- Window draw handlers (slot `+0x78`): plates (`0x008f9a40`) and menu
  plates (`TODrawMenuPlateXYWHType`) with their scissors; the pop window
  of map 0x15b clips to the `TO` window at (38, 115); the help bar draws
  two sprites and a text shape from the layout at `0x025d1618`
  (`0x00907c10` is not a function start in the dump, so its port has no
  hook).
- Testing: the allocator is a bump allocator in both oracles (its size
  below each block for `Sg_MemGetSize`); frees and the engine calls are
  logging stand-ins in a code cave (the body of `0x00764de0`). The CRT
  stubs (critical sections) are not on main yet, so the round 7 teardown
  tests still log `rcFree` instead of using the real allocator.
- Not ported (ported in round 9 unless listed below): `rcbgSceneSetMatrix`
  (projection, `_CIatan`, camera calls on stack matrices),
  `rcbgGuideSetModelFromIdMap`, `yiMapReadMaster` (CD function table,
  `fprintf`), `0x00909680` (host file loading), the jump
  thunks `yiFree` (`0x0090bbc0`), `Yn_RcbgInit` (`0x00919a70`) and the two
  `Yn_PreBgSetData` jumps (`0x00919b90`, `0x00919d30`), which are not
  functions of their own in the dump.

## Map camera, ID-map guide model, swizzle, map reads (round 9)

- Camera block (`rcbgSceneSetMatrix` `0x00920c00`, `crates/ffx-field`
  `camera`): `+0x64` a y offset (`i32`), `+0x68` eye, `+0x78` the point
  looked at, `+0x88` up, `+0xa8` the field of view in degrees (0 leaves
  everything alone; NaN does not), `+0xac` roll, `+0xb0`/`+0xb4` the x
  scale and it times 7/6, `+0xb8`/`+0xbc` the screen-centre offsets. In
  order: the player's clip distance to 1000 (`Ch_SetClipZ`), the
  projection (`rcbgSetProjectionMatrixSub` `0x009202e0`:
  `t = tan(fov / 2)` through `_CItan`, x scale `512 / (2 d t)` with `d`
  the screen depth `0x0113701c`, `FFXSamp0ViewScreenMatrix` with the
  centre `0x01137020`/`24` moved by the offsets and the near/far planes
  `0x01137008`/`0c`, times 10 and 2 * 65536 when the scene's `+0xfc` is
  set; `256 / t` back), the battle screen offset `0x01136fb8`, the clip
  matrix (`512 / sx`, `416 / sy`), the view (`rcbgSetViewMatrix`
  `0x009204b0`: eye and target scaled by the scene's `+0xf4`, then
  `MsCameraMatrix`), both to `MsSetCameraMatrix` (roll argument
  `256 / t + 256`), and the renderer's field of view
  `2 atan(448 / (2 sy) / d)` (`_CIatan`), aspect `(512 / 2 sx) / (448 /
  2 sy)` and `g_CameraMatrix`. The stack matrices and vectors are a
  `work` parameter in the port; `_CItan`/`_CIatan` are called through
  the game's import thunks (`ST0` in and out).
- The scale-vector routine (`VFX_ScaleVec3` through vf31) exists as
  fourteen identical copies (`0x009200a0`, `0x00927b00`, twelve more),
  plus two that scale all four elements (`pppVu0MulX_007500d0`) and two
  that convert to 12.4 fixed point (`rcVec4fToFixed`). Each copies its
  16-byte argument area to `0x00c8f970`: the scale, then three words of
  its caller's stack, which the port does not write.
- `rcbgModelCreate` (`0x00927be0`) builds a primitive blob from a
  description (`n`, `{type, count, flags}` triples, vertex and normal
  counts): a 0x20 header (`+4` 0x20, `+8` the vertex offset, `+0xc` the
  normal offset, `+0x10` the primitive count, `+0x12`/`+0x14` the vertex
  and normal counts, `+0x16` 0x44), packet headers (`+1` type, `+2`
  count, `+4` flags) each followed by `count` records of the type's
  stride (`0x00c5e790`: 20, 28, 32, 40, 24, 32, 40, 48), an end mark
  `0xffff`; every part rounded to 16 bytes.
- `rcbgGuideSetModelFromIdMap` (`0x0091d5e0`): the walk mesh (`ID map`,
  `0x01301a94`): `+6` groups, `+0xa` vertices (`i16` x and z at 8-byte
  steps from `+0x18`), `+0xc` the scale (vertices are divided by it),
  `+0x1c` the groups (0x10 bytes: `+8` the triangle count, `+0xc` the
  triangles, 0x10 bytes with three `u16` vertex indices). More than
  0x1ff triangles build nothing ("ID MAP no POLYGON oosugi desu"). The
  model is one type-0 packet, white triangles, vertices packed to `i16`
  by the bounding box's scale (`rcbgModelFToS0`).
- Format-0x13 images (`yiSwizzleImage` `0x0090e0b0`, from
  `yiLoadVramImage`): powers of two up to 1024 each way, cut into
  128x64 tiles; each tile's lines are gathered at 0x80 bytes,
  `yiSwizzlePage` (`0x0090e2e0`) places its 16x16 blocks by the tables at
  `0x00c5db40` (reading order) and `0x00c5db60` (the GS block layout,
  four rows of eight) and `yiSwizzleColumns` (`0x0090e000`) reorders each
  block's bytes by `0x00c5da80`; the result goes out in rows of `2 w`
  bytes (at most 0x100), `h / 2` of them (at most 0x20).
- `yiMapReadMaster` (`0x009093b0`, `Sg_MapInit`): after the renderer
  reset, the map ids `0x01934edc` and, with a CD file, `GetCD`'s table
  (`+0x0` read, `+0x8` finish, `+0x10` select a map, `+0x14` a file's
  size) reads a `MAP1` file into a `yiMalloc64` buffer (`g_YiMapData`):
  section 0 the 2D map (`graphicFieldMapLoad(name, 0)`,
  `Yn_PreBgSetData`, status bit 0x4000), 1 the 3D map (`(name, 1)`,
  `yiArc_readmem`), `alarm + 2` the alarms, 10 effect data
  (`pppSetFpPdt`), 12 and 11 more backgrounds; the CD ids go to
  `0x01934ed4`. Then status `| 0x11`, the draw mask (-1 with the battle
  map circle, else -9) and the default animations. `yiMapSetup`
  (`0x00909680`) is the debug path: `host0:<dir><name>.vpa` read whole
  through `sceOpen`/`sceRead`.
- Smaller pieces: `rcbgSceneDelObject`, `Yn_SceneFindObject` (the
  current scene, then each scene), `rcbgPreAddFp`/`rcbgPreGetMapNo` (the
  field programs of maps 0x61, 0xad, 0x235/0x236 and the battles that
  use them), `rcfx` particle forces (sorted by `+0x18`), spring, nail
  and wind set-up, palette animations (rotate, shift, white into the
  last entry), transfer entries and their GS upload packet, renderer
  resets (`+0x27624` packet, `+0xbe07c` fog), `yiRotMatrixSeq` (orders
  XYZ, YXZ, ZXY, XZY, YZX, ZYX), parser helpers, effect task kill.
- Testing: the trigonometric intrinsics are the CPU's (`fptan`, `fpatan`,
  `fcos`, `fsin`) in both oracles; `MsCameraMatrix` stores four of its
  own uninitialised locals over vf registers (`0x00c8f7d8`, `0x00c8f7e8`),
  so those bytes, like the scale routine's stack copy, are taken from the
  original's side before comparing. On snapshots the game's own heap runs
  (`user_malloc`/`user_free`, with the CRT's critical sections stubbed),
  so objects are built and torn down with the real allocator; on the image
  alone the heap needs Win32 events and the bump allocator stays.
- `yiWindowDrawBar` (`0x00907c10`) is a function start in the rebuilt
  dump and has its hook from round 10.

## Field helpers by unblocked callers (round 10)

- Parser readers: `rcdtParseGetKeyword` (`0x0092be40`, the token's index
  in a null-ended string table, same length and `_strnicmp`, else -1),
  `rcdtParseGetFloat` (`0x0092bbb0`, `atof` in text mode, `ST0`),
  `rcdtParseGetText` (`0x0092bdd0`, `strncpy` of the token, its length),
  `rcdtParseAdvanceLine` (`0x0092b9b0`).
- Field programs: `rcbgFPmGetMatrix` (`0x00922290`: rotation `+0x18` times
  the `.rdata` factor `0x00b678ec`, scale `+0x28`, translation `+0x8`,
  `op_rt_m_yxz`), `rcbgFPmGetElemMatrix` (`0x00922350`, a character
  element's matrix with its rows times 10), `rcbgFPmRestart` (`0x00922300`,
  the manager's flag 0x100, `Fp_Restart`, the element matrix at
  `mng + 0xc`), `rcbgPreAddFpObjects` (`0x00922010`).
- `rcbgPSimAddDamp`/`AddWind`/`AddGravity` add forces to the simulation's
  particle system (`+0x18`); the gravity one overwrites its vector with the
  debug gravity (0, 1, -0.1). `rcbgPalProc` (`0x00929b60`) picks the
  palette animation: flag bit 0 white, mode `+0xa` 1 rotate, 2 shift,
  else a copy to the last entry.
- Effect task lists: `rcefTaskCtrlFini` kills the tail's previous task
  until the head is reached; `rcefTaskCtrlProc` runs a `+0x8` chain, then
  kills the tasks flagged 4 (each task's next read before it is killed).
- `rcbgModelSetTexReg` (`0x009284d0`) builds a register record
  (`page << 17 | 0x1000f`, register 0x47) for `rcbgSetReg`, a bare `ret`
  here, so its `eax` is the record's stack address.
- `yiFieldMapRead`/`yiBattleMapRead` read CD directory 0x13/0x1a files
  `2 map` and `2 map + 1` (`yiMapReadMaster`), status bits 0x2000/0x1000.

## Two-operand intrinsics, angles, views and lights (round 11)

- `_CIfmod` takes its operands in `ST1` (the dividend) and `ST0` (the
  divisor) and leaves `fmod` alone in `ST0`; the float-call path has a
  two-operand form for it (`FArgs::St01`; the hook loads both with
  `fld qword` before calling the import thunk `0x009498e0`).
  `rcefAngleWrap` (`0x0092d810`) brings an angle into `0..=2π`: negative
  ones as `2π - fmod(-a, 2π)`, ones past 2π as `fmod(a, 2π)`, others (NaN
  too) unchanged.
- `rcefVecToAngle` (`0x0092ec20`): `acos(-v.z)`, mirrored to `2π - a` for
  `v.x > 0`; `rcefAngleToDir` (`0x0092ded0`): `(-sin a, 0, -cos a, 0)`.
- Renderer views: `yiSetBattleView` (`0x0090a140`, skipped with status bit
  0x800) and `yiSetViewMatrices` (`0x0090a640`) multiply the battle view
  by `g_YiViewAdjust` into the three view matrices `0x018ded78`,
  `0x018dedb8`, `0x018dedf8` and keep the screen depth at `0x018df160`;
  `yiSetLightMatrices` (`0x00908d00`) builds the normal light matrix
  (`0x018deeb8`, three normalised directions) and the light colour matrix
  (`0x018deef8`, four colours).
- `yiImageVramAlloc` (`0x00917d80`): image GS memory by format; format
  0x13 below page 0x2e00 becomes 0x1b, format 0x14 becomes 0x2c and is
  cached at `0x025d17a4` for the next 0x14 image (format 0x24).
- Keyword configs: palettes (`0x0093adf0`, table `0x00c60e94`: mode
  `+0xa`, speed `+0xb`) and text sub-configs (`0x0093aee0`, table
  `0x00c60e44`: `+0x5c`, `+0x60`, `+0x64`).
- Testing: text parsers need a bounded chunk. With `rcdtChunkSetBin`'s
  unbounded size, `rcdtParseSkipSeparators` keeps scanning through memory
  after trailing separators, because NUL counts as a separator. `_CIacos`
  is the x87's `atan2(sqrt(1 - x²), x)` in both oracles, and `_CIfmod` is
  the oracle's `fprem` stub.

## Random vectors, the pmcom environment and the renderer's bounds (round 12)

- `KeVecAddRandom` (`0x00766f10`) adds `op_random(range[i])` to x, y, z
  (times `op_random_p(1.0)` when its third argument is set, the first
  draw stored as a `double`); `KeVecScaleRandom` (`0x00766f70`) adds
  `v[i] * s - v[i]` for `s = r + r` or `r + r'` (`op_random(1.0)` draws,
  `s` stored as `f32`). Both read the draws from `ST0`, so the port calls
  `op_random` through the float path (`FArgs::Stack`) and keeps the
  original's rounding points.
- The pmcom environment: `pppInitPmcomEnv` (`0x00760610`) resets the
  arena pointers and calls `pppInitEnv(0x010d4cfc, 0, 0x00fd24f0,
  0x100000)`; `pppFreeAllEnvObjects` (`0x007605d0`) frees the object
  blocks at `0x00d4e390` (0x80 bytes, count `0x00d4b4cc`) in it.
  `KeResampleRgbaToRgb` (`0x00760640`) scales an RGBA image to BGR
  bytes, filling the destination bottom row first; its last row lands
  one row before the buffer, one pixel to the right.
- `KeThBP_Init` (`0x00763030`, the Switch build's name: it calls
  `set_ptr32` seven times, as this does) sets up a `KeThBP` object with
  its GIF tag at `+0x40 + 0x30n`.
- Sphere grid: `AbmapResetCursorCamera` (`0x00a5b030`) puts the cursor
  256 units from the current character (`+0x115bc`), 32 below it, then
  pulses it; `AbmapStoreSave` (`0x00a5bb70`) copies node bytes, link
  flags and the characters' `u16`s into `MsGetSaveAbilityMap`'s block;
  `AbmapScaledLevelValue` (`0x00a5cbe0`) is `ftol(T[i] * (level + 50) *
  T[j])` for `T` = 1, 1, 1.5, 3, 5 (100000 for `j` 4).
- `rcbgSceneLoadTextCameraCalcRef` (`0x0093b110`, the Switch name in its
  sorted place before `rcbgSceneLoadTextModel`): the reference point
  `+0x78 = +0x68 + +0xf4 * row 2` of the rotation built from the degrees
  at `+0x98`.. and the direction `+0x88 = -row 1`. It returns
  `FFXVu0RotMatrixZ`'s `eax`, the address of its stack matrix; the test
  leaves that out.
- Renderer: `yiRender_addDirectBlock` (`0x0090c120`) aligns the data
  pointer to 64 bytes and adds a ref tag to a VIF FLUSH, STMOD, STCYCL,
  DIRECTHL block; `yiTriContainsXZ` (`0x00910270`) tests a point against
  a triangle on x, z (two edge products stored as `f32`, their sum kept
  at 53 bits); `yiBoundsMinMax` (`0x009130f0`) starts from ±FLT_MAX.
  `yiImage_dumpall` (`0x00918680`) only prints, through `dbgPrintf` (a
  bare `ret`).

## Float results, waves, particle kicks and image VRAM (round 13)

- The field oracle's `Orig::eax`/`Orig::void` now fail when the original
  leaves a value in `ST0`: a port returning an integer or nothing would
  drop it (a void `Ch_CheckRadian` made every model invisible). All
  earlier field tests pass under it, and no map shim returns non-float
  for a function its callers read as `(float10)` in the dump.
- `rcbgSceneWaveDCProc` (`0x00920580`) runs the wave `Yn_SceneSetDC`
  starts: `+0xac = +0xe8 * sin`, the sine from `g_pppSinTable` at
  `ftol(+0xec * k / 2pi) >> 4` (`k` the double at `0x00b51558`), faded by
  `(+0xe4 - +0xf0) / +0xe4` for kind 2; `+0xec += +0xe0`; it stops
  (`rcbgSceneWaveDCStop`) when the counter reaches `+0xe4`.
- `rcefMainKickPar` (`0x009253a0`, the Switch name: same callees, called
  by `rcuiParProcKick`) starts a particle effect on a main: a new object
  when its task ended, else the current one unless its particles still
  run (and `force` is 0).
- `rcbgPSimDelete`/`rcbgPSimFini` (`0x0091fd40`/`0x0091fd80`) take the
  Switch names from the sorted `rcbgPSim` run of the Steam build.
- `yiKeyFrameWrapAngles` (`0x00915120`) wraps four angles into `-pi..pi`
  by `(1 - ftol(x / -2pi)) * 2pi`.
- Image VRAM: `yiVramSetup` (`0x00917f20`) sets the area block
  `0x025d1798..d4` for modes 0..2 and the limit `(0x38 - pages) << 14`;
  `yiImageRectFree` (`0x00917490`) tests a rectangle against the page and
  the other images (a zero power-of-two size divides by zero in the
  original); `yiImageTakeLastPage` (`0x00917e70`) places a PSMT8/PSMT4
  image in the last area as 0x1b/0x2c, a second PSMT4 sharing it as
  0x24.
- `rcuiLoadSetTitle` (`0x00934fb0`) titles the load window
  "Load - <map directory>".

## Character direction, layer animation, key frames and image placement (round 14)

- `rcefGetChrDir` (`0x0092dfd0`, the Switch name) returns a float in
  `ST0`: `MsGetChrDir` into a local, then `rcefAngleWrap`. Ghidra types
  it `float10`, and its shim returns `f32`.
- `rcbgLayerAnimStep` (`0x00930920`): a layer animation adds its speed
  (`+0x14`) to a sub-frame counter (`+0x18`) and moves whole 0x100 steps
  into the tick (`+4`); while the tick passes the current record's
  length (its byte `+1`) the frame (`+6`) steps forward (backward with
  bit 1), wrapping (mode 0) or holding (0x100, bits 8..11). It returns
  the record it started from, not the new one.
- `yiKeyFrameAdvance` (`0x00914180`): a key frame track's start
  (`+0x14`) follows the clock; frame records are 32 bytes at
  `r +0x9d5c4[track +8]`, each lasting twice its `+4`; a one-shot track
  (`+2` set) clears itself at the end and counts down `r +0x9cabc`.
- `yiImageRman_serch` (`0x00917b70`) looks for a place for an image:
  right of, below, left of and above each image already in the page,
  then the page's bottom-left and bottom-right corners, each checked
  with `yiImageRectFree`.
- `rcuiCollectChrHandlesByGroup` (`0x009323b0`) lists characters 0x14..0x1b
  then 0..0xe with their handles; `rcuiVu0AddVector` (`0x00937540`) adds
  through the VU0 registers at `0x00c8f980`/`0x00c8f970`.

## Hermite curves, linked key frames and the lens shapes' render data (round 15)

- `yiHermiteVec2`/`yiHermiteVec4` (`0x00914700`/`0x009147b0`): cubic
  Hermite interpolation of two or four floats. The weights
  `(t-1)^2 (2t+1)`, `t^2 (3-2t)`, `(1-t)^2 t` and `t^2 (t-1)` are each
  stored as `f32` first. Ghidra's `fVar3 * t * t` is really `t^2 * (t-1)`
  (`t^2` is computed once and reused), so the products follow the
  disassembly, not the decompilation.
- `yiKeyFrameAdvanceLinked` (`0x00914070`): the `yiKeyFrameAdvance` of
  tracks whose 0x80-byte records chain from `+0x24`; while paused it
  returns `now`, not the time difference its sibling returns.
- `yiRectTriOverlapXZ` (`0x00910070`): an x/z rectangle against a
  triangle (transformed, or twelve unsigned integers as floats), both
  ways, through `yiTriContainsXZ`. It writes 1.0 into the input
  vectors' w before transforming them.
- `0x00920100` is not ported: it ends in `FFXVu0FTOI4Vector`, a bare
  `ret` on PC, so it returns the z of an uninitialised local divided by
  0x3000.
- The lens shapes' render data: `KeLnsArnd_Draw` (`0x00769340`),
  `KeLnsCrn_Draw` (`0x00769d70`) and `KeLnsFls_Draw` (`0x0076a1f0`) build
  a matrix from the lens source (`+0x7c` alpha, `+0x8f` own matrix,
  `+0xa4` intensity, `+0x80` position plus the VFX offset at
  `0x0230ff50`), compute the colour's alpha (`fistp` under
  round-toward-zero), step the shape's clock (`KeLnsShp_Update`), stamp
  the program byte into the shape (`+0xb`) unless an ordering-table pass
  draws (`0x00cec1a4`), then hand the description (flags `+2`, colour
  `+4`, matrix pointer `+0x10`) to the PC renderer:
  `updateVFXRenderDataShp` and `graphicVFXUpdate` when render data
  exists for the instance key (program byte, instance), else
  `InitVFXRenderDataShp` with `calculateVFXSP_AlphaFunction` of the
  frame's mode `| 0x800` (`| 0x40` for flag bit 3). `KeLnsFls_Draw`
  looks the render data up first and hides it (`graphicVFXSetVisible`)
  when the flash cannot draw. Bytes 0..2 and 0x14..0x1c of the
  description are never written: the renderer gets the original's
  stack there, and the port's zeros.
- `KeMdl_Draw` (`0x00766ff0`) also feeds the renderer; it is ported in
  round 16.

### Replaced by ffx-render

Pure GS/DMA packet writing, left to the original until the external
renderer takes over:

- `0x00760ce0`: a GIF image transfer (`sceGsSyncPath`, `sceDmaSync`,
  `sceDmaSend` from two alternating buffers).
- `rcPacketDataDraw` (`0x00927600`): `sceDmaSend` of a packet.
- `0x0093c730`: a bar of `Gu_SetXyzf2` vertices (the rcui Z value
  window).
- `0x00927630`, `0x00927720`, `0x00927810`, `0x009278d0`: `rc` packet
  builders (rectangles, circles, lines, projected points as fixed-point
  GIF vertices; their `ftol2_sse` operands are x87 values the
  decompilation hides).

## The VFX model's render data, rcui helpers (round 16)

- `KeMdl_Draw` (`0x00766ff0`): the colour is the compiler's emulation of
  R5900 multimedia instructions on the EE register words at `0x00c8f508`
  (A), `0x00c8f588` (M) and `0x00c8f598` (B). The two words at
  `col + g_pppColorConShift` and the current part's `+0x70` words are
  interleaved halfword-wise with A into M and B, and multiplied
  (`pppEePmulth`). The products are shifted down 19 bits (13 kept),
  interleaved with A's low halves and packed to bytes (`pppEePpacb`). Every
  dword and halfword write to those globals is kept in the original's
  order, and `g_pppColorConShift` is cleared twice. A colour whose top
  byte is 0 draws nothing.
- Its description (0x48 bytes) is built from:
  - flags from the mode argument and the record's shape byte (0x4/0xc
    path 3, 0x80 additive, 0x8, 0x10, 0x100, 0x200, 0x2000 with the view
    matrix `g_pppViewMatrix * e +0x10`);
  - the GIF tag word `0x00c498dc`, the record's alpha byte and the colour;
  - `e +0x50`/`+0x10`/`+0x90`/`+0x94`, the model data blocks
    `0x0230fd60`/`0x0230fda0` and the packet position;
  - the Z mode argument when mode bit 5 is set.
- It goes to `updateVFXRenderData` with `graphicVFX(Path3)Update`, or to
  `InitVFX(Path3)RenderData` with the alpha function, Z mode (unless the
  matrix is set) `| 0x40 | 0x80`. Then `graphicVFXSetClutParameters`
  takes the three words at `0x010d5600`. Bytes `+0xb` and (without the
  matrix) `+0x30` are never written.
- `rcuiProjectToInt` (`0x0093d710`) leaves `1/w` on the x87 stack at
  return (both paths pop everything else), so its port returns that
  float.
- `rcuiInvertRigidMatrix` (`0x0093d090`): the transpose with the
  translation `-(R^T t)`; `rcuiClampVector` (`0x0093cd20`) keeps NaN.
- `rcbgAnimeLoadConfig` (`0x0093a650`): `DataCnt`, then that many
  four-byte frames.
- `0x0093c730` draws a bar through `Gu_SetXyzf2`/`Gu_SetRgbaq` packets:
  replaced by ffx-render.

## Round 17

- `FFXSamp0TransMatrix` (`0x0093d830`, ten callers): a matrix copy with
  its translation moved by a vector; `rcuiScaleVec4` (`0x0093d7d0`);
  `abmap_snd_chg_panel0/1` (`0x00a45910`/`0x00a45920`): sounds
  0x8000006d/0x8000006e through `SndSepPlaySimple`.
- `yiSegIntersect2` (`0x0091cd00`): the crossing of two 2D segments.
  The first cross product is kept as an `f64` (`fst qword`), the
  differences and quotients are stored as `f32`, and the epsilons
  (±1e-5) come from `.rdata`.

## Debug (mod-tool candidates)

Debug code is not ported (the user's rule from round 17). These
functions stay with the original and could become mod tools. Those
marked "ported earlier" were done before the rule; their ports are
behaviour-exact and stay.

Tweak overlays and allocator statistics:
- `BasicDebugTweakUtil_initialize`/`_render`/`_setEnabled` (`0x006b9360`,
  `0x006ba1a0`, `0x006baf60`) and `0x006ba110`: the basic debug overlay
  (character and map names, fps).
- `BtlChrParamConfigTweakUitl_initialize` (`0x006b94a0`) and `_render`:
  battle character parameter editor (invincibility, ...).
- `BtlDebugTweakUtil_initialize` (`0x006b9600`) and `_render`: battle
  debug information.
- `CharacterAnimationTweakUtil_initialize` (`0x006b97f0`),
  `CharacterTextureAnimationTweakUtil_initialize` (`0x006b9aa0`),
  `0x006b9970`, `0x006b9e20` and their `_render`s: animation and texture
  animation browsers.
- `GameParamConfigTweakUitl_initialize` (`0x006b9cd0`): game parameter
  cheats (full items, full commands).
- `CharacterSwitchTweakUtil_render`, `MapSwitchTweakUtil_render`: the
  character and map switchers' overlays.
- `mspace_malloc_stats` (`0x006ea450`): allocator statistics print.

Prints and dumps:
- `0x00760830`: VFX clip/draw counters print (`CL%d:%d DR%d:%d`).
- `prcPrintTex0` (`0x00766e60`): prints a GS TEX0 register.
- `yiAssert` (`0x0090ae50`): the renderer's assertion print.
- `yiDumpMatrix` (`0x0090b4b0`): prints a matrix.
- `yiArc_GSmemwriteV110` (`0x0090ec94`): writes GS memory to a file.
- `yiImage_dumpall` (`0x00918680`, ported earlier): prints every page's
  images (a no-op on PC).
- `yiImage_printusemem` (`0x00918ba0`): GS memory use print.
- `rcPrint` (`0x00927550`): the `rc` libraries' print.
- `rcbgModelDump` (`0x00928180`): prints a model's primitives.
- `RopeDebug2` (`0x00930e70`): prints rope (spring) points.
- `rcefMemChk`, `rcefMemDebugDisp`, `rcefMemDebugInit` (`0x00937400`,
  `0x00937440`, `0x009374f0`): the effect heap's checks and usage prints.
- `0x0093ce80`, `0x0093cfa0`: print integer vectors and matrices.
- `0x00a5bda0`: the sphere grid's "Ps2 Communication Start" debug
  entry.
- `0x00760bf0`: the VFX "drawField is not implemented" warning loop
  (listed in round 12).

Debug camera:
- `FreeCamera_handleInput` (`0x00680030`), `FreeCamera_setFreePause`
  (`0x00680320`) and `0x0067fda0`: the free debug camera.

`host0:`/`host:` readers and writers:
- `yiLoadBattleFieldParticleNo`/`yiLoadFieldParticleNo` (`0x00909120`,
  `0x009092e0`): field particle files from `host0:/ffx/proj/map/f_particle`.
- `yiImage_readonlyTIM1`/`TIM2` (`0x00918f20`, `0x00919370`): TIM images
  from `host0:`.
- `yiMapSetup` (`0x00909680`, ported earlier): builds the
  `host0:%s%s.vpa` path.

The `rcui` map editor and the map tool's text format:
- `rcuiCameraProc` (`0x00931910`), `rcuiMotionProc` (`0x00935340`), the
  menus `0x00931e90`, `0x009325c0` and `0x00935000`, `rcuiVramWrite`
  (`0x00937190`), `rcuiResetChrPos` (`0x00937b70`) and `rcuiMakeFullPath`
  (`0x00937b20`): the in-game map editor's windows.
- `rcuiLoadSetTitle` and `rcuiMakeBasePath` were ported earlier.
- `0x0093c730`: the editor's Z value bar (also under "Replaced by
  ffx-render").
- Writers to `host:` and shell calls:
  - `0x00934bc0`: the `map2d.bin` path.
  - `0x00938010`: `chmod`.
  - `rcbgGuideMapConvSub` (`0x00938080`) and `rcbgGuideMapSaveSub`
    (`0x009391d0`): guide map conversion and save.
  - `rcbgFpSaveSub` (`0x00938350`): field particle save.
  - `0x00938590` and `0x00938a10`: `gmobjdat` saves.
  - `rcbgSaveBinNormal` (`0x00939040`).
  - `rcbgSaveChkPackList` (`0x00939480`).
  - `rcbgSaveText` (`0x00939b10`).
  - `0x0093a200`: `ls -l`.
- The text map loaders:
  - `rcbgFpLoadTextSub` (`0x0093a800`), `0x0093a890`,
    `rcbgFpTrxTexLoadTextGetTrxCnt` (`0x0093a910`) and `0x0093aa60`.
  - `0x0093ab80`: a transfer's text config, ten keywords.
  - `0x0093af60` (the camera's text loader).
  - `rcbgSceneLoadTextModel`, `rcbgSceneLoadTextSub` and
    `rcbgSceneLoadTextSubConfig`.
  - `rcbgSceneLoadTexture`, `rcbgSceneLoadTextureAnime`,
    `rcbgSceneLoadTextureAnimeRect` and `rcbgSceneLoadTextureNormal`.
  - `rcbgTrxTexLoadFromFile` (`0x0093beb0`).
- Their keyword helpers `rcbgPalLoadConfig`,
  `rcbgSceneLoadTextSubConfigKeys`, `rcbgSceneLoadTextCameraCalcRef`
  (also called by `rcuiCameraProc`) and `rcbgAnimeLoadConfig` were ported
  earlier.

## Sphere grid set-up and lens beams (round 18)

From here on, functions are picked by `symbols/classes.tsv` (category
`logic`) and blocker count.
- `NkzTidusLimitTableA`/`B` (`0x00a5cb30`/`0x00a5cb80`): four-entry
  local tables (24, 20, 20, 18 and 11 to 14). An index past 3 reads the
  original's stack; the port returns 0.
- `AbmapSetKindSizes` (`0x00a5adf0`): the eight node kinds' sizes from
  `op_oef2_a_get`.
- `AbmapMarkLockedEntries` (`0x00a5a2e0`): menu entries whose save bit is
  clear.
- `AbmapLinkCells` (`0x00a5a760`): a link's end cells on the grid's
  20-wide map.
- `AbmapCollectNodeLinks` (`0x00a5b140`): each node's links (up to five;
  a sixth overflows into the next words after a `dbgPrintf`, as in the
  original).
- `KeThCp_AddBp` (`0x007644d0`): a beam block taken off its pool's free
  list, set up (`KeThBP_Init`) and pushed onto the lens's list.

## Sphere grid items, node shapes and fog (round 19)

- `AbmapItemUsableAt` (`0x00a49310`): 0 unless the item is held
  (`MsGetSaveItemNum`); else the ability bits it grants a character. The
  item's record (`MsGetRomItem`, byte `+0x5d`) picks an item excel row
  (`0x01a860e4`); a row whose byte `+0xc` is `' '` covers every node
  (`AbmapForEachNode`), else the node and the nodes linked to it
  (`AbmapForNodeAndLinked`). The context on the stack is the row, the
  character, `1 << chr`, the result and (apply only) a mark byte.
- `AbmapItemNodeEffect` (`0x00a49440`), its callback: the node's excel
  row (`0x01a860e0`, by its id `+6`). A special item kind (row `+0xd`)
  asks `0x00a5d120(kind, node index, row, chr)`; else the item's ability
  bits (`+0xa`) present in the node row, unless the node is activated for
  the character (`+0x21`) or hidden (bit 15 of the row's `+0x10`).
- `AbmapItemApplyAt` (`0x00a5b400`) and its callback
  `AbmapItemApplyNode` (`0x00a5b490`): the same walk without the held
  check; a node with an effect gets the mark ORed into `+0x22`.
- `AbmapSetupKindShapes` (`0x00a5b230`): the 130 node kinds (0x30 bytes
  from the grid's `+0xf828`): two shape records picked by the row's
  `+0x16` (`0x01a85f98`, `0x01a85fe8`), a size from the first one's box
  over 16, a size class shape (`0x0230578c`/`90`/`94`: under 24, under
  32, else), scale `size * 7.8125e-5` and for visible kinds seven circle
  points of radius `size / 2 + 3`. Every shape table entry is read.
- Fog: the colour is four bytes at `0x018df140`, the limit, near and far
  `f32`s at `0x018df144`/`48`/`4c`, the distances in units of `scale
  factor * 0x00b501a0 * 0x00b412f8` (both `f64`).
  `yiSetFogColor` (`0x0090a2c0`) and `yiSetFogParam` (`0x0090a350`) store
  them; `yiGetFogParam` (`0x00908bb0`) reads them back (far first). On map
  0xe0 only, the PC renderer's fog is used: the setters also call
  `graphicSetFogColour` (`r, g, b / 255`), `graphicSetFogNear`/`Far`
  (times `0x00b47c68`), `graphicSetFogLimit` (`(1 - limit / 255) *
  0x00b48318`) and `0x00643180(1.0)`; the getter returns
  `graphicGetFogNear`/`Far` and `graphicGetFogLimit * 255`.
- `yiSetBackColor` (`0x0090a040`): four bytes at `0x018df178`; unless the
  current magic (`MagicFile_GetMagicId`) is 0x2c2 also
  `graphicSetBackgroundColour(r, g, b / 255, a * 0x00b170b8)`.
- `yiDisableHamonEff` (`0x00908450`): clears render status bit 24 and the
  distortion word `0x018ded28`, `graphicSetDistortionEnable(0)`.
- Not ported this round: `yiGetFogColor` (`0x00908980`, the clamped
  colour scale and offset, kept on the x87 stack in a different order
  from `yiGetBackColor`'s; next).

## Fog colour and the sphere grid's pads (round 20)

- `yiGetFogColor` (`0x00908980`): with render status bit 2 the fog colour
  bytes go through the colour scale and offset, clamped to 0..=255.0 and
  converted (`_ftol2_sse`), as in `yiGetBackColor`; else the raw bytes.
  Either way the first three are then replaced by the PC renderer's
  colour times 255.0, read from `graphicGetGlobalAmbientColor` (bit 2) or
  `graphicGetFogColor`; each product stays on the x87 stack. The x87
  stack juggling of the clamps does not change the values (all are
  `f32`-exact). Returns the last conversion.
- `AbmapPadStartSlideOut`/`In` (`0x00a48c20`/`0x00a48f20`) install the
  pad steps `AbmapPadSlideOutStep`/`InStep` (`0x00a59800`/`0x00a5a270`) at
  `+0x34`: three frames moving `+0xa` from `+2` by `n h / 3 + h / 6` (`h`
  at `+6`) and setting the shown flag `+0x23`.
- `AbmapRectTweenIn`/`Out` (`0x00a5a080`/`0x00a5a170`): a rectangle at
  `+8` tweened between `+0` and `+0x2a` over four frames (counter `+0x32`,
  16.16 steps of a quarter), its cells `(y + 15) / 16` and `(h + 15) / 20`
  at `+0x14`.
- `AbmapMaskNodeFlags` (`0x00a47400`): a mask ANDed into every node's
  `+0x22`.
- `AbmapMarkEmptyMenus` (`0x00a5aca0`): marks the first menu entry when
  the character has no sphere points and the second when the item list is
  empty. It calls `TOAmapGetItemListMax` through the jump at
  `0x008c6700`.
- Not ported: `0x0092eba0` (stores five arguments in `0x025d5c18..2c`
  and sets `0x025d5c34`, which `0x009304e0` waits on; no name yet).
  `0x0083bdb0` is a `Sg_Mem` pool helper (system). The `Sqs*` functions
  stay with the sound agent.

## Effect requests, finalisers and the sphere grid's menus (round 21)

- `rcefPostRequest` (`0x0092eba0`, name ours; an exported effect import,
  `g_MagicImports` at `0x00c65494`): stores `a, f, b, 0, c, d` at
  `0x025d5c18` (`f` a float copied through the x87 stack, so a signalling
  NaN turns quiet) and sets `0x025d5c34`. Returns `d`. Nothing in the PC
  build reads the request or clears the flag;
  `rcEffectTaskRequestWait` (`0x009304e0`) is the sequence that waits for
  the flag to be 0.
- `rcefAnimeFini` (`0x0092f4a0`): `graphicVFXDestroy` of the handle
  `id << 32`. `rcefModelFini` (`0x0092f710`): the handle
  `(id << 32) + g_pppCurrentProgram` (sign-extended), destroyed when
  `graphicVFXGetRenderDataByInstance` finds it. Both return the last
  call's `eax`.
- `yiAnimDetachAll` (`0x009149a0`, name ours): `yiAnimDetach` without the
  kind test; `yiDisposeAnimDObj` uses it for a negative kind.
- `Yn_DrawScene` (`0x0091a240`, name ours) jumps through `0x00921a10` and
  `0x009215e0` to `rcbgSceneDraw(map +8)` when Yn's state and its map
  exist; otherwise `eax` is left as it was (the state pointer when only
  the map is missing). Its one caller ignores `eax`, so the shim is void.
- `AbmapItemEffectAtNode` (`0x00a493b0`, name ours, from
  `AbmapTryUseItem`): `AbmapItemUsableAt` for a single node index, same
  context layout (`+0x10` stays uninitialised).
- `AbmapEnterCommandMenu` (`0x00a59950`) and `AbmapEnterItemMenu`
  (`0x00a598a0`), names ours, hand over between the two menu pads. Each
  pad is `0x340` bytes, so `+0x16e3` is pad 7's `+0x23` and `+0x1a23` is
  pad 8's. Entering shows the pad (`+0x23` 1, `+0x24` 3, `+0x25` 1),
  stores the pad number at `+0x2700` and the handler at `+0x2704`
  (`0x00a56330` for the command menu, `0x00a560d0` for the item menu).
  The command menu runs `AbmapMarkEmptyMenus` first and sets bit 3 of
  `+0x24`. The item menu marks the items `AbmapItemUsableAt` rejects at
  the current character's node.
- Not ported:
  - `0x00916130` (`yiRender`'s 72 CLUTs of 256 entries, from
    `0x00908400`) rebuilds byte 1 of each entry from an uninitialised
    local (`[ebp-0x14]`), so its result depends on stale stack contents.
  - `0x00911620` is VIF unpack emulation (render, replaced by
    ffx-render).
  - `0x006e9920`, `0x006e9ae0`, `0x006ea140`, `0x006eab10` and
    `0x006eb1a0` are `std::map` and dlmalloc copies, and `0x0064e9e0` is
    a reference-count release: platform code that classes.tsv still has
    as logic.
  - `0x0067c170`/`0x0067c210`/`0x0067c2d0` are `MenuManager` vertex and
    material set-up for PhyreEngine (glue).
  - The `0x0081xxxx` functions are the sound sequencer's (sound agent).

## Disposing, finalising, command menus and the guide's fade (round 22)

- `yiDisposeAnimDObj` (`0x009084b0`): on `g_YiRender`, `yiAnimDetachAll`
  for a negative kind, else `yiAnimDetach`.
- `rcefObjFini` (`0x0092fee0`, an effect object's finaliser): the
  particle part's finaliser and the destructor are empty (`0x0092efc0`,
  `0x0092ed00`); between them `rcefAnimeFini(obj + 0x330)` (the
  animation's address used as the id) and `rcefModelFini(obj + 0x3c0)`,
  whose `eax` it returns.
- `AbmapFillCommandMenu` (`0x00a459e0`, name ours): `eiAbmStart` fills
  menu pads 3, 2, 4 and 5 with the menu status entries
  (`MsMenuStatusNum`) of kinds 0x40, 0x41, 0x45 and 0x44. Each entry
  goes through the inlined `addMenuPrim`: 12-byte entries from `+0x40`
  (`+0x40` command id, `+0x44` 0, `+0x48` id - 0x3000), the `i16`
  count at `+0x1e` (at most 0x40; a full pad prints the error and calls
  `debug_exit_trace(0)`), and `+0x28` set when the count is above rows
  `+0x22` times columns `+0x16`.
- `Yn_GuideEdgeFade` (`0x0091f530`, name ours, from `0x0091cf10`): `h` is
  half the rectangle's width and `r` the point's distance from the
  clamped segment (`Yn_NearestOnLine2D`). Beyond `0.2 h` it writes 1.0
  and returns 1. Otherwise it writes `(r - 0.15 h) / (0.2 h - 0.15 h)`
  clamped to 0..1 (a NaN passes through) and returns 0. Every
  intermediate is stored as `f32` except the denominator. The original
  reads the point a second time through `edx`, which
  `Yn_NearestOnLine2D` leaves alone; the port reads the argument again.
- `rcbgSceneSetChadist` (`0x00920790`, name ours; the thunk at
  `0x00920780` jumps to it): the character distance is the player's
  camera-space z (`Ch_GetPos`, w 1.0, through `g_CameraMatrix`) times
  0.1, or 98.0 without a player. It is stored at `+0x100`, `+0x104` is
  set, `+0x10` of Yn's map is cleared, and the distance goes to
  `graphicSetChadist`.
- Not ported:
  - `AbmapFillUseItemMenu` (`0x00a45fd0`) builds `+0x48` from a 32-bit
    local of which only the low half (the item) is written, so the high
    half is stale stack.
  - `0x00a45e30` and `0x00a4c460` build UI draw elements and packets
    (replaced by ffx-render).
  - `0x009161d0` is a CRT `_splitpath` copy (platform).
  - The `rcPrim*` functions are packet writers (replaced by ffx-render).

## Effect sequence helpers and Yn's layer switches (round 23)

- Waits: `rcEffectTaskSoundCueWait` (`0x00930500`, name ours) returns once
  `op_oef2_sound_cue_check(0x800)` is 0. `rcEffectTaskMotionWait`
  (`0x00930430`) returns once `MsEffectCheckMotion` of the current
  level's first argument is -1.
- Forwarders, all returning the last call's `eax` (names ours unless
  Fahrenheit's):
  - `rcefSetChrDirTarget` (`0x0092e750`) and `rcefSetChrDirTarget2`
    (`0x0092e960`) call `MsSetChrDirTarget` with modes 1 and 2.
  - `rcefWeaponSetHide` (`0x0092eb70`) calls `Ch_SetHideEffect(weapon,
    show == 0)` when `MsGetWeaponHandle(chr, 0)` returns a handle.
  - `rcefPlaySE` (`0x0092e680`) plays sound entry `id` of the effect's
    oef2 data (`0x025d5a28`) with `op_oef2_se_on(entry, 0x40, 0x100)`.
  - `rcefMainCreateTask` (`0x00925170`) creates an object and sets its
    sequence; it returns the object.
  - `rcefChrDelete` (`0x00930600`) calls `rcefChrFini`, then frees the
    object into the current ppp environment (`*g_pppEnvSlot`, its pool
    at `+0x10`).
  - `rcfxSpringDelete`/`rcfxNailDelete` (`0x00928c60`/`0x00929070`)
    call the fini, then `rcFree`.
  - `Yn_FPm173End` (`0x00919ab0`) calls `rcbgFPm173End` on the scene's
    first object of kind 0x20008.
  - `rcbgTrxPairSetNo` (`0x009221e0`) sets the number of two transfers
    at `+0x20` and `+0x60`.
- `Yn_SetChadist` (`0x0091a250`, name ours) is `Yn_DrawScene`'s sibling
  through `0x00921a30`/`0x00921610` to `rcbgSceneSetChadist`.
- `Yn_PreBgLayerDispOn`/`Off` (`0x0091a830`/`0x0091a7c0`): below
  `Yn_MapPartCount`, bit 0 of the part (`0xc0` bytes from the scene's
  `+8`) is cleared (on) or set (off). Below the same count, or 0x1c on
  maps 0x15b and 0xd4, `graphic2DShowLayer`/`HideLayer(n)` runs and
  `graphicTextureAnimationSetMaterialVisiability(n, on)`. Without a scene
  the original reads address 8. `On` first sets `0x025d5ebc` when the
  path at `0x023cbc60` contains `"te/test20"` (`strstr` through the
  import table). When the layer is past the limit, `eax` is the map
  number.

## The aeons' stat gains and the effect's stream forwarders (round 24)

- The aeon stat table (`0x00c5a604`, read by `TkMenuCtrlSummon`) has
  `0x1c`-byte rows. Row 0 is empty, then HP, MP, strength, defence,
  magic, magic defence, agility, luck, evasion and accuracy. Each row
  starts with its gain function and ends with the maximum (99999, 9999,
  then 255). The gain functions (names ours) take `(pc, v)`:
  - HP (`0x00a5c7b0`): `trunc(sqrt(v) * 0.25 + 16.0)`, at least 1.
  - MP (`0x00a5c8d0`): `trunc((sqrt(v) + sqrt(v)) / 3.0 + 10.0)`, at
    least 1.
  - The other eight (`0x00a5c660`, `0x00a5c740`, `0x00a5c910`,
    `0x00a5c980`, `0x00a5c9f0`, `0x00a5c860`, `0x00a5c6d0`,
    `0x00a5c7f0`) have identical bodies: for PCs 8..=17 (the aeons)
    `v / 2` rounded toward zero, at least 1; any other PC prints
    `"NKZ:Illigal PC Number %d"` and gives 100. HP and MP don't check the
    PC.
  - Both roots use `_CIsqrt` and `_ftol2_sse` at 53-bit precision.
- Effect forwarders, each returning the last call's `eax`:
  - `rcefSetVibrate` (`0x0092ec60`, name ours; Fahrenheit's Switch BSim
    guess was `rcefStreamCue`) calls `TkSetVibrateStream(7, arg,
    entry)` with entry `id` of kind 5 of the effect data.
  - `rcefStreamCue` (`0x0092ebf0`) calls `op_oef2_stream_cue(entry, arg,
    0x800)` with entry `id` of kind 6.
  - `rcefPlayStream` (`0x0092e6b0`) returns `op_oef2_sound_cue_check
    (0x800)` when it is non-zero, else plays the entry like
    `rcefPlaySE`.
  - `rcefObjAnimeStart`/`rcefObjSetupDraw` (`0x00925590`/`0x00925690`,
    effect imports, names ours) start the object's animation (`+0x330`)
    or set up its draw record (`+0x3c0`) from the kind 4 entry of the
    environment's data (`env +8`), with outputs at `+0x1f0` and `+0x170`.
- Left for the debug list: `rcefMemDebugInit`/`rcefMemDebugDisp`
  (`0x009374f0`/`0x00937440`).

## Copies, the morph value, interpolation and colour work (round 25)

- `rcuiScaleVector_copy_00756600` and `rcVec4fToFixed_copy_007e6cc0` are
  copies of `rcuiScaleVector` and `rcVec4fToFixed` that the compiler
  kept; their shims use the same bridges.
- `yiSetMIMeParamDObj` (`0x0090a4d0`): for a display object below
  `g_YiDObjCount`, copies the float at `p` through the x87 stack into
  the object's morph value (`0x018df624`, `0xb4` bytes apart) and passes
  it to `graphicSetMeshVertexColorMorphValue`. Returns 0.
- `yiGetColorOffsetInt` (`0x00908660`, an effect import, name ours): the
  colour offsets plus 128.0, truncated into four ints.
- `rcefLerp` (`0x0092e580`, an effect import, name ours) returns
  `a + w (b - a)` in `ST0`:
  - mode 2: `w` is the sine table's entry
    `trunc(t * 16384) >> 4 & 0xfff`.
  - mode 1: a cosine ease, `(sin(t * 32768 - 16384) + 1) / 2`, stored
    as `f32`.
  - any other mode: `w` is `t`.
  - `b - a` and the result are stored as `f32`.
- `rcefColorSet`/`rcefColorFade` (`0x0092e9c0`/`0x0092ea40`, effect
  imports, names ours) convert colour bytes to floats and start the
  effect's colour work at `0x025d5a20` with `rcefWorkSetFromVf00` (from
  the current colour) or `rcefWorkSet` (from colour 0 to colour 1).
- Not ported:
  - `0x00a48e40` and `0x0090dd00` have no caller or pointer in the image.
  - `0x0092e4f0` loops on `rand()` through the import table.
  - `0x00a45930` sets the ppp matrices and runs a sphere grid ppp part (vfx).

## Screen shake, scene wave and the sphere grid's part start (round 26)

- The screen shake and the scene's wave share a swing: the sine table
  entry `(0x4000 - trunc(phase * -65536.0 / 6.2831855)) >> 4 & 0xfff`.
  Faded, it becomes `f32(frames - done) / f32(frames) * swing`, each int
  stored as `f32` first, so frame counts beyond 2^24 round. The amplitude
  is `rand() % range + base` through the import table. A zero range
  faults in the original; the port then uses `base`.
- `rcefScreenShake` (`0x00924df0`, name ours, from
  `rcefMainProcRunAfter`): on `+0x214` with mode `+0x1f8`. The swing of
  the phase `+0x210`, faded over `+0x200` frames for modes from 3, times
  the amplitude (`+0x20c`, `+0x208`) gives the x offset (odd modes) or
  the y offset. The phase advances by `+0x1fc` and `+0x204` counts the
  frames. Within the frames it calls `MsScreenOffset(x, y)`; after them
  `MsScreenOffset(0, 0)`, and the shake turns off.
- `rcbgSceneWaveC0Update` (`0x009210e0`, name ours, from `0x00920aa0`):
  the same with mode `+0xc0`, phase `+0xd4`, step `+0xc4`, frames
  `+0xc8` (done `+0xd8`; faded for modes 3 and 4), amplitude `+0xcc`
  (plus the random part only when `+0xd0` is positive). The result goes
  to `+0xb8` (odd modes) or `+0xbc`. It calls `rcbgSceneWaveC0Stop` after
  the frames, then `rcbgSceneSetMatrix`.
- `AbmapStartPart` (`0x00a5bad0`, name ours): starts sphere grid ppp part
  `n` in the grid's environment (`0x01a86034`) through
  `pppStartPart(part, 0x1000, id, 1, 0)`; Ghidra shows
  `get_ptr32_func`'s two leftover pushes as its arguments. It then fills
  the part: `+0x40` x, y, z, 1.0, `+0x50` three truncated floats,
  `+0x5c` 1, `+0x60` three floats, `+0x6c` 1.0. Returns the third
  truncated value.
- Not ported: `0x00a461b0` and `0x0092f290` write draw packets and VFX
  render data (replaced by ffx-render).

## The Phyre map load, the sphere grid's warp and a 2D angle (round 27)

- `loadPhyreFieldMap` (`0x00908000`): with a pending map (`0x01934ed0`)
  and its name (`0x01934ee8`, a string), it switches both map kinds off
  and prints `"RAM Inuse : %d u:%d"` (0, 0) to `stdout` through
  `__iob_func`/`fprintf`. Then, for each section `yiMapGetSection` finds
  (0: 2D, 1: 3D), it switches that kind on, counts it (`0x01934ec4`),
  loads it (`graphicFieldMapLoad(name, kind)`) and sets the map kind
  (`0x01934ec0`). The 2D section also sets render status bit 14 and
  `0x0191389c`. The request is then cleared, followed by
  `graphicFieldMapInitialize` and `graphicVFXLoadLevelData` (its `eax`
  is returned). Without a request it only clears `g_BtlMapRead`.
- `AbmapStartWarp` (`0x00a48a80`, name ours, from `abmap_set_chr_pos`):
  - plays the warp sound;
  - writes the character's node position as floats (`+0x11308`/
    `+0x1130c`, again at `+0x1163c`/`+0x11640`), the target (`+0x11634`)
    and the character (`+0x11638`);
  - creates the grid's ppp heap and starts part 2 there
    (`AbmapStartPart`, scale 0.5);
  - hooks the warp's draw (`0x00a4c430`, at `+0x115ac`) and
    `AbmapState_Warping` (`+0x115a8`), saving the previous hooks unless a
    save slot is taken.
- `rcAngle2D` (`0x007e6d80`, name ours) returns in `ST0` the direction of
  `(x1 - x0, y1 - y0)` in `0..2π`:
  - the quadrant `q` (a NaN counts as negative) turns the vector into
    non-negative `(a, b)`;
  - the angle is `atan(b / a)` when `a >= b` (or unordered), else
    `π/2 - atan(a / b)` stored as `f32`;
  - then `q π/2` is added (`π/2` the `f64` of an `f32`).
- Left for the debug list: `0x007606f0` (cross lines through
  `0x00760ce0` around a point at `0x00d4b4e8`).

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
