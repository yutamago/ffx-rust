//! Texture animation (`chr_texanim_*`, the PS2 movie-texture player the
//! characters use) and the per-character texture register fix-ups.
//!
//! `g_ChrTexAnim` (`0x02310000`): 64 entries of `0x30` bytes: `+0` the
//! source data, `+4` the work buffer, `+8`, `u8` kind at `+0xd` (0 the
//! old format, 1 the new), then zeroed words up to `+0x2f`.

/// The originals.
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    use crate::layout as l;

    /// The texture animation table.
    pub const TEXANIM: Global<u32> = Global::new(0x0231_0000);
    /// Entries in [`TEXANIM`].
    pub const TEXANIM_COUNT: u32 = 64;
    /// Bytes per entry.
    pub const TEXANIM_SIZE: u32 = 0x30;
    /// `u8` cleared by `chr_texanim_init`.
    pub const TEXANIM_844: Global<u8> = Global::new(0x0112_a844);
    /// `u8` set to `0xff` by `chr_texanim_init`.
    pub const TEXANIM_845: Global<u8> = Global::new(0x0112_a845);
    /// The two 64-byte index tables of `chr_texanim_clut_swizzle`.
    pub const SWIZZLE: [Global<u8>; 2] = [Global::new(0x00c4_2130), Global::new(0x00c4_2170)];
    /// The texture list pointer `Ch_TextureSetTex0` last worked on.
    pub const TEX0_LIST: Global<u32> = Global::new(0x023c_44a0);

    fn entry(mem: &impl Mem, i: u32) -> u32 {
        TEXANIM
            .ptr(mem)
            .addr
            .wrapping_add(i.wrapping_mul(TEXANIM_SIZE))
    }

    /// `chr_texanim_init()` (`0x0077c870`).
    pub fn chr_texanim_init(mem: &mut impl Mem) {
        for i in 0..TEXANIM_COUNT {
            let e = entry(mem, i);
            mem.write(e, [0u32; 3]);
            mem.write(e + 0xd, 0x100u16);
            mem.write(e + 0xf, [0u32; 8]);
        }
        TEXANIM_844.set(mem, 0);
        TEXANIM_845.set(mem, 0xff);
    }

    /// `chr_texanim_count(&n, &max)` (`0x0077cdc0`): entries in use (a
    /// source set) and the table size, 64.
    pub fn chr_texanim_count(mem: &mut impl Mem, n: u32, max: u32) {
        let used = (0..TEXANIM_COUNT)
            .filter(|&i| mem.read::<u32>(entry(mem, i)) != 0)
            .count();
        #[allow(clippy::cast_possible_truncation)] // at most 64
        mem.write(n, used as u32);
        mem.write(max, TEXANIM_COUNT);
    }

    /// `chr_texanim_clut_swizzle(dst, src, which)` (`0x0077ca10`): 64
    /// bytes `dst[i] = src[table[i]]`, the table chosen by `which`.
    pub fn chr_texanim_clut_swizzle(mem: &mut impl Mem, dst: u32, src: u32, which: u32) {
        let table = SWIZZLE[usize::from(which != 0)].ptr(mem).addr;
        for i in 0..64 {
            let k = u32::from(mem.read::<u8>(table + i));
            let v: u8 = mem.read(src.wrapping_add(k));
            mem.write(dst.wrapping_add(i), v);
        }
    }

    /// The memory move of both formats: the entry's source, then the
    /// `u32` at `+0xc` of each of the source's 16-byte records into the
    /// work buffer from `+at`.
    fn memory_move(mem: &mut impl Mem, src: u32, i: u32, at: u32) {
        let e = entry(mem, i);
        let work: u32 = mem.read(e + 4);
        let v: u32 = mem.read(src + 0x20);
        mem.write(e, v);
        let mut k = 0u32;
        while k < u32::from(mem.read::<u16>(src + 0xc)) {
            let recs: u32 = mem.read(src + 0x1c);
            let v: u32 = mem.read(recs.wrapping_add(16 * k + 0xc));
            mem.write(work.wrapping_add(at + 4 * k), v);
            k += 1;
        }
    }

    /// `chr_texanim_memory_move_new(src, i)` (`0x0077f970`).
    pub fn chr_texanim_memory_move_new(mem: &mut impl Mem, src: u32, i: u32) {
        memory_move(mem, src, i, 0xd28);
    }

    /// `chr_texanim_memory_move_old(src, i)` (`0x007803a0`).
    pub fn chr_texanim_memory_move_old(mem: &mut impl Mem, src: u32, i: u32) {
        memory_move(mem, src, i, 0x5b8);
    }

    /// `chr_texanim_memory_move(src, i)` (`0x0077c900`): a debug print (a
    /// no-op in this build), then the entry's format's move.
    pub fn chr_texanim_memory_move(mem: &mut impl Mem, src: u32, i: u32) {
        match mem.read::<i8>(entry(mem, i) + 0xd) {
            0 => chr_texanim_memory_move_old(mem, src, i),
            1 => chr_texanim_memory_move_new(mem, src, i),
            _ => {}
        }
    }

    /// `Ch_TextureSetImage(img, tex)` (`0x0083d1e0`): a texture from an
    /// image header (`i32` bits per pixel, data): `+0x3c` 1 up to 16 bits
    /// per pixel else 4, its low half again at `+0x36`, data at `+0x30`.
    pub fn ch_texture_set_image(mem: &mut impl Mem, img: u32, tex: u32) {
        let bpp: i32 = mem.read(img);
        mem.write(tex + 0x3c, if bpp > 0x10 { 4u32 } else { 1 });
        let v: u16 = mem.read(tex + 0x3c);
        mem.write(tex + 0x36, v);
        let data: u32 = mem.read(img + 4);
        mem.write(tex + 0x30, data);
    }

    /// `Ch_TextureSetTex0(chr)` (`0x008313f0`): each entry (32 bytes) of
    /// the texture list at `+0x6e0` (count `i32` `+0x6f4`) takes the base
    /// pointers (`u16` `+0x3a`) of its image texture (index `u16` `+2`)
    /// into bits 0..13 of `+0x14` and of its clut texture (index `u16`
    /// `+0`) into bits 7..20 of `+0x18`. The list pointer is kept in
    /// [`TEX0_LIST`].
    pub fn ch_texture_set_tex0(mem: &mut impl Mem, chr: u32) {
        let list: u32 = mem.read(chr + 0x6e0);
        TEX0_LIST.set(mem, list);
        if list == 0 {
            return;
        }
        let mut i = 0i32;
        let mut e = 0u32;
        while i < mem.read::<i32>(chr + 0x6f4) {
            let at = TEX0_LIST.get(mem).wrapping_add(e);
            let img = u32::from(mem.read::<u16>(at + 2));
            let images: u32 = mem.read(chr + l::IMAGE_TEXTURES);
            let tbp = u32::from(mem.read::<u16>(images.wrapping_add(img << 6) + 0x3a));
            let w: u32 = mem.read(at + 0x14);
            mem.write(at + 0x14, w ^ ((tbp ^ w) & 0x3fff));
            let clut = u32::from(mem.read::<u16>(TEX0_LIST.get(mem).wrapping_add(e)));
            let cluts: u32 = mem.read(chr + l::CLUT_TEXTURES);
            let cbp = u32::from(mem.read::<u16>(cluts.wrapping_add(clut << 6) + 0x3a));
            let w: u32 = mem.read(at + 0x18);
            mem.write(at + 0x18, w ^ (((cbp << 7) ^ w) & 0x1f_ff80));
            i += 1;
            e = e.wrapping_add(0x20);
        }
    }
}
