//! M5 stage 1, list `render-pcui` (round 6), batch 2: specular
//! parameters, the float sort's partition, the shadow cascade's median
//! centre and the memory debug dumps.

use crate::m5r_c::quiet;

/// `FloatSort_Median9_006c7560(first, mid, last, pred)` (`cdecl`).
pub const FLOAT_MEDIAN9: u32 = 0x006c_7560;

pub const MATRIX_COPY: u32 = 0x004c_9430;
pub const MATRIX_FROM_3X4: u32 = 0x0045_1080;
pub const MATRIX_MUL_AFFINE: u32 = 0x004c_9da0;
/// `FloatSort_introsortLoop_006c77a0(first, last, depth, less)` and its
/// `FloatLess_006c94e0`.
pub const FLOAT_INTROSORT: u32 = 0x006c_77a0;
pub const FLOAT_LESS: u32 = 0x006c_94e0;
/// The cascade fit's 0.5 and its outlier distance 210.0 (`.rdata`
/// doubles).
pub const CASCADE_HALF: u32 = 0x00b9_2220;
pub const CASCADE_NEAR: u32 = 0x00b4_9970;

pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
pub const PHYRE_FREE: u32 = 0x0042_fb20;
pub const MEMSET: u32 = 0x0094_964c;
pub const MEMCPY: u32 = 0x0094_9256;
pub const FREELIST_ALLOCATE_BLOCK: u32 = 0x0049_c8d0;
pub const FREELIST_DEALLOCATE: u32 = 0x0049_c980;
pub const STRING_NODE_CTOR: u32 = 0x0064_cba0;
/// The memory debug map's node insert (`thiscall(map; node)`, `ret 4`,
/// nonzero when the key is there already) and the red-black finalize.
pub const MEMDEBUG_TREE_INSERT: u32 = 0x006d_c140;
pub const TREE_POST_INSERT: u32 = 0x0057_4980;
pub const MAP_FIND_CSTR: u32 = 0x006d_c010;
pub const MEMDEBUG_MAP_INSERT: u32 = 0x006d_dd60;
/// `sprintf_s`, `sprintf`, `fopen`, `fprintf`, `fclose`, `printf`
/// (import slots, `cdecl`).
pub const SPRINTF_S: u32 = 0x00b0_c394;
pub const SPRINTF: u32 = 0x00b0_c3dc;
pub const FOPEN: u32 = 0x00b0_c3f0;
pub const FPRINTF: u32 = 0x00b0_c38c;
pub const FCLOSE: u32 = 0x00b0_c3ec;
pub const PRINTF: u32 = 0x00b0_c380;
/// The memory debug strings: `""`, `"%s(%d)%s"`, `"%s"`,
/// `"WIN32_%s_%s_%lld.csv"`, `"w"`, the vram dump messages, `"%s_%s"`,
/// `"/"`, and the `"bin"` word.
pub const MD_EMPTY: u32 = 0x00b8_fbeb;
pub const MD_NAME_LINE: u32 = 0x00b4_a3d0;
pub const MD_FORMAT_S: u32 = 0x00b0_e160;
pub const MD_VRAM_FILE: u32 = 0x00b4_a3dc;
pub const MD_MODE_W: u32 = 0x00b0_db48;
pub const MD_VRAM_DUMPING: u32 = 0x00b4_a5a8;
pub const MD_VRAM_HEADER: u32 = 0x00b4_a590;
pub const MD_VRAM_UNSUPPORTED: u32 = 0x00b4_a550;
pub const MD_CANNOT_CREATE: u32 = 0x00b4_a454;
pub const MD_UNDERSCORE: u32 = 0x00b4_a5dc;
pub const MD_SLASH: u32 = 0x00b8_f88c;
pub const MD_BIN: u32 = 0x00b4_a5d8;
/// `"VramDetail"` (the pointer at `0x00c34aec`).
pub const MD_VRAM_KIND: u32 = 0x00c3_4aec;
pub const TOKENIZE: u32 = 0x00a2_9140;

/// The radial line's `.rdata` constants: 0.5f, -1.0f, and the doubles
/// 32767.0, 0.001 (as a float widened), 2.0, 0.002 (likewise), 0.5.
pub const RL_HALF: u32 = 0x00b9_2194;
pub const RL_MINUS_ONE: u32 = 0x00b9_2340;
pub const RL_RAND_MAX: u32 = 0x00b4_1d70;
pub const RL_JITTER: u32 = 0x00b4_7c68;
pub const RL_TWO: u32 = 0x00b9_2270;
pub const RL_BASE_WIDTH: u32 = 0x00b4_c168;
pub const RL_HALF_D: u32 = 0x00b9_2220;
/// `rand` (import slot).
pub const RAND: u32 = 0x00b0_c420;
pub const POST_MANAGER: u32 = 0x0069_2be0;
pub const POST_COPY_BUFFER: u32 = 0x0069_2c40;
pub const RL_SET_PARAM_6C: u32 = 0x006f_8b40;
pub const RL_SET_PARAM_70: u32 = 0x006f_8be0;
pub const RL_SET_COLOR_TARGET: u32 = 0x005b_b130;
pub const RL_SURFACE_CTOR: u32 = 0x005b_35c0;
pub const RL_SURFACE_DTOR: u32 = 0x005b_3640;
pub const RL_BEGIN_SCENE: u32 = 0x005b_5e30;
pub const RL_RENDER_MESH: u32 = 0x005b_9a90;
pub const RL_END_SCENE: u32 = 0x005b_6950;
/// The radial blur pass name (`0x00cdebd8`).
pub const RL_PASS: u32 = 0x00cd_ebd8;

pub const VFX_RENDER_DATA: u32 = 0x0064_53d0;
pub const DYN_GEO_BUFFER: u32 = 0x0063_fdb0;
pub const VFX_SET_VISIBLE: u32 = 0x0064_5710;
/// The bounds' `.rdata` -100000.0f and 100000.0f.
pub const VFX_BOUND_MIN: u32 = 0x00b3_68b8;
pub const VFX_BOUND_MAX: u32 = 0x00b1_0f50;

/// The PS2 movie decoder's state (`0x00c421b0`) and its IPU work block
/// (`0x0112a840`).
pub const IPU_STATE: u32 = 0x00c4_21b0;
pub const IPU_BLOCK: u32 = 0x0112_a840;
/// `g_GsRegisterSink`: bit 9 is the transfer-busy flag the decoder
/// spins on.
pub const GS_SINK: u32 = 0x0230_56e8;
pub const GS_SET_FRAME_REGS: u32 = 0x0077_e830;
pub const ASM_CLEAR_BLOCK: u32 = 0x0077_f1f0;

pub const FLUSH_CACHE: u32 = 0x0062_f4c0;
pub const RGB32_TO_24: u32 = 0x0077_a490;
pub const TEXANIM_DMA_TAG: u32 = 0x0077_eb70;
pub const TEXANIM_GIF_TAG: u32 = 0x0077_ebe0;
pub const TEXANIM_SET_TAG: u32 = 0x0077_eb00;
pub const TEXANIM_TRX_REG: u32 = 0x0077_f140;
pub const TEXANIM_TRX_POS: u32 = 0x0077_f100;
pub const TEXANIM_TRX_DIR: u32 = 0x0077_f0e0;

pub const SCISSOR_XY: u32 = 0x008e_58e0;
pub const SCISSOR_XYUV: u32 = 0x008e_5a20;
pub const DRAW_UI_ELEMENT: u32 = 0x0063_eee0;
pub const MAGIC_HANDLE: u32 = 0x009d_a390;
pub const MAGIC_ID: u32 = 0x009d_a3b0;
pub const FFX_LANG: u32 = 0x008a_c2f0;
pub const UNIFY_FILENAME: u32 = 0x0067_9820;
pub const INTERN_NAME: u32 = 0x008a_b440;
pub const MENU_ADD_NAME: u32 = 0x008e_5200;
pub const STRSTR: u32 = 0x00b0_c378;
/// The UI element's draw state (`0x025d1700`: rectangle, uv) and the two
/// words cleared before each draw (`0x025d1790`, `0x025d1794`).
pub const UI_ELEMENT: u32 = 0x025d_1700;
pub const UI_ELEMENT_CLEAR: u32 = 0x025d_1790;
/// `"/FFX_Data/GameData/PS3Data/magic"`, the texture path formats and the
/// two magic files with Chinese textures.
pub const MAGIC_DIR: u32 = 0x00b4_3b4c;
pub const MAGIC_TEX: u32 = 0x00b6_6ab4;
pub const MAGIC_0697: u32 = 0x00b6_6ae4;
pub const MAGIC_0698: u32 = 0x00b6_6af0;
pub const MAGIC_TEX_CN: u32 = 0x00b6_6afc;
/// The language with its own textures for magic 697 and 698.
pub const LANG_CHINESE: u32 = 0xb;

