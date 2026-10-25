# Post-processing: what PhyreEngine draws after the scene, and from what

This is the M4 census of the post-processing (`docs/rewrite-plan.md`, M4,
"effects and post-processing"), written the way `docs/formats/scene.md`
did the scene: every draw of one field frame outside the scene targets,
put in its pass, with each pass's inputs, targets, shader family, states
and the source of every constant it reads. It is the spec for the Rust
post-process chain. Crate: [`crates/ffx-postfx`](../../crates/ffx-postfx).

The frame is field-walk-1's (`cdsp0000`, ctrl work 0's frame count 101,
original code), the reference captures `~/ffx-work/m4/captures/fw1-{a,b}`
(never in git; `docs/drawdiff.md`, "Reference captures"). Draw numbers are
`frame.txt`'s (1-based); "item" numbers are `ffx-drawdiff`'s (`items`
example), which count clears too.

```bash
# the census: one line per draw and clear outside the scene, the
# constants with their sources, the totals; with B, the run-to-run check
FFX_GAME_DIR=~/steam-games/FFX-X2-HD cargo run --release -p ffx-postfx --example census -- \
  ~/ffx-work/m4/captures/fw1-a /map/cdsp/cdsp00/ ~/ffx-work/m4/captures/fw1-b
# the gate (skips without FFX_GAME_DIR or capture A)
FFX_GAME_DIR=~/steam-games/FFX-X2-HD cargo test --release -p ffx-postfx
```

`FFX_POSTFX_CAPTURE` and `FFX_POSTFX_CAPTURE_B` choose the test's captures
(default `~/ffx-work/m4/captures/fw1-a` and `fw1-b`).

## The frame's draws

197 draws: 128 in the scene targets (`docs/formats/scene.md`), 69 outside.

| Draws | Class | By | Census |
|---|---|---|---|
| 1-128 | scene | `Draw3DWorld` | `ffx-scene` |
| 129-141 | post: glow map and glow | `PostProcessManager_render` (`0x00694f10`) | here |
| 142-176 | effects (35, `vfx*` shaders, the dynamic rings) | `PhyreScene_DrawMostVFX` | `pc-effects` |
| 177-178 | post: distortion | `Distortion_renderDistortion` (`0x006c2dc0`) | here |
| 179-180 | post: brightness | `PostProcessManager_renderBright` (`0x00695440`) | here |
| 181-196 | UI (16: the minimap, its frame and markers) | `PhyreScene_DrawNormalUI` | `pc-ui2d` |
| 197 | post: composite to the back buffer | `PhyreScene_render`'s last `PostProcessManager_copyBuffer` | here |

