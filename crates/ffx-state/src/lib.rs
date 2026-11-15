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
pub const API_VERSION: u32 = 6;

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
        /// KO'd (`+0xdcc` `stat_death` nonzero).
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
        let menus = 4 + 4 + 4 + 2 + 4 + 2 + 2 + 2 + 2 + 4 + 4 + 4 + 4 + 4 + 1;
        // the effects (default: no running ones, no filters): magic, the
        // two list counts, shake, music
        let effects = 4 + 2 + 2 + 4 + 4 + 4;
        assert_eq!(
            b.len(),
            8 + 4 + 8 + world + camera + input + 2 + actor + menus + 2 + effects + 2
        );
    }

    /// FNV-1a of `schema::text()` at [`API_VERSION`].
    const PINNED: u64 = 0x5730_bd6c_ed24_bf4d;

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
            (6, PINNED),
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
                "battlers"
            ]
        );
        assert!(matches!(s.fields[5].ty, Ty::List(_)));
        assert_eq!(structs().len(), 13);
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
