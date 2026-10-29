//! M5 stage 1, list `phyre-render-b`: the compute and geometry shader
//! programs' D3D11 create (`0x0058d030`, `0x0058d350`), the same shape as
//! the pixel and vertex programs' (`ffx_pload::p1_d3d`): under the render
//! scope lock (`0x00463d50`/`0x00463e90` on a frame word, `ebp - 0x10`),
//! with the device holder `0x00c94f00` made on first use, the program's
//! shader (`+0x4e0`) created from its bytecode when the device exists and
//! the feature level allows the profile; then the constant buffer and the
//! sixteen default sampler words.

use ffx_graphic::host::{Conv, Host};

/// The render scope lock's enter and leave (thiscall on the lock word).
pub const SCOPE_ENTER: u32 = 0x0046_3d50;
pub const SCOPE_LEAVE: u32 = 0x0046_3e90;
/// The holder's guard word, the holder, its constructor and destructor,
/// `atexit`, its device word and its first default sampler word.
pub const HOLDER_GUARD: u32 = 0x00c9_628c;
pub const HOLDER: u32 = 0x00c9_4f00;
pub const HOLDER_CTOR: u32 = 0x004f_ee70;
pub const HOLDER_DTOR: u32 = 0x00af_b710;
pub const ATEXIT: u32 = 0x0094_9207;
pub const HOLDER_DEVICE: u32 = 0x00c9_4f6c;
pub const HOLDER_SAMPLER: u32 = 0x00c9_6218;
/// `PRendererContext_instance` (cdecl).
pub const RENDERER_CONTEXT: u32 = 0x0046_49d0;
/// `PhyrePrintf(level, format)`.
pub const PRINTF: u32 = 0x0043_52f0;
/// The program's constant buffer create (thiscall).
pub const CONSTANT_BUFFER_CREATE: u32 = 0x0058_f490;

/// A shader program kind: its profiles, its `ID3D11Device` method (a
/// vtable word index) and its "profile not supported" message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Kind {
    pub profile_50: u32,
    pub profile_41: u32,
    pub slot: u32,
    pub message: u32,
}

/// `PShaderComputeProgramD3D11` (`cs50`/`cs41`, `CreateComputeShader`).
pub const COMPUTE: Kind = Kind {
    profile_50: 0x6373_3530,
    profile_41: 0x6373_3431,
    slot: 18,
    message: 0x00b3_3a44,
};
/// `PShaderGeometryProgramD3D11` (`gs50`/`gs41`, `CreateGeometryShader`).
pub const GEOMETRY: Kind = Kind {
    profile_50: 0x6773_3530,
    profile_41: 0x6773_3431,
    slot: 13,
    message: 0x00b3_39e8,
};

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;

    fn holder(h: &mut impl Host) {
        let g = h.rebase(HOLDER_GUARD);
        let v: u32 = h.read(g);
        if v & 1 == 0 {
            h.write(g, v | 1);
            let holder = h.rebase(HOLDER);
            h.call_sym(HOLDER_CTOR, Conv::Thiscall(holder), &[]);
            let dtor = h.rebase(HOLDER_DTOR);
            h.call_sym(ATEXIT, Conv::Cdecl, &[dtor]);
        }
    }

    fn level(h: &mut impl Host) -> i32 {
        let ctx = h.call_sym(RENDERER_CONTEXT, Conv::Cdecl, &[]);
        h.read(ctx.wrapping_add(0xb4))
    }

    /// The program's create `(this)` for `k`; `lock` is the original's
    /// `ebp - 0x10`. 0.
    pub fn create(h: &mut impl Host, this: u32, lock: u32, k: Kind) -> u32 {
        // the prologue's `push ecx` reserves the lock word with `this`
        h.write(lock, this);
        let out = this.wrapping_add(0x4e0);
        if h.read::<u32>(out) == 0 {
            h.call_sym(SCOPE_ENTER, Conv::Thiscall(lock), &[]);
            holder(h);
            if h.read::<u32>(h.rebase(HOLDER_DEVICE)) == 0 {
                h.call_sym(SCOPE_LEAVE, Conv::Thiscall(lock), &[]);
                return 0;
            }
            // the 4.1 check reads the profile again, and is skipped when
            // a 5.0 profile fails
            let profile: u32 = h.read(this + 0x4dc);
            let refused = (profile == k.profile_50 && level(h) < 0xb000)
                || (h.read::<u32>(this + 0x4dc) == k.profile_41 && level(h) < 0xa100);
            if refused {
                let msg = h.rebase(k.message);
                h.call_sym(PRINTF, Conv::Cdecl, &[1, msg]);
            } else {
                let ctx = h.call_sym(RENDERER_CONTEXT, Conv::Cdecl, &[]);
                let dev: u32 = h.read(ctx.wrapping_add(0x6c));
                let vt: u32 = h.read(dev);
                let len = h.read::<u32>(this + 0xc) & 0x7fff_ffff;
                let code: u32 = h.read(this + 0x10);
                let f: u32 = h.read(vt.wrapping_add(4 * k.slot));
                h.call(f, Conv::Thiscall(vt), &[dev, code, len, 0, out]);
            }
            h.call_sym(SCOPE_LEAVE, Conv::Thiscall(lock), &[]);
        }
        h.call_sym(CONSTANT_BUFFER_CREATE, Conv::Thiscall(this), &[]);
        holder(h);
        let src = h.rebase(HOLDER_SAMPLER);
        for i in 0..16 {
            let v: u32 = h.read(src);
            h.write(this + 0x21c + 4 * i, v);
        }
        0
    }
}

