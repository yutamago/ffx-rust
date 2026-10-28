//! `ffx_prender::m5m_*`: M5 stage 1, list `map` (the debug tweak panels,
//! the free camera's attach, scene and `std::string` helpers and the
//! field-map code the static call graph reaches and no coverage run ran).
//!
//! Threads: the main thread (`PhyreScene_bootInit`, `PhyreScene_render`,
//! the debug menu, the unwind funclets of the main thread's frames); the
//! shims keep no state.

use super::{StackFrame, mem, ported};
use ffx_prender::m5m_a::{self as ma, bridge as a};

ported! {
    // ---- batch 1 (ffx_prender::m5m_a) ----
    /// `SectionVector_pushBack_006ba110(v; &item)`.
    SectionVector_pushBack_006ba110 @ 0x006b_a110 => extern "thiscall" fn section_push_back(this: u32, item: u32) {
        a::section_push_back(&mut mem(), this, item);
    }
    /// `BasicDebugTweakUtil_setEnabled(t; on)`: the strings, the line and
    /// the three numbers in the shim's frame (`ebp - 0x198..`).
    BasicDebugTweakUtil_setEnabled @ 0x006b_af60 => extern "thiscall" fn basic_set_enabled(this: u32, on: u32) -> u32 {
        let mut f = StackFrame::<0x68>::new();
        a::basic_set_enabled(&mut mem(), f.at() + 0x1a0, this, on)
    }
    /// `CharacterAnimationTweakUtil_setEnabled(t; on)`.
    CharacterAnimationTweakUtil_setEnabled @ 0x006b_b5f0 => extern "thiscall" fn char_anim_set_enabled(this: u32, on: u32) -> u32 {
        a::char_anim_set_enabled(&mut mem(), this, on)
    }
    /// `CharacterSwitchTweakUtil_setEnabled_006bb670(t; on)`.
    CharacterSwitchTweakUtil_setEnabled_006bb670 @ 0x006b_b670 => extern "thiscall" fn char_switch_set_enabled(this: u32, on: u32) -> u32 {
        a::set_enabled(&mut mem(), ma::CHAR_SWITCH_ENABLE, this, on)
    }
    /// `CharacterTextureAnimationTweakUtil_setEnabled_006bb6d0(t; on)`.
    CharacterTextureAnimationTweakUtil_setEnabled_006bb6d0 @ 0x006b_b6d0 => extern "thiscall" fn char_tex_set_enabled(this: u32, on: u32) -> u32 {
        a::char_tex_set_enabled(&mut mem(), this, on)
    }
    /// `MapSwitchTweakUtil_setEnabled_006bb880(t; on)`.
    MapSwitchTweakUtil_setEnabled_006bb880 @ 0x006b_b880 => extern "thiscall" fn map_switch_set_enabled(this: u32, on: u32) -> u32 {
        a::set_enabled(&mut mem(), ma::MAP_SWITCH_ENABLE, this, on)
    }
    /// `InlineBuffer_free_0064e9a0(b)`.
    InlineBuffer_free_0064e9a0 @ 0x0064_e9a0 => extern "thiscall" fn inline_buffer_free(this: u32) -> u32 {
        a::inline_buffer_free(&mut mem(), this)
    }
    /// `AtlCString_release_0064e9e0(s)`.
    AtlCString_release_0064e9e0 @ 0x0064_e9e0 => extern "thiscall" fn cstring_release(this: u32) -> u32 {
        a::cstring_release(&mut mem(), this)
    }
    /// `std_string_tidy_0064ea00(s)`.
    std_string_tidy_0064ea00 @ 0x0064_ea00 => extern "thiscall" fn string_tidy(this: u32) {
        a::string_tidy(&mut mem(), this);
    }
    /// `std_vector_tidy_0064ea30(v)`.
    std_vector_tidy_0064ea30 @ 0x0064_ea30 => extern "thiscall" fn vector_tidy(this: u32) -> u32 {
        a::vector_tidy(&mut mem(), this)
    }
    /// `PhyreSceneCursor_postNext4_0064f1a0(c; out, _)` -> out.
    PhyreSceneCursor_postNext4_0064f1a0 @ 0x0064_f1a0 => extern "thiscall" fn cursor_post_next4(this: u32, out: u32, _unused: u32) -> u32 {
        a::cursor_post_next4(&mut mem(), this, out)
    }
    /// `std_string_less_0064f550(a, b)`.
    std_string_less_0064f550 @ 0x0064_f550 => extern "stdcall" fn string_less(x: u32, y: u32) -> u32 {
        a::string_less(&mut mem(), x, y)
    }
    /// `std_string_append_cstr_0064f5e0(s; p)`.
    std_string_append_cstr_0064f5e0 @ 0x0064_f5e0 => extern "thiscall" fn string_append_cstr(this: u32, p: u32) -> u32 {
        a::string_append_cstr(&mut mem(), this, p)
    }
    /// `FreeCamera_attach_0067fda0(cam; target)`: the matrices in the
    /// shim's frame (`ebp - 0x80..`).
    FreeCamera_attach_0067fda0 @ 0x0067_fda0 => extern "thiscall" fn free_camera_attach(this: u32, target: u32) -> u32 {
        let mut f = StackFrame::<0x20>::new();
        a::free_camera_attach(&mut mem(), f.at() + 0x80, this, target)
    }
    /// `BasicDebugTweakUtil_initialize(t)`: the line in the shim's frame.
    BasicDebugTweakUtil_initialize @ 0x006b_9360 => extern "thiscall" fn basic_init(this: u32) -> u32 {
        let mut f = StackFrame::<0x41>::new();
        a::init_lines(&mut mem(), ma::BASIC_INIT, f.at() + 0x104, this)
    }
    /// `BtlChrParamConfigTweakUitl_initialize(t)`.
    BtlChrParamConfigTweakUitl_initialize @ 0x006b_94a0 => extern "thiscall" fn btl_chr_param_init(this: u32) -> u32 {
        let mut f = StackFrame::<0x41>::new();
        a::init_lines(&mut mem(), ma::BTL_CHR_PARAM_INIT, f.at() + 0x104, this)
    }
    /// `BtlDebugTweakUtil_initialize(t)`.
    BtlDebugTweakUtil_initialize @ 0x006b_9600 => extern "thiscall" fn btl_debug_init(this: u32) -> u32 {
        let mut f = StackFrame::<0x41>::new();
        a::btl_debug_init(&mut mem(), f.at() + 0x104, this)
    }
    /// `CharacterAnimationTweakUtil_initialize(t)`.
    CharacterAnimationTweakUtil_initialize @ 0x006b_97f0 => extern "thiscall" fn char_anim_init(this: u32) -> u32 {
        let mut f = StackFrame::<0xb>::new();
        a::char_anim_init(&mut mem(), f.at() + 0x2c, this)
    }
    /// `CharacterSwitchTweakUtil_initialize(t)`.
    CharacterSwitchTweakUtil_initialize @ 0x006b_9970 => extern "thiscall" fn char_switch_init(this: u32) -> u32 {
        let mut f = StackFrame::<6>::new();
        a::char_switch_init(&mut mem(), f.at() + 0x18, this)
    }
    /// `CharacterTextureAnimationTweakUtil_initialize(t)`.
    CharacterTextureAnimationTweakUtil_initialize @ 0x006b_9aa0 => extern "thiscall" fn char_tex_init(this: u32) -> u32 {
        let mut f = StackFrame::<0xe>::new();
        a::char_tex_init(&mut mem(), f.at() + 0x38, this)
    }
    /// `GameParamConfigTweakUitl_initialize(t)`.
    GameParamConfigTweakUitl_initialize @ 0x006b_9cd0 => extern "thiscall" fn game_param_init(this: u32) -> u32 {
        let mut f = StackFrame::<0x41>::new();
        a::init_lines(&mut mem(), ma::GAME_PARAM_INIT, f.at() + 0x104, this)
    }
    /// `BasicDebugTweakUtil_render(t; ctx, fps)`.
    BasicDebugTweakUtil_render @ 0x006b_a1a0 => extern "thiscall" fn basic_render(this: u32, ctx: u32, fps: u32) -> u32 {
        let mut f = StackFrame::<0x41>::new();
        a::render(&mut mem(), ma::BASIC_RENDER, f.at() + 0x104, this, ctx, fps)
    }
    /// `BtlChrParamConfigTweakUitl_render(t; ctx, fps)`.
    BtlChrParamConfigTweakUitl_render @ 0x006b_a240 => extern "thiscall" fn btl_chr_param_render(this: u32, ctx: u32, fps: u32) -> u32 {
        let mut f = StackFrame::<0x41>::new();
        a::render(&mut mem(), ma::BTL_CHR_PARAM_RENDER, f.at() + 0x104, this, ctx, fps)
    }
    /// `BtlDebugTweakUtil_render(t; ctx, fps)`.
    BtlDebugTweakUtil_render @ 0x006b_a2e0 => extern "thiscall" fn btl_debug_render(this: u32, ctx: u32, fps: u32) -> u32 {
        let mut f = StackFrame::<0x41>::new();
        a::render(&mut mem(), ma::BTL_DEBUG_RENDER, f.at() + 0x104, this, ctx, fps)
    }
    /// `CharacterAnimationTweakUtil_render(t; ctx, fps)`.
    CharacterAnimationTweakUtil_render @ 0x006b_a390 => extern "thiscall" fn char_anim_render(this: u32, ctx: u32, fps: u32) -> u32 {
        let mut f = StackFrame::<0x41>::new();
        a::render(&mut mem(), ma::CHAR_ANIM_RENDER, f.at() + 0x104, this, ctx, fps)
    }
    /// `CharacterSwitchTweakUtil_render(t; ctx, fps)`.
    CharacterSwitchTweakUtil_render @ 0x006b_a430 => extern "thiscall" fn char_switch_render(this: u32, ctx: u32, fps: u32) -> u32 {
        let mut f = StackFrame::<9>::new();
        a::render(&mut mem(), ma::CHAR_SWITCH_RENDER, f.at() + 0x24, this, ctx, fps)
    }
    /// `CharacterTextureAnimationTweakUtil_render(t; ctx, fps)`.
    CharacterTextureAnimationTweakUtil_render @ 0x006b_a4d0 => extern "thiscall" fn char_tex_render(this: u32, ctx: u32, fps: u32) -> u32 {
        let mut f = StackFrame::<0x41>::new();
        a::render(&mut mem(), ma::CHAR_TEX_RENDER, f.at() + 0x104, this, ctx, fps)
    }
    /// `GameParamConfigTweakUitl_render(t; ctx, fps)`.
    GameParamConfigTweakUitl_render @ 0x006b_a570 => extern "thiscall" fn game_param_render(this: u32, ctx: u32, fps: u32) -> u32 {
        let mut f = StackFrame::<0x41>::new();
        a::render(&mut mem(), ma::GAME_PARAM_RENDER, f.at() + 0x104, this, ctx, fps)
    }
    /// `MapSwitchTweakUtil_render(t; ctx, fps)`.
    MapSwitchTweakUtil_render @ 0x006b_a610 => extern "thiscall" fn map_switch_render(this: u32, ctx: u32, fps: u32) -> u32 {
        let mut f = StackFrame::<0x41>::new();
        a::render(&mut mem(), ma::MAP_SWITCH_RENDER, f.at() + 0x104, this, ctx, fps)
    }
    /// `rcbgGuideMapSave(a, loc)`: writes its second argument slot.
    rcbgGuideMapSave @ 0x006e_9570 => extern "C" fn guide_map_save(x: u32, loc: u32; slots: u32) -> u32 {
        a::guide_map_save(&mut mem(), slots, x, loc)
    }
}
