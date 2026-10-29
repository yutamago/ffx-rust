//! More of list `phyre-framework-input` (batch 5): the application
//! script's constructor, the render-interface sync, the pad index, the
//! mapping's strongest axis value, the notifier map's range erase, the
//! script proxy's command table, and the motion blur's apply.

use ffx_bridge::Mem;
use ffx_graphic::host::{Conv, Host};

/// `PApplicationScriptBuffer`'s vtable.
pub const SCRIPT_BUFFER_VTABLE: u32 = 0x00b3_fc8c;
/// `PApplicationScript`'s vtable.
pub const SCRIPT_VTABLE: u32 = 0x00b3_fc94;
/// `PArray` copy-assign (thiscall `(source)`, `ret 4`).
pub const ARRAY_ASSIGN: u32 = 0x0044_79f0;
/// `PRenderScopeLock` enter / leave (thiscall on the lock word).
pub const SCOPE_LOCK_ENTER: u32 = 0x0046_3d50;
pub const SCOPE_LOCK_LEAVE: u32 = 0x0046_3e90;
/// The render interface (a static object) and its build-once guard.
pub const RENDER_INTERFACE: u32 = 0x00c9_4f00;
pub const RENDER_INTERFACE_GUARD: u32 = 0x00c9_628c;
/// Its constructor (thiscall) and the destructor handed to `atexit`.
pub const RENDER_INTERFACE_CTOR: u32 = 0x004f_ee70;
pub const RENDER_INTERFACE_AT_EXIT: u32 = 0x00af_b710;
/// `atexit(f)` (cdecl).
pub const ATEXIT: u32 = 0x0094_9207;
/// The render interface's sync (thiscall).
pub const RENDER_INTERFACE_SYNC: u32 = 0x0059_c400;
/// `g_PInputDeviceList`: the devices' list head (each device's link at
/// `+4`, the device at the link minus 4).
pub const DEVICE_LIST: u32 = 0x00c3_0fa8;
/// One mapped source's value (thiscall `(source)`, `ret 4`, `float` in
/// `ST0`; in the list).
pub const SOURCE_VALUE: u32 = 0x0062_8f00;
/// `std::map` clear (thiscall; in the list) and single erase (thiscall
/// `(&out, where)`, `ret 8`; in the list).
pub const MAP_CLEAR: u32 = 0x0062_6b90;
pub const MAP_ERASE: u32 = 0x0062_71d0;
/// `Phyre::PhyreMalloc(size)` / `PhyreFree(p)` (cdecl).
pub const PHYRE_MALLOC: u32 = 0x0042_fb40;
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// `memcpy(dst, src, n)` (cdecl).
pub const MEMCPY: u32 = 0x0094_9256;
/// Import slot of `MSVCR110!strncpy` (cdecl).
pub const STRNCPY_SLOT: u32 = 0x00b0_c368;
/// `PMotionBlurBase`'s apply (thiscall, 5 words, `ret 0x14`).
pub const BLUR_APPLY: u32 = 0x00a1_eb10;

/// The command table's growth: double the capacity, at least 16.
#[must_use]
pub const fn grown(cap: u32) -> u32 {
    let n = cap.wrapping_add(cap);
    if n > 0x10 { n } else { 0x10 }
}

/// The table's byte size for `cap` records of 0x48 bytes, -1 when it
/// overflows 32 bits (the original's `mul` / `seto` / `or`).
#[must_use]
pub const fn table_bytes(cap: u32) -> u32 {
    let full = cap as u64 * 0x48;
    if full > u32::MAX as u64 {
        u32::MAX
    } else {
        full as u32
    }
}

pub mod bridge {
    use super::{
        ARRAY_ASSIGN, ATEXIT, BLUR_APPLY, Conv, DEVICE_LIST, Host, MAP_CLEAR, MAP_ERASE, MEMCPY,
        Mem, PHYRE_FREE, PHYRE_MALLOC, RENDER_INTERFACE, RENDER_INTERFACE_AT_EXIT,
        RENDER_INTERFACE_CTOR, RENDER_INTERFACE_GUARD, RENDER_INTERFACE_SYNC, SCOPE_LOCK_ENTER,
        SCOPE_LOCK_LEAVE, SCRIPT_BUFFER_VTABLE, SCRIPT_VTABLE, SOURCE_VALUE, STRNCPY_SLOT, grown,
        table_bytes,
    };

