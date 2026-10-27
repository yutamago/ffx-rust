//! The render-side glue, sixty-seventh batch (M4 port round 19, list
//! `h3-glue-render`): a character instance's cloth vertex buffers read
//! back to the CPU. The cookie is left out.

use ffx_bridge::Global;

/// `mspace_memalign(heap, align, size)` (cdecl) on `g_ClothHeap`
/// (`0x00ccb454`), with `user_aligned_malloc(size, align)` (cdecl) when it
/// fails; `memset` (cdecl).
pub const MSPACE_MEMALIGN: u32 = 0x006e_c8d0;
pub const CLOTH_HEAP: u32 = 0x00cc_b454;
pub const ALIGNED_MALLOC: u32 = 0x0068_6ff0;
pub const MEMSET: u32 = 0x0094_964c;
/// The vertex buffer's `isBound` (thiscall, `al`), its create
/// `0x00561a20` (thiscall, 0 when made), `mapForRead(buffer; i, &out)`
/// and `unmap(buffer; i, &out)` (thiscall, `ret 8`, 0 when done);
/// `Phyre_PhyreOS_Sleep(ms)` (cdecl).
pub const VB_IS_BOUND: u32 = 0x0056_2da0;
pub const VB_CREATE: u32 = 0x0056_1a20;
pub const VB_MAP_FOR_READ: u32 = 0x0056_2e00;
pub const VB_UNMAP: u32 = 0x0056_4ab0;
pub const SLEEP: u32 = 0x0043_0030;
/// The vertex streams' semantic names: two position-like ones (vec3s
/// copied) and the colour (four floats from 16, 8 (halves) or 4 (RGBA
/// bytes) bytes).
pub const NAME_POSITION_A: u32 = 0x00c9_85d8;
pub const NAME_POSITION_B: u32 = 0x00c9_8620;
pub const NAME_COLOR: u32 = 0x00c9_8650;
/// The cloth records' list: its count (`0x00ccb250`) and slots
/// (`0x00ccb254`).
pub const CLOTH_COUNT: u32 = 0x00cc_b250;
pub const CLOTH_SLOTS: u32 = 0x00cc_b254;
/// The bounds' start (100000 and -100000).
pub const BOUND_MAX: Global<u32> = Global::new(0x00b1_0f50);
pub const BOUND_MIN: Global<u32> = Global::new(0x00b3_68b8);

pub mod bridge {
    //! The original over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// The frame (`ebp -` these).
    const ELEMENT: u32 = 0xb8;
    const OBJECT: u32 = 0xb4;
    const ATTRIBUTES: u32 = 0xb0;
    const LAST: u32 = 0xac;
    const MAP_P: u32 = 0xa8;
    const MAP_C: u32 = 0xa0;
    const OUT: u32 = 0x98;
    const PAIR_B: u32 = 0x94;
    const BLOCK_OFF: u32 = 0x90;
    const PAIR_A: u32 = 0x8c;
    const BLOCKS_LEFT: u32 = 0x88;
    const INSTANCE: u32 = 0x84;
    const MESH: u32 = 0x80;
    const ATTRIBUTE: u32 = 0x7c;
    const ATTRIBUTE_OFF: u32 = 0x78;
    const SIZE: u32 = 0x74;
    const COUNT: u32 = 0x70;
    const ELEMENTS: u32 = 0x6c;
    const SUB: u32 = 0x68;
    const CPU: u32 = 0x64;
    const HEADER: u32 = 0x60;
    const CPU_SUB: u32 = 0x5c;
    const SCRATCH_58: u32 = 0x58;
    const BLOCK: u32 = 0x54;
    const ELEMENT_INDEX: u32 = 0x48;
    const LEFT: u32 = 0x44;
    const STRIDE: u32 = 0x40;
    const QWORD: u32 = 0x3c;
    const OFFSET: u32 = 0x38;
    const FLAG: u32 = 0x31;
    const HALF: u32 = 0x30;
    const HALF_SCALE: u32 = 0x50;
    const HALF_LIMIT: u32 = 0x4c;
    const HALVES: u32 = 0x2c;
    const BYTES: u32 = 0x1c;
    const WORDS: u32 = 0xc;

