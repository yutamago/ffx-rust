//! `PhyreScene_render` (`0x00668930`, M4 list `h3-glue-render`, agent
//! h3e): the addresses it calls and reads, and the small steps every stage
//! repeats (targets, viewports, buffer copies, world passes). The stages
//! are [`crate::h3e_scene`] (the 3D scene and its post-effects) and
//! [`crate::h3e_render`] (the rest, and the function itself).

/// The renderer (`PRenderer`, scene `+0xd0`).
pub const SET_COLOR_TARGET: u32 = 0x005b_b130;
pub const SET_DEPTH_STENCIL_TARGET: u32 = 0x005b_b5b0;
pub const SET_VIEWPORT: u32 = 0x005b_c060;
pub const SET_SCISSOR: u32 = 0x005b_bdc0;
pub const SET_CLEAR_COLOR: u32 = 0x005b_ada0;
pub const SET_CLEAR_STENCIL: u32 = 0x005b_afc0;
pub const BEGIN_SCENE: u32 = 0x005b_5e30;
pub const END_SCENE: u32 = 0x005b_6950;
pub const CLEAR_SCENE: u32 = 0x005b_6330;
pub const QUEUE_COMMAND_1E: u32 = 0x005b_b390;
/// `PRenderSurface(surface; target, b)` and its destructor (a bare `ret`).
pub const SURFACE_CTOR: u32 = 0x005b_35c0;
pub const SURFACE_DTOR: u32 = 0x005b_3640;
/// `PApplication::GetApplication()` (cdecl) and `renderWorld(app; world,
/// camera, pass)` (`ret 0xc`).
pub const GET_APPLICATION: u32 = 0x0062_3b90;
pub const RENDER_WORLD: u32 = 0x0062_be70;
/// Camera and matrix helpers.
pub const UPDATE_VIEW_MATRICES: u32 = 0x0067_20c0;
pub const MATRIX_COPY: u32 = 0x0045_3340;
pub const MATRIX_TIMES_ROWS: u32 = 0x004c_9bc0;
pub const CAMERA_COPY_4533B0: u32 = 0x0045_33b0;
pub const CAMERA_FROM_456430: u32 = 0x0045_6430;
/// The post-process manager (scene `+0x10cc`) and its methods.
pub const POST_INSTANCE: u32 = 0x0069_2be0;
pub const RESET_CROSS_FADE: u32 = 0x0069_8f70;
pub const SET_BYTE_123B1: u32 = 0x0069_83e0;
pub const SET_BYTE_123BB: u32 = 0x0069_8480;
pub const SET_CROSS_FADE_CAPTURE: u32 = 0x0069_8490;
pub const COPY_BUFFER: u32 = 0x0069_2c40;
pub const DEGRADE_RESOLUTION: u32 = 0x0069_3530;
pub const RENDER_HDAO_PASS: u32 = 0x0069_5ba0;
pub const POST_RENDER: u32 = 0x0069_4f10;
pub const RENDER_FOCUS_FRAME: u32 = 0x0069_5790;
pub const RENDER_SCREEN_FADE: u32 = 0x0069_7490;
pub const RENDER_USM: u32 = 0x0069_7670;
pub const RENDER_BRIGHT: u32 = 0x0069_5440;
pub const RENDER_DEBUG_4BY3: u32 = 0x0069_56d0;
pub const SET_NON_16BY9_VIEWPORT: u32 = 0x0069_8750;
pub const LIVE_POINTER_00AC: u32 = 0x0065_ded0;
pub const LIVE_POINTER_1232C: u32 = 0x0065_e2f0;
/// The scene's own methods (thiscall on the scene).
pub const REBUILD_MAP_OBJECTS: u32 = 0x0065_8ae0;
pub const PRE_RENDER: u32 = 0x0065_1be0;
pub const SET_SHADOW_SCALE_BIAS: u32 = 0x0065_42b0;
pub const RENDER_ABMAP: u32 = 0x0065_2a10;
pub const CLEAR_ARRAYS: u32 = 0x0065_9000;
pub const RENDER_VIDEO_AND_UI: u32 = 0x0065_3960;
pub const RENDER_CLOUD_UI: u32 = 0x0065_3060;
pub const CHECK_1195: u32 = 0x0066_62f0;
pub const RENDER_SHADOW: u32 = 0x0065_3600;
pub const RENDER_HDAO: u32 = 0x0066_b560;
pub const DRAW_NON_UI_PRERENDER: u32 = 0x0065_07d0;
pub const DRAW_3D_WORLD: u32 = 0x0064_fbb0;
pub const FUN_0065A590: u32 = 0x0065_a590;
pub const IS_GLOW_AFTER_VFX: u32 = 0x0065_efa0;
pub const IS_MOTION_BLUR_ON: u32 = 0x0066_01e0;
pub const FUN_006589E0: u32 = 0x0065_89e0;
pub const FUN_0066C770: u32 = 0x0066_c770;
pub const MERGE_VFX_ARRAYS: u32 = 0x0065_4650;
pub const DRAW_MOST_VFX: u32 = 0x0065_0670;
pub const MOVE_CLUSTER: u32 = 0x0065_8f00;
pub const RENDER_DTT_VFX: u32 = 0x0065_31b0;
pub const IS_SPECIAL_MAGIC: u32 = 0x0065_f1f0;
pub const COPY_PREVIOUS_FRAME_FOR_DTT: u32 = 0x0065_93d0;
pub const RENDER_MF152: u32 = 0x0065_3230;
pub const RENDER_POST_AA: u32 = 0x0065_35b0;
pub const IS_ANY_BATTLE_MAP: u32 = 0x0066_0330;
pub const RESTORE_DEFAULT_TARGETS: u32 = 0x0066_c7f0;
pub const DRAW_UI_PRERENDER: u32 = 0x0065_0f80;
pub const DRAW_UI_2D_VFX: u32 = 0x0065_0ea0;
pub const RENDER_VFX_AFTER_POST: u32 = 0x0065_38d0;
pub const MINIMAP_STENCIL: u32 = 0x0065_0340;
pub const DRAW_NORMAL_UI: u32 = 0x0065_0cb0;
pub const RELEASE_CLUSTER_OBJECTS: u32 = 0x0065_8f40;
/// `g_RenderDelegationHelper` and the delegation runs (thiscall on it).
pub const DELEGATION_HELPER: u32 = 0x00cc_ca98;
pub const BEFORE_SCENE: u32 = 0x006a_da20;
pub const RUN_FLAG_10: u32 = 0x006a_d8c0;
pub const RUN_FLAG_20: u32 = 0x006a_d970;
pub const RUN_FLAG_4000: u32 = 0x006a_d430;
pub const RUN_FLAG_40: u32 = 0x006a_d810;
pub const RUN_FLAG_100: u32 = 0x006a_d760;
pub const RUN_FLAG_200: u32 = 0x006a_d2d0;
pub const BEFORE_VFX: u32 = 0x006a_db80;
pub const FUN_006AE0C0: u32 = 0x006a_e0c0;
pub const BEFORE_EFFECTS: u32 = 0x006a_d6b0;
pub const RUN_FLAG_1000: u32 = 0x006a_e450;
pub const RUN_FLAG_400: u32 = 0x006a_d600;
pub const RUN_FLAG_800: u32 = 0x006a_d380;
/// Singletons (cdecl getters) and their methods.
pub const BROKEN_SCREEN_INSTANCE: u32 = 0x006d_4de0;
pub const BROKEN_SCREEN_CAPTURE: u32 = 0x006d_4e20;
pub const BROKEN_SCREEN_ACTIVE: u32 = 0x006d_5670;
pub const BROKEN_SCREEN_RENDER: u32 = 0x006d_5680;
pub const FMV_INSTANCE: u32 = 0x006d_73a0;
pub const FMV_HAS_STATE: u32 = 0x006d_a1f0;
pub const INPUT_INSTANCE: u32 = 0x0063_1300;
pub const INPUT_VIBRATION: u32 = 0x0063_1320;
pub const SKIP_PROMPT_RENDER: u32 = 0x006e_7f00;
pub const WITH_3D_SCENE: u32 = 0x0076_f0c0;
pub const PPP_FILL_WAIT: u32 = 0x0071_e810;
pub const OPERATOR_NEW: u32 = 0x0063_0310;
pub const FREE_CAMERA_CTOR: u32 = 0x0067_fe70;
pub const FREE_CAMERA_INSTANCE: u32 = 0x0042_f260;
pub const GET_MODE_OR_2: u32 = 0x0042_f750;
pub const MAGIC_ID: u32 = 0x009d_a3b0;
pub const BLUR_EX_INSTANCE: u32 = 0x006c_4a70;
pub const BLUR_EX_RENDER: u32 = 0x006c_4cf0;
pub const BLUR_INSTANCE: u32 = 0x006c_3580;
pub const BLUR_RENDER: u32 = 0x006c_3970;
pub const DISTORTION_INSTANCE: u32 = 0x006c_2290;
pub const DISTORTION_RENDER: u32 = 0x006c_2dc0;
pub const SPECULAR_INSTANCE: u32 = 0x006c_57e0;
pub const NEED_SPECULAR_PASS: u32 = 0x006c_59a0;
pub const RENDER_SPECULAR_PASS: u32 = 0x006c_5b60;
pub const RESET_SPECULAR_MAPS: u32 = 0x006c_5850;
pub const MOTION_BLUR_INSTANCE: u32 = 0x006c_5130;
pub const MOTION_BLUR_RENDER: u32 = 0x006c_5410;
pub const MOTION_BLUR_6C5570: u32 = 0x006c_5570;
pub const MOTION_BLUR_SET_WEIGHT: u32 = 0x006c_5600;
pub const MENU_COUNT: u32 = 0x0067_b970;
pub const INSTANCE_6DC800: u32 = 0x006d_c8a0;
pub const VIDEO_TEXT_RENDER: u32 = 0x006d_deb0;
pub const LOADING_SCREEN_RENDER: u32 = 0x006e_4ea0;
pub const SLEEP_MS: u32 = 0x006f_2d20;
pub const IGGY_RENDER: u32 = 0x00a2_67f0;
/// The debug tweak utilities: `CharacterLightingTweakUtil` (getter,
/// `render(util; renderer, camera, f)` `ret 0xc`), then the getter and
/// `render(util; renderer, f)` (`ret 8`) of the others, in call order.
pub const CHARACTER_LIGHTING_INSTANCE: u32 = 0x006a_6e40;
pub const CHARACTER_LIGHTING_RENDER: u32 = 0x006a_9f00;
pub const TWEAK_UTILS: [(u32, u32); 10] = [
    (0x0069_2bf0, 0x0069_53a0), // PostProcessTweakUtil
    (0x006b_cc90, 0x006b_d290), // graphicCheckPauseGame, PhyreScene_ReturnConst0
    (0x006b_6360, 0x006b_a610), // MapSwitchTweakUtil
    (0x006b_6330, 0x006b_a430), // CharacterSwitchTweakUtil
    (0x006b_6340, 0x006b_a4d0), // CharacterTextureAnimationTweakUtil
    (0x006b_6320, 0x006b_a390), // CharacterAnimationTweakUtil
    (0x006b_6310, 0x006b_a2e0), // BtlDebugTweakUtil
    (0x006b_62f0, 0x006b_a1a0), // BasicDebugTweakUtil
    (0x006b_6300, 0x006b_a240), // BtlChrParamConfigTweakUitl
    (0x006b_6350, 0x006b_a570), // GameParamConfigTweakUitl
];

