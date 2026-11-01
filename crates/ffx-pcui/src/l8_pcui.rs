//! M5 stage 1, round 8 (list `left8`, from `pc-ui.list`): Iggy `GDraw`'s
//! D3D11 context (set-up, teardown, raw textures) and the video player's
//! FMOD audio stream set-up (`PVideoPlaybackWin32_AudioInit`).

/// The originals over game memory.
pub mod bridge {
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// `g_GDrawState`, the `GDraw` D3D11 state block.
    pub const GDRAW: u32 = 0x01a8_58b4;

    /// The state block's object slots `GDraw_releaseObjects_00a35c90`
    /// releases, in its order: the shaders and input layouts (every
    /// second word from `+0xc`, `+0x9c`, `+0x104`, `+0x204`), `+0x254`,
    /// `+0x25c`, three pairs `+0x260 + 4 k` / `+0x274 + 8 k`, the states
    /// `+0x7ac..+0x7ec` and `+0x7f0..+0x800`, `+0x7ec`, the buffers
    /// `+0x984..+0x994` and last `+0x7a0`.
    pub fn slots() -> impl Iterator<Item = u32> {
        let pairs = (0..3).flat_map(|k| [0x260 + 4 * k, 0x274 + 8 * k]);
        (0xc..=0x94)
            .step_by(8)
            .chain((0x9c..=0xfc).step_by(8))
            .chain((0x104..=0x1fc).step_by(8))
            .chain((0x204..=0x24c).step_by(8))
            .chain([0x254, 0x25c])
            .chain(pairs)
            .chain((0x7ac..0x7ec).step_by(4))
            .chain((0x7f0..0x800).step_by(4))
            .chain([0x7ec, 0x984, 0x988, 0x98c, 0x990, 0x994, 0x7a0])
    }

    /// `GDraw_releaseObjects_00a35c90()` (`0x00a35c90`; its one caller,
    /// `gdraw_destroyContext`, reads no result): each slot of [`slots`]
    /// holding an object is released (its method `+8`, stdcall with the
    /// object) and cleared. `g_GDrawState` is read again after each
    /// release; the slot cleared is the one at the old base.
    pub fn release_objects(h: &mut impl Host) {
        let at = h.rebase(GDRAW);
        let mut g: u32 = h.read(at);
        for off in slots() {
            let slot = g.wrapping_add(off);
            let obj: u32 = h.read(slot);
            if obj != 0 {
                let vt: u32 = h.read(obj);
                let release: u32 = h.read(vt.wrapping_add(8));
                h.call(release, Conv::Stdcall, &[obj]);
                h.write(slot, 0_u32);
                g = h.read(at);
            }
        }
    }

    /// Iggy's `IggyGDrawMallocAnnotated(size, file, line)` and
    /// `IggyGDrawFree(p)` (import slots, stdcall), the CRT's `memset`
    /// thunk; the source file name the allocations carry.
    pub const MALLOC_IAT: u32 = 0x00b0_c86c;
    pub const FREE_IAT: u32 = 0x00b0_c870;
    pub const MEMSET: u32 = 0x0094_964c;
    pub const SOURCE_NAME: u32 = 0x00b7_a5e8;
    /// `op_node_pool_init_00a39bc0(set, n, v)`,
    /// `State_InitBuffers_00a34b10(n)` (-> `al`),
    /// `GDraw_createDeviceObjects_00a34cc0()`.
    pub const NODE_POOL_INIT: u32 = 0x00a3_9bc0;
    pub const INIT_BUFFERS: u32 = 0x00a3_4b10;
    pub const CREATE_DEVICE_OBJECTS: u32 = 0x00a3_4cc0;
    /// The limits the pools and buffers are sized from: the two pools'
    /// counts and values (`0x00c6d450`/`54`, `58`/`5c`), the state's own
    /// pool (`48`/`4c`), the buffer size (`64`).
    pub const POOL_A: u32 = 0x00c6_d450;
    pub const POOL_B: u32 = 0x00c6_d458;
    pub const POOL_STATE: u32 = 0x00c6_d448;
    pub const BUFFER_SIZE: u32 = 0x00c6_d464;
    /// The function table `GDraw` hands Iggy (`0x01a85828`) and the
    /// originals its slots get, by slot offset, in the original's order.
    pub const FUNCS: u32 = 0x01a8_5828;
    pub const FUNC_SLOTS: [(u32, u32); 33] = [
        (0x04, 0x00a3_abe0),
        (0x00, 0x00a3_9900),
        (0x64, 0x00a3_8480),
        (0x68, 0x00a3_84b0),
        (0x78, 0x00a3_a990),
        (0x7c, 0x00a3_a9a0),
        (0x08, 0x00a3_a820),
        (0x0c, 0x00a3_a980),
        (0x5c, 0x00a3_ac90),
        (0x60, 0x00a3_afb0),
        (0x20, 0x00a3_87e0),
        (0x1c, 0x00a3_9200),
        (0x10, 0x00a3_a9f0),
        (0x14, 0x00a3_7410),
        (0x18, 0x00a3_73c0),
        (0x24, 0x00a3_a070),
        (0x28, 0x00a3_a530),
        (0x2c, 0x00a3_a200),
        (0x50, 0x00a3_b0c0),
        (0x54, 0x00a3_b160),
        (0x58, 0x00a3_b100),
        (0x48, 0x00a3_98a0),
        (0x3c, 0x00a3_b050),
        (0x80, 0x00a3_7c20),
        (0x84, 0x00a3_7ac0),
        (0x30, 0x00a3_a540),
        (0x34, 0x00a3_a7a0),
        (0x38, 0x00a3_a5f0),
        (0x40, 0x00a3_b010),
        (0x4c, 0x00a3_98e0),
        (0x44, 0x00a3_b090),
        (0x6c, 0x00a3_abc0),
        (0x88, 0x00a3_a9b0),
    ];

