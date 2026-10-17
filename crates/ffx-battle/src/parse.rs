//! A command taking effect (`docs/rewrite-plan.md`, M2): the command cue's
//! first entry chosen and started (`MsCommandCueProcess`, `0x00792210`),
//! its command parsed into the actor's motions, effect and cameras
//! (`MsParseCommand`), and its magic effect file read (`MsEffectRead`).
//!
//! The effect files are system's and vfx's (`MagicFile_*`, the overlay
//! effects, the CD library): those go to the game's code through `s.call`.

/// The game's memory, through [`ffx_bridge::Mem`].
#[allow(clippy::many_single_char_names)] // indices, actors, the script calls
pub mod bridge {
    use ffx_bridge::{Global, IMAGE_BASE, Mem};

    use crate::actor::bridge::ms_get_chr;
    use crate::atel::bridge::Script;

    fn byte(mem: &impl Mem, p: u32, o: u32) -> u8 {
        mem.read(p.wrapping_add(o))
    }

    fn code(mem: &impl Mem, sym: u32) -> u32 {
        sym.wrapping_sub(IMAGE_BASE).wrapping_add(mem.image_base())
    }

    /// The magic effect being read (`MsEffectRead`; its pair is
    /// `misc::LAST_MAGIC2`).
    pub const LAST_MAGIC: Global<u32> = Global::new(0x0112_c8ec);
    /// Set: no magic file is read (`MsEffectRead` reads file 0).
    pub const NO_MAGIC_FILE: Global<u8> = Global::new(0x0112_a902);
    /// Cleared when a read starts.
    pub const EFFECT_READ_STAT: Global<u32> = Global::new(0x0113_3368);
    /// The actor whose effect file is being read (`MsEffectReadComplete`).
    pub const EFFECT_READ_CHR: Global<u32> = Global::new(0x0113_337c);
    /// Set while the overlay effect stands in for a file still loading.
    pub const EFFECT_OVERLAY_BUSY: Global<u8> = Global::new(0x0112_c9d8);
    /// The overlay effect table an actor plays while its file loads
    /// (`+0xf8c`).
    pub const EFFECT_OVERLAY_DEFAULT: Global<u8> = Global::new(0x0134_0870);
    /// Set for effect 0x84 by actor model 0x108f in scene 0x1e70000.
    pub const EFFECT_SPECIAL_SCENE: Global<u32> = Global::new(0x0194_0abc);
    /// Cleared after a read starts.
    pub const EFFECT_READ_CLEAR: Global<u32> = Global::new(0x00c4_21e8);
    /// Set once effect file 0x15e was taken preloaded.
    pub const EFFECT_PRELOADED: Global<u32> = Global::new(0x012f_b8b8);

    /// The game's functions the effect read calls.
    pub const MS_BATTLE_ERROR: u32 = 0x0079_f210;
    pub const GET_CD: u32 = 0x0076_d0d0;
    pub const MAGIC_FILE_SET_MAGIC_ID: u32 = 0x009d_a7a0;
    pub const MAGIC_FILE_LOAD: u32 = 0x009d_a430;
    pub const MAGIC_FILE_IS_PRX_PRELOADED: u32 = 0x009d_ae60;
    pub const MAGIC_FILE_START: u32 = 0x009d_a7f0;
    pub const MAGIC_FILE_GET_EFFECT_OVERLAY_TABLE: u32 = 0x009d_a370;
    pub const MAGIC_FILE_CLEANUP: u32 = 0x009d_a2d0;
    pub const IS_LOAD_TEXTURE: u32 = 0x009d_a280;
    pub const GRAPHIC_IS_MAGIC_FILE_ASYNC_LOAD_AVAILABLE: u32 = 0x0064_1c60;
    pub const OP_OVL_EFFECT_INIT: u32 = 0x007f_cdd0;
    pub const CH_GET_ID: u32 = 0x0082_6b80;
    /// `MsBattleError`'s messages (`dbgPrintf`, then it hangs).
    pub const EFFECT_READ_ERROR: u32 = 0x00b5_49f8;

