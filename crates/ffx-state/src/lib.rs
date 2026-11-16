//! `ffx-state`: the game-state API (docs/state-api.md). What a frontend
//! (gd-ffx, the Godot frontend: docs/godot-plan.md) reads of the game after
//! each update: the world, the camera, the input and the characters, typed
//! and versioned, so that scenes and mods never read game memory and the
//! game's data can later move to Rust types without breaking them.
//!
//! - [`Snapshot`] and its parts: plain Rust structs.
//! - [`read`]: a snapshot read from the game's memory through a function
//!   that reads bytes where they are mapped (no executor, no arena type: the
//!   frontend's side and the host's tests call it alike). It only reads.
//! - [`Snapshot::encode`]: the snapshot as bytes (little-endian, the fields
//!   in their declared order), the form the `GDExtension` hands to Godot.
//! - [`schema`]: every struct's fields and types, from which a frontend
//!   generates its bindings (gd-ffx generates C#).
//!
//! [`API_VERSION`] changes with every change of the fields; an encoded
//! snapshot starts with [`MAGIC`] and the version.

mod read;
pub mod schema;

pub use read::{LANGUAGE_UNKNOWN, Reader, read};
pub use schema::{ApiStruct, ApiType, Field, StructSchema, Ty};

/// The API's version: a frontend checks it before it decodes a snapshot.
pub const API_VERSION: u32 = 9;

/// The first four bytes of an encoded snapshot (`FFXS`).
pub const MAGIC: u32 = u32::from_le_bytes(*b"FFXS");

