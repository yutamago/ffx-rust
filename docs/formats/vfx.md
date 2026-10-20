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
to `dst + 0xc0`, and `pppKeShpTail3XVfCopy_00753080` (for `pppKeShpTail3XDraw`)
also `0x00c8f930..0x00c8f95f` to `dst + 0x100`; `pppKeShpTail3VfLoad_00751740` (and its byte-identical copy
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

## Model texture coordinates (`op_md_tex0_change*`)

`op_md_tex0_change_limit(packet, tex0, lo, hi)` (`0x00a69ca0`) walks a model
packet's tags (from `+0x20` to type `0xff`, as `op_md_packet_set_addr`).
Tags with type bit 2 carry a 14-bit `u` in `+8` (bits 0..13) and `v` in
`+0xc` (bits 5..18). It finds the smallest `u` and `v` over those tags and
rebases every tag with `lo <= u <= hi`: `u' = u - u_min + 0x3fff`, the same
for `v`. Results reach 0x7ffe and spill into the bit above each field, as in
the original. The PC build never reads `tex0`.
`op_md_tex0_change(packet, tex0)` (`0x00a69c80`) is the call with `lo` 0 and
`hi` 0x4000. Both return the end tag.

## Encounter grid (`opus_encount0_init`)

`opus_encount0_init(obj)` (`0x007f8480`) sets `f32 +0x20` = 16, `+0x24` = 32,
`+0x28` = 0 and lays out `u16 +6` planes of `u16 +4` rows of `u16 +2`
particles, 0x20 bytes each from `+0x30`:

| Offset | Value |
| --- | --- |
| `+0`, `+4`, `+8` | x, y, z: 0, then down by 64 per particle, row and plane |
| `+0xc`, `+0x1c` | 1 |
| `+0x10` | 2112, up by 64 per particle across the whole grid |
| `+0x14`, `+0x18` | 0 |

Each step adds 64.0 in 53 bits to the stored `f32` and rounds at the store.
Returns the plane count (0 for none) in `eax`.

## Sprite-animation commands (`opp_anmm`)

`opp_anmm` (`0x008124b0`) runs a command list per animated sprite. A
command is an `i16`: its low byte picks a handler in the table at
`0x00c49318`, the whole word is kept in the host's `u16 +0x21a`, and the
interpreter calls `handler(host, set, cmd, work)` (it returns the next
command) until one sets the wait byte `host +0x210`. `set +0x20` holds the
0x100-byte parts, `set +0x60` the oef pointers; `work u16 +0x4c` is the
sprite's part.

Flag bits 12..15 of the command say which of x, y, z (w) the vector and
colour commands read, one word each in that order. For the vector commands
`(flags >> 7) & 0xe` is an index `i`: the target is `work` + the `i16` at
`0x00c49300 + 2 i`, and x, y, z are scaled by the `i32`s at `0x00c49398 +
16 i` and 1/4096 (`fild`, rounded to `f32`, times the `f64` 1/4096).

| Slot | Address | Command |
| --- | --- | --- |
| 0 | `0x008106a0` | wait forever (`+0x210` = 0xff), returns 0 |
| 1 | `0x008106c0` | wait the command's high byte |
| 2 | `0x00810710` | jump by `i16 +2` |
| 3 | `0x008106e0` | wait `op_irandom_core(high byte)` |
| 4 | `0x008107c0` | vector set |
| 5 | `0x00810890` | vector add, each value times `2d - 3` for three VU0 `R` draws |
| 6 | `0x00810a90` | vector add |
| 7 | `0x00810720` | sprite set: texture `i16 +2` of the part's oef, `+8` = 0xff010000 with the part's byte `+0xb3`, `+0xc` = 0x100, colour 0x80808080, `+0x2c`/`+0x3c` = 0 |
| 8 | `0x00810790` | without flags, `work +8` bit 7 |
| 9 | `0x00810b70` | `i16` at `work` + `0x00c4930c[(flags >> 12) & 7]` = `+2` |
| 10 | `0x00810ba0` | the same `i16` += `op_random_per(i16 +2)` |
| 11 | `0x00810be0` | `u16` pair `+2` to `work +0x2c` (flag 0x1000) or `+0x1c` |
| 12 | `0x00810c20` | mode `work +0x4e` = `flags >> 12`; mode 2 copies `+0x10..+0x18` to `+0x40` |
| 13 | `0x00810c60` | colour bytes `work +0x1c..+0x1f` |
| 14 | `0x00810cd0` | loop count `work +0x4f` = byte `+2` |
| 15 | `0x00810cf0` | loop: count down, back by `i16 +2` while not 0 |
| 16 | `0x00810d10` | branch by `u16 +4` when `op_random_per(0x100)` < `i16 +2` |
| 17 | `0x00810d50` | fade: at `work + i16 +2` step `+0x2c` = byte `+5`, wait = `|byte +4 - byte +0x1c|` / `|step|` |
| 18 | `0x00810da0` | `work` + byte `+2` = the part's float at byte `+3` |

The random commands pass one argument to random functions that read four;
the other three are the handler's saved registers and return address, which
end up only in emulated VU0 registers (`crates/ffx-vfx/src/anmm.rs`).

## Render-data counts (`clearVFXRenderDataShp*`, `caculateVFXRenderDataShapeFieldBufferSize`)

A VFX render data (`graphicVFXGetRenderDataByInstance`) has `i32 +0x90`
parts of 0x6c bytes at `[+0x94]`. A part keeps its index count at `+4`, its
vertex count at `+8`, its index buffer at `+0x1c` (`[+0x68]` entries of 2
bytes), and from `+0x2c` one 0x1c-byte record per dynamic-geometry buffer
whose first two words are that buffer's index and vertex counts.

| Function | Effect |
| --- | --- |
| `clearVFXRenderDataShp` (`0x00713290`) | every part's counts set to 0, also in the current buffer's record (`graphicDynGeoMemManagerGetCurrentBufferId`) |
| `clearVFXRenderDataShpWithMemset` (`0x00713320`) | the same, each index buffer zeroed first |
| `caculateVFXRenderDataShapeFieldBufferSize` (`0x00712bd0`) | the first part's vertex count += 6n, index count += 4n, for `n` the `i16 +2` of shape entry `k` (`get_ptr32(*shape)`, offset `i16 +0x10 + 8k`) |

All three return 1, or 0 when the instance has no render data. They are
bookkeeping for the PC renderer's buffers, ported in `draw.rs`.

## Bone matrices (`opus_get_bone_matrix`)

`opus_get_bone_matrix(out, chr, bone)` (`0x00805430`) puts a character
element's base matrix in `out` (`Ch_ElemGetBaseMatrix`) for the opus
effects that follow bones. A bone with bit 0x8000 is an option position:
`Ch_OptposGetElement(chr, bone & 0xff, local)` returns the element it hangs
from and a 16-byte offset in the caller's local, and `out + 0x30` becomes
that offset through the matrix (`op_apply_m_w1`). Without the bit the
function also copies the emulated EE register `g_Ee_Gpr` (`0x00c8f508`, 16
bytes) to `out + 0x40`. The port keeps the local in a per-call stack buffer
in the shim.

## Character scale matrices (`opus_rt_m_chara`)

`opus_rt_m_chara(m, chr, k, angles, trans, battle)` (`0x008062c0`) builds
the matrix an opus effect uses to follow a character at its size. The
scale is the battle character's (`MsGetChrScale(chr, local)`) when `battle`
is set and `chr` (signed) is below 0x20, else `Ch_GetScaleX/Y/Z` of
`g_EtChrHandles[chr]`. It is multiplied by `Ch_GetDataScale * k * 0.001f *
(1/256)`, computed at 53 bits and stored as an `f32` (`op_xyz_scale`), and
becomes `m`'s diagonal (`op_unit_m_scale2`). Then `m` is turned by
`angles` and moved by `trans` (`op_rt_m`; `op_rot_m` or `op_trans_m` when
only one is given). Callers pass `k` = 256.0 from the overlays, or the
effect's own scale (`+0x3c`). The scale vector is a 16-byte local of the
original's 16-byte-aligned frame (`[ebp - 0x20]`); the shim keeps it in a
stack buffer.

## Freeing units by effect (`opu_unit_free_by_id`, `opu_unit_free_by_oef2`)

An opus list keeps three index lists of its units (`+0x10`, `+0x14`,
`+0x18`: `u16` unit numbers ending in a negative one). Both functions shut
units down the same way: `opus_prog_off`, `opus_draw_off` and
`yonExtMapDeleteId` for each match, then the same list entry is read again,
because `opus_prog_off` moves the rest of the list down.
`opu_unit_free_by_id(list, id)` (`0x007fde30`) looks up the first
`g_EtEff` record (`0x012a4100`, 512 records of 0x20 bytes) whose id (`+0`)
is `id`, frees it (`opu_eff_free_by_id`) and shuts down the units whose
record index (`u16 +0x14` of the 0x100-byte unit) is that record's.
`opu_unit_free_by_oef2(list, oef)` (`0x007fdf70`) works by oef2 slot
group instead (`u8 +0x1e`). Both are in `et.rs`.

## Effect units (`opu_et_unit`)

`opu_et_unit(oef, group, part, id)` (`0x007fd7f0`, 15 callers) starts a
unit of an oef2 in `g_OpusBattle`: the oef2 takes one of the 16 slots at
`+0x60` (its own or the first free; -1 without one), `opus_prog_on(b, 0)`
gives the 0x100-byte unit record, which gets the slot (`+0x1e`), its data
(`+0` = `set_ptr32` of entry `part` of `op_oef2_a_get(oef, 1, group)`), its
`g_EtEff` record (`opus_get_eff_status_index(id, slot, 1)`, index at `u16
+0x14`, layer at `+0x18`) and a unit number (`u16 +0x1a`): for a negative
`id` `opus_fullauto_no_get`, else the first `(part << 8) + 0x10 j` that no
unit of the same slot in the `i16` lists at `[0x00c88a94]` and
`[0x00c88a98]` already has. The result, in `eax` although Ghidra shows
`void`, is `slot << 16 | number`. The original also formats
"LoadedByOPU_ET_UNIT" into a 0x4c-byte stack record for `yonExtMapRegister`,
which is an empty function on PC (`et.rs`).

## CLUT cue ring (`op_oef2_tex_clut_cue_loop`, `_multi`)

Textures and CLUTs queued by `op_oef2_tex_cue` / `op_oef2_clut_cue` wait in
a 64-slot ring (masks at `0x011a03a0`, textures at `0x011a01a0`, CLUTs at
`0x011a02a0`, `u16` head and count at `0x011a04a0`/`a2`).
`op_oef2_tex_clut_cue_loop` (`0x007e4720`) loads them from the head,
counting texture bytes (`+0x10`) and 16 per CLUT entry, until 1 MiB or the
count runs out; the count is read again after every load.
`op_oef2_tex_clut_cue_multi(oef, first, last, mask)` (`0x007e4830`) queues a
range of an oef2's textures and CLUTs and then, for 25 scripted magics
(`MagicFile_GetMagicId`), records the step the range stands for in
`g_VfxPhase` (`0x00ccb9d4`), which the renderer reads (`op.rs`).

## Breaking models (`opus_mdc_break_make_VitaSafe`)

`opus_mdc_break_make_VitaSafe(mdc, out, params, view)` (`0x008059c0`) turns
a model into flying triangles: `out +4` counts them (at most 4095 per
model), and each is 0xc0 bytes from `out + 0x30`: the three corners less the
centre at `+0`, `+0x10`, `+0x20` (`w` 1, turned by the entry matrix without
translation), texture coordinates (`u16 / 4096`) at `+0x30`, `+0x34`,
`+0x40`, `+0x44`, `+0x50`, `+0x54`, the part's words at `+0x38`/`+0x3c` and
bytes at `+0x58..+0x5b`, a life `u16 +0x48` = `op_irandom_p(params +0x1e) +
0x10 + params +0x1c`, the centre at `+0x60` (through the camera times the
entry matrix, whose translation is scaled by 1000), the unit direction from
`[view +4]` at `+0x70` and its length at `+0x7c`, speeds `(a +
op_random(b)) / 256` at `+0x80`, `+0x84`, `+0xb8`, `+0xbc` and spins
`op_random` at `+0xa0..+0xa8` from `params`' `i16` pairs, and `g_Ee_Gpr`'s
r0 at `+0x90` and `+0xb4` (its `+0xb8`/`+0xbc` then taken by the speeds). `op_irandom_p` reads four arguments where one is
pushed, so three of the caller's stack words reach its VU0 scratch
registers only (`opus.rs`).

## Model-draw packets (`0x0080e160`, `0x0080e230`, `0x0080e900`)

Before `op1_mdc_draw`, the opus model units (`0x0080df10`, `0x0080e030`)
fill a 0x50-byte packet header with `0x0080e160(pk, mats, obj, unit, tex)`:
`+0` the unit's `u16 +0xc0`, `+4` `obj +0x28`, `u16 +8` the unit's `+0xc2`,
`+0x18` RGB 0x80 and the unit's `+0xcb` as A, `+0x1c` the unit's `+0xc8`
without its top byte, `+0x20`/`+0x24`/`+0x28`/`+0x2c` the matrices at
`mats + 0xc0`, `+0`, `+0x40`, `+0x80`, `+0x34` `obj +0x54`, `+0x40` the
unit's `+0xc4`, `u16 +0x4c` 0x8080; `+0xc`, `+0x10`, `+0x14`, `+0x38`,
`+0x3c` and `+0x44` are 0. It returns the model, `op_oef2_a_get(obj +0x60
[unit +0x1e], 4, tex)` (0 when `tex` is -1). `0x0080e230` is the same
without clearing `+0x14` and `+0x3c`.

`0x0080e900(pk, mats, obj, unit, nos)` then sets four lights: the colour
matrix (`pk +0x24`) and position matrix (`pk +0x28`) start as rows of
`g_Ee_Gpr`; each nonzero unit number `nos[1..=4]` names a light unit
(`opus_no_get`) whose row gets its position (`+0x90`) with `w = +0x4c *
0.0625`, and colour `+0x3c * (1/32768) * c` for R, G, B (53 bits) and
`a / 256`. `pk +0x4c` and `+0x4d` become `ftol(unit +0x30) / 2` and
`ftol(unit +0x4c) / 2`, clamped to 0..0x80.

## High range (`0x00a30000..0x00a80000`)

Ported in `crates/ffx-vfx/src/hi_md.rs`, `hi_light.rs`, `hi_draw.rs`,
and `hi_opus.rs`; the shims are
`crates/ffx-hook/src/ported/vfx_hi.rs`, the oracle tests
`crates/ffx-oracle/tests/vfx_hi.rs`.

What else lies in the range is not effect logic:

- `0x00a34000..0x00a3e000` is Iggy's GDraw back end for D3D11 (its
  allocations are tagged `gdraw_d3d1x_shared.inl`; `0x00a35780` fills the
  GDraw function table at `0x01a85828` and allocates the context
  `[0x01a858b4]`). The `op_node_*` and `op_texels_*` functions ported
  earlier are its handle cache and mip builder (state 5 is GDraw's
  user-owned handle). Middleware, left original.
