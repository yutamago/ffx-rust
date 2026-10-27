//! Continuation round 9 of list `h4-glue-other`, batch 2: the text
//! utility's line append.

use ffx_bridge::x87::{add, div, load, mul, store, sub};
use ffx_graphic::host::{Conv, Host};

use crate::h4_e1::pass;

/// `Phyre_PhyreMalloc_0042fb80(size, align)` (cdecl).
pub const MALLOC: u32 = 0x0042_fb80;
/// `Phyre_PhyreFree(p)` (`0x0042fb20`, cdecl).
pub const FREE: u32 = 0x0042_fb20;
/// `memcpy` through its import thunk (`0x00949256`, cdecl dst, src, n).
pub const MEMCPY: u32 = 0x0094_9256;
/// `Phyre_PArray_PString_pushInto_005d6b00(array; block, &string)`
/// (thiscall, ret 8).
pub const PUSH_INTO: u32 = 0x005d_6b00;
/// `Phyre_PStringBuilder_operator(builder; text)` (`0x00444bf0`,
/// thiscall, ret 4).
pub const BUILDER_APPEND: u32 = 0x0044_4bf0;
/// `Phyre_PText_PBitmapFontText_setText_00a2ad80(text; line)` (thiscall,
/// ret 4).
pub const SET_TEXT: u32 = 0x00a2_ad80;
/// `TextUtil_scrollToVisible(this; 1)` (`0x006f6860`, thiscall, ret 4).
pub const SCROLL_TO_VISIBLE: u32 = 0x006f_6860;
/// The empty string (`0x00b8fbeb`): an empty line's text.
pub const EMPTY: u32 = 0x00b8_fbeb;
/// The builder's first text (`0x00b7cae0`, another empty string).
pub const BUILDER_START: u32 = 0x00b7_cae0;
/// Bytes of [`append_line`]'s frame: the new line's string word (the
/// original's own `text` argument slot, whose address goes to
/// `pushInto`).
pub const APPEND_FRAME: u32 = 4;

/// A `PString`'s text: the word without its low "not owned" bit.
fn text_of(h: &impl Host, slot: u32) -> u32 {
    let word: u32 = h.read(slot);
    word.wrapping_sub(u32::from(h.read::<u8>(slot) & 1))
}

/// Whether the string at `a` equals the one at `b` (the inlined `strcmp`).
fn same_text(h: &impl Host, a: u32, b: u32) -> bool {
    let mut k = 0u32;
    loop {
        let x: u8 = h.read(a.wrapping_add(k));
        if x != h.read::<u8>(b.wrapping_add(k)) {
            return false;
        }
        if x == 0 {
            return true;
        }
        k = k.wrapping_add(1);
    }
}

/// A heap copy of the string `text` (`PhyreMalloc(len + 1, 2)` and
/// `memcpy`); 0 for no text or no memory.
fn copy_text(h: &mut impl Host, text: u32) -> u32 {
    if text == 0 {
        return 0;
    }
    let mut n = 0u32;
    while h.read::<u8>(text.wrapping_add(n)) != 0 {
        n = n.wrapping_add(1);
    }
    let size = n.wrapping_add(1);
    let copy = h.call_sym(MALLOC, Conv::Cdecl, &[size, 2]);
    if copy != 0 {
        h.call_sym(MEMCPY, Conv::Cdecl, &[copy, text, size]);
    }
    copy
}

