//! The effect "op" layer's small services: the scratch stack, matrices,
//! the function ordering table, pad repeat, and the `oef2` cue lists.

// a, b, d, k, m, p, s, t, v, w, x as in the descriptions
#![allow(clippy::many_single_char_names)]

/// A pad's state for `op_pad_set`: the buttons now and last time, the
/// newly pressed ones, the auto-repeat output and its timer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Pad {
    /// Buttons held now.
    pub now: u32,
    /// Buttons held last time.
    pub last: u32,
    /// Buttons pressed since last time.
    pub pressed: u32,
    /// Auto-repeat: the held buttons when it fires, else 0.
    pub repeat: u32,
    /// Frames to the next repeat.
    pub timer: u32,
}

impl Pad {
    /// One update with `input` (16 bits): a change fires the repeat at once
    /// and waits 12 updates; holding fires it every update after that.
    pub fn update(&mut self, input: u32) {
        let old = self.now;
        self.last = old;
        if input == old {
            self.timer = self.timer.wrapping_sub(1);
            if self.timer == 0 {
                self.timer = 1;
                self.repeat = input;
            } else {
                self.repeat = 0;
            }
        } else {
            self.timer = 12;
            self.repeat = input;
        }
        self.now = input;
        self.pressed = (old ^ input) & input;
    }
}

/// The op layer's state where the game keeps it.
pub mod bridge {
    use ffx_bridge::{Global, Mem, Ptr};

    use crate::x87;

    /// `g_OpSprStack`: the scratch stack's `i32` marks.
    pub const SPR_STACK: Global<u32> = Global::new(0x0119_fe10);
    /// `g_OpSprTop`: its top index (`i8`).
    pub const SPR_TOP: Global<i8> = Global::new(0x0233_2ea1);
    /// `g_OpFuncOtOrder`: the ordering table's slot lists (`i8`, -1 free),
    /// 32 from a list's base.
    pub const FUNC_OT_ORDER: Global<i8> = Global::new(0x011a_0960);
    /// `g_OpFuncOt`: its 16-byte slots (function, two words, a `u16`).
    pub const FUNC_OT: Global<[u32; 4]> = Global::new(0x011a_09e0);
    /// `g_OpWotTrigger`: the byte `op_ot_draw` reads.
    pub const WOT_TRIGGER: Global<u8> = Global::new(0x011a_0067);
    /// `g_OpPrayEffect`.
    pub const PRAY_EFFECT: Global<u8> = Global::new(0x011a_0061);
    /// `g_OpDebugMode`: while 0, `op_pad_set` and `op_heap_display_set_par`
    /// do nothing.
    pub const DEBUG_MODE: Global<u8> = Global::new(0x0232_2790);
    /// `g_OpPadInput`: while 0, `op_pad_set` takes no buttons.
    pub const PAD_INPUT: Global<u8> = Global::new(0x0232_1548);
    /// `g_OpPads`: 0x20-byte pad records ([`super::Pad`]).
    pub const PADS: Global<[u32; 8]> = Global::new(0x012a_1370);
    /// `g_OpMdcTransferCues`: eight 16-byte cues, keyed by their first word.
    pub const MDC_TRANSFER_CUES: Global<[u32; 4]> = Global::new(0x011a_11e0);
    /// `g_OpOef2StreamCues`: two stream cue pointers.
    pub const OEF2_STREAM_CUES: Global<u32> = Global::new(0x023c_3648);
    /// `g_OpOef2ClutCues`: a ring of 64 CLUT masks.
    pub const OEF2_CLUT_CUES: Global<u32> = Global::new(0x011a_03a0);
    /// `g_OpOef2ClutCueHead`: the ring's first entry.
    pub const OEF2_CLUT_CUE_HEAD: Global<u16> = Global::new(0x011a_04a0);
    /// `g_OpOef2ClutCueCount`: entries in the ring.
    pub const OEF2_CLUT_CUE_COUNT: Global<u16> = Global::new(0x011a_04a2);
    /// `g_OpOef2ClutOvlCount`: one past the last overlay CLUT kept.
    pub const OEF2_CLUT_OVL_COUNT: Global<u16> = Global::new(0x011a_04b0);
    /// `g_OpOef2ClutOvl`: 64 overlay CLUT masks (0 free).
    pub const OEF2_CLUT_OVL: Global<u32> = Global::new(0x011a_04c0);

    /// `op_spr_alloc(n)` (`0x007e6a00`): pushes a mark `n` past the top
    /// one and returns the top one, the start of the `n` bytes (Ghidra
    /// calls it void; `opp_main` uses the result as its work area).
    pub fn op_spr_alloc(mem: &mut impl Mem, n: u32) -> u32 {
        let top = SPR_TOP.get(mem);
        let stack = SPR_STACK.ptr(mem);
        let i = i32::from(top).cast_unsigned();
        let base = stack.at(i).get(mem);
        stack.at(i.wrapping_add(1)).set(mem, base.wrapping_add(n));
        SPR_TOP.set(mem, top.wrapping_add(1));
        base
    }

    /// `op_spr_free()` (`0x007e6a60`): pops a mark.
    pub fn op_spr_free(mem: &mut impl Mem) {
        let top = SPR_TOP.get(mem);
        SPR_TOP.set(mem, top.wrapping_sub(1));
    }

    /// `op_m_copy(dst, src)` (`0x007e8010`): a 4x4 matrix, word by word
    /// upward (`rep movsd`).
    pub fn op_m_copy(mem: &mut impl Mem, dst: u32, src: u32) {
        for i in 0..16 {
            let v = mem.read::<u32>(src.wrapping_add(4 * i));
            mem.write(dst.wrapping_add(4 * i), v);
        }
    }

    /// `op_unit_m_scale(m, s)` (`0x007ea700`): a scale matrix.
    pub fn op_unit_m_scale(mem: &mut impl Mem, m: u32, s: f32) {
        mem.write(m, [0u32; 16]);
        let s = x87::load(s.to_bits());
        for at in [0x28, 0x14, 0] {
            mem.write(m.wrapping_add(at), s);
        }
        mem.write(m.wrapping_add(0x3c), 1.0f32);
    }

    /// `op_func_ot_set(func, a, b, c, list)` (`0x007eb500`): when list
    /// `list` has a free place, takes the first free slot from `list` on,
    /// enters it there and returns it (-1 when either is full).
    pub fn op_func_ot_set(mem: &mut impl Mem, func: u32, a: u32, b: u32, c: u32, list: u32) -> i32 {
        let order = FUNC_OT_ORDER.ptr(mem).at(list);
        let Some(place) = (0..32).find(|&i| order.at(i).get(mem) < 0) else {
            return -1;
        };
        let slots = FUNC_OT.ptr(mem).at(list);
        let Some(slot) = (0..32u8).find(|&j| slots.at(u32::from(j)).get(mem)[0] == 0) else {
            return -1;
        };
        let s = slots.at(u32::from(slot)).addr;
        mem.write(s, func);
        mem.write(s + 4, a);
        #[allow(clippy::cast_possible_wrap)] // below 32
        order.at(place).set(mem, slot as i8);
        mem.write(s + 8, b);
        #[allow(clippy::cast_possible_truncation)] // the original stores 16 bits
        mem.write(s + 12, c as u16);
        i32::from(slot)
    }

    /// `op_func_ot_free(slot, list)` (`0x007eb4a0`): frees slot `slot`
    /// (below 0x80) of list `list` and takes it out of the list's order.
    pub fn op_func_ot_free(mem: &mut impl Mem, slot: u32, list: u32) {
        if slot > 0x7f {
            return;
        }
        let s = FUNC_OT.ptr(mem).at(slot.wrapping_add(list)).addr;
        mem.write(s, 0u32);
        let order = FUNC_OT_ORDER.ptr(mem).at(list);
        let Some(i) = (0..32).find(|&i| i32::from(order.at(i).get(mem)).cast_unsigned() == slot)
        else {
            return;
        };
        // the bytes after it move down by one, the one past the list too
        for k in i..32 {
            let v = order.at(k + 1).get(mem);
            order.at(k).set(mem, v);
        }
        order.at(31).set(mem, -1);
    }

