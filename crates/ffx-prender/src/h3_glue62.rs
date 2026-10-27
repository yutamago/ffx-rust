//! The render-side glue, sixty-second batch (M4 port round 18, list
//! `h3-glue-render`): the scene's video pass and the UI drawn over it.
//! The exception frame and cookie are left out, as in [`crate::glue10`].

pub use crate::h3_glue51::{
    BEGIN_SCENE, END_SCENE, SET_CLEAR_COLOR, SET_COLOR_TARGET, SET_DEPTH_STENCIL_TARGET,
    SET_VIEWPORT, SURFACE_CTOR, SURFACE_DTOR, UPDATE_VIEW_MATRICES,
};
pub use crate::h3_glue53::{DELEGATION_HELPER, MOVIE_FADING, SET_CLEAR_STENCIL};

/// `PostProcessManager_GetInstance()` (cdecl).
pub const POST_INSTANCE: u32 = crate::glue2::POST_PROCESS_INSTANCE;
/// `movie_have_camera()` (cdecl).
pub const MOVIE_HAVE_CAMERA: u32 = 0x0076_efc0;
/// `g_MoviePictureOn` (u32).
pub const MOVIE_PICTURE_ON: u32 = 0x0112_a00c;
/// `g_MovieDone` (u32): set when a movie's work is done; with it the
/// video pass draws the 3D world under the movie.
pub const MOVIE_DONE: u32 = 0x00c4_0e2c;
/// `PhyFMVPlayerManager_GetInstance()` (cdecl) and
/// `PhyFMVPlayerManager_Render(manager; renderer)` (`ret 4`).
pub const FMV_INSTANCE: u32 = 0x006d_73a0;
pub const FMV_RENDER: u32 = 0x006d_7570;
/// The scene's `preRender`, `RenderShadow`, `DrawMostVFX`,
/// `RenderMinimapStencil`, `DrawUI2DVFX` and `DrawNormalUI` (thiscall, no
/// arguments) and `Draw3DWorld(scene; pre)` (`ret 4`).
pub const PRE_RENDER: u32 = 0x0065_1be0;
pub const RENDER_SHADOW: u32 = 0x0065_3600;
pub const DRAW_MOST_VFX: u32 = 0x0065_0670;
pub const MINIMAP_STENCIL: u32 = 0x0065_0340;
pub const DRAW_UI_2D_VFX: u32 = 0x0065_0ea0;
pub const DRAW_NORMAL_UI: u32 = 0x0065_0cb0;
pub const DRAW_3D_WORLD: u32 = 0x0064_fbb0;
/// `g_PhyreScene3D` (byte).
pub const SCENE_3D: u32 = crate::h3_glue27::SCENE_3D;
/// `g_GraphicUi` and `MenuManager_GetMenuCount(ui)` (thiscall).
pub const GRAPHIC_UI: u32 = 0x00cc_c81c;
pub const MENU_COUNT: u32 = crate::h3_glue21::MENU_COUNT;
/// `RenderDelegationHelper_runFlag1000(helper; word)` (`ret 4`, `al`)
/// and the byte it answers into, `g_ScreenFadeDelegated`.
pub const RUN_FLAG_1000: u32 = 0x006a_e450;
pub const SCREEN_FADE_DELEGATED: u32 = 0x00cc_b9c1;
/// `PostProcessManager_renderScreenFade(post; renderer, camera, &surface,
/// 0)` (`ret 0x10`).
pub const RENDER_SCREEN_FADE: u32 = crate::h3_glue51::RENDER_SCREEN_FADE;
/// `Instance_006dc800_Get()` (cdecl) and
/// `VideoText_render_006ddeb0(text; renderer)` (`ret 4`) on its `+0x110`.
pub const INSTANCE_6DC800: u32 = crate::h3_glue38::INSTANCE_6DC800;
pub const VIDEO_TEXT_RENDER: u32 = 0x006d_deb0;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// The frame's surfaces (`ebp -` these) and the camera matrix copy.
    const VIDEO: u32 = 0x20;
    const COLOR: u32 = 0x30;
    const DEPTH: u32 = 0x40;
    const MATRIX: u32 = 0x80;
    const FADE_COLOR: u32 = 0x90;
    const FADE_DEPTH: u32 = 0xa0;

    fn renderer(h: &impl Host, scene: u32) -> u32 {
        h.read(scene + 0xd0)
    }

    /// The post targets `+0x8c` (colour) and `+0x90` (depth), read in
    /// that order.
    fn targets(h: &impl Host, scene: u32) -> (u32, u32) {
        let post: u32 = h.read(scene + 0x10cc);
        let color = h.read(post.wrapping_add(0x8c));
        let depth = h.read(post.wrapping_add(0x90));
        (color, depth)
    }

    fn surface(h: &mut impl Host, at: u32, target: u32) {
        h.call_sym(SURFACE_CTOR, Conv::Thiscall(at), &[target, 0]);
    }

    fn drop_surface(h: &mut impl Host, at: u32) {
        h.call_sym(SURFACE_DTOR, Conv::Thiscall(at), &[]);
    }

    fn scene_call(h: &mut impl Host, scene: u32, sym: u32) {
        h.call_sym(sym, Conv::Thiscall(scene), &[]);
    }

    fn clear_color(h: &mut impl Host, scene: u32) {
        h.call_sym(SET_CLEAR_COLOR, Conv::Thiscall(renderer(h, scene)), &[0; 4]);
    }

    /// The colour surface at `ebp - color` and the depth surface at
    /// `ebp - DEPTH` set as targets: the first error.
    fn set_targets(h: &mut impl Host, ebp: u32, scene: u32, color: u32) -> u32 {
        let error = h.call_sym(
            SET_COLOR_TARGET,
            Conv::Thiscall(renderer(h, scene)),
            &[ebp - color, 0],
        );
        if error != 0 {
            return error;
        }
        h.call_sym(
            SET_DEPTH_STENCIL_TARGET,
            Conv::Thiscall(renderer(h, scene)),
            &[ebp - DEPTH],
        )
    }

    /// The viewport set to the size of the surface at `ebp - at` (its
    /// texture slot's texture; none when the slot's byte `+4` is set).
    fn viewport(h: &mut impl Host, ebp: u32, scene: u32, at: u32) {
        let slot: u32 = h.read(ebp - at);
        let texture: u32 = if h.read::<u8>(slot.wrapping_add(4)) == 0 {
            h.read(slot)
        } else {
            0
        };
        let height: u32 = h.read(texture.wrapping_add(0x20));
        let width: u32 = h.read(texture.wrapping_add(0x1c));
        h.call_sym(
            SET_VIEWPORT,
            Conv::Thiscall(renderer(h, scene)),
            &[0, 0, width, height],
        );
    }

    /// The movie's frame drawn: `PhyFMVPlayerManager_Render`'s answer.
    fn fmv_render(h: &mut impl Host, scene: u32) -> u32 {
        let r = renderer(h, scene);
        let manager = h.call_sym(FMV_INSTANCE, Conv::Cdecl, &[]);
        h.call_sym(FMV_RENDER, Conv::Thiscall(manager), &[r])
    }

    /// `PhyreScene_RenderVideoAndItsUI(scene)` (thiscall, plain `ret`;
    /// frame from `ebp - 0xa0`) -> the movie render's answer (0 when no
    /// movie pass runs), or the first failing renderer call's code.
    ///
    /// The post-process manager's byte `+0x123d9` cleared. Without
    /// `g_MovieFading`, the video pass: the colour target in a surface at
    /// `ebp - 0x20`, set as the target (its answer unused), cleared
    /// black with `beginScene(1)`/`endScene(1)`. Without a movie camera
    /// and a movie picture, the movie is drawn straight into it at its
    /// size. Otherwise [`camera_pass`]. Then, with the scene's fade byte
    /// `+0x10dcc` or `g_MovieFading` (read again if it was 0), and the
    /// minimap byte `+0x78`, a pending word `+0x3860` or UI menus open:
    /// the UI on the colour and depth targets (surfaces at `ebp - 0x20`
    /// and `- 0x40`; stencil cleared, flags 8, the minimap's stencil
    /// with `+0x78`, the 2D VFX and the normal UI). With the byte
    /// `+0xf9c5`, the screen fade unless a flag-0x1000 delegation drew
    /// it (surfaces at `ebp - 0x90` and `- 0xa0`). Last, the video text
    /// on the renderer. Each surface is destroyed on every way out.
    pub fn render_video_and_its_ui(h: &mut impl Host, ebp: u32, scene: u32) -> u32 {
        let post = h.call_sym(POST_INSTANCE, Conv::Cdecl, &[]);
        h.write(post.wrapping_add(0x123d9), 0_u8);
        let mut result = 0;
        let mut fading: u32 = h.read(h.rebase(MOVIE_FADING));
        if fading == 0 {
            let (color, _) = targets(h, scene);
            surface(h, ebp - VIDEO, color);
            match video_pass(h, ebp, scene) {
                Ok(r) => result = r,
                Err(error) => {
                    drop_surface(h, ebp - VIDEO);
                    return error;
                }
            }
            drop_surface(h, ebp - VIDEO);
            fading = h.read(h.rebase(MOVIE_FADING));
        }
        if (h.read::<u8>(scene + 0x10dcc) != 0 || fading != 0) && wants_ui(h, scene) {
            let (color, depth) = targets(h, scene);
            surface(h, ebp - VIDEO, color);
            surface(h, ebp - DEPTH, depth);
            let error = ui_pass(h, ebp, scene);
            drop_surface(h, ebp - DEPTH);
            drop_surface(h, ebp - VIDEO);
            if error != 0 {
                return error;
            }
        }
        if h.read::<u8>(scene + 0xf9c5) != 0 {
            screen_fade(h, ebp, scene);
        }
        let r = renderer(h, scene);
        let text = h.call_sym(INSTANCE_6DC800, Conv::Cdecl, &[]);
        h.call_sym(
            VIDEO_TEXT_RENDER,
            Conv::Thiscall(text.wrapping_add(0x110)),
            &[r],
        );
        result
    }

    /// The video pass with its surface at `ebp - 0x20` built.
    fn video_pass(h: &mut impl Host, ebp: u32, scene: u32) -> Result<u32, u32> {
        h.call_sym(
            SET_COLOR_TARGET,
            Conv::Thiscall(renderer(h, scene)),
            &[ebp - VIDEO, 0],
        );
        clear_color(h, scene);
        let error = h.call_sym(BEGIN_SCENE, Conv::Thiscall(renderer(h, scene)), &[1]);
        if error != 0 {
            return Err(error);
        }
        let error = h.call_sym(END_SCENE, Conv::Thiscall(renderer(h, scene)), &[1]);
        if error != 0 {
            return Err(error);
        }
        let camera = h.call_sym(MOVIE_HAVE_CAMERA, Conv::Cdecl, &[]);
        if camera == 0 && h.read::<u32>(h.rebase(MOVIE_PICTURE_ON)) == 0 {
            h.call_sym(
                SET_COLOR_TARGET,
                Conv::Thiscall(renderer(h, scene)),
                &[ebp - VIDEO, 0],
            );
            viewport(h, ebp, scene, VIDEO);
            return Ok(fmv_render(h, scene));
        }
        scene_call(h, scene, PRE_RENDER);
        if h.read::<u8>(scene + 0x93e) == 0 {
            scene_call(h, scene, RENDER_SHADOW);
        }
        let (color, depth) = targets(h, scene);
        surface(h, ebp - COLOR, color);
        surface(h, ebp - DEPTH, depth);
        let r = camera_pass(h, ebp, scene);
        drop_surface(h, ebp - DEPTH);
        drop_surface(h, ebp - COLOR);
        r
    }

    /// The movie with a camera or picture: the colour and depth targets
    /// (surfaces at `ebp - 0x30` and `- 0x40`) cleared (stencil, black,
    /// flags 0xb), the movie drawn. With `g_MovieDone`, the camera's
    /// matrix (`+0x34`, 16 words) kept at `ebp - 0x80`, the 3D world
    /// drawn, and the matrix put back through the x87 with the view
    /// matrices updated unless the scene's byte `+0xf9c7` or
    /// `g_PhyreScene3D` is set. With `g_MoviePictureOn`, the VFX drawn.
    fn camera_pass(h: &mut impl Host, ebp: u32, scene: u32) -> Result<u32, u32> {
        let ok = |e: u32| if e == 0 { Ok(()) } else { Err(e) };
        ok(set_targets(h, ebp, scene, COLOR))?;
        viewport(h, ebp, scene, COLOR);
        h.call_sym(SET_CLEAR_STENCIL, Conv::Thiscall(renderer(h, scene)), &[0]);
        clear_color(h, scene);
        ok(h.call_sym(BEGIN_SCENE, Conv::Thiscall(renderer(h, scene)), &[0xb]))?;
        ok(h.call_sym(END_SCENE, Conv::Thiscall(renderer(h, scene)), &[1]))?;
        ok(set_targets(h, ebp, scene, COLOR))?;
        viewport(h, ebp, scene, COLOR);
        let result = fmv_render(h, scene);
        if h.read::<u32>(h.rebase(MOVIE_DONE)) != 0 {
            let camera: u32 = h.read(scene + 0xdc);
            for k in 0..16 {
                let w: u32 = h.read(camera.wrapping_add(0x34 + 4 * k));
                h.write(ebp - MATRIX + 4 * k, w);
            }
            ok(set_targets(h, ebp, scene, COLOR))?;
            viewport(h, ebp, scene, COLOR);
            ok(h.call_sym(BEGIN_SCENE, Conv::Thiscall(renderer(h, scene)), &[0]))?;
            h.call_sym(DRAW_3D_WORLD, Conv::Thiscall(scene), &[1]);
            ok(h.call_sym(END_SCENE, Conv::Thiscall(renderer(h, scene)), &[1]))?;
            if h.read::<u8>(scene + 0xf9c7) == 0 && h.read::<u8>(h.rebase(SCENE_3D)) == 0 {
                let camera = h.read::<u32>(scene + 0xdc).wrapping_add(4);
                for k in 0..16 {
                    let w: u32 = h.read(ebp - MATRIX + 4 * k);
                    h.write(camera.wrapping_add(0x30 + 4 * k), x87::store(x87::load(w)));
                }
                h.call_sym(UPDATE_VIEW_MATRICES, Conv::Thiscall(camera), &[]);
            }
        }
        if h.read::<u32>(h.rebase(MOVIE_PICTURE_ON)) != 0 {
            ok(set_targets(h, ebp, scene, COLOR))?;
            viewport(h, ebp, scene, COLOR);
            ok(h.call_sym(BEGIN_SCENE, Conv::Thiscall(renderer(h, scene)), &[0]))?;
            scene_call(h, scene, DRAW_MOST_VFX);
            ok(h.call_sym(END_SCENE, Conv::Thiscall(renderer(h, scene)), &[1]))?;
        }
        Ok(result)
    }

    /// Whether there is UI to draw: the minimap byte `+0x78`, the word
    /// `+0x3860`, or menus open (`g_GraphicUi`'s count, signed).
    fn wants_ui(h: &mut impl Host, scene: u32) -> bool {
        if h.read::<u8>(scene + 0x78) != 0 || h.read::<u32>(scene + 0x3860) != 0 {
            return true;
        }
        let ui: u32 = h.read(h.rebase(GRAPHIC_UI));
        ui != 0 && (h.call_sym(MENU_COUNT, Conv::Thiscall(ui), &[]) as i32) > 0
    }

    /// The UI pass with its surfaces at `ebp - 0x20` and `- 0x40`.
    fn ui_pass(h: &mut impl Host, ebp: u32, scene: u32) -> u32 {
        let error = set_targets(h, ebp, scene, VIDEO);
        if error != 0 {
            return error;
        }
        viewport(h, ebp, scene, VIDEO);
        h.call_sym(SET_CLEAR_STENCIL, Conv::Thiscall(renderer(h, scene)), &[0]);
        let error = h.call_sym(BEGIN_SCENE, Conv::Thiscall(renderer(h, scene)), &[8]);
        if error != 0 {
            return error;
        }
        if h.read::<u8>(scene + 0x78) != 0 {
            scene_call(h, scene, MINIMAP_STENCIL);
        }
        scene_call(h, scene, DRAW_UI_2D_VFX);
        scene_call(h, scene, DRAW_NORMAL_UI);
        h.call_sym(END_SCENE, Conv::Thiscall(renderer(h, scene)), &[1])
    }

    /// The screen fade (surfaces at `ebp - 0x90` and `- 0xa0`), unless
    /// a flag-0x1000 delegation answers that it drew it.
    fn screen_fade(h: &mut impl Host, ebp: u32, scene: u32) {
        let (color, depth) = targets(h, scene);
        surface(h, ebp - FADE_COLOR, color);
        surface(h, ebp - FADE_DEPTH, depth);
        let helper: u32 = h.read(h.rebase(DELEGATION_HELPER));
        let drawn = h.call_sym(RUN_FLAG_1000, Conv::Thiscall(helper), &[0]) as u8;
        h.write(h.rebase(SCREEN_FADE_DELEGATED), drawn);
        if drawn == 0 {
            let camera: u32 = h.read(scene + 0xdc);
            let camera = if camera == 0 {
                0
            } else {
                camera.wrapping_add(4)
            };
            let r = renderer(h, scene);
            let post: u32 = h.read(scene + 0x10cc);
            h.call_sym(
                RENDER_SCREEN_FADE,
                Conv::Thiscall(post),
                &[r, camera, ebp - FADE_COLOR, 0],
            );
        }
        drop_surface(h, ebp - FADE_DEPTH);
        drop_surface(h, ebp - FADE_COLOR);
    }
}
