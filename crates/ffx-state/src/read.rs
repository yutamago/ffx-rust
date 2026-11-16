//! A [`Snapshot`] read from the game's memory. Only reads; every read goes
//! through [`Reader`], which answers `false` where nothing is mapped (the
//! value then reads as 0), so a snapshot can be taken at any time, before
//! the game set anything up too.
//!
//! The addresses are `FFX.exe`'s, with their sources in `symbols/ffx.sym`
//! and docs/formats (chara.md for the characters, save.md for
//! `g_SaveData`); the character record's offsets are `ffx_chara::layout`'s.

use ffx_chara::layout as l;

use crate::{
    Actor, ActorModel, BattleEnd, BattleEndRow, Battler, Bone, Camera, ConfigMenu, ConfigOption,
    ConfigRow, EffectInstance, Effects, GridLink, GridNode, Input, ItemStack, LayerEntry, Layers,
    Light, Look, MenuOption, MenuWindow, Menus, Message, PartyMember, Save, SceneMesh, SceneObject,
    ScreenFilter, Snapshot, SphereGrid, World,
};

/// The game's memory, read only.
pub trait Reader {
    /// Reads `out.len()` bytes at `at`; false (and `out` untouched) where
    /// they are not all mapped.
    fn read_bytes(&self, at: u32, out: &mut [u8]) -> bool;
}

impl<F: Fn(u32, &mut [u8]) -> bool> Reader for F {
    fn read_bytes(&self, at: u32, out: &mut [u8]) -> bool {
        self(at, out)
    }
}

/// `g_SaveData`, the game state block (docs/formats/save.md).
const SAVE_DATA: u32 = 0x0112_ca90;
const SAVE_MAP: u32 = 0;
const SAVE_PREVIOUS_MAP: u32 = 2;
const SAVE_ENTRANCE: u32 = 0xc;
const SAVE_STORY: u32 = 0xbec;
/// `g_EventName`: the running event's name (`char[]`).
const EVENT_NAME: u32 = 0x023c_bc60;
/// Control work 0 (7 works of 0x238 bytes): `+0` flags (bit 1: the player
/// has control, `AtelUcOn`), `+0x1f4` the event's frame count.
const CTRL_WORK: u32 = 0x0132_5ba0;
const CTRL_CONTROL: u8 = 0x02;
const EVENT_FRAME: u32 = 0x0132_5d94;
/// `g_BtlStatus` (`i8`, nonzero in battle).
const BTL_STATUS: u32 = 0x0112_a8e0;
/// `g_PauseOn`.
const PAUSE_ON: u32 = 0x0134_0828;
/// `g_LocalizationManager`: `+4` the text's language (`TOGetFFXLang`).
const LOCALIZATION: u32 = 0x00cd_ed58;
/// `World::language` when the language cannot be read.
pub const LANGUAGE_UNKNOWN: u8 = 0xff;
/// `g_FadeActive` (a bit per fade slot).
const FADE_ACTIVE: u32 = 0x0130_0704;

/// `g_Camera`: the camera channel (`MsCameraGetPosNow`); `+0x50` the point looked at,
/// `+0x61c + 0x50` the eye (`MsCameraGetPosNow`).
const G_CAMERA: u32 = 0x0231_1430;
const CAMERA_TARGET: u32 = 0x50;
const CAMERA_EYE: u32 = 0x61c + 0x50;
/// `g_PhyreScene`: the scene object; `+0xdc` its main camera (a
/// `PCamera`: `+0xbc` far, `+0x150` aspect, `+0x154` vertical FOV),
/// `+0xfa04` the camera's world 3x4 (docs/formats/render.md, "Cameras").
const G_PHYRE_SCENE: u32 = 0x00cc_b9d8;
const SCENE_CAMERA: u32 = 0xdc;
const SCENE_CAMERA_WORLD: u32 = 0xfa04;
/// The scene's current level name: a `PString` (a pointer to the
/// characters; bit 0 a flag, masked off).
const SCENE_LEVEL_NAME: u32 = 0xfaf0;
const PCAMERA_FAR: u32 = 0xbc;
const PCAMERA_ASPECT: u32 = 0x150;
const PCAMERA_FOV_Y: u32 = 0x154;

/// `g_PadHeld`, the field movement's pad (`Sg_ControlMain`); the stick's
/// y and x bytes just below it.
const PAD_HELD: u32 = 0x023c_44ac;
const STICK_Y: u32 = 0x023c_44a4;
const STICK_X: u32 = 0x023c_44a5;
/// `g_PadPressed` (`AtelGetControler`).
const PAD_PRESSED: u32 = 0x0132_70d0;

/// The menus (docs/formats/menu.md, save.md; symbols/ffx.sym).
const MENU_MAIN_MASK: u32 = 0x0184_08ec;
const MENU_MAIN_CURSOR: u32 = 0x0187_1548;
const MENU_MAIN_OPTIONS: u32 = 0x0187_1688;
/// `g_TkMenuMainOptions`: 8-byte entries (`+0` the menu id, `+4` `i16`
/// enabled).
const MENU_MAIN_OPTION_LIST: u32 = 0x0187_1568;
/// At most this many main menu options.
const MAX_OPTIONS: i32 = 32;
/// `TkMenuMainMes`: the main menu's names are menu text 9.
const MENU_TEXT_MAIN: u32 = 9;
/// `g_BtlMenuText`: the menu texts' rows (12 bytes each: `+0` busy, `+8`
/// the table), as `MsMenuGetText` reads them (ffx-battle's `menu_text`).
const MENU_TEXT: u32 = 0x0113_33f0;
const MENU_TEXT_ROW: u32 = 12;
/// `g_SaveMenu`'s word: bit 3 the kana names (`MsGetSaveConfigHiragana`).
const SAVE_MENU: u32 = 0x0113_079c;
const BTL_WINDOW_OPEN: u32 = 0x023c_c08c;
const BTL_WINDOW_CHR: u32 = 0x023c_c088;
const BTL_ACTION_DEPTH: u32 = 0x023c_c092;
/// `g_BtlWindowRecords`: 8 levels of 0xf0 bytes; `+0x20` the ids' list,
/// `+0x42` the cursor, `+0x44` the count.
const BTL_WINDOW_RECORDS: u32 = 0x0133_c950;
const BTL_RECORD: u32 = 0xf0;
const BTL_RECORD_IDS: u32 = 0x20;
const BTL_RECORD_CURSOR: u32 = 0x42;
const BTL_RECORD_COUNT: u32 = 0x44;
/// At most this many ids a command window.
const MAX_IDS: u32 = 128;
const SAVE_STATE: u32 = 0x00ce_72e8;
const SAVE_TOP: u32 = 0x00ce_72ec;
const SAVE_CURSOR: u32 = 0x00ce_72f0;
const SAVE_FILES: u32 = 0x00ce_72f4;
const SAVE_ROWS: u32 = 0x00ce_72f8;
/// `g_FFEscMenu`: up while `+0x70` is set or `+0xe4` is not negative
/// (`graphicIsEscMenuActive`).
const ESC_MENU: u32 = 0x00cc_c870;
/// `g_AtelMsgWindows`: 2 banks x 8 windows of 0x2c bytes; the bank is
/// `g_AtelMenuExecStatus`; `+0`/`+2` x, y, `+8` the text, `+0x14` the
/// state, `+0x16` the flags, `+0x1d` the mode, `+0x1e` the choice.
const MSG_WINDOWS: u32 = 0x0132_6d70;
const MSG_BANK: u32 = 0x0132_6b82;
const MSG_WINDOW: u32 = 0x2c;
/// The text boxes' table (`TOGetTable_01868a90`): 8 pointers, one per
/// script window, to 0x138-byte records (`TOSetExeMessageArea` points them
/// at the current bank's). In a record byte 1 is 2 for a choice and byte 7
/// its option count; the highlighted option is the first byte at `+0x18`
/// (`TOGetTableKind2Data_008b7fd0`, as `Native_MsgSelect_Poll` reads it).
const MSG_TABLE: u32 = 0x0186_8a90;
/// `g_MsgNone`: an empty text.
const MSG_NONE: u32 = 0x0132_5b74;
/// At most this many bytes of a window's text.
const MAX_TEXT: usize = 2048;

/// `g_BtlChrTop`: the battle actors, 0x1f of 0xf90 bytes (`MsGetChr`).
const BTL_CHR_TOP: u32 = 0x0113_34d4;
const BTL_CHR_SIZE: u32 = 0xf90;
const BTL_CHR_COUNT: u32 = 0x1f;

/// `g_MagicFile_MagicId`: the spell playing, -1 none.
const MAGIC_ID: u32 = 0x00c6_4ca0;
/// `g_EtEff`: 512 records of 0x20 bytes, `+0` the id (-1 free; record 0
/// reserved).
const ET_EFF: u32 = 0x012a_4100;
const ET_EFF_COUNT: u32 = 512;
const ET_EFF_SIZE: u32 = 0x20;
/// `g_FadeSlots`: 5 x 0x10.
const FADE_SLOTS: u32 = 0x0130_0708;
/// `g_Screens[0]`'s shake offset.
const SCREEN_SHAKE: u32 = 0x0113_7030;
/// `g_SndNowPlayMusic`.
const MUSIC: u32 = 0x0132_ff5c;

