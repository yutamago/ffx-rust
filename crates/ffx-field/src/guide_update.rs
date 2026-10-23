//! `rcbgGuideUpdateEx` (`0x0091da80`): the field minimap's frame — the
//! fade, the camera, the background box and grid, the fog of war, the
//! mesh, and the markers (`docs/formats/field.md`, "Guide map").
//!
//! The function keeps every vector and matrix it hands to a callee in its
//! frame; [`rcbg_guide_update_ex`] uses a work area laid out the same way
//! (`work + 0x360 - k` is the original's `[ebp - k]`), so the callees see
//! the same relative layout.

use ffx_bridge::Mem;
use ffx_bridge::x87::{Reg, ftol};

use crate::camera::FArgs;

/// The work area [`rcbg_guide_update_ex`] needs, in bytes (the
/// original's frame below the cookie).
pub const WORK: u32 = 0x360;

/// The callees, by address.
pub mod callee {
    /// `_CItan` (`ST0` in, `ST0` out).
    pub const CI_TAN: u32 = 0x0094_97be;
    /// `FFXVu0UnitMatrix(m)`.
    pub const UNIT_MATRIX: u32 = 0x006e_d9a0;
    /// `graphicIs3DScene()`.
    pub const IS_3D_SCENE: u32 = 0x0064_1b90;
    /// `Guide_MeshObject(guide)`.
    pub const MESH_OBJECT: u32 = 0x0091_d460;
    /// `Guide_FadeOff()`.
    pub const FADE_OFF: u32 = 0x0091_d3a0;
    /// `Guide_FadeStep(show)`.
    pub const FADE_STEP: u32 = 0x0091_d280;
    /// `graphicMinimapSetEnable(alpha)`.
    pub const MINIMAP_SET_ENABLE: u32 = 0x0064_25f0;
    /// `AtelGetGuideMapAngEx(i)` (`ST0`).
    pub const GUIDE_MAP_ANG: u32 = 0x0086_b9a0;
    /// `rcbgGuideGetScreen(guide, scale, offset)`.
    pub const GET_SCREEN: u32 = 0x0091_da30;
    /// `rcCopyQword_0091cb40(dst, src)`.
    pub const COPY_QWORD_CB40: u32 = 0x0091_cb40;
    /// `AtelGetGideMapActorNumber()`.
    pub const ACTOR_NUMBER: u32 = 0x0086_b710;
    /// `AtelGetGideMapActorInfo(i, out)`.
    pub const ACTOR_INFO: u32 = 0x0086_b700;
    /// `rcbgScaleVector4_0091cb70(dst, v, s)`.
    pub const SCALE_VECTOR4: u32 = 0x0091_cb70;
    /// `rcbgSetViewMatrix(out, eye, at, up, roll)`.
    pub const SET_VIEW_MATRIX: u32 = 0x0092_04b0;
    /// `graphicMinimapCameraSetWorldMatrix(view)`.
    pub const CAMERA_SET_WORLD: u32 = 0x0064_2410;
    /// `sceVu0InversMatrix4x4(dst, m)`.
    pub const INVERS_MATRIX: u32 = 0x006e_d460;
    /// `rcbgSetProjectionMatrixSub(out, fov, dx, dy, _, distance)` (`ST0`).
    pub const PROJECTION_SUB: u32 = 0x0092_02e0;
    /// `rcbgScaleVector_0091cae0(dst, v, s)`.
    pub const SCALE_VECTOR: u32 = 0x0091_cae0;
    /// `rcMat4fSetTranslate(m, x, y, z)`.
    pub const SET_TRANSLATE: u32 = 0x0092_5850;
    /// `rcMat4fSetTranslateVec3f(m, v)`.
    pub const SET_TRANSLATE_VEC: u32 = 0x0092_5870;
    /// `MsRotMatrixY(out, m, angle)`.
    pub const ROT_MATRIX_Y: u32 = 0x007c_3480;
    /// `FFXVu0MulMatrix(dst, a, b)`.
    pub const MUL_MATRIX: u32 = 0x006e_d5c0;
    /// `rcbgSetReg(list)`: GS registers, pairs of words up to `-1, -1`.
    pub const SET_REG: u32 = 0x0092_0450;
    /// `rcPrimBox(x, y, w, h, colour, ui)`.
    pub const PRIM_BOX: u32 = 0x0092_5b60;
    /// `FFXVu0ApplyMatrix(dst, m, v)`.
    pub const APPLY_MATRIX: u32 = 0x006e_d2e0;
    /// `FFXVu0DivVectorXYZ(dst, v, s)`.
    pub const DIV_VECTOR_XYZ: u32 = 0x006e_d420;
    /// `rcPrimLine(x0, y0, x1, y1, colour, ui)`.
    pub const PRIM_LINE: u32 = 0x0092_6c00;
    /// `graphicMinimapFogSetEnable(on)`.
    pub const FOG_SET_ENABLE: u32 = 0x0064_25d0;
    /// `rcbgGuideUpdateWalk(guide, m, rect)`.
    pub const UPDATE_WALK: u32 = 0x0091_ecb0;
    /// `rcbgModelDraw(model)`.
    pub const MODEL_DRAW: u32 = 0x0092_8020;
    /// `getScenerioFlag()` (`u16`).
    pub const SCENARIO_FLAG: u32 = 0x0078_7360;
    /// `rcCopyQword_0091cbd0(dst, src)`.
    pub const COPY_QWORD_CBD0: u32 = 0x0091_cbd0;
    /// `rcbgGuideDrawPlayer(guide, a, b, rect, _, alpha, p, q, dir, smooth)`.
    pub const DRAW_PLAYER: u32 = 0x0091_cf10;
    /// `rcPrimTri(x, y, r, angle, colour, ui)`.
    pub const PRIM_TRI: u32 = 0x0092_70c0;
    /// `rcPrimCircleLine(x, y, r, colour, ui)`.
    pub const PRIM_CIRCLE_LINE: u32 = 0x0092_5ea0;
    /// `Guide_RevealFog(guide, x, z)`.
    pub const REVEAL_FOG: u32 = 0x0091_f420;
    /// `AtelGetActorType(id)`.
    pub const ACTOR_TYPE: u32 = 0x0086_a7b0;
    /// `AtelGetActorStartPos(id, x, y, z)`.
    pub const ACTOR_START_POS: u32 = 0x0086_a720;
    /// `AtelGetActorEndPos(id, x, y, z)`.
    pub const ACTOR_END_POS: u32 = 0x0086_a390;
    /// `rcPrimBatu(x, y, size, colour, ui)`.
    pub const PRIM_BATU: u32 = 0x0092_59c0;
    /// `rcbgRestoreReg()`.
    pub const RESTORE_REG: u32 = 0x0092_0250;
    /// `CloseScissor()`.
    pub const CLOSE_SCISSOR: u32 = 0x008e_5cd0;
}

