//! M5 stage 1, list `render-pcui` (round 5), batch 1: the scene's
//! finalize body and the character bound boxes of the graphic function
//! table.

use crate::m5r_c::quiet;
use ffx_bridge::Global;

pub const CLOSE_MENU: u32 = 0x006f_72a0;
pub const POST_SET_ALL_SWITCHES: u32 = 0x0069_89c0;
pub const POST_RELEASE: u32 = 0x0069_8f10;
pub const INSTANCE_GET: u32 = 0x006d_c8a0;
pub const BOOT_TEXT_TERMINATE: u32 = 0x006d_dfd0;
pub const BOOT_TEXT_RELEASE_BUFFER: u32 = 0x006d_dfa0;
/// The singletons released in order: (`GetInstance`, the method on it).
pub const SINGLETON_RELEASES: [(u32, u32); 5] = [
    (0x006e_32c0, 0x006e_3f10), // fog: resetAll
    (0x006a_bf40, 0x006a_cbd0), // morph: clear
    (0x006a_6e30, 0x006a_b540), // character lighting
    (0x006a_6e40, 0x006a_b5c0), // character lighting tweak util
    (0x0069_2bf0, 0x0069_8f60), // post process tweak util: terminate
];
pub const SAMPLER_RELEASE: u32 = 0x0059_c770;
/// `FUN_00636970`, run on the scene's `+0x10d54`.
pub const SCENE_LIST_RELEASE: u32 = 0x0063_6970;
pub const CLUSTER_DELETE_OBJECT: u32 = 0x0044_1240;
pub const ALIGNED_FREE: u32 = 0x0068_6fe0;
pub const USER_MALLOC: u32 = 0x0068_7020;
pub const USER_FREE: u32 = 0x0068_7010;
/// The arrays the finalize empties: (offset, its `Array_Reserve`).
pub const FINALIZE_ARRAYS: [(u32, u32); 10] = [
    (0x34, 0x0066_7e80),
    (0x4c, 0x0066_7e80),
    (0x58, 0x0066_7e00),
    (0x40, 0x0066_8200),
    (0xfb18, 0x0066_7e00),
    (0xfb30, 0x0066_8080),
    (0xfb24, 0x0066_8000),
    (0x28, 0x0066_7e00),
    (0xfb48, 0x0066_7f80),
    (0xfb54, 0x0066_8280),
];
/// The three object pools: (array offset, element size, object offset).
pub const POOLS: [(u32, u32, u32); 3] = [(0xfb7c, 8, 0), (0xfb90, 0x38, 0), (0xfba4, 0x1c, 4)];

pub const CHR_RENDER_INSTANCE: u32 = 0x0082_6b70;
pub const UPDATE_BOUNDS: u32 = 0x004f_96a0;
pub const MATRIX_FROM_3X4: u32 = 0x0045_1080;
pub const MATRIX_MUL_VEC4: u32 = 0x005b_4c00;
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
pub const PHYRE_FREE: u32 = 0x0042_fb20;
pub const COMPUTE_JOINTS: u32 = 0x004f_6ef0;
pub const CHR_BOUNDS: u32 = 0x0063_a410;
/// `16777216.0f` and `-16777216.0f` in `.rdata`.
pub const BIG: Global<u32> = Global::new(0x00b4_1da0);
pub const NEG_BIG: Global<u32> = Global::new(0x00b4_1db4);

pub const PRINTF_SLOT: u32 = 0x00b0_c380;
/// The formats of `graphicCharacterInstanceOutputAllVertexBuffers`.
pub const FMT_START: u32 = 0x00b4_1858;
pub const FMT_MESH_COUNT: u32 = 0x00b4_1880;
pub const FMT_MESH: u32 = 0x00b4_1890;
pub const FMT_SEGMENT: u32 = 0x00b4_18b4;
pub const FMT_VERTEX_DATA: u32 = 0x00b4_18e0;
pub const FMT_STREAM: u32 = 0x00b4_1920;
pub const FMT_FLOAT: u32 = 0x00b4_196c;
pub const FMT_BYTE: u32 = 0x00b4_1970;
pub const FMT_NEWLINE: u32 = 0x00b7_cae0;
/// The vertex stream type printed as bytes.
pub const BYTE_STREAM_TYPE: u32 = 0x00c9_8614;
pub const IS_BOUND: u32 = 0x0056_2da0;
pub const VB_CREATE: u32 = 0x0056_1a20;
pub const OS_SLEEP: u32 = 0x0043_0030;
pub const MAP_FOR_READ: u32 = 0x0056_2e00;
pub const VB_UNMAP: u32 = 0x0056_4ab0;

pub const FIND_PARAM: u32 = 0x0056_cbf0;
/// `"TextureSampler"`.
pub const TEXTURE_SAMPLER: u32 = 0x00b3_017c;
/// `"layer1"` .. `"layer6"`.
pub const LAYERS: [u32; 6] = [
    0x00b4_1724,
    0x00b4_172c,
    0x00b4_1734,
    0x00b4_173c,
    0x00b4_1744,
    0x00b4_174c,
];
/// The parameter types a sampler value is written into.
pub const SAMPLER_TYPES: [u8; 6] = [0x42, 0xd9, 0xda, 0xdb, 0xdc, 0x12];
pub const CLUSTER_MANAGER: u32 = 0x0069_a1a0;
pub const CLUSTER_BY_NAME: u32 = 0x0069_b4b0;
pub const TEXTURE_ITERATOR_CTOR: u32 = 0x0046_3bf0;
pub const LINEAR_SAMPLER: u32 = 0x0065_e050;
pub const SET_TEXTURE_CHECKED: u32 = 0x0056_e670;
/// `g_PhyreScene`.
pub const SCENE: Global<u32> = Global::new(0x00cc_b9d8);
/// `g_TexAnimSkip`: a count, then the characters 1..=count.
pub const TEXANIM_SKIP: u32 = 0x00cc_b230;