/// `g_ChrCount` (`i32`, slots) and `g_ChrArray` (the records).
const CHR_COUNT: u32 = 0x023c_44e0;
const CHR_ARRAY: u32 = 0x023c_44e4;
/// At most this many slots are read (the game makes a few dozen).
const MAX_SLOTS: u32 = 256;
/// `g_Player`: the player character's record, 0 none.
const PLAYER: u32 = 0x0130_0788;
/// The root's world matrix in a record (`Ch_CalcElement`).
const WORLD_MATRIX: u32 = 0x1d0;
/// The model header's element count (`u16` at `+0xa`, `Ch_GetNbElem`).
const HEADER_ELEMENTS: u32 = 0xa;
/// An element's global matrix (relative to the character).
const ELEM_GLOBAL: u32 = 0x88;
/// At most this many bones a character.
const MAX_BONES: u32 = 512;
/// The model header's model count (`u16` `+6`), skin entries (`u16` `+8`)
/// and skin list (`+0x14`, 12-byte entries, `u16 +2` the model).
const HEADER_MODELS: u32 = 6;
const HEADER_SKINS: u32 = 8;
const HEADER_SKIN_LIST: u32 = 0x14;
const SKIN_ENTRY: u32 = 12;
/// At most this many models and skin entries a character.
const MAX_MODELS: u32 = 64;
const MAX_SKINS: u32 = 256;
/// `+0x198` bit `0x20` (`F2_NO_OEF2`): `Ch_DrawMain` skips the character.
const PURGED: u32 = 0x20;
/// A render instance's byte `+0x14`: visible
/// (`graphicCharacterInstanceSetVisible`).
const INSTANCE_VISIBLE: u32 = 0x14;

struct Mem<'a>(&'a dyn Reader);

/// The reader as ffx-bridge's memory for the game's own read-only ports
/// (`MsMenuGetText`): unmapped bytes read as 0; writes are dropped (those
/// ports never write).
struct ReadOnly<'a>(&'a dyn Reader);

impl ffx_bridge::Mem for ReadOnly<'_> {
    fn read_bytes(&self, addr: u32, out: &mut [u8]) {
        if !self.0.read_bytes(addr, out) {
            out.fill(0);
        }
    }
    fn write_bytes(&mut self, _: u32, _: &[u8]) {}
    fn image_base(&self) -> u32 {
        0x0040_0000
    }
}

impl Mem<'_> {
    fn bytes<const N: usize>(&self, at: u32) -> [u8; N] {
        let mut b = [0u8; N];
        if !self.0.read_bytes(at, &mut b) {
            b = [0; N];
        }
        b
    }
    fn u8(&self, at: u32) -> u8 {
        self.bytes::<1>(at)[0]
    }
    fn u16(&self, at: u32) -> u16 {
        u16::from_le_bytes(self.bytes(at))
    }
    fn i16(&self, at: u32) -> i16 {
        i16::from_le_bytes(self.bytes(at))
    }
    fn u32(&self, at: u32) -> u32 {
        u32::from_le_bytes(self.bytes(at))
    }
    fn i32(&self, at: u32) -> i32 {
        i32::from_le_bytes(self.bytes(at))
    }
    fn f32(&self, at: u32) -> f32 {
        let v = f32::from_le_bytes(self.bytes(at));
        if v.is_finite() { v } else { 0.0 }
    }
    /// `N` floats (at most 16) in one read; non-finite ones read as 0.
    fn floats<const N: usize>(&self, at: u32) -> [f32; N] {
        const { assert!(N <= 16) };
        let mut b = [0u8; 64];
        let n = 4 * N.min(16);
        if !self.0.read_bytes(at, &mut b[..n]) {
            return [0.0; N];
        }
        std::array::from_fn(|i| {
            let v = f32::from_le_bytes([b[4 * i], b[4 * i + 1], b[4 * i + 2], b[4 * i + 3]]);
            if v.is_finite() { v } else { 0.0 }
        })
    }
    /// Whether `LEN` (at most 64) bytes at `at` are mapped.
    fn mapped<const LEN: usize>(&self, at: u32) -> bool {
        const { assert!(LEN <= 64) };
        let mut b = [0u8; LEN];
        self.0.read_bytes(at, &mut b)
    }
    /// The game's encoded text at `at`, up to its 0 (at most [`MAX_TEXT`]
    /// bytes).
    fn text(&self, at: u32) -> Vec<u8> {
        let mut out = Vec::new();
        let mut chunk = [0u8; 64];
        while out.len() < MAX_TEXT {
            let here = at.wrapping_add(u32::try_from(out.len()).unwrap_or(0));
            // the chunk, or byte by byte where it ends in unmapped memory
            if !self.0.read_bytes(here, &mut chunk) {
                let b = self.u8(here);
                if b == 0 {
                    break;
                }
                out.push(b);
                continue;
            }
            match chunk.iter().position(|&b| b == 0) {
                Some(end) => {
                    out.extend_from_slice(&chunk[..end]);
                    break;
                }
                None => out.extend_from_slice(&chunk),
            }
        }
        out.truncate(MAX_TEXT);
        out
    }

    /// A NUL-terminated string of at most `max` bytes (Latin-1 as UTF-8).
    fn c_string(&self, at: u32, max: usize) -> String {
        if at == 0 {
            return String::new();
        }
        let mut s = String::new();
        for i in 0..max {
            let b = self.u8(at.wrapping_add(u32::try_from(i).unwrap_or(0)));
            if b == 0 {
                break;
            }
            s.push(char::from(b));
        }
        s
    }
}

/// The snapshot now: `update` and `game_time` are the host's (the clock's
/// updates run and its time), the rest is read from the game.
#[must_use]
pub fn read(mem: &dyn Reader, update: u32, game_time: f64) -> Snapshot {
    let m = Mem(mem);
    Snapshot {
        update,
        game_time,
        world: world(&m),
        camera: camera(&m),
        input: Input {
            held: m.u32(PAD_HELD),
            pressed: m.u16(PAD_PRESSED),
            stick_x: m.u8(STICK_X),
            stick_y: m.u8(STICK_Y),
        },
        actors: actors(&m),
        menus: menus(&m),
        messages: messages(&m),
        effects: effects(&m),
        battlers: battlers(&m),
        look: look(&m),
        save: save(&m),
        objects: scene_objects(&m),
        sphere_grid: sphere_grid(&m),
        layers: layers(&m),
    }
}

/// The battle's actors taking part (`+0xdc8` `in_battle`), in battle only.
fn battlers(m: &Mem<'_>) -> Vec<Battler> {
    let top = m.u32(BTL_CHR_TOP);
    if top == 0 || m.u8(BTL_STATUS) == 0 {
        return Vec::new();
    }
    (0..BTL_CHR_COUNT)
        .filter_map(|i| {
            let at = top.wrapping_add(i * BTL_CHR_SIZE);
            let f = |o: u32| at.wrapping_add(o);
            if m.u8(f(0xdc8)) == 0 {
                return None;
            }
            Some(Battler {
                index: u8::try_from(i).unwrap_or(0),
                hp: m.i32(f(0x5d0)),
                max_hp: m.i32(f(0x594)),
                mp: m.i32(f(0x5d4)),
                max_mp: m.i32(f(0x598)),
                dead: m.u8(f(0xdcc)) != 0,
                exists: m.u8(f(0xdd2)) != 0,
                ctb: m.i32(f(0x6ec)),
            })
        })
        .collect()
}

fn effects(m: &Mem<'_>) -> Effects {
    let mut running = Vec::new();
    // the whole table in one read when it is mapped
    let mut table = vec![0u8; (ET_EFF_COUNT * ET_EFF_SIZE) as usize];
    if m.0.read_bytes(ET_EFF, &mut table) {
        for (slot, r) in table
            .as_chunks::<{ ET_EFF_SIZE as usize }>()
            .0
            .iter()
            .enumerate()
            .skip(1)
        {
            let id = u32::from_le_bytes([r[0], r[1], r[2], r[3]]);
            // -1 free; 0 in a table not set up yet
            if id == u32::MAX || id == 0 {
                continue;
            }
            let n = usize::from(r[8].min(8));
            running.push(EffectInstance {
                slot: u16::try_from(slot).unwrap_or(0),
                id,
                character: r[0x18],
                kind: r[0x1c],
                ended: r[0x1d] != 0,
                targets: r[0x10..0x10 + n].to_vec(),
            });
        }
    }
    let active = m.u32(FADE_ACTIVE);
    let filters = (0..5u8)
        .map(|i| {
            let at = FADE_SLOTS + u32::from(i) * 0x10;
            ScreenFilter {
                slot: i,
                active: active & (1 << i) != 0,
                mode: m.u8(at),
                color: [m.u8(at + 1), m.u8(at + 2), m.u8(at + 3)],
                frames: m.i16(at + 4),
                level: m.i16(at + 6),
                target: m.i16(at + 8),
                step: m.i16(at + 0xa),
            }
        })
        .collect();
    Effects {
        magic: m.i32(MAGIC_ID),
        running,
        filters,
        shake_x: m.f32(SCREEN_SHAKE),
        shake_y: m.f32(SCREEN_SHAKE + 4),
        music: m.i32(MUSIC),
    }
}

/// `TkMenuMainMes(id)`: `MsMenuGetText(i, id, kana)` as the game runs it
/// on the PC (ffx-battle's `menu_text`; its English rows are never used
/// there): the name's address, 0 while the row is busy or has no table.
fn menu_text(m: &Mem<'_>, i: u32, id: u32) -> u32 {
    let row = MENU_TEXT.wrapping_add(i.wrapping_mul(MENU_TEXT_ROW));
    let table = m.u32(row.wrapping_add(8));
    if table == 0 || m.u8(row) != 0 {
        return 0;
    }
    let kana = (m.u32(SAVE_MENU) >> 3) & 1;
    #[allow(clippy::cast_possible_wrap)] // 12 bits
    let (record, strings) =
        ffx_kernel::bridge::excel_data(&ReadOnly(m.0), (id & 0xfff) as i32, table);
    u32::from(m.u16(record.wrapping_add(kana * 4))).wrapping_add(strings)
}

