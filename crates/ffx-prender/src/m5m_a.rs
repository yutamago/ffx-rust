//! M5 stage 1, list `map`, batch 1: the debug tweak panels the static call
//! graph reaches through `PhyreScene_bootInit`, `PhyreScene_render` and
//! `DebugMenuManager_disableTweaks_006b49c0` and no coverage run ran (their
//! `initialize`, `render` and `setEnabled` methods, the event-section list
//! `BasicDebugTweakUtil_setEnabled` reads), the free camera's attach, a
//! scene cursor's post-increment, three MSVC `std::string` helpers, four
//! unwind funclets' cleanups and the guide-map save thunk.
//!
//! The panels' C++ objects are only reached through their fields; their
//! text panel (`TextUtil`) and line formatters stay calls to the originals.
//! Stack cookies and SEH states are left out: nothing on these paths
//! throws. Where the original hands a callee the address of a frame word
//! (text buffers, `std::string`s), the bridge takes the original's `ebp`
//! and keeps the word at its offset (the shim passes a `StackFrame`).

use ffx_bridge::Global;

/// `g_ViewerManager` (`ViewerManager_GetInstance`): `MapSwitchTweakUtil_render`
/// prints the string at `+0x51`, the texture-animation panel looks up the
/// one at `+0x1c`.
pub const VIEWER_MANAGER: Global<u32> = Global::new(0x00cc_cac0);
/// The table of motion-mode name pointers `CharacterAnimationTweakUtil_initialize`
/// indexes with the panel's `+0xc`.
pub const MOTION_MODE_NAMES: u32 = 0x00c3_4318;

// ---- strings ----

/// `"init failed"`.
pub const INIT_FAILED: u32 = 0x00b4_8244;
/// `">"` (the cursor's line) and `" "`.
pub const CURSOR: u32 = 0x00b4_8250;
pub const SPACE: u32 = 0x00b2_0b30;
/// `"%s%s%s"`, `"%s%s%d"`, `"%s%s"`, `"%s%s%03d"`.
pub const FMT_SSS: u32 = 0x00b4_2a3c;
pub const FMT_SSD: u32 = 0x00b4_88e0;
pub const FMT_SS: u32 = 0x00b4_0eb0;
pub const FMT_SS03D: u32 = 0x00b4_86d8;
/// `"true"`, `"false"`, `"on"`, `"off"`, `"N/A"`.
pub const TRUE: u32 = 0x00b3_9230;
pub const FALSE: u32 = 0x00b3_9238;
pub const ON: u32 = 0x00b4_88d8;
pub const OFF: u32 = 0x00b4_88dc;
pub const NA: u32 = 0x00b4_88e8;
/// `"/ffx/proj/event/Section_GoThrough.txt"`, `"\r\n"`, `","`.
pub const SECTION_FILE: u32 = 0x00b4_8a64;
pub const CRLF: u32 = 0x00b4_85d0;
pub const COMMA: u32 = 0x00b8_f884;

// ---- callees ----

