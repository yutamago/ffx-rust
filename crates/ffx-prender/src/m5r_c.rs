//! M5 stage 1, list `render`, batch 3: the `graphic*` character and
//! effect entry points the event and battle scripts call (shader
//! parameters, clip planes, HSV, lighting, summon textures, screen
//! fades, the free camera's mode), the scene's character-cluster world
//! switch and release, and the unloads that build file names in their
//! frames.
//!
//! Frames: where the original hands callees the address of a buffer in
//! its frame, the bridge takes the original's `ebp` and keeps the buffer
//! at the same offsets (the shim passes a [`StackFrame`]-backed `ebp`).
//! Stack cookies are left out.
//!
//! Floats: `f32` arguments the original passes on through `fld`/`fstp`
//! come out of the x87 quiet (a signalling NaN gets its quiet bit), as
//! [`quiet`] does.

use ffx_bridge::Global;

/// `g_PhyreScene`.
pub const SCENE: Global<u32> = Global::new(0x00cc_b9d8);
/// `g_FreeCamera`.
pub const FREE_CAMERA: Global<u32> = Global::new(0x00cc_c830);
/// `g_TexAnimSkip`: a count (at most 5) and the character instances after
/// it.
pub const TEX_ANIM_SKIP: Global<u32> = Global::new(0x00cc_b230);
/// The default menu texture list (`std::list<std::string>`, its head
/// node pointer).
pub const MENU_DEFAULT_TEXTURES: u32 = 0x00cc_c840;

// ---- imports ----
pub const IOB_FUNC: u32 = 0x00b0_c350;
pub const FPRINTF: u32 = 0x00b0_c38c;

// ---- callees ----
pub const INPUT_MANAGER: u32 = 0x0063_1300;
pub const INPUT_VIBRATION_FLAG: u32 = 0x0063_1320;
pub const LOCALIZATION_MANAGER: u32 = 0x006d_b070;
/// The skip texture paths by language (0..10), and the warning.
pub const SKIP_TEXTURES: [u32; 11] = [
    0x00b4_1388,
    0x00b4_13c0,
    0x00b4_13f8,
    0x00b4_1430,
    0x00b4_1468,
    0x00b4_14a0,
    0,
    0,
    0,
    0x00b4_14d8,
    0x00b4_1510,
];
pub const SKIP_TEXTURE_WARNING: u32 = 0x00b4_1c74;
pub const BLUR_EFFECT: u32 = 0x006c_3580;
pub const BLUR_ACTIVATE: u32 = 0x006c_35a0;
pub const REGISTER_SCREEN_FADE: u32 = 0x006a_e650;
pub const WORLD_REMOVE_CLUSTER: u32 = 0x0044_3b70;
pub const WORLD_ADD_CLUSTER: u32 = 0x0044_3250;
pub const CHARACTER_LIGHTING: u32 = 0x006a_6e30;
pub const LIGHTING_SET_4: u32 = 0x006a_9bb0;
pub const LIGHTING_SET_8: u32 = 0x006a_9cd0;
pub const MAGIC_ID: u32 = 0x009d_a3b0;
pub const CHR_RENDER_INSTANCE: u32 = 0x0082_6b70;
pub const CHR_NAME: u32 = 0x0082_6b90;
/// The two summons' names `graphicCharacterReplaceSummonTexture`
/// compares (`m179`, `m180`).
pub const NAME_M179: u32 = 0x00b4_1c64;
pub const NAME_M180: u32 = 0x00b4_1c6c;
pub const SCENE_IS_VFX_TEXTURE_LOADED: u32 = 0x0066_04d0;
pub const CLUSTER_MANAGER: u32 = 0x0069_a1a0;
pub const CLUSTER_BY_NAME: u32 = 0x0069_b4b0;
pub const CLUSTER_RELEASE: u32 = 0x0069_bdb0;
pub const SUMMON_REPLACE: u32 = 0x0063_c0c0;
pub const SUMMON_UPDATE: u32 = 0x0063_dad0;
pub const UNIFY_FILENAME: u32 = 0x0067_9820;
pub const SCENE_UNLOAD_VFX_TEXTURE: u32 = 0x0067_1300;
/// The two persistent VFX texture lists (`bat_eff`, `et_ffx`).
pub const PERSISTENT_A: u32 = 0x00b4_3a80;
pub const PERSISTENT_B: u32 = 0x00b4_3ad0;
pub const SCENE_UNLOAD_MENU_TEXTURE: u32 = 0x0067_1120;
pub const STRING_LIST_CLEAR: u32 = 0x0068_6da0;
pub const COPY_ARRAY: u32 = 0x0063_5630;
pub const ARRAY_DATA: u32 = 0x004d_45a0;
pub const FIND_PARAM: u32 = 0x0056_cbf0;
pub const MEMCPY: u32 = 0x0094_9256;
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// `hsv_scale` and the clip plane parameter's name.
pub const HSV_SCALE: u32 = 0x00b4_1bd8;
pub const CLIP_PLANE: u32 = 0x00b4_1be4;
pub const SHADER_SET_TYPE: u32 = 0x006d_fad0;
pub const SHADER_RESET: u32 = 0x006d_f620;
pub const SHADER_SET_PARAMETER: u32 = 0x006d_f8c0;
pub const SHADER_SET_PARAMETER_TEXTURE: u32 = 0x006d_f950;
pub const SCENE_DETACH_CHR_CLUSTER: u32 = 0x0065_a550;
pub const OPERATOR_NEW: u32 = 0x0063_0310;
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
pub const FREE_CAMERA_CTOR: u32 = 0x0067_fe70;
pub const FREE_CAMERA_SET_MODE: u32 = 0x0068_03a0;
pub const SCENE_MONSTER_ET_NAME: u32 = 0x0065_e200;
pub const SEGMENTS_ALL_READY: u32 = 0x0065_2250;
pub const TEXANIM_MANAGER: u32 = 0x006b_02e0;
pub const TEXANIM_UNREGISTER_ALL: u32 = 0x006b_1b60;
pub const VECTOR_COPY: u32 = 0x0064_d620;
pub const TREE_ERASE: u32 = 0x0063_5ce0;

