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
pub const API_VERSION: u32 = 11;

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
        /// The map's 2D layers (the remaster's painted backgrounds) as the
        /// scene draws them now (docs/formats/scene-maps.md, "How the game
        /// draws the 2D layers"; render.md).
        layers: Layers,
        /// The PC's boosters (F1-F4): the speed-up, the power-up, the
        /// encounter mode and the auto-battle.
        boosters: Boosters,
    }
}

api_struct! {
    /// The PC's boosters as the game keeps them (docs/formats/platform.md,
    /// the keys; render.md, `SpdCtrl_Tick`; `PhyreScene_animate` steps
    /// them, and resets the last three on maps 0x17, 0x15c and 0x15d).
    Boosters {
        /// F1's speed level (`g_SpdCtrl_b4` `0x00ce82b4`): 0 off, 1 x2, 2 x4
        /// (`g_SpdCtrlScales` `0x00c38e1c`).
        speed_level: u8,
        /// The speed-up ran in the last update (`g_SpdCtrl_b8`
        /// `0x00ce82b8`): `SpdCtrl_ScaleElapsedTime` scales only where it
        /// is allowed (the field with the control or a battle, not on a few
        /// maps or under some spells, not in the save screen, the sphere
        /// grid or a movie's fade); while it scales the sound is muted.
        speeding: bool,
        /// The scale it applied (`g_SpdCtrlScales[speed_level]` while
        /// `speeding`, else 1).
        speed: f32,
        /// F2's power-up (`g_BtlCheatPowerup` `0x012fb7cc` nonzero:
        /// `MsCheatPlayerPowerup` on each turn).
        powerup: bool,
        /// F3's encounter mode (`g_EncountMode` `0x00c421d8`): 0 none, 1
        /// normal, 2 ten times as many (`MsBattleEncountExe`).
        encounters: u8,
        /// F4's auto-battle (`i32` `0x0133d6e0` == 1; `forceAutoBattleOff`
        /// clears it).
        auto_battle: bool,
    }
}

api_struct! {
    /// The 2D layers' run-time state (`scene` is `*g_PhyreScene`): which 2D
    /// scene and layers show, and the 16:9 window the layer pass puts over
    /// the 512 x 448 picture.
    Layers {
        /// A scene exists (the values below were read).
        valid: bool,
        /// The 2D level is loaded (byte `scene +0x93c`).
        loaded: bool,
        /// The pass is switched off (byte `+0x93d`: set by
        /// `PhyreScene_fieldMapInitialize` when a layer object's `+0x5c` lies
        /// in (0, 1), e.g. bsil00's one layer).
        off: bool,
        /// The layer pass runs (`PhyreScene_render`: `loaded` and not `off`).
        drawn: bool,
        /// The layers are drawn before the 3D world (byte `+0xd4`, "map
        /// alpha fast"; clear: after it, then the world's `Transparent`
        /// pass).
        before_world: bool,
        /// The 2D scene shown (`+0x11d8`; a map starts on 0, scripts change
        /// it with `PhyreScene_change2DSceneNum`): the layers whose mesh
        /// name holds `scene<n>`.
        scene: i32,
        /// The window's pan `b` (`f32` `+0xf9bc`): after the squeeze the
        /// layer camera moves by `1/3 - 2 b`, so the top PS2 row shown is
        /// `112 - 336 b`.
        pan: f32,
        /// The window's shift (`+0xfa34`, `+0xfa38`), added to the layer
        /// camera's matrix before the squeeze.
        shift_x: f32,
        /// See `shift_x`.
        shift_y: f32,
        /// The window is squeezed to 16:9 (y x 4/3, then the pan): unless
        /// byte `+0xf9c7` or `g_PhyreScene3D` (`0x00ccb9c0`) is set.
        squeezed: bool,
        /// `PhyreScene_show2DLayer`'s table (`+0x1140`, `+0x1138` entries of
        /// 8 bytes) while `+0x1128` is set: the layers scripts show and
        /// hide one by one; a layer not listed is shown.
        entries: Vec<LayerEntry>,
    }
}

