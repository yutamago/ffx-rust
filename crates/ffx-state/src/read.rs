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
    Actor, Battler, Bone, Camera, EffectInstance, Effects, Input, ItemStack, Light, Look,
    MenuOption, Menus, Message, PartyMember, Save, ScreenFilter, Snapshot, World,
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
const CONTEXT_FOG_NEAR: u32 = 0x20;
const CONTEXT_FOG_FAR: u32 = 0x24;
const CONTEXT_FOG_CURVE: u32 = 0x28;
const CONTEXT_FOG_LIMIT: u32 = 0x2c;
const CHARACTER_LIGHTING: u32 = 0x00cc_ca84;
const LIGHTING_SET: u32 = 0xf0;
const LIGHTING_DIRECTION: u32 = 0xf4;
const LIGHTING_RECORD: u32 = 0x18c;
const RECORD_AMBIENT: u32 = 0xac;
const LIGHTING_PITCH: u32 = 0x1ac;
const LIGHTING_YAW: u32 = 0x1b0;
const LIGHTING_READY: u32 = 0x1cc;
const PLIGHT_LEVEL: u32 = 0x24;

/// `g_SaveData`'s parts the menus show (docs/formats/save.md).
const SAVE_TIME: u32 = 0xbc;
const SAVE_GIL: u32 = 0x3d48;
const SAVE_FRONTLINE: u32 = 0x3d58;
const SAVE_BACKLINE: u32 = 0x3d5b;
const SAVE_ITEM_IDS: u32 = 0x3ecc;
const SAVE_ITEM_COUNTS: u32 = 0x40cc;
const SAVE_ITEM_SLOTS: u32 = 256;
const SAVE_KEY_ITEMS: u32 = 0x448c;
const SAVE_SHARED_COMMANDS: u32 = 0x3d6c;
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
    if context == 0 || !m.mapped::<16>(scene.wrapping_add(SCENE_COLOUR_SCALE)) {
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
            })
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
