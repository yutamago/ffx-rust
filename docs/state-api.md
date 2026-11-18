# The game-state API (`ffx-state`)

What a frontend reads of the game after each update: the Godot frontend
(gd-ffx, `docs/godot-plan.md`, milestone G1) draws, animates and plays
from it, so scenes and mods never read game memory. Typed and versioned;
it only reads (the game runs the same with or without it: replays, saves).

- `crates/ffx-state`: the structs (`Snapshot` and its parts), their
  reader (`ffx_state::read`, over a function that reads mapped bytes), the
  encoding and the schema a frontend generates its bindings from.
- `ffx_host::observe`: at each frame boundary (`FFXApplication_animate`'s
  entry, the main thread, between two updates) a snapshot is read and
  handed to the game's observers (`shared.observers.add(f)` gives an id
  for `remove`; per game, called outside the lock); nothing is read while
  a game has none. `observe::snapshot(&Shared)` reads one at any time.

## Requests

The other direction: `ffx_host::requests`. A frontend queues a request
on its game (`shared.requests.push(request, reply)`, any thread); the
game's main thread serves it at the next frame boundary, between two
updates (where the harness serves its jobs; while the harness holds
lockstep, they wait for its release), and calls the reply with what it
did. Replies run on the game's main thread and must not panic; requests
still queued when the game's run ends (`Exec::run_entry` returns) are
answered `Err("the run ended")`, and so is any later request, at once;
so are the requests left in a batch after one stopped the game. A
run without requests is unchanged.

- `Request::DevSave(n)`: loads developer save `n` (1..=110, the saves
  the developers left in the archive: `ffx_host::devsave`, docs/harness.md
  "Developer saves") and starts its map; its `debugLoadForViewer` reads
  from the player's archive without the harness. Save 7 is Besaid beach
  at the save sphere (map 70, scenario 118) with the control.
- `Request::Battle { field, group, formation }`: starts a battle as a
  random encounter does (the harness's `start_battle`: the field's
  record, group and formation into `g_BtlFieldIndex`, `ResetEncountExe(1)`,
  `g_BtlExe`), raised again every 30 updates while the field drops it, and
  answered once `g_BtlStatus` says the battle runs (or after 600 updates
  without it); refused while a battle runs, and a new one answers a
  waiting one with `Err`. Field 3 group 0 formation 0 is the salvage ship's deck
  (`/btlmap/cdsp/cdsp00_a/`); the battle's characters come through the
  same character records as the field's (`actors`).

## Spaces

- **Field space** (the game's logic: positions, the camera channel's
  eye and target): x right, y down, z forward.
- **Render space** (what the renderer draws with; `Camera.world`): the
  field's x and y negated. Right-handed, y up, the camera looking along
  its −z: Godot's convention. A field point (x, y, z) is (−x, −y, z) there.
- `Camera.world` is the scene's camera world matrix as
  `graphicCameraSetWorldMatrix` stores it (`PMatrix4_toWorldMatrix3x4`):
  12 floats, `[Y.x Y.y Y.z X.x | Z.x Z.y Z.z X.y | P.x P.y P.z X.z]`, where
  X, Y, Z are the camera's axes and P its position in render space (the
  camera's world rows 1 to 3, with row 0 spread over their fourth
  column). Measured in the opening at the fire (event frame 959): the
  three axes are orthonormal and right-handed, and P is the camera
  channel's eye (`g_Camera`) with x and y negated. Early in the same event
  (frame 163) the channel reads zeros while the scene's camera is set.
- An actor's `world` is its root matrix (`+0x1d0`): rotation × 0.001 and
  its field position. Its `bones` are the elements' global matrices
  (`+0x88`), relative to the character; the root places them
  (docs/formats/chara.md, "Matrices and elements"). In events the motion
  moves the characters: in the opening the party's records stay at the
  origin and their bones carry them to the fire.

## Menus and messages (API 3)

For a native UI (gd-ffx's G4): `Menus` says which PS2 menu is open
(`g_TkMenuMainActiveMask`, a bit per menu slot: `0x2` the main menu),
the main menu's cursor and option count, the battle command window
(whose it is, its level, that level's cursor and its ids as `u16`s), the
PC save screen's state machine and cursor, and whether the PC (Esc,
Flash) menu is up. `messages` are the event script's message windows
that are not closed (`g_AtelMsgWindows`, the bank `g_AtelMenuExecStatus`
selects): state, place, flags, mode, the choice the player has
highlighted (the text box's record that `0x01868a90`'s table points at,
as `Native_MsgSelect_Poll` reads it), the one the script started it at,
and the text as the game encodes it (a frontend decodes it with ffx-
text's charset for `World.language`, the text's language
(`g_LocalizationManager` `+4`, as `TOGetFFXLang`; 0xff unknown; a fresh
`GameSetting.ini` gives Japanese, 0); since API 11 the macros are
expanded, below). The
battle record's offsets come from the ports (`TOBtlMemoryCursorPos`,
`ffx_system::tolib`), not yet from a run in battle; the title's menu and
the Esc menu's pages (Flash) are not covered.

## The main menu's options (API 4)