pub const AUDIO_SA_DECODE: u32 = 0x0077_b270;
pub const AUDIO_DELTA_DECODE: u32 = 0x0077_b1f0;
pub const AUDIO_DECODE_BLOCKS: u32 = 0x0077_afc0;
pub const MOVIE_VIEWER_AUDIO: u32 = 0x0077_c360;
pub const AUDIO_MIX_HALVES: u32 = 0x0077_b060;
pub const MOVIE_STEP: u32 = 0x0077_de40;
pub const SQS_RECORD: u32 = 0x0081_f760;
pub const DBG_PRINTF: u32 = 0x0062_f500;
pub const SQS_NEXT_HALF: u32 = 0x0081_f160;
pub const ALREADY_STARTED_INC: u32 = 0x0077_adb0;
pub const SND_SEND_STREAM: u32 = 0x0077_a8e0;
pub const SND_SEND_SYNC: u32 = 0x0088_5ee0;
/// `"sound buff err !!!!!!!!!!\n"`.
pub const SOUND_BUFF_ERR: u32 = 0x00b5_438c;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::Mem;
    use ffx_graphic::host::{Conv, Host};

    fn word(h: &impl Mem, at: u32) -> u32 {
        h.read(at)
    }

    fn method(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    /// `SpecularMapParamsManager_UpdateParams_006c6c30(inst, name, u0, u1,
    /// float1, mat1, v0, v1, v3, v2)` (stdcall, `ret 0x28`; the frame from
    /// the original's `ebp - 0x7c` at `frame`: the vector `{v0, v1, v2,
    /// v3}` `+0`, the texture iterator `+0x10`, the sampler value `+0x20`,
    /// the list copy `+0x58`). Without a name the samplers are reset as
    /// in `setSpecularMaterial`. With one and a texture in that cluster,
    /// on a copy of the mesh list each material's parameters are looked
    /// up (`SpecularMapSampler`, `SpecularUvOffset`,
    /// `Specular_SpecialFloat1`, `Specular_SpecialMat1`,
    /// `Specular_SpecialVector2`, `SpecularMapSamplerState`) and written:
    /// the state `{0x47, 0, linear sampler, 0}`, the sampler the texture
    /// with the linear sampler, the uv offset `(u0, u1)`, `float1`, the
    /// bytes at a non-null `mat1`, the vector; the copy freed unless
    /// owned elsewhere. Parameter sizes past a local copy the original's
    /// frame: 0 in the port (`write_param`). (The original keeps its
    /// loop state in its own argument slots `+0xc..+0x2c`; no caller
    /// reads them.)
    #[allow(clippy::too_many_arguments)] // the original's ten
    pub fn update_params(
        h: &mut impl Host,
        frame: u32,
        inst: u32,
        name: u32,
        uv: [u32; 2],
        float1: u32,
        mat1: u32,
        v: [u32; 4],
    ) {
        use crate::m5r_o::bridge::{
            cluster_texture, copy_param, each_mesh_material, reset_specular_samplers, write_param,
            write_sampler_state,
        };
        use crate::m5r_o::{
            SPEC_FLOAT1, SPEC_MAT1, SPEC_SAMPLER, SPEC_SAMPLER_STATE, SPEC_UV_OFFSET, SPEC_VECTOR2,
        };
        if inst == 0 {
            return;
        }
        if name == 0 {
            reset_specular_samplers(h, inst);
            return;
        }
        let uv = [quiet(uv[0]), quiet(uv[1])].map(u32::to_le_bytes).concat();
        let vector = v.map(quiet);
        h.write(frame, vector);
        let vector = vector.map(u32::to_le_bytes).concat();
        let float1 = quiet(float1).to_le_bytes();
        let Some(texture) = cluster_texture(h, frame + 0x10, name) else {
            return;
        };
        let copy = frame + 0x58;
        method(h, crate::m5r_o::ARRAY_COPY_FROM, copy, &[inst]);
        let count = word(h, copy) & 0x7fff_ffff;
        let data = word(h, copy + 4);
        let names = [
            SPEC_SAMPLER,
            SPEC_UV_OFFSET,
            SPEC_FLOAT1,
            SPEC_MAT1,
            SPEC_VECTOR2,
            SPEC_SAMPLER_STATE,
        ];
        each_mesh_material(h, count, data, |h, mat| {
            let params = word(h, mat);
            let mut p = [0_u32; 6];
            for (x, n) in p.iter_mut().zip(names) {
                let s = h.rebase(n);
                *x = method(h, crate::m5r_o::FIND_PARAM, params, &[s]);
            }
            let [sampler_p, uv_p, float_p, mat_p, vec_p, state_p] = p;
            if state_p != 0 {
                write_sampler_state(h, mat, state_p);
            }
            if sampler_p != 0 {
                let value = frame + 0x20;
                h.write(value, 0x42_u32);
                h.write(value + 0xc, texture);
                let scene = crate::m5r_o::SCENE.get(h);
                let s = method(h, crate::m5r_o::LINEAR_SAMPLER, scene, &[1]);
                let buffer = word(h, mat.wrapping_add(4));
                h.write(value + 8, s);
                h.write(value + 4, 0_u32);
                method(
                    h,
                    crate::m5r_o::SET_TEXTURE_CHECKED,
                    buffer,
                    &[sampler_p, value],
                );
            }
            let buffer = word(h, mat.wrapping_add(4));
            if uv_p != 0 {
                write_param(h, buffer, uv_p, &uv);
            }
            if float_p != 0 {
                write_param(h, buffer, float_p, &float1);
            }
            if mat_p != 0 && mat1 != 0 {
                copy_param(h, buffer, mat_p, mat1);
            }
            if vec_p != 0 {
                write_param(h, buffer, vec_p, &vector);
            }
        });
        if word(h, copy) >> 31 == 0 {
            let data = word(h, copy + 4);
            h.call_sym(crate::m5r_o::PHYRE_FREE, Conv::Cdecl, &[data]);
        }
    }

    /// The float predicate `pred(a, b)` (`cdecl`, two floats as
    /// `fld`/`fstp` pass them) -> its `al`.
    fn float_less(h: &mut impl Host, pred: u32, a: u32, b: u32) -> bool {
        h.call(pred, Conv::Cdecl, &[quiet(word(h, a)), quiet(word(h, b))]) as u8 != 0
    }

    /// Swaps the floats at `a` and `b` as the original's `fld`/`fstp`
    /// pairs do (`a` read first; both come out quiet).
    fn float_swap(h: &mut impl Mem, a: u32, b: u32) {
        let t = quiet(word(h, a));
        let v = quiet(word(h, b));
        h.write(a, v);
        h.write(b, t);
    }

    /// `FloatSort_partition_006c7900(out, first, last, pred)` (`cdecl`) ->
    /// `out`: `std::sort`'s partition by the median guess over floats
    /// (`_Partition_by_median_guess_unchecked`). The median of nine (or
    /// three) moves the pivot to the middle; the run of elements
    /// equivalent to it is grown to `[pfirst, plast)` and swept outwards,
    /// smaller ones to the left and greater to the right, and `out` gets
    /// the equal range `(pfirst, plast)`.
    pub fn float_partition(h: &mut impl Host, out: u32, first: u32, last: u32, pred: u32) -> u32 {
        let half = (last.wrapping_sub(first).cast_signed() >> 2) / 2;
        let mid = first.wrapping_add(half.wrapping_mul(4).cast_unsigned());
        h.call_sym(
            FLOAT_MEDIAN9,
            Conv::Cdecl,
            &[first, mid, last.wrapping_sub(4), pred],
        );
        let mut pfirst = mid;
        let mut plast = mid.wrapping_add(4);
        while first < pfirst {
            let prev = pfirst.wrapping_sub(4);
            if float_less(h, pred, prev, pfirst) || float_less(h, pred, pfirst, prev) {
                break;
            }
            pfirst = prev;
        }
        while plast < last {
            if float_less(h, pred, plast, pfirst) || float_less(h, pred, pfirst, plast) {
                break;
            }
            plast = plast.wrapping_add(4);
        }
        let mut gfirst = plast;
        let mut glast = pfirst;
        loop {
            while gfirst < last {
                if float_less(h, pred, pfirst, gfirst) {
                } else if float_less(h, pred, gfirst, pfirst) {
                    break;
                } else {
                    float_swap(h, plast, gfirst);
                    plast = plast.wrapping_add(4);
                }
                gfirst = gfirst.wrapping_add(4);
            }
            while first < glast {
                let g = glast.wrapping_sub(4);
                if float_less(h, pred, g, pfirst) {
                } else if float_less(h, pred, pfirst, g) {
                    break;
                } else {
                    pfirst = pfirst.wrapping_sub(4);
                    float_swap(h, pfirst, g);
                }
                glast = g;
            }
            if glast == first && gfirst == last {
                h.write(out, [pfirst, plast]);
                return out;
            }
            if glast == first {
                if plast != gfirst {
                    float_swap(h, pfirst, plast);
                }
                float_swap(h, pfirst, gfirst);
                plast = plast.wrapping_add(4);
                pfirst = pfirst.wrapping_add(4);
                gfirst = gfirst.wrapping_add(4);
            } else {
                glast = glast.wrapping_sub(4);
                if gfirst == last {
                    pfirst = pfirst.wrapping_sub(4);
                    if glast != pfirst {
                        float_swap(h, glast, pfirst);
                    }
                    plast = plast.wrapping_sub(4);
                    float_swap(h, pfirst, plast);
                } else {
                    float_swap(h, gfirst, glast);
                    gfirst = gfirst.wrapping_add(4);
                }
            }
        }
    }

    /// `ShadowManager_medianCharacterCenter_006c8480(this; out_x, out_z)`
    /// (`ret 8`; the frame from the original's `ebp - 0x328` at `frame`:
    /// the matrices `+0` (copy), `+0x40` (product), `+0x80` (world),
    /// `+0xd0` (translation) and the z and x arrays `+0x124`, `+0x224`,
    /// 64 floats each). For each of the 8x8 character slots (`+0xde8`)
    /// that is set and whose two flags (`+0x10e8 - 0x100`, `+0x10e8`, by
    /// slot) are 1, its bounds centre (`b[0] + b[4]/2`, `b[2] + b[6]/2`)
    /// is moved by its world matrix (3x4 at `[[ptr + 4]] + 4`, row 3's y
    /// 0 and w 1) and its x and z collected. Both lists are sorted; the
    /// medians, then the mean of the first and last values within 210 of
    /// them, go to `out_x` and `out_z` (0 and 0 without characters).
    pub fn median_center<H5: Host>(h: &mut H5, frame: u32, this: u32, out_x: u32, out_z: u32) {
        use ffx_bridge::x87;
        let half = f64::from_bits(h.read::<u64>(h.rebase(CASCADE_HALF)));
        let near = f64::from_bits(h.read::<u64>(h.rebase(CASCADE_NEAR)));
        let (copy, product, world, trans) = (frame, frame + 0x40, frame + 0x80, frame + 0xd0);
        let (zs, xs) = (frame + 0x124, frame + 0x224);
        let f = |h: &H5, at: u32| x87::load(word(h, at));
        let mut n = 0_u32;
        for slot in 0..64_u32 {
            let ptr = word(h, this + 0xde8 + 4 * slot);
            let flag = this + 4 * (0x43a + slot);
            if ptr == 0 || word(h, flag - 0x100) != 1 || word(h, flag) != 1 {
                continue;
            }
            let b = word(h, word(h, word(h, ptr + 4)) + 0x1c);
            let x = x87::store(x87::add(x87::mul(f(h, b + 0x10), half), f(h, b)));
            let z = x87::store(x87::add(x87::mul(f(h, b + 0x18), half), f(h, b + 8)));
            let (one, zero) = (1.0_f32.to_bits(), 0_u32);
            let m = [
                one, zero, zero, zero, zero, one, zero, zero, zero, zero, one, zero, x, zero, z,
                one,
            ];
            h.write(trans, m);
            method(h, MATRIX_COPY, copy, &[trans]);
            let src = word(h, word(h, word(h, ptr + 4)) + 4);
            method(h, MATRIX_FROM_3X4, world, &[src]);
            h.write(world + 0x34, 0_u32);
            h.write(world + 0x3c, one);
            method(h, MATRIX_MUL_AFFINE, world, &[product, copy]);
            let px = quiet(word(h, product + 0x30));
            h.write(xs + 4 * n, px);
            let pz = quiet(word(h, product + 0x38));
            h.write(zs + 4 * n, pz);
            n += 1;
        }
        if n == 0 {
            h.write(out_x, 0_u32);
            h.write(out_z, 0_u32);
            return;
        }
        let less = h.rebase(FLOAT_LESS);
        h.call_sym(FLOAT_INTROSORT, Conv::Cdecl, &[xs, xs + 4 * n, n, less]);
        h.call_sym(FLOAT_INTROSORT, Conv::Cdecl, &[zs, zs + 4 * n, n, less]);
        let mid = n / 2;
        let median = |h: &H5, a: u32| {
            if n & 1 == 1 {
                quiet(word(h, a + 4 * mid))
            } else {
                let s = x87::add(f(h, a + 4 * (mid - 1)), f(h, a + 4 * mid));
                x87::store(x87::mul(s, half))
            }
        };
        let mx = median(h, xs);
        h.write(out_x, mx);
        let mz = median(h, zs);
        h.write(out_z, mz);
        let mz = x87::load(mz);
        let close = |d: f64| {
            let d = x87::store(d) & 0x7fff_ffff;
            x87::load(d) < near
        };
        let mut lo_x = 0;
        for i in 0..n {
            if close(x87::sub(f(h, out_x), f(h, xs + 4 * i))) {
                lo_x = i;
                break;
            }
        }
        let mut lo_z = 0;
        for i in 0..n {
            if close(x87::sub(mz, f(h, zs + 4 * i))) {
                lo_z = i;
                break;
            }
        }
        let mut hi_x = n - 1;
        for i in (0..n).rev() {
            if close(x87::sub(f(h, out_x), f(h, xs + 4 * i))) {
                hi_x = i;
                break;
            }
        }
        let mut hi_z = n - 1;
        for i in (0..n).rev() {
            if close(x87::sub(mz, f(h, zs + 4 * i))) {
                hi_z = i;
                break;
            }
        }
        let x = x87::add(f(h, xs + 4 * hi_x), f(h, xs + 4 * lo_x));
        let x = x87::store(x87::mul(x, half));
        h.write(out_x, x);
        let z = x87::add(f(h, zs + 4 * hi_z), f(h, zs + 4 * lo_z));
        let z = x87::store(x87::mul(z, half));
        h.write(out_z, z);
    }

    fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f = word(h, h.rebase(slot));
        h.call(f, Conv::Cdecl, args)
    }

    /// A tagged pointer with bit 0 cleared.
    fn untag(p: u32) -> u32 {
        p.wrapping_sub(p & 1)
    }

    /// `MemoryDebug_mapInsert_006ddd60(map; key, value)` (`ret 8`) -> 0,
    /// 0xe without a node, 0x12 when the key is there: a node from the
    /// map's free list (`+0x18`, a new block when empty), its key a copy
    /// of `key` (`StringTreeNode_ctor`), its 0x52-word value copied from
    /// `value`, inserted (`0x006dc140`) and balanced; a duplicate's key is
    /// freed (untagged) and the node returned to the list.
    pub fn map_insert(h: &mut impl Host, map: u32, key: u32, value: u32) -> u32 {
        let list = map + 0x18;
        let mut node = word(h, list);
        if node == 0 {
            let block = word(h, list + 8);
            method(h, FREELIST_ALLOCATE_BLOCK, list, &[block]);
            node = word(h, list);
        }
        if node == 0 {
            return 0xe;
        }
        let next = word(h, node);
        h.write(list, next);
        method(h, STRING_NODE_CTOR, node, &[key]);
        for k in 0..0x52 {
            let v = word(h, value + 4 * k);
            h.write(node + 0x10 + 4 * k, v);
        }
        if method(h, MEMDEBUG_TREE_INSERT, map, &[node]) != 0 {
            let k = word(h, node + 0xc);
            if k & 1 == 0 {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[k]);
            }
            method(h, FREELIST_DEALLOCATE, map + 0x18, &[node]);
            return 0x12;
        }
        method(h, TREE_POST_INSERT, map, &[node]);
        0
    }

    /// `MemoryDebug_addRecord_006dd230(rec, map)` (`cdecl`; the frame from
    /// the original's `ebp - 0x3a0` at `frame`: the key cell `+4`, the
    /// value `+8` (size, count 1, a 0x140-byte note), the note buffer
    /// `+0x150`, the line `+0x290`): the key `"<name +8>(<+0x108>)"` (an
    /// empty note appended) is copied to the heap and looked up in `map`;
    /// a known key adds the record's size (`+4`) and counts it, a new one
    /// is inserted with count 1. The heap copy is freed (untagged).
    pub fn add_record(h: &mut impl Host, frame: u32, rec: u32, map: u32) {
        let (cell, value, note, line) = (frame + 4, frame + 8, frame + 0x150, frame + 0x290);
        let empty = h.rebase(MD_EMPTY);
        import(h, SPRINTF_S, &[note, 0x140, empty]);
        h.call_sym(MEMSET, Conv::Cdecl, &[line, 0, 0x100]);
        let f = h.rebase(MD_NAME_LINE);
        let id = word(h, rec + 0x108);
        import(h, SPRINTF_S, &[line, 0x100, f, rec + 8, id, note]);
        let mut n = 0_u32;
        while h.read::<u8>(line + n) != 0 {
            n += 1;
        }
        let key = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[n + 1, 2]);
        if key != 0 {
            h.call_sym(MEMCPY, Conv::Cdecl, &[key, line, n + 1]);
        }
        h.write(cell, key);
        let r = method(h, MAP_FIND_CSTR, map, &[cell]);
        if r != 0 && r.wrapping_add(0x10) != 0 {
            let size = word(h, rec + 4);
            let total = word(h, r + 0x10).wrapping_add(size);
            h.write(r + 0x10, total);
            let count = word(h, r + 0x14).wrapping_add(1);
            h.write(r + 0x14, count);
        } else {
            let size = word(h, rec + 4);
            h.write(value, size);
            h.write(value + 4, 1_u32);
            let f = h.rebase(MD_FORMAT_S);
            import(h, SPRINTF_S, &[value + 8, 0x140, f, note]);
            method(h, MEMDEBUG_MAP_INSERT, map, &[cell, value]);
        }
        if key & 1 == 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[key]);
        }
    }

    /// `MemoryDebug_dumpVram_006dd3b0(this; time_lo, time_hi, name)` (`ret
    /// 0xc`; the file name at `frame`, the original's `ebp - 0x104`, 256
    /// bytes) -> the last call's `eax` (`name` when not dumping): when
    /// the debug flags `+0` and `+1` are set, `"WIN32_VramDetail_<name>_
    /// <time>.csv"` gets a header and a line saying the dump is not
    /// supported on Windows (or a message that it can't be created).
    pub fn dump_vram(h: &mut impl Host, frame: u32, this: u32, lo: u32, hi: u32, name: u32) -> u32 {
        if h.read::<u8>(this) == 0 || h.read::<u8>(this + 1) == 0 {
            return name;
        }
        let (f, kind) = (h.rebase(MD_VRAM_FILE), word(h, h.rebase(MD_VRAM_KIND)));
        import(h, SPRINTF, &[frame, f, kind, name, lo, hi]);
        let w = h.rebase(MD_MODE_W);
        let file = import(h, FOPEN, &[frame, w]);
        if file == 0 {
            let s = h.rebase(MD_CANNOT_CREATE);
            return import(h, PRINTF, &[s, frame]);
        }
        let s = h.rebase(MD_VRAM_DUMPING);
        import(h, PRINTF, &[s, frame]);
        let s = h.rebase(MD_VRAM_HEADER);
        import(h, FPRINTF, &[file, s]);
        let s = h.rebase(MD_VRAM_UNSUPPORTED);
        import(h, FPRINTF, &[file, s]);
        import(h, FCLOSE, &[file])
    }

    /// `MemoryDebug_dumpName_006dd5a0(this; path, suffix, out, size)`
    /// (`ret 0x10`; the token array at `frame`, the original's `ebp -
    /// 0x20`): `this + 0xc` (256 bytes) is cleared, then `path` is split
    /// at `"/"` (`IggyString_tokenize`); the token before `"bin"` (or the
    /// last token when `"bin"` is first or missing) goes to `this + 0xc`
    /// and, followed by `"_<suffix>"`, to `out`. The tokens and the
    /// `"bin"` copy are freed (untagged, the array only when owned).
    pub fn dump_name<H6: Host>(
        h: &mut H6,
        frame: u32,
        this: u32,
        path: u32,
        suffix: u32,
        out: u32,
        size: u32,
    ) {
        let dir = this + 0xc;
        h.call_sym(MEMSET, Conv::Cdecl, &[dir, 0, 0x100]);
        h.write(frame, [0_u32; 2]);
        let bin = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[4, 2]);
        if bin != 0 {
            let w = word(h, h.rebase(MD_BIN));
            h.write(bin, w);
        }
        let sep = h.rebase(MD_SLASH);
        if h.call_sym(TOKENIZE, Conv::Cdecl, &[path, sep, frame]) == 0 {
            let n = word(h, frame) & 0x7fff_ffff;
            let data = word(h, frame + 4);
            let s = untag(bin);
            let token = |h: &H6, i: u32| untag(word(h, data.wrapping_add(4 * i)));
            let mut pick = None;
            for i in 0..n {
                let t = token(h, i);
                let same = t == s || (t != 0 && s != 0 && same_text(h, t, s));
                if same && i != 0 {
                    pick = Some(i - 1);
                    break;
                }
                if i == n - 1 {
                    pick = Some(i);
                    break;
                }
            }
            if let Some(i) = pick {
                let f = h.rebase(MD_FORMAT_S);
                let t = token(h, i);
                import(h, SPRINTF_S, &[dir, 0x100, f, t]);
                let t = token(h, i);
                let f = h.rebase(MD_UNDERSCORE);
                import(h, SPRINTF_S, &[out, size, f, t, suffix]);
            }
        }
        if bin & 1 == 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[bin]);
        }
        let count = word(h, frame);
        let data = word(h, frame + 4);
        if count >> 31 == 0 && data != 0 {
            for i in 0..count & 0x7fff_ffff {
                let t = word(h, data + 4 * i);
                if t & 1 == 0 {
                    h.call_sym(PHYRE_FREE, Conv::Cdecl, &[t]);
                }
            }
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
        }
    }

    /// `strcmp(a, b) == 0` as the original inlines it.
    fn same_text(h: &impl Mem, a: u32, b: u32) -> bool {
        let mut k = 0_u32;
        loop {
            let (x, y) = (
                h.read::<u8>(a.wrapping_add(k)),
                h.read::<u8>(b.wrapping_add(k)),
            );
            if x != y {
                return false;
            }
            if x == 0 {
                return true;
            }
            k += 1;
        }
    }

    /// `FrameTime_average_006e53c0()`: a leftover of a frame time
    /// average. It sums `count - 1` zeros (`0x00ce59a0`) into a float and
    /// only stores when the sum is positive, which it never is: nothing
    /// to do.
    pub fn frame_time_average(_h: &mut impl Host) {}

    /// The radial line's two counts (`+0x30`, `+0x34`) as the original
    /// reads them (signed).
    fn counts(h: &impl Mem, this: u32) -> (i32, i32) {
        (
            word(h, this + 0x30).cast_signed(),
            word(h, this + 0x34).cast_signed(),
        )
    }

    /// `f32(i) / f32(n)` as `fild; fstp dword` twice and an x87 division.
    fn ratio(i: i32, n: i32) -> f64 {
        use ffx_bridge::x87;
        let f = |v: i32| x87::load(x87::store(f64::from(v)));
        x87::div(f(i), f(n))
    }

    /// The four runs of a radial line's vertex arrays: the first `a`
    /// entries, then `b`, `a`, `b` more; `f(h, run, k, index)` for each
    /// (the counts read again for every entry, as the original does).
    fn each_run<H: Host>(h: &mut H, this: u32, mut f: impl FnMut(&mut H, usize, i32, u32)) {
        for run in 0..4 {
            let mut k = 0_i32;
            loop {
                let (a, b) = counts(h, this);
                let len = if run % 2 == 0 { a } else { b };
                if k >= len {
                    break;
                }
                let base = match run {
                    0 => 0,
                    1 => a,
                    2 => a.wrapping_add(b),
                    _ => a.wrapping_mul(2).wrapping_add(b),
                };
                f(h, run, k, base.wrapping_add(k).cast_unsigned());
                k += 1;
            }
        }
    }

    /// `RadialLine_initColors_006f7e60(this)`: every vertex pair of the
    /// line (`+0x54`, 0x20 bytes) gets the grey (0.5, 0.5, 0.5, 1) and a
    /// transparent black (0, 0, 0, 1).
    pub fn radial_init_colors(h: &mut impl Host, this: u32) {
        let half = quiet(h.read(h.rebase(RL_HALF)));
        let one = 1.0_f32.to_bits();
        each_run(h, this, |h, _, _, i| {
            let e = word(h, this + 0x54).wrapping_add(0x20 * i);
            h.write(e, [half, half, half, one, 0, 0, 0, one]);
        });
    }

    /// `RadialLine_initWidths_006f8110(this)`: for `2 (+0x30 + +0x34)`
    /// lines (the count wrapped as the original's `<< 2 >> 1`) the state
    /// (`+0x64`, 0x20 bytes) is reset: time 0, a random width
    /// `rand / 32767 * 0.001 * 2 - 0.001 + 0.002`, and six 1.0.
    pub fn radial_init_widths<H7: Host>(h: &mut H7, this: u32) {
        use ffx_bridge::x87;
        let n = |h: &H7| (word(h, this + 0x30).wrapping_add(word(h, this + 0x34))).wrapping_shl(2);
        if n(h) & !1 == 0 {
            return;
        }
        let d = |h: &H7, sym: u32| f64::from_bits(h.read::<u64>(h.rebase(sym)));
        let (max, jitter, two, base) = (
            d(h, RL_RAND_MAX),
            d(h, RL_JITTER),
            d(h, RL_TWO),
            d(h, RL_BASE_WIDTH),
        );
        let mut i = 0_u32;
        loop {
            let e = word(h, this + 0x64).wrapping_add(0x20 * i);
            h.write(e, 0_u32);
            let r = word(h, h.rebase(RAND));
            let r = h.call(r, Conv::Cdecl, &[]).cast_signed();
            let v = x87::load(x87::store(f64::from(r)));
            let x = x87::div(v, max);
            let w = x87::sub(x87::mul(x87::mul(x, jitter), two), jitter);
            let w = x87::store(x87::add(w, base));
            let e = word(h, this + 0x64).wrapping_add(0x20 * i);
            h.write(e + 4, w);
            let one = 1.0_f32.to_bits();
            h.write(e + 8, [one; 6]);
            i += 1;
            if i >= n(h) >> 1 {
                break;
            }
        }
    }

    /// `RadialLine_initUvs_006f81c0` / `_006f83b0(this)`: the uv rows of
    /// the line's array at `this + array` (`+0x5c`, `+0x58`; 0x10 bytes
    /// each), around the four edges: (0.5, 0.5, `k/a`, 0), (0.5, 0.5, 1,
    /// `k/b`), (0.5, 0.5, `1 - k/a`, 1), (0.5, 0.5, 0, `1 - k/b`).
    pub fn radial_init_uvs(h: &mut impl Host, this: u32, array: u32) {
        use ffx_bridge::x87;
        let half = quiet(h.read(h.rebase(RL_HALF)));
        let one = 1.0_f32.to_bits();
        each_run(h, this, |h, run, k, i| {
            let (a, b) = counts(h, this);
            let e = word(h, this + array).wrapping_add(0x10 * i);
            let row = match run {
                0 => [half, half, x87::store(ratio(k, a)), 0],
                1 => [half, half, one, x87::store(ratio(k, b))],
                2 => [half, half, x87::store(x87::sub(1.0, ratio(k, a))), one],
                _ => [half, half, 0, x87::store(x87::sub(1.0, ratio(k, b)))],
            };
            h.write(e, row);
        });
    }

    /// `RadialLine_initPositions_006f85a0(this)`: the vertex positions
    /// (`+0x50`, 0x18 bytes each: two points (x, y, w)) around the
    /// screen's edges, from -1 to 1: top `(0, 0, 1)`-`(p, -1, 1)`, right
    /// `(1, p, 1)`, bottom `(p', 1, 1)`, left `(-1, p', 1)` with `p =
    /// 2 k / n - 1` and `p' = 2 (n - k - 1) / n - 1`.
    pub fn radial_init_positions(h: &mut impl Host, this: u32) {
        use ffx_bridge::x87;
        let m1 = quiet(h.read(h.rebase(RL_MINUS_ONE)));
        let two = f64::from_bits(h.read::<u64>(h.rebase(RL_TWO)));
        let one = 1.0_f32.to_bits();
        let p = |q: f64| x87::store(x87::sub(x87::mul(q, two), 1.0));
        each_run(h, this, |h, run, k, i| {
            let (a, b) = counts(h, this);
            let e = word(h, this + 0x50).wrapping_add(0x18 * i);
            let row = match run {
                0 => [0, 0, one, p(ratio(k, a)), m1, one],
                1 => [0, 0, one, one, p(ratio(k, b)), one],
                2 => [
                    0,
                    0,
                    one,
                    p(ratio(a.wrapping_sub(k).wrapping_sub(1), a)),
                    one,
                    one,
                ],
                _ => [
                    0,
                    0,
                    one,
                    m1,
                    p(ratio(b.wrapping_sub(k).wrapping_sub(1), b)),
                    one,
                ],
            };
            h.write(e, row);
        });
    }

    /// `RadialLine_render_006f8850(this; target)` (`ret 4`; the frame from
    /// the original's `ebp - 0x2c` at `frame`: the render surface) -> the
    /// last renderer call's `eax`: the colour target (`+0`, `+4`) is
    /// copied to the blur buffer (`+0x14`) by the post-process manager,
    /// the line's colour (`+0x38..+0x44`) and the uv scale and bias
    /// `(1/w, 1/h, (1 - 1/w) / 2, (1 - 1/h) / 2)` of its size (`+0x48`,
    /// `+0x4c`) are set, and its mesh (`+0x24`) is drawn into `target` (or
    /// the back buffer) in the radial pass.
    pub fn radial_render(h: &mut impl Host, frame: u32, this: u32, target: u32) -> u32 {
        use ffx_bridge::x87;
        let pm = h.call_sym(POST_MANAGER, Conv::Cdecl, &[]);
        let (r, src, dst) = (word(h, this), word(h, this + 4), word(h, this + 0x14));
        method(h, POST_COPY_BUFFER, pm, &[r, src, dst, 0, 0, 0]);
        let c = [
            word(h, this + 0x38),
            word(h, this + 0x3c),
            word(h, this + 0x40),
            word(h, this + 0x44),
        ];
        method(h, RL_SET_PARAM_6C, this, &c);
        let w = x87::load(word(h, this + 0x48));
        let hh = x87::load(word(h, this + 0x4c));
        let half = f64::from_bits(h.read::<u64>(h.rebase(RL_HALF_D)));
        let p0 = x87::store(x87::div(1.0, w));
        let p1 = x87::store(x87::div(1.0, hh));
        let p2 = x87::store(x87::mul(x87::sub(1.0, x87::div(1.0, w)), half));
        let p3 = x87::store(x87::mul(x87::sub(1.0, x87::div(1.0, hh)), half));
        method(h, RL_SET_PARAM_70, this, &[p0, p1, p2, p3]);
        let r = word(h, this);
        if target == 0 {
            method(h, RL_SET_COLOR_TARGET, r, &[0, 0]);
        } else {
            method(h, RL_SURFACE_CTOR, frame, &[target, 0]);
            let r = word(h, this);
            method(h, RL_SET_COLOR_TARGET, r, &[frame, 0]);
            method(h, RL_SURFACE_DTOR, frame, &[]);
        }
        let r = word(h, this);
        let e = method(h, RL_BEGIN_SCENE, r, &[0]);
        if e != 0 {
            return e;
        }
        let pass = h.rebase(RL_PASS);
        let r = word(h, this);
        h.write(r + 0x48, pass);
        let r = word(h, this);
        let inst = word(h, this + 0x24);
        let (mesh, list) = (word(h, inst + 0x10), word(h, r + 0x13c));
        method(h, RL_RENDER_MESH, r, &[inst, mesh, 0, list]);
        let r = word(h, this);
        method(h, RL_END_SCENE, r, &[1])
    }

    /// `updateVFXRenderDataRects(a, b, rects)` (`cdecl`) -> 1, or 0
    /// without the effect's render data or a geometry buffer. `rects` is
    /// `{data, count, _, matrix[16]}`: each rectangle (0x70 bytes: four
    /// corners at words 0, 7, 14, 21) becomes four vertices (0x30 bytes
    /// in the dynamic geometry's vertex block `[+0x94] + 0xc`) and six
    /// indices (`4i, 4i+1, 4i+2, 4i+2, 4i+1, 4i+3` as u16 at
    /// `[+0x94] + 0x1c`); the colour block (`+0x14`, 0x40 bytes a
    /// rectangle) is cleared. The render data takes the matrix with its
    /// column w set to (0, 0, 0, 1), bounds of ±100000, and is made
    /// visible.
    pub fn update_vfx_rects(h: &mut impl Host, a: u32, b: u32, rects: u32) -> u32 {
        let src = word(h, rects);
        let rd = h.call_sym(VFX_RENDER_DATA, Conv::Cdecl, &[a, b]);
        if rd == 0 {
            return 0;
        }
        let geo = word(h, rd + 0x94);
        let n = word(h, rects + 4);
        h.write(geo + 8, n.wrapping_mul(6));
        h.write(geo + 4, n.wrapping_shl(2));
        let geo = word(h, rd + 0x94);
        let (verts, inds) = (word(h, geo + 4), word(h, geo + 8));
        if h.call_sym(DYN_GEO_BUFFER, Conv::Cdecl, &[rd, 0, verts, inds]) == 0 {
            return 0;
        }
        let geo = word(h, rd + 0x94);
        let (vb, ib) = (word(h, geo + 0xc), word(h, geo + 0x1c));
        let colors = word(h, geo + 0x14);
        let n = word(h, rects + 4);
        h.call_sym(MEMSET, Conv::Cdecl, &[colors, 0, n.wrapping_shl(6)]);
        let mut i = 0_u32;
        while i.cast_signed() < word(h, rects + 4).cast_signed() {
            let s = src.wrapping_add(0x70 * i);
            let d = vb.wrapping_add(0x30 * i);
            for (k, w) in (0u32..).zip([0_u32, 1, 2, 7, 8, 9, 0xe, 0xf, 0x10, 0x15, 0x16, 0x17]) {
                let v = quiet(word(h, s.wrapping_add(4 * w)));
                h.write(d.wrapping_add(4 * k), v);
            }
            let q = i.wrapping_mul(4) as u16;
            let e = ib.wrapping_add(0xc * i);
            for (k, v) in (0u32..).zip([
                q,
                q.wrapping_add(1),
                q.wrapping_add(2),
                q.wrapping_add(2),
                q.wrapping_add(1),
                q.wrapping_add(3),
            ]) {
                h.write(e.wrapping_add(2 * k), v);
            }
            i += 1;
        }
        for k in 0..16 {
            let v = word(h, rects + 0xc + 4 * k);
            h.write(rd + 4 * k, v);
        }
        for off in [0xc, 0x1c, 0x2c] {
            h.write(rd + off, 0_u32);
        }
        h.write(rd + 0x3c, 1.0_f32.to_bits());
        let lo = quiet(h.read(h.rebase(VFX_BOUND_MIN)));
        let hi = quiet(h.read(h.rebase(VFX_BOUND_MAX)));
        h.write(rd + 0x98, [lo, lo, lo, hi, hi, hi]);
        h.call_sym(VFX_SET_VISIBLE, Conv::Cdecl, &[a, b, 1]);
        1
    }

    /// Spins while the GS transfer flag (bit 9 of `g_GsRegisterSink`) is
    /// set, as the original does (another thread clears it).
    fn wait_gs(h: &impl Mem) {
        while word(h, h.rebase(GS_SINK)) & 0x200 != 0 {
            core::hint::spin_loop();
        }
    }

    /// `IpuLoadMacroblock_0077e210(mb, x, y)` (`cdecl`): the macroblock
    /// `mb` of the frame (row width `[state + 0x50514]`) offset by
    /// `(x, y)` pixels: the two rows of 16x16 blocks under it are loaded
    /// into the work block (`+0x4d0`, `+0xad0`; `0x600` bytes each from
    /// `[state + 0x503c8]`, 0x300 a block). When the 16x16 square leaves
    /// the frame (`[state + 0x5051c]`, `[state + 0x50520]`), the quarter
    /// blocks it misses are cleared (`asmClearBlock`). A zero row width
    /// divides by zero in the original; the port does nothing then.
    pub fn ipu_load_macroblock(h: &mut impl Host, mb: u32, x: u32, y: u32) {
        let state = word(h, h.rebase(IPU_STATE));
        let w = word(h, state + 0x5_0514).cast_signed();
        if w == 0 {
            return;
        }
        let (mb, x, y) = (mb.cast_signed(), x.cast_signed(), y.cast_signed());
        let (q, r) = (mb.wrapping_div(w), mb.wrapping_rem(w));
        let mut col = r.wrapping_add(x / 16);
        let mut row = q.wrapping_add(y / 16);
        let px = r.wrapping_shl(4).wrapping_add(x);
        let py = q.wrapping_shl(4).wrapping_add(y);
        if x % 16 < 0 {
            col = col.wrapping_sub(1);
        }
        if y % 16 < 0 {
            row = row.wrapping_sub(1);
        }
        let idx = w.wrapping_mul(row).wrapping_add(col);
        let block = word(h, h.rebase(IPU_BLOCK));
        let src = word(h, state + 0x5_03c8);
        let at = |i: i32| i.wrapping_mul(0x300).cast_unsigned().wrapping_add(src);
        h.call_sym(
            GS_SET_FRAME_REGS,
            Conv::Cdecl,
            &[block + 0x4d0, at(idx), 0x600],
        );
        wait_gs(h);
        let state = word(h, h.rebase(IPU_STATE));
        let w2 = word(h, state + 0x5_0514).cast_signed();
        let src = word(h, state + 0x5_03c8);
        let block = word(h, h.rebase(IPU_BLOCK));
        let at2 = w2
            .wrapping_add(idx)
            .wrapping_mul(0x300)
            .cast_unsigned()
            .wrapping_add(src);
        h.call_sym(GS_SET_FRAME_REGS, Conv::Cdecl, &[block + 0xad0, at2, 0x600]);
        let state = word(h, h.rebase(IPU_STATE));
        let fw = word(h, state + 0x5_051c).cast_signed();
        let fh = word(h, state + 0x5_0520).cast_signed();
        if px >= 0 && px <= fw.wrapping_sub(16) && py >= 0 && py <= fh.wrapping_sub(16) {
            return;
        }
        wait_gs(h);
        let mut f = [false; 4];
        let mut set = |a: usize, b: usize| {
            f[a] = true;
            f[b] = true;
        };
        if px < 0 {
            set(0, 2);
        }
        if py < 0 {
            set(0, 1);
        }
        if px <= -16 {
            set(1, 3);
        }
        if py <= -16 {
            set(2, 3);
        }
        let state = word(h, h.rebase(IPU_STATE));
        let fw = word(h, state + 0x5_051c).cast_signed();
        let fh = word(h, state + 0x5_0520).cast_signed();
        if px > fw.wrapping_sub(16) {
            set(1, 3);
        }
        if py > fh.wrapping_sub(16) {
            set(2, 3);
        }
        if px >= fw {
            set(0, 2);
        }
        if py >= fh {
            set(0, 1);
        }
        for (k, &on) in (0u32..).zip(&f) {
            if on {
                let block = word(h, h.rebase(IPU_BLOCK));
                h.call_sym(ASM_CLEAR_BLOCK, Conv::Cdecl, &[block + 0x4d0 + 0x300 * k]);
            }
        }
    }

    /// `IpuInitBlockPointers_0077ea40()`: the IPU work block's pointers to
    /// its own buffers (`+0x40..+0x48`: `+0x50`, `+0x350`, `+0x4d0`;
    /// `+0..+0xc`: the four quarter blocks from `+0x4d0`, 0x300 apart) and
    /// its clamp words (`0x80008000` twice, `0x7f007f00` twice) -> the
    /// last pointer (`eax`).
    pub fn ipu_init_block_pointers(h: &mut impl Host) -> u32 {
        let b = word(h, h.rebase(IPU_BLOCK));
        h.write(b + 0x40, [b + 0x50, b + 0x350, b + 0x4d0]);
        h.write(b, [b + 0x4d0, b + 0x7d0, b + 0xad0, b + 0xdd0]);
        h.write(
            b + 0x30,
            [0x8000_8000_u32, 0x8000_8000, 0x7f00_7f00, 0x7f00_7f00],
        );
        b + 0xdd0
    }

    /// `IpuBuildUploadChain_0077ec20(q, src, x, y, w, h, bw, base, step,
    /// half, rgb)` (`cdecl`) -> the last DMA tag's `eax`: a PS2 DMA chain
    /// at `q` (uncached, `| 0x20000000`) uploading a `w`x`h` picture as
    /// 16x16 blocks: the BITBLT and TRXREG registers first, then per
    /// block a TRXPOS at `(x + 16 c, y + 16 r)` (`y` steps by 16 a row
    /// when `step`), TRXDIR, an IMAGE GIF tag of `bpp / 16` quadwords (the
    /// last block's EOP set) and a REF tag to the next `bpp` source
    /// bytes; an END tag closes it, between two `FlushCache(0)`. `bpp` is
    /// 0x400, 0x200 with `half`, or 0x300 when the picture is first
    /// packed to 24 bits (`rgb` 0). (The original keeps its loop state in
    /// its argument slots; the shim's slots are not written.)
    #[allow(clippy::too_many_arguments)] // the original's eleven
    pub fn ipu_build_upload_chain<H8: Host>(
        h: &mut H8,
        q: u32,
        mut src: u32,
        x: u32,
        mut y: u32,
        w: u32,
        hgt: u32,
        bw: u32,
        base: u32,
        step: u32,
        half: u32,
        rgb: u32,
    ) -> u32 {
        let c = |h: &mut H8, f: u32, a: &[u32]| h.call_sym(f, Conv::Cdecl, a);
        let rows = hgt.cast_signed() >> 4;
        let cols = w.cast_signed() >> 4;
        c(h, FLUSH_CACHE, &[0]);
        let mut bpp: u32 = if half != 0 { 0x200 } else { 0x400 };
        if rgb == 0 {
            c(h, RGB32_TO_24, &[src, w.wrapping_mul(hgt)]);
            bpp = 0x300;
        }
        let qwc = (bpp.cast_signed() / 16).cast_unsigned();
        let tbw = (bw.cast_signed() / 64).cast_unsigned();
        let mode = if half != 0 { 2 } else { u32::from(rgb == 0) };
        let q = q | 0x2000_0000;
        c(h, TEXANIM_DMA_TAG, &[q, 0, 0, 0, 1, 0, 3]);
        c(h, TEXANIM_GIF_TAG, &[q + 0x10, 0xe, 0, 1, 0, 0, 0, 0, 2]);
        c(h, TEXANIM_SET_TAG, &[q + 0x20, base, tbw, mode, rgb]);
        c(h, TEXANIM_TRX_REG, &[q + 0x30, 0x10, 0x10]);
        let mut cur = q + 0x40;
        let mut r = 0_i32;
        while r < rows {
            let yoff = r.cast_unsigned().wrapping_mul(16);
            let mut bx = x;
            let mut col = 0_i32;
            while col < cols {
                let last = u32::from(r == rows - 1 && col == cols - 1);
                c(h, TEXANIM_DMA_TAG, &[cur, 0, 0, 0, 1, 0, 4]);
                c(h, TEXANIM_GIF_TAG, &[cur + 0x10, 0xe, 0, 1, 0, 0, 0, 0, 2]);
                c(
                    h,
                    TEXANIM_TRX_POS,
                    &[cur + 0x20, 0, bx, yoff.wrapping_add(y)],
                );
                c(h, TEXANIM_TRX_DIR, &[cur + 0x30, 0]);
                c(
                    h,
                    TEXANIM_GIF_TAG,
                    &[cur + 0x40, 0, 0, 0, 2, 0, 0, last, qwc],
                );
                c(h, TEXANIM_DMA_TAG, &[cur + 0x50, 0, src, 0, 3, 0, qwc]);
                src = src.wrapping_add(bpp);
                bx = bx.wrapping_add(16);
                cur = cur.wrapping_add(0x60);
                col += 1;
            }
            if step != 0 {
                y = y.wrapping_add(16);
            }
            r += 1;
        }
        let e = c(h, TEXANIM_DMA_TAG, &[cur, 0, 0, 0, 7, 0, 0]);
        c(h, FLUSH_CACHE, &[0]);
        e
    }

    /// `graphicDrawUIElementAfterSet(tex0_lo, tex0_hi, a, b)` (`cdecl`;
    /// the file name buffer at `frame`, the original's `ebp - 0x204`, and
    /// the path at `+0x100`). Without a texture (`tex0` 0) the element is
    /// scissored by its rectangle and drawn untextured; otherwise by
    /// rectangle and uv, with the texture the magic file names: the
    /// GS `TEX0` fields (`TBP0`, `TBW`, `TW`, `TH`) and the magic id make
    /// `"<dir>/magic_<id>/tex/GCM/<tbp>_<tbw>_0_0_<w>_<h>.dds.phyre"`
    /// (`tex_cn` for magic 697 and 698 in Chinese), unified and interned
    /// (`Menu_AddName`); with no magic file open the TBP itself.
    pub fn draw_ui_element_after_set(
        h: &mut impl Host,
        frame: u32,
        lo: u32,
        hi: u32,
        a: u32,
        b: u32,
    ) {
        let (buf, path) = (frame, frame + 0x100);
        h.write(buf, 0_u8);
        h.call_sym(MEMSET, Conv::Cdecl, &[buf + 1, 0, 0xff]);
        let el = h.rebase(UI_ELEMENT);
        let clear = h.rebase(UI_ELEMENT_CLEAR);
        h.write(clear, [0_u32; 2]);
        if lo | hi == 0 {
            if h.call_sym(SCISSOR_XY, Conv::Cdecl, &[el, el + 4, el + 0x20, el + 0x24]) != 0 {
                h.call_sym(DRAW_UI_ELEMENT, Conv::Cdecl, &[el, 0, a, b, 0]);
            }
            return;
        }
        let ptrs = [
            el,
            el + 4,
            el + 0x20,
            el + 0x24,
            el + 8,
            el + 0xc,
            el + 0x28,
            el + 0x2c,
        ];
        if h.call_sym(SCISSOR_XYUV, Conv::Cdecl, &ptrs) == 0 {
            return;
        }
        let tex0 = u64::from(hi) << 32 | u64::from(lo);
        let tbp = lo & 0x3fff;
        let tbw = (tex0 >> 20) as u32 & 0x3f;
        let tw = 1_u32 << ((tex0 >> 26) as u32 & 0xf);
        let th = 1_u32 << ((tex0 >> 30) as u32 & 0xf);
        let mut tex = tbp;
        if h.call_sym(MAGIC_HANDLE, Conv::Cdecl, &[]) != u32::MAX {
            let dir = h.rebase(MAGIC_DIR);
            for k in 0..33 {
                let c = h.read::<u8>(dir + k);
                h.write(path + k, c);
            }
            h.call_sym(MEMSET, Conv::Cdecl, &[path + 0x21, 0, 0xdf]);
            let id = h.call_sym(MAGIC_ID, Conv::Cdecl, &[tbp, tbw, tw, th]);
            let f = h.rebase(MAGIC_TEX);
            import(h, SPRINTF, &[buf, f, path, id, tbp, tbw, tw, th]);
            if h.call_sym(FFX_LANG, Conv::Cdecl, &[]) == LANG_CHINESE {
                let (s1, s2) = (h.rebase(MAGIC_0697), h.rebase(MAGIC_0698));
                let m1 = import(h, STRSTR, &[buf, s1]);
                let m2 = import(h, STRSTR, &[buf, s2]);
                if m1 != 0 || m2 != 0 {
                    let id = h.call_sym(MAGIC_ID, Conv::Cdecl, &[tbp, tbw, tw, th]);
                    let f = h.rebase(MAGIC_TEX_CN);
                    import(h, SPRINTF, &[buf, f, path, id, tbp, tbw, tw, th]);
                }
            }
            h.call_sym(UNIFY_FILENAME, Conv::Cdecl, &[buf, path, 0x100]);
            tex = h.call_sym(INTERN_NAME, Conv::Cdecl, &[path]);
            h.call_sym(MENU_ADD_NAME, Conv::Cdecl, &[path]);
        }
        h.call_sym(DRAW_UI_ELEMENT, Conv::Cdecl, &[el, tex, a, b, 0]);
    }

    /// `MovieDecoder_runCommand_0077e460()` -> the record's last word, or
    /// the decoder state after a sound buffer error: the decoder's current
    /// command (`[[state + 0x5059c]]`, its data from `+0x10`) runs: 0x40
    /// delta and 0x50/0x51 SA audio decodes (into `state + 0x2ddc0`; the
    /// second channel at the block's end in stereo, `+0x506c8` 1), 0x1c30
    /// and 0x3860 (by the channel) block decodes, 0x3830/0x3f30 the
    /// viewer's audio, 0x6400/0x7800/0xc800 half mixes, 0x8000/0x8700 the
    /// movie step. A command counts (`+0x50720`); with none, a pending
    /// straight stream half (`+0x5076a`, `+0x5076b`, unless `+0x50766`)
    /// is sent when the next half is ready. Then the streaming record's
    /// seven words are copied to `+0x50730`, and a sound buffer error
    /// (its last word, with `+0x50765` set) is counted (`+0x50771`).
    pub fn movie_run_command<H9: Host>(h: &mut H9) -> u32 {
        let st = |h: &H9| word(h, h.rebase(IPU_STATE));
        let c = |h: &mut H9, f: u32, a: &[u32]| h.call_sym(f, Conv::Cdecl, a);
        let s = st(h);
        let cmd_block = word(h, s + 0x5_059c);
        let cmd = word(h, cmd_block);
        let data = cmd_block + 0x10;
        let stereo = word(h, s + 0x5_06c8);
        let out = s + 0x2_ddc0;
        let mut count = true;
        match cmd {
            0x40 => {
                let n = word(h, cmd_block + 4);
                let second = if stereo == 1 {
                    let blocks = (n.wrapping_add(15).cast_signed() / 16).cast_unsigned();
                    blocks
                        .wrapping_add(1)
                        .wrapping_shl(4)
                        .wrapping_add(cmd_block)
                } else {
                    0
                };
                c(h, AUDIO_DELTA_DECODE, &[data, out, n, second, s]);
            }
            0x50 | 0x51 => {
                let second = if stereo == 1 {
                    let d = word(h, cmd_block + 8).wrapping_sub(word(h, cmd_block + 0xc));
                    data.wrapping_add(d)
                } else {
                    0
                };
                let n = word(h, cmd_block + 4);
                c(h, AUDIO_SA_DECODE, &[data, out, n, second, s]);
            }
            0x1c30 => {
                c(h, AUDIO_DECODE_BLOCKS, &[data, out, 0, s]);
            }
            0x3860 => {
                let d = stereo.wrapping_mul(0x1c30).wrapping_add(data);
                c(h, AUDIO_DECODE_BLOCKS, &[d, out, 0, s]);
            }
            0x3830 | 0x3f30 => {
                c(h, MOVIE_VIEWER_AUDIO, &[]);
            }
            0x6400 | 0x7800 => {
                c(h, AUDIO_MIX_HALVES, &[data, 0, s]);
            }
            0xc800 => {
                let d = word(h, s + 0x5_0758)
                    .wrapping_mul(stereo)
                    .wrapping_add(data);
                c(h, AUDIO_MIX_HALVES, &[d, 0, s]);
            }
            0x8000 | 0x8700 => {
                c(h, MOVIE_STEP, &[]);
            }
            _ => count = cmd != 0,
        }
        if count {
            let s = st(h);
            let n = word(h, s + 0x5_0720).wrapping_add(1);
            h.write(s + 0x5_0720, n);
        } else {
            self::stream_half(h);
        }
        let rec = c(h, SQS_RECORD, &[]);
        let s = st(h);
        let flag = h.read::<u8>(s + 0x5_0765);
        for k in 0..7 {
            let v = word(h, rec + 4 * k);
            h.write(s + 0x5_0730 + 4 * k, v);
        }
        let err = word(h, rec + 0x18);
        if flag == 0 || err == 0 {
            return err;
        }
        let m = h.rebase(SOUND_BUFF_ERR);
        c(h, DBG_PRINTF, &[m]);
        let s = st(h);
        let n = h.read::<u8>(s + 0x5_0771).wrapping_add(1);
        h.write(s + 0x5_0771, n);
        s
    }

    /// The idle step of [`movie_run_command`]: a pending straight stream
    /// half sent once the next half is ready.
    fn stream_half(h: &mut impl Host) {
        let s = word(h, h.rebase(IPU_STATE));
        if h.read::<u8>(s + 0x5_0766) != 0 {
            return;
        }
        if h.read::<u8>(s + 0x5_076a) == 0 && h.read::<u8>(s + 0x5_076b) == 0 {
            return;
        }
        if h.call_sym(SQS_NEXT_HALF, Conv::Cdecl, &[]) == 0 {
            return;
        }
        let s = word(h, h.rebase(IPU_STATE));
        h.call_sym(ALREADY_STARTED_INC, Conv::Cdecl, &[s]);
        let s = word(h, h.rebase(IPU_STATE));
        let first = word(h, s + 0x5_0724).cast_signed();
        let second = word(h, s + 0x5_0728).cast_signed();
        let k = u32::from(first == -1 || (first > second && second >= 0));
        let tex = if word(h, s + 0x5_0714) != 0 && h.read::<u8>(s + k + 0x5_0768) != 0 {
            k.wrapping_mul(0x2300)
                .wrapping_add(0x4_bdc0)
                .wrapping_add(s)
        } else {
            0
        };
        let buf = k
            .wrapping_mul(0x7800)
            .wrapping_add(0x3_cdc0)
            .wrapping_add(s);
        let half = word(h, s + 0x5_0724 + 4 * k);
        h.call_sym(SND_SEND_STREAM, Conv::Cdecl, &[buf, tex, 0, half, s]);
        h.call_sym(SND_SEND_SYNC, Conv::Cdecl, &[1]);
        let s = word(h, h.rebase(IPU_STATE));
        h.write(s + k + 0x5_076a, 0_u8);
        h.write(s + k + 0x5_0768, 0_u8);
        h.write(s + 0x5_0724 + 4 * k, u32::MAX);
    }
}
