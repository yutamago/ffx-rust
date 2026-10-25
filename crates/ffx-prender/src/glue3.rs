//! The render-side glue, third batch (M4 port round 1, list
//! `h3-glue-render`): screen-fade tasks, more blur setters, the scene's
//! small render-path predicates, and allocation helpers.
//!
//! Byte results (`u8`) are what the originals leave in `al`; their callers
//! (`PhyreScene_render`, `PhyreScene_processVFXDrawEvent`) test `al` only
//! (check-clobber).

use ffx_bridge::Global;

pub use crate::glue::SCENE;
pub use crate::glue2::{BLUR_EX_INSTANCE, MOTION_BLUR_INSTANCE};

/// `g_GraphicPauseGame`.
pub const PAUSE_GAME: Global<u32> = Global::new(0x00cc_cb00);

/// `PhyreScene_setMaterialColourScale2(this, 4 floats)` (leaves `eax`).
pub const SCENE_SET_MATERIAL_COLOUR_SCALE2: u32 = 0x0066_d1d0;
/// `BlurEffectEx_SetPositiveParams(this, 4 floats)` (never writes `eax`).
pub const BLUR_SET_POSITIVE_PARAMS: u32 = 0x006c_4620;
/// `BlurEffect_SetFloats74(this, 4 floats)` (never writes `eax`).
pub const BLUR_SET_FLOATS74: u32 = 0x006c_46e0;
/// `ScreenFadeDelegations_registerDelegation(4 floats, e, flags)` (cdecl).
pub const SCREEN_FADE_REGISTER: u32 = 0x006a_e650;
/// `PhyreScene_fieldMapLoad(this, entry, on)`.
pub const SCENE_FIELD_MAP_LOAD: u32 = 0x0065_cbf0;
/// `operator_delete_00630540(p)` (cdecl).
pub const OPERATOR_DELETE_TASK: u32 = 0x0063_0540;
/// `AsyncLoadManager_sGetInstance`.
pub const ASYNC_LOAD_INSTANCE: u32 = 0x0069_1b40;
/// The update `SetFields12400_00693be0` ends with (`0x00693d20`, thiscall).
pub const FIELDS_12400_UPDATE: u32 = 0x0069_3d20;
/// The cluster manager's entry allocation (`0x0069a1e0`, thiscall).
pub const CLUSTER_ALLOC_ENTRY: u32 = 0x0069_a1e0;
/// `MagicFile_GetMagicId()` (reads no `ecx`).
pub const MAGIC_ID: u32 = 0x009d_a3b0;
/// `Check_01326b28_008729e0()` (cdecl).
pub const CHECK_01326B28: u32 = 0x0087_29e0;
/// `operator_new(n)` (cdecl).
pub const OPERATOR_NEW: u32 = 0x0063_0310;
/// The pause-game object's constructor (`0x006bca90`, thiscall).
pub const PAUSE_GAME_CTOR: u32 = 0x006b_ca90;
/// `std::_Xbad_alloc`'s import slot (does not return).
pub const XBAD_ALLOC: u32 = 0x00b0_c2a0;
/// `TextUtil_resize(this, a, b)`.
pub const TEXT_RESIZE: u32 = 0x006f_6670;
/// The character instantiation's lookup (`0x0063c540`, cdecl `(obj, x)`).
pub const CHARACTER_LOOKUP: u32 = 0x0063_c540;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use crate::glue::bridge::{instance, on_global};
    use ffx_bridge::x87::round_trip;
    use ffx_graphic::host::{Conv, Host};

    /// Four floats (bits) through `fld`/`fstp`.
    fn rt4(f: [u32; 4]) -> [u32; 4] {
        f.map(|x| round_trip(f32::from_bits(x)).to_bits())
    }

    /// `graphicSetMaterialColourScale2(4 floats)` `0x00643670` (the
    /// setter leaves `eax`; no caller reads it).
    pub fn set_material_colour_scale2(h: &mut impl Host, f: [u32; 4]) {
        on_global(h, SCENE, SCENE_SET_MATERIAL_COLOUR_SCALE2, &rt4(f));
    }
    /// The blur effect's (`[instance]`) four-float `setter`; returns the
    /// instance, which the setter leaves in `eax`.
    fn blur_set4(h: &mut impl Host, setter: u32, f: [u32; 4]) -> u32 {
        let ex = instance(h, BLUR_EX_INSTANCE);
        let blur: u32 = h.read(ex);
        h.call_sym(setter, Conv::Thiscall(blur), &rt4(f));
        ex
    }
    /// `graphicBlurEffectSetPositiveParams_00643830(4 floats)`.
    pub fn blur_set_positive_params(h: &mut impl Host, f: [u32; 4]) -> u32 {
        blur_set4(h, BLUR_SET_POSITIVE_PARAMS, f)
    }
    /// `graphicBlurEffectSetFloats74_00643860(4 floats)`.
    pub fn blur_set_floats74(h: &mut impl Host, f: [u32; 4]) -> u32 {
        blur_set4(h, BLUR_SET_FLOATS74, f)
    }
    /// `graphicAddScreenFadeTask(4 floats, e)` `0x00639240` (`flags`
    /// `0x1001`) and `graphicAddScreenFadeTaskAfterRender` `0x00639280`
    /// (`0x801`).
    pub fn add_screen_fade_task(h: &mut impl Host, f: [u32; 4], e: u32, flags: u32) -> u32 {
        let [a, b, c, d] = rt4(f);
        h.call_sym(SCREEN_FADE_REGISTER, Conv::Cdecl, &[a, b, c, d, e, flags])
    }
    /// `PhyreScene_asyncDoFieldMapLoadTask(task)` `0x00657820`: `task =
    /// (scene, entry, c)`: `fieldMapLoad(entry, c != 0)`, then `operator
    /// delete(entry)`.
    pub fn async_field_map_load_task(h: &mut impl Host, task: u32) -> u32 {
        instance(h, ASYNC_LOAD_INSTANCE);
        let scene: u32 = h.read(task);
        let entry: u32 = h.read(task + 4);
        let c: u32 = h.read(task + 8);
        h.call_sym(
            SCENE_FIELD_MAP_LOAD,
            Conv::Thiscall(scene),
            &[entry, u32::from(c != 0)],
        );
        h.call_sym(OPERATOR_DELETE_TASK, Conv::Cdecl, &[entry])
    }
    /// `SetFields12400_00693be0(this, a, b, c, d)` (`ret 0x10`, `d`
    /// unused): `this[0] = c`, `+0x12400 = a`, `+0x12404 = b`, then the
    /// update.
    pub fn set_fields_12400(h: &mut impl Host, this: u32, a: u32, b: u32, c: u32) -> u32 {
        h.write(this, c);
        h.write(this + 0x12400, a);
        h.write(this + 0x12404, b);
        h.call_sym(FIELDS_12400_UPDATE, Conv::Thiscall(this), &[])
    }
    /// `ClusterManager_newEntry_00699300(this, key)` (`ret 4`): a new
    /// entry, its word `+0xc` = 0 and the two words at `key` copied to
    /// `+0x10` (unless that address is 0); returns the entry.
    pub fn cluster_new_entry(h: &mut impl Host, this: u32, key: u32) -> u32 {
        let entry = h.call_sym(CLUSTER_ALLOC_ENTRY, Conv::Thiscall(this), &[]);
        h.write(entry + 0xc, 0u16);
        let at = entry.wrapping_add(0x10);
        if at != 0 {
            let lo: u32 = h.read(key);
            h.write(at, lo);
            let hi: u32 = h.read(key + 4);
            h.write(at + 4, hi);
        }
        entry
    }
    /// `PhyreScene_isMotionBlurOn_006601e0(this)`: 1 when byte `+1` is
    /// above 4 (unsigned); else 1 when the post-process manager
    /// (`+0x10cc`) has byte `+0x123bb` and the motion blur its `+0x58`.
    pub fn scene_is_motion_blur_on(h: &mut impl Host, this: u32) -> u8 {
        if h.read::<u8>(this + 1) > 4 {
            return 1;
        }
        let post: u32 = h.read(this + 0x10cc);
        if h.read::<u8>(post + 0x0001_23bb) == 0 {
            return 0;
        }
        let mb = instance(h, MOTION_BLUR_INSTANCE);
        u8::from(h.read::<u8>(mb + 0x58) != 0)
    }
    /// `PhyreScene_check1195_006662f0(this)`: without byte `+0x1195`,
    /// whether `Check_01326b28` gives 0; else byte `+0x1196` != 0.
    pub fn scene_check_1195(h: &mut impl Host, this: u32) -> u8 {
        if h.read::<u8>(this + 0x1195) == 0 {
            let r = h.call_sym(CHECK_01326B28, Conv::Cdecl, &[]);
            return u8::from(r == 0);
        }
        u8::from(h.read::<u8>(this + 0x1196) != 0)
    }
    /// `PhyreScene_isGlowAfterVFX(this)` `0x0065efa0`: 0 for magic
    /// `0x2b9`; else whether `+0xfb60` or `+0xfb74` is above 0 (signed).
    pub fn scene_is_glow_after_vfx(h: &mut impl Host, this: u32) -> u8 {
        if h.call_sym(MAGIC_ID, Conv::Thiscall(this), &[]) == 0x2b9 {
            return 0;
        }
        u8::from(h.read::<i32>(this + 0xfb60) > 0 || h.read::<i32>(this + 0xfb74) > 0)
    }
    /// `graphicCreatePauseGame_006bcc40()`: creates `g_GraphicPauseGame`
    /// (0x12c bytes) when there is none. The original returns the new
    /// object (or 0), and leaves its caller's `eax` when one exists; its
    /// only caller (`graphicInitialize`) reads neither.
    pub fn create_pause_game(h: &mut impl Host) {
        if PAUSE_GAME.get(h) != 0 {
            return;
        }
        let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x12c]);
        let obj = if p == 0 {
            0
        } else {
            h.call_sym(PAUSE_GAME_CTOR, Conv::Thiscall(p), &[])
        };
        PAUSE_GAME.set(h, obj);
    }
    /// `std_Tree_BuyHeadNode_00654ed0()`: a `0x2c`-byte tree head node
    /// linked to itself (parent, left, right) with colour and "nil" 1;
    /// `std::_Xbad_alloc` when `operator new` fails.
    pub fn tree_buy_head_node(h: &mut impl Host) -> u32 {
        let node = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x2c]);
        if node == 0 {
            let at = h.rebase(XBAD_ALLOC);
            let f: u32 = h.read(at);
            return h.call(f, Conv::Cdecl, &[]);
        }
        h.write(node, node);
        h.write(node + 4, node);
        h.write(node + 8, node);
        h.write(node + 0xc, 0x0101u16);
        node
    }
    /// `PhyreScene_resizeTarget_00698060(this, a, b)` (`ret 8`): stores
    /// `a`, `b` at `+0x188`; with byte `+0x180`, `TextUtil_resize(this + 8,
    /// a, b)`'s result, else `a`.
    pub fn scene_resize_target(h: &mut impl Host, this: u32, a: u32, b: u32) -> u32 {
        h.write(this + 0x188, a);
        h.write(this + 0x18c, b);
        if h.read::<u8>(this + 0x180) == 0 {
            return a;
        }
        h.call_sym(TEXT_RESIZE, Conv::Thiscall(this + 8), &[a, b])
    }
    /// `graphicCharacterRegistHelper_00637880(out, obj, x, callback)`:
    /// when `obj +4` is set, `*out = lookup(obj, x)` and `callback(x)` if
    /// given. Returns what the original leaves in `eax`: `obj`, the 0
    /// callback, or the callback's result.
    pub fn character_regist_helper(
        h: &mut impl Host,
        out: u32,
        obj: u32,
        x: u32,
        callback: u32,
    ) -> u32 {
        if h.read::<u32>(obj + 4) == 0 {
            return obj;
        }
        let r = h.call_sym(CHARACTER_LOOKUP, Conv::Cdecl, &[obj, x]);
        h.write(out, r);
        if callback == 0 {
            return 0;
        }
        h.call(callback, Conv::Cdecl, &[x])
    }
}
