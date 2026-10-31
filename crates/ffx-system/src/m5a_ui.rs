//! M5 `system-a` (round 3): the graphic UI object's constructor
//! (`0x0067ad30`, from `GraphicUi_CreateInstance_0067b6c0`): its record
//! arrays sized and filled from the buffers `GraphicUi_AllocBuffers`
//! makes; and a shadow gradient mesh's vertex set-up (`0x006790c0`).

/// The originals over game memory and the original callees.
pub mod bridge {
    use ffx_bridge::x87;
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

    /// The three constants `0x006790c0` loads: an `f32` (-1.0, the cloth
    /// flip), an `f32`, and a double subtracted from the doubled offset.
    pub const GRAD_K1: u32 = 0x00b9_2340;
    pub const GRAD_K2: u32 = 0x00b9_2194;
    pub const GRAD_D: u32 = 0x00b9_2248;
    /// The frame of `0x006790c0`: its five factors at `ebp - 0x14..-4`.
    pub const GRAD_FRAME: u32 = 0x14;

    /// `0x006790c0(this; a, mode, c, d)` (thiscall, `ret 0x10`; from
    /// `ShadowManager_setupGradMeshesB`): a gradient mesh's three vertex
    /// blocks and index list. By `mode` the offset `q` and the four
    /// scales `s1..s4` (all rounded to `f32` in the frame): 1: `q = (a -
    /// c) / (d - c)`, `s1 = c / a`; 2: `s3 = c / a`, `s4 = d / a`; 3: `q =
    /// -c / (d - c)`, `s1 = s2 = s3 = 0`, `s4 = d / a`; otherwise `q = 0`,
    /// the scales 1. The positions (`+0x1c`, 24 floats) take the two
    /// constants and `r = 2q - D` (rounded into the caller's `c` slot,
    /// `slots + 8`), the second block (`+0x17c`, 16) `q`, the third
    /// (`+0xfc`, 24) the scales; the 12 indices at `+0x1bc`. Returns
    /// `this + 0x1c`, as `eax` ends.
    #[allow(clippy::many_single_char_names)] // the original's operands
    pub fn grad_mesh(h: &mut impl Host, ebp: u32, this: u32, mode: u32, slots: u32) -> u32 {
        let at = |off: u32| ebp.wrapping_sub(off);
        let arg = |k: u32| slots.wrapping_add(4 * k);
        let a = x87::load(h.read(arg(0)));
        let c = x87::load(h.read(arg(2)));
        let d = x87::load(h.read(arg(3)));
        let (mut q, mut s) = (0.0, [1.0; 4]);
        match mode {
            1 => {
                q = x87::div(x87::sub(a, c), x87::sub(d, c));
                s[0] = x87::div(c, a);
            }
            3 => {
                q = x87::div(x87::sub(0.0, c), x87::sub(d, c));
                s = [0.0, 0.0, 0.0, x87::div(d, a)];
            }
            2 => {
                s[2] = x87::div(c, a);
                s[3] = x87::div(d, a);
            }
            _ => {}
        }
        // the frame: q at -4, s1 -0x10, s2 -0x14, s3 -8, s4 -0xc
        h.write(at(4), x87::store(q));
        for (off, v) in [0x10u32, 0x14, 8, 0xc].into_iter().zip(s) {
            h.write(at(off), x87::store(v));
        }
        let q_bits: u32 = h.read(at(4));
        let q = x87::load(q_bits);
        let k1: u32 = h.read(h.rebase(GRAD_K1));
        let k2: u32 = h.read(h.rebase(GRAD_K2));
        let dd: f64 = h.read(h.rebase(GRAD_D));
        let r_bits = x87::store(x87::sub(x87::add(q, q), dd));
        h.write(slots.wrapping_add(8), r_bits);
        let (one, zero) = (1f32.to_bits(), 0f32.to_bits());
        let (k1, k2) = (x87::store(x87::load(k1)), x87::store(x87::load(k2)));
        let r = x87::store(x87::load(r_bits));
        let positions = [
            k1, k1, k2, one, k1, k2, k1, r, k2, one, r, k2, k1, r, k2, one, r, k2, k1, one, k2,
            one, one, k2,
        ];
        let q = q_bits;
        let second = [
            zero, zero, one, zero, zero, q, one, q, zero, q, one, q, zero, one, one, one,
        ];
        let s: [u32; 4] = [0x10u32, 0x14, 8, 0xc].map(|off| h.read(at(off)));
        let mut third = Vec::with_capacity(24);
        for sk in s {
            third.extend([one, sk, one, one, sk, one]);
        }
        for (block, values) in [
            (0x1cu32, &positions[..]),
            (0x17c, &second[..]),
            (0xfc, &third[..]),
        ] {
            for (k, v) in (0u32..).zip(values) {
                h.write(this.wrapping_add(block + 4 * k), *v);
            }
        }
        for (k, v) in (0u32..).zip([0u32, 1, 2, 3, 2, 1, 4, 5, 6, 7, 6, 5]) {
            h.write(this.wrapping_add(0x1bc + 4 * k), v);
        }
        this.wrapping_add(0x1c)
    }
}
