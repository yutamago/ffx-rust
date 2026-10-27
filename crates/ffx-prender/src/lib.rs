//! `PhyreEngine`'s map-world render chain, ported function by function
//! from FFX.exe (the M4 pilot, `~/ffx-work/m4/pilot-report.md`): the
//! path from `PApplication::renderWorld` to the render commands, as the
//! M3 pipeline ports game code (`docs/porting.md`).
//!
//! - [`sort`]: the visibility entries' `std::sort` and its comparator;
//! - [`visibility`]: `PVisibilityCheck` and the culling job;
//! - [`world`]: `renderWorld`, `renderWorldCulledSorted`, `mergeResults`;
//! - [`array`]: `PArray`/`PSharray` storage helpers (h2-render-b);
//! - [`declare`]: the `PArray<T>` class descriptors' constructors and
//!   declarations (h2-render-b round 2);
//! - [`descriptor`]: the type system's descriptor and array-member slots
//!   of the render classes (M4 port round 1, h2-render-b);
//! - [`dtor`]: destructors and storage setters (round 2);
//! - [`h2_bind`]: the render classes' `Bind()` registrations (round 3);
//! - [`h2_binds2`]: six large `Bind()`s on [`h1_bind`]'s steps (h2 continuation);
//! - [`h2_more`], [`h2_more2`], [`h2_more3`], [`h2_more4`], [`h2_more5`], [`h2_more6`], [`h2_more7`]: the larger render methods of h2-render-b
//!   round 3;
//! - [`misc`]: small render-interface, material and shader methods (the
//!   same).
//! - [`descriptor`]: the render classes' class-descriptor callbacks and
//!   `Bind` functions (M4 port round 1, h1-render-a);
//! - [`registry`]: descriptor constructors, `GetType<T>` singletons,
//!   `Bind` with a member descriptor (h1-render-a);
//! - [`texture`], [`basic`]: texture and sampler bases, small engine
//!   methods (h1-render-a).
//!
//! Everything the slice does not port (the world cursors, the job system,
//! the ring allocators, the math helpers, `D3D11`) is called at its
//! address through [`ffx_graphic::host::Host`], as other ports call
//! original code.

// 32-bit game addresses and words: indices, offsets and signed words are
// converted where the original reinterprets them; single-letter names are
// the host, entries and pointers, as in the originals.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::similar_names
)]

pub mod array;
pub mod basic;
pub mod c2_post;
pub mod c2_shadow;
pub mod c2_smaa;
pub mod class_slots;
pub mod declare;
pub mod descriptor;
pub mod dtor;
pub mod glue;
pub mod glue10;
pub mod glue11;
pub mod glue2;
pub mod glue3;
pub mod glue4;
pub mod glue5;
pub mod glue6;
pub mod glue7;
pub mod glue8;
pub mod glue9;
pub mod h1_array_type;
pub mod h1_bind;
pub mod h1_camera;
pub mod h1_small;
pub mod h1_small2;
pub mod h1_small3;
pub mod h1_small4;
pub mod h1_small5;
pub mod h2_bind;
pub mod h2_binds2;
pub mod h2_more;
pub mod h2_more10;
pub mod h2_more11;
pub mod h2_more12;
pub mod h2_more13;
pub mod h2_more14;
pub mod h2_more2;
pub mod h2_more3;
pub mod h2_more4;
pub mod h2_more5;
pub mod h2_more6;
pub mod h2_more7;
pub mod h2_more8;
pub mod h2_more9;
pub mod h3_glue12;
pub mod h3_glue13;
pub mod h3_glue14;
pub mod h3_glue15;
pub mod h3_glue16;
pub mod h3_glue17;
pub mod h3_glue18;
pub mod h3_glue19;
pub mod h3_glue20;
pub mod h3_glue21;
pub mod h3_glue22;
pub mod h3_glue23;
pub mod h3_glue24;
pub mod h3_glue25;
pub mod h3_glue26;
pub mod h3_glue27;
pub mod h3_glue28;
pub mod h3_glue29;
pub mod h3_glue30;
pub mod h3_glue31;
pub mod h3_glue32;
pub mod h3_glue33;
pub mod h3_glue34;
pub mod h3_glue35;
pub mod h3_glue36;
pub mod h3_glue37;
pub mod h3_glue38;
pub mod h3_glue39;
pub mod h3_glue40;
pub mod h3_glue41;
pub mod h3_glue42;
pub mod h3_glue43;
pub mod h3_glue44;
pub mod h3_glue45;
pub mod h3_glue46;
pub mod h3_glue47;
pub mod h3_glue48;
pub mod h3_glue49;
pub mod h3_glue50;
pub mod h3_glue51;
pub mod h3_glue52;
pub mod h3_glue53;
pub mod h3_glue54;
pub mod h3_glue55;
pub mod h3_glue56;
pub mod h3_glue57;
pub mod h3_glue58;
pub mod h3_glue59;
pub mod h3_glue60;
pub mod h3_glue61;
pub mod h3_glue62;
pub mod h3_glue63;
pub mod h3_glue64;
pub mod h3b_glue1;
pub mod h3b_glue2;
pub mod h3b_glue3;
pub mod h3b_glue4;
pub mod h3b_glue5;
pub mod h3b_glue6;
pub mod h3b_glue7;
pub mod m4r_ctxcapture;
pub mod m4r_fonttext;
pub mod m4r_gauss;
pub mod m4r_glow;
pub mod m4r_meshlist;
pub mod m4r_modnet;
pub mod m4r_nodecapture;
pub mod m4r_rendercmds;
pub mod mesh;
pub mod misc;
pub mod p6_app;
pub mod p6_appframe;
pub mod p6_appinit;
pub mod p6_binds;
pub mod p6_binds1;
pub mod p6_cmd;
pub mod p6_descriptor;
pub mod p6_encode;
pub mod p6_late;
pub mod p6_mesh;
pub mod p6_misc;
pub mod p6_more;
pub mod p6_post;
pub mod p6_quads;
pub mod p6_setup;
pub mod p6_small;
pub mod p6_tail;
pub mod p6_targets;
pub mod p6_thread;
pub mod registry;
pub mod small;
pub mod sort;
pub mod texture;
pub mod thread;
pub mod visibility;
pub mod world;
