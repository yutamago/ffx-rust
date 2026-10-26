//! The render-side glue, sixteenth batch (M4 port round 5, list
//! `h3-glue-render`): the scene's VFX render arrays, the field map's
//! shutdown, the texture video's unload, the sampler set-up, the
//! post-process parameters from the map's camera, and a four-element
//! layout description.

use ffx_bridge::Global;

/// `Phyre_PWorld_removeCluster(this, cluster)`, `Phyre_PhyreFree(p)`,
/// the texture-animation and morph managers' instances and resets,
/// `PostProcessManager_SetFirstEntry(this, list)` and
/// `Phyre_PFramework_PApplication_GetApplication`.
pub const WORLD_REMOVE_CLUSTER: u32 = 0x0044_3b70;
pub const PHYRE_FREE: u32 = 0x0042_fb20;
pub const TEXTURE_ANIMATION_INSTANCE: u32 = 0x006b_02e0;
pub const TEXTURE_ANIMATION_CLEAR: u32 = 0x006b_2f50;
pub const MORPH_INSTANCE: u32 = 0x006a_bf40;
pub const MORPH_RESET: u32 = 0x006a_c530;
pub const POST_SET_FIRST_ENTRY: u32 = 0x0069_8400;
pub const GET_APPLICATION: u32 = 0x0062_3b90;

/// The texture video's unload: `g_pppSpecialPart`,
/// `Phyre_PhyreOS_Sleep(ms)`, `graphicVFXDestroy(lo, hi)` (cdecl),
/// `PVideoPlaybackWin32_terminate(this)`, the FMV manager and its
/// `onTextureVideoStop`, `PTexture2DD3D11_isBound(this)` / `unbind`,
/// the texture's release `0x004c3010(this)` and
/// `graphicSetTextureVideoInfo(a, b)`.
pub const PPP_SPECIAL_PART: Global<u32> = Global::new(0x00cc_b9cc);
pub const SLEEP: u32 = 0x0043_0030;
pub const VFX_DESTROY: u32 = 0x0064_5280;
pub const VIDEO_TERMINATE: u32 = 0x00a2_8430;
pub const FMV_INSTANCE: u32 = 0x006d_73a0;
pub const FMV_ON_TEXTURE_VIDEO_STOP: u32 = 0x006d_a490;
pub const TEXTURE_IS_BOUND: u32 = 0x0059_5f90;
pub const TEXTURE_UNBIND: u32 = 0x0059_cc40;
pub const TEXTURE_RELEASE: u32 = 0x004c_3010;
pub const SET_TEXTURE_VIDEO_INFO: u32 = 0x0064_3aa0;

/// The samplers' anisotropy (4.0) and `PTextureD3D11_register_0058cf80(this)`.
pub const SAMPLER_ANISOTROPY: Global<f32> = Global::new(0x00b9_22f8);
pub const SAMPLER_REGISTER: u32 = 0x0058_cf80;

/// The post-process parameters: the smallest `+0x84` (0.35), the far
/// default (10000), `getScenerioFlag` and the warning.
pub const POST_MIN_84: Global<f32> = Global::new(0x00b4_80e4);
pub const POST_FAR: Global<f32> = Global::new(0x00b1_0f4c);
pub const GET_SCENARIO_FLAG: u32 = 0x0078_7360;
pub const PHYRE_PRINTF: u32 = crate::glue10::PHYRE_PRINTF;
pub const NO_GLOBAL_CAMERA: u32 = 0x00b4_2618;

