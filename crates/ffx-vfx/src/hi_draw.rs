//! The op layer's model draw set-ups at `0x00a68c90..0x00a6cd40`
//! (`op_md_chara_path3`, `op1_mdc_draw` and the `op1_md_draw` paths:
//! flag 4 with and without `0x300`, flag `0x2000`, flag `0x80` with and
//! without `0x300`, `op_md_one8` and `_fog`; `docs/formats/vfx.md`, "High
//! range"). Each path takes `0x1800` (or `0x2800`) bytes of the op scratch
//! stack as its work area `w`, fills the PS2 packet words there and pops
//! it again.

#![allow(clippy::many_single_char_names)]

/// The functions over game memory.
pub mod bridge {
    use ffx_bridge::{Mem, x87};

    use crate::op::bridge::{SPR_STACK, SPR_TOP};

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

    /// The emulated EE's `v0` (`g_Ee_Gpr + 0x20`), which the init block
    /// passes through.
    const EE_V0: u32 = 0x00c8_f528;
    /// `f64` 1/128 (`0x00b170b8`).
    const K_128TH: u32 = 0x00b1_70b8;
    /// `g_GsRegisterSink`: bit 8 set while the GS is busy.
    const GS_REGISTER_SINK: u32 = 0x0230_56e8;
    /// `g_OpMdGsWordsA_011a0740`: 16 bytes `0x00a6bfe0` copies to `w
    /// +0x140` for an init block without flag 8.
    const MD_GS_WORDS_A: u32 = 0x011a_0740;
    /// `g_OpMdGsWordsB_011a07f0`: the same with flag 8.
    const MD_GS_WORDS_B: u32 = 0x011a_07f0;

    const OP_FFXAMBIENT_TO_OMDAMBIENT: u32 = 0x00a6_8ff0;
    const OP_NORMAL_LIGHT44_FFX: u32 = 0x00a6_cf10;
    const OP_VERTEX_NORMAL_POINT44_FFX: u32 = 0x00a6_d380;
    const OP_MD_SPR_INIT_X64: u32 = 0x00a6_9940;
    const OP_MATRIX_LIGHT4_VECTOR: u32 = 0x00a6_cd80;
    const OP_MD_WHITE_VECTORS: u32 = 0x00a6_cf70;
    const OP_MD_LIGHT_00A6CD40: u32 = 0x00a6_cd40;
    const OPC: u32 = 0x00a6_d370;
    const OPC2: u32 = 0x007e_6b20;
    const OP_UNIT_M: u32 = 0x007e_a6d0;
    const OP_MUL_M: u32 = 0x007e_83b0;
    /// `op_md_packet_size_check_00a698b0` (a bare `xor eax, eax; ret`).
    const OP_MD_PACKET_SIZE_CHECK: u32 = 0x00a6_98b0;
    /// `f32` 1.00003052 (`0x3f800100`, `0x00b7c424`): the projection's
    /// `z` scale.
    const PERSP_Z: u32 = 0x00b7_c424;
    /// `f32` -2.00003052 (`0xc0000080`, `0x00b7c428`): its `z` offset.
    const PERSP_W: u32 = 0x00b7_c428;
    /// `g_OpMdGsWordsC_011a0820`: 16 bytes `0x00a6c3d0` copies to `w
    /// +0x140` (written by `0x007eb760`).
    const MD_GS_WORDS_C: u32 = 0x011a_0820;
    /// `g_OpMdPath3Block_012a8da8`: the address `0x00a6c3d0` leaves at `w
    /// +0x180`.
    const MD_PATH3_BLOCK: u32 = 0x012a_8da8;

    /// Pushes `0x1800` bytes on the op scratch stack ([`spr_push_n`]).
    fn spr_push(mem: &mut impl Mem, top_first: bool) -> u32 {
        spr_push_n(mem, top_first, 0x1800)
    }

    /// Pushes `size` bytes on the op scratch stack: `w` is the old top's
    /// mark, the next mark `w + size`. `top_first` stores the top byte
    /// before the mark (as `op_md_chara_path3` and `0x00a6bfe0` do).
    fn spr_push_n(mem: &mut impl Mem, top_first: bool, size: u32) -> u32 {
        // the index is the sign-extended top plus one, unwrapped
        let i = i32::from(SPR_TOP.get(mem)) + 1;
        let stack = SPR_STACK.ptr(mem).addr;
        let mark = |i: i32| stack.wrapping_add(i.cast_unsigned().wrapping_mul(4));
        #[allow(clippy::cast_possible_truncation)] // the byte `cl`
        let top = i as i8;
        if top_first {
            SPR_TOP.set(mem, top);
        }
        let w: u32 = mem.read(mark(i - 1));
        mem.write(mark(i), w.wrapping_add(size));
        if !top_first {
            SPR_TOP.set(mem, top);
        }
        w
    }

    /// Pops what [`spr_push`] pushed (`dec byte [g_OpSprTop]`).
    fn spr_pop(mem: &mut impl Mem) {
        let top = SPR_TOP.get(mem);
        SPR_TOP.set(mem, top.wrapping_sub(1));
    }

    /// Spins while the GS is busy (`g_GsRegisterSink` bit 8), as the
    /// original does; nothing on the PC sets it.
    fn gs_wait(mem: &impl Mem) {
        let sink = mem.rebase(GS_REGISTER_SINK);
        while mem.read::<u32>(sink) & 0x100 != 0 {}
    }

    /// The 0x50-byte init block copied to `w +0x300`, 16 bytes at a time
    /// through the EE's `v0` (all four words; the first word stored from
    /// the read, the others from `v0`).
    fn copy_init(mem: &mut impl Mem, init: u32, w: u32) {
        let v0 = mem.rebase(EE_V0);
        for row in 0..5u32 {
            let src = init.wrapping_add(16 * row);
            let dst = w.wrapping_add(0x300 + 16 * row);
            let x: u32 = mem.read(src);
            mem.write(v0, x);
            for k in 1..4u32 {
                let y: u32 = mem.read(src.wrapping_add(4 * k));
                mem.write(v0.wrapping_add(4 * k), y);
            }
            mem.write(dst, x);
            for k in 1..4u32 {
                let y: u32 = mem.read(v0.wrapping_add(4 * k));
                mem.write(dst.wrapping_add(4 * k), y);
            }
        }
    }

    /// `w +0x338` and `+0x33c` default to `obj + [obj +8]` and `obj +
    /// [obj +0xc]` when 0 (read in that order; stored `0x338` first when
    /// `first_338`).
    fn default_bases(mem: &mut impl Mem, obj: u32, w: u32, first_338: bool) {
        let mut a: u32 = mem.read(w.wrapping_add(0x338));
        if a == 0 {
            a = obj.wrapping_add(mem.read(obj.wrapping_add(8)));
        }
        let mut b: u32 = mem.read(w.wrapping_add(0x33c));
        if b == 0 {
            b = obj.wrapping_add(mem.read(obj.wrapping_add(0xc)));
        }
        if first_338 {
            mem.write(w.wrapping_add(0x338), a);
            mem.write(w.wrapping_add(0x33c), b);
        } else {
            mem.write(w.wrapping_add(0x33c), b);
            mem.write(w.wrapping_add(0x338), a);
        }
    }

