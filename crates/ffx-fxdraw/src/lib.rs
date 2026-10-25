//! The effects draw census of M4 (`docs/formats/effects-draw.md`): every
//! draw of a captured battle frame with its producer, the effect draws
//! traced to the `VFXDynamicGeometry` whose arrays they draw, its instance
//! key's maker (ppp object and program, op record) and the `PhyreScene`
//! list that draws it.
//!
//! - [`capture`]: a hash capture's draws with the ring bytes they read
//!   (objects through `ffx-drawdiff`);
//! - [`mem`]: the game's memory from a `snapshot` taken for the frame;
//! - [`game`]: `PhyreScene`'s VFX lists and draw events, the VFX render
//!   data table and its delete pool, ppp program records, spell overlays;
//! - [`shaders`]: shader families and the `vfxShader_fx.txt` table;
//! - [`census`]: the alignment of the effect draws with the lists, and the
//!   classification of the rest;
//! - [`states`]: blend, depth and raster state and used constants per draw.

pub mod capture;
pub mod census;
pub mod game;
pub mod mem;
pub mod shaders;
pub mod states;
