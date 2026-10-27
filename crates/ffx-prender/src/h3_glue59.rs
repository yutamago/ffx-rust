//! The render-side glue, fifty-ninth batch (M4 port round 17, list
//! `h3-glue-render`): the scene's cameras and menus set up once its
//! textures are registered.

pub use crate::h3_glue51::UPDATE_VIEW_MATRICES;

/// `PTextureD3D11_register_0058cf80(scene + 0x940)` (thiscall) -> 0 or an
/// error.
pub const REGISTER: u32 = 0x0058_cf80;
/// `_CIatan` (`ST0` in and out) and the image's f64 it takes (0.40625).
pub const CI_ATAN: u32 = 0x0094_9d5c;
pub const ATAN_ARG: u32 = 0x00b4_3d48;
/// `PMatrix4_Copy_004c9430(out; m)` (`ret 4`), `Matrix_RotationY(out,
/// angle)` (cdecl) and `PMatrix4_MulThis_004c9da0(a; out, b)` (`ret 8`).
pub const COPY_4C9430: u32 = 0x004c_9430;
pub const ROTATION_Y: u32 = 0x0066_c920;
pub const MUL_THIS: u32 = 0x004c_9da0;
/// `Phyre_PTypeDispenser_GetClassDescriptor_Phyre_PMatrix4x3()` (cdecl),
/// `Phyre_PCluster_allocateInstanceList(cluster; n, class, a)` (`ret
/// 0xc`), `PMatrix4_toWorldMatrix3x4(out; m)` (`ret 4`) and
/// `Phyre_PCluster_create(cluster; n, class, a)` (`ret 0xc`).
pub const MATRIX4X3_CLASS: u32 = 0x0046_2aa0;
pub const ALLOCATE_INSTANCE_LIST: u32 = 0x0044_0b70;
pub const TO_WORLD_3X4: u32 = 0x0045_1190;
pub const CLUSTER_CREATE: u32 = 0x0044_1010;
/// The class record `Phyre_PCluster_create` takes (`0x00c92090`).
pub const CREATE_CLASS: u32 = 0x00c9_2090;
/// `PSamplerStates_init_006f5470()` (cdecl).
pub const SAMPLER_STATES_INIT: u32 = 0x006f_5470;
/// `g_Settings`, `CopyTable_00c8a538(settings)` and
/// `Settings_Save(settings)` (thiscall), `Virtuos_InputManager_getInstance()`
/// (cdecl) and `Input_BuildPadMap(manager)` (thiscall).
pub const SETTINGS: u32 = 0x022f_b504;
pub const COPY_TABLE: u32 = 0x0040_2b50;
pub const SETTINGS_SAVE: u32 = 0x0040_1d80;
pub const INPUT_MANAGER: u32 = 0x0063_1300;
pub const BUILD_PAD_MAP: u32 = 0x0063_15c0;
/// `IggyMenuManager_createInstance()` (cdecl), `g_IggyMenuManager` and
/// `FUN_00a262e0(manager; renderer, a, b)` (`ret 0xc`).
pub const IGGY_MENU_CREATE: u32 = 0x00a2_6240;
pub const IGGY_MENU_MANAGER: u32 = 0x0198_4c6c;
pub const IGGY_MENU_INIT: u32 = 0x00a2_62e0;
/// `Phyre_PFramework_PApplication_GetApplication()` (cdecl).
pub const GET_APPLICATION: u32 = 0x0062_3b90;
/// `fiosUnifyFilename(path, out, size)` (cdecl) and the two menu movies'
/// paths, loaded by `0x006f3310(path)` and `FUN_0068e400(path, a, b, c,
/// d)` (cdecl).
pub const FIOS_UNIFY: u32 = 0x0067_9820;
pub const PAUSE_MENU_PATH: u32 = 0x00b4_2578;
pub const ESC_MENU_PATH: u32 = 0x00b4_25a8;
pub const LOAD_PAUSE_MENU: u32 = 0x006f_3310;
pub const LOAD_ESC_MENU: u32 = 0x0068_e400;
/// `IconPanel_create_006f77f0()` and `FUN_006e8690()` (cdecl).
pub const ICON_PANEL_CREATE: u32 = 0x006f_77f0;
pub const AFTER_MENUS: u32 = 0x006e_8690;

