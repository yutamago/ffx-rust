//! The op layer's model (`op_md_*`) helpers at `0x00a69000..0x00a6d000`:
//! packet copies and counts, the oef2 entries' links and the four-light
//! set-up of the model draws (`docs/formats/vfx.md`, "High range").

#![allow(clippy::many_single_char_names)]

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

    /// Where `g_OpMdTagSizes` (`0x00c88aa8`) holds the bytes per element
    /// of a tag of type `ty` (its low three bits).
    fn tag_size_at(mem: &impl Mem, ty: u8) -> u32 {
        crate::op::bridge::MD_TAG_SIZES
            .ptr(mem)
            .addr
            .wrapping_add(4 * u32::from(ty & 7))
    }

    /// The next tag after `t`: `0x10 + i16 t+2 * g_OpMdTagSizes[ty & 7]`
    /// on.
    fn next_tag(mem: &impl Mem, t: u32, ty: u8) -> u32 {
        let n = i32::from(mem.read::<i16>(t.wrapping_add(2)));
        let size: u32 = mem.read(tag_size_at(mem, ty));
        t.wrapping_add(0x10)
            .wrapping_add(n.cast_unsigned().wrapping_mul(size))
    }

    /// The fields `op_md_tex0_change_range_00a69ca0` rebases in a tag's
    /// `TEX0` (`+8`, 64 bits): `TBP0` (bits 0..14 of `+8`) and `CBP` (bits
    /// 5..19 of `+0xc`).
    fn tbp(mem: &impl Mem, t: u32) -> u16 {
        mem.read::<u16>(t.wrapping_add(8)) & 0x3fff
    }

    #[allow(clippy::cast_possible_truncation)] // the 14-bit field
    fn cbp(mem: &impl Mem, t: u32) -> u16 {
        (mem.read::<u32>(t.wrapping_add(0xc)) >> 5 & 0x3fff) as u16
    }

    /// `op_md_tex0_change_range_00a69ca0(packet, _, lo, hi)`
    /// (`0x00a69ca0`): the packet's textured tags (type byte bit 1) have
    /// their `TEX0` `TBP0` and `CBP` made relative to the smallest of each
    /// over those tags: with `m = min(field - 0x3fff)` (16-bit, signed,
    /// from 0x3fff), a tag whose `TBP0` lies in `lo..=hi` gets `field -
    /// m` (16-bit, 0 when negative) in both, the `TBP0` OR-ed over the
    /// bits above 14 it keeps, the `CBP` into bits 5..19 (more can spill
    /// above them). Tags step by `g_OpMdTagSizes` to the end (type 0xff).
    /// Returns `eax` as left: the end tag (`packet + 0x20` for none).
    pub fn op_md_tex0_change_range(mem: &mut impl Mem, packet: u32, lo: u16, hi: u16) -> u32 {
        let first = packet.wrapping_add(0x20);
        let (mut tmin, mut cmin) = (0x3fffu16, 0x3fffu16);
        let mut t = first;
        let mut ty: u8 = mem.read(t.wrapping_add(1));
        if ty == 0xff {
            return first;
        }
        while ty != 0xff {
            if ty & 2 != 0 {
                let a = tbp(mem, t).wrapping_sub(0x3fff);
                if a.cast_signed() < tmin.cast_signed() {
                    tmin = a;
                }
                let c = cbp(mem, t).wrapping_sub(0x3fff);
                if c.cast_signed() < cmin.cast_signed() {
                    cmin = c;
                }
            }
            t = next_tag(mem, t, ty);
            ty = mem.read(t.wrapping_add(1));
        }
        let rel = |v: u16, m: u16| {
            let d = v.wrapping_sub(m);
            if d.cast_signed() < 0 { 0 } else { u32::from(d) }
        };
        let mut t = first;
        while mem.read::<u8>(t.wrapping_add(1)) != 0xff {
            let ty: u8 = mem.read(t.wrapping_add(1));
            let a = tbp(mem, t);
            if ty & 2 != 0 && (lo..=hi).contains(&a) {
                let c = cbp(mem, t);
                let w: u32 = mem.read(t.wrapping_add(0xc));
                let v: u32 = mem.read(t.wrapping_add(8));
                mem.write(t.wrapping_add(8), v & 0xffff_c000 | rel(a, tmin));
                mem.write(t.wrapping_add(0xc), rel(c, cmin) << 5 | w & 0xfff8_001f);
            }
            t = next_tag(mem, t, ty);
        }
        t
    }

    /// `op_md_vn_index(packet, out, table)` (`0x00a69f10`): the packet's
    /// vertex-normal index lists: `[packet] |= 8`, `packet +0x1c` = `out -
    /// packet`; the `i16 packet +0x12` 64-byte lists of `table` emptied,
    /// every element (0x1c bytes from each tag's `+0x20`, `i16 +2` per tag,
    /// to the end tag 0xff) handed to `op_md_vn_index_sub`; then `u16 out`
    /// = `u16 packet +0x14` and from `out + 0x10` each list's count and
    /// entries (`u16`, a count read as `i16`). The counts are reread from
    /// the packet each time. Returns the end rounded up to 16.
    pub fn op_md_vn_index(mem: &mut impl Mem, packet: u32, out: u32, table: u32) -> u32 {
        let v: u32 = mem.read(packet);
        mem.write(packet, v | 8);
        mem.write(packet.wrapping_add(0x1c), out.wrapping_sub(packet));
        let mut j = 0i32;
        while j < i32::from(mem.read::<i16>(packet.wrapping_add(0x12))) {
            mem.write(
                table.wrapping_add(j.cast_unsigned().wrapping_mul(0x40)),
                0u16,
            );
            j += 1;
        }
        let mut t = packet.wrapping_add(0x20);
        while mem.read::<u8>(t.wrapping_add(1)) != 0xff {
            let mut e = t.wrapping_add(0x20);
            let mut i = 0i32;
            while i < i32::from(mem.read::<i16>(t.wrapping_add(2))) {
                crate::op::bridge::op_md_vn_index_sub(mem, e, table);
                i += 1;
                e = e.wrapping_add(0x1c);
            }
            t = e;
        }
        let v: u16 = mem.read(packet.wrapping_add(0x14));
        mem.write(out, v);
        let mut o = out.wrapping_add(0x10);
        let mut list = table;
        let mut j = 0i32;
        while j < i32::from(mem.read::<i16>(packet.wrapping_add(0x12))) {
            let n: u16 = mem.read(list);
            mem.write(o, n);
            o = o.wrapping_add(2);
            let mut s = list.wrapping_add(2);
            let mut k = n.cast_signed();
            while k > 0 {
                let x: u16 = mem.read(s);
                mem.write(o, x);
                k -= 1;
                o = o.wrapping_add(2);
                s = s.wrapping_add(2);
            }
            j += 1;
            list = list.wrapping_add(0x40);
        }
        o.wrapping_add(0xf) & !0xf
    }

    /// `op_ffxlight_to_omdlight_0279(colours, dirs)` (`0x00a691c0`): as
    /// `op_ffxlight_to_omdlight`, the scene lights in the model renderer's
    /// layout, but with the ambient colour in `colours` row 3: `dirs` rows
    /// 0..2 `Sg_LightGetDir(0..2)` with `xyzw` negated (`fld`/`fchs`, so a
    /// signalling NaN comes out quiet) then `w` 1.0, row 3 the emulated
    /// `r0`; `colours` rows 0..3 `Sg_LightGetColor(0..3)`. Callees through
    /// `call`. Returns 0.
    pub fn op_ffxlight_to_omdlight_0279<M: Mem>(
        mem: &mut M,
        colours: u32,
        dirs: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        for k in 0..3u32 {
            go(mem, 0x0083_61c0, &[k, dirs.wrapping_add(0x10 * k)], call);
        }
        let r0 = mem.rebase(0x00c8_f508);
        for k in 0..4u32 {
            let w: u32 = mem.read(r0.wrapping_add(4 * k));
            mem.write(dirs.wrapping_add(0x30 + 4 * k), w);
        }
        for i in 0..12u32 {
            let at = dirs.wrapping_add(4 * i);
            let x: u32 = mem.read(at);
            mem.write(at, crate::x87::load(x) ^ 0x8000_0000);
        }
        for at in [0x2cu32, 0x1c, 0xc] {
            mem.write(dirs.wrapping_add(at), 1.0f32);
        }
        for k in 0..4u32 {
            go(mem, 0x0083_60f0, &[k, colours.wrapping_add(0x10 * k)], call);
        }
        0
    }

    /// `op_ffxambient_to_omdambient()` (`0x00a68ff0`): the ambient colour
    /// (`Sg_LightGetColor(3)` through `call`, into a local at `frame -
    /// 0x14`) as RGBA bytes: each channel `x 256` (stored as `f32`), 255
    /// above 255, 0 below 0, else truncated (a NaN gives 0). The bytes go
    /// to the local at `frame - 0x18`, and they are the result (`eax`;
    /// the callers drop it).
    pub fn op_ffxambient_to_omdambient<M: Mem>(
        mem: &mut M,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87;
        let local = frame.wrapping_sub(0x14);
        go(mem, 0x0083_60f0, &[3, local], call);
        let mut bytes = [0u8; 4];
        for (k, b) in (0u32..).zip(bytes.iter_mut()) {
            let x: u32 = mem.read(local.wrapping_add(4 * k));
            let v = x87::load(x87::store(x87::load(x) * 256.0));
            *b = if v > 255.0 {
                0xff
            } else if v >= 0.0 {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 0..=255
                let n = v.trunc() as u8;
                n
            } else {
                0
            };
        }
        mem.write(frame.wrapping_sub(0x18), bytes);
        u32::from_le_bytes(bytes)
    }
}
