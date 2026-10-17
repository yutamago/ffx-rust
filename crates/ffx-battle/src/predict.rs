//! Moving the actors each frame (`docs/rewrite-plan.md`, M2): `MsMoveChr`
//! and what it runs per actor, `MsGetChrPredictPos` (the move step), and
//! the move set-up `MsGetMoveTargetDist`.

/// The game's memory, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::Mem;

    use crate::actor::bridge::ms_get_chr;
    use crate::target::bridge::{Dm, Host, Regs, ms_get_move_target_pos};
    use crate::vector::bridge as v;

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
}
