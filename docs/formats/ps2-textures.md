# PS2 textures and their HD names

The PC still runs the PS2 code that sets up GS textures (`TEX0`, CLUT
uploads, image transfers), but most of the PS2 texels are gone from its
data: it loads each texture as a named `*.dds.phyre` instead. This page
collects what the PS2 data and GS conventions behind those names are.
Source: ffx-demaster `docs/research/ps2-textures.md`, `docs/maps.md`,
`docs/effects.md`, `docs/research/hd-vs-ps2-assets.md` (checked on the
German PAL disc against the PC's `FFX_Data.vbf`). The per-file layouts are
in `docs/formats/chara.md` ("Texture data"), `field.md` (the map render
file's texture records) and `vfx.md` ("oef2 blocks").

## GS conventions the data relies on

| Item | Value | Where |
|---|---|---|
| Alpha | GS scale, `0x80` opaque; above `0x80` doubles in a blend | CLUT bytes end in `80` or `7f` |
| Colour order | R, G, B, A bytes (PSMCT32) | |
| 256-colour CLUT | sent as a 16×16 PSMCT32 rectangle, read with CPSM PSMCT32 and CSM1: index `i` is stored at `(i & 0xe7) \| (i & 8) << 1 \| (i & 0x10) >> 1` (bits 3 and 4 swapped) | characters (`Ch_TextureSetRegs` `0x0083d250` leaves CPSM, CSM, CSA 0), maps (`yiClutAlloc_makepacket` `0x00917050`, 16×16 transfers), effects |
| 16 colours of a 256 CLUT | a PSMT4 `TEX0` with `CSA n` reads entries `16n..16n+15` of the 256 a PSMT8 `TEX0` with `CLD` loaded before it | map `.vpa` packets |
| 16-colour CLUT | an 8×2 transfer, linear | effects |
| Swizzle | the GS page, block and column layouts (as GSdx/PCSX2 `GSLocalMemory`); FFX.exe has the PSMT8 one in `yiSwizzleImage` (`0x0090e0b0`, `docs/formats/field.md`) | |
| PSMT4 pre-swizzled | map T4 textures are stored as the PSMCT32 words of a (w/2)×(h/4) rectangle at half the DBW, so written as CT32 they read back as PSMT4 | `yiMapFileRead` (`0x0090f900`) case 2 |

Character images are linear PSMT8 (all 857 `.chr`, 64×64 to 512×512, all
with 256-colour CLUTs); map textures are linear except T4; effect
textures are linear (PSMT4 two texels a byte, low nibble first). Reading
a texture as the game drew it means writing the uploads into a model of
GS memory and reading the `TEX0` rectangle back: uploads and `TEX0` can
differ (an effect upload of 64×64 at DBW 1 read by a 128×64 `TEX0` at TBW
2; two 64-wide halves at x 0 and 64 with a CLUT each), and some map
names point at where a texture ended up at run time rather than where
the `.txa` put it.

## What the PC's copies of the PS2 files lack

- `.chr`: only the image texels and texture-animation patch texels are
  zeroed (857/857 files; CLUTs and upload headers kept).
- `mapout.vpa`: the texels of section 1's textures, most of the kind-1
  and kind-6 packets and the 2D background picture of section 0. The
  `.txa` files (textures and CLUT banks) are not shipped at all.
- The spell overlays `mag_NNNN.bin` are not shipped (the PC has
  `magic_NNNN.dll`).

## HD texture names

Map textures, `ffx_data/gamedata/ps3data/{map,btlmap}/<area>/<map>/tex/d3d11/`,
are named after the PS2 GS region they replace, all fields decimal:

```
TBP_PSM_X_Y_BPP_W_H[_FRAME].dds.phyre       13184_20_000_000_4_256_256
```

`TBP` is the page's GS block address (`13184` = `0x3380`), `PSM` 19 PSMT8,
20 PSMT4, 27 PSMT8H, 36 PSMT4HH, 44 PSMT4HL, `X`, `Y`, `W`, `H` the region
in texels (not powers of two: `48_48`, `104_32`), `BPP` 8 or 4, and
`FRAME` the frame of a texture-swap animation in `tex/texswap/d3d11/`. The
HD art is the region upscaled, mostly 4× (swap frames 1×). 15,950 of the
16,781 HD map textures carry such a name; the rest (`bsil01_fabric_02`,
`reflection3`, `*_colorcorrection`, `*_s` specular maps, `znkd*`) are
HD-only.

Effect textures:

| Where | Name | Example |
|---|---|---|
| `magic/magic_NNNN/tex/` | the `TEX0`: `TBP_PSM_0_0_(1 << TW)_(1 << TH)` | `13312_19_0_0_256_256` |
| `{map,btlmap}/*/fp/tex/` | the upload rectangle `TBP_PSM_X_Y_W_H` (and `TEX0` names) | `13888_19_64_0_64_64` |
| `chr/mon/*/fp/tex/` | the upload rectangle with bits per texel, `TBP_X_Y_BPP_W_H`; the file holds the whole `TEX0` area | `16128_64_0_8_64_128` |
| a later upload to the same place | `_2`, `_3`, ... (or `_0`, `_1`) after the name | `13440_19_0_0_256_256_2` |
| a second oef2 table in the file | `tex1/` with the plain name, and in `tex/` with the table index in the fourth field | `tex1/13632_19_0_0_256_256` |
| text (overdrive names, "Magic") | `tex_de/`, `tex_fr/`, `tex_it/`, `tex_es/`, `tex_us/` beside `tex/`; `tex_cn/` copies every texture | `magic_0007/tex_de/14528_19_0_0_256_128` |
| copies of the frame | `trxtex_N_W_H` | |

The game formats these with `"%d_%d_0_0_%d_%d.dds.phyre"` (`0x00b50114`:
`createVFXRenderDataBufferShp` `0x00715170`,
`FakeEnvModel_createRenderData_00715af0`,
`KaGra001_createRenderData_00715dc0`, `op_md_ref_nolight` `0x00a6a6a0`;
`graphicDrawUIElementAfterSet` `0x00906420` builds the `tex_cn` path) and
parses them back with `"%d_%d_%d_%d_%d_%d"` in
`PhyreScene_setVFXTextureLayout_00656ae0`, which maps any name holding
`trxtex` to -1. Virtuos added names with no PS2 upload (`_n`,
`99999_...`, `30000_...`, `13601_...`, `00001_19_0_0_1_1`,
`logo_hdremaster`, `_jp`). Many field-map particle textures are stored
upside down in the HD data (755 of 1,598; 119 of 189 battle-map ones; 7
magic textures upside down, 3 mirrored), and the PC draws them as stored.
The HD stores most greyscale glows, trails and rings as white with the
brightness in alpha (`docs/formats/effects-draw.md`, "States and
constants").

Menu textures (`menu/`, Tk 2D) are named `TBP_PSM` too.

The 2D backgrounds (`YNSC` layers, `docs/formats/field.md`) are
`map/<area>/<map>/2d/tex/d3d11/<layer>_<part>_<w>_<h>_<frame>.dds.phyre`,
2048-square DXT5 for a 512 x 448 PS2 part. Sorted by (layer, part) they pair
with the PS2 part transfers in file order (the PS2 sizes are padded, within 8
texels). The HD re-framed 37 rooms for 16:9 (bika01, hiku00, luca06,
kami01, ...): their PS2 picture is a zoomed crop of the HD one. Parts with
several HD frames (texture animation such as azit00's wheel) hold one
picture on the PS2 (ffx-demaster `docs/screen.md`).

## The PS2 font (`menu/base.ftc`)

The PS2 International font, `uspc/menu/base.ftc` (the German disc's
`jppc/menu/base.ftc` is the same file, 14,096 bytes), is an FTC of type 4
with a bitmap: 200 glyphs in 14 x 18 cells, the bitmap at `+0x40` one
128 x 216 page of 4 bits per texel (low nibble first) that holds two 2-bit
pages, bits 0-1 page 0 and bits 2-3 page 1. A 2-bit value is coverage (3
full, 1-2 the anti-aliased edge); the colours come from a CLUT built at run
time, not stored in the ELF. The width table at `+0x3640` (208 bytes) is in
whole texels, and every glyph starts at its cell's left edge.

The PC keeps the PS2 glyph order and cell grid: `TOGetFontInfo`
(`0x008b7240`, `docs/formats/system.md`, "Text helpers") puts glyph `i` on
page `i & 1` at `((i % 18) / 2 * 14, i / 18 * 18)` and reads the PC width
table (`ffx_ps2/ffx/master/uspc/menu/base.ftc`, type 4 without a bitmap,
230 glyphs, 128 x 234 pages) in quarter texels, so a PS2 width w is `4w`
there. Glyphs 0..185 exist in both; 99, 103 (`Œ`, `œ`), 170 and 187-229
are PC additions (ffx-demaster `docs/font.md`).

## Which HD models kept the PS2 mesh

The HD characters are PS2 meshes with upscaled textures, or remodels
(ffx-demaster `docs/research/hd-vs-ps2-assets.md`; HD vertex × 1000
against the PS2 `i16`, and the UVs):

| Kind | Same mesh and UVs | Same mesh, UVs scaled/offset per page or partly re-laid | Edited or remodelled |
|---|---|---|---|
| party (`chr/pc`) | 0 | 1 (c902) | 32 (every other c0xx, c1xx, c9xx) |
| NPCs | 128 | 0 | 96 |
| monsters | 144 | 160 | 27 |
| summons | 12 | 13 | 4 |
| weapons | 0 | 74 | 5 |
| objects | 15 | 74 | 21 |

The field and battle maps keep most PS2 geometry (median 92 % of HD
vertices on a PS2 vertex, `PS2 = (-x, -y, z)` of the HD world) and UVs
(one scale and offset per HD material), with HD-only additions (larger
ground and sky planes) and a few remodelled areas (sins02). The effects
have no HD meshes at all.