/// Globals.
pub const FREE_CAMERA: u32 = 0x00cc_c830;
pub const GRAPHIC_ABMAP: u32 = 0x00cc_c838;
pub const GRAPHIC_UI: u32 = 0x00cc_c81c;
pub const SCREEN_FADE_DELEGATED: u32 = 0x00cc_b9c1;
pub const MAP_163_OR_164: u32 = 0x00cc_b9c4;
pub const LOADING_SCREEN: u32 = 0x00ce_517c;
/// A word `PhyreScene_render` tests before copying the colour to pp
/// `+0xac` (ffx.sym has no name for it yet).
pub const WORD_CEC19C: u32 = 0x00ce_c19c;
pub const WAIT_FRAMES_BY_TIME: u32 = 0x0112_a008;
pub const MOVIE_PICTURE_ON: u32 = 0x0112_a00c;
/// A word that, when 1, draws the debug 4:3 frame.
pub const WORD_1136FC0: u32 = 0x0113_6fc0;
pub const OPUS_WAVE_OMD_END: u32 = 0x012e_9628;
pub const NOISE_FILLED_0706: u32 = 0x012e_9640;
pub const CURRENT_MAP_NO: u32 = 0x012f_bc40;
pub const MOVIE_FADING: u32 = 0x0134_0824;
pub const PAUSE_ON: u32 = 0x0134_0828;
/// The word after `g_PauseOn`: with it set the scene draws only the cloud
/// UI and returns.
pub const PAUSE_WORD_134082C: u32 = 0x0134_082c;
pub const MENU_CAPTURE_STATE: u32 = 0x0134_0844;
pub const IGGY_MENU_MANAGER: u32 = 0x0198_4c6c;
pub const YN_TEST20_SEEN: u32 = 0x025d_5ebc;
/// The pass names stored at renderer `+0x48`.
pub const OPAQUE_NAME: u32 = 0x00c1_69b4;
pub const TRANSPARENT_NAME: u32 = 0x00c1_69d4;
pub const POST_OPAQUE_NAME: u32 = 0x00c1_6a34;
pub const POST_TRANSPARENT_NAME: u32 = 0x00c1_6a44;

