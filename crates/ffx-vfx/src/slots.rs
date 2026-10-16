//! Small functions of the spell overlays' import table (`g_MagicImports`,
//! `docs/formats/magic.md`) that the Ghidra dump used to hide: flag
//! setters, getters, and thin wrappers over the battle, sound, scene
//! (`TO*`) and map-effect (`rcef*`) modules' functions, which are called
//! through a callback.

#![allow(clippy::many_single_char_names)]

/// The functions over game memory.
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    /// Calls the game function at `sym` (rebased) through `call`.
    fn go<M: Mem>(
        mem: &mut M,
        sym: u32,
        args: &[u32],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = Global::<u32>::new(sym).ptr(mem).addr;
        call(mem, f, args)
    }

    /// The address of game data `sym`, rebased.
    fn addr(mem: &impl Mem, sym: u32) -> u32 {
        Global::<u32>::new(sym).ptr(mem).addr
    }

    /// `g_OpEtFlags` (`0x011a006e`): `op_et_flag1_on/off` (bit 0) and
    /// `op_et_flag2_on/off` (bit 1).
    pub const ET_FLAGS: Global<u8> = Global::new(0x011a_006e);

    /// `op_et_flag1_on`/`off` (`0x007fc990`/`0x007fc9a0`, `bit` 1) and
    /// `op_et_flag2_on`/`off` (`0x007fc900`/`0x007fc910`, `bit` 2).
    pub fn op_et_flag(mem: &mut impl Mem, bit: u8, on: bool) {
        let v = ET_FLAGS.get(mem);
        ET_FLAGS.set(mem, if on { v | bit } else { v & !bit });
    }

    /// `g_TOBtlLibraState` (`0x0133f6a8`, `i16`).
    pub const LIBRA_STATE: Global<u16> = Global::new(0x0133_f6a8);

    /// `TOBtlLibraSetState6()` (`0x00890ec0`): the state 6; returns 6.
    pub fn to_btl_libra_set_state6(mem: &mut impl Mem) -> u32 {
        LIBRA_STATE.set(mem, 6);
        6
    }

    /// `TOBtlLibraGetState()` (`0x00897770`): the state, sign-extended.
    pub fn to_btl_libra_get_state(mem: &impl Mem) -> u32 {
        i32::from(LIBRA_STATE.get(mem).cast_signed()).cast_unsigned()
    }

    /// A float global read through the FPU: `rcefGetFloat25d5ea4`
    /// (`0x009249a0`, `0x025d5ea4`) and `TOGetFloat133f790` (`0x00897780`,
    /// `0x0133f790`).
    pub fn get_float(mem: &impl Mem, sym: u32) -> f32 {
        crate::x87::load_f32(mem.read(addr(mem, sym)))
    }

    /// `MsGetBattle112be90()` (`0x007b7f20`): the word at `0x0112be90`.
    pub fn ms_get_battle_112be90(mem: &impl Mem) -> u32 {
        mem.read(addr(mem, 0x0112_be90))
    }

    /// The map effects' work (`0x025d5a20`).
    const RCEF_WORK: u32 = 0x025d_5a20;

    /// `rcefWorkCall_00925530()` (`0x0092e770`, `f` 0x00925530) and
    /// `rcefWorkCall_00924c70()` (`0x00925130`, `f` 0x00924c70):
    /// `f(&work)` through `call`; returns its result.
    pub fn rcef_work_call<M: Mem>(
        mem: &mut M,
        f: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let w = addr(mem, RCEF_WORK);
        go(mem, f, &[w], call)
    }

    /// `rcefEffectEndMotion()` (`0x0092df90`): `MsEffectEndMotion(the word
    /// at 0x025d5bb8, 0)` through `call`; returns its result.
    pub fn rcef_effect_end_motion<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let e: u32 = mem.read(addr(mem, 0x025d_5bb8));
        go(mem, 0x0078_7950, &[e, 0], call)
    }

    /// `rcefChrAlloc()` (`0x00930800`): a 0x100-byte block from the current
    /// environment's heaps (`pppMemAlloc`), `rcefChrInit(block)` through
    /// `call`; returns the block.
    pub fn rcef_chr_alloc<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let env: u32 = mem.read(crate::heap::bridge::ENV_SLOT.get(mem));
        let b = crate::heap::bridge::ppp_mem_alloc(mem, env, env.wrapping_add(0x10), 0x100);
        go(mem, 0x0093_0770, &[b], call);
        b
    }

    /// `rcefReverseList()` (`0x0092e7a0`): the words from `0x025d5bc0`
    /// (the count at `0x025d5bbc`) reversed in place.
    pub fn rcef_reverse_list(mem: &mut impl Mem) {
        let count: i32 = mem.read(addr(mem, 0x025d_5bbc));
        let first = addr(mem, 0x025d_5bc0);
        let mut lo = first;
        let mut hi = addr(mem, 0x025d_5bbc).wrapping_add(count.cast_unsigned().wrapping_mul(4));
        // the original swaps count / 2 pairs (signed, toward zero)
        for _ in 0..(count / 2).max(0) {
            let (a, b): (u32, u32) = (mem.read(hi), mem.read(lo));
            mem.write(lo, a);
            mem.write(hi, b);
            lo = lo.wrapping_add(4);
            hi = hi.wrapping_sub(4);
        }
    }

    /// The battle voice wait and ID ([`crate::op::bridge::SETTERS`]).
    const VOICE_WAIT: u32 = 0x0112_c8f0;
    const VOICE_ID: u32 = 0x0112_c8f4;

    /// `msVoiceCheckStanbyOn()` (`0x007b2b70`): with a voice wait set, -1
    /// while `msVoiceCheckStanby(1)` is not ready, else `Voice_Prepare(wait)`
    /// and 0; 0 without one. Through `call`.
    pub fn ms_voice_check_stanby_on<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let wait: u32 = mem.read(addr(mem, VOICE_WAIT));
        if wait == 0 {
            return 0;
        }
        if go(mem, 0x007b_2c10, &[1], call) != 0 {
            return u32::MAX;
        }
        let wait: u32 = mem.read(addr(mem, VOICE_WAIT));
        go(mem, 0x0088_7570, &[wait], call);
        0
    }

    /// `msVoiceCheckStanbyOff()` (`0x007b2ba0`): with a voice wait set, -1
    /// while `msVoiceCheckStanby(0)` is not ready, else `0x00887630()` and,
    /// with a voice ID, `0x0079d090(id)` and the ID -1; returns 0. Through
    /// `call`.
    pub fn ms_voice_check_stanby_off<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let wait: u32 = mem.read(addr(mem, VOICE_WAIT));
        if wait == 0 {
            return 0;
        }
        if go(mem, 0x007b_2c10, &[0], call) != 0 {
            return u32::MAX;
        }
        go(mem, 0x0088_7630, &[], call);
        let id: u32 = mem.read(addr(mem, VOICE_ID));
        if id.cast_signed() >= 0 {
            go(mem, 0x0079_d090, &[id], call);
            mem.write(addr(mem, VOICE_ID), u32::MAX);
        }
        0
    }

    /// `msVoiceIsStreaming()` (`0x007b2be0`): 1 when `SndGetStreamState()`
    /// or `MsSystemMessageCheck()` (through `call`) is not 0, else 0.
    pub fn ms_voice_is_streaming<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        u32::from(go(mem, 0x0088_64b0, &[], call) != 0 || go(mem, 0x0079_d050, &[], call) != 0)
    }

    /// The scene names the `TO*` wrappers pass (`.rdata`): "bwakk",
    /// "bgood", "dum225".
    const BWAKK: u32 = 0x00b5_ed2c;
    const BGOOD: u32 = 0x00b5_ecdc;
    const DUM225: u32 = 0x00b5_ecbc;
    /// `g_TOBwakkOn` (`0x0133c93f`, byte).
    pub const BWAKK_ON: Global<u8> = Global::new(0x0133_c93f);

    /// `TOSepStop4e()` (`0x00890e70`): `0x0133f77c` = 0; when
    /// `SndIsSepPlay(0x8000004e)`, `SndSepStop(0x8000004e)`. Returns the
    /// last call's result.
    pub fn to_sep_stop_4e<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        mem.write(addr(mem, 0x0133_f77c), 0u32);
        let r = go(mem, 0x0088_6680, &[0x8000_004e], call);
        if r == 0 {
            return 0;
        }
        go(mem, 0x0088_6eb0, &[0x8000_004e], call)
    }

    /// `TOBwakkStop()` (`0x00890f10`): `TOSeqStopScene(2, "bwakk")`,
    /// `0x008e89d0()`, `g_TOBwakkOn` 0; returns the second call's result.
    pub fn to_bwakk_stop<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let s = addr(mem, BWAKK);
        go(mem, 0x008a_f4f0, &[2, s], call);
        let r = go(mem, 0x008e_89d0, &[], call);
        BWAKK_ON.set(mem, 0);
        r
    }

    /// `TOBwakkCheckTerm()` (`0x008977c0`): with `g_TOBwakkOn` 1,
    /// `TOSeqCheckTermScene(2, "bwakk")`, else 0.
    pub fn to_bwakk_check_term<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        if BWAKK_ON.get(mem) != 1 {
            return 0;
        }
        let s = addr(mem, BWAKK);
        go(mem, 0x008a_e070, &[2, s], call)
    }

    /// `TOBgoodCheckTerm()` (`0x008977e0`): `TOSeqCheckTermScene(2,
    /// "bgood")`.
    pub fn to_bgood_check_term<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let s = addr(mem, BGOOD);
        go(mem, 0x008a_e070, &[2, s], call)
    }

    /// `TODum225GetNum()` (`0x00898230`): `TOInitPadData()`,
    /// `TOSeqGetNum(2, "dum225", 0x0133f778)`, `g_TOBwakkOn` 0; returns
    /// `TOSeqGetNum`'s result.
    pub fn to_dum225_get_num<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        go(mem, 0x008a_dbd0, &[], call);
        let (s, out) = (addr(mem, DUM225), addr(mem, 0x0133_f778));
        let r = go(mem, 0x008a_e410, &[2, s, out], call);
        BWAKK_ON.set(mem, 0);
        r
    }

    /// `TOBwakkStart()` (`0x00898dc0`): `0x0133f520` = 0,
    /// `TOSeqStartSceneDrawContinue(2, "bwakk")`, `g_TOBwakkOn` 1; returns
    /// the call's result.
    pub fn to_bwakk_start<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        mem.write(addr(mem, 0x0133_f520), 0u32);
        let s = addr(mem, BWAKK);
        let r = go(mem, 0x008a_f1b0, &[2, s], call);
        BWAKK_ON.set(mem, 1);
        r
    }

    /// `TOBgoodStart()` (`0x0089ab80`): `TOSeqStartScene(2, "bgood")`,
    /// `SndPlayClick(0x68)`; returns the latter's result.
    pub fn to_bgood_start<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let s = addr(mem, BGOOD);
        go(mem, 0x008a_f0d0, &[2, s], call);
        go(mem, 0x0088_6ba0, &[0x68], call)
    }

    /// `g_OpDebugMode` (`0x02322790`, byte) and the debug reload bits
    /// (`0x02321580`, byte).
    pub const DEBUG_MODE: Global<u8> = Global::new(0x0232_2790);
    /// See [`DEBUG_MODE`].
    pub const DEBUG_RELOAD: Global<u8> = Global::new(0x0232_1580);

    /// One `opu_debug_reload` branch: the reload bit, the function run
    /// first on a debug object (if any), the object pair (two words of
    /// the debug table at `0x023215c0`) with the two buffers
    /// `0x007e5c10` loads them from, and whether the second is optional.
    struct Reload {
        bit: u8,
        first: Option<(u32, u32)>,
        objs: [u32; 2],
        bufs: [u32; 2],
        optional: bool,
    }

    const RELOADS: [Reload; 5] = [
        Reload {
            bit: 0x20,
            first: Some((0x007f_c5f0, 0x0232_15d8)),
            objs: [0x0232_15d8, 0x0232_15dc],
            bufs: [0x0232_2040, 0x0232_2140],
            optional: false,
        },
        Reload {
            bit: 1,
            first: None,
            objs: [0x0232_15c0, 0x0232_15c4],
            bufs: [0x0232_1640, 0x0232_1740],
            optional: false,
        },
        Reload {
            bit: 2,
            first: Some((0x007f_c650, 0x0232_15c8)),
            objs: [0x0232_15c8, 0x0232_15cc],
            bufs: [0x0232_1840, 0x0232_1940],
            optional: true,
        },
        Reload {
            bit: 4,
            first: Some((0x007f_ca90, 0x0232_15c8)),
            objs: [0x0232_15d0, 0x0232_15d4],
            bufs: [0x0232_1a40, 0x0232_1b40],
            optional: true,
        },
        Reload {
            bit: 0x10,
            first: None,
            objs: [0x0232_15e8, 0x0232_15ec],
            bufs: [0x0232_1e40, 0x0232_1f40],
            optional: false,
        },
    ];

    /// `opu_debug_reload()` (`0x007fdc80`): in debug mode, for each reload
    /// bit set (0x20 the encounter effect, 1, 2 a monster, 4 a weapon,
    /// 0x10): the object freed first where that applies
    /// (`op_et_encount_effect_free`, `op_et_monster_free`,
    /// `op_et_weapon_free`), its file reloaded (`0x007e5c10(buffer,
    /// obj)`), `op_oef2_set2(obj)`, then the second object (when there is
    /// one, or always) reloaded and `op_oef2_set(obj, second)`. All
    /// through `call`, each word read when it is passed.
    pub fn opu_debug_reload<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        if DEBUG_MODE.get(mem) == 0 {
            return;
        }
        let w = |mem: &M, sym: u32| mem.read::<u32>(addr(mem, sym));
        for r in &RELOADS {
            if DEBUG_RELOAD.get(mem) & r.bit == 0 {
                continue;
            }
            if let Some((f, arg)) = r.first {
                let a = w(mem, arg);
                go(mem, f, &[a], call);
            }
            let (b0, o0) = (addr(mem, r.bufs[0]), w(mem, r.objs[0]));
            go(mem, 0x007e_5c10, &[b0, o0], call);
            let o0 = w(mem, r.objs[0]);
            go(mem, 0x007e_3e30, &[o0], call);
            let o1 = w(mem, r.objs[1]);
            if r.optional && o1 == 0 {
                continue;
            }
            let b1 = addr(mem, r.bufs[1]);
            go(mem, 0x007e_5c10, &[b1, o1], call);
            let (o0, o1) = (w(mem, r.objs[0]), w(mem, r.objs[1]));
            go(mem, 0x007e_3b70, &[o0, o1], call);
        }
    }

    /// The texture rectangle `opp_draw_part_a74d30` builds for
    /// `op_gs_store`/`0x007e4420` from a GS register pair (`lo`, `hi`):
    /// `u16` base (`lo & 0x3fff`), width / 64 (`1 << (lo >> 26 & 15)`,
    /// signed), the `u16` bits 20..25, a zero word and 0x01000100 (14
    /// bytes, unaligned words).
    fn gs_rect(mem: &mut impl Mem, at: u32, lo: u32) {
        let w = 1i32.wrapping_shl((lo >> 26) & 0xf);
        let w = w.wrapping_add((w >> 31) & 0x3f) >> 6;
        #[allow(clippy::cast_possible_truncation)] // the u16 fields
        {
            mem.write(at, (lo & 0x3fff) as u16);
            mem.write(at + 2, w.cast_unsigned() as u16);
            mem.write(at + 4, ((lo >> 20) & 0x3f) as u16);
        }
        mem.write(at + 6, 0u32);
        mem.write(at + 0xa, 0x0100_0100u32);
    }

    /// The four `u16` at `cmd + (i16) cmd[+2]` into the object's first
    /// eight bytes; returns its first word.
    fn load_gs(mem: &mut impl Mem, obj: u32, cmd: u32) -> u32 {
        let src = cmd.wrapping_add(i32::from(mem.read::<i16>(cmd.wrapping_add(2))).cast_unsigned());
        for k in 0..4 {
            let v: u16 = mem.read(src.wrapping_add(2 * k));
            mem.write(obj.wrapping_add(2 * k), v);
        }
        mem.read(obj)
    }

    fn f64_at(mem: &impl Mem, sym: u32) -> f64 {
        mem.read(addr(mem, sym))
    }

    /// `opp_draw_part_a74d30(obj, cmd)` (`0x00a74d30`, the overlays'
    /// shared draw command; aligned stack): by the object's kind (`u16`
    /// `+0x21a & 0xf000`; `rec` = the part at `+0x220`):
    /// - 0: `yiDObjToOMD(0x6800000, 0)`; returns `cmd + 2`.
    /// - 0x1000: the matrix at `obj + 0x80` scaled by `rec +0x3c` / 1024
    ///   (`op_unit_m_scale`), `0x007e9850(m, m, rec + 0x30, rec + 0x90)`,
    ///   times the part's (`0x00801f00(obj, rec)`, `op_mul_m`), `obj +
    ///   0xc0` = `obj + 0x2c0` times it, `0x00a69300(0x6800000, obj +
    ///   0xc0, 0x1f00000)`; returns `cmd + 2`.
    /// - 0x2000: the part's buffer (`rec +0xbc`) freed and a 0x40000 one
    ///   taken (`op_free`, `0x007ff780` on `obj +0x3a8`); the GS pair from
    ///   the command into the object; a local matrix scaled by `rec +0x3c`
    ///   / 256 / 10 and moved by the map centre / 10 (`MsGetMapCenter`,
    ///   `0x007ea850`, `0x007ea550`); the rectangle ±`rec +0x40`/2560,
    ///   ±`rec +0x48`/2560 (times 10 for magic 0xaa and 0x25f); the
    ///   new buffer freed again, the texture stored (`op_gs_store` with a
    ///   fresh 0x40000 one), `movie_disp2(1, matrix, min, max, 0)`;
    ///   returns `cmd + 4`.
    /// - 0x3000: the GS pair into the object, `0x007e4420(rect, rec
    ///   +0xbc)`, `0x006438c0(rect base)`; returns `cmd + 4`.
    /// - else `cmd`.
    ///
    /// All callees through `call`. The locals the original keeps on its
    /// stack (frame pointer `frame`; down to `frame - 0x90`) are at
    /// `frame`, which the caller provides.
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn opp_draw_part<M: Mem>(
        mem: &mut M,
        obj: u32,
        cmd: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use crate::x87::{MUL, Reg};
        let div: crate::x87::Op = |a, b| a / b;
        let rec: u32 = mem.read(obj.wrapping_add(0x220));
        let f = |mem: &M, at: u32| mem.read::<f32>(at);
        match mem.read::<u16>(obj.wrapping_add(0x21a)) & 0xf000 {
            0 => {
                go(mem, 0x0090_8420, &[0x0680_0000, 0], call);
                cmd.wrapping_add(2)
            }
            0x1000 => {
                let k = f64_at(mem, 0x00b5_8b50);
                let sc = Reg::load_f32(f(mem, rec.wrapping_add(0x3c)))
                    .constant(MUL, k)
                    .store();
                let m = obj.wrapping_add(0x80);
                go(mem, 0x007e_a700, &[m, sc.to_bits()], call);
                go(
                    mem,
                    0x007e_9850,
                    &[m, m, rec.wrapping_add(0x30), rec.wrapping_add(0x90)],
                    call,
                );
                let part = go(mem, 0x0080_1f00, &[obj, rec], call);
                go(mem, 0x007e_83b0, &[m, part, m], call);
                let w = obj.wrapping_add(0xc0);
                go(mem, 0x007e_83b0, &[w, obj.wrapping_add(0x2c0), m], call);
                go(mem, 0x00a6_9300, &[0x0680_0000, w, 0x01f0_0000], call);
                cmd.wrapping_add(2)
            }
            0x2000 => {
                let heap = |mem: &M| mem.read::<u32>(obj.wrapping_add(0x3a8));
                let (h, b) = (heap(mem), mem.read::<u32>(rec.wrapping_add(0xbc)));
                go(mem, 0x007f_f1d0, &[h, b], call);
                let h = heap(mem);
                let nb = go(mem, 0x007f_f780, &[h, 0x40000], call);
                mem.write(rec.wrapping_add(0xbc), nb);
                let lo = load_gs(mem, obj, cmd);
                let (c1, c2) = (f64_at(mem, 0x00b5_0198), f64_at(mem, 0x00b9_22b0));
                let (mtx, centre, lo_v, hi_v, rect) = (
                    frame.wrapping_sub(0x48),
                    frame.wrapping_sub(0x90),
                    frame.wrapping_sub(0x80),
                    frame.wrapping_sub(0x70),
                    frame.wrapping_sub(0x58),
                );
                let sc = Reg::load_f32(f(mem, rec.wrapping_add(0x3c)))
                    .constant(MUL, c1)
                    .constant(div, c2)
                    .store();
                go(mem, 0x007e_a700, &[mtx, sc.to_bits()], call);
                go(mem, 0x0078_2ae0, &[centre], call);
                let tenth: f32 = mem.read(addr(mem, 0x00b1_0f38));
                go(
                    mem,
                    0x007e_a850,
                    &[centre, centre, crate::x87::load_f32(tenth).to_bits()],
                    call,
                );
                go(mem, 0x007e_a550, &[mtx, mtx, centre], call);
                let vf: [u32; 4] = crate::prog::bridge::VF00.get(mem);
                let half = |mem: &M, at: u32| {
                    Reg::load_f32(f(mem, rec.wrapping_add(at)))
                        .constant(MUL, c1)
                        .constant(div, c2)
                };
                let (a, bz) = (half(mem, 0x40), half(mem, 0x48));
                let neg = |r: Reg| match r {
                    Reg::Num(x) => Reg::Num(-x),
                    Reg::Nan(b) => Reg::Nan(b ^ 0x8000_0000),
                };
                let lo_vec = [
                    neg(a).store().to_bits(),
                    vf[1],
                    neg(bz).store().to_bits(),
                    vf[3],
                ];
                let hi_vec = [a.store().to_bits(), vf[1], bz.store().to_bits(), vf[3]];
                mem.write(lo_v, lo_vec);
                mem.write(hi_v, hi_vec);
                let (h, b) = (heap(mem), mem.read::<u32>(rec.wrapping_add(0xbc)));
                go(mem, 0x007f_f1d0, &[h, b], call);
                gs_rect(mem, rect, lo);
                let h = heap(mem);
                let nb = go(mem, 0x007f_f780, &[h, 0x40000], call);
                go(mem, 0x007e_4540, &[rect, nb], call);
                let id = go(mem, 0x009d_a3b0, &[], call);
                if id == 0xaa || go(mem, 0x009d_a3b0, &[], call) == 0x25f {
                    for at in [lo_v, lo_v + 4, lo_v + 8, hi_v, hi_v + 4, hi_v + 8] {
                        let v = Reg::load_f32(f(mem, at)).constant(MUL, c2).store();
                        mem.write(at, v);
                    }
                }
                go(mem, 0x0064_2fe0, &[1, mtx, lo_v, hi_v, 0], call);
                cmd.wrapping_add(4)
            }
            0x3000 => {
                let lo = load_gs(mem, obj, cmd);
                let rect = frame.wrapping_sub(0x58);
                gs_rect(mem, rect, lo);
                let b: u32 = mem.read(rec.wrapping_add(0xbc));
                go(mem, 0x007e_4420, &[rect, b], call);
                let base = i32::from(mem.read::<i16>(rect)).cast_unsigned();
                go(mem, 0x0064_38c0, &[base], call);
                cmd.wrapping_add(4)
            }
            _ => cmd,
        }
    }

    /// `TOBtlLibraGetRects()` (`0x00897dc0`): the Libra window's layout
    /// from scene 2: rectangles (`TOSeqGetRect(2, name, &x, &y, &w, &h)`)
    /// for four names and positions for seven (the names at
    /// `0x00b5ec5c`.. in `.rdata`), positions (`0x008ae450(2, name, &x,
    /// &y)`), into `0x0133f6a4`..`0x0133f774` (through
    /// `call`); then the Libra state block (`0x0133f6a8`, 0x5c bytes)
    /// cleared. Returns 0.
    pub fn to_btl_libra_get_rects<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        // (name, first output, rectangle)
        let layout: [(u32, [u32; 4], bool); 11] = [
            (
                0x00b5_ec5c,
                [0x0133_f6a4, 0x0133_f704, 0x0133_f708, 0x0133_f70c],
                true,
            ),
            (
                0x00b5_ec64,
                [0x0133_f710, 0x0133_f714, 0x0133_f718, 0x0133_f71c],
                true,
            ),
            (0x00b5_ec6c, [0x0133_f720, 0x0133_f724, 0, 0], false),
            (0x00b5_ec74, [0x0133_f728, 0x0133_f72c, 0, 0], false),
            (0x00b5_ec7c, [0x0133_f730, 0x0133_f734, 0, 0], false),
            (0x00b5_ec84, [0x0133_f738, 0x0133_f73c, 0, 0], false),
            (0x00b5_ec8c, [0x0133_f740, 0x0133_f744, 0, 0], false),
            (0x00b5_ec94, [0x0133_f748, 0x0133_f74c, 0, 0], false),
            (0x00b5_ec9c, [0x0133_f750, 0x0133_f754, 0, 0], false),
            (
                0x00b5_eca4,
                [0x0133_f758, 0x0133_f75c, 0x0133_f760, 0x0133_f764],
                true,
            ),
            (
                0x00b5_ecac,
                [0x0133_f768, 0x0133_f76c, 0x0133_f770, 0x0133_f774],
                true,
            ),
        ];
        for (name, outs, rect) in layout {
            let n = addr(mem, name);
            let o = outs.map(|a| if a == 0 { 0 } else { addr(mem, a) });
            if rect {
                go(mem, 0x008a_e4b0, &[2, n, o[0], o[1], o[2], o[3]], call);
            } else {
                go(mem, 0x008a_e450, &[2, n, o[0], o[1]], call);
            }
        }
        let block = addr(mem, 0x0133_f6a8);
        for k in 0..0x17 {
            mem.write(block + 4 * k, 0u32);
        }
        0
    }

    /// The object's kind (`u16 +0x21a & 0xf000`) and part (`+0x220`).
    fn kind_rec(mem: &impl Mem, obj: u32) -> (u16, u32) {
        (
            mem.read::<u16>(obj.wrapping_add(0x21a)) & 0xf000,
            mem.read(obj.wrapping_add(0x220)),
        )
    }

    /// `u16` at `cmd + off`, zero-extended.
    fn w(mem: &impl Mem, cmd: u32, off: u32) -> u32 {
        u32::from(mem.read::<u16>(cmd.wrapping_add(off)))
    }

    /// `i16` at `cmd + off`, sign-extended.
    fn sw(mem: &impl Mem, cmd: u32, off: u32) -> u32 {
        i32::from(mem.read::<i16>(cmd.wrapping_add(off))).cast_unsigned()
    }

    /// `opp_part_a76720(obj, cmd)` (`0x00a76720`): kind 0 clears the
    /// part's `u16 +0xc0`, kind 0x1000 calls `0x00a76770(obj, rec)`; both
    /// return `cmd + 2`, other kinds `cmd`.
    pub fn opp_part_a76720<M: Mem>(
        mem: &mut M,
        obj: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let (kind, rec) = kind_rec(mem, obj);
        match kind {
            0 => mem.write(rec.wrapping_add(0xc0), 0u16),
            0x1000 => {
                go(mem, 0x00a7_6770, &[obj, rec], call);
            }
            _ => return cmd,
        }
        cmd.wrapping_add(2)
    }

    /// `opp_map_draw_a74c60(obj, cmd)` (`0x00a74c60`): kind 0 takes a
    /// 0x27e20-byte buffer (`0x007ff780` on `obj +0x3a8`) into the part's
    /// `+0xbc`, sets `obj[0..4]` to the emulated r0 (`0x00c8f508`) with x
    /// and y 0.0 and calls `0x00a753f0(buffer, obj)`; kind 0x1000 sets
    /// them the same way with x and y 2048.0 and calls `opus_map_del_draw
    /// (obj, rec, rec +0xbc, rec + 0x90, obj +0x328)`. Both return `cmd +
    /// 2`, other kinds `cmd`.
    pub fn opp_map_draw_a74c60<M: Mem>(
        mem: &mut M,
        obj: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let (kind, rec) = kind_rec(mem, obj);
        let r0 = |mem: &mut M, xy: f32| {
            let v: [u32; 4] = mem.read(addr(mem, 0x00c8_f508));
            mem.write(obj, v);
            mem.write(obj, xy);
            mem.write(obj.wrapping_add(4), xy);
        };
        match kind {
            0 => {
                let h: u32 = mem.read(obj.wrapping_add(0x3a8));
                let b = go(mem, 0x007f_f780, &[h, 0x27e20], call);
                mem.write(rec.wrapping_add(0xbc), b);
                r0(mem, 0.0);
                let b: u32 = mem.read(rec.wrapping_add(0xbc));
                go(mem, 0x00a7_53f0, &[b, obj], call);
            }
            0x1000 => {
                let last: u32 = mem.read(obj.wrapping_add(0x328));
                let k = crate::x87::load_f32(mem.read(addr(mem, 0x00b5_1a20)));
                r0(mem, k);
                let b: u32 = mem.read(rec.wrapping_add(0xbc));
                go(
                    mem,
                    0x00a7_5170,
                    &[obj, rec, b, rec.wrapping_add(0x90), last],
                    call,
                );
            }
            _ => return cmd,
        }
        cmd.wrapping_add(2)
    }

    /// `opp_md_break_init_a74410(obj, cmd)` (`0x00a74410`): kind 0 only:
    /// the oef2 entry `op_oef2_a_get(oef, 4, i16 cmd[2])`, a buffer of
    /// `i16 entry +0x10` * 0x100 + 0x20 bytes into the part's `+0xbc`
    /// with `u16 +0x12` = the first word of the data at `cmd + i16
    /// cmd[4]` (kept at `rec +0xa8`), `0x00a744e0(entry, buffer, data)`,
    /// `rec +0xbb` = 0x82, `opu_id_list_add(obj + 0x300, obj +0x214)`, and
    /// `graphicVFXSetup(0, rec)` when
    /// `createGT3MeshBreakRenderDataBuffer(0, rec, buffer, 10)` is not 0;
    /// returns `cmd + 6`. Other kinds return `cmd`.
    pub fn opp_md_break_init_a74410<M: Mem>(
        mem: &mut M,
        obj: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let (kind, rec) = kind_rec(mem, obj);
        if kind != 0 {
            return cmd;
        }
        let data = cmd.wrapping_add(sw(mem, cmd, 4));
        let no = sw(mem, cmd, 2);
        let g = u32::from(mem.read::<u8>(rec.wrapping_add(0x1e)));
        let oef: u32 = mem.read(obj.wrapping_add(0x360).wrapping_add(4 * g));
        let e = go(mem, 0x007e_3810, &[oef, 4, no], call);
        let n = sw(mem, e, 0x10).wrapping_shl(8).wrapping_add(0x20);
        let h: u32 = mem.read(obj.wrapping_add(0x3a8));
        let b = go(mem, 0x007f_f780, &[h, n], call);
        mem.write(rec.wrapping_add(0xbc), b);
        let first: u16 = mem.read(data);
        mem.write(b.wrapping_add(0x12), first);
        mem.write(rec.wrapping_add(0xa8), data);
        go(mem, 0x00a7_44e0, &[e, b, data], call);
        mem.write(rec.wrapping_add(0xbb), 0x82u8);
        let id = w(mem, obj, 0x214);
        go(mem, 0x0080_0d60, &[obj.wrapping_add(0x300), id], call);
        if go(mem, 0x00a7_3c20, &[0, rec, b, 10], call) != 0 {
            go(mem, 0x0064_5780, &[0, rec], call);
        }
        cmd.wrapping_add(6)
    }

    /// `opus_wave_cmd_7f5f70(obj, cmd)` (`0x007f5f70`): kind 0x1000: the
    /// unit `opus_no_get(obj + 0x300, rec, u16 cmd[2])`, the oef2 data
    /// `u16 cmd[4]` read into a local (`0x007e5260(oef, no, frame -
    /// 0x30)`), `op_unit_m_scale(obj, 0.0625)`, the part's matrix
    /// (`0x00801f00(obj, rec)`), `0x0063df50()`, `0x00642fa0()`, the 10
    /// bytes at `cmd + 6` copied to `frame - 0x10`, and unless `0x012fb7d0`
    /// is set `opus_0154_wave_draw(obj + 0x300, rec, unit, frame - 0x30,
    /// frame - 0x10, obj, matrix, 0x0113fd50)`; returns `cmd + 0x10`.
    /// Kind 0x2000: `cmd + 4` when `MsGetAnimaChainOff()`, else `cmd + i16
    /// cmd[2]`. Other kinds `cmd`.
    ///
    /// The locals are below `frame` (the original's frame pointer; down
    /// to `frame - 0x30`), which the caller provides.
    pub fn opus_wave_cmd_7f5f70<M: Mem>(
        mem: &mut M,
        obj: u32,
        cmd: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let (kind, rec) = kind_rec(mem, obj);
        let g = u32::from(mem.read::<u8>(rec.wrapping_add(0x1e)));
        let oef: u32 = mem.read(obj.wrapping_add(0x360).wrapping_add(4 * g));
        match kind {
            0x1000 => {
                let (no, d) = (w(mem, cmd, 2), w(mem, cmd, 4));
                let unit = go(mem, 0x0080_1060, &[obj.wrapping_add(0x300), rec, no], call);
                let (buf, tail) = (frame.wrapping_sub(0x30), frame.wrapping_sub(0x10));
                go(mem, 0x007e_5260, &[oef, d, buf], call);
                let k: u32 = mem.read(addr(mem, 0x00b5_8354));
                go(mem, 0x007e_a700, &[obj, crate::x87::load(k)], call);
                let m = go(mem, 0x0080_1f00, &[obj, rec], call);
                go(mem, 0x0063_df50, &[], call);
                go(mem, 0x0064_2fa0, &[], call);
                let off: u32 = mem.read(addr(mem, 0x012f_b7d0));
                let a: [u32; 2] = mem.read(cmd.wrapping_add(6));
                let c: u16 = mem.read(cmd.wrapping_add(0xe));
                mem.write(tail, a);
                mem.write(tail.wrapping_add(8), c);
                if off == 0 {
                    let out = addr(mem, 0x0113_fd50);
                    go(
                        mem,
                        0x007f_65d0,
                        &[obj.wrapping_add(0x300), rec, unit, buf, tail, obj, m, out],
                        call,
                    );
                }
                cmd.wrapping_add(0x10)
            }
            0x2000 => {
                if go(mem, 0x0078_8dd0, &[], call) != 0 {
                    cmd.wrapping_add(4)
                } else {
                    cmd.wrapping_add(sw(mem, cmd, 2))
                }
            }
            _ => cmd,
        }
    }

    /// `opp_chr_center_a756e0(obj, cmd)` (`0x00a756e0`), on the character
    /// `g_EtChrHandles[rec +0x18]`: kind 0 `0x0063d810(chr, 0)` and
    /// `MsSetChrVisibleAppear(rec +0x18, 1)`, returns `cmd + 4`. Kind
    /// 0x1000: `t` = `+0x44` of the unit `opus_no_get(obj + 0x300, rec,
    /// u16 cmd[2])` / 256 (0 when negative); `0x0063a2b0(chr, rec + 0xe0,
    /// rec + 0xd0)`, both points times the root matrix
    /// (`Ch_GetRootScaledMatrix` into a local, `op_apply_m_w1`); the
    /// shader parameter "center" (`0x0063d880(chr, "center", &v)`) =
    /// `abs(b.y - (b.y - a.y) * t)` for the points `a` (`+0xd0`) and `b`
    /// (`+0xe0`), and `0x0063d870()` when `t >= 1`; returns `cmd + 0xe`.
    /// Other kinds `cmd`.
    ///
    /// The locals are below `frame` (the original's frame pointer; down
    /// to `frame - 0x50`), which the caller provides.
    pub fn opp_chr_center_a756e0<M: Mem>(
        mem: &mut M,
        obj: u32,
        cmd: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use crate::x87::{MUL, Reg, SUB};
        let (kind, rec) = kind_rec(mem, obj);
        let slot = u32::from(mem.read::<u8>(rec.wrapping_add(0x18)));
        let chr: u32 = mem.read(addr(mem, 0x012a_8100).wrapping_add(4 * slot));
        match kind {
            0 => {
                go(mem, 0x0063_d810, &[chr, 0], call);
                let slot = u32::from(mem.read::<u8>(rec.wrapping_add(0x18)));
                go(mem, 0x0079_66a0, &[slot, 1], call);
                cmd.wrapping_add(4)
            }
            0x1000 => {
                let no = w(mem, cmd, 2);
                let unit = go(mem, 0x0080_1060, &[obj.wrapping_add(0x300), rec, no], call);
                let v: f32 = mem.read(unit.wrapping_add(0x44));
                let t = if v < 0.0 {
                    0.0
                } else {
                    Reg::load_f32(v)
                        .constant(MUL, f64_at(mem, 0x00b5_0198))
                        .store()
                };
                let (a, b) = (rec.wrapping_add(0xd0), rec.wrapping_add(0xe0));
                go(mem, 0x0063_a2b0, &[chr, b, a], call);
                let m = frame.wrapping_sub(0x44);
                go(mem, 0x0082_6a80, &[m, chr], call);
                go(mem, 0x007e_79a0, &[a, m, a], call);
                go(mem, 0x007e_79a0, &[b, m, b], call);
                let by = Reg::load_f32(mem.read(b.wrapping_add(4)));
                let ay = Reg::load_f32(mem.read(a.wrapping_add(4)));
                let x = by.reg(SUB, ay).store();
                let y = by.reg(SUB, Reg::load_f32(x).mem(MUL, t)).store();
                let out = frame.wrapping_sub(0x48);
                mem.write(out, y.to_bits() & 0x7fff_ffff);
                let name = addr(mem, 0x00b7_c568);
                go(mem, 0x0063_d880, &[chr, name, out], call);
                if t >= 1.0 {
                    go(mem, 0x0063_d870, &[], call);
                }
                cmd.wrapping_add(0xe)
            }
            _ => cmd,
        }
    }

    /// `opu2_part(obj, cmd)` (`0x00a74a00`), on the part `rec` and the
    /// weapon/link block `t` = `obj +0x3b4` (its first word `p`, when
    /// `t` is not null):
    /// - 0x1000 (with `p`): the matrix at `obj + 0x40` = `op_unit_m_scale2
    ///   (p + 0x60)` rotated (`op_rt_m_xyz(m, m, p + 0x50, p + 0x40)`), the
    ///   point `*(t + 8)` into `obj`, `op_apply_m_w1(rec + 0x90, m, obj)`,
    ///   `op_xyz_scale(rec + 0x40, rec + 0x90, 16)`; returns `cmd + 2`
    ///   with or without `p`.
    /// - 0: the unit `opus_no_get(obj + 0x300, rec, u16 cmd[2])` into
    ///   `0x022fb4d0`; when its `+0xbb` is 3, the unit list's handler
    ///   (`obj +0x350`, `+0x64`) with `(obj + 0x300, unit +0xbc, u16
    ///   cmd[4], u16 cmd[6])`; returns `cmd + 8`.
    /// - 0x2000 (with `p`): `op_unit_m` and `op_rt_m_xyz` as above, then
    ///   for each non-zero offset `u16 cmd[2]`, `cmd[4]`, `cmd[6]` the
    ///   12-bit field (bits 4..16) of word 0, 1, 2 of `*(t + 0x14)` as a
    ///   float at `rec + offset`; returns `cmd + 8`.
    /// - 0x3000: (with `p`) `rec + u16 cmd[2]` = `p +0x60` * `*(t +
    ///   0x20)` (or 0.325 times it, with a warning, when `p +0x60` is 0) *
    ///   2^21, logged through `dbgPrintf`; returns `cmd + 4`.
    /// - else `cmd`.
    #[allow(clippy::too_many_lines)]
    pub fn opu2_part<M: Mem>(
        mem: &mut M,
        obj: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use crate::x87::{MUL, Reg};
        let t: u32 = mem.read(obj.wrapping_add(0x3b4));
        let rec: u32 = mem.read(obj.wrapping_add(0x220));
        let p: u32 = if t == 0 { 0 } else { mem.read(t) };
        let (c2, c4, c6) = (w(mem, cmd, 2), w(mem, cmd, 4), w(mem, cmd, 6));
        let kind = mem.read::<u16>(obj.wrapping_add(0x21a)) & 0xf000;
        let m = obj.wrapping_add(0x40);
        match kind {
            0x1000 => {
                if p != 0 {
                    go(mem, 0x007e_a730, &[m, p.wrapping_add(0x60)], call);
                    go(
                        mem,
                        0x007e_9970,
                        &[m, m, p.wrapping_add(0x50), p.wrapping_add(0x40)],
                        call,
                    );
                    let src: u32 = mem.read(t.wrapping_add(8));
                    for k in 0..3 {
                        let x: f32 = mem.read(src.wrapping_add(4 * k));
                        mem.write(obj.wrapping_add(4 * k), crate::x87::load_f32(x));
                    }
                    go(mem, 0x007e_79a0, &[rec.wrapping_add(0x90), m, obj], call);
                    let k: u32 = mem.read(addr(mem, 0x00b9_231c));
                    go(
                        mem,
                        0x007e_a850,
                        &[
                            rec.wrapping_add(0x40),
                            rec.wrapping_add(0x90),
                            crate::x87::load(k),
                        ],
                        call,
                    );
                }
                cmd.wrapping_add(2)
            }
            0 => {
                let unit = go(mem, 0x0080_1060, &[obj.wrapping_add(0x300), rec, c2], call);
                mem.write(addr(mem, 0x022f_b4d0), unit);
                if mem.read::<u8>(unit.wrapping_add(0xbb)) == 3 {
                    let list = obj.wrapping_add(0x300);
                    let vt: u32 = mem.read(list.wrapping_add(0x50));
                    let f: u32 = mem.read(vt.wrapping_add(0x64));
                    let b: u32 = mem.read(unit.wrapping_add(0xbc));
                    call(mem, f, &[list, b, c4, c6]);
                }
                cmd.wrapping_add(8)
            }
            0x2000 => {
                if p != 0 {
                    go(mem, 0x007e_a6d0, &[m], call);
                    go(
                        mem,
                        0x007e_9970,
                        &[m, m, p.wrapping_add(0x50), p.wrapping_add(0x40)],
                        call,
                    );
                    let src: u32 = mem.read(t.wrapping_add(0x14));
                    for (k, off) in (0u32..).zip([c2, c4, c6]) {
                        if off != 0 {
                            let v: u32 = mem.read(src.wrapping_add(4 * k));
                            #[allow(clippy::cast_precision_loss)] // 12 bits
                            let f = ((v.cast_signed() >> 4) & 0xfff) as f32;
                            mem.write(rec.wrapping_add(off), f);
                        }
                    }
                }
                cmd.wrapping_add(8)
            }
            0x3000 => {
                if p != 0 {
                    let s: u32 = mem.read(t.wrapping_add(0x20));
                    let scl: f32 = mem.read(p.wrapping_add(0x60));
                    let v = if scl == 0.0 {
                        go(mem, 0x0062_f500, &[addr(mem, 0x00b7_c4c8)], call);
                        Reg::load_f32(mem.read(s)).constant(MUL, f64_at(mem, 0x00b7_c558))
                    } else {
                        let d = f64::from(crate::x87::load_f32(scl)).to_bits();
                        #[allow(clippy::cast_possible_truncation)] // the halves
                        let (lo, hi) = (d as u32, (d >> 32) as u32);
                        go(mem, 0x0062_f500, &[addr(mem, 0x00b7_c520), lo, hi], call);
                        Reg::load_f32(mem.read(p.wrapping_add(0x60))).mem(MUL, mem.read(s))
                    };
                    let v = Reg::load_f32(v.store())
                        .constant(MUL, f64_at(mem, 0x00b7_c560))
                        .store();
                    mem.write(rec.wrapping_add(c2), v);
                }
                cmd.wrapping_add(4)
            }
            _ => cmd,
        }
    }

    /// `0x008062c0(out, rec +0x18, 256.0, at, rec + 0x90, i8 obj +0x21e)`,
    /// the part's model matrix.
    fn part_matrix<M: Mem>(
        mem: &mut M,
        obj: u32,
        rec: u32,
        out: u32,
        at: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let side = i32::from(mem.read::<i8>(obj.wrapping_add(0x21e))).cast_unsigned();
        let slot = u32::from(mem.read::<u8>(rec.wrapping_add(0x18)));
        let k: u32 = mem.read(addr(mem, 0x00b5_01c0));
        go(
            mem,
            0x0080_62c0,
            &[
                out,
                slot,
                crate::x87::load(k),
                at,
                rec.wrapping_add(0x90),
                side,
            ],
            call,
        );
    }

    /// `opp_chr_fade_center_a76b60(obj, cmd)` (`0x00a76b60`), on the
    /// character `g_EtChrHandles[rec +0x18]`:
    /// - 0x1000: the unit `opus_no_get(obj + 0x300, rec, u16 cmd[2])`;
    ///   the bone set `0x0080e160(obj + 0x100, obj, obj + 0x300, rec, n)`
    ///   (`n` = `i16 cmd[4]`) and `0x0080e900` with `cmd + 4`; `obj +0x14c`
    ///   = 0x80, `obj +0x110` and `+0x11c` = 0, `obj +0x118` = the unit's
    ///   `+0xc8`, `rec +0xa8` = 0. Locals: `P` = `obj +0x2c0` times the
    ///   part's matrix (`0x00801f00`); part matrices (`0x008062c0`) `A` at
    ///   `rec +0x30` and `B` at `obj + 4` (after `obj[0..4]` = the emulated
    ///   r0), and the one at the origin into `obj + 0x80`; `obj +0x12c` =
    ///   the local `C`. For each bone `i` (count `u16 set +4`, matrix
    ///   `get_ptr32_func` of the word at `set + 0x14 + 16 * i`): `obj +
    ///   0xc0` = `P` times `A` times the bone, `C` = `B` times it, the
    ///   unit's `+0x90` point through `C`'s inverse
    ///   (`Ch_GetRootGlobalMatrix`, `sceVu0InversMatrix4x4`,
    ///   `FFXVu0ApplyMatrix`) / 1000 gives the fade centres
    ///   (`graphicCharacterInstanceSetBTUFadeCenter` with 0 and y, 1 and
    ///   z), and `rec +0xa8` and `+0xaa` = the low half of `obj` and `i`
    ///   when that half is not 0; returns `cmd + 0xe`.
    /// - 0x2000: unless `obj +0x212` is set or `rec +0xa8` is 0, for the
    ///   unit `0x0080b520(obj + 0x300, rec, cmd + i16 cmd[2], 1, 0)`: the
    ///   oef2 entry `op_oef2_a_get(oef, 4, i16 cmd[4])`, its bone record
    ///   `0x00803200(entry, rec +0xaa)`, `obj` = the part matrix times the
    ///   bone, the record's point `rec +0xa8` through it into the unit's
    ///   `+0x90` (`op_apply_m_w1`), `op_xyz_scale(+0x40, +0x90, 16)` and
    ///   `+0x4c` = 0; returns `cmd + 6`.
    /// - else `cmd`.
    ///
    /// The locals are below `frame` (the original's frame pointer; down
    /// to `frame - 0x1d4`), which the caller provides.
    #[allow(clippy::too_many_lines)]
    pub fn opp_chr_fade_center_a76b60<M: Mem>(
        mem: &mut M,
        obj: u32,
        cmd: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use crate::x87::Reg;
        let div: crate::x87::Op = |a, b| a / b;
        let (kind, rec) = kind_rec(mem, obj);
        let slot = u32::from(mem.read::<u8>(rec.wrapping_add(0x18)));
        let chr: u32 = mem.read(addr(mem, 0x012a_8100).wrapping_add(4 * slot));
        let g = u32::from(mem.read::<u8>(rec.wrapping_add(0x1e)));
        let oef: u32 = mem.read(obj.wrapping_add(0x360).wrapping_add(4 * g));
        let list = obj.wrapping_add(0x300);
        match kind {
            0x1000 => {
                let (ma, mb, mc, mp) = (
                    frame.wrapping_sub(0xd4),
                    frame.wrapping_sub(0x194),
                    frame.wrapping_sub(0x94),
                    frame.wrapping_sub(0x154),
                );
                let (tmp, root, inv, v) = (
                    frame.wrapping_sub(0x54),
                    frame.wrapping_sub(0x1d4),
                    frame.wrapping_sub(0x114),
                    frame.wrapping_sub(0x14),
                );
                let no = w(mem, cmd, 2);
                let unit = go(mem, 0x0080_1060, &[list, rec, no], call);
                let bones = obj.wrapping_add(0x100);
                let n = sw(mem, cmd, 4);
                let set = go(mem, 0x0080_e160, &[bones, obj, list, rec, n], call);
                go(
                    mem,
                    0x0080_e900,
                    &[bones, obj, list, rec, cmd.wrapping_add(4)],
                    call,
                );
                mem.write(bones.wrapping_add(0x4c), 0x80u16);
                mem.write(bones.wrapping_add(0x10), 0u32);
                let c8: u32 = mem.read(unit.wrapping_add(0xc8));
                mem.write(bones.wrapping_add(0x18), c8);
                mem.write(bones.wrapping_add(0x1c), 0u32);
                mem.write(rec.wrapping_add(0xa8), 0u32);
                let m = go(mem, 0x0080_1f00, &[obj, rec], call);
                go(mem, 0x007e_83b0, &[mp, obj.wrapping_add(0x2c0), m], call);
                part_matrix(mem, obj, rec, ma, rec.wrapping_add(0x30), call);
                let r0: [u32; 4] = mem.read(addr(mem, 0x00c8_f508));
                mem.write(obj, r0);
                part_matrix(mem, obj, rec, mb, obj.wrapping_add(4), call);
                let origin = obj.wrapping_add(0x80);
                part_matrix(mem, obj, rec, origin, 0, call);
                mem.write(bones.wrapping_add(0x2c), mc);
                let mut i = 0u32;
                let mut at = set.wrapping_add(0x14);
                while i < u32::from(mem.read::<u16>(set.wrapping_add(4))) {
                    let bone = |mem: &mut M, call: &mut _| {
                        let b: u32 = mem.read(at);
                        go(mem, 0x00a4_4650, &[b], call)
                    };
                    let b = bone(mem, call);
                    go(mem, 0x007e_83b0, &[tmp, ma, b], call);
                    go(mem, 0x007e_83b0, &[obj.wrapping_add(0xc0), mp, tmp], call);
                    let b = bone(mem, call);
                    go(mem, 0x007e_83b0, &[tmp, origin, b], call);
                    let b = bone(mem, call);
                    go(mem, 0x007e_83b0, &[mc, mb, b], call);
                    go(mem, 0x0083_3f80, &[chr, root], call);
                    go(mem, 0x006e_d460, &[inv, mc], call);
                    for k in 0..3 {
                        let x: f32 = mem.read(unit.wrapping_add(0x90 + 4 * k));
                        mem.write(v.wrapping_add(4 * k), crate::x87::load_f32(x));
                    }
                    mem.write(v.wrapping_add(12), 1.0f32);
                    go(mem, 0x006e_d2e0, &[v, inv, v], call);
                    let thousand = f64_at(mem, 0x00b0_dba0);
                    for k in 0..3 {
                        let x: f32 = mem.read(v.wrapping_add(4 * k));
                        let q = Reg::load_f32(x).constant(div, thousand).store();
                        mem.write(v.wrapping_add(4 * k), q);
                    }
                    mem.write(v.wrapping_add(12), 1.0f32);
                    for (axis, at) in [(0u32, 4u32), (1, 8)] {
                        let c: u32 = mem.read(v.wrapping_add(at));
                        go(mem, 0x0063_b5d0, &[chr, axis, c], call);
                    }
                    #[allow(clippy::cast_possible_truncation)] // the low half
                    let low = obj as u16;
                    if low != 0 {
                        mem.write(rec.wrapping_add(0xa8), low);
                        #[allow(clippy::cast_possible_truncation)] // a u16 count
                        mem.write(rec.wrapping_add(0xaa), i as u16);
                    }
                    i += 1;
                    at = at.wrapping_add(0x10);
                }
                cmd.wrapping_add(0xe)
            }
            0x2000 => {
                if mem.read::<u16>(obj.wrapping_add(0x212)) != 0
                    || mem.read::<u16>(rec.wrapping_add(0xa8)) == 0
                {
                    return cmd.wrapping_add(6);
                }
                let data = cmd.wrapping_add(sw(mem, cmd, 2));
                let u = go(mem, 0x0080_b520, &[list, rec, data, 1, 0], call);
                if u == 0 {
                    return cmd.wrapping_add(6);
                }
                let e = go(mem, 0x007e_3810, &[oef, 4, sw(mem, cmd, 4)], call);
                let aa = w(mem, rec, 0xaa);
                let rec_b = go(mem, 0x0080_3200, &[e, aa], call);
                let pts: u32 = mem.read(rec_b.wrapping_add(8));
                let pt = (w(mem, rec, 0xa8) << 4)
                    .wrapping_add(pts)
                    .wrapping_add(rec_b);
                part_matrix(mem, obj, rec, obj, rec.wrapping_add(0x30), call);
                let mat: u32 = mem.read(e.wrapping_add((aa + 1) << 4).wrapping_add(4));
                let b = go(mem, 0x00a4_4650, &[mat], call);
                go(mem, 0x007e_83b0, &[obj, obj, b], call);
                go(mem, 0x007e_79a0, &[u.wrapping_add(0x90), obj, pt], call);
                let k: u32 = mem.read(addr(mem, 0x00b9_231c));
                go(
                    mem,
                    0x007e_a850,
                    &[
                        u.wrapping_add(0x40),
                        u.wrapping_add(0x90),
                        crate::x87::load(k),
                    ],
                    call,
                );
                mem.write(u.wrapping_add(0x4c), 0.0f32);
                cmd.wrapping_add(6)
            }
            _ => cmd,
        }
    }

    /// `opp2_optic_laser(obj, list)` (`0x00a73810`; `rec` = `obj +0x220`,
    /// `p` = its data `rec +0xa8`, the unit list `list`; always 0): the
    /// beam's render data (`graphicVFXGetRenderDataByInstance(0, rec)`
    /// when `0x012fb7d0` is 0); `n` = `rec +0x40` truncated
    /// (`_ftol2_sse`) points along the spline of the unit `u16 p[0]` (or
    /// the part) into `list +0x28` (`op_spline_xyz(at, t, seg +8, seg
    /// +0xc, i16 seg[0])`, `t` += 1 / `rec +0x48`); `obj + 0xc0` =
    /// `op_rt_m` of the unit `rec +0xb8`. For the target unit `u16 p[2]`
    /// (unless it is `list +0x20`): the oef2 shape `op_oef2_a_get(list
    /// +0x60[rec +0x1e], 4, i16 p[1])`, the object's header (`+2` = 0x80,
    /// `+4` = `rec +0xc8`, `+0x10`/`+0x1c` = `obj + 0x80`, `+0x14` =
    /// `list +0x54`), the render data set up and sized for `n` points
    /// (`InitVFXRenderDataShapeField`, `clearVFXRenderDataShp`,
    /// `graphicDynGeoMemManagerGetBufferForSegment`, returning 0 when it
    /// fails, and `memset`); `rec +0x16` = 0x10, `rec +0x17` at least
    /// `n`; each point (w = 1) through `obj + 0xc0` into `obj + 0xb0`, and
    /// unless its y is below `rec +0x94` (which flags `rec` and the target
    /// with 0x1000 in `+0xb0` and stops there) a scale matrix of `rec
    /// +0x3c` / 4096 at `obj + 0x80` and the point through the part's
    /// matrix (`0x00801f00`) into `obj + 0xb0`, then
    /// `updateVFXRenderDataShapeField`; the target at the last point
    /// (`op_xyz_scale` by 16), `list +0x54` = `obj +0x14`, and
    /// `graphicVFXUpdate`. All callees through `call`.
    ///
    /// The locals are below `frame` (the original's frame pointer; down
    /// to `frame - 0x28`), which the caller provides.
    #[allow(clippy::too_many_lines)]
    pub fn opp2_optic_laser<M: Mem>(
        mem: &mut M,
        obj: u32,
        list: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use crate::x87::{ADD, MUL, Reg};
        let div: crate::x87::Op = |a, b| a / b;
        let off = |mem: &M| mem.read::<u32>(addr(mem, 0x012f_b7d0)) != 0;
        let rec: u32 = mem.read(obj.wrapping_add(0x220));
        let p: u32 = mem.read(rec.wrapping_add(0xa8));
        let first = w(mem, p, 0);
        // the render instance (two words) and its data
        let (id_lo, id_hi, mut rd) = if off(mem) {
            let lo: u32 = mem.read(frame.wrapping_sub(0x28));
            (lo, rec, list)
        } else {
            let rd = go(mem, 0x0064_53d0, &[0, rec], call);
            (0, rec, rd)
        };
        let src = if first == 0 {
            mem.read(obj.wrapping_add(0x220))
        } else {
            go(mem, 0x0080_1060, &[list, rec, first], call)
        };
        let seg: u32 = mem.read(src.wrapping_add(0xbc));
        let step = Reg::Num(1.0)
            .reg(div, Reg::load_f32(mem.read(rec.wrapping_add(0x48))))
            .store();
        let mut t = 0.0f32;
        let x: f32 = mem.read(rec.wrapping_add(0x40));
        let n = ffx_bridge::x87::ftol(mem, crate::x87::Reg::load_f32(x).value()).cast_signed();
        let pts: u32 = mem.read(list.wrapping_add(0x28));
        let mut at = pts;
        for _ in 0..n.max(0) {
            let (a, b) = (
                mem.read::<u32>(seg.wrapping_add(8)),
                mem.read::<u32>(seg.wrapping_add(0xc)),
            );
            let k = sw(mem, seg, 0);
            go(mem, 0x007e_a120, &[at, t.to_bits(), a, b, k], call);
            t = Reg::load_f32(t).mem(ADD, step).store();
            at = at.wrapping_add(0x10);
        }
        let mb = w(mem, rec, 0xb8);
        let u = go(mem, 0x0080_1060, &[list, rec, mb], call);
        let m = obj.wrapping_add(0xc0);
        go(mem, 0x007e_a700, &[m, 1.0f32.to_bits()], call);
        go(
            mem,
            0x007e_9850,
            &[m, m, u.wrapping_add(0x30), u.wrapping_add(0x90)],
            call,
        );
        let tn = w(mem, p, 4);
        let target = go(mem, 0x0080_1060, &[list, rec, tn], call);
        if target == mem.read::<u32>(list.wrapping_add(0x20)) {
            return 0;
        }
        let no = sw(mem, p, 2);
        let g = u32::from(mem.read::<u8>(rec.wrapping_add(0x1e)));
        let oef: u32 = mem.read(list.wrapping_add(0x60).wrapping_add(4 * g));
        let e = go(mem, 0x007e_3810, &[oef, 4, no], call);
        let shp = e.wrapping_add(sw(mem, e, 0x10));
        mem.write(obj.wrapping_add(2), 0x80u16);
        let c8: u32 = mem.read(rec.wrapping_add(0xc8));
        mem.write(obj.wrapping_add(4), c8);
        mem.write(obj.wrapping_add(8), 0u32);
        mem.write(obj.wrapping_add(0x1c), obj.wrapping_add(0x80));
        mem.write(obj.wrapping_add(0x10), obj.wrapping_add(0x80));
        let l54: u32 = mem.read(list.wrapping_add(0x54));
        mem.write(obj.wrapping_add(0x14), l54);
        let local = frame.wrapping_sub(0x24);
        let nu = n.cast_unsigned();
        if !off(mem) {
            let ptr = go(mem, 0x00a4_4700, &[e], call);
            mem.write(local, ptr);
            if rd == 0 {
                let o: u32 = mem.read(shp.wrapping_add(4));
                let b = u32::from(mem.read::<u8>(o.wrapping_add(shp).wrapping_add(4)));
                let alpha = go(mem, 0x0071_2c30, &[b], call);
                go(mem, 0x0071_13d0, &[id_lo, id_hi, local, nu, alpha], call);
                rd = go(mem, 0x0064_53d0, &[id_lo, id_hi], call);
            }
            go(mem, 0x0071_3290, &[id_lo, id_hi], call);
            if rd != 0 {
                let geo = |mem: &M| mem.read::<u32>(rd.wrapping_add(0x94));
                let k = sw(mem, shp, 2).wrapping_mul(nu);
                mem.write(geo(mem).wrapping_add(8), k.wrapping_mul(6));
                let k = sw(mem, shp, 2).wrapping_mul(nu);
                mem.write(geo(mem).wrapping_add(4), k.wrapping_mul(4));
                let g = geo(mem);
                let (a, b) = (
                    mem.read::<u32>(g.wrapping_add(4)),
                    mem.read::<u32>(g.wrapping_add(8)),
                );
                if go(mem, 0x0063_fdb0, &[rd, 0, a, b], call) == 0 {
                    return 0;
                }
                let g = geo(mem);
                let (to, len) = (
                    mem.read::<u32>(g.wrapping_add(0x1c)),
                    mem.read::<u32>(g.wrapping_add(8)),
                );
                go(mem, 0x0094_964c, &[to, 0, len.wrapping_mul(2)], call);
            }
            go(mem, 0x0071_3290, &[id_lo, id_hi], call);
        }
        let part = go(mem, 0x0080_1f00, &[obj, rec], call);
        mem.write(rec.wrapping_add(0x16), 0x10u8);
        if i32::from(mem.read::<u8>(rec.wrapping_add(0x17))) < n {
            mem.write(rec.wrapping_add(0x17), n.to_le_bytes()[0]);
        }
        let mut at = mem.read::<u32>(list.wrapping_add(0x28));
        let out = obj.wrapping_add(0xb0);
        let mut i = 0;
        let mut stopped = false;
        while i < n {
            mem.write(at.wrapping_add(0xc), 1.0f32);
            go(mem, 0x006e_d2e0, &[out, m, at], call);
            let y: f32 = mem.read(obj.wrapping_add(0xb4));
            let floor: f32 = mem.read(rec.wrapping_add(0x94));
            if floor < y {
                for u in [rec, target] {
                    let f: u16 = mem.read(u.wrapping_add(0xb0));
                    mem.write(u.wrapping_add(0xb0), f | 0x1000);
                }
                stopped = true;
                break;
            }
            let sc: f32 = mem.read(rec.wrapping_add(0x3c));
            let k = Reg::load_f32(sc)
                .constant(MUL, f64_at(mem, 0x00b5_0190))
                .store();
            mem.write(local, k);
            for z in [
                0x8c, 0x88, 0x84, 0x9c, 0x98, 0x90, 0xac, 0xa4, 0xa0, 0xb8, 0xb4, 0xb0,
            ] {
                mem.write(obj.wrapping_add(z), 0.0f32);
            }
            for d in [0xa8, 0x94, 0x80] {
                mem.write(obj.wrapping_add(d), k);
            }
            mem.write(obj.wrapping_add(0xbc), 1.0f32);
            go(mem, 0x006e_d2e0, &[out, part, at], call);
            if !off(mem) && rd != 0 {
                go(mem, 0x0071_a4e0, &[id_lo, id_hi, shp, obj], call);
            }
            i += 1;
            at = at.wrapping_add(0x10);
        }
        if !stopped {
            at = at.wrapping_sub(0x10);
        }
        for k in 0..3 {
            let v: f32 = mem.read(at.wrapping_add(4 * k));
            mem.write(target.wrapping_add(0x90 + 4 * k), crate::x87::load_f32(v));
        }
        let sixteen: u32 = mem.read(addr(mem, 0x00b9_231c));
        go(
            mem,
            0x007e_a850,
            &[
                target.wrapping_add(0x40),
                target.wrapping_add(0x90),
                crate::x87::load(sixteen),
            ],
            call,
        );
        let o14: u32 = mem.read(obj.wrapping_add(0x14));
        mem.write(list.wrapping_add(0x54), o14);
        if !off(mem) && rd != 0 {
            go(mem, 0x0064_58a0, &[id_lo, id_hi], call);
        }
        0
    }

    /// `opp2_md_break(obj, list)` (`0x00a73d90`; `rec` = `obj +0x220`, its
    /// break data `d` = `rec +0xbc` with `d +4` pieces of 0x100 bytes from
    /// `d + 0x20`; always 0): `obj + 0x80` = a scale of `rec +0x3c` / 2^21
    /// moved to the part (`op_unit_m_scale`, `op_rt_m`), `obj + 0xc0` =
    /// `obj +0x2c0` times the part's matrix; a 0x200-byte scratch
    /// (`op_spr_alloc`). Unless `obj +0x212` is set, each piece moves (its
    /// velocities `+0x70`.. and `+0xa0`.. added to `+0x60`.. and
    /// `+0x90`.., and those to `+0x50`.. and `+0x80`.., the `+0xd0` word
    /// by `+0xd4`); its rotation (`op_rot_m(piece + 0xe0, obj + 0x80,
    /// piece + 0xa0)`) and its position at `+0x110` (the `+0x50` point,
    /// or on a circle by the sine table for mode 1 in `d +0x12`). With
    /// the render data (`graphicVFXGetRenderDataByInstance(0, rec)`) and
    /// a 3 * pieces buffer (`graphicDynGeoMemManagerGetBufferForSegment`),
    /// every piece with a positive `+0xb0` / 4096 alpha adds a triangle:
    /// its three corners through its matrix (`FFXVu0CopyMatrix`,
    /// `FFXVu0ApplyMatrix`), colours (bytes / 256, alpha times the
    /// alpha), texture coordinates (`u16` / 4096) and indices; the rest of
    /// the index buffer is cleared, the data's matrix reset with a
    /// ±1e7 box, made visible and updated. `list +0x54` passes through
    /// the scratch; with no triangle the part is flagged (`rec +0xb0` |=
    /// the data's `u16 +2`), its drawing turned off (`opus_draw_off`),
    /// `obj +0x218` decremented and the render data destroyed. All
    /// callees through `call`.
    ///
    /// The locals are below `frame` (the original's frame pointer; down
    /// to `frame - 0x54`), which the caller provides.
    #[allow(clippy::too_many_lines)]
    pub fn opp2_md_break<M: Mem>(
        mem: &mut M,
        obj: u32,
        list: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use crate::x87::{ADD, MUL, Reg};
        let f = |mem: &M, at: u32| mem.read::<f32>(at);
        let rec: u32 = mem.read(obj.wrapping_add(0x220));
        let sc = Reg::load_f32(f(mem, rec.wrapping_add(0x3c)))
            .constant(MUL, f64_at(mem, 0x00b5_8398))
            .store();
        let m = obj.wrapping_add(0x80);
        go(mem, 0x007e_a700, &[m, sc.to_bits()], call);
        go(
            mem,
            0x007e_9850,
            &[m, m, rec.wrapping_add(0x30), rec.wrapping_add(0x90)],
            call,
        );
        let part = go(mem, 0x0080_1f00, &[obj, rec], call);
        go(
            mem,
            0x007e_83b0,
            &[obj.wrapping_add(0xc0), obj.wrapping_add(0x2c0), part],
            call,
        );
        let spr = go(mem, 0x007e_6a00, &[0x200], call);
        let d: u32 = mem.read(rec.wrapping_add(0xbc));
        mem.write(spr.wrapping_add(0x10), addr(mem, 0x012a_8da8));
        let n: i32 = mem.read(d.wrapping_add(4));
        let mut pf = d.wrapping_add(0x80);
        for _ in 0..n.max(0) {
            if mem.read::<u16>(obj.wrapping_add(0x212)) == 0 {
                let v: u32 = mem.read(pf.wrapping_add(0x54));
                let c: u32 = mem.read(pf.wrapping_add(0x50));
                mem.write(pf.wrapping_add(0x50), c.wrapping_add(v));
                for (vel, pos, acc) in [
                    (0x10u32, 0u32, 0u32.wrapping_sub(0x10)),
                    (0x14, 4, 0u32.wrapping_sub(0xc)),
                    (0x18, 8, 0u32.wrapping_sub(8)),
                    (0x40, 0x30, 0x20),
                    (0x44, 0x34, 0x24),
                    (0x48, 0x38, 0x28),
                ] {
                    let a = Reg::load_f32(f(mem, pf.wrapping_add(vel)))
                        .mem(ADD, f(mem, pf.wrapping_add(pos)))
                        .store();
                    mem.write(pf.wrapping_add(pos), a);
                    let b = Reg::load_f32(a)
                        .mem(ADD, f(mem, pf.wrapping_add(acc)))
                        .store();
                    mem.write(pf.wrapping_add(acc), b);
                }
            }
            go(
                mem,
                0x007e_9530,
                &[pf.wrapping_add(0x60), m, pf.wrapping_add(0x20)],
                call,
            );
            let src = |k: u32| pf.wrapping_sub(0x10).wrapping_add(4 * k);
            match mem.read::<u16>(d.wrapping_add(0x12)) {
                0 => {
                    for k in 0..3 {
                        let x = crate::x87::load_f32(f(mem, src(k)));
                        mem.write(pf.wrapping_add(0x90 + 4 * k), x);
                    }
                }
                1 => {
                    let a = ffx_bridge::x87::ftol(mem, Reg::load_f32(f(mem, src(2))).value());
                    let i = (a & 0xfff) + 0x400;
                    let r = f(mem, src(0));
                    let c = f(mem, addr(mem, 0x00c4_3bf0).wrapping_add(4 * i));
                    mem.write(pf.wrapping_add(0x90), Reg::load_f32(c).mem(MUL, r).store());
                    let r = f(mem, src(0));
                    let sn = f(mem, addr(mem, 0x00c4_4bf0).wrapping_add(4 * (i & 0xfff)));
                    mem.write(pf.wrapping_add(0x98), Reg::load_f32(sn).mem(MUL, r).store());
                    let y = crate::x87::load_f32(f(mem, src(1)));
                    mem.write(pf.wrapping_add(0x94), y);
                }
                _ => {}
            }
            pf = pf.wrapping_add(0x100);
        }
        let l54: u32 = mem.read(list.wrapping_add(0x54));
        mem.write(spr.wrapping_add(0x20), l54);
        let n: u32 = mem.read(d.wrapping_add(4));
        mem.write(spr.wrapping_add(0x24), n);
        mem.write(spr.wrapping_add(0x28), 0u32);
        let rd = go(mem, 0x0064_53d0, &[0, rec], call);
        let count = |mem: &M| mem.read::<u32>(spr.wrapping_add(0x28));
        let geo = |mem: &M, at: u32| {
            let g: u32 = mem.read(rd.wrapping_add(0x94));
            mem.read::<u32>(g.wrapping_add(at))
        };
        let c3 = mem.read::<u32>(d.wrapping_add(4)).wrapping_mul(3);
        if rd != 0 && go(mem, 0x0063_fdb0, &[rd, 0, c3, c3], call) != 0 {
            let q = f64_at(mem, 0x00b5_0190);
            let c256 = f64_at(mem, 0x00b5_0198);
            let (mat, v, out) = (
                frame.wrapping_sub(0x54),
                frame.wrapping_sub(0x24),
                frame.wrapping_sub(0x14),
            );
            let mut i = 0i32;
            let mut pb = d.wrapping_add(0x20);
            while i < mem.read::<i32>(d.wrapping_add(4)) {
                #[allow(clippy::cast_precision_loss)] // fild, fstp m32
                let raw = mem.read::<i32>(pb.wrapping_add(0xb0)) as f32;
                let alpha = Reg::load_f32(raw).constant(MUL, q).store();
                if alpha > 0.0 {
                    go(mem, 0x006e_d360, &[mat, pb.wrapping_add(0xc0)], call);
                    mem.write(v.wrapping_add(0xc), 1.0f32);
                    go(mem, 0x006e_d2e0, &[v, m, v], call);
                    let k = count(mem).wrapping_mul(36);
                    let vb = geo(mem, 0xc).wrapping_add(k);
                    for at in [0x3c, 0x2c, 0x1c] {
                        mem.write(pb.wrapping_add(at), 1.0f32);
                    }
                    for (c, corner) in [0x10u32, 0x20, 0x30].into_iter().enumerate() {
                        go(mem, 0x006e_d2e0, &[out, mat, pb.wrapping_add(corner)], call);
                        for j in 0..3u32 {
                            let x = crate::x87::load_f32(f(mem, out.wrapping_add(4 * j)));
                            #[allow(clippy::cast_possible_truncation)] // 0..3
                            let at = vb.wrapping_add(12 * c as u32 + 4 * j);
                            mem.write(at, x);
                        }
                    }
                    let cb = geo(mem, 0x14).wrapping_add(count(mem).wrapping_mul(0x30));
                    for j in 0..12u32 {
                        let b = f64::from(mem.read::<u8>(pb.wrapping_add(j)));
                        let x = if j % 4 == 3 {
                            Reg::Num(b).mem(MUL, alpha).constant(MUL, c256)
                        } else {
                            Reg::Num(b).constant(MUL, c256)
                        };
                        mem.write(cb.wrapping_add(4 * j), x.store());
                    }
                    let uv = geo(mem, 0x18).wrapping_add(count(mem).wrapping_mul(0x18));
                    for j in 0..6u32 {
                        let t = f64::from(mem.read::<u16>(pb.wrapping_add(0x40 + 2 * j)));
                        mem.write(uv.wrapping_add(4 * j), Reg::Num(t).constant(MUL, q).store());
                    }
                    for j in 0..3u16 {
                        #[allow(clippy::cast_possible_truncation)] // the low word
                        let low = count(mem) as u16;
                        let x = low.wrapping_mul(3).wrapping_add(j);
                        let at = geo(mem, 0x1c)
                            .wrapping_add(count(mem).wrapping_mul(6))
                            .wrapping_add(u32::from(2 * j));
                        mem.write(at, x);
                    }
                    let c = count(mem);
                    mem.write(spr.wrapping_add(0x28), c.wrapping_add(1));
                }
                i += 1;
                pb = pb.wrapping_add(0x100);
            }
            let k = count(mem);
            let n: u32 = mem.read(d.wrapping_add(4));
            if k.cast_signed() < n.cast_signed() {
                let to = geo(mem, 0x1c).wrapping_add(k.wrapping_mul(6));
                go(
                    mem,
                    0x0094_964c,
                    &[to, 0, n.wrapping_sub(k).wrapping_mul(6)],
                    call,
                );
            }
            mem.write(rd.wrapping_add(0xb0), 0u32);
            go(mem, 0x006e_d9a0, &[rd], call);
            mem.write(rd.wrapping_add(0x80), 0u32);
            let lo: u32 = mem.read(addr(mem, 0x00b5_1660));
            let hi: u32 = mem.read(addr(mem, 0x00b5_1658));
            for (at, x) in [
                (0x98, lo),
                (0x9c, lo),
                (0xa0, lo),
                (0xa4, hi),
                (0xa8, hi),
                (0xac, hi),
            ] {
                mem.write(rd.wrapping_add(at), crate::x87::load(x));
            }
            go(mem, 0x0064_5710, &[0, rec, 1], call);
            go(mem, 0x0064_58a0, &[0, rec], call);
        }
        let s20: u32 = mem.read(spr.wrapping_add(0x20));
        mem.write(list.wrapping_add(0x54), s20);
        if count(mem) == 0 {
            let data: u32 = mem.read(rec.wrapping_add(0xa8));
            let bits: u16 = mem.read(data.wrapping_add(2));
            let flags: u16 = mem.read(rec.wrapping_add(0xb0));
            mem.write(rec.wrapping_add(0xb0), flags | bits);
            let id = w(mem, obj, 0x214);
            go(mem, 0x0080_0d10, &[list, id], call);
            let k: u16 = mem.read(obj.wrapping_add(0x218));
            mem.write(obj.wrapping_add(0x218), k.wrapping_sub(1));
            go(mem, 0x0064_5280, &[0, rec], call);
        }
        go(mem, 0x007e_6a60, &[0x200], call);
        0
    }

    /// `opp_0582_cmd_a70910(obj, cmd)` (`0x00a70910`), the `opus_0582_*`
    /// effect's command on the part `rec` (`obj +0x220`), the character
    /// `g_EtChrHandles[rec +0x18]` and the unit list `obj + 0x300`:
    /// - 0: a buffer of `n` = `i16 cmd[2]` 32-byte entries plus 0x10 into
    ///   `rec +0xbc` (`0x007ff780` on `obj +0x3a8`), its header `n`, 0;
    ///   `cmd + 4`.
    /// - 0x1000: `obj[0..2]` = `n` / 256, `obj[2]` = `rec +0x3c` / 256; the
    ///   matrix `obj + 0x40` scaled by them (`op_unit_m_scale2`) and moved
    ///   to `rec + 0x90` (`op_trans_m`), `obj` = the part's matrix times
    ///   it, `opus_0582_dot_draw(rec +0xbc, rec, obj, i16 obj +0x212)`;
    ///   `cmd + 4`.
    /// - 0x2000: unless `obj +0x212`, `0x00a73740(unit +0xbc, rec)` for the
    ///   unit `u16 cmd[2]`; `cmd + 4`.
    /// - 0x4000: the character's texture bases (`Ch_TextureSetClutBase(chr,
    ///   -1, 0x3c00)`, `Ch_TextureSetImageBase(chr, -1, 0x2ee0)`),
    ///   `0x00803220(points, chr, 0, points + 0x80000)` on `list +0x28`, a
    ///   buffer of `0x00a72310(points)` bytes into `rec +0xbc`, `obj` = the
    ///   part matrix, `opus_0582_break_make(buffer, points, obj)`; `cmd + 2`.
    /// - 0x5000: `obj` = the part matrix, the textures loaded
    ///   (`Ch_TextureLoadImage`, `Ch_TextureLoadClut`, slot -1),
    ///   `opus_0582_draw(rec +0xbc, rec, unit, obj, obj + 0x280, obj +
    ///   0x2c0)` for the unit `u16 cmd[2]`; `cmd + 4`.
    /// - 0x6000: a buffer of `n` 48-byte entries plus 0x10, its header `n`,
    ///   0, each entry's first `u16` cleared; `cmd + 4`.
    /// - 0x7000: `obj` = the unit `u16 cmd[2]`'s matrix (`0x008062c0` at
    ///   its `+0x30`, `+0x90`), `obj + 0xc0` = the part's matrix
    ///   (`op_unit_m` at `obj + 0x40`, `op_mul_m`),
    ///   `opus_0582_break_draw(rec +0xbc, unit +0xbc, rec, unit u16
    ///   cmd[4], obj, obj + 0xc0, i16 obj +0x212)`; `cmd + 6`.
    /// - 0x8000: `Sg_FadeOutW(1)`; `cmd + 2`.
    /// - else `cmd`.
    #[allow(clippy::too_many_lines)]
    pub fn opp_0582_cmd_a70910<M: Mem>(
        mem: &mut M,
        obj: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use crate::x87::{MUL, Reg};
        let (kind, rec) = kind_rec(mem, obj);
        let slot = u32::from(mem.read::<u8>(rec.wrapping_add(0x18)));
        let chr: u32 = mem.read(addr(mem, 0x012a_8100).wrapping_add(4 * slot));
        let list = obj.wrapping_add(0x300);
        let heap = |mem: &M| mem.read::<u32>(obj.wrapping_add(0x3a8));
        let table = |mem: &mut M, size: u32, call: &mut _| {
            let n = sw(mem, cmd, 2);
            let h = heap(mem);
            let b = go(
                mem,
                0x007f_f780,
                &[h, n.wrapping_mul(size).wrapping_add(0x10)],
                call,
            );
            mem.write(rec.wrapping_add(0xbc), b);
            mem.write(b, n);
            mem.write(b.wrapping_add(4), 0u32);
            (n, b)
        };
        match kind {
            0 => {
                table(mem, 0x20, call);
                cmd.wrapping_add(4)
            }
            0x1000 => {
                let c = f64_at(mem, 0x00b5_0198);
                #[allow(clippy::cast_precision_loss)] // an i16
                let v = f64::from(mem.read::<i16>(cmd.wrapping_add(2)));
                let s = Reg::Num(v).constant(MUL, c).store();
                mem.write(obj.wrapping_add(4), s);
                mem.write(obj, s);
                let z = Reg::Num(c)
                    .mem(MUL, mem.read(rec.wrapping_add(0x3c)))
                    .store();
                mem.write(obj.wrapping_add(8), z);
                let m = obj.wrapping_add(0x40);
                go(mem, 0x007e_a730, &[m, obj], call);
                go(mem, 0x007e_a550, &[m, m, rec.wrapping_add(0x90)], call);
                let part = go(mem, 0x0080_1f00, &[obj, rec], call);
                go(mem, 0x007e_83b0, &[obj, part, m], call);
                let flag = sw(mem, obj, 0x212);
                let b: u32 = mem.read(rec.wrapping_add(0xbc));
                go(mem, 0x00a7_2b80, &[b, rec, obj, flag], call);
                cmd.wrapping_add(4)
            }
            0x2000 => {
                if mem.read::<u16>(obj.wrapping_add(0x212)) == 0 {
                    let no = w(mem, cmd, 2);
                    let u = go(mem, 0x0080_1060, &[list, rec, no], call);
                    let b: u32 = mem.read(u.wrapping_add(0xbc));
                    go(mem, 0x00a7_3740, &[b, rec], call);
                }
                cmd.wrapping_add(4)
            }
            0x4000 => {
                let pts: u32 = mem.read(list.wrapping_add(0x28));
                go(mem, 0x0083_13c0, &[chr, u32::MAX, 0x3c00], call);
                go(mem, 0x0083_1390, &[chr, u32::MAX, 0x2ee0], call);
                let end = mem
                    .read::<u32>(list.wrapping_add(0x28))
                    .wrapping_add(0x8_0000);
                go(mem, 0x0080_3220, &[pts, chr, 0, end], call);
                let size = go(mem, 0x00a7_2310, &[pts], call);
                let h = heap(mem);
                let b = go(mem, 0x007f_f780, &[h, size], call);
                mem.write(rec.wrapping_add(0xbc), b);
                part_matrix(mem, obj, rec, obj, rec.wrapping_add(0x30), call);
                go(mem, 0x00a7_1780, &[b, pts, obj], call);
                cmd.wrapping_add(2)
            }
            0x5000 => {
                let no = w(mem, cmd, 2);
                let u = go(mem, 0x0080_1060, &[list, rec, no], call);
                part_matrix(mem, obj, rec, obj, rec.wrapping_add(0x30), call);
                go(mem, 0x0083_1300, &[chr, u32::MAX], call);
                go(mem, 0x0083_11d0, &[chr, u32::MAX], call);
                let b: u32 = mem.read(rec.wrapping_add(0xbc));
                go(
                    mem,
                    0x00a7_0d70,
                    &[
                        b,
                        rec,
                        u,
                        obj,
                        obj.wrapping_add(0x280),
                        obj.wrapping_add(0x2c0),
                    ],
                    call,
                );
                cmd.wrapping_add(4)
            }
            0x6000 => {
                let (n, b) = table(mem, 0x30, call);
                if n != 0 {
                    let mut i = 0u32;
                    loop {
                        mem.write(b.wrapping_add(0x10).wrapping_add(0x30 * i), 0u16);
                        i = i.wrapping_add(1);
                        if i >= mem.read::<u32>(b) {
                            break;
                        }
                    }
                }
                cmd.wrapping_add(4)
            }
            0x7000 => {
                let no = w(mem, cmd, 2);
                let u = go(mem, 0x0080_1060, &[list, rec, no], call);
                let side = i32::from(mem.read::<i8>(obj.wrapping_add(0x21e))).cast_unsigned();
                let k: u32 = mem.read(addr(mem, 0x00b5_01c0));
                let slot = u32::from(mem.read::<u8>(rec.wrapping_add(0x18)));
                go(
                    mem,
                    0x0080_62c0,
                    &[
                        obj,
                        slot,
                        crate::x87::load(k),
                        u.wrapping_add(0x30),
                        u.wrapping_add(0x90),
                        side,
                    ],
                    call,
                );
                let e: u32 = mem.read(u.wrapping_add(0xbc));
                let m = obj.wrapping_add(0x40);
                go(mem, 0x007e_a6d0, &[m], call);
                let part = go(mem, 0x0080_1f00, &[obj, rec], call);
                let w0 = obj.wrapping_add(0xc0);
                go(mem, 0x007e_83b0, &[w0, part, m], call);
                let no = w(mem, cmd, 4);
                let u2 = go(mem, 0x0080_1060, &[list, rec, no], call);
                let flag = sw(mem, obj, 0x212);
                let b: u32 = mem.read(rec.wrapping_add(0xbc));
                go(mem, 0x00a7_2380, &[b, e, rec, u2, obj, w0, flag], call);
                cmd.wrapping_add(6)
            }
            0x8000 => {
                go(mem, 0x0082_cbe0, &[1], call);
                cmd.wrapping_add(2)
            }
            _ => cmd,
        }
    }

    /// `obj[0..4]` = the emulated r0 (`0x00c8f508`), `obj +4` = `rec
    /// +0x34`, and `obj + 0x80` = the part matrix at `obj`.
    fn r0_matrix<M: Mem>(
        mem: &mut M,
        obj: u32,
        rec: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let r0: [u32; 4] = mem.read(addr(mem, 0x00c8_f508));
        mem.write(obj, r0);
        let y = crate::x87::load_f32(mem.read(rec.wrapping_add(0x34)));
        mem.write(obj.wrapping_add(4), y);
        part_matrix(mem, obj, rec, obj.wrapping_add(0x80), obj, call);
    }

    /// `opp_0437_cmd_a6dbe0(obj, cmd)` (`0x00a6dbe0`), the `opus_0437_*`
    /// effect's command on the part `rec` (`obj +0x220`), its oef2
    /// (`obj +0x360[rec +0x1e]`) and the unit list `obj + 0x300`:
    /// - 0: a buffer of `n` = `i16 cmd[2]` 32-byte entries plus 0x10 into
    ///   `rec +0xbc`, `0x00a6e540(buffer, n)`; `cmd + 4`.
    /// - 0x1000: `op_xyz_scale(obj, rec + 0x30, 0x00b5895c)`,
    ///   `0x00a6e5d0(rec +0xbc, rec + 0x90, obj)`; `cmd + 2`.
    /// - 0x2000: `op_m_copy(obj, part matrix)`, `opus_0437_dot_draw(rec
    ///   +0xbc, obj)`; `cmd + 2`.
    /// - 0x4000: the oef2 entry `i16 cmd[2]`, a buffer of `((0x00a6a2b0
    ///   (entry) + 48 * k) * 2 + 0x3f) & !0xf` bytes (`k` = `i16 cmd[4]`)
    ///   into `rec +0xbc`, `obj` = r0 and `obj + 0x80` its part matrix,
    ///   `0x00a6e1b0(buffer, entry, obj + 0x80, k)`; `cmd + 6`.
    /// - 0x5000: on `d` = `rec +0xbc`, the units `u16 cmd[6]` and `cmd[2]`:
    ///   the r0 matrix as for 0x6000, the depth `d +0x24 - (d +0x24 - d
    ///   +0x14) * unit +0x44 / 256`, `0x00a6ed40(entry cmd[4], d, unit6
    ///   +0xbc, obj + 0x80, depth, 0x1f80000)`, `0x00a6e260(d)`, and the
    ///   character `g_EtChrHandles[rec +0x18]` into `0x022fb3dc`; `cmd + 8`.
    /// - 0x6000: `obj + 0x80` = the r0 matrix (`obj +4` = `rec +0x34`),
    ///   `obj + 0x100` cleared (0x70 bytes) and given the bone set
    ///   (`0x0080e160`, `0x0080e900` with `cmd + 4`) and three local
    ///   matrices; `obj + 0x40` = `obj +0x2c0` times the part's matrix
    ///   (also into `0x02332ea4`); for each bone: its matrix's translation
    ///   times 1000, `L` = `obj + 0x80` times it, back / 1000, `obj + 0xc0`
    ///   = `obj + 0x40` times `L`, and `opus_0437_draw(bone, obj + 0x100,
    ///   rec, unit u16 cmd[2], rec +0xbc, previous, i)`; `cmd + 0xe`.
    /// - else `cmd`.
    ///
    /// The locals are below `frame` (the original's frame pointer; down
    /// to `frame - 0xc4`), which the caller provides.
    #[allow(clippy::too_many_lines)]
    pub fn opp_0437_cmd_a6dbe0<M: Mem>(
        mem: &mut M,
        obj: u32,
        cmd: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use crate::x87::{MUL, Reg, SUB};
        let div: crate::x87::Op = |a, b| a / b;
        let (kind, rec) = kind_rec(mem, obj);
        let g = u32::from(mem.read::<u8>(rec.wrapping_add(0x1e)));
        let oef: u32 = mem.read(obj.wrapping_add(0x360).wrapping_add(4 * g));
        let list = obj.wrapping_add(0x300);
        let heap = |mem: &M| mem.read::<u32>(obj.wrapping_add(0x3a8));
        match kind {
            0 => {
                let n = sw(mem, cmd, 2);
                let h = heap(mem);
                let b = go(
                    mem,
                    0x007f_f780,
                    &[h, n.wrapping_shl(5).wrapping_add(0x10)],
                    call,
                );
                mem.write(rec.wrapping_add(0xbc), b);
                go(mem, 0x00a6_e540, &[b, n], call);
                cmd.wrapping_add(4)
            }
            0x1000 => {
                let k: u32 = mem.read(addr(mem, 0x00b5_895c));
                go(
                    mem,
                    0x007e_a850,
                    &[obj, rec.wrapping_add(0x30), crate::x87::load(k)],
                    call,
                );
                let b: u32 = mem.read(rec.wrapping_add(0xbc));
                go(mem, 0x00a6_e5d0, &[b, rec.wrapping_add(0x90), obj], call);
                cmd.wrapping_add(2)
            }
            0x2000 => {
                let m = go(mem, 0x0080_1f00, &[obj, rec], call);
                go(mem, 0x007e_8010, &[obj, m], call);
                let b: u32 = mem.read(rec.wrapping_add(0xbc));
                go(mem, 0x00a6_e2e0, &[b, obj], call);
                cmd.wrapping_add(2)
            }
            0x4000 => {
                let no = sw(mem, cmd, 2);
                let e = go(mem, 0x007e_3810, &[oef, 4, no], call);
                let k = sw(mem, cmd, 4);
                let sz = go(mem, 0x00a6_a2b0, &[e], call);
                let size = sz
                    .wrapping_add(k.wrapping_mul(48))
                    .wrapping_mul(2)
                    .wrapping_add(0x3f)
                    & !0xf;
                let h = heap(mem);
                let b = go(mem, 0x007f_f780, &[h, size], call);
                mem.write(rec.wrapping_add(0xbc), b);
                let r0: [u32; 4] = mem.read(addr(mem, 0x00c8_f508));
                mem.write(obj, r0);
                part_matrix(mem, obj, rec, obj.wrapping_add(0x80), obj, call);
                go(mem, 0x00a6_e1b0, &[b, e, obj.wrapping_add(0x80), k], call);
                cmd.wrapping_add(6)
            }
            0x5000 => {
                let d: u32 = mem.read(rec.wrapping_add(0xbc));
                let no = w(mem, cmd, 6);
                let u6 = go(mem, 0x0080_1060, &[list, rec, no], call);
                let b6: u32 = mem.read(u6.wrapping_add(0xbc));
                let no = w(mem, cmd, 2);
                let u = go(mem, 0x0080_1060, &[list, rec, no], call);
                r0_matrix(mem, obj, rec, call);
                let far = Reg::load_f32(mem.read(d.wrapping_add(0x24)));
                let x = far
                    .reg(SUB, Reg::load_f32(mem.read(d.wrapping_add(0x14))))
                    .store();
                let t = Reg::load_f32(x)
                    .mem(MUL, mem.read(u.wrapping_add(0x44)))
                    .constant(MUL, f64_at(mem, 0x00b5_0198));
                let depth = far.reg(SUB, t).store();
                let no = sw(mem, cmd, 4);
                let e = go(mem, 0x007e_3810, &[oef, 4, no], call);
                go(
                    mem,
                    0x00a6_ed40,
                    &[
                        e,
                        d,
                        b6,
                        obj.wrapping_add(0x80),
                        depth.to_bits(),
                        0x01f8_0000,
                    ],
                    call,
                );
                go(mem, 0x00a6_e260, &[d], call);
                let slot = u32::from(mem.read::<u8>(rec.wrapping_add(0x18)));
                let chr: u32 = mem.read(addr(mem, 0x012a_8100).wrapping_add(4 * slot));
                mem.write(addr(mem, 0x022f_b3dc), chr);
                cmd.wrapping_add(8)
            }
            0x6000 => {
                r0_matrix(mem, obj, rec, call);
                let bones = obj.wrapping_add(0x100);
                go(mem, 0x0094_964c, &[bones, 0, 0x70], call);
                let n = sw(mem, cmd, 4);
                let set = go(mem, 0x0080_e160, &[bones, obj, list, rec, n], call);
                mem.write(bones.wrapping_add(0x28), frame.wrapping_sub(0xc4));
                mem.write(bones.wrapping_add(0x24), frame.wrapping_sub(0x84));
                let l = frame.wrapping_sub(0x44);
                mem.write(bones.wrapping_add(0x2c), l);
                go(
                    mem,
                    0x0080_e900,
                    &[bones, obj, list, rec, cmd.wrapping_add(4)],
                    call,
                );
                let m = go(mem, 0x0080_1f00, &[obj, rec], call);
                mem.write(addr(mem, 0x0233_2ea4), m);
                let base = obj.wrapping_add(0x40);
                go(mem, 0x007e_83b0, &[base, obj.wrapping_add(0x2c0), m], call);
                let no = w(mem, cmd, 2);
                let u = go(mem, 0x0080_1060, &[list, rec, no], call);
                let d: u32 = mem.read(rec.wrapping_add(0xbc));
                let thousand = f64_at(mem, 0x00b0_dba0);
                let mut acc = 0u32;
                let mut i = 0u32;
                let mut p = set.wrapping_add(0x10);
                while i < u32::from(mem.read::<u16>(set.wrapping_add(4))) {
                    let id = |mem: &M| mem.read::<u32>(p.wrapping_add(4));
                    // the translation scaled by 1000, the next id read
                    // before each result is stored
                    let mut next = id(mem);
                    for (op, last) in [(MUL, false), (div, true)] {
                        let mut mt = go(mem, 0x00a4_4650, &[next], call);
                        for at in [0x30u32, 0x34, 0x38] {
                            let v = Reg::load_f32(mem.read(mt.wrapping_add(at)))
                                .constant(op, thousand)
                                .store();
                            if at != 0x38 || !last {
                                next = id(mem);
                            }
                            mem.write(mt.wrapping_add(at), v);
                            if at != 0x38 {
                                mt = go(mem, 0x00a4_4650, &[next], call);
                            }
                        }
                        if !last {
                            let mt = go(mem, 0x00a4_4650, &[next], call);
                            go(mem, 0x007e_83b0, &[l, obj.wrapping_add(0x80), mt], call);
                            next = id(mem);
                        }
                    }
                    go(mem, 0x007e_83b0, &[obj.wrapping_add(0xc0), base, l], call);
                    let b: u32 = mem.read(p);
                    let x = go(mem, 0x00a4_4650, &[b], call);
                    acc = go(mem, 0x00a6_e650, &[x, bones, rec, u, d, acc, i], call);
                    i += 1;
                    p = p.wrapping_add(0x10);
                }
                cmd.wrapping_add(0xe)
            }
            _ => cmd,
        }
    }
}