fn menus(m: &Mem<'_>) -> Menus {
    // level -1: no command window (`TOSetBtlActiveChr`)
    let depth = m.u8(BTL_ACTION_DEPTH).cast_signed();
    let (cursor, ids) = if depth < 0 {
        (0, Vec::new())
    } else {
        let record = BTL_WINDOW_RECORDS + u32::from(depth.min(7).cast_unsigned()) * BTL_RECORD;
        let ids_at = m.u32(record + BTL_RECORD_IDS);
        let count = u32::from(m.u16(record + BTL_RECORD_COUNT)).min(MAX_IDS);
        let mut ids = Vec::new();
        if ids_at != 0 && count > 0 {
            let mut b = vec![0u8; 2 * count as usize];
            if m.0.read_bytes(ids_at, &mut b) {
                ids = b;
            }
        }
        (m.i16(record + BTL_RECORD_CURSOR), ids)
    };
    // an Esc menu object that is not all mapped is not up
    let esc = m.u32(ESC_MENU);
    let esc_up = esc != 0
        && m.mapped::<4>(esc.wrapping_add(0xe4))
        && (m.u8(esc.wrapping_add(0x70)) != 0 || m.i32(esc.wrapping_add(0xe4)) >= 0);
    let options = m.i32(MENU_MAIN_OPTIONS);
    let main_option_list = (0..options.clamp(0, MAX_OPTIONS).cast_unsigned())
        .map(|i| {
            let at = MENU_MAIN_OPTION_LIST + i * 8;
            let id = m.u16(at);
            let name_at = menu_text(m, MENU_TEXT_MAIN, u32::from(id));
            MenuOption {
                id,
                enabled: m.i16(at + 4) != 0,
                name: if name_at == 0 {
                    Vec::new()
                } else {
                    m.text(name_at)
                },
            }
        })
        .collect();
    Menus {
        main_mask: m.u32(MENU_MAIN_MASK),
        main_cursor: m.i32(MENU_MAIN_CURSOR),
        main_options: options,
        main_option_list,
        battle_window: m.u32(BTL_WINDOW_OPEN),
        battle_character: m.i16(BTL_WINDOW_CHR),
        battle_depth: i16::from(depth),
        battle_cursor: cursor,
        battle_ids: ids,
        save_state: m.u32(SAVE_STATE),
        save_cursor: m.i32(SAVE_CURSOR),
        save_top: m.i32(SAVE_TOP),
        save_rows: m.i32(SAVE_ROWS),
        save_files: m.i32(SAVE_FILES),
        esc_menu: esc_up,
        windows: menu_windows(m),
        current_player: menu_current_player(m),
        config: config_menu(m),
        battle_end: battle_end(m),
    }
}

/// The open message windows of the current bank.
fn messages(m: &Mem<'_>) -> Vec<Message> {
    // an i8 the game sets to 0 or 1 (`AtelGetMesWinWork`)
    let bank = u32::from(m.u8(MSG_BANK).cast_signed().clamp(0, 1).cast_unsigned());
    (0..8u8)
        .filter_map(|w| {
            let at = MSG_WINDOWS + (u32::from(w) + bank * 8) * MSG_WINDOW;
            let state = m.u16(at + 0x14);
            if state == 0 {
                return None;
            }
            let text_at = m.u32(at + 8);
            Some(Message {
                window: w,
                state,
                x: m.i16(at),
                y: m.i16(at + 2),
                flags: m.u16(at + 0x16),
                mode: m.u8(at + 0x1d),
                choice: {
                    let rec = m.u32(MSG_TABLE + 4 * u32::from(w));
                    if rec != 0
                        && m.u8(rec.wrapping_add(1)) == 2
                        && m.u8(rec.wrapping_add(7)).cast_signed() > 0
                    {
                        i16::from(m.u8(rec.wrapping_add(0x18)))
                    } else {
                        -1
                    }
                },
                initial_choice: i16::from(m.u8(at + 0x1e).cast_signed()),
                text: if text_at == 0 || text_at == MSG_NONE {
                    Vec::new()
                } else {
                    m.text(text_at)
                },
            })
        })
        .collect()
}

fn world(m: &Mem<'_>) -> World {
    World {
        map: m.u16(SAVE_DATA + SAVE_MAP),
        previous_map: m.u16(SAVE_DATA + SAVE_PREVIOUS_MAP),
        entrance: m.u8(SAVE_DATA + SAVE_ENTRANCE),
        story: m.u16(SAVE_DATA + SAVE_STORY),
        event: m.c_string(EVENT_NAME, 32),
        level: {
            let scene = m.u32(G_PHYRE_SCENE);
            if scene == 0 {
                String::new()
            } else {
                m.c_string(m.u32(scene.wrapping_add(SCENE_LEVEL_NAME)) & !1, 96)
            }
        },
        event_frame: m.u32(EVENT_FRAME),
        battle: m.u8(BTL_STATUS) != 0,
        player_control: m.u8(CTRL_WORK) & CTRL_CONTROL != 0,
        paused: m.u8(PAUSE_ON) != 0,
        fading: m.u32(FADE_ACTIVE) != 0,
        language: {
            // 0xff: unknown (no manager yet, or not a language)
            let lm = m.u32(LOCALIZATION);
            if lm == 0 || !m.mapped::<4>(lm.wrapping_add(4)) {
                LANGUAGE_UNKNOWN
            } else {
                u8::try_from(m.u32(lm.wrapping_add(4)))
                    .ok()
                    .filter(|&l| matches!(l, 0..=5 | 9 | 10))
                    .unwrap_or(LANGUAGE_UNKNOWN)
            }
        },
    }
}

/// The scene's colour scale (`PhyreScene_setColorScale`), its context
/// (`+0xcc`: `GlobalAmbientColor` `+0`, the fog `+0x10`..`+0x2c`) and
/// `g_CharacterLighting` (docs/formats/render.md, "Lights and fog at run
/// time": `+0xf0` the light set, its `+4` the `PLight` pointers (`+0` the
/// colour, `+0x24` the level), `+0xf4 + 0x10 i` the directions, `+0x18c`
/// the scene record (`+0xac` `CharacterAmbientLight`), `+0x1a8 + 0xc i`
/// intensity, pitch, yaw, `+0x1cc` initialised).
const SCENE_COLOUR_SCALE: u32 = 0xfc78;
const SCENE_CONTEXT: u32 = 0xcc;
const CONTEXT_FOG_COLOUR: u32 = 0x10;
const CONTEXT_BACKGROUND: u32 = 0x30;
const CONTEXT_GLOW: u32 = 0x80;
const CONTEXT_FOG_NEAR: u32 = 0x20;
const CONTEXT_FOG_FAR: u32 = 0x24;
const CONTEXT_FOG_CURVE: u32 = 0x28;
const CONTEXT_FOG_LIMIT: u32 = 0x2c;
/// The shaders' `GlobalTime` (`app +0x120`, the context being `app +0x50`).
const CONTEXT_GLOBAL_TIME: u32 = 0xd0;
const CHARACTER_LIGHTING: u32 = 0x00cc_ca84;
const LIGHTING_SET: u32 = 0xf0;
const LIGHTING_DIRECTION: u32 = 0xf4;
const LIGHTING_RECORD: u32 = 0x18c;
const RECORD_AMBIENT: u32 = 0xac;
const LIGHTING_PITCH: u32 = 0x1ac;
const LIGHTING_YAW: u32 = 0x1b0;
const LIGHTING_READY: u32 = 0x1cc;
const PLIGHT_LEVEL: u32 = 0x24;

/// The scene's object tree (`PhyreScene` `+0x1198`; `DynGeoTree`, as
/// `ffx_graphic`'s `tree_find_key` walks it): the head, the root at its
/// `+0xc`, nodes `+0` lower, `+4` higher, `+0xc` the key, `+0x10` the list
/// (`+0` count, `+8` the meshes); a mesh `+4` its world 3x4, `+0x1c` its
/// draw record (whose `+0x1c` is the mesh while drawn), `+0x48` its group id (`m_groupID`).
const SCENE_OBJECTS: u32 = 0x1198;
const SCENE_MAP_CLUSTER: u32 = 0x10d4;
/// At most this many objects and meshes an object: a guard against a tree
/// being rebuilt while read (the stack cap drops the rest of a deeper
/// descent; a valid tree of 512 nodes is under 20 deep, so it is no real
/// depth limit).
const MAX_OBJECTS: usize = 512;
const MAX_MESHES: u32 = 64;
/// The display objects (`g_YiDObjs`, `0xb4` bytes each; the count
/// `g_YiDObjCount`); `+0x88` the morph value (and the UV offset's u),
/// `+0x8c` the UV offset's v.
const DOBJS: u32 = 0x018d_f59c;
const DOBJ_COUNT: u32 = 0x018d_f588;
const DOBJ_SIZE: u32 = 0xb4;
const DOBJ_MORPH: u32 = 0x88;
/// At most this many display objects (the map file's objects; the render
/// state saves 128).
const MAX_DOBJS: u32 = 512;

/// The 2D layers (docs/formats/render.md, scene-maps.md).
const SCENE_ALPHA_FAST: u32 = 0xd4;
const SCENE_2D_LOADED: u32 = 0x93c;
const SCENE_2D_OFF: u32 = 0x93d;
const SCENE_2D_LAYERS: u32 = 0x1128;
const SCENE_2D_COUNT: u32 = 0x1138;
const SCENE_2D_TABLE: u32 = 0x1140;
const SCENE_2D_CURRENT: u32 = 0x11d8;
const SCENE_2D_PAN: u32 = 0xf9bc;
const SCENE_2D_NO_SQUEEZE: u32 = 0xf9c7;
const SCENE_SHIFT: u32 = 0xfa34;
/// `g_PhyreScene3D` (a byte): no squeeze while set.
const PHYRE_SCENE_3D: u32 = 0x00cc_b9c0;
/// At most this many show-table entries.
const MAX_LAYERS: u32 = 256;