The census tells them apart by shader family (each shader's effect file in
the archive, by `ffx-drawdiff`'s bytecode key) and by place: effects and
UI both use `vfx*` and `menu*` shaders (the minimap's mesh, draw 182,
uses a `vfx` one, and some `vfx` and `menu` shaders share bytecode), so a
`vfx`/`menu` draw is UI once the post-effects chain (distortion,
brightness) has started, as `PhyreScene_render` orders them. The
composite is the one draw into the back buffer, the only target the
capture never declares.

Clears outside the scene (items): 0, the back buffer to (0, 0, 0, 1) at
the start of `PhyreScene_render`; 132 and 133, the post-process target
`+0x9c` to 0 (before the ambient occlusion pass and before the
`postOpaque` pass, neither of which draws here); 183, the colour before
the distortion grid (`beginScene(1)`); 186, the colour before the
brightness copy (`copyBuffer`'s `beginScene(1)`); 190, the scene depth in
the UI (the minimap); 205, the back buffer's depth (the composite's
`beginScene(2)`).

## Order

`PhyreScene_render` (`0x00668930`), the field branch, with what ran on
this frame:

1. Back buffer: `setColorTarget(0)`, clear (0, 0, 0, 1). Item 0.
2. `PhyreScene_RenderShadow` (`0x00653600`) unless scene `+0x93e`
   (nothing drawn: shadow solution 0, `scene.md`).
3. `PostProcessManager_renderHDAO` (`0x0066b560`): despite its name
   (Fahrenheit's, a weak `cg4` match), it renders the map (`+0xac`)
   top-down with an orthographic camera into scene `+0xfc70` when
   `+0xfc00` and `+0xfc01` are set, then clears `+0xfc01`. Not run.
4. The scene (draws 1-128, `scene.md`).
5. Unless `BrokenScreenEffect_isActivatedForCurrentFrame`: target `+0x9c`,
   clear 0 (item 132), `PostProcessManager_renderHDAOPass`
   (`0x00695ba0`, the HDAO, below). No draw: HDAO off.
6. `0x006ad810`, target `+0x9c` with the scene depth, clear 0 (item 133),
   `postOpaque` pass (`0x00c16a34`) of the map and characters worlds,
   unless `+0x93e`. Nothing drawn.
7. Colour `+0x8c` with the scene depth, `postTransparent` pass
   (`0x00c16a44`) of the characters and map worlds. Nothing drawn.
8. With `PhyreScene_isGlowAfterVFX` (`0x0065efa0`) 0 (as here):
   `PostProcessManager_render`, the colour correction, depth of field and
   glow (draws 129-141). Otherwise this runs after step 12.
9. Scene `+0xfb45`/`+3`: the previous frame copy for `+0xa8` (DTT), then
   world `+0x9c`'s `Opaque`/`Transparent`/`postTransparent` passes
   (`0x006589e0(0)`; nothing drawn here).
10. `PostProcessManager_renderFocusFrame` for maps 0x189-0x18e;
    `RenderDelegationHelper_beforeVFX`; `PostProcessManager_renderScreenFade`
    (fade alpha 0 here: no draw); `BlurEffectEx` for magic 0x14.
11. Copies of the colour to `+0x9c` (magic 0x110, 0x111) or `+0xac`.
12. `PhyreScene_DrawMostVFX` into the colour with the scene depth (draws
    142-176).
13. Step 8 here when `isGlowAfterVFX`; `RenderDelegationHelper_beforeEffects`;
    DTT copies to `+0xa8`; `PhyreScene_RenderDTTVFX` when `+0x3854`; the
    specular pass (`SpecularMapParamsManager`); the cross-fade capture to
    `+0x1232c` (`+0x123ba`); the screen fade; the DTT copy.
14. `BrokenScreenEffect_render`; `MotionBlur_renderMotionBlur`
    (`0x006c5410`, needs `MotionBlur +0x58`); the motion-blur copies;
    `BlurEffect` (`0x006c3970`); `BlurEffectEx_render` (`0x006c4cf0`);
    `Distortion_renderDistortion` unless the broken screen is on (draws
    177-178); the motion-blur afterimage (`+0x123bb`: weight
    `*(+0x123bc) / 128`, `MotionBlur_setMotionBlurWeight`); the
    cross-fade copy (`+0x123c4`).
15. `PhyreScene_RenderMF152Related`; `PostProcessManager_renderUSM`
    (`0x00697670`, needs `+0x123d2` and `+0x123d3` = UnSharpMask);
    `PhyreScene_renderPostAA` (`0x006535b0`, mode scene `+0x10dd8`: 0
    `renderSMAA` `0x00696af0`, 1 `renderFXAA` `0x00695740`, 2 off, as
    here); `PostProcessManager_renderBright` (draws 179-180).
16. Colour `+0x8c` and depth `+0x90`: the pre-UI delegations
    (`0x006ad600`), the screen fade when the fade colour is locked,
    `PhyreScene_RenderVFXAfterPostProcess` when `+0x388c` (via a copy to
    `+0xac`).
17. `PhyreScene_DrawNormalUI` into the colour with the scene depth (draws
    181-196), scissor the colour's size; with scene `+0x78`, the
    viewport and scissor `+0x10d40..` first.
18. Screen fades after the UI (`+0x3860`, `g_PauseOn`, `0x025d5ebc`);
    `PostProcessManager_renderDebug4By3Frame` when `0x01136fc0` is 1.
19. `copyBuffer(colour +0x8c, back buffer, 4:3 flag 1, depth clear)`
    (draw 197); `setNon16By9Viewport`; the tweak utilities (debug, nothing
    drawn).

## Targets

The post-process manager is `*g_PostProcessManager` (`0x00ccc8a0`), the
scene's `+0x10cc`. Its first word is the scene context, which
`0x006656e0` sets to `g_PhyreApplication` (`0x00cc9cd8`) `+0x50`, the
same object as `scene +0xcc` (so the clear colour `ctx +0x30` is
`g_PhyreApplication +0x80`, `render.md`).

| Object | Capture (fw1-a) | Size | Role |
|---|---|---|---|
| back buffer | `0x3a57c940`, target0 (undeclared) | 1280x720 | composite target; its depth `0x3a57c740` |
| pp `+0x8c` | `0x3a57c640`, target1 | 1280x720 | the colour: scene, glow added, effects, distortion, brightness, UI |
| pp `+0x90` | `0x4156fb40`, target2 | 1280x720 D24S8 | the scene depth (effects and UI test against it) |
| pp `+0x94` | `0x4156fc40`, target3 | 1280x720 | view-space normal (HDAO input) |
| pp `+0x9c` | `0x4156fd40`, target7 | 1280x720 | HDAO / `postOpaque` target; cleared twice a frame |
| pp `+0xa4` | `0x2b7e780`, target8 | 1280x720 | glow map, then the brightness pass's output |
| `PLegacyGlowGPU +0x78[0..2]` (pp `+0x1178`) | target9, 11, 13 | 640x360, 320x180, 160x90 | glow levels, first target |
| `PLegacyGlowGPU +0x84[0..2]` (pp `+0x1184`) | target10, 12, 14 | as above | glow levels, second target |
| `Distortion +0x28` | `0x4cb58700`, target15 | 1280x720 | the distortion buffer (kept across frames) |

All colour targets are `R8G8B8A8_UNORM` (28). The glow object is
`PLegacyGlowGPU` at pp `+0x1100`, set up by
`Phyre_PPostProcessing_PLegacyGlowGPUBase_initialize` (`0x00a17bd0`) with
twice the pp's `+0x12400`/`+0x12404` and
`PLegacyGlowBase_initializeShaders` (`0x00a1ab00`) in
`PostProcessManager_onAllShaderBinFilesLoaded`.

Every post draw is a full-target mesh instance through
`renderMeshInstance` with a pass name set at `renderer +0x48`: 3 vertices
(a triangle, `VERTEX0`/`POSITION0` float3 + `TEXCOORD0` float2 from two
static buffers) except the brightness quad (6) and the distortion grid
(384 indices, `WPOS0`, `COLOR0`, `TEXCOORD0` from the dynamic rings).
Depth off, no depth target, sampler clamp (address 3); topology 4; cull 2 (front) in the glow passes, 1 (none) in the others.

## Passes

### Glow map (draw 129)

`PostProcessManager_prepareGlowMap(renderer, +0xa4, colour)` (`0x00694c60`),
when `+0x123b8` (shader loaded) and `+0x123b3` (glow switch, set to 1 at
scene init) are set. Mesh pp `+0x1210`, pass `0x00ccc94c`.

| | |
|---|---|
| input | colour `+0x8c` (`ColorBuffer`), point sampler (filter 0) |
| target | `+0xa4`, blend off |
| shaders | `phyreglow.fx`: vs `d881089a67e8f7d4`, ps `28b07f56c471aa15` |
| `UvScaleBias` (vs) | (1, 1, 0, 0), literal |
| `FullGlowThreshold` (ps) | scene context `+0x80` (`g_PhyreApplication +0xd0`) |
| `NoGlowThreshold` (ps) | scene context `+0x84` (`g_PhyreApplication +0xd4`) |

### Glow (draws 130-141)

`Phyre_PPostProcessing_PLegacyGlowGPUBase_render(renderer, colour, source,
0, 0)` (`0x00a1de20`) on pp `+0x1100`, called by `PostProcessManager_render`
when `+0x123b8` and `+0x123b3` are set and not (`+0x123d1` and
`+0x1260`): HDAO with its own map settings switches it off. The source is
the glow map `+0xa4` (`+0x123b8`). First `PostProcessManager_render`
copies scene context `+0x88` and `+0x8c` to the glow's `+0x54` (gain)
and `+0x58` (spread) and sets the clear colour to 0. Levels:
`min(+0x50, 3)`, 3 from the constructor (`0x009fc540`).

Down, level i = 0, 1, 2 (draws 130-135):

| Pass | Input | Target | Shaders | Constants |
|---|---|---|---|---|
| h (`0x01944e4c`) | level 0: the glow map; else `+0x84[i-1]` | `+0x78[i]` | vs `3ff5dac09b1f15f5`, ps `b155e9ffa60527b4` | `GaussianOutputScale` = (s, s, s, 1), s = (i + 1) x spread + 1 (1.65, 2.3, 2.95); `GaussianBlurBufferSize` = (w, h, 0, 0) of `+0x78[i]` |
| v (`0x01944e5c`) | `+0x78[i]` | `+0x84[i]` | vs `c9440a9e02d9dc22`, ps `b155e9ffa60527b4` | none written: the h pass's values stay |

Up, level i = 2, 1, 0 (draws 136-141):

| Pass | Input | Target | Shaders | Constants |
|---|---|---|---|---|
| h (`0x01944e4c`) | `+0x84[i]` | `+0x78[i]` | as down h | `GaussianBlurBufferSize` of `+0x78[i]`; `GaussianOutputScale` written only at level 0, (1, 1, 1, 1): at levels 2 and 1 it keeps the previous pass's value (2.95 from the down pass, then level 2's up value) |
| v + add (`0x01944e9c`) | `+0x78[i]` | level 0: the colour; else `+0x84[i]` | vs `c9440a9e02d9dc22`, ps `1685ec8c3a02c12b`; blend ONE + ONE | `GlowGain` = gain; `GaussianOutputScale` = (u, u, u, 1), u = 1 at level 0, else `+0x5c` (1.0) x 1 / (2 spread + 1) below level 3, 3 above |

All blurs sample with filter `0x14` (min/mag linear, mip point). The x87
arithmetic: s and u are computed at 53 bits and stored as `f32`
(1 / 2.3 = `0x3ede9bd4`, 0.43478262); `ffx_postfx::census` reproduces
every word bit for bit.

Only level 0's up pass reaches the screen: levels 2 and 1 add into their
own `+0x84[i]`, which nothing reads afterwards. Round 2 must still make
those draws (the draw stream is compared).

### Distortion (draws 177-178)

`Distortion_renderDistortion` (`0x006c2dc0`) on `Distortion_GetInstance`,
when `+0x7c` is set and `+0x7d` clear (else it sets `+0x7d` = 0x100 and
returns), unless the broken-screen effect is on; `+0x7f` clear makes it
`Distortion_createRenderTarget` first.

| | Copy (177) | Grid (178) |
|---|---|---|
| by | `PostProcessManager_copyDepthBuffer(renderer, colour, +0x28, 0x00ccc8ec, &ColorScale, 1, 1, 0, 0)` (`0x00692e90`); with `+0x7e` set instead a plain `copyBuffer` and `+0x7e` cleared | the grid mesh `+0x38`, pass `0x00cdeb90` |
| input | colour | `+0x28` |
| target | `+0x28` (the distortion buffer) | the colour (`+8`), cleared to 0 first (item 183), viewport its size |
| blend | `SRC_ALPHA`, `INV_SRC_ALPHA`: blends over the buffer's previous contents | off |
| shaders | `copybuffer.fx` with `ColorScale`: vs `b7829326eaf816e0`, ps `e928905e31636cf1` | `phyredistortionshader.fx`: vs `7c92ebb614734d54`, ps `de2b6b9db45acd86` |
| raster | cull 1 (none) | cull 1 |
| constants | `UvScaleBias` (1, 1, 0, 0); `ColorScale` = (1, 1, 1, min(`+0x78`, 0.8)) (0.8 here) | `UvScaleBias` (1, 1, 0, 0) |

The grid's vertices (`Distortion_setupDistortionGrid`,
`Distortion_updateDistortionVertex`) are the effects census's
(`pc-effects`): the census checks them only as ring contents.