    /// The colour `w +0x370..+0x37c` from the signed bytes at `from`
    /// (`fild`/`fstp m32`, exact).
    fn colour_from_bytes(mem: &mut impl Mem, from: u32, w: u32) {
        for k in 0..4u32 {
            let c = f32::from(mem.read::<i8>(from.wrapping_add(k)));
            mem.write(w.wrapping_add(0x370 + 4 * k), c);
        }
    }

    /// `w +0x390` = `(i16 obj +0x12 << shift) + [base]`, `w +0x394` =
    /// `[base]` (read twice).
    fn vertex_area(mem: &mut impl Mem, obj: u32, w: u32, base: u32, shift: u32) {
        let n = i32::from(mem.read::<i16>(obj.wrapping_add(0x12))).cast_unsigned();
        let at: u32 = mem.read(base);
        mem.write(w.wrapping_add(0x390), (n << shift).wrapping_add(at));
        let at: u32 = mem.read(base);
        mem.write(w.wrapping_add(0x394), at);
    }

    /// The texture words of a textured tag at `tag`: `w +0x120` = `u16
    /// init +0xc + [tag +8]`, `w +0x124` = `(u16 init +0xe << 5) + [tag
    /// +0xc]`.
    fn tex_words(mem: &mut impl Mem, w: u32, init: u32, tag: u32) {
        let a = u32::from(mem.read::<u16>(init.wrapping_add(0xc)));
        let a = a.wrapping_add(mem.read(tag.wrapping_add(8)));
        mem.write(w.wrapping_add(0x120), a);
        let b = u32::from(mem.read::<u16>(init.wrapping_add(0xe))) << 5;
        let b = b.wrapping_add(mem.read(tag.wrapping_add(0xc)));
        mem.write(w.wrapping_add(0x124), b);
    }

    /// `w +0x1f0` = 0x8001, `w +0x1f8` = `mode`.
    fn tag_mode(mem: &mut impl Mem, w: u32, mode: u32) {
        mem.write(w.wrapping_add(0x1f0), 0x8001u32);
        mem.write(w.wrapping_add(0x1f8), mode);
    }

    /// `op_md_chara_path3(obj, init, chr)` (`0x00a6a3a0`): the set-up of a
    /// character effect model's draw. Pushes the work area `w`, copies the
    /// init block to `w +0x300` ([`copy_init`]), defaults `w +0x338`/`+0x33c`
    /// to the packet's offsets; with `a = op_ffxambient_to_omdambient()`'s
    /// RGBA bytes and `t = f32(byte w +0x34c x 1/128)`, each colour
    /// channel `w +0x370 + 4k` is `f32(byte w +0x31c + k x (1 - t) + a[k]
    /// x t)`. `w +0x390` = `(i16 obj +0x12 << 5) + [w +0x304]`, `+0x394` =
    /// `[w +0x304]`. Flag `0x40` of `[w +0x300]`:
    /// `op_vertex_normal_point44_ffx(obj, w + 0x300, w, chr)`; else, with
    /// `i16 obj +0x14` nonzero and no flag `0x20`,
    /// `op_normal_light44_ffx(obj, w + 0x300, w)`. Pops `w`; returns 0.
    /// Callees through `call`.
    pub fn op_md_chara_path3<M: Mem>(
        mem: &mut M,
        obj: u32,
        init: u32,
        chr: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let w = spr_push(mem, true);
        copy_init(mem, init, w);
        default_bases(mem, obj, w, true);
        let ambient = go(mem, OP_FFXAMBIENT_TO_OMDAMBIENT, &[], call).to_le_bytes();
        let k: f64 = mem.read(mem.rebase(K_128TH));
        let t = f64::from(mem.read::<u8>(w.wrapping_add(0x34c))) * k;
        // rounded to f32 through a local, exact (a byte / 128)
        let t = x87::load(x87::store(t));
        let s = 1.0 - t;
        for (i, &a) in (0u32..).zip(&ambient) {
            let own = f64::from(mem.read::<u8>(w.wrapping_add(0x31c + i)));
            // a byte times a multiple of 1/128 plus another: exact in f64
            // and in f32, whatever the order
            let c = own * s + f64::from(a) * t;
            mem.write(w.wrapping_add(0x370 + 4 * i), x87::store(c));
        }
        let model = w.wrapping_add(0x300);
        vertex_area(mem, obj, w, w.wrapping_add(0x304), 5);
        let flags: u32 = mem.read(model);
        if flags & 0x40 != 0 {
            go(
                mem,
                OP_VERTEX_NORMAL_POINT44_FFX,
                &[obj, model, w, chr],
                call,
            );
        } else if mem.read::<i16>(obj.wrapping_add(0x14)) != 0 && flags & 0x20 == 0 {
            go(mem, OP_NORMAL_LIGHT44_FFX, &[obj, model, w], call);
        }
        spr_pop(mem);
        0
    }

