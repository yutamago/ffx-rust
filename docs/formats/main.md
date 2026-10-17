# Main module state (`Sg`)

Layouts the main module's accessors rely on, recovered from `FFX.exe`
(`crates/ffx-sg`). Addresses are `symbols/ffx.sym`'s.

## Point lights

`0x013008cc` points to a table of `0x2c`-byte entries; `0x013008d0` holds
the last valid index (`i32`). Every `Sg_PLight*` accessor compares the
index with it signed (`jg`): a negative index reaches before the table.
Getters return 0 past the last light.

| Offset | Type | Field | Accessors |
|---|---|---|---|
| `+0x00` | `f32[3]` | position | `Sg_PLightSetPos(i, mask, x, y, z)` (bit 1/2/4 of `mask` per component), `Sg_PLightGetPos(i, &x, &y, &z)` (null outputs skipped) |
| `+0x10` | `f32[3]` | colour, 0..1 | `Sg_PLightSetColor(i, r, g, b)` stores each `int` (rounded to `f32` first) / 255.0 (`f64` at `0x00b170c0`); `Sg_PLightGetColor` multiplies back and truncates (`_ftol2_sse`) |
| `+0x20` | `f32` | radius | `Sg_PLightSetRadius`, `Sg_PLightGetRadius` |
| `+0x24` | `f32` | attenuation | `Sg_PLightSetAttenuation`, `Sg_PLightGetAttenuation` |
| `+0x28` | `i8` | enabled | `Sg_PLightSetEnable`, `Sg_PLightGetEnable` |
| `+0x29` | `i8` | the light it belongs to | `Sg_PLightSetLight`, `Sg_PLightGetLight` |

## GS packet (`Sg_Gs*`)

`0x013032c4` points to an array of 16-byte quadwords, `0x013032bc` is the
next one. `Sg_GsStart` remembers the count in `0x013032c0` and the tag's
quadword in `0x013032c8` and writes a GIF tag (`0x4d2`, `0x10000000`, `0xe`,
0: A+D, one register). Each register writer fills one A+D quadword (64-bit
data, the register number as 64 bits) and advances the count:

| Writer | Register | Data |
|---|---|---|
| `Sg_GsPRIM(p)` | `0x00` | `p` |
| `Sg_GsRGBAQ(r, g, b, a)` | `0x01` | `r \| g<<8 \| b<<16 \| a<<24` |
| `Sg_GsUV(u, v)` / `Sg_GsUV2` | `0x03` | `u*16`, `v*16` as `u16` (UV2: `+15`, the texel's far edge) |
| `Sg_GsXYZF2(x, y)` / `XYZF2V(x, y, zf)` | `0x04` | `((4096 - w)/2 + x) * 16`, the same for y, as `u16`; `zf` (0) above |
| `Sg_GsTexHokan(on)` | `0x14` | `0x25` (bilinear) or `5` |
| `Sg_GsXYOFFSET(x, y)` / `2` | `0x18`/`0x19` | `(2048 - w/2 - x) * 16`, the same for y in the high word |
| `Sg_GsTEXFLUSH()` | `0x3f` | not written |
| `Sg_GsZBUF(zmsk)` / `2` | `0x4e`/`0x4f` | `0x31000000 \| Sg_VramGetAddrZ() / 2048`, `zmsk` high |
| `Sg_GsSCISSOR(x0, x1, y0, y1)` / `2` | `0x40`/`0x41` | 16 bits each, `x0`, `y0` at least 0, `x1` at most `w - 1`, `y1` at most `h - 1` (signed) |
| `Sg_GsALPHA(a, b, c, d, fix)` | `0x42` | `a \| b<<2 \| c<<4 \| d<<6 \| fix<<32` |
| `Sg_GsDIMX()` | `0x44` | `0xffff_ffff_ffff_fffc` |
| `Sg_GsDTHE(on)` | `0x45` | `on` |
| `Sg_GsTEST(...)` / `2` | `0x47`/`0x48` | `ATE, ATST<<1, AREF<<4, AFAIL<<12, DATE<<14, DATM<<15, ZTE<<16, ZTST<<17` |
| `Sg_GsFRAME(fbp, mask)` / `2` | `0x4c`/`0x4d` | `fbp & 0x1ff`, width `w/64` (6 bits) and `0x100` at bit 16, `mask` high |
| `Sg_GsFRAME_16` / `2_16` | `0x4c`/`0x4d` | `fbp & 0x1ff \| 0x0a080000` (width 8, 16-bit), `mask` high |

`w` and `h` are `g_GuScreenSize` (`0x023cbbe4` width, `0x023cbbe0` height,
`Sg_SetScreenWH(w, h)`). The data words are built with 64-bit shifts of the
sign-extended `int` arguments, so a negative field sets every bit above it.
`Sg_GsEnd2` fills the first three quadwords with the DMA and VIF headers
sized by the count (`n - 1 | 0x10000000`, ..., `n - 3 | 0x8000`).
`Sg_GsEnd` (`0x00841770`) closes the packet for DMA instead: the tag's low
15 bits become `n - start - 1`, an end quadword (`0x8001`, `0x10000000`,
`0xf`, 0) goes at `n`, the first quadword becomes `n + 1 | 0x70000000`
(the rest of it zero), then `FlushCache(0)`, `sceDmaSync(ch, 0, 0)` and
`sceDmaSend(ch, packet)` on the channel in `0x023cbbf4`.

The debug lines (`Sg_DbLine`, `Sg_DbLineStrip`, `Sg_DbVertex`) append to
the buffer `0x013032d0` points to only while `g_FFXIsDebug` is set: a word
`0x10001` (line) or `0x20001` (strip), or a vertex record `3, x, y, z`.

## Switches

Bytes the main loop tests each frame, set by `Sg_Enable*(on)` (bit 0; the
old value sign-extended is returned):

| Address | Switch |
|---|---|
| `0x012fb8fe` | calculate characters (`Sg_EnableCalcChr`) |
| `0x012fb8ff` | draw characters (`Sg_EnableDrawChr`) |
| `0x012fbbb0` | calculate the camera (`Sg_EnableCalcCam`) |
| `0x012fbbb1` | calculate the map (`Sg_EnableCalcMap`) |
| `0x012fbbb2` | draw the map (`Sg_EnableDrawMap`) |
| `0x012fbbb3` | draw the GUI (`Sg_EnableDrawGui`) |
| `0x012fbbb4` | draw the menu (`Sg_EnableDrawMenu`) |
| `0x012fbbb7` | full-screen anti-aliasing (`Sg_EnableTkAnti`; `Sg_ToggleTkAnti` flips it) |
| `0x012fbbbb` | pause (`Sg_SetPause`, any byte) |
| `0x012fbc34` | keep the frame rate (`Sg_SetKeepFps`, any byte) |
| `0x00c496d4` | frames run per game frame (`Sg_SetExecFrames`, clamped to 0..=99) |

## Lights

Three directions (16 bytes each) at `0x00c49724`, `0x00c49734` and
`0x013008e8`, and four colours (the third the ambient one last) from
`0x00c49744`, 16 bytes apart. `Sg_InitLight` builds the colour matrix
(`0x023c4400`, `FFXVu0LightColorMatrix`) and then the normal matrix
(`0x01300978`, `FFXVu0NormalLightMatrix`) from them. `Sg_LightSetChr(chr)`
does the same for a character: bit `i` (0..=2) of the `i8` at `+0x188`
takes direction `i` from `+0x3c4 + 16 i` and colour `i` from
`+0x384 + 16 i`, bit 3 the ambient colour from `+0x3b4`.

`Sg_AnimValF` eases `{f32 value, f32 target, i32 frames}`: unless value
equals target (a NaN never does), value += `(target - value) / frames`
(the frame count rounded to `f32` first), or becomes the target when no
frames are left; the count then drops by one unless 0.

## Camera and screen

`Sg_CalcScreenMatrix` calls the battle camera's `MsSetScreen(0, depth,
...)` with `MsCameraGetScrDpt()`, the screen centre (`0x01301bbc`, set by
`Sg_SetScreenCenter(x, y)` to `2048 + (x - 256)`, `2048 + (y - 208)`) and
fixed planes. `Sg_CameraGetPos` copies the battle camera's side-1 position
into `0x00c4dec0`; the reference point is `0x00c4ded0`, the look vector
(reference minus position) `0x01301bd0`.

## Sequence channels (`SG_StepChannels`)

A channel list hangs from an object's `+0x74c` (next channel at `+0`).
Channel: byte stream pointer `+0xc`, wait count `u16` `+6`, delta `i16`
`+0x14`, value `u16` `+0x16`, previous value `+0x18`. Each frame the value
moves by the delta; with no wait pending the next stream byte first sets:

| Code | Meaning |
|---|---|
| `0xxxxxxx` | delta = the 7 bits, signed |
| `10nnnnnn` | wait `n` frames |
| `11llllll hhhhhhhh` | delta = `h` (signed) above the 6 bits `l`, 14-bit signed |

`SG_RelocateSequence(seq, base)` adds `base` to the two table pointers
(`+0xc`, `+0x10`, `i16` counts at `+8` and `+0xa`) and to the pointers at
`+8` and `+0xc` of each 16-byte entry.

## Character kinds (`sg_getdb*`)

| Kind | Letter | Name |
|---|---|---|
| 0 | `c` | `pc` |
| 1 | `m` | `mon` |
| 2 | `n` | `npc` |
| 3 | `s` | `sum` |
| 4 | `w` | `wep` |
| 5 | `f` | `obj` |
| 6 | `k` | `skl` |

`sg_getdbname` accepts the number or the letter; another value gives the
empty string.

## Memory blocks (`Sg_Mem*`)

The `Sg` heap (first block at `0x01301978`) chains 16-byte headers, each
right before its data:

| Offset (from the header) | Field |
|---|---|
| `+0x0` | next header |
| `+0x4` | previous header (`Sg_MemCache_SerchFree` walks this way) |
| `+0xc` | owner bits in the low nibble, 0 when free (`Sg_MemIsFree`) |
| `+0xa` (data `-6`) | `u16`: debug name id in the low 11 bits (`Sg_MemGetStr`, `Sg_MemClearStr`) |

Debug names are 32-byte slots at `0x05fb0000 + id * 32` (only in debug
builds). `Sg_MemGetSum(header)` sums the data's halfwords into `AX`;
`Sg_MemSetRtAddr(a)` stores `((a - 8) >> 2) & 0x7fffff` at `0x013019b0`.

## Motion groups and sequences (`SG_*MotionGroup`, `SG_RemoveSequence`)

Loaded motion groups: 50 entries of 12 bytes at `0x013011e0`,
`{group, i16 a, i16 b, key}`; `SG_FindMotionGroup(a, b, key)` returns the
first match. `g_ChrSequences` (`0x01300a08`) holds 100 entries of 20
bytes, the sequence pointer first; removing one clears its slot and
relinks the table (`Ch_SeqLink`).

## Quaternions (`Sg_Quaternion*`, `Sg_MatrixToQuaternion`)

`(x, y, z, w)`, row-vector matrices. `Sg_QuaternionSlerp` negates `b` for
a negative dot product and interpolates linearly within `0.0001`
(`f64` in `.rdata`). Sine, cosine and arc cosine come from MSVCR110's
`_CIsin`, `_CIcos`, `_CIacos` (thunks `0x009498c8`, `0x009498c2`,
`0x009498da`); the port calls the game's own import, and the oracle test
replaces all three on both sides by exact stand-ins (`x²`, `2x`, `x + 1`)
so the surrounding arithmetic is compared bit for bit.
`Sg_CreateVectorFromHV(out, h, v)` gives `(sin h cos v, sin v, cos h cos v, 1)`
for degrees; `Sg_VectorToMatrix2` builds a frame along `v` with up
`(0, -1, 0)`.

## Mouse buttons (`Sg_MouseButtonMain`)

Per button: held, pressed this frame (`<< 1`), released this frame
(`<< 2`); left at bit 0, right at bit 8, middle at bit 16; bit 3 is a
left double click.

## Character packet settings (`Ch_PacketSet*`)

The character draw sets these before building a model's VU1 packet:

| Address | Type | Setter | Source in the draw |
|---|---|---|---|
| `0x013033a8` | u32 | `Ch_PacketSetName` | model header `+0x18` |
| `0x01303342` | u8 | `Ch_PacketSetCull` | character flag bit 19 |
| `0x01303343` | u8 | `Ch_PacketSetShadow` | shadow mode 2 or 3 |
| `0x01303372` | u8 | `Ch_PacketSetSpecular` | `!g_ChrDrawDisableSpec` |
| `0x01303373` | u8 | `Ch_PacketSetShadowTexture` | model `+0x65` |
| `0x01303374` | u8 | `Ch_PacketSetTexture` | `Ch_EnableGetTexture()` |
| `0x01303375` | u8 | `Ch_PacketSetFSpecular` | `g_ChrDrawFourceSpec` |
| `0x01303376` | 3 × u8 | `Ch_PacketSetShadowColor` | `g_ChrShadowColor` and neighbours |
| `0x01303379` | u8 | `Ch_PacketSetShadowSemi` | `Sg_SetMovieGomiAvoid`'s byte |
| `0x0130337a`, `0x01303384` | 2 + 2 × u8 | `Ch_PacketSetSpecColor` | model `+0x60..0x63` |
| `0x0130337c` | f32 | `Ch_PacketSetSpecSize` | model `+0x58` |
| `0x01303380` | f32 | `Ch_PacketSetSpecCoefficient` | model `+0x5c` |

Names follow the Switch build's order (Name, Cull, Texture, Shadow,
ShadowSemi, ShadowColor, ShadowTexture, FSpecular, Specular, SpecSize,
SpecCoefficient, SpecColor) matched by argument counts and sources; the
texture and shadow-texture bytes are both cleared when no texture list is
loaded.

## Requests passed on (`crates/ffx-sg/src/control.rs`)

The main loop's requests to the renderer and the sound system; the work
behind them stays with those modules, which the bridges call.

| Function | State | Then calls |
|---|---|---|
| `Sg_Blur(weight)` `0x0082bee0` | `0x01300704` bit 1, byte `0x01300718` = 1; on (`u16` `0x0130071c`) and weight (`u16` `+2`), both cleared as one word for 0 | `graphicSetMotionBlurEnable(on)`, then `graphicSetMotionBlurWeight(i16 weight / 128)` |
| `Sg_AccSetAlpha(alpha, frames)` `0x0082bd10` | bit 0, pending `0x01300708` = 0, frames `0x0130070c`; over frames the target `0x01300710` and step `0x01300712` = `\|current - alpha\| / frames` (signed, at least 1); at once the alpha `0x0130070e` (and on map `0x102` the step 0); a non-zero alpha with no frames counts one | `graphicUpdateCrossFade(&alpha, &frames)` |
| `Sg_AccRequestCurrent()` `0x0082bdf0` | `g_AccCurrentRequest` `0x012ffae4` = 1, read by `sg_AccSetCurrent` | `0x00639150` (cross-fade capture on) first |
| `Sg_SetPauseAllSounds(on)` `0x00821cc0` | `0x012fb8ec`: 0 normal → 2 paused and back, unless `isInTkMenuPause()` | voices and effects paused or resumed |
| `Sg_SetMuteSound(on)` `0x00821c60` | the same word: 0 → 1 muted and back | one-shot effects stopped, effects paused, `iSoundSetMute(on, 2)` |
| `Sg_SetMapAlphaFast(on)` `0x00821b00` | byte `0x012fbbb6` | `graphicSetMapAlphaFast(on)` |
| `Sg_PLightDispose()` `0x00831ba0` | point-light table freed, table and count 0 | the memory move's trigger |
| `dreamParamReset()` `0x00822410` | `0x012ffaf4`, `0x012fb8f0`, `0x012ffae8`, `0x012ffaec`, `0x012fb8f4` 0; `0x00c496d0` -1 | `graphicMenuUnloadSpecialTexture(10)` |
| `Sg_LightSetDir(i, dir)` `0x00836430` | light `i` (0-2: `0x00c49724`, `0x00c49734`, `0x013008e8`) gets four words | `FFXVu0NormalLightMatrix(g_Vu0ClipScreen, the three)`, `graphicCharacterLightingSetLightDir(i, x, y, z)` |

## Map id data, motion groups, player settings

- `Sg_IdFixupAndSet(map)` (`0x00844bc0`): a map's id data arrives with
  offsets taken from the base in its word 0. When `map - base` is not 0,
  word 0 becomes that difference and `+0x1c`, `+0x18` and `+0xc` of each
  16-byte entry in `+0x1c`'s table (`i16` count at `+6`) move by it. The id
  shades (`g_ChrIdShadeOn`) go on for a version (`i16` `+4`) above
  `0x1202`; a change is told to
  `graphicCharacterLightingSetMapIdFlagChanged(old, new)`. Then
  `Sg_IdSetPtr(map)`.
- `SG_DisposeMotionGroup(a, b, key)` (`0x00836e20`): removes the group's
  sequences (`+4` of them, 20 bytes each at `+0xc`) and its table entry.
  A character group (`a` 0) or the system group (`a` 2) goes back to the
  cache, `Cache_Unuse(2, b << 16 | key)` or `Cache_Unuse(2, 0x903e7)`;
  any other is freed and the memory move triggered. Active characters
  whose `+0x7e8` is the group get `Ch_MotionSetNull`.
- `Sg_PlayerSetDiving(v)` (`0x0082d9d0`): byte `0x013007c5`.
  `Sg_PlayerSetSpeed(kind, v)` (`0x0082d9e0`): `f32` speeds for kinds 1,
  2, `0x12`, `0x13` at `0x013007b4..c0`; others ignored.
- `Sg_SetCameraFollow(on)` (`0x0082dab0`): bit 1 of `0x00c496d5` (bit 0 is
  `0x0082d030`'s). With both set, `Sg_ControlMain` turns the camera after
  the player's heading.
- Initialisers: `Sg_InitFilter` (`0x0082cd50`) clears `0x01300708..58`,
  the request flags `0x01300704` and `0x01300768`, then `Sg_FocusReset`.
  `Sg_IdInitAll` is `Sg_IdSetPtr(0)`. `Sg_PlayerInitAll` is
  `Sg_SetPlayer(0)` and diving on. `Sg_SetUserName(name)` keeps
  `SgStrdupD(name)` in `0x00c494a8`; the old copy is not freed.
  `Sg_PLightAllocate(n)` (`0x00831950`) disposes the old lights, then
  allocates `n` 44-byte lights, each off with `+0xc` and `+0x1c` 1.0.

## Start-up, event-jump initialisers and helpers (round 3)

- `Sg_InitArgs(argc, argv)` (`0x00820310`, start-up): L/R button mode 0,
  `0x012fb8fa` 0xff, `0x012fb8f9` and `0x012fb8f8` 0; a debug build takes
  the user name from `argv[1]`.
- `Sg_AccInitAll` (`0x0082bcd0`) and `Sg_PlayerInitDefault` (`0x0082d8d0`),
  both from `Sg_EventjumpInit`: the accumulation requests off (alpha 0 at
  once, map, background and characters off, clear colour 0, the current
  frame requested); no player, diving on, speeds 16 (kind 1), 38 (kind 2)
  and 20 (kind `0x12`).
- `setFramSync` (`0x00822720`): `0x012fb7e4` = 1 in a battle
  (`MsBattleCheck`) with a magic file (`MagicFile_GetMagicId() != -1`).
- `Sg_LightSetDirXYZ(i, x, y, z)` (`0x008364f0`): `Sg_LightSetDir(i, &(x,
  y, z, 0))`.
- `Sg_MatrixFromDir(m, dir, rows)` (`0x008435a0`): basis `d = |dir|`, `s =
  |(0,0,1) x d|`, `u = |s x d|` into the identity's rows named by the
  nibbles of `rows` (`0x12`: s in row 0, u in row 1, d in row 2). Rows
  past 3 are written past the matrix.
- `Sg_CalcTriangleNormal(v0, v1, v2, out)` (`0x0083e420`): `i16` vertices;
  `|(v2 - v0) x (v1 - v0)|`.
- `Sg_IntVecToFloat(out, v)` (`0x00845d90`): three `i32` to `f32`.

## Bones, sequences, id kinds, the dream effect, cloth rods (round 4)

- `SG_ApplyBone(_, chan, bone, scale)` (`0x00838280`): nine channels of
  `0x2c` bytes (value `+0x20`, changed byte `+0x28`, cleared when read).
  Rotations `Angle_Wrap(v * 2 * pi)` go to `bone+0x114..` and
  `bone+0xe4..`, translations `v * scale * 4096` to `+0x124..` and
  `+0xf4..`, scales copied to `+0x134..` and `+0x104..`.
  `SG_ApplyBoneDamped` (`0x00838470`) sets only the targets for rotations
  and translations. The current values then move toward them
  (`Chr_LerpAngleDamped(t, c, 0.7, 0, 10)`, `Chr_LerpDamped(t, c, 1, 0,
  1)`). `Angle_Wrap` steps by 2 pi, so an infinite angle never returns.
- `SG_AddSequence(seq, v)` (`0x00836d30`): the first free `g_ChrSequences`
  entry gets `seq`, its `u16` `+4` and `v` at `+0x10`. A full table asserts
  ("SG:AddMseq"). Either way `Ch_SeqLink` runs.
- `Sg_IdMatchChr(chr, i)` (`0x0083e4e0`): id entry kinds (low 7 bits of
  `+0xc`): 1 everyone, 2 not the player, 13 characters with `+0x198` bit 2,
  14 the player; `0x30..0x3f` through `g_ChrIdMapTable`.
- `recallSceneEffect(layer)` / `recallSceneEffect2(a, b)` (`0x00822580`,
  `0x00822620`): the dream effect. A shown layer whose display byte
  (`Yn_PreBgLayerDispGet`) has bits 0-1 clear starts it: `0x012fb8f0` = 1,
  phase `0x012fb8f4` = 0, special texture 10 loaded. It hides the layers
  and draws the effect in phase 5. A layer with bits 0-1 set ends a started
  effect past phase 0.
- The cloth rods, kinds 1-6 of the cloth step `0x00822de0`
  (`0x00822890`, `0x00822960`, `0x00822a50`, `0x00822b20`, `0x00822c10`,
  `0x00822cf0`): 12-byte rods `{i16 a, i16 b, f32 len, f32 lim}`.
  - Kinds 1 and 3 move `b` to `len` from `a` when `|b - a|^2` is above or
    below `lim`; kinds 2 and 4 move both halfway.
  - Kinds 5 and 6 always apply. `b = a` when `|d|^2` (5) or `|d|` (6) is
    at most `1e-5`.

## Cut-scene sync, id normals, dispatchers (round 5)

- `Sg_SyncCutScene()` (`0x00821d60`) calls `iSyncGetData(&time, &rate,
  &frame)` with the frame in a local, so a callee writes through a pointer
  to the original's stack. The bridge gets that memory from
  `ffx_bridge::Host::callee_scratch` (a static buffer in the game, the
  emulated stack's bottom in the oracle). An answer of 999 clears the sync:
  `0x012fb8a0..b0` 0, the frame table `0x012fb828` (30 words), its index
  `0x012fb814`, `0x00c494d4` 1. Any other answer turns it on, stores `frame
  << 7` at the index and makes time `0x012fb8ac` and rate `0x012fb8b0`
  milliseconds (`x 1000`).
- `Sg_IdGetNormal(out, i)` (`0x0083e130`): the normal of id triangle `i`
  from the id data's `i16` vertex numbers and vertex block (`0x01301a8c`, 8
  bytes per vertex).
- `SG_ApplyBoneSel` (`0x00838240`): `SG_ApplyBoneDamped` when the bone's
  `+0x144` is set, else `SG_ApplyBone`.
- `drawDreamEffect` (`0x008223a0`): which maps have the dream effect and on
  which layers: 0 on `0x86`, `0xa5`, `0xbf`, `0xf9`, `0xfc`; 3 and 4 on
  `0x129`; 7 and 8 on `0x187`. Elsewhere a started effect is reset.

## Effect strips, channel sampling, pose blending (round 6)

- `Ch_EffectPacketStoreStrip(v)` (`0x00844370`): effect geometry comes as
  triangle strips of 8-byte vertices (`u16` x, y, u, v). Each vertex goes
  into a ring of three (`0x01303354`, slot `count % 3`; count `u16`
  `0x01303370`, the newest slot's address `0x0130336c`). An x or y with
  bit 15 set refers back to an earlier vertex: the value `(x & 0x7fff) * 4
  / 3` words before (y: one word further). From the third vertex on, each
  new vertex makes a triangle with the two before it for
  `Ch_EffectPacketStorePoly`. Even counts give `(n-1, n-2, new)`, odd
  `(n-2, n-1, new)`, so all faces keep one winding. A null vertex starts a
  new strip.
- `SG_SampleChannels(obj)` (`0x008393c0`): the motion time `+0x740` (8.8
  frames) rounded up to a frame, clamped to the end (`+0x71c`) for a motion
  that stops (`u16` `+0x724` 1), else wrapped by the loop (`+0x720`). The
  channels rewind when the time goes back, then step to it (frame count
  `+0x7dc`). Each channel's `+0x1c` becomes its key or, between frames, the
  blend with the key before: `AngleTurn_Lerp` for angles, linear
  otherwise.
- `SG_BlendToTarget(obj, n)` (`0x008395b0`): one of `n` steps from the
  current pose toward the target. A channel motion (`+0x196` bit 0) steps
  each channel's `+0x20` toward `+0x1c`. Otherwise every bone steps its 3
  rotations (`Chr_FracStep`) and 6 values (`cur += (target - cur) / n`,
  skipped when already equal). `n` 1 sets `+0x194` bit 16.

## Edge slide, free camera, raw clips (round 7)

- `Sg_IdSlideEdge(from, to)` (`0x0083e180`): when a move from `from` to
  `to` crosses an id edge (endpoints `0x01301aec`, `0x01301af0`, found by
  `0x0083d960`), the move slides along it. The move is turned into the
  edge's frame (a rotation about y; `FFXVu0ApplyMatrix`) and loses its
  across component. While the result back in the world
  (`FFXVu0InversMatrix`) is still past the edge (`w.z n.x - w.x n.z > 0`),
  it steps one unit back along z. `to` = `from + w` in x and z.
- `Sg_ControlCamera()` (`0x0083f4b0`): the free camera. The right stick
  (bytes `0x023c44a6/a7`, dead zone `0x51..0xaf`) turns yaw `0x00c4def0`
  and pitch `0x00c4def4` by `(stick - 0x80)` degrees / 12, each wrapped
  once into `-pi..pi`. With the L/R button mode, the held buttons
  (`0x023c44ac`) 2/8 move the height `0x00c4dee4` by 0.2 and 1/4 the
  distance `0x01301bb8` by 0.4. Pressing `0x400` (`0x023c44a8`) cycles
  six distances (50, 100, 180, 250, 1000, 30). A debug build captures on
  `0x100`.
- `SG_ApplyRawClip(obj, clip, t, t2)` (`0x00838750`): uncompressed clips.
  The header gives the `i16` stride (frames per channel) and the bone
  count; the data starts at `+0x10`, nine channels per bone, frames
  contiguous. Frame `t >> 8` is blended toward `t2 >> 8` by `t & 0xff`
  into each bone's raw `i16` pose (`+0x146..`), through the bone map
  `+0x758` when present. `+0x7ec` then copies some bones' raw values to
  their neighbours: 1 copies bone 24's to 25 and 26, 2 copies bone 42's
  to 43, 3 copies bone 42's to 43 and 44. Last, every bone's floats are
  set: rotations through `SG_RawToRad`, translations times the scale
  `+0x38` of `+0x1c4`, scales / 4096.

## Motion groups, id heights, bones per character, the motion script (round 8)

- `SG_AddMotionGroup(group, a, b, key)` (`0x00836c90`): a loaded group takes
  the first free motion-group entry (50; full asserts "nb mgrp err"), is
  relocated (`SG_RelocateGroup`) and adds each of its sequences
  (`SG_AddSequence`).
- `Sg_IdGetHeight(p, i)` (`0x0083e040`, result in ST0): the ground height
  under `p` on id triangle `i`. With the triangle's normal `n` and first
  vertex `v`, it is `(n.v - (n.x p.x + n.z p.z)) / n.y`; 0 without id
  data.
- `SG_ApplyBones(chr)` (`0x00838f70`): every bone takes its channels at the
  character's scale / 1000. A character with `+0x871` set blends (damped).
  One with `+0x870` set and `+0x820` bit 2 clear blends when near the camera
  (`Ch_AnimCopyNear`): damped when `Ch_AnimCopy2` says so, else by the
  bone's own byte. Some maps apply directly anyway: `0x170`, `0x11d`;
  `0x50` for kind 4, `0x137` for kind `0x5045`, `100` for kinds other
  than 0, and `0x3c` while the scenario is below `0x9a`.
- `SG_SeqStep(chr)` (`0x00837900`): one opcode of the motion script
  (`+0x714`). 0 end (returns 2), 5 hold (3); 1 play a clip (`Ch_SetMotionSeq`
  with the clip's start and end marks, the end one frame later unless a
  loop operand is 1); 2 wait for `+0x728` 0; 3 wait a count (`+0x72a`); 4
  jump; 6 wait until `+0x737` reaches the operand. A wait returns 1 with
  the script unmoved, a step 0, an unknown code -1 (printed).

## The frame's pad (round 9)

`Sg_ControlMain()` (`0x0082d070`) reads pad 0 each frame. The held buttons
come from `TkGetControlerHist(0, 0, 0)` (16 bits, `0x023c44ac`) and the
four stick axes from `TkGetControlerAnalogHist(0, 0, axis, 0)` (axis 0..3
into `0x023c44a7` down to `0x023c44a4`). Pressed buttons (`0x023c44a8`)
are the ones newly held since last frame (`0x023c44b0`). Then the free
camera runs (`Sg_ControlCamera`). A debug build without the L/R button
mode steps the frames per step (`0x00c496d4`): 8+4 with 4 pressed raises
it, 4+8 with 8 pressed lowers it, and below 0 it returns to 0 and sets
`0x013007b3`. Button 4 alone requests `0x023c44b4`; button 1 is copied to
`0x013007b1`.

## Bones from the model (round 9)

`SG_InitBones(chr)` (`0x008277f0`) builds a character's skeleton from its
model (`+0x1c0`). It allocates named blocks: the bones' elements
(`0x160` each, "%s:Elem", `+0x328`), their channels (`0x18c`, "%s:Fcv",
`+0x748`), the model entries (`0x38`, "%s:Mdl", `+0x1bc`) and any
collisions (`0x60`, "%s:Collision", `+0x32c`; only `0x38` each cleared).
The model's bone table (`+0x1c`, 20 bytes each) gives the bind pose:

| Offset | Type | Meaning |
|---|---|---|
| `+0` | `u16` | parent bone; the bone itself means the root element `chr+0x1c8` |
| `+2..+6` | `i16` x 3 | rotation in hundredths of a degree (`Angle_Wrap(v / 100 * pi / 180)`) |
| `+8..+0xc` | `i16` x 3 | translation x 1000 |
| `+0xe..+0x12` | `i16` x 3 | scale x 4096 (divided, rounding to 0) |

When option points 7 and 8 share an ancestor, that ancestor and its own
ancestors lose their damping byte (`+0x144`), as does every bone whose
parent has none. Each model entry gets `Chr_ModelEntry`, a weight of 1.0 at
`+0x14`, and `+0x194` bit 21 when its `+0x18` is set. Then
`Ch_ModelCreatePacket` builds the packet.

## The envelope (round 9, `crates/ffx-sg/src/envelope.rs`)

The skinning code transforms a bone's vertex lists by its matrix
(`cdc_Mul3x4`) into the model's vertex buffer (16 bytes per vertex), in
thousandths with w 1.0. Three lists follow each other: rigid vertices,
first weighted vertices (they set), further weighted vertices (they add).

- `sg_calcenvelopeW_asm_opt(lists, out, m)` (`0x006ee270`), float form. Each
  list is a `u32` count, then entries: rigid `f32 x, y, z, u32 index`
  (16 bytes); weighted `f32 x, y, z, weight, u32 index` (20 bytes). The
  added w is `w + 0.0`.
- `sg_calcenvelope_asm_opt(lists, out, m)` (`0x006eee40`), packed form.
  The rigid list is a `u16` count and 2 bytes, then 8-byte entries (`i16 x,
  y, z` / 1000, `u16 index`). Weighted vertices come in groups: a `u16`
  count (`0xffff` ends the list), a `u16` base, then 8-byte entries (`i16
  x, y, z`, `u8` weight / 255, `u8` index + base x 256). The further
  weighted groups start 2 bytes after the first list's terminator, and
  they set w to 1.0.

Products are stored as `f32` at each step (vertex x weight, then x
1000).
- `sg_calcenvelopeW_asm(lists, out, mask, m)` (`0x006ed9e0`), source form
  of the float lists. The rigid list is a `u16` count and 2 bytes, then
  8-byte entries (`i16 x, y, z` in thousandths, `u16 index`). Each of the
  two weighted lists is a `u16` count, then 10-byte entries (x, y, z,
  index, `u16` weight in 1/10000). It keeps only vertices whose
  `mask[index]` is set (all with no mask). It converts them to the
  `_opt` float form, which runs as the list is built. With a mask, the
  new list is returned in a fresh block; else it returns 0.
- `sg_calcenvelope_asm(lists, out, mask, m)` (`0x006ee570`): the packed
  lists filtered the same way. Entries are copied as they are, each group
  count is replaced by the kept count, and terminators keep the 2 bytes
  after them. The 2 bytes after the rigid count come from the original's
  uninitialised stack. With a mask, the new list is returned in a fresh
  block.
- Neither port passes a stack buffer to game code. They build the list in
  Rust and write it straight into the allocated block, where the originals
  `memcpy` from a 32 KB stack frame.

## Pushing two characters apart (round 10)

`Sg_ChrPushApart(a, b, reach, dist)` (`0x0083ecd0`, called from
`0x0083d3b0`) separates two characters that are `dist` apart when they
should be `reach` apart. The overlap `d = reach - dist` is shared along the
x-z direction from `b` to `a` (`FFXVu0Normalize` of `a+0xc - b+0xc` with y
cleared). The shares come from the push weights at `+0x500` (set by
`Native_ChrSetPushWeight`, 0x5009):

| `a+0x500` | `b+0x500` | `wa`, `wb` |
|---|---|---|
| ≥ 0 | ≥ 0 | the weights (both 0: 1, 1) |
| < 0 | ≥ 0 | 1, 0 |
| ≥ 0 | < 0 | 0, 1 |
| < 0 | < 0 | `Sg_VectorGetLengthXZ(+0x1c, +0xc)` of each, over the larger; equal lengths return 0 |

`t = wa·d / (wa + wb)` (f32) and `u = d - t`; `t < 0` returns 0. In the
both-negative case `a` moves by `t` and `b` by `u`; otherwise `a` moves by
`u` and `b` by `t`. A move of `s ≠ 0` adds `n·s` to `a`'s step (`+0x4c`,
`+0x54`) or subtracts it from `b`'s, and sets `+0x194` bit 5 when the new
step is not zero. The result is bit 5 of `a+0x194 | b+0x194`. Comparisons
are the original's x87 ones: a NaN weight counts as non-negative.

## Cloth anchors (round 10)

`Sg_ClothPlaceAnchors(chr)` (`0x0082dbc0`, from `0x0082e390` after the cloth
step `0x00822de0` unless `0x012fb7d8` is set) moves each cloth's anchor
vertices onto the model. It needs the cloth (`chr+0x834`), the cloth vertex
buffer (`Cache_Exist_ClothVertexBuff(1, chr[0])`) and `chr+0x194` bit 21,
and first copies the sign-extended byte `chr+0x180` to `cloth+8`.

| where | what |
|---|---|
| `cloth+0x14` | the number of lists |
| `cloth+0x30[i]` | list: `u32` count, then entry pointers `e` |
| `vb+4[i]` | the paired list: a word, then pointers `v` |
| `e+0x40`, `e+0x42` | model group `g`, sub-mesh `k % 100` |
| `e+0x44` | vertex count; `e+0x48` set to 1 when done; `e+0x4c` output (16 bytes a vertex) |
| `v+8` | 8 bytes a vertex: flags (bit 0: lift), `u16` triangle (`0xffff` none), `f32` lift |
| `v+0xc` | three `f32` weights a vertex |
| `[chr+0x1bc] + 0x38 g` | `+0` header (`u16` sub-mesh count at `+0x18`), `+8` vertices (16 bytes), `+0x2c` sub-mesh table |
| sub-mesh `+0x44` | triangles: three `i16` corner indices |

A vertex is `w0 c0 + w1 c1 + w2 c2` (each component `(w0 c0 + c1 w1) + c2 w2`,
stored `f32`). When `Ch_UseAnchorVertex(chr, g, k)` is non-zero or the
vertex's bit 0 is set, it moves along `n = (c1 - c0) × (c2 - c0)` by
`lift / |n| · 1.1` (3.0 for character 1, group 0, sub-mesh 2), through
`Sg_VecSub`, `Sg_VecCross`, `Sg_VecDot`, `_CIsqrt`, `Sg_VecScale` and
`Sg_VecAdd`. On level hash `0xc58` (`graphicLevelGetNameHash`) a character
whose matrix rows `+0x1d0`, `+0x1e0` or `+0x1f0` has a squared length below
2.5e-7 gets all its anchors zeroed instead.

## The cloth step (round 10, `crates/ffx-sg/src/cloth.rs`)

`Sg_ClothStep(chr)` (`0x00822de0`, from `0x0082e390` before
`Sg_ClothPlaceAnchors`) runs when `chr+0x194` has bits 21 and 22.

Set-up:

- `scale` = `[chr+0x1c4]+0x3c`; `s2 = scale²`.
- The wind: strength, heading, elevation from `chr+0x80c/0x804/0x808` when
  `chr+0x198` bit 1, else the globals `0x012fbc44/48/4c`; the strength
  times `scale`, the direction from `Sg_CreateVectorFromHV`. On maps 0x15d
  and 0x84 (but for characters 1 and 0x4001) with `getScenerioFlag() < 2`
  a strength above 399 skips the step; on map 0xc0 the strength is 2000.
  `MagicFile_GetMagicId() == 0x1b4` with character 0x1120/0x1154 in
  motion 0x3008101a/b skips it too.
- The character's motion: `dist = |chr+0x684 - chr+0x694|`, `dy = (chr+0x10
  - chr+0x20) s2`, weights `ka = chr+0x814`, `kb = chr+0x818`. The reach is
  17 in battle (`MsBattleCheck`; `dist` and `dy` 0, character 3 in motion
  0x310b0/1 gets `ka = kb = 0`, and `chr+0x81c` caps the follow), else 0.3
  or 3.4 (0.15 or 1.7 with `chr+0x820` bit 1) as `dist` is at most 0.1 or
  above. `chr+0x820` loses bits 1 and 2 (bit 2 also when bit 3 is set).
- Repeats: `iters` = `chr+0x812` (doubled for characters 6, 7 on maps 0x15d,
  0x84) when `chr+0x810` is set, `dist` is out of reach or `chr+0x820` bit 2
  was set; the step runs `iters + 1` times. Without doubling, the first run
  is *lifted*: points rise by `100000 scale`.
- With `chr+0x820` bit 3, a floor: `chr+0x18 = 1`, the normal `chr+0x7c`
  through the transposed `chr+0x210`, the point `chr+0x1d0`'s translation.

Per model group (`[chr+0x1bc] + 0x38 g`, `u16 [chr+0x1c0]+6` of them) with
strands (`hdr = [grp]`, `u16 hdr+0x18`):

| group | |
|---|---|
| `+0` | header: `u16` root bone, `+0x18` strand count, `+0x1c` strands (0x24 each) |
| `+8` | the model vertices (16 bytes) |
| `+0xc` | the cloth points: per strand `3 tc` points (positions, velocities, previous positions) |
| `+0x1c` | where the group was (vec3) |
| `+0x30` | marks (a byte per vertex), set for the strands' vertices until `+0x34` is 1 |

`M` = `chr+0x1d0` x root bone (`FFXVu0MulMatrix`, the bones' translations at
`+0xb8` scaled by 1000 around every product). The follow `F = (grp+0x1c -
M.t)`: zero with `chr+0x194` bit 23; beyond the reach it is zero, the group
not free, the run lifted and `iters` reset from `chr+0x812`; else `F.x, F.z`
times `s2 (|F|/1.7)² kb` (the square capped by `chr+0x81c`) and `F.y =
(F.y s2 + dy) ka` with `chr+0x820` bit 0, else 0. The group remembers
`M.t`. Then each collision record (`[chr+0x1c0]+0x30`, 0x40 each: radius,
reach, `u16` kind, `i16` bone, axis scale, local axis `+0x10`, local point
`+0x20`) places its point at `[chr+0x32c]` (0x60 each, axis at `+0x10` for
kinds 1 and 2) in the group's space, scaled by `s2`.

Per strand:

| strand | |
|---|---|
| `+0` | `i16` fixed points (copied from the model vertices) |
| `+4` | lift (times `scale`, added to the push's y) |
| `+8` | `u16` vertex indices |
| `+0xc` | rod runs: `i16` kind (1-6, 0 ends), `i16` n, 8 bytes, n rods of 12 |
| `+0x12`, `+0x14` | `i8` collision count, `i16` record indices |
| `+0x13`, `+0x20` | `i8` anchor count, anchors (0x20: point, `i16` bone at `+0x10`) |
| `+0x18` | damping (velocity scale) when the group is free |
| `+0x1c`, `+0x1e` | `i16` points, `i16` free ends (no collisions) |

The push = wind `x (rand() % 1000) strength + F`, its y lifted, into the
group's space (`Sg_MatrixApplyVec3`). Moving points get the push (plus their
velocity when free), the rods run, map 0x107 lifts two points of character
2's group 2 strand 2 (scenario 0x532-0x53c), then each moving point is
pushed out of the spheres (kind 0), capsules (2) and planes (1) by the
radius times `Ch_UseLargeCollisionRad`, and out of the floor by 1.2 its
depth; written to its vertex with `w = 1`, its velocity the damped change.

## Round 11: the main loop's callees

`ExportValidateInterface(id)` (`0x008202e0`) answers 1 for the interface
ids 0x40400003 and 0x40800001..0x40800003, else 0.

Glue (GS packets and the PS2 DMA FIFO, left to the original):
`Sg_MFIFOSpadSync` (`0x008432d0`, a wait on the GS register sink's bit 8),
`Sg_MFIFOEnd` (`0x00843260`, an end tag sent to the FIFO),
`Sg_AccSetFrameMap` (`0x0082bc50`), `Sg_DrawSetEnvironment` (`0x0082c7b0`),
`Sg_DrawSetEnvironmentInit` (`0x0082c8d0`) and
`Sg_DrawSetEnvironmentInitMovie` (`0x0082caa0`), which build GS register
packets.

`Sg_DrawFilter()` (`0x0082c0c0`) runs the five screen fades at
`0x01300708` (0x10 each: mode, r, g, b, then `i16` frames left, total or
level, target, step; `0x01300704` a bit per running slot) and the focus
(`0x01300768` requested, `0x0130076c` drawn, depths at `0x01300758/c`
through `graphicSetFocusZ`). Each slot with frames left calls
`graphicAddScreenFadeTask(r/256, g/256, b/256, a, flags)`:

| mode | alpha | |
|---|---|---|
| 0 | the level, moved by its step towards the target | quad `0x0082baf0` when the screen is 0x200 wide and no request is current; dream share |
| 2 | `1 - (127 - n' 127 / total) / 127`, black | then `Sg_AccSetAlpha(0, 0)` |
| 3 | `1 - (n' 127 / total) / 127`, black | held at the last frame |
| 4 | `(n 255 / total) / 127` | dream share `/ 255` |
| 5 | `(255 - n' 255 / total) / 127` | held; dream share |
| 6 | `(n 127 / total) / 127` | |
| 7, 8 | `level / 127` | 7 zeroes the level with the dream effect on; 8 passes flag 0x10 |

`n'` is the frame count after the decrement; integer divisions truncate.
A total of 0 is a division fault in the original; the port takes 0.

`sg_calcenvelope_normW`'s translated R5900/VU0 code calls small leaves
(names ours): `sg_Daddiu` (`0x00845cc0`, 64-bit add of a sign-extended
immediate), `sg_SetFloat` (`0x00845cf0`, a float through the x87),
`sg_Sll` (`0x00845d70`, shift by `n & 31`), and three scalar products on
the VU0 accumulator at `0x00c8f790`: `sg_VmulScalar` (`0x00845e80`,
`dst = v s`), `sg_VmaddScalar` (`0x00845e40`, `dst = ACC + v s`) and
`sg_VmaddaScalar` (`0x00845df0`, `ACC += v s`); x and y multiply x87
registers, z takes `v.z` as a memory operand, and the accumulator's y and
z lanes are memory operands. `Ch_EffectPacketCreateMain` sets the two
`u16`s at `0x0130366c` (`g_ChrPacketState`) and `0x01303684` through
`Sg_SetChrPacketState` (`0x00845c30`) and `Sg_SetChrPacketWord`
(`0x00845c40`).

## Round 13: the player's control (`crates/ffx-sg/src/player.rs`)

`Sg_ControlPlayer()` (`0x0082d180`) steers the player (`0x01300788`,
`Sg_SetPlayer`) from the frame's pad. Axis 3 (`0x023c44a4`) is the left
stick's y and axis 2 (`0x023c44a5`) its x: 0x59..0xa7 on y and 88 < x <
168 (an `f32` compare) count as 0x80; the results go to `0x013007ac` and
`0x013007a8`. Without a player nothing else happens; with its control
off (`Sg_GetPlayerControlEnable`) the speed is 0.

Four levels at `0x0130078c` (`i32`, up, down, right, left) ramp by 32 a
frame. The d-pad (any of 0xf000: 0x1000 up, 0x4000 down, 0x2000 right,
0x8000 left) raises its levels to 256 and lowers the others to 0.
Otherwise the stick, doubled about its rest (`2y - 256`, `2(128 - x)`, as
`f32`) and zeroed in -96 < v < 96, raises its side's level, capped at the
stick's magnitude (through `ftol`), and lowers the opposite one to 0.

The speed is `sqrt((right-left)^2 + (up-down)^2)` (`f32`), 256 above 256
and 0 below 48; the angle `atan2(right - left, up - down)` (`_CIatan2`,
stored as `f32` in `0x013007a0`) is kept only when both axes rest in the
dead zone and a falling level of each hit 0 this frame. In motion type 2
with button 0x40 while diving (`0x013007c5`) a gravity below 2 gets 0.2
more (`Ch_AddGrav`).

The camera's yaw (`Sg_CameraGetYaw`, `0x0083f2d0`, a jump to
`MsCameraGetYaw`: `MsVectorRect2Polar` from `g_Camera + 0x66c` to
`+ 0x50`) goes to `0x0130079c` every frame, except with `g_CameraFollow`
3: there it is taken when the angle moved more than `Angle_Wrap(0.349)`
from `0x013007a4` (which then takes the angle, and the offset `0x01300780`
0), and on the levels whose name hash (`graphicLevelGetNameHash`) is
0x19c or 0x220 every frame; standing still there sets `0x013007a4` to
-1000. Standing still the speed is 0. Moving, the character gets
`Ch_SetSp(speed * s / 256)` with `s` the type 2 speed (`0x013007bc`) or
the walk and run speeds (`0x013007b4`, `0x013007b8`; run without button
0x40) and `Ch_SetDir(-((angle - offset) - yaw))`, the fixed direction
(`Sg_PlayerSetDirFix`, `0x013007c8`) replacing the yaw when its flag
(`0x013007c4`) is set. Every intermediate is an `f32` store of a 53-bit
x87 result.