/// `TextUtil_appendLine(this; text)` (`0x006f5e30`, thiscall, ret 4;
/// `frame` see [`APPEND_FRAME`]; the SEH frame is not kept): the first
/// empty line of the array `+0x74` (count, top bit masked) / `+0x78`
/// (strings) gets a heap copy of `text` (its old string freed when
/// owned); without one the copy is pushed as a new line (`pushInto` with a
/// block of count + 1 words; the copy freed afterwards, also when 0). The
/// builder `+0x14` gets [`BUILDER_START`] and the line; for a new line the
/// bitmap text `+0x10` (if any) is set to line `+0x1c` - 1 (0 for 0) and
/// the view scrolls to it. Returns the last callee's `eax`.
pub fn append_line(h: &mut impl Host, frame: u32, this: u32, text: u32) -> u32 {
    let at = |off: u32| this.wrapping_add(off);
    // the slot holds the argument until the new line's string replaces it
    h.write(frame, text);
    let empty = h.rebase(EMPTY);
    let start = h.rebase(BUILDER_START);
    let count = h.read::<u32>(at(0x74)) & 0x7fff_ffff;
    if count != 0 {
        let slots: u32 = h.read(at(0x78));
        for i in 0..count {
            let slot = slots.wrapping_add(4 * i);
            let line = text_of(h, slot);
            if line == empty || (line != 0 && same_text(h, line, empty)) {
                let copy = copy_text(h, text);
                // a failed copy leaves the old string
                if text == 0 || copy != 0 {
                    if h.read::<u8>(slot) & 1 == 0 {
                        let old: u32 = h.read(slot);
                        h.call_sym(FREE, Conv::Cdecl, &[old]);
                    }
                    h.write(slot, copy);
                }
                h.call_sym(BUILDER_APPEND, Conv::Thiscall(at(0x14)), &[start]);
                let slots: u32 = h.read(at(0x78));
                let line = text_of(h, slots.wrapping_add(4 * i));
                return h.call_sym(BUILDER_APPEND, Conv::Thiscall(at(0x14)), &[line]);
            }
        }
    }
    let copy = copy_text(h, text);
    h.write(frame, copy);
    let n = h.read::<u32>(at(0x74)) & 0x7fff_ffff;
    let block = h.call_sym(MALLOC, Conv::Cdecl, &[n.wrapping_mul(4).wrapping_add(4), 4]);
    h.call_sym(PUSH_INTO, Conv::Thiscall(at(0x74)), &[block, frame]);
    if copy & 1 == 0 {
        h.call_sym(FREE, Conv::Cdecl, &[copy]);
    }
    h.call_sym(BUILDER_APPEND, Conv::Thiscall(at(0x14)), &[start]);
    let slots: u32 = h.read(at(0x78));
    let line = text_of(h, slots.wrapping_add(4 * count));
    h.call_sym(BUILDER_APPEND, Conv::Thiscall(at(0x14)), &[line]);
    let bitmap: u32 = h.read(at(0x10));
    if bitmap != 0 {
        let line = h.read::<u32>(at(0x1c)).saturating_sub(1);
        h.call_sym(SET_TEXT, Conv::Thiscall(bitmap), &[line]);
    }
    h.call_sym(SCROLL_TO_VISIBLE, Conv::Thiscall(this), &[1])
}

// ---- RemapButtonIcon_remapIconPosition

/// `g_RemapInputCountdown_01841d24`: picks the 0x40 button's shift.
pub const REMAP_COUNTDOWN: u32 = 0x0184_1d24;
/// The shifts (f64 image constants): 3 (`0x00b2c5c8`), 5 (`0x00b41d20`),
/// 19 (`0x00b4b478`), 3.9 (`0x00b4b450`), 13 (`0x00b4b470`), 6.5
/// (`0x00b4b460`), 4 (`0x00b43d80`), 2.5 (`0x00b43d70`), 2
/// (`0x00b92270`), 0.5 (`0x00b92220`), -7 (`0x00b4b488`), -5
/// (`0x00b4b480`).
pub const SHIFT_3: u32 = 0x00b2_c5c8;
/// See [`SHIFT_3`].
pub const SHIFT_5: u32 = 0x00b4_1d20;
/// See [`SHIFT_3`].
pub const SHIFT_19: u32 = 0x00b4_b478;
/// See [`SHIFT_3`].
pub const SHIFT_3_9: u32 = 0x00b4_b450;
/// See [`SHIFT_3`].
pub const SHIFT_13: u32 = 0x00b4_b470;
/// See [`SHIFT_3`].
pub const SHIFT_6_5: u32 = 0x00b4_b460;
/// See [`SHIFT_3`].
pub const SHIFT_4: u32 = 0x00b4_3d80;
/// See [`SHIFT_3`].
pub const SHIFT_2_5: u32 = 0x00b4_3d70;
/// See [`SHIFT_3`].
pub const SHIFT_2: u32 = 0x00b9_2270;
/// See [`SHIFT_3`].
pub const SHIFT_0_5: u32 = 0x00b9_2220;
/// See [`SHIFT_3`].
pub const SHIFT_MINUS_7: u32 = 0x00b4_b488;
/// See [`SHIFT_3`].
pub const SHIFT_MINUS_5: u32 = 0x00b4_b480;