    /// `op_md_x64_00a6b710(obj, init)` (`0x00a6b710`; from `op1_md_draw`
    /// for packets with flag 4 and none of `0x300`, and from `0x00a69300`):
    /// pushes the work area `w`; `p = op_md_spr_init_x64(obj, init, w)`
    /// (`w + 0x300`); the colour `w +0x370..` from the signed bytes `p
    /// +0x1c..`; `w +0x390` = `(i16 obj +0x12 << 4) + [p +4]`, `+0x394` =
    /// `[p +4]`. The lights by `[p]`: without flag `0x40` and with `i16
    /// obj +0x14` nonzero, `op_matrix_light4_vector(local, [p +0x28], [p
    /// +0x2c])` (the local at `frame - 0x44`, never read) or with flag
    /// `0x20` `op_md_white_vectors_00a6cf70(obj, p, w)`; with flag `0x40`,
    /// `opc(obj, p, w)` when neither `[obj]` nor `[p]` has flag `0x400`,
    /// else `op_md_light_00a6cd40(obj, p, w)` unless `[obj]` has flag 8.
    /// Then the tags from `obj + [obj +4]` (type `i8 +1`): 0/1, 2/3, 4/5
    /// and 6/7 set the mode words (2/3 and 6/7 the texture words) and step
    /// 0x10; 0x20 and 0x22 step 0x20 and act only when `i16 +2` is
    /// nonzero, 0x22 filling the GS words `w +0x100..+0x168` and calling
    /// `opc2()`; any other type ends. Waits for the GS, pops `w` and
    /// returns `[w +0x334]`. Callees through `call`.
    pub fn op_md_x64<M: Mem>(
        mem: &mut M,
        obj: u32,
        init: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let w = spr_push(mem, false);
        let p = go(mem, OP_MD_SPR_INIT_X64, &[obj, init, w], call);
        colour_from_bytes(mem, p.wrapping_add(0x1c), w);
        vertex_area(mem, obj, w, p.wrapping_add(4), 4);
        let flags: u32 = mem.read(p);
        if flags & 0x40 == 0 {
            if mem.read::<i16>(obj.wrapping_add(0x14)) != 0 {
                if flags & 0x20 == 0 {
                    let m: u32 = mem.read(p.wrapping_add(0x28));
                    let c: u32 = mem.read(p.wrapping_add(0x2c));
                    let local = frame.wrapping_sub(0x44);
                    go(mem, OP_MATRIX_LIGHT4_VECTOR, &[local, m, c], call);
                } else {
                    go(mem, OP_MD_WHITE_VECTORS, &[obj, p, w], call);
                }
            }
        } else {
            let own: u32 = mem.read(obj);
            if (own | flags) & 0x400 == 0 {
                go(mem, OPC, &[obj, p, w], call);
            } else if own & 8 == 0 {
                go(mem, OP_MD_LIGHT_00A6CD40, &[obj, p, w], call);
            }
        }
        let mut tag = obj.wrapping_add(mem.read(obj.wrapping_add(4)));
        loop {
            let ty = mem.read::<i8>(tag.wrapping_add(1));
            let step = match ty {
                0 | 1 => {
                    tag_mode(mem, w, 0x0041_4141);
                    0x10
                }
                2 | 3 | 6 | 7 => {
                    tag_mode(mem, w, 0x1241_2412);
                    tex_words(mem, w, p, tag);
                    0x10
                }
                4 | 5 => {
                    tag_mode(mem, w, 0x4141_4141);
                    0x10
                }
                0x20 => {
                    if mem.read::<i16>(tag.wrapping_add(2)) != 0 {
                        tag_mode(mem, w, 0x0041_4141);
                    }
                    0x20
                }
                0x22 => {
                    if mem.read::<i16>(tag.wrapping_add(2)) != 0 {
                        tag_path3(mem, w, p, tag);
                        go(mem, OPC2, &[], call);
                    }
                    0x20
                }
                _ => break,
            };
            tag = tag.wrapping_add(step);
        }
        gs_wait(mem);
        spr_pop(mem);
        mem.read(w.wrapping_add(0x334))
    }

    /// A type-0x22 tag's GS words for `opc2`.
    fn tag_path3(mem: &mut impl Mem, w: u32, p: u32, tag: u32) {
        tag_mode(mem, w, 0x1241_2412);
        let a = u32::from(mem.read::<u16>(p.wrapping_add(0xc)));
        let a = a.wrapping_add(mem.read(tag.wrapping_add(8)));
        mem.write(w.wrapping_add(0x120), a);
        let b = u32::from(mem.read::<u16>(p.wrapping_add(0xe))) << 5;
        let b = b.wrapping_add(mem.read(tag.wrapping_add(0xc)));
        mem.write(w.wrapping_add(0x100), 0x7000_0006u32);
        mem.write(w.wrapping_add(0x124), b);
        mem.write(w.wrapping_add(0x110), 0x8001u32);
        let x: u32 = mem.read(tag.wrapping_add(0x18));
        mem.write(w.wrapping_add(0x150), x);
        let mut y: u32 = mem.read(tag.wrapping_add(0x10));
        if y == 0 {
            y = mem.read(w.wrapping_add(0x120));
        }
        mem.write(w.wrapping_add(0x160), y);
        mem.write(w.wrapping_add(0x168), 6u32);
    }

    /// `op_md_x64_fog_00a6bfe0(obj, init)` (`0x00a6bfe0`; from
    /// `op1_md_draw` for packets with flag 4 and one of `0x300`): pushes
    /// the work area `w`, copies the init block ([`copy_init`]), defaults
    /// `w +0x338`/`+0x33c`, the colour `w +0x370..` from the signed bytes
    /// `w +0x31c..`; `w +0x394` = `[w +0x304]`, `w +0x39c` = `(i16 obj
    /// +0x12 << 4) + [w +0x304]`. The GS words: `+0x100` 0x70000005,
    /// `+0x108` 0, `+0x110` 0x8001, `+0x118` 0xeeee, `+0x128` 6, `+0x130`
    /// and `+0x134` the bytes `w +0x308`/`+0x309` (or, when `u16 w +0x308`
    /// is 0, `obj +0x16`/`+0x17`), `+0x138` 0x42, `+0x140..+0x14c` 16
    /// bytes from `0x011a07f0` with init flag 8 else `0x011a0740`, `+0x150`
    /// 0 with `[obj]` flag 4 else 5, `+0x158` 8. Then the tags from `obj +
    /// [obj +4]` (type `i8 +1`, step 0x10): 0 and 4 set the mode words, 2
    /// and 6 the mode and texture words (`u16 w +0x30c`/`+0x30e`); any
    /// other type ends. Waits for the GS, pops `w`; returns `[w +0x334]`.
    pub fn op_md_x64_fog(mem: &mut impl Mem, obj: u32, init: u32) -> u32 {
        let w = spr_push(mem, true);
        copy_init(mem, init, w);
        default_bases(mem, obj, w, false);
        colour_from_bytes(mem, w.wrapping_add(0x31c), w);
        let base: u32 = mem.read(w.wrapping_add(0x304));
        mem.write(w.wrapping_add(0x394), base);
        let n = i32::from(mem.read::<i16>(obj.wrapping_add(0x12))).cast_unsigned();
        let base: u32 = mem.read(w.wrapping_add(0x304));
        mem.write(w.wrapping_add(0x39c), (n << 4).wrapping_add(base));
        let tags = obj.wrapping_add(mem.read(obj.wrapping_add(4)));
        for (at, v) in [
            (0x100u32, 0x7000_0005u32),
            (0x108, 0),
            (0x110, 0x8001),
            (0x118, 0xeeee),
            (0x128, 6),
        ] {
            mem.write(w.wrapping_add(at), v);
        }
        let from = if mem.read::<u16>(w.wrapping_add(0x308)) == 0 {
            obj.wrapping_add(0x16)
        } else {
            w.wrapping_add(0x308)
        };
        let a = u32::from(mem.read::<u8>(from));
        mem.write(w.wrapping_add(0x130), a);
        let b = u32::from(mem.read::<u8>(from.wrapping_add(1)));
        mem.write(w.wrapping_add(0x134), b);
        mem.write(w.wrapping_add(0x138), 0x42u32);
        let words = if mem.read::<u8>(w.wrapping_add(0x300)) & 8 == 0 {
            MD_GS_WORDS_A
        } else {
            MD_GS_WORDS_B
        };
        let words = mem.rebase(words);
        for k in 0..4u32 {
            let x: u32 = mem.read(words.wrapping_add(4 * k));
            mem.write(w.wrapping_add(0x140 + 4 * k), x);
        }
        let alpha = if mem.read::<u8>(obj) & 4 == 0 {
            5u32
        } else {
            0
        };
        mem.write(w.wrapping_add(0x150), alpha);
        mem.write(w.wrapping_add(0x158), 8u32);
        let init = w.wrapping_add(0x300);
        let mut tag = tags;
        loop {
            match mem.read::<i8>(tag.wrapping_add(1)) {
                0 => tag_mode(mem, w, 0x0041_4141),
                4 => tag_mode(mem, w, 0x4141_4141),
                2 | 6 => {
                    tag_mode(mem, w, 0x1241_2412);
                    tex_words(mem, w, init, tag);
                }
                _ => break,
            }
            tag = tag.wrapping_add(0x10);
        }
        gs_wait(mem);
        spr_pop(mem);
        mem.read(w.wrapping_add(0x334))
    }

