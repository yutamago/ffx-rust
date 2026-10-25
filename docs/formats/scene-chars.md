# The character world: what its draws take, and from where

The census and builder spec for `PhyreScene +0x7c`, the characters' world
(M4 round 2, task m4-chars). It is the next part after the map world
(`docs/formats/scene.md`, `docs/scene-builder.md`): the same renderer
(`renderWorldCulledSorted` `0x009f3c30`, `renderMeshInstance`
`0x005b9a90`, `allocateAndFillParameterBuffers` `0x005b5a50`), with
skinning, cloth, character lighting and the blob shadow on top. Code:
[`ffx_scene::chars`](../../crates/ffx-scene/src/chars.rs) (the arithmetic),
[`ffx_scene::chars_check`](../../crates/ffx-scene/src/chars_check.rs) (the
check against captures); the inputs are the character part of the
`SceneInputs` record (`ffx_gfx::scene_inputs::CharWorld`).

```bash
# the gate (skips without FFX_GAME_DIR and FFX_SCENE_CHARS)
C=~/ffx-work/m4/captures
FFX_GAME_DIR=~/steam-games/FFX-X2-HD \
FFX_SCENE_CHARS=$C/chars-cdsp0000-a/cdsp0000-101/capture.ffxgfx:$C/chars-cdsp0000-b/cdsp0000-101/capture.ffxgfx:$C/chars-bsil0000-a/bsil0000-101/capture.ffxgfx:$C/chars-bsil0000-b/bsil0000-101/capture.ffxgfx \
  cargo test --release -p ffx-scene --test chars -- --nocapture
# one capture: a line per draw, what differs, the culling and order per pass
FFX_GAME_DIR=~/steam-games/FFX-X2-HD cargo run --release -p ffx-scene --example charcheck -- <capture.ffxgfx>...
# the raw character records
cargo run --release -p ffx-scene --example charsdump -- <capture.ffxgfx>
```

## Captures

Original code, test sandbox (no user mods, `TQ_LOW`), the inputs hook of
this round: `~/ffx-work/m4/chars/capture.sh <map> <run>` runs
`tools/replays/draw-<map>.txt` into `~/ffx-work/m4/captures/chars-<map>-<run>`
(never in git). Two runs each of `cdsp0000` (S.S. Winno deck, shadow
solution 2: the blob shadow is in this world) and `bsil0000` (Besaid
village, solution 4: no blob shadow; Tidus and 13 NPC instances).

## Passes and clusters

`Draw3DWorld` (`0x0064fbb0`) renders `+0x7c` twice per frame, between the
map's opaque and transparent passes (`docs/formats/scene.md`, "Passes"):
pass name `Opaque` (dir 1) and `Transparent` (dir -1), the plain names even
with MSAA off (the map uses the `NoMSAA` ones). `renderWorld` is also
called for `postOpaque`/`postTransparent` on the same world; on these
frames they draw nothing into the scene targets.

The world holds two clusters:

| Cluster | What | Instances |
|---|---|---|
| world list 1 | the characters' cluster (scene `+0x11e4`, `attachChrClusterTo3DWorld`): every character's and weapon's `PMeshInstance`, each in an instance list of its own (so a snapshot slot is 0; the instance's place is its record index) | cdsp0000: 0 a 15-segment character (82 joints), 1-3 `f001` (Tidus's sword, three unskinned meshes), 4 Tidus `c001` (13 segments, 79 joints, 53 skeleton matrices); bsil0000: 14 |
| world list 0 | the ground-shadow cluster (scene `+0x28c4`, added to `+0x7c` for shadow solutions 0 and 2, `docs/formats/scene-maps.md` Q7): a `ClassDynamicMesh`/`ClassDynamicMeshInstance`, one segment | 1 |

## Draws of cdsp0000 (scene draws 100-116)

The same 17 draws in every capture (the old fw1 captures with the user's
mods have the same layout). Tied by the check (below): instance and
segment of the record, every constant rebuilt from it.

| Scene draw | Pass | Instance, segment | Model (census) | Shader (vertex) | Notes |
|---|---|---|---|---|---|
| 100-105 | Opaque | 4, 0-5 | `c001#0-5` | `phyrechrlitshader.fx#0f527c1b...` | skinned (`BoneTransforms` '1') |
| 106, 107 | Opaque | 4, 6-7 | `c001#6-7` | `#08c09273...` | skinned + cloth (`ClothWorld`, stream 5) |
| 108 | Opaque | 4, 8 | `c001#8` | `#0f527c1b...` | skinned |
| 109 | Opaque | 4, 9 | `c001#9` | `#34f5b696...` | skinned, 140 constants (specular map set) |
| 110, 111 | Opaque | 4, 10-11 | `c001#10-11` | `#08c09273...` | skinned + cloth |
| 112 | Opaque | 2, 0 | `f001#1` | `#17faea94...` | unskinned, `SgSpec_*` (the specular segment) |
| 113 | Opaque | 3, 0 | `f001#0` | `#0a8d78d2...` | unskinned |
| 114 | Opaque | 1, 0 | `f001#2` | `#0a8d78d2...` | unskinned |
| 115 | Transparent | 5, 0 | (CPU-built) | `abmapshadercone.fx` / `chry0shadow.fx` | the blob shadows of both characters, 12 indices |
| 116 | Transparent | 4, 12 | `c001#12` | `#08c09273...` | skinned + cloth |

Instance 0 draws nothing: its bounds fail `BoundingBox_IsVisible_00663a30`
(the character stands outside the frustum). Its blob shadow is still in
draw 115 (the shadow walk does not cull).

## Culling and sort

As for the map: the bounds walk, the box test on the camera's `+0x70`
matrix times each bounds' own world, the key on the view's z row (negated
for the opaque pass), `std::sort` with `compareEntries`, then per instance
the segments in bucket order (`crate::cull`, `crate::sort`). Checked per
pass (`chars_check::order_check`): the instances of the tied draws are, in
draw order, the predicted order with the instances that have no segment in
that pass left out.

| Capture | Opaque: predicted / drawn | Transparent: predicted / drawn | Culled |
|---|---|---|---|
| cdsp0000 | 4, 2, 3, 1, 5 / 4, 2, 3, 1 | 5, 1, 2, 3, 4 / 5, 4 | 0 |
| bsil0000 | 14 instances / all 14, same order | 14 / 6 | none |

The characters' render orders are 0, so the key decides; the shadow
(instance 5) sorts first in the transparent pass (far to near). Segments
skipped: the hidden byte (`+0x14`, segments 1 and 2 of instance 0
of cdsp0000), and in `Opaque` the segments whose material has no Opaque
technique (c001 #12 is transparent only).

## Per-node constants (`generateNodeParameterCaptureBuffer` `0x005b7fe0`)

The node buffer is in memory order (column-major). Every location entry of
kind `'1'` (0x31, a float4x4) is copied into the constant buffer
**transposed**, kind `'0'` (0x30, a float4x3) as 3 rows of 4 from 4 groups
of 3, and both stop at an element whose first word is `0xffffffff`: the
render thread's copy `0x0059a650`. That holds for material values too
(`LgtMatVec`, `LgtMatClr`, `ClothWorld`, `SgSpec_LightMat`, ... are
transposed on the way). `ffx_scene::chars::cb_copy`.

| Semantic | Value |
|---|---|
| 0x82 `World` | the joint slot the segment's `+4` names (`renderMeshInstance` `local_24 + seg[+4] * 0x40`): -1 is the instance world, every captured character segment uses -1 |
| 0x83 `WorldView` | `(renderer +0xa0) . World` (`PMatrix4_MulThis_004c9da0`); the same bits as the map's `View . World`, f32 sums |
| 0x84 `WorldViewProjection` | `ViewProjection . World`, f64 sums rounded once (as the map) |
| 0x85 `WorldInverse` | `PMatrix4_Inverse_004cc0c0` of `World`: Vectormath `inverse(Transform3)` (`0x004cc890`): the columns' cross products (each component `a b - c d`, one rounding), `det = (c2.x t2.x + c2.y t2.y) + c2.z t2.z` and `1/det` as f32 steps, each element times it, the translation `-((i0 tx + i1 ty) + i2 tz)` per row (`chars::affine_inverse`) |
| 0x86 `WorldViewInverse` | `WorldInverse . (renderer +0x60)` (the second matrix the record now keeps) |
| 0x8f `BoneTransforms` | per `m_skinBones` entry (segment `+8` count, `+0xc` `PSkinBoneRemap`s: hierarchy, skeleton index) `joint[hierarchy] . skeleton[skeleton]` (`PMatrix4_MulThis_004c9da0`, f32 sums in k order), type `'1'`: a 4x4 per bone (room `size >> 6 & 0x7f`, 80 on `phyrechrlitshader`), `'0'`: its first 12 floats; then `0xffffffff` in the next slot when there is room, so the constant buffer after the last bone keeps what it held |

The joints (`computeJointMatrices` `0x004f6ef0`, `0x004f6fc0`): slot -1
the instance world (`PMatrix4_FromMatrix3x4(*(inst +4))`, the identity
when `inst +0x6c` bit 0 is set), slot k = slot `parent[k]` times the
instance's joint matrix k, where `parent` is the mesh's `+0x2c` table
(`i32`, -1 the world; mesh `+0x28` count) and the joint matrices are
`inst +0xc` (mesh `+0x18` of them). The product takes the joint matrix's
three axis columns as vectors and its fourth as a point through the parent
(`0x004ca140`, `0x004c9eb0`: per row `((m[r] x + m[4+r] y) + m[8+r] z)
[+ m[12+r]]`, f32 steps) and makes the four results the columns
(`0x00450de0`); the joint matrix's fourth row is dropped
(`chars::joints`). The skeleton matrices are the mesh's `+8` array (count
`+8`, items `+0xc`): each joint's inverse bind (semantic 0x8e
`boneCount` reads the same count).

