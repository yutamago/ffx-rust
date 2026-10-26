//! List `h4-glue-other`, second agent (h4b), batch 4: Iggy's `GDraw`
//! render-target allocation and the menu manager's set-up.

use ffx_graphic::host::{Conv, Host};

use ffx_bridge::x87::{add, div, load, mul, store};

use crate::h4_e1::{REMAP_X2, REMAP_Y2, int_f32, pass};
use crate::h4b_b::{GDRAW_FREE, GDRAW_STATE, GDRAW_WARNING};

/// `StoreComputed_00a3a010(block, kind)` (cdecl): requeues a block.
pub const REQUEUE: u32 = 0x00a3_a010;
/// `IggyHeap_takeFree_00a39a30(heap)` (cdecl).
pub const TAKE_FREE: u32 = 0x00a3_9a30;
/// `IggyHeap_moveBlock_00a39a90(block, size, owner, kind)` (cdecl).
pub const MOVE_BLOCK: u32 = 0x00a3_9a90;
/// `StoreComputed_00a39b00(block)` (cdecl): frees a block.
pub const FREE_BLOCK: u32 = 0x00a3_9b00;
/// `GDraw`'s device error report `0x00a3c170(hr, call, what)` (cdecl).
pub const REPORT_HR: u32 = 0x00a3_c170;

/// `obj->Release()` when `obj` is set (vtable `+8`).
fn release(h: &mut impl Host, obj: u32) -> bool {
    if obj == 0 {
        return false;
    }
    let f: u32 = h.read(h.read::<u32>(obj) + 8);
    h.call(f, Conv::Stdcall, &[obj]);
    true
}

/// A device method (vtable `off` of `[state]`, stdcall, `this` first).
fn device_call(h: &mut impl Host, off: u32, args: &[u32]) -> u32 {
    let state = GDRAW_STATE.get(h);
    let device: u32 = h.read(state);
    let f: u32 = h.read(h.read::<u32>(device) + off);
    let mut a = vec![device];
    a.extend_from_slice(args);
    h.call(f, Conv::Stdcall, &a)
}

