//! The render-side glue, fortieth batch (M4 port round 10, list
//! `h3-glue-render`): the minimap's stencil pass, the menu windows'
//! divider shapes, the cross-button icons.

use ffx_bridge::Global;

/// The scene's fade flag reads `g_MovieFading`; `g_GraphicUi` holds the
/// menus. The fade list skips the meshes named `g(0x00ccb9e4) & !1`
/// elsewhere ([`crate::h3_glue12`]); here it draws only those.
pub const MOVIE_FADING: Global<u32> = crate::h3_glue21::MOVIE_FADING;
pub const GRAPHIC_UI: Global<u32> = crate::h3_glue21::GRAPHIC_UI;
pub const STENCIL_NAME: Global<u32> = crate::h3_glue12::FADE_SKIP_NAME;

/// `setCamera(renderer; camera)` (`ret 4`), `PCamera_updateViewMatrices
/// (camera)`, `renderMeshInstance(renderer; mesh, instance, 0, pass)`
/// (`ret 0x10`), `setViewport` / `setScissor(renderer; x, y, w, h)` (`ret
/// 0x10`), the minimap viewport's getters (`this` only), the menus'
/// `MenuManager_GetMenuCount(ui)` and `0x0067b990(ui; i)` (`ret 4`).
pub const SET_CAMERA: u32 = crate::h3_glue21::SET_CAMERA;
pub const UPDATE_VIEW_MATRICES: u32 = crate::h3_glue21::UPDATE_VIEW_MATRICES;
pub const RENDER_MESH_INSTANCE: u32 = crate::h3_glue21::RENDER_MESH_INSTANCE;
pub const SET_VIEWPORT: u32 = crate::h3_glue21::SET_VIEWPORT;
pub const SET_SCISSOR: u32 = crate::h3_glue21::SET_SCISSOR;
pub const MINIMAP_X: u32 = crate::h3_glue21::MINIMAP_X;
pub const MINIMAP_65E4A0: u32 = crate::h3_glue21::MINIMAP_65E4A0;
pub const MINIMAP_Y: u32 = crate::h3_glue21::MINIMAP_Y;
pub const MINIMAP_SIZE: u32 = crate::h3_glue21::MINIMAP_SIZE;
pub const MENU_COUNT: u32 = crate::h3_glue21::MENU_COUNT;
pub const MENU_AT: u32 = crate::h3_glue21::MENU_AT;

/// The map objects' visibility: `PMatrix4_FromMatrix3x4_00451080(out;
/// m)` (`ret 4`), `PMatrix4_Multiply_004ca360(out, a, b)` (cdecl) and
/// `BoundingBox_IsVisible_00663a30(box; matrix)` (`ret 4`, `al`).
pub const MATRIX_FROM_3X4: u32 = crate::h3_glue21::MATRIX_FROM_3X4;
pub const MATRIX_MULTIPLY: u32 = crate::h3_glue21::MATRIX_MULTIPLY;
pub const BOX_IS_VISIBLE: u32 = 0x0066_3a30;

/// The passes stored in the renderer's `+0x48`: `"SetStencilFlag"`,
/// `"Transparent"` and `"EnableStencilTest"`.
pub const SET_STENCIL_PASS: u32 = 0x00c1_6b64;
pub const TRANSPARENT_PASS: u32 = crate::h3_glue21::TRANSPARENT_PASS;
pub const STENCIL_TEST_PASS: u32 = 0x00c1_6b74;

/// `graphicUiRemapX2(v)` / `graphicUiRemapY2(v)` (cdecl, a float in
/// `ST0`) and `TOMkpShapeXYWHUV(shape, x, y, w, h, u0, v0, u1, v1)`
/// (cdecl).
pub const UI_REMAP_X2: u32 = crate::h3_glue20::UI_REMAP_X2;
pub const UI_REMAP_Y2: u32 = crate::h3_glue20::UI_REMAP_Y2;
pub const MKP_SHAPE_XYWHUV: u32 = 0x0090_3bb0;

