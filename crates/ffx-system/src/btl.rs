//! Battle-window state of the `TO` layer (`TOBtl*`): help and sign windows,
//! the cursor position stack, Lulu's limit table, target callbacks.

/// One entry of `TOBtlSetLuluLimitTbl`'s running sums of ASCII digits:
/// the previous entry (0 for the first) plus the digit's value (bytes,
/// wrapping).
#[must_use]
pub const fn lulu_limit_step(prev: u8, digit: u8) -> u8 {
    prev.wrapping_sub(0x30).wrapping_add(digit)
}

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    /// The battle help window is open (`TOBtlCloseHelpWin` clears it).
    pub const HELP_WIN: Global<u8> = Global::new(0x0133_c93c);
    /// Set by `TOBtlInitAnm`.
    pub const INIT_ANM: Global<u8> = Global::new(0x0134_0452);
    /// The sign window (`TOBtlCloseSignWindow` clears it).
    pub const SIGN_WINDOW: Global<u32> = Global::new(0x0133_f520);
    /// The simple help message (`TOBtlCloseSimpleHelpMes` clears it).
    pub const SIMPLE_HELP: Global<u32> = Global::new(0x0133_f510);
    /// `TOGetBtlWindowControlEnable`'s flag.
    pub const WINDOW_CONTROL: Global<u32> = Global::new(0x00c5_8ef8);
    /// Cursor positions pushed (`i16`, at most 4).
    pub const CUR_POS_COUNT: Global<i16> = Global::new(0x0133_d6f0);
    /// The cursor positions: 8-byte records, x at `+0`, y at `+4`, z at `+6`.
    pub const CUR_POS: Global<[u16; 4]> = Global::new(0x0133_f7b4);
    /// Lulu's limit table (16 bytes).
    pub const LULU_LIMIT: Global<[u8; 16]> = Global::new(0x0133_f798);

    /// `TOBtlCloseHelpWin()` (`0x00890eb0`).
    pub fn close_help_win(mem: &mut impl Mem) {
        HELP_WIN.set(mem, 0);
    }

    /// `TOBtlInitAnm()` (`0x008a0cd0`).
    pub fn init_anm(mem: &mut impl Mem) {
        INIT_ANM.set(mem, 1);
    }

    /// `TOBtlCloseSignWindow()` (`0x00890ee0`).
    pub fn close_sign_window(mem: &mut impl Mem) {
        SIGN_WINDOW.set(mem, 0);
    }

    /// `TOBtlCloseSimpleHelpMes()` (`0x00890ef0`).
    pub fn close_simple_help(mem: &mut impl Mem) {
        SIMPLE_HELP.set(mem, 0);
    }

    /// `TOGetBtlWindowControlEnable()` (`0x0089b590`).
    pub fn window_control_enable(mem: &impl Mem) -> u32 {
        WINDOW_CONTROL.get(mem)
    }

    /// `TOBtlSetCurPos(x, y, z)` (`0x008999f0`): pushes a cursor position
    /// while fewer than 4 are pushed (the count is signed).
    pub fn set_cur_pos(mem: &mut impl Mem, x: u16, y: u16, z: u16) {
        let count = CUR_POS_COUNT.get(mem);
        if count >= 4 {
            return;
        }
        let r = CUR_POS
            .ptr(mem)
            .addr
            .wrapping_add(i32::from(count).cast_unsigned().wrapping_mul(8));
        mem.write(r, x);
        mem.write(r.wrapping_add(4), y);
        mem.write(r.wrapping_add(6), z);
        CUR_POS_COUNT.set(mem, count.wrapping_add(1));
    }

    /// `TOBtlSetLuluLimitTbl(digits)` (`0x0089a2f0`): 16 entries, each
    /// read back from the table and the digits as the original goes.
    pub fn set_lulu_limit_table(mem: &mut impl Mem, digits: u32) {
        let t = LULU_LIMIT.ptr(mem).addr;
        for i in 0..16 {
            let prev = if i == 0 { 0 } else { mem.read::<u8>(t + i - 1) };
            let d = mem.read::<u8>(digits.wrapping_add(i));
            mem.write(t + i, super::lulu_limit_step(prev, d));
        }
    }

    /// `TOBtlSetTargetCallback(w)` (`0x008a1090`): the window's target
    /// handlers (`+0x90`, `+0x98`, `+0x9c`, `+0xa0`; `+0x94` cleared), code
    /// addresses of this image.
    pub fn set_target_callback(mem: &mut impl Mem, w: u32) {
        let at = |o: u32| w.wrapping_add(o);
        mem.write(at(0x94), 0u32);
        for (o, code) in [
            (0x98, 0x008a_8780),
            (0x9c, 0x008a_8be0),
            (0xa0, 0x008a_2ac0),
            (0x90, 0x008a_84c0),
        ] {
            let c = mem.rebase(code);
            mem.write(at(o), c);
        }
    }
}
