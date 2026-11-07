//! M5 stage 1, round 9 (`tail9`, list `phyre-media`): `PhyreEngine`'s
//! movie player `PVideoPlaybackWin32` around the libraries `ffx_movie`
//! replaced (docs/formats/platform.md, "The libraries in Rust"): its two
//! threads (reading, decoding), the FMOD stream's read callback and
//! shutdown, the clock, pause and state queries, `initialize`,
//! `terminate`, and the texture uploads of a decoded frame
//! (`MapTexture`, `0x005a8550`).

/// The originals over game memory.
#[allow(clippy::many_single_char_names)] // the planes and the original's registers
pub mod bridge {
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// `PVideoPlaybackWin32_DemuxLoop` (`0x00a27650`, thiscall; Rust since
    /// m5-movie), `PVideoPlaybackWin32_DecodeLoop` (`0x00a279b0`, thiscall),
    /// `PVideoPlaybackWin32_AudioRead_00a27170` (thiscall `(out, unused)`,
    /// `ret 8`), `PVideoPlaybackWin32_close` (`0x00a28350`, thiscall), the
    /// reading thread's body (`0x00a27880`, its start routine).
    pub const DEMUX_LOOP: u32 = 0x00a2_7650;
    pub const DECODE_LOOP: u32 = 0x00a2_79b0;
    pub const AUDIO_READ: u32 = 0x00a2_7170;
    pub const CLOSE: u32 = 0x00a2_8350;
    pub const READING_THREAD: u32 = 0x00a2_7880;
    /// `PhyreEngine`'s Win32 primitives (thiscall): `PEventFlagWin32`'s
    /// `wait`, `initialize(manual)` (`ret 4`), `signal`;
    /// `PThreadInitializationWin32(start)` (`ret 4`), `PThreadWin32`'s
    /// create `(&init)` (`ret 4`) and `join(&code)` (`ret 4`);
    /// `Phyre_ReturnZero_00430260` (cdecl, `ecx` unread);
    /// `Phyre_CloseHandle_00430240` (thiscall).
    pub const EVENT_WAIT: u32 = 0x0043_02a0;
    pub const EVENT_INIT: u32 = 0x0043_0050;
    pub const EVENT_SIGNAL: u32 = 0x0043_0200;
    pub const THREAD_INIT: u32 = 0x0042_fca0;
    pub const THREAD_CREATE: u32 = 0x0043_00b0;
    pub const THREAD_JOIN: u32 = 0x0043_01b0;
    pub const RETURN_ZERO: u32 = 0x0043_0260;
    pub const CLOSE_HANDLE: u32 = 0x0043_0240;
    /// The thread name the reading thread is created with (`0x00b78bdc`).
    pub const READING_NAME: u32 = 0x00b7_8bdc;
    /// `CDataFrameQueue` (thiscall): `Close`, `Destroy`, `Front`, `Pop`,
    /// `Reserve(len)` (`ret 4`), `Commit`, `Count`.
    pub const QUEUE_CLOSE: u32 = 0x00a3_dea0;
    pub const QUEUE_DESTROY: u32 = 0x00a3_df20;
    pub const QUEUE_FRONT: u32 = 0x00a3_dd90;
    pub const QUEUE_POP: u32 = 0x00a3_ddc0;
    pub const QUEUE_RESERVE: u32 = 0x00a3_dec0;
    pub const QUEUE_COMMIT: u32 = 0x00a3_df00;
    pub const QUEUE_COUNT: u32 = 0x00a3_dde0;
    /// libvpx's `vpx_codec_decode(ctx, data, size, user, deadline)` and
    /// `vpx_codec_get_frame(ctx, &iter)` (cdecl; Rust since m5-movie).
    pub const VPX_DECODE: u32 = 0x0040_9680;
    pub const VPX_GET_FRAME: u32 = 0x0040_96e0;
    /// The CRT's `memcpy` thunk (cdecl).
    pub const MEMCPY: u32 = 0x0094_9256;
    /// FMOD Ex import thunks (stdcall, the object first):
    /// `Sound::getOpenState(state*, percent*, starving*, busy*)`,
    /// `Sound::getUserData(&data)`, `Sound::release()`, `Channel::stop()`,
    /// `Channel::setPaused(paused)`.
    pub const SOUND_GET_OPEN_STATE: u32 = 0x00a7_c8f2;
    pub const SOUND_GET_USER_DATA: u32 = 0x00a7_c8fe;
    pub const SOUND_RELEASE: u32 = 0x0094_8cfa;
    pub const CHANNEL_STOP: u32 = 0x00a7_c904;
    pub const CHANNEL_SET_PAUSED: u32 = 0x00a7_c90a;
    /// Import slot of `KERNEL32!Sleep` (stdcall `(ms)`).
    pub const SLEEP_SLOT: u32 = 0x00b0_c100;
    /// libVorbis' clears (cdecl, Rust since m5-movie) and libogg's
    /// allocator reset `SeveralStoresCopyIntoGlobal_0042e200` (cdecl).
    pub const VORBIS_BLOCK_CLEAR: u32 = 0x00a9_2040;
    pub const VORBIS_DSP_CLEAR: u32 = 0x00a9_17a0;
    pub const VORBIS_COMMENT_CLEAR: u32 = 0x00a9_0030;
    pub const VORBIS_INFO_CLEAR: u32 = 0x00a8_fd80;
    pub const OGG_RESET: u32 = 0x0042_e200;
    /// The constants `getCurrentTime` reads: `2^32` (`0x00b0dbb0`) and
    /// `2^63` (`0x00b13bd8`), doubles.
    pub const TWO_32: u32 = 0x00b0_dbb0;
    pub const TWO_63: u32 = 0x00b1_3bd8;
    /// `PTexture2DD3D11` (thiscall): `isBound` (-> `al`), `bind`, the
    /// two level uploads `(level, out)` (`ret 8`): `0x00596b90` (the
    /// mapping) and `Phyre_PTextureD3D11_uploadLevel_0059d8f0`; the
    /// vtable of the placeholder texture `MapTexture` refuses
    /// (`0x00ca34cc`).
    pub const TEX_IS_BOUND: u32 = 0x0059_5f90;
    pub const TEX_BIND: u32 = 0x0058_db00;
    pub const TEX_MAP_LEVEL: u32 = 0x0059_6b90;
    pub const TEX_UPLOAD_LEVEL: u32 = 0x0059_d8f0;
    pub const PLACEHOLDER_VTABLE: u32 = 0x00ca_34cc;

