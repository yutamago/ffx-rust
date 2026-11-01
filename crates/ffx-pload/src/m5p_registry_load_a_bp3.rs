//! M5 stage 1, list `phyre-registry-load-a`, round big-phyre3
//! (`ffx_pload::m5p_registry_load_a_bp3`). Parity tests:
//! `crates/ffx-oracle/tests/m5p_registry_load_a.rs` (`bp3_*`).
//!
//! The cluster iterator the original keeps in its frame (and hands the
//! header's seek and next methods) sits at the original's offsets below
//! `ebp`; the shim passes the top of a `StackFrame`, the tests the
//! original's frame address.

/// `0x00466da0`'s callees: `PhyreMalloc(size, align)`, the render
/// interface's constructor (`0x004fee70`, `ecx`), `atexit`,
/// `PRenderInterface::getTextureQuality` (`ecx`),
/// `PStreamReader::readExact` (thiscall `(reader; buffer, size)`),
/// `PhyreFree(p)`.
pub const RELOAD_CALLS: [u32; 6] = [
    0x0042_fb80,
    0x004f_ee70,
    0x0094_9207,
    0x0059_47d0,
    0x0057_dd30,
    0x0042_fb20,
];
/// The render interface singleton, its guard word (bit 0) and the
/// teardown `atexit` gets.
pub const RENDER_INTERFACE: [u32; 3] = [0x00c9_4f00, 0x00c9_628c, 0x00af_b710];
/// The bad texture format (creation leaves a texture on it: 7).
pub const BAD_FORMAT: u32 = 0x00ca_34cc;
/// `0x00466da0`'s frame below `ebp` (the iterator at `ebp - 0x54`), in
/// words.
pub const RELOAD_FRAME_WORDS: u32 = 0x54 / 4;

/// One texture kind's calls: the class descriptor, the iterator's seek
/// and next, `isBound`, `unbind`, the dimension setter, the device
/// creation and the bind.
#[derive(Clone, Copy)]
pub struct Kind {
    pub class: u32,
    pub seek: u32,
    pub next: u32,
    pub is_bound: u32,
    pub unbind: u32,
    pub dimensions: u32,
    pub create: u32,
    pub bind: u32,
}

/// `Texture2D`, `Texture3D`, `TextureCubeMap`, in the original's order.
pub const KINDS: [Kind; 3] = [
    Kind {
        class: 0x00ca_37d0,
        seek: 0x0046_4f30,
        next: 0x0046_53e0,
        is_bound: 0x0059_5f90,
        unbind: 0x0059_cc40,
        dimensions: 0x004c_40c0,
        create: 0x0058_fc20,
        bind: 0x0058_db00,
    },
    Kind {
        class: 0x00ca_3b60,
        seek: 0x0046_4fa0,
        next: 0x0046_5440,
        is_bound: 0x0059_5fa0,
        unbind: 0x0059_cf00,
        dimensions: 0x004c_5980,
        create: 0x0059_03a0,
        bind: 0x0058_db50,
    },
    Kind {
        class: 0x00ca_3ef0,
        seek: 0x0046_5010,
        next: 0x0046_54a0,
        is_bound: 0x0059_5fb0,
        unbind: 0x0059_d0f0,
        dimensions: 0x004c_7260,
        create: 0x0059_0800,
        bind: 0x0058_dba0,
    },
];

pub mod reload {
    //! `0x00466da0` over game memory.

    use super::{BAD_FORMAT, KINDS, Kind, RELOAD_CALLS, RENDER_INTERFACE};
    use ffx_graphic::host::{Conv, Host};