    /// `PApplicationScript::PApplicationScript(owner, text)` (`0x0061fac0`,
    /// `ret 8`): the buffer's vtable and an empty `PArray` at `+4`, the
    /// text array copied in, the cursor `+0xc` and end `+0x10` over it,
    /// `+0x20` the owner, then its own vtable and `+0x14..+0x1c`, `+0x24`
    /// zero; `this`.
    pub fn script_ctor(h: &mut impl Host, this: u32, owner: u32, text: u32) -> u32 {
        let vt = h.rebase(SCRIPT_BUFFER_VTABLE);
        h.write(this, vt);
        h.write(this + 4, 0u32);
        h.write(this + 8, 0u32);
        h.call_sym(ARRAY_ASSIGN, Conv::Thiscall(this + 4), &[text]);
        let count: u32 = h.read(this + 4);
        let data: u32 = h.read(this + 8);
        h.write(this + 0x10, (count & 0x7fff_ffff).wrapping_add(data));
        h.write(this + 0x20, owner);
        h.write(this + 0xc, data);
        let vt = h.rebase(SCRIPT_VTABLE);
        h.write(this, vt);
        for off in [0x14, 0x18, 0x1c, 0x24] {
            h.write(this + off, 0u32);
        }
        this
    }

    /// `0x0062d3d0` (cdecl): under the render scope lock (its word at the
    /// frame address `lock`), the render interface (built on first use,
    /// its destructor registered with `atexit`) syncs; 0.
    pub fn render_sync(h: &mut impl Host, lock: u32) -> u32 {
        h.call_sym(SCOPE_LOCK_ENTER, Conv::Thiscall(lock), &[]);
        let guard = h.rebase(RENDER_INTERFACE_GUARD);
        let g: u32 = h.read(guard);
        let ri = h.rebase(RENDER_INTERFACE);
        if g & 1 == 0 {
            h.write(guard, g | 1);
            h.call_sym(RENDER_INTERFACE_CTOR, Conv::Thiscall(ri), &[]);
            let at_exit = h.rebase(RENDER_INTERFACE_AT_EXIT);
            h.call_sym(ATEXIT, Conv::Cdecl, &[at_exit]);
        }
        h.call_sym(RENDER_INTERFACE_SYNC, Conv::Thiscall(ri), &[]);
        h.call_sym(SCOPE_LOCK_LEAVE, Conv::Thiscall(lock), &[]);
        0
    }

    /// The `index`-th pad (device type 2, vtable slot 1) of the device
    /// list, or 0 when the list has fewer.
    fn nth_pad(h: &mut impl Host, index: u32) -> u32 {
        let head = h.rebase(DEVICE_LIST);
        let mut seen = 0;
        let mut link: u32 = h.read(head);
        while link != head && link != 0 && link != 4 {
            let dev = link - 4;
            if h.call_virtual(dev, 1, &[]) == 2 {
                if seen == index {
                    return dev;
                }
                seen += 1;
            }
            link = h.read(dev + 4);
        }
        0
    }

    /// `PInput::GetPadIndex` (`0x00628170`, thiscall on the device): the
    /// first `i < 18` whose `i`-th pad is `this` (a null `this` matches a
    /// missing pad), or -1. The original rescans the list for each `i`.
    pub fn pad_index(h: &mut impl Host, this: u32) -> u32 {
        for i in 0..0x12 {
            if nth_pad(h, i) == this {
                return i;
            }
        }
        u32::MAX
    }

    /// `0x00629620` (thiscall `(mapping)`, `ret 4`): the value of the
    /// mapping's sources (`+0xc`, count `+8`) with the largest magnitude
    /// (the first on ties; NaNs never win), 0 without any, in `ST0`.
    pub fn strongest_value(h: &mut impl Host, this: u32, mapping: u32) -> f32 {
        let mut best = 0.0f32;
        if mapping == 0 {
            return best;
        }
        let n = h.read::<u32>(mapping + 8) & 0x7fff_ffff;
        for i in 0..n {
            let sources: u32 = h.read(mapping + 0xc);
            let source: u32 = h.read(sources.wrapping_add(i.wrapping_mul(4)));
            let v = h.call_sym_st0(SOURCE_VALUE, Conv::Thiscall(this), &[source]);
            if best.abs() < v.abs() {
                best = v;
            }
        }
        best
    }

    fn is_nil(m: &impl Mem, node: u32) -> bool {
        m.read::<u8>(node + 0xd) != 0
    }

    /// The in-order successor (a nil node stays).
    fn next_node(m: &impl Mem, node: u32) -> u32 {
        if is_nil(m, node) {
            return node;
        }
        let right: u32 = m.read(node + 8);
        if !is_nil(m, right) {
            let mut cur = right;
            loop {
                let left: u32 = m.read(cur);
                if is_nil(m, left) {
                    return cur;
                }
                cur = left;
            }
        }
        let mut cur = node;
        let mut p: u32 = m.read(node + 4);
        while !is_nil(m, p) && cur == m.read::<u32>(p + 8) {
            cur = p;
            p = m.read(p + 4);
        }
        p
    }

