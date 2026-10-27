//! The render-side glue, fifty-third batch (M4 port round 15, list
//! `h3-glue-render`): the scene's ability-map (sphere grid) pass. The
//! exception frame is left out, as in [`crate::glue10`].

pub use crate::h3_glue51::{
    BEGIN_SCENE, COPY_BUFFER, DEGRADE_RESOLUTION, END_SCENE, RENDER_MESH_INSTANCE, SET_CAMERA,
    SET_COLOR_TARGET, SET_DEPTH_STENCIL_TARGET, SET_VIEWPORT, SURFACE_CTOR, SURFACE_DTOR,
    UPDATE_VIEW_MATRICES,
};

/// `g_MovieFading` (a word the scene's fade byte `+0x10dcc` stands
/// beside).
pub const MOVIE_FADING: u32 = 0x0134_0824;
/// The image's f32 the camera's `+0x150` takes for the pass
/// (0x3fe38e39, 16/9) and the f64 whose arctangent its `+0x154` takes.
pub const ASPECT: u32 = 0x00b4_1cf0;
pub const ATAN_ARG: u32 = 0x00b4_3d58;
/// `_CIatan` (`ST0` in and out).
pub const CI_ATAN: u32 = 0x0094_9d5c;
/// `PMatrix4_Copy_00453340(out; m)` (`ret 4`).
pub const MATRIX_COPY: u32 = 0x0045_3340;
/// `g_RenderDelegationHelper` and its `beforeScene`, `beforeVFX`,
/// `beforeEffects` and `runFlag800` (thiscall, no arguments).
pub const DELEGATION_HELPER: u32 = 0x00cc_ca98;
pub const BEFORE_SCENE: u32 = 0x006a_da20;
pub const BEFORE_VFX: u32 = 0x006a_db80;
pub const BEFORE_EFFECTS: u32 = 0x006a_d6b0;
pub const RUN_FLAG_800: u32 = 0x006a_d380;
/// `setClearStencil(renderer; stencil)` (`ret 4`) and
/// `clearScene(renderer; flags)` (`ret 4`).
pub const SET_CLEAR_STENCIL: u32 = 0x005b_afc0;
pub const CLEAR_SCENE: u32 = 0x005b_6330;
/// `g_GraphicAbmap` and `AbmapManager_GetMeshInstace(manager; i)` (`ret
/// 4`).
pub const ABMAP: u32 = 0x00cc_c838;
pub const ABMAP_MESH_INSTANCE: u32 = 0x0068_4ca0;
/// The pass name pointers the renderer's `+0x48` takes.
pub const OPAQUE_NAME: u32 = 0x00c1_69b4;
pub const TRANSPARENT_NAME: u32 = 0x00c1_69d4;
/// `JobScheduleWait_pppPart_fillBuffer()` (cdecl).
pub const PPP_FILL_WAIT: u32 = 0x0071_e810;
/// `PhyreScene_MergeVFXRenderArrays(scene)` and
/// `PhyreScene_clearArrays_00659000(scene)` (thiscall).
pub const MERGE_VFX_ARRAYS: u32 = 0x0065_4650;
pub const CLEAR_ARRAYS: u32 = 0x0065_9000;
/// `MeshInstance_isVisible_00663cc0(mesh; matrix)` (`ret 4`, `al`) and
/// `renderMeshInstanceForPass(renderer; instance)` (`ret 4`).
pub const IS_VISIBLE: u32 = 0x0066_3cc0;
pub const RENDER_FOR_PASS: u32 = 0x005b_9a70;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// The surfaces of the post targets `+0x8c` and `+0x90`.
    const COLOR: u32 = 0x28;
    const DEPTH: u32 = 0x38;

    /// A texture slot's texture: none when its byte `+4` is set.
    fn texture(h: &impl Host, slot: u32) -> u32 {
        if h.read::<u8>(slot.wrapping_add(4)) == 0 {
            h.read(slot)
        } else {
            0
        }
    }

    fn renderer(h: &impl Host, scene: u32) -> u32 {
        h.read(scene + 0xd0)
    }

    /// The camera's virtual `+4` (it takes the new projection).
    fn update_camera(h: &mut impl Host, scene: u32) {
        let camera: u32 = h.read(scene + 0xdc);
        h.call_virtual(camera, 1, &[]);
    }

    /// One of the render delegation helper's passes.
    fn delegations(h: &mut impl Host, pass: u32) {
        let helper: u32 = h.read(h.rebase(DELEGATION_HELPER));
        h.call_sym(pass, Conv::Thiscall(helper), &[]);
    }

    /// The renderer's pass name (`+0x48`).
    fn set_pass(h: &mut impl Host, scene: u32, name: u32) {
        let renderer = renderer(h, scene);
        let name = h.rebase(name);
        h.write(renderer.wrapping_add(0x48), name);
    }

    /// The colour and depth surfaces set as targets: the first error.
    fn set_targets(h: &mut impl Host, ebp: u32, scene: u32) -> u32 {
        let r = renderer(h, scene);
        let error = h.call_sym(SET_COLOR_TARGET, Conv::Thiscall(r), &[ebp - COLOR, 0]);
        if error != 0 {
            return error;
        }
        let r = renderer(h, scene);
        h.call_sym(SET_DEPTH_STENCIL_TARGET, Conv::Thiscall(r), &[ebp - DEPTH])
    }

    /// The viewport set to the colour target's size, the resolution
    /// degraded.
    fn viewport(h: &mut impl Host, ebp: u32, scene: u32) {
        let slot: u32 = h.read(ebp - COLOR);
        let texture = texture(h, slot);
        let height: u32 = h.read(texture.wrapping_add(0x20));
        let width: u32 = h.read(texture.wrapping_add(0x1c));
        let r = renderer(h, scene);
        h.call_sym(SET_VIEWPORT, Conv::Thiscall(r), &[0, 0, width, height]);
        let r = renderer(h, scene);
        let post: u32 = h.read(scene + 0x10cc);
        h.call_sym(DEGRADE_RESOLUTION, Conv::Thiscall(post), &[r]);
    }

    /// The renderer's camera: `scene + at + 4`, or 0 where `scene + at`
    /// is 0.
    fn set_camera(h: &mut impl Host, scene: u32, at: u32) {
        let camera = if scene.wrapping_add(at) == 0 {
            0
        } else {
            scene.wrapping_add(at + 4)
        };
        let r = renderer(h, scene);
        h.call_sym(SET_CAMERA, Conv::Thiscall(r), &[camera]);
    }

    /// The ability map's mesh instance `i`.
    fn abmap_instance(h: &mut impl Host, i: u32) -> u32 {
        let abmap: u32 = h.read(h.rebase(ABMAP));
        h.call_sym(ABMAP_MESH_INSTANCE, Conv::Thiscall(abmap), &[i])
    }

    /// Whether the mesh instance (its mesh `+0x1c`) is in the camera's
    /// view (`+0x74`).
    fn visible(h: &mut impl Host, scene: u32, instance: u32) -> bool {
        let camera: u32 = h.read(scene + 0xdc);
        let mesh: u32 = h.read(instance.wrapping_add(0x1c));
        let seen = h.call_sym(
            IS_VISIBLE,
            Conv::Thiscall(mesh),
            &[camera.wrapping_add(0x74)],
        );
        seen & 0xff != 0
    }

    fn render_for_pass(h: &mut impl Host, scene: u32, instance: u32) {
        let r = renderer(h, scene);
        h.call_sym(RENDER_FOR_PASS, Conv::Thiscall(r), &[instance]);
    }

    /// `PhyreScene_RenderABmap(scene)` (thiscall, plain `ret`; frame from
    /// `ebp - 0x38`) -> 0 or the first failing renderer call's code.
    /// Nothing (0) unless the scene's byte `+0x10dcc` or `g_MovieFading`
    /// is set. The camera's `+0x150` and `+0x154` saved through the x87
    /// (`ebp - 0x14`, `- 0x18`) and set to 16/9 and `atan` of the image's
    /// f64 (`ebp - 0x10`), each followed by the camera's virtual `+4`; its
    /// view matrices updated, its matrix (`+0x34`) copied into `[scene +
    /// 0xcc] + 0x40`, the delegations' `beforeScene`. The post targets
    /// `+0x8c` and `+0x90` go into surfaces at `ebp - 0x28` and `- 0x38`.
    /// Then: the ability map's instances 0-9 drawn opaque and transparent
    /// (stencil cleared, flags 0xb, camera `+0xbe0`); `beforeVFX`; the
    /// effect particles waited for and the VFX arrays merged; the scene's
    /// VFX instances (`+0x38cc`, count `+0xf8cc`) drawn where visible
    /// (camera `[+0xdc] + 4`); the arrays cleared; with sphere-grid
    /// instances (`+0x385c`, count `+0x3854`) the colour target copied to
    /// `+0xa8`, `beforeEffects`, and those drawn where visible; the camera
    /// put back, the colour target copied to the back buffer, and the
    /// instances 10-14 drawn on it (camera `+0xa88`, cleared with 10),
    /// then `runFlag800`. The surfaces are destroyed on every way out.
    pub fn render_abmap(h: &mut impl Host, ebp: u32, scene: u32) -> u32 {
        let fading: u32 = h.read(h.rebase(MOVIE_FADING));
        if h.read::<u8>(scene + 0x10dcc) == 0 && fading == 0 {
            return 0;
        }
        let camera: u32 = h.read(scene + 0xdc);
        let aspect: u32 = h.read(camera.wrapping_add(0x150));
        h.write(ebp - 0x14, x87::store(x87::load(aspect)));
        let fov: u32 = h.read(camera.wrapping_add(0x154));
        h.write(ebp - 0x18, x87::store(x87::load(fov)));
        let wide: u32 = h.read(h.rebase(ASPECT));
        h.write(camera.wrapping_add(0x150), x87::store(x87::load(wide)));
        update_camera(h, scene);
        let x = f64::from_bits(h.read::<u64>(h.rebase(ATAN_ARG)));
        let atan = h.rebase(CI_ATAN);
        let angle = h.call_intrinsic(atan, x).to_bits();
        h.write(ebp - 0x10, angle);
        let camera: u32 = h.read(scene + 0xdc);
        h.write(camera.wrapping_add(0x154), x87::store(x87::load(angle)));
        update_camera(h, scene);
        let camera: u32 = h.read(scene + 0xdc);
        h.call_sym(
            UPDATE_VIEW_MATRICES,
            Conv::Thiscall(camera.wrapping_add(4)),
            &[],
        );
        let camera: u32 = h.read(scene + 0xdc);
        let out: u32 = h.read(scene + 0xcc);
        h.call_sym(
            MATRIX_COPY,
            Conv::Thiscall(out.wrapping_add(0x40)),
            &[camera.wrapping_add(0x34)],
        );
        delegations(h, BEFORE_SCENE);
        let post: u32 = h.read(scene + 0x10cc);
        let depth: u32 = h.read(post.wrapping_add(0x90));
        let color: u32 = h.read(post.wrapping_add(0x8c));
        h.call_sym(SURFACE_CTOR, Conv::Thiscall(ebp - COLOR), &[color, 0]);
        h.call_sym(SURFACE_CTOR, Conv::Thiscall(ebp - DEPTH), &[depth, 0]);
        let result = passes(h, ebp, scene);
        h.call_sym(SURFACE_DTOR, Conv::Thiscall(ebp - DEPTH), &[]);
        h.call_sym(SURFACE_DTOR, Conv::Thiscall(ebp - COLOR), &[]);
        result
    }

    /// The passes between the surfaces' construction and destruction.
    fn passes(h: &mut impl Host, ebp: u32, scene: u32) -> u32 {
        let error = abmap_pass(h, ebp, scene);
        if error != 0 {
            return error;
        }
        let error = vfx_pass(h, ebp, scene);
        if error != 0 {
            return error;
        }
        let error = grid_pass(h, ebp, scene);
        if error != 0 {
            return error;
        }
        // the camera put back
        let camera: u32 = h.read(scene + 0xdc);
        let aspect: u32 = h.read(ebp - 0x14);
        h.write(camera.wrapping_add(0x150), x87::store(x87::load(aspect)));
        update_camera(h, scene);
        let fov: u32 = h.read(ebp - 0x18);
        let camera: u32 = h.read(scene + 0xdc);
        h.write(camera.wrapping_add(0x154), x87::store(x87::load(fov)));
        update_camera(h, scene);
        // the colour target onto the back buffer, instances 10-14 on top
        let post: u32 = h.read(scene + 0x10cc);
        let slot: u32 = h.read(post.wrapping_add(0x8c));
        let source = texture(h, slot);
        let r = renderer(h, scene);
        h.call_sym(COPY_BUFFER, Conv::Thiscall(post), &[r, source, 0, 1, 0, 0]);
        let r = renderer(h, scene);
        h.call_sym(SET_COLOR_TARGET, Conv::Thiscall(r), &[0, 0]);
        let r = renderer(h, scene);
        h.call_sym(SET_DEPTH_STENCIL_TARGET, Conv::Thiscall(r), &[0]);
        let r = renderer(h, scene);
        let error = h.call_sym(BEGIN_SCENE, Conv::Thiscall(r), &[0]);
        if error != 0 {
            return error;
        }
        set_camera(h, scene, 0xa84);
        h.call_sym(
            UPDATE_VIEW_MATRICES,
            Conv::Thiscall(scene.wrapping_add(0xa88)),
            &[],
        );
        for i in 10..15 {
            let instance = abmap_instance(h, i);
            if instance != 0 {
                set_pass(h, scene, OPAQUE_NAME);
                render_for_pass(h, scene, instance);
                set_pass(h, scene, TRANSPARENT_NAME);
                render_for_pass(h, scene, instance);
            }
        }
        let r = renderer(h, scene);
        h.call_sym(CLEAR_SCENE, Conv::Thiscall(r), &[10]);
        let r = renderer(h, scene);
        let error = h.call_sym(END_SCENE, Conv::Thiscall(r), &[1]);
        if error != 0 {
            return error;
        }
        delegations(h, RUN_FLAG_800);
        0
    }

    /// The ability map's instances 0-9 into the surfaces; `beforeVFX`, the
    /// particles waited for, the VFX arrays merged.
    fn abmap_pass(h: &mut impl Host, ebp: u32, scene: u32) -> u32 {
        let error = set_targets(h, ebp, scene);
        if error != 0 {
            return error;
        }
        let r = renderer(h, scene);
        h.call_sym(SET_CLEAR_STENCIL, Conv::Thiscall(r), &[0]);
        viewport(h, ebp, scene);
        let r = renderer(h, scene);
        let error = h.call_sym(BEGIN_SCENE, Conv::Thiscall(r), &[0xb]);
        if error != 0 {
            return error;
        }
        set_camera(h, scene, 0xbdc);
        for i in 0..10 {
            let instance = abmap_instance(h, i);
            if instance != 0 {
                for name in [OPAQUE_NAME, TRANSPARENT_NAME] {
                    set_pass(h, scene, name);
                    let r = renderer(h, scene);
                    let context: u32 = h.read(r.wrapping_add(0x13c));
                    let mesh: u32 = h.read(instance.wrapping_add(0x10));
                    h.call_sym(
                        RENDER_MESH_INSTANCE,
                        Conv::Thiscall(r),
                        &[instance, mesh, 0, context],
                    );
                }
            }
        }
        let r = renderer(h, scene);
        let error = h.call_sym(END_SCENE, Conv::Thiscall(r), &[1]);
        if error != 0 {
            return error;
        }
        delegations(h, BEFORE_VFX);
        viewport(h, ebp, scene);
        h.call_sym(PPP_FILL_WAIT, Conv::Cdecl, &[]);
        h.call_sym(MERGE_VFX_ARRAYS, Conv::Thiscall(scene), &[]);
        0
    }

    /// The scene's VFX mesh instances (`+0x38cc`, count `+0xf8cc`,
    /// signed) where visible; the arrays cleared; with sphere-grid
    /// instances the colour target copied to the post target `+0xa8`.
    fn vfx_pass(h: &mut impl Host, ebp: u32, scene: u32) -> u32 {
        let error = set_targets(h, ebp, scene);
        if error != 0 {
            return error;
        }
        let r = renderer(h, scene);
        let error = h.call_sym(BEGIN_SCENE, Conv::Thiscall(r), &[0]);
        if error != 0 {
            return error;
        }
        let camera: u32 = h.read(scene + 0xdc);
        let camera = if camera == 0 {
            0
        } else {
            camera.wrapping_add(4)
        };
        let r = renderer(h, scene);
        h.call_sym(SET_CAMERA, Conv::Thiscall(r), &[camera]);
        set_pass(h, scene, TRANSPARENT_NAME);
        let mut k = 0u32;
        while k.cast_signed() < h.read::<i32>(scene + 0xf8cc) {
            let slot = scene.wrapping_add(0x38cc).wrapping_add(4 * k);
            let instance: u32 = h.read(slot);
            if instance != 0 && visible(h, scene, instance) {
                let instance: u32 = h.read(slot);
                render_for_pass(h, scene, instance);
            }
            k += 1;
        }
        let r = renderer(h, scene);
        let error = h.call_sym(END_SCENE, Conv::Thiscall(r), &[1]);
        if error != 0 {
            return error;
        }
        h.call_sym(CLEAR_ARRAYS, Conv::Thiscall(scene), &[]);
        if h.read::<u32>(scene + 0x3854) != 0 {
            let post: u32 = h.read(scene + 0x10cc);
            let slot: u32 = h.read(post.wrapping_add(0x8c));
            let source = texture(h, slot);
            let to: u32 = h.read(post.wrapping_add(0xa8));
            let r = renderer(h, scene);
            h.call_sym(COPY_BUFFER, Conv::Thiscall(post), &[r, source, to, 1, 0, 0]);
        }
        0
    }

    /// `beforeEffects`, then the sphere-grid instances (`+0x385c`, count
    /// `+0x3854`) where visible.
    fn grid_pass(h: &mut impl Host, ebp: u32, scene: u32) -> u32 {
        let error = set_targets(h, ebp, scene);
        if error != 0 {
            return error;
        }
        delegations(h, BEFORE_EFFECTS);
        if h.read::<u32>(scene + 0x3854) == 0 {
            return 0;
        }
        let error = set_targets(h, ebp, scene);
        if error != 0 {
            return error;
        }
        viewport(h, ebp, scene);
        let r = renderer(h, scene);
        let error = h.call_sym(BEGIN_SCENE, Conv::Thiscall(r), &[0]);
        if error != 0 {
            return error;
        }
        set_pass(h, scene, TRANSPARENT_NAME);
        let mut k = 0u32;
        while k < h.read::<u32>(scene + 0x3854) {
            let array: u32 = h.read(scene + 0x385c);
            let instance: u32 = h.read(array.wrapping_add(4 * k));
            if instance != 0 && visible(h, scene, instance) {
                let array: u32 = h.read(scene + 0x385c);
                let instance: u32 = h.read(array.wrapping_add(4 * k));
                render_for_pass(h, scene, instance);
            }
            k += 1;
        }
        let r = renderer(h, scene);
        h.call_sym(END_SCENE, Conv::Thiscall(r), &[1])
    }
}
