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

A plain `gfxcapture` waits for two Presents, which lockstep holds back. So
the example sends `lockstep off` and `gfxcapture` in one write (the
replays capture in lockstep with `now async`, "Scene gates"). The harness
reads both lines and sets the capture up while the main thread is still
in lockstep's 1 ms hold. All six runs captured at count 101 with 197
draws, and they differ only by the noise below.

`~/ffx-work/m4/t1/capture-pair.sh <worktree> <name>` takes two of them
into `~/ffx-work/m4/captures/<name>-{a,b}`. Captures stay out of git.

| Capture | capture.ffxgfx | Folder | Run |
|---|---|---|---|
| fw1-a, fw1-b | 69.4 MB | 171 MB | 68 s (first launch), 16 s |
| fw2-a, fw2-b, fw3-a, fw3-b | 69.4 MB | 171–174 MB | 14–16 s each, 3.2–4.9 s of it in `gfxcapture` |

drawdiff reads a pair in 0.28 s and compares it in under 10 ms.

### draw-field references ref2-a, ref2-b, ref2-c (2026-10-27)

Since replay-flakes3, `draw-field.txt` loads in lockstep (`load direct 1
cdsp0000 lockstep`) and pins after `lockstep on`, and sound keeps the update
count (`docs/harness.md`, "Sound in lockstep"). The captures of the older
script (ref-a/b/c below) fell on whatever update the free-running load
left, and the blink seeds with them: the BC1/BC3 draw 113 and the
captures 1-2 frames late. `~/ffx-work/m4/captures/ref2-{a,b,c}` are three
original-code captures of the new script, run three at a time: each held
event count 101 (`0x65`) and 459 (`0x1cb`) and the beach's 101, and the
three pairs pass `draw-field-gate.sh` (fw-101 and fw-459 0 differences,
3 allowed each; besaid-101 0 differences, the chocobo icon and its
neighbours allowed). `~/ffx-work/m4/draw-field-gate.sh <run>` gates against
its default reference (`default_ref` in the script, `DRAW_FIELD_REF`);
`draw-field-gate.sh <reference> <run>` against any. A capture only matches
references of the same script: gate new captures against ref2-*.

The scene gates' replays load in lockstep too since then, and pin right
after `lockstep on`; their references are `dg-<replay>-q1/q2` (original
code, run side by side), which pass `scene-gate.sh` against each other on
all five scenes (battle: both loads 77 updates). The p1..p6 folders belong
to the old scripts; `dg-{besaid,kami}-pin64-q*` were taken with the load in
lockstep but the pin still at count 64.

kami0100 on original code (the h3 agent, 2026-10-27: 1 pass in 3 with the
old script, failing on draw 42, a texture-animation bind, `vs 598aa...`):
with the new script five original captures (q1..q5, three of them side by
side) pass against q1, and draw 42 matched in all of them and in the five
pin64 captures. What still differs between original kami0100 runs, all of
it on the scene's allow list: `scene.GlobalTime` by 2 ulp (two clusters),
the cloth ring (`vb5`) and the eye-blink bind of draw 91 (`vs db349612...`,
32x32 or 128x128). draw-field's fw-101 shows the same 2 ulp in two of five
Rust captures, while its three references agree. These come from the
updates the load runs before `pin` (`load direct ... lockstep` steps until
the event script opens, and `pin` follows): pinning at the load's boundary
itself would be the next step. besaid-101 failed once in five Rust
draw-field runs on the back buffer only: the chocobo icon (wall clock, its
draws allowed) drew in none of the three frames (609 draws against
780-784), so the back buffer hashes had no allowed draw before them.

### draw-field references (ref-a, ref-b, ref-c)