    const MASK: u32 = 0x7fff_ffff;

    /// `size` bytes 16-aligned from the cloth heap, else from
    /// `user_aligned_malloc`.
    fn alloc(h: &mut impl Host, size: u32) -> u32 {
        let heap: u32 = h.read(h.rebase(CLOTH_HEAP));
        let p = h.call_sym(MSPACE_MEMALIGN, Conv::Cdecl, &[heap, 0x10, size]);
        if p != 0 {
            return p;
        }
        h.call_sym(ALIGNED_MALLOC, Conv::Cdecl, &[size, 0x10])
    }

    /// A 0x204-byte list: its count, then 0x200 bytes cleared.
    fn list(h: &mut impl Host, at_frame: u32, count: u32) -> u32 {
        let p = alloc(h, 0x204);
        h.write(at_frame, p);
        h.write(p, count);
        h.call_sym(MEMSET, Conv::Cdecl, &[p + 4, 0, 0x200]);
        p
    }

    /// `graphicCharacterInstanceGetClothVertexBuffers(instance, &cpu)`
    /// (cdecl; frame from `ebp - 0xb8`; Ghidra: void) -> the cloth record
    /// (0 without an instance). The record (0x230 bytes): the instance,
    /// the CPU-side lists (`*cpu`, or new ones stored there when it was
    /// 0), its mesh count, bounds (100000 / -100000), a list per mesh.
    /// For each mesh's element with both pair halves (`+0x20`, `+0x24`)
    /// set, each vertex block's position streams ([`positions`]) and
    /// colour stream ([`colours`]) are read back. The record goes into
    /// the first free slot of the cloth list, or on its end.
    pub fn get_cloth_vertex_buffers(h: &mut impl Host, ebp: u32, instance: u32, out: u32) -> u32 {
        // the map outputs, read even when no map ran (junk reads: 0)
        for at in [MAP_P, MAP_P - 4, MAP_C, MAP_C - 4] {
            h.write(ebp - at, 0_u32);
        }
        let cpu: u32 = h.read(out);
        let new = cpu == 0;
        h.write(ebp - INSTANCE, instance);
        h.write(ebp - OUT, out);
        h.write(ebp - CPU, cpu);
        h.write(ebp - FLAG, u8::from(new));
        if instance == 0 {
            return 0;
        }
        let count = h.read::<u32>(instance) & MASK;
        h.write(ebp - COUNT, count);
        let header = alloc(h, 0x230);
        h.write(ebp - HEADER, header);
        h.write(header, instance);
        h.write(header + 0x14, count);
        h.write(header + 0xc, 0_u32);
        let max = x87::store(x87::load(BOUND_MAX.get(h)));
        for off in [0x20, 0x1c, 0x18] {
            h.write(header + off, max);
        }
        let min = x87::store(x87::load(BOUND_MIN.get(h)));
        for off in [0x2c, 0x28, 0x24] {
            h.write(header + off, min);
        }
        h.write(header + 0x10, 1_u32);
        h.call_sym(MEMSET, Conv::Cdecl, &[header + 0x30, 0, 0x200]);
        let cpu = if new { list(h, ebp - CPU, count) } else { cpu };
        h.write(header + 4, cpu);
        let mut i = 0_u32;
        h.write(ebp - MESH, i);
        while (i as i32) < (count as i32) {
            mesh(h, ebp, instance, header, i, new);
            i += 1;
            h.write(ebp - MESH, i);
        }
        register(h, header);
        if new {
            let cpu: u32 = h.read(ebp - CPU);
            h.write(out, cpu);
        }
        header
    }

