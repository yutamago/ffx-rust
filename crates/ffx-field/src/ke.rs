//! `Ke`: resource heaps and the lens flare objects of the `Ke` module.
//! Their pointers go through `set_ptr32`/`get_ptr32_func`, the identity
//! on PC (`ffx_system::stub`).

/// A resource heap header (`KeThResHd_Init`): `count` blocks of a stride
/// derived from `size` follow it from `+0x10`, linked in a free list.
pub mod res_hd {
    /// `+0x0`: blocks (`u8`).
    pub const COUNT: u32 = 0x0;
    /// `+0x1`: the size class (`u8`).
    pub const SIZE: u32 = 0x1;
    /// `+0x2`: bytes per block (`u16`).
    pub const STRIDE: u32 = 0x2;
    /// `+0x4`: the first free block.
    pub const FREE: u32 = 0x4;
    /// `+0x10`: the first block.
    pub const BLOCKS: u32 = 0x10;
}

/// Bytes per block for a size class: `(size - 1) * 48 + 0x80`, in the
/// original's 8- and 16-bit arithmetic.
#[must_use]
pub fn res_stride(size: u8) -> u16 {
    u16::from(size.wrapping_sub(1))
        .wrapping_mul(48)
        .wrapping_add(0x80)
}

/// The lens flares' default shape table (`KeLnsFls_Init` and siblings).
pub const LNS_DEFAULT: u32 = 0x00d5_2398;

/// The VFX packet position `KeLnsShp_Draw` records (`0x0230fd2c`).
pub const PACKET_POS: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x0230_fd2c);
/// 65535.0, an `f64` in `.rdata` (`KeLnsClm_SetShp`).
pub const PHASE_SCALE: ffx_bridge::Global<f64> = ffx_bridge::Global::new(0x00b1_70d8);
/// The magic effects whose models share one instance
/// (`KeDttMdl_UseSingleInstance`).
pub const SINGLE_INSTANCE_MAGIC: [u32; 10] = [
    0x203, 0x15c, 0x15e, 0x182, 0x241, 0x20a, 0x23a, 0x29b, 0x1db, 0x46,
];

/// A pointer to the current VFX data pointer (`KeBornRnd2`).
pub const VFX_DATA: u32 = 0x00c3_a4cc;
/// `KeDttSpr_MkGifPk_Pre`'s `.rdata` constants: 1/1024 (`f32`), 0.0625
/// (`f64`), 4096 (`f32`), 4096 (`f64`).
pub const SPR_PRE_CONSTS: [u32; 4] = [0x00b5_1b9c, 0x00b9_21e0, 0x00b5_0224, 0x00b5_15c8];
/// The `pmcom` arena's next address (`KePmcomAlloc`).
pub const PMCOM_NEXT: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x00d5_24a4);
/// Its base.
pub const PMCOM_BASE: u32 = 0x00e5_24f0;
/// Its end.
pub const PMCOM_END: u32 = 0x00f5_24f0;
/// The `pmcom` queue: 256 entries of (kind, pointer) (`KePmcomQueuePush`).
pub const PMCOM_QUEUE: u32 = 0x010d_4d28;
/// Its write index.
pub const PMCOM_QUEUE_INDEX: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x010d_552c);
/// `op_random(x)` (`0x007e6700`): a draw in `[-x, x)`, `f32` in, `ST0` out.
pub const OP_RANDOM: u32 = 0x007e_6700;
/// `op_random_p(x)` (`0x007e6780`): a draw in `[0, x)`.
pub const OP_RANDOM_P: u32 = 0x007e_6780;
/// `pppAllFreePObject(block)` (`0x00711f20`).
pub const PPP_ALL_FREE_POBJECT: u32 = 0x0071_1f20;
/// `pppInitEnv(env, a, heap, size)` (`0x007169d0`).
pub const PPP_INIT_ENV: u32 = 0x0071_69d0;
/// The object block count `pppFreeAllEnvObjects` walks (`i32`).
pub const PPP_OBJECT_COUNT: u32 = 0x00d4_b4cc;
/// The object blocks, 0x80 bytes each.
pub const PPP_OBJECTS: u32 = 0x00d4_e390;
/// The pmcom environment (`pppInitEnv`'s first argument).
pub const PMCOM_ENV: u32 = 0x010d_4cfc;
/// Its heap, 0x100000 bytes.
pub const PMCOM_HEAP: u32 = 0x00fd_24f0;
/// The pmcom free pointer, set to `+8` past itself (`pppInitPmcomEnv`).
pub const PMCOM_FREE: u32 = 0x00d5_24a0;
/// `g_pppOtDrawing` (`0x00cec1a4`): set while an ordering-table pass draws.
pub const PPP_OT_DRAWING: u32 = 0x00ce_c1a4;
/// `g_pppCurrentProgram`'s low byte (`0x0230fd20`).
pub const PPP_CURRENT_PROGRAM: u32 = 0x0230_fd20;
/// The VFX offset the lens positions add (`0x0230ff50`, three floats).
pub const VFX_OFFSET: u32 = 0x0230_ff50;
/// `graphicVFXGetRenderDataByInstance(lo, hi)` (`0x006453d0`).
pub const GRAPHIC_VFX_GET_RENDER_DATA_BY_INSTANCE: u32 = 0x0064_53d0;
/// `graphicVFXUpdate(lo, hi)` (`0x006458a0`).
pub const GRAPHIC_VFX_UPDATE: u32 = 0x0064_58a0;
/// `graphicVFXSetVisible(lo, hi, on)` (`0x00645710`).
pub const GRAPHIC_VFX_SET_VISIBLE: u32 = 0x0064_5710;
/// `updateVFXRenderDataShp(lo, hi, data, desc, frame)` (`0x0071b8a0`).
pub const UPDATE_VFX_RENDER_DATA_SHP: u32 = 0x0071_b8a0;
/// `InitVFXRenderDataShp(lo, hi, data, desc, frame, flags)` (`0x00711460`).
pub const INIT_VFX_RENDER_DATA_SHP: u32 = 0x0071_1460;
/// `calculateVFXSP_AlphaFunction(mode)` (`0x00712c30`).
pub const CALCULATE_VFX_SP_ALPHA_FUNCTION: u32 = 0x0071_2c30;
/// `FFXVu0CopyMatrix(dst, src)` (`0x006ed360`).
pub const VU0_COPY_MATRIX: u32 = 0x006e_d360;
/// 2.0 (`f64`, `.rdata`).
pub const TWO: u32 = 0x00b9_2270;
/// 4.0 (`f64`, `.rdata`).
pub const FOUR: u32 = 0x00b4_3d80;
/// `FFXVu0UnitMatrix(m)` (`0x006ed9a0`).
pub const VU0_UNIT_MATRIX: u32 = 0x006e_d9a0;
/// `FFXVu0CopyVector(dst, src)` (`0x006ed370`).
pub const VU0_COPY_VECTOR: u32 = 0x006e_d370;
/// `FFXVu0ScaleVector(dst, src, k)` (`0x006ed8f0`).
pub const VU0_SCALE_VECTOR: u32 = 0x006e_d8f0;
/// `FFXVu0ApplyMatrix(dst, m, v)` (`0x006ed2e0`).
pub const VU0_APPLY_MATRIX: u32 = 0x006e_d2e0;
/// `FFXVu0Normalize(dst, src)` (`0x006ed6f0`).
pub const VU0_NORMALIZE: u32 = 0x006e_d6f0;
/// `FFXVu0InnerProduct(a, b)` (`0x0062fce0`, the result in `ST0`).
pub const VU0_INNER_PRODUCT: u32 = 0x0062_fce0;
/// `KeVu0VecLen(v)` (`0x0075fda0`, `ST0`).
pub const KE_VU0_VEC_LEN: u32 = 0x0075_fda0;
/// `KeVu0VecLenXY(v)` (`0x0075fde0`, `ST0`).
pub const KE_VU0_VEC_LEN_XY: u32 = 0x0075_fde0;
/// -1.0 (`f32`, `.rdata`): `KeLnsLp_Update`'s translation negation.
pub const LNS_NEG_ONE: u32 = 0x00b9_2340;
/// 0.9230769 (`f64` of an `f32`): its screen y squash (12 / 13).
pub const LNS_Y_SCALE: u32 = 0x00b5_1bd8;
/// 512.0 (`f32`): its screen depth.
pub const LNS_DEPTH: u32 = 0x00b4_1d9c;
/// 1/512 (`f64`): its depth scale.
pub const LNS_DEPTH_SCALE: u32 = 0x00b4_1cc0;
/// 0.5 (`f64`).
pub const LNS_HALF: u32 = 0x00b9_2220;
/// 0.3 (`f64` of an `f32`): its view cone.
pub const LNS_CONE: u32 = 0x00b4_8308;
/// `FFXVu0SubVector(dst, a, b)` (`0x006ed940`).
pub const VU0_SUB_VECTOR: u32 = 0x006e_d940;
/// `KeVu0CopyVecXYZ(dst, src)` (`0x0075ede0`).
pub const KE_VU0_COPY_VEC_XYZ: u32 = 0x0075_ede0;
/// `KeVu0AddVecXYZ(dst, a, b)` (`0x0075eca0`).
pub const KE_VU0_ADD_VEC_XYZ: u32 = 0x0075_eca0;
/// `pppCreatePObject(part, base)` (`0x00716770`).
pub const PPP_CREATE_POBJECT: u32 = 0x0071_6770;
/// `set_ptr32(p)` (`0x00a44700`).
pub const SET_PTR32: u32 = 0x00a4_4700;
/// `is_ptr32_null(p)` (`0x00a446a0`).
pub const IS_PTR32_NULL: u32 = 0x00a4_46a0;
/// `get_ptr32_func(p)` (`0x00a44650`).
pub const GET_PTR32_FUNC: u32 = 0x00a4_4650;
/// The emulated EE registers (`0x00c8f500`..): `+8` the register
/// `KeMdl_Draw` interleaves with, `+0x88` and `+0x98` its operands.
pub const EE_REGS: u32 = 0x00c8_f500;
/// `g_pppColorConShift` (`0x00c8b500`): a byte offset added once to the
/// colour address, cleared after use.
pub const PPP_COLOR_CON_SHIFT: u32 = 0x00c8_b500;
/// `g_pppCurrentPart` (`0x0230ffe4`).
pub const PPP_CURRENT_PART: u32 = 0x0230_ffe4;
/// `pppEePmulth(dst, a, b)` (`0x007374e0`).
pub const PPP_EE_PMULTH: u32 = 0x0073_74e0;
/// `pppEePpacb(dst, a, b)` (`0x007375d0`).
pub const PPP_EE_PPACB: u32 = 0x0073_75d0;
/// `g_pppViewMatrix` (`0x0230ff20`).
pub const PPP_VIEW_MATRIX: u32 = 0x0230_ff20;
/// `FFXVu0MulMatrix(dst, a, b)` (`0x006ed5c0`).
pub const VU0_MUL_MATRIX: u32 = 0x006e_d5c0;
/// The GIF tag word `KeMdl_Draw` copies (`0x00c498dc`).
pub const REF_GIF_TAG: u32 = 0x00c4_98dc;
/// The VFX packet position (`0x0230fd2c`).
pub const PACKET_POS_ADDR: u32 = 0x0230_fd2c;
/// The two VFX model data blocks `KeMdl_Draw`'s description points at
/// (`0x0230fd60`, `0x0230fda0`).
pub const MDL_FP_DATA: u32 = 0x0230_fd60;
/// See [`MDL_FP_DATA`].
pub const MDL_FQ_DATA: u32 = 0x0230_fda0;
/// `updateVFXRenderData(lo, hi, rec, desc)` (`0x00718610`).
pub const UPDATE_VFX_RENDER_DATA: u32 = 0x0071_8610;
/// `InitVFXRenderData(lo, hi, rec, desc, flags)` (`0x00711180`).
pub const INIT_VFX_RENDER_DATA: u32 = 0x0071_1180;
/// `InitVFXPath3RenderData(lo, hi, rec, desc, flags)` (`0x00710fd0`).
pub const INIT_VFX_PATH3_RENDER_DATA: u32 = 0x0071_0fd0;
/// `graphicVFXPath3Update(lo, hi)` (`0x00645600`).
pub const GRAPHIC_VFX_PATH3_UPDATE: u32 = 0x0064_5600;
/// `graphicVFXSetClutParameters(lo, hi, a, b, c)` (`0x00645630`).
pub const GRAPHIC_VFX_SET_CLUT_PARAMETERS: u32 = 0x0064_5630;
/// `calculateVFXSP_ZMat(mode)` (`0x00712d00`).
pub const CALCULATE_VFX_SP_ZMAT: u32 = 0x0071_2d00;

