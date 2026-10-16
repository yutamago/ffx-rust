//! Character texture animation (`tex_anim_*`, `chr_texanim_*`): the PS2
//! code that builds GS upload packets for animated textures, kept in the
//! remaster. The PS2 GS registers it once wrote are one sink word in the
//! port (`g_GsRegisterSink`): every register write lands there, in order.

#![allow(clippy::many_single_char_names)] // the originals' word arguments

/// The 64-bit DMA tag `FUN_0077eb70` packs: `(((((a << 31) | (b & !0xf))
/// << 1 | c) << 3 | d) << 2 | e) << 26 | f`, the ORs into the low word.
#[must_use]
pub fn dma_tag(a: u32, b: u32, c: u32, d: u32, e: u32, f: u32) -> u64 {
    let mut v = u64::from(a) << 31;
    v |= u64::from(b & !0xf);
    v = (v << 1) | u64::from(c);
    v = (v << 3) | u64::from(d);
    v = (v << 2) | u64::from(e);
    (v << 26) | u64::from(f)
}

/// The GIF tag words `FUN_0077ebe0` writes: `[p8 << 15 | p9,
/// ((((p4 << 2 | p5) << 11 | p6) << 1 | p7) << 14, p2, p3]`.
#[must_use]
pub fn gif_tag(p: [u32; 8]) -> [u32; 4] {
    let [p2, p3, p4, p5, p6, p7, p8, p9] = p;
    let w0 = p8 << 15 | p9;
    let w1 = ((((p4 << 2 | p5) << 11 | p6) << 1) | p7) << 14;
    [w0, w1, p2, p3]
}

/// The originals, over game memory.
pub mod bridge {
    use crate::host::{Conv, Host};
    use ffx_bridge::{Global, Mem};

    /// `g_GsRegisterSink`: the word the PS2 GS register writes land in.
    pub const GS_SINK: Global<u32> = Global::new(0x0230_56e8);
    /// `g_ChrTexAnim`: 64 entries of `0x30` bytes (`+4` the work buffer,
    /// `+0xd` the format: 0 old, 1 new, `+0xe` the timer byte).
    pub const CHR_TEXANIM: Global<u32> = Global::new(0x0231_0000);

    /// `tex_anim_no_set_new(i, a, lo, hi, d, e)`.
    pub const NO_SET_NEW: u32 = 0x0077_fce0;
    /// `tex_anim_no_set_old(i, a, b)`.
    pub const NO_SET_OLD: u32 = 0x0078_07c0;
    /// `sceGsSetDefDrawEnv2()`, which `tex_anim_dlg` asks first.
    pub const DLG_CHECK: u32 = 0x0088_7c60;
    /// `chr_anim_win(arg)`, which `tex_anim_dlg` tail-calls when it says 1.
    pub const DLG_RUN: u32 = 0x0093_e510;

    fn sink(m: &mut impl Mem, v: u32) {
        GS_SINK.set(m, v);
    }

    /// An address the PS2 code maps down (`>= 0x70000000`: minus
    /// `0x80000000`).
    fn ps2_address(a: u32) -> u32 {
        if a >= 0x7000_0000 {
            a.wrapping_sub(0x8000_0000)
        } else {
            a
        }
    }

