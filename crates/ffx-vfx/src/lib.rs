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
//! - [`mdl`]: the model draws (`pppDrawMdl*`);
//! - [`mdl_cam`]: the camera-facing and sea model draws;
//! - [`lns`]: the lens-flare draws (`pppKeLns*`);
//! - [`tail`]: the tail draws (`pppKeShpTail*`);
//! - [`heap`]: the particle heaps;
//! - [`part`]: parts, their data and objects;
//! - [`fp`]: the field's particle effects;
//! - [`op`]: the op layer's services;
//! - [`et`]: its character and effect tables;
//! - [`opus`]: opus effect objects and their script commands;
//! - [`anmm`]: `opp_anmm`'s sprite-animation commands;
//! - [`opd`]: the debug display;
//! - [`hi_md`], [`hi_light`], [`hi_draw`], [`hi_opus`], [`hi_0283`]: the
//!   high range's model helpers, vertex lights, model draw set-ups and opus
//!   effects.
//!
//! [`x87`] reproduces the game thread's float arithmetic bit for bit.

pub mod anmm;
pub mod draw;
pub mod et;
pub mod fp;
pub mod heap;
pub mod hi_0283;
pub mod hi_draw;
pub mod hi_light;
pub mod hi_md;
pub mod hi_opus;
pub mod lns;
pub mod mat;
pub mod md;
pub mod mdl;
pub mod mdl_cam;
pub mod op;
pub mod opd;
pub mod opus;
pub mod part;
pub mod ppp;
pub mod prog;
pub mod rand;
pub mod slots;
pub mod small;
pub mod tail;
pub mod tree;
pub mod vec;
pub mod wave;
pub mod x87;
