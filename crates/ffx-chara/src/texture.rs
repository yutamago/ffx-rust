//! A character's textures and cluts (palettes): the texture lists at
//! `+0x6e4`/`+0x6ec`, the clut table at `+0x704` and the model block's
//! texture data ([`crate::layout::BLK_TEX`]).

/// `Ch_ClutGetSize`'s palette entries for a clut of `bytes` bytes
/// (a 32-byte header, 4 bytes per entry; divided toward zero).
#[must_use]
pub const fn clut_entries(bytes: i32) -> i32 {
    bytes.wrapping_sub(0x20) / 4
}

/// The originals, where the game keeps the textures.
pub mod bridge {
    use ffx_bridge::Mem;

    use super::clut_entries;
    use crate::chr::bridge::{blk, set_flag};
    use crate::layout as l;

    /// Entry `i`'s word at `+off` of a texture list, or 0 past its end (the
    /// original lets `i == count` through).
    fn texture(mem: &impl Mem, chr: u32, list: u32, count: u32, i: i32, off: u32) -> Option<u32> {
        if i32::from(mem.read::<i16>(chr + count)) < i {
            return None;
        }
        let t: u32 = mem.read(chr + list);
        #[allow(clippy::cast_sign_loss)] // shl on the raw index
        Some(t.wrapping_add((i as u32).wrapping_shl(6)).wrapping_add(off))
    }

    /// `Ch_TextureAnimSetEnable(chr, on)` (`0x00830d10`).
    pub fn ch_texture_anim_set_enable(mem: &mut impl Mem, chr: u32, on: u32) -> u32 {
        set_flag(mem, chr, l::FLAGS2, l::F2_TEXTURE_ANIM, on)
    }

    /// `Ch_TextureGetClutAddr(chr, i)` (`0x00830d30`).
    pub fn ch_texture_get_clut_addr(mem: &impl Mem, chr: u32, i: i32) -> u32 {
        texture(mem, chr, l::CLUT_TEXTURES, l::CLUT_TEXTURE_COUNT, i, 0x30)
            .map_or(0, |a| mem.read(a))
    }

    /// `Ch_TextureGetClutBase(chr, i)` (`0x00830d60`).
    pub fn ch_texture_get_clut_base(mem: &impl Mem, chr: u32, i: i32) -> u32 {
        texture(mem, chr, l::CLUT_TEXTURES, l::CLUT_TEXTURE_COUNT, i, 0x3a)
            .map_or(0, |a| u32::from(mem.read::<u16>(a)))
    }

    /// `Ch_TextureGetImageAddr(chr, i)` (`0x00830d90`).
    pub fn ch_texture_get_image_addr(mem: &impl Mem, chr: u32, i: i32) -> u32 {
        texture(mem, chr, l::IMAGE_TEXTURES, l::IMAGE_TEXTURE_COUNT, i, 0x30)
            .map_or(0, |a| mem.read(a))
    }

    /// `Ch_TextureGetImageBase(chr, i)` (`0x00830dc0`).
    pub fn ch_texture_get_image_base(mem: &impl Mem, chr: u32, i: i32) -> u32 {
        texture(mem, chr, l::IMAGE_TEXTURES, l::IMAGE_TEXTURE_COUNT, i, 0x3a)
            .map_or(0, |a| u32::from(mem.read::<u16>(a)))
    }

    fn clut(mem: &impl Mem, chr: u32, i: u32, off: u32) -> Option<u32> {
        let c: u32 = mem.read(chr + l::CLUTS);
        (c != 0).then(|| c.wrapping_add(i.wrapping_mul(16)).wrapping_add(off))
    }

    /// `Ch_ClutEnable(chr, on)` (`0x0083c940`): puts each clut texture's
    /// clut (on) or its original (off) in place.
    pub fn ch_clut_enable(mem: &mut impl Mem, chr: u32, on: u32) {
        let cluts: u32 = mem.read(chr + l::CLUTS);
        if cluts == 0 {
            return;
        }
        let tex: u32 = mem.read(chr + l::CLUT_TEXTURES);
        #[allow(clippy::cast_possible_truncation)] // the low byte
        mem.write(chr + l::CLUT_ENABLED, on as u8);
        let mut i = 0i32;
        while i < i32::from(mem.read::<i16>(chr + l::CLUT_TEXTURE_COUNT)) {
            let k = i.cast_unsigned();
            let src = cluts + 16 * k + if on == 0 { 4 } else { 0 };
            let v: u32 = mem.read(src);
            mem.write(tex + 0x30 + 0x40 * k, v);
            i += 1;
        }
    }

