//! The render-side glue at 0x00690000 and up (M4, list `h3-glue-render`,
//! agent h3b): `PostProcessManager_onAllShaderBinFilesLoaded`, the
//! post-process manager's set-up once its shaders are in: render targets,
//! full-screen meshes with one material each, the glow and blur effects,
//! the HDAO preset tables read from two files, sampler states.

/// `PostProcessManager_lookup(this; index)` (`ret 4`): a shader variant, or 0.
pub const POST_LOOKUP: u32 = 0x0069_35f0;
/// `PApplication_GetApplication()` (cdecl); its `+0x284` is the MSAA mode.
pub const GET_APPLICATION: u32 = 0x0062_3b90;
/// A sampler state's register (`ecx` the state).
pub const REGISTER: u32 = 0x0058_cf80;
/// `PCluster_create(cluster; 1, class, 0)` (`ret 0xc`) and its classes:
/// a texture and a material array.
pub const CLUSTER_CREATE: u32 = 0x0044_1010;
pub const TEXTURE_CLASS: u32 = 0x00ca_37d0;
pub const ARRAY_CLASS: u32 = 0x00c9_98e8;
/// `PTexture2DBase_setDimensions(texture; w, h, format, 0)` (`ret 0x10`)
/// and `updateMaxLevel(texture)`.
pub const SET_DIMENSIONS: u32 = 0x004c_40c0;
pub const UPDATE_MAX_LEVEL: u32 = 0x004c_4250;
/// The texture formats.
pub const FORMAT_COLOR: u32 = 0x00ca_310c;
pub const FORMAT_NORMAL: u32 = 0x00ca_347c;
pub const FORMAT_DEPTH: u32 = 0x00ca_3314;
/// `0x0064adf0(cluster; texture)` (`ret 4`): the render target.
pub const MAKE_TARGET: u32 = 0x0064_adf0;
/// The full-screen meshes (cdecl, the cluster): a coloured triangle, a
/// quad, and the HDAO one.
pub const MESH_TRIANGLE: u32 = 0x00a0_6120;
pub const MESH_QUAD: u32 = 0x00a0_6470;
pub const MESH_HDAO: u32 = 0x00a0_6750;
/// `PEffectVariant_createMaterial(variant; cluster, 0, 0)` (`ret 0xc`).
pub const CREATE_MATERIAL: u32 = 0x004e_6a40;
/// `PhyreFree` (cdecl) and the array's `setWord(array; index, value)`
/// (`ret 8`).
pub const FREE: u32 = 0x0042_fb20;
pub const SET_WORD: u32 = 0x0048_c540;
/// The legacy glow (`+0x140`) and the two blurs (`+0x1100`, `+0x1190`):
/// `initialize(effect; cluster, w, h)` (`ret 0xc`) and
/// `initializeShaders(effect; cluster, variant)` (`ret 8`).
pub const GLOW_INITIALIZE: u32 = 0x00a1_76d0;
pub const GLOW_SHADERS: u32 = 0x00a1_a6c0;
pub const BLUR_INITIALIZE: u32 = 0x00a1_7bd0;
pub const BLUR_SHADERS: u32 = 0x00a1_ab00;
pub const BLUR2_INITIALIZE: u32 = 0x00a1_7980;
pub const BLUR2_SHADERS: u32 = 0x00a1_a810;
/// The screen-fade mesh (`ecx` the manager).
pub const SCREEN_FADE_MESH: u32 = 0x0069_31a0;
/// `0x00698a30(this; index)` (`ret 4`).
pub const SETTING_TABLE: u32 = 0x0069_8a30;
/// Sampler-state set-up (`ret 4` each): filter, then three address modes.
pub const SAMPLER_FILTER: u32 = 0x004c_07f0;
pub const SAMPLER_ADDRESS_U: u32 = 0x004c_0870;
pub const SAMPLER_ADDRESS_V: u32 = 0x004c_08a0;
pub const SAMPLER_ADDRESS_W: u32 = 0x004c_0840;
/// The file reader: ctor `(reader; path)` (`ret 4`), `isOpen` (`al`),
/// `close`, dtor; its stream (`reader + 0x14`): `getFileSize`,
/// `read(stream; buffer, size)` (`ret 8`).
pub const READER_CTOR: u32 = 0x0057_66d0;
pub const READER_IS_OPEN: u32 = 0x0057_6980;
pub const READER_CLOSE: u32 = 0x0057_6930;
pub const READER_DTOR: u32 = 0x0057_67c0;
pub const STREAM_SIZE: u32 = 0x0060_7dc0;
pub const STREAM_READ: u32 = 0x0060_8090;
/// `operator new` / `operator delete` (cdecl), `memcpy`, `DJB_hash`.
pub const NEW: u32 = 0x0063_04c0;
pub const DELETE: u32 = 0x0063_0540;
pub const MEMCPY: u32 = 0x0094_9256;
pub const DJB_HASH: u32 = 0x0067_b7d0;
/// The presets' map insert `(map; &hash, &record)` (`ret 8`).
pub const MAP_INSERT: u32 = 0x0069_3c50;
/// The two preset files' path pointers.
pub const PRESET_PATHS: u32 = 0x00c3_414c;
/// f32 stored at `+0x12328`.
pub const C_B1F2C8: u32 = 0x00b1_f2c8;
/// The target names.
pub const NAMES: [u32; 10] = [
    0x00b4_75d8, // colorBuffer
    0x00b4_75f8, // normalBuffer
    0x00b4_7618, // depthBuffer
    0x00b4_7638, // backupDepthBuffer
    0x00b4_7660, // previousFrameTarget
    0x00b4_7688, // backupBuffer
    0x00b4_76a8, // d3d11FrameBuffer
    0x00b4_76cc, // colorCorrection
    0x00b4_76f0, // glowMap
    0x00b4_770c, // the HDAO target
];