pub const SCENARIO_FLAG: u32 = 0x0078_7360;
pub const DRAW_UI_ELEMENT: u32 = 0x0063_eee0;
pub const RAND_SLOT: u32 = 0x00b0_c420;
/// The dream's alpha bytes (`6e 1e c8 5a`, one per element).
pub const DREAM_ALPHA: u32 = 0x5ac8_1e6e;
/// The dream level (`0x00c496d0`; the effect fades for 0..5).
pub const DREAM_LEVEL: u32 = 0x00c4_96d0;
/// `g_GraphicMapId`.
pub const MAP_ID: u32 = 0x0133_0844;
pub const DREAM_FADE_ON: u32 = 0x012f_fae8;
/// The fade (`f32`; 1.0 draws nothing).
pub const DREAM_FADE: u32 = 0x012f_faf4;
/// `g_PauseOn`.
pub const PAUSE_ON: u32 = 0x0134_0828;
/// The four elements saved for the pause (0x98 bytes each).
pub const DREAM_SAVED: u32 = 0x00cc_b470;
/// `g_DreamMapTextureNames`.
pub const DREAM_TEXTURES: u32 = 0x00c5_9ebc;
/// The frame counter of the fourth element (`u16`, 0..2).
pub const DREAM_COUNTER: u32 = 0x00cc_b46c;
/// `.rdata` doubles: 32767.0, 1.875, 1.0546875, 512.0, 416.0, 10.5.
pub const DREAM_DIV: u32 = 0x00b4_1d70;
pub const DREAM_U1: u32 = 0x00b4_1d08;
pub const DREAM_V1: u32 = 0x00b4_1cf8;
pub const DREAM_SCALE_W: u32 = 0x00b4_1d38;
pub const DREAM_SCALE_H: u32 = 0x00b4_1d30;
pub const DREAM_OFFSET: u32 = 0x00b4_1d28;
/// `.rdata` floats 512.0 and 416.0.
pub const DREAM_SIZE_W: u32 = 0x00b4_1d9c;
pub const DREAM_SIZE_H: u32 = 0x00b4_1d98;

/// `g_ClothHeap` and the start of its megabyte.
pub const CLOTH_HEAP: u32 = 0x00cc_b454;
pub const CLOTH_HEAP_BASE: u32 = 0x00cc_b458;
pub const MSPACE_FREE: u32 = 0x006e_bee0;
/// `FUN_00635630(count, items)`: a `PhyreMalloc`ed copy of a word list.
pub const LIST_COPY: u32 = 0x0063_5630;
pub const ARRAY_DATA: u32 = 0x004d_45a0;
/// `"Ball_Center"`, `"Ball_Range"`, `"Ball_Rate"`.
pub const BALL_CENTER: u32 = 0x00b4_1754;
pub const BALL_RANGE: u32 = 0x00b4_1760;
pub const BALL_RATE: u32 = 0x00b4_176c;
/// `"VertexColorHeightNow"`, `"VertexColorHeightGradient"`,
/// `"VertexColorOrigin"`, `"VertexColorModify"`.
pub const COLOR_HEIGHT_NOW: u32 = 0x00b4_19d0;
pub const COLOR_HEIGHT_GRADIENT: u32 = 0x00b4_19e8;
pub const COLOR_ORIGIN: u32 = 0x00b4_1a04;
pub const COLOR_MODIFY: u32 = 0x00b4_1a18;

