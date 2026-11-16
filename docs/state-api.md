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
`GameSetting.ini` gives Japanese, 0); `{VAR}` codes are not filled). The
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
damage numbers, sound effects.

## Battlers (API 6)

`Snapshot.battlers`, in battle only (`g_BtlStatus`): the actors taking part
(`+0xdc8` `in_battle`) of `g_BtlChrTop` (0x1f x 0xf90; 0..0x11 the party,
0x14..0x1b the monsters) with HP, MP and their maximums, KO (`+0xdcc`),
shown (`+0xdd2`) and the turn counter (`+0x6ec`): a frontend's battle HUD.
A monster that dies with a death pattern (`+0x3f5`) usually leaves the list
(its `+0xdc8` cleared) rather than showing `dead`
(docs/formats/battle.md, "The battle actor").

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
commands everybody learnt (from `0x3060`). Names are in the game's text encoding (a frontend decodes
them with the game's character set, as the messages). The battle's party
index is the member's `index`, so a battle HUD names its battlers from
here. `menus.windows` lists the PS2 menus' windows in use
(`g_TkMenuWindows`): their rows, first row shown, cursor, list kind and
the opener's words (kind, character, index), so each screen's cursor and
the character it shows can be drawn.

## Objects (API 8)

`objects` is the map's object tree as the scene keeps it (`PhyreScene`
`+0x1198`): per object id (the number in its meshes' names, `N_kShape`)
its meshes' live world matrices (3x4, the camera's packing, render space)
and whether each is drawn (the layer mask). Scripts move objects
(`graphicObjectSetTranslation`/`Rotation` rewrite the matrices) and show
one of several meshes at a time (layers), e.g. Besaid's birds (objects
18..32, two wing poses each, placed at the origin in the file).

## Sphere grid (API 8)

`sphere_grid` is the sphere grid screen's data while it is loaded
(`*g_Abmap`, docs/formats/pc-ui.md): the character shown, the node under
the cursor, the nodes (place,
kind, flags) and the links between them. The standard grid has 828 nodes
and 834 links. The node's `+0x20` flags (a bit per character from bit 8)
are read as who activated it; that meaning is observed, not checked in the
code.

## Schema

`cargo run -p ffx-state --example schema` prints it; the test
`the_doc_lists_the_schema` keeps this block equal to it. Every change of
the fields raises `API_VERSION`.

```text
ffx-state API 8
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
struct Camera
  valid: bool
  world: mat3x4
  fov_y: f32
  aspect: f32
  far: f32
  eye: vec3
  target: vec3
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
struct Bone
  matrix: mat4
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
struct Effects
  magic: i32
  running: list<EffectInstance>
  filters: list<ScreenFilter>
  shake_x: f32
  shake_y: f32
  music: i32
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
struct Battler
  index: u8
  hp: i32
  max_hp: i32
  mp: i32
  max_mp: i32
  dead: bool
  exists: bool
  ctb: i32
struct Look
  valid: bool
  colour_scale: vec3
  ambient: vec3
  fog_colour: vec3
  fog_near: f32
  fog_far: f32
  fog_curve: f32
  fog_limit: f32
  character_ambient: vec3
  lights: list<Light>
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
struct SceneMesh
  world: mat3x4
  drawn: bool
  layer: u8
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
  commands: bytes
  overdrive_mode: u8
  overdrive_modes: u32
struct ItemStack
  slot: u8
  id: u16
  count: u8
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

`crates/ffx-state` (the reader on a made-up memory, the encoding, the
schema) and `crates/ffx-host/tests/boot.rs`
`the_state_api_sees_the_opening`: on the player's game (virtual clock,
null device) an observer gets one snapshot an update; in the opening at
the fire it sees the event `lo/loopdemo`, the level `/map/znkd/znkd06/`,
the renderer's camera, and the
party shown, posed by their skeletons and animating.

`crates/ffx-host/tests/boot.rs` `a_developer_save_loads_on_request`: in
the opening a frontend's `DevSave(7)` loads Besaid without the harness;
the snapshots then show the level `/map/bsil/bsil00/`, scenario 118, the
control, and the player character posed. `a_battle_starts_on_request`: from there
`Battle { 3, 0, 0 }` starts the deck's battle; the snapshots show
`World.battle`, the party, their weapons and the two monsters as actors,
and Tidus's command window (level 0, its command ids).
