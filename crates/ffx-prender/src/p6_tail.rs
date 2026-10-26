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

/// `PBitmapFontText`'s capacity callees: the vertex streams' format,
/// `isBound` and release (as the full-screen meshes'), the index block's
/// `isBound` `0x00562db0` and release `0x005648c0`, its map
/// (`PRingBuffer_submit_00434cd0`) and unmap `0x00565000` (thiscall),
/// `memset` (cdecl) and the index format's size `0x0055bc40(0xc)`.
pub const INDEX_IS_BOUND: u32 = 0x0056_2db0;
pub const INDEX_RELEASE: u32 = 0x0056_48c0;
pub const INDEX_MAP: u32 = 0x0043_4cd0;
pub const INDEX_UNMAP: u32 = 0x0056_5000;
pub const MEMSET: u32 = 0x0094_964c;

pub mod text {
    use super::{
        Conv, FORMAT_SIZE, FREE, Host, INDEX_IS_BOUND, INDEX_MAP, INDEX_RELEASE, INDEX_UNMAP,
        MALLOC, MEMSET,
    };
    use crate::p6_quads::{
        IS_BOUND, NAME_COLOR, NAME_POSITION, NAME_TEXCOORD, RELEASE, STREAM_FORMAT,
    };

    /// A vertex stream's format and semantic: `0x004835d0(s; stride,
    /// count, 1)`, the block size `[s] * [s+4]` into `+0x38` (a bound
    /// buffer released first); a failing format returns its `eax` before
    /// the semantic record (`[s+0xc]`: byte `+8`, name `+4`, `+0` 0).
    fn stream(h: &mut impl Host, s: u32, stride: u32, count: u32, semantic: u8, name: u32) -> u32 {
        let r = h.call_sym(STREAM_FORMAT, Conv::Thiscall(s), &[stride, count, 1]);
        let bytes = h.read::<u32>(s + 4).wrapping_mul(h.read(s));
        if h.read::<u32>(s + 0x38) != bytes {
            if h.call_sym(IS_BOUND, Conv::Thiscall(s), &[]) & 0xff != 0 {
                h.call_sym(RELEASE, Conv::Thiscall(s), &[]);
            }
            h.write(s + 0x38, bytes);
        }
        if r != 0 {
            return r;
        }
        let record: u32 = h.read(s + 0xc);
        h.write(record + 8, semantic);
        let name = h.rebase(name);
        h.write(record + 4, name);
        h.write(record, 0u32);
        0
    }

