//! List p6-render-c, continuation round 4: the list's larger remaining
//! functions (the Iggy warning callback, ...).
//!
//! Conventions are read from the disassembly; `eax` is returned wherever
//! the original defines it.

use ffx_graphic::host::{Conv, Host};

/// The CRT's `sprintf_s` (cdecl, through its import slot) and the TTY
/// dispatch `0x00435390(level, text)` (cdecl).
pub const SPRINTF_S: u32 = 0x00b0_c394;
pub const TTY_DISPATCH: u32 = 0x0043_5390;
/// The Iggy callback's strings: `"Warning: "`, `"Error: "`, `"Iggy: %s"`
/// and `"%s"`.
pub const IGGY_WARNING: u32 = 0x00b7_aa18;
pub const IGGY_ERROR: u32 = 0x00b7_aa24;
pub const IGGY_FORMAT: u32 = 0x00b7_aa2c;
pub const PERCENT_S: u32 = 0x00b0_e160;
/// The Iggy callback's text buffer, `ebp - 0x4004`.
pub const IGGY_BUFFER: u32 = 0x4000;

pub mod bridge {
    use super::{
        Conv, Host, IGGY_BUFFER, IGGY_ERROR, IGGY_FORMAT, IGGY_WARNING, PERCENT_S, SPRINTF_S,
        TTY_DISPATCH,
    };

    fn sprintf_s(h: &mut impl Host, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(SPRINTF_S));
        h.call(f, Conv::Cdecl, args)
    }

    fn strlen(h: &impl Host, p: u32) -> u32 {
        let mut n = 0u32;
        while h.read::<u8>(p.wrapping_add(n)) != 0 {
            n = n.wrapping_add(1);
        }
        n
    }

    /// `0x00a3e600(_, _, code, text)` (stdcall, `ret 0x10`), the warning
    /// callback `Iggy_init` hands to Iggy: `"Iggy: Warning: "` (code below
    /// 400, level 1) or `"Iggy: Error: "` (level 3), the text and a
    /// newline, built with `sprintf_s` in `buf` (the original's 16 KB
    /// frame buffer, `IGGY_BUFFER` bytes) and sent to the TTY dispatch;
    /// returns the dispatch's `eax`.
    pub fn iggy_warning(h: &mut impl Host, buf: u32, code: i32, text: u32) -> u32 {
        let (level, kind) = if code < 400 {
            (1, IGGY_WARNING)
        } else {
            (3, IGGY_ERROR)
        };
        let kind = h.rebase(kind);
        let format = h.rebase(IGGY_FORMAT);
        sprintf_s(h, &[buf, IGGY_BUFFER, format, kind]);
        let at = buf.wrapping_add(strlen(h, buf));
        let percent_s = h.rebase(PERCENT_S);
        let room = buf.wrapping_add(IGGY_BUFFER).wrapping_sub(at);
        sprintf_s(h, &[at, room, percent_s, text]);
        let end = at.wrapping_add(strlen(h, at));
        h.write(end, 0x000au16);
        h.call_sym(TTY_DISPATCH, Conv::Cdecl, &[level, buf])
    }
}

/// `PBitmapTextMaterial::initialize`'s callees: the render pass
/// (`PUtilityText::GetRenderPassForTechnique(i)`, cdecl), the cursor's
/// seek (`PhyreSceneCursor_Seek4`, thiscall on the cursor) over the
/// cluster's objects of type `0x00ca5268`, the pass state's
/// `0x0058cf80(state)`, the parameter lookup by name `0x0056cbf0(shader;
/// name)`, the parameter buffer's `setTextureChecked(buffer; desc, tex)`
/// and `setValueDesc(buffer; desc, src)`, `PEffectVariant::createMaterial
/// (shader; cluster, 0, 0)`, `setTextColor(material; rgb)`, the CRT's
/// `memcpy` (cdecl), `PhyreFree` (cdecl) and `PSharray_setWord(array; i,
/// v)`.
pub const GET_RENDER_PASS: u32 = 0x00a3_e9b0;
pub const CURSOR_SEEK: u32 = 0x0065_ecb0;
pub const MATERIAL_CURSOR_TYPE: u32 = 0x00ca_5268;
pub const STATE_REGISTER: u32 = 0x0058_cf80;
pub const FIND_PARAMETER: u32 = 0x0056_cbf0;
pub const SET_TEXTURE_CHECKED: u32 = 0x0056_e670;
pub const SET_VALUE_DESC: u32 = 0x0056_e600;
pub const CREATE_MATERIAL: u32 = 0x004e_6a40;
pub const SET_TEXT_COLOR: u32 = 0x00a2_a650;
pub const MEMCPY: u32 = 0x0094_9256;
pub const FREE: u32 = 0x0042_fb20;
pub const SET_WORD: u32 = 0x0048_c540;
/// The parameter names `"BitmapFontTexture"` and the pass-state one at
/// `0x00b3018c`.
pub const FONT_TEXTURE_NAME: u32 = 0x00b7_8ca8;
pub const STATE_NAME: u32 = 0x00b3_018c;