/// The frame (`ebp` offsets): the file reader (its stream at `- 0x30`),
/// the map key and value.
pub const READER: u32 = 0x44;
pub const STREAM: u32 = 0x30;
pub const VALUE: u32 = 0x20;
pub const KEY: u32 = 0x10;

pub mod bridge {
    //! The original over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// An error to return.
    type Step = Result<(), u32>;

    fn lookup(h: &mut impl Host, this: u32, index: u32) -> u32 {
        h.call_sym(POST_LOOKUP, Conv::Thiscall(this), &[index])
    }

    fn copy_f32(h: &mut impl Host, to: u32, from: u32) {
        let v: u32 = h.read(from);
        h.write(to, x87::store(x87::load(v)));
    }

    /// A render target at `this + field`: a texture of the manager's size
    /// (`+0x12400`, `+0x12404`) in `format`, made a target and named.
    /// 0xd when the texture (with `checked`) or the target is missing,
    /// `setDimensions`' error.
    fn target(
        h: &mut impl Host,
        this: u32,
        field: u32,
        format: u32,
        name: u32,
        checked: bool,
    ) -> Result<u32, u32> {
        let cluster = this + 0x3c;
        let class = h.rebase(TEXTURE_CLASS);
        let tex = h.call_sym(CLUSTER_CREATE, Conv::Thiscall(cluster), &[1, class, 0]);
        if checked && tex == 0 {
            return Err(0xd);
        }
        let w: u32 = h.read(this + 0x12400);
        let ht: u32 = h.read(this + 0x12404);
        let format = h.rebase(format);
        let error = h.call_sym(SET_DIMENSIONS, Conv::Thiscall(tex), &[w, ht, format, 0]);
        if error != 0 {
            return Err(error);
        }
        let flags: u32 = h.read(tex + 0x14);
        h.write(tex + 0x14, flags & !2);
        h.call_sym(UPDATE_MAX_LEVEL, Conv::Thiscall(tex), &[]);
        let t = h.call_sym(MAKE_TARGET, Conv::Thiscall(cluster), &[tex]);
        h.write(this + field, t);
        if t == 0 {
            return Err(0xd);
        }
        h.write(t + 0xc, h.rebase(name));
        Ok(t)
    }

