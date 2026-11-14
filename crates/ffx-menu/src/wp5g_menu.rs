//! M5 finish, WP5 part 6 (wp5g): menu functions in gaps of the dump that
//! the build without Unicorn stopped at: the summon screen's layout read
//! (`code_008cc1a0` jumps to it) and the main command window's control
//! callback (`TkMenuAppearMainCmdWindow` stores it in the window, `+0xc`;
//! its draw callback `+0x10` is `MenuMain_DrawCmdWindow`, ported in
//! `crate::tk`). Read from the machine code.

#[allow(clippy::similar_names, clippy::many_single_char_names)]
pub mod bridge {
    use ffx_graphic::host::{Conv, Host};

    pub const TK_MN2_GET_COMMON_PLATE: u32 = 0x008c_1a60;
    pub const TK_MN2_SEQ_GET_NUM: u32 = 0x008c_2b10;
    pub const TK_MN2_SEQ_GET_RECT: u32 = 0x008c_2b90;
    pub const TK_MN2_SEQ_GET_POS: u32 = 0x008c_2b50;

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    fn rect(h: &mut impl Host, name: u32, out: u32) -> u32 {
        let (name, out) = (h.rebase(name), h.rebase(out));
        cdecl(h, TK_MN2_SEQ_GET_RECT, &[name, out])
    }

    fn pos(h: &mut impl Host, name: u32, out: u32) -> u32 {
        let (name, out) = (h.rebase(name), h.rebase(out));
        cdecl(h, TK_MN2_SEQ_GET_POS, &[name, out])
    }

    fn plate(h: &mut impl Host, at: [u32; 4]) -> u32 {
        let [p, r, c1, c2] = at.map(|x| h.rebase(x));
        cdecl(h, TK_MN2_GET_COMMON_PLATE, &[p, r, c1, c2, 0, 0])
    }

    /// `MenuSummon_InitLayout_008ceef0()`: the summon screen's layout:
    /// two plates (shapes 0, 0), four positions into `0x023cc424..`, three
    /// rects and seven positions into `0x0186a5cc..`, and the row count
    /// into `0x0186a5dc`, which it returns.
    pub fn summon_init_layout(h: &mut impl Host) -> u32 {
        plate(h, [0x0186_a5b4, 0x00b6_2b10, 0x00b6_2b1c, 0x00b6_2b28]);
        plate(h, [0x023c_c440, 0x00b6_5c44, 0x00b6_5c34, 0x00b6_5c24]);
        for (name, out) in [
            (0x00b6_5c50, 0x023c_c430),
            (0x00b6_5c5c, 0x023c_c42c),
            (0x00b6_5c68, 0x023c_c428),
            (0x00b6_5c74, 0x023c_c424),
        ] {
            pos(h, name, out);
        }
        for (name, out) in [
            (0x00b6_5c80, 0x0186_a608),
            (0x00b6_5c90, 0x0186_a5e8),
            (0x00b6_5ca0, 0x0186_a5f0),
        ] {
            rect(h, name, out);
        }
        for (name, out) in [
            (0x00b6_5cb0, 0x0186_a5f8),
            (0x00b6_2b34, 0x0186_a5cc),
            (0x00b6_5cb8, 0x0186_a5d0),
            (0x00b6_5cc4, 0x0186_a5d4),
            (0x00b6_5cd4, 0x0186_a5d8),
            (0x00b6_28dc, 0x0186_a5e0),
            (0x00b6_5ce4, 0x0186_a5e4),
        ] {
            pos(h, name, out);
        }
        let num = h.rebase(0x00b6_5cf0);
        let n = cdecl(h, TK_MN2_SEQ_GET_NUM, &[num]);
        h.write(h.rebase(0x0186_a5dc), n);
        n
    }

    /// The main command window's globals (`ffx.sym`): done flag, slide
    /// (i32), busy mask (u16), option cursor, row delays and row slides
    /// (u16[8] each).
    pub const CMD_DONE: u32 = 0x0187_1554;
    pub const CMD_SLIDE: u32 = 0x0187_1558;
    pub const CMD_BUSY: u32 = 0x0187_1564;
    pub const OPTION_CURSOR: u32 = 0x0187_1548;
    pub const ROW_DELAY: u32 = 0x0187_1668;
    pub const ROW_SLIDE: u32 = 0x0187_1678;

    /// `TkMenuMainCmdWindow_ctrl_008e2040()`: with the done flag set the
    /// slide grows by 0x155 up to 0x1000 (signed), and each of the eight
    /// rows counts its delay down, or once it is 0 slides by 0xe3 up to
    /// 0x1000 (signed 16-bit); the busy mask is 0x1000 and'ed with every
    /// row's slide (0x1000 once all are in). Returns 0x10 (the loop's
    /// end). Without the flag the slide shrinks by 0x155 down to 0; that
    /// slide is the result.
    pub fn cmd_window_ctrl(h: &mut impl Host) -> u32 {
        let slide = h.rebase(CMD_SLIDE);
        if h.read::<u32>(h.rebase(CMD_DONE)) == 0 {
            let v = h.read::<u32>(slide).wrapping_sub(0x155);
            let v = if v.cast_signed() < 0 { 0 } else { v };
            h.write(slide, v);
            return v;
        }
        let v = h.read::<u32>(slide).wrapping_add(0x155);
        h.write(slide, if v.cast_signed() > 0x1000 { 0x1000 } else { v });
        let (delay, row) = (h.rebase(ROW_DELAY), h.rebase(ROW_SLIDE));
        let mut busy = 0x1000u16;
        for i in 0..8 {
            let d = delay.wrapping_add(2 * i);
            let s = row.wrapping_add(2 * i);
            let c: u16 = h.read(d);
            if c == 0 {
                let v = h.read::<u16>(s).wrapping_add(0xe3);
                h.write(s, if v.cast_signed() > 0x1000 { 0x1000 } else { v });
            } else {
                h.write(d, c - 1);
            }
            busy &= h.read::<u16>(s);
        }
        h.write(h.rebase(CMD_BUSY), busy);
        0x10
    }
}