    fn k64(h: &impl Host, sym: u32) -> f64 {
        f64::from_bits(h.read(h.rebase(sym)))
    }

    /// `_ftol2` (`0x00949816`): toward zero as a 64-bit integer; NaN and
    /// anything outside `i64` give the x87's integer indefinite.
    fn ftol2_64(x: f64) -> u64 {
        if x.is_nan() || x >= 9_223_372_036_854_775_808.0 || x < -9_223_372_036_854_775_808.0 {
            0x8000_0000_0000_0000
        } else {
            #[allow(clippy::cast_possible_truncation)] // in range, checked
            let v = x.trunc() as i64;
            v.cast_unsigned()
        }
    }

    /// The compiler's double to `u64`: `x` below `c` (`2^63`; NaN too)
    /// through `_ftol2`; else `x - c`, and `2^63` added back when `x - c`
    /// stays below `c`.
    fn to_u64(x: f64, c: f64) -> u64 {
        // fcom: not (c <= x) when c > x or unordered
        if c.partial_cmp(&x)
            .is_none_or(|o| o == std::cmp::Ordering::Greater)
        {
            return ftol2_64(x);
        }
        let y = x87::sub(x, c);
        if c > y {
            ftol2_64(y).wrapping_add(0x8000_0000_0000_0000)
        } else {
            ftol2_64(y)
        }
    }

    /// `PVideoPlaybackWin32__ReadingThread(player)` (`0x00a27880`, cdecl,
    /// the reading thread's start routine) -> 0: `+0xd4` = 1 (running);
    /// until `+0xd0` (stop) is 1: wait on the event `+0x2ac`, then while
    /// playing or paused (`+0xe4` 1 or 2) and not at the end (`+0xe8` !=
    /// 2) one demux pass; `+0xd4` = 2 (done).
    pub fn reading_thread(h: &mut impl Host, player: u32) -> u32 {
        let at = |o: u32| player.wrapping_add(o);
        let stop: u32 = h.read(at(0xd0));
        h.write(at(0xd4), 1_u32);
        if stop != 1 {
            loop {
                h.call_sym(EVENT_WAIT, Conv::Thiscall(at(0x2ac)), &[]);
                let state: u32 = h.read(at(0xe4));
                if (state == 1 || state == 2) && h.read::<u32>(at(0xe8)) != 2 {
                    h.call_sym(DEMUX_LOOP, Conv::Thiscall(player), &[]);
                }
                if h.read::<u32>(at(0xd0)) == 1 {
                    break;
                }
            }
        }
        h.write(at(0xd4), 2_u32);
        0
    }

    /// `PVideoPlaybackWin32_AudioShutdown(player)` (`0x00a278f0`,
    /// thiscall) -> the libogg reset's `eax`: the audio queue `+0x80`
    /// closed, the channel `+0x2a0` stopped; the sound `+0x29c` polled
    /// with `getOpenState` into `state` (the original's `[ebp-4]`) until
    /// the call succeeds with state 0 (`Sleep(1)` between successful
    /// polls of another state; `Sleep` read from its import slot once),
    /// then released; `+0x29c` and `+0x2a0` cleared; the Vorbis block,
    /// synthesis, comment and info states (`+0x220`, `+0x1b0`, `+0x1a0`,
    /// `+0x180`) cleared; the queue destroyed; libogg's allocators reset.
    pub fn audio_shutdown(h: &mut impl Host, player: u32, state: u32) -> u32 {
        let at = |o: u32| player.wrapping_add(o);
        h.call_sym(QUEUE_CLOSE, Conv::Thiscall(at(0x80)), &[]);
        let channel: u32 = h.read(at(0x2a0));
        h.call_sym(CHANNEL_STOP, Conv::Stdcall, &[channel]);
        let sleep: u32 = h.read(h.rebase(SLEEP_SLOT));
        loop {
            let sound: u32 = h.read(at(0x29c));
            let r = h.call_sym(
                SOUND_GET_OPEN_STATE,
                Conv::Stdcall,
                &[sound, state, 0, 0, 0],
            );
            if r != 0 {
                continue;
            }
            if h.read::<u32>(state) == 0 {
                break;
            }
            h.call(sleep, Conv::Stdcall, &[1]);
        }
        let sound: u32 = h.read(at(0x29c));
        h.call_sym(SOUND_RELEASE, Conv::Stdcall, &[sound]);
        h.write(at(0x29c), 0_u32);
        h.write(at(0x2a0), 0_u32);
        h.call_sym(VORBIS_BLOCK_CLEAR, Conv::Cdecl, &[at(0x220)]);
        h.call_sym(VORBIS_DSP_CLEAR, Conv::Cdecl, &[at(0x1b0)]);
        h.call_sym(VORBIS_COMMENT_CLEAR, Conv::Cdecl, &[at(0x1a0)]);
        h.call_sym(VORBIS_INFO_CLEAR, Conv::Cdecl, &[at(0x180)]);
        h.call_sym(QUEUE_DESTROY, Conv::Thiscall(at(0x80)), &[]);
        h.call_sym(OGG_RESET, Conv::Cdecl, &[])
    }

    /// One plane of a decoded image into the frame slot: `rows` rows of
    /// `width` bytes from `src` (stride `stride`) to `dst` (packed); one
    /// `memcpy` when the strides agree, none when `rows` or `width` is 0.
    fn copy_plane(h: &mut impl Host, dst: u32, src: u32, stride: u32, width: u32, rows: u32) {
        if rows == 0 || width == 0 {
            return;
        }
        if stride == width {
            h.call_sym(MEMCPY, Conv::Cdecl, &[dst, src, stride.wrapping_mul(rows)]);
            return;
        }
        let (mut d, mut s) = (dst, src);
        for _ in 0..rows {
            h.call_sym(MEMCPY, Conv::Cdecl, &[d, s, width]);
            s = s.wrapping_add(stride);
            d = d.wrapping_add(width);
        }
    }

