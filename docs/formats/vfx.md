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
steps each frame unless `g_pppStopStatus` (`0x0230fd34`) is set:
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
at this seam. Under the junk-read ruling (2026-10-24, 2026-10-27) the
port is hooked with those zeroes (`ported/m5_vfx.rs`, M5 left8); no
coverage run has reached it, so whether a flare in the original showed
(and now stays hidden on its measuring frames) is still open.

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

If the flare shows in the original, the port needs those values from
somewhere other than zeroed locals (`FFX_HOOK_DISABLE=pppKeLnsLpTOtDraw`
turns the hook off for the comparison).

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

### Camera draws (`pppDrawMdlCameraDraw`, `pppDrawMdlCameraLoopDraw`)

Ported in `crates/ffx-vfx/src/mdl_cam.rs`. Both keep their state in work
area 2 (`obj + 0xa0 + [[prog + 0xc] + 8]`), set up on the first frame by
`pppDrawMdlCameraTex_0073dfe0` (while `+0x88` is a null `ptr32`) or
`pppDrawMdlCameraLoopInit_007400f0` (while `+0`), step it unless
`g_pppStopStatus` and animate a texture (`Yn_FpTrxTexGet`, next frame
by `0x00919bd0` once a byte counter reaches the key's limit; the frame
wraps at the work area's count byte). The colour, request and submit are
the other model draws' (`Camera` never takes path 3, `CameraLoop` does
for flag 4).

- `pppDrawMdlCameraDraw` (`0x0073d930`): two `[pos, speed, accel]`
  channels at `+0x70`/`+0x7c` (or, with byte `+0x8f`, the positions moved
  by `+0x60`/`+0x64`), wrapped with `_CIfmod(x, 32768)`, give the
  request's `+0x10`/`+0x14` in 16.16. `op_rt_m_yxz(w, M, angles, vf00)`
  writes the rotation into the work area's first 0x40 bytes: `M` is a
  unit matrix moved to `w +0x40` plus the inverse camera's position, the
  angles `w +0x50..+0x58` times 4096/2pi (the double at `0x00b51590`).
  The request's `+0x30` is camera x `w` x the object's matrix, `+0x20` a
  copy of it, `+0x2c` the object's matrix; flags from key bytes
  `+0x24..+0x29`, the blend from `+0x22`. `pppVu0Length_3` runs on the
  object's three rows and its results are dropped.
- `pppDrawMdlCameraLoopDraw` (`0x0073fa50`): channels at `+4`/`+0x10`
  (only with byte `+0x1c`); with key byte `+0x20` the request's matrices
  are the world through the inverse camera (the world copy's position
  replaced by the inverse camera's, `FFXVu0CopyVectorXYZ`), else copies
  of the object's `+0x50` and `+0x10`; flags from key byte `+0x26` and
  the model's first byte, `0x200`/`0x100` for `+0x27`/`+0x28`, and for
  `+0x29` `0x2000` with `g_pppViewMatrix x obj +0x10` in `+0x30`; the
  blend from `+0x24`.

### Sea draw (`pppDrawMdlSeaDraw`)

`pppDrawMdlSeaDraw` (`0x0073abe0`, `crates/ffx-vfx/src/mdl_cam.rs`)
draws only on 14 levels (`graphicLevelGetNameHash` 0x199, 0x17e6,
0x24c9, 0xa95, 0x245b, 0xa27, 0x170a, 0xe58, 0x1b3b, 0xb3, 0x1acd, 0x99,
0xe6c, 0xf48). Work area 2 is set up by `pppDrawMdlSeaDrawSetup` while
its `+0x70` is a null `ptr32`. Its channels `+0x50`/`+0x5c` step like
`CameraLoop`'s (on the key's frame its `+8..+0x1c` are added to all six
words). With byte `+0x77` the model's vertices (`i16` triples at its
`+8` offset, `i16 +0x12` of them, a flag byte each in `ptr32 +0x6c`)
are rebuilt from the rest positions (`ptr32 +0x68`): with `s = f32(x +
z)`, phase `k` is `ftol((w[+0x40 + 4k] + s) * key[+0x40 + 4k])`, and
coordinate `k` gets `ftol(g_pppSinTable[phase >> 4 & 0xfff] * key[+0x20
+ 4k])` (16-bit add); then the phases move by key `+0x30..+0x38`. The
texture animation counts in `+0x74` up to key byte `+0x51` (cleared
even when `Yn_FpTrxTexGet(key[+0x50])` gives none), steps the frame
`+0x75` (wrapping at `+0x76`), formats `trxtex_<frame>_<a>_<b>.dds.phyre`
with `a`, `b` from `pppStoreComputed_0091af10` (the CRT's `sprintf`) and
hands the record `graphicVFXIsTextureLoaded` fills to
`graphicVFXUpdateTexture`. The colour inlines the `pextlh` steps; the
request (`ebp - 0x2e0`) has flags 4 (`0x14` with key byte `+0x54`), the
object's `+0x50`/`+0x10` matrices, and after an update
`graphicDynamicGeometryUpdateTexture`.

### Point-light draws (`pppNeiDrawMdl*PointLightDraw`)

`pppNeiDrawMdlPointLightDraw` (`0x0074a1e0`), `...SemiPointLightDraw`
(`0x0074a850`) and `...TsPointLightDraw` (`0x0074ae70`) are
`pppDrawMdlDraw`, `pppDrawMdlSemiDraw` and `pppDrawMdlTsDraw` without the
level checks, with flag `0x40` added (`0xc0`/`0x4c`, the `Semi` one
`0xc0`/`0xd0`) and a light: for a target (object `n`, field) at key `+0x28`
(`Semi` `+0x2c`, `Ts` `+0x40`; `n` -1 for none), `t` = the current part's
object `n` (`+0x3c` records, data at `+4` through `get_ptr32`) plus the
field; the request's bytes `+0x4d` and `+0x4c` get `t +0xa8` and `t +0xb4`
times 128 (`fistp` truncating, the low byte), and four references (key
`+8`, `Semi` `+0xc`, `Ts` `+0x20`, 8 bytes each) are resolved by
`0x0074a760` (copies `0x0074ad80`, `0x0074b540`) into the request's tables:
the position at `0x0230fda0 + 0x10 k` (three floats of `ptr32(t' +0xac)`
and `t' +0xa8`), the rotation at `0x0230fd60 + 0x10 k` (four `i16` of
`ptr32(t' +0xb0)` / 16384).

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

## Tail draws (`pppKeShpTail*Draw`)

Ported in `crates/ffx-vfx/src/tail.rs`. A tail object records its path in
work area 0 (`obj + 0xa0 + [[ctrl + 0xc]]`): ring size byte `+0`, head
byte `+1`, the shape's frame time `u16 +2` and frame `u16 +4`, and the
ring of 16-byte positions from `+0x10`. The draw lays `n` sprites (key
`u16 +0x20`) of the key's shape (`+4`, `0xffff` none) backwards along the
ring from the head, `key +0x1c` apart, until the count, the length or the
ring runs out (the walk stops when it comes back to the head). A spacing
of exactly 0 sets `n` to 0, which the loops' `dec; je` read as 2^32: only
the lengths end them then.

| Draw | Address |
| --- | --- |
| `pppKeShpTailDraw` | `0x0074c570` |
| `pppKeShpTailXDraw` | `0x0074d570` |
| `pppKeShpTail2Draw` | `0x0074e570` |
| `pppKeShpTail2XDraw` | `0x0074f5b0` |

- Colour: from work area 1's four `i16` to the key's (`+0x10`), each
  `(c * u16 part[+0x70 + 2k]) >> 19` as `f32`, in `n - 1` steps
  (`FFXVu0DivVector` by `f32(n - 1)`); the scale from 1 down by `(1 -
  key[+0x18]) / (n - 1)`, the length left down by spacing times that.
- The `2` draws: colours from key bytes `+0x14..+0x17` to `+0x18..+0x1b`,
  each `(b * u16 part[+0x70 + 2k]) >> 12`, the alphas times work area 1's
  `i16 +6` times the `f64` 2^-14 at `0x00b51550`; the scale from key `+0xc`
  to `+0x10` (step `(s0 - key[+0x10]) / (n - 1)`), the length's step the
  spacing times `step / s0` (unrounded); the first sprite only with key
  byte `+0x22` (else the loop starts with a step). `pppKeShpTail2Draw`
  alone turns a zero divisor into a zero step and triples the vertex and
  index counts it asks the dynamic-geometry buffer for.
- Request (at `ebp - 0x120`, shape field): flags `+2` 0x80 (`X`: `| 0x800`
  with key byte `+0x22`, `| 0x400` with `+0x23`; `2X` one byte up), `+4` the colour
  (`pppFtoiRgba`), `+0x10` the matrix: the object's (`+0x10`) rows 0 and
  1 times the part's `+0x60`/`+0x64` and the scale (`Tail` rounds the
  part's product into the copy once; `X` rounds part times scale per
  sprite, then times the row; `Tail2` as `Tail`, `Tail2X` as `X`), row 2
  unit, row 3 the point through `g_pppViewMatrix`; `+0x14`
  `g_PppPacketPos`; `+0x1c` 0 (`X`: work area 3 with key byte `+0x24`,
  `2X` `+0x25`).
