//! Attachment points: a model block's list of named points on elements
//! (`Ch_ElemGetAttr`), binding a character to one (`Ch_BindToElem`), and
//! the optional positions (`Ch_OptposGet`).
//!
//! The list is at the block's `+0x6c` (pointer): entries of 16 bytes from
//! the pointer at `+0x20`, count at `+0x24`. An entry has a `u16` with the
//! point id in its low 14 bits and a kind in the top two, the `i16`
//! element at `+2`, and an offset x, y, z at `+4`.

/// What `Ch_ElemGetAttr` makes of an entry's kind bits: (result, whether
/// the offset is written).
#[must_use]
pub const fn attr_kind(kind: u16) -> (u32, bool) {
    match kind & 0xc000 {
        0x4000 => (1, true),
        0x8000 => (2, false),
        _ => (0, false),
    }
}

/// The originals, on the characters.
pub mod bridge {
    use ffx_bridge::Mem;

    use super::attr_kind;
    use crate::chr::bridge::{blk, f, put_f};
    use crate::layout as l;
    use crate::x87::Reg;

    /// Model block: the attachment point list.
    pub const BLK_ATTRS: u32 = 0x6c;
    /// Optional positions: 16 bytes each (x, y, z, w) from here; one
    /// exists when a component is not 0 (`Ch_OptposGetExist` tests w).
    pub const OPTPOS: u32 = 0x524;
    /// `f32` x, y, z: where `Ch_OptposGet` falls back to when position 0
    /// is empty too.
    pub const OPTPOS_FALLBACK: u32 = 0x200;

    /// The entry of point `id`, if the list has it.
    fn find(mem: &impl Mem, list: u32, id: u32) -> Option<u32> {
        let mut n: u32 = mem.read(list + 0x24);
        let mut e: u32 = mem.read(list + 0x20);
        while n != 0 {
            #[allow(clippy::cast_sign_loss)] // movsx then masked
            let key = i32::from(mem.read::<i16>(e)) as u32 & 0x3fff;
            n -= 1;
            if key == id {
                return Some(e);
            }
            e = e.wrapping_add(16);
        }
        None
    }

    fn element(mem: &impl Mem, e: u32) -> u32 {
        #[allow(clippy::cast_sign_loss)] // movsx
        let v = i32::from(mem.read::<i16>(e + 2)) as u32;
        v
    }

    fn unit(mem: &mut impl Mem, out: u32) {
        mem.write(out, [0.0f32, 0.0, 0.0, 1.0]);
    }

    /// `Ch_ElemGetAttr(chr, id, &elem, &offset)` (`0x00833a70`): point
    /// `id`'s element and, for kind 1, its offset scaled by 10000 / scale²
    /// × the bind scale (`offset` is a vector of four, w = 1; null skips
    /// it). Returns the kind: 1, 2, or 0 (also when there is no such
    /// point).
    pub fn ch_elem_get_attr(mem: &mut impl Mem, chr: u32, id: u32, elem: u32, out: u32) -> u32 {
        elem_get_attr(mem, chr, id, Some(elem), out).0
    }

    /// [`ch_elem_get_attr`] with the element also returned (and stored
    /// only when `elem` is given): (kind, element).
    fn elem_get_attr(
        mem: &mut impl Mem,
        chr: u32,
        id: u32,
        elem: Option<u32>,
        out: u32,
    ) -> (u32, u32) {
        if let Some(at) = elem {
            mem.write(at, 0u32);
        }
        if out != 0 {
            unit(mem, out);
        }
        let b = blk(mem, chr);
        let list: u32 = mem.read(b + BLK_ATTRS);
        if list == 0 {
            return (0, 0);
        }
        let Some(e) = find(mem, list, id) else {
            return (0, 0);
        };
        let v = element(mem, e);
        if let Some(at) = elem {
            mem.write(at, v);
        }
        let (kind, offset) = attr_kind(mem.read(e));
        if mem.read::<u16>(e) & 0xc000 == 0 {
            if out != 0 {
                unit(mem, out);
            }
            return (0, v);
        }
        if !offset || out == 0 {
            return (kind, v);
        }
        let s = f(mem, b + l::BLK_DATA_SCALE);
        let k = Reg::constant(1.0e4)
            .div_st(Reg::load(s))
            .div_m32(s)
            .mul_m32(f(mem, b + l::BLK_BIND_SCALE))
            .store();
        let k = Reg::load(k);
        let x = Reg::load(f(mem, e + 4)).mul_st(k).store();
        let y = Reg::load(f(mem, e + 8)).mul_st(k).store();
        let z = k.mul_m32(f(mem, e + 0xc)).store();
        mem.write(out, [x, y, z, 1.0]);
        (kind, v)
    }