    /// The record in the cloth list's first free slot, or appended.
    fn register(h: &mut impl Host, header: u32) {
        let count_at = h.rebase(CLOTH_COUNT);
        let slots = h.rebase(CLOTH_SLOTS);
        let mut n: u32 = h.read(count_at);
        let mut k = 0_u32;
        while k < n {
            if h.read::<u32>(slots + 4 * k) == 0 {
                h.write(slots + 4 * k, header);
                n = h.read(count_at);
                break;
            }
            k += 1;
        }
        if k == n {
            h.write(slots.wrapping_add(4 * n), header);
            let count: u32 = h.read(count_at);
            h.write(count_at, count.wrapping_add(1));
        }
    }

    /// Mesh `i`: its list (and the CPU side's when new), then its
    /// elements.
    fn mesh(h: &mut impl Host, ebp: u32, instance: u32, header: u32, i: u32, new: bool) {
        let meshes: u32 = h.read(instance + 4);
        let slot: u32 = h.read(meshes.wrapping_add(4 * i));
        let object: u32 = h.read(slot);
        h.write(ebp - OBJECT, object);
        let elements = h.read::<u32>(object) & MASK;
        h.write(ebp - ELEMENTS, elements);
        let sub = alloc(h, 0x204);
        h.write(ebp - SUB, sub);
        h.write(header.wrapping_add(4 * i + 0x30), sub);
        h.write(sub, elements);
        h.call_sym(MEMSET, Conv::Cdecl, &[sub + 4, 0, 0x200]);
        h.write(ebp - CPU_SUB, 0_u32);
        if new {
            let cs = alloc(h, 0x204);
            h.write(ebp - CPU_SUB, cs);
            let cpu: u32 = h.read(ebp - CPU);
            h.write(cpu.wrapping_add(4 * i + 4), cs);
            h.write(cs, elements);
            h.call_sym(MEMSET, Conv::Cdecl, &[cs + 4, 0, 0x200]);
        }
        let mut j = 0_u32;
        h.write(ebp - ELEMENT_INDEX, j);
        while (j as i32) < (elements as i32) {
            element(h, ebp, object, i, j, new);
            j += 1;
            h.write(ebp - ELEMENT_INDEX, j);
        }
    }

    /// Element `j` of the mesh: skipped unless both pair halves are set
    /// and it has blocks; each block's streams by their semantic.
    fn element(h: &mut impl Host, ebp: u32, object: u32, i: u32, j: u32, new: bool) {
        let base: u32 = h.read(object + 4);
        let e = j.wrapping_mul(0x6c).wrapping_add(base);
        h.write(ebp - ELEMENT, e);
        let a = u32::from(h.read::<u16>(e.wrapping_add(0x20)));
        let b = u32::from(h.read::<u16>(e.wrapping_add(0x24)));
        h.write(ebp - PAIR_A, a);
        h.write(ebp - PAIR_B, b);
        if a == 0xffff || b == 0xffff {
            return;
        }
        let mut left = h.read::<u32>(e.wrapping_add(0x28)) & MASK;
        h.write(ebp - BLOCKS_LEFT, left);
        if left == 0 {
            return;
        }
        let mut off = 0_u32;
        h.write(ebp - BLOCK_OFF, off);
        loop {
            let blocks: u32 = h.read(e.wrapping_add(0x2c));
            let stride: u32 = h.read(blocks.wrapping_add(off));
            let block = blocks.wrapping_add(off);
            h.write(ebp - BLOCK, block);
            let n = h.read::<u32>(block.wrapping_add(8)) & MASK;
            h.write(ebp - ATTRIBUTES, n);
            h.write(ebp - STRIDE, stride);
            h.write(ebp - ATTRIBUTE, 0_u32);
            if n != 0 {
                h.write(ebp - LAST, n - 1);
                let mut k = 0_u32;
                h.write(ebp - ATTRIBUTE_OFF, 0_u32);
                loop {
                    attribute(h, ebp, block, stride, k, n, i, j, new);
                    k += 1;
                    h.write(ebp - ATTRIBUTE, k);
                    h.write(ebp - ATTRIBUTE_OFF, k.wrapping_mul(0xc));
                    if (k as i32) >= (n as i32) {
                        break;
                    }
                }
            }
            off = off.wrapping_add(0x40);
            left = left.wrapping_sub(1);
            h.write(ebp - BLOCK_OFF, off);
            h.write(ebp - BLOCKS_LEFT, left);
            if left == 0 {
                break;
            }
        }
    }