    /// `PVideoPlaybackWin32_DecodeLoop(player)` (`0x00a279b0`, thiscall)
    /// -> the queue close's `eax`: until `+0xec` (stop) is 1, each packet
    /// at the front of the packet queue (the player itself; none: stop)
    /// is decoded (`vpx_codec_decode(+0x130, data + 4, [data], 0, 0)`)
    /// and popped; a frame `vpx_codec_get_frame` gives (`iter`, the
    /// original's `[ebp-0x10]`, zeroed first) goes into a slot of the
    /// frame queue `+0x40` (`Reserve(+0x2d0)`; none: stop): Y (`+0x14c`
    /// by `+0x150`, from image `+0x24`/`+0x34`) at the slot `+ 4`, U and
    /// V (`+0x2c0` by `+0x2c4`, image `+0x28`/`+0x38` and `+0x2c`/`+0x3c`)
    /// at `+ 4 + [+0x2c8]` and that `+ [+0x2cc]`, then `Commit`. At the
    /// end the frame queue is closed and `+0x154` (decoding) cleared.
    pub fn decode_loop(h: &mut impl Host, player: u32, iter: u32) -> u32 {
        let at = |o: u32| player.wrapping_add(o);
        if h.read::<u32>(at(0xec)) != 1 {
            loop {
                let packet = h.call_sym(QUEUE_FRONT, Conv::Thiscall(player), &[]);
                if packet == 0 {
                    break;
                }
                let size: u32 = h.read(packet);
                h.call_sym(
                    VPX_DECODE,
                    Conv::Cdecl,
                    &[at(0x130), packet.wrapping_add(4), size, 0, 0],
                );
                h.call_sym(QUEUE_POP, Conv::Thiscall(player), &[]);
                h.write(iter, 0_u32);
                let img = h.call_sym(VPX_GET_FRAME, Conv::Cdecl, &[at(0x130), iter]);
                if img != 0 {
                    let len: u32 = h.read(at(0x2d0));
                    let slot = h.call_sym(QUEUE_RESERVE, Conv::Thiscall(at(0x40)), &[len]);
                    if slot == 0 {
                        break;
                    }
                    let y = slot.wrapping_add(4);
                    let (rows, width) = (h.read(at(0x150)), h.read(at(0x14c)));
                    let (stride, src) = (
                        h.read(img.wrapping_add(0x34)),
                        h.read(img.wrapping_add(0x24)),
                    );
                    copy_plane(h, y, src, stride, width, rows);
                    let u = y.wrapping_add(h.read(at(0x2c8)));
                    let (rows, width) = (h.read(at(0x2c4)), h.read(at(0x2c0)));
                    let (stride, src) = (
                        h.read(img.wrapping_add(0x38)),
                        h.read(img.wrapping_add(0x28)),
                    );
                    copy_plane(h, u, src, stride, width, rows);
                    let v = u.wrapping_add(h.read(at(0x2cc)));
                    let (rows, width) = (h.read(at(0x2c4)), h.read(at(0x2c0)));
                    let (stride, src) = (
                        h.read(img.wrapping_add(0x3c)),
                        h.read(img.wrapping_add(0x2c)),
                    );
                    copy_plane(h, v, src, stride, width, rows);
                    h.call_sym(QUEUE_COMMIT, Conv::Thiscall(at(0x40)), &[]);
                }
                if h.read::<u32>(at(0xec)) == 1 {
                    break;
                }
            }
        }
        let r = h.call_sym(QUEUE_CLOSE, Conv::Thiscall(at(0x40)), &[]);
        h.write(at(0x154), 0_u8);
        r
    }

    /// `PVideoPlaybackWin32__VideoDecodingThread(player)` (`0x00a27b90`,
    /// cdecl, the decoding thread's start routine) -> 0: the decode loop.
    pub fn decoding_thread(h: &mut impl Host, player: u32) -> u32 {
        h.call_sym(DECODE_LOOP, Conv::Thiscall(player), &[]);
        0
    }

    /// `PVideoPlaybackWin32_getCurrentTime(player)` (`0x00a27bd0`,
    /// thiscall) -> ms in `edx:eax`: `(frames +0x100 + 1)` (unsigned:
    /// `2^32` added when the incremented word is negative) times the
    /// frame time `+0x108` (double), as `u64`.
    pub fn get_current_time(h: &mut impl Host, player: u32) -> u64 {
        let n = h.read::<u32>(player.wrapping_add(0x100)).wrapping_add(1);
        let mut x = f64::from(n.cast_signed());
        if n.cast_signed() < 0 {
            x = x87::add(x, k64(h, TWO_32));
        }
        let ms = f64::from_bits(h.read(player.wrapping_add(0x108)));
        let t = x87::mul(x, ms);
        to_u64(t, k64(h, TWO_63))
    }

    /// `PVideoPlaybackWin32_getEndTime(player)` (`0x00a27c40`, thiscall)
    /// -> the end time `+0xf8` (`edx:eax`).
    pub fn get_end_time(h: &impl Host, player: u32) -> u64 {
        h.read(player.wrapping_add(0xf8))
    }

    /// `PVideoPlaybackWin32_isPlaying(player)` (`0x00a27c50`, thiscall)
    /// -> `al`: 1 while decoding (`+0x154`), while an audio track
    /// (`+0x294`) has a sound that is not open yet (the sound `+0x29c`'s
    /// `getOpenState` into `state`, the original's `[ebp-4]`, succeeding
    /// with a state other than 0), or while frames are queued (`+0x40`);
    /// else 0.
    pub fn is_playing(h: &mut impl Host, player: u32, state: u32) -> u8 {
        let at = |o: u32| player.wrapping_add(o);
        let mut ready = true;
        let sound: u32 = h.read(at(0x29c));
        if sound != 0 {
            let r = h.call_sym(
                SOUND_GET_OPEN_STATE,
                Conv::Stdcall,
                &[sound, state, 0, 0, 0],
            );
            if r == 0 {
                ready = h.read::<u32>(state) == 0;
            }
        }
        if h.read::<u8>(at(0x154)) != 0 {
            return 1;
        }
        if h.read::<u32>(at(0x294)) != 0 && !ready {
            return 1;
        }
        u8::from(h.call_sym(QUEUE_COUNT, Conv::Thiscall(at(0x40)), &[]) != 0)
    }

