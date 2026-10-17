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
}
