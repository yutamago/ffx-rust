//! The game's dynamic meshes (`ClassDynamicMesh`, the distortion grid,
//! the radial lines, the broken-screen polygons, cloth instancing): their
//! instance creation, stream tables and per-buffer sizes.
//!
//! The meshes keep `PSharray`-style arrays: a count word whose bit 31
//! means "not owned", then the pointer. Replacing one frees the old
//! storage when it is owned, non-null and not the new one.

use ffx_graphic::host::{Conv, Host};

/// `Phyre_PhyreMalloc_0042fb80` (cdecl `(size, align)`).
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
/// `Phyre_PCluster_allocateInstanceList` (thiscall `(cluster, n, desc,
/// x)`, `ret 0xc`).
pub const ALLOCATE_INSTANCE_LIST: u32 = 0x0044_0b70;
/// `DistortionGridDynamicMeshInstance`'s class descriptor.
pub const DISTORTION_DESC: u32 = 0x00cc_bce0;
/// `DistortionGridDynamicMeshInstance_ctor` (thiscall `(this, mesh)`).
pub const DISTORTION_CTOR: u32 = 0x0067_4bf0;
/// `RadialLineDynamicMeshInstance`'s class descriptor.
pub const RADIAL_DESC: u32 = 0x00cc_bdb8;
/// `RadialLineDynamicMeshInstance_ctor` (thiscall `(this, mesh)`).
pub const RADIAL_CTOR: u32 = 0x0067_5510;
/// `graphicDynGeoMemManagerGetCurrentBufferId()` (cdecl).
pub const CURRENT_BUFFER_ID: u32 = 0x0063_ff50;
/// The dynamic-geometry manager's "buffers on" word.
pub const DYN_GEO_BUFFERS: u32 = 0x00c3_2694;
/// `0x00498f30` (thiscall `(array, n)`, `ret 4`): resizes a 16-byte
/// element `PSharray` to `n`.
pub const RESIZE16: u32 = 0x0049_8f30;
/// `Phyre_PRendering_PDynamicMeshInstance_PDynamicMeshInstance`
/// (thiscall `(this, mesh)`).
pub const MESH_INSTANCE_CTOR: u32 = 0x004f_a420;
/// `ClothInstancingDynamicMeshInstance`'s vtable.
pub const CLOTH_INSTANCE_VTABLE: u32 = 0x00b4_ab28;
/// `crt_memset_thunk` (cdecl `(p, value, n)`).
pub const MEMSET: u32 = 0x0094_964c;
/// The three stream semantics [`bridge::set_streams`] names (`.data`).
pub const STREAM_NAMES: [u32; 3] = [0x00c9_85b4, 0x00c9_8650, 0x00c9_8638];
/// The four [`bridge::set_streams4`] names.
pub const STREAMS4: [u32; 4] = [0x00c9_8620, 0x00c9_862c, 0x00c9_8650, 0x00c9_8638];

/// `0x00ff34`: an empty stream entry's type word.
const EMPTY_STREAM: u16 = 0xff34;

/// Whether a `PSharray` count word owns its storage (bit 31 clear).
#[must_use]
pub fn owns(count: u32) -> bool {
    count >> 31 == 0
}

pub mod bridge {
    //! The originals over game memory.

    use super::{
        ALLOCATE_INSTANCE_LIST, CLOTH_INSTANCE_VTABLE, CURRENT_BUFFER_ID, Conv, DYN_GEO_BUFFERS,
        EMPTY_STREAM, Host, MEMSET, MESH_INSTANCE_CTOR, PHYRE_MALLOC, RESIZE16, STREAM_NAMES,
        STREAMS4, owns,
    };
    use crate::PHYRE_FREE;

