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
| `0x01c` | `f32` ×4 | the position again, copied word by word after each set | `Ch_SetPos` (`0x0082b440`) |
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
| `0x180` | `u8` | hide bits: 1 event, 2 effect, 0x10 battle | `Chr_SetHideFlags` (`0x0082ba30`) |
| `0x181` | `u8` | hidden from the calculation pass | `Ch_SetHideCalc` (`0x0082b280`) |
| `0x182` | `i8` | ground mode | `Ch_SetGravMode` (`0x0082b240`) |
| `0x183` | `i8` | shadow mode | `Ch_SetShadowMode` (`0x0082b820`) |
| `0x185` | `i8` | field mode (also the default motion's slot) | `Ch_SetFieldMode` (`0x0082b1e0`) |
| `0x186` | `u8` | motion type | `Ch_SetMotionType` (`0x0082b3a0`) |
| `0x18c` | `u32` | battle character number | `Ch_SetBattleChrNum` (`0x0082b0d0`) |
| `0x194` | `u32` | flags (below) | |
| `0x198` | `u32` | more flags: bit 2 chocobo, bit 3 VU1 clip mode, bit 5 skipped by the OEF2 lookup | `Ch_SetChocobo` (`0x0082b130`), `Ch_Vu1SetClipmode` (`0x0082b970`) |
| `0x19c` | ptr | the character whose shade this one follows | `Ch_SetShadeID`'s helper (`0x0082b7a0`) |
| `0x1bc` | ptr | models, `0x38` bytes each: `+4` normals, `+8` vertices, `+0x14` alpha, `+0x18` hidden | `Ch_ModelGetHide` (`0x0082add0`), `Ch_ModelGetNromPtr` (`0x0082adf0`) |
| `0x1c0` | ptr | header, `u16` model count at `+6` | `Ch_ModelSetHide` (`0x0082aea0`) |
| `0x1c4` | ptr | model block in `g_ChrBlk` (below) | `Ch_GetBlkDirName` (`0x00826a60`) |
| `0x330` | fade ×3 | shade colour r, g, b | `Ch_SetShadeCol` (`0x0082b5d0`) |
| `0x354` | fade | alpha; 0 also hides from the calculation pass | `Ch_SetTransparent` (`0x0082b900`), `Ch_GetHideCalc` (`0x0082abe0`) |
| `0x360` | fade | shade | `Ch_SetShade` (`0x0082b5b0`) |
| `0x378` | fade | specular alpha | `Ch_SetSpecularAlpha` (`0x0082b860`) |
| `0x408` | `u32` | effect level | `Ch_EffectSetEffectLevel` (`0x0082a9d0`) |
| `0x4e4` | `f32` | collision radius; 0 or less takes the block's `+0x1c` | `Ch_SetThickness` (`0x0082b880`) |
| `0x4ec` | `f32` | battle camera thickness; negative takes the block's `+0x24` | `Ch_SetThicknessBtlCam` (`0x0082b8c0`) |
| `0x4f0` | `f32` | fixed height; negative takes the block's `+0x28` | `Ch_SetFixHeight` (`0x0082b200`) |
| `0x4f4` | `f32` | camera height; negative takes the block's `+0x2c` | `Ch_SetCamHeight` (`0x0082b0f0`) |
| `0x4f8` | `f32` | hit height | `Ch_SetHitHeight` (`0x0082b320`) |
| `0x500` | `f32` | weight when characters push each other | `Ch_SetWeight` (`0x0082b940`) |
| `0x504` | `f32` | gravity | `Ch_AddGrav` (`0x0082a890`), `Ch_GetGrav` (`0x0082abb0`) |
| `0x51c` | `f32` | (`hiju`) | `Ch_SetHiju` (`0x0082b340`) |
| `0x700` | `u32` | texture animation id | `Ch_TextureGetAnimId` (`0x0082b960`) |
| `0x728` | `u16` | cleared for each new motion | `Ch_ResetMotion` (`0x0082b080`) |
| `0x730` | `u32` | 1 for each new motion | `Ch_ResetMotion` |
| `0x736` | `u8` | a scripted motion plays | `Ch_ResetMotion` |
| `0x73a` | `u16` | interpolation frames into the next clip; 0 marks the object dirty | `Ch_MotionSetHokan` (`0x0082afb0`) |
| `0x73c` | `u16` | interpolation frames of the next motion; -1 is stored as 0, 0 as 1 | `Ch_MotionSetNextHokan` (`0x0082b010`) |
| `0x750` | `i16` | motion speed, 8.8 fixed | `Ch_SetMotionSpeed` (`0x0082b380`) |
| `0x75c` | `u32` ×n | motions by slot (stand, walk, run); -1 takes `g_ChrDefaultMoves` (`0x00c49680`) | `Ch_MotionSetMoves` (`0x0082afe0`) |
| `0x820` | `u32` | bit 2: needs an update | `Chr_SetHideFlags` (on clearing) |
| `0x824` | `i16` | -1 after each `Ch_SetPos` | `Ch_ResetPos` (`0x0082b0b0`), `Ch_IdGetPNum` (`0x0082adc0`) |
| `0x830` | ptr | render instance | `Ch_GetChrRenderInstance` (`0x00826b70`) |

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
| 13 | hidden from the calculation pass | `Ch_GetHideCalc` |
| 16 | cleared with the interpolation frames | `Ch_MotionSetHokan` |
| 18 | shaded by map id | `Ch_IdEnableShade` (`0x008340f0`) |
| 25 | position set | `Ch_SetPos` |
| 30 | overlap hit | `Ch_SetOverlapHit` (`0x0082b420`) |

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
| `0x30` | `f32` | data scale | `Ch_GetDataScale` (`0x0082ab60`) |
| `0x71` | string | name (the object's `+4` points here) | the snapshots |
| `0xb8` | string | directory name | `Ch_GetBlkDirName` |
| `0x124` | ptr, `u32` | resident data and its size | `Ch_EffectGetResidentPtr` (`0x0082a9b0`), `Ch_GetChrRenderInsatanceFromOEF2Content` (`0x00826af0`) |

## Floats

These functions move floats through the x87. `fld m32` quiets a signalling
NaN, so a float the original stores after loading it (`Ch_SetDir`,
`Ch_GetPos`, ...) comes out quiet. Copies made with integer moves keep
every bit: `Ch_SetPos` copies the position that way. When two quiet NaNs
are added (`Ch_AddGrav`), the result is the one with the larger
significand, or the positive one if the significands are equal.
`ffx_chara::x87` implements these rules, and its tests check them against
the host CPU's x87 unit. The oracle's emulator does not quiet signalling
NaNs on `fld`, so the parity tests leave signalling NaNs out.

`Ch_GetRot` wraps the angle with `Chr_WrapAngle` (`0x0082ba70`). It adds or
subtracts 6.283184 until the angle is within ±3.141592, using the `double`
widenings of those `f32` values, and stores each step as `f32`. Rust's
`f64` arithmetic followed by `as f32` gives the same result bit for bit.
The loop never ends for infinities, in the original and in Rust.