- Render data: without it `InitVFXRenderDataShapeField` (alpha from the
  frame's data, `X` from frame 0's, and `X` then draws with frame 0's
  data); with it the buffer is sized per sprite and the draw ends when
  `graphicDynGeoMemManagerGetBufferForSegment` gives none.
- Segment lengths after the first (`KeVu0VecLen`) are `_CIsqrt` of the
  `f32` sum of squares; comparisons treat NaN as "go on".
- Last, unless `g_pppStopStatus`: the frame time steps by key `u16 +8`; at
  the frame's length (`i16 +0x12` of its 8-byte entry at `set + 0x10`)
  the next frame, and past the set's count (`i16 +6`) frame 0 when the
  entry's byte `+0x14` has bit 7, else the last frame held.

## `Tail3` draws (`pppKeShpTail3Draw`, `pppKeShpTail3XDraw`)

Ported in `crates/ffx-vfx/src/tail.rs` (`ppp_ke_shp_tail3_draw`, one
function for both, the differences in `Tail3`). Work area 0 holds two
colours (four `i16` each at `+0` and `+8`), a ring of 28 16-byte points
from `+0x30` (head byte `+0x1f8`; the walk goes `head + 1, ...` and wraps
after entry 27), a time `i32 +0x1f0` (stepped by key `i32 +8` unless
`g_pppStopStatus`) and a random seed `u16 +0x1f6`.

| Draw | Address | Helpers |
| --- | --- | --- |
| `pppKeShpTail3Draw` | `0x00750620` | `0x007501d0` sub, `0x00750130` ftoi, `0x00751740` load, `0x00751680` copy |
| `pppKeShpTail3XDraw` | `0x00751fc0` | `0x007519d0`, `0x00751930`, `0x00751ab0`, `0x00753080` |

- `n` = key byte `+0x1c`; colours `(c * u16 part[+0x70 + 2k])` times the
  `f64` 2^-19 at `0x00b51650` (`X`: the integer `>> 19`), the alphas times
  work area 1's `i16 +6` / 2^14; the size from key `+0xc` to `+0x10`, the
  spacing key `+0x18` (spacing times step / start, unrounded; a spacing of
  exactly 0 sets `n` to 0 as in the other tails).
- Random numbers: `r = (r * 0x80d + 7) & 0xffff` from the seed, once per
  sprite (the sizing pass runs the same sequence). The sprite's shape
  frame is `((time + r) / len) % frames` (`div` on `u32`, `len` = `i16 set
  +0x12` and `frames` = `i16 set +6` sign-extended; a zero divisor faults
  in the original), its size `size * f32(1 - f32(r / 65535.0) * key
  +0x14)`, and with key `i16 +0x1e` its rows 0 and 1 turn by `r` through
  `g_pppSinTable` (`[(r >> 4) & 0xfff]`, cosine `(r + 0x4000)`).