    /// `Ch_ElemGetDirect(chr, id)` (`0x00833bf0`): point `id`'s element,
    /// or 0. The list must exist.
    pub fn ch_elem_get_direct(mem: &impl Mem, chr: u32, id: u32) -> u32 {
        let list: u32 = mem.read(blk(mem, chr) + BLK_ATTRS);
        find(mem, list, id).map_or(0, |e| element(mem, e))
    }

    /// `Ch_BindToElem(chr, parent, id)` (`0x00832630`): binds to the
    /// parent's point `id` (element and offset by `Ch_ElemGetAttr`, whose
    /// result it returns); a null parent only unbinds (and returns
    /// nothing; 0 here).
    pub fn ch_bind_to_elem(mem: &mut impl Mem, chr: u32, parent: u32, id: u32) -> u32 {
        mem.write(chr + l::BIND_PARENT, parent);
        if parent == 0 {
            mem.write(chr + l::BIND_PARENT_ID, u32::MAX);
            return 0;
        }
        let pid: u32 = mem.read(parent + l::BIND_SELF);
        mem.write(chr + l::BIND_PARENT_ID, pid);
        ch_elem_get_attr(mem, parent, id, chr + l::BIND_ELEM, chr + l::BIND_OFFSET)
    }

    /// `Ch_OptposGetElement(chr, id, &offset)` (`0x00835660`): point
    /// `id`'s element (`Ch_ElemGetAttr`, the element in a local).
    pub fn ch_optpos_get_element(mem: &mut impl Mem, chr: u32, id: u32, out: u32) -> u32 {
        elem_get_attr(mem, chr, id, None, out).1
    }

    /// `Ch_OptposGet(chr, i, out)` (`0x008354f0`): optional position `i`
    /// when any of its components is not 0, else position 0 when its w is
    /// not 0, else the fallback point with w = 1.
    pub fn ch_optpos_get(mem: &mut impl Mem, chr: u32, i: u32, out: u32) {
        let v = optpos(mem, chr, i);
        mem.write(out, v);
    }

    /// What [`ch_optpos_get`] writes, as bits.
    fn optpos(mem: &impl Mem, chr: u32, i: u32) -> [u32; 4] {
        let at = |k: u32| chr.wrapping_add(OPTPOS).wrapping_add(k.wrapping_mul(16));
        let p = at(i);
        // w first, then x, y, z; NaN counts as not 0
        let set = [12u32, 0, 4, 8]
            .iter()
            .any(|&c| f(mem, p.wrapping_add(c)) != 0.0);
        if set {
            mem.read(p)
        } else if f(mem, at(0) + 12) != 0.0 {
            mem.read(at(0))
        } else {
            let v = [0, 4, 8].map(|c| crate::x87::load(f(mem, chr + OPTPOS_FALLBACK + c)));
            [
                v[0].to_bits(),
                v[1].to_bits(),
                v[2].to_bits(),
                1f32.to_bits(),
            ]
        }
    }

    /// `Ch_OptposGetBase(chr, out)` (`0x008355e0`): `Ch_OptposGet(chr, 0,
    /// out)`.
    pub fn ch_optpos_get_base(mem: &mut impl Mem, chr: u32, out: u32) {
        ch_optpos_get(mem, chr, 0, out);
    }

