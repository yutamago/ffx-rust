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