Mesh (`PMesh`) layout read here: `+0` segments (count, `+4` array of
`0x6c`), `+8` skeleton matrices, `+0x10` (55, unread), `+0x18` joint
count and `+0x1c` default matrices, `+0x20` (unread), `+0x28` parent
count and `+0x2c` parents.

Where the joint matrices come from (all ported, so a Rust frame makes them
the same way): `Ch_CalcElement` (`0x00832760`,
`ffx_chara::parts::bridge::ch_calc_element`) builds each element's matrix
and the skin matrices; `graphicCharacterInstanceSetSkeletonMatrices`
(`0x0063be60`, `ffx_graphic::state::bridge::character_set_skeleton_matrices`)
copies the entries each mesh's remap table names into `inst +0xc` of both
instances. They are relative to the parent joint: Tidus's joint 31 is a
4.3° turn about x and (2, 0, 0).

## Material constants

Every other used constant of these draws is a material entry: the live
`PParameterBuffer` of the segment's material (`inst +0x10` set, entry
`seg[+0]`), copied through the pass's location table (with the transpose
above). Bit for bit from the record on all four captures. By writer:

| Constants | Written by |
|---|---|
| `CharacterAmbientLight` | `Phyre_CharacterLighting_instantiateUniqueMaterialParameters` (`0x006a96b0`) and `Phyre_PS2LightingSetup_setupCharacterShader` (`0x006e6160`); the colour from `graphicCharacterLightingSetLightColor` (`0x0063cb00`, kind 3, `Phyre_CharacterLighting_setLightColor_006aa940`; `Sg_LightSetColor`) |
| `CharacterShadeColor` | `Phyre_CharacterLighting_setShadeColor` (`0x006ab010`) |
| `CharacterShadeCoeff` | `Phyre_CharacterLighting_setShadeCoeff` (`0x006aaef0`) through `graphicCharacterLightingSetShadeCoeff` (`0x0063cdd0`, `calcShadeCoeffWithConfidence`) |
| `Light0`-`Light2` (`m_direction`, `m_colorIntensity`) | the lighting's three `PLight`s (`setLightColor_006aa940`: colour `+0x124 + 0x10 kind`; directions from `Sg_LightSetDir` -> `graphicCharacterLightingSetLightDir`, `docs/formats/main.md`); `m_direction.w` is `0xcdcdcdcd` (heap fill, never written) |
| `LgtMatVec`, `LgtMatClr` | `Phyre_CharacterLighting_setLightMatrix` (`0x006aabf0`) from `graphicCharacterLightingSetLightMatrix` (`0x0063cb80`, `ffx_graphic::util::bridge::set_light_matrix`): one light per column, so the transposed copy gives the shader one per row |
| `ClothWorld` | `batchSetClothVertexBuffers` (`0x006365f0`, from `graphicCharacterInstanceSetClothVertexBuffers` `0x0063b740` in `FFXApplication_render`): the first 16 floats of the segment's cloth object (Tidus: 1/1000 scale, the character's facing, translation (33.886, 11.1, 81.76)) |
| `SgSpec_Color`, `_LightVec`, `_CameraVec`, `_LightMat`, `_Size`, `_Brightness` | `graphicSgSetSpecular` (`0x00643cc0`, `ShaderParamBuffer_SetValue2` `0x006386c0`) from `Ch_DrawMain` (`0x0082ed50`, `ffx_chara::draw_main`); scene draw 112 only |
| `MaterialColour`, `fgEff`, `VertexColor*`, `AlphaThreshold`, `clrScaleFactor`, `GrayScaleParam`, `hsv_scale`, `clipPlane`, the `ffx*`/`*ReservedVar*` set, `Specular_*`, `SpecularUvOffset`, texture animation (`AnimScale_MAYA*`, `layer1-6`), the shadow's `globalColorMask(N)` | the cluster's values or the game's run-time writes (`graphicSetHSVParam`, `graphicCharacterInstanceSetBTUFadeCenter`, `Ch_ModelSetAlpha`, ...; `docs/formats/render.md`); read from the live buffer |