    /// `PVideoPlaybackWin32_initialize(player; unused, frames, system)`
    /// (`0x00a27cf0`, thiscall, `ret 0xc`) -> the thread create's `eax`:
    /// the FMOD system `+0x298`, the flags `+0xd0`, `+0xd4`, `+0xe4`,
    /// `+0xe8` and `+0x154` cleared, the queue length `+0x158` (`frames`,
    /// 30 for 0), the event `+0x2ac` (manual reset) and `+0x2b0` = 1;
    /// the reading thread `+0xc0` created from a
    /// `PThreadInitializationWin32` at `ebp - 0x18` (the reading thread's
    /// routine; then `+0x14` 1, `+0xc` and `+4` 0, `+0x10` the name, `+8`
    /// the player) and `+0xc4` = 1.
    pub fn initialize(h: &mut impl Host, ebp: u32, player: u32, frames: u32, system: u32) -> u32 {
        let at = |o: u32| player.wrapping_add(o);
        h.write(at(0x298), system);
        for o in [0xd0, 0xd4, 0xe4, 0xe8] {
            h.write(at(o), 0_u32);
        }
        h.write(at(0x154), 0_u8);
        let n = if frames == 0 { 0x1e } else { frames };
        h.write(at(0x158), n);
        h.call_sym(EVENT_INIT, Conv::Thiscall(at(0x2ac)), &[1]);
        h.write(at(0x2b0), 1_u8);
        let init = ebp.wrapping_sub(0x18);
        let routine = h.rebase(READING_THREAD);
        h.call_sym(THREAD_INIT, Conv::Thiscall(init), &[routine]);
        let name = h.rebase(READING_NAME);
        h.write(ebp.wrapping_sub(4), 1_u32);
        h.write(ebp.wrapping_sub(0xc), 0_u32);
        h.write(ebp.wrapping_sub(0x14), 0_u32);
        h.write(ebp.wrapping_sub(8), name);
        h.write(ebp.wrapping_sub(0x10), player);
        let r = h.call_sym(THREAD_CREATE, Conv::Thiscall(at(0xc0)), &[init]);
        h.write(at(0xc4), 1_u8);
        r
    }

    /// The level `MapTexture` and `0x005a8550` upload: `+8 + 1`, or 0
    /// when that is the texture's level count (`+0x24` without bit 31, 1
    /// for 0).
    fn next_level(h: &impl Host, tex: u32) -> u32 {
        let n = h.read::<u32>(tex.wrapping_add(8)).wrapping_add(1);
        let mut count = h.read::<u32>(tex.wrapping_add(0x24)) & 0x7fff_ffff;
        if count == 0 {
            count = 1;
        }
        if n == count { 0 } else { n }
    }

    /// `PVideoPlaybackWin32_MapTexture(texture; out)` (`0x00a27e20`,
    /// thiscall, `ret 4`) -> 0 or an error: 7 for the placeholder
    /// texture; an unbound texture is bound first (its error returned);
    /// then the next level mapped into `out` (`0x00596b90`), `+4` = 2
    /// (mapped) when that succeeds.
    pub fn map_texture(h: &mut impl Host, tex: u32, out: u32) -> u32 {
        let vt: u32 = h.read(tex);
        if vt == h.rebase(PLACEHOLDER_VTABLE) {
            return 7;
        }
        let [bound, ..] = h
            .call_sym(TEX_IS_BOUND, Conv::Thiscall(tex), &[])
            .to_le_bytes();
        if bound == 0 {
            let r = h.call_sym(TEX_BIND, Conv::Thiscall(tex), &[]);
            if r != 0 {
                return r;
            }
        }
        let level = next_level(h, tex);
        let r = h.call_sym(TEX_MAP_LEVEL, Conv::Thiscall(tex), &[level, out]);
        if r == 0 {
            h.write(tex.wrapping_add(4), 2_u8);
        }
        r
    }

    /// `PTexture2DD3D11_unmapLevel_005a8550(texture; data)` (`0x005a8550`,
    /// thiscall, `ret 4`) -> the upload's result: the next level (as
    /// `MapTexture` picks it) uploaded from `data`
    /// (`Phyre_PTextureD3D11_uploadLevel_0059d8f0`); on success `+8` = that
    /// level and `+4` = 0.
    pub fn unmap_level(h: &mut impl Host, tex: u32, data: u32) -> u32 {
        let level = next_level(h, tex);
        let r = h.call_sym(TEX_UPLOAD_LEVEL, Conv::Thiscall(tex), &[level, data]);
        if r == 0 {
            h.write(tex.wrapping_add(8), level);
            h.write(tex.wrapping_add(4), 0_u8);
        }
        r
    }

    /// `PVideoPlaybackWin32_togglePause(player)` (`0x00a27e90`, thiscall)
    /// -> `al` 1: playing (`+0xe4` 1) becomes paused (2) and the channel
    /// `+0x2a0` (when set) is paused; paused becomes playing and the
    /// channel resumed; other states stay.
    pub fn toggle_pause(h: &mut impl Host, player: u32) -> u8 {
        let at = |o: u32| player.wrapping_add(o);
        let state: u32 = h.read(at(0xe4));
        let (next, paused) = match state {
            1 => (2_u32, 1_u32),
            2 => (1, 0),
            _ => return 1,
        };
        let channel: u32 = h.read(at(0x2a0));
        h.write(at(0xe4), next);
        if channel != 0 {
            h.call_sym(CHANNEL_SET_PAUSED, Conv::Stdcall, &[channel, paused]);
        }
        1
    }

    /// `PVideoPlaybackWin32_FmodReadCallback_00a27ee0(sound, data, len)`
    /// (`0x00a27ee0`, stdcall, FMOD's PCM read callback) -> 0: the player
    /// from the sound's user data (into `slot`, the original's `[ebp-4]`,
    /// zeroed first) fills `data` (`AudioRead(data, len)`) when the call
    /// succeeds and gives one.
    pub fn fmod_read_callback(
        h: &mut impl Host,
        slot: u32,
        sound: u32,
        data: u32,
        len: u32,
    ) -> u32 {
        h.write(slot, 0_u32);
        let r = h.call_sym(SOUND_GET_USER_DATA, Conv::Stdcall, &[sound, slot]);
        if r == 0 {
            let player: u32 = h.read(slot);
            if player != 0 {
                h.call_sym(AUDIO_READ, Conv::Thiscall(player), &[data, len]);
            }
        }
        0
    }

