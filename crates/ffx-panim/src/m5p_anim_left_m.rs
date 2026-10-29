//! List `phyre-anim-left`, batch 13 (M5 stage 1):
//! `PAnimationChannelTargetBindDataMember::bindTargets` (`0x005cdba0`):
//! a target's dotted member path (`a.b.c`) resolved through the class
//! descriptors from the bound object, down to a scalar member or an
//! element of an array member. Each path segment is copied into a
//! `PString` in the caller's frame (`frame`).

use ffx_graphic::host::{Conv, Host};

use crate::m5p_anim_small::PHYRE_FREE;

/// MSVCR110's `strchr` import slot.
pub const STRCHR: u32 = 0x00b0_c3c0;
/// The name copy of a text range `0x0050c7e0` (in this list), and
/// `Phyre_PString_Assign` (thiscall `(text)`).
pub const ASSIGN_RANGE: u32 = 0x0050_c7e0;
/// See [`ASSIGN_RANGE`].
pub const STRING_ASSIGN: u32 = 0x0044_5cf0;
/// `Phyre_PClassDescriptor_findMember_00435ca0` (thiscall `(name)`).
pub const FIND_MEMBER: u32 = 0x0043_5ca0;
/// `Phyre_PClassDataMember_getStoredSize_0043cd30` (thiscall).
pub const STORED_SIZE: u32 = 0x0043_cd30;
/// `Phyre::PhyrePrintf(level, fmt, ...)` (cdecl).
pub const PRINTF: u32 = 0x0043_52f0;
/// "Not binding %s:%s because array index (%d) exceeds array size (%d)".
pub const INDEX_FMT: u32 = 0x00b3_8808;
/// The `PString` word the path segment takes (`ebp - 0x14`).
pub const FRAME_WORDS: u32 = 1;

pub mod bridge {
    use super::{
        ASSIGN_RANGE, Conv, FIND_MEMBER, Host, INDEX_FMT, PHYRE_FREE, PRINTF, STORED_SIZE, STRCHR,
        STRING_ASSIGN,
    };
    use crate::m5p_anim_left_a::text;

    /// A member's address in its object: its `+0x24` less its class's
    /// base (`[[member + 0xc] + 0x88]`).
    fn member_offset(h: &impl Host, m: u32) -> u32 {
        let off: u32 = h.read(m.wrapping_add(0x24));
        let class: u32 = h.read(m.wrapping_add(0xc));
        off.wrapping_sub(h.read(class.wrapping_add(0x88)))
    }

    /// `0x005cdba0` (`thiscall(out, targets, n)`, `ret 0xc`): 0 for none,
    /// 1 without targets or outputs. Each unbound output (0x18 bytes) of a
    /// type-2 target (0x1c bytes) whose class is on the bind's class
    /// chain (`[this + 4]`, through `+0x40`) and that has an object (the
    /// bind's `+8`, else the target's `+4`) and a path (`+0x14`): the
    /// path's segments looked up member by member (each segment a
    /// `PString` in the frame, freed when owned), stepping into each
    /// member's class (its `+0x18` slot 1) and address. The last member
    /// binds: without an array type (its slot 1's `+0x30`) its address
    /// and stored size; for array kinds 1-4 element `+0x18` of it when in
    /// range (the type's slots 4 count, 9 element type `+0x1c` size, 6
    /// data), else a warning. Bound outputs get `+0` 1, `+4` the address,
    /// `+8` the size up to 16; 0.
    #[allow(clippy::too_many_lines)]
    pub fn bind_members(
        h: &mut impl Host,
        this: u32,
        out: u32,
        targets: u32,
        n: u32,
        frame: u32,
    ) -> u32 {
        if n == 0 {
            return 0;
        }
        if targets == 0 || out == 0 {
            return 1;
        }
        let slot = h.rebase(STRCHR);
        for k in 0..n {
            let t = targets.wrapping_add(0x1c * k);
            let o = out.wrapping_add(0x18 * k);
            if h.read::<u32>(t.wrapping_add(0x10)) != 2 {
                continue;
            }
            let head: u32 = h.read(this.wrapping_add(4));
            let want: u32 = h.read(t);
            let mut cls = head;
            while cls != 0 && cls != want {
                cls = h.read(cls.wrapping_add(0x40));
            }
            if cls == 0 || h.read::<u32>(o) != 0 {
                continue;
            }
            let mut obj: u32 = h.read(this.wrapping_add(8));
            if obj == 0 {
                obj = h.read(t.wrapping_add(4));
            }
            let mut path = text(h, t.wrapping_add(0x14));
            if obj == 0 || path == 0 {
                continue;
            }
            let mut class = head;
            let mut parent = obj;
            let mut member = 0u32;
            while h.read::<u8>(path) != 0 {
                let strchr: u32 = h.read(slot);
                let dot = h.call(strchr, Conv::Cdecl, &[path, u32::from(b'.')]);
                h.write(frame, 0u32);
                let rest = if dot == 0 {
                    h.call_sym(STRING_ASSIGN, Conv::Thiscall(frame), &[path]);
                    0
                } else {
                    h.call_sym(ASSIGN_RANGE, Conv::Thiscall(frame), &[path, dot]);
                    dot.wrapping_add(1)
                };
                let word: u32 = h.read(frame);
                let segment = word.wrapping_sub(word & 1);
                member = h.call_sym(FIND_MEMBER, Conv::Thiscall(class), &[segment]);
                if member != 0 {
                    let ty: u32 = h.read(member.wrapping_add(0x18));
                    class = h.call_virtual(ty, 1, &[]);
                    parent = obj;
                    obj = obj.wrapping_add(member_offset(h, member));
                }
                if word & 1 == 0 {
                    h.call_sym(PHYRE_FREE, Conv::Cdecl, &[word]);
                }
                if member == 0 {
                    break;
                }
                path = rest;
                if path == 0 {
                    break;
                }
            }
            if member == 0 {
                continue;
            }
            let array = h.call_virtual(member, 1, &[]);
            let kind = if array == 0 {
                0
            } else {
                h.read::<u32>(array.wrapping_add(0x30))
            };
            let (addr, size) = if kind == 0 {
                let addr = member_offset(h, member).wrapping_add(parent);
                (addr, h.call_sym(STORED_SIZE, Conv::Thiscall(member), &[]))
            } else if kind.wrapping_sub(1) > 3 {
                continue;
            } else {
                let idx: u32 = h.read(t.wrapping_add(0x18));
                let cnt = h.call_virtual(array, 4, &[parent]);
                if idx >= cnt {
                    let fmt = h.rebase(INDEX_FMT);
                    let class_name: u32 = h.read(head.wrapping_add(0x18));
                    let name = text(h, t.wrapping_add(0x14));
                    h.call_sym(PRINTF, Conv::Cdecl, &[2, fmt, class_name, name, idx, cnt]);
                    continue;
                }
                let el = h.call_virtual(array, 9, &[]);
                let size: u32 = h.read(el.wrapping_add(0x1c));
                let base = h.call_virtual(array, 6, &[parent]);
                (base.wrapping_add(idx.wrapping_mul(size)), size)
            };
            if addr != 0 {
                h.write(o, 1u32);
                h.write(o.wrapping_add(4), addr);
                h.write(o.wrapping_add(8), size.min(0x10));
            }
        }
        0
    }
}