- `0x00a5d120..0x00a65570` is the op debug display (`opd_*`): the
  `op_num_*` wrappers reach the digit-sprite writers only under
  `g_OpDebugMode` or `0x02322794`, and the command printers
  `0x00a64c00..0x00a64ff0` only call `op_num` (`0x007e63a0`), a bare `ret`.

### Model packets

A model list holds `u16 +4` packets, each a `ptr32` at `+0x10 + 0x10 k`.
A packet's tags start at `+0x20`: type byte `+1`, `i16` count `+2`.

- `op_md_copy(src, dst)` (`0x00a69290`) copies `op_md_packet_size(src)`
  bytes (`0x00a698c0`) 16 at a time, one word at a time from the start; a
  size of 0 goes to `dbgPrintf` (`"op_md_copy error !!!"`, `0x00b7c3b4`)
  and `debug_exit_trace(1)`, both bare `ret`s here.
- `op_md_list_count10_00a6a2b0(list)` sums the counts of each packet's
  leading tags of type 0x10, stepping `0x20 + 0x1c n`
  (`opp_0437_cmd_a6dbe0` kind 0x4000 sizes its buffer with it).
- `opus_0582_break_bytes_00a72310(list)` is `0x40 + 0x300` per element
  of every tag up to the end tag (type 0xff), with the same step
  (`opp_0582_cmd_a70910` kind 0x4000).