fn layers(m: &Mem<'_>) -> Layers {
    let scene = m.u32(G_PHYRE_SCENE);
    let s = |o: u32| scene.wrapping_add(o);
    // the scene mapped from its show table to the current scene, and over
    // the pan to the shift
    if scene == 0
        || !m.mapped::<4>(s(SCENE_2D_CURRENT))
        || !m.mapped::<4>(s(SCENE_2D_PAN))
        || !m.mapped::<8>(s(SCENE_SHIFT))
    {
        return Layers::default();
    }
    let loaded = m.u8(s(SCENE_2D_LOADED)) != 0;
    let off = m.u8(s(SCENE_2D_OFF)) != 0;
    let count = m.u32(s(SCENE_2D_COUNT));
    let table = m.u32(s(SCENE_2D_TABLE));
    let entries = if m.u32(s(SCENE_2D_LAYERS)) == 0 || table == 0 || count > MAX_LAYERS {
        Vec::new()
    } else {
        let mut b = vec![0u8; 8 * count as usize];
        if m.0.read_bytes(table, &mut b) {
            b.as_chunks::<8>()
                .0
                .iter()
                .map(|e| LayerEntry {
                    shown: e[0] != 0,
                    scene: i16::from(e[1].cast_signed()),
                    id: i16::from_le_bytes([e[2], e[3]]),
                })
                .collect()
        } else {
            Vec::new()
        }
    };
    Layers {
        valid: true,
        loaded,
        off,
        drawn: loaded && !off,
        before_world: m.u8(s(SCENE_ALPHA_FAST)) != 0,
        scene: m.i32(s(SCENE_2D_CURRENT)),
        pan: m.f32(s(SCENE_2D_PAN)),
        shift_x: m.f32(s(SCENE_SHIFT)),
        shift_y: m.f32(s(SCENE_SHIFT + 4)),
        squeezed: m.u8(s(SCENE_2D_NO_SQUEEZE)) == 0 && m.u8(PHYRE_SCENE_3D) == 0,
        entries,
    }
}

fn scene_objects(m: &Mem<'_>) -> Vec<SceneObject> {
    let scene = m.u32(G_PHYRE_SCENE);
    if scene == 0 || m.u32(scene.wrapping_add(SCENE_MAP_CLUSTER)) == 0 {
        return Vec::new();
    }
    let head = scene.wrapping_add(SCENE_OBJECTS);
    let dobjs = m.u32(DOBJ_COUNT).min(MAX_DOBJS);
    let mut out = Vec::new();
    // in order, iteratively (lower subtree, node, higher subtree)
    let mut stack = Vec::new();
    let mut node = m.u32(head.wrapping_add(0xc));
    while (node != head && node != 0) || !stack.is_empty() {
        while node != head && node != 0 && stack.len() < MAX_OBJECTS {
            stack.push(node);
            node = m.u32(node);
        }
        let Some(n) = stack.pop() else { break };
        if out.len() >= MAX_OBJECTS {
            break;
        }
        let list = m.u32(n.wrapping_add(0x10));
        let count = if list == 0 {
            0
        } else {
            m.u32(list).min(MAX_MESHES)
        };
        let data = if list == 0 {
            0
        } else {
            m.u32(list.wrapping_add(8))
        };
        let meshes = (0..count)
            .filter_map(|i| {
                let mesh = m.u32(data.wrapping_add(4 * i));
                if mesh == 0 {
                    return None;
                }
                let world = m.u32(mesh.wrapping_add(4));
                let rec = m.u32(mesh.wrapping_add(0x1c));
                Some(SceneMesh {
                    world: if world == 0 {
                        [0.0; 12]
                    } else {
                        m.floats(world)
                    },
                    drawn: rec != 0 && m.u32(rec.wrapping_add(0x1c)) == mesh,
                    layer: m.u32(mesh.wrapping_add(0x48)),
                })
            })
            .collect();
        let id = m.u32(n.wrapping_add(0xc));
        let dobj = id < dobjs;
        let at = DOBJS.wrapping_add(id.wrapping_mul(DOBJ_SIZE));
        out.push(SceneObject {
            id,
            meshes,
            dobj,
            morph: if dobj { m.f32(at + DOBJ_MORPH) } else { 0.0 },
            morph_v: if dobj {
                m.f32(at + DOBJ_MORPH + 4)
            } else {
                0.0
            },
        });
        node = m.u32(n.wrapping_add(4));
    }
    out
}

/// `g_Abmap`: the sphere grid's address (`g_AbmapData`; docs/formats/pc-ui.md),
/// set once and never cleared; `g_AbmapRunning` says whether the screen
/// runs.
const G_ABMAP: u32 = 0x0230_5834;
const G_ABMAP_RUNNING: u32 = 0x01a8_5f70;
const ABMAP_NODES: u32 = 0x808;
const ABMAP_NODE: u32 = 0x28;
const ABMAP_LINKS: u32 = 0xa808;
const ABMAP_LINK: u32 = 0x14;
const ABMAP_CURRENT: u32 = 0x115bc;
const ABMAP_CURSOR: u32 = 0x112fc;
/// At most this many nodes and links: their regions hold 1024 records each
/// (nodes `+0x808..+0xa808`, links `+0xa808..` up to the kind sizes at
/// `+0xf834`); the standard grid has 828 and 834.
const MAX_GRID: u32 = 1024;

fn sphere_grid(m: &Mem<'_>) -> SphereGrid {
    let g = m.u32(G_ABMAP);
    if g == 0 || m.u32(G_ABMAP_RUNNING) == 0 {
        return SphereGrid::default();
    }
    let count = |o: u32| {
        u32::try_from(m.i16(g.wrapping_add(o)).max(0))
            .unwrap_or(0)
            .min(MAX_GRID)
    };
    let (n, l) = (count(2), count(4));
    if n == 0 {
        return SphereGrid::default();
    }
    let nodes = (0..n)
        .map(|i| {
            let at = g.wrapping_add(ABMAP_NODES + ABMAP_NODE * i);
            GridNode {
                x: m.i16(at),
                y: m.i16(at.wrapping_add(2)),
                kind: m.u16(at.wrapping_add(6)),
                flags: m.u16(at.wrapping_add(0x20)),
            }
        })
        .collect();
    let links = (0..l)
        .map(|i| {
            let at = g.wrapping_add(ABMAP_LINKS + ABMAP_LINK * i);
            GridLink {
                a: m.u16(at),
                b: m.u16(at.wrapping_add(2)),
            }
        })
        .collect();
    SphereGrid {
        open: true,
        current: m.u8(g.wrapping_add(ABMAP_CURRENT)),
        cursor: m.u16(g.wrapping_add(ABMAP_CURSOR)),
        nodes,
        links,
    }
}

/// `g_TkMenuPlayers` (the u8 ids the menus cycle through) and
/// `g_TkMenuCurrentPlayerPos` (docs/formats/menu.md "Characters and
/// aeons"): the list is 8 bytes (`+8` is `g_TkMenuPlayerListMax`; at most
/// 7 ids are listed, character 7 never).
const MENU_PLAYERS: u32 = 0x0184_1c14;
const MENU_PLAYERS_LEN: u32 = 8;
const MENU_CURRENT_PLAYER_POS: u32 = 0x0184_1c28;

fn menu_current_player(m: &Mem<'_>) -> u8 {
    let pos = m.u32(MENU_CURRENT_PLAYER_POS);
    if pos < MENU_PLAYERS_LEN {
        m.u8(MENU_PLAYERS + pos)
    } else {
        0xff
    }
}

/// `g_TkMenuWindows`: 32 records of `0x98` bytes (docs/formats/menu.md).
const MENU_WINDOWS: u32 = 0x0184_0900;
const MENU_WINDOW_SIZE: u32 = 0x98;

fn menu_windows(m: &Mem<'_>) -> Vec<MenuWindow> {
    (0..32u32)
        .filter_map(|i| {
            let w = MENU_WINDOWS + MENU_WINDOW_SIZE * i;
            (m.u8(w + 0x40) != 0).then(|| MenuWindow {
                slot: u8::try_from(i).unwrap_or(0),
                state: m.u32(w + 0x28),
                rows: m.u16(w + 0x30),
                first: m.i16(w + 0x32),
                cursor: m.i16(w + 0x48),
                kind: m.u8(w + 0x43),
                args: m.bytes::<20>(w + 0x58).to_vec(),
            })
        })
        .collect()
}

/// The config menu (docs/formats/menu.md "Config menu").
const MENU_CONFIG_MASK: u32 = 0x400;
const MENU_CONFIG_PC_ROW: u32 = 0x0186_a438;
const MENU_CONFIG_CURSOR: u32 = 0x0186_a444;
const MENU_CONFIG_ROWS: u32 = 0x0186_a448;
const MENU_CONFIG_ROW_LIST: u32 = 0x0186_a44c;
/// `TkMenuConfigMes`: the config texts are menu text 7.
const MENU_TEXT_CONFIG: u32 = 7;
/// The PC row's localisation ids (title; option `k`: `0x1d0 - 2 k`).
const CONFIG_PC_TITLE: u16 = 0x1cd;
const CONFIG_PC_OPTION: u16 = 0x1d0;
/// At most this many rows and options a row.
const MAX_CONFIG_ROWS: u32 = 32;
const MAX_CONFIG_OPTIONS: u32 = 16;

fn config_menu(m: &Mem<'_>) -> ConfigMenu {
    if m.u32(MENU_MAIN_MASK) & MENU_CONFIG_MASK == 0 {
        return ConfigMenu::default();
    }
    let pc_row = m.i32(MENU_CONFIG_PC_ROW);
    let list = m.u32(MENU_CONFIG_ROW_LIST);
    let count = u32::try_from(m.i32(MENU_CONFIG_ROWS))
        .unwrap_or(0)
        .min(MAX_CONFIG_ROWS);
    let text = |id: u16| {
        let at = menu_text(m, MENU_TEXT_CONFIG, u32::from(id));
        if at == 0 { Vec::new() } else { m.text(at) }
    };
    let rows = if list == 0 {
        Vec::new()
    } else {
        (0..count)
            .filter_map(|r| {
                let rec = m.u32(list.wrapping_add(4 * r));
                if rec == 0 {
                    return None;
                }
                let at = |o: u32| rec.wrapping_add(o);
                let pc = i64::from(r) == i64::from(pc_row);
                let n = u32::try_from(m.i32(at(4)))
                    .unwrap_or(0)
                    .min(MAX_CONFIG_OPTIONS);
                let options = (0..n)
                    .map(|k| {
                        if pc {
                            #[allow(clippy::cast_possible_truncation)] // k < 16
                            let id = CONFIG_PC_OPTION.wrapping_sub(2 * k as u16);
                            ConfigOption {
                                id,
                                text: Vec::new(),
                            }
                        } else {
                            let id = m.u16(at(0x1c + 4 * k));
                            ConfigOption { id, text: text(id) }
                        }
                    })
                    .collect();
                let title_id = if pc { CONFIG_PC_TITLE } else { m.u16(at(0x18)) };
                Some(ConfigRow {
                    title_id,
                    title: if pc { Vec::new() } else { text(title_id) },
                    value: m.u32(at(8)),
                    enabled: m.u32(at(0xc)) != 0,
                    options,
                })
            })
            .collect()
    };
    ConfigMenu {
        open: true,
        cursor: i16::from(m.u8(MENU_CONFIG_CURSOR).cast_signed()),
        pc_row,
        rows,
    }
}