    /// `PVideoPlaybackWin32_terminate(player)` (`0x00a28430`, thiscall)
    /// -> the event close's `eax`: `close`, `+0xd0` (stop) = 1, the event
    /// `+0x2ac` signalled, the reading thread `+0xc0` joined (its code
    /// into `code`, the original's `[ebp-4]`), `Phyre_ReturnZero`,
    /// `+0xc4` = 0, the event closed, `+0x2b0` = 0.
    pub fn terminate(h: &mut impl Host, player: u32, code: u32) -> u32 {
        let at = |o: u32| player.wrapping_add(o);
        h.call_sym(CLOSE, Conv::Thiscall(player), &[]);
        h.write(at(0xd0), 1_u32);
        h.call_sym(EVENT_SIGNAL, Conv::Thiscall(at(0x2ac)), &[]);
        h.call_sym(THREAD_JOIN, Conv::Thiscall(at(0xc0)), &[code]);
        h.call_sym(RETURN_ZERO, Conv::Cdecl, &[]);
        h.write(at(0xc4), 0_u8);
        let r = h.call_sym(CLOSE_HANDLE, Conv::Thiscall(at(0x2ac)), &[]);
        h.write(at(0x2b0), 0_u8);
        r
    }

    /// `PVideoPlaybackWin32_initialize` (`0x00a27cf0`), which `open` runs
    /// on a player not set up yet; `KERNEL32!GetTickCount`'s import slot
    /// and the tick word `open` keeps (`0x01a84c80`).
    pub const INITIALIZE: u32 = 0x00a2_7cf0;
    pub const GET_TICK_COUNT_SLOT: u32 = 0x00b0_c0a0;
    pub const OPEN_TICKS: u32 = 0x01a8_4c80;
    /// libvpx's allocator words (`0x00c6b274..0x00c6b280`: `malloc`,
    /// `calloc`, `realloc`, `free`), the player's three copy callbacks
    /// (`PVideoPlaybackWin32_memcpy/memset/memmove`), and
    /// `vpx_mem_set_functions`, `vpx_codec_vp8_dx`,
    /// `vpx_codec_dec_init_ver` (cdecl).
    pub const VPX_ALLOCATORS: u32 = 0x00c6_b274;
    pub const PLAYER_MEMCPY: u32 = 0x00a2_6fb0;
    pub const PLAYER_MEMSET: u32 = 0x00a2_6fd0;
    pub const PLAYER_MEMMOVE: u32 = 0x00a2_6fc0;
    pub const VPX_MEM_SET: u32 = 0x0040_a5d0;
    pub const VPX_VP8_DX: u32 = 0x0040_a430;
    pub const VPX_DEC_INIT: u32 = 0x0040_94c0;
    /// mkvparser's allocator words (`0x00c6b284`, `0x00c6b288`) and its
    /// calls: `SetAllocator(malloc, free)`, `Segment::CreateInstance(reader,
    /// pos (2 words), &segment)` (cdecl); `Segment::Load`, `GetTracks`,
    /// `GetDuration` (i64), `Tracks::GetTracksCount`,
    /// `GetTrackByIndex(i)` (`ret 4`), `Track::GetType`,
    /// `VideoTrack::GetFrameRate` (`ST0`), `GetWidth`, `GetHeight`
    /// (thiscall); `VbfMkvReader_Open(name)` (thiscall, `ret 4`).
    pub const MKV_ALLOCATORS: u32 = 0x00c6_b284;
    pub const MKV_SET_ALLOCATOR: u32 = 0x0041_e5b0;
    pub const SEGMENT_CREATE: u32 = 0x0041_fd50;
    pub const SEGMENT_LOAD: u32 = 0x0042_1450;
    pub const SEGMENT_TRACKS: u32 = 0x0042_51b0;
    pub const SEGMENT_DURATION: u32 = 0x0042_5200;
    pub const TRACKS_COUNT: u32 = 0x0042_94c0;
    pub const TRACK_BY_INDEX: u32 = 0x0042_9de0;
    pub const TRACK_TYPE: u32 = 0x0042_7b10;
    pub const FRAME_RATE: u32 = 0x0042_8dd0;
    pub const WIDTH: u32 = 0x0042_8db0;
    pub const HEIGHT: u32 = 0x0042_8dc0;
    pub const READER_OPEN: u32 = 0x00a3_e1f0;
    /// The doubles `open` reads: the frame rate for a track that gives 0
    /// (`0x00b78c48`), 1000 (`0x00b0dba0`) and the ns-to-ms factor applied
    /// twice (`0x00b921b0`).
    pub const DEFAULT_FPS: u32 = 0x00b7_8c48;
    pub const THOUSAND: u32 = 0x00b0_dba0;
    pub const NS_STEP: u32 = 0x00b9_21b0;
    /// `PVideoPlaybackWin32_AudioInit_00a273b0` (fastcall), `CDataFrameQueue_Init(capacity,
    /// extra, alloc, free)` (thiscall, `ret 0x10`), the queues' allocator
    /// words (`0x00c6b29c`, `0x00c6b2a0`), the decoding thread's start
    /// routine and name (`0x00b78c00`).
    pub const AUDIO_INIT: u32 = 0x00a2_73b0;
    pub const QUEUE_INIT: u32 = 0x00a3_ddf0;
    pub const QUEUE_ALLOCATORS: u32 = 0x00c6_b29c;
    pub const DECODING_THREAD: u32 = 0x00a2_7b90;
    pub const DECODING_NAME: u32 = 0x00b7_8c00;

    /// `open`'s frame below `ebp`: the decoding thread's
    /// `PThreadInitializationWin32` at `ebp - 0x28` (the original's
    /// scratch words above it, the frame rate and the conversions, are its
    /// own: nothing gets their address).
    pub const OPEN_FRAME: u32 = 0x28;