/// `g_LoadingScreen`.
pub const LOADING_SCREEN: u32 = 0x00ce_517c;
pub const GET_APPLICATION: u32 = 0x0062_3b90;
pub const SET_PARAMETER_MANUALLY: u32 = 0x006e_4ff0;
/// `.rdata` 3.0f.
pub const LOADING_TIME: u32 = 0x00b3_eb94;
pub const TREE_SUCCESSOR: u32 = 0x0057_5160;
/// `"clrScaleFactor"`.
pub const CLR_SCALE_FACTOR: u32 = 0x00b4_3c30;
/// `.rdata` double 1e-4 (as an `f32`).
pub const COLOR_SCALE_MIN: u32 = 0x00b3_eb50;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::{Mem, x87};
    use ffx_graphic::host::{Conv, Host};

    fn word(h: &impl Mem, at: u32) -> u32 {
        h.read(at)
    }

    fn f(h: &impl Mem, at: u32) -> f64 {
        f64::from(f32::from_bits(word(h, at)))
    }

    /// `fld dword` / `fstp dword`: the word, a signalling NaN quieted.
    fn copy(h: &mut impl Mem, from: u32, to: u32) {
        let v = quiet(word(h, from));
        h.write(to, v);
    }

    /// `if (count < 0 && capacity < 0) Array_Reserve(0)`, then the count
    /// cleared: an owned array emptied.
    fn empty_array(h: &mut impl Host, at: u32, reserve: u32) {
        if (word(h, at) as i32) < 0 && (word(h, at + 4) as i32) < 0 {
            h.call_sym(reserve, Conv::Thiscall(at), &[0]);
        }
        h.write(at, 0_u32);
    }

    /// An owned byte array (`+0` count, `+4` capacity, `+8` data) emptied
    /// with `user_malloc(0)` / `user_free_thunk`.
    fn empty_flags(h: &mut impl Host, at: u32) {
        if (word(h, at) as i32) < 0 && (word(h, at + 4) as i32) < 0 {
            let data = word(h, at + 8);
            if data == 0 {
                let p = h.call_sym(USER_MALLOC, Conv::Cdecl, &[0]);
                h.write(at + 8, p);
            } else {
                h.call_sym(USER_FREE, Conv::Cdecl, &[data]);
                h.write(at + 8, 0_u32);
            }
            h.write(at + 4, 0_u32);
        }
        h.write(at, 0_u32);
    }

    /// `PhyreScene_finalize_00659d20(scene)`: the menu closed, the post
    /// process manager (`+0x10cc`) switched off, released and cleared, the
    /// boot text, fog, morph, lighting and tweak singletons released, the
    /// sampler at `+0xfaa8` released, ten arrays emptied, `+0x10d54`
    /// released, `+0xfb60..+0xfb78` cleared; then for each of 3000 slots,
    /// the objects of the three pools (flag bytes at `+0xfb88`, `+0xfb9c`,
    /// `+0xfbb0`) deleted from the cluster `+0x12d4`; the block `+0xfbb8`
    /// freed and the four flag arrays emptied.
    pub fn finalize(h: &mut impl Host, this: u32) {
        h.call_sym(CLOSE_MENU, Conv::Cdecl, &[]);
        let post = word(h, this + 0x10cc);
        h.call_sym(POST_SET_ALL_SWITCHES, Conv::Thiscall(post), &[0]);
        let post = word(h, this + 0x10cc);
        h.call_sym(POST_RELEASE, Conv::Thiscall(post), &[]);
        h.write(this + 0x10cc, 0_u32);
        let boot = h.call_sym(INSTANCE_GET, Conv::Cdecl, &[]);
        h.call_sym(
            BOOT_TEXT_TERMINATE,
            Conv::Thiscall(boot.wrapping_add(0x110)),
            &[],
        );
        for (get, method) in SINGLETON_RELEASES {
            let s = h.call_sym(get, Conv::Cdecl, &[]);
            h.call_sym(method, Conv::Thiscall(s), &[]);
        }
        let boot = h.call_sym(INSTANCE_GET, Conv::Cdecl, &[]);
        h.call_sym(BOOT_TEXT_RELEASE_BUFFER, Conv::Thiscall(boot), &[]);
        h.call_sym(SAMPLER_RELEASE, Conv::Thiscall(this + 0xfaa8), &[]);
        for (k, (off, reserve)) in FINALIZE_ARRAYS.into_iter().enumerate() {
            empty_array(h, this + off, reserve);
            if k == 3 {
                h.call_sym(SCENE_LIST_RELEASE, Conv::Thiscall(this + 0x10d54), &[]);
            }
        }
        for off in (0xfb60..=0xfb78).step_by(4) {
            h.write(this + off, 0_u32);
        }
        for i in 0..3000_u32 {
            for (array, size, extra) in POOLS {
                if i >= word(h, this + array + 4) {
                    continue;
                }
                let flags = word(h, this + array + 0xc);
                if h.read::<u8>(flags.wrapping_add(i)) == 0 {
                    continue;
                }
                let at = word(h, this + array).wrapping_add(i.wrapping_mul(size));
                if at == 0 || at.wrapping_add(extra) == 0 {
                    continue;
                }
                h.call_sym(
                    CLUSTER_DELETE_OBJECT,
                    Conv::Thiscall(this + 0x12d4),
                    &[at + extra],
                );
            }
        }
        let block = word(h, this + 0xfbb8);
        h.call_sym(ALIGNED_FREE, Conv::Cdecl, &[block]);
        for off in [0xfb80, 0xfb94, 0xfba8, 0xfbbc] {
            empty_flags(h, this + off);
        }
    }

    /// `min[k] = v[k]` when `v[k] < min[k]`, `max[k] = w[k]` when
    /// `max[k] < w[k]` (ordered compares), for `k` in 0..3.
    fn grow(h: &mut impl Mem, min: u32, max: u32, v: u32, w: u32) {
        for k in 0..3 {
            if f(h, min + 4 * k) > f(h, v + 4 * k) {
                copy(h, v + 4 * k, min + 4 * k);
            }
            if f(h, max + 4 * k) < f(h, w + 4 * k) {
                copy(h, w + 4 * k, max + 4 * k);
            }
        }
    }

    /// Writes three copies of the `.rdata` float `c` from `at`.
    fn fill3(h: &mut impl Mem, at: u32, c: Global<u32>) {
        let v = quiet(c.get(h));
        for k in 0..3 {
            h.write(at + 4 * k, v);
        }
    }

    /// `graphicCharacterGetBounds_0063a410(chr, max, min)` (the frame from
    /// `ebp - 0xc4` at `frame`: the matrix at `+0`, the two products at
    /// `+0x40`, `+0x50`, the corner pair at `+0x9c`): `min` = 2^24 and
    /// `max` = -2^24 in each axis, then each mesh instance of the
    /// character's render instance has its bounds updated from the
    /// skeleton and its box (`+0x1c`: the corner at `+0`, the size at
    /// `+0x10`, a 3x4 transform at `+0xc`) grown into them: transformed
    /// when it has a transform (both corners as `w = 1` points, then
    /// sorted per axis). -> what the original leaves in `eax`.
    pub fn chr_bounds(h: &mut impl Host, frame: u32, chr: u32, max: u32, min: u32) -> u32 {
        let inst = h.call_sym(CHR_RENDER_INSTANCE, Conv::Cdecl, &[chr]);
        if inst == 0 {
            return 0;
        }
        fill3(h, min, BIG);
        fill3(h, max, NEG_BIG);
        let corners = frame + 0x9c;
        let meshes = word(h, inst) & 0x7fff_ffff;
        if meshes == 0 {
            return min;
        }
        let mut i = 0;
        while i < word(h, inst) & 0x7fff_ffff {
            let mesh = word(h, word(h, inst + 4).wrapping_add(4 * i));
            h.call_sym(UPDATE_BOUNDS, Conv::Thiscall(mesh), &[]);
            let mesh = word(h, word(h, inst + 4).wrapping_add(4 * i));
            let b = word(h, mesh.wrapping_add(0x1c));
            let mut lo = [0_u32; 3];
            let mut size = [0_u32; 3];
            for k in 0..3 {
                lo[k as usize] = quiet(word(h, b.wrapping_add(4 * k)));
                size[k as usize] = quiet(word(h, b.wrapping_add(0x10 + 4 * k)));
            }
            let transform = word(h, b.wrapping_add(0xc));
            let one = 1.0_f32.to_bits();
            for k in 0..3 {
                h.write(corners + 4 * k, lo[k as usize]);
            }
            h.write(corners + 0xc, one);
            for k in 0..3 {
                let sum = x87::add(x87::load(lo[k as usize]), x87::load(size[k as usize]));
                h.write(corners + 0x10 + 4 * k, x87::store(sum));
            }
            h.write(corners + 0x1c, one);
            if transform != 0 {
                h.call_sym(MATRIX_FROM_3X4, Conv::Thiscall(frame), &[transform]);
                for (out, at) in [(frame + 0x50, corners), (frame + 0x40, corners + 0x10)] {
                    let p = h.call_sym(MATRIX_MUL_VEC4, Conv::Thiscall(frame), &[out, at]);
                    for k in 0..4 {
                        copy(h, p.wrapping_add(4 * k), at + 4 * k);
                    }
                }
                for k in 0..3 {
                    let (a, b) = (corners + 4 * k, corners + 0x10 + 4 * k);
                    if f(h, b) < f(h, a) {
                        let t = quiet(word(h, a));
                        copy(h, b, a);
                        h.write(b, quiet(t));
                    }
                }
            }
            grow(h, min, max, corners, corners + 0x10);
            i += 1;
        }
        word(h, inst) & 0x7fff_ffff
    }

    /// `graphicCharacterGetJointBounds_0063a610(chr, max, min, jmax,
    /// jmin)` (the corner at `frame`, the original's `ebp - 0x24`):
    /// [`chr_bounds`] into `max`/`min`, then `jmin` = 2^24 and `jmax` =
    /// -2^24 and each mesh instance's joint matrices (computed into a
    /// `PhyreMalloc` block of `count + 1` matrices, from the second)
    /// whose translation lies inside `min..max` grow them; the block freed
    /// unless its count has the top bit. -> the render instance.
    pub fn chr_joint_bounds(
        h: &mut impl Host,
        frame: u32,
        chr: u32,
        max: u32,
        min: u32,
        jmax: u32,
        jmin: u32,
    ) -> u32 {
        let inst = h.call_sym(CHR_RENDER_INSTANCE, Conv::Cdecl, &[chr]);
        if inst == 0 {
            return 0;
        }
        h.call_sym(CHR_BOUNDS, Conv::Cdecl, &[chr, max, min]);
        fill3(h, jmin, BIG);
        fill3(h, jmax, NEG_BIG);
        let mut i = 0;
        while i < word(h, inst) & 0x7fff_ffff {
            let mesh = word(h, word(h, inst + 4).wrapping_add(4 * i));
            i += 1;
            if mesh == 0 {
                continue;
            }
            let object = word(h, mesh);
            let count = word(h, object.wrapping_add(0x18)) & 0x7fff_ffff;
            if count == 0 {
                continue;
            }
            let n = count + 1;
            let block = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[n << 6, 4]);
            if block == 0 {
                continue;
            }
            let joints = block + 0x40;
            h.call_sym(COMPUTE_JOINTS, Conv::Thiscall(mesh), &[joints]);
            if word(h, object.wrapping_add(0x2c)) != 0 {
                for j in 0..count {
                    let t = joints.wrapping_add(0x30 + 0x40 * j);
                    for k in 0..3 {
                        copy(h, t.wrapping_add(4 * k), frame + 4 * k);
                    }
                    let inside = (0..3).all(|k| {
                        let v = f(h, frame + 4 * k);
                        v >= f(h, min + 4 * k) && f(h, max + 4 * k) >= v
                    });
                    if inside {
                        grow(h, jmin, jmax, frame, frame);
                    }
                }
            }
            if n >> 31 == 0 {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[block]);
            }
        }
        inst
    }

    /// `printf(format, args...)`: the format is an `.rdata` address.
    fn printf(h: &mut impl Host, format: u32, args: &[u32]) {
        let f = word(h, h.rebase(PRINTF_SLOT));
        let mut words = vec![h.rebase(format)];
        words.extend_from_slice(args);
        h.call(f, Conv::Cdecl, &words);
    }

    /// The data of a `PArray` whose one element is stored in place: the
    /// pointer at `+4` for two or more, `+4` itself for one, 0 for none.
    fn array_data(h: &impl Mem, a: u32) -> u32 {
        match word(h, a) & 0x7fff_ffff {
            0 => 0,
            1 => a.wrapping_add(4),
            _ => word(h, a.wrapping_add(4)),
        }
    }

    /// `graphicCharacterInstanceOutputAllVertexBuffers(inst)` (the mapped
    /// range `{data, size}` at `frame`, the original's `ebp - 0x58`):
    /// printf dumps every mesh, segment, vertex data and stream of the
    /// render instance; each stream is mapped for reading (bound, or
    /// created; waiting with `Sleep(1)` while its buffer is busy; the
    /// data's `+0x14` set once mapped), its rows printed as bytes (the
    /// type `0x00c98614`) or floats, then unmapped (`+0x14` cleared).
    /// When the buffer is neither bound nor created, the range left by the
    /// stream before is printed; for the first one the original reads its
    /// never-written frame (the port's frame starts zeroed). A stride of 0
    /// (or `i32::MIN` over -1) faults the original's `idiv`; the port
    /// returns there.
    pub fn output_vertex_buffers(h: &mut impl Host, frame: u32, inst: u32) {
        if inst == 0 {
            return;
        }
        printf(h, FMT_START, &[]);
        let meshes = word(h, inst) & 0x7fff_ffff;
        printf(h, FMT_MESH_COUNT, &[meshes]);
        for m in 0..meshes {
            let mesh = word(h, word(h, word(h, inst + 4).wrapping_add(4 * m)));
            let segments = word(h, mesh) & 0x7fff_ffff;
            printf(h, FMT_MESH, &[m, segments]);
            for s in 0..segments {
                let seg = word(h, mesh.wrapping_add(4)).wrapping_add(0x6c * s);
                let datas = word(h, seg.wrapping_add(0x28)) & 0x7fff_ffff;
                printf(h, FMT_SEGMENT, &[s, datas]);
                for d in 0..datas {
                    let vd = word(h, seg.wrapping_add(0x2c)).wrapping_add(0x40 * d);
                    let streams = word(h, vd.wrapping_add(8)) & 0x7fff_ffff;
                    let stride = word(h, vd);
                    printf(h, FMT_VERTEX_DATA, &[d, streams, stride]);
                    for k in 0..streams {
                        if !dump_stream(h, frame, vd, k, streams, stride) {
                            return;
                        }
                    }
                }
            }
        }
    }

    /// One stream of [`output_vertex_buffers`]; false where the original
    /// faults.
    fn dump_stream(h: &mut impl Host, frame: u32, vd: u32, k: u32, n: u32, stride: u32) -> bool {
        let list = word(h, vd.wrapping_add(0xc));
        let e = list.wrapping_add(0xc * k);
        let offset = word(h, e);
        let end = if k == n - 1 {
            stride
        } else {
            word(h, e.wrapping_add(0xc))
        };
        let cell = end.wrapping_sub(offset);
        let ty = word(h, e.wrapping_add(4));
        let bound = h.call_sym(IS_BOUND, Conv::Thiscall(vd), &[]) as u8 != 0;
        if bound || h.call_sym(VB_CREATE, Conv::Thiscall(vd), &[]) == 0 {
            let idx = word(h, vd.wrapping_add(0x10));
            let buffers = array_data(h, vd.wrapping_add(0x18));
            let busy = buffers
                .wrapping_add(idx.wrapping_mul(20))
                .wrapping_add(0x10);
            while (word(h, busy) as i32) > 0 {
                h.call_sym(OS_SLEEP, Conv::Cdecl, &[1]);
            }
            if h.call_sym(MAP_FOR_READ, Conv::Thiscall(vd), &[idx, frame]) == 0 {
                h.write(vd.wrapping_add(0x14), 1_u8);
            }
        }
        let Some(rows) = (word(h, frame + 4) as i32).checked_div(stride as i32) else {
            return false;
        };
        let base = word(h, frame);
        let name = word(h, ty);
        printf(h, FMT_STREAM, &[k, name, offset, cell, rows as u32]);
        let mut row = offset.wrapping_add(base);
        for _ in 0..rows.max(0) {
            if ty == h.rebase(BYTE_STREAM_TYPE) {
                for b in 0..cell {
                    let v = h.read::<u8>(row.wrapping_add(b));
                    printf(h, FMT_BYTE, &[u32::from(v)]);
                }
            } else {
                for w in 0..cell >> 2 {
                    let bits = x87::load(word(h, row.wrapping_add(4 * w))).to_bits();
                    printf(h, FMT_FLOAT, &[bits as u32, (bits >> 32) as u32]);
                }
            }
            printf(h, FMT_NEWLINE, &[]);
            row = row.wrapping_add(stride);
        }
        let idx = word(h, vd.wrapping_add(0x10));
        if h.call_sym(VB_UNMAP, Conv::Thiscall(vd), &[idx, frame]) == 0 {
            h.write(vd.wrapping_add(0x14), 0_u8);
        }
        true
    }

    /// Each mesh segment's material of the render instance `inst`:
    /// `f(h, params, buffer)` with the material entry's parameter set
    /// (`[entry]`) and buffer (`[entry + 4]`), the entry picked by the
    /// segment's first word from the mesh entry's material array (`+0x10`).
    fn each_material<H: Host>(h: &mut H, inst: u32, mut f: impl FnMut(&mut H, u32, u32)) {
        let mut i = 0;
        while i < word(h, inst) & 0x7fff_ffff {
            let mesh = word(h, word(h, word(h, inst + 4).wrapping_add(4 * i)));
            let mut s = 0;
            while s < word(h, mesh) & 0x7fff_ffff {
                let idx = word(h, word(h, mesh.wrapping_add(4)).wrapping_add(0x6c * s));
                let entry = word(h, word(h, inst + 4).wrapping_add(4 * i));
                let arr = word(h, entry.wrapping_add(0x10));
                let data = array_data(h, arr);
                let params = word(h, word(h, data.wrapping_add(4 * idx)));
                let data = array_data(h, arr);
                let buffer = word(h, word(h, data.wrapping_add(4 * idx)).wrapping_add(4));
                f(h, params, buffer);
                s += 1;
            }
            i += 1;
        }
    }

    /// `memcpy(buffer + offset, value, size)` for the parameter `p` (`+8`
    /// offset, `+0xa` size in its low 13 bits) when it fits the buffer
    /// (its size at `+0`) and the offset is at least 4. `value` holds the
    /// original's local; the original copies `size` bytes from it, past
    /// the local into its frame when `size` is larger (the port writes 0
    /// there: the parameters are the local's size in the game data).
    fn write_param(h: &mut impl Mem, buffer: u32, p: u32, value: &[u8]) {
        let offset = u32::from(h.read::<u16>(p.wrapping_add(8)));
        let size = u32::from(h.read::<u16>(p.wrapping_add(0xa)) & 0x1fff);
        if offset + size > word(h, buffer) || offset < 4 {
            return;
        }
        for b in 0..size {
            // junk read: the bytes past the local are the original's frame; 0 (brief 2026-10-24)
            let v = value.get(b as usize).copied().unwrap_or(0);
            h.write(buffer.wrapping_add(offset + b), v);
        }
    }

    /// The `TextureSampler` value `{0x42, 0, 0, 0}` written into each
    /// sampler-typed parameter of the instance's materials.
    fn reset_samplers(h: &mut impl Host, inst: u32) {
        let mut value = [0_u8; 16];
        value[0] = 0x42;
        each_material(h, inst, |h, params, buffer| {
            let p = h.call_sym(
                FIND_PARAM,
                Conv::Thiscall(params),
                &[h.rebase(TEXTURE_SAMPLER)],
            );
            if p != 0 && SAMPLER_TYPES.contains(&h.read::<u8>(p.wrapping_add(2))) {
                write_param(h, buffer, p, &value);
            }
        });
    }

    /// `graphicCharacterSetSummonTexture_0063c0c0(inst, cluster_name)`
    /// (the texture iterator at `frame`, the original's `ebp - 0x44`; the
    /// sampler value at `+0x10`): without a name, every material's
    /// `TextureSampler` is reset to `{0x42, 0, 0, 0}`; with one, the
    /// named cluster's first texture (iterator `+0` plus `+0x10`; none
    /// when the iterator's `+0xc` is 0) is set into every material's
    /// `TextureSampler` with the scene's linear sampler.
    pub fn set_summon_texture(h: &mut impl Host, frame: u32, inst: u32, name: u32) {
        if inst == 0 {
            return;
        }
        if name == 0 {
            reset_samplers(h, inst);
            return;
        }
        let cm = h.call_sym(CLUSTER_MANAGER, Conv::Cdecl, &[]);
        let cluster = h.call_sym(CLUSTER_BY_NAME, Conv::Thiscall(cm), &[name]);
        if cluster == 0 {
            return;
        }
        h.call_sym(TEXTURE_ITERATOR_CTOR, Conv::Thiscall(frame), &[cluster]);
        if word(h, frame + 0xc) == 0 {
            return;
        }
        let texture = word(h, frame).wrapping_add(word(h, frame + 0x10));
        let value = frame + 0x10;
        each_material(h, inst, |h, params, buffer| {
            let p = h.call_sym(
                FIND_PARAM,
                Conv::Thiscall(params),
                &[h.rebase(TEXTURE_SAMPLER)],
            );
            if p != 0 {
                h.write(value, 0x42_u32);
                h.write(value + 0xc, texture);
                let scene = SCENE.get(h);
                let sampler = h.call_sym(LINEAR_SAMPLER, Conv::Thiscall(scene), &[1]);
                h.write(value + 8, sampler);
                h.write(value + 4, 0_u32);
                h.call_sym(SET_TEXTURE_CHECKED, Conv::Thiscall(buffer), &[p, value]);
            }
        });
    }

    /// `graphicCharacterClearSummonLayers_0063dad0(inst)`: the float 0.0
    /// written into the parameters `layer1` .. `layer6` of every material.
    pub fn clear_summon_layers(h: &mut impl Host, inst: u32) {
        if inst == 0 {
            return;
        }
        each_material(h, inst, |h, params, buffer| {
            for name in LAYERS {
                let p = h.call_sym(FIND_PARAM, Conv::Thiscall(params), &[h.rebase(name)]);
                if p != 0 {
                    write_param(h, buffer, p, &[0; 4]);
                }
            }
        });
    }

    /// `graphicCharacterRestoreSummonTexture()`: each character of
    /// `g_TexAnimSkip` (1..=count, the count read again after each) has its
    /// samplers reset as [`set_summon_texture`] does without a name and
    /// its entry cleared; then the count. -> the count read last.
    pub fn restore_summon_texture(h: &mut impl Host) -> u32 {
        let list = h.rebase(TEXANIM_SKIP);
        let mut count = word(h, list);
        let mut k = 1_u32;
        while (k as i32) <= (count as i32) {
            let inst = word(h, list + 4 * k);
            if inst != 0 {
                reset_samplers(h, inst);
                h.write(list + 4 * k, 0_u32);
                count = word(h, list);
            }
            k += 1;
        }
        h.write(list, 0_u32);
        count
    }

    /// `int` to `f32` the way `fild` / `fstp dword` does it.
    fn int_f32(i: u32) -> u32 {
        x87::store(f64::from(i as i32))
    }

    /// `fistp dword` with truncation: the low byte of the 32-bit result
    /// (the indefinite integer for NaN and out-of-range values).
    fn trunc_byte(x: f64) -> u8 {
        let i = if x.is_nan() || x >= 2_147_483_648.0 || x <= -2_147_483_649.0 {
            0x8000_0000_u32
        } else {
            x.trunc() as i32 as u32
        };
        i as u8
    }

    /// `f32(a + b)` with the x87's NaN rules (operands as loaded).
    fn add_f32(a: f64, b: f64) -> u32 {
        x87::store(x87::add(a, b))
    }

    /// Writes the colour words of a UI element: `rgb` at `+0x10` and
    /// `+0x30`, `alpha` at `+0x1c` and `+0x3c`.
    fn element_colours(h: &mut impl Mem, e: u32, rgb: [u32; 3], alpha: u32) {
        for (k, c) in (0u32..).zip(rgb) {
            h.write(e + 0x10 + 4 * k, c);
            h.write(e + 0x30 + 4 * k, c);
        }
        h.write(e + 0x1c, alpha);
        h.write(e + 0x3c, alpha);
    }

    /// The `.rdata` floats 512.0 and 416.0 at `+0x20`, `+0x24`.
    fn element_size(h: &mut impl Mem, e: u32) {
        copy(h, h.rebase(DREAM_SIZE_W), e + 0x20);
        copy(h, h.rebase(DREAM_SIZE_H), e + 0x24);
    }

    fn draw(h: &mut impl Host, e: u32, texture: u32) {
        h.call_sym(DRAW_UI_ELEMENT, Conv::Cdecl, &[e, texture, 1, 0, 0]);
    }

    fn rand_f32(h: &mut impl Host) -> f64 {
        let f = word(h, h.rebase(RAND_SLOT));
        let r = h.call(f, Conv::Cdecl, &[]);
        x87::load(int_f32(r))
    }

    fn rdata_f64(h: &impl Mem, sym: u32) -> f64 {
        f64::from_bits(h.read::<u64>(h.rebase(sym)))
    }

    /// `graphicDrawDream()` (the four 0x98-byte UI elements at `frame`, the
    /// original's `ebp - 0x264`, in the order they are saved): the alpha
    /// bytes `6e 1e c8 5a`, scaled by `1 - g_DreamFade` (truncated) while
    /// `g_DreamLevel` is 0..5 (not on map 0x86 with scenario 0x38e and
    /// level 0) and `g_DreamFadeOn` is set; nothing is drawn when the fade
    /// is 1.0, or when it is off for those levels. With the pause on, the
    /// four saved elements are drawn again. Otherwise three elements with
    /// random offsets (`rand() / 32767`) are drawn, the fourth three times
    /// every third frame (counter `0x00ccb46c`), and all four saved to
    /// `0x00ccb470..`. The words of the elements the original never writes
    /// (`+0x40..`, and the fourth on the frames it isn't drawn) are 0 in
    /// the port (junk read: [ebp-0x264..] never written; 0, brief
    /// 2026-10-24).
    pub fn draw_dream(h: &mut impl Host, frame: u32) {
        let mut alpha = DREAM_ALPHA.to_le_bytes();
        let scenario = u32::from(h.call_sym(SCENARIO_FLAG, Conv::Cdecl, &[]) as u16);
        let level = word(h, h.rebase(DREAM_LEVEL));
        let special = word(h, h.rebase(MAP_ID)) == 0x86 && scenario == 0x38e && level == 0;
        if !special && (level as i32) < 6 && level != 0xffff_ffff {
            if word(h, h.rebase(DREAM_FADE_ON)) == 0 {
                return;
            }
            for b in &mut alpha {
                let v = x87::load(word(h, h.rebase(DREAM_FADE)));
                #[allow(clippy::float_cmp)] // fucom: exactly 1.0
                if v == 1.0 {
                    return;
                }
                let scaled = x87::mul(x87::sub(1.0, v), f64::from(*b));
                *b = trunc_byte(scaled);
            }
        }
        let saved = h.rebase(DREAM_SAVED);
        let textures = h.rebase(DREAM_TEXTURES);
        if word(h, h.rebase(PAUSE_ON)) != 0 {
            for (k, t) in [(1_u32, Some(3_u32)), (3, None), (2, Some(4)), (0, Some(0))] {
                let tex = t.map_or(0, |i| word(h, textures + 4 * i));
                draw(h, saved + 0x98 * k, tex);
            }
            return;
        }
        let [e0, e1, e2, e3] = [frame, frame + 0x98, frame + 0x130, frame + 0x1c8];
        let c = rdata_f64(h, DREAM_DIV);
        let w = x87::store(x87::div(rand_f32(h), c));
        let hh = x87::store(x87::div(rand_f32(h), c));
        let (w, hh) = (x87::load(w), x87::load(hh));
        element_colours(h, e1, [0xbe; 3], u32::from(alpha[1]));
        h.write(e1, 0_u32);
        h.write(e1 + 4, 0_u32);
        h.write(e1 + 8, add_f32(w, 0.0));
        h.write(e1 + 0xc, add_f32(0.0, hh));
        element_size(h, e1);
        h.write(e1 + 0x28, add_f32(w, rdata_f64(h, DREAM_U1)));
        h.write(e1 + 0x2c, add_f32(hh, rdata_f64(h, DREAM_V1)));
        let tex = word(h, textures + 0xc);
        draw(h, e1, tex);
        let one = 1.0_f32.to_bits();
        for (e, rgb, a) in [
            (e3, [0xba, 0x9e, 0x55], alpha[3]),
            (e2, [0x80; 3], alpha[2]),
        ] {
            for k in 0..4 {
                h.write(e + 4 * k, 0_u32);
            }
            element_colours(h, e, rgb, u32::from(a));
            element_size(h, e);
            h.write(e + 0x28, one);
            h.write(e + 0x2c, one);
            let tex = if e == e3 { 0 } else { word(h, textures + 0x10) };
            draw(h, e, tex);
        }
        let counter = h.rebase(DREAM_COUNTER);
        let n = h.read::<u16>(counter);
        if n == 0 {
            h.write(counter, 1_u16);
            let (sx, sy) = (rdata_f64(h, DREAM_SCALE_W), rdata_f64(h, DREAM_SCALE_H));
            let off = rdata_f64(h, DREAM_OFFSET);
            for k in 0..3 {
                let w = x87::store(x87::div(x87::mul(rand_f32(h), sx), c));
                let hh = x87::store(x87::div(x87::mul(rand_f32(h), sy), c));
                let (w, hh) = (x87::load(w), x87::load(hh));
                element_colours(h, e0, [0x80; 3], u32::from(alpha[0]));
                h.write(e0, add_f32(w, 0.0));
                h.write(e0 + 4, add_f32(0.0, hh));
                h.write(e0 + 8, 0_u32);
                h.write(e0 + 0xc, 0_u32);
                h.write(e0 + 0x20, add_f32(w, off));
                h.write(e0 + 0x24, add_f32(hh, off));
                h.write(e0 + 0x28, one);
                h.write(e0 + 0x2c, one);
                let tex = word(h, textures + 4 * k);
                draw(h, e0, tex);
            }
        } else {
            let n = n.wrapping_add(1);
            h.write(counter, if n == 3 { 0 } else { n });
        }
        for k in 0..4 {
            for w in 0..0x26 {
                let v = word(h, frame + 0x98 * k + 4 * w);
                h.write(saved + 0x98 * k + 4 * w, v);
            }
        }
    }

    /// Frees `p` into the cloth heap when it lies in its megabyte, with
    /// `user_free_thunk` otherwise. -> the callee's `eax`.
    fn cloth_free(h: &mut impl Host, p: u32) -> u32 {
        let base = word(h, h.rebase(CLOTH_HEAP_BASE));
        if p >= base && p < base.wrapping_add(0x10_0000) {
            let heap = word(h, h.rebase(CLOTH_HEAP));
            h.call_sym(MSPACE_FREE, Conv::Cdecl, &[heap, p])
        } else {
            h.call_sym(USER_FREE, Conv::Cdecl, &[p])
        }
    }

    /// `Cache_FreeClothVertexBuff_00640e70(slot)`: the three-level tree at
    /// `[slot]` (each node a count and that many child pointers; a leaf a
    /// count and three buffers) freed bottom up: a leaf whose first buffer
    /// is set has its three buffers freed and cleared (once per count:
    /// cleared, the later rounds find nothing), then each leaf, node and
    /// the root; `[slot]` cleared. -> the last free's `eax`.
    pub fn free_cloth_buffers(h: &mut impl Host, slot: u32) -> u32 {
        let root = word(h, slot);
        let mut i = 0;
        while i < word(h, root) {
            let node = word(h, root + 4 + 4 * i);
            i += 1;
            if node == 0 {
                continue;
            }
            let mut j = 0;
            while j < word(h, node) {
                let leaf = word(h, node + 4 + 4 * j);
                j += 1;
                if leaf == 0 {
                    continue;
                }
                let mut k = 0;
                while k < word(h, leaf) {
                    k += 1;
                    if word(h, leaf + 4) == 0 {
                        continue;
                    }
                    for off in [4, 8, 0xc] {
                        let p = word(h, leaf + off);
                        cloth_free(h, p);
                        h.write(leaf + off, 0_u32);
                    }
                }
                cloth_free(h, leaf);
            }
            cloth_free(h, node);
        }
        let r = cloth_free(h, root);
        h.write(slot, 0_u32);
        r
    }

    /// Copies `size` bytes from `src` (read first, as a `memmove`) to
    /// `buffer + offset` for the parameter `p`, under [`write_param`]'s
    /// conditions.
    fn copy_param(h: &mut impl Mem, buffer: u32, p: u32, src: u32) {
        let size = u32::from(h.read::<u16>(p.wrapping_add(0xa)) & 0x1fff);
        let bytes: Vec<u8> = (0..size).map(|b| h.read(src.wrapping_add(b))).collect();
        write_param(h, buffer, p, &bytes);
    }

    /// `graphicMagic0708ConvBallChr(list, center, range, rate)` (`slots`:
    /// the caller's argument words): a copy of the mesh list
    /// (`FUN_00635630`, freed after) walked; each material's `Ball_Center`
    /// gets `(-x, -y, z, 1)` of `center` (16 bytes; past them the original
    /// copies its frame, the port 0), `Ball_Range` and `Ball_Rate` the
    /// bytes of the argument words from `range` and `rate` (the original
    /// copies from its argument slots; the port reads the same slots).
    /// The first argument slot ends holding the last material entry.
    pub fn conv_ball_chr(h: &mut impl Host, slots: u32, list: u32, center: u32) {
        if list == 0 {
            return;
        }
        let neg = |x: u32| x87::store(-x87::load(x));
        let v = [
            neg(word(h, center)),
            neg(word(h, center + 4)),
            quiet(word(h, center + 8)),
            1.0_f32.to_bits(),
        ];
        let value: Vec<u8> = v.iter().flat_map(|w| w.to_le_bytes()).collect();
        let count = word(h, list) & 0x7fff_ffff;
        let items = word(h, list + 4);
        let copy = h.call_sym(LIST_COPY, Conv::Cdecl, &[count, items]);
        let n = if copy == 0 { 0 } else { count };
        for i in 0..n {
            let arr = word(h, word(h, copy.wrapping_add(4 * i)).wrapping_add(0x10));
            let mut j = 0;
            while j < word(h, arr) & 0x7fff_ffff {
                let data = h.call_sym(ARRAY_DATA, Conv::Thiscall(arr), &[]);
                let entry = word(h, data.wrapping_add(4 * j));
                h.write(slots, entry);
                let params = word(h, entry);
                let names = [BALL_CENTER, BALL_RANGE, BALL_RATE];
                let mut found = [0_u32; 3];
                for (f, name) in found.iter_mut().zip(names) {
                    *f = h.call_sym(FIND_PARAM, Conv::Thiscall(params), &[h.rebase(name)]);
                }
                let buffer = word(h, entry.wrapping_add(4));
                if found[0] != 0 {
                    write_param(h, buffer, found[0], &value);
                }
                for (k, &p) in found.iter().enumerate().skip(1) {
                    if p != 0 {
                        let buffer = word(h, word(h, slots).wrapping_add(4));
                        copy_param(h, buffer, p, slots + 4 * k as u32 + 4);
                    }
                }
                j += 1;
            }
        }
        h.call_sym(PHYRE_FREE, Conv::Cdecl, &[copy]);
    }

    /// `graphicMagic_DrawColorModel_KaMdldrw(inst, origin, modify, now,
    /// gradient)`: each material of the instance's meshes gets the
    /// parameters `VertexColorHeightNow`, `...HeightGradient`,
    /// `VertexColorOrigin` and `VertexColorModify` copied from the
    /// non-null source pointers.
    pub fn color_model(h: &mut impl Host, inst: u32, src: [u32; 4]) {
        let [origin, modify, now, gradient] = src;
        let mut i = 0;
        while i < word(h, inst) & 0x7fff_ffff {
            let entry = word(h, word(h, inst + 4).wrapping_add(4 * i));
            let n = word(h, word(h, entry.wrapping_add(0x10))) & 0x7fff_ffff;
            for j in 0..n {
                let arr = word(h, entry.wrapping_add(0x10));
                let mat = word(h, array_data(h, arr).wrapping_add(4 * j));
                for (name, from) in [
                    (COLOR_HEIGHT_NOW, now),
                    (COLOR_HEIGHT_GRADIENT, gradient),
                    (COLOR_ORIGIN, origin),
                    (COLOR_MODIFY, modify),
                ] {
                    let params = word(h, mat);
                    let p = h.call_sym(FIND_PARAM, Conv::Thiscall(params), &[h.rebase(name)]);
                    let buffer = word(h, mat.wrapping_add(4));
                    if p != 0 && from != 0 {
                        copy_param(h, buffer, p, from);
                    }
                }
            }
            i += 1;
        }
    }
    /// `graphicSetLoadingScreen_00643500(on, mode)` (`slots`: the
    /// caller's argument words; `frame`: the original's `ebp - 8`): with
    /// `on` and mode 0, unless the loading screen's state (`+0x10`) is
    /// set: the application's `+0x3a0` set, the screen's parameters set
    /// manually (3.0, 1 and 0.0, the last through the first argument
    /// slot) and its `+1` set; modes 1 (state 0..2) and 2 set the
    /// application's `+0x3a0` and the screen's `+1`. Without `on`, mode 0
    /// (state 1..2) and mode 2 clear `+1`. -> what the original leaves in
    /// `eax`.
    pub fn loading_screen(h: &mut impl Host, slots: u32, frame: u32, on: u32, mode: u32) -> u32 {
        let screen = h.rebase(LOADING_SCREEN);
        let ls = word(h, screen);
        if on == 0 {
            match mode {
                0 => {
                    let state = word(h, ls.wrapping_add(0x10)) as i32;
                    if (1..=2).contains(&state) {
                        h.write(ls.wrapping_add(1), 0_u8);
                    }
                    ls
                }
                2 => {
                    h.write(ls.wrapping_add(1), 0_u8);
                    ls
                }
                _ => mode,
            }
        } else {
            match mode {
                0 => {
                    if word(h, ls.wrapping_add(0x10)) != 0 {
                        return ls;
                    }
                    let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
                    h.write(app.wrapping_add(0x3a0), 1_u8);
                    copy(h, h.rebase(LOADING_TIME), frame);
                    h.write(slots, 0_u32);
                    h.write(frame + 4, 1_u32);
                    let ls = word(h, screen);
                    h.call_sym(
                        SET_PARAMETER_MANUALLY,
                        Conv::Thiscall(ls),
                        &[frame, frame + 4, slots],
                    );
                    let ls = word(h, screen);
                    h.write(ls.wrapping_add(1), 1_u8);
                    ls
                }
                1 | 2 => {
                    if mode == 1 {
                        let state = word(h, ls.wrapping_add(0x10));
                        if state as i32 > 2 || (state as i32) < 0 {
                            return state;
                        }
                    }
                    let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
                    h.write(app.wrapping_add(0x3a0), 1_u8);
                    let ls = word(h, screen);
                    h.write(ls.wrapping_add(1), 1_u8);
                    ls
                }
                _ => mode,
            }
        }
    }

    /// `PhyreScene_setColorScale_0066d3b0(scene; r, g, b, ...)` (`ret
    /// 0x1c`; the last four words unread): with `+0x10d4` set, the scale
    /// vector `+0xfc78` multiplied by `(r, g, b)` per component; when the
    /// squared length of `+0x10d30` (each square and partial sum rounded
    /// to `f32`) passes 1e-4, the scaled `(x, y, z)` go to `[+0xcc] +
    /// 0x30`; then each material under the tree `+0x1198` (in order; the
    /// successor taken before the node is used) gets the vector as its
    /// `clrScaleFactor`.
    pub fn set_color_scale(h: &mut impl Host, this: u32, rgb: [u32; 3]) {
        if word(h, this + 0x10d4) == 0 {
            return;
        }
        let v = this + 0xfc78;
        let mut t = [0_u32; 3];
        for (k, x) in t.iter_mut().enumerate() {
            let at = v + 4 * k as u32;
            *x = x87::store(x87::mul(x87::load(rgb[k]), x87::load(word(h, at))));
        }
        let w = quiet(word(h, v + 0xc));
        for (k, &x) in (0u32..).zip(&t) {
            h.write(v + 4 * k, x);
        }
        h.write(v + 0xc, w);
        let d = this + 0x10d30;
        let (x, y, z) = (
            x87::load(word(h, d)),
            x87::load(word(h, d + 4)),
            x87::load(word(h, d + 8)),
        );
        let s = x87::store(x87::mul(x, x));
        let s = x87::store(x87::add(x87::load(s), x87::mul(y, y)));
        let len = x87::load(x87::store(x87::add(x87::load(s), x87::mul(z, z))));
        if len > rdata_f64(h, COLOR_SCALE_MIN) {
            let out = word(h, this + 0xcc);
            let s0 = x87::store(x87::mul(x, x87::load(word(h, v))));
            let s1 = x87::store(x87::mul(x87::load(t[1]), y));
            let s2 = x87::store(x87::mul(x87::load(t[2]), z));
            for (k, s) in (0u32..).zip([s0, s1, s2]) {
                h.write(out.wrapping_add(0x30 + 4 * k), s);
            }
        }
        let tree = this + 0x1198;
        let mut node = word(h, tree + 0xc);
        if node != tree && node != 0 {
            loop {
                let left = word(h, node);
                if left == tree {
                    break;
                }
                node = left;
            }
        }
        while node != tree {
            let next = h.call_sym(TREE_SUCCESSOR, Conv::Thiscall(tree), &[node]);
            let list = word(h, node.wrapping_add(0x10));
            let mut i = 0;
            while i < word(h, list) {
                let e = word(h, word(h, list.wrapping_add(8)).wrapping_add(4 * i));
                let arr = word(h, e.wrapping_add(0x10));
                let mut j = 0;
                while j < word(h, arr) & 0x7fff_ffff {
                    let data = h.call_sym(ARRAY_DATA, Conv::Thiscall(arr), &[]);
                    let mat = word(h, data.wrapping_add(4 * j));
                    let params = word(h, mat);
                    let name = h.rebase(CLR_SCALE_FACTOR);
                    let p = h.call_sym(FIND_PARAM, Conv::Thiscall(params), &[name]);
                    if p != 0 {
                        let buffer = word(h, mat.wrapping_add(4));
                        copy_param(h, buffer, p, v);
                    }
                    j += 1;
                }
                i += 1;
            }
            node = next;
        }
    }
}