## Round 13: the normals' envelope (`sg_calcenvelope_normW`)

`sg_calcenvelope_normW(lists, out, m)` (`0x00845ec0`, called by
`Apply_Envelope`, `0x00832110`, for the normals) is PS2 R5900/VU0 code
translated register by register: the registers live in memory
(`g_Ee_Gpr + 16 n`: a0..a2 the arguments at `0x00c8f548/58/68`, t0..t6 at
`0x00c8f588..e8`, t8 the list's end at `0x00c8f688`; VU0 registers from
`0x00c8f784`), and every step goes through a leaf (`sg_Sll`, `sg_Daddiu`,
`op_add64`, the `pextlw`/`pcpyld` copies `CopyFields_007ecb20` and
`sg_CopyFields_00845d00`, `VFX_Copy16*`, the scalar products). The port
keeps every register write.

The matrix rows go to `0x00c8f7d0/e0/f0`; 0.0001 (`.rdata 0x00b92154`)
reaches `0x00c8f840` through the word at `0x00c8f784` (saved at
`0x013036a8` and restored); the result register `0x00c8f820` starts as
`vf00`. Three lists follow:

| list | header | entry | `out[index]` (16 bytes each) |
|---|---|---|---|
| rigid | `u16` count, 2 bytes | 8: `i16` x, y, z, `u16` index | rows x (x, y, z); w as the result register had it |
| first weights | `u16` count | 10: x, y, z, `u16` weight, index | rows x v x (weight x 0.0001) |
| further weights | `u16` count | 10 | that plus `out[index]`'s x, y, z |

The products run as `sg_VmulScalar` (x), `sg_VmaddaScalar` (y) into the
accumulator and `sg_VmaddScalar` (z); weights scale with `opus_vec3_scale`.
A loop ends when a0 equals t8 in both words; a count equal to r0 (0)
skips a weighted list.