`Menus.main_option_list`: the main menu's options (`g_TkMenuMainOptions`,
8 bytes each: the menu id, a `u32` of which the low 16 bits are kept, and
enabled) with their names as the game shows them: `MsMenuGetText(9, id,
kana)` (menu text 9 is the main menu's, `TkMenuMainMes`), reimplemented
with ffx-kernel only (`g_BtlMenuText` rows, `ffx_kernel::bridge::excel_data`;
the English rows are never used on the PC), with the kana bit of the save
config, on the snapshot's reader.

## Effects, screen filters, music (API 5)

`Snapshot.effects`, for G3 (effects as events, Godot's post-processing and
audio): the spell playing (`g_MagicFile_MagicId`), the effect records in
use (`g_EtEff`, 512 x 0x20: the id as `MsGetEffID` builds it, the effect
character, kind, ended flag and targets; no start frame is stored, so a
frontend sees starts by comparing slots between snapshots), the five
screen filter slots (`g_FadeSlots` with `g_FadeActive`'s bits: 0
cross-fade, 1 motion blur, 2 flash, 3 colour filter, 4 fade; mode,
colour and the `+4`..`+0xa` words, whose meaning differs per slot: see
`ScreenFilter`'s fields), the screen's shake offset (`g_Screens[0]`
`+0x30`, `+0x34`) and the music playing (`g_SndNowPlayMusic`). Not yet:
the field's particle effects (`g_FpData`/`g_FpRun`), effect positions,
sound effects (the damage numbers since API 12, below).

## Battlers (API 6)

`Snapshot.battlers`, in battle only (`g_BtlStatus`): the actors taking part
(`+0xdc8` `in_battle`) of `g_BtlChrTop` (0x1f x 0xf90; 0..0x11 the party,
0x14..0x1b the monsters) with HP, MP and their maximums, KO (`+0xdcc`),
shown (`+0xdd2`) and the turn counter (`+0x6ec`): a frontend's battle HUD.
A monster that dies with a death pattern (`+0x3f5`) usually leaves the list
(its `+0xdc8` cleared) rather than showing `dead`
(docs/formats/battle.md, "The battle actor").

## Damage numbers (API 12)

`Effects.numbers`, in battle only: the numbers the battle shows over its
battlers, as the game keeps them, so a frontend shows the game's own
values. `MsNumberRegist` (`0x0079fa00`) puts a number's characters into
the pool `*g_BtlNumbers` (`0x011360b8`: 0x80 records of 0x14 bytes, `+0`
the style, 0 free; `MsNumberPut`) and links them at the end of the list
`g_BtlNumberList` (`0x011360bc`, the next at `+0x10`): its digits lowest
first (`+3` the place, `+4` the digit), then the word (one glyph of the
damage font) at the place after them, each with the colour (`+1`), the
battler (`+2`), a delay of its place plus one (`+5`; a number's delay
is its digit count, plus one with a word), the age (`+6`) and the life
(`+7`, 30). A new number reads age 0 or 1 in the snapshot after its
update, by who put it (effect and motion damage are put after the
update's ageing step, the status step's Regen/Zombie and `chr_damage`'s
HP loss before it); a frontend spots new numbers by comparing snapshots. The reader walks the list (at most 0x80 records,
each inside the pool on a record's boundary) and joins a number's
characters back: the same battler, style, colour, age and life, the
places in a row. `MsNumberCalcProcess` (`0x0079f5e0`) counts each delay
down and each age up once an update and drops a record past its life;
`MsNumberDrawProcess` draws a character once its delay is 0, by the row
`age - 1 - place` of `g_BtlNumberFrames` (`0x00c424b0`: x and y offsets,
alpha), fading over the last 5 updates.

`MsSubHP` (`0x0078e230`) registers the HP loss it is handed before it
clamps the HP at 0, so a killing blow shows its overkill;
`MsDamageCheckDeath` (`0x0078c740`) adds the overkill word (kind 5,
glyph `0x3d`, colour 2) when the monster's overkill HP (`+0xf60`) is
spent. Measured in the deck's
battle (`Battle { 3, 0, 0 }`, Circle): a monster's 34 on Tidus (520 to
486 HP); Tidus's 268 on monster 0x14 with the overkill word, and 108 on
0x15, which had 100 HP (no word: below its threshold); both monsters left
`battlers` in the update of the blow. A new number reads age 0 in the
snapshot after its update (it is put after the update's ages were
counted), its delay its digit count. `Battler.damage` is `+0xd34`
(`damage_hp`), the HP the last action took as `MsCalcDamage` calculated
it, set before the HP changes (seen one snapshot before the loss).

## Effect draws (`ffx_host::vfx`, not in the snapshot)

The geometry the game itself builds for its effects, frame by frame, so a
frontend can draw the game's own effects (docs/formats/effects-draw.md).
Not part of `Snapshot` (an effect-heavy frame is hundreds of kilobytes):
a capture in `ffx-host` with a blob of its own, off unless a frontend
switches it on (`shared.vfx.set_enabled(true)`; off again drops what it
kept).

- **What.** A wrapper on `PhyreScene_DrawMostVFX` (`0x00650670`, the
  `DrawMostVFX` pass: every effect draw of the census, transparent pass,
  scene colour target). Off, it only checks the switch and calls the
  port; on, it first reads the scene's list `+0x38cc`, its draw events
  and, for each entry, the `VFXDynamicGeometry` of `g_VFXRenderDataTable`
  whose mesh instance it is (the map, then the delete pool), and every
  segment with indices: one `vfx::Draw` per indexed draw, in the game's
  draw order (the entry of instance `(0x78, 0x5bc)` last while magic
  0x296 plays, as the pass draws it). The same read takes the scene's
  other VFX lists, which the same updates fill and later passes of the
  same render draw (`vfx::Pass`, below). It only reads memory: replays,
  saves and timing are the same with it on or off. Not captured: entries
  whose mesh instance is in no geometry (`Frame.unresolved`).
- **When.** `DrawMostVFX` runs on the game's main thread between update
  `n` and the boundary of update `n + 1`; `Frame.update` is `n`, the
  `Snapshot.update` of the state it draws. A frontend pairs them with
  `shared.vfx.frame(snapshot.update)` (the newest four frames are kept;
  `latest()` the newest): the frame of update `n` is there before the
  snapshot of update `n` reaches the observers. Measured on Besaid
  (developer save 7, 60 frames): each frame's arrays equal those read at
  the boundary of the same update on every draw (30 of 30; one run of 60 frames, measured with the probe's boundary reads, not a test), and differ
  from the neighbouring boundaries' on 29 of 30 (the sea's strips move
  every update).
- **Space.** `positions` are in render space (Godot's: the field's x and y
  negated, "Spaces" above): the arrays hold the effect's local
  coordinates and the mesh instance's world matrix (`*(mesh instance
  +4)`, 12 floats in `Camera.world`'s layout, kept as `Draw.world`)
  places them (`vfx::to_render`). The field effects' matrices carry the
  field-to-render flip (the sea's: X = (-1, 0, 0), Y = (0, -1, 0),
  Z = (0, 0, 1), P = (0, -1.8, 0)).

`vfx::Draw`: `pass` (below), `place` (the entry in its pass's list; for
the main pass the position draw events count in), `key` (the instance key
`(hi << 32) | lo`: for a ppp program `(g_pppCurrentProgram, object)`, for
the op system `(0, record)`, part `i` at `lo + 10000 i`; effects-draw.md,
"Instance keys and makers"), `geometry` (an address), `flags` (the VSP
flags `+0x3c`, which pick the shader family: `0x8` modulate, `0xa`
additive, `0x18` subtract, `0x9` z-matrix, `0x2` additive untextured;
effects-draw.md, "Shaders"), `pooled` (destroyed this frame, still
drawn), `segment`, `technique` and `blends` (below), `sampler` (below), `world`, `texture` (segment `+0x20` as the game
holds it: `../../../FFX_Data/GameData/PS3Data/<folder>/D3D11/<name>.dds.phyre`,
`NoTexture`, or `FrameBuffer` for draws that sample the frame copy), and
the arrays: `positions`, `colours` (RGBA floats; 0.5 is the PS2's `0x80`,
which the VFX vertex shaders double), `uvs` (garbage on untextured draws:
the game leaves them unset; below), `indices` (`u16`, a triangle list).
The arrays are the ones the game's draw copies: each segment's slot of
the current dynamic-geometry buffer, else its own arrays
(effects-draw.md, "Which arrays the draw reads"; the same bytes wherever
measured). Some vertices a fill reserved but did not write hold colours
(and uvs) no fill writes: `0xcdcdcdcd` (-4.3e8) or another effect's
stale numbers. Only degenerate triangles (a repeated index; the fills
leave such quads' indices 0) use them, and the only exception seen, the
battle's full-screen quads of flags `0x40000000`, writes no colour (write
mask 0), so the game draws no colour from them; a frontend may drop
triangles with a repeated index.

**Texture coordinates** (`Draw.uvs`, `Draw.sampler`; effects-draw.md,
"Texture coordinates"): the shaders sample the texture at the uvs as
they are, with no transform, frame offset or per-draw constant (an
effect's animation frames are in the uvs the fills write). What a
frontend does:

1. **Orientation** (file textures, `.dds.phyre`; not the DTT's
   `FrameBuffer`, a render target whose first row is the screen's top,
   nor the `0x28` family, whose shaders were not decoded: their
   convention is unverified). `v = 0` is the texture's first row *as the
   file stores it*, which is the image's bottom row: the game uploads the
   stored rows without a flip and D3D samples the first row in memory at
   `v = 0`. On an upright image (top row first: `ffx_phyre`'s decoders,
   its DDS export, `ffx-import`'s PNGs) sample `(u, 1 - v)`
   (`vfx::upright_uv`); with the stored rows uploaded as they are
   (`Texture::pixels`), `(u, v)`. Godot's `Image` from such a PNG has
   `(0, 0)` at its top left, so a Godot frontend sets `UV = (u, 1 - v)`.
2. **Addressing and filtering.** `sampler` (`vfx::Sampler`) is the
   `D3D11_SAMPLER_DESC` of the material's `TextureSamplerState` as the
   game makes it: `filter` (`D3D11_FILTER`: `0x15` min, mag and mip
   linear; `0x55` anisotropic, which the game uses for every sampler
   while its anisotropy setting is over 1), `address` U, V, W
   (`vfx::Address`, D3D11's numbers: 1 wrap, 2 mirror, 3 clamp, 4
   border, 5 mirror once; outside 0..1 the uvs need it: Besaid's field
   effects run u 1.0..2.98, v -1.98..0.5 with wrap), `mip_lod_bias`,
   `max_anisotropy`, `border` (bytes, `/255`, for border). The
   comparison function (`LESS_EQUAL` when `filter` has the comparison bit
   `0x80`, else `NEVER`) and the LOD range (`-FLT_MAX .. FLT_MAX`) are
   not encoded: the game sets them so. Mip levels: the
   texture's own chain (as the game uploads it, by texture quality;
   docs/scene-builder.md, "Per draw", the textures). `None` when it could
   not be read (no material or no `TextureSamplerState`): in the probe
   runs exactly the untextured families (`0x0`, `0x2`, `0x3`, `0x10`,
   `0x40000000`), whose shaders sample no texture.

At Besaid (developer save 7) the field effects (`map/bsil/bsil00/fp/tex`:
the sea's strips, the save sphere's orb hemispheres and disc) are `0x15`,
wrap; the sphere's two sparkle quads (`bat_eff`, texture `15456`, uv
block u 0.0039..0.2461, v 0.4385..0.4990, the same every frame) `0x15`,
clamp; read with `1 - v` the block is the one soft glow ball of that
atlas, top-down it shows four other animation frames.

**Blend** (`Draw.technique`, `Draw.blends`; effects-draw.md, "Blend"):
the flags do not decide the blend. The segment's material picks the
effect's technique, `vfx::Technique`: `Transparent`, or
`OptimizedTrans` when the effect code remapped it (the shp and mdl fills
do so while no vertex colour component is above 0.5), or `Other`. The
technique's pixel shaders write the colour differently: for the textured
families `Transparent` writes it premultiplied by the alpha (which can
reach 2.0, the PS2's `0xff`) and blends with source factor one,
`OptimizedTrans` writes it straight for source alpha; the table per
family is in effects-draw.md. `blends` holds one `vfx::Blend` per shader
pass, in draw order: the game draws the segment once per pass (one pass
for every VFX shader seen but the DTT one, `0x1000`, with two). A
`Blend` is the pass's `D3D11_BLEND_DESC` for render target 0 as the
game made its blend state: `enabled`, `src`, `dest`, `op` (colour),
`src_alpha`, `dest_alpha`, `op_alpha`, `write_mask` (bit 0 red to bit 3
alpha); `vfx::BlendFactor` and `vfx::BlendOp` carry D3D11's numbers
(factors 1 zero, 2 one, 5 src alpha, 6 inv src alpha, …; operations 1
add, 2 subtract `src - dest`, 3 reverse subtract `dest - src`, 4 min, 5
max). The blend factor constant is 0, 0, 0, 0 and the scene colour
target `R8G8B8A8_UNORM`: the shader's output is clamped to 0..1 before
blending. `technique` is `None` and `blends` empty when the chain could
not be read (no material yet, a context variant the renderer has not
picked, a value outside D3D11's); none was in the probe runs.

Seen (Besaid and battles; the full table, with `0x28`, `0x80a`, `0x0`,
`0x10` and `0x40000000` (write mask 0), is effects-draw.md's; alpha (one,
zero, add) except the write-mask-0 draws): `0xa`,
`0xb` `Transparent` (one, one, add), `OptimizedTrans` (src alpha, one,
add); `0x8`, `0x9` `Transparent` (one, inv src alpha, add),
`OptimizedTrans` (src alpha, inv src alpha, add); `0x18` either, (one,
one, reverse subtract); `0x2`, `0x3` `Transparent` (one, one, add); DTT
(one, inv src alpha, add) then (one, src alpha, subtract). At Besaid's
save sphere the orb's hemispheres are `Transparent` (one, inv src alpha)
and of the two sparkle quads (flags `0xa`, `bat_eff`) one is
`OptimizedTrans` (src alpha, one) and one `Transparent` (one, one).

**Passes** (`vfx::Pass`; `Frame.draws` holds the main pass's draws first,
then each other pass's in frame order; docs/formats/postfx.md "Order" for
the frame):

| Pass | Number | List (count) | Drawn |
|---|---|---|---|
| `Main` | 0 | `+0x38cc` (`+0xf8cc`) | `PhyreScene_DrawMostVFX`: into the scene colour against the scene depth, after the glow (before it with `isGlowAfterVFX`), the draw events between its draws |
| `Dtt` | 1 | `+0x385c` (`+0x3854`) | `PhyreScene_RenderDTTVFX` (not while the menu capture state `0x01340844` is 2, which skips the post chain; nor captured then), right after the main pass and the DTT copy of the colour (pp `+0xa8`), before the specular pass, the screen fade and the distortion; its `FrameBuffer` draws sample that copy |
| `AfterPost` | 2 | `+0x3888` (`+0x388c`) | `PhyreScene_RenderVFXAfterPostProcess`, after the post-processing (distortion, blurs, AA, brightness), before the UI, with the scene camera |
| `Ui2d` | 3 | `+0x3868` (`+0x3860`) | `PhyreScene_DrawUI2DVFX`, in the UI step before the normal UI, with the scene's second camera (`+0x500`); only while the scene or a movie fades (the capture applies that test and the pass's name skip, so a frame holds them only when drawn); on maps 0x163/0x164 the late step draws it instead, after the post-processing and before the screen fade and `AfterPost`, and the frame holds these draws before the `AfterPost` ones |
| `Undrawn` | 4 | `+0x3880` (`+0x3878`) | no pass found: captured for diagnosis, not to be drawn |

**Draw events** (`vfx::Event`: `kind`, `at`, `words`, `actor`; main pass
only): each runs once the main pass has drawn the entries before place
`at` (before the draws of place `at`; events at or past the last place
run after the last draw). `Event::info()` decodes the words
(effects-draw.md, "Draw events"):

| Kind | `EventInfo` | What a frontend does there |
|---|---|---|
| 0 | `Characters { layer }` | draw the characters of that layer (instance byte `+0x15`) with the scene camera |
| 1 | `CopyBuffer { from, to, rgb, clear }` | copy one buffer into another (`Colour` pp `+0x8c`, `Copy` pp `+0xa8`, `CrossFade` pp `+0x1232c`); with `clear` clear the colour to 0 afterwards; skipped while the motion blur is on and the destination byte is 1 (decoded `Copy`; 3 and more also decode `Copy` and are not skipped) |
| 2 | `ScreenFade { colour, blend }` | draw a full-screen quad of `colour` (RGBA, 1.0 the PS2's `0x80`) over the colour with `blend` (`Alpha`, `Additive`, `Subtract`) |
| 3 | `Ps2Lighting { instance }` | nothing drawn: the PS2 point light of character `actor` is applied |
| 4 | `Specular { instance }` | the specular pass of character `actor` |
| 5 | `DrawCharacter { instance, clip }` | draw character `actor` now, between the effects (shown for the draw); `clip`: the `TransparentClipY` pass |
| 6 | `ZPrePass { instance }` | draw character `actor` into the depth only |

`actor` is the `Snapshot.actors` `slot` of the character whose instance
the event names (kinds 3 to 6; -1 otherwise or when it is no
`g_ChrArray` record).

`Frame.distortion` (`vfx::Distortion`, `None` while it is off): the
screen distortion `Distortion_renderDistortion` draws later in the same
render, after the effects (docs/formats/postfx.md, "Distortion": the
field's under-water and heat ripple). The same read takes it from
`Distortion_GetInstance` (`*0x00cdeb88`); nothing writes it between the
two. To draw it as the game does, keep a screen-sized buffer across
frames:

1. Each frame not `skipped`: `buffer = colour` when `reset`, else
   `buffer = colour * a + buffer * (1 - a)` with `a = copy_alpha()`
   (`min(blur, 0.8)`), `colour` being the frame so far (scene, glow,
   effects).
2. Then replace the colour with the grid: 81 vertices at `positions`,
   triangles `indices`, each pixel `buffer(uv) * colour` (interpolated
   `uvs` and `colours`, blend off; the grid covers the screen).

A `skipped` frame draws neither; the game then resets on the next. While
the broken-screen effect draws the game skips the pass too (not
captured). Fields: `skipped` (`+0x7d`), `reset` (`+0x7e`), `blur`
(`+0x78`), and the grid: `positions` (normalised screen: (0, 0) top
left, (1, 1) bottom right), `uvs` (where each vertex samples the buffer,
the same space: undistorted equal to `positions`), `colours` (RGBA, 1.0
the field's `0x80`; alpha 0 where the ripple is faded out) and `indices`
(`u16`, a triangle list, 384).

**Encoding** (`Frame::encode`, `Frame::decode`; little-endian, version 5;
versions 1 to 4 are refused: version 5 added the draw's sampler (20
bytes a draw with one, 1 without, after the blends; the rest as in version 4), version 4 the
draw's technique and blends, version 3 the event's `actor` and the
draw's `pass`; the events, passes and the distortion part are as in
version 3):

| Bytes | Field |
|---|---|
| 4 | `FXDR` |
| 2, 2 | version (5), 0 |
| 4 | `update` |
| 4 | `unresolved` (every list) |
| 4, 4 | event count `e`, draw count `d` |
| 20 `e` | events: |
| 4, 4, 4, 4 | kind, place `at`, word 0, word 1 (`u32` each) |
| 2, 2 | `actor` (`i16`, -1 none), 0 |
| `d` draws | each: |
| 4 | `place` |
| 8 | `key` (`lo` first) |
| 4, 4 | `geometry`, `flags` |
| 2, 1, 1 | `segment`, `pooled` (0/1), `pass` (0-4; another value is refused) |
| 1, 1 | `technique` (0 none, 1 `Transparent`, 2 `OptimizedTrans`, 3 `Other`), shader pass count `p` (0 with no technique) |
| 8 `p` | each pass's blend: `enabled` (0/1), `src`, `dest`, `op`, `src_alpha`, `dest_alpha`, `op_alpha` (D3D11's numbers), `write_mask`; a value outside the enums is refused |
| 1 | sampler: 0 none (the rest of the sampler absent), 1 present; another value is refused |
| 1, 1, 1 | `address` U, V, W (D3D11's numbers, 1-5; another is refused) |
| 4 | `filter` (`u32`, `D3D11_FILTER`) |
| 4 | `mip_lod_bias` (`f32`) |
| 4 | `max_anisotropy` (`u32`) |
| 4 | `border` (R, G, B, A bytes) |
| 48 | `world` (12 `f32`) |
| 2 + `t` | texture name: byte count and UTF-8 |
| 4, 4 | vertex count `v`, index count `i` |
| 12 `v` | positions (`f32` x, y, z) |
| 16 `v` | colours (`f32` r, g, b, a) |
| 8 `v` | uvs (`f32` u, v) |
| 2 `i` | indices (`u16`) |
| | then the distortion: |
| 4 | flags: bit 0 on (the rest is empty without it), bit 1 `skipped`, bit 2 `reset` |
| 4 | `blur` (`f32`) |
| 4, 4 | vertex count `n` (81), index count `m` (384); 0 and 0 when off |
| 8 `n` | `positions` (`f32` x, y) |
| 8 `n` | `uvs` (`f32` u, v) |
| 16 `n` | `colours` (`f32` r, g, b, a) |
| 2 `m` | `indices` (`u16`) |

`Frame::decode` checks the layout (magic, version, lengths), not the
contents: an index past its vertex count, a draw's or the distortion's,
decodes as it is. The capture never writes one (it refuses the
distortion's indices of 81 or more); a frontend that reads blobs from
elsewhere checks indices before drawing.

**On Besaid** (developer save 7, map 70, `/map/bsil/bsil00/`; the probe
below): 30 draws a frame (25 list entries), no events, none unresolved,
348,080 bytes encoded (version 1; 351,456 in version 2 with the
distortion: on, blur 1, the grid flat, colours (1, 1, 1, 0); version 4
adds 10 bytes a draw, one shader pass each, version 5 20 bytes a draw
with a sampler, 1 without). Textures from `map/bsil/bsil00/fp/tex` (the map's
field effects), `yonishi_data/dat_et/bat_eff/et_tex/tex` (two sparkle
quads) and `NoTexture` (two untextured discs). Entries 0-18: the sea's
foam and wave strips at y -1.8 to -11 (flags 8, 0xa, one 0x18). Entries
19-24 sit on the save sphere `f001` (render (309.7, 8.7, -476.9)): two
hemispheres of 128 vertices (flags 8, centred 6.2 above it, radius about
2.5), two sparkle quads (0xa), a textured disc (8, two segments) and an
untextured disc (2, two segments) at 2.4 and 2.5 above it. Their keys'
low words are 6, 8 and 9 and their high words addresses in `0x00d29c68`
.. `0x00d2c118`: ppp objects of the map's field programs. Which entry of the
map's field-program table draws a frame's entry follows from the objects'
owners (effects-draw.md, "Instance keys and makers"): the sea's strips are
entries 0 to 13, the save sphere's six entries (8 draws) entry 15 of
`bsil00`'s table (gd-ffx R7a: all 30 draws resolve to 15 entries).

The distortion moves on developer saves 4 (`cdsp07`), 29 (`lchb08`) and
76 (`stbv00`): samples up to 0.0076 of the screen off their vertices
(postfx.md, "Where it is on"); the probe prints it per frame, with the
ripple's parameters.

**In battle** (battle field 34, `kami00_00`; effects-draw.md, "The other
lists live"): the battle's fade-in is a kind 2 event on its first 46
frames (black, alpha 1.0, then down by 1/16 a frame); DTT draws
(`FrameBuffer`, flags `0x1000`) on most frames; Valefor's overdrive adds
kinds 3 and 4 naming his character. No after-post, UI 2D or `+0x3880`
draw and no event of kinds 0, 1, 5 or 6 was reached (effects-draw.md,
open question 4).

```text
FFX_GAME_DIR=~/steam-games/FFX-X2-HD cargo run --release -p ffx-host \
  --example vfx-probe -- 7 20      # developer save, frames
FFX_GAME_DIR=~/steam-games/FFX-X2-HD cargo run --release -p ffx-host \
  --example vfx-probe -- 12 6000 /tmp/vfx-probe --party 01000405ff020306 \
  --battle 34,0,0 --drive --summary   # a battle played: the summon, the overdrive
```

## Cloth (`ffx_host::cloth`, not in the snapshot)

The characters' simulated hair and cloth as the game draws them, frame by
frame, so a frontend can move them (the snapshot's `bones` are the
elements' matrices only: cloth is a vertex simulation). Like the effect
draws, a capture in `ffx-host` with a blob of its own, off unless a
frontend switches it on (`shared.cloth.set_enabled(true)`; off again
drops what it kept).

- **What the game does** (docs/formats/chara.md, "Cloth"; docs/host.md,
  "The cloth race"). `Ch_DrawMain` queues one `Ch_Cloth_PerCharacter`
  job per cloth character; `Sg_ClothStep` (`0x00822de0`) moves the PS2
  model's cloth points, then `Sg_ClothPlaceAnchors` (`0x0082dbc0`) puts
  every vertex of each HD cloth segment on the PS2 triangle
  `Ch_PreCalcActiveClothMesh` bound it to (the weighted corners, lifted
  along the normal for anchors) and writes it into the segment's cloth
  object (`+0x4c`, 16 bytes a vertex, `+0x44` of them, in the segment's
  vertex order). `Sg_MainLoop` waits for the jobs at the end of the
  update. After the update `FFXApplication_render` (`0x0042f930`, main
  thread) calls `graphicCharacterInstanceSetClothVertexBuffers`
  (`0x0063b740`): `batchSetClothVertexBuffers` hands each cloth segment's
  draw those vertices (its `COLOR1` stream) and the object's first 16
  floats as the shader's `ClothWorld`. The character shaders' cloth path
  mixes the skinned position `s` and the cloth one `c = ClothWorld(v)`:
  `mix(s, c, min(|v|² · 10000, 1))` with `v` the vertex's x, y, z. A vertex
  the simulation leaves at zero (the skin-coloured ones, vertex colour
  <= 0.3, which `Ch_PreCalcActiveClothMesh` never binds; not the
  `Ch_UseAnchorVertex` anchors above, which move and weigh 1) stays
  skinned. The reserved `u16` after the version is 0; `decode` refuses
  another value.
- **What.** A wrapper on `graphicCharacterInstanceSetClothVertexBuffers`.
  Off, it only checks the switch and calls the port; on, it first reads
  what the call is about to hand over, with its conditions: the cloth
  table's slots (`0x00ccb254`, count `0x00ccb250`; one cloth record each,
  a character's `+0x834`) whose record is ready (instance list `+0`
  set, hide word `+8` 0, state `+0xc` 2) and that the call submits (it
  submits in batches of `EighthAtLeastOne(slot count)` ready slots and
  drops a short last batch, which can happen only with 16 slots or more),
  each instance `k` of the record (`+0x14` of them) with a cloth table
  (`+0x30 + 4k`), each segment of the instance's mesh whose
  `PMeshSegment` `+0x20`/`+0x24` (the PS2 model and piece) are not
  `0xffff` and that has a cloth object. It only reads memory: replays,
  saves and timing are the same with it on or off.
- **When.** Between update `n` and the boundary of update `n + 1`, after
  the cloth jobs (no race with them) and before `graphicRender` copies
  the buffers, in the same `FFXApplication_render` as the effect draws:
  `cloth::Frame.update` is `n`, the `Snapshot.update` of the state it
  draws; `shared.cloth.frame(snapshot.update)` pairs them (the newest
  four kept; `latest()` the newest). No frame is captured when the call
  does not run (utility mode, the trophy error, the sphere grid's state
  below 4). The renderer copies a segment's vertices only when the render
  thread has staged the previous ones; natively it nearly always has, and
  when it has not the game draws the previous update's vertices for that
  segment (docs/host.md, "The cloth race"): the capture holds what the
  call hands over.
- **Space.** `positions` are in render space (Godot's: the field's x and
  y negated, "Spaces" above), the space of `Camera.world` and the effect
  draws: `ClothWorld` (`Ch_UpdateClothMatrix` `0x008314c0`: the segment
  model's base element with its translation × 1000, the character's root
  `world`, then x and y negated) applied to the vertex's x, y, z as the
  game's matrices apply, to a row vector: `p = x · row 0 + y · row 1 + z
  · row 2 + row 3` of the 16 floats in memory order (`cloth::to_render`).
  `local` keeps the vertices as uploaded and `world` the matrix.

`cloth::Character`: `actor` (the `Snapshot.actors` `slot` whose `+0x834`
the record is; -1 when no `g_ChrArray` record holds it), `id` (the
character's id), `table_slot`, and `segments`. `cloth::Segment`:
`instance` (the instance's place in the record's list: 0 for the body,
more for a character's other instances), `segment` (its index in the
instance's mesh), `model` and `piece` (the segment's `+0x20` and `+0x24`
as the segment holds them: what an importer reads from the HD mesh to
know which segments are cloth; `Sg_ClothPlaceAnchors` takes the PS2
sub-mesh as `piece % 100`), `segment_vertices` (its first `PDataBlock`'s
`m_elementCount`; equal to the vertex count on every segment measured),
`world` (`ClothWorld`, 16 floats, memory order), and per vertex in the
segment's vertex order: `local` (the 4 floats uploaded; the fourth is
not read for the position), `positions` (render space) and `weights`
(`min(|local.xyz|² · 10000, 1)`).

A frontend draws a cloth segment's vertex `k` at `mix(skinned_k,
positions[k], weights[k])`, with `skinned_k` its skinned position in
render space: in practice it replaces the positions of the vertices with
weight 1 and keeps the others skinned. A character that is drawn but
missing from a frame was not handed over (hidden, not ready, or the
segment has no cloth object): its cloth follows the skin, as the game
would draw it without new vertices.

**Measured** (`cloth-probe`, release, null device): developer save 7
(Besaid at the save sphere): Tidus (actor 0, `c001`) only, five segments
(model 0 pieces 0, 1, 2 and model 1 pieces 1, 2; 68, 28, 22, 16 and 64
vertices, 104 simulated), their centres 5 to 14 units from his render
point, every
simulated vertex moving relative to him on each frame (up to 0.014 units
a frame standing still); the blob 6,760 bytes. Developer save 29: an NPC
`c105` with two segments (165 and 16 vertices), up to 0.46 units a
frame. Developer save 12 with the party `01000405ff020306` in battle 34
(`--battle 34,0,0 --skip 300`): Tidus, Yuna (`c002`, 17 segments, pieces
0 to 313), Wakka (`c005`, 4 segments, 568 vertices) and the weapon
`w012` (actor 5, instance 1, 168 vertices): 1,200 of 1,840 vertices a
frame simulated, all moving; the blob 61,088 bytes. Read with the matrix the
other way round (column vectors) the characters' cloth lands 23 to 2,400
units off them.

```text
FFX_GAME_DIR=~/steam-games/FFX-X2-HD cargo run --release -p ffx-host \
  --example cloth-probe -- 7 20      # developer save, frames
FFX_GAME_DIR=~/steam-games/FFX-X2-HD cargo run --release -p ffx-host \
  --example cloth-probe -- 12 6 /tmp/cloth-probe --party 01000405ff020306 \
  --battle 34,0,0 --skip 300         # Tidus, Yuna, Wakka in battle
```

**Encoding** (`cloth::Frame::encode`, `decode`; version 1), little
endian:

| Bytes | What |
|---|---|
| 4 | `CLTH` |
| 2, 2 | version (1), 0 |
| 4, 4 | `update`, character count |
| per character: 2, 2, 2, 2 | `actor` (`i16`), `id`, `table_slot`, segment count |
| per segment: 2, 2, 2, 2 | `instance`, `segment`, `model`, `piece` |
| 4, 4 | `segment_vertices`, vertex count `n` |
| 64 | `world` (16 `f32`, memory order) |
| 16 n | `local` (4 `f32` a vertex) |
| 12 n | `positions` (3 `f32`, render space) |
| 4 n | `weights` (`f32`) |

`decode` refuses another magic or version, bytes cut short and bytes
left over.

## Encoding

`Snapshot::encode`: `FFXS`, the API version (`u32`), then the fields in
their declared order. Little-endian; `bool` one byte; `string` a `u16`
byte count and UTF-8; `bytes` a `u16` count and the bytes; `rgb` 3 bytes; `list<T>` a `u16` count and the items; `vec3` 3
floats, `mat3x4` 12, `mat4` 16 (row-major as the game keeps them).

## Look (API 7)

`look` holds what the renderer colours the scene with, so a frontend can
draw the game's own shader result (docs/formats/render.md, "Scenery light
levels", "Lights and fog at run time"):

- `colour_scale`: `PhyreScene +0xfc78`, the `clrScaleFactor` that
  `PhyreScene_setColorScale` last gave the map's materials (the tree
  `+0x1198`; nothing while `+0x10d4` is 0). The map shaders
  multiply by it just before the tone curve `2.1 min(c, 0.4) + 1.08
  clamp(c, 0.4, 0.58) - 0.47`.
- `background`: the scene target's clear colour (context `+0x30`, the
  map's `GlobalCamera` `m_bgColor`), not tone-curved; the post passes still
  apply.
- `glow_full`, `glow_none`, `glow_gain`, `glow_spread` (API 9): the glow
  pass's thresholds, gain and spread (context `+0x80..+0x8c`,
  docs/formats/postfx.md "Glow"); 1, 1, 0, 0 without a `GlobalCamera` and
  on map 0x199 (0x12ea and 0x1733 while `getScenerioFlag` is 0).
- `ambient`, the fog (`fog_colour`, `fog_near`, `fog_far`, `fog_curve`,
  `fog_limit`): the scene context (`*(scene +0xcc)`) `+0x00`, `+0x10`..
  `+0x2c`.
- `character_ambient` and `lights`: `g_CharacterLighting`'s
  `CharacterAmbientLight` and its three `PLight`s (direction, colour,
  level, the map's pitch and yaw). `PhyreChrLitShader` lights with
  `ambient + character_ambient + CharacterShadeColor x
  CharacterShadeCoeff + sum colour x level x (dot(L, N) x 0.5 + 0.5)^2`;
  the shade term is not in the snapshot.

## Save (API 8)

`save` holds the parts of the game state block (`g_SaveData`
`0x0112ca90`, docs/formats/save.md) that the menus and the save screens
show: gil, the play time, the front and back lines, the 18 characters
(`ply_save`: name, joined, HP/MP, the 8 stats, AP, sphere levels, the
limit gauge, weapon and armour slots, the commands learnt, the overdrive
mode and modes), the inventory's used slots, the key-item bits and the
commands everybody learnt (from `0x3060`). Names are in the game's text
encoding (a frontend decodes
them with the game's character set, as the messages). The battle's party
index is the member's `index`, so a battle HUD names its battlers from
here.

## The menus' windows and current character (API 8)

`menus.windows`: the PS2 menus' windows in use (`g_TkMenuWindows`, the
records with `+0x40` set): state, rows, the first row shown, the cursor
(the row is `cursor >> 1` in two-column lists), an item window's list kind
(`+0x43`) and the opener's five words (kind, character, index, ...).
`menus.current_player`: the character the menus show
(`TkMenuGetCurrentPlayer`): below 8 a party member, 8 and up an aeon, 0xff
when the position is past the 8-byte list.

## Objects (API 8)

`objects` is the map's object tree as the scene keeps it (`PhyreScene`
`+0x1198`): per object id (the number in its meshes' names, `N_kShape`)
its meshes' live world matrices (3x4, the camera's packing, render space)
and whether each is drawn (the layer mask). Scripts move objects
(`graphicObjectSetTranslation`/`Rotation` rewrite the matrices) and show
the meshes of a group (the mask), e.g. Besaid's birds (objects 18..32,
placed at the origin in the file). The id is each mesh instance's `+0x58`
(`obj_id`); observed at Besaid, it is the number in the meshes' names
(`N_kShape`).

## Sphere grid (API 8)

`sphere_grid` is the sphere grid screen's data while the screen runs
(`g_AbmapRunning`)
(`*g_Abmap`, docs/formats/pc-ui.md): the character shown, the node under
the cursor, the nodes (place,
kind, flags) and the links between them. The standard grid has 828 nodes
and 834 links. The node's `+0x20` flags (a bit per character from bit 8)
are read as who activated it; that meaning is observed, not checked in the
code.

## Drawn characters, model parts, display objects, 2D layers, config, battle results (API 10)

- `look.global_time`: the shaders' `GlobalTime` (the scene context
  `+0xd0`, `*g_PhyreApplication +0x120`), the clock of the scrolling
  UVs (`phyreuvanimationshader`: UV + `fract(ScrollSpeed x GlobalTime)`,
  e.g. Besaid's clouds) and the water. Not the game time: it runs while
  the game runs (not loading, not paused), wraps to 0 at 2400 s and is 0
  again after every map change (docs/harness.md, "GlobalTime and the
  title band").
- `Actor.drawn`: the game draws the character, as the last `Ch_DrawMain`
  left it (read at the next frame boundary). `Ch_DrawMain`
  decides it after the hide byte (`hidden`), `flags2` bit `0x20`
  (`F2_NO_OEF2`), the distance cull (`Ch_CullByDistance`, with `flags` bit
  `0x800`), the alpha and a few per-id and per-spell cases, and hands it
  to `graphicCharacterInstanceSetVisible`, which sets the render
  instance's (`+0x830`) byte `+0x14`; that byte is read here. `hidden ==
  0 && alpha > 0` is not enough: a character can be culled or skipped
  (`flags2` bit `0x20`) while both hold. `flags` bit `0x2000`
  (`F_DRAW_EFFECT`) is not a hide bit: it hides the character only under
  magic `0x280`/`0x245`. `flags` and
  `flags2` are the raw words `+0x194` and `+0x198`.
- `Actor.models`: per model (part) its base element and whether the
  header's skin list names it. `Ch_DrawMain`'s walk 1 gives a skinned
  model the root's world matrix (`+0x1d0` x 1000, the `world` here) and an
  unskinned one that times its base element's matrix (`bones[base]`),
  `graphicCharacterInstanceSetWorldMatrixByMdlID`; the save sphere
  `f001`'s three parts are unskinned.
- `SceneObject.dobj`, `morph`, `morph_v`: an object id below
  `g_YiDObjCount` is a display object's (`g_YiDObjs`, `0xb4` bytes each;
  `yiRender_draw` moves object `i` with `graphicObjectSetTranslation(i,
  ..)`). Its `+0x88` is the morph value a script sets
  (`yiSetMIMeParamDObj`, then `graphicSetMeshVertexColorMorphValue`;
  the morph weights are `1 - w`, `w`) or, with `+0x8c`, the UV offset a
  keyframe track of kind 2 sets (`graphicSetMaterialUV_006436e0`).
- `layers`: the 2D layers as the scene draws them
  (docs/formats/scene-maps.md, "How the game draws the 2D layers";
  render.md): the 2D level loaded (`+0x93c`) and the pass switched off
  (`+0x93d`), whether the pass runs, whether it runs before the 3D world
  (`+0xd4`), the 2D scene shown (`+0x11d8`), the window's pan `b`
  (`+0xf9bc`) and shift (`+0xfa34`, `+0xfa38`), whether the window is
  squeezed to 16:9 (not while `+0xf9c7` or `g_PhyreScene3D` is set), and
  `PhyreScene_show2DLayer`'s table (shown, scene, layer id; a layer not
  listed is shown).
- `menus.config`: while the config menu runs (`main_mask` bit `0x400`),
  its cursor, the PC's own row and the rows: title, value, enabled and
  options, the texts as `MsMenuGetText(7, id, kana)` (`TkMenuConfigMes`)
  gives them in the game's encoding. The PC row's texts come from the
  localisation manager (`SaveDataGetLoc`): its ids are given (title
  `0x1cd`, option `k` `0x1d0 - 2 k`), its texts are empty.
- `menus.battle_end`: while the battle results run (`main_mask` bit
  `0x20000`, slot 0x11), the screen's state, the gil won and still to
  count, the AP each character still has to count (18 words), the rows
  (character, kind, slide timer, front line, colour, level-up flash), the
  items and gear won, and the spoils pages' cursors.

## Choices, variables, battle targets, the camera's adjustment, loads, boosters, fades (API 11)

For gd-ffx's second playtest round.

- **Choices** (`Message.choice`, `picked`): `choice` is the option the
  cursor is on now, counted from the first option: the window's text box
  record (`0x01868a90[w]`, `TOFuncPadMessageSelect` `0x008b64e0`) `i8 +2`
  less its first row `i8 +4` (`TOLayoutMessageChoices` clears `+4`), -1
  when the record lays out no rows (`+5` not above `+4`). The pad moves
  `+2` while the choice is open, and the confirm leaves it on the pick.
  Before API 11 `choice` was the confirmed pick, so the cursor seemed not
  to move until Circle. `picked` is now the confirmed pick: `+0x18`, what
  `Native_MsgSelect_Poll` hands the script, once `+1` is 2 (all picks
  made, `+7`), else -1.
- **Variables** (`Message.text`, `vars`): the text's macros are expanded
  as the game draws them (`TOGetChangeMacroAddress` `0x008b6660`,
  recursively, as `TODrawChangeMacro2`): `{VAR:n}` (code `0x12`) from the
  window's records (`g_MsgVars` `0x01869ab0`, 32 a window, the kind at
  `+1`, the value `i32 +4`), the characters' names (`0x13`), item texts
  (`0x16`, `0x17`) and key items (`0x23`). By the variable's kind: 0 a text
  at the value; 1 and 2 the value in decimal (2 with the digit-group mark
  `0x46`, `TOFuncMacroCmdValue`); `0x13`..`0x23` a macro code (`value /
  0xd0` added to it, `value % 0xd0` its argument); `0x2b`..`0x32` the value
  in `kind - 0x2a` digits (`TOIntToMes` mode 0, padded as the language
  pads); `0x33`..`0x36` an item's, a weapon's (`MsGetSaveWeaponName`), a
  command's (`MsGetComData`) or an ability's name; `0x37` the field pickup
  (`msFieldItemGet`'s `takara` record by the value: its kind, id and count
  in `pickup`, `pickup_id`, `pickup_count`; an item's or key item's name,
  the gear's from the spoils `MsFieldItemGetMes` remade at the last draw,
  and for gil nothing, since `TOIntToMes(gil, out, 0, 0)` keeps no digit),
  so "Obtained {VAR:0}!" reads with the item's name. The reader builds the
  bytes itself, never writing the game's buffers or calling its code. The
  dictionary codes (`0x19`..`0x22`) and anything not mapped stay as codes.
  `vars` keeps each record that is set, its expansion with it.
- **Battle targets** (`Menus.battle_target`): while the command window's
  level is a target selection (its record's kind, byte `+2` of
  `g_BtlWindowRecords + 0xf0 g_BtlActionDepth`, is 1), the character the
  cursor is on (`g_BtlTargetChr` `0x0133d1f4`), the selected bits
  (`g_BtlTargetSelMask` `0x0133d178`; `TOBtlDrawTargetCursor` puts a cursor
  over each bit below 28), the cursor's index in the side's list
  (`0x0133d1fc`), the list's length (`0x0133d1f6`), the side
  (`0x0133d200`: 1 the monsters', 0 the party's) and whether it can be
  swapped (`0x0133d1fe`). The command lists' hand cursors are
  `g_BtlCurPos` (`0x0133f7b4`, `TOBtlSetCurPos(kind, x, y)` on the PS2
  screen); the draw clears their count after drawing them, so the records
  are given, zeroed ones left out.
- **The camera's adjustment** (`Camera.shift_x`, `shift_y`, `pan`,
  `map_pan`, `squeezed`, `scene_3d`, `offset_y`). The game's
  view-projection (`PhyreScene_getViewProjectionMatrix` `0x0065e660`,
  render.md, round 24) is the camera's own 4x4 (`*(scene +0xdc) +0x74`,
  made from `world`, `fov_y`, `aspect` and its planes) times the
  translation by the shift (`+0xfa34`, `+0xfa38`), then,
  when `squeezed` (neither `+0xf9c7` nor `g_PhyreScene3D`), times
  `PhyreScene_GetScreenAdjustedView`'s (`0x00651650`) `[1 0 0 0; 0 4/3 0
  0; 0 0 1 0; 0 d 0 1]` (row vectors), `d = 0.33333337 - 2 (+0xf9c0 +
  +0xf9bc)` = `offset_y`. In clip space, with `(x, y, z, w)` the camera's
  projection of a point:
  `x' = x + shift_x w`, `y' = (y + shift_y w) 4/3 + offset_y w` (squeezed;
  else `y + shift_y w`), `z`, `w` unchanged. `aspect` is the camera's own:
  16/9 on 3D maps (`PhyreScene_setCameraAspectRatio` makes the PS2's 4/3
  16/9 there), 4/3 in the 2D-background rooms (`scene_3d` clear), where the
  PS2 picture is stretched by 4/3 across the 16:9 target and the squeeze
  picks the 16:9 window over it; the game's last copy fits the result into
  the window at 16:9 with bars (`PostProcessManager_setNon16By9Viewport`
  `0x00698750`). `pan` is the window's pan `b` (the layer pass uses the
  same window without `map_pan`, `Layers`); `map_pan` (`+0xf9c0`) is the
  per-map term `PhyreScene_fieldMapInitialize` sets. Measured: Besaid
  (`bsil00`, 3D) 16/9, not squeezed, pan 0.035; the S.S. Liki's corridor
  (`slik04`, 2D) 4/3, squeezed, pan 0.295, `map_pan` 0, `offset_y`
  -0.25666666, `shift_y` swaying by a few thousandths from update to update
  (the ship).
- **Loads and map changes** (`World.scene_loading`, `map_loading`,
  `characters_loading`, `assets_loading`, `map_change`):
  `scene_loading` is `Sg_IsSceneLoading` (`0x0081fcd0`), the test
  `Sg_MainLoop` holds the update on: the field map loads (`PhyreScene
  +0x1195`) outside a battle map (`+0x1196`, `+0x1197`), characters load
  or instantiate (`g_GraphicCharacterLoading`) on one of the 16 maps
  `graphicIsMapInList` names outside a battle map, or the asset loader has
  requests left (`g_AsyncLoadManager +0x20`, `+0x2c`) while the screen
  fade's colour is locked. The parts are given too: a frontend can hold
  its picture while any of them runs. `map_change` is
  `g_TkEventJumpRequest` (`0x0133084c`): `TkSetEventJump` (the scripts'
  map changes, `0x0011`, `0x00ab`, `0x010c`, through `AtelSetEventJump`/`2`)
  writes the target map and entrance into `g_SaveData` and sets it;
  `TkInitScene` clears it when the new map starts. While it is set,
  `World.map` and `entrance` are the target and `level` still the map
  shown. On the virtual clock a map loads within one update (the reads
  wait in `CdReadSync`), so only the request is seen between two updates;
  on the real clock the asset loader runs on its own and the loading
  fields show it.
- **Boosters** (`Snapshot.boosters`): F1's speed level (`g_SpdCtrl_b4`)
  and whether it scaled the last update (`g_SpdCtrl_b8`) with its scale
  (`g_SpdCtrlScales`), F2's power-up (`g_BtlCheatPowerup`), F3's encounter
  mode (`g_EncountMode`: 0 none, 1 normal, 2 ten times) and F4's
  auto-battle (`0x0133d6e0`). A new game: 0, false, 1, false, 1, false.
- **Fades** (`ScreenFilter.drawn`, `alpha`, `draw_colour`, `subtract`):
  what `Sg_DrawFilter` (`0x0082c0c0`) drew for each fade slot in the update
  just run, by its integer formulas, from the words it left (it runs after
  the update, steps `frames` down and holds a fade to black or white at
  `frames` 1): the formulas are in `ScreenFilter`'s fields. A held fade to
  black draws 1 (not `1 - 1/level`); the frame of its last step reads as
  held too (it leaves the same words). Measured at the beach's fade-in
  (mode 2, total 15): 0.929 at `frames` 14, then 0.866, 0.795 .. 0.063 at
  1 and 0 at 0; the held fade-out before it (mode 3, `frames` 1, total 1)
  1.0.

## The free camera, the numbers' frames, the weapons' names (API 13)

For gd-ffx, which read these from game memory until now.

- **The free camera** (`Snapshot.free_camera`): the PC build's fly camera,
  the singleton `g_FreeCamera` (`0x00ccc830`, 0x340 bytes, made on first
  use; docs/formats/field.md, "Free camera"). `free` is its word `+0` (the
  capture counter `FreeCamera_handleInput` toggles on bit 0x40000, tips
  0x2ed/0x2ee), `time_stop` its byte `+0x338` (`FreeCamera_setFreePause`
  on bit 0x100000, tips 0x2f3/0x2f4), `pause_freeze` its byte `+0x33c`
  (`FreeCamera_setFreeCameraMode`: `Pause_Enter` sets it, `Pause_Leave`
  clears it). While `time_stop` or `pause_freeze` is set,
  `graphicGetFreeCameraType` answers 2 and the world stops. The game asks
  its icon panel for icons 8 and 9 for the first two, which it refuses
  (`IconPanel_showIcon` takes 0..6), so a frontend shows them itself. Bits
  0x40000 and 0x100000 have no settings word in `Input_BuildPadMap`
  (pairs 30 and 32 are pushed with 0): no key reaches them in the PC
  build.
- **The damage numbers' frames** (`Effects.number_frames`, while `numbers`
  is not empty): `g_BtlNumberFrames` (`0x00c424b0`, 32 rows of 3 bytes up
  to `g_BtlShoutStreams` at `0x00c42510`), the x and y offsets (`i8`,
  widened) and alpha `MsNumberDrawProcess` draws a number's character by
  (row `age - 1 - place`). Read live: x always 0; y 0, 3, 6, 8, 10, 11,
  12, 11, 10, 8, 6, 3, 0, then a small second bounce (2, 3, 2, 0, 1) and 0
  from row 19 on; alpha 32, 60, 84, 104, 120, then 128 (opaque).
- **The weapons' names** (`PartyMember.weapon_gear`, the members who
  joined and have a weapon): the `g_Equipment` record of slot `weapon`
  as `MsGetSaveWeapon` finds it, with its `w_name` record, character, the
  config's kana bit and `MsGetSaveWeaponName`'s text, as
  `BattleEnd.gear_spoils` gives the gear won.

## Schema

`cargo run -p ffx-state --example schema` prints it; the test
`the_doc_lists_the_schema` keeps this block equal to it. Every change of
the fields raises `API_VERSION`.

```text
ffx-state API 13
struct Snapshot
  update: u32
  game_time: f64
  world: World
  camera: Camera
  input: Input
  actors: list<Actor>
  menus: Menus
  messages: list<Message>
  effects: Effects
  battlers: list<Battler>
  look: Look
  save: Save
  objects: list<SceneObject>
  sphere_grid: SphereGrid
  layers: Layers
  boosters: Boosters
  free_camera: FreeCamera
struct World
  map: u16
  previous_map: u16
  entrance: u8
  story: u16
  event: string
  level: string
  event_frame: u32
  battle: bool
  player_control: bool
  paused: bool
  fading: bool
  language: u8
  scene_loading: bool
  map_loading: bool
  characters_loading: u32
  assets_loading: bool
  map_change: bool
struct Camera
  valid: bool
  world: mat3x4
  fov_y: f32
  aspect: f32
  far: f32
  eye: vec3
  target: vec3
  shift_x: f32
  shift_y: f32
  pan: f32
  map_pan: f32
  squeezed: bool
  scene_3d: bool
  offset_y: f32
struct Input
  held: u32
  pressed: u16
  stick_x: u8
  stick_y: u8
struct Actor
  slot: u16
  id: u16
  kind: u8
  model: string
  position: vec3
  yaw: f32
  scale: vec3
  hidden: u8
  alpha: f32
  motion: u32
  motion_frame: f32
  motion_speed: f32
  parent: i16
  parent_bone: u16
  player: bool
  world: mat4
  bones: list<Bone>
  drawn: bool
  flags: u32
  flags2: u32
  models: list<ActorModel>
struct Bone
  matrix: mat4
struct ActorModel
  base: u16
  skinned: bool
struct Menus
  main_mask: u32
  main_cursor: i32
  main_options: i32
  main_option_list: list<MenuOption>
  battle_window: u32
  battle_character: i16
  battle_depth: i16
  battle_cursor: i16
  battle_ids: bytes
  save_state: u32
  save_cursor: i32
  save_top: i32
  save_rows: i32
  save_files: i32
  esc_menu: bool
  windows: list<MenuWindow>
  current_player: u8
  config: ConfigMenu
  battle_end: BattleEnd
  battle_target: BattleTarget
struct MenuOption
  id: u16
  enabled: bool
  name: bytes
struct Message
  window: u8
  state: u16
  x: i16
  y: i16
  flags: u16
  mode: u8
  choice: i16
  initial_choice: i16
  text: bytes
  picked: i16
  vars: list<MessageVar>
struct Effects
  magic: i32
  running: list<EffectInstance>
  filters: list<ScreenFilter>
  shake_x: f32
  shake_y: f32
  music: i32
  numbers: list<BattleNumber>
  number_frames: list<NumberFrame>
struct EffectInstance
  slot: u16
  id: u32
  character: u8
  kind: u8
  ended: bool
  targets: bytes
struct ScreenFilter
  slot: u8
  active: bool
  mode: u8
  color: rgb
  frames: i16
  level: i16
  target: i16
  step: i16
  drawn: bool
  alpha: f32
  draw_colour: vec3
  subtract: bool
struct Battler
  index: u8
  hp: i32
  max_hp: i32
  mp: i32
  max_mp: i32
  dead: bool
  exists: bool
  ctb: i32
  damage: i32
struct Look
  valid: bool
  colour_scale: vec3
  ambient: vec3
  background: vec3
  glow_full: f32
  glow_none: f32
  glow_gain: f32
  glow_spread: f32
  fog_colour: vec3
  fog_near: f32
  fog_far: f32
  fog_curve: f32
  fog_limit: f32
  character_ambient: vec3
  lights: list<Light>
  global_time: f32
struct Light
  direction: vec3
  colour: vec3
  level: f32
  pitch: f32
  yaw: f32
struct MenuWindow
  slot: u8
  state: u32
  rows: u16
  first: i16
  cursor: i16
  kind: u8
  args: bytes
struct SphereGrid
  open: bool
  current: u8
  cursor: u16
  nodes: list<GridNode>
  links: list<GridLink>
struct GridNode
  x: i16
  y: i16
  kind: u16
  flags: u16
struct GridLink
  a: u16
  b: u16
struct SceneObject
  id: u32
  meshes: list<SceneMesh>
  dobj: bool
  morph: f32
  morph_v: f32
struct SceneMesh
  world: mat3x4
  drawn: bool
  layer: u32
struct Save
  valid: bool
  gil: u32
  play_time: u32
  frontline: bytes
  backline: bytes
  members: list<PartyMember>
  items: list<ItemStack>
  key_items: bytes
  shared_commands: bytes
struct PartyMember
  index: u8
  name: bytes
  joined: bool
  hp: u32
  mp: u32
  max_hp: u32
  max_mp: u32
  stats: bytes
  ap: u32
  total_ap: u32
  sphere_levels: u8
  limit_charge: u8
  limit_charge_max: u8
  weapon: u8
  armour: u8
  weapon_gear: GearSpoil
  commands: bytes
  overdrive_mode: u8
  overdrive_modes: u32
struct ItemStack
  slot: u8
  id: u16
  count: u8
struct ConfigMenu
  open: bool
  cursor: i16
  pc_row: i32
  rows: list<ConfigRow>
struct ConfigRow
  title_id: u16
  title: bytes
  value: u32
  enabled: bool
  options: list<ConfigOption>
struct ConfigOption
  id: u16
  text: bytes
struct BattleEnd
  open: bool
  state: u32
  done: bool
  gil_won: u32
  gil_left: u32
  gil_step: u32
  ap: bytes
  rows: list<BattleEndRow>
  items: list<ItemStack>
  gear: bytes
  gear_full: bool
  side: u32
  cursor: i32
  cursor_won: i32
  picked: bool
  top: i32
  gear_list: i32
  gear_spoils: list<GearSpoil>
struct BattleEndRow
  character: i16
  kind: u16
  timer: i16
  in_party: bool
  colour: u8
  flash: i16
struct Layers
  valid: bool
  loaded: bool
  off: bool
  drawn: bool
  before_world: bool
  scene: i32
  pan: f32
  shift_x: f32
  shift_y: f32
  squeezed: bool
  entries: list<LayerEntry>
struct LayerEntry
  shown: bool
  scene: i16
  id: i16
struct Boosters
  speed_level: u8
  speeding: bool
  speed: f32
  powerup: bool
  encounters: u8
  auto_battle: bool
struct BattleTarget
  open: bool
  kind: u8
  character: u16
  mask: u32
  cursor: i16
  count: u16
  side: u16
  switchable: bool
  hands: list<HandCursor>
struct HandCursor
  kind: i16
  x: i16
  y: i16
struct GearSpoil
  id: u16
  used: bool
  name_id: u16
  character: u8
  kana: bool
  name: bytes
struct MessageVar
  slot: u8
  kind: u8
  value: i32
  text: bytes
  pickup: u8
  pickup_id: u16
  pickup_count: u32
struct BattleNumber
  battler: u8
  value: i32
  digits: u8
  colour: u8
  word: u8
  style: u8
  age: u8
  life: u8
  delay: u8
  x: i32
  y: i32
struct NumberFrame
  x: i16
  y: i16
  alpha: u8
struct FreeCamera
  exists: bool
  free: bool
  time_stop: bool
  pause_freeze: bool
```

## Sources

The addresses and offsets, with their evidence in `symbols/ffx.sym`:
`g_SaveData` `0x0112ca90` (docs/formats/save.md), the scene's level name
(`g_PhyreScene` `+0xfaf0`, docs/formats/render.md), `g_EventName`
`0x023cbc60`, control work 0 `0x01325ba0` (its frame count `0x01325d94`),
`g_BtlStatus` `0x0112a8e0`, `g_PauseOn` `0x01340828`, `g_FadeActive`
`0x01300704`; the scene `g_PhyreScene` `0x00ccb9d8` (`+0xdc` its main
camera: `+0xbc` far, `+0x150` aspect, `+0x154` vertical FOV; `+0xfa04`
the camera world; docs/formats/render.md, "Cameras"); the camera
channel `g_Camera` `0x02311430` (`MsCameraGetPosNow`; zeros early in
the opening's event, so the drawn camera comes from the scene); the pad `g_PadHeld`
`0x023c44ac` (stick y, x at `0x023c44a4`, `0x023c44a5`) and
`g_PadPressed` `0x013270d0`; the characters `g_ChrCount` `0x023c44e0`,
`g_ChrArray` `0x023c44e4` (records of `0x880` bytes,
`ffx_chara::layout`), `g_Player` `0x01300788`.

## Tests

API 13: `crates/ffx-state`'s `the_free_camera_and_the_number_frames` (the
three flags, a camera not mapped, the frame table's signed rows, none
without numbers); `crates/ffx-host/tests/boot.rs`
`the_state_api_sees_the_damage_numbers` (the frame table listed in every
snapshot with numbers and only there, the same 32 rows each time) and
`the_state_api_sees_the_free_camera_and_the_weapons`: in developer save
6's cut scene the singleton exists and stays off while Start is pressed
twice (Start does not pause it there); after the run each flag set alone
in the game's memory is read at its offset; Tidus's weapon is in use,
his, and named.

API 12: `crates/ffx-state`'s `the_damage_numbers_join_their_characters`
(the list walked out of the pool's order, digits and words joined, a walk
leaving the pool stopped, none out of battle) and
`crates/ffx-host/tests/boot.rs` `the_state_api_sees_the_damage_numbers`:
the deck's battle fought with Circle to the results; every new HP number
(age 0) stands over a battler that lost that much HP in its update (all
it had when the number was more), equal to its `damage` while it is
listed; three hits, two of them killing blows doing more than the HP
left (one of them with the overkill word).

API 11: `crates/ffx-state`'s unit tests (the live and the confirmed
choice, a variable of each number kind and a raw text expanded in a
window's text) and `crates/ffx-host/tests/boot.rs`
`the_state_api_follows_a_choice_and_a_2d_rooms_camera`: at Besaid
(developer save 7) a map change is seen before the beach; the beach is 3D
(16/9, not squeezed) with a new game's boosters; Circle at the save
sphere opens its two-way choice, Down, Up, Down read 1, 0, 1 before the
confirm and Circle picks 1; then the S.S. Liki's corridor (developer save
12) is a 2D room: 4/3, squeezed, `offset_y` = `0.33333337 - 2 (map_pan +
pan)`. `the_state_api_sees_the_battle_target_selection`: in the deck's
battle the command list's hand is at (50, 314); Circle (Attack) opens the
target selection on character 0x14 (its bit, two targets, side 1), Right
moves it to 0x15 and back, Cross closes it.

`crates/ffx-state` (the reader on a made-up memory, the encoding, the
schema) and `crates/ffx-host/tests/boot.rs`
`the_state_api_sees_the_opening`: on the player's game (virtual clock,
null device) an observer gets one snapshot an update; in the opening at
the fire it sees the event `lo/loopdemo`, the level `/map/znkd/znkd06/`,
the renderer's camera, and the
party shown, posed by their skeletons and animating.

`crates/ffx-host/tests/boot.rs` `the_effect_draws_are_captured_at_the_save_sphere`:
with the capture on at Besaid (developer save 7), the frame of a
snapshot's update is there when the snapshot comes; its draws use the
map's effect textures, its orb lies on the save sphere `f001`'s render
point, and its blob decodes to the same frame. `ffx_host::vfx`'s unit
tests: the encoding and its layout, the matrix, the kept frames, the
sampler read from a material, `upright_uv`.
`crates/ffx-host/tests/boot.rs` `the_cloth_is_captured_at_the_save_sphere`:
with the cloth capture on at Besaid (developer save 7), the frames of two
snapshots' updates are there when the snapshots come; the player (Tidus)
has his five cloth segments, each with as many vertices as the segment,
the simulated ones within 20 units of his render point, more than 50 of
them moving between the two frames apart from his own move; each blob
decodes to its frame. `ffx_host::cloth`'s unit tests: the encoding and
its layout, the matrix and the weight, the frame read from a made-up
memory (the call's conditions, the actor slot, the dropped short batch).
`crates/ffx-host/tests/vfx_uv.rs` (needs `FFX_GAME_DIR`): the save
sphere's sparkle uv block, read with `upright_uv`, is the one glow ball
of its atlas texture.

`crates/ffx-host/tests/boot.rs` `a_developer_save_loads_on_request`: in
the opening a frontend's `DevSave(7)` loads Besaid without the harness;
the snapshots then show the level `/map/bsil/bsil00/`, scenario 118, the
control, and the player character posed. `a_battle_starts_on_request`: from there
`Battle { 3, 0, 0 }` starts the deck's battle; the snapshots show
`World.battle`, the party, their weapons and the two monsters as actors,
and Tidus's command window (level 0, its command ids).

`crates/ffx-host/tests/boot.rs` `the_state_api_sees_the_beach_and_the_config_menu`
(API 10): at Besaid (developer save 7) the global time advances 2.0 s
over 60 updates (6.27 to 8.27); the save sphere `f001` is drawn, its three
parts unskinned on elements 2, 3, 4; a second `f002` out of the distance
cull (`hidden` 8) reads not drawn; the display objects 0..35 are listed,
objects 18..32 with morph values that change from update to update;
bsil00's 2D level is loaded with its pass off. Triangle, the pad to menu
8 (`TkMenuMainOpenOption` opens slot 10) and Circle open the config menu:
8 rows, the PC row first (ids `0x1cd`, `0x1d0`, `0x1ce`), then subtitles,
their names, the cursor, help, summons, the guide map and vibration with
their options. `the_state_api_sees_the_battle_results`: the deck's
battle won with Circle; the results screen shows the rows of characters
0 and 4, 3 AP each counted down to 0 (state 0xa, then 0xe), 5 gil won and
an item `0x2049`.
