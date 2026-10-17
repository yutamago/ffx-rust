//! The battle's requests to the event scripts (`docs/rewrite-plan.md`,
//! M2): `MsAtelRequestExe` runs the first queued request
//! (`MsAtelRequestAdd`) whose script handler exists, as a signal to the
//! script work behind it, and `MsSetMotion` queues an actor's motion
//! requests and runs them.
//!
//! A handler lives in one of three places: the battle's two script records
//! (kind 2, 0x20 bytes from `0x0112a984`: `+0` the first record's work
//! count, `+0x10` the script file), an actor's monster file (kind 3,
//! `+0xf7c`, its work indices after the actors' before it, `+0xdf6`) or the
//! script camera's (kind 4, `MsAtelCamRegist`). The script calls go
//! through [`Script`]; the scripts are their module's.

/// The game's memory, through [`ffx_bridge::Mem`].
#[allow(clippy::many_single_char_names)] // indices, handlers, vectors
pub mod bridge {
    use ffx_bridge::frame::Frame;
    use ffx_bridge::{Global, IMAGE_BASE, Mem};

    use crate::actor::bridge::{ms_btl_chr_num_check, ms_get_chr};
    use crate::exec::bridge::Call;
    use crate::minor::bridge::{ATEL_CAM, ATEL_CAM_ON, mon_find_handler};
    use crate::state::bridge::{
        ATEL_COUNT, ATEL_REQUEST, ms_atel_request_add, ms_atel_request_init,
    };

