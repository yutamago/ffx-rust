//! The `pppDrawMdl*` draws: a model record drawn with the object's colour
//! through a render request on the stack (`docs/formats/vfx.md`, "Model
//! draws").

/// The functions over game memory.
pub mod bridge {
    use crate::draw::bridge::{ENV_SLOT, GET_PTR32, NAME_HASH, STOP, f, g, instance};
    use ffx_bridge::Mem;
    use ffx_bridge::x87::{add, div, ftol, load, mul, store};

    const PEXTLH: u32 = 0x0073_7450;
    const PMULTH: u32 = 0x0073_74e0;
    const PPACB: u32 = 0x0073_75d0;
    const GET_RENDER_DATA: u32 = 0x0064_53d0;
    const UPDATE_RENDER_DATA: u32 = 0x0071_8610;
    const PATH3_UPDATE: u32 = 0x0064_5600;
    const VFX_UPDATE: u32 = 0x0064_58a0;
    const ALPHA_FUNCTION: u32 = 0x0071_2c30;
    const Z_MAT: u32 = 0x0071_2d00;
    const INIT_PATH3_RENDER_DATA: u32 = 0x0071_0fd0;
    const INIT_RENDER_DATA: u32 = 0x0071_1180;
    const GET_PTR_LOG_COUNT: u32 = 0x0064_1370;
    /// `g_pppColorConShift`: added once to a colour read, then cleared.
    const COLOR_SHIFT: u32 = 0x00c8_b500;
    /// `g_pppCurrentPart`.
    const CURRENT_PART: u32 = 0x0230_ffe4;
    /// `g_PppPacketPos`.
    const PACKET_POS: u32 = 0x0230_fd2c;
    /// The request's `+4` word.
    const REQUEST_WORD4: u32 = 0x00c4_98dc;
    /// The request's two matrix-table pointers (`+0x24`, `+0x28`).
    const REQUEST_TABLES: [u32; 2] = [0x0230_fd60, 0x0230_fda0];
    /// The emulated EE's `r0` (`g_Ee_Gpr`) and the two work registers.
    const EE_R0: u32 = 0x00c8_f508;
    const EE_A: u32 = 0x00c8_f588;
    const EE_B: u32 = 0x00c8_f598;
    /// The level that skips `pppDrawMdlDraw`.
    const LEVEL_SKIP: u32 = 0xec1;
    /// The level where `pppDrawMdlTsDraw` draws even at alpha 0.
    const LEVEL_TS_ALWAYS: u32 = 0x169c;
    const GET_MAGIC_HANDLE: u32 = 0x009d_a390;
    const GET_MAGIC_ID: u32 = 0x009d_a3b0;
    const INVERSE_MATRIX: u32 = 0x006e_d460;
    const MUL_MATRIX: u32 = 0x006e_d5c0;
    const GET_LANG: u32 = 0x008a_c2f0;
    const UPDATE_TEXTURE: u32 = 0x0064_0060;
    /// `g_pppViewMatrix`.
    const VIEW_MATRIX: u32 = 0x0230_ff20;
    /// `g_ScreenMatrix`.
    const SCREEN_MATRIX: u32 = 0x0231_1480;
    /// `g_pppScreenMatrix`.
    const PPP_SCREEN_MATRIX: u32 = 0x0230_fe60;
    /// `g_VfxPass`.
    const VFX_PASS: u32 = 0x0113_335c;
    /// `g_CurrentMapNo`.
    const MAP_NO: u32 = 0x012f_bc40;
    /// `g_pppFpCurrentId`: the playing field effect's id (`pppFpLoop`).
    const FP_ID: u32 = 0x00c3_a500;
    /// `g_VfxYiqMode`.
    const YIQ_MODE: u32 = 0x00c3_a504;
    /// The language whose screen-space models get their `+0x30` divided
    /// (`TOGetFFXLang`).
    const LANG_SCALED: u32 = 0xb;
    /// The divisor, `1.8f32` as a double (`0x00b51560`).
    const LANG_SCALE: f64 = 1.799_999_952_316_284;
    const IS_PTR32_NULL: u32 = 0x00a4_46a0;
    const SET_PTR32: u32 = 0x00a4_4700;
    const UNIT_MATRIX: u32 = 0x006e_d9a0;
    const COPY_VECTOR: u32 = 0x006e_d370;
    /// `g_pppViewMatrixCopy`.
    const VIEW_MATRIX_COPY: u32 = 0x0230_fe20;
    /// `g_pppWorldMatrix`'s translation row.
    const WORLD_POSITION: u32 = 0x0230_ffd0;
    /// The `Loop` draws' angle period (`0x00b51588`).
    const LOOP_PERIOD: f64 = 32768.0;
    /// The work area's float-to-fixed scale of the `Ts` draws (`0x00b51558`).
    const TS_SCALE: f64 = 65536.0;

    fn at(base: u32, off: u32) -> u32 {
        base.wrapping_add(off)
    }

    /// `[[prog + 0xc] + 4 k]`, the program's `k`-th work-area offset.
    fn offset(mem: &impl Mem, prog: u32, k: u32) -> u32 {
        mem.read(at(mem.read(at(prog, 0xc)), 4 * k))
    }

    /// The colour every model draw starts with: the object's colour (the
    /// halves of the work area's `+0xa0` words, read through
    /// `g_pppColorConShift`) times the current part's `+0x70` halves, each
    /// product `>> 19 & 0x1fff`, packed to bytes. The original runs it on
    /// the emulated EE: two `pextlh` with `r0` (calls in `pppDrawMdlDraw`,
    /// inlined through a stack temporary everywhere else), `pmulth`, the
    /// halves packed back with `r0`'s low halves above, `ppacb`. Returns
    /// the packed colour (the work register's low word).
    fn ee_colour<M: Mem>(
        mem: &mut M,
        obj: u32,
        prog: u32,
        pextlh: bool,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let base = at(offset(mem, prog, 0), obj);
        ee_colour_at(mem, base, pextlh, call)
    }

    /// [`ee_colour`] of the work area at `base` (its `+0xa0` words, read
    /// through `g_pppColorConShift`).
    pub(crate) fn ee_colour_at<M: Mem>(
        mem: &mut M,
        base: u32,
        pextlh: bool,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let (r0, a, b, shift) = (
            g(mem, EE_R0),
            g(mem, EE_A),
            g(mem, EE_B),
            g(mem, COLOR_SHIFT),
        );
        let area = base.wrapping_add(mem.read::<u32>(shift));
        let lo: u32 = mem.read(at(area, 0xa0));
        mem.write(a, lo);
        let hi: u32 = mem.read(at(area, 0xa4));
        let part: u32 = mem.read(g(mem, CURRENT_PART));
        mem.write(shift, 0u32);
        mem.write(at(a, 4), hi);
        let (p0, p1): (u32, u32) = (mem.read(at(part, 0x70)), mem.read(at(part, 0x74)));
        mem.write(b, p0);
        mem.write(at(b, 4), p1);
        mem.write(shift, 0u32);
        if pextlh {
            call(mem, g(mem, PEXTLH), &[a, r0, a]);
            call(mem, g(mem, PEXTLH), &[b, r0, b]);
        } else {
            // pextlh rd, r0, rd: rd's low halves interleaved with r0's.
            // The inlined stores overlap and come in another order; only
            // the final words are visible (nothing reads them between).
            let z = [mem.read::<u32>(r0), mem.read::<u32>(at(r0, 4))];
            for (reg, w) in [(a, [lo, hi]), (b, [p0, p1])] {
                for k in 0..4u32 {
                    let src = w[(k / 2) as usize] >> (16 * (k % 2)) & 0xffff;
                    let zh = z[(k / 2) as usize] >> (16 * (k % 2)) & 0xffff;
                    mem.write(at(reg, 4 * k), src | zh << 16);
                }
            }
            mem.write(shift, 0u32);
        }
        call(mem, g(mem, PMULTH), &[a, a, b]);
        // the products >> 19 & 0x1fff as halves, r0's low halves above
        let word =
            |mem: &M, at: u32| (mem.read::<u32>(at).cast_signed() >> 19).cast_unsigned() & 0x1fff;
        let ch = [0u32, 4, 8, 0xc].map(|o| word(mem, a + o));
        let r = [0u32, 4, 8, 0xc].map(|o| mem.read::<u32>(r0 + o) & 0xffff);
        mem.write(a, ch[0] | ch[1] << 16);
        mem.write(a + 4, ch[2] | ch[3] << 16);
        mem.write(a + 8, r[0] | r[1] << 16);
        mem.write(a + 0xc, r[2] | r[3] << 16);
        call(mem, g(mem, PPACB), &[a, r0, a]);
        mem.read(a)
    }

    /// The colour stored in the request's `+0x18` (at `req + 0x18 +
    /// g_pppColorConShift`, then the shift cleared); returns it.
    fn store_colour(mem: &mut impl Mem, req: u32, colour: u32) -> u32 {
        let shift = g(mem, COLOR_SHIFT);
        let sh: u32 = mem.read(shift);
        mem.write(at(req, 0x18).wrapping_add(sh), colour);
        mem.write(shift, 0u32);
        colour
    }

    /// The key's model record, `env[+0x20] + 32 [key + 4]`.
    fn model_record(mem: &impl Mem, key: u32) -> u32 {
        let env: u32 = mem.read(mem.read::<u32>(g(mem, ENV_SLOT)));
        let model: u32 = mem.read(at(key, 4));
        mem.read::<u32>(at(env, 0x20)).wrapping_add(model << 5)
    }

