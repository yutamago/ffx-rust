//! M5 stage 1, list `menu`, batch 7: the debug font's EUC entry (EUC-JP
//! text converted to Shift JIS in 28-byte pieces) and the event GUI's
//! event list (the selector of `eventid.bin`).

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names
)]

pub mod bridge {
    use ffx_bridge::Global;
    use ffx_graphic::host::{Conv, Host};

    /// The event GUI's loaded `eventid.bin` and its selector.
    pub const EVE_IDS: Global<u32> = Global::new(0x0132_5924);
    pub const EVE_SELECTOR: Global<u32> = Global::new(0x0132_592c);

    pub const TK_PUT_DEBUG_FONT_CORE: u32 = 0x0088_f240;
    pub const USER_FREE: u32 = 0x0068_7010;
    pub const SG_PC_READ: u32 = 0x0083_ae30;
    pub const SG_GET_RESOURCE_STR: u32 = 0x0083_fc30;
    pub const GU_SELECTOR_ALLOCATE: u32 = 0x0084_9bb0;
    pub const GU_SELECTOR_RESET_NB_ITEM: u32 = 0x0084_9f40;
    pub const GU_SELECTOR_SET_ITEM: u32 = 0x0084_9fd0;
    pub const GU_SELECTOR_SET_SELECT_STR: u32 = 0x0084_a110;
    pub const GU_SELECTOR_SORT_ITEM: u32 = 0x0084_a180;
    pub const GU_SELECTOR_OPEN_XYWH: u32 = 0x0084_9dc0;
    /// `"/ffx/proj/event/header/eventid.bin"` in `.rdata`.
    pub const EVENT_IDS_PATH: u32 = 0x00b5_b8e4;

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// `TkPutDebugFontStringPEuc(color, flags, text)` (`0x0088f4e0`, frame
    /// at `ebp`): the text in pieces of at most 28 bytes converted into
    /// `ebp - 0x24` (ASCII as it is; an EUC pair `a b` to Shift JIS: lead
    /// `((a & 0x7f) + 1) / 2 + (0x70 below 0x5f, else 0xb0)`, trail
    /// `(b & 0x7f) + (0x7e for an even lead, else 0x1f or 0x20 past
    /// 0x5f)`), each printed proportionally (`0x0088f240(1, color,
    /// flags, piece)`). The lead and trail are spilled to `ebp - 0x2c` and
    /// `ebp - 0x25` as the original does. A null text prints the buffer as
    /// it is (junk read: `ebp - 0x24` never written; 0 here, brief
    /// 2026-10-24). Returns the last print's `eax`.
    pub fn put_string_p_euc(h: &mut impl Host, ebp: u32, color: u32, flags: u32, text: u32) -> u32 {
        let buf = ebp.wrapping_sub(0x24);
        let mut s = text;
        loop {
            let mut room = 0x1c;
            let mut out = buf;
            if s != 0 {
                loop {
                    let a = h.read::<u8>(s);
                    let b = h.read::<u8>(s.wrapping_add(1));
                    let n = if a == 0 {
                        h.write(out, 0u8);
                        0
                    } else if a < 0x80 {
                        h.write(out, a);
                        1
                    } else {
                        let (a, b) = (a & 0x7f, b & 0x7f);
                        let base: u8 = if a < 0x5f { 0x70 } else { 0xb0 };
                        h.write(ebp.wrapping_sub(0x2c), u32::from(a));
                        h.write(ebp.wrapping_sub(0x25), b);
                        let trail: u8 = if a & 1 == 0 {
                            0x7e
                        } else if b > 0x5f {
                            0x20
                        } else {
                            0x1f
                        };
                        let lead = ((u32::from(a) + 1) >> 1) as u8;
                        h.write(out, lead.wrapping_add(base));
                        h.write(out.wrapping_add(1), trail.wrapping_add(b));
                        2
                    };
                    room -= n as i32;
                    s = s.wrapping_add(n);
                    out = out.wrapping_add(n);
                    if room <= 0 {
                        h.write(out, 0u8);
                        break;
                    }
                    if n == 0 {
                        s = 0;
                        break;
                    }
                }
            }
            let r = cdecl(h, TK_PUT_DEBUG_FONT_CORE, &[1, color, flags, buf]);
            if s == 0 {
                return r;
            }
        }
    }

    /// `gui_eve_GWM_BUTTONUP(window)` (`0x00854e80`): the event list
    /// reloaded (the old one freed; `Sg_PcRead(eventid.bin)`: a count of
    /// 8-byte entries in its first word, each an offset to the event's
    /// name), a selector of them (value = index), sorted, the current
    /// event (`Sg_GetResourceSTR(3)`) selected, and opened at (200, 40)
    /// 150x120 for `window` with message 0x2c. Returns the open's `eax`,
    /// or 0 when the file is missing.
    pub fn eve_gwm_buttonup(h: &mut impl Host, window: u32) -> u32 {
        let old = EVE_IDS.get(h);
        if old != 0 {
            cdecl(h, USER_FREE, &[old]);
            EVE_IDS.set(h, 0);
        }
        let path = h.rebase(EVENT_IDS_PATH);
        let ids = cdecl(h, SG_PC_READ, &[path]);
        EVE_IDS.set(h, ids);
        if ids == 0 {
            return 0;
        }
        let n = h.read::<i32>(ids) / 8;
        let sel = cdecl(h, GU_SELECTOR_ALLOCATE, &[]);
        EVE_SELECTOR.set(h, sel);
        cdecl(h, GU_SELECTOR_RESET_NB_ITEM, &[sel, n as u32]);
        for i in 0..n.max(0) as u32 {
            let table = EVE_IDS.get(h);
            let off: u32 = h.read(table.wrapping_add(i * 8));
            let sel = EVE_SELECTOR.get(h);
            cdecl(h, GU_SELECTOR_SET_ITEM, &[sel, i, off.wrapping_add(ids), i]);
        }
        let sel = EVE_SELECTOR.get(h);
        cdecl(h, GU_SELECTOR_SORT_ITEM, &[sel]);
        let cur = cdecl(h, SG_GET_RESOURCE_STR, &[3]);
        let sel = EVE_SELECTOR.get(h);
        cdecl(h, GU_SELECTOR_SET_SELECT_STR, &[sel, cur]);
        let sel = EVE_SELECTOR.get(h);
        cdecl(
            h,
            GU_SELECTOR_OPEN_XYWH,
            &[sel, window, 0x2c, 200, 0x28, 0x96, 0x78],
        )
    }
}
