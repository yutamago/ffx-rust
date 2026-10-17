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
    use ffx_bridge::{Global, Mem};

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

    /// Clut disposal is held while this is set (`Ch_ClutDisposeCheck`).
    pub const CLUT_DISPOSE_HOLD: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x0130_19c0);

    /// `Ch_ClutDisposeCheck()` (`0x0083c910`): 1 when cluts may be freed:
    /// not held, and no spell effect loaded but `0x2a0`'s.
    pub fn ch_clut_dispose_check(mem: &impl Mem) -> u32 {
        use ffx_system::instance::bridge as sys;
        if CLUT_DISPOSE_HOLD.get(mem) != 0 {
            return 0;
        }
        if sys::magic_handle(mem) == u32::MAX {
            return 1;
        }
        u32::from(sys::magic_id(mem) != 0x2a0)
    }

    /// Two GS packets for the specular textures (`Ch_TextureSendSpecular`).
    pub const SPECULAR: [ffx_bridge::Global<u32>; 2] = [
        ffx_bridge::Global::new(0x0130_19e0),
        ffx_bridge::Global::new(0x0130_1a20),
    ];
    /// Their clut data pointers (`+0x30` of each).
    pub const SPECULAR_DATA: [ffx_bridge::Global<u32>; 2] = [
        ffx_bridge::Global::new(0x0130_1a10),
        ffx_bridge::Global::new(0x0130_1a50),
    ];

    /// `Ch_TextureSendSpecular(out)` (`0x0083cf80`): two DMA tags (a
    /// reference to each specular packet, then its data) to `out`; marks
    /// each data block `0x5100`. Returns `out + 0x40`.
    pub fn ch_texture_send_specular(mem: &mut impl Mem, out: u32) -> u32 {
        for (i, (pk, data)) in (0u32..).zip(SPECULAR.into_iter().zip(SPECULAR_DATA)) {
            let o = out + 0x20 * i;
            let at = pk.ptr(mem).addr;
            mem.write(o, [0x3000_0005u32, at, 0, 0]);
            let d = data.get(mem);
            mem.write(d.wrapping_sub(0x12), 0x5100u16);
            let n = mem.read::<u32>(d.wrapping_sub(0x10)) & 0x7fff;
            mem.write(o + 0x10, n.wrapping_add(2) | 0x3000_0000);
            if i == 0 {
                mem.write(o + 0x18, [0u32; 2]);
            }
            let d = data.get(mem);
            mem.write(o + 0x14, d.wrapping_sub(0x20));
            if i == 1 {
                mem.write(o + 0x18, [0u32; 2]);
            }
        }
        out + 0x40
    }

    /// `Ch_GetBigImageSize(chr, &w, &h, rows)` (`0x00833e60`): the widest
    /// image and the heights of all of them stacked (the images of the
    /// archive's texture list), and in `rows` where each starts.
    pub fn ch_get_big_image_size(mem: &mut impl Mem, chr: u32, w: u32, h: u32, rows: u32) {
        let arc: u32 = mem.read(blk(mem, chr) + 0x6c);
        let t: u32 = mem.read(arc + 0x18);
        let n = u32::from(mem.read::<u16>(t + 4));
        let height = |mem: &mut _, k: u32| -> u32 {
            let list: u32 = Mem::read(mem, t + 0x14);
            let idx = u32::from(Mem::read::<u8>(mem, list.wrapping_add(16 * k)));
            let images: u32 = Mem::read(mem, t + 0x1c);
            u32::from(Mem::read::<u16>(mem, images.wrapping_add(16 * idx) + 0xa))
        };
        let (mut max_w, mut sum) = (0u32, 0u32);
        if n > 0 {
            let images: u32 = mem.read(t + 0x1c);
            let list: u32 = mem.read(t + 0x14);
            for k in 0..n {
                let idx = u32::from(mem.read::<u8>(list.wrapping_add(16 * k)));
                let img = images.wrapping_add(16 * idx);
                max_w = max_w.max(u32::from(mem.read::<u16>(img + 8)));
                sum = sum.wrapping_add(u32::from(mem.read::<u16>(img + 0xa)));
            }
        }
        mem.write(w, max_w);
        mem.write(h, sum);
        mem.write(rows, 0u32);
        let mut i = 1u32;
        while i < u32::from(mem.read::<u16>(t + 4)) {
            let at = rows + 4 * i;
            mem.write(at, 0u32);
            let mut s = 0u32;
            for k in 0..i {
                s = s.wrapping_add(height(mem, k));
                mem.write(at, s);
            }
            i += 1;
        }
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

    /// The callees of `Ch_ClutDispose`.
    pub const STATUS_STONE: u32 = 0x007f_cc80;
    pub const GET_LIGHT_VALUE: u32 = 0x0083_cb90;
    pub const CALC_LIGHT: u32 = 0x0083_c6f0;
    pub const CLUT_ENABLE: u32 = 0x0083_c940;
    pub const CLUT_DISPOSE_CHECK: u32 = 0x0083_c910;
    pub const USER_FREE: u32 = 0x0068_7010;
    /// `0x00642a10(instance)`: the renderer drops the instance's clut.
    pub const RENDER_CLUT_RELEASE: u32 = 0x0064_2a10;
    pub const GET_WEAPON_HANDLE: u32 = 0x0079_5b40;
    pub const GET_RENDER_INSTANCE: u32 = 0x0082_6b70;
    pub const SET_HSV_PARAM: u32 = 0x0064_3340;
    pub const ENABLE_HSV: u32 = 0x0064_0100;
    /// A colour change on a weapon's instance waiting for its clut's
    /// disposal (set by `op_chara_hsv`, `0x00807870`).
    pub const HSV_PENDING: Global<u32> = Global::new(0x012f_4110);
    pub const HSV_INSTANCE: Global<u32> = Global::new(0x012f_4114);

    /// `Ch_ClutDispose(chr)` (`0x0083c810`): a character's own clut
    /// (`+0x704`) goes unless it is petrified (`op_get_chr_status_stone`,
    /// asked twice as the original): the light recalculated
    /// (`Ch_CalcLight(chr, Ch_GetLightValue(chr))`), cluts off, the block
    /// freed; the renderer told when `Ch_ClutDisposeCheck()`; and a colour
    /// change pending on its instance is undone on weapon (6, 0)
    /// (`graphicSetHSVParam(instance, 0, 0, 0, -1)`, `graphicEnableHSV(
    /// instance, 0)`) and cleared. Returns what the original leaves in EAX.
    pub fn ch_clut_dispose<M: Mem>(
        mem: &mut M,
        chr: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        if mem.read::<u32>(chr + 0x704) == 0 {
            return 0;
        }
        call(mem, mem.rebase(STATUS_STONE), &[chr]);
        let stone = call(mem, mem.rebase(STATUS_STONE), &[chr]);
        if stone != 0 {
            return stone;
        }
        let light = call(mem, mem.rebase(GET_LIGHT_VALUE), &[chr]);
        call(mem, mem.rebase(CALC_LIGHT), &[chr, light]);
        call(mem, mem.rebase(CLUT_ENABLE), &[chr, 0]);
        let p: u32 = mem.read(chr + 0x704);
        call(mem, mem.rebase(USER_FREE), &[p]);
        mem.write(chr + 0x704, 0u32);
        let mut eax = call(mem, mem.rebase(CLUT_DISPOSE_CHECK), &[]);
        if eax != 0 {
            let inst: u32 = mem.read(chr + 0x830);
            eax = call(mem, mem.rebase(RENDER_CLUT_RELEASE), &[inst]);
        }
        if HSV_PENDING.get(mem) == 0 {
            return eax;
        }
        let pending = HSV_INSTANCE.get(mem);
        if pending == 0 || mem.read::<u32>(chr + 0x830) != pending {
            return pending;
        }
        let w = call(mem, mem.rebase(GET_WEAPON_HANDLE), &[6, 0]);
        if w != 0 {
            let minus_one = (-1f32).to_bits();
            let inst = call(
                mem,
                mem.rebase(GET_RENDER_INSTANCE),
                &[w, 0, 0, 0, minus_one],
            );
            call(mem, mem.rebase(SET_HSV_PARAM), &[inst, 0, 0, 0, minus_one]);
            let inst = call(mem, mem.rebase(GET_RENDER_INSTANCE), &[w, 0]);
            eax = call(mem, mem.rebase(ENABLE_HSV), &[inst, 0]);
        } else {
            eax = 0;
        }
        HSV_INSTANCE.set(mem, 0);
        HSV_PENDING.set(mem, 0);
        eax
    }

    /// `Sg_MemSetAlign(n)` (`0x0083c4a0`), `Ch_ClutMapping(chr, copy)`
    /// (`0x0083ca20`), `Ch_ClutDispose(chr)` (`0x0083c810`).
    pub const MEM_SET_ALIGN: u32 = 0x0083_c4a0;
    pub const CLUT_MAPPING: u32 = 0x0083_ca20;
    pub const CLUT_DISPOSE: u32 = 0x0083_c810;
    /// Bytes of scratch `ch_clut_create` needs (the block's name).
    pub const CLUT_CREATE_SCRATCH: u32 = 0x40;

    /// `Ch_ClutCreate(chr, scratch)` (`0x0083c710`): unless petrified
    /// (`op_get_chr_status_stone`, asked twice), the character's own clut
    /// block: the old one disposed (`Ch_ClutDispose` with
    /// [`CLUT_DISPOSE_HOLD`] set), then `n * 16` bytes plus each of
    /// the `n` cluts (`i16` `+0x6e8`; the definition's `+0x14` list at
    /// `+0x18`, 8-byte entries, data pointer `+4` with its size in the
    /// 15 low bits of the word 0x10 before it) aligned so that the next
    /// start plus 0x20 is a multiple of 0x80, then `(size + 4) * 16` bytes; aligned
    /// to 0x40 (`Sg_MemSetAlign`), `user_malloc`ed into `+0x704` and named
    /// `"<name>:CLUT"` (in `scratch`, the original's stack); then
    /// `Ch_ClutMapping(chr, 1)` and `Ch_ClutEnable(chr, 1)`. Returns
    /// what the original leaves in EAX.
    pub fn ch_clut_create<M: Mem>(
        mem: &mut M,
        chr: u32,
        scratch: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let def: u32 = mem.read(chr + 0x1c4);
        let tex: u32 = mem.read(def + 0x14);
        let stone = mem.rebase(crate::texture::bridge::STATUS_STONE);
        call(mem, stone, &[chr]);
        let s = call(mem, stone, &[chr]);
        if s != 0 {
            return s;
        }
        CLUT_DISPOSE_HOLD.set(mem, 1);
        call(mem, mem.rebase(CLUT_DISPOSE), &[chr]);
        CLUT_DISPOSE_HOLD.set(mem, 0);
        let n = i32::from(mem.read::<i16>(chr + 0x6e8));
        let mut size = n.cast_unsigned() << 4;
        if n > 0 {
            let mut e = mem.read::<u32>(tex + 0x18).wrapping_add(4);
            for _ in 0..n {
                while (size.wrapping_add(0x20)) & 0x7f != 0 {
                    size = size.wrapping_add(1);
                }
                let p: u32 = mem.read(e);
                e = e.wrapping_add(8);
                let w: u32 = mem.read(p.wrapping_sub(0x10));
                size = size.wrapping_add(((w & 0x7fff) + 4) << 4);
            }
        }
        call(mem, mem.rebase(MEM_SET_ALIGN), &[0x40]);
        let block = call(mem, mem.rebase(crate::handle::bridge::USER_MALLOC), &[size]);
        mem.write(chr + 0x704, block);
        let name_at: u32 = mem.read(chr + 4);
        let mut name = crate::handle::bridge::c_string(mem, name_at);
        name.extend(b":CLUT");
        crate::handle::bridge::put_c_string(mem, scratch, &name);
        let block: u32 = mem.read(chr + 0x704);
        call(
            mem,
            mem.rebase(crate::handle::bridge::MEM_SET_STR),
            &[block, scratch, u32::MAX],
        );
        call(mem, mem.rebase(CLUT_MAPPING), &[chr, 1]);
        call(
            mem,
            mem.rebase(crate::texture::bridge::CLUT_ENABLE),
            &[chr, 1],
        )
    }

    /// `Sg_MemGetSize(p)` (`0x0083c050`), `memset` through its thunk
    /// (`0x0094964c`), `Ch_TextureReMapping(chr)` (`0x0083cdd0`),
    /// `chr_texanim_set(def, handle)` (`0x0077c950`), and the block name
    /// `"Texture"` (`0x00b5a3f4`).
    pub const MEM_GET_SIZE: u32 = 0x0083_c050;
    pub const MEMSET_THUNK: u32 = 0x0094_964c;
    pub const TEXTURE_REMAPPING_FN: u32 = 0x0083_cdd0;
    pub const TEXANIM_SET: u32 = 0x0077_c950;
    pub const TEXTURE_NAME: u32 = 0x00b5_a3f4;

    /// `Ch_TextureSetup(chr)` (`0x0083d0a0`): texture data of version
    /// 0x1029 on (the model block's `+0x14`, `u16` `+0x10`) gets a cleared
    /// block named `"Texture"` for its lists (`+0x6dc`; 0x20 bytes per
    /// `u16` `+4`, 0x40 per `+8` and per `+0xc`), laid out by
    /// `Ch_TextureReMapping`; a character with a texture animation
    /// (`+0x6fc`) registers it (`+0x700` = `chr_texanim_set(data, chr)`).
    /// Returns what EAX holds: 0 without data, the data for an older
    /// version, else the last call's result.
    pub fn ch_texture_setup<M: Mem>(
        mem: &mut M,
        chr: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let data: u32 = mem.read(mem.read::<u32>(chr + 0x1c4).wrapping_add(0x14));
        if data == 0 || mem.read::<u16>(data.wrapping_add(0x10)) < 0x1029 {
            return data;
        }
        let n = |o: u32| u32::from(mem.read::<u16>(data.wrapping_add(o)));
        let size = (n(0xc) << 6) + (n(8) << 6) + (n(4) << 5);
        let p = call(mem, mem.rebase(crate::handle::bridge::USER_MALLOC), &[size]);
        let len = call(mem, mem.rebase(MEM_GET_SIZE), &[p]);
        call(mem, mem.rebase(MEMSET_THUNK), &[p, 0, len]);
        let name = mem.rebase(TEXTURE_NAME);
        call(
            mem,
            mem.rebase(crate::handle::bridge::MEM_SET_STR),
            &[p, name, u32::MAX],
        );
        mem.write(chr + 0x6dc, p);
        let r = call(mem, mem.rebase(TEXTURE_REMAPPING_FN), &[chr]);
        if mem.read::<u32>(chr + 0x6fc) == 0 {
            return r;
        }
        let data: u32 = mem.read(mem.read::<u32>(chr + 0x1c4).wrapping_add(0x14));
        let h = call(mem, mem.rebase(TEXANIM_SET), &[data, chr]);
        mem.write(chr + 0x700, h);
        h
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