/// One coordinate change: `+ c` or `- c`, `c` an image f64 or 0 (`fldz`).
#[derive(Clone, Copy)]
enum Shift {
    Add(Option<u32>),
    Sub(Option<u32>),
}

/// Applies `shifts` (offset in the quad, change) in order: each float
/// loaded, changed at 53 bits and stored as f32.
fn shift(h: &mut impl Host, quad: u32, shifts: &[(u32, Shift)]) {
    for &(off, s) in shifts {
        let at = quad.wrapping_add(off);
        let x = load(h.read(at));
        let v = match s {
            Shift::Add(sym) => add(x, shift_value(h, sym)),
            Shift::Sub(sym) => sub(x, shift_value(h, sym)),
        };
        h.write(at, store(v));
    }
}

/// A shift's value: the image f64 at `sym`, or 0.
fn shift_value(h: &impl Host, sym: Option<u32>) -> f64 {
    sym.map_or(0.0, |sym| f64::from_bits(h.read::<u64>(h.rebase(sym))))
}

/// The x of both corners (`+0`, `+0x20`) by `sx`, then the y (`+4`,
/// `+0x24`) by `sy`.
fn shift_xy(h: &mut impl Host, quad: u32, sx: Shift, sy: Shift) {
    shift(h, quad, &[(0, sx), (0x20, sx), (4, sy), (0x24, sy)]);
}

/// `RemapButtonIcon_remapIconPosition(this; kind, quad, button)`
/// (`0x006e6a50`, thiscall, ret 0xc): moves a button icon's quad (f32
/// vertices 0x20 apart, x at `+0`, y at `+4`) by the button's offsets:
/// kind 6 with button 0x41/0x42/0x44/0x48 all four x by -3; 0x20 x and y
/// by +0; 0x41/0x42/0x44/0x48 x -3, y +0; 0x40 by the countdown
/// (3: +5/+0, 2: +19/-3.9, 1: +13/-6.5, 0: +0/-4; each read again after
/// the previous change); 0x38 +2.5/-2; 0x39 +0/-0.5; 0x43 by the
/// machine `+0x150` (3: -0/-0, 2: +7/-4); 0x46, 0x49, 0x4c (3: -2/-0,
/// 2: +5/-4). Returns the `eax` it leaves: `quad` on the 0x40..0x4c
/// paths it takes, else `button`.
pub fn remap_icon_position<H: Host>(
    h: &mut H,
    this: u32,
    kind: u32,
    quad: u32,
    button: u32,
) -> u32 {
    use Shift::{Add, Sub};
    let corner_button = matches!(button, 0x41 | 0x42 | 0x44 | 0x48);
    if kind == 6 && corner_button {
        let s = Sub(Some(SHIFT_3));
        shift(h, quad, &[(0, s), (0x20, s), (0x40, s), (0x60, s)]);
        return quad;
    }
    match button {
        0x20 => shift_xy(h, quad, Add(None), Add(None)),
        _ if corner_button => shift_xy(h, quad, Sub(Some(SHIFT_3)), Add(None)),
        0x40 => {
            let countdown = |h: &mut H| -> u32 { h.read(h.rebase(REMAP_COUNTDOWN)) };
            if countdown(h) == 3 {
                shift_xy(h, quad, Add(Some(SHIFT_5)), Add(None));
            }
            if countdown(h) == 2 {
                shift_xy(h, quad, Add(Some(SHIFT_19)), Sub(Some(SHIFT_3_9)));
            }
            if countdown(h) == 1 {
                shift_xy(h, quad, Add(Some(SHIFT_13)), Sub(Some(SHIFT_6_5)));
            }
            if countdown(h) == 0 {
                shift_xy(h, quad, Add(None), Sub(Some(SHIFT_4)));
            }
        }
        0x38 => shift_xy(h, quad, Add(Some(SHIFT_2_5)), Sub(Some(SHIFT_2))),
        0x39 => shift_xy(h, quad, Add(None), Sub(Some(SHIFT_0_5))),
        0x43 | 0x46 | 0x49 | 0x4c => {
            let machine = |h: &mut H| -> u32 { h.read(this.wrapping_add(0x150)) };
            let (three, two) = if button == 0x43 {
                (Sub(None), SHIFT_MINUS_7)
            } else {
                (Sub(Some(SHIFT_2)), SHIFT_MINUS_5)
            };
            if machine(h) == 3 {
                shift_xy(h, quad, three, Sub(None));
            }
            if machine(h) == 2 {
                shift_xy(h, quad, Sub(Some(two)), Sub(Some(SHIFT_4)));
            }
        }
        _ => return button,
    }
    quad
}

