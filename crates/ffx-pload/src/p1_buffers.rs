//! List `p1-load`, continuation round 3, batch 3: a class descriptor's
//! default and validation buffers (`PClassDescriptor::
//! calculateDefaultBuffer`, `calculateValidationBuffer`).
//!
//! A descriptor (as these use it): `+0x18` its name, `+0x1c` its size,
//! `+0x40` its parent, `+0x44` its member list (a ring through each
//! member's `+4`, the member 4 bytes before its link), `+0x64`, `+0x68`
//! and `+0x6c` three buffers of its size (the default instance, a
//! scratch instance and the validation mask: the default buffer is
//! checked with the scratch one, the validation buffer is the scratch
//! one's job afterwards), `+0x8c` the offset of its own part in them,
//! `+0x90` flags (bit 31 done, bit 30 its slot 12 ran, bit 28 nothing to
//! validate, bit 27 holds a pointer-like member, bits 0..3 how it
//! constructs). A member: `+0xc` its owner (`+0x88` the owner's base),
//! `+0x10` its name, `+0x14` flags, `+0x18` its type object (`+0x1c` the
//! element size; slot 1 its class descriptor), `+0x1c` its annotations,
//! `+0x24` its offset; slot 1 its kind info (`+0x30`: 1 fixed array, 2
//! pointer, 4 an object).
//!
//! `memset` / `memcpy` stay calls to the CRT, as the originals make them.

use ffx_graphic::host::{Conv, Host};

use crate::descriptor::MEMCPY;
use crate::p1_misc::{MEMSET, PRINTF};

/// The two, called at their addresses (they recurse into each other on
/// the parent and the members' classes).
pub const DEFAULT_BUFFER: u32 = 0x0043_c420;
pub const VALIDATION_BUFFER: u32 = 0x0043_c630;
/// `fillDefaultBuffer` (`0x0043cfd0`, thiscall).
const FILL_DEFAULT: u32 = 0x0043_cfd0;
/// `PClassDescriptor::getDefaultBuffer` (thiscall).
const GET_DEFAULT: u32 = 0x0043_cb80;
/// `PClassDataMemberArrayFixedSize::getFixedSize` (thiscall on the
/// member).
const FIXED_SIZE: u32 = 0x0057_6050;
/// The annotation list's lookup (thiscall on `member + 0x1c`, `(type)`).
const FIND_ANNOTATION: u32 = 0x0057_5d40;
/// The default-value annotation's type (`PAnnotationWithValue<T>`'s).
const DEFAULT_ANNOTATION: u32 = 0x00c9_07f4;
/// The messages.
const POSITION_DEPENDANT: u32 = 0x00b0_ed08;
const OVERLAPS: u32 = 0x00b0_ed94;

fn memset(h: &mut impl Host, at: u32, byte: u32, n: u32) {
    h.call_sym(MEMSET, Conv::Cdecl, &[at, byte, n]);
}

fn memcpy(h: &mut impl Host, to: u32, from: u32, n: u32) {
    h.call_sym(MEMCPY, Conv::Cdecl, &[to, from, n]);
}

/// The members of the ring at `head`: the first member (`link - 4`), or
/// none at the head, a null link or a null member.
fn member_of(head: u32, link: u32) -> Option<u32> {
    (link != head && link != 0 && link != 4).then(|| link - 4)
}

/// A member's kind (its slot 1's `+0x30`, 0 for none).
fn kind(h: &mut impl Host, member: u32) -> u32 {
    let info = h.call_virtual(member, 1, &[]);
    if info == 0 { 0 } else { h.read(info + 0x30) }
}

/// `desc`'s buffers worked out when they are not yet (flags bit 31
/// clear): its slot 12 first unless bit 30, then both calculations.
fn ensure(h: &mut impl Host, desc: u32) {
    let f: u32 = h.read(desc + 0x90);
    if (f as i32) < 0 {
        return;
    }
    if f & 0x4000_0000 == 0 {
        h.call_virtual(desc, 12, &[]);
    }
    h.call_sym(DEFAULT_BUFFER, Conv::Thiscall(desc), &[]);
    h.call_sym(VALIDATION_BUFFER, Conv::Thiscall(desc), &[]);
}

/// The ports over game memory.
pub mod bridge {
    use super::{
        Conv, DEFAULT_ANNOTATION, FILL_DEFAULT, FIND_ANNOTATION, FIXED_SIZE, GET_DEFAULT, Host,
        OVERLAPS, POSITION_DEPENDANT, PRINTF, ensure, kind, member_of, memcpy, memset,
    };

