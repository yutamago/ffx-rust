//! M5 finish, WP5: originals the ports still called in Unicorn, by the
//! blocks gate run 1 emulated for them (ffx-rust's profile; docs/host.md,
//! "Rust and Unicorn"). Each follows its original's machine code
//! (`objdump -d FFX.exe`).

use ffx_bridge::Mem;

/// The render thread's job list (`PRenderThread_processRenderCommands`
/// builds two with `eh vector constructor iterator`, element size
/// `0x200c`): a 16-bit count at `+0`, a word at `+4`, 1024 two-word
/// entries from `+8`, a word at `+0x2008`.
pub const JOB_LIST_ENTRIES: u32 = 0x400;

/// `PRenderThread_JobList_ctor_00613f70(this)` (thiscall, no stack words):
/// the count (`u16` at `+0`, the two bytes after it untouched), the word
/// at `+4`, every entry and the word at `+0x2008` zeroed; `this` in
/// `eax`. 107 M of the emulated blocks of gate run 1 (8 replays), the most
/// of any `FFX.exe` function the ports call.
pub fn job_list_ctor(m: &mut impl Mem, this: u32) -> u32 {
    m.write(this, 0u16);
    m.write(this + 4, 0u32);
    let entries = vec![0u8; (8 * JOB_LIST_ENTRIES) as usize];
    m.write_bytes(this + 8, &entries);
    m.write(this + 8 + 8 * JOB_LIST_ENTRIES, 0u32);
    this
}