Scene-wide entries (`scene.*`) are the map's (`docs/formats/scene.md`) and
are not re-checked here.

## Junk reads (`SgSpec_*[0].z`)

`Ch_DrawMain` hands `graphicSgSetSpecular` two vectors (x, z, junk, w)
whose third words it never writes: the light vector's `[ebp-0x3c]` and the
camera vector's `[ebp-0x2c]`. In every original capture of cdsp0000 the
camera one is `0x00000001` (the denormal 1e-45: an integer 1 an earlier
call on the main thread left in that stack slot), the light one about
5e-30 and different per run. The record holds them as the material buffer
does, so the check reproduces them. Under the junk-read policy the port
passes 0 for both, so a Rust frame's material buffer, and a builder
copying it, has 0: `crates/ffx-scene/tests/chars-allow.txt` allows both
fields (drawdiff syntax, any scene), with this cause.

## Cloth (Tidus's segments 6, 7, 10, 11, 12)

`Ch_Cloth_PerCharacter` (`0x0082e390`, a JobWorker job,
`ffx_chara::draw::bridge::ch_cloth_per_character`) runs `Sg_ClothStep`
(`0x00822de0`, `ffx_sg::cloth::bridge::cloth_step`) for each cloth
character; its vertices end up at the cloth object's `+0x4c` (`+0x44`
vertices of 16 bytes). In `FFXApplication_render`,
`batchSetClothVertexBuffers` walks the character table (`0x00ccb250`
count, `0x00ccb254` slots; a slot's `+0` instance list, `+4` a parallel
table, `+8` 0, `+0xc` 2, `+0x14` instance count, `+0x30` per instance the
cloth objects by segment + 1), and for each segment whose `+0x20`/`+0x24`
halves are not -1 and whose data block has a stream of type `0x00c98650`
uploads the cloth vertices into that block's next version (or, for the
list's first instances, hands the pointer to the deferred path at
`*cloth +0x48`). The draw binds them as vertex stream 5 (stride 16,
`COLOR1` of `phyrechrlitshader.fx#08c09273...`) from the 12 MB dynamic
ring. The record's cloth vertices equal the drawn stream on every cloth
draw of the four captures.

## The blob shadow (scene draw 115)

`Ch_DrawMain` runs the shadow walk between `graphicShadowStart_00643dc0`
(`DynGeo_ResetCurrentBuffer` `0x0069fcb0`: the manager at `0x00ccca74`
picks the current buffer and clears its lists) and
`graphicShadowEnd_00643dd0`. `Ch_DrawShadow` (`0x00830180`,
`ffx_chara::chr::bridge::ch_draw_shadow`) calls `Ch_DrawShadowQuad`
(`0x0082e3e0`) for every character in shadow mode 1, which places a quad
under it (`-facing`, position `+0x7c`, `graphicUpdateShadowMap`) and
appends it with `Ch_DrawQuad` (`0x00831d10`,
`ffx_chara::parts::bridge::ch_draw_quad`) to the lists of
`graphicShaderGetVertexInfo` (record `+0x94`: `+4` vertices, `+8`
indices, `+0xc` positions (12 bytes), `+0x10` 12 bytes (zero), `+0x14`
normals (16: 0 0 0 1), `+0x18` texture coordinates (8), `+0x1c` `u16`
indices n, n+2, n+1, n, n+3, n+2). The cluster `+0x28c4`'s dynamic mesh
draws those lists: the four arrays as vertex streams 0-3 and the indices
widened to `R32_UINT` in the index ring. cdsp0000: 8 vertices, 12 indices
(both characters, the culled one too). World is the dynamic instance's
`(-1, -1, 1)` diagonal; the pixel program binds no constant buffer, only
the 1024 x 1024 shadow target (`TextureSampler`). The record's lists equal
the drawn streams and indices.

