# The UI and 2D draws: what the menus, the text and Iggy draw, and from what

This is the M4 census of the UI (`docs/rewrite-plan.md`, M4: scene,
effects and post-processing, **UI**, movies, audio, input and window), in
the form `docs/formats/scene.md` gave the scene: every UI draw of a set
of captured frames traced to its producer, texture atlas, vertex source,
states and constants. It is the spec for the Rust UI layer. Crate:
[`crates/ffx-uidraw`](../../crates/ffx-uidraw). The PS2 side that decides
*what* is drawn (windows, menus, text layout) is in `docs/formats/menu.md`,
`text.md` and `pc-ui.md`; this page is about how it reaches Direct3D 11.

There are two UI paths, and nothing else draws 2D:

| Path | Producers | Drawn by | Shaders |
|---|---|---|---|
| Tk (the PS2 menu, window and text code) | `graphicDrawUIElement` (`0x0063eee0`), `graphicFontDraw` (`0x00640660`) | `PhyreScene_DrawNormalUI` (`0x00650cb0`), PhyreEngine mesh instances | `MenuDefaultShader*`, `OptimizedFontShader` (archive clusters) |
| Iggy (the PC's Flash screens: the Esc menu, its settings pages, the save and load lists) | Iggy movies (`IggyMenu`, `docs/formats/pc-ui.md`) | `IggyMenuManager_Render_00a267f0`, RAD's `GDraw` Direct3D 11 back end compiled into `FFX.exe` | DXBC blobs in `FFX.exe`'s `.data` |

## Captures

Five frames on the original code (`original`, i.e. `--hook-disable
replacements`), full captures (contents, not hashes), plus `fw1-a/b`
(`docs/formats/scene.md`, whose 15 UI draws are the minimap):

| Capture | Session | What | Draws (UI) |
|---|---|---|---|
| `ui-dialogue-a/title` | `tools/replays/draw-ui-dialogue.txt` | the title menu (map 0x17, count 101) | 73 (54) |
| `ui-dialogue-a/choice` | the same | New Game's "Select a Sphere Grid to use!": an event message window with a choice, map 0x17 | 57 (37) |
| `ui-menu-a/menu-main` | `tools/replays/draw-ui-menu.txt` | the main menu (save 1) | 448 (447) |
| `ui-menu-a/menu-items` | the same | down, down, circle (menu-browse-2's keys; on PC2's save the cursor starts on Sphere Grid, so this is Abilities' character choice) | 443 (442) |
| `ui-menu-a/menu-esc` | the same | Esc from the field: the PC pause menu (Iggy) | 151 (150) |
| `fw1-a` | `field_capture` | field-walk-1, cdsp0000 | 197 (15) |

The captures live in `~/ffx-work/m4/captures/ui-{menu,dialogue}-{a,b}`
(never in git). Both sessions boot from launch: the logo runs into the
opening (map 0x15d), Start skips to the title a few seconds in, so they
need no save for the title and the choice. The menu session then runs
`loadsave 1`, so its frames depend on the machine's save slot 1 (PC2:
Macalania Woods `mcfr0400`, late game; the main machine: the salvage ship
deck). The sandbox runs as `docs/harness.md` describes: no user mods,
`tools/sandbox/GameSetting.ini`, 1280 x 720, English.

```bash
# the captures (background, through the game slot; sound off)
FFX_GFX=1 FFX_TEST_VOLUME=0 target/release/examples/replay tools/replays/draw-ui-menu.txt '<win dir>\draw-ui-menu.trace' original
FFX_GFX=1 FFX_TEST_VOLUME=0 target/release/examples/replay tools/replays/draw-ui-dialogue.txt '<win dir>\draw-ui-dialogue.trace' original
# the census: one line per UI draw, the families of every draw, the constants
FFX_GAME_DIR=~/steam-games/FFX-X2-HD cargo run --release -p ffx-uidraw --example census -- <capture>... [--constants]
# every draw with its shaders' origin and texture files
FFX_GAME_DIR=~/steam-games/FFX-X2-HD cargo run --release -p ffx-uidraw --example draws -- <capture> [archive filter...]
# the gate (skips without FFX_GAME_DIR and FFX_UIDRAW_CAPTURES; `;` between captures)
FFX_GAME_DIR=~/steam-games/FFX-X2-HD FFX_UIDRAW_CAPTURES="$C/ui-menu-a/menu-main;$C/ui-menu-a/menu-items;$C/ui-menu-a/menu-esc;$C/ui-dialogue-a/title;$C/ui-dialogue-a/choice;$C/fw1-a" \
  cargo test --release -p ffx-uidraw
```

## How a draw is classified

The census (`ffx_uidraw::census`) names every draw's shaders by their
bytecode (FNV-1a 64, `ffx_dxbc::hash`): the effect name of each archive
shader cluster that holds the blob (`shaders/d3d11/<effect>.fx#<md5>.phyre`;
a vertex shader can sit in several clusters, and the pixel shader's
cluster is preferred), or `FFX.exe@<address>` for a blob in the image. The
pixel shader decides the family:

| Family | Pixel shader | Producer |
|---|---|---|
| `scene` | any, into the two scene targets and a depth target | `docs/formats/scene.md` |
| `menu-nt` | `menudefaultshadernt.fx` | `graphicDrawUIElement`, texture name `"NoTexture"` |
| `menu-tex` | `menudefaultshader.fx` | `graphicDrawUIElement` with a texture |
| `menu-fb` | `menudefaultshaderna.fx` | `graphicDrawUIElement`, texture `"RealFrameBuffer"` (not in these frames) |
| `menu-other` | `menudefaultshadernb`, `…ntnb`, `…ntztest` (not in these frames) | |
| `font` | `optimizedfontshader.fx` | `graphicFontDraw` |
| `abmap` | `abmapshader*` | the sphere grid (`graphicDrawUIAbmapElement`, not in these frames) |
| `iggy` | `FFX.exe@…`, or none with a `FFX.exe@…` vertex shader (stencil masks) | GDraw |
| `effect`, `other` | `vfx*` and `phyredistortionshader`; any other archive shader | the effects and post-processing censuses |
| `unknown` | anything else | (none in the frames) |

Textures are matched by their top level's texels against every level of
the archive's texture clusters under `ps3data/menu/`, `ps3data/menu_us/`
and `abmap` (`ffx_uidraw::UI_TEXTURE_FOLDERS`); render targets are named by
role (n-th target of the frame); anything else is "runtime" (made at run
time, not from a cluster).

## Where the UI sits in the frame

`PhyreScene_render` (`0x00668930`), after the scene, the effects and the
post-processing chain (`docs/formats/postfx.md`), all into the post
manager's main colour surface (`*(scene +0x10cc) +0x8c`, the frame's target
role 1 on every capture):

