//! The cloth instancing meshes (`ClothInstancing*`) and the dynamic
//! meshes' buffer copies.

use ffx_graphic::host::{Conv, Host};

/// `InterlockedCompareExchange`'s import slot (stdcall `(p, new, cmp)`).
pub const INTERLOCKED_COMPARE_EXCHANGE: u32 = 0x00b0_c12c;
/// `memcpy` (cdecl `(dst, src, n)`).
pub const MEMCPY: u32 = 0x0094_9256;
/// `operator_delete` (cdecl `(p)`).
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
/// `Phyre_PRendering_PDynamicMeshInstance_dtor` (thiscall `(inst)`).
pub const MESH_INSTANCE_DTOR: u32 = 0x004f_a470;
/// `ClothInstancingDynamicMeshInstance`'s vtable.
pub const CLOTH_INSTANCE_VTABLE: u32 = 0x00b4_ab28;
/// The cloth instancing entries: count (`& 0x7fffffff`) and pointer.
pub const G_CLOTH_ENTRIES: u32 = 0x00cd_efa4;

const COUNT: u32 = 0x7fff_ffff;

pub mod bridge {
    //! The originals over game memory.

    use super::{
        CLOTH_INSTANCE_VTABLE, COUNT, Conv, G_CLOTH_ENTRIES, Host, INTERLOCKED_COMPARE_EXCHANGE,
        MEMCPY, MESH_INSTANCE_DTOR, OPERATOR_DELETE,
    };

    /// `ClothInstancingDynamicMesh_submit(mesh, inst, job)` (`0x006e28f0`,
    /// `ret 8`): marks the instance's job word (`inst + 8 + [inst + 8]`)
    /// busy; when the job's slot (`+0x20`) flag `+0x810 + 4 slot` was set
    /// (cleared with `InterlockedCompareExchange`), copies the slot's
    /// staged buffer (`+0xc10`, `+0x1810` bytes) into `+0x210`; stores the
    /// mesh after the busy word and runs the inner mesh's (`+0xc`)
    /// `vtable[2](inst, job)`, whose `eax` it returns.
    pub fn submit(h: &mut impl Host, mesh: u32, inst: u32, job: u32) -> u32 {
        let off: u32 = h.read(inst + 8);
        let busy = off.wrapping_add(8).wrapping_add(inst);
        h.write(busy, 1u32);
        let slot: u32 = h.read(job + 0x20);
        let at = mesh.wrapping_add(slot.wrapping_mul(4));
        let cas: u32 = h.read(h.rebase(INTERLOCKED_COMPARE_EXCHANGE));
        if h.call(cas, Conv::Stdcall, &[at.wrapping_add(0x810), 0, 1]) == 1 {
            let n: u32 = h.read(at.wrapping_add(0x1810));
            let src: u32 = h.read(at.wrapping_add(0xc10));
            let dst: u32 = h.read(at.wrapping_add(0x210));
            let f = h.rebase(MEMCPY);
            h.call(f, Conv::Cdecl, &[dst, src, n]);
        }
        h.write(busy + 4, mesh);
        let inner: u32 = h.read(mesh + 0xc);
        h.call_virtual(inner, 2, &[inst, job])
    }

    /// `ClothInstancing_releaseExpired()` (`0x006e2f70`): each entry
    /// (16 bytes: object, second object, shared flag, countdown) with a
    /// non-negative countdown: at 0 its objects are deleted (their
    /// `vtable[0](1)`; unless shared (`+8`), first the object's own
    /// child, `+0x1c` resp. `+0xc`) and cleared; then the countdown drops
    /// by one. The table pointer is reread after every call.
    pub fn release_expired(h: &mut impl Host) {
        let g = h.rebase(G_CLOTH_ENTRIES);
        let mut count: u32 = h.read(g);
        let mut i = 0u32;
        while i < count & COUNT {
            let e = 16 * i;
            let left: u32 = h.read(entry(h, g, e) + 0xc);
            if (left as i32) >= 0 {
                if left == 0 {
                    for (field, child) in [(0u32, 0x1cu32), (4, 0xc)] {
                        let obj: u32 = h.read(entry(h, g, e) + field);
                        if obj == 0 {
                            continue;
                        }
                        if h.read::<u32>(entry(h, g, e) + 8) == 0 {
                            let c: u32 = h.read(obj + child);
                            if c != 0 {
                                h.call_virtual(c, 0, &[1]);
                            }
                            let obj: u32 = h.read(entry(h, g, e) + field);
                            h.write(obj + child, 0u32);
                        }
                        let obj: u32 = h.read(entry(h, g, e) + field);
                        if obj != 0 {
                            h.call_virtual(obj, 0, &[1]);
                        }
                        let p = entry(h, g, e) + field;
                        h.write(p, 0u32);
                    }
                }
                let p = entry(h, g, e) + 0xc;
                let v: u32 = h.read(p);
                h.write(p, v.wrapping_sub(1));
                count = h.read(g);
            }
            i += 1;
        }
    }

