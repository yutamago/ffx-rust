//! The render-side glue, twenty-second batch (M4 port round 6, list
//! `h3-glue-render`): the HDAO preset picked by the map path. The cookie
//! is left out, as in [`crate::glue10`].

/// `"/"`, `DJB_hash(text)` (cdecl) and `MapFindU32_00691dc0(head, &key)`.
pub const SLASH: u32 = 0x00b8_f88c;
pub const DJB_HASH: u32 = 0x0067_b7d0;
pub const MAP_FIND: u32 = 0x0069_1dc0;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// C `strstr(hay, needle)`: the first occurrence, `hay` itself for an
    /// empty needle, 0 without one.
    fn strstr(h: &impl Host, hay: u32, needle: u32) -> u32 {
        let mut at = hay;
        loop {
            let mut k = 0u32;
            loop {
                let n: u8 = h.read(needle.wrapping_add(k));
                if n == 0 {
                    return at;
                }
                if h.read::<u8>(at.wrapping_add(k)) != n {
                    break;
                }
                k += 1;
            }
            if h.read::<u8>(at) == 0 {
                return 0;
            }
            at = at.wrapping_add(1);
        }
    }

    /// The `alloca` buffer of [`post_set_hdao_setting`] for `len` bytes
    /// below the frame whose `ebp` is given: `_alloca_probe_16` leaves the
    /// stack 16-aligned under the three saved registers.
    #[must_use]
    pub fn hdao_buffer(ebp: u32, len: u32) -> u32 {
        (ebp - 0x1c).wrapping_sub(len) & !0xf
    }

    /// `PostProcessManager_setHDAOSetting(this, path)` (`ret 4`; frame:
    /// the segment's start, then its key, at `ebp - 8`, the length's sign
    /// word at `ebp - 0xc`; the segment is copied to `buffer(size)`, which
    /// the original allocates on its stack, [`hdao_buffer`]): the segment
    /// between the third and fourth
    /// `/` is hashed and looked up in the presets (`+0x1226c`); a preset's
    /// 13 floats go to `+0x1224` and the flag `+0x123d1` is set, else
    /// cleared. Fewer than four `/` change nothing.
    pub fn post_set_hdao_setting(
        h: &mut impl Host,
        ebp: u32,
        buffer: impl FnOnce(u32) -> u32,
        this: u32,
        path: u32,
    ) {
        let slash = h.rebase(SLASH);
        let mut at = path;
        for _ in 0..3 {
            at = strstr(h, at, slash);
            if at == 0 {
                return;
            }
            at = at.wrapping_add(1);
        }
        let start = at;
        h.write(ebp - 8, start);
        let end = strstr(h, start, slash);
        if end == 0 {
            return;
        }
        let n = end.wrapping_sub(start);
        let buffer = buffer(n.wrapping_add(2));
        h.write(ebp - 0xc, if (n as i32) < 0 { u32::MAX } else { 0 });
        // strncpy(buffer, start, n), then the terminator
        let mut ended = false;
        for k in 0..n {
            let b: u8 = if ended { 0 } else { h.read(start + k) };
            ended |= b == 0;
            h.write(buffer.wrapping_add(k), b);
        }
        h.write(buffer.wrapping_add(n), 0u8);
        let key = h.call_sym(DJB_HASH, Conv::Cdecl, &[buffer]);
        h.write(ebp - 8, key);
        let node = h.call_sym(MAP_FIND, Conv::Thiscall(this + 0x1_226c), &[ebp - 8]);
        if node == 0 || node.wrapping_add(0x10) == 0 {
            h.write(this + 0x1_23d1, 0u8);
            return;
        }
        h.write(this + 0x1_23d1, 1u8);
        let preset: u32 = h.read(node + 0x10);
        for k in 0..13 {
            let word: u32 = h.read(preset.wrapping_add(4 * k));
            h.write(this + 0x1224 + 4 * k, x87::store(x87::load(word)));
        }
    }
}
