//! The render-side glue, thirteenth batch (M4 port round 4, list
//! `h3-glue-render`): more scene, effect and manager methods. Exception
//! frames and cookies left out, as in [`crate::glue10`].

use ffx_bridge::Global;

/// `Phyre_PhyreMalloc(size)` (cdecl) and the cluster constructor
/// `Phyre_PCluster_operator_new(this)` the broken-screen effect builds
/// its two clusters with.
pub const PHYRE_MALLOC_SIZE: u32 = 0x0042_fb40;
pub const CLUSTER_CTOR: u32 = 0x0057_6630;
/// `g_VFXRenderDataTable`, `GetVFXDataByInstance(this; a, b)`,
/// `operator delete[]`, `operator new[]` (cdecl).
pub const VFX_RENDER_DATA_TABLE: Global<u32> = Global::new(0x00cc_ca6c);
pub const VFX_DATA_BY_INSTANCE: u32 = 0x0069_f6e0;
pub const ARRAY_DELETE: u32 = 0x0063_0540;
pub const ARRAY_NEW: u32 = 0x0063_04c0;
/// The map fog set-up: the default colour, near and far, the density
/// divisor, `Phyre_FogEffectManager_GetInstance` and
/// `FogEffectManager_copyParameters_006e3b80(fog)`.
pub const FOG_DEFAULT_COLOUR: [Global<f32>; 3] = [
    Global::new(0x00b4_3cf8),
    Global::new(0x00b4_3d04),
    Global::new(0x00b3_eb48),
];
pub const FOG_DEFAULT_NEAR: Global<f32> = Global::new(0x00b4_3ddc);
pub const FOG_DEFAULT_FAR: Global<f32> = Global::new(0x00b4_3de0);
pub const FOG_DENSITY_DIVISOR: Global<f64> = Global::new(0x00b1_70c0);
pub const FOG_INSTANCE: u32 = 0x006e_32c0;
pub const FOG_COPY_PARAMETERS: u32 = 0x006e_3b80;
/// The table clear's message (`printf` import slot), `MapFind(this;
/// &out, &key)`, `InsertToDeletePool(this; a, b, 3, geometry)` and the
/// map's clear `0x006a0fe0(this)`.
pub const PRINTF_SLOT: u32 = crate::h3_glue12::PRINTF_SLOT;
pub const TABLE_CLEAR_MESSAGE: u32 = 0x00b4_8004;
pub const MAP_FIND: u32 = 0x006a_1f60;
pub const INSERT_TO_DELETE_POOL: u32 = 0x0069_f900;
pub const MAP_CLEAR: u32 = 0x006a_0fe0;
/// The delegation list's mutex and its import slots, the helper's
/// `setDataToFreeList(this; delegation)` and compaction
/// `0x006ad590(this)`, `operator_delete` (cdecl).
pub const DELEGATION_MUTEX: Global<u32> = crate::glue10::DELEGATION_MUTEX;
pub const WAIT_FOR_SINGLE_OBJECT: u32 = crate::glue::WAIT_FOR_SINGLE_OBJECT;
pub const RELEASE_MUTEX: u32 = crate::glue::RELEASE_MUTEX;
pub const SET_DATA_TO_FREE_LIST: u32 = 0x006a_e1a0;
pub const DELEGATION_COMPACT: u32 = 0x006a_d590;
pub const OPERATOR_DELETE: u32 = crate::glue10::OPERATOR_DELETE;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use crate::glue10::bridge::successor;
    use crate::h3_glue12::bridge::copy_str;
    use ffx_bridge::x87::{self, round_trip};
    use ffx_graphic::host::{Conv, Host};

    /// A new cluster for the broken-screen effect: `PhyreMalloc(0x50)`,
    /// constructed, its list `+0x48` empty; 0 when the allocation fails.
    fn new_cluster(h: &mut impl Host) -> u32 {
        let p = h.call_sym(PHYRE_MALLOC_SIZE, Conv::Cdecl, &[0x50]);
        if p == 0 {
            return 0;
        }
        h.call_sym(CLUSTER_CTOR, Conv::Thiscall(p), &[]);
        h.write(p + 0x48, p + 0x48);
        h.write(p + 0x4c, p + 0x48);
        p
    }

    /// `BrokenScreenEffect_ctor_006d4ba0(this)`: `+8` = 1.0, the flags
    /// and words cleared (`+0`, `+4`, `+0x2c`, `+0x38`, `+0x3c`,
    /// `+0x58..+0x60`, `+0x74`), `+6` = 1, the size 1280 x 720
    /// (`+0x6c`, `+0x70`) and two new clusters (`+0x50`, `+0x54`).
    /// `eax`: `this`. (The C++ exception frame is left out.)
    pub fn broken_screen_ctor(h: &mut impl Host, this: u32) -> u32 {
        h.write(this + 8, 1f32);
        h.write(this, 0u32);
        h.write(this + 4, 0u16);
        h.write(this + 6, 1u8);
        for off in [0x2c, 0x58, 0x5c, 0x60] {
            h.write(this + off, 0u32);
        }
        h.write(this + 0x6c, 0x500u32);
        h.write(this + 0x70, 0x2d0u32);
        h.write(this + 0x74, 0u16);
        h.write(this + 0x3c, 0u32);
        let a = new_cluster(h);
        h.write(this + 0x50, a);
        let b = new_cluster(h);
        h.write(this + 0x54, b);
        h.write(this + 0x38, 0u32);
        this
    }

    /// The length of the C string at `s`.
    fn str_len(h: &impl Host, s: u32) -> u32 {
        let mut n = 0u32;
        while h.read::<u8>(s.wrapping_add(n)) != 0 {
            n = n.wrapping_add(1);
        }
        n
    }

    /// `graphicVFXUpdateTexture(a, b, name)`: for the VFX data of
    /// instance `(a, b)` (nothing when both are 0 or none is found), each
    /// of its texture slots (`+0x94`, 0x6c bytes, count `+0x90`, the name
    /// pointer `+0x20`, not null) gets `name` copied in; a name outside
    /// the table's string pool (`+0x40`, 0xfa000 bytes) shorter than
    /// `name` is first replaced by a new one (`operator new[]`, the old
    /// one `delete[]`d). No result.
    pub fn vfx_update_texture(h: &mut impl Host, a: u32, b: u32, name: u32) {
        if a | b == 0 {
            return;
        }
        let table = VFX_RENDER_DATA_TABLE.get(h);
        let data = h.call_sym(VFX_DATA_BY_INSTANCE, Conv::Thiscall(table), &[a, b]);
        if data == 0 {
            return;
        }
        let mut k = 0i32;
        while k < h.read::<i32>(data + 0x90) {
            let slots: u32 = h.read(data + 0x94);
            let slot = slots.wrapping_add(0x6c * k as u32) + 0x20;
            let s: u32 = h.read(slot);
            if s != 0 {
                let pool: u32 = h.read(VFX_RENDER_DATA_TABLE.get(h) + 0x40);
                if s.wrapping_sub(pool) > 0xf_9fff && str_len(h, s) < str_len(h, name) {
                    h.call_sym(ARRAY_DELETE, Conv::Cdecl, &[s]);
                    let n = str_len(h, name).wrapping_add(1);
                    let fresh = h.call_sym(ARRAY_NEW, Conv::Cdecl, &[n]);
                    let slots: u32 = h.read(data + 0x94);
                    h.write(slots.wrapping_add(0x6c * k as u32) + 0x20, fresh);
                }
                let slots: u32 = h.read(data + 0x94);
                let dst: u32 = h.read(slots.wrapping_add(0x6c * k as u32) + 0x20);
                copy_str(h, dst, name);
            }
            k += 1;
        }
    }

    /// The scene's fog record (`+0xcc`).
    fn fog_record(h: &impl Host, scene: u32) -> u32 {
        h.read(scene + 0xcc)
    }

    /// `PhyreScene_setupMapFog_0066ee70(scene)`: the fog record `+0xcc`
    /// gets the map's fog (`+0x4f8`: colour `+0xd4..+0xdc` to
    /// `+0x10..+0x18`, near/far `+0xe4`/`+0xe8` to `+0x20`/`+0x24`, the
    /// density `+0xf0 / 255` to `+0x2c`, `+0xec` to `+0x28`), or the
    /// defaults without a map (`+0x28` and `+0x2c` 1.0); then the fog
    /// manager copies its parameters. `eax`: that copy's.
    pub fn scene_setup_map_fog(h: &mut impl Host, scene: u32) -> u32 {
        let map: u32 = h.read(scene + 0x4f8);
        if map == 0 {
            let dst = fog_record(h, scene);
            for (k, c) in (0u32..).zip(FOG_DEFAULT_COLOUR) {
                let v = round_trip(c.get(h));
                h.write(dst + 0x10 + 4 * k, v);
            }
            let v = round_trip(FOG_DEFAULT_NEAR.get(h));
            h.write(fog_record(h, scene) + 0x20, v);
            let v = round_trip(FOG_DEFAULT_FAR.get(h));
            h.write(fog_record(h, scene) + 0x24, v);
            h.write(fog_record(h, scene) + 0x2c, 1f32);
            h.write(fog_record(h, scene) + 0x28, 1f32);
        } else {
            let dst = fog_record(h, scene);
            for k in 0..3 {
                let v = round_trip(h.read(map + 0xd4 + 4 * k));
                h.write(dst + 0x10 + 4 * k, v);
            }
            for (from, to) in [(0xe4, 0x20), (0xe8, 0x24)] {
                let map: u32 = h.read(scene + 0x4f8);
                let v = round_trip(h.read(map + from));
                h.write(fog_record(h, scene) + to, v);
            }
            let map: u32 = h.read(scene + 0x4f8);
            let density = x87::div(x87::load(h.read(map + 0xf0)), FOG_DENSITY_DIVISOR.get(h));
            let v = round_trip(f32::from_bits(x87::store(density)));
            h.write(fog_record(h, scene) + 0x2c, v);
            let map: u32 = h.read(scene + 0x4f8);
            let v = round_trip(h.read(map + 0xec));
            h.write(fog_record(h, scene) + 0x28, v);
        }
        let manager = h.call_sym(FOG_INSTANCE, Conv::Cdecl, &[]);
        h.call_sym(FOG_COPY_PARAMETERS, Conv::Thiscall(manager), &[])
    }

    /// `ClassVFXRenderDataTable_Clear(this)` (frame: the key `{a, b}` at
    /// `ebp - 0x14` and the found node at `ebp - 8`, which `MapFind`
    /// sees; `this` at `ebp - 0xc` and `a` at `ebp - 4` kept as the
    /// original keeps them): after the message, each effect of the map
    /// (in order) with a key: its geometry (`g_VFXRenderDataTable`'s
    /// cached one for the same key, else found and cached) gets its
    /// vtable `+0x10` call, and the effect goes to the delete pool (kind
    /// 3); then the map is cleared and `this`'s cached key zeroed.
    pub fn vfx_table_clear(h: &mut impl Host, ebp: u32, this: u32) {
        let key = ebp.wrapping_sub(0x14);
        let out = ebp.wrapping_sub(8);
        h.write(ebp.wrapping_sub(0xc), this);
        let at = h.rebase(PRINTF_SLOT);
        let printf: u32 = h.read(at);
        let text = h.rebase(TABLE_CLEAR_MESSAGE);
        h.call(printf, Conv::Cdecl, &[text]);
        let head: u32 = h.read(this);
        let mut node: u32 = h.read(head);
        if node != head {
            loop {
                let a: u32 = h.read(node + 0x10);
                let b: u32 = h.read(node + 0x14);
                h.write(ebp.wrapping_sub(4), a);
                if a | b != 0 {
                    let table = VFX_RENDER_DATA_TABLE.get(h);
                    h.write(key, a);
                    h.write(key + 4, b);
                    let ca: u32 = h.read(table + 0x20);
                    let cb: u32 = h.read(table + 0x24);
                    let geometry = if ca | cb != 0 && ca == a && cb == b {
                        Some(h.read::<u32>(table + 0x28))
                    } else {
                        h.call_sym(MAP_FIND, Conv::Thiscall(table), &[out, key]);
                        let found: u32 = h.read(out);
                        if found == h.read::<u32>(table) {
                            None
                        } else {
                            h.write(table + 0x20, a);
                            h.write(table + 0x24, b);
                            let g: u32 = h.read(found + 0x18);
                            h.write(table + 0x28, g);
                            Some(h.read::<u32>(found + 0x18))
                        }
                    };
                    if let Some(g) = geometry.filter(|&g| g != 0) {
                        let vtable: u32 = h.read(g);
                        let method: u32 = h.read(vtable + 0x10);
                        h.call(method, Conv::Thiscall(g), &[]);
                    }
                    let value: u32 = h.read(node + 0x18);
                    h.call_sym(
                        INSERT_TO_DELETE_POOL,
                        Conv::Thiscall(this),
                        &[a, b, 3, value],
                    );
                }
                node = successor(h, node);
                if node == h.read::<u32>(this) {
                    break;
                }
            }
        }
        h.call_sym(MAP_CLEAR, Conv::Thiscall(this), &[]);
        h.write(this + 0x20, 0u32);
        h.write(this + 0x24, 0u32);
    }

    /// Calls the import whose slot is at `slot`; its `eax`.
    fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let at = h.rebase(slot);
        let f: u32 = h.read(at);
        h.call(f, Conv::Stdcall, args)
    }

    /// Whether the `n` bytes at `a` and `b` are equal (`memcmp` == 0).
    fn bytes_equal(h: &impl Host, a: u32, b: u32, n: u32) -> bool {
        (0..n).all(|k| h.read::<u8>(a.wrapping_add(k)) == h.read::<u8>(b.wrapping_add(k)))
    }

    /// `RenderDelegationHelper_unregisterDelegationOfGivenTaskAndData(this;
    /// task, data, size)` (`ret 0xc`), under the delegation mutex: each
    /// delegation (`+4`, count `+0` & 0x7fffffff, the walk stopping at a
    /// null one; the first is always looked at, the last never) for
    /// `task` (`+0x10`) whose data (`*d`) has `size` bytes equal to
    /// `data` goes back to the free list; when that fails (negative) the
    /// delegation and its buffer `+4` are deleted. The slot is cleared,
    /// the list compacted and the same index looked at again. `eax`:
    /// `ReleaseMutex`'s.
    pub fn delegation_unregister(
        h: &mut impl Host,
        this: u32,
        task: u32,
        data: u32,
        size: u32,
    ) -> u32 {
        let mutex = DELEGATION_MUTEX.get(h);
        import(h, WAIT_FOR_SINGLE_OBJECT, &[mutex, u32::MAX]);
        let mut i = 0u32;
        if count(h, this).wrapping_sub(1) != 0 {
            loop {
                let list: u32 = h.read(this + 4);
                let slot = list.wrapping_add(4 * i);
                let d: u32 = h.read(slot);
                if d == 0 {
                    break;
                }
                if h.read::<u32>(d + 0x10) == task {
                    let own: u32 = h.read(d);
                    if bytes_equal(h, own, data, size) {
                        let d: u32 = h.read(slot);
                        let r = h.call_sym(SET_DATA_TO_FREE_LIST, Conv::Thiscall(this), &[d]);
                        if (r as i32) < 0 {
                            let list: u32 = h.read(this + 4);
                            let e: u32 = h.read(list.wrapping_add(4 * i));
                            if e != 0 {
                                let buffer: u32 = h.read(e + 4);
                                if buffer != 0 {
                                    h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[buffer]);
                                }
                                h.write(e + 8, 0u32);
                                h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[e]);
                            }
                        }
                        let list: u32 = h.read(this + 4);
                        h.write(list.wrapping_add(4 * i), 0u32);
                        i = i.wrapping_sub(1);
                        h.call_sym(DELEGATION_COMPACT, Conv::Thiscall(this), &[]);
                    }
                }
                i = i.wrapping_add(1);
                if i >= count(h, this).wrapping_sub(1) {
                    break;
                }
            }
        }
        let mutex = DELEGATION_MUTEX.get(h);
        import(h, RELEASE_MUTEX, &[mutex])
    }

    /// The delegation count (`+0` & 0x7fffffff).
    fn count(h: &impl Host, this: u32) -> u32 {
        h.read::<u32>(this) & 0x7fff_ffff
    }
}
