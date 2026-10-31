//! M5 stage 1, list `vfx`, round 5 batch 9: the VFX dynamic geometry's
//! material set-up (the vtable `0x00b47e58` override of
//! `ClassDynamicGeometry_setupMaterial`): one material per mesh segment,
//! the special shader parameters from the names' `GL/` part.
//!
//! The cursor and the shader name the original keeps in its frame come in
//! as an address (the shim passes a `StackFrame`).

/// `"GL/"`, the cursor's class check (`0x00ca5268`).
pub const GL_PREFIX: u32 = 0x00b4_7f78;
pub const CURSOR_CLASS: u32 = 0x00ca_5268;
/// `g_PhyreScene`.
pub const SCENE: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x00cc_b9d8);
/// `MSVCR110` slots: `strstr`, `strncpy` (cdecl).
pub const STRSTR_SLOT: u32 = 0x00b0_c378;
pub const STRNCPY_SLOT: u32 = 0x00b0_c368;
/// The frame: `ebp` minus the cursor (`ebp - 0xb4`); the name at `ebp -
/// 0x84`.
pub const FRAME: u32 = 0xb4;
pub const NAME_AT: u32 = 0xb4 - 0x84;

// ---- callees ----

/// `ClassDynamicGeometry_setupMaterial(g; n, names, clusters)` (`ret
/// 0xc`), `PhyreSceneCursor_Seek4(cursor)` (`ecx`),
/// `Phyre_PArray_resizeInline_0048c3b0(array; n)` (`ret 4`),
/// `user_malloc(n)`, `crt_memset_thunk(p, v, n)` (cdecl),
/// `PhyreScene_createMaterial_00659710(scene; variant, cluster, a, b)`
/// (`ret 0x10`), `VfxShaderManager_sGetInstance()` (cdecl),
/// `VfxShaderManager_setupSpecialShaderParameters(m; name, material,
/// base, params)` (`ret 0x10`).
pub const SETUP_MATERIAL: u32 = 0x006a_3b50;
pub const CURSOR_SEEK4: u32 = 0x0065_ecb0;
pub const RESIZE_INLINE: u32 = 0x0048_c3b0;
pub const USER_MALLOC: u32 = 0x0068_7020;
pub const MEMSET: u32 = 0x0094_964c;
pub const CREATE_MATERIAL: u32 = 0x0065_9710;
pub const SHADER_MANAGER: u32 = 0x006c_14c0;
pub const SETUP_SPECIAL: u32 = 0x006c_1520;

