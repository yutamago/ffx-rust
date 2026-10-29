//! List `phyre-anim-left`, batch 9 (M5 stage 1): `PAnimationSet`'s
//! target list rebuilt from its clips (`0x0051c490`): the distinct
//! channel targets of every clip, the set's target array remade from
//! them, the slots rebuilt and every clip's data rebuilt in the cluster.

use ffx_bridge::Mem;
use ffx_graphic::host::{Conv, Host};

use crate::m5p_anim_small::{NOT_OWNED, PHYRE_FREE};

/// `Phyre::PhyreMalloc(size, align)` (cdecl).
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
/// The old target array's release (cdecl `(targets, count)`).
pub const TARGETS_RELEASE: u32 = 0x0051_a890;
/// A target's comparison with another (thiscall, `al`).
pub const TARGET_EQUALS: u32 = 0x0050_b7b0;
/// A target's assignment (thiscall `(other)`).
pub const TARGET_ASSIGN: u32 = 0x0050_b730;
/// The named targets' construction `0x0051bb80`, the slots `0x0051de90`
/// and each clip's data rebuild `0x00516240` (in this list).
pub const TARGETS_CONSTRUCT: u32 = 0x0051_bb80;
/// See [`TARGETS_CONSTRUCT`].
pub const BUILD_SLOTS: u32 = 0x0051_de90;
/// See [`TARGETS_CONSTRUCT`].
pub const CLIP_REBUILD: u32 = 0x0051_6240;

pub mod bridge {
    use super::{
        BUILD_SLOTS, CLIP_REBUILD, Conv, Host, Mem, NOT_OWNED, PHYRE_FREE, PHYRE_MALLOC,
        TARGET_ASSIGN, TARGET_EQUALS, TARGETS_CONSTRUCT, TARGETS_RELEASE,
    };

    fn count(m: &impl Mem, at: u32) -> u32 {
        m.read::<u32>(at) & !NOT_OWNED
    }

    fn free(h: &mut impl Host, p: u32) {
        h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p]);
    }

    /// Clip `c` of the set's clips (one kept in place), 0 past the count.
    fn clip(m: &impl Mem, set: u32, c: u32) -> u32 {
        let n = count(m, set);
        if c >= n {
            return 0;
        }
        let data = if n > 1 {
            m.read(set.wrapping_add(4))
        } else {
            set.wrapping_add(4)
        };
        m.read(data.wrapping_add(4 * c))
    }

    /// `item` appended to the `n` distinct targets at `list` unless one
    /// already equals it ([`TARGET_EQUALS`] on each held one).
    fn add_distinct(h: &mut impl Host, list: u32, n: &mut u32, item: u32) {
        for i in 0..*n {
            let held: u32 = h.read(list.wrapping_add(4 * i));
            if h.call_sym(TARGET_EQUALS, Conv::Thiscall(held), &[item])
                .trailing_zeros()
                < 8
            {
                return;
            }
        }
        h.write(list.wrapping_add(4 * *n), item);
        *n += 1;
    }

    /// `0x0051c490` (`thiscall(cluster)`, `ret 4`): the set's targets
    /// rebuilt: the old target array (`+8` count, `+0xc` data) released
    /// when set; the distinct targets of all clips' channels and constant
    /// channels gathered in a zeroed `PhyreMalloc` list; the target array
    /// remade to that size when it differs (`0x1c`-byte named targets,
    /// the old ones' owned names freed; 0xd when the allocation fails)
    /// and assigned; the slots rebuilt (their error returned). The list
    /// freed, then (also when there are no targets) every clip's data
    /// rebuilt in `cluster`; 0.
    pub fn rebuild_targets(h: &mut impl Host, this: u32, cluster: u32) -> u32 {
        let n_clips = count(h, this);
        let raw: u32 = h.read(this.wrapping_add(8));
        if raw != 0 {
            let data: u32 = h.read(this.wrapping_add(0xc));
            if data != 0 && raw & NOT_OWNED == 0 {
                h.call_sym(TARGETS_RELEASE, Conv::Cdecl, &[data, raw & !NOT_OWNED]);
            }
            h.write(this.wrapping_add(0xc), 0u32);
            h.write(this.wrapping_add(8), 0u32);
        }
        let mut total = 0u32;
        for c in 0..n_clips {
            let cl = clip(h, this, c);
            total = total.wrapping_add(count(h, cl.wrapping_add(4)));
            let cl = clip(h, this, c);
            total = total.wrapping_add(count(h, cl.wrapping_add(0xc)));
        }
        if total != 0 {
            let list = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[total.wrapping_mul(4), 4]);
            if list == 0 {
                free(h, 0);
            } else {
                for i in 0..total {
                    h.write(list.wrapping_add(4 * i), 0u32);
                }
                let r = fill(h, this, list, n_clips);
                free(h, list);
                if r != 0 {
                    return r;
                }
            }
        }
        for c in 0..n_clips {
            let cl = clip(h, this, c);
            h.call_sym(CLIP_REBUILD, Conv::Thiscall(cl), &[cluster, this]);
        }
        0
    }

    /// The distinct targets into `list`, the target array remade and
    /// assigned, the slots rebuilt; the first error.
    fn fill(h: &mut impl Host, this: u32, list: u32, n_clips: u32) -> u32 {
        let mut n = 0u32;
        for c in 0..n_clips {
            let cl = clip(h, this, c);
            if cl == 0 {
                continue;
            }
            let chans = count(h, cl.wrapping_add(4));
            for k in 0..chans {
                let items: u32 = h.read(cl.wrapping_add(8));
                let item: u32 = h.read(items.wrapping_add(4 * k));
                add_distinct(h, list, &mut n, item);
            }
            let consts = count(h, cl.wrapping_add(0xc));
            for j in 0..consts {
                let recs: u32 = h.read(cl.wrapping_add(0x10));
                add_distinct(h, list, &mut n, recs.wrapping_add(0x34 * j));
            }
        }
        if n != h.read::<u32>(this.wrapping_add(8)) {
            let mut new = 0;
            if n != 0 {
                new = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[n.wrapping_mul(0x1c), 4]);
                if new == 0 {
                    return 0xd;
                }
                h.call_sym(TARGETS_CONSTRUCT, Conv::Cdecl, &[new, n]);
            }
            let raw: u32 = h.read(this.wrapping_add(8));
            let old: u32 = h.read(this.wrapping_add(0xc));
            if old != new && raw & NOT_OWNED == 0 && old != 0 {
                for k in 0..raw & !NOT_OWNED {
                    let name: u32 = h.read(old.wrapping_add(0x14 + 0x1c * k));
                    if name & 1 == 0 {
                        free(h, name);
                    }
                }
                free(h, old);
            }
            h.write(this.wrapping_add(0xc), new);
            h.write(this.wrapping_add(8), n);
        }
        for i in 0..n {
            let data: u32 = h.read(this.wrapping_add(0xc));
            let item: u32 = h.read(list.wrapping_add(4 * i));
            h.call_sym(
                TARGET_ASSIGN,
                Conv::Thiscall(data.wrapping_add(0x1c * i)),
                &[item],
            );
        }
        h.call_sym(BUILD_SLOTS, Conv::Thiscall(this), &[])
    }
}
