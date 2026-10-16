//! Particle effects (the PS2 `ppp` and `op*` effect modules).
//!
//! The `vfx` module of `docs/rewrite-plan.md`: each original function
//! becomes a core (portable, on its own types) where it has logic of its
//! own, and a `bridge` function over game memory (`docs/porting.md`), one
//! `bridge` per area:
//! - [`rand`]: VU0's random register (`op_random`, `op_random_p`);
//! - [`ppp`]: particle programs and rotation matrices;
//! - [`prog`]: the program table's set-ups and steps;
//! - [`mat`]: the matrix programs and their draws;
//! - [`heap`]: the particle heaps;
//! - [`part`]: parts, their data and objects;
//! - [`fp`]: the field's particle effects;
//! - [`op`]: the op layer's services;
//! - [`et`]: its character and effect tables;
//! - [`opus`]: opus effect objects and their script commands;
//! - [`opd`]: the debug display.
//!
//! [`x87`] reproduces the game thread's float arithmetic bit for bit.

pub mod et;
pub mod fp;
pub mod heap;
pub mod mat;
pub mod op;
pub mod opd;
pub mod opus;
pub mod part;
pub mod ppp;
pub mod prog;
pub mod rand;
pub mod x87;