pub mod bridge {
    use super::{
        CREATE_MATERIAL, CURSOR_CLASS, CURSOR_SEEK4, GL_PREFIX, MEMSET, NAME_AT, RESIZE_INLINE,
        SCENE, SETUP_MATERIAL, SETUP_SPECIAL, SHADER_MANAGER, STRNCPY_SLOT, STRSTR_SLOT,
        USER_MALLOC,
    };
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Cdecl, args)
    }

    fn strlen(h: &impl Host, s: u32) -> u32 {
        let mut n = 0_u32;
        while h.read::<u8>(s.wrapping_add(n)) != 0 {
            n += 1;
        }
        n
    }

    /// `VFXDynamicGeometry_setupMaterial_006a4240(g; n, names, clusters)`
    /// (`ret 0xc`) -> 0, or 0x13 without the geometry's material, 3 past
    /// the material array, `resizeInline`'s error: without a material slot
    /// (`+0x3c` not -1) the plain `ClassDynamicGeometry_setupMaterial`;
    /// otherwise the cluster's first material (the cursor at `frame`, the
    /// original's `ebp - 0xb4`) goes to `+0x28`, `n` materials are made
    /// (the per-material words `+0x18`/`+0x1c`: inline for one, else
    /// `user_malloc`, zeroed) each with its special shader parameters from
    /// its name's part after `GL/` (copied to `frame + 0x30`; a name
    /// without it leaves the copy as it was, a never-written frame buffer
    /// the first time: junk read, zeroed here, brief 2026-10-24), stored
    /// in the material array (`+0x14`) and `+0x50` set.
    pub fn setup_material(
        h: &mut impl Host,
        frame: u32,
        g: u32,
        n: u32,
        names: u32,
        clusters: u32,
    ) -> u32 {
        if h.read::<u32>(g + 0x3c) != 0xffff_ffff {
            return h.call_sym(SETUP_MATERIAL, Conv::Thiscall(g), &[n, names, clusters]);
        }
        // junk read: [ebp-0x84] never written before a name with "GL/"; 0
        // (brief 2026-10-24)
        let list = h.read::<u32>(g + 0x20).wrapping_add(0x1c);
        let head: u32 = h.read(list);
        let first = if head == list { 0 } else { head };
        for k in 0..5 {
            h.write(frame + 4 * k, 0_u32);
        }
        h.write(frame + 0x14, list);
        h.write(frame + 0x18, first);
        h.write(frame + 0x1c, h.rebase(CURSOR_CLASS));
        h.call_sym(CURSOR_SEEK4, Conv::Thiscall(frame), &[]);
        if h.read::<u32>(frame + 0xc) == 0 {
            return 0x13;
        }
        let at = h.read::<u32>(frame + 0x10).wrapping_add(h.read(frame));
        let material: u32 = h.read(at);
        h.write(g + 0x28, material);
        let array: u32 = h.read(g + 0x14);
        let r = h.call_sym(RESIZE_INLINE, Conv::Thiscall(array), &[n]);
        if r != 0 {
            return r;
        }
        let bytes = n.wrapping_mul(4);
        if n.wrapping_mul(8) > 8 {
            let a = cdecl(h, USER_MALLOC, &[bytes]);
            h.write(g + 0x18, a);
            let b = cdecl(h, USER_MALLOC, &[bytes]);
            h.write(g + 0x1c, b);
        } else {
            h.write(g + 0x18, g + 0x31);
            h.write(g + 0x1c, g + 0x35);
        }
        let a: u32 = h.read(g + 0x18);
        cdecl(h, MEMSET, &[a, 0, bytes]);
        let b: u32 = h.read(g + 0x1c);
        cdecl(h, MEMSET, &[b, 0, bytes]);
        let name = frame + NAME_AT;
        let prefix = h.rebase(GL_PREFIX);
        let mut i = 0_u32;
        while i.cast_signed() < n.cast_signed() {
            let text: u32 = h.read(names.wrapping_add(4 * i));
            let s = import(h, STRSTR_SLOT, &[text, prefix]);
            if s != 0 {
                let len = strlen(h, s);
                import(h, STRNCPY_SLOT, &[name, s + 3, len.wrapping_sub(3)]);
                let len = strlen(h, s);
                h.write(name.wrapping_add(len).wrapping_sub(3), 0_u8);
            }
            let scene = SCENE.get(h);
            let (cluster, a, b): (u32, u32, u32) =
                (h.read(g + 0x2c), h.read(g + 0x18), h.read(g + 0x1c));
            let m2 = h.call_sym(
                CREATE_MATERIAL,
                Conv::Thiscall(scene),
                &[material, cluster, a + 4 * i, b + 4 * i],
            );
            let mesh: u32 = h.read(h.read::<u32>(g + 0x24) + 0x94);
            let params: u32 = h.read(mesh.wrapping_add(0x6c * i) + 0x24);
            let base: u32 = h.read(g + 0x28);
            let manager = cdecl(h, SHADER_MANAGER, &[]);
            h.call_sym(
                SETUP_SPECIAL,
                Conv::Thiscall(manager),
                &[name, m2, base, params],
            );
            let array: u32 = h.read(g + 0x14);
            let count = h.read::<u32>(array) & 0x7fff_ffff;
            if i >= count {
                return 3;
            }
            let data = match count {
                0 => 0,
                1 => array + 4,
                _ => h.read(array + 4),
            };
            h.write(data.wrapping_add(4 * i), m2);
            h.write(g + 0x50, 1_u8);
            i += 1;
        }
        0
    }
}