    /// A target that also takes the MSAA mode (`+5`) and flag 1 (`+8`).
    fn msaa_target(
        h: &mut impl Host,
        this: u32,
        field: u32,
        format: u32,
        name: u32,
        mode: u32,
    ) -> Step {
        let t = target(h, this, field, format, name, true)?;
        h.write(t + 5, mode as u8);
        let flags: u32 = h.read(t + 8);
        h.write(t + 8, flags | 1);
        Ok(())
    }

    /// A one-element material array: emptied to its inline slot (old
    /// storage freed unless borrowed, top bit), then `material` set.
    fn material_array(h: &mut impl Host, cluster: u32, material: u32) -> u32 {
        let class = h.rebase(ARRAY_CLASS);
        let array = h.call_sym(CLUSTER_CREATE, Conv::Thiscall(cluster), &[1, class, 0]);
        let head: u32 = h.read(array);
        if head != 1 {
            let data = match head & 0x7fff_ffff {
                0 => 0,
                1 => array + 4,
                _ => h.read(array + 4),
            };
            let slot = array.wrapping_add(4);
            if slot != 0 {
                h.write(slot, 0u32);
                let head: u32 = h.read(array);
                if data != slot && head >> 31 == 0 {
                    h.call_sym(FREE, Conv::Cdecl, &[data]);
                }
                h.write(array, 1u32);
            }
        }
        h.call_sym(SET_WORD, Conv::Thiscall(array), &[0, material]);
        array
    }

    /// A full-screen mesh (`make`) at `this + mesh_at` with one material of
    /// `variant` (kept at `this + material_at` when given).
    fn mesh(
        h: &mut impl Host,
        this: u32,
        make: u32,
        mesh_at: u32,
        material_at: Option<u32>,
        variant: u32,
    ) {
        let cluster = this + 0x3c;
        let m = h.call_sym(make, Conv::Cdecl, &[cluster]);
        h.write(this + mesh_at, m);
        let material = h.call_sym(CREATE_MATERIAL, Conv::Thiscall(variant), &[cluster, 0, 0]);
        if let Some(at) = material_at {
            h.write(this + at, material);
        }
        let array = material_array(h, cluster, material);
        let m: u32 = h.read(this + mesh_at);
        h.write(m + 0x10, array);
    }

    /// A sampler state at `s`: `filter`, the address modes 2, and its
    /// `+1` byte set to `enable` unless flag `0x80` of `+0x1c` is set.
    fn sampler(h: &mut impl Host, s: u32, filter: u32, enable: u8) {
        let c = Conv::Thiscall(s);
        h.call_sym(SAMPLER_FILTER, c, &[filter]);
        if h.read::<u32>(s + 0x1c) >> 7 & 1 == 0 {
            h.write(s + 1, enable);
        }
        for f in [SAMPLER_ADDRESS_U, SAMPLER_ADDRESS_V, SAMPLER_ADDRESS_W] {
            h.call_sym(f, c, &[2]);
        }
        h.call_sym(REGISTER, c, &[]);
    }

    /// `PostProcessManager_onAllShaderBinFilesLoaded` (`0x00693d70`,
    /// `ecx` the manager, no stack arguments; frame at `ebp`, see the
    /// module's offsets). 0 when done, 0x13 for a missing variant or an
    /// MSAA mode other than 0, 2, 4, 8, 16, 0xd for a missing texture or
    /// target, else a callee's error.
    pub fn on_all_shader_bin_files_loaded(h: &mut impl Host, ebp: u32, this: u32) -> u32 {
        match set_up(h, ebp, this) {
            Ok(()) => 0,
            Err(e) => e,
        }
    }