    /// `0x00a2a110(text; n)` (thiscall, `ret 4`) -> 0 or an error: the
    /// text's character buffer (`+0x15c` count, bit 31 not owned; `+0x160`
    /// data) made `n + 1` bytes (a `PhyreMalloc(n + 1, 1)` copy of what
    /// fits, 0xd without memory; the old owned data freed) unless it is
    /// that size, the text ended at `n` (or zeroed from the old size
    /// up), `4 n` vertices in the streams at `[+0x64]` (position and
    /// texture coordinates, semantic 1 both, and a colour stream when
    /// byte `+0x174`), the index block at `+0x68` set to `6 n` 16-bit
    /// indices of vertices `0..4n - 1` and filled with two triangles per
    /// character (`v, v+1, v+2, v, v+2, v+3`) through the pointer its map
    /// leaves in `frame` (the original's `ebp - 0x10`), then unmapped at
    /// the next version.
    ///
    /// The second result: the `n` argument slot's last value (`4 n`) once
    /// the original reuses it.
    pub fn set_capacity(h: &mut impl Host, text: u32, frame: u32, n: u32) -> (u32, Option<u32>) {
        let need = n.wrapping_add(1);
        let head: u32 = h.read(text + 0x15c);
        let cap = head & 0x7fff_ffff;
        if need != head {
            let keep = need.min(cap);
            let mut src: u32 = h.read(text + 0x160);
            let mut block = 0;
            if need != 0 {
                block = h.call_sym(MALLOC, Conv::Cdecl, &[need, 1]);
                if block == 0 {
                    return (0xd, None);
                }
                for k in 0..keep {
                    let dst = block.wrapping_add(k);
                    if dst != 0 {
                        let b: u8 = h.read(src);
                        h.write(dst, b);
                        src = src.wrapping_add(1);
                    }
                }
            }
            let old: u32 = h.read(text + 0x160);
            if old != block && h.read::<u32>(text + 0x15c) >> 31 == 0 && old != 0 {
                h.call_sym(FREE, Conv::Cdecl, &[old]);
            }
            h.write(text + 0x160, block);
            h.write(text + 0x15c, need);
        }
        let chars: u32 = h.read(text + 0x160);
        if n < cap {
            h.write(chars.wrapping_add(n), 0u8);
        } else {
            h.call_sym(MEMSET, Conv::Cdecl, &[chars.wrapping_add(cap), 0, n - cap]);
        }
        let vertices = n.wrapping_mul(4);
        let indices = n.wrapping_mul(6);
        let slot = Some(vertices);
        let streams: u32 = h.read(text + 0x64);
        let r = stream(h, streams, 8, vertices, 1, NAME_POSITION);
        if r != 0 {
            return (r, slot);
        }
        let streams: u32 = h.read(text + 0x64);
        let r = stream(h, streams.wrapping_add(0x40), 8, vertices, 1, NAME_TEXCOORD);
        if r != 0 {
            return (r, slot);
        }
        if h.read::<u8>(text + 0x174) != 0 {
            let streams: u32 = h.read(text + 0x64);
            let r = stream(h, streams.wrapping_add(0x80), 0x10, vertices, 3, NAME_COLOR);
            if r != 0 {
                return (r, slot);
            }
        }
        let ib = text + 0x68;
        h.write(ib + 8, indices);
        h.write(ib + 0xc, 0xcu8);
        h.write(ib, 0u32);
        h.write(ib + 4, vertices.wrapping_sub(1));
        let size = h.call_sym(FORMAT_SIZE, Conv::Cdecl, &[0xc]);
        let bytes = size.wrapping_mul(indices);
        if h.read::<u32>(ib + 0x34) != bytes {
            if h.call_sym(INDEX_IS_BOUND, Conv::Thiscall(ib), &[]) & 0xff != 0 {
                h.call_sym(INDEX_RELEASE, Conv::Thiscall(ib), &[]);
            }
            h.write(ib + 0x34, bytes);
        }
        let r = h.call_sym(INDEX_MAP, Conv::Thiscall(ib), &[frame]);
        if r != 0 {
            return (r, slot);
        }
        let p: u32 = h.read(frame);
        if p != 0 && indices != 0 {
            let quads = (indices - 1) / 6 + 1;
            for q in 0..quads {
                let v = q.wrapping_mul(4) as u16;
                let at = p.wrapping_add(12 * q);
                let tri = [
                    v,
                    v.wrapping_add(1),
                    v.wrapping_add(2),
                    v,
                    v.wrapping_add(2),
                    v.wrapping_add(3),
                ];
                for (k, &i) in tri.iter().enumerate() {
                    h.write(at.wrapping_add(2 * k as u32), i);
                }
            }
        }
        let mut version = h.read::<u32>(ib + 0x10).wrapping_add(1);
        if version == h.read::<u32>(ib + 0x14) & 0x7fff_ffff {
            version = 0;
        }
        let r = h.call_sym(INDEX_UNMAP, Conv::Thiscall(ib), &[version, frame]);
        if r == 0 {
            h.write(ib + 0x10, version);
            h.write(ib + 0xe, 0u8);
        }
        (r, slot)
    }
}

/// `PApplication`'s constructor: its vtable and the thread-base word
/// (`0x00b31404`), the members' constructors (the defaults `+0x50`
/// `0x00620780`, the renderer `+0x124` `PRenderer::PRenderer(r; defaults)`,
/// the dynamic process buffer allocation `+0x2a0`, the world `+0x2c4`,
/// the input mapper `+0x2dc`, all thiscall), the title `PString::Assign
/// (+0x398; "...")`, `g_PhyreApplication`, and the module objects whose
/// virtual slot 1 runs unless the "initialised" byte is set.
pub const APP_VTABLE: u32 = 0x00b3_fe38;
pub const APP_THREAD_WORD: u32 = 0x00b3_1404;
pub const APP_DEFAULTS_CTOR: u32 = 0x0062_0780;
pub const RENDERER_CTOR: u32 = 0x005b_45b0;
pub const DYNAMIC_ALLOCATION_CTOR: u32 = 0x004f_ee50;
pub const WORLD_CTOR: u32 = 0x0044_29e0;
pub const INPUT_MAPPER_CTOR: u32 = 0x0062_0590;
pub const STRING_ASSIGN: u32 = 0x0044_5cf0;
pub const APP_TITLE: u32 = 0x00b2_0b30;
pub const G_APPLICATION: u32 = 0x00cc_9cd8;
pub const MODULES_READY: u32 = 0x00c0_a09c;
pub const APP_MODULES: [u32; 7] = [
    0x00c0_a33c,
    0x00c0_a37c,
    0x00c0_a2d0,
    0x00c6_81ec,
    0x00c6_845c,
    0x00c6_13f0,
    0x00c6_8494,
];