pub mod material {
    use super::{
        CREATE_MATERIAL, CURSOR_SEEK, Conv, FIND_PARAMETER, FONT_TEXTURE_NAME, FREE,
        GET_RENDER_PASS, Host, MATERIAL_CURSOR_TYPE, MEMCPY, SET_TEXT_COLOR, SET_TEXTURE_CHECKED,
        SET_VALUE_DESC, SET_WORD, STATE_NAME, STATE_REGISTER,
    };

    /// `PBitmapTextMaterial::initialize(this; cluster, technique)`
    /// (thiscall, `ret 8`) -> 0 or an error (7 without a render pass or a
    /// shader, 0xe without a material, a callee's). `frame` is the
    /// original's `ebp - 0x20`: the cursor (8 words: 5 zero, the
    /// cluster's list head `+0x1c`, its first object or 0, the type),
    /// reused for the texture descriptor `{0x42, 0, state, [[this+0x30]
    /// +0x20]}` at `+0x10` and then the colour (1, 1, 1).
    ///
    /// The cursor's object is `{shader, buffer}`: the pass state (`this +
    /// 0xc`; bytes 5, 1, 2, 2, 2 unless bit 7 of `+0x28`) is registered,
    /// the buffer's `"BitmapFontTexture"` and state parameters set, the
    /// material created into `+0`, its colour set, the buffer copied into
    /// the material's (`memcpy(dst, buffer, [buffer])`), the word array
    /// `+4` made one inline word (an owned block freed) and its word 0 the
    /// material.
    ///
    /// The second result: the `technique` argument slot's last value when
    /// the original reuses it (the shader).
    #[allow(clippy::too_many_lines)]
    pub fn initialize(
        h: &mut impl Host,
        this: u32,
        frame: u32,
        cluster: u32,
        technique: u32,
    ) -> (u32, Option<u32>) {
        let pass = h.call_sym(GET_RENDER_PASS, Conv::Cdecl, &[technique]);
        h.write(this.wrapping_add(0x34), pass);
        if pass == 0 {
            return (7, None);
        }
        let head = cluster.wrapping_add(0x1c);
        let first: u32 = h.read(head);
        let first = if first == head { 0 } else { first };
        for k in 0..5 {
            h.write(frame + 4 * k, 0u32);
        }
        h.write(frame + 0x14, head);
        h.write(frame + 0x18, first);
        let ty = h.rebase(MATERIAL_CURSOR_TYPE);
        h.write(frame + 0x1c, ty);
        h.call_sym(CURSOR_SEEK, Conv::Thiscall(frame), &[]);
        if h.read::<u32>(frame + 0xc) == 0 {
            return (7, None);
        }
        let entry = h.read::<u32>(frame + 0x10).wrapping_add(h.read(frame));
        if entry == 0 {
            return (0, None);
        }
        let state = this.wrapping_add(0xc);
        if h.read::<u32>(this.wrapping_add(0x28)) & 0x80 == 0 {
            for (k, v) in [5u8, 1, 2, 2, 2].into_iter().enumerate() {
                h.write(state + k as u32, v);
            }
        }
        let r = h.call_sym(STATE_REGISTER, Conv::Thiscall(state), &[]);
        if r != 0 {
            return (r, None);
        }
        let shader: u32 = h.read(entry);
        h.write(frame + 0x14, 0u32);
        let owner: u32 = h.read(this.wrapping_add(0x30));
        let owner_tex: u32 = h.read(owner.wrapping_add(0x20));
        let slot = Some(shader);
        h.write(frame + 0x10, 0x42u32);
        h.write(frame + 0x1c, owner_tex);
        h.write(frame + 0x18, state);
        let name = h.rebase(FONT_TEXTURE_NAME);
        let param = h.call_sym(FIND_PARAMETER, Conv::Thiscall(shader), &[name]);
        let buffer: u32 = h.read(entry.wrapping_add(4));
        if param != 0 {
            let r = h.call_sym(
                SET_TEXTURE_CHECKED,
                Conv::Thiscall(buffer),
                &[param, frame + 0x10],
            );
            if r != 0 {
                return (r, slot);
            }
        }
        let name = h.rebase(STATE_NAME);
        let param = h.call_sym(FIND_PARAMETER, Conv::Thiscall(shader), &[name]);
        if param != 0 {
            let r = h.call_sym(SET_VALUE_DESC, Conv::Thiscall(buffer), &[param, state]);
            if r != 0 {
                return (r, slot);
            }
        }
        let material = h.call_sym(CREATE_MATERIAL, Conv::Thiscall(shader), &[cluster, 0, 0]);
        h.write(this, material);
        if material == 0 {
            return (0xe, slot);
        }
        for k in 0..3 {
            h.write(frame + 0x10 + 4 * k, 1.0f32);
        }
        let r = h.call_sym(SET_TEXT_COLOR, Conv::Thiscall(this), &[frame + 0x10]);
        if r != 0 {
            return (r, slot);
        }
        let n: u32 = h.read(buffer);
        let material: u32 = h.read(this);
        let dst: u32 = h.read(material.wrapping_add(4));
        h.call_sym(MEMCPY, Conv::Cdecl, &[dst, buffer, n]);
        let words = this.wrapping_add(4);
        let count: u32 = h.read(words);
        if count != 1 {
            let n = count & 0x7fff_ffff;
            let inline = this.wrapping_add(8);
            let old = if n > 1 {
                h.read(inline)
            } else if n != 0 {
                inline
            } else {
                0
            };
            if inline == 0 {
                return (0xd, slot);
            }
            h.write(inline, 0u32);
            if old != inline && h.read::<u32>(words) >> 31 == 0 {
                h.call_sym(FREE, Conv::Cdecl, &[old]);
            }
            h.write(words, 1u32);
        }
        let material: u32 = h.read(this);
        (
            h.call_sym(SET_WORD, Conv::Thiscall(words), &[0, material]),
            slot,
        )
    }
}