    /// Attribute `k` of the block: its offset and size (to the next
    /// one's offset, the last one's to the stride), by its semantic.
    #[allow(clippy::too_many_arguments)]
    fn attribute(
        h: &mut impl Host,
        ebp: u32,
        block: u32,
        stride: u32,
        k: u32,
        n: u32,
        i: u32,
        j: u32,
        new: bool,
    ) {
        let attrs: u32 = h.read(block.wrapping_add(0xc));
        let at = attrs.wrapping_add(k.wrapping_mul(0xc));
        let offset: u32 = h.read(at);
        h.write(ebp - OFFSET, offset);
        let end: u32 = if k == n - 1 {
            stride
        } else {
            h.read(at.wrapping_add(0xc))
        };
        let size = end.wrapping_sub(offset);
        let name: u32 = h.read(at.wrapping_add(4));
        h.write(ebp - SIZE, size);
        if name == h.rebase(NAME_POSITION_A) || name == h.rebase(NAME_POSITION_B) {
            positions(h, ebp, block, stride, offset, j, new);
        } else if name == h.rebase(NAME_COLOR) {
            colours(h, ebp, block, stride, offset, size, i, j);
        }
    }

    /// The block's buffer mapped for reading into `ebp - at` (pointer,
    /// size) when it is bound or could be made: after waiting (1 ms
    /// sleeps) while its entry's pending count (`+0x10` of entry `+0x10`
    /// in the `+0x1c` array of `+0x18` entries) is positive; `+0x14` set
    /// when the map answers 0. `scratch`: the index's frame copy.
    fn map(h: &mut impl Host, ebp: u32, block: u32, at: u32, scratch: bool) {
        if h.call_sym(VB_IS_BOUND, Conv::Thiscall(block), &[]) as u8 == 0
            && h.call_sym(VB_CREATE, Conv::Thiscall(block), &[]) != 0
        {
            return;
        }
        let entries = h.read::<u32>(block.wrapping_add(0x18)) & MASK;
        let index: u32 = h.read(block.wrapping_add(0x10));
        if scratch {
            h.write(ebp - SCRATCH_58, index);
        }
        let array: u32 = match entries {
            0 => 0,
            1 => block.wrapping_add(0x1c),
            _ => h.read(block.wrapping_add(0x1c)),
        };
        let pending = array
            .wrapping_add(index.wrapping_mul(20))
            .wrapping_add(0x10);
        while h.read::<i32>(pending) > 0 {
            h.call_sym(SLEEP, Conv::Cdecl, &[1]);
        }
        let r = h.call_sym(VB_MAP_FOR_READ, Conv::Thiscall(block), &[index, ebp - at]);
        if r == 0 {
            h.write(block.wrapping_add(0x14), 1_u8);
        }
    }

    /// The map undone; `+0x14` cleared when the unmap answers 0.
    fn unmap(h: &mut impl Host, ebp: u32, block: u32, at: u32) {
        let index: u32 = h.read(block.wrapping_add(0x10));
        let r = h.call_sym(VB_UNMAP, Conv::Thiscall(block), &[index, ebp - at]);
        if r == 0 {
            h.write(block.wrapping_add(0x14), 0_u8);
        }
    }

    /// Element `j`'s record in the mesh list (0x50 bytes, made with the
    /// pair and 1 when missing).
    fn record(h: &mut impl Host, ebp: u32, j: u32) -> u32 {
        let sub: u32 = h.read(ebp - SUB);
        let slot = sub.wrapping_add(4 * j + 4);
        if h.read::<u32>(slot) == 0 {
            let r = alloc(h, 0x50);
            h.write(slot, r);
            let a: u32 = h.read(ebp - PAIR_A);
            h.write(r + 0x40, a as u16);
            let r: u32 = h.read(slot);
            let b: u32 = h.read(ebp - PAIR_B);
            h.write(r.wrapping_add(0x42), b as u16);
            let r: u32 = h.read(slot);
            h.write(r.wrapping_add(0x48), 1_u32);
        }
        h.read(slot)
    }

