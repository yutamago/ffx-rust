//! The battle's small bookkeeping (`docs/rewrite-plan.md`, M2): the
//! subsystems' init functions, which reset their globals at battle start,
//! and short setters and getters around the camera, motions and summons.
//! What each global is, beyond who resets it, is open for most of them;
//! they are named after their functions.

/// The game's memory, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::{Global, Mem, Ptr};

    /// Reset by `MsComBufInit`.
    pub const COM_BUF: Global<u32> = Global::new(0x0113_6070);
    /// Reset by `MsEtEffectReadInit`; `MsEtEffectStop` stops an effect
    /// while it is set.
    pub const ET_EFFECT_READ: Global<u32> = Global::new(0x0113_60a8);
    /// Reset by `MsNumberInit` (the damage numbers).
    pub const NUMBERS: Global<u32> = Global::new(0x0113_60b8);
    /// Set to -1 by `MsActionInit`.
    pub const ACTION: Global<i32> = Global::new(0x0113_6a68);
    /// Reset by `MsEffectCameraReset`.
    pub const EFFECT_CAMERA: Global<u32> = Global::new(0x0231_142c);
    /// The user character ID `MsSaveUcID` keeps.
    pub const UC_ID: Global<u16> = Global::new(0x0113_f3d2);
    /// The battle font (`MsBattleGetFontAddress`).
    pub const FONT: Global<u32> = Global::new(0x0112_a9bc);
    /// Reset by `MsRegMotionInit`, with [`REG_MOTION_FLAG`].
    pub const REG_MOTION: Global<u32> = Global::new(0x0113_606c);
    pub const REG_MOTION_FLAG: Global<u8> = Global::new(0x0112_c9cf);
    /// The AP of 18 actors at battle start (`MsBtlStartAP` copies the
    /// current ones, 0x48 bytes on, here).
    pub const START_AP: Global<[u32; 18]> = Global::new(0x0231_0ed8);
    /// Reset by `MsBtlInitItemMem`.
    pub const ITEM_MEM: Global<u32> = Global::new(0x0113_6068);
    pub const ITEM_MEM2: Global<u32> = Global::new(0x0231_0cd8);
    /// Reset by `MsBtlInitItemMem_007a0cd0`.
    pub const ITEM_MEM3: Global<[u32; 2]> = Global::new(0x0113_63b0);
    /// Reset by `MsMessageCueInit` (a `u32` at `0x0112ca0a`, which covers
    /// the stone effect counters from `0x0112ca0b`, and the cue's count and
    /// start).
    pub const MESSAGE_CUE_INIT: Global<u32> = Global::new(0x0112_ca0a);
    pub const MESSAGE_CUE_INIT2: Global<u32> = Global::new(0x0112_c90c);
    /// Reset by `MsSplineInit`.
    pub const SPLINES: Global<[u32; 2]> = Global::new(0x0113_6b60);
    /// The screen size `MsScreenInit` stores.
    pub const SCREEN_SIZE: Global<[u32; 2]> = Global::new(0x00c4_4240);
    /// The effect buffer and its size (`MsEffectGetBufferAddress`).
    pub const EFFECT_BUFFER: Global<u32> = Global::new(0x0112_be98);
    pub const EFFECT_BUFFER_SIZE: Global<u32> = Global::new(0x0112_be9c);
    /// The last magic of kind 2 (`MsEffectGetLastMagicNum2`).
    pub const LAST_MAGIC2: Global<u32> = Global::new(0x0112_c8e8);
    /// The special camera: on, saved camera, camera (`MsBattleSpecialCamera*`).
    pub const SPECIAL_CAMERA_ON: Global<u8> = Global::new(0x0112_a8e8);
    pub const SPECIAL_CAMERA_SAVED: Global<u8> = Global::new(0x0112_a8e9);
    pub const SPECIAL_CAMERA: Global<u8> = Global::new(0x0112_a8e7);
    /// The read cue: entries of 12 bytes from `0x0112bea0`, the current one
    /// (`MsBtlReadStatusComplete` marks its `+2` done).
    pub const READ_CUE: Global<u8> = Global::new(0x0112_bea0);
    pub const READ_CUE_INDEX: Global<u8> = Global::new(0x0112_c020);
    /// The menu command `MsSetMenuCommand` copies (0x48 bytes) and its flag.
    pub const MENU_COMMAND: Global<[u32; 0x12]> = Global::new(0x0112_bdf0);
    pub const MENU_COMMAND_SET: Global<u8> = Global::new(0x0112_c9e6);
    /// The three screens' focus, -1 for none (`MsScreenFocusReset`), 0x1c0
    /// bytes apart.
    pub const SCREEN_FOCUS: Global<i32> = Global::new(0x0113_70d0);
    /// Pointer to the screen in use (`MsScreenOffset`: its offset at
    /// `+0x30`).
    pub const SCREEN: Global<u32> = Global::new(0x0231_14c0);
    /// The offset `MsScreenResetOffset` keeps (x, y).
    pub const SCREEN_OFFSET: Global<[u32; 2]> = Global::new(0x0113_6fb8);
    /// Summons keep the overdrive gauge while this is set
    /// (`MsSummonLimitSave`).
    pub const SUMMON_LIMIT: Global<u8> = Global::new(0x0112_ca03);
    /// The battle field list (`MsBtlListInit`): the loaded file, its
    /// records' count and the two tables in it.
    pub const FIELD_LIST: Global<u32> = Global::new(0x0112_a9c4);
    pub const FIELD_COUNT: Global<u16> = Global::new(0x0112_a9d0);
    pub const FIELD_TABLE2: Global<u32> = Global::new(0x0112_a9cc);
    /// The three move splines (`MsMoveSplineSet`).
    pub const MOVE_SPLINES: Global<[u32; 3]> = Global::new(0x0113_6418);
    /// Reset by `MsBtlReadInit`.
    pub const BTL_READ: Global<u32> = Global::new(0x0112_ca80);
    pub const BTL_READ2: Global<u16> = Global::new(0x0112_a929);
    pub const BTL_READ3: Global<u32> = Global::new(0x0112_ca60);
    /// The move target position (`MsMoveTargetPos`).
    pub const MOVE_TARGET: Global<u8> = Global::new(0x0113_6a58);
    /// Reset by `FinishReadPhyreBattleMap`.
    pub const MAP_READ: Global<u32> = Global::new(0x0112_ca68);
    /// Set to -1 by `MsDebugCameraSleepInit`.
    pub const DEBUG_CAMERA_SLEEP: Global<i32> = Global::new(0x0113_34c8);
    /// 16 floats `MsEffectSetCameraBufFloat` sets.
    pub const EFFECT_CAMERA_FLOATS: Global<[u32; 16]> = Global::new(0x0113_6ad8);

    fn byte(mem: &impl Mem, p: u32, o: u32) -> u8 {
        Ptr::<u8>::new(p).field::<u8>(o).get(mem)
    }

    /// A float through `fld m32` (signalling NaNs come out quiet), as bits.
    fn x87_copy(bits: u32) -> u32 {
        ffx_bridge::x87::store(ffx_bridge::x87::load(bits))
    }

    /// `MsComBufInit` (`0x0079d290`).
    pub fn ms_com_buf_init(mem: &mut impl Mem) {
        COM_BUF.set(mem, 0);
    }

    /// `MsEtEffectReadInit` (`0x0079e730`).
    pub fn ms_et_effect_read_init(mem: &mut impl Mem) {
        ET_EFFECT_READ.set(mem, 0);
    }

    /// `MsNumberInit` (`0x0079f9f0`).
    pub fn ms_number_init(mem: &mut impl Mem) {
        NUMBERS.set(mem, 0);
    }

    /// `MsActionInit` (`0x007ace90`).
    pub fn ms_action_init(mem: &mut impl Mem) {
        ACTION.set(mem, -1);
    }

    /// `MsEffectCameraReset` (`0x007bbb00`).
    pub fn ms_effect_camera_reset(mem: &mut impl Mem) {
        EFFECT_CAMERA.set(mem, 0);
    }

    /// `MsSaveUcID(id)` (`0x007d42e0`).
    pub fn ms_save_uc_id(mem: &mut impl Mem, id: u16) {
        UC_ID.set(mem, id);
    }

    /// `MsBattleGetFontAddress` (`0x00781680`): the font while in battle,
    /// else null.
    pub fn ms_battle_get_font_address(mem: &impl Mem) -> u32 {
        if crate::state::bridge::STATUS.get(mem) == 0 {
            0
        } else {
            FONT.get(mem)
        }
    }

    /// `MsRegMotionInit` (`0x0079ccd0`).
    pub fn ms_reg_motion_init(mem: &mut impl Mem) {
        REG_MOTION.set(mem, 0);
        REG_MOTION_FLAG.set(mem, 0);
    }

    /// `MsBtlStartAP` (`0x00798e90`): 18 words from 0x48 bytes on copied to
    /// [`START_AP`].
    pub fn ms_btl_start_ap(mem: &mut impl Mem) {
        let at = START_AP.ptr(mem).addr;
        for i in 0..18 {
            let v: u32 = mem.read(at + 4 * i + 0x48);
            mem.write(at + 4 * i, v);
        }
    }

    /// `MsBtlInitItemMem` (`0x0079a000`).
    pub fn ms_btl_init_item_mem(mem: &mut impl Mem) {
        ITEM_MEM.set(mem, 0);
        ITEM_MEM2.set(mem, 0);
    }

    /// `MsBtlInitItemMem_007a0cd0` (`0x007a0cd0`).
    pub fn ms_btl_init_item_mem2(mem: &mut impl Mem) {
        ITEM_MEM3.set(mem, [0, 0]);
    }

    /// `MsMessageCueInit` (`0x007a2730`).
    pub fn ms_message_cue_init(mem: &mut impl Mem) {
        MESSAGE_CUE_INIT.set(mem, 0);
        MESSAGE_CUE_INIT2.set(mem, 0);
    }

    /// `MsSplineInit` (`0x007baaa0`).
    pub fn ms_spline_init(mem: &mut impl Mem) {
        SPLINES.set(mem, [0, 0]);
    }

    /// `MsScreenInit(w, h)` (`0x007c0540`).
    pub fn ms_screen_init(mem: &mut impl Mem, w: u32, h: u32) {
        SCREEN_SIZE.set(mem, [w, h]);
    }

    /// `MsResetDirMove(actor)` (`0x007ab170`).
    pub fn ms_reset_dir_move(mem: &mut impl Mem, actor: u32) {
        mem.write(actor.wrapping_add(0x42e), 0u8);
        mem.write(actor.wrapping_add(0x42b), 0u16);
    }

    /// `MsEffectGetBufferAddress(&size)` (`0x007b7eb0`): the effect buffer;
    /// its size to `size` when not null.
    pub fn ms_effect_get_buffer_address(mem: &mut impl Mem, size: u32) -> u32 {
        if size != 0 {
            let v = EFFECT_BUFFER_SIZE.get(mem);
            mem.write(size, v);
        }
        EFFECT_BUFFER.get(mem)
    }

    /// `MsEffectGetLastMagicNum2(id)` (`0x00787bf0`): [`LAST_MAGIC2`] for an
    /// ID of kind 2 (byte 1), else -1.
    pub fn ms_effect_get_last_magic_num2(mem: &impl Mem, id: u32) -> u32 {
        if id.to_le_bytes()[1] == 2 {
            LAST_MAGIC2.get(mem)
        } else {
            u32::MAX
        }
    }

    /// `MsPaymentProcess(actor)` (`0x007afa60`): 1 once its motion is over
    /// (`+0x432` clear).
    pub fn ms_payment_process(mem: &impl Mem, actor: u32) -> u32 {
        u32::from(byte(mem, actor, 0x432) == 0)
    }

    /// `MsBattleSpecialCameraFree` (`0x0079dd90`): back to the saved camera.
    pub fn ms_battle_special_camera_free(mem: &mut impl Mem) {
        if SPECIAL_CAMERA_ON.get(mem) != 0 {
            let c = SPECIAL_CAMERA_SAVED.get(mem);
            SPECIAL_CAMERA_ON.set(mem, 0);
            SPECIAL_CAMERA.set(mem, c);
        }
    }

    /// `MsBattleSpecialCameraPause(camera)` (`0x0079ddb0`): saves the
    /// camera and switches to `camera`, unless already paused.
    pub fn ms_battle_special_camera_pause(mem: &mut impl Mem, camera: u8) {
        if SPECIAL_CAMERA_ON.get(mem) == 0 {
            let c = SPECIAL_CAMERA.get(mem);
            SPECIAL_CAMERA_ON.set(mem, 1);
            SPECIAL_CAMERA_SAVED.set(mem, c);
            SPECIAL_CAMERA.set(mem, camera);
        }
    }

    /// `MsMotionSpeedSetNormal(actor)` (`0x007aa260`): speed 0x100.
    pub fn ms_motion_speed_set_normal(mem: &mut impl Mem, actor: u32) {
        mem.write(actor.wrapping_add(0x404), 0u8);
        mem.write(actor.wrapping_add(0x402), 0x100u16);
    }

    /// `msMoveReset(actor)` (`0x007aa280`).
    pub fn ms_move_reset(mem: &mut impl Mem, actor: u32) {
        mem.write(actor.wrapping_add(0x404), 1u8);
        mem.write(actor.wrapping_add(0x402), 0x100u16);
    }

    /// `MsBtlReadStatusComplete` (`0x007ad550`): marks the current read cue
    /// entry done (2) while there is a read cue level.
    pub fn ms_btl_read_status_complete(mem: &mut impl Mem) {
        if crate::state::bridge::READ_CUE_LEVEL.get(mem) > 0 {
            let i = u32::from(READ_CUE_INDEX.get(mem));
            let at = READ_CUE.ptr(mem).addr + 12 * i + 2;
            mem.write(at, 2u8);
        }
    }

    /// `MsSetMenuCommand(command)` (`0x00792e50`): copies the 0x48-byte
    /// command entry and flags it.
    pub fn ms_set_menu_command(mem: &mut impl Mem, command: u32) {
        MENU_COMMAND_SET.set(mem, 1);
        let c: [u32; 0x12] = mem.read(command);
        MENU_COMMAND.set(mem, c);
    }

    /// `MsScreenFocusReset` (`0x007bbd80`).
    pub fn ms_screen_focus_reset(mem: &mut impl Mem) {
        let at = SCREEN_FOCUS.ptr(mem).addr;
        for i in 0..3 {
            mem.write(at + 0x1c0 * i, -1i32);
        }
    }

    /// `MsScreenOffset(x, y)` (`0x007bc0c0`): the screen's offset.
    pub fn ms_screen_offset(mem: &mut impl Mem, x: u32, y: u32) {
        let (x, y) = (x87_copy(x), x87_copy(y));
        let s = SCREEN.get(mem);
        mem.write(s.wrapping_add(0x30), x);
        let s = SCREEN.get(mem);
        mem.write(s.wrapping_add(0x34), y);
    }

    /// `MsScreenResetOffset(x, y)` (`0x007bc720`): the same, kept in
    /// [`SCREEN_OFFSET`] too.
    pub fn ms_screen_reset_offset(mem: &mut impl Mem, x: u32, y: u32) {
        ms_screen_offset(mem, x, y);
        SCREEN_OFFSET.set(mem, [x87_copy(x), x87_copy(y)]);
    }

    /// `MsSummonLimitSave(actor)` (`0x007b06e0`): with [`SUMMON_LIMIT`],
    /// keeps the overdrive gauge (`+0x5bc` to `+0x5be`) and fills it.
    pub fn ms_summon_limit_save(mem: &mut impl Mem, actor: u32) {
        if SUMMON_LIMIT.get(mem) != 0 {
            let g = byte(mem, actor, 0x5bc);
            mem.write(actor.wrapping_add(0x5be), g);
            let max = byte(mem, actor, 0x5bd);
            mem.write(actor.wrapping_add(0x5bc), max);
            mem.write(actor.wrapping_add(0x5bf), 1u8);
        }
    }

    /// `MsSummonLimitLoad(actor)` (`0x007b06b0`): gives back the kept gauge.
    pub fn ms_summon_limit_load(mem: &mut impl Mem, actor: u32) {
        if byte(mem, actor, 0x5bf) != 0 {
            let g = byte(mem, actor, 0x5be);
            mem.write(actor.wrapping_add(0x5bf), 0u8);
            mem.write(actor.wrapping_add(0x5bc), g);
        }
    }

    /// `MsBtlListInit` (`0x0079d250`): the battle field list's tables (at
    /// the file's `+4` and `+8`) and record count (14 bytes each).
    pub fn ms_btl_list_init(mem: &mut impl Mem) {
        let t = FIELD_LIST.get(mem);
        let a: u32 = mem.read(t.wrapping_add(4));
        let b: u32 = mem.read(t.wrapping_add(8));
        let n = b.wrapping_sub(a) / 14;
        FIELD_COUNT.set(
            mem,
            n.to_le_bytes()[..2]
                .try_into()
                .map_or(0, u16::from_le_bytes),
        );
        crate::state::bridge::FIELD_RECORDS.set(mem, a.wrapping_add(t));
        FIELD_TABLE2.set(mem, b.wrapping_add(t));
    }

    /// `MsMoveSplineSet(i)` (`0x007ab140`): clears move spline `i` (0-2).
    pub fn ms_move_spline_set(mem: &mut impl Mem, i: u32) {
        if i <= 2 {
            let at = MOVE_SPLINES.ptr(mem).addr + 4 * i;
            mem.write(at, 0u32);
        }
    }

    /// `MsBtlReadInit` (`0x00783000`).
    pub fn ms_btl_read_init(mem: &mut impl Mem) {
        BTL_READ.set(mem, 0);
        BTL_READ2.set(mem, 0);
        BTL_READ3.set(mem, 0);
    }

    /// `MsMoveTargetPos` (`0x007ab160`).
    pub fn ms_move_target_pos(mem: &impl Mem) -> u32 {
        MOVE_TARGET.ptr(mem).addr
    }

    /// `FinishReadPhyreBattleMap` (`0x00782fd0`).
    pub fn finish_read_phyre_battle_map(mem: &mut impl Mem) {
        MAP_READ.set(mem, 0);
    }

    /// `MsDebugCameraSleepInit` (`0x00792200`).
    pub fn ms_debug_camera_sleep_init(mem: &mut impl Mem) {
        DEBUG_CAMERA_SLEEP.set(mem, -1);
    }

    /// `MsEffectSetCameraBufFloat(i, f)` (`0x007b7f50`): float `i` (0-15).
    pub fn ms_effect_set_camera_buf_float(mem: &mut impl Mem, i: u32, f: u32) {
        if i <= 15 {
            let at = EFFECT_CAMERA_FLOATS.ptr(mem).addr + 4 * i;
            mem.write(at, x87_copy(f));
        }
    }

    /// The word `GetGlobal00c44248_007c6db0` returns (read by the map name
    /// banner, `MapNameText_Show_006bb480`); what it holds is open.
    pub const GLOBAL_00C44248: Global<u32> = Global::new(0x00c4_4248);
    /// The screen data's aspect `setScreenDataAspectZ` stores (an `f32`).
    pub const SCREEN_ASPECT_Z: Global<u32> = Global::new(0x0113_701c);
    /// Four 4-byte reel entries (`MsGetReelEntry_007b19d0`): `+0` and `+1`
    /// bytes that `MsSetReelResult`'s helpers read.
    pub const REEL_ENTRIES: Global<[u32; 4]> = Global::new(0x0112_be80);
    /// 16 words after [`EFFECT_CAMERA_FLOATS`] (`MsEffectSetCameraBuf_007b7f30`).
    pub const EFFECT_CAMERA_WORDS: Global<[u32; 16]> = Global::new(0x0113_6b18);

    /// `GetGlobal00c44248_007c6db0()` (`0x007c6db0`).
    pub fn get_global_00c44248(mem: &impl Mem) -> u32 {
        GLOBAL_00C44248.get(mem)
    }

    /// `MsMenuSetWindowSystem(v)` (`0x007901f0`): the command flag byte
    /// (`MsSetBtlCommand` clears it) to the low byte of `v`.
    pub fn ms_menu_set_window_system(mem: &mut impl Mem, v: u32) {
        crate::action::bridge::SET_COMMAND_FLAG.set(mem, v.to_le_bytes()[0]);
    }

    /// `MsBattleEncountFlag(v)` (`0x00780fe0`): the encounter fade flag
    /// to bit 0 of `v`.
    pub fn ms_battle_encount_flag(mem: &mut impl Mem, v: u32) {
        crate::process::bridge::ENCOUNT_FADE.set(mem, v.to_le_bytes()[0] & 1);
    }

    /// `MsEffectWaitFlag(v)` (`0x00788b20`): the soft reset's wait flag to
    /// bit 0 of `v`.
    pub fn ms_effect_wait_flag(mem: &mut impl Mem, v: u32) {
        crate::process::bridge::RESET_WAIT2.set(mem, v.to_le_bytes()[0] & 1);
    }

    /// `setScreenDataAspectZ(f)` (`0x007c0e60`): [`SCREEN_ASPECT_Z`] = `f`
    /// (through `fld`/`fstp`).
    pub fn set_screen_data_aspect_z(mem: &mut impl Mem, f: u32) {
        SCREEN_ASPECT_Z.set(mem, x87_copy(f));
    }

    /// `MsCameraGetRollNow_007bd500()` (`0x007bd500`): the current camera's
    /// roll now (`[g_Camera] + 0xc90`), the `f32` left in ST0, as bits.
    pub fn ms_camera_get_roll_now(mem: &impl Mem) -> u32 {
        let camera = crate::camera::bridge::CAMERA.get(mem);
        x87_copy(mem.read(camera.wrapping_add(0xc90)))
    }

    /// `Clamp0To6_007a86e0(n)` (`0x007a86e0`): `n` (signed) within 0..=6.
    #[must_use]
    pub fn clamp0_to6(n: u32) -> u32 {
        n.cast_signed().clamp(0, 6).cast_unsigned()
    }

    /// `MsDebugSetAllSP_00784ae0()` (`0x00784ae0`): every one of the 18
    /// `ply_save` records' SP byte (`+0x3b`) to 0x62 (98). `EAX` ends one
    /// record past the last.
    pub fn ms_debug_set_all_sp(mem: &mut impl Mem) -> u32 {
        let mut at = crate::save::bridge::PLY_SAVE.ptr(mem).addr + 0x3b;
        for _ in 0..18 {
            mem.write(at, 0x62u8);
            at += 0x94;
        }
        at
    }

    /// `MsGetReelEntry_007b19d0(i)` (`0x007b19d0`): reel entry `i` (below
    /// 4, unsigned) of [`REEL_ENTRIES`], 0 for the others.
    pub fn ms_get_reel_entry(mem: &impl Mem, i: u32) -> u32 {
        if i < 4 {
            REEL_ENTRIES.ptr(mem).addr + 4 * i
        } else {
            0
        }
    }

    /// `Vec4Scale4_007c11c0(out, v, sx, sy, sz, sw)` (`0x007c11c0`): each
    /// of `v`'s four floats times its own scale (`f32` arguments as bits),
    /// stored as `f32` in `out`. Returns `out` (the original's `EAX`).
    pub fn vec4_scale4(mem: &mut impl Mem, out: u32, v: u32, s: [u32; 4]) -> u32 {
        use ffx_bridge::x87::{load, mul, store};
        for (k, s) in (0..4).zip(s) {
            let x: u32 = mem.read(v + 4 * k);
            mem.write(out + 4 * k, store(mul(load(x), load(s))));
        }
        out
    }

    /// `Vu0MaddaScalar4_007c1000(out, v, _, _, s)` (`0x007c1000`): VU0
    /// `vmadda`-style, `out = v * s + ACC` per float (ACC the emulated
    /// accumulator `0x00c8f790`), each rounded to `f32` at its store; the
    /// third and fourth arguments are unused. Returns `out`.
    pub fn vu0_madda_scalar4(mem: &mut impl Mem, out: u32, v: u32, s: u32) -> u32 {
        use ffx_bridge::x87::{add, load, mul, store};
        let acc = crate::vector::bridge::VF_ACC.ptr(mem).addr;
        for k in 0..4 {
            let x: u32 = mem.read(v + 4 * k);
            let a: u32 = mem.read(acc + 4 * k);
            mem.write(out + 4 * k, store(add(mul(load(x), load(s)), load(a))));
        }
        out
    }

    /// `MsEffectGetCameraBuf_007b7ed0(i)` (`0x007b7ed0`): word `i` (0-15)
    /// of [`EFFECT_CAMERA_WORDS`], 0 for the others.
    pub fn ms_effect_get_camera_buf(mem: &impl Mem, i: u32) -> u32 {
        if i <= 15 {
            mem.read(EFFECT_CAMERA_WORDS.ptr(mem).addr + 4 * i)
        } else {
            0
        }
    }

    /// `MsEffectSetCameraBuf_007b7f30(i, v)` (`0x007b7f30`): word `i`
    /// (0-15) of [`EFFECT_CAMERA_WORDS`] = `v`; the others ignored.
    pub fn ms_effect_set_camera_buf(mem: &mut impl Mem, i: u32, v: u32) {
        if i <= 15 {
            let at = EFFECT_CAMERA_WORDS.ptr(mem).addr + 4 * i;
            mem.write(at, v);
        }
    }

    /// `MsEffectGetCameraBufFloat_007b7ef0(i)` (`0x007b7ef0`): float `i`
    /// (0-15) of [`EFFECT_CAMERA_FLOATS`] (`fld`/`fstp` through a local),
    /// 0.0 for the others; the `f32` the original leaves in ST0, as bits.
    pub fn ms_effect_get_camera_buf_float(mem: &impl Mem, i: u32) -> u32 {
        if i <= 15 {
            x87_copy(mem.read(EFFECT_CAMERA_FLOATS.ptr(mem).addr + 4 * i))
        } else {
            0
        }
    }

    /// `MsMenuReadTextDone_0078fed0(request)` (`0x0078fed0`): the menu text
    /// `request + 0xc` (at most 0x10, unsigned) no longer busy: its first
    /// byte in `g_BtlMenuText` cleared.
    pub fn ms_menu_read_text_done(mem: &mut impl Mem, request: u32) {
        let i: u32 = mem.read(request.wrapping_add(0xc));
        if i <= 0x10 {
            let at = crate::flow::bridge::MENU_TEXT.ptr(mem).addr + 12 * i;
            mem.write(at, 0u8);
        }
    }

    /// `isNeedShowJapanLogo()` (`0x00787390`): 1 when the language
    /// (`TOGetFFXLang`) is 0 (Japanese), else 0.
    #[must_use]
    pub fn is_need_show_japan_logo(lang: u32) -> u32 {
        u32::from(lang == 0)
    }

    /// `isNeedRenamePlayer(lang_was)` (`0x00787370`): 1 when the signed
    /// low byte of `lang_was` is not the language (`TOGetFFXLang`), else 0.
    #[must_use]
    pub fn is_need_rename_player(lang_was: u32, lang: u32) -> u32 {
        let was = i32::from(lang_was.to_le_bytes()[0].cast_signed());
        u32::from(was != lang.cast_signed())
    }

    /// `NameHash_007cf000(s)` (`0x007cf000`): a case-blind hash of the C
    /// string `s`: each byte (`A`-`Z` lowered), sign-extended and `XOR`ed with
    /// 0x6f times its index, summed (wrapping).
    pub fn name_hash(mem: &impl Mem, s: u32) -> u32 {
        let mut sum = 0u32;
        let mut salt = 0u32;
        let mut p = s;
        loop {
            let mut c: u8 = mem.read(p);
            if c == 0 {
                return sum;
            }
            if c.wrapping_sub(b'A') <= 25 {
                c = c.wrapping_add(0x20);
            }
            sum = sum.wrapping_add(i32::from(c.cast_signed()).cast_unsigned() ^ salt);
            salt = salt.wrapping_add(0x6f);
            p = p.wrapping_add(1);
        }
    }

    /// `MsDivSqrt_007c13c0(out, a, b)` (`0x007c13c0`): `out` = `a` over the
    /// square root of `|b|` (`f32`s; `_CIsqrt`, rounded to `f32`, the
    /// division in `f64` of `a`), or 0.0 when `|b|` is 0.0.
    pub fn ms_div_sqrt(mem: &mut impl Mem, out: u32, a: u32, b: u32) {
        use ffx_bridge::x87::{div, load, sqrt, store};
        let t = store(load(b).abs());
        // `fucom` with 0.0: a NaN takes the root
        #[allow(clippy::float_cmp)]
        let v = if load(t) == 0.0 {
            0
        } else {
            let root = store(sqrt(load(t)));
            store(div(load(a), load(root)))
        };
        mem.write(out, v);
    }
}