    /// A pool of `n` nodes (`0x38` bytes each after a `0x1a8`-byte head)
    /// from `IggyGDrawMallocAnnotated`, set up by `op_node_pool_init`
    /// with `v`, its `+0xc` bit 0 cleared or set (`second`); 0 when the
    /// allocation fails.
    fn pool(h: &mut impl Host, malloc: u32, limits: u32, second: bool) -> u32 {
        let n: u32 = h.read(h.rebase(limits));
        let v: u32 = h.read(h.rebase(limits).wrapping_add(4));
        let name = h.rebase(SOURCE_NAME);
        let p = h.call(
            malloc,
            Conv::Stdcall,
            &[n.wrapping_mul(0x38).wrapping_add(0x1a8), name, 0x868],
        );
        if p != 0 {
            h.call_sym(NODE_POOL_INIT, Conv::Cdecl, &[p, n, v]);
            let flags: u32 = h.read(p.wrapping_add(0xc));
            h.write(
                p.wrapping_add(0xc),
                if second { flags | 1 } else { flags & !1 },
            );
        }
        p
    }

    /// `gdraw_createContext_00a35780(device, context, w, h)` (`0x00a35780`)
    /// -> the `GDraw` function table (`0x01a85828`) or 0: the state block
    /// (`0xad0` bytes from `IggyGDrawMallocAnnotated`, zeroed) into
    /// `g_GDrawState` with the device `+0`, the context `+4` and the
    /// sizes `+0x6c0`/`+0x6c4`; its two node pools `+0xab0`/`+0xab4`
    /// ([`pool`]) and its own pool at `+0x290`; with both pools and
    /// `State_InitBuffers` succeeding, `GDraw_createDeviceObjects` and the
    /// function table filled ([`FUNC_SLOTS`]). Otherwise
    /// the pools and the block are freed and `g_GDrawState` cleared
    /// (`g_GDrawState` read again after each free).
    #[allow(clippy::many_single_char_names)]
    pub fn create_context(h: &mut impl Host, device: u32, context: u32, w: u32, hh: u32) -> u32 {
        let at = h.rebase(GDRAW);
        let malloc: u32 = h.read(h.rebase(MALLOC_IAT));
        let name = h.rebase(SOURCE_NAME);
        let g = h.call(malloc, Conv::Stdcall, &[0xad0, name, 0x8d9]);
        h.write(at, g);
        if g == 0 {
            return 0;
        }
        h.call_sym(MEMSET, Conv::Cdecl, &[g, 0, 0xad0]);
        h.write(g.wrapping_add(0x6c0), w);
        h.write(g.wrapping_add(0x6c4), hh);
        h.write(g, device);
        h.write(g.wrapping_add(4), context);
        let a = pool(h, malloc, POOL_A, false);
        let g: u32 = h.read(at);
        h.write(g.wrapping_add(0xab0), a);
        let b = pool(h, malloc, POOL_B, true);
        let mut g: u32 = h.read(at);
        h.write(g.wrapping_add(0xab4), b);
        let n: u32 = h.read(h.rebase(POOL_STATE));
        let v: u32 = h.read(h.rebase(POOL_STATE).wrapping_add(4));
        h.call_sym(NODE_POOL_INIT, Conv::Cdecl, &[g.wrapping_add(0x290), n, v]);
        if h.read::<u32>(g.wrapping_add(0xab0)) != 0 && h.read::<u32>(g.wrapping_add(0xab4)) != 0 {
            let size: u32 = h.read(h.rebase(BUFFER_SIZE));
            if h.call_sym(INIT_BUFFERS, Conv::Cdecl, &[size]) & 0xff != 0 {
                h.call_sym(CREATE_DEVICE_OBJECTS, Conv::Cdecl, &[]);
                let funcs = h.rebase(FUNCS);
                for (off, f) in FUNC_SLOTS {
                    let f = h.rebase(f);
                    h.write(funcs.wrapping_add(off), f);
                }
                return funcs;
            }
            g = h.read(at);
        }
        if g != 0 {
            let free: u32 = h.read(h.rebase(FREE_IAT));
            for off in [0xab0, 0xab4] {
                let p: u32 = h.read(g.wrapping_add(off));
                if p != 0 {
                    h.call(free, Conv::Stdcall, &[p]);
                    g = h.read(at);
                }
            }
            h.call(free, Conv::Stdcall, &[g]);
            h.write(at, 0_u32);
        }
        0
    }

    /// `IggyGDrawSendWarning(0, format, ...)` (import slot, cdecl varargs),
    /// `gdraw_wrapTexture_00a383a0(view)`, the warnings' formats and the
    /// two calls' names (and the empty suffix).
    pub const WARNING_IAT: u32 = 0x00b0_c874;
    pub const WRAP_TEXTURE: u32 = 0x00a3_83a0;
    pub const RAW_FORMAT_WARNING: u32 = 0x00b7_a8c8;
    pub const OUT_OF_MEMORY_WARNING: u32 = 0x00b7_a4ec;
    pub const ERROR_WARNING: u32 = 0x00b7_a50c;
    pub const CREATE_TEXTURE_NAME: u32 = 0x00b7_a688;
    pub const CREATE_VIEW_NAME: u32 = 0x00b7_a908;
    pub const EMPTY: u32 = 0x00b8_fbeb;
    /// `E_OUTOFMEMORY`.
    const OUT_OF_MEMORY: u32 = 0x8007_000e;
    /// `GDraw_createRawTexture_00a37c20`'s frame: the
    /// `D3D11_SUBRESOURCE_DATA` array at `ebp - 0x124` (24 of 12 bytes,
    /// the first word cleared, the rest by `memset`), the
    /// `D3D11_TEXTURE2D_DESC` at `ebp - 0x150`, the outs.
    pub const RAW_FRAME: u32 = 0x178;