- The matrix is built in the emulated VU0 registers (`0x00c8f790` the
  accumulator, `0x00c8f7b0..0x00c8f7ef` the sprite's matrix, the part's
  scales at `0x00c8f7f0`, the view matrix at `0x00c8f830`, segment
  `B`/`D`/`A`/point at `0x00c8f930..0x00c8f96f`) and copied to the
  `op_spr_alloc(0x180)` block's `+0xc0`, which the request points at.
  Row 2 (`0x00c8f7d0`) is whatever the register holds.
- Key byte `+0x1d` draws the first sprite at the head. `X`: no stop or
  skip-frame gates at the start (but shape `0xffff` ends it), request
  flags from key bytes `+0x50..+0x52` (`0x880`, `| 0x400`, work area 3 as
  the second matrix), the render data asked for at each use, one buffer
  per segment (`[rd + 0x90]` of them, `0x6c` apart from `[rd + 0x94]`),
  and the registers reloaded from the block after each sprite.
- When `graphicDynGeoMemManagerGetBufferForSegment` gives none, the draw
  returns at once without `op_spr_free`.
- Stale stack: `g_Vu0_VfMove`'s `y`, `z`, `w` and `0x00c8f8f8`,
  `0x00c8f8fc` come from frame words the draws never write first (plain:
  `ebp - 0x10` (the last turn's cosine once a sprite turned), `- 0xc`,
  `- 0x8`; `X`: `ebp - 0x16c`, `- 0x168`, `- 0x164`, set by the walk along
  the ring). They are scratch registers the draw overwrites before
  reading; the port's shim reads 0 there.

## `pppKeShpTailPhtDraw` (`0x00754300`)

Ported in `crates/ffx-vfx/src/tail.rs` (`ppp_ke_shp_tail_pht_draw`). The
`Tail3` scheme on a 27-entry ring (work area 0 `+0x30`, head `+0x1f4`,
time `i32 +0x1ec`, seed `u16 +0x1f2`; key count byte `+0x60`, `+0x61`
first sprite at the head, `+0x62` turn), with these differences:

- Colours are work area 0's `i16 >> 7` (arithmetic, as `f32`); the alphas
  times work area 1's fourth `>> 7` times the `f64` 1/128 at `0x00b170b8`.
- The sprite's matrix is a frame local (`ebp - 0x98`, unit to start): the
  object's rows 0 and 1 times `f32(part[+0x60] * size * share)` and
  `f32(size * part[+0x64] * share)`; the turn multiplies it
  (`vu_mul_matrix_00753670`) by a z rotation `[c, s; -s, c]`.
- Each sprite also fills a distortion-sprite record at `ebp - 0x160` for
  `KeDttSpr_MkGifPk_Pre` and `KeDttSpr_MkGifPk`: `+0` the point through
  `g_pppViewMatrix`, `+0x10` work area 1's colour (`>> 7`) with `+0x1c` the
  sprite's alpha, `+0x20`/`+0x28` `g_pppScreenMatrix`, `+0x24` the packet
  position `+ 0x10`, `+0x30` `(0, -area0[+0x1e0], 0, 0)` through work
  area 2's matrix with `+0x3c` key byte `+0x64`, `+0x40` the frame
  buffer's TEX0 (`Sg_GetDBuffDC`, the half by key byte `+0x63` plus the
  buffer index; `((psm | 0x2a80) << 6 | fbw) << 14 | tbp << 5` from
  `[dc + 0x60]`), `+0x48` `[dc + 0x84]` sign-extended, `+0x50`/`+0x54`
  `[dc + 0x78]`/`[dc + 0x7c]`, `+0x58`/`+0x5c` key `+0x28`/`+0x2c` times
  the part's scales, the size and the share. A nonzero `KeDttSpr_MkGifPk`
  (never on PC) puts the packet into the ordering table at depth
  `record[+0x2c] / 12288`, a word the draw never writes.
- No `op_spr_alloc` block, no stop or skip gates; the time always steps.

## `pppKeMdlBmpDraw` (`0x007490d0`)

Ported in `crates/ffx-vfx/src/mdl_cam.rs`; the drawing itself is
`pppKeMdlBmpDraw_Body_00748b00` (`draw.rs`).

- First pass (`obj +0xc` == key `+0` == 0, not `g_pppStopStatus`; both
  shapes key `+0x24`/`+0x28` not `0xffff`): an `op_spr_alloc(0x50)` block;
  the vertex set `key[+0x24]` (`+[+8]`, `i16` x, y, z per 6 bytes) and the
  triangle records of `key[+0x28]` (`+[+4]`; kind byte `+1` 2 with `i16 +2`
  triangles of 32 bytes after a 0x10-byte head, until kind 0xff; any other
  kind frees the block and returns at once; a negative count walks back
  and never ends, in both). Per triangle, `u16` points `(x, y)` at `+0x14`,
  `+0x18`, `+0x1c` and vertex indices at `+0xc`, `+0xe`, `+0x10`: the
  longest of `|x2 - x1|`, `|x0 - x2|`, `|x1 - x0|` (ties to the first two in
  that order, the third only when strictly longer) picks the corner
  order; the block gets the edge's x and y differences as `f32` (`+0`,
  `+4`, `+0x10`, `+0x18`), the slope term `dx * dy / x` (`+0x14`), the
  three vertices as `i32` (`+0x20`, `+0x30`, `+0x40`), and the triangle's
  first 12 bytes become the first vertex's three `i32` (through `fld`/`fstp`,
  so the bits are copied). Triangle number `key[+0x3c]` (counted down)
  goes to `dbgPrintf` (format `0x00b51618`/`20`/`28` by the case, then
  `0x00b51630` with the three words as doubles).
- Every frame with a target object (`key[+0x2c]` not -1: the part's record
  `[part+0x3c] + 16k + 4`, then `+0xa0 + key[+0x30]`, not
  `g_pppRandTarget`): work area 1's colour `>> 7` times 1/128 (`f32` at
  `0x00b51640`), `g_pppWorldMatrix` times the object's matrix, the
  direction from its position to the target through the inverse
  (normalized before and after), and the body's block at `ebp - 0x100`:
  matrix, `obj + 0x50`, packet position, the two shape sets, `+0x18`/`+0x1c`
  the colour, `+0x20` the direction, `+0x24` `key[+0x3c]`.
  `g_PppPacketPos` is the block's `+8` after the body.