    fn this_call(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    /// Where the frame iterator lives: `{cursor, node, stride, left,
    /// offset, list, first, class}` from `ebp - 0x54`.
    fn iter(ebp: u32) -> u32 {
        ebp - 0x54
    }

    /// The iterator over the header's object list `list` for `class`,
    /// sought to the first object.
    fn start(h: &mut impl Host, ebp: u32, list: u32, kind: &Kind) {
        let it = iter(ebp);
        let head: u32 = h.read(list);
        let first = if head == list { 0 } else { head };
        let class = h.rebase(kind.class);
        h.write(it, [0u32, 0, 0, 0, 0, list, first, class]);
        this_call(h, kind.seek, it, &[]);
    }

    /// The iterator moved on one object (past the block ends it meets);
    /// `false` when it has no more.
    fn advance(h: &mut impl Host, ebp: u32, kind: &Kind) -> bool {
        let it = iter(ebp);
        let mut left: u32 = h.read(it + 0xc);
        if left == 0 {
            return false;
        }
        loop {
            let cursor = h.read::<u32>(it).wrapping_add(h.read::<u32>(it + 8));
            left -= 1;
            h.write(it, cursor);
            h.write(it + 0xc, left);
            let node: u32 = h.read(it + 4);
            if node != cursor {
                if left != 0 {
                    return true;
                }
                break;
            }
            let next: u32 = h.read(node);
            h.write(it + 4, next);
            if left == 0 {
                break;
            }
        }
        this_call(h, kind.next, it, &[]);
        h.read::<u32>(it + 0xc) != 0
    }

    /// A dimension of the texture's record, or the texture's own (`at`)
    /// when 0.
    fn or_own(h: &impl Host, v: u32, texture: u32, at: u32) -> u32 {
        if v == 0 { h.read(texture + at) } else { v }
    }

    /// Halved, at least 1.
    fn half(v: u32) -> u32 {
        (v >> 1).max(1)
    }

    /// The byte size of one level of `texture` (`w`, `h`, `d` its
    /// record's level dimensions, 0 meaning the texture's own at
    /// `+0x1c`/`+0x20`/`+0x24`): `bits * w * h / 8`, block formats (format
    /// byte `+0x10` bit 0) rounding `w` and `h` up to 4; times the depth
    /// for a 3D texture; square for a cube face.
    fn level_size(h: &impl Host, texture: u32, kind: usize, [w, hg, d]: [u32; 3]) -> u32 {
        let fmt: u32 = h.read(texture);
        let mut ww = or_own(h, w, texture, 0x1c);
        let mut hh = if kind == 2 {
            ww
        } else {
            or_own(h, hg, texture, 0x20)
        };
        if h.read::<u8>(fmt + 0x10) & 1 != 0 {
            ww = ww.wrapping_add(3) & !3;
            hh = hh.wrapping_add(3) & !3;
        }
        let bits: u32 = h.read(fmt + 0xc);
        let size = bits.wrapping_mul(ww).wrapping_mul(hh) >> 3;
        if kind == 1 {
            size.wrapping_mul(or_own(h, d, texture, 0x24))
        } else {
            size
        }
    }

    /// The levels `0..=levels` of a texture (six times for a cube) read
    /// from the stream into `buf`, each kept (the next read after it)
    /// unless it is one of the `quality` levels dropped (`level <
    /// quality <= levels`), the dimensions halved down to 1 per level.
    /// The first read error.
    fn read_levels(
        h: &mut impl Host,
        reader: u32,
        texture: u32,
        buf: u32,
        (kind, levels, quality): (usize, u32, u32),
        dims: [u32; 3],
    ) -> u32 {
        let read_exact = RELOAD_CALLS[4];
        let mut dst = buf;
        let faces = if kind == 2 { 6 } else { 1 };
        for _ in 0..faces {
            let mut d3 = dims;
            let mut level = 0u32;
            loop {
                let size = level_size(h, texture, kind, d3);
                let err = h.call_sym(read_exact, Conv::Thiscall(reader), &[dst, size]);
                if err != 0 {
                    return err;
                }
                if level >= quality || levels < quality {
                    dst = dst.wrapping_add(size);
                }
                d3 = d3.map(half);
                level += 1;
                if level > levels {
                    break;
                }
            }
        }
        0
    }

    /// One texture of a kind: unbound when bound, its levels read
    /// ([`read_levels`]), its dimensions set to the kept levels when the
    /// quality drops some (each dimension `>> quality`, `levels -
    /// quality`), the device texture made from `buf`, 7 when it ends on
    /// the bad format, bound when not; the first error.
    fn one(
        h: &mut impl Host,
        reader: u32,
        (k, rec): (usize, u32),
        texture: u32,
        buf: u32,
        quality: u32,
    ) -> u32 {
        let kind = &KINDS[k];
        if this_call(h, kind.is_bound, texture, &[]) & 0xff != 0 {
            this_call(h, kind.unbind, texture, &[]);
        }
        let levels: u32 = h.read(rec + 0xc);
        let dims: [u32; 3] = h.read(rec + 0x1c);
        let dims = if k == 0 { [dims[0], dims[1], 0] } else { dims };
        let err = read_levels(h, reader, texture, buf, (k, levels, quality), dims);
        if err != 0 {
            return err;
        }
        if levels >= quality {
            let fmt: u32 = h.read(texture);
            let lv = levels.wrapping_sub(quality);
            let s = |v: u32| v >> (quality & 31);
            match k {
                0 => this_call(
                    h,
                    kind.dimensions,
                    texture,
                    &[s(dims[0]), s(dims[1]), fmt, lv],
                ),
                1 => this_call(
                    h,
                    kind.dimensions,
                    texture,
                    &[s(dims[0]), s(dims[1]), s(dims[2]), fmt, lv],
                ),
                _ => this_call(h, kind.dimensions, texture, &[s(dims[0]), fmt, lv]),
            };
        }
        let err = this_call(h, kind.create, texture, &[buf, 1, 0, 0]);
        if err != 0 {
            return err;
        }
        if h.read::<u32>(texture) == h.rebase(BAD_FORMAT) {
            return 7;
        }
        if this_call(h, kind.is_bound, texture, &[]).to_le_bytes()[0] == 0 {
            let err = this_call(h, kind.bind, texture, &[]);
            if err != 0 {
                return err;
            }
        }
        0
    }

    /// `Phyre_PBinary_reloadClusterTextures(reader, info)` (stdcall, ret
    /// 8): nothing (0) without textures (`[info + 0x24]`); else a buffer
    /// of twice that many bytes (`PhyreMalloc(.., 0x10)`), the render
    /// interface made once (guard bit, `atexit`), its texture quality,
    /// then the cluster header's (`[info + 4]`) 2D textures (records
    /// 0x74 bytes at `[info + 0x10]`), 3D textures (0x68 bytes at
    /// `[info + 0x18]`) and cube maps (0x68 bytes at `[info + 0x20]`)
    /// each reloaded ([`one`]) through the frame iterator; the buffer
    /// freed; 0, or the first error (the buffer not freed then).
    pub fn reload_cluster_textures(h: &mut impl Host, ebp: u32, reader: u32, info: u32) -> u32 {
        let [malloc, ctor, atexit, quality_of, _, free] = RELOAD_CALLS;
        let header: u32 = h.read(info + 4);
        let count: u32 = h.read(info + 0x24);
        if count == 0 {
            return 0;
        }
        let buf = h.call_sym(malloc, Conv::Cdecl, &[count.wrapping_mul(2), 0x10]);
        let [ri, guard, teardown] = RENDER_INTERFACE.map(|a| h.rebase(a));
        let g: u32 = h.read(guard);
        if g & 1 == 0 {
            h.write(guard, g | 1);
            this_call(h, ctor, ri, &[]);
            h.call_sym(atexit, Conv::Cdecl, &[teardown]);
        }
        let quality = this_call(h, quality_of, ri, &[]);
        let list = header + 0x1c;
        for (k, (records, stride)) in [(0x10u32, 0x74u32), (0x18, 0x68), (0x20, 0x68)]
            .into_iter()
            .enumerate()
        {
            let kind = &KINDS[k];
            start(h, ebp, list, kind);
            let it = iter(ebp);
            if h.read::<u32>(it + 0xc) == 0 {
                continue;
            }
            let mut off = 0u32;
            loop {
                let texture = h.read::<u32>(it + 0x10).wrapping_add(h.read(it));
                let rec = h.read::<u32>(info + records).wrapping_add(off);
                let err = one(h, reader, (k, rec), texture, buf, quality);
                if err != 0 {
                    return err;
                }
                off = off.wrapping_add(stride);
                if !advance(h, ebp, kind) {
                    break;
                }
            }
        }
        h.call_sym(free, Conv::Cdecl, &[buf]);
        0
    }
}
