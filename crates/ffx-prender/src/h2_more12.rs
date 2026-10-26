//! h2-render-b, last round, batch 1: the static descriptor of
//! `PSharray<PMeshInstanceSegmentStreamBinding const*>`, the dynamic
//! buffers' layout of a stream set (`0x0058c0d0`), and the mesh
//! instance's skinning matrices (`0x004f98b0`).

use ffx_bridge::{Mem, x87};
use ffx_graphic::host::{Conv, Host};

use crate::declare::{
    ADD_ANNOTATION, ADD_TO_NAMESPACE, ANNOTATION_CTOR, ANNOTATION_VALUE, ANNOTATION_VTABLE, ATEXIT,
    DATA_MEMBER_CTOR, M_COUNT, MEMBER_ARRAY_CTOR, MEMBER_HANDLER, REGISTER_TYPE, TYPE_UINT,
};
use crate::descriptor::PHYRE_FREE;

/// The `PSharray<PMeshInstanceSegmentStreamBinding const*>` descriptor
/// (`0x00ca79a8`) and what its declaration stores.
pub mod sharray {
    /// The static descriptor.
    pub const DESC: u32 = 0x00ca_79a8;
    /// Its constructor (`PClassDescriptorAbstract<...>`, thiscall).
    pub const CTOR: u32 = 0x004f_2160;
    /// `PClassDescriptorConcrete<PSharray<...>>`'s vtable.
    pub const VTABLE: u32 = 0x00b2_319c;
    /// The class name's global (keeps the first name).
    pub const NAME: u32 = 0x00ca_7e84;
    /// The elements' class descriptor (`PMeshInstanceSegmentStreamBinding`).
    pub const ELEMENT: u32 = 0x00ca_70f8;
    /// The string `"m_els"` this copy names its elements member with.
    pub const M_ELS: u32 = 0x00b0_edd0;
    /// `PClassDataMemberArrayDynamic<PSharray<...>>`'s vtable.
    pub const MEMBER_VTABLE: u32 = 0x00b2_31e0;
    /// The member's storage setter (`PArray_assignOrStore_004f9100`) and
    /// the handler stored on every declaration (`+0x38`).
    pub const SET_STORAGE: u32 = 0x004f_9100;
    pub const MEMBER_HANDLER2: u32 = 0x004f_8140;
    /// The `atexit` destructors: descriptor, elements member, count
    /// member, annotation.
    pub const AT_EXIT: [u32; 4] = [0x00af_f370, 0x00af_f700, 0x00af_f560, 0x00af_f340];
}

/// `0x004f98b0`'s callees.
const RESIZE_U32_ARRAY: u32 = 0x004f_8880;
const NEW_CONSTANT_BUFFER: u32 = 0x004f_49d0;
const RESIZE_BLOCK: u32 = 0x0049_4c20;
const COMPUTE_JOINT_MATRICES: u32 = 0x004f_6ef0;
const MAPPING_CTOR: u32 = 0x0058_44e0;
const MAPPING_DTOR: u32 = 0x0058_59f0;
const RING_NEXT: u32 = 0x004f_8460;
const MATRIX_MUL: u32 = 0x004c_9da0;
const TRANSPOSE: u32 = 0x004f_93f0;
const MARK_SLOT: u32 = 0x0059_d320;

/// The most matrices (with one spare) `0x004f98b0` keeps in its own frame
/// (`_alloca`); more go to the heap.
pub const SKIN_STACK_MATRICES: u32 = 0x80;

/// The bytes a stream set's buffers take: each size rounded up to 16.
fn align16(n: u32) -> u32 {
    n.wrapping_add(0xf) & !0xf
}

/// A `PSharray`'s elements at `array`: the count word at `array`, the
/// storage word after it (inline for one element, none for zero).
fn sharray_data(m: &impl Mem, array: u32) -> u32 {
    let n = m.read::<u32>(array) & 0x7fff_ffff;
    match n {
        0 => 0,
        1 => array + 4,
        _ => m.read(array + 4),
    }
}

