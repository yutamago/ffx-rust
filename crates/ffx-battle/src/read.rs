//! The battle's reads (`docs/rewrite-plan.md`, M2): leaves of
//! `MsBtlReadManage` (`0x00783020`), which loads a battle's scene and
//! actors across frames, and places and shows them when they are in.
//!
//! The CD object, the heap (`Sg_MallocA`), chara's functions with
//! callbacks, the map's and graphics' loaders, sound and script, the
//! battle functions not ported yet (`MsSetRamChrData`,
//! `MsBtlBridgeEvent2Btl`, the monster loaders) and, from the state
//! machine, `MsBtlReadSetScene` and `MsBtlReadMonster` (its shim makes the
//! request block) go to the game's code through `call`.

/// The game's memory, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::{IMAGE_BASE, Mem};

    use ffx_bridge::Global;

    use crate::actor::bridge::ms_get_chr;
    use crate::atel::bridge::Script;
    use crate::exec::bridge::Call;
    use crate::state::bridge::STATUS;
    use crate::target::bridge::{Dm, Host, Regs};

    fn code(mem: &impl Mem, sym: u32) -> u32 {
        sym.wrapping_sub(IMAGE_BASE).wrapping_add(mem.image_base())
    }

    fn game<M: Mem>(mem: &mut M, call: Call<'_, M>, sym: u32, args: &[u32]) -> u32 {
        let f = code(mem, sym);
        call(mem, f, args)
    }

    /// `Sg_MallocA(n, align, pool, name, flags)` (main) and the scene
    /// buffer's name.
    pub const SG_MALLOC_A: u32 = 0x0083_f0a0;
    pub const SCENE_NAME: u32 = 0x00b5_4940;
    /// The scene read's completion callback (`0x00783dd0`, the CD object
    /// calls it).
    pub const SCENE_READ_DONE: u32 = 0x0078_3dd0;
    /// `Ch_SetHideEvent(chr, flags)` and `Ch_SetPos3(chr, x, y, z)` (chara).
    pub const CH_SET_HIDE_EVENT: u32 = 0x0082_b2c0;
    pub const CH_SET_POS3: u32 = 0x0082_b480;

    /// `MsBtlReadScene(file)` (`0x00783d80`): `file` of directory 0x29 (CD
    /// methods `+0x10`, `+0x14`) into a new block (`Sg_MallocA(n, 0x10, 5,
    /// name, -1)`, `g_BtlRead3`), read asynchronously (method `+0`, with
    /// [`SCENE_READ_DONE`] and 0). The read's result.
    pub fn ms_btl_read_scene<M: Mem>(mem: &mut M, call: Call<'_, M>, file: u32) -> u32 {
        let cd = |mem: &mut M, call: Call<'_, M>, slot: u32, args: &[u32]| {
            let obj = game(mem, &mut *call, crate::getmem::bridge::GET_CD, &[]);
            let f: u32 = mem.read(obj.wrapping_add(slot));
            call(mem, f, args)
        };
        cd(mem, &mut *call, 0x10, &[0x29]);
        let n = cd(mem, &mut *call, 0x14, &[file]);
        let name = code(mem, SCENE_NAME);
        let p = game(mem, &mut *call, SG_MALLOC_A, &[n, 0x10, 5, name, u32::MAX]);
        crate::misc::bridge::BTL_READ3.set(mem, p);
        let done = code(mem, SCENE_READ_DONE);
        cd(mem, call, 0, &[file, p, done, 0])
    }

    /// The motion `MsGetRamChrCTBbasic` waits on.
    pub const CTB_WAIT_MOTION: u8 = 0xd0;

    /// `MsGetRamChrCTBbasic()` (`0x00784220`): the first of the 0x1f actors
    /// still loading (`+0x433`) goes to `g_BtlEffectWait` with motion 0xd0:
    /// -1; 0 when none is.
    pub fn ms_get_ram_chr_ctb_basic(mem: &mut impl Mem) -> i32 {
        for i in 0..0x1f {
            let a = ms_get_chr(mem, i);
            if mem.read::<u8>(a.wrapping_add(0x433)) != 0 {
                #[allow(clippy::cast_possible_truncation)] // below 0x1f
                crate::flow::bridge::EFFECT_WAIT.set(mem, [i as u8, CTB_WAIT_MOTION]);
                return -1;
            }
        }
        0
    }

    /// `MsPlaceMonsters_00784420()`: each monster (actors 0x14-0x1b) with
    /// a model is shown to the event (`Ch_SetHideEvent(chr, 0)`), placed at
    /// its battle position (`MsGetChrBtlPos`, `Ch_SetPos3`) and, unless
    /// `+0x407`, made visible (`MsSetChrVisibleAppear(i, 0)`). The count.
    pub fn ms_place_monsters<'d>(mem: &mut Dm<'d>, host: &Host<'_>, call: Call<'_, Dm<'d>>) -> u32 {
        use ffx_bridge::x87::{load, store};
        mem.locals(0x10, |mem, v| {
            let mut n = 0;
            for i in 0x14..0x1c {
                let a = ms_get_chr(mem, i);
                let chr: u32 = mem.read(a);
                if chr == 0 {
                    continue;
                }
                n += 1;
                crate::pos::bridge::ms_get_chr_btl_pos(mem, host.trig, i, v);
                game(mem, &mut *call, CH_SET_HIDE_EVENT, &[chr, 0]);
                let p: [u32; 3] = mem.read(v);
                let p = p.map(|w| store(load(w)));
                game(mem, &mut *call, CH_SET_POS3, &[chr, p[0], p[1], p[2]]);
                if mem.read::<u8>(a.wrapping_add(0x407)) == 0 {
                    crate::visible::bridge::ms_set_chr_visible_appear(mem, &mut *call, i, 0);
                }
            }
            n
        })
    }

    /// `MsCalcChrDirAll()` (`0x00793cb0`): each of the 0x1f actors faces
    /// its direction ([`crate::exchange::bridge::ms_calc_chr_dir`]): its
    /// directions `+0x514`, `+0x518` and `+0x49c`. With the caller's
    /// registers (`regs`) the direction's own caller words are this
    /// function's (`i`, the actor and the caller's `ebx`).
    pub fn ms_calc_chr_dir_all(mem: &mut Dm<'_>, host: &Host<'_>, regs: Option<Regs>) {
        for i in 0..0x1f {
            let a = ms_get_chr(mem, i);
            let inner = regs.map(|r| Regs {
                edi: i,
                esi: a,
                ebx: r.ebx,
            });
            let d = crate::exchange::bridge::ms_calc_chr_dir(mem, host, i, inner);
            for o in [0x514, 0x518, 0x49c] {
                mem.write(a.wrapping_add(o), d);
            }
        }
    }

    /// `MsSetChrVisibleEvent(i, flags)` (`0x007967f0`): actor `i`'s model
    /// (1 when it has one) and its two weapons' models get the event hide
    /// flags (`Ch_SetHideEvent`).
    pub fn ms_set_chr_visible_event<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        i: u32,
        flags: u32,
    ) -> u32 {
        let i = i & 0xff;
        let chr: u32 = mem.read(ms_get_chr(mem, i));
        let mut r = 0;
        if chr != 0 {
            game(mem, &mut *call, CH_SET_HIDE_EVENT, &[chr, flags]);
            r = 1;
        }
        for k in 0..2 {
            let w = crate::actor::bridge::ms_get_weapon(mem, i, k);
            if w == 0 {
                continue;
            }
            let m: u32 = mem.read(w);
            if m != 0 {
                game(mem, &mut *call, CH_SET_HIDE_EVENT, &[m, flags]);
            }
        }
        r
    }

    /// The formation file's section 1 (`u32`, `MsBtlReadSetScene`): its
    /// first byte goes to the second script record's `+0`.
    pub const SECTION1: Global<u32> = Global::new(0x0112_a9b4);
    /// The battle file's text table (`u32`, 0 without one; `+4i` holds a
    /// `u16` offset from the table, `MsGetMenuChrInfo`), from
    /// [`FILE_TEXT_DATA`].
    pub const FILE_TEXT: Global<u32> = Global::new(0x0112_a9b8);
    /// The battle file's loaded text and font (`u32` each, 0 when absent;
    /// only `MsBtlReadSetScene` reads them) and whether the file is not
    /// [`SYSTEM_FILE`] (`u32` 1/0).
    pub const FILE_TEXT_DATA: Global<u32> = Global::new(0x0231_12c4);
    pub const FILE_FONT_DATA: Global<u32> = Global::new(0x0231_12c0);
    pub const NOT_SYSTEM_FILE: Global<u32> = Global::new(0x0231_12bc);
    /// `"system_01"` (`.rdata`), the battle file without its own text.
    pub const SYSTEM_FILE: u32 = 0x00b5_48f0;
    /// `TOFTCTransfer(font)` (system), `MsSetRamChrData()`,
    /// `MsBtlBridgeEvent2Btl()` (battle, not ported),
    /// `AtelEventRemoveForBattle(map)`, `TkGetNowEventJumpMapNo()` (script),
    /// `Sg_SetPlayer(n)`, `Sg_IdSetEffectTableIndex(i)` (main).
    pub const TO_FTC_TRANSFER: u32 = 0x008a_c130;
    pub const MS_SET_RAM_CHR_DATA: u32 = 0x0079_c110;
    pub const MS_BTL_BRIDGE_EVENT2BTL: u32 = 0x0078_24e0;
    pub const ATEL_EVENT_REMOVE_FOR_BATTLE: u32 = 0x0086_5dc0;
    pub const TK_GET_NOW_EVENT_JUMP_MAP_NO: u32 = 0x0086_c1f0;

    /// The C string at `at` (at most 0x40 bytes).
    fn c_string(mem: &impl Mem, at: u32) -> Vec<u8> {
        (0..0x40)
            .map(|i| mem.read::<u8>(at.wrapping_add(i)))
            .take_while(|&c| c != 0)
            .collect()
    }

    /// `MsBtlReadSetScene()` (`0x00783e20`), the battle file's sections
    /// once it is read (`docs/formats/battle.md`, "Formation files"): the
    /// script, section 1 (its first byte into the second script record),
    /// the formation ([`VOICE_EVENT`](crate::state::bridge::VOICE_EVENT);
    /// byte 3 the underwater flag) and the positions; the file's text table
    /// and font (`TOFTCTransfer`) when it brought them, and whether it is
    /// not `system_01`. With `MsBattleExe`'s mode set the battle status
    /// becomes 0 and that is all (0). Else the battle's actions, its
    /// monsters (`MsSetRamChrData`), positions and the event's actors
    /// (`MsBtlBridgeEvent2Btl`) are set up, the event map's script actors
    /// removed, player 0 and effect table 1 selected, and the status is 10
    /// when the event jumps to the battle's map (`g_BtlBattleMusic`, signed)
    /// or 8; it is returned. `MsGetSaveConfigEnglish` and `FFXIsDebug`,
    /// which the original calls with mode 0 and ignores, are not called
    /// (the first is `xor eax, eax`, the second reads a flag).
    pub fn ms_btl_read_set_scene<M: Mem>(mem: &mut M, call: Call<'_, M>) -> u32 {
        use crate::exe::bridge::{BATTLE_MUSIC, FILE_BUFFER, SG_SET_PLAYER};
        use crate::state::bridge::{UNDERWATER, VOICE_EVENT};
        let file = FILE_BUFFER.get(mem);
        let section =
            |mem: &M, k: u32| mem.read::<u32>(file.wrapping_add(4 * k)).wrapping_add(file);
        let (script, first, formation, positions) = (
            section(mem, 1),
            section(mem, 2),
            section(mem, 3),
            section(mem, 4),
        );
        SCENE_SCRIPT.set(mem, script);
        SECTION1.set(mem, first);
        VOICE_EVENT.set(mem, formation);
        crate::pos::bridge::POS_FILE.set(mem, positions);
        FILE_TEXT.set(mem, 0);
        crate::misc::bridge::FONT.set(mem, 0);
        let mode = EXE_MODE.get(mem);
        let text = FILE_TEXT_DATA.get(mem);
        if text != 0 {
            FILE_TEXT.set(mem, text);
            let font = FILE_FONT_DATA.get(mem);
            if font != 0 {
                crate::misc::bridge::FONT.set(mem, font);
                game(mem, &mut *call, TO_FTC_TRANSFER, &[font]);
            }
        }
        let name = c_string(mem, code(mem, FILE_NAME));
        let system = c_string(mem, code(mem, SYSTEM_FILE));
        NOT_SYSTEM_FILE.set(mem, u32::from(name != system));
        let record1 = crate::getmem::bridge::SCRIPT_RECORD1.ptr(mem).addr;
        let b: u8 = mem.read(first);
        mem.write(record1, b);
        let underwater: i8 = mem.read(formation.wrapping_add(3));
        UNDERWATER.set(mem, underwater);
        if mode != 0 {
            STATUS.set(mem, 0);
            return 0;
        }
        crate::misc::bridge::ms_action_init(mem);
        game(mem, &mut *call, MS_SET_RAM_CHR_DATA, &[]);
        crate::minor::bridge::ms_push_btl_pos_all(mem);
        game(mem, &mut *call, MS_BTL_BRIDGE_EVENT2BTL, &[]);
        let map = i32::from(BATTLE_MUSIC.get(mem).cast_signed()).cast_unsigned();
        game(mem, &mut *call, ATEL_EVENT_REMOVE_FOR_BATTLE, &[map]);
        game(mem, &mut *call, SG_SET_PLAYER, &[0]);
        game(
            mem,
            &mut *call,
            crate::free::bridge::SG_ID_SET_EFFECT_TABLE_INDEX,
            &[1],
        );
        let jump = game(mem, call, TK_GET_NOW_EVENT_JUMP_MAP_NO, &[]);
        let map = i32::from(BATTLE_MUSIC.get(mem).cast_signed()).cast_unsigned();
        let status: i8 = if jump == map { 10 } else { 8 };
        STATUS.set(mem, status);
        status.cast_unsigned().into()
    }

    /// The debug menu's read wait (`u8`, on: [`READ_WAIT`] frames pass
    /// before each step) and the frames left (`MsBattleExe` clears it;
    /// 0x3c after each camera request).
    pub const DEBUG_READ_WAIT: Global<u8> = Global::new(0x0112_a918);
    pub const READ_WAIT: Global<u8> = Global::new(0x0112_a919);
    /// Frames the read holds before the battle starts (`i32`, counted down
    /// each call; status 0x16 waits for 0).
    pub const READ_HOLD: Global<i32> = Global::new(0x0112_ca64);
    /// `MsBattleExe`'s mode (`i8`) and the battle file's number.
    pub const EXE_MODE: Global<i8> = Global::new(0x0112_c9d9);
    pub const FILE_NUMBER: Global<u32> = Global::new(0x0112_a8ec);
    /// The battle file's name (the CD object's `+0x58`, copied).
    pub const FILE_NAME: u32 = 0x0231_12e0;
    /// 0 for battle files 0x102-0x104, else 1 (later 8 or 9).
    pub const FILE_KIND: Global<u32> = Global::new(0x0112_ca78);
    /// The map's read was started (`MsBattleExe` clears it): the monsters
    /// are placed when the actors come in.
    pub const MAP_STARTED: Global<u8> = Global::new(0x0112_c9da);
    /// Who strikes first (`MsGetFirstAttack`: 1 the party, 2 the
    /// monsters) and a script's choice (`i8`, -1 none).
    pub const FIRST_ATTACK: Global<i8> = Global::new(0x0112_c9dc);
    pub const FIRST_ATTACK_SET: Global<i8> = Global::new(0x0112_a923);
    /// The two scripts registered in manager 0 (the second is the
    /// formation file's section 0, `MsBtlReadSetScene`).
    pub const FIRST_SCRIPT: Global<u32> = Global::new(0x0112_a98c);
    pub const SCENE_SCRIPT: Global<u32> = Global::new(0x0112_a9ac);
    /// The start's readiness flag (`u8`, `0x00781650` reads it) and a
    /// field mode flag (`u32`, 1 while `0x00c416c0` is 0xb and
    /// `0x0112a058` is 1).
    pub const START_READY: Global<u8> = Global::new(0x0112_c9cc);
    pub const FIELD_MODE_B: Global<u32> = Global::new(0x0112_a060);
    /// `AsyncLoadNewCharacter`'s count (`i32`, it counts up while not
    /// negative).
    pub const NEW_CHARACTER_LOADS: Global<i32> = Global::new(0x00c4_21dc);
    /// Three counters handed out to monsters' `+0x5c3`.
    pub const MONSTER_SLOTS: Global<[u32; 3]> = Global::new(0x0112_ca84);
    /// The special texture's path buffer
    /// (`graphicMenuLoadSpecialTexture`).
    pub const SPECIAL_TEXTURE: u32 = 0x025d_06a0;
    /// The scene id that gets the longer ambush message.
    pub const LONG_AMBUSH_SCENE: u32 = 0x01e6_0046;

    /// `AtelIsCallbackQue()` (script), `GetCD()`, `graphicRegistAsyncTask`,
    /// `TOInitBattleFontPath` (system), `TkGetLang_thunk`,
    /// `graphicMenuLoadSpecialTexture`, `FlushCache`,
    /// `graphicSetIsBattleMap`, `yiBattleMapReadNB`, `yiFieldMapReadNB`,
    /// `yiGetRenderStatus`, `loadPhyreFieldMap` (map), the sound's
    /// `SqsSetReverbPlaySwitch` thunk, `Ch_CheckNbBattleChr` (chara), and the
    /// battle readers `MsBtlReadSetScene`, `MsBtlReadMonster`.
    pub const ATEL_IS_CALLBACK_QUE: u32 = 0x0087_5180;
    pub const GRAPHIC_LEVEL_GET_NAME_HASH: u32 = 0x0064_1da0;
    pub const GRAPHIC_REGIST_ASYNC_TASK: u32 = 0x0064_2950;
    pub const TO_INIT_BATTLE_FONT_PATH: u32 = 0x008a_d780;
    pub const TK_GET_LANG: u32 = 0x0088_7c30;
    pub const GRAPHIC_MENU_LOAD_SPECIAL_TEXTURE: u32 = 0x0064_2300;
    pub const FLUSH_CACHE: u32 = 0x0062_f4c0;
    pub const GRAPHIC_SET_IS_BATTLE_MAP: u32 = 0x0064_3490;
    pub const YI_BATTLE_MAP_READ_NB: u32 = 0x0090_83a0;
    pub const YI_FIELD_MAP_READ_NB: u32 = 0x0090_8600;
    pub const YI_GET_RENDER_STATUS: u32 = 0x0090_8f10;
    pub const LOAD_PHYRE_FIELD_MAP: u32 = 0x0090_8000;
    pub const SQS_SET_REVERB_PLAY_SWITCH: u32 = 0x0088_7170;
    pub const CH_CHECK_NB_BATTLE_CHR: u32 = 0x0083_37f0;
    pub const MS_BTL_READ_SET_SCENE: u32 = 0x0078_3e20;
    pub const MS_BTL_READ_MONSTER: u32 = 0x0078_3680;
    /// The async task `graphicRegistAsyncTask` gets and the battle file's
    /// read completion (`MsBtlReadManage` hands both to the game).
    pub const READ_ASYNC_TASK: u32 = 0x0078_4500;
    pub const FILE_READ_DONE: u32 = 0x0078_2fe0;

    /// The battle's scenario (`getScenerioFlag`) and level hash where the
    /// read takes the first visit's path (no start message, the cue
    /// waits on the character loads).
    const FIRST_SCENARIO: u16 = 0x9ce;
    const FIRST_LEVEL: u32 = 0x223f;

    fn camera<'d>(mem: &mut Dm<'d>, s: &mut Script<'_, Dm<'d>>, a: u32) {
        crate::atel::bridge::ms_atel_request_camera(mem, s, 0, 0xff, a, 0xffff, 0, 1, 0, 0);
        READ_WAIT.set(mem, 0x3c);
    }

    /// The character loads are not done: cues still read and loads
    /// counting.
    fn loading(mem: &Dm<'_>) -> bool {
        crate::state::bridge::ms_btl_read_cue_level(mem) != 0 && NEW_CHARACTER_LOADS.get(mem) > 0
    }

    /// `MsBtlReadManage()` (`0x00783020`): one step of the battle's read,
    /// by `g_BtlStatus`, then the read cue (`MsBtlReadCueManage`). With the
    /// debug wait on, a step waits [`READ_WAIT`] frames first (and nothing
    /// else runs). The steps (each falls into the next unless it waits):
    ///
    /// - 4: waits for the script callbacks (`AtelIsCallbackQue`).
    /// - 5 (to 6): the file's index (`MsBattleExeSetIndex`), name and an
    ///   asynchronous read into the buffer (CD methods `+0x58`, `+0`), its
    ///   font path, the special texture (unless a language is set).
    /// - 7: the scene's sections (`MsBtlReadSetScene`; 0xa skips the map).
    /// - 8 (to 9): the map's read, a battle map (`g_BtlBattleMusic` bit
    ///   0x400) or a field map, number its low ten bits.
    /// - 9 (to 0x22): waits for the renderer (bits 0x20, 0x40), loads it.
    /// - 0x22 (to 0xa): waits for the map (`g_BtlMapRead`).
    /// - 0xa (to 0xb): directions, first strike, the two scripts, the
    ///   battle count, camera 0x18, the party shown (or motion 0x3f) and the
    ///   monsters placed.
    /// - 0xb (to 0xc): each party member's model and weapons queued.
    /// - 0xc: waits for them, camera 0x75, the monsters' data
    ///   (`MsBtlReadMonster`).
    /// - 0xe (to 0xf): each monster's model and weapon queued.
    /// - 0xf: waits for them, camera 0x76; 0x10 (to 0x11): the scene's
    ///   footsteps (`MsBtlReadScene`).
    /// - 0x13 (to 0x14): the party's second models queued; 0x14 waits.
    /// - 0x15: the first strike's message; 0x16 (to 1): waits for the
    ///   message window, the sound transfers, the actors' loads and the
    ///   hold, then the battle starts (system check 5, camera 0x7c).
    ///
    /// The others (6, 0xd, 0x11, 0x12, 0x17-0x21, and out of range) only run
    /// the cue.
    #[allow(clippy::too_many_lines)]
    pub fn ms_btl_read_manage<'d>(mem: &mut Dm<'d>, s: &mut Script<'_, Dm<'d>>, host: &Host<'_>) {
        use crate::exchange::bridge::{
            ms_btl_read_cue_manage, ms_btl_read_cue_regist, ms_btl_read_cue_regist_weapon,
        };
        /// Where a step goes on (the original's fall-throughs and jumps).
        #[derive(Clone, Copy, PartialEq, Eq)]
        enum Step {
            File,
            Map,
            Render,
            MapWait,
            Party,
            PartyModels,
            PartyWait,
            MonsterModels,
            MonsterWait,
            Footsteps,
            SecondModels,
            SecondWait,
            Message,
            Start,
            Done,
        }
        if DEBUG_READ_WAIT.get(mem) != 0 && READ_WAIT.get(mem) != 0 {
            READ_WAIT.set(mem, READ_WAIT.get(mem) - 1);
            return;
        }
        if READ_HOLD.get(mem) > 0 {
            READ_HOLD.set(mem, READ_HOLD.get(mem) - 1);
        }
        let first = crate::save::bridge::get_scenerio_flag(mem) == FIRST_SCENARIO
            && game(mem, &mut *s.call, GRAPHIC_LEVEL_GET_NAME_HASH, &[]) == FIRST_LEVEL;
        let mut step = match STATUS.get(mem) {
            4 => {
                if game(mem, &mut *s.call, ATEL_IS_CALLBACK_QUE, &[]) == 0 {
                    Step::File
                } else {
                    Step::Done
                }
            }
            5 => Step::File,
            7 => {
                game(mem, &mut *s.call, FLUSH_CACHE, &[0]);
                if game(mem, &mut *s.call, MS_BTL_READ_SET_SCENE, &[]) == 0xa {
                    Step::Party
                } else {
                    Step::Map
                }
            }
            8 => Step::Map,
            9 => Step::Render,
            0x22 => Step::MapWait,
            0xa => Step::Party,
            0xb => Step::PartyModels,
            0xc => Step::PartyWait,
            0xe => Step::MonsterModels,
            0xf => Step::MonsterWait,
            0x10 => Step::Footsteps,
            0x13 => Step::SecondModels,
            0x14 => Step::SecondWait,
            0x15 => Step::Message,
            0x16 => Step::Start,
            _ => Step::Done,
        };
        while step != Step::Done {
            step = match step {
                Step::File => {
                    read_file(mem, &mut *s.call);
                    Step::Done
                }
                Step::Map => {
                    crate::misc::bridge::MAP_READ.set(mem, 1);
                    game(mem, &mut *s.call, GRAPHIC_SET_IS_BATTLE_MAP, &[1]);
                    let map = crate::exe::bridge::BATTLE_MUSIC.get(mem);
                    STATUS.set(mem, 9);
                    MAP_STARTED.set(mem, 1);
                    let read = if map & 0x400 != 0 {
                        YI_BATTLE_MAP_READ_NB
                    } else {
                        YI_FIELD_MAP_READ_NB
                    };
                    game(mem, &mut *s.call, read, &[u32::from(map & 0x3ff), 0]);
                    Step::Done
                }
                Step::Render => {
                    let busy = |mem: &mut Dm<'d>, s: &mut Script<'_, Dm<'d>>, bit: u32| {
                        game(mem, &mut *s.call, YI_GET_RENDER_STATUS, &[]) & bit != 0
                    };
                    if busy(mem, s, 0x20) || busy(mem, s, 0x40) {
                        Step::Done
                    } else {
                        game(mem, &mut *s.call, LOAD_PHYRE_FIELD_MAP, &[]);
                        STATUS.set(mem, 0x22);
                        Step::MapWait
                    }
                }
                Step::MapWait => {
                    if crate::misc::bridge::MAP_READ.get(mem) != 0 {
                        Step::Done
                    } else {
                        STATUS.set(mem, 0xa);
                        Step::Party
                    }
                }
                Step::Party => {
                    party(mem, s, host);
                    STATUS.set(mem, 0xb);
                    Step::PartyModels
                }
                Step::PartyModels => {
                    NEW_CHARACTER_LOADS.set(mem, 0);
                    let counts = crate::flow::bridge::COUNTS_BATTLE.ptr(mem).addr;
                    for i in 0..0x12 {
                        let a = ms_get_chr(mem, i);
                        let c = u32::from(mem.read::<u8>(a.wrapping_add(0xc)));
                        if mem.read::<u8>(a.wrapping_add(0xdc8)) != 0 {
                            mem.write(counts.wrapping_add(c), 1u8);
                            let model: u32 = mem.read(a.wrapping_add(4));
                            ms_btl_read_cue_regist(mem, s, 0, c, c, model, 1);
                        }
                    }
                    for i in 0..0x12 {
                        let a = ms_get_chr(mem, i);
                        if mem.read::<u8>(a.wrapping_add(0xdc8)) != 0 {
                            let c = u32::from(mem.read::<u8>(a.wrapping_add(0xc)));
                            ms_btl_read_cue_regist_weapon(mem, s, c, 3);
                        }
                    }
                    STATUS.set(mem, 0xc);
                    Step::PartyWait
                }
                Step::PartyWait => {
                    if loading(mem) {
                        Step::Done
                    } else {
                        if first {
                            START_READY.set(mem, 0);
                        } else if FIELD_MODE_B.get(mem) == 0 {
                            START_READY.set(mem, 1);
                        }
                        NEW_CHARACTER_LOADS.set(mem, -1);
                        camera(mem, s, 0x75);
                        game(mem, &mut *s.call, MS_BTL_READ_MONSTER, &[]);
                        Step::Done
                    }
                }
                Step::MonsterModels => {
                    NEW_CHARACTER_LOADS.set(mem, 0);
                    MONSTER_SLOTS.set(mem, [0; 3]);
                    for i in 0x14..0x1c {
                        let a = ms_get_chr(mem, i);
                        let id: u16 = mem.read(a.wrapping_add(0xe));
                        if id != 0xffff && mem.read::<u8>(a.wrapping_add(0x24)) != 0 {
                            let weapon: u32 = mem.read(a.wrapping_add(8));
                            let model: u32 = mem.read(a.wrapping_add(4));
                            ms_btl_read_cue_regist(mem, s, 0, i, i, model, 0);
                            ms_btl_read_cue_regist(mem, s, 2, i, i, weapon, 3);
                        }
                    }
                    STATUS.set(mem, 0xf);
                    Step::Done
                }
                Step::MonsterWait => {
                    if loading(mem) {
                        Step::Done
                    } else {
                        NEW_CHARACTER_LOADS.set(mem, -1);
                        camera(mem, s, 0x76);
                        Step::Footsteps
                    }
                }
                Step::Footsteps => {
                    STATUS.set(mem, 0x11);
                    let formation = crate::state::bridge::VOICE_EVENT.get(mem);
                    let n = i32::from(mem.read::<i8>(formation.wrapping_add(2)));
                    ms_btl_read_scene(mem, &mut *s.call, n.cast_unsigned());
                    Step::Done
                }
                Step::SecondModels => {
                    if first {
                        NEW_CHARACTER_LOADS.set(mem, 0);
                    }
                    for i in 0..0x12 {
                        let a = ms_get_chr(mem, i);
                        let c = u32::from(mem.read::<u8>(a.wrapping_add(0xc)));
                        if mem.read::<u8>(a.wrapping_add(0xdc8)) != 0 {
                            let model: u32 = mem.read(a.wrapping_add(8));
                            ms_btl_read_cue_regist(mem, s, 2, c, c, model, 2);
                        }
                    }
                    STATUS.set(mem, 0x14);
                    Step::SecondWait
                }
                Step::SecondWait => {
                    let reading = crate::state::bridge::ms_btl_read_cue_level(mem) != 0;
                    if first {
                        if reading && NEW_CHARACTER_LOADS.get(mem) > 0 {
                            Step::Done
                        } else {
                            NEW_CHARACTER_LOADS.set(mem, -1);
                            START_READY.set(mem, 1);
                            STATUS.set(mem, 0x15);
                            Step::Message
                        }
                    } else if reading {
                        Step::Done
                    } else {
                        if FIELD_MODE_B.get(mem) == 1 {
                            START_READY.set(mem, 1);
                        }
                        STATUS.set(mem, 0x15);
                        Step::Message
                    }
                }
                Step::Message => {
                    first_strike_message(mem, &mut *s.call);
                    Step::Start
                }
                Step::Start => {
                    STATUS.set(mem, 0x16);
                    if crate::battle_end::bridge::MESSAGE_WINDOW.get(mem) == 0
                        && crate::state::bridge::ms_sep_trans_check(mem) == 0
                        && ms_get_ram_chr_ctb_basic(mem) == 0
                        && READ_HOLD.get(mem) == 0
                    {
                        game(mem, &mut *s.call, CH_CHECK_NB_BATTLE_CHR, &[]);
                        STATUS.set(mem, 1);
                        crate::process::bridge::LIMIT_TYPE_CUE.set(mem, 1);
                        crate::battle_end::bridge::ms_atel_request_system_check_exe(
                            mem, s, 5, 1, 0, 0,
                        );
                        crate::atel::bridge::ms_atel_request_camera(
                            mem, s, 0, 0xff, 0x7c, 0xffff, 0, 1, 0, 0,
                        );
                    }
                    Step::Done
                }
                Step::Done => Step::Done,
            };
        }
        ms_btl_read_cue_manage(mem, s);
    }

    /// Step 5 of [`ms_btl_read_manage`]: status 6, the file's index, its
    /// name copied ([`FILE_NAME`]), the async task, the file's kind, the
    /// read into the file buffer, the battle font's path and, unless a
    /// language is set (`TkGetLang`), the special texture.
    fn read_file<M: Mem>(mem: &mut M, call: Call<'_, M>) {
        let cd = |mem: &mut M, call: Call<'_, M>, slot: u32, args: &[u32]| {
            let obj = game(mem, &mut *call, crate::getmem::bridge::GET_CD, &[]);
            let f: u32 = mem.read(obj.wrapping_add(slot));
            call(mem, f, args)
        };
        let buffer = crate::exe::bridge::FILE_BUFFER.get(mem);
        STATUS.set(mem, 6);
        let file = FILE_NUMBER.get(mem);
        let mode = i32::from(EXE_MODE.get(mem)).cast_unsigned();
        crate::getmem::bridge::ms_battle_exe_set_index(mem, &mut *call, mode, file);
        let name = cd(mem, &mut *call, 0x58, &[file]);
        let to = code(mem, FILE_NAME);
        for k in 0.. {
            let c: u8 = mem.read(name.wrapping_add(k));
            mem.write(to.wrapping_add(k), c);
            if c == 0 {
                break;
            }
        }
        let task = code(mem, READ_ASYNC_TASK);
        game(mem, &mut *call, GRAPHIC_REGIST_ASYNC_TASK, &[task, 0]);
        FILE_KIND.set(mem, u32::from(file.wrapping_sub(0x102) > 2));
        let done = code(mem, FILE_READ_DONE);
        cd(mem, &mut *call, 0, &[file, buffer, done, 0]);
        let name = cd(mem, &mut *call, 0x58, &[file]);
        game(mem, &mut *call, TO_INIT_BATTLE_FONT_PATH, &[name]);
        if game(mem, &mut *call, TK_GET_LANG, &[]) == 0 {
            let path = code(mem, SPECIAL_TEXTURE);
            game(mem, call, GRAPHIC_MENU_LOAD_SPECIAL_TEXTURE, &[8, path, 0]);
        }
    }

    /// Step 0xa of [`ms_btl_read_manage`]: the actors face their
    /// directions, the first strike (`MsGetFirstAttack(0x20)` or a
    /// script's), the scene's two scripts in manager 0 and set up, the
    /// reverb (formation byte 1), one more battle (below 0x0fffffff, unless
    /// the steal rule is 2), camera 0x18, each party slot shown to the event
    /// and made visible (none in the battle: motion 0x3f), the monsters
    /// placed once the map's read started.
    fn party<'d>(mem: &mut Dm<'d>, s: &mut Script<'_, Dm<'d>>, host: &Host<'_>) {
        ms_calc_chr_dir_all(mem, host, None);
        let who = crate::flow::bridge::ms_get_first_attack(mem, 0x20);
        FIRST_ATTACK.set(mem, i8::from_le_bytes([who.to_le_bytes()[0]]));
        for script in [FIRST_SCRIPT.get(mem), SCENE_SCRIPT.get(mem)] {
            crate::minor::bridge::ms_atel_regist(mem, 0, script);
        }
        crate::effect::bridge::ms_atel_set_up(mem, &mut *s.call, 0);
        let set = FIRST_ATTACK_SET.get(mem);
        if set != -1 {
            FIRST_ATTACK.set(mem, set);
        }
        let formation = crate::state::bridge::VOICE_EVENT.get(mem);
        let reverb = u32::from(mem.read::<u8>(formation.wrapping_add(1)) == 1);
        game(mem, &mut *s.call, SQS_SET_REVERB_PLAY_SWITCH, &[reverb]);
        let count = crate::save::bridge::BATTLE_COUNT;
        let n = count.get(mem);
        if n < 0x0fff_ffff && crate::save::bridge::STEAL_RULE.get(mem) != 2 {
            count.set(mem, n + 1);
        }
        camera(mem, s, 0x18);
        for i in 0..0x12 {
            let a = ms_get_chr(mem, i);
            ms_set_chr_visible_event(mem, &mut *s.call, i, 0);
            if mem.read::<u8>(a.wrapping_add(0xdc8)) == 0 {
                crate::visible::bridge::ms_set_chr_visible_appear(mem, &mut *s.call, i, 0);
            } else {
                crate::atel::bridge::ms_set_motion(mem, s, i, 0x3f, 0, 0, 1, 0, 0);
            }
        }
        if MAP_STARTED.get(mem) != 0 {
            ms_place_monsters(mem, host, &mut *s.call);
        }
    }

    /// Step 0x15 of [`ms_btl_read_manage`]: the first strike's message
    /// (`MsMessageCueRegist`, message 0x11): the party's (kind 2), the
    /// monsters' (kind 3; 100 frames on [`LONG_AMBUSH_SCENE`], else 0x19).
    fn first_strike_message<M: Mem>(mem: &mut M, call: Call<'_, M>) {
        let (kind, time) = match FIRST_ATTACK.get(mem) {
            1 => (2, 0x19),
            2 if crate::state::bridge::ms_get_battle_scene(mem) == LONG_AMBUSH_SCENE => (3, 100),
            2 => (3, 0x19),
            _ => return,
        };
        crate::state::bridge::ms_message_cue_regist(mem, kind, 0, 0, [0x11, time], |m| {
            game(
                m,
                &mut *call,
                crate::death::bridge::TO_BTL_CLOSE_SIGN_WINDOW,
                &[],
            );
        });
    }

    /// The next monster slot to read (`i8`, 0-7; `MsBtlReadMonster`).
    pub const NEXT_MONSTER: Global<i8> = Global::new(0x0112_a92a);
    /// The monsters' kernel text tables: IDs below 0x65, 0x65-0xb4, the
    /// rest (and the special three).
    pub const MONSTER_TEXTS: [u32; 3] = [0x0112_ca6c, 0x0112_ca70, 0x0112_ca74];
    /// `MsGetChrID(actor)`, `readBtlMonsterName()`, `Mon_AttachData(i)`,
    /// `Mon_LoadAll()` (battle, not ported), `"mon_data"` and the monster
    /// file read's completion (`0x00783990`).
    pub const MS_GET_CHR_ID: u32 = 0x0079_a440;
    pub const READ_BTL_MONSTER_NAME: u32 = 0x0078_44c0;
    pub const MON_ATTACH_DATA: u32 = 0x0078_3b00;
    pub const MON_LOAD_ALL: u32 = 0x0078_4070;
    pub const MON_DATA_NAME: u32 = 0x00b5_4920;
    pub const MONSTER_READ_DONE: u32 = 0x0078_3990;
    /// The request block the monster file's read gets (`+0xc` the slot).
    pub const MONSTER_REQUEST: u32 = 0x10;
    /// The monster IDs whose name is a character's default name, by the
    /// file kind they set (`g_BtlFileKind`).
    pub const NAMED_MONSTERS: [(u32, u32); 3] = [(0xfe, 8), (0x11c, 9), (0x11f, 0xc)];

    /// A text of at most `max` bytes from `from` into the cleared `len`
    /// bytes at `to` (the original's `memset`, `strlen`, `memcpy`).
    fn text(mem: &mut impl Mem, to: u32, len: u32, from: u32, max: u32) {
        mem.write_bytes(to, &vec![0; len as usize]);
        let mut n = 0;
        while mem.read::<u8>(from.wrapping_add(n)) != 0 {
            n += 1;
        }
        let n = n.min(max);
        let mut bytes = vec![0; n as usize];
        mem.read_bytes(from, &mut bytes);
        mem.write_bytes(to, &bytes);
    }

    /// `MsBtlReadMonster()` (`0x00783680`): status 0xd; from slot
    /// [`NEXT_MONSTER`] on, each monster with an ID (`+0xe`) and a file
    /// (directory 0x10, `MsGetChrID`, CD `+0x14`) shares the data of an
    /// earlier slot with the same ID (`+0x48`; then `+0x11` 0 and `+0x14`
    /// that slot + 0x14), gets its battle name (`readBtlMonsterName`) and
    /// its kernel texts (by ID: name `+0x50`, 0x3f bytes; help `+0x90`,
    /// 0xff; scan `+0x190`, 0x1ff; the table by [`MONSTER_TEXTS`]), or for
    /// the [`NAMED_MONSTERS`] while `g_BtlFileKind` is set, a character's
    /// default name (the kind then 1). A shared one is attached
    /// (`Mon_AttachData`) and the next slot tried; else its file is read
    /// into a new block (`Sg_MallocA(n, 0x10, 5, "mon_data", -1)`, CD `+0`
    /// with [`MONSTER_READ_DONE`] and `request`'s `+0xc` the slot), the
    /// next slot noted, -1. With none left `Mon_LoadAll`, status 0xe, 0.
    #[allow(clippy::too_many_lines)]
    pub fn ms_btl_read_monster<M: Mem>(mem: &mut M, call: Call<'_, M>, request: u32) -> u32 {
        use crate::actor::bridge::ms_get_mon;
        let cd = |mem: &mut M, call: Call<'_, M>, slot: u32, args: &[u32]| {
            let obj = game(mem, &mut *call, crate::getmem::bridge::GET_CD, &[]);
            let f: u32 = mem.read(obj.wrapping_add(slot));
            call(mem, f, args)
        };
        STATUS.set(mem, 0xd);
        let mut i = i32::from(NEXT_MONSTER.get(mem));
        while i < 8 {
            let m = ms_get_mon(mem, i.cast_unsigned());
            if mem.read::<u16>(m.wrapping_add(0xe)) == 0xffff {
                i += 1;
                continue;
            }
            cd(mem, &mut *call, 0x10, &[0x10]);
            let id = game(mem, &mut *call, MS_GET_CHR_ID, &[m]);
            let size = cd(mem, &mut *call, 0x14, &[id]);
            if size == 0 {
                i += 1;
                continue;
            }
            let mut shared = 0;
            for j in 0..i.max(0) {
                let other = ms_get_mon(mem, j.cast_unsigned());
                let same: u16 = mem.read(other.wrapping_add(0xe));
                let data: u32 = mem.read(other.wrapping_add(0x48));
                if same == mem.read::<u16>(m.wrapping_add(0xe)) && data != 0 {
                    shared = data;
                    mem.write(m.wrapping_add(0x11), 0u8);
                    mem.write(m.wrapping_add(0x14), j.to_le_bytes()[0].wrapping_add(0x14));
                    break;
                }
            }
            game(mem, &mut *call, READ_BTL_MONSTER_NAME, &[]);
            let named = if FILE_KIND.get(mem) == 0 {
                None
            } else {
                NAMED_MONSTERS
                    .iter()
                    .find(|&&(k, _)| k == id)
                    .map(|&(_, kind)| kind)
            };
            let table = match named {
                Some(kind) => {
                    FILE_KIND.set(mem, kind);
                    MONSTER_TEXTS[2]
                }
                None if id.cast_signed() < 0x65 => MONSTER_TEXTS[0],
                None if id.wrapping_sub(0x65) <= 0x4f => MONSTER_TEXTS[1],
                None => MONSTER_TEXTS[2],
            };
            let table: u32 = Global::<u32>::new(table).get(mem);
            if table != 0 {
                let (r, strings) = ffx_kernel::bridge::excel_data(mem, id.cast_signed(), table);
                for (field, at, len) in [
                    (0u32, 0x50u32, 0x40u32),
                    (2, 0x90, 0x100),
                    (6, 0x190, 0x200),
                ] {
                    let from =
                        u32::from(mem.read::<u16>(r.wrapping_add(2 * field))).wrapping_add(strings);
                    text(mem, m.wrapping_add(at), len, from, len - 1);
                }
            }
            let kind = FILE_KIND.get(mem);
            if kind.cast_signed() > 1 {
                let name = crate::save::bridge::get_default_chr_name(mem, kind);
                text(mem, m.wrapping_add(0x50), 0x40, name, u32::MAX);
                FILE_KIND.set(mem, 1);
            }
            if shared == 0 {
                let tag = code(mem, MON_DATA_NAME);
                let block = game(
                    mem,
                    &mut *call,
                    crate::read::bridge::SG_MALLOC_A,
                    &[size, 0x10, 5, tag, u32::MAX],
                );
                mem.write(m.wrapping_add(0x48), block);
                mem.write(request.wrapping_add(0xc), i);
                NEXT_MONSTER.set(mem, i.to_le_bytes()[0].cast_signed().wrapping_add(1));
                let done = code(mem, MONSTER_READ_DONE);
                cd(mem, call, 0, &[id, block, done, request]);
                return u32::MAX;
            }
            mem.write(m.wrapping_add(0x48), shared);
            game(mem, &mut *call, MON_ATTACH_DATA, &[i.cast_unsigned()]);
            i += 1;
        }
        game(mem, call, MON_LOAD_ALL, &[]);
        STATUS.set(mem, 0xe);
        0
    }

    /// `MsMonShuffleVariants_007842f0()` (`0x007842f0`, name ours; caller
    /// `Mon_LoadAll`): the monsters (0x14-0x1b) sharing a model each get
    /// their own variant (`+0x424`). For each shown monster (`+0x10`,
    /// `+0x11`) with later ones whose model owner (`+0x14`) it is, `n`
    /// (1 plus those) variants `(brnd & 3) + 8 m` are drawn (`m` from 0,
    /// the generator of `MsGetRndChr(monster, 0)`); then every monster
    /// from it on that it owns takes one of the left ones at random
    /// (`brnd % n`, its own generator), which leaves the list.
    pub fn ms_mon_shuffle_variants(mem: &mut impl Mem) {
        use crate::damage::bridge::ms_get_rnd_chr;
        use crate::rng::bridge::brnd;
        fn owner(mem: &impl Mem, j: u32) -> u32 {
            u32::from(mem.read::<u8>(ms_get_chr(mem, j).wrapping_add(0x14)))
        }
        for k in 0x14..0x1c {
            let a = ms_get_chr(mem, k);
            if mem.read::<u8>(a.wrapping_add(0x10)) == 0
                || mem.read::<u8>(a.wrapping_add(0x11)) == 0
            {
                continue;
            }
            let mut n = 1 + (k + 1..0x1c).filter(|&j| owner(mem, j) == k).count();
            if n <= 1 {
                continue;
            }
            let mut left = [0u8; 8];
            for (m, v) in (0u8..).zip(left.iter_mut().take(n)) {
                let r = brnd(mem, ms_get_rnd_chr(mem, k, 0));
                *v = (r.to_le_bytes()[0] & 3).wrapping_add(m << 3);
            }
            for j in k..0x1c {
                let b = ms_get_chr(mem, j);
                if u32::from(mem.read::<u8>(b.wrapping_add(0x14))) != k {
                    continue;
                }
                let r = brnd(mem, ms_get_rnd_chr(mem, j, 0));
                let pick = r as usize % n;
                n -= 1;
                mem.write(b.wrapping_add(0x424), left[pick]);
                left.copy_within(pick + 1..=n, pick);
            }
        }
    }
}
