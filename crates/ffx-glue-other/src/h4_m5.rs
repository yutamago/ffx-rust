//! Continuation round 11 of list `h4-glue-other`, batch 2: the sphere
//! grid's render object update.

use ffx_graphic::host::{Conv, Host};

/// `operator_delete(p)` (cdecl).
pub const DELETE: u32 = 0x0063_0390;
/// `operator_delete_00630540(p)` (cdecl, `delete[]`).
pub const DELETE_ARRAY: u32 = 0x0063_0540;
/// `GraphicAbmap_releaseGeometry_006a1570(geometry)` (thiscall): the
/// scene's material of the geometry (`+0xc`) released and cleared.
pub const RELEASE_GEOMETRY: u32 = 0x006a_1570;
/// `Phyre_PhyrePrintf(level, format, ...)` (cdecl).
pub const PHYRE_PRINTF: u32 = 0x0043_52f0;
/// The missing render data message (`0x00b45b48`).
pub const NO_RENDER_DATA: u32 = 0x00b4_5b48;
/// `g_ChrDrawDetached` (`0x012fb7d8`).
pub const DRAW_DETACHED: u32 = 0x012f_b7d8;

/// The render data the destroy state (3) frees, in the original's order,
/// with their segment counts.
pub const RENDER_DATA: [(u32, u32); 23] = [
    (0x8c, 1),
    (0x90, 1),
    (0x94, 2),
    (0xb0, 1),
    (0xb8, 1),
    (0xb4, 4),
    (0xc0, 4),
    (0xbc, 4),
    (0xc4, 1),
    (0xd8, 1),
    (0xdc, 1),
    (0xd0, 1),
    (0xa0, 1),
    (0xd4, 1),
    (0xe0, 1),
    (0x98, 1),
    (0x9c, 4),
    (0xa8, 4),
    (0xa4, 4),
    (0xac, 1),
    (0xc8, 1),
    (0xcc, 1),
    (0x88, 1),
];

/// The segment buffers freed per segment, in order: `+0x20` with
/// `delete`, the rest with `delete[]`.
const SEGMENT_BUFFERS: [(u32, u32); 6] = [
    (0x20, DELETE),
    (0x14, DELETE_ARRAY),
    (0x1c, DELETE_ARRAY),
    (0x10, DELETE_ARRAY),
    (0x18, DELETE_ARRAY),
    (0xc, DELETE_ARRAY),
];

/// One render data freed (the original rereads the slot and the segment
/// array before each call): per segment (`+0x94`, 0x6c bytes each) its
/// buffers ([`SEGMENT_BUFFERS`]), then the segments and the render data;
/// the slot cleared.
fn free_render_data(h: &mut impl Host, this: u32, slot: u32, segments: u32) {
    for k in 0..segments {
        for (field, sym) in SEGMENT_BUFFERS {
            let data: u32 = h.read(this + slot);
            let records: u32 = h.read(data + 0x94);
            let p: u32 = h.read(records.wrapping_add(0x6c * k + field));
            h.call_sym(sym, Conv::Cdecl, &[p]);
        }
    }
    let data: u32 = h.read(this + slot);
    let records: u32 = h.read(data + 0x94);
    h.call_sym(DELETE_ARRAY, Conv::Cdecl, &[records]);
    let data: u32 = h.read(this + slot);
    h.call_sym(DELETE, Conv::Cdecl, &[data]);
    h.write(this + slot, 0u32);
}

/// A geometry handle (`this + handle`, 64 bits) given the render data at
/// `this + data` (its `+0x24`), or the missing render data message.
fn attach(h: &mut impl Host, this: u32, handle: u32, data: u32) {
    let lo: u32 = h.read(this + handle);
    let hi: u32 = h.read(this + handle + 4);
    let rd: u32 = h.read(this + data);
    if lo | hi != 0 {
        if rd == 0 {
            let m = h.rebase(NO_RENDER_DATA);
            h.call_sym(PHYRE_PRINTF, Conv::Cdecl, &[1, m]);
        } else {
            h.write(lo + 0x24, rd);
        }
    }
}

/// `GraphicAbmap_update_00685840(this)` (`0x00685840`, `ecx`, plain ret;
/// `graphicAbmapUpdate` tail-jumps to it with `g_GraphicAbmap`), by the
/// state at `+4`:
/// - below 2: unless `g_ChrDrawDetached`, the fifteen geometry handles
///   (`+0x10..+0x88`, 64 bits each) get their render data (the second set
///   `+0xb0..+0xc4` for a state of 0 or 1, the first `+0x98..+0xac` for a
///   negative one; `+0xd8/+0xdc` or `+0xd0/+0xd4` by the sign of `+8`),
///   and the state is negated;
/// - 3: the 23 render data freed, each geometry released and deleted
///   (its virtual destructor with 1), the state incremented;
/// - else the state incremented.
///
/// Returns what the original leaves in `eax`: the old state, its
/// negation, or for state 3 the last geometry's destructor result (0 for
/// an empty handle).
pub fn abmap_update(h: &mut impl Host, this: u32) -> u32 {
    let state: u32 = h.read(this + 4);
    let signed = state.cast_signed();
    if signed < 2 {
        let detached: u32 = h.read(h.rebase(DRAW_DETACHED));
        if detached != 0 {
            return state;
        }
        for (handle, data) in [(0x10, 0x88), (0x18, 0x8c), (0x20, 0x90), (0x28, 0x94)] {
            attach(h, this, handle, data);
        }
        let set: [(u32, u32); 6] = if h.read::<u32>(this + 4).cast_signed() < 0 {
            [
                (0x58, 0x98),
                (0x60, 0xa0),
                (0x68, 0x9c),
                (0x70, 0xa4),
                (0x78, 0xa8),
                (0x80, 0xac),
            ]
        } else {
            [
                (0x58, 0xb0),
                (0x60, 0xb8),
                (0x68, 0xb4),
                (0x70, 0xbc),
                (0x78, 0xc0),
                (0x80, 0xc4),
            ]
        };
        for (handle, data) in set {
            attach(h, this, handle, data);
        }
        attach(h, this, 0x50, 0xc8);
        attach(h, this, 0x30, 0xcc);
        let pair: [(u32, u32); 2] = if h.read::<u32>(this + 8).cast_signed() < 0 {
            [(0x38, 0xd0), (0x40, 0xd4)]
        } else {
            [(0x38, 0xd8), (0x40, 0xdc)]
        };
        for (handle, data) in pair {
            attach(h, this, handle, data);
        }
        attach(h, this, 0x48, 0xe0);
        let negated = h.read::<u32>(this + 4).wrapping_neg();
        h.write(this + 4, negated);
        return negated;
    }
    if state != 3 {
        h.write(this + 4, state.wrapping_add(1));
        return state;
    }
    for (slot, segments) in RENDER_DATA {
        free_render_data(h, this, slot, segments);
    }
    let mut eax = 0;
    for k in 0..15u32 {
        let lo: u32 = h.read(this + 0x10 + 8 * k);
        let hi: u32 = h.read(this + 0x14 + 8 * k);
        eax = lo | hi;
        if eax != 0 {
            eax = h.call_sym(RELEASE_GEOMETRY, Conv::Thiscall(lo), &[]);
            if lo != 0 {
                let vtable: u32 = h.read(lo);
                let destroy: u32 = h.read(vtable);
                eax = h.call(destroy, Conv::Thiscall(lo), &[1]);
            }
        }
    }
    let state: u32 = h.read(this + 4);
    h.write(this + 4, state.wrapping_add(1));
    eax
}