    /// `MsEffectRead(i, id)` (`0x00787f20`): actor `i & 0xff` starts
    /// reading magic effect `id` (`+0xdfb` its read state). Reading while
    /// an event effect plays or the actor reads already is an error
    /// (`MsBattleError`, which hangs): -1. With a stone effect playing only
    /// the state goes to 0xe. Otherwise the battle camera is reset
    /// (`MsAtelCamInit`), the CD library told 0x11 (its `+0x10`), state 1,
    /// and the effect's file (file 0 when [`NO_MAGIC_FILE`]) set and loaded
    /// (`MagicFile_SetMagicId`, `MagicFile_Load`); effect 0x84 by model
    /// 0x108f in scene 0x1e70000 marks [`EFFECT_SPECIAL_SCENE`]. A load
    /// to wait for (positive, without `AEON_PLAIN2`) has the actor play
    /// (`+0xdfd`) either the asynchronous file (state 0x10) or the overlay
    /// effect meanwhile ([`EFFECT_OVERLAY_DEFAULT`], `op_ovl_effect_init`);
    /// a preloaded effect file 0x15e completes at once
    /// ([`ms_effect_read_complete`]). Else nothing plays. 0.
    pub fn ms_effect_read<M: Mem>(mem: &mut M, s: &mut Script<'_, M>, i: u32, id: u32) -> i32 {
        let i = i & 0xff;
        let a = ms_get_chr(mem, i);
        if crate::state::bridge::EVENT_EFFECT_STAT.get(mem) != 0 || byte(mem, a, 0xdfb) != 0 {
            let (f, msg) = (code(mem, MS_BATTLE_ERROR), code(mem, EFFECT_READ_ERROR));
            (s.call)(mem, f, &[msg]);
            return -1;
        }
        LAST_MAGIC.set(mem, id);
        crate::misc::bridge::LAST_MAGIC2.set(mem, id);
        if crate::state::bridge::ms_stone_effect_check(mem) != 0 {
            mem.write(a.wrapping_add(0xdfb), 0xeu8);
            return 0;
        }
        let file = if NO_MAGIC_FILE.get(mem) == 0 { id } else { 0 };
        crate::minor::bridge::ms_atel_cam_init(mem);
        mem.write(a.wrapping_add(0xdfe), 0u8);
        mem.write(a.wrapping_add(0xe0d), 0u8);
        EFFECT_READ_STAT.set(mem, 0);
        let f = code(mem, GET_CD);
        let cd = (s.call)(mem, f, &[]);
        let f: u32 = mem.read(cd.wrapping_add(0x10));
        (s.call)(mem, f, &[0x11]);
        mem.write(a.wrapping_add(0xdfb), 1u8);
        let n = crate::exec::bridge::ms_effect_file_number(mem, file.cast_signed(), -1);
        let f = code(mem, MAGIC_FILE_SET_MAGIC_ID);
        (s.call)(mem, f, &[file]);
        let f = code(mem, MAGIC_FILE_LOAD);
        let load = (s.call)(mem, f, &[]).cast_signed();
        if file == 0x84 && crate::state::bridge::ms_get_battle_scene(mem) == 0x01e7_0000 {
            let f = code(mem, CH_GET_ID);
            let model: u32 = mem.read(a);
            if (s.call)(mem, f, &[model]) == 0x108f {
                EFFECT_SPECIAL_SCENE.set(mem, 1);
            }
        }
        if crate::action::bridge::AEON_PLAIN2.get(mem) != 0 || load <= 0 {
            mem.write(a.wrapping_add(0xdfd), 0u8);
            mem.write(a.wrapping_add(0xf8c), 0u32);
            return 0;
        }
        let async_load = code(mem, GRAPHIC_IS_MAGIC_FILE_ASYNC_LOAD_AVAILABLE);
        let available = (s.call)(mem, async_load, &[]);
        EFFECT_READ_CHR.set(mem, i);
        if available != 0 {
            mem.write(a.wrapping_add(0xdfd), 1u8);
            mem.write(a.wrapping_add(0xdfb), 0x10u8);
            mem.write(a.wrapping_add(0xf8c), 0u32);
        } else {
            EFFECT_OVERLAY_BUSY.set(mem, 1);
            mem.write(a.wrapping_add(0xdfd), 1u8);
            let table = EFFECT_OVERLAY_DEFAULT.ptr(mem).addr;
            mem.write(a.wrapping_add(0xf8c), table);
            let f = code(mem, OP_OVL_EFFECT_INIT);
            (s.call)(mem, f, &[]);
        }
        EFFECT_READ_CLEAR.set(mem, 0);
        if n != 0x15e {
            return 0;
        }
        let f = code(mem, MAGIC_FILE_IS_PRX_PRELOADED);
        if (s.call)(mem, f, &[0x15e]) == 0
            || (s.call)(mem, async_load, &[]) != 3
            || EFFECT_PRELOADED.get(mem) != 0
        {
            return 0;
        }
        let f = code(mem, IS_LOAD_TEXTURE);
        if (s.call)(mem, f, &[]) == 0 {
            return 0;
        }
        ms_effect_read_complete(mem, s);
        EFFECT_PRELOADED.set(mem, 1);
        0
    }

