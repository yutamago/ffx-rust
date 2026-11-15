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

## Encoding

`Snapshot::encode`: `FFXS`, the API version (`u32`), then the fields in
their declared order. Little-endian; `bool` one byte; `string` a `u16`
byte count and UTF-8; `bytes` a `u16` count and the bytes; `list<T>` a `u16` count and the items; `vec3` 3
floats, `mat3x4` 12, `mat4` 16 (row-major as the game keeps them).

## Schema

`cargo run -p ffx-state --example schema` prints it; the test
`the_doc_lists_the_schema` keeps this block equal to it. Every change of
the fields raises `API_VERSION`.

```text
ffx-state API 4
struct Snapshot
  update: u32
  game_time: f64
  world: World
  camera: Camera
  input: Input
  actors: list<Actor>
  menus: Menus
  messages: list<Message>
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
