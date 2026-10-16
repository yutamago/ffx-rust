//! The effects' debug display (`opd_*`): the number printer's cursor and
//! the heap gauges.
//!
//! `g_OpdHeapDisplay` (`0x022c9760`) has four 0x40-byte gauges: flags at
//! `+0` (0xffff for none; bits 8-11 a priority, where a lower or equal one
//! takes the gauge over; 0x8000 for a ppp heap), the heap at `+4` (for
//! `opd_heap_display_set_par`), the used and total bytes at `+8` and
//! `+0xc` (for `opd_heap_display_set`), and four words from `+0x10`.

/// The display state where the game keeps it.
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    /// `g_OpdNumHome`: where `opd_num_vdisp` puts the cursor back (x, y).
    pub const NUM_HOME: Global<[u16; 2]> = Global::new(0x0228_6148);
    /// `g_OpdNumPos`: the number printer's cursor (x, y).
    pub const NUM_POS: Global<[u16; 2]> = Global::new(0x0228_614c);
    /// `g_OpdNumColumn`: characters printed on the line.
    pub const NUM_COLUMN: Global<u16> = Global::new(0x0228_6150);
    /// `g_OpdNumDisplay`: `opd_num`'s display mode.
    pub const NUM_DISPLAY: Global<u16> = Global::new(0x0228_6154);
    /// `g_OpdNumPal`: the printer's palette.
    pub const NUM_PAL: Global<u8> = Global::new(0x0228_615c);
    /// `g_OpdNumFrames`: counted up by `opd_num_vdisp`.
    pub const NUM_FRAMES: Global<u16> = Global::new(0x0228_81d4);
    /// `g_OpdHeapDisplay`: the four gauges.
    pub const HEAP_DISPLAY: Global<[u32; 16]> = Global::new(0x022c_9760);

    /// `opd_num_pal_default()` (`0x00a5de50`).
    pub fn opd_num_pal_default(mem: &mut impl Mem) {
        NUM_PAL.set(mem, 0);
    }

    /// `opd_num_pal_set(pal)` (`0x00a5de60`).
    pub fn opd_num_pal_set(mem: &mut impl Mem, pal: u32) {
        #[allow(clippy::cast_possible_truncation)] // a byte
        NUM_PAL.set(mem, pal as u8);
    }

    /// `opd_num_display(mode)` (`0x00a5dd90`).
    pub fn opd_num_display(mem: &mut impl Mem, mode: u32) {
        #[allow(clippy::cast_possible_truncation)] // 16 bits
        NUM_DISPLAY.set(mem, mode as u16);
    }

    /// `opd_num_xy_set(x, y)` (`0x00a5e2b0`): the cursor, at a line start.
    pub fn opd_num_xy_set(mem: &mut impl Mem, x: u32, y: u32) {
        #[allow(clippy::cast_possible_truncation)] // 16 bits each
        NUM_POS.set(mem, [x as u16, y as u16]);
        NUM_COLUMN.set(mem, 0);
    }

    /// `opd_num_vdisp()` (`0x00a5dfb0`): a new frame: the cursor home and
    /// the default palette.
    pub fn opd_num_vdisp(mem: &mut impl Mem) {
        let n = NUM_FRAMES.get(mem).wrapping_add(1);
        NUM_FRAMES.set(mem, n);
        NUM_COLUMN.set(mem, 0);
        let home = NUM_HOME.get(mem);
        NUM_POS.set(mem, home);
        NUM_PAL.set(mem, 0);
    }

    fn gauge(mem: &impl Mem, flags: u32) -> u32 {
        HEAP_DISPLAY.ptr(mem).at(flags & 3).addr
    }

    /// `opd_heap_display_clear(n)` (`0x00a649c0`): gauge `n & 3` unused.
    pub fn opd_heap_display_clear(mem: &mut impl Mem, n: u32) {
        let g = gauge(mem, n);
        let mut w = [0u32; 8];
        w[0] = 0xffff;
        mem.write(g, w);
    }

    /// `opd_heap_display_init()` (`0x00a64a00`): every gauge unused.
    pub fn opd_heap_display_init(mem: &mut impl Mem) {
        for n in 0..4 {
            opd_heap_display_clear(mem, n);
        }
    }

    /// Whether `flags` may take its gauge over.
    fn takes(mem: &impl Mem, g: u32, flags: u32) -> bool {
        (flags & 0xf00) <= (mem.read::<u32>(g) & 0xf00)
    }

    /// `opd_heap_display_set(used, total, flags)` (`0x00a64b20`).
    pub fn opd_heap_display_set(mem: &mut impl Mem, used: u32, total: u32, flags: u32) {
        if total == 0 {
            return;
        }
        let g = gauge(mem, flags);
        if takes(mem, g, flags) {
            mem.write(g, flags);
            mem.write(g + 8, used);
            mem.write(g + 0xc, total);
        }
    }

    /// `opd_heap_display_set_par(heap, flags)` (`0x00a64b60`): a ppp heap.
    pub fn opd_heap_display_set_par(mem: &mut impl Mem, heap: u32, flags: u32) {
        if heap == 0 {
            return;
        }
        let g = gauge(mem, flags);
        if takes(mem, g, flags) {
            mem.write(g, flags);
            mem.write(g + 4, heap);
        }
    }
}