    #[allow(clippy::too_many_lines)] // the original's sequence
    fn set_up(h: &mut impl Host, ebp: u32, this: u32) -> Step {
        // the depth sampler state at +0x123dc: both bytes unless flag 0x80
        let s = this + 0x123dc;
        if h.read::<u32>(this + 0x123f8) >> 7 & 1 == 0 {
            h.write(s, 1u8);
            h.write(s + 1, 1u8);
        }
        h.call_sym(REGISTER, Conv::Thiscall(s), &[]);
        let v4 = lookup(h, this, 4);
        if v4 == 0 {
            return Err(0x13);
        }
        let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        let mode: u32 = h.read(app + 0x284);
        if mode != 0 {
            if ![2, 4, 8, 0x10].contains(&mode) {
                return Err(0x13);
            }
            h.write(this + 0x122b4, 1u8);
        }
        msaa_target(h, this, 0x8c, FORMAT_COLOR, NAMES[0], mode)?;
        msaa_target(h, this, 0x94, FORMAT_COLOR, NAMES[1], mode)?;
        msaa_target(h, this, 0x90, FORMAT_NORMAL, NAMES[2], mode)?;
        for (field, format, name) in [
            (0x98, FORMAT_DEPTH, NAMES[3]),
            (0x1232c, FORMAT_COLOR, NAMES[4]),
            (0xa8, FORMAT_COLOR, NAMES[5]),
            (0xac, FORMAT_COLOR, NAMES[6]),
        ] {
            target(h, this, field, format, name, true)?;
        }
        mesh(h, this, MESH_TRIANGLE, 0x1208, None, v4);
        h.write(this + 0x123b4, 1u8);

        let v0 = lookup(h, this, 0);
        if v0 != 0 {
            target(h, this, 0x9c, FORMAT_COLOR, NAMES[7], true)?;
            mesh(h, this, MESH_TRIANGLE, 0x1204, None, v0);
            if h.read::<u32>(this + 0x9c) != 0 && h.read::<u32>(this + 0x1204) != 0 {
                h.write(this + 0x123b5, 1u8);
            }
        }

        let cluster = this + 0x3c;
        let v2 = lookup(h, this, 2);
        if v2 == 0 {
            return Err(0x13);
        }
        let glow = Conv::Thiscall(this + 0x140);
        let (w, ht): (u32, u32) = (h.read(this + 0x12400), h.read(this + 0x12404));
        h.call_sym(GLOW_INITIALIZE, glow, &[cluster, w, ht]);
        h.call_sym(GLOW_SHADERS, glow, &[cluster, v2]);
        let settings: u32 = h.read(this);
        if settings != 0 {
            for k in 0..3 {
                copy_f32(h, this + 0x154 + 4 * k, settings + 0x90 + 4 * k);
            }
        }
        h.write(this + 0x123b7, 1u8);
        let v3 = lookup(h, this, 3);
        if v3 == 0 {
            return Err(0x13);
        }
        // the glow map's texture is not tested for 0
        target(h, this, 0xa4, FORMAT_COLOR, NAMES[8], false)?;
        let blur = Conv::Thiscall(this + 0x1100);
        let (w, ht): (u32, u32) = (h.read(this + 0x12400), h.read(this + 0x12404));
        h.call_sym(
            BLUR_INITIALIZE,
            blur,
            &[cluster, w.wrapping_mul(2), ht.wrapping_mul(2)],
        );
        h.call_sym(BLUR_SHADERS, blur, &[cluster, v3]);
        let settings: u32 = h.read(this);
        if settings != 0 {
            for k in 0..2 {
                copy_f32(h, this + 0x1154 + 4 * k, settings + 0x88 + 4 * k);
            }
        }
        let v6 = lookup(h, this, 6);
        if v6 == 0 {
            return Err(0x13);
        }
        let blur2 = Conv::Thiscall(this + 0x1190);
        let (w, ht): (u32, u32) = (h.read(this + 0x12400), h.read(this + 0x12404));
        let error = h.call_sym(BLUR2_INITIALIZE, blur2, &[cluster, w, ht]);
        if error != 0 {
            return Err(error);
        }
        h.call_sym(BLUR2_SHADERS, blur2, &[cluster, v6]);
        h.write(this + 0x123b9, 1u8);
        // the glow's variant, not the blur's
        mesh(h, this, MESH_TRIANGLE, 0x1210, None, v3);
        h.write(this + 0x123b8, 1u8);
        let v5 = lookup(h, this, 5);
        if v5 == 0 {
            return Err(0x13);
        }
        mesh(h, this, MESH_TRIANGLE, 0x12384, Some(0x12388), v5);
        h.call_sym(SCREEN_FADE_MESH, Conv::Thiscall(this), &[]);
        let v7 = lookup(h, this, 7);
        if v7 == 0 {
            return Err(0x13);
        }
        mesh(h, this, MESH_QUAD, 0x1214, Some(0x1218), v7);

        let v8 = lookup(h, this, 8);
        if v8 != 0 {
            mesh(h, this, MESH_QUAD, 0x121c, Some(0x1220), v8);
            for k in 0..2 {
                let path: u32 = h.read(h.rebase(PRESET_PATHS) + 4 * k);
                read_presets(h, ebp, this, path);
            }
            h.write(this + 0x123d0, 0x101u16);
        }
        let v9 = lookup(h, this, 9);
        if v9 != 0 {
            mesh(h, this, MESH_QUAD, 0x1229c, Some(0x122a0), v9);
            h.write(this + 0x123d2, 0x101u16);
        }
        for (index, at) in [(0xb, 0x122c4), (0xc, 0x122c8)] {
            let v = h.call_sym(SETTING_TABLE, Conv::Thiscall(this), &[index]);
            h.write(this + at, v);
        }
        let v10 = lookup(h, this, 10);
        if v10 != 0 {
            mesh(h, this, MESH_HDAO, 0x122b8, Some(0x122bc), v10);
            sampler(h, this + 0x122cc, 1, 1);
            sampler(h, this + 0x122f0, 0, 0);
            target(h, this, 0x122c0, FORMAT_NORMAL, NAMES[9], true)?;
            h.write(this + 0x123d4, 0x101u16);
        }
        let v13 = lookup(h, this, 0xd);
        if v13 != 0 {
            mesh(h, this, MESH_QUAD, 0x12320, Some(0x12324), v13);
            let c = h.rebase(C_B1F2C8);
            copy_f32(h, this + 0x12328, c);
            h.write(this + 0x123d6, 0x101u16);
        }
        Ok(())
    }