/// The globals it reads and writes.
pub mod global {
    /// `g_ScreenDataAspectZ` (`f32`).
    pub const ASPECT_Z: u32 = 0x0113_701c;
    /// `g_GraphicMapId`.
    pub const GRAPHIC_MAP_ID: u32 = 0x0133_0844;
    /// `g_CurrentMapNo`.
    pub const CURRENT_MAP_NO: u32 = 0x012f_bc40;
    /// `g_GuideEnabled`.
    pub const ENABLED: u32 = 0x00c5_e478;
    /// `g_GuideFlags`: bit 0 the minimap may show, bit 8 the mesh is drawn.
    pub const FLAGS: u32 = 0x00c5_e350;
    /// `g_GuideFade` (0..=128).
    pub const FADE: u32 = 0x0193_bf78;
    /// `g_YnMapSystem`.
    pub const YN_MAP_SYSTEM: u32 = 0x0193_bf80;
    /// Where the minimap camera's inverse goes (`0x0230fce0`, 64 bytes).
    pub const CAMERA_INVERSE: u32 = 0x0230_fce0;
    /// `g_GuideMarkerPalette`: 16 RGBA float colours (128 = 1).
    pub const PALETTE: u32 = 0x00c5_e370;
    /// The player triangle's colour (`0x00c5e3a0`).
    pub const PLAYER_COLOUR: u32 = 0x00c5_e3a0;
    /// The player circle's colour (`0x00c5e3c0`).
    pub const CIRCLE_COLOUR: u32 = 0x00c5_e3c0;
}