## `pppDrawMdlInfDraw` (`0x0073b9b0`)

Ported in `crates/ffx-vfx/src/mdl_cam.rs` (`ppp_draw_mdl_inf_draw`): a
model drawn on a grid of tiles round the object, its vertices waved.

- Work area 2 (`w = obj + 0xa0 + [[ctrl+0xc]+8]`). In levels `0x1432`,
  `0x6e1`, `0x24de` (`graphicLevelGetNameHash`) the draw only copies
  `vf00` to `w +0x40` and `+0x50` (unless `w` is 0) and returns. Else it
  needs key `+4` not `0xffff`, `TkGetSaveEventAddress()`'s first `u16` not
  `0x5e`, the model record's set not null, and `pppDrawMdlInfDrawSetup(key,
  w)` nonzero when `ptr32 w[+0xdc]` is null.
- Step (not while `g_pppStopStatus`): without byte `+0xe4`, on the key's
  frame (`key[+0]` = `obj[+0xc]`) key `+8..+0x1c` are added to `w
  +0xb8..+0xcc`; then `+0xbc += +0xc0`, `+0xb8 += +0xbc`, `+0xc8 += +0xcc`,
  `+0xc4 += +0xc8` (with `+0xe4`: `+0xb8 += +0x60`, `+0xc4 += +0x64`); both
  phases `fmod` 32768. The model's vertices (`set + [set+8]`, `i16 +0x12`
  of 3 `u16`): byte `+0xe3` bit 2 copies them from `ptr32 w[+0xd0]`; else
  bit 0 sets each axis `k` to `src + ftol(sin[((ftol(w[+0x70+4k]) + B[i] +
  C[i]) * ftol(key[+0x40+4k]) >> 4) & 0xfff] * key[+0x20+4k])` (`B`, `C`
  the `u16` phases at `ptr32 w[+0xd4]`, `[+0xd8]`) and moves `w +0x70..`
  by key `+0x30.. * 256`; bit 1 the same from `w +0x80..`, key `+0x70..`,
  `+0x50..`, `+0x60..` with `B` alone (after bit 0, from the same
  source). Key byte `+0x81` counts frames in `w +0xe0`, then steps the
  texture frame `w +0xe1` (wrapping at `w +0xe2`) of
  `Yn_FpTrxTexGet(key[+0x80])`.
- Draw: angles `w +0x50..+0x58` times 651.8986 (`f64` at `0x00b51590`),
  tile spacing `w +0xb0`/`+0xb4` times the lengths of the object's rows 0
  and 2 (`pppVu0Length`, row 1's length is computed and dropped); with key
  byte `+0x90` an offset of the camera's position (inverse of
  `g_pppCameraMatrix`) snapped to the spacing, else `vf00`. The request
  at `ebp - 0x260`: flags 4 (`| 0x10` with key byte `+0x84`, `& ~0x10`
  `+0x85`, `| 0x200` `+0x86`, `| 8` `+0x87`, `| 0x20` without `+0x88`),
  blend key `u16 +0x82` (set `+0x16` when its low byte is `0xff`), the
  colour (`ee_colour`, nothing drawn at alpha 0), `+0x10`/`+0x14` the
  phases in 16.16, matrices `ebp - 0x168`, `obj + 0x10`, `ebp - 0xa8`, the
  textures `ptr32 obj[+0x90]`, `[+0x94]`. Tiles `(i, j)` for `|i| <=
  ftol(r / sx + 1)`, `|j| <= ftol(r / sz + 1)` (`r` key `+0x8c`) whose
  centre is within `r` (`pppVu0Length`), the centre tile only with key
  byte `+0x89`; key byte `+0x85` sets flag `0x2000` on the 3 x 3 tiles
  round the centre and clears it elsewhere. Each tile: `w`'s matrix by
  `op_rt_m_yxz` of the angles at `w +0x40 + offset + tile` (or, with byte
  `+0xe5`, `w`'s matrix times a translation), times the object's, times
  the camera, then submitted as the other model draws (path 3, flag 4 is
  always set). `ebp - 0x1e8` (screen projection times the tile matrix) is
  computed and never read.

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

## Shape clip test (`VertexGroupApplyZMatZClip`, `0x007119d0`)

`VertexGroupApplyZMatZClip(m, a, b, c, d, flags)` (Fahrenheit's name;
cdecl; from `updateVFXRenderDataShapeField` 0x0071a7af/0x0071b04f,
`updateVFXRenderDataShp_OShapeVu1` 0x0071c408/0x0071c98a and the shape job
`0x00727110` +0x435/+0xaf3; `flags` is the shape's `u16 *b`) returns 0 when
the group is drawn and 1 when it is clipped. Ported in
`ffx_vfx::mat::bridge::vertex_group_apply_zmat_zclip`.

`L = FFXVu0MulMatrix(b, m)` goes into the frame (`ebp - 0x54`), then
`m = a * m`. A screen depth `q` is a row dotted with `g_ScreenMatrix`
column 2 over the same row dotted with column 3 (`S2, S6, S10, S14` and
`S3, S7, S11, S15`). Every partial sum is stored as an `f32`, and a zero
denominator gives `q = 0` (a NaN one divides). `q` is inside when it is
not below 0 and not above 2^23 (`f32` at `0x00b501c4`); a NaN `q` counts
as inside, because the `fcom`s fall through to the draw path (Ghidra's C
reads it the other way).

- Flag 8 clear: `q2` is `L` row 0 over `L` row 3, `q3` is `m` row 3 over
  itself. Drawn when either is inside.
- Flag 8 set: `q0..q2` come from `L` rows 0..2, and the column-3 sums run
  on from row to row (row 1's denominator starts from row 0's rounded
  sum). `q3` comes from `m` row 3, its column-2 sum starting from the
  local `[ebp-8]`, which nothing writes first: stack junk. The port uses 0
  there (brief 2026-10-24). With a zero denominator `q3` is 0 whatever the
  junk. Drawn when any of the four is inside.

The packet's other "unwritten" reads (`ebp - 0x10`, `- 0xc`) are `q1` and
`q2`, which the flag-8 loop writes.

Drawn: `m = c * m`, `L = c * L`. For each row `i`, `m[4i+2] = (m[4i+3] /
L[4i+3]) x L[4i+2]` (the quotient unrounded until the product is stored;
`0 x L[4i+2]` when `L[4i+3]` is 0). Then for each row `p = w z` (`z =
m[4i+2]`, `w = m[4i+3]`): `p <= 0` stores +0 in `z`; otherwise (NaN too)
`|w| < |z|` stores `w` in `z`, unless `graphicLevelGetNameHash()` is
0x21e8 or 0x4a6 or `MagicFile_GetMagicId()` is 0x2b9 (called in that
order, short-circuit). Last, `m = d * m` and `FFXVu0ScaleVector(row, row,
1/w)` for each row (0 when `w` is 0).

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

## oef2 blocks (`op_oef2_set`, `op_oef2_a_get`)

The texture functions of the effect code take an oef2 (ffx-demaster
`docs/effects.md`). It has two parts with one header layout, 16-byte
aligned: the table (what goes where) and the data block (where the
bytes are). `op_oef2_set(table, data)` (`0x007e3b70`) adds the data
block's address to each of its non-zero offsets and writes the result
into the table's entries, once: it sets the data block's `u16 +0x40`
(0 in the file) to 1.

| Offset | Type | Meaning |
|---|---|---|
| `+4 × kind` | `u32` | offset (from the header) of the kind's sub-table; `op_oef2_a_get(oef, kind, i)` (`0x007e3810`) |
| `+0x08` | | kind 2, textures: table entries of 32 bytes, data one `u32` offset per texture |
| `+0x0c` | | kind 3, CLUTs: table entries of 16 bytes, data one `u32` offset per CLUT |
| `+0x18`, `+0x20` | | kind 6 (16-byte entries) and kind 8 (particle groups; the PC prints "Virtuos: OEF2 get particle data for VFX!!") |
| `+0x3c` | `u32` | table only: offset of its own data block |
| `+0x40 + 2 × kind` | `u16` | entry count per kind (`+0x44` textures, `+0x46` CLUTs) |

`op_oef2_a_get` returns 0 for a texture entry whose size (`+0x10`) is 0
and a CLUT entry whose size (`+2`) is 0. A texture entry:

| Offset | Type | Meaning |
|---|---|---|
| `+0x00` | `u64` | the `TEX0` the effect draws with; its low 14 bits are also the upload's DBP (`op_oef2_texture_load` `0x007e5030`; `op_oef2_tex_pos_set` `0x007e4ff0` rewrites them) |
| `+0x08`, `+0x09` | `u8` | upload x / 16, y / 16 |
| `+0x0a` | `u8` | DBW = `byte × 16 >> 6` |
| `+0x0c`, `+0x0e` | `u16` | upload width and height (may be smaller than `TEX0`'s) |
| `+0x10` | `u32` | byte size (0 = unused entry); `op_oef2_tex_clut_cue_loop` sums it |
| `+0x14` | `u32` | the texel pointer `op_oef2_set` writes |
| `+0x18` | `u8` | upload PSM (equal to `TEX0`'s: PSMT8 nearly always, PSMCT32 in 112, PSMT4 in 21 of the disc's entries) |
| `+0x1f` | `u8` | set by `op_oef2_tex_pos_set` |

A CLUT entry is `u16` CBP, `u16` byte size (`0x400` 256 colours, `0x40`
16), `u32` pointer (written by `op_oef2_set`), `u32 0x100`, `u32 0`. Every
`TEX0`'s CBP has its CLUT in the same table (7,574 checked), CSA is
always 0. Texels are linear as uploaded (`sceGsSetDefLoadImage(dbp, dbw,
psm, x, y, w, h)`); `op_oef2_texture_load` splits uploads over 256 KB
into 256-row strips. 256-colour palettes are in CSM1 order, 16-colour
ones (an 8×2 upload) linear (`docs/formats/ps2-textures.md`).

On the PS2 disc the spell data are `host0:magic/mag_NNNN.bin` (584
files, `cdrom.fnd` names them without the `host0:/ffx/master/` prefix):
a MIPS overlay linked at `0x01b00000` followed by its data, with no index
of its oef2s (the code holds their addresses). The field and battle
maps' particle effects keep theirs in `mapout.txa`, monsters in their
`.chr`. The PC has the overlays as `magic_NNNN.dll` and loads each
texture as a named Phyre file; the HD names are in
`docs/formats/ps2-textures.md`.

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

## Unit commands (`opu_unit_etc`)

`opu_unit_etc(host, cmd)` (`0x0080b180`) acts by the host's flag nibble
(`u16 +0x21a & 0xf000`), `u` the host's unit (`+0x220`), the list `host +
0x300`: 0 a conditional jump by `i16 cmd +4` on the unit `opus_no_get`
names; 0x1000 restarts the host's own unit (its number in the `i16` list
`[host +0x310]`: `opus_prog_off`, `opus_list_add(.., 1)`,
`yonExtMapDeleteId`, `yonExtMapRegister`, then `u16 +0x218` down by one,
`+0x216` 0, byte `+0x210` 1); 0x2000 copies `u16 host +0x214` into a
unit; 0x3000 sets a program slot of part `i16 [u + i16 cmd +2]` (`set_ptr32`,
keeping bit 31); 0x4000 registers the list's function `[[host +0x350]
+0x74]` in one of the four op function lists; 0x5000 `op_wot_triger`.
`yonExtMapRegister` (a bare `ret` here) takes a 0x4c-byte struct by value:
0, `u`, the list, the number, an unwritten word, 0, then a 0x34-byte
`sprintf` buffer with `"LoadedByOPU_UNIT_ETC"` whose tail is never written.
The hook passes it with `call_cdecl_words` (docs/hooking.md).

## High range (`0x00a30000..0x00a80000`)

Ported in `crates/ffx-vfx/src/hi_md.rs`, `hi_light.rs`, `hi_draw.rs`,
`hi_opus.rs` and `hi_0283.rs`; the shims are
`crates/ffx-hook/src/ported/vfx_hi.rs`, the oracle tests
`crates/ffx-oracle/tests/vfx_hi.rs`.

What else lies in the range is not effect logic:

- `0x00a34000..0x00a3e000` is Iggy's GDraw back end for D3D11 (its
  allocations are tagged `gdraw_d3d1x_shared.inl`; `0x00a35780` fills the
  GDraw function table at `0x01a85828` and allocates the context
  `[0x01a858b4]`). The `op_node_*` and `op_texels_*` functions ported
  earlier are its handle cache and mip builder (state 5 is GDraw's
  user-owned handle). Middleware, left original. Three slots drive the
  D3D11 device context `[ctx+4]` directly (vtable offsets checked in the
  asm): `0x00a3a820` (slot `0x01a85830`, stored at `0x00a35900`) sets the
  tile rectangle `+0x6c0..+0x6f4` and calls `ClearDepthStencilView`
  (`+0xd4`, flags 3, 1.0, 0) on `+0x784`/`+0x788`; `0x00a37410` (slot
  `0x01a8583c`, `0x00a35946`) binds `+0x784`/`+0x788` with
  `OMSetRenderTargets` (`+0x84`) and `RSSetState` (`+0xac`, `+0x7ac`);
  `0x00a39200` (slot `0x01a85844`, `0x00a35932`) is the render-target
  end, with `OMSetBlendState` (`+0x8c`), `OMSetDepthStencilState`
  (`+0x90`), `RSSetScissorRects` (`+0xb4`), `ResolveSubresource` (`+0xe4`)
  and `CopySubresourceRegion` (`+0xb8`). Each has one reference, the
  table store in `0x00a35780` (`tools/refs.py`); effects round 54 left
  them original as platform, like the other slots.
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
- `op_md_flag80_00a6acb0(obj, init)` (`0x00a6acb0`, flag `0x80` without
  `0x300`): `0x00a6c3d0` without the fog words: `w +0x390` = `(i16 obj
  +0x12 << 4) + [w +0x304]`, `+0x394` = `[w +0x304]`; lights by `[w
  +0x300]`: flag `0x40` `opc(obj, w + 0x300, w)` (a bare `ret`), else
  with `i16 obj +0x14` nonzero and no flag `0x20`
  `op_matrix_light4_vector` into a local nothing reads. Same GS words and
  waits; types 1, 3, 5, 7 act as 0, 2, 4, 6 (jumps `0x00a6b1f8`).
  Returns `[w +0x334]` (no `+0x40`).
- `op_md_one8(obj, init)` (`0x00a6b220`, none of `0x2000`, 4, `0x80`,
  `0x300`) and `op_md_one8_fog` (`0x00a6c8b0`, with one of `0x300`): the
  same `0x2800`-byte start (size check, init block, `w +0x334` rounded
  to 64). `one8` has `0x00a6acb0`'s vertex words and lights, where flag
  `0x40` first sets `[w +0x344]` to `[w +0x320]` when 0; `_fog` has no
  lights and the fog vertex words (`+0x394`, `+0x39c`). GS words as
  `0x00a6c3d0` plus `+0x100` 0x20000000, `+0x108` 0, `+0x2e0` 0x8001,
  `+0x2e8` 0xe, and `+0x150` from `g_OpMdGsWordsB_011a07f0` with init
  flag 8. No GS waits. Tags (jumps `0x00a6b6f0` / `0x00a6cd18`): `w
  +0x1f0` 0x8001 and `+0x1f8` 0x414141 (0, 1), 0x41414141 (4, 5),
  0x12412412 with the texture words (2, 3, 6, 7); an odd type ends the
  chain with `dbgPrintf` (`"one8 draw mode unsuport light program"`,
  `0x00b7c3e0`, after its mode words; `_fog`: `"(op)\top_md0_onef() light
  no support !"`, `0x00b7c42c`, without them) and `debug_exit_trace(1)`.
  Both return `[w +0x334] + 0x40`.
