//! The physics classes' create callbacks (`PPhysicsBox`, `...Capsule`,
//! `...Cylinder`, `...Mesh`, `...Plane`, `...Shape`, `...Sphere`,
//! `...TaperedCapsule`, `...TaperedCylinder`, each in its plain, `Base` and
//! `Bullet` flavour): the shape's base constructor (`0x005e0740`, thiscall
//! `(shape)`), then its fields and vtable in the original's store order.
//! Plus the small value types' create callbacks (no constructor call).
//!
//! The float fields are `fld1`/`fldz` (1.0, 0.0) or copies of an `.rdata`
//! float (`fld m32` / `fstp m32`, bit-exact for the non-NaN constants
//! there), which the port reads from the image.

use crate::{thiscall, with_al};
use ffx_graphic::host::Host;

/// The shapes' base constructor (thiscall `(shape)`).
pub const SHAPE_BASE: u32 = 0x005e_0740;
/// `.rdata` 2.0f.
pub const TWO: u32 = 0x00b9_2268;
/// `.rdata` float the tapered shapes store at `+0x6c`.
pub const TAPER: u32 = 0x00b3_eb94;
/// 1.0f (`fld1`).
pub const ONE_BITS: u32 = 0x3f80_0000;

/// One store of a create callback.
#[derive(Debug, Clone, Copy)]
pub enum Store {
    /// A 32-bit constant (zeroes, enum values, `fld1` / `fldz` bits).
    Word(u32, u32),
    /// A byte constant.
    Byte(u32, u8),
    /// An image address (a vtable), rebased.
    Image(u32, u32),
    /// A copy of the 32-bit `.rdata` value at the address.
    Rdata(u32, u32),
}

/// A create callback: the base constructor to call first (none for the
/// value types) and the stores.
#[derive(Debug, Clone, Copy)]
pub struct Create {
    pub base: Option<u32>,
    pub stores: &'static [Store],
}

use Store::{Byte, Image, Rdata, Word};

const fn shape(stores: &'static [Store]) -> Create {
    Create {
        base: Some(SHAPE_BASE),
        stores,
    }
}