    /// `op_func_before_ot_set(func, a, b, c)` (`0x007eb370`): list 0.
    pub fn op_func_before_ot_set(mem: &mut impl Mem, func: u32, a: u32, b: u32, c: u32) -> i32 {
        op_func_ot_set(mem, func, a, b, c, 0)
    }

    /// `op_func_after_ot3d_set(func, a, b, c)` (`0x007eb320`): list 0x20.
    pub fn op_func_after_ot3d_set(mem: &mut impl Mem, func: u32, a: u32, b: u32, c: u32) -> i32 {
        op_func_ot_set(mem, func, a, b, c, 0x20)
    }

    /// `op_func_before_wot_set(func, a, b, c)` (`0x007eb3c0`): list 0x40.
    pub fn op_func_before_wot_set(mem: &mut impl Mem, func: u32, a: u32, b: u32, c: u32) -> i32 {
        op_func_ot_set(mem, func, a, b, c, 0x40)
    }

    /// `op_func_before_ot_free(slot)` (`0x007eb350`): list 0.
    pub fn op_func_before_ot_free(mem: &mut impl Mem, slot: u32) {
        op_func_ot_free(mem, slot, 0);
    }

    /// `op_func_after_ot3d_free(slot)` (`0x007eb300`): list 0x20.
    pub fn op_func_after_ot3d_free(mem: &mut impl Mem, slot: u32) {
        op_func_ot_free(mem, slot, 0x20);
    }

    /// `op_func_before_wot_free(slot)` (`0x007eb3a0`): list 0x40.
    pub fn op_func_before_wot_free(mem: &mut impl Mem, slot: u32) {
        op_func_ot_free(mem, slot, 0x40);
    }

    /// `op_heap_display_set_par(heap, flags)` (`0x007ff2b0`): in debug
    /// mode, clears gauge `flags & 3` and shows the ppp heap there.
    pub fn op_heap_display_set_par(mem: &mut impl Mem, heap: u32, flags: u32) {
        if DEBUG_MODE.get(mem) == 0 {
            return;
        }
        crate::opd::bridge::opd_heap_display_clear(mem, flags);
        crate::opd::bridge::opd_heap_display_set_par(mem, heap, flags);
    }

    /// `op_wot_triger(on)` (`0x007ec7d0`).
    pub fn op_wot_triger(mem: &mut impl Mem, on: u32) {
        WOT_TRIGGER.set(mem, u8::from(on != 0));
    }

    /// `op_pray_effect_check()` (`0x007fce10`): -1 while the prayer effect
    /// flag is set, else 0.
    pub fn op_pray_effect_check(mem: &impl Mem) -> i32 {
        -i32::from(PRAY_EFFECT.get(mem) != 0)
    }

    /// `op_pad_set(pad, _, input)` (`0x007e6610`): one update of pad
    /// record `pad` ([`super::Pad::update`]).
    pub fn op_pad_set(mem: &mut impl Mem, pad: u32, input: u32) {
        if DEBUG_MODE.get(mem) == 0 {
            return;
        }
        let input = if PAD_INPUT.get(mem) != 0 {
            input & 0xffff
        } else {
            0
        };
        let at = PADS.ptr(mem).addr.wrapping_add(pad << 5);
        let r = Ptr::<[u32; 5]>::new(at);
        let [now, last, pressed, repeat, timer] = r.get(mem);
        let mut p = super::Pad {
            now,
            last,
            pressed,
            repeat,
            timer,
        };
        p.update(input);
        r.set(mem, [p.now, p.last, p.pressed, p.repeat, p.timer]);
    }

    /// `op_mdc_mem_transfer_cue_free(key)` (`0x00803730`): clears the first
    /// cue with that key.
    pub fn op_mdc_mem_transfer_cue_free(mem: &mut impl Mem, key: u32) {
        let cues = MDC_TRANSFER_CUES.ptr(mem);
        if let Some(i) = (0..8).find(|&i| mem.read::<u32>(cues.at(i).addr) == key) {
            mem.write(cues.at(i).addr, 0u32);
        }
    }

    /// `op_mdc_mem_transfer_cue_free_all()` (`0x00803760`).
    pub fn op_mdc_mem_transfer_cue_free_all(mem: &mut impl Mem) {
        let cues = MDC_TRANSFER_CUES.ptr(mem);
        for i in 0..8 {
            mem.write(cues.at(i).addr, 0u32);
        }
    }

    /// `op_oef2_tex_clut_ovl_clear(mask)` (`0x007e4e40`): frees the
    /// overlay CLUTs that share a bit with `mask`; the count becomes one
    /// past the last one left, which it also returns (in `eax`, as a word).
    pub fn op_oef2_tex_clut_ovl_clear(mem: &mut impl Mem, mask: u32) -> u32 {
        let ovl = OEF2_CLUT_OVL.ptr(mem);
        let mut last: i32 = -1;
        for i in 0..64 {
            let p = ovl.at(i);
            let v = p.get(mem);
            if v == 0 {
                continue;
            }
            if v & mask != 0 {
                p.set(mem, 0);
            } else {
                last = i.cast_signed();
            }
        }
        let count = (last + 1).cast_unsigned();
        #[allow(clippy::cast_possible_truncation)] // 0..=64
        OEF2_CLUT_OVL_COUNT.set(mem, count as u16);
        count
    }

    /// `op_oef2_tex_clut_cue_check(mask)` (`0x007e46d0`): -1 when a queued
    /// CLUT shares a bit with `mask`, else 0.
    pub fn op_oef2_tex_clut_cue_check(mem: &impl Mem, mask: u32) -> i32 {
        let head = u32::from(OEF2_CLUT_CUE_HEAD.get(mem));
        let n = u32::from(OEF2_CLUT_CUE_COUNT.get(mem));
        let ring = OEF2_CLUT_CUES.ptr(mem);
        if (0..n).any(|i| ring.at((i + head) & 0x3f).get(mem) & mask != 0) {
            -1
        } else {
            0
        }
    }

    /// `op_oef2_clut_change(clut, v)` (`0x007e45c0`): sets the CLUT word
    /// (14 bits), marks it changed (`+0xf`) and returns the old word.
    pub fn op_oef2_clut_change(mem: &mut impl Mem, clut: u32, v: u32) -> u32 {
        let old = mem.read::<u16>(clut);
        #[allow(clippy::cast_possible_truncation)] // 14 bits
        mem.write(clut, (v & 0x3fff) as u16);
        mem.write(clut.wrapping_add(0xf), 1u8);
        u32::from(old)
    }

    /// `op_oef2_obj_a_get(base, part, n)` (`0x007e3b40`): entry `n` of the
    /// offset table of part `part & 0xff` (tables of self-relative words).
    pub fn op_oef2_obj_a_get(mem: &impl Mem, base: u32, part: u32, n: u32) -> u32 {
        let table = mem.read::<u32>(base.wrapping_add(4));
        let p = mem.read::<u32>(table.wrapping_add((part & 0xff) * 4).wrapping_add(base));
        let at = n.wrapping_mul(4);
        at.wrapping_add(mem.read::<u32>(at.wrapping_add(p)))
            .wrapping_add(p)
    }

    /// `op_oef2_stream_cue(cue, channel)` (`0x00800390`): queues a stream
    /// cue (type byte 2) on a free channel (`channel & 1`): 0, else -1.
    pub fn op_oef2_stream_cue(mem: &mut impl Mem, cue: u32, channel: u32) -> i32 {
        if mem.read::<u8>(cue) != 2 {
            return -1;
        }
        let slot = OEF2_STREAM_CUES.ptr(mem).at(channel & 1);
        if slot.get(mem) != 0 {
            return -1;
        }
        slot.set(mem, cue);
        0
    }

