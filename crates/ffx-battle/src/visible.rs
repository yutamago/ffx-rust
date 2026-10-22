//! Whether actors are shown (`docs/rewrite-plan.md`, M2): an actor is
//! visible when all four of its visibility requests (`+0x16`-`+0x19`: the
//! camera's is slot 3) are set; `msSetChrVisibleReq` applies that to its
//! character, its weapons, its bound actors and the actors standing on it.
//! The characters' flags are `ffx_chara`'s; `op_et_bindeff` (vfx) goes
//! through a [`Call`].

/// The game's memory, through [`ffx_bridge::Mem`].
#[allow(clippy::many_single_char_names)] // indices, actors, records
pub mod bridge {
    use ffx_bridge::{Global, IMAGE_BASE, Mem};
    use ffx_chara::chr::bridge as ch;

    use crate::actor::bridge::{ms_get_chr, ms_get_weapon};
    use crate::exec::bridge::Call;

    fn byte(mem: &impl Mem, p: u32, o: u32) -> u8 {
        mem.read(p.wrapping_add(o))
    }

    fn sbyte(mem: &impl Mem, p: u32, o: u32) -> u32 {
        i32::from(mem.read::<i8>(p.wrapping_add(o))).cast_unsigned()
    }

    /// `op_et_bindeff` (vfx).
    pub const OP_ET_BINDEFF: u32 = 0x007f_b290;
    /// Set while the camera hides actors (`MsSetChrVisibleCamera`).
    pub const CAMERA_HIDES: Global<u8> = Global::new(0x0112_c9fe);
    /// While a bit of [`HIDE_LATCH`] is in this mask, hiding a character
    /// queues it in [`HIDE_QUEUE`] instead (once; the latch clears).
    pub const HIDE_MASK: Global<u32> = Global::new(0x0113_6b58);
    pub const HIDE_LATCH: Global<u32> = Global::new(0x0113_63b8);
    /// The queue's last index (it starts at 1) and its characters.
    pub const HIDE_QUEUE_LAST: Global<i32> = Global::new(0x0113_34d0);
    pub const HIDE_QUEUE: Global<u32> = Global::new(0x0231_1060);

    /// `0x00797020(p, k, v)` (`msSetChrShadowReq`, name ours): shadow request
    /// `k` (below 3) of actor or weapon record `p` set to `v`; the first
    /// non-negative request, stopping at a 0 (`+0x25`), picks the shadow
    /// mode `+0x2a[that]` (`+0x29`), given to its character
    /// (`Ch_SetShadowMode`). The character.
    pub fn ms_set_chr_shadow_req(mem: &mut impl Mem, p: u32, k: i32, v: u8) -> u32 {
        let chr: u32 = mem.read(p);
        if k < 3 {
            mem.write(p.wrapping_add(0x26).wrapping_add(k.cast_unsigned()), v);
        }
        let mut pick = 0u32;
        for e in 0..3 {
            let c = mem.read::<i8>(p.wrapping_add(0x26 + e));
            if c >= 0 {
                pick = u32::from(c.cast_unsigned());
                if c == 0 {
                    break;
                }
            }
        }
        mem.write(p.wrapping_add(0x25), pick.to_le_bytes()[0]);
        let mode = sbyte(mem, p, 0x2a + pick);
        mem.write(p.wrapping_add(0x29), mode.to_le_bytes()[0]);
        if chr != 0 {
            ch::ch_set_shadow_mode(mem, chr, mode);
        }
        chr
    }

    /// A character shown or hidden, as `msSetChrVisibleReq` does it for an
    /// actor or weapon record `p` (shown: its map ID `+0x32` and normals
    /// `+0x33`, shadow request 1 -1; hidden: 0, shadow request 1 0).
    fn show(mem: &mut impl Mem, p: u32, chr: u32, on: bool) {
        let v = u32::from(!on);
        ch::ch_set_hide_battle(mem, chr, v);
        ch::ch_set_hide_effect(mem, chr, v);
        show_rest(mem, p, chr, on);
    }

    fn show_rest(mem: &mut impl Mem, p: u32, chr: u32, on: bool) {
        if on {
            ch::ch_en_map_id(mem, chr, sbyte(mem, p, 0x32));
            ffx_chara::parts::bridge::ch_set_calc_norm(mem, chr, sbyte(mem, p, 0x33));
            ms_set_chr_shadow_req(mem, p, 1, 0xff);
        } else {
            ch::ch_en_map_id(mem, chr, 0);
            ffx_chara::parts::bridge::ch_set_calc_norm(mem, chr, 0);
            ms_set_chr_shadow_req(mem, p, 1, 0);
        }
    }

