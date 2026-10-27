//! The render-side glue, sixty-third batch (M4 port round 18, list
//! `h3-glue-render`): the scene's HDAO depth pass, drawn from an
//! orthographic camera built in the frame. The exception frame and cookie
//! are left out, as in [`crate::glue10`]; the surfaces and the camera are
//! destroyed as the original's unwinding does.

pub use crate::h3_glue51::{
    BEGIN_SCENE, END_SCENE, SET_CAMERA, SET_CLEAR_COLOR, SET_COLOR_TARGET,
    SET_DEPTH_STENCIL_TARGET, SET_VIEWPORT, SURFACE_CTOR, SURFACE_DTOR, UPDATE_VIEW_MATRICES,
};
pub use crate::h3_glue53::{OPAQUE_NAME, TRANSPARENT_NAME};

/// The camera: `PObject_InitDefaults_004530a0(camera + 4)`,
/// `Phyre_PCameraOrthographic_vftable`,
/// `Phyre_PCameraOrthographic_updateProjection(camera)` (thiscall), and
/// the `Phyre_PCameraProjection_vftable` its destructor puts back.
pub const INIT_DEFAULTS: u32 = 0x0045_30a0;
pub const ORTHO_VTABLE: u32 = 0x00b1_1040;
pub const ORTHO_UPDATE: u32 = 0x0045_5fe0;
pub const PROJECTION_VTABLE: u32 = 0x00b1_0f5c;
/// The matrices: `PMatrix4_Copy_004c9430(out; m)` (`ret 4`),
/// `PMatrix4_MulThis_004c9da0(a; out, b)` (`ret 8`),
/// `Matrix_RotationY(out, angle)` and `PMatrix4_Multiply_004ca360(out, a,
/// b)` (cdecl), `Matrix3x4_Identity_00450ee0(m)` (thiscall),
/// `PMatrix4_toWorldMatrix3x4(dst; m)` (`ret 4`) and
/// `PMatrix4_orthographic_00455dc0(out, left, right, bottom, top, near,
/// far)` (cdecl).
pub const COPY: u32 = 0x004c_9430;
pub const MUL_THIS: u32 = 0x004c_9da0;
pub const ROTATION_Y: u32 = 0x0066_c920;
pub const MULTIPLY: u32 = 0x004c_a360;
pub const IDENTITY_3X4: u32 = 0x0045_0ee0;
pub const TO_WORLD_3X4: u32 = 0x0045_1190;
pub const ORTHOGRAPHIC: u32 = 0x0045_5dc0;
/// The image's constants: -1.0 (f32, `g_ChrClothFlip`), pi/2 (f32), 10.0
/// (f64), 40.0 (f32) and 100000.0 (f32).
pub const MINUS_ONE: u32 = 0x00b9_2340;
pub const HALF_PI: u32 = 0x00b4_3d38;
pub const TEN: u32 = 0x00b9_22b0;
pub const FORTY: u32 = 0x00b4_3db4;
pub const FAR: u32 = 0x00b1_0f50;
/// The map tree's walk: `PRedBlackTree_iteratorBegin_0064dae0(it;
/// tree)` (`ret 4`), `Phyre_PRedBlackTree_successor(tree; node)` (`ret
/// 4`), `PArray_Data_004d45a0(array)`, `ShaderParams_FindByName(shader;
/// name)` (`ret 4`), `memcpy` (cdecl) and `"clrScaleFactor"`.
pub const TREE_ITERATOR_BEGIN: u32 = crate::glue11::TREE_ITERATOR_BEGIN;
pub const TREE_SUCCESSOR: u32 = crate::h3_glue25::TREE_SUCCESSOR;
pub const ARRAY_DATA: u32 = crate::h3_glue25::ARRAY_DATA;
pub const FIND_PARAMETER: u32 = crate::h3_glue25::FIND_PARAMETER;
pub const MEMCPY: u32 = crate::h3_glue25::MEMCPY;
pub const CLR_SCALE_FACTOR: u32 = crate::h3_glue25::CLR_SCALE_FACTOR;
/// `setClearDepth(renderer; depth)` (`ret 4`).
pub const SET_CLEAR_DEPTH: u32 = 0x005b_af10;
/// `PApplication::GetApplication()` (cdecl) and `renderWorld(app; world,
/// camera, flags)` (`ret 0xc`).
pub const GET_APPLICATION: u32 = crate::h3_glue54::GET_APPLICATION;
pub const RENDER_WORLD: u32 = 0x0062_be70;
/// `Phyre_PhyreFree(p)` (cdecl).
pub const PHYRE_FREE: u32 = crate::glue4::PHYRE_FREE;