    /// `PVideoPlaybackWin32_open(player; path, looping)` (`0x00a27f20`,
    /// thiscall, `ret 8`) -> 0, or -1 (libvpx's allocators refused), -2
    /// (the decoder not made), -3 (the file not opened): the player closed
    /// (when set up, `+0xc4`) or initialized (30 frames, no FMOD system);
    /// `+0x2a8` (prebuffering) = 1, the frame count `+0x100` and the sound
    /// and channel cleared, the tick count kept, `+0xf0` = `looping`'s low
    /// byte; libvpx's allocators and the VP8 decoder `+0x130`;
    /// mkvparser's allocators, the reader `+0x118` opened on `path`, the
    /// segment `+0x110` made and loaded, the first video and audio tracks
    /// (`+0x290`, `+0x294`, by type); the frame time `+0x108` = 1000 / fps
    /// (the default fps for 0), the size `+0x14c`/`+0x150`, the end time
    /// `+0xf8` = duration(ns) x 1e-3 x 1e-3 as `u64`, the chroma size
    /// `+0x2c0`/`+0x2c4` (halves), the plane sizes `+0x2c8` (w h) and
    /// `+0x2cc` (w h / 4); no FMOD system drops the audio track; the frame
    /// count `+0x2bc` = end / fps (truncated, the low word); the audio set
    /// up (`AudioInit`; failing drops the track); the packet queue (the
    /// player, 4 MB) and frame queue `+0x40` of `+0x2d0` = 1.5 w h bytes a
    /// frame, `+0x158` frames; the decoding thread `+0xc8` (`+0xec` 0,
    /// `+0xcc` 1), the event `+0x2b4` (`+0x154` decoding 1, `+0x2b8` 1),
    /// playing (`+0xe4` 1), the reading event `+0x2ac` signalled.
    #[allow(clippy::too_many_lines)]
    pub fn open(h: &mut impl Host, ebp: u32, player: u32, path: u32, looping: u32) -> u32 {
        let at = |o: u32| player.wrapping_add(o);
        if h.read::<u8>(at(0xc4)) != 0 {
            h.call_sym(CLOSE, Conv::Thiscall(player), &[]);
        } else {
            h.call_sym(INITIALIZE, Conv::Thiscall(player), &[0, 0x1e, 0]);
        }
        h.write(at(0x2a8), 1_u8);
        h.write(at(0x100), 0_u32);
        h.write(at(0x29c), 0_u32);
        h.write(at(0x2a0), 0_u32);
        let ticks_fn: u32 = h.read(h.rebase(GET_TICK_COUNT_SLOT));
        let ticks = h.call(ticks_fn, Conv::Stdcall, &[]);
        h.write(h.rebase(OPEN_TICKS), ticks);
        h.write(at(0xf0), looping.to_le_bytes()[0]);
        let va = h.rebase(VPX_ALLOCATORS);
        let [m, c, r, f] = [0, 4, 8, 0xc].map(|o| h.read::<u32>(va.wrapping_add(o)));
        let copies = [PLAYER_MEMCPY, PLAYER_MEMSET, PLAYER_MEMMOVE].map(|s| h.rebase(s));
        let args = [m, c, r, f, copies[0], copies[1], copies[2]];
        if h.call_sym(VPX_MEM_SET, Conv::Cdecl, &args) != 0 {
            return 0xffff_ffff;
        }
        let iface = h.call_sym(VPX_VP8_DX, Conv::Cdecl, &[]);
        if h.call_sym(VPX_DEC_INIT, Conv::Cdecl, &[at(0x130), iface, 0, 0, 9]) != 0 {
            return 0xffff_fffe;
        }
        let ma = h.rebase(MKV_ALLOCATORS);
        let (mm, mf) = (h.read::<u32>(ma), h.read::<u32>(ma.wrapping_add(4)));
        h.call_sym(MKV_SET_ALLOCATOR, Conv::Cdecl, &[mm, mf]);
        let reader = at(0x118);
        if h.call_sym(READER_OPEN, Conv::Thiscall(reader), &[path]) != 0 {
            return 0xffff_fffd;
        }
        h.call_sym(SEGMENT_CREATE, Conv::Cdecl, &[reader, 0, 0, at(0x110)]);
        let seg: u32 = h.read(at(0x110));
        h.call_sym(SEGMENT_LOAD, Conv::Thiscall(seg), &[]);
        let seg: u32 = h.read(at(0x110));
        let tracks = h.call_sym(SEGMENT_TRACKS, Conv::Thiscall(seg), &[]);
        h.write(at(0x290), 0_u32);
        h.write(at(0x294), 0_u32);
        let n = h.call_sym(TRACKS_COUNT, Conv::Thiscall(tracks), &[]);
        for i in 0..n {
            let track = h.call_sym(TRACK_BY_INDEX, Conv::Thiscall(tracks), &[i]);
            match h.call_sym(TRACK_TYPE, Conv::Thiscall(track), &[]) {
                1 => h.write(at(0x290), track),
                2 => h.write(at(0x294), track),
                _ => {}
            }
        }
        let video: u32 = h.read(at(0x290));
        let mut fps = h.call_sym_st0_f64(FRAME_RATE, Conv::Thiscall(video), &[]);
        // fucomp with 0: only an ordered equal takes the default
        if fps == 0.0 {
            fps = f64::from_bits(h.read(h.rebase(DEFAULT_FPS)));
        }
        let thousand = f64::from_bits(h.read(h.rebase(THOUSAND)));
        h.write(at(0x108), x87::div(thousand, fps).to_bits());
        let video: u32 = h.read(at(0x290));
        let width = h.call_sym(WIDTH, Conv::Thiscall(video), &[]);
        h.write(at(0x14c), width);
        let video: u32 = h.read(at(0x290));
        let height = h.call_sym(HEIGHT, Conv::Thiscall(video), &[]);
        h.write(at(0x150), height);
        let seg: u32 = h.read(at(0x110));
        let ns = h.call_sym_u64(SEGMENT_DURATION, Conv::Thiscall(seg), &[]);
        // fild qword, fstp qword (rounded to a double), fld
        #[allow(clippy::cast_precision_loss)] // the original's rounding
        let x = ns.cast_signed() as f64;
        let step = f64::from_bits(h.read(h.rebase(NS_STEP)));
        let ms = x87::mul(x87::mul(x, step), step);
        let end = to_u64(ms, f64::from_bits(h.read(h.rebase(TWO_63))));
        h.write(at(0xf8), end);
        let (w, ht): (u32, u32) = (h.read(at(0x14c)), h.read(at(0x150)));
        let area = ht.wrapping_mul(w);
        h.write(at(0x2c0), w >> 1);
        h.write(at(0x2c8), area);
        h.write(at(0x2c4), ht >> 1);
        h.write(at(0x2cc), area >> 2);
        if h.read::<u32>(at(0x298)) == 0 {
            h.write(at(0x294), 0_u32);
        }
        // the end time as an unsigned 64-bit integer, a double
        #[allow(clippy::cast_precision_loss)] // the original's rounding
        let total = h.read::<u64>(at(0xf8)) as f64;
        let frames = x87::div(total, fps);
        h.write(at(0x2bc), x87::ftol2(frames));
        if h.read::<u32>(at(0x294)) != 0 {
            let r = h.call_sym(AUDIO_INIT, Conv::Fastcall, &[player, area >> 2]);
            if r.to_le_bytes()[0] == 0 {
                h.write(at(0x294), 0_u32);
            }
        }
        let qa = h.rebase(QUEUE_ALLOCATORS);
        let (qa0, qa1): (u32, u32) = (h.read(qa), h.read(qa.wrapping_add(4)));
        let plane: u32 = h.read(at(0x2c8));
        let frame = plane.wrapping_add(plane >> 1);
        h.write(at(0x2d0), frame);
        h.call_sym(
            QUEUE_INIT,
            Conv::Thiscall(player),
            &[0x40_0000, frame, qa0, qa1],
        );
        let (qa0, qa1): (u32, u32) = (h.read(qa), h.read(qa.wrapping_add(4)));
        let frame: u32 = h.read(at(0x2d0));
        let depth: u32 = h.read(at(0x158));
        h.call_sym(
            QUEUE_INIT,
            Conv::Thiscall(at(0x40)),
            &[frame.wrapping_add(4).wrapping_mul(depth), frame, qa0, qa1],
        );
        let init = ebp.wrapping_sub(0x28);
        let routine = h.rebase(DECODING_THREAD);
        h.write(at(0xec), 0_u32);
        h.call_sym(THREAD_INIT, Conv::Thiscall(init), &[routine]);
        let name = h.rebase(DECODING_NAME);
        h.write(ebp.wrapping_sub(0x14), 1_u32);
        h.write(ebp.wrapping_sub(0x1c), 0_u32);
        h.write(ebp.wrapping_sub(0x24), 0_u32);
        h.write(ebp.wrapping_sub(0x18), name);
        h.write(ebp.wrapping_sub(0x20), player);
        h.call_sym(THREAD_CREATE, Conv::Thiscall(at(0xc8)), &[init]);
        h.write(at(0xcc), 1_u8);
        h.write(at(0x154), 1_u8);
        h.call_sym(EVENT_INIT, Conv::Thiscall(at(0x2b4)), &[1]);
        h.write(at(0x2b8), 1_u8);
        h.write(at(0xe4), 1_u32);
        h.call_sym(EVENT_SIGNAL, Conv::Thiscall(at(0x2ac)), &[]);
        0
    }

