//! M5 stage 1, list `render-pcui` (round 6), batch 1: the scene's water
//! wave presets, the cloud UI pass and ATL's `CStringW::Replace`.

use crate::m5r_c::quiet;
use ffx_bridge::Global;

/// `ObjectTree_Find` (`thiscall(tree; key)`, `ret 4`).
pub const OBJECT_TREE_FIND: u32 = 0x0064_b9f0;
/// The water wave controller's setters (all `stdcall`, `ecx` unread).
pub const WAVE_BLEND: u32 = 0x006f_5840;
pub const WAVE_FOAM: u32 = 0x006f_5810;
pub const WAVE_SINE: u32 = 0x006f_59a0;
pub const WAVE_CIRCLE: u32 = 0x006f_5790;
pub const WAVE_DRAUGHT: u32 = 0x006f_57e0;
pub const WAVE_BUOYANCY: u32 = 0x006f_5760;
pub const WAVE_SCROLL: u32 = 0x006f_5870;
/// The wave preset (`0x025d5eb8`, set by `Native_805c`/`Native_806b`).
pub const WAVE_MODE: Global<u32> = Global::new(0x025d_5eb8);
/// `.rdata` floats of the presets, in the order the frame takes them
/// (`ebp - 0x18 .. ebp - 0x8`).
pub const WAVE_FRAME_FLOATS: [u32; 5] = [
    0x00b1_0f38,
    0x00b4_3d30,
    0x00b4_3d28,
    0x00b9_2194,
    0x00b1_0f3c,
];
/// The blend factor and foam strength of presets 0, 1 and 2 (1's foam is
/// `fld1`), and preset 1's scroll speed.
pub const WAVE_BLEND_0: u32 = 0x00b9_2180;
pub const WAVE_FOAM_0: u32 = 0x00b3_eb48;
pub const WAVE_BLEND_1: u32 = 0x00b4_3d10;
pub const WAVE_BLEND_2: u32 = 0x00b4_3d04;
pub const WAVE_FOAM_2: u32 = 0x00b3_4254;
pub const WAVE_SCROLL_1: u32 = 0x00b9_2338;

pub const SET_COLOR_TARGET: u32 = 0x005b_b130;
pub const SET_CLEAR_COLOR: u32 = 0x005b_ada0;
pub const BEGIN_SCENE: u32 = 0x005b_5e30;
pub const SET_CAMERA: u32 = 0x005b_abd0;
pub const UPDATE_VIEW_MATRICES: u32 = 0x0067_20c0;
pub const VIEWER_MANAGER_INSTANCE: u32 = 0x006b_6370;
pub const MENU_COUNT: u32 = 0x0067_b970;
pub const MENU_MESH_INSTANCE: u32 = 0x0067_b9d0;
pub const NAME_IS: u32 = 0x004e_a7a0;
pub const RENDER_MESH_INSTANCE: u32 = 0x005b_9a90;
pub const END_SCENE: u32 = 0x005b_6950;
/// `g_GraphicUi` (the menu manager).
pub const GRAPHIC_UI: Global<u32> = Global::new(0x00cc_c81c);
/// The pass name a cloud UI mesh must not have (`0x00ccb9e0`, low bit
/// cleared before the compare).
pub const CLOUD_SKIP_NAME: Global<u32> = Global::new(0x00cc_b9e0);
/// The renderer's pass names: `"Opaque"`, `"Transparent"` (pointer tables).
pub const PASS_OPAQUE: u32 = 0x00c1_69b4;
pub const PASS_TRANSPARENT: u32 = 0x00c1_69d4;

/// `wcsstr`, `memmove_s`, `memcpy_s` (MSVCR110 import slots, `cdecl`).
pub const WCSSTR: u32 = 0x00b0_c404;
pub const MEMMOVE_S: u32 = 0x00b0_c438;
pub const MEMCPY_S: u32 = 0x00b0_c434;
pub const PREPARE_WRITE2: u32 = 0x0065_2600;
pub const ATL_CRT_ERROR_CHECK: u32 = 0x0064_f8d0;
/// `ThrowStdException_0064fa50(hr)`: never returns.
pub const THROW: u32 = 0x0064_fa50;
pub const E_INVALIDARG: u32 = 0x8007_0057;

/// `std::string::assign(src, pos, n)` (`thiscall`, `ret 0xc`).
pub const STRING_ASSIGN_SUBSTR: u32 = 0x0040_3a10;
/// The cluster list's vector copy constructor (`thiscall(this; src)`,
/// `ret 4`).
pub const CLUSTER_VECTOR_COPY: u32 = 0x0064_d620;
/// `PClusterHeader` iterator seek (`thiscall(it)`).
pub const ITER_SEEK: u32 = 0x0046_4f30;
/// `std::vector<T*>::reserve` storage swap (`thiscall(this; n)`, `ret 4`).
pub const VECTOR_REALLOC: u32 = 0x0065_59a0;
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
/// `strstr`, `std::_Xlength_error` (import slots, `cdecl`).
pub const STRSTR: u32 = 0x00b0_c378;
pub const XLENGTH_ERROR: u32 = 0x00b0_c29c;
/// `"vector<T> too long"`.
pub const VECTOR_TOO_LONG: u32 = 0x00b3_4498;
/// `g_ClassDesc_PTexture2D_00ca37d0`.
pub const CLASS_TEXTURE_2D: u32 = 0x00ca_37d0;

/// The shadow mesh's member assignments (`thiscall(this; src)`, `ret 4`):
/// `+8` and `+0x10`.
pub const SHADOW_ASSIGN_8: u32 = 0x0048_1940;
pub const SHADOW_ASSIGN_10: u32 = 0x0049_6aa0;
/// The gradient mesh's `.rdata` -1.0f and 0.5f.
pub const GRAD_MINUS_ONE: u32 = 0x00b9_2340;
pub const GRAD_HALF: u32 = 0x00b9_2194;

pub const TEXT_INIT: u32 = 0x006f_6370;
pub const TEXT_APPEND_LINE: u32 = 0x006f_5e30;
pub const TEXT_SET_ENABLED: u32 = 0x006f_68f0;
/// `crt_memset_thunk` (`cdecl`).
pub const MEMSET: u32 = 0x0094_964c;
/// `sprintf_s` (import slot, `cdecl`).
pub const SPRINTF_S: u32 = 0x00b0_c394;
/// `g_PostProcessManager`.
pub const POST_PROCESS_MANAGER: Global<u32> = Global::new(0x00cc_c8a0);
/// The fog far tweak's step 50.0 (`.rdata` double).
pub const TWEAK_FOG_STEP: u32 = 0x00b1_f2c0;
/// `">%s: %.3f"` (the selected line), `"%s: %.3f"`.
pub const TWEAK_LINE_SELECTED: u32 = 0x00b4_7c24;
pub const TWEAK_LINE: u32 = 0x00b4_7c30;
/// The post-process tweak's 22 labels (`"Fog far"`, ...) by the slot
/// they go in (`+0x98 + 4 k`).
pub const TWEAK_LABELS: [(u32, u32); 22] = [
    (0x98, 0x00b4_7ab4),
    (0x9c, 0x00b4_7abc),
    (0xa0, 0x00b4_7ac8),
    (0xa4, 0x00b4_7ad4),
    (0xa8, 0x00b4_7ae0),
    (0xac, 0x00b4_7af0),
    (0xb0, 0x00b4_7b00),
    (0xb4, 0x00b4_7b10),
    (0xb8, 0x00b4_7b1c),
    (0xbc, 0x00b4_7b34),
    (0xc0, 0x00b4_7b4c),
    (0xc4, 0x00b4_7b58),
    (0xc8, 0x00b4_7b64),
    (0xd0, 0x00b4_7b70),
    (0xd4, 0x00b4_7b7c),
    (0xd8, 0x00b4_7b8c),
    (0xdc, 0x00b4_7ba4),
    (0xe0, 0x00b4_7bbc),
    (0xe4, 0x00b4_7bd8),
    (0xe8, 0x00b4_7bec),
    (0xec, 0x00b4_7c00),
    (0xcc, 0x00b4_7c0c),
];

/// `Phyre_PStringBuilder_operator+=(text)` (`thiscall`, `ret 4`).
pub const STRING_BUILDER_APPEND: u32 = 0x0044_4bf0;
/// A newline string.
pub const NEWLINE: u32 = 0x00b7_cae0;
pub const TEXT_REBUILD: u32 = 0x006f_6420;
pub const FONT_SET_TEXT: u32 = 0x00a2_aa30;
pub const RENDERER_PUSH_COMMAND20: u32 = 0x005b_b2e0;
pub const FONT_RENDER_INSTANCE: u32 = 0x00a2_a400;
pub const TEXT_SET_TITLE: u32 = 0x006f_6930;
pub const TEXT_RENDER: u32 = 0x006f_65b0;
/// `"PostProcess: %.2f fps"`.
pub const TWEAK_TITLE: u32 = 0x00b4_7c3c;
/// The 4:3 debug frame's pass name (`0x00ccc95c`).
pub const DEBUG_FRAME_PASS: u32 = 0x00cc_c95c;