/// `gdraw_allocRenderTarget_00a3b610(stats)` (cdecl): a render target of
/// the screen's size (`+0x6c0` x `+0x6c4` of the state). A cached one
/// (the ring at `+0x2d8`, its first block `+0x2f4`) is reused: in state 1
/// with owner 1 it is requeued as kind 2 (`0x00a3a010`). Otherwise the
/// size (4 bytes a pixel) must fit the budget (`+0x290`; a warning with
/// the limit `+0x294` and 0), a block comes from the heap
/// (`IggyHeap_takeFree`; a warning and 0 without one), cleared, and gets
/// the texture (`CreateTexture2D`, vtable `+0x14`, RGBA, bind 0x28;
/// description in the frame at `desc`, the original's `ebp - 0x30`), its
/// view (`+0x1c`) and render-target view (`+0x24`). Done: the block moves
/// to kind 2 owned by 1, `stats` gets flag 4, one more target and the
/// bytes. A failure releases what was made, frees the block and reports
/// the call (`0x00a3c170`; the view's failure is named after the
/// texture, as in the original). Returns the block or 0.
pub fn alloc_render_target(h: &mut impl Host, desc: u32, stats: u32) -> u32 {
    let state = GDRAW_STATE.get(h);
    let ring = state + 0x2d8;
    let cached: u32 = h.read(ring + 0x1c);
    if cached != ring && cached != 0 {
        let kind = h.read::<u32>(cached + 0x30) >> 28;
        if kind != 5 && h.read::<u32>(cached + 0x14) == 1 && kind == 1 {
            h.call_sym(REQUEUE, Conv::Cdecl, &[cached, 2]);
        }
        return cached;
    }
    let (w, hh): (u32, u32) = (h.read(state + 0x6c0), h.read(state + 0x6c4));
    let size = w.wrapping_mul(hh).wrapping_shl(2);
    let pool = state + 0x290;
    if h.read::<i32>(pool) < size.cast_signed() {
        let limit: u32 = h.read(state + 0x294);
        let f = GDRAW_WARNING.get(h);
        let fmt = h.rebase(0x00b7_a6f8);
        h.call(f, Conv::Cdecl, &[0, fmt, limit]);
        return 0;
    }
    let block = h.call_sym(TAKE_FREE, Conv::Cdecl, &[pool]);
    if block == 0 {
        let f = GDRAW_WARNING.get(h);
        let fmt = h.rebase(0x00b7_a73c);
        h.call(f, Conv::Cdecl, &[0, fmt]);
        return 0;
    }
    let words = [w, hh, 1, 1, 0x1c, 1, 0, 0, 0x28, 0, 0];
    for (k, v) in words.iter().enumerate() {
        h.write(desc + 4 * k as u32, *v);
    }
    for off in [0, 4, 8] {
        h.write(block + off, 0u32);
    }
    let mut call = 0x00b7_a688;
    let mut hr = device_call(h, 0x14, &[desc, 0, block]);
    if hr.cast_signed() >= 0 {
        let tex: u32 = h.read(block);
        hr = device_call(h, 0x1c, &[tex, 0, block + 4]);
        if hr.cast_signed() >= 0 {
            let tex: u32 = h.read(block);
            hr = device_call(h, 0x24, &[tex, 0, block + 8]);
            call = 0x00b7_a774;
            if hr.cast_signed() >= 0 {
                h.call_sym(MOVE_BLOCK, Conv::Cdecl, &[block, size, 1, 2]);
                let flags: u16 = h.read(stats);
                h.write(stats, flags | 4);
                let n: u16 = h.read(stats + 0xa);
                h.write(stats + 0xa, n.wrapping_add(1));
                let bytes: u32 = h.read(stats + 0x1c);
                h.write(stats + 0x1c, bytes.wrapping_add(size));
                return block;
            }
        }
    }
    for off in [0, 4, 8] {
        if release(h, h.read(block + off)) {
            h.write(block + off, 0u32);
        }
    }
    h.call_sym(FREE_BLOCK, Conv::Cdecl, &[block]);
    let (c, w) = (h.rebase(call), h.rebase(0x00b7_a78c));
    h.call_sym(REPORT_HR, Conv::Cdecl, &[hr, c, w]);
    0
}

// ---- the Iggy menu manager's set-up

/// `Phyre::PhyreMalloc(size, align)` (cdecl).
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
/// `Phyre::PhyreFree(p)` (cdecl).
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// `Config_InitDefaults_00a34a60(this)` (thiscall).
pub const CONFIG_DEFAULTS: u32 = 0x00a3_4a60;
/// `GDraw`'s setup `0x00a3d5f0(device, &flag, &config)` (cdecl).
pub const GDRAW_INIT: u32 = 0x00a3_d5f0;
/// `IggyMenuManager_SetViewport_00a26a20(this; w, h)` (thiscall).
pub const SET_VIEWPORT: u32 = 0x00a2_6a20;
/// `IggyMenuManager_callback_00a260f0` (stdcall), Iggy's AS3 callback.
pub const AS3_CALLBACK: u32 = 0x00a2_60f0;
/// `_IggySetAS3ExternalFunctionCallbackUTF8@8` (import slot).
pub const SET_AS3_CALLBACK: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x00b0_c83c);