/// `1.0f`.
pub const ONE: u32 = 0x3f80_0000;

#[allow(clippy::missing_errors_doc)] // `Err` is the renderer's error code, documented once
pub mod bridge {
    //! The steps over game memory, shared by the stages.

    #[allow(clippy::wildcard_imports)] // the module's addresses
    use super::*;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// A renderer call's answer: an error code stops the stage.
    pub type Step = Result<(), u32>;

    /// 0 is success; anything else is the error the function returns.
    pub fn ok(code: u32) -> Step {
        if code == 0 { Ok(()) } else { Err(code) }
    }

    /// A byte answer (`al`; the rest of `eax` is the callee's garbage).
    #[must_use]
    pub fn al(answer: u32) -> bool {
        answer.to_le_bytes()[0] != 0
    }

    /// A float copied through the x87 stack (`fld dword`, `fstp dword`):
    /// a signalling NaN comes out quiet.
    #[must_use]
    pub fn copied(bits: u32) -> u32 {
        x87::store(x87::load(bits))
    }

    pub fn renderer(h: &impl Host, scene: u32) -> u32 {
        h.read(scene + 0xd0)
    }

    pub fn post(h: &impl Host, scene: u32) -> u32 {
        h.read(scene + 0x10cc)
    }

    /// The post-process manager's word at `off` (a target slot).
    pub fn post_word(h: &impl Host, scene: u32, off: u32) -> u32 {
        let pp = post(h, scene);
        h.read(pp.wrapping_add(off))
    }