    /// `op_oef2_a_get(oef, kind, n)` (`0x007e3810`): entry `n & 0xff` of
    /// table `kind` of an `oef2` block: for kind 2 the 0x20-byte record
    /// (0 when its `+0x10` is 0), kind 3 the 0x10-byte record (0 when its
    /// `u16` at `+2` is 0), kind 6 the 0x10-byte record, else the word in
    /// the table (the table offsets at `+4 kind`, from the block).
    pub fn op_oef2_a_get(mem: &impl Mem, oef: u32, kind: u32, n: u32) -> u32 {
        let n = n & 0xff;
        let table = |k: u32| mem.read::<u32>(oef.wrapping_add(4 * k));
        match kind {
            2 => {
                let r = table(2).wrapping_add(n * 0x20).wrapping_add(oef);
                if mem.read::<u32>(r.wrapping_add(0x10)) != 0 {
                    r
                } else {
                    0
                }
            }
            3 => {
                let r = table(3).wrapping_add(n * 0x10).wrapping_add(oef);
                if mem.read::<u16>(r.wrapping_add(2)) != 0 {
                    r
                } else {
                    0
                }
            }
            6 => table(6).wrapping_add(n * 0x10).wrapping_add(oef),
            _ => {
                let t = mem.read::<u32>(oef.wrapping_add(kind.wrapping_mul(4)));
                mem.read(t.wrapping_add(n * 4).wrapping_add(oef))
            }
        }
    }

    /// One table of `op_oef2_set`: for each of the source words (`src..`,
    /// the count re-read at `count` each turn), a nonzero one rebased on
    /// `data` into the destination (`dst`, `stride` bytes apart).
    fn rebase_table(mem: &mut impl Mem, data: u32, src: u32, dst: u32, stride: u32, count: u32) {
        let mut i = 0;
        while i < u32::from(mem.read::<u16>(count)) {
            let v: u32 = mem.read(src.wrapping_add(4 * i));
            if v != 0 {
                mem.write(dst.wrapping_add(stride * i), v.wrapping_add(data));
            }
            i += 1;
        }
    }

    /// `op_oef2_set(oef, data)` (`0x007e3b70`): once per data block (its
    /// `u16` at `+0x40` marks it done), fills the `oef2` block's tables
    /// with the data's offsets made pointers: the counts at `data + 0x42..
    /// 0x52` (`u16`), the offsets at `data + 4 ..0x24` (from `data`), the
    /// destinations at `oef + 4 ..0x24` (from `oef`): plain words, the
    /// `+0x14` of 0x20-byte records, the `+4` of 0x10-byte records, and
    /// the sound records (type byte, `+8` the data, `+0xc` an end: when
    /// the old `+8` is below 0x100000, the type is 1 and `+0xc` > 0, it
    /// becomes `+8` plus the old one; then a negative `+0xc` other than
    /// `0x80000000` takes the `+0xc` of the record its low byte names).
    /// Reads every count again each turn, as the original does.
    #[allow(clippy::many_single_char_names)] // n, i, r, v, p
    pub fn op_oef2_set(mem: &mut impl Mem, oef: u32, data: u32) {
        if mem.read::<u16>(data.wrapping_add(0x40)) != 0 {
            return;
        }
        mem.write(data.wrapping_add(0x40), 1u16);
        for (k, extra, stride, n_at) in [
            (1u32, 0u32, 4u32, 0x42u32),
            (2, 0x14, 0x20, 0x44),
            (3, 4, 0x10, 0x46),
            (4, 0, 4, 0x48),
            (5, 0, 4, 0x4a),
        ] {
            let n: u16 = mem.read(data.wrapping_add(n_at));
            if n != 0 {
                let src = mem.read::<u32>(data.wrapping_add(4 * k)).wrapping_add(data);
                let dst = mem
                    .read::<u32>(oef.wrapping_add(4 * k))
                    .wrapping_add(extra)
                    .wrapping_add(oef);
                rebase_table(mem, data, src, dst, stride, data.wrapping_add(n_at));
            }
        }
        let n: u16 = mem.read(data.wrapping_add(0x4c));
        if n != 0 {
            let src = mem.read::<u32>(data.wrapping_add(0x18)).wrapping_add(data);
            let table = mem.read::<u32>(oef.wrapping_add(0x18)).wrapping_add(oef);
            let mut i = 0;
            while i < u32::from(mem.read::<u16>(data.wrapping_add(0x4c))) {
                let v: u32 = mem.read(src.wrapping_add(4 * i));
                let r = table.wrapping_add(0x10 * i);
                let old: i32 = mem.read(r.wrapping_add(8));
                if v != 0 && old < 0x10_0000 {
                    let p = v.wrapping_add(data);
                    mem.write(r.wrapping_add(8), p);
                    if mem.read::<u8>(r) == 1 && mem.read::<i32>(r.wrapping_add(0xc)) > 0 {
                        mem.write(r.wrapping_add(0xc), p.wrapping_add(old.cast_unsigned()));
                    }
                }
                i += 1;
            }
            let table = mem.read::<u32>(oef.wrapping_add(0x18)).wrapping_add(oef);
            let mut i = 0;
            while i < u32::from(mem.read::<u16>(data.wrapping_add(0x4c))) {
                let r = table.wrapping_add(0x10 * i);
                let end: u32 = mem.read(r.wrapping_add(0xc));
                if mem.read::<u8>(r) == 1 && end.cast_signed() < 0 && end != 0x8000_0000 {
                    let v: u32 = mem.read(table.wrapping_add(0xc + 0x10 * (end & 0xff)));
                    mem.write(r.wrapping_add(0xc), v);
                }
                i += 1;
            }
        }
        for (k, n_at) in [(8u32, 0x50u32), (9, 0x52)] {
            let n: u16 = mem.read(data.wrapping_add(n_at));
            if n != 0 {
                let dst = mem.read::<u32>(oef.wrapping_add(4 * k)).wrapping_add(oef);
                let src = mem.read::<u32>(data.wrapping_add(4 * k)).wrapping_add(data);
                rebase_table(mem, data, src, dst, 4, data.wrapping_add(n_at));
            }
        }
    }

    /// `op_oef2_set2(oef)` (`0x007e3e30`): `op_oef2_set(oef, oef + [oef +
    /// 0x3c])`.
    pub fn op_oef2_set2(mem: &mut impl Mem, oef: u32) {
        let data = mem.read::<u32>(oef.wrapping_add(0x3c)).wrapping_add(oef);
        op_oef2_set(mem, oef, data);
    }

    /// Four words from `src` to `dst`; returns the last (the `eax` of
    /// `VFX_Copy16From` (`0x00710e30`, `(dst, src)`), `VFX_Copy16To`
    /// (`0x00710e70`, `(src, dst)`) and its twin `0x0072f3c0`).
    pub fn copy16(mem: &mut impl Mem, dst: u32, src: u32) -> u32 {
        let mut w = 0;
        for i in 0..4 {
            w = mem.read::<u32>(src.wrapping_add(4 * i));
            mem.write(dst.wrapping_add(4 * i), w);
        }
        w
    }

    /// `VFX_ScaleVec3(dst, v, s)` (`0x00710ea0`): `v`'s x, y, z times `s`
    /// (z with `v`'s element as the memory operand); returns `dst`.
    pub fn vfx_scale_vec3(mem: &mut impl Mem, dst: u32, v: u32, scale: f32) -> u32 {
        use ffx_bridge::x87::Reg;
        let k = Reg::load(scale);
        for i in 0..2 {
            let e: f32 = mem.read(v.wrapping_add(4 * i));
            mem.write(dst.wrapping_add(4 * i), (Reg::load(e) * k).store());
        }
        let z: f32 = mem.read(v.wrapping_add(8));
        mem.write(dst.wrapping_add(8), k.mul_m32(z).store());
        dst
    }

    /// `DJB_hash(s)` (`0x0067b7d0`): Bernstein's hash (`h * 33 + c`, from
    /// 5381, the characters signed) of a C string, the top bit cleared.
    pub fn djb_hash(mem: &impl Mem, s: u32) -> u32 {
        let mut h: u32 = 0x1505;
        let mut p = s;
        loop {
            let c: i8 = mem.read(p);
            if c == 0 {
                return h & 0x7fff_ffff;
            }
            h = h
                .wrapping_mul(33)
                .wrapping_add(i32::from(c).cast_unsigned());
            p = p.wrapping_add(1);
        }
    }