/// `DynGeoTree_Find(tree; &key)`, `Phyre_PRedBlackTree_remove(tree; node)`,
/// `Phyre_PFreeList_deallocate(list; node)`, `Phyre_PCluster_deleteObject(cluster;
/// obj)` (thiscall).
pub const TREE_FIND: u32 = 0x006d_edd0;
pub const TREE_REMOVE: u32 = 0x0057_5000;
pub const FREE_LIST_FREE: u32 = 0x0049_c980;
pub const DELETE_OBJECT: u32 = 0x0044_1240;
/// `Phyre_PhyreFree` (cdecl).
pub const PHYRE_FREE: u32 = 0x0042_fb20;

/// The state one `restoreCharacterShader` copy works on: the two maps
/// (red-black trees keyed by the instance at `+0xc`, their free lists
/// at `+0x18`), the two words it keeps the found values in, and the
/// cluster the replaced objects are deleted from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Restore {
    pub tree: u32,
    pub tree2: u32,
    pub objects: u32,
    pub material: u32,
    pub cluster: u32,
}

/// `Phyre_CharacterShaderManager_restoreCharacterShader` (`0x006df6d0`).
pub const SHADER_MANAGER: Restore = Restore {
    tree: 0x00c3_4b4c,
    tree2: 0x00c3_4b1c,
    objects: 0x00cd_ee14,
    material: 0x00cd_ee18,
    cluster: 0x00cd_ee20,
};
/// `Phyre_PS2LightingSetup_restoreCharacterShader` (`0x006e5ee0`).
pub const PS2_LIGHTING: Restore = Restore {
    tree: 0x00c3_4ffc,
    tree2: 0x00c3_4fcc,
    objects: 0x00ce_63dc,
    material: 0x00ce_63e0,
    cluster: 0x00ce_6d70,
};

pub mod restore {
    //! `restoreCharacterShader(inst)` over game memory.

    use super::{DELETE_OBJECT, FREE_LIST_FREE, PHYRE_FREE, Restore, TREE_FIND, TREE_REMOVE};
    use ffx_graphic::host::{Conv, Host};

    fn this_call(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    /// A `PSharray`'s data: the buffer for more than one element, the
    /// word itself for one, 0 for none.
    fn data(h: &mut impl Host, arr: u32) -> u32 {
        let n = h.read::<u32>(arr) & 0x7fff_ffff;
        if n > 1 {
            h.read(arr + 4)
        } else if n != 0 {
            arr + 4
        } else {
            0
        }
    }

    /// The map node keyed `key` taken out of `tree` (left when the
    /// node's key is greater, right when less) and given back to its free
    /// list.
    fn remove(h: &mut impl Host, tree: u32, key: u32) {
        let mut node: u32 = h.read(tree + 0xc);
        while node != tree {
            let k: u32 = h.read(node + 0xc);
            if k == key {
                this_call(h, TREE_REMOVE, tree, &[node]);
                this_call(h, FREE_LIST_FREE, tree + 0x18, &[node]);
                return;
            }
            node = h.read(if k > key { node } else { node + 4 });
        }
    }

    /// `restoreCharacterShader(inst)` (cdecl, `key` the address of its
    /// argument, which the map lookups take): with the instance in the
    /// shader map, its saved material (or `[inst] + 0x30` when none) back
    /// at `+0x10`, the saved objects deleted from the cluster and their
    /// array emptied, both maps' entries for it removed, the saved words
    /// cleared. Nothing without an entry.
    pub fn restore(h: &mut impl Host, r: Restore, inst: u32, key: u32) {
        let tree = h.rebase(r.tree);
        let tree2 = h.rebase(r.tree2);
        let n1 = this_call(h, TREE_FIND, tree, &[key]);
        let v1 = if n1 == 0 { 0 } else { n1 + 0x10 };
        let n2 = this_call(h, TREE_FIND, tree2, &[key]);
        let v2 = if n2 == 0 { 0 } else { n2 + 0x10 };
        if v1 == 0 {
            return;
        }
        let objects_at = h.rebase(r.objects);
        let material_at = h.rebase(r.material);
        let mut material: u32 = h.read(v1);
        h.write(material_at, material);
        let objects: u32 = h.read(v2);
        h.write(objects_at, objects);
        if material == 0 {
            material = h.read::<u32>(inst).wrapping_add(0x30);
        }
        h.write(inst + 0x10, material);
        let cluster = h.rebase(r.cluster);
        let n = h.read::<u32>(objects) & 0x7fff_ffff;
        for i in 0..n {
            let arr: u32 = h.read(objects_at);
            let d = data(h, arr);
            let obj: u32 = h.read(d.wrapping_add(4 * i));
            this_call(h, DELETE_OBJECT, cluster, &[obj]);
            let arr: u32 = h.read(objects_at);
            if i < h.read::<u32>(arr) & 0x7fff_ffff {
                let d = data(h, arr);
                h.write(d.wrapping_add(4 * i), 0u32);
            }
        }
        let arr: u32 = h.read(objects_at);
        let count: u32 = h.read(arr);
        if count != 0 {
            let d = data(h, arr);
            if d != arr + 4 && count & 0x8000_0000 == 0 {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[d]);
            }
            h.write(arr, 0u32);
        }
        remove(h, tree, inst);
        remove(h, tree2, inst);
        let arr: u32 = h.read(objects_at);
        this_call(h, DELETE_OBJECT, cluster, &[arr]);
        h.write(objects_at, 0u32);
        h.write(material_at, 0u32);
    }
}