api_struct! {
    /// One entry of the 2D layers' show table.
    LayerEntry {
        /// Shown (`+0`).
        shown: bool,
        /// Its 2D scene (`+1`, a signed byte).
        scene: i16,
        /// The layer's id (`i16` `+2`): `scene<scene>layer<id>` in the 2D
        /// model.
        id: i16,
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
        /// `Sg_DrawFilter` drew the slot as a fade task in the update just
        /// run: its `g_FadeActive` bit is set (`Sg_DrawFilter` clears it
        /// when it finds no frames left) and its mode is 2..8; mode 0, the
        /// cross-fade, draws its level quad instead (`level`), mode 1
        /// nothing.
        drawn: bool,
        /// The alpha it drew, by the integer formulas of `Sg_DrawFilter`
        /// (`0x0082c0c0`), which runs after the update and steps `frames`
        /// down: with `n = frames` (what is left after its step) and `t =
        /// level` (the total), mode 2 `1 - (127 - n 127 / t) / 127`; 3 `1 -
        /// (n 127 / t) / 127`, 1 once held; 4 `((n + 1) 255 / t) / 127` (up
        /// to 2: a frontend clamps it); 5 `(255 - n 255 / t) / 127`, 255 /
        /// 127 once held; 6 `((n + 1) 127 / t) / 127`; 7 and 8 `level / 127`
        /// (`/` integer division, then the `f64` 127 as the game does). A
        /// held fade (modes 3 and 5) keeps `frames` at 1, which its last
        /// step leaves too: `frames` 1 gives the hold's alpha, so the one
        /// frame before the hold reads as held. 0 when not `drawn`.
        alpha: f32,
        /// The colour it drew: black for modes 2 and 3, else `color / 256`
        /// (the `f64` 1/256 the game multiplies by).
        draw_colour: [f32; 3],
        /// Drawn subtracting (mode 8: `graphicAddScreenFadeTask`'s flag
        /// `0x10`), else blended over the picture.
        subtract: bool,
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
        /// The config menu (menu slot 10, `main_mask` bit `0x400`).
        config: ConfigMenu,
        /// The battle results screen (menu slot 0x11, `main_mask` bit
        /// `0x20000`).
        battle_end: BattleEnd,
        /// The battle command window's target selection and hand cursors.
        battle_target: BattleTarget,
    }
}

api_struct! {
    /// The battle command window's target selection (`TOBtlGetTargetPad`
    /// `0x0089d390`, `TOBtlMakeTargetTargetBit` `0x00898640`) and the hand
    /// cursors the battle windows put on the screen (`TOBtlSetCurPos`).
    BattleTarget {
        /// The command window's level chooses a target (`kind` 1).
        open: bool,
        /// The level's kind, as `TOBtlCtrlHelpWin` reads it: byte `+2` of its
        /// record (`g_BtlWindowRecords` + 0xf0 x `g_BtlActionDepth`): 0 a
        /// command list, 1 the target; 0xff at level -1.
        kind: u8,
        /// The character the target cursor is on (`g_BtlTargetChr`
        /// `0x0133d1f4`: the highest bit of `mask` below 28; 0..0x11 the
        /// party, 0x14..0x1b the monsters).
        character: u16,
        /// The targets selected (`g_BtlTargetSelMask` `0x0133d178`), a bit
        /// per character: `TOBtlDrawTargetCursor` puts a target cursor over
        /// each bit below 28 (`MsGetChrTargetPos`; none while the
        /// auto-battle runs).
        mask: u32,
        /// The cursor's index in its side's list (`i16` `0x0133d1fc`).
        cursor: i16,
        /// That list's length (`u16` `0x0133d1f6`).
        count: u16,
        /// The side the list is (`u16` `0x0133d200`, which the pad swaps
        /// while `switchable`): 1 the monsters' (`0x0133d1d4`), 0 the
        /// party's (`0x0133d180`); observed: Attack opens on side 1.
        side: u16,
        /// The side can be swapped (`0x0133d1fe` == 1).
        switchable: bool,
        /// The hand cursors the last draw put on the screen (`g_BtlCurPos`
        /// `0x0133f7b4`, 8 bytes each, pushed by `TOBtlSetCurPos` while the
        /// windows draw; `TOBtlDrawCursors` draws them and clears their
        /// count, `g_BtlCurPosCount` `0x0133d6f0`, so between two updates
        /// the records are read: as many as the count says, at least the
        /// first, at most 4), but zeroed ones (observed while the target is
        /// chosen, which draws the target cursor instead); none at level -1.
        hands: Vec<HandCursor>,
    }
}