    /// `opu_id_list_add(unit, id)` (`0x00800d60`): adds `id` to the unit's
    /// `i16` list (`+0x1c`, `u16` length `+0x34`, the last place kept)
    /// unless it is there before the first negative entry; it goes there
    /// (or at the end). Returns the index.
    pub fn opu_id_list_add(mem: &mut impl Mem, unit: u32, id: u16) -> u32 {
        let n = u32::from(mem.read::<u16>(unit.wrapping_add(0x34)).wrapping_sub(1));
        let list = mem.read::<u32>(unit.wrapping_add(0x1c));
        let mut i = 0;
        while i < n {
            let v: u16 = mem.read(list.wrapping_add(2 * i));
            if v.cast_signed() < 0 {
                break;
            }
            if v == id {
                return i;
            }
            i += 1;
        }
        mem.write(list.wrapping_add(2 * i), id);
        i
    }

    /// `op_strlen(s)` (`0x007e2250`): the length of a C string, 0 for
    /// null.
    pub fn op_strlen(mem: &impl Mem, s: u32) -> u32 {
        if s == 0 {
            return 0;
        }
        let mut n = 0;
        while mem.read::<u8>(s.wrapping_add(n)) != 0 {
            n += 1;
        }
        n
    }

    /// `lmemcpy(dst, src, n)` (`0x0076b0f0`): `n` words forward; returns
    /// the end of `dst`.
    pub fn lmemcpy(mem: &mut impl Mem, dst: u32, src: u32, n: u32) -> u32 {
        let mut d = dst;
        for i in 0..n {
            let w: u32 = mem.read(src.wrapping_add(4 * i));
            mem.write(d, w);
            d = d.wrapping_add(4);
        }
        d
    }

    /// `g_VfxShapeEnlargeBufferFactor` (`0x00c3a4d8`): multiplies the
    /// shape draws' vertex counts.
    pub const SHAPE_ENLARGE: Global<u32> = Global::new(0x00c3_a4d8);

    /// `setVfxShapeEnlargeBufferFactor(n)` (`0x00717610`): returns `n`
    /// (left in `eax`); `resetVfxShapeEnlargeBufferFactor()`
    /// (`0x00717600`) is `n` = 1 (and leaves `eax` alone).
    pub fn set_vfx_shape_enlarge_buffer_factor(mem: &mut impl Mem, n: u32) -> u32 {
        SHAPE_ENLARGE.set(mem, n);
        n
    }

    /// `calculateVFXSP_ZMat(z)` (`0x00712d00`): 1 when `z` is nonzero.
    #[must_use]
    pub fn calculate_vfx_sp_zmat(z: u32) -> u32 {
        u32::from(z != 0)
    }

    /// `calculateVFXSP_AlphaFunction(f)` (`0x00712c30`): the renderer's
    /// blend flags for a PS2 alpha function (0x41, 0x42, 0x46, 0x48,
    /// 0x88; 0x44 and others none, the others with a debug message).
    #[must_use]
    pub fn calculate_vfx_sp_alpha_function(f: u32) -> u32 {
        match f {
            0x41 => 0x100,
            0x42 => 0x10,
            0x46 => 0x20,
            0x48 => 2,
            0x88 => 0x200,
            _ => 0,
        }
    }

    /// `graphicVFXGetRenderDataByGeometry(g)` (`0x006453b0`): its render
    /// data (`+0x24`), 0 for none.
    pub fn graphic_vfx_get_render_data_by_geometry(mem: &impl Mem, g: u32) -> u32 {
        if g == 0 {
            0
        } else {
            mem.read(g.wrapping_add(0x24))
        }
    }

    /// `g_VfxCurrentType` (`0x00c322aa`, a byte).
    pub const CURRENT_TYPE: Global<u8> = Global::new(0x00c3_22aa);

    /// The axis rotations `op_rt_m_x/y/z(dst, src, angle)`: `angle` in
    /// `g_pppSinTable` steps (4096 a turn, `ftol` of it), the matrix
    /// `src` turned about the axis into `dst` (every row, the translation
    /// too; `src` read first, so they may be the same).
    #[allow(clippy::many_single_char_names)] // i, s, c, m, p, q, u, v
    pub fn op_rt_m(mem: &mut impl Mem, dst: u32, src: u32, angle: f32, axis: crate::mat::Axis) {
        use crate::mat::Axis;
        use ffx_bridge::x87::Reg;
        let i = ffx_bridge::x87::ftol(mem, f64::from(angle));
        let table = crate::ppp::bridge::SIN.ptr(mem);
        let s = ffx_bridge::x87::round_trip(f32::from_bits(table.at(i & 0xfff).get(mem)));
        let c = ffx_bridge::x87::round_trip(f32::from_bits(
            table.at(i.wrapping_add(0x400) & 0xfff).get(mem),
        ));
        let m: [f32; 16] = mem.read(src);
        mem.write(dst, m);
        let (p, q) = match axis {
            Axis::X => (1, 2),
            Axis::Y => (2, 0),
            Axis::Z => (0, 1),
        };
        let (s, c) = (Reg::load(s), Reg::load(c));
        for r in 0..4 {
            let (u, v) = (Reg::load(m[4 * r + p]), Reg::load(m[4 * r + q]));
            // p' = u c - v s, q' = v c + u s
            let pu = (u * c - v * s).store();
            let qv = (v * c + u * s).store();
            #[allow(clippy::cast_possible_truncation)] // below 16
            {
                mem.write(dst.wrapping_add(4 * (4 * r + p) as u32), pu);
                mem.write(dst.wrapping_add(4 * (4 * r + q) as u32), qv);
            }
        }
    }

    /// `op_rt_m_yxz(dst, src, angles, t)` (`0x007e9a30`): `src` turned
    /// about Y, X then Z (angles `[1]`, `[0]`, `[2]`), `t` added to the
    /// translation.
    pub fn op_rt_m_yxz(mem: &mut impl Mem, dst: u32, src: u32, angles: u32, t: u32) {
        use crate::mat::Axis;
        let a: [f32; 3] = mem.read(angles);
        op_rt_m(mem, dst, src, a[1], Axis::Y);
        let a0: f32 = mem.read(angles);
        op_rt_m(mem, dst, dst, a0, Axis::X);
        let a2: f32 = mem.read(angles.wrapping_add(8));
        op_rt_m(mem, dst, dst, a2, Axis::Z);
        for i in 0..3 {
            let at = dst.wrapping_add(0x30 + 4 * i);
            let v = ffx_bridge::x87::Reg::load(mem.read(t.wrapping_add(4 * i)))
                .add_m32(mem.read(at))
                .store();
            mem.write(at, v);
        }
    }

    /// `g_OpMdTagSizes` (`0x00c88aa8`): bytes per element of each of the
    /// eight model packet tag types.
    pub const MD_TAG_SIZES: Global<[u32; 8]> = Global::new(0x00c8_8aa8);

    /// `op_md_packet_set_addr(packet, a, b)` (`0x00a69de0`): unless the
    /// running spell is 0x94, 0x239 or 0xd0, every tag of the packet (from
    /// `+0x20`, until a type byte of 0xff at `+1`) takes `a` at `+8` and `b`
    /// at `+0xc`; a tag is 0x10 bytes plus its count (`i16` at `+2`) times
    /// its type's size.
    pub fn op_md_packet_set_addr(mem: &mut impl Mem, packet: u32, a: u32, b: u32) {
        for id in [0x94, 0x239, 0xd0] {
            if ffx_system::instance::bridge::magic_id(mem) == id {
                return;
            }
        }
        let sizes = MD_TAG_SIZES.ptr(mem).addr;
        let mut t = packet.wrapping_add(0x20);
        while mem.read::<u8>(t.wrapping_add(1)) != 0xff {
            let kind = u32::from(mem.read::<u8>(t.wrapping_add(1)) & 7);
            let n = i32::from(mem.read::<i16>(t.wrapping_add(2)));
            mem.write(t.wrapping_add(8), a);
            mem.write(t.wrapping_add(0xc), b);
            let size: u32 = mem.read(sizes + 4 * kind);
            t = t
                .wrapping_add(0x10)
                .wrapping_add(n.cast_unsigned().wrapping_mul(size));
        }
    }