### Brightness (draws 179-180)

`PostProcessManager_renderBright(renderer, colour)` (`0x00695440`), when
`+0x123d7` and `+0x123d6` are set (the shader loaded,
`onAllShaderBinFilesLoaded`); it always runs then, at Brightness 50 too.

| | Bright (179) | Copy back (180) |
|---|---|---|
| by | mesh `+0x12320` (6 vertices), pass `0x00ccca3c` | `copyBuffer(+0xa4, colour, 1, 0, 0)` (`0x00692c40`), mesh `+0x1208`, pass `0x00ccc8bc` |
| input | colour (`g_BrightTex`) | `+0xa4` (`ColorBuffer`) |
| target | `+0xa4`, viewport its size | the colour, cleared to (0, 0, 0, 0) (item 186, `beginScene(1)` with renderBright's clear colour) |
| shaders | `brightnessshader.fx`: vs `158d4ed17f37e1bb`, ps `be5d30cde4fe432a` | `copybuffer.fx`: vs `b7829326eaf816e0`, ps `00c1982e952d4506` |
| constants | `g_brightPara` = (pp `+0x12328` - 50) / 100, `+0x12328` = Brightness as a float (`g_Settings +0x24`, `PhyreScene_ApplySettings` `0x0066f010`): 0 here | `UvScaleBias` (1, 1, 0, 0) (2/3 with the 4:3 flag and `+0x123d9`) |

### Composite (draw 197)

`copyBuffer(renderer, colour +0x8c, 0, 1, 0, 1)`: target 0 is the back
buffer, viewport the window (`g_PhyreApplication +0x26c`, `+0x270`); the
last argument makes it `beginScene(2)`, clearing the back buffer's depth
(item 205). Same shaders as the brightness copy; `UvScaleBias` (1, 1, 0, 0)
or (2/3, 2/3, 0, 0) with `+0x123d9` (the 4:3 flag is 1). Then
`PostProcessManager_setNon16By9Viewport(renderer, 0)`.

## Where the constants come from

Of the 36 used constants of the 18 post draws (the census checks each):

| Source | Constants | Values here |
|---|---|---|
| literal in the function | `UvScaleBias` (6 draws), `GaussianOutputScale` (1, 1, 1, 1) at up level 0 h | (1, 1, 0, 0) |
| map data: the `GlobalCamera` | `FullGlowThreshold`, `NoGlowThreshold`, `GlowGain`, and the spread in `GaussianOutputScale` | 1, 0.35, 0.318; spread 0.65 |
| derived | `GaussianBlurBufferSize` (the level's target size), `GaussianOutputScale` | 640x360 ... 160x90; 1.65, 2.3, 2.95, 0.43478262, 1 |
| left from the previous pass | `GaussianOutputScale` of the down v and up h passes | |
| setting | `g_brightPara` (Brightness) | 0 |
| game state | `ColorScale` alpha (`Distortion +0x78`, capped at 0.8) | 0.8 |

The `GlobalCamera`: `PhyreScene_fieldMapInitialize` (`0x0065b8a0`) zeroes
scene `+0x4f8`, then `PhyreScene_findGlobalCamera` (`0x006519f0`) stores
the map cluster's (`+0x10d4`) `PCamera` named `GlobalCamera` there, and
`PhyreScene_setupPostProcessParams` (`0x0066f140`) copies into the scene
context:

| Camera (object offset) | Member | Context | `g_PhyreApplication` | Use |
|---|---|---|---|---|
| `+0xf4..+0xfc` | `m_bgColor` | `+0x30` (`0x0065b8a0`) | `+0x80` | clear colour (0.235, 0.235, 0.275) |
| `+0x104` | `m_glowUpperThreshold` | `+0x80` | `+0xd0` | `FullGlowThreshold` |
| `+0x108` | `m_glowLowerThreshold` | `+0x84`, at least 0.35 | `+0xd4` | `NoGlowThreshold` |
| `+0x10c` | `m_glowGain` | `+0x88` | `+0xd8` | `GlowGain` (glow `+0x54`) |
| `+0x110` | `m_glowSpread` | `+0x8c` | `+0xdc` | glow `+0x58` |
| `+0x114..+0x11c` | `m_dofFocusPlaneDistance`, ... | `+0x90..+0x98` | `+0xe0..+0xe8` | depth of field (`0x00a1ca10`) |

Maps 0x199, and 0x12ea and 0x1733 while `getScenerioFlag` is 0 (scene
`+0xfaf4`), get 1, 1, 0, 0 instead; a map without a `GlobalCamera` gets
1, 1, 0, 0 and 10000 x 3 with a warning. The cluster's member table puts
`PCamera`'s members 4 bytes below these offsets (it leaves out the vtable
pointer); the object bytes are as FFX.exe reads them. In `cdsp00.dae.phyre`
it is `PCameraPerspective` #0, the one camera of the map whose `m_bgColor`
is the frame's clear colour (glow 1, 0.35, 0.318, 0.65; DOF 90, 2559,
1.6). The census picks it that way and checks the glow constants against
it.

