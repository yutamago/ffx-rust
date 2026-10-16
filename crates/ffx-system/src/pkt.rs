//! The `TO` layer's GS packet buffer and screen size.

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
}