1. `PhyreScene_restoreDefaultRenderTargets` (`0x0066c7f0`: colour
   `+0x8c`, depth `+0x90`) and `PhyreScene_DrawUI2DVFX` (`0x00650ea0`):
   the 2D VFX list (scene `+0x3860` count, `+0x3868` meshes, filled by
   `VfxGeometry_Register_006a52f0`), Transparent pass, UI camera.
2. When the scene's UI flag `+0x78` is set, the movie-with-UI viewport
   (`+0x10d40..+0x10d4c`) and `0x00650340`.
3. `PhyreScene_DrawNormalUI`: the Tk records (below), only when `+0x10dcc`
   or `g_MovieFading` (`0x01340824`, "a menu is open") is set.
4. Screen fades (`PostProcessManager_renderScreenFade`), then
   `PostProcessManager_copyBuffer` of the main surface into the back
   buffer: the last `copybuffer.fx` draw of every capture (target role 0).
5. `IggyMenuManager_Render_00a267f0`: the Iggy movies, straight into the
   back buffer.

With a menu open (`g_MovieFading`) the 3D scene is not drawn at all:
`menu-main` and `menu-items` start with two clears and go straight to the
menu's own background (`menu/d3d11/ffx_bg.dds`). The Esc menu pauses the
game: its frame copies the last image into the back buffer
(`copybuffer.fx`) and draws Iggy over it.

All Direct3D calls run on PhyreEngine's render thread
(`docs/renderer.md`, "Threads"), Iggy's too: `IggyMenu_Draw_00a28eb0`
queues a render-thread command (`IggyPlayer_QueueDraw_00a3d9b0`) instead of
drawing on the main thread.

## The Tk path

### Producers

The PS2 menu code builds GS packets on the PS2; the PC build replaced the
packet writers' leaves with two functions that append quads to "menu
records" (`docs/formats/menu.md`, "Draw cursor and packets", for the
callers' side):