pub mod bridge {
    //! The originals over game memory.

    use super::{
        ADD_ANNOTATION, ADD_TO_NAMESPACE, ANNOTATION_CTOR, ANNOTATION_VALUE, ANNOTATION_VTABLE,
        ATEXIT, COMPUTE_JOINT_MATRICES, Conv, DATA_MEMBER_CTOR, Host, M_COUNT, MAPPING_CTOR,
        MAPPING_DTOR, MARK_SLOT, MATRIX_MUL, MEMBER_ARRAY_CTOR, MEMBER_HANDLER, Mem,
        NEW_CONSTANT_BUFFER, PHYRE_FREE, REGISTER_TYPE, RESIZE_BLOCK, RESIZE_U32_ARRAY, RING_NEXT,
        SKIN_STACK_MATRICES, TRANSPOSE, TYPE_UINT, align16, sharray, sharray_data, x87,
    };

    /// `atexit(f)` of the image function `f`.
    fn at_exit(h: &mut impl Host, f: u32) {
        let f = h.rebase(f);
        h.call_sym(ATEXIT, Conv::Cdecl, &[f]);
    }

    /// `PSharray<PMeshInstanceSegmentStreamBinding const*>`'s
    /// `DeclareClassDescriptor(name)` (`0x004f4d60`, cdecl): on first use
    /// the static descriptor (guard bit 0 of `+0x94`: the constructor,
    /// flag 2 of `+0x90` cleared, the vtable); then, for a name when the
    /// descriptor has none (`+0x18`), the name (the class's global keeps
    /// the first one), the elements member at `+0x98` (bit 1), the
    /// member's flag `0x40` and handler (`+0x38`) and element size
    /// (`+0x2c`) on every such call, the count member at `+0xd8` (bit
    /// 2), its annotation at `+0x104` (bit 3, added to `+0xf4` while that
    /// list is empty), the registration and the namespace. Returns the
    /// descriptor. (The other `PSharray` copies build the count member
    /// first, [`crate::declare`].)
    pub fn declare(h: &mut impl Host, name: u32) -> u32 {
        let d = h.rebase(sharray::DESC);
        let guard = d + 0x94;
        let mut g: u32 = h.read(guard);
        if g & 1 == 0 {
            h.write(guard, g | 1);
            h.call_sym(sharray::CTOR, Conv::Thiscall(d), &[]);
            let flags: u32 = h.read(d + 0x90);
            h.write(d + 0x90, flags & !2);
            let vt = h.rebase(sharray::VTABLE);
            h.write(d, vt);
            at_exit(h, sharray::AT_EXIT[0]);
            g = h.read(guard);
        }
        if name == 0 || h.read::<u32>(d + 0x18) != 0 {
            return d;
        }
        let ng = h.rebase(sharray::NAME);
        let mut n: u32 = h.read(ng);
        if n == 0 {
            n = name;
            h.write(ng, n);
        }
        h.write(d + 0x18, n);
        let member = d + 0x98;
        if g & 2 == 0 {
            h.write(guard, g | 2);
            let elem = h.rebase(sharray::ELEMENT);
            let m = h.rebase(sharray::M_ELS);
            h.call_sym(
                MEMBER_ARRAY_CTOR,
                Conv::Thiscall(member),
                &[d, elem, m, 4, 2, 2, 4],
            );
            let vt = h.rebase(sharray::MEMBER_VTABLE);
            h.write(member, vt);
            let f = h.rebase(MEMBER_HANDLER);
            h.write(member + 0x34, f);
            h.write(member + 0x38, 0u32);
            let f = h.rebase(sharray::SET_STORAGE);
            h.write(member + 0x3c, f);
            at_exit(h, sharray::AT_EXIT[1]);
            g = h.read(guard);
        }
        let flags: u32 = h.read(member + 0x14);
        h.write(member + 0x14, flags | 0x40);
        let f = h.rebase(sharray::MEMBER_HANDLER2);
        h.write(member + 0x38, f);
        h.write(member + 0x2c, 4u32);
        if g & 4 == 0 {
            h.write(guard, g | 4);
            let t = h.call_sym(TYPE_UINT, Conv::Cdecl, &[]);
            let m = h.rebase(M_COUNT);
            h.call_sym(
                DATA_MEMBER_CTOR,
                Conv::Thiscall(d + 0xd8),
                &[d, t, m, 0, 0, 0],
            );
            at_exit(h, sharray::AT_EXIT[2]);
            g = h.read(guard);
        }
        let annotation = d + 0x104;
        if g & 8 == 0 {
            h.write(guard, g | 8);
            let t = h.call_sym(TYPE_UINT, Conv::Cdecl, &[]);
            let v = h.rebase(ANNOTATION_VALUE);
            h.call_sym(ANNOTATION_CTOR, Conv::Thiscall(annotation), &[v, t, 0]);
            let vt = h.rebase(ANNOTATION_VTABLE);
            h.write(annotation, vt);
            h.write(annotation + 0x18, 0x7fff_ffffu32);
            at_exit(h, sharray::AT_EXIT[3]);
        }
        if h.read::<u32>(annotation + 4) == annotation + 4 {
            h.call_sym(ADD_ANNOTATION, Conv::Thiscall(d + 0xf4), &[annotation]);
        }
        h.call_sym(REGISTER_TYPE, Conv::Cdecl, &[d]);
        h.call_sym(ADD_TO_NAMESPACE, Conv::Thiscall(d), &[]);
        d
    }