/// A `PArray`-style member (`count` at `at`, top bit "not owned", data at
/// `at + 4`) grown to `n` elements of `size` bytes when its count differs:
/// a new block (`PhyreMalloc(n * size, 4)`), the old one freed when owned
/// and different (and, with `null_check`, set).
fn reserve(h: &mut impl Host, at: u32, n: u32, size: u32, null_check: bool) {
    if h.read::<u32>(at) == n {
        return;
    }
    let block = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[n * size, 4]);
    if block == 0 {
        return;
    }
    if size == 4 {
        for k in 0..n {
            h.write(block + 4 * k, 0u32);
        }
    }
    let old: u32 = h.read(at + 4);
    if old != block && h.read::<u32>(at) >> 31 == 0 && (!null_check || old != 0) {
        h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
    }
    h.write(at + 4, block);
    h.write(at, n);
}

/// `IggyMenuManager_init_00a262e0(this; device, w, h)` (thiscall, `ret
/// 0xc`): the manager's arrays (`+8` five menu pointers, cleared; `+0x10`
/// ten 16-byte records, their first byte and next two words cleared;
/// both reallocated only when their size differs), the device at `+0`,
/// `GDraw` set up (`0x00a3d5f0(device, &1, &config)` with the config
/// defaults and then 0xa8c0000, 10, 0x400000, 2000, 0x4000000, 200 in the
/// frame, `frame` the original's `ebp - 0x24`), `+4` from `0x01a858c4`,
/// the viewport (`w`, `h`) and Iggy's AS3 callback. Returns the last
/// call's `eax`.
pub fn menu_manager_init(
    h: &mut impl Host,
    frame: u32,
    this: u32,
    device: u32,
    w: u32,
    hh: u32,
) -> u32 {
    reserve(h, this + 8, 5, 4, false);
    let mut i = 0;
    while i < h.read::<u32>(this + 8) & 0x7fff_ffff {
        let data: u32 = h.read(this + 0xc);
        h.write(data + 4 * i, 0u32);
        i += 1;
    }
    reserve(h, this + 0x10, 10, 0x10, true);
    let mut i = 0;
    while i < h.read::<u32>(this + 0x10) & 0x7fff_ffff {
        let r = 0x10 * i;
        let data: u32 = h.read(this + 0x14);
        h.write(data + r, 0u8);
        let data: u32 = h.read(this + 0x14);
        h.write(data + r + 4, 0u32);
        let data: u32 = h.read(this + 0x14);
        h.write(data + r + 8, 0u32);
        i += 1;
    }
    h.write(this, device);
    h.call_sym(CONFIG_DEFAULTS, Conv::Thiscall(frame), &[]);
    let ebp = frame + 0x24;
    for (off, v) in [
        (0x14, 0x40_0000),
        (0x10, 2000),
        (0xc, 0x400_0000),
        (8, 200),
        (0x24, 0xa8c_0000),
        (0x20, 10),
    ] {
        h.write(ebp - off, v as u32);
    }
    h.write(ebp - 1, 1u8);
    let device: u32 = h.read(this);
    h.call_sym(GDRAW_INIT, Conv::Cdecl, &[device, ebp - 1, frame]);
    let v: u32 = h.read(h.rebase(0x01a8_58c4));
    h.write(this + 4, v);
    h.call_sym(SET_VIEWPORT, Conv::Thiscall(this), &[w, hh]);
    let f = SET_AS3_CALLBACK.get(h);
    let cb = h.rebase(AS3_CALLBACK);
    h.call(f, Conv::Stdcall, &[cb, 0])
}

// ---- TODrawBtlCtbParts

/// `TODrawCrossBoxXYWHC2(x, y, w, h, c0, c1)` (cdecl).
pub const CROSS_BOX: u32 = 0x008f_4b20;
/// `DrawCrossCtbIcon(id, x, y, w, h)` (cdecl).
pub const CTB_ICON: u32 = 0x0089_e660;
/// `AtelGetScenarioFlag()` (cdecl).
pub const SCENARIO_FLAG: u32 = 0x0086_c470;
/// `MsGetRamChrIconType(chr)` (cdecl).
pub const CHR_ICON_TYPE: u32 = 0x0079_ae30;
/// `MsGetRamChrIconNumber(chr)` (cdecl).
pub const CHR_ICON_NUMBER: u32 = 0x0079_ae10;

