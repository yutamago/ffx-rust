//! `yiKeyFrame_main` (`0x00914a10`): one frame of the field renderer's key
//! frame tracks (`docs/formats/field.md`, "Key frame tracks").
//!
//! The render block (`g_YiRender`) holds 64 tracks of 0x28 bytes at
//! `+0x9cac0`; each moves one display object (`+0x6879c + 0xb4 · object`:
//! matrix `+0`, rotation `+0x40`, position `+0x50`, rotation order `+0x76`,
//! 2D offset `+0x88`) or a texture animation (`+0x9d9c8 + 0x720 · i`).
//! The function keeps its vectors and matrices in its frame and hands
//! their addresses to the callees; [`bridge::yi_key_frame_main`] uses a
//! work area laid out the same way (`work + WORK - k` is the original's
//! `[ebp - k]`).

/// The work area [`bridge::yi_key_frame_main`] needs, in bytes (the
/// original's frame below the cookie).
pub const WORK: u32 = 0x198;

/// The callees, by address.
pub mod callee {
    /// `yiGetFCount()`.
    pub const GET_FCOUNT: u32 = 0x0090_8970;
    /// `yiGetVCount()`.
    pub const GET_VCOUNT: u32 = 0x0090_8f70;
    /// `Ch_ElemGetMatrix(chr, elem, out)`.
    pub const ELEM_GET_MATRIX: u32 = 0x0083_3c30;
    /// `FFXVu0UnitMatrix(m)`.
    pub const UNIT_MATRIX: u32 = 0x006e_d9a0;
    /// `Ch_GetDataScale(chr)` (`ST0`).
    pub const GET_DATA_SCALE: u32 = 0x0082_ab60;
    /// `FFXVu0MulMatrix(out, a, b)`.
    pub const MUL_MATRIX: u32 = 0x006e_d5c0;
    /// `yiRotMatrixSeq(m, angles, order)`.
    pub const ROT_MATRIX_SEQ: u32 = 0x0091_51c0;
    /// `graphicObjectSetRotation(object, angles, order)`.
    pub const SET_ROTATION: u32 = 0x0064_26a0;
    /// `FFXSamp0TransMatrix(out, m, t)`.
    pub const TRANS_MATRIX: u32 = 0x0093_d830;
    /// `graphicObjectSetTranslation(object, pos)`.
    pub const SET_TRANSLATION: u32 = 0x0064_26c0;
    /// `yiKeyFrameAdvance(r, track, now)`.
    pub const ADVANCE: u32 = 0x0091_4180;
    /// `FFXVu0CopyVector(dst, src)`.
    pub const COPY_VECTOR: u32 = 0x006e_d370;
    /// `yiHermiteVec4(t, p0, p1, t0, t1, out)`.
    pub const HERMITE_VEC4: u32 = 0x0091_47b0;
    /// `yiLerpVec4(t, a, b, out)`.
    pub const LERP_VEC4: u32 = 0x0091_48d0;
    /// `yiKeyFrameAdvanceLinked(r, track, now)`.
    pub const ADVANCE_LINKED: u32 = 0x0091_4070;
    /// `yiKeyFrameWrapAngles(v)`.
    pub const WRAP_ANGLES: u32 = 0x0091_5120;
    /// `yiHermiteVec2(t, p0, p1, t0, t1, out)`.
    pub const HERMITE_VEC2: u32 = 0x0091_4700;
    /// `getInterValefor()` (the save's Valefor byte, in `al`).
    pub const GET_INTER_VALEFOR: u32 = 0x0078_7340;
    /// `graphicSetMaterialUV_006436e0(object, &x, &y)`.
    pub const SET_OFFSET_2D: u32 = 0x0064_36e0;
}

/// `g_YiAnimUseVCount`: the tracks time by `yiGetVCount` when set.
pub const USE_VCOUNT: u32 = 0x0193_bb08;
/// `100.0` (f64) the character scale is divided into.
pub const HUNDRED: u32 = 0x00b9_22e0;

