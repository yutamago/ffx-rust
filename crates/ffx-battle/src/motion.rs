//! The actors' motions each frame (`docs/rewrite-plan.md`, M2):
//! `MsMotionProcess` (`0x007aa160`), which the first script's update
//! (`0x007972e0`) runs: every actor's motion speed, its queued motion, and
//! an idle actor's stance check against the others standing near it.

/// The game's memory, through [`ffx_bridge::Mem`].
#[allow(clippy::many_single_char_names)] // actors, indices, the script
pub mod bridge {
    use ffx_bridge::Mem;

    use crate::actor::bridge::ms_get_chr;
    use crate::atel::bridge::Script;
    use crate::pos::bridge::Trig;
    use crate::target::bridge::Dm;

    fn byte(mem: &impl Mem, p: u32, o: u32) -> u8 {
        mem.read(p.wrapping_add(o))
    }

    fn half(mem: &impl Mem, p: u32, o: u32) -> u16 {
        mem.read(p.wrapping_add(o))
    }

    /// `MsCalcMotionSpeed(actor)` (`0x0078ac20`): the actor's speed factor
    /// (`+0x406`, in eighths): a ride's (`+0x12`) its rider's (`+0x14`),
    /// 0 while `+0xdce` or while a `+0x606` bit 11 actor without `+0x5c6`
    /// neither waits on a motion (`+0x432`) nor moves (`+0x508`), 16 with
    /// `+0x613`, 4 with `+0x614` in motions 0x10-0x2a (`+0x3f0`), else 8.
    /// The speed is `+0x402` with `+0x404` 1, else `+0x3fc` times the
    /// factor over 8; a model shown (`+0x38`) whose speed (`+0x400`)
    /// changes gets it (`Ch_SetMotionSpeed`), its texture animation timer
    /// started from 0 or stopped at 0 (`Ch_TextureSetAnimTimer`). Returns
    /// the speed.
    pub fn ms_calc_motion_speed(mem: &mut impl Mem, a: u32) -> u32 {
        let chr: u32 = mem.read(a);
        let factor = if byte(mem, a, 0x12) != 0 {
            let rider = ms_get_chr(mem, u32::from(byte(mem, a, 0x14)));
            u32::from(byte(mem, rider, 0x406))
        } else if byte(mem, a, 0xdce) != 0 {
            0
        } else if half(mem, a, 0x606) & 0x800 != 0 && byte(mem, a, 0x5c6) == 0xff {
            if byte(mem, a, 0x432) == 0 && byte(mem, a, 0x508) == 0 {
                0
            } else {
                8
            }
        } else if byte(mem, a, 0x613) != 0 {
            0x10
        } else if byte(mem, a, 0x614) != 0
            && u32::from(byte(mem, a, 0x3f0)).wrapping_sub(0x10) <= 0x1a
        {
            4
        } else {
            8
        };
        #[allow(clippy::cast_possible_truncation)] // the original's byte store
        mem.write(a.wrapping_add(0x406), factor as u8);
        let speed = if byte(mem, a, 0x404) == 1 {
            u32::from(half(mem, a, 0x402))
        } else {
            // a signed product and division, never negative here
            ((u32::from(half(mem, a, 0x3fc)) * factor).cast_signed() / 8).cast_unsigned()
        };
        let old = u32::from(half(mem, a, 0x400));
        if chr != 0 && speed != old && byte(mem, a, 0x38) != 0 {
            if old == 0 && speed != 0 {
                ffx_chara::texanim::bridge::ch_texture_set_anim_timer(mem, chr, 1);
            }
            if old != 0 && speed == 0 {
                ffx_chara::texanim::bridge::ch_texture_set_anim_timer(mem, chr, 0);
            }
            ffx_chara::chr::bridge::ch_set_motion_speed(mem, chr, speed);
            #[allow(clippy::cast_possible_truncation)] // the original's halfword store
            mem.write(a.wrapping_add(0x400), speed as u16);
        }
        speed
    }

    /// The motion `MsCheckChrTouch_007ab750` records with the actor it
    /// touched (`g_BtlEffectWait`'s second byte).
    pub const TOUCH_MOTION: u8 = 0xfc;