pub mod app {
    use super::{
        APP_DEFAULTS_CTOR, APP_MODULES, APP_THREAD_WORD, APP_TITLE, APP_VTABLE, Conv,
        DYNAMIC_ALLOCATION_CTOR, G_APPLICATION, Host, INPUT_MAPPER_CTOR, MODULES_READY,
        RENDERER_CTOR, STRING_ASSIGN, WORLD_CTOR,
    };

    /// `PApplication::PApplication(app)` (thiscall) -> `app`.
    pub fn ctor(h: &mut impl Host, app: u32) -> u32 {
        let vt = h.rebase(APP_VTABLE);
        h.write(app, vt);
        let w = h.rebase(APP_THREAD_WORD);
        h.write(app + 0xc, w);
        for off in [0x20, 0x24, 0x28, 0x30, 0x3c] {
            h.write(app + off, 0.0f32);
        }
        h.write(app + 0x10, u32::MAX);
        for off in [0x14, 0x18, 0x2c, 0x34, 0x40, 0x44, 0x48, 0x4c] {
            h.write(app + off, 0u32);
        }
        h.write(app + 0x1c, 0u16);
        h.write(app + 0x38, 0u16);
        h.call_sym(APP_DEFAULTS_CTOR, Conv::Thiscall(app + 0x50), &[]);
        h.call_sym(RENDERER_CTOR, Conv::Thiscall(app + 0x124), &[app + 0x50]);
        let words: [(u32, u32); 10] = [
            (0x264, 0),
            (0x268, 0),
            (0x26c, 0x500),
            (0x270, 0x2d0),
            (0x274, 0x18),
            (0x278, 8),
            (0x27c, 0x18),
            (0x280, 8),
            (0x284, 0),
            (0x28c, 2),
        ];
        for (off, v) in words {
            h.write(app + off, v);
        }
        h.write(app + 0x288, 1u8);
        h.write(app + 0x290, 0u8);
        h.write(app + 0x294, 0u32);
        h.write(app + 0x298, 0u8);
        h.write(app + 0x29c, 0u32);
        h.call_sym(DYNAMIC_ALLOCATION_CTOR, Conv::Thiscall(app + 0x2a0), &[]);
        h.write(app + 0x2bc, 0u32);
        h.write(app + 0x2c0, 0u32);
        h.call_sym(WORLD_CTOR, Conv::Thiscall(app + 0x2c4), &[]);
        h.write(app + 0x2d4, 0u32);
        h.write(app + 0x2d8, 0u32);
        h.call_sym(INPUT_MAPPER_CTOR, Conv::Thiscall(app + 0x2dc), &[]);
        h.write(app + 0x398, 0u32);
        let g = h.rebase(G_APPLICATION);
        h.write(g, app);
        h.write(app + 0x2b8, app + 0x2dc);
        h.write(app + 8, 5u32);
        h.write(app + 4, 4u32);
        let title = h.rebase(APP_TITLE);
        h.call_sym(STRING_ASSIGN, Conv::Thiscall(app + 0x398), &[title]);
        h.write(app + 0x39c, 0x7f00u32);
        for (off, v) in [
            (0x2a0, 0xc0_0000u32),
            (0x2a4, 0x18_0000),
            (0x2a8, 0xc0_0000),
            (0x2ac, 0x18_0000),
        ] {
            h.write(app + off, v);
        }
        let ready = h.rebase(MODULES_READY);
        for m in APP_MODULES {
            if h.read::<u8>(ready) != 0 {
                break;
            }
            let m = h.rebase(m);
            h.call_virtual(m, 1, &[]);
        }
        app
    }
}

