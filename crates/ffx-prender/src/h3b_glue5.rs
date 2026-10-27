//! The render-side glue at 0x00690000 and up (M4, list `h3-glue-render`,
//! agent h3b): `ShadowManager_createResources_006d1b40`, the shadow
//! manager's meshes, materials, sampler states and initial state
//! (`ShadowManager_Initialize` tail-jumps to it).

pub use crate::h3b_glue3::{ARRAY_CLASS, CLUSTER_CREATE, CREATE_MATERIAL, FREE, REGISTER};

/// The full-screen triangle and the split quad (cdecl).
pub const MESH_TRIANGLE: u32 = 0x00a0_6120;
pub const MESH_SPLIT: u32 = 0x00a0_6e60;
/// `ShadowDynamicMesh_create_006c9580(this; out)` (`ret 4`).
pub const SHADOW_MESH: u32 = 0x006c_9580;
/// `PhyreSceneCursor_Seek3(cursor)` and the class it looks for.
pub const CURSOR_SEEK: u32 = 0x0065_ec40;
pub const CURSOR_CLASS: u32 = 0x00ca_5ff0;
/// The split's u (a global) and v (f32 0.95).
pub const SPLIT_U: u32 = 0x00cd_ec34;
pub const SPLIT_V: u32 = 0x00b4_9954;
/// The classes of `+0x84` (stored 4 below) and `+0x88`/`+0x8c`.
pub const CLASS_84: u32 = 0x00c9_30e8;
pub const CLASS_88: u32 = 0x00c9_2090;
/// f32 constants for `+0x4018`, `+0x4020`, `+0x4024`, `+0x4028`.
pub const C_4018: u32 = 0x00b4_997c;
pub const C_4020: u32 = 0x00b4_9998;
pub const C_4024: u32 = 0x00b4_999c;
pub const C_4028: u32 = 0x00b3_eb44;

/// The frame: the scene cursor at `ebp - 0x28` (seven words), the
/// material at `- 8`, a pointer at `- 4`.
pub const CURSOR: u32 = 0x28;
pub const MATERIAL: u32 = 8;
pub const WALK: u32 = 4;

/// The identity matrices' offsets from `this + 0x90 + 0x40 i` (i = 0, 1).
pub const MATRICES: [u32; 8] = [0, 0x80, 0x100, 0x180, 0x220, 0x2a0, 0x320, 0x3a0];

pub mod bridge {
    //! The original over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    const ONE: u32 = 0x3f80_0000;

    fn copy_bits(h: &impl Host, at: u32) -> u32 {
        x87::store(x87::load(h.read(h.rebase(at))))
    }

    /// A sampler state at `s`: the five bytes `values`, each unless flag
    /// 0x80 of `+0x1c` is set, then registered.
    fn sampler(h: &mut impl Host, s: u32, values: [u8; 5]) {
        for (k, v) in (0..5).zip(values) {
            if h.read::<u32>(s + 0x1c) >> 7 & 1 == 0 {
                h.write(s + k, v);
            }
        }
        h.call_sym(REGISTER, Conv::Thiscall(s), &[]);
    }

    /// An array `{head, data}`: emptied unless its storage is borrowed
    /// (top bit), keeping the top bit.
    fn release(h: &mut impl Host, head_at: u32) {
        let head = h.read::<u32>(head_at) & 0x8000_0000;
        h.write(head_at, head);
        if head == 0 {
            let data: u32 = h.read(head_at + 4);
            h.call_sym(FREE, Conv::Cdecl, &[data]);
            h.write(head_at + 4, 0u32);
        }
    }

