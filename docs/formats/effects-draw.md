# Effect draws: what PhyreEngine draws for the effects, and from what

The M4 census of the effects (`docs/rewrite-plan.md`, M4; the scene's was
`docs/formats/scene.md`): every effect draw of captured battle frames,
traced to the `VFXDynamicGeometry` it draws, the effect object that made
it (ppp object or op record), the `PhyreScene` list and pass that draw it,
its shader, states and constants. It is the spec for the effects part of
the Rust scene builder. Crate: [`crates/ffx-fxdraw`](../../crates/ffx-fxdraw).

```bash
# one frame: every draw with its producer, the effect state table, the gate
FFX_GAME_DIR=~/steam-games/FFX-X2-HD cargo run --release -p ffx-fxdraw --example census -- \
  ~/ffx-work/m4/captures/fx-magic/battle-magic-2 ~/ffx-work/m4/captures/fx-magic/battle-magic-2.snap
# the gate over every <name>/ + <name>.snap pair of the folders (skips when unset)
FFX_GAME_DIR=~/steam-games/FFX-X2-HD \
  FFX_FXDRAW_CAPTURES=~/ffx-work/m4/captures/fx-magic:~/ffx-work/m4/captures/fx-overdrive \
  cargo test --release -p ffx-fxdraw --test captures
```

Other examples: `draws` (a capture's draws with targets, shader families
and ring slots), `probe` (where a draw's ring bytes sit in a snapshot),
`peek` (words of a snapshot; `find` for a word).

## The captures

`tools/replays/draw-fx-magic.txt` and `draw-fx-overdrive.txt` are
`battle-magic.txt` and `battle-overdrive.txt` up to their last effect shot,
with a hash capture (`gfxcapture … hash frames 1 now async`) and a memory
snapshot (`snapshot`) at each effect shot: battle-magic 1-3 (magic 481,
the `op_shape_draw` burst) and battle-overdrive 2 (Spiral Cut), 3 and 4
(Valefor's entry, magic 0x253) and 6 and 7 (Valefor's overdrive). Original
code (`original`), graphics layer on (`FFX_GFX=1`), sound off.

The snapshot is taken one frame after the capture's held point (`wait 1`,
then `snapshot`): the effect vertices a captured frame draws are those of
the update after the held point. A 3-frame capture against snapshots taken
0, 3 and 4 frames after the held point showed it: frame k's effect draws
match the snapshot taken k + 1 frames on (frame 2 against the 3-frame
snapshot: 205 of 212 traced at once, against 22-43 for every other pair).
`PhyreScene_render` merges the frame's VFX arrays itself
(`PhyreScene_MergeVFXRenderArrays` `0x00654650`, render thread), while the
main thread runs the next update.

**On PC2** (2026-10-25) the replays' saves are missing (the main
machine's `ffx_001` is `cdsp0000`, its `ffx_005` `kami0100`; here slot 1 is
story 3210 at Macalania). The captures above were taken with variants of
the two scripts (`~/ffx-work/m4/pcfx-scripts/`, not in git): save 1, the
party written (Rikku, Tidus, Wakka for the magic; Yuna and Tidus for the
overdrive), battle `genk16_03` on land for both, Tidus using the first
item of his list (a Hi-Potion: item 1's effect IDs, table `+0x84`, made 481
as item 0's), Valefor's overdrive matched by the ID's high byte, and the
load waits raised to 300000 frames (a battle took up to 17000 lockstep
frames to load here). Same effects, other party, map and timing: the
numbers below are these captures'. The scripts in git are for the main
machine's saves.

## Where the effects are drawn

`PhyreScene_render` (`0x00668930`), for these battle frames, in this order
(draw indices of battle-magic-2; the census's "runs"):

| Draws | Producer | Pass |
|---|---|---|
| 0-65 | the 3D scene (map, characters, monsters, blob shadows) | Draw3DWorld into the scene targets (`docs/formats/scene.md`) |
| 66-78 | post | the glow chain (`phyreglow.fx`, 13 draws) |
| (64-70 in overdrive 6/7) | late world | the world at scene `+0x9c` and the postOpaque/postTransparent passes, into the colour target alone, after the glow: Valefor in its overdrive (7 `phyrechrlitshader` draws) |
| 79-284 | **effects** | `PhyreScene_DrawMostVFX` (`0x00650670`) |
| 285-286 | post | `copybuffer.fx`, `brightnessshader.fx` (`phyreblur.fx` and `phyredistortionshader.fx` in some frames) |
| 287-353 | UI | `menudefaultshader*.fx`, the fonts |
| 354 | post | the final copy to the back buffer |

Every effect draw of the eight frames is in one block, the
`DrawMostVFX` pass: Transparent pass name (`0x00c169d4`), into the scene
colour target (`1280x720` R8G8B8A8, view format 28) against the scene
depth (R24G8, depth test on, write off), one render target only (the
normal target of the scene pass is not bound).

### The VFX lists of `PhyreScene`

A `VFXDynamicGeometry` is put on a list every frame by its update
(`VfxGeometry_Register_006a52f0`, vtable `+8`, called by
`graphicVFXUpdateWithType` `0x00645ba0` with the type in its flags):

| Kind (flags) | List | Drawn by |
|---|---|---|
| 0, flag bit 31 set (`graphicVFXPath3Update`) | `+0x38cc` (fixed, 0xc00 entries, count `+0xf8cc`): the mesh instance (`geometry +0xc`) | `PhyreScene_DrawMostVFX` |
| 0, bit 31 clear | `+0x68cc` (count `+0xf8d0`): the geometry itself, to be depth-sorted | merged into `+0x38cc` by `PhyreScene_MergeVFXRenderArrays` |
| 0, geometry byte `+0x50` set | `+0x3880` (count `+0x3878`) (`PhyreScene_AddVFXToList3880` `0x006565e0`) | no pass found in `PhyreScene_render` (open question 4) |
| 1 | `+0x3868` (count `+0x3860`) (`0x006a0d40`) | `PhyreScene_DrawUI2DVFX` (`0x00650ea0`), only while fading |
| 2 | `+0x385c` (count `+0x3854`) (`0x00656990`) | `PhyreScene_RenderDTTVFX` (`0x006531b0`) |
| 4 (`0x00ccca58 == 1` and kind != 2, or geometry `+0x3c` bit 8) | `+0x3888` (count `+0x388c`) (`0x006567e0`) | `PhyreScene_RenderVFXAfterPostProcess` (`0x006538d0`) |

Bit 30 of the flags skips the geometry's own update (vtable `+0x28`,
`VFXDynamicGeometry_updateRenderDataWithoutAddingToArray`).

`PhyreScene_MergeVFXRenderArrays` (`0x00654650`) closes the current sync
path (`PhyreScene_VFXSyncPath` `0x00654cc0` opens them; counts at `+0xf8d4`
and `+0xf914`, `+0xf954` paths) and rebuilds `+0x38cc` path by path: that
path's unsorted mesh instances first, then its sorted geometries' mesh
instances. The sort (`PhyreScene_GetVFXSortMode` `0x006721b0`): mode 1 (the
default, and 24 magic IDs) puts the geometries into 4096 buckets by `ftol`
of their depth (geometry `+0xdc`, f32, -1 from the constructor), each
bucket in insertion order; mode 2 (magic 0x203, 0x29b) is the CRT `qsort`
with `VFXGeometry_CompareDepth` (`0x00659140`); mode 0 (five level hashes)
leaves them. `PhyreScene_AddDrawEntry` (`0x00656c10`) sets `+0x10d2c` when
the mode is not 0.

`PhyreScene_DrawMostVFX` then renders the list in order: each non-zero
entry through `renderMeshInstance(mi, mi +0x10, 0, scene +0xd0 +0x13c)`,
with the draw events (`PhyreScene_addVFXDrawEvent` `0x00656940`, 16-byte
records at `+0xfb5c`, count `+0xfb54`) run when their position (`+4`, the
list count at the time of the event) is reached
(`PhyreScene_processVFXDrawEvent` `0x00666e40`: 0 characters with the
event camera, 1 copy the colour or depth buffer, 2 screen fade, 3
PS2-lighting set-up `0x006e5a40`, 4 specular, 5 draw a character, 6 a
z pre-pass; their words: "Draw events" below). For magic 0x296 the
geometry of instance (0x78, 0x5bc) is held back and drawn last, after the
last event. No frame here had an event.

A mesh instance draws one indexed draw per segment of its render data
with indices (segment `+8` > 0), in segment order.

### What the census checked

The census builds that sequence from the snapshot (every list entry, the
geometry whose `+0xc` is the entry, its segments with indices) and aligns
it with the captured effect draws (draws with the VFX vertex layout or a
`vfx*.fx` shader), a pair counting only when the draw's ring bytes are the
segment's: the four vertex arrays from the draw's first record and the
indices relative to their smallest. On all eight frames:

- every expected draw is drawn, in list order, and every effect draw is an
  expected one, but for 24 draws (battle-magic-1: 7, battle-magic-3: 17)
  whose list entry the snapshot holds as 0: their geometry sits in the
  table's delete pool (destroyed by `graphicVFXDestroy`; after the frame's
  render `PhyreScene_doDeleteMeshInstances` zeroed the entry,
  `PhyreScene_removeVFXFromRenderArray` `0x006687a0`, before the
  snapshot), with the
  bytes the frame drew, at a zeroed entry between its neighbours' places;