/// `PWindowWin32Base::createWindow`'s imports (stdcall through their
/// slots), its window procedure, class name, the globals it sets and
/// reads, the device-interface class GUID and the warning print.
pub const GET_MODULE_HANDLE: u32 = 0x00b0_c114;
pub const LOAD_ICON: u32 = 0x00b0_c65c;
pub const LOAD_CURSOR: u32 = 0x00b0_c660;
pub const REGISTER_CLASS: u32 = 0x00b0_c6b8;
pub const CHANGE_DISPLAY_SETTINGS: u32 = 0x00b0_c658;
pub const ADJUST_WINDOW_RECT: u32 = 0x00b0_c6cc;
pub const CREATE_WINDOW_EX: u32 = 0x00b0_c6ec;
pub const SET_WINDOW_LONG: u32 = 0x00b0_c6bc;
pub const REGISTER_DEVICE_NOTIFICATION: u32 = 0x00b0_c6a8;
pub const WND_PROC: u32 = 0x0062_5130;
pub const CLASS_NAME: u32 = 0x00b4_0d3c;
pub const G_ICON: u32 = 0x00cc_9cec;
pub const G_HOSTED: u32 = 0x00cc_9cd2;
pub const G_HOST_HWND: u32 = 0x00cc_9ce0;
pub const G_BORDERLESS: u32 = 0x00cc_9cd1;
pub const DEVICE_GUID: u32 = 0x00b6_950c;
pub const IME_DISASSOCIATE: u32 = 0x009d_a254;
pub const NOTIFY_FAILED: u32 = 0x00b4_0d50;
pub const WARN: u32 = 0x0043_52f0;

pub mod window {
    use super::{
        ADJUST_WINDOW_RECT, CHANGE_DISPLAY_SETTINGS, CLASS_NAME, CREATE_WINDOW_EX, Conv,
        DEVICE_GUID, G_APPLICATION, G_BORDERLESS, G_HOST_HWND, G_HOSTED, G_ICON, GET_CLIENT_RECT,
        GET_MODULE_HANDLE, HWND, Host, IME_DISASSOCIATE, LOAD_CURSOR, LOAD_ICON, MEMCPY, MEMSET,
        NOTIFY_FAILED, REGISTER_CLASS, REGISTER_DEVICE_NOTIFICATION, SET_WINDOW_LONG, WARN,
        WND_PROC,
    };

    fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Stdcall, args)
    }

    /// The original's frame from `ebp - 0x110` (`FRAME` bytes).
    pub const FRAME: u32 = 0x110;

    fn strlen(h: &impl Host, p: u32) -> u32 {
        let mut n = 0u32;
        while h.read::<u8>(p.wrapping_add(n)) != 0 {
            n = n.wrapping_add(1);
        }
        n
    }

    /// `PWindowWin32Base::createWindow(window; suffix)` (thiscall, `ret
    /// 4`) -> 0: the class `"PhyreFrameworkClass"` registered (the
    /// application's icon `+0x39c` into `0x00cc9cec`, the arrow cursor,
    /// background brush 6), fullscreen (byte `+0x2c` of the settings at
    /// `[window]`) switches the display mode (32 bits, `CDS_FULLSCREEN`);
    /// the style is `WS_POPUP` when hosted (`0x00cc9cd2` with a client
    /// rectangle), borderless (`0x00cc9cd1`) or fullscreen, else
    /// `WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX` sized by
    /// `AdjustWindowRect`; the title (the application's `+0x398`, plus
    /// `suffix`, concatenated in `alloca(len)`'s buffer); the window is
    /// the settings' `+0x28` when set, else created (`CreateWindowExA`,
    /// 12 words: called as a thiscall with an unused `ecx`, which pops
    /// the same) and the IME detached; `GWL_USERDATA` the window object;
    /// device-interface notifications registered (a warning when that
    /// fails). `frame` is the original's `ebp - 0x110`.
    #[allow(clippy::too_many_lines)]
    pub fn create(
        h: &mut impl Host,
        window: u32,
        frame: u32,
        suffix: u32,
        alloca: &mut impl FnMut(u32) -> u32,
    ) -> u32 {
        let l = |off: i32| frame.wrapping_add((0x110 + off) as u32);
        h.write(l(-0xdc), suffix);
        let app: u32 = h.read(h.rebase(G_APPLICATION));
        let icon: u32 = h.read(app.wrapping_add(0x39c));
        h.write(l(-0xe8), window);
        h.write(l(-0x110), 0u32);
        let proc_ = h.rebase(WND_PROC);
        h.write(l(-0x10c), proc_);
        h.write(l(-0x108), 0u32);
        h.write(l(-0x104), 0u32);
        let module = import(h, GET_MODULE_HANDLE, &[0]);
        h.write(l(-0x100), module);
        let hicon = import(h, LOAD_ICON, &[module, icon & 0xffff]);
        let g_icon = h.rebase(G_ICON);
        h.write(g_icon, hicon);
        h.write(l(-0xfc), hicon);
        let cursor = import(h, LOAD_CURSOR, &[0, 0x7f00]);
        h.write(l(-0xf8), cursor);
        h.write(l(-0xf4), 6u32);
        h.write(l(-0xf0), 0u32);
        let class = h.rebase(CLASS_NAME);
        h.write(l(-0xec), class);
        import(h, REGISTER_CLASS, &[l(-0x110)]);
        let settings: u32 = h.read(window);
        let fullscreen = h.read::<u8>(settings.wrapping_add(0x2c)) != 0;
        let w: u32 = h.read(settings);
        let ht: u32 = h.read(settings.wrapping_add(4));
        h.write(l(-0xd8), w);
        h.write(l(-0xe0), ht);
        if fullscreen {
            h.call_sym(MEMSET, Conv::Cdecl, &[l(-0xd0), 0, 0x9c]);
            h.write(l(-0xac), 0x9cu16);
            h.write(l(-0x64), w);
            h.write(l(-0x60), ht);
            h.write(l(-0x68), 0x20u32);
            h.write(l(-0xa8), 0x001c_0000u32);
            import(h, CHANGE_DISPLAY_SETTINGS, &[l(-0xd0), 4]);
        }
        let hosted = h.rebase(G_HOSTED);
        let mut adjust = true;
        let mut style = 0;
        if h.read::<u8>(hosted) != 0 {
            let host: u32 = h.read(h.rebase(G_HOST_HWND));
            if import(h, GET_CLIENT_RECT, &[host, l(-0x14)]) == 0 {
                h.write(hosted, 0u8);
            } else if h.read::<u8>(hosted) != 0 {
                style = 0x8000_0000;
                h.write(l(-0xd4), style);
                adjust = false;
            }
        }
        if adjust {
            style = if h.read::<u8>(h.rebase(G_BORDERLESS)) != 0 || fullscreen {
                0x8000_0000
            } else {
                0x00ca_0000
            };
            h.write(l(-0xd4), style);
            h.write(l(-0x14), 0u32);
            h.write(l(-0x10), 0u32);
            h.write(l(-0xc), w);
            h.write(l(-8), ht);
            import(h, ADJUST_WINDOW_RECT, &[l(-0x14), style, 0]);
            let w = h.read::<u32>(l(-0xc)).wrapping_sub(h.read(l(-0x14)));
            let ht = h.read::<u32>(l(-8)).wrapping_sub(h.read(l(-0x10)));
            h.write(l(-0xd8), w);
            h.write(l(-0xe0), ht);
        }
        let app: u32 = h.read(h.rebase(G_APPLICATION));
        let raw: u32 = h.read(app.wrapping_add(0x398));
        let mut title = raw.wrapping_sub(raw & 1);
        h.write(l(-0xe4), title);
        if title != 0 && suffix != 0 {
            let a = strlen(h, title);
            let b = strlen(h, suffix);
            let buf = alloca(a.wrapping_add(1).wrapping_add(b));
            h.call_sym(MEMCPY, Conv::Cdecl, &[buf, title, a]);
            h.call_sym(
                MEMCPY,
                Conv::Cdecl,
                &[buf.wrapping_add(a), suffix, b.wrapping_add(1)],
            );
            title = buf;
        }
        let g_hwnd = h.rebase(HWND);
        let given: u32 = h.read(settings_of(h, window).wrapping_add(0x28));
        let hwnd = if given != 0 {
            h.write(g_hwnd, given);
            given
        } else {
            let module = import(h, GET_MODULE_HANDLE, &[0]);
            let w: u32 = h.read(l(-0xd8));
            let ht: u32 = h.read(l(-0xe0));
            let f: u32 = h.read(h.rebase(CREATE_WINDOW_EX));
            let hwnd = h.call(
                f,
                Conv::Thiscall(0),
                &[
                    0,
                    class,
                    title,
                    style | 0x0600_0000,
                    0x8000_0000,
                    0x8000_0000,
                    w,
                    ht,
                    0,
                    0,
                    module,
                    window,
                ],
            );
            h.write(g_hwnd, hwnd);
            if hwnd != 0 {
                h.call_sym(IME_DISASSOCIATE, Conv::Stdcall, &[hwnd, 0]);
            }
            h.read(g_hwnd)
        };
        import(h, SET_WINDOW_LONG, &[hwnd, 0xffff_ffeb, window]);
        for off in [-0x34, -0x30, -0x28, -0x24, -0x20, -0x1c, -0x2c, -0x18] {
            h.write(l(off), 0u32);
        }
        let guid = h.rebase(DEVICE_GUID);
        for k in 0..4u32 {
            let v: u32 = h.read(guid + 4 * k);
            h.write(l(-0x28) + 4 * k, v);
        }
        h.write(l(-0x34), 0x20u32);
        h.write(l(-0x30), 5u32);
        let hwnd: u32 = h.read(g_hwnd);
        if import(h, REGISTER_DEVICE_NOTIFICATION, &[hwnd, l(-0x34), 4]) == 0 {
            let text = h.rebase(NOTIFY_FAILED);
            h.call_sym(WARN, Conv::Cdecl, &[3, text]);
        }
        0
    }

    fn settings_of(h: &impl Host, window: u32) -> u32 {
        h.read(window)
    }
}