    /// `PClassDescriptor::calculateDefaultBuffer()` (`0x0043c420`,
    /// thiscall): nothing without the buffers `+0x64`/`+0x68` or once
    /// done (flags bit 31). Construct flag bit 0: `fillDefaultBuffer`.
    /// Else both buffers filled with 0xa5, and unless bit 1: an instance
    /// constructed in place in `+0x68` (slot 10; when it refuses,
    /// `fillDefaultBuffer`), and unless bit 3 a second one in `+0x64`
    /// compared with it (a message when they differ: the constructor
    /// depends on where it runs) and destroyed (slot 11); then `+0x68`
    /// copied to `+0x64` and destroyed; then each member whose class has
    /// a default buffer and that is an object (kind 4) or flags bit 6,
    /// not bit 1, gets that buffer (`getDefaultBuffer`, the class's size)
    /// at its offset. Last, flags bit 31. `eax` is not read.
    pub fn calculate_default_buffer(h: &mut impl Host, desc: u32) {
        let def: u32 = h.read(desc + 0x64);
        let scratch: u32 = h.read(desc + 0x68);
        if def == 0 || scratch == 0 {
            return;
        }
        let flags: u32 = h.read(desc + 0x90);
        if (flags as i32) < 0 {
            return;
        }
        let off: u32 = h.read(desc + 0x8c);
        let construct = flags & 0x7ff_ffff;
        let base = def.wrapping_add(off);
        let size: u32 = h.read(desc + 0x1c);
        if construct & 1 != 0 {
            h.call_sym(FILL_DEFAULT, Conv::Thiscall(desc), &[]);
            let f: u32 = h.read(desc + 0x90);
            h.write(desc + 0x90, f | 0x8000_0000);
            return;
        }
        memset(h, scratch, 0xa5, size);
        let def: u32 = h.read(desc + 0x64);
        memset(h, def, 0xa5, size);
        if construct & 2 == 0 {
            let scratch: u32 = h.read(desc + 0x68);
            let name: u32 = h.read(desc + 0x18);
            let scratch_at = scratch.wrapping_add(off);
            let def_at = h.read::<u32>(desc + 0x64).wrapping_add(off);
            if h.call_virtual(desc, 10, &[scratch]) as u8 == 0 {
                h.call_sym(FILL_DEFAULT, Conv::Thiscall(desc), &[]);
            } else {
                if (construct >> 3) & 1 == 0 {
                    let def: u32 = h.read(desc + 0x64);
                    h.call_virtual(desc, 10, &[def]);
                    let a: u32 = h.read(desc + 0x68);
                    let b: u32 = h.read(desc + 0x64);
                    let mut differ = false;
                    for i in 0..size {
                        if h.read::<u8>(a.wrapping_add(i)) != h.read::<u8>(b.wrapping_add(i)) {
                            differ = true;
                            break;
                        }
                    }
                    if differ {
                        let msg = h.rebase(POSITION_DEPENDANT);
                        h.call_sym(PRINTF, Conv::Cdecl, &[1, msg, name]);
                    }
                    h.call_virtual(desc, 11, &[def_at]);
                }
                let to: u32 = h.read(desc + 0x64);
                let from: u32 = h.read(desc + 0x68);
                memcpy(h, to, from, size);
                h.call_virtual(desc, 11, &[scratch_at]);
            }
            let head = desc + 0x44;
            let mut m = member_of(head, h.read(head));
            while let Some(member) = m {
                let mflags: u32 = h.read(member + 0x14);
                let k = kind(h, member);
                let ty_obj: u32 = h.read(member + 0x18);
                let ty = h.call_virtual(ty_obj, 1, &[]);
                if ty != 0 && mflags & 2 == 0 && (mflags & 0x40 != 0 || k == 4) {
                    let src = h.call_sym(GET_DEFAULT, Conv::Thiscall(ty), &[]);
                    let n: u32 = h.read(ty + 0x1c);
                    let moff: u32 = h.read(member + 0x24);
                    let owner: u32 = h.read(member + 0xc);
                    let obase: u32 = h.read(owner + 0x88);
                    let at = moff.wrapping_sub(obase).wrapping_add(base);
                    memcpy(h, at, src, n);
                }
                m = member_of(head, h.read(member + 4));
            }
        }
        let f: u32 = h.read(desc + 0x90);
        h.write(desc + 0x90, f | 0x8000_0000);
    }