    pub fn global(h: &impl Host, sym: u32) -> u32 {
        h.read(h.rebase(sym))
    }

    /// The camera argument the passes take: the scene camera's `+4`, or 0
    /// without a camera.
    pub fn camera_arg(h: &impl Host, scene: u32) -> u32 {
        let camera: u32 = h.read(scene + 0xdc);
        if camera == 0 {
            0
        } else {
            camera.wrapping_add(4)
        }
    }

    /// A target slot's texture: none when the slot's byte `+4` is set.
    pub fn texture(h: &impl Host, slot: u32) -> u32 {
        if h.read::<u8>(slot.wrapping_add(4)) == 0 {
            h.read(slot)
        } else {
            0
        }
    }

    /// A singleton (its cdecl getter).
    pub fn instance(h: &mut impl Host, getter: u32) -> u32 {
        h.call_sym(getter, Conv::Cdecl, &[])
    }

    pub fn set_color(h: &mut impl Host, scene: u32, surface: u32, index: u32) -> u32 {
        h.call_sym(
            SET_COLOR_TARGET,
            Conv::Thiscall(renderer(h, scene)),
            &[surface, index],
        )
    }

    pub fn set_depth(h: &mut impl Host, scene: u32, surface: u32) -> u32 {
        h.call_sym(
            SET_DEPTH_STENCIL_TARGET,
            Conv::Thiscall(renderer(h, scene)),
            &[surface],
        )
    }