    /// `PTexture2DD3D11` and `PTexture2DBase` methods `updateTexture`
    /// calls (thiscall): `unbind`, `setDimensions(w, h, format, 0)`
    /// (`ret 0x10`), `setBufferCount(n)` (`ret 4`), `updateMaxLevel`;
    /// `Phyre_PhyrePrintf(1, text)` (cdecl) and its text (`0x00b42278`);
    /// the format record (`g_TextureFormats_00ca2fcc`, the first).
    pub const TEX_UNBIND: u32 = 0x0059_cc40;
    pub const TEX_SET_DIMENSIONS: u32 = 0x004c_40c0;
    pub const TEX_SET_BUFFER_COUNT: u32 = 0x0059_9b70;
    pub const TEX_UPDATE_MAX_LEVEL: u32 = 0x004c_4250;
    pub const PRINTF: u32 = 0x0043_52f0;
    pub const BOUND_WARNING: u32 = 0x00b4_2278;
    pub const FORMAT: u32 = 0x00ca_2fcc;
    /// The mapping object's constructor (`Phyre_ReturnThis_00583460`) and
    /// destructor (`Phyre_Nop_00584f40`), thiscall;
    /// `PVideoPlaybackWin32_CopyPlane_00a26fe0` (cdecl).
    pub const MAPPING_CTOR: u32 = 0x0058_3460;
    pub const MAPPING_DTOR: u32 = 0x0058_4f40;
    pub const COPY_PLANE: u32 = 0x00a2_6fe0;
    /// `updateTexture`'s mapping object (`ebp - 0x74`, up to the cookie
    /// at `ebp - 0x10`).
    pub const MAPPING_SIZE: u32 = 0x64;

    fn is_bound(h: &mut impl Host, tex: u32) -> bool {
        let [b, ..] = h
            .call_sym(TEX_IS_BOUND, Conv::Thiscall(tex), &[])
            .to_le_bytes();
        b != 0
    }

    /// The mapping object's data pointer and pitch (`+4`, `+8`).
    fn mapped(h: &impl Host, mapping: u32) -> (u32, u32) {
        (
            h.read(mapping.wrapping_add(4)),
            h.read(mapping.wrapping_add(8)),
        )
    }

