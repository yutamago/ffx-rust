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
   snapped to whole texels.
2. **Ray tracing structures** (where the GPU has ray queries): the opaque,
   unskinned scenery as bottom-level structures (kept while their buffers
   are), all of them in a top-level structure each frame.
3. **G-buffer**: albedo (roughness in alpha), world normal and material,
   the game's baked vertex light and fog weight. Material maps (normal,
   roughness, cavity) come from a pack or are made from the colour
   texture.
4. **Light probes**: up to two per frame at the characters; the scenery
   around each, drawn into a small cube, becomes nine spherical
   harmonics.
5. **Ambient occlusion** (screen space, blurred).
6. **Lighting** (linear HDR). Scenery keeps its baked light as the base
   (the game's art), reshaped by the key light and its shadow; characters
   get the map's key and fill light with the game's soft wrap, light from
   the probes, and skin scattering on skin-coloured parts. Shadows are
   ray-traced from the scenery where possible, from the cascades
   otherwise.
7. **Water**: the game's water draws, with its two normal maps, its cube
   map tinted as the game tints it, screen-space reflection, refraction
   with absorption, and the game's opacity.
8. **Volumetric light**: the key light through the cascades, half
   resolution.
9. **Tone mapping** (the game's curve, ACES or AgX-like).
10. **TAA** (sub-pixel jitter, reprojection by depth) or FXAA.
11. **Resolve** into the game's targets at the game's size (the enhanced
    passes run at `render_scale`).
12. **Effect light**: the game's additive effect draws (fire, magic) drawn
    once more onto black, blurred wide, light the surfaces around them.

Then the game's own draws for the rest of the frame.

## Settings and the overlay

F10 shows the overlay in the game: every setting, live. "Save" writes
`ffx-render.toml` next to `ffx-render.exe`; edits to that file apply
within half a second. Debug views show one input of the lighting (shadow,
occlusion, normals, albedo, baked light, cascades, skin, probe light).

## Material packs

Each texture is known by the FNV-1a hash of its top level as the game
creates it. `materials/<hash>.png` next to `ffx-render.exe` (RGBA8: red
and green the tangent-space normal, blue roughness, alpha cavity) replaces
the map made from that texture. To start a pack, export the generated
ones: `FFX_RENDER_EXPORT_MATERIALS=<dir>` in the game, or
`ffx-render replay ... --export-materials <dir>`. Upscaled texture mods
that replace the game's files work as they are: the renderer draws
whatever the game loads.

## Tools

```bash
tools/build-renderer.sh                       # ffx-render.exe (and DXC's DLLs, see below)
tools/run-game.sh --renderer                  # the game with the renderer
FFX_PROBE_BATTLE=1 cargo run --release -p ffx-harness --example live_probe -- <out> 1 120 3
cargo run --release -p ffx-harness --example gfx_capture -- <dir>          # capture a frame
target/x86_64-pc-windows-msvc/release/ffx-render.exe replay <dir>/capture.ffxgfx <out> \
    --enhanced [--until N] [--set key=value] [--materials DIR]
```

`replay` without `--enhanced` draws the captured frame with the game's
shaders only; it matches the original within rounding.

DXC compiles the Direct3D 12 shaders about twice as fast as the built-in
FXC (shorter stalls when a new area loads). `tools/build-renderer.sh`
copies `dxcompiler.dll` and `dxil.dll` next to the renderer when the
[DirectXShaderCompiler release](https://github.com/microsoft/DirectXShaderCompiler/releases)
is unpacked in `FFX_DXC_DIR` (default `~/tools/dxc`).