    /// Replaces the array at `arr` (count word, pointer) with `n`
    /// elements at `p`, freeing the old storage as the originals do.
    fn replace(h: &mut impl Host, arr: u32, p: u32, n: u32) {
        let old: u32 = h.read(arr + 4);
        let count: u32 = h.read(arr);
        if old != p && owns(count) && old != 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
        }
        h.write(arr + 4, p);
        h.write(arr, n);
    }

    /// The stream table at `+8` (count) / `+0xc`: reallocated
    /// (`Phyre_PhyreMalloc(0xc n, 4)`, every entry `{0, 0, 0xff34}`)
    /// unless it already has `n` entries, or when that fails.
    fn stream_table(h: &mut impl Host, mesh: u32, n: u32) {
        if h.read::<u32>(mesh + 8) == n {
            return;
        }
        let p = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[0xc * n, 4]);
        if p == 0 {
            return;
        }
        for k in 0..n {
            // the original tests each entry's address for null
            let e = p.wrapping_add(0xc * k);
            if e != 0 {
                h.write(e, 0u32);
                h.write(e + 4, 0u32);
                h.write(e + 8, EMPTY_STREAM);
            }
        }
        replace(h, mesh + 8, p, n);
    }

    /// Names the stream entries (`{0, semantic, components}`; the table
    /// is reread for each entry), and returns the table (`+0xc`), as the
    /// originals leave it in `eax`.
    fn name_streams(h: &mut impl Host, mesh: u32, streams: &[(u32, u8)]) -> u32 {
        for (i, &(name, components)) in streams.iter().enumerate() {
            let s: u32 = h.read(mesh + 0xc);
            let e = s.wrapping_add(0xc * i as u32);
            let name = h.rebase(name);
            if i == 0 {
                // the first entry's words go in the order zero, count, name
                h.write(e, 0u32);
                h.write(e + 8, components);
                h.write(e + 4, name);
            } else {
                h.write(e + 8, components);
                h.write(e + 4, name);
                h.write(e, 0u32);
            }
        }
        h.read(mesh + 0xc)
    }

    /// `0x00676680` / `0x00677ce0` (thiscall `(mesh, a, b, c)`, `ret 0xc`)
    /// and `0x00674740` (thiscall `(mesh, b, c)`, `ret 8`, `a = None`):
    /// `+0x1c = a`; a one-element draw table at `+0x10` (`{0, 3, b, c}`,
    /// reallocated (`Phyre_PhyreMalloc(0x10, 4)`) unless it has one) and
    /// three stream entries (the semantics [`STREAM_NAMES`], component
    /// counts 2, 3, 1). Returns the stream table.
    pub fn set_streams(h: &mut impl Host, mesh: u32, a: Option<u32>, b: u32, c: u32) -> u32 {
        if let Some(a) = a {
            h.write(mesh + 0x1c, a);
        }
        if h.read::<u32>(mesh + 0x10) != 1 {
            let p = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[0x10, 4]);
            if p != 0 {
                for at in [0, 4, 8, 0xc] {
                    h.write(p + at, 0u32);
                }
                replace(h, mesh + 0x10, p, 1);
            }
        }
        stream_table(h, mesh, 3);
        let draw: u32 = h.read(mesh + 0x14);
        h.write(draw, 0u32);
        let draw: u32 = h.read(mesh + 0x14);
        h.write(draw + 4, 3u32);
        let draw: u32 = h.read(mesh + 0x14);
        h.write(draw + 8, b);
        let draw: u32 = h.read(mesh + 0x14);
        h.write(draw + 0xc, c);
        let [n0, n1, n2] = STREAM_NAMES;
        name_streams(h, mesh, &[(n0, 2), (n1, 3), (n2, 1)])
    }

    /// `0x00673030` (thiscall `(mesh, src)`, `ret 4`): the draw table
    /// (`+0x10`, sized by `0x00498f30` to the source's count `+0x90`)
    /// gets `{0, 4, rec +4, rec +8}` from each of the source's 0x6c-byte
    /// records (`+0x94`); four stream entries ([`STREAMS4`], component
    /// counts 2, 2, 3, 1). Returns the stream table.
    pub fn set_streams4(h: &mut impl Host, mesh: u32, src: u32) -> u32 {
        let n: i32 = h.read(src + 0x90);
        let records: u32 = h.read(src + 0x94);
        h.call_sym(RESIZE16, Conv::Thiscall(mesh + 0x10), &[n as u32]);
        stream_table(h, mesh, 4);
        let mut k = 0i32;
        while k < n {
            let e = (k as u32).wrapping_mul(0x10);
            let rec = records.wrapping_add((k as u32).wrapping_mul(0x6c));
            let t: u32 = h.read(mesh + 0x14);
            h.write(e.wrapping_add(t), 0u32);
            let t: u32 = h.read(mesh + 0x14);
            h.write(e.wrapping_add(t) + 4, 4u32);
            let t: u32 = h.read(mesh + 0x14);
            let v: u32 = h.read(rec + 4);
            h.write(e.wrapping_add(t) + 8, v);
            let t: u32 = h.read(mesh + 0x14);
            let v: u32 = h.read(rec + 8);
            h.write(e.wrapping_add(t) + 0xc, v);
            k += 1;
        }
        let [a, b, c, d] = STREAMS4;
        name_streams(h, mesh, &[(a, 2), (b, 2), (c, 3), (d, 1)])
    }

    /// `0x00673e20` / `0x00675d90` (thiscall `(cluster, p)`, `ret 4`; Ghidra
    /// shows a stdcall, but the entry `ecx` goes on to the allocator): one
    /// instance of the class `desc` (`cluster.allocateInstanceList(1,
    /// desc, 0)`, its storage `+0x34`) built by `ctor(storage, *p)`, whose
    /// `eax` it returns; 0 when there is no list or no storage.
    pub fn create_instance(h: &mut impl Host, cluster: u32, desc: u32, ctor: u32, p: u32) -> u32 {
        let d = h.rebase(desc);
        let list = h.call_sym(ALLOCATE_INSTANCE_LIST, Conv::Thiscall(cluster), &[1, d, 0]);
        if list == 0 {
            return 0;
        }
        let storage: u32 = h.read(list + 0x34);
        if storage == 0 {
            return 0;
        }
        let mesh: u32 = h.read(p);
        h.call_sym(ctor, Conv::Thiscall(storage), &[mesh])
    }

    /// `0x006732a0` (thiscall `(mesh, src)`, `ret 4`): sizes the mesh's
    /// draw table (`+0x10`, 16-byte entries) to the source's count
    /// (`+0x90`) and fills each entry's `+8`/`+0xc` from the source's
    /// 0x6c-byte records (`+0x94`): the current dynamic buffer's pair
    /// (`+0x2c + 0x1c id`, `+0x30`) when buffers are on and its `+0x34`
    /// is set, else the record's `+4`/`+8`; an empty entry (0 at `+8`)
    /// becomes `{4, 6}`. Returns the word the original leaves in its
    /// argument's slot (the records' running offset, `0x6c n` after a
    /// non-empty loop, else `src`).
    pub fn fill_draw_table(h: &mut impl Host, mesh: u32, src: u32) -> u32 {
        let n: i32 = h.read(src + 0x90);
        let id = h.call_sym(CURRENT_BUFFER_ID, Conv::Cdecl, &[]) as i32;
        let records: u32 = h.read(src + 0x94);
        h.call_sym(RESIZE16, Conv::Thiscall(mesh + 0x10), &[n as u32]);
        let buffers = h.rebase(DYN_GEO_BUFFERS);
        let mut i = 0i32;
        while i < n {
            let rec = records.wrapping_add((i as u32).wrapping_mul(0x6c));
            let entry = (i as u32).wrapping_mul(0x10);
            let mut from = None;
            if id >= 0 && h.read::<u32>(buffers) != 0 {
                let b = rec.wrapping_add((id as u32).wrapping_mul(0x1c));
                if h.read::<u32>(b + 0x34) != 0 {
                    from = Some((b + 0x2c, b + 0x30));
                }
            }
            let (first, second) = from.unwrap_or((rec + 4, rec + 8));
            let v: u32 = h.read(first);
            let t: u32 = h.read(mesh + 0x14);
            h.write(entry.wrapping_add(t) + 8, v);
            let v: u32 = h.read(second);
            let t: u32 = h.read(mesh + 0x14);
            h.write(entry.wrapping_add(t) + 0xc, v);
            let t: u32 = h.read(mesh + 0x14);
            if h.read::<u32>(entry.wrapping_add(t) + 8) == 0 && i < n {
                h.write(entry.wrapping_add(t) + 8, 4u32);
                let t: u32 = h.read(mesh + 0x14);
                h.write(entry.wrapping_add(t) + 0xc, 6u32);
            }
            i += 1;
        }
        if n > 0 {
            (n as u32).wrapping_mul(0x6c)
        } else {
            src
        }
    }

    /// `ClothInstancingDynamicMeshInstance(inst, mesh)` (`0x006e0860`,
    /// `ret 4`) -> `inst`: the `PDynamicMeshInstance` base, its vtable,
    /// `+0xc = 0`, fourteen 0x200-byte blocks cleared, and the 0x80 words
    /// at `+0x610` and `+0x810` set to 1.
    pub fn cloth_instance_ctor(h: &mut impl Host, inst: u32, mesh: u32) -> u32 {
        h.call_sym(MESH_INSTANCE_CTOR, Conv::Thiscall(inst), &[mesh]);
        h.write(inst, h.rebase(CLOTH_INSTANCE_VTABLE));
        h.write(inst + 0xc, 0u32);
        for at in [
            0xa10, 0x1010, 0x1210, 0xc10, 0xe10, 0x1410, 0x1810, 0x1610, 0x1a10, 0x1c10, 0x1e10,
            0x10, 0x210, 0x410,
        ] {
            h.call_sym(MEMSET, Conv::Cdecl, &[inst + at, 0, 0x200]);
        }
        for k in 0..0x80 {
            h.write(inst + 0x610 + 4 * k, 1u32);
            h.write(inst + 0x810 + 4 * k, 1u32);
        }
        inst
    }
}