/// The `f32` constant at image address `sym` (`fld`/`fstp`).
fn konst(h: &impl Host, sym: u32) -> u32 {
    pass(h.read(h.rebase(sym)))
}

/// `graphicUiRemapX2`: `ST0`.
fn remap_x(h: &mut impl Host, v: u32) -> f64 {
    h.call_sym_st0_f64(REMAP_X2, Conv::Cdecl, &[v])
}

/// `graphicUiRemapY2`: `ST0`.
fn remap_y(h: &mut impl Host, v: u32) -> f64 {
    h.call_sym_st0_f64(REMAP_Y2, Conv::Cdecl, &[v])
}

/// The bar's height `remapY2(0x00b5ee90)` (asked first) and unit width
/// `remapX2(0x00b92268)`.
fn bar_metrics(h: &mut impl Host) -> (u32, f64) {
    let k = konst(h, 0x00b5_ee90);
    let height = store(remap_y(h, k));
    let k = konst(h, 0x00b9_2268);
    (height, remap_x(h, k))
}

/// A bar segment `TODrawCrossBoxXYWHC2(x, y, w, h, c, c)`.
fn bar(h: &mut impl Host, xywh: [u32; 4], c: u32) {
    let [x, y, w, hh] = xywh;
    h.call_sym(
        CROSS_BOX,
        Conv::Cdecl,
        &[pass(x), pass(y), pass(w), hh, c, c],
    );
}

/// The fading tip of a bar: width `remapX2(edge)` (`edge` an `f32`), at
/// `x + unit * m` (`mf` the `f32` of `m`), colour `c`.
fn bar_tip(h: &mut impl Host, x: u32, y: u32, edge: u32, mf: u32, c: u32) {
    let wv = store(remap_x(h, edge));
    let k = konst(h, 0x00b5_ee90);
    let height = store(remap_y(h, k));
    let k = konst(h, 0x00b9_2268);
    let unit = remap_x(h, k);
    let xx = store(add(mul(unit, load(mf)), load(x)));
    bar(h, [xx, y, wv, height], c);
}

/// A CTB icon `DrawCrossCtbIcon(id, x + unit * n, y, remapX2(0x00b5f314),
/// remapY2(0x00b5f304))`, the height and width asked first; the x is
/// computed in an argument slot (`slot`). `id` is a closure: the icon
/// number calls come after the x.
fn ctb_icon<H: Host>(
    h: &mut H,
    n: u32,
    x: u32,
    y: u32,
    slot: Option<u32>,
    id: impl FnOnce(&mut H) -> u32,
) -> u32 {
    let k = konst(h, 0x00b5_f304);
    let height = store(remap_y(h, k));
    let k = konst(h, 0x00b5_f314);
    let width = store(remap_x(h, k));
    let k = konst(h, 0x00b9_2268);
    let unit = remap_x(h, k);
    let nf = int_f32(n.cast_signed());
    let xx = store(add(mul(unit, load(nf)), load(x)));
    if let Some(s) = slot {
        h.write(s, xx);
    }
    let id = id(h);
    h.call_sym(
        CTB_ICON,
        Conv::Cdecl,
        &[id, pass(xx), pass(y), width, height],
    )
}