`~/ffx-work/m4/captures/ref-{a,b,c}` hold the original-code captures of
`tools/replays/draw-field.txt`: fw-101, fw-459 and besaid-101, three
frames each, hash mode. They were recorded on 2026-10-25 on main plus
m4-pilot (merge f855ea8b, FFX.exe build check passed), with
`FFX_GFX=1 target/release/examples/replay tools/replays/draw-field.txt
'<win dir>\draw-field.trace' original`
(`~/ffx-work/m4/pilot/capture.sh <worktree> <tag> original`). They
replace orig3/orig4 (2026-10-24). Those no longer match an original capture
on the current harness: the camera is off by 8214 ulp and 13 draws are
missing or extra in fw-101, while two fresh original runs agree.

The three references give 0 differences against each other in drawdiff,
and pass `~/ffx-work/m4/draw-field-gate.sh` (hashes) pairwise, with
`~/ffx-work/m4/allow-draw-field.txt`. The update a capture holds at event
count 101 ranges from 361 to 425 across them, because load time varies.
Gate a change against two of them:

```bash
DRAWDIFF=target/release/drawdiff ~/ffx-work/m4/draw-field-gate.sh ref-a <run>
DRAWDIFF=target/release/drawdiff ~/ffx-work/m4/draw-field-gate.sh ref-b <run>
```

To capture the original code plus only some ports, set `FFX_HOOK_ONLY` to
the replacements to keep, or to `@<file>` for a long list (`ffx-dinput8`;
`tools/run-game.sh` passes it on; `docs/hooking.md`).

Known varying parts, beyond the allow list:
- In fw-101, character draw 113 (index count 2928, vs `db349612ecc598d8`,
  ps `87fbd5b2438b7d8a`) binds `ps.srv1` = `fbf8b85b98a2de64` (32x16 BC3,
  `f77`) on a quiet machine, and sometimes `20f96f7d906ae4dd` (32x16 BC1,
  `f71`) for 2 or 3 of the capture's frames. Those are two frames of a
  character texture animation: the run of 2 frames, at a different place
  in the 3 from run to run (frames 1-2, 0-1, 0-2), is an eye blink
  (`chr_texanim_main`, blink mode: wait `60 + rand() % 90`). The original
  code does it too (m4-tex113, 2026-10-26): with no replacements
  installed (`replay ... original`), 1 of 3 captures run three at a time
  bound BC1. Captures run one at a time gave BC3 every time: 9 of 9 with
  every M4 hook (`FFX_HOOK_ONLY`, main 99de8d1f) and 3 of 3 more with all
  but p1-load. The BC1 runs on main (main-m4only-f/g) and in the pilot
  (only3-a/b) were all taken while other games ran. So it follows machine
  load, not the hooks. Each blink wait is drawn from the harness's
  per-batch seed (`seeded_job`: the first character of the
  `Ch_CalcMiddle_Batch` batch and the event frame), so the blinks shift
  whenever load timing changes a batch or the frame a character starts
  animating. Gate draw-field one capture at a time. A BC1 at draw 113 on
  its own is not a port difference; rerun it.
- The post-processing targets after draws 177-180 (fw-101) and 191-194
  (fw-459), and the back buffer, differ between runs. The hash comparison
  excuses them only when a noisy draw comes before them, so an unlucky
  pairing reports them as beyond the noise. If the drawdiff part of the
  same pair is 0, rerun against the other reference.

## Scene gates

One gate per scene (M4 item 3, 2026-10-26, draw-gates): original against
original first, then the ports against the original, each at 0 within the
scene's allow list.

```bash
tools/scene-gate-capture.sh <replay> <run> [original|only-m4|all]
tools/scene-gate.sh <scene> <run A> <run B>
```