    /// `0x00797080(i, k, v)` (`msSetChrVisibleReq`, name ours): actor
    /// `i & 0xff`'s visibility request `k` set to `v`; visible (`+0x15`)
    /// when all four are. Becoming visible with a pending bind effect
    /// (`+0x36`) starts it (`+0x1f` set, `op_et_bindeff`). Its character
    /// is shown or hidden (hidden while [`HIDE_LATCH`] meets [`HIDE_MASK`]:
    /// queued in [`HIDE_QUEUE`] instead of its hide flags) with its body hit
    /// (`msSetChrBodyHit(p, 1, visible)`). The same request goes to the
    /// actor it is bound to (`+0x13`) and, with `+0x440`, to the actors on
    /// it (`+0x43f`); its two weapons follow (visible only with `+0x1d`).
    /// 1 when it has a character, else 0.
    pub fn ms_set_chr_visible_req<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        i: u32,
        k: u32,
        v: u32,
    ) -> u32 {
        let i = i & 0xff;
        let a = ms_get_chr(mem, i);
        mem.write(a.wrapping_add(0x16).wrapping_add(k), v.to_le_bytes()[0]);
        let chr: u32 = mem.read(a);
        let vis = (0x16..0x1a).all(|o| byte(mem, a, o) != 0);
        mem.write(a.wrapping_add(0x15), u8::from(vis));
        let mut has = 0;
        let bind = vis && byte(mem, a, 0x36) != 0;
        if !(bind && chr == 0) {
            if bind {
                mem.write(a.wrapping_add(0x36), 0u8);
                mem.write(a.wrapping_add(0x1f), 1u8);
                let f = OP_ET_BINDEFF
                    .wrapping_sub(IMAGE_BASE)
                    .wrapping_add(mem.image_base());
                call(mem, f, &[chr]);
            }
            if chr != 0 {
                if vis {
                    show(mem, a, chr, true);
                } else {
                    if HIDE_LATCH.get(mem) & HIDE_MASK.get(mem) != 0 {
                        let n = HIDE_QUEUE_LAST.get(mem).wrapping_add(1);
                        HIDE_QUEUE_LAST.set(mem, n);
                        HIDE_QUEUE.ptr(mem).at(n.cast_unsigned()).set(mem, chr);
                        HIDE_LATCH.set(mem, 0);
                    } else {
                        ch::ch_set_hide_battle(mem, chr, 1);
                        ch::ch_set_hide_effect(mem, chr, 1);
                    }
                    show_rest(mem, a, chr, false);
                }
                crate::body::bridge::ms_set_chr_body_hit(mem, a, 1, i32::from(vis));
                has = 1;
            }
        }
        let bound = byte(mem, a, 0x13);
        if bound != 0xff {
            ms_set_chr_visible_req(mem, call, u32::from(bound), k, v);
        }
        if byte(mem, a, 0x440) != 0 {
            for j in 0..0x1f {
                if u32::from(byte(mem, ms_get_chr(mem, j), 0x43f)) == i {
                    ms_set_chr_visible_req(mem, call, j, k, v);
                }
            }
        }
        let wvis = vis && byte(mem, a, 0x1d) != 0;
        for n in 0..2 {
            let w = ms_get_weapon(mem, i, n);
            if w == 0 {
                continue;
            }
            let c: u32 = mem.read(w);
            if c == 0 {
                continue;
            }
            show(mem, w, c, wvis);
            crate::body::bridge::ms_set_chr_body_hit(mem, w, 1, i32::from(wvis));
        }
        has
    }

    /// `MsSetChrVisible(i, v)` (`0x00796660`): request 2 of
    /// [`ms_set_chr_visible_req`] (EAX its result).
    pub fn ms_set_chr_visible<M: Mem>(mem: &mut M, call: Call<'_, M>, i: u32, v: u32) -> u32 {
        ms_set_chr_visible_req(mem, call, i, 2, v)
    }

    /// `MsSetMonsterVisible_007969a0(m, v)` (name ours; caller
    /// `Native_7127`): [`ms_set_chr_visible`] of monster `m` (actor `m +
    /// 0x14`); returns `m + 0x14`.
    pub fn ms_set_monster_visible<M: Mem>(mem: &mut M, call: Call<'_, M>, m: u32, v: u32) -> u32 {
        let i = m.wrapping_add(0x14);
        ms_set_chr_visible(mem, call, i, v);
        i
    }

    /// `MsSetChrVisibleAppear(i, v)` (`0x007966a0`): request 1 of
    /// [`ms_set_chr_visible_req`] (EAX its result).
    pub fn ms_set_chr_visible_appear<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        i: u32,
        v: u32,
    ) -> u32 {
        ms_set_chr_visible_req(mem, call, i, 1, v)
    }

    /// `MsSetChrVisibleCamera(i, v)` (`0x007967a0`): [`CAMERA_HIDES`] set
    /// and the camera's request (3) of actor `i` set to `v`.
    pub fn ms_set_chr_visible_camera<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        i: u32,
        v: u32,
    ) -> u32 {
        CAMERA_HIDES.set(mem, 1);
        ms_set_chr_visible_req(mem, call, i, 3, v)
    }

    /// `MsSetChrVisibleCameraAll(v)` (`0x007967c0`): the same for the 31
    /// actors; the last one's result.
    pub fn ms_set_chr_visible_camera_all<M: Mem>(mem: &mut M, call: Call<'_, M>, v: u32) -> u32 {
        let mut r = 0;
        for i in 0..0x1f {
            r = ms_set_chr_visible_camera(mem, call, i, v);
        }
        r
    }

    /// `MsReturnCameraVisible()` (`0x00795c70`): when the camera hid actors
    /// ([`CAMERA_HIDES`], cleared), all shown again; `None` otherwise (the
    /// original leaves `eax` as it was).
    pub fn ms_return_camera_visible<M: Mem>(mem: &mut M, call: Call<'_, M>) -> Option<u32> {
        if CAMERA_HIDES.get(mem) == 0 {
            return None;
        }
        CAMERA_HIDES.set(mem, 0);
        Some(ms_set_chr_visible_camera_all(mem, call, 1))
    }

    /// The clip distance [`ms_set_chr_engine`] gives (`0x00b3eba0`).
    pub const CLIP_Z: f32 = 5000.0;

    /// `0x00796f70(chr, p)` (`MsSetChrEngine_00796f70`, name ours; callers
    /// `MsBtlChrRegist`, `MsBtlWeaponRegist`): character `chr` set up from
    /// actor or weapon record `p`: no offset, its hide-calc (`+0x37`),
    /// motion type and field mode (`+0x20`), map ID (`+0x32`), ground mode
    /// (`+0x34`), normals (`+0x33`), clipping off at [`CLIP_Z`]; the
    /// record's body hit 2 on and shadow request 2 to -1. The bytes are
    /// signed. `Ch_EngineSetFast(chr, +0x23)` (`0x00830370`) is a bare
    /// `ret`: left out. Returns the record's character (the shadow
    /// request's).
    pub fn ms_set_chr_engine<M: Mem>(mem: &mut M, call: Call<'_, M>, chr: u32, p: u32) -> u32 {
        ch::ch_set_offset(mem, chr, 0.0, 0.0, 0.0);
        let field = |mem: &M, o: u32| sbyte(mem, p, o);
        let v = field(mem, 0x37);
        ch::ch_set_u8(mem, chr, ffx_chara::layout::HIDE_CALC, v);
        let v = field(mem, 0x20);
        ch::ch_set_u8(mem, chr, ffx_chara::layout::MOTION_TYPE, v);
        let v = field(mem, 0x20);
        ch::ch_set_u8(mem, chr, ffx_chara::layout::FIELD_MODE, v);
        let v = field(mem, 0x32);
        ch::ch_en_map_id(mem, chr, v);
        let v = field(mem, 0x34);
        ch::ch_set_u8(mem, chr, ffx_chara::layout::GRAV_MODE, v);
        let v = field(mem, 0x33);
        ffx_chara::parts::bridge::ch_set_calc_norm(mem, chr, v);
        ch::ch_en_clip(mem, chr, 0);
        ch::ch_set_clip_z(mem, chr, CLIP_Z, &mut |m, f, a| call(m, f, a));
        crate::body::bridge::ms_set_chr_body_hit(mem, p, 2, 1);
        ms_set_chr_shadow_req(mem, p, 2, 0xff)
    }
}
