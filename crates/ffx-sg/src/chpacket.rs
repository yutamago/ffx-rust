//! The character packet's draw settings: the `Ch_PacketSet*` setters the
//! character draw (`FUN_0082e610`) and the debug parameter window call
//! before building a model's VU1 packet. Each stores its argument in a
//! global the packet builder (`FUN_008423c0`, `FUN_00843f10`,
//! `FUN_008440f0`) reads.

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::x87;
    use ffx_bridge::{Global, Mem};

    use crate::lo8;

    /// The model's name word (`Ch_PacketSetName`).
    pub const NAME: Global<u32> = Global::new(0x0130_33a8);
    /// Back-face culling (`Ch_PacketSetCull`).
    pub const CULL: Global<u8> = Global::new(0x0130_3342);
    /// The shadow switch (`Ch_PacketSetShadow`).
    pub const SHADOW: Global<u8> = Global::new(0x0130_3343);
    /// Specular on (`Ch_PacketSetSpecular`).
    pub const SPECULAR: Global<u8> = Global::new(0x0130_3372);
    /// The shadow's texture (`Ch_PacketSetShadowTexture`); cleared with
    /// the texture when no texture list is loaded.
    pub const SHADOW_TEXTURE: Global<u8> = Global::new(0x0130_3373);
    /// Texturing on (`Ch_PacketSetTexture`).
    pub const TEXTURE: Global<u8> = Global::new(0x0130_3374);
    /// Forced specular (`Ch_PacketSetFSpecular`).
    pub const FSPECULAR: Global<u8> = Global::new(0x0130_3375);
    /// The shadow colour, three bytes (`Ch_PacketSetShadowColor`).
    pub const SHADOW_COLOR: Global<[u8; 3]> = Global::new(0x0130_3376);
    /// Semi-transparent shadow (`Ch_PacketSetShadowSemi`).
    pub const SHADOW_SEMI: Global<u8> = Global::new(0x0130_3379);
    /// The specular colour's first two bytes (`Ch_PacketSetSpecColor`).
    pub const SPEC_COLOR_RG: Global<[u8; 2]> = Global::new(0x0130_337a);
    /// The specular size (`Ch_PacketSetSpecSize`).
    pub const SPEC_SIZE: Global<f32> = Global::new(0x0130_337c);
    /// The specular coefficient (`Ch_PacketSetSpecCoefficient`).
    pub const SPEC_COEFFICIENT: Global<f32> = Global::new(0x0130_3380);
    /// The specular colour's last two bytes (`Ch_PacketSetSpecColor`).
    pub const SPEC_COLOR_BA: Global<[u8; 2]> = Global::new(0x0130_3384);

    /// `Ch_PacketSetName(v)` (`0x00843000`): returns `v` (left in EAX).
    pub fn set_name(mem: &mut impl Mem, v: u32) -> u32 {
        NAME.set(mem, v);
        v
    }

    /// `Ch_PacketSetCull(v)` (`0x00843010`).
    pub fn set_cull(mem: &mut impl Mem, v: u32) {
        CULL.set(mem, lo8(v));
    }

    /// `Ch_PacketSetFSpecular(v)` (`0x00843020`).
    pub fn set_fspecular(mem: &mut impl Mem, v: u32) {
        FSPECULAR.set(mem, lo8(v));
    }

    /// `Ch_PacketSetShadow(v)` (`0x00843040`).
    pub fn set_shadow(mem: &mut impl Mem, v: u32) {
        SHADOW.set(mem, lo8(v));
    }

    /// `Ch_PacketSetShadowColor(r, g, b)` (`0x00843050`).
    pub fn set_shadow_color(mem: &mut impl Mem, r: u32, g: u32, b: u32) {
        SHADOW_COLOR.set(mem, [lo8(r), lo8(g), lo8(b)]);
    }

    /// `Ch_PacketSetShadowSemi(v)` (`0x00843070`).
    pub fn set_shadow_semi(mem: &mut impl Mem, v: u32) {
        SHADOW_SEMI.set(mem, lo8(v));
    }

    /// `Ch_PacketSetShadowTexture(v)` (`0x00843080`).
    pub fn set_shadow_texture(mem: &mut impl Mem, v: u32) {
        SHADOW_TEXTURE.set(mem, lo8(v));
    }

    /// `Ch_PacketSetSpecCoefficient(x)` (`0x00843090`): through `fld`, so
    /// a signalling NaN is stored quiet.
    pub fn set_spec_coefficient(mem: &mut impl Mem, x: f32) {
        SPEC_COEFFICIENT.set(mem, x87::round_trip(x));
    }

    /// `Ch_PacketSetSpecColor(r, g, b, a)` (`0x008430a0`).
    pub fn set_spec_color(mem: &mut impl Mem, r: u32, g: u32, b: u32, a: u32) {
        SPEC_COLOR_RG.set(mem, [lo8(r), lo8(g)]);
        SPEC_COLOR_BA.set(mem, [lo8(b), lo8(a)]);
    }

    /// `Ch_PacketSetSpecSize(x)` (`0x008430d0`): through `fld`.
    pub fn set_spec_size(mem: &mut impl Mem, x: f32) {
        SPEC_SIZE.set(mem, x87::round_trip(x));
    }

    /// `Ch_PacketSetSpecular(v)` (`0x008430e0`).
    pub fn set_specular(mem: &mut impl Mem, v: u32) {
        SPECULAR.set(mem, lo8(v));
    }

    /// `Ch_PacketSetTexture(v)` (`0x008430f0`).
    pub fn set_texture(mem: &mut impl Mem, v: u32) {
        TEXTURE.set(mem, lo8(v));
    }
}