/// The parameter layout (`0x005a0600`, `0x005a08b0`, `0x005a0cf0`): the
/// records `{u16 offset, u16 size, u32 alignment, u32 index}` (12 bytes,
/// a `PArray` `{count (bit 31 not owned), data, end}`) built from the
/// 16-byte parameter descriptors (`+0` u16 array count, byte `+2` type,
/// byte `+3` format, `+8` u16 offset, `+0xa` u16 size and 3 flag bits).
///
/// The sizer object's virtual slots 2 and 3 (`thiscall(obj; type
/// descriptor)`) give a type's size and alignment; the descriptors
/// (image data) by parameter type are [`layout_descriptor`]'s.
/// `0x005a04d0(type)` (cdecl, `al`) marks the matrix types,
/// `0x0055bc40(format)` (cdecl, `ax`) gives a plain format's size.
pub const IS_MATRIX_TYPE: u32 = 0x005a_04d0;
pub const FORMAT_SIZE: u32 = 0x0055_bc40;
pub const MALLOC: u32 = 0x0042_fb80;
/// The CRT's `qsort` (cdecl, import slot) and the record comparator
/// `0x005a0820` (size descending, then index descending).
pub const QSORT: u32 = 0x00b0_c3ac;
pub const RECORD_COMPARE: u32 = 0x005a_0820;
/// `build_records` and `pack` (thiscall on the record array).
pub const BUILD_RECORDS: u32 = 0x005a_08b0;
pub const PACK_RECORDS: u32 = 0x005a_0cf0;