- `graphicDrawUIElement(elem, texture, ps2_space, shape, type)`: one quad.
  Callers: the plates, boxes and shapes of the menu and window code
  (`TODrawMenuPlate…`, `TODrawBoxXYWHC2`, `TODrawCrossBoxXYWHC2`,
  `TOMkpShapeXYWHUV`, `TOMkpShapeXYWHUVC2`, `TOMkpRotateShapeXYWHUVC2`,
  `TOMakePktBoxXYWHRGBA`, `TkMenuDrawRect`, `TkMn2DrawRect`,
  `TkMn2DrawCrossRect`, `TkMn2DrawCrossCharFace`, `DrawCrossMenuIconXYWHRGBA`,
  `TOAddPktMenuIconRGBA`, `DrawBtlMenuWithTex`/`WithoutTex`,
  `DrawWaterWaveShapeC2`, `TODrawAlphaBG`, `DrawBlackBackGround`,
  `graphicDrawDream`, `PhyreScene_drawMainMenuWaveEffect`, the pause
  background `PhyFMVPlayerManager_renderPauseBG`, `rcPrimDraw2D`,
  `AbmapDrawBar`; 36 callers in `calls.tsv`). PC button icons go through
  `RemapButtonIcon_remapButtonIcon` first and may be replaced. The pad
  glyphs come from `menu/d3d11/pad_icon` (or `keyboard_icon`), a 12 x 8
  grid of 34 x 34 cells (`RemapButtonIcon_remapIconUV`: 1/12 by 1/8);
  row 0 is X A B Y LB RB LT RT Back Start LS RS (ffx-demaster
  `docs/ui.md`, which swaps PlayStation glyphs into those cells).
- `graphicFontDraw(glyph, texture, …, page)`: one glyph quad. Callers: the
  font packet writers `TOMakePktFont1612Font`/`Shadow`/`Edge`,
  `TOMakePktFontLItalicFont`/`Edge`, `TOMakePktBtlEasyFont`/`Shadow`,
  `TOMakeBtlEasyFontEdge`, `TOAdpFontM_ASCII`, `TOAdpCrossFontM_FFX`,
  `TOAdpCrossFontS_ASCII`, `TODrawFont3d80Glyph_008eeee0`.

Both do nothing while `g_GraphicSkipFrame` is set, and hand the sphere
grid (`g_GraphicAbmap` with state below 2) to `graphicDrawUIAbmapElement`
instead.

The texture argument is a name (a `char *`), the archive path the PS2 code
picks from tables such as `0x00c33500..` (per language: `battle_kuang`,
`guide_circle`, `stonetexture`, `meswin`) and `0x00c59568..` (per
language: `base_ftc/font_0_0`, `font_0_1`, `shadow_0_0`, `shadow_0_1`);
`"NoTexture"` and `"RealFrameBuffer"` are special.

### Records and batching

`MenuManager_GetMenuVertexInfo(name, type)` (`0x0067ba50`) returns the record
the next quad goes into. The manager (`g_MenuManager`, the `this` of the
`MenuManager_*` functions) keeps two record pools per chain (chain `+0x5b7c`
0 or 1, swapped each frame), one-quad records (`MenuRecord_InitSingle_0067c2d0`,
4 vertices, 6 indices) and 12-quad records (`MenuRecord_InitMulti_0067c030`,
48 vertices, 72 indices), and a draw list per chain:

- types 1, 2 and 10 always take a new one-quad record
  (`MenuManager_TakeSingleRecord_0067c210`);
- a quad with the same texture name as the last one (`+0x5b88`) goes into
  the same record: the second quad moves the first into a 12-quad record
  (`0x0067dc70`, `MenuManager_TakeMultiRecord_0067c170`), up to 12 quads
  (`+0x5b78`); the 13th starts a new record;
- under draw type 1 (`graphicSetUiDrawType`, `+0x5b8c`) up to five textures
  are batched at once (`0x0067e310` looks the name up in `0x00ccc7d4`,
  counts at `0x00ccc7ec`).

So **one record is one draw**, and draws keep the PS2 code's order: the
captures show runs of 1 to 12 quads (6 to 72 indices), a new draw at each
texture change. A glyph's shadow and the glyph itself use different
textures (`shadow_0_n`, `font_0_n`), so text alternates shadow runs and
glyph runs.