/// The battle results screen (docs/formats/menu.md "Battle end").
const MENU_BATTLE_END_MASK: u32 = 0x2_0000;
const BTL_END_STATE: u32 = 0x0186_9f08;
const BTL_END_DONE: u32 = 0x00c5_a068;
const BTL_END_GIL: u32 = 0x0186_9f10;
const BTL_END_LIST: u32 = 0x0186_9f18;
const BTL_END_ENTRY: u32 = 14;
/// At most this many rows (the entries end before `0x01869f90`).
const BTL_END_MAX_ROWS: u32 = 8;
const BTL_END_RESULTS: u32 = 0x0231_0f20;
const BTL_END_GIL_WON: u32 = 0x0231_0f6c;
const BTL_END_ITEM_COUNT: u32 = 0x0231_0f70;
const BTL_END_GEAR_COUNT: u32 = 0x0231_0f72;
const BTL_END_ITEM_IDS: u32 = 0x0231_0f74;
const BTL_END_ITEM_COUNTS: u32 = 0x0231_0f84;
const BTL_END_GEAR: u32 = 0x0231_0f8e;
/// 8 items (ids `0x02310f74..0x84`) and 6 gear.
const BTL_END_MAX_ITEMS: u8 = 8;
const BTL_END_MAX_GEAR: u8 = 6;
const BTL_END_GEAR_LIST: u32 = 0x0186_a188;
const BTL_END_GEAR_FULL: u32 = 0x0186_a184;
const BTL_END_TOP: u32 = 0x0186_a190;
const BTL_END_SIDE: u32 = 0x0186_a19c;
const BTL_END_CURSOR: u32 = 0x0186_a1a0;
const BTL_END_PICKED: u32 = 0x0186_a1a8;

fn battle_end(m: &Mem<'_>) -> BattleEnd {
    if m.u32(MENU_MAIN_MASK) & MENU_BATTLE_END_MASK == 0 {
        return BattleEnd::default();
    }
    let n = u32::from(m.u8(BTL_END_LIST)).min(BTL_END_MAX_ROWS);
    let rows = (0..n)
        .map(|i| {
            let e = BTL_END_LIST + 8 + BTL_END_ENTRY * i;
            BattleEndRow {
                character: m.i16(e + 6),
                kind: m.u16(e + 4),
                timer: m.i16(e + 2),
                in_party: m.u8(e + 9) != 0,
                colour: m.u8(e + 8),
                flash: i16::from(m.u8(e + 0xa).cast_signed()),
            }
        })
        .collect();
    // signed bytes: a negative count is none
    let count = |at: u32, max: u8| m.u8(at).cast_signed().max(0).cast_unsigned().min(max);
    let items = (0..count(BTL_END_ITEM_COUNT, BTL_END_MAX_ITEMS))
        .map(|i| ItemStack {
            slot: i,
            id: m.u16(BTL_END_ITEM_IDS + 2 * u32::from(i)),
            count: m.u8(BTL_END_ITEM_COUNTS + u32::from(i)),
        })
        .collect();
    let gear_n = count(BTL_END_GEAR_COUNT, BTL_END_MAX_GEAR);
    BattleEnd {
        open: true,
        state: m.u32(BTL_END_STATE),
        done: m.u8(BTL_END_DONE) != 0,
        gil_won: m.u32(BTL_END_GIL_WON),
        gil_left: m.u32(BTL_END_GIL + 4),
        gil_step: m.u32(BTL_END_GIL),
        ap: m.bytes::<{ 4 * 18 }>(BTL_END_RESULTS).to_vec(),
        rows,
        items,
        gear: m.bytes::<{ 2 * BTL_END_MAX_GEAR as usize }>(BTL_END_GEAR)[..2 * usize::from(gear_n)]
            .to_vec(),
        gear_full: m.u32(BTL_END_GEAR_FULL) != 0,
        side: m.u32(BTL_END_SIDE),
        cursor: m.i32(BTL_END_CURSOR),
        cursor_won: m.i32(BTL_END_CURSOR + 4),
        picked: m.u32(BTL_END_PICKED) != 0,
        top: m.i32(BTL_END_TOP),
        gear_list: m.i32(BTL_END_GEAR_LIST),
    }
}

/// `g_SaveData`'s parts the menus show (docs/formats/save.md).
const SAVE_TIME: u32 = 0xbc;
const SAVE_GIL: u32 = 0x3d48;
const SAVE_FRONTLINE: u32 = 0x3d58;
const SAVE_BACKLINE: u32 = 0x3d5b;
const SAVE_SHARED_COMMANDS: u32 = 0x3d6c;
const SAVE_ITEM_IDS: u32 = 0x3ecc;
const SAVE_ITEM_COUNTS: u32 = 0x40cc;
const SAVE_ITEM_SLOTS: u32 = 256;
const SAVE_KEY_ITEMS: u32 = 0x448c;
const SAVE_PLY: u32 = 0x55cc;
const PLY_SIZE: u32 = 0x94;
const PLY_COUNT: u32 = 18;
const SAVE_NAMES: u32 = 0x634c;
const NAME_SIZE: u32 = 20;

fn save(m: &Mem<'_>) -> Save {
    let s = |o: u32| SAVE_DATA + o;
    if !m.mapped::<4>(s(SAVE_NAMES + NAME_SIZE * PLY_COUNT - 4)) {
        return Save::default();
    }
    let members = (0..PLY_COUNT)
        .map(|i| {
            let p = |o: u32| s(SAVE_PLY + PLY_SIZE * i + o);
            let raw: [u8; NAME_SIZE as usize] = m.bytes(s(SAVE_NAMES + NAME_SIZE * i));
            let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
            PartyMember {
                index: u8::try_from(i).unwrap_or(0),
                name: raw[..end].to_vec(),
                joined: m.u8(p(0x2c)) & 1 != 0,
                hp: m.u32(p(0x1c)),
                mp: m.u32(p(0x20)),
                max_hp: m.u32(p(0x24)),
                max_mp: m.u32(p(0x28)),
                stats: m.bytes::<8>(p(0x2f)).to_vec(),
                ap: m.u32(p(0x18)),
                total_ap: m.u32(p(0x14)),
                sphere_levels: m.u8(p(0x3b)),
                limit_charge: m.u8(p(0x39)),
                limit_charge_max: m.u8(p(0x3a)),
                weapon: m.u8(p(0x2d)),
                armour: m.u8(p(0x2e)),
                commands: m.bytes::<12>(p(0x3e)).to_vec(),
                overdrive_mode: m.u8(p(0x38)),
                overdrive_modes: m.u32(p(0x88)),
            }
        })
        .collect();
    let items = (0..SAVE_ITEM_SLOTS)
        .filter_map(|i| {
            let count = m.u8(s(SAVE_ITEM_COUNTS + i));
            (count > 0).then(|| ItemStack {
                slot: u8::try_from(i).unwrap_or(0),
                id: m.u16(s(SAVE_ITEM_IDS + 2 * i)),
                count,
            })
        })
        .collect();
    Save {
        valid: true,
        gil: m.u32(s(SAVE_GIL)),
        play_time: m.u32(s(SAVE_TIME)),
        frontline: m.bytes::<3>(s(SAVE_FRONTLINE)).to_vec(),
        backline: m.bytes::<17>(s(SAVE_BACKLINE)).to_vec(),
        members,
        items,
        key_items: m.bytes::<16>(s(SAVE_KEY_ITEMS)).to_vec(),
        shared_commands: m.bytes::<56>(s(SAVE_SHARED_COMMANDS)).to_vec(),
    }
}

fn look(m: &Mem<'_>) -> Look {
    let scene = m.u32(G_PHYRE_SCENE);
    let context = if scene == 0 {
        0
    } else {
        m.u32(scene.wrapping_add(SCENE_CONTEXT))
    };
    if context == 0
        || !m.mapped::<0x3c>(context)
        || !m.mapped::<16>(context.wrapping_add(CONTEXT_GLOW))
        || !m.mapped::<16>(scene.wrapping_add(SCENE_COLOUR_SCALE))
    {
        return Look::default();
    }
    let c = |o: u32| context.wrapping_add(o);
    let cl = m.u32(CHARACTER_LIGHTING);
    let ready = cl != 0 && m.u8(cl.wrapping_add(LIGHTING_READY)) != 0;
    let record = if ready {
        m.u32(cl.wrapping_add(LIGHTING_RECORD))
    } else {
        0
    };
    let set = if ready {
        m.u32(cl.wrapping_add(LIGHTING_SET))
    } else {
        0
    };
    let table = if set == 0 {
        0
    } else {
        m.u32(set.wrapping_add(4))
    };
    let lights = if table == 0 {
        Vec::new()
    } else {
        (0..3u32)
            .map(|i| {
                let light = m.u32(table.wrapping_add(4 * i));
                let at = |o: u32| cl.wrapping_add(o).wrapping_add(i * 0xc);
                Light {
                    direction: m.floats(cl.wrapping_add(LIGHTING_DIRECTION + 0x10 * i)),
                    colour: if light == 0 {
                        [0.0; 3]
                    } else {
                        m.floats(light)
                    },
                    level: if light == 0 {
                        0.0
                    } else {
                        m.f32(light.wrapping_add(PLIGHT_LEVEL))
                    },
                    pitch: m.f32(at(LIGHTING_PITCH)),
                    yaw: m.f32(at(LIGHTING_YAW)),
                }
            })
            .collect()
    };
    Look {
        valid: true,
        colour_scale: m.floats(scene.wrapping_add(SCENE_COLOUR_SCALE)),
        ambient: m.floats(context),
        background: m.floats(c(CONTEXT_BACKGROUND)),
        glow_full: m.f32(c(CONTEXT_GLOW)),
        glow_none: m.f32(c(CONTEXT_GLOW + 4)),
        glow_gain: m.f32(c(CONTEXT_GLOW + 8)),
        glow_spread: m.f32(c(CONTEXT_GLOW + 12)),
        fog_colour: m.floats(c(CONTEXT_FOG_COLOUR)),
        fog_near: m.f32(c(CONTEXT_FOG_NEAR)),
        fog_far: m.f32(c(CONTEXT_FOG_FAR)),
        fog_curve: m.f32(c(CONTEXT_FOG_CURVE)),
        fog_limit: m.f32(c(CONTEXT_FOG_LIMIT)),
        character_ambient: if record == 0 {
            [0.0; 3]
        } else {
            m.floats(record.wrapping_add(RECORD_AMBIENT))
        },
        lights,
        global_time: m.f32(c(CONTEXT_GLOBAL_TIME)),
    }
}