/// `operator_new` (cdecl `(size)`).
pub const OPERATOR_NEW: u32 = 0x0063_0310;
/// `MenuDynamicGeometry_MenuDynamicGeometry` (thiscall `(geom)` -> geom).
pub const MENU_GEOMETRY_CTOR: u32 = 0x0069_e320;
/// `ClassDynamicGeometry_hide` (thiscall `(geom)`).
pub const GEOMETRY_HIDE: u32 = 0x006a_2370;
/// `g_PhyreScene`.
pub const PHYRE_SCENE: u32 = 0x00cc_b9d8;

/// For each menu geometry kind `0x00681970` knows: its scene's hidden
/// byte and its value word (`g_PhyreScene + offset`).
const MENU_KINDS: [(i32, u32, u32); 6] = [
    (1, 0xfa9d, 0xfa84),
    (-1, 0xfaa0, 0xfa94),
    (-2, 0xfa9f, 0xfa90),
    (0, 0xfa9a, 0xfa78),
    (2, 0xfa98, 0xfa70),
    (3, 0xfa99, 0xfa74),
];

pub mod bridge2 {
    //! The originals over game memory.

    use super::{
        Conv, GEOMETRY_HIDE, Host, MENU_GEOMETRY_CTOR, MENU_KINDS, OPERATOR_NEW, PHYRE_SCENE,
    };