/// [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::Mem;
    use ffx_bridge::x87::{self, Reg, ftol};

    use super::{HUNDRED, USE_VCOUNT, WORK, callee as c};

    /// `1.0f`.
    const ONE: u32 = 0x3f80_0000;

    /// `yiKeyFrame_main(r)` (`0x00914a10`): copies `r +0xba3dc` to
    /// `+0xba3e0`, takes the time (`yiGetVCount` when
    /// `g_YiAnimUseVCount`, else `yiGetFCount`), then for each of the 64
    /// tracks at `r +0x9cac0` (0x28 bytes) whose flag bit 0 is set: the
    /// progress `t = (cur - start) / key length` (as unsigned floats,
    /// at most 1; 0 for a zero length; kept from the last track when the
    /// track has no key), and by its kind (`+4`):
    /// - 0: the position (key `+2`: 0 lerp, 1 Hermite, 2 the end point,
    ///   else the vector kept from the last such track), w = 1, into the
    ///   object's position, its matrix rebuilt (rotation order `+0x76`,
    ///   then the translation), `graphicObjectSetTranslation`;
    /// - 1: the rotation the same way, wrapped (`yiKeyFrameWrapAngles`),
    ///   `graphicObjectSetRotation`;
    /// - 2: the 2D offset (two floats, 0 unless interpolated) into the
    ///   object's `+0x88`, its x 0 when `getInterValefor` has bit 2 and is
    ///   not negative and the object is not 0x15..0x1b or is 0x18, then
    ///   `graphicSetMaterialUV_006436e0`;
    /// - 3: texture animation `+6` jumps to the key's frame (`ftol` of
    ///   key `+0x40`) when that frame is set;
    /// - 4: the object's matrix is element `+0x10` of character `+0xc`'s,
    ///   scaled by `100 / Ch_GetDataScale` (no object bound check, no
    ///   advance);
    /// - 5: the object follows entry `+0xa` of the bone list `+8`
    ///   (`r +0x9d5c4`): rotation from the entry's `+8`, translation the
    ///   object's position plus the entry's `+0x14`, `yiKeyFrameAdvance`.
    ///
    /// Kinds 0, 1, 2 and 5 need the object below `r +0x68788`, kind 3 the
    /// animation below `r +0x9d9c4`; kinds 0..3 step with
    /// `yiKeyFrameAdvanceLinked`. Returns 0.
    ///
    /// A kind 0 or 1 track whose key interpolation is not 0, 1 or 2 hands
    /// on the vector last computed for that kind in this call, which the
    /// original reads from its frame unwritten on the first such track:
    /// junk read, 0 here (brief 2026-10-24).
    #[allow(clippy::too_many_lines)] // the original's six kinds in turn
    pub fn yi_key_frame_main<M, C, F>(
        mem: &mut M,
        r: u32,
        work: u32,
        call: &mut C,
        fcall: &mut F,
    ) -> u32
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
        F: FnMut(&mut M, u32, &[u32]) -> f64,
    {
        // `[ebp - o]` of the original's frame
        let fr = |o: u32| work.wrapping_add(WORK - o);
        let (elem_m, bone_m, pos_m, scale_m, rot_m) =
            (fr(0x184), fr(0x144), fr(0x104), fr(0xc4), fr(0x84));
        let (v44, v34, v24, v14) = (fr(0x44), fr(0x34), fr(0x24), fr(0x14));
        // junk read: [ebp-0x44..0x38] and [ebp-0x34..0x28] never written
        // before a kind 0/1 track with no interpolation; 0 (brief
        // 2026-10-24)
        for k in 0..4 {
            mem.write(v44.wrapping_add(4 * k), 0u32);
            mem.write(v34.wrapping_add(4 * k), 0u32);
        }
        let f = |mem: &M, at: u32| mem.read::<f32>(at);
        let obj = |i: u32| r.wrapping_add(0x6879c).wrapping_add(i.wrapping_mul(0xb4));
        let copy_matrix = |mem: &mut M, dst: u32, src: u32| {
            for k in 0..16 {
                let w = mem.read::<u32>(src.wrapping_add(4 * k));
                mem.write(dst.wrapping_add(4 * k), w);
            }
        };
        let latest = mem.read::<u32>(r.wrapping_add(0xba3dc));
        mem.write(r.wrapping_add(0xba3e0), latest);
        let clock = if mem.read::<u32>(mem.rebase(USE_VCOUNT)) == 0 {
            c::GET_FCOUNT
        } else {
            c::GET_VCOUNT
        };
        let now = call(mem, mem.rebase(clock), &[]);
        let objects = mem.read::<u32>(r.wrapping_add(0x68788));
        // the progress, kept from track to track ([ebp-0x188])
        let mut t = 0f32;
        for n in 0..0x40u32 {
            let track = r.wrapping_add(0x9cac0).wrapping_add(0x28 * n);
            if mem.read::<u16>(track) & 1 == 0 {
                continue;
            }
            let key = mem.read::<u32>(track.wrapping_add(0x24));
            if key != 0 {
                let length = mem.read::<u32>(key.wrapping_add(4));
                // fild, plus 2^32 when negative: the unsigned value
                let length = Reg::constant(f64::from(length)).store();
                t = if length == 0.0 {
                    0.0
                } else {
                    let start = mem.read::<u32>(track.wrapping_add(0x14));
                    let cur = mem.read::<u32>(track.wrapping_add(0x18));
                    let done = Reg::constant(f64::from(cur.wrapping_sub(start))).store();
                    let t = Reg::load(length).divr_m32(done).store();
                    // fcom: 1 < t only (no NaN: both operands finite)
                    if 1.0 < t { 1.0 } else { t }
                };
            }
            let tb = t.to_bits();
            let object = u32::from(mem.read::<u16>(track.wrapping_add(6)));
            let o = obj(object);
            let (rot, pos, order) = (
                o.wrapping_add(0x40),
                o.wrapping_add(0x50),
                o.wrapping_add(0x76),
            );
            let interp = |mem: &mut M, call: &mut C, out: u32, k: u16| {
                let ka = |d: u32| key.wrapping_add(d);
                match k {
                    0 => {
                        call(
                            mem,
                            mem.rebase(c::LERP_VEC4),
                            &[tb, ka(0x30), ka(0x40), out],
                        );
                    }
                    1 => {
                        let args = [tb, ka(0x30), ka(0x40), ka(0x10), ka(0x20), out];
                        call(mem, mem.rebase(c::HERMITE_VEC4), &args);
                    }
                    2 => {
                        call(mem, mem.rebase(c::COPY_VECTOR), &[out, ka(0x40)]);
                    }
                    _ => {}
                }
            };
            match mem.read::<u16>(track.wrapping_add(4)) {
                // the position
                0 => {
                    if object >= objects {
                        continue;
                    }
                    let k = mem.read::<u16>(key.wrapping_add(2));
                    interp(mem, call, v34, k);
                    mem.write(v34.wrapping_add(0xc), ONE);
                    call(mem, mem.rebase(c::COPY_VECTOR), &[pos, v34]);
                    call(mem, mem.rebase(c::UNIT_MATRIX), &[pos_m]);
                    let order = u32::from(mem.read::<u16>(order));
                    call(mem, mem.rebase(c::ROT_MATRIX_SEQ), &[pos_m, rot, order]);
                    call(mem, mem.rebase(c::TRANS_MATRIX), &[pos_m, pos_m, pos]);
                    copy_matrix(mem, o, pos_m);
                    call(mem, mem.rebase(c::SET_TRANSLATION), &[object, v34]);
                    call(mem, mem.rebase(c::ADVANCE_LINKED), &[r, track, now]);
                }
                // the rotation
                1 => {
                    if object >= objects {
                        continue;
                    }
                    let k = mem.read::<u16>(key.wrapping_add(2));
                    interp(mem, call, v44, k);
                    mem.write(v44.wrapping_add(0xc), ONE);
                    call(mem, mem.rebase(c::COPY_VECTOR), &[rot, v44]);
                    call(mem, mem.rebase(c::WRAP_ANGLES), &[rot]);
                    call(mem, mem.rebase(c::UNIT_MATRIX), &[rot_m]);
                    let ord = u32::from(mem.read::<u16>(order));
                    call(mem, mem.rebase(c::ROT_MATRIX_SEQ), &[rot_m, rot, ord]);
                    call(mem, mem.rebase(c::TRANS_MATRIX), &[rot_m, rot_m, pos]);
                    copy_matrix(mem, o, rot_m);
                    let ord = u32::from(mem.read::<u16>(order));
                    call(mem, mem.rebase(c::SET_ROTATION), &[object, v44, ord]);
                    call(mem, mem.rebase(c::ADVANCE_LINKED), &[r, track, now]);
                }
                // the 2D offset
                2 => {
                    for k in 0..4 {
                        mem.write(v14.wrapping_add(4 * k), 0u32);
                    }
                    if object >= objects {
                        continue;
                    }
                    match mem.read::<u16>(key.wrapping_add(2)) {
                        0 => {
                            for d in [0, 4] {
                                let (a, b) = (
                                    f(mem, key.wrapping_add(0x30 + d)),
                                    f(mem, key.wrapping_add(0x40 + d)),
                                );
                                let v = (Reg::load(b).sub_m32(a) * Reg::load(t)).add_m32(a).store();
                                mem.write(v14.wrapping_add(d), v);
                            }
                        }
                        1 => {
                            let ka = |d: u32| key.wrapping_add(d);
                            let args = [tb, ka(0x30), ka(0x40), ka(0x10), ka(0x20), v14];
                            call(mem, mem.rebase(c::HERMITE_VEC2), &args);
                        }
                        2 => {
                            call(
                                mem,
                                mem.rebase(c::COPY_VECTOR),
                                &[v14, key.wrapping_add(0x40)],
                            );
                        }
                        _ => {}
                    }
                    for d in [0, 4] {
                        let v = Reg::load(f(mem, v14.wrapping_add(d))).store();
                        mem.write(o.wrapping_add(0x88 + d), v);
                    }
                    let valefor = call(mem, mem.rebase(c::GET_INTER_VALEFOR), &[]);
                    if valefor & 4 != 0
                        && call(mem, mem.rebase(c::GET_INTER_VALEFOR), &[]) & 0x80 == 0
                        && !((0x15..=0x1b).contains(&object) && object != 0x18)
                    {
                        mem.write(v14, 0u32);
                    }
                    call(
                        mem,
                        mem.rebase(c::SET_OFFSET_2D),
                        &[object, v14, v14.wrapping_add(4)],
                    );
                    call(mem, mem.rebase(c::ADVANCE_LINKED), &[r, track, now]);
                }
                // a texture animation's frame
                3 => {
                    if object >= mem.read::<u32>(r.wrapping_add(0x9d9c4)) {
                        continue;
                    }
                    // (the original also tests bit 16 of the track's flag
                    // word, which its movzx always clears)
                    let frame = ftol(mem, x87::load(mem.read::<u32>(key.wrapping_add(0x40))));
                    let anim = r.wrapping_add(object.wrapping_mul(0x720));
                    let at = anim
                        .wrapping_add(frame.wrapping_mul(0x1c))
                        .wrapping_add(0x9d9f0);
                    if mem.read::<u32>(at) != 0 {
                        let flags = mem.read::<u32>(anim.wrapping_add(0x9d9c8));
                        mem.write(anim.wrapping_add(0x9d9c8), flags | 1);
                        mem.write(anim.wrapping_add(0x9d9cc), frame);
                    }
                    call(mem, mem.rebase(c::ADVANCE_LINKED), &[r, track, now]);
                }
                // a character's bone
                4 => {
                    let chr = mem.read::<u32>(track.wrapping_add(0xc));
                    let elem = mem.read::<u32>(track.wrapping_add(0x10));
                    call(mem, mem.rebase(c::ELEM_GET_MATRIX), &[chr, elem, elem_m]);
                    call(mem, mem.rebase(c::UNIT_MATRIX), &[scale_m]);
                    let scale = fcall(mem, mem.rebase(c::GET_DATA_SCALE), &[chr]);
                    let hundred = mem.read::<f64>(mem.rebase(HUNDRED));
                    let s = Reg::constant(x87::mul(x87::div(1.0, scale), hundred)).store();
                    for d in [0x28, 0x14, 0] {
                        mem.write(scale_m.wrapping_add(d), s);
                    }
                    call(mem, mem.rebase(c::MUL_MATRIX), &[elem_m, elem_m, scale_m]);
                    copy_matrix(mem, o, elem_m);
                }
                // a model's bone entry
                5 => {
                    if object >= objects {
                        continue;
                    }
                    let list = u32::from(mem.read::<u16>(track.wrapping_add(8)));
                    let entry = u32::from(mem.read::<u16>(track.wrapping_add(0xa)));
                    let e = mem
                        .read::<u32>(r.wrapping_add(0x9d5c4).wrapping_add(4 * list))
                        .wrapping_add(entry << 5);
                    call(mem, mem.rebase(c::UNIT_MATRIX), &[bone_m]);
                    mem.write(v24.wrapping_add(0xc), ONE);
                    for d in [0, 4, 8] {
                        let v = Reg::load(f(mem, e.wrapping_add(8 + d))).store();
                        mem.write(v24.wrapping_add(d), v);
                    }
                    let ord = u32::from(mem.read::<u16>(order));
                    call(mem, mem.rebase(c::ROT_MATRIX_SEQ), &[bone_m, v24, ord]);
                    let ord = u32::from(mem.read::<u16>(order));
                    call(mem, mem.rebase(c::SET_ROTATION), &[object, v24, ord]);
                    for d in [0, 4, 8] {
                        let v = Reg::load(f(mem, pos.wrapping_add(d)))
                            .add_m32(f(mem, e.wrapping_add(0x14 + d)))
                            .store();
                        mem.write(v24.wrapping_add(d), v);
                    }
                    call(mem, mem.rebase(c::TRANS_MATRIX), &[bone_m, bone_m, v24]);
                    call(mem, mem.rebase(c::SET_TRANSLATION), &[object, v24]);
                    copy_matrix(mem, o, bone_m);
                    call(mem, mem.rebase(c::ADVANCE), &[r, track, now]);
                }
                _ => {}
            }
        }
        0
    }
}