/// The `Ke` objects where the game keeps them, through [`ffx_bridge::Mem`].
pub mod bridge {
    use crate::camera::FArgs;
    use crate::x87;
    use ffx_bridge::Mem;
    use ffx_bridge::x87::Reg;
    use ffx_system::stub::{is_ptr32_null, ptr32};

    use super::{LNS_DEFAULT, res_hd, res_stride};

    /// `KeThResHd_Init(hd, count, size)` (`0x00764bb0`): the header and the
    /// free list through the blocks (`count` 0 still terminates at `+0x10`).
    pub fn ke_th_res_hd_init(mem: &mut impl Mem, hd: u32, count: u8, size: u8) {
        mem.write(hd + res_hd::SIZE, size);
        let stride = res_stride(size);
        mem.write(hd + res_hd::COUNT, count);
        mem.write(hd + res_hd::STRIDE, stride);
        mem.write(hd + res_hd::FREE, ptr32(0));
        let mut block = hd + res_hd::BLOCKS;
        mem.write(hd + res_hd::FREE, ptr32(block));
        for _ in 1..i32::from(count) {
            let next = block.wrapping_add(u32::from(stride));
            mem.write(block, ptr32(next));
            block = next;
        }
        mem.write(block, ptr32(0));
    }

    /// `KeThCp_Init(cp)` (`0x00764800`).
    pub fn ke_th_cp_init(mem: &mut impl Mem, cp: u32) {
        mem.write(cp, ptr32(0));
        mem.write(cp + 4, 0u8);
    }

    /// `KeLnsFls_Init(obj)` (`0x00769690`), and `KeLnsCrn_Init`
    /// (`0x0076a170`), the same code: the default shapes, no data.
    pub fn ke_lns_init(mem: &mut impl Mem, o: u32) {
        let shapes = mem.rebase(LNS_DEFAULT);
        mem.write(o, ptr32(shapes));
        mem.write(o + 4, ptr32(0));
    }

    /// `KeLnsClm_Init(obj)` (`0x00769ca0`): as [`ke_lns_init`], and byte
    /// `+9` cleared first.
    pub fn ke_lns_clm_init(mem: &mut impl Mem, o: u32) {
        mem.write(o + 9, 0u8);
        ke_lns_init(mem, o);
    }

    /// `KeLnsFlr_Init(obj, src)` (`0x0076a1d0`): the `u16` at `+0x12` of
    /// what `*src` points at, and byte `+2` cleared.
    pub fn ke_lns_flr_init(mem: &mut impl Mem, o: u32, src: u32) {
        let p = ptr32(mem.read(src));
        let v = mem.read::<u16>(p + 0x12);
        mem.write(o, v);
        mem.write(o + 2, 0u8);
    }

    // ---- round 4: lens flare shapes, the magic model instance rule ----

    /// `KeLnsLp_Init(obj)` (`0x0076a670`): its matrix at `+0x40` the
    /// identity, `+0xac` 1.0; returns `FFXVu0UnitMatrix`'s `eax`.
    pub fn ke_lns_lp_init(mem: &mut impl Mem, o: u32) -> u32 {
        let r = ffx_ps2::vu0::bridge::matrix_identity(mem, o.wrapping_add(0x40));
        mem.write(o.wrapping_add(0xac), 1f32);
        r
    }

    /// `KeLnsCrn_SetShp(obj, shp)` (`0x0076a1a0`): the shape, its first
    /// frame's duration (`u16` at `+0x12` of the shape's frame table) at
    /// `+8` and frame 0 at `+0xa`. Returns the table pointer's high half
    /// with that duration (what `eax` holds after `mov ax`).
    pub fn ke_lns_crn_set_shp(mem: &mut impl Mem, o: u32, shp: u32) -> u32 {
        mem.write(o.wrapping_add(4), ptr32(shp));
        let frames = ptr32(mem.read::<u32>(shp));
        let d = mem.read::<u16>(frames.wrapping_add(0x12));
        mem.write(o.wrapping_add(8), d);
        mem.write(o.wrapping_add(0xa), 0u8);
        (frames & 0xffff_0000) | u32::from(d)
    }

    /// `KeLnsShp_Draw(obj)` (`0x0076ab90`): reads its shape and state (as
    /// the original does, without using them), then `+0x18` = 0, `+0x28`
    /// = 0, `+0x24` = the packet position (`0x0230fd2c`), `+0x1c` = `0x11`,
    /// `+0x12` |= `0xb0`; returns `0xb0`.
    pub fn ke_lns_shp_draw(mem: &mut impl Mem, o: u32) -> u32 {
        let shp = ptr32(mem.read::<u32>(o));
        let _ = ptr32(mem.read::<u32>(shp));
        let _ = ptr32(mem.read::<u32>(o.wrapping_add(4)));
        mem.write(o.wrapping_add(0x18), 0u32);
        mem.write(o.wrapping_add(0x28), 0u8);
        let pk = super::PACKET_POS.get(mem);
        mem.write(o.wrapping_add(0x24), pk);
        mem.write(o.wrapping_add(0x1c), 0x11u16);
        let f = mem.read::<u16>(o.wrapping_add(0x12));
        mem.write(o.wrapping_add(0x12), f | 0xb0);
        0xb0
    }

    /// `KeLnsShp_Update(obj)` (`0x0076abe0`): the shape animation's clock
    /// (`u16` at the state's `+0`) moves 0x200 a frame; past the frame's
    /// duration (16-bit signed compares) it goes to the next frame, and
    /// past the last frame back to 0 when the frame's `+0x14` bit 7 is
    /// set, else stays on the last. Returns the `eax` it ends with: the
    /// new clock, the shape table, or 0.
    pub fn ke_lns_shp_update(mem: &mut impl Mem, o: u32) -> u32 {
        let st = ptr32(mem.read::<u32>(o.wrapping_add(4)));
        let shp = ptr32(mem.read::<u32>(o));
        let mut idx = mem.read::<u8>(st.wrapping_add(2));
        let table = ptr32(mem.read::<u32>(shp));
        let frame = table.wrapping_add(u32::from(idx) * 8);
        let t = mem.read::<u16>(st).wrapping_add(0x200);
        mem.write(st, t);
        let dur = mem.read::<u16>(frame.wrapping_add(0x12));
        #[allow(clippy::cast_possible_wrap)] // 16-bit signed compares
        if (t as i16) < (dur as i16) {
            return u32::from(t);
        }
        mem.write(st, t.wrapping_sub(dur));
        idx = idx.wrapping_add(1);
        mem.write(st.wrapping_add(2), idx);
        let shp = ptr32(mem.read::<u32>(o));
        let idx = mem.read::<u8>(st.wrapping_add(2));
        let hdr = ptr32(mem.read::<u32>(shp));
        #[allow(clippy::cast_possible_wrap)]
        if i16::from(idx) < (mem.read::<u16>(hdr.wrapping_add(6)) as i16) {
            return hdr;
        }
        mem.write(st, 0u16);
        if mem.read::<u8>(frame.wrapping_add(0x14)) & 0x80 != 0 {
            mem.write(st.wrapping_add(2), 0u8);
        } else {
            mem.write(st.wrapping_add(2), idx.wrapping_sub(1));
        }
        0
    }

    /// `KeLnsClm_SetShp(obj, shp, seed)` (`0x00769cd0`): the shape, each of
    /// the `+8` (`u8`) elements' duration (`+0x10 + 8i`) and frame 0; the
    /// first time (`+9` bit 0 clear) each element's phase (`+0x14 + 8i`)
    /// `s / 65535.0` (an `f64` in `.rdata`) with `s = s * 0x80d + 7` in 16
    /// bits from the seed, then sets the bit.
    pub fn ke_lns_clm_set_shp(mem: &mut impl Mem, o: u32, shp: u32, seed: u32) {
        mem.write(o.wrapping_add(4), ptr32(shp));
        let n = u32::from(mem.read::<u8>(o.wrapping_add(8)));
        for i in 0..n {
            let frames = ptr32(mem.read::<u32>(shp));
            let d = mem.read::<u16>(frames.wrapping_add(0x12));
            let e = o.wrapping_add(0x10 + 8 * i);
            mem.write(e, d);
            mem.write(e.wrapping_add(2), 0u8);
        }
        if mem.read::<u8>(o.wrapping_add(9)) & 1 != 0 {
            return;
        }
        let k = super::PHASE_SCALE.get(mem);
        let mut s = seed;
        let mut i = 0;
        while i < u32::from(mem.read::<u8>(o.wrapping_add(8))) {
            s = s.wrapping_mul(0x80d).wrapping_add(7) & 0xffff;
            // fild, fstp m32 (exact), fld, fdiv by the f64
            let v = f64::from(s) / k;
            mem.write(o.wrapping_add(0x14 + 8 * i), crate::x87::store(v));
            i += 1;
        }
        let f = mem.read::<u8>(o.wrapping_add(9));
        mem.write(o.wrapping_add(9), f | 1);
    }

    /// `KeDttMdl_UseSingleInstance()` (`0x0072e6d0`): 1 for the magic
    /// effects that share one model instance (ids `0x203`, `0x15c`,
    /// `0x15e`, `0x182`, `0x241`, `0x20a`, `0x23a`, `0x29b`, `0x1db`,
    /// `0x46`), else 0.
    pub fn ke_dtt_mdl_use_single_instance(mem: &impl Mem) -> u32 {
        let id = ffx_system::instance::bridge::magic_id(mem);
        u32::from(super::SINGLE_INSTANCE_MAGIC.contains(&id))
    }

    /// `KeThBP_Quit(bp)` (`0x007636c0`): every object on the lists of the
    /// `+0x3c` buckets (`0x30` bytes from `+0x58`) and of the `+0x38`
    /// buckets from bucket `+0x34` is flagged to end (`+0x3e` |= 8 and the
    /// owner's bit 5) and loses its `+0xc`; then the owner is unlinked
    /// from its list (`+8` next, `+0xc` previous).
    pub fn ke_th_bp_quit(mem: &mut impl Mem, bp: u32) {
        let bits = (mem.read::<u16>(bp.wrapping_add(0x3e)) & 0x20) | 8;
        let first = u32::from(mem.read::<u16>(bp.wrapping_add(0x34)));
        let ranges = [
            (0, mem.read::<u16>(bp.wrapping_add(0x3c))),
            (first, mem.read::<u16>(bp.wrapping_add(0x38))),
        ];
        for (start, n) in ranges {
            for k in 0..u32::from(n) {
                let head = bp.wrapping_add(0x58 + (start + k) * 0x30);
                let mut o = ptr32(mem.read::<u32>(head));
                while o != 0 {
                    let f = mem.read::<u16>(o.wrapping_add(0x3e));
                    mem.write(o.wrapping_add(0x3e), f | bits);
                    mem.write(o.wrapping_add(0xc), ptr32(0));
                    o = ptr32(mem.read::<u32>(o.wrapping_add(8)));
                }
            }
        }
        let prev = mem.read::<u32>(bp.wrapping_add(0xc));
        if is_ptr32_null(prev) == 0 {
            let next = mem.read::<u32>(bp.wrapping_add(8));
            if is_ptr32_null(next) == 0 {
                let prev = mem.read::<u32>(bp.wrapping_add(0xc));
                mem.write(ptr32(next).wrapping_add(0xc), prev);
            }
            let prev = ptr32(mem.read::<u32>(bp.wrapping_add(0xc)));
            let next = mem.read::<u32>(bp.wrapping_add(8));
            mem.write(prev, next);
        }
    }

