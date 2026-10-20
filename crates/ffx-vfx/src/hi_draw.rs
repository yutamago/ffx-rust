//! The op layer's model draw set-ups at `0x00a6a3a0..0x00a6c8b0`
//! (`op_md_chara_path3` and four `op1_md_draw` paths: flag 4 with and
//! without `0x300`, flag `0x2000`, flag `0x80` with `0x300`;
//! `docs/formats/vfx.md`, "High range"). Each takes `0x1800` (or
//! `0x2800`) bytes of the op scratch stack as its work area `w`, fills the
//! PS2 packet words there and pops it again.

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
        let size: u32 = mem.read(init.wrapping_add(0x34));
        if go(mem, OP_MD_PACKET_SIZE_CHECK, &[obj, size, 1], call) != 0 {
            return mem.read(init.wrapping_add(0x34));
        }
        let w = spr_push_n(mem, true, 0x2800);
        copy_init(mem, init, w);
        let end: u32 = mem.read(w.wrapping_add(0x334));
        mem.write(w.wrapping_add(0x334), end.wrapping_add(0x3f) & 0x0fff_ffc0);
        default_bases(mem, obj, w, false);
        colour_from_bytes(mem, w.wrapping_add(0x31c), w);
        let base: u32 = mem.read(w.wrapping_add(0x304));
        mem.write(w.wrapping_add(0x394), base);
        let n = i32::from(mem.read::<i16>(obj.wrapping_add(0x12))).cast_unsigned();
        let base: u32 = mem.read(w.wrapping_add(0x304));
        mem.write(w.wrapping_add(0x39c), (n << 4).wrapping_add(base));
        let tags = obj.wrapping_add(mem.read(obj.wrapping_add(4)));
        mem.write(w.wrapping_add(0x110), 0x8001u32);
        mem.write(w.wrapping_add(0x118), 0x000e_eeeeu32);
        mem.write(w.wrapping_add(0x128), 6u32);
        fog_bytes(mem, obj, w);
        mem.write(w.wrapping_add(0x138), 0x42u32);
        copy_words(mem, MD_GS_WORDS_C, w, 0x140);
        copy_words(mem, MD_GS_WORDS_A, w, 0x150);
        let alpha = alpha_test(mem, obj);
        mem.write(w.wrapping_add(0x160), alpha);
        mem.write(w.wrapping_add(0x168), 8u32);
        let block = mem.rebase(MD_PATH3_BLOCK);
        mem.write(w.wrapping_add(0x180), block);
        gs_wait(mem);
        let init = w.wrapping_add(0x300);
        let mut tag = tags;
        loop {
            let (gif, mode, textured) = match mem.read::<i8>(tag.wrapping_add(1)) {
                0 => (0x2000_0009u32, 0x4141_41eeu32, false),
                2 => (0x2000_000d, 0x1241_2eee, true),
                4 => (0x2000_000b, 0x4141_41ee, false),
                6 => (0x2000_0010, 0x1241_2eee, true),
                _ => break,
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
        spr_pop(mem);
        let end: u32 = mem.read(w.wrapping_add(0x334));
        end.wrapping_add(0x40)
    }
}