    /// The bytes `w +0x130`/`+0x134`: `w +0x308`/`+0x309`, or `obj
    /// +0x16`/`+0x17` when `u16 w +0x308` is 0.
    fn fog_bytes(mem: &mut impl Mem, obj: u32, w: u32) {
        let from = if mem.read::<u16>(w.wrapping_add(0x308)) == 0 {
            obj.wrapping_add(0x16)
        } else {
            w.wrapping_add(0x308)
        };
        let a = u32::from(mem.read::<u8>(from));
        mem.write(w.wrapping_add(0x130), a);
        let b = u32::from(mem.read::<u8>(from.wrapping_add(1)));
        mem.write(w.wrapping_add(0x134), b);
    }

    /// 16 bytes from the table at `sym` to `w + at`.
    fn copy_words(mem: &mut impl Mem, sym: u32, w: u32, at: u32) {
        let words = mem.rebase(sym);
        for k in 0..4u32 {
            let x: u32 = mem.read(words.wrapping_add(4 * k));
            mem.write(w.wrapping_add(at + 4 * k), x);
        }
    }

    /// 5 without flag 4 in the packet's first byte, else 0.
    fn alpha_test(mem: &impl Mem, obj: u32) -> u32 {
        if mem.read::<u8>(obj) & 4 == 0 { 5 } else { 0 }
    }

    /// `op_md_persp_00a6bae0(obj, init)` (`0x00a6bae0`; `op1_md_draw`'s
    /// path for packets with flag `0x2000`): pushes the work area `w`,
    /// copies the init block ([`copy_init`]), defaults `w +0x338`/`+0x33c`
    /// (`+0x33c` stored first), the colour `w +0x370..` from the signed
    /// bytes `w +0x31c..`. The matrix `w +0x200`: `op_unit_m`, then `[0][0]`
    /// and `[1][1]` 1, `[2][2]` 1.00003052 and `[3][2]` -2.00003052 (both
    /// from `.rdata`), `[2][3]` 1, `[3][3]` 0 (a projection), times `[w
    /// +0x330]` (`op_mul_m(m, m, [w +0x330])`). `w +0x390` = `[w +0x304]`,
    /// `+0x394` = `(i16 obj +0x12 << 4) + [w +0x304]`, `+0x398` the same `<<
    /// 5`; the 64 bytes at `[w +0x320]` to `w +0x240`. The GS words as
    /// [`op_md_x64_fog`] writes them. Tags from `obj + [obj +4]` (type `i8
    /// +1`, step 0x10; jumps `0x00a6bfb0`): 0, 1, 4, 5 set `w +0x1f0` and
    /// `+0x280` 0x8000, `+0x1f8` and `+0x288` 0x41; 2, 3, 6, 7 the same with
    /// 0x412 and the texture words; anything else ends. Waits for the GS,
    /// pops `w`; returns `[w +0x334]`. Callees through `call`.
    pub fn op_md_persp<M: Mem>(
        mem: &mut M,
        obj: u32,
        init: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let w = spr_push(mem, true);
        copy_init(mem, init, w);
        default_bases(mem, obj, w, false);
        colour_from_bytes(mem, w.wrapping_add(0x31c), w);
        let m = w.wrapping_add(0x200);
        go(mem, OP_UNIT_M, &[m], call);
        let one = 1.0f32.to_bits();
        let z = x87::round_trip(f32::from_bits(mem.read(mem.rebase(PERSP_Z))));
        let zw = x87::round_trip(f32::from_bits(mem.read(mem.rebase(PERSP_W))));
        for (at, v) in [
            (0x00u32, one),
            (0x14, one),
            (0x28, z.to_bits()),
            (0x38, zw.to_bits()),
            (0x2c, one),
            (0x3c, 0),
        ] {
            mem.write(m.wrapping_add(at), v);
        }
        let view: u32 = mem.read(w.wrapping_add(0x330));
        go(mem, OP_MUL_M, &[m, m, view], call);
        let base: u32 = mem.read(w.wrapping_add(0x304));
        mem.write(w.wrapping_add(0x390), base);
        let n = i32::from(mem.read::<i16>(obj.wrapping_add(0x12))).cast_unsigned();
        let base: u32 = mem.read(w.wrapping_add(0x304));
        mem.write(w.wrapping_add(0x394), (n << 4).wrapping_add(base));
        let n = i32::from(mem.read::<i16>(obj.wrapping_add(0x12))).cast_unsigned();
        let base: u32 = mem.read(w.wrapping_add(0x304));
        mem.write(w.wrapping_add(0x398), (n << 5).wrapping_add(base));
        let src: u32 = mem.read(w.wrapping_add(0x320));
        for k in 0..16u32 {
            let x: u32 = mem.read(src.wrapping_add(4 * k));
            mem.write(w.wrapping_add(0x240 + 4 * k), x);
        }
        let tags = obj.wrapping_add(mem.read(obj.wrapping_add(4)));
        for (at, v) in [
            (0x100u32, 0x7000_0005u32),
            (0x108, 0),
            (0x110, 0x8001),
            (0x118, 0xeeee),
            (0x128, 6),
        ] {
            mem.write(w.wrapping_add(at), v);
        }
        fog_bytes(mem, obj, w);
        mem.write(w.wrapping_add(0x138), 0x42u32);
        let words = if mem.read::<u8>(w.wrapping_add(0x300)) & 8 == 0 {
            MD_GS_WORDS_A
        } else {
            MD_GS_WORDS_B
        };
        copy_words(mem, words, w, 0x140);
        let alpha = alpha_test(mem, obj);
        mem.write(w.wrapping_add(0x150), alpha);
        mem.write(w.wrapping_add(0x158), 8u32);
        let init = w.wrapping_add(0x300);
        let mut tag = tags;
        loop {
            let mode = match mem.read::<i8>(tag.wrapping_add(1)) {
                0 | 1 | 4 | 5 => 0x41u32,
                2 | 3 | 6 | 7 => 0x412,
                _ => break,
            };
            for at in [0x1f0u32, 0x280] {
                mem.write(w.wrapping_add(at), 0x8000u32);
                mem.write(w.wrapping_add(at + 8), mode);
            }
            if mode == 0x412 {
                tex_words(mem, w, init, tag);
            }
            tag = tag.wrapping_add(0x10);
        }
        gs_wait(mem);
        spr_pop(mem);
        mem.read(w.wrapping_add(0x334))
    }