pub mod bridge {
    //! The original over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    const ONE: u32 = 0x3f80_0000;
    /// 16/9.
    const WIDE: u32 = 0x3fe3_8e39;
    const FAR_65536: u32 = 0x4780_0000;
    const FAR_1337: u32 = 0x44a7_2431;
    const FAR_540: u32 = 0x4407_0000;

    /// The frame (`ebp` offsets).
    const ATAN_TMP: u32 = 0x108;
    const PATH: u32 = 0x104;
    const START: u32 = 0x148;
    const ROTATION: u32 = 0x188;
    const SAVED: u32 = 0x1c8;

    /// A camera's virtual `+4` (it takes the new projection).
    fn update(h: &mut impl Host, camera: u32) {
        h.call_virtual(camera, 1, &[]);
    }

    fn view_matrices(h: &mut impl Host, camera: u32) {
        h.call_sym(
            UPDATE_VIEW_MATRICES,
            Conv::Thiscall(camera.wrapping_add(4)),
            &[],
        );
    }

    /// `fld m32; fstp m32` of `from` into `to`.
    fn copy_f32(h: &mut impl Host, from: u32, to: u32) {
        let v: u32 = h.read(from);
        h.write(to, x87::store(x87::load(v)));
    }

    /// A perspective camera through the scene's pointer at `slot`: 16/9,
    /// near 1, far 65536, each followed by its virtual `+4`; then its view
    /// matrices.
    fn perspective(h: &mut impl Host, scene: u32, slot: u32) {
        for (off, v) in [(0x150, WIDE), (0xb8, ONE), (0xbc, FAR_65536)] {
            let camera: u32 = h.read(scene + slot);
            h.write(camera.wrapping_add(off), v);
            update(h, camera);
        }
        let camera: u32 = h.read(scene + slot);
        view_matrices(h, camera);
    }

    /// A new 3x4 matrix from the cluster, the 12 floats of `matrix`
    /// copied into it (the scene's word at `field` set after the first).
    fn new_matrix(h: &mut impl Host, scene: u32, matrix: u32, field: u32) -> u32 {
        let class = h.rebase(CREATE_CLASS);
        let out = h.call_sym(
            CLUSTER_CREATE,
            Conv::Thiscall(scene + 0x11e4),
            &[1, class, 0],
        );
        copy_f32(h, matrix, out);
        h.write(scene + field, out);
        for k in 1..12 {
            copy_f32(h, matrix.wrapping_add(4 * k), out.wrapping_add(4 * k));
        }
        out
    }