/// `TODrawBtlCtbParts(chr, count, x, y)` (cdecl; x, y floats): one row of
/// the CTB window: the bar of `count` ticks in three colour bands (20
/// ticks each: `0x80d675b5` below 20, `0x80c13995` 20..40, `0x807c0f59`
/// 40..60, a full band from 60), each partial band with a tip in the
/// band below's colour whose width fades over its last ticks (`remapX2((95 - count) * 2)` or
/// `(75 - count) * 2`, else `0x00b567a8`), then the character's icon at
/// `x + unit * min(count, 20)`: party members `0x1b8 + chr` (Rikku before
/// scenario 0x434: `0x1d4`), 8..17 `0x1c0`, monsters by icon type (0x14:
/// number + 0x1cb, 0x15: 0x1c2, 0x16: number + 0x1c3, 0x17: 0x1d3, else
/// 0x1c1).
///
/// The original keeps temporaries in its argument slots (`slots`, the
/// caller's first argument word): the count slot ends 20 from 20 up (and
/// holds float temporaries before), the icon x goes to the chr slot
/// (party, 8..17) or the count slot (Rikku early, monsters). Returns the
/// icon call's `eax` (or a bar's; none the callers read).
#[allow(clippy::too_many_lines)]
pub fn btl_ctb_parts(
    h: &mut impl Host,
    chr: u32,
    count: u32,
    x: u32,
    y: u32,
    slots: Option<u32>,
) -> u32 {
    let slot = |k: u32| slots.map(|s| s + 4 * k);
    let c = count.cast_signed();
    let mut eax = 0;
    if c >= 0x3c {
        let (height, unit) = bar_metrics(h);
        let w = store(mul(unit, h.read(h.rebase(0x00b5_af28))));
        bar(h, [x, y, w, height], 0x807c_0f59);
    }
    if count.wrapping_sub(0x28) <= 0x13 {
        let mf = int_f32(c - 0x28);
        let (height, unit) = bar_metrics(h);
        let w = store(mul(unit, load(mf)));
        bar(h, [x, y, w, height], 0x807c_0f59);
        let edge = if c * 2 > 0x6e {
            int_f32((0x5f - c) * 2)
        } else {
            konst(h, 0x00b5_67a8)
        };
        bar_tip(h, x, y, edge, mf, 0x80c1_3995);
    }
    let n = if c < 0x14 {
        let (height, unit) = bar_metrics(h);
        let w = store(mul(unit, load(int_f32(c))));
        bar(h, [x, y, w, height], 0x80d6_75b5);
        count
    } else {
        if c < 0x28 {
            let mf = int_f32(c - 0x14);
            let (height, unit) = bar_metrics(h);
            let w = store(mul(unit, load(mf)));
            bar(h, [x, y, w, height], 0x80c1_3995);
            let edge = if c * 2 + 0x28 > 0x6e {
                int_f32((0x4b - c) * 2)
            } else {
                konst(h, 0x00b5_67a8)
            };
            bar_tip(h, x, y, edge, mf, 0x80d6_75b5);
        }
        if let Some(s) = slot(1) {
            h.write(s, 0x14u32);
        }
        0x14
    };
    let ci = chr.cast_signed();
    if ci <= 7 {
        if ci == 6 && h.call_sym(SCENARIO_FLAG, Conv::Cdecl, &[]).cast_signed() < 0x434 {
            return ctb_icon(h, n, x, y, slot(1), |_| 0x1d4);
        }
        eax = ctb_icon(h, n, x, y, slot(0), |_| chr.wrapping_add(0x1b8));
    }
    if chr.wrapping_sub(8) <= 9 {
        eax = ctb_icon(h, n, x, y, slot(0), |_| 0x1c0);
    }
    if ci >= 0x14 {
        let kind = h.call_sym(CHR_ICON_TYPE, Conv::Cdecl, &[chr]);
        return match kind {
            0x14 => ctb_icon(h, n, x, y, slot(1), |h| {
                h.call_sym(CHR_ICON_NUMBER, Conv::Cdecl, &[chr])
                    .wrapping_add(0x1cb)
            }),
            0x15 => ctb_icon(h, n, x, y, slot(1), |_| 0x1c2),
            0x16 => ctb_icon(h, n, x, y, slot(1), |h| {
                h.call_sym(CHR_ICON_NUMBER, Conv::Cdecl, &[chr])
                    .wrapping_add(0x1c3)
            }),
            0x17 => ctb_icon(h, n, x, y, slot(1), |_| 0x1d3),
            _ => ctb_icon(h, n, x, y, slot(1), |_| 0x1c1),
        };
    }
    eax
}

