//! List `phyre-anim-left`, batch 8 (M5 stage 1): `PAnimationSet`'s slot
//! list (`0x0051de90`): one 16-byte slot per (channel class, target) its
//! clips animate, sorted by batch and indexed back per target.
//!
//! The target lookup it consults lives in the caller's frame (`frame`);
//! its per-target table (an `alloca` of flag, count and offset per
//! target) is per-call scratch here.

#![allow(clippy::many_single_char_names)]

use ffx_bridge::Mem;
use ffx_graphic::host::{Conv, Host};

use crate::m5p_anim_small::NOT_OWNED;

/// The target lookup's constructor, key and destructor (thiscall).
pub const LOOKUP_CTOR: u32 = 0x005c_ca80;
/// See [`LOOKUP_CTOR`].
pub const LOOKUP_KEY: u32 = 0x005c_ce50;
/// See [`LOOKUP_CTOR`].
pub const LOOKUP_DTOR: u32 = 0x005c_cc50;
/// The slot array's resize `0x0051d670` (thiscall on `+0x10`, `(n)`).
pub const SLOTS_RESIZE: u32 = 0x0051_d670;
/// MSVCR110's `qsort` import slot, and the slot order (`0x00508ba0`).
pub const QSORT: u32 = 0x00b0_c3ac;
/// See [`QSORT`].
pub const SLOT_ORDER: u32 = 0x0050_8ba0;
/// How many channel classes there are.
pub const CLASSES: u32 = 0x00ca_9738;
/// How many words of the caller's frame the lookup takes (`ebp -
/// 0x60`).
pub const FRAME_WORDS: u32 = 8;

pub mod bridge {
    use super::{
        CLASSES, Conv, Host, LOOKUP_CTOR, LOOKUP_DTOR, LOOKUP_KEY, Mem, NOT_OWNED, QSORT,
        SLOT_ORDER, SLOTS_RESIZE,
    };

    fn count(m: &impl Mem, at: u32) -> u32 {
        m.read::<u32>(at) & !NOT_OWNED
    }

    /// Clip `c` of the set's clip array (one kept in place at `+4`).
    fn clip(m: &impl Mem, set: u32, c: u32) -> u32 {
        let n = count(m, set);
        let data = if n > 1 {
            m.read(set.wrapping_add(4))
        } else if n != 0 {
            set.wrapping_add(4)
        } else {
            0
        };
        m.read(data.wrapping_add(4 * c))
    }

    /// One target of a pass: what the target is, and what to record.
    struct Hit {
        target: u32,
        class: u32,
        word: u32,
        /// Constant channels' targets are range-checked.
        checked: bool,
    }

    /// Every channel (`[clip + 8]`, count `+4`) and constant channel
    /// (`0x34` bytes at `+0x10`, count `+0xc`) of the clips whose class
    /// (`+0x1c`) is class `g`, with its target from the lookup.
    fn visit(
        h: &mut impl Host,
        set: u32,
        lookup: u32,
        g: u32,
        n_clips: u32,
        f: &mut dyn FnMut(Hit),
    ) {
        for c in 0..n_clips {
            let cl = clip(h, set, c);
            if cl == 0 {
                continue;
            }
            let mut k = 0u32;
            while k < count(h, cl.wrapping_add(4)) {
                let list: u32 = h.read(cl.wrapping_add(8));
                let ch: u32 = h.read(list.wrapping_add(4 * k));
                k += 1;
                if ch == 0 {
                    continue;
                }
                let class: u32 = h.read(ch.wrapping_add(0x1c));
                if class == 0 || h.read::<u32>(class.wrapping_add(0xc)) != g {
                    continue;
                }
                let target = h.call_sym(LOOKUP_KEY, Conv::Thiscall(lookup), &[ch]);
                let word: u32 = h.read(ch.wrapping_add(0x20));
                f(Hit {
                    target,
                    class,
                    word,
                    checked: false,
                });
            }
            let mut j = 0u32;
            while j < count(h, cl.wrapping_add(0xc)) {
                let recs: u32 = h.read(cl.wrapping_add(0x10));
                let cc = recs.wrapping_add(0x34 * j);
                j += 1;
                let class: u32 = h.read(cc.wrapping_add(0x1c));
                if class == 0 || h.read::<u32>(class.wrapping_add(0xc)) != g {
                    continue;
                }
                let target = h.call_sym(LOOKUP_KEY, Conv::Thiscall(lookup), &[cc]);
                let word: u32 = h.read(cc.wrapping_add(0x20));
                f(Hit {
                    target,
                    class,
                    word,
                    checked: true,
                });
            }
        }
    }