    /// A failed device call's warning: out of memory or the error code,
    /// with the call's name.
    fn d3d_warning(h: &mut impl Host, name: u32, hr: u32) {
        let warn: u32 = h.read(h.rebase(WARNING_IAT));
        let (name, empty) = (h.rebase(name), h.rebase(EMPTY));
        if hr == OUT_OF_MEMORY {
            let f = h.rebase(OUT_OF_MEMORY_WARNING);
            h.call(warn, Conv::Cdecl, &[0, f, name, empty]);
        } else {
            let f = h.rebase(ERROR_WARNING);
            h.call(warn, Conv::Cdecl, &[0, f, name, empty, hr]);
        }
    }

    /// `GDraw_createRawTexture_00a37c20(base, _, info)` (`0x00a37c20`,
    /// `ret 0xc`; slot `+0x80` of `GDraw`'s function table) -> the
    /// texture handle or 0: an immutable shader-resource texture of
    /// `info`'s format (`+4`: 0 RGBA8 (0x1c, 4 bytes a texel), 9/10/11
    /// BC1/BC2/BC3 (0x47/0x4a/0x4d, 8/16/16 bytes a 4x4 block), else a
    /// warning and 0), size `u16` `+6` x `+8`, `+5` mip levels, its data
    /// at `base + [info]`, one level after the other (each level's size
    /// halved, at least 1, in blocks rounded up). `CreateTexture2D` and
    /// `CreateShaderResourceView` on the device (`[[g_GDrawState]]`,
    /// its methods `+0x14`/`+0x1c`), the view wrapped
    /// (`gdraw_wrapTexture`), whose first word gets the texture; on a
    /// failure (warned) both objects are released.
    ///
    /// The original also widens formats 5 and 6 into a temporary copy
    /// (`op_mip_bytes_00a3c0f0`, `IggyGDrawMallocAnnotated`), but its
    /// switch sends those formats to the warning first: that code never
    /// runs, and the copy it would free is always null.
    #[allow(clippy::too_many_lines)]
    pub fn create_raw_texture(h: &mut impl Host, ebp: u32, base: u32, info: u32) -> u32 {
        let fr = |o: u32| ebp.wrapping_sub(o);
        h.write(fr(0x16c), 0_u32);
        h.write(fr(0x160), 0_u32);
        h.write(fr(0x168), 0_u32);
        h.write(fr(0x124), 0_u32);
        h.call_sym(MEMSET, Conv::Cdecl, &[fr(0x120), 0, 0x11c]);
        let format = h.read::<u8>(info.wrapping_add(4));
        let w = u32::from(h.read::<u16>(info.wrapping_add(6)));
        let ht = u32::from(h.read::<u16>(info.wrapping_add(8)));
        let mips = u32::from(h.read::<u8>(info.wrapping_add(5)));
        h.write(fr(0x164), w);
        h.write(fr(0x170), ht);
        h.write(fr(0x154), mips);
        h.write(fr(0x158), 1_u32);
        for (o, v) in [
            (0x150, w),
            (0x14c, ht),
            (0x148, mips),
            (0x144, 1),
            (0x140, 0),
            (0x13c, 1),
            (0x138, 0),
            (0x134, 1),
            (0x130, 8),
            (0x12c, 0),
            (0x128, 0),
        ] {
            h.write(fr(o), v);
        }
        let (bpb, dxgi, block) = match format {
            0 => (4_u32, 0x1c_u32, 1_u32),
            9 => (8, 0x47, 4),
            10 => (0x10, 0x4a, 4),
            11 => (0x10, 0x4d, 4),
            _ => {
                let warn: u32 = h.read(h.rebase(WARNING_IAT));
                let f = h.rebase(RAW_FORMAT_WARNING);
                h.call(warn, Conv::Cdecl, &[0, f, u32::from(format)]);
                // nothing was made: the two outs are still null
                return 0;
            }
        };
        h.write(fr(0x15c), bpb);
        h.write(fr(0x158), block);
        h.write(fr(0x140), dxgi);
        let mut data = h.read::<u32>(info).wrapping_add(base);
        let (block, bpb) = (block.cast_signed(), bpb.cast_signed());
        // sar by cl: the count taken mod 32
        let level = |v: u32, k: u32| v.cast_signed().wrapping_shr(k).max(1);
        for k in 0..mips {
            let bw = (level(w, k) - 1 + block) / block;
            let entry = fr(0x124).wrapping_add(12 * k);
            h.write(entry.wrapping_add(4), bw.wrapping_mul(bpb).cast_unsigned());
            h.write(entry, data);
            let bh = (block - 1 + level(ht, k)) / block;
            data = data.wrapping_add(bh.wrapping_mul(bw).wrapping_mul(bpb).cast_unsigned());
        }
        let state: u32 = h.read(h.rebase(GDRAW));
        let device: u32 = h.read(state);
        let vt: u32 = h.read(device);
        let create = h.read(vt.wrapping_add(0x14));
        let hr = h.call(
            create,
            Conv::Stdcall,
            &[device, fr(0x150), fr(0x124), fr(0x160)],
        );
        let handle = if hr.cast_signed() < 0 {
            d3d_warning(h, CREATE_TEXTURE_NAME, hr);
            0
        } else {
            let state: u32 = h.read(h.rebase(GDRAW));
            let device: u32 = h.read(state);
            let vt: u32 = h.read(device);
            let view = h.read(vt.wrapping_add(0x1c));
            let texture: u32 = h.read(fr(0x160));
            let hr = h.call(view, Conv::Stdcall, &[device, texture, 0, fr(0x168)]);
            if hr.cast_signed() < 0 {
                d3d_warning(h, CREATE_VIEW_NAME, hr);
                0
            } else {
                let srv: u32 = h.read(fr(0x168));
                h.call_sym(WRAP_TEXTURE, Conv::Cdecl, &[srv])
            }
        };
        if handle != 0 {
            let texture: u32 = h.read(fr(0x160));
            h.write(handle, texture);
            return handle;
        }
        for o in [0x168, 0x160] {
            let obj: u32 = h.read(fr(o));
            if obj != 0 {
                let vt: u32 = h.read(obj);
                let release: u32 = h.read(vt.wrapping_add(8));
                h.call(release, Conv::Stdcall, &[obj]);
            }
        }
        0
    }

