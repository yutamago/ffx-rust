//! The render-side glue, twelfth batch (M4 port round 4, list
//! `h3-glue-render`): the four functions round 3 left for a closer look
//! (the character-fog fade, the minimap's top edge, the default render
//! targets restored, the auto-test manager's constructor) and more
//! render-side methods. Exception frames and cookies left out, as in
//! [`crate::glue10`].

use ffx_bridge::Global;

pub use crate::glue::SCENE;

/// `Phyre_FogEffectManager_setChrFogEffect(this; value)` (`ret 4`).
pub const SET_CHR_FOG_EFFECT: u32 = 0x006e_35c0;
/// `PhyreScene_getMinimapViewportSize(scene)` and `_ftol2_sse`'s
/// constants: the minimap bottoms (blitzball, the two types' others) and
/// the layout height.
pub const MINIMAP_VIEWPORT_SIZE: u32 = 0x0065_e320;
pub const MINIMAP_BOTTOM_BLITZBALL: Global<f32> = Global::new(0x00b4_3dc4);
pub const MINIMAP_BOTTOM_LOW: Global<f32> = Global::new(0x00b4_3dbc);
pub const MINIMAP_BOTTOM_HIGH: Global<f32> = Global::new(0x00b4_3dd0);
pub const MINIMAP_LAYOUT_HEIGHT: Global<f64> = Global::new(0x00b4_1d30);
/// The render-surface constructor `(this; target, 0)`, destructor
/// `(this)`, and the renderer's `setColorTarget(this; &surface, 0)` and
/// `setDepthStencilTarget(this; &surface)`.
pub const RENDER_SURFACE_CTOR: u32 = 0x005b_35c0;
pub const RENDER_SURFACE_DTOR: u32 = 0x005b_3640;
pub const SET_COLOR_TARGET: u32 = 0x005b_b130;
pub const SET_DEPTH_STENCIL_TARGET: u32 = 0x005b_b5b0;
/// `AutoTestManager`'s vtable, its command thread's entry, the word handed
/// to it and its name; `CreateThread_ps2(&param, name)`,
/// `StartThread_ps2(thread, 0)` (cdecl).
pub const AUTO_TEST_VTABLE: u32 = 0x00b4_8e30;
pub const AUTO_TEST_THREAD: u32 = 0x006b_d1e0;
pub const AUTO_TEST_THREAD_WORD: u32 = 0x00cc_cb10;
pub const AUTO_TEST_THREAD_NAME: u32 = 0x00b4_8e50;
pub const CREATE_THREAD_PS2: u32 = 0x006f_2e10;
pub const START_THREAD_PS2: u32 = 0x006f_2d30;
/// The fade-mesh pass: `g_MovieFading`, the renderer's `beginScene(this;
/// 0)`, `setCamera(this; camera)`, `endScene(this; 1)` and
/// `renderMeshInstance(this; mesh, instance, 0, pass)`,
/// `Phyre_PCamera_updateViewMatrices(camera)`, the name check
/// `Phyre_PRendering_NameIs_004ea7a0(name; other)` (`al`), the name it is
/// checked against and the technique word stored in the renderer.
pub const MOVIE_FADING: Global<u32> = Global::new(0x0134_0824);
pub const BEGIN_SCENE: u32 = 0x005b_5e30;
pub const SET_CAMERA: u32 = 0x005b_abd0;
pub const END_SCENE: u32 = 0x005b_6950;
pub const RENDER_MESH_INSTANCE: u32 = 0x005b_9a90;
pub const UPDATE_VIEW_MATRICES: u32 = 0x0067_20c0;
pub const NAME_IS: u32 = 0x004e_a7a0;
pub const FADE_SKIP_NAME: Global<u32> = Global::new(0x00cc_b9e4);
pub const FADE_TECHNIQUE: u32 = 0x00c1_69d4;
/// `user_malloc(size)`, `user_free_thunk(p)`, `memcpy` (cdecl).
pub const USER_MALLOC: u32 = 0x0068_7020;
pub const USER_FREE: u32 = 0x0068_7010;
pub const MEMCPY: u32 = 0x0094_9256;
/// `fiosUnifyFilename(name, out, size)`, the magic VFX texture folder,
/// the `sprintf` import slot and its format, `PhyreScene_unloadVFXTexture
/// (this; path, id)`.
pub const FIOS_UNIFY_FILENAME: u32 = 0x0067_9820;
pub const MAGIC_VFX_FOLDER: u32 = 0x00b4_3b4c;
pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;
pub const MAGIC_VFX_FORMAT: u32 = 0x00b4_3b70;
pub const UNLOAD_VFX_TEXTURE: u32 = 0x0067_1300;
/// The `printf` import slot, the shadow map's "full" message and the name
/// the single-cell insert remembers.
pub const PRINTF_SLOT: u32 = 0x00b0_c380;
pub const SHADOW_MAP_FULL: u32 = 0x00b4_9758;
pub const SHADOW_SPECIAL_NAME: u32 = 0x00b4_86bc;
/// The resize chain: `PhyreScene_setResolutionScale_00652910(scene)`, the
/// post-process resize `0x00697f20(this; w, h)`, the (getter, resize,
/// offset of `this`) triples and the Iggy viewport.
pub const SET_RESOLUTION_SCALE: u32 = 0x0065_2910;
pub const POST_PROCESS_RESIZE: u32 = 0x0069_7f20;
pub const RESIZE_PAIRS: [(u32, u32, u32); 6] = [
    (0x006c_3580, 0x006c_3c40, 0),
    (0x006d_4de0, 0x006d_59c0, 0),
    (0x006c_2290, 0x006c_2fd0, 0),
    (0x006a_6e40, 0x006a_a3d0, 0),
    (0x0069_2bf0, 0x0069_8060, 0),
    (0x006d_c8a0, 0x006d_ded0, 0x110),
];
pub const GET_APPLICATION: u32 = 0x0062_3b90;
pub const IGGY_MENU_MANAGER: Global<u32> = Global::new(0x0198_4c6c);
pub const IGGY_SET_VIEWPORT: u32 = 0x00a2_6960;
/// The fade-world pass: `clearScene(this; flags)`,
/// `PApplication::renderWorld(app; world, camera, pass)` and the two
/// techniques.
pub const CLEAR_SCENE: u32 = 0x005b_6330;
pub const RENDER_WORLD: u32 = 0x0062_be70;
pub const FADE_WORLD_TECHNIQUES: [u32; 2] = [0x00c1_6a04, 0x00c1_6a14];
/// The vector parameter writer's texture install `0x0059e520(texture;
/// arg)` and `PParameterBuffer_writeParameter(this; parameter, src)`.
pub const TEXTURE_INSTALL: u32 = 0x0059_e520;
pub const WRITE_PARAMETER: u32 = 0x004d_57d0;
/// The DTT frame copy's inputs: `MotionBlur_GetInstance`,
/// `g_OpusWaveOmdEnd`, `MagicFile_GetMagicId` and the magic IDs that
/// keep the frame in any scene state.
pub const MOTION_BLUR_INSTANCE: u32 = 0x006c_5130;
pub const OPUS_WAVE_OMD_END: Global<u32> = Global::new(0x012e_9628);
pub const MAGIC_ID: u32 = 0x009d_a3b0;
pub const DTT_MAGIC_IDS: [u32; 16] = [
    0x6a, 0x119, 0x133, 0x1ae, 0x25d, 0x203, 0x182, 0x241, 0x233, 0x264, 0x20a, 0x29b, 0x1bd, 0x49,
    0x10e, 0x92,
];
/// `Phyre_PFreeList_PFreeList(this; size, count, align, name)` and the
/// name the shader manager's three lists take.
pub const FREE_LIST_CTOR: u32 = 0x0049_c480;
pub const FREE_LIST_NAME: u32 = 0x00b1_3bc8;
/// `graphicSgSetSpecular`'s pieces: `PArray_Data_004d45a0(array)`,
/// `ShaderParams_FindByName(shader; name)`,
/// `ShaderParamBuffer_SetValue2(buffer; parameter, value)` and the four
/// parameter names (`SgSpec_Color`, `_LightVec`, ...).
pub const ARRAY_DATA: u32 = 0x004d_45a0;
pub const FIND_PARAMETER: u32 = 0x0056_cbf0;
pub const SET_PARAMETER_VALUE: u32 = 0x0063_86c0;
pub const SPECULAR_NAMES: [u32; 4] = [0x00b4_1b94, 0x00b4_1ba4, 0x00b4_1bb4, 0x00b4_1bc8];
/// The character loading list (count word with the owned bit, data),
/// `Ch_IsLoadingClothFile(chr)`, the instance maker `0x0063c540(chr,
/// arg)`, `operator_delete`, `Phyre_PhyreFree` (cdecl).
pub const CHARACTER_LOADING: Global<u32> = crate::glue10::CHARACTER_LOADING;
pub const CHARACTER_LOADING_DATA: Global<u32> = crate::glue10::CHARACTER_LOADING_DATA;
pub const IS_LOADING_CLOTH: u32 = 0x0082_a100;
pub const MAKE_INSTANCE: u32 = 0x0063_c540;
pub const OPERATOR_DELETE: u32 = crate::glue10::OPERATOR_DELETE;
pub const PHYRE_FREE: u32 = crate::glue10::PHYRE_FREE;
/// `PTexture2DD3D11::isBound(this)` (`al`) and `unbind(this)`.
pub const TEXTURE_IS_BOUND: u32 = 0x0059_5f90;
pub const TEXTURE_UNBIND: u32 = 0x0059_cc40;
/// `TextureAnimationManager_unregisterChararcterInstance(this; instance)`
/// and the animation entry's release `0x006b3fe0(entry)`.
pub const UNREGISTER_CHARACTER_INSTANCE: u32 = 0x006b_4330;
pub const ANIMATION_RELEASE: u32 = 0x006b_3fe0;
/// `addChrDrawSituation`'s callees: `Phyre_PWorld_removeCluster(world;
/// cluster)`, `Phyre_PWorld_addCluster(world; cluster)` and
/// `Array_Reserve_00668280(array; n)` (16-byte elements).
pub const WORLD_REMOVE_CLUSTER: u32 = 0x0044_3b70;
pub const WORLD_ADD_CLUSTER: u32 = 0x0044_3250;
pub const ARRAY_RESERVE_16: u32 = 0x0066_8280;
/// The loading screen's pieces: `graphicProcessWindowsMessage()`, the
/// import slot it is followed by (stdcall, one word),
/// `SleepThreadInMillisecond(ms)`, the application's update
/// `0x00628df0(app)`, `g_LoadingScreen`, its time setter
/// `0x006e50a0(this; f)` and `LoadingScreen_render(this; renderer)`,
/// `IggyMenuManager_Render_00a267f0(manager)`,
/// `PRenderer::flip(renderer)`, `PhyreScene_doDeleteMeshInstances(scene)`.
pub const PROCESS_WINDOWS_MESSAGE: u32 = 0x0064_2860;
pub const LOADING_IMPORT_SLOT: u32 = 0x00b0_c1c8;
pub const SLEEP_MS: u32 = 0x006f_2d20;
pub const APP_UPDATE: u32 = 0x0062_8df0;
pub const LOADING_SCREEN: Global<u32> = Global::new(0x00ce_517c);
pub const LOADING_SET_TIME: u32 = 0x006e_50a0;
pub const LOADING_RENDER: u32 = 0x006e_4ea0;
pub const IGGY_RENDER: u32 = 0x00a2_67f0;
pub const RENDERER_FLIP: u32 = 0x005b_6af0;
pub const DELETE_MESH_INSTANCES: u32 = 0x0065_a620;
/// `Phyre_PhyreMalloc_0042fb80(size, align)`, `operator_new(size)`
/// (cdecl).
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
pub const OPERATOR_NEW: u32 = 0x0063_0310;
/// The model buffers: the used count and the 0x18c-byte entries.
pub const MODEL_BUFFER_COUNT: Global<u32> = Global::new(0x00d4_b460);
pub const MODEL_BUFFERS: u32 = 0x00d3_2860;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// `FogEffectManager_updateChrFogFade_006e3f40(this; dt)` (`ret 4`):
    /// with the fade type `+0xb4` 0 (out) or 1 (in), the time `+0xb0`
    /// grows by `dt` and the character fog `+0xac` becomes `1 - t` (out)
    /// or `t` (in) clamped to `0..=1` (a NaN stays), handed to
    /// `setChrFogEffect`; past 1 the type becomes 2 (done). Any other
    /// type zeroes the time. No result (`eax` holds `fnstsw`).
    pub fn fog_update_chr_fade(h: &mut impl Host, this: u32, dt: u32) {
        let kind: u32 = h.read(this + 0xb4);
        if kind > 1 {
            h.write(this + 0xb0, 0f32);
            return;
        }
        // fadd with the argument as the memory operand
        let time = x87::Reg::load(h.read(this + 0xb0))
            .add_m32(f32::from_bits(dt))
            .store();
        h.write(this + 0xb0, time);
        let into = if kind == 0 { 0.0 } else { 1.0 };
        let from = f64::from(f32::from_bits(x87::store(x87::sub(1.0, into))));
        // from + (into - from) * time, rounded once at the store
        let step = x87::mul(x87::sub(into, from), x87::load(time.to_bits()));
        let value = x87::store(x87::add(step, from));
        h.write(this + 0xac, value);
        let v = f64::from(f32::from_bits(value));
        let clamped = if v > 1.0 {
            1f32.to_bits()
        } else if v < 0.0 {
            0
        } else {
            value
        };
        h.write(this + 0xac, clamped);
        h.call_sym(SET_CHR_FOG_EFFECT, Conv::Thiscall(this), &[clamped]);
        let after: f32 = h.read(this + 0xb0);
        if f64::from(after) > 1.0 {
            h.write(this + 0xb4, 2u32);
        }
    }

    /// `PhyreScene_getMinimapViewportTop_0065e4a0(scene)`: the target's
    /// height `h` (`+0x20` of the live render target
    /// `*(scene + 0x10cc) + 0x8c`), less `trunc(f32(h) - k * f32(h) /
    /// 416)` and the minimap's size, `k` the bottom by the minimap type
    /// `+0x74` (1, 3: 136; 2, 4: 384; 316.125 for both in blitzball,
    /// `scene[0]`; else 0).
    pub fn scene_minimap_viewport_top(h: &mut impl Host, scene: u32) -> u32 {
        let holder: u32 = h.read(h.read::<u32>(scene + 0x10cc) + 0x8c);
        let target = if h.read::<u8>(holder + 4) == 0 {
            h.read::<u32>(holder)
        } else {
            0
        };
        let height: u32 = h.read(target + 0x20);
        let bottom = match h.read::<u32>(scene + 0x74) {
            1..=4 if h.read::<u8>(scene) != 0 => MINIMAP_BOTTOM_BLITZBALL.get(h),
            1 | 3 => MINIMAP_BOTTOM_LOW.get(h),
            2 | 4 => MINIMAP_BOTTOM_HIGH.get(h),
            _ => 0.0,
        };
        let d = f64::from(f64::from(height) as f32);
        let size = h.call_sym(MINIMAP_VIEWPORT_SIZE, Conv::Thiscall(scene), &[]);
        let scaled = x87::div(
            x87::mul(x87::load(bottom.to_bits()), d),
            MINIMAP_LAYOUT_HEIGHT.get(h),
        );
        let t = x87::ftol(h, x87::sub(d, scaled));
        height.wrapping_sub(t).wrapping_sub(size)
    }

    /// `PhyreScene_restoreDefaultRenderTargets(scene)` (frame: two render
    /// surfaces at `ebp - 0x2c` and `ebp - 0x1c`): surfaces over the
    /// scene's default colour and depth targets (`*(scene + 0x10cc)`
    /// `+0x8c`, `+0x90`) set on the renderer `+0xd0`, then destroyed in
    /// reverse. `eax`: the last destructor's.
    pub fn scene_restore_default_targets(h: &mut impl Host, ebp: u32, scene: u32) -> u32 {
        let colour = ebp.wrapping_sub(0x2c);
        let depth = ebp.wrapping_sub(0x1c);
        let targets: u32 = h.read(scene + 0x10cc);
        let first: u32 = h.read(targets + 0x8c);
        let second: u32 = h.read(targets + 0x90);
        h.call_sym(RENDER_SURFACE_CTOR, Conv::Thiscall(colour), &[first, 0]);
        h.call_sym(RENDER_SURFACE_CTOR, Conv::Thiscall(depth), &[second, 0]);
        let renderer: u32 = h.read(scene + 0xd0);
        h.call_sym(SET_COLOR_TARGET, Conv::Thiscall(renderer), &[colour, 0]);
        let renderer: u32 = h.read(scene + 0xd0);
        h.call_sym(SET_DEPTH_STENCIL_TARGET, Conv::Thiscall(renderer), &[depth]);
        h.call_sym(RENDER_SURFACE_DTOR, Conv::Thiscall(depth), &[]);
        h.call_sym(RENDER_SURFACE_DTOR, Conv::Thiscall(colour), &[])
    }

    /// `AutoTestManager_ctor_006bca90(this)` (frame: the thread parameters
    /// at `ebp - 0x30`): the vtable, 0xff zero bytes from `+4` (`+0x103`
    /// keeps its byte), zeros and 0.0s up to `+0x120` and the word
    /// `+0x128`; then the "Read command" thread made (`+0x124`) and
    /// started. The parameter block's first word and the frame above
    /// `ebp - 0x18` stay unwritten: `CreateThread_ps2` reads only `+4`
    /// (the entry) and `+0xc` (the stack size). `eax`: `this`.
    pub fn auto_test_manager_ctor(h: &mut impl Host, ebp: u32, this: u32) -> u32 {
        let vtable = h.rebase(AUTO_TEST_VTABLE);
        h.write(this, vtable);
        h.write_bytes(this + 4, &[0; 0xff]);
        for (off, v) in [
            (0x104, 0u32),
            (0x108, 0),
            (0x10c, 0),
            (0x110, 0),
            (0x114, 0),
            (0x118, 0),
            (0x11c, 0),
            (0x120, 0),
        ] {
            h.write(this + off, v);
        }
        h.write(this + 0x128, 0u16);
        let param = ebp.wrapping_sub(0x30);
        let entry = h.rebase(AUTO_TEST_THREAD);
        let word = h.rebase(AUTO_TEST_THREAD_WORD);
        for (off, v) in [(4, entry), (8, word), (0xc, 0x8000), (0x10, 0), (0x14, 1)] {
            h.write(param + off, v);
        }
        let name = h.rebase(AUTO_TEST_THREAD_NAME);
        let thread = h.call_sym(CREATE_THREAD_PS2, Conv::Cdecl, &[param, name]);
        h.write(this + 0x124, thread);
        h.call_sym(START_THREAD_PS2, Conv::Cdecl, &[thread, 0]);
        this
    }

    /// `PhyreScene_DrawUI2DVFX(scene)`: while the scene
    /// fades (`+0x10dcc`) or a movie does, and the list `+0x3868` (count
    /// `+0x3860`) is not empty, a scene on the renderer `+0xd0` with the
    /// scene camera (`+0x500`, its matrices updated) and the fade
    /// technique: each mesh in the list not named
    /// `g(0x00ccb9e4) & !1` rendered with its instance `+0x10` and the
    /// renderer's pass `+0x13c`. `eax`: `beginScene`'s error, else
    /// `endScene`'s; 0 when nothing is drawn.
    pub fn scene_render_fade_meshes(h: &mut impl Host, scene: u32) -> u32 {
        if h.read::<u8>(scene + 0x10dcc) == 0 && MOVIE_FADING.get(h) == 0 {
            return 0;
        }
        if h.read::<u32>(scene + 0x3860) == 0 {
            return 0;
        }
        let renderer: u32 = h.read(scene + 0xd0);
        let err = h.call_sym(BEGIN_SCENE, Conv::Thiscall(renderer), &[0]);
        if err != 0 {
            return err;
        }
        let camera = if scene.wrapping_add(0x4fc) == 0 {
            0
        } else {
            scene.wrapping_add(0x500)
        };
        let renderer: u32 = h.read(scene + 0xd0);
        h.call_sym(SET_CAMERA, Conv::Thiscall(renderer), &[camera]);
        h.call_sym(
            UPDATE_VIEW_MATRICES,
            Conv::Thiscall(scene.wrapping_add(0x500)),
            &[],
        );
        let renderer: u32 = h.read(scene + 0xd0);
        let technique = h.rebase(FADE_TECHNIQUE);
        h.write(renderer + 0x48, technique);
        let mut i = 0u32;
        while i < h.read::<u32>(scene + 0x3860) {
            let list: u32 = h.read(scene + 0x3868);
            let mesh: u32 = h.read(list.wrapping_add(4 * i));
            if mesh != 0 {
                let skip = FADE_SKIP_NAME.get(h);
                let name = skip.wrapping_sub(skip & 1);
                let same = h.call_sym(NAME_IS, Conv::Thiscall(mesh + 0x2c), &[name]) as u8;
                if same == 0 {
                    let renderer: u32 = h.read(scene + 0xd0);
                    let pass: u32 = h.read(renderer + 0x13c);
                    let instance: u32 = h.read(mesh + 0x10);
                    h.call_sym(
                        RENDER_MESH_INSTANCE,
                        Conv::Thiscall(renderer),
                        &[mesh, instance, 0, pass],
                    );
                }
            }
            i += 1;
        }
        let renderer: u32 = h.read(scene + 0xd0);
        h.call_sym(END_SCENE, Conv::Thiscall(renderer), &[1])
    }

    /// `PhyreScene_addUniqueToArray386c_00656a00(scene; v)` (`ret 4`):
    /// `v` appended to the word array `+0x3874` (count `+0x386c`,
    /// capacity `+0x3870`) unless there already. Growing to `n + n / 8 +
    /// 2` (signed) copies `n` words, one more than were there, from the
    /// old storage, which is freed (a failed allocation leaves none).
    /// `eax`: the entry found, or the new entry's address.
    pub fn scene_add_unique(h: &mut impl Host, scene: u32, v: u32) -> u32 {
        let count: u32 = h.read(scene + 0x386c);
        let data: u32 = h.read(scene + 0x3874);
        for k in 0..count {
            let at = data.wrapping_add(4 * k);
            if h.read::<u32>(at) == v {
                return at;
            }
        }
        let count = count.wrapping_add(1);
        h.write(scene + 0x386c, count);
        if count as i32 > h.read::<i32>(scene + 0x3870) {
            let n = count as i32;
            let cap = (n / 8 + 2).wrapping_add(n);
            let old: u32 = h.read(scene + 0x3874);
            if old == 0 {
                let p = h.call_sym(USER_MALLOC, Conv::Cdecl, &[(cap as u32) << 2]);
                h.write(scene + 0x3874, p);
            } else {
                let mut p = 0;
                if cap > 0 {
                    p = h.call_sym(USER_MALLOC, Conv::Cdecl, &[(cap as u32) << 2]);
                    if p != 0 {
                        let n: u32 = h.read(scene + 0x386c);
                        let old: u32 = h.read(scene + 0x3874);
                        h.call_sym(MEMCPY, Conv::Cdecl, &[p, old, n << 2]);
                    }
                }
                let old: u32 = h.read(scene + 0x3874);
                h.call_sym(USER_FREE, Conv::Cdecl, &[old]);
                h.write(scene + 0x3874, p);
            }
            h.write(scene + 0x3870, cap);
        }
        let n: u32 = h.read(scene + 0x386c);
        let data: u32 = h.read(scene + 0x3874);
        let at = data.wrapping_add(n.wrapping_mul(4)).wrapping_sub(4);
        if at != 0 {
            h.write(at, v);
        }
        at
    }

    /// `PhyreScene_unloadMagicVFXTexture(scene; id)` (`ret 4`; frame: the
    /// folder at `ebp - 0x104`, the path at `ebp - 0x204`, the cookie at
    /// `ebp - 4`): the magic VFX folder unified; an `id` (not 0) in the
    /// loaded list `+0x30` (count `+0x28`) is cleared and, unless the
    /// magic path `+0xfb00` (less its bit 0) is 0 (then 5), its texture
    /// path formatted from the folder and the magic path and unloaded
    /// (`unloadVFXTexture`'s result). 0 when not in the list.
    pub fn scene_unload_magic_vfx_texture(h: &mut impl Host, ebp: u32, scene: u32, id: u32) -> u32 {
        let folder = ebp.wrapping_sub(0x104);
        let path = ebp.wrapping_sub(0x204);
        let name = h.rebase(MAGIC_VFX_FOLDER);
        h.call_sym(FIOS_UNIFY_FILENAME, Conv::Cdecl, &[name, folder, 0x100]);
        let n: u32 = h.read(scene + 0x28);
        let list: u32 = h.read(scene + 0x30);
        let mut found = None;
        for k in 0..n {
            let e: u32 = h.read(list.wrapping_add(4 * k));
            if e != 0 && e == id {
                found = Some(k);
                break;
            }
        }
        let Some(k) = found else {
            return 0;
        };
        let list: u32 = h.read(scene + 0x30);
        h.write(list.wrapping_add(4 * k), 0u32);
        let magic: u32 = h.read(scene + 0xfb00);
        let magic = magic.wrapping_sub(magic & 1);
        if magic == 0 {
            return 5;
        }
        let at = h.rebase(SPRINTF_SLOT);
        let sprintf: u32 = h.read(at);
        let format = h.rebase(MAGIC_VFX_FORMAT);
        h.call(sprintf, Conv::Cdecl, &[path, format, folder, magic]);
        h.call_sym(UNLOAD_VFX_TEXTURE, Conv::Thiscall(scene), &[path, id])
    }

    /// The shadow map's grid: 8 x 8 cell bytes at `+0x2ae8`; per cell `k`
    /// two words (`+0xde8`, `+0xee8`), its size (`+0x29e8`), a flag
    /// (`+0x12e8`), a 12-byte name (`+0x2ba8`) and a word (`+0x2ea8`).
    fn shadow_fill(h: &mut impl Host, this: u32, k: u32, a: u32, b: u32, size: u32) {
        h.write(this + 0xde8 + 4 * k, a);
        h.write(this + 0xee8 + 4 * k, b);
        h.write(this + 0x29e8 + 4 * k, size);
    }

    /// Copies the C string at `src` to `dst`, its 0 included.
    fn copy_str(h: &mut impl Host, dst: u32, src: u32) {
        let mut i = 0u32;
        loop {
            let c: u8 = h.read(src.wrapping_add(i));
            h.write(dst.wrapping_add(i), c);
            if c == 0 {
                break;
            }
            i = i.wrapping_add(1);
        }
    }

    /// Whether the C strings at `a` and `b` are equal.
    fn str_eq(h: &impl Host, a: u32, b: u32) -> bool {
        let mut i = 0u32;
        loop {
            let x: u8 = h.read(a.wrapping_add(i));
            if x != h.read::<u8>(b.wrapping_add(i)) {
                return false;
            }
            if x == 0 {
                return true;
            }
            i = i.wrapping_add(1);
        }
    }

    /// The "full" message through the `printf` import; `eax` with `al`
    /// cleared.
    fn shadow_full(h: &mut impl Host) -> u32 {
        let at = h.rebase(PRINTF_SLOT);
        let printf: u32 = h.read(at);
        let text = h.rebase(SHADOW_MAP_FULL);
        h.call(printf, Conv::Cdecl, &[text]) & 0xffff_ff00
    }

    /// `ShadowManager_InsertIntoShadowMap(this; a, b, &col, &row,
    /// name, word)` (`ret 0x18`): the first free 2 x 2 block of the grid
    /// (top-left rows and columns 0..=6, row-major) is taken: its row and
    /// column out, the four cell bytes set, the top-left cell's words,
    /// size 2, flag, name and word. `al`: 1 (`eax`'s rest is the
    /// word's), or 0 after the "full" message.
    #[allow(clippy::too_many_arguments)] // the original's
    pub fn shadow_insert_block(
        h: &mut impl Host,
        this: u32,
        a: u32,
        b: u32,
        col: u32,
        row: u32,
        name: u32,
        word: u32,
    ) -> u32 {
        let grid = this + 0x2ae8;
        for i in 0..7u32 {
            for j in 0..7u32 {
                let k = j + 8 * i;
                let cells = [k, k + 1, k + 8, k + 9];
                if cells.iter().any(|&c| h.read::<u8>(grid + c) != 0) {
                    continue;
                }
                h.write(row, i);
                h.write(col, j);
                shadow_fill(h, this, k, a, b, 2);
                for c in cells {
                    h.write(grid + c, 1u8);
                }
                h.write(this + 0x12e8 + 4 * k, 1u32);
                copy_str(h, this + 0x2ba8 + 12 * k, name);
                h.write(this + 0x2ea8 + 4 * k, word);
                return (word & 0xffff_ff00) | 1;
            }
        }
        shadow_full(h)
    }

    /// `ShadowManager_InsertIntoShadowMap_Double(this; a, b, &col, &row, name,
    /// word)` (`ret 0x18`): as [`shadow_insert_block`] with the first
    /// free cell (size 1); a `name` equal to the special one also goes to
    /// `+0x4010` (row) and `+0x4014` (column).
    #[allow(clippy::too_many_arguments)] // the original's
    pub fn shadow_insert_cell(
        h: &mut impl Host,
        this: u32,
        a: u32,
        b: u32,
        col: u32,
        row: u32,
        name: u32,
        word: u32,
    ) -> u32 {
        let grid = this + 0x2ae8;
        for k in 0..64u32 {
            if h.read::<u8>(grid + k) != 0 {
                continue;
            }
            let (i, j) = (k / 8, k % 8);
            h.write(row, i);
            h.write(col, j);
            shadow_fill(h, this, k, a, b, 1);
            h.write(grid + k, 1u8);
            h.write(this + 0x12e8 + 4 * k, 1u32);
            let special = h.rebase(SHADOW_SPECIAL_NAME);
            if str_eq(h, name, special) {
                h.write(this + 0x4010, i);
                h.write(this + 0x4014, j);
            }
            copy_str(h, this + 0x2ba8 + 12 * k, name);
            h.write(this + 0x2ea8 + 4 * k, word);
            return (word & 0xffff_ff00) | 1;
        }
        shadow_full(h)
    }

    /// The scene's render size (`+0x10dd0`, `+0x10dd4`).
    fn render_size(h: &impl Host, scene: u32) -> [u32; 2] {
        [h.read(scene + 0x10dd0), h.read(scene + 0x10dd4)]
    }

    /// `PhyreScene_resize(scene)`: the resolution scale set, then the
    /// post-process targets (`+0x10cc`) resized to the render size (a
    /// non-zero result returned), then the blur, broken screen,
    /// distortion, character lighting, post-process tweak and video text
    /// managers, and the Iggy viewport to the application's `+0x26c` x
    /// `+0x270`. `eax`: 0.
    pub fn scene_resize(h: &mut impl Host, scene: u32) -> u32 {
        h.call_sym(SET_RESOLUTION_SCALE, Conv::Thiscall(scene), &[]);
        let [w, ht] = render_size(h, scene);
        let targets: u32 = h.read(scene + 0x10cc);
        let err = h.call_sym(POST_PROCESS_RESIZE, Conv::Thiscall(targets), &[w, ht]);
        if err != 0 {
            return err;
        }
        for (getter, resize, off) in RESIZE_PAIRS {
            // the size is pushed before the getter (which takes nothing)
            let [w, ht] = render_size(h, scene);
            let this = h.call_sym(getter, Conv::Cdecl, &[]);
            h.call_sym(resize, Conv::Thiscall(this.wrapping_add(off)), &[w, ht]);
        }
        let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        let height: u32 = h.read(app + 0x270);
        let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        let width: u32 = h.read(app + 0x26c);
        let iggy = IGGY_MENU_MANAGER.get(h);
        h.call_sym(IGGY_SET_VIEWPORT, Conv::Thiscall(iggy), &[width, height]);
        0
    }

    /// The fade camera: `*(scene + 0x7e0) + 4`, 0 without one.
    fn fade_camera(h: &impl Host, scene: u32) -> u32 {
        let c: u32 = h.read(scene + 0x7e0);
        if c == 0 { 0 } else { c.wrapping_add(4) }
    }

    /// `PhyreScene_DrawUIPrerenderScene(scene)`: while the scene or
    /// a movie fades, a scene on the renderer `+0xd0` cleared (0xa); with
    /// `+0x93c` set, the fade camera (its matrices updated) and the world
    /// `+0x8c` rendered twice, with the two fade techniques (passes 1 and
    /// -1). `eax`: `beginScene`'s error, else `endScene`'s; 0 when not
    /// fading.
    pub fn scene_render_fade_world(h: &mut impl Host, scene: u32) -> u32 {
        if h.read::<u8>(scene + 0x10dcc) == 0 && MOVIE_FADING.get(h) == 0 {
            return 0;
        }
        let renderer: u32 = h.read(scene + 0xd0);
        let err = h.call_sym(BEGIN_SCENE, Conv::Thiscall(renderer), &[0]);
        if err != 0 {
            return err;
        }
        let renderer: u32 = h.read(scene + 0xd0);
        h.call_sym(CLEAR_SCENE, Conv::Thiscall(renderer), &[0xa]);
        if h.read::<u8>(scene + 0x93c) != 0 {
            let cam = fade_camera(h, scene);
            let renderer: u32 = h.read(scene + 0xd0);
            h.call_sym(SET_CAMERA, Conv::Thiscall(renderer), &[cam]);
            let c: u32 = h.read(scene + 0x7e0);
            h.call_sym(UPDATE_VIEW_MATRICES, Conv::Thiscall(c.wrapping_add(4)), &[]);
            for (technique, pass) in FADE_WORLD_TECHNIQUES.into_iter().zip([1, u32::MAX]) {
                let renderer: u32 = h.read(scene + 0xd0);
                let t = h.rebase(technique);
                h.write(renderer + 0x48, t);
                let cam = fade_camera(h, scene);
                let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
                h.call_sym(
                    RENDER_WORLD,
                    Conv::Thiscall(app),
                    &[scene + 0x8c, cam, pass],
                );
            }
        }
        let renderer: u32 = h.read(scene + 0xd0);
        h.call_sym(END_SCENE, Conv::Thiscall(renderer), &[1])
    }

    /// Whether a parameter's type byte is one the vector writers take.
    fn vector_parameter(kind: u8) -> bool {
        matches!(kind, 0x42..=0x44 | 0xd9..=0xdc | 0x12)
    }

    /// `Render_SurfaceFormat_0066e4f0(buffer; parameter, &record)` (`ret
    /// 8`; frame: the record copy at `ebp - 0x10`): 5 for a parameter
    /// type (`+2`) the writer doesn't take or no record; a record with a
    /// texture (`+8`, `+0xc`) installs it first (a non-zero result
    /// returned); then the record is copied and its kind word, unless
    /// already 0x42..0x44, becomes the parameter's type for 0x43 and
    /// 0x44, else 0x42, and the copy is written
    /// (`PParameterBuffer_writeParameter`'s result).
    pub fn write_vector_parameter(
        h: &mut impl Host,
        ebp: u32,
        this: u32,
        parameter: u32,
        record: u32,
    ) -> u32 {
        if !vector_parameter(h.read(parameter + 2)) {
            return 5;
        }
        if record == 0 {
            return 5;
        }
        let texture: u32 = h.read(record + 8);
        if texture != 0 {
            let arg: u32 = h.read(record + 0xc);
            if arg != 0 {
                let err = h.call_sym(TEXTURE_INSTALL, Conv::Thiscall(texture), &[arg]);
                if err != 0 {
                    return err;
                }
            }
        }
        let copy = ebp.wrapping_sub(0x10);
        let words: [u32; 4] = [
            h.read(record),
            h.read(record + 4),
            h.read(record + 8),
            h.read(record + 0xc),
        ];
        for (k, w) in (0u32..).zip(words) {
            h.write(copy + 4 * k, w);
        }
        if !matches!(words[0], 0x42..=0x44) {
            let kind = u32::from(h.read::<u8>(parameter + 2));
            let kind = if matches!(kind, 0x43 | 0x44) {
                kind
            } else {
                0x42
            };
            h.write(copy, kind);
        }
        h.call_sym(WRITE_PARAMETER, Conv::Thiscall(this), &[parameter, copy])
    }

    /// `PhyreScene_copyPreviousFrameBufferForDtt(scene)` (`al`; the rest
    /// of `eax` is garbage the one caller ignores): whether the frame is
    /// kept for the DTT effects. Not for a scene state `+1` above 4, nor
    /// while motion blur runs (with the targets' `+0x123bb` set); yes for
    /// a state 1 scene with `+2` and DTT objects (`+0x3854`), or with the
    /// opus wave buffer end set; else by the magic ID (a list, and 0x46
    /// and 0x23a in state 1).
    pub fn scene_copy_previous_frame_for_dtt(h: &mut impl Host, scene: u32) -> u8 {
        let state: u8 = h.read(scene + 1);
        if state > 4 {
            return 0;
        }
        let targets: u32 = h.read(scene + 0x10cc);
        if h.read::<u8>(targets + 0x123bb) != 0 {
            let blur = h.call_sym(MOTION_BLUR_INSTANCE, Conv::Cdecl, &[]);
            if h.read::<u8>(blur + 0x58) != 0 {
                return 0;
            }
        }
        if h.read::<u32>(scene + 0x3854) != 0 && state == 1 && h.read::<u8>(scene + 2) != 0 {
            return 1;
        }
        if OPUS_WAVE_OMD_END.get(h) != 0 {
            return 1;
        }
        let id = h.call_sym(MAGIC_ID, Conv::Cdecl, &[]);
        let state: u8 = h.read(scene + 1);
        u8::from(match id {
            0x46 | 0x23a => state == 1,
            _ => DTT_MAGIC_IDS.contains(&id),
        })
    }

    /// `BrokenScreenEffect_setMatrix_006d4a90(this; &m)` (`ret 4`): the
    /// 16 floats copied through the x87 into `+0x60..+0xa0`. `eax`: `m`.
    pub fn broken_screen_set_matrix(h: &mut impl Host, this: u32, m: u32) -> u32 {
        let mut v = [0f32; 16];
        for (k, x) in (0u32..).zip(v.iter_mut()) {
            *x = x87::round_trip(h.read(m + 4 * k));
        }
        for (k, x) in (0u32..).zip(v) {
            h.write(this + 0x60 + 4 * k, x);
        }
        m
    }

    /// An empty list head at `at`: its six words point at itself.
    fn empty_head(h: &mut impl Host, at: u32) {
        for k in 0..6 {
            h.write(at + 4 * k, at);
        }
    }

    /// `VfxShaderManager_ctor_006bf4a0(this)`: three empty lists (`+0`,
    /// `+0x30`, `+0x68`), each with a free list of 8 nodes of 0x14 bytes
    /// after its head; the bytes `+0x60` and `+0x98` cleared, `+0x64` = -1.
    /// `eax`: `this`. (The C++ exception frame is left out.)
    pub fn vfx_shader_manager_ctor(h: &mut impl Host, this: u32) -> u32 {
        let name = h.rebase(FREE_LIST_NAME);
        for head in [this, this + 0x30, this + 0x68] {
            empty_head(h, head);
            h.call_sym(
                FREE_LIST_CTOR,
                Conv::Thiscall(head + 0x18),
                &[0x14, 8, 4, name],
            );
        }
        h.write(this + 0x60, 0u8);
        h.write(this + 0x98, 0u8);
        h.write(this + 0x64, u32::MAX);
        this
    }

    /// `graphicSgSetSpecular(model, index, &colour, &light, &camera,
    /// &param)`: for each material of the model's mesh `index` (`+4`
    /// list, `+0x10` its materials, count `& 0x7fffffff`) the four
    /// specular parameters found by name are set, the missing ones
    /// skipped. Nothing for a null model or an index past the count. No
    /// result the caller reads.
    pub fn sg_set_specular(h: &mut impl Host, model: u32, index: u32, values: [u32; 4]) {
        if model == 0 {
            return;
        }
        if index >= h.read::<u32>(model) & 0x7fff_ffff {
            return;
        }
        let meshes: u32 = h.read(model + 4);
        let mesh: u32 = h.read(meshes.wrapping_add(4 * index));
        let list: u32 = h.read(mesh + 0x10);
        let mut k = 0u32;
        while k < h.read::<u32>(list) & 0x7fff_ffff {
            let data = h.call_sym(ARRAY_DATA, Conv::Thiscall(list), &[]);
            let entry: u32 = h.read(data.wrapping_add(4 * k));
            let shader: u32 = h.read(entry);
            let mut found = [0u32; 4];
            for (slot, name) in found.iter_mut().zip(SPECULAR_NAMES) {
                let name = h.rebase(name);
                *slot = h.call_sym(FIND_PARAMETER, Conv::Thiscall(shader), &[name]);
            }
            for (parameter, value) in found.into_iter().zip(values) {
                if parameter != 0 {
                    let buffer: u32 = h.read(entry + 4);
                    h.call_sym(
                        SET_PARAMETER_VALUE,
                        Conv::Thiscall(buffer),
                        &[parameter, value],
                    );
                }
            }
            k += 1;
        }
    }

    /// `graphicCharacterInstantiateLoaded_00639db0()`: walks the
    /// character loading list from its end; a request whose character is
    /// loaded (`+0x1c`) and not loading cloth is finished (with an asset,
    /// `+4`, the instance made into `*request` and the callback `+0xc`
    /// called with `+8`) and deleted; the others are swapped to the front.
    /// The count becomes the kept ones (never more than it was); an
    /// unowned, emptied list's storage is freed.
    pub fn character_instantiate_loaded(h: &mut impl Host) {
        let mut word = CHARACTER_LOADING.get(h);
        let mut last = (word & 0x7fff_ffff) as i32 - 1;
        let mut kept = 0i32;
        if last >= 0 {
            loop {
                let data = CHARACTER_LOADING_DATA.get(h);
                let request: u32 = h.read(data.wrapping_add(4 * last as u32));
                let chr: u32 = h.read(request + 4);
                let cloth = h.call_sym(IS_LOADING_CLOTH, Conv::Cdecl, &[chr]);
                let chr: u32 = h.read(request + 4);
                if h.read::<u8>(chr + 0x1c) != 0 && cloth == 0 {
                    let out: u32 = h.read(request);
                    let arg: u32 = h.read(request + 8);
                    let callback: u32 = h.read(request + 0xc);
                    if h.read::<u32>(chr + 4) != 0 {
                        let instance = h.call_sym(MAKE_INSTANCE, Conv::Cdecl, &[chr, arg]);
                        h.write(out, instance);
                        if callback != 0 {
                            h.call(callback, Conv::Cdecl, &[arg]);
                        }
                    }
                    h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[request]);
                    last -= 1;
                } else {
                    kept += 1;
                    let data = CHARACTER_LOADING_DATA.get(h);
                    let front = data.wrapping_add(4 * (kept as u32 - 1));
                    let moved: u32 = h.read(front);
                    h.write(front, request);
                    let data = CHARACTER_LOADING_DATA.get(h);
                    h.write(data.wrapping_add(4 * last as u32), moved);
                }
                if kept > last {
                    break;
                }
            }
            word = CHARACTER_LOADING.get(h);
        }
        let count = (word & 0x7fff_ffff).min(kept as u32);
        let word = (word & 0x8000_0000) | count;
        CHARACTER_LOADING.set(h, word);
        if word == 0 {
            let data = CHARACTER_LOADING_DATA.get(h);
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
            CHARACTER_LOADING_DATA.set(h, 0);
        }
    }

    /// The live object of a holder: `*holder` unless its byte `+4` is
    /// set (then 0).
    fn live(h: &impl Host, holder: u32) -> u32 {
        if h.read::<u8>(holder + 4) == 0 {
            h.read(holder)
        } else {
            0
        }
    }

    /// Unbinds the holder's live texture when it is bound.
    fn unbind_live(h: &mut impl Host, holder: u32) {
        let texture = live(h, holder);
        if h.call_sym(TEXTURE_IS_BOUND, Conv::Thiscall(texture), &[]) as u8 != 0 {
            h.call_sym(TEXTURE_UNBIND, Conv::Thiscall(texture), &[]);
        }
    }

    /// `PostProcessManager_setMSAA(pp; samples)` (`ret 4`): 0x13 unless
    /// `samples` is 0, 2, 4, 8 or 16; `+0x122b4` = `samples != 0`; the
    /// targets' textures unbound when bound (`+0x98` only without MSAA,
    /// then `+0x8c`, `+0x90`, `+0x94`) and those three holders' byte `+5`
    /// set to `samples`. `eax`: the last holder.
    pub fn post_process_set_msaa(h: &mut impl Host, pp: u32, samples: u32) -> u32 {
        if !matches!(samples, 0 | 2 | 4 | 8 | 16) {
            return 0x13;
        }
        let on = samples != 0;
        h.write(pp + 0x122b4, u8::from(on));
        if !on {
            let holder: u32 = h.read(pp + 0x98);
            unbind_live(h, holder);
        }
        let mut last = 0;
        for off in [0x8c, 0x90, 0x94] {
            let holder: u32 = h.read(pp + off);
            unbind_live(h, holder);
            last = h.read(pp + off);
            h.write(last + 5, samples as u8);
        }
        last
    }

    /// `TextureAnimationManager_unregisterByName_006b1a70(this; name)`
    /// (`ret 4`): each animation entry (`+0x68`, 0x20 bytes, count
    /// `+0x64`) whose name (`+0x18`, less its bit 0; not null) is `name`:
    /// the character instances registered on it (`+0x70`, 12-byte items
    /// of entry index and instance, count `+0x6c`) unregistered, then the
    /// entry released. Nothing for a null name.
    pub fn texture_animation_unregister_by_name(h: &mut impl Host, this: u32, name: u32) {
        if name == 0 {
            return;
        }
        let mut k = 0u32;
        while k < h.read::<u32>(this + 0x64) & 0x7fff_ffff {
            let entries: u32 = h.read(this + 0x68);
            let entry = entries.wrapping_add(0x20 * k);
            let word: u32 = h.read(entry + 0x18);
            let own = word.wrapping_sub(word & 1);
            if own != 0 && str_eq(h, own, name) {
                let mut j = 0u32;
                while j < h.read::<u32>(this + 0x6c) & 0x7fff_ffff {
                    let items: u32 = h.read(this + 0x70);
                    let item = items.wrapping_add(0xc * j);
                    if h.read::<u32>(item) == k {
                        let instance: u32 = h.read(item + 4);
                        h.call_sym(
                            UNREGISTER_CHARACTER_INSTANCE,
                            Conv::Thiscall(this),
                            &[instance],
                        );
                    }
                    j += 1;
                }
                let entries: u32 = h.read(this + 0x68);
                h.call_sym(
                    ANIMATION_RELEASE,
                    Conv::Thiscall(entries.wrapping_add(0x20 * k)),
                    &[],
                );
            }
            k += 1;
        }
    }

    /// `PhyreScene_addChrDrawSituation(scene; on)` (`ret 4`; `on` a
    /// byte): with `on`, the pending cluster `+0x11e4` moved from world
    /// `+0x7c` to world `+0x9c` once (`+0xfb44`); a 16-byte situation
    /// appended to `+0xfb5c` (count `+0xfb54`, capacity `+0xfb58`, grown
    /// to `n + n / 8 + 2`): 0, the draw index `+0xf8cc` plus the level's
    /// base (`+0xf910` by the depth `+0xf954`, when above 0), `on`
    /// sign-extended and the original's never-written `[ebp-4]` (0
    /// here); `+0xfb60` counted; the index and `+0xf8d0` pushed on the
    /// depth stacks `+0xf8d4`/`+0xf914`. `eax`: `+0xf8d0` (the caller's
    /// own `eax` without `on`: 0 here).
    pub fn scene_add_chr_draw_situation(h: &mut impl Host, scene: u32, on: u32) -> u32 {
        let on = on as u8;
        if on == 0 {
            return 0;
        }
        if h.read::<u8>(scene + 0xfb44) != 0 {
            let cluster = scene + 0x11e4;
            h.call_sym(
                WORLD_REMOVE_CLUSTER,
                Conv::Thiscall(scene + 0x7c),
                &[cluster],
            );
            h.call_sym(WORLD_ADD_CLUSTER, Conv::Thiscall(scene + 0x9c), &[cluster]);
            h.write(scene + 0xfb44, 0u8);
        }
        let depth: i32 = h.read(scene + 0xf954);
        let mut index: u32 = h.read(scene + 0xf8cc);
        if depth > 0 {
            let base: u32 = h.read((scene + 0xf910).wrapping_add((depth as u32).wrapping_mul(4)));
            index = index.wrapping_add(base);
        }
        let array = scene + 0xfb54;
        let count = h.read::<u32>(array).wrapping_add(1);
        h.write(array, count);
        if count as i32 > h.read::<i32>(scene + 0xfb58) {
            let n = count as i32;
            let cap = (n / 8).wrapping_add(n.wrapping_add(2));
            h.call_sym(ARRAY_RESERVE_16, Conv::Thiscall(array), &[cap as u32]);
        }
        let count: u32 = h.read(array);
        let data: u32 = h.read(scene + 0xfb5c);
        let entry = (count << 4).wrapping_add(data.wrapping_sub(0x10));
        if entry != 0 {
            // junk read: [ebp-0x4] never written; 0 (brief 2026-10-24)
            let junk = 0u32;
            for (k, w) in (0u32..).zip([0, index, i32::from(on as i8) as u32, junk]) {
                h.write(entry.wrapping_add(4 * k), w);
            }
        }
        let n: u32 = h.read(scene + 0xfb60);
        h.write(scene + 0xfb60, n.wrapping_add(1));
        let depth: u32 = h.read(scene + 0xf954);
        let index: u32 = h.read(scene + 0xf8cc);
        h.write((scene + 0xf8d4).wrapping_add(depth.wrapping_mul(4)), index);
        let depth: u32 = h.read(scene + 0xf954);
        let top: u32 = h.read(scene + 0xf8d0);
        h.write((scene + 0xf914).wrapping_add(depth.wrapping_mul(4)), top);
        h.write(scene + 0xf954, depth.wrapping_add(1));
        top
    }

    /// `PhyreScene_renderLoadingScreen(scene)`: unless the application
    /// shows the loading screen (`+0x3a0` or `+0x3a1`), only window
    /// messages (the import after a quit) and a 100 ms sleep. Else the
    /// application updated, the loading screen given the time `+0x24`
    /// (through the x87) and drawn on the renderer `+0x124` (its busy
    /// byte set around it), the Iggy menus drawn, the frame flipped and
    /// presented (vtable `+8`; a non-zero result of either returned), the
    /// application's vtable `+0x28`, window messages, a 60 ms sleep and
    /// the deleted mesh instances freed. `eax`: 0.
    pub fn scene_render_loading_screen(h: &mut impl Host, scene: u32) -> u32 {
        let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        let mut shown = h.read::<u8>(app + 0x3a0) != 0;
        if !shown {
            let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
            shown = h.read::<u8>(app + 0x3a1) != 0;
        }
        if !shown {
            pump_messages(h);
            h.call_sym(SLEEP_MS, Conv::Cdecl, &[100]);
            return 0;
        }
        let renderer = h
            .call_sym(GET_APPLICATION, Conv::Cdecl, &[])
            .wrapping_add(0x124);
        let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        h.call_sym(APP_UPDATE, Conv::Thiscall(app), &[]);
        let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        let time = x87::round_trip(x87::round_trip(h.read(app + 0x24)));
        let screen = LOADING_SCREEN.get(h);
        h.write(screen, 1u8);
        let screen = LOADING_SCREEN.get(h);
        h.call_sym(LOADING_SET_TIME, Conv::Thiscall(screen), &[time.to_bits()]);
        let screen = LOADING_SCREEN.get(h);
        h.call_sym(LOADING_RENDER, Conv::Thiscall(screen), &[renderer]);
        let screen = LOADING_SCREEN.get(h);
        h.write(screen, 0u8);
        let iggy = IGGY_MENU_MANAGER.get(h);
        if iggy != 0 {
            h.call_sym(IGGY_RENDER, Conv::Thiscall(iggy), &[]);
        }
        let err = h.call_sym(RENDERER_FLIP, Conv::Thiscall(renderer), &[]);
        if err != 0 {
            return err;
        }
        let vtable: u32 = h.read(renderer);
        let present: u32 = h.read(vtable + 8);
        let err = h.call(present, Conv::Thiscall(renderer), &[]);
        if err != 0 {
            return err;
        }
        let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        let vtable: u32 = h.read(app);
        let method: u32 = h.read(vtable + 0x28);
        h.call(method, Conv::Thiscall(app), &[]);
        pump_messages(h);
        h.call_sym(SLEEP_MS, Conv::Cdecl, &[0x3c]);
        h.call_sym(DELETE_MESH_INSTANCES, Conv::Thiscall(scene), &[]);
        0
    }

    /// `RenderDelegationHelper_init_006adc90(this)`: a 16-word index
    /// array at `+4` (count `+0`; made when the count is not 16, the old
    /// owned storage freed; a failed allocation keeps the old one) with
    /// every element cleared, then the 64 eight-byte slots from `+8`
    /// cleared and each given a new zeroed 0x14-byte node (`operator_new`;
    /// a null one is written through as the original does). `eax`: the
    /// last node.
    pub fn delegation_helper_init(h: &mut impl Host, this: u32) -> u32 {
        if h.read::<u32>(this) != 0x10 {
            let p = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[0x40, 4]);
            if p != 0 {
                h.write_bytes(p, &[0; 0x40]);
                let old: u32 = h.read(this + 4);
                if old != p && h.read::<u32>(this) >> 31 == 0 {
                    h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
                }
                h.write(this + 4, p);
                h.write(this, 0x10u32);
            }
        }
        let mut k = 0u32;
        while k < h.read::<u32>(this) & 0x7fff_ffff {
            let data: u32 = h.read(this + 4);
            h.write(data.wrapping_add(4 * k), 0u32);
            k += 1;
        }
        h.write_bytes(this + 8, &[0; 0x200]);
        let mut node = 0;
        for slot in 0..64u32 {
            node = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x14]);
            h.write(this + 8 + 8 * slot, node);
            h.write_bytes(node, &[0; 0x14]);
        }
        node
    }

    /// `ModelBuffer_ReleaseBufferByIndex(index)` (a 16-bit index): an
    /// entry in use (`+0`) loses a reference (`+4`); at 0 its buffers are
    /// freed (`+0x14`, `+0x18`, and the three 30-word arrays from
    /// `+0x24`), the entry cleared, and when it was the last one the
    /// count drops to just past the last used entry. No result.
    pub fn model_buffer_release(h: &mut impl Host, index: u32) {
        let index = index & 0xffff;
        if index as i32 >= MODEL_BUFFER_COUNT.get(h) as i32 {
            return;
        }
        let entry = h.rebase(MODEL_BUFFERS) + index * 0x18c;
        if h.read::<u32>(entry) == 0 {
            return;
        }
        let refs = h.read::<u32>(entry + 4).wrapping_sub(1);
        h.write(entry + 4, refs);
        if refs as i32 > 0 {
            return;
        }
        for at in [entry + 0x14, entry + 0x18] {
            free_word(h, at);
        }
        for k in 0..30 {
            for base in [0x24, 0x9c, 0x114] {
                free_word(h, entry + base + 4 * k);
            }
        }
        h.write_bytes(entry, &[0; 0x18c]);
        let count = MODEL_BUFFER_COUNT.get(h);
        if index != count.wrapping_sub(1) {
            return;
        }
        let base = h.rebase(MODEL_BUFFERS);
        let mut j = count as i32 - 2;
        while j >= 0 {
            if h.read::<u32>(base + j as u32 * 0x18c) != 0 {
                MODEL_BUFFER_COUNT.set(h, j as u32 + 1);
                return;
            }
            j -= 1;
        }
        MODEL_BUFFER_COUNT.set(h, 0);
    }

    /// Frees (`user_free_thunk`) the pointer at `at` unless it is null.
    fn free_word(h: &mut impl Host, at: u32) {
        let p: u32 = h.read(at);
        if p != 0 {
            h.call_sym(USER_FREE, Conv::Cdecl, &[p]);
        }
    }

    /// `graphicProcessWindowsMessage()`, and after a quit the import at
    /// [`LOADING_IMPORT_SLOT`] with 0.
    fn pump_messages(h: &mut impl Host) {
        if h.call_sym(PROCESS_WINDOWS_MESSAGE, Conv::Cdecl, &[]) != 0 {
            let at = h.rebase(LOADING_IMPORT_SLOT);
            let f: u32 = h.read(at);
            h.call(f, Conv::Stdcall, &[0]);
        }
    }
}