    /// `MsEffectReadComplete()` (`0x00788110`): the reading actor
    /// ([`EFFECT_READ_CHR`]) plays the overlay effect while the file can
    /// load asynchronously (state 1); a started file (`MagicFile_Start`)
    /// gives its overlay table (`+0xf8c`), the read ends ([`EFFECT_READ_CHR`]
    /// 0); a failed one is cleaned up (`MagicFile_Cleanup`). Returns the
    /// table or the clean-up's result.
    pub fn ms_effect_read_complete<M: Mem>(mem: &mut M, s: &mut Script<'_, M>) -> u32 {
        let a = ms_get_chr(mem, EFFECT_READ_CHR.get(mem) & 0xff);
        let f = code(mem, GRAPHIC_IS_MAGIC_FILE_ASYNC_LOAD_AVAILABLE);
        if (s.call)(mem, f, &[]) != 0 {
            EFFECT_OVERLAY_BUSY.set(mem, 1);
            mem.write(a.wrapping_add(0xdfd), 1u8);
            mem.write(a.wrapping_add(0xdfb), 1u8);
            let table = EFFECT_OVERLAY_DEFAULT.ptr(mem).addr;
            mem.write(a.wrapping_add(0xf8c), table);
            let f = code(mem, OP_OVL_EFFECT_INIT);
            (s.call)(mem, f, &[]);
        }
        let f = code(mem, MAGIC_FILE_START);
        if (s.call)(mem, f, &[]) != 0 {
            let f = code(mem, MAGIC_FILE_GET_EFFECT_OVERLAY_TABLE);
            let table = (s.call)(mem, f, &[0]);
            mem.write(a.wrapping_add(0xf8c), table);
            mem.write(a.wrapping_add(0xdfd), 0u8);
            EFFECT_OVERLAY_BUSY.set(mem, 0);
            EFFECT_READ_CHR.set(mem, 0);
            return table;
        }
        EFFECT_OVERLAY_BUSY.set(mem, 0);
        mem.write(a.wrapping_add(0xdfd), 0u8);
        mem.write(a.wrapping_add(0xf8c), 0u32);
        let f = code(mem, MAGIC_FILE_CLEANUP);
        (s.call)(mem, f, &[])
    }

