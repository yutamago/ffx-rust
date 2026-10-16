# The external renderer

The game keeps running as it is: its logic, menus (Iggy Flash), movies
and sound are untouched. What changes is who draws the 3D scene. The hook
streams every scene draw the game makes to a separate 64-bit process,
`ffx-render.exe`, which draws the scene with its own lighting and hands
the finished image back. The game then draws its effects, glow and menus
over it as usual.

## Pieces

| Crate | Role |
|---|---|
| `ffx-hook` (`src/gfx/`) | Direct3D 11 interception in the game: device and context hooks, the scene stream, the composite of the renderer's image |
| `ffx-gfx` | The stream's messages and the shared-memory rings |
| `ffx-dxbc` | DXBC reader, SM4 decoder and DXBC→WGSL translator (the game's 2,515 shaders) |
| `ffx-render` | The renderer: a mirror of the game's GPU objects, the replay of its draws, the enhanced path, the overlay |

## The scene stream

The runtime swaps the entries of the immediate context's vtable as it
goes, so patching that vtable loses draws. The game gets a proxy context
instead (`gfx/proxy.rs`): an object whose stubs forward each call to the
real context's current method. Device methods are patched per vtable.

Every object the scene uses (buffers, textures, views, shaders, input
layouts, states) is sent once, and again when it changes; each frame then
goes as a list of commands (draws with their bound state, clears, buffer
updates, copies). Constant buffers are read from the layer's cached copy:
mapped GPU memory is write-combined and slow to read. Dynamic vertex and
index rings are read per draw, only the range the draw uses.

The scene is the draws into the multiple render targets (colour,
view-space normal) with a depth buffer. Once the renderer answers frames
reliably the game skips those draws (`FFX_RENDERER_SKIP=0` keeps them);
after a missed answer it draws them again.

Before each frame goes a `World` message: the scene (the name of the last
event file the game opened, e.g. `cdsp0000` on the S.S. Winno deck) and
the camera's eye and look-at point from `g_Camera`. At Present, when the
renderer asks for them (answer flag 32), an `Effects` message carries the
glowing effects the game drew after the scene went out: draws into the
scene colour against its depth, depth-tested, blending with the source or
the destination whole (save spheres, lamps, light beams). The game still
draws those itself.

## The image handed back

The renderer runs on Direct3D 12 (`FFX_RENDER_BACKEND=vulkan` for
Vulkan). It draws the finished colour, normal and depth into three sets
of textures shared under the names `Local\ffx-gfx-<pid>-<generation>-<set>-<plane>`,
opened by the game with `ID3D11Device1::OpenSharedResourceByName`. A shared
fence, its handle duplicated into the game, tells the game's GPU when a
set is written (`ID3D11Device5::OpenSharedFence`,
`ID3D11DeviceContext4::Wait`), so neither side waits for the other's GPU.
`FFX_GFX_FENCE=0` makes the renderer wait for its GPU instead;
`FFX_GFX_SHARE=0` sends the pixels through the ring, which is several times
slower.

## The enhanced path

`ffx-render/src/enhanced/`. Draws are sorted by the shader family of their
pixel shader (the cluster file it comes from: `ffx-render-shaders.tsv`,
made from the archives on first run). Scenery, characters, scrolling and
ice surfaces are drawn by the renderer's own shaders, their inputs read
from the game's constant buffers by name (`World`, `BoneTransforms`,
`MaterialColour`, `scene.ViewProjection`, fog, ...). Everything else
(sky, effects, pre-rendered backdrops) is drawn with the game's own
shaders, translated to WGSL, in the game's order.

Passes, in order:

1. **Shadow cascades**: three, each a box around its slice of the view,
   snapped to whole texels. The game culls for its camera only: each
   cascade (and probe face) draws just the draws inside it, by their
   bounds from the vertices their indices use (static vertex and index
   data stay on the CPU; skinned draws are never culled;
   `FFX_RENDER_NO_CULL=1` turns it off).
2. **Ray tracing structures** (where the GPU has ray queries): the opaque,
   unskinned scenery as bottom-level structures (kept while their buffers
   are), all of them in a top-level structure each frame.