    /// A sound function (the sound module's), called through `call`.
    fn snd<M: Mem>(
        mem: &mut M,
        f: u32,
        args: &[u32],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = Global::<u32>::new(f).ptr(mem).addr;
        call(mem, f, args)
    }

    /// `SndSepPlay(id, pan, vol)`.
    const SND_SEP_PLAY: u32 = 0x0088_6e50;
    /// `SndStopBattleStream()`.
    const SND_STOP_BATTLE_STREAM: u32 = 0x0088_7760;
    /// `SndStartBattleStream(channel, pan, vol)`.
    const SND_START_BATTLE_STREAM: u32 = 0x0088_75a0;
    /// `SndSepVolumeTranslate(id, vol, 0)`.
    const SND_SEP_VOLUME_TRANSLATE: u32 = 0x0088_6ff0;
    /// `SndSepTransSepOnly(sep, +0xc)`.
    const SND_SEP_TRANS_SEP_ONLY: u32 = 0x0088_6f30;
    /// `SndDataSendSync(0)`.
    const SND_DATA_SEND_SYNC: u32 = 0x0088_5ee0;
    /// `MagicFile_GetMagicId()` (the system module's).
    const MAGIC_FILE_GET_MAGIC_ID: u32 = 0x009d_a3b0;

    /// `g_OpOef2BattleStreamPlaying` (`0x011a0083`): set by
    /// `op_oef2_se_on` when it starts a battle stream.
    pub const BATTLE_STREAM_ON: Global<u8> = Global::new(0x011a_0083);
    /// `g_OpOef2SoundCueHead`/`Count` (`0x023c3640`/`0x023c3642`) and the
    /// 32-entry ring of cues (`0x023c3540`) and their arguments
    /// (`0x023c35c0`).
    pub const SOUND_CUE_HEAD: Global<u16> = Global::new(0x023c_3640);
    /// See [`SOUND_CUE_HEAD`].
    pub const SOUND_CUE_COUNT: Global<u16> = Global::new(0x023c_3642);
    const SOUND_CUES: Global<u32> = Global::new(0x023c_3540);
    const SOUND_CUE_ARGS: Global<u32> = Global::new(0x023c_35c0);
    /// The second word of `g_OpOef2StreamCues` (`0x023c364c`).
    pub const STREAM_CUE2: Global<u32> = Global::new(0x023c_364c);

    /// The separate-sound ID of a kind-1 cue (`+8` points at its record,
    /// the ID at `+8`).
    fn sep_id(mem: &impl Mem, cue: u32) -> u32 {
        let rec = ffx_system::stub::ptr32(mem.read(cue.wrapping_add(8)));
        mem.read(rec.wrapping_add(8))
    }

    /// `op_oef2_se_on(cue, pan, vol)` (`0x007ffe10`): with `vol * 127 /
    /// 256` (rounded toward 0), a kind-1 cue plays its separate sound
    /// (`SndSepPlay`, when its ID is not negative), a kind-2 cue sets
    /// `g_OpOef2BattleStreamPlaying`, stops the battle stream and starts
    /// channel 0 and/or 1 for whichever of `0x023c3520`/`0x023c3524` it is
    /// (all through `call`); returns 1, or 0 for another kind.
    pub fn op_oef2_se_on<M: Mem>(
        mem: &mut M,
        cue: u32,
        pan: u32,
        vol: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let v = scale_volume(vol);
        se_play(mem, cue, pan, v, call)
    }

    /// `vol * 127 / 256`, rounded toward 0 (the cue volume scale).
    fn scale_volume(vol: u32) -> u32 {
        let v = vol.cast_signed().wrapping_mul(0x7f);
        (v.wrapping_add((v >> 31) & 0xff) >> 8).cast_unsigned()
    }

    /// The common part of `op_oef2_se_on` and `op_oef2_se_3d_on`.
    fn se_play<M: Mem>(
        mem: &mut M,
        cue: u32,
        pan: u32,
        v: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        match mem.read::<u8>(cue) {
            1 => {
                let id = sep_id(mem, cue);
                if id.cast_signed() >= 0 {
                    snd(mem, SND_SEP_PLAY, &[id, pan, v], call);
                }
                1
            }
            2 => {
                BATTLE_STREAM_ON.set(mem, 1);
                snd(mem, SND_STOP_BATTLE_STREAM, &[], call);
                for (ch, at) in [(0u32, 0x023c_3520u32), (1, 0x023c_3524)] {
                    if Global::<u32>::new(at).get(mem) == cue {
                        snd(mem, SND_START_BATTLE_STREAM, &[ch, pan, v], call);
                    }
                }
                1
            }
            _ => 0,
        }
    }

    /// `op_oef2_se_volume(cue, vol)` (`0x007ffde0`): a kind-1 cue's
    /// separate sound to `vol` (`SndSepVolumeTranslate(id, vol, 0)` through
    /// `call`); returns its result, or the cue (left in `eax`) for another
    /// kind.
    pub fn op_oef2_se_volume<M: Mem>(
        mem: &mut M,
        cue: u32,
        vol: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        if mem.read::<u8>(cue) != 1 {
            return cue;
        }
        let id = sep_id(mem, cue);
        snd(mem, SND_SEP_VOLUME_TRANSLATE, &[id, vol, 0], call)
    }

    /// `op_oef2_sound_cue_push(cue, arg)` (`0x007ffeb0`): a kind-1 cue
    /// with a record queues (after `SndSepTransSepOnly(record, record
    /// +0xc)` through `call`) at slot `(head + count) & 31` of the ring
    /// with `arg`, the count one more; returns 0, or -1 when the ring is
    /// full (32) or the cue is not one.
    pub fn op_oef2_sound_cue_push<M: Mem>(
        mem: &mut M,
        cue: u32,
        arg: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let count = SOUND_CUE_COUNT.get(mem);
        if count >= 0x20 || mem.read::<u8>(cue) != 1 || mem.read::<u32>(cue.wrapping_add(8)) == 0 {
            return u32::MAX;
        }
        let rec = ffx_system::stub::ptr32(mem.read(cue.wrapping_add(8)));
        let extra: u32 = mem.read(rec.wrapping_add(0xc));
        snd(mem, SND_SEP_TRANS_SEP_ONLY, &[rec, extra], call);
        let slot = u32::from(
            SOUND_CUE_HEAD
                .get(mem)
                .wrapping_add(SOUND_CUE_COUNT.get(mem))
                & 0x1f,
        );
        SOUND_CUES.ptr(mem).at(slot).set(mem, cue);
        SOUND_CUE_ARGS.ptr(mem).at(slot).set(mem, arg);
        let n = SOUND_CUE_COUNT.get(mem).wrapping_add(1);
        SOUND_CUE_COUNT.set(mem, n);
        0
    }

    /// `op_oef2_sound_cue_multi(oef2, from, to, arg)` (`0x008002c0`):
    /// unless the running magic is 0x1fb (`MagicFile_GetMagicId` through
    /// `call`), the kind-1 cues `from ..= to` (at most the block's `u16`
    /// count at `+0x4c`) of table 6 queued with `arg`; returns 0.
    pub fn op_oef2_sound_cue_multi<M: Mem>(
        mem: &mut M,
        oef2: u32,
        from: u32,
        to: u32,
        arg: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        if snd(mem, MAGIC_FILE_GET_MAGIC_ID, &[], call) == 0x1fb {
            return 0;
        }
        let n: u16 = mem.read(oef2.wrapping_add(0x4c));
        if n == 0 {
            return 0;
        }
        #[allow(clippy::cast_possible_truncation)] // u16 arguments
        let end = u32::from((to as u16).wrapping_add(1).min(n));
        #[allow(clippy::cast_possible_truncation)]
        let mut i = u32::from(from as u16);
        while i < end {
            let cue = op_oef2_a_get(mem, oef2, 6, i);
            if cue != 0 && mem.read::<u8>(cue) == 1 {
                op_oef2_sound_cue_push(mem, cue, arg, call);
            }
            i += 1;
        }
        0
    }