    /// Entry `e` (a byte offset) of the table, its pointer reread.
    fn entry(h: &impl Host, g: u32, e: u32) -> u32 {
        h.read::<u32>(g + 4).wrapping_add(e)
    }

    /// `ClothInstancingDynamicMeshInstance::~ClothInstancingDynamicMeshInstance`
    /// (`0x006e0ab0`, thiscall): its vtable, then for each of the 128
    /// slots the buffers `+0x1c10`, `+0x10`, `+0x210`, `+0x410` (`+ 4 i`)
    /// deleted and cleared, then the `PDynamicMeshInstance` dtor.
    pub fn instance_dtor(h: &mut impl Host, inst: u32) {
        let vt = h.rebase(CLOTH_INSTANCE_VTABLE);
        h.write(inst, vt);
        let del = h.rebase(OPERATOR_DELETE);
        for i in 0..0x80u32 {
            let base = inst + 0x10 + 4 * i;
            for at in [0x1c00, 0, 0x200, 0x400] {
                let p: u32 = h.read(base + at);
                if p != 0 {
                    h.call(del, Conv::Cdecl, &[p]);
                }
                h.write(base + at, 0u32);
            }
        }
        let f = h.rebase(MESH_INSTANCE_DTOR);
        h.call(f, Conv::Thiscall(inst), &[]);
    }

    /// The dynamic meshes' `copyBuffers(x, src, outs, mesh)` (`ret 0x10`)
    /// -> 0 (`0x00674570` distortion grid, `0x00677b10` broken screen):
    /// the source's `+0x20` vertices' positions (12 bytes each, `+0x10`),
    /// colours (16, `+0x14`) and coordinates (8, `+0x18`) into the three
    /// outputs' (`outs[0..3]`, null skipped as a null destination)
    /// buffers `+0x3c`, which get the count at `+4`; the `+0x24` index
    /// words (`+0x1c`) into the mesh's `+0x38`, the count at its `+8`;
    /// the source's `+4` cleared.
    pub fn copy_buffers(h: &mut impl Host, src: u32, outs: u32, mesh: u32) -> u32 {
        let mut d = [0u32; 3];
        for (k, slot) in d.iter_mut().enumerate() {
            let o: u32 = h.read(outs + 4 * k as u32);
            if o != 0 {
                *slot = h.read(o + 0x3c);
            }
        }
        let [d0, d1, d2] = d;
        let n: u32 = h.read(src + 0x20);
        let indices: u32 = h.read(src + 0x24);
        let f = h.rebase(MEMCPY);
        for (d, at, size) in [(d0, 0x10, 12u32), (d1, 0x14, 16), (d2, 0x18, 8)] {
            let s: u32 = h.read(src + at);
            h.call(f, Conv::Cdecl, &[d, s, n.wrapping_mul(size)]);
        }
        for k in 0..3 {
            let o: u32 = h.read(outs + 4 * k);
            if o != 0 {
                h.write(o + 4, n);
            }
        }
        let out: u32 = h.read(mesh + 0x38);
        for i in 0..indices {
            let from: u32 = h.read(src + 0x1c);
            let v: u32 = h.read(from.wrapping_add(4 * i));
            h.write(out.wrapping_add(4 * i), v);
        }
        h.write(mesh + 8, indices);
        if src != 0 {
            h.write(src + 4, 0u32);
        }
        0
    }
}