    /// `0x0058c0d0(renderer; kind, streams, n, extra)` (thiscall, `ret
    /// 0x10`): places a stream set in the renderer's dynamic buffers.
    /// `kind` 0 uses the buffer pair at `+0x320`, any other the one at
    /// `+0x348` (each: buffers `+0`/`+8` per frame `[+0x31c]`, used bytes
    /// `+0x10`/`+0x14`, capacities `+0x18`/`+0x1c`). `streams` are `n`
    /// 0x40-byte stream entries (size `+0x38`), `extra` (or 0) one more
    /// (size `+0x34`), each rounded up to 16. Returns 0xd when either
    /// buffer has no room (nothing written), else 0, the used bytes
    /// advanced: for each entry with a size, its slot record (20 bytes,
    /// index `+0x10` into the `PSharray` at `+0x14`) gets the buffer, its
    /// `+0x30` flag bit 0 when the buffer is set, its offset `+0x2c`.
    /// `kind` 1 stores the frame's view (`+0x368`/`+0x370`, or 0 for an
    /// empty entry) at the record's `+8` and sets bit 0 for every entry;
    /// the others store the mapped address (`+0x340`/`+0x344` plus the
    /// offset) at `+0x38` and its null flag (4) in byte `+0xe`.
    #[allow(clippy::too_many_lines)]
    pub fn place_streams(
        m: &mut impl Mem,
        this: u32,
        kind: u32,
        streams: u32,
        n: u32,
        extra: u32,
    ) -> u32 {
        let pair = if kind == 0 {
            this + 0x320
        } else {
            this + 0x348
        };
        // the entries' sizes, two at a time, then the odd one
        let mut even = 0u32;
        let mut odd = 0u32;
        let mut done = 0u32;
        if n.cast_signed() >= 2 {
            let pairs = ((n - 2) >> 1) + 1;
            for k in 0..pairs {
                let at = streams.wrapping_add(k.wrapping_mul(0x80));
                even = even.wrapping_add(align16(m.read(at + 0x38)));
                odd = odd.wrapping_add(align16(m.read(at + 0x78)));
            }
            done = pairs.wrapping_mul(2);
        }
        let mut last = 0u32;
        if done < n {
            last = align16(m.read(streams.wrapping_add(done << 6).wrapping_add(0x38)));
        }
        let total = last.wrapping_add(even.wrapping_add(odd));
        let extra_size = if extra == 0 {
            0
        } else {
            align16(m.read(extra + 0x34))
        };
        let mut used_a: u32 = m.read(pair + 0x10);
        let mut used_b: u32 = m.read(pair + 0x14);
        if used_a.wrapping_add(total) > m.read::<u32>(pair + 0x18) {
            return 0xd;
        }
        let used_b_after = used_b.wrapping_add(extra_size);
        if used_b_after > m.read::<u32>(pair + 0x1c) {
            return 0xd;
        }
        let frame: u32 = m.read(this + 0x31c);
        let buffer_a: u32 = m.read(pair.wrapping_add(frame << 2));
        let buffer_b: u32 = m.read(pair.wrapping_add(frame << 2).wrapping_add(8));
        if kind == 1 {
            for k in 0..n {
                let e = streams.wrapping_add(k << 6) + 0x1c;
                let size: u32 = m.read(e + 0x1c);
                let slot: u32 = m.read(e.wrapping_sub(0xc));
                let mut view = 0u32;
                if size != 0 {
                    let records = sharray_data(m, e.wrapping_sub(4));
                    m.write(records.wrapping_add(slot.wrapping_mul(20)), buffer_a);
                    if buffer_a != 0 {
                        let f: u32 = m.read(e + 0x18);
                        m.write(e + 0x18, f | 1);
                    }
                    m.write(e + 0x14, used_a);
                    let frame: u32 = m.read(this + 0x31c);
                    view = m.read(this.wrapping_add(frame << 2).wrapping_add(0x368));
                    used_a = used_a.wrapping_add(align16(size));
                }
                let records = sharray_data(m, e.wrapping_sub(4));
                m.write(
                    records.wrapping_add(slot.wrapping_mul(20)).wrapping_add(8),
                    view,
                );
                let f: u32 = m.read(e + 0x18);
                m.write(e + 0x18, f | 1);
            }
            if extra != 0 {
                let slot: u32 = m.read(extra + 0x10);
                let mut view = 0u32;
                if extra_size != 0 {
                    let records = sharray_data(m, extra + 0x14);
                    m.write(records.wrapping_add(slot.wrapping_mul(20)), buffer_b);
                    if buffer_b != 0 {
                        let f: u32 = m.read(extra + 0x30);
                        m.write(extra + 0x30, f | 1);
                    }
                    m.write(extra + 0x2c, used_b);
                    let frame: u32 = m.read(this + 0x31c);
                    view = m.read(this.wrapping_add(frame << 2).wrapping_add(0x370));
                    used_b = used_b_after;
                }
                let records = sharray_data(m, extra + 0x14);
                m.write(
                    records.wrapping_add(slot.wrapping_mul(20)).wrapping_add(8),
                    view,
                );
                let f: u32 = m.read(extra + 0x30);
                m.write(extra + 0x30, f | 1);
            }
        } else {
            let mapped_a: u32 = m.read(this + 0x340);
            let mapped_b: u32 = m.read(this + 0x344);
            for k in 0..n {
                let e = streams.wrapping_add(k << 6) + 0x1c;
                let size: u32 = m.read(e + 0x1c);
                if size == 0 {
                    continue;
                }
                let slot: u32 = m.read(e.wrapping_sub(0xc));
                let records = sharray_data(m, e.wrapping_sub(4));
                m.write(records.wrapping_add(slot.wrapping_mul(20)), buffer_a);
                if buffer_a != 0 {
                    let f: u32 = m.read(e + 0x18);
                    m.write(e + 0x18, f | 1);
                }
                m.write(e + 0x14, used_a);
                let at = mapped_a.wrapping_add(used_a);
                m.write(e + 0x20, at);
                m.write(e.wrapping_sub(8), if at == 0 { 4u8 } else { 0 });
                used_a = used_a.wrapping_add(align16(size));
                m.write(e + 0x1c, size);
            }
            if extra_size != 0 {
                let records = sharray_data(m, extra + 0x14);
                let slot: u32 = m.read(extra + 0x10);
                m.write(records.wrapping_add(slot.wrapping_mul(20)), buffer_b);
                if buffer_b != 0 {
                    let f: u32 = m.read(extra + 0x30);
                    m.write(extra + 0x30, f | 1);
                }
                m.write(extra + 0x2c, used_b);
                let at = used_b.wrapping_add(mapped_b);
                m.write(extra + 0x38, at);
                m.write(extra + 0xe, if at == 0 { 4u8 } else { 0 });
                m.write(extra + 0x34, extra_size);
                used_b = used_b_after;
            }
        }
        m.write(pair + 0x10, used_a);
        m.write(pair + 0x14, used_b);
        0
    }