    /// `x / 16` rounding toward zero (`cdq; and edx, 0xf; add; sar 4`).
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_wrap)]
    fn qwords(x: u32) -> u32 {
        ((x as i32) / 16) as u32
    }

    /// `loadImage(addr, bytes)` (`final` 0x100) and `FUN_0077e7c0`
    /// (`final` 0x101): the address, the quadword count, the command.
    /// Returns the quadwords (`eax`).
    pub fn load_image(m: &mut impl Mem, addr: u32, bytes: u32, command: u32) -> u32 {
        sink(m, ps2_address(addr));
        let q = qwords(bytes);
        sink(m, q);
        sink(m, command);
        q
    }

    /// `FUN_0077e770(a, b, bytes)`: a register set-up (`0xff0000`, masked
    /// `& 0x7cf`), `a`, `b`, the quadwords, then command 0x100.
    /// Returns the quadwords (`eax`).
    pub fn load_image_regs(m: &mut impl Mem, a: u32, b: u32, bytes: u32) -> u32 {
        sink(m, 0x00ff_0000);
        let v = GS_SINK.get(m) & 0x7cf;
        sink(m, v);
        sink(m, a);
        sink(m, b);
        let q = qwords(bytes);
        sink(m, q);
        sink(m, 0x100);
        q
    }

    /// `FUN_0077e800(a)` / `FUN_0077eaa0(a)`: `a`, 0, then command 0x105.
    /// Returns `a` (`eax`).
    pub fn sync_path(m: &mut impl Mem, a: u32) -> u32 {
        sink(m, a);
        sink(m, 0);
        sink(m, 0x105);
        a
    }

    /// `FUN_0077ebc0(q, a, b, c)`: the quadword `[b, c, a, 0]`. Returns `a`.
    pub fn qword(m: &mut impl Mem, q: u32, a: u32, b: u32, c: u32) -> u32 {
        m.write(q, b);
        m.write(q + 4, c);
        m.write(q + 8, a);
        m.write(q + 0xc, 0u32);
        a
    }

    /// `FUN_0077eb70(q, a..f)`: the DMA tag at `q` (low word, then high,
    /// stored high first). Returns `q`.
    pub fn dma_tag(m: &mut impl Mem, q: u32, p: [u32; 6]) -> u32 {
        let v = super::dma_tag(p[0], p[1], p[2], p[3], p[4], p[5]);
        #[allow(clippy::cast_possible_truncation)] // the halves
        {
            m.write(q + 4, (v >> 32) as u32);
            m.write(q, v as u32);
        }
        q
    }

    /// `FUN_0077ebe0(q, p2..p9)`: the GIF tag at `q`. Returns `p3`.
    pub fn gif_tag(m: &mut impl Mem, q: u32, p: [u32; 8]) -> u32 {
        for (i, w) in (0u32..).zip(super::gif_tag(p)) {
            m.write(q + 4 * i, w);
        }
        p[1]
    }

    /// Entry `i` of `g_ChrTexAnim`.
    fn entry(m: &impl Mem, i: u32) -> u32 {
        CHR_TEXANIM.ptr(m).addr.wrapping_add(i.wrapping_mul(0x30))
    }

    /// `FUN_0077e9f0(i)`: entry `i`'s work buffer (`+4`).
    pub fn work(m: &impl Mem, i: u32) -> u32 {
        m.read(entry(m, i) + 4)
    }

    /// `tex_anim_timer(i, v)`: entry `i`'s timer byte (`+0xe`). Returns
    /// `v` in `al`.
    pub fn timer(m: &mut impl Mem, i: u32, v: u32) -> u8 {
        #[allow(clippy::cast_possible_truncation)] // the byte argument
        let b = v as u8;
        let at = entry(m, i) + 0xe;
        m.write(at, b);
        b
    }

    /// `tex_anim_no_set(i, a, b, c, d)`: by entry `i`'s format, the new
    /// set (`b` split into its two low bytes) or the old one; other
    /// formats do nothing and return the format less 1 (`eax`).
    pub fn no_set(h: &mut impl Host, i: u32, a: u32, b: u32, c: u32, d: u32) -> u32 {
        let format = i32::from(h.read::<i8>(entry(h, i) + 0xd));
        match format {
            1 => h.call_sym(
                NO_SET_NEW,
                Conv::Cdecl,
                &[i, a, b & 0xff, (b >> 8) & 0xff, c, d],
            ),
            0 => h.call_sym(NO_SET_OLD, Conv::Cdecl, &[i, a, b & 0xff]),
            #[allow(clippy::cast_sign_loss)] // eax
            _ => (format - 1) as u32,
        }
    }

    /// `tex_anim_dlg(arg)`: when the check says 1, `chr_anim_win(arg)`'s
    /// result; else 0.
    pub fn dlg(h: &mut impl Host, arg: u32) -> u32 {
        if h.call_sym(DLG_CHECK, Conv::Cdecl, &[]) == 1 {
            h.call_sym(DLG_RUN, Conv::Cdecl, &[arg])
        } else {
            0
        }
    }

    /// One animation's reset: `+0xc` = `v`, `+0x1c`, `+0x20` and the
    /// floats `+0x2c..+0x38` = 0.
    fn reset_one(m: &mut impl Mem, at: u32, v: u32) {
        m.write(at + 0xc, v);
        for off in [0x1c, 0x20, 0x2c, 0x30, 0x34, 0x38] {
            m.write(at + off, 0u32);
        }
    }

    /// `TexAnim_ResetPos(i, which, v)` (`FUN_0077fda0`): for a loaded entry
    /// (`+0` set), resets animation `which` of its work buffer (`0x34`
    /// bytes each), or with `which == 1` every one of the 64 whose active
    /// byte (`+0xd08 + n`) is 1.
    pub fn reset_pos(m: &mut impl Mem, i: u32, which: u32, v: u32) {
        let e = entry(m, i);
        if m.read::<u32>(e) == 0 {
            return;
        }
        let work: u32 = m.read(e + 4);
        if which == 1 {
            for n in 0..64u32 {
                if m.read::<u8>(work + 0xd08 + n) == 1 {
                    reset_one(m, work + 0x34 * n, v);
                }
            }
        } else {
            reset_one(m, work.wrapping_add(which.wrapping_mul(0x34)), v);
        }
    }
}
