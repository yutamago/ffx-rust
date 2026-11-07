# Characters

The PS2 `Ch` module keeps every character on the map or in battle (party,
NPCs, monsters, objects) in one array of fixed-size objects. Each points to
a model block, which holds what was read from the character's `.chr` file
(`docs/formats/motion.md`). Code: [`crates/ffx-chara`](../../crates/ffx-chara)
(`layout` has these offsets as constants, `chr` and `handle` the ported
functions).

## Characters in memory

`g_ChrArray` (`0x023c44e4`) points to `g_ChrCount` (`0x023c44e0`) objects
of `0x880` bytes (`Ch_CheckHandle`, `0x008261b0`). A handle is an object's
address. The field snapshot has 37 slots, 4 of them in use (ids `0x0001`,
`0x5001`, `0x500b`, `0x0029`). The battle snapshot has 36 slots: 7 in use
and 7 more that still have their model block.

| Offset | Type | Meaning | Shown by |
|---|---|---|---|
| `0x000` | `u16` | id, `kind << 12 \| number` | `Ch_GetId` (`0x00826b80`) |
| `0x002` | `u8` | slot in use | `Ch_CheckHandle`, `Ch_HandleGetNext` (`0x00826d70`) |
| `0x004` | ptr | name: the model block's `+0x71` | `Ch_GetName` (`0x00826b90`) |
| `0x00c` | `f32` ×4 | position x, y, z, w | `Ch_GetPos` (`0x0082ac90`), `Ch_SetPos3` (`0x0082b480`) |
| `0x01c` | `f32` ×4 | where the character goes this frame; `Ch_SetPos` copies the position here word by word | `Ch_GetVel` (`0x00834000`, this minus the position), `Ch_SetPos` (`0x0082b440`) |
| `0x02c` | `f32` ×3 | a second point | `Ch_GetVel2` (`0x00834030`) |
| `0x050` | `f32` | vertical velocity, the argument × 0.05 | `Ch_SetVelY` (`0x0082b920`) |
| `0x05c` | `f32` ×3 | scale | `Ch_SetScale` (`0x0082b560`), `Ch_GetScaleX` (`0x0082ad30`) |
| `0x06c` | `f32` ×3 | offset | `Ch_SetOffset` (`0x0082b3c0`) |
| `0x07c` | `f32` ×3 | ground normal; (0, -1, 0) when the character leaves the ground | `Ch_EnMapID` (`0x0082aa70`) |
| `0x0bc` | `f32` ×3 | shade colour multipliers | `Ch_SetShadeColEffect` (`0x0082b6e0`) |
| `0x10c` | ptr | matrix the character is placed by | `Ch_SetOrgMatrix` (`0x0082b3e0`) |
| `0x110` | `f32` | scale for that matrix | `Ch_SetOrgMatrixScale` (`0x0082b400`) |
| `0x154` | `f32` | speed | `Ch_SetSp` (`0x0082b840`) |
| `0x158` | `f32` | rotation about y (radians) | `Ch_GetRot` (`0x0082ace0`) |
| `0x168` | `f32` | direction | `Ch_SetDir` (`0x0082b190`) |
| `0x16c` | `f32` | ground height | `Ch_SetJimen` (`0x0082b360`) |
| `0x170` | `f32` | speed from which the character runs; 18 for 0 or less | `Ch_MoveSetChgRun` (`0x0082b040`) |
| `0x17c` | `f32` | shadow height | `Ch_ShadowSetHeight` (`0x00830cb0`) |
| `0x180` | `u8` | hide bits: 1 event, 2 effect, 0x10 battle | `Chr_SetHideFlags` (`0x0082ba30`) |
| `0x181` | `u8` | hidden from the calculation pass | `Ch_SetHideCalc` (`0x0082b280`) |
| `0x182` | `i8` | ground mode | `Ch_SetGravMode` (`0x0082b240`) |
| `0x183` | `i8` | shadow mode | `Ch_SetShadowMode` (`0x0082b820`) |
| `0x185` | `i8` | field mode (also the default motion's slot) | `Ch_SetFieldMode` (`0x0082b1e0`) |
| `0x184` | `u8` | sound id | `Ch_SoundSetId` (`0x00835b50`) |
| `0x186` | `u8` | motion type | `Ch_SetMotionType` (`0x0082b3a0`), `Ch_GetMotionType` (`0x00833f70`) |
| `0x188` | `u8` | point light bits | `Ch_PLightSetEnableMask` (`0x00830b00`) |
| `0x18c` | `u32` | battle character number | `Ch_SetBattleChrNum` (`0x0082b0d0`) |
| `0x194` | `u32` | flags (below) | |
| `0x198` | `u32` | more flags: bit 2 chocobo, bit 3 VU1 clip mode, bit 5 skipped by the OEF2 lookup | `Ch_SetChocobo` (`0x0082b130`), `Ch_Vu1SetClipmode` (`0x0082b970`) |
| `0x190` | `u32` | what a bound child copies to its `+0x1b8` | `Ch_BindToElemDirect` (`0x00832680`) |
| `0x19c` | ptr | the character this one is bound to; bound characters take its shade | `Ch_BindToElemDirect`, `Ch_SetShadeID`'s helper (`0x0082b7a0`) |
| `0x1a0` | `u32` | the parent's element it is bound to | `Ch_BindToElemDirect` |
| `0x1a4` | `f32` ×3 | offset from that element: the arguments × 100 × the block's `+0x38` | `Ch_BindToElemDirectOfs` (`0x008326e0`) |
| `0x1b0` | `f32` | 1 after each bind | `Ch_BindToElemDirect` |
| `0x1b4` | `f32` | bind scale | `Ch_BindSetScale` (`0x00832610`) |
| `0x1b8` | `u32` | the parent's `+0x190`, or -1 when unbound | `Ch_BindToElemDirect` |
| `0x1bc` | ptr | models, `0x38` bytes each: `+4` normals, `+8` vertices, `+0x14` alpha, `+0x18` hidden | `Ch_ModelGetHide` (`0x0082add0`), `Ch_ModelGetNromPtr` (`0x0082adf0`) |
| `0x1c0` | ptr | header: `u16` version at `+4` (counts valid from `0x831`), models at `+6`, elements at `+0xa` | `Ch_GetNbModel` (`0x00830410`), `Ch_GetNbElem` (`0x008303f0`) |
| `0x1c4` | ptr | model block in `g_ChrBlk` (below) | `Ch_GetBlkDirName` (`0x00826a60`) |
| `0x328` | ptr | elements (bones), `0x160` bytes each: `+0` data (its `+4` the parent), `+0xc8` replacement matrix, `+0xcc` quaternion matrix, `+0xe0` its weight | `Ch_ElemGetParent` (`0x00833cf0`), `Ch_ElemSetQuatMatrix` (`0x00833df0`) |
| `0x330` | fade ×3 | shade colour r, g, b | `Ch_SetShadeCol` (`0x0082b5d0`), `Ch_DrawCalcShade` (`0x0082eaf0`) |
| `0x354` | fade | alpha; 0 also hides from the calculation pass | `Ch_SetTransparent` (`0x0082b900`), `Ch_GetHideCalc` (`0x0082abe0`) |
| `0x360` | fade | shade | `Ch_SetShade` (`0x0082b5b0`) |
| `0x36c` | fade | light value; multiplies the shade when `g_ChrShadeByLight` is set | `Ch_GetLightValue` (`0x0083cb90`), `Ch_DrawCalcShade` |
| `0x378` | fade | specular alpha | `Ch_SetSpecularAlpha` (`0x0082b860`) |
| `0x408` | `u32` | effect level | `Ch_EffectSetEffectLevel` (`0x0082a9d0`) |
| `0x4e4` | `f32` | collision radius; 0 or less takes the block's `+0x1c` | `Ch_SetThickness` (`0x0082b880`) |
| `0x4ec` | `f32` | battle camera thickness; negative takes the block's `+0x24` | `Ch_SetThicknessBtlCam` (`0x0082b8c0`) |
| `0x4f0` | `f32` | fixed height; negative takes the block's `+0x28` | `Ch_SetFixHeight` (`0x0082b200`) |
| `0x4f4` | `f32` | camera height; negative takes the block's `+0x2c` | `Ch_SetCamHeight` (`0x0082b0f0`) |
| `0x4f8` | `f32` | hit height | `Ch_SetHitHeight` (`0x0082b320`) |
| `0x500` | `f32` | weight when characters push each other | `Ch_SetWeight` (`0x0082b940`) |
| `0x504` | `f32` | gravity | `Ch_AddGrav` (`0x0082a890`), `Ch_GetGrav` (`0x0082abb0`) |
| `0x420` | `f32` | neck turning speed; 0 is stored as 6 | `Ch_NeckSetSpeed` (`0x00835020`) |
| `0x424` | `f32` ×2 | neck limits | `Ch_NeckSetLimit` (`0x00834fd0`) |
| `0x4d4` | `i8` | automatic mouth movement | `Ch_MouthSetAutoTalk` (`0x00834770`) |
| `0x4d8` | `f32` | 1234 after `Ch_MouthInit` (`0x00834750`) | |
| `0x51c` | `f32` | (`hiju`) | `Ch_SetHiju` (`0x0082b340`) |
| `0x524` | 16 bytes ×n | optional positions x, y, z, w; one exists when its w (`0x530`) is not 0. `Ch_OptposGet` takes position `i` if any component is not 0, else position 0 if its w is not 0, else the point at `0x200` with w = 1 | `Ch_OptposGetExist` (`0x00835680`), `Ch_OptposGet` (`0x008354f0`) |
| `0x694` | 16 bytes | bottom position | `Ch_OptposGetBottom` (`0x00835600`) |
| `0x6a4` | 16 bytes | top position | `Ch_OptposGetTop` (`0x008356e0`) |
| `0x6b4` | 16 bytes | camera reference | `Ch_OptposGetCameraRef` (`0x00835630`) |
| `0x6d8` | `u32` | horizontal shadow direction | `Ch_SetShadowDirH` (`0x00830c90`) |
| `0x6e4` | ptr, `i16` at `0x6e8` | clut textures, `0x40` bytes each: address `+0x30`, `u16` base `+0x3a` | `Ch_TextureGetClutAddr` (`0x00830d30`), `Ch_ClutGetNb` (`0x0083c9a0`) |
| `0x6ec` | ptr, `i16` at `0x6f0` | image textures, the same layout | `Ch_TextureGetImageAddr` (`0x00830d90`) |
| `0x700` | `u32` | texture animation id | `Ch_TextureGetAnimId` (`0x0082b960`), `Ch_TextureAnimGetHandle` (`0x00835ed0`) |
| `0x704` | ptr | cluts, 16 bytes each: pointer, original pointer, size in bytes (32-byte header, 4 per entry) | `Ch_ClutGetPtr` (`0x0083c9d0`), `Ch_ClutGetSize` (`0x0083c9f0`) |
| `0x708` | `u8` | which of the two `Ch_ClutEnable` (`0x0083c940`) put in the clut textures | |
| `0x726` | `u16` | motion loop | `Ch_MotionSetLoop` (`0x00839960`) |
| `0x728` | `u16` | cleared for each new motion | `Ch_ResetMotion` (`0x0082b080`) |
| `0x730` | `u32` | 1 for each new motion | `Ch_ResetMotion` |
| `0x736` | `u8` | a scripted motion plays | `Ch_ResetMotion` |
| `0x73a` | `u16` | interpolation frames into the next clip; 0 marks the object dirty | `Ch_MotionSetHokan` (`0x0082afb0`) |
| `0x73c` | `u16` | interpolation frames of the next motion; -1 is stored as 0, 0 as 1 | `Ch_MotionSetNextHokan` (`0x0082b010`) |
| `0x750` | `i16` | motion speed, 8.8 fixed | `Ch_SetMotionSpeed` (`0x0082b380`) |
| `0x72c` | `u32` | current motion id | `Ch_MotionGetId` (`0x008371a0`) |
| `0x740` | `u32` | current motion frame, 8.8 fixed | `Ch_MotionGetFrame` (`0x00839860`) |
| `0x75c` | `u32` ×n | motions by slot (stand, walk, run); -1 takes `g_ChrDefaultMoves` (`0x00c49680`) | `Ch_MotionSetMoves` (`0x0082afe0`) |
| `0x810` | `u16` | 1 after `Ch_ClothSetFast` (`0x00824920`) | |
| `0x812` | `u16` | cloth settle frames: 10, 15, 20 or 1 by character id (table `g_ChrClothFastCases`) | `Ch_ClothSetFast` |
| `0x81c` | `f32` | 0, or 2 for the ids of case 1 | `Ch_ClothSetFast` |
| `0x820` | `u32` | bit 0: cloth gravity; bit 2: needs an update | `Ch_ClothEnableGrav` (`0x008245f0`), `Chr_SetHideFlags` (on clearing) |
| `0x824` | `i16` | -1 after each `Ch_SetPos` | `Ch_ResetPos` (`0x0082b0b0`), `Ch_IdGetPNum` (`0x0082adc0`) |
| `0x828` | `u32` | map id word (below) | `Ch_IdGetMap` (`0x0083d840`) |
| `0x830` | ptr | render instance | `Ch_GetChrRenderInstance` (`0x00826b70`) |
| `0x864` | `f32` ×3 | cloth mesh offset; character `0x65` gets y = 11245 | `Ch_ClothMeshMappingOffset` (`0x00824620`) |

A fade is 12 bytes: current `f32`, target `f32`, frames `i32`. Setting one
(`Chr_FadeSet`, `0x0082b990`) stores target and frames, and the current
value too when frames is 0.

Flags at `0x194`:

| Bit | Meaning | Shown by |
|---|---|---|
| 7 | off the ground (`Ch_EnMapID(chr, 0)`) | `Ch_EnMapID` |
| 8 | no body hit (`Ch_EnBodyHit(chr, 0)`) | `Ch_EnBodyHit` (`0x0082a9f0`), `Ch_GetBodyHit` (`0x0082ab10`) |
| 10 | interpolate rotations | `Ch_EnRotHokan` (`0x0082aae0`) |
| 11 | hide past the clip distance | `Ch_EnClip` (`0x0082aa30`) |
| 0 | recompute the normals every frame | `Ch_SetCalcNorm` (`0x00835750`) |
| 3, 4 | one budgeted normal update (two kinds), unless bit 0 | `Ch_ReserveCalcNorm` (`0x00835710`), `Ch_ReserveCalcNorm2` (`0x00835730`) |
| 13 | effects drawn; also counts as hidden from the calculation pass | `Ch_DrawEnEffect` (`0x0082eb60`), `Ch_GetHideCalc` |
| 16 | cleared with the interpolation frames | `Ch_MotionSetHokan` |
| 18 | shaded by map id | `Ch_IdEnableShade` (`0x008340f0`) |
| 19 | cull | `Ch_DrawSetCull` (`0x008300b0`) |
| 20 | keep the frame rate | `Ch_SetKeepFps` (`0x00839b50`) |
| 22 | cloth | `Ch_ClothSetEnable` (`0x0082a930`) |
| 25 | position set | `Ch_SetPos` |
| 30 | overlap hit | `Ch_SetOverlapHit` (`0x0082b420`) |

At `0x198` there are more flags: bit 2 marks a chocobo (`Ch_SetChocobo`), bit 3 is the VU1 clip mode, bit 5 excludes the character from the OEF2 lookup, and bit 8 turns on texture animation (`Ch_TextureAnimSetEnable`, `0x00830d10`).

### The map id word (`+0x828`)

These fields describe the ground under the character. Each one has its own `Ch_IdGet*` reader; the 13 functions sit in name order at `0x0083d7a0`..`0x0083d940`.

| Bits | Field | Reader |
|---|---|---|
| 0-6 | map id; ids `0x30`-`0x3f` go through `g_ChrIdMapTable` (`0x013019d8`) | `Ch_IdGetMap`, `Ch_IdGetMapB` (raw) |
| 7-8 | encounter | `Ch_IdGetEncount` |
| 9-10 | effect, an index into the current set of `g_ChrIdEffectTable` | `Ch_IdGetEffect`, `Ch_IdGetEffectTable` |
| 11-12 | dictionary | `Ch_IdGetDictionary` |
| 13-14 | water | `Ch_IdGetWater` |
| 15-16 | sound | `Ch_IdGetSound` |
| 17-31 | shade: three 5-bit fields (`Ch_IdShadeCalc`, `0x00834110`, through `Sg_IdGetShade`, `0x0083e980`; both ported) | `Ch_IdGetShade0`-`2` |

`Ch_IdSetDefPid` sets the word to `0xfffe0000`. `Ch_IdGetEffectTable` returns 8 or 9 (bit 29 of `+0x194`) for motion type 2.

The `En` setters set the bit to bit 0 of their argument and return the old
bit. `Ch_EnBodyHit` and `Ch_EnMapID` store the inverse, since their bits
mean "off". The `Set` flag setters return what is left in `eax`: the bit
that changed. `Ch_EnMapID` ends with a tail call to `Ch_IdEnableShade`
and returns its result.

## Model blocks

`g_ChrBlk` (`0x023c8d00`): 40 blocks of `0x12c` bytes
(`Ch_BlkGetHandleFast`, `0x00825ea0`, finds one by its first word).

| Offset | Type | Meaning | Shown by |
|---|---|---|---|
| `0x1c` | `f32` | default collision radius | `Ch_GetThickness` (`0x0082ad70`) |
| `0x20` | `f32` | battle thickness | `Ch_GetThicknessBtl` (`0x0082ad90`) |
| `0x24` | `f32` | default battle camera thickness | `Ch_SetThicknessBtlCam` |
| `0x28` | `f32` | default fixed height | `Ch_SetFixHeight` |
| `0x2c` | `f32` | default camera height | `Ch_SetCamHeight` |
| `0x14` | ptr | texture data: `u16` clut count `+8`, image count `+0xc`; cluts of 8 bytes from the pointer at `+0x18`; images of 16 bytes (`u16` width `+8`, height `+0xa`, data `+0xc`) from the pointer at `+0x1c` | `Ch_TexGetNbClut` (`0x0083cd90`), `Ch_TexGetImage` (`0x0083cd50`) |
| `0x30` | `f32` | data scale | `Ch_GetDataScale` (`0x0082ab60`) |
| `0x38` | `f32` | scale of bind offsets | `Ch_BindToElemDirectOfs` |
| `0x71` | string | name (the object's `+4` points here) | the snapshots |
| `0xb8` | string | directory name | `Ch_GetBlkDirName` |
| `0x124` | ptr, `u32` | resident data and its size | `Ch_EffectGetResidentPtr` (`0x0082a9b0`), `Ch_GetChrRenderInsatanceFromOEF2Content` (`0x00826af0`) |
| `0x6c` | ptr | the character archive it came from; attachment points: 16-byte entries from the archive's `+0x20`, count `+0x24`; entry `u16` id in bits 0..13 and kind in 14..15, `i16` element `+2`, offset x, y, z `+4` | `Ch_ElemGetAttr` (`0x00833a70`), `Ch_ElemGetDirect` (`0x00833bf0`) |

`Ch_ElemGetAttr` returns the kind: 1 writes the offset scaled by
10000 / data scale² × bind scale, 2 does not, 0 (or no such point) gives
the unit vector (0, 0, 0, 1). Its element output is written in every case.

## Loaded data and its pointers

Loaded character data keeps its own address in its first word. When it
moves, the difference to its new address is added to each of its pointers
(`Ch_ArcMapping` `0x008256f0`, `Ch_ModelMapping` `0x00827590`,
`Ch_TexDataMapping` `0x0083cba0`). A second call finds a difference of 0
and changes nothing.

- Archive (`.chr`): self pointer, `u32` entry count `+4`, (pointer, size)
  pairs from `+0x10`. Entry 0 is the model header, 1 the texture data, 4 a
  list of pointers (the pair is the list and its count,
  `Ch_RelElemMapping`), 9 the model parameters (`Ch_CbArcMapping`
  `0x00825ee0` copies them into the model block, defaults when the archive
  has 9 entries or fewer, the second set of floats from parameter version
  `0x1624`), 10 one more table (block `+0x120`).
- Model header: `u16` version `+4`, model count `+6`, element count `+0xa`;
  the model table at `+0x10` has 24-byte entries for versions
  `0x831..0x1313` and 40-byte ones otherwise (`Chr_ModelEntry`
  `0x00828be0`). A model has a mesh table (`u16` count `+6`, 12-byte
  entries at `+0x10`) and a cloth table (`u16` count `+0x18`, `0x24`-byte
  entries at `+0x1c`).
- Texture data from version `0x1029` (`+0x10`) is fixed up, `+0x20` from
  `0x1126`. A clut starting with the colour key `00 ff 00` or `ff 00 00`
  has its first entry cleared.
- `Ch_ClutMapping` (`0x0083ca20`) lays the cluts out after the clut table,
  each at a 128-byte boundary less 0x20, size `(header & 0x7fff) + 2`
  × 16 bytes, copying them when asked.
- `Ch_ModelCreatePacket` (`0x00827280`) gives each model (0x38 bytes at
  the character's `+0x1bc`) the blocks it lacks. Each block is named
  `"<name>:vtex"`, `":norm"` or `":physics"`:
  - `+8`, the vertices: `u16` count `+4`, `i16` triples at `+8`, as
    `f32` x, y, z, 1;
  - `+4`, the normals: count `+0x16`, triples at `+0xc`, normalised;
  - `+0xc`, room for the cloth table's points: 0x30 bytes each. The
    point count is the sum of each entry's `i8` `+0x13` and `i16`
    `+0x1c`, and is kept at `+0x10`.

  Character 0x402e stays hidden during magic 0x295.

## Cloth

A model's cloth entry (`0x24` bytes): vertex index list `+8` (`u16`), `i8`
extra count `+0x13`, `i16` vertex count `+0x1c`. The model's buffer
(`+0xc`) holds per cloth `3 × k` vectors (`k` = count + extra): positions,
velocities, previous positions. `Ch_ClothReset2` (`0x00824670`) copies the
model vertices (`+8`) into positions and previous positions and zeroes the
velocities' x, y, z, then sets flag bit 23 and resets the characters bound
to it; it needs flag bit 21 and stops at a model without a buffer.

The PC port's exceptions are literal tables in the code, by character id,
model, part, motion id and the current map number (`g_CurrentMapNo`,
`0x012fbc40`): `Ch_UseAnchorVertex`, `Ch_UseLargeCollisionRad` (radius
scales 1.03 by default, 1.05, 1.165, 1.2, 1.25), `Ch_ShouldDisableClothForMotion`,
`Ch_ClothSetAtariJimenCheck` (a case byte per id at `0x008248ac`).

### The PC's cloth files (`.cdf`, `.cmf`)

`Ch_LoadClothFileTask` (`0x0082a140`) writes `cdf` and then `cmf` over
the last characters of the model path and loads each
(`ClusterManager_createPreallocateDataList`). `Ch_ClothMeshMapping`
(`0x00826340`) reads the `.cdf` (`Cache_Exist_ClothMeshData`): six `f32`
(a bounding box in HD units, kept × 1000 at `+0x840..+0x854`, with the
reciprocal extents after it), then per PS2 cloth piece a header `u16`
model, `u16` piece, `u16` triangle count, `u16` pad, then that many
32-byte triangles: `u16` vertex indices of the model (each one marked in
the model's per-vertex byte map, model `+0x30`), a `u16` kept in the
piece's second list (piece + 100 × k), and three `f32` (u, v) pairs (PS2
UVs, 1/16 texel; not read here). A model id `0xffff` ends the file. Every
PS2 cloth piece is listed, also those the HD model has no segment for
(c001 model 1 pieces 0, 3, 4). No character ships a `.cmf` (baked
bindings).

A segment draws a piece when `PMeshSegment` `+0x20`/`+0x24` (model, piece;
-1 none) name it (`graphicCharacterInstanceGetClothVertexBuffers`
`0x0063a810`). `Ch_PreCalcActiveClothMesh` (`0x00827e90`) binds each of
its vertices to the simulated triangle it lies in (`InsideTriangle`
`0x00828c10`, bind pose × 1000 against the PS2 model vertices), unless
the vertex colour keeps it skinned: with r, g and b all at most 0.3 the
vertex follows the skin; larger values pick the binding mode. The HD
cloth segments use (255, 255, 255) or (255, 0, 0) on free vertices and
(26, 26, 26) on anchored ones (ffx-demaster `docs/models.md`, "Cloth").


## Module settings

The `Ch` module keeps its own settings in globals (`ffx_chara::sys`):

- drawing switches: `g_ChrDrawDisableSpec`, `g_ChrDrawFourceSpec` and `g_ChrDrawParaEdit` (`0x013007db`..`d9`, `Ch_DrawSet*`/`Ch_DrawGet*`);
- the shadow: mode `0x013008bd`, colour r, g, b at `0x013008bf`, and direction (two words) at `0x013008c4`;
- the camera and screen matrices it draws with (64 bytes each at `0x013008f8` and `0x01300938`, `Ch_SysSet*Matrix`);
- cloth editing values (`0x012fbc54`..`5c`);
- the mouth threshold, clamped to 0..200 (`Ch_GuiSetMouthTh`);
- the effect packet writer (`0x01303658`). `Ch_EffectPacketStoreEND` writes the 16-byte end tag `0xff00` there, or, while the pointer is 0, only adds 16 to the size at `0x0130365c`.

## Floats

These functions move floats through the x87. `fld m32` quiets a signalling
NaN, so a float the original stores after loading it (`Ch_SetDir`,
`Ch_GetPos`, ...) comes out quiet. Copies made with integer moves keep
every bit: `Ch_SetPos` copies the position that way. When two quiet NaNs
are added (`Ch_AddGrav`), the result is the one with the larger
significand, or the positive one if the significands are equal.
A quiet NaN wins over a signalling one taken straight from memory
(`fmul m32`), and an invalid operation (∞ − ∞, 0 × ∞) gives the negative
"indefinite" NaN. Chains of operations on the stack (`Ch_DrawCalcShade`'s
three-way products, `Ch_BindToElemDirectOfs`'s × 100 × scale,
`Ch_GetVel`'s differences) are rounded to `double` at every step and to
`f32` only when stored. `ffx_chara::x87::Reg` computes the same way with
`f64` and the x87's NaN rules. Its tests check single operations and
chains against the host CPU's x87 unit, about 50,000 cases, signalling
NaNs included. The oracle's emulator does not quiet signalling NaNs on
`fld`, so the parity tests leave signalling NaNs out.

`Ch_GetRot` wraps the angle with `Chr_WrapAngle` (`0x0082ba70`). It adds or
subtracts 6.283184 until the angle is within ±3.141592, using the `double`
widenings of those `f32` values, and stores each step as `f32`. Rust's
`f64` arithmetic followed by `as f32` gives the same result bit for bit.
The loop never ends for infinities, in the original and in Rust.

## Matrices and elements

An element (`0x160` bytes, from `+0x328`) has its data pointer at `+0`,
a local matrix at `+8` and a global matrix at `+0x88` (`Ch_ElemInit`
`0x00828b60` sets both to the identity, `+0xe4..+0x100` to 0 and
`+0x104..+0x110` to 1.0). The root global and light matrices are the
character's `+0x1d0` and `+0x210` (`Ch_GetRootGlobalMatrix` `0x00833f80`).
A model's base matrix is the global matrix of the element its data starts
with (`u16` at the model data's `+0`, `Ch_ModelGetBaseMatrix`
`0x00830500`); `Ch_ModelGetMatrix` (`0x00830550`) multiplies it by the
root global matrix with `cdc_Mul4x4`.

Measured (ffx-remix M10; the Besaid arrival, `bsil0000` from save
`ffx_002`; two runs of 740 ticks, read at `MsScreenProcess`): the root
global matrix `+0x1d0` holds the rotation times 0.001 (each row times
1000 has length 1) with the field position (`+0xc`) as its translation
(the 1000 of `Ch_DrawMainForSituation`, below). The elements' global
matrices `+0x88` are relative to the character (its own space, not the
field's); the root matrix places them. Their rotation rows mostly have
length 1. Some elements carry a scale (rows more than 1% off): 0.8 to
1.2 for `c101`'s 105 to 107, 109, 113 to 117, 121 and 122 and for
`c001`'s 89 and 102, and 0.5 to 1.8 for `c101`'s 138, 142, 170 and 183.
Some have a zero rotation part (`c001`'s 118 to 135). ffx-remix posed
the HD models with them (the model's skeleton node `SkeletonNode_i`
takes element `i` as it is) and placed them with the root rotation times
1000; its offline pictures matched the game's shots of the same ticks by
eye (five shots of one run). A slot can change characters within a
scene: the arrival's slot 0 went from `0x0001` (`c001`) to `0x0065`
(`c101`), at tick 62 in one run and 73 in the other.

## Motion frames and sequences

`Ch_MotionLoopFrame` (`0x008392d0`) keeps the frame (`i32` `+0x740`, 8.8
fixed) inside the motion: past the end (`+0x71c`) it steps back by the
loop length (`+0x720`) while loops are left (`i16` `+0x724`, 0 loops for
ever), restarts at `+0x718` when the length is 0, or stops on the end's
last whole frame when the loops run out (clearing `+0x728` and `+0x754`).

The sequence table `g_ChrSequences` (`0x01300a08`, 100 entries of `0x14`
bytes) is relinked by `Ch_SeqLink` (`0x00838170`): one chain per key
(`u16` `+4`) through `+8`, the chains' first entries through `+0xc` from
`g_ChrSequenceHead` (`0x013011d8`).

## Texture animation

`g_ChrTexAnim` (`0x02310000`): 64 entries of `0x30` bytes: source `+0`,
work buffer `+4`, format byte `+0xd` (0 old, 1 new). When the source
moves, `chr_texanim_memory_move` (`0x0077c900`) copies the word at `+0xc`
of each of its 16-byte records into the work buffer from `+0x5b8` (old)
or `+0xd28` (new). `Ch_TextureSetTex0` (`0x008313f0`) writes each
texture-list entry's base pointers: the image texture's (index `+2`) into
bits 0..13 of `+0x14`, the clut texture's (index `+0`) into bits 7..20 of
`+0x18`.

`chr_texanim_set` (`0x0077c950`) takes the first free entry, copying
the source's kind (byte `+0`, to `+0xc`) and form (byte `+2`, to `+0xd`).
A kind-2 source gets a work buffer from `chr_texanim_set_old`
(`0x007803f0`, `0x5d8` bytes) or `chr_texanim_set_new` (`0x0077f9c0`,
`0xd4c` bytes). The buffer holds slots of `0x34` bytes (old: 16 from
`+0x40c`; new: 32 from `+0x0`), one mode byte per slot (old `+0x5a8`,
new `+0xd08`, copied from the source's `+0xc`; -1 for none), the handle
(old `+0x404`, new `+4`) and the slot count (old `+0x400`, new `+0`).
The source's per-slot records (`[src+8]`, `0x90` bytes old, `0x110`
new) give a mode-2 slot its scroll position. `chr_texanim_main_sub_old`
(`0x0077ff00`) steps an old buffer each frame. A slot plays the frame
list of the entry's track (`+8`): frames of 4 bytes, holding an `i16`
time, a rectangle index (byte 2) and a random-wait scale (byte 3). Its
play modes are 1 and 4 (hold), 2 (blink: wait 60-149 frames, then 4
steps per frame) and 3 (step and scroll). For mode 3 it then copies the
frame's rectangle (12 bytes from `[src+4]`: x, y, width, height, pixel
offset) into the destination, row by row with `memcpy`.

The new-format source (texture data `+0x20`; kind 2, form 1 in every
`.chr` the demaster surveyed; ffx-demaster `docs/models.md`, "Texture
animation", `docs/research/ps2-textures.md`), as `chr_texanim_set_new`
(`0x0077f9c0`) and `chr_texanim_main_sub_new` (`0x0077f360`) read it:

| Offset | Meaning |
|---|---|
| `+0` `u8` | kind (2 = patch animation) |
| `+1` `u8` | entry count |
| `+2` `u8` | form (0 old, 1 new) |
| `+4` `u32` | patch table: 12 bytes each, `u16` destination image width and height, `u16` w, h, `u32` offset of a 32-byte upload header with the 8-bit texels after it (drawn through the image's CLUT 0) |
| `+8` `u32` | entries, `0x110` bytes each: `u16` x, y of the destination `+8`, `u16` image `+0xc`, then 32 states × (`u32` frame list offset, `i16` count) from `+0x10` |
| `+0xc` | 32 bytes: per entry its mode (`0xff` unused), copied to work `+0xd08` |

A frame is `i16` duration, `u8` patch, `u8` random extra. Mode 0 blinks (a
random 60-150 update wait, then the list), 4 plays the list, 1 holds a
frame, 3 scrolls. States 1, 2 and 21 always run, plus the one the event
sets (`tex_anim_no_set_new`). `setLoadImageTagsTexAnim` (`0x0077eff0`)
builds the PSMT8 transfer of a patch into the image;
`chr_texanim_draw_sub_new` (`0x0077f210`) checks it stays inside ("tex
anim area over").

On the PC the step also calls `TextureAnimationManager_setAnimatedMaterialsChar`
(`0x006b3060`) with group `G<entry>`, animation `Animation<state>` and the
frame index. The `textureanimationchar.ags.phyre` record
(`PSpriteAnimationInfoChar`) of those names gives `m_subTextureIDs[frame]`,
a texture: the n-th cluster of the character's asset list, which lists
the animation textures before `textureAnimationChar`
(`PhyreScene_doOnCharacterAllAssetsLoaded` `0x0065a760` passes that
range). Its floats `mUStart, mVStart, mUEnd, mVEnd` hold the PS2 patch
rectangle as `w/W, h/H, x/W, y/H` (W, H the image size). The material that
shows it is named `Mat_CTA_G0G1…` after the groups it takes; it stacks the
groups set in layers 1-3 (1-6 with `CHR_TEXTURE_ANIMATION_UV`, newest on
top) and the pixel shader replaces the colour inside each layer's
rectangle: `t = (uv - T) / S`, drawn where `0.001 <= t <= 0.999`, with v up
from the image's bottom row. Two shader variants: `_GAME` takes
`AnimTranslate_GAME<n> = (mUEnd, mVEnd)` and `AnimScale_GAME<n> = (mUStart,
mVStart)` from the record; `_MAYA` (c001-c003, c005-c008, c043) has one
fixed rectangle per layer in the material and ignores the record's
floats (ffx-demaster `docs/models.md`).

## Motion sequences

A sequence (`+0x714`) is a byte code `Ch_GetSeqFrame` (`0x00836af0`)
walks to count its frames: 0 and 5 end it; 1 is a segment of 9 bytes
(motion `+1` of the group at `+0x710`, repeat count `+3`, frames between
the key indices `+5` and `+7`); 2 skips a byte; 3 adds the value at `+1`
(3 bytes); 4 jumps through the list at `+0x70c`; 6 skips 3 bytes. Its
16-bit operands are read by `ReadU16Unaligned` (`0x00838220`), which
sign-extends both bytes before or-ing them: a low byte from `0x80` makes
the high one `0xff`. `Ch_SetSequence` (`0x00837ac0`) starts motion
`character << 16 | number` from the sequence table (`g_ChrSequences`,
chains by character from `Ch_SeqLink`); another character's motion must
be in the model block's lists and is kept at `+0x758`.
`Ch_SetMotionSeq` (`0x00839b80`) plays a clip from a start to an end
frame (×256; a negative end means the clip's length). One case is
special: Tidus (`"c001"`) on map 0x7a, playing 0x3000 to -0x100, stops
at 0x2800.

## Element matrices

`Ch_CalcElement` (`0x00832760`) builds the character's world matrix
`+0x1d0` from one of three sources: a parent's element (`+0x19c`,
`+0x1a0`), a given matrix (`+0x10c`), or its own scale, pitch, facing and
position. The scaled copy goes to `+0x210`. It then builds each
element's matrix (`+0x88` of the 0x160-byte elements at `+0x328`). The
skin matrices (the element relative to its bone's inverse) go into a
512-entry stack buffer at each element's index `+4`, and
`graphicCharacterInstanceSetSkeletonMatrices` (`0x0063be60`) uploads that
buffer to both instances. The upload reads only the entries its meshes'
remap tables name. On the snapshots' characters those are all computed
entries; the other entries are undefined in the original, and the oracle
check `ch_calc_element_reads_only_computed_matrices` confirms this with a
poisoned stack.

`Ch_MemMove` (`0x00826fa0`) is one function; the dump splits it at the
early return. It moves each active character's heap blocks
(`Sg_MemMoveWork`) and stops after the first character that moved
anything.

## Character numbers

`Ch_GetCharID` (`0x0082a080`) reads a character's number from its table
entry's file name (`g_CdLib_98(Chr_TableIndex(id))`). It takes the 7th to
5th characters from the end and passes them to `atoi`. The original
keeps them in a 4-byte stack slot made by `push ecx`, so the slot's 4th
byte is whatever the caller left in ECX. So is any byte that a name
shorter than 7 characters does not reach. That input is undefined. The
port treats those bytes as NUL, which gives the original's result
whenever the byte is not a digit; the oracle test enters the original
with ECX = `"AAAA"`.

## The id map walk

`Sg_IdWalk` (`0x0083d960`) finds the id-map triangle under a position. A
triangle is 16 bytes in the table at `0x01301a84`: three `i16` vertex
indices, then the neighbours across edges 0-1, 1-2 and 2-0. Vertices are
8 bytes in the table at `0x01301a8c`: `i16` x, y, z. Each edge test
evaluates the edge's line at the position. The coefficients are the
edge's integer coordinates and their integer products, each rounded to
`f32`.

The first edge the position lies beyond wins. A later edge overrides an
earlier one only when the character's previous position (`+0x3c`,
`+0x44`) is also beyond it. Inside all three edges, the triangle's
vertices go to `+0x8c`..`+0xb4` and its index to `+0x824`. Beyond an
edge, the walk moves to the neighbour (depth `0x01301ae8`, at most 10)
unless one of these stops it:

- there is no neighbour;
- `Sg_IdMatchChr` rejects the neighbour for this character;
- the neighbour's floor is too low: `Sg_IdGetHeight` is below the
  character's `+0x40` less the slack `0x01301af4`. The floor is checked
  only for a sloped normal.

When the walk stops, it records the crossed edge (`0x01301aec`,
`0x01301af0`).

## Head, mouth and automatic motion

`Ch_Neck` (`0x008348d0`) turns the head (option point 9's element)
toward a target. The target depends on the mode byte `+0x40d`: 1 the
camera, 2 another character's point 9 (handle `+0x4c0`), 3 a position at
`+0x4c0`, 4 fixed angles straight into the matrix `+0x46c`. The weight
`+0x418` eases toward `+0x414` by `1 / +0x420`. The yaw is measured in
degrees against the shoulders (points 0x13, 0x14) and the facing
`+0x158`. It is clamped to a window from `+0x424` and `+0x428`; flag
`+0x4d0` remembers the side and shifts the lower edge by ±15. `+0x41c`
follows the clamped yaw: by a third, or by a quarter past 90 degrees.
`Ch_NeckInit` (`0x00834e60`) sets the limits (20, 160) and the rest
matrix `+0x42c`.

`Ch_Mouth` (`0x008344f0`) moves the opening `+0x4dc` toward a goal
`+0x4de` (in tenths of a degree), by a step `+0x4e3`, while a counter
`+0x4e1` runs. The global mode byte `0x013008d9` selects the behaviour: 1
a scripted shape, 2 shut, 3 talking, and other values follow the
character's own byte `+0x4d4`. While talking, the goal is a random value
from 0 to 69.

`Ch_AutoMotion` (`0x00835d30`) runs the stand/walk/run motions once the
script releases a character. Standing plays `+0x79c`; moving plays
`+0x7a8` above the water height + 1 and `+0x7a4` below it.

## Texture layout

`Ch_TextureReMapping` (`0x0083cdd0`) lays the texture data out in the
buffer at `+0x6dc`: the texture list (32 bytes per entry, count at
`+0x6f4`, GS registers from `Ch_TextureSetRegs`), then the clut textures
(64 bytes, `+0x6e4`, `Ch_TextureSetClut`), then the image textures (64
bytes, `+0x6ec`, `Ch_TextureSetImage`). A texture's base is `u16` `+0x38`
and `+0x3a`, copied into bits 0..13 of `+0x24` (`Sg_GsloadSetBase`).
List entries take the image base into bits 0..13 of `+0x14` and the clut
base into bits 7..20 of `+0x18` (`Ch_TextureSetTex0`).

### Texture data (archive entry 1)

The file side of the texture data (ffx-demaster
`docs/research/ps2-textures.md`; offsets relative to the block, relocated
by `Ch_TexDataMapping` `0x0083cba0`):

| Offset | Type | Meaning | Read by |
|---|---|---|---|
| `+0x00` | `u32` | self pointer (0 in the file) | `Ch_TexDataMapping` |
| `+0x04` | `u16` | texture-list count | `Ch_TextureReMapping` (`0x0083cdd0`) |
| `+0x08` | `u16` | CLUT count | `Ch_TexGetNbClut` (`0x0083cd90`) |
| `+0x0c` | `u16` | image count | `Ch_TexGetNbImage` (`0x0083cdb0`) |
| `+0x10` | `u16` | version (`0x1126` in every file) | `Ch_TexDataMapping` |
| `+0x14` | `u32` | texture list, 16 bytes per entry | `Ch_TextureSetRegs` (`0x0083d250`) |
| `+0x18` | `u32` | CLUT table, 8 bytes per entry | `Ch_TexGetClut` (`0x0083cd20`) |
| `+0x1c` | `u32` | image table, 16 bytes per entry | `Ch_TexGetImage` (`0x0083cd50`) |
| `+0x20` | `u32` | texture animation (0 = none) | `Ch_TextureReMapping` (to `+0x6fc`), `chr_texanim_set` |

A texture-list entry is a material, an image read through a CLUT
(`Ch_TextureSetRegs` fills the 32-byte runtime entry from it):

| Offset | Type | Meaning |
|---|---|---|
| `+0` | `u8` | image index (runtime `+2`, the TEX0 base) |
| `+1` | `u8` | CLUT index (runtime `+0`, the CLUT base) |
| `+2` | `u16` | PSM (`0x13` PSMT8 in all 857 files), into TEX0 bits 20..25 |
| `+4`, `+6` | `u16` | width, height (TW, TH from the next power of two; TBW = width / 64) |
| `+8`, `+9` | `u8` | runtime `+0x11`, `+0x12`: the `ALPHA_2` register (below); `44 00` in every file |

The high TEX0 word gets `& 0x801fff9f | 0x10`, which leaves CPSM, CSM
and CSA 0: a PSMCT32 palette in CSM1 order (`docs/formats/ps2-textures.md`).

A CLUT entry is a `u32` (`0x400` in every file; `Ch_TextureSetClut`
`0x0083d1e0` reserves one GS block when it is `<= 0x10`, else four) and
the `u32` data offset. An image entry is `u16` PSM, `u16` size in GS
blocks (runtime `+0x36`; 256×256 at 8 bits is `0x100`), `u32` 0, `u16`
width and height at `+8`, and the `u32` data offset at `+0xc`
(`Ch_TextureSetImage` `0x0083d220`; transfer width `+8 >> 6`). Both data
blocks start with a 32-byte upload header: 12 zero bytes, VIF `DIRECT`
(`0x50000041`; the PC patches it to `0x51`, `DIRECTHL`), and a GIF tag with
`FLG=IMAGE`, `EOP` and `NLOOP` 0x40 for a palette (256 entries) or
`w × h / 16` for an image. Image texels are linear PSMT8 (row-major; the
GS swizzles them on upload). One image is usually shared by several list
entries with different CLUTs (c001: one 256×256 image, CLUT 0 for face,
skin and hair, CLUT 1 for clothes and metal).

All 857 `.chr` files the PC ships (`FFX_Data.vbf`) differ from the PS2
disc's only inside the image texels and the texture-animation patch
texels: CLUTs and upload headers are kept, the texels zeroed (the test
files `k999`, `c999` and the patches of 17 monsters and NPCs, m003, m030,
m105, n238, n356, ..., kept theirs).

## Mesh groups and the character packet (`0x008423c0`)

A model's mesh group (12 bytes, ffx-demaster `docs/models.md`, as the
packet builder `0x008423c0` walks them; caller `0x0082e610`):

| Offset | Type | Meaning |
|---|---|---|
| `+0` | `u8` | flags: bit 0 strips (else triangles), bit 1 textured, bits 2 and 3 pick the draw state (index into the 3-byte table `0x00b5a6b0`) with the cull byte (`Ch_PacketSetCull`), the specular byte (`Ch_PacketSetSpecular`) and the shadow byte (+4) |
| `+1` | `u8` | MAT: index into the material table (`Ch_PacketSetName`'s pointer); `0x008437f0` sends its RGB (bytes `+4..+6`) × 2 × the effect colour and the effect alpha |
| `+2` | `u8` | texture-list index; `0x008438b0` sends that entry's TEX0 when it changes |
| `+4` | `u16` | count (strips or triangles) |
| `+8` | `u32` | data |

Points are four `u16`: vertex, normal, u, v (UVs in 1/16 texel, so a
256-texel image spans 0..4096). A vertex or normal with bit 15 set repeats
an earlier halfword: with `k = value & 0x7fff` and `p` the point's first
halfword, the vertex is `p[-(k × 4 / 3)]` and the normal `p[-1 - (k × 4 /
3)]` (`Ch_EffectPacketCreateMain` `0x00842c80`,
`Ch_EffectPacketStoreStrip` `0x00844370`). In a strip, point `k >= 2`
makes the triangle (k-2, k-1, k) for odd `k` and (k-1, k-2, k) for even
`k`; a triangle group holds 24 bytes per triangle (three points).

The GIF tag's `PRIM` of every group has ABE and context 2 (`0x240`), plus
TME and FST (`0x110`) for a textured group while textures are on
(`Ch_PacketSetTexture`), FGE when `0x00842ff0` says so, and the
triangle or strip type with Gouraud shading (`0xb` / `0xc`). So every
character polygon is alpha blended. `0x008438b0` writes, per texture:
`TEXFLUSH`, `TEX0_2` (the runtime entry's `+0x14`), and `ALPHA_2`
(register `0x43`) with the low word from the runtime `+0x11` and the high
word from `+0x12`: the list entry's `+8`/`+9`, `0x44` and 0 in every file,
which is (Cs − Cd) × As + Cd with the texture's alpha. A texel with GS
alpha under `0x80` is see-through (hair, fur and feather fringes). The PC
draws characters with PhyreEngine materials instead; there the alpha comes
from the material's effect variant (`ALPHA_ENABLED`, `UATFOPQ`,
`AlphaThreshold`; ffx-demaster `docs/models.md`, "Alpha", on its branch
of 2026-11-02).

The draw state table `g_ChrPacketDrawModes` (`0x00b5a6b0`) holds 8 × 3
bytes (MSCAL entry, ?, matrix-set kind): `04 00 00`, `06 00 00`,
`08 01 01`, `0a 01 01`, `0c 00 00`, `0e 00 00`, `10 00 02`, `12 00 02`.
The builder's caller `Ch_DrawPs2Packets_0082e610` is the PS2 character
draw kept in FFX.exe with its packets unused (shadow mode,
`Sg_LightSetChr`, `Ch_DrawCalcShade` -> `Ch_EffectSetColor`, per model the
VU1 matrices, then the builder); the VU1 program the MSCAL entries call is
in `docs/formats/ps2-elf.md`.

The PC's material switches (ffx-demaster `docs/models.md`, "Alpha";
`PShaderPassD3D11.m_state` and DXBC reflection):

- `UATFOPQ`: the `Opaque` pass with alpha to coverage on and the alpha
  test (`AlphaThreshold` read by 39 of 43 pixel shaders).
- `ALPHA_ENABLED`: only a `Transparent` pass (src alpha / inverse src
  alpha, depth write, `LESS_EQUAL`; `AlphaThreshold` read by all).
- neither: opaque, alpha ignored (`AlphaThreshold` unused in 41 of 43).

Of the 220 variants 86 are `ALPHA_ENABLED` and 35 `UATFOPQ`.
`PMaterial.m_effectVariant` is a user fixup: a big-endian index into the
`PAssetReferenceImport` list (`PS3Data/Shaders/PhyreChrLitShader.fx#<MD5>`);
a variant that a model newly uses must be in the character's `.ahwin32`
D3D11 list (`Ch_LoadHdRenderData` `0x0082a1c0`). In battle,
`ALPHA_ENABLED` segments that lie over the same character's opaque body
are not drawn (Lulu's necklace, Kimahri's leg wraps, Tidus's wristband);
the same segments draw in cut scenes and with `UATFOPQ`. The cause is not
known.

## Effect packets

`Ch_EffectPacketStorePoly` (`0x00845820`) writes 28 bytes per triangle
after a 32-byte block tag (`0x1000`, the texture's registers or
`0x8044`), opened again whenever the texture index, MAT, PRG or the
fourth state word changes. Texture coordinates are `v × 4096 / (size ×
16)`, truncated (`fistp` in chop mode). While the packet pointer is 0 it
only measures: size, triangle count, largest x and y.

## Debug text and generated names

- `Ch_FunctionHistory` (`0x00833770`) and `Ch_CheckRadian` (`0x00836790`)
  `sprintf` into the debug line at `0x023c4000`: `"%s:%s:%d\n"` (character
  name at chr+4, source file, line) for each active character with no
  models, and `"radian value error =%f"` for an angle outside
  `[0x00b599e0, 0x00b599f0)` or NaN. `%f` follows MSVCR110: 17 significant
  digits, then zeros (see `ffx_chara::msvc::fmt_f`).
- `ch_create_name` (`0x00838100`) formats `"%c%03d"` into `0x01300a00`: the
  letter is the database type (c, m, n, s, w, f, k; `-` otherwise) and the
  number is the id within it. Id 0 returns the constant at `0x00b59a08`.
- `Ch_AnalysisChrNum` (`0x00829e60`) turns `.../mdl/xxx.chr` into the
  `.../xxx.ah` name: it copies up to the dot, appends `ah`, and drops the
  first `mdl/`.

## Moving motion groups

`Ch_MemMove` (`0x00826fa0`, MoveChrMemory) compacts the character heap
when `0x012fbda0` is set. Its event step `Ch_MemMoveEventMgrp`
(`0x00837050`) moves the first loaded motion group (table `0x013011e0`,
50 x 12 bytes: group, `i16` state `+4` == 1) that `Sg_MemMoveWork`
(`0x0083c350`) can move lower, then `Ch_MgrpMove(new, old)` (`0x008370c0`)
fixes what pointed at it: `SG_RelocateGroup(new)`, the table entry, every
used sequence (`0x01300a08`, 100 x `0x14`) whose group `+0x10` is `old`
(data `+0` and group by `new - old`), and every active character whose
`+0x7e8` is `old` (`Ch_MoveMgrp`). Names from the Switch build's call graph.

## Allocation and disposal (battle round 35)

- `Ch_Allocate(key)` (`0x00824f90`, `ffx_chara::handle`): first free slot
  (`+2` clear), block `Ch_BlkGetHandle(key)` (missing: 0 back when
  `g_ChrAllocateErr` was set, else block 1 with `+0xf8` set), cleared,
  mapped (`Ch_CbMapping`) and given its defaults; `id >> 12` picks the
  shadow mode (0-3: 1, 4-6: 0, 15: 3 and flag `0x40`). Id tables:
  `g_ChrFlag2Ids` (`0x00c494d8`, FLAGS bit 2) and `g_ChrIdFloats`
  (`0x00c495b0`, `+0x814`/`+0x818`). `+0x87c` marks Rikku (`0x3004`) in
  magic file `0x113`.
- `ChN_DataDispose(key, check)` (`0x00824f00`): -1 while a character of
  that id is in use, else the block and its cache entry go.
- `Ch_Dispose(chr)` (`0x008266f0`): frees what the character holds (the
  models' three buffers, the `+0x874` table, texture animation, five
  blocks) and destroys its render and shadow instances; EAX is the cloth
  vertex buffers' destroy result.

## Shades, models and reads (chara-main-map round 1)

- `Ch_SetShadeIDRelation(chr, v, frames)` (`0x0082b7a0`, `ffx_chara::chr`):
  `Ch_SetShadeID` on every other active character bound to `chr`
  (`+0x19c`). Like every walk over `g_ChrArray`, the array and its count
  are read again after each call, as the walk's end.
- `Ch_IdShadeCalc(chr)` (`0x00834110`, `ffx_chara::id`): with id shades on
  and flag bit 18 (`+0x194`), the shade id is the length of a vector made
  from the distances `+0x3c` to `+0x8c`, `+0x9c`, `+0xac`: normalized,
  each component `1 - c`, normalized again, then times the id word's
  shades 0..2 over 31; set over 3 frames. With id shades off, the
  renderer's debug shade coefficient of `chr`'s instance, once per bound
  character and once more (not while `g_ChrDrawDetached`).
  `Sg_VectorGetLength` takes one word: the 3 pushed after it is unused.
- `Ch_ModelSetAlpha(chr, i, a)` (`0x0082ae30`): model `i`'s `+0x14` and
  the renderer's mesh segment alpha; `i` past the header's count prints
  the `VIRTUOS ERROR` line to `stderr`.
- `0x008338d0` (`Ch_SoundFootEffect_008338d0`, `ffx_chara::attach`): the
  footstep events of `Ch_Sound` put the walk effect at optional position
  8 when position 7's y is below 8's, else at 7 (also for a NaN).
- `Ch_RomReadEventMGRP(file, x)` (`0x0082a6e0`, `ffx_chara::sys`): event
  motion group `file`; a size of 0 falls back to the Eternal Calm group
  `file / 18` (signed). The buffer is named `"[EventMotionGroup x]"`; the
  read request carries `x` at `+0xc` and the buffer at `+0x10`
  (`Ch_RomReadEventMGRPcb` hands both to `ChN_AddEventMgrp`).
- `Chb_AttachMotionTBL_OLD(blk)` (`0x00837fa0`): the host-PC viewer's
  `inf/sysmot.tbl` (`+0x108`) and `inf/relation.tbl` (`+0x10c`); each
  table's first word is the address it was built for, and the tables
  relocate their pointers (system: `+0xc`, `+0x14`, `+0x1c`, `+0x24`;
  relation: `[+4]` words from `+0x10`) by the difference.
- `Ch_DisposeAll()` (`0x008268e0`) and `Ch_Allocate2(key, v)`
  (`0x00825650`, `v` into `+0x190`).
- `Apply_Envelope(chr, all)` (`0x00832110`, `ffx_chara::draw`): skinning.
  The header's skin entries (`u16` count `+8`, 12 bytes each from `+0x14`:
  element, model, base element, kind, weight lists at `+8`) each run a
  kernel with `inverse(base) x element` (element matrices at `+0x88`);
  the weighted kernels for a non-zero kind. A model with `+0x34` set keeps
  its kernel results in a cache (`+0x874`, a word per entry) and runs the
  fast kernels after the first frame; models are skipped unless shown
  (`+0x2c`) or `all`. In magic file `0x1b6` (characters `0x1114`,
  `0x1064`) and `0x1b5` (`0x1063`) everything is skinned, the normals too
  (entries from `+0x24`), and every model's normals are normalized.
  Recompute requests: `g_ChrGuiCalcNormal` (`0x013009e0`), flag bits 0, 4,
  and bit 3 while `g_ChrCalcNormalOnce` (`0x013009e4`) counts down.
- `Ch_DrawMainForSituation()` (`0x0082fe00`): per shown character its
  `+0x1d0` matrix scaled by 1000 is the world matrix; whole when every
  model is skinned, else unskinned models get it times their base
  element's matrix (model id by model id).
- `Ch_EffectPacketCreateMain(p)` (`0x00842c80`): an effect packet from a
  model's groups (12 bytes: word, MAT and state bytes, `u16` count, data
  at `+8`): strips (bit 0 of the word byte) or triangles of 24 bytes whose
  indices 0, 1 of each vertex pair may point back (bit 15: `(i & 0x7fff) *
  4 / 3` halfwords before the pair).
- `Ch_WorkRestore(chr, saved)` (`0x00828620`): which fields a character
  takes back from a saved copy (the sym line lists them); the parent is
  looked up again from the bound id (`AtelGetEventCharHandleFromID`).
- `Ch_PLightCalc(chr)` (`0x008305a0`): point lights of 0x2c bytes
  (position, colour `+0x10`, radius `+0x20`, exponent `+0x24`, enabled
  `+0x28`, slot `i8` `+0x29`) add `att * colour` and `att * direction`
  into the character's four light slots (`+0x384` colours, `+0x3c4`
  directions, w = 1), `att = 1 - (dist / radius) ^ exponent`. The
  renderer gets only the slot of the last light inside its radius, and
  only when its position is not NaN and its exponent and attenuation stay
  under `0x00c49714` (999999); else white light from above. Tunables in
  `.data`: `0x00c496ec` radius scale, `0x00c496f0` colour scale,
  `0x00c49700` largest colour component, `0x00c49704` direction scale.