    /// `MsCheckChrTouch_007ab750(i, actor)`: 0 when actor `i` (not held,
    /// `+0xdf0`; not running a motion, `g_BtlMotionBusy`; no script camera
    /// unless a command is queued; not in battle state 0x1b) stands at its
    /// battle position within the widths of one of the first 0x1c others
    /// that has a model and `+0x2d` (2D distance to that one's position,
    /// less than the two widths' `f32` sum): that one goes to
    /// `g_BtlEffectWait` with [`TOUCH_MOTION`]. Else -1.
    pub fn ms_check_chr_touch(
        mem: &mut Dm<'_>,
        trig: &dyn Fn(Trig, u32) -> u32,
        i: u32,
        a: u32,
    ) -> i32 {
        use crate::actor::bridge::ms_get_chr_width;
        use crate::pos::bridge::{ms_get_chr_btl_pos, ms_get_chr_pos};
        use ffx_bridge::x87::{add, load, store};
        if byte(mem, a, 0xdf0) != 0 {
            return -1;
        }
        if crate::body::bridge::MOTION_BUSY.get(mem) & (1 << (i & 0x1f)) != 0
            || (crate::atel::bridge::CAMERA_STATE.get(mem) != 0
                && crate::state::bridge::COMMAND_CUE_COUNT.get(mem) == 0)
            || crate::state::bridge::STATUS.get(mem) == 0x1b
        {
            return -1;
        }
        let w = ms_get_chr_width(mem, i).to_bits();
        mem.locals(0x20, |mem, at| {
            let (pos, other) = (at + 0x10, at);
            ms_get_chr_btl_pos(mem, trig, i, pos);
            for j in 0..0x1c {
                let c = ms_get_chr(mem, j);
                if mem.read::<u32>(c) == 0 || byte(mem, c, 0x2d) == 0 || j == i {
                    continue;
                }
                let sum = store(add(load(ms_get_chr_width(mem, j).to_bits()), load(w)));
                ms_get_chr_pos(mem, trig, j, other);
                let len = crate::vector::bridge::ms_point_len_2d(mem, pos, other);
                if f32::from_bits(sum) > len {
                    #[allow(clippy::cast_possible_truncation)] // below 0x1c
                    crate::flow::bridge::EFFECT_WAIT.set(mem, [j as u8, TOUCH_MOTION]);
                    return 0;
                }
            }
            -1
        })
    }

    /// `MsMotionProcess()` (`0x007aa160`): for each of the 0x1f actors,
    /// its motion speed ([`ms_calc_motion_speed`], with a model); then,
    /// unless it waits on a motion (`+0x432`), its queued motion
    /// (`+0x3f2`, cleared to 0xff) set (`MsSetMotion(i, m, 0, 0, 1, 0,
    /// 0)`), or, not moving to a target (`+0x412`, `+0x415`): with
    /// `+0x508` set and no other actor in reach ([`ms_check_chr_touch`]
    /// not 0) it goes back to motion 0, its counter `+0x428` reset and,
    /// with that counter below 2, every fourth time (`+0x427`) its body
    /// part 2 off (`msSetChrBodyHit`); without `+0x508` the counter
    /// counts up to 2.
    pub fn ms_motion_process<'d>(
        mem: &mut Dm<'d>,
        trig: &dyn Fn(Trig, u32) -> u32,
        s: &mut Script<'_, Dm<'d>>,
    ) {
        use crate::atel::bridge::ms_set_motion;
        for i in 0..0x1f {
            let a = ms_get_chr(mem, i);
            if mem.read::<u32>(a) != 0 {
                ms_calc_motion_speed(mem, a);
            }
            if byte(mem, a, 0x432) != 0 {
                continue;
            }
            let m = byte(mem, a, 0x3f2);
            if m != 0xff {
                mem.write(a.wrapping_add(0x3f2), 0xffu8);
                ms_set_motion(mem, s, i, i32::from(m), 0, 0, 1, 0, 0);
                continue;
            }
            if byte(mem, a, 0x412) != 0 || byte(mem, a, 0x415) != 0 {
                continue;
            }
            let n = byte(mem, a, 0x428);
            if byte(mem, a, 0x508) == 0 {
                if n < 2 {
                    mem.write(a.wrapping_add(0x428), n + 1);
                }
                continue;
            }
            if ms_check_chr_touch(mem, trig, i, a) == 0 {
                continue;
            }
            if n <= 1 {
                let k = byte(mem, a, 0x427).wrapping_add(1);
                mem.write(a.wrapping_add(0x427), k);
                if k >= 4 {
                    mem.write(a.wrapping_add(0x427), 0u8);
                    crate::body::bridge::ms_set_chr_body_hit(mem, a, 2, 0);
                }
            } else {
                mem.write(a.wrapping_add(0x427), 0u8);
            }
            mem.write(a.wrapping_add(0x428), 0u8);
            ms_set_motion(mem, s, i, 0, 0, 0, 1, 0, 0);
        }
    }
}