api_struct! {
    /// A battle hand cursor (a `g_BtlCurPos` record), on the PS2 screen
    /// (512 x 448; `TOMakePktShapeCursor(kind, x, y)`).
    HandCursor {
        /// `i16 +0`: `TOBtlSetCurPos`'s first argument.
        kind: i16,
        /// `i16 +4`.
        x: i16,
        /// `i16 +6`.
        y: i16,
    }
}

api_struct! {
    /// The config menu (`TkMenuCtrlConfig`, docs/formats/menu.md "Config
    /// menu"), read while it is open.
    ConfigMenu {
        /// It is open (`main_mask` bit `0x400`; the values below were
        /// read).
        open: bool,
        /// The cursor's row (`i8` `0x0186a444`).
        cursor: i16,
        /// The PC's own row (the remaster's settings, `0x0186a438`): its
        /// texts come from the localisation manager, not the menu text.
        pc_row: i32,
        /// The rows (`0x0186a44c` a list of record pointers, the count
        /// `0x0186a448`).
        rows: Vec<ConfigRow>,
    }
}

api_struct! {
    /// One config menu row (its record: `MenuConfig_DrawOptionRow`).
    ConfigRow {
        /// The title's text id (`u16 +0x18`; on the PC row the
        /// localisation id `0x1cd`).
        title_id: u16,
        /// The title in the game's encoding (`MsMenuGetText(7, id, kana)`,
        /// `TkMenuConfigMes`); empty on the PC row.
        title: Vec<u8>,
        /// The value chosen (`+8`): an index into `options`.
        value: u32,
        /// It can be changed (`+0xc`; 0 dims the options).
        enabled: bool,
        /// The options (`+4` the count; text ids `u16` every 4 bytes from
        /// `+0x1c`).
        options: Vec<ConfigOption>,
    }
}

api_struct! {
    /// One option of a config row.
    ConfigOption {
        /// Its text id (on the PC row the localisation id `0x1d0 - 2 k`,
        /// its help `0x1d1 - 2 k`).
        id: u16,
        /// Its text in the game's encoding (`TkMenuConfigMes`); empty on
        /// the PC row.
        text: Vec<u8>,
    }
}

