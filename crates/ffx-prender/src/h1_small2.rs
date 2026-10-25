//! More small `PhyreEngine` render methods (M4 port round 2, h1-render-a,
//! batch 3): descriptor callbacks of the material and mesh segment
//! types, a `PSharray` buffer setter, the named-semantic type singleton,
//! and a texture's binding release.

use ffx_graphic::host::{Conv, Host};

use crate::basic::FREE;
use crate::registry::ATEXIT;

/// `PClassDescriptor::find`-like getter (thiscall on a descriptor, no
/// arguments) `0x0043cbb0`: the material constructor's two class lookups.
pub const DESCRIPTOR_GET: u32 = 0x0043_cbb0;
/// The two descriptors `PMaterial`'s constructor takes.
pub const MATERIAL_ARGS: [u32; 2] = [0x00ca_5ff0, 0x00ca_5040];
/// `PMaterial::PMaterial(this, a, b)` (thiscall, two words).
pub const MATERIAL_CTOR: u32 = 0x004d_7b10;
/// The mesh segment's member destructors (thiscall, no arguments) on
/// `p+0x30` and `p+0x28`.
pub const SEGMENT_DTORS: [(u32, u32); 2] = [(0x0055_e060, 0x30), (0x0043_3900, 0x28)];
/// `0x0047fae0`'s singleton: `[guard, object, constructor, name, f1, f2,
/// vtable, destructor]` (`PNamedSemanticDescriptor`, 0xc bytes).
pub const SEMANTIC_TYPE: [u32; 8] = [
    0x00c9_872c,
    0x00c9_86f8,
    0x005a_0200,
    0x00b1_3be4,
    0x0047_fd20,
    0x0047_fdf0,
    0x00b1_3d88,
    0x00af_c490,
];
/// The texture unbinders by binding kind (`+4` byte 0, 1, 2): thiscall on
/// the texture, one word (0).
pub const UNBIND: [u32; 3] = [0x004c_28f0, 0x004c_2920, 0x004c_2950];

pub mod bridge {
    //! The originals over game memory.

    use super::{
        ATEXIT, Conv, DESCRIPTOR_GET, FREE, Host, MATERIAL_ARGS, MATERIAL_CTOR, SEGMENT_DTORS,
        UNBIND,
    };

    /// `constructInstance(descriptor, p)` of `PMaterial` (`0x004d8630`,
    /// `ret 4`): both class lookups must give non-zero, else `al` = 0;
    /// then `PMaterial::PMaterial(p, a, b)` when `p` is non-zero; `al` = 1.
    pub fn construct_material(h: &mut impl Host, p: u32) -> u8 {
        let [da, db] = MATERIAL_ARGS.map(|d| h.rebase(d));
        let a = h.call_sym(DESCRIPTOR_GET, Conv::Thiscall(da), &[]);
        let b = h.call_sym(DESCRIPTOR_GET, Conv::Thiscall(db), &[]);
        if a == 0 || b == 0 {
            return 0;
        }
        if p != 0 {
            h.call_sym(MATERIAL_CTOR, Conv::Thiscall(p), &[a, b]);
        }
        1
    }

    /// `destroyInstance(descriptor, p)` of `PMeshSegment`
    /// (`0x00488ff0`, `ret 4`): the members' destructors, the owned block
    /// `+0xc` (bit 31 of `+8` clear) freed when non-zero, then `+0xc` and
    /// `+8` zeroed; `al` = 1.
    pub fn destroy_mesh_segment(h: &mut impl Host, p: u32) -> u8 {
        for (dtor, at) in SEGMENT_DTORS {
            h.call_sym(dtor, Conv::Thiscall(p + at), &[]);
        }
        let word: u32 = h.read(p + 8);
        let block: u32 = h.read(p + 0xc);
        if word >> 31 == 0 && block != 0 {
            h.call_sym(FREE, Conv::Cdecl, &[block]);
        }
        h.write(p + 0xc, 0u32);
        h.write(p + 8, 0u32);
        1
    }

    /// `0x0048c650(this, data, count)` (thiscall, `ret 8`): a `PSharray`
    /// (`+0` count, bit 31 "not owned"; one element inline at `+4`, more
    /// through the pointer at `+4`) takes an external buffer: 5 for a
    /// negative count; one element is copied inline (count word 1),
    /// otherwise `+4` = `data` with the count marked not owned. An owned
    /// old buffer, not inline and not `data`, is freed. `eax` = 0.
    pub fn sharray_set(h: &mut impl Host, this: u32, data: u32, count: u32) -> u32 {
        if (count as i32) < 0 {
            return 5;
        }
        let word: u32 = h.read(this);
        let n = word & 0x7fff_ffff;
        let inline = this + 4;
        let old = if n > 1 {
            h.read(inline)
        } else if n != 0 {
            inline
        } else {
            0
        };
        let (target, new_word) = if count == 1 {
            let first: u32 = h.read(data);
            h.write(inline, first);
            (inline, 1)
        } else {
            (data, count | 0x8000_0000)
        };
        if old != target && old != inline && word >> 31 == 0 {
            h.call_sym(FREE, Conv::Cdecl, &[old]);
        }
        if target != inline {
            h.write(inline, target);
        }
        h.write(this, new_word);
        0
    }

    /// `0x0047fae0()`: the named-semantic type singleton, made on the
    /// first call (bit 0 of its guard): its constructor (thiscall `(name,
    /// 0xc, 4, f1, f2)`), its vtable, its destructor with `atexit`.
    pub fn semantic_type(
        h: &mut impl Host,
        [guard, object, ctor, name, f1, f2, vtable, dtor]: [u32; 8],
    ) -> u32 {
        let obj = h.rebase(object);
        let at = h.rebase(guard);
        let flags: u32 = h.read(at);
        if flags & 1 == 0 {
            h.write(at, flags | 1);
            let args = [h.rebase(name), 0xc, 4, h.rebase(f1), h.rebase(f2)];
            h.call_sym(ctor, Conv::Thiscall(obj), &args);
            let vtable = h.rebase(vtable);
            h.write(obj, vtable);
            let dtor = h.rebase(dtor);
            h.call_sym(ATEXIT, Conv::Cdecl, &[dtor]);
        }
        obj
    }

    /// `0x004c29b0(this)`: a texture binding (`+0` the texture, `+4` byte
    /// its kind 0..2) released when the texture still points back at it
    /// (`+0x18`): its flag bit 8 (`+0x14`) cleared, the kind's unbinder,
    /// `+0` = 0. `eax`: the unbinder's, else 0 for kinds 0..2 and the kind
    /// - 2 for others (the original's `sub`/`dec` dispatch).
    pub fn release_binding(h: &mut impl Host, this: u32) -> u32 {
        let kind: u8 = h.read(this + 4);
        let Some(&unbind) = UNBIND.get(usize::from(kind)) else {
            return u32::from(kind).wrapping_sub(2);
        };
        let texture: u32 = h.read(this);
        if texture == 0 || h.read::<u32>(texture + 0x18) != this {
            return 0;
        }
        let flags: u32 = h.read(texture + 0x14);
        h.write(texture + 0x14, flags & 0xffff_feff);
        let r = h.call_sym(unbind, Conv::Thiscall(texture), &[0]);
        h.write(this, 0u32);
        r
    }
}