/// The window procedure's imports (user32/shell32/kernel32, stdcall
/// through their slots), globals and callees.
pub const REGISTER_WINDOW_MESSAGE: u32 = 0x00b0_c68c;
pub const GET_WINDOW_LONG: u32 = 0x00b0_c6c0;
pub const SHOW_WINDOW: u32 = 0x00b0_c6e4;
pub const DESTROY_WINDOW: u32 = 0x00b0_c6e8;
pub const GET_MODULE_FILE_NAME: u32 = 0x00b0_c1a4;
pub const PATH_REMOVE_FILE_SPEC: u32 = 0x00b0_c638;
pub const PATH_COMBINE: u32 = 0x00b0_c63c;
pub const SHELL_EXECUTE: u32 = 0x00b0_c630;
pub const POST_QUIT_MESSAGE: u32 = 0x00b0_c6b4;
pub const DEF_WINDOW_PROC: u32 = 0x00b0_c6b0;
pub const GET_CURRENT_THREAD_ID: u32 = 0x00b0_c108;
pub const SET_WINDOWS_HOOK: u32 = 0x00b0_c698;
pub const UNHOOK_WINDOWS_HOOK: u32 = 0x00b0_c668;
pub const SHOW_CURSOR: u32 = 0x00b0_c688;
pub const MESSAGE_BOX: u32 = 0x00b0_c684;
pub const LOCAL_FREE: u32 = 0x00b0_c61c;
/// `"PHYREMSG_HOSTHWND"`'s message id (registered once), the launcher's
/// file name, the Alt+F4 box's caption, the mouse hook procedure.
pub const G_HOST_MESSAGE: u32 = 0x00cc_a308;
pub const HOST_MESSAGE_NAME: u32 = 0x00b4_0d78;
pub const LAUNCHER_NAME: u32 = 0x00b4_0d8c;
pub const QUIT_CAPTION: u32 = 0x00b4_0db8;
pub const MOUSE_HOOK_PROC: u32 = 0x0062_50e0;
pub const G_HOST_LPARAM: u32 = 0x00cc_9ce4;
pub const G_DEVICE_ARRIVED: u32 = 0x00cc_9cd0;
pub const G_DRAGGING: u32 = 0x00cc_9cd3;
pub const G_MOUSE_HOOK: u32 = 0x00cc_9cf0;
/// The host message's handler `0x00623d50(hwnd host, hwnd)` (cdecl), the
/// window's close `0x006270a0(window)` and `setFocus(window; bool)`
/// (thiscall), the cursor-release pair `0x00627850`/`0x006278b0`, the
/// resource string `0x0061b330(&text; id)` (thiscall).
pub const ON_HOST_MESSAGE: u32 = 0x0062_3d50;
pub const ON_CLOSE: u32 = 0x0062_70a0;
pub const SET_FOCUS: u32 = 0x0062_a460;
pub const CURSOR_ENTER: u32 = 0x0062_7850;
pub const CURSOR_LEAVE: u32 = 0x0062_78b0;
pub const LOAD_STRING: u32 = 0x0061_b330;