pub const COPY_BUFFER: u32 = 0x0069_2c40;
pub const SURFACE_CTOR: u32 = 0x005b_35c0;
/// `PRenderSurface`'s destructor: a bare `ret`.
pub const SURFACE_DTOR: u32 = 0x005b_3640;
pub const SET_DEPTH_STENCIL_TARGET: u32 = 0x005b_b5b0;
pub const FIND_PARAM: u32 = 0x0056_cbf0;
/// `writeVectorParameter` (`Render_SurfaceFormat_0066e4f0`, `ret 8`).
pub const WRITE_VECTOR_PARAM: u32 = 0x0066_e4f0;
pub const SHADER_SET_VALUE: u32 = 0x0063_8780;
pub const SHADER_SET_VALUE2: u32 = 0x0063_86c0;
pub const SET_VIEWPORT: u32 = 0x005b_c060;
pub const GET_APPLICATION: u32 = 0x0062_3b90;
/// `"ColorBuffer"`, `"ColorScale"`, `"FocusDepth"`, `"UvScaleBias"`,
/// `"DepthBuffer2/4/8"`.
pub const P_COLOR_BUFFER: u32 = 0x00b4_25d8;
pub const P_COLOR_SCALE: u32 = 0x00b4_7a48;
pub const P_FOCUS_DEPTH: u32 = 0x00b4_7a54;
pub const P_UV_SCALE_BIAS: u32 = 0x00b4_25e4;
pub const P_DEPTH_BUFFER: [(u8, u32, u32); 3] = [
    (2, 0x00b4_7a84, 0x00cc_c91c),
    (4, 0x00b4_7a94, 0x00cc_c92c),
    (8, 0x00b4_7aa4, 0x00cc_c93c),
];
/// The focus frame's pass name, its colour scale's alpha 0.6f and the
/// depth range's -1.0f.
pub const FOCUS_PASS: u32 = 0x00cc_c8ec;
pub const FOCUS_ALPHA: u32 = 0x00b3_eb48;
pub const MINUS_ONE: u32 = 0x00b9_2340;

pub const OPERATOR_NEW: u32 = 0x0063_0310;
/// `Clear4_006e0800` (`thiscall`, returns `this`).
pub const CLEAR4: u32 = 0x006e_0800;
pub const CLOTH_ATTACH: u32 = 0x006e_2600;
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
pub const PARRAY_PUSH_INTO: u32 = 0x006a_2700;
pub const TEXT_UTIL_CTOR: u32 = 0x006f_5b50;
/// The tweak panels' text scale 0.02f, `"%s"` and `"test"`.
pub const TWEAK_TEXT_SCALE: u32 = 0x00b4_82d8;
pub const FORMAT_S: u32 = 0x00b0_e160;
pub const TEXT_TEST: u32 = 0x00b4_8b14;

/// `g_ViewerManager`-side globals: the texture animation viewer
/// (`[0x00cccad4] + 0x18`).
pub const TEXANIM_VIEWER: Global<u32> = Global::new(0x00cc_cad4);
/// `"grid00"`.
pub const GRID00: u32 = 0x00b4_24c8;
pub const TEXANIM_PAIR_2C: u32 = 0x006b_1ce0;
pub const TEXANIM_PAIR_24: u32 = 0x006b_1cc0;
/// `Sg_PlayerSetAnimatedMaterials_00829c00` (`cdecl`, ten words).
pub const SET_ANIMATED_MATERIALS: u32 = 0x0082_9c00;
/// `vector<std::string>` growth (`thiscall(this; n)`, `ret 4`) and the
/// string copy constructor (`thiscall(this; src)`, `ret 4`).
pub const STRING_VECTOR_GROW: u32 = 0x0065_5de0;
pub const STRING_COPY_CTOR: u32 = 0x0040_2cf0;
/// `std::string::assign(text, n)` (`thiscall`, `ret 8`).
pub const STRING_ASSIGN_N: u32 = 0x0040_4270;
/// `AutoTestManager_pushCommand_006bd160`.
pub const AUTOTEST_PUSH: u32 = 0x006b_d160;
pub const INIT_EVENT_LIST: u32 = 0x0090_7f50;
pub const FLAG_01326B28: u32 = 0x0087_29c0;
pub const VIEWER_JUMP_MAP: u32 = 0x006b_6730;
pub const GET_APP_TIME: u32 = 0x0042_f690;
/// `printf`, `sscanf` (import slots, `cdecl`).
pub const PRINTF: u32 = 0x00b0_c380;
pub const SSCANF: u32 = 0x00b0_c448;
/// The auto test commands: `"EnableAutoTest"`, `"DisableAutoTest"`,
/// `"Execute:%s\n"`, `"JumpMap %s"`, `"Crash"`, `"SetBattle %s"`,
/// `"Enabled"`, `"Disabled"`.
pub const AT_ENABLE: u32 = 0x00b4_8e40;
pub const AT_DISABLE: u32 = 0x00b4_8e90;
pub const AT_EXECUTE: u32 = 0x00b4_8ea0;
pub const AT_JUMP_MAP: u32 = 0x00b4_8eac;
pub const AT_CRASH: u32 = 0x00b4_8eb8;
pub const AT_SET_BATTLE: u32 = 0x00b4_8ec0;
pub const AT_ENABLED: u32 = 0x00b4_8ed0;
pub const AT_DISABLED: u32 = 0x00b4_8ed8;
/// `g_BtlLabelLock`.
pub const BTL_LABEL_LOCK: Global<u8> = Global::new(0x0112_ca2c);