- only `DrawMostVFX` was in use (the other four lists were empty).

## The geometry

`VFXDynamicGeometry` (constructor `0x0069e3e0`, vtable `0x00b47e40`), what
the census reads:

| Offset | Meaning |
|---|---|
| `+0x0c` | the mesh instance the lists hold |
| `+0x20` | the shader (`ClassDynamicGeometry_SetShader_006a4800`) |
| `+0x24` | the render data (`docs/formats/render.md`): `+0x90` segment count, `+0x94` segments of `0x6c` bytes |
| `+0x2c` | the cluster the shader came from (default `g_PhyreScene +0x2874`) |
| `+0x3c` | the VSP flags (below) |
| `+0x50` | byte: list `+0x3880` instead |
| `+0xdc` | f32 depth key of the sort (-1 at construction) |

A segment's vertex arrays (`graphicDynGeoMemManagerGetBufferForSegment`
`0x0063fdb0`): `+4` vertices, `+8` indices, `+0xc` positions (12 bytes a
vertex), `+0x10` normals (12), `+0x14` colours (16), `+0x18` uvs (8),
`+0x1c` indices (u16); the same at `+0x2c + 0x1c n` for buffer slot `n`
(counts `+0x2c`, `+0x30`, positions `+0x34`, normals `+0x38`, colours
`+0x3c`, uvs `+0x40`, indices `+0x44`), which is what the draw reads
("Which arrays the draw reads" below).
The draw binds them as rings: positions slot 0, normals slot 1, colours
slot 2, uvs slot 3 (strides 12, 12, 16, 8; `POSITION0` f6, `NORMAL0` f6,
`COLOR0` f2, `TEXCOORD0` f16), the indices as a ring `R16_UINT` buffer, the
bytes as the arrays hold them (the census compares them byte for byte).
In these frames every drawn segment used its own arrays (slot `+4`).

The table (the one `g_VFXRenderDataTable` `0x00ccca6c` points to,
`docs/formats/render.md`)
also keeps a delete pool: count `+8` (bit 31 a flag), keys `+0xc` (8 bytes
each), counters `+0x14`, geometries `+0x1c`
(`ClassVFXRenderDataTable_UpdateDeletePool` `0x0069fe50` counts them down
and clears their segments).

## Shaders

The shader is picked by the geometry's VSP flags (`+0x3c`): 
`VfxShaderManager_FindShader` (`0x006c06e0`) looks the texture name up in
the level's or spell's special-shader table first, then the flags in the
table that `VfxShaderManager_loadVfxShader` (`0x006c0fc0`) builds from
`ffx_data/gamedata/ps3data/shaders/vfxShader_fx.txt` (`Name.fx.phyre:VSP_A|VSP_B`
lines, the bit values in the file's own comment block: `VSP_ZMAT` 1,
`VSP_ALPHA_ADDITIVE` 2, `VSP_ENABLE_TEXTURE` 8, `VSP_ALPHA_SUBSTRACT` 0x10,
`VSP_ALPHA_SUBSTRACT2` 0x20, `VSP_ZWRITE_ENABLE` 0x40, …;
`VfxShaderManager_loadShader` `0x006c0a70`). With no match,
`ClassDynamicGeometry_SetShader_006a4800` retries with only bit 8 kept. The
census reads the same file from the archive: every effect draw's pixel
shader is the file's shader for its flags (0 mismatches, 0 unlisted flags).

| Flags | Shader (pixel; vertex) | Draws |
|---|---|---|
| `0xa` additive, texture | `vfxadditiveshader.fx`; vertex shader shared with `vfxadditivenclipshader.fx` | 836 |
| `0x8` modulate, texture | `vfxdefaultshader.fx` | 149 |
| `0x18` subtract, texture | `vfxsubstractshader.fx`; vertex `vfxdefaultshader.fx`'s | 46 |
| `0x9` zmat, modulate, texture | `vfxshapeshader.fx` | 10 |
| `0x2` additive | `vfxadditiveshadernt.fx` | 1 |

(The family of a shader is the first archive cluster holding its bytecode;
several clusters share vertex shaders.) The field capture `fw1-a` adds
`vfxadditiveshadernt`, `vfxzmatadditiventshader`/`vfxshapeshadernt` and
`vfxnzadditiveshadernt` (one draw after the UI: another list, open
question 4).

## States and constants

From `ffx-drawdiff`'s decoding (Direct3D 11 enum values), per flags and
pixel shader, over the eight frames:

| Flags, shader | Blend (src/dest/op, alpha) | Depth | Raster |
|---|---|---|---|
| `0xa` `vfxadditiveshader` | 2/2/1 (one, one, add) or 5/2/1 (src alpha, one, add); alpha 2/1/1 | test on, `LESS_EQUAL` (4), write off | cull none (1) |
| `0x8` `vfxdefaultshader` | 5/6/1 (src alpha, inv src alpha) or 2/6/1; alpha 2/1/1 | the same | the same |
| `0x18` `vfxsubstractshader` | 2/2/3 (one, one, reverse subtract) | the same | the same |
| `0x9` `vfxshapeshader` | 5/6/1 | the same | the same |
| `0x2` `vfxadditiveshadernt` | 2/2/1 | the same | the same |

Write mask 15. One flags value comes with two source factors: the
material's technique picks the blend and the pixel shader ("Blend"
below).

What `vfxadditiveshader` computes in its `OptimizedTrans` technique
(ffx-demaster `docs/effects.md`, from its DXBC through `ffx-dxbc`): the
vertex shader doubles the vertex colour (PS2 `0x80` = 1.0), the pixel
shader writes `tex.rgb × colour.rgb` and `tex.a × colour.a × fades`, each
clamped to 1 by the 8-bit target; its `Transparent` technique writes the
colour premultiplied ("Blend"). The GS
clamps `Ct × Cv` to 1.0 as well, but `At × Av` only at `0xff`, which is
2.0, so on the PS2 a vertex alpha above `0x80` can still double a
texture's colour in the `Cs × As` blend and burn it out to white; on the
PC the alpha side stops at 1.0. The HD effect textures store most glows,
trails and rings (greyscale on the PS2, alpha `0x80`) as white with the
brightness in alpha, which keeps that headroom on the colour side.

Used constants (`cb0` of each stage; reflection, used variables only):

| Shader | Vertex | Pixel |
|---|---|---|
| all | `WorldViewProjection` | `softParticleContrast`, `softParticleEnabled`, `softParticleScale` |
| `vfxadditiveshader` | | also `cameraFadeoutFactor`, `doCameraFadeout` |
| `vfxshapeshader` (zmat) | also `ViewProjection`, `zCrct`, `zMatrix` | |