/// `Phyre_PhyreMalloc_0042fb80` (cdecl `(size, align)`).
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;

pub mod bridge2 {
    //! The originals over game memory.

    use super::{COUNT, Conv, G_CLOTH_ENTRIES, Host, PHYRE_MALLOC};
    use crate::PHYRE_FREE;

    /// `ClothInstancing::terminate(cloth, x)` (`0x006e2e40`, thiscall, `ret
    /// 4`; `x` unused, Ghidra shows a fastcall of one):
    /// queues the cloth's objects for release ([`super::bridge::
    /// release_expired`]): the entry `{[cloth], [cloth + 0x10], [cloth +
    /// 8], countdown 4}` goes into the first free entry (a negative
    /// countdown), else at the end of a table one entry longer
    /// (`Phyre_PhyreMalloc(16 (n + 1), 4)`; the old one copied, and freed
    /// when owned). Nothing changes when that allocation fails.
    pub fn terminate(h: &mut impl Host, cloth: u32) {
        let g = h.rebase(G_CLOTH_ENTRIES);
        let n = h.read::<u32>(g) & COUNT;
        let mut free = None;
        if n != 0 {
            let base: u32 = h.read(g + 4);
            for i in 0..n {
                let e = base.wrapping_add(16 * i);
                if h.read::<i32>(e + 0xc) < 0 {
                    free = Some(e);
                    break;
                }
            }
        }
        let entry = [
            h.read::<u32>(cloth),
            h.read::<u32>(cloth + 0x10),
            h.read::<u32>(cloth + 8),
            4,
        ];
        if let Some(e) = free {
            for (k, w) in entry.into_iter().enumerate() {
                h.write(e + 4 * k as u32, w);
            }
            return;
        }
        let size = ((h.read::<u32>(g) & COUNT) + 1) << 4;
        let p = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[size, 4]);
        if p == 0 {
            return;
        }
        let n = h.read::<u32>(g) & COUNT;
        let mut src: u32 = h.read(g + 4);
        let mut dst = p;
        for _ in 0..n {
            let d = dst;
            dst = dst.wrapping_add(16);
            // the copy skips (without moving on) a null destination
            if d != 0 {
                for k in 0..4 {
                    let w: u32 = h.read(src + 4 * k);
                    h.write(d + 4 * k, w);
                }
                src = src.wrapping_add(16);
            }
        }
        let last = p.wrapping_add(n << 4);
        if last != 0 {
            for (k, w) in entry.into_iter().enumerate() {
                h.write(last + 4 * k as u32, w);
            }
        }
        let old: u32 = h.read(g + 4);
        if old != p && h.read::<u32>(g) >> 31 == 0 && old != 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
        }
        h.write(g + 4, p);
        h.write(g, n + 1);
    }
}

/// `Phyre_PhyreMalloc` (cdecl `(size)`).
pub const PHYRE_MALLOC1: u32 = 0x0042_fb40;
/// `Phyre_PGeometry_PDynamicMesh_PDynamicMesh` (thiscall `(mesh)`).
pub const DYNAMIC_MESH_CTOR: u32 = 0x0049_68c0;
/// `Phyre_ClothInstancingDynamicMesh_vftable`.
pub const CLOTH_MESH_VTABLE: u32 = 0x00b4_aa10;
/// `Phyre_DynamicMeshDefaultImplmenetation_vftable` (sic).
pub const DEFAULT_IMPL_VTABLE: u32 = 0x00b4_a8f0;
/// The stream semantic a new cloth mesh starts with (`.data`).
pub const CLOTH_STREAM: u32 = 0x00c9_8650;
/// `ClothInstancingDynamicMeshInstance_ctor_006e0860` (thiscall `(inst,
/// mesh)`).
pub const CLOTH_INSTANCE_CTOR: u32 = 0x006e_0860;
/// `Phyre_PRendering_PDynamicMeshInstance_PDynamicMeshInstance`
/// (thiscall `(inst, mesh)`).
pub const MESH_INSTANCE_CTOR: u32 = 0x004f_a420;
/// `0x006e17f0` (thiscall `(mesh, src)`, `ret 4`): sets the cloth mesh up
/// from the source.
pub const CLOTH_SETUP: u32 = 0x006e_17f0;

