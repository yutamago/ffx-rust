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
