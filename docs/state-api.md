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
  column). Measured in the opening: the three axes are orthonormal and
  right-handed, P is the camera channel's eye (`g_Camera`) with x and y negated.
- An actor's `world` is its root matrix (`+0x1d0`): rotation × 0.001 and
  its field position. Its `bones` are the elements' global matrices
  (`+0x88`), relative to the character; the root places them
  (docs/formats/chara.md, "Matrices and elements"). In events the motion
  moves the characters: in the opening the party's records stay at the
  origin and their bones carry them to the fire.

## Encoding

`Snapshot::encode`: `FFXS`, the API version (`u32`), then the fields in
their declared order. Little-endian; `bool` one byte; `string` a `u16`
byte count and UTF-8; `list<T>` a `u16` count and the items; `vec3` 3
floats, `mat3x4` 12, `mat4` 16 (row-major as the game keeps them).

## Schema

`cargo run -p ffx-state --example schema` prints it; the test
`the_doc_lists_the_schema` keeps this block equal to it. Every change of
the fields raises `API_VERSION`.

```text
ffx-state API 1
struct Snapshot
  update: u32
  game_time: f64
  world: World
  camera: Camera
  input: Input
  actors: list<Actor>
struct World
  map: u16
  previous_map: u16
  entrance: u8
  story: u16
  event: string
  event_frame: u32
  battle: bool
  player_control: bool
  paused: bool
  fading: bool
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
```

## Sources

The addresses and offsets, with their evidence in `symbols/ffx.sym`:
`g_SaveData` `0x0112ca90` (docs/formats/save.md), `g_EventName`
`0x023cbc60`, control work 0 `0x01325ba0` (its frame count `0x01325d94`),
`g_BtlStatus` `0x0112a8e0`, `g_PauseOn` `0x01340828`, `g_FadeActive`
`0x01300704`; the scene `g_PhyreScene` `0x00ccb9d8` (`+0xdc` its main
camera: `+0xbc` far, `+0x150` aspect, `+0x154` vertical FOV; `+0xfa04`
the camera world; docs/formats/render.md, "Cameras"); the camera
channel `g_Camera` `0x02311430` (`MsCameraGetPosNow`; zeros in the
opening's event, so the drawn camera comes from the scene); the pad `g_PadHeld`
`0x023c44ac` (stick y, x at `0x023c44a4`, `0x023c44a5`) and
`g_PadPressed` `0x013270d0`; the characters `g_ChrCount` `0x023c44e0`,
`g_ChrArray` `0x023c44e4` (records of `0x880` bytes,
`ffx_chara::layout`), `g_Player` `0x01300788`.

## Tests

`crates/ffx-state` (the reader on a made-up memory, the encoding, the
schema) and `crates/ffx-host/tests/boot.rs`
`the_state_api_sees_the_opening`: on the player's game (virtual clock,
null device) an observer gets one snapshot an update; in the opening at
the fire it sees the event `lo/loopdemo`, the renderer's camera, and the
party shown, posed by their skeletons and animating.