    /// `Ch_ClutGetNb(chr)` (`0x0083c9a0`), sign-extended.
    pub fn ch_clut_get_nb(mem: &impl Mem, chr: u32) -> i32 {
        i32::from(mem.read::<i16>(chr + l::CLUT_TEXTURE_COUNT))
    }

    /// `Ch_ClutGetPtr(chr, i)` (`0x0083c9d0`).
    pub fn ch_clut_get_ptr(mem: &impl Mem, chr: u32, i: u32) -> u32 {
        clut(mem, chr, i, 0).map_or(0, |a| mem.read(a))
    }

    /// `Ch_ClutGetOrgPtr(chr, i)` (`0x0083c9b0`).
    pub fn ch_clut_get_org_ptr(mem: &impl Mem, chr: u32, i: u32) -> u32 {
        clut(mem, chr, i, 4).map_or(0, |a| mem.read(a))
    }

    /// `Ch_ClutGetSize(chr, i)` (`0x0083c9f0`): palette entries.
    pub fn ch_clut_get_size(mem: &impl Mem, chr: u32, i: u32) -> i32 {
        clut(mem, chr, i, 8).map_or(0, |a| clut_entries(mem.read(a)))
    }

    /// `Ch_GetLightValue(chr)` (`0x0083cb90`): the address of the light
    /// value fade.
    #[must_use]
    pub const fn ch_get_light_value(chr: u32) -> u32 {
        chr.wrapping_add(l::FADE_LIGHT)
    }

    fn tex(mem: &impl Mem, chr: u32) -> u32 {
        mem.read(blk(mem, chr) + l::BLK_TEX)
    }

    /// `Ch_TexGetClut(chr, i, &out)` (`0x0083cd20`): clut `i`'s first word
    /// to `out`, returns its second; 0 without texture data.
    pub fn ch_tex_get_clut(mem: &mut impl Mem, chr: u32, i: u32, out: u32) -> u32 {
        let t = tex(mem, chr);
        if t == 0 {
            return 0;
        }
        let e = mem.read::<u32>(t + 0x18).wrapping_add(i.wrapping_mul(8));
        let v: u32 = mem.read(e);
        mem.write(out, v);
        mem.read(e + 4)
    }

    /// `Ch_TexGetImage(chr, i, &w, &h)` (`0x0083cd50`): image `i`'s size
    /// to `w`, `h`, returns its data; 0 without texture data.
    pub fn ch_tex_get_image(mem: &mut impl Mem, chr: u32, i: u32, width: u32, height: u32) -> u32 {
        let data = tex(mem, chr);
        if data == 0 {
            return 0;
        }
        let e = i.wrapping_shl(4).wrapping_add(mem.read(data + 0x1c));
        let vw = u32::from(mem.read::<u16>(e + 8));
        mem.write(width, vw);
        let vh = u32::from(mem.read::<u16>(e + 0xa));
        mem.write(height, vh);
        mem.read(e + 0xc)
    }

    /// `Ch_TexGetNbClut(chr)` (`0x0083cd90`).
    pub fn ch_tex_get_nb_clut(mem: &impl Mem, chr: u32) -> u32 {
        let t = tex(mem, chr);
        if t == 0 {
            0
        } else {
            u32::from(mem.read::<u16>(t + 8))
        }
    }

    /// `Ch_TexGetNbImage(chr)` (`0x0083cdb0`).
    pub fn ch_tex_get_nb_image(mem: &impl Mem, chr: u32) -> u32 {
        let t = tex(mem, chr);
        if t == 0 {
            0
        } else {
            u32::from(mem.read::<u16>(t + 0xc))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clut_sizes_round_toward_zero() {
        assert_eq!(clut_entries(0x20 + 64), 16);
        assert_eq!(clut_entries(0x1f), 0);
        assert_eq!(clut_entries(0x1b), -1);
    }
}