| Scene | Replay (`tools/replays/`) | Capture point |
|---|---|---|
| `bsil0000` | `draw-gate-besaid.txt` | Besaid beach (save `ffx_003`), ctrl work 0's count 101 |
| `bsil0300` | `draw-gate-besaid.txt` | the river below the cliff, reached by event-besaid's walk, count 32 |
| `kami0100` | `draw-gate-kami.txt` | the Thunder Plains agency (save `ffx_005`), count 101 |
| `menu` | `draw-gate-deck.txt` | the main menu on the deck (save `ffx_001`), as menu-browse-1 |
| `battle` | `draw-gate-battle.txt` | battle-attack's battle, 20 frames after Rikku's first command window |
| `draw-field` | `draw-field.txt` | fw-101, fw-459, besaid-101 (the draw-field gate above) |

`scene-gate-capture.sh` runs the replay with the graphics layer through
the game slot and copies its captures to
`${FFX_CAPTURES:-~/ffx-work/m4/captures}/<run>/<capture>`. `only-m4` sets
`FFX_HOOK_ONLY` to the M4 port hooks (`ported/{h,p}<n>_*.rs`, `c2_new.rs`,
`m4_rest.rs`, `phyre_render.rs`; the list is kept as `<run>.hooks.txt`);
`all` keeps every hook, and passes a caller's `FFX_HOOK_ONLY` or
`FFX_HOOK_DISABLE` on (bisecting). Each capture is three frames in hash
mode, 3-10 MB.

`scene-gate.sh` runs drawdiff and `tools/compare-captures.py` (the
`hashes.txt` records: target and back buffer contents, held scene inputs)
against `crates/ffx-drawdiff/allow/gate/<scene>.txt`. compare-captures
takes its noise from the same file (`any vs=` drops a shader's draws and
the target hashes after them, `resource vs= field=vb<n>` or `ps.srv<n>`
skips that slot), so each allowance and its cause are written once.
`draw-field-gate.sh` keeps working as before; `scene-gate.sh draw-field`
does the same with the repository's copy of its list
(`allow/gate/draw-field.txt`).

Every entry in a gate list has its cause in the file, and every one but
one was seen between two runs of the original code. Ports never add
entries. The exception is `SgSpec_CameraVec[0].z`, a junk read the policy
of 2026-10-24 sets to 0 (below): it is allowed on bsil0000, kami0100 and
battle as on draw-field (`6ba788e7`).

