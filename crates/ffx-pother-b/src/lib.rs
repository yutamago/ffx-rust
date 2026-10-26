//! `PhyreEngine` code the game reaches outside rendering, ported function
//! by function from FFX.exe (M4 port round 1, list `p4-other-b`): the M3
//! pipeline of `docs/porting.md` with the pilot's "Library code" rules.
//!
//! - [`physics`]: the `PPhysics` class descriptors' instance callbacks;
//! - [`object`]: scalar deleting destructors, dynamic-mesh instance
//!   constructors, class registrations;
//! - [`p4_input`], [`p4_light`], [`p4_fog`], [`p4_morph`], [`p4_cloth`]
//!   (continuation rounds): input devices and mappings, the character
//!   lighting manager, the fog manager, morph weights and material
//!   parameters, cloth instancing and the dynamic meshes' buffer copies;
//! - [`p4_bind`]: the input and game-settings classes' reflection
//!   registrations and start-up;
//! - [`p4_mesh`], [`p4_frames`]: the dynamic meshes' instances and
//!   tables; originals that hand their own locals to callees (the shader
//!   cluster loads, two tree inserts).
//!
//! Everything outside the list (the allocator `Phyre_PhyreFree`, the
//! physics objects' own destructors, virtual methods) is called at its
//! address through [`ffx_graphic::host::Host`], in the original's
//! convention.

// 32-bit game addresses and words.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::similar_names
)]

pub mod methods;
pub mod object;
pub mod p4_bind;
pub mod p4_cloth;
pub mod p4_fog;
pub mod p4_frames;
pub mod p4_input;
pub mod p4_light;
pub mod p4_light2;
pub mod p4_mesh;
pub mod p4_morph;
pub mod p4_ps2light;
pub mod p4_tree;
pub mod physics;
pub mod small;

/// `Phyre_PhyreFree` (cdecl `(ptr)`).
pub const PHYRE_FREE: u32 = 0x0042_fb20;