    /// `ShadowManager_createResources_006d1b40` (`ecx` the manager, no
    /// stack words; frame at `ebp`, see the module's offsets): 0x15
    /// without the triangle mesh (`+0x8b8`), else 0.
    #[allow(clippy::too_many_lines)] // the original's sequence
    pub fn create_resources(h: &mut impl Host, ebp: u32, this: u32) -> u32 {
        let cluster = this + 4;
        let mesh = h.call_sym(MESH_TRIANGLE, Conv::Cdecl, &[cluster]);
        h.write(this + 0x8b8, mesh);
        if mesh == 0 {
            return 0x15;
        }
        h.write(ebp - WALK, 0x10u32);
        for k in 0..16 {
            let slot = this + 0xcc4 + 4 * k;
            let m = h.call_sym(SHADOW_MESH, Conv::Thiscall(this), &[slot + 0x80]);
            h.write(slot, m);
            let m = h.call_sym(SHADOW_MESH, Conv::Thiscall(this), &[slot + 0xc0]);
            h.write(ebp - WALK, 15 - k);
            h.write(slot + 0x40, m);
            h.write(this + 0xdc4 + k, 0u8);
            h.write(this + 0xdd4 + k, 0u8);
        }
        let u = copy_bits(h, SPLIT_U);
        let v = copy_bits(h, SPLIT_V);
        let split = h.call_sym(MESH_SPLIT, Conv::Cdecl, &[cluster, u, v, 0, 0, ONE]);
        h.write(this + 0xcbc, split);

        // the scene cursor over the cluster's objects of the class
        let list = h.read::<u32>(this).wrapping_add(0x1c);
        let first: u32 = h.read(list);
        let cursor = ebp - CURSOR;
        for k in 0..5 {
            h.write(cursor + 4 * k, 0u32);
        }
        h.write(cursor + 0x14, list);
        h.write(cursor + 0x18, if first == list { 0 } else { first });
        h.write(cursor + 0x1c, h.rebase(CURSOR_CLASS));
        h.call_sym(CURSOR_SEEK, Conv::Thiscall(cursor), &[]);
        let variant = if h.read::<u32>(cursor + 0xc) == 0 {
            0
        } else {
            let base: u32 = h.read(cursor);
            base.wrapping_add(h.read(cursor + 0x10))
        };
        let material = h.call_sym(CREATE_MATERIAL, Conv::Thiscall(variant), &[cluster, 0, 0]);
        h.write(ebp - MATERIAL, material);
        let class = h.rebase(ARRAY_CLASS);
        let array = h.call_sym(CLUSTER_CREATE, Conv::Thiscall(cluster), &[1, class, 0]);
        h.write(this + 0xde4, array);
        let head: u32 = h.read(array);
        if head != 1 {
            let data = match head & 0x7fff_ffff {
                0 => 0,
                1 => array + 4,
                _ => h.read(array + 4),
            };
            let slot = array.wrapping_add(4);
            if slot != 0 {
                h.write(slot, 0u32);
                let head: u32 = h.read(array);
                if data != slot && head >> 31 == 0 {
                    h.call_sym(FREE, Conv::Cdecl, &[data]);
                }
                h.write(array, 1u32);
            }
        }
        let array: u32 = h.read(this + 0xde4);
        let n = h.read::<u32>(array) & 0x7fff_ffff;
        if n != 0 {
            let data = if n > 1 { h.read(array + 4) } else { array + 4 };
            h.write(data, material);
        }
        for at in [0x8b8, 0xcbc] {
            let m: u32 = h.read(this + at);
            let mut materials: u32 = h.read(this + 0xde4);
            if materials == 0 {
                materials = h.read::<u32>(m).wrapping_add(0x30);
            }
            h.write(m + 0x10, materials);
        }

        sampler(h, this + 0x3fc4, [1, 1, 2, 2, 2]);
        sampler(h, this + 0x3fe8, [0, 0, 2, 2, 2]);
        for (at, n) in [
            (0xde8, 0x100),
            (0xee8, 0x100),
            (0x2ae8, 0x40),
            (0x29e8, 0x100),
        ] {
            for k in (0..n).step_by(4) {
                h.write(this + at + k, 0u32);
            }
        }
        let class = h.rebase(CLASS_84);
        let c = h.call_sym(CLUSTER_CREATE, Conv::Thiscall(cluster), &[1, class, 0]);
        h.write(this + 0x84, if c == 0 { 0 } else { c - 4 });
        let class = h.rebase(CLASS_88);
        for at in [0x88, 0x8c] {
            let c = h.call_sym(CLUSTER_CREATE, Conv::Thiscall(cluster), &[1, class, 0]);
            h.write(this + at, c);
        }
        // the 8 x 8 cascade tables
        for k in 0..64 {
            let at = this + 0xfe8 + 4 * k;
            h.write(at, u32::MAX);
            h.write(at + 0x300, 1u32);
            h.write(at + 0x200, 1u32);
            h.write(at + 0x100, 0u32);
        }
        h.write(this + 0x3fc0, 0u32);
        h.write(this + 0x3fbc, 1u8);
        h.write(this + 0x400c, 1u8);
        h.write(this + 0x4010, 0u32);
        h.write(this + 0x4014, 0u32);
        h.write(ebp - WALK, this + 0x3fb0);
        for i in 0..2 {
            let base = this + 0x90 + 0x40 * i;
            for m in MATRICES {
                for k in 0..16 {
                    let v = if k % 5 == 0 { ONE } else { 0 };
                    h.write(base + m + 4 * k, v);
                }
            }
            release(h, this + 0x290 + 8 * i);
            release(h, this + 0x2a0 + 8 * i);
            h.write(this + 0x37a8 + 4 * i, 0u32);
            h.write(this + 0x3fb0 + 4 * i, 0u32);
            h.write(ebp - MATERIAL, 1 - i);
            h.write(ebp - WALK, this + 0x3fb4 + 4 * i);
        }
        let c = copy_bits(h, C_4018);
        h.write(this + 0x4018, c);
        h.write(this + 0x401c, 0u32);
        for (at, from) in [(0x4020, C_4020), (0x4024, C_4024), (0x4028, C_4028)] {
            let c = copy_bits(h, from);
            h.write(this + at, c);
        }
        h.write(this + 0x402c, 0u32);
        h.write(this + 0x4030, 0u8);
        h.write(this + 0x4044, 1u8);
        h.write(this + 0x404c, 0u8);
        0
    }
}
