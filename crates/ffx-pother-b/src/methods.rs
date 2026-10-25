//! Short methods of the lighting manager, the tweak utility, the fog
//! manager, the cloth meshes and the scene's search trees.
//!
//! Float copies go through the x87 (`fld`/`fstp`), so a signalling NaN is
//! stored quieted: [`quiet`].

use ffx_bridge::x87;
use ffx_graphic::host::{Conv, Host};

/// `Phyre_PhyreOS_Sleep` (cdecl `(ms)`).
pub const SLEEP: u32 = 0x0043_0030;
/// `TextUtil_resize` (thiscall `(text, w, h)`).
pub const TEXT_RESIZE: u32 = 0x006f_6670;
/// `Phyre_CharacterLighting_applyRotation` (thiscall `(cl, i)`).
pub const APPLY_ROTATION: u32 = 0x006a_7280;
/// `0x006e2000` (thiscall `(mesh, a, b, c)`, `ret 0xc`): the cloth mesh's
/// update the slot-5 method runs first.
pub const CLOTH_PREPARE: u32 = 0x006e_2000;
/// `operator_new_006304c0` (cdecl `(size)`).
pub const OPERATOR_NEW: u32 = 0x0063_04c0;
/// `crt_memset_thunk` (cdecl `(p, value, n)`).
pub const MEMSET: u32 = 0x0094_964c;

/// The mesh getters `0x006770f0` reads (thiscall, no stack words), in its
/// order: `+0x40`, `+0x48`, `+0x4c`, `+0x50`, `+0x8`, the vertex size.
pub const MESH_COUNT_GETTERS: [u32; 6] = [
    0x006d_3f90,
    0x006d_3f30,
    0x006d_3f80,
    0x006d_3f40,
    0x006d_3fa0,
    0x006d_3f50,
];
/// `Phyre_CharacterLighting_applyIntensityMultiplier` (thiscall `(cl, i,
/// flag)`).
pub const APPLY_INTENSITY: u32 = 0x006a_6e90;
/// `0x006e3480` (thiscall `(fog)` -> `al`): whether the fog is locked.
pub const FOG_LOCKED: u32 = 0x006e_3480;
/// `g_FogEffectManager`.
pub const G_FOG_EFFECT_MANAGER: u32 = 0x00ce_5170;
/// `operator new` `0x00630310` (cdecl `(size)`).
pub const OPERATOR_NEW_SCALAR: u32 = 0x0063_0310;
/// `g_PInputDeviceList`, the input devices' list head.
pub const G_INPUT_DEVICE_LIST: u32 = 0x00c3_0fa8;

/// A float word as `fld`/`fstp` stores it (a signalling NaN quieted).
#[must_use]
pub fn quiet(bits: u32) -> u32 {
    x87::store(x87::load(bits))
}

pub mod bridge {
    //! The originals over game memory.

    use super::{
        APPLY_ROTATION, CLOTH_PREPARE, Conv, Host, MEMSET, OPERATOR_NEW, SLEEP, TEXT_RESIZE, quiet,
    };

    /// `0x006e2df0(mesh)` (`ret 4`): sleeps 1 ms at a time while the word
    /// at `mesh + 8 + [mesh + 8]` is set (a worker's busy flag), then
    /// clears `mesh + 4`.
    pub fn wait_idle(h: &mut impl Host, mesh: u32) {
        let off: u32 = h.read(mesh + 8);
        let flag = off.wrapping_add(mesh).wrapping_add(8);
        while h.read::<u32>(flag) != 0 {
            let f = h.rebase(SLEEP);
            h.call(f, Conv::Cdecl, &[1]);
        }
        h.write(mesh + 4, 0u32);
    }

    /// `CharacterLightingTweakUtil::resize(tw, w, h)`: stores the size at
    /// `+0xe0`/`+0xe4`; when the text is up (`+0xc8`), tail-calls
    /// `TextUtil_resize(tw + 4, w, h)` and returns its `eax`, else `w`.
    pub fn tweak_resize(h: &mut impl Host, tw: u32, w: u32, hh: u32) -> u32 {
        h.write(tw + 0xe0, w);
        h.write(tw + 0xe4, hh);
        if h.read::<u8>(tw + 0xc8) == 0 {
            return w;
        }
        let f = h.rebase(TEXT_RESIZE);
        h.call(f, Conv::Thiscall(tw + 4), &[w, hh])
    }

    /// The cloth instancing mesh's slot 5 `(mesh, a, b, c, d)` (`0x006e2150`,
    /// `ret 0x10`): `CLOTH_PREPARE(mesh, a, b, c)`, then the inner mesh's
    /// (`+0x1c`) `vtable[5](a, b, c, d)`, or `b` when that gives 0.
    pub fn cloth_slot5(h: &mut impl Host, mesh: u32, a: u32, b: u32, c: u32, d: u32) -> u32 {
        let f = h.rebase(CLOTH_PREPARE);
        h.call(f, Conv::Thiscall(mesh), &[a, b, c]);
        let inner: u32 = h.read(mesh + 0x1c);
        let r = h.call_virtual(inner, 5, &[a, b, c, d]);
        if r == 0 { b } else { r }
    }