/// The type descriptor `0x005a08b0` asks the sizer about for parameter
/// type `t`, or `None` for a type sized by its format.
#[must_use]
pub fn layout_descriptor(h: &mut impl Host, t: u8) -> Option<u32> {
    let d = match t {
        0x42 | 0xd9..=0xdc | 0x12 | 0x47 => 0x00c9_bef8,
        0x43 => 0x00c9_bf90,
        0x44 => 0x00c9_c028,
        0x48 => 0x00c9_c158,
        0x49 | 0x14 | 0x15 => 0x00c9_c1f0,
        0x4b => 0x00c9_c288,
        0x4c => 0x00c9_c320,
        _ => {
            if h.call_sym(IS_MATRIX_TYPE, Conv::Cdecl, &[u32::from(t)]) & 0xff != 0 {
                0x00c9_be60
            } else {
                match t {
                    0xde..=0xe1 => 0x00c9_c3b8,
                    0xe2 => 0x00c9_c450,
                    0x4d => 0x00c9_c4e8,
                    0x4e => 0x00c9_c580,
                    _ => return None,
                }
            }
        }
    };
    Some(d)
}

pub mod layout {
    use super::{
        BUILD_RECORDS, Conv, FORMAT_SIZE, FREE, Host, MALLOC, PACK_RECORDS, QSORT, RECORD_COMPARE,
        layout_descriptor,
    };

    /// `0x005a08b0(records; params, n, sizer)` (thiscall, `ret 0xc`) -> 0,
    /// 1 (no parameters for a non-zero count) or 0xd (no memory): the
    /// array resized to `n` fresh records (`{0, 1, -1}` each; the old
    /// owned data freed) unless it holds `n`, then record `k` = `{0, size
    /// times the array count (when not 0), alignment, k}` for descriptor
    /// `k`. Plain formats take their size from `0x0055bc40` and its
    /// lowest set bit as the alignment (at most 16, 1 for size 0).
    pub fn build_records(h: &mut impl Host, records: u32, params: u32, n: u32, sizer: u32) -> u32 {
        if params == 0 && n != 0 {
            return 1;
        }
        if n != h.read::<u32>(records) {
            let mut data = 0;
            if n != 0 {
                data = h.call_sym(MALLOC, Conv::Cdecl, &[n.wrapping_mul(12), 4]);
                if data == 0 {
                    return 0xd;
                }
                for k in 0..n {
                    let r = data.wrapping_add(12 * k);
                    if r != 0 {
                        h.write(r, 0u32);
                        h.write(r + 4, 1u32);
                        h.write(r + 8, u32::MAX);
                    }
                }
            }
            let old: u32 = h.read(records + 4);
            let head: u32 = h.read(records);
            if old != data && head >> 31 == 0 && old != 0 {
                h.call_sym(FREE, Conv::Cdecl, &[old]);
            }
            h.write(records + 4, data);
            h.write(records, n);
        }
        let mut out: u32 = h.read(records + 4);
        for k in 0..n {
            let p = params.wrapping_add(16 * k);
            let t: u8 = h.read(p.wrapping_add(2));
            let (size, align) = if let Some(d) = layout_descriptor(h, t) {
                let d = h.rebase(d);
                let size = h.call_virtual(sizer, 2, &[d]);
                (size, h.call_virtual(sizer, 3, &[d]))
            } else {
                let format: u8 = h.read(p.wrapping_add(3));
                let size = h.call_sym(FORMAT_SIZE, Conv::Cdecl, &[u32::from(format)]) & 0xffff;
                let mut align = 1u32;
                if size != 0 {
                    let mut bits = ((size - 1) ^ size) >> 1;
                    while bits != 0 {
                        align = align.wrapping_add(align);
                        bits >>= 1;
                    }
                    align = align.min(0x10);
                }
                (size, align)
            };
            let count = u32::from(h.read::<u16>(p));
            let total = if count == 0 {
                size
            } else {
                count.wrapping_mul(size)
            };
            h.write(out.wrapping_add(4), align);
            h.write(out.wrapping_add(2), total as u16);
            h.write(out.wrapping_add(8), k);
            h.write(out, 0u16);
            out = out.wrapping_add(12);
        }
        0
    }