// ---- the face cells

/// `TOGetShapTextureName(id)` (cdecl).
pub const SHAPE_TEXTURE_NAME: u32 = 0x008a_c8c0;
/// `TOGetImageWH(id, &w, &h)` (cdecl).
pub const IMAGE_WH: u32 = 0x008a_c400;
/// `AtelGetAlbhedRikku()` (cdecl).
pub const ALBHED_RIKKU: u32 = 0x0086_a7e0;
/// `graphicDrawUIElement(record, texture, a, b, c)` (cdecl).
pub const DRAW_UI_ELEMENT: u32 = 0x0063_eee0;

/// `TODrawChrFaceCell_008efd90(pos, face, x, y, c0, c1, c2, c3, mode)`
/// (cdecl; x and y integers, the colours the record's four shade words):
/// a 100 x 100 face cell as a `graphicDrawUIElement` record in the frame.
/// Faces from 0x12 draw nothing. Mode 6 (the sphere grid's panel): image
/// 0x2ed0 (4 a row; Rikku's face 6 is 7 when Al Bhed), the cell's far
/// corner at `x + 0x00b66ca8`, `y + 0x00b66cb8` (`f64` constants), drawn
/// with flags (1, 0, 6). Otherwise faces 0..=6 as above and 7.. from
/// 0x2ed4 (`face - 8`, 5 a row, signed), the far corner at `x +
/// 0x00b922d8` and the integer `y + 0x62`, flags (1, 0, 0); this case
/// keeps the column offset in its face argument slot (`slots`, the
/// caller's first argument word).
///
/// `frame` is the original's `ebp - 0xb4`: `+0` the image height, `+4`
/// its width, `+0x18` the record (0x98 bytes; the words it never writes
/// stay, its last two are 0). Returns `pos`.
#[allow(clippy::too_many_arguments)]
pub fn chr_face_cell(
    h: &mut impl Host,
    frame: u32,
    pos: u32,
    face: u32,
    x: u32,
    y: u32,
    shades: [u32; 4],
    mode: u32,
    slots: Option<u32>,
) -> u32 {
    let ebp = frame + 0xb4;
    h.write(ebp - 0xc, 0u32);
    h.write(ebp - 8, 0u32);
    if face > 0x11 {
        return pos;
    }
    let rec = ebp - 0x9c;
    let grid = mode == 6;
    let img = if grid || face <= 6 { 0x2ed0 } else { 0x2ed4 };
    let texture = h.call_sym(SHAPE_TEXTURE_NAME, Conv::Cdecl, &[img]);
    h.call_sym(IMAGE_WH, Conv::Cdecl, &[img, ebp - 0xb0, ebp - 0xb4]);
    let mut face = face;
    if img == 0x2ed0 && face == 6 && h.call_sym(ALBHED_RIKKU, Conv::Cdecl, &[]) == 1 {
        face = 7;
    }
    let xf = int_f32(x.cast_signed());
    let yf = int_f32(y.cast_signed());
    let (col, row) = if img == 0x2ed0 {
        let k = face.cast_signed();
        (
            k.wrapping_rem(4).wrapping_mul(100),
            (k / 4).wrapping_mul(100),
        )
    } else {
        let k = face.wrapping_sub(8).cast_signed();
        let col = k.wrapping_rem(5).wrapping_mul(100);
        if let Some(s) = slots {
            h.write(s + 4, col);
        }
        (col, (k / 5).wrapping_mul(100))
    };
    let iw = load(h.read(ebp - 0xb0));
    let ih = load(h.read(ebp - 0xb4));
    let u0 = store(div(load(int_f32(col)), iw));
    let v0 = store(div(load(int_f32(row)), ih));
    let (x1, y1) = if grid {
        let x1 = store(add(load(xf), h.read::<f64>(h.rebase(0x00b6_6ca8))));
        let y1 = store(add(load(yf), h.read::<f64>(h.rebase(0x00b6_6cb8))));
        (x1, y1)
    } else {
        let x1 = store(add(load(xf), h.read::<f64>(h.rebase(0x00b9_22d8))));
        (x1, int_f32(y.cast_signed().wrapping_add(0x62)))
    };
    let iw = load(h.read(ebp - 0xb0));
    let ih = load(h.read(ebp - 0xb4));
    let u1 = store(div(load(int_f32(col.wrapping_add(100))), iw));
    let v1 = store(div(load(int_f32(row.wrapping_add(100))), ih));
    let head = [xf, yf, u0, v0, shades[0], shades[1], shades[2], shades[3]];
    let tail = [x1, y1, u1, v1, shades[0], shades[1], shades[2], shades[3]];
    for (k, v) in head.iter().chain(tail.iter()).enumerate() {
        h.write(rec + 4 * k as u32, *v);
    }
    let flags = if grid { 6 } else { 0 };
    h.call_sym(DRAW_UI_ELEMENT, Conv::Cdecl, &[rec, texture, 1, 0, flags]);
    pos
}

