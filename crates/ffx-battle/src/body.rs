//! Battle actors' models (`docs/rewrite-plan.md`, M2): the getters and
//! setters that go through an actor's character (`ffx_chara`, the word at
//! the actor's `+0`, null without a model) for its height, direction,
//! ground, position, weight, collision and specular fade.
//!
//! | Offset | Type | Field |
//! |---|---|---|
//! | `+0` | `u32` | the character (`ffx_chara::chr`), null for none |
//! | `+0x2d`, `+0x2e` | `u8`, 4 × `u8` | body hit: on when all four parts are |
//! | `+0x34` | `u8` | set: position by x and z only (`Ch_SetPos`) |
//! | `+0x3a4`, `+0x3a8` | `f32` | y and x scale |
//! | `+0x412`, `+0x420` | `u8` | move flag, motion level |
//! | `+0x432`, `+0x436` | `u8` | motion running, motion waiting |
//! | `+0x43f`, `+0x440` | `u8` | the actor it belongs to; set: has followers |
//! | `+0x4dc` | `f32` | its own weight |

/// The game's memory, through [`ffx_bridge::Mem`].
#[allow(clippy::many_single_char_names)] // actor, character, index, value
pub mod bridge {
    use ffx_bridge::x87::{load, mul, store};
    use ffx_bridge::{Global, Mem};
    use ffx_chara::chr::bridge as ch;

    use crate::actor::bridge::{ms_btl_chr_num_check, ms_get_chr, ms_get_chr_bit, ms_get_weapon};

    fn chr_of(mem: &impl Mem, actor: u32) -> u32 {
        mem.read(actor)
    }

    /// The actors whose motion `MsCheckChrMotionAll` found running, a bit
    /// each.
    pub const MOTION_BUSY: Global<u32> = Global::new(0x0112_c8d4);
    /// -1.0 (`0x00b92340`).
    pub const MINUS_ONE: Global<u32> = Global::new(0x00b9_2340);

    /// `Battle_FirstSelectedActor(chr, selector, all, any)` (`0x007947f0`):
    /// the lowest actor in [`ms_get_chr_bit`]'s set, -1 for none.
    pub fn battle_first_selected_actor(
        mem: &impl Mem,
        chr: u32,
        selector: u32,
        all: i32,
        any: i32,
    ) -> i32 {
        let set = ms_get_chr_bit(mem, chr, selector, all, any);
        (0..0x1f).find(|&i| set >> i & 1 != 0).unwrap_or(-1)
    }

    /// `msSetChrBodyHit(actor, part, on)` (`0x00796f20`): part `part`
    /// (signed, below 4) on or off when `on` is not negative; the body hit
    /// (`+0x2d`) on when all four parts are, and given to the character;
    /// returns the character.
    pub fn ms_set_chr_body_hit(mem: &mut impl Mem, actor: u32, part: i32, on: i32) -> u32 {
        let c = chr_of(mem, actor);
        if on >= 0 && part < 4 {
            mem.write(
                actor.wrapping_add(0x2e).wrapping_add(part.cast_unsigned()),
                on.to_le_bytes()[0],
            );
        }
        let all = (0..4).all(|k| mem.read::<u8>(actor.wrapping_add(0x2e + k)) != 0);
        mem.write(actor.wrapping_add(0x2d), u8::from(all));
        if c != 0 {
            ch::ch_en_body_hit(mem, c, u32::from(all));
        }
        c
    }

    /// A character value times one of the actor's floats, as `f32`; 0
    /// without a character (`MsGetChrHeight` and the camera ones).
    fn scaled<M: Mem>(mem: &M, i: u32, o: u32, get: impl FnOnce(&M, u32) -> f32) -> f32 {
        let a = ms_get_chr(mem, i & 0xff);
        let c = chr_of(mem, a);
        if c == 0 {
            return 0.0;
        }
        let v = get(mem, c);
        let s: u32 = mem.read(a.wrapping_add(o));
        f32::from_bits(store(mul(load(v.to_bits()), load(s))))
    }

    /// `MsGetChrHeight(i)` (`0x007950f0`): the fixed height times the y
    /// scale.
    pub fn ms_get_chr_height(mem: &impl Mem, i: u32) -> f32 {
        scaled(mem, i, 0x3a4, ch::ch_get_fix_height)
    }

    /// `MsGetChrCamHeight(i)` (`0x00794a30`): the camera height times the
    /// y scale.
    pub fn ms_get_chr_cam_height(mem: &impl Mem, i: u32) -> f32 {
        scaled(mem, i, 0x3a4, ch::ch_get_cam_height)
    }

    /// `MsGetChrCamWidth(i)` (`0x00794a70`): the battle camera thickness
    /// times the x scale.
    pub fn ms_get_chr_cam_width(mem: &impl Mem, i: u32) -> f32 {
        scaled(mem, i, 0x3a8, ch::ch_get_thickness_btl_cam)
    }

    /// The actor's character when `i & 0xff` is an actor that has one.
    fn model(mem: &impl Mem, i: u32) -> Option<(u32, u32)> {
        let i = i & 0xff;
        if ms_btl_chr_num_check(i) == 0 {
            return None;
        }
        let a = ms_get_chr(mem, i);
        let c = chr_of(mem, a);
        (c != 0).then_some((a, c))
    }

    /// `MsGetChrDir(i, &dir)` (`0x00794d20`): the character's rotation into
    /// `dir`; -1, or 0 without one.
    pub fn ms_get_chr_dir(mem: &mut impl Mem, i: u32, dir: u32) -> i32 {
        let Some((_, c)) = model(mem, i) else {
            return 0;
        };
        let r = ch::ch_get_rot(mem, c);
        mem.write(dir, store(load(r.to_bits())));
        -1
    }