    /// `SetGlobals_0042e1d0(a, b, c, d)` and the four allocator pointers
    /// it gets (`0x00c6b28c..98`); libvorbis' `vorbis_info_init`,
    /// `vorbis_comment_init`, `vorbis_synthesis_headerin`,
    /// `vorbis_synthesis_init`, `vorbis_block_init`; mkvparser's
    /// `Track::GetCodecPrivate(&size)` (thiscall, `ret 4`),
    /// `AudioTrack::GetSamplingRate()` (`ST0`, a double),
    /// `AudioTrack::GetChannels()`, `Segment::GetDuration()` (`edx:eax`);
    /// `CDataFrameQueue::Init(capacity, extra, alloc, free)` (`ret 0x10`)
    /// and `::Destroy()`; the queue's allocator pointers
    /// (`0x00c6b29c`/`a0`); FMOD's `System::createSound`,
    /// `Sound::setUserData`, `System::playSound`, `Channel::setPriority`
    /// (import thunks, stdcall with the object first); the stream's
    /// `pcmreadcallback` (`0x00a27ee0`).
    pub const SET_GLOBALS: u32 = 0x0042_e1d0;
    pub const ALLOCATORS: u32 = 0x00c6_b28c;
    pub const VORBIS_INFO_INIT: u32 = 0x00a8_ffd0;
    pub const VORBIS_COMMENT_INIT: u32 = 0x00a9_0420;
    pub const VORBIS_HEADERIN: u32 = 0x00a8_f650;
    pub const VORBIS_SYNTHESIS_INIT: u32 = 0x00a9_10e0;
    pub const VORBIS_BLOCK_INIT: u32 = 0x00a9_21b0;
    pub const GET_CODEC_PRIVATE: u32 = 0x0042_7b80;
    pub const GET_SAMPLING_RATE: u32 = 0x0042_9170;
    pub const GET_CHANNELS: u32 = 0x0042_9180;
    pub const GET_DURATION: u32 = 0x0042_5200;
    pub const QUEUE_INIT: u32 = 0x00a3_ddf0;
    pub const QUEUE_DESTROY: u32 = 0x00a3_df20;
    pub const QUEUE_ALLOCATORS: u32 = 0x00c6_b29c;
    pub const CREATE_SOUND: u32 = 0x0094_8cee;
    pub const SET_USER_DATA: u32 = 0x00a7_c8f8;
    pub const PLAY_SOUND: u32 = 0x0094_8cf4;
    pub const SET_PRIORITY: u32 = 0x00a7_c910;
    pub const PCM_READ: u32 = 0x00a2_7ee0;
    /// `1e-9` (nanoseconds to seconds) and `2^32` (the unsigned
    /// conversions' correction), doubles.
    pub const NANO: u32 = 0x00b9_2178;
    pub const TWO_32: u32 = 0x00b0_dbb0;
    /// `PVideoPlaybackWin32_AudioInit`'s frame: the
    /// `FMOD_CREATESOUNDEXINFO` (0x88 bytes) at `ebp - 0x9c`, the codec
    /// data's size at `ebp - 0x14`.
    pub const AUDIO_FRAME: u32 = 0x9c;

    fn k64(h: &impl Host, sym: u32) -> f64 {
        f64::from_bits(h.read(h.rebase(sym)))
    }

    /// `fild dword` of an unsigned value: `2^32` added when its top bit
    /// is set.
    fn unsigned(h: &impl Host, v: u32) -> f64 {
        let x = f64::from(v.cast_signed());
        if v.cast_signed() < 0 {
            x + k64(h, TWO_32)
        } else {
            x
        }
    }