// ---- LoadingScreen_update_006e50a0

/// `Phyre_PFramework_PApplication_GetApplication()` (`0x00623b90`).
pub const GET_APPLICATION: u32 = 0x0062_3b90;
/// `SleepThreadInMillisecond(ms)` (`0x006f2d20`, cdecl).
pub const SLEEP_MS: u32 = 0x006f_2d20;
/// `PhyreScene_getLinearSampler(scene; kind)` (`0x0065e050`, thiscall,
/// ret 4).
pub const LINEAR_SAMPLER: u32 = 0x0065_e050;
/// `ShaderParams_FindByName(shader; name)` (`0x0056cbf0`, thiscall, ret
/// 4).
pub const FIND_BY_NAME: u32 = 0x0056_cbf0;
/// `ShaderParamBuffer_SetValue(buffer; parameter, value)` (`0x00638780`,
/// thiscall, ret 8).
pub const SET_VALUE: u32 = 0x0063_8780;
/// `Phyre_PParameterBuffer_setTextureChecked_0056e670(buffer; parameter,
/// value)` (thiscall, ret 8).
pub const SET_TEXTURE_CHECKED: u32 = 0x0056_e670;
/// `g_PhyreScene` (`0x00ccb9d8`).
pub const PHYRE_SCENE: u32 = 0x00cc_b9d8;
/// `"ColorScale"` (`0x00b47a48`), `"ColorBuffer"` (`0x00b425d8`).
pub const COLOR_SCALE: u32 = 0x00b4_7a48;
/// See [`COLOR_SCALE`].
pub const COLOR_BUFFER: u32 = 0x00b4_25d8;
/// The update's constants: the step limit (f64 `0x00b412e8`, 0.05) and
/// the step used past it (f32 `0x00b47c5c`), the hold time (f32
/// `0x00b3eb58`, 1.5), the fade rate (f64 `0x00b3eb68`, 0.8), the full
/// fade (f32 `0x00b34254`), the scale divisor (f64 `0x00b48320`, 0.6),
/// the frame time (f32 `0x00b47c58` and f64 `0x00b4b378`, 0.04).
pub const STEP_LIMIT: u32 = 0x00b4_12e8;
/// See [`STEP_LIMIT`].
pub const STEP_MAX: u32 = 0x00b4_7c5c;
/// See [`STEP_LIMIT`].
pub const HOLD: u32 = 0x00b3_eb58;
/// See [`STEP_LIMIT`].
pub const FADE_RATE: u32 = 0x00b3_eb68;
/// See [`STEP_LIMIT`].
pub const FADE_FULL: u32 = 0x00b3_4254;
/// See [`STEP_LIMIT`].
pub const SCALE_DIVISOR: u32 = 0x00b4_8320;
/// See [`STEP_LIMIT`].
pub const FRAME_TIME: u32 = 0x00b4_7c58;
/// See [`STEP_LIMIT`].
pub const FRAME_TIME_F64: u32 = 0x00b4_b378;
/// Bytes of [`loading_update`]'s frame, from the original's `ebp -
/// 0x14`: the parameter value (`+0`, 4 words) that the shader calls
/// get, and at `+0x1c` the original's own `dt` argument slot, where it
/// spills its f32 results.
pub const LOADING_FRAME: u32 = 0x20;