Captures are armed exactly: `gfxcapture ... now` in lockstep starts after
the present of the update the main thread is held after and checks that
frame `k` shows update `f + 1 + k` (`docs/renderer.md`, "Reference
captures"). The replays `pin` once the map's event runs (count 64), not
right after `lockstep on`: pinned before the load, the original's
`GlobalTime` differed by 16 ulp on bsil0000 and 2 on kami0100; pinned
after it, by 4 and 0.

What the original runs showed, and so what the lists hold:

| Scene | Original pairs | Entries |
|---|---|---|
| bsil0000 | o1/o2 (pin before the load), p1/p2 | `GlobalTime` (4-16 ulp), the `SgSpec_LightVec` junk read, the loader's 16x8 texture (`f1de6dea8bd5f574` srv1) |
| bsil0300 | o1/o2, p1/p2 | `GlobalTime` (47-76 ulp: the walk waits on a real-time voice line), the minimap's NORMAL0 ring, the chocobo icon (o1/o2: 6 draws missing, 6 extra) |
| kami0100 | o1/o2, p1/p2 | the cloth ring (`vb5`, 27 draws, in every pair: carried from the free-running frames before lockstep, inferred), `GlobalTime` (o1/o2, 2 ulp), the eye-blink texture (o1/o2), the `SgSpec_LightVec` junk read (not seen in kami0100's own pairs; the same read differs between original runs on bsil0000 and cdsp0000) |
| menu | p1/p2 | a `NoTexture` quad's UVs (`vb3`; not sampled, stale element words, `docs/formats/ui-draw.md`) |
| battle | p3-p6 (p1/p2 with an older script) | `NoTexture` quads' UVs (`vb3`, p6 against p3; stale caller data, not sampled), the eye-blink texture (not in these pairs; as fw-101's draw 113 in original runs, seen once with a hook subset) |

Results on main a3b34047 plus the draw-gates branch (2026-10-26), against
p1 (bsil0000, bsil0300, kami0100, menu) or p3 (battle), before the
`SgSpec_CameraVec` entry. With it, the three FAILs below pass: their only
difference is that word (`docs/rewrite-plan.md`, M4 status: every scene
passes with only the M4 ports and with every hook on):

| Scene | only-m4 (`FFX_HOOK_ONLY`, 3,511 hooks) | all hooks |
|---|---|---|
| bsil0000 | FAIL: `SgSpec_CameraVec[0].z` only (1 draw, `0x006c0000`) | FAIL: the same word only (`0.0`) |
| bsil0300 | PASS | PASS |
| kami0100 | FAIL: `SgSpec_CameraVec[0].z` only (`0x005d0000`) | FAIL: the same word only (`0.0`) |
| menu | PASS | PASS |
| battle | FAIL: `SgSpec_CameraVec[0].z` only (6 draws, `0x00840000`) | FAIL: the same word only (`0.0`; three runs) |

With every hook on, the battle's `NoTexture` UVs differ from p3 in 11
draws in each of three runs (allowed; p6 differs in 3). A Rust UI that
writes the same element words would remove that entry (ui-draw.md's open
question 1).

Two things the original runs do not pin and the replays avoid:
- a battle requested after the main menu was opened and closed never
  started (the request byte `0x0112a8e2` stayed 1), on the deck and in
  the agency, so the menu and the battle have replays of their own;
- `battle 33 0 3` from the agency loaded in 127 or 128 updates, and the
  later run drew 31 more items; battle-attack's battle loaded in 77
  updates (from the end of `wait 10` to the phase byte) in four original
  runs and in 79 in one, whose intro camera stood elsewhere (17,584 ulp in
  `WorldView`). The first command window opens on the phase byte's update,
  so waiting for it does not help. `scene-gate.sh battle` compares the two
  runs' load lengths (from `replay.out`) first and answers NOT COMPARABLE
  (exit 3) when they differ: rerun one.

`SgSpec_CameraVec[0].z`: `Ch_DrawMain` never writes the word
(`[ebp-0x2c]`), and every original run passes 1 (`0x00000001`, a stale
integer). The port passes 0 (junk-read policy), and with only the M4
hooks the original `Ch_DrawMain` reads what ported functions further up
left there (`0x006c0000` on bsil0000, `0x005d0000` on kami0100). The
draw-field list allowed it first; since `6ba788e7` the bsil0000, kami0100
and battle lists do too, by the same policy. Bisected on
kami0100 with only the M4 hooks (`FFX_HOOK_ONLY` halves of the 1,756
hooks of the second half, 11 steps): `Sg_AccSetFrameChr` (`0x0082be20`,
h4-glue-other) hooked alone makes it `0x00d90000`; other subsets gave
other values (`0x00490000`, `0x000a0000`, `0x005e0000`), so several
ports' frames reach that stack word. Not a port bug: the junk read is
`Ch_DrawMain`'s.

## Noise in cdsp0000

Six original captures (fw1..fw3, a and b) gave 15 pairs, each with 0
unallowed differences against `crates/ffx-drawdiff/allow/cdsp0000.txt`.
That list then had three entries. The harness of 2026-10-24
(harness-main-thread) removed most of the first and all of the second:

| Entry | Seen | Cause |
|---|---|---|
| `constant var=scene field=*.cb0.scene.GlobalTime` (251 words: every draw's vs and ps `scene` buffer) | before: 3 values in 6 runs, 1 ms apart (3.4360008, 3.4370003, 3.435999). Now: 1 to 5 f32 ulp apart (3.4701328 to 3.470134) in 5 of 6 pairs of 4 captures (n1, n2), equal in the sixth | Before: the virtual clock started at `Phyre_PTimerWin32_GetTime`'s first reading (FILETIME / 1e7, about 1.34e10 s, where an f64 ulp is 1.9e-6 s: the 8-ulp f32 steps), and the frame limiter (`Game_Main` `0x0062f2a0`..`0x0062f3bb`) sleeps the time left chopped to whole ms, then 1 ms, so the clock stood 0.3-1 ms past each frame's time in a pattern that depends on how many frames have run (the 1 ms steps: the loads took 380-505 frames). Now the clock starts at 2^20 s and the limiter's `Sleep` moves it to the frame's time exactly (`harness.rs` `VIRTUAL_START`, `fast_sleep`). The few ulp left are likely the game's own f32 time since start (`Phyre_PFramework_PApplication_frame` `0x00627940` keeps the elapsed time and the frame's dt as f32), which rounds differently after another number of frames |
| `resource kind=draw vs=bb9bf1c768e1480f field=vb5` (5 draws, 256–1120 B each) | before: every pair. Now: in no pair of 4 original captures (n1, n2: 6 pairs) or of the 2 Rust ones (r1), and the Rust captures' cloth equals the original's. The entry is gone | COLOR1, slot 5, a dynamic ring of the skinned shader `bb9bf1c768e1480f`: the cloth vertices. `Sg_ClothStep` (`0x00822de0`) adds wind from the CRT `rand()`, which keeps its seed per thread, and the cloth job (`Ch_Cloth_PerCharacter`, `0x0082e390`) runs on whichever JobWorker thread is free. The harness now queues its own cloth job, which seeds the worker's `rand` for the job from the character's place in the array and ctrl work 0's event count at the last frame boundary (`harness.rs` `cloth_job`; `docs/harness.md`, the "Cloth" row). Seeding from the count read inside the job (the main thread counts it on beside the job) still left 2 of 8 captures (made with the user's mods) with other cloth |
| `constant var=SgSpec_LightVec field=*.SgSpec_LightVec[0].z` (1 draw, about 5e-30) | every pair | A junk read: `Ch_DrawMain` (`0x0082ed50`) passes `graphicSgSetSpecular` (`0x00643cc0`) a light vector whose third word, `[ebp-0x3c]`, it never writes (`ffx_chara::draw_main`; the port passes 0 under the junk-read policy) |

The captures since then run without the user's mods (`docs/harness.md`,
"Parallel games"), so they differ from fw1..fw3 in the characters' draws.
`field_capture <dir> rust` captures the Rust replacements' frame. Against
the original's, the two Rust captures (r1) differ only in the junk read
above and in `vs.cb0.SgSpec_CameraVec[0].z` of one draw (`A#0116`, vs
`1e6f10bb647d242c`): `0.0` in Rust, the denormal `1e-45` in the original.

## Open problems

- **Dynamic index buffers.** The capture streams a new version of a
  dynamic *vertex* buffer before each draw that uses one, but a dynamic
  *index* buffer only as it stands at Present. cdsp0000's index ring
  (1.5 MB) gives each of its 57 draws an offset of its own, so the final
  contents still hold every draw's indices. A ring that wraps within a
  frame would key its earlier draws by later indices (capture.rs, T2).
- **Lockstep capture** (solved). A plain `gfxcapture` blocks the harness
  connection and needs two Presents, so it cannot run in lockstep;
  `field_capture` still sends `lockstep off` with it. The replays use
  `gfxcapture <dir> ... now async` and `gfxcapture result` instead, which
  capture in lockstep, armed at the held boundary's update ("Scene
  gates"; `docs/renderer.md`, "Reference captures").
- **Stale textures.** The layer's tables can describe a dead texture at a
  reused address (fw3-b, at the back buffer). Target keys avoid it, but a
  sampled texture declared at a stale address would key wrongly.
- **Only vertex and pixel shaders** are in the stream: geometry shader
  bindings are not compared.