pub mod bridge3 {
    //! The originals over game memory.

    use super::{
        CLOTH_INSTANCE_CTOR, CLOTH_MESH_VTABLE, CLOTH_SETUP, CLOTH_STREAM, Conv,
        DEFAULT_IMPL_VTABLE, DYNAMIC_MESH_CTOR, Host, MESH_INSTANCE_CTOR, PHYRE_MALLOC1,
    };

    /// `ClothInstancing_attach(ci, src)` (`0x006e2600`, thiscall, `ret 4`)
    /// -> the new instance (`ci + 0x10`): a `ClothInstancingDynamicMesh`
    /// (0x28 bytes: the `PDynamicMesh` base, its vtable, `+0x1c = 0`,
    /// `+0x20` its stream, `+0x24 = 3`) into `ci + 0`, its instance
    /// (0x2010 bytes) into `ci + 0x10`. With the source's own instance
    /// (`src + 0x18`, kept at `ci + 4`) the cloth mesh takes that
    /// instance's mesh (`ci + 8`) and the new instance points at it;
    /// without one, a default implementation mesh (0x1c bytes) and a
    /// `PDynamicMeshInstance` (0xc bytes) of it. Then `0x006e17f0(mesh,
    /// src)`, and the source's instance becomes the new one. Also returns
    /// the word the original leaves in its argument's slot (the last
    /// allocation of the second path; `src` on the first).
    pub fn attach(h: &mut impl Host, ci: u32, src: u32) -> (u32, u32) {
        let mut slot = src;
        let p = h.call_sym(PHYRE_MALLOC1, Conv::Cdecl, &[0x28]);
        let mesh = if p == 0 {
            0
        } else {
            h.call_sym(DYNAMIC_MESH_CTOR, Conv::Thiscall(p), &[]);
            h.write(p, h.rebase(CLOTH_MESH_VTABLE));
            h.write(p + 0x1c, 0u32);
            h.write(p + 0x20, h.rebase(CLOTH_STREAM));
            h.write(p + 0x24, 3u32);
            p
        };
        h.write(ci, mesh);
        let q = h.call_sym(PHYRE_MALLOC1, Conv::Cdecl, &[0x2010]);
        let inst = if q == 0 {
            0
        } else {
            let m: u32 = h.read(ci);
            h.call_sym(CLOTH_INSTANCE_CTOR, Conv::Thiscall(q), &[m])
        };
        h.write(ci + 0x10, inst);
        let own: u32 = h.read(src + 0x18);
        h.write(ci + 4, own);
        if own == 0 {
            // (the original keeps this allocation in its argument's slot
            // too, until the next one replaces it)
            let r = h.call_sym(PHYRE_MALLOC1, Conv::Cdecl, &[0x1c]);
            let imp = if r == 0 {
                0
            } else {
                h.call_sym(DYNAMIC_MESH_CTOR, Conv::Thiscall(r), &[]);
                h.write(r, h.rebase(DEFAULT_IMPL_VTABLE));
                r
            };
            let m: u32 = h.read(ci);
            h.write(m + 0x1c, imp);
            let s = h.call_sym(PHYRE_MALLOC1, Conv::Cdecl, &[0xc]);
            slot = s;
            let sub = if s == 0 {
                0
            } else {
                let m: u32 = h.read(ci);
                let imp: u32 = h.read(m + 0x1c);
                h.call_sym(MESH_INSTANCE_CTOR, Conv::Thiscall(s), &[imp])
            };
            let inst: u32 = h.read(ci + 0x10);
            h.write(inst + 0xc, sub);
        } else {
            let m: u32 = h.read(own + 4);
            h.write(ci + 8, m);
            h.write(inst + 0xc, own);
            let mesh: u32 = h.read(ci);
            let m: u32 = h.read(ci + 8);
            h.write(mesh + 0x1c, m);
        }
        let mesh: u32 = h.read(ci);
        h.call_sym(CLOTH_SETUP, Conv::Thiscall(mesh), &[src]);
        let inst: u32 = h.read(ci + 0x10);
        h.write(src + 0x18, inst);
        (inst, slot)
    }
}
