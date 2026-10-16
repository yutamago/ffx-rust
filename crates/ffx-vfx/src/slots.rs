//! Small functions of the spell overlays' import table (`g_MagicImports`,
//! `docs/formats/magic.md`) that the Ghidra dump used to hide: flag
//! setters, getters, and thin wrappers over the battle, sound, scene
//! (`TO*`) and map-effect (`rcef*`) modules' functions, which are called
//! through a callback.

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
}