    /// The request flags of `pppDrawMdlDraw` and the `Ts` draws, from the
    /// model's first byte `m0`: `0x80` (bit 0 set) or `0xc`, `| 0x10`
    /// unless bit 1.
    fn flags_model(m0: u8) -> u32 {
        let f = if m0 & 1 != 0 { 0x80 } else { 0xc };
        if m0 & 2 == 0 { f | 0x10 } else { f }
    }

    /// The request flags from a key byte `d` and the model's first byte
    /// `m0`: `d & 2` gives `4` (`0xc` unless `m0` bit 0); else `0x80` for
    /// `d & 1`, `| 8` unless `m0` bit 0; with `exact4`, `d == 4` gives `8`;
    /// then `| 0x10` unless `m0` bit 1.
    fn flags_key(d: u8, m0: u8, exact4: bool) -> u32 {
        let mut f = 0;
        if d & 2 != 0 {
            f = if m0 & 1 != 0 { 4 } else { 0xc };
        } else {
            if d & 1 != 0 {
                f = 0x80;
            }
            if m0 & 1 == 0 {
                f |= 8;
            }
        }
        if exact4 && d == 4 {
            f = 8;
        }
        if m0 & 2 == 0 { f | 0x10 } else { f }
    }

    /// The request words every model draw sets, from `req`: flags, `+4`,
    /// the blend `+8` (and `+0xa` cleared), `+0x1c` the part's `+0x78`,
    /// the object's matrices `+0x50` (`+0x20`) and `+0x10` (`+0x2c`), the
    /// matrix tables, the packet position, the textures `+0x38`/`+0x3c`,
    /// and `+0xc..+0x14`, `+0x40`, `+0x44` cleared (callers set those that
    /// differ). The colour (`+0x18`), `+0xb` and `+0x30` are left alone.
    fn fill_request(mem: &mut impl Mem, req: u32, obj: u32, flags: u32, blend: u16, tex: [u32; 2]) {
        mem.write(req, flags);
        mem.write(at(req, 4), mem.read::<u32>(g(mem, REQUEST_WORD4)));
        mem.write(at(req, 8), blend);
        mem.write(at(req, 0xa), 0u8);
        let part: u32 = mem.read(g(mem, CURRENT_PART));
        mem.write(at(req, 0x1c), mem.read::<u32>(at(part, 0x78)));
        mem.write(at(req, 0x20), at(obj, 0x50));
        mem.write(at(req, 0x24), g(mem, REQUEST_TABLES[0]));
        mem.write(at(req, 0x28), g(mem, REQUEST_TABLES[1]));
        mem.write(at(req, 0x2c), at(obj, 0x10));
        mem.write(at(req, 0x34), mem.read::<u32>(g(mem, PACKET_POS)));
        mem.write(at(req, 0x38), tex[0]);
        mem.write(at(req, 0x3c), tex[1]);
        for o in [0xcu32, 0x10, 0x14, 0x40, 0x44] {
            mem.write(at(req, o), 0u32);
        }
    }

    /// `get_ptr32` of the object's two textures (`+0x90`, `+0x94`).
    fn textures<M: Mem>(
        mem: &mut M,
        obj: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> [u32; 2] {
        let t0 = call(mem, g(mem, GET_PTR32), &[mem.read(at(obj, 0x90))]);
        let t1 = call(mem, g(mem, GET_PTR32), &[mem.read(at(obj, 0x94))]);
        [t0, t1]
    }

    /// How a model draw hands its request to the renderer.
    #[derive(Debug, Clone, Copy)]
    struct Submit {
        /// Whether flag 4 picks the path-3 entry points.
        path3: bool,
        /// Whether flag `0x2000` skips the Z matrix (else it is always
        /// added).
        z_test: bool,
    }

    /// The instance's render data updated from the request and the
    /// instance updated (path 3 for flag 4 when `path3`); returns the
    /// update's eax.
    fn update<M: Mem>(
        mem: &mut M,
        obj: u32,
        rec: u32,
        req: u32,
        path3: bool,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let id = instance(mem, obj);
        call(mem, g(mem, UPDATE_RENDER_DATA), &[id[0], id[1], rec, req]);
        let id = instance(mem, obj);
        let f = if path3 && mem.read::<u8>(req) & 4 != 0 {
            PATH3_UPDATE
        } else {
            VFX_UPDATE
        };
        call(mem, g(mem, f), &id)
    }

    /// Render data created for the instance with `alpha` (path 3 for flag
    /// 4 when `path3`); returns the call's eax.
    fn create<M: Mem>(
        mem: &mut M,
        obj: u32,
        rec: u32,
        req: u32,
        path3: bool,
        alpha: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let id = instance(mem, obj);
        let f = if path3 && mem.read::<u8>(req) & 4 != 0 {
            INIT_PATH3_RENDER_DATA
        } else {
            INIT_RENDER_DATA
        };
        call(mem, g(mem, f), &[id[0], id[1], rec, req, alpha])
    }

    /// Whether the instance has render data.
    fn has_render_data<M: Mem>(
        mem: &mut M,
        obj: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> bool {
        let id = instance(mem, obj);
        call(mem, g(mem, GET_RENDER_DATA), &id) != 0
    }

    /// The instance's render data [`update`]d when it has some, else
    /// [`create`]d with the alpha function of the blend, the Z matrix of
    /// `+0x44` and `0x40` for flag 8. Returns the last call's eax.
    fn submit<M: Mem>(
        mem: &mut M,
        obj: u32,
        rec: u32,
        req: u32,
        how: Submit,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        if has_render_data(mem, obj, call) {
            return update(mem, obj, rec, req, how.path3, call);
        }
        let blend = u32::from(mem.read::<u8>(at(req, 8)));
        let mut alpha = call(mem, g(mem, ALPHA_FUNCTION), &[blend]);
        if !how.z_test || mem.read::<u32>(req) & 0x2000 == 0 {
            alpha |= call(mem, g(mem, Z_MAT), &[mem.read(at(req, 0x44))]);
        }
        if mem.read::<u8>(req) & 8 != 0 {
            alpha |= 0x40;
        }
        create(mem, obj, rec, req, how.path3, alpha, call)
    }

    const MDL: Submit = Submit {
        path3: true,
        z_test: true,
    };
    const SEMI: Submit = Submit {
        path3: false,
        z_test: false,
    };
    const SEMI2: Submit = Submit {
        path3: false,
        z_test: true,
    };

    /// `pppDrawMdlDraw(obj, key, prog)` (`0x00737770`): unless the key's
    /// model (`+4`) is `0xffff` or the level is `0xec1`, the colour
    /// ([`ee_colour`]); with a non-zero alpha, a render request on the
    /// stack (`frame - 0x80`: flags from the model record's word 0, its
    /// `+0x16` blend, the colour, the part's `+0x78`, the object's matrix
    /// `+0x10` and `+0x50` and textures `+0x90`/`+0x94`) updates the
    /// instance's render data, or creates it (path 3 for flag 4). The
    /// request's `+0xb` byte and `+0x30` word are left as they were.
    /// Returns what the original leaves in eax.
    pub fn ppp_draw_mdl_draw<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        prog: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        if mem.read::<u32>(at(key, 4)) == 0xffff {
            return key;
        }
        let h = call(mem, g(mem, NAME_HASH), &[]);
        if h == LEVEL_SKIP {
            return h;
        }
        let req = frame.wrapping_sub(0x80);
        let colour = ee_colour(mem, obj, prog, true, call);
        store_colour(mem, req, colour);
        if mem.read::<u8>(at(req, 0x1b)) == 0 {
            return colour;
        }
        let rec = model_record(mem, key);
        let mp = call(mem, g(mem, GET_PTR32), &[mem.read(at(rec, 0x1c))]);
        let flags = flags_model(mem.read(mp));
        let tex = textures(mem, obj, call);
        fill_request(mem, req, obj, flags, mem.read(at(mp, 0x16)), tex);
        submit(mem, obj, rec, req, MDL, call)
    }

    /// `pppDrawMdlSemiDraw(obj, key, prog)` (`0x00737b20`): as
    /// [`ppp_draw_mdl_draw`] without the level check, the request at
    /// `frame - 0xa0` with flags `0x80` (`0x90` unless the model's bit 1),
    /// the key's `+8` blend and `+0x44` the object's `+0x50`, and always
    /// the plain (not path-3) entry points with the Z matrix.
    pub fn ppp_draw_mdl_semi_draw<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        prog: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        if mem.read::<u32>(at(key, 4)) == 0xffff {
            return key;
        }
        let req = frame.wrapping_sub(0xa0);
        let colour = ee_colour(mem, obj, prog, false, call);
        store_colour(mem, req, colour);
        if mem.read::<u8>(at(req, 0x1b)) == 0 {
            return colour;
        }
        let rec = model_record(mem, key);
        let mp = call(mem, g(mem, GET_PTR32), &[mem.read(at(rec, 0x1c))]);
        let flags = if mem.read::<u8>(mp) & 2 != 0 {
            0x80
        } else {
            0x90
        };
        let tex = textures(mem, obj, call);
        fill_request(mem, req, obj, flags, mem.read(at(key, 8)), tex);
        mem.write(at(req, 0x44), at(obj, 0x50));
        submit(mem, obj, rec, req, SEMI, call)
    }