- `op1_mdc_draw(list, init, m, view)` (`0x00a68c90`): `[init +0x20]` and
  `[init +0x2c]` point at two local matrices `W` and `L` during the call;
  `view` (`g_Op3dListener`, `0x0113fcd0`, when 0) is copied to a third,
  `V`. Each of the `u16 list +4` entries (packet, matrix; 16 bytes from
  `list + 0x10`, through `get_ptr32_func`, the identity): the matrix's
  translation times 1000 (`f64`, `0x00b0dba0`), `L = op_mul_m(m, matrix)`,
  the translation times 0.001 (`f64` `0x00b47c68`, an `f32` 0.001
  widened), each an `f32` store, so both roundings stay; `W =
  op_mul_m(V, L)`, `op_md_chara_path3(packet, init, [list])`. At the end
  `[init +0x20]` is restored, but `[init +0x2c]` gets the old `[init
  +0x34]`, which is also the `eax` left.

### Refracting models (`op_md_ref_nolight`)

`op_md_ref_nolight(obj, init, tex, scale, slot)` (`0x00a6a6a0`, from
`op1_mdc_ref_nolight`) only acts for magic 0x1b6 or 0x1b5. It sets up the
model as `opus_0279_draw` does (colour from the signed bytes `p +0x1c..`,
`w +0x400`), fills `g_MagicBuffersB[slot]` (made on first use,
`user_aligned_malloc(i16 obj +0x14 << 4, 16)`) through
`op_refrection_st_float(.., ftol(scale), byte tex +0xa << 4, byte tex +0xb
<< 4)`. A first tag of type 0x10 hands the model to the renderer as
instance `(0x1b5, slot + 100)`, created on first use with
`graphicVFXCreate(.., 0x800a)` and `graphicVFXCreateDataBuffer(.., 1, &3n,
&3n, name, &1)` (`n = i16 obj +0x10`). The texture name is
`"%d_%d_0_0_%d_%d.dds.phyre"` of the `TEX0` at `tex`: `TBP0`, `TBW`, `1 <<
TW`, `1 << TH`, or the name `graphicVFXIsTextureLoaded` reports. The fill
request is kind 6: `+0x10` the render data's `+0x94`, `+0x14` `obj`,
`+0x18` `[p +0x38]`, `+0x1c` the coordinate buffer, `+0x20..+0x28` 256.0,
`+0x2c` `byte tag +7`, `+0x30` the bytes `p +0x18..` / 255, `+0x40` `t x`
the bytes `p +0x1c..` with `t = byte p +0x4d / 255`. Then
`FFXVu0MulMatrix(r, g_pppFillMatrix, [p +0x20])`, `graphicVFXSetVisible`
and `graphicVFXSetup` (new) or `graphicVFXPath3Update`. Other first tags
only set the GS words (types 3, 7, 0x10). When
`graphicDynGeoMemManagerGetBufferForSegment` gives no buffer, the
original returns 0 at once, leaving its `op_spr_alloc` mark pushed; the
port does the same.