    /// `PClassDescriptor::calculateValidationBuffer()` (`0x0043c630`,
    /// thiscall) -> `al` 1, or 0 when a member overlaps another (with a
    /// message; `eax` the rest of the message's result). `al` 1 at once
    /// without the buffers `+0x68`/`+0x6c`. Else flags bits 27-28
    /// cleared, `+0x68` filled with 0xff and `+0x6c` with 0; the
    /// parent's buffers (worked out first) copied in at its offset, its
    /// bit 27 taken; then per member, at its offset: its bytes in `+0x68`
    /// must all still be set; they become 0, and its validation bytes
    /// (`+0x6c`) its class's (worked out first; per element of a fixed
    /// array, 0xff where the class has none), or its default-value
    /// annotation's value, or 0xff; pointers (kind 2 without flags bit
    /// 6) and flags bit 1 leave them 0. Kind 4 sets bit 27. Last, bit 28
    /// when `+0x68` is all zero (nothing left unclaimed).
    #[allow(clippy::too_many_lines)]
    pub fn calculate_validation_buffer(h: &mut impl Host, desc: u32) -> u32 {
        let scratch: u32 = h.read(desc + 0x68);
        if scratch == 0 {
            return 1;
        }
        if h.read::<u32>(desc + 0x6c) == 0 {
            return (scratch & 0xffff_ff00) | 1;
        }
        let f: u32 = h.read(desc + 0x90);
        h.write(desc + 0x90, f & 0xe7ff_ffff);
        let size: u32 = h.read(desc + 0x1c);
        memset(h, scratch, 0xff, size);
        let mask: u32 = h.read(desc + 0x6c);
        memset(h, mask, 0, size);
        let off: u32 = h.read(desc + 0x8c);
        let claim = h.read::<u32>(desc + 0x68).wrapping_add(off);
        let valid = h.read::<u32>(desc + 0x6c).wrapping_add(off);
        let parent: u32 = h.read(desc + 0x40);
        if parent != 0 {
            let psize: u32 = h.read(parent + 0x1c);
            let poff: u32 = h.read(parent + 0x8c);
            ensure(h, parent);
            let src1: u32 = h.read(parent + 0x68);
            ensure(h, parent);
            let src2: u32 = h.read(parent + 0x6c);
            if src1 != 0 {
                memcpy(h, claim.wrapping_sub(poff), src1, psize);
            }
            if src2 != 0 {
                memcpy(h, valid.wrapping_sub(poff), src2, psize);
            }
            let pf = h.read::<u32>(parent + 0x90) & 0x0800_0000;
            let f: u32 = h.read(desc + 0x90);
            h.write(desc + 0x90, f | pf);
        }
        let head = desc + 0x44;
        let mut m = member_of(head, h.read(head));
        while let Some(member) = m {
            let ty_obj: u32 = h.read(member + 0x18);
            let mut esize: u32 = h.read(ty_obj + 0x1c);
            let info = h.call_virtual(member, 1, &[]);
            let mflags: u32 = h.read(member + 0x14);
            let pointer = info != 0 && h.read::<u32>(info + 0x30) == 2;
            if (mflags >> 6) & 1 == 0 && pointer || (mflags >> 1) & 1 != 0 {
                esize = 4;
            }
            let moff: u32 = h.read(member + 0x24);
            let owner: u32 = h.read(member + 0xc);
            let rel = moff.wrapping_sub(h.read(owner + 0x88));
            let mut c = claim.wrapping_add(rel);
            let mut v = valid.wrapping_add(rel);
            let k = kind(h, member);
            let ty_obj: u32 = h.read(member + 0x18);
            let ty = h.call_virtual(ty_obj, 1, &[]);
            let mut count = 1u32;
            if k == 1 {
                count = h.call_sym(FIXED_SIZE, Conv::Thiscall(member), &[]);
            }
            let total = count.wrapping_mul(esize);
            for i in 0..total {
                if h.read::<u8>(c.wrapping_add(i)) == 0 {
                    let mname: u32 = h.read(member + 0x10);
                    let dname: u32 = h.read(desc + 0x18);
                    let msg = h.rebase(OVERLAPS);
                    let r = h.call_sym(PRINTF, Conv::Cdecl, &[1, msg, mname, dname]);
                    return r & 0xffff_ff00;
                }
            }
            let cleared = (mflags >> 1) & 1 != 0 || (k == 2 && (mflags >> 6) & 1 == 0);
            if cleared {
                memset(h, c, 0, total);
            } else {
                if k == 4 {
                    let f: u32 = h.read(desc + 0x90);
                    h.write(desc + 0x90, f | 0x0800_0000);
                }
                if ty == 0 {
                    memset(h, c, 0, total);
                    let ann = h.call_sym(
                        FIND_ANNOTATION,
                        Conv::Thiscall(member + 0x1c),
                        &[h.rebase(DEFAULT_ANNOTATION)],
                    );
                    if ann == 0 {
                        memset(h, v, 0xff, total);
                    } else {
                        let value = h.call_virtual(ann, 1, &[]);
                        memcpy(h, v, value, esize);
                    }
                } else {
                    ensure(h, ty);
                    let src1: u32 = h.read(ty + 0x68);
                    ensure(h, ty);
                    let src2: u32 = h.read(ty + 0x6c);
                    for _ in 0..count {
                        if src1 == 0 {
                            memset(h, c, 0, esize);
                        } else {
                            memcpy(h, c, src1, esize);
                        }
                        if src2 == 0 {
                            memset(h, v, 0xff, esize);
                        } else {
                            memcpy(h, v, src2, esize);
                        }
                        c = c.wrapping_add(esize);
                        v = v.wrapping_add(esize);
                    }
                }
            }
            m = member_of(head, h.read(member + 4));
        }
        let size: u32 = h.read(desc + 0x1c);
        let scratch: u32 = h.read(desc + 0x68);
        let mut i = 0u32;
        while i < size && h.read::<u8>(scratch.wrapping_add(i)) == 0 {
            i += 1;
        }
        if i == size {
            let f: u32 = h.read(desc + 0x90);
            h.write(desc + 0x90, f | 0x1000_0000);
        }
        (i & 0xffff_ff00) | 1
    }
}
