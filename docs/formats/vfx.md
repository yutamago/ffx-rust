# Particle (ppp) draws

The ppp programs (`g_pppProgramTable`, `0x00c3a508`) draw through
`crates/ffx-vfx`. Most of them only compute: positions, colours, UVs and
alpha go into render requests (`InitVFXRenderData` and friends) that the
64-bit renderer takes over. This page lists what is left at the edge.

## Renderer seams

Two draws hand data straight to the PS2 GS. In `FFX.exe` the GS calls they
use are stubs (`xor eax, eax; ret`, or a bare `ret` for `FlushCache` and
`FFXVu0FTOI4Vector`), so on PC neither transfer happens. The ports compute
everything up to the transfer and make the GS calls through the callee
path, where a renderer can take them over.

| Draw | Seam | What crosses it |
| --- | --- | --- |
| `pppKeTkFadeDraw` (`0x0072ee50`) | `sceGsSetDefLoadImage` (`0x0062f820`), `sceGsExecLoadImage` (`0x0062f7a0`) | a 256-entry, 32-bit CLUT built in a `0x420`-byte scratch buffer (`op_spr_alloc`), uploaded 16 x 16 to the texture record's CLUT (`+0x10`: base `>> 7 & 0x3fff`, format `>> 21 & 0xf`) |
| `pppKeLnsLpTOtDraw` (`0x0075b230`) | `sceGsSetDefStoreImage` (`0x0062f830`), `sceGsExecStoreImage` (`0x0062f7b0`) | the depth buffer (`PSMZ24`, `0x31`) round the flare's projected point, read back into a `0x3060`-byte scratch buffer at `+0x60`, 3 bytes per texel |

### `pppKeTkFadeDraw`

The work area `[pos, speed, accel]` (at `obj + 0xa0 + [[prog + 0xc]]`)
steps each frame unless `g_pppStop` (`0x0230fd34`) is set:
`speed += accel`, `pos += speed`, and on the key's frame the key's
`+8..+0x10` are added. Unless the key's texture (`+4`) is `0xffff`, the
CLUT is filled with `n = ftol(pos)`:

- `n >= 0` (clamped to `0xff`): entries `0..n` get colour `+0x18`, the rest
  a ramp from `+0x18` toward `+0x14` over `256 - n` steps.
- `n < 0` (clamped to `-0x100`, then `+ 0x100`): the ramp covers entries
  `0..n` (over `n` steps), the rest get `+0x14`.

Entries are in the GS's CSM1 order: `0x00c3e6c0` gives an entry's position
within its 32-entry block, `0x00c3e6a0` the step to the next one. Ramp
channels are `fistp`-truncated and OR-ed together without masking.

### `pppKeLnsLpTOtDraw`

The flare's visibility `+0x7c` is the fraction of the box
`+0xd0 x +0xd1` (half-extents, in 1/8 pixels) round its projected point
whose depth lies in front of the point. The box is clamped to the draw
buffer's scissor (`Sg_GetDBuffDC`, `0x00820530`, the half picked by the
field parity it returns). It is re-measured only on the object's frame
(`+0x8c` and the frame number modulo `+0x8d + 1`); between, `+0x7c` decays
by `+0xac`, the last fraction.

On PC the projected point is never computed (`FFXVu0FTOI4Vector`,
`0x0062fcd0`, is a bare `ret`), so the original reads it from stale stack,
and the depth comes from stale scratch memory. The port reads its own
zeroed locals instead, which gives an empty box and a visibility of 0. A
renderer that wants flare occlusion has to supply the point and the depth
at this seam. Until then the port stays out of the hook table.

### To observe

`pppKeLnsLpTOtDraw` belongs to the `pppKeLnsLpT` program (record at
`0x00c3bae8`): `pppKeLnsLpTCon` (`0x0075b090`) registers it on
`g_pppOtList` (`0x0230fff0`), and `pppKeLnsLpTDraw` (`0x0075b160`) draws
the flare with the visibility it leaves in `+0x7c`. Which spell or field
effect uses the program is not yet known; a breakpoint on `0x0075b230`
(x64dbg or Frida) during effects with lens flares finds one. Then check
in the original:

- whether the flare is visible at all, and whether it flickers or fades;
- the values at `+0x7c` and `+0xac` of the object each frame (the
  visibility and the last fraction), and whether they stay 0;