/// `crt_memset_thunk` (cdecl).
pub const MEMSET: u32 = 0x0094_964c;
/// The `sprintf_s` import slot (cdecl, varargs).
pub const SPRINTF_S: u32 = 0x00b0_c394;
/// The `atoi` import slot (cdecl).
pub const ATOI: u32 = 0x00b0_c40c;
/// The `free` import slot (cdecl).
pub const FREE: u32 = 0x00b0_c370;
/// The `InterlockedExchangeAdd` import slot (stdcall, 2 words): the
/// original's `lock xadd`.
pub const INTERLOCKED_ADD: u32 = 0x00b0_c128;
/// `operator_delete` (cdecl).
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
/// `TextUtil_init(tu)` (thiscall, no words) -> 0 on success.
pub const TEXT_INIT: u32 = 0x006f_6370;
/// `TextUtil_appendLine(tu; text)` (`ret 4`).
pub const TEXT_APPEND: u32 = 0x006f_5e30;
/// `TextUtil_setEnabled(tu; on)` (`ret 4`).
pub const TEXT_SET_ENABLED: u32 = 0x006f_68f0;
/// `TextUtil_resize(tu; w, h)` (`ret 8`).
pub const TEXT_RESIZE: u32 = 0x006f_6670;
/// `TextUtil_changeLine(tu; i, text)` (`ret 8`).
pub const TEXT_CHANGE_LINE: u32 = 0x006f_6070;
/// `TextUtil_setTitleText(tu; text)` (`ret 4`).
pub const TEXT_SET_TITLE: u32 = 0x006f_6930;
/// `TextUtil_render(tu; ctx)` (`ret 4`).
pub const TEXT_RENDER: u32 = 0x006f_65b0;
/// The panels' line formatters: `(panel; i, out)` (`ret 8`).
pub const BASIC_LINE_TEXT: u32 = 0x006b_69a0;
pub const BTL_CHR_PARAM_LINE_TEXT: u32 = 0x006b_6bb0;
pub const GAME_PARAM_LINE_TEXT: u32 = 0x006b_72e0;
/// `Sg_PcRead(path)` (cdecl) -> the file's text or 0.
pub const SG_PC_READ: u32 = 0x0083_ae30;
/// `std_string_ctor_cstr(s; p)` (`ret 4`).
pub const STRING_CTOR_CSTR: u32 = 0x0040_2c90;
/// `std_string_findFirstOf_006b68c0(s; set, pos, n)` (`ret 0xc`) -> the
/// index or -1.
pub const STRING_FIND_FIRST_OF: u32 = 0x006b_68c0;
/// `std_string_substr_00402e20(s; out, pos, n)` (`ret 0xc`) -> out.
pub const STRING_SUBSTR: u32 = 0x0040_2e20;
/// `std_string_moveAssign_004039b0(s; from)` (`ret 4`).
pub const STRING_MOVE: u32 = 0x0040_39b0;
/// `std_string_append_ptr(s; p, n)` (`ret 8`).
pub const STRING_APPEND: u32 = 0x0065_76d0;
/// `std_string_compare_006590d0(s; pos, n, p, m)` (`ret 0x10`).
pub const STRING_COMPARE: u32 = 0x0065_90d0;
/// `SectionVector_grow_006b6500(v; n)` (`ret 4`).
pub const SECTION_VECTOR_GROW: u32 = 0x006b_6500;
/// `SectionVector_pushBack_006ba110(v; &item)` (`ret 4`).
pub const SECTION_VECTOR_PUSH: u32 = 0x006b_a110;
/// `CharacterAnimationTweakUtil_updateAnimation(t)` and
/// `CharacterAnimationTweakUtil_updateText_006bc750(t)` (thiscall).
pub const CHAR_ANIM_UPDATE: u32 = 0x006b_c130;
pub const CHAR_ANIM_UPDATE_TEXT: u32 = 0x006b_c750;
/// `isDebugMode()` -> `al`.
pub const IS_DEBUG_MODE: u32 = 0x0082_2550;
/// `enableGameControlTextureAnimation(on)` (cdecl).
pub const ENABLE_TEX_ANIM: u32 = 0x0083_6710;
/// `TextureAnimationManager_GetInstance()` (cdecl).
pub const TEX_ANIM_MANAGER: u32 = 0x006b_02e0;
/// `TextureAnimationManager_FindAnimation(mgr; name)` (`ret 4`).
pub const TEX_ANIM_FIND: u32 = 0x006b_1ef0;
/// `TextureAnimationInfoChar_active(info)`, `_deactive(info)` (thiscall).
pub const TEX_ANIM_ACTIVE: u32 = 0x006b_1c30;
pub const TEX_ANIM_DEACTIVE: u32 = 0x006b_04a0;
/// `TextureAnimation_SetByte1c(a; v)` (`ret 4`).
pub const TEX_ANIM_SET_BYTE_1C: u32 = 0x006b_3650;
/// `CharacterTextureAnimationTweakUtil_updateText_006bc850(t)` (thiscall).
pub const CHAR_TEX_UPDATE_TEXT: u32 = 0x006b_c850;
/// The texture-animation panel's getters (thiscall on its `+0x18`
/// animation): two `PString`s, a count and a flag (`al`).
pub const TEX_ANIM_NAME_A: u32 = 0x006b_2050;
pub const TEX_ANIM_NAME_B: u32 = 0x006b_2060;
pub const TEX_ANIM_COUNT: u32 = 0x006b_1f90;
pub const TEX_ANIM_FLAG: u32 = 0x006b_28e0;
/// `PhyreSceneCursor_Next4(cursor)` (thiscall).
pub const CURSOR_NEXT4: u32 = 0x0065_ee40;
/// `PMatrix4_toWorldMatrix3x4(dst; m)` and
/// `PMatrix4_FromMatrix3x4_00451080(dst; m)` (`ret 4`).
pub const TO_WORLD_3X4: u32 = 0x0045_1190;
pub const FROM_3X4: u32 = 0x0045_1080;
/// `ftol2_sse` is `ffx_bridge::x87::ftol`; `GuideMap_setLocationText_006e9870(a, loc)` (cdecl).
pub const GUIDE_SET_LOCATION_TEXT: u32 = 0x006e_9870;

/// A panel's `initialize` that fills each line with its formatter.
#[derive(Debug, Clone, Copy)]
pub struct Init {
    /// The `TextUtil`'s offset in the panel.
    pub text: u32,
    /// The "initialised" byte.
    pub flag: u32,
    /// `(offset, label)` stores, in the original's order.
    pub labels: &'static [(u32, u32)],
    /// The word cleared with `TextUtil_init`'s 0 (the cursor).
    pub cursor: u32,
    /// The line count.
    pub lines: u32,
    /// The line formatter.
    pub line_text: u32,
}

pub const BASIC_INIT: Init = Init {
    text: 0x34,
    flag: 0x28,
    labels: &[
        (0xc4, 0x00b4_8968),
        (0xcc, 0x00b4_8978),
        (0xc8, 0x00b4_8984),
        (0xd0, 0x00b4_89a0),
        (0xd4, 0x00b4_89b0),
        (0xd8, 0x00b4_89c4),
        (0xdc, 0x00b4_89d0),
        (0xe0, 0x00b4_89e0),
        (0xe4, 0x00b4_89f0),
        (0xe8, 0x00b4_8a10),
        (0xec, 0x00b4_8a20),
        (0xf0, 0x00b4_8a38),
        (0xf4, 0x00b4_8a4c),
    ],
    cursor: 0xfc,
    lines: 13,
    line_text: BASIC_LINE_TEXT,
};

pub const BTL_CHR_PARAM_INIT: Init = Init {
    text: 0x20,
    flag: 0x1c,
    labels: &[
        (0xb0, 0x00b4_8bac),
        (0xb4, 0x00b4_8bb8),
        (0xb8, 0x00b4_8bc8),
        (0xbc, 0x00b4_8bd8),
        (0xc0, 0x00b4_8be4),
        (0xc4, 0x00b4_8bf0),
        (0xc8, 0x00b4_8bf8),
        (0xcc, 0x00b4_8c00),
        (0xd0, 0x00b4_8c08),
        (0xd4, 0x00b4_8c14),
        (0xd8, 0x00b4_8c20),
        (0xdc, 0x00b4_8c2c),
        (0xe0, 0x00b4_8c3c),
        (0xe4, 0x00b4_8c48),
        (0xe8, 0x00b4_8c54),
        (0xec, 0x00b4_8c60),
        (0xf0, 0x00b4_8c6c),
    ],
    cursor: 0xf8,
    lines: 17,
    line_text: BTL_CHR_PARAM_LINE_TEXT,
};