fn camera(m: &Mem<'_>) -> Camera {
    let scene = m.u32(G_PHYRE_SCENE);
    let pcamera = if scene == 0 {
        0
    } else {
        m.u32(scene.wrapping_add(SCENE_CAMERA))
    };
    let world_at = scene.wrapping_add(SCENE_CAMERA_WORLD);
    let valid = pcamera != 0 && m.mapped::<48>(world_at);
    let channel = m.u32(G_CAMERA);
    let (eye, target) = if channel == 0 {
        ([0.0; 3], [0.0; 3])
    } else {
        (
            m.floats(channel.wrapping_add(CAMERA_EYE)),
            m.floats(channel.wrapping_add(CAMERA_TARGET)),
        )
    };
    let at = |o: u32| m.f32(pcamera.wrapping_add(o));
    Camera {
        valid,
        world: if valid { m.floats(world_at) } else { [0.0; 12] },
        fov_y: if valid { at(PCAMERA_FOV_Y) } else { 0.0 },
        aspect: if valid { at(PCAMERA_ASPECT) } else { 0.0 },
        far: if valid { at(PCAMERA_FAR) } else { 0.0 },
        eye,
        target,
    }
}

fn actors(m: &Mem<'_>) -> Vec<Actor> {
    let base = m.u32(CHR_ARRAY);
    let count = u32::try_from(m.i32(CHR_COUNT)).unwrap_or(0).min(MAX_SLOTS);
    if base == 0 || count == 0 {
        return Vec::new();
    }
    let player = m.u32(PLAYER);
    let slot_of = |handle: u32| -> i16 {
        if handle < base || !(handle - base).is_multiple_of(l::SIZE) {
            return -1;
        }
        let i = (handle - base) / l::SIZE;
        if i < count {
            i16::try_from(i).unwrap_or(-1)
        } else {
            -1
        }
    };
    (0..count)
        .filter_map(|slot| {
            let at = base.wrapping_add(slot * l::SIZE);
            let f = |o: u32| at.wrapping_add(o);
            if m.u8(f(l::ACTIVE)) == 0 {
                return None;
            }
            let id = m.u16(f(l::ID));
            let flags2 = m.u32(f(l::FLAGS2));
            let frame = m.u32(f(l::MOTION_FRAME));
            #[allow(clippy::cast_precision_loss)] // 8.8 fixed frames
            let motion_frame = frame as f32 / 256.0;
            Some(Actor {
                slot: u16::try_from(slot).unwrap_or(0),
                id,
                kind: u8::try_from(id >> 12).unwrap_or(0),
                model: m.c_string(m.u32(f(l::NAME)), 16),
                position: m.floats(f(l::POS)),
                yaw: m.f32(f(l::ROT)),
                scale: m.floats(f(l::SCALE)),
                hidden: m.u8(f(l::HIDE)),
                alpha: m.f32(f(l::FADE_ALPHA)),
                motion: m.u32(f(l::MOTION_ID)),
                motion_frame,
                motion_speed: f32::from(m.i16(f(l::MOTION_SPEED))) / 256.0,
                parent: slot_of(m.u32(f(l::BIND_PARENT))),
                parent_bone: u16::try_from(m.u32(f(l::BIND_ELEM))).unwrap_or(0),
                player: at == player,
                world: m.floats(f(WORLD_MATRIX)),
                bones: bones(m, at),
                drawn: {
                    let inst = m.u32(f(l::RENDER_INSTANCE));
                    flags2 & PURGED == 0
                        && inst != 0
                        && m.u8(inst.wrapping_add(INSTANCE_VISIBLE)) == 1
                },
                flags: m.u32(f(l::FLAGS)),
                flags2,
                models: models(m, at),
            })
        })
        .collect()
}

/// The record at `at`'s models: each one's base element and whether the
/// header's skin list names it (`Ch_DrawMain`'s walk 1).
fn models(m: &Mem<'_>, at: u32) -> Vec<ActorModel> {
    let header = m.u32(at.wrapping_add(l::MODEL_HEADER));
    let list = m.u32(at.wrapping_add(l::MODELS));
    if header == 0 || list == 0 {
        return Vec::new();
    }
    let n = u32::from(m.u16(header.wrapping_add(HEADER_MODELS))).min(MAX_MODELS);
    let skins = u32::from(m.u16(header.wrapping_add(HEADER_SKINS))).min(MAX_SKINS);
    let skin_list = m.u32(header.wrapping_add(HEADER_SKIN_LIST));
    let mut skinned = vec![false; n as usize];
    if skin_list != 0 && skins > 0 {
        let mut b = vec![0u8; (SKIN_ENTRY * skins) as usize];
        if m.0.read_bytes(skin_list, &mut b) {
            for e in b.as_chunks::<{ SKIN_ENTRY as usize }>().0 {
                if let Some(s) = skinned.get_mut(usize::from(u16::from_le_bytes([e[2], e[3]]))) {
                    *s = true;
                }
            }
        }
    }
    (0..n)
        .map(|i| {
            let model = m.u32(list.wrapping_add(i * l::MODEL_SIZE));
            ActorModel {
                base: if model == 0 { 0 } else { m.u16(model) },
                skinned: skinned[i as usize],
            }
        })
        .collect()
}

