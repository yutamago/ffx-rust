# Movies (`ffx_movie`)

The movie player is PC-port and PhyreEngine code that stays original
(glue and library): `PhyFMVPlayerManager` (`0x006d6970..0x006daec0`) and
`PVideoPlaybackWin32` (`0x00a26e70..0x00a28890`: WebM demuxing with
libwebm's mkvparser, libvpx 1.4.0 VP8 video, libVorbis 1.3.5 audio played
through an FMOD stream, `0x00a273b0` and its read callback `0x00a27170`).
Its boundary, threads, timing and every import it calls are in
`docs/formats/platform.md` ("Movies").

## `FFXSamp0UnitMatrix` (`0x0093d900`)

`(m) -> 4`: the PS2 sample library's identity, 16 f32 row-major at `m`
(`fld1`/`fldz`, so +0.0), row by row; the row counter (4) is left in
`eax`. Called by the other `FFXSamp0*` matrix builders
(`FFXSamp0ViewScreenMatrix` `0x0093d960`) and `eiAbmCalc`.

## `FFXSamp0ViewScreenMatrix` (`0x0093d960`)

`(m, scrz, ax, ay, cx, cy, zmin, zmax, nearz, farz)`, libvu0's
`sceVu0ViewScreenMatrix`: `m` becomes the projection (the identity but
`[0]` = `[5]` = `scrz`, `[10]` = `[15]` = 0, `[11]` = `[14]` = 1), a
matrix in its frame the screen transform (the identity but `[0]` = `ax`,
`[5]` = `ay`, `[12]` = `cx`, `[13]` = `cy`, `[10]` = `(zmax - zmin) *
(nearz * farz) / (farz - nearz)`, `[14]` = `(farz * zmin - nearz * zmax)
/ (farz - nearz)`, each computed on the x87 at 53 bits in that operand
order and stored as f32), then `rcuiMulMatrix(m, screen, m)` (`0x0093d1b0`).
The arguments are copied with `fld`/`fstp`, so a signalling NaN comes out
quiet. `eax` is `rcuiMulMatrix`'s; nothing reads it.