    /// `op_md_flag80_fog_00a6c3d0(obj, init)` (`0x00a6c3d0`; `op1_md_draw`'s
    /// path for packets with flag `0x80` and one of `0x300`): when
    /// `op_md_packet_size_check(obj, [init +0x34], 1)` (always 0) is
    /// nonzero, returns `[init +0x34]`. Else pushes `0x2800` bytes as the
    /// work area `w`, copies the init block ([`copy_init`]), rounds `w
    /// +0x334` up to 64 (`& 0x0fffffc0`), defaults `w +0x338`/`+0x33c`
    /// (`+0x33c` stored first), the colour `w +0x370..` from the signed
    /// bytes `w +0x31c..`; `w +0x394` = `[w +0x304]`, `+0x39c` = `(i16 obj
    /// +0x12 << 4) + [w +0x304]`. The GS words: `+0x110` 0x8001, `+0x118`
    /// 0xeeeee, `+0x128` 6, `+0x130`/`+0x134` ([`fog_bytes`]), `+0x138` 0x42,
    /// `+0x140..` 16 bytes from `0x011a0820`, `+0x150..` 16 bytes from
    /// `0x011a0740`, `+0x160` 0 with `[obj]` flag 4 else 5, `+0x168` 8,
    /// `+0x180` the address `0x012a8da8`. Waits for the GS; the tags from
    /// `obj + [obj +4]` (type `i8 +1`, step 0x10; jumps `0x00a6c88c`): 0
    /// and 4 set `w +0x100` 0x20000009 / 0x2000000b, `+0x1f0` 0x8001,
    /// `+0x1f8` 0x414141ee; 2 and 6 `+0x100` 0x2000000d / 0x20000010,
    /// `+0x1f8` 0x12412eee, the texture words and `+0x128` 6; each waits for
    /// the GS. 1, 3, 5 and anything else end. Pops `w`; returns
    /// `[w +0x334] + 0x40`. Callees through `call`.
    pub fn op_md_flag80_fog<M: Mem>(
        mem: &mut M,
        obj: u32,
        init: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let w = match setup_2800(mem, obj, init, call) {
            Ok(w) => w,
            Err(early) => return early,
        };
        fog_vertex_area(mem, obj, w);
        let tags = obj.wrapping_add(mem.read(obj.wrapping_add(4)));
        gs_words_2800(mem, obj, w, false);
        gs_wait(mem);
        tags_80(mem, w, tags, false);
        spr_pop(mem);
        let end: u32 = mem.read(w.wrapping_add(0x334));
        end.wrapping_add(0x40)
    }

    /// `dbgPrintf` and `debug_exit_trace` (both bare `ret`s here).
    const DBG_PRINTF: u32 = 0x0062_f500;
    const DEBUG_EXIT_TRACE: u32 = 0x0062_f510;
    /// `"one8 draw mode unsuport light program ---..."` (`0x00b7c3e0`).
    const ONE8_MSG: u32 = 0x00b7_c3e0;
    /// `"(op)\top_md0_onef() light no support !\n"` (`0x00b7c42c`).
    const ONE8_FOG_MSG: u32 = 0x00b7_c42c;