`WorldViewProjection` and `ViewProjection` are PhyreEngine's derived values
(`docs/formats/scene.md`, "Constants"); `zMatrix`/`zCrct` are the z-matrix
correction (`VFXDynamicGeometry_setZCorrectionValue` `0x006a3b20`); the
soft-particle and camera-fade values are material parameters of the VFX
shaders (where they are written: open question 3). The billboard
particles' positions in the arrays are camera-facing quads with one z for
all four corners (the fill requests carry `g_pppFillMatrix`,
`docs/formats/render.md`); which matrix `WorldViewProjection` then holds
for them is for round 2 to check against the frame's camera.

## Blend (2026-11-16)

Which blend a draw uses is not in the geometry: it is the material's.
`PRendererBase::renderMeshInstance` (`0x005b9a90`, `ffx_prender::p6_mesh`)
draws each segment of a mesh instance with the material of the segment
(the instance's `+0x10`, a `PSharray` of `PMaterial`s indexed by the mesh
segment's `+0`) in the render pass the VFX list sets (`renderer +0x48`:
`"Transparent"`, `0x00c169d4`, for `DrawMostVFX`, `RenderDTTVFX`,
`RenderVFXAfterPostProcess` and `DrawUI2DVFX` alike).
`selectTechniquePass` (`0x005ba690`, `ffx_prender::mesh`) first applies
the material's pass remapping (`PMaterial +8` → `+0xc`), then takes the
effect's technique for the pass (by the pass's id `+0xc` into the
effect's table `+0x18`, else the 40-byte technique entries `+0x10` whose
first word is the pass), the context variant (the segment draw data's
cached one, else `PEffect_findContextVariant`; every VFX technique seen has
one variant) and its shader (`technique +8` plus 32 bytes a variant).
`queueMeshCommands` (`0x005ba380`) queues one draw per shader pass (the
shader's `+4` count, `+8` the first `PShaderPassStateD3D11`, the others
`0x254` bytes apart); the render thread binds each pass's states
(`PShaderPassStateD3D11_bind` `0x0058c6c0`: blend state block `+0x174`,
blend factor 0, 0, 0, 0, sample mask all ones), made from the
`D3D11_BLEND_DESC` at block `+0x60` (pass `+0x78`;
`PShaderPassStateD3D11::apply` `0x0059ad20`).

The remapping is the whole answer to the two factors:
`PhyreScene_UseVFXOptimizedTrans(material, on)` (`0x00654b20`) remaps
`Transparent` to `"OptimizedTrans"` (`0x00c16b84`) or clears it, for
every material of a geometry through `graphicUseOptimizedVFXShader`
(`0x00644ac0`). The shp and mdl fills
(`JobSchedule_pppPart_fillBuffer_drawShp_00727110`, `…_drawMdl`
`0x0071f320`) call it for each instance they fill, with `on` = no vertex
colour component above 0.5 (`ffx_glue_other::h4y_shp`, `h4z_mdl`); the
DTT fill (`ffx_field::kedtt`) with 0; others through
`graphicUseOptimizedVFXShaderByVFXInstance`. The two techniques of a VFX
effect have their own pixel shaders and blends (the untextured families
were only seen in `Transparent`).

Read live (`ffx-host`'s `vfx-probe`, headless; the capture reads the
same chain, below). Both techniques blend alpha (one, zero, add), write
mask 15 (except `0x40000000`), blend factor unused. D3D11 numbers: 1
zero, 2 one, 5 src alpha, 6 inv src alpha; operations 1 add, 2 subtract,
3 reverse subtract.

| Flags | `Transparent` (src, dest, op) | `OptimizedTrans` | Seen |
|---|---|---|---|
| `0xa` additive | one, one, add | src alpha, one, add | both, every run |
| `0xb` | one, one, add | src alpha, one, add | battles |
| `0x8` modulate | one, inv src alpha, add | src alpha, inv src alpha, add | both |
| `0x9` z-matrix | one, inv src alpha, add | src alpha, inv src alpha, add | battles |
| `0x18` subtract | one, one, reverse subtract | one, one, reverse subtract | both |
| `0x28` | | zero, inv src alpha, add | Lulu's magic |
| `0x80a` | | src alpha, one, add | Valefor |
| `0x2` additive untextured | one, one, add | | both |
| `0x3` | one, one, add | | battles |
| `0x0` | one, inv src alpha, add | | battles |
| `0x10` | one, one, reverse subtract | | battles |
| `0x1000` (DTT, `FrameBuffer`) | two passes: one, inv src alpha, add; then one, src alpha, subtract | | battles |
| `0x40000000` | zero, one, add, write mask 0 (writes no colour) | | battles |

What each pixel shader writes (DXBC of the shaders bound with those
blends, through `ffx-dxbc`'s `dump_ops`). `v` is the captured vertex
colour (`0.5` the PS2's `0x80`), `t` the texture sample, `f` the soft
particle and camera fade factor (1 when both are off); the target is
`R8G8B8A8_UNORM`, so the output is clamped to 0..1 before the blend.

| Flags | Vertex colour `c` | `Transparent` writes | `OptimizedTrans` writes |
|---|---|---|---|
| `0x8`, `0x9`, `0xa`, `0xb` | `2v` | `a = t.a c.a f`; `(a min(t.rgb c.rgb, 1), a)`: premultiplied | `(t.rgb c.rgb, t.a c.a f)`: straight |
| `0x18` | `2v` | `a = t.a c.a f`; `(a min(t.rgb c.rgb, 1), a)` | the same |
| `0x80a` | `2v` | | `t c` (no fade) |
| `0x2`, `0x3` | `(2 v.a v.rgb, v.a)` | `(c.rgb, c.a f)`: premultiplied by the vertex shader, no fade on the colour | |
| `0x0`, `0x10` | `(v.rgb, 2 v.a)` | `a = c.a f`; `(a c.rgb, a)` | |

(`0x28`'s and the DTT's pixel shaders were not decoded.)

So the `Transparent` technique premultiplies (with the alpha up to 2.0,
the PS2's `0xff`: a vertex alpha above `0x80` brightens the colour up to
twice, clamped at 1) and blends with source factor one; `OptimizedTrans`
writes straight colour for the hardware's source alpha. The fills choose
`OptimizedTrans` only when every vertex colour is at most 0.5 (`c` and
`a` at most 1), where the two give the same result. No colour or alpha
scale constant differs per draw: the shaders' constants are the
transforms, the soft particle and camera fade parameters (`f`) and the
z-matrix correction ("States and constants"); the material's white colour
parameter is not used by them, and `g_AlphaScale` is in the vertex colours
already.

At Besaid (developer save 7, 300 frames, each draw the same every frame):
the sea's strips (keys lo 7 and 9) draw `OptimizedTrans` but one
modulated strip (key `(7, 0xd2afa8)`, `Transparent`); the other beach
effects (keys lo 6 and 8) `Transparent` (one, one), but the subtract
strip (`0x18`, `OptimizedTrans`); the save sphere's orb hemispheres and discs (flags
`0x8`, map texture `14336`) `Transparent` (one, inv src alpha), its two
sparkle quads (flags `0xa`, `yonishi_data/dat_et/bat_eff/et_tex` texture
`15456`) one `OptimizedTrans` (src alpha, one, key `(6, 0xd29c68)`) and
one `Transparent` (one, one, key `(8, 0xd29da8)`), its untextured glow
(`0x2`) `Transparent` (one, one). Battles (field 34, saves 51 and 12
driven: black magic, a summon, Valefor's overdrive, 4,000 and 6,000
frames): the table above, every draw's blend read.

Checked against the draws the game makes: with a trace of every D3D11
draw's bound blend state (the graphics layer's `OMSetBlendState` objects,
headless, null device), the capture's per-segment blend states and index
counts appear in the same order as the game's draws (runs of up to 21
consecutive draws on Besaid), and the DTT's second pass is drawn as often
as the first (26,308 times each in the Valefor run).

