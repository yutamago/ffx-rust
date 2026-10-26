//! The render-side glue, twenty-first batch (M4 port round 6, list
//! `h3-glue-render`): two post-process passes with a render surface in
//! their frame (`renderFullScreenTexture`, `renderScreenFade`) and the
//! scene's normal UI pass. The exception frame and cookie are left out,
//! as in [`crate::glue10`]; the surface is destroyed on every path, as the
//! original's unwinding does.

use ffx_bridge::Global;

/// `Phyre_PRendering_PRenderSurface_PRenderSurface(this, texture, 0)` and
/// its destructor, `setColorTarget(this, surface, slot)`,
/// `setDepthStencilTarget(this, target)`, `setViewport(this, x, y, w,
/// h)`, `beginScene(this, a)`, `endScene(this, a)`,
/// `renderMeshInstance(this, mesh, data, 0, context)` and
/// `PApplication::GetApplication`.
pub const SURFACE_CTOR: u32 = crate::h3_glue18::SURFACE_CTOR;
pub const SURFACE_DTOR: u32 = crate::h3_glue18::SURFACE_DTOR;
pub const SET_COLOR_TARGET: u32 = crate::h3_glue18::SET_COLOR_TARGET;
pub const SET_DEPTH_STENCIL_TARGET: u32 = 0x005b_b5b0;
pub const SET_VIEWPORT: u32 = 0x005b_c060;
pub const BEGIN_SCENE: u32 = crate::h3_glue18::BEGIN_SCENE;
pub const END_SCENE: u32 = crate::h3_glue18::END_SCENE;
pub const RENDER_MESH_INSTANCE: u32 = 0x005b_9a90;
pub const GET_APPLICATION: u32 = crate::glue::GET_APPLICATION;

/// `ShaderParams_FindByName(shader, name)`, the texture binder
/// `Render_SurfaceFormat_0066e4f0(buffer, parameter, record)`,
/// `PostProcessManager_setNon16By9Viewport(renderer, target)` (stdcall,
/// `ecx` the manager), `"ColorBuffer"` and the full-screen pass entry.
pub const FIND_PARAMETER: u32 = 0x0056_cbf0;
pub const BIND_TEXTURE: u32 = 0x0066_e4f0;
pub const SET_NON_16_BY_9_VIEWPORT: u32 = 0x0069_8750;
pub const COLOR_BUFFER: u32 = 0x00b4_25d8;
pub const FULL_SCREEN_PASS: u32 = 0x00cc_c8bc;

/// The screen fade's colour parameter name and its three blend passes
/// (mode 2, mode 0x10, any other).
pub const FADE_COLOUR: u32 = 0x00b4_7a38;
pub const FADE_PASS_2: u32 = 0x00cc_c96c;
pub const FADE_PASS_16: u32 = 0x00cc_c97c;
pub const FADE_PASS: u32 = 0x00cc_c95c;

/// The normal UI pass: `g_MovieFading`, `g_PauseOn`, `g_GraphicUi`,
/// `clearScene(this, flags)`, `setCamera 0x005babd0(this, camera)`,
/// `PCamera_updateViewMatrices(this)`, `ViewerManager_GetInstance`,
/// `MenuManager_GetMenuCount(this)`, `0x0067b990(this, i)` (the menu),
/// `PhyreScene_isFading(this)`, the minimap viewport's size, y, `0x0065e4a0`
/// and x (`this`), `setScissor(this, x, y, w, h)`, `0x005b9a70(this,
/// mesh)`, the passes `"Opaque"`, `"Transparent"` and `0x00ccb9e8`.
pub const MOVIE_FADING: Global<u32> = Global::new(0x0134_0824);
pub const PAUSE_ON: Global<u32> = crate::h3_glue17::PAUSE_ON;
pub const GRAPHIC_UI: Global<u32> = Global::new(0x00cc_c81c);
pub const CLEAR_SCENE: u32 = 0x005b_6330;
pub const SET_CAMERA: u32 = 0x005b_abd0;
pub const UPDATE_VIEW_MATRICES: u32 = 0x0067_20c0;
pub const VIEWER_MANAGER: u32 = 0x006b_6370;
pub const MENU_COUNT: u32 = 0x0067_b970;
pub const MENU_AT: u32 = 0x0067_b990;
pub const IS_FADING: u32 = 0x0065_23e0;
pub const MINIMAP_SIZE: u32 = 0x0065_e320;
pub const MINIMAP_Y: u32 = 0x0065_e560;
pub const MINIMAP_65E4A0: u32 = 0x0065_e4a0;
pub const MINIMAP_X: u32 = 0x0065_e3f0;
pub const SET_SCISSOR: u32 = 0x005b_bdc0;
pub const RENDER_MESH: u32 = 0x005b_9a70;
pub const OPAQUE_PASS: u32 = 0x00c1_69b4;
pub const TRANSPARENT_PASS: u32 = 0x00c1_69d4;
pub const MINIMAP_PASS: u32 = 0x00cc_b9e8;