## Passes not on this frame

| Pass | Function | Runs when | Constants (source) |
|---|---|---|---|
| HDAO | `PostProcessManager_renderHDAOPass` (`0x00695ba0`) | `+0x123d0` (shader) and `+0x123d1`, set by `setHDAOSetting` (`0x006985d0`) for maps listed in `map/hdao_settings.bin` or `btlmap/hdao_settings.bin`, or forced by HDAO level `+0x1264` (`g_Settings +0x48`) above 0; level 0 clears `+0x123d1` | `g_numRing` 6/12/20 for levels 1/2/3; `g_fadeoutScale`, `g_kernelScale*`, `g_rejectRadius*`, `g_acceptRadius*`, `g_intensity*`, `g_blurDistance*`, `g_acceptAngle*` (Chr/Sce, the map's 13 floats at `+0x1224..+0x1254`); `g_widthSize`/`g_heightSize` (colour size); `g_ssao_projInfo` (main camera projection), `g_ssao_axis`, `g_ssao_mainTex_TexelSize`, `g_ssao_edgeSharpness`; textures colour, normal, depth (resolved first when MSAA, `+0x122b4`). Passes: AO into `+0x9c` (`0x00ccc98c`), blur into `+0xa4` (`0x00ccc99c`), back into `+0x9c` |
| colour correction | `PostProcessManager_render` | `+0x123b5` (shader), `+0x123b0` (switch; set to 0 at scene init `0x006656e0`) and a map list `+0x1200` (`PhyreScene_setupColorCorrection`, `PostProcessManager_SetFirstEntry`) | `ColorBuffer`, `ColorCorrectionBuffer`, `UvScaleBias`; into `+0x9c` (`0x00ccc8ac`), then `copyBuffer` back |
| depth of field | `0x00a1ca10` | `+0x123b7` and `+0x123b2` | scene context `+0x90..+0x98` (`GlobalCamera`) |
| screen fade | `PostProcessManager_renderScreenFade` (`0x00697490`) | fade alpha `+0x12398` not 0 | `ScreenFadeColor` = `+0x1238c..+0x12398` (`graphicSetScreenFadeColor`, `ScreenFadeDelegations`); pass by alpha mode `+0x1239c`: 2 `0x00ccc96c`, 0x10 `0x00ccc97c`, else `0x00ccc95c`; mesh `+0x12384` |
| motion blur, afterimage, cross-fade | `MotionBlur_renderMotionBlur` (`0x006c5410`), `0x006c5570`, `PhyreScene_render` | `MotionBlur +0x58`; `+0x123bb` with weight `*(+0x123bc) / 128`; `+0x123ba`/`+0x123c4` | copies of the colour into `+0xac`/`+0xa8`/`+0x1232c`, pass `0x00cdec0c` |
| blur | `BlurEffect` (`0x006c3970`), `BlurEffectEx_render` (`0x006c4cf0`) | activated by effects (`graphicActivateRadialBlur`) | the effects census |
| unsharp mask | `PostProcessManager_renderUSM` (`0x00697670`) | `+0x123d2` and `+0x123d3` (= UnSharpMask) | |
| FXAA, SMAA | `PostProcessManager_renderFXAA` (`0x00695740`), `renderSMAA` (`0x00696af0`) | PostAA (scene `+0x10dd8` 1, 0); FXAA not on maps 0x17 and 0x15c | SMAA quality `+0x1231c` 1-4 |
| focus frame | `PostProcessManager_renderFocusFrame` | maps 0x189-0x18e | `FocusDepth` |

## Settings

`GameSetting.ini` keys are `g_Settings` (`0x022fb504`) words in the key
table's order (`0x00c8a248`: Language +0, Resolution +4, ScreenMode +8,
Quality +0xc, ... Brightness +0x24, Shadow +0x30, TextureQuality +0x34,
MSAA +0x38, ColorCorrection +0x3c, PostAA +0x40, UnSharpMask +0x44, HDAO
+0x48). `PhyreScene_ApplySettings` (`0x0066f010`) passes them on:

| Key (sandbox value) | `g_Settings` | Goes to | Switches |
|---|---|---|---|
| `Quality` (`VQ_LOW`) | `+0xc` | no reader of `g_Settings +0xc` in the dump besides the parser: a preset of the options screen (open question 7) | none directly |
| `Brightness` (50) | `+0x24` | pp `+0x12328` | `g_brightPara`; the brightness pass runs at every value |
| `MSAA` (0) | `+0x38` | `PostProcessManager_setMSAA` (`0x00697e10`, which takes sample counts 0, 2, 4, 8, 16): the count on the colour, depth and normal surfaces, `+0x122b4` | the scene's `Opaque` instead of `OpaqueNoMSAA` (`scene.md`); HDAO resolves the depth first; the options screen warns about MSAA with HDAO |
| `ColorCorrection` (On) | `+0x3c` | the options screen only; the pass needs `+0x123b0` | (open question 2) |
| `PostAA` (`PAA_OFF`) | `+0x40` | `PAA_FXAA` (1): scene `+0x10dd8` = 1; `PAA_SMAA1-4` (2-5): `+0x10dd8` = 0, pp `+0x1231c` = 1-4; else 2 | FXAA / SMAA / none before the brightness pass |
| `UnSharpMask` (Off) | `+0x44` | pp `+0x123d3` | `renderUSM` |
| `HDAO` (Off) | `+0x48` | pp `+0x1264` (Off 0, Low 1, Medium 2, High 3) | the HDAO pass and its ring count; HDAO active (`+0x123d1`) with the byte `+0x1260` set turns the glow off |

