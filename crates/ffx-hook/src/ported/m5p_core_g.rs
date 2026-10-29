//! M5 stage 1, list `phyre-core`, batch 7 (`ffx_pcore::m5p_core_g`):
//! `PGameplay`'s spline: tangents, the Hermite point, the segment
//! evaluator and direction, the position and direction along the spline
//! (one generic port for the two copies), in world space, and a vector's
//! length.
//!
//! Threads: wherever gameplay code moves things along splines (main
//! thread). The control points the original callees see live in per-call
//! `StackFrame`s.

use super::{StackFrame, mem, ported};
use ffx_pcore::m5p_core_g::{self as c, bridge as b};

ported! {
    /// `Phyre_PSpline_tangent_00548980(out, &p0, &p1, &p2, tension)` -> `out`.
    Phyre_PSpline_tangent_00548980 @ 0x0054_8980 => extern "C" fn tangent(out: u32, p0: u32, p1: u32, p2: u32, tension: u32) -> u32 {
        b::tangent(&mut mem(), out, p0, p1, p2, tension)
    }
    /// `Phyre_PSpline_hermite_005487d0(out, &p0, &p1, &p2, &p3, t, tension)` -> `out`.
    Phyre_PSpline_hermite_005487d0 @ 0x0054_87d0 => extern "C" fn hermite(out: u32, p0: u32, p1: u32, p2: u32, p3: u32, t: u32, tension: u32) -> u32 {
        b::hermite(&mut mem(), out, p0, p1, p2, p3, t, tension)
    }
    /// `Phyre_PSpline_evaluate_00549920(spline; out, i, t)` -> `out`; the control points in a per-call frame.
    Phyre_PSpline_evaluate_00549920 @ 0x0054_9920 => extern "thiscall" fn evaluate(this: u32, out: u32, i: u32, t: u32) -> u32 {
        let mut frame = StackFrame::<{ (c::EVAL_FRAME / 4) as usize }>::new();
        b::eval(&mut mem(), this, out, i, t, frame.at())
    }
    /// `Phyre_PSpline_direction_00549a60(spline; out, i, t)` -> `out`.
    Phyre_PSpline_direction_00549a60 @ 0x0054_9a60 => extern "thiscall" fn direction(this: u32, out: u32, i: u32, t: u32) -> u32 {
        let mut frame = StackFrame::<{ (c::DIRECTION_FRAME / 4) as usize }>::new();
        b::direction(&mut mem(), this, out, i, t, frame.at())
    }
    /// `Phyre_PSpline_directionAt_0054b710(spline; out, distance)` -> `out`.
    Phyre_PSpline_directionAt_0054b710 @ 0x0054_b710 => extern "thiscall" fn direction_at(this: u32, out: u32, distance: u32) -> u32 {
        let mut frame = StackFrame::<{ (c::AT_FRAME / 4) as usize }>::new();
        b::at(&mut mem(), this, out, distance, c::Along::Direction, frame.at())
    }
    /// `Phyre_PSpline_positionAt_0054b8f0(spline; out, distance)` -> `out`.
    Phyre_PSpline_positionAt_0054b8f0 @ 0x0054_b8f0 => extern "thiscall" fn position_at(this: u32, out: u32, distance: u32) -> u32 {
        let mut frame = StackFrame::<{ (c::AT_FRAME / 4) as usize }>::new();
        b::at(&mut mem(), this, out, distance, c::Along::Position, frame.at())
    }
    /// `Phyre_PSpline_worldDirectionAt_0054b860(spline; out, distance)` -> `out`.
    Phyre_PSpline_worldDirectionAt_0054b860 @ 0x0054_b860 => extern "thiscall" fn world_direction(this: u32, out: u32, distance: u32) -> u32 {
        let mut frame = StackFrame::<0x50>::new();
        b::world_direction(&mut mem(), this, out, distance, frame.at())
    }
    /// `Phyre_PSpline_worldPositionAt_0054ba40(spline; out, distance)` -> `out`.
    Phyre_PSpline_worldPositionAt_0054ba40 @ 0x0054_ba40 => extern "thiscall" fn world_position(this: u32, out: u32, distance: u32) -> u32 {
        let mut frame = StackFrame::<0x50>::new();
        b::world_position(&mut mem(), this, out, distance, frame.at())
    }
    /// `Phyre_Vector3_lengthTo_0054bd60(out, &v)` -> `out`.
    Phyre_Vector3_lengthTo_0054bd60 @ 0x0054_bd60 => extern "C" fn length_of(out: u32, v: u32) -> u32 {
        b::length_of(&mut mem(), out, v)
    }
}