/// The first material of mesh instance `instance` (`+0x10` array):
/// {shader parameters, buffer}.
fn first_material(h: &impl Host, instance: u32) -> u32 {
    let materials: u32 = h.read(instance.wrapping_add(0x10));
    let n = h.read::<u32>(materials) & 0x7fff_ffff;
    let list = if n > 1 {
        h.read(materials.wrapping_add(4))
    } else if n != 0 {
        materials.wrapping_add(4)
    } else {
        0
    };
    h.read(list)
}

/// `FindByName(shader +0x30; name)`, then `call(buffer of the first
/// material of instance at this + instance; parameter, value)`; the
/// parameter is not checked.
fn set_parameter(
    h: &mut impl Host,
    this: u32,
    instance: u32,
    name: u32,
    call: u32,
    value: u32,
) -> u32 {
    let instance: u32 = h.read(this.wrapping_add(instance));
    let material = first_material(h, instance);
    let shader: u32 = h.read(this.wrapping_add(0x30));
    let name = h.rebase(name);
    let p = h.call_sym(FIND_BY_NAME, Conv::Thiscall(shader), &[name]);
    let buffer: u32 = h.read(material.wrapping_add(4));
    h.call_sym(call, Conv::Thiscall(buffer), &[p, value])
}

/// The image f64 at `sym`.
fn image_f64(h: &impl Host, sym: u32) -> f64 {
    f64::from_bits(h.read::<u64>(h.rebase(sym)))
}

/// The image f32 at `sym`, as `fld` loads it.
fn image_f32(h: &impl Host, sym: u32) -> f64 {
    load(h.read(h.rebase(sym)))
}

/// Whether either show flag (bytes `+0`, `+1`) is set.
fn shown(h: &impl Host, this: u32) -> bool {
    h.read::<u8>(this) != 0 || h.read::<u8>(this.wrapping_add(1)) != 0
}

/// The loading screen's stop: the times and the fade 0, the state 0,
/// the application's loading flag (`+0x3a0`) cleared.
fn stop(h: &mut impl Host, this: u32) {
    h.write(this.wrapping_add(4), 0u32);
    h.write(this.wrapping_add(0x10), 0u32);
    h.write(this.wrapping_add(8), 0u32);
    h.write(this.wrapping_add(0x14), 0u32);
    let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
    h.write(app.wrapping_add(0x3a0), 0u8);
}