- `op_md_tex0_change_range_00a69ca0(packet, _, lo, hi)` (`0x00a69ca0`;
  `op_md_tex0_change` at `0x00a69c80` passes `0..0x4000`) rebases the
  `TEX0` (`+8`, 64 bits) of the textured tags (type bit 1): with `m` the
  smallest `TBP0 - 0x3fff` (bits 0..13) over them and `n` the same for
  `CBP` (bits 37..50, `+0xc >> 5`), each tag whose `TBP0` is in `lo..=hi`
  gets `TBP0 - m` and `CBP - n` (16-bit; never negative, so they come out
  0x3fff above the smallest, and the `TBP0` overflows into bit 14). Tags
  step `0x10 + n x g_OpMdTagSizes[type & 7]` (`0x00c88aa8`) to the end tag
  (0xff).
- `op_md_vn_index(packet, out, table)` (`0x00a69f10`): `[packet] |= 8`,
  `packet +0x1c = out - packet`; the `i16 +0x12` 64-byte lists in `table`
  are emptied and filled by `op_md_vn_index_sub` from every element
  (0x1c bytes, tags as above); `out` gets `u16 +0x14`, then from `+0x10`
  each list's count and entries. Returns the end rounded up to 16.

- `op_md_list_bounds_00a6a0c0(list, m, min, max)` (`0x00a6a0c0`; from
  `0x00a6e1b0` and `opus_0582_break_make`): the box round every vertex
  (16 bytes from `packet + [packet + 8]`, `i16 +0x12` of them) under
  `op_mul_m(m, packet matrix)`, through `FFXVu0ApplyMatrix`; `min`/`max`
  start at `+-16777216.0` and widen on `<` (never on a NaN). Each
  packet's matrix (the `ptr32` after the packet's) has its translation
  multiplied by 1000 for the product and divided by 1000 after, as `f32`
  both times, so it keeps both roundings.

