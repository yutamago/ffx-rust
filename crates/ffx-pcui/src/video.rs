//! The movie player (`PVideoPlaybackWin32`): its Vorbis audio decoded to
//! interleaved 16-bit samples for the FMOD stream.

/// A decoded float sample as a 16-bit one: `f32(x * 32767 + 0.5)`
/// (`fmul qword`, `fadd qword`, `fstp dword`), `floor`, back to `f32`,
/// `ftol` (`to_int`), clamped to `-0x8000..=0x7fff`; `true` when clamped.
/// NaN (`ftol`'s integer indefinite) clamps to `-0x8000`.
pub fn sample(x: f32, to_int: impl FnOnce(f64) -> u32) -> (u16, bool) {
    use ffx_bridge::x87;
    let t = x87::load(x87::store(x87::add(x87::mul(f64::from(x), 32767.0), 0.5)));
    let v = to_int(x87::load(x87::store(t.floor()))).cast_signed();
    if v > 0x7fff {
        (0x7fff, true)
    } else if v < -0x8000 {
        (0x8000, true)
    } else {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // i16's bits
        (v as u16, false)
    }
}

/// The originals, where the game keeps their state (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::{Mem, x87};
    use ffx_graphic::host::{Conv, Host};

    /// `vorbis_synthesis_pcmout(vd, &pcm)`, `vorbis_synthesis_read(vd, n)`,
    /// `vorbis_synthesis(vb, op)`, `vorbis_synthesis_blockin(vd, vb)`
    /// (libvorbis, `cdecl`; the player's `vd` at `+0x1b0`, `vb` at
    /// `+0x220`, `op` at `+0x160`).
    pub const PCMOUT: u32 = 0x00a9_0a10;
    pub const PCM_READ: u32 = 0x00a9_09e0;
    pub const SYNTHESIS: u32 = 0x00a9_2460;
    pub const BLOCKIN: u32 = 0x00a9_0a70;
    /// The packet queue at `+0x80` (`thiscall`, Phyre): the front packet
    /// (its length word, then its bytes) or null, and its release.
    pub const QUEUE_FRONT: u32 = 0x00a3_dd90;
    pub const QUEUE_POP: u32 = 0x00a3_ddc0;
    /// The import slots of `__iob_func` and `fprintf` (MSVCR110).
    pub const IOB_FUNC_SLOT: u32 = 0x00b0_c350;
    pub const FPRINTF_SLOT: u32 = 0x00b0_c38c;
    /// `"Clipping in frame %ld\n"`.
    pub const CLIPPING_FORMAT: u32 = 0x00b7_8c2c;
    /// Bytes of work [`audio_read`] needs: the `pcm` pointer
    /// `vorbis_synthesis_pcmout` stores.
    pub const WORK: u32 = 4;

    /// `PVideoPlaybackWin32_AudioRead_00a27170(player, out, _)` (name ours;
    /// `thiscall`, `ret 8`, the second word unread; from the FMOD read
    /// callback `0x00a27ee0`, which drops the result): `+0x2a4` samples per
    /// channel into `out`, interleaved over the `+0x184` channels.
    ///
    /// Nothing when `+0x2a8` is set (`al` 0). Else, while samples are
    /// wanted and `+0xd8` is not 1: the decoded samples
    /// ([`PCMOUT`], at most the rest) go through [`super::sample`] channel
    /// by channel (the channel count re-read at each step, a clamped one
    /// printed as `"Clipping in frame %ld"` with `+0x1f0` on `stderr`,
    /// once per block) and are consumed ([`PCM_READ`]); with none decoded,
    /// packets from the queue at `+0x80` (until it is empty, `+0xd8` is 1,
    /// or one gives a block: `+0x178` counts them as a `u64`, `+0x160` the
    /// bytes, `+0x164` the length) go to [`SYNTHESIS`] and, when it
    /// succeeds, [`BLOCKIN`]. Returns 1 when nothing is left to write
    /// (or `+0xf0` is clear and samples are left), else 0.
    pub fn audio_read(h: &mut impl Host, this: u32, out: u32, work: u32) -> u32 {
        let at = |off: u32| this.wrapping_add(off);
        if h.read::<u8>(at(0x2a8)) != 0 {
            return 0;
        }
        let channels = |h: &mut _| -> i32 { Mem::read::<i32>(h, at(0x184)) };
        let stopped = |h: &mut _| Mem::read::<u32>(h, at(0xd8)) == 1;
        let mut remaining: i32 = h.read(at(0x2a4));
        let mut out = out;
        while remaining > 0 && !stopped(h) {
            let n = h
                .call_sym(PCMOUT, Conv::Cdecl, &[at(0x1b0), work])
                .cast_signed();
            if n != 0 {
                let n = n.min(remaining);
                convert(h, this, out, n, work);
                h.call_sym(PCM_READ, Conv::Cdecl, &[at(0x1b0), n.cast_unsigned()]);
                let step = channels(h).wrapping_mul(n).wrapping_mul(2);
                out = out.wrapping_add_signed(step);
                remaining = remaining.wrapping_sub(n);
                continue;
            }
            while !stopped(h) {
                let packet = h.call_sym(QUEUE_FRONT, Conv::Thiscall(at(0x80)), &[]);
                if packet == 0 {
                    break;
                }
                let len: u32 = h.read(packet);
                let count: u64 = h.read(at(0x178));
                h.write(at(0x178), count.wrapping_add(1));
                h.write(at(0x160), packet.wrapping_add(4));
                h.write(at(0x164), len);
                let block = h.call_sym(SYNTHESIS, Conv::Cdecl, &[at(0x220), at(0x160)]) == 0
                    && h.call_sym(BLOCKIN, Conv::Cdecl, &[at(0x1b0), at(0x220)]) == 0;
                h.call_sym(QUEUE_POP, Conv::Thiscall(at(0x80)), &[]);
                if block {
                    break;
                }
            }
        }
        let left = if h.read::<u8>(at(0xf0)) == 0 && remaining > 0 {
            0
        } else {
            remaining
        };
        u32::from(left == 0)
    }

    /// One decoded block of `n` samples per channel into `out`.
    #[allow(clippy::many_single_char_names)] // the loop's pointers and counts
    fn convert(h: &mut impl Host, this: u32, out: u32, n: i32, work: u32) {
        let channels = |h: &mut _| -> i32 { Mem::read::<i32>(h, this.wrapping_add(0x184)) };
        let mut clipped = false;
        if channels(h) <= 0 {
            return;
        }
        let mut ch = 0i32;
        let mut dst = out;
        loop {
            let pcm: u32 = h.read(work);
            let src: u32 = h.read(pcm.wrapping_add_signed(ch.wrapping_mul(4)));
            let mut p = dst;
            let mut i = 0i32;
            while i < n {
                let x: f32 = h.read(src.wrapping_add_signed(i.wrapping_mul(4)));
                let (v, clip) = super::sample(x, |t| x87::ftol(h, t));
                clipped |= clip;
                h.write(p, v);
                p = p.wrapping_add_signed(channels(h).wrapping_mul(2));
                i += 1;
            }
            ch += 1;
            dst = dst.wrapping_add(2);
            if ch >= channels(h) {
                break;
            }
        }
        if clipped {
            let frame: u32 = h.read(this.wrapping_add(0x1f0));
            let iob_func: u32 = h.read(h.rebase(IOB_FUNC_SLOT));
            let iob = h.call(iob_func, Conv::Cdecl, &[]);
            let fprintf: u32 = h.read(h.rebase(FPRINTF_SLOT));
            let format = h.rebase(CLIPPING_FORMAT);
            h.call(
                fprintf,
                Conv::Cdecl,
                &[iob.wrapping_add(0x40), format, frame],
            );
        }
    }
}