pub const GAME_PARAM_INIT: Init = Init {
    text: 0x14,
    flag: 0x10,
    labels: &[
        (0xa4, 0x00b4_8cb8),
        (0xac, 0x00b4_8cc4),
        (0xb0, 0x00b4_8cd4),
        (0xb4, 0x00b4_8ce4),
        (0xa8, 0x00b4_8cfc),
        (0xb8, 0x00b4_8d08),
        (0xbc, 0x00b4_8d18),
        (0xc0, 0x00b4_8d28),
        (0xc4, 0x00b4_8d44),
        (0xc8, 0x00b4_8d60),
        (0xcc, 0x00b4_8d70),
        (0xd0, 0x00b4_8d7c),
        (0xd4, 0x00b4_8d90),
        (0xd8, 0x00b4_8da8),
        (0xdc, 0x00b4_8dc4),
    ],
    cursor: 0xe4,
    lines: 15,
    line_text: GAME_PARAM_LINE_TEXT,
};

/// A panel's `render(ctx, fps)`.
#[derive(Debug, Clone, Copy)]
pub struct Render {
    /// The "shown" byte.
    pub flag: u32,
    /// The `TextUtil`'s offset in the panel.
    pub text: u32,
    /// The title's format (`fps` as a double).
    pub format: u32,
    /// The title buffer: its `ebp` offset and size (`memset` or, for a
    /// small one, inline word stores).
    pub buf: u32,
    pub size: u32,
    /// `MapSwitchTweakUtil_render` also prints the viewer's map name.
    pub map_name: bool,
}

const fn render(flag: u32, text: u32, format: u32) -> Render {
    Render {
        flag,
        text,
        format,
        buf: 0x104,
        size: 0x100,
        map_name: false,
    }
}

pub const BASIC_RENDER: Render = render(0x100, 0x34, 0x00b4_8af0);
pub const BTL_CHR_PARAM_RENDER: Render = render(0xfc, 0x20, 0x00b4_8c8c);
pub const BTL_DEBUG_RENDER: Render = render(0x1c4, 0x114, 0x00b4_8b88);
pub const CHAR_ANIM_RENDER: Render = render(0xc0, 0x18, 0x00b4_8948);
pub const CHAR_SWITCH_RENDER: Render = Render {
    buf: 0x24,
    size: 0x20,
    ..render(0xd4, 0x4, 0x00b4_86e4)
};
pub const CHAR_TEX_RENDER: Render = render(0xd0, 0x20, 0x00b4_88ec);
pub const GAME_PARAM_RENDER: Render = render(0xe8, 0x14, 0x00b4_8e0c);
pub const MAP_SWITCH_RENDER: Render = Render {
    map_name: true,
    ..render(0x1f4, 0x4, 0x00b4_885c)
};

/// A `setEnabled(on)` that only shows the panel and resizes it.
#[derive(Debug, Clone, Copy)]
pub struct Enable {
    /// The "initialised" byte, the "shown" byte, the `TextUtil` and the
    /// size words.
    pub flag: u32,
    pub shown: u32,
    pub text: u32,
    pub w: u32,
}

/// `CharacterSwitchTweakUtil_setEnabled_006bb670`.
pub const CHAR_SWITCH_ENABLE: Enable = Enable {
    flag: 0,
    shown: 0xd4,
    text: 4,
    w: 0xdc,
};
/// `MapSwitchTweakUtil_setEnabled_006bb880`.
pub const MAP_SWITCH_ENABLE: Enable = Enable {
    flag: 0,
    shown: 0x1f4,
    text: 4,
    w: 0x1fc,
};