    /// `PVideoPlaybackWin32_AudioInit_00a273b0(player)` (`0x00a273b0`,
    /// fastcall: `player` in `ecx`) -> `al` 1 when the FMOD stream plays,
    /// else 0 (the rest of `eax` the last call's): the Vorbis decoder
    /// set up from the track's codec data (`+0x294`; its three headers
    /// through `vorbis_synthesis_headerin` on the packet at `+0x160`: the
    /// data past its 3-byte lacing, the first header's bytes, then the
    /// second's and 4 more, as the original steps them), the synthesis
    /// and block states (`+0x1b0`, `+0x220`); the frame queue `+0x80`
    /// (1 MB, `channels << 13` extra, `+0x2a4` 0x1000); an FMOD stream
    /// (`System` `+0x298`, mode `0x4c8`) of PCM16 at the track's rate
    /// (`fistp`, truncated) and channels, its length
    /// `duration(ns) * 1e-9 * rate * 2 channels` (`fistp`, truncated,
    /// the unsigned words through `2^32`), decoding through
    /// `0x00a27ee0`; the sound `+0x29c` gets the player as user data
    /// and plays on channel `+0x2a0`, priority 0. A failed
    /// `createSound` destroys the queue.
    #[allow(clippy::too_many_lines)]
    #[allow(clippy::many_single_char_names)]
    pub fn audio_init(h: &mut impl Host, ebp: u32, this: u32) -> u32 {
        let fr = |o: u32| ebp.wrapping_sub(o);
        let at = |o: u32| this.wrapping_add(o);
        let alloc = h.rebase(ALLOCATORS);
        let g = [0, 4, 8, 0xc].map(|o| h.read::<u32>(alloc.wrapping_add(o)));
        h.call_sym(SET_GLOBALS, Conv::Cdecl, &g);
        let (info, comment) = (at(0x180), at(0x1a0));
        h.call_sym(VORBIS_INFO_INIT, Conv::Cdecl, &[info]);
        h.call_sym(VORBIS_COMMENT_INIT, Conv::Cdecl, &[comment]);
        let track: u32 = h.read(at(0x294));
        h.write(fr(0x14), 0_u32);
        let data = h.call_sym(GET_CODEC_PRIVATE, Conv::Thiscall(track), &[fr(0x14)]);
        let size: u32 = h.read(fr(0x14));
        for (o, v) in [
            (0x168, 1_u32),
            (0x16c, 0),
            (0x178, 0),
            (0x17c, 0),
            (0x170, 0),
            (0x174, 0),
        ] {
            h.write(at(o), v);
        }
        h.write(at(0x164), size);
        let op = at(0x160);
        h.write(op, data.wrapping_add(3));
        let r = h.call_sym(VORBIS_HEADERIN, Conv::Cdecl, &[info, comment, op]);
        let v: u32 = h.read(op);
        h.write(op, v.wrapping_add(r));
        let v: u32 = h.read(at(0x164));
        h.write(at(0x164), v.wrapping_sub(r));
        let r = h
            .call_sym(VORBIS_HEADERIN, Conv::Cdecl, &[info, comment, op])
            .wrapping_add(4);
        let v: u32 = h.read(at(0x164));
        h.write(at(0x164), v.wrapping_sub(r));
        let v: u32 = h.read(op);
        h.write(op, v.wrapping_add(r));
        h.call_sym(VORBIS_HEADERIN, Conv::Cdecl, &[info, comment, op]);
        // al = 0 from here on unless the stream plays
        let fail = |eax: u32| eax & !0xff;
        let dsp = at(0x1b0);
        let r = h.call_sym(VORBIS_SYNTHESIS_INIT, Conv::Cdecl, &[dsp, info]);
        if r != 0 {
            return fail(r);
        }
        let r = h.call_sym(VORBIS_BLOCK_INIT, Conv::Cdecl, &[dsp, at(0x220)]);
        if r != 0 {
            return fail(r);
        }
        let track: u32 = h.read(at(0x294));
        let rate = h.call_sym_st0_f64(GET_SAMPLING_RATE, Conv::Thiscall(track), &[]);
        // fistp qword with the rounding set to truncate; the low word
        let rate = x87::ftol2(rate);
        let track: u32 = h.read(at(0x294));
        let channels = h.call_sym(GET_CHANNELS, Conv::Thiscall(track), &[]);
        let qa = h.rebase(QUEUE_ALLOCATORS);
        let (qa0, qa1): (u32, u32) = (h.read(qa), h.read(qa.wrapping_add(4)));
        h.write(at(0x2a4), 0x1000_u32);
        h.call_sym(
            QUEUE_INIT,
            Conv::Thiscall(at(0x80)),
            &[0x10_0000, channels << 13, qa0, qa1],
        );
        let ex = fr(0x9c);
        h.call_sym(MEMSET, Conv::Cdecl, &[ex, 0, 0x88]);
        h.write(ex, 0x88_u32);
        h.write(fr(0x84), 0x1000_u32);
        let segment: u32 = h.read(at(0x110));
        let ns = h.call_sym_u64(GET_DURATION, Conv::Thiscall(segment), &[]);
        // fild qword, then the double through the frame
        #[allow(clippy::cast_precision_loss)]
        let mut len = ns.cast_signed() as f64;
        len = x87::mul(len, k64(h, NANO));
        len = x87::mul(len, unsigned(h, rate));
        len = x87::mul(len, unsigned(h, channels.wrapping_mul(2)));
        h.write(fr(0x8c), rate);
        h.write(fr(0x90), channels);
        h.write(fr(0x88), 2_u32);
        let pcm = h.rebase(PCM_READ);
        h.write(fr(0x70), pcm);
        h.write(fr(0x98), x87::ftol2(len));
        let system: u32 = h.read(at(0x298));
        let sound = at(0x29c);
        let r = h.call_sym(CREATE_SOUND, Conv::Stdcall, &[system, 0, 0x4c8, ex, sound]);
        if r != 0 {
            let r = h.call_sym(QUEUE_DESTROY, Conv::Thiscall(at(0x80)), &[]);
            return fail(r);
        }
        let s: u32 = h.read(sound);
        h.call_sym(SET_USER_DATA, Conv::Stdcall, &[s, this]);
        let s: u32 = h.read(sound);
        let system: u32 = h.read(at(0x298));
        let r = h.call_sym(
            PLAY_SOUND,
            Conv::Stdcall,
            &[system, 0xffff_ffff, s, 0, at(0x2a0)],
        );
        if r != 0 {
            return fail(r);
        }
        let channel: u32 = h.read(at(0x2a0));
        let r = h.call_sym(SET_PRIORITY, Conv::Stdcall, &[channel, 0]);
        (r & !0xff) | 1
    }

