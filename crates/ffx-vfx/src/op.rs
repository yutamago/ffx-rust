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

    /// `op_func_final_set(func, a, b, c)` (`0x007eb410`): list 0x60.
    pub fn op_func_final_set(mem: &mut impl Mem, func: u32, a: u32, b: u32, c: u32) -> i32 {
        op_func_ot_set(mem, func, a, b, c, 0x60)
    }

    /// `opd_exit_from_ovl()` (`0x007ff8e0`): every texture/CLUT overlay
    /// cleared (`op_oef2_tex_clut_ovl_clear(-1)`); returns its result.
    pub fn opd_exit_from_ovl(mem: &mut impl Mem) -> u32 {
        op_oef2_tex_clut_ovl_clear(mem, u32::MAX)
    }

    /// `op_heap_display_set(a, b, c)` (`0x007ff290`): in debug mode a jump
    /// to `opd_heap_display_set(a, b, c)` (through `call`), returning its
    /// result; otherwise nothing (`eax` as the caller left it: `None`).
    pub fn op_heap_display_set<M: Mem>(
        mem: &mut M,
        args: [u32; 3],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> Option<u32> {
        if DEBUG_MODE.get(mem) == 0 {
            return None;
        }
        let f = Global::<u32>::new(0x00a6_4b20).ptr(mem).addr;
        Some(call(mem, f, &args))
    }

    /// `op_num_vdisp()` (`0x007e6560`): `opd_num_vdisp()` (through `call`)
    /// when [`NUM_SHOW`] is set, and again (a jump) in debug mode. Returns
    /// the last call's result (`None`: `eax` untouched).
    pub fn op_num_vdisp<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> Option<u32> {
        let f = Global::<u32>::new(0x00a5_dfb0).ptr(mem).addr;
        let mut r = None;
        if NUM_SHOW.get(mem) != 0 {
            r = Some(call(mem, f, &[]));
        }
        if DEBUG_MODE.get(mem) != 0 {
            r = Some(call(mem, f, &[]));
        }
        r
    }

    /// `op_num_display(x)` (`0x007e6440`): as [`op_num_vdisp`] with
    /// `opd_num_display(x)`.
    pub fn op_num_display<M: Mem>(
        mem: &mut M,
        x: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> Option<u32> {
        let f = Global::<u32>::new(0x00a5_dd90).ptr(mem).addr;
        let mut r = None;
        if NUM_SHOW.get(mem) != 0 {
            r = Some(call(mem, f, &[x]));
        }
        if DEBUG_MODE.get(mem) != 0 {
            r = Some(call(mem, f, &[x]));
        }
        r
    }

    /// `g_OpSprEnd` (`0x0119ff90`): the end of [`SPR_STACK`]'s block.
    pub const SPR_END: Global<u32> = Global::new(0x0119_ff90);
    /// The byte `0x02332ea0` beside [`SPR_TOP`], cleared with it.
    pub const SPR_FLAG: Global<u8> = Global::new(0x0233_2ea0);

    /// `ops_init_spr()` (`0x007e6c30`): the op scratch stack is 16 KB from
    /// `user_malloc(0x4000)` (through `call`): [`SPR_STACK`] its start,
    /// [`SPR_END`] its end, [`SPR_FLAG`] and [`SPR_TOP`] 0. Returns the end.
    pub fn ops_init_spr<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = Global::<u32>::new(0x0068_7020).ptr(mem).addr;
        let b = call(mem, f, &[0x4000]);
        SPR_STACK.set(mem, b);
        let end = b.wrapping_add(0x4000);
        SPR_END.set(mem, end);
        SPR_TOP.set(mem, 0);
        SPR_FLAG.set(mem, 0);
        end
    }

    /// `op_rt_m_xyz(m, src, angles, t)` (`0x007e9970`): `src` turned about
    /// X, then Y, then Z by the three floats at `angles` (`op_rot_m_x/y/z`)
    /// into `m`, and `t`'s x, y, z added to `m +0x30..+0x38`. Returns `t`.
    pub fn op_rt_m_xyz(mem: &mut impl Mem, m: u32, src: u32, angles: u32, t: u32) -> u32 {
        use crate::mat::Axis;
        for (k, axis) in [Axis::X, Axis::Y, Axis::Z].into_iter().enumerate() {
            #[allow(clippy::cast_possible_truncation)] // 0..3
            let a: u32 = mem.read(angles.wrapping_add(4 * k as u32));
            let from = if k == 0 { src } else { m };
            op_rt_m(mem, m, from, f32::from_bits(x87::load(a)), axis);
        }
        for k in 0..3u32 {
            let v = x87::add(
                mem.read(t.wrapping_add(4 * k)),
                mem.read(m.wrapping_add(0x30 + 4 * k)),
            );
            mem.write(m.wrapping_add(0x30 + 4 * k), v);
        }
        t
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

    /// The axis rotations `op_rot_m_x/y/z(dst, src, angle)`: `angle` in
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

    /// `op_rot_m(dst, src, angles)` (`0x007e9530`): `src` turned about Z,
    /// Y then X (angles `[2]`, `[1]`, `[0]`, each read after the previous
    /// turn, so `angles` may lie in `dst`).
    pub fn op_rot_m(mem: &mut impl Mem, dst: u32, src: u32, angles: u32) {
        use crate::mat::Axis;
        let a2: f32 = mem.read(angles.wrapping_add(8));
        op_rt_m(mem, dst, src, a2, Axis::Z);
        let a1: f32 = mem.read(angles.wrapping_add(4));
        op_rt_m(mem, dst, dst, a1, Axis::Y);
        let a0: f32 = mem.read(angles);
        op_rt_m(mem, dst, dst, a0, Axis::X);
    }

    /// `op_rt_m(dst, src, angles, t)` (`0x007e9850`): [`op_rot_m`], then
    /// `t` added to the translation.
    pub fn op_rt_m_zyx(mem: &mut impl Mem, dst: u32, src: u32, angles: u32, t: u32) {
        op_rot_m(mem, dst, src, angles);
        for i in 0..3 {
            let at = dst.wrapping_add(0x30 + 4 * i);
            let v = ffx_bridge::x87::Reg::load(mem.read(t.wrapping_add(4 * i)))
                .add_m32(mem.read(at))
                .store();
            mem.write(at, v);
        }
    }

    /// `opus_prog_off(list, no)` (`0x008014c0`): unit `no` (16 bits) of the
    /// list (the battle's list, `0x012a12fc + 0x300`, when there is one)
    /// leaves whichever index list holds it (`+0x14`, `+0x10`, `+0x18`;
    /// later entries move down) and is shut down: its first 16 bytes
    /// cleared; its buffer (`+0xbc`) handed to the list's handler
    /// (`[+0x50] +0x54`) for draw kind 3, else freed (`op_free` on the list
    /// heap `+0xa8`) after the kind's render data are destroyed
    /// (`graphicVFXDestroy`: kind 5 the buffer's instance, 0xa and 0x11
    /// one per buffer entry (`u16 +4`), 9 one per shape of `+0xa8`); then
    /// the trail instances (`+0x17` of them for type 0x10 at `+0x16`),
    /// kinds 1 and 2's own instance, and instance 0x15; the list's unit
    /// count (`u16 +0x38`) goes down. All callees through `call`.
    pub fn opus_prog_off<M: Mem>(
        mem: &mut M,
        list: u32,
        no: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let battle: u32 = mem.read(Global::<u32>::new(0x012a_12fc).ptr(mem).addr);
        let l = if battle == 0 {
            list
        } else {
            battle.wrapping_add(0x300)
        };
        #[allow(clippy::cast_possible_truncation)] // the 16-bit argument
        let want = no as u16;
        let mut hit = None;
        'lists: for at in [0x14u32, 0x10, 0x18] {
            let mut e: u32 = mem.read(l.wrapping_add(at));
            loop {
                let v: u16 = mem.read(e);
                if v & 0x8000 != 0 {
                    break;
                }
                if v == want {
                    hit = Some(e);
                    break 'lists;
                }
                e = e.wrapping_add(2);
            }
        }
        let Some(mut e) = hit else { return };
        loop {
            let v: u16 = mem.read(e.wrapping_add(2));
            if v & 0x8000 != 0 {
                break;
            }
            mem.write(e, v);
            e = e.wrapping_add(2);
        }
        mem.write(e, 0xffffu16);
        let destroy = Global::<u32>::new(0x0064_5280).ptr(mem).addr;
        let unit = (i32::from(want.cast_signed()) << 8)
            .cast_unsigned()
            .wrapping_add(mem.read::<u32>(l.wrapping_add(0x20)));
        mem.write(unit, [0u32; 4]);
        let buf: u32 = mem.read(unit.wrapping_add(0xbc));
        let sign = |x: u32| (x.cast_signed() >> 31).cast_unsigned();
        if buf != 0 {
            match mem.read::<u8>(unit.wrapping_add(0xbb)) {
                3 => {
                    let vt: u32 = mem.read(l.wrapping_add(0x50));
                    let f: u32 = mem.read(vt.wrapping_add(0x54));
                    call(mem, f, &[unit]);
                }
                k => {
                    match k {
                        5 => {
                            call(mem, destroy, &[buf, sign(buf)]);
                        }
                        0xa | 0x11 => {
                            let n = u32::from(mem.read::<u16>(buf.wrapping_add(4)));
                            for i in 0..n {
                                call(mem, destroy, &[i, unit]);
                            }
                        }
                        9 => {
                            let d: u32 = mem.read(unit.wrapping_add(0xa8));
                            let mut i = 0u32;
                            while i < u32::from(mem.read::<u16>(d)) {
                                let id = d.wrapping_add(0x20 + 0x50 * i);
                                let get = Global::<u32>::new(0x0064_53d0).ptr(mem).addr;
                                if call(mem, get, &[0, id]) != 0 {
                                    call(mem, destroy, &[0, id]);
                                }
                                i += 1;
                            }
                        }
                        _ => {}
                    }
                    let heap: u32 = mem.read(l.wrapping_add(0xa8));
                    let b: u32 = mem.read(unit.wrapping_add(0xbc));
                    let free = Global::<u32>::new(0x007f_f1d0).ptr(mem).addr;
                    call(mem, free, &[heap, b]);
                }
            }
            mem.write(unit.wrapping_add(0xbc), 0u32);
        }
        if mem.read::<u8>(unit.wrapping_add(0x16)) == 0x10 {
            let mut i = 0u32;
            while i < u32::from(mem.read::<u8>(unit.wrapping_add(0x17))) {
                call(mem, destroy, &[i, unit]);
                i += 1;
            }
        }
        for k in [1u8, 2] {
            if mem.read::<u8>(unit.wrapping_add(0xbb)) == k {
                call(mem, destroy, &[0, unit]);
            }
        }
        call(mem, destroy, &[0x15, unit]);
        let n: u16 = mem.read(l.wrapping_add(0x38));
        mem.write(l.wrapping_add(0x38), n.wrapping_sub(1));
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

    /// The tag after `t` (as in [`op_md_packet_set_addr`]): 0x10 bytes
    /// plus its count (`i16 +2`) times its type's size (`i8 +1` & 7).
    fn md_next_tag(mem: &impl Mem, t: u32) -> u32 {
        let kind = u32::from(mem.read::<u8>(t.wrapping_add(1)) & 7);
        let n = i32::from(mem.read::<i16>(t.wrapping_add(2)));
        let size: u32 = mem.read(MD_TAG_SIZES.ptr(mem).addr + 4 * kind);
        t.wrapping_add(0x10)
            .wrapping_add(n.cast_unsigned().wrapping_mul(size))
    }

    /// A tag's `u` (`+8 & 0x3fff`) and `v` (`+0xc >> 5 & 0x3fff`).
    #[allow(clippy::cast_possible_truncation)] // 14-bit fields
    fn md_tag_uv(mem: &impl Mem, t: u32) -> (u16, u16) {
        let u = mem.read::<u16>(t.wrapping_add(8)) & 0x3fff;
        let v = (mem.read::<u32>(t.wrapping_add(0xc)) >> 5 & 0x3fff) as u16;
        (u, v)
    }

    /// `op_md_tex0_change_limit(packet, tex0, lo, hi)` (`0x00a69ca0`; the
    /// `tex0` argument is not read): the texture coordinates of the tags
    /// with type bit 2, the 14-bit `u` in `+8` and `v` in `+0xc >> 5`,
    /// moved so that the smallest becomes 0x3fff. First the lowest `u -
    /// 0x3fff` and `v - 0x3fff` (`i16`, starting at 0x3fff) over all those
    /// tags; then each with `lo <= u <= hi` (`u16`s) gets `u - low_u` and
    /// `v - low_v` (16-bit, 0 when negative) back into its fields, the
    /// other bits kept (`+8 & 0xffffc000`, `+0xc & 0xfff8001f`; a result
    /// over 0x3fff spills into the bits above, as in the original). Returns
    /// the end tag (type 0xff), as left in `eax`.
    pub fn op_md_tex0_change_limit(mem: &mut impl Mem, packet: u32, lo: u32, hi: u32) -> u32 {
        let first = packet.wrapping_add(0x20);
        let (mut low_u, mut low_v) = (0x3fffu16, 0x3fffu16);
        let mut t = first;
        loop {
            let kind = mem.read::<u8>(t.wrapping_add(1));
            if kind == 0xff {
                break;
            }
            if kind & 2 != 0 {
                let (u, v) = md_tag_uv(mem, t);
                let (du, dv) = (u.wrapping_sub(0x3fff), v.wrapping_sub(0x3fff));
                if du.cast_signed() < low_u.cast_signed() {
                    low_u = du;
                }
                if dv.cast_signed() < low_v.cast_signed() {
                    low_v = dv;
                }
            }
            t = md_next_tag(mem, t);
        }
        #[allow(clippy::cast_possible_truncation)] // u16 arguments
        let (lo, hi) = (lo as u16, hi as u16);
        let clamp = |d: u16| if d.cast_signed() < 0 { 0 } else { u32::from(d) };
        let mut t = first;
        loop {
            let kind = mem.read::<u8>(t.wrapping_add(1));
            if kind == 0xff {
                return t;
            }
            let (u, v) = md_tag_uv(mem, t);
            if kind & 2 != 0 && lo <= u && u <= hi {
                let c: u32 = mem.read(t.wrapping_add(0xc));
                let a: u32 = mem.read(t.wrapping_add(8));
                mem.write(
                    t.wrapping_add(8),
                    a & 0xffff_c000 | clamp(u.wrapping_sub(low_u)),
                );
                mem.write(
                    t.wrapping_add(0xc),
                    clamp(v.wrapping_sub(low_v)) << 5 | c & 0xfff8_001f,
                );
            }
            t = md_next_tag(mem, t);
        }
    }

    /// `op_md_tex0_change(packet, tex0)` (`0x00a69c80`):
    /// [`op_md_tex0_change_limit`] over every `u` (0..=0x4000); returns its
    /// `eax`.
    pub fn op_md_tex0_change(mem: &mut impl Mem, packet: u32) -> u32 {
        op_md_tex0_change_limit(mem, packet, 0, 0x4000)
    }

    /// `0x00a698c0(packet)`: the size of the packet's vertex block after
    /// its tags: walking the tags from `+0x20` (as in
    /// [`op_md_packet_set_addr`]), 0 at a type nibble (`+1` & 0xf0) of 0x10
    /// or 0x20; at 0xf0 (the end), `(6 (i16 +0x12 + i16 +0x14) + 0x1f +
    /// end - packet) & 0x7ffffff0`, `end` the end tag.
    pub fn op_md_packet_size(mem: &impl Mem, packet: u32) -> u32 {
        let sizes = MD_TAG_SIZES.ptr(mem).addr;
        let mut t = packet.wrapping_add(0x20);
        loop {
            let b: u8 = mem.read(t.wrapping_add(1));
            match b & 0xf0 {
                0x10 | 0x20 => return 0,
                0xf0 => {
                    let a = i32::from(mem.read::<i16>(packet.wrapping_add(0x12)));
                    let c = i32::from(mem.read::<i16>(packet.wrapping_add(0x14)));
                    let n = c.wrapping_add(a).wrapping_mul(6).cast_unsigned();
                    return n.wrapping_sub(packet).wrapping_add(0x1f).wrapping_add(t) & 0x7fff_fff0;
                }
                _ => {}
            }
            let n = i32::from(mem.read::<i16>(t.wrapping_add(2)));
            let size: u32 = mem.read(sizes + 4 * u32::from(b & 7));
            t = t
                .wrapping_add(n.cast_unsigned().wrapping_mul(size))
                .wrapping_add(0x10);
        }
    }

    /// `0x007e1cc0(src, out, max)`: the next line of the op script reader
    /// `src` (bytes `[src]`, position `+0xc`, end `+0x10`) into `out`, at
    /// most `max - 1` bytes and a NUL: it stops at 0, 0xff, CR or LF (a
    /// byte of 0x80 or more takes the next byte with it, which only 0 and
    /// 0xff stop) or at the end (the position one further), then steps the
    /// position back one when it is positive. Returns the bytes stored.
    pub fn op_read_line(mem: &mut impl Mem, src: u32, out: u32, max: u32) -> u32 {
        let lim = max.cast_signed().wrapping_sub(1);
        let mut out = out;
        let mut n = 0i32;
        let pos_at = src.wrapping_add(0xc);
        let mut pos: i32;
        'read: loop {
            pos = mem.read(pos_at);
            if pos >= mem.read::<i32>(src.wrapping_add(0x10)) {
                break 'read;
            }
            let base: u32 = mem.read(src);
            let c: u8 = mem.read(base.wrapping_add(pos.cast_unsigned()));
            pos = pos.wrapping_add(1);
            mem.write(pos_at, pos);
            if c == 0xff || c == 0 {
                return finish(mem, pos_at, pos, out, n);
            }
            if c < 0x80 {
                if c == 0xd || c == 0xa {
                    return finish(mem, pos_at, pos, out, n);
                }
            } else {
                if n < lim {
                    mem.write(out, c);
                    out = out.wrapping_add(1);
                    n += 1;
                }
                pos = mem.read(pos_at);
                if pos >= mem.read::<i32>(src.wrapping_add(0x10)) {
                    break 'read;
                }
                let base: u32 = mem.read(src);
                let c2: u8 = mem.read(base.wrapping_add(pos.cast_unsigned()));
                pos = pos.wrapping_add(1);
                mem.write(pos_at, pos);
                if c2 == 0xff || c2 == 0 {
                    return finish(mem, pos_at, pos, out, n);
                }
                if n < lim {
                    mem.write(out, c2);
                    out = out.wrapping_add(1);
                    n += 1;
                }
                continue;
            }
            if n < lim {
                mem.write(out, c);
                out = out.wrapping_add(1);
                n += 1;
            }
        }
        let p: i32 = mem.read(pos_at);
        mem.write(pos_at, p.wrapping_add(1));
        let pos: i32 = mem.read(pos_at);
        finish(mem, pos_at, pos, out, n)
    }

    /// The op script reader's line counter (`0x0113fc80`).
    pub const LINE_COUNT: Global<u32> = Global::new(0x0113_fc80);

    /// `0x007e1cc0`'s sibling `0x007e2660(src, lines)`: the length of the
    /// reader's next line (bytes before an LF; a byte of 0x80 or more
    /// counts with the next), the position past the LF. Without an LF (0,
    /// 0xff or the end, which is read once and passed by one) the result is
    /// -1 and the position steps back one when positive. Either way
    /// `*lines` (when not null) and [`LINE_COUNT`] go up by one.
    pub fn op_line_length(mem: &mut impl Mem, src: u32, lines: u32) -> u32 {
        let pos_at = src.wrapping_add(0xc);
        let end: i32 = mem.read(src.wrapping_add(0x10));
        let mut n = 0u32;
        let ok = 'read: loop {
            let mut pos: i32 = mem.read(pos_at);
            if pos >= end {
                mem.write(pos_at, pos.wrapping_add(1));
                break 'read false;
            }
            let base: u32 = mem.read(src);
            pos = pos.wrapping_add(1);
            let c: u8 = mem.read(base.wrapping_add(pos.cast_unsigned()).wrapping_sub(1));
            mem.write(pos_at, pos);
            match c {
                0 | 0xff => break 'read false,
                0xa => break 'read true,
                0x01..0x80 => n = n.wrapping_add(1),
                _ => {
                    n = n.wrapping_add(1);
                    if pos >= end {
                        mem.write(pos_at, pos.wrapping_add(1));
                        break 'read false;
                    }
                    let c: u8 = mem.read(base.wrapping_add(pos.cast_unsigned()));
                    mem.write(pos_at, pos.wrapping_add(1));
                    if c == 0 || c == 0xff {
                        break 'read false;
                    }
                    n = n.wrapping_add(1);
                }
            }
        };
        if !ok {
            n = u32::MAX;
            let pos: i32 = mem.read(pos_at);
            if pos > 0 {
                mem.write(pos_at, pos - 1);
            }
        }
        if lines != 0 {
            let l: u32 = mem.read(lines);
            mem.write(lines, l.wrapping_add(1));
        }
        let c = LINE_COUNT.get(mem);
        LINE_COUNT.set(mem, c.wrapping_add(1));
        n
    }

    /// `0x007e5680()`: the CLUT tables cleared: [`OEF2_CLUT_OVL`] (64
    /// words), `0x011a01a0`.., `0x011a02a0`.. and [`OEF2_CLUT_CUES`] (64
    /// words each), the cue head and count (one dword at `0x011a04a0`) and
    /// [`OEF2_CLUT_OVL_COUNT`]. Returns 0 (left in `eax`).
    pub fn op_oef2_tex_clut_init(mem: &mut impl Mem) -> u32 {
        let ovl = OEF2_CLUT_OVL.ptr(mem).addr;
        for i in 0..0x40u32 {
            mem.write(ovl + 4 * i, 0u32);
        }
        let a = Global::<u32>::new(0x011a_01a0).ptr(mem).addr;
        let b = Global::<u32>::new(0x011a_02a0).ptr(mem).addr;
        let c = OEF2_CLUT_CUES.ptr(mem).addr;
        for i in 0..0x40u32 {
            mem.write(a + 4 * i, 0u32);
            mem.write(b + 4 * i, 0u32);
            mem.write(c + 4 * i, 0u32);
        }
        let head = OEF2_CLUT_CUE_HEAD.ptr(mem).addr;
        mem.write(head, 0u32);
        OEF2_CLUT_OVL_COUNT.set(mem, 0);
        0
    }

    /// `op_read_line`'s end: the position back one when positive, the NUL.
    fn finish(mem: &mut impl Mem, pos_at: u32, pos: i32, out: u32, n: i32) -> u32 {
        if pos > 0 {
            mem.write(pos_at, pos - 1);
        }
        mem.write(out, 0u8);
        n.cast_unsigned()
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
    /// `g_OpOvlEffectOn` (`0x011a006a`): set by `op_ovl_effect_init`,
    /// cleared by `op_ovl_effect_free`; the mdc transfer cues run only
    /// while it is set.
    pub const OVL_EFFECT_ON: Global<u8> = Global::new(0x011a_006a);
    /// `g_OpusCameraLimit` (`0x011a006c`): 0x1000 plus the camera command
    /// 9's depth while the camera limit is on, 0 when off.
    pub const CAMERA_LIMIT: Global<u16> = Global::new(0x011a_006c);

    /// `op_ovl_effect_init()` (`0x007fcdd0`): every texture/CLUT overlay
    /// and oef2 sound slot freed (`op_oef2_tex_clut_ovl_clear(0xffff)`,
    /// `op_oef2_sep_free_all(0xffff)`), the mdc transfer cues dropped,
    /// battle clipping turned off (`opus_clip_off_all_in_battle`), then
    /// [`OVL_EFFECT_ON`] = 1, [`CAMERA_LIMIT`] = 0 and
    /// [`BATTLE_STREAM_ON`] = 0. Its `dbgPrintf` is a bare `ret`. Returns
    /// `eax` as left: 0.
    pub fn op_ovl_effect_init<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        op_oef2_tex_clut_ovl_clear(mem, 0xffff);
        op_oef2_sep_free_all(mem, 0xffff, call);
        op_mdc_mem_transfer_cue_free_all(mem);
        crate::opus::bridge::opus_clip_off_all_in_battle(mem, call);
        OVL_EFFECT_ON.set(mem, 1);
        CAMERA_LIMIT.set(mem, 0);
        BATTLE_STREAM_ON.set(mem, 0);
        0
    }

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

    /// `op_oef2_sound_init_00800440()` (`0x00800440`, from the op layer's
    /// set-up `0x007ff960`): the 32-entry cue ring and its arguments, the
    /// 64 words from `0x023c36a0` (two 32-slot tables, the second
    /// `g_OpOef2Seps`), the ring head and count, both stream cues and the
    /// byte `0x023c3644` cleared; the volume and pan curve constants
    /// (`0x023c3660..0x023c368f`) set from the image's floats, each copied
    /// as it is (`fld`/`fstp` of a normal `f32` keeps its bits). Returns 0
    /// (left in `eax`).
    pub fn op_oef2_sound_init(mem: &mut impl Mem) -> u32 {
        let cues = SOUND_CUES.ptr(mem).addr;
        let args = SOUND_CUE_ARGS.ptr(mem).addr;
        let slots = Global::<u32>::new(0x023c_36a0).ptr(mem).addr;
        for i in 0..32u32 {
            mem.write(cues + 4 * i, 0u32);
            mem.write(args + 4 * i, 0u32);
            for k in 0..2u32 {
                mem.write(slots + 8 * i + 4 * k, 0u32);
                mem.write(slots + 0x100 + 8 * i + 4 * k, 0u32);
            }
        }
        for a in [0x023c_3640u32, 0x023c_364c, 0x023c_3648] {
            let at = Global::<u32>::new(a).ptr(mem).addr;
            mem.write(at, 0u32);
        }
        let at = Global::<u8>::new(0x023c_3644).ptr(mem).addr;
        mem.write(at, 0u8);
        // (destination, the image's f32 constant)
        for (to, from) in [
            (0x023c_3660u32, 0x00b4_1d84u32),
            (0x023c_3664, 0x00b4_1d9c),
            (0x023c_3668, 0x00b4_f5e4),
            (0x023c_366c, 0x00b4_3db8),
            (0x023c_3670, 0x00b5_83dc),
            (0x023c_3674, 0x00b5_870c),
            (0x023c_3678, 0x00b4_3db8),
            (0x023c_367c, 0x00b4_1d84),
            (0x023c_3688, 0x00b5_83dc),
            (0x023c_3684, 0x00b5_8714),
            (0x023c_3680, 0x00b5_8708),
            (0x023c_368c, 0x00b5_8704),
        ] {
            let v = Global::<u32>::new(from).get(mem);
            Global::<u32>::new(to).set(mem, v);
        }
        0
    }

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

    /// The emulated VU0 registers `op_mul_m` uses besides those of
    /// `op_apply_m_w1`: the right matrix and the result at `0x00c8f910`.
    const MULM_C: u32 = 0x00c8_f910;

    /// `op_mul_m(dst, b, c)` (`0x007e83b0`): VU0's `MULAx`, `MADDAy`, `MADDAz`
    /// and `MADDw` for each row of `c`: row `r` of `dst` is `c[r].x * b[0] +
    /// c[r].y * b[1] + c[r].z * b[2] + c[r].w * b[3]` (so `dst` = `c` times
    /// `b` for row vectors). The arguments go to `a0`..`a2` (`a3` 0), `b`
    /// to `0x00c8f950`, `c` and then each result row to `0x00c8f910`, the
    /// sums through the accumulator `0x00c8f790` (each an `f32` after a
    /// product added in 53 bits); the result is copied to `dst` last, so
    /// `dst` may be `b` or `c`.
    pub fn op_mul_m(mem: &mut impl Mem, dst: u32, b: u32, c: u32) {
        use x87::{ADD, MUL, Reg};
        for (n, a) in [dst, b, c, 0].into_iter().enumerate() {
            #[allow(clippy::cast_possible_truncation)] // 0..4
            ffx_ps2::ee::bridge::set_func_param(mem, n as u32, a);
        }
        let (br, cr, acc) = (
            Global::<u32>::new(MW1_M).ptr(mem).addr,
            Global::<u32>::new(MULM_C).ptr(mem).addr,
            Global::<u32>::new(MW1_ACC).ptr(mem).addr,
        );
        let bb: [u32; 16] = mem.read(b);
        mem.write(br, bb);
        let cc: [u32; 16] = mem.read(c);
        mem.write(cr, cc);
        for r in 0..4u32 {
            let row: [f32; 4] = mem.read(cr + 0x10 * r);
            for i in 0..4 {
                let v = Reg::load_f32(row[0]).mem(MUL, rf(mem, br + 4 * i)).store();
                mem.write(acc + 4 * i, v);
            }
            for k in 1..3u32 {
                for i in 0..4 {
                    let v = Reg::load_f32(row[k as usize])
                        .mem(MUL, rf(mem, br + 0x10 * k + 4 * i))
                        .mem(ADD, rf(mem, acc + 4 * i))
                        .store();
                    mem.write(acc + 4 * i, v);
                }
            }
            for i in 0..4 {
                let v = Reg::load_f32(row[3])
                    .mem(MUL, rf(mem, br + 0x30 + 4 * i))
                    .mem(ADD, rf(mem, acc + 4 * i))
                    .store();
                mem.write(cr + 0x10 * r + 4 * i, v);
            }
        }
        let o: [u32; 16] = mem.read(cr);
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

    /// `op_oef2_clut_change_monster(rec, pos)` (`0x007e45e0`): a table-3 record's
    /// `u16` becomes `(pos & 0x3fff) + (old & 0x3c)` and its byte `+0xf`
    /// 1; returns the new `u16`.
    pub fn op_oef2_clut_change_monster(mem: &mut impl Mem, rec: u32, pos: u32) -> u32 {
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
    /// `op_oef2_sound_wait()` (`0x00800330`): until no stream cue is
    /// pending (a -1 cue waits for `SndDataSendSync(0)` and is then
    /// cleared) and no sound cue is queued, `op_oef2_sound_cue_loop()`
    /// (`0x00800170`); both through `call`. Returns 0.
    pub fn op_oef2_sound_wait<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        loop {
            let mut busy = false;
            for g in [OEF2_STREAM_CUES.sym, STREAM_CUE2.sym] {
                let g = Global::<u32>::new(g);
                let v = g.get(mem);
                if v == 0 {
                    continue;
                }
                if v != u32::MAX || snd(mem, SND_DATA_SEND_SYNC, &[0], call) != 0 {
                    busy = true;
                    break;
                }
                g.set(mem, 0);
            }
            if !busy && SOUND_CUE_COUNT.get(mem) == 0 {
                return 0;
            }
            snd(mem, 0x0080_0170, &[], call);
        }
    }

    /// `op_xyz_scale(dst, src, s)` (`0x007ea850`): VU0's `MUL.xyz` by a
    /// scalar: `src` and `s` through `0x00c8f970`/`0x00c8f980` (the
    /// arguments in `a0`..`a2`, `a3` 0), `dst.xyz` = `src.xyz * s` (`w`
    /// untouched). Returns the last word written (`z`, left in `eax`).
    pub fn op_xyz_scale(mem: &mut impl Mem, dst: u32, src: u32, s: u32) -> u32 {
        use x87::{MUL, Reg};
        for (n, a) in [dst, src, s, 0].into_iter().enumerate() {
            #[allow(clippy::cast_possible_truncation)] // 0..4
            ffx_ps2::ee::bridge::set_func_param(mem, n as u32, a);
        }
        let vr = Global::<u32>::new(0x00c8_f970).ptr(mem).addr;
        let sr = Global::<u32>::new(0x00c8_f980).ptr(mem).addr;
        let v: [u32; 4] = mem.read(src);
        mem.write(vr, v);
        mem.write(sr, x87::load(s));
        let k = Reg::load(s);
        for i in 0..2 {
            let r = Reg::load_f32(rf(mem, vr + 4 * i)).reg(MUL, k).store();
            mem.write(vr + 4 * i, r);
        }
        let r = k.mem(MUL, rf(mem, vr + 8)).store();
        mem.write(vr + 8, r);
        let out: [u32; 3] = mem.read(vr);
        mem.write(dst, out);
        out[2]
    }

    /// `op_heap_display_clear()` (`0x007ff260`): in debug mode,
    /// `opd_heap_display_clear()` (`0x00a649c0`, through `call`).
    pub fn op_heap_display_clear<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        if DEBUG_MODE.get(mem) != 0 {
            let f = Global::<u32>::new(0x00a6_49c0).ptr(mem).addr;
            call(mem, f, &[]);
        }
    }

    /// `g_OpOef2Seps` (`0x023c37a0`): 32 sound-sep slots of (handle, user
    /// mask).
    pub const OEF2_SEPS: Global<[u32; 2]> = Global::new(0x023c_37a0);

    /// `op_oef2_sep_free_all(mask)` (`0x007fff30`): `mask` cleared from
    /// every sep slot's users; a slot left with none is stopped and
    /// cleared (`SndSepStop`, `SndSepClear`, through `call`) and its
    /// handle set to 0. Returns `mask` (left in `eax`).
    pub fn op_oef2_sep_free_all<M: Mem>(
        mem: &mut M,
        mask: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        for i in 0..32 {
            let at = OEF2_SEPS.ptr(mem).at(i).addr;
            let users: u32 = mem.read(at + 4);
            if users & mask == 0 {
                continue;
            }
            mem.write(at + 4, users & !mask);
            if users & !mask == 0 {
                let h: u32 = mem.read(at);
                for f in [0x0088_6eb0u32, 0x0088_6d80] {
                    let f = Global::<u32>::new(f).ptr(mem).addr;
                    call(mem, f, &[h]);
                }
                mem.write(at, 0u32);
            }
        }
        mask
    }

    /// `op_free(heap, p)` (`0x007ff1d0`): frees block `p` of an op heap.
    /// The heap's 16-byte block records go down from `heap` (record `i`
    /// at `heap - 16 * i`: `+0` the block, `+8` and `+0xa` the `i16`
    /// previous and next record, 0 when free); the highest record number
    /// is at `heap - 4`, the count of blocks at `heap - 2`. The record of
    /// `p` (from 2 up) is unlinked and cleared, the count decremented and
    /// the highest number lowered when it was the last; a `p` not found
    /// is reported through `dbgPrintf` (`0x00b586bc`, through `call`).
    pub fn op_free<M: Mem>(
        mem: &mut M,
        heap: u32,
        p: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        if p == 0 {
            return;
        }
        let top = i32::from(mem.read::<i16>(heap.wrapping_sub(4)));
        let rec = |i: i32| heap.wrapping_sub(i.cast_unsigned().wrapping_shl(4));
        let found = (2..=top).find(|&i| {
            mem.read::<u32>(rec(i).wrapping_add(8)) != 0 && mem.read::<u32>(rec(i)) == p
        });
        let Some(i) = found else {
            let f = Global::<u32>::new(0x0062_f500).ptr(mem).addr;
            let msg = Global::<u32>::new(0x00b5_86bc).ptr(mem).addr;
            call(mem, f, &[msg]);
            return;
        };
        if i >= top {
            #[allow(clippy::cast_possible_truncation)] // an i16 count
            mem.write(heap.wrapping_sub(4), (top - 1) as i16);
        }
        let e = rec(i);
        let prev = i32::from(mem.read::<i16>(e.wrapping_add(8)));
        let next: i16 = mem.read(e.wrapping_add(0xa));
        mem.write(rec(prev).wrapping_add(0xa), next);
        if next != 0 {
            #[allow(clippy::cast_possible_truncation)] // the i16 field
            mem.write(rec(i32::from(next)).wrapping_add(8), prev as i16);
        }
        mem.write(e.wrapping_add(8), 0u32);
        let n: u16 = mem.read(heap.wrapping_sub(2));
        mem.write(heap.wrapping_sub(2), n.wrapping_sub(1));
    }

    /// `op_oef2_a_set(oef, kind, i, v)` (`0x007e38a0`): word `i` of the
    /// oef2 table `kind` (at `oef + oef[kind]`) set to `v` when `i` is
    /// below its count (`u16 +0x40 + 2 kind`) rounded up to 4 in the low
    /// byte; kinds 2, 3 and 6 (and an empty table) are left alone.
    /// Returns 1 when it wrote. Kind 8 is logged through `dbgPrintf`
    /// (`0x00b580c8`, `i`, `oef +0x20`; through `call`) first.
    pub fn op_oef2_a_set<M: Mem>(
        mem: &mut M,
        oef: u32,
        kind: u32,
        i: u32,
        v: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        if kind == 8 {
            let f = Global::<u32>::new(0x0062_f500).ptr(mem).addr;
            let msg = Global::<u32>::new(0x00b5_80c8).ptr(mem).addr;
            let w: u32 = mem.read(oef.wrapping_add(0x20));
            call(mem, f, &[msg, i, w]);
        }
        if matches!(kind, 2 | 3 | 6) {
            return 0;
        }
        let n =
            u32::from(mem.read::<u16>(oef.wrapping_add(kind.wrapping_mul(2)).wrapping_add(0x40)));
        if n == 0 || (n.wrapping_add(3) & 0xfc).cast_signed() <= i.cast_signed() {
            return 0;
        }
        let t: u32 = mem.read(oef.wrapping_add(kind.wrapping_mul(4)));
        mem.write(t.wrapping_add(i.wrapping_mul(4)).wrapping_add(oef), v);
        1
    }

    /// `0x02322794`: a byte that also shows `op_num_xy_set`'s numbers.
    pub const NUM_SHOW: Global<u8> = Global::new(0x0232_2794);

    /// `op_num_xy_set(x, y)` (`0x007e65e0`): `opd_num_xy_set(x, y)` (through
    /// `call`) once when `NUM_SHOW` is set and again in debug mode.
    pub fn op_num_xy_set<M: Mem>(
        mem: &mut M,
        x: u32,
        y: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let f = Global::<u32>::new(0x00a5_e2b0).ptr(mem).addr;
        if NUM_SHOW.get(mem) != 0 {
            call(mem, f, &[x, y]);
        }
        if DEBUG_MODE.get(mem) != 0 {
            call(mem, f, &[x, y]);
        }
    }

    /// `op_heap_init(heap, size)` (`0x007ff2d0`): the `size` bytes before
    /// `heap + size` cleared in 16-byte steps from the top (the last step
    /// may start below `heap` by less than 16 when `size` is not a
    /// multiple of 16), then the two first block records set up below the
    /// end (record 1 at `end - 0x10`: block `heap`, end `heap`; record 2
    /// at `end - 0x20` = the emulated r0, as the rest of record 1). Returns
    /// the end (the heap's base for `op_malloc`/`op_free`), or 0 for a
    /// size of 0.
    pub fn op_heap_init(mem: &mut impl Mem, heap: u32, size: u32) -> u32 {
        if size == 0 {
            return 0;
        }
        let mut at = size.wrapping_sub(0x10).cast_signed();
        if at >= 0 {
            loop {
                at = at.wrapping_sub(0x10);
                let a = heap.wrapping_add(at.cast_unsigned()).wrapping_add(0x10);
                mem.write(a, [0u32; 4]);
                if at < 0 {
                    break;
                }
            }
        }
        let r0: [u32; 4] = mem.read(Global::<u32>::new(0x00c8_f508).ptr(mem).addr);
        let end = heap.wrapping_add(size);
        mem.write(end.wrapping_sub(0x10), r0);
        mem.write(end.wrapping_sub(0x20), [r0[0], r0[1], r0[2]]);
        mem.write(end.wrapping_sub(0xc), heap);
        mem.write(end.wrapping_sub(0x10), heap);
        let r3: u32 = mem.read(Global::<u32>::new(0x00c8_f514).ptr(mem).addr);
        mem.write(end.wrapping_sub(0x14), r3);
        end
    }

    /// `op_malloc(heap, n)` (`0x007ff780`): `n` rounded up to 16 bytes
    /// (the top nibble dropped) from an op heap (see [`op_free`]): the
    /// first gap between consecutive blocks (from record 1, following the
    /// `+0xa` links) at least that large, else after the last block; a
    /// free record (the first from 2 down whose `u16 +8` is 0) is linked
    /// in between, the highest record number and the block count updated.
    /// Returns the block. A block reaching `heap` is fatal: the original
    /// prints a message to stderr (`__iob_func`, `fprintf`, through
    /// `call`) and writes to address 0.
    pub fn op_malloc<M: Mem>(
        mem: &mut M,
        heap: u32,
        n: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let size = n.wrapping_add(0xf) & 0x0fff_fff0;
        let rec = |i: i32| heap.wrapping_sub(i.cast_unsigned().wrapping_shl(4));
        let index = |r: u32| heap.wrapping_sub(r).cast_signed() / 16;
        let mut prev = heap.wrapping_sub(0x10);
        let mut next = i32::from(mem.read::<i16>(heap.wrapping_sub(6)));
        while next != 0 {
            let r = rec(next);
            let gap = mem
                .read::<u32>(r)
                .wrapping_sub(mem.read::<u32>(prev.wrapping_add(4)))
                .cast_signed();
            if gap >= size.cast_signed() {
                break;
            }
            next = i32::from(mem.read::<i16>(r.wrapping_add(0xa)));
            prev = r;
        }
        let at = index(prev);
        let start: u32 = mem.read(prev.wrapping_add(4));
        let end = size.wrapping_add(start);
        if heap <= end {
            let slot = |mem: &M, a: u32| mem.read::<u32>(Global::<u32>::new(a).ptr(mem).addr);
            let iob = slot(mem, 0x00b0_c350);
            let file = call(mem, iob, &[]).wrapping_add(0x40);
            let fprintf = slot(mem, 0x00b0_c38c);
            let msg = Global::<u32>::new(0x00b5_8698).ptr(mem).addr;
            call(mem, fprintf, &[file, msg]);
            mem.write(0, 0u32);
        }
        let mut k = 2i32;
        let mut r = heap.wrapping_sub(0x20);
        while mem.read::<u16>(r.wrapping_add(8)) != 0 {
            r = r.wrapping_sub(0x10);
            k += 1;
        }
        mem.write(r.wrapping_add(4), end);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // the i16 record fields
        {
            mem.write(r.wrapping_add(8), at as u16);
            mem.write(r, start);
            mem.write(r.wrapping_add(0xa), next as u16);
            mem.write(rec(at).wrapping_add(0xa), k as u16);
            if next != 0 {
                mem.write(rec(next).wrapping_add(8), k as u16);
            }
            if i32::from(mem.read::<i16>(heap.wrapping_sub(4))) < k {
                mem.write(heap.wrapping_sub(4), k as u16);
            }
        }
        let c: u16 = mem.read(heap.wrapping_sub(2));
        mem.write(heap.wrapping_sub(2), c.wrapping_add(1));
        start
    }

    /// `op_trans_m(dst, src, v)` (`0x007ea550`): `dst` = `src` with
    /// `v.xyz` added to its translation (row 3; `w` kept), through the
    /// emulated VU0 registers (`v` at `0x00c8f980`, row 3 at `0x00c8f970`,
    /// rows 0..2 at `0x00c8f578`; arguments in `a0`..`a2`). Returns the
    /// translation's `w` (left in `eax`).
    pub fn op_trans_m(mem: &mut impl Mem, dst: u32, src: u32, v: u32) -> u32 {
        for (n, a) in [dst, src, v, 0].into_iter().enumerate() {
            #[allow(clippy::cast_possible_truncation)] // 0..4
            ffx_ps2::ee::bridge::set_func_param(mem, n as u32, a);
        }
        let (vr, tr, rows) = (
            Global::<u32>::new(0x00c8_f980).ptr(mem).addr,
            Global::<u32>::new(0x00c8_f970).ptr(mem).addr,
            Global::<u32>::new(0x00c8_f578).ptr(mem).addr,
        );
        let vv: [u32; 4] = mem.read(v);
        mem.write(vr, vv);
        let t: [u32; 4] = mem.read(src.wrapping_add(0x30));
        mem.write(tr, t);
        let r: [u32; 12] = mem.read(src);
        mem.write(rows, r);
        for i in 0..3 {
            let s = x87::add(rf(mem, tr + 4 * i), rf(mem, vr + 4 * i));
            mem.write(tr + 4 * i, s);
        }
        let r: [u32; 12] = mem.read(rows);
        mem.write(dst, r);
        let t: [u32; 4] = mem.read(tr);
        mem.write(dst.wrapping_add(0x30), t);
        t[3]
    }

    /// `op_rtp_w1(dst, m, v)` (`0x007e9b50`): the point `m` times (`v.xyz`,
    /// `vf00.w`) as in `op_apply_m_w1` (into `0x00c8f930`), divided by its
    /// `w` (`q` = `vf00.w / w`, kept at `0x00c8f788`; `w` itself kept),
    /// then each component times 16 truncated (`_ftol2_sse`) into
    /// `0x00c8f940` and `dst` (four `i32`). Returns the last (left in
    /// `eax`).
    pub fn op_rtp_w1(mem: &mut impl Mem, dst: u32, m: u32, v: u32) -> u32 {
        use x87::{ADD, MUL, Reg};
        let div: x87::Op = |a, b| a / b;
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
            let r = Reg::load_f32(rf(mem, mr + 0x30 + 4 * i))
                .mem(MUL, w)
                .mem(ADD, rf(mem, acc + 4 * i))
                .store();
            mem.write(out + 4 * i, r);
        }
        let qa = Global::<u32>::new(0x00c8_f788).ptr(mem).addr;
        let q = Reg::load_f32(w).mem(div, rf(mem, out + 0xc)).store();
        mem.write(qa, q);
        for i in 0..3 {
            let r = Reg::load_f32(rf(mem, out + 4 * i)).mem(MUL, q).store();
            mem.write(out + 4 * i, r);
        }
        let o: [f32; 4] = mem.read(out);
        let k = f64_const(mem, 0x00b3_13a0);
        for (i, c) in (0u32..).zip(o) {
            let n = ffx_bridge::x87::ftol(mem, Reg::load_f32(c).constant(MUL, k).value());
            mem.write(vr + 4 * i, n);
        }
        let r: [u32; 4] = mem.read(vr);
        mem.write(dst, r);
        r[3]
    }

    fn f64_const(mem: &impl Mem, sym: u32) -> f64 {
        mem.read(Global::<u32>::new(sym).ptr(mem).addr)
    }

    /// `op_vector_m(dst, t, v)` (`0x007ea760`): a frame looking along `v`
    /// moved to `t`: after a `dbgPrintf(0x00b582b0, 0x9ee)`, `side` = `v`
    /// cross an up vector (`0x00c48bf0`, or `0x00c48c00` when `v.x | v.z`
    /// and the emulated `a0`/`a1` bits are all zero, as the original's
    /// `r0` compare decides; those registers are written as it does); rows
    /// 0 and 2 of a unit matrix the normalized `side` and `v`, row 1 their
    /// cross product, the rows' `w` 0; `FFXSamp0TransMatrix(dst, m, t)`.
    /// All callees (`FFXVu0OuterProduct`, `FFXVu0UnitMatrix`,
    /// `FFXVu0Normalize`) through `call`; the locals lie below `frame`
    /// (down to `frame - 0x54`). Returns the last call's result.
    pub fn op_vector_m<M: Mem>(
        mem: &mut M,
        dst: u32,
        t: u32,
        v: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let g = |mem: &M, a: u32| Global::<u32>::new(a).ptr(mem).addr;
        let dbg = g(mem, 0x0062_f500);
        let msg = g(mem, 0x00b5_82b0);
        call(mem, dbg, &[msg, 0x9ee]);
        let (r2, r3, r6, r7) = (
            g(mem, 0x00c8_f528),
            g(mem, 0x00c8_f52c),
            g(mem, 0x00c8_f538),
            g(mem, 0x00c8_f53c),
        );
        let vx: u32 = mem.read(v);
        let hi = mem.read::<u32>(r3) | mem.read::<u32>(r7);
        mem.write(r2, vx);
        let vz: u32 = mem.read(v.wrapping_add(8));
        mem.write(r6, vz);
        let lo = vx | vz;
        mem.write(r2, lo);
        mem.write(r3, hi);
        let r0: u32 = mem.read(g(mem, 0x00c8_f508));
        let r0h: u32 = mem.read(g(mem, 0x00c8_f50c));
        let flag = if lo == r0 && hi == r0h {
            r0
        } else {
            r0.wrapping_add(1)
        };
        let up = g(mem, if flag == 0 { 0x00c4_8c00 } else { 0x00c4_8bf0 });
        let (side, m) = (frame.wrapping_sub(0x14), frame.wrapping_sub(0x54));
        let outer = g(mem, 0x006e_d720);
        let unit = g(mem, 0x006e_d9a0);
        let norm = g(mem, 0x006e_d6f0);
        call(mem, outer, &[side, up, v]);
        call(mem, unit, &[m]);
        call(mem, norm, &[m, side]);
        call(mem, norm, &[m.wrapping_add(0x20), v]);
        call(mem, outer, &[m.wrapping_add(0x10), m.wrapping_add(0x20), m]);
        for at in [0x0cu32, 0x1c, 0x2c] {
            mem.write(m.wrapping_add(at), 0.0f32);
        }
        let trans = g(mem, 0x0093_d830);
        call(mem, trans, &[dst, m, t])
    }

    /// `op_matrix_light4_vector(dst, m, colours)` (`0x00a6cd80`): the light
    /// matrix of four directional lights: `L` = `m` with every row's
    /// `xyz` negated, each row (w 0) normalized and scaled by `m`'s row
    /// `w` (`FFXVu0Normalize`, `FFXVu0ScaleVector`, w 0 again), then
    /// `0x006ed990(L, L)`; `C` = `colours` with rows 0..2 (w 0) normalized
    /// and row 3 zero; `dst` rows 0..2 = `L` times `C`'s rows
    /// (`FFXVu0ApplyMatrix`), row 3 = `L`'s row 3 (`FFXVu0CopyVector`). All
    /// callees through `call`; the locals lie below `frame` (down to
    /// `frame - 0x84`). Returns the last call's result.
    pub fn op_matrix_light4_vector<M: Mem>(
        mem: &mut M,
        dst: u32,
        m: u32,
        colours: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let g = |mem: &M, a: u32| Global::<u32>::new(a).ptr(mem).addr;
        let (l, c) = (frame.wrapping_sub(0x44), frame.wrapping_sub(0x84));
        let copy = g(mem, 0x006e_d360);
        call(mem, copy, &[l, m]);
        for r in 0..4u32 {
            for k in 0..3u32 {
                let at = l.wrapping_add(0x10 * r + 4 * k);
                let x: u32 = mem.read(at);
                mem.write(at, x87::load(x) ^ 0x8000_0000);
            }
        }
        let (norm, scale) = (g(mem, 0x006e_d6f0), g(mem, 0x006e_d8f0));
        for r in 0..4u32 {
            let row = l.wrapping_add(0x10 * r);
            mem.write(row.wrapping_add(0xc), 0.0f32);
            call(mem, norm, &[row, row]);
            let k: u32 = mem.read(m.wrapping_add(0xc + 0x10 * r));
            call(mem, scale, &[row, row, x87::load(k)]);
            mem.write(row.wrapping_add(0xc), 0.0f32);
        }
        let f990 = g(mem, 0x006e_d990);
        call(mem, f990, &[l, l]);
        call(mem, copy, &[c, colours]);
        for r in 0..3u32 {
            let row = c.wrapping_add(0x10 * r);
            mem.write(row.wrapping_add(0xc), 0.0f32);
            call(mem, norm, &[row, row]);
            mem.write(row.wrapping_add(0xc), 0.0f32);
        }
        for at in [0x3cu32, 0x38, 0x34, 0x30] {
            mem.write(c.wrapping_add(at), 0.0f32);
        }
        let apply = g(mem, 0x006e_d2e0);
        for r in 0..3u32 {
            call(
                mem,
                apply,
                &[dst.wrapping_add(0x10 * r), l, c.wrapping_add(0x10 * r)],
            );
        }
        let cv = g(mem, 0x006e_d370);
        call(mem, cv, &[dst.wrapping_add(0x30), l.wrapping_add(0x30)])
    }

    /// `g_OpOef2ClutOvlFlags` (`0x011a05c0`): a byte per overlay CLUT,
    /// nonzero for a plain 16-colour CLUT.
    pub const OEF2_CLUT_OVL_FLAGS: Global<u8> = Global::new(0x011a_05c0);
    /// `g_OpOef2ClutOvlRecs` (`0x011a0600`): the texture record of each
    /// overlay CLUT.
    pub const OEF2_CLUT_OVL_RECS: Global<u32> = Global::new(0x011a_0600);
    /// The bytes per pixel row factor of each pixel format (`0x00c44bb8`).
    const PSM_BITS: u32 = 0x00c4_4bb8;

    /// A CLUT's area: (base, x, y, width - 1, height - 1, stride) for a
    /// texture record (`flag` 0: `+0` base & 0x3fff, bytes `+8`, `+9`,
    /// `+0xa` times 16, `u16 +0xc`, `+0xe`, format `+0x18`) or a plain
    /// CLUT (`u16 +0` base, 16 x 16 at 0, 0, format `+8`), and its last
    /// block.
    fn clut_area(mem: &impl Mem, rec: u32, flag: u8) -> ([i32; 6], i32) {
        let b = |o: u32| i32::from(mem.read::<u8>(rec.wrapping_add(o)));
        let w = |o: u32| i32::from(mem.read::<u16>(rec.wrapping_add(o)));
        let (a, psm) = if flag == 0 {
            let base = (mem.read::<u32>(rec) & 0x3fff).cast_signed();
            (
                [
                    base,
                    b(8) << 4,
                    b(9) << 4,
                    w(0xc) - 1,
                    w(0xe) - 1,
                    b(0xa) << 4,
                ],
                b(0x18),
            )
        } else {
            ([w(0), 0, 0, 15, 15, 16], b(8))
        };
        let k = i32::from(
            mem.read::<u8>(
                Global::<u8>::new(PSM_BITS)
                    .ptr(mem)
                    .addr
                    .wrapping_add(psm.cast_unsigned()),
            ),
        );
        let blocks = k.wrapping_mul(a[4]).wrapping_mul(a[5]) / 16 / 256;
        (a, blocks - 1 + a[0])
    }

    /// `op_oef2_tex_clut_ovl_add_007e56d0(rec, mask, flag)` (`0x007e56d0`):
    /// drops every overlay CLUT that shares a bit with `mask` and whose
    /// area overlaps the new one's (the same base and stride with
    /// overlapping rectangles, or a block range that overlaps with a
    /// different base); packs the rest down; then, unless `rec` is still
    /// there (-1) or all 64 are taken (-1), appends (`mask`, `flag`,
    /// `rec`), sets the count to one past it and returns its index.
    pub fn op_oef2_tex_clut_ovl_add(mem: &mut impl Mem, rec: u32, mask: u32, flag: u32) -> i32 {
        let (new, last) = clut_area(mem, rec, u8::from(flag != 0));
        let masks = OEF2_CLUT_OVL.ptr(mem);
        let flags = OEF2_CLUT_OVL_FLAGS.ptr(mem);
        let recs = OEF2_CLUT_OVL_RECS.ptr(mem);
        for i in 0..64 {
            if masks.at(i).get(mem) & mask == 0 {
                continue;
            }
            let (old, old_last) = clut_area(mem, recs.at(i).get(mem), flags.at(i).get(mem));
            let near = if new[0] > old[0] {
                new[0] <= old_last
            } else {
                last >= old[0]
            };
            if !near {
                continue;
            }
            let overlap = if new[0] != old[0] || new[5] != old[5] {
                true
            } else {
                let xs = if new[1] > old[1] {
                    new[1] <= old[3] + old[1]
                } else {
                    new[3] + new[1] >= old[1]
                };
                xs && if new[2] > old[2] {
                    new[2] <= old[4] + old[2]
                } else {
                    new[4] + new[2] >= old[2]
                }
            };
            if overlap {
                masks.at(i).set(mem, 0);
            }
        }
        let (mut k, mut dup) = (0u32, false);
        for i in 0..64 {
            let mi = masks.at(i).get(mem);
            if mi == 0 {
                continue;
            }
            masks.at(k).set(mem, mi);
            let f = flags.at(i).get(mem);
            flags.at(k).set(mem, f);
            let r = recs.at(i).get(mem);
            recs.at(k).set(mem, r);
            dup |= r == rec;
            k += 1;
        }
        for j in k..64 {
            masks.at(j).set(mem, 0);
            flags.at(j).set(mem, 0);
            recs.at(j).set(mem, 0);
        }
        if dup || k >= 64 {
            return -1;
        }
        masks.at(k).set(mem, mask);
        #[allow(clippy::cast_possible_truncation)] // the flag byte
        flags.at(k).set(mem, flag as u8);
        recs.at(k).set(mem, rec);
        #[allow(clippy::cast_possible_truncation)] // below 64
        OEF2_CLUT_OVL_COUNT.set(mem, (k + 1) as u16);
        k.cast_signed()
    }

    /// `op_unit_m_scale2(m, v)` (`0x007ea730`): `m` zeroed (`memset`), its
    /// diagonal `v.x`, `v.y`, `v.z`, 1.0. Returns `v` (left in `eax`).
    pub fn op_unit_m_scale2(mem: &mut impl Mem, m: u32, v: u32) -> u32 {
        mem.write(m, [0u32; 16]);
        for (i, at) in [0u32, 0x14, 0x28].into_iter().enumerate() {
            #[allow(clippy::cast_possible_truncation)] // 0..3
            let x: u32 = mem.read(v.wrapping_add(4 * i as u32));
            mem.write(m.wrapping_add(at), x87::load(x));
        }
        mem.write(m.wrapping_add(0x3c), 1.0f32);
        v
    }

    /// `op_fp_effect_free()` (`0x007fcc40`) and `op_fp_effect_init()`
    /// (`0x007fcc60`): a `dbgPrintf` (`msg`, through `call`), then the
    /// overlay CLUTs of bit 30 cleared (`op_oef2_tex_clut_ovl_clear`).
    /// Returns its result.
    pub fn op_fp_effect_reset<M: Mem>(
        mem: &mut M,
        msg: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = Global::<u32>::new(0x0062_f500).ptr(mem).addr;
        let m = Global::<u32>::new(msg).ptr(mem).addr;
        call(mem, f, &[m]);
        op_oef2_tex_clut_ovl_clear(mem, 0x4000_0000)
    }

    /// `op_et_prayeff_send_read_through(id)` (`0x007fc920`): after a
    /// `dbgPrintf(0x00b58568)` (through `call`), the first `g_EtEff`
    /// record with that ID gets flag 0x1000 in its part's `u16 +0xb0`.
    /// Returns the part, or the table's end when there is none.
    pub fn op_et_prayeff_send_read_through<M: Mem>(
        mem: &mut M,
        id: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = Global::<u32>::new(0x0062_f500).ptr(mem).addr;
        let m = Global::<u32>::new(0x00b5_8568).ptr(mem).addr;
        call(mem, f, &[m]);
        let eff = crate::et::bridge::EFF.ptr(mem);
        for i in 0..crate::et::bridge::EFFS {
            let e = eff.at(i).addr;
            if mem.read::<u32>(e) == id {
                let part: u32 = mem.read(e + 4);
                let v: u16 = mem.read(part.wrapping_add(0xb0));
                mem.write(part.wrapping_add(0xb0), v | 0x1000);
                return part;
            }
        }
        eff.at(crate::et::bridge::EFFS).addr
    }

    /// `op_deg_in_check(a, b, c)` (`0x007e7f60`): whether angle `a` lies in
    /// the arc from `b` to `c` (each truncated, `_ftol2_sse`, to 12 bits),
    /// the arc wrapping when `b > c`; 1 or 0.
    pub fn op_deg_in_check(mem: &impl Mem, a: f32, b: f32, c: f32) -> u32 {
        let t = |x: f32| {
            (ffx_bridge::x87::ftol(mem, x87::Reg::load_f32(x).value()) & 0xfff).cast_signed()
        };
        let (x, y, z) = (t(a), t(b), t(c));
        let inside = |x: i32, z: i32| x <= z;
        let r = if y > z {
            y <= x || inside(x, z)
        } else {
            y <= x && inside(x, z)
        };
        u32::from(r)
    }

    /// The op script symbol hash: the bytes (sign-extended, ASCII letters
    /// lowered when `nocase`) each combined by XOR with `0x6f` times its index,
    /// summed (wrapping).
    fn op_symbol_hash(mem: &impl Mem, name: u32, nocase: bool) -> u32 {
        let mut sum = 0u32;
        let mut salt = 0u32;
        let mut at = name;
        loop {
            let mut c: u8 = mem.read(at);
            if c == 0 {
                return sum;
            }
            if nocase && c.wrapping_sub(b'A') <= 0x19 {
                c = c.wrapping_add(0x20);
            }
            sum = sum.wrapping_add(i32::from(c.cast_signed()).cast_unsigned() ^ salt);
            salt = salt.wrapping_add(0x6f);
            at = at.wrapping_add(1);
        }
    }

    /// `0x007e11a0(name, list)` (`op_symbol_find`): the first symbol of the
    /// list (linked by `+0x20`) that `name` names. A symbol has its name at
    /// `[+0]`, an alias at `[+4]`, `i16 +0x12` set for a case-blind one,
    /// and the name's hashes (see [`op_symbol_hash`]) at `+0x18` (exact)
    /// and `+0x1c` (case-blind). An exact symbol matches when the hash and
    /// the name match and the alias is absent or differs from `name`; a
    /// case-blind one when they match with letters folded and
    /// `op_str_equal(name, alias, 1)` is 0. Returns the symbol, or 0.
    pub fn op_symbol_find(mem: &impl Mem, name: u32, list: u32) -> u32 {
        let exact = op_symbol_hash(mem, name, false);
        let blind = op_symbol_hash(mem, name, true);
        let fold = |c: u8| {
            if c.wrapping_sub(b'A') <= 0x19 {
                c.wrapping_add(0x20)
            } else {
                c
            }
        };
        // strcmp, with or without folding
        let same = |a: u32, b: u32, nocase: bool| {
            let mut i = 0u32;
            loop {
                let (mut x, mut y) = (
                    mem.read::<u8>(a.wrapping_add(i)),
                    mem.read::<u8>(b.wrapping_add(i)),
                );
                if nocase {
                    (x, y) = (fold(x), fold(y));
                }
                if x != y {
                    return false;
                }
                if x == 0 {
                    return true;
                }
                i = i.wrapping_add(1);
            }
        };
        let mut sym = list;
        while sym != 0 {
            let own: u32 = mem.read(sym);
            let alias: u32 = mem.read(sym.wrapping_add(4));
            if mem.read::<i16>(sym.wrapping_add(0x12)) == 0 {
                if exact == mem.read::<u32>(sym.wrapping_add(0x18))
                    && own != 0
                    && same(name, own, false)
                    && (alias == 0 || !same(name, alias, false))
                {
                    return sym;
                }
            } else if blind == mem.read::<u32>(sym.wrapping_add(0x1c))
                && own != 0
                && same(name, own, true)
                && op_str_equal(mem, name, alias, 1) == 0
            {
                return sym;
            }
            sym = mem.read(sym.wrapping_add(0x20));
        }
        0
    }

    /// The byte `0x011a0064` that picks `op_oef2_tex0_get`'s second
    /// register layout.
    pub const TEX0_ALT: Global<u8> = Global::new(0x011a_0064);
    /// The word `0x011a0730` whose low 9 bits give `op_oef2_tex0_get`'s
    /// default base.
    pub const TEX0_BASE: Global<u32> = Global::new(0x011a_0730);

    /// `0x007e5260(oef, no, out)` (`op_oef2_tex0_get`): the 0x20-byte
    /// texture record `no` of `oef` (`op_oef2_a_get(oef, 2, no)`) copied to
    /// `out` for a non-negative `i16 no`. Otherwise a default: a base `b`
    /// from [`TEX0_BASE`]'s low 9 bits (`b << 5` for an even `no`, else
    /// 0xd00 for 0 and 0 for anything else; 0x1a00 for `no` = 0x8000 in the
    /// plain layout). With [`TEX0_ALT`]: `+0` = `b | 0x64120000` (0x64a21a00
    /// for 0x8000), `+8` = 0x20200000, `+0xc` = 0x2000200, `+0x10`/`+0x14`
    /// = 0, byte `+0x18` = 1 (10 for 0x8000), byte `+0x19` = 0. Plain: `+0`
    /// = `b | 0xe4020000`, `+8` = 0x8200000, `+0xc` = 0x800200, `+0x10`/
    /// `+0x14` = 0, `u16 +0x18` = 0. Returns `eax` as left: the record's
    /// last word, `+0` (alt layout; 0xffff8000 for 0x8000) or `out`.
    pub fn op_oef2_tex0_get(mem: &mut impl Mem, oef: u32, no: u16, out: u32) -> u32 {
        let n = no.cast_signed();
        if n >= 0 {
            let rec = op_oef2_a_get(mem, oef, 2, i32::from(n).cast_unsigned());
            let words: [u32; 8] = mem.read(rec);
            mem.write(out, words);
            return words[7];
        }
        let base = || {
            let b = TEX0_BASE.get(mem) & 0x1ff;
            if no & 1 == 0 {
                b << 5
            } else if b != 0 {
                0
            } else {
                0xd00
            }
        };
        if TEX0_ALT.get(mem) != 0 {
            let ret = if no == 0x8000 {
                mem.write(out, 0x64a2_1a00u32);
                mem.write(out.wrapping_add(0x18), 10u8);
                0xffff_8000
            } else {
                let v = base() | 0x6412_0000;
                mem.write(out, v);
                mem.write(out.wrapping_add(0x18), 1u8);
                v
            };
            mem.write(out.wrapping_add(8), [0x2020_0000u32, 0x0200_0200, 0, 0]);
            mem.write(out.wrapping_add(0x19), 0u8);
            return ret;
        }
        let b = if no == 0x8000 { 0x1a00 } else { base() };
        mem.write(out, b | 0xe402_0000);
        mem.write(out.wrapping_add(8), [0x0820_0000u32, 0x0080_0200, 0, 0]);
        mem.write(out.wrapping_add(0x18), 0u16);
        out
    }

    /// `op_str_equal_007e2600(a, b, nocase)` (`0x007e2600`): -1 when the
    /// two strings are equal (ASCII letters folded to lower case when
    /// `nocase`), else 0; 0 for a null string.
    pub fn op_str_equal(mem: &impl Mem, a: u32, b: u32, nocase: u32) -> u32 {
        if a == 0 || b == 0 {
            return 0;
        }
        let fold = |c: u8| {
            if nocase != 0 && c.wrapping_sub(b'A') <= 0x19 {
                c.wrapping_add(0x20)
            } else {
                c
            }
        };
        let mut i = 0u32;
        loop {
            let x = fold(mem.read(a.wrapping_add(i)));
            let y = fold(mem.read(b.wrapping_add(i)));
            if x != y {
                return 0;
            }
            if x == 0 {
                return u32::MAX;
            }
            i += 1;
        }
    }

    /// `g_OpPairTable` (`0x022fb400`): a count, then (a, b) word pairs from
    /// entry 1.
    pub const PAIR_TABLE: Global<u32> = Global::new(0x022f_b400);

    /// `op_pair_index_00a70ce0(a, b)` (`0x00a70ce0`): the index (from 1) of
    /// the pair (`a`, `b`) in `PAIR_TABLE`, appended when absent (the
    /// count's carry going into entry 0's second word). Returns the index.
    pub fn op_pair_index(mem: &mut impl Mem, a: u32, b: u32) -> u32 {
        let t = PAIR_TABLE.ptr(mem).addr;
        let n: u32 = mem.read(t);
        let mut i = 1u32;
        while i.cast_signed() <= n.cast_signed() {
            let e = t.wrapping_add(8 * i);
            if mem.read::<u32>(e) == a && mem.read::<u32>(e + 4) == b {
                return i;
            }
            i += 1;
        }
        let (k, carry) = n.overflowing_add(1);
        let hi: u32 = mem.read(t + 4);
        mem.write(t + 4, hi.wrapping_add(u32::from(carry)));
        mem.write(t, k);
        mem.write(t.wrapping_add(k.wrapping_mul(8)), a);
        let k: u32 = mem.read(t);
        mem.write(t.wrapping_add(k.wrapping_mul(8)).wrapping_add(4), b);
        k
    }

    /// `g_OpFog` (`0x011a0010`): the effects' fog: colour bytes (`+0`..`+2`),
    /// near, far and a third parameter (`+4`, `+8`, `+0xc`, from
    /// `yiGetFogParam`), then the derived scale, offset, negated near and
    /// 255.0 (`+0x10`..`+0x1c`).
    pub const FOG: Global<u8> = Global::new(0x011a_0010);

    /// `op_fog_param_set()` (`0x007ff100`): the map's fog into `FOG`: the
    /// colour (`yiGetFogColor` into a local, its three low bytes), the
    /// parameters (`yiGetFogParam(&near, &far, &c)`; both through `call`),
    /// then `+0x10` = `-(255 / (far - near))` (0 when they are equal),
    /// `+0x14` = `255 - c`, `+0x18` = `-near`, `+0x1c` = 255.0. The local
    /// lies below `frame` (from `frame - 0x14`).
    pub fn op_fog_param_set<M: Mem>(
        mem: &mut M,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use x87::{Reg, SUB};
        let div: x87::Op = |a, b| a / b;
        let g = |mem: &M, a: u32| Global::<u32>::new(a).ptr(mem).addr;
        let local = frame.wrapping_sub(0x14);
        let colour = g(mem, 0x0090_8980);
        call(mem, colour, &[local]);
        let fog = FOG.ptr(mem).addr;
        for k in 0..3u32 {
            let b: u8 = mem.read(local + 4 * k);
            mem.write(fog + k, b);
        }
        let param = g(mem, 0x0090_8bb0);
        call(mem, param, &[fog + 4, fog + 8, fog + 0xc]);
        for at in [4u32, 8] {
            let x: u32 = mem.read(fog + at);
            mem.write(fog + at, x87::load(x));
        }
        let (near, far) = (Reg::load(mem.read(fog + 4)), Reg::load(mem.read(fog + 8)));
        let k: f64 = mem.read(g(mem, 0x00b1_70c0));
        #[allow(clippy::float_cmp)] // fucomp: exactly equal
        let equal = near.value() == far.value();
        let scale = if equal {
            0.0
        } else {
            let d = far.reg(SUB, near);
            match Reg::Num(k).reg(div, d) {
                Reg::Num(x) => Reg::Num(-x),
                Reg::Nan(b) => Reg::Nan(b ^ 0x8000_0000),
            }
            .store()
        };
        mem.write(fog + 0x10, scale);
        let off = Reg::Num(k).mem(SUB, mem.read(fog + 0xc)).store();
        mem.write(fog + 0x14, off);
        let n = match near {
            Reg::Num(x) => Reg::Num(-x),
            Reg::Nan(b) => Reg::Nan(b ^ 0x8000_0000),
        };
        mem.write(fog + 0x18, n.store());
        let top: u32 = mem.read(g(mem, 0x00b4_80d4));
        mem.write(fog + 0x1c, x87::load(top));
    }

    /// `op_oef2_sep_wave_load(cue, mask)` (`0x007fffa0`): the sound sep of
    /// the cue's wave (`cue +8`, its handle `+8`) gets a slot in
    /// `OEF2_SEPS` unless one holds it already: the first free one, with
    /// `mask` among its users, then `SndSepTransSepOnly(wave, wave +0xc)`;
    /// then, for a positive `cue +0xc`, `SndStandbyBattleWave(it, 0)`
    /// (both through `call`) and 1, else 0.
    pub fn op_oef2_sep_wave_load<M: Mem>(
        mem: &mut M,
        cue: u32,
        mask: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let wave = stub_ptr(mem.read(cue.wrapping_add(8)));
        let h: u32 = mem.read(wave.wrapping_add(8));
        let seps = OEF2_SEPS.ptr(mem);
        if !(0..32).any(|i| mem.read::<u32>(seps.at(i).addr) == h)
            && let Some(i) = (0..32).find(|&i| mem.read::<u32>(seps.at(i).addr) == 0)
        {
            {
                let at = seps.at(i).addr;
                mem.write(at, h);
                let u: u32 = mem.read(at + 4);
                mem.write(at + 4, u | mask);
                let len: u32 = mem.read(wave.wrapping_add(0xc));
                let f = Global::<u32>::new(0x0088_6f30).ptr(mem).addr;
                call(mem, f, &[wave, len]);
            }
        }
        let n: u32 = mem.read(cue.wrapping_add(0xc));
        if n.cast_signed() > 0 {
            let f = Global::<u32>::new(0x0088_7450).ptr(mem).addr;
            call(mem, f, &[n, 0]);
            1
        } else {
            0
        }
    }

    fn stub_ptr(p: u32) -> u32 {
        ffx_system::stub::ptr32(p)
    }

    /// `op_ffxlight_to_omdlight(colours, dirs)` (`0x00a690c0`): the scene
    /// lights in the model renderer's layout: `dirs` rows 0..2 =
    /// `Sg_LightGetDir(0..2)` negated (all four words) with `w` 1.0, row 3
    /// the emulated r0; `colours` rows 0..2 = `Sg_LightGetColor(0..2)`, row
    /// 3 r0, and the ambient colour (`Sg_LightGetColor(3)`) into a local
    /// below `frame` (`frame - 0x14`). Callees through `call`. Returns 0.
    pub fn op_ffxlight_to_omdlight<M: Mem>(
        mem: &mut M,
        colours: u32,
        dirs: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let g = |mem: &M, a: u32| Global::<u32>::new(a).ptr(mem).addr;
        let dir = g(mem, 0x0083_61c0);
        for k in 0..3u32 {
            call(mem, dir, &[k, dirs.wrapping_add(0x10 * k)]);
        }
        let r0: [u32; 4] = mem.read(g(mem, 0x00c8_f508));
        mem.write(dirs.wrapping_add(0x30), r0);
        for i in 0..12u32 {
            let at = dirs.wrapping_add(4 * i);
            let x: u32 = mem.read(at);
            mem.write(at, x87::load(x) ^ 0x8000_0000);
        }
        for at in [0x2cu32, 0x1c, 0xc] {
            mem.write(dirs.wrapping_add(at), 1.0f32);
        }
        let colour = g(mem, 0x0083_60f0);
        for k in 0..3u32 {
            call(mem, colour, &[k, colours.wrapping_add(0x10 * k)]);
        }
        let r0: [u32; 4] = mem.read(g(mem, 0x00c8_f508));
        mem.write(colours.wrapping_add(0x30), r0);
        call(mem, colour, &[3, frame.wrapping_sub(0x14)]);
        0
    }

    /// `op_md_vn_index_sub(tri, table)` (`0x00a69fe0`): for each of the
    /// triangle's three vertices (`u16 tri[0..3]`), its 64-byte list in
    /// `table` (an `u16` count, then `i16` entries) gets the vertex's
    /// normal index (`u16 tri[0xa + k]`) appended unless present. Returns
    /// what the original leaves in `eax`: the last list's new count, or
    /// the index when it was there.
    pub fn op_md_vn_index_sub(mem: &mut impl Mem, tri: u32, table: u32) -> u32 {
        let mut eax = 0;
        for k in 0..3u32 {
            let list = u32::from(mem.read::<u16>(tri.wrapping_add(2 * k)))
                .wrapping_shl(6)
                .wrapping_add(table);
            let n: u16 = mem.read(list);
            let key: u16 = mem.read(tri.wrapping_add(0x14 + 2 * k));
            let count = i32::from(n.cast_signed());
            let mut at = list.wrapping_add(2);
            let mut found = false;
            let mut i = 0;
            while i < count {
                if i32::from(mem.read::<i16>(at)) == i32::from(key) {
                    found = true;
                    break;
                }
                i += 1;
                at = at.wrapping_add(2);
            }
            if found {
                eax = u32::from(key);
            } else {
                mem.write(at, key);
                let c = u32::from(n) + 1;
                #[allow(clippy::cast_possible_truncation)] // the u16 count
                mem.write(list, c as u16);
                eax = c;
            }
        }
        eax
    }

    /// The last op script error (`0x0113e238`): its code plus 1.
    pub const LAST_ERROR: Global<u32> = Global::new(0x0113_e238);

    /// `op_error_007e1450(code, arg)` (`0x007e1450`): reports an op script
    /// error through `dbgPrintf` (through `call`): the file name and line
    /// (`0x0113fc58`, `0x0113fc5c`) when set, then code 1..=11's message
    /// (from `0x00b57e24`.., each with `arg` except code 3); keeps `code +
    /// 1` in `LAST_ERROR` and returns it.
    pub fn op_error<M: Mem>(
        mem: &mut M,
        code: u32,
        arg: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        const MSGS: [u32; 11] = [
            0x00b5_7e24,
            0x00b5_7e44,
            0x00b5_7e68,
            0x00b5_7e90,
            0x00b5_7eb0,
            0x00b5_7ed0,
            0x00b5_7efc,
            0x00b5_7f20,
            0x00b5_7f3c,
            0x00b5_7f64,
            0x00b5_7f88,
        ];
        let g = |mem: &M, a: u32| Global::<u32>::new(a).ptr(mem).addr;
        let dbg = g(mem, 0x0062_f500);
        for (at, fmt) in [(0x0113_fc58u32, 0x00b5_7e14u32), (0x0113_fc5c, 0x00b5_7e1c)] {
            let v: u32 = mem.read(g(mem, at));
            if v != 0 {
                let f = g(mem, fmt);
                call(mem, dbg, &[f, v]);
            }
        }
        if let Some(&m) = MSGS.get(code.wrapping_sub(1) as usize) {
            let f = g(mem, m);
            if code == 3 {
                call(mem, dbg, &[f]);
            } else {
                call(mem, dbg, &[f, arg]);
            }
        }
        let r = code.wrapping_add(1);
        LAST_ERROR.set(mem, r);
        r
    }

    /// `op_spline(t, y, x, d, n)` (`0x007e9e20`): the cubic spline through
    /// (`x[i]`, `y[i]`) with slopes `d` (`op_spline_table`'s) at `t`:
    /// `y[0]` for one point, the line for two; else the interval `i` by
    /// binary search (`x[mid] < t` moves right), `h = x[i+1] - x[i]` and
    /// `a = t - x[i]` as `f32`, `y[i]` when `h` is 0, else
    /// `((s - (2 d[i] + d[i+1]) h + ((d[i+1] - d[i]) a / h + 3 d[i]) a) a +
    /// y[i]` with `s = (y[i+1] - y[i]) / h`, in 53 bits, as `f32`. Returns
    /// it in `ST0`.
    #[allow(clippy::many_single_char_names)]
    pub fn op_spline(mem: &impl Mem, t: f32, y: u32, x: u32, d: u32, n: u32) -> f32 {
        use x87::{ADD, MUL, Reg, SUB};
        let div: x87::Op = |p, q| p / q;
        let f = |base: u32, i: i32| {
            mem.read::<f32>(base.wrapping_add(i.cast_unsigned().wrapping_mul(4)))
        };
        let n = n.cast_signed();
        if n == 1 {
            return x87::load_f32(f(y, 0));
        }
        if n == 2 {
            let dy = Reg::load_f32(f(y, 1)).mem(SUB, f(y, 0)).store();
            return Reg::load_f32(dy).mem(MUL, t).mem(ADD, f(y, 0)).store();
        }
        let (mut lo, mut hi) = (0i32, n - 1);
        if hi > 0 {
            while lo < hi {
                let mid = hi.wrapping_add(lo) / 2;
                if f(x, mid) < x87::load_f32(t) {
                    lo = mid + 1;
                } else {
                    hi = mid;
                }
            }
            if lo > 0 {
                lo -= 1;
            }
        }
        let i = lo;
        let h = Reg::load_f32(f(x, i + 1)).mem(SUB, f(x, i)).store();
        let a = Reg::load_f32(t).mem(SUB, f(x, i)).store();
        if h == 0.0 {
            return x87::load_f32(f(y, i));
        }
        let hr = Reg::load_f32(h);
        let s = Reg::load_f32(f(y, i + 1)).mem(SUB, f(y, i)).reg(div, hr);
        let dd2 = Reg::load_f32(f(d, i));
        let two = dd2.reg(ADD, dd2).mem(ADD, f(d, i + 1)).reg(MUL, hr);
        let b = s.reg(SUB, two);
        let dd = Reg::load_f32(f(d, i + 1)).mem(SUB, f(d, i));
        let ar = Reg::load_f32(a);
        let c = dd.reg(MUL, ar).reg(div, hr);
        let three = Reg::load_f32(f(d, i)).constant(MUL, 3.0);
        let c3 = c.reg(ADD, three);
        let r = b.reg(ADD, c3.reg(MUL, ar)).reg(MUL, ar).mem(ADD, f(y, i));
        r.store()
    }

    /// `op_spline_table(d, x, y, n)` (`0x007e9f20`): the slopes of the
    /// natural cubic spline through (`x[i]`, `y[i]`) into `d` (`d[0]` and
    /// `d[n-1]` 0), by the tridiagonal sweep the original runs on a
    /// scratch buffer (`op_spr_alloc` of `(n + 3) & 0xfc` pairs of floats:
    /// the steps `h` and the slopes `w`, each `f32`, a zero step giving a
    /// zero slope), forward then back.
    pub fn op_spline_table(mem: &mut impl Mem, d: u32, x: u32, y: u32, n: u32) {
        use x87::{MUL, Reg, SUB};
        let div: x87::Op = |p, q| p / q;
        let m = n.wrapping_add(3) & 0xfc;
        let bytes = m.wrapping_mul(8);
        let h = op_spr_alloc(mem, bytes);
        let w = h.wrapping_add(m.wrapping_mul(4));
        let at = |base: u32, i: i32| base.wrapping_add(i.cast_unsigned().wrapping_mul(4));
        let n = n.cast_signed();
        mem.write(d, 0.0f32);
        mem.write(at(d, n - 1), 0.0f32);
        for i in 0..(n - 1).max(0) {
            let dv = Reg::load_f32(rf(mem, at(x, i + 1)))
                .mem(SUB, rf(mem, at(x, i)))
                .store();
            mem.write(at(h, i), dv);
            let r = if dv == 0.0 {
                0.0
            } else {
                Reg::load_f32(rf(mem, at(y, i + 1)))
                    .mem(SUB, rf(mem, at(y, i)))
                    .reg(div, Reg::load_f32(dv))
                    .store()
            };
            mem.write(at(w, i + 1), r);
        }
        let d1 = Reg::load_f32(rf(mem, at(w, 2)))
            .mem(SUB, rf(mem, at(w, 1)))
            .reg(SUB, Reg::load_f32(rf(mem, h)).mem(MUL, rf(mem, d)))
            .store();
        mem.write(at(d, 1), d1);
        let c: f64 = mem.read(Global::<u32>::new(0x00b9_2270).ptr(mem).addr);
        let w1 = Reg::load_f32(rf(mem, at(x, 2)))
            .mem(SUB, rf(mem, x))
            .constant(MUL, c)
            .store();
        mem.write(at(w, 1), w1);
        let k = n - 2;
        if k > 1 {
            for j in 0..k - 1 {
                let wj = rf(mem, at(w, j + 1));
                let r = if wj == 0.0 {
                    0.0
                } else {
                    Reg::load_f32(rf(mem, at(h, j + 1))).mem(div, wj).store()
                };
                let xx = Reg::load_f32(rf(mem, at(w, j + 3))).mem(SUB, rf(mem, at(w, j + 2)));
                let dj = Reg::load_f32(rf(mem, at(d, j + 1))).mem(MUL, r);
                mem.write(at(d, j + 2), xx.reg(SUB, dj).store());
                let yy = Reg::load_f32(rf(mem, at(x, j + 3)))
                    .mem(SUB, rf(mem, at(x, j + 1)))
                    .constant(MUL, c);
                let rh = Reg::load_f32(r).mem(MUL, rf(mem, at(h, j + 1)));
                mem.write(at(w, j + 2), yy.reg(SUB, rh).store());
            }
        }
        let last = Reg::load_f32(rf(mem, at(d, k)))
            .reg(
                SUB,
                Reg::load_f32(rf(mem, at(h, k))).mem(MUL, rf(mem, at(d, n - 1))),
            )
            .store();
        mem.write(at(d, k), last);
        let mut i = k;
        while i > 0 {
            let hi = rf(mem, at(h, i));
            let wi = rf(mem, at(w, i));
            let v = if hi == 0.0 || wi == 0.0 {
                0.0
            } else {
                Reg::load_f32(rf(mem, at(d, i)))
                    .reg(SUB, Reg::load_f32(rf(mem, at(d, i + 1))).mem(MUL, hi))
                    .mem(div, wi)
                    .store()
            };
            mem.write(at(d, i), v);
            i -= 1;
        }
        op_spr_free(mem);
    }

    /// `opd_num_init(x)` (`0x00a5dda0`): the debug number display's setup:
    /// `sceDmaGetChan(2)` into `0x022881d0`, the base column `x / 256` and
    /// four columns 0x10, 0x14, 0x18, 0x1c past it (`u16` at `0x00c87870`,
    /// `..880`, `..890`, `..8a0`, `..8b0`), `0x00a5e4a0()`, and its GS
    /// packet header at `0x02286140` (through `call`). Returns 0x1a0.
    pub fn opd_num_init<M: Mem>(
        mem: &mut M,
        x: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let g = |mem: &M, a: u32| Global::<u32>::new(a).ptr(mem).addr;
        let chan = g(mem, 0x0062_f6e0);
        let c = call(mem, chan, &[2]);
        mem.write(g(mem, 0x0228_81d0), c);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // the low half
        let col = (x.cast_signed() / 256) as u16;
        for (at, k) in [
            (0x00c8_7880u32, 0x10u16),
            (0x00c8_7890, 0x14),
            (0x00c8_78a0, 0x18),
        ] {
            mem.write(g(mem, at), col.wrapping_add(k));
        }
        mem.write(g(mem, 0x00c8_7870), col);
        mem.write(g(mem, 0x00c8_78b0), col.wrapping_add(0x1c));
        let f = g(mem, 0x00a5_e4a0);
        call(mem, f, &[]);
        let hdr = g(mem, 0x0228_6140);
        mem.write(hdr, g(mem, 0x0228_6170));
        mem.write(hdr + 8, 0x0001_0001u32);
        mem.write(hdr + 0x10, 0x0008_0000u32);
        mem.write(hdr + 4, u32::MAX);
        mem.write(hdr + 0x14, 0x0200_0001u32);
        mem.write(hdr + 0x1b, 0xb0u16);
        mem.write(hdr + 0x18, 0x1a0u16);
        mem.write(hdr + 0x1a, 0x80u8);
        0x1a0
    }

    /// `op_spline_xyz(out, t, pts, x, n)` (`0x007ea120`): the spline of
    /// each coordinate of the 16-byte points `pts` at `t`: the coordinate
    /// copied into a scratch block (the `op_spr` stack, `(n + 3) & 0xfc`
    /// words), then `op_spline(t, scratch, x, x + 4 n k + 4 n, n)` (the
    /// slopes of coordinate `k` follow the knots `x`) into `out[k]`.
    pub fn op_spline_xyz(mem: &mut impl Mem, out: u32, t: f32, pts: u32, x: u32, n: u32) {
        let buf = op_spr_alloc(mem, (n.wrapping_add(3) & 0xfc).wrapping_mul(4));
        let t = x87::load_f32(t);
        for k in 0..3u32 {
            for i in 0..n.cast_signed().max(0).cast_unsigned() {
                let v: u32 = mem.read(pts.wrapping_add(16 * i + 4 * k));
                mem.write(buf.wrapping_add(4 * i), x87::load(v));
            }
            let d = x.wrapping_add(n.wrapping_mul(4).wrapping_mul(k + 1));
            let r = op_spline(mem, t, buf, x, d, n);
            mem.write(out.wrapping_add(4 * k), r);
        }
        op_spr_free(mem);
    }

    /// `op_func_ot(list)` (`0x007eb430`): runs the functions of OT list
    /// `list` in order (`FUNC_OT_ORDER`, up to 32, until a free place):
    /// each slot's function with its two words (through `call`); a slot
    /// with a frame count (`u16 +0xc`) counts down and is freed at 0
    /// (`op_func_ot_free`), the next one moving into its place.
    pub fn op_func_ot<M: Mem>(
        mem: &mut M,
        list: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let mut i = 0i32;
        while i < 32 {
            let idx = FUNC_OT_ORDER
                .ptr(mem)
                .at(list.wrapping_add(i.cast_unsigned()))
                .get(mem);
            if idx < 0 {
                break;
            }
            let slot = i32::from(idx).cast_unsigned();
            let e = FUNC_OT.ptr(mem).at(slot.wrapping_add(list)).addr;
            let [f, a, b, _] = mem.read::<[u32; 4]>(e);
            call(mem, f, &[a, b]);
            let c: u16 = mem.read(e + 0xc);
            if c != 0 {
                mem.write(e + 0xc, c - 1);
                if c == 1 {
                    op_func_ot_free(mem, slot, list);
                    i -= 1;
                }
            }
            i += 1;
        }
    }

    /// `g_OpCharClass` (`0x0113fb58`): the op script tokenizer's class of
    /// each byte (1 word, 2 two-byte word, 3/4 quotes, 5 space, 6/8 single
    /// character, 7 comment or statement end).
    pub const CHAR_CLASS: Global<u8> = Global::new(0x0113_fb58);
    /// The tokenizer's block-comment depth (`0x0113fc64`) and whether it
    /// is inside one (`0x0113fc60`).
    pub const COMMENT_DEPTH: Global<u32> = Global::new(0x0113_fc64);
    /// See [`COMMENT_DEPTH`].
    pub const IN_COMMENT: Global<u32> = Global::new(0x0113_fc60);

    /// The op script reader's line counter (`0x0113fc5c`), which
    /// `op_line_length` advances.
    pub const SCRIPT_LINE: Global<u32> = Global::new(0x0113_fc5c);

    /// `0x007e1d50(file, src, out, pos, max, eol, flags)` (`op_token_get`):
    /// the next token into `out` ([`op_token`] on `src`, `*pos`, `max` and
    /// the line end `*eol`; a class-1 result is read again unless `flags &
    /// 2`). A `\` token at or past the line end (class 6) continues on
    /// the next line, unless `flags & 4`: the line counter advances
    /// (`op_line_length(file, SCRIPT_LINE)`), the next line is read into
    /// `src` (`op_read_line(file, src, max)`, its length to `*eol`) and
    /// `*pos` = 0. With `flags & 1` the token's length is returned.
    /// Otherwise further tokens are appended to `out` (each at its current
    /// end, with what room is left) until one adds nothing; a class-1
    /// token stops it, cut off again and `*pos` put back before it.
    /// Returns `out`'s length.
    #[allow(clippy::too_many_arguments)]
    pub fn op_token_get<M: Mem>(
        mem: &mut M,
        file: u32,
        src: u32,
        out: u32,
        pos: u32,
        max: u32,
        eol: u32,
        flags: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let backslash = Global::<u32>::new(0x00b8_f87c).ptr(mem).addr;
        let mut end;
        loop {
            end = mem.read::<u32>(eol);
            let mut class = op_token(mem, src, out, pos, max, end, call);
            if class == 1 && flags & 2 == 0 {
                class = op_token(mem, src, out, pos, max, end, call);
            }
            if mem.read::<i32>(pos) < end.cast_signed()
                || class != 6
                || out == 0
                || op_str_equal(mem, out, backslash, 0) == 0
            {
                break;
            }
            if flags & 4 != 0 {
                break;
            }
            let lines = SCRIPT_LINE.ptr(mem).addr;
            op_line_length(mem, file, lines);
            let n = op_read_line(mem, file, src, max);
            mem.write(eol, n);
            mem.write(pos, 0u32);
        }
        let mut len = if out == 0 { 0 } else { op_strlen(mem, out) };
        if flags & 1 != 0 {
            return len;
        }
        loop {
            let before: u32 = mem.read(pos);
            let class = op_token(
                mem,
                src,
                out.wrapping_add(len),
                pos,
                max.wrapping_sub(len),
                end,
                call,
            );
            let n = op_strlen(mem, out);
            if n == len {
                return n;
            }
            if class == 1 {
                mem.write(out.wrapping_add(len), 0u8);
                mem.write(pos, before);
                return len;
            }
            len = n;
        }
    }

    /// `op_token_007e1f80(src, out, pos, max, eol)` (`0x007e1f80`): the
    /// next token of an op script from `src + *pos` into `out` (at most
    /// `max - 1` bytes, then a terminator), by the class of its first byte:
    /// a run of class 1, a run of class 2 (bytes from 0x80 take the next
    /// one too), a quoted run up to the closing quote of the same class
    /// or a terminator, a run of spaces (5), or one byte (6, 8). Class 7:
    /// `;` and `//` skip to `eol + 1`; `/*` opens a block comment (depth
    /// up, `IN_COMMENT` set) and `*/` closes one (an error 3 through
    /// `op_error` when none is open); others keep the byte. Inside a
    /// comment the token is empty. `*pos` is left after the token (one
    /// byte back, as the original reads one ahead). Returns the first byte's
    /// class (sign-extended).
    #[allow(clippy::too_many_lines)]
    pub fn op_token<M: Mem>(
        mem: &mut M,
        src: u32,
        out: u32,
        pos: u32,
        max: u32,
        eol: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let class =
            |mem: &M, c: u8| mem.read::<u8>(CHAR_CLASS.ptr(mem).addr.wrapping_add(u32::from(c)));
        let max = max.cast_signed();
        let mut i: u32 = mem.read(pos);
        let mut o = out;
        let next = |mem: &M, i: &mut u32| {
            let c: u8 = mem.read(src.wrapping_add(*i));
            *i = i.wrapping_add(1);
            c
        };
        let mut c = next(mem, &mut i);
        let ch = class(mem, c);
        let mut n: i32 = 1;
        let put = |mem: &mut M, o: &mut u32, n: &mut i32, b: u8| {
            if *n < max {
                mem.write(*o, b);
                *o = o.wrapping_add(1);
                *n += 1;
            }
        };
        match ch {
            1 => loop {
                put(mem, &mut o, &mut n, c);
                c = next(mem, &mut i);
                if class(mem, c) != 1 {
                    break;
                }
            },
            2 => loop {
                put(mem, &mut o, &mut n, c);
                if c >= 0x80 {
                    let b = next(mem, &mut i);
                    put(mem, &mut o, &mut n, b);
                }
                c = next(mem, &mut i);
                if class(mem, c) != 2 {
                    break;
                }
            },
            3 | 4 => {
                let mut done = false;
                loop {
                    put(mem, &mut o, &mut n, c);
                    if c >= 0x80 {
                        let b = next(mem, &mut i);
                        put(mem, &mut o, &mut n, b);
                    }
                    c = next(mem, &mut i);
                    if c == 0 || done {
                        break;
                    }
                    if class(mem, c) == ch {
                        done = true;
                    }
                }
            }
            5 => {
                while c != 0 {
                    put(mem, &mut o, &mut n, c);
                    c = next(mem, &mut i);
                    if class(mem, c) != 5 {
                        break;
                    }
                }
            }
            6 | 8 => {
                if max > n {
                    mem.write(o, c);
                    o = o.wrapping_add(1);
                }
                i = i.wrapping_add(1);
            }
            7 => {
                mem.write(o, c);
                if c == b';' {
                    i = eol.wrapping_add(1);
                } else {
                    let b = next(mem, &mut i);
                    if c == b'/' {
                        if b == b'/' {
                            i = eol.wrapping_add(1);
                        } else if b == b'*' {
                            let d = COMMENT_DEPTH.get(mem);
                            COMMENT_DEPTH.set(mem, d.wrapping_add(1));
                            IN_COMMENT.set(mem, 1);
                            i = i.wrapping_add(1);
                        } else {
                            o = o.wrapping_add(1);
                        }
                    } else if b == b'/' {
                        let d = COMMENT_DEPTH.get(mem);
                        if d == 0 {
                            let f = Global::<u32>::new(0x007e_1450).ptr(mem).addr;
                            call(mem, f, &[3, 0]);
                        } else {
                            COMMENT_DEPTH.set(mem, d - 1);
                            if d == 1 {
                                IN_COMMENT.set(mem, 0);
                            }
                        }
                        i = i.wrapping_add(1);
                    } else {
                        o = o.wrapping_add(1);
                    }
                }
            }
            _ => {}
        }
        i = i.wrapping_sub(1);
        if IN_COMMENT.get(mem) != 0 {
            o = out;
        }
        mem.write(o, 0u8);
        mem.write(pos, i);
        i32::from(ch.cast_signed()).cast_unsigned()
    }
}
