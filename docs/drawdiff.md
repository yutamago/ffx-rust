# drawdiff: the draw-stream comparison

`ffx-drawdiff` compares two captures of a frame's Direct3D 11 calls, draw
by draw. It is the gate for every M4 part (`docs/rewrite-plan.md`, "M4"):
the Rust side draws on the game's own device with the game's own shaders,
so its call stream must match PhyreEngine's bit for bit. There is no
cross-API rounding to allow for. Pixel differences (shader numerics) are
the pixel gate's concern (T3).

```bash
cargo build --release -p ffx-drawdiff
target/release/drawdiff A B [--part scene|all] [--tolerances F] [--allow F] \
    [--limit N] [--show-allowed] [--suggest-allow]
```

`A` and `B` are `capture.ffxgfx` files, or folders holding one (what
`gfxcapture <dir>` writes, `crates/ffx-hook/src/gfx/capture.rs`). A is the
reference (the original), B the candidate. The exit status is 0 when every
difference is allowed, 1 when any is not, and 2 on bad arguments or an
unreadable file.

- `--part scene` compares only draws into the scene targets (render
  targets 0 and 1 and a depth target, as `live::draw` selects them), plus
  clears and copies of those targets. The default is `all`.
- `--limit N` prints at most N field lines per changed pair (default 12);
  the rest are counted.
- `--show-allowed` also prints the differences the allow list covers.
- `--suggest-allow` prints allow-list lines for the differences found, each
  with `cause: TODO`. The rule reader refuses `TODO`, so every cause must
  be written before the list can be used.

`cargo run --release -p ffx-drawdiff --example items -- <capture> [n]` lists
a capture's items with their keys, or every field and constant of item `n`.

## Design

`crates/ffx-drawdiff/src/`:

| Module | What it does |
|---|---|
| `capture` | Reads the stream (`ffx_gfx::read_all`) and keeps the objects by id, as the stream declares them (a new object at a reused id replaces the old one). Each `Frame` becomes a list of `Item`s: one per draw, clear and copy. The frame's `UpdateBuffer` commands are applied to the dynamic buffers in order, so each draw sees its own version |
| `key` | Stable content keys (below). T4 uses this module |
| `state` | Blend, depth-stencil, rasterizer and sampler descriptions, decoded field by field |
| `cb` | Constant buffers decoded by the shader's reflection |
| `align` | Myers' O((N+M)D) diff |
| `diff` | Alignment and field-by-field comparison |
| `rules` | Tolerance and allow-list files |
| `report` | The text report |

An `Item` holds its fields (name, class, value), with every object
reference replaced by a key. It also holds the used constants of each
stage and slot, a *match key* (what alignment pairs on) and a
*fingerprint* (a hash of everything compared). Two items with the same
fingerprint have no differences.

### Keys

Ids in a capture are the game's object addresses, which change from run
to run. Every reference is replaced by a key:

| Object | Key |
|---|---|
| shader | FNV-1a 64 of the bytecode (`ffx_dxbc::hash`, the graphics layer's shader names) |
| texture | FNV of its width, height, mips, array size, format and samples, then the texels of mip 0 of every slice, without the readback's row padding |
| static buffer | FNV of its width, bind flags and contents |
| dynamic ring (`D3D11_USAGE_DYNAMIC`) | FNV of the bytes the draw reads, never the offset |
| input layout | FNV of its elements |
| render or depth target | its role: the n-th target in order of first use in the frame (as render target, depth target, shader input or copy) |
| view | its resource's key and the view description (format, dimension, ...) |

States have no key. They are compared field by field.

Targets are keyed by role, not by texture description. The swap chain's
buffers are not created through the device, and the layer's texture table
can hold a stale texture at their address (in capture fw3-b, an R24G8
depth texture sits at the back buffer's address). Each target's
description is still shown in the report labels. The view's format is
part of the key. Sizes are checked through the viewport, and through the
pixel gate.

**Rings.** The bytes a draw reads from a dynamic vertex buffer are:

- for a per-vertex slot of an indexed draw, records `min index + base`
  through `max index + base`;
- for a plain draw, records `start` through `start + count - 1`;
- for a per-instance slot, records `first_instance` through
  `first_instance + ceil(instances / step) - 1`.

Each range is taken from the binding's offset. A stride of 0 reads its
first 16 bytes. A dynamic index buffer is keyed by the index values it
holds. These are shifted down by the smallest index when every per-vertex
slot is a ring, because then the vertex window moves with them. The
fields that only say where a ring was written are not compared: the
binding offset of a ring, `start` (into a ring index buffer, or into ring
vertices for a plain draw), `base` (when every per-vertex slot is a ring)
and `first_instance` (when every per-instance slot is a ring). Static
buffers keep all of these fields.

### Constants

The window each shader saw (the capture keeps the bound window of every
constant buffer per draw) is decoded with the shader's own reflection
(`ffx_dxbc::Reflection`, `RDEF`). Only slots the shader binds are decoded,
and only variables flagged `D3D_SVF_USED`, word by word (floats by their
bits). Padding and unused variables are never compared.

A word is reported as `<stage>.cb<slot>.<variable>`. If the variable is
wider than one word, a `[register].component` suffix counts from the
variable's start. Struct members take their member path, for example
`vs.cb0.scene.GlobalTime` or `ps.cb0.scene.ViewProjection[2].y`. A shader
without reflection has its whole window compared as `cb<slot>` (none in
cdsp0000: all 207 items decode by reflection).

Shader resources and samplers are compared only on registers the shader
binds (texture and sampler bindings in `RDEF`).

### States and the other fields

| Field | Compared |
|---|---|
| `draw.kind`, `draw.count`, `draw.instances`, `draw.start`, `draw.base`, `draw.first_instance`, `topology` | always (ring offsets excepted, above) |
| `blend.*` | `alpha_to_coverage`, `independent`, and per target `enable, src, dest, op, src_alpha, dest_alpha, op_alpha, write_mask`. Targets 1–7 only with `IndependentBlendEnable` (Direct3D ignores them otherwise) |
| `blend_factor.*` | only when an enabled target blends with `BLEND_FACTOR` or `INV_BLEND_FACTOR` |
| `sample_mask` | always |
| `depth.*` | enable, write mask, func, stencil enable; with the stencil on, its masks and both faces |
| `stencil_ref` | only with the stencil on |
| `raster.*` | all ten fields (bias clamp and slope as float bits) |
| `viewport.*` | six floats, by bits |
| `scissor.*` | only with `ScissorEnable` |
| `<stage>.sampler<n>.*` | all thirteen fields (LOD bias, border and LODs as float bits) |
| `layout`, `vs`, `ps`, `vb<n>`, `ib`, `<stage>.srv<n>`, `rtv<n>`, `dsv` | by key |
| `vb<n>.stride`, `vb<n>.offset`, `ib.format`, `ib.offset` | as values (offsets of static buffers only) |
| clears | the target, the colour, or the flags, depth and stencil |
| copies | the destination and the source |

A null state (id 0) decodes as Direct3D's default description, so an
explicit state equal to the default equals none. Padding bytes in the
descriptions (after `RenderTargetWriteMask`, after the stencil masks) are
skipped.

### Alignment

1. Myers on the fingerprints: equal items pair up as equal.
2. Each gap between equal runs is aligned again, by Myers on the match
   keys. A draw's match key is (kind, vs key, ps key, per vertex slot its
   key, index key, start, count), the plan's alignment tuple. A clear's or
   copy's match key is its kind and target. Ring slots enter the match key
   as "a ring", not by their contents: other bytes in the same place make
   a changed draw (a `resource` difference), not a missing one and an
   extra one. Pairs found here are compared field by field.
3. What is left pairs up as **moved**, first by fingerprint (the same item
   elsewhere in the order, with no other difference) and then by match
   key (elsewhere, and its field differences are reported too).
4. What is still left is **missing** (only in A) or **extra** (only in B).

Edit distances past a 64 M-entry trace are not searched (the rest becomes
deletions and insertions, which step 3 still pairs). Frame sizes of a few
thousand items are far below that.

### Classes

| Class | Means |
|---|---|
| `missing` | an item of A with no counterpart in B |
| `extra` | an item of B with no counterpart in A |
| `moved` | the same item at another place in the order |
| `state` | draw parameters, topology, blend, depth, raster, sampler, viewport, scissor, sample mask, clear values, ring-less offsets and strides |
| `constant` | a used shader constant |
| `resource` | a bound object: buffer, ring bytes, texture, target, layout, shader |

### Floats

Floats are compared by their bits. `-0` differs from `+0`, and a NaN
differs from a NaN with another payload. The report shows both bit
patterns, both values and the distance in ULPs (`-0` and `+0` are 1
apart; NaNs have no distance).

## Rules

Tolerance (`--tolerances`) and allow-list (`--allow`) files use one
syntax (`src/rules.rs`): one rule per line, `#` comments, and `[scene]`
sections (the scene is the `World` message's event file, for example
`cdsp0000`; `[*]` or no section means every scene).

```text
[cdsp0000]
constant var=scene field=*.cb0.scene.GlobalTime cause: ...
resource kind=draw vs=bb9bf1c768e1480f field=vb5 cause: ...
ulp 1 ps=0123456789abcdef var=Fog* cause: ...
```

- An allow line starts with a class (`missing`, `extra`, `moved`,
  `state`, `constant`, `resource`) or `any`. A tolerance line starts with
  `ulp <n>` and goes only in the tolerance file.
- Selectors: `vs=<hex>`, `ps=<hex>` (shader keys), `kind=draw|clear|
  clear-depth|copy`, `field=<glob>`, `var=<glob>` (the constant's
  variable). `*` is the only wildcard.
- Every rule needs `cause:` followed by the reason. A rule without one, or
  with `TODO`, is refused.

A tolerance covers two float words of a matching field that are at most
`n` ULPs apart (NaNs never). There is no tolerance file yet: in-process
comparisons have no rounding to tolerate. An allowed difference is still
counted and listed with `--show-allowed`.

The noise allow lists live in `crates/ffx-drawdiff/allow/<scene>.txt`.
To build one, diff original runs with `--suggest-allow` and write a cause
for every line.

## Report

```text
drawdiff
  A <path> (<n> frames)
  B <path> (<n> frames)
  part all, <n> tolerance rules, <n> allow rules

frame 0 (scene cdsp0000): A 207 items, B 207 items; 207 pairs, 80 equal
  missing 0, extra 0, moved 0, state 0, constant 252, resource 5, allowed 0, within tolerance 0
  missing  A#<n> <label>
  extra    B#<n> <label>
  moved    A#<n> -> B#<m> <label>
  changed  A#0116 B#0116 draw Indexed count 288 vs 1e6f10bb647d242c ps faa929e5c2397622
    constant vs.cb0.SgSpec_LightVec[0].z: 0x0ee26660 (5.5811885e-30) | 0x0ef14740 (5.94797e-30), 975072 ulp
  changed  A#0111 B#0111 draw Indexed count 192 vs bb9bf1c768e1480f ps 8b5157eb9c138a63
    constant vs.cb0.scene.GlobalTime: 0x405be770 (3.4360008) | 0x405bf7d0 (3.4370003), 4192 ulp
    resource vb5: ring 8dde9a219c37c1af 1024 B | ring 3eeb2e7fdf2219d2 1024 B
      ... and <n> more in this pair

result: <n> differences (<n> more allowed)
```

The `changed` lines are from fw1-a against fw1-b without the allow list
(trimmed). `A#n` and `B#n` are item indices in the frame: draws, clears and copies
in command order (`items` lists them). A changed pair prints one header
line, then its fields as `A value | B value`. Lines are in A's order.

## Gate

`cargo test -p ffx-drawdiff` (`tests/synthetic.rs`, built from synthetic
captures through the file format):

| Test | Change | Expected |
|---|---|---|
| `a_capture_equals_itself` | none | 0 differences |
| `a_dropped_draw_is_missing` | a draw removed (next to one with the same match key) | 1 `missing` |
| `two_swapped_draws_are_moved` | two draws swapped | only `moved` |
| `one_ulp_in_a_used_constant` | `MaterialColour.y` + 1 ULP | 1 `constant`, `ps.cb0.MaterialColour[0].y`; a `ulp 1` rule tolerates it |
| `an_unused_constant_is_not_compared` | an unused variable and bytes past the used ones | 0 |
| `signed_zero_and_nan_compare_by_bits` | `+0` → `-0`, a NaN payload | 1 `constant`, 1 `state` |
| `a_blend_state_field` | `SrcBlend` 5 → 4; another state object with the same fields | 7 × `state blend.rt0.src`; 0 |
| `a_swapped_texture` | the other texture bound | 1 `resource ps.srv0` |
| `a_texture_at_another_address_is_the_same` | the texture's id changed | 0 |
| `a_shifted_ring_offset_with_the_same_bytes` | the same ring bytes 96 bytes on (by binding offset, by start); other bytes | 0; 0; 1 `resource vb0` |
| `a_static_buffer_offset_is_compared` | a static buffer's binding offset | 1 `state vb0.offset` |
| `the_scene_part_and_the_allow_list` | a draw outside the scene targets | 1 `extra`; 0 with `--part scene`; allowed by a `[test0000]` rule, not by a `[cdsp0000]` one |

## Reference captures

`crates/ffx-drawdiff/examples/field_capture.rs <windows dir>` captures
field-walk-1's frame. It uses save slot 1 (cdsp0000) on the original code
(`--hook-disable replacements`), seeded, in fast mode, on the virtual
clock, with lockstep, in background mode, through the game-slot queue. As
`tools/replays/field-walk.txt` does, it waits for ctrl work 0's event
count (`0x01325d94`) to enter 64..127, then 37 frames, which pins the
count at 101.

`gfxcapture` waits for two Presents, which lockstep holds back. So the
example sends `lockstep off` and `gfxcapture` in one write. The harness
reads both lines and sets the capture up while the game thread is still
in lockstep's 1 ms hold. All six runs captured at count 101 with 197
draws, and they differ only by the noise below.

`~/ffx-work/m4/t1/capture-pair.sh <worktree> <name>` takes two of them
into `~/ffx-work/m4/captures/<name>-{a,b}`. Captures stay out of git.

| Capture | capture.ffxgfx | Folder | Run |
|---|---|---|---|
| fw1-a, fw1-b | 69.4 MB | 171 MB | 68 s (first launch), 16 s |
| fw2-a, fw2-b, fw3-a, fw3-b | 69.4 MB | 171–174 MB | 14–16 s each, 3.2–4.9 s of it in `gfxcapture` |

drawdiff reads a pair in 0.28 s and compares it in under 10 ms.

## Noise in cdsp0000

Six original captures give 15 pairs. In each part, every pair has 0
unallowed differences with `crates/ffx-drawdiff/allow/cdsp0000.txt`. That
file has three entries:

| Entry | Seen | Cause |
|---|---|---|
| `constant var=scene field=*.cb0.scene.GlobalTime` (251 words: every draw's vs and ps `scene` buffer) | 3 values in 6 runs: 3.4360008, 3.4370003, 3.435999 | `Phyre_PTimerWin32_GetTime` (`0x0042fd80`) returns FILETIME / 1e7, about 1.34e10 s, where an f64 ulp is 1.9e-6 s. The harness's virtual clock starts at the real clock's first reading (`harness.rs` `time_now`), so differences of game times carry that rounding. The 8-ulp f32 step at 3.436 is 1.9e-6 s. The frame limiter's whole-millisecond sleep, taken from such a difference, now and then truncates to one more ms, which gives the 1 ms steps |
| `resource kind=draw vs=bb9bf1c768e1480f field=vb5` (5 draws, 256–1120 B each) | every pair | COLOR1, slot 5, a dynamic ring of the skinned shader `bb9bf1c768e1480f`: the cloth vertices. `Sg_ClothStep` (`0x00822de0`) adds wind from the CRT `rand()`, which keeps its seed per thread. The cloth job (`Ch_Cloth_PerCharacter`, `0x0082e390`) runs on whichever JobWorker thread is free, so each run draws other numbers. `FFX_HARNESS_SEED` seeds the game's generators, not the workers' CRT state |
| `constant var=SgSpec_LightVec field=*.SgSpec_LightVec[0].z` (1 draw, about 5e-30) | every pair | A junk read: `Ch_DrawMain` (`0x0082ed50`) passes `graphicSgSetSpecular` (`0x00643cc0`) a light vector whose third word, `[ebp-0x3c]`, it never writes (`ffx_chara::draw_main`; the port passes 0 under the junk-read policy) |

The first entry would go away if the virtual clock started at a round
value. The second would go away if the harness seeded the workers' CRT
`rand` per job, or made the cloth's wind deterministic. Both are harness
changes, not drawdiff ones.

## Open problems

- **Dynamic index buffers.** The capture streams a new version of a
  dynamic *vertex* buffer before each draw that uses one, but a dynamic
  *index* buffer only as it stands at Present. cdsp0000's index ring
  (1.5 MB) gives each of its 57 draws an offset of its own, so the final
  contents still hold every draw's indices. A ring that wraps within a
  frame would key its earlier draws by later indices (capture.rs, T2).
- **Lockstep capture.** `gfxcapture` blocks the harness connection and
  needs two Presents, so it cannot run in lockstep. The pipelined
  `lockstep off` works, but a lockstep-aware capture would not rely on the
  game thread's sleep (T2's N-frame runs).
- **Stale textures.** The layer's tables can describe a dead texture at a
  reused address (fw3-b, at the back buffer). Target keys avoid it, but a
  sampled texture declared at a stale address would key wrongly.
- **Only vertex and pixel shaders** are in the stream: geometry shader
  bindings are not compared.