pub mod wndproc {
    use super::{
        CURSOR_ENTER, CURSOR_LEAVE, Conv, DEF_WINDOW_PROC, DESTROY_WINDOW, G_APPLICATION,
        G_DEVICE_ARRIVED, G_DRAGGING, G_HOST_HWND, G_HOST_LPARAM, G_HOST_MESSAGE, G_HOSTED,
        G_MOUSE_HOOK, GET_CLIENT_RECT, GET_CURRENT_THREAD_ID, GET_MODULE_FILE_NAME,
        GET_WINDOW_LONG, HOST_MESSAGE_NAME, HWND, Host, LAUNCHER_NAME, LOAD_STRING, LOCAL_FREE,
        MESSAGE_BOX, MOUSE_HOOK_PROC, ON_CLOSE, ON_HOST_MESSAGE, PATH_COMBINE,
        PATH_REMOVE_FILE_SPEC, POST_QUIT_MESSAGE, QUIT_CAPTION, REGISTER_WINDOW_MESSAGE, SET_FOCUS,
        SET_WINDOWS_HOOK, SHELL_EXECUTE, SHOW_CURSOR, SHOW_WINDOW, UNHOOK_WINDOWS_HOOK,
    };

    /// The original's frame from `ebp - 0x43c` (`FRAME` bytes).
    pub const FRAME: u32 = 0x42c;

    fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Stdcall, args)
    }

    fn default(h: &mut impl Host, args: [u32; 4]) -> u32 {
        import(h, DEF_WINDOW_PROC, &args)
    }

    /// `PWindowWin32Base`'s window procedure `(hwnd, msg, wparam, lparam)`
    /// (stdcall) -> the message's result. The window object is the
    /// creation parameter (`WM_CREATE`) or `GWL_USERDATA`; without it,
    /// `DefWindowProcA`. The registered `PHYREMSG_HOSTHWND` makes the
    /// window hosted (once). Handled: `WM_CREATE` (`g_hWnd`, shown),
    /// `WM_DESTROY` (quit), `WM_SIZE` (the client size into the
    /// application unless minimised), `WM_ACTIVATE` (focus),
    /// `WM_CLOSE` (closes, destroys, starts the launcher next to the
    /// exe), `WM_ERASEBKGND`, `WM_GETMINMAXINFO` (max track 64K),
    /// non-client button down/up (a mouse hook while dragging), Alt+F4
    /// (a confirmation box), F10 and Alt swallowed, `WM_DEVICECHANGE`
    /// arrivals flagged. `frame` is the original's `ebp - 0x43c`.
    #[allow(clippy::too_many_lines)]
    pub fn wnd_proc(h: &mut impl Host, frame: u32, args: [u32; 4]) -> u32 {
        let [hwnd, msg, wparam, lparam] = args;
        let l = |off: i32| frame.wrapping_add((0x43c + off) as u32);
        let g_message = h.rebase(G_HOST_MESSAGE);
        h.write(l(-0x43c), hwnd);
        h.write(l(-0x434), lparam);
        if h.read::<u32>(g_message) == 0 {
            let name = h.rebase(HOST_MESSAGE_NAME);
            let id = import(h, REGISTER_WINDOW_MESSAGE, &[name]);
            h.write(g_message, id);
        }
        if hwnd == 0 {
            return 1;
        }
        let window = if msg == 1 {
            h.read(lparam)
        } else {
            import(h, GET_WINDOW_LONG, &[hwnd, 0xffff_ffeb])
        };
        let host_message: u32 = h.read(g_message);
        h.write(l(-0x438), window);
        if window == 0 {
            return default(h, args);
        }
        let hosted = h.rebase(G_HOSTED);
        if msg == host_message && h.read::<u8>(hosted) == 0 {
            h.write(hosted, 1u8);
            let host = h.rebase(G_HOST_HWND);
            h.write(host, wparam);
            let lp = h.rebase(G_HOST_LPARAM);
            h.write(lp, lparam);
            h.call_sym(ON_HOST_MESSAGE, Conv::Cdecl, &[wparam, hwnd]);
        }
        match msg {
            1 => {
                let g = h.rebase(HWND);
                h.write(g, hwnd);
                import(h, SHOW_WINDOW, &[hwnd, 1]);
                1
            }
            2 => {
                import(h, POST_QUIT_MESSAGE, &[0]);
                1
            }
            5 => {
                if wparam != 1 {
                    import(h, GET_CLIENT_RECT, &[hwnd, l(-0x430)]);
                    let app: u32 = h.read(h.rebase(G_APPLICATION));
                    let bottom: u32 = h.read(l(-0x424));
                    let right: u32 = h.read(l(-0x428));
                    h.write(app.wrapping_add(0x26c), right);
                    h.write(app.wrapping_add(0x270), bottom);
                    h.write(app.wrapping_add(0x14), 1u8);
                }
                1
            }
            6 => {
                let state = wparam as u16;
                let active = u32::from(state == 1 || state == 2);
                h.call_sym(SET_FOCUS, Conv::Thiscall(window), &[active]);
                1
            }
            0x10 => {
                h.call_sym(ON_CLOSE, Conv::Thiscall(window), &[]);
                import(h, DESTROY_WINDOW, &[hwnd]);
                import(h, GET_MODULE_FILE_NAME, &[0, l(-0x218), 0x104]);
                import(h, PATH_REMOVE_FILE_SPEC, &[l(-0x218)]);
                let name = h.rebase(LAUNCHER_NAME);
                import(h, PATH_COMBINE, &[l(-0x420), l(-0x218), name]);
                import(h, SHELL_EXECUTE, &[0, 0, l(-0x420), 0, 0, 1]);
                1
            }
            0x14 => {
                if h.read::<u8>(hosted) == 0 && wparam == 0x8000 {
                    let g = h.rebase(G_DEVICE_ARRIVED);
                    h.write(g, 1u8);
                }
                1
            }
            0x24 => {
                let r = default(h, args);
                h.write(l(-0x438), r);
                h.write(lparam.wrapping_add(0x20), 0x1_0000u32);
                h.write(lparam.wrapping_add(0x24), 0x1_0000u32);
                r
            }
            0xa1 | 0xa4 => {
                let dragging = h.rebase(G_DRAGGING);
                if h.read::<u8>(dragging) == 0 {
                    h.write(dragging, 1u8);
                    h.call_sym(CURSOR_ENTER, Conv::Cdecl, &[]);
                    let thread = import(h, GET_CURRENT_THREAD_ID, &[]);
                    let proc_ = h.rebase(MOUSE_HOOK_PROC);
                    let hook = import(h, SET_WINDOWS_HOOK, &[7, proc_, 0, thread]);
                    let g = h.rebase(G_MOUSE_HOOK);
                    h.write(g, hook);
                }
                default(h, args)
            }
            0xa2 | 0xa5 => {
                let dragging = h.rebase(G_DRAGGING);
                if h.read::<u8>(dragging) != 0 {
                    let g = h.rebase(G_MOUSE_HOOK);
                    let hook: u32 = h.read(g);
                    h.write(dragging, 0u8);
                    import(h, UNHOOK_WINDOWS_HOOK, &[hook]);
                    h.write(g, 0u32);
                    h.call_sym(CURSOR_LEAVE, Conv::Cdecl, &[]);
                }
                default(h, args)
            }
            0x104 | 0x105 => match wparam {
                0x73 if lparam & 0x2000_0000 != 0 => {
                    h.write(l(-0x438), 0u32);
                    h.call_sym(LOAD_STRING, Conv::Thiscall(l(-0x438)), &[0x78ac]);
                    let text: u32 = h.read(l(-0x438));
                    let mut confirmed = false;
                    if msg == 0x104 {
                        h.call_sym(CURSOR_ENTER, Conv::Cdecl, &[]);
                        import(h, SHOW_CURSOR, &[1]);
                        let caption = h.rebase(QUIT_CAPTION);
                        let r = import(h, MESSAGE_BOX, &[hwnd, text, caption, 0x31]);
                        h.call_sym(CURSOR_LEAVE, Conv::Cdecl, &[]);
                        import(h, SHOW_CURSOR, &[0]);
                        confirmed = r == 1;
                    }
                    import(h, LOCAL_FREE, &[text]);
                    if confirmed { default(h, args) } else { 1 }
                }
                0x79 | 0x12 => 1,
                _ => default(h, args),
            },
            0x219 => {
                if wparam == 0x8000 {
                    let g = h.rebase(G_DEVICE_ARRIVED);
                    h.write(g, 1u8);
                }
                1
            }
            _ => default(h, args),
        }
    }
}