    /// The command being parsed (`MsParseCommand`), twice.
    pub const PARSED_COMMAND: Global<u32> = Global::new(0x0112_c8d8);
    pub const PARSED_COMMAND2: Global<u32> = Global::new(0x0112_c8dc);
    /// The parsed command's flags bits 29 and 30 (`+0x1c`); the second
    /// lowers the music (`MsMusicVolume(0x65, 0x1e)`).
    pub const COMMAND_FLAG29: Global<u8> = Global::new(0x0112_ca05);
    pub const COMMAND_MUSIC_DOWN: Global<u8> = Global::new(0x0112_ca04);
    /// Gil thrown (command 0x302a), its achievement (0x13) at 100000.
    pub const GIL_THROWN: Global<i32> = Global::new(0x0113_0844);
    /// Counts commands with ID 0x3029 among their own (`+0x6d0`).
    pub const COMMAND_3029_COUNT: Global<u8> = Global::new(0x0112_ca15);
    /// Set by a party swap (0x3002) or an escape (0x3018).
    pub const PARTY_LEAVES: Global<u8> = Global::new(0x0112_c9f5);
    /// Cleared when an aeon is called (`MsParseCommand`), set when a
    /// command starts (`MsCommandCueProcess`).
    pub const COMMAND_STARTED: Global<u8> = Global::new(0x0112_c023);
    /// `TkGetLang` (menu) through its thunk, for `MsGetMagicEffNum`.
    pub const TK_GET_LANG: u32 = 0x0088_7c30;
    /// `achievementUnlockAchievement(n)` (main).
    pub const ACHIEVEMENT_UNLOCK: u32 = 0x0082_2380;

    /// Where a parsed effect command's tail starts (`MsParseCommand`).
    enum Tail {
        /// `+0xe06` the second effect, no MP or limit used.
        Second,
        /// `+0xe06` this, no MP or limit used.
        NoMp(u32),
        /// `+0xe06` this.
        Plain(u32),
    }