    /// The colour surface at `color` and the depth surface at `depth` set
    /// as the targets (index 0).
    pub fn set_targets(h: &mut impl Host, scene: u32, color: u32, depth: u32) -> Step {
        ok(set_color(h, scene, color, 0))?;
        ok(set_depth(h, scene, depth))
    }

    pub fn begin(h: &mut impl Host, scene: u32, flags: u32) -> u32 {
        h.call_sym(BEGIN_SCENE, Conv::Thiscall(renderer(h, scene)), &[flags])
    }

    pub fn end(h: &mut impl Host, scene: u32, flags: u32) -> u32 {
        h.call_sym(END_SCENE, Conv::Thiscall(renderer(h, scene)), &[flags])
    }

    pub fn clear_scene(h: &mut impl Host, scene: u32, flags: u32) {
        h.call_sym(CLEAR_SCENE, Conv::Thiscall(renderer(h, scene)), &[flags]);
    }

    pub fn clear_color(h: &mut impl Host, scene: u32, rgba: [u32; 4]) -> u32 {
        h.call_sym(SET_CLEAR_COLOR, Conv::Thiscall(renderer(h, scene)), &rgba)
    }

    /// The clear colour the scene context (`+0xcc`) holds at `+0x30..0x38`
    /// (each copied through the x87 stack), alpha 1.
    pub fn scene_clear_color(h: &mut impl Host, scene: u32) -> u32 {
        let ctx: u32 = h.read(scene + 0xcc);
        let rgb: [u32; 3] = [0x30, 0x34, 0x38].map(|off| copied(h.read(ctx.wrapping_add(off))));
        clear_color(h, scene, [rgb[0], rgb[1], rgb[2], ONE])
    }

    pub fn viewport(h: &mut impl Host, scene: u32, rect: [u32; 4]) {
        h.call_sym(SET_VIEWPORT, Conv::Thiscall(renderer(h, scene)), &rect);
    }

    pub fn scissor(h: &mut impl Host, scene: u32, rect: [u32; 4]) {
        h.call_sym(SET_SCISSOR, Conv::Thiscall(renderer(h, scene)), &rect);
    }

    /// `(0, 0, width, height)` of the slot's texture (`+0x1c`, `+0x20`).
    pub fn slot_rect(h: &impl Host, slot: u32) -> [u32; 4] {
        let t = texture(h, slot);
        let height: u32 = h.read(t.wrapping_add(0x20));
        let width: u32 = h.read(t.wrapping_add(0x1c));
        [0, 0, width, height]
    }

    /// `(0, 0, width, height)` of the application's window (`+0x26c`,
    /// `+0x270`; the application fetched twice, the height first).
    pub fn window_rect(h: &mut impl Host) -> [u32; 4] {
        let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        let height: u32 = h.read(app.wrapping_add(0x270));
        let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        let width: u32 = h.read(app.wrapping_add(0x26c));
        [0, 0, width, height]
    }

    /// The viewport set to the size of the surface at `surface` (its
    /// first word is its slot).
    pub fn viewport_to(h: &mut impl Host, scene: u32, surface: u32) {
        let slot: u32 = h.read(surface);
        let rect = slot_rect(h, slot);
        viewport(h, scene, rect);
    }

    /// `PostProcessManager_degradeResolution(post; renderer)`.
    pub fn degrade(h: &mut impl Host, scene: u32) {
        let r = renderer(h, scene);
        h.call_sym(DEGRADE_RESOLUTION, Conv::Thiscall(post(h, scene)), &[r]);
    }