### Vertex lights

Three set-ups before a model draw, from `0x00a6d380` on. The fog
vectors `g_OpFog + 0x10` and `+0x40` (`0x011a0020`, `0x011a0050`) go
through the EE's `v0` (`0x00c8f528`; the first leaves its `x` alone) into
`work +0x280` and `+0x290`. Each "light" matrix is a copy of `[model
+0x2c]` with `x` and `y` of rows 0..2 negated (`fchs`).

- `op_vertex_normal_point44_ffx(obj, model, work, chr)` (`0x00a6d380`;
  16-aligned frame): the model matrix `[model +0x28]` becomes light x
  inverse(`[model +0x2c]`) x it (`sceVu0InversMatrix4x4`, two
  `FFXVu0MulMatrix`); `op_ffxlight_to_omdlight(w + 0x80, w + 0xc0, w +
  0x100)` and `op_matrix_light4_vector(w + 0xc0, w + 0xc0, light)` with `w
  = [work +0x390]`; the fog vectors. With a character: `Ch_DrawCalcShade`,
  the colour scale `work +0x370..+0x37c` x 1/128 (`f32` each) and
  `graphicPS2LightingSetupEffect(instance, name[0] == 'w', matrix, [model
  +0x24], f32 byte +0x4d, w + 0xc0, w + 0x80, f32 byte +0x4c, shade,
  scale, 0)`, whose `eax` (0) it returns; 0 without one.