/// `0x005f7a10` `PPhysicsBox`.
pub const BOX: Create = shape(&[
    Word(0x58, 0),
    Word(0x5c, ONE_BITS),
    Word(0x60, ONE_BITS),
    Word(0x64, ONE_BITS),
    Word(0x54, 0),
    Word(0x6c, 0),
    Image(0, 0x00b3_9cf0),
]);
/// `0x005f7a50` `PPhysicsBoxBase`.
pub const BOX_BASE: Create = shape(&[
    Word(0x58, 0),
    Image(0, 0x00b3_9af8),
    Word(0x5c, ONE_BITS),
    Word(0x60, ONE_BITS),
    Word(0x64, ONE_BITS),
    Word(0x54, 0),
]);
/// `0x005f7a90` `PPhysicsBoxBullet`.
pub const BOX_BULLET: Create = shape(&[
    Word(0x58, 0),
    Word(0x5c, ONE_BITS),
    Word(0x60, ONE_BITS),
    Word(0x64, ONE_BITS),
    Word(0x54, 0),
    Image(0, 0x00b3_9bf0),
    Word(0x6c, 0),
]);
/// `0x005f7ad0` `PPhysicsCapsule`.
pub const CAPSULE: Create = shape(&[
    Rdata(0x64, TWO),
    Word(0x58, 0),
    Word(0x60, ONE_BITS),
    Word(0x5c, ONE_BITS),
    Word(0x68, 0),
    Word(0x54, 1),
    Word(0x6c, 0),
    Image(0, 0x00b3_a5e0),
]);
/// `0x005f7b20` `PPhysicsCapsuleBase`.
pub const CAPSULE_BASE: Create = shape(&[
    Rdata(0x64, TWO),
    Word(0x58, 0),
    Word(0x60, ONE_BITS),
    Word(0x5c, ONE_BITS),
    Word(0x68, 0),
    Image(0, 0x00b3_a3e0),
    Word(0x54, 1),
]);
/// `0x005f7b70` `PPhysicsCapsuleBullet`.
pub const CAPSULE_BULLET: Create = shape(&[
    Rdata(0x64, TWO),
    Word(0x58, 0),
    Word(0x60, ONE_BITS),
    Word(0x5c, ONE_BITS),
    Word(0x68, 0),
    Word(0x54, 1),
    Image(0, 0x00b3_a4e0),
    Word(0x6c, 0),
]);
/// `0x005f7cd0` `PPhysicsCylinder`.
pub const CYLINDER: Create = shape(&[
    Rdata(0x64, TWO),
    Word(0x58, 0),
    Word(0x60, ONE_BITS),
    Word(0x5c, ONE_BITS),
    Word(0x54, 2),
    Word(0x68, 0),
    Image(0, 0x00b3_a2e0),
]);
/// `0x005f7d20` `PPhysicsCylinderBase`.
pub const CYLINDER_BASE: Create = shape(&[
    Rdata(0x64, TWO),
    Word(0x58, 0),
    Image(0, 0x00b3_a0e0),
    Word(0x60, ONE_BITS),
    Word(0x5c, ONE_BITS),
    Word(0x54, 2),
]);
/// `0x005f7d60` `PPhysicsCylinderBullet`.
pub const CYLINDER_BULLET: Create = shape(&[
    Rdata(0x64, TWO),
    Word(0x58, 0),
    Word(0x60, ONE_BITS),
    Word(0x5c, ONE_BITS),
    Word(0x54, 2),
    Image(0, 0x00b3_a1e0),
    Word(0x68, 0),
]);
/// `0x005f7e50` `PPhysicsMesh`.
pub const MESH: Create = shape(&[
    Word(0x58, 0),
    Word(0x5c, 0),
    Word(0x54, 7),
    Word(0x60, 0),
    Word(0x64, 0),
    Word(0x68, 0),
    Image(0, 0x00b3_9fe8),
]);
/// `0x005f7ea0` `PPhysicsMeshBase`.
pub const MESH_BASE: Create = shape(&[
    Word(0x58, 0),
    Image(0, 0x00b3_9de8),
    Word(0x5c, 0),
    Word(0x54, 7),
]);
/// `0x005f7ee0` `PPhysicsMeshBullet`.
pub const MESH_BULLET: Create = shape(&[
    Word(0x58, 0),
    Word(0x5c, 0),
    Word(0x54, 7),
    Image(0, 0x00b3_9ee8),
    Word(0x60, 0),
    Word(0x64, 0),
    Word(0x68, 0),
]);
/// `0x005f7f60` `PPhysicsPlane`.
pub const PLANE: Create = shape(&[
    Word(0x58, 0),
    Word(0x5c, ONE_BITS),
    Word(0x60, ONE_BITS),
    Word(0x64, ONE_BITS),
    Word(0x68, ONE_BITS),
    Word(0x54, 3),
    Word(0x6c, 0),
    Image(0, 0x00b3_aad8),
]);
/// `0x005f7fb0` `PPhysicsPlaneBase`.
pub const PLANE_BASE: Create = shape(&[
    Word(0x58, 0),
    Image(0, 0x00b3_a8d8),
    Word(0x5c, ONE_BITS),
    Word(0x60, ONE_BITS),
    Word(0x64, ONE_BITS),
    Word(0x68, ONE_BITS),
    Word(0x54, 3),
]);
/// `0x005f7ff0` `PPhysicsPlaneBullet`.
pub const PLANE_BULLET: Create = shape(&[
    Word(0x58, 0),
    Word(0x5c, ONE_BITS),
    Word(0x60, ONE_BITS),
    Word(0x64, ONE_BITS),
    Word(0x68, ONE_BITS),
    Word(0x54, 3),
    Image(0, 0x00b3_a9d8),
    Word(0x6c, 0),
]);
/// `0x005f8170` `PPhysicsShape`.
pub const SHAPE: Create = shape(&[Word(0x58, 0), Image(0, 0x00b3_9708)]);
/// `0x005f81a0` `PPhysicsShapeBullet`.
pub const SHAPE_BULLET: Create = shape(&[Image(0, 0x00b3_9608), Word(0x58, 0)]);
/// `0x005f81d0` `PPhysicsSphere`.
pub const SPHERE: Create = shape(&[
    Word(0x5c, ONE_BITS),
    Word(0x58, 0),
    Word(0x54, 4),
    Word(0x60, 0),
    Image(0, 0x00b3_9a00),
]);
/// `0x005f8210` `PPhysicsSphereBase`.
pub const SPHERE_BASE: Create = shape(&[
    Word(0x5c, ONE_BITS),
    Word(0x58, 0),
    Image(0, 0x00b3_9800),
    Word(0x54, 4),
]);
/// `0x005f8250` `PPhysicsSphereBullet`.
pub const SPHERE_BULLET: Create = shape(&[
    Word(0x5c, ONE_BITS),
    Word(0x58, 0),
    Word(0x54, 4),
    Image(0, 0x00b3_9900),
    Word(0x60, 0),
]);
/// `0x005f8290` `PPhysicsTaperedCapsule`.
pub const TAPERED_CAPSULE: Create = shape(&[
    Rdata(0x6c, TAPER),
    Word(0x58, 0),
    Word(0x60, ONE_BITS),
    Word(0x5c, ONE_BITS),
    Rdata(0x68, TWO),
    Rdata(0x64, TWO),
    Image(0, 0x00b3_a7d8),
    Word(0x54, 5),
]);
/// `0x005f82e0` `PPhysicsTaperedCylinder`.
pub const TAPERED_CYLINDER: Create = shape(&[
    Rdata(0x6c, TAPER),
    Word(0x58, 0),
    Image(0, 0x00b3_a6d8),
    Word(0x54, 6),
    Word(0x60, ONE_BITS),
    Word(0x5c, ONE_BITS),
    Rdata(0x68, TWO),
    Rdata(0x64, TWO),
]);