/// The record at `at`'s elements, posed.
fn bones(m: &Mem<'_>, at: u32) -> Vec<Bone> {
    let header = m.u32(at.wrapping_add(l::MODEL_HEADER));
    let elems = m.u32(at.wrapping_add(l::ELEMS));
    if header == 0 || elems == 0 {
        return Vec::new();
    }
    let n = u32::from(m.u16(header.wrapping_add(HEADER_ELEMENTS))).min(MAX_BONES);
    (0..n)
        .map(|i| Bone {
            matrix: m.floats(elems.wrapping_add(i * l::ELEM_SIZE + ELEM_GLOBAL)),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// A sparse memory: only the bytes written are mapped.
    struct Sparse(HashMap<u32, u8>);

    impl Sparse {
        fn put(&mut self, at: u32, b: &[u8]) {
            for (i, &v) in b.iter().enumerate() {
                self.0.insert(at.wrapping_add(u32::try_from(i).unwrap()), v);
            }
        }
    }

    impl Reader for Sparse {
        fn read_bytes(&self, at: u32, out: &mut [u8]) -> bool {
            let mut tmp = vec![0u8; out.len()];
            for (i, b) in tmp.iter_mut().enumerate() {
                match self.0.get(&at.wrapping_add(u32::try_from(i).unwrap())) {
                    Some(&v) => *b = v,
                    None => return false,
                }
            }
            out.copy_from_slice(&tmp);
            true
        }
    }

    fn f(v: f32) -> [u8; 4] {
        v.to_le_bytes()
    }

    #[test]
    fn the_look_through_the_scene_and_the_lighting() {
        // no scene: the default
        assert_eq!(read(&Sparse(HashMap::new()), 0, 0.0).look, Look::default());
        // a scene and its context, the lighting not initialised
        let (scene, context, cl) = (0x0100_0000_u32, 0x0110_0000_u32, 0x0120_0000_u32);
        let mut m = Sparse(HashMap::new());
        m.put(G_PHYRE_SCENE, &scene.to_le_bytes());
        m.put(scene + SCENE_CONTEXT, &context.to_le_bytes());
        m.put(
            scene + SCENE_COLOUR_SCALE,
            &[f(0.5), f(0.6), f(0.7), f(1.0)].concat(),
        );
        m.put(context, &[0u8; 0x90]);
        m.put(context + CONTEXT_FOG_NEAR, &f(62.0));
        m.put(
            context + CONTEXT_BACKGROUND,
            &[f(0.3), f(0.35), f(0.5)].concat(),
        );
        m.put(CHARACTER_LIGHTING, &cl.to_le_bytes());
        m.put(cl + LIGHTING_READY, &[0]);
        let l = read(&m, 0, 0.0).look;
        assert!(l.valid);
        assert_eq!(l.colour_scale, [0.5, 0.6, 0.7]);
        assert!((l.fog_near - 62.0).abs() < f32::EPSILON);
        assert_eq!(l.background, [0.3, 0.35, 0.5]);
        assert!(l.glow_gain.abs() < f32::EPSILON);
        // four distinct glow values, each in its field
        m.put(
            context + CONTEXT_GLOW,
            &[f(1.0), f(0.35), f(0.32), f(0.65)].concat(),
        );
        let l = read(&m, 0, 0.0).look;
        assert_eq!(
            [l.glow_full, l.glow_none, l.glow_gain, l.glow_spread],
            [1.0, 0.35, 0.32, 0.65]
        );
        assert_eq!(l.lights, [] as [Light; 0]);
        assert_eq!(l.character_ambient, [0.0; 3]);
        // initialised, with a light set whose second PLight is null
        let (set, table, light, record) = (
            0x0130_0000_u32,
            0x0131_0000_u32,
            0x0132_0000_u32,
            0x0133_0000_u32,
        );
        m.put(cl + LIGHTING_READY, &[1]);
        m.put(cl + LIGHTING_SET, &set.to_le_bytes());
        m.put(set + 4, &table.to_le_bytes());
        m.put(
            table,
            &[light.to_le_bytes(), [0; 4], light.to_le_bytes()].concat(),
        );
        m.put(light, &[f(0.2), f(0.3), f(0.4)].concat());
        m.put(light + PLIGHT_LEVEL, &f(2.1));
        m.put(cl + LIGHTING_RECORD, &record.to_le_bytes());
        m.put(
            record + RECORD_AMBIENT,
            &[f(0.34), f(0.34), f(0.45)].concat(),
        );
        let l = read(&m, 0, 0.0).look;
        assert_eq!(l.lights.len(), 3);
        assert_eq!(l.lights[0].colour, [0.2, 0.3, 0.4]);
        assert!((l.lights[0].level - 2.1).abs() < f32::EPSILON);
        assert_eq!(l.lights[1].colour, [0.0; 3]);
        assert!(l.lights[1].level.abs() < f32::EPSILON);
        assert_eq!(l.character_ambient, [0.34, 0.34, 0.45]);
        // the context mapped only to +0x3c (no glow range): nothing
        let mut short = Sparse(HashMap::new());
        short.put(G_PHYRE_SCENE, &scene.to_le_bytes());
        short.put(scene + SCENE_CONTEXT, &context.to_le_bytes());
        short.put(scene + SCENE_COLOUR_SCALE, &[0u8; 16]);
        short.put(context, &[0u8; 0x3c]);
        assert_eq!(read(&short, 0, 0.0).look, Look::default());
        // a context pointer that is not mapped reads as nothing
        let mut stale = Sparse(HashMap::new());
        stale.put(G_PHYRE_SCENE, &scene.to_le_bytes());
        stale.put(scene + SCENE_CONTEXT, &context.to_le_bytes());
        stale.put(scene + SCENE_COLOUR_SCALE, &[0u8; 16]);
        assert!(!read(&stale, 0, 0.0).look.valid);
    }

    #[test]
    fn the_save_through_g_save_data() {
        let mut m = Sparse(HashMap::new());
        // nothing mapped: not valid
        assert!(!read(&m, 0, 0.0).save.valid);
        // the whole block mapped as zeros, then the fields set
        m.put(SAVE_DATA, &vec![0u8; 0x68c0]);
        m.put(SAVE_DATA + SAVE_GIL, &665u32.to_le_bytes());
        m.put(SAVE_DATA + SAVE_TIME, &4023u32.to_le_bytes());
        m.put(SAVE_DATA + SAVE_FRONTLINE, &[0, 4, 0xff]);
        // member 1: a name with a 0 inside its 20 bytes, HP, stats, joined,
        // a command bit (0x3005) and the overdrive fields
        m.put(SAVE_DATA + SAVE_NAMES + NAME_SIZE, &[0x63, 0x58, 0, 0x53]);
        let p = SAVE_DATA + SAVE_PLY + PLY_SIZE;
        m.put(p + 0x1c, &520u32.to_le_bytes());
        m.put(p + 0x24, &600u32.to_le_bytes());
        m.put(p + 0x2c, &[1]);
        m.put(p + 0x2f, &[15, 10, 5, 5, 10, 18, 10, 10]);
        m.put(p + 0x38, &[2]);
        m.put(p + 0x3e, &[0x20]);
        m.put(p + 0x88, &4u32.to_le_bytes());
        // item slot 3, and a shared command bit (0x3061)
        m.put(SAVE_DATA + SAVE_ITEM_IDS + 6, &0x2001u16.to_le_bytes());
        m.put(SAVE_DATA + SAVE_ITEM_COUNTS + 3, &[3]);
        m.put(SAVE_DATA + SAVE_SHARED_COMMANDS, &[0x02]);
        let s = read(&m, 0, 0.0).save;
        assert!(s.valid);
        assert_eq!((s.gil, s.play_time), (665, 4023));
        assert_eq!(s.frontline, [0, 4, 0xff]);
        assert_eq!(s.members.len(), 18);
        let b = &s.members[1];
        assert_eq!(b.name, [0x63, 0x58]);
        assert!(b.joined && !s.members[0].joined);
        assert_eq!((b.hp, b.max_hp), (520, 600));
        assert_eq!(b.stats, [15, 10, 5, 5, 10, 18, 10, 10]);
        assert_eq!((b.overdrive_mode, b.overdrive_modes), (2, 4));
        assert_eq!(b.commands[0], 0x20);
        assert_eq!(s.shared_commands[0], 0x02);
        assert_eq!(
            s.items,
            [ItemStack {
                slot: 3,
                id: 0x2001,
                count: 3
            }]
        );
    }

    #[test]
    fn the_scene_objects_in_order_with_their_meshes() {
        let scene = 0x0100_0000_u32;
        let head = scene + SCENE_OBJECTS;
        let mut m = Sparse(HashMap::new());
        m.put(G_PHYRE_SCENE, &scene.to_le_bytes());
        // no map cluster: nothing
        m.put(scene + SCENE_MAP_CLUSTER, &0u32.to_le_bytes());
        assert_eq!(read(&m, 0, 0.0).objects, []);
        m.put(scene + SCENE_MAP_CLUSTER, &1u32.to_le_bytes());
        // a tree 5 (root) with 3 lower and 8 higher; leaves point at the head
        let node = |k: u32| 0x0200_0000 + 0x100 * k;
        let mut link = |n: u32, lower: u32, higher: u32, key: u32, list: u32| {
            m.put(n, &lower.to_le_bytes());
            m.put(n + 4, &higher.to_le_bytes());
            m.put(n + 0xc, &key.to_le_bytes());
            m.put(n + 0x10, &list.to_le_bytes());
        };
        let list = 0x0300_0000_u32;
        link(node(5), node(3), node(8), 5, list);
        link(node(3), head, head, 3, 0);
        link(node(8), head, head, 8, 0);
        m.put(head + 0xc, &node(5).to_le_bytes());
        // object 5: two meshes, one drawn (its record points back), one
        // without a record
        let (data, mesh_a, mesh_b, world, rec) = (
            0x0310_0000_u32,
            0x0320_0000_u32,
            0x0330_0000_u32,
            0x0340_0000_u32,
            0x0350_0000_u32,
        );
        m.put(list, &2u32.to_le_bytes());
        m.put(list + 8, &data.to_le_bytes());
        m.put(data, &[mesh_a.to_le_bytes(), mesh_b.to_le_bytes()].concat());
        for (mesh, r) in [(mesh_a, rec), (mesh_b, 0)] {
            m.put(mesh + 4, &world.to_le_bytes());
            m.put(mesh + 0x1c, &r.to_le_bytes());
            m.put(mesh + 0x48, &0x21u32.to_le_bytes());
        }
        m.put(rec + 0x1c, &mesh_a.to_le_bytes());
        m.put(world, &[0u8; 48]);
        m.put(world, &[f(10.0), f(0.0), f(0.0), f(0.0)].concat());
        m.put(world + 32, &[f(1.0), f(2.0), f(3.0)].concat());
        let o = read(&m, 0, 0.0).objects;
        assert_eq!(o.iter().map(|o| o.id).collect::<Vec<_>>(), [3, 5, 8]);
        let five = &o[1].meshes;
        assert_eq!(five.len(), 2);
        assert!(five[0].drawn && !five[1].drawn);
        assert_eq!(five[0].layer, 0x21);
        assert!((five[0].world[0] - 10.0).abs() < f32::EPSILON);
        assert!((five[0].world[9] - 2.0).abs() < f32::EPSILON);
        // a node whose lower link points at itself: bounded, no hang
        let mut looped = Sparse(HashMap::new());
        looped.put(G_PHYRE_SCENE, &scene.to_le_bytes());
        looped.put(scene + SCENE_MAP_CLUSTER, &1u32.to_le_bytes());
        looped.put(head + 0xc, &node(1).to_le_bytes());
        looped.put(node(1), &node(1).to_le_bytes());
        looped.put(node(1) + 4, &head.to_le_bytes());
        looped.put(node(1) + 0xc, &1u32.to_le_bytes());
        looped.put(node(1) + 0x10, &0u32.to_le_bytes());
        assert!(read(&looped, 0, 0.0).objects.len() <= MAX_OBJECTS);
    }

    #[test]
    fn the_sphere_grid_only_while_its_screen_runs() {
        let g = 0x016a_d8b0_u32;
        let mut m = Sparse(HashMap::new());
        m.put(G_ABMAP, &g.to_le_bytes());
        m.put(g, &[0u8; 8]);
        m.put(g + 2, &2i16.to_le_bytes());
        m.put(g + 4, &1i16.to_le_bytes());
        m.put(
            g + ABMAP_NODES,
            &[
                (-5i16).to_le_bytes(),
                7i16.to_le_bytes(),
                [0, 0],
                2u16.to_le_bytes(),
            ]
            .concat(),
        );
        m.put(g + ABMAP_NODES + 0x20, &0x100u16.to_le_bytes());
        m.put(
            g + ABMAP_NODES + ABMAP_NODE,
            &[0, 0, 0, 0, 0, 0, 0xff, 0xff],
        );
        m.put(
            g + ABMAP_LINKS,
            &[0u16.to_le_bytes(), 1u16.to_le_bytes()].concat(),
        );
        m.put(g + ABMAP_CURRENT, &[4]);
        m.put(g + ABMAP_CURSOR, &1u16.to_le_bytes());
        // set up but not running (after the screen closed): nothing
        m.put(G_ABMAP_RUNNING, &0u32.to_le_bytes());
        assert_eq!(read(&m, 0, 0.0).sphere_grid, SphereGrid::default());
        m.put(G_ABMAP_RUNNING, &1u32.to_le_bytes());
        let s = read(&m, 0, 0.0).sphere_grid;
        assert!(s.open);
        assert_eq!((s.current, s.cursor), (4, 1));
        assert_eq!(
            (
                s.nodes[0].x,
                s.nodes[0].y,
                s.nodes[0].kind,
                s.nodes[0].flags
            ),
            (-5, 7, 2, 0x100)
        );
        assert_eq!(s.nodes[1].kind, 0xffff);
        assert_eq!(s.links, [GridLink { a: 0, b: 1 }]);
    }

    #[test]
    fn the_menus_current_player_within_the_list() {
        let mut m = Sparse(HashMap::new());
        m.put(MENU_PLAYERS, &[0, 4, 1, 2, 3, 5, 6, 0]);
        m.put(MENU_PLAYERS + 8, &[2, 0, 0, 0]);
        m.put(MENU_CURRENT_PLAYER_POS, &1u32.to_le_bytes());
        assert_eq!(read(&m, 0, 0.0).menus.current_player, 4);
        // past the 8-byte list (the count behind it is not an id)
        m.put(MENU_CURRENT_PLAYER_POS, &8u32.to_le_bytes());
        assert_eq!(read(&m, 0, 0.0).menus.current_player, 0xff);
    }

    #[test]
    fn nothing_mapped_reads_as_an_empty_world() {
        let s = read(&Sparse(HashMap::new()), 5, 2.0);
        assert_eq!(s.update, 5);
        assert_eq!(
            s.world,
            World {
                language: LANGUAGE_UNKNOWN,
                ..World::default()
            }
        );
        assert!(!s.camera.valid);
        assert_eq!(s.actors.len(), 0);
    }

    #[test]
    fn the_level_name_through_a_tagged_pointer() {
        let mut mem = Sparse(HashMap::new());
        let (scene, text): (u32, u32) = (0x0400_0000, 0x0410_0000);
        mem.put(G_PHYRE_SCENE, &scene.to_le_bytes());
        // a PString: the characters' pointer, bit 0 a flag
        mem.put(scene + SCENE_LEVEL_NAME, &(text | 1).to_le_bytes());
        mem.put(text, b"/map/znkd/znkd06/\0");
        assert_eq!(read(&mem, 0, 0.0).world.level, "/map/znkd/znkd06/");
    }

    #[test]
    fn an_open_message_window_and_the_battle_menu() {
        let mut mem = Sparse(HashMap::new());
        // bank 0, window 2 open with a text
        mem.put(MSG_BANK, &[0]);
        let w = MSG_WINDOWS + 2 * MSG_WINDOW;
        mem.put(MSG_WINDOWS, &vec![0u8; 16 * MSG_WINDOW as usize]);
        let text: u32 = 0x0420_0000;
        mem.put(w + 8, &text.to_le_bytes());
        mem.put(w + 0x14, &2u16.to_le_bytes());
        mem.put(text, &[0x30, 0x31, 0x32, 0x00, 0x99]);
        // the battle window at level 1: three ids, the cursor on the second
        mem.put(BTL_ACTION_DEPTH, &[1]);
        let r = BTL_WINDOW_RECORDS + BTL_RECORD;
        let ids: u32 = 0x0430_0000;
        mem.put(r + BTL_RECORD_IDS, &ids.to_le_bytes());
        mem.put(r + BTL_RECORD_CURSOR, &1u16.to_le_bytes());
        mem.put(r + BTL_RECORD_COUNT, &3u16.to_le_bytes());
        mem.put(ids, &[0x01, 0x30, 0x02, 0x30, 0x03, 0x30]);
        let s = read(&mem, 0, 0.0);
        assert_eq!(s.messages.len(), 1);
        assert_eq!((s.messages[0].window, s.messages[0].state), (2, 2));
        assert_eq!(s.messages[0].text, [0x30, 0x31, 0x32]);
        assert_eq!(s.menus.battle_depth, 1);
        assert_eq!(s.menus.battle_cursor, 1);
        assert_eq!(s.menus.battle_ids, [0x01, 0x30, 0x02, 0x30, 0x03, 0x30]);
    }

    #[test]
    fn the_live_choice_through_the_text_box_table() {
        let mut mem = Sparse(HashMap::new());
        mem.put(MSG_BANK, &[0]);
        mem.put(MSG_WINDOWS, &vec![0u8; 16 * MSG_WINDOW as usize]);
        // window 3 open; its table entry points at a record: a choice of
        // 3 options with the second highlighted
        mem.put(MSG_WINDOWS + 3 * MSG_WINDOW + 0x14, &2u16.to_le_bytes());
        mem.put(MSG_WINDOWS + 3 * MSG_WINDOW + 0x1e, &[0]);
        mem.put(MSG_TABLE, &[0u8; 32]);
        let rec: u32 = 0x0440_0000;
        mem.put(MSG_TABLE + 12, &rec.to_le_bytes());
        mem.put(rec, &[0u8; 0x138]);
        mem.put(rec + 1, &[2]);
        mem.put(rec + 7, &[3]);
        mem.put(rec + 0x18, &[1]);
        let s = read(&mem, 0, 0.0);
        assert_eq!((s.messages[0].window, s.messages[0].choice), (3, 1));
        assert_eq!(s.messages[0].initial_choice, 0);
        // no options: no choice
        mem.put(rec + 7, &[0]);
        assert_eq!(read(&mem, 0, 0.0).messages[0].choice, -1);
    }

    #[test]
    fn the_language_the_save_list_and_the_menus_edge_cases() {
        let mut mem = Sparse(HashMap::new());
        // no localization manager: unknown
        assert_eq!(read(&mem, 0, 0.0).world.language, LANGUAGE_UNKNOWN);
        let lm: u32 = 0x0450_0000;
        mem.put(LOCALIZATION, &lm.to_le_bytes());
        mem.put(lm, &[0u8; 8]);
        mem.put(lm + 4, &0u32.to_le_bytes());
        assert_eq!(read(&mem, 0, 0.0).world.language, 0);
        mem.put(lm + 4, &1u32.to_le_bytes());
        assert_eq!(read(&mem, 0, 0.0).world.language, 1);
        mem.put(lm + 4, &7u32.to_le_bytes());
        assert_eq!(read(&mem, 0, 0.0).world.language, LANGUAGE_UNKNOWN);
        mem.put(lm + 4, &0x1234u32.to_le_bytes());
        assert_eq!(read(&mem, 0, 0.0).world.language, LANGUAGE_UNKNOWN);
        // the save list: the index is top + cursor
        mem.put(SAVE_TOP, &5i32.to_le_bytes());
        mem.put(SAVE_CURSOR, &2i32.to_le_bytes());
        mem.put(SAVE_ROWS, &9i32.to_le_bytes());
        mem.put(SAVE_FILES, &8i32.to_le_bytes());
        let s = read(&mem, 0, 0.0).menus;
        assert_eq!(
            (s.save_top, s.save_cursor, s.save_rows, s.save_files),
            (5, 2, 9, 8)
        );
        // battle level -1: no command window
        mem.put(BTL_ACTION_DEPTH, &[0xff]);
        mem.put(BTL_WINDOW_RECORDS + BTL_RECORD_CURSOR, &4u16.to_le_bytes());
        let s = read(&mem, 0, 0.0).menus;
        assert_eq!((s.battle_depth, s.battle_cursor), (-1, 0));
        assert_eq!(s.battle_ids.len(), 0);
        // an Esc menu object that is not mapped is not up
        mem.put(ESC_MENU, &0x0460_0000u32.to_le_bytes());
        assert!(!read(&mem, 0, 0.0).menus.esc_menu);
        // a bank byte of -1 reads bank 0: its open window is listed
        mem.put(MSG_WINDOWS, &vec![0u8; 16 * MSG_WINDOW as usize]);
        mem.put(MSG_WINDOWS + 0x14, &2u16.to_le_bytes());
        mem.put(MSG_BANK, &[0xff]);
        assert_eq!(read(&mem, 0, 0.0).messages.len(), 1);
    }

    #[test]
    fn garbage_pointers_read_as_nothing() {
        // pointers near the top of the address space: every offset wraps
        let mut mem = Sparse(HashMap::new());
        let top: u32 = 0xffff_fff0;
        mem.put(CHR_ARRAY, &top.to_le_bytes());
        mem.put(CHR_COUNT, &i32::MAX.to_le_bytes());
        mem.put(G_PHYRE_SCENE, &top.to_le_bytes());
        mem.put(G_CAMERA, &top.to_le_bytes());
        mem.put(top, &[1; 16]);
        let s = read(&mem, 0, 0.0);
        assert!(!s.camera.valid);
        // the slot at the top reads as in use; its fields and bones wrap to nothing
        assert!(s.actors.iter().all(|a| a.bones.is_empty()));
    }

    #[test]
    fn the_live_characters_by_slot() {
        let mut mem = Sparse(HashMap::new());
        let base: u32 = 0x0400_0000;
        mem.put(CHR_ARRAY, &base.to_le_bytes());
        mem.put(CHR_COUNT, &3i32.to_le_bytes());
        mem.put(PLAYER, &(base + 2 * l::SIZE).to_le_bytes());
        // three slots of zeros, the second and third in use
        mem.put(base, &vec![0u8; 3 * l::SIZE as usize]);
        for slot in [1u32, 2] {
            let at = base + slot * l::SIZE;
            mem.put(at + l::ACTIVE, &[1]);
            mem.put(
                at + l::ID,
                &(0x0001u16 + u16::try_from(slot).unwrap()).to_le_bytes(),
            );
            mem.put(
                at + l::POS,
                &[1.0f32, -2.0, 3.0].map(f32::to_le_bytes).concat(),
            );
            mem.put(at + l::MOTION_FRAME, &(10 * 256 + 128u32).to_le_bytes());
        }
        // slot 1 is bound to slot 2 (a weapon to its holder)
        mem.put(
            base + l::SIZE + l::BIND_PARENT,
            &(base + 2 * l::SIZE).to_le_bytes(),
        );
        let s = read(&mem, 0, 0.0);
        assert_eq!(s.actors.len(), 2);
        let (a, b) = (&s.actors[0], &s.actors[1]);
        assert_eq!((a.slot, a.id, a.kind), (1, 2, 0));
        assert_eq!(a.position, [1.0, -2.0, 3.0]);
        assert!((a.motion_frame - 10.5).abs() < 1e-6);
        assert_eq!(a.parent, 2);
        assert!(!a.player);
        assert!(b.player);
        assert_eq!(b.parent, -1);
    }
}