    /// `10000.0`, the `double` `Ch_OptposCalc` divides by the data scale.
    pub const K_10000: ffx_bridge::Global<f64> = ffx_bridge::Global::new(0x00b3_0720);
    /// `0.5` (`double`).
    pub const K_HALF: ffx_bridge::Global<f64> = ffx_bridge::Global::new(0x00b9_2220);
    /// `0.8f` widened to `double`.
    pub const K_0_8: ffx_bridge::Global<f64> = ffx_bridge::Global::new(0x00b3_eb68);
    /// `1000.0f`: the w scale of the element matrices.
    pub const K_1000: ffx_bridge::Global<f32> = ffx_bridge::Global::new(0x00b1_f2cc);

    /// `Ch_OptposCalc(chr)` (`0x008350f0`): every optional position from
    /// the attachment points (an element's position through the root
    /// matrix, or for kind 1 the point's scaled offset through it), then
    /// the derived points: camera reference (`+0x6b4`, position 0 lowered
    /// by the camera height), bottom (`+0x694`, the middle of positions
    /// 7, 8 and 1 at the higher of 7's and 8's heights; the old one kept
    /// at `+0x684`), top (`+0x6a4`), the heights `+0x510` and `+0x514`,
    /// and `+0x6c4`.
    #[allow(clippy::too_many_lines)]
    pub fn ch_optpos_calc(mem: &mut impl Mem, chr: u32) {
        use ffx_system::math::{mul4x4, mul4x4_component};
        for i in 0..22 {
            put_f(mem, chr + OPTPOS + 12 + 16 * i, 0.0);
        }
        let b = blk(mem, chr);
        let list: u32 = mem.read(b + BLK_ATTRS);
        if list == 0 {
            return;
        }
        let mut n: i32 = mem.read(list + 0x24);
        let mut e: u32 = mem.read(list + 0x20);
        let s = f(mem, b + l::BLK_DATA_SCALE);
        let k = Reg::constant(K_10000.get(mem))
            .div_st(Reg::load(s))
            .div_m32(s)
            .mul_m32(f(mem, b + l::BLK_BIND_SCALE))
            .store();
        let root: [f32; 16] = mem.read(chr + 0x1d0);
        while n > 0 {
            let elem = i32::from(mem.read::<i16>(e + 2));
            #[allow(clippy::cast_sign_loss)] // movsx, then masked
            let id = i32::from(mem.read::<i16>(e)) as u32 & 0x3fff;
            let elems: u32 = mem.read(chr + l::ELEMS);
            let g = elems.wrapping_add(elem.cast_unsigned().wrapping_mul(l::ELEM_SIZE)) + 0x88;
            let mut unit = ffx_ps2::vu0::identity();
            unit[15] = crate::x87::load(K_1000.get(mem));
            let global: [f32; 16] = mem.read(g);
            let mut local = mul4x4(&global, &unit);
            local[15] = 1.0;
            let m = mul4x4(&root, &local);
            let p = chr.wrapping_add(OPTPOS).wrapping_add(id * 16);
            match mem.read::<u16>(e) & 0xc000 {
                0 | 0x8000 => {
                    mem.write(p, [12, 13, 14].map(|i| crate::x87::load(m[i])));
                }
                0x4000 => {
                    let rk = Reg::load(k);
                    let v = [
                        Reg::load(f(mem, e + 4)).mul_st(rk).store(),
                        Reg::load(f(mem, e + 8)).mul_st(rk).store(),
                        rk.mul_m32(f(mem, e + 0xc)).store(),
                        1.0,
                    ];
                    let r: [f32; 4] = std::array::from_fn(|i| {
                        #[allow(clippy::cast_possible_truncation)] // 0..4
                        let i = i as u32;
                        let col = [0, 4, 8, 12].map(|row| m[(row + i) as usize]);
                        mul4x4_component(i, v, col)
                    });
                    mem.write(p, r);
                }
                _ => {}
            }
            put_f(
                mem,
                chr.wrapping_add(OPTPOS + 12).wrapping_add(id * 16),
                1.0,
            );
            e = e.wrapping_add(16);
            n -= 1;
        }
        let half = Reg::constant(K_HALF.get(mem));
        let fv = |v: [u32; 4]| v.map(f32::from_bits);
        let a = optpos(mem, chr, 0);
        mem.write(chr + 0x6b4, a);
        let cam = Reg::load(f(mem, chr + 0x6b8))
            .sub_m32(f(mem, chr + l::CAM_HEIGHT))
            .store();
        put_f(mem, chr + 0x6b8, cam);
        let mut a = fv(optpos(mem, chr, 7));
        let mut bv = fv(optpos(mem, chr, 8));
        a[0] = Reg::load(a[0]).add_m32(bv[0]).store();
        a[2] = Reg::load(a[2]).add_m32(bv[2]).store();
        a[0] = Reg::load(a[0]).mul_st(half).store();
        a[2] = half.mul_m32(a[2]).store();
        // the higher of the two; NaN takes position 8's
        if bv[1].partial_cmp(&a[1]) != Some(std::cmp::Ordering::Less) {
            a[1] = crate::x87::load(bv[1]);
        }
        bv = fv(optpos(mem, chr, 1));
        bv[0] = Reg::load(a[0]).add_m32(bv[0]).store();
        let old: [u32; 4] = mem.read(chr + 0x694);
        mem.write(chr + 0x684, old);
        bv[2] = Reg::load(a[2]).add_m32(bv[2]).store();
        bv[0] = Reg::load(bv[0]).mul_st(half).store();
        bv[2] = half.mul_m32(bv[2]).store();
        a[0] = crate::x87::load(bv[0]);
        a[2] = crate::x87::load(bv[2]);
        mem.write(chr + 0x694, [a[0], a[1], a[2], a[3]]);
        mem.write(chr + 0x6a4, bv);
        let d = Reg::load(f(mem, chr + 0x6a8)).sub_m32(f(mem, chr + 0x698));
        let d = if d.get() < 0.0 { d.chs() } else { d }.store();
        put_f(mem, chr + 0x510, d);
        let h = Reg::load(f(mem, chr + l::POS + 4)).sub_m32(f(mem, chr + 0x698));
        let h = if h.get() < 0.0 { h.chs() } else { h }.store();
        put_f(mem, chr + 0x514, h);
        let t = Reg::load(a[1])
            .sub_st(Reg::load(d).mul_st(Reg::constant(K_0_8.get(mem))))
            .store();
        mem.write(chr + 0x6c4, [a[0], t, a[2], a[3]]);
    }

