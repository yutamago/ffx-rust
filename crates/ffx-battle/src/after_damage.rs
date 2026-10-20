//! What a hit leaves behind (`docs/rewrite-plan.md`, M2):
//! `AfterDamageProcess` (`0x0078f060`), which `MsSetDamage` runs on each
//! target once its damage is set, and its pieces: MP and CTB lost,
//! revivals, the steal effects, the overdrive modes, Auto-Med and
//! Auto-Potion.
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

    /// The actors' sound tables (`u16` per sound kind, 0x21 kinds): party
    /// members (below 8), monsters (0x14-0x1b), the others.
    pub const SE_TABLES: [ffx_bridge::Global<[u16; 0x21]>; 3] = [
        ffx_bridge::Global::new(0x00c4_2550),
        ffx_bridge::Global::new(0x00c4_25e0),
        ffx_bridge::Global::new(0x00c4_2598),
    ];
    /// The last sound `MsChrSePlay_0079fd40` started (cleared each battle
    /// frame, `process::FRAME_CLEARED`'s first word).
    pub const LAST_CHR_SE: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x0112_c8e0);

    /// `MsChrSeGet_0079fe80(i, kind)` (`0x0079fe80`, name ours): actor
    /// `i`'s sound for `kind` (0 to 0x20), 0 for none. Kind 6 only while
    /// `+0x40f` is set and `+0xdcc` is not.
    ///
    /// The kind's entry in the actor's table ([`SE_TABLES`]) is a base
    /// (low byte) plus a variant: bit 8 cycles `+0x410` through 0-3 (the
    /// count's C remainder, the next count one more), bit 9 through 0-1,
    /// bit 10 the underwater flag's bit 0, bit 11 8 when underwater. Its
    /// top nibble picks the source: 1 the scene's transferred sounds
    /// (`SEP_TRANS_TABLE`, ten per actor, while `SEP_TRANS_FLAG`), 2 the
    /// number itself with bit 31, any other the sounds of the actor's
    /// model owner (`+0x14`: `+0xeac`, 0x20 of them, while its `+0xf2c`).
    pub fn ms_chr_se_get(mem: &mut impl Mem, i: u32, kind: u32) -> u32 {
        let class = match i.cast_signed() {
            ..8 => 0,
            0x14..0x1c => 1,
            _ => 2,
        };
        let a = ms_get_chr(mem, i);
        if kind == 6 {
            if byte(mem, a, 0x40f) == 0 || byte(mem, a, 0xdcc) != 0 {
                return 0;
            }
        } else if kind.cast_signed() < 0 || kind.cast_signed() >= 0x21 {
            return 0;
        }
        let entry: u16 = mem.read(SE_TABLES[class].ptr(mem).addr + 2 * kind);
        let mut variant = 0i32;
        for (bit, n) in [(0x100, 4), (0x200, 2)] {
            if entry & bit != 0 {
                let count = byte(mem, a, 0x410).cast_signed();
                let r = count % n;
                variant = i32::from(r);
                mem.write(a.wrapping_add(0x410), r.wrapping_add(1));
            }
        }
        let water = crate::state::bridge::UNDERWATER.get(mem);
        if entry & 0x400 != 0 {
            variant = i32::from(water) & 1;
        }
        if entry & 0x800 != 0 && water != 0 {
            variant = 8;
        }
        let [v0, v1, ..] = variant.to_le_bytes();
        let sound =
            u32::from(u16::from(entry.to_le_bytes()[0]).wrapping_add(u16::from_le_bytes([v0, v1])));
        match entry & 0xf000 {
            0x1000 => {
                let on = mem.read::<u8>(crate::minor::bridge::SEP_TRANS_FLAG.ptr(mem).addr) != 0;
                if on && sound < 10 {
                    let table = crate::minor::bridge::SEP_TRANS_TABLE.ptr(mem).addr;
                    mem.read(table.wrapping_add(4 * sound.wrapping_add(i.wrapping_mul(10))))
                } else {
                    0
                }
            }
            0x2000 => sound | 0x8000_0000,
            _ => {
                let owner = ms_get_chr(mem, u32::from(byte(mem, a, 0x14)));
                if byte(mem, owner, 0xf2c) != 0 && sound < 0x20 {
                    mem.read(owner.wrapping_add(0xeac + 4 * sound))
                } else {
                    0
                }
            }
        }
    }

    /// `MsChrSeFade_0079fcf0(i, kind, frames)` (`0x0079fcf0`, name ours):
    /// actor `i & 0xff`'s sound for `kind` ([`ms_chr_se_get`]) fades out
    /// over `frames` (`MsSEP3DfadeOut`): -1, 0 without one.
    pub fn ms_chr_se_fade(mem: &mut impl Mem, i: u32, kind: u32, frames: u32) -> i32 {
        let se = ms_chr_se_get(mem, i & 0xff, kind);
        if se == 0 {
            return 0;
        }
        crate::damage_end::bridge::ms_sep_3d_fade_out(mem, se, frames);
        -1
    }

    /// `MsChrSePlay_0079fd40(i, kind)` (`0x0079fd40`, name ours): actor
    /// `i & 0xff`'s sound for `kind` starts where it stands
    /// (`MsSEP3Dregist` kind 3) unless it is the one this frame already
    /// started ([`LAST_CHR_SE`]); the sound, else 0 (also without actors).
    pub fn ms_chr_se_play<M: Mem>(
        mem: &mut M,
        trig: &dyn Fn(crate::pos::bridge::Trig, u32) -> u32,
        call: Call<'_, M>,
        i: u32,
        kind: u32,
    ) -> u32 {
        if crate::actor::bridge::ms_get_chr_top(mem) == 0 {
            return 0;
        }
        let i = i & 0xff;
        let se = ms_chr_se_get(mem, i, kind);
        if se == 0 || LAST_CHR_SE.get(mem) == se {
            return 0;
        }
        ms_sep_3d_regist(mem, trig, call, 3, i, se, 0, 0);
        LAST_CHR_SE.set(mem, se);
        se
    }

    /// `MsChrSePlay2_0079fda0(i, kind)` (`0x0079fda0`, name ours):
    /// [`ms_chr_se_play`] once (not again while it plays) and without the
    /// frame's check.
    pub fn ms_chr_se_play2<M: Mem>(
        mem: &mut M,
        trig: &dyn Fn(crate::pos::bridge::Trig, u32) -> u32,
        call: Call<'_, M>,
        i: u32,
        kind: u32,
    ) -> u32 {
        if crate::actor::bridge::ms_get_chr_top(mem) == 0 {
            return 0;
        }
        let i = i & 0xff;
        let se = ms_chr_se_get(mem, i, kind);
        if se == 0 {
            return 0;
        }
        ms_sep_3d_regist(mem, trig, call, 3, i, se, 1, 0);
        se
    }

    /// `MsChrSePlayUnhasted_0079fdf0(i, kind)` (`0x0079fdf0`, name
    /// ours): [`ms_chr_se_play`] unless the actor is hasted (`+0x613`).
    pub fn ms_chr_se_play_unhasted<M: Mem>(
        mem: &mut M,
        trig: &dyn Fn(crate::pos::bridge::Trig, u32) -> u32,
        call: Call<'_, M>,
        i: u32,
        kind: u32,
    ) -> u32 {
        let i = i & 0xff;
        let a = ms_get_chr(mem, i);
        if byte(mem, a, 0x613) != 0 {
            return 0;
        }
        ms_chr_se_play(mem, trig, call, i, kind)
    }

    /// `MsRegSEplay3(i, se)` (`0x007a0170`): [`ms_reg_se_play`] unless
    /// the actor is hasted (`+0x613`): 0 then. (As
    /// [`ms_chr_se_play_unhasted`] with `MsRegSEplay`.)
    pub fn ms_reg_se_play3<M: Mem>(
        mem: &mut M,
        trig: &dyn Fn(crate::pos::bridge::Trig, u32) -> u32,
        call: Call<'_, M>,
        i: u32,
        se: u32,
    ) -> u32 {
        let i = i & 0xff;
        let a = ms_get_chr(mem, i);
        if byte(mem, a, 0x613) != 0 {
            return 0;
        }
        ms_reg_se_play(mem, trig, call, i, se)
    }

    /// `MsChrSePlayAlive_0079fe30(i, kind)` (`0x0079fe30`, name ours):
    /// [`ms_chr_se_play`] for a shown actor (`+0x15`) with HP (`+0x40`
    /// not 0.0; a NaN counts).
    pub fn ms_chr_se_play_alive<M: Mem>(
        mem: &mut M,
        trig: &dyn Fn(crate::pos::bridge::Trig, u32) -> u32,
        call: Call<'_, M>,
        i: u32,
        kind: u32,
    ) -> u32 {
        if crate::actor::bridge::ms_get_chr_top(mem) == 0 {
            return 0;
        }
        let i = i & 0xff;
        let a = ms_get_chr(mem, i);
        let hp: u32 = mem.read(a.wrapping_add(0x40));
        // fucompp: a NaN is not 0
        #[allow(clippy::float_cmp)]
        let dead = ffx_bridge::x87::load(hp) == 0.0;
        if byte(mem, a, 0x15) == 0 || dead {
            return 0;
        }
        ms_chr_se_play(mem, trig, call, i, kind)
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

    fn byte(mem: &impl Mem, p: u32, o: u32) -> u8 {
        mem.read(p.wrapping_add(o))
    }

    /// `MsLimitTypeDamageCheck(i, by, j, actor, hp, raw, crit)`
    /// (`0x007b0d50`): the overdrive modes a hit of actor `i` (`by`) on
    /// `j` (`actor`) feeds (`MsLimitTypeExpGet`; the mode in use, `+0x5bb`,
    /// also gains `MsLimitUp`). A monster's hit on the party: mode 0x13
    /// unless it heals (`raw × 18 / max HP + 1`; `raw` the entry's `+0x1c`); with HP lost, the
    /// target's mode 2 (`hp × 30 / max HP + 1`) and mode 1 of everybody
    /// else on the field (`+0xdc8`; `hp × 20 / the target's max HP + 1`):
    /// how many, the target included. The party's hit on a monster with
    /// `crit`: mode 0 (`hp × 10 / +0x6f4 + 1`, at most 16) and 0x13 (`hp ×
    /// 16 / +0x6f4 / 10 + 1`): 1 (also without `crit`). HP healed by
    /// another of the party: mode 3 (the HP it could still take, `× 16 /
    /// max HP + 1`): 1. Else 0.
    #[allow(clippy::too_many_arguments)]
    pub fn ms_limit_type_damage_check(
        mem: &mut impl Mem,
        i: u32,
        by: u32,
        j: u32,
        actor: u32,
        hp: i32,
        raw: i32,
        crit: u32,
    ) -> u32 {
        use crate::actor::bridge::ms_get_ram_chr_monster as monster;
        use crate::exec::bridge::ms_limit_type_exp_get as exp;
        use crate::turn::bridge::ms_limit_up as up;
        let from = monster(i);
        let to = monster(j);
        if byte(mem, actor, 0x5bb) == 0x13 && hp >= 0 && from == 1 && to == 0 {
            let n = raw.cast_unsigned().wrapping_mul(0x12) / max_hp(mem, actor);
            up(mem, j, actor, n.wrapping_add(1).cast_signed());
        }
        if hp > 0 {
            if from == 1 {
                if to != 0 {
                    return 0;
                }
                exp(mem, j, 2, 0);
                let h = hp.cast_unsigned();
                if byte(mem, actor, 0x5bb) == 2 {
                    let n = h.wrapping_mul(0x1e) / max_hp(mem, actor);
                    up(mem, j, actor, n.wrapping_add(1).cast_signed());
                }
                let mut count = 1;
                for u in 0..7 {
                    let a = ms_get_chr(mem, u);
                    if byte(mem, a, 0xdc8) == 0 || u == j {
                        continue;
                    }
                    count += 1;
                    exp(mem, u, 1, 0);
                    if byte(mem, a, 0x5bb) == 1 {
                        let n = h.wrapping_mul(0x14) / max_hp(mem, actor);
                        up(mem, u, a, n.wrapping_add(1).cast_signed());
                    }
                }
                return count;
            }
            if from != 0 || to != 1 {
                return 0;
            }
            if crit == 0 {
                return 1;
            }
            exp(mem, i, 0, 0);
            let own = word(mem, by, 0x6f4);
            if byte(mem, by, 0x5bb) == 0 {
                let n = hp
                    .wrapping_mul(10)
                    .wrapping_div(own)
                    .wrapping_add(1)
                    .min(0x10);
                up(mem, i, by, n);
            }
            if byte(mem, by, 0x5bb) == 0x13 {
                let own = word(mem, by, 0x6f4);
                let n = hp.wrapping_shl(4).wrapping_div(own) / 10;
                up(mem, i, by, n.wrapping_add(1));
            }
            return 1;
        }
        if hp >= 0 || from != 0 || to != 0 || i == j {
            return 0;
        }
        exp(mem, i, 3, 0);
        if byte(mem, by, 0x5bb) == 3 {
            let m = max_hp(mem, actor);
            let room = m
                .wrapping_sub(mem.read(actor.wrapping_add(0x5d0)))
                .cast_signed();
            let v = hp.wrapping_neg().min(room);
            let n = v.cast_unsigned().wrapping_shl(4) / m;
            up(mem, i, by, n.wrapping_add(1).cast_signed());
        }
        1
    }

    fn max_hp(mem: &impl Mem, actor: u32) -> u32 {
        mem.read(actor.wrapping_add(0x594))
    }

    /// `MsLimitTypeStatusCheck(i, by, j, actor, n, bits)` (`0x007b12c0`):
    /// the overdrive modes statuses feed (as
    /// [`ms_limit_type_damage_check`]). A monster's on the party: the
    /// target's mode 5 with `n` positive, 6 with `bits` bit 0, 10 with
    /// bit 1 (`MsLimitUp` 16, 16, 10); how many. The party's on a monster
    /// with `n` positive: mode 4 (16), 1. Else 0.
    pub fn ms_limit_type_status_check(
        mem: &mut impl Mem,
        i: u32,
        by: u32,
        j: u32,
        actor: u32,
        n: i32,
        bits: u32,
    ) -> u32 {
        use crate::actor::bridge::ms_get_ram_chr_monster as monster;
        use crate::exec::bridge::ms_limit_type_exp_get as exp;
        use crate::turn::bridge::ms_limit_up as up;
        let from = monster(i);
        let to = monster(j);
        let mut count = 0;
        if from == 1 {
            if to != 0 {
                return 0;
            }
            for (on, mode, gain) in [
                (n > 0, 5, 0x10),
                (bits & 1 != 0, 6, 0x10),
                (bits & 2 != 0, 10, 10),
            ] {
                if on {
                    count += 1;
                    exp(mem, j, mode, 1);
                    if u32::from(byte(mem, actor, 0x5bb)) == mode {
                        up(mem, j, actor, gain);
                    }
                }
            }
        } else if from == 0 && to == 1 && n > 0 {
            exp(mem, i, 4, 0);
            count = 1;
            if byte(mem, by, 0x5bb) == 4 {
                up(mem, i, by, 0x10);
            }
        }
        count
    }

    /// `MsAutoCureProcess(i, actor, by, poison, zombie, dark, silence)`
    /// (`0x007b2510`): Auto-Med. Each status the hit gave (on now, off
    /// before: the flags, and not warded, `+0x62a`/`+0x62c`): Poison
    /// (`+0x606` bit 3), Zombie (bit 1), Darkness (`+0x60a`), Silence
    /// (`+0x609`). Unless a command of kind 0x10 of `i` is queued
    /// (`MsCheckCommandCue2`) or it may not act
    /// (`MsAutoCommandStatusCheck(.., 1, 1, 1)`), `i` uses on itself after
    /// `by`'s ([`crate::death::bridge::ms_auto_insert_command`]) a Remedy
    /// (0x200f, priority 0x15) for two or more, else the first it has of
    /// Antidote (0x200a), Holy Water (0x200e), Eye Drops (0x200c), Echo
    /// Screen (0x200d) (priority 0x11-0x14), else a Remedy. The item, or
    /// 0.
    #[allow(clippy::too_many_arguments)]
    pub fn ms_auto_cure_process<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        i: u32,
        actor: u32,
        by: u32,
        poison: u32,
        zombie: u32,
        dark: u32,
        silence: u32,
    ) -> u32 {
        use crate::save::bridge::ms_get_save_item_num as have;
        if crate::state::bridge::ms_check_command_cue2(mem, i, 0x10) == 0 {
            return 0;
        }
        let st: u16 = mem.read(actor.wrapping_add(0x606));
        let want = [
            st & 8 != 0 && poison == 0 && byte(mem, actor, 0x62a) & 8 == 0,
            st & 2 != 0 && zombie == 0 && byte(mem, actor, 0x62a) & 2 == 0,
            byte(mem, actor, 0x60a) != 0 && dark == 0 && byte(mem, actor, 0x62c) & 4 == 0,
            byte(mem, actor, 0x609) != 0 && silence == 0 && byte(mem, actor, 0x62c) & 2 == 0,
        ];
        let n = want.iter().filter(|&&w| w).count();
        if n == 0 || crate::actor::bridge::ms_auto_command_status_check(mem, actor, 1, 1, 1) == 0 {
            return 0;
        }
        let mut pick = None;
        if n < 2 || have(mem, 0x200f).cast_signed() < 1 {
            for (k, (item, prio)) in [(0x200a, 1), (0x200e, 2), (0x200c, 3), (0x200d, 4)]
                .into_iter()
                .enumerate()
            {
                if want[k] && have(mem, item).cast_signed() > 0 {
                    pick = Some((item, prio));
                    break;
                }
            }
            if pick.is_none() && have(mem, 0x200f).cast_signed() < 1 {
                return 0;
            }
        }
        let (item, k) = pick.unwrap_or((0x200f, 5));
        crate::death::bridge::ms_auto_insert_command(mem, host, s, i, i, by, item, k + 0x10);
        item
    }

    /// `MsAutoPotionProcess(i, actor, by)` (`0x007b2850`): Auto-Potion.
    /// With HP at most half the maximum, no command of kind 0x20 of `i`
    /// queued and free to act, `i` drinks the first it has of Potion,
    /// Hi-Potion, X-Potion (0x2000-0x2002, priority 0x21-0x23) after
    /// `by`'s; the item, or 0.
    pub fn ms_auto_potion_process<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        i: u32,
        actor: u32,
        by: u32,
    ) -> u32 {
        if mem.read::<u32>(actor.wrapping_add(0x5d0)) > max_hp(mem, actor) >> 1
            || crate::state::bridge::ms_check_command_cue2(mem, i, 0x20) == 0
            || crate::actor::bridge::ms_auto_command_status_check(mem, actor, 1, 1, 1) == 0
        {
            return 0;
        }
        for (item, prio) in [(0x2000, 0x21), (0x2001, 0x22), (0x2002, 0x23)] {
            if crate::save::bridge::ms_get_save_item_num(mem, item).cast_signed() > 0 {
                crate::death::bridge::ms_auto_insert_command(mem, host, s, i, i, by, item, prio);
                return item;
            }
        }
        0
    }

    /// `AfterDamageProcess(i, k, j, out, flags)` (`0x0078f060`): what
    /// actor `i`'s hit `k` leaves on `j`, from the hit records `j` keeps
    /// (`+0x774` and `+0xa4c`, 0x2d8 bytes: `+0` entries done, `+1` how
    /// many, `+2` the attacker, `+3` the hit, `+4` transfers, `+5`
    /// progress bits, `+6` the critical flag, `+7`/`+0xc` steal flags with
    /// the item (`+0xa` ID, `+8` count) and gil (`+0x10`), `+0x17` the
    /// actor whose command is learnt, the entries of 0x2c bytes from
    /// `+0x18`). For each record of this attacker and hit: `+0xf5e` the
    /// flags' low 7 bits, `+0xded` the attacker. Unless `flags` bit 3: the
    /// steal messages (4, 8) and, when something came, its effect, sound
    /// 0x41 and, for gil, its payment. Transfers: bit 0 the attacker's
    /// overdrive gauge (`+0x5bc`) moves over (up to `+0x5bd`); bit 1 a
    /// learnt command (Lancet: `j` 3, the command `+6` of actor `+0x17`'s
    /// `+0xf88`, new to the save) with its message 6, a full gauge and,
    /// once every shared command 0x3068-0x306f/0x3070-0x3073 is known,
    /// achievement 0x19. Then the next entry: unless bit 3 its kind's word
    /// (`+1`: 1 kind 3, 2 kind 4), the overdrive modes
    /// ([`ms_limit_type_damage_check`]), HP, MP and CTB lost by the bits
    /// of `+0x18` (HP also from the overkill counter `+0xf60`), the status
    /// modes, the statuses (`+0x14`, `+7`-`+0x13`, `+0x16`), the doubled
    /// HP/MP ([`crate::flow::bridge::ms_limit_status_process`]), Regen's
    /// counter reset when it starts, `+0xdce` petrified, what changes
    /// with KO, stone, blown away and Threaten, Auto-Med, Auto-Potion
    /// (HP lost), the weak pose; the entry counted, result bit 1. Then the
    /// Auto-Life by a helper ([`ms_auto_relife_command`]), the entry
    /// chosen for the motion (unless `flags` bit 1; a kind-1 entry only
    /// once), and unless bit 4 the status effects and the warded ones (`out`
    /// gets `j`). Entries left: result bit 0; else, once something
    /// happened, the target's reaction (`MsActionRequest(j, i, 3, 0, 1,
    /// 0)`, `+0x6dd` cleared) and its battle position back (`flags` bit
    /// 10: the record freed, `+2` 0xff). With none left, the death check
    /// ([`crate::death::bridge::ms_damage_check_death`]); if nobody died,
    /// the chosen entry's motion (`+0`; with `flags` bit 5 kinds 5 and 8
    /// become 0xd-0x10 and 6 0xf/0x10, `brnd(9)`). The result.
    ///
    /// (The original keeps the entry pointer in its `j` argument slot.)
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    pub fn after_damage_process<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        i: u32,
        k: u32,
        j: u32,
        out: u32,
        flags: u32,
    ) -> u32 {
        let mut result = 0u32;
        let mut chosen = 0u32;
        let by = ms_get_chr(mem, i);
        let actor = ms_get_chr(mem, j);
        let other = u32::from(i != j);
        for n in 0..2 {
            let rec = actor.wrapping_add(0x774 + 0x2d8 * n);
            if u32::from(byte(mem, rec, 2)) != i || u32::from(byte(mem, rec, 3)) != k {
                continue;
            }
            mem.write(actor.wrapping_add(0xf5e), (flags & 0x7f).to_le_bytes()[0]);
            mem.write(actor.wrapping_add(0xded), i.to_le_bytes()[0]);
            if flags & 8 == 0 {
                steal_messages(mem, host, s, j, rec);
            }
            transfers(mem, s, j, by, actor, rec);
            if byte(mem, rec, 0) < byte(mem, rec, 1) {
                let e = rec + u32::from(byte(mem, rec, 0)) * 0x2c + 0x18;
                if flags & 8 == 0 {
                    let mut hurt = 0i32;
                    let kind = i32::from(mem.read::<i8>(e + 1)).cast_unsigned();
                    let c = u32::from(byte(mem, e, 2));
                    match kind {
                        1 => {
                            crate::input::bridge::ms_number_regist(mem, j, 3, 0, 0, 1, c, 0x81);
                        }
                        2 => {
                            crate::input::bridge::ms_number_regist(mem, j, 4, 0, 0, 2, c, 0x81);
                        }
                        _ => {}
                    }
                    let crit = u32::from(byte(mem, rec, 6));
                    ms_limit_type_damage_check(
                        mem,
                        i,
                        by,
                        j,
                        actor,
                        word(mem, e, 0x20),
                        word(mem, e, 0x1c),
                        crit,
                    );
                    for slot in 0..3 {
                        let v = word(mem, e, 0x20 + 4 * slot);
                        if u32::from(mem.read::<u16>(e + 0x18)) & (1 << slot) == 0 {
                            continue;
                        }
                        match slot {
                            0 => {
                                let other = word(mem, e, 0x24);
                                crate::input::bridge::ms_sub_hp(
                                    mem, j, actor, v, other, kind, c, 0x81,
                                );
                                let o = word(mem, actor, 0xf60).wrapping_sub(v);
                                mem.write(actor.wrapping_add(0xf60), o);
                                hurt = hurt.wrapping_add(v);
                            }
                            1 => {
                                let other = word(mem, e, 0x20);
                                ms_sub_mp(mem, j, actor, v, other, kind, c, 0x81);
                            }
                            // its `dbgPrintf("CTB DAMAGE ...")` is empty
                            _ => ms_sub_ctb(mem, j, actor, v, kind, c, 0x81),
                        }
                    }
                    let (n4, b3) = (byte(mem, e, 4), byte(mem, e, 3));
                    ms_limit_type_status_check(mem, i, by, j, actor, i32::from(n4), u32::from(b3));
                    let st: u16 = mem.read(actor.wrapping_add(0x606));
                    let dark = u32::from(byte(mem, actor, 0x60a));
                    let silence = u32::from(byte(mem, actor, 0x609));
                    let regen = byte(mem, actor, 0x612);
                    let blown = byte(mem, actor, 0x617) & 1;
                    let v: u16 = mem.read(e + 0x14);
                    mem.write(actor.wrapping_add(0x606), v);
                    let v: [u8; 0xd] = mem.read(e + 7);
                    mem.write(actor.wrapping_add(0x608), v);
                    let v: u16 = mem.read(e + 0x16);
                    mem.write(actor.wrapping_add(0x616), v);
                    let f = i32::from(byte(mem, e, 6));
                    crate::flow::bridge::ms_limit_status_process(mem, actor, f);
                    if byte(mem, actor, 0x612) != 0 && regen == 0 {
                        mem.write(actor.wrapping_add(0x6d2), 0u8);
                    }
                    let now: u16 = mem.read(actor.wrapping_add(0x606));
                    mem.write(actor.wrapping_add(0xdce), u8::from(now & 4 != 0));
                    if (now ^ st) & 1 != 0 {
                        ms_alive_process(mem, s, j, actor);
                    }
                    let now: u16 = mem.read(actor.wrapping_add(0x606));
                    if (now ^ st) & 4 != 0 {
                        ms_stone_process(mem, host.trig, &mut *s.call, j, actor);
                    }
                    if byte(mem, actor, 0x617) & 1 != blown {
                        crate::chr_damage::bridge::ms_blow_process(mem, &mut *s.call, j, actor);
                    }
                    let now: u16 = mem.read(actor.wrapping_add(0x606));
                    if (now ^ st) & 0x800 != 0 {
                        crate::exchange::bridge::ms_threat_process(mem, host, s, j, actor);
                    }
                    or_byte(mem, rec, 5, 1);
                    let auto: u16 = mem.read(actor.wrapping_add(0x6bc));
                    if auto & 0x800 != 0 {
                        let (p, z) = (u32::from(st >> 3 & 1), u32::from(st >> 1 & 1));
                        ms_auto_cure_process(mem, host, s, j, actor, i, p, z, dark, silence);
                    }
                    if hurt > 0 && mem.read::<u16>(actor.wrapping_add(0x6bc)) & 0x400 != 0 {
                        ms_auto_potion_process(mem, host, s, j, actor, i);
                    }
                    crate::input::bridge::ms_set_chr_weak(mem, j, 0xffff_ffff);
                    result |= 2;
                    mem.write(rec, byte(mem, rec, 0).wrapping_add(1));
                }
                if byte(mem, actor, 0x6dc) != 0 {
                    ms_auto_relife_command(mem, host, s, i, by, j, actor);
                }
                if flags & 2 == 0 && (byte(mem, e, 0) != 1 || byte(mem, rec, 5) & 4 == 0) {
                    or_byte(mem, rec, 5, 4);
                    chosen = e;
                }
                if flags & 0x10 == 0 {
                    crate::status::bridge::ms_status_effect_check(
                        mem,
                        &mut *s.call,
                        &mut *s.off,
                        j,
                    );
                    let bits = u32::from(mem.read::<u16>(e + 0x18));
                    if ms_status_defense_effect(mem, &mut *s.call, i, j, bits) != 0 {
                        mem.write(out, j);
                    }
                }
            }
            if byte(mem, rec, 0) < byte(mem, rec, 1) {
                result |= 1;
                continue;
            }
            let p = byte(mem, rec, 5);
            if p & 1 != 0 && p & 2 == 0 {
                mem.write(actor.wrapping_add(0x6dd), 0u8);
                or_byte(mem, rec, 5, 2);
                crate::action::bridge::ms_action_request(mem, host, s, j, i, 3, 0, 1, 0);
            }
            if flags & 0x400 == 0 {
                crate::actor::bridge::ms_pop_btl_pos(mem, actor);
            } else {
                mem.write(rec + 2, 0xffu8);
            }
        }
        if result & 1 == 0
            && crate::death::bridge::ms_damage_check_death(mem, host, s, i, j, 0, other) != 0
        {
            return result;
        }
        if chosen == 0 {
            return result;
        }
        let mut kind = i32::from(mem.read::<i8>(chosen)).cast_unsigned();
        if flags & 0x20 != 0 {
            match kind {
                5 | 8 => kind = (crate::rng::bridge::brnd(mem, 9) & 3) + 0xd,
                6 => kind = (crate::rng::bridge::brnd(mem, 9) & 1) + 0xf,
                _ => {}
            }
        }
        crate::status::bridge::ms_damage_set_motion(mem, host, s, j, kind, other);
        result
    }

    fn or_byte(mem: &mut impl Mem, p: u32, o: u32, bits: u8) {
        let v = byte(mem, p, o);
        mem.write(p.wrapping_add(o), v | bits);
    }

    /// The steal messages of a hit record (`AfterDamageProcess`): an item
    /// (`+7`, ID `+0xa`, count `+8`: message 4) and gil (`+0xc`, amount
    /// `+0x10`: message 8, paid), each with its effect and sound 0x41 when
    /// something came; `+5` bit 0.
    fn steal_messages<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        j: u32,
        rec: u32,
    ) {
        use crate::death::bridge::TO_BTL_CLOSE_SIGN_WINDOW;
        use crate::state::bridge::{ms_menu_close_title_window, ms_message_cue_regist};
        let got = u32::from(byte(mem, rec, 1));
        if byte(mem, rec, 7) != 0 {
            let call = &mut *s.call;
            ms_menu_close_title_window(mem, 0, |m| {
                let f = code(m, TO_BTL_CLOSE_SIGN_WINDOW);
                call(m, f, &[]);
            });
            mem.write(rec + 7, 0u8);
            let n: i16 = mem.read(rec + 8);
            let item = u32::from(mem.read::<u16>(rec + 0xa));
            ms_message_cue_regist(
                mem,
                4,
                item,
                i32::from(n).cast_unsigned(),
                [0x1b, 0x23],
                |m| {
                    let f = code(m, TO_BTL_CLOSE_SIGN_WINDOW);
                    call(m, f, &[]);
                },
            );
            if mem.read::<i16>(rec + 8) > 0 {
                ms_set_steal_effect(mem, &mut *s.call, j, got);
                ms_reg_se_play2(mem, host.trig, &mut *s.call, j, 0x41);
            }
            or_byte(mem, rec, 5, 1);
        }
        if byte(mem, rec, 0xc) != 0 {
            let call = &mut *s.call;
            ms_menu_close_title_window(mem, 0, |m| {
                let f = code(m, TO_BTL_CLOSE_SIGN_WINDOW);
                call(m, f, &[]);
            });
            mem.write(rec + 0xc, 0u8);
            let gil = mem.read::<u32>(rec + 0x10);
            ms_message_cue_regist(mem, 8, gil, 0, [0x1b, 0x23], |m| {
                let f = code(m, TO_BTL_CLOSE_SIGN_WINDOW);
                call(m, f, &[]);
            });
            let gil = word(mem, rec, 0x10);
            if gil > 0 {
                crate::exec::bridge::ms_pay_gil(mem, gil.wrapping_neg());
                ms_set_steal_gill_effect(mem, &mut *s.call, j, got);
                ms_reg_se_play2(mem, host.trig, &mut *s.call, j, 0x41);
            }
            or_byte(mem, rec, 5, 1);
        }
    }

    /// A hit record's transfers (`+4`, `AfterDamageProcess`), then
    /// cleared: bit 0 the overdrive gauge, bit 1 a learnt command.
    fn transfers<'d>(
        mem: &mut Dm<'d>,
        s: &mut Script<'_, Dm<'d>>,
        j: u32,
        by: u32,
        actor: u32,
        rec: u32,
    ) {
        let t = byte(mem, rec, 4);
        if t == 0 {
            return;
        }
        if t & 1 != 0 {
            let g = i32::from(byte(mem, by, 0x5bc));
            mem.write(by.wrapping_add(0x5bc), 0u8);
            let top = i32::from(byte(mem, actor, 0x5bd));
            let v = ms_check_range(i32::from(byte(mem, actor, 0x5bc)).wrapping_add(g), 0, top);
            mem.write(actor.wrapping_add(0x5bc), v.to_le_bytes()[0]);
        }
        if byte(mem, rec, 4) & 2 != 0 {
            use crate::save::bridge::SHARED_COMMANDS;
            let who = ms_get_chr(mem, u32::from(byte(mem, rec, 0x17)));
            let p: u32 = mem.read(who.wrapping_add(0xf88));
            let cmd = if j == 3 && p != 0 {
                u32::from(mem.read::<u16>(p + 6))
            } else {
                0
            };
            if cmd != 0 && crate::save::bridge::ms_get_save_command(mem, 3, cmd) == 0 {
                crate::ability::bridge::ms_set_save_command(mem, 3, cmd, 1);
                let call = &mut *s.call;
                crate::state::bridge::ms_message_cue_regist(mem, 6, 3, cmd, [0x1e, 0x32], |m| {
                    let f = code(m, crate::death::bridge::TO_BTL_CLOSE_SIGN_WINDOW);
                    call(m, f, &[]);
                });
                mem.write(actor.wrapping_add(0x5bc), byte(mem, actor, 0x5bd));
                let shared = SHARED_COMMANDS.get(mem);
                let more: u8 = mem.read(SHARED_COMMANDS.ptr(mem).addr + 2);
                if shared & 0xff00 == 0xff00 && more & 0xf == 0xf {
                    let f = code(mem, crate::parse::bridge::ACHIEVEMENT_UNLOCK);
                    (s.call)(mem, f, &[0x19]);
                }
            }
        }
        mem.write(rec + 4, 0u8);
    }

    /// `MsSetDamage(i, sel, flags)` (`0x0078d980`): actor `i & 0xff`'s
    /// command lands (hit `+2` of its cue block,
    /// [`crate::actor::bridge::ms_get_chr_command`]): of the 31 actors,
    /// those its target selector picks (`MsGetChrBit(i, sel, 1, 1)`) take
    /// it ([`after_damage_process`]; who warded a status kept in a local,
    /// -1 at first), the rest are only checked
    /// ([`crate::chr_damage::bridge::ms_check_hit_pending`]); how many
    /// have entries left. When none have, on the last hit (`+3` at most
    /// `+2` + 1) of a command that queued an input entry (`+0xde9`, entry
    /// `+0xde4` not 0xff): sound 0x3a on who warded
    /// ([`ms_reg_se_play`]), the user's self-destruct (`+0x6ca`, then
    /// cleared: blown away `+0x616` bit 8, `+0xf5e` bit 2, HP 0,
    /// [`crate::chr_damage::bridge::ms_blow_process`], its death without
    /// spoils), `+0xde9` cleared, the battle paused 6 frames
    /// ([`crate::process::bridge::PAUSE`]) and the input entry deleted
    /// (`MsDeleteInputCue(i, entry, 1, 0)`). 0 for no actor.
    pub fn ms_set_damage<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        i: u32,
        sel: u32,
        flags: u32,
    ) -> u32 {
        let i = i & 0xff;
        if crate::actor::bridge::ms_btl_chr_num_check(i) == 0 {
            return 0;
        }
        mem.locals(4, |mem, warded| {
            mem.write(warded, u32::MAX);
            let user = ms_get_chr(mem, i);
            let cmd = crate::actor::bridge::ms_get_chr_command(mem, i);
            let hit = u32::from(byte(mem, cmd, 2));
            let set = crate::actor::bridge::ms_get_chr_bit(mem, i, sel, 1, 1);
            let mut left = 0;
            for j in 0..0x1f {
                let r = if set.cast_signed() >> j & 1 != 0 {
                    after_damage_process(mem, host, s, i, hit, j, warded, flags)
                } else {
                    crate::chr_damage::bridge::ms_check_hit_pending(mem, i, hit, j)
                };
                left += r & 1;
            }
            if left != 0
                || byte(mem, user, 0xde9) == 0
                || byte(mem, user, 0xde4) == 0xff
                || u32::from(byte(mem, cmd, 2)) + 1 < u32::from(byte(mem, cmd, 3))
            {
                return left;
            }
            let who: i32 = mem.read(warded);
            if who >= 0 {
                ms_reg_se_play(mem, host.trig, &mut *s.call, who.cast_unsigned(), 0x3a);
            }
            if byte(mem, user, 0x6ca) != 0 {
                or_byte(mem, user, 0xf5e, 4);
                let st: u16 = mem.read(user.wrapping_add(0x616));
                mem.write(user.wrapping_add(0x616), st | 0x100);
                mem.write(user.wrapping_add(0x5d0), 0u32);
                crate::chr_damage::bridge::ms_blow_process(mem, &mut *s.call, i, user);
                crate::death::bridge::ms_damage_check_death(mem, host, s, i, i, 1, 0);
                mem.write(user.wrapping_add(0x6ca), 0u8);
            }
            mem.write(user.wrapping_add(0xde9), 0u8);
            crate::process::bridge::PAUSE.set(mem, 6);
            let entry = u32::from(byte(mem, user, 0xde4));
            crate::action::bridge::ms_delete_input_cue(mem, host, s, i, entry, 1, 0);
            left
        })
    }
}