### Init blocks with unwritten words

Two callers build `op1_md_draw`'s 0x50-byte init block on their
16-aligned stack frame (`ebp - 0x80`) and leave words of it unwritten,
which the callee copies on (`op_md_spr_init_x64`, `copy_init`). The ports
take those words as an explicit `junk` argument; the hooks pass 0 and the
oracle tests write known values into the original's frame.

- `op_md_x64_draw_00a69300(obj, b, e)` (`0x00a69300`; from
  `opp_draw_part_a74d30`, `0x00814800`, `0x00911fc0`): `op_md_x64(obj,
  block)` with `+0` 0x41c, `+4` `e`, `u16 +8` 0x88, `+0xc..+0x14` 0,
  `+0x18` 0x80808080, `+0x1c` 0, `+0x20` `b`, `+0x34`, `+0x38`, `+0x40`,
  `+0x44` 0; unwritten: `u16 +0xa`, `+0x24..+0x30`, `+0x3c`, `+0x48`,
  `+0x4c`. `eax` is `op_md_x64`'s.
- `opu_md_etc_draw_00a693b0(obj, b, m, view, e)` (`0x00a693b0`; from
  `opu_md_etc`): four points from four `f32` angles (`0x022fb180`,
  `0x00c88c28`, `0x00c88c40`, `0x00c88c58`; `t = a x 3.141592 / 2048`,
  `_CIsin`/`_CIcos` each an `f32`): `(40 sin, 0, 40 cos)` at `0x00c88c18`,
  `(0, 40 sin, 40 cos)` at `0x00c88c2c`, `(0 sin, 40 sin, 40 cos)` at
  `0x00c88c44`, `(50 sin, 80 cos, 80 cos)` at `0x00c88c5c`. The quad block
  `0x022fb188` gets them times -1, `w` 1.0 (16-byte rows), `+0x50` 0x8004,
  `+0x58` 0x441. `FFXVu0RotTransPers` turns each into a 0x30-byte screen
  record at `0x022fb1f8` (`view`, keep 0), whose `x`, `y` then lose 0x100
  and 0x80 and whose words 4..7 are `x + 0x100`, `y + 0x80` and words 2, 3
  (the last record's words 0..3 also into the EE's `v0`).
  `op_ot_add_after(0x022fb1c8, 0xe, 0, 0)`, `FFXVu0InversMatrix(m, m)`,
  then `op_md_chara_path3(obj, block, 0)` with `+0` 0x41c, `+4` `e`, `u16
  +8` 0x88, `+0xc` 0, `+0x18` 0x80808080, `+0x1c` 0x40404040, `+0x20`
  `b`, `+0x24` `0x00c88b78`, `+0x28` the quad block, `+0x2c` `m`, `+0x34`
  0x6c00000, `+0x38`, `+0x40` 0, `+0x44` `b`; unwritten: `u16 +0xa`,
  `+0x10`, `+0x14`, `+0x30`, `+0x3c`, `+0x48`, `+0x4c`. `eax` is
  `op_md_chara_path3`'s (0).