    /// The script calls `MsAtelRequestExe` and its helpers make.
    pub struct Script<'a, M> {
        /// The script functions taking no pointers (by their run-time
        /// address).
        pub call: Call<'a, M>,
        /// `AtelRegistSignal2` with the signal block the original builds
        /// on its stack (8 words).
        pub signal: &'a mut dyn FnMut(&mut M, [u32; 8]) -> u32,
        /// `op_et_eff_off_signal(id)` (vfx): an effect ends.
        pub off: &'a mut dyn FnMut(&mut M, u32),
    }

    /// `AtelChangeCurrentCtrl`.
    pub const ATEL_CHANGE_CURRENT_CTRL: u32 = 0x0086_2d30;
    /// `AtelSetCurrentCtrl`.
    pub const ATEL_SET_CURRENT_CTRL: u32 = 0x0086_f730;
    /// `AtelGetActorExecspaceID`.
    pub const ATEL_GET_ACTOR_EXECSPACE_ID: u32 = 0x0086_a420;
    /// `AtelGetActorId`.
    pub const ATEL_GET_ACTOR_ID: u32 = 0x0086_a5d0;
    /// `AtelGetBattleIdAdrs`.
    pub const ATEL_GET_BATTLE_ID_ADRS: u32 = 0x0086_a970;
    /// `AtelSkipReqLevel2`.
    pub const ATEL_SKIP_REQ_LEVEL2: u32 = 0x0087_1730;
    /// `AtelExecReturn2`.
    pub const ATEL_EXEC_RETURN2: u32 = 0x0086_9350;
    /// `AtelDecodeSignal`.
    pub const ATEL_DECODE_SIGNAL: u32 = 0x0086_53f0;
    /// `AtelDecodeCommand`.
    pub const ATEL_DECODE_COMMAND: u32 = 0x0086_41c0;
    /// `AtelGetRequestLevel`.
    pub const ATEL_GET_REQUEST_LEVEL: u32 = 0x0086_c3f0;
    /// `AtelGetBasicWork`.
    pub const ATEL_GET_BASIC_WORK: u32 = 0x0086_a830;
    /// `AtelGetCameraWorkAdrs`.
    pub const ATEL_GET_CAMERA_WORK_ADRS: u32 = 0x0086_ae50;
    /// `Sg_Blur` (main).
    pub const SG_BLUR: u32 = 0x0082_bee0;

    /// The battle's two script records (0x20 bytes each).
    pub const SCRIPT_RECORDS: Global<u8> = Global::new(0x0112_a984);
    /// Set while the actors' scripts are loaded (kind 3 handlers).
    pub const CHR_SCRIPTS: Global<u8> = Global::new(0x0112_c9db);
    /// Where `Mon_FindHandler`'s entry index lives here.
    pub const HANDLER_FRAME: u32 = 0xff40_0000;

    fn byte(mem: &impl Mem, p: u32, o: u32) -> u8 {
        mem.read(p.wrapping_add(o))
    }

    fn code(mem: &impl Mem, sym: u32) -> u32 {
        sym.wrapping_sub(IMAGE_BASE).wrapping_add(mem.image_base())
    }

    fn call<M: Mem>(mem: &mut M, s: &mut Script<'_, M>, sym: u32, args: &[u32]) -> u32 {
        let f = code(mem, sym);
        (s.call)(mem, f, args)
    }

    /// `Mon_FindHandler(file, i, &k)` with `k` a local: the handler and
    /// the new `k`.
    fn find(mem: &mut impl Mem, file: u32, i: i32, k: i32) -> (u32, i32) {
        let mut f = Frame::new(mem, HANDLER_FRAME, 4);
        let at = f.at(0);
        f.write(at, k);
        let h = mon_find_handler(&mut f, file, i, at);
        (h, f.read(at))
    }

    /// A script record's file (`+0x10`).
    fn record_file(mem: &impl Mem, i: u32) -> u32 {
        let p = SCRIPT_RECORDS.ptr(mem).addr;
        mem.read(p.wrapping_add(i << 5).wrapping_add(0x10))
    }

    /// [`ms_atel_get_handler`] without the pointer: the handler (null for
    /// none) and the work index its byte counts from.
    pub fn atel_get_handler(mem: &mut impl Mem, kind: u32, i: u32, n: i32) -> (u32, Option<u32>) {
        match kind {
            4 => {
                if i != 0 || ATEL_CAM_ON.get(mem) == 0 {
                    return (0, None);
                }
                let file = ATEL_CAM.get(mem)[1];
                (find(mem, file, n, 0).0, Some(0))
            }
            3 => {
                if CHR_SCRIPTS.get(mem) == 0 {
                    return (0, None);
                }
                let a = ms_get_chr(mem, i);
                if ms_btl_chr_num_check(i) == 0 || mem.read::<u32>(a.wrapping_add(0x48)) == 0 {
                    return (0, None);
                }
                let file = mem.read(a.wrapping_add(0xf7c));
                let h = find(mem, file, n, 0).0;
                let mut base = 0u32;
                for j in 0..i.cast_signed().max(0).cast_unsigned() {
                    let b = ms_get_chr(mem, j);
                    base = base.wrapping_add(
                        i32::from(mem.read::<i8>(b.wrapping_add(0xdf6))).cast_unsigned(),
                    );
                }
                (h, Some(base))
            }
            2 => {
                if i > 1 {
                    return (0, None);
                }
                let file = record_file(mem, i);
                let h = find(mem, file, n, 0).0;
                let base = if i == 0 {
                    0
                } else {
                    let p = SCRIPT_RECORDS.ptr(mem).addr;
                    i32::from(mem.read::<i8>(p)).cast_unsigned()
                };
                (h, Some(base))
            }
            _ => (0, Some(0)),
        }
    }

    /// `0x00798590(kind, i, n, &base)` (`MsAtelGetHandler`, name ours):
    /// handler `n` of script owner `i` of `kind` (2 the battle's records
    /// 0-1, 3 actor `i` while [`CHR_SCRIPTS`], 4 the script camera,
    /// `i` 0), `base` the work index its byte counts from (written unless
    /// the owner has no scripts; 0 for other kinds). Null for none.
    pub fn ms_atel_get_handler(mem: &mut impl Mem, kind: u32, i: u32, n: i32, out: u32) -> u32 {
        let (h, base) = atel_get_handler(mem, kind, i, n);
        if let Some(base) = base
            && out != 0
        {
            mem.write(out, base);
        }
        h
    }

    /// `MsAtelRequestExe(kind, i, n, flags, actor, &same)` (`0x00797d50`):
    /// with handler `n` of `kind`/`i` there (the script work `w`, its byte
    /// plus base), the first queued request whose own handler is there
    /// (2 a script record, 3 an actor's file, 4 the camera's; its entry
    /// `k` not negative; a request of another kind leaves the last one's
    /// handler) is signalled to `w` (`AtelGetBattleIdAdrs(w)` `+1`
    /// cleared, `AtelSkipReqLevel2(w, 0)`, `AtelExecReturn2(w, 0, 0)`,
    /// `AtelRegistSignal2` with `[0xffff, w, 0, 7, k, kind, owner,
    /// handler byte]`); then with `flags` 4 the blur and camera priorities
    /// ([`ms_atel_blur`]), and with `flags & 3` 1 (or 2 and `actor` not
    /// `w`'s: its execution space `kind` and ID `w`) `w` decodes at once
    /// (`AtelDecodeSignal(w, -1)`, `AtelDecodeCommand(w, 0)`). `same`
    /// (unless null) says whether `actor` was `w`. The queue is emptied
    /// and the current script control restored. The request's kind less 2
    /// (0-2), -1 when nothing ran.
    #[allow(clippy::too_many_arguments)]
    pub fn ms_atel_request_exe<M: Mem>(
        mem: &mut M,
        s: &mut Script<'_, M>,
        kind: u32,
        i: u32,
        n: i32,
        flags: u32,
        actor: u32,
        out: u32,
    ) -> i32 {
        let cur = call(mem, s, ATEL_CHANGE_CURRENT_CTRL, &[kind]);
        let (h, base) = atel_get_handler(mem, kind, i, n);
        let (mut ret, mut same) = (-1i32, 0u32);
        if h != 0 {
            let base = base.unwrap_or(0);
            let count = ATEL_COUNT.get(mem);
            let (mut hit, mut k, mut owner, mut side, mut at) = (0u32, 0i32, 0u32, 0i32, 0);
            for e in 0..count.max(0).cast_unsigned() {
                let r: [u16; 4] = ATEL_REQUEST.ptr(mem).at(e).get(mem);
                at = e;
                k = i32::from(r[3].cast_signed());
                let who = i32::from(r[1].cast_signed());
                let which = i32::from(r[2].cast_signed());
                match r[0].cast_signed() {
                    2 => {
                        side = 0;
                        owner = who.cast_unsigned();
                        let file = record_file(mem, owner);
                        (hit, k) = find(mem, file, which, k);
                    }
                    3 => {
                        side = 1;
                        let a = ms_get_chr(mem, who.cast_unsigned());
                        owner = i32::from(mem.read::<i8>(a.wrapping_add(0xdf5))).cast_unsigned();
                        let file = mem.read(a.wrapping_add(0xf7c));
                        (hit, k) = find(mem, file, which, k);
                    }
                    4 => {
                        side = 2;
                        owner = 0;
                        let file = ATEL_CAM.get(mem)[1];
                        (hit, k) = find(mem, file, which, k);
                    }
                    _ => {}
                }
                if hit != 0 && k >= 0 {
                    break;
                }
            }
            if count > 0 && hit != 0 && k >= 0 {
                let w = u32::from(byte(mem, h, 0)).wrapping_add(base);
                let mut mode = flags & 3;
                let mut mine = false;
                if actor != 0 {
                    let space = call(mem, s, ATEL_GET_ACTOR_EXECSPACE_ID, &[actor]);
                    let id = call(mem, s, ATEL_GET_ACTOR_ID, &[actor]);
                    if kind == space && w == id {
                        mine = true;
                        same = 1;
                    }
                }
                if mode == 2 {
                    mode = u32::from(!mine);
                }
                let p = call(mem, s, ATEL_GET_BATTLE_ID_ADRS, &[w]);
                mem.write(p.wrapping_add(1), 0u8);
                call(mem, s, ATEL_SKIP_REQ_LEVEL2, &[w, 0]);
                call(mem, s, ATEL_EXEC_RETURN2, &[w, 0, 0]);
                let r: [u16; 4] = ATEL_REQUEST.ptr(mem).at(at).get(mem);
                let block = [
                    0xffff,
                    w,
                    0,
                    7,
                    k.cast_unsigned(),
                    i32::from(r[0].cast_signed()).cast_unsigned(),
                    owner,
                    u32::from(byte(mem, hit, 0)),
                ];
                (s.signal)(mem, block);
                ret = side;
                if flags & 4 != 0 {
                    ms_atel_blur(mem, s);
                }
                if mode == 1 {
                    call(mem, s, ATEL_DECODE_SIGNAL, &[w, u32::MAX]);
                    call(mem, s, ATEL_DECODE_COMMAND, &[w, 0]);
                }
            }
        }
        if out != 0 {
            mem.write(out, same);
        }
        ms_atel_request_init(mem);
        call(mem, s, ATEL_SET_CURRENT_CTRL, &[cur]);
        ret
    }

    /// `0x00798760(kind, i, n, bonus)` (`MsAtelCameraPriority`, name ours):
    /// the camera of handler `n`'s script work (`AtelGetBasicWork`,
    /// `AtelGetCameraWorkAdrs`), if any, gets priority bonus `bonus`
    /// (`MsCameraPriority`).
    pub fn ms_atel_camera_priority<M: Mem>(
        mem: &mut M,
        s: &mut Script<'_, M>,
        kind: u32,
        i: u32,
        n: i32,
        bonus: u8,
    ) {
        let (h, base) = atel_get_handler(mem, kind, i, n);
        if h == 0 {
            return;
        }
        let w = u32::from(byte(mem, h, 0)).wrapping_add(base.unwrap_or(0));
        let work = call(mem, s, ATEL_GET_BASIC_WORK, &[w]);
        let p = call(mem, s, ATEL_GET_CAMERA_WORK_ADRS, &[work]);
        let cam: u32 = mem.read(p);
        if cam != 0 {
            crate::camera::bridge::ms_camera_priority(mem, cam, bonus);
        }
    }

    /// `0x007986c0()` (`MsAtelBlur`, name ours): no blur (`Sg_Blur(0)`),
    /// and with script control 2 the cameras of the battle's handlers
    /// 0x3e (record 1), 0, 1 and 2 (record 0) get no bonus.
    pub fn ms_atel_blur<M: Mem>(mem: &mut M, s: &mut Script<'_, M>) {
        call(mem, s, SG_BLUR, &[0]);
        let cur = call(mem, s, ATEL_CHANGE_CURRENT_CTRL, &[2]);
        for (i, n) in [(1, 0x3e), (0, 0), (0, 1), (0, 2)] {
            ms_atel_camera_priority(mem, s, 2, i, n, 0);
        }
        call(mem, s, ATEL_SET_CURRENT_CTRL, &[cur]);
    }

    /// `MsSetMotion(i, motion, wait, x, flags, actor, &same)`
    /// (`0x007ab360`): for a motion (not negative), actor `i`'s motion
    /// state reset (`+0x3fb` from `+0x3fa`; `+0x50d`, `+0x448`, `+0x429`
    /// cleared; `+0x39c` -1 with `+0x414`; with `wait`, `+0xdf2` `x` and
    /// `+0x432` set), its requests queued (a monster: its own handler 0x40,
    /// the battle's 4 or 3 (`+0x39` 1 or not) of record 1, its handler 4,
    /// the battle's of record 0; a party member: handler `i + 5` and 3 of
    /// records 1 and 0, all with `k` the motion) and run as
    /// [`ms_atel_request_exe`] for the battle's handler `i + 5`. 1 when
    /// one ran (`+0x432` cleared without `wait`); else 0 with the two
    /// fields back (and with `wait`, `+0xdf2` cleared and the motion
    /// ended, `MsTerminateMotion`).
    #[allow(clippy::too_many_arguments)]
    pub fn ms_set_motion<M: Mem>(
        mem: &mut M,
        s: &mut Script<'_, M>,
        i: u32,
        motion: i32,
        wait: u32,
        x: u8,
        flags: u32,
        actor: u32,
        out: u32,
    ) -> u32 {
        let a = ms_get_chr(mem, i);
        if motion < 0 {
            return 0;
        }
        let saved: [u8; 2] = [
            mem.read(a.wrapping_add(0x50d)),
            mem.read(a.wrapping_add(0x448)),
        ];
        let v = byte(mem, a, 0x3fa);
        mem.write(a.wrapping_add(0x3fb), v);
        mem.write(a.wrapping_add(0x50d), 0u8);
        mem.write(a.wrapping_add(0x448), 0u8);
        mem.write(a.wrapping_add(0x429), 0u8);
        if byte(mem, a, 0x414) != 0 {
            mem.write(a.wrapping_add(0x39c), u32::MAX);
        }
        if wait != 0 {
            mem.write(a.wrapping_add(0xdf2), x);
            mem.write(a.wrapping_add(0x432), 1u8);
        }
        let m = motion.to_le_bytes();
        let m = u16::from_le_bytes([m[0], m[1]]);
        let ii = i.to_le_bytes();
        let ii = u16::from_le_bytes([ii[0], ii[1]]);
        let n = i.wrapping_add(5);
        let nn = ii.wrapping_add(5);
        ms_atel_request_init(mem);
        if crate::actor::bridge::ms_get_ram_chr_monster(i) != 0 {
            let h = if byte(mem, a, 0x39) == 1 { 4 } else { 3 };
            ms_atel_request_add(mem, [3, ii, 0x40, m]);
            ms_atel_request_add(mem, [2, 1, h, m]);
            ms_atel_request_add(mem, [3, ii, 4, m]);
            ms_atel_request_add(mem, [2, 0, h, m]);
        } else {
            ms_atel_request_add(mem, [2, 1, nn, m]);
            ms_atel_request_add(mem, [2, 1, 3, m]);
            ms_atel_request_add(mem, [2, 0, nn, m]);
            ms_atel_request_add(mem, [2, 0, 3, m]);
        }
        let r = ms_atel_request_exe(mem, s, 2, 0, n.cast_signed(), flags, actor, out);
        if r < 0 {
            mem.write(a.wrapping_add(0x448), saved[1]);
            mem.write(a.wrapping_add(0x50d), saved[0]);
            if wait != 0 {
                mem.write(a.wrapping_add(0xdf2), 0u8);
                crate::actor::bridge::ms_terminate_motion(mem, a);
            }
            return 0;
        }
        if wait == 0 {
            mem.write(a.wrapping_add(0x432), 0u8);
        }
        1
    }

    /// `Ch_NeckSetEnable` and `Ch_NeckSetRot` (chara).
    pub const CH_NECK_SET_ENABLE: u32 = 0x0083_4f60;
    pub const CH_NECK_SET_ROT: u32 = 0x0083_4ff0;

    /// `msResetHeadTarget(actor)` (`0x007b1f10`): with a head target
    /// (`+0x438` not 0xff), none (0xff) and 1; a petrified actor (`+0xdce`)
    /// turns its neck straight (`Ch_NeckSetRot(chr, 0.0, 0.0)`,
    /// `Ch_NeckSetEnable(chr, 0)` on its character `+0`, if any; `+0x439`
    /// 0), any other leaves that for later (`+0x439` 1). 0 without one.
    pub fn ms_reset_head_target<M: Mem>(mem: &mut M, call: Call<'_, M>, actor: u32) -> u32 {
        if byte(mem, actor, 0x438) == 0xff {
            return 0;
        }
        mem.write(actor.wrapping_add(0x438), 0xffu8);
        if byte(mem, actor, 0xdce) != 0 {
            mem.write(actor.wrapping_add(0x439), 0u8);
            let chr: u32 = mem.read(actor);
            if chr != 0 {
                let f = code(mem, CH_NECK_SET_ROT);
                call(mem, f, &[chr, 0, 0]);
                let f = code(mem, CH_NECK_SET_ENABLE);
                call(mem, f, &[chr, 0]);
            }
        } else {
            mem.write(actor.wrapping_add(0x439), 1u8);
        }
        1
    }

    /// `0x00791fa0(chr, cue, k, &far)` (`MsSetTargetsMotion`, name ours):
    /// for entry `k` of `chr`'s cue, its command (`MsGetCommand`; none:
    /// the targets cleared, -1) and its targets (`+0x10`): those out of
    /// its reach (`+0x4ff` above `MsGetCommandFar`, not `chr` itself) are
    /// counted in `far`; the rest (KO'd ones only when the command's
    /// `+0x1a` bit 6 allows) stay targets, remember `chr` (`+0xded`), run
    /// motion 0x1c (`MsSetMotion(+0x422, 0x1c, 0, 0, 1)`) and, when they
    /// are others able to act (`+0xdcc` clear, `MsAutoCommandStatusCheck`),
    /// look at `chr` (`MsSetHeadTarget`) or ahead (`msResetHeadTarget`,
    /// command `+0x1c` bit 27). The targets left go to the entry and
    /// `g_BtlChrSet`. -1 when some stayed and the command hits (`+0x2b`),
    /// has `+0x1c` bit 9 or `+0x21` bit 0, or without any of those; else 0.
    pub fn ms_set_targets_motion<M: Mem>(
        mem: &mut M,
        s: &mut Script<'_, M>,
        chr: u32,
        cue: u32,
        k: u32,
        out: u32,
    ) -> i32 {
        let (r, far) = set_targets_motion(mem, s, chr, cue, k);
        if out != 0 {
            mem.write(out, far);
        }
        r
    }

    /// [`ms_set_targets_motion`] returning the count of targets out of
    /// reach instead of storing it.
    pub fn set_targets_motion<M: Mem>(
        mem: &mut M,
        s: &mut Script<'_, M>,
        chr: u32,
        cue: u32,
        k: u32,
    ) -> (i32, u32) {
        let e = cue.wrapping_add(k << 4);
        let targets: u32 = mem.read(e.wrapping_add(0x10));
        let (c, _) = crate::command::bridge::get_command(mem, u32::MAX, e.wrapping_add(8), 0xff);
        let (mut kept, mut far) = (0u32, 0u32);
        let r = if c == 0 {
            -1
        } else {
            let look_ahead = mem.read::<u32>(c.wrapping_add(0x1c)) >> 27 & 1 != 0;
            let ko_ok = mem.read::<u16>(c.wrapping_add(0x1a)) >> 6 & 1 != 0;
            let reach =
                crate::actor::bridge::ms_get_command_far(mem, chr.cast_signed(), c).cast_signed();
            for j in 0..0x1f {
                let a = ms_get_chr(mem, j);
                if targets >> j & 1 == 0 {
                    continue;
                }
                if i32::from(mem.read::<i8>(a.wrapping_add(0x4ff))) > reach && chr != j {
                    far += 1;
                    continue;
                }
                if !ko_ok && byte(mem, a, 0xdcc) != 0 {
                    continue;
                }
                kept |= 1 << j;
                mem.write(a.wrapping_add(0xded), chr.to_le_bytes()[0]);
                let who = i32::from(mem.read::<i8>(a.wrapping_add(0x422))).cast_unsigned();
                ms_set_motion(mem, s, who, 0x1c, 0, 0, 1, 0, 0);
                if chr != j
                    && byte(mem, a, 0xdcc) == 0
                    && crate::actor::bridge::ms_auto_command_status_check(mem, a, 1, 1, 1) != 0
                {
                    if look_ahead {
                        ms_reset_head_target(mem, &mut *s.call, a);
                    } else {
                        crate::flow::bridge::ms_set_head_target(mem, j, 1u32.wrapping_shl(chr));
                    }
                }
            }
            let hits = byte(mem, c, 0x2b) != 0
                || mem.read::<u32>(c.wrapping_add(0x1c)) & 0x200 != 0
                || byte(mem, c, 0x21) & 1 != 0;
            if hits { -i32::from(kept != 0) } else { -1 }
        };
        mem.write(e.wrapping_add(0x10), kept);
        crate::actor::bridge::CHR_SET.set(mem, kept);
        (r, far)
    }

    /// `MsPaymentProcessStart(actor)` (`0x007afa80`): Yojimbo's payment
    /// motion: `MsSetMotion(+0x6d1, 0x4e, 1, 0, 1)`.
    pub fn ms_payment_process_start<M: Mem>(mem: &mut M, s: &mut Script<'_, M>, actor: u32) {
        let i = u32::from(byte(mem, actor, 0x6d1));
        ms_set_motion(mem, s, i, 0x4e, 1, 0, 1, 0, 0);
    }

    /// `MsSetVoiceCheck(i, voice, flags, actor, &same)` (`0x007a07b0`):
    /// for a voice (not negative), the battle's handler `i + 0x6d` with
    /// `k` the voice from record 1 (and record 0 while voice events are on,
    /// `MsGetVoiceEventFlag`) run as [`ms_atel_request_exe`]: 1 when one
    /// ran, else 0.
    pub fn ms_set_voice_check<M: Mem>(
        mem: &mut M,
        s: &mut Script<'_, M>,
        i: u32,
        voice: i32,
        flags: u32,
        actor: u32,
        out: u32,
    ) -> u32 {
        if voice < 0 {
            return 0;
        }
        let v = voice.to_le_bytes();
        let v = u16::from_le_bytes([v[0], v[1]]);
        let n = i.wrapping_add(0x6d);
        let nn = n.to_le_bytes();
        let nn = u16::from_le_bytes([nn[0], nn[1]]);
        ms_atel_request_init(mem);
        ms_atel_request_add(mem, [2, 1, nn, v]);
        if crate::state::bridge::ms_get_voice_event_flag(mem) != 0 {
            ms_atel_request_add(mem, [2, 0, nn, v]);
        }
        u32::from(ms_atel_request_exe(mem, s, 2, 0, n.cast_signed(), flags, actor, out) >= 0)
    }

    /// `0x007979d0(kind, i, n)` (`MsAtelParseSysTag`, Fahrenheit-layout name): with
    /// handler `n` of `kind`/`i` there, its script work runs
    /// (`AtelDecodeCommand(w, 0)`, `AtelDecodeSignal(w, -1)`) until its
    /// request level (`AtelGetRequestLevel`) is below 7: 0; -1 without
    /// it. Under script control `kind`.
    pub fn ms_atel_parse_sys_tag<M: Mem>(
        mem: &mut M,
        s: &mut Script<'_, M>,
        kind: u32,
        i: u32,
        n: i32,
    ) -> i32 {
        let cur = call(mem, s, ATEL_CHANGE_CURRENT_CTRL, &[kind]);
        let (h, base) = atel_get_handler(mem, kind, i, n);
        let r = if h == 0 {
            -1
        } else {
            let w = u32::from(byte(mem, h, 0)).wrapping_add(base.unwrap_or(0));
            loop {
                call(mem, s, ATEL_DECODE_COMMAND, &[w, 0]);
                call(mem, s, ATEL_DECODE_SIGNAL, &[w, u32::MAX]);
                if call(mem, s, ATEL_GET_REQUEST_LEVEL, &[w]).cast_signed() < 7 {
                    break;
                }
            }
            0
        };
        call(mem, s, ATEL_SET_CURRENT_CTRL, &[cur]);
        r
    }

    /// The frame `MsDefenseStartProcess` keeps its three vectors in.
    pub const DEFENSE_FRAME: u32 = 0xff30_0000;

    /// `MsDefenseStartProcess(regs)` (`0x007af920`): every able actor
    /// (`+0xdc8`) defending another (`+0x6d4`, from `+0x6d3`'s attacker)
    /// steps between them (`+0x411` set): its position (`+0x3b0`) the
    /// defended one's battle position plus, towards the attacker's, both
    /// widths and 5.0 more (`MsVectorNormal2`, whose scale's other words are
    /// its caller's `edi`, `esi`, `ebx`: `regs`, or as they were), facing
    /// the attacker (`MsVectorRect2Polar` into `+0x520`); its direction
    /// mode reset (`MsSetDirThetaMode`) and motion 0x3c (0x3d with `+0x616`
    /// bit 13) set, and the attacker's motion 0x34 when it can take one
    /// (`MsCheckSetMotion`).
    pub fn ms_defense_start_process<M: Mem>(
        mem: &mut M,
        s: &mut Script<'_, M>,
        trig: &dyn Fn(crate::pos::bridge::Trig, u32) -> u32,
        regs: Option<crate::target::bridge::Regs>,
    ) {
        use crate::vector::bridge as v;
        use ffx_bridge::x87::{add, load, store};
        for i in 0..0x1f {
            let a = ms_get_chr(mem, i);
            if byte(mem, a, 0xdc8) == 0 {
                continue;
            }
            let from = u32::from(byte(mem, a, 0x6d3));
            let to = u32::from(byte(mem, a, 0x6d4));
            if to == 0xff {
                continue;
            }
            mem.write(a.wrapping_add(0x411), 1u8);
            {
                let mut f = Frame::new(mem, DEFENSE_FRAME, 0x30);
                let (p, q, d) = (f.at(0), f.at(0x10), f.at(0x20));
                crate::pos::bridge::ms_get_chr_btl_pos(&mut f, trig, from, p);
                crate::pos::bridge::ms_get_chr_btl_pos(&mut f, trig, to, q);
                let wt = f64::from(crate::actor::bridge::ms_get_chr_width(&f, to));
                let wi = crate::actor::bridge::ms_get_chr_width(&f, i).to_bits();
                let more = f64::from_bits(Global::<u64>::new(0x00b4_1d20).get(&f));
                let scale = store(add(add(load(wi), wt), more));
                let rest = match regs {
                    Some(r) => [r.edi, r.esi, r.ebx],
                    None => {
                        let at = v::VF_M.ptr(&f).addr + 0x24;
                        f.read(at)
                    }
                };
                v::ms_vector_normal2(&mut f, d, p, q, [scale, rest[0], rest[1], rest[2]]);
                let pos = a.wrapping_add(0x3b0);
                v::ms_add_point(&mut f, pos, q, d);
                v::ms_vector_rect2_polar(&mut f, trig, p, pos, a.wrapping_add(0x520), 0, 0);
            }
            crate::minor::bridge::ms_set_dir_theta_mode(mem, a);
            let bit = u32::from(mem.read::<u16>(a.wrapping_add(0x616)) >> 13 & 1);
            ms_set_motion(mem, s, i, (0x3c | bit).cast_signed(), 0, 0, 1, 0, 0);
            let b = ms_get_chr(mem, from);
            if crate::actor::bridge::ms_check_set_motion(mem, b, 0) != 0 {
                ms_set_motion(mem, s, from, 0x34, 0, 0, 1, 0, 0);
            }
        }
    }

    /// Set while the battle camera is fixed (no script cameras).
    pub const CAMERA_LOCK: Global<u8> = Global::new(0x0112_a8fe);
    /// The battle's camera state (2: no script cameras unless forced).
    pub const CAMERA_STATE: Global<u8> = Global::new(0x0112_a8e4);

    /// `MsAtelRequestCamera(i, monster, a, b, show, flags, actor, &same)`
    /// (`0x00797bc0`): the camera script for handler `i`: -1 while the
    /// camera is locked ([`CAMERA_LOCK`]) or, without `actor`, in camera
    /// state 2 or with an effect camera; with `show` the actors hidden by
    /// the camera come back (`MsReturnCameraVisible`, its calls through
    /// `s.call`). Requests `a` and `b` (each unless 0xffff) of the battle's
    /// record 1, of the monster `monster` (when it is one) and of record 0
    /// are queued and run ([`ms_atel_request_exe`] for record 0's handler
    /// `i`, flags with 4).
    #[allow(clippy::too_many_arguments)]
    pub fn ms_atel_request_camera<M: Mem>(
        mem: &mut M,
        s: &mut Script<'_, M>,
        i: u32,
        monster: u32,
        a: u32,
        b: u32,
        show: u32,
        flags: u32,
        actor: u32,
        out: u32,
    ) -> i32 {
        if CAMERA_LOCK.get(mem) != 0 {
            return -1;
        }
        if actor == 0
            && (CAMERA_STATE.get(mem) == 2
                || crate::minor::bridge::EFFECT_CAMERA_MODE.get(mem) != 0)
        {
            return -1;
        }
        if show != 0 {
            crate::visible::bridge::ms_return_camera_visible(mem, &mut *s.call);
        }
        let h = |v: u32| u16::from_le_bytes([v.to_le_bytes()[0], v.to_le_bytes()[1]]);
        ms_atel_request_init(mem);
        let reqs: Vec<u16> = [a, b].into_iter().filter(|&r| r != 0xffff).map(h).collect();
        for &r in &reqs {
            ms_atel_request_add(mem, [2, 1, h(i), r]);
        }
        if monster != 0xff && crate::actor::bridge::ms_get_ram_chr_monster(monster) != 0 {
            for &r in &reqs {
                ms_atel_request_add(mem, [3, h(monster), h(i), r]);
            }
        }
        for &r in &reqs {
            ms_atel_request_add(mem, [2, 0, h(i), r]);
        }
        ms_atel_request_exe(mem, s, 2, 0, i.cast_signed(), flags | 4, actor, out)
    }

    /// `MsAtelRequestMagicCamera(i, monster, id, a, b, show, flags, actor,
    /// &same)` (`0x00798000`): as [`ms_atel_request_camera`], with the
    /// command `id`'s own camera request first: handler `i` plus 0x44
    /// (items, 0x2xxx), 0x41 (0x3xxx), 0x47 (0x4xxx) or 0x4a (0x6xxx), `k`
    /// the ID's low 12 bits; the script camera's (kind 4) requests `a` and
    /// `b` come before record 0's.
    #[allow(clippy::too_many_arguments)]
    pub fn ms_atel_request_magic_camera<M: Mem>(
        mem: &mut M,
        s: &mut Script<'_, M>,
        i: u32,
        monster: u32,
        id: u32,
        a: u32,
        b: u32,
        show: u32,
        flags: u32,
        actor: u32,
        out: u32,
    ) -> i32 {
        if CAMERA_LOCK.get(mem) != 0 {
            return -1;
        }
        if actor == 0
            && (CAMERA_STATE.get(mem) == 2
                || crate::minor::bridge::EFFECT_CAMERA_MODE.get(mem) != 0)
        {
            return -1;
        }
        if show != 0 {
            crate::visible::bridge::ms_return_camera_visible(mem, &mut *s.call);
        }
        let h = |v: u32| u16::from_le_bytes([v.to_le_bytes()[0], v.to_le_bytes()[1]]);
        let low = h(id & 0xfff);
        let own = match (id.cast_signed() >> 12).wrapping_sub(2) {
            0 => Some(0x44),
            1 => Some(0x41),
            2 => Some(0x47),
            4 => Some(0x4a),
            _ => None,
        };
        let own = own.map(|base: u32| h(base.wrapping_add(i)));
        let reqs: Vec<u16> = [a, b].into_iter().filter(|&r| r != 0xffff).map(h).collect();
        ms_atel_request_init(mem);
        if let Some(o) = own {
            ms_atel_request_add(mem, [2, 1, o, low]);
        }
        for &r in &reqs {
            ms_atel_request_add(mem, [2, 1, h(i), r]);
        }
        if monster != 0xff && crate::actor::bridge::ms_get_ram_chr_monster(monster) != 0 {
            if let Some(o) = own {
                ms_atel_request_add(mem, [3, h(monster), o, low]);
            }
            for &r in &reqs {
                ms_atel_request_add(mem, [3, h(monster), h(i), r]);
            }
        }
        for &r in &reqs {
            ms_atel_request_add(mem, [4, 0, h(i), r]);
        }
        if let Some(o) = own {
            ms_atel_request_add(mem, [2, 0, o, low]);
        }
        for &r in &reqs {
            ms_atel_request_add(mem, [2, 0, h(i), r]);
        }
        ms_atel_request_exe(mem, s, 2, 0, i.cast_signed(), flags | 4, actor, out)
    }

    /// `MsEscapeAll()` (`0x007ada60`): every battle party member
    /// (`0x0112c895`, 7 slots) able (`+0xdc8`, not KO'd or petrified)
    /// escapes: `+0x41c` cleared (for any present one), escaping counts for
    /// its overdrive (`MsLimitTypeEscapeCheck`), `+0x4fd` up by 3, motion
    /// 0x12 (`MsSetMotion(i, 0x12, 1, 1, 1)`) and its battle position kept
    /// (`MsPushBtlPos`).
    pub fn ms_escape_all<M: Mem>(mem: &mut M, s: &mut Script<'_, M>) {
        let party = crate::save::bridge::BATTLE_PARTY.ptr(mem).addr;
        for k in 0..7 {
            let c = u32::from(mem.read::<u8>(party + k));
            if c == 0xff {
                continue;
            }
            let a = ms_get_chr(mem, c);
            mem.write(a.wrapping_add(0x41c), 0u8);
            if byte(mem, a, 0xdc8) == 0 || byte(mem, a, 0xdcc) != 0 || byte(mem, a, 0xdce) != 0 {
                continue;
            }
            crate::exec::bridge::ms_limit_type_escape_check(mem, c, a);
            let v = byte(mem, a, 0x4fd).wrapping_add(3);
            mem.write(a.wrapping_add(0x4fd), v);
            ms_set_motion(mem, s, c, 0x12, 1, 1, 1, 0, 0);
            crate::actor::bridge::ms_push_btl_pos(mem, a);
        }
    }
}