    /// `KePmcomAlloc(bytes)` (`0x00762f50`): bytes + 15 rounded down to 8
    /// from the `pmcom` arena (`0x00d524a4` next; `0x00e524f0` up to
    /// `0x00f524f0`); past its end it restarts at the base (and spins
    /// forever if the request alone does not fit, as the original).
    pub fn ke_pmcom_alloc(mem: &mut impl Mem, bytes: u32) -> u32 {
        let n = bytes.wrapping_add(0xf) & !7;
        let at = super::PMCOM_NEXT.get(mem);
        let end = at.wrapping_add(n);
        super::PMCOM_NEXT.set(mem, end);
        let (base, top) = (mem.rebase(super::PMCOM_BASE), mem.rebase(super::PMCOM_END));
        if end < top {
            return at;
        }
        let again = n.wrapping_add(base);
        super::PMCOM_NEXT.set(mem, again);
        while super::PMCOM_NEXT.get(mem) >= top {
            std::hint::spin_loop(); // the original spins here forever
        }
        base
    }

    /// `KePmcomQueuePush(kind, p)` (`0x00762fa0`): appends `(kind, p)` to
    /// the 256-entry ring at `0x010d4d28` (index `0x010d552c`, back to 0
    /// past 255).
    pub fn ke_pmcom_queue_push(mem: &mut impl Mem, kind: u32, p: u32) {
        let i = super::PMCOM_QUEUE_INDEX.get(mem);
        let e = mem
            .rebase(super::PMCOM_QUEUE)
            .wrapping_add(i.wrapping_mul(8));
        let i = i.wrapping_add(1);
        super::PMCOM_QUEUE_INDEX.set(mem, i);
        mem.write(e, kind);
        mem.write(e.wrapping_add(4), ptr32(p));
        let i = super::PMCOM_QUEUE_INDEX.get(mem);
        let keep = if i.cast_signed() >= 0x100 {
            0
        } else {
            u32::MAX
        };
        super::PMCOM_QUEUE_INDEX.set(mem, i & keep);
    }

    /// `KeLnsDefault_Init(obj)` (`0x0076a640`, `KeLnsFls_Init`'s code):
    /// the default shapes, no data; returns `set_ptr32(0)`.
    pub fn ke_lns_default_init(mem: &mut impl Mem, o: u32) -> u32 {
        ke_lns_init(mem, o);
        0
    }

    /// `KeVu0MulX(dst, src, s)` (`0x007676c0`): `dst.x = src.x * s`.
    pub fn ke_vu0_mul_x(mem: &mut impl Mem, dst: u32, src: u32, s: f32) {
        let x = mem.read::<f32>(src);
        mem.write(dst, Reg::load(x).mul_m32(s).store());
    }

    /// `KeVu0SubX(dst, src, s)` (`0x00767720`): `dst.x = src.x - s`, then
    /// `restoreVf00Register` (a tail call).
    pub fn ke_vu0_sub_x(mem: &mut impl Mem, dst: u32, src: u32, s: f32) {
        let x = mem.read::<f32>(src);
        mem.write(dst, Reg::load(x).sub_m32(s).store());
        ffx_ps2::ee::bridge::restore_vf00(mem);
    }

    /// `KeBornRnd2(obj, arg, born, &index, shape, random, n)`
    /// (`0x00769230`): spawns up to `n` (8-bit) particles at points of
    /// shape `shape` of the current VFX data (`[[0x00c3a4cc]] + 0x28`, 8
    /// bytes each: `i16` model, `i16` point count, the point list): for
    /// each, `born(obj, arg, vertices + 6 * point, &more)`; it stops when
    /// `born` clears `more` (returning 0). The vertices are the object's
    /// (`+0x90`) or the model's (`+0x14` of its `0x20`-byte record at `+0x20`;
    /// none for model `-0x6fe8`). Points go in order from `*index`
    /// (wrapping at the count), or at random (`KeVu0RndI() % count`) when
    /// `random`; the last one is kept in `*index` and 1 returned.
    #[allow(clippy::too_many_arguments)]
    pub fn ke_born_rnd2<M: Mem>(
        mem: &mut M,
        o: u32,
        arg: u32,
        born: u32,
        index: u32,
        shape: i16,
        random: u8,
        n: u8,
        mut rnd: impl FnMut(&mut M) -> u32,
        mut call: impl FnMut(&mut M, u32, u32, u32, u32) -> u32,
    ) -> u32 {
        let data = mem.read::<u32>(mem.read::<u32>(mem.rebase(super::VFX_DATA)));
        let rec = mem
            .read::<u32>(data.wrapping_add(0x28))
            .wrapping_add(i32::from(shape).cast_unsigned().wrapping_mul(8));
        let mut verts = ptr32(mem.read::<u32>(o.wrapping_add(0x90)));
        if verts == 0 {
            let model = mem.read::<u16>(rec);
            if model != 0x9018 {
                let data = mem.read::<u32>(mem.read::<u32>(mem.rebase(super::VFX_DATA)));
                let models = mem.read::<u32>(data.wrapping_add(0x20));
                let m = i32::from(model.cast_signed()).cast_unsigned();
                verts = ptr32(mem.read::<u32>(models.wrapping_add(m << 5).wrapping_add(0x14)));
            }
        }
        let count = |mem: &M| i32::from(mem.read::<i16>(rec.wrapping_add(2)));
        let draw = |mem: &mut M, rnd: &mut dyn FnMut(&mut M) -> u32| -> u32 {
            let r = rnd(mem).cast_signed();
            r.wrapping_rem(count(mem)).cast_unsigned()
        };
        let mut i = if random != 0 {
            draw(mem, &mut rnd)
        } else {
            u32::from(mem.read::<u16>(index))
        };
        let mut left = n;
        while left != 0 {
            let points = ptr32(mem.read::<u32>(rec.wrapping_add(4)));
            let pt = u32::from(mem.read::<u16>(points.wrapping_add(i.wrapping_mul(2))));
            let more = call(mem, born, o, arg, verts.wrapping_add(pt * 6));
            if more == 0 {
                return 0;
            }
            if random != 0 {
                i = draw(mem, &mut rnd);
            } else {
                i = i.wrapping_add(1);
                if i.cast_signed() >= count(mem) {
                    i = 0;
                }
            }
            left -= 1;
        }
        #[allow(clippy::cast_possible_truncation)] // the low half
        mem.write(index, i as u16);
        1
    }

    /// `KeDttSpr_MkGifPk_Pre(spr)` (`0x00768d30`): loads the sprite
    /// packet's emulated VU0 registers (`0x00c8f7a0`..`0x00c8f9ac`) before
    /// its GIF packets: the scratchpad block (`op_spr_alloc(0x90)`, `alloc`;
    /// its `+0x80`, `+0x84` set to -8) into eight registers, the sprite's
    /// matrix rows (`+0x20`, `+0x28` data), its scale (`+0x30`) and
    /// position (`+0x50`), and the products the VU program needs (the
    /// emulated VU0 constants at `0x00c0a004`..`0x00c0a010`, 1/sqrt(2) as
    /// `0x3f3504f3`, `.rdata` constants 1/1024, 0.0625, 4096). The x87
    /// chains are kept, rounding only where the original stores. Frees the
    /// block (`op_spr_free`, `free`).
    #[allow(clippy::too_many_lines)] // one register after another
    pub fn ke_dtt_spr_mk_gif_pk_pre<M: Mem>(
        mem: &mut M,
        o: u32,
        alloc: impl FnOnce(&mut M) -> u32,
        free: impl FnOnce(&mut M),
    ) {
        let reg = |mem: &M, a: u32| mem.rebase(a);
        let get = |mem: &M, a: u32| -> f32 { mem.read::<f32>(mem.rebase(a)) };
        let put = |mem: &mut M, a: u32, v: f32| {
            let at = mem.rebase(a);
            mem.write(at, v);
        };
        let b = alloc(mem);
        mem.write(b.wrapping_add(0x80), [0xffff_fff8u32, 0xffff_fff8]);
        for (i, dst) in [0x920u32, 0x940, 0x960, 0x8e0, 0x870, 0x8d0, 0x820, 0x7c0]
            .into_iter()
            .enumerate()
        {
            #[allow(clippy::cast_possible_truncation)] // 0..8
            let v = mem.read::<[u32; 4]>(b.wrapping_add(16 * i as u32));
            let at = reg(mem, 0x00c8_f000 + dst);
            mem.write(at, v);
        }
        let p20 = ptr32(mem.read::<u32>(o.wrapping_add(0x20)));
        let row = mem.read::<[u32; 4]>(p20);
        let at = reg(mem, 0x00c8_f7b0);
        mem.write(at, row);
        let r8 = Reg::load(get(mem, 0x00c0_a008));
        let row = mem.read::<[u32; 4]>(p20.wrapping_add(0x10));
        let at = reg(mem, 0x00c8_f980);
        mem.write(at, row);
        put(mem, 0x00c8_f7b4, r8.add_m32(f32::from_bits(row[1])).store());
        let k = mem.read::<u32>(mem.rebase(super::SPR_PRE_CONSTS[0]));
        let at = reg(mem, 0x00c8_f910);
        mem.write(at, [x87::copy(k), 0, 0, 0]);
        let p28 = ptr32(mem.read::<u32>(o.wrapping_add(0x28)));
        for i in 0..4u32 {
            let v = mem.read::<[u32; 4]>(p28.wrapping_add(16 * i));
            let at = reg(mem, 0x00c8_f7d0 + 16 * i);
            mem.write(at, v);
        }
        let s = mem.read::<[i32; 4]>(b.wrapping_add(0x80));
        let at = reg(mem, 0x00c8_f840);
        mem.write(at, s);
        // fild, fstp m32, fld: each an f32 of the int
        let f = |v: i32| Reg::load(f32::from_bits(x87::store(f64::from(v))));
        let sixteenth = Reg::constant(mem.read::<f64>(mem.rebase(super::SPR_PRE_CONSTS[1])));
        put(mem, 0x00c8_f840, (f(s[0]) * sixteenth).store());
        put(mem, 0x00c8_f844, (f(s[1]) * sixteenth).store());
        put(mem, 0x00c8_f848, (f(s[2]) * sixteenth).store());
        let s3 = f32::from_bits(x87::store(f64::from(s[3])));
        put(mem, 0x00c8_f84c, sixteenth.mul_m32(s3).store());
        let pos = mem.read::<[u32; 4]>(o.wrapping_add(0x50));
        let at = reg(mem, 0x00c8_f980);
        mem.write(at, pos);
        let r4 = Reg::load(get(mem, 0x00c0_a004));
        put(mem, 0x00c8_f880, r4.store());
        let w = Reg::load(get(mem, 0x00c8_f98c));
        let r8 = Reg::load(get(mem, 0x00c0_a008));
        let c = 0x3f35_04f3u32;
        let at = reg(mem, 0x00c8_f528);
        mem.write(at, c);
        let at = reg(mem, 0x00c8_f890);
        mem.write(at, [c, c]);
        let v530 = mem.read::<u32>(mem.rebase(0x00c8_f530));
        let at = reg(mem, 0x00c8_f898);
        mem.write(at, v530);
        put(mem, 0x00c8_f884, (w + r8).store());
        let v534 = mem.read::<u32>(mem.rebase(0x00c8_f534));
        let z = Reg::load(get(mem, 0x00c8_f988));
        let at = reg(mem, 0x00c8_f52c);
        mem.write(at, c);
        let at = reg(mem, 0x00c8_f89c);
        mem.write(at, v534);
        put(mem, 0x00c8_f8a0, (r4 + z).store());
        put(mem, 0x00c8_f8a4, r8.store());
        let x = Reg::load(get(mem, 0x00c8_f7b0));
        let k528 = get(mem, 0x00c8_f528);
        put(mem, 0x00c8_f890, x.mul_m32(k528).store());
        let c894 = Reg::load(get(mem, 0x00c8_f894));
        let y = Reg::load(get(mem, 0x00c8_f7b4));
        put(mem, 0x00c8_f894, (c894 * y).store());
        let sc = mem.read::<[u32; 4]>(o.wrapping_add(0x30));
        let at = reg(mem, 0x00c8_f7a0);
        mem.write(at, sc);
        put(mem, 0x00c8_f7a0, x.mul_m32(f32::from_bits(sc[0])).store());
        put(
            mem,
            0x00c8_f7a4,
            (Reg::load(f32::from_bits(sc[1])) * y).store(),
        );
        let zz = Reg::load(get(mem, 0x00c8_f7a8));
        put(mem, 0x00c8_f8b4, (zz * y).store());
        put(mem, 0x00c8_f8b0, (zz * x).store());
        let cy = Reg::load(get(mem, 0x00c8_f894));
        put(mem, 0x00c8_f8c4, (zz * cy).store());
        let cx = Reg::load(get(mem, 0x00c8_f890));
        put(mem, 0x00c8_f8c0, (zz * cx).store());
        let a = Reg::load(get(mem, 0x00c8_f8a0));
        put(mem, 0x00c8_f890, (cx * a).store());
        let bb = Reg::load(get(mem, 0x00c8_f884));
        put(mem, 0x00c8_f894, (cy * bb).store());
        put(mem, 0x00c8_f884, (y * bb).store());
        put(mem, 0x00c8_f8a0, (x * a).store());
        let k = mem.read::<u32>(mem.rebase(super::SPR_PRE_CONSTS[2]));
        let at = reg(mem, 0x00c8_f950);
        mem.write(at, [x87::copy(k), 0, 0, 0]);
        let big = Reg::constant(mem.read::<f64>(mem.rebase(super::SPR_PRE_CONSTS[3])));
        put(mem, 0x00c8_f954, (r8 + big).store());
        let r10 = Reg::load(get(mem, 0x00c0_a010));
        put(mem, 0x00c8_f938, r10.add_m32(get(mem, 0x00c0_a00c)).store());
        #[allow(clippy::eq_op)] // fsub st(0), st: 0, or NaN for infinities
        put(mem, 0x00c8_f97c, (r10 - r10).store());
        free(mem);
    }