    /// The debug GUI calls `Gui_NameinputProc` makes (cdecl):
    /// `Gu_WindowGetArg(w)`, `Gu_WindowSetArg(w, p)`,
    /// `Gu_WindowSetTitle(w, s)`, `Gu_WindowAddButton(w, id, s, x, y, cx,
    /// cy)`, `Gu_WindowControlSetText(w, id, s)`, `set_path_name(w, p, id,
    /// n)`, `FUN_00942030(dst, src)` (a string copy), `Gu_PrintSetX(x)`,
    /// `Gu_PrintSetY(y)`, `Gu_Printf(s, ...)`; `user_malloc(n)`,
    /// `user_free_thunk(p)`, `movie_err(expr, file, line)`,
    /// `dbgPrintf(fmt, ...)`, `sceOpen(path, mode)`, `sceClose(fd)`.
    pub const WINDOW_GET_ARG: u32 = 0x0084_aec0;
    pub const WINDOW_SET_ARG: u32 = 0x0084_b560;
    pub const WINDOW_SET_TITLE: u32 = 0x0084_b6a0;
    pub const WINDOW_ADD_BUTTON: u32 = 0x0084_a470;
    pub const CONTROL_SET_TEXT: u32 = 0x0094_11f0;
    pub const SET_PATH_NAME: u32 = 0x0094_2360;
    pub const STRING_COPY: u32 = 0x0094_2030;
    pub const PRINT_SET_X: u32 = 0x0084_8820;
    pub const PRINT_SET_Y: u32 = 0x0084_8860;
    pub const PRINTF: u32 = 0x0084_8880;
    pub const USER_MALLOC: u32 = 0x0068_7020;
    pub const USER_FREE: u32 = 0x0068_7010;
    pub const MOVIE_ERR: u32 = 0x0076_efa0;
    pub const DBG_PRINTF: u32 = 0x0062_f500;
    pub const SCE_OPEN: u32 = 0x0062_fa30;
    pub const SCE_CLOSE: u32 = 0x0062_f610;
    /// The name window's data: its key table (`0x00c612c0`, string
    /// pointers, 0xa a new row, 0 the end), the default name
    /// (`0x00c6137c`), the empty list text (`0x00c61384`), the camera
    /// directory (`0x00c61238`), the browsed path (`0x025d5fa0`, its
    /// length `0x025d5f94`, its tail past "host:..." `0x025d5fb1`), the
    /// selected file name and its flag (`0x025d60a0`, `0x025d5f9c`); the
    /// strings it prints and its buttons' texts.
    pub const NAME_KEYS: u32 = 0x00c6_12c0;
    pub const NAME_DEFAULT: u32 = 0x00c6_137c;
    pub const NAME_EMPTY: u32 = 0x00c6_1384;
    pub const NAME_CAMERA_DIR: u32 = 0x00c6_1238;
    pub const NAME_PATH: u32 = 0x025d_5fa0;
    pub const NAME_PATH_LEN: u32 = 0x025d_5f94;
    pub const NAME_PATH_TAIL: u32 = 0x025d_5fb1;
    pub const NAME_FILE: u32 = 0x025d_60a0;
    pub const NAME_FILE_SET: u32 = 0x025d_5f9c;
    pub const NAME_TITLE: u32 = 0x00b6_902c;
    pub const NAME_ERR_EXPR: u32 = 0x00b6_8f34;
    pub const NAME_ERR_FILE: u32 = 0x00b6_8fac;
    pub const NAME_OK: u32 = 0x00b5_1c04;
    pub const NAME_UP: u32 = 0x00b6_7ef4;
    pub const NAME_DOWN: u32 = 0x00b5_b24c;
    pub const NAME_FREE_MSG: u32 = 0x00b6_9038;
    pub const NAME_OK_MSG: u32 = 0x00b6_9050;
    pub const NAME_HOST_FMT: u32 = 0x00b6_8f58;
    pub const NAME_PICK_MSG: u32 = 0x00b6_8f4c;
    pub const NAME_NAME2_FMT: u32 = 0x00b6_9054;
    pub const NAME_NAME_FMT: u32 = 0x00b6_8f74;
    pub const NAME_BLANK: u32 = 0x00b2_0b30;
    pub const NAME_CURSOR: u32 = 0x00b5_5e74;
    /// `Gui_NameinputProc`'s frame: the `sprintf` buffer at `ebp - 0x204`.
    pub const NAME_FRAME: u32 = 0x204;

    /// The CRT's `sprintf` (import slot).
    pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;

    /// The `i32` at `p + o`.
    fn wi(h: &impl Host, p: u32, o: u32) -> i32 {
        h.read::<u32>(p.wrapping_add(o)).cast_signed()
    }

    /// The byte at `p + o` (a signed offset).
    fn bi(h: &impl Host, p: u32, o: i32) -> u8 {
        h.read(p.wrapping_add_signed(o))
    }