    /// `PhyreScene_initScene(scene)` (thiscall, plain `ret`; frame from
    /// `ebp - 0x1c8`, the security cookie left out) -> 0, or
    /// `PTextureD3D11_register_0058cf80`'s error. Clears `+0x10ddc`; the
    /// settings at `+0x940` (unless bit 7 of `+0x95c`: byte 5 at `+0x940`,
    /// 1 at `+0x941`, 4.0 at `+0x94c`) registered. Then the cameras: the
    /// three perspective ones (`+0xdc`, `+0x394`, `+0x238`), the embedded
    /// one at `+0xbdc` (2.37, 2 atan 0.40625, near 1, the main far), the
    /// shadow camera at `+0xa84` and the one at `+0x4fc` with a 3x4 matrix
    /// each from the cluster at `+0x11e4` (the world form of a rotation by
    /// pi about y of a start matrix), the light camera `+0x7e0` (that
    /// matrix, near 0.001, 1.142857, its `+0x154` 512 / that * 2.5 * 0.5)
    /// and `+0x654` (the main aspect, near 1, far 65536). Then the sampler
    /// states, the pad map unless the settings' byte `+0x638`, the Iggy
    /// menu manager and the pause, escape and icon menus.
    #[allow(clippy::too_many_lines)]
    pub fn init_scene(h: &mut impl Host, ebp: u32, scene: u32) -> u32 {
        h.write(scene + 0x10ddc, 0u32);
        let settings = scene + 0x940;
        let unset = (h.read::<u32>(settings + 0x1c) >> 7) & 1 == 0;
        if unset {
            h.write(settings, 5u8);
        }
        if unset {
            h.write(settings + 1, 1u8);
        }
        if (h.read::<u32>(settings + 0x1c) >> 7) & 1 == 0 {
            h.write(settings + 0xc, 0x4080_0000u32);
        }
        let error = h.call_sym(REGISTER, Conv::Thiscall(settings), &[]);
        if error != 0 {
            return error;
        }
        for slot in [0xdc, 0x394, 0x238] {
            perspective(h, scene, slot);
        }
        // the camera at +0xbdc
        let camera = scene + 0xbdc;
        h.write(scene + 0xd2c, 0x4017_b426u32);
        update(h, camera);
        let x = f64::from_bits(h.read::<u64>(h.rebase(ATAN_ARG)));
        let atan = h.rebase(CI_ATAN);
        let angle = h.call_intrinsic(atan, x).to_bits();
        h.write(ebp - ATAN_TMP, angle);
        let a = x87::load(angle);
        h.write(camera + 0x154, x87::store(x87::add(a, a)));
        update(h, camera);
        h.write(camera + 0xb8, ONE);
        update(h, camera);
        let main: u32 = h.read(scene + 0xdc);
        copy_f32(h, main.wrapping_add(0xbc), camera + 0xbc);
        update(h, camera);
        view_matrices(h, camera);
        // the shadow camera at +0xa84
        let shadow = scene + 0xa84;
        h.write(scene + 0xbd8, FAR_1337);
        h.write(shadow + 0x150, ONE);
        update(h, shadow);
        let matrix = world_matrix(h, ebp, scene);
        let first = new_matrix(h, scene, matrix, 0xb38);
        embedded_ortho_tail(h, scene, 0xa84);
        // the light camera +0x7e0
        let light: u32 = h.read(scene + 0x7e0);
        h.write(light.wrapping_add(0x154), FAR_1337);
        h.write(light.wrapping_add(0x150), ONE);
        update(h, light);
        let light: u32 = h.read(scene + 0x7e0);
        h.write(light.wrapping_add(0xb4), first);
        let light: u32 = h.read(scene + 0x7e0);
        h.write(light.wrapping_add(0xb8), 0x3a83_126fu32);
        update(h, light);
        let light: u32 = h.read(scene + 0x7e0);
        let main: u32 = h.read(scene + 0xdc);
        copy_f32(h, main.wrapping_add(0xbc), light.wrapping_add(0xbc));
        update(h, light);
        let light: u32 = h.read(scene + 0x7e0);
        h.write(light.wrapping_add(0x150), 0x3f92_4925u32);
        update(h, light);
        let light: u32 = h.read(scene + 0x7e0);
        let aspect = x87::load(h.read(light.wrapping_add(0x150)));
        let size = x87::mul(x87::mul(x87::div(512.0, aspect), 2.5), 0.5);
        h.write(light.wrapping_add(0x154), x87::store(size));
        update(h, light);
        let light: u32 = h.read(scene + 0x7e0);
        view_matrices(h, light);
        // the camera at +0x4fc
        let other = scene + 0x4fc;
        h.write(scene + 0x650, FAR_1337);
        h.write(other + 0x150, ONE);
        update(h, other);
        new_matrix(h, scene, matrix, 0x5b0);
        embedded_ortho_tail(h, scene, 0x4fc);
        // the camera +0x654
        let main: u32 = h.read(scene + 0xdc);
        let camera: u32 = h.read(scene + 0x654);
        copy_f32(h, main.wrapping_add(0x150), camera.wrapping_add(0x150));
        update(h, camera);
        for (off, v) in [(0xb8, ONE), (0xbc, FAR_65536)] {
            let camera: u32 = h.read(scene + 0x654);
            h.write(camera.wrapping_add(off), v);
            update(h, camera);
        }
        let camera: u32 = h.read(scene + 0x654);
        view_matrices(h, camera);
        menus(h, ebp, scene);
        0
    }

