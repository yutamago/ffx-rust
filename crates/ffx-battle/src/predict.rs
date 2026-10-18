//! Moving the actors each frame (`docs/rewrite-plan.md`, M2): `MsMoveChr`
//! and what it runs per actor, `MsGetChrPredictPos` (the move step), and
//! the move set-up `MsGetMoveTargetDist`.

/// The game's memory, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    use crate::actor::bridge::ms_get_chr;
    use crate::target::bridge::{Dm, Host, Regs, ms_get_move_target_pos};
    use crate::vector::bridge as v;

    /// pi/2 and -pi/2 as `f32` in doubles (`0x00b11338`, `0x00b3ebb0`):
    /// the spline move's turn limits.
    pub const HALF_PI: Global<f64> = Global::new(0x00b1_1338);
    pub const MINUS_HALF_PI: Global<f64> = Global::new(0x00b3_ebb0);
    /// `MsGetChrInfo` kind 0xd of the ball thrower's targets
    /// (`MsSetBallSplineInit`), added to the ball's end (`0x01136ac4`).
    pub const BALL_OFFSET: Global<[u32; 4]> = Global::new(0x0113_6ac4);

    fn byte(mem: &impl Mem, a: u32, o: u32) -> u8 {
        mem.read(a.wrapping_add(o))
    }

    fn quiet(bits: u32) -> u32 {
        ffx_bridge::x87::store(ffx_bridge::x87::load(bits))
    }

    fn f32_at(mem: &impl Mem, a: u32, o: u32) -> u32 {
        quiet(mem.read(a.wrapping_add(o)))
    }

    /// `MsCheckNeckMotion(actor)` (`0x007ab700`): 0 when the actor's motion
    /// (`+0x3f0`) is 0x20-0x24 or 0x48-0x4c (no neck target then,
    /// `MsMoveChr`), else -1.
    pub fn ms_check_neck_motion(mem: &impl Mem, actor: u32) -> u32 {
        match byte(mem, actor, 0x3f0) {
            0x20..=0x24 | 0x48..=0x4c => 0,
            _ => u32::MAX,
        }
    }

    /// `MsGetMoveTargetDist(i, kind, with, speed, n, accel, brake, a, b,
    /// c, gravity)` (`0x007ab540`): starts actor `i`'s move: frame
    /// `+0x452` 0, kind `+0x415` (`kind & 0x3f`), `+0x417` 0, `n` frames
    /// (`+0x45a`) split into accelerating `n * accel / 128` (`+0x454`),
    /// braking `n * brake / 128` (`+0x458`) and the rest cruising
    /// (`+0x456`); the start speed `+0x45c` (`speed`, or `+0x460` with
    /// `kind` bit 0x40); `with` (`+0x416`), `a`, `b`, `c` (`+0x488`,
    /// `+0x48c`, `+0x498`); the start position `+0x390`; the gravity
    /// `+0x4a8` (+0.0 for a zero, `+0x419` 0, else 0xff); the distance to
    /// [`ms_get_move_target_pos`]'s point (`+0x4a4`, 2D with `+0x34`) and
    /// `+0x4a0` 0. `ebx` is the caller's (see `ffx_battle::target`).
    #[allow(clippy::too_many_arguments, clippy::many_single_char_names)]
    pub fn ms_get_move_target_dist(
        mem: &mut Dm<'_>,
        host: &Host<'_>,
        i: u32,
        kind: u32,
        with: u32,
        speed: u32,
        n: u32,
        accel: u32,
        brake: u32,
        abc: [u32; 3],
        gravity: u32,
        ebx: Option<u32>,
    ) {
        use ffx_bridge::x87::load;
        let a = ms_get_chr(mem, i);
        let at = |o: u32| a.wrapping_add(o);
        mem.write(at(0x452), 0u16);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // the low bytes
        {
            mem.write(at(0x415), (kind & 0x3f) as u8);
            let n = n.cast_signed();
            let acc = n.wrapping_mul(accel.cast_signed()) / 128;
            let brk = n.wrapping_mul(brake.cast_signed()) / 128;
            mem.write(at(0x417), 0u8);
            mem.write(at(0x454), acc as u16);
            mem.write(at(0x458), brk as u16);
            mem.write(at(0x456), n.wrapping_sub(brk).wrapping_sub(acc) as u16);
            let s = if kind & 0x40 != 0 {
                f32_at(mem, a, 0x460)
            } else {
                quiet(speed)
            };
            mem.write(at(0x45c), s);
            mem.write(at(0x416), with as u8);
            mem.write(at(0x488), quiet(abc[0]));
            mem.write(at(0x45a), n as u16);
        }
        mem.write(at(0x48c), quiet(abc[1]));
        mem.write(at(0x498), quiet(abc[2]));
        mem.frame(0x20, |mem, f| {
            let (to, pos) = (f, f + 0x10);
            crate::pos::bridge::ms_get_chr_pos(mem, host.trig, i, pos);
            for k in 0..3 {
                let w = quiet(mem.read(pos + 4 * k));
                mem.write(at(0x390 + 4 * k), w);
            }
            if load(gravity) == 0.0 {
                mem.write(at(0x419), 0u8);
                mem.write(at(0x4a8), 0u32);
            } else {
                mem.write(at(0x419), 0xffu8);
                mem.write(at(0x4a8), quiet(gravity));
            }
            let regs = ebx.map(|ebx| Regs {
                edi: a,
                esi: i,
                ebx,
            });
            let k = u32::from(byte(mem, a, 0x415));
            ms_get_move_target_pos(mem, host, i, a, k, to, 0, regs);
            v::ms_sub_point(mem, pos, to, pos);
            let d = if byte(mem, a, 0x34) == 0 {
                v::ms_vector_len(mem, pos)
            } else {
                v::ms_vector_len_2d(mem, pos)
            };
            mem.write(at(0x4a4), quiet(d.to_bits()));
            mem.write(at(0x4a0), 0u32);
        });
    }
    /// `fld lim; fcomp d` then `fld -lim; fcom d` as the move step clamps
    /// a turn `d` to `±lim` (both `f32` bits): `lim` when `lim < d`,
    /// `-lim` (stored) when `-lim > d`, NaN comparisons leaving `d`.
    fn clamp(d: u32, lim: u32) -> u32 {
        use ffx_bridge::x87::{load, store};
        let d = if load(lim) < load(d) { lim } else { d };
        let n = store(-load(lim));
        if load(n) > load(d) { n } else { d }
    }

    /// `d = MsNearRag(now, to) - now`, clamped to `±lim`, added to `now`
    /// (all `f32` bits, `now` re-read): the turn towards `to`; `to` gets
    /// the near angle.
    fn turn(mem: &mut Dm<'_>, a: u32, now: u32, to: u32, lim: u32) -> u32 {
        use ffx_bridge::x87::{add, load, store, sub};
        let n = v::ms_near_rag(mem, f32_at(mem, a, now), quiet(mem.read(to))).to_bits();
        mem.write(to, n);
        let d = store(sub(load(n), load(f32_at(mem, a, now))));
        let d = clamp(d, f32_at(mem, a, lim));
        store(add(load(d), load(f32_at(mem, a, now))))
    }

    /// `MsGetChrPredictPos(i)` (`0x007aa2a0`): one frame of actor `i`'s
    /// move (`+0x415 & 0x7f`; none for 0, or without a character): 7 along
    /// move spline `+0x454` ([`ms_set_ball_spline`] first for spline 2) at
    /// frame `+0x452` of `+0x45a`, facing along its tangent with
    /// `+0x46c`; 0x10 a straight step (`+0x470` yaw, `+0x47c` pitch,
    /// `+0x460` speed); else towards [`ms_get_move_target_pos`]'s point
    /// (actor 0's also into `EE_MOVE` and `g_BtlMoveTarget`), with bit
    /// 0x80 steering (speed `+0x460` up by `+0x468` to `+0x464`, yaw and
    /// pitch turned at most `+0x474`/`+0x480`, which grow by
    /// `+0x478`/`+0x484`; past twice the length a new move,
    /// [`ms_get_move_target_dist`]) or accelerated
    /// (`MsGetChrMoveAccelPos`); with `+0x419` the height follows the
    /// gravity `+0x4a8` from `+0x394`. The move ends (`+0x415`, `+0x418`
    /// 0) at its length. Then, while `+0x42e` counts down, the direction
    /// eases (`MsGetDirMode` into `+0x4f4`, `MsCameraMoveCalc` curve 2,
    /// `Ch_SetDirF`). `regs` are the caller's `edi`, `esi`, `ebx` (the
    /// radius words of `MsVectorPolar2Rect`); `None` leaves those words as
    /// they were. Frame-local vectors the original leaves unwritten are
    /// zero here (the original's are stack garbage).
    #[allow(
        clippy::too_many_lines,
        clippy::many_single_char_names,
        clippy::similar_names
    )]
    pub fn ms_get_chr_predict_pos(mem: &mut Dm<'_>, host: &Host<'_>, i: u32, regs: Option<Regs>) {
        use ffx_bridge::x87::{add, div, load, mul, store, sub};
        let a = ms_get_chr(mem, i);
        let model: u32 = mem.read(a);
        if model == 0 {
            return;
        }
        let at = |o: u32| a.wrapping_add(o);
        let u16_at = |mem: &Dm<'_>, o: u32| mem.read::<u16>(a.wrapping_add(o));
        let set_dir = |mem: &mut Dm<'_>, chr: u32, d: u32| {
            ffx_chara::chr::bridge::ch_set_dir_f(mem, chr, f32::from_bits(d));
        };
        let own_dir = |mem: &mut Dm<'_>, d: u32| {
            let chr: u32 = mem.read(a);
            ffx_chara::chr::bridge::ch_set_dir_f(mem, chr, f32::from_bits(d));
        };
        let end = |mem: &mut Dm<'_>| {
            mem.write(a.wrapping_add(0x415), 0u8);
            mem.write(a.wrapping_add(0x418), 0u8);
        };
        let step = |mem: &mut Dm<'_>| {
            let n = u16_at(mem, 0x452).wrapping_add(1);
            mem.write(a.wrapping_add(0x452), n);
            n
        };
        let polar = |mem: &mut Dm<'_>, out: u32| {
            let r0 = f32_at(mem, a, 0x460);
            let r = match regs {
                Some(r) => [r0, r.edi, r.esi, r.ebx],
                None => {
                    let mut w: [u32; 4] = mem.read(v::VF_M.ptr(mem).addr + 0x40);
                    w[0] = r0;
                    w
                }
            };
            let (yaw, pitch) = (f32_at(mem, a, 0x470), f32_at(mem, a, 0x47c));
            v::ms_vector_polar2_rect(mem, host.trig, out, 0, yaw, pitch, r);
        };
        mem.frame(0x60, |mem, f| {
            let (pos, to, d, mv) = (f, f + 0x10, f + 0x20, f + 0x30);
            let (yaw_to, pitch_to, mode, plain) = (f + 0x40, f + 0x44, f + 0x48, f + 0x4c);
            let kind = u32::from(byte(mem, a, 0x415));
            match kind & 0x7f {
                0 => {}
                7 => {
                    if step(mem) >= u16_at(mem, 0x45a) {
                        end(mem);
                    }
                    if u16_at(mem, 0x454) == 2 {
                        let own = Regs {
                            edi: i,
                            esi: model,
                            ebx: a,
                        };
                        ms_set_ball_spline(mem, host, i, Some(own));
                    }
                    let k = u32::from(u16_at(mem, 0x454));
                    let table = crate::target::bridge::MOVE_SPLINES
                        .ptr(mem)
                        .addr
                        .wrapping_add(k * 0x210);
                    let t = |mem: &Dm<'_>| {
                        let fr = f64::from(u16_at(mem, 0x452));
                        store(div(fr, f64::from(u16_at(mem, 0x45a))))
                    };
                    let tt = t(mem);
                    let count: u32 = mem.read(table);
                    (host.spline)(mem, pos, table + 0x10, count, tt);
                    crate::body::bridge::ms_set_chr_pos(mem, i, pos);
                    let tt = t(mem);
                    let count: u32 = mem.read(table);
                    (host.spline_tangent)(mem, pos, table + 0x10, count, tt);
                    let rate = f32_at(mem, a, 0x46c);
                    if load(rate) != 0.0 {
                        let len = f64::from(v::ms_vector_len(mem, pos));
                        if len != 0.0 {
                            v::ms_vector_rect2_polar(mem, host.trig, pos, 0, yaw_to, 0, 0);
                            let now = f32_at(mem, a, 0x490);
                            let n = v::ms_near_rag(mem, now, quiet(mem.read(yaw_to))).to_bits();
                            mem.write(yaw_to, n);
                            let mut dd = store(sub(load(n), load(f32_at(mem, a, 0x490))));
                            let side = byte(mem, a, 0x42a).cast_signed();
                            let flip = if side < 0 {
                                load(dd) > HALF_PI.get(mem)
                            } else {
                                side > 0 && load(dd) < MINUS_HALF_PI.get(mem)
                            };
                            if flip {
                                dd = store(-load(dd));
                            }
                            let r = store(add(
                                mul(load(dd), load(f32_at(mem, a, 0x46c))),
                                load(f32_at(mem, a, 0x490)),
                            ));
                            mem.write(at(0x490), r);
                            own_dir(mem, r);
                        }
                    }
                }
                0x10 => {
                    crate::pos::bridge::ms_get_chr_pos(mem, host.trig, i, pos);
                    polar(mem, d);
                    v::ms_add_point(mem, pos, pos, d);
                    crate::body::bridge::ms_set_chr_pos(mem, i, pos);
                    if step(mem) >= u16_at(mem, 0x45a) {
                        end(mem);
                    }
                }
                _ => {
                    let own = Regs {
                        edi: i,
                        esi: model,
                        ebx: a,
                    };
                    let dist = quiet(ms_get_move_target_pos(
                        mem,
                        host,
                        i,
                        a,
                        kind,
                        to,
                        0,
                        Some(own),
                    ));
                    if i == 0 {
                        let p: [u32; 4] = mem.read(to);
                        crate::camera::bridge::EE_MOVE.set(mem, p);
                        let mt = crate::misc::bridge::MOVE_TARGET.ptr(mem).addr;
                        mem.write(mt, p);
                    }
                    if byte(mem, a, 0x415) & 0x80 == 0 {
                        let n = step(mem);
                        let own = Regs {
                            edi: i,
                            esi: u32::from(n),
                            ebx: a,
                        };
                        let r = crate::target::bridge::ms_get_chr_move_accel_pos(
                            mem,
                            host.trig,
                            a,
                            u32::from(n),
                            dist,
                            to,
                            pos,
                            Some(own),
                        );
                        if r != 0 {
                            crate::body::bridge::ms_set_chr_pos(mem, i, pos);
                        }
                        if n >= u16_at(mem, 0x45a) {
                            end(mem);
                        }
                    } else {
                        crate::pos::bridge::ms_get_chr_pos(mem, host.trig, i, pos);
                        v::ms_vector_rect2_polar(mem, host.trig, to, pos, yaw_to, pitch_to, 0);
                        let sp = store(add(
                            load(f32_at(mem, a, 0x468)),
                            load(f32_at(mem, a, 0x460)),
                        ));
                        mem.write(at(0x460), sp);
                        let cap = f32_at(mem, a, 0x464);
                        if load(cap) < load(sp) {
                            mem.write(at(0x460), cap);
                        }
                        let lim = store(add(
                            load(f32_at(mem, a, 0x478)),
                            load(f32_at(mem, a, 0x474)),
                        ));
                        mem.write(at(0x474), lim);
                        let yaw = turn(mem, a, 0x470, yaw_to, 0x474);
                        mem.write(at(0x470), yaw);
                        let (fr, len) = (u16_at(mem, 0x452), u16_at(mem, 0x45a));
                        if fr > len {
                            let diff = store(sub(load(mem.read(yaw_to)), load(yaw)));
                            let k = store(f64::from(u32::from(fr) - u32::from(len)));
                            let den = store(f64::from(4 * u32::from(len) + 1));
                            let q = store(div(mul(load(diff), load(k)), load(den)));
                            mem.write(at(0x470), store(add(load(yaw), load(q))));
                        }
                        if byte(mem, a, 0x34) == 0 {
                            let lim = store(add(
                                load(f32_at(mem, a, 0x484)),
                                load(f32_at(mem, a, 0x480)),
                            ));
                            mem.write(at(0x480), lim);
                            let p = turn(mem, a, 0x47c, pitch_to, 0x480);
                            mem.write(at(0x47c), p);
                        }
                        polar(mem, d);
                        v::ms_add_point(mem, pos, pos, d);
                        let rate = f32_at(mem, a, 0x46c);
                        if load(rate) != 0.0 {
                            let yaw = f32_at(mem, a, 0x470);
                            mem.write(yaw_to, yaw);
                            let r = turn(mem, a, 0x490, yaw_to, 0x46c);
                            mem.write(at(0x490), r);
                            own_dir(mem, r);
                        }
                        let n = step(mem);
                        crate::body::bridge::ms_set_chr_pos(mem, i, pos);
                        if u32::from(n) > 2 * u32::from(u16_at(mem, 0x45a)) {
                            let len = if byte(mem, a, 0x34) == 0 {
                                v::ms_point_len(mem, pos, to)
                            } else {
                                v::ms_point_len_2d(mem, pos, to)
                            };
                            let q = div(load(store(f64::from(len))), load(f32_at(mem, a, 0x464)));
                            let frames = ffx_bridge::x87::ftol(mem, q) | 1;
                            let quarter = frames.cast_signed() / 4;
                            crate::target::bridge::ms_set_chr_dir_target(
                                mem,
                                host,
                                i,
                                8,
                                0,
                                0,
                                quarter.cast_unsigned(),
                                1,
                            );
                            let k = u32::from(byte(mem, a, 0x415)) & 0x7f;
                            let with = u32::from(byte(mem, a, 0x416));
                            let speed = f32_at(mem, a, 0x460);
                            let abc = [
                                f32_at(mem, a, 0x488),
                                f32_at(mem, a, 0x48c),
                                f32_at(mem, a, 0x498),
                            ];
                            let g = f32_at(mem, a, 0x4a8);
                            ms_get_move_target_dist(
                                mem,
                                host,
                                i,
                                k,
                                with,
                                speed,
                                frames,
                                0,
                                0x20,
                                abc,
                                g,
                                Some(a),
                            );
                        }
                    }
                    if byte(mem, a, 0x419) != 0 {
                        let half = crate::target::bridge::HALF.get(mem);
                        let len = load(store(f64::from(u16_at(mem, 0x45a))));
                        let fr = i32::from(u16_at(mem, 0x452));
                        let ty: u32 = mem.read(to + 4);
                        let g = load(f32_at(mem, a, 0x4a8));
                        let y0 = load(f32_at(mem, a, 0x394));
                        let m = store(sub(sub(load(ty), y0), mul(mul(mul(g, len), len), half)));
                        let v0 = store(div(load(m), len));
                        crate::pos::bridge::ms_get_chr_pos(mem, host.trig, i, pos);
                        let fr = load(store(f64::from(fr)));
                        let g = load(f32_at(mem, a, 0x4a8));
                        let y0 = load(f32_at(mem, a, 0x394));
                        let y = store(add(
                            add(mul(fr, load(v0)), y0),
                            mul(mul(mul(g, fr), fr), half),
                        ));
                        mem.write(pos + 4, y);
                        crate::body::bridge::ms_set_chr_pos(mem, i, pos);
                    }
                }
            }
            let left = byte(mem, a, 0x42e);
            if left != 0 {
                let (b42f, b430) = (
                    u16::from(byte(mem, a, 0x42f)),
                    u16::from(byte(mem, a, 0x430)),
                );
                let total = b42f.wrapping_mul(2).wrapping_add(b430);
                let words = [
                    2u16,
                    0,
                    0,
                    b42f,
                    b430,
                    b42f,
                    total,
                    total.wrapping_sub(u16::from(left)),
                ];
                for (k, w) in (0u32..).zip(words) {
                    if k != 1 {
                        mem.write(mv + 2 * k, w);
                    }
                }
                mem.write(at(0x42e), left.wrapping_sub(1));
                let r = crate::target::bridge::ms_get_dir_mode(mem, host, i, a, to, 1, mode, plain);
                let r = quiet(r);
                mem.write(yaw_to, r);
                mem.write(at(0x4f4), r);
                if mem.read::<u32>(mode) != 0 {
                    let near: u32 = mem.read(plain);
                    let start = f32_at(mem, a, 0x490);
                    crate::camera::bridge::ms_camera_move_calc(
                        mem, host.cos, near, yaw_to, start, mv,
                    );
                    let dir = quiet(mem.read(yaw_to));
                    set_dir(mem, model, dir);
                }
            }
        });
    }

    /// `MsMoveChr()` (`0x007aab80`): each frame, for each of the 31
    /// actors: one that moves (`+0x10` or `+0x12`) is first bound to the
    /// character it rides (`+0x43f`, element `+0x441`, `Ch_BindToElem`)
    /// and steps its move ([`ms_get_chr_predict_pos`]); then the neck
    /// delay `+0x439` counts down, and at 0 the neck follows actor
    /// `+0x438`'s character (`Ch_NeckSetTarget`, unless
    /// [`ms_check_neck_motion`] says no) or, for 0xff, stops
    /// (`Ch_NeckSetEnable(chr, 0)`). The character is the one read before
    /// the move step, as the original keeps it in a register.
    pub fn ms_move_chr(mem: &mut Dm<'_>, host: &Host<'_>) {
        for i in 0..0x1f {
            let a = ms_get_chr(mem, i);
            let model: u32 = mem.read(a);
            if byte(mem, a, 0x10) != 0 || byte(mem, a, 0x12) != 0 {
                let ride = byte(mem, a, 0x43f);
                if model != 0 && ride != 0xff {
                    let p: u32 = mem.read(ms_get_chr(mem, u32::from(ride)));
                    let elem = u32::from(byte(mem, a, 0x441));
                    ffx_chara::attach::bridge::ch_bind_to_elem(mem, model, p, elem);
                }
                let regs = Regs {
                    edi: i,
                    esi: a,
                    ebx: model,
                };
                ms_get_chr_predict_pos(mem, host, i, Some(regs));
            }
            let delay = byte(mem, a, 0x439);
            if delay == 0 {
                continue;
            }
            let delay = delay - 1;
            mem.write(a.wrapping_add(0x439), delay);
            if delay != 0 {
                continue;
            }
            let t = byte(mem, a, 0x438);
            if t == 0xff {
                (host.neck_enable)(mem, model, 0);
            } else {
                let tm: u32 = mem.read(ms_get_chr(mem, u32::from(t)));
                if model != 0 && tm != 0 && ms_check_neck_motion(mem, a) != 0 {
                    (host.neck_target)(mem, model, tm);
                }
            }
        }
    }

    /// The by-value vector `[x, w1, w2, w3]` a callee reads past its pushed
    /// word: the caller's `edi`, `esi`, `ebx` (`regs`), else the words of
    /// the VU register at `reg` as they were.
    fn spare(mem: &Dm<'_>, x: u32, regs: Option<Regs>, reg: u32) -> [u32; 4] {
        match regs {
            Some(r) => [x, r.edi, r.esi, r.ebx],
            None => {
                let mut w: [u32; 4] = mem.read(reg);
                w[0] = x;
                w
            }
        }
    }

    /// `MsMoveSplineRegist(k, i, kind, point)` (`0x007ab0b0`): appends a
    /// point to move spline `k` (0-2, at most 0x20 points; the count
    /// `g_BtlMoveSplines[k]` first grows, the point going to `+0x10 *
    /// count` of the spline): `point`'s four words when given (also into
    /// `EE_MOVE`), then [`ms_get_move_target_pos`] of actor `i` for `kind`
    /// over it. `ebx` is the caller's (`None`: the register words as they
    /// were).
    pub fn ms_move_spline_regist(
        mem: &mut Dm<'_>,
        host: &Host<'_>,
        k: u32,
        i: u32,
        kind: u32,
        point: u32,
        ebx: Option<u32>,
    ) {
        if k > 2 {
            return;
        }
        let counts = crate::misc::bridge::MOVE_SPLINES.ptr(mem).addr;
        let n: i32 = mem.read(counts + 4 * k);
        if n >= 0x20 {
            return;
        }
        let n = n + 1;
        let at = crate::target::bridge::MOVE_SPLINES
            .ptr(mem)
            .addr
            .wrapping_add(
                (k * 0x21)
                    .wrapping_add(n.cast_unsigned())
                    .wrapping_mul(0x10),
            );
        mem.write(counts + 4 * k, n);
        if point != 0 {
            let p: [u32; 4] = mem.read(point);
            crate::camera::bridge::EE_MOVE.set(mem, p);
            mem.write(at, p);
        }
        let a = ms_get_chr(mem, i);
        let regs = ebx.map(|ebx| Regs {
            edi: at,
            esi: i,
            ebx,
        });
        ms_get_move_target_pos(mem, host, i, a, kind, at, 0, regs);
    }

    /// `MsSetBallSpline(i)` (`0x007b78c0`): rebuilds move spline 2, the
    /// ball's arc for actor `i` towards actor `+0x14` (`t`), by the ball
    /// state `+0x420`. States 0 and 1: from the start `+0x390` (twice)
    /// over seven midpoints (`MsMiddlePoint` at `s` growing by 1.3, less
    /// 0.001 each, and `10 - s`, their heights pulled `k / 32` back towards
    /// the start's for `k` 7 down to 1) to an end (twice): 40 units in
    /// front of `t` when `t`'s first target (`+0xdc0`) is itself and the
    /// state 0; else `MsGetMoveTargetPos` kind 12 plus `0x01136ac4`, which,
    /// when `t`'s result on that target (`MsGetDamageResult`) is 1, is
    /// moved on by the distance returned (`MsVectorNormal2` from the
    /// start); those two ends on the ground (`MsGetChrGround` of `t` or the
    /// target) unless underwater. States 2 and 3: from the start (twice)
    /// over seven midpoints (`s` growing by 1.1, less 0.001 each), raised
    /// by a sum of `v = -1 - len / 3` (`+0x45a`) growing by `-v / 4`, to
    /// `t`'s character's optional position 11 (twice); state 3 within the
    /// last 8 frames of the move. Other states nothing. `regs` are the
    /// caller's `edi`, `esi`, `ebx`, the words past the by-value vectors'
    /// first.
    #[allow(
        clippy::too_many_lines,
        clippy::similar_names,
        clippy::many_single_char_names
    )]
    pub fn ms_set_ball_spline(mem: &mut Dm<'_>, host: &Host<'_>, i: u32, regs: Option<Regs>) {
        use ffx_bridge::x87::{add, load, mul, store, sub};
        let a = ms_get_chr(mem, i);
        let t = u32::from(byte(mem, a, 0x14));
        let ta = ms_get_chr(mem, t);
        crate::misc::bridge::ms_move_spline_set(mem, 2);
        let bits: i32 = mem.read(ta.wrapping_add(0xdc0));
        let first = (0..0x1f).find(|&k| (bits >> k) & 1 != 0);
        let result = match first {
            Some(k) => crate::cue::bridge::ms_get_damage_result(mem, t, k, 0, 0, 0),
            None => 0,
        };
        let state = byte(mem, a, 0x420);
        if state > 3 {
            return;
        }
        let ebx = regs.map(|r| r.ebx);
        let start = a.wrapping_add(0x390);
        let vf = crate::vector::bridge::VF_M.ptr(mem).addr;
        let ten = 10.0f64;
        mem.frame(0x50, |mem, f| {
            let (end, mid, opt, dir) = (f, f + 0x10, f + 0x20, f + 0x30);
            let regist = |mem: &mut Dm<'_>, p: u32| {
                ms_move_spline_regist(mem, host, 2, 0, 0, p, ebx);
            };
            let middle = |mem: &mut Dm<'_>, q: u32, s: u32, tt: u32| {
                let w = spare(mem, tt, regs, vf + 0x30);
                v::ms_middle_point(mem, mid, start, q, [s, w[0], w[1], w[2], w[3]]);
            };
            if state >= 2 {
                regist(mem, start);
                regist(mem, start);
                let model: u32 = mem.read(ta);
                ffx_chara::attach::bridge::ch_optpos_get(mem, model, 0xb, opt);
                let len = i32::from(mem.read::<u16>(a.wrapping_add(0x45a)));
                let mut vv = store(f64::from(-1 - len / 3));
                let dv = store(mul(-load(vv), 0.25));
                let (mut acc, mut acc_y) = (0.0f32.to_bits(), 0.0f32.to_bits());
                let mut step = 1.1f32.to_bits();
                for _ in 0..7 {
                    acc = store(add(load(acc), load(step)));
                    acc_y = store(add(load(acc_y), load(vv)));
                    vv = store(add(load(vv), load(dv)));
                    let tt = store(sub(ten, load(acc)));
                    middle(mem, opt, acc, tt);
                    let y: u32 = mem.read(mid + 4);
                    mem.write(mid + 4, store(add(load(y), load(acc_y))));
                    regist(mem, mid);
                    step = store(sub(load(step), 0.001));
                }
                regist(mem, opt);
                regist(mem, opt);
                let fr = i32::from(mem.read::<u16>(a.wrapping_add(0x452)));
                if len - fr < 8 {
                    mem.write(a.wrapping_add(0x420), 3u8);
                }
                return;
            }
            let ground_of = if first == Some(t) && state == 0 {
                crate::pos::bridge::ms_get_chr_pos(mem, host.trig, t, end);
                crate::body::bridge::ms_get_chr_dir(mem, t, dir);
                let yaw: u32 = mem.read(dir);
                let r = spare(mem, 40.0f32.to_bits(), regs, vf + 0x40);
                v::ms_vector_polar2_rect(mem, host.trig, mid, 0, yaw, 0, r);
                v::ms_add_point(mem, end, end, mid);
                Some(t)
            } else {
                let own = Regs {
                    edi: t,
                    esi: a,
                    ebx: first.unwrap_or(u32::MAX),
                };
                let d = quiet(ms_get_move_target_pos(
                    mem,
                    host,
                    i,
                    a,
                    0xc,
                    end,
                    0,
                    Some(own),
                ));
                let off = BALL_OFFSET.ptr(mem).addr;
                v::ms_add_point(mem, end, end, off);
                if result == 1 {
                    let s = spare(mem, d, regs, vf + 0x20);
                    v::ms_vector_normal2(mem, mid, start, end, s);
                    v::ms_add_point(mem, end, end, mid);
                    first
                } else {
                    None
                }
            };
            if let Some(g) = ground_of
                && crate::state::bridge::UNDERWATER.get(mem) == 0
            {
                let y = crate::body::bridge::ms_get_chr_ground(mem, g);
                mem.write(end + 4, y.to_bits());
            }
            regist(mem, start);
            regist(mem, start);
            let mut acc = 0.0f32.to_bits();
            let mut step = 1.3f32.to_bits();
            for k in (1..=7).rev() {
                acc = store(add(load(acc), load(step)));
                let tt = store(sub(ten, load(acc)));
                middle(mem, end, acc, tt);
                let my = load(mem.read(mid + 4));
                let y0 = load(f32_at(mem, a, 0x394));
                let kf = load(store(f64::from(k)));
                let y = store(add(mul(mul(sub(y0, my), kf), 0.031_25), my));
                mem.write(mid + 4, y);
                regist(mem, mid);
                step = store(sub(load(step), 0.001));
            }
            regist(mem, end);
            regist(mem, end);
        });
    }
}