The record (`+0x94` points to its render data header, `+0xe0` in the
record):

| Header offset | |
|---|---|
| `+0` | 1 textured, 0 `NoTexture` |
| `+4`, `+8` | vertex and index counts (4 and 6 per quad) |
| `+0xc`, `+0x10`, `+0x14`, `+0x18`, `+0x1c` | positions (float3), normals (float3), colours (float4), UVs (float2), indices (u16) |
| `+0x20` | the texture name |
| `+0x28` | the type (copied to record `+0xb4`): 0 normal, 1 "RenderBeforeVFX", 2 the minimap, 10 |
| `+0x2c + 0x1c n` | counts per dynamic-geometry buffer `n` (`DynGeoMemManager_getCurrentBufferId`) |

Record `+0xbc` is 1 for `graphicFontDraw`, 0 for `graphicDrawUIElement`.

### Vertices (`graphicDrawUIElement`)

Per quad four vertices and six indices `b, b+1, b+3, b+3, b+1, b+2` (`b`
the record's vertex count before the quad), positions in a 1920 x 1080
space centred on the screen (z = 1), normals (0, 0, 1), colours
`int / 255.0` per channel (the PS2 alpha 0x80 comes out as 0.502), UVs with
`v` flipped (`1 - v`). The element (`elem`, floats) is remapped first
(`graphicUiElementRemap`, mode 0, 1 or 4 by `shape`):

- `ps2_space` != 0: PS2 coordinates (512 x 416), `x * 3.75 - 960`,
  `y * 2.5961537 - 540`;
- else `shape == 1`: as given; `shape == 3`: `x - 960`, `y - 540`;
  otherwise the PS2-size quad is scaled by 3.75 / 2.5961537 about its
  centre;
- `shape % 5 == 1` takes four corners, `shape == 4` four free corners and
  colours, `shape == 2` swaps the colour pairs (a vertical gradient).

### Vertices (`graphicFontDraw`)

One quad per glyph from the glyph record: UV rectangle `+8`/`+0xc` and
`+0x28`/`+0x2c` (v flipped), corner colours from `+0x10..+0x1c` (top) and
`+0x30..+0x3c` (bottom) as `c / 255.0` with alpha `a * 2 / 255.0` (so the
PS2's 0x80 is 1.004 and 0xff 2.0: the captures' colour range reaches
2.008), multiplied per corner by the colours packed in the 64-bit word at
`+0x88` (shifted, `aullshr`) and `+0x90`; positions from `ftol` of the
glyph's corners less 960 / 540; `+0x80 == 0x7d1` (italic) moves the
lower two vertices 5 to the right.

### Vertex source as captured

Every one of the 995 Tk draws of the five frames (and `fw1-a`) is indexed
(topology 4, `R16_UINT`), reads dynamic rings (`D3D11_USAGE_DYNAMIC`; the
census checks `ring` on all of them) in four streams, one per array:

| Slot | Element | Stride |
|---|---|---|
| 0 | `POSITION0` float3 (6) | 12 |
| 1 | `NORMAL0` float3 (6) | 12 |
| 2 | `COLOR0` float4 (2) | 16 |
| 3 | `TEXCOORD0` float2 (16) | 8 |

The census decodes every vertex the draw reads and checks the index
pattern and the flat normals and z: all 995 pass (`traced`). The UVs of
`NoTexture` quads are whatever the element block held (they differ
between runs, below; the NT shader does not sample).

### Material and shader

`MenuManager_SwapMenuBufferChain` (`0x0067e7f0`, from
`MenuManager_UpdateMenu`) gives each listed record a geometry slot (from
`+0x2bd4`, 600 one-quad, or `+0x2bdc`, 800 twelve-quad; chain 1 `+0x582c`,
`+0x5834`) and `MenuManager_MenuInstanceUpdateMaterialShader`
(`0x0067d920`) binds its material when the texture name changed:

| Record | Shader cluster (scene field) | Hidden when |
|---|---|---|
| `+0xbc` = 1 (font) | `OptimizedFontShader.cgfx` (`+0xfa94`) | `+0xfaa0` |
| `"RealFrameBuffer"` | `MenuDefaultShaderNA.cgfx` (`+0xfa88`) | |
| `"NoTexture"` | `MenuDefaultShaderNT.cgfx` (`+0xfa80`) | `+0xfa9c` |
| any other name | `MenuDefaultShader.cgfx` (`+0xfa84`) | `+0xfa9d` |

The clusters load from `/FFX_Data/GameData/PS3Data/Shaders/GCM/<name>.cgfx.phyre`
(`fiosUnifyFilename` maps them to the D3D11 builds), each by its loader:
`PhyreScene_loadMenuNoTextureShader` (`0x00665010`), `PhyreScene_loadMenuShader`
(`0x006650f0`), `PhyreScene_loadNAMenuShader` (`0x00665200`),
`PhyreScene_loadOptimizedFontShader` (`0x00665580`); also
`MenuDefaultShaderNB` (`+0xfa8c`, `0x006652e0`), `…NTZTest` (`+0xfa78`,
`0x006653c0`) and `…NTNB` (`+0xfa90`, `0x006654a0`). The texture is the cluster of that name
(`0x006a3100`; `"FrameBuffer"` and `"RealFrameBuffer"` name the frame
buffer copies), the sampler `PhyreScene_getLinearSampler(header +0x24)`.
The menu font vertex shader is the same bytecode as `abmapshadercone.fx`'s
(the census names it by the pixel shader's cluster).

### Drawing (`PhyreScene_DrawNormalUI`)

Clears depth (`clearScene(2)`), sets the UI camera (scene `+0xa88`,
`Phyre_PRendering_PRendererBase_setCamera`), then for each record of the
current chain (`MenuManager_GetMenuCount`, `MenuManager_GetMenuRecord_0067b990`)
with a mesh instance (`+0xc`):

- type 0: `renderMeshInstance` in the Opaque pass, then the Transparent
  pass (pass name `*(scene +0xd0) +0x48`, as in `docs/formats/scene.md`);
  with the game paused (`g_PauseOn`) and no fade it stops after the first
  record;
- type 2: the scissor set to the minimap viewport
  (`PhyreScene_getMinimapViewportX/Y/Size`), Opaque and the pass at
  `0x00ccb9e8`, then the scissor back to the full surface.

Type 1 records are the "RenderBeforeVFX" ones (`MenuManager_GetMenuMeshInstace`,
used by `PhyreScene_RenderCloudUI` `0x00653060`).

### States (all Tk draws of the captures)

| | Menu (`menu-nt`, `menu-tex`) | Font |
|---|---|---|
| blend | `ONE`, `INV_SRC_ALPHA`, add; alpha `ONE`, `ZERO` | `SRC_ALPHA`, `INV_SRC_ALPHA`, add; alpha `ONE`, `ZERO` |
| depth | off, no write | off, no write |
| raster | cull none | cull none |
| scissor | off; on (minimap viewport `[97, 56, 337, 236]` on `fw1-a`) for the type-2 records | off |
| viewport | 0, 0, 1280, 720 | the same |
| target | role 1 (the post manager's main surface, 1280 x 720 `R8G8B8A8_UNORM`) | the same |

### Constants

The menu and font shaders read two variables of `$Globals` (vertex stage) and
nothing in the pixel stage; every Tk draw of every capture has the same
values:

| Variable | Value | Source |
|---|---|---|
| `World` | diag(-1, -1, 1, 1) | the record's matrix (identity, `MenuRecord_Init*`) through `PhyreScene_setMeshInstanceWorldMatrix` (`0x0066d810`), which negates x and y (`MenuDynamicGeometry_setMeshWorldMatrix`) |
| `WorldViewProjection` | rows (1/960, 0, -1.5728731e-10, 0), (0, -1/540, 0, 0), (2.3040482e-12, 0, 1.5259022e-5, 0), (0, 0, 0, 1) | derived: the UI camera (`PCameraProjection` at scene `+0xa84`, set up at `0x00650000+`: world = rotation by pi (`0x40490fda`) at `+0xb38`, near 1 `+0xb3c`, far = the main camera's far `+0xb40`, aspect 16/9 `+0xbd4`, half height 540 `+0xbd8`) times `World` |

The tiny terms are the f32 `sin(pi)` of the camera's rotation. A Rust UI
must reproduce the matrix bit for bit (the ortho map is 1920 x 1080 to
-1..1 with y down).

### Texture atlases seen

| File (`ps3data/…`) | What | Frames |
|---|---|---|
| `menu_us/base_ftc/d3d11/font_0_0`, `font_0_1` | the glyph pages | title, choice, menus |
| `menu_us/base_ftc/d3d11/shadow_0_0`, `shadow_0_1` | the glyph shadow pages | the same |
| `menu_us/d3d11/meswin` | window frame (8 quads: corners and edges), the hand cursor | title, choice, menus |
| `menu/d3d11/stonetexture` | the window body | choice |
| `menu/d3d11/menu_new`, `battle_kuang`, `texture`, `ffx_bg` | menu plates, frames, row highlights, background | menus |
| `menu/d3d11/guide_circle` | the save sphere marker on the minimap | `fw1-a` |
| `menu/titl/d3d11/water_25` | the title's water band | title |

Every texture a Tk draw samples is an archive file at its top level (no
level dropped on `TQ_LOW`, unlike the scene's).

### The frames, Tk part

- **choice** (37 UI draws): two windows. Each is a shadow plate (`menu-nt`,
  6 px down and right), the stone body (`stonetexture`), a gradient
  (`menu-nt`) and the frame (`meswin`, 8 quads); then each text line as
  shadow runs and glyph runs per page; the hand cursor (`meswin`, 1 quad)
  last. This is the event message window (`g_AtelMsgWindows` `0x01326d70`
  in use), drawn by the same producers as the menus.
- **title** (54): the water band (`water_25`), "New Game" and "Load" as
  shadow (10 quads a glyph: an edge around it, 22 px wide against the
  glyph's 16) and glyph draws alternating
  (each letter changes texture twice), the cursor.
- **menu-main** / **menu-items** (447 / 442): background, plates, row
  highlights and frames, 388 / 382 font draws, cursors.
- **fw1-a** (15): the minimap. Draw 188 (`menu-nt`, the minimap panel) is
  drawn before the depth clear and next to the 2D VFX draw 189, so it is
  likely from `PhyreScene_DrawUI2DVFX`'s list rather than a Tk record (open
  question 2); 191-204 are type-2 records under the minimap scissor (grid
  lines, the player and the save sphere markers).

## The Iggy path

`IggyMenuManager_Render_00a267f0` (when `+0x30` is set) binds the back
buffer without depth, then draws every enabled menu (`IggyMenu +0x70`)
layer by layer (`+0x98`, 0..2) and ends with
`IggyMenuManager_EndDraw_00a362e0`. Each draw is a render-thread command
(`IggyPlayer_QueueDraw_00a3d9b0` → `0x00a3d9f0`:
`IggyPlayerReadyToTick` under the critical section `0x01a858cc`, then the
player's draw), and Iggy (`iggy_w32.dll`) renders through `GDraw`
(`gdraw_d3d1x_shared`, linked into `FFX.exe` at `0x00a34cc0..`): its own
shaders, dynamic buffers, textures and render targets, none of them
PhyreEngine's.

**menu-esc** (the PC pause menu): 150 Iggy draws after one `copybuffer`.

| Vertex shader | Pixel shader | Draws | Used constants |
|---|---|---|---|
| `0x00c84a98` | `0x00c6e0d8` | 70 | `world0`, `world1`, `texgen_s`, `texgen_t`, `x3d`, `y3d`, `w3d`; `color_mul` |
| `0x00c84a98` | `0x00c6e580` | 2 | the same and `color_add` |
| `0x00c84f60` | `0x00c6d518` | 12 | `world0`, `world1`, `x_off`, `texgen_s/t`, `x3d`, `y3d`, `w3d`; `color_mul` |
| `0x00c84f60` | none | 6 | the same (stencil-only mask draws) |
| `0x00c855e8` | `0x00c6e0d8` | 7 | `world0`, `world1`, `x3d`, `y3d`, `w3d`; `color_mul` |
| `0x00c855e8` | `0x00c77120` | 7 | the same; `clamp0`, `clamp1`, `color`, `tc_off` (cb1) |
| `0x00c855e8` | `0x00c807d8` | 2 | `clampv`, `tap` (cb1: a 9-tap filter) |
| `0x00c855e8` | `0x00c80cd8` | 12 | |
| `0x00c855e8` | `0x00c84308` | 2 | |
| `0x00c855e8` | `0x00c847e0` | 30 | |

(addresses of the DXBC blobs in `FFX.exe`). `x3d` = 2/1280, `y3d` = -2/720,
`w3d` = (-1, 1, 1, 1): pixel coordinates of the 1280 x 720 back buffer.

- Targets: the back buffer (role 0, 26 draws, with a 1280 x 720 depth-stencil
  target cleared first) and three 1280 x 720 `R8G8B8A8` targets (roles 8,
  16, 17: 124 draws) for the Flash filters (blur passes with `tap`,
  composited back with `clamp*`).
- Textures: all made at run time: the glyph cache and the movie's
  bitmaps (`t0`, 27 x 89 to 2048 x 1024), a 2 x 1 texture in slot 7 on 91
  draws (GDraw's), and the filter targets.
- States: `ONE`/`INV_SRC_ALPHA` (premultiplied) with the depth test on
  and writing for shapes; the filter passes without blending; scissor on,
  the full screen.

The title menu is **not** drawn by Iggy: its frame has no GDraw draw, and
"New Game" and "Load" are `graphicFontDraw` glyphs. The title's Flash
movie only reads keys (`docs/harness.md`: Up, Enter).

## Noise (a against b)

A second run of both sessions (`ui-*-b`) against the first, with drawdiff
(`docs/drawdiff.md`):

| Frame | Differences | Cause |
|---|---|---|
| menu-esc | 0 | |
| menu-main | 6 `resource` | the UVs of the three row highlights (`texture.dds`, scrolling), the UVs of two `NoTexture` bars (not sampled; stale element words), the hand cursor's x (it bobs) |
| menu-items | 1 missing, 3 extra, 7 `resource` | the same, and the second cursor's blink changing the batching (a glyph run split differently) |
| title, choice | 4 `constant`, 5-6 `resource` | `scene.GlobalTime` of the title's two water draws (the opening is skipped at a real-time moment), a count-0 scene draw's ring bytes, the cursor's x, one `NoTexture` quad's UVs |

The cursor's bob and the highlight scroll follow a timer the virtual clock
does not pin; the NT UVs come from the element block the caller left
(`graphicDrawUIElement` writes them whatever the texture).

## Gate

`cargo test -p ffx-uidraw` (`tests/ui_frames.rs`):
`every_ui_draw_of_the_captured_frames_is_traced` asserts, for every frame
of `FFX_UIDRAW_CAPTURES`: no draw has a shader of unknown origin, every UI
draw is traced (`census::traced`: its family has a producer, every texture
slot its shader reads is bound and placed, and a Tk draw's quads have the
menu code's index pattern, normals and z), and every texture a Tk draw
samples is an archive file. Result on the twelve captures (`ui-*-a`,
`ui-*-b`, `fw1-a`, `fw1-b`): **2,292 UI draws, all traced, 0 unknown
shaders**. `families_by_shader_origin` and the two `census` unit tests
(synthetic quads) run without the game.

## Open questions

1. The `NoTexture` UVs: which element words `graphicDrawUIElement` reads
   for them (stale caller data); a Rust UI must write the same words or
   the draws differ in their ring bytes (not in pixels).
2. `fw1-a` draw 188 (`menu-nt` before the depth clear): a record of type
   10, or a VFX geometry on the 2D VFX list using `MenuDefaultShaderNT`?
   `VfxGeometry_Register_006a52f0` is the only writer of the list
   (`0x006a0d40`).
3. The UI camera's projection: the exact formula behind the
   `WorldViewProjection` bits (an orthographic `PCameraProjection` with
   half height 540, aspect 16/9, near 1, far from the main camera, view the
   inverse of a pi rotation); the far plane's effect on the z row.
4. The cursor bob and highlight scroll timers (the noise above): which
   clock (`graphicGetTime`, the menu's frame counter `0x01841c60`).
5. Iggy: the GDraw command stream is fully determined by the movie and
   the Iggy runtime (`iggy_w32.dll`, closed source). A Rust UI either
   keeps Iggy and GDraw (the D3D11 device is the game's own) or
   re-implements the four PC screens; the census gives the draws to match
   but not their generator.
6. Types 1 and 10, `MenuDefaultShaderNA`/`NB`/`NTNB`/`NTZTest` and the
   sphere grid (`abmap`) do not occur in these frames: a battle frame, the
   sphere grid and a movie with UI need their own captures.