// ---- GDraw's texture end

/// `op_texels_mip_00a384d0(dst, dst_step, w, h, src, src_step, size)`.
pub const TEXELS_MIP: u32 = 0x00a3_84d0;

/// Moves heap block `block` into the ring whose head is `head` (links
/// `+0x1c` next, `+0x20` prev; linked in before the head).
fn relink(h: &mut impl Host, block: u32, head: u32) {
    let prev: u32 = h.read(block + 0x20);
    let next: u32 = h.read(block + 0x1c);
    h.write(prev + 0x1c, next);
    let next: u32 = h.read(block + 0x1c);
    let prev: u32 = h.read(block + 0x20);
    h.write(next + 0x20, prev);
    h.write(block + 0x1c, head);
    let last: u32 = h.read(head + 0x20);
    h.write(block + 0x20, last);
    h.write(last + 0x1c, block);
    let head: u32 = h.read(block + 0x1c);
    h.write(head + 0x20, block);
}

/// `gdraw_makeTextureEnd_00a3a200(info, stats)` (stdcall, `ret 8`): the
/// texture `gdraw_makeTextureBegin_00a3a070` staged. Each mip level's
/// initial data (pointer into the staging block, pitch `max(w >> k, 1) *
/// texel`, 0) goes into the frame's array, the levels after the first
/// built from the one before (`op_texels_mip_00a384d0`); the description
/// (`w`, `h`, levels, 1, format, 1, 0, usage 1 unless bit 1 of the flags,
/// bind 8, 0, 0) too. `CreateTexture2D` (vtable `+0x14`) into the record
/// (`info + 0x18`) and its view (`+0x1c`). Done: the record takes the
/// owner (`+0x1c`), its size (low 28 bits of `+0x30`), the heap's budget
/// pays it, and it moves to the ring of kind 4 (flag bit 2) or 2, the
/// kind in its top nibble; `stats` gets flag 4, one more texture and the
/// bytes. A failure releases both objects, returns the record to its
/// heap's free ring and warns. The staging block is freed. Returns the
/// record, 0 on failure.
///
/// `frame` is the original's `ebp - 0x150`: the description (11 words),
/// then the level array (`+0x2c`, 12 bytes a level).
#[allow(clippy::too_many_lines)]
pub fn make_texture_end(h: &mut impl Host, frame: u32, info: u32, stats: u32) -> u32 {
    let desc = frame;
    let init = frame + 0x2c;
    let w: i32 = h.read(info + 0x38);
    let hh: i32 = h.read(info + 0x3c);
    let levels: i32 = h.read(info + 0x4c);
    let texel: i32 = h.read(info + 0x50);
    let record: u32 = h.read(info + 0x18);
    let mut ptr: u32 = h.read(info);
    for k in 0..levels.max(0) as u32 {
        let e = init + 12 * k;
        h.write(e, ptr);
        let lw = w.wrapping_shr(k);
        let pitch = lw.max(1).wrapping_mul(texel);
        h.write(e + 4, pitch);
        h.write(e + 8, 0u32);
        let lh = hh.wrapping_shr(k);
        ptr = ptr.wrapping_add(pitch.wrapping_mul(lh.max(1)) as u32);
        if k != 0 {
            let (src, src_pitch): (u32, u32) = (h.read(e - 12), h.read(e - 8));
            let dst: u32 = h.read(e);
            h.call_sym(
                TEXELS_MIP,
                Conv::Cdecl,
                &[
                    dst,
                    pitch as u32,
                    lw as u32,
                    lh as u32,
                    src,
                    src_pitch,
                    texel as u32,
                ],
            );
        }
    }
    let flags: u32 = h.read(info + 0x40);
    let words = [
        w as u32,
        hh as u32,
        levels as u32,
        1,
        h.read(info + 0x44),
        1,
        0,
        !(flags >> 1) & 1,
        8,
        0,
        0,
    ];
    for (k, v) in words.iter().enumerate() {
        h.write(desc + 4 * k as u32, *v);
    }
    let mut call = 0x00b7_a688;
    let mut hr = device_call(h, 0x14, &[desc, init, record]);
    if hr.cast_signed() >= 0 {
        call = 0x00b7_a698;
        let tex: u32 = h.read(record);
        hr = device_call(h, 0x1c, &[tex, 0, record + 4]);
    }
    let result = if hr.cast_signed() >= 0 {
        let kind = if h.read::<u8>(info + 0x40) & 4 != 0 {
            4
        } else {
            2
        };
        let owner: u32 = h.read(info + 0x1c);
        let size: u32 = h.read(info + 0x48);
        h.write(record + 0x14, owner);
        let v: u32 = h.read(record + 0x30);
        h.write(record + 0x30, v ^ ((v ^ size) & 0x0fff_ffff));
        let heap: u32 = h.read(record + 0x18);
        let budget: u32 = h.read(heap);
        h.write(heap, budget.wrapping_sub(size));
        let heap: u32 = h.read(record + 0x18);
        relink(h, record, heap.wrapping_add(8 * (7 * kind + 2)));
        let v: u32 = h.read(record + 0x30);
        h.write(record + 0x30, (v & 0x0fff_ffff) | kind << 28);
        let f: u16 = h.read(stats);
        h.write(stats, f | 4);
        let n: u16 = h.read(stats + 0xa);
        h.write(stats + 0xa, n.wrapping_add(1));
        let bytes: u32 = h.read(stats + 0x1c);
        let size: u32 = h.read(info + 0x48);
        h.write(stats + 0x1c, bytes.wrapping_add(size));
        record
    } else {
        for off in [0, 4] {
            if release(h, h.read(record + off)) {
                h.write(record + off, 0u32);
            }
        }
        let heap: u32 = h.read(record + 0x18);
        relink(h, record, heap.wrapping_add(0x10));
        let v: u32 = h.read(record + 0x30);
        h.write(record + 0x30, v & 0x0fff_ffff);
        let f = GDRAW_WARNING.get(h);
        let (c, what) = (h.rebase(call), h.rebase(0x00b7_a6b4));
        if hr == 0x8007_000e {
            let fmt = h.rebase(0x00b7_a4ec);
            h.call(f, Conv::Cdecl, &[0, fmt, c, what]);
        } else {
            let fmt = h.rebase(0x00b7_a50c);
            h.call(f, Conv::Cdecl, &[0, fmt, c, what, hr]);
        }
        0
    };
    let free = GDRAW_FREE.get(h);
    let block: u32 = h.read(info);
    h.call(free, Conv::Stdcall, &[block]);
    result
}