- `op_vertex_normal_point44_ffx_2` (`0x00a6d670`, same arguments): only
  with a character; the lights are locals
  (`op_ffxlight_to_omdlight_0279`), no fog vectors, the scale is (1, 1,
  0.5, `w`) for magic 0x245 (`MagicFile_GetMagicId`), and the last
  argument 1.
- `op_0437_vertex_normal_point44_ffx(packet, model, work, base, mul1,
  mul0, slot)` (`0x00a6d890`): the vertex colours into
  `g_MagicBuffersA[slot]` (`0x022fb3e0`; made on first use,
  `user_aligned_malloc(i16 packet +0x14 x 16, 16)`). Lights as locals
  (`op_ffxlight_to_omdlight`, `op_matrix_light4_vector` with `[model
  +0x2c]`), the fog vectors; then the `i16` stream at `packet + [packet
  +0x1c] + 0x10`, `i16 packet +0x12` groups of a count and that many
  vertex numbers `v`, `i = (v << 2) as i16`: the normal `-[model
  +0x3c][i..i+3]` (w 1) through the directions (`FFXVu0ApplyMatrix`),
  negatives to +0, w 1, through the colours; per channel negatives to +0,
  `c = f32(c x f32(byte +0x4c / 128))`, `buffer[i + k] = f32((c x
  mul0[k] x mul1[k] + base[k]) / 255)` (`f64` 255 at `0x00b170c0`).
  Returns `packet` without groups, else the count. It also normalises
  rows 0..2 of `[model +0x2c]` (`_CIsqrt`) into a local nothing reads.

### oef2 entry links

`op_oef2_entry_link_00a6a320(e, i)` (`0x00a6a320`, from `opu_part_init`
kind 0x2000): `t = ptr32(e + 0x14)`. Below `0x100000`, `t` is an offset
from `e` and so is the link `ptr32(e + t + 32 i + 0x1c)` (0 when null);
otherwise the link is the absolute `ptr32(t + 32 i + 0x1c)`, 0 when it
is `e` itself.

### Lights

`op_normal_light44_ffx(obj, model, work)` (`0x00a6cf10`) fills the work
area `w = [work + 0x390]`: `op_ffxlight_to_omdlight(w + 0x80, w + 0xc0)`,
then `op_matrix_light4_vector(w + 0x40, model +0x28, model +0x2c)` and
`op_matrix_light4_vector(w + 0xc0, w + 0xc0, model +0x2c)`. Its `eax` is
the last copy's last word, the light matrix's `w` 0.

