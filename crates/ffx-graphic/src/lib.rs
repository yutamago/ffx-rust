//! The remaster renderer over `PhyreEngine` (`graphic*`, `PhyreScene`, the managers).
//!
//! The `render` module of `docs/rewrite-plan.md`: each original function
//! becomes a core (portable, on its own types) where it has logic of its
//! own, and a `bridge` function over game memory (`docs/porting.md`), one
//! `bridge` per area:
//! - [`vfx`]: the effects' render-data table and the `graphicVFX*` calls;
//! - [`scene`]: `PhyreScene` and the `graphic*` getters and setters of its
//!   state;
//! - [`ui`]: the UI's coordinate remaps and font metrics;
//! - [`managers`]: the post-process, shadow, texture-animation and
//!   dynamic-geometry managers' accessors;
//! - [`cache`]: the model cache (`Cache_*`).
//!
//! `PhyreEngine` and the C++ objects on it are library code: bridges call
//! them through [`host::Host`], never port them. The data layouts are in
//! `docs/formats/render.md`.

pub mod cache;
pub mod host;
pub mod managers;
pub mod scene;
pub mod ui;
pub mod vfx;