    /// `KeAddScaledColor(out, base, step, k)` (`0x00764c40`): each of the
    /// four channels `step * k + base`, stored as `f32`, then clamped to
    /// 0.0..=255.0 (NaN kept).
    pub fn ke_add_scaled_color(mem: &mut impl Mem, out: u32, base: u32, step: u32, k: u32) {
        let k = ffx_bridge::x87::load(k);
        for i in 0..4u32 {
            let d = ffx_bridge::x87::load(mem.read(step.wrapping_add(4 * i)));
            let b = ffx_bridge::x87::load(mem.read(base.wrapping_add(4 * i)));
            let v = x87::store(d * k + b);
            let at = out.wrapping_add(4 * i);
            mem.write(at, v);
            let f = f32::from_bits(v);
            if f > 255.0 {
                mem.write(at, 255.0f32.to_bits());
            } else if f < 0.0 {
                mem.write(at, 0u32);
            }
        }
    }

    // ---- round 12: random vectors, the pmcom environment, KeThBP_Init ----

    /// `KeVecAddRandom(dst, range, scaled)` (`0x00766f10`): for x, y, z,
    /// `dst[i] += op_random(range[i])`, times `op_random_p(1.0)` when
    /// `scaled` (the first draw rounded to `f64`, the sum at 53 bits).
    pub fn ke_vec_add_random<M: Mem, F: FnMut(&mut M, u32, FArgs<'_>) -> f64>(
        mem: &mut M,
        dst: u32,
        range: u32,
        scaled: u32,
        fcall: &mut F,
    ) {
        let (rnd, rnd_p) = (mem.rebase(super::OP_RANDOM), mem.rebase(super::OP_RANDOM_P));
        for i in 0..3u32 {
            let r = x87::copy(mem.read(range.wrapping_add(4 * i)));
            let v = if scaled == 0 {
                fcall(mem, rnd, FArgs::Stack(&[r]))
            } else {
                let a = fcall(mem, rnd, FArgs::Stack(&[r]));
                let b = fcall(mem, rnd_p, FArgs::Stack(&[1.0f32.to_bits()]));
                b * a
            };
            let at = dst.wrapping_add(4 * i);
            let d = x87::load(mem.read(at));
            mem.write(at, x87::store(v + d));
        }
    }

    /// `KeVecScaleRandom(dst, v, pair)` (`0x00766f70`): `s = r + r` for
    /// one `op_random(1.0)` draw `r` (`f32`), or `r + op_random(1.0)` when
    /// `pair`; then `dst[i] += v[i] * s - v[i]` for x, y, z. Returns
    /// `dst`.
    pub fn ke_vec_scale_random<M: Mem, F: FnMut(&mut M, u32, FArgs<'_>) -> f64>(
        mem: &mut M,
        dst: u32,
        v: u32,
        pair: u32,
        fcall: &mut F,
    ) -> u32 {
        let rnd = mem.rebase(super::OP_RANDOM);
        let one = 1.0f32.to_bits();
        let r = x87::store(fcall(mem, rnd, FArgs::Stack(&[one])));
        let s = if pair == 0 {
            x87::load(r) + x87::load(r)
        } else {
            fcall(mem, rnd, FArgs::Stack(&[one])) + x87::load(r)
        };
        let s = x87::load(x87::store(s));
        for i in 0..3u32 {
            let vi = x87::load(mem.read(v.wrapping_add(4 * i)));
            let at = dst.wrapping_add(4 * i);
            let d = x87::load(mem.read(at));
            mem.write(at, x87::store(s * vi - vi + d));
        }
        dst
    }