    /// `0x005a0cf0(records; start)` (thiscall, `ret 4`) -> 0: the records
    /// sorted by pointer (the CRT's `qsort` with `0x005a0820` over an
    /// array the original takes with `_alloca`, here `alloc(count)`), then
    /// placed from `start` in that order: each at its alignment, the gap
    /// before it filled first with later records that fit (size, and
    /// aligned where they land), pass after pass; the end into `+8`.
    pub fn pack(
        h: &mut impl Host,
        records: u32,
        start: u32,
        alloc: impl FnOnce(u32) -> u32,
    ) -> u32 {
        let n = h.read::<u32>(records) & 0x7fff_ffff;
        let ptrs = alloc(n);
        for i in 0..n {
            let data: u32 = h.read(records + 4);
            h.write(ptrs.wrapping_add(4 * i), data.wrapping_add(12 * i));
        }
        let qsort: u32 = h.read(h.rebase(QSORT));
        let compare = h.rebase(RECORD_COMPARE);
        h.call(qsort, Conv::Cdecl, &[ptrs, n, 4, compare]);
        let mut offset = start;
        let at = |i: u32| ptrs.wrapping_add(4 * i);
        for i in 0..n {
            let r: u32 = h.read(at(i));
            h.write(at(i), 0u32);
            if r == 0 {
                continue;
            }
            let align: u32 = h.read(r + 4);
            let aligned = offset.wrapping_sub(1).wrapping_add(align) & !align.wrapping_sub(1);
            let mut gap = aligned.wrapping_sub(offset);
            let mut found = true;
            'fill: while gap != 0 && found {
                found = false;
                for j in i + 1..n {
                    let q: u32 = h.read(at(j));
                    if q != 0
                        && u32::from(h.read::<u16>(q + 2)) <= gap
                        && offset & h.read::<u32>(q + 4).wrapping_sub(1) == 0
                    {
                        h.write(at(j), 0u32);
                        h.write(q, offset as u16);
                        offset = offset.wrapping_add(u32::from(h.read::<u16>(q + 2)));
                        gap = aligned.wrapping_sub(offset);
                        found = true;
                    }
                    if gap == 0 {
                        break 'fill;
                    }
                }
            }
            h.write(r, aligned as u16);
            offset = u32::from(h.read::<u16>(r + 2)).wrapping_add(aligned);
        }
        h.write(records + 8, offset);
        0
    }

    /// `0x005a0600(base, params, n, &end, sizer)` (cdecl) -> 0 or the
    /// first error: a record array in `frame` (the original's `ebp -
    /// 0x1c`, 3 words) built (`0x005a08b0`) and packed from `base`
    /// (`0x005a0cf0`), each descriptor's offset (`+8`) and size (`+0xa`,
    /// keeping its top 3 bits) taken from its record, `*end` the packed
    /// end; the array's owned data freed.
    pub fn layout(h: &mut impl Host, frame: u32, args: [u32; 5]) -> u32 {
        let [base, params, n, end, sizer] = args;
        for k in 0..3 {
            h.write(frame + 4 * k, 0u32);
        }
        let mut r = h.call_sym(BUILD_RECORDS, Conv::Thiscall(frame), &[params, n, sizer]);
        if r == 0 {
            r = h.call_sym(PACK_RECORDS, Conv::Thiscall(frame), &[base]);
        }
        if r == 0 {
            let count = h.read::<u32>(frame) & 0x7fff_ffff;
            let data: u32 = h.read(frame + 4);
            for i in 0..n {
                let src = if i < count {
                    data.wrapping_add(12 * i)
                } else {
                    0
                };
                let p = params.wrapping_add(16 * i);
                let offset: u16 = h.read(src);
                h.write(p.wrapping_add(8), offset);
                let size: u16 = h.read(src.wrapping_add(2));
                let old: u16 = h.read(p.wrapping_add(0xa));
                h.write(p.wrapping_add(0xa), (old & 0xe000) | size);
            }
            let e: u32 = h.read(frame + 8);
            h.write(end, e);
        }
        let head: u32 = h.read(frame);
        let data: u32 = h.read(frame + 4);
        if head >> 31 == 0 && data != 0 {
            h.call_sym(FREE, Conv::Cdecl, &[data]);
        }
        r
    }
}