    /// `MenuDynamicGeometry_create(owner, kind)` (`0x00681970`, stdcall,
    /// `ret 8`) -> the new geometry, sign-extended into `edx:eax` (the
    /// original ends `mov eax, edi; cdq`), and the word it leaves in its
    /// `kind` argument's slot (`operator_new`'s result). For the kinds of
    /// [`MENU_KINDS`] the scene says whether that menu part is hidden and,
    /// if not, the value to show; any other kind is its own value. A new
    /// `MenuDynamicGeometry` (0x3c bytes) gets `owner` at `+0x24`, then
    /// either `vtable[1](value, scene + 0x1284)` and `vtable[2](0)`, or
    /// `ClassDynamicGeometry_hide`.
    pub fn create(h: &mut impl Host, owner: u32, kind: u32) -> (u64, u32) {
        let mut hidden = 0u8;
        let mut value = kind;
        if let Some(&(_, flag, word)) = MENU_KINDS.iter().find(|k| k.0 as u32 == kind) {
            let scene: u32 = h.read(h.rebase(PHYRE_SCENE));
            hidden = h.read(scene + flag);
            if hidden == 0 {
                value = h.read(scene + word);
            }
        }
        let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x3c]);
        let geom = if p == 0 {
            0
        } else {
            h.call_sym(MENU_GEOMETRY_CTOR, Conv::Thiscall(p), &[])
        };
        let scene: u32 = h.read(h.rebase(PHYRE_SCENE));
        h.write(geom.wrapping_add(0x24), owner);
        if hidden == 0 {
            h.call_virtual(geom, 1, &[value, scene.wrapping_add(0x1284)]);
            h.call_virtual(geom, 2, &[0]);
        } else {
            h.call_sym(GEOMETRY_HIDE, Conv::Thiscall(geom), &[]);
        }
        (i64::from(geom as i32) as u64, p)
    }
}
