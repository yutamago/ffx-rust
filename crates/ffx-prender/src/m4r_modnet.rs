//! The modifier network's packet run (`0x005c1f90`, list p6-render-c;
//! `m4-rest`): `PModifierNetworkDynamicMesh::fillPacket`
//! (`0x005ae0f0`/`0x005ae1d0`) hands it a packet, and it streams the
//! packet's items through the network's modifiers in chunks that fit a
//! 256 KB scratch buffer.
//!
//! `cdecl(packet)` -> the packet. Layouts used:
//! - the packet: `+0x10` the network (`+4` k, `+8` the modifier count,
//!   `+0xc` the source stream count, `+0xe` the output stream count,
//!   `+0x20` 32-byte modifier records (`+0` the modifier object, its
//!   `+0x24` the process function; `+4` its input count, `+8` its output
//!   count), from `+0x20 (k + 1)` 32-byte modifier entries (`+0` a base
//!   added to the packet's `+0x18`, byte `+8` the record, bytes `+9..`
//!   its input then output stream numbers), then the u16 element sizes
//!   of the source and output streams); `+0x12` the offset (from `+0x10`)
//!   of the outputs' destinations (one word each, 0 = scratch only);
//!   `+0x14` the source pointers; `+0x24` the item count; `+0x24 + 2m`
//!   (u16, from `+0x10`) each modifier's argument; `+0x28`/`+0x2c` a
//!   word stored through `+0x28` at the end;
//! - the stream records (20 bytes, `_alloca`'d): `+0` the region start,
//!   `+4` the other cursor, `+8` this chunk's data, `+0xc` the source or
//!   destination, `+0x10` the element size.
//!
//! The scratch buffer is split as the original splits it: the outputs
//! without a destination from its start, the sources and the outputs
//! with one from `chunk * zero-size`, their second cursors from `chunk *
//! sizes` (the two last regions overlap as in the original).

use ffx_graphic::host::{Conv, Host};

/// `Phyre_PhyreMalloc_0042fb80` (cdecl `(size, align)`).
pub const MALLOC: u32 = 0x0042_fb80;
/// `Phyre_PhyreFree` (cdecl `(p)`).
pub const FREE: u32 = 0x0042_fb20;
/// `memcpy` (cdecl `(dst, src, n)`).
pub const MEMCPY: u32 = 0x0094_9256;
/// The scratch buffer's size.
pub const SCRATCH: u32 = 0x4_0000;

/// The chunk length in items for streams of `sizes` bytes per item
/// (`zero + 2 * dest`, at least 1), a multiple of 8.
#[must_use]
pub fn chunk_items(total: u32) -> u32 {
    (SCRATCH / total.max(1)) & !7
}

pub mod bridge {
    //! The original over game memory.

    use super::{Conv, FREE, Host, MALLOC, MEMCPY, SCRATCH, chunk_items};

    /// A 20-byte stream record's field.
    fn field(rec: u32, off: u32) -> u32 {
        rec.wrapping_add(off)
    }

    /// The network's layout, read once as the original reads it.
    struct Net {
        /// `packet + 0x10`.
        desc: u32,
        modifiers: u32,
        sources: u32,
        outputs: u32,
        /// `+0x20`: the 32-byte modifier records.
        records: u32,
        /// The 32-byte modifier entries.
        entries: u32,
        /// The u16 element sizes, sources then outputs.
        sizes: u32,
        /// The outputs' destinations (one word each).
        dests: u32,
    }

    impl Net {
        fn read(h: &impl Host, packet: u32) -> Self {
            let desc = packet.wrapping_add(0x10);
            let n: u32 = h.read(desc);
            let k: u32 = h.read(n + 4);
            let modifiers: u32 = h.read(n + 8);
            Self {
                desc,
                modifiers,
                sources: u32::from(h.read::<u16>(n + 0xc)),
                outputs: u32::from(h.read::<u16>(n + 0xe)),
                records: n + 0x20,
                entries: (k.wrapping_add(1) << 5).wrapping_add(n),
                sizes: (k.wrapping_add(1).wrapping_add(modifiers) << 5).wrapping_add(n),
                dests: u32::from(h.read::<u16>(desc + 0x12)).wrapping_add(desc),
            }
        }