/// `LoadingScreen_update_006e50a0(this; dt)` (thiscall, ret 4, `dt` an
/// f32; `frame` see [`LOADING_FRAME`]; callers read no result): with
/// frames (`+0x20`), a shader (`+0x30`) and a mesh (`+0x2c`): the frame
/// clock `+4` moves by `dt` (at most the step limit), the time `+8` by
/// `dt`. In state 0 (`+0x10`) the fade `+0x14` is 0; the state turns 1
/// when the time passes the hold time; with neither show flag (bytes `+0`,
/// `+1`) the screen stops; while the time stays within the hold time the
/// thread sleeps 140 ms. State 1 fades in by `dt` times the rate (full:
/// state 2); without show flags states 1 and 2 turn 3, which fades out
/// and stops below 0. Otherwise `ColorScale` (1, 1, 1, fade / 0.6) and
/// (1, 1, 1, fade) go to the meshes `+0x38` and `+0x40`, the frame
/// index `+0xc` steps (modulo the frame count) for each frame time the
/// clock holds, and the frame's texture (`+0x24`) goes to `+0x40`'s
/// `ColorBuffer`. NaN comparisons follow the original's flags.
pub fn loading_update(h: &mut impl Host, frame: u32, this: u32, dt: u32) {
    let at = |off: u32| this.wrapping_add(off);
    let slot = frame + 0x1c;
    h.write(slot, dt);
    let frames = h.read::<u32>(at(0x20)) & 0x7fff_ffff;
    if frames == 0 || h.read::<u32>(at(0x2c)) == 0 || h.read::<u32>(at(0x30)) == 0 {
        return;
    }
    let d = load(dt);
    let step = if d < image_f64(h, STEP_LIMIT) {
        pass(dt)
    } else {
        pass(h.read(h.rebase(STEP_MAX)))
    };
    let clock = store(add(load(h.read(at(4))), load(step)));
    h.write(at(4), clock);
    let hold = image_f32(h, HOLD);
    // `fcom` with `test ah, 1`: within the hold unless above it or NaN
    let held = hold >= load(h.read(at(8)));
    let time = store(add(load(h.read(at(8))), d));
    h.write(at(8), time);
    let value = frame;
    let one = 1.0f32.to_bits();
    h.write(value, one);
    h.write(value + 4, one);
    h.write(value + 8, one);
    h.write(value + 0xc, 0u32);
    if h.read::<u32>(at(0x10)) == 0 {
        h.write(at(0x14), 0u32);
        let t = load(time);
        if held && t > hold {
            h.write(at(0x10), 1u32);
        }
        if !shown(h, this) {
            stop(h, this);
            return;
        }
        if held && t <= hold {
            h.call_sym(SLEEP_MS, Conv::Cdecl, &[0x8c]);
        }
    }
    let rate = image_f64(h, FADE_RATE);
    if h.read::<u32>(at(0x10)) == 1 {
        let fade = store(add(mul(d, rate), load(h.read(at(0x14)))));
        h.write(slot, fade);
        h.write(at(0x14), fade);
        if load(fade) >= rate {
            let full = pass(h.read(h.rebase(FADE_FULL)));
            h.write(at(0x10), 2u32);
            h.write(at(0x14), full);
        }
        if !shown(h, this) {
            h.write(at(0x10), 3u32);
        }
    }
    if h.read::<u32>(at(0x10)) == 2 && !shown(h, this) {
        h.write(at(0x10), 3u32);
    }
    if h.read::<u32>(at(0x10)) == 3 {
        let fade = store(sub(load(h.read(at(0x14))), mul(rate, d)));
        h.write(slot, fade);
        h.write(at(0x14), fade);
        if load(fade) < 0.0 {
            stop(h, this);
            return;
        }
    }
    let scale = store(div(load(h.read(at(0x14))), image_f64(h, SCALE_DIVISOR)));
    h.write(value + 0xc, scale);
    set_parameter(h, this, 0x38, COLOR_SCALE, SET_VALUE, value);
    let fade = pass(h.read(at(0x14)));
    h.write(value + 0xc, fade);
    set_parameter(h, this, 0x40, COLOR_SCALE, SET_VALUE, value);
    let frame_time = image_f32(h, FRAME_TIME);
    let c = load(h.read(at(4)));
    // C0 or C3 alone: at or past a frame time (never with a NaN)
    if frame_time <= c {
        let count = h.read::<u32>(at(0x20)) & 0x7fff_ffff;
        let frame_time_f64 = image_f64(h, FRAME_TIME_F64);
        let mut index: u32 = h.read(at(0xc));
        loop {
            index = index.wrapping_add(1) % count;
            let clock = store(sub(load(h.read(at(4))), frame_time_f64));
            h.write(slot, clock);
            h.write(at(4), clock);
            if load(clock) < frame_time || load(clock).is_nan() {
                break;
            }
        }
        h.write(at(0xc), index);
    }
    let index: u32 = h.read(at(0xc));
    let textures: u32 = h.read(at(0x24));
    let texture: u32 = h.read(textures.wrapping_add(index.wrapping_mul(4)));
    h.write(value, 0x42u32);
    h.write(value + 0xc, texture);
    let scene: u32 = h.read(h.rebase(PHYRE_SCENE));
    let sampler = h.call_sym(LINEAR_SAMPLER, Conv::Thiscall(scene), &[2]);
    h.write(value + 8, sampler);
    h.write(value + 4, 0u32);
    set_parameter(h, this, 0x40, COLOR_BUFFER, SET_TEXTURE_CHECKED, value);
}
