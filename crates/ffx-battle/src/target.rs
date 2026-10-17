//! Where actors face and move to (`docs/rewrite-plan.md`, M2):
//! `MsSetTargetDir` and the chain under it, `MsGetChrInfo` (a property of
//! a set of actors, averaged), the motion targets (`MsGetMoveTargetPos`)
//! and the accelerated moves.
//!
//! The chain is recursive (`MsGetChrMovePos` asks `MsGetMoveTargetPos`,
//! which asks `MsGetChrInfo`, which may ask `MsGetChrMovePos` again), and
//! the originals pass pointers to their stack locals down it. Here the
//! locals live in [`Dm`] frames, a stack of [`Frame`]s of their own
//! addresses below `0xfe000000` (an address range the game never maps).
//!
//! Several originals pass fewer arguments than their callees read
//! (`MsMulVector`, `MsDivVector`, `MsVectorNormal2`, `MsVectorPolar2Rect`
//! by value): the rest is the caller's saved registers on the stack, and
//! goes only to a VU register. Within the chain those are known
//! ([`Regs`]); for a call from outside it the register words are left as
//! they were.

/// The game's memory, through [`ffx_bridge::Mem`].
#[allow(clippy::many_single_char_names)] // vectors and their parts
pub mod bridge {
    use ffx_bridge::frame::Frame;
    use ffx_bridge::{Global, Mem};

    use crate::actor::bridge::{ms_btl_chr_num_check, ms_get_chr};
    use crate::pos::bridge::Trig;
    use crate::vector::bridge as v;

    trait Raw {
        fn raw_read(&self, addr: u32, out: &mut [u8]);
        fn raw_write(&mut self, addr: u32, data: &[u8]);
        fn raw_base(&self) -> u32;
    }

    impl<M: Mem> Raw for M {
        fn raw_read(&self, addr: u32, out: &mut [u8]) {
            self.read_bytes(addr, out);
        }
        fn raw_write(&mut self, addr: u32, data: &[u8]) {
            self.write_bytes(addr, data);
        }
        fn raw_base(&self) -> u32 {
            self.image_base()
        }
    }

