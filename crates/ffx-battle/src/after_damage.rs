//! What a hit leaves behind (`docs/rewrite-plan.md`, M2): the pieces of
//! `AfterDamageProcess` (`0x0078f060`), which `MsSetDamage` runs on each
//! target once its damage is set: MP and CTB lost, revivals, the steal
//! effects.
//!
//! The effects are vfx's (`op_et_eff`): they go to the game's code through
//! `s.call`.

/// The game's memory, through [`ffx_bridge::Mem`].
#[allow(clippy::many_single_char_names)] // indices, actors, amounts
pub mod bridge {
    use ffx_bridge::{IMAGE_BASE, Mem};
    use ffx_kernel::bridge::ms_check_range;

    use crate::actor::bridge::ms_get_chr;
    use crate::atel::bridge::Script;
    use crate::exec::bridge::Call;
    use crate::target::bridge::{Dm, Host};

    fn word(mem: &impl Mem, p: u32, o: u32) -> i32 {
        mem.read(p.wrapping_add(o))
    }

    fn code(mem: &impl Mem, sym: u32) -> u32 {
        sym.wrapping_sub(IMAGE_BASE).wrapping_add(mem.image_base())
    }

    /// `MsSubMP(i, actor, n, other, x, flags, style)` (`0x0078e3b0`): actor
    /// `i` loses `n` MP (within 0 and its maximum `+0x598`; none when
    /// `DebugCanTakeDamage` says so), its number (kind 1) shown with
    /// `flags` bit 1.
    #[allow(clippy::too_many_arguments)]
    pub fn ms_sub_mp(
        mem: &mut impl Mem,
        i: u32,
        actor: u32,
        n: i32,
        other: i32,
        x: u32,
        flags: u32,
        style: u32,
    ) {
        if flags & 2 != 0 {
            crate::input::bridge::ms_number_regist(mem, i, 1, n, other, x, flags, style);
        }
        if crate::damage::bridge::debug_can_take_damage(mem, i) != 0 {
            let v = word(mem, actor, 0x5d4).wrapping_sub(n);
            let max = word(mem, actor, 0x598);
            mem.write(actor.wrapping_add(0x5d4), ms_check_range(v, 0, max));
        }
    }

    /// `MsSubCTB(i, actor, n, x, flags, style)` (`0x0078e1e0`): actor `i`'s
    /// wait (`+0x65c`) grows by `n` (within 0 and 0xff), its number (kind
    /// 2) shown with `flags` bit 2.
    pub fn ms_sub_ctb(
        mem: &mut impl Mem,
        i: u32,
        actor: u32,
        n: i32,
        x: u32,
        flags: u32,
        style: u32,
    ) {
        if flags & 4 != 0 {
            crate::input::bridge::ms_number_regist(mem, i, 2, n, n, x, flags, style);
        }
        let w = i32::from(mem.read::<u8>(actor.wrapping_add(0x65c)));
        let v = ms_check_range(w.wrapping_add(n), 0, 0xff);
        mem.write(actor.wrapping_add(0x65c), v.to_le_bytes()[0]);
    }

    /// `MsAliveProcess(i, actor)` (`0x00789160`): Life on a KO'd actor
    /// (`+0x606` bit 0 still set: its HP stays 0, 1); else it comes back
    /// ([`crate::chr_damage::bridge::ms_relife`], full) with its voice 10
    /// (`MsSetVoiceCheck(i, 10, 1)`): 0.
    pub fn ms_alive_process<M: Mem>(mem: &mut M, s: &mut Script<'_, M>, i: u32, actor: u32) -> u32 {
        if mem.read::<u8>(actor.wrapping_add(0x606)) & 1 != 0 {
            mem.write(actor.wrapping_add(0x5d0), 0u32);
            return 1;
        }
        crate::chr_damage::bridge::ms_relife(mem, &mut *s.call, i, actor, 1);
        crate::atel::bridge::ms_set_voice_check(mem, s, i, 10, 1, 0, 0);
        0
    }