api_struct! {
    /// The battle results screen (`TkBtlEndCtrl`, docs/formats/menu.md
    /// "Battle end"): AP, gil, items and gear won. FFX has no EXP.
    BattleEnd {
        /// It runs (`main_mask` bit `0x20000`; the values below were read).
        open: bool,
        /// The screen's state (`g_BtlEnd_State` `0x01869f08`, 0..0x2b):
        /// level-ups, the AP count, the gil count, the spoils, the end.
        state: u32,
        /// The control is done (`g_BtlEndCtrlDone`, byte `0x00c5a068`).
        done: bool,
        /// The gil won (`g_TkBtlEndGilWon` `0x02310f6c`).
        gil_won: u32,
        /// The gil still to count into the party's (`g_TkBtlEndGil` `+4`;
        /// each step pays `gil_step` with `MsPayGIL`).
        gil_left: u32,
        /// See `gil_left` (`+0`).
        gil_step: u32,
        /// The AP each character still has to count (`g_TkBtlEndResults`
        /// `0x02310f20`, 18 `u32` little-endian, counted down to 0).
        ap: Vec<u8>,
        /// The results rows (`g_TkBtlEndList`, 14-byte entries from
        /// `0x01869f20`, the count byte `0x01869f18`).
        rows: Vec<BattleEndRow>,
        /// The items won (`i8` count `0x02310f70`, `u16` ids `0x02310f74`,
        /// `u8` counts `0x02310f84`); `slot` is the row.
        items: Vec<ItemStack>,
        /// The gear won (`i8` count `0x02310f72`, up to 6 `u16` ids at
        /// `0x02310f8e`, `MsGetSaveWeapon`'s), `u16` little-endian each.
        gear: Vec<u8>,
        /// The gear doesn't fit (`0x0186a184`): the exchange pages.
        gear_full: bool,
        /// The spoils page's side (`0x0186a19c`).
        side: u32,
        /// The cursors of the two sides (`0x0186a1a0 + 4 side`; side 0's
        /// relative to `top`).
        cursor: i32,
        /// See `cursor`.
        cursor_won: i32,
        /// A pick is held (`0x0186a1a8`).
        picked: bool,
        /// The gear list's first row shown (`0x0186a190`) and its length
        /// (`0x0186a188`).
        top: i32,
        /// See `top`.
        gear_list: i32,
        /// The gear won, named: each `gear` id's record as
        /// `MsGetSaveWeapon` finds it (`0x7000 + i` at `0x02310f9e`,
        /// `0xb000 + i` at `0x02310dde`, else `g_Equipment`; 22 bytes each,
        /// an index out of range record 0) and the name
        /// `MsGetSaveWeaponName` gives it.
        gear_spoils: Vec<GearSpoil>,
    }
}

api_struct! {
    /// One piece of gear won (its 22-byte record; docs/formats/kernel.md
    /// "Equipment").
    GearSpoil {
        /// Its id (`gear`'s).
        id: u16,
        /// The record is in use (`+2` nonzero; the results screen draws
        /// only those).
        used: bool,
        /// Its `w_name` record (`u16 +0`), named by the kernel's
        /// `w_name` table (`MsWeaponName`).
        name_id: u16,
        /// Its character (`+4`, 0..6 the party): the record's string.
        character: u8,
        /// The config's hiragana (`g_SaveMenu` bit 3): the second set of
        /// names.
        kana: bool,
        /// The name as the game gives it (`MsGetSaveWeaponName`), in the
        /// game's encoding; empty for a record not in use.
        name: Vec<u8>,
    }
}

api_struct! {
    /// One battle results row (a character's AP).
    BattleEndRow {
        /// The character (`i16 +6`).
        character: i16,
        /// Its kind (`u16 +4`: 0 and 5 not drawn, 1 sliding in, 4 out).
        kind: u16,
        /// Its slide timer (`i16 +2`, 0..0x1000).
        timer: i16,
        /// In the front line (`+9`).
        in_party: bool,
        /// The name's colour class (`+8`, `TkMenuGetPlayerDamegedColor`).
        colour: u8,
        /// The level-up flash counter (`i8 +0xa`: 0xf when the AP reach a
        /// sphere level, down by one each draw).
        flash: i16,
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
        /// The option the cursor is on now, from the first option (0): the
        /// text box's record (`0x01868a90`'s table, 0x138 bytes) `i8 +2`
        /// less its first row `i8 +4`, which `TOFuncPadMessageSelect`
        /// moves while the choice is open and leaves on the pick; -1 when
        /// the window lays out no options (`+5` not above `+4`).
        choice: i16,
        /// The choice the script set it to at the start (`+0x1e`).
        initial_choice: i16,
        /// The text in the game's encoding (ffx-text's charset decodes
        /// it), its macros expanded as the game draws them
        /// (`TOGetChangeMacroAddress`, recursively): `{VAR:n}` (code
        /// `0x12`) from the window's variables (`vars`), the characters'
        /// names (`0x13`), items (`0x16`, `0x17`) and key items (`0x23`);
        /// the dictionary codes (`0x19`..`0x22`) and anything that cannot be
        /// read stay as their codes.
        text: Vec<u8>,
        /// The option picked, once the player confirmed it: the record's
        /// first pick (`+0x18`, what `TOGetTableKind2Data` gives
        /// `Native_MsgSelect_Poll`, the row as the cursor `+2` had it) after
        /// `+1` is 2 (`TOFuncPadMessageSelect` sets it when every pick is
        /// made; `+7` picks made); else -1.
        picked: i16,
        /// The window's text variables that are set (`g_MsgVars`
        /// `0x01869ab0`, 32 pointers a window to 8-byte records), the raw
        /// values with what each expands to.
        vars: Vec<MessageVar>,
    }
}