    /// One preset file: records of a length-prefixed name (signed byte)
    /// and 13 words, appended to the table at `+0x126c` (0x44 bytes each,
    /// count `+0x1268`) and put into the map `+0x1226c` by the name's
    /// hash. The reader lives in the frame; the key and value too.
    fn read_presets(h: &mut impl Host, ebp: u32, this: u32, path: u32) {
        let reader = ebp - READER;
        let stream = ebp - STREAM;
        h.call_sym(READER_CTOR, Conv::Thiscall(reader), &[path]);
        let open = h.call_sym(READER_IS_OPEN, Conv::Thiscall(reader), &[]) as u8;
        if open != 0 {
            let size = h.call_sym(STREAM_SIZE, Conv::Thiscall(stream), &[]);
            let buf = h.call_sym(NEW, Conv::Cdecl, &[size]);
            h.call_sym(STREAM_READ, Conv::Thiscall(stream), &[buf, size]);
            h.call_sym(READER_CLOSE, Conv::Thiscall(reader), &[]);
            let count: i32 = h.read(buf);
            let mut p = buf.wrapping_add(4);
            for _ in 0..count.max(0) {
                let index: u32 = h.read(this + 0x1268);
                let record = this
                    .wrapping_add(0x126c)
                    .wrapping_add(index.wrapping_mul(0x44));
                let len = i32::from(h.read::<i8>(p)) as u32;
                p = p.wrapping_add(1);
                let name = record.wrapping_add(0x34);
                h.call_sym(MEMCPY, Conv::Cdecl, &[name, p, len]);
                p = p.wrapping_add(len);
                h.write(name.wrapping_add(len), 0u8);
                for k in 0..13 {
                    let w: u32 = h.read(p + 4 * k);
                    h.write(record + 4 * k, w);
                }
                p = p.wrapping_add(0x34);
                let hash = h.call_sym(DJB_HASH, Conv::Cdecl, &[name]);
                h.write(ebp - KEY, hash);
                h.write(ebp - VALUE, record);
                h.call_sym(
                    MAP_INSERT,
                    Conv::Thiscall(this + 0x1226c),
                    &[ebp - KEY, ebp - VALUE],
                );
                let n: u32 = h.read(this + 0x1268);
                h.write(this + 0x1268, n.wrapping_add(1));
            }
            h.call_sym(DELETE, Conv::Cdecl, &[buf]);
        }
        h.call_sym(READER_DTOR, Conv::Thiscall(reader), &[]);
    }
}
