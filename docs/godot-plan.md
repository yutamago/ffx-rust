# The Godot frontend (gd-ffx)

A plan for a community-driven remake of FINAL FANTASY X on top of
`ffx-rust`: `ffx-rust` stays the game (logic, scripts, battles, saves,
determinism), and a Godot 4 project, `gd-ffx`, becomes everything the
player sees, hears and touches. Decisions by the user, 2026-11-15; this
replaces the "native platforms through a Godot frontend later" line of
`docs/rewrite-plan.md` ("M6: ports") with a concrete plan.

## Decisions

| Topic | Decision |
|---|---|
| Role of each side | `ffx-rust` is the game-logic core; Godot draws, plays sound, takes input and hosts the editor. |
| Language in Godot | C# (Godot's .NET build). It matches Atel# and what Fahrenheit modders know. |
| Repository | Its own repository, `gd-ffx`, private until a first playable map, then public. `ffx-rust` stays lean; `gd-ffx` depends on it as a library. |
| Start | Now, alongside M6 (G0 needs neither the Deck nor PC2). |
| Godot version | The latest 4.x stable; each upgrade is a deliberate step with a test pass. |
| Rendering path | The remake directly: Godot's own renderer from the start. No back end that reproduces the original's D3D11 picture inside Godot. |
| Content not yet remade | Auto-converted "classic" scenes: an importer turns the original's models, textures, maps, cameras and animations (read from the player's own copy) into Godot scenes, so every map plays from the first day. Remade content replaces them piece by piece. |
| Visual target | A faithful modernisation: the original's art direction and camera language, with modern lighting, PBR materials and more detailed models and textures. |
| UI | Native Godot UI (Control scenes in C#) driven by `ffx-rust`'s menu state. Ruffle shows the remaster's Iggy menus only as a bridge at the start. |
| Frame rate | The logic stays at 30 updates a second (replays, determinism and saves unchanged); Godot renders at the display's rate and interpolates positions, cameras and animations between updates. |
| Sound | Godot's audio engine: the game's sound requests are routed to Godot's buses (positional sound, effects). Not sample-exact against the original; `ffx-rust`'s mixer stays for headless runs and the audio gates. |
| Mods | Godot packs (`.pck`: scenes, models, UI, C#) for remake content, plus `ffx-rust`'s own mod layers (files, Atel# scripts, function hooks) for game data and logic; one launcher composes both in load order. |

## How the two sides fit

```
 Godot 4 (.NET)                         ffx-rust (Rust, a GDExtension)
 +------------------------------+       +-------------------------------+
 | gd-ffx scenes (C#)           |       | the game: logic, Atel scripts, |
 |  - classic scenes (imported) | <---- |   battles, menus, saves        |
 |  - remade scenes             | state |   at 30 updates a second       |
 | native UI (C#)               |       |                               |
 | audio buses                  | <---- | sound requests (what, where)   |
 | input                        | ----> | input (pads, keys, mouse)      |
 | window, platforms, export    |       | Platform trait: GodotPlatform  |
 +------------------------------+       +-------------------------------+
```

- **Loading.** `ffx-rust` is built as a library and loaded as a GDExtension
  through godot-rust (`gdext`, MPL-2.0: file-level copyleft, acceptable as
  a dependency of an MIT OR Apache-2.0 project; Godot itself is MIT). A
  thin C# layer in `gd-ffx` wraps the extension's classes for the scenes.
- **The platform seam.** `ffx_host::platform::Platform` (docs/host.md,
  "The platform seam") gets a third implementation, `GodotPlatform`: window
  events and input come from Godot, the frame goes to Godot, and Steam,
  the documents folder and the process end go through Godot's equivalents.
  The Windows and headless implementations stay; replays keep running on
  them.
- **The game loop.** The game keeps its own pacing (the frame limiter at
  30 updates a second) on its own thread. After each update it publishes a
  snapshot of the state the scenes need; Godot reads the newest two
  snapshots every rendered frame and interpolates between them. Lockstep
  replays keep stepping the game themselves, as today.
- **No original picture.** The game's D3D11 calls are not drawn in Godot.
  `ffx-host` runs with the null graphics device under Godot; what the
  player sees comes from the state API alone.

## The game-state API

The core of the plan: what `ffx-rust` tells Godot each update, and what
Godot can ask. Typed and versioned, so scenes and mods don't read game
memory, and the arena can later move to Rust types without breaking them.

- **World:** the current map and its entrance, the camera (position, look
  target, field of view, cuts), the map's lights and fog, weather and
  time-of-day flags, the event state that scenes react to.
- **Actors:** every character, NPC and monster on screen: model id,
  position, rotation, scale, visibility, the animation playing and its
  time, attachments (weapons), facial animation, cloth state.
- **Battle:** the participants, their stats and states, the command menu,
  targets, the active effect and its timeline, damage numbers.
- **Effects:** the original's particle and effect systems as events
  (which effect, where, when, parameters), so a scene can play either the
  imported classic version or a remade Godot effect.
- **Menus:** the menu state machine (which screen, the cursor, the list
  contents, the help text) for the native UI.
- **Sound:** each sound request as an event: the sound or music cue, the
  position or emitter, the volume, pan and pitch the game asked for, the
  bank and the event names from the `.fev`/`.fsb` files.
- **Input in:** pads, keys and mouse mapped to the game's buttons, as the
  original's input layer expects them.

The API is defined in `ffx-rust` (a crate of its own, versioned) and
exposed through the GDExtension; `gd-ffx` generates its C# bindings from
it.

## Content

- **The classic importer.** Converters from the game's formats to Godot
  resources: maps (geometry, collision for the camera, lights), character
  and monster models with their skeletons, textures, animations, cameras,
  and the effects. Built on `ffx-rust`'s loaders (`ffx-phyre` and the
  formats in `docs/formats/`). It runs on the player's machine, from the
  player's own copy, through the launcher; no game data is shipped or
  committed, in either repository.
- **Replacing content.** Every map, model, effect and UI screen has an id
  (the game's own). A remade scene, model or effect registers for an id
  and takes over from the classic one; anything without a remake stays
  classic. Remade and classic content can sit side by side on one map
  (a remade character on a classic map).
- **The look.** Godot's Forward+ renderer with PBR materials, real-time
  lights and shadows, global illumination where it fits, and post effects
  close to the original's grading. The Steam Deck gets a lighter quality
  preset (the Mobile renderer if needed).

## Sound

The game's sound requests (music, voices, effects, the movie's audio) go
to Godot's audio engine instead of `ffx-rust`'s mixer: music and voices
on their own buses, sound effects as positional players in the scene,
reverb and filters as bus effects. The original's banks are decoded by
`ffx-rust`'s `.fsb`/`.fev` readers and handed to Godot as streams;
remastered music and voices arrive later as replacement banks through
mods. The mixer stays in `ffx-rust` for headless runs and the
sample-exact audio gates.

## UI

The menus are rebuilt as Godot Control scenes in C#, driven by the menu
state from the API: the field menu, the Sphere Grid, equipment,
abilities, items, the shop, save and load, the settings and the
remaster's screens. They keep the original's layouts and flow and take
mouse, keyboard and pad input natively. Until a screen is rebuilt, the
remaster's Iggy screens show through Ruffle as a texture.

## Mods

- Godot packs (`.pck`) carry remake content: scenes, models, textures,
  effects, UI and C# scripts.
- `ffx-rust`'s mod layers carry game data and logic: file overlays
  (kernel tables, text), Atel# event scripts and function hooks by
  original address.
- The launcher lists both kinds, resolves the load order and builds the
  result from the player's own files. Mods compose; none replaces the
  base files.
- The Godot editor is the modding tool: a mod project opens next to
  `gd-ffx`, previews the game state live, and exports a pack.

## Milestones

| | Step | Size | Needs |
|---|---|---|---|
| G0 | `gd-ffx` repository (private); `ffx-rust` as a GDExtension; `GodotPlatform` (window, input); the game runs headless inside Godot and its 30-update loop ticks; a debug view of the state | M | the platform seam (done) |
| G1 | The game-state API, first version: world, camera, actors, input; snapshots and interpolation; C# bindings | L | G0 |
| G2 | The classic importer for maps, models, textures, animations and cameras; the first map (Besaid beach) playable in Godot from start to the next map | L | G1 |
| G3 | Sound through Godot's audio engine; effects as events with the classic effects imported; battles playable with classic content | XL | G2 |
| G4 | Native UI for the field menu, battle menus, save and load; Ruffle only for screens not yet rebuilt | L | G1 |
| G5 | The whole game playable with classic content on Windows, Linux, the Steam Deck and macOS (Godot's exports); the repository goes public | XL | G2-G4 |
| G6 | Remake tooling: per-id replacement, the editor workflow, mod packs, the launcher; the first remade map as the example | L | G5 |

From G6 on, the remake grows map by map, character by character, with
the community.

## Gates

- **The game stays correct.** Every change to `ffx-rust` for the API keeps
  the replays, scene gates, audio gates and the saves round trip passing on
  the Windows and headless platforms. The API only reads state and feeds
  input; it never changes what the game does.
- **The bridge is faithful.** A replay can run under Godot: the same inputs
  give the same game events (trace ranges) as the headless run. Rendering
  and sound are not pixel or sample gated in Godot.
- **Coverage.** A list of every map, model, effect and screen with its
  status (classic, remade, missing) shows how far the remake is.
- **Platforms.** Each export (Windows, Linux, the Steam Deck, macOS) plays
  Besaid from the title to the first battle, then the whole game with
  classic content by G5.

## Open questions

- Steam in Godot: Valve's library through `ffx-rust`'s existing loader,
  or a Godot Steam plugin.
- Movies: the FMVs play as Godot video (the game's `.webm` files decoded
  by `ffx-rust` and handed over as frames), or are replaced by remade
  scenes later.
- How much of the PS2-era camera behaviour (fixed angles, cuts) the
  remade maps keep; the faithful-modernisation target says most of it.
- The licence for community assets in `gd-ffx` (code stays MIT OR
  Apache-2.0; art may need a Creative Commons licence).
