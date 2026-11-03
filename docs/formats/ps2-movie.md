# PS2 movies (`movie/adpcm/*.dat`, `*.sz`)

The PC plays WebM files (`docs/formats/movie.md`, `docs/formats/platform.md`,
"Movies"). The PS2 disc's movies are not in the PC data, but FFX.exe still
carries the PS2 movie viewer, unreached: `MovieViewer_IntraDecode_0077b480`,
the predicted-picture decoder `MovieViewer_PredictedDecode_0077b630`,
`IpuLoadMacroblock_0077e210`, `MovieViewer_BlockStep_0077c3c0` and the
record dispatcher `MovieDecoder_runCommand_0077e460`. Their IPU commands are
writes to `g_GsRegisterSink`. The format below was read from those
functions and checked by decoding every movie of the German PAL disc (source:
ffx-demaster `docs/movies.md`; disc layout `docs/formats/ps2-disc.md`).

## Files

`jppc/movie/adpcm/<name>.dat` and `<name>.sz`: 53 movies, 2.6 GB on the
German disc, stored uncompressed. The eight small `jppc/movie/*.dat`
(`ikai`, `hiroba`, `luca4_*`, ...) are in-map texture videos, not this
format's FMVs.

`.sz`: the IPU's intra and non-intra quantiser matrices (64 bytes each, in
the raster order `SETIQ` uploads them), then one `u32` per frame: the size
of that frame's chunk in `.dat` (0 ends the list). The chunk sizes add up
to the `.dat`'s size.

## Chunks

A chunk is a run of records, each with a 16-byte header:

| Record | Header | Body |
|---|---|---|
| picture (always first) | `u32` bitstream size, `u32`, `+8` type (0 intra, 1 predicted), `+9` intra quantiser, `+0xa` non-intra quantiser, `+0xb`/`+0xc` width/height in macroblocks (`0x28 x 0x1a` = 640 x 416; `0x20` = 512 wide in four movies), `+0xe`/`+0xf` IPU_CTRL bits (`+0xf >> 1` the DC precision; 10 bits in most movies) | the bitstream, padded; the end-of-picture code comes within 32 bytes of the size |
| `0x40` | `0x40`, code bytes, padded size, 0 | delta-coded sound |
| `0x50`, `0x51` | type, unpacked size, packed size, second track size (0 in every movie) | `sa_decode` LZ of delta-coded sound |
| `0x7800` | `0x7800` | raw PCM |
| `0x30` | `0x30`, -1, -1, -1 | 0x30 bytes of camera floats (the movies with a 3D scene on top) |
| end | 16 zero bytes | |

`MovieViewer_PredictedDecode_0077b630` builds the IPU control word from
`+0xe` and `+0xf`: `((+0xe + (+0xf & 1) * 4) * 0x10 + (+0xf >> 1)) << 16`.

## Sound

`MovieDecoder_runCommand_0077e460` dispatches the sound records to
`AudioSaDecode_0077b270` (`0x50`/`0x51`), `AudioDeltaDecode_0077b1f0`
(`0x40`) and `AudioMixHalves_0077b060`. Samples are `s16` in blocks of 256
left then 256 right, 48 kHz stereo. Delta code: a byte with bit 7 set adds
its low 7 bits (signed); otherwise it and the next byte add a 15-bit signed
value; the running sum is the sample. One sound record holds 7,680 sample
frames (4 PAL frames at 48 kHz; the 0x6400 and 0x3c00 sizes in the
dispatcher are the NTSC equivalents). No movie on the disc uses ADPCM
despite the folder name, and none is 5.1.

## Pictures

The bitstream is MPEG-2 macroblock syntax that the PS2 decoded with the IPU
(`VDEC`, `BDEC`) and the EE (motion compensation):

- Intra (`MovieViewer_IntraDecode_0077b480`): per macroblock a
  macroblock-address-increment code (always 1) and an intra `BDEC`: four
  luma blocks, Cb, Cr; DC predictors reset at the first macroblock; intra
  VLC table B.15 (IPU_CTRL IVF always set), zigzag scan, linear quantiser
  scale (`q * 2`), the quantiser matrix indexed by scan position, no
  mismatch control.
- Predicted (`MovieViewer_PredictedDecode_0077b630`; Ghidra drops most of
  it, because the IPU busy-waits look endless, so read it with objdump):
  per macroblock a mode coded with the address-increment table (1 motion +
  residual, 2 intra, 3 motion only, 4 end of picture), a second
  address-increment code (escapes skipped), then for modes 1 and 3 four
  motion codes a, b, c, d (table B.10 with sign bits): the vector is
  `(16 a + c, 16 b + d)` in whole pixels. Mode 1 reads a non-intra `BDEC`
  (coded block pattern table B.14, the non-intra quantiser), mode 2 an
  intra one (DC predictors reset at the first intra macroblock of the
  picture).
- Prediction reads the last intra picture, not the previous picture: the
  macroblock store (viewer state `+0x503c8`) is written only by the intra
  decoder. Chroma moves by `floor(x / 2)` whole pixels (half-pel averaging
  matched the next intra picture worse).
- `IpuLoadMacroblock_0077e210` clears the quarters of its 2x2-macroblock
  window that lie outside the picture. A few vectors point there with no
  residual, so the encoder saw real pixels there; a decoder should repeat
  the edge.

All 53 movies decode with every picture ending at its record's size
(68,616 pictures).

## Against the PC's videos

The PC's files are 1280 x 720 VP8 at 29.97 fps with Vorbis 48 kHz stereo,
named per movie number by `ffx_data/gamedata/ps3data/video/<lang>/ffx_videolist.txt`
(lines `id:,FILE.webm,FILE.webm`). PS2 `saw` is the PC's `swa`; the PC has
no counterpart of `fal`, `syuin` and the `*_480` texture videos. The PS2
movies are 25 fps 640 x 416 (or 512 x 416) for a 4:3 screen and show more at
the top and bottom than the HD cut. The four movies with the 3D scene on
top (`movie_have_camera`: ids 0x20, 0x2e, 0x46, 0x48; `mst`, `far`, `ggm`,
`sns`) switch to 512 x 416 where the scene is drawn; the PC re-rendered
them for its 16:9 cameras (`video/<id>.dat`). The PS2 sound is the same mix
as the HD `_us` files (correlation 0.97 on `sak`).
