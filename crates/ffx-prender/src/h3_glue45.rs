//! The render-side glue, forty-fifth batch (M4 port round 12, list
//! `h3-glue-render`): the auto-test manager's console thread.

/// The command line buffer (100 bytes), `crt_memset_thunk` (cdecl), the
/// import slots of `std::basic_istream::getline(char *, __int64)`
/// (thiscall, `ret 0xc`) and of `std::cin` (the object's address),
/// `printf` and `strstr` (cdecl), and their strings (`"Input:%s\n"`,
/// `"EnableAutoTest"`).
pub const BUFFER: u32 = 0x00cd_4b10;
pub const MEMSET: u32 = 0x0094_964c;
pub const GETLINE_SLOT: u32 = 0x00b0_c1fc;
pub const CIN_SLOT: u32 = 0x00b0_c200;
pub const PRINTF_SLOT: u32 = 0x00b0_c380;
pub const STRSTR_SLOT: u32 = 0x00b0_c378;
pub const ECHO_FORMAT: u32 = 0x00b4_8e34;
pub const ENABLE_PREFIX: u32 = 0x00b4_8e40;

/// The instance the commands go to (`[0x00cccb00]`), its flag `+0x129`,
/// its two command handlers (`this; line`, `ret 4`): `0x006bce00` for a
/// line starting with the prefix, `0x006bccb0` for the others while the
/// flag is set; `SleepThreadInMillisecond(ms)` (cdecl).
pub const INSTANCE: u32 = 0x00cc_cb00;
pub const ENABLE_COMMAND: u32 = 0x006b_ce00;
pub const OTHER_COMMAND: u32 = 0x006b_ccb0;
pub const SLEEP: u32 = 0x006f_2d20;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_graphic::host::{Conv, Host};

    /// One pass of [`input_thread`]'s loop, with the `printf` and
    /// `strstr` the thread read from their import slots at its start:
    /// the buffer cleared, a line (at most 99 bytes) read from `std::cin`
    /// (both slots read each pass); a non-empty line is echoed and goes
    /// to the instance's handler `0x006bce00` when `strstr` finds the
    /// prefix at its start, else to `0x006bccb0` when its `+0x129` is
    /// set. Then 5 ms of sleep.
    pub fn input_pass(h: &mut impl Host, printf: u32, strstr: u32) {
        let buffer = h.rebase(BUFFER);
        h.call_sym(MEMSET, Conv::Cdecl, &[buffer, 0, 0x64]);
        let cin: u32 = h.read(h.rebase(CIN_SLOT));
        let getline: u32 = h.read(h.rebase(GETLINE_SLOT));
        h.call(getline, Conv::Thiscall(cin), &[buffer, 0x63, 0]);
        if h.read::<u8>(buffer) != 0 {
            let format = h.rebase(ECHO_FORMAT);
            h.call(printf, Conv::Cdecl, &[format, buffer]);
            let prefix = h.rebase(ENABLE_PREFIX);
            let found = h.call(strstr, Conv::Cdecl, &[buffer, prefix]);
            let instance: u32 = h.read(h.rebase(INSTANCE));
            if found == buffer {
                h.call_sym(ENABLE_COMMAND, Conv::Thiscall(instance), &[buffer]);
            } else if h.read::<u8>(instance + 0x129) != 0 {
                h.call_sym(OTHER_COMMAND, Conv::Thiscall(instance), &[buffer]);
            }
        }
        h.call_sym(SLEEP, Conv::Cdecl, &[5]);
    }

    /// The thread procedure `0x006bd1e0` (started by
    /// `AutoTestManager_ctor_006bca90` with a 32 KB stack; its argument
    /// is never read): reads `printf` and `strstr` from their import
    /// slots, then runs [`input_pass`] forever.
    pub fn input_thread(h: &mut impl Host) -> ! {
        let printf: u32 = h.read(h.rebase(PRINTF_SLOT));
        let strstr: u32 = h.read(h.rebase(STRSTR_SLOT));
        loop {
            input_pass(h, printf, strstr);
        }
    }
}