`op_ffxlight_to_omdlight_0279(colours, dirs)` (`0x00a691c0`) is
`op_ffxlight_to_omdlight` with the ambient colour (`Sg_LightGetColor(3)`)
in `colours` row 3 instead of `r0`. `op_ffxambient_to_omdambient()`
(`0x00a68ff0`) turns the ambient colour into RGBA bytes (`x 256` as
`f32`, above 255 is 255, below 0 is 0, else truncated) and leaves them in
`eax`; its callers drop them.

### Refraction coordinates

`op_refrection_st_float(obj, model, _, out, scale, du, dv)` (`0x00a6cfb0`;
from `op_md_ref_nolight`, `opus_0279_draw`, `0x00a705f0`): `u` is the
unit matrix with rows 0..2 `FFXVu0Normalize`d from `[model +0x2c]`, `s`
= `op_unit_m_scale(f32(f32(scale) / 4096))` x `u` (`op_mul_m(s, u, s)`).
With `ku = 0x1000 / du` and `kv = 0x1000 / dv` (`idiv`: 0 faults, and so
does `ftol`'s `0x80000000 % -1`), for each of the `i16 obj +0x12` groups:
`p = s x [model +0x38][i]`, `fu = f32((ftol(p.x) % du) x ku) / 4096`, `fv`
the same from `p.y` and `dv`. Its vertices are `i` itself when `[obj
+0x1c]` is 0, else the next count and vertex numbers of the `i16` stream
at `obj + 0x10 + [obj +0x1c]`. Each vertex `v`: `t = u x ([model
+0x3c][v], w 1)` (`FFXVu0ApplyMatrix`), `out[v].x = f32(f32((t.x + 3) x
0.5) + fu)` and `.y` the same (16 bytes a vertex). `eax` is the group
count reread at the end (`obj` or 0 with none); no caller reads it.

### Model draw set-ups

`op1_md_draw` (`0x00a65750`) picks a path by the packet's flags
(`0x2000`: `0x00a6bae0`; 4: `0x00a6bfe0` with any of `0x300`, else
`0x00a6b710`; then `op_md_one8`/`_fog`, `0x00a6c3d0`, `0x00a6acb0`). Each
path takes `0x1800` bytes of the op scratch stack (`w` = the old top's
mark; the index is the sign-extended `g_OpSprTop` + 1, unwrapped) and pops
them at the end; the 0x50-byte init block goes to `w +0x300` 16 bytes at
a time through the EE's `v0` (`0x00c8f528`, all four words), and `w
+0x338`/`+0x33c` default to `obj + [obj +8]`/`obj + [obj +0xc]` when 0.
Tags start at `obj + [obj +4]`, type `i8 +1`. The draw paths spin while
`g_GsRegisterSink` (`0x023056e8`) has bit 8 set and return `[w +0x334]`.

- `op_md_chara_path3(obj, init, chr)` (`0x00a6a3a0`; from `opu_mdc`,
  `op1_mdc_draw`, `0x00a693b0`): with `a` the RGBA bytes of
  `op_ffxambient_to_omdambient()` and `t = f32(byte w +0x34c / 128)`, the
  colour `w +0x370 + 4k` = `byte w +0x31c + k x (1 - t) + a[k] x t`
  (exact). `w +0x390` = `(i16 obj +0x12 << 5) + [w +0x304]`, `+0x394` =
  `[w +0x304]`; init flag `0x40`: `op_vertex_normal_point44_ffx(obj, w +
  0x300, w, chr)`, else with `i16 obj +0x14` and no flag `0x20`
  `op_normal_light44_ffx(obj, w + 0x300, w)`. Returns 0.
- `op_md_x64_00a6b710(obj, init)` (`0x00a6b710`): `p =
  op_md_spr_init_x64(obj, init, w)`; the colour from the signed bytes `p
  +0x1c..`; `w +0x390` = `(i16 obj +0x12 << 4) + [p +4]`. Lights: no flag
  `0x40` and `i16 obj +0x14` nonzero: `op_matrix_light4_vector` into a
  local nothing reads, or with flag `0x20` `op_md_white_vectors`; flag
  `0x40`: `opc` (a bare `ret`) when neither `[obj]` nor `[p]` has `0x400`,
  else `0x00a6cd40` (a bare `ret`) unless `[obj]` has 8. Tags (byte table
  `0x00a6ba98`, jumps `0x00a6ba7c`): types 0..7 set `w +0x1f0` 0x8001 and
  `w +0x1f8` (0/1 `0x414141`, 4/5 `0x41414141`, 2/3 and 6/7 `0x12412412`
  with `w +0x120` = `u16 p +0xc + [tag +8]`, `w +0x124` = `(u16 p +0xe << 5)
  + [tag +0xc]`) and step 0x10; 0x20 and 0x22 step 0x20 and act only when
  `i16 tag +2` is nonzero (0x22 also `w +0x100` 0x70000006, `+0x110`
  0x8001, `+0x150` `[tag +0x18]`, `+0x160` `[tag +0x10]` or `w +0x120`,
  `+0x168` 6, then `opc2()`, a bare `ret`); other types end.
- `op_md_x64_fog_00a6bfe0(obj, init)` (`0x00a6bfe0`): the colour from the
  signed bytes `w +0x31c..`; `w +0x394` = `[w +0x304]`, `+0x39c` = `(i16
  obj +0x12 << 4) + [w +0x304]`; the GS words as `op_md_spr_init_x64`
  writes them except `+0x118` (0xeeee here) and `+0x150` (only 0 for
  `[obj]` flag 4, else 5); the 16 bytes at `+0x140` from
  `g_OpMdGsWordsB_011a07f0` with init flag 8, else
  `g_OpMdGsWordsA_011a0740`. Tags (jumps `0x00a6c3b0`): 0, 2, 4, 6 as
  above, step 0x10; 1, 3, 5 and anything else end.
- `op_md_persp_00a6bae0(obj, init)` (`0x00a6bae0`, flag `0x2000`):
  as `0x00a6bfe0` (the same GS words) plus a projection at `w +0x200`:
  `op_unit_m`, then `[2][2]` = 1.00003052 and `[3][2]` = -2.00003052
  (`f32` at `0x00b7c424`), `[2][3]` 1 and `[3][3]` 0, times `[w +0x330]`
  (`op_mul_m(m, m, [w +0x330])`); the 64 bytes at `[w +0x320]` go to `w
  +0x240`; `w +0x390` = `[w +0x304]`, `+0x394`/`+0x398` = `(i16 obj
  +0x12 << 4 / 5) + [w +0x304]`. Tags (jumps `0x00a6bfb0`): 0, 1, 4, 5
  set `w +0x1f0`/`+0x280` 0x8000 and `+0x1f8`/`+0x288` 0x41; 2, 3, 6, 7
  the same with 0x412 and the texture words; type 8 and up (or negative)
  ends.
- `op_md_flag80_fog_00a6c3d0(obj, init)` (`0x00a6c3d0`, flag `0x80` with
  any of `0x300`): returns `[init +0x34]` at once when
  `op_md_packet_size_check(obj, [init +0x34], 1)` (`0x00a698b0`, `xor
  eax, eax; ret`) is nonzero, which it never is. Pushes `0x2800` bytes,
  rounds `w +0x334` up to 64 (`& 0x0fffffc0`), the colour and `w
  +0x394`/`+0x39c` as `0x00a6bfe0`; GS words `+0x110` 0x8001, `+0x118`
  0xeeeee, `+0x128` 6, `+0x130`/`+0x134` as above, `+0x138` 0x42,
  `+0x140` 16 bytes from `g_OpMdGsWordsC_011a0820`, `+0x150` 16 from
  `g_OpMdGsWordsA_011a0740`, `+0x160` 0/5 by `[obj]` flag 4, `+0x168` 8,
  `+0x180` the address `0x012a8da8`. Tags (jumps `0x00a6c88c`): 0 and 4
  set `w +0x100` 0x20000009/0x2000000b and `+0x1f8` 0x414141ee; 2 and 6
  0x2000000d/0x20000010, 0x12412eee and the texture words; each sets `w
  +0x1f0` 0x8001 and spins on the GS; 1, 3, 5 and anything else end.
  Returns `[w +0x334] + 0x40`.

### `opus_0437` dots

The dot buffer `b`: `b[0]` the entry count, `b[1]` the next entry
(counted up, taken modulo the count), 32-byte entries from `b + 0x10`.

- `opus_0437_dot_init_00a6e540(b, n)` (`0x00a6e540`, kind 0 of
  `opp_0437_cmd_a6dbe0`): both halves of each entry the EE's `r0`, then
  `u16 +0x1c` 0x20, `u16 +0xc` 0x8080, `u8 +0xf` 0x80 and `u8 +0xe`
  `0x40 + op_random_per_call_007e5d40(0x40)`. `op_random_per` reads four
  words; the three past the pushed one are the caller's `edi`, `esi`,
  `ebx` the original saved on entry, and only reach VU0 registers
  (`0x00c8f974..0x00c8f97c`). The hook passes 0 for them.
- `opus_0437_dot_set(b, pos, colour)` (`0x00a6e5d0`, kind 0x1000): the
  entry's first half `op_xyz_scale(e, pos, 1.0)`, its second the colour
  (through the EE's `v0`, `0x00c8f528`), `u16 +0x1c` cleared. An empty
  buffer divides by zero in the original.
- `opus_0437_dot_draw(b)` (`0x00a6e2e0`, kind 0x2000) still builds the
  PS2 packet in a `0x1100`-byte `op_spr_alloc` scratch and sends its DMA
  block (`+0x1000`, the `TEST`/`ALPHA` writes `0x48`, `0x42`) through
  `sceGsSyncPath`/`sceDmaSend`; two quadwords at `+0` are copied to
  `+0x800` through `v0`. Then three line instances `(1000 + i, 0x1b5)`
  (`createVFXRenderDataBufferLine(id, 3000, 2)` when missing, then
  `+0xb8` = 1 and `graphicVFXSetup`) each get a segment buffer, a unit
  matrix, `graphicVFXSetVisible` and `graphicVFXPath3Update`; any missing
  piece stops the draw. With all three and a valid fill buffer, the fill
  request (`JobSchedule_pppPart_getFillBufferRequest(b[0])`) gets kind 4,
  the three segments, `b + 0x10` and `b[0]`.

### `opus_0437` polygons

`opus_0437_poly_init_00a6e1b0(b, list, m, k)` (`0x00a6e1b0`,
`opp_0437_cmd_a6dbe0` kind 0x4000) lays out the buffer
`opus_0437_poly_check` (`0x00a6ed40`) reads: `u16 +0` `k`, `u16 +2` 0,
`u16 +4` `op_md_list_count10(list)`; `k` 0x60-byte entries from `+0x30`
(`u16 +0xc` 0xffff each), `ptr32 +0xc` their end, from which
`op_md_list_count10(list)` `u16`s are 0xffff; the box
`op_md_list_bounds(list, m, b + 0x10, b + 0x20)` (its `eax` returned).

### `opus_0582` dots

`opus_0582_dot_set_00a73740(b, rec)` (`0x00a73740`,
`opp_0582_cmd_a70910` kind 0x2000) fills the next entry (`b[1] % b[0]`)
of the dot buffer `opus_0582_dot_draw` reads, from the part `rec`: radii
`+0x40`, `+0x48` / 256, height `+0x94`, scale `+0x3c` / 16, angle
`ftol(+0x30)`, step `ftol(ftol(+0x34) / 256)` (the integer back through
an `f32`), spread `ftol(+0x38)` (0 when negative), life `ftol(+0x4c) +
16`, colour `+0xc8`, fade-in 8. It returns the colour.

### Map draw

`opp_map_draw_init_00a753f0(b, pos)` (`0x00a753f0`, kind 0 of
`opp_map_draw_a74c60`, which allocates `0x27e20` bytes): `[b]` = 0 and
0x740 0x58-byte entries from `b + 0x20`. With `X = f32(pos.x + 2048)`, `Y
= f32(pos.y + 2048)`, `x0 = X - 256`, `y0 = Y - 208` (unrounded until
each store) and `k` = 1/512, the entries go round the rectangle (`+0x10`
x, `+0x14` y, `+0x18` 0, `+0x38`/`+0x3c` uv): 0x200 along the bottom (`x0
+ i`, `y0`, uv `(i k, 0)`), 0x1a0 up the right (`X + 256`, `y0 + i`, uv
`(1, i k)`), 0x200 back along the top (`i` from 0x1ff down: `x0 + i`, `Y +
208`, uv `(i k, 0.8125)`) and 0x1a0 down the left (`i` from 0x19f: `x0`,
`y0 + i`), whose uv goes to the next entry's `+0x38`/`+0x3c` (the left
side's first entry keeps whatever its uv held, and the last pair lands 8
bytes past the buffer). Then each entry gets `+0` `X`, `+4` `Y`, `+8` 0,
`+0x20`/`+0x24` 0x80808080, `+0x30` `f32((X - 1792) k)`, `+0x34` `f32(k (Y
- 1840))`, `+0x40..+0x48` 0, an `op_random_p(0.001)` whose value is
dropped (it only advances the generator), `+0x4c` 0.01, `+0x50`/`+0x54`
0. The `eax` the last `op_random_p` left is its result; no caller reads
it.