/// Declares an API struct: the struct, its encoding and its schema.
macro_rules! api_struct {
    (
        $(#[doc = $doc:literal])*
        $name:ident {
            $(
                $(#[doc = $fdoc:literal])*
                $field:ident : $ty:ty
            ),* $(,)?
        }
    ) => {
        $(#[doc = $doc])*
        #[derive(Debug, Clone, Default, PartialEq)]
        pub struct $name {
            $(
                $(#[doc = $fdoc])*
                pub $field: $ty,
            )*
        }

        impl $crate::schema::ApiStruct for $name {
            fn schema() -> &'static $crate::schema::StructSchema {
                static SCHEMA: $crate::schema::StructSchema = $crate::schema::StructSchema {
                    name: stringify!($name),
                    doc: concat!($($doc, "\n"),*),
                    fields: &[
                        $(
                            $crate::schema::Field {
                                name: stringify!($field),
                                ty: <$ty as $crate::schema::ApiType>::TY,
                                doc: concat!($($fdoc, "\n"),*),
                            },
                        )*
                    ],
                };
                &SCHEMA
            }

            fn encode_fields(&self, out: &mut Vec<u8>) {
                $( $crate::schema::ApiType::encode(&self.$field, out); )*
            }
        }

        impl $crate::schema::ApiType for $name {
            const TY: $crate::schema::Ty =
                $crate::schema::Ty::Struct(<$name as $crate::schema::ApiStruct>::schema);
            fn encode(&self, out: &mut Vec<u8>) {
                $crate::schema::ApiStruct::encode_fields(self, out);
            }
        }
    };
}

api_struct! {
    /// The game after one update: everything a frontend draws from.
    Snapshot {
        /// Updates the game has run (30 a second); the snapshot is taken
        /// between update `update - 1` and update `update`.
        update: u32,
        /// The game's clock in seconds.
        game_time: f64,
        /// The map, the story, what runs.
        world: World,
        /// The view.
        camera: Camera,
        /// The pad as the game logic reads it.
        input: Input,
        /// The field characters in use (party, NPCs, monsters, objects,
        /// weapons), by slot.
        actors: Vec<Actor>,
        /// The menus' state (which is open, the cursors).
        menus: Menus,
        /// The script's message windows that are not closed (dialogue,
        /// choices, captions).
        messages: Vec<Message>,
        /// What a frontend plays and draws besides the world: effects,
        /// screen filters, the music.
        effects: Effects,
        /// The battle's actors taking part (party and monsters), empty out
        /// of battle.
        battlers: Vec<Battler>,
        /// How the scene is lit and coloured: the colour scale every
        /// material gets, the fog, the character lights.
        look: Look,
        /// The game's own record of play (`g_SaveData`): gil, the party,
        /// the characters' names and stats, the inventory; what the menus
        /// and the save screens show.
        save: Save,
        /// The map's objects as the scene keeps them now (`PhyreScene`
        /// `+0x1198`): each id's meshes with their live world matrices and
        /// whether they are drawn, so objects that scripts move
        /// (`graphicObjectSetTranslation`/`Rotation`) or show and hide (the
        /// layer mask) can be drawn where the game draws them.
        objects: Vec<SceneObject>,
        /// The sphere grid while its screen is up (`*g_Abmap`), else empty.
        sphere_grid: SphereGrid,
    }
}

api_struct! {
    /// A battle actor (`g_BtlChrTop` `0x011334d4`, 0xf90 bytes each:
    /// docs/formats/battle.md "The battle actor").
    Battler {
        /// Its index (0..0x11 the party, 0x14..0x1b the monsters).
        index: u8,
        /// HP and its maximum (`+0x5d0`, `+0x594`).
        hp: i32,
        /// See `hp`.
        max_hp: i32,
        /// MP and its maximum (`+0x5d4`, `+0x598`).
        mp: i32,
        /// See `mp`.
        max_mp: i32,
        /// KO'd (`+0xdcc` `stat_death` nonzero: 1 KO'd, 2 the death pose
        /// set). A monster that dies with a death pattern usually leaves
        /// `battlers` instead (its `+0xdc8` cleared).
        dead: bool,
        /// Shown on the field (`+0xdd2` `stat_exist_flag`).
        exists: bool,
        /// Its turn counter (`+0x6ec` `current_ctb`).
        ctb: i32,
    }
}

api_struct! {
    /// The effects running, the screen's filters and shake, the music
    /// (docs/formats/vfx.md, main.md, sound.md).
    Effects {
        /// The spell (magic file) playing (`g_MagicFile_MagicId`), -1 none.
        magic: i32,
        /// The effect records in use (`g_EtEff`: battle and event effects),
        /// by slot.
        running: Vec<EffectInstance>,
        /// The screen filters (`g_FadeSlots`, `g_FadeActive`): slot 0 the
        /// cross-fade, 1 motion blur, 2 a flash, 3 a colour filter, 4 the
        /// fade.
        filters: Vec<ScreenFilter>,
        /// The screen's shake offset (`g_Screens[0]` `+0x30`, `+0x34`).
        shake_x: f32,
        /// See `shake_x`.
        shake_y: f32,
        /// The music playing (`g_SndNowPlayMusic`), -1 none.
        music: i32,
    }
}

api_struct! {
    /// An effect record of `g_EtEff`.
    EffectInstance {
        /// Its slot (0..511): the key between snapshots.
        slot: u16,
        /// The effect id (`MsGetEffID`: group << 16 | a << 8 | b; bind
        /// effects 0x40000400.. + the character).
        id: u32,
        /// The effect's first character (`+0x18`: out of battle
        /// `g_EtChr`'s index, in battle `MsGetEff2Chr`'s).
        character: u8,
        /// Its kind (`+0x1c`).
        kind: u8,
        /// It ended (`+0x1d`).
        ended: bool,
        /// Its targets' character indices (`+0x10`, `+8` the count).
        targets: Vec<u8>,
    }
}

api_struct! {
    /// A screen filter slot of `g_FadeSlots` (ffx-sg's `Sg_DrawFilter`):
    /// its `+4`..`+0xa` words mean different things per slot.
    ScreenFilter {
        /// The slot: 0 the cross-fade, 1 motion blur, 2 a flash, 3 a colour
        /// filter, 4 the fade.
        slot: u8,
        /// It runs (`g_FadeActive`'s bit).
        active: bool,
        /// Its mode (`+0`): 0 the cross-fade's level, 1 the blur's set byte
        /// (not drawn by itself), 2 the fade from black, 3 to black (held),
        /// 4 from white, 5 to white (`Sg_FadeIn`, `Sg_FadeOut`, `..W`; mode
        /// 4's alpha starts near 2: clamp it), 6 a flash, 7 an additive
        /// filter (`Sg_FilterAdd`), 8 a subtractive one (`Sg_FilterSub`).
        mode: u8,
        /// Its colour (`+1..+3`; every fade writes 0xff here: modes 2 and 3
        /// draw black, 4 and 5 that white).
        color: [u8; 3],
        /// `+4`: frames left (the cross-fade, the flash, the fade), or on
        /// (the blur, the filter).
        frames: i16,
        /// `+6`: the frames in all (the flash, the fade), the alpha now
        /// (the cross-fade), the weight (the blur), the level / 127 (the
        /// filter).
        level: i16,
        /// `+8`: the cross-fade's target alpha.
        target: i16,
        /// `+0xa`: the cross-fade's step.
        step: i16,
    }
}

api_struct! {
    /// The menus' state, for a native UI (docs/formats/menu.md, save.md,
    /// battle.md).
    Menus {
        /// The open PS2 menus: a bit per slot of `g_TkMenuSlots` (`0x2`
        /// the main menu, `0x1000` the weapon shop, `0x2000` the item shop;
        /// `g_TkMenuMainActiveMask`).
        main_mask: u32,
        /// The main menu's highlighted option (`g_TkMenuMainOptionCursor`).
        main_cursor: i32,
        /// The main menu's options (`g_TkMenuMainMaxOptions`).
        main_options: i32,
        /// Those options: their menu id, whether they can be chosen and
        /// their name (`g_TkMenuMainOptions`; `TkMenuMainMes`).
        main_option_list: Vec<MenuOption>,
        /// The battle command window that is open: the character's bit
        /// (Tidus 1, Yuna 2, Auron 4, Kimahri 8, Wakka 0x10, Lulu 0x20,
        /// Rikku 0x40; `g_BtlWindowOpen`), 0 none.
        battle_window: u32,
        /// The character whose window it is (`g_BtlWindowChr`, 0xff none).
        battle_character: i16,
        /// The command window's level (`g_BtlActionDepth`: 0 the top
        /// commands, then sub-menus).
        battle_depth: i16,
        /// That level's highlighted row (its record's `+0x42`, in
        /// `g_BtlWindowRecords`); 0 at level -1 (no command window).
        battle_cursor: i16,
        /// That level's ids, `u16` little-endian each (its record's `+0x20`
        /// list, `+0x44` count; `id >> 12`: 2 item, 3 command, 4 or 6
        /// monster command); none at level -1.
        battle_ids: Vec<u8>,
        /// The PC save/load screen's state (`g_SaveMachineState`: 1..6
        /// saving, 0xb..0x11 loading, 0 closed).
        save_state: u32,
        /// The highlighted row on screen, 0..4 (`g_SaveListCursor`); the
        /// list's index is `save_top + save_cursor`.
        save_cursor: i32,
        /// The list's first row on screen (`g_SaveListTop`).
        save_top: i32,
        /// The rows listed (saving: the files but the first, and a new
        /// file's row unless the list is full).
        save_rows: i32,
        /// The save files found.
        save_files: i32,
        /// The PC menu (Esc, Flash) is up (`g_FFEscMenu`).
        esc_menu: bool,
        /// The PS2 menus' windows in use (`g_TkMenuWindows`, 32 of `0x98`
        /// bytes, `+0x40` set; docs/formats/menu.md "Windows"): each
        /// screen's lists and cursors.
        windows: Vec<MenuWindow>,
        /// The character the PS2 menus show (`TkMenuGetCurrentPlayer`:
        /// `g_TkMenuPlayers` `0x01841c14` at `g_TkMenuCurrentPlayerPos`
        /// `0x01841c28`; below 8 a party member, 8 and up an aeon); 0xff
        /// when the position is 8 or more (the list is 8 bytes).
        current_player: u8,
    }
}

api_struct! {
    /// One PS2 menu window in use (docs/formats/menu.md, "Windows").
    MenuWindow {
        /// Its record (0..31).
        slot: u8,
        /// Its state (`+0x28`, 0 idle).
        state: u32,
        /// Its row count (`+0x30`).
        rows: u16,
        /// The first row shown (`+0x32`).
        first: i16,
        /// The cursor (`+0x48`; the row is `cursor >> 1` in two-column
        /// lists).
        cursor: i16,
        /// The list kind of an item window (`+0x43`: 0 items, 1 key items,
        /// 2 commands).
        kind: u8,
        /// The opener's five words (`+0x58`..`+0x6b`: kind, character,
        /// index, ...), `u32` little-endian each.
        args: Vec<u8>,
    }
}

api_struct! {
    /// An option of the main menu.
    MenuOption {
        /// The menu it opens (`g_TkMenuMainOptions` `+0`, a `u32` the game
        /// reads; its low 16 bits here: the names' lookup takes 12).
        id: u16,
        /// It can be chosen (`+4`).
        enabled: bool,
        /// Its name in the game's encoding (`MsMenuGetText(9, id, kana)`, the
        /// menu text the game loaded; kana names when the config says so).
        name: Vec<u8>,
    }
}

api_struct! {
    /// A message window of the event script (`g_AtelMsgWindows`, the bank
    /// `g_AtelMenuExecStatus` selects; docs/formats/script.md).
    Message {
        /// The window (0..7).
        window: u8,
        /// 1 opening, 2 open, 3 closing, 4 (unknown).
        state: u16,
        /// Its place on the PS2 screen (512 x 448).
        x: i16,
        /// See `x`.
        y: i16,
        /// `Msg_Flags` (the high byte: the number of choices).
        flags: u16,
        /// The mode bits (0x02 a choice, 0x10 waiting, 0x20 a selection).
        mode: u8,
        /// The choice the player has highlighted now (the text box's
        /// record through `0x01868a90`'s table, `TOGetTableKind2Data`), -1
        /// when the window holds no choice.
        choice: i16,
        /// The choice the script set it to at the start (`+0x1e`).
        initial_choice: i16,
        /// The text in the game's encoding (ffx-text's charset decodes
        /// it; `{VAR}` codes are filled from `g_MsgVars`, not here).
        text: Vec<u8>,
    }
}

api_struct! {
    /// The map and the state of play.
    World {
        /// The current map (`g_SaveData` `+0`).
        map: u16,
        /// The map before it (`+2`).
        previous_map: u16,
        /// The entrance the map was entered by (`+0xc`).
        entrance: u8,
        /// The story progress (`g_SaveScenario`, `+0xbec`).
        story: u16,
        /// The running event's name, e.g. `bl/bltz0000` (`g_EventName`).
        event: String,
        /// The level the renderer has loaded (the scene's current level
        /// name, `PhyreScene` `+0xfaf0`): the map model to draw.
        level: String,
        /// The running event's frame count (control work 0).
        event_frame: u32,
        /// A battle runs (`g_BtlStatus` nonzero).
        battle: bool,
        /// The player controls the party (control work 0, bit 1).
        player_control: bool,
        /// The game is paused (`g_PauseOn`).
        paused: bool,
        /// A screen fade runs (`g_FadeActive` nonzero).
        fading: bool,
        /// The language of the game's text (`TOGetFFXLang`: 0 Japanese,
        /// 1 English, 2 French, 3 Spanish, 4 German, 5 Italian, 9 Korean,
        /// 10 Chinese; 0xff unknown): the character set that decodes the
        /// messages.
        language: u8,
    }
}

api_struct! {
    /// The view the game's renderer draws with: the scene's main camera
    /// (`PhyreScene` `+0xdc`) and its world matrix (`+0xfa04`), in render
    /// space (the field's x and y negated: y up).
    Camera {
        /// A scene and its camera exist (the values below were read).
        valid: bool,
        /// The camera's world matrix, 3x4 as the scene keeps it
        /// (`graphicCameraSetWorldMatrix`: `PMatrix4_toWorldMatrix3x4` of
        /// the inverted view).
        world: [f32; 12],
        /// The vertical field of view, radians (`PCamera` `+0x154`).
        fov_y: f32,
        /// Width over height (`+0x150`).
        aspect: f32,
        /// The far plane (`+0xbc`).
        far: f32,
        /// The camera channel's eye in field space (`g_Camera`,
        /// `MsCameraGetPosNow`, which the battle reads; zeros early in the
        /// opening's event, where the scene's camera above is the one drawn).
        eye: [f32; 3],
        /// The camera channel's look-at point in field space.
        target: [f32; 3],
    }
}

api_struct! {
    /// The sphere grid screen's data: `g_Abmap` (`0x02305834`) holds the
    /// address of `g_AbmapData` (set by `InitLpamng`); docs/formats/pc-ui.md
    /// "Sphere grid helpers" and docs/formats/field.md (x, y, the panel bits
    /// at `+0x21`).
    SphereGrid {
        /// The grid screen runs (`g_AbmapRunning` `0x01a85f70`, which
        /// `TOMenuTermAbilityMap2` clears; `g_Abmap` itself stays set).
        open: bool,
        /// The character the screen shows (`+0x115bc`,
        /// `TkMenuGetCurrentPlayer`).
        current: u8,
        /// The node under the cursor (`+0x112fc`, an index into `nodes`; its
        /// kind names the help line, `AbmapSetHelpMessage`).
        cursor: u16,
        /// The nodes (`+0x808`, 0x28 bytes each, the count `i16` at `+2`).
        nodes: Vec<GridNode>,
        /// The links between nodes (`+0xa808`, 0x14 bytes each, the count
        /// `i16` at `+4`).
        links: Vec<GridLink>,
    }
}

api_struct! {
    /// One sphere grid node.
    GridNode {
        /// Its place on the grid (`+0`, `+2`; observed: y grows down).
        x: i16,
        y: i16,
        /// What it holds (`+6`; 0xffff an empty node), a record of the
        /// kernel's node table (`MsGetExcelData`).
        kind: u16,
        /// `+0x20`: a bit per character from bit 8 (observed: set on the
        /// nodes around the characters' starting places early in the
        /// game; read as the characters who activated it, not checked in
        /// the code).
        flags: u16,
    }
}

api_struct! {
    /// One link between two sphere grid nodes (`+0`, `+2`: their indices).
    GridLink {
        a: u16,
        b: u16,
    }
}

api_struct! {
    /// One map object: a key of the scene's object tree (`PhyreScene`
    /// `+0x1198`), each mesh instance's `+0x58` (`obj_id`;
    /// `PhyreScene_rebuildMapObjects` `0x00658ae0`). Observed at Besaid:
    /// the number in its meshes' names, `N_kShape`.
    SceneObject {
        /// The object id (the tree node's key, `+0xc`).
        id: u32,
        /// Its meshes in the scene's order (the node's list, `+0x10`).
        meshes: Vec<SceneMesh>,
    }
}

api_struct! {
    /// One mesh instance of a map object.
    SceneMesh {
        /// Its world matrix (`*(mesh +4)`), 3x4 in the camera's packing
        /// (`PMatrix4_toWorldMatrix3x4`; render space, the map's x 10
        /// included).
        world: [f32; 12],
        /// It is drawn: its draw record (`+0x1c`) points back at it (what
        /// `setMeshInstanceGroupIdVisible` sets from the mask `+0xfb14`);
        /// false without a record.
        drawn: bool,
        /// Its group id (`+0x48`, `m_groupID`): the bit `1 << (group id & 31)` of
        /// the scene's mask `+0xfb14`.
        layer: u32,
    }
}

api_struct! {
    /// The game state block's parts the menus show (`g_SaveData`
    /// `0x0112ca90`, docs/formats/save.md; the names Fahrenheit's).
    Save {
        /// `g_SaveData` is readable (the values below were read).
        valid: bool,
        /// Gil (`+0x3d48`, `g_SaveGil`).
        gil: u32,
        /// Play time in seconds (`+0xbc`; the game stops it at 3,599,999,
        /// 999:59:59).
        play_time: u32,
        /// The front line: 3 character indices (`+0x3d58`,
        /// `g_SaveInParty`; observed: 0xff in an empty place).
        frontline: Vec<u8>,
        /// The back line: 17 (`+0x3d5b`, `g_SaveOutParty`; observed: 0xff
        /// in an empty place).
        backline: Vec<u8>,
        /// The 18 characters (`ply_save`, `+0x55cc`, 0x94 bytes each; and
        /// their names, `+0x634c`).
        members: Vec<PartyMember>,
        /// The inventory's used slots (`+0x3ecc` the ids, `+0x40cc` the
        /// counts; 256 slots): a slot with a count.
        items: Vec<ItemStack>,
        /// The key items, a bit each (`+0x448c`, 8 `u16`, as bytes).
        key_items: Vec<u8>,
        /// The commands from `0x3060` learnt by everybody
        /// (`g_SaveSharedCommands`, `+0x3d6c`, 56 bytes): command `0x3060 +
        /// i` is bit `i % 16` of `u16` `i / 16` (`MsSetSaveCommand`).
        shared_commands: Vec<u8>,
    }
}

api_struct! {
    /// One character's `ply_save` (docs/formats/save.md).
    PartyMember {
        /// Its index (0 Tidus .. 17; the battle's party index too).
        index: u8,
        /// Its name as the player set it, in the game's text encoding (20
        /// bytes at `+0x634c + 20 i`, up to the first 0; decode with the
        /// game's character set).
        name: Vec<u8>,
        /// Joined (`+0x2c` bit 0, `MsGetSavePlyJoin`).
        joined: bool,
        /// HP, MP and their maxima (`+0x1c`..`+0x28`).
        hp: u32,
        mp: u32,
        max_hp: u32,
        max_mp: u32,
        /// The 8 stats (`+0x2f`..`+0x36`): strength, defence, magic, magic
        /// defence, agility, luck, evasion, accuracy.
        stats: Vec<u8>,
        /// AP and all AP gained (`+0x18`, `+0x14`).
        ap: u32,
        total_ap: u32,
        /// Sphere levels available (`+0x3b`).
        sphere_levels: u8,
        /// The limit gauge and its full value (`+0x39`, `+0x3a`; copied to
        /// and from the battle actor's `+0x5bc`/`+0x5bd` by `MsSetChrParam`
        /// and `MsSetSaveData`).
        limit_charge: u8,
        limit_charge_max: u8,
        /// Weapon and armour: their slots in `g_Equipment` (`+0x2d`, `+0x2e`;
        /// 0xff none, docs/formats/battle.md "Saved parameters").
        weapon: u8,
        armour: u8,
        /// The commands `0x3000`..`0x305f` learnt (`+0x3e`, 6 `u16` as
        /// bytes): command `0x3000 + i` is bit `i % 16` of `u16` `i / 16`
        /// (`MsSetSaveCommand`); later ones are `Save.shared_commands`.
        commands: Vec<u8>,
        /// The overdrive mode in use (`+0x38`, copied to the battle actor's
        /// `+0x5bb` by `MsSetChrParam`/`MsSetSaveData`) and the modes had,
        /// bit `k` for mode `k` (`+0x88`, `MsLimitTypeExpGet`,
        /// `MsLimitTypeProcess`).
        overdrive_mode: u8,
        overdrive_modes: u32,
    }
}

api_struct! {
    /// One inventory slot in use.
    ItemStack {
        /// The slot (0..255): the menu's order.
        slot: u8,
        /// The item id (observed: `0x2000` + the kernel's item index, as the
        /// battle menu's ids, `id >> 12` 2).
        id: u16,
        /// How many.
        count: u8,
    }
}

api_struct! {
    /// How the renderer colours the scene (docs/formats/render.md, "Lights
    /// and fog at run time" and "Scenery light levels"): what a frontend
    /// needs to draw the game's own shaders' result. `scene` is
    /// `*g_PhyreScene`, the context its `+0xcc` (`*g_PhyreApplication +
    /// 0x50`), the lighting `*g_CharacterLighting`.
    Look {
        /// A scene and its context exist (the values below were read).
        valid: bool,
        /// The colour scale (`scene +0xfc78`): the scale
        /// `PhyreScene_setColorScale` last gave the map's materials as their
        /// `clrScaleFactor` (the tree `+0x1198`; nothing while `+0x10d4` is
        /// 0, and materials loaded after the last call keep their file's
        /// value), the last multiplier before the map shaders' tone curve.
        colour_scale: [f32; 3],
        /// `GlobalAmbientColor` (context `+0x00`; the map's
        /// `GlobalCamera` `m_ambientColor`, 0 without character lighting).
        ambient: [f32; 3],
        /// The background (clear) colour (context `+0x30`: the map's
        /// `GlobalCamera` `m_bgColor`, docs/formats/postfx.md; e.g. 0.235,
        /// 0.235, 0.275 on cdsp00): the scene target's clear colour, not
        /// tone-curved; the post passes still apply.
        background: [f32; 3],
        /// The glow pass (docs/formats/postfx.md, "Glow map", "Glow"), from
        /// the map's `GlobalCamera`: `FullGlowThreshold` (context `+0x80`),
        /// `NoGlowThreshold` (`+0x84`, at least 0.35), `GlowGain` (`+0x88`)
        /// and the spread (`+0x8c`). Without a `GlobalCamera`, and on map
        /// 0x199 (0x12ea and 0x1733 while `getScenerioFlag` is 0): 1, 1, 0,
        /// 0 (docs/formats/postfx.md, "Where the constants come from").
        glow_full: f32,
        glow_none: f32,
        glow_gain: f32,
        glow_spread: f32,
        /// The fog's colour (context `+0x10`).
        fog_colour: [f32; 3],
        /// The fog's near and far distances (`+0x20`, `+0x24`), its curve
        /// (`+0x28`) and limit (`+0x2c`, the map's limit / 255). Without a
        /// `GlobalCamera`: colour (0.28, 0.35, 0.6), near 65536, far 65537,
        /// curve 1, limit 1.
        fog_near: f32,
        fog_far: f32,
        fog_curve: f32,
        fog_limit: f32,
        /// `CharacterAmbientLight` (the scene context `+0xac`, reached as
        /// the lighting's `+0x18c`: the ambient colour times its level).
        character_ambient: [f32; 3],
        /// The character lights 0-2 (`PhyreChrLitShader`'s `Light0`..
        /// `Light2`), empty before the lighting is initialised.
        lights: Vec<Light>,
    }
}

api_struct! {
    /// One character light (`g_CharacterLighting`'s `PLight` i). The
    /// shader adds `colour x level x (dot(direction, N) x 0.5 + 0.5)^2`.
    Light {
        /// The direction as `Phyre_CharacterLighting_setLightDir` stored it
        /// (lighting `+0xf4 + 0x10 i`, render space: the field's x and y
        /// negated), before the map's yaw and pitch below turn it.
        direction: [f32; 3],
        /// The colour (`PLight +0`).
        colour: [f32; 3],
        /// The level (`PLight +0x24`: the map's intensity, the glow
        /// balance and the PC's factors; docs/formats/render.md).
        level: f32,
        /// The map's turn of this light, degrees (lighting `+0x1ac` pitch,
        /// `+0x1b0` yaw, `+ 0xc i`): `Rx(pitch) * Ry(yaw) * normalize(d)`,
        /// the degrees as `/ 180 * 3.14`, skipped while both are within
        /// 0.001 of 0.
        pitch: f32,
        yaw: f32,
    }
}

api_struct! {
    /// The pad as the game logic reads it (PS2 bits: `0x10` triangle,
    /// `0x20` circle, `0x40` cross, `0x80` square, `0x800` start,
    /// `0x1000`.. up, right, down, left; ffx-rust docs/harness.md "Input
    /// facts").
    Input {
        /// Buttons held (`g_PadHeld`, the field's movement reads it).
        held: u32,
        /// Buttons newly pressed this update (`g_PadPressed`, the scripts'
        /// `AtelGetControler`).
        pressed: u16,
        /// The left stick, 0..255 with 128 at rest.
        stick_x: u8,
        /// See `stick_x`.
        stick_y: u8,
    }
}

api_struct! {
    /// A field character (a `Ch` record, ffx-rust docs/formats/chara.md).
    Actor {
        /// The record's slot in `g_ChrArray`: stable while the character
        /// lives, the key to match it between snapshots.
        slot: u16,
        /// The character id: `kind << 12 | number`.
        id: u16,
        /// The kind: 0 party (`c`), 1 monster (`m`), 2 NPC (`n`), 3 aeon
        /// (`s`), 4 weapon (`w`), 5 object (`f`), 6 skeleton (`k`).
        kind: u8,
        /// The model's name (its folder: `chr/<kind>/<name>`).
        model: String,
        /// The position in field space.
        position: [f32; 3],
        /// The facing, radians (the world matrix turns by `-yaw - pi/2`).
        yaw: f32,
        /// The scale.
        scale: [f32; 3],
        /// Hidden: by an event (1), an effect (2), a battle (0x10), or the
        /// distance cull (8, 0x20); 0 shown.
        hidden: u8,
        /// The fade's alpha, 0..1 (0 also hides).
        alpha: f32,
        /// The motion playing: `character << 16 | number`.
        motion: u32,
        /// The motion's frame (frames of 1/30 s, fractional).
        motion_frame: f32,
        /// The motion's speed (1 normal).
        motion_speed: f32,
        /// The slot of the character this one is bound to (a weapon to its
        /// holder), -1 none.
        parent: i16,
        /// The parent's element (bone) it is bound to.
        parent_bone: u16,
        /// The player's character (`g_Player`).
        player: bool,
        /// The root's world matrix as the game builds it (rotation x 0.001,
        /// translation the position; row-major, field space).
        world: [f32; 16],
        /// The skeleton's elements (bones) as posed this update, in the
        /// model's order (an HD model's `SkeletonNode_i` takes bone `i`).
        bones: Vec<Bone>,
    }
}

api_struct! {
    /// One element (bone) of a character's skeleton.
    Bone {
        /// Its global matrix relative to the character (`+0x88` of the
        /// element; row-major): the root's `world` places it in the field.
        matrix: [f32; 16],
    }
}

impl Snapshot {
    /// The snapshot as bytes: [`MAGIC`], [`API_VERSION`], then the fields.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(256 + self.actors.len() * 160);
        out.extend_from_slice(&MAGIC.to_le_bytes());
        out.extend_from_slice(&API_VERSION.to_le_bytes());
        ApiStruct::encode_fields(self, &mut out);
        out
    }
}

/// Every struct of the API, the root ([`Snapshot`]) first.
#[must_use]
pub fn structs() -> Vec<&'static StructSchema> {
    vec![
        Snapshot::schema(),
        World::schema(),
        Camera::schema(),
        Input::schema(),
        Actor::schema(),
        Bone::schema(),
        Menus::schema(),
        MenuOption::schema(),
        Message::schema(),
        Effects::schema(),
        EffectInstance::schema(),
        ScreenFilter::schema(),
        Battler::schema(),
        Look::schema(),
        Light::schema(),
        MenuWindow::schema(),
        SphereGrid::schema(),
        GridNode::schema(),
        GridLink::schema(),
        SceneObject::schema(),
        SceneMesh::schema(),
        Save::schema(),
        PartyMember::schema(),
        ItemStack::schema(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_snapshot_encodes_its_fields_in_order() {
        let s = Snapshot {
            update: 7,
            game_time: 1.5,
            actors: vec![Actor {
                slot: 3,
                model: "c001".into(),
                ..Actor::default()
            }],
            ..Snapshot::default()
        };
        let b = s.encode();
        assert_eq!(&b[0..4], b"FFXS");
        assert_eq!(u32::from_le_bytes(b[4..8].try_into().unwrap()), API_VERSION);
        assert_eq!(u32::from_le_bytes(b[8..12].try_into().unwrap()), 7);
        assert_eq!(f64::from_le_bytes(b[12..20].try_into().unwrap()), 1.5);
        // update, game time, the world (with an empty event), the camera,
        // the input, then one actor ("c001", no bones)
        let world = 2 + 2 + 1 + 2 + 2 + 2 + 4 + 4 + 1;
        let camera = 1 + 48 + 4 + 4 + 4 + 12 + 12;
        let input = 4 + 2 + 1 + 1;
        let actor = 2 + 2 + 1 + (2 + 4) + 12 + 4 + 12 + 1 + 4 + 4 + 4 + 4 + 2 + 2 + 1 + 64 + 2;
        let menus = 4 + 4 + 4 + 2 + 4 + 2 + 2 + 2 + 2 + 4 + 4 + 4 + 4 + 4 + 1 + 2 + 1;
        // the effects (default: no running ones, no filters): magic, the
        // two list counts, shake, music
        let effects = 4 + 2 + 2 + 4 + 4 + 4;
        // the look (no lights): valid, the scale, ambient, background, glow,
        // fog, character ambient, the lights' count
        let look = 1 + 12 + 12 + 12 + 16 + 12 + 16 + 12 + 2;
        // the save (default: empty lists): valid, gil, time, five counts
        let save = 1 + 4 + 4 + 2 + 2 + 2 + 2 + 2 + 2;
        // the sphere grid (closed): open, current, cursor, two list counts
        let grid = 1 + 1 + 2 + 2 + 2;
        assert_eq!(
            b.len(),
            8 + 4
                + 8
                + world
                + camera
                + input
                + 2
                + actor
                + menus
                + 2
                + effects
                + 2
                + look
                + save
                + 2
                + grid
        );
    }

    /// FNV-1a of `schema::text()` at [`API_VERSION`].
    const PINNED: u64 = 0xf493_18ba_2217_a2c0;

    /// A field change must raise [`API_VERSION`]: the schema's text is
    /// pinned to the version here.
    #[test]
    fn the_version_pins_the_schema() {
        let text = schema::text();
        let hash = text.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| {
            (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
        });
        assert_eq!(
            (API_VERSION, hash),
            (9, PINNED),
            "the schema changed: raise API_VERSION and pin the new hash"
        );
    }

    #[test]
    fn the_schema_lists_every_field() {
        let s = Snapshot::schema();
        let names: Vec<_> = s.fields.iter().map(|f| f.name).collect();
        assert_eq!(
            names,
            [
                "update",
                "game_time",
                "world",
                "camera",
                "input",
                "actors",
                "menus",
                "messages",
                "effects",
                "battlers",
                "look",
                "save",
                "objects",
                "sphere_grid"
            ]
        );
        assert!(matches!(s.fields[5].ty, Ty::List(_)));
        assert_eq!(structs().len(), 24);
        assert!(Actor::schema().fields.iter().all(|f| !f.doc.is_empty()));
    }
}

#[cfg(test)]
mod doc_tests {
    #[test]
    fn the_doc_lists_the_schema() {
        let doc = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/state-api.md"
        ))
        .unwrap();
        assert!(
            doc.contains(&crate::schema::text()),
            "docs/state-api.md's schema block is not `cargo run -p ffx-state --example schema`'s output"
        );
    }
}
