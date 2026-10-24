# ffx-draw: the in-process renderer core

M4 rebuilds the platform inside the game's process (`docs/rewrite-plan.md`,
"M4" and "One process, then 64-bit"). The scene renderer draws on the
game's own Direct3D 11 device with the game's own DXBC shaders, so its
output can be identical to PhyreEngine's, bit for bit. `crates/ffx-draw`
is the layer it draws through. It is portable (it builds for i686 and
x86_64, Windows or not) and has one backend, Direct3D 11.

Round 1 proves the layer: a captured frame's scene is drawn through it in
place of PhyreEngine's calls (the replay mode, below). It must give the
same render targets and the same call stream. Later rounds replace "replay
a capture" with the Rust scene builder.

## Layers

| Module | What it is |
|---|---|
| `desc` | Object descriptions field by field: `BufferDesc`, `TextureDesc`, `ViewDesc`, `InputElement`, `BlendDesc`, `DepthStencilDesc`, `RasterDesc`, `SamplerDesc`. States read and write the exact bytes of the D3D11 structures (`from_d3d11`/`to_d3d11`; padding is not a field) |
| `state` | `DrawState`: everything a draw has bound (topology, layout, vertex and index buffers, per stage the shader, constant buffer windows, shader resources and samplers, blend/depth/raster states with factor, mask and stencil reference, viewport, scissor, targets). `DrawCall`: the four draw calls |
| `backend` | The `Backend` trait and its handles (`Shader`, `Layout`, `Blend`, `DepthState`, `Raster`, `Sampler`, `Buffer`, `Texture`, `View`) |
| `cache` | The pipeline-state layer: one backend object per shader (by its DXBC), input layout (elements and vertex shader), state description and constant buffer size, made on first use |
| `replay` | A `capture.ffxgfx` issued through a backend: `Capture::parse`, `select_scene`, `Replayer`, and `compare_draws` / `write_capture` for checking it |
| `d3d11` (Windows) | `D3d11`, the backend on a device and context it is given |

The vocabulary is Direct3D 11's: DXGI formats, `D3D11_BLEND`,
`D3D11_FILTER` and the rest keep their numbers, because the game's data and
shaders are written against them. A later backend translates from those
numbers, as `ffx-render`'s `formats.rs` does for wgpu today.

### The backend trait

```rust
pub trait Backend {
    fn begin(&mut self);   // save what the backend may change
    fn end(&mut self);     // put it back
    fn create_shader(&mut self, stage: Stage, dxbc: &[u8]) -> Result<Shader>;
    fn create_layout(&mut self, elements: &[InputElement], vs_dxbc: &[u8]) -> Result<Layout>;
    fn create_blend(&mut self, desc: &BlendDesc) -> Result<Blend>;
    fn create_depth_state(&mut self, desc: &DepthStencilDesc) -> Result<DepthState>;
    fn create_raster(&mut self, desc: &RasterDesc) -> Result<Raster>;
    fn create_sampler(&mut self, desc: &SamplerDesc) -> Result<Sampler>;
    fn create_buffer(&mut self, desc: &BufferDesc, data: Option<&[u8]>) -> Result<Buffer>;
    fn create_texture(&mut self, desc: &TextureDesc, init: &[InitData]) -> Result<Texture>;
    fn create_view(&mut self, texture: Texture, kind: ViewKind, desc: Option<&ViewDesc>) -> Result<View>;
    fn write_buffer(&mut self, buffer: Buffer, offset: u32, data: &[u8]);
    fn clear_target(&mut self, view: View, color: [f32; 4]);
    fn clear_depth(&mut self, view: View, flags: u32, depth: f32, stencil: u8);
    fn copy_texture(&mut self, dst: Texture, src: Texture);
    fn draw(&mut self, state: &DrawState, call: DrawCall);
    fn view_texture(&self, view: View) -> Option<Texture>;
    fn read_texture(&mut self, texture: Texture) -> Result<Readback>;
}
```

A draw is a whole `DrawState`, not a series of setters. The backend
decides what to send. The D3D11 backend sends only what changed since its
last draw. A Vulkan or wgpu backend (M6) would turn the same state into a
pipeline object and bind groups, as `ffx-render` does. Nothing else in the
trait is specific to D3D11.

The backend never makes an output of its own. Inputs (textures, buffers)
are made from descriptions. Outputs come from the host:
`D3d11::import_view` takes one of the game's views and the texture behind
it.

### The D3D11 backend

- `D3d11::borrow(device, context)` takes a reference on the game's device
  and on its real immediate context, the one behind the graphics layer's
  proxy (`gfx/proxy.rs`). Its context calls therefore bypass the layer's
  hooks: the layer's record of what the game bound stays the game's. Its
  device calls go through the layer's device hooks, which only record.
- `begin` reads back everything `draw` can change: IA, VS/PS/GS shaders,
  constant buffers with their `…1` windows, shader resources, samplers,
  rasterizer state, viewports, scissors, render targets, depth target and
  UAVs, blend and depth-stencil states. `end` sets it all again, targets
  before shader resources. PhyreEngine caches what it has bound and skips
  redundant calls, so this restore is required.
- Dynamic buffers are written with `Map(WRITE_DISCARD)` of the whole
  buffer. Other buffers are written with `UpdateSubresource`. The backend
  keeps the bytes it last wrote to each buffer (`written`) for the replayed
  stream.
- `bound()` reads the context's state back with `Get*` and maps every
  object back to a handle. The replayed stream is built from this, so it
  records what the API really had bound at each draw, not what the
  replayer meant to bind.

## The replay mode