pub const MAGIC_ID: u32 = 0x009d_a3b0;
pub const SET_VISIBLE: u32 = 0x0063_c390;
pub const SPECULAR_CORRESPOND: u32 = 0x006c_5ec0;
/// The passes of the corresponding specular draw: the z pre-pass (magic
/// 0x144 and 0x272) and the specular pass by the mode (`+0x5a0`): 0
/// no light, 1 lit, 2 `kohno1`, 3 `akaza2`, 4 `akaza1`, 5 `akaza`; 3..5
/// also set the per-character parameters.
pub const PASS_Z_PRE: u32 = 0x00c1_6a74;
pub const PASS_SPECULAR: [u32; 6] = [
    0x00c1_6aa4,
    0x00c1_6a94,
    0x00c1_6ab4,
    0x00c1_6ad4,
    0x00c1_6ac4,
    0x00c1_6ae4,
];

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

    fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f = word(h, h.rebase(slot));
        h.call(f, Conv::Cdecl, args)
    }

    /// A float constant as `fld dword; fstp dword` copies it.
    fn f32c(h: &impl Mem, sym: u32) -> u32 {
        quiet(h.read(h.rebase(sym)))
    }

    /// `PhyreScene_setWaveStrength(this)` -> the last setter's `eax` (the
    /// frame at `frame`, the original's `ebp - 0x1c`: the key 0x1e and
    /// five floats). The scene's object tree (`+0x1198`) gives the water
    /// object 0x1e; its controller (`[[[[+0x10] + 8] + 4] + 0x10]`, an
    /// array: count `& 0x7fffffff` above 1 holds a pointer, 1 the value
    /// inline, 0 none) is set to the preset `WAVE_MODE` names: 0 calm, 1
    /// stormy (sine and circle waves, draught, buoyancy, scroll -0.5), 2
    /// the third. Another preset, or no object, sets nothing.
    pub fn set_wave_strength(h: &mut impl Host, frame: u32, this: u32) -> u32 {
        h.write(frame, 0x1e_u32);
        for (k, &c) in (1u32..).zip(&WAVE_FRAME_FLOATS) {
            let v = f32c(h, c);
            h.write(frame + 4 * k, v);
        }
        let node = method(h, OBJECT_TREE_FIND, this.wrapping_add(0x1198), &[frame]);
        if node == 0 || node.wrapping_add(0x10) == 0 {
            return 0;
        }
        let a = word(h, node + 0x10);
        let a = word(h, word(h, word(h, a + 8) + 4) + 0x10);
        let n = word(h, a) & 0x7fff_ffff;
        let slot = match n {
            0 => 0,
            1 => a + 4,
            _ => word(h, a + 4),
        };
        let ctl = word(h, slot);
        let mode = WAVE_MODE.get(h);
        let scroll = match mode {
            0 => {
                let (b, f) = (f32c(h, WAVE_BLEND_0), f32c(h, WAVE_FOAM_0));
                h.call_sym(WAVE_BLEND, Conv::Stdcall, &[ctl, b]);
                h.call_sym(WAVE_FOAM, Conv::Stdcall, &[ctl, f]);
                h.call_sym(WAVE_CIRCLE, Conv::Stdcall, &[ctl, frame + 4, frame + 4]);
                0
            }
            1 => {
                let b = f32c(h, WAVE_BLEND_1);
                h.call_sym(WAVE_BLEND, Conv::Stdcall, &[ctl, b]);
                h.call_sym(WAVE_FOAM, Conv::Stdcall, &[ctl, 1.0_f32.to_bits()]);
                h.call_sym(WAVE_SINE, Conv::Stdcall, &[ctl, frame + 4, frame + 4]);
                h.call_sym(WAVE_CIRCLE, Conv::Stdcall, &[ctl, frame + 8, frame + 0xc]);
                h.call_sym(WAVE_DRAUGHT, Conv::Stdcall, &[ctl, 0]);
                h.call_sym(WAVE_BUOYANCY, Conv::Stdcall, &[ctl, 1.0_f32.to_bits()]);
                f32c(h, WAVE_SCROLL_1)
            }
            2 => {
                let (b, f) = (f32c(h, WAVE_BLEND_2), f32c(h, WAVE_FOAM_2));
                h.call_sym(WAVE_BLEND, Conv::Stdcall, &[ctl, b]);
                h.call_sym(WAVE_FOAM, Conv::Stdcall, &[ctl, f]);
                h.call_sym(WAVE_SINE, Conv::Stdcall, &[ctl, 0, 0]);
                h.call_sym(
                    WAVE_CIRCLE,
                    Conv::Stdcall,
                    &[ctl, frame + 0x10, frame + 0x14],
                );
                0
            }
            _ => return mode.wrapping_sub(2),
        };
        h.call_sym(
            WAVE_SCROLL,
            Conv::Stdcall,
            &[ctl, scroll, scroll, scroll, scroll],
        )
    }

    /// `PhyreScene_RenderCloudUI(this)` -> 0, or the renderer's error
    /// (`setClearColor`, `beginScene`). The renderer (`+0xd0`) draws to
    /// the back buffer, cleared to black, with the scene's camera
    /// (`+0xa88`); with `g_GraphicUi` set and the viewer manager's flag
    /// `+0x50`, every menu mesh instance not named
    /// `CLOUD_SKIP_NAME` is drawn in the opaque, then the transparent
    /// pass.
    pub fn render_cloud_ui<H: Host>(h: &mut H, this: u32) -> u32 {
        let renderer = |h: &mut H| word(h, this.wrapping_add(0xd0));
        let r = renderer(h);
        method(h, SET_COLOR_TARGET, r, &[0, 0]);
        let r = renderer(h);
        let e = method(h, SET_CLEAR_COLOR, r, &[0, 0, 0, 1.0_f32.to_bits()]);
        if e != 0 {
            return e;
        }
        let r = renderer(h);
        let e = method(h, BEGIN_SCENE, r, &[3]);
        if e != 0 {
            return e;
        }
        let camera = if this.wrapping_add(0xa84) == 0 {
            0
        } else {
            this.wrapping_add(0xa88)
        };
        let r = renderer(h);
        method(h, SET_CAMERA, r, &[camera]);
        method(h, UPDATE_VIEW_MATRICES, this.wrapping_add(0xa88), &[]);
        if GRAPHIC_UI.get(h) != 0 {
            let viewer = h.call_sym(VIEWER_MANAGER_INSTANCE, Conv::Cdecl, &[]);
            if h.read::<u8>(viewer.wrapping_add(0x50)) != 0 {
                let ui = GRAPHIC_UI.get(h);
                if method(h, MENU_COUNT, ui, &[]).cast_signed() > 0 {
                    let mut i = 0_u32;
                    loop {
                        let ui = GRAPHIC_UI.get(h);
                        let inst = method(h, MENU_MESH_INSTANCE, ui, &[i]);
                        if inst != 0 {
                            let name = CLOUD_SKIP_NAME.get(h);
                            let name = name.wrapping_sub(name & 1);
                            if method(h, NAME_IS, inst.wrapping_add(0x2c), &[name]) as u8 == 0 {
                                for pass in [PASS_OPAQUE, PASS_TRANSPARENT] {
                                    let r = renderer(h);
                                    let p = h.rebase(pass);
                                    h.write(r + 0x48, p);
                                    let r = renderer(h);
                                    let (mesh, list) = (word(h, inst + 0x10), word(h, r + 0x13c));
                                    method(h, RENDER_MESH_INSTANCE, r, &[inst, mesh, 0, list]);
                                }
                            }
                        }
                        i += 1;
                        let ui = GRAPHIC_UI.get(h);
                        if i.cast_signed() >= method(h, MENU_COUNT, ui, &[]).cast_signed() {
                            break;
                        }
                    }
                }
            }
        }
        let r = renderer(h);
        method(h, END_SCENE, r, &[1]);
        0
    }

    /// The length of the wide string at `p` (the inlined `wcslen`).
    fn wcslen(h: &impl Mem, p: u32) -> u32 {
        let mut n = 0_u32;
        while h.read::<u16>(p.wrapping_add(2 * n)) != 0 {
            n += 1;
        }
        n
    }

    /// The length of the wide string at `p`, 0 for a null pointer.
    fn wcslen_or_0(h: &impl Mem, p: u32) -> u32 {
        if p == 0 { 0 } else { wcslen(h, p) }
    }

    /// ATL's `CStringW::Replace(this; old, new)` (`0x00654070`, `ret 8`)
    /// -> the number of matches. A null or empty `old` replaces nothing.
    /// The matches are counted over every NUL-separated piece of the
    /// buffer (`wcsstr` from each piece's start); with any, the buffer is
    /// made writable for the longer of the old and new lengths
    /// (`PrepareWrite2`), each match's tail moved (`memmove_s`) and `new`
    /// copied in (`memcpy_s`, both checked by `AtlCrtErrorCheck`), and the
    /// new length stored with its NUL. A negative or overlong length
    /// throws `E_INVALIDARG`.
    pub fn string_replace(h: &mut impl Host, this: u32, old: u32, new: u32) -> u32 {
        if old == 0 {
            return 0;
        }
        let old_len = wcslen(h, old);
        if old_len == 0 {
            return 0;
        }
        let new_len = wcslen_or_0(h, new);
        let data = word(h, this);
        let end = data.wrapping_add(2 * word(h, data.wrapping_sub(0xc)));
        let mut count = 0_u32;
        let mut p = data;
        while p < end {
            let mut m = import(h, WCSSTR, &[p, old]);
            while m != 0 {
                p = m.wrapping_add(2 * old_len);
                count += 1;
                m = import(h, WCSSTR, &[p, old]);
            }
            p = p.wrapping_add(2 * wcslen_or_0(h, p)).wrapping_add(2);
        }
        if count.cast_signed() <= 0 {
            return count;
        }
        let mut length = word(h, word(h, this).wrapping_sub(0xc));
        let result = new_len
            .wrapping_sub(old_len)
            .wrapping_mul(count)
            .wrapping_add(length);
        let need = if result.cast_signed() > length.cast_signed() {
            result
        } else {
            length
        };
        if need.cast_signed() < 0 {
            h.call_sym(THROW, Conv::Cdecl, &[E_INVALIDARG]);
            return 0;
        }
        let d = word(h, this);
        let shared = 1_u32.wrapping_sub(word(h, d.wrapping_sub(4)));
        let short = word(h, d.wrapping_sub(8)).wrapping_sub(need);
        if (shared | short).cast_signed() < 0 {
            method(h, PREPARE_WRITE2, this, &[need]);
        }
        let data = word(h, this);
        let end = data.wrapping_add(2 * length);
        let mut p = data;
        while p < end {
            let mut m = import(h, WCSSTR, &[p, old]);
            while m != 0 {
                let before = (m.wrapping_sub(data).cast_signed() >> 1).cast_unsigned();
                let tail = length.wrapping_sub(before).wrapping_sub(old_len);
                p = m.wrapping_add(2 * new_len);
                let e = import(
                    h,
                    MEMMOVE_S,
                    &[p, 2 * tail, m.wrapping_add(2 * old_len), 2 * tail],
                );
                h.call_sym(ATL_CRT_ERROR_CHECK, Conv::Cdecl, &[e]);
                let e = import(h, MEMCPY_S, &[m, 2 * new_len, new, 2 * new_len]);
                h.call_sym(ATL_CRT_ERROR_CHECK, Conv::Cdecl, &[e]);
                length = length.wrapping_add(new_len.wrapping_sub(old_len));
                h.write(m.wrapping_add(2 * tail.wrapping_add(new_len)), 0_u16);
                m = import(h, WCSSTR, &[p, old]);
            }
            p = p.wrapping_add(2 * wcslen_or_0(h, p)).wrapping_add(2);
        }
        let d = word(h, this);
        if result.cast_signed() < 0
            || result.cast_signed() > word(h, d.wrapping_sub(8)).cast_signed()
        {
            h.call_sym(THROW, Conv::Cdecl, &[E_INVALIDARG]);
            return 0;
        }
        h.write(d.wrapping_sub(0xc), result);
        h.write(d.wrapping_add(2 * result), 0_u16);
        count
    }

    /// The in-order successor of a `std::map` node (`+0xd` set on the
    /// head/nil node; left `+0`, parent `+4`, right `+8`).
    fn tree_next(h: &impl Mem, node: u32) -> u32 {
        let nil = |at: u32| h.read::<u8>(at.wrapping_add(0xd)) != 0;
        if nil(node) {
            return node;
        }
        let right = word(h, node.wrapping_add(8));
        if !nil(right) {
            let mut n = right;
            while !nil(word(h, n)) {
                n = word(h, n);
            }
            return n;
        }
        let mut n = node;
        let mut p = word(h, n.wrapping_add(4));
        while !nil(p) && n == word(h, p.wrapping_add(8)) {
            n = p;
            p = word(h, p.wrapping_add(4));
        }
        p
    }

    /// `vector<u32>::push_back(value)` as the original inlines it, the
    /// value held at `slot` (a frame local: the aliasing branch, a value
    /// inside the vector itself, can't happen for it but is kept). Growth:
    /// half again, at least one more (`VECTOR_REALLOC`); past 0x3fffffff
    /// `_Xlength_error`.
    fn push_back(h: &mut impl Host, v: u32, slot: u32, value: u32) {
        let end = word(h, v.wrapping_add(4));
        let aliased = slot < end && word(h, v) <= slot;
        let idx = (slot.wrapping_sub(word(h, v)).cast_signed() >> 2).cast_unsigned();
        let cap = word(h, v.wrapping_add(8));
        if end == cap {
            let begin = word(h, v);
            let size = (end.wrapping_sub(begin).cast_signed() >> 2).cast_unsigned();
            if 0x3fff_ffff_u32.wrapping_sub(size) < 1 {
                let s = h.rebase(VECTOR_TOO_LONG);
                import(h, XLENGTH_ERROR, &[s]);
                return;
            }
            let want = size + 1;
            let room = (cap.wrapping_sub(begin).cast_signed() >> 2).cast_unsigned();
            let half = room >> 1;
            let grow = if 0x3fff_ffff_u32.wrapping_sub(half) < room {
                0
            } else {
                room.wrapping_add(half)
            };
            let grow = if grow < want { want } else { grow };
            method(h, VECTOR_REALLOC, v, &[grow]);
        }
        let end = word(h, v.wrapping_add(4));
        if end != 0 {
            let x = if aliased {
                word(h, word(h, v).wrapping_add(idx.wrapping_mul(4)))
            } else {
                value
            };
            h.write(end, x);
        }
        h.write(v.wrapping_add(4), end.wrapping_add(4));
    }

    /// `FUN_0065da80(this; name, out)` (`ret 8`) -> the size of `out`
    /// (the frame at `frame`, the original's `ebp - 0x6c`: the texture
    /// iterator `+0`, the cluster vector `+0x30`, the pushed value `+0x40`,
    /// the key string `+0x44`). For every entry of the scene's cluster map
    /// (`+0x11dc`, in order) whose key contains `name` (`strstr` on a
    /// copy), each cluster of its list (`[+0x28] + 4`, copied) has its
    /// first `PTexture2D` looked up; the found ones' objects go to the
    /// `vector<PTexture2D*>` `out`. The original's SEH frame only frees
    /// the copies if `_Xlength_error` or an allocation throws.
    pub fn collect_textures(h: &mut impl Host, frame: u32, this: u32, name: u32, out: u32) -> u32 {
        let iter = frame;
        let clusters = frame + 0x30;
        let slot = frame + 0x40;
        let key = frame + 0x44;
        let mut node = word(h, word(h, this.wrapping_add(0x11dc)));
        while node != word(h, this.wrapping_add(0x11dc)) {
            h.write(key + 0x14, 0xf_u32);
            h.write(key + 0x10, 0_u32);
            h.write(key, 0_u8);
            method(
                h,
                STRING_ASSIGN_SUBSTR,
                key,
                &[node.wrapping_add(0x10), 0, u32::MAX],
            );
            let text = if word(h, key + 0x14) < 0x10 {
                key
            } else {
                word(h, key)
            };
            if import(h, STRSTR, &[text, name]) != 0 {
                let list = word(h, node.wrapping_add(0x28)).wrapping_add(4);
                method(h, CLUSTER_VECTOR_COPY, clusters, &[list]);
                let mut begin = word(h, clusters);
                let n =
                    (word(h, clusters + 4).wrapping_sub(begin).cast_signed() >> 2).cast_unsigned();
                let mut i = 0_u32;
                while i < n {
                    let head = word(h, begin.wrapping_add(4 * i)).wrapping_add(0x1c);
                    let first = word(h, head);
                    let first = if first == head { 0 } else { first };
                    let class = h.rebase(CLASS_TEXTURE_2D);
                    h.write(iter, [0_u32, 0, 0, 0, 0, head, first, class]);
                    method(h, ITER_SEEK, iter, &[]);
                    if word(h, iter + 0xc) != 0 {
                        let value = word(h, iter + 0x10).wrapping_add(word(h, iter));
                        h.write(slot, value);
                        push_back(h, out, slot, value);
                    }
                    begin = word(h, clusters);
                    i += 1;
                }
                if begin != 0 {
                    h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[begin]);
                    h.write(clusters, [0_u32; 3]);
                }
            }
            if word(h, key + 0x14) >= 0x10 {
                let p = word(h, key);
                h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[p]);
            }
            node = tree_next(h, node);
        }
        (word(h, out.wrapping_add(4))
            .wrapping_sub(word(h, out))
            .cast_signed()
            >> 2)
            .cast_unsigned()
    }

    /// `ShadowDynamicMesh_assign_00678310(this; src)` (`ret 4`) -> `this`:
    /// the copy assignment. `+4` and `+0x18` copied, `+8` and `+0x10`
    /// through their own assignments, the 0x68 floats `+0x1c..+0x1bc`
    /// copied as `fld`/`fstp` copy them (a signalling NaN comes out
    /// quiet) and the 12 words `+0x1bc..+0x1ec` as they are. (The
    /// original's `rep lods` over its first 7 words only reads.)
    pub fn shadow_assign(h: &mut impl Host, this: u32, src: u32) -> u32 {
        let v = word(h, src + 4);
        h.write(this + 4, v);
        method(h, SHADOW_ASSIGN_8, this + 8, &[src + 8]);
        method(h, SHADOW_ASSIGN_10, this + 0x10, &[src + 0x10]);
        let v = word(h, src + 0x18);
        h.write(this + 0x18, v);
        for off in (0x1c..0x1bc).step_by(4) {
            let v = quiet(word(h, src + off));
            h.write(this + off, v);
        }
        for off in (0x1bc..0x1ec).step_by(4) {
            let v = word(h, src + off);
            h.write(this + off, v);
        }
        this
    }

    /// `GradMesh_setGradient_00678e60(this; a, mode, b, c)` (`ret 0x10`;
    /// `a`, `b`, `c` floats): a quad strip of six vertices from -1 to the
    /// split `2 l - 1` (`+0x1c`, x/y/z/w with z 0.5) whose colour ramp
    /// (`+0xfc`, 24 floats) and the split texture row (`+0x17c`) take
    /// the stops of `mode`: 1 `l = (a-b)/(c-b)`, the last stop
    /// `1 - (c-a)/(1-a)`; 3 `l = (1-b)/(c-b)` and the first stop
    /// `1 - (b-a)/(1-a)`, the rest 0; 2 the last two stops
    /// `1 - (b-a)/(1-a)`, `1 - (c-a)/(1-a)`; else all 1 and `l = 0`.
    /// The 12 indices (`+0x1bc`) are two quads. The split, rounded to
    /// f32, is stored in the original's own `a` slot first.
    pub fn grad_setup(h: &mut impl Host, this: u32, a: u32, mode: u32, b: u32, c: u32) {
        use ffx_bridge::x87;
        let (a, b, c) = (x87::load(a), x87::load(b), x87::load(c));
        let one = 1.0_f32.to_bits();
        let zero = 0_u32;
        let ramp = |x: f64, y: f64, z: f64| {
            // 1 - (x - y) / (z - y), the ramp stop between y and z
            x87::store(x87::sub(1.0, x87::div(x87::sub(x, y), x87::sub(z, y))))
        };
        let (l, s1, s2, s3, s4) = match mode {
            1 => (
                x87::store(x87::div(x87::sub(a, b), x87::sub(c, b))),
                one,
                one,
                one,
                ramp(c, a, 1.0),
            ),
            3 => (
                x87::store(x87::div(x87::sub(1.0, b), x87::sub(c, b))),
                ramp(b, a, 1.0),
                zero,
                zero,
                zero,
            ),
            2 => (zero, one, one, ramp(b, a, 1.0), ramp(c, a, 1.0)),
            _ => (zero, one, one, one, one),
        };
        let m1 = quiet(h.read(h.rebase(GRAD_MINUS_ONE)));
        let k = quiet(h.read(h.rebase(GRAD_HALF)));
        let lx = x87::load(l);
        let split = x87::store(x87::sub(x87::add(lx, lx), 1.0));
        let p = [
            m1, m1, k, one, m1, k, m1, split, k, one, split, k, m1, split, k, one, split, k, m1,
            one, k, one, one, k,
        ];
        h.write(this + 0x1c, p);
        let q = [
            zero, zero, one, zero, zero, l, one, l, zero, l, one, l, zero, one, one, one,
        ];
        h.write(this + 0x17c, q);
        let r = [
            one, s1, one, one, s1, one, one, s2, one, one, s2, one, one, s3, one, one, s3, one,
            one, s4, one, one, s4, one,
        ];
        h.write(this + 0xfc, r);
        h.write(this + 0x1bc, [0_u32, 1, 2, 3, 2, 1, 4, 5, 6, 7, 6, 5]);
    }

    /// `AbmapManager_CheckWordVertex(this; x, y, z)` (`ret 0xc`): the
    /// word mesh (`[[+0xa8] + 0x94]` when `+4` is negative, else
    /// `[[+0xc0] + 0x94]`) hides every quad whose vertex block (0x30 bytes
    /// each) has `x < [+0x18]`, `y > [+0x1c]` and `z < [+0x10]` (x, y, z
    /// rounded to f32; NaNs never match): its six indices (u16) are
    /// cleared in both index arrays. Two sub-meshes: count `+4`, vertices
    /// `+0xc`, indices `+0x1c`/`+0x88`; count `+0xdc`, vertices `+0xe4`,
    /// indices `+0xf4`/`+0x160`. The chain is read again for each store,
    /// as the original does. (The original also rounds the coordinates
    /// into its own `x` slot in the second pass.)
    pub fn check_word_vertex<H2: Host>(h: &mut H2, this: u32, x: u32, y: u32, z: u32) {
        let src = if word(h, this + 4).cast_signed() < 0 {
            0xa8
        } else {
            0xc0
        };
        let mesh = |h: &mut H2| word(h, word(h, this.wrapping_add(src)).wrapping_add(0x94));
        let fx = ffx_bridge::x87::load(ffx_bridge::x87::store(f64::from(x.cast_signed())));
        let fy = ffx_bridge::x87::load(ffx_bridge::x87::store(f64::from(y.cast_signed())));
        let fz = ffx_bridge::x87::load(ffx_bridge::x87::store(f64::from(z.cast_signed())));
        let f = |h: &H2, at: u32| f64::from(f32::from_bits(word(h, at)));
        for (count, verts, ia, ib) in [
            (4_u32, 0xc_u32, 0x1c_u32, 0x88_u32),
            (0xdc, 0xe4, 0xf4, 0x160),
        ] {
            let mut m = mesh(h);
            if word(h, m.wrapping_add(count)) >> 2 == 0 {
                continue;
            }
            let mut i = 0_u32;
            loop {
                let v = word(h, m.wrapping_add(verts)).wrapping_add(0x30 * i);
                let hit = f(h, v.wrapping_add(0x18)) > fx
                    && fy > f(h, v.wrapping_add(0x1c))
                    && fz < f(h, v.wrapping_add(0x10));
                if hit {
                    let mut first = true;
                    for k in 0..6_u32 {
                        for arr in [ia, ib] {
                            let mm = if first { m } else { mesh(h) };
                            first = false;
                            let p = word(h, mm.wrapping_add(arr));
                            h.write(p.wrapping_add(12 * i + 2 * k), 0_u16);
                        }
                    }
                }
                i += 1;
                m = mesh(h);
                if i >= word(h, m.wrapping_add(count)) >> 2 {
                    break;
                }
            }
        }
    }

    /// `PostProcessTweakUtil_initWithSceneContext(this; context)` (`ret 4`;
    /// the line buffer at `frame`, the original's `ebp - 0x44`, 64
    /// bytes): the text panel (`+8`) is initialised and `+0x178` set to
    /// `context + 0x10`. Without a context, or when the panel fails, the
    /// panel is disabled and `+4` cleared. Otherwise the fog far value
    /// (`+0x158`) steps by 50, the 22 value pointers (`+0xf0..+0x148`:
    /// the context's fog and colour fields, this object's own, the
    /// post-process manager's `+0x122a4..+0x122ac`) and labels
    /// (`+0x98..+0xec`) are set, the selection (`+0x17c`) reset, each line
    /// printed (`"%s: %.3f"`, `">"` on the selected one) and appended,
    /// `+4` set and the panel disabled.
    pub fn tweak_init(h: &mut impl Host, frame: u32, this: u32, context: u32) {
        let text = this + 8;
        let r = method(h, TEXT_INIT, text, &[]);
        h.write(this + 0x178, context.wrapping_add(0x10));
        if context == 0 || r != 0 {
            method(h, TEXT_SET_ENABLED, text, &[0]);
            h.write(this + 4, 0_u8);
            return;
        }
        let fog = ffx_bridge::x87::load(word(h, this + 0x158));
        let step = f64::from_bits(h.read::<u64>(h.rebase(TWEAK_FOG_STEP)));
        let fog = ffx_bridge::x87::store(ffx_bridge::x87::add(fog, step));
        h.write(this + 0x158, fog);
        let ctx = |off: u32| context.wrapping_add(off);
        let own = |off: u32| this + off;
        let values = [
            (0xf0, ctx(0x24)),
            (0xf4, ctx(0x20)),
            (0xf8, ctx(0x28)),
            (0xfc, ctx(0x2c)),
            (0x100, own(0x16c)),
            (0x104, own(0x170)),
            (0x108, own(0x174)),
            (0x10c, own(0x148)),
            (0x110, ctx(0x80)),
            (0x114, ctx(0x84)),
            (0x118, ctx(0x88)),
            (0x11c, ctx(0x8c)),
            (0x120, own(0x14c)),
            (0x128, own(0x154)),
            (0x12c, own(0x158)),
        ];
        for (off, v) in values {
            h.write(this + off, v);
        }
        for (off, label) in TWEAK_LABELS {
            let l = h.rebase(label);
            h.write(this + off, l);
        }
        for (off, delta) in [(0x130, 0x1_22ac_u32), (0x134, 0x1_22a8), (0x138, 0x1_22a4)] {
            let v = POST_PROCESS_MANAGER.get(h).wrapping_add(delta);
            h.write(this + off, v);
        }
        for (off, v) in [
            (0x13c, own(0x160)),
            (0x140, own(0x15c)),
            (0x144, own(0x164)),
            (0x124, own(0x150)),
        ] {
            h.write(this + off, v);
        }
        h.write(this + 0x17c, 0_u32);
        for k in 0..22_u32 {
            h.call_sym(MEMSET, Conv::Cdecl, &[frame, 0, 0x40]);
            let value = word(h, this + 0xf0 + 4 * k);
            let v = ffx_bridge::x87::load(word(h, value)).to_bits();
            let label = word(h, this + 0x98 + 4 * k);
            let format = if k == word(h, this + 0x17c) {
                TWEAK_LINE_SELECTED
            } else {
                TWEAK_LINE
            };
            let format = h.rebase(format);
            import(
                h,
                SPRINTF_S,
                &[frame, 0x40, format, label, v as u32, (v >> 32) as u32],
            );
            method(h, TEXT_APPEND_LINE, text, &[frame]);
        }
        h.write(this + 4, 1_u8);
        method(h, TEXT_SET_ENABLED, text, &[0]);
    }

    /// A pointer with its low tag bit cleared.
    fn untag(p: u32) -> u32 {
        p.wrapping_sub(p & 1)
    }

    /// `TextUtil_rebuildMutableString(this)`: the string builder (`+0x14`)
    /// restarts (`+0x1c` = 0) from the title (`+0x7c`), then each
    /// non-empty line of the `+0x78` array (count `+0x74 & 0x7fffffff`)
    /// follows after a newline; `+0x70` (dirty) cleared. (Pointers
    /// carry a tag in bit 0.)
    pub fn text_rebuild(h: &mut impl Host, this: u32) {
        h.write(this + 0x1c, 0_u32);
        let title = untag(word(h, this + 0x7c));
        method(h, STRING_BUILDER_APPEND, this + 0x14, &[title]);
        let n = word(h, this + 0x74) & 0x7fff_ffff;
        for i in 0..n {
            let line = untag(word(h, word(h, this + 0x78).wrapping_add(4 * i)));
            if line != 0 && h.read::<u8>(line) != 0 {
                let nl = h.rebase(NEWLINE);
                method(h, STRING_BUILDER_APPEND, this + 0x14, &[nl]);
                let line = untag(word(h, word(h, this + 0x78).wrapping_add(4 * i)));
                method(h, STRING_BUILDER_APPEND, this + 0x14, &[line]);
            }
        }
        h.write(this + 0x70, 0_u8);
    }

    /// `TextUtil_render(this; renderer)` (`ret 4`) -> 0 or the first
    /// error: when enabled (`+0x80`) with a font text (`+0x10`), the text
    /// is rebuilt if dirty (`+0x70`), set (the builder's string `+0x18`
    /// when `+0x1c` is set, else none) and drawn in its own scene.
    pub fn text_render(h: &mut impl Host, this: u32, renderer: u32) -> u32 {
        if h.read::<u8>(this + 0x80) == 0 || word(h, this + 0x10) == 0 {
            return 0;
        }
        if h.read::<u8>(this + 0x70) != 0 {
            method(h, TEXT_REBUILD, this, &[]);
        }
        let e = method(h, BEGIN_SCENE, renderer, &[2]);
        if e != 0 {
            return e;
        }
        let text = if word(h, this + 0x1c) == 0 {
            0
        } else {
            word(h, this + 0x18)
        };
        let font = word(h, this + 0x10);
        let e = method(h, FONT_SET_TEXT, font, &[text]);
        if e != 0 {
            return e;
        }
        let e = method(h, RENDERER_PUSH_COMMAND20, renderer, &[0]);
        if e != 0 {
            return e;
        }
        let font = word(h, this + 0x10);
        let e = method(h, FONT_RENDER_INSTANCE, font, &[renderer]);
        if e != 0 {
            return e;
        }
        method(h, END_SCENE, renderer, &[1])
    }

    /// `PostProcessTweakUtil_render(this; renderer, fps)` (`ret 8`, `fps` a
    /// float; the title buffer at `frame`, the original's `ebp - 0x24`, 32
    /// bytes) -> 0, or the panel's render result: when shown (`+0x180`)
    /// and given a renderer, the title `"PostProcess: %.2f fps"` is set
    /// on the panel (`+8`) and the panel drawn.
    pub fn tweak_render(h: &mut impl Host, frame: u32, this: u32, renderer: u32, fps: u32) -> u32 {
        if h.read::<u8>(this + 0x180) == 0 || renderer == 0 {
            return 0;
        }
        let v = ffx_bridge::x87::load(fps).to_bits();
        h.write(frame, [0_u32; 8]);
        let title = h.rebase(TWEAK_TITLE);
        import(
            h,
            SPRINTF_S,
            &[frame, 0x20, title, v as u32, (v >> 32) as u32],
        );
        method(h, TEXT_SET_TITLE, this + 8, &[frame]);
        method(h, TEXT_RENDER, this + 8, &[renderer])
    }

    /// `PostProcessManager_renderDebug4By3Frame(this; renderer)` (`ret 4`)
    /// -> the first error, else `endScene`'s result: the 4:3 frame mesh
    /// (`+0x123ac`) drawn to the back buffer in the debug frame pass.
    pub fn debug_4by3(h: &mut impl Host, this: u32, renderer: u32) -> u32 {
        let pass = h.rebase(DEBUG_FRAME_PASS);
        h.write(renderer + 0x48, pass);
        let e = method(h, SET_COLOR_TARGET, renderer, &[0, 0]);
        if e != 0 {
            return e;
        }
        let e = method(h, BEGIN_SCENE, renderer, &[0]);
        if e != 0 {
            return e;
        }
        let list = word(h, renderer + 0x13c);
        let inst = word(h, this.wrapping_add(0x1_23ac));
        let mesh = word(h, inst + 0x10);
        let e = method(h, RENDER_MESH_INSTANCE, renderer, &[inst, mesh, 0, list]);
        if e != 0 {
            return e;
        }
        method(h, END_SCENE, renderer, &[1])
    }

    /// A render target's surface: `null` when its flag `+4` is set.
    fn target_surface(h: &impl Mem, t: u32) -> u32 {
        if h.read::<u8>(t.wrapping_add(4)) == 0 {
            word(h, t)
        } else {
            0
        }
    }

    /// The first element of a counted array (`& 0x7fffffff` above 1:
    /// out of line at `+4`, 1: inline at `+4`, 0: none).
    fn first_elem(h: &impl Mem, d: u32) -> u32 {
        let n = word(h, d) & 0x7fff_ffff;
        let p = match n {
            0 => 0,
            1 => d.wrapping_add(4),
            _ => word(h, d.wrapping_add(4)),
        };
        word(h, p)
    }

    /// `PostProcessManager_renderFocusFrame(this; renderer, unused)`
    /// (`ret 8`) -> 0 (the frame at `frame`, the original's `ebp - 0x50`:
    /// two render surfaces, the colour scale `+0x20` and the colour
    /// buffer's vector `+0x30`; the focus depth cell at `+0x58`, the
    /// original's own `renderer` slot). When the focus frame is on
    /// (`+0x123c5`), the colour target (`+0x8c`) is copied to the work
    /// buffer (`+0x1178`), then drawn back over itself with the depth
    /// target (`+0x90`) in the focus pass: colour buffer the work buffer,
    /// scale (1, 1, 1, 0.6), the frame mesh (`+0x1214`) once for each of
    /// the two focus depths (`+0x123c8`, `+0x123cc`) inside [-1, 1].
    pub fn focus_frame(h: &mut impl Host, frame: u32, this: u32, renderer: u32) -> u32 {
        if h.read::<u8>(this.wrapping_add(0x1_23c5)) == 0 {
            return 0;
        }
        let (surf_color, surf_depth) = (frame, frame + 0x10);
        let (scale, vector, cell) = (frame + 0x20, frame + 0x30, frame + 0x58);
        let src = target_surface(h, word(h, this + 0x8c));
        let work = word(h, this + 0x1178);
        method(h, COPY_BUFFER, this, &[renderer, src, work, 1, 1, 0]);
        let t = word(h, this + 0x8c);
        method(h, SURFACE_CTOR, surf_color, &[t, 0]);
        let t = word(h, this + 0x90);
        method(h, SURFACE_CTOR, surf_depth, &[t, 0]);
        method(h, SET_COLOR_TARGET, renderer, &[surf_color, 0]);
        method(h, SET_DEPTH_STENCIL_TARGET, renderer, &[surf_depth]);
        method(h, BEGIN_SCENE, renderer, &[0]);
        let pass = h.rebase(FOCUS_PASS);
        h.write(renderer + 0x48, pass);
        let params = word(h, word(h, this + 0x1218));
        let name = h.rebase(P_COLOR_BUFFER);
        let color_buffer = method(h, FIND_PARAM, params, &[name]);
        let name = h.rebase(P_COLOR_SCALE);
        let color_scale = method(h, FIND_PARAM, params, &[name]);
        let name = h.rebase(P_FOCUS_DEPTH);
        let focus_depth = method(h, FIND_PARAM, params, &[name]);
        let work = target_surface(h, word(h, this + 0x1178));
        h.write(vector, [0xff, 0, this.wrapping_add(0x1_23dc), work]);
        let one = 1.0_f32.to_bits();
        let alpha = quiet(h.read(h.rebase(FOCUS_ALPHA)));
        h.write(scale, [one, one, one, alpha]);
        let buffer = word(h, word(h, this + 0x1218) + 4);
        method(h, WRITE_VECTOR_PARAM, buffer, &[color_buffer, vector]);
        let buffer = word(h, word(h, this + 0x1218) + 4);
        method(h, SHADER_SET_VALUE, buffer, &[color_scale, scale]);
        let lo = f64::from(f32::from_bits(h.read(h.rebase(MINUS_ONE))));
        for off in [0x1_23c8_u32, 0x1_23cc] {
            let depth = quiet(word(h, this.wrapping_add(off)));
            h.write(cell, depth);
            let d = f64::from(f32::from_bits(depth));
            if lo <= d && d <= 1.0 {
                let buffer = word(h, word(h, this + 0x1218) + 4);
                method(h, SHADER_SET_VALUE2, buffer, &[focus_depth, cell]);
                let list = word(h, renderer + 0x13c);
                let inst = word(h, this + 0x1214);
                let mesh = word(h, inst + 0x10);
                method(h, RENDER_MESH_INSTANCE, renderer, &[inst, mesh, 0, list]);
            }
        }
        method(h, END_SCENE, renderer, &[1]);
        method(h, SURFACE_DTOR, surf_depth, &[]);
        method(h, SURFACE_DTOR, surf_color, &[]);
        0
    }

    /// `PostProcessManager_resolveDepth(this; renderer, texture, target)`
    /// (`ret 0xc`) -> 0 or the first error (the frame at `frame`, the
    /// original's `ebp - 0x34`: a render surface, the parameter vector at
    /// `+0x10`). The depth mesh (`+0x1208`)'s material gives the
    /// parameters and buffer; by the depth target's sample count
    /// (`[+0x90] + 5`: 2, 4 or 8, else nothing and 0) its
    /// `"DepthBuffer<n>"` takes the texture, `"UvScaleBias"` (1, 1, 0,
    /// 0); the target (or the back buffer) is bound, colour slot 1 and
    /// depth cleared, the viewport set to the target's size (or the
    /// application's), and the mesh drawn in the sample count's pass.
    pub fn resolve_depth(
        h: &mut impl Host,
        frame: u32,
        this: u32,
        renderer: u32,
        texture: u32,
        target: u32,
    ) -> u32 {
        let (surf, vector) = (frame, frame + 0x10);
        let d = word(h, word(h, this + 0x1208) + 0x10);
        let params = word(h, first_elem(h, d));
        let buffer = word(h, first_elem(h, d) + 4);
        let kind = h.read::<u8>(word(h, this + 0x90) + 5);
        let Some(&(_, name, pass)) = P_DEPTH_BUFFER.iter().find(|e| e.0 == kind) else {
            return 0;
        };
        let name = h.rebase(name);
        let p = method(h, FIND_PARAM, params, &[name]);
        if p != 0 {
            h.write(vector, [0xff, 0, this.wrapping_add(0x1_23dc), texture]);
            let e = method(h, WRITE_VECTOR_PARAM, buffer, &[p, vector]);
            if e != 0 {
                return e;
            }
        }
        let name = h.rebase(P_UV_SCALE_BIAS);
        let p = method(h, FIND_PARAM, params, &[name]);
        if p != 0 {
            let one = 1.0_f32.to_bits();
            h.write(vector, [one, one, 0, 0]);
            method(h, SHADER_SET_VALUE, buffer, &[p, vector]);
        }
        if target == 0 {
            let e = method(h, SET_COLOR_TARGET, renderer, &[0, 0]);
            if e != 0 {
                return e;
            }
        } else {
            method(h, SURFACE_CTOR, surf, &[target, 0]);
            let e = method(h, SET_COLOR_TARGET, renderer, &[surf, 0]);
            method(h, SURFACE_DTOR, surf, &[]);
            if e != 0 {
                return e;
            }
        }
        let e = method(h, SET_COLOR_TARGET, renderer, &[0, 1]);
        if e != 0 {
            return e;
        }
        let e = method(h, SET_DEPTH_STENCIL_TARGET, renderer, &[0]);
        if e != 0 {
            return e;
        }
        let (w, ht) = if target == 0 {
            let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
            let ht = word(h, app + 0x270);
            let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
            (word(h, app + 0x26c), ht)
        } else {
            let s = target_surface(h, target);
            (word(h, s.wrapping_add(0x1c)), word(h, s.wrapping_add(0x20)))
        };
        method(h, SET_VIEWPORT, renderer, &[0, 0, w, ht]);
        let e = method(h, BEGIN_SCENE, renderer, &[0]);
        if e != 0 {
            return e;
        }
        let list = word(h, renderer + 0x13c);
        let pass = h.rebase(pass);
        h.write(renderer + 0x48, pass);
        let inst = word(h, this + 0x1208);
        let mesh = word(h, inst + 0x10);
        method(h, RENDER_MESH_INSTANCE, renderer, &[inst, mesh, 0, list]);
        method(h, END_SCENE, renderer, &[1])
    }

    /// `CharacterCloth_addInstance_006a1320(this; desc)` (`ret 4`; the
    /// pushed pointer's cell at `cell`, the original's own `desc` slot)
    /// -> `PArray_PushInto`'s `eax`. When any of the description's cloth
    /// entries (`[desc]`: count `& 0x7fffffff`, 0x6c-byte entries at
    /// `[+4]`) has both `+0x20` and `+0x24` set (not -1), a cloth
    /// instancing object (0x14 bytes, `Clear4`) is made and attached to
    /// `desc`; that object (or null) is pushed onto the array at `+8`,
    /// grown to its count + 1 words (`PhyreMalloc(.., 4)`).
    pub fn cloth_add_instance(h: &mut impl Host, cell: u32, this: u32, desc: u32) -> u32 {
        let arr = word(h, desc);
        let n = word(h, arr) & 0x7fff_ffff;
        let mut cloth = false;
        let mut p = word(h, arr.wrapping_add(4)).wrapping_add(0x24);
        for _ in 0..n {
            cloth = cloth || (word(h, p.wrapping_sub(4)) != u32::MAX && word(h, p) != u32::MAX);
            p = p.wrapping_add(0x6c);
        }
        h.write(cell, 0_u32);
        if cloth {
            let o = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x14]);
            h.write(cell, o);
            let o = if o == 0 { 0 } else { method(h, CLEAR4, o, &[]) };
            h.write(cell, o);
            method(h, CLOTH_ATTACH, o, &[desc]);
        }
        let count = word(h, this + 8) & 0x7fff_ffff;
        let m = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[(count + 1).wrapping_mul(4), 4]);
        method(h, PARRAY_PUSH_INTO, this + 8, &[m, cell])
    }

    /// `DebugTweakUtil_ctor_006b52a0(this)` -> `this`: the panel's words
    /// `+0..+0x18` (1, 1, 1, 0, 1, 0, 1), `+0x1c..+0x30` cleared, the text
    /// panel (`+0x34`) constructed with its scale 0.02, a 1280x720 area
    /// and colour (0, 0, 0, 1), the 0x34 bytes at `+0xc4` cleared.
    pub fn debug_tweak_ctor(h: &mut impl Host, this: u32) -> u32 {
        h.write(this + 0x1c, [0_u32; 3]);
        h.write(this + 0x28, 0_u8);
        method(h, TEXT_UTIL_CTOR, this + 0x34, &[]);
        let scale = quiet(h.read(h.rebase(TWEAK_TEXT_SCALE)));
        h.write(this + 0x104, scale);
        h.write(this + 0xf8, [0_u32; 2]);
        h.write(this + 0x100, 0_u8);
        h.write(this + 0x108, [0x500_u32, 0x2d0, 0, 0, 0, 1.0_f32.to_bits()]);
        h.write(this, [1_u32, 1, 1, 0, 1, 0, 1]);
        h.write(this + 0x30, 0_u32);
        h.write(this + 0x2c, 0_u32);
        h.call_sym(MEMSET, Conv::Cdecl, &[this + 0xc4, 0, 0x34]);
        this
    }

    /// `DebugTextTweakUtil_ctor_006b5440(this)` -> `this`: the text panel
    /// (`+0x114`, its `+0x110` flag cleared) constructed with its scale
    /// 0.02, a 1280x720 area and colour (0, 0, 0, 1); `+0` 1, `+4`,
    /// `+0x108`, `+0x10c` and `+0x1a4..+0x1bc` cleared, the name buffer
    /// (`+8`, 256 bytes) set to `"test"`.
    pub fn debug_text_tweak_ctor(h: &mut impl Host, this: u32) -> u32 {
        h.write(this + 0x110, 0_u8);
        method(h, TEXT_UTIL_CTOR, this + 0x114, &[]);
        let scale = quiet(h.read(h.rebase(TWEAK_TEXT_SCALE)));
        h.write(this + 0x1c8, scale);
        h.write(this + 0x1bc, [0_u32; 2]);
        h.write(this + 0x1c4, 0_u8);
        h.write(this + 0x1cc, [0x500_u32, 0x2d0, 0, 0, 0, 1.0_f32.to_bits()]);
        h.write(this + 4, 0_u32);
        h.write(this, 1_u32);
        h.write(this + 0x108, [0_u32; 2]);
        let (f, s) = (h.rebase(FORMAT_S), h.rebase(TEXT_TEST));
        import(h, SPRINTF_S, &[this + 8, 0x100, f, s]);
        h.write(this + 0x1a4, [0_u32; 6]);
        this
    }

    /// `ViewerManager_DrawTexAnimInfo_006bc210(this; time)` (`ret 4`, `time`
    /// a float; the frame at `frame`, the original's `ebp - 0x214`: the
    /// two float pairs `+0`, `+8`, the name buffers `+0x10` and
    /// `+0x110`, 256 bytes each). On a `"grid00"` viewer map (`+0x30`)
    /// with a texture animation viewer, the animation advances
    /// (virtual 1), its group and animation names (tagged strings at
    /// `+0x20` and `+4`, the stubs `0x006b2050`/`0x006b2060`) are copied
    /// out and its pairs read; `Sg_PlayerSetAnimatedMaterials` takes the
    /// names, the frame count (`+0x18`, the stub `0x006b1f90`), the pairs
    /// (`+0xc`, `+0x10` of the first two words each), 1, 1 and `+0x1c`.
    pub fn draw_texanim_info<H3: Host>(h: &mut H3, frame: u32, this: u32, time: u32) {
        let map = h.rebase(GRID00);
        if import(h, STRSTR, &[this + 0x30, map]) == 0 {
            return;
        }
        let anim = word(h, TEXANIM_VIEWER.get(h).wrapping_add(0x18));
        if anim == 0 {
            return;
        }
        h.call_virtual(anim, 1, &[quiet(time)]);
        let (pair24, pair2c) = (frame, frame + 8);
        let (name_b, name_a) = (frame + 0x10, frame + 0x110);
        let f = h.rebase(FORMAT_S);
        let group = untag(word(h, anim.wrapping_add(0x20)));
        import(h, SPRINTF_S, &[name_a, 0x100, f, group]);
        let id = untag(word(h, anim.wrapping_add(4)));
        import(h, SPRINTF_S, &[name_b, 0x100, f, id]);
        method(h, TEXANIM_PAIR_2C, anim, &[pair2c]);
        method(h, TEXANIM_PAIR_24, anim, &[pair24]);
        let count = word(h, anim.wrapping_add(0x18));
        let one = 1.0_f32.to_bits();
        let p = |h: &H3, at: u32| quiet(word(h, at));
        let (a0, a1, b0, b1) = (
            p(h, pair2c),
            p(h, pair2c + 4),
            p(h, pair24),
            p(h, pair24 + 4),
        );
        h.call_sym(
            SET_ANIMATED_MATERIALS,
            Conv::Cdecl,
            &[name_a, name_b, count, a0, a1, b0, b1, one, one, this + 0x1c],
        );
    }

    /// `AutoTestManager_pushCommand_006bd160(vector; s)` (`ret 4`):
    /// `vector<std::string>::push_back(s)` (0x18-byte strings), `s` taken
    /// by its index when it lies inside the vector (the growth moves it).
    pub fn push_command(h: &mut impl Host, v: u32, s: u32) {
        let end = word(h, v + 4);
        let src = if s < end && word(h, v) <= s {
            let idx = s.wrapping_sub(word(h, v)).cast_signed() / 0x18;
            if end == word(h, v + 8) {
                method(h, STRING_VECTOR_GROW, v, &[1]);
            }
            let e = word(h, v + 4);
            if e != 0 {
                Some((e, word(h, v).wrapping_add((idx * 0x18).cast_unsigned())))
            } else {
                None
            }
        } else {
            if end == word(h, v + 8) {
                method(h, STRING_VECTOR_GROW, v, &[1]);
            }
            let e = word(h, v + 4);
            if e != 0 { Some((e, s)) } else { None }
        };
        if let Some((e, from)) = src {
            method(h, STRING_COPY_CTOR, e, &[from]);
        }
        let e = word(h, v + 4);
        h.write(v + 4, e.wrapping_add(0x18));
    }

    /// `AutoTestManager_queueCommand_006bccb0(this; text)` (`ret 4`; the
    /// string at `frame`, the original's `ebp - 0x28`): `text` copied into
    /// a `std::string` and pushed onto the command queue (`+0x118`).
    pub fn queue_command(h: &mut impl Host, frame: u32, this: u32, text: u32) {
        h.write(frame + 0x14, 0xf_u32);
        h.write(frame + 0x10, 0_u32);
        h.write(frame, 0_u8);
        let mut n = 0_u32;
        while h.read::<u8>(text.wrapping_add(n)) != 0 {
            n += 1;
        }
        method(h, STRING_ASSIGN_N, frame, &[text, n]);
        method(h, AUTOTEST_PUSH, this + 0x118, &[frame]);
        if word(h, frame + 0x14) >= 0x10 {
            let p = word(h, frame);
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[p]);
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

    /// `AutoTestManager_execute_006bce00(this; command)` (`ret 4`; the
    /// argument buffer at `frame`, the original's `ebp - 0x104`, 256
    /// bytes) -> 1 when handled, else 0 (`al`). `"EnableAutoTest"` (at the
    /// start) resets the event list and sets `+0x129`,
    /// `"DisableAutoTest"` clears it; while enabled a command is printed
    /// and `"JumpMap <map>"` jumps (the viewer manager; `+0x128` set,
    /// `+0x114` the application time), `"Crash"` anywhere writes to
    /// address 0, `"SetBattle Enabled/Disabled"` sets the battle label
    /// lock to 0/1.
    pub fn autotest_execute(h: &mut impl Host, frame: u32, this: u32, command: u32) -> u8 {
        let s = h.rebase(AT_ENABLE);
        if import(h, STRSTR, &[command, s]) == command {
            h.call_sym(INIT_EVENT_LIST, Conv::Cdecl, &[]);
            h.write(this + 0x129, 1_u8);
            return 1;
        }
        let s = h.rebase(AT_DISABLE);
        if import(h, STRSTR, &[command, s]) == command {
            h.write(this + 0x129, 0_u8);
            return 1;
        }
        if h.read::<u8>(this + 0x129) == 0 {
            return 0;
        }
        let s = h.rebase(AT_EXECUTE);
        import(h, PRINTF, &[s, command]);
        let s = h.rebase(AT_JUMP_MAP);
        if import(h, SSCANF, &[command, s, frame]) == 1 {
            h.call_sym(FLAG_01326B28, Conv::Cdecl, &[]);
            let viewer = h.call_sym(VIEWER_MANAGER_INSTANCE, Conv::Cdecl, &[]);
            method(h, VIEWER_JUMP_MAP, viewer, &[frame]);
            h.write(this + 0x128, 1_u8);
            let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
            let t = h.call_sym_st0(GET_APP_TIME, Conv::Thiscall(app), &[]);
            h.write(this + 0x114, t.to_bits());
            return 1;
        }
        let s = h.rebase(AT_CRASH);
        if import(h, STRSTR, &[command, s]) != 0 {
            // the original's deliberate crash: `mov [0], 0`
            h.write(0, 0_u32);
            return 1;
        }
        let s = h.rebase(AT_SET_BATTLE);
        if import(h, SSCANF, &[command, s, frame]) != 1 {
            return 0;
        }
        let on = h.rebase(AT_ENABLED);
        if same_text(h, frame, on) {
            BTL_LABEL_LOCK.set(h, 0);
            return 1;
        }
        let off = h.rebase(AT_DISABLED);
        if same_text(h, frame, off) {
            BTL_LABEL_LOCK.set(h, 1);
            return 1;
        }
        0
    }

    /// `SpecularMapParamsManager_setCorrespondParams_006c5ec0(inst, rec)`
    /// (stdcall, `ret 8`; the frame from the original's `ebp - 0x88` at
    /// `frame`: the vector `+0`, the texture iterator `+0x10`, the list
    /// copy `+0x58`). An empty cluster name (`rec` starts with it) resets
    /// every material's `SpecularMapSampler` of a sampler type to
    /// `{0x42, 0, 0, 0}`. With a name and a texture in that cluster, on a
    /// copy of the mesh list each material gets, as each is found:
    /// `SpecularMapSamplerState` `{0x47, 0, linear sampler, 0}`,
    /// `SpecularMapSampler` the texture with the linear sampler,
    /// `SpecularUvOffset` (0, 0), `Specular_SpecialMat1` and
    /// `Specular_SpecialVector1` the bytes at `rec + 0x100` and `+0x150`,
    /// `Specular_SpecialVector2` the four floats at `rec + 0x140`,
    /// `Specular_SpecialFloat1` 1.0; the copy freed unless owned
    /// elsewhere. Parameter sizes past a local copy the original's frame:
    /// 0 in the port (`write_param`).
    pub fn set_correspond_params(h: &mut impl Host, frame: u32, inst: u32, rec: u32) {
        use crate::m5r_o::bridge::{
            copy_param, each_mesh_material, find, reset_specular_samplers, write_param,
        };
        use crate::m5r_o::{
            SPEC_FLOAT1, SPEC_MAT1, SPEC_SAMPLER, SPEC_SAMPLER_STATE, SPEC_UV_OFFSET, SPEC_VECTOR1,
            SPEC_VECTOR2,
        };
        if inst == 0 {
            return;
        }
        if h.read::<u8>(rec) == 0 {
            reset_specular_samplers(h, inst);
            return;
        }
        let vector: [u32; 4] = core::array::from_fn(|k| quiet(word(h, rec + 0x140 + 4 * k as u32)));
        h.write(frame, vector);
        let iter = frame + 0x10;
        let cm = h.call_sym(crate::m5r_o::CLUSTER_MANAGER, Conv::Cdecl, &[]);
        let cluster = h.call_sym(crate::m5r_o::CLUSTER_BY_NAME, Conv::Thiscall(cm), &[rec]);
        if cluster == 0 {
            return;
        }
        method(h, crate::m5r_o::TEXTURE_ITERATOR_CTOR, iter, &[cluster]);
        if word(h, iter + 0xc) == 0 {
            return;
        }
        let texture = word(h, iter).wrapping_add(word(h, iter + 0x10));
        let copy = frame + 0x58;
        method(h, crate::m5r_o::ARRAY_COPY_FROM, copy, &[inst]);
        let count = word(h, copy) & 0x7fff_ffff;
        let data = word(h, copy + 4);
        let vector = vector.map(u32::to_le_bytes).concat();
        let one = 1.0_f32.to_bits().to_le_bytes();
        each_mesh_material(h, count, data, |h, mat| {
            let params = word(h, mat);
            let p = find(h, params, SPEC_SAMPLER_STATE);
            if p != 0 {
                let buffer = word(h, mat.wrapping_add(4));
                let scene = crate::m5r_o::SCENE.get(h);
                let s = method(h, crate::m5r_o::LINEAR_SAMPLER, scene, &[1]);
                write_param(
                    h,
                    buffer,
                    p,
                    &[0x47_u32, 0, s, 0].map(u32::to_le_bytes).concat(),
                );
            }
            let p = find(h, params, SPEC_SAMPLER);
            if p != 0 {
                let value = frame + 0x20;
                h.write(value, 0x42_u32);
                h.write(value + 0xc, texture);
                let scene = crate::m5r_o::SCENE.get(h);
                let s = method(h, crate::m5r_o::LINEAR_SAMPLER, scene, &[1]);
                h.write(value + 8, s);
                let buffer = word(h, mat.wrapping_add(4));
                h.write(value + 4, 0_u32);
                method(h, crate::m5r_o::SET_TEXTURE_CHECKED, buffer, &[p, value]);
            }
            let p = find(h, params, SPEC_UV_OFFSET);
            if p != 0 {
                let buffer = word(h, mat.wrapping_add(4));
                write_param(h, buffer, p, &[0; 8]);
            }
            for (name, off) in [(SPEC_MAT1, 0x100_u32), (SPEC_VECTOR1, 0x150)] {
                let p = find(h, params, name);
                if p != 0 && rec.wrapping_add(off) != 0 {
                    let buffer = word(h, mat.wrapping_add(4));
                    copy_param(h, buffer, p, rec.wrapping_add(off));
                }
            }
            let p = find(h, params, SPEC_VECTOR2);
            if p != 0 {
                let buffer = word(h, mat.wrapping_add(4));
                write_param(h, buffer, p, &vector);
            }
            let p = find(h, params, SPEC_FLOAT1);
            if p != 0 {
                let buffer = word(h, mat.wrapping_add(4));
                write_param(h, buffer, p, &one);
            }
        });
        if word(h, copy) >> 31 == 0 {
            let data = word(h, copy + 4);
            h.call_sym(crate::m5r_o::PHYRE_FREE, Conv::Cdecl, &[data]);
        }
    }

    /// `SpecularMapParamsManager_renderCorrespondSpecular(this; only,
    /// renderer)` (`ret 8`): each of the four character slots (`+0`) not
    /// drawn yet (`+0x5b0`) — only `only` when given — is made visible,
    /// drawn in the z pre-pass for magic 0x144 and 0x272, then in its
    /// mode's specular pass (`+0x5a0`; modes 3..5 set the slot's
    /// parameters, `+0x10 + 0x164 i`, first), hidden again if it was
    /// hidden, and marked drawn; with `only` the first match ends it.
    pub fn render_correspond<H4: Host>(h: &mut H4, this: u32, only: u32, renderer: u32) {
        let draw_all = |h: &mut H4, inst: u32| {
            let mut k = 0_u32;
            while k < word(h, inst) & 0x7fff_ffff {
                let list = word(h, renderer + 0x13c);
                let m = word(h, word(h, inst + 4).wrapping_add(4 * k));
                let mesh = word(h, m.wrapping_add(0x10));
                method(h, RENDER_MESH_INSTANCE, renderer, &[m, mesh, 0, list]);
                k += 1;
            }
        };
        for i in 0..4_u32 {
            let inst = word(h, this + 4 * i);
            let done = this + 0x5b0 + i;
            if inst == 0 || h.read::<u8>(done) != 0 || (only != 0 && inst != only) {
                continue;
            }
            let visible = h.read::<u8>(inst + 0x14) != 0;
            h.call_sym(SET_VISIBLE, Conv::Cdecl, &[inst, 1]);
            if h.call_sym(MAGIC_ID, Conv::Cdecl, &[]) == 0x144
                || h.call_sym(MAGIC_ID, Conv::Cdecl, &[]) == 0x272
            {
                let p = h.rebase(PASS_Z_PRE);
                h.write(renderer + 0x48, p);
                draw_all(h, inst);
            }
            let mode = word(h, this + 0x5a0 + 4 * i);
            if let Some(&pass) = PASS_SPECULAR.get(mode as usize) {
                let p = h.rebase(pass);
                h.write(renderer + 0x48, p);
                if mode >= 3 {
                    h.call_sym(
                        SPECULAR_CORRESPOND,
                        Conv::Stdcall,
                        &[inst, this + 0x10 + 0x164 * i],
                    );
                }
            }
            draw_all(h, inst);
            if !visible {
                h.call_sym(SET_VISIBLE, Conv::Cdecl, &[inst, 0]);
            }
            h.write(done, 1_u8);
            if only != 0 {
                return;
            }
        }
    }
}