/// The shadow casters' visibility: `PMatrix4_FromMatrix3x4_00451080(out,
/// m)`, `PMatrix4_Multiply_004ca360(out, a, b)` (cdecl) and the caster
/// test `0x006c9270(this, matrix)` (`al`).
pub const MATRIX_FROM_3X4: u32 = 0x0045_1080;
pub const MATRIX_MULTIPLY: u32 = 0x004c_a360;
pub const CASTER_VISIBLE: u32 = 0x006c_9270;

/// The casters' extent: `PMatrix4_MulThis_004c9da0(a, out, b)`,
/// `PBounds_forward_005008c0(this, min, max, matrix)` and
/// `Phyre_Vec3Min_004f8580` / `Phyre_Vec3Max_004f84f0(out, a, b)` (cdecl).
pub const MATRIX_MUL_THIS: u32 = 0x004c_9da0;
pub const BOUNDS_FORWARD: u32 = 0x0050_08c0;
pub const VEC3_MIN: u32 = 0x004f_8580;
pub const VEC3_MAX: u32 = 0x004f_84f0;

/// The map's object arrays: `PhyreScene_detachChrClusterFrom3DWorld(this)`,
/// `PhyreSceneCursor_ctor_00633880(this, world)`, its step
/// `PhyreSceneCursor_Next_006377a0(this)`, `ObjectTree_Find(head, &key)`,
/// `operator new(size)` (cdecl), `PRedBlackTree_insertNew_0065fd50(tree,
/// &key, &value)`, `Array_Reserve_00668100(array, n)` and
/// `PhyreScene_fieldMapInitialize(this, a)`.
pub const DETACH_CHR_CLUSTER: u32 = 0x0065_66c0;
pub const CURSOR_CTOR: u32 = 0x0063_3880;
pub const CURSOR_NEXT: u32 = 0x0063_77a0;
pub const TREE_FIND: u32 = 0x0064_b9f0;
pub const OPERATOR_NEW: u32 = 0x0063_0310;
pub const TREE_INSERT: u32 = 0x0065_fd50;
pub const ARRAY_RESERVE: u32 = 0x0066_8100;
pub const FIELD_MAP_INITIALIZE: u32 = 0x0065_b8a0;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// `PArray`-style element pointer: the data `+4` with a count over 1,
    /// the inline element `+4` with 1, none with 0.
    fn array_data(h: &impl Host, array: u32) -> u32 {
        match h.read::<u32>(array) & 0x7fff_ffff {
            0 => 0,
            1 => array + 4,
            _ => h.read(array + 4),
        }
    }

    /// A render target's texture: `*target` unless its byte `+4` is set.
    fn target_texture(h: &impl Host, target: u32) -> u32 {
        if h.read::<u8>(target + 4) == 0 {
            h.read(target)
        } else {
            0
        }
    }

    /// `PostProcessManager_renderFullScreenTexture(this, renderer, texture,
    /// target)` (`ret 0xc`; frame: the surface at `ebp - 0x30`, the
    /// texture record `{0xff, 0, this + 0x123dc, texture}` at `ebp - 0x20`,
    /// `this` at `ebp - 0x10`): the full-screen mesh's (`+0x1208`) first
    /// material gets `texture` as its `"ColorBuffer"`; then the target (a
    /// surface over `target`, or the back buffer), no depth target, the
    /// viewport of the target's texture (or the application's size), the
    /// 16:9 fit and the mesh drawn in a scene. `eax`: the first error, or
    /// `endScene`'s.
    pub fn post_render_full_screen_texture(
        h: &mut impl Host,
        ebp: u32,
        this: u32,
        renderer: u32,
        texture: u32,
        target: u32,
    ) -> u32 {
        h.write(ebp - 0x10, this);
        let mesh: u32 = h.read(this + 0x1208);
        let materials: u32 = h.read(mesh + 0x10);
        let material: u32 = h.read(array_data(h, materials));
        let shader: u32 = h.read(material);
        let material: u32 = h.read(array_data(h, materials));
        let buffer: u32 = h.read(material + 4);
        let name = h.rebase(COLOR_BUFFER);
        let parameter = h.call_sym(FIND_PARAMETER, Conv::Thiscall(shader), &[name]);
        if parameter != 0 {
            let record = ebp - 0x20;
            h.write(ebp - 0x14, texture);
            h.write(record, 0xffu32);
            h.write(ebp - 0x18, this + 0x1_23dc);
            h.write(ebp - 0x1c, 0u32);
            let error = h.call_sym(BIND_TEXTURE, Conv::Thiscall(buffer), &[parameter, record]);
            if error != 0 {
                return error;
            }
        }
        if target == 0 {
            let error = h.call_sym(SET_COLOR_TARGET, Conv::Thiscall(renderer), &[0, 0]);
            if error != 0 {
                return error;
            }
        } else {
            let surface = ebp - 0x30;
            h.call_sym(SURFACE_CTOR, Conv::Thiscall(surface), &[target, 0]);
            let error = h.call_sym(SET_COLOR_TARGET, Conv::Thiscall(renderer), &[surface, 0]);
            h.call_sym(SURFACE_DTOR, Conv::Thiscall(surface), &[]);
            if error != 0 {
                return error;
            }
        }
        let error = h.call_sym(SET_DEPTH_STENCIL_TARGET, Conv::Thiscall(renderer), &[0]);
        if error != 0 {
            return error;
        }
        let (w, ht) = if target == 0 {
            let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
            let ht: u32 = h.read(app + 0x270);
            let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
            (h.read(app + 0x26c), ht)
        } else {
            let a = target_texture(h, target);
            let b = target_texture(h, target);
            let ht: u32 = h.read(a.wrapping_add(0x20));
            (h.read(b.wrapping_add(0x1c)), ht)
        };
        h.call_sym(SET_VIEWPORT, Conv::Thiscall(renderer), &[0, 0, w, ht]);
        let this: u32 = h.read(ebp - 0x10);
        h.call_sym(
            SET_NON_16_BY_9_VIEWPORT,
            Conv::Thiscall(this),
            &[renderer, target],
        );
        let error = h.call_sym(BEGIN_SCENE, Conv::Thiscall(renderer), &[1]);
        if error != 0 {
            return error;
        }
        let context: u32 = h.read(renderer + 0x13c);
        let pass = h.rebase(FULL_SCREEN_PASS);
        h.write(renderer + 0x48, pass);
        let mesh: u32 = h.read(this + 0x1208);
        let data: u32 = h.read(mesh + 0x10);
        h.call_sym(
            RENDER_MESH_INSTANCE,
            Conv::Thiscall(renderer),
            &[mesh, data, 0, context],
        );
        h.call_sym(END_SCENE, Conv::Thiscall(renderer), &[1])
    }

    /// `PostProcessManager_renderScreenFade(this, renderer, _, target, keep;
    /// slots)` (`ret 0x10`; frame: the surface at `ebp - 0x30`, the colour
    /// at `ebp - 0x20`, the alpha as loaded at `ebp - 0x10`): nothing with
    /// a zero alpha (`+0x12398`). Else a surface over `+0x8c` is the target
    /// unless `target` (or `keep`) is given; the fade mesh's (`+0x12388`)
    /// colour parameter gets the colour `+0x1238c` (each word stored back
    /// through the x87), its buffer goes over `keep`'s slot; the blend
    /// mode `+0x1239c` picks the pass, and the fade mesh (`+0x12384`) is
    /// drawn in a scene begun with 0. `eax`: 0, 0x13 (no mesh or
    /// parameter), 3 (the parameter out of the buffer), or the first
    /// renderer error, or `endScene`'s.
    pub fn post_render_screen_fade(
        h: &mut impl Host,
        ebp: u32,
        slots: u32,
        this: u32,
        renderer: u32,
        target: u32,
        keep: u32,
    ) -> u32 {
        let alpha_word: u32 = h.read(this + 0x1_2398);
        let alpha = x87::load(alpha_word);
        h.write(ebp - 0x10, x87::store(alpha));
        if alpha == 0.0 {
            return 0;
        }
        let surface = ebp - 0x30;
        let texture: u32 = h.read(this + 0x8c);
        h.call_sym(SURFACE_CTOR, Conv::Thiscall(surface), &[texture, 0]);
        let result = fade_pass(h, ebp, slots, this, renderer, target, keep);
        h.call_sym(SURFACE_DTOR, Conv::Thiscall(surface), &[]);
        result
    }

    fn fade_pass(
        h: &mut impl Host,
        ebp: u32,
        slots: u32,
        this: u32,
        renderer: u32,
        target: u32,
        keep: u32,
    ) -> u32 {
        let target = if target == 0 && keep as u8 == 0 {
            ebp - 0x30
        } else {
            target
        };
        let mesh: u32 = h.read(this + 0x1_2388);
        if mesh == 0 {
            return 0x13;
        }
        let shader: u32 = h.read(mesh);
        let name = h.rebase(FADE_COLOUR);
        let parameter = h.call_sym(FIND_PARAMETER, Conv::Thiscall(shader), &[name]);
        if parameter == 0 {
            return 0x13;
        }
        let colour = this + 0x1_238c;
        for k in 0..4 {
            let word: u32 = h.read(colour + 4 * k);
            h.write(ebp - 0x20 + 4 * k, word);
        }
        for k in 0..4 {
            let word: u32 = h.read(ebp - 0x20 + 4 * k);
            h.write(colour + 4 * k, x87::store(x87::load(word)));
        }
        let mesh: u32 = h.read(this + 0x1_2388);
        let buffer: u32 = h.read(mesh + 4);
        h.write(slots + 0xc, buffer);
        if colour == 0 {
            return 1;
        }
        let offset = u32::from(h.read::<u16>(parameter + 8));
        let size = u32::from(h.read::<u16>(parameter + 0xa) & 0x1fff);
        if offset + size > h.read::<u32>(buffer) || offset < 4 {
            return 3;
        }
        let mut bytes = vec![0; size as usize];
        h.read_bytes(colour, &mut bytes);
        h.write_bytes(buffer.wrapping_add(offset), &bytes);
        let pass = match h.read::<u32>(this + 0x1_239c) {
            2 => FADE_PASS_2,
            0x10 => FADE_PASS_16,
            _ => FADE_PASS,
        };
        let pass = h.rebase(pass);
        h.write(renderer + 0x48, pass);
        let error = h.call_sym(SET_COLOR_TARGET, Conv::Thiscall(renderer), &[target, 0]);
        if error != 0 {
            return error;
        }
        let error = h.call_sym(BEGIN_SCENE, Conv::Thiscall(renderer), &[0]);
        if error != 0 {
            return error;
        }
        let context: u32 = h.read(renderer + 0x13c);
        let mesh: u32 = h.read(this + 0x1_2384);
        let data: u32 = h.read(mesh + 0x10);
        h.call_sym(
            RENDER_MESH_INSTANCE,
            Conv::Thiscall(renderer),
            &[mesh, data, 0, context],
        );
        h.call_sym(END_SCENE, Conv::Thiscall(renderer), &[1])
    }

    /// One UI pass of `mesh` through `renderMeshInstance`.
    fn draw_ui_mesh(h: &mut impl Host, this: u32, pass: u32, mesh: u32) {
        let renderer: u32 = h.read(this + 0xd0);
        let pass = h.rebase(pass);
        h.write(renderer + 0x48, pass);
        let renderer: u32 = h.read(this + 0xd0);
        let context: u32 = h.read(renderer + 0x13c);
        let data: u32 = h.read(mesh + 0x10);
        h.call_sym(
            RENDER_MESH_INSTANCE,
            Conv::Thiscall(renderer),
            &[mesh, data, 0, context],
        );
    }

    /// One pass of `mesh` through `0x005b9a70`.
    fn draw_minimap_mesh(h: &mut impl Host, this: u32, pass: u32, mesh: u32) {
        let renderer: u32 = h.read(this + 0xd0);
        let pass = h.rebase(pass);
        h.write(renderer + 0x48, pass);
        let renderer: u32 = h.read(this + 0xd0);
        h.call_sym(RENDER_MESH, Conv::Thiscall(renderer), &[mesh]);
    }

    /// `PhyreScene_DrawNormalUI(this)` (`ecx` only): with the scene's UI
    /// flag (`+0x10dcc`) or a movie fading, the scene is cleared (2), the
    /// UI camera (`+0xa88`) set and updated; with the UI and its viewer
    /// shown (`+0x50`), each menu's mesh (`+0xc`) is drawn by its kind
    /// (`[+0x24] + 0xb4`): 0 opaque then transparent (and with the game
    /// paused and no fade, nothing after it), 2 while fading the minimap
    /// in its scissor, then the scissor back to the first post-process
    /// target's size. Returns 0.
    pub fn scene_draw_normal_ui(h: &mut impl Host, this: u32) -> u32 {
        if h.read::<u8>(this + 0x1_0dcc) == 0 && MOVIE_FADING.get(h) == 0 {
            return 0;
        }
        let renderer: u32 = h.read(this + 0xd0);
        h.call_sym(CLEAR_SCENE, Conv::Thiscall(renderer), &[2]);
        let camera = if this.wrapping_add(0xa84) == 0 {
            0
        } else {
            this + 0xa88
        };
        let renderer: u32 = h.read(this + 0xd0);
        h.call_sym(SET_CAMERA, Conv::Thiscall(renderer), &[camera]);
        h.call_sym(UPDATE_VIEW_MATRICES, Conv::Thiscall(this + 0xa88), &[]);
        if GRAPHIC_UI.get(h) == 0 {
            return 0;
        }
        let viewer = h.call_sym(VIEWER_MANAGER, Conv::Cdecl, &[]);
        if h.read::<u8>(viewer + 0x50) == 0 {
            return 0;
        }
        let ui = GRAPHIC_UI.get(h);
        let mut count = h.call_sym(MENU_COUNT, Conv::Thiscall(ui), &[]);
        let mut k = 0u32;
        while (k as i32) < (count as i32) {
            let ui = GRAPHIC_UI.get(h);
            let menu = h.call_sym(MENU_AT, Conv::Thiscall(ui), &[k]);
            let mesh = if menu == 0 { 0 } else { h.read(menu + 0xc) };
            if mesh != 0 {
                let info: u32 = h.read(menu + 0x24);
                let kind: u16 = h.read(info + 0xb4);
                if kind == 0 {
                    draw_ui_mesh(h, this, OPAQUE_PASS, mesh);
                    draw_ui_mesh(h, this, TRANSPARENT_PASS, mesh);
                    if PAUSE_ON.get(h) != 0
                        && h.call_sym(IS_FADING, Conv::Thiscall(this), &[]) as u8 == 0
                    {
                        return 0;
                    }
                } else if kind == 2 && h.call_sym(IS_FADING, Conv::Thiscall(this), &[]) as u8 != 0 {
                    let size = h.call_sym(MINIMAP_SIZE, Conv::Thiscall(this), &[]);
                    let y = h.call_sym(MINIMAP_Y, Conv::Thiscall(this), &[]);
                    let c = h.call_sym(MINIMAP_65E4A0, Conv::Thiscall(this), &[]);
                    let x = h.call_sym(MINIMAP_X, Conv::Thiscall(this), &[]);
                    let renderer: u32 = h.read(this + 0xd0);
                    h.call_sym(SET_SCISSOR, Conv::Thiscall(renderer), &[x, c, y, size]);
                    draw_minimap_mesh(h, this, OPAQUE_PASS, mesh);
                    draw_minimap_mesh(h, this, MINIMAP_PASS, mesh);
                    let post: u32 = h.read(this + 0x10cc);
                    let target: u32 = h.read(post + 0x8c);
                    let a = target_texture(h, target);
                    let b = target_texture(h, target);
                    let ht: u32 = h.read(a.wrapping_add(0x20));
                    let w: u32 = h.read(b.wrapping_add(0x1c));
                    let renderer: u32 = h.read(this + 0xd0);
                    h.call_sym(SET_SCISSOR, Conv::Thiscall(renderer), &[0, 0, w, ht]);
                }
            }
            k += 1;
            let ui = GRAPHIC_UI.get(h);
            count = h.call_sym(MENU_COUNT, Conv::Thiscall(ui), &[]);
        }
        0
    }

    /// `ShadowManager_updateCasterVisibility_006ce6f0(this, camera)` (`ret
    /// 4`; frame: the product at `ebp - 0xdc`, the camera copy `- 0x9c`,
    /// the node's matrix `- 0x5c`, the list `- 0x1c`, the node `- 0x18`,
    /// `camera + 0x70` `- 0x14`, `this` `- 0x10`, the list cursor `- 0xc`,
    /// the slot `- 8` and the block's first index `- 4`; the kind cursor
    /// goes over `camera`'s argument slot, `slots`). Without the shadows on
    /// (`+0x4044`) the 64 flags `+0x10e8` are set to 1. Else for the 8
    /// blocks of 8 casters (lists `+0xde8`, kinds `+0x2ba8`, 12 bytes
    /// apart) that were visible (the flag 0x40 words before), the flag
    /// becomes the `al` of the first of the list's nodes that passes the caster test
    /// with the camera (times the node's matrix `+0x1c` -> `+0xc`, when it
    /// has one); a kind other than 'c' is always visible, an empty list
    /// never. `eax`: 1 without the shadows (the fill), else none the
    /// callers read.
    pub fn shadow_update_caster_visibility(
        h: &mut impl Host,
        ebp: u32,
        slots: u32,
        this: u32,
        camera: u32,
    ) -> u32 {
        h.write(ebp - 0x10, this);
        if h.read::<u8>(this + 0x4044) == 0 {
            for k in 0..64 {
                h.write(this + 0x10e8 + 4 * k, 1u32);
            }
            return 1;
        }
        let view = camera.wrapping_add(0x70);
        h.write(ebp - 0x14, view);
        let mut kind_at = this + 0x2ba8;
        let mut list_at = this + 0xde8;
        let mut base = 0x43au32;
        h.write(ebp - 4, base);
        while base < 0x47a {
            for k in 0..8u32 {
                h.write(ebp - 8, k);
                h.write(ebp - 0xc, list_at);
                h.write(slots, kind_at);
                let list: u32 = h.read(list_at);
                h.write(ebp - 0x1c, list);
                if list != 0 {
                    let index = base + k;
                    let was: u32 = h.read(this.wrapping_add(4 * index).wrapping_sub(0x100));
                    if was == 1 {
                        let visible = caster_visible(h, ebp, list, kind_at, view);
                        h.write(this.wrapping_add(4 * index), u32::from(visible));
                    }
                }
                list_at += 4;
                kind_at += 0xc;
            }
            h.write(ebp - 8, 8u32);
            h.write(ebp - 0xc, list_at);
            h.write(slots, kind_at);
            base += 8;
            h.write(ebp - 4, base);
        }
        base
    }

    /// One caster block's test (see [`shadow_update_caster_visibility`]):
    /// the first passing test's `al`, 1 for a kind other than 'c', 0.
    fn caster_visible(h: &mut impl Host, ebp: u32, list: u32, kind_at: u32, view: u32) -> u8 {
        let count = h.read::<u32>(list) & 0x7fff_ffff;
        if count == 0 {
            return 0;
        }
        if h.read::<u8>(kind_at) != b'c' {
            return 1;
        }
        let mut j = 0;
        loop {
            let nodes: u32 = h.read(list + 4);
            let node: u32 = h.read(h.read::<u32>(nodes.wrapping_add(4 * j)).wrapping_add(0x1c));
            h.write(ebp - 0x18, node);
            let local: u32 = h.read(node.wrapping_add(0xc));
            let matrix = if local == 0 {
                let mut words = [0u8; 0x40];
                h.read_bytes(view, &mut words);
                h.write_bytes(ebp - 0x9c, &words);
                ebp - 0x9c
            } else {
                h.call_sym(MATRIX_FROM_3X4, Conv::Thiscall(ebp - 0x5c), &[local]);
                h.call_sym(
                    MATRIX_MULTIPLY,
                    Conv::Cdecl,
                    &[ebp - 0xdc, view, ebp - 0x5c],
                )
            };
            let node: u32 = h.read(ebp - 0x18);
            let al = h.call_sym(CASTER_VISIBLE, Conv::Thiscall(node), &[matrix]) as u8;
            if al != 0 {
                return al;
            }
            j += 1;
            if j >= h.read::<u32>(list) & 0x7fff_ffff {
                return 0;
            }
        }
    }

    /// `fld dword` + `fstp dword`: a signalling NaN comes out quiet.
    fn quiet(word: u32) -> u32 {
        x87::store(x87::load(word))
    }

    /// One caster's bounds in the camera: its matrix (`[+0x1c] + 0xc`) as
    /// a 4x4 at `ebp - 0x98`, times the camera copy (`ebp - 0xe8`) into
    /// `ebp - 0x128`, its bounds forwarded into `min` and `max`.
    fn caster_bounds(h: &mut impl Host, ebp: u32, list: u32, k: u32, min: u32, max: u32) {
        let nodes: u32 = h.read(list + 4);
        let node: u32 = h.read(nodes.wrapping_add(4 * k));
        let m: u32 = h.read(node.wrapping_add(0x1c));
        let local: u32 = h.read(m.wrapping_add(0xc));
        h.call_sym(MATRIX_FROM_3X4, Conv::Thiscall(ebp - 0x98), &[local]);
        let nodes: u32 = h.read(list + 4);
        let node: u32 = h.read(nodes.wrapping_add(4 * k));
        let bounds: u32 = h.read(node.wrapping_add(0x1c));
        let product = h.call_sym(
            MATRIX_MUL_THIS,
            Conv::Thiscall(ebp - 0x98),
            &[ebp - 0x128, ebp - 0xe8],
        );
        h.call_sym(BOUNDS_FORWARD, Conv::Thiscall(bounds), &[min, max, product]);
    }

    /// `ShadowManager_castersExtent_006d2530(list, dx, dy, dz, camera)`
    /// (cdecl; `slots` its arguments; frame: the product at `ebp - 0x128`,
    /// the camera copy `- 0xe8`, the caster matrix `- 0x98`, the min and
    /// max results `- 0xa8`/`- 0x58`, the caster's min and max `- 0x48`/`-
    /// 0x38`, the running max `- 0x28` and min `- 0x18`, two scratch words
    /// `- 8`/`- 4`): the casters' (`list`, nodes `+4`, count `+0`) bounds in
    /// the camera (64 bytes at `camera`) are merged component-wise, and
    /// `max - min` goes to `dx`, `dz` and `dy` in that order; each
    /// difference is formed through `list`'s argument slot and the scratch
    /// words, as the original does.
    pub fn shadow_casters_extent(
        h: &mut impl Host,
        ebp: u32,
        slots: u32,
        list: u32,
        out: [u32; 3],
        camera: u32,
    ) {
        if list == 0 {
            return;
        }
        let mut words = [0u8; 0x40];
        h.read_bytes(camera, &mut words);
        h.write_bytes(ebp - 0xe8, &words);
        let (min, max) = (ebp - 0x18, ebp - 0x28);
        caster_bounds(h, ebp, list, 0, min, max);
        let mut k = 1;
        while k < h.read::<u32>(list) & 0x7fff_ffff {
            caster_bounds(h, ebp, list, k, ebp - 0x48, ebp - 0x38);
            let r = h.call_sym(VEC3_MIN, Conv::Cdecl, &[ebp - 0xa8, min, ebp - 0x48]);
            for c in 0..3 {
                let v: u32 = h.read(r + 4 * c);
                h.write(min + 4 * c, quiet(v));
            }
            let r = h.call_sym(VEC3_MAX, Conv::Cdecl, &[ebp - 0x58, max, ebp - 0x38]);
            for c in 0..3 {
                let v: u32 = h.read(r + 4 * c);
                h.write(max + 4 * c, quiet(v));
            }
            k += 1;
        }
        // x into dx (scratch -4 the min, -8 the max), then z into dz and y
        // into dy (scratch -8 the min, -4 the max)
        let [dx, dy, dz] = out;
        for (c, target, lo_at, hi_at) in [(0, dx, 4, 8), (2, dz, 8, 4), (1, dy, 8, 4)] {
            let lo = quiet(h.read(min + 4 * c));
            h.write(slots, lo);
            h.write(ebp - lo_at, lo);
            let hi = quiet(h.read(max + 4 * c));
            h.write(ebp - hi_at, hi);
            h.write(slots, hi);
            let d = x87::store(x87::sub(x87::load(hi), x87::load(lo)));
            h.write(target, d);
        }
    }

    /// Appends `value` to the array (count `+0`, capacity `+4`, data
    /// `+8`), grown by an eighth and 2 when the count passes the capacity
    /// (signed); a null slot is skipped.
    fn push_word(h: &mut impl Host, array: u32, value: u32) {
        let count = h.read::<u32>(array).wrapping_add(1);
        h.write(array, count);
        let count: u32 = h.read(array);
        if (count as i32) > (h.read::<u32>(array + 4) as i32) {
            let n = ((count as i32) / 8) as u32;
            let n = n.wrapping_add(count).wrapping_add(2);
            h.call_sym(ARRAY_RESERVE, Conv::Thiscall(array), &[n]);
        }
        let count: u32 = h.read(array);
        let data: u32 = h.read(array + 8);
        let slot = data.wrapping_add(count.wrapping_mul(4)).wrapping_sub(4);
        if slot != 0 {
            h.write(slot, value);
        }
    }

    /// `PhyreScene_rebuildMapObjects_00658ae0(this)` (`ecx`; frame: the
    /// cursor at `ebp - 0x38`, the new array `- 0x18`, `this` `- 0x14`,
    /// the key (then the second array) `- 0x10`, the tree `- 0xc`, the
    /// object `- 8` and its array `- 4`): the character clusters are
    /// detached; with the rebuild flag (`+0x1194`) and a field map
    /// (`+0x10d1`, `+0x10d4`), each of the map's objects is appended to
    /// the tree `+0x1198`'s array for its key (`+0x58`; a new empty array
    /// inserted when there is none), and those of kind 1 (`+0x4c`) to
    /// `+0x11c8` as well; then the field map is initialized again and the
    /// flag word cleared. `eax`: the initializer's, or the detach's
    /// without the flag.
    pub fn scene_rebuild_map_objects(h: &mut impl Host, ebp: u32, this: u32) -> u32 {
        h.write(ebp - 0x14, this);
        let r = h.call_sym(DETACH_CHR_CLUSTER, Conv::Thiscall(this), &[]);
        if h.read::<u8>(this + 0x1194) == 0 {
            return r;
        }
        let map: u32 = h.read(this + 0x10d4);
        if h.read::<u8>(this + 0x10d1) != 0 && map != 0 {
            let cursor = ebp - 0x38;
            h.call_sym(CURSOR_CTOR, Conv::Thiscall(cursor), &[map]);
            if h.read::<u32>(cursor + 0xc) != 0 {
                h.write(ebp - 0xc, this + 0x1198);
                crate::h3_glue14::bridge::walk(h, cursor, CURSOR_NEXT, |h, base| {
                    map_object(h, ebp, cursor, base);
                });
            }
        }
        let this: u32 = h.read(ebp - 0x14);
        let r = h.call_sym(FIELD_MAP_INITIALIZE, Conv::Thiscall(this), &[0]);
        h.write(this + 0x1194, 0u16);
        r
    }

    /// One map object of [`scene_rebuild_map_objects`].
    fn map_object(h: &mut impl Host, ebp: u32, cursor: u32, base: u32) {
        let offset: u32 = h.read(cursor + 0x10);
        let object = offset.wrapping_add(base);
        h.write(ebp - 8, object);
        let key: u32 = h.read(object.wrapping_add(0x58));
        h.write(ebp - 0x10, key);
        let tree: u32 = h.read(ebp - 0xc);
        let node = h.call_sym(TREE_FIND, Conv::Thiscall(tree), &[ebp - 0x10]);
        let array = if node != 0 && node.wrapping_add(0x10) != 0 {
            let array: u32 = h.read(node + 0x10);
            h.write(ebp - 4, array);
            array
        } else {
            let array = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0xc]);
            h.write(ebp - 4, array);
            if array != 0 {
                for k in 0..3 {
                    h.write(array + 4 * k, 0u32);
                }
            }
            h.write(ebp - 0x18, array);
            let tree: u32 = h.read(ebp - 0xc);
            h.call_sym(TREE_INSERT, Conv::Thiscall(tree), &[ebp - 0x10, ebp - 0x18]);
            h.read(ebp - 4)
        };
        let object: u32 = h.read(ebp - 8);
        push_word(h, array, object);
        if h.read::<u32>(object.wrapping_add(0x4c)) == 1 {
            let this: u32 = h.read(ebp - 0x14);
            h.write(ebp - 0x10, this + 0x11c8);
            let array: u32 = h.read(ebp - 0x10);
            let object: u32 = h.read(ebp - 8);
            push_word(h, array, object);
        }
    }
}