    /// `pppDrawMdl2Draw(obj, key, prog)` (`0x00738530`): as
    /// [`ppp_draw_mdl_draw`] without the level check, with
    /// `graphicGetPtrLogCount` (`0x00641370`, its result `n` returned for
    /// the `0xffff` model): the request at `frame - 0xa0`, flags from the
    /// key's `+8` byte ([`flags_key`], with the `== 4` case), `+0x44` the
    /// object's `+0x50`, and `n` added to the colour's three low bytes;
    /// with `n != 0` the first texture is the model's own (`get_ptr32` of
    /// the record's `+0x1c` plus its `+8`).
    pub fn ppp_draw_mdl_2_draw<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        prog: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let n = call(mem, g(mem, GET_PTR_LOG_COUNT), &[]);
        if mem.read::<u32>(at(key, 4)) == 0xffff {
            return n;
        }
        let req = frame.wrapping_sub(0xa0);
        let colour = ee_colour(mem, obj, prog, false, call);
        store_colour(mem, req, colour);
        if mem.read::<u8>(at(req, 0x1b)) == 0 {
            return colour;
        }
        let rec = model_record(mem, key);
        let mp = call(mem, g(mem, GET_PTR32), &[mem.read(at(rec, 0x1c))]);
        let flags = flags_key(mem.read(at(key, 8)), mem.read(mp), true);
        let blend: u16 = mem.read(at(mp, 0x16));
        let t0 = if n == 0 {
            call(mem, g(mem, GET_PTR32), &[mem.read(at(obj, 0x90))])
        } else {
            let m = call(mem, g(mem, GET_PTR32), &[mem.read(at(rec, 0x1c))]);
            let skip: u32 = mem.read(at(m, 8));
            call(mem, g(mem, GET_PTR32), &[mem.read(at(rec, 0x1c))]).wrapping_add(skip)
        };
        let t1 = call(mem, g(mem, GET_PTR32), &[mem.read(at(obj, 0x94))]);
        fill_request(mem, req, obj, flags, blend, [t0, t1]);
        mem.write(at(req, 0x44), at(obj, 0x50));
        #[allow(clippy::cast_possible_truncation)] // add byte [..], al
        for o in 0x18..0x1b {
            let v: u8 = mem.read(at(req, o));
            mem.write(at(req, o), v.wrapping_add(n as u8));
        }
        submit(mem, obj, rec, req, MDL, call)
    }

    /// `pppDrawMdlSemi2Draw(obj, key, prog)` (`0x00738a40`): as
    /// [`ppp_draw_mdl_semi_draw`], with flags from the key's `+0xa` byte
    /// ([`flags_key`], without the `== 4` case) and the Z matrix skipped
    /// for flag `0x2000`. Before the colour, its alpha byte is first
    /// written as `[part + 0x76] * [area + 0xa6] >> 19` (the work area
    /// read without the colour shift). The colour then overwrites it (the
    /// shift is already cleared when the colour is stored), so the alpha
    /// tested is the colour's top byte.
    pub fn ppp_draw_mdl_semi2_draw<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        prog: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        if mem.read::<u32>(at(key, 4)) == 0xffff {
            return key;
        }
        let req = frame.wrapping_sub(0xa0);
        let part: u32 = mem.read(g(mem, CURRENT_PART));
        let v = i32::from(mem.read::<i16>(at(offset(mem, prog, 0), obj).wrapping_add(0xa6)));
        let scale = i32::from(mem.read::<u16>(at(part, 0x76)));
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // mov byte, cl
        mem.write(at(req, 0x1b), (scale.wrapping_mul(v) >> 19) as u8);
        let colour = ee_colour(mem, obj, prog, false, call);
        store_colour(mem, req, colour);
        if mem.read::<u8>(at(req, 0x1b)) == 0 {
            return colour;
        }
        let rec = model_record(mem, key);
        let mp = call(mem, g(mem, GET_PTR32), &[mem.read(at(rec, 0x1c))]);
        let flags = flags_key(mem.read(at(key, 0xa)), mem.read(mp), false);
        let tex = textures(mem, obj, call);
        fill_request(mem, req, obj, flags, mem.read(at(key, 8)), tex);
        mem.write(at(req, 0x44), at(obj, 0x50));
        submit(mem, obj, rec, req, SEMI2, call)
    }

    fn global(mem: &impl Mem, a: u32) -> u32 {
        mem.read(g(mem, a))
    }

    /// The `3` draws' extra request bits from four key bytes: `0x200`,
    /// `0x100`, a view-space matrix (`0x2000`, `+0x30` pointing at
    /// `g_pppViewMatrix x obj + 0x10` built at `matrix`), and a Z-matrix
    /// work area (`+0x44`: `obj + 0xa0 + [[prog + 0xc] + 4 z_index]`, else
    /// 0). Returns the flags.
    #[allow(clippy::too_many_arguments)]
    fn flags_3<M: Mem>(
        mem: &mut M,
        req: u32,
        obj: u32,
        prog: u32,
        mut flags: u32,
        bytes: [u8; 4],
        matrix: u32,
        z_index: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        if bytes[0] != 0 {
            flags |= 0x200;
        }
        if bytes[1] != 0 {
            flags |= 0x100;
        }
        if bytes[2] != 0 {
            flags |= 0x2000;
            mem.write(at(req, 0x30), matrix);
            call(
                mem,
                g(mem, MUL_MATRIX),
                &[matrix, g(mem, VIEW_MATRIX), at(obj, 0x10)],
            );
        }
        let z = if bytes[3] == 0 {
            0
        } else {
            at(obj, offset(mem, prog, z_index)).wrapping_add(0xa0)
        };
        mem.write(at(req, 0x44), z);
        flags
    }

    /// Whether the open magic file (`MagicFile_GetHandle` not `-1`) has an
    /// id in `ids` (`MagicFile_GetMagicId` asked once per id, stopping at
    /// the first match).
    fn magic_is<M: Mem>(
        mem: &mut M,
        ids: &[u32],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> bool {
        if call(mem, g(mem, GET_MAGIC_HANDLE), &[]) == u32::MAX {
            return false;
        }
        ids.iter()
            .any(|&id| call(mem, g(mem, GET_MAGIC_ID), &[]) == id)
    }

    /// `[get_ptr32(m)+ [get_ptr32(m) + 4] + 8] & 0x3fff` for the model
    /// record's `m = +0x1c`: a field of the model's first part.
    fn model_field<M: Mem>(
        mem: &mut M,
        rec: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let m = call(mem, g(mem, GET_PTR32), &[mem.read(at(rec, 0x1c))]);
        let skip: u32 = mem.read(at(m, 4));
        let m = call(mem, g(mem, GET_PTR32), &[mem.read(at(rec, 0x1c))]);
        mem.read::<u32>(m.wrapping_add(skip).wrapping_add(8)) & 0x3fff
    }

    /// The magic files `0x29e`, `0x29f` and `0x2c9` with a `g_VfxPass`:
    /// the object's matrix `+0x50` moved to screen space (`x
    /// g_pppScreenMatrix`, then `x` the inverse of `g_ScreenMatrix` built
    /// at `inverse`), its `+0x30` divided by 1.8 in language `0xb`.
    fn screen_fix<M: Mem>(
        mem: &mut M,
        obj: u32,
        inverse: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        if !magic_is(mem, &[0x29e, 0x29f, 0x2c9], call) || global(mem, VFX_PASS) == 0 {
            return;
        }
        let m = at(obj, 0x50);
        call(
            mem,
            g(mem, INVERSE_MATRIX),
            &[inverse, g(mem, SCREEN_MATRIX)],
        );
        call(mem, g(mem, MUL_MATRIX), &[m, g(mem, PPP_SCREEN_MATRIX), m]);
        call(mem, g(mem, MUL_MATRIX), &[m, inverse, m]);
        if call(mem, g(mem, GET_LANG), &[]) == LANG_SCALED {
            mem.write(at(m, 0x30), store(div(f(mem, at(m, 0x30)), LANG_SCALE)));
        }
    }

    /// The `3` draws' flags from the key's `+8` byte `d`: `d & 4` gives
    /// `0xc` with blend byte `0x44`; `d & 2` gives `4`, or `0xc` with
    /// blend byte `0x44` unless the model's bit 0; else `0x80` for `d & 1`
    /// and `| 8` unless the model's bit 0. The model's `+0x16` blend
    /// otherwise; `| 0x10` unless the model's bit 1.
    fn flags_mdl3(mem: &mut impl Mem, req: u32, d: u8, mp: u32) -> u32 {
        let m0: u8 = mem.read(mp);
        let mut f = 0;
        let own_blend = if d & 4 != 0 {
            f = 0xc;
            false
        } else if d & 2 != 0 {
            f = if m0 & 1 != 0 { 4 } else { 0xc };
            m0 & 1 != 0
        } else {
            if d & 1 != 0 {
                f = 0x80;
            }
            if m0 & 1 == 0 {
                f |= 8;
            }
            true
        };
        if own_blend {
            mem.write(at(req, 8), mem.read::<u16>(at(mp, 0x16)));
        } else {
            mem.write(at(req, 8), 0x44u8);
        }
        if m0 & 2 == 0 { f | 0x10 } else { f }
    }

    /// `pppDrawMdl3Draw(obj, key, prog)` (`0x007394c0`): the level hash
    /// asked (and returned for the `0xffff` model) but unused; the colour
    /// and a request at `frame - 0x120` (flags [`flags_mdl3`] and
    /// [`flags_3`] from the key's `+9`, `+0xb`, `+0xa`, `+0xc`; the view
    /// matrix at `frame - 0x88`), then per-effect fixes: magic `0x296`
    /// (pass 1) clears flag 4; magic `0x241` (pass 1) sets it for model
    /// fields `0x36c0`/`0x3700`; [`screen_fix`]. On creation the Z matrix
    /// also when the key's `+8` has bit 2, and the alpha bits are patched:
    /// `0x40` for magic `0x17a` field `0x37e0`; magics `0x110`/`0x111`
    /// add `0x8000` for flag `0x10` and replace everything for models
    /// `0x1e`/`0x1f` (`0x10000000`) and `0x1a`/`0x1c` (`0x80000000`); magic
    /// `0x231` (pass 1) adds `0x800`; maps `0x17c`, `0x3e`, `0xc6`,
    /// `0xc2`/`0xff` clear bit 0 for some YIQ modes or field effects.
    #[allow(clippy::too_many_lines)]
    pub fn ppp_draw_mdl_3_draw<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        prog: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let h = call(mem, g(mem, NAME_HASH), &[]);
        if mem.read::<u32>(at(key, 4)) == 0xffff {
            return h;
        }
        let req = frame.wrapping_sub(0x120);
        let colour = ee_colour(mem, obj, prog, false, call);
        store_colour(mem, req, colour);
        if mem.read::<u8>(at(req, 0x1b)) == 0 {
            return colour;
        }
        let rec = model_record(mem, key);
        let mp = call(mem, g(mem, GET_PTR32), &[mem.read(at(rec, 0x1c))]);
        let flags = flags_mdl3(mem, req, mem.read(at(key, 8)), mp);
        let bytes = [9, 0xb, 0xa, 0xc].map(|o| mem.read::<u8>(at(key, o)));
        let mut flags = flags_3(
            mem,
            req,
            obj,
            prog,
            flags,
            bytes,
            frame.wrapping_sub(0x88),
            2,
            call,
        );
        let tex = textures(mem, obj, call);
        let (blend, z): (u16, u32) = (mem.read(at(req, 8)), mem.read(at(req, 0x44)));
        fill_request(mem, req, obj, flags, blend, tex);
        mem.write(at(req, 0x44), z);
        if magic_is(mem, &[0x296], call) && global(mem, VFX_PASS) == 1 {
            flags &= !4;
        }
        if magic_is(mem, &[0x241], call) && global(mem, VFX_PASS) == 1 {
            let v = model_field(mem, rec, call);
            if v == 0x36c0 || v == 0x3700 {
                flags |= 4;
            }
        }
        mem.write(req, flags);
        screen_fix(mem, obj, frame.wrapping_sub(0x48), call);
        if has_render_data(mem, obj, call) {
            return update(mem, obj, rec, req, true, call);
        }
        let blend = u32::from(mem.read::<u8>(at(req, 8)));
        let mut alpha = call(mem, g(mem, ALPHA_FUNCTION), &[blend]);
        if mem.read::<u8>(at(key, 8)) & 4 != 0 || flags & 0x2000 == 0 {
            alpha |= call(mem, g(mem, Z_MAT), &[mem.read(at(req, 0x44))]);
        }
        if flags & 8 != 0 {
            alpha |= 0x40;
        }
        if call(mem, g(mem, GET_MAGIC_ID), &[]) == 0x17a
            && global(mem, VFX_PASS) != 0
            && model_field(mem, rec, call) == 0x37e0
        {
            alpha |= 0x40;
        }
        let model: u32 = mem.read(at(key, 4));
        let special = [(0x110u32, 0x1eu32, 0x1au32), (0x111, 0x1f, 0x1c)]
            .into_iter()
            .find(|&(id, _, _)| {
                call(mem, g(mem, GET_MAGIC_ID), &[]) == id && global(mem, VFX_PASS) != 0
            });
        if let Some((_, top, low)) = special {
            if flags & 0x10 != 0 {
                alpha |= 0x8000;
            }
            if model == top {
                alpha = 0x1000_0000;
            } else if model == low {
                alpha = 0x8000_0000;
            }
        } else if magic_is(mem, &[0x231], call) && global(mem, VFX_PASS) == 1 {
            alpha |= 0x800;
        } else {
            let (yiq, fp) = (global(mem, YIQ_MODE), global(mem, FP_ID));
            match global(mem, MAP_NO) {
                0x17c if yiq == 0x19 => alpha = alpha & !1 | 0x800,
                0x3e if matches!(fp, 0x2c..=0x2f) => alpha = alpha & !1 | 0x800,
                0xc6 if yiq == 5 => alpha &= !1,
                0xc2 | 0xff if yiq == 0 => alpha &= !1,
                _ => {}
            }
        }
        create(mem, obj, rec, req, true, alpha, call)
    }

    /// `pppDrawMdlSemi3Draw(obj, key, prog)` (`0x00739d30`): the colour and
    /// a request at `frame - 0x120` (flags [`flags_key`] from the key's
    /// `+0xa` and [`flags_3`] from `+0xb`, `+0xd`, `+0xc`, `+0xe`; the
    /// key's `+8` blend), then [`screen_fix`]. An update is followed by
    /// `graphicDynamicGeometryUpdateTexture` (its eax returned); on
    /// creation a zero blend takes alpha `2` instead of the alpha function.
    pub fn ppp_draw_mdl_semi3_draw<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        prog: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        if mem.read::<u32>(at(key, 4)) == 0xffff {
            return key;
        }
        let req = frame.wrapping_sub(0x120);
        let colour = ee_colour(mem, obj, prog, false, call);
        store_colour(mem, req, colour);
        if mem.read::<u8>(at(req, 0x1b)) == 0 {
            return colour;
        }
        let rec = model_record(mem, key);
        let mp = call(mem, g(mem, GET_PTR32), &[mem.read(at(rec, 0x1c))]);
        let flags = flags_key(mem.read(at(key, 0xa)), mem.read(mp), false);
        let bytes = [0xb, 0xd, 0xc, 0xe].map(|o| mem.read::<u8>(at(key, o)));
        let flags = flags_3(
            mem,
            req,
            obj,
            prog,
            flags,
            bytes,
            frame.wrapping_sub(0x88),
            2,
            call,
        );
        let z: u32 = mem.read(at(req, 0x44));
        let tex = textures(mem, obj, call);
        fill_request(mem, req, obj, flags, mem.read(at(key, 8)), tex);
        mem.write(at(req, 0x44), z);
        screen_fix(mem, obj, frame.wrapping_sub(0x48), call);
        if has_render_data(mem, obj, call) {
            update(mem, obj, rec, req, true, call);
            let id = instance(mem, obj);
            return call(mem, g(mem, UPDATE_TEXTURE), &id);
        }
        let blend: u8 = mem.read(at(req, 8));
        let mut alpha = if blend == 0 {
            2
        } else {
            call(mem, g(mem, ALPHA_FUNCTION), &[u32::from(blend)])
        };
        if flags & 0x2000 == 0 {
            alpha |= call(mem, g(mem, Z_MAT), &[mem.read(at(req, 0x44))]);
        }
        if flags & 8 != 0 {
            alpha |= 0x40;
        }
        create(mem, obj, rec, req, true, alpha, call)
    }

    /// `pppDrawMdlTs3Draw(obj, key, prog)` (`0x0073a370`): the work area
    /// stepped ([`ts_step`]); the colour and a request at `frame - 0xe0`
    /// (flags [`flags_key`] from the key's `+0x20` and [`flags_3`] from
    /// `+0x21`, `+0x23`, `+0x22`, `+0x24`, the Z-matrix work area at offset
    /// 3; the view matrix at `frame - 0x48`; the model's blend, its low
    /// byte `0x48` for magic `0x1f9`; [`ts_fixed`]). In some field effects
    /// of maps `0xc6`, `0x102`, `0x103`/`0xf4`, `0x110`, `0x139` and `0x7a`
    /// flag `0x10` is cleared and alpha bit `0x800` set; on creation magic
    /// `0x231` (pass 1) also sets `0x800`.
    pub fn ppp_draw_mdl_ts3_draw<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        prog: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let (w, eax) = ts_step(mem, obj, key, prog);
        if mem.read::<u32>(at(key, 4)) == 0xffff {
            return eax;
        }
        let req = frame.wrapping_sub(0xe0);
        let colour = ee_colour(mem, obj, prog, false, call);
        store_colour(mem, req, colour);
        if mem.read::<u8>(at(req, 0x1b)) == 0 {
            return colour;
        }
        let rec = model_record(mem, key);
        let mp = call(mem, g(mem, GET_PTR32), &[mem.read(at(rec, 0x1c))]);
        let flags = flags_key(mem.read(at(key, 0x20)), mem.read(mp), false);
        let bytes = [0x21, 0x23, 0x22, 0x24].map(|o| mem.read::<u8>(at(key, o)));
        let mut flags = flags_3(
            mem,
            req,
            obj,
            prog,
            flags,
            bytes,
            frame.wrapping_sub(0x48),
            3,
            call,
        );
        let z: u32 = mem.read(at(req, 0x44));
        let mut blend: u16 = mem.read(at(mp, 0x16));
        if call(mem, g(mem, GET_MAGIC_ID), &[]) == 0x1f9 {
            blend = blend & 0xff00 | 0x48;
        }
        let tex = textures(mem, obj, call);
        fill_request(mem, req, obj, flags, blend, tex);
        ts_fixed(mem, req, w);
        mem.write(at(req, 0x44), z);
        let fp = global(mem, FP_ID);
        let fix = match global(mem, MAP_NO) {
            0xc6 => fp == 0x3f || fp == 0x40,
            0x102 => fp == 0xe,
            0x103 | 0xf4 => fp == 3,
            0x110 => fp == 5 || fp == 4,
            0x139 => fp == 0x12,
            0x7a => fp == 0x41,
            _ => false,
        };
        let mut alpha = 0;
        if fix {
            flags &= !0x10;
            mem.write(req, flags);
            alpha = 0x800;
        }
        if has_render_data(mem, obj, call) {
            return update(mem, obj, rec, req, true, call);
        }
        alpha |= call(
            mem,
            g(mem, ALPHA_FUNCTION),
            &[u32::from(mem.read::<u8>(at(req, 8)))],
        );
        if flags & 0x2000 == 0 {
            alpha |= call(mem, g(mem, Z_MAT), &[mem.read(at(req, 0x44))]);
        }
        if flags & 8 != 0 {
            alpha |= 0x40;
        }
        if magic_is(mem, &[0x231], call) && global(mem, VFX_PASS) == 1 {
            alpha |= 0x800;
        }
        create(mem, obj, rec, req, true, alpha, call)
    }

    /// The `Ts` draws' work area `w = obj + 0xa0 + [[prog + 0xc] + 8]`:
    /// two `[pos, speed, accel]` triples (`+0`, `+0xc`) stepped as in
    /// `pppKeTkFadeDraw` (`speed += accel`, `pos += speed`, the key's
    /// `+8..+0x1c` added on its frame) unless `g_pppStop`. Returns `w` and
    /// what the original leaves in eax (the program, else the key's
    /// frame).
    fn ts_step(mem: &mut impl Mem, obj: u32, key: u32, prog: u32) -> (u32, u32) {
        let w = at(obj, offset(mem, prog, 2)).wrapping_add(0xa0);
        if step_triples(mem, obj, key, w, &[(0, 8), (0xc, 0x14)]) {
            (w, mem.read(key))
        } else {
            (w, prog)
        }
    }

    /// The request's `+0x10` and `+0x14`: the floats at `x` and `y` in
    /// 16.16 fixed point (`ftol(v * 65536)`).
    fn fixed_16_16(mem: &mut impl Mem, req: u32, x: u32, y: u32) {
        let v = ftol(mem, mul(f(mem, x), TS_SCALE));
        mem.write(at(req, 0x10), v);
        let v = ftol(mem, mul(TS_SCALE, f(mem, y)));
        mem.write(at(req, 0x14), v);
    }

    /// The `Ts` draws' request `+0x10`, `+0x14`: the two positions
    /// ([`fixed_16_16`]).
    fn ts_fixed(mem: &mut impl Mem, req: u32, w: u32) {
        fixed_16_16(mem, req, w, at(w, 0xc));
    }

    /// `pppDrawMdlTsDraw(obj, key, prog)` (`0x00737f40`): the work area
    /// stepped ([`ts_step`]); then as [`ppp_draw_mdl_draw`] with the
    /// request at `frame - 0xa0`, `+0x10`/`+0x14` the positions in 16.16
    /// ([`ts_fixed`]), and in level `0x169c` drawn even at alpha 0 (else
    /// returning 0).
    pub fn ppp_draw_mdl_ts_draw<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        prog: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let (w, eax) = ts_step(mem, obj, key, prog);
        if mem.read::<u32>(at(key, 4)) == 0xffff {
            return eax;
        }
        let req = frame.wrapping_sub(0xa0);
        let colour = ee_colour(mem, obj, prog, false, call);
        store_colour(mem, req, colour);
        let always = call(mem, g(mem, NAME_HASH), &[]) == LEVEL_TS_ALWAYS;
        if mem.read::<u8>(at(req, 0x1b)) == 0 && !always {
            return 0;
        }
        let rec = model_record(mem, key);
        let mp = call(mem, g(mem, GET_PTR32), &[mem.read(at(rec, 0x1c))]);
        let flags = flags_model(mem.read(mp));
        let tex = textures(mem, obj, call);
        fill_request(mem, req, obj, flags, mem.read(at(mp, 0x16)), tex);
        ts_fixed(mem, req, w);
        submit(mem, obj, rec, req, MDL, call)
    }

    /// `pppDrawMdlTs2Draw(obj, key, prog)` (`0x00738ec0`): as
    /// [`ppp_draw_mdl_ts_draw`] without the level check, with flags from
    /// the key's `+0x20` byte ([`flags_key`], without the `== 4` case) and
    /// `+0x44` the object's `+0x50`.
    pub fn ppp_draw_mdl_ts2_draw<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        prog: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let (w, eax) = ts_step(mem, obj, key, prog);
        if mem.read::<u32>(at(key, 4)) == 0xffff {
            return eax;
        }
        let req = frame.wrapping_sub(0xa0);
        let colour = ee_colour(mem, obj, prog, false, call);
        store_colour(mem, req, colour);
        if mem.read::<u8>(at(req, 0x1b)) == 0 {
            return colour;
        }
        let rec = model_record(mem, key);
        let mp = call(mem, g(mem, GET_PTR32), &[mem.read(at(rec, 0x1c))]);
        let flags = flags_key(mem.read(at(key, 0x20)), mem.read(mp), false);
        let tex = textures(mem, obj, call);
        fill_request(mem, req, obj, flags, mem.read(at(mp, 0x16)), tex);
        ts_fixed(mem, req, w);
        mem.write(at(req, 0x44), at(obj, 0x50));
        submit(mem, obj, rec, req, MDL, call)
    }

    /// `_CIfmod(x, y)` (the import thunk `0x009498e0`), an `fprem` loop:
    /// the exact remainder with the sign of `x`; a NaN passes, an infinite
    /// `x` gives the indefinite NaN.
    fn fmod(x: f64, y: f64) -> f64 {
        if x.is_nan() {
            x
        } else if x.is_infinite() {
            f64::from_bits(0xfff8_0000_0000_0000)
        } else {
            x % y
        }
    }

    /// The `Loop` draws' work area `w = obj + 0xa0 + [[prog + 0xc] + 8]`:
    /// a `ptr32` to the key at `+0`, set up (with the key's `+8` and
    /// `+0x14` as the two positions, everything else 0) while it is null;
    /// then, unless `g_pppStop`, two `[pos, speed, accel]` channels (`+4`,
    /// `+0x10`) step: on the key's frame its `+0xc`, `+0x10`, `+0x18`,
    /// `+0x1c` are added to the speeds and accelerations, `speed +=
    /// accel`, `pos = fmod(pos + speed, 32768)`. Returns `w`.
    fn loop_step<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        prog: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let w = at(obj, offset(mem, prog, 2)).wrapping_add(0xa0);
        if call(mem, g(mem, IS_PTR32_NULL), &[mem.read(w)]) != 0 {
            mem.write(at(w, 4), store(f(mem, at(key, 8))));
            mem.write(at(w, 0x10), store(f(mem, at(key, 0x14))));
            for o in [0x18u32, 0x14, 0xc, 8] {
                mem.write(at(w, o), 0u32);
            }
            let p = call(mem, g(mem, SET_PTR32), &[key]);
            mem.write(w, p);
        }
        if mem.read::<u32>(g(mem, STOP)) != 0 {
            return w;
        }
        if mem.read::<u32>(key) == mem.read::<u32>(at(obj, 0xc)) {
            for (k, o) in [(0xcu32, 8u32), (0x10, 0xc), (0x18, 0x14), (0x1c, 0x18)] {
                let v = store(add(f(mem, at(key, k)), f(mem, at(w, o))));
                mem.write(at(w, o), v);
            }
        }
        let speed = store(add(f(mem, at(w, 0xc)), f(mem, at(w, 8))));
        mem.write(at(w, 8), speed);
        let speed2 = store(add(f(mem, at(w, 0x18)), f(mem, at(w, 0x14))));
        mem.write(at(w, 0x14), speed2);
        let p = store(add(load(speed), f(mem, at(w, 4))));
        mem.write(at(w, 4), store(fmod(load(p), LOOP_PERIOD)));
        let q = store(add(f(mem, at(w, 0x10)), load(speed2)));
        mem.write(at(w, 0x10), store(fmod(load(q), LOOP_PERIOD)));
        w
    }

    /// Where a `Loop` draw puts the object's matrices.
    #[derive(Debug, Clone, Copy)]
    enum LoopSpace {
        /// The object's own `+0x50` and `+0x10` (`Loop`, `LoopZ`).
        Object,
        /// Relative to the running part's position (`LoopDisPos`): `M =
        /// T(-g_pppWorldMatrix position) x obj + 0x10` at `frame - 0x88`,
        /// and `g_pppViewMatrixCopy x M` at `frame - 0xc8`, in place of
        /// `+0x10` and `+0x50`.
        PartRelative,
    }

    /// One `Loop` draw: the request at `frame - req_off`, the view-space
    /// matrix at `frame - view_off`, the Z-matrix work area from the key's
    /// `+0x26` (`z`), the map fix (`map_fix`), path 3 (`path3`).
    #[derive(Debug, Clone, Copy)]
    struct LoopKind {
        req_off: u32,
        view_off: u32,
        z: bool,
        map_fix: bool,
        path3: bool,
        space: LoopSpace,
    }

    /// The `Loop` draws: [`loop_step`]; unless the key's model is
    /// `0xffff` (returning `[prog + 0xc]`), the colour; with a non-zero
    /// alpha a request with flags [`flags_key`] from the key's `+0x22`,
    /// `0x200`/`0x100` for `+0x23`/`+0x24`, for `+0x25` a view-space matrix
    /// (`0x2000` instead of `0x10`; `+0x30` 0 otherwise), the key's `+0x20`
    /// blend (the model's for `0xff`), `+0x1c` 0, the two positions in
    /// 16.16.
    #[allow(clippy::too_many_lines)]
    fn mdl_loop<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        prog: u32,
        frame: u32,
        kind: LoopKind,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let table: u32 = mem.read(at(prog, 0xc));
        if mem.read::<u32>(at(key, 4)) == 0xffff {
            return table;
        }
        let rec = model_record(mem, key);
        let mp = call(mem, g(mem, GET_PTR32), &[mem.read(at(rec, 0x1c))]);
        let w = loop_step(mem, obj, key, prog, call);
        let req = frame.wrapping_sub(kind.req_off);
        let colour = ee_colour(mem, obj, prog, false, call);
        store_colour(mem, req, colour);
        if mem.read::<u8>(at(req, 0x1b)) == 0 {
            return colour;
        }
        let (world, local) = match kind.space {
            LoopSpace::Object => (at(obj, 0x50), at(obj, 0x10)),
            LoopSpace::PartRelative => {
                let (unit, local, world) = (
                    frame.wrapping_sub(0x48),
                    frame.wrapping_sub(0x88),
                    frame.wrapping_sub(0xc8),
                );
                call(mem, g(mem, UNIT_MATRIX), &[unit]);
                let t = at(unit, 0x30);
                call(mem, g(mem, COPY_VECTOR), &[t, g(mem, WORLD_POSITION)]);
                for o in [0u32, 4, 8] {
                    mem.write(at(t, o), store(-f(mem, at(t, o))));
                }
                call(mem, g(mem, MUL_MATRIX), &[local, unit, at(obj, 0x10)]);
                call(
                    mem,
                    g(mem, MUL_MATRIX),
                    &[world, g(mem, VIEW_MATRIX_COPY), local],
                );
                (world, local)
            }
        };
        let mut flags = flags_key(mem.read(at(key, 0x22)), mem.read(mp), false);
        if mem.read::<u8>(at(key, 0x23)) != 0 {
            flags |= 0x200;
        }
        if mem.read::<u8>(at(key, 0x24)) != 0 {
            flags |= 0x100;
        }
        let view = if mem.read::<u8>(at(key, 0x25)) == 0 {
            0
        } else {
            flags = flags & !0x10 | 0x2000;
            let m = frame.wrapping_sub(kind.view_off);
            call(mem, g(mem, MUL_MATRIX), &[m, g(mem, VIEW_MATRIX), local]);
            m
        };
        let blend: u16 = if mem.read::<u8>(at(key, 0x20)) == 0xff {
            mem.read(at(mp, 0x16))
        } else {
            mem.read(at(key, 0x20))
        };
        let tex = textures(mem, obj, call);
        fill_request(mem, req, obj, flags, blend, tex);
        fixed_16_16(mem, req, at(w, 4), at(w, 0x10));
        mem.write(at(req, 0x1c), 0u32);
        mem.write(at(req, 0x20), world);
        mem.write(at(req, 0x2c), local);
        mem.write(at(req, 0x30), view);
        if kind.z && mem.read::<u8>(at(key, 0x26)) != 0 {
            mem.write(
                at(req, 0x44),
                at(obj, offset(mem, prog, 3)).wrapping_add(0xa0),
            );
        }
        if kind.map_fix {
            let fp = global(mem, FP_ID);
            let fix = match global(mem, MAP_NO) {
                0xc6 => fp == 0x3f || fp == 0x40,
                0x102 => fp == 0xe,
                0x103 | 0xf4 => fp == 3,
                0x110 => fp == 5 || fp == 4,
                0x139 => fp == 0x12,
                0x7a => fp == 0x41,
                _ => false,
            };
            if fix {
                mem.write(req, flags & !0x10);
            }
        }
        let rec = model_record(mem, key);
        let how = Submit {
            path3: kind.path3,
            z_test: true,
        };
        submit(mem, obj, rec, req, how, call)
    }

    /// `pppDrawMdlLoopDraw(obj, key, prog)` (`0x0073e2b0`): [`mdl_loop`]
    /// with the request at `frame - 0xe0`, the view matrix at `frame -
    /// 0x48`, path 3 for flag 4.
    pub fn ppp_draw_mdl_loop_draw<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        prog: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let kind = LoopKind {
            req_off: 0xe0,
            view_off: 0x48,
            z: false,
            map_fix: false,
            path3: true,
            space: LoopSpace::Object,
        };
        mdl_loop(mem, obj, key, prog, frame, kind, call)
    }

    /// `pppDrawMdlLoopZDraw(obj, key, prog)` (`0x0073ea40`): as
    /// [`ppp_draw_mdl_loop_draw`] with the Z-matrix work area at offset 3
    /// for the key's `+0x26`, flag `0x10` cleared in some field effects of
    /// maps `0xc6`, `0x102`, `0x103`/`0xf4`, `0x110`, `0x139` and `0x7a`
    /// (as `pppDrawMdlTs3Draw`, without its alpha bit), and no path 3.
    pub fn ppp_draw_mdl_loop_z_draw<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        prog: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let kind = LoopKind {
            req_off: 0xe0,
            view_off: 0x48,
            z: true,
            map_fix: true,
            path3: false,
            space: LoopSpace::Object,
        };
        mdl_loop(mem, obj, key, prog, frame, kind, call)
    }

    /// `pppDrawMdlLoopDisPosDraw(obj, key, prog)` (`0x0073f230`): as
    /// [`ppp_draw_mdl_loop_draw`] with the object's matrices relative to
    /// the running part's position ([`LoopSpace::PartRelative`]), the
    /// request at `frame - 0x1a0`, the view matrix at `frame - 0x108`, the
    /// Z-matrix work area at offset 3 for the key's `+0x26`, and no path 3.
    pub fn ppp_draw_mdl_loop_dis_pos_draw<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        prog: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let kind = LoopKind {
            req_off: 0x1a0,
            view_off: 0x108,
            z: true,
            map_fix: false,
            path3: false,
            space: LoopSpace::PartRelative,
        };
        mdl_loop(mem, obj, key, prog, frame, kind, call)
    }

    const MK_TK_CLUT: u32 = 0x0076_7530;
    const KE_MDL_DRAW: u32 = 0x0076_6ff0;

    /// Steps `[pos, speed, accel]` triples of the work area `w` (each
    /// `(area offset, key offset)`) unless `g_pppStop`: `speed += accel`,
    /// `pos += speed` for every triple, then on the key's frame (`[key] ==
    /// [obj + 0xc]`) each triple's three key floats added to the position
    /// and speed still in registers and to the acceleration in memory.
    /// Returns whether it stepped.
    fn step_triples(
        mem: &mut impl Mem,
        obj: u32,
        key: u32,
        w: u32,
        triples: &[(u32, u32)],
    ) -> bool {
        if mem.read::<u32>(g(mem, STOP)) != 0 {
            return false;
        }
        let mut regs = [(0u32, 0u32); 3];
        for (r, &(o, _)) in regs.iter_mut().zip(triples) {
            let speed = store(add(f(mem, at(w, o + 8)), f(mem, at(w, o + 4))));
            mem.write(at(w, o + 4), speed);
            let pos = store(add(f(mem, at(w, o)), load(speed)));
            mem.write(at(w, o), pos);
            *r = (pos, speed);
        }
        if mem.read::<u32>(key) != mem.read::<u32>(at(obj, 0xc)) {
            return true;
        }
        for (&(pos, speed), &(o, k)) in regs.iter().zip(triples) {
            let v = store(add(load(pos), f(mem, at(key, k))));
            mem.write(at(w, o), v);
            let v = store(add(load(speed), f(mem, at(key, k + 4))));
            mem.write(at(w, o + 4), v);
            let v = store(add(f(mem, at(key, k + 8)), f(mem, at(w, o + 8))));
            mem.write(at(w, o + 8), v);
        }
        true
    }

    /// The three `pppKeMdlTfd*` draw families' `KeMdl_Draw` arguments.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Tfd {
        /// `Tfd`, `TfdUv`: mode 1, no Z matrix.
        One,
        /// `Tfd2`, `TfdUv2`: the mode from a key byte.
        Two,
        /// `Tfd3`, `TfdUv3`: the mode from five key bytes OR-ed, the Z
        /// matrix the work area at offset 3.
        Three,
    }

    /// `pppKeMdlTfdDraw` (`0x00749810`), `pppKeMdlTfd2Draw` (`0x00749b40`),
    /// `pppKeMdlTfd3Draw` (`0x00749e90`), and with `uv`
    /// `pppKeMdlTfdUvDraw` (`0x00749950`), `pppKeMdlTfdUv2Draw`
    /// (`0x00749c90`), `pppKeMdlTfdUv3Draw` (`0x00749ff0`) `(obj, key,
    /// prog)`: a fading model. The work area `w = obj + 0xa0 + [[prog +
    /// 0xc] + 8]` holds one `[pos, speed, accel]` triple (three with
    /// `uv`, at `+0x10` and `+0x1c`, key floats `+0x20`, `+0x2c`), stepped
    /// ([`step_triples`]). Unless the key's model (`+4`) is `0xffff`,
    /// `KeMdl_MkTkClut(key + 0x14, key + 0x18, pos, 0)`, then
    /// `KeMdl_Draw(obj, model, obj + 0xa0 + [[prog + 0xc]], key[0x1c] << 2,
    /// u, v, mode, zmat, part + 0x70, part[0x78])` with `u, v` the other
    /// two positions truncated (`uv`) or 0. Returns what the original
    /// leaves in eax.
    pub fn ppp_ke_mdl_tfd_draw<M: Mem>(
        mem: &mut M,
        kind: Tfd,
        uv: bool,
        obj: u32,
        key: u32,
        prog: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let table: u32 = mem.read(at(prog, 0xc));
        let w = at(obj, 0xa0).wrapping_add(mem.read(at(table, 8)));
        let triples: &[(u32, u32)] = if uv {
            &[(0, 8), (0x10, 0x20), (0x1c, 0x2c)]
        } else {
            &[(0, 8)]
        };
        let eax = if step_triples(mem, obj, key, w, triples) {
            mem.read(at(obj, 0xc))
        } else {
            table
        };
        let model: u32 = mem.read(at(key, 4));
        if model == 0xffff {
            return eax;
        }
        let shift = u32::from(mem.read::<u8>(at(key, 0x1c))) << 2;
        let pos = store(f(mem, w));
        let clut = [mem.read(at(key, 0x14)), mem.read(at(key, 0x18)), pos, 0];
        call(mem, g(mem, MK_TK_CLUT), &clut);
        let part: u32 = mem.read(g(mem, CURRENT_PART));
        let (u, v) = if uv {
            (
                ftol(mem, f(mem, at(w, 0x10))),
                ftol(mem, f(mem, at(w, 0x1c))),
            )
        } else {
            (0, 0)
        };
        let bytes = if uv { 0x38 } else { 0x1d };
        let (mode, zmat) = match kind {
            Tfd::One => (1, 0),
            Tfd::Two => (u32::from(mem.read::<u8>(at(key, bytes))), 0),
            Tfd::Three => {
                let any = (bytes..bytes + 5).fold(0u8, |a, o| a | mem.read::<u8>(at(key, o)));
                (
                    u32::from(any),
                    at(obj, 0xa0).wrapping_add(mem.read(at(table, 0xc))),
                )
            }
        };
        let col = at(obj, 0xa0).wrapping_add(mem.read(table));
        let args = [
            obj,
            model,
            col,
            shift,
            u,
            v,
            mode,
            zmat,
            at(part, 0x70),
            mem.read(at(part, 0x78)),
        ];
        call(mem, g(mem, KE_MDL_DRAW), &args)
    }

    /// `pppDrawMdlCameraLoopInit_007400f0(rec, area)` (`0x007400f0`): the
    /// work area of `pppDrawMdlCameraLoopDraw` on its first frame: `+0` =
    /// `set_ptr32(rec)`, the floats `+4` = `rec +8` and `+0x10` = `rec
    /// +0x14` (copied through the x87), `+8`, `+0xc`, `+0x14`, `+0x18` 0;
    /// the byte `+0x1c` = 1 when any of the floats `rec +0xc`, `+0x10`,
    /// `+0x18`, `+0x1c` is not 0 (a NaN is not), `+0x1d` = `rec +0x23` and
    /// `u16 +0x1e` 0. With the byte `rec +0x21` set, the texture
    /// `Yn_FpTrxTexGet(rec +0x22)` (when there is one) is pointed at the
    /// image and CLUT words `+8`/`+0xc` past the shape record's (`[env
    /// +0x20]` entry `rec +4`, through `get_ptr32_func`) header offset `+4`
    /// (`Yn_FpTrxTexSetAddr`), and `+0x1e`, `+0x1f` take
    /// `Yn_EmbeddedTrxGetCnt`'s byte. Callees through `call`.
    pub fn ppp_draw_mdl_camera_loop_init<M: Mem>(
        mem: &mut M,
        rec: u32,
        area: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let env: u32 = mem.read(mem.read::<u32>(g(mem, ENV_SLOT)));
        let shapes: u32 = mem.read(env.wrapping_add(0x20));
        let index: u32 = mem.read(rec.wrapping_add(4));
        let word: u32 = mem.read((index << 5).wrapping_add(0x1c).wrapping_add(shapes));
        let shape = call(mem, g(mem, GET_PTR32), &[word]);
        let me = call(mem, g(mem, SET_PTR32), &[rec]);
        mem.write(area, me);
        let copy = |mem: &M, off: u32| crate::x87::load(mem.read(rec.wrapping_add(off)));
        mem.write(at(area, 4), copy(mem, 8));
        mem.write(at(area, 0x1c), 0u8);
        mem.write(at(area, 0x10), copy(mem, 0x14));
        for off in [0x18, 0x14, 0xc, 8] {
            mem.write(at(area, off), 0u32);
        }
        let moving = [0xc, 0x10, 0x18, 0x1c]
            .iter()
            .any(|&off| f32::from_bits(mem.read(rec.wrapping_add(off))) != 0.0);
        if moving {
            mem.write(at(area, 0x1c), 1u8);
        }
        let b: u8 = mem.read(rec.wrapping_add(0x23));
        mem.write(at(area, 0x1d), b);
        mem.write(at(area, 0x1e), 0u16);
        if mem.read::<u8>(rec.wrapping_add(0x21)) == 0 {
            return;
        }
        let id = u32::from(mem.read::<u8>(rec.wrapping_add(0x22)));
        let tex = call(mem, g(mem, 0x0091_9ba0), &[id]);
        if tex == 0 {
            return;
        }
        let header = shape.wrapping_add(mem.read(shape.wrapping_add(4)));
        let image: u32 = mem.read(header.wrapping_add(8));
        let clut: u32 = mem.read(header.wrapping_add(0xc));
        call(mem, g(mem, 0x0091_9be0), &[tex, image, clut]);
        #[allow(clippy::cast_possible_truncation)] // the byte the original keeps
        let count = call(mem, g(mem, 0x0091_9bc0), &[tex]) as u8;
        mem.write(at(area, 0x1e), [count, count]);
    }

    /// `ppp_mdl_rev_random_flags_00740960(rec, out)` (`0x00740960`, from
    /// `pppDrawMdlRevDraw`): `out[0]` = 1; then for each of the bytes
    /// `rec +0xd..=+0xf` that is set, `op_random(1.0)` (through `fcall`)
    /// and `out[1 + j]` = 1 when it came out above 0 (a NaN does not).
    /// The other flags are left as they were.
    pub fn ppp_mdl_rev_random_flags<M: Mem>(
        mem: &mut M,
        rec: u32,
        out: u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) {
        mem.write(out, 1u8);
        for j in 0..3 {
            if mem.read::<u8>(rec.wrapping_add(0xd + j)) == 0 {
                continue;
            }
            let r = fcall(mem, g(mem, 0x007e_6700), &[1.0f32.to_bits()]);
            if r > 0.0 {
                mem.write(out.wrapping_add(1 + j), 1u8);
            }
        }
    }

    /// `0x0073dfe0(rec, m)` (`pppDrawMdlCameraTex`, ours; for
    /// `pppDrawMdlCameraDraw`): nothing for a texture `rec +4` of 0xffff
    /// or one the environment's table (`[env +0x20] + 32 n + 0x1c`,
    /// `get_ptr32`) has no data for (`p`). Otherwise `m` becomes the unit
    /// matrix, three `vf00` copies at `+0x40`, `+0x50`, `+0x60`, six 0.0
    /// from `+0x70`, `+0x88` = `set_ptr32(rec)`, byte `+0x8c` = `rec
    /// +0x21`, `u16 +0x8d` = 0; when `Yn_FpTrxTexGet(byte rec +0x20)` gives
    /// a texture `t`, `Yn_FpTrxTexSetAddr(t, [p + [p +4] + 8], [.. + 0xc])`
    /// and bytes `+0x8d`, `+0x8e` = `Yn_EmbeddedTrxGetCnt(t)`; byte `+0x8f`
    /// = 0. The `Yn_*` callees through `call`.
    #[allow(clippy::many_single_char_names)]
    pub fn ppp_draw_mdl_camera_tex<M: Mem>(
        mem: &mut M,
        rec: u32,
        m: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_ps2::vu0::bridge as vu0;
        let n: u32 = mem.read(rec.wrapping_add(4));
        if n == 0xffff {
            return;
        }
        let env: u32 = mem.read(mem.read::<u32>(g(mem, ENV_SLOT)));
        let table: u32 = mem.read(env.wrapping_add(0x20));
        let p = ffx_system::stub::ptr32(mem.read(table.wrapping_add(n << 5).wrapping_add(0x1c)));
        if p == 0 {
            return;
        }
        vu0::matrix_identity(mem, m);
        let vf00: [u32; 4] = mem.read(g(mem, 0x00c0_a004));
        for k in 0..3 {
            mem.write(m.wrapping_add(0x40 + 0x10 * k), vf00);
        }
        mem.write(m.wrapping_add(0x70), [0f32; 6]);
        mem.write(m.wrapping_add(0x88), ffx_system::stub::ptr32(rec));
        mem.write(m.wrapping_add(0x8c), mem.read::<u8>(rec.wrapping_add(0x21)));
        mem.write(m.wrapping_add(0x8d), 0u16);
        let no = u32::from(mem.read::<u8>(rec.wrapping_add(0x20)));
        let t = call(mem, g(mem, 0x0091_9ba0), &[no]);
        if t != 0 {
            let q = p.wrapping_add(mem.read(p.wrapping_add(4)));
            let (image, clut) = (mem.read(q.wrapping_add(8)), mem.read(q.wrapping_add(0xc)));
            call(mem, g(mem, 0x0091_9be0), &[t, image, clut]);
            #[allow(clippy::cast_possible_truncation)] // mov byte, al
            let count = call(mem, g(mem, 0x0091_9bc0), &[t]) as u8;
            mem.write(m.wrapping_add(0x8d), count);
            mem.write(m.wrapping_add(0x8e), count);
        }
        mem.write(m.wrapping_add(0x8f), 0u8);
    }

    // ---- round 49 b ----

    /// `pppDrawMdlPSimDraw(obj, key, prog)` (`0x0073d070`): the cloth
    /// (particle simulation) model draw. The work area `w = obj + 0xa0 +
    /// [[prog + 0xc] + 8]` holds the simulation (`ptr32 +0`, made by
    /// `pppDrawMdlPSimInit(obj, key, w)` while `+0x1c` is null) and two
    /// `[pos, speed, accel]` channels (`+4`, `+0x10`). Unless the key's
    /// model (`+4`) is `0xffff` (returning `[[prog + 0xc] + 8]`) or its
    /// record's `ptr32 +0x1c` is null (or the set-up fails: 0), and
    /// unless `g_pppStop`, the channels step as the `Loop` draws' (on the
    /// key's frame its `+8..+0x1c` added, `speed += accel`, `pos =
    /// fmod(pos + speed, 32768)`). The request is the simulation's OMD
    /// (`rcbgPSimGetOmdIn`): the colour ([`ee_colour`], the `pextlh`
    /// calls) in `+0x18`; with a zero alpha (`+0x1b`) it returns the
    /// colour. Else `+4`, the key's `+0x20` blend (the model's `+0x16` for
    /// `0xff`), `+0xa`/`+0xc` cleared, the positions in 16.16
    /// ([`fixed_16_16`]), `+0x30` a view-space matrix (`g_pppViewMatrix x
    /// obj + 0x10` at `frame - 0x44`) for key byte `+0x75` (else 0), the
    /// object's matrices and textures, the tables and the packet position;
    /// its flags word (`+0`) is left alone. `rcbgPSimProc(sim)` steps the
    /// cloth, then the render data is updated or created ([`submit`],
    /// [`SEMI2`]); returns that eax.
    #[allow(clippy::too_many_lines)]
    pub fn ppp_draw_mdl_psim_draw<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        prog: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        const PSIM_INIT: u32 = 0x0073_d4d0;
        const GET_OMD_IN: u32 = 0x0091_fdc0;
        const PSIM_PROC: u32 = 0x0091_fe70;
        let off = offset(mem, prog, 2);
        let w = at(obj, off).wrapping_add(0xa0);
        if mem.read::<u32>(at(key, 4)) == 0xffff {
            return off;
        }
        let rec = model_record(mem, key);
        let mp = call(mem, g(mem, GET_PTR32), &[mem.read(at(rec, 0x1c))]);
        if mp == 0 {
            return 0;
        }
        if call(mem, g(mem, IS_PTR32_NULL), &[mem.read(at(w, 0x1c))]) != 0
            && call(mem, g(mem, PSIM_INIT), &[obj, key, w]) == 0
        {
            return 0;
        }
        if mem.read::<u32>(g(mem, STOP)) == 0 {
            if mem.read::<u32>(key) == mem.read::<u32>(at(obj, 0xc)) {
                for o in (4u32..=0x18).step_by(4) {
                    let v = store(add(f(mem, at(key, o + 4)), f(mem, at(w, o))));
                    mem.write(at(w, o), v);
                }
            }
            let speed = store(add(f(mem, at(w, 0xc)), f(mem, at(w, 8))));
            mem.write(at(w, 8), speed);
            let speed2 = store(add(f(mem, at(w, 0x18)), f(mem, at(w, 0x14))));
            mem.write(at(w, 0x14), speed2);
            let p = store(add(load(speed), f(mem, at(w, 4))));
            mem.write(at(w, 4), store(fmod(load(p), LOOP_PERIOD)));
            let q = store(add(load(speed2), f(mem, at(w, 0x10))));
            mem.write(at(w, 0x10), store(fmod(load(q), LOOP_PERIOD)));
        }
        let sim = call(mem, g(mem, GET_PTR32), &[mem.read(w)]);
        let req = call(mem, g(mem, GET_OMD_IN), &[sim]);
        let colour = ee_colour(mem, obj, prog, true, call);
        store_colour(mem, req, colour);
        if mem.read::<u8>(at(req, 0x1b)) == 0 {
            return colour;
        }
        mem.write(at(req, 4), mem.read::<u32>(g(mem, REQUEST_WORD4)));
        let blend: u16 = if mem.read::<u8>(at(key, 0x20)) == 0xff {
            mem.read(at(mp, 0x16))
        } else {
            mem.read(at(key, 0x20))
        };
        mem.write(at(req, 8), blend);
        mem.write(at(req, 0xa), 0u8);
        mem.write(at(req, 0xc), 0u32);
        fixed_16_16(mem, req, at(w, 4), at(w, 0x10));
        if mem.read::<u8>(at(key, 0x75)) == 0 {
            mem.write(at(req, 0x30), 0u32);
        } else {
            let m = frame.wrapping_sub(0x44);
            mem.write(at(req, 0x30), m);
            call(
                mem,
                g(mem, MUL_MATRIX),
                &[m, g(mem, VIEW_MATRIX), at(obj, 0x10)],
            );
        }
        mem.write(at(req, 0x1c), 0u32);
        mem.write(at(req, 0x20), at(obj, 0x50));
        mem.write(at(req, 0x44), 0u32);
        mem.write(at(req, 0x24), g(mem, REQUEST_TABLES[0]));
        mem.write(at(req, 0x28), g(mem, REQUEST_TABLES[1]));
        mem.write(at(req, 0x2c), at(obj, 0x10));
        mem.write(at(req, 0x34), mem.read::<u32>(g(mem, PACKET_POS)));
        let tex = textures(mem, obj, call);
        mem.write(at(req, 0x38), tex[0]);
        mem.write(at(req, 0x3c), tex[1]);
        mem.write(at(req, 0x40), 0u32);
        let sim = call(mem, g(mem, GET_PTR32), &[mem.read(w)]);
        call(mem, g(mem, PSIM_PROC), &[sim]);
        let rec = model_record(mem, key);
        submit(mem, obj, rec, req, SEMI2, call)
    }

    // ---- round 49 c ----

    /// `pppDrawMdlRevDraw(obj, key, prog)` (`0x00740410`): unless the
    /// key's model (`+4`) is `0xffff` (returning work area 5, at offset
    /// `[[prog + 0xc] + 0x14]`): the flip flags drawn while area 5's first
    /// byte is 0 (`ppp_mdl_rev_random_flags(key, area)`), the colour of
    /// work area 3 ([`ee_colour_at`], inlined) into the request at
    /// `frame - 0xe0`; with a non-zero alpha the matrix
    /// (`crate::mat::bridge::ppp_mdl_rev_matrix`), a request with flags
    /// ([`flags_key`]) from the key's `+8`, `0x200` for `+9`, `0x100` for
    /// `+0xb`, and for `+0xa` `0x2000` and a view-space matrix (the view
    /// times `obj + 0x10`, at `frame - 0x48`, into `+0x30`; else `+0x30`
    /// is left as it was), the model's blend, `+0x44` work area 4 for key
    /// byte `+0xc`; then [`submit`] ([`SEMI2`]). Returns what the original
    /// leaves in eax (the colour when the alpha is 0).
    pub fn ppp_draw_mdl_rev_draw<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        prog: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let flips = at(obj, offset(mem, prog, 5)).wrapping_add(0xa0);
        if mem.read::<u32>(at(key, 4)) == 0xffff {
            return flips;
        }
        if mem.read::<u8>(flips) == 0 {
            call(mem, g(mem, 0x0074_0960), &[key, flips]);
        }
        let req = frame.wrapping_sub(0xe0);
        let base = at(obj, offset(mem, prog, 3));
        let colour = ee_colour_at(mem, base, false, call);
        store_colour(mem, req, colour);
        if mem.read::<u8>(at(req, 0x1b)) == 0 {
            return colour;
        }
        let rec = model_record(mem, key);
        let mp = call(mem, g(mem, GET_PTR32), &[mem.read(at(rec, 0x1c))]);
        crate::mat::bridge::ppp_mdl_rev_matrix(mem, obj, prog, call);
        let mut flags = flags_key(mem.read(at(key, 8)), mem.read(mp), false);
        if mem.read::<u8>(at(key, 9)) != 0 {
            flags |= 0x200;
        }
        if mem.read::<u8>(at(key, 0xb)) != 0 {
            flags |= 0x100;
        }
        if mem.read::<u8>(at(key, 0xa)) != 0 {
            flags |= 0x2000;
            let m = frame.wrapping_sub(0x48);
            mem.write(at(req, 0x30), m);
            call(
                mem,
                g(mem, MUL_MATRIX),
                &[m, g(mem, VIEW_MATRIX), at(obj, 0x10)],
            );
        }
        let z = if mem.read::<u8>(at(key, 0xc)) == 0 {
            0
        } else {
            at(obj, offset(mem, prog, 4)).wrapping_add(0xa0)
        };
        let tex = textures(mem, obj, call);
        fill_request(mem, req, obj, flags, mem.read(at(mp, 0x16)), tex);
        mem.write(at(req, 0x44), z);
        submit(mem, obj, rec, req, SEMI2, call)
    }
}