    /// The start the `0x2800`-byte paths share: `Err([init +0x34])` when
    /// `op_md_packet_size_check(obj, [init +0x34], 1)` (always 0) is
    /// nonzero; else pushes the work area `w` (top byte first), copies the
    /// init block ([`copy_init`]), rounds `w +0x334` up to 64 (`&
    /// 0x0fffffc0`), defaults `w +0x338`/`+0x33c` (`+0x33c` stored first)
    /// and sets the colour from the signed bytes `w +0x31c..`.
    fn setup_2800<M: Mem>(
        mem: &mut M,
        obj: u32,
        init: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> Result<u32, u32> {
        let size: u32 = mem.read(init.wrapping_add(0x34));
        if go(mem, OP_MD_PACKET_SIZE_CHECK, &[obj, size, 1], call) != 0 {
            return Err(mem.read(init.wrapping_add(0x34)));
        }
        let w = spr_push_n(mem, true, 0x2800);
        copy_init(mem, init, w);
        let end: u32 = mem.read(w.wrapping_add(0x334));
        mem.write(w.wrapping_add(0x334), end.wrapping_add(0x3f) & 0x0fff_ffc0);
        default_bases(mem, obj, w, false);
        colour_from_bytes(mem, w.wrapping_add(0x31c), w);
        Ok(w)
    }

    /// The fog paths' vertex words: `w +0x394` = `[w +0x304]`, `+0x39c` =
    /// `(i16 obj +0x12 << 4) + [w +0x304]` (read twice).
    fn fog_vertex_area(mem: &mut impl Mem, obj: u32, w: u32) {
        let base: u32 = mem.read(w.wrapping_add(0x304));
        mem.write(w.wrapping_add(0x394), base);
        let n = i32::from(mem.read::<i16>(obj.wrapping_add(0x12))).cast_unsigned();
        let base: u32 = mem.read(w.wrapping_add(0x304));
        mem.write(w.wrapping_add(0x39c), (n << 4).wrapping_add(base));
    }

    /// The GS words of the `0x2800`-byte paths: `+0x110` 0x8001, `+0x118`
    /// 0xeeeee, `+0x128` 6, `+0x130`/`+0x134` ([`fog_bytes`]), `+0x138`
    /// 0x42, `+0x140` 16 bytes from `0x011a0820`, `+0x150` 16 bytes from
    /// `0x011a0740`, `+0x160` 0 with `[obj]` flag 4 else 5, `+0x168` 8,
    /// `+0x180` the address `0x012a8da8`. `one8` (`op_md_one8` and
    /// `_fog`) also sets `+0x100` 0x20000000, `+0x108` 0, `+0x2e0` 0x8001
    /// and `+0x2e8` 0xe, and takes `+0x150` from `0x011a07f0` with init
    /// flag 8.
    fn gs_words_2800(mem: &mut impl Mem, obj: u32, w: u32, one8: bool) {
        if one8 {
            mem.write(w.wrapping_add(0x100), 0x2000_0000u32);
            mem.write(w.wrapping_add(0x108), 0u32);
        }
        mem.write(w.wrapping_add(0x110), 0x8001u32);
        mem.write(w.wrapping_add(0x118), 0x000e_eeeeu32);
        mem.write(w.wrapping_add(0x128), 6u32);
        fog_bytes(mem, obj, w);
        mem.write(w.wrapping_add(0x138), 0x42u32);
        copy_words(mem, MD_GS_WORDS_C, w, 0x140);
        let words = if one8 && mem.read::<u8>(w.wrapping_add(0x300)) & 8 != 0 {
            MD_GS_WORDS_B
        } else {
            MD_GS_WORDS_A
        };
        copy_words(mem, words, w, 0x150);
        let alpha = alpha_test(mem, obj);
        mem.write(w.wrapping_add(0x160), alpha);
        mem.write(w.wrapping_add(0x168), 8u32);
        if one8 {
            mem.write(w.wrapping_add(0x2e0), 0x8001u32);
            mem.write(w.wrapping_add(0x2e8), 0xeu32);
        }
        let block = mem.rebase(MD_PATH3_BLOCK);
        mem.write(w.wrapping_add(0x180), block);
    }

    /// The tags of the flag-`0x80` paths from `tags` (type `i8 +1`, step
    /// 0x10): 0 and 4 set `w +0x100` 0x20000009 / 0x2000000b, `+0x1f0`
    /// 0x8001, `+0x1f8` 0x414141ee; 2 and 6 `+0x100` 0x2000000d /
    /// 0x20000010, `+0x1f8` 0x12412eee, the texture words and `+0x128` 6;
    /// each waits for the GS. With `pairs` the odd types act as the even
    /// one below them (`0x00a6acb0`), else they end the chain
    /// (`0x00a6c3d0`); 8 and up or negative end it.
    fn tags_80(mem: &mut impl Mem, w: u32, tags: u32, pairs: bool) {
        let init = w.wrapping_add(0x300);
        let mut tag = tags;
        loop {
            let ty = mem.read::<i8>(tag.wrapping_add(1));
            if !(0..8).contains(&ty) || (!pairs && ty & 1 != 0) {
                break;
            }
            let (gif, mode, textured) = match ty >> 1 {
                0 => (0x2000_0009u32, 0x4141_41eeu32, false),
                1 => (0x2000_000d, 0x1241_2eee, true),
                2 => (0x2000_000b, 0x4141_41ee, false),
                _ => (0x2000_0010, 0x1241_2eee, true),
            };
            mem.write(w.wrapping_add(0x100), gif);
            tag_mode(mem, w, mode);
            if textured {
                tex_words(mem, w, init, tag);
                mem.write(w.wrapping_add(0x128), 6u32);
            }
            gs_wait(mem);
            tag = tag.wrapping_add(0x10);
        }
    }

    /// The lights of `0x00a6acb0` and `op_md_one8` by `[p]` (`p = w
    /// +0x300`): with flag `0x40`, `opc(obj, p, w)` (a bare `ret`; `one8`
    /// first sets `[p +0x44]` to `[p +0x20]` when it is 0); else, with `i16
    /// obj +0x14` nonzero and no flag `0x20`, `op_matrix_light4_vector(local,
    /// [p +0x28], [p +0x2c])` into the local at `frame - 0x44`, which
    /// nothing reads.
    fn lights_2800<M: Mem>(
        mem: &mut M,
        obj: u32,
        w: u32,
        frame: u32,
        one8: bool,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let p = w.wrapping_add(0x300);
        let flags: u32 = mem.read(p);
        if flags & 0x40 != 0 {
            if one8 && mem.read::<u32>(p.wrapping_add(0x44)) == 0 {
                let x: u32 = mem.read(p.wrapping_add(0x20));
                mem.write(p.wrapping_add(0x44), x);
            }
            go(mem, OPC, &[obj, p, w], call);
        } else if mem.read::<i16>(obj.wrapping_add(0x14)) != 0 && flags & 0x20 == 0 {
            let m: u32 = mem.read(p.wrapping_add(0x28));
            let c: u32 = mem.read(p.wrapping_add(0x2c));
            let local = frame.wrapping_sub(0x44);
            go(mem, OP_MATRIX_LIGHT4_VECTOR, &[local, m, c], call);
        }
    }

    /// `op_md_flag80_00a6acb0(obj, init)` (`0x00a6acb0`; `op1_md_draw`'s
    /// path for packets with flag `0x80` and none of `0x300`): as
    /// [`op_md_flag80_fog`] (`[init +0x34]` back when the size check
    /// fires, the `0x2800`-byte work area, the GS words, a GS wait), but
    /// `w +0x390` = `(i16 obj +0x12 << 4) + [w +0x304]` and `+0x394` = `[w
    /// +0x304]`, the lights ([`lights_2800`], the local at `frame -
    /// 0x44`), and types 1, 3, 5, 7 act as 0, 2, 4, 6 (jumps
    /// `0x00a6b1f8`). Pops `w`; returns `[w +0x334]`. Callees through
    /// `call`.
    pub fn op_md_flag80<M: Mem>(
        mem: &mut M,
        obj: u32,
        init: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let w = match setup_2800(mem, obj, init, call) {
            Ok(w) => w,
            Err(early) => return early,
        };
        vertex_area(mem, obj, w, w.wrapping_add(0x304), 4);
        lights_2800(mem, obj, w, frame, false, call);
        let tags = obj.wrapping_add(mem.read(obj.wrapping_add(4)));
        gs_words_2800(mem, obj, w, false);
        gs_wait(mem);
        tags_80(mem, w, tags, true);
        spr_pop(mem);
        mem.read(w.wrapping_add(0x334))
    }

    /// The tags of `op_md_one8` and `_fog` from `tags` (type `i8 +1`, step
    /// 0x10): `w +0x1f0` 0x8001 and `+0x1f8` 0x414141 (0, 1), 0x41414141 (4,
    /// 5) or 0x12412412 with the texture words (2, 3, 6, 7). An odd type
    /// ends the chain with `dbgPrintf(msg)` and `debug_exit_trace(1)`
    /// (`op_md_one8` after its mode words, `_fog` with none: `fog_odd`); 8
    /// and up or negative end it.
    fn tags_one8<M: Mem>(
        mem: &mut M,
        w: u32,
        tags: u32,
        fog_odd: bool,
        msg: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let init = w.wrapping_add(0x300);
        let mut tag = tags;
        loop {
            let ty = mem.read::<i8>(tag.wrapping_add(1));
            if !(0..8).contains(&ty) {
                return;
            }
            if !(fog_odd && ty & 1 != 0) {
                match ty >> 1 {
                    0 => tag_mode(mem, w, 0x0041_4141),
                    2 => tag_mode(mem, w, 0x4141_4141),
                    _ => {
                        tag_mode(mem, w, 0x1241_2412);
                        tex_words(mem, w, init, tag);
                    }
                }
            }
            if ty & 1 != 0 {
                let msg = mem.rebase(msg);
                go(mem, DBG_PRINTF, &[msg], call);
                go(mem, DEBUG_EXIT_TRACE, &[1], call);
                return;
            }
            tag = tag.wrapping_add(0x10);
        }
    }

    /// `op_md_one8(obj, init)` (`0x00a6b220`; `op1_md_draw`'s path for
    /// packets with none of flags `0x2000`, 4, `0x80`, `0x300`): the
    /// `0x2800`-byte start ([`setup_2800`]; `[init +0x34]` back when the
    /// size check fires), `w +0x390` = `(i16 obj +0x12 << 4) + [w +0x304]`
    /// and `+0x394` = `[w +0x304]`, the lights ([`lights_2800`]), the GS
    /// words ([`gs_words_2800`] with the `one8` extras), then the tags
    /// ([`tags_one8`], jumps `0x00a6b6f0`; an odd type reports
    /// `0x00b7c3e0`). No GS wait. Pops `w`; returns `[w +0x334] + 0x40`.
    /// Callees through `call`.
    pub fn op_md_one8<M: Mem>(
        mem: &mut M,
        obj: u32,
        init: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let w = match setup_2800(mem, obj, init, call) {
            Ok(w) => w,
            Err(early) => return early,
        };
        vertex_area(mem, obj, w, w.wrapping_add(0x304), 4);
        lights_2800(mem, obj, w, frame, true, call);
        let tags = obj.wrapping_add(mem.read(obj.wrapping_add(4)));
        gs_words_2800(mem, obj, w, true);
        tags_one8(mem, w, tags, false, ONE8_MSG, call);
        spr_pop(mem);
        let end: u32 = mem.read(w.wrapping_add(0x334));
        end.wrapping_add(0x40)
    }

    /// `op_md_one8_fog(obj, init)` (`0x00a6c8b0`; `op1_md_draw`'s path for
    /// packets with one of `0x300` and none of `0x2000`, 4, `0x80`): as
    /// [`op_md_one8`] without the lights, with the fog paths' vertex words
    /// (`w +0x394` = `[w +0x304]`, `+0x39c` = `(i16 obj +0x12 << 4) + [w
    /// +0x304]`); an odd type sets no mode words and reports `0x00b7c42c`
    /// (jumps `0x00a6cd18`). Returns `[w +0x334] + 0x40`. Callees through
    /// `call`.
    pub fn op_md_one8_fog<M: Mem>(
        mem: &mut M,
        obj: u32,
        init: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let w = match setup_2800(mem, obj, init, call) {
            Ok(w) => w,
            Err(early) => return early,
        };
        fog_vertex_area(mem, obj, w);
        let tags = obj.wrapping_add(mem.read(obj.wrapping_add(4)));
        gs_words_2800(mem, obj, w, true);
        tags_one8(mem, w, tags, true, ONE8_FOG_MSG, call);
        spr_pop(mem);
        let end: u32 = mem.read(w.wrapping_add(0x334));
        end.wrapping_add(0x40)
    }

    /// `get_ptr32_func` (`0x00a44650`): returns its argument.
    fn ptr32(p: u32) -> u32 {
        ffx_system::stub::ptr32(p)
    }

    /// `f64` 1000 (`0x00b0dba0`) and 0.001 as an `f32` widened
    /// (`0x00b47c68`): the scales round `op1_mdc_draw`'s product.
    const K_1000: u32 = 0x00b0_dba0;
    const K_1000TH: u32 = 0x00b4_7c68;
    const OP_M_COPY: u32 = 0x007e_8010;
    const OP_MD_CHARA_PATH3: u32 = 0x00a6_a3a0;
    /// `g_Op3dListener`, the matrix used when `op1_mdc_draw` gets none.
    const OP_3D_LISTENER: u32 = 0x0113_fcd0;

    /// The translation `+0x30..+0x38` of the matrix `[entry +4]` times `k`
    /// (`fmul qword`, each stored as an `f32`; the pointer reread each
    /// time).
    fn scale_translation(mem: &mut impl Mem, entry: u32, k: f64) {
        for o in [0x30u32, 0x34, 0x38] {
            let t = ptr32(mem.read(entry.wrapping_add(4))).wrapping_add(o);
            let x = x87::store(x87::mul(x87::load(mem.read(t)), k));
            mem.write(t, x);
        }
    }

    /// `op1_mdc_draw(list, init, m, view)` (`0x00a68c90`): draws each
    /// packet of a character model list through `op_md_chara_path3`.
    /// `[init +0x20]` is pointed at the local matrix `frame - 0x84` and
    /// `[init +0x2c]` at `frame - 0x44`; `view` (or `g_Op3dListener`
    /// when 0) is copied to `frame - 0xc4` (`op_m_copy`). For each of the
    /// `u16 list +4` entries (16 bytes from `list + 0x10`: packet, matrix):
    /// the matrix's translation `+0x30..+0x38` times 1000 (each an `f32`),
    /// `op_mul_m(frame - 0x44, m, matrix)`, the translation times 0.001
    /// (`f32` again), `op_mul_m(frame - 0x84, frame - 0xc4, frame - 0x44)`
    /// and `op_md_chara_path3(packet, init, [list])`. Then `[init +0x20]`
    /// gets its old value back, and `[init +0x2c]` the old `[init +0x34]`
    /// (as the original does), which is also the result. Callees through
    /// `call`.
    pub fn op1_mdc_draw<M: Mem>(
        mem: &mut M,
        list: u32,
        init: u32,
        m: u32,
        view: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let (world, local, eye) = (
            frame.wrapping_sub(0x84),
            frame.wrapping_sub(0x44),
            frame.wrapping_sub(0xc4),
        );
        let old_20: u32 = mem.read(init.wrapping_add(0x20));
        mem.write(init.wrapping_add(0x20), world);
        mem.write(init.wrapping_add(0x2c), local);
        let view = if view == 0 {
            mem.rebase(OP_3D_LISTENER)
        } else {
            view
        };
        go(mem, OP_M_COPY, &[eye, view], call);
        let old_34: u32 = mem.read(init.wrapping_add(0x34));
        let up: f64 = mem.read(mem.rebase(K_1000));
        let down: f64 = mem.read(mem.rebase(K_1000TH));
        let n = mem.read::<u16>(list.wrapping_add(4));
        let mut at = list.wrapping_add(0x10);
        for _ in 0..n {
            scale_translation(mem, at, up);
            let tm = ptr32(mem.read(at.wrapping_add(4)));
            go(mem, OP_MUL_M, &[local, m, tm], call);
            scale_translation(mem, at, down);
            go(mem, OP_MUL_M, &[world, eye, local], call);
            let chr: u32 = mem.read(list);
            let packet = ptr32(mem.read(at));
            go(mem, OP_MD_CHARA_PATH3, &[packet, init, chr], call);
            at = at.wrapping_add(0x10);
        }
        mem.write(init.wrapping_add(0x20), old_20);
        mem.write(init.wrapping_add(0x2c), old_34);
        old_34
    }

    const OP_SPR_ALLOC: u32 = 0x007e_6a00;
    const OP_SPR_FREE: u32 = 0x007e_6a60;
    const OP_VERTEX_NORMAL_POINT44_FFX_2: u32 = 0x00a6_d670;
    const OP_REFRECTION_ST_FLOAT: u32 = 0x00a6_cfb0;
    /// `g_EtChrHandles` (`0x012a8100`): 240 character handles.
    const ET_CHR_HANDLES: u32 = 0x012a_8100;
    /// `f64` 16 (`g_Vu0FixedScale`, `0x00b313a0`).
    const K_16: u32 = 0x00b3_13a0;
    /// The emulated EE's `r0` (`g_Ee_Gpr`, `0x00c8f508`).
    const EE_R0: u32 = 0x00c8_f508;
    /// `g_Vu0_VfMove` (`0x00c8f980`), the VU0 register the colour goes
    /// through.
    const VF_MOVE: u32 = 0x00c8_f980;

    /// The four words the EE makes of a colour word `c` and `r0`
    /// (`pextlb` then `pextlh` with `r0`, as `opus_0279_draw` emulates them
    /// byte by byte): word `k` is `c` byte `k`, then `r0` bytes `(k, 0, 1)`,
    /// `(1, 2, 3)`, `(2, 4, 5)`, `(3, 6, 7)` for `k` 0..3. With `r0` zero,
    /// the colour's bytes as words.
    fn expand_colour(c: u32, r0: [u8; 8]) -> [u32; 4] {
        let c = c.to_le_bytes();
        [
            u32::from_le_bytes([c[0], r0[0], r0[0], r0[1]]),
            u32::from_le_bytes([c[1], r0[1], r0[2], r0[3]]),
            u32::from_le_bytes([c[2], r0[2], r0[4], r0[5]]),
            u32::from_le_bytes([c[3], r0[3], r0[6], r0[7]]),
        ]
    }

    /// `opus_0279_draw(obj, init, rec, _, tex)` (`0x00a70210`, from
    /// `opp_mdc_draw_a6f4d0`): a refracting character model. `w =
    /// op_spr_alloc(0x1800)`, `[obj] |= 4`, `p = op_md_spr_init_x64(obj,
    /// init, w)`; with `a` the RGBA bytes of `op_ffxambient_to_omdambient()`
    /// and `t = f32(byte p +0x4c / 128)`, colour `w +0x370 + 4k` = `byte p
    /// +0x1c + k x (1 - t) + a[k] x t` (exact). `w +0x390` = `(i16 obj +0x12
    /// << 5) + [p +4]`, `+0x394` = `[p +4]`, `+0x400` = `(i16 obj +0x14 <<
    /// 4) + [w +0x390]`; `op_vertex_normal_point44_ffx_2(obj, p, w,
    /// g_EtChrHandles[byte rec +0x18])`, `op_refrection_st_float(obj, p, w,
    /// [w +0x400], ftol(f32 rec +0x3c x 16), byte tex +0xa << 4, byte tex
    /// +0xb << 4)`. Each tag of type 0x10 from `obj + [obj +4]` (0x20
    /// bytes each): `w +0x1f0` 0x8001, `+0x1f8` 0x12412412, `+0x120` `[tex]`,
    /// the colour word `[tag +4 + g_pppColorConShift]` (the shift then 0)
    /// expanded with `r0` ([`expand_colour`]) into the EE's `v0`, and as
    /// `f32`s of the signed words into `g_Vu0_VfMove` and `w +0x2e0`. Spins
    /// while the GS is busy, `op_spr_free()`. Returns `eax` as left: the
    /// last `g_GsRegisterSink` read. Callees through `call`.
    pub fn opus_0279_draw<M: Mem>(
        mem: &mut M,
        obj: u32,
        init: u32,
        rec: u32,
        tex: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let w = go(mem, OP_SPR_ALLOC, &[0x1800], call);
        mem.write(obj, mem.read::<u32>(obj) | 4);
        let p = go(mem, OP_MD_SPR_INIT_X64, &[obj, init, w], call);
        let ambient = go(mem, OP_FFXAMBIENT_TO_OMDAMBIENT, &[], call).to_le_bytes();
        let k: f64 = mem.read(mem.rebase(K_128TH));
        let t = f64::from(mem.read::<u8>(p.wrapping_add(0x4c))) * k;
        // rounded to f32 through a local, exact (a byte / 128)
        let t = x87::load(x87::store(t));
        let s = 1.0 - t;
        for (i, &a) in (0u32..).zip(&ambient) {
            let own = f64::from(mem.read::<u8>(p.wrapping_add(0x1c + i)));
            // a byte times a multiple of 1/128 plus another: exact
            let c = own * s + f64::from(a) * t;
            mem.write(w.wrapping_add(0x370 + 4 * i), x87::store(c));
        }
        vertex_area(mem, obj, w, p.wrapping_add(4), 5);
        let n = i32::from(mem.read::<i16>(obj.wrapping_add(0x14))).cast_unsigned();
        let area: u32 = mem.read(w.wrapping_add(0x390));
        mem.write(w.wrapping_add(0x400), (n << 4).wrapping_add(area));
        let who = u32::from(mem.read::<u8>(rec.wrapping_add(0x18)));
        let chr: u32 = mem.read(mem.rebase(ET_CHR_HANDLES).wrapping_add(4 * who));
        go(mem, OP_VERTEX_NORMAL_POINT44_FFX_2, &[obj, p, w, chr], call);
        let k16: f64 = mem.read(mem.rebase(K_16));
        let scale = x87::ftol(
            mem,
            x87::mul(x87::load(mem.read(rec.wrapping_add(0x3c))), k16),
        );
        let du = u32::from(mem.read::<u8>(tex.wrapping_add(0xa))) << 4;
        let dv = u32::from(mem.read::<u8>(tex.wrapping_add(0xb))) << 4;
        let out: u32 = mem.read(w.wrapping_add(0x400));
        go(
            mem,
            OP_REFRECTION_ST_FLOAT,
            &[obj, p, w, out, scale, du, dv],
            call,
        );
        let shift = crate::rand::bridge::COLOR_CON_SHIFT;
        let (r0, v0, vf) = (mem.rebase(EE_R0), mem.rebase(EE_V0), mem.rebase(VF_MOVE));
        let mut tag = obj.wrapping_add(mem.read(obj.wrapping_add(4)));
        while mem.read::<u8>(tag.wrapping_add(1)) == 0x10 {
            mem.write(w.wrapping_add(0x1f0), 0x8001u32);
            mem.write(w.wrapping_add(0x1f8), 0x1241_2412u32);
            mem.write(w.wrapping_add(0x120), mem.read::<u32>(tex));
            let c: u32 = mem.read(shift.get(mem).wrapping_add(tag).wrapping_add(4));
            let words = expand_colour(c, mem.read(r0));
            mem.write(v0, words);
            shift.set(mem, 0);
            #[allow(clippy::cast_precision_loss)] // fild, then an f32 store
            let f = words.map(|x| x.cast_signed() as f32);
            mem.write(vf, f);
            mem.write(w.wrapping_add(0x2e0), f);
            tag = tag.wrapping_add(0x20);
        }
        gs_wait(mem);
        let sink: u32 = mem.read(mem.rebase(GS_REGISTER_SINK));
        go(mem, OP_SPR_FREE, &[], call);
        sink
    }
}