/// Its `.rdata` constants (`f64` unless noted).
pub mod k {
    /// π/8, the tangent's argument.
    pub const PI_8: u32 = 0x00b6_7658;
    /// 724.07 (`f32`): map 0xec's depth.
    pub const DEPTH_EC: u32 = 0x00b6_76c8;
    /// 579.4 (`f32`): map 0x142's depth.
    pub const DEPTH_142: u32 = 0x00b6_76c4;
    /// 645.12.
    pub const K645: u32 = 0x00b6_76b8;
    /// 1.1666666.
    pub const SEVEN_SIXTHS: u32 = 0x00b6_7698;
    /// 416.
    pub const K416: u32 = 0x00b4_1d30;
    /// 448.
    pub const K448: u32 = 0x00b5_0210;
    /// 1.33.
    pub const K1_33: u32 = 0x00b6_76a0;
    /// 0.865.
    pub const K0_865: u32 = 0x00b6_7668;
    /// 0.8.
    pub const K0_8: u32 = 0x00b3_eb68;
    /// 1.1.
    pub const K1_1: u32 = 0x00b6_7688;
    /// 1.165.
    pub const K1_165: u32 = 0x00b6_7690;
    /// 0.969999.
    pub const K0_97: u32 = 0x00b6_7670;
    /// 1.039999.
    pub const K1_04: u32 = 0x00b6_7680;
    /// 0.9828.
    pub const K0_9828: u32 = 0x00b6_7678;
    /// 1/128.
    pub const K1_128: u32 = 0x00b1_70b8;
    /// 4.
    pub const K4: u32 = 0x00b4_3d80;
    /// 45 (`f32`): the field of view.
    pub const K45F: u32 = 0x00b5_ee90;
    /// 10 (`f32`).
    pub const K10F: u32 = 0x00b9_2318;
    /// 256.
    pub const K256: u32 = 0x00b9_22f0;
    /// 0.8125.
    pub const K0_8125: u32 = 0x00b6_7660;
    /// 0.5.
    pub const HALF: u32 = 0x00b9_2220;
    /// 208.
    pub const K208: u32 = 0x00b5_4c70;
    /// 30 (`f32`).
    pub const K30F: u32 = 0x00b4_1d80;
    /// 20 (`f32`).
    pub const K20F: u32 = 0x00b4_ae10;
    /// 80.
    pub const K80: u32 = 0x00b5_f200;
    /// 2048.
    pub const K2048: u32 = 0x00b5_1bc0;
    /// 60 (`f32`).
    pub const K60F: u32 = 0x00b3_eb9c;
    /// 100 (`f32`).
    pub const K100F: u32 = 0x00b3_04f8;
    /// 80 (`f32`).
    pub const K80F: u32 = 0x00b5_67a8;
    /// 48.
    pub const K48: u32 = 0x00b5_f1e0;
    /// 1/32.
    pub const K1_32: u32 = 0x00b5_01a0;
    /// 32.
    pub const K32: u32 = 0x00b4_12f8;
    /// 34.
    pub const K34: u32 = 0x00b6_76a8;
    /// 128.
    pub const K128: u32 = 0x00b4_c170;
    /// 15.
    pub const K15: u32 = 0x00b9_22c0;
    /// 6 (`f32`).
    pub const K6F: u32 = 0x00b9_2308;
    /// 6.5 (`f32`).
    pub const K6_5F: u32 = 0x00b6_76c0;
    /// 8 (`f32`).
    pub const K8F: u32 = 0x00b9_2314;
}

/// The maps whose 2D minimap is narrowed (x 0.8, y 0.865).
const NARROW_MAPS: [u32; 6] = [0x131, 0xb4, 0xb6, 0x100, 0xb5, 0xf3];

/// Marker flags (`AtelGetGideMapActorInfo`'s `+4`).
mod flag {
    pub const SHAPE: u32 = 0xf_0300;
    pub const SQUARE: u32 = 0x100;
    pub const TRIANGLE: u32 = 0x200;
    pub const RING: u32 = 0x400;
    pub const PLAYER: u32 = 0x800;
    pub const BLINK: u32 = 0x1000;
    pub const DESTINATION: u32 = 0x2000;
    pub const EXIT_LINE: u32 = 0x1_0000;
    pub const DIAMOND: u32 = 0x2_0000;
}

/// The scissor of the whole screen (`rcbgSetReg`'s SCISSOR words).
const FULL_SCISSOR: [u32; 2] = [0x01ff_0000, 0x019f_0000];