- the projected point it reads (`ebp - 0x14 .. - 0xc`) and whether that
  stale data repeats from frame to frame.

If the flare shows in the original, the hook needs those values from
somewhere other than zeroed locals before it can go in.

## Model draws (`pppDrawMdl*`)

Ported in `crates/ffx-vfx/src/mdl.rs`. Each draw builds the colour, then a
render request on its stack frame and hands it to `updateVFXRenderData`
(`0x00718610`) when the instance has render data
(`graphicVFXGetRenderDataByInstance`, `0x006453d0`), or to
`InitVFXRenderData` / `InitVFXPath3RenderData` with an alpha word from
`calculateVFXSP_AlphaFunction` (`0x00712c30`) and `calculateVFXSP_ZMat`
(`0x00712d00`). The instance is `(g_pppCurrentProgram, obj +
sign(g_pppCurrentProgram))`.

### Colour

The work area's `+0xa0` halves (read through `g_pppColorConShift`) times
the current part's `+0x70` halves on the emulated EE: `pextlh` with `r0`
on both, `pmulth`, each product `>> 19 & 0x1fff`, packed back with `r0`'s
low halves above, `ppacb`. `pppDrawMdlDraw` calls `pppEePextlh`; the other
eleven inline it through a stack temporary, writing `0x00c8f588..0x00c8f5a7`
as overlapping halfwords in three different orders. A symbolic run of the
eleven sequences (`~/ffx-work/vfx/tools/eesim.py`) gives the same final
registers as `pextlh` in every one, and nothing reads them between the
stores, so the port writes the final words. The packed colour goes to the
request's `+0x18`; its top byte is the alpha, and a zero alpha skips the
draw.

### Request

| Offset | Contents |
| --- | --- |
| `+0` | flags: `4` path 3, `8` adds alpha bit `0x40`, `0x10`, `0x80`, `0x100`, `0x200`, `0x2000` view matrix (skips the Z matrix) |
| `+4` | `[0x00c498dc]` |
| `+8` | blend (`u16`); `+0xa` cleared; `+0xb` never written |
| `+0xc..+0x14` | 0, or (`Ts`, `Loop`) two work-area floats in 16.16 (`ftol(v x 65536.0)`, double at `0x00b51558`) |
| `+0x18` | colour |
| `+0x1c` | the part's `+0x78` (0 in the `Loop` draws) |
| `+0x20`, `+0x2c` | the object's matrices `+0x50` and `+0x10` |
| `+0x24`, `+0x28` | `0x0230fd60`, `0x0230fda0` |
| `+0x30` | a view-space matrix on the stack (`g_pppViewMatrix x obj + 0x10`) for flag `0x2000`; else never written (0 in the `Loop` draws) |
| `+0x34` | `g_PppPacketPos` |
| `+0x38`, `+0x3c` | `get_ptr32` of the object's `+0x90`, `+0x94` |
| `+0x40` | 0 |
| `+0x44` | the Z-matrix argument: 0, the object's `+0x50`, or a work area `obj + 0xa0 + [[prog + 0xc] + 4k]` |

Flags come from the model's first byte (bit 0 opaque, bit 1 no `0x10`)
and, in most variants, a key byte (bit 1 path 3, bit 0 `0x80`). The `Semi`
draws take their blend from the key (`+8`), the others from the model
(`+0x16`).

### Work areas

- `Ts`, `Ts2`, `Ts3`: `obj + 0xa0 + [[prog + 0xc] + 8]` holds two
  `[pos, speed, accel]` triples, stepped like `pppKeTkFadeDraw` unless
  `g_pppStop`; the key's `+8..+0x1c` are added on its frame.
