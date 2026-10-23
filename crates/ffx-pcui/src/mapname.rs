//! The PC port's map name overlay (a `TextUtil` at `+0x114` of its
//! object): shown or hidden with the map name and up to three lines.

/// The originals, where the game keeps their state (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_graphic::host::{Conv, Host};

    /// `TextUtil_setEnabled(this, on)`, `TextUtil_resize(this, w, h)`,
    /// `TextUtil_changeLine(this, i, text)` (`thiscall` on the overlay's
    /// `TextUtil` at `+0x114`).
    pub const TEXT_SET_ENABLED: u32 = 0x006f_68f0;
    pub const TEXT_RESIZE: u32 = 0x006f_6670;
    pub const TEXT_CHANGE_LINE: u32 = 0x006f_6070;
    /// `yiGetMapName(out)`, `GetGlobal00c44248_007c6db0()`,
    /// `GetGlobal01139e48_007c6da0()`.
    pub const YI_GET_MAP_NAME: u32 = 0x0090_7e70;
    pub const GET_GLOBAL_00C44248: u32 = 0x007c_6db0;
    pub const GET_GLOBAL_01139E48: u32 = 0x007c_6da0;
    /// `0x006b6ef0(this, i, out)` (`thiscall`, unnamed): line `i` of the
    /// overlay's text into `out`.
    pub const LINE_TEXT: u32 = 0x006b_6ef0;
    /// Bytes of work [`show`] needs: the line buffer (the original's
    /// frame from `ebp - 0x104`).
    pub const WORK: u32 = 0x100;

    /// `MapNameText_Show_006bb480(this, show)` (name ours; `thiscall`,
    /// `ret 4`, only the low byte of `show` read): the text shown
    /// (`+0x1c4`, then `TextUtil_setEnabled`) when `show` and `+0x110`
    /// are set. With `show`: the shown text resized to `(+0x1cc, +0x1d0)`,
    /// the map name into `+8`, the two words `GetGlobal00c44248` and
    /// `GetGlobal01139e48` into `+0` and `+4`; then, while `+0x110` stays
    /// set, lines 2, 1, 0 each into a zeroed 0x100-byte buffer
    /// (`0x006b6ef0`) and onto the text (`TextUtil_changeLine`). `eax` is
    /// the stack cookie check's (callers read none).
    pub fn show(mem: &mut impl Host, this: u32, show: u32, work: u32) {
        let show = show.to_le_bytes()[0] != 0;
        let text = this.wrapping_add(0x114);
        let on = show && mem.read::<u8>(this.wrapping_add(0x110)) != 0;
        mem.write(this.wrapping_add(0x1c4), u8::from(on));
        mem.call_sym(TEXT_SET_ENABLED, Conv::Thiscall(text), &[u32::from(on)]);
        if !show {
            return;
        }
        if mem.read::<u8>(this.wrapping_add(0x1c4)) != 0 {
            let w: u32 = mem.read(this.wrapping_add(0x1cc));
            let h: u32 = mem.read(this.wrapping_add(0x1d0));
            mem.call_sym(TEXT_RESIZE, Conv::Thiscall(text), &[w, h]);
        }
        mem.call_sym(YI_GET_MAP_NAME, Conv::Cdecl, &[this.wrapping_add(8)]);
        let a = mem.call_sym(GET_GLOBAL_00C44248, Conv::Cdecl, &[]);
        mem.write(this, a);
        let b = mem.call_sym(GET_GLOBAL_01139E48, Conv::Cdecl, &[]);
        mem.write(this.wrapping_add(4), b);
        for line in [2u32, 1, 0] {
            if mem.read::<u8>(this.wrapping_add(0x110)) == 0 {
                break;
            }
            mem.write_bytes(work, &[0u8; WORK as usize]);
            mem.call_sym(LINE_TEXT, Conv::Thiscall(this), &[line, work]);
            mem.call_sym(TEXT_CHANGE_LINE, Conv::Thiscall(text), &[line, work]);
        }
    }
}
