//! The effect "op" layer's small services: the scratch stack, matrices,
//! the function ordering table, pad repeat, and the `oef2` cue lists.

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
    /// `g_OpPadActive`: `op_pad_set` does nothing while 0.
    pub const PAD_ACTIVE: Global<u8> = Global::new(0x0232_2790);
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
        if PAD_ACTIVE.get(mem) == 0 {
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
    /// past the last one left.
    pub fn op_oef2_tex_clut_ovl_clear(mem: &mut impl Mem, mask: u32) {
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
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 0..=64
        OEF2_CLUT_OVL_COUNT.set(mem, (last + 1) as u16);
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
}
