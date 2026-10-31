//! M5 `system-a` (round 3): the graphic UI object's constructor
//! (`0x0067ad30`, from `GraphicUi_CreateInstance_0067b6c0`): its record
//! arrays sized and filled from the buffers `GraphicUi_AllocBuffers`
//! makes.

/// The originals over game memory and the original callees.
pub mod bridge {
    use ffx_graphic::host::{Conv, Host};

    /// `GraphicUi_AllocBuffers_0067b770` (thiscall).
    pub const ALLOC_BUFFERS: u32 = 0x0067_b770;
    /// `Phyre_PhyreMalloc_0042fb80(size, align)` and `Phyre_PhyreFree`
    /// (cdecl).
    pub const MALLOC_ALIGNED: u32 = 0x0042_fb80;
    pub const PHYRE_FREE: u32 = 0x0042_fb20;
    /// `MenuRecord_InitSingle_0067c2d0` / `MenuRecord_InitMulti_0067c030`
    /// (thiscall, `ret 4`).
    pub const INIT_SINGLE: u32 = 0x0067_c2d0;
    pub const INIT_MULTI: u32 = 0x0067_c030;
    /// `crt_memset_thunk` (cdecl).
    pub const MEMSET: u32 = 0x0094_964c;
    /// The pointer stored at `+0x5b88`.
    pub const DEFAULT_5B88: u32 = 0x00b4_5ba0;

    /// The 4-byte array at `this + at` (`+0` its count, its top bit set
    /// for a borrowed block; `+4` the block) resized to `n` elements: a
    /// zeroed aligned block (none when that fails), the old one freed
    /// when owned and different.
    fn resize(h: &mut impl Host, this: u32, at: u32, n: u32) {
        let count_at = this.wrapping_add(at);
        if h.read::<u32>(count_at) == n {
            return;
        }
        let p = h.call_sym(MALLOC_ALIGNED, Conv::Cdecl, &[n * 4, 4]);
        if p == 0 {
            return;
        }
        for k in 0..n {
            h.write(p.wrapping_add(4 * k), 0u32);
        }
        let old: u32 = h.read(count_at.wrapping_add(4));
        if old != p && h.read::<u32>(count_at) >> 31 == 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
        }
        h.write(count_at.wrapping_add(4), p);
        h.write(count_at, n);
    }

    /// The array at `this + at` filled: element `i` = `init(this;
    /// buffer)`, the buffer moving on by the word at `this + step` after
    /// each. Returns the buffer after the last.
    fn fill(
        h: &mut impl Host,
        this: u32,
        at: u32,
        n: u32,
        init: u32,
        step: u32,
        mut buf: u32,
    ) -> u32 {
        for k in 0..n {
            let block: u32 = h.read(this.wrapping_add(at + 4));
            let slot = block.wrapping_add(4 * k);
            let v = h.call_sym(init, Conv::Thiscall(this), &[buf]);
            h.write(slot, v);
            buf = buf.wrapping_add(h.read(this.wrapping_add(step)));
        }
        buf
    }

    /// `0x0067ad30(this)` (thiscall): the four array headers (`+0x15e0`,
    /// `+0x2bd0`, `+0x4238`, `+0x5828`, four words each) and `+0x2ec30`
    /// cleared, the buffers allocated (`GraphicUi_AllocBuffers`), then
    /// from the first buffer (`+0x5b80`) 600 single records into the
    /// array at `+0x15e0`, four more into `+0x2be4..=+0x2bf0`, 800 multi
    /// records into `+0x15e8`, and from the second (`+0x5b84`) 600 single
    /// and 800 multi into `+0x4238` and `+0x4240` (the record sizes
    /// `+0x5b94` and `+0x5b98`); the rest of the state reset (`+0x5848`
    /// 100 words, `+0..+0x15e0` zeroed, `+0x5b7c` 1). Returns `this`.
    pub fn ctor(h: &mut impl Host, this: u32) -> u32 {
        let at = |off: u32| this.wrapping_add(off);
        for base in [0x15e0u32, 0x2bd0, 0x4238, 0x5828] {
            for k in 0..4 {
                h.write(at(base + 4 * k), 0u32);
            }
        }
        h.write(at(0x2ec30), 0u32);
        h.call_sym(ALLOC_BUFFERS, Conv::Thiscall(this), &[]);
        let buf: u32 = h.read(at(0x5b80));
        h.write(at(0x2be0), 0u32);
        resize(h, this, 0x15e0, 0x258);
        let mut buf = fill(h, this, 0x15e0, 0x258, INIT_SINGLE, 0x5b94, buf);
        for off in [0x2be4u32, 0x2be8, 0x2bec, 0x2bf0] {
            let v = h.call_sym(INIT_SINGLE, Conv::Thiscall(this), &[buf]);
            buf = buf.wrapping_add(h.read(at(0x5b94)));
            h.write(at(off), v);
        }
        resize(h, this, 0x15e8, 0x320);
        fill(h, this, 0x15e8, 0x320, INIT_MULTI, 0x5b98, buf);
        let buf: u32 = h.read(at(0x5b84));
        resize(h, this, 0x4238, 0x258);
        let buf = fill(h, this, 0x4238, 0x258, INIT_SINGLE, 0x5b94, buf);
        resize(h, this, 0x4240, 0x320);
        fill(h, this, 0x4240, 0x320, INIT_MULTI, 0x5b98, buf);
        for k in 0..0x64u32 {
            h.write(at(0x5848 + 4 * k), 0u32);
        }
        let p = h.rebase(DEFAULT_5B88);
        h.write(at(0x5b88), p);
        for off in [
            0x2c50u32, 0x5b78, 0x2bf8, 0x2bfc, 0x5840, 0x5838, 0x583c, 0x5b68,
        ] {
            h.write(at(off), 0u32);
        }
        h.call_sym(MEMSET, Conv::Cdecl, &[this, 0, 0x15e0]);
        h.write(at(0x5ba0), 0u32);
        h.write(at(0x5b74), 0u8);
        h.write(at(0x5b6c), 0u32);
        h.write(at(0x5b7c), 1u32);
        h.write(at(0x5b8c), 0u32);
        h.write(at(0x5844), 0u32);
        h.write(at(0x5b9c), 0u16);
        this
    }
}