    /// [`viewport_to`] and [`degrade`], which every pass on the colour
    /// target starts with.
    pub fn viewport_degraded(h: &mut impl Host, scene: u32, surface: u32) {
        viewport_to(h, scene, surface);
        degrade(h, scene);
    }

    pub fn surface(h: &mut impl Host, at: u32, target: u32) {
        h.call_sym(SURFACE_CTOR, Conv::Thiscall(at), &[target, 0]);
    }

    pub fn drop_surface(h: &mut impl Host, at: u32) {
        h.call_sym(SURFACE_DTOR, Conv::Thiscall(at), &[]);
    }

    /// `copyBuffer(post; renderer, texture of slot, dst, a, b, c)`.
    pub fn copy_buffer(h: &mut impl Host, scene: u32, slot: u32, dst: u32, abc: [u32; 3]) {
        let t = texture(h, slot);
        let r = renderer(h, scene);
        h.call_sym(
            COPY_BUFFER,
            Conv::Thiscall(post(h, scene)),
            &[r, t, dst, abc[0], abc[1], abc[2]],
        );
    }

    /// The colour (pp `+0x8c`) copied into the target at pp `+dst`, flag 1.
    pub fn copy_color(h: &mut impl Host, scene: u32, dst: u32) {
        let slot = post_word(h, scene, 0x8c);
        let to = post_word(h, scene, dst);
        copy_buffer(h, scene, slot, to, [1, 0, 0]);
    }

    /// The pass name the renderer's next draws use (renderer `+0x48`).
    pub fn set_pass(h: &mut impl Host, scene: u32, name: u32) {
        let r = renderer(h, scene);
        let at = h.rebase(name);
        h.write(r.wrapping_add(0x48), at);
    }

    /// `renderWorld(app; scene + world, camera, pass)`.
    pub fn render_world(h: &mut impl Host, scene: u32, world: u32, pass: u32) {
        let camera = camera_arg(h, scene);
        let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        h.call_sym(
            RENDER_WORLD,
            Conv::Thiscall(app),
            &[scene + world, camera, pass],
        );
    }

    /// `PostProcessManager_renderScreenFade(post; renderer, camera,
    /// surface, 0)`.
    pub fn screen_fade(h: &mut impl Host, scene: u32, surface: u32) {
        let camera = camera_arg(h, scene);
        let r = renderer(h, scene);
        h.call_sym(
            RENDER_SCREEN_FADE,
            Conv::Thiscall(post(h, scene)),
            &[r, camera, surface, 0],
        );
    }

    /// `MagicFile_GetMagicId()`.
    pub fn magic_id(h: &mut impl Host) -> u32 {
        h.call_sym(MAGIC_ID, Conv::Cdecl, &[])
    }

    /// `PhyreScene_isSpecialMagic_0065f1f0` (no `this`): its `al`.
    pub fn special_magic(h: &mut impl Host) -> bool {
        al(h.call_sym(IS_SPECIAL_MAGIC, Conv::Cdecl, &[]))
    }

    /// `FreeCamera_CreateInstance` then `FFXApplication_getModeOr2` on it.
    pub fn free_camera_mode(h: &mut impl Host) -> u32 {
        {
            let obj = instance(h, FREE_CAMERA_INSTANCE);
            h.call_sym(GET_MODE_OR_2, Conv::Thiscall(obj), &[])
        }
    }

    /// The motion blur singleton.
    pub fn motion_blur(h: &mut impl Host) -> u32 {
        h.call_sym(MOTION_BLUR_INSTANCE, Conv::Cdecl, &[])
    }

    /// A camera matrix (16 floats at camera `+0x34`) written from `from`
    /// through the x87 stack, then the camera's view matrices updated.
    pub fn set_camera_matrix(h: &mut impl Host, scene: u32, from: u32) {
        let camera: u32 = h.read(scene + 0xdc);
        for i in 0..16 {
            let v: u32 = h.read(from + 4 * i);
            h.write(camera.wrapping_add(0x34 + 4 * i), copied(v));
        }
        h.call_sym(
            UPDATE_VIEW_MATRICES,
            Conv::Thiscall(camera.wrapping_add(4)),
            &[],
        );
    }
}