3. **G-buffer**: albedo (roughness in alpha), world normal and material,
   the game's baked vertex light and fog weight, and motion vectors.
   Material maps (normal, roughness, cavity) come from a pack or are made
   from the colour texture. Motion comes from each draw's World and bones
   last frame: draws are matched across frames by their shaders, buffers
   and range. Scenery meshes that do not quite meet leave pinholes along
   their edges, which the jitter of TAA and FSR moves about (a dotted
   dark line; the game's own samples happen to miss them): with "Close
   gaps between scenery meshes" the scenery's triangles are drawn again
   as lines, 0.2% further away, which fill only those (in the G-buffer
   and in the full-size depth). The back of a two-sided draw (hair
   cards, cloth) turns its normal toward the camera (against the
   triangle's own, from screen derivatives). Skin-coloured texels take the
   skin material except on cut-out textures (hair: by the texture's mean
   alpha, its smallest mip). Scenery wholly beyond 2500 units, or a sky
   dome over the camera wider than twice that, keeps the game's shading
   (the parts of a sky the enhanced passes cannot draw do too: no line
   where the two meet).
4. **Light probes**: up to two per frame at the characters; the scenery
   around each, drawn into a small cube, becomes nine spherical
   harmonics, eased over 0.3 s (a probe that stays in place), so that a
   flash in its small faces (sparkling water) does not flicker the light.
5. **Ambient occlusion** (screen space, blurred).
6. **Lighting** (linear HDR), by the scene's lights (its *rig*,
   `rig.rs`, below). Scenery keeps the original light (the game's
   painted light) as far as the rig has it, its shadows from the key
   light; dynamic lights light it as far as their "affects scenery" goes,
   with the light the surroundings bounce (from the probes, which see the
   art) where the original light is turned down. Beyond 1500–2500 units
   (the sky dome, distant land) scenery keeps the game's own colours,
   whatever the lights. Characters get their lights with the game's soft
   wrap, light from the probes, and skin scattering on skin-coloured
   parts. Shadows are ray-traced from the scenery where possible, from the
   cascades otherwise. Other lights (`lights.rs`, up to 64: point and spot
   lights falling with the square of the distance over a source two units
   across, ending smoothly at their radius, and unshadowed directional
   lights) come from the rig and from the glows, with ray-traced shadows
   where rays are on. Static lights keep their shadows (`statics.rs`): a
   static point or spot light (at most 8) has six depth faces of the
   scenery within its reach, made again only when the light or that
   scenery changes (two lights a frame at most), sampled in place of rays;
   a static key light keeps the scenery's cascades while they and the
   scenery stay the same (the second set, drawn every frame, has what
   moves). As in Unreal, what moves casts no shadow from a static point
   or spot light.
7. **Water**: the game's water draws, with its two normal maps, its cube
   map tinted as the game tints it, screen-space reflection, refraction
   with absorption, and the game's opacity.
8. **Volumetric light**: the key light through the cascades, half
   resolution.
9. **Camera effects** (each optional): depth of field (focus on what the
   game's camera looks at, its look-at point from `g_Camera` sent in the
   `World` message, which in the field follows the player at chest
   height, refined to the surface around it nearest its depth; else at
   the centre of the screen; eased over a few frames; a gather in two
   layers: what is at or behind a pixel blurs no farther than the pixel
   itself, what is nearer and more blurred spreads over it with the share
   of the pixel its circle covers, the reach found from per-tile maxima
   of the near blur), motion blur (along the
   motion vectors; the camera's motion where the G-buffer drew nothing),
   bloom (six half-size levels, added back up).
10. **Tone mapping** (the game's curve, ACES or AgX-like).
11. **TAA** (sub-pixel jitter; history found by the motion vectors, by
    depth and the camera elsewhere; history from another depth, or
    outside the colours around the pixel now, is dropped) or FXAA.
    Or AMD FSR 3 (`antialiasing = 3`, `enhanced/fsr.rs`): the passes run
    at the size its mode asks (native, or 1.5, 1.7, 2 or 3 times smaller)
    with FSR's jitter sequence; the tone-mapped image, the depth and the
    motion (the camera's where the G-buffer drew nothing) go to AMD's
    upscaler through the FidelityFX API, recorded in a Direct3D 12
    command list of its own on wgpu's queue between the frame's two
    submissions (wgpu puts the textures in the states the dispatch names
    first); its output at the game's size is resolved. Without its DLLs,
    or off Direct3D 12, TAA stands in.
12. **Resolve** into the game's targets at the game's size (the enhanced
    passes run at `render_scale`), with sharpening. Where the passes run
    at another size (FSR, `render_scale`) or the editor outlines a pick,
    the scene's depth and draw ids are drawn once more at the game's size
    without jitter (`fs_edge`): the depth the game's later draws test
    against (save spheres, effects) then follows the upscaled image's
    edges, and the editor's dashed outline is drawn at the screen's
    resolution, over the finished image.
13. **Effect light**: the game's additive effect draws (fire, magic) drawn
    once more onto black, blurred wide, light the surfaces around them.

**Glow lights** (`glow.rs`): the `Effects` of a frame are drawn again
alone, against the scene's depth and for their own nearest depth;
`glow.wgsl` sums per 32×32 tile the light, its position and the world
area its pixels cover. A frame later the tiles are read back, tiles within
12 units become one point light (its colour the glow's, its brightness by
light × area, so it does not depend on distance), and lights found again
within 15 units ease toward the new figures. A light stays while the
scene lasts, off screen too; where it should show and no glow is found,
it fades. A new scene forgets them.

Then the game's own draws for the rest of the frame. On many maps these
include the game's afterimage: the scene times a colour whose alpha is
the new frame's share (0.8 on the S.S. Winno) is blended into a history
image (pixel shader `e928905e31636cf1`), which the next draw lays over the
scene, so whatever moves leaves a faint trail. With `afterimage = false`
(the default) the renderer's answer tells the hook to draw that blend
without blending, so the history is the frame itself; the game's later
use of it (fades, distortion) is unchanged.

## Settings and the overlay

F10 shows the overlay in the game: every setting, live, with the frame
rate (figures over half a second, in fixed widths). The overlay goes to
the game as the part of the screen that changed (what the panels cover
now and covered before), read back from the GPU a frame late, and not at
all while it stays the same; the hook keeps the whole image and lays it
over each frame. The mouse is the
overlay's while it shows; keys reach the game unless the pointer is over
the panel or a text field has focus (the renderer's answer carries flag
16, and the hook's DirectInput wrap then reads the keyboard as released).
What does not fit on the screen scrolls.

Settings live in profiles: `profiles/<name>.toml` next to
`ffx-render.exe`, the active one's name in `profiles/active`; the first
start makes "Default" from `ffx-render.toml`. The active settings are also
in `ffx-render.toml` (edits to that file apply within half a second).
A scene can ask for a profile (the editor's Light tab, "Settings
profile"; for all scenes: the one for scenes without their own): entering
it switches to that profile, and scenes that ask for none go back to the
profile last chosen by hand (`profiles/chosen`). Unsaved changes are
kept: then the profile stays and the overlay says so.
"Save" writes the active profile, "Revert" goes back to it, and a setting
that differs from it has a ↺ button that resets just that one. Profiles
can be made (factory defaults), duplicated, renamed and deleted; "Copy"
puts the saved profile on the clipboard as TOML under a
`# FFX renderer profile: <name>` line, "Paste" makes a profile from such
text. The second line has the renderer's CPU and GPU time per frame (GPU
timestamps between the passes, a few frames old; "GPU time per pass"
lists them; where the GPU cannot write timestamps it shows "?"); the
game's own drawing and the wait for the screen are not in it.
Debug views show one input of the lighting (shadow, occlusion,
normals, albedo, baked light, cascades, skin, probe light).

## Minimap

The renderer draws its own field minimap (`minimap.rs`; overlay section
"Minimap": on/off, size, range, opacity) in place of the game's: round, at
the top right of the game's 16:9 picture (not on the bars of a wider
screen) and beside the editor's panel, under the overlay's windows; always
centred on the player, turned so that where the camera looks is up. It
takes the game's look on a small CRT screen: a dark green panel, a light
green grid fixed to the world (four cells to the edge), the walkable area
in steel blue with a glowing edge, the player as a yellow arrow of the
character's facing in a magenta ring; scanlines, a slow refresh band,
darker toward the rim, a sheen on the glass. Markers keep the game's shape
and colour; beyond the edge they sit on the rim line and fade in as the
player comes nearer (from 1.5 radii beyond it); the destination marker
always shows on the rim, exits show as bars. The renderer's answer carries flag 512; the hook then
wraps `rcbgGuideUpdateEx` (`0x0091da80`): it keeps the fade (`Guide_FadeStep`),
the fog of war (`Guide_RevealFog`) and the blink counter, turns the game's
mesh and fog passes off, and sends `Msg::Minimap` before each frame: the
fade (`g_GuideFade`), the markers from `AtelGetGuideMapActorInfo`
(`0x0086b740`, colours from `g_GuideMarkerPalette`), the player among them
(flag 0x800: position, yaw), and the mesh (`Guide_MeshObject`: matrix at
+0xc0, i16 vertices and type 0 triangle packets, `g_GuidePacketStrides`)
when it changes. Maps the game shows fixed in place scroll too. The HUD
shows with the overlay's panels hidden (flag 256: the hook lays the
overlay image over the frame).

## Free camera

"Free camera" in the overlay (`freecam.rs`) starts where the game's view
is, orbiting the point the game's camera looks at, and is driven as
Blender's viewport: middle-drag orbits (turntable), Shift+middle pans
(the pivot follows the pointer), Ctrl+middle (up and down) and the wheel
zoom, Ctrl/Shift+wheel pan across and up; number pad 1, 3, 7 front, right,
top (Ctrl: the opposite), 2/4/6/8 orbit by 15°, 9 turns round, +/− zoom,
. centres on the selected point or spot light. Shift+` (the key left of 1:
VK_OEM_3 or VK_OEM_5) walks as Blender's walk navigation: the mouse looks,
W/A/S/D or the arrows move, Q/E down and up, Shift runs (×5), Alt slows,
the wheel or +/− sets the speed (2.5 m/s to start); left click or Enter
keeps the view, right click or Esc goes back. While walking the renderer
holds the pointer (flag 1024: the hook keeps it at the window's centre,
hidden, and sends `Input::Motion`). The screen's right comes from the
game's view matrix (render space's handedness). "zoom" narrows or widens
the game's field of view. The overlay keeps the keyboard meanwhile (Alt
keys too, but Alt+F4). The renderer's answer carries it (flag 64: eye and
look-at in game space, the field of view, the roll); the hook wraps
`MsScreenProcess` (`0x007bc110`): right before, the game's own override camera
(`MsEffectCameraSet`) holds it, and after, the view and field of view go to
the game's renderer as `Cam_Apply` would send them (`graphicCameraSetFOVy`,
`graphicCameraSetWorldMatrix`), which the override's branch skips. The game then draws
and culls for it; what lies outside the map's camera views may be
unfinished. Off: the override ends.

## Sequence editor

The editor's Sequence tab (`sequence.rs`) records real-time cutscenes and
keys their depth of field. Record keeps the frames the game sends from
then on (compressed with LZ4, about 0.2 MB a frame; two minutes at most,
a new scene ends it) and their cameras; Stop makes a sequence of the scene
with that take; only frames while the game plays are kept. Pause game uses the game's own pause (`g_PauseOn`: the
world stops, rendering goes on; its "PAUSE" text is hidden, the
renderer's answer carries flag 128; Resume starts the pause's fade-out as
Start does). Paused, the take's frame at the timeline's cursor is drawn
(with the current settings and the keys' depth of field) and shown over
the whole screen; it plays, steps and jumps. Keys set the focus distance
(world units; "pick" takes it from a click in the picture), strength and
radius at a frame; between keys they ease from one to the next, and
their depth of field applies at once (not eased by "focus speed"). The
timeline shows the keys as ticks where the cursor stands on their frames.
`sequences.toml` beside the renderer keeps each sequence's camera path
and keys (takes last the session: they draw with the game's objects of
the moment). In play, a scene's camera is matched to its sequences'
paths (eye within 0.5 units, look-at within 2), followed frame by frame;
where it follows one, its keys set the depth of field.

## Materials

The G-buffer has, besides albedo (roughness in alpha), normal, baked
light and motion, a material image (metallic, specular, emission, skin)
and the id of the draw at each pixel (for picking; its top bit marks what
the editor highlights, which the resolve outlines). A draw's material
comes from its colour texture (by the hash material packs use): the
editor's `Material` for it, else automatic: the generated material map,
and on characters skin found by its colour (one atlas holds a
character's face and clothes), which takes `skin_roughness` and
`skin_detail` (a share of the surface detail) by `skin_materials`.
Kinds (skin, hair, cloth, leather, metal, stone, wood, glossy, glowing)
give starting values; every value can be set on its own. Metals lose
their diffuse light and reflect their colour; emission adds the
surface's colour as light. An image in `textures/` can stand in for a
texture (`Material::texture`, mip levels made on load).

## The editor

F11 (or "Editor" in the panel) opens a sidebar: changes for this scene or
for all scenes (`edits.rs`), kept in `edits.toml` next to
`ffx-render.exe`.

Light: the scene's settings profile, then its lights. Every scene without
lights of its own uses the default lights ("all scenes"); "Give it its
own" copies them for the scene, "Back to the default" drops its own. A
rig has an exposure (EV100: higher is darker; 12 shows the game's light
as the game does) and a list of lights, each with Unreal-like properties
in physical units: the original light (a multiple of the game's painted
light, with its key shadows, occlusion and highlights, and the bounce it
gives through the probes), sky light (nits, upper and lower colour),
directional lights (lux; the first with shadows is the key light:
cascades, ray-traced shadows, light shafts; source angle), point and spot
lights (candela; source radius, attenuation radius, cone angles; placed by
a click in the scene: the pick reads the draw id and the depth around the
pixel for the surface and its facing); colour or temperature (kelvin);
how strongly each lights scenery and characters; specular; static or
dynamic. The default lights are the game's: the original light and the
map's key, fill and ambient light, which follow the game's direction and
colour (at the game's brightness 1 the intensity is what is shown) until
told not to. World units: 20 to the height of a character, about 1.75 m.
Every point and spot light shows as an icon over the scene (a click
selects it); the selected light has Blender-like handles (`gizmo.rs`,
through the game's camera, drawn under the panels): Move, arrows along
the world axes; Rotate, rings about them (directional lights at what the
camera looks at); Scale, the circle of its reach. A drag on a handle does
not pick.
Then the lights found in glows (off, brighter or dimmer, or made into a
light of one's own).

Materials: a click picks a texture (its draws outlined, a preview shown),
whose material is then set. Files: save, revert, export to
`edits/<name>.toml`, and take in a file, which lists each scene it changes
as new, differing or the same, to tick the ones to take.

## Material packs

Each texture is known by the FNV-1a hash of its top level as the game
creates it. `materials/<hash>.png` next to `ffx-render.exe` (RGBA8: red
and green the tangent-space normal, blue roughness, alpha cavity) replaces
the map made from that texture. To start a pack, export the generated
ones: `FFX_RENDER_EXPORT_MATERIALS=<dir>` in the game, or
`ffx-render replay ... --export-materials <dir>`. Upscaled texture mods
that replace the game's files work as they are (loose files under
`mods/<mod name>/<archive path>`, see `docs/hooking.md`): the renderer
draws whatever the game loads. Material maps made from colour textures
are at most 2048 texels across (from the texture's mip of that size), so
4K and 8K textures do not cost hundreds of MB each.

## Tools

```bash
tools/build-renderer.sh                       # ffx-render.exe (and DXC's DLLs, see below)
tools/run-game.sh --renderer                  # the game with the renderer
FFX_PROBE_BATTLE=1 cargo run --release -p ffx-harness --example live_probe -- <out> 1 120 3
cargo run --release -p ffx-harness --example gfx_capture -- <dir>          # capture a frame
target/x86_64-pc-windows-msvc/release/ffx-render.exe replay <dir>/capture.ffxgfx <out> \
    --enhanced [--until N] [--settings ffx-render.toml] [--set key=value] [--materials DIR]
```

In the game, Ctrl+F12 captures the next frame the same way into
`captures/<seconds since 1970>` beside the game (a few hundred MB), for a
look at a frame that shows something wrong.

`replay` without `--enhanced` draws the captured frame with the game's
shaders only; it matches the original within rounding.

DXC compiles the Direct3D 12 shaders about twice as fast as the built-in
FXC (shorter stalls when a new area loads). `tools/build-renderer.sh`
copies `dxcompiler.dll` and `dxil.dll` next to the renderer when the
[DirectXShaderCompiler release](https://github.com/microsoft/DirectXShaderCompiler/releases)
is unpacked in `FFX_DXC_DIR` (default `~/tools/dxc`).

FSR's DLLs (`amd_fidelityfx_loader_dx12.dll`,
`amd_fidelityfx_upscaler_dx12.dll`, signed, from the FidelityFX SDK v2.0.0
`Kits/FidelityFX/signedbin`, MIT licence) go next to the renderer with
`LICENSE-FidelityFX.txt` when the SDK is in `FFX_SDK_DIR` (default
`~/tools/ffx-sdk`); they are not in the repository.