    /// The rest of an embedded camera's set-up after its matrix.
    fn embedded_ortho_tail(h: &mut impl Host, scene: u32, at: u32) {
        let camera = scene + at;
        h.write(camera + 0xb8, ONE);
        update(h, camera);
        let main: u32 = h.read(scene + 0xdc);
        copy_f32(h, main.wrapping_add(0xbc), camera + 0xbc);
        update(h, camera);
        h.write(camera + 0x150, WIDE);
        update(h, camera);
        h.write(camera + 0x154, FAR_540);
        update(h, camera);
        view_matrices(h, camera);
    }

    /// The start matrix (`ebp - 0x148`: 1 on the diagonal and at `[14]`),
    /// saved at `ebp - 0x1c8`, rotated by pi about y into `ebp - 0x188`;
    /// the cluster's new 3x4 instance (or 0) set from it. Its address.
    fn world_matrix(h: &mut impl Host, ebp: u32, scene: u32) -> u32 {
        for k in 0..16u32 {
            let one = matches!(k, 0 | 5 | 10 | 14 | 15);
            h.write(ebp - START + 4 * k, if one { ONE } else { 0 });
        }
        h.call_sym(COPY_4C9430, Conv::Thiscall(ebp - SAVED), &[ebp - START]);
        let m = h.call_sym(ROTATION_Y, Conv::Cdecl, &[ebp - ROTATION, 0x4049_0fda]);
        h.call_sym(COPY_4C9430, Conv::Thiscall(ebp - START), &[m]);
        h.call_sym(
            MUL_THIS,
            Conv::Thiscall(ebp - START),
            &[ebp - ROTATION, ebp - SAVED],
        );
        let class = h.call_sym(MATRIX4X3_CLASS, Conv::Cdecl, &[]);
        let list = h.call_sym(
            ALLOCATE_INSTANCE_LIST,
            Conv::Thiscall(scene + 0x11e4),
            &[1, class, 0],
        );
        let matrix = if list == 0 {
            0
        } else {
            h.read(list.wrapping_add(0x34))
        };
        if matrix != 0 {
            h.call_sym(TO_WORLD_3X4, Conv::Thiscall(matrix), &[ebp - ROTATION]);
        }
        matrix
    }

    /// The sampler states, the pad map, the Iggy menus.
    fn menus(h: &mut impl Host, ebp: u32, scene: u32) {
        h.call_sym(SAMPLER_STATES_INIT, Conv::Cdecl, &[]);
        let settings: u32 = h.read(h.rebase(SETTINGS));
        if h.read::<u8>(settings.wrapping_add(0x638)) == 0 {
            h.call_sym(COPY_TABLE, Conv::Thiscall(settings), &[]);
            let settings: u32 = h.read(h.rebase(SETTINGS));
            h.call_sym(SETTINGS_SAVE, Conv::Thiscall(settings), &[]);
            let input = h.call_sym(INPUT_MANAGER, Conv::Cdecl, &[]);
            h.call_sym(BUILD_PAD_MAP, Conv::Thiscall(input), &[]);
        }
        h.call_sym(IGGY_MENU_CREATE, Conv::Cdecl, &[]);
        let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        let b: u32 = h.read(app.wrapping_add(0x270));
        let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        let manager: u32 = h.read(h.rebase(IGGY_MENU_MANAGER));
        let a: u32 = h.read(app.wrapping_add(0x26c));
        let renderer: u32 = h.read(scene + 0xd0);
        h.call_sym(IGGY_MENU_INIT, Conv::Thiscall(manager), &[renderer, a, b]);
        let path = ebp - PATH;
        let pause = h.rebase(PAUSE_MENU_PATH);
        h.call_sym(FIOS_UNIFY, Conv::Cdecl, &[pause, path, 0x100]);
        h.call_sym(LOAD_PAUSE_MENU, Conv::Cdecl, &[path]);
        let esc = h.rebase(ESC_MENU_PATH);
        h.call_sym(FIOS_UNIFY, Conv::Cdecl, &[esc, path, 0x100]);
        h.call_sym(LOAD_ESC_MENU, Conv::Cdecl, &[path, 0, 0, ONE, ONE]);
        h.call_sym(ICON_PANEL_CREATE, Conv::Cdecl, &[]);
        h.call_sym(AFTER_MENUS, Conv::Cdecl, &[]);
    }
}