## The scene-input record's character part

For worlds `+0x7c` and `+0xbc` the hook (`ffx-hook` `world.rs`,
`render_world`) appends a `CharWorld` after the map fields (older records
end before it; `ffx_gfx::scene_inputs`):

- per record instance: `+0x6c`, the joint matrices (`+0xc`), the parent
  table (mesh `+0x2c`), the skeleton matrices (mesh `+8`), per segment
  `m_skinBones`;
- the cloth vertex buffers per (instance, segment): character slot, list
  place, the cloth object's first 16 floats, count and vertices;
- the dynamic geometry lists of the current buffer;
- the renderer's `+0x60` and `+0xa0` matrices.

Read at `renderWorldCulledSorted`'s entry on the main thread, like the map
part: the joints, materials, cloth and lists do not change during the
pass.

## Check (`tests/chars.rs`)

For each census draw of a character model (`chr/`) or the blob shadow:
the effect variant whose pass binds the draw's programs (several variant
clusters share bytecode; the one that ties best is the material's), every
drawable (instance, segment) whose material values give the draw's is a
candidate, and the one with the most exact constants is the tie (equal
candidates: the segment the census's vertex data names). Results:

| Capture | Draws | Constants exact | Vertex comparisons equal | Order |
|---|---|---|---|---|
| chars-cdsp0000-a, -b | 17 | 1715 / 1715 | 10 / 10 | in order, both passes |
| chars-bsil0000-a, -b | 44 | 4730 / 4746 | 5 / 5 | in order, both passes |

The 16 bsil0000 constants that are not exact are `WorldInverse` and
`WorldViewInverse` of one NPC (record instance 12, four draws), 1-2 ulp:
open item 1.

## What a character builder needs next

1. **The bsil0000 `WorldInverse` ulps.** The NPC's world has
   `-0.99999994` on its diagonal; our inverse comes out 1 ulp short in
   several elements (as if the determinant were 2 ulp smaller). No
   arithmetic variant of `0x004cc890` reproduces it from the `World`
   bits, so the input may differ (a world written between the joint copy
   and the inverse, or another source): a frida read of the node's
   `+0x28` matrix and its input would settle it.
2. **Technique selection per segment** (which segments draw in `Opaque`
   and which in `Transparent`): the map builder's rule
   (`selectTechniquePass` `0x005ba690`) applies; the character materials'
   effect variants come from the character clusters (`chr/*/mdl/d3d11`),
   which `ffx_scene::map::MapModel` would have to load (not tried).
3. **Joint matrices without the record**: from `Ch_CalcElement`'s skin
   matrices (ported) and the mesh's parent table (static, in the
   cluster), with `chars::joints`; the record gives both for now.
4. **Cloth and shadow vertices** as dynamic buffers: the record's arrays
   are what to upload (vertex stream 5 at stride 16; streams 0-3 and
   32-bit indices for the shadow).
5. **The material-matrix transpose** applies to the map too: `build.rs`
   `fill_cb` copies kind-0x31 entries plainly. The map gate passes
   (cdsp0000's map materials have no such entry whose transpose differs,
   or the same writers), but a builder sharing the fill must transpose
   kind `'1'`/`'0'` entries and honour the `0xffffffff` stop.
6. **Shadow solutions 1 and 3** put cluster `+0x28c4` in world `+0xbc`;
   the hook records that world too, but no capture of this round has one.
7. **Names for `symbols/ffx.sym`** (not owned by this task):
   `0x0059a650` (the constant-buffer fill with the matrix transpose),
   `0x004cc890` (`inverse(Transform3)`), `0x004f6fc0` (the joint chain),
   `0x004ca140`/`0x004c9eb0` (Matrix4 times vector / point), `0x00450de0`
   (Matrix4 from columns), mesh `+0x2c` (joint parents).