/// `rcbgGuideUpdateEx(guide, show)` (`0x0091da80`, was `Guide_Update`):
/// one frame of the field minimap. Returns the original's `EAX`: the map
/// id when the guide is disabled, 0 without a mesh, else the last call's.
///
/// The scale `S` (x = 645.12 / (2 · aspect · tan π/8), or the fixed
/// depths of maps 0xec and 0x142; y the x · 7/6 · 416/448, · 1.33 · 416/448
/// more in a 2D scene; per-map factors) is built first. With the guide
/// enabled and its mesh (`Guide_MeshObject`): flag bit 0 clear hides it
/// (`Guide_FadeOff`); else `Guide_FadeStep(show)`,
/// `graphicMinimapSetEnable(fade)`, and with a fade: the mesh's draw
/// state, the camera above `guide +0x50` (lowered by `+0x60 · 4 / scale`,
/// clamped to `-(+0x6c)`, then looking at the player marker), the view,
/// projection and model matrices (mesh `+0x100`, guide `+0x10`), the
/// scissor of the minimap's rect, the background box and the 32 × 34
/// grid that follows the map origin, the fog (`rcbgGuideUpdateWalk`), the
/// mesh (flag bit 8), the markers (palette colours, blinking, squares,
/// triangles, diamonds, exit lines, rings), the destinations
/// (`rcbgGuideDrawPlayer` from the player's position), the player's
/// triangle and circle and the fog revealed around it; the guide's frame
/// counter `+0x74` steps.
///
/// A destination drawn when no marker is the player hands
/// `rcbgGuideDrawPlayer` the player slot (`work + 0`) unwritten: the
/// original's frame holds whatever an earlier call left there.
#[allow(clippy::too_many_lines)] // the original's steps in turn
pub fn rcbg_guide_update_ex<M, C, F>(
    mem: &mut M,
    [g, show]: [u32; 2],
    work: u32,
    call: &mut C,
    fcall: &mut F,
) -> u32
where
    M: Mem,
    C: FnMut(&mut M, u32, &[u32]) -> u32,
    F: FnMut(&mut M, u32, FArgs<'_>) -> f64,
{
    use callee as c;
    // `[ebp - o]` of the original's frame
    let fr = |o: u32| work.wrapping_add(WORK - o);
    let rd = |mem: &M, at: u32| mem.read::<u32>(at);
    let ld = |bits: u32| Reg::load(f32::from_bits(bits));
    let st = |r: Reg| r.store().to_bits();
    let kd = |mem: &M, at: u32| Reg::constant(mem.read::<f64>(mem.rebase(at)));
    let kf = |mem: &M, at: u32| mem.read::<u32>(mem.rebase(at));
    let gl = |mem: &M, at: u32| mem.read::<u32>(mem.rebase(at));
    let int = |v: u32| st(Reg::constant(f64::from(v.cast_signed())));
    let (player, rect, off, scale) = (fr(0x360), fr(0x350), fr(0x340), fr(0x330));
    let info = fr(0x2f0);
    let (proj, t2, t1, r, view, m) = (
        fr(0x2c0),
        fr(0x280),
        fr(0x240),
        fr(0x200),
        fr(0x1c0),
        fr(0x160),
    );
    let (q, p, up, sc, s) = (fr(0x180), fr(0x170), fr(0x118), fr(0x108), fr(0xf8));
    let (regs, mask_regs, at, scissor) = (fr(0xb8), fr(0x98), fr(0x78), fr(0x68));
    let (line_a, line_b, col, v) = (fr(0x44), fr(0x34), fr(0x24), fr(0x14));
    let put = |mem: &mut M, at: u32, words: &[u32]| {
        for (k, &w) in (0u32..).zip(words) {
            mem.write(at.wrapping_add(4 * k), w);
        }
    };
    // the register lists: test, frame, alpha, then the fog's and the scissor's
    put(
        mem,
        regs,
        &[
            0,
            0,
            0x47,
            0,
            0,
            0,
            0x4e,
            0,
            0,
            0,
            0x42,
            0,
            0,
            0,
            u32::MAX,
            u32::MAX,
        ],
    );
    put(mem, scissor, &[0, 0, 0x40, 0, 0, 0, u32::MAX, u32::MAX]);
    let tan = fcall(
        mem,
        mem.rebase(c::CI_TAN),
        FArgs::St0(kd(mem, k::PI_8).value()),
    );
    let tan = Reg::constant(tan).store();
    let map_id = gl(mem, global::GRAPHIC_MAP_ID);
    let aspect = ld(gl(mem, global::ASPECT_Z));
    let mut depth = st(Reg::load(tan) * (aspect + aspect));
    if map_id == 0xec {
        depth = x87_copy(kf(mem, k::DEPTH_EC));
    } else if map_id == 0x142 {
        depth = x87_copy(kf(mem, k::DEPTH_142));
    }
    let sx = st(kd(mem, k::K645) / ld(depth));
    let sy = st(ld(sx) * kd(mem, k::SEVEN_SIXTHS) * kd(mem, k::K416) / kd(mem, k::K448));
    if gl(mem, global::ENABLED) == 0 {
        return map_id;
    }
    call(mem, mem.rebase(c::UNIT_MATRIX), &[s]);
    mem.write(s, sx);
    let s5 = s.wrapping_add(0x14);
    let sy = if call(mem, mem.rebase(c::IS_3D_SCENE), &[]) == 0 {
        st(ld(sy) * kd(mem, k::K1_33) * kd(mem, k::K416) / kd(mem, k::K448))
    } else {
        sy
    };
    mem.write(s5, sy);
    let scale_s = |mem: &mut M, kx: Reg, ky: Reg| {
        let x = st(ld(rd(mem, s)) * kx);
        mem.write(s, x);
        let y = st(ld(rd(mem, s5)) * ky);
        mem.write(s5, y);
    };
    if call(mem, mem.rebase(c::IS_3D_SCENE), &[]) == 0 {
        let narrow = kd(mem, k::K0_865);
        if NARROW_MAPS.contains(&map_id) {
            scale_s(mem, kd(mem, k::K0_8), narrow);
        }
        match map_id {
            0xab => scale_s(mem, narrow, narrow),
            0x9e => scale_s(mem, kd(mem, k::K1_1), kd(mem, k::K1_165)),
            0x114 | 0x142 => scale_s(mem, kd(mem, k::K0_97), kd(mem, k::K1_04)),
            _ => {}
        }
    } else {
        let f = kd(mem, k::K0_9828);
        scale_s(mem, f, f);
    }
    let mesh = call(mem, mem.rebase(c::MESH_OBJECT), &[g]);
    if mesh == 0 {
        return 0;
    }
    if gl(mem, global::FLAGS) & 1 == 0 {
        return call(mem, mem.rebase(c::FADE_OFF), &[]);
    }
    call(mem, mem.rebase(c::FADE_STEP), &[show]);
    let fade = gl(mem, global::FADE);
    let shown = call(mem, mem.rebase(c::MINIMAP_SET_ENABLE), &[fade]);
    let fade = gl(mem, global::FADE);
    if fade == 0 {
        return shown;
    }
    let alpha = st(ld(int(fade)) * kd(mem, k::K1_128));
    let draw = rd(mem, mesh.wrapping_add(0x20)) & !8;
    mem.write(mesh.wrapping_add(0x20), draw);
    mem.write(mesh.wrapping_add(0x28), 0x48u8);
    mem.write(mesh.wrapping_add(0x38), 0x2020u16);
    mem.write(mesh.wrapping_add(0x3a), 0x40u8);
    let level = fade.wrapping_mul(0x60).cast_signed() / 128;
    mem.write(mesh.wrapping_add(0x3b), level.to_le_bytes()[0]);
    let sys = gl(mem, global::YN_MAP_SYSTEM);
    let scene = match sys {
        0 => 0,
        _ => match rd(mem, sys) {
            0 => 0,
            map => rd(mem, map.wrapping_add(8)),
        },
    };
    let zoom = x87_copy(rd(mem, scene.wrapping_add(0xf4)));
    let shown_map = mem.read::<u16>(scene.wrapping_add(0x1a)).cast_signed();
    let angle = fcall(
        mem,
        mem.rebase(c::GUIDE_MAP_ANG),
        FArgs::Stack(&[i32::from(shown_map).cast_unsigned()]),
    );
    let angle = Reg::constant(angle).store().to_bits();
    call(mem, mem.rebase(c::GET_SCREEN), &[g, scale, off]);
    put(mem, up, &[0, 0, 0x3f80_0000, 0]);
    let eye = v;
    call(
        mem,
        mem.rebase(c::COPY_QWORD_CB40),
        &[eye, g.wrapping_add(0x50)],
    );
    call(
        mem,
        mem.rebase(c::COPY_QWORD_CB40),
        &[at, g.wrapping_add(0x50)],
    );
    let drop = ld(rd(mem, g.wrapping_add(0x60))) * kd(mem, k::K4) / ld(rd(mem, scale));
    let ey = st(ld(rd(mem, eye.wrapping_add(4))) - drop);
    mem.write(eye.wrapping_add(4), ey);
    let floor = st(-ld(rd(mem, g.wrapping_add(0x6c))));
    // fcom; test ah, 0x41: clamped only when floor > eye.y (ordered)
    if ld(floor).value() > ld(ey).value() {
        mem.write(eye.wrapping_add(4), x87_copy(floor));
        let n = call(mem, mem.rebase(c::ACTOR_NUMBER), &[]).cast_signed();
        for i in 0..n.max(0) {
            let i = i.cast_unsigned();
            if call(mem, mem.rebase(c::ACTOR_INFO), &[i, info]) != 0
                && rd(mem, info.wrapping_add(4)) & flag::PLAYER != 0
            {
                let x = x87_copy(rd(mem, info.wrapping_add(8)));
                let z = x87_copy(rd(mem, info.wrapping_add(0x10)));
                put(mem, at, &[x, 0, z, 0x3f80_0000]);
                let inv = st(Reg::ONE / ld(zoom));
                call(mem, mem.rebase(c::SCALE_VECTOR4), &[at, at, inv]);
                break;
            }
        }
    }
    call(mem, mem.rebase(c::SET_VIEW_MATRIX), &[view, eye, at, up, 0]);
    call(mem, mem.rebase(c::CAMERA_SET_WORLD), &[view]);
    let inverse = mem.rebase(global::CAMERA_INVERSE);
    call(mem, mem.rebase(c::INVERS_MATRIX), &[inverse, view]);
    let (ox, oy) = (rd(mem, off), rd(mem, off.wrapping_add(4)));
    let fov = x87_copy(kf(mem, k::K45F));
    fcall(
        mem,
        mem.rebase(c::PROJECTION_SUB),
        FArgs::Stack(&[proj, fov, x87_copy(ox), x87_copy(oy), 0, 0]),
    );
    let ten = x87_copy(kf(mem, k::K10F));
    call(mem, mem.rebase(c::SCALE_VECTOR), &[sc, at, ten]);
    call(mem, mem.rebase(c::UNIT_MATRIX), &[t1]);
    let neg = |mem: &M, k: u32| st(-ld(rd(mem, sc.wrapping_add(4 * k))));
    let (nx, ny, nz) = (neg(mem, 0), neg(mem, 1), neg(mem, 2));
    call(mem, mem.rebase(c::SET_TRANSLATE), &[t1, nx, ny, nz]);
    call(mem, mem.rebase(c::UNIT_MATRIX), &[t2]);
    call(mem, mem.rebase(c::SET_TRANSLATE_VEC), &[t2, sc]);
    call(mem, mem.rebase(c::UNIT_MATRIX), &[r]);
    call(mem, mem.rebase(c::ROT_MATRIX_Y), &[r, r, x87_copy(angle)]);
    let mul = mem.rebase(c::MUL_MATRIX);
    let (mm, gm) = (mesh.wrapping_add(0x100), g.wrapping_add(0x10));
    for args in [
        [mm, r, t1],
        [mm, t2, mm],
        [gm, mm, mesh.wrapping_add(0xc0)],
        [gm, view, gm],
        [gm, s, gm],
        [m, view, mm],
        [m, proj, m],
    ] {
        call(mem, mul, &args);
    }
    put(mem, regs, &[0x3_0000, 0]);
    put(mem, regs.wrapping_add(0x10), &[0x3100_0108, 1]);
    put(mem, mask_regs, &[0x44, 0]);
    let set_reg = mem.rebase(c::SET_REG);
    call(mem, set_reg, &[regs]);
    // the minimap's rect on screen, in pixels
    let w = st(ld(rd(mem, scale)) * kd(mem, k::K256));
    let h = st(ld(w) * kd(mem, k::K0_8125));
    let half = kd(mem, k::HALF);
    let half_w = st(ld(w) * half);
    let x0 = ftol(mem, (ld(ox) - ld(half_w) + kd(mem, k::K256)).value());
    let half_h = st(half * ld(h));
    let y0 = ftol(mem, (ld(oy) - ld(half_h) + kd(mem, k::K208)).value());
    let x1 = ftol(mem, ld(w).value()).wrapping_sub(1).wrapping_add(x0);
    let y1 = ftol(mem, ld(h).value()).wrapping_add(y0);
    let left = int(x0.wrapping_add(0x700));
    let top = int(y0.wrapping_add(0x730));
    put(
        mem,
        rect,
        &[
            left,
            top,
            int(x1.wrapping_add(0x700)),
            int(y1.wrapping_add(0x72f)),
        ],
    );
    let sext = |v: u32| i64::from(v.cast_signed());
    let mut sci = sext(y1.wrapping_sub(1)) << 16 | sext(y0);
    sci = sci << 16 | sext(x1);
    sci = sci << 16 | sext(x0);
    let [b0, b1, b2, b3, b4, b5, b6, b7] = sci.to_le_bytes();
    let rect_scissor = [
        u32::from_le_bytes([b0, b1, b2, b3]),
        u32::from_le_bytes([b4, b5, b6, b7]),
    ];
    put(mem, scissor, &rect_scissor);
    call(mem, set_reg, &[scissor]);
    // the background box
    let bg_alpha = st(ld(alpha) * kd(mem, k::K80));
    let (k30, k20) = (kf(mem, k::K30F), kf(mem, k::K20F));
    put(mem, col, &[0, x87_copy(k30), x87_copy(k20), bg_alpha]);
    let k2048 = kd(mem, k::K2048);
    let by = st(ld(oy) + k2048);
    let bx = st(k2048 + ld(ox));
    let prim_box = mem.rebase(c::PRIM_BOX);
    call(mem, prim_box, &[bx, by, x87_copy(w), x87_copy(h), col, 1]);
    // the grid, through the map's origin
    let grid_alpha = st(ld(alpha) * kd(mem, k::K48));
    let (k60, k100, k80) = (kf(mem, k::K60F), kf(mem, k::K100F), kf(mem, k::K80F));
    put(
        mem,
        col,
        &[x87_copy(k60), x87_copy(k100), x87_copy(k80), grid_alpha],
    );
    put(mem, v, &[0, 0, 0x3f80_0000, 0x3f80_0000]);
    let (apply, div) = (mem.rebase(c::APPLY_MATRIX), mem.rebase(c::DIV_VECTOR_XYZ));
    let project = |mem: &mut M, call: &mut C, dst: u32, src: u32| {
        call(mem, apply, &[src, m, src]);
        let w = x87_copy(rd(mem, src.wrapping_add(0xc)));
        call(mem, div, &[dst, src, w]);
    };
    project(mem, call, v, v);
    let origin_y = ftol(mem, (ld(rd(mem, v.wrapping_add(4))) + half - k2048).value());
    let row_phase = origin_y.wrapping_sub(y0).wrapping_add(0xd0).cast_signed() % 0x22;
    let cols = ftol(mem, (ld(w) * kd(mem, k::K1_32) + Reg::ONE).value());
    let origin_x = ftol(mem, (half + ld(rd(mem, v)) - k2048).value());
    let col_phase = origin_x.wrapping_sub(x0).wrapping_add(0x100).cast_signed() % 32;
    let mut gx = int(col_phase
        .cast_unsigned()
        .wrapping_add(0x700)
        .wrapping_add(x0));
    let bottom = int(y1.wrapping_add(0x730));
    let prim_line = mem.rebase(c::PRIM_LINE);
    if cols.cast_signed() >= 0 {
        let k32 = kd(mem, k::K32);
        for _ in 0..cols.wrapping_add(1) {
            call(mem, prim_line, &[gx, top, gx, bottom, col, 1]);
            gx = st(ld(gx) + k32);
        }
    }
    let k34 = kd(mem, k::K34);
    let rows = ftol(mem, (ld(h) / k34 + Reg::ONE).value());
    let mut gy = int(y0
        .wrapping_add(0x730)
        .wrapping_add(row_phase.cast_unsigned()));
    let right = int(x1.wrapping_add(0x701));
    if rows.cast_signed() >= 0 {
        for _ in 0..rows.wrapping_add(1) {
            call(mem, prim_line, &[left, gy, right, gy, col, 1]);
            gy = st(ld(gy) + k34);
        }
    }
    // the fog of the unwalked, the mesh
    let fogged = rd(mem, g.wrapping_add(0x70)) != 0;
    call(mem, mem.rebase(c::FOG_SET_ENABLE), &[u32::from(fogged)]);
    if rd(mem, g.wrapping_add(0x70)) != 0 {
        put(mem, mask_regs, &[0x48, 0]);
        call(mem, set_reg, &[mask_regs]);
        call(mem, mem.rebase(c::UPDATE_WALK), &[g, m, rect]);
        put(mem, mask_regs, &[0x44, 0]);
    }
    if gl(mem, global::FLAGS) & 0x100 != 0 {
        call(mem, mem.rebase(c::MODEL_DRAW), &[mesh]);
    }
    call(mem, set_reg, &[regs]);
    // the markers
    let mut phase = rd(mem, g.wrapping_add(0x74)).cast_signed() % 30;
    if phase > 15 {
        phase = 30 - phase;
    }
    let blink = st(ld(int(phase.cast_unsigned())) * kd(mem, k::K128) / kd(mem, k::K15) * ld(alpha));
    let n = call(mem, mem.rebase(c::ACTOR_NUMBER), &[]);
    let info_at = mem.rebase(c::ACTOR_INFO);
    let palette = mem.rebase(global::PALETTE);
    let copy = mem.rebase(c::COPY_QWORD_CBD0);
    let mut player_index = u32::MAX;
    if n.cast_signed() > 0 {
        let mut destinations = false;
        let field = |mem: &M, o: u32| rd(mem, info.wrapping_add(o));
        for i in 0..n {
            if call(mem, info_at, &[i, info]) == 0 {
                continue;
            }
            let flags = if gl(mem, global::CURRENT_MAP_NO) == 0x99
                && i == 6
                && call(mem, mem.rebase(c::SCENARIO_FLAG), &[]) & 0xffff == 0x618
            {
                mem.write(info.wrapping_add(4), 0x107u32);
                0x107
            } else {
                field(mem, 4)
            };
            if flags == 0 {
                continue;
            }
            if flags & flag::DESTINATION != 0 {
                destinations = true;
                continue;
            }
            let heading = st(ld(field(mem, 0x14)) + ld(angle));
            let ring = (flags.cast_signed() >> 4).cast_unsigned() & 0xf;
            let (x, z) = (x87_copy(field(mem, 8)), x87_copy(field(mem, 0x10)));
            put(mem, v, &[x, 0, z, 0x3f80_0000]);
            project(mem, call, v, v);
            if flags & flag::PLAYER != 0 {
                player_index = i;
                call(mem, copy, &[player, v]);
                continue;
            }
            call(mem, copy, &[col, palette.wrapping_add((flags & 0xf) << 4)]);
            let ca = col.wrapping_add(0xc);
            let a = if flags & flag::BLINK != 0 {
                x87_copy(blink)
            } else {
                st(ld(rd(mem, ca)) * ld(alpha))
            };
            mem.write(ca, a);
            let (vx, vy) = (x87_copy(rd(mem, v)), x87_copy(rd(mem, v.wrapping_add(4))));
            let six = x87_copy(kf(mem, k::K6F));
            match flags & flag::SHAPE {
                flag::EXIT_LINE => {
                    let id = field(mem, 0);
                    let kind = call(mem, mem.rebase(c::ACTOR_TYPE), &[id]);
                    if kind == 2 || kind == 5 {
                        let (a, b) = (line_a, line_b);
                        let pa = |k: u32| a.wrapping_add(4 * k);
                        let pb = |k: u32| b.wrapping_add(4 * k);
                        call(
                            mem,
                            mem.rebase(c::ACTOR_START_POS),
                            &[id, pa(0), pa(1), pa(2)],
                        );
                        call(
                            mem,
                            mem.rebase(c::ACTOR_END_POS),
                            &[id, pb(0), pb(1), pb(2)],
                        );
                        mem.write(pb(3), 0x3f80_0000u32);
                        mem.write(pa(3), 0x3f80_0000u32);
                        project(mem, call, a, a);
                        project(mem, call, b, b);
                        let w = |mem: &M, at: u32| x87_copy(rd(mem, at));
                        let args = [w(mem, pa(0)), w(mem, pa(1)), w(mem, pb(0)), w(mem, pb(1))];
                        call(
                            mem,
                            prim_line,
                            &[args[0], args[1], args[2], args[3], col, 1],
                        );
                    }
                }
                flag::SQUARE => {
                    let tall = x87_copy(kf(mem, k::K6_5F));
                    call(mem, prim_box, &[vx, vy, six, tall, col, 1]);
                }
                flag::TRIANGLE => {
                    let tri = mem.rebase(c::PRIM_TRI);
                    call(mem, tri, &[vx, vy, six, x87_copy(heading), col, 1]);
                }
                flag::DIAMOND => {
                    let eight = x87_copy(kf(mem, k::K8F));
                    call(mem, mem.rebase(c::PRIM_BATU), &[vx, vy, eight, col, 1]);
                }
                _ => {}
            }
            if flags & flag::RING != 0 {
                let ten = x87_copy(kf(mem, k::K10F));
                let colour = palette.wrapping_add(ring << 4);
                let circle = mem.rebase(c::PRIM_CIRCLE_LINE);
                call(mem, circle, &[vx, vy, ten, colour, 1]);
            }
        }
        if destinations {
            put(mem, scissor, &FULL_SCISSOR);
            call(mem, set_reg, &[scissor]);
            for i in 0..n {
                if call(mem, info_at, &[i, info]) == 0 || field(mem, 4) & flag::DESTINATION == 0 {
                    continue;
                }
                for (dst, xo, zo) in [(p, 0x1c, 0x20), (q, 0x24, 0x28), (v, 8, 0x10)] {
                    let (x, z) = (x87_copy(field(mem, xo)), x87_copy(field(mem, zo)));
                    put(mem, v, &[x, 0, z, 0x3f80_0000]);
                    project(mem, call, dst, v);
                }
                let dir = field(mem, 0x18);
                let smooth = u32::from(mem.read::<u8>(info.wrapping_add(0x2c)));
                call(
                    mem,
                    mem.rebase(c::DRAW_PLAYER),
                    &[
                        g,
                        player,
                        v,
                        rect,
                        x87_copy(angle),
                        x87_copy(alpha),
                        p,
                        q,
                        dir,
                        smooth,
                    ],
                );
            }
            put(mem, scissor, &rect_scissor);
            call(mem, set_reg, &[scissor]);
        }
        if player_index != u32::MAX && call(mem, info_at, &[player_index, info]) != 0 {
            let ca = col.wrapping_add(0xc);
            call(mem, copy, &[col, mem.rebase(global::PLAYER_COLOUR)]);
            let a = st(ld(rd(mem, ca)) * ld(alpha));
            mem.write(ca, a);
            let heading = st(ld(field(mem, 0x14)) + ld(angle));
            let (px, py) = (
                x87_copy(rd(mem, player)),
                x87_copy(rd(mem, player.wrapping_add(4))),
            );
            let six = x87_copy(kf(mem, k::K6F));
            let tri = mem.rebase(c::PRIM_TRI);
            call(mem, tri, &[px, py, six, x87_copy(heading), col, 1]);
            call(mem, copy, &[col, mem.rebase(global::CIRCLE_COLOUR)]);
            let a = st(ld(rd(mem, ca)) * ld(alpha));
            mem.write(ca, a);
            let ten = x87_copy(kf(mem, k::K10F));
            let circle = mem.rebase(c::PRIM_CIRCLE_LINE);
            call(mem, circle, &[px, py, ten, col, 1]);
            if rd(mem, g.wrapping_add(0x70)) != 0 {
                let (x, z) = (x87_copy(field(mem, 8)), x87_copy(field(mem, 0x10)));
                call(mem, mem.rebase(c::REVEAL_FOG), &[g, x, z]);
            }
        }
    }
    put(mem, scissor, &FULL_SCISSOR);
    call(mem, set_reg, &[scissor]);
    call(mem, mem.rebase(c::RESTORE_REG), &[]);
    let frames = rd(mem, g.wrapping_add(0x74)).wrapping_add(1);
    mem.write(g.wrapping_add(0x74), frames);
    call(mem, mem.rebase(c::CLOSE_SCISSOR), &[])
}

/// `fld m32` then `fstp m32` ([`crate::x87::copy`]).
const fn x87_copy(bits: u32) -> u32 {
    crate::x87::copy(bits)
}