    /// `MsParseCommand(cue)` (`0x007ae370`): command cue entry `cue`'s
    /// command (`cue[2]` of its lists) taken by actor `cue[0]`. The
    /// command (`MsGetCommand`) and its flags go to the globals and the
    /// actor (`+0xf5c` its ID, `+0x6df` the turn count, `+0x451` its kind
    /// mode, the 0x3029 count, gil thrown); by its kind (`+0x15`) the
    /// actor's effect motions (`+0xe00`-`+0xe0b`: the motion IDs by
    /// monster, aeon or party member) are set and the magic effect file
    /// read ([`ms_effect_read`], the effect `MsGetMagicEffNum` chose), or
    /// the actor moves in to hit; kind 0 commands act by their ID (swap,
    /// escape, weapon change, an aeon's calling, the magus sisters'). Then
    /// the actor takes its action (`MsActionRequest`, unless the command
    /// set one up), uses its MP and limit (`MsUseChrMpLimit`), shows the
    /// command's title (`MsMenuOpenTitleWindow`), starts its motion and the
    /// cameras (`MsAtelRequestCamera` 0x3d, the magic camera). A party
    /// member (below 8) took part (`g_BtlTookPart`) unless swapped out.
    /// Returns the actor's `+0xdf2`.
    #[allow(clippy::too_many_lines, clippy::similar_names)]
    pub fn ms_parse_command<'d>(
        mem: &mut crate::target::bridge::Dm<'d>,
        host: &crate::target::bridge::Host<'_>,
        s: &mut Script<'_, crate::target::bridge::Dm<'d>>,
        cue: u32,
    ) -> i32 {
        use crate::atel::bridge as at;
        use crate::exec::bridge as ex;
        let i = u32::from(byte(mem, cue, 0));
        let sub = u32::from(byte(mem, cue, 2));
        let list = cue.wrapping_add(8).wrapping_add(sub * 0x10);
        let (rec, id) = mem.frame(4, |mem, idp| {
            let rec = crate::command::bridge::ms_get_command(mem, u32::MAX, list, idp);
            (rec, mem.read::<u32>(idp))
        });
        let a = ms_get_chr(mem, i);
        let w = |mem: &mut crate::target::bridge::Dm<'d>, o: u32, v: u16| {
            mem.write(a.wrapping_add(o), v);
        };
        PARSED_COMMAND.set(mem, id);
        PARSED_COMMAND2.set(mem, id);
        let flags: u32 = mem.read(rec.wrapping_add(0x1c));
        COMMAND_FLAG29.set(mem, u8::from(flags >> 29 & 1 != 0));
        COMMAND_MUSIC_DOWN.set(mem, u8::from(flags >> 30 & 1 != 0));
        w(mem, 0xe1a, 0xffff);
        w(mem, 0xf5c, u16::try_from(id & 0xffff).unwrap_or_default());
        mem.write(a.wrapping_add(0xe16), 1u8);
        mem.write(a.wrapping_add(0xe19), 1u8);
        let turns = ffx_kernel::bridge::ms_check_range(i32::from(byte(mem, a, 0x6df)) + 1, 1, 0xff);
        mem.write(a.wrapping_add(0x6df), turns.to_le_bytes()[0]);
        mem.write(a.wrapping_add(0x44a), 0u8);
        let h: u16 = mem.read(rec.wrapping_add(0x1a));
        mem.write(a.wrapping_add(0xe14), u8::from(h >> 14 & 1 != 0));
        let mode = ex::ms_command_kind_mode(mem, i, rec);
        mem.write(a.wrapping_add(0x451), mode.to_le_bytes()[0]);
        if PARSED_COMMAND.get(mem) == 0x302a {
            let gil = GIL_THROWN
                .get(mem)
                .wrapping_add(crate::damage::bridge::GIL_AMOUNT.get(mem));
            GIL_THROWN.set(mem, gil);
            if gil >= 100_000 {
                let f = code(mem, ACHIEVEMENT_UNLOCK);
                (s.call)(mem, f, &[0x13]);
            }
        }
        if sub == 0 {
            let v = ex::ms_check_command_3029(mem, cue.wrapping_add(8));
            mem.write(a.wrapping_add(0x6d0), v.to_le_bytes()[0]);
        }
        if byte(mem, a, 0x6d0) != 0 {
            let n = COMMAND_3029_COUNT.get(mem).wrapping_add(1);
            COMMAND_3029_COUNT.set(mem, n);
        }
        if mem.read::<u32>(a.wrapping_add(0x4ac)) & 0x10000 != 0 {
            crate::body::bridge::ms_set_chr_weight(mem, i, -1);
        }
        // the effect motions: hit, cast, and the two magic pairs
        let (mut eff1, mut eff2, e3c, e40, e28, e2c) =
            if crate::actor::bridge::ms_get_ram_chr_monster(i) != 0 {
                (0x2c, 0x2d, 0x2e, 0x2f, 0x85, 0x86)
            } else if crate::damage::bridge::ms_get_ram_chr_summon(mem, i) != 0 {
                (0x78, 0x79, 0x32, 0x33, 0x7a, 0x7b)
            } else {
                (0x1b, 0x34, 0x32, 0x33, 0x3b, 0x3c)
            };
        let weapon = crate::actor::bridge::ms_get_weapon_special(mem, i, 0);
        let targets: u32 = mem.read(a.wrapping_add(0xdc0));
        if weapon != 0 {
            mem.write(weapon.wrapping_add(0xdc0), targets);
        }
        let target = if byte(mem, a, 0xdea) == 1 {
            eff1 = 0xffff;
            eff2 = 0x89;
            0
        } else {
            targets
        };
        crate::body::bridge::MOTION_BUSY.set(mem, targets);
        if COMMAND_MUSIC_DOWN.get(mem) != 0 {
            ex::ms_music_volume(mem, &mut *s.call, 0x65, 0x1e);
        }
        crate::target::bridge::ms_set_target_dir(mem, host, i, target);
        let flags: u32 = mem.read(rec.wrapping_add(0x1c));
        if flags & 0x40_0000 != 0 {
            mem.write(a.wrapping_add(0x6ca), 1u8);
        }
        if flags & 0x400_0000 == 0 {
            mem.write(a.wrapping_add(0xe0f), 0u8);
        }
        for k in 0..4 {
            mem.write(a.wrapping_add(0xe00 + k), 0xffu8);
            w(mem, 0xe04 + 2 * k, 0xffff);
        }
        let lang = code(mem, TK_GET_LANG);
        let effect = {
            let call = &mut *s.call;
            crate::flow::bridge::ms_get_magic_eff_num(mem, i.cast_signed(), id, rec, |m| {
                call(m, lang, &[]).cast_signed()
            })
        };
        crate::minor::bridge::ms_set_save_command_use(mem, i.cast_signed(), id, 1);
        let id16 = id & 0xffff;
        if id16 == 0x3025 || id16 == 0x3026 {
            let voice = if id16 == 0x3025 { 0xe } else { 0xd };
            at::ms_set_voice_check(mem, s, i, voice, 1, 0, 0);
        }
        crate::flow::bridge::ms_limit_command_process(mem, a, cue, sub);
        let kind = byte(mem, rec, 0x15);
        let (mut act, mut title, mut mp) = (true, true, true);
        let (mut motion, mut cam, mut cam1, mut cam2) = (-1i32, false, 0xffffu32, 0xffffu32);
        let (mut hold, mut f3fa, mut took) = (1u8, 0u8, true);
        let low = |mem: &mut crate::target::bridge::Dm<'d>, o: u32, v: u32| {
            mem.write(a.wrapping_add(o), v.to_le_bytes()[0]);
        };
        let word = |mem: &mut crate::target::bridge::Dm<'d>, o: u32, v: u32| {
            mem.write(
                a.wrapping_add(o),
                u16::try_from(v & 0xffff).unwrap_or_default(),
            );
        };
        let tail = |mem: &mut crate::target::bridge::Dm<'d>,
                    s: &mut Script<'_, crate::target::bridge::Dm<'d>>,
                    t: Tail,
                    mp: &mut bool| {
            let v = match t {
                Tail::Second => {
                    *mp = false;
                    eff2
                }
                Tail::NoMp(v) => {
                    *mp = false;
                    v
                }
                Tail::Plain(v) => v,
            };
            word(mem, 0xe06, v);
            ms_effect_read(mem, s, i, effect);
        };
        let ret_motion = |mem: &mut crate::target::bridge::Dm<'d>, e00: u32, e04: u32| {
            low(mem, 0xe00, e00);
            word(mem, 0xe04, e04);
            crate::minor::bridge::ms_set_motion_return(mem, i);
        };
        let res: u8 = match kind {
            1 | 0x1e => {
                let l38 = if kind == 0x1e {
                    f3fa = 1;
                    0x50
                } else {
                    2
                };
                crate::target::bridge::ms_check_chr_near(mem, host, i, a);
                let m = if crate::actor::bridge::ms_get_ram_chr_monster(i) != 0 {
                    0x31
                } else if i < 0xf {
                    i + 0x1d
                } else {
                    i + 0x7b
                };
                cam2 = 0x1a;
                cam1 = m;
                if byte(mem, a, 0x4ac) & 0x10 != 0 {
                    low(mem, 0xe00, l38);
                    act = false;
                    title = false;
                    word(mem, 0xe04, m);
                    word(mem, 0xe08, 0x1a);
                    crate::minor::bridge::ms_set_motion_attack_wait(mem, i);
                } else {
                    motion = l38.cast_signed();
                    cam = true;
                }
                1
            }
            0x17 | 0x18 => {
                act = false;
                title = false;
                mp = false;
                hold = 0;
                crate::target::bridge::ms_check_chr_near(mem, host, i, a);
                word(mem, 0xe04, e28);
                word(mem, 0xe00, if kind == 0x17 { 0x20a } else { 0x420a });
                word(mem, 0xe06, e2c);
                ms_effect_read(mem, s, i, effect);
                3
            }
            5..=8 | 0x12..=0x15 => {
                let (e00, e04) = match kind {
                    5 => (0x13, 0x37),
                    6 => (0x14, 0x38),
                    7 => (0x15, 0x39),
                    8 => (0x16, 0x3a),
                    0x12 => (0x17, 0x7e),
                    0x13 => (0x18, 0x7f),
                    0x14 => (0x19, 0x80),
                    _ => (0x1a, 0x81),
                };
                act = false;
                title = false;
                ret_motion(mem, e00, e04);
                0
            }
            0xe | 0xf => {
                cam = true;
                motion = if kind == 0xe { 0x28 } else { 0x29 };
                cam1 = if kind == 0xe { 0x40 } else { 0x41 };
                1
            }
            _ => {
                let t = match kind {
                    9..=0xd | 0x19..=0x1b => {
                        let (k0, k1) = if kind <= 0xd {
                            (0x14, 0x19)
                        } else {
                            (0x2b, 0x2e)
                        };
                        word(mem, 0xe04, eff1);
                        low(mem, 0xe00, u32::from(kind) + k0);
                        low(mem, 0xe01, u32::from(kind) + k1);
                        Some(Tail::Second)
                    }
                    0x1c | 2 => {
                        word(mem, 0xe04, e3c);
                        word(mem, 0xe00, if kind == 2 { 0x706 } else { 0x22ff });
                        Some(Tail::Plain(e40))
                    }
                    3 | 0x10 | 0x1d | 0x16 => {
                        if kind == 0x16 {
                            mem.write(a.wrapping_add(0xdfc), 1u8);
                        }
                        let e00 = match kind {
                            3 => 0x504,
                            0x10 => 0x2b2a,
                            0x1d => 0x4d4c,
                            _ => 0x2b4b,
                        };
                        word(mem, 0xe00, e00);
                        word(mem, 0xe04, eff1);
                        Some(Tail::Second)
                    }
                    0x11 => {
                        word(mem, 0xe04, eff1);
                        Some(Tail::Second)
                    }
                    4 => {
                        word(mem, 0xe04, e28);
                        word(mem, 0xe00, 0xb0a);
                        Some(Tail::NoMp(e2c))
                    }
                    _ => None,
                };
                if let Some(t) = t {
                    act = false;
                    title = false;
                    tail(mem, s, t, &mut mp);
                    2
                } else {
                    // by the command's ID
                    let arg = |mem: &crate::target::bridge::Dm<'d>| {
                        u32::from(mem.read::<u16>(list.wrapping_add(2)))
                    };
                    match id16 {
                        0x3117 | 0x3118 | 0x30ff => {
                            let summon = if id16 == 0x30ff { 0xf } else { arg(mem) };
                            act = false;
                            mp = false;
                            cam = true;
                            COMMAND_STARTED.set(mem, 0);
                            crate::exchange::bridge::ms_exchange_summon(mem, host, s, i, summon);
                            crate::actor::bridge::ms_set_end_dir(mem, a);
                        }
                        0x3129 => {
                            ex::ms_insert_script_command(mem, 0xf, i, 0x30ea);
                        }
                        0x3002 => {
                            let other = arg(mem);
                            PARTY_LEAVES.set(mem, 1);
                            took = false;
                            cam = true;
                            cam1 = 0x36;
                            mem.write(a.wrapping_add(0xdea), 6u8);
                            crate::exchange::bridge::ms_exchange_chr(mem, host, s, i, other, 0, 0);
                            at::ms_set_voice_check(mem, s, other, 0xb, 1, 0, 0);
                            act = false;
                        }
                        0x3003 => {
                            cam = true;
                            cam1 = 0x35;
                            if crate::damage::bridge::ms_calc_escape(mem, i) != 0 {
                                ex::ms_limit_type_escape_check(mem, i, a);
                                mem.write(a.wrapping_add(0x41c), 0u8);
                                motion = 0x11;
                                let d = crate::target::bridge::ms_get_escape_dir(mem, host, i);
                                mem.write(a.wrapping_add(0x49c), d);
                                let f: u16 = mem.read(a.wrapping_add(0x606));
                                mem.write(a.wrapping_add(0x606), f & 0xfff7);
                                mem.write(a.wrapping_add(0xdea), 4u8);
                            } else {
                                motion = 0x37;
                            }
                        }
                        0x3004 | 0x3005 => {
                            word(mem, 0xe04, 0x82);
                            word(mem, 0xe00, 0x2f2e);
                            act = false;
                            title = false;
                            let v = arg(mem);
                            crate::minor::bridge::ms_weapon_read_set(
                                mem,
                                a,
                                u8::from(id16 == 0x3005),
                                v,
                            );
                        }
                        0x3018 => {
                            PARTY_LEAVES.set(mem, 1);
                            let f: u16 = mem.read(a.wrapping_add(0x606));
                            mem.write(a.wrapping_add(0x606), f & 0xfff7);
                            act = false;
                            cam = true;
                            cam1 = 0x35;
                            mem.write(a.wrapping_add(0xdea), 5u8);
                            at::ms_escape_all(mem, s);
                            at::ms_set_voice_check(mem, s, i, 0xf, 1, 0, 0);
                        }
                        0x3021 | 0x3023 | 0x3054 | 0x3022 | 0x3055 => {
                            let (e00, e04) = match id16 {
                                0x3022 => (0x3e, 0x7d),
                                0x3055 => (0x4a, 0x47),
                                _ => (0x35, 0x77),
                            };
                            act = false;
                            title = false;
                            ret_motion(mem, e00, e04);
                        }
                        0x3056 | 0x3057 => {
                            act = false;
                            cam = true;
                            let m = crate::util::bridge::ms_check_megas(i.cast_signed());
                            crate::action::bridge::ms_insert_chr_command(
                                mem,
                                m.cast_unsigned(),
                                i,
                                1,
                            );
                        }
                        _ => {}
                    }
                    0
                }
            }
        };
        mem.write(a.wrapping_add(0x446), res);
        mem.write(a.wrapping_add(0x413), hold);
        if res == 2 && (byte(mem, a, 0x15) == 0 || byte(mem, a, 0xdce) != 0) {
            word(mem, 0xe00, 0xffff);
        }
        if act {
            crate::action::bridge::ms_action_request(mem, host, s, i, 0xff, 7, 0, 2, 0);
            crate::actor::bridge::ms_pop_btl_pos(mem, a);
        }
        if mp {
            crate::flow::bridge::ms_use_chr_mp_limit(mem, i);
        }
        if title {
            ex::ms_menu_open_title_window(mem, &mut *s.call, id, 0x3c);
        }
        mem.write(a.wrapping_add(0x3fa), f3fa);
        if motion >= 0 {
            mem.write(a.wrapping_add(0x408), 0u8);
            crate::status::bridge::ms_reset_chr_pos(mem, host.trig, i, a);
            at::ms_set_motion(mem, s, i, motion, 1, 1, 1, 0, 0);
        }
        mem.write(a.wrapping_add(0x3fa), 0u8);
        at::ms_atel_request_camera(mem, s, 0, i, 0x3d, 0xffff, 0, 1, 0, 0);
        if cam {
            let cid = u32::from(mem.read::<u16>(a.wrapping_add(0xf5c)));
            at::ms_atel_request_magic_camera(mem, s, 0, i, cid, cam1, cam2, 0, 1, 0, 0);
        }
        if took && i < 8 {
            let p = crate::flow::bridge::TOOK_PART.ptr(mem).addr;
            mem.write(p + i, 1u8);
        }
        i32::from(mem.read::<i8>(a.wrapping_add(0xdf2)))
    }
}