    /// One texture brought to `w` x `h` when its size (`+0x1c`, `+0x20`)
    /// differs: unbound when bound, resized (the first format), three
    /// buffers unless it has three (`+0x24` without bit 31; a bound one
    /// prints the warning first; `+8` = 0 when that succeeds), flag 2 of
    /// `+0x14` off around `updateMaxLevel`, flag 0x200 on, and bound again
    /// unless it is the placeholder or still bound.
    fn fit_texture(h: &mut impl Host, tex: u32, w: u32, ht: u32) {
        let at = |o: u32| tex.wrapping_add(o);
        let (tw, th): (u32, u32) = (h.read(at(0x1c)), h.read(at(0x20)));
        if (th ^ ht) | (tw ^ w) == 0 {
            return;
        }
        if is_bound(h, tex) {
            h.call_sym(TEX_UNBIND, Conv::Thiscall(tex), &[]);
        }
        let format = h.rebase(FORMAT);
        h.call_sym(TEX_SET_DIMENSIONS, Conv::Thiscall(tex), &[w, ht, format, 0]);
        let count = h.read::<u32>(at(0x24)) & 0x7fff_ffff;
        if count != 3 {
            if is_bound(h, tex) {
                let text = h.rebase(BOUND_WARNING);
                h.call_sym(PRINTF, Conv::Cdecl, &[1, text]);
            }
            if h.call_sym(TEX_SET_BUFFER_COUNT, Conv::Thiscall(tex), &[3]) == 0 {
                h.write(at(8), 0_u32);
            }
        }
        let flags: u32 = h.read(at(0x14));
        h.write(at(0x14), flags & !2);
        h.call_sym(TEX_UPDATE_MAX_LEVEL, Conv::Thiscall(tex), &[]);
        let flags: u32 = h.read(at(0x14));
        h.write(at(0x14), flags | 0x200);
        let vt: u32 = h.read(tex);
        if vt != h.rebase(PLACEHOLDER_VTABLE) && !is_bound(h, tex) {
            h.call_sym(TEX_BIND, Conv::Thiscall(tex), &[]);
        }
    }

    /// `Phyre_PVideo_PVideoPlaybackWin32_updateTexture(player; y, u, v)`
    /// (`0x00a284a0`, thiscall, `ret 0xc`) -> `al` 1 when a frame went
    /// into the three textures, else 0: only while playing (`+0xe4` 1);
    /// while prebuffering (`+0x2a8`) until the frame queue `+0x40` holds
    /// `+0x158` frames (then `+0x2a8` = 0); the textures fitted to the
    /// size (`+0x14c` x `+0x150`, the chroma ones to the halves); the
    /// front frame's planes (Y at `+4`, U `+ [+0x2c8]`, V `+ [+0x2cc]`)
    /// copied through the mapping object at `mapping` (the original's
    /// `ebp - 0x74`) into each texture `MapTexture` maps
    /// (`CopyPlane(data, pitch, src, w, pitch, rows)`, then
    /// `0x005a8550`); the frame count `+0x100` up, the frame popped.
    pub fn update_texture(
        h: &mut impl Host,
        mapping: u32,
        player: u32,
        y: u32,
        u: u32,
        v: u32,
    ) -> u8 {
        let at = |o: u32| player.wrapping_add(o);
        if h.read::<u32>(at(0xe4)) != 1 {
            return 0;
        }
        if h.read::<u8>(at(0x2a8)) != 0 {
            let n = h.call_sym(QUEUE_COUNT, Conv::Thiscall(at(0x40)), &[]);
            if n < h.read::<u32>(at(0x158)) {
                return 0;
            }
            h.write(at(0x2a8), 0_u8);
        }
        let (w, ht): (u32, u32) = (h.read(at(0x14c)), h.read(at(0x150)));
        fit_texture(h, y, w, ht);
        let (w, ht): (u32, u32) = (h.read(at(0x14c)), h.read(at(0x150)));
        fit_texture(h, u, w >> 1, ht >> 1);
        let (w, ht): (u32, u32) = (h.read(at(0x14c)), h.read(at(0x150)));
        fit_texture(h, v, w >> 1, ht >> 1);
        let frame = h.call_sym(QUEUE_FRONT, Conv::Thiscall(at(0x40)), &[]);
        if frame == 0 {
            return 0;
        }
        h.call_sym(MAPPING_CTOR, Conv::Thiscall(mapping), &[]);
        if h.call_sym(MAP_TEXTURE, Conv::Thiscall(y), &[mapping]) == 0 {
            let (dst, pitch) = mapped(h, mapping);
            let (w, rows): (u32, u32) = (h.read(at(0x14c)), h.read(at(0x150)));
            let src = frame.wrapping_add(4);
            h.call_sym(COPY_PLANE, Conv::Cdecl, &[dst, pitch, src, w, pitch, rows]);
            h.call_sym(UNMAP_LEVEL, Conv::Thiscall(y), &[mapping]);
        }
        let plane_u = frame.wrapping_add(4).wrapping_add(h.read(at(0x2c8)));
        if h.call_sym(MAP_TEXTURE, Conv::Thiscall(u), &[mapping]) == 0 {
            let (dst, pitch) = mapped(h, mapping);
            let rows: u32 = h.read(at(0x2c4));
            let w = h.read::<u32>(at(0x14c)) >> 1;
            h.call_sym(
                COPY_PLANE,
                Conv::Cdecl,
                &[dst, pitch, plane_u, w, pitch, rows],
            );
            h.call_sym(UNMAP_LEVEL, Conv::Thiscall(u), &[mapping]);
        }
        if h.call_sym(MAP_TEXTURE, Conv::Thiscall(v), &[mapping]) == 0 {
            let (dst, pitch) = mapped(h, mapping);
            let rows: u32 = h.read(at(0x2c4));
            let w = h.read::<u32>(at(0x14c)) >> 1;
            let plane_v = plane_u.wrapping_add(h.read(at(0x2cc)));
            h.call_sym(
                COPY_PLANE,
                Conv::Cdecl,
                &[dst, pitch, plane_v, w, pitch, rows],
            );
            h.call_sym(UNMAP_LEVEL, Conv::Thiscall(v), &[mapping]);
        }
        let n: u32 = h.read(at(0x100));
        h.write(at(0x100), n.wrapping_add(1));
        h.call_sym(QUEUE_POP, Conv::Thiscall(at(0x40)), &[]);
        h.call_sym(MAPPING_DTOR, Conv::Thiscall(mapping), &[]);
        1
    }

    /// `PVideoPlaybackWin32_MapTexture` (`0x00a27e20`) and
    /// `PTexture2DD3D11_unmapLevel_005a8550` (thiscall, `ret 4`).
    pub const MAP_TEXTURE: u32 = 0x00a2_7e20;
    pub const UNMAP_LEVEL: u32 = 0x005a_8550;
}