    /// `CharacterLighting::setRotation(cl, i, x, y)` (`ret 0xc`): for
    /// `i != 3` on an initialised manager (`+0x1cc`), stores the pair at
    /// `+0x1ac + 12 i` and, with `+0xf0` set, runs `applyRotation(i)`.
    pub fn set_rotation(h: &mut impl Host, cl: u32, i: u32, x: u32, y: u32) {
        if i == 3 || h.read::<u8>(cl + 0x1cc) == 0 {
            return;
        }
        let at = cl.wrapping_add(i.wrapping_mul(12));
        h.write(at.wrapping_add(0x1ac), quiet(x));
        h.write(at.wrapping_add(0x1b0), quiet(y));
        if h.read::<u32>(cl + 0xf0) != 0 {
            let f = h.rebase(APPLY_ROTATION);
            h.call(f, Conv::Thiscall(cl), &[i]);
        }
    }

    /// `0x006e3b80(fog)`: copies the fog parameters from its source
    /// `+0xa0` (`+0x30..0x38` to `+0..8`, `+0x10..0x18` to `+0x10..0x18`,
    /// `+0x28, 0x24, 0x20, 0x2c` to `+0x2c, 0x24, 0x20, 0x28`), in that
    /// order. Returns the source (`eax`).
    pub fn fog_copy(h: &mut impl Host, fog: u32) -> u32 {
        let src: u32 = h.read(fog + 0xa0);
        for (from, to) in [
            (0x30, 0),
            (0x34, 4),
            (0x38, 8),
            (0x10, 0x10),
            (0x14, 0x14),
            (0x18, 0x18),
            (0x28, 0x2c),
            (0x24, 0x24),
            (0x20, 0x20),
            (0x2c, 0x28),
        ] {
            let v: u32 = h.read(src + from);
            h.write(fog + to, quiet(v));
        }
        src
    }

    /// A search tree's `find(tree, &key)` (`ret 4`): from the root
    /// `[tree + 0xc]` down to the sentinel `tree`, each node's key `+0xc`
    /// against `key`; a greater node goes to `[node]`, a smaller to
    /// `[node + 4]`. Returns the matching node or 0. `signed` picks the
    /// comparison (`0x0064b9a0` signed, `0x006a5ec0` unsigned).
    pub fn tree_find(h: &mut impl Host, tree: u32, key_at: u32, signed: bool) -> u32 {
        let mut node: u32 = h.read(tree + 0xc);
        if node == tree {
            return 0;
        }
        let key: u32 = h.read(key_at);
        loop {
            let k: u32 = h.read(node + 0xc);
            let order = if signed {
                (k as i32).cmp(&(key as i32))
            } else {
                k.cmp(&key)
            };
            node = match order {
                std::cmp::Ordering::Equal => return node,
                std::cmp::Ordering::Greater => h.read(node),
                std::cmp::Ordering::Less => h.read(node + 4),
            };
            if node == tree {
                return 0;
            }
        }
    }

    /// `BrokenScreenPolygonDynamicMeshInstance` vtable slot 2 `(inst, out,
    /// x)` (`0x006770f0`, `ret 8`) -> 0: the mesh's (`[inst + 4] + 0x1c`)
    /// six counts from its getters into `out + 0x10..0x24`.
    pub fn broken_screen_preprocess(h: &mut impl Host, inst: u32, out: u32) -> u32 {
        let owner: u32 = h.read(inst + 4);
        let mesh: u32 = h.read(owner + 0x1c);
        for (k, getter) in super::MESH_COUNT_GETTERS.into_iter().enumerate() {
            let f = h.rebase(getter);
            let v = h.call(f, Conv::Thiscall(mesh), &[]);
            h.write(out + 0x10 + 4 * k as u32, v);
        }
        0
    }

    /// `PInput_GetDevice(kind, index)` (cdecl): the `index`-th device of
    /// `kind` (its `vtable[1]()`) in the device list `g_PInputDeviceList`
    /// (links at the device's `+4`), or 0.
    pub fn get_device(h: &mut impl Host, kind: u32, index: u32) -> u32 {
        let head = h.rebase(super::G_INPUT_DEVICE_LIST);
        let mut link: u32 = h.read(head);
        let mut seen = 0u32;
        while link != head && link != 0 && link != 4 {
            let device = link - 4;
            if h.call_virtual(device, 1, &[]) == kind {
                if seen == index {
                    return device;
                }
                seen = seen.wrapping_add(1);
            }
            link = h.read(device + 4);
        }
        0
    }

