//! Skinning and placing the models for drawing: `Apply_Envelope` (the
//! vertices from the element matrices) and `Ch_DrawMainForSituation`
//! (each character's world matrices for the renderer's situation pass).
//!
//! The model header (`+0x1c0`) holds the skin lists: `u16` count at `+8`
//! of 12-byte entries from the pointer at `+0x14` (`u16` element, model,
//! base element, kind; the weight lists at `+8`), and as many normal
//! entries from `+0x24` (`u16` model at `+2`, kind at `+6`, lists at
//! `+8`).

/// The originals, on the characters.
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    use crate::layout as l;

    /// `u32`: every character recomputes its normals (`Ch_GuiSetCalcNormAll`).
    pub const GUI_CALC_NORMAL: Global<u32> = Global::new(0x0130_09e0);
    /// `u8`: characters still to recompute their normals once (flag bit 3).
    pub const CALC_NORMAL_ONCE: Global<u8> = Global::new(0x0130_09e4);

    /// `MagicFile_GetMagicId()` (`0x009da3b0`), `user_malloc`, `memset`'s
    /// thunk (`0x0094964c`), `FFXVu0InversMatrix(dst, m)` (`0x006ed450`),
    /// `FFXVu0MulMatrix(dst, a, b)` (`0x006ed5c0`), the skinning kernels
    /// `sg_calcenvelope_asm(lists, out, mask, m)` (`0x006ee570`),
    /// `sg_calcenvelope_asm_opt(cache, out, m)` (`0x006eee40`), their
    /// weighted versions (`0x006ed9e0`, `0x006ee270`), the normal kernels
    /// `sg_calcenvelope_norm(lists, out, m)` (`0x00845eb0`) and
    /// `sg_calcenvelope_normW` (`0x00845ec0`), and `sg_normalize(v, n)`
    /// (`0x00846710`): all through the game.
    pub const MAGIC_GET_ID: u32 = 0x009d_a3b0;
    pub const MEMSET_THUNK: u32 = 0x0094_964c;
    pub const VU0_INVERS_MATRIX: u32 = 0x006e_d450;
    pub const VU0_MUL_MATRIX: u32 = 0x006e_d5c0;
    pub const CALC_ENVELOPE: u32 = 0x006e_e570;
    pub const CALC_ENVELOPE_OPT: u32 = 0x006e_ee40;
    pub const CALC_ENVELOPE_W: u32 = 0x006e_d9e0;
    pub const CALC_ENVELOPE_W_OPT: u32 = 0x006e_e270;
    pub const CALC_ENVELOPE_NORM: u32 = 0x0084_5eb0;
    pub const CALC_ENVELOPE_NORM_W: u32 = 0x0084_5ec0;
    pub const SG_NORMALIZE: u32 = 0x0084_6710;
    /// Bytes of scratch [`apply_envelope`] needs: the inverted base
    /// element (`[ebp-0x84]`) and the skinning matrix (`[ebp-0x44]`).
    pub const APPLY_ENVELOPE_SCRATCH: u32 = 0x80;

    /// The (magic file, character id) pairs whose models get their normals
    /// skinned too, in the order `Apply_Envelope` tests them.
    pub const SKIN_NORMALS: [(u32, u16); 3] = [(0x1b6, 0x1114), (0x1b5, 0x1063), (0x1b6, 0x1064)];

    /// `Apply_Envelope(chr, all)` (`0x00832110`; callers `Ch_DrawMain`,
    /// `Ch_Cloth_PerCharacter`, three effect functions): skins the
    /// character's models.
    /// - Normal recomputation requests: with [`GUI_CALC_NORMAL`], flag bit
    ///   0, bit 3 while [`CALC_NORMAL_ONCE`] is not 0, or bit 4 (`+0x194`),
    ///   bit 3 is cleared (taking one off [`CALC_NORMAL_ONCE`], which may
    ///   wrap) and so is bit 4.
    /// - In magic file `0x1b6` characters `0x1114` and `0x1064`, in `0x1b5`
    ///   `0x1063` (`MagicFile_GetMagicId` asked once per test, up to three
    ///   times) skin every model and their normals; nothing at all while
    ///   `g_ChrDrawDetached`. Otherwise a first call with `all` 0 makes the
    ///   kernel cache (`+0x874`, a zeroed word per skin entry).
    /// - Each skin entry whose model is shown (`+0x2c`) or every entry with
    ///   `all`: the inverse of its base element's matrix (`+0x88` of the
    ///   0x160-byte elements at `+0x328`; kept while the base element
    ///   repeats) times its element's is the matrix the kernel runs with
    ///   (weighted for a non-zero kind). Without `all`, a model with `+0x34`
    ///   set uses the cache: the first run (`mask` `+0x30`) stores what the
    ///   kernel returns, later runs take the fast kernel; else the kernel
    ///   runs with mask 0. The model's vertices (`+8`) are the output.
    /// - The normal entries go along in the magic case (lists at `+8`, when
    ///   set, into the normals `+4` of model `+2`), and the normals of every
    ///   model are normalized at the end (`sg_normalize(+4, data +0x16)`).
    ///
    /// The counts are read again from the header at each step (`u16`,
    /// compared signed). No caller reads `eax`.
    #[allow(clippy::too_many_lines)]
    pub fn apply_envelope<M: Mem>(
        mem: &mut M,
        chr: u32,
        all: u32,
        scratch: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let at = |o: u32| chr.wrapping_add(o);
        let flags: u32 = mem.read(at(l::FLAGS));
        let once = CALC_NORMAL_ONCE.get(mem);
        if GUI_CALC_NORMAL.get(mem) != 0
            || flags & 1 != 0
            || (flags & 8 != 0 && once != 0)
            || flags & 0x10 != 0
        {
            if flags & 8 != 0 {
                mem.write(at(l::FLAGS), flags & !8);
                CALC_NORMAL_ONCE.set(mem, once.wrapping_sub(1));
            }
            let flags: u32 = mem.read(at(l::FLAGS));
            if flags & 0x10 != 0 {
                mem.write(at(l::FLAGS), flags & !0x10);
            }
        }
        let g = |mem: &M, f: u32| mem.rebase(f);
        let chr_id: u16 = mem.read(chr);
        let mut special = false;
        for (magic, id) in SKIN_NORMALS {
            if call(mem, g(mem, MAGIC_GET_ID), &[]) == magic && chr_id == id {
                special = true;
                break;
            }
        }
        let all = if special {
            if crate::sys::bridge::DRAW_DETACHED.get(mem) != 0 {
                return;
            }
            1
        } else {
            if all == 0 && mem.read::<u32>(at(0x874)) == 0 {
                let hdr: u32 = mem.read(at(l::MODEL_HEADER));
                let n = u32::from(mem.read::<u16>(hdr.wrapping_add(8)));
                let malloc = g(mem, crate::handle::bridge::USER_MALLOC);
                let p = call(mem, malloc, &[n << 2]);
                mem.write(at(0x874), p);
                let hdr: u32 = mem.read(at(l::MODEL_HEADER));
                let n = u32::from(mem.read::<u16>(hdr.wrapping_add(8)));
                call(mem, g(mem, MEMSET_THUNK), &[p, 0, n << 2]);
            }
            all
        };
        let (inv, m) = (scratch, scratch + 0x40);
        let header = |mem: &M| mem.read::<u32>(at(l::MODEL_HEADER));
        let count = |mem: &M, o: u32| i32::from(mem.read::<u16>(header(mem).wrapping_add(o)));
        let model = |mem: &M, i: u16| {
            mem.read::<u32>(at(l::MODELS))
                .wrapping_add(u32::from(i) * l::MODEL_SIZE)
        };
        let elem_matrix = |mem: &M, e: u16| {
            mem.read::<u32>(at(0x328))
                .wrapping_add(0x88)
                .wrapping_add(u32::from(e) * l::ELEM_SIZE)
        };
        let mut last_base = u32::MAX;
        let mut i = 0i32;
        while i < count(mem, 8) {
            let off = i.cast_unsigned() * 0xc;
            let e = mem
                .read::<u32>(header(mem).wrapping_add(0x14))
                .wrapping_add(off);
            let mi: u16 = mem.read(e.wrapping_add(2));
            if all != 0 || mem.read::<u32>(model(mem, mi).wrapping_add(0x2c)) != 0 {
                let base: u16 = mem.read(e.wrapping_add(4));
                if last_base != u32::from(base) {
                    last_base = u32::from(base);
                    let src = elem_matrix(mem, base);
                    call(mem, g(mem, VU0_INVERS_MATRIX), &[inv, src]);
                }
                let elem: u16 = mem.read(e);
                let src = elem_matrix(mem, elem);
                call(mem, g(mem, VU0_MUL_MATRIX), &[m, inv, src]);
                let weighted = mem.read::<u16>(e.wrapping_add(6)) != 0;
                let (full, fast) = if weighted {
                    (CALC_ENVELOPE_W, CALC_ENVELOPE_W_OPT)
                } else {
                    (CALC_ENVELOPE, CALC_ENVELOPE_OPT)
                };
                let lists: u32 = mem.read(e.wrapping_add(8));
                let mdl = model(mem, mi);
                let out: u32 = mem.read(mdl.wrapping_add(8));
                if all == 0 && mem.read::<u32>(mdl.wrapping_add(0x34)) != 0 {
                    let slot = 4 * i.cast_unsigned();
                    let cache: u32 = mem.read(mem.read::<u32>(at(0x874)).wrapping_add(slot));
                    if cache == 0 {
                        let mask: u32 = mem.read(mdl.wrapping_add(0x30));
                        let r = call(mem, g(mem, full), &[lists, out, mask, m]);
                        let table: u32 = mem.read(at(0x874));
                        mem.write(table.wrapping_add(slot), r);
                    } else {
                        call(mem, g(mem, fast), &[cache, out, m]);
                    }
                } else {
                    call(mem, g(mem, full), &[lists, out, 0, m]);
                }
                if special {
                    let ne = mem
                        .read::<u32>(header(mem).wrapping_add(0x24))
                        .wrapping_add(off);
                    let lists: u32 = mem.read(ne.wrapping_add(8));
                    if lists != 0 {
                        let f = if mem.read::<u16>(ne.wrapping_add(6)) == 0 {
                            CALC_ENVELOPE_NORM
                        } else {
                            CALC_ENVELOPE_NORM_W
                        };
                        let nm: u16 = mem.read(ne.wrapping_add(2));
                        let normals: u32 = mem.read(model(mem, nm).wrapping_add(4));
                        call(mem, g(mem, f), &[lists, normals, m]);
                    }
                }
            }
            i += 1;
        }
        if !special {
            return;
        }
        let mut k = 0i32;
        while k < count(mem, 6) {
            let mdl = mem
                .read::<u32>(at(l::MODELS))
                .wrapping_add(k.cast_unsigned() * l::MODEL_SIZE);
            let data: u32 = mem.read(mdl);
            let n = u32::from(mem.read::<u16>(data.wrapping_add(0x16)));
            let normals: u32 = mem.read(mdl.wrapping_add(4));
            call(mem, g(mem, SG_NORMALIZE), &[normals, n]);
            k += 1;
        }
    }

    /// `FFXVu0UnitMatrix(m)` (`0x006ed9a0`),
    /// `graphicCharacterInstanceSetWorldMatrix(inst, m)` (`0x0063c3e0`),
    /// `graphicCharacterInstanceSetWorldMatrixByMdlID(inst, m, i)`
    /// (`0x0063c430`), `graphicCharacterInstanceUpdateSkeletonBounds(inst)`
    /// (`0x0063c500`) and `Ch_UpdateClothMatrix(chr)` (`0x008314c0`): all
    /// through the game.
    pub const VU0_UNIT_MATRIX: u32 = 0x006e_d9a0;
    pub const SET_WORLD_MATRIX: u32 = 0x0063_c3e0;
    pub const SET_WORLD_MATRIX_BY_MDL_ID: u32 = 0x0063_c430;
    pub const UPDATE_SKELETON_BOUNDS: u32 = 0x0063_c500;
    pub const UPDATE_CLOTH_MATRIX: u32 = 0x0083_14c0;
    /// Bytes of scratch [`ch_draw_main_for_situation`] needs: the scale
    /// (`[ebp-0x44]`), the character's world matrix (`[ebp-0x84]`) and a
    /// model's (`[ebp-0xc4]`).
    pub const DRAW_SITUATION_SCRATCH: u32 = 0xc0;

    /// `Ch_DrawMainForSituation()` (`0x0082fe00`; caller
    /// `Ch_DrawMainEffect`): the world matrices for every active character
    /// that is not hidden (`+0x180`), not flagged out (`+0x198` bit 5) and
    /// not fully transparent (alpha `+0x354` is not ±0; NaN draws). With a
    /// render instance (`+0x830`) its matrix (`+0x1d0`) times a unit matrix
    /// scaled by 1000 in x, y, z (`f32(1.0 * 1000.0)`) is the world matrix:
    /// - when every model (`u16` header `+6`) is in the skin list (an entry
    ///   whose `u16` `+2` names it), or there are none, it goes to the
    ///   instance and the shadow instance (`+0x838`) whole;
    /// - else per model: a skinned one gets it as is, the others times
    ///   their base element's matrix (`+0x88` of element `u16 [data]`), by
    ///   model id.
    ///
    /// The renderer recomputes the skeleton's bounds when any model was
    /// skinned (not while `g_ChrDrawDetached`). Then `Ch_UpdateClothMatrix`
    /// (also without an instance). The array is read again after each
    /// character. Model counts are taken as the data has them (the
    /// original keeps the skin flags in 0x80 words on its stack).
    #[allow(clippy::too_many_lines)]
    pub fn ch_draw_main_for_situation<M: Mem>(
        mem: &mut M,
        scratch: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use crate::handle::bridge::{CHR_ARRAY, CHR_COUNT};
        let g = |mem: &M, f: u32| mem.rebase(f);
        let (scale, world, part) = (scratch, scratch + 0x40, scratch + 0x80);
        call(mem, g(mem, VU0_UNIT_MATRIX), &[scale]);
        for k in [0u32, 5, 10] {
            let v: f32 = mem.read(scale + 4 * k);
            #[allow(clippy::cast_possible_truncation)] // fstp dword
            mem.write(
                scale + 4 * k,
                (f64::from(crate::x87::load(v)) * 1000.0) as f32,
            );
        }
        let end = |mem: &M| {
            let n = CHR_COUNT.get(mem).cast_unsigned();
            CHR_ARRAY.get(mem).wrapping_add(n.wrapping_mul(l::SIZE))
        };
        let mut c = CHR_ARRAY.get(mem);
        while c != end(mem) {
            let at = |o: u32| c.wrapping_add(o);
            let alpha: f32 = mem.read(at(0x354));
            if mem.read::<u8>(at(l::ACTIVE)) == 0
                || mem.read::<u8>(at(0x180)) != 0
                || mem.read::<u8>(at(l::FLAGS2)) & 0x20 != 0
                || alpha == 0.0
            {
                c = c.wrapping_add(l::SIZE);
                continue;
            }
            if mem.read::<u32>(at(l::RENDER_INSTANCE)) != 0 {
                call(mem, g(mem, VU0_MUL_MATRIX), &[world, at(0x1d0), scale]);
                let hdr: u32 = mem.read(at(l::MODEL_HEADER));
                let models = usize::from(mem.read::<u16>(hdr.wrapping_add(6)));
                let mut skinned = vec![false; models];
                let mut any = false;
                let entries = mem.read::<u16>(hdr.wrapping_add(8));
                for (m, flag) in skinned.iter_mut().enumerate() {
                    let list: u32 = mem.read(hdr.wrapping_add(0x14));
                    for e in 0..u32::from(entries) {
                        let mi: u16 = mem.read(list.wrapping_add(e * 0xc + 2));
                        if usize::from(mi) == m {
                            *flag = true;
                            any = true;
                            break;
                        }
                    }
                }
                if skinned.iter().all(|&s| s) {
                    for o in [l::RENDER_INSTANCE, 0x838] {
                        let inst: u32 = mem.read(at(o));
                        call(mem, g(mem, SET_WORLD_MATRIX), &[inst, world]);
                    }
                } else {
                    let mut m = 0u32;
                    let count = |mem: &M| {
                        let hdr: u32 = mem.read(c.wrapping_add(l::MODEL_HEADER));
                        u32::from(mem.read::<u16>(hdr.wrapping_add(6)))
                    };
                    while m < count(mem) {
                        let inst: u32 = mem.read(at(l::RENDER_INSTANCE));
                        let by_id = g(mem, SET_WORLD_MATRIX_BY_MDL_ID);
                        if skinned.get(m as usize).copied().unwrap_or(false) {
                            call(mem, by_id, &[inst, world, m]);
                        } else {
                            let mdl = mem
                                .read::<u32>(at(l::MODELS))
                                .wrapping_add(m * l::MODEL_SIZE);
                            let base: u16 = mem.read(mem.read::<u32>(mdl));
                            let e = mem
                                .read::<u32>(at(0x328))
                                .wrapping_add(0x88)
                                .wrapping_add(u32::from(base) * l::ELEM_SIZE);
                            call(mem, g(mem, VU0_MUL_MATRIX), &[part, world, e]);
                            let inst: u32 = mem.read(at(l::RENDER_INSTANCE));
                            call(mem, by_id, &[inst, part, m]);
                        }
                        m += 1;
                    }
                }
                if any && crate::sys::bridge::DRAW_DETACHED.get(mem) == 0 {
                    let inst: u32 = mem.read(at(l::RENDER_INSTANCE));
                    call(mem, g(mem, UPDATE_SKELETON_BOUNDS), &[inst]);
                }
            }
            call(mem, g(mem, UPDATE_CLOTH_MATRIX), &[c]);
            c = c.wrapping_add(l::SIZE);
        }
    }
}