    /// `0x0051de90` (`thiscall`): `PAnimationSet`'s slots: with the
    /// target lookup made over the set in the frame, the targets each
    /// class animates are counted per target (once per class), the slot
    /// array (`+0x10`) resized to the total ([`SLOTS_RESIZE`], its error
    /// returned; 0xd without storage), filled with `{class, +0x20,
    /// target}` per class in order, sorted by batch (`qsort`,
    /// [`SLOT_ORDER`]), and each slot's number stored at its target's next
    /// place (`+0xc` of the slot there). The lookup destroyed; 0.
    pub fn build_slots(h: &mut impl Host, this: u32, frame: u32) -> u32 {
        let n_targets = count(h, this.wrapping_add(8));
        let n_clips = count(h, this);
        h.call_sym(LOOKUP_CTOR, Conv::Thiscall(frame), &[this]);
        // per target: flag, count, offset
        let mut flag = vec![false; n_targets as usize];
        let mut cnt = vec![0u8; n_targets as usize];
        let mut off = vec![0u16; n_targets as usize];
        let n = n_targets as usize;
        let classes_at = h.rebase(CLASSES);
        let mut g = 0u32;
        while g < h.read::<u32>(classes_at) {
            flag.fill(false);
            visit(h, this, frame, g, n_clips, &mut |hit| {
                // a target past the table lands outside the original's
                // `alloca` (its own frame); nothing here counts it
                let t = hit.target as usize;
                if t >= n {
                    return;
                }
                if !flag[t] {
                    cnt[t] = cnt[t].wrapping_add(1);
                    flag[t] = true;
                }
            });
            g += 1;
        }
        let mut total = 0u32;
        for (o, &c) in off.iter_mut().zip(&cnt).take(n_targets as usize) {
            #[allow(clippy::cast_possible_truncation)] // the u16 store
            {
                *o = total as u16;
            }
            total = total.wrapping_add(u32::from(c));
        }
        let r = h.call_sym(
            SLOTS_RESIZE,
            Conv::Thiscall(this.wrapping_add(0x10)),
            &[total],
        );
        let result = if r != 0 {
            r
        } else if total == 0 {
            0
        } else {
            let arr: u32 = h.read(this.wrapping_add(0x14));
            if arr == 0 {
                0xd
            } else {
                let mut idx = 0u32;
                let mut g = 0u32;
                while g < h.read::<u32>(classes_at) {
                    flag.fill(false);
                    let mut out: Vec<(u32, u32, u32)> = Vec::new();
                    visit(h, this, frame, g, n_clips, &mut |hit| {
                        let t = hit.target;
                        if hit.checked
                            && (t.cast_signed() < 0 || t.cast_signed() >= n_targets.cast_signed())
                        {
                            return;
                        }
                        if t as usize >= n {
                            return;
                        }
                        if !flag[t as usize] {
                            flag[t as usize] = true;
                            out.push((hit.class, hit.word, t));
                        }
                    });
                    for (class, word, t) in out {
                        let e = arr.wrapping_add(16 * idx);
                        h.write(e.wrapping_add(4), word);
                        h.write(e, class);
                        h.write(e.wrapping_add(8), t);
                        idx += 1;
                    }
                    g += 1;
                }
                let qsort_slot = h.rebase(QSORT);
                let qsort: u32 = h.read(qsort_slot);
                let order = h.rebase(SLOT_ORDER);
                h.call(qsort, Conv::Cdecl, &[arr, total, 16, order]);
                for i in 0..total {
                    let t: u32 = h.read(arr.wrapping_add(16 * i + 8));
                    let Some(&place) = off.get(t as usize) else {
                        continue;
                    };
                    off[t as usize] = place.wrapping_add(1);
                    h.write(arr.wrapping_add(u32::from(place) * 16 + 0xc), i);
                }
                0
            }
        };
        h.call_sym(LOOKUP_DTOR, Conv::Thiscall(frame), &[]);
        result
    }
}
