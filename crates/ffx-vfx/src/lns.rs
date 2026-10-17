//! The `pppKeLns*` draws: the lens-flare programs' wrappers round the
//! `KeLns*` drawers (`docs/formats/vfx.md`, "Lens-flare draws").

/// The functions over game memory.
pub mod bridge {
    use crate::draw::bridge::g;
    use ffx_bridge::Mem;
    use ffx_bridge::x87::{add, cvttsd2si, load, store};

    const SET_PTR32: u32 = 0x00a4_4700;
    const COPY_VECTOR: u32 = 0x006e_d370;
    const APPLY_MATRIX: u32 = 0x006e_d2e0;
    const COPY_VEC_XYZ: u32 = 0x0075_ede0;
    const LP_UPDATE: u32 = 0x0076_a690;
    /// `g_pppCurrentPart`.
    const CURRENT_PART: u32 = 0x0230_ffe4;
    /// `g_pppScreenMatrix`.
    const SCREEN_MATRIX: u32 = 0x0230_fe60;
    /// `g_pppViewMatrix`.
    const VIEW_MATRIX: u32 = 0x0230_ff20;

    /// One of the three lens-flare kinds with a descriptor draw.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Lns {
        /// `pppKeLnsArnd*`, drawn by `KeLnsArnd_Draw`.
        Arnd,
        /// `pppKeLnsCrn*`, drawn by `KeLnsCrn_Draw`.
        Crn,
        /// `pppKeLnsFls*`, drawn by `KeLnsFls_Draw`.
        Fls,
    }

    /// Where a kind keeps its fields in the work area `a = obj + 0xa0 +
    /// [[prog + 0xc]]`.
    #[derive(Debug, Clone, Copy)]
    struct Layout {
        /// `pppKeLns*Update(obj, key, prog)`.
        update: u32,
        /// `KeLns*_Draw(desc, flags)`.
        draw: u32,
        /// The size (`+0x2c`/`+0x30`/`+0x34`, `f32`).
        size: u32,
        /// The colour bytes.
        colour: u32,
        /// The position (`x, y, z, w`).
        pos: u32,
    }

    impl Lns {
        const fn layout(self) -> Layout {
            match self {
                Self::Arnd => Layout {
                    update: 0x0075_a2d0,
                    draw: 0x0076_9340,
                    size: 0x34,
                    colour: 0x38,
                    pos: 0x14,
                },
                Self::Crn => Layout {
                    update: 0x0075_aab0,
                    draw: 0x0076_9d70,
                    size: 0x2c,
                    colour: 0x30,
                    pos: 0xc,
                },
                Self::Fls => Layout {
                    update: 0x0075_ae90,
                    draw: 0x0076_a1f0,
                    size: 0x30,
                    colour: 0x34,
                    pos: 0x10,
                },
            }
        }
    }

    fn at(base: u32, off: u32) -> u32 {
        base.wrapping_add(off)
    }

    /// An `f32` copied through the FPU (`fld`, `fstp`).
    fn copy(mem: &impl Mem, from: u32) -> u32 {
        store(load(mem.read(from)))
    }

    /// The key's colour (`+8..+0xb`) scaled by the current part's `+0x70`
    /// halves: `part * key >> 12` per channel, into `to..to + 4`.
    fn colour(mem: &mut impl Mem, key: u32, to: u32) {
        for i in 0..4 {
            let part: u32 = mem.read(g(mem, CURRENT_PART));
            let p = u32::from(mem.read::<u16>(at(part, 0x70 + 2 * i)));
            let k = u32::from(mem.read::<u8>(at(key, 8 + i)));
            #[allow(clippy::cast_possible_truncation)] // mov byte, cl
            mem.write(at(to, i), ((p * k) >> 12) as u8);
        }
    }

    fn set_ptr32<M: Mem>(
        mem: &mut M,
        v: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        call(mem, g(mem, SET_PTR32), &[v])
    }

    /// `x` (an `f32`) as `fistp` with truncation stores it, the low half
    /// kept (0 for anything outside `i32`).
    #[allow(clippy::cast_possible_truncation)] // the low half
    fn chop16(mem: &impl Mem, from: u32) -> u16 {
        cvttsd2si(load(mem.read(from))) as u16
    }

    /// `pppKeLnsArndDraw` (`0x00759ff0`), `pppKeLnsCrnDraw` (`0x0075a760`)
    /// and `pppKeLnsFlsDraw` (`0x0075ab50`) `(obj, key, prog)`: the
    /// kind's update, then a descriptor at `frame - 0x30` for its drawer
    /// (flags 0): `set_ptr32` of the object and of its work area, the
    /// key's colour scaled by the part ([`colour`]), `g_pppScreenMatrix`,
    /// the key's size (`+0xc`), and the part's position (`+0x60`); `Crn`
    /// and `Fls` also the key's two halves `+0x10`, `+0x12` before the
    /// position. Returns the drawer's eax.
    pub fn ppp_ke_lns_draw<M: Mem>(
        mem: &mut M,
        kind: Lns,
        obj: u32,
        key: u32,
        prog: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let l = kind.layout();
        let part: u32 = mem.read(g(mem, CURRENT_PART));
        call(mem, g(mem, l.update), &[obj, key, prog]);
        let d = frame.wrapping_sub(0x30);
        let v = set_ptr32(mem, obj, call);
        mem.write(d, v);
        colour(mem, key, at(d, 8));
        mem.write(at(d, 0x10), copy(mem, at(key, 0xc)));
        if kind != Lns::Arnd {
            mem.write(at(d, 0x14), mem.read::<u16>(at(key, 0x10)));
            mem.write(at(d, 0x16), mem.read::<u16>(at(key, 0x12)));
        }
        let v = set_ptr32(mem, g(mem, SCREEN_MATRIX), call);
        mem.write(at(d, 0xc), v);
        let area = at(obj, 0xa0).wrapping_add(mem.read(mem.read::<u32>(at(prog, 0xc))));
        if kind == Lns::Arnd {
            let v = set_ptr32(mem, at(part, 0x60), call);
            mem.write(at(d, 0x14), v);
            let v = set_ptr32(mem, area, call);
            mem.write(at(d, 4), v);
        } else {
            let v = set_ptr32(mem, area, call);
            mem.write(at(d, 4), v);
            let v = set_ptr32(mem, at(part, 0x60), call);
            mem.write(at(d, 0x18), v);
        }
        call(mem, g(mem, l.draw), &[d, 0])
    }

    /// `pppKeLnsArndTDraw` (`0x0075a190`), `pppKeLnsCrnTDraw`
    /// (`0x0075a900`) and `pppKeLnsFlsTDraw` (`0x0075ad00`) `(obj, key,
    /// prog)`: the kind's update; on the key's frame the key's size into
    /// the work area (`Crn`: also its halves `+0x10`, `+0x12` as floats at
    /// `+0x34`, `+0x38`; `Fls`: `+0x10` as a float at `+0x38`, `+0x12` at
    /// `+0x3c`); then the scaled colour ([`colour`]) and the part's
    /// position (`FFXVu0CopyVector`, whose eax it returns). The drawing
    /// itself is the `TOt` draw on `g_pppOtList`.
    pub fn ppp_ke_lns_t_draw<M: Mem>(
        mem: &mut M,
        kind: Lns,
        obj: u32,
        key: u32,
        prog: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let l = kind.layout();
        let off: u32 = mem.read(mem.read::<u32>(at(prog, 0xc)));
        let part: u32 = mem.read(g(mem, CURRENT_PART));
        call(mem, g(mem, l.update), &[obj, key, prog]);
        let a = at(obj, off).wrapping_add(0xa0);
        if mem.read::<u32>(key) == mem.read::<u32>(at(obj, 0xc)) {
            mem.write(at(a, l.size), copy(mem, at(key, 0xc)));
            let half = |mem: &M, o| store(f64::from(mem.read::<u16>(at(key, o))));
            match kind {
                Lns::Arnd => {}
                Lns::Crn => {
                    mem.write(at(a, 0x34), half(mem, 0x10));
                    mem.write(at(a, 0x38), half(mem, 0x12));
                }
                Lns::Fls => {
                    mem.write(at(a, 0x38), half(mem, 0x10));
                    mem.write(at(a, 0x3c), mem.read::<u16>(at(key, 0x12)));
                }
            }
        }
        colour(mem, key, at(a, l.colour));
        call(mem, g(mem, COPY_VECTOR), &[at(a, l.pos), at(part, 0x60)])
    }

    /// `pppKeLnsArndTOtDraw` (`0x0075a250`), `pppKeLnsCrnTOtDraw`
    /// (`0x0075a9e0`) and `pppKeLnsFlsTOtDraw` (`0x0075ade0`) `(area)`:
    /// the `g_pppOtList` draw the `TCon` registers. A descriptor at
    /// `frame - 0x30` built from the work area (`set_ptr32` of the area twice,
    /// its colour word, `g_pppScreenMatrix`, its size, its position; `Crn`
    /// the floats `+0x34`, `+0x38` truncated to halves, `Fls` `+0x38`
    /// truncated and `+0x3c`) goes to the kind's drawer with flags 4.
    /// Returns the drawer's eax.
    pub fn ppp_ke_lns_t_ot_draw<M: Mem>(
        mem: &mut M,
        kind: Lns,
        area: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let l = kind.layout();
        let d = frame.wrapping_sub(0x30);
        let v = set_ptr32(mem, area, call);
        mem.write(d, v);
        mem.write(at(d, 0x10), copy(mem, at(area, l.size)));
        mem.write(at(d, 8), mem.read::<u32>(at(area, l.colour)));
        match kind {
            Lns::Arnd => {}
            Lns::Crn => {
                mem.write(at(d, 0x14), chop16(mem, at(area, 0x34)));
                mem.write(at(d, 0x16), chop16(mem, at(area, 0x38)));
            }
            Lns::Fls => {
                mem.write(at(d, 0x14), chop16(mem, at(area, 0x38)));
                mem.write(at(d, 0x16), mem.read::<u16>(at(area, 0x3c)));
            }
        }
        let v = set_ptr32(mem, g(mem, SCREEN_MATRIX), call);
        mem.write(at(d, 0xc), v);
        let v = set_ptr32(mem, area, call);
        mem.write(at(d, 4), v);
        let v = set_ptr32(mem, at(area, l.pos), call);
        mem.write(at(d, if kind == Lns::Arnd { 0x14 } else { 0x18 }), v);
        call(mem, g(mem, l.draw), &[d, 4])
    }

    /// `pppKeLnsLpDraw(obj, key, prog)` (`0x0075af80`): in the work area
    /// `w = obj + 0xa0 + [[prog + 0xc]]`, on the key's frame its `+8` is
    /// added to `+0xa8` and its `+4` to `+0x9c`, its bytes `+0xc`, `+0xd`
    /// stored at `+0x8e`, `+0x8f`. While `+0x8f` is set, `+0x90` gets the
    /// x, y, z of the object's matrix `+0x10` applied to `(0, 0, 1, 1)` (at
    /// `frame - 0x14`). The object's position (`+0x40`) is copied to
    /// `+0x80`, and `KeLnsLp_Update` gets a descriptor at `frame - 0x30`:
    /// `set_ptr32(w)`, the position, `g_pppViewMatrix`,
    /// `g_pppScreenMatrix`. Returns its eax.
    pub fn ppp_ke_lns_lp_draw<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        prog: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let w = at(obj, 0xa0).wrapping_add(mem.read(mem.read::<u32>(at(prog, 0xc))));
        let f = |mem: &M, a| load(mem.read(a));
        if mem.read::<u32>(key) == mem.read::<u32>(at(obj, 0xc)) {
            mem.write(
                at(w, 0xa8),
                store(add(f(mem, at(key, 8)), f(mem, at(w, 0xa8)))),
            );
            mem.write(at(w, 0x8e), mem.read::<u8>(at(key, 0xc)));
            mem.write(
                at(w, 0x9c),
                store(add(f(mem, at(key, 4)), f(mem, at(w, 0x9c)))),
            );
            mem.write(at(w, 0x8f), mem.read::<u8>(at(key, 0xd)));
        }
        if mem.read::<u8>(at(w, 0x8f)) != 0 {
            let v = frame.wrapping_sub(0x14);
            let one = 1.0f32.to_bits();
            for (o, x) in [(4u32, 0u32), (0, 0), (8, one), (0xc, one)] {
                mem.write(at(v, o), x);
            }
            call(mem, g(mem, APPLY_MATRIX), &[v, at(obj, 0x10), v]);
            call(mem, g(mem, COPY_VEC_XYZ), &[at(w, 0x90), v]);
        }
        for o in [0u32, 4, 8] {
            mem.write(at(w, 0x80 + o), copy(mem, at(obj, 0x40 + o)));
        }
        let d = frame.wrapping_sub(0x30);
        mem.write(at(d, 4), at(obj, 0x40));
        mem.write(at(d, 8), g(mem, VIEW_MATRIX));
        mem.write(at(d, 0xc), g(mem, SCREEN_MATRIX));
        let v = set_ptr32(mem, w, call);
        mem.write(d, v);
        call(mem, g(mem, LP_UPDATE), &[d])
    }
}