    /// `std::map::erase(first, last)` (`0x00627130`, thiscall `(out,
    /// first, last)`, `ret 0xc`): the whole map is cleared when the range
    /// is `begin()..end()`, else each node is erased (its single erase
    /// writes an iterator into the frame word `local`); `*out` = the
    /// iterator after the range; `out`.
    pub fn erase_range(
        h: &mut impl Host,
        this: u32,
        out: u32,
        first: u32,
        last: u32,
        local: u32,
    ) -> u32 {
        let head: u32 = h.read(this);
        let begin: u32 = h.read(head);
        if first == begin && last == head {
            h.call_sym(MAP_CLEAR, Conv::Thiscall(this), &[]);
            let head: u32 = h.read(this);
            let begin: u32 = h.read(head);
            h.write(out, begin);
            return out;
        }
        let mut it = first;
        while it != last {
            let cur = it;
            it = next_node(h, cur);
            h.call_sym(MAP_ERASE, Conv::Thiscall(this), &[local, cur]);
        }
        h.write(out, it);
        out
    }

    /// `PApplicationScriptProxy`'s command registration (`0x0062bab0`,
    /// thiscall `(handler, name)`, `ret 8`): the 0x48-byte record table
    /// (`+0x14` data, `+0x18` count, `+0x1c` capacity) grows when full
    /// ([`grown`], [`table_bytes`]; the old records copied, the old table
    /// freed; kept as it was when the allocation fails), then the record
    /// `(handler, name[0x40] by strncpy, NUL at +0x44)` is appended while
    /// there is room. Its callers read no result.
    pub fn add_command(h: &mut impl Host, this: u32, handler: u32, name: u32) {
        let cap: u32 = h.read(this + 0x1c);
        let count: u32 = h.read(this + 0x18);
        if count >= cap {
            let cap = grown(cap);
            let table = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[table_bytes(cap)]);
            if table != 0 {
                let count: u32 = h.read(this + 0x18);
                if count != 0 {
                    let old: u32 = h.read(this + 0x14);
                    h.call_sym(MEMCPY, Conv::Cdecl, &[table, old, count.wrapping_mul(0x48)]);
                }
                let old: u32 = h.read(this + 0x14);
                if old != 0 {
                    h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
                }
                h.write(this + 0x14, table);
                h.write(this + 0x1c, cap);
            }
        }
        let count: u32 = h.read(this + 0x18);
        if count < h.read::<u32>(this + 0x1c) {
            let table: u32 = h.read(this + 0x14);
            let rec = table.wrapping_add(count.wrapping_mul(0x48));
            h.write(this + 0x18, count.wrapping_add(1));
            h.write(rec, handler);
            let f: u32 = h.read(h.rebase(STRNCPY_SLOT));
            h.call(f, Conv::Cdecl, &[rec + 4, name, 0x40]);
            h.write(rec + 0x44, 0u8);
        }
    }

    /// `PMotionBlur` slot 6 (`0x00a21c40`, thiscall `(a, b, target, d,
    /// e)`, `ret 0x14`): unless already set up (`+0x10`), the virtual
    /// `+4` with the target's size (`+0x1c`, `+0x20`; 0, 0 without one),
    /// whose non-zero answer is returned; 5 without a target; the virtual
    /// `+8` when `+8 != +0xc`; with a valid `+8` the apply `(a, b, a's
    /// +0x5c, target, d)`, its `eax`; else 0x15. `e` is never read.
    pub fn motion_blur_apply(h: &mut impl Host, this: u32, args: [u32; 4]) -> u32 {
        let [a, b, target, d] = args;
        let ready: u8 = h.read(this + 0x10);
        if ready == 0 {
            let (w, hgt) = if target == 0 {
                (0, 0)
            } else {
                (h.read(target + 0x1c), h.read(target + 0x20))
            };
            let r = h.call_virtual(this, 1, &[w, hgt]);
            if r != 0 {
                return r;
            }
        }
        if target == 0 {
            return 5;
        }
        let cur: u32 = h.read(this + 8);
        if cur != h.read::<u32>(this + 0xc) {
            h.call_virtual(this, 2, &[]);
        }
        let cur: u32 = h.read(this + 8);
        if cur != 0 && h.read::<u32>(cur) != 0 {
            let a5c: u32 = h.read(a + 0x5c);
            return h.call_sym(BLUR_APPLY, Conv::Thiscall(this), &[a, b, a5c, target, d]);
        }
        0x15
    }
}
