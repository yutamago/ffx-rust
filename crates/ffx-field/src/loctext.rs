//! `LocText_ConvertLineBreaks_006e9700` (name ours): the PC menus' text
//! line breaks. The Esc menu's pages (`FFEscMenu_*`), `LocString_Load` and
//! others call it through the `jmp` at `0x006e96b0` (33 calls) to turn the
//! localisation strings' `@el@` markers into `\n` before handing them to
//! Iggy. Classified with the map module; it belongs to the PC UI.

/// The marker's pointer (`char *`, `"@el@"`).
pub const MARKER: u32 = 0x00c3_5770;
/// The `"\n"` the original stores as a word (`'\n'`, 0).
pub const NEWLINE: u32 = 0x00b7_cae0;

/// [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::Mem;

    use super::{MARKER, NEWLINE};

    /// `strlen(s)`.
    fn strlen<M: Mem>(mem: &M, s: u32) -> u32 {
        let mut n = 0u32;
        while mem.read::<u8>(s.wrapping_add(n)) != 0 {
            n = n.wrapping_add(1);
        }
        n
    }

    /// The C runtime's `strstr(hay, needle)`: the first match, `hay` for
    /// an empty needle, 0 for none.
    fn strstr<M: Mem>(mem: &M, hay: u32, needle: u32) -> u32 {
        let n = strlen(mem, needle);
        let mut at = hay;
        loop {
            if (0..n).all(|k| {
                mem.read::<u8>(at.wrapping_add(k)) == mem.read::<u8>(needle.wrapping_add(k))
            }) {
                return at;
            }
            if mem.read::<u8>(at) == 0 {
                return 0;
            }
            at = at.wrapping_add(1);
        }
    }

    /// `LocText_ConvertLineBreaks_006e9700(src, dst, len)` (`0x006e9700`):
    /// `memset(dst, 0, len)`, then each piece of `src` before a marker
    /// (`*0x00c35770`) is copied (`strncpy` of its length) and followed
    /// by `'\n', 0` at the first zero byte from the piece's end; the rest
    /// of `src` is copied with its terminator. Returns 0 (`eax`).
    pub fn loc_text_convert_line_breaks<M: Mem>(mem: &mut M, src: u32, dst: u32, len: u32) -> u32 {
        for k in 0..len {
            mem.write(dst.wrapping_add(k), 0u8);
        }
        let marker_at = mem.rebase(MARKER);
        let (mut s, mut d) = (src, dst);
        let mut marker = mem.read::<u32>(marker_at);
        let mut hit = strstr(mem, s, marker);
        while hit != 0 {
            let n = hit.wrapping_sub(s);
            // strncpy(d, s, n): s has n bytes before the marker, none 0
            for k in 0..n {
                let b = mem.read::<u8>(s.wrapping_add(k));
                mem.write(d.wrapping_add(k), b);
            }
            marker = mem.read::<u32>(marker_at);
            d = d.wrapping_add(n);
            s = s.wrapping_add(strlen(mem, marker).wrapping_add(n));
            let mut end = d;
            while mem.read::<u8>(end) != 0 {
                end = end.wrapping_add(1);
            }
            let newline = mem.read::<u16>(mem.rebase(NEWLINE));
            mem.write(end, newline);
            d = d.wrapping_add(1);
            hit = strstr(mem, s, marker);
        }
        loop {
            let b = mem.read::<u8>(s);
            mem.write(d, b);
            s = s.wrapping_add(1);
            d = d.wrapping_add(1);
            if b == 0 {
                break;
            }
        }
        0
    }
}