    /// The words of `0x004f98b0`'s matrix block: `matrices` 0 and this
    /// many at most in its frame ([`SKIN_STACK_MATRICES`]).
    #[must_use]
    pub fn skin_block_matrices(m: &impl Mem, this: u32) -> u32 {
        let skeleton: u32 = m.read(this);
        (m.read::<u32>(skeleton + 0x18) & 0x7fff_ffff).wrapping_add(1)
    }

    /// `0x004f98b0(instance; matrices)` (thiscall, `ret 4`): uploads a
    /// mesh instance's skinning matrices. With joints (count at
    /// `[this] + 0x18`; else 0): on the first call (`+0x70` empty) the
    /// two per-segment constant-buffer arrays `+0x70` and `+0x78` sized
    /// to the segments (`0x004f8880`, its error returned) and a new
    /// constant buffer for every skinned segment in each; then the two
    /// arrays swapped. With no `matrices` the joint matrices are computed
    /// (`computeJointMatrices`) into a block of `joints + 1` matrices
    /// (`block`, at most [`SKIN_STACK_MATRICES`], or the heap through
    /// `0x00494c20` on the frame's array `ebp - 0x34`, freed after), from
    /// its second matrix on. For each skinned segment (0x6c-byte entries
    /// `[this] + 4`, joint count `+8`, at most 0xc0, pairs `+0xc`): the
    /// buffer's next ring slot mapped (`0x004f8460` into `ebp - 0x14`),
    /// each pair's product joint `[0]` x bind `[1]` (`[this] + 0xc`)
    /// transposed into it, the slot marked (`0x0059d320`) and the buffer's
    /// cursor `+4` advanced (wrapping at the count `+0xc`, 1 for none).
    /// Returns 0, or the first error of the mapping or the marking.
    /// `ebp` is the original's frame (`ebp - 0xc4` up to it).
    #[allow(clippy::too_many_lines)]
    pub fn upload_skin_matrices(
        h: &mut impl Host,
        this: u32,
        matrices: u32,
        ebp: u32,
        block: u32,
    ) -> u32 {
        let mapping = ebp - 0x14;
        let heap = ebp - 0x34;
        let product = ebp - 0x84;
        let transposed = ebp - 0xc4;
        let skeleton: u32 = h.read(this);
        let joints = h.read::<u32>(skeleton + 0x18) & 0x7fff_ffff;
        if joints == 0 {
            return 0;
        }
        let segments = h.read::<u32>(skeleton) & 0x7fff_ffff;
        // the count without its flag bit (31) is 0: the first call
        if h.read::<u32>(this + 0x70) << 1 == 0 {
            for array in [this + 0x70, this + 0x78] {
                let r = h.call_sym(RESIZE_U32_ARRAY, Conv::Thiscall(array), &[segments]);
                if r != 0 {
                    return r;
                }
            }
            for i in 0..segments {
                let entry: u32 = h.read(skeleton + 4);
                let skinned = h.read::<u32>(entry.wrapping_add(i * 0x6c) + 8) & 0x7fff_ffff;
                if skinned == 0 {
                    continue;
                }
                for array in [this + 0x74, this + 0x7c] {
                    let to: u32 = h.read(array);
                    let b = h.call_sym(NEW_CONSTANT_BUFFER, Conv::Cdecl, &[]);
                    h.write(to.wrapping_add(i * 4), b);
                }
            }
        }
        let (old_data, new_data): (u32, u32) = (h.read(this + 0x74), h.read(this + 0x7c));
        h.write(this + 0x74, new_data);
        h.write(this + 0x7c, old_data);
        let (old_count, new_count): (u32, u32) = (h.read(this + 0x70), h.read(this + 0x78));
        h.write(this + 0x70, new_count);
        h.write(this + 0x78, old_count);
        let mut owned = 0u32;
        h.write(heap, 0u32);
        h.write(heap + 4, 0u32);
        let mut base = matrices;
        if matrices == 0 {
            let k = joints + 1;
            let at = if k <= SKIN_STACK_MATRICES {
                block
            } else {
                h.call_sym(RESIZE_BLOCK, Conv::Thiscall(heap), &[k]);
                owned = h.read(heap);
                h.read(heap + 4)
            };
            base = at.wrapping_add(0x40);
            h.call_sym(COMPUTE_JOINT_MATRICES, Conv::Thiscall(this), &[base]);
        }
        let mut result = 0;
        for i in 0..segments {
            let entry = h.read::<u32>(skeleton + 4).wrapping_add(i * 0x6c);
            let mut count = h.read::<u32>(entry + 8) & 0x7fff_ffff;
            if count == 0 {
                continue;
            }
            h.call_sym(MAPPING_CTOR, Conv::Thiscall(mapping), &[]);
            let buffer: u32 = h.read(h.read::<u32>(this + 0x74).wrapping_add(i * 4));
            let r = h.call_sym(RING_NEXT, Conv::Thiscall(buffer), &[mapping]);
            if r != 0 {
                h.call_sym(MAPPING_DTOR, Conv::Thiscall(mapping), &[]);
                result = r;
                break;
            }
            let binds: u32 = h.read(skeleton + 0xc);
            let pairs: u32 = h.read(entry + 0xc);
            count = count.min(0xc0);
            if count != 0 {
                let to: u32 = h.read(mapping);
                for k in 0..count {
                    let pair = pairs.wrapping_add(k * 4);
                    let bind = u32::from(h.read::<u16>(pair + 2));
                    let joint = u32::from(h.read::<u16>(pair));
                    let b = (bind << 6).wrapping_add(binds);
                    let a = (joint << 6).wrapping_add(base);
                    let p = h.call_sym(MATRIX_MUL, Conv::Thiscall(a), &[product, b]);
                    let t = h.call_sym(TRANSPOSE, Conv::Cdecl, &[transposed, p]);
                    for w in 0..16 {
                        let v: u32 = h.read(t + 4 * w);
                        h.write(to.wrapping_add(k * 0x40 + 4 * w), x87::store(x87::load(v)));
                    }
                }
            }
            let buffer: u32 = h.read(h.read::<u32>(this + 0x74).wrapping_add(i * 4));
            let next = h.read::<u32>(buffer + 4).wrapping_add(1);
            let mut slots = h.read::<u32>(buffer + 0xc) & 0x7fff_ffff;
            if slots == 0 {
                slots = 1;
            }
            let next = if next == slots { 0 } else { next };
            let r = h.call_sym(MARK_SLOT, Conv::Thiscall(buffer), &[next, mapping]);
            if r != 0 {
                h.call_sym(MAPPING_DTOR, Conv::Thiscall(mapping), &[]);
                result = r;
                break;
            }
            h.write(buffer + 4, next);
            h.write(buffer + 8, 0u8);
            h.call_sym(MAPPING_DTOR, Conv::Thiscall(mapping), &[]);
        }
        let data: u32 = h.read(heap + 4);
        if owned >> 31 == 0 && data != 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
        }
        result
    }
}
