//! The battle's start (`docs/rewrite-plan.md`, M2): `MsBattleExe`
//! (`0x00781020`) loads a formation's battle file and resets the battle's
//! state before `MsBtlReadManage` reads the rest.
//!
//! Other modules' functions (the CD object, the heap, main's `Sg_*`, the
//! C runtime's `sprintf`, script, system) and the battle functions not
//! ported here go to the game's code through `s.call`.

/// The game's memory, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::{Global, IMAGE_BASE, Mem};

    use crate::atel::bridge::Script;
    use crate::target::bridge::Dm;

    fn code(mem: &impl Mem, sym: u32) -> u32 {
        sym.wrapping_sub(IMAGE_BASE).wrapping_add(mem.image_base())
    }

    fn game<'d>(mem: &mut Dm<'d>, s: &mut Script<'_, Dm<'d>>, sym: u32, args: &[u32]) -> u32 {
        let f = code(mem, sym);
        (s.call)(mem, f, args)
    }

    /// A store of [`ms_battle_exe`]'s reset: a constant or an argument.
    #[derive(Clone, Copy)]
    enum Exe {
        K(u32),
        Cdpos,
        Group,
        Slot,
        Mode,
        Field,
    }

    /// `MsBattleExe`'s state reset, in its order: address (`ffx.sym`),
    /// width, value. Some stores overlap their neighbours' bytes (the
    /// original's own widths).
    const RESET: [(u32, u32, Exe); 68] = [
        (0x0112_c948, 2, Exe::K(0)),
        (0x0112_be98, 4, Exe::K(0)),
        (0x0112_be9c, 4, Exe::K(0)),
        (0x0112_c02b, 1, Exe::K(0)),
        (0x0112_c9cc, 1, Exe::K(0)),
        (0x0112_a8ec, 4, Exe::Cdpos),
        (0x0112_c258, 1, Exe::Group),
        (0x00c4_21d4, 4, Exe::Group),
        (0x0112_c259, 1, Exe::Slot),
        (0x0112_c94a, 2, Exe::K(0)),
        (0x0112_c022, 2, Exe::K(0x0)),
        (0x0112_c9d9, 1, Exe::Mode),
        (0x0112_c9db, 4, Exe::K(0x0001_0000)),
        (0x0112_c9e2, 2, Exe::K(0xffff)),
        (0x0112_c9df, 2, Exe::K(0xffff)),
        (0x0112_c9e1, 1, Exe::K(0xff)),
        (0x0112_c9d6, 2, Exe::K(0x0)),
        (0x0112_c9d8, 1, Exe::K(0x0)),
        (0x0112_bde1, 2, Exe::K(0x0)),
        (0x0112_a8e3, 2, Exe::K(0x0)),
        (0x0112_c9e4, 4, Exe::K(0xff)),
        (0x0112_c9fb, 1, Exe::K(0x0)),
        (0x0112_bde0, 1, Exe::K(0x0)),
        (0x0112_c256, 2, Exe::Field),
        (0x00c4_21d0, 4, Exe::Field),
        (0x0112_a8e6, 2, Exe::K(0x0)),
        (0x0112_a8e8, 1, Exe::K(0x0)),
        (0x0112_bded, 2, Exe::K(0x0)),
        (0x0112_bdec, 1, Exe::K(0x0)),
        (0x0112_c8c8, 4, Exe::K(0x0)),
        (0x0112_c8cc, 4, Exe::K(0x0)),
        (0x0112_c8d0, 4, Exe::K(0x0)),
        (0x0112_c8d4, 4, Exe::K(0x0)),
        (0x0112_c8d8, 4, Exe::K(0x0)),
        (0x0112_c8dc, 4, Exe::K(0xff)),
        (0x0112_c8f0, 4, Exe::K(0x0)),
        (0x0112_c8f4, 4, Exe::K(0xffff_ffff)),
        (0x0112_c8f8, 4, Exe::K(0x0)),
        (0x0112_c8fc, 4, Exe::K(0xffff_ffff)),
        (0x0112_c900, 4, Exe::K(0x0)),
        (0x0112_a8fe, 1, Exe::K(0x0)),
        (0x0112_a919, 1, Exe::K(0x0)),
        (0x0112_ca00, 1, Exe::K(0x1)),
        (0x0112_ca02, 4, Exe::K(0x0)),
        (0x0112_c9da, 1, Exe::K(0x0)),
        (0x0112_c9e8, 4, Exe::K(0xff00)),
        (0x0112_ca06, 1, Exe::K(0x0)),
        (0x0112_ca08, 2, Exe::K(0x0)),
        (0x0112_ca0f, 4, Exe::K(0x0100_0000)),
        (0x0112_ca13, 4, Exe::K(0x0)),
        (0x0112_c94e, 2, Exe::K(0x29e)),
        (0x0112_c9d0, 4, Exe::K(0x0)),
        (0x0112_c9ec, 2, Exe::K(0x0)),
        (0x0112_c9ef, 1, Exe::K(0x0)),
        (0x0112_c9f1, 2, Exe::K(0x0)),
        (0x0112_c9f4, 4, Exe::K(0x0)),
        (0x0112_a8f0, 4, Exe::K(0x0)),
        (0x0112_c8e0, 4, Exe::K(0x0)),
        (0x0112_c8e4, 4, Exe::K(0x0)),
        (0x0112_c8e8, 4, Exe::K(0x0)),
        (0x0112_c8ec, 4, Exe::K(0xffff_ffff)),
        (0x0112_c9f3, 1, Exe::K(0x0)),
        (0x0112_c9cd, 1, Exe::K(0x0)),
        (0x0112_c9fc, 4, Exe::K(0x1)),
        (0x0112_a928, 2, Exe::K(0x0)),
        (0x0112_a92b, 1, Exe::K(0x0)),
        (0x0112_c9d4, 2, Exe::K(0x0)),
        (0x0112_c9fa, 1, Exe::K(0x0)),
    ];

    /// `FFXIsDebug()` (system), `SsSetProjectBase(path)` (sound),
    /// `Sg_MemSetAlign(n)`, `Sg_Blur(on)`, `Sg_FadeIn(on)`,
    /// `Sg_SetExecFrames(n)`, `Sg_SetPlayer(on)`,
    /// `Sg_MallocA(n, align, pool, name, flags)`, `Sg_SetKeepFps(on)`
    /// (main), `AtelEventSetUpForBattleMusic(n)` (script),
    /// `TOInitBtlWindow()` (system), `MsBtlReadManage()` (battle, ported in
    /// `read`; the original's tail call, so its `eax` stays the result) and
    /// the battle inits called through the game.
    pub const FFX_IS_DEBUG: u32 = 0x0088_7c50;
    pub const SS_SET_PROJECT_BASE: u32 = 0x0088_7a90;
    pub const SG_MEM_SET_ALIGN: u32 = 0x0083_c4a0;
    pub const SG_BLUR: u32 = 0x0082_bee0;
    pub const MS_SCREEN_FOCUS_RESET_ALL: u32 = 0x007b_bda0;
    pub const SG_FADE_IN: u32 = 0x0082_cb80;
    pub const SG_SET_EXEC_FRAMES: u32 = 0x0082_da60;
    pub const SG_SET_PLAYER: u32 = 0x0082_dad0;
    pub const SG_MALLOC_A: u32 = 0x0083_f0a0;
    pub const MS_MEM_CHECK_REGIST: u32 = 0x0079_9f00;
    pub const SG_SET_KEEP_FPS: u32 = 0x0082_1ae0;
    pub const ATEL_EVENT_SET_UP_FOR_BATTLE_MUSIC: u32 = 0x0086_6240;
    pub const TO_INIT_BTL_WINDOW: u32 = 0x0089_b660;
    pub const MS_BTL_READ_MANAGE: u32 = 0x0078_3020;
    pub const MS_DEBUG_SET_SCENE: u32 = 0x007c_5f60;
    pub const MS_BTL_START_AP: u32 = 0x0079_8e90;
    pub const MS_SET_ITEM_WINDOW: u32 = 0x0079_06a0;
    /// The inits between the file's buffer and the reset, in order:
    /// `MsBtlReadInit`, `MsSEPtransInit`, `MsSEP3Dinit`, `ResetEncountExe(0)`,
    /// `MsSetGlobals_0079cdd0`, `MsEffectCameraBufInit`.
    pub const INITS: [(u32, &[u32]); 6] = [
        (0x0078_3000, &[]),
        (0x007a_0500, &[]),
        (0x007a_0910, &[]),
        (0x0078_0ff0, &[0]),
        (0x0079_cdd0, &[]),
        (0x0078_76e0, &[]),
    ];
    /// `MsCameraPriorityMask(on)`, then `MsInitMimic`, `MsComBufInit`,
    /// `MsMessageCueInit`.
    pub const MS_CAMERA_PRIORITY_MASK: u32 = 0x007b_e100;
    pub const INITS2: [u32; 3] = [0x0079_9ab0, 0x0079_d290, 0x007a_2730];
    /// The C runtime's `sprintf` import slot.
    pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;
    /// `"/usr/local/ffx/%s/ps2/sound"`, the path buffer and its name;
    /// `"btlbin"`; `"%s_%02d"`.
    pub const SOUND_PATH_FORMAT: u32 = 0x00b5_4764;
    pub const SOUND_PATH: u32 = 0x0112_ca38;
    pub const SOUND_PROJECT: u32 = 0x0231_1420;
    pub const BTLBIN: u32 = 0x00b5_4780;
    pub const SCENE_NAME_FORMAT: u32 = 0x00b5_4788;
    /// The battle file's name (`<map>_<nn>`, 8 bytes from the field record
    /// then the formation number).
    pub const SCENE_NAME: Global<[u8; 9]> = Global::new(0x0112_c25a);
    /// The formation's battle music (`MsBattleExe`: its group's bytes 1, 2).
    pub const BATTLE_MUSIC: Global<u16> = Global::new(0x0112_c254);
    /// The file's size (low half) and buffer (`g_BtlScriptRecord1`'s
    /// `+2`, `+4`).
    pub const FILE_SIZE: Global<u16> = Global::new(0x0112_a9a6);
    pub const FILE_BUFFER: Global<u32> = Global::new(0x0112_a9a8);
    /// `MsCameraPriorityMask`'s argument (`i8`).
    pub const PRIORITY_MASK_ARG: Global<i8> = Global::new(0x0112_a924);
    /// Something `MsBattleExe` sets from 0x00b92194 (`f32`).
    pub const EXE_FLOAT: Global<u32> = Global::new(0x0112_bde4);
    pub const EXE_FLOAT_INIT: Global<u32> = Global::new(0x00b9_2194);
    /// The scene's list entries (`MsBattleExe`): field, encounters, group,
    /// and the formation's byte in the group.
    pub const SCENE_ENCOUNT: Global<u32> = Global::new(0x0112_a9f4);
    pub const SCENE_GROUP_ENTRY: Global<u32> = Global::new(0x0112_a9f8);
    /// The encounter type byte (`MsBattleExe` asks its effect).
    pub const ENCOUNT_TYPE: Global<i8> = Global::new(0x0112_a9d5);
    /// `Sg_SetKeepFps(1)`'s result, kept.
    pub const KEEP_FPS: Global<u8> = Global::new(0x0112_c9f8);

    fn sprintf<'d>(mem: &mut Dm<'d>, s: &mut Script<'_, Dm<'d>>, args: &[u32]) {
        let f: u32 = Global::<u32>::new(SPRINTF_SLOT).get(mem);
        (s.call)(mem, f, args);
    }

    /// `MsBattleExe(mode, field, group, slot)` (`0x00781020`): starts
    /// formation `slot` of `group` on `field`: for a battle from the field
    /// (`mode` 0) the CD object's third method (`GetCD`, with 0; on a debug
    /// build after the sound project path, `SsSetProjectBase`); the event
    /// effect ended (`MsEventEffectFree`); the file's number
    /// (`MsBtlListCDpos`, which clamps `group` and `slot`) and size
    /// (`MsBattleExeSetIndex`), 0 and nothing more without one. The group's
    /// music; the file's buffer: from the field the screen reset (blur off,
    /// focus, fade, one frame, no player), the battle before freed
    /// (`MsBattleFree`) and a fresh block (`Sg_MallocA(n, 0x10, 5,
    /// "btlbin", -1)`); else script record 0's or a new one (aligned 0x40,
    /// `user_malloc`, `MsMemCheckRegist`). The inits, the reset
    /// ([`RESET`]), the scene's list entries and the battle file's name
    /// (`"%s_%02d"`); then status 5 (from an event) or, from the field, the
    /// actors', items' and numbers' memory, AP, the item and battle
    /// windows, the effects, the encounter effect (`0x007829b0`,
    /// `MsEtEffectEncount`), the music, status 4; and `MsBtlReadManage`,
    /// whose result is the result.
    ///
    /// The original passes its own `group`, `slot` (and later `field`'s
    /// slot) argument slots as outputs; the port keeps them in locals.
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn ms_battle_exe<'d>(
        mem: &mut Dm<'d>,
        s: &mut Script<'_, Dm<'d>>,
        mode: u32,
        field: u32,
        group: u32,
        slot: u32,
    ) -> u32 {
        use crate::getmem::bridge as gm;
        use crate::state::bridge::{SCENE_FIELD, SCENE_GROUP};
        let debug = game(mem, s, FFX_IS_DEBUG, &[]) != 0;
        if mode == 0 {
            if debug {
                let args = [
                    code(mem, SOUND_PATH),
                    code(mem, SOUND_PATH_FORMAT),
                    code(mem, SOUND_PROJECT),
                ];
                sprintf(mem, s, &args);
                let path = code(mem, SOUND_PATH);
                game(mem, s, SS_SET_PROJECT_BASE, &[path]);
            }
            let cd = game(mem, s, gm::GET_CD, &[]);
            let f: u32 = mem.read(cd.wrapping_add(8));
            (s.call)(mem, f, &[0]);
        }
        gm::ms_event_effect_free(mem, s);
        crate::state::bridge::EXE.set(mem, 0);
        mem.locals(0xc, |mem, at| {
            let (gp, sp, op) = (at, at + 4, at + 8);
            mem.write(gp, group);
            mem.write(sp, slot);
            let cdpos = gm::ms_btl_list_cd_pos(mem, field, gp, sp);
            let size = gm::ms_battle_exe_set_index(mem, &mut *s.call, mode, cdpos);
            if size == 0 {
                return 0;
            }
            let group: u32 = mem.read(gp);
            let slot: u32 = mem.read(sp);
            let f = field.cast_signed();
            let g = crate::flow::bridge::ms_btl_list_group(mem, f, group.cast_signed());
            let music = u16::from(mem.read::<u8>(g.wrapping_add(2)))
                .wrapping_mul(0x100)
                .wrapping_add(u16::from(mem.read::<u8>(g.wrapping_add(1))));
            BATTLE_MUSIC.set(mem, music);
            let buffer = if mode != 0 {
                let rec0 = crate::atel::bridge::SCRIPT_RECORDS.ptr(mem).addr + 4;
                let p: u32 = mem.read(rec0);
                if p == 0 {
                    game(mem, s, SG_MEM_SET_ALIGN, &[0x40]);
                    let p = game(mem, s, gm::USER_MALLOC, &[size]);
                    game(mem, s, MS_MEM_CHECK_REGIST, &[p, size]);
                    p
                } else {
                    p
                }
            } else {
                game(mem, s, SG_BLUR, &[0]);
                game(mem, s, MS_SCREEN_FOCUS_RESET_ALL, &[]);
                game(mem, s, SG_FADE_IN, &[1]);
                game(mem, s, SG_SET_EXEC_FRAMES, &[1]);
                game(mem, s, SG_SET_PLAYER, &[0]);
                crate::free::bridge::ms_battle_free(mem, s);
                let name = code(mem, BTLBIN);
                game(mem, s, SG_MALLOC_A, &[size, 0x10, 5, name, u32::MAX])
            };
            #[allow(clippy::cast_possible_truncation)] // the original's halfword store
            FILE_SIZE.set(mem, size as u16);
            FILE_BUFFER.set(mem, buffer);
            for (f, args) in INITS {
                game(mem, s, f, args);
            }
            let mask = i32::from(PRIORITY_MASK_ARG.get(mem)).cast_unsigned();
            game(mem, s, MS_CAMERA_PRIORITY_MASK, &[mask]);
            for f in INITS2 {
                game(mem, s, f, &[]);
            }
            let v = ffx_bridge::x87::store(ffx_bridge::x87::load(EXE_FLOAT_INIT.get(mem)));
            EXE_FLOAT.set(mem, v);
            for (a, width, v) in RESET {
                let v = match v {
                    Exe::K(k) => k,
                    Exe::Cdpos => cdpos,
                    Exe::Group => group,
                    Exe::Slot => slot,
                    Exe::Mode => mode,
                    Exe::Field => field,
                };
                let at = Global::<u8>::new(a).ptr(mem).addr;
                let bytes = v.to_le_bytes();
                mem.write_bytes(at, &bytes[..width as usize]);
            }
            let rec = crate::minor::bridge::ms_btl_list_field(mem, f);
            SCENE_FIELD.set(mem, rec);
            let e = crate::minor::bridge::ms_btl_list_encount(mem, f);
            SCENE_ENCOUNT.set(mem, e);
            let g = crate::flow::bridge::ms_btl_list_group(mem, f, group.cast_signed());
            SCENE_GROUP_ENTRY.set(mem, g);
            let entry = g.wrapping_add(slot.wrapping_mul(2)).wrapping_add(5);
            SCENE_GROUP.set(mem, entry);
            if mode == 0 && game(mem, s, FFX_IS_DEBUG, &[]) != 0 {
                let map = i32::from(mem.read::<i16>(rec)).cast_unsigned();
                let n = u32::from(mem.read::<u8>(entry));
                game(mem, s, MS_DEBUG_SET_SCENE, &[field, group, slot, map, n]);
            }
            let mut name = [0u8; 9];
            for (k, b) in (0u32..).zip(name.iter_mut().take(8)) {
                *b = mem.read(rec.wrapping_add(6 + k));
            }
            SCENE_NAME.set(mem, name);
            let buf = SCENE_NAME.ptr(mem).addr;
            let n = u32::from(mem.read::<u8>(entry));
            let fmt = code(mem, SCENE_NAME_FORMAT);
            sprintf(mem, s, &[buf, fmt, buf, n]);
            let status = crate::state::bridge::STATUS;
            if mode != 0 {
                status.set(mem, 5);
                return game(mem, s, MS_BTL_READ_MANAGE, &[]);
            }
            let keep = game(mem, s, SG_SET_KEEP_FPS, &[1]);
            #[allow(clippy::cast_possible_truncation)] // the low byte
            KEEP_FPS.set(mem, keep as u8);
            gm::ms_btl_chr_get_mem(mem, &mut *s.call);
            gm::ms_btl_get_item_mem(mem, &mut *s.call);
            gm::ms_number_get_mem(mem, &mut *s.call);
            game(mem, s, MS_BTL_START_AP, &[]);
            game(mem, s, MS_SET_ITEM_WINDOW, &[]);
            game(mem, s, TO_INIT_BTL_WINDOW, &[]);
            gm::ms_et_effect_set(mem, &mut *s.call);
            mem.write(op, cdpos);
            let kind = i32::from(ENCOUNT_TYPE.get(mem)).cast_unsigned();
            let k = gm::ms_btl_encount_effect(mem, &mut *s.call, kind, op);
            if k.cast_signed() >= 0 {
                let se: u32 = mem.read(op);
                gm::ms_et_effect_encount(mem, &mut *s.call, k, se);
            }
            let music = i32::from(BATTLE_MUSIC.get(mem).cast_signed()).cast_unsigned();
            game(mem, s, ATEL_EVENT_SET_UP_FOR_BATTLE_MUSIC, &[music]);
            status.set(mem, 4);
            game(mem, s, MS_BTL_READ_MANAGE, &[])
        })
    }
}