/// A material array's count.
const COUNT_MASK: u32 = 0x7fff_ffff;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// The frame (`ebp -` these): the colour and depth surfaces, the
    /// camera object (its `PCamera` at `+4`), its world matrix (3x4), the
    /// two matrices it is built from, the products' outputs, the tree
    /// iterator and the colour `{1, 1, 1, 1}`.
    const COLOR: u32 = 0xd0;
    const DEPTH: u32 = 0xe0;
    const CAMERA: u32 = 0x238;
    const WORLD: u32 = 0x268;
    const MATRIX: u32 = 0x60;
    const VIEW: u32 = 0xac;
    const PRODUCT: u32 = 0x2a8;
    const ROTATED: u32 = 0x2e8;
    const ITERATOR: u32 = 0x6c;
    const ONES: u32 = 0xc0;

    const ONE: u32 = 0x3f80_0000;

    /// A word through the x87 (`fld dword` / `fstp dword`).
    fn through(w: u32) -> u32 {
        x87::store(x87::load(w))
    }

    fn renderer(h: &impl Host, scene: u32) -> u32 {
        h.read(scene + 0xd0)
    }

    /// `PostProcessManager_renderHDAO(scene)` (thiscall, plain `ret`;
    /// frame from `ebp - 0x2e8`) -> 0. Only with the bytes `+0xfc00` and
    /// `+0xfc01` set and a colour target `+0xfc70`: the targets `+0xfc70`
    /// and `+0xfc74` in surfaces, the camera built ([`build_camera`]),
    /// with a map cluster (`+0x10d4`) every map material's
    /// `"clrScaleFactor"` set to 1s; the depth cleared to 1, the colour to
    /// (0, 0, 0, 1) with flags 0xb, the worlds `+0xac` (opaque, then
    /// transparent) and with `+0xf9cc` 1 or 3 `+0xbc` drawn from the
    /// camera; the scene's camera back, the factors set to the scene's
    /// colour `+0xfc78`, and `+0xfc01` cleared.
    pub fn render_hdao(h: &mut impl Host, ebp: u32, scene: u32) -> u32 {
        if h.read::<u8>(scene + 0xfc00) == 0 || h.read::<u8>(scene + 0xfc01) == 0 {
            return 0;
        }
        let color: u32 = h.read(scene + 0xfc70);
        if color == 0 {
            return 0;
        }
        h.call_sym(SURFACE_CTOR, Conv::Thiscall(ebp - COLOR), &[color, 0]);
        let depth: u32 = h.read(scene + 0xfc74);
        h.call_sym(SURFACE_CTOR, Conv::Thiscall(ebp - DEPTH), &[depth, 0]);
        build_camera(h, ebp, scene);
        if h.read::<u32>(scene + 0x10d4) != 0 {
            walk_materials(h, ebp, scene, Some(ebp - ONES), true);
        }
        draw(h, ebp, scene);
        if h.read::<u32>(scene + 0x10d4) != 0 {
            let colour = scene.wrapping_add(0xfc78);
            let src = if colour == 0 { None } else { Some(colour) };
            walk_materials(h, ebp, scene, src, false);
        }
        h.write(scene + 0xfc01, 0_u8);
        // the camera's destructor
        let camera = ebp - CAMERA;
        h.write(camera, h.rebase(PROJECTION_VTABLE));
        let owned: u32 = h.read(camera + 0xd0);
        if owned & 1 == 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[owned]);
        }
        h.call_sym(SURFACE_DTOR, Conv::Thiscall(ebp - DEPTH), &[]);
        h.call_sym(SURFACE_DTOR, Conv::Thiscall(ebp - COLOR), &[]);
        0
    }

    /// The orthographic camera at `ebp - 0x238`: its defaults, aspect and
    /// `+0x154` 1, its projection updated. Its view: the matrix
    /// `+0xfc04` with the last row (0, 0, 0, 1), times the matrix whose
    /// rows are (1, 0, -1, 0), (0, -1, 0, 1) (the rest 0; `ebp - 0xac`),
    /// then rotated a quarter turn about y; its position (`+0xfc34` x 10,
    /// 40, `-+0xfc3c` x 10) after the 0xc04/0xb48 swap of `+0xfc34`,
    /// `+0xfc38` and `+0xfc3c`; the world matrix `ebp - 0x268` built from
    /// it. The projection: orthographic over `+0xfc44`, `+0xfc54`,
    /// `+0xfc4c`, `+0xfc5c` (x 10), near 1, far 100000, then the view
    /// matrices updated.
    fn build_camera(h: &mut impl Host, ebp: u32, scene: u32) {
        let camera = ebp - CAMERA;
        h.call_sym(INIT_DEFAULTS, Conv::Thiscall(camera + 4), &[]);
        h.write(camera + 0x150, ONE);
        h.write(camera + 0x154, ONE);
        h.write(camera, h.rebase(ORTHO_VTABLE));
        h.call_sym(ORTHO_UPDATE, Conv::Thiscall(camera), &[]);
        let matrix = ebp - MATRIX;
        let view = ebp - VIEW;
        for k in 0..16 {
            h.write(matrix + 4 * k, 0_u32);
        }
        h.call_sym(COPY, Conv::Thiscall(view), &[matrix]);
        let minus_one = through(h.read(h.rebase(MINUS_ONE)));
        h.write(view + 8, minus_one);
        h.write(view + 0x10, minus_one);
        h.write(view + 0x24, ONE);
        for k in 0..16 {
            let w: u32 = h.read(scene + 0xfc04 + 4 * k);
            h.write(matrix + 4 * k, w);
        }
        h.write(view + 0x3c, ONE);
        for k in 12..15 {
            h.write(matrix + 4 * k, 0_u32);
        }
        h.write(matrix + 0x3c, ONE);
        let product = h.call_sym(MUL_THIS, Conv::Thiscall(matrix), &[ebp - PRODUCT, view]);
        for k in 0..16 {
            let w: u32 = h.read(product.wrapping_add(4 * k));
            h.write(view + 4 * k, through(w));
        }
        let angle = through(h.read(h.rebase(HALF_PI)));
        let rotation = h.call_sym(ROTATION_Y, Conv::Cdecl, &[ebp - PRODUCT, angle]);
        let rotated = h.call_sym(MULTIPLY, Conv::Cdecl, &[ebp - ROTATED, rotation, view]);
        h.call_sym(COPY, Conv::Thiscall(matrix), &[rotated]);
        for k in 0..12 {
            let w: u32 = h.read(matrix + 4 * k);
            h.write(view + 4 * k, through(w));
        }
        let w: u32 = h.read(matrix + 0x3c);
        h.write(view + 0x3c, through(w));
        for code in [0xc04, 0xb48] {
            if h.read::<u32>(scene + 0xfaf4) == code {
                let w = through(h.read(scene + 0xfc34));
                h.write(scene + 0xfc3c, w);
                let w = through(h.read(scene + 0xfc38));
                h.write(scene + 0xfc34, w);
            }
        }
        let ten = f64::from_bits(h.read::<u64>(h.rebase(TEN)));
        let x = x87::store(x87::mul(x87::load(h.read(scene + 0xfc34)), ten));
        let z = x87::store(x87::mul(-x87::load(h.read(scene + 0xfc3c)), ten));
        h.write(view + 0x30, x);
        h.write(view + 0x34, through(h.read(h.rebase(FORTY))));
        h.write(view + 0x38, z);
        let world = ebp - WORLD;
        h.call_sym(IDENTITY_3X4, Conv::Thiscall(world), &[]);
        h.call_sym(TO_WORLD_3X4, Conv::Thiscall(world), &[view]);
        let far = through(h.read(h.rebase(FAR)));
        h.write(camera + 0xb4, world);
        let scaled = |v: u32| x87::store(x87::mul(x87::load(v), ten));
        let top = scaled(h.read(scene + 0xfc5c));
        let bottom = scaled(h.read(scene + 0xfc4c));
        let right = scaled(h.read(scene + 0xfc54));
        let left = scaled(h.read(scene + 0xfc44));
        h.call_sym(
            ORTHOGRAPHIC,
            Conv::Cdecl,
            &[matrix, left, right, bottom, top, ONE, far],
        );
        for k in 0..16 {
            let w: u32 = h.read(matrix + 4 * k);
            h.write(camera + 0x34 + 4 * k, through(w));
        }
        h.call_sym(UPDATE_VIEW_MATRICES, Conv::Thiscall(camera + 4), &[]);
    }

    /// Every material of the map tree (`+0x1198`; the iterator at `ebp -
    /// 0x6c`; with `ones` the colour at `ebp - 0xc0` set to 1s after it) whose `"clrScaleFactor"` fits
    /// its buffer (offset 4 on) takes `src`'s bytes (none: skipped).
    fn walk_materials(h: &mut impl Host, ebp: u32, scene: u32, src: Option<u32>, ones: bool) {
        let it = ebp - ITERATOR;
        h.call_sym(TREE_ITERATOR_BEGIN, Conv::Thiscall(it), &[scene + 0x1198]);
        if ones {
            for k in 0..4 {
                h.write(ebp - ONES + 4 * k, ONE);
            }
        }
        let tree: u32 = h.read(it);
        let mut node: u32 = h.read(it + 4);
        let mut next: u32 = h.read(it + 8);
        if node == tree {
            return;
        }
        loop {
            let list: u32 = h.read(node.wrapping_add(0x10));
            let mut k = 0u32;
            while k < h.read::<u32>(list) {
                let items: u32 = h.read(list.wrapping_add(8));
                let item: u32 = h.read(items.wrapping_add(4 * k));
                let array: u32 = h.read(item.wrapping_add(0x10));
                let mut j = 0u32;
                while j < h.read::<u32>(array) & COUNT_MASK {
                    let data = h.call_sym(ARRAY_DATA, Conv::Thiscall(array), &[]);
                    let material: u32 = h.read(data.wrapping_add(4 * j));
                    let shader: u32 = h.read(material);
                    let name = h.rebase(CLR_SCALE_FACTOR);
                    let parameter = h.call_sym(FIND_PARAMETER, Conv::Thiscall(shader), &[name]);
                    if let (true, Some(src)) = (parameter != 0, src) {
                        let buffer: u32 = h.read(material.wrapping_add(4));
                        let offset = u32::from(h.read::<u16>(parameter.wrapping_add(8)));
                        let size = u32::from(h.read::<u16>(parameter.wrapping_add(0xa)) & 0x1fff);
                        if offset + size <= h.read::<u32>(buffer) && offset >= 4 {
                            h.call_sym(
                                MEMCPY,
                                Conv::Cdecl,
                                &[buffer.wrapping_add(offset), src, size],
                            );
                        }
                    }
                    j += 1;
                }
                k += 1;
            }
            node = next;
            if next != tree {
                next = h.call_sym(TREE_SUCCESSOR, Conv::Thiscall(tree), &[next]);
            }
            if node == tree {
                return;
            }
        }
    }

    /// The depth pass from the camera.
    fn draw(h: &mut impl Host, ebp: u32, scene: u32) {
        let camera = ebp - CAMERA + 4;
        h.call_sym(SET_CLEAR_DEPTH, Conv::Thiscall(renderer(h, scene)), &[ONE]);
        h.call_sym(SET_CAMERA, Conv::Thiscall(renderer(h, scene)), &[camera]);
        let slot: u32 = h.read(ebp - COLOR);
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
        h.call_sym(
            SET_COLOR_TARGET,
            Conv::Thiscall(renderer(h, scene)),
            &[ebp - COLOR, 0],
        );
        h.call_sym(
            SET_DEPTH_STENCIL_TARGET,
            Conv::Thiscall(renderer(h, scene)),
            &[0],
        );
        h.call_sym(
            SET_CLEAR_COLOR,
            Conv::Thiscall(renderer(h, scene)),
            &[0, 0, 0, ONE],
        );
        h.call_sym(BEGIN_SCENE, Conv::Thiscall(renderer(h, scene)), &[0xb]);
        let opaque = h.rebase(OPAQUE_NAME);
        h.write(renderer(h, scene).wrapping_add(0x48), opaque);
        render_world(h, scene + 0xac, camera, 1);
        let transparent = h.rebase(TRANSPARENT_NAME);
        h.write(renderer(h, scene).wrapping_add(0x48), transparent);
        render_world(h, scene + 0xac, camera, u32::MAX);
        let kind: u32 = h.read(scene + 0xf9cc);
        if kind == 1 || kind == 3 {
            render_world(h, scene + 0xbc, camera, u32::MAX);
        }
        h.call_sym(END_SCENE, Conv::Thiscall(renderer(h, scene)), &[1]);
        let own: u32 = h.read(scene + 0xdc);
        let own = if own == 0 { 0 } else { own.wrapping_add(4) };
        h.call_sym(SET_CAMERA, Conv::Thiscall(renderer(h, scene)), &[own]);
    }

    fn render_world(h: &mut impl Host, world: u32, camera: u32, flags: u32) {
        let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        h.call_sym(RENDER_WORLD, Conv::Thiscall(app), &[world, camera, flags]);
    }
}