        fn size(&self, h: &impl Host, i: u32) -> u32 {
            u32::from(h.read::<u16>(self.sizes.wrapping_add(i << 1)))
        }

        fn dest(&self, h: &impl Host, j: u32) -> u32 {
            h.read(self.dests.wrapping_add(j << 2))
        }
    }

    /// The modifier run: `packet` the packet, `ctx` the original's two
    /// frame words at `ebp - 0x58` (the modifier's context: its base,
    /// and a count the original clears before each chunk's modifiers and
    /// adds the chunk to after them), `alloca(bytes)` the original's
    /// `_alloca` (16-byte aligned, below the previous one).
    #[allow(clippy::too_many_lines)] // one original
    pub fn run(
        h: &mut impl Host,
        packet: u32,
        ctx: u32,
        alloca: &mut impl FnMut(u32) -> u32,
    ) -> u32 {
        let buf = h.call_sym(MALLOC, Conv::Cdecl, &[SCRATCH, 1]);
        let net = Net::read(h, packet);
        // each modifier's argument
        let args = alloca(net.modifiers << 2);
        for m in 0..net.modifiers {
            let off = u32::from(h.read::<u16>(net.desc + 0x24 + 2 * m));
            h.write(args + 4 * m, off.wrapping_add(net.desc));
        }
        // the per-item sizes: all sources, then each output on the side
        // its destination says
        let mut sources_size = 0_u32;
        for i in 0..net.sources {
            sources_size = sources_size.wrapping_add(net.size(h, i));
        }
        let (mut dest_size, mut zero_size) = (sources_size, 0_u32);
        for j in 0..net.outputs {
            let s = net.size(h, net.sources + j);
            if net.dest(h, j) == 0 {
                zero_size = zero_size.wrapping_add(s);
            } else {
                dest_size = dest_size.wrapping_add(s);
            }
        }
        let chunk = chunk_items(zero_size.wrapping_add(dest_size.wrapping_mul(2)));
        let mut zero_cursor = buf;
        let mut first = chunk.wrapping_mul(zero_size).wrapping_add(buf);
        let mut second = chunk.wrapping_mul(dest_size).wrapping_add(buf);
        let src_recs = alloca(net.sources.wrapping_mul(20));
        let out_recs = alloca(net.outputs.wrapping_mul(20));
        let sources: u32 = h.read(packet + 0x14);
        let mut left: u32 = h.read(packet + 0x24);
        for i in 0..net.sources {
            let rec = src_recs.wrapping_add(i * 20);
            let size = net.size(h, i);
            let bytes = size.wrapping_mul(chunk);
            h.write(field(rec, 0), first);
            h.write(field(rec, 4), second);
            second = second.wrapping_add(bytes);
            first = first.wrapping_add(bytes);
            let src: u32 = h.read(sources.wrapping_add(i << 2));
            h.write(field(rec, 0xc), src);
            h.write(field(rec, 0x10), size);
        }
        for j in 0..net.outputs {
            let rec = out_recs.wrapping_add(j * 20);
            let size = net.size(h, net.sources + j);
            let dest = net.dest(h, j);
            h.write(field(rec, 0xc), dest);
            h.write(field(rec, 0x10), size);
            let bytes = size.wrapping_mul(chunk);
            if dest != 0 {
                h.write(field(rec, 0), first);
                first = first.wrapping_add(bytes);
                h.write(field(rec, 4), second);
                second = second.wrapping_add(bytes);
            } else {
                h.write(field(rec, 0), zero_cursor);
                h.write(field(rec, 4), zero_cursor);
                zero_cursor = zero_cursor.wrapping_add(bytes);
            }
        }
        // the widest modifier's input and output pointer lists
        let (mut max_in, mut max_out) = (0_u32, 0_u32);
        for m in 0..net.modifiers {
            let record = u32::from(h.read::<u8>(net.entries + 32 * m + 8));
            let r = (record << 5).wrapping_add(net.records);
            max_in = max_in.max(h.read(r + 4));
            max_out = max_out.max(h.read(r + 8));
        }
        let inputs = alloca(max_in << 2);
        let outputs = alloca(max_out << 2);
        while left != 0 {
            let n = left.min(chunk);
            left -= n;
            for i in 0..net.sources {
                let rec = src_recs.wrapping_add(i * 20);
                let src: u32 = h.read(field(rec, 0xc));
                h.write(field(rec, 8), src);
                let size: u32 = h.read(field(rec, 0x10));
                h.write(field(rec, 0xc), src.wrapping_add(size.wrapping_mul(n)));
            }
            for j in 0..net.outputs {
                let rec = out_recs.wrapping_add(j * 20);
                let start: u32 = h.read(field(rec, 0));
                h.write(field(rec, 8), start);
            }
            // the items done, as the modifiers see them: 0 in every chunk
            h.write(ctx + 4, 0_u32);
            for m in 0..net.modifiers {
                let entry = net.entries.wrapping_add(32 * m);
                let record = u32::from(h.read::<u8>(entry + 8));
                let r = (record << 5).wrapping_add(net.records);
                let modifier: u32 = h.read(r);
                let mut byte = entry + 9;
                let n_in: u32 = h.read(r + 4);
                for c in 0..n_in {
                    let s = u32::from(h.read::<u8>(byte));
                    byte += 1;
                    let data: u32 = if s < net.sources {
                        h.read(src_recs.wrapping_add(s * 20) + 8)
                    } else {
                        h.read(out_recs.wrapping_add((s - net.sources).wrapping_mul(20)) + 8)
                    };
                    h.write(inputs.wrapping_add(c << 2), data);
                }
                let n_out: u32 = h.read(r + 8);
                for c in 0..n_out {
                    // an 8-bit difference: a source number wraps past 255
                    let o = u32::from(h.read::<u8>(byte).wrapping_sub(net.sources as u8));
                    byte += 1;
                    let data: u32 = if o < net.outputs {
                        h.read(out_recs.wrapping_add(o * 20) + 8)
                    } else {
                        0
                    };
                    h.write(outputs.wrapping_add(c << 2), data);
                }
                let base: u32 = h.read(entry);
                let offset: u32 = h.read(packet + 0x18);
                h.write(ctx, base.wrapping_add(offset));
                let process: u32 = h.read(modifier + 0x24);
                let arg: u32 = h.read(args.wrapping_add(m << 2));
                h.call(process, Conv::Cdecl, &[arg, n, inputs, outputs, ctx]);
            }
            for j in 0..net.outputs {
                let rec = out_recs.wrapping_add(j * 20);
                let dest: u32 = h.read(field(rec, 0xc));
                if dest != 0 {
                    let size: u32 = h.read(field(rec, 0x10));
                    let bytes = size.wrapping_mul(n);
                    let data: u32 = h.read(field(rec, 8));
                    h.call_sym(MEMCPY, Conv::Cdecl, &[dest, data, bytes]);
                    let dest: u32 = h.read(field(rec, 0xc));
                    h.write(field(rec, 0xc), dest.wrapping_add(bytes));
                }
            }
            let done: u32 = h.read(ctx + 4);
            h.write(ctx + 4, done.wrapping_add(n));
        }
        let out: u32 = h.read(packet + 0x28);
        if out != 0 {
            let v: u32 = h.read(packet + 0x2c);
            h.write(out, v);
        }
        if buf != 0 {
            h.call_sym(FREE, Conv::Cdecl, &[buf]);
        }
        packet
    }
}