/// `PInputDeviceMouse::update`'s imports (user32, stdcall through their
/// slots), the window handle and settings globals, and the base
/// device's update `0x0062d550` (thiscall).
pub const GET_CURSOR_POS: u32 = 0x00b0_c6c8;
pub const GET_WINDOW_RECT: u32 = 0x00b0_c6d0;
pub const GET_CLIENT_RECT: u32 = 0x00b0_c680;
pub const CLIP_CURSOR: u32 = 0x00b0_c6c4;
pub const GET_WINDOW_INFO: u32 = 0x00b0_c654;
pub const GET_SYSTEM_METRICS: u32 = 0x00b0_c6dc;
pub const HWND: u32 = 0x00cc_9ce8;
pub const SETTINGS: u32 = 0x022f_b504;
pub const DEVICE_UPDATE: u32 = 0x0062_d550;
/// `DIERR_INPUTLOST`, which `Acquire` keeps returning while another
/// program holds the mouse.
pub const INPUT_LOST: u32 = 0x8007_001e;

pub mod mouse {
    use super::{
        CLIP_CURSOR, Conv, DEVICE_UPDATE, GET_CLIENT_RECT, GET_CURSOR_POS, GET_SYSTEM_METRICS,
        GET_WINDOW_INFO, GET_WINDOW_RECT, HWND, Host, INPUT_LOST, SETTINGS,
    };

    /// The frame `update` hands to its callees, at the original's `ebp`
    /// offsets from `frame = ebp - 0x84`: the cursor point (`+0`), the
    /// window info (`+0x10`), the window and client rectangles (`+0x4c`,
    /// `+0x5c`) and the device state (`+0x6c`, `DIMOUSESTATE2`'s 0x14
    /// bytes).
    pub const FRAME: u32 = 0x84;
    const POINT: u32 = 0;
    const INFO: u32 = 0x10;
    const WINDOW: u32 = 0x4c;
    const CLIENT: u32 = 0x5c;
    const STATE: u32 = 0x6c;
    /// The original's locals for the cursor's x and the frame height.
    const FRAME_X: u32 = 8;
    const FRAME_FY: u32 = 0xc;

    fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Stdcall, args)
    }

    /// The three buttons the device keeps (`+0x118`, `+0x119`, `+0x11a`):
    /// the state's buttons 0, 2 and 1, pressed when bit 7 is set; only
    /// those held already when `held_only`.
    fn buttons(h: &mut impl Host, mouse: u32, frame: u32, held_only: bool) {
        for (off, b) in [(0x118, 0), (0x119, 2), (0x11a, 1)] {
            let at = mouse.wrapping_add(off);
            if !held_only || h.read::<u8>(at) != 0 {
                let v: u8 = h.read(frame + STATE + 0xc + b);
                h.write(at, v >> 7);
            }
        }
    }

    /// `PInputDeviceMouse::update(mouse)` (thiscall; `PInputDeviceMouse`'s
    /// vtable slot 5) -> `eax`: with a device (`+0x14`: byte `+0` "in the
    /// window", `+4` the `IDirectInputDevice8`), its state
    /// (`GetDeviceState(0x14, state)`, vtable slot 9); on failure
    /// `Acquire` (slot 7), repeated while it says the input is lost, and
    /// its result. In absolute mode (byte `+0x108`) the cursor's place in
    /// the client area (window rectangle less the frame and caption
    /// metrics, the cursor clipped to the window in fullscreen,
    /// `[g_Settings+8] == 1`) times 255 into `+0x10c`/`+0x110`, the wheel
    /// as 255, 0 or -255 into `+0x114`, the buttons (outside the client
    /// area, or entering it, only released); else the relative axes
    /// times 255 and the buttons. Then the base update `0x0062d550` and
    /// its `eax`.
    ///
    /// Without a device the original returns its stack cookie's word in
    /// `eax` (garbage): 0 here. The window info's `cbSize` is never
    /// written before `GetWindowInfo` reads it (junk read: [ebp-0x74]
    /// never written; 0, brief 2026-10-24): the frame is zeroed.
    pub fn update(h: &mut impl Host, mouse: u32, frame: u32) -> u32 {
        let dev: u32 = h.read(mouse.wrapping_add(0x14));
        if dev == 0 {
            return 0;
        }
        for k in 0..5 {
            h.write(frame + STATE + 4 * k, 0u32);
        }
        let com: u32 = h.read(dev.wrapping_add(4));
        let vt: u32 = h.read(com);
        let get_state: u32 = h.read(vt.wrapping_add(0x24));
        let r = h.call(get_state, Conv::Stdcall, &[com, 0x14, frame + STATE]);
        if (r as i32) < 0 {
            let mut r;
            loop {
                let com: u32 = h.read(dev.wrapping_add(4));
                let vt: u32 = h.read(com);
                let acquire: u32 = h.read(vt.wrapping_add(0x1c));
                r = h.call(acquire, Conv::Stdcall, &[com]);
                if r != INPUT_LOST {
                    break;
                }
            }
            return r;
        }
        if h.read::<u8>(mouse.wrapping_add(0x108)) == 1 {
            absolute(h, mouse, dev, frame);
        } else {
            for (k, off) in [0x10c, 0x110, 0x114].into_iter().enumerate() {
                let v: u32 = h.read(frame + STATE + 4 * k as u32);
                h.write(mouse.wrapping_add(off), v.wrapping_mul(0xff));
            }
            buttons(h, mouse, frame, false);
        }
        h.call_sym(DEVICE_UPDATE, Conv::Thiscall(mouse), &[])
    }

    fn absolute(h: &mut impl Host, mouse: u32, dev: u32, frame: u32) {
        if import(h, GET_CURSOR_POS, &[frame + POINT]) == 0 {
            return;
        }
        let hwnd: u32 = h.read(h.rebase(HWND));
        if import(h, GET_WINDOW_RECT, &[hwnd, frame + WINDOW]) == 0 {
            return;
        }
        if import(h, GET_CLIENT_RECT, &[hwnd, frame + CLIENT]) == 0 {
            return;
        }
        let settings: u32 = h.read(h.rebase(SETTINGS));
        if h.read::<u32>(settings.wrapping_add(8)) == 1 {
            import(h, CLIP_CURSOR, &[frame + WINDOW]);
        }
        import(h, GET_WINDOW_INFO, &[hwnd, frame + INFO]);
        let style: u32 = h.read(frame + INFO + 0x24);
        let (fx, fy) = if style & 0x0080_0000 == 0 {
            (0, 0)
        } else {
            let fx = import(h, GET_SYSTEM_METRICS, &[0x20]);
            (fx, import(h, GET_SYSTEM_METRICS, &[0x21]))
        };
        let px: u32 = h.read(frame + POINT);
        let py: u32 = h.read(frame + POINT + 4);
        let left: u32 = h.read(frame + WINDOW);
        let top: u32 = h.read(frame + WINDOW + 4);
        h.write(frame + FRAME_FY, fy);
        let x = px.wrapping_sub(left).wrapping_sub(fx);
        h.write(frame + FRAME_X, x);
        let x = x as i32;
        let mut y = py.wrapping_sub(top).wrapping_sub(fy);
        if style & 0x00c0_0000 != 0 {
            y = y.wrapping_sub(import(h, GET_SYSTEM_METRICS, &[4]));
        }
        let y = y as i32;
        h.write(mouse.wrapping_add(0x10c), x.wrapping_mul(0xff));
        h.write(mouse.wrapping_add(0x110), y.wrapping_mul(0xff));
        let z: i32 = h.read(frame + STATE + 8);
        let wheel: u32 = match z {
            1.. => 0xff,
            0 => 0,
            _ => 0xffff_ff01,
        };
        h.write(mouse.wrapping_add(0x114), wheel);
        let w = h
            .read::<u32>(frame + CLIENT + 8)
            .wrapping_sub(h.read(frame + CLIENT)) as i32;
        let ht = h
            .read::<u32>(frame + CLIENT + 0xc)
            .wrapping_sub(h.read(frame + CLIENT + 4)) as i32;
        let inside = x >= 0 && x < w && y >= 0 && y < ht;
        if inside {
            let was = h.read::<u8>(dev) == 1;
            buttons(h, mouse, frame, !was);
            h.write(dev, 1u8);
        } else {
            buttons(h, mouse, frame, true);
            h.write(dev, 0u8);
        }
    }
}