- `Loop`, `LoopZ`, `LoopDisPos`: the same offset holds a `ptr32` to the key
  (set up, with the key's `+8` and `+0x14` as positions, while null) and
  two channels at `+4` and `+0x10`; positions wrap with
  `_CIfmod(pos + speed, 32768.0)` (double at `0x00b51588`).
  `LoopDisPos` draws the object relative to the running part: its matrices
  are `T(-g_pppWorldMatrix position) x obj + 0x10` and
  `g_pppViewMatrixCopy x` that.

### Per-effect fixes

The `3` draws and `LoopZ` patch requests for particular content:

- magic files (`MagicFile_GetHandle`/`GetMagicId`) with `g_VfxPass`
  (`0x0113335c`): `0x296` clears path 3, `0x241` sets it for model fields
  `0x36c0`/`0x3700`; `0x29e`, `0x29f` and `0x2c9` move the object's matrix
  to screen space (`x g_pppScreenMatrix`, then `x` the inverse of
  `g_ScreenMatrix`) and divide its `+0x30` by 1.8 when `TOGetFFXLang`
  returns `0xb`; alpha bits for `0x17a`, `0x110`, `0x111`, `0x231`; blend
  byte `0x48` for `0x1f9` (`Ts3`).
- maps (`g_CurrentMapNo`) with `g_VfxYiqMode` or `g_pppFpCurrentId`
  (`0x00c3a500`, the playing field effect's `+0x44` halfword, set by
  `pppFpLoop`): `0x17c`, `0x3e`, `0xc6`, `0xc2`/`0xff` in `3`; `0xc6`,
  `0x102`, `0x103`/`0xf4`, `0x110`, `0x139`, `0x7a` in `Ts3` (clears `0x10`,
  alpha `0x800`) and `LoopZ` (clears `0x10` only).

## Lens-flare draws (`pppKeLns*`)

Ported in `crates/ffx-vfx/src/lns.rs`. `Arnd`, `Crn` and `Fls` each come
in three draws around one work area `a = obj + 0xa0 + [[prog + 0xc]]`:

- `Draw` (`0x00759ff0`, `0x0075a760`, `0x0075ab50`) runs the kind's update
  and draws at once from the key;
- `TDraw` (`0x0075a190`, `0x0075a900`, `0x0075ad00`) runs the update and
  stores the key's values in the work area;
- `TOtDraw` (`0x0075a250`, `0x0075a9e0`, `0x0075ade0`), the `g_pppOtList`
  draw the `TCon` registers, draws from the work area.

Both drawing paths hand `KeLns*_Draw(desc, flags)` (flags 0 from `Draw`,
4 from `TOtDraw`) a descriptor on the stack:

| Word | Contents |
| --- | --- |
| 0 | `set_ptr32` of the object (`Draw`) or work area (`TOtDraw`) |
| 1 | `set_ptr32` of the work area |
| 2 | colour bytes: the key's `+8..+0xb` times the part's `+0x70` halves, `>> 12` |
| 3 | `set_ptr32(g_pppScreenMatrix)` |
| 4 | size (`f32`) |
| 5 | `Arnd`: `set_ptr32` of the position; `Crn`/`Fls`: two halves |
| 6 | `Crn`/`Fls`: `set_ptr32` of the position |

The position is the part's `+0x60` (`Draw`) or the work area's copy of it
(`TDraw` copies it with `FFXVu0CopyVector`).

| Kind | size | colour | position | halves (work area) |
| --- | --- | --- | --- | --- |
| `Arnd` | `+0x34` | `+0x38` | `+0x14` | |
| `Crn` | `+0x2c` | `+0x30` | `+0xc` | floats `+0x34`, `+0x38`, truncated (`fistp`, chop) |
| `Fls` | `+0x30` | `+0x34` | `+0x10` | float `+0x38` truncated, `u16 +0x3c` |

`pppKeLnsLpDraw` (`0x0075af80`) steps its own work area (the key's `+8`
and `+4` added to `+0xa8`, `+0x9c` on its frame, bytes `+0xc`, `+0xd` to
`+0x8e`, `+0x8f`), puts the object's matrix applied to `(0, 0, 1, 1)` in
`+0x90` while `+0x8f` is set, copies the object's position `+0x40` to
`+0x80` and calls `KeLnsLp_Update` with `[set_ptr32(area), obj + 0x40,
g_pppViewMatrix, g_pppScreenMatrix]`.

## Fading models (`pppKeMdlTfd*`)

Ported with the model draws (`mdl.rs`, `ppp_ke_mdl_tfd_draw`). The work
area `obj + 0xa0 + [[prog + 0xc] + 8]` holds one `[pos, speed, accel]`
triple (`Tfd`, `Tfd2`, `Tfd3`) or three at `+0`, `+0x10`, `+0x1c` (`Uv`
variants, key floats from `+8`, `+0x20`, `+0x2c`), stepped like the `Ts`
draws unless `g_pppStop`. Unless the key's model is `0xffff`:
`KeMdl_MkTkClut(key[0x14], key[0x18], pos, 0)`, then

    KeMdl_Draw(obj, model, obj + 0xa0 + [[prog + 0xc]], key[0x1c] << 2,
               u, v, mode, zmat, part + 0x70, part[0x78])

| Variant | `u`, `v` | `mode` | `zmat` |
| --- | --- | --- | --- |
| `Tfd` (`0x00749810`), `TfdUv` (`0x00749950`) | 0, or `ftol` of positions 2, 3 | 1 | 0 |
| `Tfd2` (`0x00749b40`), `TfdUv2` (`0x00749c90`) | as above | key `+0x1d` / `+0x38` | 0 |
| `Tfd3` (`0x00749e90`), `TfdUv3` (`0x00749ff0`) | as above | key `+0x1d..+0x21` / `+0x38..+0x3c` OR-ed | `obj + 0xa0 + [[prog + 0xc] + 0xc]` |

## Draw matrices

`pppFpSetDrawMatrix(mode)` (`0x0072a260`, `fp.rs`) sets `g_pppViewMatrix`
for a field effect: mode 1 keeps `g_pppWorldMatrix`'s rotation and moves
its position by `g_pppCameraMatrix`; mode 2 is `g_pppWorldMatrix x
g_pppFpWoodMatrix` (`0x0230fea0`, the camera billboard `pppFpSetWoodMatrix`
builds); anything else `g_pppWorldMatrix x g_pppCameraMatrix`. The result
is copied to `g_pppViewMatrixCopy`.

`pppDrawMatrixWoodSta()` (`0x00734d90`, `mat.rs`) builds the same kind of
billboard from `g_pppViewMatrix` in `0x0230fee0` and copies it to
`g_pppWoodMatrix`: row 1 the view's row 1 (`w` 0) normalised, row 0
`(y, -x, 0, 0)` of it normalised, row 2 their outer product normalised,
row 3 the view's translation.

`pppKeZCrctShpDraw(obj, key, ctrl)` (`0x007354c0`, `mat.rs`) builds a
shape's draw matrix in work area 0 on the emulated VU0: rows 0..2 are the
object's rows times the running part's scale (`+0x60`), and row 3 depends
on the key's mode (`u8 +0x30`), with `v` = the object's `+0x40` times the
key's `+0x20` (xyz): 0 is `(v + key +0x10) x g_pppViewMatrix`, 1 is
`v x g_pppViewMatrix + key +0x10`, 2 is the part's scale plus the scale
times the key's `+0x10` (`v x g_pppWorldMatrix` is computed into the VU0
registers but not used). Any other mode leaves row 3 as the stack held it
(the port's hook: zero). The VU0 registers `0x00c8f790` (accumulator),
`0x00c8f820..0x00c8f85c` and `0x00c8f930..0x00c8f98c` keep the original's
final values.

## Dynamic geometry (`opus_0582_dot_draw`)

`opus_0582_dot_draw(b, rec, m, hold)` (`0x00a72b80`, `slots.rs`; called by
`opp_0582_cmd_a70910` kind 0x1000) draws the `opus_0582` dots through two
render instances: `(0, b)`, two segments of 9000 points and 18000 indices
(line pairs), used when any entry's spread / 16 is not 0, and `(0x1000,
b)`, one segment of 2500 points and indices. A missing instance is made
with `graphicVFXCreate` / `graphicVFXCreateDataBuffer` (names
`"NoTexture"`, `0x00b41b50`) / `graphicVFXSetup` and registered with
`op_pair_index_00a70ce0`; the buffers come from
`graphicDynGeoMemManagerGetBufferForSegment`. The entries (32 bytes from
`b + 0x10`, count at `b`):

| Offset | Contents |
| --- | --- |
| `+0..+0xc` | `f32` radius x, height, radius z, scale |
| `+0x10` | `u16` angle (`& 0xfff`, a turn of 4096 `g_pppSinTable` steps) |
| `+0x12` | `i16` angle step per frame |
| `+0x14` | `i16` spread: `n` = spread / 16 points (32 from 64), stepping the angle by spread / `n` |
| `+0x16` | `u16` life (alpha `8 x life`, 0x80 from 16); 0 skips the entry |
| `+0x18..+0x1a` | colour bytes (`/ 256`) |
| `+0x1c` | `i8` fade-in (alpha `(8 - c) x 16`), counted before the life |

A point is `(sin a x rx x s, y, cos a x rz x s)`. Strands alternate between
the two line segments, the alpha falling by `128 / n` per point (not below
0). The segments' counts are set, their unused tails cleared, and each
instance gets `m` times `g_pppFillMatrix`, bounds of +-100000 and
`graphicVFXSetVisible` / `graphicVFXUpdate`. A failed allocation returns
before `op_spr_free`.

## Texture mip filters

The op layer builds a texture's smaller levels on the CPU (caller
`0x00a384d0`), per row, for 32-bit RGBA texels (`size` 4) or 8-bit ones
(`size` 1); other sizes leave the level alone.

| Function | Filter | Rounding |
| --- | --- | --- |
| `op_texels_halve_00a385f0` | two neighbours in a row | up: `(a \| b) - ((a ^ b) >> 1 & 0x7f7f7f7f)` per byte |
| `op_texels_halve_v_00a38560` | two neighbours in a column | as above |
| `op_texels_box_00a38670` | 2x2 box from two rows | nearest: `(a + b + c + d + 2) >> 2` per byte |

For 32-bit texels the box filter sums bytes 0 and 2 in one masked word
(`& 0x00ff00ff`) and bytes 1 and 3 in another (`>> 2 & 0x3fc03fc0`), so no
carry crosses a byte; the result is bit-exact to the per-byte formula.

## Saved VU0 state (`pppKeShpTail3`, `pppKeShpTail3X`)

The tail draws save the emulated VU0 registers they use and restore them
after drawing: `pppKeShpTail3VfCopy_00751680` copies `0x00c8f7b0..0x00c8f7ef`
to `dst + 0xc0`; `pppKeShpTail3VfLoad_00751740` (and its byte-identical copy
`pppKeShpTail3XVfLoad_00751ab0`) loads a saved block back:

| Block words | VU0 registers |
| --- | --- |
| `0x00..0x1f` | `0x00c8f830..0x00c8f8af` |
| `0x20..0x27` | `0x00c8f7f0..0x00c8f80f` |
| `0x40..0x4b` | `0x00c8f930..0x00c8f95f` |

## Bind-effect signals (`op_et_bindeff_send_*`)

A character's bind effects are the `g_EtEff` records with IDs `0x40000400`,
`0x40000500`, `0x40000600` and `0x40000700` plus the character's `g_EtChr`
record index (`et.rs`). Two senders reach all four:

| Function | Caller | Writes |
| --- | --- | --- |
| `op_et_bindeff_send_common_signal` (`0x007fbdf0`) | `MsEffectSendSignal` (tail call) | record flags `+0xc` \|= `bits` |
| `op_et_bindeff_send_signal` (`0x007fbef0`) | `opu_et_etc` | object signals `u16 +0xb0` \|= `bits` |

Each effect is the first record with its ID; a missing one is skipped. Both
return `eax` as the last search leaves it (the record, or the object, of
the `0x700` effect; the end of `g_EtEff` without it; the end of `g_EtChr`
for an unknown handle). The names come from Fahrenheit's Switch call graph.

## Sound cues (`op_oef2_sound_init_00800440`)

The op layer's set-up (`0x007ff960`) clears the oef2 sound state: the
32-cue ring `g_OpOef2SoundCueRing` and its arguments, the 64 words from
`0x023c36a0` (two tables of 32 slots, the second `g_OpOef2Seps`), the ring
head and count, both `g_OpOef2StreamCues` and the byte `0x023c3644`. It
then sets the curve constants `op_3d_vol_pan_get` reads, copied from
`f32` constants in the image:

| Address | Value |
| --- | --- |
| `0x023c3660`, `0x023c367c` | 32 |
| `0x023c3664` | 512 |
| `0x023c3668` (`g_OpSoundCurves`) | 127 |
| `0x023c366c`, `0x023c3678` | 96 |
| `0x023c3670`, `0x023c3688` | 64 |
| `0x023c3674` | 768 |
| `0x023c3680` | 129.0667 |
| `0x023c3684` | -0.0645833 |
| `0x023c368c` | 1/24 |