const fn value(stores: &'static [Store]) -> Create {
    Create { base: None, stores }
}

/// `0x005f79f0`: `{0, 0}`.
pub const PAIR: Create = value(&[Word(0, 0), Word(4, 0)]);
/// `0x005f7db0`: `{byte 0, 0, 0x10, 0}`.
pub const BYTE_HEADER: Create = value(&[Byte(0, 0), Word(4, 0), Word(8, 0x10), Word(0xc, 0)]);
/// `0x005f7e30`: three 0.0f (`fldz`).
pub const VEC3_ZERO: Create = value(&[Word(0, 0), Word(4, 0), Word(8, 0)]);
/// `0x005f7f30`: three zero words.
pub const TRIPLE: Create = value(&[Word(0, 0), Word(4, 0), Word(8, 0)]);
/// `0x005f8390`: `+0x20`, `+0x24` zero.
pub const TAIL_PAIR: Create = value(&[Word(0x20, 0), Word(0x24, 0)]);

/// The rigid bodies' destroy-and-clear (`0x005f8040`, `0x005f8100`): the
/// release (thiscall `(body)`) then `+0xdc`, `+0xe0` zero.
pub const BODY_RELEASE: u32 = 0x005e_0590;

pub mod bridge {
    //! The originals over game memory.

    use super::{BODY_RELEASE, Create, Host, Store, thiscall, with_al};

    fn stores(h: &mut impl Host, p: u32, stores: &[Store]) {
        for &s in stores {
            match s {
                Store::Word(off, v) => h.write(p.wrapping_add(off), v),
                Store::Byte(off, v) => h.write(p.wrapping_add(off), v),
                Store::Image(off, sym) => {
                    let v = h.rebase(sym);
                    h.write(p.wrapping_add(off), v);
                }
                Store::Rdata(off, sym) => {
                    let v: u32 = h.read(h.rebase(sym));
                    h.write(p.wrapping_add(off), v);
                }
            }
        }
    }

    /// A create callback (`stdcall (p)`) unless `p` is null. `eax`: `p`
    /// with `al = 1` for the value types; for the shapes only `al = 1` is
    /// defined (the base constructor's `eax` is left).
    pub fn create(h: &mut impl Host, p: u32, c: &Create) -> u32 {
        if p == 0 {
            return 1;
        }
        if let Some(base) = c.base {
            thiscall(h, base, p, &[]);
        }
        stores(h, p, c.stores);
        with_al(p, 1)
    }

    /// `0x005f8040` / `0x005f8100` (destroy): `0x005e0590(p)` and two
    /// zero words, unless null.
    pub fn destroy_body(h: &mut impl Host, p: u32) -> u8 {
        if p != 0 {
            thiscall(h, BODY_RELEASE, p, &[]);
            h.write(p + 0xdc, 0u32);
            h.write(p + 0xe0, 0u32);
        }
        1
    }
}
