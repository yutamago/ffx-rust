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
at this seam.
