//! The `TO` layer's GS packet buffer, screen size and menu scissor.

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    /// Whether packets are added (`TOInitAddPkt` 1, `TOSetAddPkt` 0).
    pub const ADD_PKT: Global<u32> = Global::new(0x00c5_d780);
    /// The packet buffer: start, then two more addresses into it
    /// (`TOSetPktBuffAdrs`).
    pub const PKT_BUFF: Global<[u32; 3]> = Global::new(0x0187_16c0);
    /// Screen width (`u16` read).
    pub const SCREEN_W: Global<u16> = Global::new(0x00c5_d770);
    /// Screen height (`u16` read).
    pub const SCREEN_H: Global<u16> = Global::new(0x00c5_d774);

    /// `TOInitAddPkt()` (`0x00904650`).
    pub fn init_add_pkt(mem: &mut impl Mem) {
        ADD_PKT.set(mem, 1);
    }

    /// `TOSetAddPkt()` (`0x008fae10`).
    pub fn set_add_pkt(mem: &mut impl Mem) {
        ADD_PKT.set(mem, 0);
    }

    /// `TOSetPktBuffAdrs(base, a, b)` (`0x009047c0`): `base + a` first,
    /// then `base`, then `base + b`.
    pub fn set_pkt_buff(mem: &mut impl Mem, base: u32, a: u32, b: u32) {
        let p = PKT_BUFF.ptr(mem).addr;
        mem.write(p + 4, base.wrapping_add(a));
        mem.write(p, base);
        mem.write(p + 8, base.wrapping_add(b));
    }

    /// `TOGetScreenWH(&w, &h)` (`0x008fa900`).
    pub fn screen_wh(mem: &mut impl Mem, w: u32, h: u32) {
        let v = SCREEN_W.get(mem);
        mem.write(w, v);
        let v = SCREEN_H.get(mem);
        mem.write(h, v);
    }

    /// `TODrawScissorXYWH(x, y, w, h)` (`0x008f9230`): the menu scissor on
    /// (not in XYUV mode), its box `(x, y, x + w, y + h)` as floats (each
    /// `i32` rounded to `f32`, the sums wrapping). Returns `y + h`, left in
    /// `eax`.
    pub fn draw_scissor_xywh(mem: &mut impl Mem, x: i32, y: i32, w: i32, h: i32) -> u32 {
        use ffx_menu::scissor::bridge::{SCISSOR_BOX, SCISSOR_ON, SCISSOR_XYUV};
        let (right, bottom) = (x.wrapping_add(w), y.wrapping_add(h));
        #[allow(clippy::cast_precision_loss)] // fild then fstp dword: rounded
        let corners = [x, y, right, bottom].map(|v| v as f32);
        SCISSOR_ON.set(mem, 1);
        SCISSOR_XYUV.set(mem, 0);
        SCISSOR_BOX.set(mem, corners);
        bottom.cast_unsigned()
    }

    /// `TOMakePktScissor(x0, y0, x1, y1)` (`0x008fdee0`, the same code at
    /// `0x008f91f0`): the menu scissor on (not in XYUV mode), its box the
    /// four integers as floats.
    pub fn make_pkt_scissor(mem: &mut impl Mem, x0: i32, y0: i32, x1: i32, y1: i32) {
        use ffx_menu::scissor::bridge::{SCISSOR_BOX, SCISSOR_ON, SCISSOR_XYUV};
        #[allow(clippy::cast_precision_loss)] // fild then fstp dword: rounded
        let corners = [x0, y0, x1, y1].map(|v| v as f32);
        SCISSOR_ON.set(mem, 1);
        SCISSOR_XYUV.set(mem, 0);
        SCISSOR_BOX.set(mem, corners);
    }

    /// `0x008fa7b0()`: a constant GS register word, in `edx:eax`.
    pub const GS_WORD_008FA7B0: u64 = 0x2005_d406_2131_3e00;
}
