//! The ppp draw programs' set-up (`Con`) and tear-down (`Des`) records and
//! draws: the per-instance work areas they initialise and release.

/// The functions over game memory.
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    fn g(mem: &impl Mem, a: u32) -> u32 {
        Global::<u32>::new(a).ptr(mem).addr
    }

    const GET_PTR32: u32 = 0x00a4_4650;
    const SET_PTR32: u32 = 0x00a4_4700;
    const UNIT_MATRIX: u32 = 0x006e_d9a0;
    const VFX_DESTROY: u32 = 0x0064_5280;
    const MEM_FREE: u32 = 0x0071_6a90;
    const NAME_HASH: u32 = 0x0064_1da0;
    /// `g_pppEnvSlot`: where the current ppp environment pointer is kept.
    pub const ENV_SLOT: u32 = 0x00c3_a4cc;
    /// `g_pppCurrentProgram`.
    pub const CURRENT_PROGRAM: u32 = 0x0230_fd20;

    /// The 64-bit renderer instance of object `obj` in the current program:
    /// `(program, obj + sign(program))`.
    fn instance(mem: &impl Mem, obj: u32) -> [u32; 2] {
        let prog: u32 = mem.read(g(mem, CURRENT_PROGRAM));
        [
            prog,
            obj.wrapping_add(prog.cast_signed().wrapping_shr(31).cast_unsigned()),
        ]
    }

    /// A word of the set-up: a null pointer (`set_ptr32(0)`) or 0.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Init {
        /// `set_ptr32(0)`.
        Null,
        /// 0.
        Zero,
    }

    /// One `pppDrawShape*Con` layout.
    #[derive(Debug, Clone, Copy)]
    pub struct ConLayout {
        /// The words from `+0x40` of the work area.
        pub words: &'static [Init],
        /// The shape record's (`[[obj +8]]`) end and start fields of the
        /// frame count.
        pub span: (u32, u32),
    }

    /// `pppDrawShapeFieldCon` (`0x00742b50`).
    pub const FIELD_CON: ConLayout = ConLayout {
        words: &[Init::Null, Init::Null, Init::Zero, Init::Zero, Init::Null],
        span: (0x1c, 0x18),
    };
    /// `pppDrawShapeFieldRevCon` (`0x00743f00`).
    pub const FIELD_REV_CON: ConLayout = ConLayout {
        words: &[Init::Null, Init::Null, Init::Zero, Init::Zero, Init::Null],
        span: (0x14, 0x10),
    };
    /// `pppDrawShapeFieldSpdCon` (`0x007450a0`).
    pub const FIELD_SPD_CON: ConLayout = ConLayout {
        words: &[
            Init::Null,
            Init::Null,
            Init::Zero,
            Init::Zero,
            Init::Null,
            Init::Null,
        ],
        span: (0x14, 0x10),
    };
    /// `pppDrawShapeFieldGlobalCon` (`0x00746200`).
    pub const FIELD_GLOBAL_CON: ConLayout = ConLayout {
        words: &[Init::Null, Init::Zero, Init::Zero, Init::Null],
        span: (0x1c, 0x18),
    };
    /// `pppDrawShapeCameraCon` (`0x00747410`) and
    /// `pppDrawShapeCameraDisPosCon` (`0x00748660`).
    pub const CAMERA_CON: ConLayout = ConLayout {
        words: &[Init::Null, Init::Zero, Init::Zero, Init::Null],
        span: (0x14, 0x10),
    };

    /// A `pppDrawShape*Con(obj, prog)`: the work area `obj + 0xa0 +
    /// [[prog +0xc]]` gets a unit matrix, then `layout`'s words from `+0x40`,
    /// then the frame count `(end - start) / 4096 + 1` of the shape record
    /// `[[obj +8]]` (through `get_ptr32_func`) and a 0. Returns the count.
    pub fn shape_con<M: Mem>(
        mem: &mut M,
        obj: u32,
        prog: u32,
        layout: ConLayout,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let t: u32 = mem.read(prog.wrapping_add(0xc));
        let off: u32 = mem.read(t);
        let w = obj.wrapping_add(0xa0).wrapping_add(off);
        let (get, set) = (g(mem, GET_PTR32), g(mem, SET_PTR32));
        let s8: u32 = mem.read(obj.wrapping_add(8));
        let p = call(mem, get, &[s8]);
        let pp: u32 = mem.read(p);
        let shape = call(mem, get, &[pp]);
        call(mem, g(mem, UNIT_MATRIX), &[w]);
        let mut at = w.wrapping_add(0x40);
        for &k in layout.words {
            let x = match k {
                Init::Null => call(mem, set, &[0]),
                Init::Zero => 0,
            };
            mem.write(at, x);
            at = at.wrapping_add(4);
        }
        let end: i32 = mem.read(shape.wrapping_add(layout.span.0));
        let start: i32 = mem.read(shape.wrapping_add(layout.span.1));
        let n = (end.wrapping_sub(start) / 0x1000).wrapping_add(1);
        mem.write(at, n);
        mem.write(at.wrapping_add(4), 0u32);
        n.cast_unsigned()
    }

    /// A pointer the tear-down releases.
    #[derive(Debug, Clone, Copy)]
    pub struct Free {
        /// Its offset in the work area.
        pub off: u32,
        /// Freed only when not null (else always).
        pub check: bool,
        /// Then set to `set_ptr32(0)`.
        pub reset: bool,
    }

    /// How a `Des` destroys the renderer instance.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Destroy {
        /// Not at all.
        Never,
        /// First.
        First,
        /// First, unless the level's name hash is one of these.
        FirstUnless(&'static [u32]),
        /// First and again at the end (its result returned).
        Twice,
    }

    /// One `ppp*Des` layout.
    #[derive(Debug, Clone, Copy)]
    pub struct DesLayout {
        /// The index in `[prog +0xc]` of the work area's offset.
        pub slot: u32,
        /// The renderer instance's destruction.
        pub destroy: Destroy,
        /// The pointers released, in order.
        pub frees: &'static [Free],
    }

    const fn fr(off: u32, check: bool, reset: bool) -> Free {
        Free { off, check, reset }
    }

    /// `pppDrawShapeFieldDes` (`0x00742bd0`).
    pub const FIELD_DES: DesLayout = DesLayout {
        slot: 0,
        destroy: Destroy::FirstUnless(&[
            0x76f, 0x18e7, 0x5c8, 0x1f7, 0x4c4, 0x1b8a, 0x324, 0x1e8a, 0x1139,
        ]),
        frees: &[
            fr(0xe0, true, false),
            fr(0xe4, true, false),
            fr(0xf0, true, false),
        ],
    };
    /// `pppDrawShapeFieldRevDes` (`0x00743f80`).
    pub const FIELD_REV_DES: DesLayout = DesLayout {
        slot: 0,
        destroy: Destroy::First,
        frees: &[
            fr(0xe0, true, true),
            fr(0xe4, false, true),
            fr(0xf0, true, true),
        ],
    };
    /// `pppDrawShapeFieldSpdDes` (`0x00745130`).
    pub const FIELD_SPD_DES: DesLayout = DesLayout {
        slot: 0,
        destroy: Destroy::First,
        frees: &[
            fr(0xe0, true, true),
            fr(0xe4, false, false),
            fr(0xf0, true, true),
            fr(0xf4, true, true),
        ],
    };
    /// `pppDrawShapeFieldGlobalDes` (`0x00746280`).
    pub const FIELD_GLOBAL_DES: DesLayout = DesLayout {
        slot: 0,
        destroy: Destroy::First,
        frees: &[fr(0xe0, true, true), fr(0xec, true, true)],
    };
    /// `pppDrawShapeCameraDes` (`0x00747490`) and
    /// `pppDrawShapeCameraDisPosDes` (`0x007486e0`).
    pub const CAMERA_DES: DesLayout = DesLayout {
        slot: 0,
        destroy: Destroy::Twice,
        frees: &[fr(0xe0, true, true), fr(0xec, true, true)],
    };
    /// `pppDrawRainDes` (`0x00756270`).
    pub const RAIN_DES: DesLayout = DesLayout {
        slot: 1,
        destroy: Destroy::Never,
        frees: &[fr(0xa4, true, true)],
    };

    /// A `ppp*Des(obj, prog)`: the renderer instance destroyed
    /// (`graphicVFXDestroy`) as `layout` says, and the work area's
    /// (`obj + [[prog +0xc] + 4 slot]`) pointers released (`pppMemFree` in
    /// the current environment), as `layout` says. Returns `eax` as the
    /// original leaves it.
    pub fn ppp_des<M: Mem>(
        mem: &mut M,
        obj: u32,
        prog: u32,
        layout: DesLayout,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let t: u32 = mem.read(prog.wrapping_add(0xc));
        let off: u32 = mem.read(t.wrapping_add(4 * layout.slot));
        let w = obj.wrapping_add(off);
        let destroy = g(mem, VFX_DESTROY);
        let mut eax = match layout.destroy {
            Destroy::Never => 0,
            Destroy::First | Destroy::Twice => {
                let id = instance(mem, obj);
                call(mem, destroy, &id)
            }
            Destroy::FirstUnless(skip) => {
                let h = call(mem, g(mem, NAME_HASH), &[]);
                if skip.contains(&h) {
                    h
                } else {
                    let id = instance(mem, obj);
                    call(mem, destroy, &id)
                }
            }
        };
        let (get, set, free) = (g(mem, GET_PTR32), g(mem, SET_PTR32), g(mem, MEM_FREE));
        for f in layout.frees {
            let raw: u32 = mem.read(w.wrapping_add(f.off));
            let p = call(mem, get, &[raw]);
            eax = p;
            if p != 0 || !f.check {
                let env_at: u32 = mem.read(g(mem, ENV_SLOT));
                let env: u32 = mem.read(env_at);
                eax = call(mem, free, &[env, env.wrapping_add(0x10), p]);
            }
            if f.reset {
                eax = call(mem, set, &[0]);
                mem.write(w.wrapping_add(f.off), eax);
            }
        }
        if layout.destroy == Destroy::Twice {
            let id = instance(mem, obj);
            eax = call(mem, destroy, &id);
        }
        eax
    }

    const SPR_ALLOC: u32 = 0x007e_6a00;
    const SPR_FREE: u32 = 0x007e_6a60;
    const FLUSH_CACHE: u32 = 0x0062_f4c0;
    const SYNC_PATH: u32 = 0x0062_f880;
    const SET_DEF_LOAD_IMAGE: u32 = 0x0062_f820;
    const EXEC_LOAD_IMAGE: u32 = 0x0062_f7a0;
    const SET_DEF_STORE_IMAGE: u32 = 0x0062_f830;
    const EXEC_STORE_IMAGE: u32 = 0x0062_f7b0;
    const SUB_VECTOR: u32 = 0x006e_d940;
    const DIV_VECTOR: u32 = 0x006e_d3a0;
    const ADD_VECTOR: u32 = 0x006e_d290;
    const FTOI4_VECTOR: u32 = 0x0062_fcd0;
    const GET_DBUFF_DC: u32 = 0x0082_0530;
    /// The CLUT's 32-entry step table (signed entry deltas, CSM1 order).
    const CLUT_STEP: u32 = 0x00c3_e6a0;
    /// The CLUT's 32-entry position table (entry `i` within its block).
    const CLUT_POS: u32 = 0x00c3_e6c0;
    /// `g_pppStop`.
    const STOP: u32 = 0x0230_fd34;
    /// The current lens-flare view (`u16`).
    const LENS_VIEW: u32 = 0x0230_fffe;

    fn f(mem: &impl Mem, at: u32) -> f64 {
        ffx_bridge::x87::load(mem.read(at))
    }

    fn sbyte(mem: &impl Mem, at: u32) -> u32 {
        i32::from(mem.read::<u8>(at).cast_signed()).cast_unsigned()
    }

    /// `pppKeTkFadeDraw(obj, key, prog)` (`0x0072ee50`): the fade's work
    /// area `[pos, speed, accel]` at `obj + 0xa0 + [[prog + 0xc]]` steps
    /// (`speed += accel`, `pos += speed`, the key's `+8..+0x10` added on
    /// its frame) unless stopped; then, unless the key's texture (`+4`) is
    /// `0xffff`, a 256-entry CLUT in a scratch buffer: `n = ftol(pos)`
    /// clamped to `[-0x100, 0xff]`; for `n >= 0` the first `n` entries hold
    /// colour `+0x18` and a ramp from it to colour `+0x14` fills the rest,
    /// for `n < 0` the ramp covers the first `n + 0x100` and `+0x14` the
    /// rest (entries in CSM1 order, through the tables at `0x00c3e6a0`).
    /// The upload (`sceGsSetDefLoadImage`, `sceGsExecLoadImage` to the
    /// texture's CLUT at `+0x10`) is the renderer seam, a call through
    /// `call` (no-op stubs in `FFX.exe`); the locals lie below `frame`.
    /// Returns what the original leaves in eax.
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn ppp_ke_tk_fade_draw<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        prog: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::{add, ftol, ftol2, load, store};
        let off: u32 = mem.read(mem.read::<u32>(prog.wrapping_add(0xc)));
        let w = obj.wrapping_add(off).wrapping_add(0xa0);
        let mut eax = mem.read::<u32>(prog.wrapping_add(0xc));
        if mem.read::<u32>(g(mem, STOP)) == 0 {
            let speed = store(add(f(mem, w.wrapping_add(8)), f(mem, w.wrapping_add(4))));
            mem.write(w.wrapping_add(4), speed);
            let pos = store(add(f(mem, w), load(speed)));
            mem.write(w, pos);
            eax = mem.read(obj.wrapping_add(0xc));
            if eax == mem.read::<u32>(key) {
                mem.write(w, store(add(load(pos), f(mem, key.wrapping_add(8)))));
                mem.write(
                    w.wrapping_add(4),
                    store(add(load(speed), f(mem, key.wrapping_add(0xc)))),
                );
                let accel = store(add(
                    f(mem, key.wrapping_add(0x10)),
                    f(mem, w.wrapping_add(8)),
                ));
                mem.write(w.wrapping_add(8), accel);
            }
        }
        let tex_id: u32 = mem.read(key.wrapping_add(4));
        if tex_id == 0xffff {
            return eax;
        }
        let buf = call(mem, g(mem, SPR_ALLOC), &[0x420]);
        for i in 0..8 {
            let v: u32 = mem.read(g(mem, CLUT_STEP).wrapping_add(4 * i));
            mem.write(buf.wrapping_add(0x400 + 4 * i), v);
        }
        let env: u32 = mem.read(mem.read::<u32>(g(mem, ENV_SLOT)));
        let rec = mem
            .read::<u32>(env.wrapping_add(0x20))
            .wrapping_add(tex_id << 5);
        let tex: u32 = mem.read(rec.wrapping_add(0x10));
        let tex = tex | (tex.cast_signed() >> 31).cast_unsigned();
        let step =
            |mem: &M, i: u32| sbyte(mem, buf.wrapping_add(0x400 + (i & 0x1f))).wrapping_mul(4);
        let n = ftol(mem, f(mem, w)).cast_signed();
        let (end_c, start_c) = (
            mem.read::<u32>(key.wrapping_add(0x14)),
            mem.read::<u32>(key.wrapping_add(0x18)),
        );
        let (mut at, from, to, div);
        if n < 0 {
            // the tail [n, 0x100) is the end colour, the ramp [0, n) before it
            let n = n.max(-0x100) + 0x100;
            let pos = sbyte(mem, g(mem, CLUT_POS).wrapping_add(n.cast_unsigned() & 0x1f))
                .wrapping_add(n.cast_unsigned() & 0xe0);
            let mut p = buf.wrapping_add(pos.wrapping_mul(4));
            for i in n..0x100 {
                mem.write(p, end_c);
                p = p.wrapping_add(step(mem, i.cast_unsigned()));
            }
            #[allow(clippy::cast_precision_loss)] // fild of a byte count
            let d = n as f32;
            (at, from, to, div) = (buf, 0, n, d);
        } else {
            // the head [0, n) is the start colour, the ramp [n, 0x100) after it
            let n = n.min(0xff);
            let mut p = buf;
            for i in 0..n {
                mem.write(p, start_c);
                p = p.wrapping_add(step(mem, i.cast_unsigned()));
            }
            #[allow(clippy::cast_precision_loss)] // 256 - n, exact
            let d = (0x100 - n) as f32;
            (at, from, to, div) = (p, n, 0x100, d);
        }
        // the ramp: start + i (end - start) / div, per channel, on the stack
        let (s, e) = (frame.wrapping_sub(0x14), frame.wrapping_sub(0x24));
        for (k, b) in (0u32..).zip(start_c.to_le_bytes()) {
            mem.write(s.wrapping_add(4 * k), f32::from(b).to_bits());
        }
        for (k, b) in (0u32..).zip(end_c.to_le_bytes()) {
            mem.write(e.wrapping_add(4 * k), f32::from(b).to_bits());
        }
        call(mem, g(mem, SUB_VECTOR), &[e, e, s]);
        call(mem, g(mem, DIV_VECTOR), &[e, e, div.to_bits()]);
        for i in from..to {
            let mut c = 0u32;
            for k in (0..4).rev() {
                c = (c << 8) | ftol2(f(mem, s.wrapping_add(4 * k)));
            }
            mem.write(at, c);
            call(mem, g(mem, ADD_VECTOR), &[s, s, e]);
            at = at.wrapping_add(step(mem, i.cast_unsigned()));
        }
        // the renderer seam: the CLUT upload
        let lp = frame.wrapping_sub(0x80);
        call(
            mem,
            g(mem, SET_DEF_LOAD_IMAGE),
            &[
                lp,
                (tex >> 7) & 0x3fff,
                4,
                (tex >> 21) & 0xf,
                0,
                0,
                0x10,
                0x10,
            ],
        );
        call(mem, g(mem, FLUSH_CACHE), &[0]);
        call(mem, g(mem, EXEC_LOAD_IMAGE), &[lp, buf]);
        call(mem, g(mem, SYNC_PATH), &[0, 0]);
        call(mem, g(mem, SPR_FREE), &[0x420])
    }

    /// `pppKeLnsLpTOtDraw(obj, _, frame_no)` (`0x0075b230`): the lens
    /// flare's occlusion test. Off its view (`u16` `+0xcc` against
    /// `0x0230fffe`) the visibility `+0x7c` is 0; at 0 it stays; off its
    /// frame (`+0x8c` and `frame_no` modulo `+0x8d + 1` differ) it decays
    /// by `+0xac`. Otherwise the `+0xd0 x +0xd1` half-extent box (in 1/8
    /// pixels) round the projected point (`FFXVu0FTOI4Vector(+0xc0)`),
    /// clamped to the draw buffer's scissor, is read back from the depth
    /// buffer into a scratch buffer (`sceGsSetDefStoreImage`,
    /// `sceGsExecStoreImage`: the renderer seam, calls through `call`;
    /// no-op stubs in `FFX.exe`), the texels whose 24-bit z lies below the
    /// point's are counted, and with `v = count / (+0xd0 * +0xd1)` the
    /// visibility becomes `v * +0x7c` and `+0xac` becomes `v` (both 0 for
    /// an empty box). The locals lie below `frame`.
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn ppp_ke_lns_lp_t_ot_draw<M: Mem>(
        mem: &mut M,
        obj: u32,
        frame_no: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_bridge::x87::{div, load, mul, store};
        let at = |o: u32| obj.wrapping_add(o);
        if mem.read::<u16>(at(0xcc)) != mem.read::<u16>(g(mem, LENS_VIEW)) {
            mem.write(at(0x7c), 0u32);
            return;
        }
        // fucompp: equal to 0.0 (not unordered)
        if f(mem, at(0x7c)) == 0.0 {
            return;
        }
        let m = i32::from(mem.read::<u8>(at(0x8d))) + 1;
        let a = i32::from(mem.read::<u8>(at(0x8c)));
        if a % m != frame_no.cast_signed().wrapping_rem(m) {
            let v = store(mul(f(mem, at(0xac)), f(mem, at(0x7c))));
            mem.write(at(0x7c), v);
            return;
        }
        let slot = frame.wrapping_sub(0x98);
        mem.write(slot, m.cast_unsigned());
        let dc = call(mem, g(mem, GET_DBUFF_DC), &[slot]);
        let gs = if mem.read::<u8>(slot) & 1 != 0 {
            dc.wrapping_add(0x190)
        } else {
            dc.wrapping_add(0x60)
        };
        let ox = u32::from(mem.read::<u16>(gs.wrapping_add(0x18)));
        let oy = u32::from(mem.read::<u16>(gs.wrapping_add(0x1c)));
        let sx: u32 = mem.read(gs.wrapping_add(0x24));
        let sy: u32 = mem.read(gs.wrapping_add(0x28));
        let x0 = ((sx & 0x7ff) << 4).wrapping_add(ox).cast_signed();
        let x1 = ((((sx >> 16) & 0x7ff) + 1) << 4)
            .wrapping_add(ox)
            .cast_signed();
        let y0 = ((sy & 0x7ff) << 4).wrapping_add(oy).cast_signed();
        let y1 = ((((sy >> 16) & 0x7ff) + 1) << 4)
            .wrapping_add(oy)
            .cast_signed();
        let v = frame.wrapping_sub(0x14);
        call(mem, g(mem, FTOI4_VECTOR), &[v, at(0xc0)]);
        let px = mem.read::<u32>(v).cast_signed();
        let py = mem.read::<u32>(v.wrapping_add(4)).cast_signed();
        let hx = i32::from(mem.read::<u8>(at(0xd0)));
        let hy = i32::from(mem.read::<u8>(at(0xd1)));
        // clamped low end first, as the original
        let clamp = |x: i32, lo: i32, hi: i32| {
            if x < lo {
                lo
            } else if x > hi {
                hi
            } else {
                x
            }
        };
        let left = clamp(px.wrapping_sub(hx * 8), x0, x1);
        let top = clamp(py.wrapping_sub(hy * 8), y0, y1);
        let right = clamp(px.wrapping_add(hx * 8), x0, x1);
        let bottom = clamp(py.wrapping_add(hy * 8), y0, y1);
        let w = right.wrapping_sub(left).wrapping_add(0xf) >> 4;
        let h = bottom.wrapping_sub(top).wrapping_add(0xf) >> 4;
        if w <= 0 || h <= 0 {
            mem.write(at(0x7c), 0u32);
            mem.write(at(0xac), 0u32);
            return;
        }
        let wa = (w.wrapping_add(7)) & !7;
        let buf = call(mem, g(mem, SPR_ALLOC), &[0x3060]);
        let ha = (h + 3) & !3;
        let tx = left.wrapping_sub(ox.cast_signed()) >> 4;
        let ty = top.wrapping_sub(oy.cast_signed()) >> 4;
        let psm = u32::from(mem.read::<u16>(gs.wrapping_add(2)) & 0x3f);
        let fbp = u32::from((mem.read::<u16>(gs.wrapping_add(0xc)) & 0x1ff) << 5);
        // the renderer seam: the depth readback
        let lp = frame.wrapping_sub(0x80);
        call(
            mem,
            g(mem, SET_DEF_STORE_IMAGE),
            &[
                lp,
                fbp,
                psm,
                0x31,
                tx.cast_unsigned(),
                ty.cast_unsigned(),
                wa.cast_unsigned(),
                ha.cast_unsigned(),
            ],
        );
        call(mem, g(mem, FLUSH_CACHE), &[0]);
        let mut p = buf.wrapping_add(0x60);
        call(mem, g(mem, EXEC_STORE_IMAGE), &[lp, p]);
        call(mem, g(mem, SYNC_PATH), &[0, 0]);
        let z = (mem.read::<u32>(v.wrapping_add(8)).cast_signed() >> 4).cast_unsigned();
        let pad = (wa - w).wrapping_mul(3).cast_unsigned();
        let mut count = 0.0f32;
        for _ in 0..h {
            for _ in 0..w {
                let t = u32::from_le_bytes([
                    mem.read(p),
                    mem.read(p.wrapping_add(1)),
                    mem.read(p.wrapping_add(2)),
                    0,
                ]);
                if t < z {
                    count += 1.0;
                }
                p = p.wrapping_add(3);
            }
            p = p.wrapping_add(pad);
        }
        call(mem, g(mem, SPR_FREE), &[0x3060]);
        #[allow(clippy::cast_precision_loss)] // fild, then fstp m32
        let area = hy.wrapping_mul(hx) as f32;
        let vis = store(div(f64::from(count), load(area.to_bits())));
        mem.write(at(0x7c), store(mul(load(vis), f(mem, at(0x7c)))));
        mem.write(at(0xac), vis);
    }

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
    /// `g_pppColorConShift`: added once to a colour read, then cleared.
    const COLOR_SHIFT: u32 = 0x00c8_b500;
    /// `g_pppCurrentPart`.
    const CURRENT_PART: u32 = 0x0230_ffe4;
    /// `g_PppPacketPos`.
    const PACKET_POS: u32 = 0x0230_fd2c;
    /// The emulated EE's `r0` (`g_Ee_Gpr`) and the two work registers.
    const EE_R0: u32 = 0x00c8_f508;
    const EE_A: u32 = 0x00c8_f588;
    const EE_B: u32 = 0x00c8_f598;

    /// `pppDrawMdlDraw(obj, key, prog)` (`0x00737770`): unless the key's
    /// model (`+4`) is `0xffff` or the level is `0xec1`, the object's
    /// colour (the work area's `+0xa0` halves, read through
    /// `g_pppColorConShift`) times the part's `+0x70` halves, `>> 19 &
    /// 0x1fff` per channel, packed to bytes (`pextlh`, `pmulth`, `ppacb` on
    /// the emulated EE); with a non-zero alpha, a render request on the
    /// stack (`frame - 0x80`: flags from the model record's word 0, its
    /// `+0x16` blend, the colour, the part's `+0x78`, the object's matrix
    /// `+0x10` and `+0x50` and textures `+0x90`/`+0x94`) updates the
    /// instance's render data, or creates it (path 3 for flag 4). The
    /// request's `+0xb` byte and `+0x30` word are left as they were.
    /// Returns what the original leaves in eax.
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn ppp_draw_mdl_draw<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        prog: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let model: u32 = mem.read(key.wrapping_add(4));
        if model == 0xffff {
            return key;
        }
        let h = call(mem, g(mem, NAME_HASH), &[]);
        if h == 0xec1 {
            return h;
        }
        let (r0, a, b, shift) = (
            g(mem, EE_R0),
            g(mem, EE_A),
            g(mem, EE_B),
            g(mem, COLOR_SHIFT),
        );
        let area = mem
            .read::<u32>(mem.read::<u32>(prog.wrapping_add(0xc)))
            .wrapping_add(obj)
            .wrapping_add(mem.read::<u32>(shift));
        let lo: u32 = mem.read(area.wrapping_add(0xa0));
        mem.write(a, lo);
        let hi: u32 = mem.read(area.wrapping_add(0xa4));
        let part: u32 = mem.read(g(mem, CURRENT_PART));
        mem.write(shift, 0u32);
        mem.write(a.wrapping_add(4), hi);
        let v: u32 = mem.read(part.wrapping_add(0x70));
        mem.write(b, v);
        let v: u32 = mem.read(part.wrapping_add(0x74));
        mem.write(b.wrapping_add(4), v);
        mem.write(shift, 0u32);
        call(mem, g(mem, PEXTLH), &[a, r0, a]);
        call(mem, g(mem, PEXTLH), &[b, r0, b]);
        call(mem, g(mem, PMULTH), &[a, a, b]);
        // the products >> 19 & 0x1fff as halves, r0's low halves above
        let word =
            |mem: &M, at: u32| (mem.read::<u32>(at).cast_signed() >> 19).cast_unsigned() & 0x1fff;
        let ch = [
            word(mem, a),
            word(mem, a + 4),
            word(mem, a + 8),
            word(mem, a + 0xc),
        ];
        let r = [0u32, 4, 8, 0xc].map(|o| mem.read::<u32>(r0 + o) & 0xffff);
        mem.write(a, ch[0] | ch[1] << 16);
        mem.write(a + 4, ch[2] | ch[3] << 16);
        mem.write(a + 8, r[0] | r[1] << 16);
        mem.write(a + 0xc, r[2] | r[3] << 16);
        call(mem, g(mem, PPACB), &[a, r0, a]);
        let colour: u32 = mem.read(a);
        let sh: u32 = mem.read(shift);
        mem.write(frame.wrapping_sub(0x68).wrapping_add(sh), colour);
        mem.write(shift, 0u32);
        if mem.read::<u8>(frame.wrapping_sub(0x65)) == 0 {
            return colour;
        }
        // the request
        let req = frame.wrapping_sub(0x80);
        let env: u32 = mem.read(mem.read::<u32>(g(mem, ENV_SLOT)));
        let rec = mem
            .read::<u32>(env.wrapping_add(0x20))
            .wrapping_add(model << 5);
        let mp = call(mem, g(mem, GET_PTR32), &[mem.read(rec.wrapping_add(0x1c))]);
        let m0: u8 = mem.read(mp);
        let mut flags = if m0 & 1 != 0 { 0x80u32 } else { 0xc };
        if m0 & 2 == 0 {
            flags |= 0x10;
        }
        mem.write(req, flags);
        let blend: u16 = mem.read(mp.wrapping_add(0x16));
        mem.write(req.wrapping_add(8), blend);
        mem.write(req.wrapping_add(0xa), 0u8);
        let part: u32 = mem.read(g(mem, CURRENT_PART));
        let v: u32 = mem.read(part.wrapping_add(0x78));
        mem.write(req.wrapping_add(0x1c), v);
        let t0 = call(mem, g(mem, GET_PTR32), &[mem.read(obj.wrapping_add(0x90))]);
        mem.write(req.wrapping_add(0x38), t0);
        let t1 = call(mem, g(mem, GET_PTR32), &[mem.read(obj.wrapping_add(0x94))]);
        mem.write(req.wrapping_add(0x3c), t1);
        let v: u32 = mem.read(g(mem, 0x00c4_98dc));
        mem.write(req.wrapping_add(4), v);
        mem.write(req.wrapping_add(0x20), obj.wrapping_add(0x50));
        mem.write(req.wrapping_add(0x2c), obj.wrapping_add(0x10));
        for o in [0xcu32, 0x10, 0x14, 0x40, 0x44] {
            mem.write(req.wrapping_add(o), 0u32);
        }
        mem.write(req.wrapping_add(0x24), g(mem, 0x0230_fd60));
        mem.write(req.wrapping_add(0x28), g(mem, 0x0230_fda0));
        let v: u32 = mem.read(g(mem, PACKET_POS));
        mem.write(req.wrapping_add(0x34), v);
        let id = instance(mem, obj);
        if call(mem, g(mem, GET_RENDER_DATA), &id) != 0 {
            let id = instance(mem, obj);
            call(mem, g(mem, UPDATE_RENDER_DATA), &[id[0], id[1], rec, req]);
            let id = instance(mem, obj);
            let f = if mem.read::<u8>(req) & 4 != 0 {
                PATH3_UPDATE
            } else {
                VFX_UPDATE
            };
            return call(mem, g(mem, f), &id);
        }
        let blend = u32::from(mem.read::<u8>(req.wrapping_add(8)));
        let mut alpha = call(mem, g(mem, ALPHA_FUNCTION), &[blend]);
        if mem.read::<u32>(req) & 0x2000 == 0 {
            alpha |= call(mem, g(mem, Z_MAT), &[mem.read(req.wrapping_add(0x44))]);
        }
        let fl: u8 = mem.read(req);
        if fl & 8 != 0 {
            alpha |= 0x40;
        }
        let id = instance(mem, obj);
        let f = if fl & 4 != 0 {
            INIT_PATH3_RENDER_DATA
        } else {
            INIT_RENDER_DATA
        };
        call(mem, g(mem, f), &[id[0], id[1], rec, req, alpha])
    }
}