/// The divider shapes' `f32` constants. Style 1 (shape 200): the height
/// 30, width 1920, the tops 149 and 999, the UVs 0.1943359375,
/// 0.419921875 and 0.390625. The others (shape -1): x 210, width 1500,
/// height 2, the tops 224, 250, 305, 410, 590 and 1015, the UVs 1631,
/// 567, 1829 and 570.
pub const BAR_HEIGHT: u32 = 0x00b4_1d80;
pub const BAR_WIDTH: u32 = 0x00b4_a1ac;
pub const BAR_TOP: u32 = 0x00b5_eefc;
pub const BAR_BOTTOM: u32 = 0x00b9_2324;
pub const BAR_U: u32 = 0x00b6_5800;
pub const BAR_V0: u32 = 0x00b6_581c;
pub const BAR_V1: u32 = 0x00b6_5818;
pub const LINE_X: u32 = 0x00b5_ef1c;
pub const LINE_WIDTH: u32 = 0x00b5_f430;
pub const LINE_HEIGHT: u32 = 0x00b9_2268;
pub const LINE_224: u32 = 0x00b6_6da0;
pub const LINE_250: u32 = 0x00b5_a484;
pub const LINE_305: u32 = 0x00b5_ef44;
pub const LINE_410: u32 = 0x00b5_ef60;
pub const LINE_590: u32 = 0x00b5_efc4;
pub const LINE_1015: u32 = 0x00b6_6e20;
pub const LINE_U0: u32 = 0x00b6_58c0;
pub const LINE_V0: u32 = 0x00b6_58b8;
pub const LINE_U1: u32 = 0x00b6_58c4;
pub const LINE_V1: u32 = 0x00b4_9990;

/// `TOMkpShapeXYWHUVC2(shape, x, y, w, h, u0, v0, u1, v1, c0, c1)`
/// (cdecl) and the icons' `f64` factors 0.45, 0.39, 0.305, 0.55 and 0.5.
pub const MKP_SHAPE_XYWHUV_C2: u32 = 0x0090_3ee0;
pub const F_045: u32 = 0x00b4_3d50;
pub const F_039: u32 = 0x00b6_6c58;
pub const F_0305: u32 = 0x00b6_6c50;
pub const F_055: u32 = 0x00b5_f1a0;
pub const F_05: u32 = 0x00b9_2220;

/// How an icon's rectangle is placed in the sheet cell: shrunk (the
/// face buttons: `A` / `C` horizontal bars, `B` vertical, `D` left
/// aligned) or as given.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Fit {
    A,
    B,
    C,
    D,
    Plain,
}