    /// `CharacterLightingTweakUtil::setShadeCoeff_dbgHook(tw, id, coeff,
    /// mode)` (`ret 0xc`): when the utility is on (`[tw]`), the first of
    /// the 100 12-byte entries at `[tw + 0xf8]` with `id` gets `mode` at
    /// `+8`, and for mode 2 the coefficient at `+4`.
    pub fn tweak_set_shade_coeff(h: &mut impl Host, tw: u32, id: u32, coeff: u32, mode: u32) {
        if h.read::<u8>(tw) == 0 {
            return;
        }
        let table: u32 = h.read(tw + 0xf8);
        let Some(i) = (0..100).find(|i| h.read::<u32>(table + 12 * i) == id) else {
            return;
        };
        h.write(table + 12 * i + 8, mode);
        if mode == 2 {
            let table: u32 = h.read(tw + 0xf8);
            h.write(table + 12 * i + 4, quiet(coeff));
        }
    }

    /// `0x006aa8c0(cl, i, value)` (`ret 8`): intensity `i` 3 is the global
    /// one at `+0x1a0`; the others, on an initialised manager (`+0x1cc`),
    /// go to `+0x1a8 + 12 i`. Then (for 3 always, else with `+0xf0` set)
    /// a tail call of `applyIntensityMultiplier(i, 1)`.
    pub fn set_intensity(h: &mut impl Host, cl: u32, i: u32, value: u32) {
        if i == 3 {
            h.write(cl + 0x1a0, quiet(value));
        } else {
            if h.read::<u8>(cl + 0x1cc) == 0 {
                return;
            }
            let at = cl.wrapping_add(i.wrapping_mul(12)).wrapping_add(0x1a8);
            h.write(at, quiet(value));
            if h.read::<u32>(cl + 0xf0) == 0 {
                return;
            }
        }
        let f = h.rebase(super::APPLY_INTENSITY);
        h.call(f, Conv::Thiscall(cl), &[i, 1]);
    }

    /// `0x006e3aa0(fog, limit)` (`ret 4`): unless the fog is locked
    /// (`FOG_LOCKED(fog)`, `al`), stores `limit` at the source's `+0x2c` and
    /// copies the parameters as [`fog_copy`].
    pub fn fog_set_limit(h: &mut impl Host, fog: u32, limit: u32) {
        let f = h.rebase(super::FOG_LOCKED);
        if h.call(f, Conv::Thiscall(fog), &[]) & 0xff != 0 {
            return;
        }
        let src: u32 = h.read(fog + 0xa0);
        h.write(src + 0x2c, quiet(limit));
        fog_copy(h, fog);
    }

    /// `0x006e3240()`: creates the `FogEffectManager` singleton when there
    /// is none: `new(0xb8)` with `+0xa0..0xa8 = 0`, `+0xac = 1.0`, `+0xb0 =
    /// 0.0`, `+0xb4 = 2`; the global gets it (or 0 when `new` fails).
    pub fn fog_create(h: &mut impl Host) {
        let g = h.rebase(super::G_FOG_EFFECT_MANAGER);
        if h.read::<u32>(g) != 0 {
            return;
        }
        let f = h.rebase(super::OPERATOR_NEW_SCALAR);
        let p = h.call(f, Conv::Cdecl, &[0xb8]);
        if p != 0 {
            h.write(p + 0xa0, 0u32);
            h.write(p + 0xac, 1f32);
            h.write(p + 0xa4, 0u32);
            h.write(p + 0xa8, 0u32);
            h.write(p + 0xb0, 0f32);
            h.write(p + 0xb4, 2u32);
        }
        h.write(g, p);
    }

    /// `0x006e2ac0(mesh, a, b, i)` (`ret 0xc`): stores `a` at `+0xc10 + 4
    /// i` and `b` at `+0x1810 + 4 i`; slot `+0x210 + 4 i` gets a zeroed
    /// `new[b]` when empty. Returns `a`, or `memset`'s result.
    pub fn cloth_slot_buffer(h: &mut impl Host, mesh: u32, a: u32, b: u32, i: u32) -> u32 {
        let base = mesh.wrapping_add(i.wrapping_mul(4));
        let empty = h.read::<u32>(base.wrapping_add(0x210)) == 0;
        h.write(base.wrapping_add(0xc10), a);
        h.write(base.wrapping_add(0x1810), b);
        if !empty {
            return a;
        }
        let f = h.rebase(OPERATOR_NEW);
        let p = h.call(f, Conv::Cdecl, &[b]);
        h.write(base.wrapping_add(0x210), p);
        let f = h.rebase(MEMSET);
        h.call(f, Conv::Cdecl, &[p, 0, b])
    }
}