    fn c(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    fn sprintf(h: &mut impl Host, args: &[u32]) {
        let f: u32 = h.read(h.rebase(SPRINTF_SLOT));
        h.call(f, Conv::Cdecl, args);
    }

    /// The six list rows from the scroll position `+0x21c`: the names
    /// (`[+0x214] + 0x200 k`, `+0x218` of them), then blanks.
    fn name_rows(h: &mut impl Host, window: u32, p: u32) {
        let first = h.read::<u32>(p.wrapping_add(0x21c)).cast_signed();
        let mut row = 0_u32;
        let mut idx = first;
        let mut off = first.cast_unsigned() << 9;
        loop {
            if idx >= h.read::<u32>(p.wrapping_add(0x218)).cast_signed() {
                break;
            }
            let names: u32 = h.read(p.wrapping_add(0x214));
            c(
                h,
                CONTROL_SET_TEXT,
                &[window, row + 0x103, names.wrapping_add(off)],
            );
            row += 1;
            idx = idx.wrapping_add(1);
            off = off.wrapping_add(0x200);
            if row >= 6 {
                return;
            }
        }
        let blank = h.rebase(NAME_BLANK);
        while row < 6 {
            c(h, CONTROL_SET_TEXT, &[window, row + 0x103, blank]);
            row += 1;
        }
    }

    /// `Gui_NameinputProc(window, msg, param)` (`0x0093e570`) -> 0: the
    /// debug camera's "NAME EDIT" window (its data from
    /// `Gu_WindowGetArg`, `p`). 0x8001 builds it (a `0x224`-byte block:
    /// the name at `+0`, the cursor `+0x200`, the list `+0x214` /
    /// `+0x218` / `+0x21c`, the text row `+0x220`; the key buttons from
    /// the key table, OK, up/down and six list rows); 0x8003 frees it;
    /// 0x8011 prints the name and the cursor; 0x8006: OK (0x100) takes
    /// `host:` + the camera directory + the name as the file, up/down
    /// (0x101/0x102) scroll the list, a row (0x103..0x108) opens the
    /// file or, when it can't, enters (or for a leading '.' leaves) that
    /// directory, then shows `name:` + the path (and the file); a key
    /// (its index + 1) types its character, 0x2b/0x2c delete before/at
    /// the cursor, 0x2d/0x2e move it (within 0..0x40).
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn nameinput_proc(h: &mut impl Host, ebp: u32, window: u32, msg: u32, param: u32) -> u32 {
        let buf = ebp.wrapping_sub(NAME_FRAME);
        let mut p = c(h, WINDOW_GET_ARG, &[window]);
        let at = |p: u32, o: u32| p.wrapping_add(o);
        match msg {
            0x8001 => {
                p = c(h, USER_MALLOC, &[0x224]);
                if p == 0 {
                    let (e, f) = (h.rebase(NAME_ERR_EXPR), h.rebase(NAME_ERR_FILE));
                    c(h, MOVIE_ERR, &[e, f, 0x1d4]);
                }
                c(h, MEMSET, &[p, 0, 0x224]);
                c(h, WINDOW_SET_ARG, &[window, p]);
                let title = h.rebase(NAME_TITLE);
                c(h, WINDOW_SET_TITLE, &[window, title]);
                let (mut x, mut y, mut id) = (4_u32, 0_u32, 1_u32);
                let mut key = h.rebase(NAME_KEYS);
                loop {
                    let s: u32 = h.read(key);
                    if s == 0xa {
                        y = y.wrapping_add(0xc);
                        x = 4;
                    } else if s == 0 {
                        break;
                    } else {
                        let mut len = 0_u32;
                        while h.read::<u8>(s.wrapping_add(len)) != 0 {
                            len = len.wrapping_add(1);
                        }
                        c(
                            h,
                            WINDOW_ADD_BUTTON,
                            &[window, id, s, x, y, len.wrapping_mul(14), 10],
                        );
                        x = x.wrapping_add(len << 4);
                    }
                    key = key.wrapping_add(4);
                    id = id.wrapping_add(1);
                }
                let ok = h.rebase(NAME_OK);
                c(
                    h,
                    WINDOW_ADD_BUTTON,
                    &[window, 0x100, ok, x.wrapping_add(0x3a), y, 0x18, 10],
                );
                // strcpy of the default name
                let def = h.rebase(NAME_DEFAULT);
                let mut k = 0_u32;
                loop {
                    let b: u8 = h.read(def.wrapping_add(k));
                    h.write(p.wrapping_add(k), b);
                    if b == 0 {
                        break;
                    }
                    k = k.wrapping_add(1);
                }
                h.write(at(p, 0x220), y.wrapping_add(0xe));
                h.write(at(p, 0x200), 0_u32);
                let y = y.wrapping_add(0x18);
                let (up, down) = (h.rebase(NAME_UP), h.rebase(NAME_DOWN));
                c(
                    h,
                    WINDOW_ADD_BUTTON,
                    &[window, 0x101, up, 0x8c, y.wrapping_add(8), 0xe, 0xc],
                );
                c(
                    h,
                    WINDOW_ADD_BUTTON,
                    &[window, 0x102, down, 0x8c, y.wrapping_add(0x1c), 0xe, 0xc],
                );
                let empty = h.rebase(NAME_EMPTY);
                for k in 0..6 {
                    c(
                        h,
                        WINDOW_ADD_BUTTON,
                        &[window, 0x103 + k, empty, 8, y.wrapping_add(9 * k), 0x78, 8],
                    );
                }
                c(h, SET_PATH_NAME, &[window, p, 0x108, 0x11]);
            }
            0x8003 => {
                let m = h.rebase(NAME_FREE_MSG);
                c(h, DBG_PRINTF, &[m]);
                let list: u32 = h.read(at(p, 0x214));
                if list != 0 {
                    c(h, USER_FREE, &[list]);
                }
                c(h, USER_FREE, &[p]);
            }
            0x8011 => {
                c(h, PRINT_SET_X, &[2]);
                let row: u32 = h.read(at(p, 0x220));
                c(h, PRINT_SET_Y, &[row]);
                c(h, PRINTF, &[p]);
                let cur: u32 = h.read(at(p, 0x200));
                c(h, PRINT_SET_X, &[cur.wrapping_mul(9).wrapping_add(0x2f)]);
                let row: u32 = h.read(at(p, 0x220));
                c(h, PRINT_SET_Y, &[row.wrapping_add(2)]);
                let cursor = h.rebase(NAME_CURSOR);
                c(h, PRINTF, &[cursor]);
            }
            0x8006 => name_command(h, buf, window, p, param),
            _ => {}
        }
        0
    }

    /// `Gui_NameinputProc`'s 0x8006 (a button) with the window's data
    /// `p` and the frame's buffer `buf`.
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    fn name_command(h: &mut impl Host, buf: u32, window: u32, p: u32, param: u32) {
        let at = |o: u32| p.wrapping_add(o);
        match param.wrapping_sub(0x100) {
            0 => {
                let m = h.rebase(NAME_OK_MSG);
                c(h, DBG_PRINTF, &[m]);
                let (f, dir) = (h.rebase(NAME_HOST_FMT), h.rebase(NAME_CAMERA_DIR));
                sprintf(h, &[buf, f, dir, at(5)]);
                let file = h.rebase(NAME_FILE);
                c(h, STRING_COPY, &[file, buf]);
                let set = h.rebase(NAME_FILE_SET);
                h.write(set, 1_u32);
            }
            1 => {
                let first = wi(&*h, p, 0x21c);
                if first > 0 {
                    h.write(at(0x21c), first - 1);
                }
                name_rows(h, window, p);
            }
            2 => {
                let first = wi(&*h, p, 0x21c);
                if first.wrapping_add(6) < wi(&*h, p, 0x218) {
                    h.write(at(0x21c), first.wrapping_add(1));
                }
                name_rows(h, window, p);
            }
            3..=8 => {
                let k = param.wrapping_sub(0x103).wrapping_add(h.read(at(0x21c)));
                if k.cast_signed() >= wi(&*h, p, 0x218) {
                    return;
                }
                let names: u32 = h.read(at(0x214));
                let name = (k << 9).wrapping_add(names);
                if name == 0 {
                    return;
                }
                let m = h.rebase(NAME_PICK_MSG);
                c(h, DBG_PRINTF, &[m, name]);
                let (f, path) = (h.rebase(NAME_HOST_FMT), h.rebase(NAME_PATH));
                sprintf(h, &[buf, f, path, name]);
                let fd = c(h, SCE_OPEN, &[buf, 1]);
                let tail = h.rebase(NAME_PATH_TAIL);
                if fd.cast_signed() >= 0 {
                    c(h, SCE_CLOSE, &[fd]);
                    let f = h.rebase(NAME_NAME2_FMT);
                    sprintf(h, &[buf, f, tail, name]);
                } else {
                    let len_at = h.rebase(NAME_PATH_LEN);
                    let first: u8 = h.read(name);
                    let mut i: i32 = h.read::<u32>(len_at).cast_signed();
                    if first == b'.' {
                        // back over the last directory: to its '/', then
                        // to the one before it, and cut after that
                        while i >= 0 && h.read::<u8>(path.wrapping_add_signed(i)) != b'/' {
                            i -= 1;
                        }
                        i -= 1;
                        while i >= 0 && h.read::<u8>(path.wrapping_add_signed(i)) != b'/' {
                            i -= 1;
                        }
                        i += 1;
                        h.write(path.wrapping_add_signed(i), 0_u8);
                        h.write(len_at, i);
                    } else {
                        h.write(path.wrapping_add_signed(i), first);
                        let mut s = name;
                        while h.read::<u8>(s) != 0 {
                            i = i.wrapping_add(1);
                            s = s.wrapping_add(1);
                            h.write(len_at, i);
                            let b: u8 = h.read(s);
                            h.write(path.wrapping_add_signed(i), b);
                        }
                        h.write(path.wrapping_add_signed(i), b'/');
                        i = i.wrapping_add(1);
                        h.write(len_at, i);
                        h.write(path.wrapping_add_signed(i), 0_u8);
                    }
                    c(h, SET_PATH_NAME, &[window, p, 0x108, 0x11]);
                    let f = h.rebase(NAME_NAME_FMT);
                    sprintf(h, &[buf, f, tail]);
                }
                c(h, STRING_COPY, &[p, buf]);
                h.write(at(0x200), 0_u32);
                let mut n = 0_u32;
                while h.read::<u8>(buf.wrapping_add(n)) != 0 {
                    n = n.wrapping_add(1);
                    h.write(at(0x200), n);
                }
                let cur: u32 = h.read(at(0x200));
                h.write(at(0x200), cur.wrapping_sub(5));
            }
            _ => {
                // a key: the table entry whose index + 1 is param
                let keys = h.rebase(NAME_KEYS);
                let mut idx = 0_u32;
                let mut e: u32 = h.read(keys);
                loop {
                    if e == 0 {
                        return;
                    }
                    if e != 0xa && param == idx.wrapping_add(1) {
                        break;
                    }
                    e = h.read(keys.wrapping_add(4 * (idx + 1)));
                    idx += 1;
                }
                let cur = wi(&*h, p, 0x200);
                match param {
                    0x2b => {
                        if cur == 0 {
                            return;
                        }
                        let b = bi(&*h, p, cur + 5);
                        h.write(p.wrapping_add_signed(cur + 4), b);
                        let mut j = 0;
                        while bi(&*h, p, wi(&*h, p, 0x200) + j + 5) != 0 {
                            j += 1;
                            let i = wi(&*h, p, 0x200) + j;
                            let b = bi(&*h, p, i + 5);
                            h.write(p.wrapping_add_signed(i + 4), b);
                        }
                        let cur = wi(&*h, p, 0x200);
                        h.write(at(0x200), cur.wrapping_sub(1));
                    }
                    0x2c => {
                        let b = bi(&*h, p, cur + 6);
                        h.write(p.wrapping_add_signed(cur + 5), b);
                        let mut j = 0;
                        while bi(&*h, p, wi(&*h, p, 0x200) + j + 6) != 0 {
                            j += 1;
                            let i = wi(&*h, p, 0x200) + j;
                            let b = bi(&*h, p, i + 6);
                            h.write(p.wrapping_add_signed(i + 5), b);
                        }
                    }
                    0x2d => {
                        if cur != 0 {
                            h.write(at(0x200), cur - 1);
                        }
                    }
                    0x2e => {
                        if cur != 0x40 {
                            h.write(at(0x200), cur + 1);
                        }
                    }
                    _ => {
                        if cur == 0x40 {
                            return;
                        }
                        let mut end = 0_i32;
                        while bi(&*h, p, end) != 0 {
                            end += 1;
                        }
                        while end >= wi(&*h, p, 0x200) + 5 {
                            let b = bi(&*h, p, end - 1);
                            h.write(p.wrapping_add_signed(end), b);
                            end -= 1;
                        }
                        let s: u32 = h.read(keys.wrapping_add(4 * idx));
                        let ch: u8 = h.read(s);
                        let cur = wi(&*h, p, 0x200);
                        h.write(p.wrapping_add_signed(cur + 5), ch);
                        h.write(at(0x200), cur.wrapping_add(1));
                    }
                }
            }
        }
    }

    #[cfg(test)]
    mod tests {
        #[test]
        fn the_slots_are_the_originals() {
            let s: Vec<u32> = super::slots().collect();
            assert_eq!(s.len(), 18 + 13 + 32 + 10 + 2 + 6 + 16 + 4 + 7);
            assert_eq!(&s[..2], &[0xc, 0x14]);
            assert_eq!(
                &s[73..81],
                &[0x254, 0x25c, 0x260, 0x274, 0x264, 0x27c, 0x268, 0x284]
            );
            assert_eq!(s.last(), Some(&0x7a0));
        }
    }
}
