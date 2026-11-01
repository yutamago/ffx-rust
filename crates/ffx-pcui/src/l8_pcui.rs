//! M5 stage 1, round 8 (list `left8`, from `pc-ui.list`): Iggy `GDraw`'s
//! D3D11 object release at context teardown (`GDraw_releaseObjects`).

/// The originals over game memory.
pub mod bridge {
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