    /// A position stream: its count (mapped size / stride, signed); for a
    /// new CPU side, the vec3s copied into a new array. The record's
    /// `+0x44` = the count; the CPU side's entry (made when missing):
    /// the count, the copy, a pair list (0, 0xffff each) and a new
    /// count * 12 bytes (uncleared).
    #[allow(clippy::too_many_arguments, clippy::cast_possible_wrap)]
    fn positions(
        h: &mut impl Host,
        ebp: u32,
        block: u32,
        stride: u32,
        offset: u32,
        j: u32,
        new: bool,
    ) {
        map(h, ebp, block, MAP_P, false);
        let ptr: u32 = h.read(ebp - MAP_P);
        h.write(ebp - SCRATCH_58, ptr);
        let size: u32 = h.read(ebp - MAP_P + 4);
        let n = ((size as i32).wrapping_div(stride as i32)) as u32;
        h.write(ebp - SIZE, n);
        let mut copy = 0;
        if new {
            copy = alloc(h, n.wrapping_mul(12));
            let mut src = ptr.wrapping_add(offset);
            let mut dst = copy;
            for _ in 0..(n as i32).max(0) {
                for w in 0..3 {
                    let v: u32 = h.read(src.wrapping_add(4 * w));
                    h.write(dst.wrapping_add(4 * w), v);
                }
                src = src.wrapping_add(stride);
                dst = dst.wrapping_add(12);
            }
        }
        let bytes = n.wrapping_mul(12);
        h.write(ebp - OFFSET, bytes);
        unmap(h, ebp, block, MAP_P);
        let r = record(h, ebp, j);
        let m = ((bytes as i32) / 12) as u32;
        h.write(r.wrapping_add(0x44), m);
        if !new {
            return;
        }
        let cs: u32 = h.read(ebp - CPU_SUB);
        let slot = cs.wrapping_add(4 * j + 4);
        if h.read::<u32>(slot) == 0 {
            let e = alloc(h, 0x10);
            h.write(slot, e);
        }
        let e: u32 = h.read(slot);
        h.write(e.wrapping_add(4), copy);
        let e: u32 = h.read(slot);
        h.write(e, m);
        let list = alloc(h, m.wrapping_mul(8));
        let e: u32 = h.read(slot);
        h.write(e.wrapping_add(8), list);
        let mut q = 0_i32;
        while q < m as i32 {
            q += 1;
            let at = (q as u32).wrapping_mul(8);
            let e: u32 = h.read(slot);
            let list: u32 = h.read(e.wrapping_add(8));
            h.write(list.wrapping_add(at).wrapping_sub(6), 0xffff_u16);
            let e: u32 = h.read(slot);
            let list: u32 = h.read(e.wrapping_add(8));
            h.write(list.wrapping_add(at).wrapping_sub(8), 0_u16);
        }
        let buffer = alloc(h, bytes);
        let e: u32 = h.read(slot);
        h.write(e.wrapping_add(0xc), buffer);
    }