## Texture coordinates (2026-11-16)

**The shaders use the uvs as the arrays hold them.** Every textured VFX
family's vertex shader copies `TEXCOORD0` to its output unchanged
(`mov o2.xy, v3.xy`: `vfxdefaultshader` for `0x8` and `0x18`,
`vfxadditiveshader`'s, shared with `vfxadditivenclipshader`, for `0xa` and
`0xb`, `vfxshapeshader` for `0x9`), and every pixel shader of those
families samples `TextureSampler` (`t1`) with `TextureSamplerState` (`s0`)
at the interpolated value (`sample r0, v2.xy, t1, s0`), its first
instruction (`ffx-dxbc`'s `dump_ops` on every container of the five
files). No texture matrix, constant, second uv stream or per-draw value
takes part: the shaders' constants are the ones in "States and
constants". An effect's frames are in the arrays: the fills write each
frame's uvs (the field's strips and the save sphere's orb scroll theirs
every update; the sphere's sparkle quads keep one block, the same over
updates 60-210).

**`v` counts from the texture's bottom row** (file textures; the DTT
draws sample `FrameBuffer`, the frame copy at pp `+0xa8`, a render target
whose first row is the screen's top, and the `0x28` family's shaders were
not decoded: for those the convention is unverified). The game uploads a
texture's rows as the file stores them, bottom row first, with no flip
(docs/formats/scene.md, "Asset parity": `0x0058fc20`), and D3D11 samples
the first row in memory at `v = 0`. `ffx_phyre`'s decoders, its DDS export
and the PNGs `ffx-import` makes turn the image upright (top row first), so
on those a captured `(u, v)` samples `(u, 1 - v)` (`vfx::upright_uv`). A
frontend that uploads the stored rows (`Texture::pixels`) uses the uvs as
they are. The same holds for the scene's own meshes (the scene builder
uploads the stored rows); the UI draw flips `v` in its fill instead
(docs/formats/ui-draw.md).

Seen at Besaid (developer save 7): the save sphere's two sparkle quads
(flags `0xa`, keys `(6, 0xd29c68)` and `(8, 0xd29da8)`) sample
`yonishi_data/dat_et/bat_eff/et_tex/tex/D3D11/15456_19_0_0_256_1024.dds.phyre`
(512 x 2048 in the file: animation frames of 64 texels, 8 a row, then
larger images) at u
0.0039..0.2461, v 0.4385..0.4990. Corners `(0.0039, 0.4990)`, `(0.2461,
0.4990)`, `(0.0039, 0.4385)`, `(0.2461, 0.4385)`: texels (2, 1022), (126,
1022), (2, 898), (126, 898) from the first stored row, which are rows
1026 and 1150 of the upright image. That block, x 2..126 and upright y
1026..1150, is one of the larger images: one soft white ball (alpha 255
in the middle, 0 at the corners), the glow the game shows around the
orb. Read top-down (upright rows 898..1022) the block covers four
64-texel frames of other animations (two lightning frames, a ball and a
ring of dots), clear in the middle (alpha 2). `tests/vfx_uv.rs` checks both on the
archive's texture.

**Each draw has its own sampler.** The pixel shader's `s0` is the
material's `TextureSamplerState`: a tweakable definition of the
material's effect (`+0x20` count, `+0x24` 16-byte definitions: `+2` the
type, `0x47` a sampler state, `+4` the name, bit 0 a flag, `+8` the
offset in the parameter buffer), whose slot in the parameter buffer
(material `+4`) is `{0x47, 0, PSamplerState, 0}`;
`PShaderProgramBindingD3D11::setTextureParameters` (`0x0059b3d0`) binds
the state's D3D11 object (`+0x20`). The VFX effects also declare
`GlobalSamplerState`, `NormalMapSamplerState` and `ZBufferSamplerState`
(the last for the soft particles' depth read, `ld`, no sampler). The
description is `PSamplerStateD3D11::create`'s (`0x00595bf0`): filter by
`0x0058aa10` from the min filter byte `+0` (0, 2 point; 1, 3 linear; 4
point with linear mips; 5 linear with linear mips), the mag filter byte
`+1` (non-zero linear, `+4`) and the comparison bit 6 of `+0x1c` (`+0x80`),
anisotropic (`0x55`) whenever the renderer's maximum anisotropy
(`0x00c25370`) is over 1; the address modes from the wrap bytes `+2..+4`
(`0x0058ab10`: 1 wrap, 3 border, 4 mirror, else clamp); the bias `+8`,
the anisotropy `+0xc`, the border bytes `+0x10..+0x13`. `ffx_host::vfx`
captures it (`Draw::sampler`, FXDR version 5).

At Besaid every field effect of `map/bsil/bsil00/fp/tex` (the sea's
strips, the orb's hemispheres and its disc) is `0x15` (min, mag and mip
linear), wrap, and the uvs need it: the strips run u 1.0..2.4, v
-1.83..-0.14, the hemispheres u 1.2..2.98, v -1.98..0.5. The sparkle
quads (`bat_eff`) are `0x15`, clamp. Checked against the game's own binds:
a trace of the bound pixel sampler at every `DrawIndexed` (the graphics
layer's `CreateSamplerState` descriptions, headless, null device; a
throwaway patch, not kept) gives, in draw order, wrap for the 24 draws
before the sparkle quads (the strips and the hemispheres, index counts
960, 576, 720, …, 192, 192), clamp for the two 6-index sparkle draws,
wrap for the disc's two after them, all `0x15`, as the capture reads
them. The orb's hemispheres follow the same rules (the
same shader path and upload); their texture (`14336`) is a soft pattern
whose mean alpha at the vertices is about the same either way (195 with
`1 - v`, 192 without), so it does not tell the two readings apart the
way the atlas does.

In battle (field 34, save 51 driven, 4,000 frames: black magic, attacks)
the mode varies within one family and texture folder, so it is per
material, not per family or folder: flags `0xa` with `yonishi_data`
textures 130,402 draws clamp and 1,630 wrap, with `magic` textures 29,890
clamp and 2,031 wrap, with the battle map's (`btlmap`) 8,895 wrap; `0x8`
`btlmap` 15,952 clamp; `0x9` wrap; `0xb` `yonishi_data` 8,003 wrap; `0x18`
(`yonishi_data`) and `0x28` (`magic`) clamp; the DTT draws (`0x1000`,
`FrameBuffer`) clamp. Every sampler read was `0x15` (the renderer's
maximum anisotropy was not over 1 in these headless runs). The untextured families (`0x0`,
`0x2`, `0x3`, `0x10`, `0x40000000`) have no `TextureSamplerState`
(`Draw::sampler` `None`).

## Instance keys and makers

A geometry is found in the table by its 64-bit instance key, `(lo, hi)` as
the setup calls pass it (`graphicVFXGetRenderDataByInstance(lo, hi)`); a
several-part effect keeps part `i` at `lo + 10000 i`. The key says who made
it:

| Key | Maker | Setup callers |
|---|---|---|
| `(g_pppCurrentProgram, object)` | a ppp program of the ppp object at `hi` (`pppRunPartStd` `0x00716c40`) | `pppDrawShape0` `0x00740a70` and the other ppp draws (`InitVFXRenderData`, `…Shp`, `…ShapeField`, `…Line`, `…Path3…`) |
| `(0, record)` | the op effect system: the op record at `hi` (0x50-byte records in `FFX.exe`'s op area after `g_Op3dListener` `0x0113fcd0`, or the heap) | `opp_md` `0x008020a0`, `opp_anm` `0x00811120`, `opp_anmm` `0x008124b0`, `opp_anm_yodare` `0x00811bc0`, `opus_0283_draw` `0x00a76770`, `opus_0582_*`, `opu_mdc_break` `0x00804270`, `opp_md_break_init_a74410` |
| fixed pairs | `opus_0706_noise_draw` `(0x15, …)`, `opus_0706_noise_update`/`_acc_update` `(1|2, 0x2c2)`, `opus_0154_wave_draw` `(b, 0x9a)`, `setPolyTris0434`/`setPolyQuad0434` `(0|1, id)`, `AtelExecDrawShape`, `rcbgGuideUpdateWalk`, `rcefModelProc`, … | |

**Which field program.** A map's field program (scene-maps.md, "Map
effects: the field-program table") runs entry `i` of the table `g_FpData`
points to as the ppp part `g_FpRun` (`0x00d2c260`) `+ 0x80 i`
(`pppStartPart` from `pppFpLoop`), and `pppCreatePObject` links each object
it makes into that part's list: the head at part `+0x30`, each object's next
at `+0`, its birth record at `+8`. So the entry of a field-program draw is
the part whose list holds the key's object. Walking the lists of the
table's parts (the table's `u16` count is at `[g_FpData] + 4`) at the end
of the frame's effect passes (frame-timing.md, "The effects step in the
draw": `exec` passes, at least one) resolved every field-program draw on Besaid
beach (developer save 7; gd-ffx R7a, its `rust/ffx-gd/src/fxowner.rs`): all
30 draws to 15 table entries, the save sphere's glow to table entry 15
(state-api.md, "Effect draws"); other maps were not counted. In general
(worked out from the code, not seen): an object deleted in the frame's
passes after it drew (its end frame, or its entry ending: `end_objects` in
`pppRunPartFp`, `free_objects` in `pppFpLoop`) is no longer in any list,
so its draw has no entry; and if a later birth in those passes reuses its
address, the walk names that later entry instead, a wrong answer with
no sign of failure. A table entry's position
(`+0`, field units) is its place as `(-x, -y, z)` (scene-maps.md,
"Position") unless any of the low four bits or `0x100` of its `g_FpDisp`
record's `+0` is set: then the display record's matrix (`+0xc`) places it,
times the stick target's matrix for bits 1, 2 and 8 (`pppFpLoopMatrix`);
`rcbgFPmRestart` (from a character element) and `rcbgFPmStartList` (from
its list entry) set those. Besaid's entry 15 is at the origin
while its draws' world matrices hold the sphere's place, like `cdsp00`'s glow
(entry 0, bound by a worker's script); what places Besaid's was not checked.

For a ppp key the census walks object `+8` (its birth record) `+0` (the
program list), entry `index` (`+0x28 + 0x10 index`, count `+0x26`) to the
`g_pppProgramTable` record. A list inside a spell overlay is read from
the overlay's file (`magicFiles/FFX/magic_<id>.dll`, mapped at the base in
`g_MagicFile_Handle`; snapshots leave modules out): on disk an entry's
first word is the table index, which the loader turns into the record's
address (magic 481's shape part: `pppPoint`, `pppAngle`, `pppScale`,
`pppColor`, `pppMatrixXYZ`, `pppDrawMatrixFront`, `pppDrawShape`,
`pppVertexAp`).

Makers over the eight frames:

| Frame (overlay) | Effect draws | ppp | op | other |
|---|---|---|---|---|
| battle-magic-1 (481) | 192 | 189 | 3 | 0 |
| battle-magic-2 (481) | 206 | 188 | 18 | 0 |
| battle-magic-3 (481) | 353 | 349 | 3 | 1 |
| battle-overdrive-2 (390, Spiral Cut) | 37 | 4 (object in the overlay's image) | 32 | 1 |
| battle-overdrive-3 (595, Valefor's entry) | 21 | 0 | 21 | 0 |
| battle-overdrive-4 (595) | 21 | 0 | 21 | 0 |
| battle-overdrive-6 (604, the overdrive) | 75 | 42 | 33 | 0 |
| battle-overdrive-7 (604) | 137 | 110 | 27 | 0 |
| **all** | **1042** | **882** | **158** | **2** |

Of the 882 ppp draws, 92 name a drawing program (`pppKeLnsLp`, the
lens-flare loop, in the overdrive); for the other 786 the key's index
names a program without a draw or runner (`pppVertexAp`, `pppVertexApLc`,
`pppRandUpFV`): the draws come from that ppp object, but which program of
it made the instance is not settled (open question 1). The census marks
them `NO-DRAW`.

## A spell live: Fire (ffx-remix M9, 2026-11-07)

A trace in the 32-bit game: ffx-remix's runtime with `FFXREMIX_FX_TRACE=1`
(`Effects/EffectTrace.cs`), harness lockstep, fast mode, virtual clock,
seed 1. Lulu alone against the Thunder Plains' first formation (battle
field 34, formation 0: Larva, actor 20, and Gold Element, actor 21) casts
Fire (magic `0x4c`, `magic_0076.dll`) on the Larva. The runs' probe raised
the monsters' HP to 99999; its writes meant for Lulu's HP landed at her
`+0x674` and `+0x6b0`, inside `limits` (`+0x670`). Hooks on
`FFXApplication_animate` (one count per update; the magic ID and handle
read at the start of each update, before the game's own),
`PhyreScene_DrawMostVFX` (each list entry, its geometry through the table
and its instance key, labelled with the update count of the moment),
`MsSetDamage` (logged when it returns), `MsNumberRegist` and
`MsEffectReady`. Counting from u0, the first update that starts with
`g_MagicFile_MagicId` = `0x4c` (it is set during update u0 - 1):

| Update | What |
|---|---|
| u0 | starts with the magic ID `0x4c` |
| u0 + 1 | starts with `g_MagicFile_Handle` valid |
| u0 + 50 | `MsEffectReady(0x205, 1)` (kind 2, actor 5) |
| u0 + 79 | the spell's own first draws (draws labelled with this count) |
| u0 + 89 | `MsNumberRegist(0x14, …, 170, …)` and `MsSetDamage(5, -5, 0)`: the hit and its damage number |
| u0 + 93 | `MsSetDamage(5, -5, 0)` again |
| u0 + 129 | the spell's own last draws |
| u0 + 138 | `MsSetDamage(5, -5, 0x400)` (the command's end); the magic ID is -1 at the start of u0 + 139 |

Seven traced runs (ffx-remix `m9/probe` trace1, trace2, live1, live2,
exit-vanilla, exit-fire, exit-fire2) had the same updates for the calls;
the draw and handle rows are from the six after trace1 (its tracer did not
yet read the table through the pointer or log the handle's change). The
traced runs with ffx-remix's hide (live1, live2, exit-fire, exit-fire2)
log the spell's draws too: the tracer reads the list before the hide
zeroes it.

The hit most likely comes from the overlay itself (inferred: the tracer
did not log the caller): `magic_0076.dll` has one call to `MsSetDamage`
through `g_MagicImports` slot 239 (`mov eax, [imports+0x3bc]` at
`0x100064b9`, arguments `(…, -5, 0)`), in a timer routine that steps 1/30
a call and calls it at its stages' thresholds 0.35 and 0.5, which fits two
calls 4 updates apart (u0 + 89, u0 + 93). `FFX.exe` has one other caller
with the constants `(…, -5, 0)`, `MsEffectSignalDamage_007b7680` (reached
through `MsEffectSignal_007b74d0`, from the motions' effect signals and
native `0x709e`); the `0x400` call at u0 + 138 has no such site in the
overlay; the only `MsSetDamage` call in `FFX.exe` passing a constant
`0x400` is `MsBattleProcess_ChrDamage`'s (the call at `0x007912cb`, `push
0x400` at `0x007912c3`) (`docs/formats/battle.md`; the tracer did not log
the caller). That `MsNumberRegist` runs inside that `MsSetDamage` is
inferred from the log (the number's line before the call's, which is
written as it returns, in the same update). Hiding all of the spell's
draws (below) left every row above on the same update, the target's HP
changed on the same harness frame, and frame-by-frame shots showed the
number first on the same shot (exit-vanilla against exit-fire and
exit-fire2).

**Which draws are the spell's.** In the 142 drawn frames from u0 - 1 to
u0 + 140 (the tracer's window: it logs three frames with the ID -1; the same
in each of the six runs that read the table), 4,680 entries are the
spell's: ppp keys (`(g_pppCurrentProgram, object)`) whose object, and its
birth record's program list (object `+8`, `+0`), lie inside the overlay's
image (the module handle up to `SizeOfImage`); the two tests never
disagree, and all of them use the overlay's textures
(`magic/magic_0076/tex/...`). The other draws of those frames: shared
battle effects, ppp objects outside the image with
`yonishi_data/dat_et/bat_eff/et_tex` textures (20-62 a frame; 21-31 while
the spell's own draws are on screen); 2-5 untextured ppp draws a frame
outside the image (texture name `NoTexture`; flags 2 or 3, programs 4, 5
and 12); two entries in all (at u0 - 1 and u0) with the texture name
"empty"; and the battle map's field programs (`btlmap/kami/kami00_a/fp`
textures). No op-record (`(0, record)`) draws.

**List entries missing from the table.** In each of the six runs, 83 of
the 142 frames have 1-16 entries of `+0x38cc` whose mesh instance is in no
table entry (329 entries, 212 addresses, 154 of them in one frame only).
`graphicVFXDestroy` takes a geometry out of the table at once
(`ClassVFXRenderDataTable_Remove_0069fc10`: the map node erased, the
geometry to the delete pool), but leaves its list entry: its virtual
`+0x10` (`VfxRenderData_pushPair_006a0f50`) only queues the mesh instance
on scene `+0x3894`, and `PhyreScene_doDeleteMeshInstances` (`0x0065a620`)
zeroes the entry (`PhyreScene_removeVFXFromRenderArray`) after the frame's
render. The entries missing from the table are most likely instances
between those two steps. A hide by instance key therefore remembers the
spell's entries while the spell plays: in exit-fire2 ffx-remix's hide
zeroed 4,891 entries (a running count, Fire's alone in that run) against
the 4,680 the tracer resolved as the spell's; the other 211 are most
likely remembered entries whose geometry had left the table (exit-fire2
has 224 "?" entries from u0 + 79 to u0 + 140); the count also covers one
frame after the tracer's window. The remembered addresses must be dropped
a few frames after the spell: the game reuses them for other effects'
geometry (an earlier version that kept them hid later effects' draws).

**Zeroing list entries** of `+0x38cc` (count `+0xf8cc`) before
`DrawMostVFX` runs (and putting them back after) hides those draws: the
loop skips zero entries and the draw events still run at their places
(read from the code: no Fire frame had an event).

**Where the actors are.** The caster is the actor with a command cue entry
(`+0xde5` not `0xff`) and an effect state (`+0xdfb` not 0; sampled 16, 2,
4, 5 over Fire; 16 appears nowhere in battle.md's `MsEffectStart` section,
3 and 11 were not sampled); its targets are the bits of its `+0xdc4`
(`docs/formats/battle.md`). A battle character's place in the HD scene
(the space the draws' world matrices use) was taken as `(-x, -y, z)` of
`Ch_GetPos` (chr `+0xc`), as on field maps: an effect placed there sat on
the Larva in the live frames (checked by eye, run live3). The centre of
the character's mesh bounds (the render instance at chr `+0x830`, each
mesh instance's box) is not that place: in live2 it lay 31.8 units from it
for the Larva, 51.4 for the Gold Element and 55.4 for Lulu, and an effect
placed by it stood on the ground beside the Larva.

## Live: the capture in ffx-host (2026-11-16)

`ffx_host::vfx` (docs/state-api.md, "Effect draws") reads the
`DrawMostVFX` list of every frame as the census reads a snapshot, for
gd-ffx. What it adds to the census:

- **The world matrix.** A list entry is a mesh instance whose `+4` points
  at its world matrix: 12 floats in the layout of the camera's
  (`PMatrix4_toWorldMatrix3x4`: `[Y.x Y.y Y.z X.x | Z.x Z.y Z.z X.y | P.x P.y
  P.z X.z]`, render space). The vertex arrays are the effect's local
  coordinates: for Besaid's field effects the matrix flips the field's x
  and y (the sea's strips: P = (0, -1.8, 0)), for the save sphere's orb it
  holds the sphere's place, a rotation and a scale of about `3e-4` on
  positions up to 9,448. (The census compared array bytes, which the
  matrix leaves alone.)
- **The texture name** at segment `+0x20` is a pointer to the path as the
  game opened it: `../../../FFX_Data/GameData/PS3Data/map/bsil/bsil00/fp/tex/D3D11/13824_19_0_0_128_128.dds.phyre`;
  `NoTexture` for untextured segments (flags 2), whose UV arrays hold
  `0xcdcdcdcd` (unset).
- **The timing** in `ffx-host`: `DrawMostVFX` runs on the game's main
  thread after update `n` and before the boundary of update `n + 1`, and
  draws the arrays the boundary of update `n` reads (60 frames on Besaid:
  every draw equal to the same update's boundary read, 29 of 30 different
  from the neighbours'). The census's "frame k draws the arrays of the
  snapshot taken k + 1 frames on" counts from the capture's held point,
  one frame before the snapshot: the same pairing.
- **The blend** of each draw: the material's technique and the blend of
  each of its shader passes ("Blend" above), read from the material
  chain as `selectTechniquePass` walks it (FXDR version 4).
- **The sampler** of each draw: the material's `TextureSamplerState` as
  D3D11 describes it ("Texture coordinates" above; FXDR version 5).
- **The arrays** the draw copies: the current buffer's slot of each
  segment, else its own arrays ("Which arrays the draw reads" below;
  the same bytes wherever measured, FXDR unchanged).
- **Field effects.** Besaid at the save sphere (developer save 7): 30
  draws, all from `DrawMostVFX`, no draw events, every entry in the
  table. The save sphere `f001`'s glow is six entries (two hemispheres,
  two sparkle quads with `bat_eff` textures, two discs), on the sphere.

## Draw events (2026-11-16)

A draw event is a 16-byte record in the scene's `PArray` `+0xfb54` (count
`+0xfb54`, capacity `+0xfb58`, data `+0xfb5c`), with a count per kind at
`+0xfb60 + 4 kind` (cleared by `PhyreScene_clearArrays_00659000` with
the scene's other arrays). `+0` the kind, `+4` the place: the main list's
count `+0xf8cc` when the event is added, plus the current sync path's offset
(`+0xf910 + 4 [+0xf954]`) when one is open. `DrawMostVFX` draws the
entries before the place, runs the event, and goes on: an event of place
`p` runs after the draws of places `< p` and before those of place `p`.
`+8` and `+0xc` (words 0 and 1) depend on the kind. Three functions add
them: `PhyreScene_addVFXDrawEvent` (`0x00656940`, a record built by the
caller), `PhyreScene_addChrDrawSituation` (`0x006564a0`, kind 0) and
`PhyreScene_addCopyBufferEvent_006568a0` (kind 1). From the makers and
`PhyreScene_processVFXDrawEvent` (`0x00666e40`, ported as
`ffx_prender::h3_glue51`):

| Kind | Makers | Word 0 | Word 1 | What the pass does |
|---|---|---|---|---|
| 0 | `graphicCharacterInsertDrawSituation` (`0x00639ff0`) → `addChrDrawSituation(scene, on)` | the layer, `(i8) on` | the maker's junk | with the scene camera (`+0xdc`) set, every character instance of `+0xfb50` (count `+0xfb48`) that is shown (`+0x14`) and of that layer (`(i8) +0x15`) drawn by `PhyreScene_vfxEventMeshPass_006673b0` (the camera's matrix `+0x34` put back after each), then `endScene` and the targets, viewport and scene back. The same call opens a sync path of the main list (`+0xf954` up). |
| 1 | `graphicVFXAddCopyBufferEvent(a, b)` (`0x00644be0`), `…WithClear` (`0x00644c10`), `…WithClearRGB` (`0x00644c40`), `…WithRGB` (`0x00644c70`), all → `0x006568a0(a, b, clear, rgb)`; none while `g_GraphicSkipFrame` | `a \| b << 8 \| rgb << 16` | `clear` | skipped while the motion blur is on and `b` is 1. `endScene`; source by `a`: 1 pp `+0xa8`, 2 pp `+0x1232c`, else pp `+0x8c` (the colour); destination by `b`: 0 pp `+0x8c`, 2 pp `+0x1232c`, else pp `+0xa8`. With `rgb` (`+0xa`) `PostProcessManager_copyDepthBuffer` (pass `0x00ccc8ec`, `ColorScale` 1, 1, 1, 1), else `PostProcessManager_copyBuffer`. Then the targets back; with `clear` the scene begins cleared to 0 (`setClearColor(0)`, `beginScene(0xb)`). |
| 2 | `graphicVFXAddScreenFadeEvent(r, g, b, a, alpha)` (`0x00644d40`; `op_flash_path3` among its callers) | the bytes R, G, B, A | `calculateVFXSP_AlphaFunction(alpha)` | `endScene`; the colour `bytes / 128` to the post-process's fade colour (`setColor4`), word 1 to its alpha mode `+0x1239c` unless the fade is locked (`+0x123a0`); `PostProcessManager_renderScreenFade` (`0x00697490`: mode 2 pass `0x00ccc96c`, 0x10 `0x00ccc97c`, else `0x00ccc95c`; VSP bits: 2 additive, 0x10 subtract, else the alpha blend); the targets back |
| 3 | `Phyre_PS2LightingSetup_registerWithPointLightEffect` (`0x006e5c90`) | a character instance | junk | `PS2LightingSetup_apply(instance)` (`0x006e5a40`); nothing drawn |
| 4 | `SpecularMapParamsManager_AddSpecularEvent` (`0x006c56f0`; not for magic 0x245, 0x295, 0x289, 0x242), from the specular set-ups `0x006c5b80`, `0x006c5c90`, `0x006c5e30`, `0x006c7090` | a character instance (the key of the manager's slot) | junk | `SpecularMapParamsManager_renderCorrespondSpecular(instance, renderer)` |
| 5 | `graphicVFXAddDrawCharacterEvent(chr, flag)` (`0x00644ca0`) | a character instance | `flag` (a byte; the rest junk) | the instance shown (`graphicCharacterInstanceSetVisible`), drawn by `0x006673b0` (`flag`: the `TransparentClipY` pass twice; else `Opaque`, `Transparent`, `postTransparent`), `endScene`, hidden again unless it was shown, the targets back |
| 6 | `graphicVFXAddDrawZPrePassEvent(chr)` (`0x00644cf0`) | a character instance | junk | each mesh instance of the character instance drawn with the `ZPrePass` pass (`0x00c16a74`) |

A character instance (what kinds 3-6 name, `+0xfb50`'s entries,
`graphicCharacterInstanceAddToPipeline`): `+0` its mesh instances' count
(bit 31 a flag), `+4` their array, `+0x14` shown, `+0x15` the layer
(signed), `+0x18` its `g_ChrArray` record (`Ch_GetId` reads its `+0`),
`+0x1c` a callback `0x006673b0` calls first. The capture gives the record's
slot (`vfx::Event::actor`; docs/state-api.md, "Effect draws").

## The other lists live (2026-11-16)

`ffx_host::vfx` reads the other lists at the same point as the main list
(`DrawMostVFX`'s entry; the same updates fill them and
`PhyreScene_render` draws them later in the same frame) and tags each
draw with its pass (`vfx::Pass`): `RenderDTTVFX` right after the main
pass and the DTT copy (not while the menu capture state is 2, which
skips the post chain), `RenderVFXAfterPostProcess` after the
post-processing, `DrawUI2DVFX` in the UI step (only while the scene or a
movie fades, without the mesh instances named `[0x00ccb9e4]`; on maps
0x163/0x164 in the late step instead, after the post-processing and
before the screen fade and the after-post VFX), and `+0x3880`, which no pass draws.

Probe runs (ffx-host's `vfx-probe`, headless, null device, virtual clock;
battle field 34 formation 0, `kami00_00`: Larva and Gold Element; the
`--drive` runs hold the monsters' HP at 99999 and press each command:
Yuna summons, Lulu casts, Valefor uses his overdrive, the others attack):

| Run | Frames | Events | Other lists |
|---|---|---|---|
| save 7 at the save sphere, 600 frames from the load (the map's fade-in) | 600 | none | none |
| save 7, battle, auto-battle on | 3000 | kind 2: 46 frames | DTT: 710 frames, 6,640 draws |
| save 7 (Tidus, Wakka attack), battle, driven | 4000 | kind 2: 46 frames | DTT: 1,047 frames, 9,994 draws |
| save 51 (Lulu casts, Kimahri and Auron attack), battle, driven | 4000 | kind 2: 46 frames | DTT: 3,418 frames, 34,517 draws |
| save 12 with the party Yuna, Tidus, Wakka (`g_SaveInParty` written), battle, driven: Valefor summoned, his overdrive | 6000 | kind 2: 46 frames; kinds 3 and 4: 176 frames (updates 1347-1522: two kind 3 and one kind 4 a frame) | DTT: 2,660 frames, 26,308 draws |

No run had an after-post, UI 2D or `+0x3880` draw, none an unresolved
entry.

- **Kind 2, the battle's fade-in**: from the battle's first drawn frame,
  46 frames of black (`words[0]` `0x80000000`: R, G, B 0, A 0x80, 1.0;
  `words[1]` 0x200, the alpha blend), alpha 1.0 for 30 frames, then down
  by 1/16 a frame to 0. Its place moves with the list (0 to 27).
- **Kinds 3 and 4, Valefor's overdrive**: at place 5, all three name the
  same character instance, actor slot 10, character id `0x3001` (kind 3,
  an aeon: Valefor; the same in a second run of 1600 frames), in that
  order: the PS2 point light applied twice, then the
  specular pass of the same character.
- **DTT**: every draw flags `0x1000`, texture `FrameBuffer` (the DTT
  copy pp `+0xa8` sampled), 9 vertices and 24 indices a segment, keys in
  several parts (`lo + 10000 i`). Which effect makes them and which
  `vfxShader_fx.txt` line flag `0x1000` picks were not looked up.

## Which arrays the draw reads (2026-11-17)

A segment holds its arrays twice: its own (`+4` … `+0x1c`) and one slot
per dynamic-geometry buffer (`+0x2c + 0x1c n` …). From the ports:

1. **The fill.** `graphicDynGeoMemManagerGetBufferForSegment`
   (`0x0063fdb0`, `ffx_graphic::managers`) takes one block from the
   current buffer of `g_DynGeoFillBuffer` (`0x00cdedf0`) and writes the
   same counts and array addresses to the segment's own words and to the
   slot of the current buffer (`graphicDynGeoMemManagerGetCurrentBufferId`
   `0x0063ff50`: the manager's set `+4`). The fills then write their
   vertices through the own words.
2. **The swap.** Once per drawn frame `FFXApplication::afterGraphicSync`
   calls `graphicDynGeoMemManagerSwapBuffer(1)` (`0x0063ff70`): the next
   of the two buffers becomes current, its fill position goes back to its
   start (`DynGeoMemManager_swapBuffer_006dece0`), and
   `ClassVFXRenderDataTable_ClearVFXDrawData` (`0x0069f2b0`) clears that
   buffer's slot positions (`+0x34 + 0x1c n`) in every segment of the
   table. While the frame is held (a pause, a busy load, the free camera)
   it does neither.
3. **The draw.** `ClassDynamicGeometry_updateRenderData` (`0x006a4a50`)
   fills the dynamic mesh's draw table
   (`ClassDynamicMesh_fillDrawTable_006732a0`, `ffx_pother_b::p4_mesh`):
   the counts of the current buffer's slot when the buffers are on
   (`g_DynGeoBuffersOn` `0x00c32694`) and the slot's `+0x34` is set, else
   the segment's own. `ClassDynamicMeshInstance_populatePreprocessData`
   (`0x00673b20`) keeps the render data and the current buffer id with the
   draw (`+0x10`, `+0x14`), and `ClassDynamicMesh_fillDynamicBlocks`
   (`0x00672b80`, `ffx_pother_b::m4r_dynblocks`) copies the vertex streams
   and indices into the blocks the renderer binds from the slot of that id
   when its `+0x34` is set, else from the segment's own words; with no id
   or the buffers off it draws nothing.

So the draw of an update reads the slot of the buffer that update filled:
the render thread can draw it while the main thread's next update fills
the other buffer and moves the segment's own words there. A segment not
filled in an update has its slot cleared and is drawn from its own words:
the arrays of its last fill, intact in the other buffer only when that
fill was the update before; filled two or more updates back, its words
point into the buffer being refilled now, holding other effects' data.

`ffx_host::vfx` reads the arrays the same way (`arrays_at`: the slot of
the current buffer id at `DrawMostVFX`, on the main thread after the
update and before the swap, else the own words). Where the game would
draw nothing (no buffer id, or the buffers off) the capture still reads
the own words; no run had that.

**Measured** (`vfx-probe`, a throwaway read of both slots beside the own
words, not kept): developer save 7 at the save sphere (60 frames, 1,800
segment draws), save 29 at Luca (120 frames, 30,613), save 8 walked out
of the lodge to Besaid village's `bsvr00` (`--script`, 60 frames). On
every segment draw the current buffer's slot held exactly the segment's
own counts and arrays (the ids alternating 0, 1 frame by frame), and no
drawn segment had an empty slot. The capture's bytes are the same with
either read.

### Vertex colours the game never wrote

The unset (`0xcdcdcdcd`, the allocator's fill of buffer memory no fill
has reached yet) and stale colours that gd-ffx found are in the arrays the
draw copies, on vertices that only **degenerate triangles** index: a
fill reserves its vertices and indices (`+4 += 4n`, `+8 += 6n`, as
`updateVFXRenderDataShp` does) and leaves some quads collapsed, their six
indices all 0 (one vertex), so D3D11 draws nothing for them. Examples:

- Besaid village's lens flare (`bsvr00`, key `(0xd, 0xd2bea8)`, texture
  `13888_19_0_0_128_64`, flags `0x80a`): 16 vertices with positions and
  uvs, every colour unset, all 24 indices 0 in each of the 60 frames; the
  flare's other parts (keys lo `0xa`, `0xb`, `0xc`) likewise in every
  frame where they hold odd colours (`0xcdcdcdcd` or stale numbers).
- The save sphere's orb hemisphere (save 7, key `(6, 0xd2a1f8)`, 128
  vertices, 192 indices): vertices 12-15 and 108-111 unset (positions
  set, colours and uvs `0xcdcdcdcd`), no index names them; their quads'
  indices are 0.
- Luca (save 29): battle-effect quads (`bat_eff` texture `15456`, keys lo
  8, 4 vertices, 6 indices) whose indices are all 0 and vertex 0's
  position `(0, 0, 0)`, its colour another effect's numbers.

| Run | Draws with odd colours | Odd vertices | In a triangle with area |
|---|---|---|---|
| save 7, 60 frames | 60 | 480 | 0 |
| save 29, 120 frames | 517 | 854 | 0 |
| save 8 to `bsvr00`, 60 frames | 208 | 6,193 | 0 |
| save 51, battle field 34 formation 0 driven (`--drive`), 2,000 frames | 6,571 | 278,966 | 143, in 54 draws of flags `0x40000000` |

(odd: a component not finite or outside 0..8, with a tolerance of 1e-3
below 0; with area: three different
indices and positions not on one line; the probe's tally, printed after
every run.) The battle's exception writes no colour either: flags
`0x40000000` are the five untextured full-screen quads of keys
`(1..5, 0)` (positions `(±1, ±1, 1)`, two triangles each) in the
battle's first updates (157-260 of that run), drawn with blend (zero,
one, add) and write mask 0 ("Blend"), so their vertex colours never reach
the target; 54 of the run's 100 such draws had odd colours.

The zero indices are most likely written by the fills (the buffer
memory is never cleared, and unwritten words hold `0xcdcdcdcd`); which
code writes them for the flare's hidden parts was not traced. A frontend
may drop triangles with a repeated index (they
draw nothing in the game either) and need not draw the `0x40000000`
family's colour.

## Gate

The M4 census: every effect draw of the eight captured frames is traced:
1042 of 1042, to the `DrawMostVFX` list place it is drawn from (1018 by
the list entry, 24 by a zeroed entry and the delete pool), the geometry
and segment whose bytes it draws, the instance key and its maker (ppp
object or op record), and the shader its flags give (`cargo test -p
ffx-fxdraw --test captures`: "8 frames, 1042 effect draws traced"; no
unexplained draw of any class in the frames). `tests/synthetic.rs` checks
the census itself on a synthetic frame.

## Open questions

1. **Which ppp program made the instance.** (Which *spell* made it: see "A
   spell live: Fire".) The key's low word is
   `g_pppCurrentProgram`, but for 786 draws it names `pppVertexAp`,
   `pppVertexApLc` or `pppRandUpFV`, which have no draw (`pppVertexAp`
   births particles at vertices, `pppBornVertex`). Either the born objects
   draw with the parent's index left in `g_pppCurrentProgram`, or the draws
   run later (the OT list, `pppOtFunc_Control` `0x00716ac0`). A coverage
   run of `pppDrawShape0` with the program index and object logged settles
   it.
2. **The blend factors**: settled ("Blend", 2026-11-16): the material's
   technique (`Transparent` or `OptimizedTrans`, set each fill by
   `graphicUseOptimizedVFXShader`) picks the blend and the pixel shader;
   `ffx_host::vfx` reads both per draw. Left open: the DTT shader's two
   pixel shaders (not decoded), and what other callers of
   `graphicUseOptimizedVFXShaderByVFXInstance` decide.
3. **The VFX material constants** (`softParticle*`, `cameraFadeout*`,
   `zCrct`): who writes them and when.
4. **The other lists and the events not seen live**: the live capture
   ("The other lists live") reached the DTT list and events of kinds 2, 3
   and 4; no run reached `+0x3880` (no draw pass found for it), the UI 2D
   or the after-post list, or events of kinds 0, 1, 5 and 6 (their words
   are decoded from the code only). The field capture's
   `vfxnzadditiveshadernt` draw after the UI comes from one of those
   lists. Candidates: field fades and movies for UI 2D (its pass draws
   only then), effects whose geometry has VSP bit 8 or a set
   `0x00ccca58` for after-post, other summons and overdrives, the
   aeons' entries and event scenes for kinds 0, 1, 5 and 6. The UI 2D
   pass draws with the scene's second camera (`+0x500`), which the
   capture does not read.
5. **The op records**: name the op system's record layout (0x50 bytes,
   `FFX.exe` area after `g_Op3dListener`) and which `opp_*` function each
   record belongs to; the census stops at the record.
6. **The late world** (scene `+0x9c` and the post passes): the scene
   builder's, with the summon's character drawn after the glow.
7. **PC2 captures** use other saves and timing (above); the main
   machine should take the git scripts' captures and run the gate on them.
8. **Vertex colours the game never wrote** (found by gd-ffx, 2026-11-17):
   answered ("Which arrays the draw reads"). In 94 of the 110 developer
   saves some captured effect draws index vertices whose colour
   components have the heap fill's bits `0xcdcdcdcd` or stale values
   (the Besaid village lens flare's ring quad on `bsvr00` the clearest).
   Not a wrong slot: the draw reads the current buffer's slot, which held
   the segment's own arrays on every measured draw, and the capture now
   reads it the same way. The vertices are ones a fill reserved and left
   in collapsed quads (indices all 0): only degenerate triangles name
   them, and the only ones with area in the measured runs are drawn
   with write mask 0 (flags `0x40000000`), so the game draws no colour
   with them. Left open: which code writes the zero indices (the
   flare's hidden parts), and the other saves of gd-ffx's sweep, which
   counted by vertex (its view leaves such triangles out, the same
   pictures either way).