    /// `op_oef2_sound_cue_check()` (`0x008000f0`): for `g_OpOef2StreamCues`
    /// then `0x023c364c`: a cue pending (not -1) gives 0x2000/0x2001; -1
    /// waits for `SndDataSendSync(0)` (through `call`; 0x1000/0x1001 while
    /// it is busy) and clears it; then the queued sound cue count.
    pub fn op_oef2_sound_cue_check<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        for (g, k) in [(OEF2_STREAM_CUES.sym, 0u32), (STREAM_CUE2.sym, 1)] {
            let g = Global::<u32>::new(g);
            let v = g.get(mem);
            if v == 0 {
                continue;
            }
            if v != u32::MAX {
                return 0x2000 + k;
            }
            if snd(mem, SND_DATA_SEND_SYNC, &[0], call) != 0 {
                return 0x1000 + k;
            }
            g.set(mem, 0);
        }
        u32::from(SOUND_CUE_COUNT.get(mem))
    }

    /// A character module function, called through `call`.
    fn ch<M: Mem>(
        mem: &mut M,
        f: u32,
        args: &[u32],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        snd(mem, f, args, call)
    }

    /// `op_chara_hsv_size_get(chr)` (`0x00807b20`): 8 bytes per entry of
    /// each of the character's CLUTs (`Ch_ClutGetNb`, `Ch_ClutGetSize`
    /// through `call`), rounded up to 16, plus 0x20; 0 when that is 0.
    pub fn op_chara_hsv_size_get<M: Mem>(
        mem: &mut M,
        chr: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let n = ch(mem, 0x0083_c9a0, &[chr], call).cast_signed();
        if n == 0 {
            return 0;
        }
        let mut total = 0u32;
        let mut k = 0i32;
        while k < n {
            let s = ch(mem, 0x0083_c9f0, &[chr, k.cast_unsigned()], call);
            total = total.wrapping_add(s.wrapping_mul(8));
            k += 1;
        }
        let t = total.wrapping_add(0xf) & !0xf;
        if t == 0 { 0 } else { t + 0x20 }
    }

    /// `op_chara_packet_size_get(chr, mode, visible)` (`0x008037c0`): for
    /// a mimic (`opus_mimic_check`) what `0x0080f580(mode, visible)`
    /// gives; else per model (`Ch_GetNbModel`; with `visible`, hidden
    /// ones, `Ch_ModelGetHide`, are skipped) its effect packet size
    /// (`Ch_ModelCalcSizeEffectPacket`) rounded up to 16 (masked to 28
    /// bits), with mode 2 also `(size / 28) * 12` rounded the same way;
    /// plus 0x10 per model and one more; 0 without models. All through
    /// `call`.
    pub fn op_chara_packet_size_get<M: Mem>(
        mem: &mut M,
        chr: u32,
        mode: u32,
        visible: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        if ch(mem, 0x0080_f630, &[chr], call) != 0 {
            return ch(mem, 0x0080_f580, &[mode, visible], call);
        }
        let models = ch(mem, 0x0083_0410, &[chr], call);
        if models == 0 {
            return 0;
        }
        let mut total = 0u32;
        let mut model = 0i32;
        while model < models.cast_signed() {
            let hidden = ch(mem, 0x0082_add0, &[chr, model.cast_unsigned()], call);
            if hidden == 0 || visible == 0 {
                let size = ch(mem, 0x0083_0450, &[chr, model.cast_unsigned()], call);
                let rounded = size.wrapping_add(0xf) & 0x0fff_fff0;
                let extra = if mode == 2 {
                    #[allow(clippy::cast_possible_truncation)] // the high word
                    let hi = ((u64::from(rounded) * 0x2492_4925) >> 32) as u32;
                    let q = (((rounded - hi) >> 1) + hi) >> 4;
                    (q * 0xc + 0xf) & 0x0fff_fff0
                } else {
                    0
                };
                total = total.wrapping_add(extra).wrapping_add(rounded);
            }
            model += 1;
        }
        models.wrapping_add(1).wrapping_shl(4).wrapping_add(total)
    }

    /// The emulated VU0 registers `op_apply_m_w1` uses: the vector
    /// `0x00c8f940`, the matrix `0x00c8f950`, the result `0x00c8f930`, the
    /// accumulator `0x00c8f790`.
    const MW1_V: u32 = 0x00c8_f940;
    const MW1_M: u32 = 0x00c8_f950;
    const MW1_OUT: u32 = 0x00c8_f930;
    const MW1_ACC: u32 = 0x00c8_f790;
    /// `vf00.w` (1.0).
    const VF00_W: u32 = 0x00c0_a010;

    fn rf(mem: &impl Mem, at: u32) -> f32 {
        mem.read(at)
    }

    /// `op_apply_m_w1(dst, m, v)` (`0x007e79a0`): VU0's
    /// `MULAx/MADDAy/MADDAz/MADDw` with `w` from `vf00` (1.0): `dst` = `m`
    /// times (`v.xyz`, 1). The arguments go to the emulated `a0`..`a2`
    /// (`a3` 0), `v` and `m` through `0x00c8f940`/`0x00c8f950`, the sums
    /// through the accumulator `0x00c8f790`, the result through
    /// `0x00c8f930`; each step is a product added in 53 bits, then stored
    /// as `f32`.
    pub fn op_apply_m_w1(mem: &mut impl Mem, dst: u32, m: u32, v: u32) {
        use x87::{ADD, MUL, Reg};
        for (n, a) in [dst, m, v, 0].into_iter().enumerate() {
            #[allow(clippy::cast_possible_truncation)] // 0..4
            ffx_ps2::ee::bridge::set_func_param(mem, n as u32, a);
        }
        let (vr, mr, out, acc) = (
            Global::<u32>::new(MW1_V).ptr(mem).addr,
            Global::<u32>::new(MW1_M).ptr(mem).addr,
            Global::<u32>::new(MW1_OUT).ptr(mem).addr,
            Global::<u32>::new(MW1_ACC).ptr(mem).addr,
        );
        let vv: [u32; 4] = mem.read(v);
        mem.write(vr, vv);
        let mm: [u32; 16] = mem.read(m);
        mem.write(mr, mm);
        let x = rf(mem, vr);
        for i in 0..4 {
            let r = Reg::load_f32(x).mem(MUL, rf(mem, mr + 4 * i)).store();
            mem.write(acc + 4 * i, r);
        }
        for (k, c) in [(1u32, rf(mem, vr + 4)), (2, rf(mem, vr + 8))] {
            for i in 0..4 {
                let r = Reg::load_f32(c)
                    .mem(MUL, rf(mem, mr + 0x10 * k + 4 * i))
                    .mem(ADD, rf(mem, acc + 4 * i))
                    .store();
                mem.write(acc + 4 * i, r);
            }
        }
        let w = rf(mem, Global::<u32>::new(VF00_W).ptr(mem).addr);
        for i in 0..4 {
            let r = Reg::load_f32(w)
                .mem(MUL, rf(mem, mr + 0x30 + 4 * i))
                .mem(ADD, rf(mem, acc + 4 * i))
                .store();
            mem.write(out + 4 * i, r);
        }
        let o: [u32; 4] = mem.read(out);
        mem.write(dst, o);
    }

    /// `op_length_get(a, b)` (`0x007e7fc0`): the distance from `a` to `b`
    /// (`xyz`): the differences as `f32`, the sum of their squares in 53
    /// bits, `_CIsqrt` (an exact `f64` square root), as `f32`.
    pub fn op_length_get(mem: &impl Mem, a: u32, b: u32) -> f32 {
        let d = [0u32, 4, 8].map(|o| x87::sub(rf(mem, b + o), rf(mem, a + o)));
        let [dx, dy, dz] = d.map(f64::from);
        let s = (dx * dx + dy * dy) + dz * dz;
        #[allow(clippy::cast_possible_truncation)] // fstp m32
        let r = s.sqrt() as f32;
        r
    }

    /// `op_deg_get(x0, y0, x1, y1, len)` (`0x007e7ed0`): the direction
    /// from (`x1`, `y1`) to (`x0`, `y0`) in 4096ths of a turn: `a = y0 -
    /// y1`, `b = x0 - x1` (`f32`); with `len`, `sqrt(a² + b²)` stored
    /// there; `t = atan2(a, b)` (through `atan2`, as `f32`), `t * 2048 /
    /// 3.1415920257568359` (`f32`), `-that - 1024` (`f32`), truncated
    /// (`ftol2_sse`) `& 0xfff`, as a float.
    pub fn op_deg_get<M: Mem>(
        mem: &mut M,
        p: [f32; 4],
        len: u32,
        atan2: &mut impl FnMut(&mut M, f64, f64) -> f64,
    ) -> f32 {
        use x87::{MUL, Reg, SUB};
        let a = x87::sub(p[1], p[3]);
        let b = x87::sub(p[0], p[2]);
        if len != 0 {
            let (fa, fb) = (f64::from(a), f64::from(b));
            #[allow(clippy::cast_possible_truncation)] // fstp m32
            let l = (fa * fa + fb * fb).sqrt() as f32;
            mem.write(len, l);
        }
        let k = |mem: &M, at: u32| mem.read::<f64>(Global::<u32>::new(at).ptr(mem).addr);
        let (c2048, cpi, c1024) = (
            k(mem, 0x00b5_1bc0),
            k(mem, 0x00b5_82f0),
            k(mem, 0x00b5_1ba8),
        );
        #[allow(clippy::cast_possible_truncation)] // fstp m32
        let t = atan2(mem, f64::from(a), f64::from(b)) as f32;
        let t = Reg::load_f32(t)
            .constant(MUL, c2048)
            .constant(|x, y| x / y, cpi)
            .store();
        let t = neg(Reg::load_f32(t)).constant(SUB, c1024).store();
        let i = ffx_bridge::x87::ftol(mem, f64::from(t)) & 0xfff;
        #[allow(clippy::cast_precision_loss)] // below 4096, exact
        let r = i as f32;
        r
    }

    /// `fchs`.
    fn neg(r: x87::Reg) -> x87::Reg {
        match r {
            x87::Reg::Num(x) => x87::Reg::Num(-x),
            x87::Reg::Nan(b) => x87::Reg::Nan(b ^ 0x8000_0000),
        }
    }

    /// `g_Op3dListener` (`0x0113fcd0`): the matrix `op_3d_vol_pan_get`
    /// takes positions into the listener's space with.
    pub const LISTENER: Global<u32> = Global::new(0x0113_fcd0);
    /// The volume and pan curves (`0x023c3668`..`0x023c368c`).
    const VOL_MAX: u32 = 0x023c_3668;
    const VOL_MIN: u32 = 0x023c_366c;
    const PAN_MAX: u32 = 0x023c_3678;
    const PAN_MIN: u32 = 0x023c_367c;
    const VOL_BASE: u32 = 0x023c_3680;
    const VOL_SLOPE: u32 = 0x023c_3684;
    const PAN_SLOPE: u32 = 0x023c_368c;
    /// Where `op_3d_vol_pan_get`'s locals live ([`ffx_bridge::frame`]).
    const VOL_PAN_FRAME: u32 = 0xfe70_0000;

    /// `op_3d_vol_pan_get(pos, vol, out)` (`0x007ffa90`): the position in
    /// listener space (`op_apply_m_w1` with `g_Op3dListener`, into a
    /// local), its distance from the origin (the emulated `r0`,
    /// `op_length_get`) giving `slope * d + base` clamped to the volume
    /// curve's ends, times `vol` / 256 (toward 0, at most 0x7f) into
    /// `*out`; the direction (`op_deg_get(0, 0, x, z)`) folded to a pan
    /// around 64 (`±` up to 1024 4096ths times the pan slope), clamped and
    /// truncated: returned.
    pub fn op_3d_vol_pan_get<M: Mem>(
        mem: &mut M,
        pos: u32,
        vol: u32,
        out: u32,
        atan2: &mut impl FnMut(&mut M, f64, f64) -> f64,
    ) -> u32 {
        let (vol_i, pan) = vol_pan(mem, pos, vol, atan2);
        mem.write(out, vol_i);
        pan
    }

    /// `op_3d_vol_pan_get` returning the volume instead of storing it.
    fn vol_pan<M: Mem>(
        mem: &mut M,
        pos: u32,
        vol: u32,
        atan2: &mut impl FnMut(&mut M, f64, f64) -> f64,
    ) -> (u32, u32) {
        use x87::{ADD, MUL, Reg};
        let listener = LISTENER.ptr(mem).addr;
        let mut f = ffx_bridge::frame::Frame::new(mem, VOL_PAN_FRAME, 0x20);
        let (v, r0) = (f.at(0), f.at(0x10));
        op_apply_m_w1(&mut f, v, listener, pos);
        let zero: [u32; 4] = f.read(ffx_ps2::ee::bridge::GPR.ptr(&f).addr);
        f.write(r0, zero);
        let d = op_length_get(&f, v, r0);
        let (x, z): (f32, f32) = (f.read(v), f.read(v + 8));
        let mem = f.inner();
        let g = |mem: &M, at: u32| mem.read::<f32>(Global::<u32>::new(at).ptr(mem).addr);
        let mut t = Reg::load_f32(g(mem, VOL_SLOPE))
            .mem(MUL, d)
            .mem(ADD, g(mem, VOL_BASE))
            .store();
        let hi = g(mem, VOL_MAX);
        if hi < t {
            t = x87::load_f32(hi);
        }
        let lo = g(mem, VOL_MIN);
        if lo > t {
            t = x87::load_f32(lo);
        }
        let n = ffx_bridge::x87::ftol(mem, f64::from(t))
            .cast_signed()
            .wrapping_mul(vol.cast_signed());
        let vol_i = (n.wrapping_add((n >> 31) & 0xff) >> 8)
            .min(0x7f)
            .cast_unsigned();
        let deg = op_deg_get(
            mem,
            [0.0, 0.0, x87::load_f32(x), x87::load_f32(z)],
            0,
            atan2,
        );
        let k = g(mem, PAN_SLOPE);
        let r = Reg::load_f32(deg);
        let p = if deg > 1024.0 && deg < 3072.0 {
            if deg < 2048.0 {
                Reg::Num(2048.0).reg(x87::SUB, r).mem(MUL, k)
            } else {
                neg(r.constant(x87::SUB, 2048.0).mem(MUL, k))
            }
        } else if deg < 1024.0 {
            r.mem(MUL, k)
        } else {
            neg(Reg::Num(4096.0).reg(x87::SUB, r).mem(MUL, k))
        };
        let p = Reg::load_f32(p.store()).constant(ADD, 64.0).store();
        let mut p = x87::load_f32(p);
        let lo = g(mem, PAN_MIN);
        if lo > p {
            p = x87::load_f32(lo);
        }
        let hi = g(mem, PAN_MAX);
        if hi < p {
            p = x87::load_f32(hi);
        }
        (vol_i, ffx_bridge::x87::ftol(mem, f64::from(p)))
    }

    /// `SndSepPanDirect(id, pan)`.
    const SND_SEP_PAN_DIRECT: u32 = 0x0088_6df0;
    /// `SndSepVolumeDirect(id, vol)`.
    const SND_SEP_VOLUME_DIRECT: u32 = 0x0088_6f60;
    /// `SndIsSepPlay(id)`.
    const SND_IS_SEP_PLAY: u32 = 0x0088_6680;
    /// `SndVagStreamVolumeDirect(vol)`.
    const SND_VAG_STREAM_VOLUME_DIRECT: u32 = 0x0088_7990;
    /// The battle stream's pan (`0x00887970`).
    const SND_STREAM_PAN_DIRECT: u32 = 0x0088_7970;

    /// The pan and volume a 3D cue plays at: 0x40 and `vol * 127 / 256`
    /// without a position, else `op_3d_vol_pan_get`'s.
    fn pan_volume<M: Mem>(
        mem: &mut M,
        pos: u32,
        vol: u32,
        atan2: &mut impl FnMut(&mut M, f64, f64) -> f64,
    ) -> (u32, u32) {
        if pos == 0 {
            (0x40, scale_volume(vol))
        } else {
            let (v, pan) = vol_pan(mem, pos, vol, atan2);
            (pan, v)
        }
    }

    /// `op_oef2_se_3d_on(cue, pos, vol)` (`0x007ffd20`): as
    /// [`op_oef2_se_on`] with the pan and volume of a position.
    pub fn op_oef2_se_3d_on<M: Mem>(
        mem: &mut M,
        cue: u32,
        pos: u32,
        vol: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        atan2: &mut impl FnMut(&mut M, f64, f64) -> f64,
    ) -> u32 {
        let (pan, v) = pan_volume(mem, pos, vol, atan2);
        se_play(mem, cue, pan, v, call)
    }

    /// `op_oef2_se_3d_move(cue, pos, vol)` (`0x007ffc60`): a kind-1 cue
    /// still playing (`SndIsSepPlay`) gets the position's pan and volume
    /// (`SndSepPanDirect`, `SndSepVolumeDirect`); a kind-2 cue sets the
    /// battle stream's (`SndVagStreamVolumeDirect`, then `0x00887970`).
    /// All through `call`; returns 1, or 0 when nothing changed.
    pub fn op_oef2_se_3d_move<M: Mem>(
        mem: &mut M,
        cue: u32,
        pos: u32,
        vol: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        atan2: &mut impl FnMut(&mut M, f64, f64) -> f64,
    ) -> u32 {
        let (pan, v) = pan_volume(mem, pos, vol, atan2);
        let kind: u8 = mem.read(cue);
        if kind == 1 {
            let id = sep_id(mem, cue);
            if id.cast_signed() >= 0 {
                if snd(mem, SND_IS_SEP_PLAY, &[id], call) == 0 {
                    return 0;
                }
                snd(mem, SND_SEP_PAN_DIRECT, &[id, pan], call);
                snd(mem, SND_SEP_VOLUME_DIRECT, &[id, v], call);
                return 1;
            }
        }
        if kind != 2 {
            return 0;
        }
        snd(mem, SND_VAG_STREAM_VOLUME_DIRECT, &[v], call);
        snd(mem, SND_STREAM_PAN_DIRECT, &[pan], call);
        1
    }

    /// The globals the one-line setters store to: `0x01130838`
    /// (`0x007b2b40`), the battle voice ID `0x0112c8f4` (`0x007b2b50`, -1
    /// when none), the battle voice wait `0x0112c8f0` (`0x007b2b60`, read by
    /// `msVoiceCheckStanbyOn/Off`), and the byte `0x011a0066`
    /// (`op_tex_vdisp_load_triger`, `0x007e5210`).
    pub const SETTERS: [u32; 3] = [0x0113_0838, 0x0112_c8f4, 0x0112_c8f0];
    /// See [`SETTERS`].
    pub const TEX_VDISP_LOAD: Global<u8> = Global::new(0x011a_0066);

    /// One of the [`SETTERS`].
    pub fn set_word(mem: &mut impl Mem, i: usize, v: u32) {
        Global::<u32>::new(SETTERS[i]).set(mem, v);
    }

    /// `op_tex_vdisp_load_triger(on)` (`0x007e5210`): the byte at
    /// `0x011a0066`.
    pub fn op_tex_vdisp_load_triger(mem: &mut impl Mem, on: u32) {
        TEX_VDISP_LOAD.set(mem, on.to_le_bytes()[0]);
    }

    /// `op_word_copy(dst, src)` (`0x0075e4b0`, `0x007e6c60`): one word.
    pub fn op_word_copy(mem: &mut impl Mem, dst: u32, src: u32) {
        let w: u32 = mem.read(src);
        mem.write(dst, w);
    }

    /// `op_vec_w_scale(dst, v, s)` (`0x007ecec0`): `dst.w = v.w * s`.
    pub fn op_vec_w_scale(mem: &mut impl Mem, dst: u32, v: u32, s: f32) {
        let w = x87::mul(rf(mem, v.wrapping_add(0xc)), s);
        mem.write(dst.wrapping_add(0xc), w);
    }

    /// `op_add64(dst, src, n)` (`0x00817c30`): the 64-bit `src` plus `n`
    /// (sign-extended) into `dst`.
    pub fn op_add64(mem: &mut impl Mem, dst: u32, src: u32, n: u32) {
        let v: u64 = mem.read(src);
        let r = v.wrapping_add(i64::from(n.cast_signed()).cast_unsigned());
        mem.write(dst, r);
    }

    /// `op_sum_words(p, n)` (`0x0076b1e0`): the sum of `n` words.
    pub fn op_sum_words(mem: &impl Mem, p: u32, n: u32) -> u32 {
        (0..n).fold(0u32, |a, i| a.wrapping_add(mem.read(p.wrapping_add(4 * i))))
    }

    /// `vfx_vec3_scale3(dst, v, sx, sy, sz)` (`0x00728a60`): each of
    /// `v.xyz` times its own scale.
    pub fn vfx_vec3_scale3(mem: &mut impl Mem, dst: u32, v: u32, s: [f32; 3]) {
        for (i, k) in (0u32..).zip(s) {
            let x = x87::mul(rf(mem, v.wrapping_add(4 * i)), k);
            mem.write(dst.wrapping_add(4 * i), x);
        }
    }

    /// `pppVec4Copy(dst, src)` (`0x00750020`): four floats through the
    /// FPU.
    pub fn ppp_vec4_copy(mem: &mut impl Mem, dst: u32, src: u32) {
        for i in 0..4 {
            let x = x87::load_f32(rf(mem, src.wrapping_add(4 * i)));
            mem.write(dst.wrapping_add(4 * i), x);
        }
    }

    /// `op_color_shift_load16(dst, base)` (`0x007eca60`): the `u16` at
    /// `base + g_pppColorConShift`; the shift cleared.
    pub fn op_color_shift_load16(mem: &mut impl Mem, dst: u32, base: u32) {
        let shift = crate::ppp::bridge::COLOR_CON_SHIFT;
        let v: u16 = mem.read(base.wrapping_add(shift.get(mem)));
        mem.write(dst, v);
        shift.set(mem, 0);
    }

    /// `op_oef2_clut_pos_set(rec, pos)` (`0x007e45e0`): a table-3 record's
    /// `u16` becomes `(pos & 0x3fff) + (old & 0x3c)` and its byte `+0xf`
    /// 1; returns the new `u16`.
    pub fn op_oef2_clut_pos_set(mem: &mut impl Mem, rec: u32, pos: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // 16-bit arithmetic
        let v = ((pos & 0x3fff) as u16).wrapping_add(mem.read::<u16>(rec) & 0x3c);
        mem.write(rec, v);
        mem.write(rec.wrapping_add(0xf), 1u8);
        u32::from(v)
    }

    /// `op_oef2_tex_pos_set(rec, pos)` (`0x007e4ff0`): a table-2 record's
    /// low 14 bits become `(pos & 0x3fff) + (old & 0xfc)` (16-bit, or'd
    /// over the rest) and its byte `+0x1f` 1; returns the new low half.
    pub fn op_oef2_tex_pos_set(mem: &mut impl Mem, rec: u32, pos: u32) -> u32 {
        let old: u32 = mem.read(rec);
        #[allow(clippy::cast_possible_truncation)] // 16-bit arithmetic
        let v = ((pos & 0x3fff) as u16).wrapping_add((old as u16) & 0xfc);
        mem.write(rec, (old & 0xffff_c000) | u32::from(v));
        mem.write(rec.wrapping_add(0x1f), 1u8);
        u32::from(v)
    }

    /// `opus_vec3_scale(dst, v, _, _, _, s)` (`0x00808cb0`): `dst.xyz =
    /// v.xyz * s`, `s` the sixth argument.
    pub fn opus_vec3_scale(mem: &mut impl Mem, dst: u32, v: u32, s: f32) {
        for i in 0..3 {
            let x = x87::mul(rf(mem, v.wrapping_add(4 * i)), s);
            mem.write(dst.wrapping_add(4 * i), x);
        }
    }
}