    /// `MsAutoRelifeProcess(i, actor, by, helper)` (`0x0078d8d0`): when
    /// `helper` may act (`MsAutoCommandStatusCheck(helper, 1, 1, 1)`):
    /// `actor`'s `+0x6dd` from `+0x446` being 1, and `by`'s command
    /// `helper +0x6c6` on `i` inserted after `i`'s
    /// ([`crate::death::bridge::insert_one`], priority 0): 1; else 0.
    /// `helper +0x6dc` cleared either way.
    #[allow(clippy::too_many_arguments)]
    pub fn ms_auto_relife_command<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        i: u32,
        actor: u32,
        by: u32,
        helper: u32,
    ) -> u32 {
        let r = if crate::actor::bridge::ms_auto_command_status_check(mem, helper, 1, 1, 1) != 0 {
            let v = mem.read::<u8>(actor.wrapping_add(0x446)) == 1;
            mem.write(actor.wrapping_add(0x6dd), u8::from(v));
            let id = u32::from(mem.read::<u16>(helper.wrapping_add(0x6c6)));
            crate::death::bridge::insert_one(mem, host, s, by & 0xff, i, i, id | 0x00ff_0000, 0);
            1
        } else {
            0
        };
        mem.write(helper.wrapping_add(0x6dc), 0u8);
        r
    }

    /// `0x0079ebe0(i, a, b)` (`MsSetChrEffectNext_0079ebe0`): effect `b`
    /// of group `a` (`op_et_eff` by the battle's owner, through the game)
    /// with actor `i`'s next ID (`MsGetEffID(+0xe2a, 0xe, i)`, `+0xe2a`
    /// counted on); once it started (positive) `COMMAND_HOLD2` counts it:
    /// 1, else 0.
    pub fn ms_set_chr_effect_next<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        i: u32,
        a: u32,
        b: u32,
    ) -> u32 {
        let p = ms_get_chr(mem, i);
        let n: u16 = mem.read(p.wrapping_add(0xe2a));
        let id = crate::util::bridge::ms_get_eff_id(u32::from(n), 0xe, i);
        mem.write(p.wrapping_add(0xe2a), n.wrapping_add(1));
        let owner = crate::status::bridge::EFFECT_OWNER.get(mem);
        let f = code(mem, crate::death::bridge::OP_ET_EFF);
        if call(mem, f, &[owner, a, b, id]).cast_signed() > 0 {
            use crate::chr_damage::bridge::COMMAND_HOLD2;
            COMMAND_HOLD2.set(mem, COMMAND_HOLD2.get(mem).wrapping_add(1));
            1
        } else {
            0
        }
    }

    /// `MsSetStealEffect(i, got)` (`0x0079ed00`): the steal effect, 10 when
    /// something was stolen, else 9 ([`ms_set_chr_effect_next`]).
    pub fn ms_set_steal_effect<M: Mem>(mem: &mut M, call: Call<'_, M>, i: u32, got: u32) {
        ms_set_chr_effect_next(mem, call, i, 0, 9 + u32::from(got != 0));
    }

    /// `MsSetStealGillEffect(i, got)` (`0x0079ed20`): the gil steal effect,
    /// 0x4d or 0x4c.
    pub fn ms_set_steal_gill_effect<M: Mem>(mem: &mut M, call: Call<'_, M>, i: u32, got: u32) {
        ms_set_chr_effect_next(mem, call, i, 0, 0x4c + u32::from(got != 0));
    }

    /// Set: no 3D sound effects start (`MsSEP3Dregist`).
    pub const SEP_3D_OFF: ffx_bridge::Global<u8> = ffx_bridge::Global::new(0x0112_ca02);
    /// The 3D sound effects' 16 nodes of 0x20 bytes (the list is
    /// `minor::SEP_3D`).
    pub const SEP_3D_NODES: u32 = 0x0113_61b0;
    /// The sound `MsRegSEplay` played last (bit 31 set).
    pub const LAST_SE: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x0112_c8e4);
    /// The emulated EE register a sound position passes through.
    pub const EE_GPR_MOVE: ffx_bridge::Global<[u32; 4]> = ffx_bridge::Global::new(0x00c8_f588);
    /// `op_3d_vol_pan_get` (vfx) and `SndSepPlay` (sound).
    pub const OP_3D_VOL_PAN_GET: u32 = 0x007f_fa90;
    pub const SND_SEP_PLAY: u32 = 0x0088_6e50;

    /// `0x007a0bb0(kind, i, vec)` (`MsSEP3DPos`, name ours): where a sound
    /// of actor `i` sits by `kind` (bit 8 ignored): 1 its position, 2 its
    /// top, 3 its centre, 4 its bottom; else `vec` is left alone.
    pub fn ms_sep_3d_pos<M: Mem>(
        mem: &mut M,
        trig: &dyn Fn(crate::pos::bridge::Trig, u32) -> u32,
        kind: u32,
        i: u32,
        vec: u32,
    ) {
        use crate::target::bridge as t;
        match kind & !0x100 {
            1 => {
                crate::pos::bridge::ms_get_chr_pos(mem, trig, i, vec);
            }
            2 => {
                t::ms_get_chr_top_pos(mem, trig, i, vec);
            }
            3 => {
                t::ms_get_chr_center_pos(mem, trig, i, vec);
            }
            4 => {
                t::ms_get_chr_bottom_pos(mem, trig, i, vec);
            }
            _ => {}
        }
    }

    /// `MsSEP3Dregist(kind, i, se, once, pos)` (`0x007a0a30`): sound `se`
    /// played where actor `i` is (pan and volume from its position:
    /// `op_3d_vol_pan_get` with volume 0x100; centred 0x3f/0x7f for actor
    /// 0xff) through `SndSepPlay`. Its node is the first already playing
    /// `se` in the list ([`crate::minor::bridge::SEP_3D`]; with `once`
    /// nothing more happens and the result is 1), else the first free of
    /// the 16 (`+0` clear; with none the result is 1), linked at the
    /// list's end (the walk breaks a node linked to itself). The node
    /// keeps `+0` 1, `+3` 1, `+2` the actor, `+8` the sound, `+4` cleared,
    /// `+1` the kind's low byte with bit 8, and the position at `+0x10`:
    /// `pos` itself for kind 0 (through [`EE_GPR_MOVE`]), else
    /// [`ms_sep_3d_pos`]. With [`SEP_3D_OFF`] nothing. 0.
    #[allow(clippy::too_many_arguments)]
    pub fn ms_sep_3d_regist<M: Mem>(
        mem: &mut M,
        trig: &dyn Fn(crate::pos::bridge::Trig, u32) -> u32,
        call: Call<'_, M>,
        kind: u32,
        i: u32,
        se: u32,
        once: u32,
        pos: u32,
    ) -> u32 {
        if SEP_3D_OFF.get(mem) != 0 {
            return 0;
        }
        let head = crate::minor::bridge::SEP_3D.ptr(mem).addr;
        let mut link = head;
        let mut found = 0u32;
        let mut node: u32 = mem.read(head);
        if node != 0 {
            loop {
                if mem.read::<u32>(node.wrapping_add(8)) == se && found == 0 {
                    found = node;
                }
                link = node.wrapping_add(0xc);
                node = mem.read(link);
                if node == 0 {
                    break;
                }
                if node == mem.read::<u32>(node.wrapping_add(0xc)) {
                    mem.write(node.wrapping_add(0xc), 0u32);
                }
            }
        }
        let n = if found != 0 {
            if once != 0 {
                return 1;
            }
            found
        } else {
            let base = code(mem, SEP_3D_NODES);
            let Some(k) = (0..16).find(|&k| mem.read::<u8>(base + 0x20 * k) == 0) else {
                return 1;
            };
            let n = base + 0x20 * k;
            mem.write(n + 0xc, 0u32);
            mem.write(link, n);
            n
        };
        let (mut pan, mut vol) = (0x3f, 0x7f);
        if i != 0xff {
            mem.write(n, 1u8);
            mem.write(n + 3, 1u8);
            mem.write(n + 2, i.to_le_bytes()[0]);
            mem.write(n + 8, se);
            mem.write(n + 4, 0u32);
            if kind == 0 && pos != 0 {
                let v: [u32; 4] = mem.read(pos);
                EE_GPR_MOVE.set(mem, v);
                mem.write(n + 0x10, v);
            } else {
                ms_sep_3d_pos(mem, trig, kind, i, n + 0x10);
            }
            let low = if kind & 0x100 != 0 {
                kind.to_le_bytes()[0]
            } else {
                0
            };
            mem.write(n + 1, low);
            // the volume goes to the node's +4 (game memory, the
            // original's stack local), which is cleared again
            let f = code(mem, OP_3D_VOL_PAN_GET);
            pan = call(mem, f, &[n + 0x10, 0x100, n + 4]);
            vol = mem.read::<u32>(n + 4);
            mem.write(n + 4, 0u32);
        }
        let f = code(mem, SND_SEP_PLAY);
        call(mem, f, &[se, pan, vol]);
        0
    }

    /// `MsRegSEplay(i, se)` (`0x007a0100`): sound `se` (bit 31 set) for
    /// actor `i & 0xff` unless it is the one played last ([`LAST_SE`]):
    /// [`ms_sep_3d_regist`] kind 3 (its centre); the sound, else 0.
    pub fn ms_reg_se_play<M: Mem>(
        mem: &mut M,
        trig: &dyn Fn(crate::pos::bridge::Trig, u32) -> u32,
        call: Call<'_, M>,
        i: u32,
        se: u32,
    ) -> u32 {
        if se == 0 {
            return 0;
        }
        let se = se | 0x8000_0000;
        if LAST_SE.get(mem) == se {
            return 0;
        }
        ms_sep_3d_regist(mem, trig, call, 3, i & 0xff, se, 0, 0);
        LAST_SE.set(mem, se);
        se
    }

    /// `MsRegSEplay2(i, se)` (`0x007a0140`): sound `se` (bit 31 set) for
    /// actor `i & 0xff`, once (not again while it plays); the sound, else
    /// 0.
    pub fn ms_reg_se_play2<M: Mem>(
        mem: &mut M,
        trig: &dyn Fn(crate::pos::bridge::Trig, u32) -> u32,
        call: Call<'_, M>,
        i: u32,
        se: u32,
    ) -> u32 {
        if se == 0 {
            return 0;
        }
        ms_sep_3d_regist(mem, trig, call, 3, i & 0xff, se | 0x8000_0000, 1, 0);
        se | 0x8000_0000
    }

    /// `MsStoneProcess(i, actor)` (`0x0078e150`): for an actor with a
    /// character: petrified (`+0x606` bit 2) it gets sound 0x59, weight -1,
    /// Provoke off and, without stone HP (`+0x5d0` below 1), `+0x616` bit
    /// 8: 1. Else it comes back to normal: its eternal revival, its own
    /// weight (`+0x4dc`) and its automatic statuses redone: 0.
    pub fn ms_stone_process<M: Mem>(
        mem: &mut M,
        trig: &dyn Fn(crate::pos::bridge::Trig, u32) -> u32,
        call: Call<'_, M>,
        i: u32,
        actor: u32,
    ) -> u32 {
        if mem.read::<u32>(actor) == 0 {
            return 0;
        }
        if mem.read::<u8>(actor.wrapping_add(0x606)) & 4 != 0 {
            ms_reg_se_play(mem, trig, call, i, 0x59);
            crate::body::bridge::ms_set_chr_weight2(mem, actor, 0xbf80_0000);
            crate::minor::bridge::ms_prov_off(mem, i);
            if word(mem, actor, 0x5d0) < 1 {
                let st: u16 = mem.read(actor.wrapping_add(0x616));
                mem.write(actor.wrapping_add(0x616), st | 0x100);
            }
            return 1;
        }
        crate::actor::bridge::ms_set_eternal_relife(mem, actor);
        let w: u32 = mem.read(actor.wrapping_add(0x4dc));
        crate::body::bridge::ms_set_chr_weight2(mem, actor, w);
        crate::actor::bridge::ms_reset_auto_status(mem, actor);
        crate::actor::bridge::ms_set_auto_status(mem, actor);
        0
    }

    /// `MsStatusDefenseEffect(i, j, bits)` (`0x0079ee20`): the effects of
    /// the statuses actor `j` warded off (`bits`: 0x20 0xd, 0x40 0xe, 8
    /// 0x15, 0x10 0x4b, 0x200-0x4000 2-7, 0x8000 0x49), each through
    /// [`ms_set_chr_effect_next`]; how many.
    pub fn ms_status_defense_effect<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        i: u32,
        j: u32,
        bits: u32,
    ) -> u32 {
        let _ = i;
        let mut n = 0;
        for (bit, e) in [
            (0x20, 0xd),
            (0x40, 0xe),
            (8, 0x15),
            (0x10, 0x4b),
            (0x200, 2),
            (0x400, 3),
            (0x800, 4),
            (0x1000, 5),
            (0x2000, 6),
            (0x4000, 7),
            (0x8000, 0x49),
        ] {
            if bits & bit != 0 {
                ms_set_chr_effect_next(mem, &mut *call, j, 0, e);
                n += 1;
            }
        }
        n
    }
}