### `opus_0279` draw

`opus_0279_draw(obj, init, rec, _, tex)` (`0x00a70210`, from
`opp_mdc_draw_a6f4d0`) draws a refracting character model:
`op_spr_alloc(0x1800)` gives `w`, `[obj] |= 4`, `p =
op_md_spr_init_x64(obj, init, w)`, the colour as in `op_md_chara_path3`
but from `p +0x1c` and `p +0x4c`, `w +0x390`/`+0x394` as there, `w +0x400 =
(i16 obj +0x14 << 4) + [w +0x390]`; `op_vertex_normal_point44_ffx_2(obj,
p, w, g_EtChrHandles[byte rec +0x18])`, `op_refrection_st_float(obj, p, w,
[w +0x400], ftol(rec +0x3c x 16), byte tex +0xa << 4, byte tex +0xb <<
4)`. Each 0x20-byte tag of type 0x10 from `obj + [obj +4]` sets `w +0x1f0`
0x8001, `+0x1f8` 0x12412412, `+0x120` `[tex]`, and expands its colour word
`[tag +4 + g_pppColorConShift]` (the shift then 0) the PS2 way: the byte
moves the original does are `pextlb` then `pextlh` with `r0`, so with `c`
the colour's bytes and `r` `r0`'s, the EE's `v0` gets the words `(c0, r0,
r0, r1)`, `(c1, r1, r2, r3)`, `(c2, r2, r4, r5)`, `(c3, r3, r6, r7)` (bytes,
low first): with `r0` zero the colour's channels. Those words as `f32`s of
signed integers go to `g_Vu0_VfMove` and `w +0x2e0`. After the GS wait
and `op_spr_free`, `eax` is the last `g_GsRegisterSink` read.

### `opus_0283` draw

`opus_0283_draw(obj, rec)` (`0x00a76770`, kind 0x1000 of
`opp_part_a76720`) does nothing while `g_GraphicSkipFrame` is set. It
builds a packet at `b = [obj +0x328]`: header `+0` 0, `+4` 0x20, `+8`
0x5040, `+0x10` 0x08000200, `+0x14` 0x880000, `u16 +0x20` 0x600, `u16
+0x22` the low half of `+0x10`, `+0x28` 0x1a00; 16 x 32 quads of 0x28
bytes from `+0x30` (colours 0x80808080 x 4, vertex numbers `33 row +
col`, `+1`, `+33`, `+34`, texture `u16` pairs `(0x80 col, 0xd0 row)` to
`(+0x80, +0xd0)`), the end tag's type byte 0xff after them. The vertex
table at `+0x5040`, 17 rows of 33 `i16` triples: with `r = rec +0x30`,
`k = f32(rec +0x34 / 256)`, `x = 8 j - 128`, `y = 13 i - 104`, `d =
f32(y^2 + x^2 + r^2)`, `s = f32(sqrt d)`, `q = f32(f32(s - f32(f32(s -
r) k)) / s)`: `ftol(q x 128 x 1.333)`, `ftol(q y 128 x 1.333)`, `ftol(-128
q r)` (1.333 is `f64` `0x00b7c570`; low halves stored). The request at
`obj +0x100` (`+0` 0x90, `+4` `b + 0xd040`, `u16 +8` `rec +0xc2`, `+0x18`
`rec +0xc8`, `+0x20` `obj + 0xc0`, `+0x34` `[obj +0x354]`, `+0x38` `b +
[b +8]`, `+0x40` `rec +0xc4`, the rest 0), the matrix `obj +0x80`
(`op_unit_m_scale(f32(rec +0x3c x 2^-21))`, `op_trans_m(rec + 0x90)`, the
parent's matrix (`opus_parent_get`) times it, copied to `obj +0xc0`).
Then `updateVFXRenderData(0, id, part, obj + 0x100)` and
`graphicVFXUpdate(0, id)` when `(0, id)` has render data, else
`InitVFXRenderData(0, id, part, obj + 0x100, 8)`, `id` =
`MagicFile_GetMagicId()`; `part` is a stack local with only `+0x1c` (=
`b`) set, the only word the renderer reads. Ported in
`crates/ffx-vfx/src/hi_0283.rs`.

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

`opus_0437_poly_check(list, b, dots, m, limit)` (`0x00a6ed40`, kind
0x1000) fills it. Each packet's vertices go through `m` x its matrix
(translation x1000 for the product, /1000 after, `f32` both times) into a
`user_malloc` buffer. Polygon `idx` (counted over the list's type-0x10
tags, `0x1c`-byte polygons from `tag +0x20`, vertex numbers `u16 +0..+4`)
whose table word is 0xffff and whose height `f32(f32(f32(v1.y + v0.y) +
v2.y) / 3)` is `<= limit` takes entry `s = u16 b +2`: table word `s`, the
entry's old polygon (`i16 +0xc`) 0xfffe unless negative, `u16 b +2 = (s +
1) % u16 b +0`, the centre `f32((v1 + v0 + v2) x 0.333333)` at `+0`, the
EE's `r0` at `+0x50`, `+0xc` `idx`, `+0x5e` 0x1000. With a dot buffer,
eight dots per polygon: `op_xyz_scale(dot, centre + op_random(4), 1.0)`
and `dot +0x10` = `(0, (op_random(16) - 20) / 16, 0, w)` (also into the
EE's `v0`) with `u16 +0x1c` 0, where the vector's `w` and the last word
are frame words the original never writes (the hook passes 0). `eax` is
`u16 list +4`.

### `opus_0582` dots

`opus_0582_dot_set_00a73740(b, rec)` (`0x00a73740`,
`opp_0582_cmd_a70910` kind 0x2000) fills the next entry (`b[1] % b[0]`)
of the dot buffer `opus_0582_dot_draw` reads, from the part `rec`: radii
`+0x40`, `+0x48` / 256, height `+0x94`, scale `+0x3c` / 16, angle
`ftol(+0x30)`, step `ftol(ftol(+0x34) / 256)` (the integer back through
an `f32`), spread `ftol(+0x38)` (0 when negative), life `ftol(+0x4c) +
16`, colour `+0xc8`, fade-in 8. It returns the colour.

### Breaking-model pieces

`opp_md_break_pieces_00a744e0(src, out, rec)` (`0x00a744e0`, from
`opp_md_break_init_a74410`): `out[1]` = `n = i16 src +0x10`, `out[0]` =
`src[0]`, `out[2..4]` = `src +0x28..`, `u16 out +0x10` = `u16 src +0x16`.
Piece `k` reads 0x20 bytes from `src +0x30` and fills 0x100 from `out +0x20`:
words `+0..+8` and `+0x14..+0x1c` copied to `+0` and `+0x40`; the
vertices `i16 +0xc`, `+0xe`, `+0x10` (`i16` triples from `src + [src +8]`)
`A`, `B`, `C`; the centre `f32((B + A + C) / 3)` at `+0x50` and each vertex
minus it at `+0x10`, `+0x20`, `+0x30`. Twelve spreads
`f32(op_random(i16 rec + r) + i16 rec + b)` go to `+0x60..+0x68`,
`+0x70..+0x78` (`b` from `rec +6`, `r` = `b + 6`) and `+0x90..+0x98`,
`+0xa0..+0xa8` (`b` from `rec +0x1e`), with `+0x80..+0x88` 0; `+0xb0` =
`i16 rec +0x36`, `+0xb4` = `op_random_per(i16 rec +0x3a) + i16 rec +0x38`.
Kind `u16 out +0x12` 1 replaces `+0x50`/`+0x58` with the length and angle
of `op_deg_get(0, 0, centre.x, centre.z)`; kind 0 scales the six spreads
at `+0x60`/`+0x70` by `f32(i16 rec +4 / 4096)`, kind 1 only `+0x60`,
`+0x64`, `+0x70`, `+0x74`.

`op_random` (`0x007e6700`) stores its result into its own argument slot
(`fstp [ebp+8]`) before returning it. This function relies on it: the
slot is the second of the four words `op_random_per` reads, so it gets
the last spread's draw. The hooked `op_random` cannot write the caller's
slot (its thunk copies the arguments), so an original that reads the slot
after a call to it sees the operand there.

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