    /// `Ch_OptposGetMatrix(chr, id, offset, out)` (`0x008356b0`):
    /// `Ch_ElemGetAttr` for point `id` (offset into `offset`), then that
    /// element's `Ch_ElemGetMatrix` into `out`.
    pub fn ch_optpos_get_matrix(mem: &mut impl Mem, chr: u32, id: u32, offset: u32, out: u32) {
        let (_, e) = elem_get_attr(mem, chr, id, None, offset);
        crate::matrix::bridge::ch_elem_get_matrix(mem, chr, e, out);
    }

    /// `op_et_bindeff2_walk(chr, pos, 0, 0)` (`0x007fb740`), called through
    /// the game.
    pub const OP_ET_BINDEFF2_WALK: u32 = 0x007f_b740;
    /// Bytes of scratch [`ch_sound_foot_effect`] needs: the two feet
    /// (`[ebp-0x24]`, `[ebp-0x14]`).
    pub const FOOT_EFFECT_SCRATCH: u32 = 0x20;

    /// `Ch_SoundFootEffect_008338d0(chr)` (name ours; caller `Ch_Sound`'s
    /// footstep events): optional positions 8 and 7 (`Ch_OptposGet`, the
    /// feet) into `scratch + 0x10` and `scratch`; the walk effect
    /// (`op_et_bindeff2_walk(chr, foot, 0, 0)`) at position 8 when 7's y is
    /// below 8's, else at 7 (also when either y is NaN). EAX is the
    /// effect's.
    pub fn ch_sound_foot_effect<M: Mem>(
        mem: &mut M,
        chr: u32,
        scratch: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let (seven, eight) = (scratch, scratch + 0x10);
        ch_optpos_get(mem, chr, 8, eight);
        ch_optpos_get(mem, chr, 7, seven);
        let (y8, y7) = (f(mem, eight + 4), f(mem, seven + 4));
        let foot = if y7 < y8 { eight } else { seven };
        call(mem, mem.rebase(OP_ET_BINDEFF2_WALK), &[chr, foot, 0, 0])
    }