/// The layout description: `memset` (the import thunk),
/// `Phyre_PhyreMalloc_0042fb80(size, align)`, and its four elements'
/// name records.
pub const MEMSET: u32 = crate::glue8::MEMSET;
pub const PHYRE_MALLOC_ALIGNED: u32 = 0x0042_fb80;
pub const LAYOUT_NAMES: [u32; 4] = [0x00c9_85b4, 0x00c9_8650, 0x00c9_8638, 0x00c9_862c];

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::x87::round_trip;
    use ffx_graphic::host::{Conv, Host};

    /// The index of `mesh` among the `count` words at `at` (`count` signed).
    fn find(h: &impl Host, at: u32, count: i32, mesh: u32) -> Option<u32> {
        (0..count.max(0) as u32).find(|&k| h.read::<u32>(at.wrapping_add(4 * k)) == mesh)
    }

    /// `PhyreScene_removeVFXFromRenderArray(this, mesh)` (`ret 4`): the
    /// first place `mesh` is found is cleared: the scene's VFX meshes
    /// (`+0x38cc`, count `+0xf8cc`), the VFX instances' meshes
    /// (`+0x68cc`, count `+0xf8d0`, the mesh at `+0xc`), then the three
    /// arrays `+0x3880` (count `+0x3878`), `+0x385c` (`+0x3854`),
    /// `+0x3868` (`+0x3860`) and the masked array `+0x3888` (`+0x3884`).
    /// `eax`: `mesh`, or the cleared array's pointer for the last four.
    pub fn scene_remove_vfx_from_render_array(h: &mut impl Host, this: u32, mesh: u32) -> u32 {
        if let Some(k) = find(h, this + 0x38cc, h.read(this + 0xf8cc), mesh) {
            h.write(this + 0x38cc + 4 * k, 0u32);
            return mesh;
        }
        let n: i32 = h.read(this + 0xf8d0);
        for k in 0..n.max(0) as u32 {
            let instance: u32 = h.read(this + 0x68cc + 4 * k);
            if instance != 0 && h.read::<u32>(instance + 0xc) == mesh {
                h.write(this + 0x68cc + 4 * k, 0u32);
                return mesh;
            }
        }
        // (an index of -1 would fall through to the next array; counts
        // never reach it)
        for (count, data) in [(0x3878, 0x3880), (0x3854, 0x385c), (0x3860, 0x3868)] {
            let at: u32 = h.read(this + data);
            if let Some(k) = find(h, at, h.read(this + count), mesh) {
                let at: u32 = h.read(this + data);
                h.write(at.wrapping_add(4 * k), 0u32);
                return at;
            }
        }
        let n = h.read::<u32>(this + 0x3884) & 0x7fff_ffff;
        let at: u32 = h.read(this + 0x3888);
        if let Some(k) = (0..n).find(|&k| h.read::<u32>(at.wrapping_add(4 * k)) == mesh) {
            let at: u32 = h.read(this + 0x3888);
            h.write(at.wrapping_add(4 * k), 0u32);
            return at;
        }
        mesh
    }

    /// `PhyreScene_fieldMapShutdown(this)`: the map's two clusters
    /// (`+0x10d4`, `+0x1128`) leave their worlds (`+0xac`, `+0x8c`), the
    /// fog record's owned array (`+0xcc`: `+0x9c/+0xa0`) is freed and its
    /// colour zeroed, five lists emptied (`+0xdc`, `+0x238`, `+0x394`,
    /// `+0x7e0`, `+0x654` point past themselves), the texture animations
    /// and morphs reset, the colour scales `+0xfc78..` reset (1, 1, 1, 1;
    /// `+0xfc88` = 0; 1 x4, 0 x4, 1 x4, 0 x4), the post-process list
    /// cleared and the application's `+0x3b0` zeroed. Returns 0.
    pub fn scene_field_map_shutdown(h: &mut impl Host, this: u32) -> u32 {
        for (cluster, world) in [(0x10d4, 0xac), (0x1128, 0x8c)] {
            let c: u32 = h.read(this + cluster);
            if c != 0 {
                h.call_sym(WORLD_REMOVE_CLUSTER, Conv::Thiscall(this + world), &[c]);
            }
        }
        let fog: u32 = h.read(this + 0xcc);
        let word: u32 = h.read(fog + 0x9c);
        if word != 0 {
            let data: u32 = h.read(fog + 0xa0);
            if data != 0 && word & 0x8000_0000 == 0 {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
            }
            h.write(fog + 0xa0, 0u32);
            h.write(fog + 0x9c, 0u32);
        }
        for off in [0xdc, 0x238, 0x394, 0x7e0, 0x654] {
            h.write(this + off, this + off + 4);
        }
        let fog: u32 = h.read(this + 0xcc);
        for k in 0..3 {
            h.write(fog + 4 * k, 0f32);
        }
        let animations = h.call_sym(TEXTURE_ANIMATION_INSTANCE, Conv::Cdecl, &[]);
        h.call_sym(TEXTURE_ANIMATION_CLEAR, Conv::Thiscall(animations), &[]);
        let morphs = h.call_sym(MORPH_INSTANCE, Conv::Cdecl, &[]);
        h.call_sym(MORPH_RESET, Conv::Thiscall(morphs), &[]);
        for k in 0..4 {
            h.write(this + 0xfc78 + 4 * k, 1f32);
        }
        let post: u32 = h.read(this + 0x10cc);
        h.call_sym(POST_SET_FIRST_ENTRY, Conv::Thiscall(post), &[0]);
        h.write(this + 0xfc88, 0u8);
        for (start, v) in [(0xfc8c, 1f32), (0xfc9c, 0.0), (0xfcac, 1.0), (0xfcbc, 0.0)] {
            for k in 0..4 {
                h.write(this + start + 4 * k, v);
            }
        }
        let application = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        h.write(application + 0x3b0, 0f32);
        0
    }

    /// Unbinds and releases the texture at `+off`, then clears it.
    fn release_texture(h: &mut impl Host, this: u32, off: u32) {
        let texture: u32 = h.read(this + off);
        if h.call_sym(TEXTURE_IS_BOUND, Conv::Thiscall(texture), &[]) & 0xff != 0 {
            h.call_sym(TEXTURE_UNBIND, Conv::Thiscall(texture), &[]);
        }
        let texture: u32 = h.read(this + off);
        if texture != 0 {
            h.call_sym(TEXTURE_RELEASE, Conv::Thiscall(texture), &[]);
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[texture]);
        }
        h.write(this + off, 0u32);
    }

    /// `PhyreScene_textureVideoUnload(this)`: `g_pppSpecialPart` = 0;
    /// with a texture video (`+0xdd0`): waits up to 60 ms for its
    /// decoder (`+0xdcc`), destroys its `+0x10c0` effects (instances
    /// `0x5bc:0x78 + k`, their records `+0xd68` marked), stops the
    /// playback (`+0xde8`) and the FMV delegation, releases the three
    /// textures (`+0xdd8`, `+0xddc`, `+0xde0`), clears its state and
    /// resets the video info. `eax`: that reset's; 0 without a video (the
    /// original leaves its caller's `eax`, which no shim can return).
    pub fn scene_texture_video_unload(h: &mut impl Host, this: u32) -> u32 {
        PPP_SPECIAL_PART.set(h, 0);
        if h.read::<u8>(this + 0xdd0) == 0 {
            return 0;
        }
        let mut waited = 0;
        while h.read::<u32>(this + 0xdcc) != 0 && waited < 0x3c {
            h.call_sym(SLEEP, Conv::Cdecl, &[1]);
            waited += 1;
        }
        h.write(this + 0xdc8, 0u32);
        let mut k = 0i32;
        while k < h.read::<i32>(this + 0x10c0) {
            let lo = (k as u32).wrapping_add(0x78);
            let hi = 0x5bcu32.wrapping_add(u32::from((lo as i32) < 0).wrapping_neg());
            h.call_sym(VFX_DESTROY, Conv::Cdecl, &[lo, hi]);
            h.write(this + 0xd68 + 0x18 * k as u32, 1u32);
            k += 1;
        }
        h.call_sym(VIDEO_TERMINATE, Conv::Thiscall(this + 0xde8), &[]);
        let fmv = h.call_sym(FMV_INSTANCE, Conv::Cdecl, &[]);
        h.call_sym(FMV_ON_TEXTURE_VIDEO_STOP, Conv::Thiscall(fmv), &[]);
        for off in [0xdd8, 0xddc, 0xde0] {
            release_texture(h, this, off);
        }
        h.write(this + 0x10c0, 0u32);
        h.write(this + 0xdd0, 0u8);
        h.write(this + 0x10c9, 0u8);
        h.call_sym(SET_TEXTURE_VIDEO_INFO, Conv::Cdecl, &[u32::MAX, 0])
    }

    /// `PhyreScene_setupSamplers_0065f290(this)`: the eight samplers at
    /// `+0x964` (0x24 bytes each) not locked (bit 7 of `+0x1c`) get the
    /// filter 5 (`+0`), `+1` = 1, their two address modes (`+2`, `+3`:
    /// 1/1, 2/2, 1/2, 2/1, twice over), `+4` = 1 and the anisotropy 4.0
    /// (`+0xc`); each is registered. Returns 0.
    pub fn scene_setup_samplers<H: Host>(h: &mut H, this: u32) -> u32 {
        const MODES: [(u8, u8); 4] = [(1, 1), (2, 2), (1, 2), (2, 1)];
        for k in 0..8u32 {
            let sampler = this + 0x964 + 0x24 * k;
            let free = |h: &H| h.read::<u32>(sampler + 0x1c) & 0x80 == 0;
            if free(h) {
                h.write(sampler, 5u8);
                h.write(sampler + 1, 1u8);
            }
            if free(h) {
                let (u, v) = MODES[k as usize % 4];
                h.write(sampler + 2, u);
                h.write(sampler + 3, v);
            }
            if free(h) {
                h.write(sampler + 4, 1u8);
                let anisotropy = round_trip(SAMPLER_ANISOTROPY.get(h));
                h.write(sampler + 0xc, anisotropy);
            }
            h.call_sym(SAMPLER_REGISTER, Conv::Thiscall(sampler), &[]);
        }
        0
    }

    /// `PhyreScene_setupPostProcessParams(this)`: with the map's camera
    /// (`+0x4f8`), its `+0x104..+0x110` to the fog record's (`+0xcc`)
    /// `+0x80..+0x8c` (`+0x84` at least 0.35), overridden by 1, 1, 0, 0 in
    /// level 0x199 and, before the scenario flag, levels 0x12ea and
    /// 0x1733 (`+0xfaf4`), then `+0x114..+0x11c` to `+0x90..+0x98`;
    /// without one the defaults and a warning. `eax`: the camera, or the
    /// warning's.
    pub fn scene_setup_post_process_params<H: Host>(h: &mut H, this: u32) -> u32 {
        let fog = |h: &H| h.read::<u32>(this + 0xcc);
        let camera: u32 = h.read(this + 0x4f8);
        if camera == 0 {
            for (off, v) in [(0x80, 1f32), (0x84, 1.0), (0x88, 0.0), (0x8c, 0.0)] {
                h.write(fog(h) + off, v);
            }
            let far = round_trip(POST_FAR.get(h));
            for off in [0x90, 0x94, 0x98] {
                h.write(fog(h) + off, far);
            }
            let message = h.rebase(NO_GLOBAL_CAMERA);
            return h.call_sym(PHYRE_PRINTF, Conv::Cdecl, &[2, message]);
        }
        let copy = |h: &mut H, from: u32, to: u32| {
            let camera: u32 = h.read(this + 0x4f8);
            let v = round_trip(h.read(camera + from));
            let dst = h.read::<u32>(this + 0xcc);
            h.write(dst + to, v);
        };
        for k in 0..4 {
            copy(h, 0x104 + 4 * k, 0x80 + 4 * k);
        }
        let least = POST_MIN_84.get(h);
        let v: f32 = h.read(fog(h) + 0x84);
        if v < least {
            h.write(fog(h) + 0x84, round_trip(least));
        }
        let level: u32 = h.read(this + 0xfaf4);
        let reset = level == 0x199
            || ((level == 0x12ea || level == 0x1733)
                && h.call_sym(GET_SCENARIO_FLAG, Conv::Cdecl, &[])
                    .to_le_bytes()[..2]
                    == [0, 0]);
        if reset {
            for (off, v) in [(0x80, 1f32), (0x84, 1.0), (0x88, 0.0), (0x8c, 0.0)] {
                h.write(fog(h) + off, v);
            }
        }
        for k in 0..3 {
            copy(h, 0x114 + 4 * k, 0x90 + 4 * k);
        }
        h.read(this + 0x4f8)
    }

    /// Replaces an owned array's storage (`count` / `data`) by `fresh`
    /// (freeing an owned old one) and sets its count to `n`.
    fn adopt(h: &mut impl Host, count: u32, data: u32, fresh: u32, n: u32) {
        let old: u32 = h.read(data);
        let word: u32 = h.read(count);
        if old != fresh && word & 0x8000_0000 == 0 && old != 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
        }
        h.write(data, fresh);
        h.write(count, n);
    }

    /// `LayoutDesc_setup_00678bd0(this, a, b)` (`ret 8`): `+0x7c..+0xfc`
    /// cleared; one 16-byte stream (`+0x10/+0x14`: `{0, 4, a, b}`) and
    /// four 12-byte elements (`+8/+0xc`: offset 0, a name record from
    /// `LAYOUT_NAMES`, kinds 2, 3, 1, 2), the arrays made when their
    /// counts differ (an allocation that fails keeps the old one).
    /// `eax`: the elements.
    pub fn layout_setup<H: Host>(h: &mut H, this: u32, a: u32, b: u32) -> u32 {
        h.call_sym(MEMSET, Conv::Cdecl, &[this + 0x7c, 0, 0x80]);
        if h.read::<u32>(this + 0x10) != 1 {
            let p = h.call_sym(PHYRE_MALLOC_ALIGNED, Conv::Cdecl, &[0x10, 4]);
            if p != 0 {
                for off in [0, 4, 8, 0xc] {
                    h.write(p + off, 0u32);
                }
                adopt(h, this + 0x10, this + 0x14, p, 1);
            }
        }
        if h.read::<u32>(this + 8) != 4 {
            let p = h.call_sym(PHYRE_MALLOC_ALIGNED, Conv::Cdecl, &[0x30, 4]);
            if p != 0 {
                for k in 0..4u32 {
                    let e = p.wrapping_add(12 * k);
                    if k == 0 || e != 0 {
                        h.write(e, 0u32);
                        h.write(e.wrapping_add(4), 0u32);
                        h.write(e.wrapping_add(8), 0xff34u16);
                    }
                }
                adopt(h, this + 8, this + 0xc, p, 4);
            }
        }
        let stream = |h: &H| h.read::<u32>(this + 0x14);
        for (off, v) in [(0, 0u32), (4, 4), (8, a), (0xc, b)] {
            h.write(stream(h) + off, v);
        }
        for (k, (name, kind)) in (0u32..).zip(LAYOUT_NAMES.into_iter().zip([2u8, 3, 1, 2])) {
            let elements: u32 = h.read(this + 0xc);
            let e = elements.wrapping_add(12 * k);
            let name = h.rebase(name);
            h.write(e.wrapping_add(8), kind);
            h.write(e.wrapping_add(4), name);
            h.write(e, 0u32);
        }
        h.read(this + 0xc)
    }
}