`gfxreplay <capture.ffxgfx> [out dir]` is a harness command
(`crates/ffx-hook/src/gfx/replay.rs`; Windows paths; needs `FFX_GFX=1`).
The default out dir is `replay` beside the capture.

1. The harness thread reads the capture and selects its scene
   (`select_scene`): the first run of draws into two render targets with a
   depth target and nothing else (the layer's `live::draw` rule), plus the
   clears of those three targets, the buffer updates, and the texture
   copies inside the run. The dumps `gfxcapture` took come from the
   capture's `frame.txt`.
2. It points six entries of the proxy context's vtable at the replay (the
   four draws and the two clears) and waits.
3. At the game's first scene draw of the next frame (render thread, inside
   that draw), it checks that the game's three scene views match the
   capture's in size and format, imports them, and calls `begin`. It then
   issues the selected commands through `Replayer::run`. Each object a
   command needs is made on first use from the capture: shaders from their
   DXBC as they are, input layouts against the draw's vertex shader, states
   from their D3D11 bytes, buffers and textures with their contents.
   Constant buffers are not objects in the capture: each draw's bytes go
   into one dynamic buffer per stage, slot and size.
4. After each draw it reads the context's state back and compares it with
   the captured draw (`compare_draws`). After a draw where the capture
   dumped a scene target, it reads that target back and compares the bytes
   with the dump. At the end of the scene it also compares every target
   with the capture's copy of its texture. That copy was read back when the
   captured frame ended, so it is a reference only for a target nothing
   draws into after the scene: on cdsp0000 that is the normal target.
5. `end` restores the context. The game's own draw is skipped, and so are
   its later scene draws and its clears of the scene targets until the
   frame ends. At the next frame the six vtable entries go back to the
   layer's.

Off, the mode costs nothing: the vtable holds the layer's own entries and
no replay code runs. A replay that sees no scene draw within 120 s gives up
and puts the entries back. The replay changes only that one frame. Its
texture copies go to textures the replay made, not the game's own, so the
game's effects later in that frame read the game's copy as it was.

The out dir gets:
- `replay.txt`: the summary, the gaps, and one line per dump and per
  target;
- `*.replayed.raw` and `scene_*.raw`: the targets as the replay left them;
- `replay.ffxgfx`: the captured frame with its scene commands replaced by
  the replayed ones (state read back from the context, constant buffers as
  written), for `drawdiff`.

### Running it

```bash
tools/build-hook.sh
tools/draw/replay.sh <run> [capture dir...]   # ~/ffx-work/m4/captures/<run>
cargo build --release -p ffx-drawdiff
tools/draw/diff-run.sh <run> [capture dir...] # drawdiff --part scene, cdsp0000 allow list
```

`replay.sh` plays `tools/draw/replay-field.txt` (S.S. Winno deck, the
field-walk-1 frame at event count 101) through the harness in background
mode:
- it captures a frame (`cap`) and replays it in the next frame (`replay`);
- 200 frames later it captures again (`cap2`), replays the first capture
  (`replay-later`) and the second one (`replay2`);
- it replays each extra capture dir given, at the end (`replay-<name>`).

`replay-later` checks that the output is the replay's own: the game is
200 frames on by then, so its own draws would not match the old capture.

Off-game tests: `cargo test -p ffx-draw` runs the replayer against a
recording backend on a synthetic capture. With
`FFX_DRAW_CAPTURE=<capture.ffxgfx>` it also runs a real capture and checks
that every state description survives the decoding.

## Results (2026-10-24, cdsp0000)

Run `t3-d`, field-walk-1 frame:

| Capture replayed | Scene draws | Colour (rt dump) | Depth (rt dump) | Normal (texture at frame end) | drawdiff `--part scene` |
|---|---|---|---|---|---|
| `cap` (next frame) | 128 | identical | identical | identical | 0 |
| `cap`, 200 frames later | 128 | identical | identical | identical | 0 |
| `cap2` | 127 | identical | identical | identical | 0 |
| `fw1-a` (T1 reference, another process) | 128 | identical | identical | identical | 0 |
| `fw2-b` (T1 reference, another process) | 128 | identical | identical | identical | 0 |

- The in-process stream compare (`compare_draws`) found 0 differing draws
  in every replay. As a negative check, drawdiff of `cap` against
  `replay2` finds 6,377 differences.
- No gaps: every object the scene uses could be made from the capture.
- Per replay the backend made 99 textures, 99 views, 655–660 buffers,
  26 shaders, 12 layouts, 11 states and 21 constant buffers, all in about
  0.23 s.
- Colour and depth differ from their end-of-frame texture copies (as
  expected: the post passes and a depth clear come after the scene).

### What the capture lacks, and what the replay makes

- Constant buffers: the capture keeps each draw's window bytes, not the
  buffer object. The replay makes its own dynamic buffers, sized to the
  window. A window that ran past the end of the game's buffer is cut short
  in the capture and padded with zeros, which is what the API reads there.
- Geometry shaders are not in the layer's bound state or in the stream.
  The replay binds none. Whether the game binds one for a scene draw is
  unknown (the targets match bit for bit on cdsp0000, so not there).
- Textures are read back when the captured frame ends. A texture the frame
  writes before a scene draw reads it would be wrong in the replay. None
  is on cdsp0000; scene copies (`CopyTexture`) are replayed in order.
- Dynamic index buffers get no per-draw updates in the stream (dynamic
  vertex buffers do): their contents are the copy at the frame's end. On
  cdsp0000 one scene draw uses one, and that copy was right for it.
- Render target view descriptions are five words (the layer records
  five). Depth and shader resource views are six.