api_struct! {
    /// One text variable of a message window (`g_MsgVars`; set by the
    /// script natives `0x009e`..`0x00a0`, `TOMacroSetType`/`SetValue`;
    /// docs/formats/text.md, code `12`).
    MessageVar {
        /// Its slot (0..31): `{VAR:n}` is code `0x12`, `0x30 + n`.
        slot: u8,
        /// Its kind as the text reads it (`+1`; `TOResetMessageVars`
        /// copies the script's `+0` there): 0 a text at `value`, 1 a
        /// number, 2 a number with digit groups, `0x13`..`0x23` a macro
        /// code (`value / 0xd0` added to the code, `value % 0xd0` its
        /// argument), `0x2b`..`0x32` a number of `kind - 0x2a` digits, `0x33`
        /// an item's name, `0x34` a weapon's, `0x35` a command's, `0x36` an
        /// ability's, `0x37` the field pickup (`value` a treasure, `takara`).
        kind: u8,
        /// Its value (`i32 +4`).
        value: i32,
        /// What `{VAR:n}` expands to, in the game's encoding (empty when
        /// it expands to nothing).
        text: Vec<u8>,
        /// For `0x37`: the treasure's kind (`takara` record `+0`: 2 an
        /// item, 5 gear, 10 a key item, others gil; `msFieldItemGet`), else
        /// 0xff.
        pickup: u8,
        /// For `0x37`: the item or key item id (record `u16 +2`), or the
        /// gear's id in the pickup's spoils (`0x02310dce`, a `0xb000` id).
        pickup_id: u16,
        /// For `0x37`: the items' count (record `+1`), or the gil (`+1` x
        /// 100).
        pickup_count: u32,
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
        /// The game holds its update for a load (`Sg_IsSceneLoading`
        /// `0x0081fcd0`, which `Sg_MainLoop` asks): the field map loads
        /// outside a battle map (`map_loading`), characters load or
        /// instantiate on one of 16 listed maps outside a battle map, or
        /// the scene's assets are not all in (`assets_loading`) while the
        /// screen fade's colour is locked.
        scene_loading: bool,
        /// The field map loads (`graphicIsFieldMapLoading`: `PhyreScene`
        /// `+0x1195`).
        map_loading: bool,
        /// The character loads and instantiations waiting
        /// (`graphicIsCharacterLoadingOrInstanciating`:
        /// `g_GraphicCharacterLoading` `0x00ccb6d4` & `0x7fffffff`).
        characters_loading: u32,
        /// The asset loader has requests left (`graphicIsAllSceneAssetsLoaded`
        /// false: `g_AsyncLoadManager` `+0x20` or `+0x2c` nonzero).
        assets_loading: bool,
        /// A map change is under way (`g_TkEventJumpRequest` `0x0133084c`:
        /// `TkSetEventJump`, from `AtelSetEventJump`/`2` and the script's map
        /// changes, sets it after writing the new map and entrance to
        /// `g_SaveData`, so `map` and `entrance` are already the target;
        /// `TkInitScene` clears it when the map starts).
        map_change: bool,
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
        /// The screen shift (`PhyreScene` `+0xfa34`, `+0xfa38`), added to
        /// the projected x and y (`PhyreScene_getViewProjectionMatrix`
        /// `0x0065e660`, always; docs/state-api.md "Camera adjustment").
        shift_x: f32,
        /// See `shift_x`.
        shift_y: f32,
        /// The 16:9 window's pan `b` (`+0xf9bc`; also `Layers.pan`).
        pan: f32,
        /// The map's own term of the main camera's offset (`+0xf9c0`, set
        /// per map by `PhyreScene_fieldMapInitialize`), which the layer
        /// pass leaves out.
        map_pan: f32,
        /// The main camera gets the screen adjustment
        /// (`PhyreScene_GetScreenAdjustedView` `0x00651650`): neither byte
        /// `+0xf9c7` nor `g_PhyreScene3D` (`0x00ccb9c0`) is set.
        squeezed: bool,
        /// `g_PhyreScene3D`: a 3D map (`PhyreScene_fieldMapLoad` sets it
        /// for 3D maps outside the 2D list). Clear in the 2D-background
        /// rooms, whose camera keeps the PS2's 4/3 `aspect` on the 16:9
        /// target (`PhyreScene_setCameraAspectRatio`).
        scene_3d: bool,
        /// The adjustment's y offset `d = 0.33333337 - 2 (map_pan + pan)`
        /// (computed in `f64` from the two floats, stored as an `f32`, as
        /// the game does), 0 when not `squeezed`.
        offset_y: f32,
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
        /// The id is a display object's (`g_YiDObjs` `0x018df59c`, `0xb4`
        /// bytes each, the count `0x018df588`; `yiRender_draw` hands object
        /// `i` to `graphicObjectSetTranslation(i, ..)`): the two values
        /// below were read.
        dobj: bool,
        /// The display object's `+0x88`: the morph value
        /// (`yiSetMIMeParamDObj`, handed to
        /// `graphicSetMeshVertexColorMorphValue`; weights `1 - w`, `w`), or
        /// the UV offset's u that a keyframe track of kind 2 sets
        /// (`graphicSetMaterialUV_006436e0`).
        morph: f32,
        /// `+0x8c`: that UV offset's v.
        morph_v: f32,
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
        /// The shaders' `GlobalTime`, seconds (context `+0xd0`, that is
        /// `*g_PhyreApplication +0x120`: `FFXApplication +0x3b0`'s copy):
        /// the clock of scrolling UVs (`phyreuvanimationshader`'s
        /// `ScrollSpeed x GlobalTime`) and the water; it advances while the
        /// game runs (not loading, not paused), wraps to 0 at 2400 and is 0
        /// again after every map change (docs/harness.md, the section on
        /// `GlobalTime` and the title band).
        global_time: f32,
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
        /// The game draws it, as the last character draw (`Ch_DrawMain`,
        /// read at the next frame boundary) left it: its render instance
        /// (`+0x830`) is visible
        /// (byte `+0x14`, which `graphicCharacterInstanceSetVisible` sets
        /// in `Ch_DrawMain` after the hide byte, the distance cull, the
        /// alpha and the special cases) and the model is loaded (`flags2`
        /// bit `0x20` clear).
        drawn: bool,
        /// The flag words `+0x194` (bit `0x2000`, `F_DRAW_EFFECT`: skips
        /// walk 2's `|= 0x50` and hides the character only under magic
        /// `0x280`/`0x245`, not a "not drawn" bit; bit `0x800`: the distance
        /// cull applies) and `+0x198` (bit `0x20`, `F2_NO_OEF2`: skipped by
        /// the OEF2 lookup and by `Ch_DrawMain`; `0x50` set by walk 2 before
        /// the cull and the hide byte, so culled characters carry it too).
        flags: u32,
        /// See `flags`.
        flags2: u32,
        /// The models (`+0x1bc`, `0x38` bytes each; the count `u16` at the
        /// header's `+6`): how `Ch_DrawMain` places each (walk 1).
        models: Vec<ActorModel>,
    }
}