/// `x` as `fld dword` leaves it: a signalling NaN gets its quiet bit.
#[must_use]
pub fn widen(x: u32) -> f64 {
    let q = if x & 0x7f80_0000 == 0x7f80_0000 && x & 0x007f_ffff != 0 {
        x | 0x0040_0000
    } else {
        x
    };
    f64::from(f32::from_bits(q))
}

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::Mem;
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, f: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Cdecl, args)
    }

    fn method(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    fn word(h: &impl Mem, at: u32) -> u32 {
        h.read(at)
    }

    fn byte(h: &impl Mem, at: u32) -> u8 {
        h.read(at)
    }

    /// The import in slot `slot` (cdecl).
    fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f = word(h, h.rebase(slot));
        h.call(f, Conv::Cdecl, args)
    }

    fn sprintf(h: &mut impl Host, args: &[u32]) -> u32 {
        import(h, SPRINTF_S, args)
    }

    /// The cursor mark of line `i`: `">"` on the panel's cursor line.
    fn mark(h: &mut impl Host, cursor: u32, i: u32) -> u32 {
        if word(h, cursor) == i {
            h.rebase(CURSOR)
        } else {
            h.rebase(SPACE)
        }
    }

    /// The text of a `std::string` at `s` (MSVC: inline below 16 bytes of
    /// capacity).
    fn c_str(h: &impl Mem, s: u32) -> u32 {
        if word(h, s + 0x14) >= 0x10 {
            word(h, s)
        } else {
            s
        }
    }

    /// `SectionVector_pushBack_006ba110(v; &item)` (`ret 4`): a
    /// `std::vector` of 12-byte items (begin, end, capacity end); an item
    /// inside the vector is found again by index after the grow
    /// (`SectionVector_grow_006b6500(v; 1)` when full). The `eax` left is
    /// no caller's.
    pub fn section_push_back(h: &mut impl Host, v: u32, item: u32) {
        let end = word(h, v + 4);
        let begin = word(h, v);
        let src = if item < end && begin <= item {
            let index = item.wrapping_sub(begin).cast_signed() / 12;
            if end == word(h, v + 8) {
                method(h, SECTION_VECTOR_GROW, v, &[1]);
            }
            word(h, v).wrapping_add(index.cast_unsigned().wrapping_mul(12))
        } else {
            if end == word(h, v + 8) {
                method(h, SECTION_VECTOR_GROW, v, &[1]);
            }
            item
        };
        let dst = word(h, v + 4);
        if dst != 0 {
            for k in 0..3 {
                let x = word(h, src.wrapping_add(4 * k));
                h.write(dst.wrapping_add(4 * k), x);
            }
        }
        let end = word(h, v + 4);
        h.write(v + 4, end.wrapping_add(12));
    }

    /// `BasicDebugTweakUtil_setEnabled(t; on)` (`ret 4`; frame at `ebp`):
    /// shown = on and initialised (`+0x28`) into `+0x100` and the text
    /// panel (`+0x34`); when on: resized when shown, then the event
    /// sections reread: `Sg_PcRead("/ffx/proj/event/Section_GoThrough.txt")`,
    /// each `"\r\n"` line with a `','` split into three `atoi` numbers
    /// pushed onto `+0x1c`; line 9 reformatted when initialised. Returns
    /// the last callee's `eax`.
    pub fn basic_set_enabled(h: &mut impl Host, ebp: u32, t: u32, on: u32) -> u32 {
        let on = on & 0xff != 0;
        let shown = u32::from(on && byte(h, t + 0x28) != 0);
        h.write(t + 0x100, shown as u8);
        let tu = t + 0x34;
        let mut eax = method(h, TEXT_SET_ENABLED, tu, &[shown]);
        if !on {
            return eax;
        }
        if byte(h, t + 0x100) != 0 {
            let (w, hh) = (word(h, t + 0x108), word(h, t + 0x10c));
            method(h, TEXT_RESIZE, tu, &[w, hh]);
        }
        let path = h.rebase(SECTION_FILE);
        let text = cdecl(h, SG_PC_READ, &[path]);
        eax = text;
        if text != 0 {
            let file = ebp - 0x170;
            let line = ebp - 0x140;
            let part = ebp - 0x158;
            let field = ebp - 0x128;
            let unused = ebp - 0x194;
            let item = ebp - 0x17c;
            method(h, STRING_CTOR_CSTR, file, &[text]);
            for (s, at) in [(unused, 0x14_u32), (field, 0x14)] {
                h.write(s + at, 0xf_u32);
                h.write(s + 0x10, 0_u32);
                h.write(s, 0_u8);
            }
            let crlf = h.rebase(CRLF);
            let comma = h.rebase(COMMA);
            let mut start = 0_u32;
            let mut stop = method(h, STRING_FIND_FIRST_OF, file, &[crlf, start, 2]);
            eax = stop;
            while stop < word(h, file + 0x10) {
                method(h, STRING_SUBSTR, file, &[line, start, stop - start]);
                let mut comma_at = method(h, STRING_FIND_FIRST_OF, line, &[comma, 0, 1]);
                if comma_at < word(h, line + 0x10) {
                    let mut numbers = [0_u32; 3];
                    for (k, n) in numbers.iter_mut().enumerate() {
                        let (pos, len) = match k {
                            0 => (0, comma_at),
                            1 => {
                                let pos = comma_at + 1;
                                comma_at = method(h, STRING_FIND_FIRST_OF, line, &[comma, pos, 1]);
                                (pos, comma_at.wrapping_sub(pos))
                            }
                            _ => (
                                comma_at.wrapping_add(1),
                                word(h, line + 0x10).wrapping_sub(comma_at),
                            ),
                        };
                        let r = method(h, STRING_SUBSTR, line, &[part, pos, len]);
                        if r != field {
                            if word(h, field + 0x14) >= 0x10 {
                                let p = word(h, field);
                                cdecl(h, OPERATOR_DELETE, &[p]);
                            }
                            h.write(field + 0x14, 0xf_u32);
                            h.write(field + 0x10, 0_u32);
                            h.write(field, 0_u8);
                            method(h, STRING_MOVE, field, &[r]);
                        }
                        if word(h, part + 0x14) >= 0x10 {
                            let p = word(h, part);
                            cdecl(h, OPERATOR_DELETE, &[p]);
                        }
                        let p = c_str(h, field);
                        *n = import(h, ATOI, &[p]);
                        h.write(item + 4 * k as u32, *n);
                    }
                    method(h, SECTION_VECTOR_PUSH, t + 0x1c, &[item]);
                }
                start = stop.wrapping_add(2);
                stop = method(h, STRING_FIND_FIRST_OF, file, &[crlf, start, 2]);
                eax = stop;
                if word(h, line + 0x14) >= 0x10 {
                    let p = word(h, line);
                    eax = cdecl(h, OPERATOR_DELETE, &[p]);
                }
            }
            if word(h, field + 0x14) >= 0x10 {
                let p = word(h, field);
                eax = cdecl(h, OPERATOR_DELETE, &[p]);
            }
            h.write(field + 0x14, 0xf_u32);
            h.write(field + 0x10, 0_u32);
            h.write(field, 0_u8);
            if word(h, file + 0x14) >= 0x10 {
                let p = word(h, file);
                eax = cdecl(h, OPERATOR_DELETE, &[p]);
            }
        }
        if byte(h, t + 0x28) != 0 {
            let buf = ebp - 0x110;
            cdecl(h, MEMSET, &[buf, 0, 0x100]);
            method(h, BASIC_LINE_TEXT, t, &[9, buf]);
            eax = method(h, TEXT_CHANGE_LINE, tu, &[9, buf]);
        }
        eax
    }

    /// `CharacterAnimationTweakUtil_setEnabled(t; on)` (`ret 4`): shown =
    /// on and initialised (`+0x14`) into `+0xc0` and the text panel
    /// (`+0x18`); when on: `+0xc` and `+4` cleared, the animation and its
    /// text updated, resized when shown. Returns the last callee's `eax`.
    pub fn char_anim_set_enabled(h: &mut impl Host, t: u32, on: u32) -> u32 {
        let on = on & 0xff != 0;
        let shown = u32::from(on && byte(h, t + 0x14) != 0);
        h.write(t + 0xc0, shown as u8);
        let mut eax = method(h, TEXT_SET_ENABLED, t + 0x18, &[shown]);
        if on {
            h.write(t + 0xc, 0_u32);
            h.write(t + 4, 0_u32);
            method(h, CHAR_ANIM_UPDATE, t, &[]);
            eax = method(h, CHAR_ANIM_UPDATE_TEXT, t, &[]);
            if byte(h, t + 0xc0) != 0 {
                let (w, hh) = (word(h, t + 0xc8), word(h, t + 0xcc));
                eax = method(h, TEXT_RESIZE, t + 0x18, &[w, hh]);
            }
        }
        eax
    }

    /// The plain `setEnabled(t; on)` (`ret 4`): shown = on and initialised
    /// into its byte and the text panel; resized when on and shown.
    pub fn set_enabled(h: &mut impl Host, e: Enable, t: u32, on: u32) -> u32 {
        let on = on & 0xff != 0;
        let shown = u32::from(on && byte(h, t + e.flag) != 0);
        h.write(t + e.shown, shown as u8);
        let mut eax = method(h, TEXT_SET_ENABLED, t + e.text, &[shown]);
        if on && byte(h, t + e.shown) != 0 {
            let (w, hh) = (word(h, t + e.w), word(h, t + e.w + 4));
            eax = method(h, TEXT_RESIZE, t + e.text, &[w, hh]);
        }
        eax
    }

    /// `CharacterTextureAnimationTweakUtil_setEnabled_006bb6d0(t; on)`
    /// (`ret 4`): only in debug mode. Shown = on and initialised (`+0x1c`)
    /// into `+0xd0` and the text panel (`+0x20`); the game's texture
    /// animation control on, and with on: off again, resized when shown,
    /// the viewer's animation (`+0x1c` name) looked up into `+0x14`, each
    /// of its 0x40-byte infos activated, the first deactivated into
    /// `+0x18` with its byte `+0x1c` = 1; the text updated.
    pub fn char_tex_set_enabled(h: &mut impl Host, t: u32, on: u32) -> u32 {
        let mut eax = cdecl(h, IS_DEBUG_MODE, &[]);
        if eax.to_le_bytes()[0] == 0 {
            return eax;
        }
        let on = on & 0xff != 0;
        let shown = u32::from(on && byte(h, t + 0x1c) != 0);
        h.write(t + 0xd0, shown as u8);
        method(h, TEXT_SET_ENABLED, t + 0x20, &[shown]);
        eax = cdecl(h, ENABLE_TEX_ANIM, &[1]);
        if !on {
            return eax;
        }
        cdecl(h, ENABLE_TEX_ANIM, &[0]);
        if byte(h, t + 0xd0) != 0 {
            let (w, hh) = (word(h, t + 0xd8), word(h, t + 0xdc));
            method(h, TEXT_RESIZE, t + 0x20, &[w, hh]);
        }
        let name = VIEWER_MANAGER.get(h).wrapping_add(0x1c);
        let mgr = cdecl(h, TEX_ANIM_MANAGER, &[]);
        let anim = method(h, TEX_ANIM_FIND, mgr, &[name]);
        h.write(t + 0x14, anim);
        if anim != 0 && word(h, anim + 8) & 0x7fff_ffff != 0 {
            let mut k = 0_u32;
            loop {
                let anim = word(h, t + 0x14);
                let info = word(h, anim + 0xc).wrapping_add(0x40 * k);
                method(h, TEX_ANIM_ACTIVE, info, &[]);
                k += 1;
                let anim = word(h, t + 0x14);
                if k >= word(h, anim + 8) & 0x7fff_ffff {
                    break;
                }
            }
            let anim = word(h, t + 0x14);
            let first = word(h, anim + 0xc);
            h.write(t + 0x18, first);
            method(h, TEX_ANIM_DEACTIVE, first, &[]);
            let first = word(h, t + 0x18);
            method(h, TEX_ANIM_SET_BYTE_1C, first, &[1]);
        }
        method(h, CHAR_TEX_UPDATE_TEXT, t, &[])
    }

    /// The `initialize` of a panel whose lines come from its formatter
    /// (frame: the 0x100-byte line at `ebp - 0x104`): `TextUtil_init`;
    /// on failure the panel hidden and not initialised. Else initialised,
    /// the labels stored, the cursor 0, each line formatted and appended,
    /// the panel hidden. Returns the last callee's `eax`.
    pub fn init_lines(h: &mut impl Host, i: Init, ebp: u32, t: u32) -> u32 {
        let tu = t + i.text;
        let r = method(h, TEXT_INIT, tu, &[]);
        if r != 0 {
            let eax = method(h, TEXT_SET_ENABLED, tu, &[0]);
            h.write(t + i.flag, 0_u8);
            return eax;
        }
        h.write(t + i.flag, 1_u8);
        for &(off, label) in i.labels {
            let l = h.rebase(label);
            h.write(t + off, l);
        }
        h.write(t + i.cursor, r);
        let buf = ebp - 0x104;
        for line in 0..i.lines {
            cdecl(h, MEMSET, &[buf, 0, 0x100]);
            method(h, i.line_text, t, &[line, buf]);
            method(h, TEXT_APPEND, tu, &[buf]);
        }
        method(h, TEXT_SET_ENABLED, tu, &[0])
    }

    /// The failure path every `initialize` shares: hidden, not initialised.
    fn init_failed(h: &mut impl Host, tu: u32, flag: u32) -> u32 {
        let eax = method(h, TEXT_SET_ENABLED, tu, &[0]);
        h.write(flag, 0_u8);
        eax
    }

    fn labels(h: &mut impl Host, t: u32, labels: &[(u32, u32)]) {
        for &(off, label) in labels {
            let l = h.rebase(label);
            h.write(t + off, l);
        }
    }

    fn true_false(h: &mut impl Host, v: u32) -> u32 {
        h.rebase(if v != 0 { TRUE } else { FALSE })
    }

    fn on_off(h: &mut impl Host, v: bool) -> u32 {
        h.rebase(if v { ON } else { OFF })
    }

    /// `BtlDebugTweakUtil_initialize(t)` (frame: the 0x100-byte line at
    /// `ebp - 0x104`): six lines, the map name (`+8`), the two numbers
    /// (`+0`, `+4`) and three flags (`+0x10c`, `+0x1b4`'s label alone,
    /// `+0x108`), cursor `+0x1c0`; hidden after.
    pub fn btl_debug_init(h: &mut impl Host, ebp: u32, t: u32) -> u32 {
        let tu = t + 0x114;
        let r = method(h, TEXT_INIT, tu, &[]);
        if r != 0 {
            return init_failed(h, tu, t + 0x110);
        }
        h.write(t + 0x110, 1_u8);
        labels(
            h,
            t,
            &[
                (0x1a4, 0x00b4_8b1c),
                (0x1a8, 0x00b4_8b38),
                (0x1ac, 0x00b4_8b4c),
                (0x1b0, 0x00b4_8b60),
                (0x1b8, 0x00b4_8b74),
            ],
        );
        h.write(t + 0x1c0, r);
        let buf = ebp - 0x104;
        for i in 0..6 {
            cdecl(h, MEMSET, &[buf, 0, 0x100]);
            if byte(h, t + 0x110) == 0 {
                let f = h.rebase(INIT_FAILED);
                sprintf(h, &[buf, 0x100, f]);
            } else {
                let m = mark(h, t + 0x1c0, i);
                let sss = h.rebase(FMT_SSS);
                let ssd = h.rebase(FMT_SSD);
                match i {
                    0 => {
                        let l = word(h, t + 0x1a4);
                        sprintf(h, &[buf, 0x100, sss, m, l, t + 8]);
                    }
                    1 | 2 => {
                        let l = word(h, t + 0x1a4 + 4 * i);
                        let v = word(h, t + 4 * (i - 1));
                        sprintf(h, &[buf, 0x100, ssd, m, l, v]);
                    }
                    3 => {
                        let s = true_false(h, word(h, t + 0x10c));
                        let l = word(h, t + 0x1b0);
                        sprintf(h, &[buf, 0x100, sss, m, l, s]);
                    }
                    4 => {
                        let l = word(h, t + 0x1b4);
                        let ss = h.rebase(FMT_SS);
                        sprintf(h, &[buf, 0x100, ss, m, l]);
                    }
                    _ => {
                        let s = true_false(h, word(h, t + 0x108));
                        let l = word(h, t + 0x1b8);
                        sprintf(h, &[buf, 0x100, sss, m, l, s]);
                    }
                }
            }
            method(h, TEXT_APPEND, tu, &[buf]);
        }
        method(h, TEXT_SET_ENABLED, tu, &[0])
    }

    /// `CharacterAnimationTweakUtil_initialize(t)` (frame: the 0x28-byte
    /// line at `ebp - 0x2c`, cleared by word stores): four lines, the
    /// motion mode's name (`+0xc` into the name table), on/off (`+0`) and
    /// two numbers (`+4`, `+8`), cursor `+0xbc`; shown after.
    pub fn char_anim_init(h: &mut impl Host, ebp: u32, t: u32) -> u32 {
        let tu = t + 0x18;
        let r = method(h, TEXT_INIT, tu, &[]);
        if r != 0 {
            return init_failed(h, tu, t + 0x14);
        }
        h.write(t + 0x14, 1_u8);
        labels(
            h,
            t,
            &[
                (0xa8, 0x00b4_8914),
                (0xac, 0x00b4_8924),
                (0xb0, 0x00b4_892c),
                (0xb4, 0x00b4_8934),
            ],
        );
        h.write(t + 0xbc, r);
        let buf = ebp - 0x2c;
        for i in 0..4 {
            for k in 0..10 {
                h.write(buf + 4 * k, 0_u32);
            }
            if byte(h, t + 0x14) == 0 {
                let f = h.rebase(INIT_FAILED);
                sprintf(h, &[buf, 0x28, f]);
            } else {
                let m = mark(h, t + 0xbc, i);
                let sss = h.rebase(FMT_SSS);
                let ssd = h.rebase(FMT_SSD);
                match i {
                    0 => {
                        let mode = word(h, t + 0xc);
                        let names = h.rebase(MOTION_MODE_NAMES);
                        let v = word(h, names.wrapping_add(mode.wrapping_mul(4)));
                        let l = word(h, t + 0xa8);
                        sprintf(h, &[buf, 0x28, sss, m, l, v]);
                    }
                    1 => {
                        let s = on_off(h, byte(h, t) != 0);
                        let l = word(h, t + 0xac);
                        sprintf(h, &[buf, 0x28, sss, m, l, s]);
                    }
                    _ => {
                        let l = word(h, t + 0xa8 + 4 * i);
                        let v = word(h, t + 4 * (i - 1));
                        sprintf(h, &[buf, 0x28, ssd, m, l, v]);
                    }
                }
            }
            method(h, TEXT_APPEND, tu, &[buf]);
        }
        method(h, TEXT_SET_ENABLED, tu, &[1])
    }

    /// `CharacterSwitchTweakUtil_initialize(t)` (frame: the 0x14-byte line
    /// at `ebp - 0x18`, cleared by word stores): seven lines `"%s%s%03d"`
    /// of the labels at `+0x94..` with 1, each line's `+0xb0..` word set
    /// to 1 first, cursor `+0xd0`; hidden after.
    pub fn char_switch_init(h: &mut impl Host, ebp: u32, t: u32) -> u32 {
        let tu = t + 4;
        let r = method(h, TEXT_INIT, tu, &[]);
        if r != 0 {
            return init_failed(h, tu, t);
        }
        h.write(t, 1_u8);
        labels(
            h,
            t,
            &[
                (0x98, 0x00b4_86c8),
                (0x94, 0x00b4_86c4),
                (0x9c, 0x00b4_86cc),
                (0xa0, 0x00b4_86d0),
                (0xa4, 0x00b3_0da0),
                (0xa8, 0x00b0_db48),
                (0xac, 0x00b4_86d4),
            ],
        );
        h.write(t + 0xd0, r);
        let buf = ebp - 0x18;
        for i in 0..7 {
            let label = t + 0x94 + 4 * i;
            h.write(label + 0x1c, 1_u32);
            for k in 0..5 {
                h.write(buf + 4 * k, 0_u32);
            }
            if byte(h, t) == 0 {
                let f = h.rebase(INIT_FAILED);
                sprintf(h, &[buf, 0x14, f]);
            } else {
                let m = mark(h, t + 0xd0, i);
                let l = word(h, label);
                let fmt = h.rebase(FMT_SS03D);
                sprintf(h, &[buf, 0x14, fmt, m, l, 1]);
            }
            method(h, TEXT_APPEND, tu, &[buf]);
        }
        method(h, TEXT_SET_ENABLED, tu, &[0])
    }

    /// `CharacterTextureAnimationTweakUtil_initialize(t)` (frame: the
    /// 0x32-byte line at `ebp - 0x38`): six lines, on/off (`+0x11`), a
    /// label alone, the animation's (`+0x18`) two names, count and flag
    /// (`" "`, 0 and `"N/A"` without one), cursor `+0xcc`; hidden after.
    pub fn char_tex_init(h: &mut impl Host, ebp: u32, t: u32) -> u32 {
        let tu = t + 0x20;
        let r = method(h, TEXT_INIT, tu, &[]);
        if r != 0 {
            return init_failed(h, tu, t + 0x1c);
        }
        h.write(t + 0x1c, 1_u8);
        labels(
            h,
            t,
            &[
                (0xb0, 0x00b4_8880),
                (0xb4, 0x00b4_8890),
                (0xb8, 0x00b4_88a0),
                (0xbc, 0x00b4_88ac),
                (0xc0, 0x00b4_88bc),
                (0xc4, 0x00b4_88cc),
            ],
        );
        h.write(t + 0xcc, r);
        let buf = ebp - 0x38;
        for i in 0..6 {
            cdecl(h, MEMSET, &[buf, 0, 0x32]);
            if byte(h, t + 0x1c) == 0 {
                let f = h.rebase(INIT_FAILED);
                sprintf(h, &[buf, 0x14, f]);
            } else {
                let m = mark(h, t + 0xcc, i);
                let sss = h.rebase(FMT_SSS);
                let l = word(h, t + 0xb0 + 4 * i);
                let anim = word(h, t + 0x18);
                match i {
                    0 => {
                        let s = on_off(h, byte(h, t + 0x11) != 0);
                        sprintf(h, &[buf, 0x32, sss, m, l, s]);
                    }
                    1 => {
                        let ss = h.rebase(FMT_SS);
                        sprintf(h, &[buf, 0x32, ss, m, l]);
                    }
                    2 | 3 => {
                        let v = if anim == 0 {
                            h.rebase(SPACE)
                        } else {
                            let f = if i == 2 {
                                TEX_ANIM_NAME_A
                            } else {
                                TEX_ANIM_NAME_B
                            };
                            let p = method(h, f, anim, &[]);
                            let s = word(h, p);
                            s.wrapping_sub(u32::from(byte(h, p) & 1))
                        };
                        sprintf(h, &[buf, 0x32, sss, m, l, v]);
                    }
                    4 => {
                        let v = if anim == 0 {
                            0
                        } else {
                            method(h, TEX_ANIM_COUNT, anim, &[])
                        };
                        let ssd = h.rebase(FMT_SSD);
                        sprintf(h, &[buf, 0x32, ssd, m, l, v]);
                    }
                    _ => {
                        let s = if anim == 0 {
                            h.rebase(NA)
                        } else {
                            let al = method(h, TEX_ANIM_FLAG, anim, &[]) & 0xff;
                            on_off(h, al != 0)
                        };
                        sprintf(h, &[buf, 0x32, sss, m, l, s]);
                    }
                }
            }
            method(h, TEXT_APPEND, tu, &[buf]);
        }
        method(h, TEXT_SET_ENABLED, tu, &[0])
    }

    /// A panel's `render(t; ctx, fps)` (`ret 8`; frame: the title buffer):
    /// when shown and given a context, the title `format` of `fps` (and
    /// the viewer's map name) and `TextUtil_render(ctx)` -> its `eax`;
    /// else 0.
    pub fn render(h: &mut impl Host, r: Render, ebp: u32, t: u32, ctx: u32, fps: u32) -> u32 {
        if byte(h, t + r.flag) == 0 || ctx == 0 {
            return 0;
        }
        let buf = ebp - r.buf;
        if r.size == 0x100 {
            cdecl(h, MEMSET, &[buf, 0, 0x100]);
        } else {
            for k in 0..r.size / 4 {
                h.write(buf + 4 * k, 0_u32);
            }
        }
        let x = widen(fps).to_bits();
        #[allow(clippy::cast_possible_truncation)] // the double's halves
        let (lo, hi) = (x as u32, (x >> 32) as u32);
        let fmt = h.rebase(r.format);
        if r.map_name {
            let name = VIEWER_MANAGER.get(h).wrapping_add(0x51);
            sprintf(h, &[buf, r.size, fmt, lo, hi, name]);
        } else {
            sprintf(h, &[buf, r.size, fmt, lo, hi]);
        }
        let tu = t + r.text;
        method(h, TEXT_SET_TITLE, tu, &[buf]);
        method(h, TEXT_RENDER, tu, &[ctx])
    }

    /// `InlineBuffer_free_0064e9a0(b)` (an unwind funclet's cleanup, `ecx`
    /// the holder): the CRT's `free([b])` unless it is the inline storage
    /// at `b + 4`. `eax`: `free`'s, else `b + 4`.
    pub fn inline_buffer_free(h: &mut impl Host, b: u32) -> u32 {
        let p = word(h, b);
        if p == b.wrapping_add(4) {
            return p;
        }
        import(h, FREE, &[p])
    }

    /// `AtlCString_release_0064e9e0(s)` (`ecx` the string): the header at
    /// `[s] - 0x10` loses a reference (`InterlockedExchangeAdd(+0xc, -1)`);
    /// at 0 or below its string manager (`[header]`) frees it (virtual
    /// slot 1). `eax`: the method's, else `&header.refs`.
    pub fn cstring_release(h: &mut impl Host, s: u32) -> u32 {
        let header = word(h, s).wrapping_sub(0x10);
        let refs = header.wrapping_add(0xc);
        let f = word(h, h.rebase(INTERLOCKED_ADD));
        let old = h.call(f, Conv::Stdcall, &[refs, 0xffff_ffff]);
        if old.wrapping_sub(1).cast_signed() > 0 {
            return refs;
        }
        let manager = word(h, header);
        h.call_virtual(manager, 1, &[header])
    }

    /// `std_string_tidy_0064ea00(s)`: the heap buffer freed (capacity 16
    /// or more), then empty with capacity 15. `eax`: `operator_delete`'s
    /// or the caller's (the unwind funclets read neither).
    pub fn string_tidy(h: &mut impl Host, s: u32) {
        if word(h, s + 0x14) >= 0x10 {
            let p = word(h, s);
            cdecl(h, OPERATOR_DELETE, &[p]);
        }
        h.write(s + 0x14, 0xf_u32);
        h.write(s + 0x10, 0_u32);
        h.write(s, 0_u8);
    }

    /// `std_vector_tidy_0064ea30(v)`: with storage, `operator_delete`d and
    /// the three words cleared. `eax`: `operator_delete`'s, else 0.
    pub fn vector_tidy(h: &mut impl Host, v: u32) -> u32 {
        let p = word(h, v);
        if p == 0 {
            return 0;
        }
        let eax = cdecl(h, OPERATOR_DELETE, &[p]);
        for k in 0..3 {
            h.write(v + 4 * k, 0_u32);
        }
        eax
    }

    /// `PhyreSceneCursor_postNext4_0064f1a0(c; out, _)` (`ret 8`) -> out:
    /// the cursor's eight words copied to `out`, then one step: with a
    /// count (`+0xc`) left, it drops by one and the position (`+0`) moves
    /// by the stride (`+8`); reaching the block end (`+4`) moves to the
    /// next block (`[end]`) and repeats while the count lasts. At count 0,
    /// `PhyreSceneCursor_Next4(c)`.
    pub fn cursor_post_next4(h: &mut impl Host, c: u32, out: u32) -> u32 {
        for k in 0..8 {
            let x = word(h, c + 4 * k);
            h.write(out + 4 * k, x);
        }
        if word(h, c + 0xc) == 0 {
            return out;
        }
        let stride = word(h, c + 8);
        loop {
            let n = word(h, c + 0xc).wrapping_sub(1);
            h.write(c + 0xc, n);
            let end = word(h, c + 4);
            let pos = word(h, c).wrapping_add(stride);
            h.write(c, pos);
            let count = word(h, c + 0xc);
            if end == word(h, c) {
                let next = word(h, end);
                h.write(c + 4, next);
                if count != 0 {
                    continue;
                }
            }
            if count == 0 {
                method(h, CURSOR_NEXT4, c, &[]);
            }
            return out;
        }
    }

    /// `std_string_less_0064f550(a, b)` (stdcall, `ret 8`): `a.compare(0,
    /// a.size, b.data, b.size) < 0` in `al`; the rest of `eax` is the
    /// compare's.
    pub fn string_less(h: &mut impl Host, a: u32, b: u32) -> u32 {
        let n = word(h, b + 0x10);
        let p = c_str(h, b);
        let size = word(h, a + 0x10);
        let r = method(h, STRING_COMPARE, a, &[0, size, p, n]);
        (r & 0xffff_ff00) | u32::from(r.cast_signed() < 0)
    }

    /// `std_string_append_cstr_0064f5e0(s; p)` (`ret 4`): `s.append(p,
    /// strlen(p))` -> its `eax`.
    pub fn string_append_cstr(h: &mut impl Host, s: u32, p: u32) -> u32 {
        let mut n = 0_u32;
        while byte(h, p.wrapping_add(n)) != 0 {
            n += 1;
        }
        method(h, STRING_APPEND, s, &[p, n])
    }

    /// `FreeCamera_attach_0067fda0(cam; target)` (`ret 4`; frame: two
    /// matrices at `ebp - 0x80`/`- 0x70` and `- 0x40`/`- 0x30`): 0x16 when
    /// attached to another target already. Else the target's matrix
    /// pointer (`+0xb4`) taken into `+0x74` and replaced by the camera's
    /// own (`+4`), the target's `+4` kept in `+0x78`; the 4x4 at `+0x34`
    /// built from that 3x4 (identity's when there was none) and the 3x4 at
    /// `+4` from it. -> 0.
    pub fn free_camera_attach(h: &mut impl Host, ebp: u32, cam: u32, target: u32) -> u32 {
        let attached = word(h, cam + 0x78);
        if attached != 0 {
            let own = if target == 0 { 0 } else { target + 4 };
            if attached != own {
                return 0x16;
            }
        }
        let m = word(h, target.wrapping_add(0xb4));
        h.write(cam + 0x74, m);
        h.write(cam + 0x78, target.wrapping_add(4));
        h.write(target.wrapping_add(0xb4), cam + 4);
        let src = word(h, cam + 0x74);
        let from = if src == 0 {
            let id = ebp - 0x40;
            for k in 0..16 {
                let one = k % 5 == 0;
                h.write(id + 4 * k, if one { 0x3f80_0000_u32 } else { 0 });
            }
            let m3 = ebp - 0x70;
            method(h, TO_WORLD_3X4, m3, &[id]);
            m3
        } else {
            let to = ebp - 0x30;
            for k in 0..12 {
                let x = word(h, src.wrapping_add(4 * k));
                h.write(to + 4 * k, x);
            }
            to
        };
        let m4 = cam + 0x34;
        method(h, FROM_3X4, m4, &[from]);
        let copy = ebp - 0x80;
        for k in 0..16 {
            let x = word(h, m4 + 4 * k);
            h.write(copy + 4 * k, x);
        }
        method(h, TO_WORLD_3X4, cam + 4, &[copy]);
        0
    }

    /// `rcbgGuideMapSave(a, loc)`: the double at `loc + 8` truncated
    /// (`ftol2_sse`) into its own argument slot, then a tail jump to
    /// `GuideMap_setLocationText_006e9870(a, that)` -> its `eax`.
    pub fn guide_map_save(h: &mut impl Host, slots: u32, a: u32, loc: u32) -> u32 {
        let x: f64 = h.read(loc.wrapping_add(8));
        let v = ffx_bridge::x87::ftol(h, x);
        h.write(slots + 4, v);
        cdecl(h, GUIDE_SET_LOCATION_TEXT, &[a, v])
    }
}
