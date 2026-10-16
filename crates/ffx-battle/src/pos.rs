//! Battle positions (`docs/rewrite-plan.md`, M2): the battle map's
//! position file (`*0x0112a9b0`) that `MsBtlGetPos` reads and writes.
//!
//! The file starts with a flag byte (set: one record of 0x20 bytes per
//! group, only kind 3 readable; clear: three records of 0x20 per group)
//! and an `i8` group count. A record has `i8` counts at `+2`-`+6` and `+9`,
//! file offsets of position tables at `+0x10`-`+0x24` and `+0x2c`, and its
//! own vectors at `+0x30`, `+0x40` and `+0x50`. Positions are vec4 (16
//! bytes); kind 11 is a table of single floats.

/// The game's memory, through [`ffx_bridge::Mem`].
#[allow(clippy::many_single_char_names)] // file, record, pointer, value
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    /// The position file.
    pub const POS_FILE: Global<u32> = Global::new(0x0112_a9b0);

    fn i8_at(mem: &impl Mem, p: u32, o: u32) -> i32 {
        i32::from(mem.read::<i8>(p.wrapping_add(o)))
    }

    fn quiet(v: u32) -> u32 {
        ffx_bridge::x87::store(ffx_bridge::x87::load(v))
    }

    /// Where `MsBtlGetPos` finds a position.
    enum Loc {
        /// -1: a bad group, kind or index.
        Bad,
        /// The actor's own (`+0x3b0`), in a defense position.
        Own,
        /// A float of kind 11.
        Float(u32),
        /// Kinds 12-14: a summoner's position.
        Summoner,
        /// A vec4, and what the function returns.
        Vec(u32, i32),
    }

    fn locate(mem: &impl Mem, actor: u32, group: i32, kind: i32, index: i32, vec: bool) -> Loc {
        let file = POS_FILE.get(mem);
        if group < 0 || group >= i8_at(mem, file, 1) {
            return Loc::Bad;
        }
        let rec = if mem.read::<u8>(file) != 0 {
            if kind != 3 {
                return Loc::Bad;
            }
            file.wrapping_add(group.cast_unsigned() * 0x20)
        } else {
            file.wrapping_add(group.cast_unsigned() * 0x60)
        };
        if actor != 0 && vec && mem.read::<u8>(actor.wrapping_add(0x6d4)) != 0xff {
            return Loc::Own;
        }
        let table = |count: u32, at: u32| -> Option<u32> {
            if index < 0 || index >= i8_at(mem, rec, count) {
                return None;
            }
            let o: u32 = mem.read(rec.wrapping_add(at));
            Some(
                o.wrapping_add(index.cast_unsigned() * 16)
                    .wrapping_add(file),
            )
        };
        let mut ret = 0;
        let p = match kind {
            2 => Some(rec.wrapping_add(0x30)),
            3 => table(4, 0x10),
            6 => table(4, 0x14),
            4 => table(5, 0x18),
            7 => table(5, 0x1c),
            5 => table(6, 0x20),
            8 => table(6, 0x24),
            9 => {
                ret = i8_at(mem, rec, 2);
                Some(rec.wrapping_add(0x40))
            }
            10 => {
                ret = i8_at(mem, rec, 3);
                Some(rec.wrapping_add(0x50))
            }
            11 => {
                if index < 0 || index >= i8_at(mem, rec, 9) {
                    return Loc::Bad;
                }
                let o: u32 = mem.read(rec.wrapping_add(0x2c));
                let f = o.wrapping_add(index.cast_unsigned() * 4).wrapping_add(file);
                return if f == 0 { Loc::Bad } else { Loc::Float(f) };
            }
            12..=14 => return Loc::Summoner,
            _ => None,
        };
        p.map_or(Loc::Bad, |p| Loc::Vec(p, ret))
    }

    /// The actor's own position (`+0x3b0`), through `EE_MOVE`.
    fn own(mem: &mut impl Mem, actor: u32) -> [u32; 4] {
        let v: [u32; 4] = mem.read(actor.wrapping_add(0x3b0));
        crate::camera::bridge::EE_MOVE.set(mem, v);
        v
    }

    /// `MsBtlGetPos(write, actor, group, kind, index, vec)` (`0x007abfe0`):
    /// reads (or with `write` writes) position `index` of kind `kind` of
    /// `group` through `vec` (floats through the x87). An actor in a
    /// defense position (`+0x6d4` not 0xff) reads its own (`+0x3b0`) instead.
    /// Kinds: 2, 9, 10 the record's own vectors (9 and 10 return its
    /// counts at `+2`, `+3`); 3-8 the tables (counts `+4`, `+5`, `+6`); 11 a
    /// float; 12-14 a summoner's position (`MsGetSummonerPos(actor, index,
    /// k, vec)`, from the caller: [`ms_get_summoner_pos`]). -1 for a bad
    /// group, kind or index.
    #[allow(clippy::too_many_arguments)]
    pub fn ms_btl_get_pos<M: Mem>(
        mem: &mut M,
        write: u32,
        actor: u32,
        group: i32,
        kind: i32,
        index: i32,
        vec: u32,
        summoner: impl FnOnce(&mut M, u32, i32, i32, u32),
    ) -> i32 {
        match locate(mem, actor, group, kind, index, vec != 0) {
            Loc::Bad => -1,
            Loc::Own => {
                let v = own(mem, actor);
                mem.write(vec, v);
                0
            }
            Loc::Float(f) => {
                if vec != 0 {
                    if write != 0 {
                        let v = quiet(mem.read(vec));
                        mem.write(f, v);
                    } else {
                        let v = quiet(mem.read(f));
                        mem.write(vec, v);
                    }
                }
                0
            }
            Loc::Summoner => {
                if vec != 0 {
                    summoner(mem, actor, index, kind - 12, vec);
                }
                0
            }
            Loc::Vec(p, ret) => {
                let here: [u32; 4] = mem.read(p);
                if vec == 0 {
                    return ret;
                }
                if write != 0 {
                    let v: [u32; 4] = mem.read(vec);
                    mem.write(p, v.map(quiet));
                } else {
                    mem.write(vec, here.map(quiet));
                }
                ret
            }
        }
    }

    /// `MsBtlGetPos(0, actor, group, kind, index, &v)` on a value: the
    /// position read into `v` (left alone on -1).
    fn read_pos<M: Mem>(
        mem: &mut M,
        trig: &dyn Fn(Trig, u32) -> u32,
        actor: u32,
        [group, kind, index]: [i32; 3],
        v: &mut [u32; 4],
    ) -> i32 {
        match locate(mem, actor, group, kind, index, true) {
            Loc::Bad => -1,
            Loc::Own => {
                *v = own(mem, actor);
                0
            }
            Loc::Float(f) => {
                v[0] = quiet(mem.read(f));
                0
            }
            Loc::Summoner => {
                summoner_pos(mem, trig, actor, index.cast_unsigned(), kind - 12, v);
                0
            }
            Loc::Vec(p, ret) => {
                let here: [u32; 4] = mem.read(p);
                *v = here.map(quiet);
                ret
            }
        }
    }

    /// An actor's own position arguments (`+0x4fc`-`+0x4fe`, signed).
    fn place(mem: &impl Mem, a: u32) -> [i32; 3] {
        [0x4fc, 0x4fd, 0x4fe].map(|o| i8_at(mem, a, o))
    }

    /// A C runtime function of one float the battle code calls through
    /// the x87 (`_CIcos`, `_CIsin`, `_CIatan`): the caller supplies it (in
    /// the game, MSVCR110's own) taking the argument as `fld dword` loads
    /// it (`f32` bits) and returning what `fstp dword` stores.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Trig {
        /// `_CIcos` (thunk `0x009498c2`).
        Cos,
        /// `_CIsin` (thunk `0x009498c8`).
        Sin,
        /// `_CIatan` (thunk `0x00949d5c`).
        Atan,
    }

    /// `MsGetSummonerPos`'s angle offsets (`f64`) and radii (`f32`).
    pub const QUARTER: Global<f64> = Global::new(0x00b5_4f30);

    /// `MsGetSummonerPos(actor, chr, k, vec)` (`0x007ac2b0`): where an aeon
    /// (`chr`, ids 0x3001-0x300b at `+4`) stands for `k` (0 its own
    /// position, kinds 12-14 of `MsBtlGetPos` giving 0-2): its position
    /// (`r0`, then its own `MsBtlGetPos`), moved by a radius along its
    /// direction (`+0x514[+0x511]` less pi/4) turned by an angle per aeon
    /// (and for 0x3007 per `k`; 0x3005 never moves). The Magus Sisters
    /// (0x3009-0x300b) take the middle of actors 0xf-0x11 and their mean
    /// direction (stored at `actor+0x518`). The move is `x += r(cos - sin)`,
    /// `z += r(sin + cos)` with the C runtime's `_CIcos` and `_CIsin`
    /// (`trig`, from the caller).
    ///
    /// The original builds the result in `vec` itself; this writes it once
    /// at the end. The sisters' `MsDivVector` leaves its caller's saved
    /// `edi`, `esi`, `ebx` in `VF_B`'s last three words (this leaves them
    /// as they were), and a sister whose position fails adds the original's
    /// stack garbage (here the previous sister's position, zeros at first).
    pub fn ms_get_summoner_pos<M: Mem>(
        mem: &mut M,
        trig: &dyn Fn(Trig, u32) -> u32,
        actor: u32,
        chr: u32,
        k: u32,
        vec: u32,
    ) {
        let mut v = [0; 4];
        summoner_pos(mem, trig, actor, chr, k.cast_signed(), &mut v);
        mem.write(vec, v);
    }

    #[allow(clippy::too_many_lines)]
    fn summoner_pos<M: Mem>(
        mem: &mut M,
        trig: &dyn Fn(Trig, u32) -> u32,
        actor: u32,
        chr: u32,
        k: i32,
        v: &mut [u32; 4],
    ) {
        use crate::actor::bridge::ms_get_chr;
        use crate::vector::bridge::{add_point, div_vector, ms_near_rag};
        use ffx_bridge::x87::{add, div, load, store, sub};
        let f64_at = |mem: &M, at: u32| Global::<f64>::new(at).get(mem);
        let f32_g = |mem: &M, at: u32| Global::<u32>::new(at).get(mem);
        let direction = |mem: &M, a: u32| -> u32 {
            let i = i8_at(mem, a, 0x511).cast_unsigned();
            mem.read(a.wrapping_add(0x514).wrapping_add(i.wrapping_mul(4)))
        };
        let r0 = crate::camera::bridge::EE_R0.get(mem);
        *v = r0;
        let a = ms_get_chr(mem, chr);
        let at = place(mem, a);
        read_pos(mem, trig, a, at, v);
        let mut ang = store(sub(load(direction(mem, a)), QUARTER.get(mem)));
        let id: u32 = mem.read(a.wrapping_add(4));
        // (radius, angle offset, added) per aeon
        let turn = |r: u32, o: u32, plus: bool| (r, o, plus);
        let (r, o, plus) = match id.wrapping_sub(0x3001) {
            0 => turn(0x00b9_2318, 0x00b5_4f70, true),
            1 => turn(0x00b9_231c, 0x00b5_4f50, false),
            2 => turn(0x00b5_01b8, 0x00b5_4f48, true),
            3 => turn(0x00b5_4f84, 0x00b5_4f60, true),
            5 => turn(0x00b4_a64c, 0x00b5_4f58, false),
            6 => turn(0x00b5_4f90, 0x00b5_4f68, true),
            7 => match k {
                1 => turn(0x00b5_4f8c, 0x00b5_4f40, false),
                2 => turn(0x00b5_4f88, 0x00b5_4f38, true),
                3 => turn(0x00b5_4f80, 0x00b5_4f70, true),
                _ => return,
            },
            8..=10 => {
                *v = r0;
                let mut first = 0u32;
                let mut sum = 0u32;
                let mut tmp = [0u32; 4];
                for j in 0..3 {
                    let c = ms_get_chr(mem, j + 0xf);
                    let at = place(mem, c);
                    read_pos(mem, trig, c, at, &mut tmp);
                    *v = add_point(mem, *v, tmp);
                    let d = store(load(direction(mem, c)));
                    if j == 0 {
                        first = ms_near_rag(mem, 0, d).to_bits();
                        sum = first;
                    } else {
                        let n = ms_near_rag(mem, first, d).to_bits();
                        sum = store(add(load(n), load(sum)));
                    }
                }
                let mut b = crate::vector::bridge::VF_B.get(mem);
                b[0] = f32_g(mem, 0x00b3_eb94);
                *v = div_vector(mem, *v, b);
                sum = store(div(load(sum), f64_at(mem, 0x00b2_c5c8)));
                mem.write(actor.wrapping_add(0x518), sum);
                ang = store(sub(load(sum), f64_at(mem, 0x00b5_4f78)));
                if k == 0 {
                    return;
                }
                move_by(trig, v, ang, f32_g(mem, 0x00b9_2318));
                return;
            }
            _ => return,
        };
        if id != 0x3008 && k == 0 {
            return;
        }
        let off = f64_at(mem, o);
        ang = store(if plus {
            add(load(ang), off)
        } else {
            sub(load(ang), off)
        });
        move_by(trig, v, ang, f32_g(mem, r));
    }

    /// `x += r(cos - sin)`, `z += r(sin + cos)` in the original's order.
    fn move_by(trig: &dyn Fn(Trig, u32) -> u32, v: &mut [u32; 4], ang: u32, r: u32) {
        use ffx_bridge::x87::{add, load, mul, store, sub};
        let c = load(trig(Trig::Cos, ang));
        let s = load(trig(Trig::Sin, ang));
        let r = load(r);
        v[0] = store(add(mul(sub(c, s), r), load(v[0])));
        v[2] = store(add(mul(add(c, s), r), load(v[2])));
    }

    /// `MsGetChrBtlPos(i, vec)` (`0x007948e0`): actor `i`'s battle
    /// position (its own `MsBtlGetPos`, summoners' through
    /// [`ms_get_summoner_pos`]), lifted by its height (`+0x534`) unless it
    /// stands on the ground (`+0x608` 0 and alive or `+0xde7`, and `+0x50d`
    /// 0). With `+0x6dd` its x and z are its current position
    /// ([`ms_get_chr_pos`]), keeping that y. -1, or 0 for a non-actor.
    pub fn ms_get_chr_btl_pos<M: Mem>(
        mem: &mut M,
        trig: &dyn Fn(Trig, u32) -> u32,
        i: u32,
        vec: u32,
    ) -> i32 {
        use crate::actor::bridge::{ms_btl_chr_num_check, ms_get_chr};
        use ffx_bridge::x87::{add, load, store};
        let i = i & 0xff;
        if ms_btl_chr_num_check(i) == 0 {
            return 0;
        }
        let a = ms_get_chr(mem, i);
        let [group, kind, index] = place(mem, a);
        ms_btl_get_pos(
            mem,
            0,
            a,
            group,
            kind,
            index,
            vec,
            |m, actor, index, k, vec| {
                ms_get_summoner_pos(
                    m,
                    trig,
                    actor,
                    index.cast_unsigned(),
                    k.cast_unsigned(),
                    vec,
                );
            },
        );
        let b = |mem: &M, o: u32| mem.read::<u8>(a.wrapping_add(o)) != 0;
        let lift = if b(mem, 0x608) {
            b(mem, 0x50d)
        } else {
            !b(mem, 0xdcc) || b(mem, 0xde7) || b(mem, 0x50d)
        };
        let y = vec.wrapping_add(4);
        if lift {
            let h: u32 = mem.read(a.wrapping_add(0x534));
            let v: u32 = mem.read(y);
            mem.write(y, store(add(load(h), load(v))));
        }
        if b(mem, 0x6dd) {
            let keep = quiet(mem.read(y));
            ms_get_chr_pos(mem, trig, i, vec);
            mem.write(y, keep);
        }
        -1
    }

    /// `MsGetChrPos(i, vec)` (`0x007956e0`): actor `i`'s current position
    /// ([`ms_get_chr_pos_of`]); 0.
    pub fn ms_get_chr_pos<M: Mem>(
        mem: &mut M,
        trig: &dyn Fn(Trig, u32) -> u32,
        i: u32,
        vec: u32,
    ) -> i32 {
        use crate::actor::bridge::{ms_btl_chr_num_check, ms_get_chr};
        let i = i & 0xff;
        if ms_btl_chr_num_check(i) != 0 {
            let a = ms_get_chr(mem, i);
            ms_get_chr_pos_of(mem, trig, a, vec);
        }
        0
    }

    /// `msGetChrPos(actor, vec)` (`0x00796ea0`): an actor's position with
    /// w = 1: the optional position `+0x441` of the character of actor
    /// `+0x43f` (one it rides or is attached to) when that has one, else
    /// its own character's (`+0`) position, else [`ms_get_chr_pos_no_chr`].
    pub fn ms_get_chr_pos_of<M: Mem>(
        mem: &mut M,
        trig: &dyn Fn(Trig, u32) -> u32,
        actor: u32,
        vec: u32,
    ) {
        use crate::actor::bridge::ms_get_chr_handle;
        let chr: u32 = mem.read(actor);
        let on = mem.read::<u8>(actor.wrapping_add(0x43f));
        let h = if on == 0xff {
            0
        } else {
            ms_get_chr_handle(mem, u32::from(on))
        };
        if h != 0 {
            let i = mem.read::<u8>(actor.wrapping_add(0x441));
            ffx_chara::attach::bridge::ch_optpos_get(mem, h, u32::from(i), vec);
        } else if chr != 0 {
            ffx_chara::chr::bridge::ch_get_pos(
                mem,
                chr,
                vec,
                vec.wrapping_add(4),
                vec.wrapping_add(8),
            );
        } else {
            ms_get_chr_pos_no_chr(mem, trig, actor, vec);
        }
        mem.write(vec.wrapping_add(12), 1.0f32.to_bits());
    }

    /// `msGetChrPosNoChr(actor, vec)` (`0x00795ac0`): an actor's position
    /// without a character of its own: the optional position `+0x441` of
    /// actor `+0x43f`'s character (nothing when that has none), or with
    /// `+0x43f` 0xff its battle position ([`ms_get_chr_btl_pos`] of its
    /// index `+0xc`).
    pub fn ms_get_chr_pos_no_chr<M: Mem>(
        mem: &mut M,
        trig: &dyn Fn(Trig, u32) -> u32,
        actor: u32,
        vec: u32,
    ) {
        use crate::actor::bridge::ms_get_chr_handle;
        let on = mem.read::<u8>(actor.wrapping_add(0x43f));
        if on == 0xff {
            let i = mem.read::<u8>(actor.wrapping_add(0xc));
            ms_get_chr_btl_pos(mem, trig, u32::from(i), vec);
            return;
        }
        let h = ms_get_chr_handle(mem, u32::from(on));
        if h != 0 {
            let i = mem.read::<u8>(actor.wrapping_add(0x441));
            ffx_chara::attach::bridge::ch_optpos_get(mem, h, u32::from(i), vec);
        }
    }

    /// [`ms_check_btl_pos`]'s two vectors (on the original's stack).
    pub const CHECK_FRAME: u32 = 0xffa0_0000;

    /// `MsCheckBtlPos(i, actor, flat)` (`0x007a9230`): 1 when actor `i` is
    /// at least `actor`'s `+0x4f0` away from its battle position (in x and
    /// z with `flat`, `MsPointLen2D`; else `MsPointLen`) or the distance is
    /// NaN; 0 when nearer. (For a non-actor, or one whose position calls
    /// leave the vectors alone, the original compares its stack garbage;
    /// here zeros.)
    pub fn ms_check_btl_pos<M: Mem>(
        mem: &mut M,
        trig: &dyn Fn(Trig, u32) -> u32,
        i: u32,
        actor: u32,
        flat: u32,
    ) -> u32 {
        use crate::vector::bridge::{ms_point_len, ms_point_len_2d};
        let limit: u32 = mem.read(actor.wrapping_add(0x4f0));
        let mut f = ffx_bridge::frame::Frame::new(mem, CHECK_FRAME, 0x20);
        let (now, home) = (f.at(0), f.at(0x10));
        ms_get_chr_pos(&mut f, trig, i, now);
        ms_get_chr_btl_pos(&mut f, trig, i, home);
        let d = if flat != 0 {
            ms_point_len_2d(&mut f, now, home)
        } else {
            ms_point_len(&mut f, now, home)
        };
        let d = ffx_bridge::x87::load(d.to_bits());
        let limit = ffx_bridge::x87::load(limit);
        u32::from(limit.partial_cmp(&d) != Some(std::cmp::Ordering::Greater))
    }
}