    /// The game's memory with a stack of frames for the originals' locals.
    pub struct Dm<'a> {
        mem: &'a mut dyn Raw,
        sp: u32,
        /// [`Dm::locals`]' blocks (base, bytes), innermost last.
        locals: Vec<(u32, Vec<u8>)>,
    }

    /// Where [`Dm`]'s frames start (growing down).
    pub const FRAMES_TOP: u32 = 0xfe00_0000;

    impl<'a> Dm<'a> {
        /// The memory, frames from [`FRAMES_TOP`].
        pub fn new<M: Mem + 'a>(mem: &'a mut M) -> Self {
            Self {
                mem,
                sp: FRAMES_TOP,
                locals: Vec::new(),
            }
        }

        /// Runs `f` with `len` bytes of locals (zeroed; their address the
        /// second argument) on this same memory: unlike [`Dm::frame`],
        /// `f` keeps the lifetime, so callbacks made for it (a script's
        /// calls) still take it.
        pub fn locals<R>(&mut self, len: u32, f: impl FnOnce(&mut Self, u32) -> R) -> R {
            let old = self.sp;
            let base = old.wrapping_sub(len) & !0xf;
            self.sp = base;
            self.locals.push((base, vec![0; len as usize]));
            let r = f(self, base);
            self.locals.pop();
            self.sp = old;
            r
        }

        fn local(&mut self, addr: u32, len: usize) -> Option<&mut [u8]> {
            self.locals.iter_mut().rev().find_map(|(base, bytes)| {
                let start = addr.checked_sub(*base)? as usize;
                bytes.get_mut(start..start.checked_add(len)?)
            })
        }

        /// Runs `f` with `len` bytes of locals (zeroed; their address the
        /// second argument).
        pub fn frame<R>(&mut self, len: u32, f: impl FnOnce(&mut Dm<'_>, u32) -> R) -> R {
            let base = self.sp.wrapping_sub(len) & !0xf;
            let mut fr = Frame::new(self, base, len as usize);
            let mut d = Dm {
                mem: &mut fr,
                sp: base,
                locals: Vec::new(),
            };
            f(&mut d, base)
        }
    }

    impl Mem for Dm<'_> {
        fn read_bytes(&self, addr: u32, out: &mut [u8]) {
            let local = self.locals.iter().rev().find_map(|(base, bytes)| {
                let start = addr.checked_sub(*base)? as usize;
                bytes.get(start..start.checked_add(out.len())?)
            });
            match local {
                Some(b) => out.copy_from_slice(b),
                None => self.mem.raw_read(addr, out),
            }
        }
        fn write_bytes(&mut self, addr: u32, data: &[u8]) {
            match self.local(addr, data.len()) {
                Some(b) => b.copy_from_slice(data),
                None => self.mem.raw_write(addr, data),
            }
        }
        fn image_base(&self) -> u32 {
            self.mem.raw_base()
        }
    }

    /// What the chain calls in other modules (the C runtime's
    /// trigonometry, script, menu), from the caller.
    pub struct Host<'h> {
        /// `_CIcos`, `_CIsin`, `_CIatan`.
        pub trig: &'h dyn Fn(Trig, u32) -> u32,
        /// `TkMtCalcSpline(out, points, n, t)` (menu, `0x0088a4a0`).
        pub spline: &'h dyn Fn(&mut Dm<'_>, u32, u32, u32, u32),
        /// `AtelGetTotalActorNumber2()` (script).
        pub atel_total: &'h dyn Fn(&mut Dm<'_>) -> u32,
        /// `AtelGetNowCtrlTopId()` (script).
        pub atel_top: &'h dyn Fn(&mut Dm<'_>) -> u32,
        /// `AtelIsBind(i)` (script, `0x0086d950`).
        pub atel_is_bind: &'h dyn Fn(&mut Dm<'_>, u32) -> u32,
        /// The script actor's direction (script, `0x0086a6e0`, an `f32` on
        /// the x87 stack: its bits).
        pub atel_dir: &'h dyn Fn(&mut Dm<'_>, u32) -> u32,
        /// The script actor's position into three floats (script,
        /// `0x0086b1e0(i, &x, &y, &z)`).
        pub atel_pos: &'h dyn Fn(&mut Dm<'_>, u32, u32, u32, u32),
    }

    /// A caller's saved registers, as a callee taking more arguments than
    /// pushed reads them from the stack.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Regs {
        pub edi: u32,
        pub esi: u32,
        pub ebx: u32,
    }

    fn f32_at(mem: &impl Mem, at: u32) -> u32 {
        mem.read(at)
    }

    fn quiet(bits: u32) -> u32 {
        ffx_bridge::x87::store(ffx_bridge::x87::load(bits))
    }

    fn byte(mem: &impl Mem, a: u32, o: u32) -> u8 {
        mem.read(a.wrapping_add(o))
    }

    /// The VU register words a short argument list leaves as they were.
    fn vf_words(mem: &impl Mem, at: u32) -> [u32; 4] {
        mem.read(at)
    }

    /// 0.5 (`0x00b92220`).
    pub const HALF: Global<f64> = Global::new(0x00b9_2220);
    /// `0x0112c9dd`: the camera keeps a KO'd actor's `+0xf70` height.
    pub const CAMERA_KO_HEIGHT: Global<u8> = Global::new(0x0112_c9dd);

    fn lift_half(mem: &mut impl Mem, a: u32, i: u32, vec: u32) {
        use ffx_bridge::x87::{add, load, mul, store};
        let under = crate::state::bridge::UNDERWATER.get(mem) != 0;
        if byte(mem, a, 0x505) != 0 || under {
            let h = crate::body::bridge::ms_get_chr_height(mem, i).to_bits();
            let y = vec.wrapping_add(4);
            let r = store(add(mul(load(h), HALF.get(mem)), load(f32_at(mem, y))));
            mem.write(y, r);
        }
    }

    /// `MsGetChrBottomPos(i, vec)` (`0x00794830`): actor `i`'s position
    /// (`msGetChrPos`), half its height up when it floats (`+0x505`) or the
    /// battle is under water; -1, 0 for a non-actor.
    pub fn ms_get_chr_bottom_pos<M: Mem>(
        mem: &mut M,
        trig: &dyn Fn(Trig, u32) -> u32,
        i: u32,
        vec: u32,
    ) -> i32 {
        let i = i & 0xff;
        if ms_btl_chr_num_check(i) == 0 {
            return 0;
        }
        let a = ms_get_chr(mem, i);
        crate::pos::bridge::ms_get_chr_pos_of(mem, trig, a, vec);
        lift_half(mem, a, i, vec);
        -1
    }

    /// `MsGetChrCameraPos(i, vec)` (`0x00794ab0`): where the camera looks at
    /// actor `i`: its model's matrix origin (`Ch_GetOrgMatrix` `+0x30`),
    /// else its position (the KO height `+0xf70` with [`CAMERA_KO_HEIGHT`]
    /// and `+0xdcc` 2), half its height up as [`ms_get_chr_bottom_pos`];
    /// w 1. -1, 0 for a non-actor.
    pub fn ms_get_chr_camera_pos<M: Mem>(
        mem: &mut M,
        trig: &dyn Fn(Trig, u32) -> u32,
        i: u32,
        vec: u32,
    ) -> i32 {
        let i = i & 0xff;
        if ms_btl_chr_num_check(i) == 0 {
            return 0;
        }
        let a = ms_get_chr(mem, i);
        let chr: u32 = mem.read(a);
        if chr != 0 {
            let m = ffx_chara::chr::bridge::ch_get_org_matrix(mem, chr);
            if m != 0 {
                let p: [u32; 3] = mem.read(m.wrapping_add(0x30));
                mem.write(
                    vec,
                    [quiet(p[0]), quiet(p[1]), quiet(p[2]), 1.0f32.to_bits()],
                );
                return -1;
            }
        }
        crate::pos::bridge::ms_get_chr_pos_of(mem, trig, a, vec);
        if CAMERA_KO_HEIGHT.get(mem) != 0 && byte(mem, a, 0xdcc) == 2 {
            let h = quiet(f32_at(mem, a.wrapping_add(0xf70)));
            mem.write(vec.wrapping_add(4), h);
        }
        lift_half(mem, a, i, vec);
        mem.write(vec.wrapping_add(12), 1.0f32.to_bits());
        -1
    }

    /// `MsGetChrTargetDir(i, dir)` (`0x007958c0`): `MsNearRag(dir, d)` with
    /// `d` actor `i`'s direction (`+0x53c` when `+0x510`, else `+0x49c`;
    /// 0 for a non-actor), an `f32` (on the x87 stack).
    pub fn ms_get_chr_target_dir(mem: &impl Mem, i: u32, dir: u32) -> f32 {
        let i = i & 0xff;
        let d = if ms_btl_chr_num_check(i) == 0 {
            0
        } else {
            let a = ms_get_chr(mem, i);
            let o = if byte(mem, a, 0x510) != 0 {
                0x53c
            } else {
                0x49c
            };
            quiet(f32_at(mem, a.wrapping_add(o)))
        };
        v::ms_near_rag(mem, quiet(dir), d)
    }

    /// `0x01136424`: the frames into the current move (`MsGetChrMovePos`).
    pub const MOVE_FRAME: Global<i32> = Global::new(0x0113_6424);
    /// `0x01136428`: the move splines (0x210 bytes each: a `u32` count,
    /// then the points from `+0x10`).
    pub const MOVE_SPLINES: Global<u8> = Global::new(0x0113_6428);
    /// `0x0112c9e8`: the summoner's index (`i8`).
    pub const SUMMONER: Global<i8> = Global::new(0x0112_c9e8);

    /// `MsTurnMoveVector(i, actor, vec, angle)` (`0x007ab190`): turns the
    /// move vector `vec` (x and z) by `angle`, negated when the direction
    /// from it to the actor's battle position (`+0x6d4` ignored) is, taken
    /// near the vector's own (`MsNearRag`), below it.
    pub fn ms_turn_move_vector(
        mem: &mut Dm<'_>,
        host: &Host<'_>,
        i: u32,
        actor: u32,
        vec: u32,
        angle: u32,
    ) {
        use ffx_bridge::x87::{add, load, mul, store, sub};
        mem.frame(0x30, |mem, f| {
            let (cur, home, a1, a2) = (f, f + 0x10, f + 0x20, f + 0x24);
            crate::pos::bridge::ms_get_chr_pos(mem, host.trig, i, cur);
            v::ms_vector_rect2_polar(mem, host.trig, vec, 0, a1, 0, 0);
            let keep = byte(mem, actor, 0x6d4);
            mem.write(actor.wrapping_add(0x6d4), 0xffu8);
            crate::pos::bridge::ms_get_chr_btl_pos(mem, host.trig, i, home);
            mem.write(actor.wrapping_add(0x6d4), keep);
            v::ms_vector_rect2_polar(mem, host.trig, home, vec, a2, 0, 0);
            let y1 = f32_at(mem, a1);
            let near = v::ms_near_rag(mem, y1, f32_at(mem, a2)).to_bits();
            let mut angle = angle;
            if sub(load(near), load(y1)) < 0.0 {
                angle = store(-load(angle));
            }
            let x = quiet(f32_at(mem, vec));
            let z = quiet(f32_at(mem, vec.wrapping_add(8)));
            let s = (host.trig)(Trig::Sin, angle);
            let c = (host.trig)(Trig::Cos, angle);
            let (x, z, s, c) = (load(x), load(z), load(s), load(c));
            mem.write(vec, store(sub(mul(c, x), mul(s, z))));
            mem.write(vec.wrapping_add(8), store(add(mul(s, x), mul(z, c))));
        });
    }

    /// `0x00794060(i, kind, k, dir, out)`: one actor's property `kind`
    /// into `out` (added): 1 position, 2 direction, 3 height, 4 width, 5
    /// battle position, 6 direction pair (`+0x514`/`+0x524` at `+0x511`),
    /// 7 [`ms_get_chr_target_dir`] of `dir`, 8 bottom, 9 camera position,
    /// 10 camera height, 11 camera width, 12 [`ms_get_chr_move_pos`], 13
    /// optional position `k` relative to the position. 1, 0 for another
    /// kind (adding `r0`).
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    pub fn ms_get_chr_info_one(
        mem: &mut Dm<'_>,
        host: &Host<'_>,
        i: u32,
        kind: u32,
        k: u32,
        dir: u32,
        out: u32,
    ) -> u32 {
        use crate::body::bridge as b;
        use crate::pos::bridge as p;
        let i = i & 0xff;
        mem.frame(0x20, |mem, f| {
            let (loc, tmp) = (f, f + 0x10);
            let a = ms_get_chr(mem, i);
            let r0 = crate::camera::bridge::EE_R0.get(mem);
            mem.write(loc, r0);
            let chr: u32 = mem.read(a);
            let mut ok = 1;
            match kind {
                1 => {
                    p::ms_get_chr_pos(mem, host.trig, i, loc);
                }
                2 => {
                    b::ms_get_chr_dir(mem, i, loc);
                }
                3 => mem.write(loc, b::ms_get_chr_height(mem, i).to_bits()),
                4 => mem.write(
                    loc,
                    crate::actor::bridge::ms_get_chr_width(mem, i).to_bits(),
                ),
                5 => {
                    p::ms_get_chr_btl_pos(mem, host.trig, i, loc);
                }
                6 => {
                    let n = i32::from(byte(mem, a, 0x511).cast_signed()).cast_unsigned();
                    let at = |o: u32| a.wrapping_add(o).wrapping_add(n.wrapping_mul(4));
                    let x = quiet(f32_at(mem, at(0x514)));
                    let y = quiet(f32_at(mem, at(0x524)));
                    mem.write(loc, [x, y]);
                }
                7 => mem.write(loc, ms_get_chr_target_dir(mem, i, dir).to_bits()),
                8 => {
                    ms_get_chr_bottom_pos(mem, host.trig, i, loc);
                }
                9 => {
                    ms_get_chr_camera_pos(mem, host.trig, i, loc);
                }
                10 => mem.write(loc, b::ms_get_chr_cam_height(mem, i).to_bits()),
                11 => mem.write(loc, b::ms_get_chr_cam_width(mem, i).to_bits()),
                12 => {
                    let regs = Regs {
                        edi: 1,
                        esi: i,
                        ebx: out,
                    };
                    ms_get_chr_move_pos(mem, host, i, loc, Some(regs));
                }
                13 => {
                    if chr != 0 {
                        p::ms_get_chr_pos(mem, host.trig, i, tmp);
                        ffx_chara::attach::bridge::ch_optpos_get(mem, chr, k, loc);
                        v::ms_sub_point(mem, loc, loc, tmp);
                    }
                }
                _ => ok = 0,
            }
            if out != 0 {
                v::ms_add_point(mem, out, out, loc);
            }
            ok
        })
    }

    /// `MsGetChrInfo(kind, bits, k, out)` (`0x00795170`): property `kind`
    /// (low byte; [`ms_get_chr_info_one`]) averaged over the actors of
    /// `bits` (`out` from `r0`; the direction kind 7 near the first one's),
    /// or with bit 0x100 the script actors' ([`ms_atel_chr_info`] of
    /// selector `k`). Returns how many answered (0 for a null `out`).
    /// `caller` are the caller's `edi` and `ebx`, which the averaging
    /// `MsDivVector` leaves in `VF_B` (as they were without).
    pub fn ms_get_chr_info(
        mem: &mut Dm<'_>,
        host: &Host<'_>,
        kind: u32,
        bits: u32,
        k: u32,
        out: u32,
        caller: Option<[u32; 2]>,
    ) -> u32 {
        use ffx_bridge::x87::{load, store};
        if out == 0 {
            return 0;
        }
        let low = kind & 0xff;
        if kind & 0x100 != 0 {
            return ms_atel_chr_info(mem, host, k, low, out);
        }
        let r0 = crate::camera::bridge::EE_R0.get(mem);
        mem.write(out, r0);
        let mut dir = 0.0f32.to_bits();
        let mut first = true;
        let mut n = 0u32;
        for i in 0..0x1f {
            if bits.cast_signed() >> i & 1 == 0 {
                continue;
            }
            n = n.wrapping_add(ms_get_chr_info_one(mem, host, i, low, k, dir, out));
            if first {
                dir = quiet(f32_at(mem, out));
                first = false;
            }
        }
        if n.cast_signed() > 0 {
            #[allow(clippy::cast_precision_loss)] // fild, then fstp dword
            let s = store(f64::from(n.cast_signed() as f32));
            let s = store(load(s));
            let mut b = vf_words(mem, v::VF_B.ptr(mem).addr);
            b[0] = s;
            if let Some([edi, ebx]) = caller {
                b[1] = edi;
                b[2] = ebx;
                b[3] = n;
            }
            v::ms_div_vector(mem, out, b);
        }
        n
    }

    /// `MsAtelChrInfo(selector, kind, out)` (`0x007975c0`): property
    /// `kind` of the script actors (`0x007984c0`: 1, 8, 9 the position, 2
    /// the direction, 7 its reverse, 3 and 10 18.0, 4 and 11 10.0) summed
    /// over the bound ones (selector -21 or -5) or of one (-2 the current
    /// one), averaged into `out` (w 0); 1, 0 for none or a null `out`.
    /// (Kinds without a y and z, and the others, add the original's stack
    /// garbage; here zeros.)
    pub fn ms_atel_chr_info(
        mem: &mut Dm<'_>,
        host: &Host<'_>,
        selector: u32,
        kind: u32,
        out: u32,
    ) -> u32 {
        use ffx_bridge::x87::{div, load, store};
        let total = (host.atel_total)(mem);
        mem.frame(0x10, |mem, acc| {
            let n: i32 = if matches!(selector.cast_signed(), -21 | -5) {
                let mut n = 0;
                for i in 0..total.cast_signed().max(0).cast_unsigned() {
                    if (host.atel_is_bind)(mem, i) != 0 {
                        ms_atel_chr_info_one(mem, host, i, kind, acc);
                        n += 1;
                    }
                }
                n
            } else {
                let s = if selector.cast_signed() == -2 {
                    (host.atel_top)(mem)
                } else {
                    selector
                };
                if s.cast_signed() < 0 || s.cast_signed() >= total.cast_signed() {
                    return 0;
                }
                ms_atel_chr_info_one(mem, host, s, kind, acc);
                1
            };
            if n < 1 || out == 0 {
                return 0;
            }
            #[allow(clippy::cast_precision_loss)] // fild, then fstp dword
            let c = load(store(f64::from(n as f32)));
            let a: [u32; 3] = mem.read(acc);
            mem.write(out, a.map(|x| store(div(load(x), c))));
            mem.write(out.wrapping_add(12), 0.0f32.to_bits());
            1
        })
    }

    /// `MsAtelChrInfoOne(i, kind, acc)` (`0x007984c0`): adds script actor `i`'s property to
    /// `acc` (x, y, z).
    pub fn ms_atel_chr_info_one(mem: &mut Dm<'_>, host: &Host<'_>, i: u32, kind: u32, acc: u32) {
        use ffx_bridge::x87::{add, load, store};
        let i = i & 0xff;
        mem.frame(0x10, |mem, f| {
            let (x, y, z) = (f, f + 4, f + 8);
            match kind {
                1 | 8 | 9 => (host.atel_pos)(mem, i, x, y, z),
                2 => {
                    let d = (host.atel_dir)(mem, i);
                    mem.write(x, d);
                }
                7 => {
                    let d = (host.atel_dir)(mem, i);
                    let pi = crate::vector::bridge::PI.get(mem);
                    mem.write(x, store(add(load(d), pi)));
                }
                3 | 10 => {
                    let c = Global::<u32>::new(0x00b5_4c84).get(mem);
                    mem.write(x, quiet(c));
                }
                4 | 0xb => {
                    let c = Global::<u32>::new(0x00b9_2318).get(mem);
                    mem.write(x, quiet(c));
                }
                _ => {}
            }
            if acc != 0 {
                for (k, l) in [(0, x), (4, y), (8, z)] {
                    let at = acc.wrapping_add(k);
                    let r = store(add(load(f32_at(mem, at)), load(f32_at(mem, l))));
                    mem.write(at, r);
                }
            }
        });
    }

    /// `MsGetChrTargetInfo(i, kind, &pos, &width, &height)` (`0x007aa050`):
    /// property `kind` averaged over actor `i`'s targets (`+0xdc0`, as
    /// `MsEffectCheckTarget` keeps them) into `pos`, their mean width and
    /// height into `width`, `height` when not null; returns
    /// `msCheckSameTarget(i, actor, targets)`.
    pub fn ms_get_chr_target_info(
        mem: &mut Dm<'_>,
        host: &Host<'_>,
        i: u32,
        kind: u32,
        pos: u32,
        width: u32,
        height: u32,
    ) -> i32 {
        let a = ms_get_chr(mem, i);
        let set = crate::flow::bridge::ms_effect_check_target(mem, mem.read(a.wrapping_add(0xdc0)));
        let same = crate::actor::bridge::ms_check_same_target(mem, i, a, set);
        let regs = Some([set, pos]);
        ms_get_chr_info(mem, host, kind, set, 0, pos, regs);
        mem.frame(0x10, |mem, loc| {
            ms_get_chr_info(mem, host, 4, set, 0, loc, regs);
            if width != 0 {
                let w = quiet(f32_at(mem, loc));
                mem.write(width, w);
            }
            ms_get_chr_info(mem, host, 3, set, 0, loc, regs);
            if height != 0 {
                let h = quiet(f32_at(mem, loc));
                mem.write(height, h);
            }
        });
        same
    }

    /// The VU words a short call leaves: the caller's registers within the
    /// chain, else what the register held.
    fn regs_or(mem: &impl Mem, at: u32, regs: Option<Regs>, first: u32) -> [u32; 4] {
        match regs {
            Some(r) => [first, r.esi, r.edi, r.ebx],
            None => {
                let mut w = vf_words(mem, at);
                w[0] = first;
                w
            }
        }
    }

    /// `MsVectorNormal2(out, a, b, 10.0)` as `MsGetMoveTargetPos` calls it
    /// (the scale's other words its caller's `esi`, `edi`, `ebx`).
    fn normal10(mem: &mut Dm<'_>, out: u32, a: u32, b: u32, regs: Option<Regs>) {
        let ten = Global::<u32>::new(0x00b9_2318).get(mem);
        let at = v::VF_M.ptr(mem).addr + 0x20;
        let s = regs_or(mem, at, regs, ten);
        v::ms_vector_normal2(mem, out, a, b, s);
    }

    /// `MsGetMoveTargetPos(i, actor, kind, out, &done)` (`0x007a9ac0`):
    /// where actor `i` moves for move kind `kind` (& 0x7f). Kind 1: its
    /// battle position; 2: in front of its targets (their mean position,
    /// less its width, theirs and `+0x488` away, `+0x498` of their height
    /// down); 3: its escape position; 4 and 5: the summoner's position
    /// (kind and kind plus 3); 6: its position; 8: actor `+0xded`'s
    /// position; 9: 10 units from its position (or `+0x390`) towards
    /// `+0xded`'s (kept at `+0x3d0`), turned by pi/4 with `+0x411`; 10:
    /// actor `+0x416`'s position; 11 and 12: like 2 (12 counting
    /// `MOVE_FRAME`); 13: its battle position on the ground, a quarter
    /// height down; 14: its position on the ground; 15: its handle's
    /// optional position 0xe; 17: 10 units along actor `+0x14`'s
    /// direction; 18: a summoner's position, kind 3. Returns the distance
    /// kept (`f32`); `done` gets -1 when kind 12 targets only itself, else
    /// 0. `regs` are the caller's `esi`, `edi`, `ebx` (see the module).
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    pub fn ms_get_move_target_pos(
        mem: &mut Dm<'_>,
        host: &Host<'_>,
        i: u32,
        actor: u32,
        kind: u32,
        out: u32,
        done: u32,
        regs: Option<Regs>,
    ) -> u32 {
        use crate::actor::bridge::ms_get_chr_width;
        use crate::pos::bridge as p;
        use ffx_bridge::x87::{add, load, mul, store, sub};
        let trig = host.trig;
        let mut ret = 0.0f32.to_bits();
        let mut flag = 0u32;
        let summoner = |mem: &Dm<'_>| {
            let g = i32::from(SUMMONER.get(mem)).cast_unsigned();
            (g, ms_get_chr(mem, g))
        };
        let place = |mem: &Dm<'_>, a: u32| {
            [0x4fc, 0x4fd, 0x4fe].map(|o| i32::from(byte(mem, a, o).cast_signed()))
        };
        let btl_get = |mem: &mut Dm<'_>, a: u32, [g, k, x]: [i32; 3]| {
            p::ms_btl_get_pos(mem, 0, a, g, k, x, out, |m, actor, index, kk, vec| {
                p::ms_get_summoner_pos(
                    m,
                    trig,
                    actor,
                    index.cast_unsigned(),
                    kk.cast_unsigned(),
                    vec,
                );
            });
        };
        mem.frame(0x20, |mem, f| {
            let (tmp, w, h) = (f, f + 0x10, f + 0x14);
            let in_front = |mem: &mut Dm<'_>, info: u32, ret: &mut u32, plus_width: bool| {
                ms_get_chr_target_info(mem, host, i, info, out, w, h);
                if (1u32 << (i & 0x1f)) == mem.read::<u32>(actor.wrapping_add(0xdc0)) {
                    *ret = 0.0f32.to_bits();
                    return false;
                }
                let far = f32_at(mem, actor.wrapping_add(0x488));
                *ret = if plus_width {
                    let mine = ms_get_chr_width(mem, i).to_bits();
                    store(add(add(load(mine), load(f32_at(mem, w))), load(far)))
                } else {
                    quiet(far)
                };
                let y = out.wrapping_add(4);
                let drop = mul(
                    load(f32_at(mem, actor.wrapping_add(0x498))),
                    load(f32_at(mem, h)),
                );
                let r = store(sub(load(f32_at(mem, y)), drop));
                mem.write(y, r);
                true
            };
            match kind & 0x7f {
                1 => {
                    p::ms_get_chr_btl_pos(mem, trig, i, out);
                }
                2 => {
                    in_front(mem, 1, &mut ret, true);
                }
                3 => {
                    crate::flow::bridge::ms_get_chr_escape_pos(
                        mem,
                        i,
                        out,
                        |m, actor, index, kk, vec| {
                            p::ms_get_summoner_pos(
                                m,
                                trig,
                                actor,
                                index.cast_unsigned(),
                                kk.cast_unsigned(),
                                vec,
                            );
                        },
                    );
                }
                4 | 5 => {
                    let (_, c) = summoner(mem);
                    let mut at = place(mem, c);
                    if kind & 0x7f == 5 {
                        at[1] += 3;
                    }
                    btl_get(mem, c, at);
                }
                6 => {
                    p::ms_get_chr_pos(mem, trig, i, out);
                }
                8 => {
                    let t = i32::from(byte(mem, actor, 0xded).cast_signed()).cast_unsigned();
                    p::ms_get_chr_pos(mem, trig, t, out);
                }
                9 => {
                    let t = i32::from(byte(mem, actor, 0xded).cast_signed()).cast_unsigned();
                    let goal = actor.wrapping_add(0x3d0);
                    if byte(mem, actor, 0x417) == 0 {
                        mem.write(actor.wrapping_add(0x417), 1u8);
                        p::ms_get_chr_pos(mem, trig, t, goal);
                    }
                    let quarter = Global::<u32>::new(0x00b1_1328).get(mem);
                    let from = if byte(mem, actor, 0x415) == 0 {
                        p::ms_get_chr_pos(mem, trig, i, tmp);
                        tmp
                    } else {
                        actor.wrapping_add(0x390)
                    };
                    normal10(mem, out, from, goal, regs);
                    if byte(mem, actor, 0x411) != 0 {
                        ms_turn_move_vector(mem, host, i, actor, out, quarter);
                    }
                    v::ms_add_point(mem, out, from, out);
                }
                0xa => {
                    let t = u32::from(byte(mem, actor, 0x416));
                    p::ms_get_chr_pos(mem, trig, t, out);
                    let wt = f64::from(ms_get_chr_width(mem, t));
                    let wi = ms_get_chr_width(mem, i).to_bits();
                    let far = f32_at(mem, actor.wrapping_add(0x488));
                    ret = store(add(add(load(wi), wt), load(far)));
                }
                0xb => {
                    in_front(mem, 1, &mut ret, false);
                }
                0xc => {
                    if (1u32 << (i & 0x1f)) == mem.read::<u32>(actor.wrapping_add(0xdc0)) {
                        ms_get_chr_target_info(mem, host, i, 1, out, w, h);
                        ret = 0.0f32.to_bits();
                        flag = u32::MAX;
                    } else {
                        let mut n = i32::from(byte(mem, actor, 0x40c)) + 1;
                        if byte(mem, actor, 0x418) != 0 {
                            n -= i32::from(mem.read::<u16>(actor.wrapping_add(0x452)));
                        }
                        MOVE_FRAME.set(mem, n);
                        in_front(mem, 0xc, &mut ret, true);
                    }
                }
                0xd => {
                    p::ms_get_chr_btl_pos(mem, trig, i, out);
                    let g = f64::from(crate::body::bridge::ms_get_chr_ground(mem, i));
                    let hh = crate::body::bridge::ms_get_chr_height(mem, i).to_bits();
                    let quarter = Global::<f64>::new(0x00b3_4f90).get(mem);
                    mem.write(out.wrapping_add(4), store(sub(g, mul(load(hh), quarter))));
                }
                0xe => {
                    p::ms_get_chr_pos(mem, trig, i, out);
                    let g = crate::body::bridge::ms_get_chr_ground(mem, i).to_bits();
                    mem.write(out.wrapping_add(4), quiet(g));
                }
                0xf => {
                    let t = u32::from(byte(mem, actor, 0x14));
                    let hnd = crate::actor::bridge::ms_get_chr_handle(mem, t);
                    ffx_chara::attach::bridge::ch_optpos_get(mem, hnd, 0xe, out);
                }
                0x11 => {
                    let t = u32::from(byte(mem, actor, 0x14));
                    let d = f + 0x18;
                    p::ms_get_chr_pos(mem, trig, t, tmp);
                    crate::body::bridge::ms_get_chr_dir(mem, t, d);
                    let ten = Global::<u32>::new(0x00b9_2318).get(mem);
                    let at = v::VF_M.ptr(mem).addr + 0x40;
                    let r = regs_or(mem, at, regs, ten);
                    let yaw = quiet(f32_at(mem, d));
                    v::ms_vector_polar2_rect(mem, trig, out, 0, yaw, 0, r);
                    v::ms_add_point(mem, out, tmp, out);
                }
                0x12 => {
                    let (g, c) = summoner(mem);
                    p::ms_get_summoner_pos(mem, trig, c, g, 3, out);
                }
                _ => {}
            }
        });
        if done != 0 {
            mem.write(done, flag);
        }
        ret
    }

    /// `MsGetChrMoveAccelPos(actor, n, speed, target, out)` (`0x007aac40`):
    /// frame `n` of an accelerated move of actor `actor` (its index `+0xc`)
    /// from its position towards `target` (`+0x34`: in x and z only) at
    /// `speed`: the move's phases (`+0x454` frames accelerating, `+0x456`
    /// cruising, `+0x458` braking, from `+0x45c`) give the distance
    /// covered, scaled to the remaining one (`+0x4a4` of `+0x4a4 - +0x4a0`),
    /// into `out` (from the position); `+0x4a0` and `+0x4a4` get the
    /// distance covered and the scale. 0 (nothing written) when the scale
    /// is 0. `regs` are the caller's `edi`, `esi`, `ebx`, which the scaling
    /// `MsMulVector` leaves in `VF_B`.
    #[allow(
        clippy::too_many_lines,
        clippy::similar_names,
        clippy::too_many_arguments
    )]
    pub fn ms_get_chr_move_accel_pos(
        mem: &mut Dm<'_>,
        trig: &dyn Fn(Trig, u32) -> u32,
        actor: u32,
        n: u32,
        speed: u32,
        target: u32,
        out: u32,
        regs: Option<Regs>,
    ) -> u32 {
        use ffx_bridge::x87::{add, div, load, mul, store, sub};
        let n = n.cast_signed();
        mem.frame(0x20, |mem, f| {
            let (cur, d) = (f, f + 0x10);
            let i = u32::from(byte(mem, actor, 0xc));
            crate::pos::bridge::ms_get_chr_pos(mem, trig, i, cur);
            v::ms_sub_point(mem, d, target, cur);
            let dist = if byte(mem, actor, 0x34) == 0 {
                v::ms_vector_len(mem, d).to_bits()
            } else {
                let l = v::ms_vector_len_2d(mem, d).to_bits();
                mem.write(d.wrapping_add(4), 0.0f32.to_bits());
                l
            };
            let a4 = load(f32_at(mem, actor.wrapping_add(0x4a4)));
            let a0 = load(f32_at(mem, actor.wrapping_add(0x4a0)));
            let g = load(store(sub(a4, a0)));
            let k = if g == 0.0 {
                0.0f32.to_bits()
            } else {
                store(div(mul(load(dist), a4), g))
            };
            v::ms_vector_normal(mem, d, d);
            let b = match regs {
                Some(r) => [k, r.edi, r.esi, r.ebx],
                None => {
                    let mut w = vf_words(mem, v::VF_B.ptr(mem).addr);
                    w[0] = k;
                    w
                }
            };
            v::ms_mul_vector(mem, d, b);
            v::ms_sub_point(mem, cur, target, d);
            let kv = load(k);
            if kv == 0.0 {
                return 0;
            }
            let u16_at = |mem: &Dm<'_>, o: u32| i32::from(mem.read::<u16>(actor.wrapping_add(o)));
            let (acc, cruise, brake) = (u16_at(mem, 0x454), u16_at(mem, 0x456), u16_at(mem, 0x458));
            let v0 = load(f32_at(mem, actor.wrapping_add(0x45c)));
            let s = load(speed);
            let two = Global::<f64>::new(0x00b9_2270).get(mem);
            let half = HALF.get(mem);
            let int = |x: i32| load(store(f64::from(x)));
            // the acceleration: (k - speed) · 2 - (2b + c + 2a) · v0, over
            // (a + c + 2b) · a
            let t = int(acc
                .wrapping_add(brake)
                .wrapping_add(2 * cruise)
                .wrapping_mul(acc));
            let q = if t == 0.0 {
                0.0
            } else {
                let u = int(brake.wrapping_add(2 * acc.wrapping_add(cruise)));
                div(sub(mul(sub(kv, s), two), mul(u, v0)), t)
            };
            let q = store(q);
            let r = if n < acc {
                let nf = load(store(load(store(f64::from(n)))));
                let p = mul(mul(mul(nf, load(q)), nf), half);
                add(mul(nf, v0), p)
            } else {
                // after the acceleration: its distance d1, the cruising
                // speed vv, e = cruise · vv, and the braking dec
                let (d1, vv, e, dec);
                if acc != 0 {
                    let ad = f64::from(acc);
                    let qa = mul(load(q), ad);
                    let d = add(mul(ad, v0), mul(mul(qa, ad), half));
                    d1 = store(d);
                    let w = add(qa, v0);
                    vv = store(w);
                    e = store(mul(f64::from(cruise), w));
                    dec = if brake == 0 {
                        store(-0.0)
                    } else {
                        store(-div(w, f64::from(brake)))
                    };
                } else {
                    d1 = 0.0f32.to_bits();
                    if brake != 0 {
                        let vf = load(store(load(store(f64::from(
                            (2 * cruise)
                                .wrapping_add(brake)
                                .wrapping_mul(brake)
                                .wrapping_neg(),
                        )))));
                        dec = if vf == 0.0 {
                            0.0f32.to_bits()
                        } else {
                            store(div(mul(sub(kv, s), two), vf))
                        };
                        vv = store(mul(-load(dec), f64::from(brake)));
                    } else {
                        dec = 0.0f32.to_bits();
                        vv = if cruise == 0 {
                            0.0f32.to_bits()
                        } else {
                            store(div(sub(kv, s), f64::from(cruise)))
                        };
                    }
                    e = store(mul(f64::from(cruise), load(vv)));
                }
                if n < acc.wrapping_add(cruise) {
                    let m = load(store(f64::from(n.wrapping_sub(acc))));
                    add(mul(load(vv), m), load(d1))
                } else {
                    let m = load(store(load(store(f64::from(
                        n.wrapping_sub(acc.wrapping_add(cruise)),
                    )))));
                    let g = add(mul(load(vv), m), add(load(e), load(d1)));
                    add(mul(half, mul(m, mul(m, load(dec)))), g)
                }
            };
            let mut r = store(r);
            let last = u16_at(mem, 0x45a);
            if n >= last {
                r = store(sub(kv, s));
            }
            let rv = load(r);
            let dv: [u32; 3] = mem.read(d);
            let x = store(div(mul(load(dv[0]), rv), kv));
            let y = store(div(mul(load(dv[1]), rv), kv));
            let z = store(div(mul(rv, load(dv[2])), kv));
            mem.write(d, [x, y, z]);
            v::ms_add_point(mem, out, cur, d);
            mem.write(actor.wrapping_add(0x4a0), r);
            mem.write(actor.wrapping_add(0x4a4), k);
            1
        })
    }

    /// `MsGetChrMovePos(i, out)` (`0x007a93e0`): where actor `i` is on its
    /// move: a fixed point (`+0x425`: `+0x3c0`, through `EE_MOVE`), else its
    /// position plus, by the move kind `+0x415`: 7 a spline (`+0x454`) at
    /// frame `+0x452` + [`MOVE_FRAME`] of `+0x45a`; other kinds the step
    /// towards [`ms_get_move_target_pos`]'s point, at `+0x460` growing by
    /// `+0x468` per frame up to `+0x464` (bit 0x80) or accelerated
    /// ([`ms_get_chr_move_accel_pos`], its `+0x4a0`/`+0x4a4` kept). `regs`
    /// are the caller's `edi`, `esi`, `ebx` (see the module). (For a kind
    /// `MsGetMoveTargetPos` writes nothing for, 16 and above 0x12, or a
    /// spline `TkMtCalcSpline` writes nothing for, of 2 or fewer points,
    /// the original moves by its stack garbage; here the zero vector.)
    #[allow(clippy::too_many_lines)]
    pub fn ms_get_chr_move_pos(
        mem: &mut Dm<'_>,
        host: &Host<'_>,
        i: u32,
        out: u32,
        regs: Option<Regs>,
    ) {
        use ffx_bridge::x87::{add, div, load, store};
        let a = ms_get_chr(mem, i);
        if byte(mem, a, 0x425) != 0 {
            let p: [u32; 4] = mem.read(a.wrapping_add(0x3c0));
            crate::camera::bridge::EE_MOVE.set(mem, p);
            mem.write(out, p);
            return;
        }
        crate::pos::bridge::ms_get_chr_pos(mem, host.trig, i, out);
        let kind = u32::from(byte(mem, a, 0x415));
        let start = i32::from(mem.read::<u16>(a.wrapping_add(0x452)));
        let own = Regs {
            edi: out,
            esi: a,
            ebx: start.cast_unsigned(),
        };
        mem.frame(0x20, |mem, f| {
            let (tmp, done) = (f, f + 0x10);
            let frame = MOVE_FRAME.get(mem);
            if kind & 0x7f == 7 {
                let len = i32::from(mem.read::<u16>(a.wrapping_add(0x45a)));
                let at = start.wrapping_add(frame).min(len);
                let int = |x: i32| load(store(f64::from(x)));
                let t = store(load(store(div(int(at), int(len)))));
                let k = u32::from(mem.read::<u16>(a.wrapping_add(0x454)));
                let table = MOVE_SPLINES.ptr(mem).addr.wrapping_add(k * 0x210);
                let count: u32 = mem.read(table);
                (host.spline)(mem, tmp, table.wrapping_add(0x10), count, t);
                v::ms_add_point(mem, out, out, tmp);
                return;
            }
            if kind.trailing_zeros() >= 7 {
                return;
            }
            let ret = ms_get_move_target_pos(mem, host, i, a, kind, tmp, done, Some(own));
            if mem.read::<u32>(done) != 0 {
                return;
            }
            if kind & 0x80 != 0 {
                let step = load(quiet(f32_at(mem, a.wrapping_add(0x468))));
                let cap = load(quiet(f32_at(mem, a.wrapping_add(0x464))));
                let mut speed = quiet(f32_at(mem, a.wrapping_add(0x460)));
                let mut sum = 0.0f32.to_bits();
                for _ in 0..frame.max(0) {
                    speed = store(add(load(speed), step));
                    if load(speed) > cap {
                        speed = store(cap);
                    }
                    sum = store(add(load(speed), load(sum)));
                }
                let d = v::ms_point_len(mem, out, tmp).to_bits();
                let m = if load(d) < load(sum) { d } else { sum };
                // here the words above are the caller's edi, esi, ebx
                let s = match regs {
                    Some(r) => [m, r.edi, r.esi, r.ebx],
                    None => {
                        let mut w = vf_words(mem, v::VF_M.ptr(mem).addr + 0x20);
                        w[0] = m;
                        w
                    }
                };
                v::ms_vector_normal2(mem, tmp, tmp, out, s);
                v::ms_add_point(mem, out, out, tmp);
            } else {
                let keep4 = quiet(f32_at(mem, a.wrapping_add(0x4a4)));
                let keep0 = quiet(f32_at(mem, a.wrapping_add(0x4a0)));
                let n = frame.wrapping_add(start).cast_unsigned();
                ms_get_chr_move_accel_pos(mem, host.trig, a, n, ret, tmp, out, Some(own));
                mem.write(a.wrapping_add(0x4a4), keep4);
                mem.write(a.wrapping_add(0x4a0), keep0);
            }
        });
    }

    /// `MsSetTargetDir(i, targets)` (`0x007969f0`): faces actor `i` and
    /// the others for an action: `+0x49c` = a direction plus pi, near 0
    /// (`MsNearRag`). `i`'s is towards its targets' mean battle position
    /// when `msCheckSameTarget` says it targets others, else its battle
    /// direction; each other actor of `targets` gets the direction from it
    /// to `i`, every actor not in `targets` `i`'s battle direction.
    pub fn ms_set_target_dir(mem: &mut Dm<'_>, host: &Host<'_>, i: u32, targets: u32) {
        use ffx_bridge::x87::{add, load, store};
        let i = i & 0xff;
        let a = ms_get_chr(mem, i);
        mem.frame(0x30, |mem, f| {
            let (home, center, dir) = (f, f + 0x10, f + 0x20);
            crate::pos::bridge::ms_get_chr_btl_pos(mem, host.trig, i, home);
            let d = crate::actor::bridge::ms_get_chr_btl_dir(mem, i).to_bits();
            mem.write(dir, store(load(d)));
            if crate::actor::bridge::ms_check_same_target(mem, i, a, targets) != 0 {
                let t = crate::flow::bridge::ms_effect_check_target(mem, targets);
                ms_get_chr_info(mem, host, 5, t, 0, center, Some([i, targets]));
                v::ms_vector_rect2_polar(mem, host.trig, center, home, dir, 0, 0);
            }
            let pi = crate::vector::bridge::PI.get(mem);
            let face = |mem: &mut Dm<'_>, who: u32| {
                let x = store(add(load(f32_at(mem, dir)), pi));
                let r = v::ms_near_rag(mem, 0, x).to_bits();
                mem.write(dir, r);
                mem.write(who.wrapping_add(0x49c), quiet(r));
            };
            face(mem, a);
            for j in 0..0x1f {
                if j == i {
                    continue;
                }
                let c = ms_get_chr(mem, j);
                if targets.cast_signed() >> j & 1 != 0 {
                    crate::pos::bridge::ms_get_chr_btl_pos(mem, host.trig, j, center);
                    v::ms_vector_rect2_polar(mem, host.trig, home, center, dir, 0, 0);
                } else {
                    let d = crate::actor::bridge::ms_get_chr_btl_dir(mem, i).to_bits();
                    mem.write(dir, store(load(d)));
                }
                face(mem, c);
            }
        });
    }

    /// `0x01730870`: Rikku's mix table (`MsGetExcelData`'s layout): record
    /// `max` holds, at `2 · min`, what items `min` and `max` (12 bits)
    /// make.
    pub const MIX_TABLE: Global<u8> = Global::new(0x0173_0870);

    /// `MsSetRikkuLimit(chr, actor, cue)` (`0x007b0520`): sets up Rikku's
    /// Mix in the cue entry: the first two item IDs of its commands (not
    /// 0xff, 0x311e or 0x3001) are used up and replaced by the item of the
    /// [`MIX_TABLE`] (one item: just used); the entry gets that single
    /// command (`+3` 1, `+8` the ID, `+0xa` 0xff), its default targets
    /// (`MsGetCommandTarget`, `+0x10`), the actors face them
    /// ([`ms_set_target_dir`]), a mix with targets is learned
    /// (`MsSetSaveCommand`), and `+0x717` is set. Returns 1 when there are
    /// no targets (also at once with `+0xdfd`, or with `+0x717` already set
    /// and `+0x435`), else 0.
    pub fn ms_set_rikku_limit(
        mem: &mut Dm<'_>,
        host: &Host<'_>,
        chr: u32,
        actor: u32,
        cue: u32,
    ) -> u32 {
        use crate::save::bridge::ms_save_item_use;
        if byte(mem, actor, 0xdfd) != 0 {
            return 1;
        }
        if byte(mem, actor, 0x717) != 0 {
            return u32::from(byte(mem, actor, 0x435) != 0);
        }
        let mut items = [0xffu32; 2];
        let mut n = 0;
        for k in 0..u32::from(byte(mem, cue, 3)) {
            for o in [8, 10] {
                let id = u32::from(mem.read::<u16>(cue.wrapping_add(k * 16 + o)));
                if !matches!(id, 0xff | 0x311e | 0x3001) && n < 2 {
                    items[n] = id;
                    n += 1;
                }
            }
        }
        let mut item = items[0];
        if n >= 2 {
            ms_save_item_use(mem, items[0], -1);
            ms_save_item_use(mem, items[1], -1);
            let (x, y) = (items[0] & 0xfff, items[1] & 0xfff);
            let (lo, hi) = if x > y { (y, x) } else { (x, y) };
            let table = MIX_TABLE.ptr(mem).addr;
            #[allow(clippy::cast_possible_wrap)] // 12 bits
            let r = ffx_kernel::bridge::ms_get_excel_data(mem, hi as i32, table, 0);
            item = u32::from(mem.read::<u16>(r.wrapping_add(lo * 2)));
        } else {
            ms_save_item_use(mem, item, -1);
        }
        mem.write(cue.wrapping_add(3), 1u8);
        #[allow(clippy::cast_possible_truncation)] // a u16 ID
        mem.write(cue.wrapping_add(8), [item as u16, 0xff]);
        let targets = crate::stat::bridge::ms_get_command_target(mem, host.trig, chr, item, -1, 1);
        mem.write(cue.wrapping_add(0x10), targets);
        ms_set_target_dir(mem, host, chr, targets);
        if targets != 0 && n >= 2 {
            crate::ability::bridge::ms_set_save_command(mem, chr, item, 1);
        }
        mem.write(actor.wrapping_add(0x717), 1u8);
        u32::from(targets == 0)
    }

    /// `2pi/3` and its negation, the turn limits of [`ms_dir_turn`].
    pub const TURN_LIMIT: Global<u32> = Global::new(0x00b5_4f28);
    pub const TURN_LIMIT_NEG: Global<u32> = Global::new(0x00b5_4f2c);
    /// `2pi` (`double`).
    pub const TWO_PI_D: Global<f64> = Global::new(0x00b3_9378);
    /// `pi` (`f32`), the turn `MsGetDirMode` adds for "away" modes.
    pub const PI_F: Global<u32> = Global::new(0x00b9_22d0);
    /// The EE's `r0` (16 zero bytes), which `MsSetChrDirTarget` starts its
    /// target vector from.
    pub const EE_R0: Global<[u32; 4]> = Global::new(0x00c8_f508);
    /// The actor whose command runs (`g_BtlCommandActor`).
    pub const COMMAND_ACTOR: Global<u8> = Global::new(0x0112_c9e0);

    /// `0x007a9650(actor, dir)` (`MsDirTurn`, name ours): `dir` as a turn
    /// from the actor's direction (`+0x490`, `MsNearRag`): the near angle
    /// `n`; with `d = n - +0x490`, a turn the other way round is taken
    /// (`+0x490 + d -/+ 2pi`) when the remaining turn (`+0x494`) lies
    /// beyond `-2pi/3` (for `d` not below 0, `d` beyond `2pi/3`) or below
    /// `2pi/3` (`d` below `-2pi/3`). An `f32` in ST0.
    pub fn ms_dir_turn(mem: &impl Mem, actor: u32, dir: u32) -> u32 {
        use ffx_bridge::x87::{add, load, store, sub};
        let base = f32_at(mem, actor.wrapping_add(0x490));
        let n = v::ms_near_rag(mem, base, dir).to_bits();
        let d = store(sub(load(n), load(base)));
        let (ld, rest) = (load(d), load(f32_at(mem, actor.wrapping_add(0x494))));
        let (lim, neg) = (load(TURN_LIMIT.get(mem)), load(TURN_LIMIT_NEG.get(mem)));
        let two_pi = TWO_PI_D.get(mem);
        // `fcom` with an unordered result reads as below
        let below = ld < 0.0 || ld.is_nan();
        if !below {
            if neg > rest && lim < ld {
                return store(sub(add(ld, load(base)), two_pi));
            }
        } else if lim < rest && neg > ld {
            return store(add(add(ld, load(base)), two_pi));
        }
        n
    }

    /// `MsGetDirMode(i, actor, vec, follow, &mode, &plain)` (`0x007a9710`):
    /// the direction actor `i` should face for its direction mode
    /// (`+0x42b`): 1 its battle direction (`+0x514[+0x511]`), 6 the
    /// summoner's; 3, 4, 5, 8, 13 towards its battle position, actor
    /// `+0x416`, actor `+0xded`, `vec` itself or its escape position
    /// (`vec` getting the point); 2 and 7 towards its (or the command
    /// actor's) targets' centre (`MsGetChrTargetInfo` kind 12; mode 2 with
    /// `+0x431` an actor faces that actor instead, its own index meaning
    /// mode 1); 11 towards `+0x3e0` (set once from actor `+0xded`'s
    /// position with `follow`); 12 `+0x4f4`. Modes 9, 10, 13 add pi to
    /// modes 2, 5, 13's. With `+0x42c` the result goes through
    /// [`ms_dir_turn`] (`plain` 0, else 1). `mode` gets -1, or the target
    /// lookup's result, 0 for none. An `f32` in ST0.
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    pub fn ms_get_dir_mode(
        mem: &mut Dm<'_>,
        host: &Host<'_>,
        i: u32,
        actor: u32,
        vec: u32,
        follow: u32,
        mode_out: u32,
        plain_out: u32,
    ) -> u32 {
        use crate::pos::bridge as p;
        use ffx_bridge::x87::{add, load, store};
        let trig = host.trig;
        mem.frame(0x20, |mem, f| {
            let (pos, ang) = (f, f + 0x10);
            mem.write(ang, 0.0f32.to_bits());
            let mut base = 0.0f32.to_bits();
            let mut mode: i32 = 0;
            p::ms_get_chr_pos(mem, trig, i, pos);
            let toward = |mem: &mut Dm<'_>, from: u32| {
                v::ms_vector_rect2_polar(mem, trig, from, pos, ang, 0, 0);
            };
            let battle_dir = |mem: &mut Dm<'_>, a: u32| {
                let k = i32::from(mem.read::<i8>(a.wrapping_add(0x511)));
                let at = a.wrapping_add(0x514).wrapping_add((k * 4).cast_unsigned());
                quiet(f32_at(mem, at))
            };
            let kind = byte(mem, actor, 0x42b);
            if matches!(kind, 9 | 10 | 13) {
                base = PI_F.get(mem);
            }
            match kind {
                1 => {
                    let d = battle_dir(mem, actor);
                    mem.write(ang, d);
                    mode = -1;
                }
                6 => {
                    mode = -1;
                    let s = i32::from(SUMMONER.get(mem)).cast_unsigned();
                    let a = ms_get_chr(mem, s);
                    let d = battle_dir(mem, a);
                    mem.write(ang, d);
                }
                2 | 9 => {
                    let t = byte(mem, actor, 0x431);
                    let one = if t == 0xff {
                        MOVE_FRAME.set(mem, 4);
                        mode = ms_get_chr_target_info(mem, host, i, 0xc, vec, 0, 0);
                        mode == 0
                    } else if i == u32::from(t) {
                        true
                    } else {
                        p::ms_get_chr_pos(mem, trig, u32::from(t), vec);
                        mode = -1;
                        false
                    };
                    if one {
                        let d = battle_dir(mem, actor);
                        mem.write(ang, d);
                        mode = -1;
                    } else {
                        toward(mem, vec);
                    }
                }
                3 => {
                    p::ms_get_chr_btl_pos(mem, trig, i, vec);
                    mode = -1;
                    toward(mem, vec);
                }
                4 => {
                    p::ms_get_chr_pos(mem, trig, u32::from(byte(mem, actor, 0x416)), vec);
                    mode = -1;
                    toward(mem, vec);
                }
                5 | 10 => {
                    let t = i32::from(byte(mem, actor, 0xded).cast_signed()).cast_unsigned();
                    p::ms_get_chr_pos(mem, trig, t, vec);
                    mode = -1;
                    toward(mem, vec);
                }
                7 => {
                    MOVE_FRAME.set(mem, 4);
                    let c = u32::from(COMMAND_ACTOR.get(mem));
                    mode = ms_get_chr_target_info(mem, host, c, 0xc, vec, 0, 0);
                    toward(mem, vec);
                }
                8 => {
                    mode = -1;
                    toward(mem, vec);
                }
                11 => {
                    mode = -1;
                    let at = actor.wrapping_add(0x3e0);
                    if byte(mem, actor, 0x42d) == 0 && follow != 0 {
                        mem.write(actor.wrapping_add(0x42d), 1u8);
                        let t = i32::from(byte(mem, actor, 0xded).cast_signed()).cast_unsigned();
                        p::ms_get_chr_pos(mem, trig, t, at);
                    }
                    toward(mem, at);
                }
                12 => {
                    let d = quiet(f32_at(mem, actor.wrapping_add(0x4f4)));
                    mem.write(ang, d);
                    mode = -1;
                }
                13 => {
                    mode = -1;
                    crate::flow::bridge::ms_get_chr_escape_pos(
                        mem,
                        i,
                        vec,
                        |m, a, index, kk, out| {
                            p::ms_get_summoner_pos(
                                m,
                                trig,
                                a,
                                index.cast_unsigned(),
                                kk.cast_unsigned(),
                                out,
                            );
                        },
                    );
                    toward(mem, vec);
                }
                _ => {}
            }
            let plain = if byte(mem, actor, 0x42c) != 0 {
                let d = f32_at(mem, ang);
                let t = ms_dir_turn(mem, actor, d);
                mem.write(ang, t);
                0u32
            } else {
                1
            };
            let r = store(add(load(base), load(f32_at(mem, ang))));
            if mode_out != 0 {
                mem.write(mode_out, mode);
            }
            if plain_out != 0 {
                mem.write(plain_out, plain);
            }
            r
        })
    }

    /// `MsSetChrDirTarget(i, mode, turn, target, a, b)` (`0x00795eb0`):
    /// actor `i & 0xff` (unless `+0x41b` holds it) gets direction mode
    /// `mode` (`+0x42b`), `turn` (`+0x42c`), `+0x42d` cleared, `target`
    /// (`+0x431`), `a` and `b` (`+0x42f`, `+0x430`; `b` 1 when `b + 2a`
    /// is not above 0) and their sum `2a + b` (`+0x42e`), its direction
    /// (`MsGetChrDir`, as `MsNearRag(0, dir)`) in `+0x490` and, without
    /// `turn`, the turn left to [`ms_get_dir_mode`]'s direction in
    /// `+0x494`. -1, or 0 for a non-actor or a held one.
    #[allow(clippy::too_many_arguments)]
    pub fn ms_set_chr_dir_target(
        mem: &mut Dm<'_>,
        host: &Host<'_>,
        i: u32,
        mode: u32,
        turn: u32,
        target: u32,
        a: u32,
        b: u32,
    ) -> i32 {
        use ffx_bridge::x87::{load, store, sub};
        let i = i & 0xff;
        if ms_btl_chr_num_check(i) == 0 {
            return 0;
        }
        let actor = ms_get_chr(mem, i);
        if byte(mem, actor, 0x41b) != 0 {
            return 0;
        }
        mem.write(actor.wrapping_add(0x42b), mode.to_le_bytes()[0]);
        mem.write(actor.wrapping_add(0x42c), turn.to_le_bytes()[0]);
        mem.write(actor.wrapping_add(0x42d), 0u8);
        let b = if a.wrapping_mul(2).wrapping_add(b).cast_signed() <= 0 {
            1
        } else {
            b
        };
        let sum = a.to_le_bytes()[0]
            .wrapping_mul(2)
            .wrapping_add(b.to_le_bytes()[0]);
        mem.write(actor.wrapping_add(0x42e), sum);
        mem.write(actor.wrapping_add(0x431), target.to_le_bytes()[0]);
        mem.write(actor.wrapping_add(0x42f), a.to_le_bytes()[0]);
        mem.write(actor.wrapping_add(0x430), b.to_le_bytes()[0]);
        mem.frame(0x20, |mem, f| {
            crate::body::bridge::ms_get_chr_dir(mem, i, f);
            let d = f32_at(mem, f);
            let r = v::ms_near_rag(mem, 0, d).to_bits();
            mem.write(actor.wrapping_add(0x490), r);
            if turn == 0 {
                let vec = f + 0x10;
                let r0 = EE_R0.get(mem);
                mem.write(vec, r0);
                let m = ms_get_dir_mode(mem, host, i, actor, vec, 0, 0, 0);
                let n = v::ms_near_rag(mem, r, m);
                let t = store(sub(f64::from(n), load(r)));
                mem.write(actor.wrapping_add(0x494), t);
            }
        });
        -1
    }

    /// `12.0`, `0.2` (`double`): [`ms_check_targets_near`]'s distances.
    pub const NEAR_REACH: Global<u32> = Global::new(0x00b4_7de8);
    pub const NEAR_STOP: Global<f64> = Global::new(0x00b9_2208);

    /// `MsGetEscapeDir(i)` (`0x007a99b0`): the direction from actor `i`'s
    /// position to its escape position (`MsVectorRect2Polar` yaw, as
    /// `MsNearRag(0, yaw)`). An `f32` in ST0.
    pub fn ms_get_escape_dir(mem: &mut Dm<'_>, host: &Host<'_>, i: u32) -> u32 {
        use crate::pos::bridge as p;
        let trig = host.trig;
        mem.frame(0x30, |mem, f| {
            let (esc, pos, yaw) = (f, f + 0x10, f + 0x20);
            crate::flow::bridge::ms_get_chr_escape_pos(mem, i, esc, |m, a, index, kk, out| {
                p::ms_get_summoner_pos(m, trig, a, index.cast_unsigned(), kk.cast_unsigned(), out);
            });
            p::ms_get_chr_pos(mem, trig, i, pos);
            v::ms_vector_rect2_polar(mem, trig, esc, pos, yaw, 0, 0);
            let y = f32_at(mem, yaw);
            v::ms_near_rag(mem, 0, y).to_bits()
        })
    }

    /// `0x007af0f0(i, actor, targets)` (`MsCheckTargetsNear`, name ours):
    /// for an actor that walks up to its targets (`+0x4ac` bit 12), 1 when
    /// a target (not itself with `+0x4b4`) within 12.0 of it (2D distance
    /// less both widths) has its move target (`MsGetMoveTargetPos` kind
    /// 0xc, `+0x40c` from `+0x40d` meanwhile) within 0.2 of that target's
    /// battle position; the position compared from then on is that
    /// target's battle position. 0 when there are several targets.
    pub fn ms_check_targets_near(
        mem: &mut Dm<'_>,
        host: &Host<'_>,
        i: u32,
        actor: u32,
        targets: u32,
    ) -> u32 {
        use crate::actor::bridge::ms_get_chr_width;
        use crate::pos::bridge as p;
        use ffx_bridge::x87::{load, store, sub};
        if mem.read::<u32>(actor.wrapping_add(0x4ac)) & 0x1000 == 0 {
            return 0;
        }
        let trig = host.trig;
        mem.frame(0x20, |mem, f| {
            let (pos, q) = (f, f + 0x10);
            p::ms_get_chr_pos(mem, trig, i, pos);
            let wi = ms_get_chr_width(mem, i).to_bits();
            let (mut n, mut near) = (0u32, 0u32);
            for j in 0..0x1f {
                if targets.cast_signed() >> j & 1 == 0 {
                    continue;
                }
                n += 1;
                if j == i && byte(mem, actor, 0x4b4) != 0 {
                    continue;
                }
                p::ms_get_chr_btl_pos(mem, trig, j, q);
                let d = v::ms_point_len_2d(mem, pos, q).to_bits();
                // (d - wi) as a double, less the target's width, as f32
                let t = sub(load(d), load(wi));
                let wj = f64::from(ms_get_chr_width(mem, j));
                let r = store(t - wj);
                let close = load(r) < load(NEAR_REACH.get(mem));
                if !close {
                    continue;
                }
                p::ms_get_chr_btl_pos(mem, trig, j, pos);
                let saved = byte(mem, actor, 0x40c);
                let v40d = byte(mem, actor, 0x40d);
                mem.write(actor.wrapping_add(0x40c), v40d);
                let regs = Regs {
                    edi: actor,
                    esi: j,
                    ebx: u32::from(saved),
                };
                ms_get_move_target_pos(mem, host, i, actor, 0xc, q, 0, Some(regs));
                mem.write(actor.wrapping_add(0x40c), saved);
                let d2 = f64::from(v::ms_point_len_2d(mem, pos, q));
                if d2 < NEAR_STOP.get(mem) {
                    near = 1;
                }
            }
            if n > 1 { 0 } else { near }
        })
    }

    /// `0x007af230(i, actor)` (`MsCheckChrNear`, name ours): unless
    /// `+0x44b`, [`ms_check_targets_near`] over its targets (`+0xdc0`)
    /// into `+0x44a` (else `+0x44b` there); the value stored.
    pub fn ms_check_chr_near(mem: &mut Dm<'_>, host: &Host<'_>, i: u32, actor: u32) -> u32 {
        let mut v = u32::from(byte(mem, actor, 0x44b));
        if v == 0 {
            let t = mem.read(actor.wrapping_add(0xdc0));
            v = ms_check_targets_near(mem, host, i, actor, t);
        }
        mem.write(actor.wrapping_add(0x44a), v.to_le_bytes()[0]);
        v
    }
}