api_struct! {
    /// One model (part) of a character.
    ActorModel {
        /// Its base element (`u16` at `*(model +0)`): an unskinned model's
        /// world matrix is the root's `world` (x 1000 scale, as `bones`)
        /// times this element's `matrix`
        /// (`graphicCharacterInstanceSetWorldMatrixByMdlID`).
        base: u16,
        /// It is skinned (named by an entry of the header's skin list,
        /// `+0x14`, 12-byte entries, `u16 +2` the model; the count `u16`
        /// `+8`): it takes the root's `world` whole.
        skinned: bool,
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
        ActorModel::schema(),
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
        ConfigMenu::schema(),
        ConfigRow::schema(),
        ConfigOption::schema(),
        BattleEnd::schema(),
        BattleEndRow::schema(),
        Layers::schema(),
        LayerEntry::schema(),
        Boosters::schema(),
        BattleTarget::schema(),
        HandCursor::schema(),
        GearSpoil::schema(),
        MessageVar::schema(),
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
        // (API 11: the loading flags, the characters' count, the map change)
        let world = 2 + 2 + 1 + 2 + 2 + 2 + 4 + 4 + 1 + (1 + 1 + 4 + 1 + 1);
        // (API 11: shift, pan, the map's pan, squeezed, 3D, the offset)
        let camera = 1 + 48 + 4 + 4 + 4 + 12 + 12 + (4 + 4 + 4 + 4 + 1 + 1 + 4);
        let input = 4 + 2 + 1 + 1;
        // (no bones; drawn, the two flag words, no models)
        let actor = 2
            + 2
            + 1
            + (2 + 4)
            + 12
            + 4
            + 12
            + 1
            + 4
            + 4
            + 4
            + 4
            + 2
            + 2
            + 1
            + 64
            + 2
            + 1
            + 4
            + 4
            + 2;
        // the config menu (closed: open, cursor, the PC row, no rows) and
        // the battle end (closed: its scalars and four empty lists)
        let config = 1 + 2 + 4 + 2;
        // (API 11: the named gear, empty)
        let battle_end = 1 + 4 + 1 + 4 + 4 + 4 + 2 + 2 + 2 + 2 + 1 + 4 + 4 + 4 + 1 + 4 + 4 + 2;
        // the target selection (none: its scalars, no hands)
        let target = 1 + 1 + 2 + 4 + 2 + 2 + 2 + 1 + 2;
        let menus = 4
            + 4
            + 4
            + 2
            + 4
            + 2
            + 2
            + 2
            + 2
            + 4
            + 4
            + 4
            + 4
            + 4
            + 1
            + 2
            + 1
            + config
            + battle_end
            + target;
        // the effects (default: no running ones, no filters): magic, the
        // two list counts, shake, music
        let effects = 4 + 2 + 2 + 4 + 4 + 4;
        // the look (no lights): valid, the scale, ambient, background, glow,
        // fog, character ambient, the lights' count, the global time
        let look = 1 + 12 + 12 + 12 + 16 + 12 + 16 + 12 + 2 + 4;
        // the save (default: empty lists): valid, gil, time, five counts
        let save = 1 + 4 + 4 + 2 + 2 + 2 + 2 + 2 + 2;
        // the sphere grid (closed): open, current, cursor, two list counts
        let grid = 1 + 1 + 2 + 2 + 2;
        // the 2D layers (no scene): the flags, scene, pan, shift, squeezed,
        // no entries
        let layers = 1 + 1 + 1 + 1 + 1 + 4 + 4 + 8 + 1 + 2;
        // the boosters: level, speeding, the scale, power-up, encounters,
        // auto-battle
        let boosters = 1 + 1 + 4 + 1 + 1 + 1;
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
                + layers
                + boosters
        );
    }

    /// FNV-1a of `schema::text()` at [`API_VERSION`].
    const PINNED: u64 = 0x50fc_4bd9_0abc_f2fd;

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
            (11, PINNED),
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
                "sphere_grid",
                "layers",
                "boosters"
            ]
        );
        assert!(matches!(s.fields[5].ty, Ty::List(_)));
        assert_eq!(structs().len(), 37);
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