    /// `Ch_IdGetSound(chr)` (`0x0083d900`), `SndKickFoot(kind, party,
    /// sound, variation)` (`0x008866b0`), `Ch_SoundFootEffect_008338d0`,
    /// `Ch_GetDumpMotEv()` (`0x00833f50`), `dbgPrintf` (`0x0062f500`) and
    /// `MsChrSePlayAlive_0079fe30(i, kind)`: through the game.
    pub const ID_GET_SOUND: u32 = 0x0083_d900;
    pub const SND_KICK_FOOT: u32 = 0x0088_66b0;
    pub const SOUND_FOOT_EFFECT: u32 = 0x0083_38d0;
    pub const GET_DUMP_MOT_EV: u32 = 0x0083_3f50;
    pub const DBG_PRINTF: u32 = 0x0062_f500;
    pub const CHR_SE_PLAY_ALIVE: u32 = 0x0079_fe30;
    /// `g_SgFrame`, the main loop's frame counter (the debug lines print it).
    pub const SG_FRAME: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x023c_bbf0);
    /// The debug lines: `"SG:%s:%d:SndKickFoot( ATEL_SND_FOOT_SWIM%c,..."`
    /// and `"... ATEL_SND_FOOT_TYPE_SOUNDEVENT,%d,0,%d)"`.
    pub const SWIM_FMT: u32 = 0x00b5_9888;
    pub const SOUND_EVENT_FMT: u32 = 0x00b5_98c0;
    /// The character's footstep counters (`+0x82c`): bits 0-1 walking,
    /// bit 2 kind 3, bits 3-4 swimming.
    pub const FOOT_STEPS: u32 = 0x82c;

    /// The party byte `Ch_Sound` passes on (`i8` `+0x184`).
    fn sound_party(mem: &impl Mem, chr: u32) -> u32 {
        i32::from(mem.read::<i8>(chr.wrapping_add(0x184))).cast_unsigned()
    }

    /// `SndKickFoot(kind, party, Ch_IdGetSound(chr), var)`: `var` is the
    /// word pushed for `Ch_IdGetSound` and left on the stack.
    fn kick_foot<M: Mem>(
        mem: &mut M,
        chr: u32,
        kind: u32,
        var: u32,
        call: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let sound = call(mem, mem.rebase(ID_GET_SOUND), &[chr, var]);
        let p = sound_party(mem, chr);
        call(mem, mem.rebase(SND_KICK_FOOT), &[kind, p, sound, var]);
    }

    /// `Ch_Sound(chr, ev)` (`0x00835860`; callers `Ch_MotEvent`,
    /// `0x00835bb0`): a motion's sound event. With `ev`'s second byte 1,
    /// the battle sound `MsChrSePlayAlive(chr +0x18c, low byte)` and, for
    /// 6, the footstep effect. With 0, by the low byte (jump table
    /// `0x00835b0c`, 1..=0x11):
    /// - 1, 2: a step (`SndKickFoot` kind 1 or 0) with the walking counter
    ///   (bits 0-1 of `+0x82c`, stepped on first), then the footstep
    ///   effect; 10, 11 the same as 2 without the effect;
    /// - 3: kind 4 with bit 2 (flipped);
    /// - 4: swimming, kind 3 when flag bit 29 (`+0x194`) is set else 2,
    ///   with bits 3-4 (stepped on); a debug line first while motion
    ///   events are dumped;
    /// - 5: kind 5 with sound slot 0;
    /// - 0xc..=0x11: kind `0xff` with sound 0 and variation `ev - 10`
    ///   (a debug line first while dumped).
    ///
    /// The sound is `Ch_IdGetSound(chr)`; `SndKickFoot` reads a fourth
    /// word the original never pushed for it: the counter value it pushed
    /// for `Ch_IdGetSound` and did not pop. The party is `i8` `+0x184`.
    /// No caller reads `eax`.
    pub fn ch_sound<M: Mem>(
        mem: &mut M,
        chr: u32,
        ev: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let g = |mem: &M, f: u32| mem.rebase(f);
        let low = ev & 0xff;
        let high = (ev >> 8) & 0xff;
        let steps = chr.wrapping_add(FOOT_STEPS);
        let party = |mem: &M| sound_party(mem, chr);
        let kick =
            |mem: &mut M, call: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32, kind: u32, var: u32| {
                kick_foot(mem, chr, kind, var, call);
            };
        if high != 0 {
            if high == 1 {
                let i: u32 = mem.read(chr.wrapping_add(0x18c));
                call(mem, g(mem, CHR_SE_PLAY_ALIVE), &[i, low]);
                if low == 6 {
                    call(mem, g(mem, SOUND_FOOT_EFFECT), &[chr]);
                }
            }
            return;
        }
        match low {
            1 | 2 | 10 | 11 => {
                let old: u32 = mem.read(steps);
                mem.write(steps, ((old.wrapping_add(1) ^ old) & 3) ^ old);
                kick(mem, call, u32::from(low == 1), old & 3);
                if low <= 2 {
                    call(mem, g(mem, SOUND_FOOT_EFFECT), &[chr]);
                }
            }
            3 => {
                // the original's ((old & !3) - 4 ^ old) & 4 ^ old: bit 2 flipped
                let old: u32 = mem.read(steps);
                mem.write(steps, old ^ 4);
                kick(mem, call, 4, (old >> 2) & 1);
            }
            4 => {
                let swim = |mem: &M| mem.read::<u32>(chr.wrapping_add(0x194)) & 0x2000_0000 != 0;
                if call(mem, g(mem, GET_DUMP_MOT_EV), &[]) != 0 {
                    let var = (mem.read::<u32>(steps) >> 3) & 3;
                    let sound = call(mem, g(mem, ID_GET_SOUND), &[chr, var]);
                    let p = party(mem);
                    let c = 0x31 + u32::from(swim(mem));
                    let (name, frame) = (mem.read(chr.wrapping_add(4)), SG_FRAME.get(mem));
                    call(
                        mem,
                        g(mem, DBG_PRINTF),
                        &[g(mem, SWIM_FMT), name, frame, c, p, sound, var],
                    );
                }
                let old: u32 = mem.read(steps);
                mem.write(steps, (((old & !7).wrapping_add(8) ^ old) & 0x18) ^ old);
                let kind = if swim(mem) { 3 } else { 2 };
                kick(mem, call, kind, (old >> 3) & 3);
            }
            5 => kick(mem, call, 5, 0),
            0xc..=0x11 => {
                let var = low - 10;
                if call(mem, g(mem, GET_DUMP_MOT_EV), &[]) != 0 {
                    let p = party(mem);
                    let (name, frame) = (mem.read(chr.wrapping_add(4)), SG_FRAME.get(mem));
                    call(
                        mem,
                        g(mem, DBG_PRINTF),
                        &[g(mem, SOUND_EVENT_FMT), name, frame, p, var],
                    );
                }
                let p = party(mem);
                call(mem, g(mem, SND_KICK_FOOT), &[0xff, p, 0, var]);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds() {
        assert_eq!(attr_kind(0x4001), (1, true));
        assert_eq!(attr_kind(0x8001), (2, false));
        assert_eq!(attr_kind(0xc001), (0, false));
        assert_eq!(attr_kind(0x0001), (0, false));
    }
}