    /// `pppFreeAllEnvObjects()` (`0x007605d0`): for each of the
    /// [`super::PPP_OBJECT_COUNT`] object blocks (0x80 bytes from
    /// [`super::PPP_OBJECTS`]), the environment slot set to the pmcom
    /// environment and `pppAllFreePObject(block)`.
    pub fn ppp_free_all_env_objects<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) {
        let count = mem.rebase(super::PPP_OBJECT_COUNT);
        let mut block = mem.rebase(super::PPP_OBJECTS);
        let env = mem.rebase(super::PMCOM_ENV);
        let f = mem.rebase(super::PPP_ALL_FREE_POBJECT);
        let mut i = 0i32;
        while i < mem.read::<i32>(count) {
            let slot = mem.read::<u32>(mem.rebase(super::VFX_DATA));
            mem.write(slot, env);
            call(mem, f, &[block]);
            i += 1;
            block = block.wrapping_add(0x80);
        }
    }

    /// `pppInitPmcomEnv()` (`0x00760610`): the pmcom free pointers
    /// (`0x00d524a0` to `0x00d524a8`, the arena's next to its base) and
    /// `pppInitEnv(env, 0, heap, 0x100000)`; returns that.
    pub fn ppp_init_pmcom_env<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) -> u32 {
        let (env, heap) = (mem.rebase(super::PMCOM_ENV), mem.rebase(super::PMCOM_HEAP));
        let at = mem.rebase(super::PMCOM_FREE);
        let v = mem.rebase(super::PMCOM_FREE + 8);
        mem.write(at, v);
        let base = mem.rebase(super::PMCOM_BASE);
        super::PMCOM_NEXT.set(mem, base);
        let f = mem.rebase(super::PPP_INIT_ENV);
        call(mem, f, &[env, 0, heap, 0x10_0000])
    }

    /// `KeResampleRgbaToRgb(src, sw, sh, dst, dw, dh)` (`0x00760640`):
    /// nearest-neighbour scaling of a `sw` x `sh` RGBA image to `dw` x
    /// `dh` BGR bytes, the destination filled bottom row first, by
    /// Bresenham error terms (half the source size to start). Returns the
    /// final row error.
    #[allow(clippy::many_single_char_names)] // the original's terms
    pub fn ke_resample_rgba_to_rgb(
        mem: &mut impl Mem,
        src: u32,
        sw: i32,
        sh: i32,
        dst: u32,
        dw: i32,
        dh: i32,
    ) -> u32 {
        let mut ey = sh >> 1;
        if sh <= 0 {
            return ey.cast_unsigned();
        }
        let mut row = dh.wrapping_sub(1).wrapping_mul(dw);
        let mut line = 0i32;
        for _ in 0..sh {
            ey = ey.wrapping_sub(dh);
            if ey < 0 {
                ey = ey.wrapping_add(sh);
                row = row.wrapping_sub(dw);
                let mut ex = sw >> 1;
                let mut x = 0i32;
                for i in 0..sw.max(0) {
                    ex = ex.wrapping_sub(dw);
                    if ex < 0 {
                        x = x.wrapping_add(1);
                        ex = ex.wrapping_add(sw);
                        let s =
                            src.wrapping_add(line.wrapping_add(i).wrapping_mul(4).cast_unsigned());
                        let d =
                            dst.wrapping_add(row.wrapping_add(x).wrapping_mul(3).cast_unsigned());
                        let p = mem.read::<[u8; 3]>(s);
                        mem.write(d.wrapping_add(2), p[0]);
                        mem.write(d.wrapping_add(1), p[1]);
                        mem.write(d, p[2]);
                    }
                }
            }
            line = line.wrapping_add(sw);
        }
        ey.cast_unsigned()
    }

    /// `KeThBP_Init(args)` (`0x00763030`): the object at `args[0]` with
    /// `args[1]`'s byte as its segment count `n` (`+0x37`): itself at
    /// `+0x10`, its links, counters and `+0x18` cleared, `+0x4c` 1.0,
    /// `+0x23` 0x70, `+0x20` `3n + 2`, `+0x28` 0x40000b0, and a GIF tag
    /// (0, 0x11000000, 0x14000000, 0x11000000) at `+0x40 + 0x30n`.
    /// Returns `6n`.
    pub fn ke_th_bp_init(mem: &mut impl Mem, args: u32) -> u32 {
        let o = mem.read::<u32>(args);
        mem.write(o.wrapping_add(0x3e), 0u16);
        mem.write(o.wrapping_add(0x10), ptr32(o));
        mem.write(o.wrapping_add(0x18), 0f32);
        let n = mem.read::<u8>(args.wrapping_add(4));
        mem.write(o.wrapping_add(0x37), n);
        mem.write(o.wrapping_add(0x1c), 0u16);
        mem.write(o.wrapping_add(0x34), 0u16);
        mem.write(o.wrapping_add(0x36), 0u8);
        mem.write(o.wrapping_add(0x3c), 0u16);
        mem.write(o.wrapping_add(0x38), 0u16);
        mem.write(o.wrapping_add(0x1e), 0u8);
        for off in [0x14u32, 0x4, 0x0, 0xc, 0x8, 0x58] {
            mem.write(o.wrapping_add(off), ptr32(0));
        }
        mem.write(o.wrapping_add(0x4c), 1f32);
        mem.write(o.wrapping_add(0x23), 0x70u8);
        let n = u32::from(mem.read::<u8>(o.wrapping_add(0x37)));
        #[allow(clippy::cast_possible_truncation)] // the low half
        mem.write(o.wrapping_add(0x20), (n * 3 + 2) as u16);
        mem.write(o.wrapping_add(0x28), 0x0400_00b0u32);
        let n = u32::from(mem.read::<u8>(o.wrapping_add(0x37)));
        let tag = o.wrapping_add(0x40 + n * 0x30);
        mem.write(tag, [0u32, 0x1100_0000, 0x1400_0000, 0x1100_0000]);
        n * 6
    }

    // ---- round 15: the lens shapes' render data ----

    /// The work layout of the lens `*_Draw` functions (the original's
    /// frame from `ebp - 0x90`): the shape pair `KeLnsShp_Update` takes
    /// (`+0`, `+4`), the render description the renderer gets (`+0x10`:
    /// flags `+0x12`, colour `+0x14` with its alpha byte `+0x17`, the
    /// matrix pointer `+0x20`) and the matrix (`+0x48`).
    pub mod lns_work {
        /// Bytes.
        pub const SIZE: u32 = 0x88;
        /// The render description.
        pub const DESC: u32 = 0x10;
        /// The matrix.
        pub const MATRIX: u32 = 0x48;
    }

    /// The lens object's renderer instance key: the shape's program byte
    /// (`+0xb`, sign-extended) as the low word, the instance (`e[0]`) plus
    /// its sign as the high word.
    fn lns_instance(mem: &impl Mem, e: u32) -> (u32, u32) {
        let shape = ptr32(mem.read::<u32>(e.wrapping_add(4)));
        let prog = i32::from(mem.read::<i8>(shape.wrapping_add(0xb)));
        let inst = ptr32(mem.read::<u32>(e));
        (
            prog.cast_unsigned(),
            inst.wrapping_add((prog >> 31).cast_unsigned()),
        )
    }

    /// Stamps the current VFX program into the shape (`+0xb`) unless an
    /// ordering-table pass is drawing.
    fn lns_stamp_program(mem: &mut impl Mem, e: u32) {
        if mem.read::<u32>(mem.rebase(super::PPP_OT_DRAWING)) == 0 {
            let shape = ptr32(mem.read::<u32>(e.wrapping_add(4)));
            let p = mem.read::<u8>(mem.rebase(super::PPP_CURRENT_PROGRAM));
            mem.write(shape.wrapping_add(0xb), p);
        }
    }

    /// The shared middle: the colour (`e[2]`) with `alpha` as its top
    /// byte, the shape pair (`shape[1]`, `shape + 8`) and its clock
    /// (`KeLnsShp_Update`), the description's matrix pointer, flags and
    /// cleared words.
    fn lns_describe(mem: &mut impl Mem, e: u32, flags: u16, alpha: u8, w: u32) {
        mem.write(w.wrapping_add(0x17), alpha);
        let shape = ptr32(mem.read::<u32>(e.wrapping_add(4)));
        let data = mem.read::<u32>(shape.wrapping_add(4));
        mem.write(w, data);
        mem.write(w.wrapping_add(4), ptr32(shape.wrapping_add(8)));
        ke_lns_shp_update(mem, w);
        mem.write(w.wrapping_add(0x20), w.wrapping_add(lns_work::MATRIX));
        mem.write(w.wrapping_add(0x12), flags);
        mem.write(w.wrapping_add(0x2c), 0u32);
        mem.write(w.wrapping_add(0x18), 0u32);
        mem.write(w.wrapping_add(0x1c), 0u32);
    }

    /// The shared end: with render data for the instance, its update
    /// (`updateVFXRenderDataShp(lo, hi, data, desc, frame)`,
    /// `graphicVFXUpdate`); without, `InitVFXRenderDataShp` with the
    /// frame's alpha function (`calculateVFXSP_AlphaFunction`) | 0x800,
    /// | 0x40 for flag bit 3.
    fn lns_submit<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        (lo, hi): (u32, u32),
        rd: u32,
        w: u32,
        call: &mut C,
    ) {
        let desc = w.wrapping_add(lns_work::DESC);
        let frame = |mem: &M| {
            u32::from(mem.read::<u8>(ptr32(mem.read::<u32>(w.wrapping_add(4))).wrapping_add(2)))
        };
        if rd != 0 {
            let b2 = frame(mem);
            let data = ptr32(mem.read::<u32>(w));
            let f = mem.rebase(super::UPDATE_VFX_RENDER_DATA_SHP);
            call(mem, f, &[lo, hi, data, desc, b2]);
            let f = mem.rebase(super::GRAPHIC_VFX_UPDATE);
            call(mem, f, &[lo, hi]);
            return;
        }
        let data = ptr32(mem.read::<u32>(w));
        let b2 = frame(mem);
        let first = ptr32(mem.read::<u32>(data));
        let rec = first.wrapping_add(b2 * 8);
        let off = i32::from(mem.read::<i16>(rec.wrapping_add(0x10)));
        let e = first.wrapping_add(off.cast_unsigned());
        let mode = mem.read::<u8>(
            mem.read::<u32>(e.wrapping_add(4))
                .wrapping_add(e)
                .wrapping_add(4),
        );
        let f = mem.rebase(super::CALCULATE_VFX_SP_ALPHA_FUNCTION);
        let mut af = call(mem, f, &[u32::from(mode)]) | 0x800;
        if mem.read::<u8>(w.wrapping_add(0x12)) & 8 != 0 {
            af |= 0x40;
        }
        let b2 = frame(mem);
        let data = ptr32(mem.read::<u32>(w));
        let f = mem.rebase(super::INIT_VFX_RENDER_DATA_SHP);
        call(mem, f, &[lo, hi, data, desc, b2, af]);
    }

    /// The shape's data when it can draw: not the default shapes, with
    /// a second word, and its source (`shape[0]`).
    fn lns_source(mem: &impl Mem, e: u32) -> Option<u32> {
        let shape = ptr32(mem.read::<u32>(e.wrapping_add(4)));
        let src = ptr32(mem.read::<u32>(shape));
        if src == mem.rebase(super::LNS_DEFAULT)
            || is_ptr32_null(mem.read::<u32>(shape.wrapping_add(4))) != 0
        {
            return None;
        }
        Some(src)
    }

    /// The matrix of a lens source `s`: `s` itself (`+0x8f` set,
    /// `FFXVu0CopyMatrix`), else the unit matrix moved to `s +0x80` plus
    /// the VFX offset ([`super::VFX_OFFSET`]).
    fn lns_matrix<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        s: u32,
        m: u32,
        call: &mut C,
    ) {
        if mem.read::<u8>(s.wrapping_add(0x8f)) != 0 {
            let f = mem.rebase(super::VU0_COPY_MATRIX);
            call(mem, f, &[m, s]);
        } else {
            let f = mem.rebase(super::VU0_UNIT_MATRIX);
            call(mem, f, &[m]);
            lns_translate(mem, s, m);
        }
    }

    /// Row 3 of `m` = `s +0x80` plus the VFX offset.
    fn lns_translate(mem: &mut impl Mem, s: u32, m: u32) {
        let off = mem.rebase(super::VFX_OFFSET);
        for i in 0..3u32 {
            let a = x87::load(mem.read(s.wrapping_add(0x80 + 4 * i)));
            let b = x87::load(mem.read(off.wrapping_add(4 * i)));
            mem.write(m.wrapping_add(0x30 + 4 * i), x87::store(a + b));
        }
    }

    /// `FFXVu0ScaleVector(row, row, k)` for the first `n` rows of `m`, `k`
    /// from the scale vector at `e[off / 4]`.
    fn lns_scale_rows<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        e: u32,
        off: u32,
        m: u32,
        n: u32,
        call: &mut C,
    ) {
        let f = mem.rebase(super::VU0_SCALE_VECTOR);
        for i in 0..n {
            let sv = ptr32(mem.read::<u32>(e.wrapping_add(off)));
            let k = x87::copy(mem.read(sv.wrapping_add(4 * i)));
            let row = m.wrapping_add(0x10 * i);
            call(mem, f, &[row, row, k]);
        }
    }

    /// The colour's alpha byte times `x` (an `f64`), truncated (the x87
    /// `fistp` under round-toward-zero: out of range gives 0).
    fn lns_alpha(mem: &mut impl Mem, e: u32, w: u32, x: impl FnOnce(f64) -> f64) -> u8 {
        let c = mem.read::<u32>(e.wrapping_add(8));
        mem.write(w.wrapping_add(0x14), c);
        let v = ffx_bridge::x87::cvttsd2si(x(f64::from(c >> 24)));
        v.to_le_bytes()[0]
    }

    /// `KeLnsArnd_Draw(e, flags, work)` (`0x00769340`): a lens "around"
    /// shape's render data. The source's matrix ([`lns_matrix`]) with
    /// x of rows 0 and 1 times `e +0x10`, the three rows scaled by `e
    /// +0x14`'s vector, alpha `colour >> 24 * s +0x7c`; nothing for a
    /// source alpha of 0.
    pub fn ke_lns_arnd_draw<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        e: u32,
        flags: u16,
        w: u32,
        call: &mut C,
    ) {
        let Some(s) = lns_source(mem, e) else { return };
        let a = x87::load(mem.read(s.wrapping_add(0x7c)));
        if a == 0.0 {
            return;
        }
        let m = w.wrapping_add(lns_work::MATRIX);
        lns_matrix(mem, s, m, call);
        for o in [0u32, 0x14] {
            let k = x87::load(mem.read(e.wrapping_add(0x10)));
            let v = x87::load(mem.read(m.wrapping_add(o)));
            mem.write(m.wrapping_add(o), x87::store(k * v));
        }
        lns_scale_rows(mem, e, 0x14, m, 3, call);
        let f7c = x87::load(mem.read(s.wrapping_add(0x7c)));
        let alpha = lns_alpha(mem, e, w, |c| c * f7c);
        lns_describe(mem, e, flags, alpha, w);
        if is_ptr32_null(mem.read::<u32>(e)) != 0 {
            return;
        }
        lns_stamp_program(mem, e);
        let key = lns_instance(mem, e);
        let f = mem.rebase(super::GRAPHIC_VFX_GET_RENDER_DATA_BY_INSTANCE);
        let rd = call(mem, f, &[key.0, key.1]);
        lns_submit(mem, key, rd, w, call);
    }

    /// `KeLnsCrn_Draw(e, flags, work)` (`0x00769d70`): a lens "corona".
    /// Rows from the source's vectors (`+0x40`, `+0x50`, `+0x60`, the
    /// position `+0x30`), rows 0 and 1 scaled by `-k` and `k` for `k = e
    /// +0x10 * s +0xa4 / e +0x14` (`u16`), the position moved as in
    /// [`lns_matrix`] unless `+0x8f`, the rows scaled by `e +0x18`'s
    /// vector, alpha `colour >> 24 * s +0x7c * min(s +0xa4 / e +0x16,
    /// 1)`.
    pub fn ke_lns_crn_draw<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        e: u32,
        flags: u16,
        w: u32,
        call: &mut C,
    ) {
        let Some(s) = lns_source(mem, e) else { return };
        if x87::load(mem.read(s.wrapping_add(0x7c))) == 0.0 {
            return;
        }
        let m = w.wrapping_add(lns_work::MATRIX);
        let f = mem.rebase(super::VU0_COPY_VECTOR);
        for (row, src) in [(0u32, 0x40u32), (0x10, 0x50), (0x20, 0x60), (0x30, 0x30)] {
            call(mem, f, &[m.wrapping_add(row), s.wrapping_add(src)]);
        }
        let n = f64::from(mem.read::<u16>(e.wrapping_add(0x14)));
        let k =
            x87::load(mem.read(e.wrapping_add(0x10))) * x87::load(mem.read(s.wrapping_add(0xa4)));
        let k = x87::store(k / n);
        let f = mem.rebase(super::VU0_SCALE_VECTOR);
        call(mem, f, &[m, m, x87::store(-x87::load(k))]);
        call(mem, f, &[m.wrapping_add(0x10), m.wrapping_add(0x10), k]);
        if mem.read::<u8>(s.wrapping_add(0x8f)) == 0 {
            lns_translate(mem, s, m);
        }
        lns_scale_rows(mem, e, 0x18, m, 3, call);
        let n = f64::from(mem.read::<u16>(e.wrapping_add(0x16)));
        let r = x87::store(x87::load(mem.read(s.wrapping_add(0xa4))) / n);
        let r = if x87::load(r) > 1.0 {
            1.0
        } else {
            x87::load(r)
        };
        let f7c = x87::load(mem.read(s.wrapping_add(0x7c)));
        let alpha = lns_alpha(mem, e, w, |c| c * f7c * r);
        lns_describe(mem, e, flags, alpha, w);
        if is_ptr32_null(mem.read::<u32>(e)) != 0 {
            return;
        }
        lns_stamp_program(mem, e);
        let key = lns_instance(mem, e);
        let f = mem.rebase(super::GRAPHIC_VFX_GET_RENDER_DATA_BY_INSTANCE);
        let rd = call(mem, f, &[key.0, key.1]);
        lns_submit(mem, key, rd, w, call);
    }

    /// `KeLnsFls_Draw(e, flags, work)` (`0x0076a1f0`): a lens flash. The
    /// program stamp and the render data lookup come first; a shape that
    /// cannot draw, or a source alpha of 0, hides existing render data
    /// (`graphicVFXSetVisible(.., 0)`). Else `f = s +0xa4 / e +0x14`
    /// (`u16`), `g = 1 - f` (0 past 1); row 0's x is `e +0x10`, times `g`
    /// (mode `e +0x16` 1) or `f + 1` (2), row 1's y likewise by `e +0x17`;
    /// rows 0 and 1 scaled by `e +0x18`'s vector; alpha `colour >> 24 *
    /// s +0x7c * g`.
    pub fn ke_lns_fls_draw<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        e: u32,
        flags: u16,
        w: u32,
        call: &mut C,
    ) {
        lns_stamp_program(mem, e);
        let key = lns_instance(mem, e);
        let f = mem.rebase(super::GRAPHIC_VFX_GET_RENDER_DATA_BY_INSTANCE);
        let rd = call(mem, f, &[key.0, key.1]);
        let hide = |mem: &mut M, call: &mut C| {
            if is_ptr32_null(mem.read::<u32>(e)) == 0 && rd != 0 {
                let f = mem.rebase(super::GRAPHIC_VFX_SET_VISIBLE);
                call(mem, f, &[key.0, key.1, 0]);
            }
        };
        let Some(s) = lns_source(mem, e) else {
            hide(mem, call);
            return;
        };
        if x87::load(mem.read(s.wrapping_add(0x7c))) == 0.0 {
            hide(mem, call);
            return;
        }
        let n = f64::from(mem.read::<u16>(e.wrapping_add(0x14)));
        let fv = x87::store(x87::load(mem.read(s.wrapping_add(0xa4))) / n);
        let f = x87::load(fv);
        let g = x87::load(x87::store(if f > 1.0 { 0.0 } else { 1.0 - f }));
        let m = w.wrapping_add(lns_work::MATRIX);
        lns_matrix(mem, s, m, call);
        for (o, mode_at) in [(0u32, 0x16u32), (0x14, 0x17)] {
            let x = x87::copy(mem.read(e.wrapping_add(0x10)));
            mem.write(m.wrapping_add(o), x);
            let k = match mem.read::<u8>(e.wrapping_add(mode_at)) {
                1 => g,
                2 => f + 1.0,
                _ => continue,
            };
            let v = x87::load(mem.read(m.wrapping_add(o)));
            mem.write(m.wrapping_add(o), x87::store(v * k));
        }
        lns_scale_rows(mem, e, 0x18, m, 2, call);
        let f7c = x87::load(mem.read(s.wrapping_add(0x7c)));
        let alpha = lns_alpha(mem, e, w, |c| c * f7c * g);
        lns_describe(mem, e, flags, alpha, w);
        if is_ptr32_null(mem.read::<u32>(e)) != 0 {
            return;
        }
        lns_submit(mem, key, rd, w, call);
    }

    // ---- round 16: KeMdl_Draw ----

    /// The work layout of `KeMdl_Draw` (the original's frame from `ebp -
    /// 0xe0`): the render description the renderer gets (`+0`, 0x48
    /// bytes) and the view matrix (`+0x98`).
    pub mod mdl_work {
        /// Bytes.
        pub const SIZE: u32 = 0xd8;
        /// The view matrix.
        pub const MATRIX: u32 = 0x98;
    }

    /// The emulated EE registers' words at `0x00c8f500 + off`.
    fn ee(mem: &impl Mem, off: u32) -> u32 {
        mem.rebase(super::EE_REGS + off)
    }

    /// `KeMdl_Draw(e, rec, col, p4, p5, p6, mode, zmat, _, p10, work)`
    /// (`0x00766ff0`): a VFX model's render data. The colour: the two
    /// words at `col + g_pppColorConShift` and the current part's `+0x70`
    /// words are interleaved halfword-wise with the EE register at
    /// `0x00c8f508` into registers `0x00c8f588` and `0x00c8f598`,
    /// multiplied (`pppEePmulth`), shifted down by 19 bits (13 kept),
    /// interleaved again and packed to bytes (`pppEePpacb`); a colour whose
    /// top byte is 0 draws nothing. Else the description: flags from
    /// `mode` and the record's shape flags (`rec`'s `+0x1c` byte 0), the
    /// view matrix (`g_pppViewMatrix * e +0x10`, mode bit 4), the colour,
    /// the record's alpha byte (`+0x16`), `p4`..`p6`, `e`'s `+0x50`,
    /// `+0x10`, `+0x90`/`+0x94`, the packet position, `zmat` with mode
    /// bit 5; then `updateVFXRenderData` and the (path 3) update with
    /// render data, else `InitVFX(Path3)RenderData` with the alpha
    /// function, Z mode (unless bit 4) | 0x40 (flag 8) | 0x80; and the
    /// CLUT parameters (`0x010d5600`).
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)] // the original's steps
    pub fn ke_mdl_draw<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        args: [u32; 10],
        w: u32,
        call: &mut C,
    ) {
        let [e, rec_index, col, p4, p5, p6, mode, zmat, _, p10] = args;
        let shift = mem.rebase(super::PPP_COLOR_CON_SHIFT);
        let at = mem.read::<u32>(shift).wrapping_add(col);
        let d = mem.read::<u32>(at);
        mem.write(ee(mem, 0x88), d);
        let c = mem.read::<u32>(at.wrapping_add(4));
        let part = mem.read::<u32>(mem.rebase(super::PPP_CURRENT_PART));
        mem.write(ee(mem, 0x8c), c);
        mem.write(shift, 0u32);
        let p70 = mem.read::<u32>(part.wrapping_add(0x70));
        mem.write(ee(mem, 0x98), p70);
        let p74 = mem.read::<u32>(part.wrapping_add(0x74));
        let lo = |v: u32| (v & 0xffff) as u16;
        let hi = |v: u32| (v >> 16) as u16;
        let a0 = mem.read::<u32>(ee(mem, 0x08));
        mem.write(ee(mem, 0x8a), lo(a0));
        mem.write(ee(mem, 0x8e), hi(a0));
        let a1 = mem.read::<u32>(ee(mem, 0x0c));
        mem.write(ee(mem, 0x92), lo(a1));
        mem.write(ee(mem, 0x96), hi(a1));
        mem.write(ee(mem, 0x8c), hi(d));
        mem.write(ee(mem, 0x94), hi(c));
        mem.write(ee(mem, 0x88), lo(d));
        let a0 = mem.read::<u32>(ee(mem, 0x08));
        mem.write(ee(mem, 0x90), lo(c));
        let a1 = mem.read::<u32>(ee(mem, 0x0c));
        mem.write(ee(mem, 0x9c), p74);
        mem.write(ee(mem, 0x9e), hi(a0));
        mem.write(ee(mem, 0xa6), hi(a1));
        mem.write(ee(mem, 0x9c), hi(p70));
        mem.write(shift, 0u32);
        mem.write(ee(mem, 0x9a), lo(a0));
        mem.write(ee(mem, 0xa2), lo(a1));
        mem.write(ee(mem, 0x98), lo(p70));
        mem.write(ee(mem, 0xa0), lo(p74));
        mem.write(ee(mem, 0xa4), hi(p74));
        let (m, b, a) = (ee(mem, 0x88), ee(mem, 0x98), ee(mem, 0x08));
        let f = mem.rebase(super::PPP_EE_PMULTH);
        call(mem, f, &[m, m, b]);
        let top =
            |mem: &M, off: u32| (mem.read::<i32>(ee(mem, off)) >> 19).cast_unsigned() & 0x1fff;
        let s0 = top(mem, 0x88);
        let s1 = top(mem, 0x8c);
        let s3 = top(mem, 0x94);
        let s2 = top(mem, 0x90);
        mem.write(ee(mem, 0x94), s3);
        mem.write(ee(mem, 0x8c), s1);
        mem.write(ee(mem, 0x8e), lo(s3));
        let a0 = mem.read::<u32>(ee(mem, 0x08));
        mem.write(ee(mem, 0x90), s2);
        mem.write(ee(mem, 0x90), lo(a0));
        let a1 = mem.read::<u32>(ee(mem, 0x0c));
        mem.write(ee(mem, 0x92), lo(a1));
        let a2 = mem.read::<u32>(ee(mem, 0x10));
        mem.write(ee(mem, 0x94), lo(a2));
        let a3 = mem.read::<u32>(ee(mem, 0x14));
        mem.write(ee(mem, 0x88), s0);
        mem.write(ee(mem, 0x88), lo(s0));
        mem.write(ee(mem, 0x8a), lo(s1));
        mem.write(ee(mem, 0x8c), lo(s2));
        mem.write(ee(mem, 0x96), lo(a3));
        let f = mem.rebase(super::PPP_EE_PPACB);
        call(mem, f, &[m, a, m]);
        let sh = mem.read::<u32>(shift);
        let colour = mem.read::<u32>(ee(mem, 0x88));
        mem.write(w.wrapping_add(0x18).wrapping_add(sh), colour);
        let alpha = mem.read::<u8>(w.wrapping_add(0x1b));
        mem.write(shift, 0u32);
        if alpha == 0 {
            return;
        }
        // the description
        let env = mem.read::<u32>(mem.read::<u32>(mem.rebase(super::VFX_DATA)));
        let rec = rec_index
            .wrapping_shl(5)
            .wrapping_add(mem.read::<u32>(env.wrapping_add(0x20)));
        let pb = ptr32(mem.read::<u32>(rec.wrapping_add(0x1c)));
        let shape = mem.read::<u8>(pb);
        let mut fl = 0u32;
        if mode & 2 != 0 {
            fl = 4;
            if shape & 1 == 0 {
                fl = 0xc;
            }
        } else {
            if mode & 1 != 0 {
                fl = 0x80;
            }
            if shape & 1 == 0 {
                fl |= 8;
            }
        }
        if mem.read::<u8>(pb) & 2 == 0 {
            fl |= 0x10;
        }
        if mode & 4 != 0 {
            fl |= 0x200;
        }
        if mode & 8 != 0 {
            fl |= 0x100;
        }
        mem.write(w, fl);
        if mode & 0x10 != 0 {
            fl |= 0x2000;
            mem.write(w, fl);
            let mtx = w.wrapping_add(mdl_work::MATRIX);
            mem.write(w.wrapping_add(0x30), mtx);
            let view = mem.rebase(super::PPP_VIEW_MATRIX);
            let f = mem.rebase(super::VU0_MUL_MATRIX);
            call(mem, f, &[mtx, view, e.wrapping_add(0x10)]);
        }
        let z = if mode & 0x20 != 0 { zmat } else { 0 };
        mem.write(w.wrapping_add(0x44), z);
        let ab = mem.read::<u16>(pb.wrapping_add(0x16));
        mem.write(w.wrapping_add(8), ab);
        mem.write(w.wrapping_add(0x1c), p10);
        let r90 = ptr32(mem.read::<u32>(e.wrapping_add(0x90)));
        mem.write(w.wrapping_add(0x38), r90);
        let r94 = ptr32(mem.read::<u32>(e.wrapping_add(0x94)));
        mem.write(w.wrapping_add(0x3c), r94);
        let tag = mem.read::<u32>(mem.rebase(super::REF_GIF_TAG));
        mem.write(w.wrapping_add(4), tag);
        mem.write(w.wrapping_add(0xc), 0u16);
        #[allow(clippy::cast_possible_truncation)] // a u16 argument
        mem.write(w.wrapping_add(0xe), p4 as u16);
        mem.write(w.wrapping_add(0x10), p5 << 16);
        mem.write(w.wrapping_add(0x14), p6 << 16);
        mem.write(w.wrapping_add(0x20), e.wrapping_add(0x50));
        mem.write(w.wrapping_add(0x2c), e.wrapping_add(0x10));
        let prog = mem.read::<i32>(mem.rebase(super::PPP_CURRENT_PROGRAM));
        let key = [
            prog.cast_unsigned(),
            e.wrapping_add((prog >> 31).cast_unsigned()),
        ];
        let pos = mem.read::<u32>(mem.rebase(super::PACKET_POS_ADDR));
        mem.write(w.wrapping_add(0x40), 0u32);
        mem.write(w.wrapping_add(0xa), 0u8);
        let (fp, fq) = (
            mem.rebase(super::MDL_FP_DATA),
            mem.rebase(super::MDL_FQ_DATA),
        );
        mem.write(w.wrapping_add(0x24), fp);
        mem.write(w.wrapping_add(0x28), fq);
        mem.write(w.wrapping_add(0x34), pos);
        let f = mem.rebase(super::GRAPHIC_VFX_GET_RENDER_DATA_BY_INSTANCE);
        let rd = call(mem, f, &key);
        let path3 = |mem: &M| mem.read::<u8>(w) & 4 != 0;
        if rd != 0 {
            let f = mem.rebase(super::UPDATE_VFX_RENDER_DATA);
            call(mem, f, &[key[0], key[1], rec, w]);
            let f = mem.rebase(if path3(mem) {
                super::GRAPHIC_VFX_PATH3_UPDATE
            } else {
                super::GRAPHIC_VFX_UPDATE
            });
            call(mem, f, &key);
        } else {
            let ab = u32::from(mem.read::<u8>(w.wrapping_add(8)));
            let f = mem.rebase(super::CALCULATE_VFX_SP_ALPHA_FUNCTION);
            let mut fa = call(mem, f, &[ab]);
            if mem.read::<u32>(w) & 0x2000 == 0 {
                let z = mem.read::<u32>(w.wrapping_add(0x44));
                let f = mem.rebase(super::CALCULATE_VFX_SP_ZMAT);
                fa |= call(mem, f, &[z]);
            }
            let b = mem.read::<u8>(w);
            if b & 8 != 0 {
                fa |= 0x40;
            }
            fa |= 0x80;
            let f = mem.rebase(if b & 4 != 0 {
                super::INIT_VFX_PATH3_RENDER_DATA
            } else {
                super::INIT_VFX_RENDER_DATA
            });
            call(mem, f, &[key[0], key[1], rec, w, fa]);
        }
        let [c0, c1, c2] = crate::vu0::TK_CLUT.get(mem);
        let f = mem.rebase(super::GRAPHIC_VFX_SET_CLUT_PARAMETERS);
        call(mem, f, &[key[0], key[1], c0, c1, x87::copy(c2)]);
    }

    // ---- round 18 ----

    /// `KeThCp_AddBp(cp, pool, a, b)` (`0x007644d0`): takes a block off
    /// the pool's free list (`pool[1]`), sets it up (`KeThBP_Init` with
    /// the pool's size byte `+1`, through `work`, the original's 8-byte
    /// local), clears `+0x5c` and `+0x30`, ORs flags from `cp`'s byte `+4`
    /// into `+0x3e` (0x100, 0x180 with bit 2, | 0x200 bit 4, | 0x400 bit 3,
    /// | 0x1000 bit 5), copies `a` and `b` to `+0x40` and `+0x60`
    /// (`KeVu0CopyVecXYZ`) and pushes it on the front of `cp`'s list.
    /// Returns the block, 0 with an empty pool, 1 without a free list.
    pub fn ke_th_cp_add_bp(
        mem: &mut impl Mem,
        cp: u32,
        pool: u32,
        ab: (u32, u32),
        work: u32,
    ) -> u32 {
        let free = mem.read::<u32>(pool.wrapping_add(4));
        if is_ptr32_null(free) != 0 {
            return is_ptr32_null(free);
        }
        let o = ptr32(free);
        let next = mem.read::<u32>(ptr32(mem.read::<u32>(pool.wrapping_add(4))));
        mem.write(pool.wrapping_add(4), next);
        if o == 0 {
            return 0;
        }
        let size = mem.read::<u8>(pool.wrapping_add(1));
        mem.write(work.wrapping_add(4), size);
        mem.write(work, o);
        ke_th_bp_init(mem, work);
        mem.write(o.wrapping_add(0x5c), 0f32);
        mem.write(o.wrapping_add(0x30), 0f32);
        let b = mem.read::<u8>(cp.wrapping_add(4));
        let mut f = if b & 4 != 0 { 0x180u16 } else { 0x100 };
        if b & 0x10 != 0 {
            f |= 0x200;
        }
        if b & 8 != 0 {
            f |= 0x400;
        }
        if b & 0x20 != 0 {
            f |= 0x1000;
        }
        let v = mem.read::<u16>(o.wrapping_add(0x3e)) | f;
        mem.write(o.wrapping_add(0x3e), v);
        crate::vu0::bridge::ke_vu0_copy_vec_xyz(mem, o.wrapping_add(0x40), ab.0);
        crate::vu0::bridge::ke_vu0_copy_vec_xyz(mem, o.wrapping_add(0x60), ab.1);
        let head = mem.read::<u32>(cp);
        if is_ptr32_null(head) == 0 {
            mem.write(ptr32(head).wrapping_add(4), ptr32(o));
        }
        let head = mem.read::<u32>(cp);
        mem.write(o, head);
        mem.write(o.wrapping_add(4), ptr32(cp));
        mem.write(cp, ptr32(o));
        ptr32(o)
    }

    // ---- field round 32 ----

    /// `KeLnsLp_Update(lp, work, fcall)` (`0x0076a690`, from
    /// `pppKeLnsLpDraw`): the lens flare object's (`get_ptr32_func(lp[0])`,
    /// `o`) screen frame and fade. `work` holds the original's locals (0xc0
    /// bytes): the matrix `lp[2]` (`M`, `+0`), a work matrix (`W`, `+0x40`,
    /// its translation `+0x70`), `d` (`+0x80`), `v0` (`+0x90`), `v1`
    /// (`+0xa0`).
    /// - Object space (`o +0x8f` set): `v0 = W lp[1]` with `W` = `M` less
    ///   its translation, `v1` = -`M`'s translation; else world space: `v0`
    ///   = the flare's point (`o +0x80` + the VFX offset `0x0230ff50`) and
    ///   `v1` = the offset, each through a unit matrix with that
    ///   translation and divided by its `w`; `d.y` scaled by 0.9230769 and
    ///   `d.z` 512.0. `d = v1 - v0`.
    /// - `o` becomes the unit matrix with `+0x30` = `M lp[1]`; with `d.x` or
    ///   `d.y` non-zero (or NaN) its rows `+0x40`, `+0x50` the 2D rotation
    ///   by `d` (`/ f32(|d.xy|)`), `+0x60` (0, 0, 1, 0).
    /// - `+0x70..` copies `+0x30..`, `+0x78` = depth (`+0x38`, or the
    ///   flare's z plus the offset) / 512, `+0x70`/`+0x74` that times the
    ///   normalised `d`; `v1 -= v0`, `+0xa4` its xy length, `v0` its
    ///   direction.
    /// - The fade `+0x7c`: world space `1 - |xy| / +0xa8` (0 beyond, 0 for
    ///   `+0xa8 <= 0`); object space `1 - (1 - (1 - v0.z) / 2) / 0.3` (0
    ///   below), times `1 - (1 - (dir . v0)) / 2 / +0x9c` (0 below; `dir`
    ///   `+0x90` against `v1` through `W`) when `+0x9c >= 0`, times
    ///   `1 - length / (+0xa8 - +0x8e)` (0 beyond; the fade 0 when that
    ///   range is not positive). Every stored step is an `f32`.
    pub fn ke_lns_lp_update<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        lp: u32,
        work: u32,
        call: &mut C,
        mut fcall: impl FnMut(&mut M, u32, FArgs<'_>) -> f64,
    ) {
        use ffx_bridge::x87::{add, div, load, mul, round_trip, sqrt, store, sub};
        let st = |x: f64| load(store(x));
        let f32at = |mem: &M, at: u32| load(mem.read::<u32>(at));
        let c64 = |mem: &M, at: u32| mem.read::<f64>(mem.rebase(at));
        let (m, w, d, v0, v1) = (
            work,
            work.wrapping_add(0x40),
            work.wrapping_add(0x80),
            work.wrapping_add(0x90),
            work.wrapping_add(0xa0),
        );
        let wt = w.wrapping_add(0x30);
        let fns = |mem: &M, f: u32| mem.rebase(f);
        let f = fns(mem, super::GET_PTR32_FUNC);
        let o = call(mem, f, &[mem.read::<u32>(lp)]);
        let f = fns(mem, super::VU0_COPY_MATRIX);
        call(mem, f, &[m, mem.read::<u32>(lp.wrapping_add(8))]);
        let local = mem.read::<u8>(o.wrapping_add(0x8f)) != 0;
        let off = mem.rebase(super::VFX_OFFSET);
        if local {
            let f = fns(mem, super::VU0_COPY_MATRIX);
            call(mem, f, &[w, m]);
            for i in 0..3 {
                mem.write(wt.wrapping_add(4 * i), 0u32);
            }
            let f = fns(mem, super::VU0_APPLY_MATRIX);
            call(mem, f, &[v0, w, mem.read::<u32>(lp.wrapping_add(4))]);
            let k = round_trip(mem.read::<f32>(mem.rebase(super::LNS_NEG_ONE))).to_bits();
            let f = fns(mem, super::VU0_SCALE_VECTOR);
            call(mem, f, &[v1, m.wrapping_add(0x30), k]);
            let f = fns(mem, super::VU0_SUB_VECTOR);
            call(mem, f, &[d, v1, v0]);
        } else {
            for i in 0..3 {
                mem.write(v0.wrapping_add(4 * i), 0u32);
                mem.write(v1.wrapping_add(4 * i), 0u32);
            }
            mem.write(v0.wrapping_add(0xc), 1.0f32.to_bits());
            mem.write(v1.wrapping_add(0xc), 1.0f32.to_bits());
            let f = fns(mem, super::VU0_UNIT_MATRIX);
            call(mem, f, &[w]);
            for i in 0..3u32 {
                let v = add(
                    f32at(mem, o.wrapping_add(0x80 + 4 * i)),
                    f32at(mem, off.wrapping_add(4 * i)),
                );
                mem.write(wt.wrapping_add(4 * i), store(v));
            }
            let f = fns(mem, super::VU0_APPLY_MATRIX);
            call(mem, f, &[v0, w, v0]);
            let k = store(div(1.0, f32at(mem, v0.wrapping_add(0xc))));
            let f = fns(mem, super::VU0_SCALE_VECTOR);
            call(mem, f, &[v0, v0, k]);
            let f = fns(mem, super::VU0_UNIT_MATRIX);
            call(mem, f, &[w]);
            let f = fns(mem, super::VU0_COPY_VECTOR);
            call(mem, f, &[wt, off]);
            let f = fns(mem, super::VU0_APPLY_MATRIX);
            call(mem, f, &[v1, w, v1]);
            let k = store(div(1.0, f32at(mem, v1.wrapping_add(0xc))));
            let f = fns(mem, super::VU0_SCALE_VECTOR);
            call(mem, f, &[v1, v1, k]);
            let f = fns(mem, super::VU0_SUB_VECTOR);
            call(mem, f, &[d, v1, v0]);
            let y = store(mul(
                f32at(mem, d.wrapping_add(4)),
                c64(mem, super::LNS_Y_SCALE),
            ));
            mem.write(d.wrapping_add(4), y);
            let z = round_trip(mem.read::<f32>(mem.rebase(super::LNS_DEPTH))).to_bits();
            mem.write(d.wrapping_add(8), z);
        }
        let f = fns(mem, super::VU0_UNIT_MATRIX);
        call(mem, f, &[o]);
        let f = fns(mem, super::VU0_APPLY_MATRIX);
        call(
            mem,
            f,
            &[o.wrapping_add(0x30), m, mem.read::<u32>(lp.wrapping_add(4))],
        );
        let (dx, dy) = (f32at(mem, d), f32at(mem, d.wrapping_add(4)));
        // fucom: equal only when ordered and equal
        let nonzero = |v: f64| v.is_nan() || v != 0.0;
        if nonzero(dx) || nonzero(dy) {
            let r = st(sqrt(add(mul(dx, dx), mul(dy, dy))));
            let rows = [
                (0x40, store(div(dx, r))),
                (0x44, store(div(dy, r))),
                (0x48, 0),
                (0x4c, 0),
                (0x50, store(div(dy, r))),
                (0x54, store(div(-dx, r))),
                (0x58, 0),
                (0x5c, 0),
                (0x60, 0),
                (0x64, 0),
                (0x68, 1.0f32.to_bits()),
                (0x6c, 0),
            ];
            for (at, v) in rows {
                mem.write(o.wrapping_add(at), v);
            }
        }
        for i in 0..4u32 {
            let v = round_trip(mem.read::<f32>(o.wrapping_add(0x30 + 4 * i))).to_bits();
            mem.write(o.wrapping_add(0x70 + 4 * i), v);
        }
        let depth = if local {
            f32at(mem, o.wrapping_add(0x38))
        } else {
            add(
                f32at(mem, o.wrapping_add(0x88)),
                f32at(mem, off.wrapping_add(8)),
            )
        };
        mem.write(
            o.wrapping_add(0x78),
            store(mul(depth, c64(mem, super::LNS_DEPTH_SCALE))),
        );
        let f = fns(mem, super::VU0_NORMALIZE);
        call(mem, f, &[d, d]);
        let dep = f32at(mem, o.wrapping_add(0x78));
        let x = store(mul(dep, f32at(mem, d)));
        mem.write(o.wrapping_add(0x70), x);
        let dep = f32at(mem, o.wrapping_add(0x78));
        let y = store(mul(dep, f32at(mem, d.wrapping_add(4))));
        mem.write(o.wrapping_add(0x74), y);
        let f = fns(mem, super::VU0_SUB_VECTOR);
        call(mem, f, &[v1, v1, v0]);
        let f = fns(mem, super::KE_VU0_VEC_LEN);
        let len = st(fcall(mem, f, FArgs::Stack(&[v1])));
        let f = fns(mem, super::KE_VU0_VEC_LEN_XY);
        let xy = store(fcall(mem, f, FArgs::Stack(&[v1])));
        mem.write(o.wrapping_add(0xa4), xy);
        let f = fns(mem, super::VU0_NORMALIZE);
        call(mem, f, &[v0, v1]);
        let fade = o.wrapping_add(0x7c);
        let a8 = f32at(mem, o.wrapping_add(0xa8));
        if !local {
            // the C0/C2 test: on unless 0 < a8
            if !(0.0 < a8) {
                mem.write(fade, 0u32);
                return;
            }
            let t = st(div(f32at(mem, o.wrapping_add(0xa4)), a8));
            let v = if 1.0 < t { 0 } else { store(sub(1.0, t)) };
            mem.write(fade, v);
            return;
        }
        let half = c64(mem, super::LNS_HALF);
        // the angle term
        mem.write(v1, 0u32);
        mem.write(v1.wrapping_add(4), 0u32);
        mem.write(v1.wrapping_add(8), 1.0f32.to_bits());
        let f = fns(mem, super::VU0_INNER_PRODUCT);
        let ip = fcall(mem, f, FArgs::Stack(&[v1, v0]));
        let h = mul(sub(1.0, ip), half);
        let q = st(sub(1.0, h));
        let u = st(div(q, c64(mem, super::LNS_CONE)));
        let g = if 1.0 < u { 0 } else { store(sub(1.0, u)) };
        mem.write(fade, g);
        // the direction term
        let w9c = f32at(mem, o.wrapping_add(0x9c));
        if !(w9c < 0.0 || w9c.is_nan()) {
            let f = fns(mem, super::VU0_COPY_MATRIX);
            call(mem, f, &[w, m]);
            for i in 0..3 {
                mem.write(wt.wrapping_add(4 * i), 0u32);
            }
            let f = fns(mem, super::VU0_APPLY_MATRIX);
            call(mem, f, &[v1, w, v1]);
            let f = fns(mem, super::VU0_INNER_PRODUCT);
            let ip2 = fcall(mem, f, FArgs::Stack(&[o.wrapping_add(0x90), v0]));
            let h2 = st(mul(sub(1.0, ip2), half));
            // fucompp with 0.0: equal only when ordered and equal
            let w9c = f32at(mem, o.wrapping_add(0x9c));
            let vv = if w9c == 0.0 { 0.0 } else { st(div(h2, w9c)) };
            let g2 = if 1.0 < vv { 0.0 } else { st(sub(1.0, vv)) };
            mem.write(v1, store(g2));
            let fv = store(mul(g2, f32at(mem, fade)));
            mem.write(fade, fv);
        }
        // the distance term
        let n = f64::from(mem.read::<u8>(o.wrapping_add(0x8e)));
        let r = st(sub(f32at(mem, o.wrapping_add(0xa8)), n));
        if !(0.0 < r) {
            mem.write(fade, 0u32);
            return;
        }
        let x = st(div(len, r));
        let k = if x < 1.0 || x.is_nan() {
            st(sub(1.0, x))
        } else {
            0.0
        };
        let fv = store(mul(k, f32at(mem, fade)));
        mem.write(fade, fv);
    }

    // ---- field round 31 ----
    /// `KeBornPtCmlp(pc, pd, prog, out, work)` (`0x00759400`, from
    /// `pppKeBornPtCmpl`): spawns the particle objects that fill the gap
    /// between this frame's point and the last one.
    /// - One object first: `pppCreatePObject(part, pd[8] * 16 + part
    ///   +0x3c)` (no object, 0, when that address is 0; a failed creation
    ///   returns at once), into `out`, its `+4` the owner
    ///   (`set_ptr32(pc)`); its position (`+0xa0 + pd[0xc]`, `dst`) is the
    ///   point at `pc + prog[0xc][1] + 0xa0` (`KeVu0CopyVecXYZ`).
    /// - Without a previous point (`pc + prog[0xc][0] + 0xa0` null), the
    ///   step `+0xa8` there becomes `1 / f32(pd[4] + 1)`.
    /// - Else, with `prev` that point (`get_ptr32_func`, copied into
    ///   `work`) and `d = dst - prev` (`work + 0x20`), for `f` from
    ///   `f32(pd[4])` down by 1.0 while above 0, another object goes to
    ///   `prev + d * f32(f * step)` (scaled into `work + 0x10`,
    ///   `KeVu0AddVecXYZ`).
    /// - Last, the previous point becomes `dst`.
    pub fn ke_born_pt_cmlp<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        args: [u32; 4],
        work: u32,
        call: &mut C,
    ) {
        use ffx_bridge::x87::{load, mul, round_trip, store, sub};
        let [pc, pd, prog, out] = args;
        // one particle object into `out`; false when its creation failed
        let spawn = |mem: &mut M, call: &mut C| -> bool {
            let part = mem.read::<u32>(mem.rebase(super::PPP_CURRENT_PART));
            let base = mem
                .read::<u32>(pd.wrapping_add(8))
                .wrapping_shl(4)
                .wrapping_add(mem.read::<u32>(part.wrapping_add(0x3c)));
            if base == 0 {
                mem.write(out, 0u32);
                return true;
            }
            let f = mem.rebase(super::PPP_CREATE_POBJECT);
            let o = call(mem, f, &[part, base]);
            mem.write(out, o);
            if o == 0 {
                return false;
            }
            let f = mem.rebase(super::SET_PTR32);
            let v = call(mem, f, &[pc]);
            mem.write(o.wrapping_add(4), v);
            true
        };
        if !spawn(mem, call) {
            return;
        }
        let dst = mem
            .read::<u32>(out)
            .wrapping_add(mem.read::<u32>(pd.wrapping_add(0xc)))
            .wrapping_add(0xa0);
        let tab = mem.read::<u32>(prog.wrapping_add(0xc));
        let src = pc
            .wrapping_add(mem.read::<u32>(tab.wrapping_add(4)))
            .wrapping_add(0xa0);
        let f = mem.rebase(super::KE_VU0_COPY_VEC_XYZ);
        call(mem, f, &[dst, src]);
        let prev_at = pc.wrapping_add(mem.read::<u32>(tab)).wrapping_add(0xa0);
        let f = mem.rebase(super::IS_PTR32_NULL);
        if call(mem, f, &[mem.read::<u32>(prev_at)]) != 0 {
            let n = u32::from(mem.read::<u8>(pd.wrapping_add(4))) + 1;
            #[allow(clippy::cast_precision_loss)] // a byte plus one
            let q = load((n as f32).to_bits());
            mem.write(prev_at.wrapping_add(8), store(1.0 / q));
        } else {
            let f = mem.rebase(super::GET_PTR32_FUNC);
            let prev = call(mem, f, &[mem.read::<u32>(prev_at)]);
            let f = mem.rebase(super::VU0_COPY_VECTOR);
            call(mem, f, &[work, prev]);
            let f = mem.rebase(super::VU0_SUB_VECTOR);
            call(mem, f, &[work.wrapping_add(0x20), dst, work]);
            let step = load(round_trip(mem.read::<f32>(prev_at.wrapping_add(8))).to_bits());
            let mut c = f64::from(f32::from(mem.read::<u8>(pd.wrapping_add(4))));
            while c > 0.0 {
                if !spawn(mem, call) {
                    return;
                }
                let o = mem.read::<u32>(out);
                let s = store(mul(c, step));
                let f = mem.rebase(super::VU0_SCALE_VECTOR);
                call(
                    mem,
                    f,
                    &[work.wrapping_add(0x10), work.wrapping_add(0x20), s],
                );
                let at = mem
                    .read::<u32>(pd.wrapping_add(0xc))
                    .wrapping_add(0xa0)
                    .wrapping_add(o);
                let f = mem.rebase(super::KE_VU0_ADD_VEC_XYZ);
                call(mem, f, &[at, work, work.wrapping_add(0x10)]);
                c = load(store(sub(c, 1.0)));
            }
        }
        let f = mem.rebase(super::SET_PTR32);
        let v = call(mem, f, &[dst]);
        mem.write(prev_at, v);
    }

    /// `KeVu0ItrsBallLine(q)` (`0x0075eed0`, from `pppKeHitChkPxB`): where
    /// the ray `p + t d` (`p` at `+0`, `d` at `+0x10`) enters the sphere of
    /// radius `+0x20` around the origin: with `a = d.d`, `b = 2 p.d`,
    /// `c = p.p - r²` (every partial sum stored as `f32`), no hit (0) when
    /// `b² - 4ac < 0`; else `t = (-b - sqrt(disc)) / f32(2a)` to `+0x24`
    /// and 1.
    pub fn ke_vu0_itrs_ball_line(mem: &mut impl Mem, q: u32) -> u32 {
        use ffx_bridge::x87::{add, div, load, mul, sqrt, store, sub};
        let st = |x: f64| load(store(x));
        let v = |mem: &dyn Fn(u32) -> u32, at: u32| load(mem(at));
        let rd = |at: u32| mem.read::<u32>(q.wrapping_add(at));
        let (p0, p1, p2) = (v(&rd, 0), v(&rd, 4), v(&rd, 8));
        let (d0, d1, d2) = (v(&rd, 0x10), v(&rd, 0x14), v(&rd, 0x18));
        let r = v(&rd, 0x20);
        let two: f64 = mem.read(mem.rebase(super::TWO));
        let four: f64 = mem.read(mem.rebase(super::FOUR));
        let a = st(add(st(add(st(mul(d0, d0)), mul(d1, d1))), mul(d2, d2)));
        let dot = st(add(st(add(st(mul(p0, d0)), mul(d1, p1))), mul(d2, p2)));
        let b = st(mul(dot, two));
        let c0 = st(add(st(add(st(mul(p0, p0)), mul(p1, p1))), mul(p2, p2)));
        let c = st(sub(c0, mul(r, r)));
        let disc = st(sub(mul(b, b), mul(c, mul(a, four))));
        if disc < 0.0 {
            return 0;
        }
        let s = st(sqrt(disc));
        let t = div(sub(-b, s), st(mul(a, two)));
        mem.write(q.wrapping_add(0x24), store(t));
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strides_grow_by_48() {
        assert_eq!(res_stride(1), 0x80);
        assert_eq!(res_stride(2), 0xb0);
        assert_eq!(res_stride(0), 0x80u16.wrapping_add(255 * 48));
    }
}