`PhyreScene_render` also sets pp `+0x123b1` to 1 when the window is at
most 720 rows high (`g_PhyreApplication +0x270 < 0x2d1`) and no movie
plays, 0 during a movie.

## Run A against run B

`ffx-drawdiff` on fw1-a and fw1-b, every item: 257 differences, all in
scene items and all of them the noise `docs/drawdiff.md` documents
(`scene.GlobalTime` in every scene draw, the cloth ring `vb5` of five
Tidus draws, the `SgSpec_LightVec` junk read). Outside the scene: 0. The
dumped targets agree: the glow map and every glow level are byte-equal;
the colour differs after the scene in 4,972 bytes (the cloth), and that
difference carries through the glow (3,970), the distortion and the
brightness (3,767) to the back buffer (3,755 bytes, at most 71 levels). The gate test checks that no
difference falls outside the scene.

## For round 2

- State across frames: the distortion buffer `+0x28` is blended into
  (alpha 0.8), never cleared, so its contents date from earlier frames;
  `+0xa4` and the glow targets are fully overwritten each frame.
- The glow's parameter buffer keeps values between passes; the builder
  must write the same words in the same passes (the v passes and up h
  passes 2 and 1 write none).
- Levels 2 and 1 of the glow's up chain are drawn and never read.
- `+0x9c` is cleared twice even with HDAO off.
- Effects (`pc-effects`) sit between the glow and the distortion, the UI
  (`pc-ui2d`) between the brightness pass and the composite: the
  post-process builder hands the colour to them and takes it back.