    /// A colour stream: its count (mapped size / stride, signed, raised
    /// to the CPU side's count for the element, unsigned), four floats
    /// per vertex (16 bytes copied; 8 bytes as halves; 4 bytes as RGBA
    /// bytes / 255, high byte first; other sizes left), kept in the
    /// record's `+0x4c`.
    #[allow(clippy::too_many_arguments, clippy::cast_possible_wrap)]
    fn colours(
        h: &mut impl Host,
        ebp: u32,
        block: u32,
        stride: u32,
        offset: u32,
        size: u32,
        i: u32,
        j: u32,
    ) {
        map(h, ebp, block, MAP_C, true);
        let size_mapped: u32 = h.read(ebp - MAP_C + 4);
        let mut n = ((size_mapped as i32).wrapping_div(stride as i32)) as u32;
        let ptr: u32 = h.read(ebp - MAP_C);
        h.write(ebp - LEFT, n);
        let cpu: u32 = h.read(ebp - CPU);
        let cs: u32 = h.read(cpu.wrapping_add(4 * i + 4));
        if cs != 0 {
            let e: u32 = h.read(cs.wrapping_add(4 * j + 4));
            if e != 0 {
                let c: u32 = h.read(e);
                if n < c {
                    n = c;
                    h.write(ebp - LEFT, c);
                }
            }
        }
        let out = alloc(h, n << 4);
        h.write(ebp - SCRATCH_58, out);
        if (n as i32) > 0 {
            let mut src = ptr.wrapping_add(offset);
            let mut dst = out;
            let mut left = n;
            loop {
                match size {
                    0x10 => {
                        for w in 0..4 {
                            let v: u32 = h.read(src.wrapping_add(4 * w));
                            h.write(dst.wrapping_add(4 * w), v);
                        }
                    }
                    8 => halves(h, ebp, src, dst),
                    4 => bytes(h, ebp, src, dst),
                    _ => {}
                }
                src = src.wrapping_add(stride);
                dst = dst.wrapping_add(0x10);
                left = left.wrapping_sub(1);
                h.write(ebp - LEFT, left);
                if left == 0 {
                    break;
                }
            }
        }
        unmap(h, ebp, block, MAP_C);
        let r = record(h, ebp, j);
        let out: u32 = h.read(ebp - SCRATCH_58);
        h.write(r.wrapping_add(0x4c), out);
    }

    /// Four halves (two words at `src`) as floats: the magnitude's bits
    /// moved up 13, times 2^112 (`fmul`, f32), infinity's exponent or'd in
    /// above 65536, the sign put back. The frame keeps the words, the
    /// scratch, the constants and the four results.
    fn halves(h: &mut impl Host, ebp: u32, src: u32, dst: u32) {
        let lo: u32 = h.read(src);
        let hi: u32 = h.read(src.wrapping_add(4));
        h.write(ebp - WORDS, lo);
        h.write(ebp - WORDS + 4, hi);
        let mut out = [0_u32; 4];
        for (k, half) in [lo & 0xffff, lo >> 16, hi & 0xffff, hi >> 16]
            .into_iter()
            .enumerate()
        {
            let bits = (half & 0x7fff) << 13;
            h.write(ebp - HALF, bits);
            h.write(ebp - HALF_SCALE, 0x7780_0000_u32);
            h.write(ebp - HALF_LIMIT, 0x4780_0000_u32);
            let v = x87::store(x87::mul(x87::load(bits), x87::load(0x7780_0000)));
            h.write(ebp - HALF, v);
            let limit = x87::load(0x4780_0000);
            let v = if limit < x87::load(v) {
                v | 0x7f80_0000
            } else {
                v
            };
            out[k] = ((half & 0x8000) << 16) | v;
            if k == 0 {
                h.write(ebp - OFFSET, out[0]);
            }
            h.write(ebp - HALVES + 4 * k as u32, out[k]);
        }
        for (k, v) in (0u32..).zip(out) {
            h.write(dst.wrapping_add(4 * k), v);
        }
    }

    /// An RGBA word's bytes, high first, each `fild`, kept as a qword,
    /// / 255 and stored as f32.
    fn bytes(h: &mut impl Host, ebp: u32, src: u32, dst: u32) {
        let word: u32 = h.read(src);
        h.write(ebp - LEFT, word);
        let mut out = [0_u32; 4];
        for (k, shift) in [24_u32, 16, 8, 0].into_iter().enumerate() {
            let byte = (word >> shift) & 0xff;
            h.write(ebp - OFFSET, byte);
            let v = f64::from(byte);
            h.write(ebp - QWORD, v);
            out[k] = x87::store(x87::div(v, 255.0));
            h.write(ebp - BYTES + 4 * k as u32, out[k]);
        }
        for (k, v) in (0u32..).zip(out) {
            h.write(dst.wrapping_add(4 * k), v);
        }
    }
}
