//! The op layer's model (`op_md_*`) helpers at `0x00a69000..0x00a6d000`:
//! packet copies and counts, the oef2 entries' links and the four-light
//! set-up of the model draws (`docs/formats/vfx.md`, "High range").

/// The functions over game memory.
pub mod bridge {
    use ffx_bridge::Mem;

    /// Calls the game function at `sym` (rebased) through `call`.
    fn go<M: Mem>(
        mem: &mut M,
        sym: u32,
        args: &[u32],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = mem.rebase(sym);
        call(mem, f, args)
    }

    fn ptr32(p: u32) -> u32 {
        ffx_system::stub::ptr32(p)
    }

    /// `op_md_copy(src, dst)` (`0x00a69290`): the packet `src` copied to
    /// `dst`, `op_md_packet_size(src) / 16` (C division) 16-byte blocks,
    /// one word read and stored at a time from the start (so an
    /// overlapping `dst` above `src` sees its own stores). A size of 0 is
    /// reported (`dbgPrintf` of `0x00b7c3b4`, `debug_exit_trace(1)`,
    /// through `call`) and copies nothing. Returns `eax` as left: the
    /// last word copied, or the size when no block was.
    pub fn op_md_copy<M: Mem>(
        mem: &mut M,
        src: u32,
        dst: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let size = crate::op::bridge::op_md_packet_size(mem, src);
        if size == 0 {
            let msg = mem.rebase(0x00b7_c3b4);
            go(mem, 0x0062_f500, &[msg], call);
            go(mem, 0x0062_f510, &[1], call);
        }
        let blocks = size.cast_signed() / 16;
        let mut eax = size;
        let delta = dst.wrapping_sub(src);
        let mut s = src;
        for _ in 0..blocks.max(0) {
            for k in 0..4u32 {
                let w: u32 = mem.read(s.wrapping_add(4 * k));
                mem.write(s.wrapping_add(delta).wrapping_add(4 * k), w);
                eax = w;
            }
            s = s.wrapping_add(0x10);
        }
        eax
    }

    /// The packets of a model list: `u16 list +4` of them, a `ptr32` to
    /// each at `list + 0x10 + 0x10 k`.
    fn packets(mem: &impl Mem, list: u32) -> impl Iterator<Item = u32> + '_ {
        let n = u32::from(mem.read::<u16>(list.wrapping_add(4)));
        (0..n).map(move |k| ptr32(mem.read(list.wrapping_add(0x10 + 0x10 * k))))
    }

    /// The tags of a packet from `+0x20` while `keep(type byte +1)`: each
    /// `0x20 + 0x1c n` bytes, `n` the `i16 +2`; yields the `n`s.
    fn tag_counts(mem: &impl Mem, packet: u32, keep: impl Fn(u8) -> bool) -> Vec<i32> {
        let mut out = Vec::new();
        let mut t = packet.wrapping_add(0x20);
        while keep(mem.read(t.wrapping_add(1))) {
            let n = i32::from(mem.read::<i16>(t.wrapping_add(2)));
            out.push(n);
            t = t
                .wrapping_add(0x20)
                .wrapping_add(n.cast_unsigned().wrapping_mul(0x1c));
        }
        out
    }

    /// `op_md_list_count10_00a6a2b0(list)` (`0x00a6a2b0`): the `i16 +2`
    /// counts summed over every packet's leading tags of type byte 0x10
    /// (0x1c bytes each element), 32-bit wrapping;
    /// `opp_0437_cmd_a6dbe0` kind 0x4000 sizes its buffer from it.
    pub fn op_md_list_count10(mem: &impl Mem, list: u32) -> u32 {
        let mut sum = 0i32;
        for p in packets(mem, list).collect::<Vec<_>>() {
            for n in tag_counts(mem, p, |b| b == 0x10) {
                sum = sum.wrapping_add(n);
            }
        }
        sum.cast_unsigned()
    }

    /// `opus_0582_break_bytes_00a72310(list)` (`0x00a72310`): the
    /// `opus_0582` break buffer's size for a model list: `0x300` bytes per
    /// element of every tag up to each packet's end tag (type byte
    /// 0xff), plus 0x40 (32-bit wrapping); `opp_0582_cmd_a70910` kind
    /// 0x4000.
    pub fn opus_0582_break_bytes(mem: &impl Mem, list: u32) -> u32 {
        let mut sum = 0i32;
        for p in packets(mem, list).collect::<Vec<_>>() {
            for n in tag_counts(mem, p, |b| b != 0xff) {
                sum = sum.wrapping_add(n.wrapping_mul(0x300));
            }
        }
        sum.wrapping_add(0x40).cast_unsigned()
    }

    /// `op_oef2_entry_link_00a6a320(e, i)` (`0x00a6a320`): link `i` of the
    /// oef2 entry `e`, `t = ptr32(e +0x14)` its table: below `0x100000`
    /// `t` is an offset from `e` and the link `ptr32(e + t + 32 i +
    /// 0x1c)` one too (0 when null); else the link is `ptr32(t + 32 i +
    /// 0x1c)` itself, 0 when it is `e`. `opu_part_init` kind 0x2000.
    pub fn op_oef2_entry_link(mem: &impl Mem, e: u32, i: u32) -> u32 {
        let t = ptr32(mem.read(e.wrapping_add(0x14)));
        let off = i.wrapping_shl(5);
        if t < 0x10_0000 {
            let at = t.wrapping_add(off).wrapping_add(e).wrapping_add(0x1c);
            let l: u32 = mem.read(at);
            if ffx_system::stub::is_ptr32_null(l) != 0 {
                return 0;
            }
            ptr32(l).wrapping_add(e)
        } else {
            let at = t.wrapping_add(off).wrapping_add(0x1c);
            let l = ptr32(mem.read(at));
            if l == e { 0 } else { l }
        }
    }

    /// `op_normal_light44_ffx(obj, model, work)` (`0x00a6cf10`): the four
    /// light matrices of a model draw in the work area `w = [work
    /// +0x390]`: `op_ffxlight_to_omdlight(w + 0x80, w + 0xc0, w + 0x100)`,
    /// then `op_matrix_light4_vector(w + 0x40, [model +0x28], [model
    /// +0x2c])` and `op_matrix_light4_vector(w + 0xc0, w + 0xc0, [model
    /// +0x2c])`, all through `call` (`model +0x2c` read before each).
    /// Returns the last call's `eax`.
    pub fn op_normal_light44_ffx<M: Mem>(
        mem: &mut M,
        model: u32,
        work: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let w: u32 = mem.read(work.wrapping_add(0x390));
        let dirs = w.wrapping_add(0xc0);
        go(
            mem,
            0x00a6_90c0,
            &[w.wrapping_add(0x80), dirs, w.wrapping_add(0x100)],
            call,
        );
        let c: u32 = mem.read(model.wrapping_add(0x2c));
        let m: u32 = mem.read(model.wrapping_add(0x28));
        go(mem, 0x00a6_cd80, &[w.wrapping_add(0x40), m, c], call);
        let c: u32 = mem.read(model.wrapping_add(0x2c));
        go(mem, 0x00a6_cd80, &[dirs, dirs, c], call)
    }
}