    /// `MsGetChrGround(i)` (`0x00794fb0`): the character's ground height,
    /// 0 without one.
    pub fn ms_get_chr_ground(mem: &impl Mem, i: u32) -> f32 {
        model(mem, i).map_or(0.0, |(_, c)| ch::ch_get_jimen(mem, c))
    }

    /// `MsSetChrPos(i, pos)` (`0x00796150`): the character to `pos` (x, y,
    /// z; x and z only with `+0x34`); -1, or 0 without one.
    pub fn ms_set_chr_pos(mem: &mut impl Mem, i: u32, pos: u32) -> i32 {
        let Some((a, c)) = model(mem, i) else {
            return 0;
        };
        let p: [u32; 3] = mem.read(pos);
        let [x, y, z] = p.map(|v| f32::from_bits(store(load(v))));
        if mem.read::<u8>(a.wrapping_add(0x34)) == 0 {
            ch::ch_set_pos3(mem, c, x, y, z);
        } else {
            ch::ch_set_pos(mem, c, x, z);
        }
        -1
    }

    /// `MsSetChrWeight(i, how)` (`0x007968e0`): the character's weight: the
    /// actor's own (`+0x4dc`) for `how` above 0, -1 below, else 0.
    pub fn ms_set_chr_weight(mem: &mut impl Mem, i: u32, how: i32) {
        let Some((a, c)) = model(mem, i) else {
            return;
        };
        let w = match how {
            1.. => mem.read(a.wrapping_add(0x4dc)),
            0 => 0,
            _ => MINUS_ONE.get(mem),
        };
        ch::ch_set_weight(mem, c, f32::from_bits(store(load(w))));
    }

    /// `msSetChrWeight(actor, w)` (`0x007972c0`): the character's weight.
    pub fn ms_set_chr_weight2(mem: &mut impl Mem, actor: u32, w: u32) {
        let c = chr_of(mem, actor);
        if c != 0 {
            ch::ch_set_weight(mem, c, f32::from_bits(store(load(w))));
        }
    }

    /// A byte of actor `i & 0xff`; -1, or 0 for no actor
    /// (`MsSetChrMoveFlag`, `MsSetChrMotionLevel`).
    fn set_byte(mem: &mut impl Mem, i: u32, o: u32, v: u8) -> i32 {
        let i = i & 0xff;
        if ms_btl_chr_num_check(i) == 0 {
            return 0;
        }
        let a = ms_get_chr(mem, i);
        mem.write(a.wrapping_add(o), v);
        -1
    }

    /// `MsSetChrMoveFlag(i, v)` (`0x00796110`): `+0x412`.
    pub fn ms_set_chr_move_flag(mem: &mut impl Mem, i: u32, v: u8) -> i32 {
        set_byte(mem, i, 0x412, v)
    }

    /// `MsSetChrMotionLevel(i, v)` (`0x007960d0`): `+0x420`.
    pub fn ms_set_chr_motion_level(mem: &mut impl Mem, i: u32, v: u8) -> i32 {
        set_byte(mem, i, 0x420, v)
    }

    /// `MsSetChrSpecular(i, v, frames)` (`0x00796360`): the specular fade
    /// of actor `i & 0xff`'s character, its two weapons' and, with
    /// followers (`+0x440`), theirs (actors whose `+0x43f` is `i`, in
    /// turn); 1 when the actor has a character.
    pub fn ms_set_chr_specular(mem: &mut impl Mem, i: u32, v: u32, frames: i32) -> u32 {
        let i = i & 0xff;
        let a = ms_get_chr(mem, i);
        let c = chr_of(mem, a);
        let fv = f32::from_bits(store(load(v)));
        if c != 0 {
            ch::ch_set_fade(mem, c, ffx_chara::layout::FADE_SPECULAR, fv, frames);
        }
        for k in 0..2 {
            let w = ms_get_weapon(mem, i, k);
            if w != 0 {
                let wc = chr_of(mem, w);
                if wc != 0 {
                    ch::ch_set_fade(mem, wc, ffx_chara::layout::FADE_SPECULAR, fv, frames);
                }
            }
        }
        if mem.read::<u8>(a.wrapping_add(0x440)) != 0 {
            for k in 0..0x1f {
                let f = ms_get_chr(mem, k);
                if u32::from(mem.read::<u8>(f.wrapping_add(0x43f))) == i {
                    ms_set_chr_specular(mem, k, v, frames);
                }
            }
        }
        u32::from(c != 0)
    }

    /// `MsCheckChrMotionAll(mode)` (`0x00791630`): 1 when one of the first
    /// 18 actors runs a motion (`+0x432`), or with mode 2 waits on one
    /// (`+0x436`); the first such is marked in [`MOTION_BUSY`] unless
    /// `mode` is 0. 0 when none.
    pub fn ms_check_chr_motion_all(mem: &mut impl Mem, mode: i32) -> u32 {
        for k in 0..0x12 {
            let a = ms_get_chr(mem, k);
            let run = mem.read::<u8>(a.wrapping_add(0x432)) != 0;
            if run || (mem.read::<u8>(a.wrapping_add(0x436)) != 0 && mode == 2) {
                if mode != 0 {
                    let m = MOTION_BUSY.get(mem);
                    MOTION_BUSY.set(mem, m | 1 << k);
                }
                return 1;
            }
        }
        0
    }
}