/// `DrawCrossButtonIconRGBA`'s icons 600..=0x269: the fit and the `f32`
/// constants of `u0, v0, u1, v1` (0: 0.0).
pub const CROSS_ICONS: [(Fit, [u32; 4]); 18] = [
    (Fit::A, [0x00b4_3dd0, 0x00b6_6d9c, 0x00b6_58a4, 0x00b4_80d4]),
    (Fit::A, [0x00b4_1d98, 0x00b6_6d9c, 0x00b6_6218, 0x00b4_80d4]),
    (Fit::B, [0x00b4_3dd0, 0x00b5_01c0, 0x00b4_1d98, 0x00b5_f388]),
    (Fit::B, [0x00b4_1d98, 0x00b5_01c0, 0x00b5_ef74, 0x00b5_f388]),
    (Fit::C, [0x00b5_ef74, 0x00b6_6d9c, 0x00b6_68a8, 0x00b4_80d4]),
    (Fit::C, [0x00b6_6dd4, 0x00b6_6d9c, 0x00b6_6ddc, 0x00b4_80d4]),
    (Fit::D, [0x00b4_3dd0, 0x00b6_6db0, 0x00b4_1d98, 0x00b5_ef48]),
    (Fit::D, [0x00b4_1d98, 0x00b6_6db0, 0x00b5_ef74, 0x00b5_ef48]),
    (Fit::Plain, [0, 0x00b6_6da0, 0x00b5_ee8c, 0x00b5_4cfc]),
    (
        Fit::Plain,
        [0x00b4_3db8, 0x00b6_6da0, 0x00b6_6d8c, 0x00b5_4cfc],
    ),
    (
        Fit::Plain,
        [0x00b6_6d94, 0x00b6_6da0, 0x00b5_f374, 0x00b5_4cfc],
    ),
    (
        Fit::Plain,
        [0x00b6_61d4, 0x00b6_6da0, 0x00b5_f394, 0x00b5_4cfc],
    ),
    (
        Fit::Plain,
        [0x00b5_ef98, 0x00b6_6d98, 0x00b6_6de0, 0x00b4_80d4],
    ),
    (
        Fit::Plain,
        [0x00b6_6e00, 0x00b6_6d98, 0x00b6_6e0c, 0x00b4_80d4],
    ),
    (
        Fit::Plain,
        [0x00b6_5b24, 0x00b6_6d98, 0x00b6_64f0, 0x00b4_80d4],
    ),
    (
        Fit::Plain,
        [0x00b4_ae14, 0x00b6_6d98, 0x00b6_6e18, 0x00b4_80d4],
    ),
    (
        Fit::Plain,
        [0x00b5_f35c, 0x00b6_589c, 0x00b6_6dac, 0x00b6_61f4],
    ),
    (Fit::Plain, [0, 0x00b6_589c, 0x00b6_56cc, 0x00b6_5f08]),
];

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// The renderer's pass word `+0x48`.
    fn set_pass(h: &mut impl Host, this: u32, pass: u32) {
        let renderer: u32 = h.read(this + 0xd0);
        let pass = h.rebase(pass);
        h.write(renderer + 0x48, pass);
    }

    /// `renderMeshInstance(mesh, mesh + 0x10, 0, renderer + 0x13c)`.
    fn render_mesh(h: &mut impl Host, this: u32, mesh: u32) {
        let renderer: u32 = h.read(this + 0xd0);
        let pass: u32 = h.read(renderer + 0x13c);
        let instance: u32 = h.read(mesh + 0x10);
        h.call_sym(
            RENDER_MESH_INSTANCE,
            Conv::Thiscall(renderer),
            &[mesh, instance, 0, pass],
        );
    }

    /// The first post-process target's (`[this + 0x10cc] + 0x8c`) texture
    /// size, `(w, h)`: `*target` unless the target's byte `+4` is set
    /// (then 0, whose fields the original reads all the same).
    fn target_size(h: &impl Host, this: u32) -> (u32, u32) {
        let post: u32 = h.read(this + 0x10cc);
        let target: u32 = h.read(post + 0x8c);
        let texture: u32 = if h.read::<u8>(target + 4) == 0 {
            h.read(target)
        } else {
            0
        };
        (
            h.read(texture.wrapping_add(0x1c)),
            h.read(texture.wrapping_add(0x20)),
        )
    }

    /// Whether the tagged names (low bit a flag) are the same string: the
    /// same pointer, or both set and equal bytes up to the terminator.
    fn same_name(h: &impl Host, a: u32, b: u32) -> bool {
        let a = a.wrapping_sub(a & 1);
        let b = b.wrapping_sub(b & 1);
        if a == b {
            return true;
        }
        if a == 0 || b == 0 {
            return false;
        }
        let mut k = 0u32;
        loop {
            let x: u8 = h.read(a.wrapping_add(k));
            if x != h.read::<u8>(b.wrapping_add(k)) {
                return false;
            }
            if x == 0 {
                return true;
            }
            k = k.wrapping_add(1);
        }
    }

    /// The menus (`g_GraphicUi`) of kind 1 (`[+0x24] + 0xb4`): their mesh
    /// `+0xc` drawn.
    fn draw_kind1_menus(h: &mut impl Host, this: u32) {
        let ui = GRAPHIC_UI.get(h);
        if ui != 0 {
            let mut k = 0u32;
            let mut count = h.call_sym(MENU_COUNT, Conv::Thiscall(ui), &[]);
            while (k as i32) < (count as i32) {
                let ui = GRAPHIC_UI.get(h);
                let menu = h.call_sym(MENU_AT, Conv::Thiscall(ui), &[k]);
                if menu != 0 {
                    let info: u32 = h.read(menu + 0x24);
                    if h.read::<u16>(info + 0xb4) == 1 {
                        let mesh: u32 = h.read(menu + 0xc);
                        if mesh != 0 {
                            render_mesh(h, this, mesh);
                        }
                    }
                }
                k += 1;
                let ui = GRAPHIC_UI.get(h);
                count = h.call_sym(MENU_COUNT, Conv::Thiscall(ui), &[]);
            }
        }
    }

    /// The map objects (`+0x3874`, count `+0x386c`) whose box the camera
    /// `+0x654` sees, drawn with that camera (frame as
    /// [`minimap_stencil`]).
    fn draw_visible_objects(h: &mut impl Host, ebp: u32, this: u32) {
        let mut i = 0u32;
        h.write(ebp - 4, i);
        while i < h.read::<u32>(this + 0x386c) {
            let list: u32 = h.read(this + 0x3874);
            let object: u32 = h.read(list.wrapping_add(4 * i));
            if object != 0 {
                let bounds: u32 = h.read(object + 0x1c);
                let view = h.read::<u32>(this + 0x654).wrapping_add(0x74);
                h.write(ebp - 8, bounds);
                let local: u32 = h.read(bounds + 0xc);
                let matrix = if local == 0 {
                    for k in 0..16 {
                        let v: u32 = h.read(view.wrapping_add(4 * k));
                        h.write(ebp - 0x88 + 4 * k, v);
                    }
                    ebp - 0x88
                } else {
                    h.call_sym(MATRIX_FROM_3X4, Conv::Thiscall(ebp - 0x48), &[local]);
                    h.call_sym(
                        MATRIX_MULTIPLY,
                        Conv::Cdecl,
                        &[ebp - 0xc8, view, ebp - 0x48],
                    )
                };
                let seen = h.call_sym(BOX_IS_VISIBLE, Conv::Thiscall(bounds), &[matrix]) as u8;
                if seen != 0 {
                    let camera: u32 = h.read(this + 0x654);
                    let camera = if camera == 0 {
                        0
                    } else {
                        camera.wrapping_add(4)
                    };
                    let renderer: u32 = h.read(this + 0xd0);
                    h.call_sym(SET_CAMERA, Conv::Thiscall(renderer), &[camera]);
                    let list: u32 = h.read(this + 0x3874);
                    let object: u32 = h.read(list.wrapping_add(4 * i));
                    render_mesh(h, this, object);
                }
            }
            i += 1;
            h.write(ebp - 4, i);
        }
    }

    /// `0x00650340(this)` (`ecx` only; frame: the product at `ebp - 0xc8`,
    /// the camera copy `- 0x88`, the object's 4x4 `- 0x48`, the object's
    /// box `- 8`, the loop index `- 4`). With the scene's UI flag
    /// (`+0x10dcc`) or a movie fading: the UI camera (`+0xa88`) set and
    /// updated; pass "SetStencilFlag": the fade list's (`+0x3868`, count
    /// `+0x3860`) meshes named `g(0x00ccb9e4)`; the minimap viewport into
    /// `+0x10d40..+0x10d4c`; pass "Transparent", the viewport and scissor;
    /// the menus of kind 1 (`[+0x24] + 0xb4`) drawn; pass
    /// `"EnableStencilTest"` (with `+0xf9c6`) or `"Transparent"`: each map
    /// object (`+0x3874`, count `+0x386c`) whose box (`+0x1c`) the camera
    /// `+0x654` (times the box's matrix `+0xc`, when it has one) sees,
    /// drawn with that camera; the scissor back to the target's size.
    /// Returns 0.
    pub fn minimap_stencil(h: &mut impl Host, ebp: u32, this: u32) -> u32 {
        if h.read::<u8>(this + 0x1_0dcc) == 0 && MOVIE_FADING.get(h) == 0 {
            return 0;
        }
        let camera = if this.wrapping_add(0xa84) == 0 {
            0
        } else {
            this.wrapping_add(0xa88)
        };
        let renderer: u32 = h.read(this + 0xd0);
        h.call_sym(SET_CAMERA, Conv::Thiscall(renderer), &[camera]);
        h.call_sym(
            UPDATE_VIEW_MATRICES,
            Conv::Thiscall(this.wrapping_add(0xa88)),
            &[],
        );
        set_pass(h, this, SET_STENCIL_PASS);
        let mut i = 0u32;
        h.write(ebp - 4, i);
        while i < h.read::<u32>(this + 0x3860) {
            let list: u32 = h.read(this + 0x3868);
            let mesh: u32 = h.read(list.wrapping_add(4 * i));
            if mesh != 0 && same_name(h, h.read(mesh + 0x2c), STENCIL_NAME.get(h)) {
                render_mesh(h, this, mesh);
            }
            i += 1;
            h.write(ebp - 4, i);
        }
        for (getter, at) in [
            (MINIMAP_X, 0x1_0d40),
            (MINIMAP_65E4A0, 0x1_0d44),
            (MINIMAP_Y, 0x1_0d48),
            (MINIMAP_SIZE, 0x1_0d4c),
        ] {
            let v = h.call_sym(getter, Conv::Thiscall(this), &[]);
            h.write(this + at, v);
        }
        set_pass(h, this, TRANSPARENT_PASS);
        let (w, ht) = target_size(h, this);
        let renderer: u32 = h.read(this + 0xd0);
        h.call_sym(SET_VIEWPORT, Conv::Thiscall(renderer), &[0, 0, w, ht]);
        let rect: [u32; 4] = core::array::from_fn(|k| h.read(this + 0x1_0d40 + 4 * k as u32));
        let renderer: u32 = h.read(this + 0xd0);
        h.call_sym(SET_SCISSOR, Conv::Thiscall(renderer), &rect);
        draw_kind1_menus(h, this);
        let pass = if h.read::<u8>(this + 0xf9c6) == 0 {
            TRANSPARENT_PASS
        } else {
            STENCIL_TEST_PASS
        };
        set_pass(h, this, pass);
        draw_visible_objects(h, ebp, this);
        let (w, ht) = target_size(h, this);
        let renderer: u32 = h.read(this + 0xd0);
        h.call_sym(SET_SCISSOR, Conv::Thiscall(renderer), &[0, 0, w, ht]);
        0
    }

    /// The `f32` at `sym`, as `fld dword` / `fstp dword` copy it.
    fn constant(h: &impl Host, sym: u32) -> u32 {
        let at = h.rebase(sym);
        x87::store(x87::load(h.read(at)))
    }

    /// `remap(f32 at sym)`, stored as `f32`.
    fn remapped(h: &mut impl Host, remap: u32, sym: u32) -> u32 {
        let v = constant(h, sym);
        x87::store(h.call_sym_st0_f64(remap, Conv::Cdecl, &[v]))
    }

    /// A style-1 bar: shape 200 at x 0, the top `top`, width 1920 and
    /// height 30 remapped (height, width, then top), the UVs as given.
    fn bar(h: &mut impl Host, top: u32, uv: [u32; 4]) -> u32 {
        let ht = remapped(h, UI_REMAP_Y2, BAR_HEIGHT);
        let w = remapped(h, UI_REMAP_X2, BAR_WIDTH);
        let y = remapped(h, UI_REMAP_Y2, top);
        let uv = uv.map(|sym| if sym == 0 { 0 } else { constant(h, sym) });
        h.call_sym(
            MKP_SHAPE_XYWHUV,
            Conv::Cdecl,
            &[200, 0, y, w, ht, uv[0], uv[1], uv[2], uv[3]],
        )
    }

    /// A line: shape -1 at x 210, the top `top`, width 1500 and height 2
    /// remapped (height, width, top, then x).
    fn line(h: &mut impl Host, top: u32) -> u32 {
        let ht = remapped(h, UI_REMAP_Y2, LINE_HEIGHT);
        let w = remapped(h, UI_REMAP_X2, LINE_WIDTH);
        let y = remapped(h, UI_REMAP_Y2, top);
        let x = remapped(h, UI_REMAP_X2, LINE_X);
        let uv = [LINE_U0, LINE_V0, LINE_U1, LINE_V1].map(|sym| constant(h, sym));
        h.call_sym(
            MKP_SHAPE_XYWHUV,
            Conv::Cdecl,
            &[u32::MAX, x, y, w, ht, uv[0], uv[1], uv[2], uv[3]],
        )
    }

    /// `0x008e71d0(style)` (cdecl): a menu window's divider shapes. Style
    /// 1: the top and bottom bars; else a line at 224 (2), 250 (4), 410
    /// (5), 305 and 590 (6) or 305 (7), then the line at 1015 (3 to 7). The original shares one argument block between the calls
    /// (the UV words stay from one call to the next); each call here gets
    /// the words the block holds. Returns the last draw's `eax`, or
    /// `style` (nothing drawn).
    pub fn menu_divider_shapes(h: &mut impl Host, style: u32) -> u32 {
        let first = match style {
            1 => {
                bar(h, BAR_TOP, [BAR_U, BAR_V0, 0, BAR_V1]);
                return bar(h, BAR_BOTTOM, [0, BAR_V1, BAR_U, BAR_V0]);
            }
            2 => Some(LINE_224),
            3 => None,
            4 => Some(LINE_250),
            5 => Some(LINE_410),
            6 => {
                line(h, LINE_305);
                Some(LINE_590)
            }
            7 => Some(LINE_305),
            _ => return style,
        };
        if let Some(top) = first {
            line(h, top);
        }
        line(h, LINE_1015)
    }

    /// The `f64` at `sym`.
    fn factor(h: &impl Host, sym: u32) -> f64 {
        let at = h.rebase(sym);
        f64::from_bits(h.read::<u64>(at))
    }

    /// `f32(v * factor)`.
    fn scaled(h: &impl Host, v: u32, sym: u32) -> u32 {
        x87::store(x87::mul(x87::load(v), factor(h, sym)))
    }

    /// `f32(v * factor + at)`.
    fn shifted(h: &impl Host, v: u32, sym: u32, at: u32) -> u32 {
        x87::store(x87::add(
            x87::mul(x87::load(v), factor(h, sym)),
            x87::load(at),
        ))
    }

    /// `DrawCrossButtonIconRGBA(id, x, y, w, h, c0, c1; slots)` (cdecl,
    /// the floats as bits): icon `id` (600..=0x269) drawn by
    /// `TOMkpShapeXYWHUVC2(600, ...)` with its cell's UVs; the face
    /// buttons' rectangles shrunk ([`Fit`]). The fitted ones leave their
    /// scratch values in `id`'s slot (the height, then the width) and
    /// `y`'s (the new `y`, then the new `x`). Returns the draw's `eax`,
    /// or `id - 600` (another id: nothing drawn).
    #[allow(clippy::too_many_arguments)] // the original's seven and the slots
    pub fn draw_cross_button_icon(
        h: &mut impl Host,
        slots: u32,
        id: u32,
        x: u32,
        y: u32,
        w: u32,
        ht: u32,
        c0: u32,
        c1: u32,
    ) -> u32 {
        let index = id.wrapping_sub(600);
        let Some(&(fit, cell)) = CROSS_ICONS.get(index as usize) else {
            return index;
        };
        let copy = |v: u32| x87::store(x87::load(v));
        let (rx, ry, rw, rh) = match fit {
            Fit::Plain => (copy(x), copy(y), copy(w), copy(ht)),
            Fit::A => {
                let rh = scaled(h, ht, F_045);
                let rw = scaled(h, w, F_039);
                let rx = shifted(h, w, F_0305, x);
                h.write(slots + 8, rx);
                (rx, copy(y), rw, rh)
            }
            Fit::B | Fit::C => {
                let (fh, fw, fy, fx) = if fit == Fit::B {
                    (F_039, F_045, F_0305, F_055)
                } else {
                    (F_045, F_039, F_05, F_0305)
                };
                let rh = scaled(h, ht, fh);
                let rw = scaled(h, w, fw);
                let ry = shifted(h, ht, fy, y);
                let rx = shifted(h, w, fx, x);
                h.write(slots + 8, rx);
                (rx, ry, rw, rh)
            }
            Fit::D => {
                let rh = scaled(h, ht, F_039);
                let rw = scaled(h, w, F_045);
                let ry = shifted(h, ht, F_0305, y);
                h.write(slots + 8, ry);
                (copy(x), ry, rw, rh)
            }
        };
        if fit != Fit::Plain {
            h.write(slots, rw);
        }
        let uv = cell.map(|sym| if sym == 0 { 0 } else { constant(h, sym) });
        h.call_sym(
            MKP_SHAPE_XYWHUV_C2,
            Conv::Cdecl,
            &[600, rx, ry, rw, rh, uv[0], uv[1], uv[2], uv[3], c0, c1],
        )
    }
}