/// `x` as the x87 passes it on: a signalling NaN gets its quiet bit.
#[must_use]
pub const fn quiet(x: u32) -> u32 {
    if x & 0x7f80_0000 == 0x7f80_0000 && x & 0x007f_ffff != 0 {
        x | 0x0040_0000
    } else {
        x
    }
}

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::Mem;
    use ffx_graphic::host::{Conv, Host};

    fn byte(h: &impl Mem, at: u32) -> u8 {
        h.read(at)
    }

    fn word(h: &impl Mem, at: u32) -> u32 {
        h.read(at)
    }

    fn method(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    fn cdecl(h: &mut impl Host, f: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Cdecl, args)
    }

    fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Cdecl, args)
    }

    /// `strcmp(a, b) == 0` as the original's inline loop.
    fn same_name(h: &impl Mem, a: u32, b: u32) -> bool {
        let mut k = 0;
        loop {
            let (x, y) = (byte(h, a + k), byte(h, b + k));
            if x != y {
                return false;
            }
            if x == 0 {
                return true;
            }
            k += 1;
        }
    }

    /// `getCurrentControlMode()` -> the input manager's vibration flag
    /// (`al`, zero-extended).
    pub fn current_control_mode(h: &mut impl Host) -> u32 {
        let m = cdecl(h, INPUT_MANAGER, &[]);
        method(h, INPUT_VIBRATION_FLAG, m, &[]) & 0xff
    }

    /// `graphicGetSkipTexturePath_00638e90()` -> the skip texture's path
    /// for the language (`LocalizationManager +4`; no manager: 0); for 6,
    /// 7, 8 or past 10 a warning on stderr and 0.
    pub fn skip_texture_path(h: &mut impl Host) -> u32 {
        let lang = if cdecl(h, LOCALIZATION_MANAGER, &[]) == 0 {
            0
        } else {
            let m = cdecl(h, LOCALIZATION_MANAGER, &[]);
            word(h, m + 4)
        };
        match SKIP_TEXTURES.get(lang as usize) {
            Some(&p) if p != 0 => h.rebase(p),
            _ => {
                let iob = import(h, IOB_FUNC, &[]);
                let text = h.rebase(SKIP_TEXTURE_WARNING);
                import(h, FPRINTF, &[iob + 0x40, text]);
                0
            }
        }
    }

    /// `graphicActivateRadialBlur()`: `BlurEffect_activate` on the blur
    /// effect (a tail jump).
    pub fn activate_radial_blur(h: &mut impl Host) -> u32 {
        let b = cdecl(h, BLUR_EFFECT, &[]);
        method(h, BLUR_ACTIVATE, b, &[])
    }

    /// `graphicAddScreenFadeTaskBeforeVFX(r, g, b, a, frames)` ->
    /// `ScreenFadeDelegations_registerDelegation(r, g, b, a, frames,
    /// 0x8001)`'s `eax` (the colour through the x87: [`quiet`]).
    pub fn add_screen_fade_before_vfx(h: &mut impl Host, c: [u32; 4], frames: u32) -> u32 {
        cdecl(
            h,
            REGISTER_SCREEN_FADE,
            &[
                quiet(c[0]),
                quiet(c[1]),
                quiet(c[2]),
                quiet(c[3]),
                frames,
                0x8001,
            ],
        )
    }

    /// `PhyreScene_attachChrClusterTo3DWorld(scene)`: unless attached
    /// (`+0xfb44`), the character cluster `+0x11e4` moved from the world
    /// `+0x9c` to `+0x7c` and the byte set.
    pub fn attach_chr_cluster(h: &mut impl Host, scene: u32) {
        if byte(h, scene + 0xfb44) != 0 {
            return;
        }
        let c = scene + 0x11e4;
        method(h, WORLD_REMOVE_CLUSTER, scene + 0x9c, &[c]);
        method(h, WORLD_ADD_CLUSTER, scene + 0x7c, &[c]);
        h.write(scene + 0xfb44, 1_u8);
    }

    /// `PhyreScene_detachChrClusterFrom3DWorld_0065a550(scene)`: attached,
    /// the cluster moved back (`+0x7c` to `+0x9c`) and the byte cleared.
    pub fn detach_chr_cluster(h: &mut impl Host, scene: u32) {
        if byte(h, scene + 0xfb44) == 0 {
            return;
        }
        let c = scene + 0x11e4;
        method(h, WORLD_REMOVE_CLUSTER, scene + 0x7c, &[c]);
        method(h, WORLD_ADD_CLUSTER, scene + 0x9c, &[c]);
        h.write(scene + 0xfb44, 0_u8);
    }

    /// `graphicDetachChrClusterFrom3DWorld()`: on `g_PhyreScene` (a tail
    /// jump).
    pub fn graphic_detach_chr_cluster(h: &mut impl Host) -> u32 {
        let scene = SCENE.get(h);
        method(h, SCENE_DETACH_CHR_CLUSTER, scene, &[])
    }

    /// `graphicCharacterInstanceSetDrawSituationCallback(inst, cb)`: `+0x1c`.
    pub fn set_draw_situation_callback(h: &mut impl Host, inst: u32, cb: u32) {
        if inst != 0 {
            h.write(inst + 0x1c, cb);
        }
    }

    /// `graphicCharacterLightingSet4_0063c9e0(list, a, b, c, d)`: for every
    /// instance of the list (count low 31 bits, data `+4`) the character
    /// lighting's `0x006a9bb0(inst, a, b, c, d)` (the floats through the
    /// x87: [`quiet`]).
    pub fn lighting_set_4(h: &mut impl Host, list: u32, f: [u32; 4]) {
        lighting_set(h, list, LIGHTING_SET_4, &f);
    }

    /// `graphicCharacterLightingSet8_0063ca50(list, 8 floats)`: as
    /// [`lighting_set_4`] with `0x006a9cd0` and eight floats.
    pub fn lighting_set_8(h: &mut impl Host, list: u32, f: [u32; 8]) {
        lighting_set(h, list, LIGHTING_SET_8, &f);
    }

    fn lighting_set(h: &mut impl Host, list: u32, f: u32, floats: &[u32]) {
        if list == 0 {
            return;
        }
        let mut i = 0;
        while i < word(h, list) & 0x7fff_ffff {
            let inst = word(h, word(h, list + 4) + 4 * i);
            let mut args = vec![inst];
            args.extend(floats.iter().map(|&x| quiet(x)));
            let l = cdecl(h, CHARACTER_LIGHTING, &[]);
            // call width: 9 (the instance and at most eight floats)
            method(h, f, l, &args);
            i += 1;
        }
    }

    /// `graphicCharacterReplaceSummonTexture(chr)` (frame: the cluster
    /// name at `ebp - 0x104`): during magic 0x276..=0x27d, the summon's
    /// texture set (1; 2 for `m179` and 3 for `m180` during 0x27d) looked
    /// up (`PhyreScene_isVFXTextureLoaded(set, 0x13, 0, 0, 1, 1, name)`);
    /// loaded (2), or found (1) and its cluster present, the character's
    /// textures replaced (`0x0063c0c0(inst, name)`, `0x0063dad0(inst)`);
    /// then, below 5, the instance appended to `g_TexAnimSkip`.
    pub fn replace_summon_texture(h: &mut impl Host, ebp: u32, chr: u32) {
        let id = cdecl(h, MAGIC_ID, &[]);
        if id.cast_signed() < 0x276 {
            return;
        }
        if cdecl(h, MAGIC_ID, &[]).cast_signed() > 0x27d {
            return;
        }
        let inst = cdecl(h, CHR_RENDER_INSTANCE, &[chr]);
        let name = cdecl(h, CHR_NAME, &[chr]);
        let mut set = 1;
        if cdecl(h, MAGIC_ID, &[]) == 0x27d && same_name(h, name, h.rebase(NAME_M179)) {
            set = 2;
        }
        if cdecl(h, MAGIC_ID, &[]) == 0x27d && same_name(h, name, h.rebase(NAME_M180)) {
            set = 3;
        }
        let buf = ebp - 0x104;
        let scene = SCENE.get(h);
        let r = method(
            h,
            SCENE_IS_VFX_TEXTURE_LOADED,
            scene,
            &[set, 0x13, 0, 0, 1, 1, buf],
        );
        let found = r == 2
            || (r == 1 && {
                let m = cdecl(h, CLUSTER_MANAGER, &[]);
                method(h, CLUSTER_BY_NAME, m, &[buf]) != 0
            });
        if found {
            cdecl(h, SUMMON_REPLACE, &[inst, buf]);
            cdecl(h, SUMMON_UPDATE, &[inst]);
        }
        let n = TEX_ANIM_SKIP.get(h);
        if n.cast_signed() < 5 {
            let n = n.wrapping_add(1);
            TEX_ANIM_SKIP.set(h, n);
            let at = h.rebase(TEX_ANIM_SKIP.sym).wrapping_add(n.wrapping_mul(4));
            h.write(at, inst);
        }
    }

    /// `PhyreScene_unloadPersistentVFXTexture(scene)` (frame: the two
    /// unified names at `ebp - 0x104` and `ebp - 0x204`): the `bat_eff`
    /// list unloaded (`PhyreScene_unloadVFXTexture(name, -1)`), else the
    /// `et_ffx` one; the unload's `eax`.
    pub fn unload_persistent_vfx_texture(h: &mut impl Host, ebp: u32, scene: u32) -> u32 {
        let (a, b) = (ebp - 0x104, ebp - 0x204);
        let pa = h.rebase(PERSISTENT_A);
        cdecl(h, UNIFY_FILENAME, &[pa, a, 0x100]);
        let pb = h.rebase(PERSISTENT_B);
        cdecl(h, UNIFY_FILENAME, &[pb, b, 0x100]);
        let r = method(h, SCENE_UNLOAD_VFX_TEXTURE, scene, &[a, 0xffff_ffff]);
        if r != 0 {
            return r;
        }
        method(h, SCENE_UNLOAD_VFX_TEXTURE, scene, &[b, 0xffff_ffff])
    }

    /// `MenuTextureManager_unloadDefaultTextures_006857c0()` (frame: the
    /// unified name at `ebp - 0x104`): every name of the default menu
    /// texture list (`std::string` at node `+8`, long past capacity 16 at
    /// `+0x1c`) unified and unloaded from the scene; the list cleared
    /// (`StdStringList_Clear`'s `eax`).
    pub fn unload_default_menu_textures(h: &mut impl Host, ebp: u32) -> u32 {
        let list = h.rebase(MENU_DEFAULT_TEXTURES);
        let buf = ebp - 0x104;
        let mut node = word(h, word(h, list));
        while node != word(h, list) {
            let s = if word(h, node + 0x1c) >= 0x10 {
                word(h, node + 8)
            } else {
                node + 8
            };
            cdecl(h, UNIFY_FILENAME, &[s, buf, 0x100]);
            let scene = SCENE.get(h);
            method(h, SCENE_UNLOAD_MENU_TEXTURE, scene, &[buf]);
            node = word(h, node);
        }
        method(h, STRING_LIST_CLEAR, list, &[])
    }

    /// Every material of a model's meshes: the model's mesh array copied
    /// (`0x00635630`), each mesh's `+0x10` materials (`PArray_Data`), the
    /// parameter `name` found (`ShaderParams_FindByName`) and, when its
    /// offset (`u16 +8`, at least 4) and size (`u16 +0xa` low 13 bits) fit
    /// the material's buffer (`+4`, size `+0`), `size` bytes copied from
    /// `src` (`memcpy`); the copy freed when owned. The mesh index is kept
    /// in the caller's first argument slot `slot`.
    fn set_material_param(h: &mut impl Host, slot: u32, model: u32, name: u32, src: Option<u32>) {
        let n = word(h, model) & 0x7fff_ffff;
        let data = word(h, model + 4);
        let meshes = cdecl(h, COPY_ARRAY, &[n, data]);
        let count = if meshes == 0 { 0 } else { n };
        h.write(slot, 0_u32);
        let name = h.rebase(name);
        let mut k = 0_u32;
        while k < count & 0x7fff_ffff {
            let materials = word(h, word(h, meshes + 4 * k) + 0x10);
            let mut i = 0;
            while i < word(h, materials) & 0x7fff_ffff {
                let list = method(h, ARRAY_DATA, materials, &[]);
                let m = word(h, list + 4 * i);
                let params = word(h, m);
                let p = method(h, FIND_PARAM, params, &[name]);
                if let (true, Some(src)) = (p != 0, src) {
                    let buf = word(h, m + 4);
                    let off = u32::from(h.read::<u16>(p + 8));
                    let size = u32::from(h.read::<u16>(p + 0xa) & 0x1fff);
                    if off + size <= word(h, buf) && off >= 4 {
                        cdecl(h, MEMCPY, &[buf + off, src, size]);
                    }
                }
                i += 1;
            }
            k += 1;
            h.write(slot, k);
        }
        if count >> 31 == 0 {
            cdecl(h, PHYRE_FREE, &[meshes]);
        }
    }

    /// `graphicSetHSVParam(model, h, s, v, a)` (frame: the four floats at
    /// `ebp - 0x20`, through the x87: [`quiet`]): `hsv_scale` set on every
    /// material ([`set_material_param`]). The mesh index is kept in its
    /// first argument slot `slot`.
    pub fn set_hsv_param(h: &mut impl Host, ebp: u32, slot: u32, model: u32, f: [u32; 4]) {
        if model == 0 {
            return;
        }
        let src = ebp - 0x20;
        for (k, &x) in f.iter().enumerate() {
            h.write(src + 4 * k as u32, quiet(x));
        }
        set_material_param(h, slot, model, HSV_SCALE, Some(src));
    }

    /// `graphicCharacterSetClipPlane(model, plane)`: the clip plane
    /// parameter set from `plane` (none: looked up, not set) on every
    /// material ([`set_material_param`]); the mesh index kept in its
    /// first argument slot `slot`.
    pub fn set_clip_plane(h: &mut impl Host, slot: u32, model: u32, plane: u32) {
        if model == 0 {
            return;
        }
        let src = if plane == 0 { None } else { Some(plane) };
        set_material_param(h, slot, model, CLIP_PLANE, src);
    }

    /// The character instance's meshes (count low 31 bits, data `+4`)
    /// each handed to `f(mesh, args...)` (cdecl); 0.
    fn each_mesh(h: &mut impl Host, chr: u32, f: u32, args: &[u32]) -> u32 {
        let inst = cdecl(h, CHR_RENDER_INSTANCE, &[chr]);
        if inst == 0 {
            return 0;
        }
        let mut i = 0;
        while i < word(h, inst) & 0x7fff_ffff {
            let mesh = word(h, word(h, inst + 4) + 4 * i);
            let mut a = vec![mesh];
            a.extend_from_slice(args);
            cdecl(h, f, &a);
            i += 1;
        }
        0
    }

    /// `graphicCharacterSetShaderType(chr, kind)` -> 0.
    pub fn set_shader_type(h: &mut impl Host, chr: u32, kind: u32) -> u32 {
        each_mesh(h, chr, SHADER_SET_TYPE, &[kind])
    }

    /// `graphicCharacterShaderRestore()` -> 0.
    pub fn shader_restore(h: &mut impl Host) -> u32 {
        cdecl(h, SHADER_RESET, &[]);
        0
    }

    /// `graphicCharacterShaderSetParameter(chr, name, value)` -> 0.
    pub fn shader_set_parameter(h: &mut impl Host, chr: u32, name: u32, value: u32) -> u32 {
        each_mesh(h, chr, SHADER_SET_PARAMETER, &[name, value])
    }

    /// `graphicCharacterShaderSetParameterTexture(chr, name, texture)` -> 0.
    pub fn shader_set_parameter_texture(h: &mut impl Host, chr: u32, name: u32, tex: u32) -> u32 {
        each_mesh(h, chr, SHADER_SET_PARAMETER_TEXTURE, &[name, tex])
    }

    /// `graphicSetFreeCameraMode_00640e20(mode)`: the free camera (made on
    /// demand, as in `graphicGetFreeCameraType`) `0x006803a0(cam; mode)`,
    /// its `eax`.
    pub fn set_free_camera_mode(h: &mut impl Host, mode: u32) -> u32 {
        let mut cam = FREE_CAMERA.get(h);
        if cam == 0 {
            let p = cdecl(h, OPERATOR_NEW, &[0x340]);
            cam = if p == 0 {
                0
            } else {
                method(h, FREE_CAMERA_CTOR, p, &[])
            };
            FREE_CAMERA.set(h, cam);
        }
        method(h, FREE_CAMERA_SET_MODE, cam, &[mode])
    }

    /// `PhyreScene_getMonsterEtTextureName(scene; et, which)` (`ret 8`) ->
    /// the name slot of the first of the 8 monster texture records
    /// (`+0xfccc`, 0x20c apart: used word, the `et`'s id `[et + 0x10]`,
    /// two `u16` kinds) holding `which` (compared as a whole word): the
    /// record's `+0xfcd8 + 0x100 j`; 0 if none.
    pub fn monster_et_texture_name(h: &mut impl Host, scene: u32, et: u32, which: u32) -> u32 {
        for e in 0..8 {
            let rec = scene + 0xfcd0 + 0x20c * e;
            if word(h, rec - 4) == 0 || word(h, rec) != word(h, et + 0x10) {
                continue;
            }
            for j in 0..2 {
                if u32::from(h.read::<u16>(rec + 4 + 2 * j)) == which {
                    return scene + 0x20c * e + 0xfcd8 + (j << 8);
                }
            }
        }
        0
    }

    /// `graphicGetMonsterEtTextureName(et, which)`: on `g_PhyreScene`.
    pub fn graphic_monster_et_texture_name(h: &mut impl Host, et: u32, which: u32) -> u32 {
        let scene = SCENE.get(h);
        method(h, SCENE_MONSTER_ET_NAME, scene, &[et, which])
    }

    /// `graphicSegmentsAllReady_006522b0()`: `SegmentList_AllReady` on
    /// `g_PhyreScene` (a tail jump).
    pub fn segments_all_ready(h: &mut impl Host) -> u32 {
        let scene = SCENE.get(h);
        method(h, SEGMENTS_ALL_READY, scene, &[])
    }

    /// `PhyreScene_releaseCharacterClusters_00670960(scene)` -> 0 (frame:
    /// the cluster vector at `ebp - 0x20`, `ebp - 0x10` the main cluster):
    /// texture animations unregistered; for every entry of the scene's
    /// character map (`+0x11dc`, value `+0x28`: main cluster `+0`, vector
    /// `+4..+0xc`): the vector copied into the frame (`0x0064d620`), each
    /// cluster in it released, the entry's vector emptied, the main
    /// cluster released, the entry's storage and the entry deleted, the
    /// frame copy deleted; then the map erased (`0x00635ce0` from the root)
    /// and emptied, texture animations unregistered again.
    pub fn release_character_clusters(h: &mut impl Host, ebp: u32, scene: u32) -> u32 {
        let texanim = cdecl(h, TEXANIM_MANAGER, &[]);
        method(h, TEXANIM_UNREGISTER_ALL, texanim, &[]);
        let map = scene + 0x11dc;
        let v = ebp - 0x20;
        let mut node = word(h, word(h, map));
        while node != word(h, map) {
            let entry = word(h, node + 0x28);
            let main = word(h, entry);
            h.write(ebp - 0x10, main);
            method(h, VECTOR_COPY, v, &[entry + 4]);
            let (start, end) = (word(h, v), word(h, v + 4));
            let mut p = start;
            while p != end {
                let c = word(h, p);
                let m = cdecl(h, CLUSTER_MANAGER, &[]);
                method(h, CLUSTER_RELEASE, m, &[c]);
                p += 4;
            }
            let entry = word(h, node + 0x28);
            let first = word(h, entry + 4);
            h.write(entry + 8, first);
            let main = word(h, ebp - 0x10);
            let m = cdecl(h, CLUSTER_MANAGER, &[]);
            method(h, CLUSTER_RELEASE, m, &[main]);
            let entry = word(h, node + 0x28);
            if entry != 0 {
                let storage = word(h, entry + 4);
                if storage != 0 {
                    cdecl(h, OPERATOR_DELETE, &[storage]);
                    h.write(entry + 4, 0_u32);
                    h.write(entry + 8, 0_u32);
                    h.write(entry + 0xc, 0_u32);
                }
                cdecl(h, OPERATOR_DELETE, &[entry]);
            }
            let copy = word(h, v);
            if copy != 0 {
                cdecl(h, OPERATOR_DELETE, &[copy]);
                h.write(v, 0_u32);
                h.write(v + 4, 0_u32);
                h.write(v + 8, 0_u32);
            }
            node = crate::m5r_a::bridge::tree_next(h, node);
        }
        let root = word(h, word(h, map) + 4);
        method(h, TREE_ERASE, map, &[root]);
        let head = word(h, map);
        h.write(head + 4, head);
        let head = word(h, map);
        h.write(head, head);
        let head = word(h, map);
        h.write(head + 8, head);
        h.write(scene + 0x11e0, 0_u32);
        let texanim = cdecl(h, TEXANIM_MANAGER, &[]);
        method(h, TEXANIM_UNREGISTER_ALL, texanim, &[]);
        0
    }
}