## Open questions

1. `PostProcessManager_renderHDAO` (`0x0066b560`) is not the HDAO; what
   it renders top-down into scene `+0xfc70` (the minimap's map image?)
   and when `+0xfc00`/`+0xfc01` are set. A better name is open.
2. Where the `ColorCorrection` setting reaches the colour-correction
   switch `+0x123b0` (0 after scene init; `setAllSwitches` 0x006989c0 is
   called by `PhyreScene_createRWTTTarget`), and which maps have a
   `_ColorCorrection` list.
3. Captures with HDAO, MSAA, PostAA, UnSharpMask and a fade on, to check
   the passes listed from the code above against the draw stream (the
   harness uses `tools/sandbox/GameSetting.ini`; a capture with other
   settings needs a way to pass another file).
4. The depth-of-field function `0x00a1ca10` (a `PhyreEngine`
   post-processing class) and the glow constructor `0x009fc540`: names.
5. `PostProcessManager_copyDepthBuffer` (`0x00692e90`, a cg-match lead)
   copies with a `ColorScale` for the distortion: the name may be wrong.
6. The distortion's `+0x78` alpha, `+0x7c..+0x7f` flags and its grid's
   writers on this map (what turns it on in `cdsp0000`).
7. Whether anything reads `Quality` (`g_Settings +0xc`) through the key
   table rather than by offset.
8. The glow-off byte `+0x1260` next to the HDAO settings: who sets it.
