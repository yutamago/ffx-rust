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

    /// `Ch_TextureSetClut(clut, tex)` (`0x0083d1e0`): a clut texture from
    /// a clut list entry (`i32` colours, data): `+0x3c` 1 up to 16
    /// colours else 4, its low half again at `+0x36`, data at `+0x30`.
    pub fn ch_texture_set_clut(mem: &mut impl Mem, img: u32, tex: u32) {
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

    /// `Ch_TextureSetImage(img, tex)` (`0x0083d220`): an image texture
    /// from an image list entry: data (`+0xc`) at `+0x30`, width (`u16`
    /// `+8`) / 64 at `+0x34`, `u16` `+2` at `+0x36`.
    pub fn ch_texture_set_image(mem: &mut impl Mem, clut: u32, tex: u32) {
        let d: u32 = mem.read(clut + 0xc);
        mem.write(tex + 0x30, d);
        let n: u16 = mem.read(clut + 8);
        mem.write(tex + 0x34, n >> 6);
        let v: u16 = mem.read(clut + 2);
        mem.write(tex + 0x36, v);
    }

    /// The doubling loop: `out` doubles from 1 until the low half reaches
    /// `size`, the low half written each step; returns the steps (log2).
    fn pow2(mem: &mut impl Mem, size: u16, out: u32) -> u32 {
        let mut steps = 0u32;
        if size > 1 {
            let mut v = 1u32;
            loop {
                v = v.wrapping_add(v);
                #[allow(clippy::cast_possible_truncation)] // the low half
                let low = v as u16;
                mem.write(out, low);
                steps += 1;
                if size <= low {
                    break;
                }
            }
        }
        steps
    }

    /// `Ch_TextureSetRegs(img, tex)` (`0x0083d250`): a texture from an
    /// image header: its two first bytes swapped into `u16`s at `+0` and
    /// `+2`, width and height at `+0xc`, `+0xe`, the bytes `+8`, `+9` at
    /// `+0x11`, `+0x12`, the powers of two at or above them at `+8` and
    /// `+0xa`, and the GS register fields at `+0x14..+0x1f` (buffer width,
    /// log2 sizes, `u16` `+2` as the pixel format; fixed bits).
    pub fn ch_texture_set_regs(mem: &mut impl Mem, img: u32, tex: u32) {
        let b: [u8; 2] = mem.read(img);
        mem.write(tex, [u16::from(b[1]), u16::from(b[0])]);
        let (w, h): (u16, u16) = (mem.read(img + 4), mem.read(img + 6));
        mem.write(tex + 0xc, [w, h]);
        let v: [u8; 2] = mem.read(img + 8);
        mem.write(tex + 0x11, v);
        mem.write(tex + 8, 0x0001_0001u32);
        let lh = pow2(mem, h, tex + 0xa);
        let lw = pow2(mem, w, tex + 8);
        let x: u32 = mem.read(tex + 0x14);
        let bw = u32::from(mem.read::<u16>(tex + 0xc)) & 0xfc0;
        let r14 = ((bw | (lw & 0xf) << 0x12) << 8) | (x & 0xc3f0_3fff);
        let y: u32 = mem.read(tex + 0x18);
        mem.write(tex + 0x18, y ^ ((y ^ lh) & 0xf));
        mem.write(tex + 0x14, r14);
        let psm = u32::from(mem.read::<u16>(img + 2)) << 0x14;
        let y: u32 = mem.read(tex + 0x18);
        mem.write(tex + 0x14, r14 ^ ((psm ^ r14) & 0x03f0_0000));
        let z: u32 = mem.read(tex + 0x1c);
        mem.write(tex + 0x18, (y & 0x801f_ff9f) | 0x10);
        mem.write(tex + 0x1c, (z & 0xffff_fff9) | 1);
    }

    /// Where `Ch_TextureReMapping` put the clut textures.
    pub const REMAP_CLUTS: Global<u32> = Global::new(0x0130_19c8);
    /// Where `Ch_TextureReMapping` put the image textures.
    pub const REMAP_IMAGES: Global<u32> = Global::new(0x0130_19c4);

    /// `Ch_TextureReMapping(chr)` (`0x0083cdd0`): lays the texture data's
    /// textures (versions from `0x1029`) out in the character's texture
    /// buffer (`+0x6dc`): the texture list (32 bytes each, count `u16`
    /// `+4`), then the clut textures (64 bytes, `u16` `+8`), then the
    /// image textures (64 bytes, `u16` `+0xc`); sets them up
    /// (`Ch_TextureSetClut`, `Ch_TextureSetImage`, `Ch_TextureSetRegs`)
    /// and sums their `+0x36` into `+0x6ea` and `+0x6f2`; re-enables the
    /// clut; from version `0x1126` takes `+0x20` into `+0x6fc`.
    pub fn ch_texture_re_mapping(mem: &mut impl Mem, chr: u32) {
        let t: u32 = mem.read(crate::chr::bridge::blk(mem, chr) + l::BLK_TEX);
        if t == 0 || mem.read::<u16>(t + 0x10) < 0x1029 {
            return;
        }
        let buf: u32 = mem.read(chr + 0x6dc);
        if buf == 0 {
            return;
        }
        let list = u32::from(mem.read::<u16>(t + 4)) << 5;
        let cluts = list.wrapping_add(buf);
        let images = (u32::from(mem.read::<u16>(t + 8)) << 6)
            .wrapping_add(list)
            .wrapping_add(buf);
        mem.write(chr + 0x6e0, buf);
        mem.write(chr + l::CLUT_TEXTURES, cluts);
        mem.write(chr + l::IMAGE_TEXTURES, images);
        let n: u16 = mem.read(t + 8);
        mem.write(chr + l::CLUT_TEXTURE_COUNT, n);
        let n: u16 = mem.read(t + 0xc);
        mem.write(chr + l::IMAGE_TEXTURE_COUNT, n);
        let n = u32::from(mem.read::<u16>(t + 4));
        mem.write(chr + 0x6f4, n);
        REMAP_CLUTS.set(mem, cluts);
        REMAP_IMAGES.set(mem, images);
        let mut sum = 0u32;
        let mut i = 0u32;
        while i < u32::from(mem.read::<u16>(t + 8)) {
            let src = mem.read::<u32>(t + 0x18).wrapping_add(8 * i);
            let dst = REMAP_CLUTS.get(mem).wrapping_add(0x40 * i);
            ch_texture_set_clut(mem, src, dst);
            let at = REMAP_CLUTS.get(mem).wrapping_add(0x40 * i) + 0x36;
            sum = sum.wrapping_add(u32::from(mem.read::<u16>(at)));
            i += 1;
        }
        #[allow(clippy::cast_possible_truncation)] // the low half
        mem.write(chr + 0x6ea, sum as u16);
        let images = if i > 0 { REMAP_IMAGES.get(mem) } else { images };
        let mut sum = 0u32;
        let mut i = 0u32;
        while i < u32::from(mem.read::<u16>(t + 0xc)) {
            let src = mem.read::<u32>(t + 0x1c).wrapping_add(16 * i);
            let dst = images.wrapping_add(0x40 * i);
            ch_texture_set_image(mem, src, dst);
            sum = sum.wrapping_add(u32::from(mem.read::<u16>(dst + 0x36)));
            i += 1;
        }
        #[allow(clippy::cast_possible_truncation)] // the low half
        mem.write(chr + 0x6f2, sum as u16);
        let mut i = 0u32;
        while i < u32::from(mem.read::<u16>(t + 4)) {
            let src = mem.read::<u32>(t + 0x14).wrapping_add(16 * i);
            ch_texture_set_regs(mem, src, buf.wrapping_add(0x20 * i));
            i += 1;
        }
        if mem.read::<u8>(chr + l::CLUT_ENABLED) != 0 {
            crate::texture::bridge::ch_clut_enable(mem, chr, 1);
        }
        if mem.read::<u16>(t + 0x10) >= 0x1126 {
            let v: u32 = mem.read(t + 0x20);
            mem.write(chr + 0x6fc, v);
        }
    }

    /// `Sg_GsloadSetBase(tex)` (`0x0083d370`), the `main` module's, as the
    /// texture functions use it: bits 0..13 of `+0x24` from `u16` `+0x3a`.
    fn gsload_set_base(mem: &mut impl Mem, tex: u32) {
        let b = u32::from(mem.read::<u16>(tex + 0x3a));
        let w: u32 = mem.read(tex + 0x24);
        mem.write(tex + 0x24, w ^ ((b ^ w) & 0x3fff));
    }

    /// `Ch_TextureSetBase(tex, n, base, i)` (`0x00832050`): texture `i`
    /// (64 bytes each) gets base `base` (`u16` `+0x38` and `+0x3a`, then
    /// its register); returns the next free base (`base + u16 +0x36`, or
    /// -1 for -1). With `i` = -1 all `n` textures, one after another.
    pub fn ch_texture_set_base(mem: &mut impl Mem, tex: u32, n: i32, base: u32, i: u32) -> u32 {
        let set = |mem: &mut _, t: u32, b: u32| {
            #[allow(clippy::cast_possible_truncation)] // the low half
            let h = b as u16;
            Mem::write(mem, t + 0x38, [h, h]);
        };
        if i != u32::MAX {
            let t = tex.wrapping_add(i << 6);
            set(mem, t, base);
            gsload_set_base(mem, t);
            if base == u32::MAX {
                return base;
            }
            return u32::from(mem.read::<u16>(t + 0x36)).wrapping_add(base);
        }
        let mut base = base;
        for k in 0..n.max(0).cast_unsigned() {
            let t = tex.wrapping_add(0x40 * k);
            set(mem, t, base);
            gsload_set_base(mem, t);
            if base != u32::MAX {
                base = base.wrapping_add(u32::from(mem.read::<u16>(t + 0x36)));
            }
        }
        base
    }

    /// `Ch_TextureSetImageBase(chr, i, base)` (`0x00831390`): on the clut
    /// textures (`+0x6e4`, count `i16` `+0x6e8`).
    pub fn ch_texture_set_image_base(mem: &mut impl Mem, chr: u32, i: u32, base: u32) -> u32 {
        let t: u32 = mem.read(chr + l::CLUT_TEXTURES);
        let n = i32::from(mem.read::<i16>(chr + l::CLUT_TEXTURE_COUNT));
        ch_texture_set_base(mem, t, n, base, i)
    }

    /// `Ch_TextureSetClutBase(chr, i, base)` (`0x008313c0`): on the image
    /// textures (`+0x6ec`, count `i16` `+0x6f0`).
    pub fn ch_texture_set_clut_base(mem: &mut impl Mem, chr: u32, i: u32, base: u32) -> u32 {
        let t: u32 = mem.read(chr + l::IMAGE_TEXTURES);
        let n = i32::from(mem.read::<i16>(chr + l::IMAGE_TEXTURE_COUNT));
        ch_texture_set_base(mem, t, n, base, i)
    }

    /// `ch_TextureLoad(tex, n)` (`0x00831f00`): each of the `n` textures
    /// with a base (`u16` `+0x38` not `0xffff`) takes it as `+0x3a` and
    /// into its register; the DMA uploads around it do nothing in this
    /// build.
    pub fn ch_texture_load(mem: &mut impl Mem, tex: u32, n: i32) {
        for k in 0..n.max(0).cast_unsigned() {
            let t = tex.wrapping_add(0x40 * k);
            let b: u16 = mem.read(t + 0x38);
            if b != 0xffff {
                mem.write(t + 0x3a, b);
                gsload_set_base(mem, t);
            }
        }
    }

    /// The specular texture data (`Ch_TextureSpecInitAll`).
    pub const SPECULAR_TEX: Global<u32> = Global::new(0x00c4_deac);
    /// The specular clut texture.
    pub const SPECULAR_CLUT: Global<u32> = Global::new(0x0130_1a20);
    /// The specular image texture.
    pub const SPECULAR_IMAGE: Global<u32> = Global::new(0x0130_19e0);
    /// The specular texture list entry.
    pub const SPECULAR_REGS: Global<u32> = Global::new(0x0130_1a60);

    /// `Ch_TextureSpecInitAll()` (`0x0083d140`): the specular texture
    /// data mapped and set up (image, clut, list entry), with the fixed
    /// bases `0x2ea0` and `0x2ef8`.
    pub fn ch_texture_spec_init_all(mem: &mut impl Mem) {
        let t = SPECULAR_TEX.get(mem);
        crate::mapping::bridge::ch_tex_data_mapping(mem, t);
        let clut = SPECULAR_CLUT.ptr(mem).addr;
        let image = SPECULAR_IMAGE.ptr(mem).addr;
        let regs = SPECULAR_REGS.ptr(mem).addr;
        let src: u32 = mem.read(t + 0x18);
        ch_texture_set_clut(mem, src, clut);
        let src: u32 = mem.read(t + 0x1c);
        ch_texture_set_image(mem, src, image);
        let src: u32 = mem.read(t + 0x14);
        ch_texture_set_regs(mem, src, regs);
        mem.write(image + 0x3a, 0x2ea0u16);
        mem.write(clut + 0x3a, 0x2ef8u16);
        gsload_set_base(mem, image);
        gsload_set_base(mem, clut);
        let a = u32::from(mem.read::<u16>(image + 0x3a));
        let w: u32 = mem.read(regs + 0x14);
        mem.write(regs + 0x14, w ^ ((a ^ w) & 0x3fff));
        let c = u32::from(mem.read::<u16>(clut + 0x3a)) << 7;
        let w: u32 = mem.read(regs + 0x18);
        mem.write(regs + 0x18, w ^ ((c ^ w) & 0x1f_ff80));
    }

    /// `Ch_TextureSetAnimTimer(chr, v)` (`0x0083d070`): with a texture
    /// animation (`+0x6fc`), its entry's (`+0x700`) byte `+0xe`
    /// (`tex_anim_timer`).
    pub fn ch_texture_set_anim_timer(mem: &mut impl Mem, chr: u32, v: u32) {
        if mem.read::<u32>(chr + 0x6fc) == 0 {
            return;
        }
        let i: u32 = mem.read(chr + 0x700);
        #[allow(clippy::cast_possible_truncation)] // the low byte
        mem.write(entry(mem, i) + 0xe, v as u8);
    }

    /// `sg_getdbname`'s strings (kinds 0..6, the last for anything else):
    /// the addresses of its `.rdata` names, by its jump table's order.
    const DB_NAMES: [u32; 8] = [
        0x00b5_9334,
        0x00b5_9338,
        0x00b5_933c,
        0x00b5_9340,
        0x00b5_9344,
        0x00b5_9348,
        0x00b5_9330,
        0x00b8_fbeb,
    ];
    /// `sg_getdbname`'s byte table: which of [`DB_NAMES`] for each of
    /// `0..=0x77`.
    const DB_INDEX: Global<u8> = Global::new(0x0082_9d04);

    /// `ch_create_section(id)` (`0x00838150`): `sg_getdbname` of the kind
    /// (`id` bits 12..15): the folder name of a character kind.
    pub fn ch_create_section(mem: &impl Mem, id: u32) -> u32 {
        let kind = (id.cast_signed() >> 12).cast_unsigned() & 0xf;
        let i = DB_INDEX.ptr(mem).at(kind).get(mem);
        Global::<u8>::new(DB_NAMES[usize::from(i)]).ptr(mem).addr
    }

    /// Where `ch_create_name` prints.
    pub const CREATE_NAME: Global<u8> = Global::new(0x0130_0a00);
    /// `"0000"`, `ch_create_name`'s name for id 0.
    pub const NAME_0000: Global<u8> = Global::new(0x00b5_9a08);

    /// `sg_getdbtype(kind)` (`0x00829e10`, the main module's): the letter
    /// of a character kind, `-` past 6.
    const fn db_type(kind: u32) -> u8 {
        match kind {
            0 => b'c',
            1 => b'm',
            2 => b'n',
            3 => b's',
            4 => b'w',
            5 => b'f',
            6 => b'k',
            _ => b'-',
        }
    }

    /// `ch_create_name(id)` (`0x00838100`): `"0000"` for 0, else
    /// `sprintf("%c%03d", kind letter, id & 0xfff)` into its buffer; returns
    /// the string.
    pub fn ch_create_name(mem: &mut impl Mem, id: u32) -> u32 {
        if id == 0 {
            return NAME_0000.ptr(mem).addr;
        }
        let kind = (id.cast_signed() >> 12).cast_unsigned() & 0xf;
        let s = format!("{}{:03}", char::from(db_type(kind)), id & 0xfff);
        let at = CREATE_NAME.ptr(mem).addr;
        crate::handle::bridge::put_c_string(mem, at, s.as_bytes());
        at
    }

    /// `Ch_TextureLoadClut(chr, _)` (`0x008311d0`): `ch_TextureLoad` on
    /// the clut textures (`+0x6e4`, count `i16` `+0x6e8`).
    pub fn ch_texture_load_clut(mem: &mut impl Mem, chr: u32) {
        let t: u32 = mem.read(chr + 0x6e4);
        let n = i32::from(mem.read::<i16>(chr + 0x6e8));
        ch_texture_load(mem, t, n);
    }

    /// Where `ch_create_chrname` prints.
    pub const CREATE_CHRNAME: Global<u8> = Global::new(0x0130_09f4);
    /// Where `ch_create_chrpath` prints.
    pub const CREATE_CHRPATH: Global<u8> = Global::new(0x0130_1758);

    /// The `"<section>"`, kind letter and number of an id, as
    /// `ch_create_chrname` and `ch_create_chrpath` print them.
    fn name_parts(mem: &impl Mem, id: u32) -> (Vec<u8>, u8, u32) {
        let kind = (id.cast_signed() >> 12).cast_unsigned() & 0xf;
        let section = crate::handle::bridge::c_string(mem, ch_create_section(mem, id));
        (section, db_type(kind), id & 0xfff)
    }

    fn print_parts(mem: &mut impl Mem, at: u32, head: &[u8], sep: u8, id: u32) -> u32 {
        let (section, letter, n) = name_parts(mem, id);
        let mut s = head.to_vec();
        s.extend(section);
        s.push(sep);
        s.push(letter);
        s.extend(format!("{n:03}").bytes());
        crate::handle::bridge::put_c_string(mem, at, &s);
        at
    }

    /// `ch_create_chrname(id)` (`0x00838060`): `sprintf("%s:%c%03d",
    /// section, kind letter, id & 0xfff)` into its buffer; returns it.
    pub fn ch_create_chrname(mem: &mut impl Mem, id: u32) -> u32 {
        let at = CREATE_CHRNAME.ptr(mem).addr;
        print_parts(mem, at, b"", b':', id)
    }

    /// `ch_create_chrpath(id)` (`0x008380b0`): `sprintf(
    /// "/ffx/proj2/chr/data/%s/%c%03d", ...)` into its buffer; returns it.
    pub fn ch_create_chrpath(mem: &mut impl Mem, id: u32) -> u32 {
        let at = CREATE_CHRPATH.ptr(mem).addr;
        print_parts(mem, at, b"/ffx/proj2/chr/data/", b'/', id)
    }

    /// `tex_anim_no_set` (`0x0077e940`, the renderer's).
    pub const TEX_ANIM_NO_SET: u32 = 0x0077_e940;

    /// `Ch_TextureSetAnim(chr, a, b)` (`0x0083d040`): with texture
    /// animations (`+0x6fc`), `tex_anim_no_set(+0x700, a, b, name +4,
    /// instance +0x830)`. Returns what the original leaves in EAX: that
    /// call's result, else `chr`.
    pub fn ch_texture_set_anim<M: Mem>(
        mem: &mut M,
        chr: u32,
        a: u32,
        b: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        if mem.read::<u32>(chr + 0x6fc) == 0 {
            return chr;
        }
        let args = [
            mem.read(chr + 0x700),
            a,
            b,
            mem.read(chr + 4),
            mem.read(chr + 0x830),
        ];
        call(mem, mem.rebase(TEX_ANIM_NO_SET), &args)
    }

    /// `i8` texture animation switch read by `Ch_TextureLoadAnim`.
    pub const TEXANIM_ON: Global<u8> = Global::new(0x0130_07eb);
    /// The DMA channel `Ch_TextureLoadAnim` sends on.
    pub const TEXANIM_DMA_CHANNEL: Global<u32> = Global::new(0x023c_bbf8);
    /// `g_GsRegisterSink`: bit 8 set while the GS is busy.
    pub const GS_REGISTER_SINK: Global<u32> = Global::new(0x0230_56e8);
    /// `Ch_TextureGetImageAddr`, `chr_texanim_draw`, `FlushCache`,
    /// `sceDmaSend`.
    pub const TEXTURE_GET_IMAGE_ADDR: u32 = 0x0083_0d90;
    pub const CHR_TEXANIM_DRAW: u32 = 0x0077_c830;
    pub const FLUSH_CACHE: u32 = 0x0062_f4c0;
    pub const DMA_SEND: u32 = 0x0062_f710;
    /// `ch_TextureLoad(tex, n, x)` (`0x00831f00`).
    pub const TEXTURE_LOAD: u32 = 0x0083_1f00;
    /// Bytes of scratch `ch_texture_load_anim` needs (the original's stack
    /// packet buffer).
    pub const LOAD_ANIM_SCRATCH: u32 = 0xa00;

    /// `Ch_TextureLoadAnim(chr, scratch)` (`0x00831110`): with texture
    /// animations (`+0x6fc`) and [`TEXANIM_ON`], `chr_texanim_draw(+0x700,
    /// scratch, Ch_TextureGetImageAddr(chr, 0))` builds a packet in
    /// `scratch` ([`LOAD_ANIM_SCRATCH`] bytes; the original's stack); if it
    /// wrote anything, an end tag (`0x70000000`, three 0 words) at its end,
    /// `FlushCache(0)`, and the packet sent (`sceDmaSend(channel,
    /// scratch)`) between waits for the GS.
    pub fn ch_texture_load_anim<M: Mem>(
        mem: &mut M,
        chr: u32,
        scratch: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        if mem.read::<u32>(chr + 0x6fc) == 0 || TEXANIM_ON.get(mem) == 0 {
            return;
        }
        let image = call(mem, mem.rebase(TEXTURE_GET_IMAGE_ADDR), &[chr, 0]);
        let anim: u32 = mem.read(chr + 0x700);
        let end = call(mem, mem.rebase(CHR_TEXANIM_DRAW), &[anim, scratch, image]);
        if end == scratch {
            return;
        }
        mem.write(end, [0x7000_0000u32, 0, 0, 0]);
        call(mem, mem.rebase(FLUSH_CACHE), &[0]);
        while GS_REGISTER_SINK.get(mem) & 0x100 != 0 {}
        let ch = TEXANIM_DMA_CHANNEL.get(mem);
        call(mem, mem.rebase(DMA_SEND), &[ch, scratch]);
        while GS_REGISTER_SINK.get(mem) & 0x100 != 0 {}
    }

    /// `Ch_TextureLoadImage(chr, x)` (`0x00831300`): `ch_TextureLoad` on
    /// the image textures (`+0x6ec`, count `i16` `+0x6f0`), then
    /// [`ch_texture_load_anim`].
    pub fn ch_texture_load_image<M: Mem>(
        mem: &mut M,
        chr: u32,
        x: u32,
        scratch: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let t: u32 = mem.read(chr + 0x6ec);
        let n = i32::from(mem.read::<i16>(chr + 0x6f0));
        call(mem, mem.rebase(TEXTURE_LOAD), &[t, n.cast_unsigned(), x]);
        ch_texture_load_anim(mem, chr, scratch, call);
    }

    /// The character texture animation table (`chr_texanim_*`): 64
    /// entries of 0x30 bytes: `+0` source, `+4` work block, `+8`, `u8`
    /// kind `+0xc` (1 new form, 2 old).
    pub const TEX_ANIMS: Global<u32> = Global::new(0x0231_0000);
    /// `g_ChrTexAnim844` and the byte after it.
    pub const TEX_ANIM_844: Global<[u8; 2]> = Global::new(0x0112_a844);
    /// `movie_err(message, file, line)` (`0x0076efa0`).
    pub const MOVIE_ERR: u32 = 0x0076_efa0;

    /// `chr_texanim_del(id)` (`0x0077c770`): an id outside 0..64 is
    /// reported (`movie_err("id >= 0" / "id < CHR_TEX_ANIM_MAX",
    /// "..\\program\\lib\\src\\movie_str.c", 0xb39 / 0xb3a)`, then used
    /// anyway). A kind-1 entry's work block is shut down (the function at
    /// `+4` of the table at its `+0x460f8`), freed, and the two bytes at
    /// [`TEX_ANIM_844`] reset to 0, 0xff; a kind-2 entry's block is
    /// freed; the block, source and `+8` cleared. Returns what the
    /// original leaves in EAX.
    pub fn chr_texanim_del<M: Mem>(
        mem: &mut M,
        id: i32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let err = mem.rebase(MOVIE_ERR);
        let file = mem.rebase(0x00b5_40a8);
        if id < 0 {
            call(mem, err, &[mem.rebase(0x00b5_44cc), file, 0xb39]);
        }
        if id >= 0x40 {
            call(mem, err, &[mem.rebase(0x00b5_44d4), file, 0xb3a]);
        }
        let e = TEX_ANIMS
            .ptr(mem)
            .addr
            .wrapping_add(0x30u32.wrapping_mul(id.cast_unsigned()));
        let kind = i32::from(mem.read::<i8>(e + 0xc));
        let free = mem.rebase(crate::chr::bridge::USER_FREE);
        let block: u32 = mem.read(e + 4);
        let eax = match kind {
            1 if block != 0 => {
                let table: u32 = mem.read(block.wrapping_add(0x460f8));
                let f: u32 = mem.read(table.wrapping_add(4));
                call(mem, f, &[]);
                let block: u32 = mem.read(e + 4);
                let r = call(mem, free, &[block]);
                TEX_ANIM_844.set(mem, [0, 0xff]);
                mem.write(e + 4, 0u32);
                r
            }
            2 if block != 0 => {
                let r = call(mem, free, &[block]);
                mem.write(e + 4, 0u32);
                r
            }
            1 | 2 => 0,
            k => (k - 2).cast_unsigned(),
        };
        mem.write(e, 0u32);
        mem.write(e + 8, 0u32);
        eax
    }

    /// `TexAnim_StartNew(id, k, mode)` (`0x0077fc90`), `TexAnim_ResetPos(id,
    /// k, x)` (`0x0077fda0`), `TexAnim_StartOld(id, k, mode)` (`0x00780750`)
    /// and `tex_anim_pos_set_old(id, k, x)` (`0x007808f0`).
    pub const START_NEW: u32 = 0x0077_fc90;
    pub const RESET_POS: u32 = 0x0077_fda0;
    pub const START_OLD: u32 = 0x0078_0750;
    pub const POS_SET_OLD: u32 = 0x0078_08f0;

    /// The two forms of `chr_texanim_set_*`.
    struct SetForm {
        size: u32,
        /// Offsets zeroed in each of the first 8 or 64 slots (0x34 bytes).
        zero: &'static [u32],
        slots: u32,
        /// Where the source's slot modes go (from its `+0xc`) and how many.
        modes: u32,
        mode_len: u32,
        /// The slots' position pair, the source's stride per slot.
        pos: u32,
        stride: u32,
        handle: u32,
        count: u32,
        extra: u32,
        start: u32,
        reset: u32,
        old: bool,
    }

    /// `chr_texanim_set_new` / `_old`: a work block for entry `id` of
    /// [`TEX_ANIMS`] (its `+8` and, new form, the dwords at `+0xf`..`+0x2b`
    /// cleared); slot fields zeroed, the source's slot modes copied in,
    /// the handle stored. Each slot with a mode (not -1) is started
    /// (mode 0 as 2, 1/3 as 1, 2 as 3, 4 as 0; others untouched) and
    /// positioned; a mode-2 slot whose mode survives gets the source's
    /// position pair (its slot at `[src+8]`, floats through x87). The
    /// count of slots up to the last used one (at least the source's `+1`
    /// byte) is stored, then each of `def`'s `u16` `+0xc` entries (0x10
    /// bytes at `+0x1c`) gives a dword (`+0xc`) and two halves (`+8`,
    /// `+0xa`).
    fn chr_texanim_set_form<M: Mem>(
        mem: &mut M,
        f: &SetForm,
        id: u32,
        handle: u32,
        def: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let e = TEX_ANIMS
            .ptr(mem)
            .addr
            .wrapping_add(0x30u32.wrapping_mul(id));
        let src: u32 = mem.read(e);
        let malloc = mem.rebase(crate::handle::bridge::USER_MALLOC);
        let buf = call(mem, malloc, &[f.size]);
        mem.write(e + 4, buf);
        mem.write(e + 8, 0u32);
        if !f.old {
            for k in 0..8 {
                mem.write(e + 0xf + 4 * k, 0u32);
            }
        }
        for k in 0..f.slots {
            for &o in f.zero {
                mem.write(buf.wrapping_add(o + 0x34 * k), 0u32);
            }
        }
        for k in 0..f.mode_len / 4 {
            let v: u32 = mem.read(src.wrapping_add(0xc + 4 * k));
            mem.write(buf.wrapping_add(f.modes + 4 * k), v);
        }
        mem.write(buf.wrapping_add(f.handle), handle);
        if !f.old {
            mem.write(buf.wrapping_add(0xd48), 0u32);
        }
        let mut most = if f.old {
            i32::from(mem.read::<i8>(src.wrapping_add(1)))
        } else {
            i32::from(mem.read::<u8>(src.wrapping_add(1)))
        };
        let start = mem.rebase(f.start);
        let reset = mem.rebase(f.reset);
        for k in 0..f.mode_len {
            let mode = mem.read::<i8>(buf.wrapping_add(f.modes + k));
            if mode == -1 {
                continue;
            }
            let (m, r) = match mode {
                0 => (Some(2), 0),
                1 | 3 => (Some(1), u32::from(!f.old)),
                2 => (Some(3), 0),
                4 => (Some(0), 0),
                _ => (None, 0),
            };
            if let Some(m) = m {
                call(mem, start, &[id, k, m]);
                call(mem, reset, &[id, k, r]);
                if mode == 2 && mem.read::<i8>(buf.wrapping_add(f.modes + k)) != -1 {
                    let at: u32 = mem.read(src.wrapping_add(8));
                    let p = at.wrapping_add(f.stride * k).wrapping_add(src);
                    let d = buf.wrapping_add(f.pos + 0x34 * k);
                    for w in 0..2 {
                        let v = crate::x87::load(mem.read::<f32>(p.wrapping_add(4 * w)));
                        mem.write(d.wrapping_add(4 * w), v);
                    }
                }
            }
            most = most.max(k.cast_signed() + 1);
        }
        mem.write(buf.wrapping_add(f.count), most);
        let mut eax = 0u32;
        let mut i = 0u32;
        while i < u32::from(mem.read::<u16>(def + 0xc)) {
            let t: u32 = mem.read(def + 0x1c);
            let v: u32 = mem.read(t.wrapping_add(0x10 * i + 0xc));
            mem.write(buf.wrapping_add(f.extra + 4 * i), v);
            let t: u32 = mem.read(def + 0x1c);
            let v: u16 = mem.read(t.wrapping_add(0x10 * i + 8));
            mem.write(buf.wrapping_add(f.extra + 0x10 + 2 * i), v);
            let t: u32 = mem.read(def + 0x1c);
            let v: u16 = mem.read(t.wrapping_add(0x10 * i + 0xa));
            mem.write(buf.wrapping_add(f.extra + 0x18 + 2 * i), v);
            eax = u32::from(mem.read::<u16>(def + 0xc));
            i += 1;
        }
        if f.old { buf } else { eax }
    }

    /// `chr_texanim_set_new(id, handle, def)` (`0x0077f9c0`): the new form
    /// (0xd4c bytes, 32 slots); returns `def`'s count (what EAX holds).
    pub fn chr_texanim_set_new<M: Mem>(
        mem: &mut M,
        id: u32,
        handle: u32,
        def: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        const FORM: SetForm = SetForm {
            size: 0xd4c,
            zero: &[0x8, 0xc, 0x1c, 0x20, 0x24, 0x2c, 0x30, 0x34, 0x38],
            slots: 64,
            modes: 0xd08,
            mode_len: 32,
            pos: 0x2c,
            stride: 0x110,
            handle: 4,
            count: 0,
            extra: 0xd28,
            start: START_NEW,
            reset: RESET_POS,
            old: false,
        };
        chr_texanim_set_form(mem, &FORM, id, handle, def, call)
    }

    /// `chr_texanim_set_old(id, handle, def)` (`0x007803f0`): the old form
    /// (0x5d8 bytes, 16 slots, positioned with 0); returns the block.
    pub fn chr_texanim_set_old<M: Mem>(
        mem: &mut M,
        id: u32,
        handle: u32,
        def: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        const FORM: SetForm = SetForm {
            size: 0x5d8,
            zero: &[0x40c, 0x41c, 0x420, 0x424, 0x42c, 0x430, 0x434, 0x438],
            slots: 8,
            modes: 0x5a8,
            mode_len: 16,
            pos: 0x42c,
            stride: 0x90,
            handle: 0x404,
            count: 0x400,
            extra: 0x5b8,
            start: START_OLD,
            reset: POS_SET_OLD,
            old: true,
        };
        chr_texanim_set_form(mem, &FORM, id, handle, def, call)
    }

    /// The import slots `chr_texanim_main_sub_old` calls through: `rand`,
    /// `memcpy`.
    pub const RAND_SLOT: u32 = 0x00b0_c420;
    pub const MEMCPY_SLOT: u32 = 0x00b0_c364;

    /// `chr_texanim_main_sub_old(id)` (`0x0077ff00`): one step of an old
    /// form entry (source kind byte 2) of [`TEX_ANIMS`]: for each of its
    /// work block's slots (count `+0x400`, 0x34 bytes from `+0x40c`) with
    /// a mode (byte `+0x5a8 + k`, not -1), its source slot (0x90 bytes at
    /// `[src+8]`) gives the frame list for the entry's track (`+8`; frames
    /// of 4 bytes at its `+0x10`, `i16` count `+0x14`; none sets the
    /// slot's done flag `+0x12`). By the slot's play mode (`+0x10`): 1/4
    /// show frame `+0`; 2 blinks (state `+0xe`: wait 60-149, 4 steps per
    /// frame, then back); 3 and others step the frame when a counter
    /// (`+8`) passes the frame's `i16` time plus a random wait (`rand` &
    /// 0xff × frame byte 3 / 256, set when it steps; the step keeps the
    /// old frame for this pass), mode 3 also scrolling its offsets (`+0x20`,
    /// `+0x24` by `+0x18`, `+0x1c`, wrapped into the frame's rectangle,
    /// floats through x87). The frame's rectangle (12 bytes at `[src+4]`,
    /// index byte 2) and the slot's placement are stored; mode 3 then
    /// copies the rectangle's rows into the destination (`memcpy`; the
    /// block's `+0x5b8` and `+0x5c8` by the slot's `+0xc`), starting at the
    /// row of its scroll `y`. Returns what EAX holds: the id with no source,
    /// kind − 2 for another kind, the track with no slots, else the block.
    #[allow(clippy::too_many_lines)] // one step of every slot
    pub fn chr_texanim_main_sub_old<M: Mem>(
        mem: &mut M,
        id: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let e = TEX_ANIMS
            .ptr(mem)
            .addr
            .wrapping_add(0x30u32.wrapping_mul(id));
        let src: u32 = mem.read(e);
        if src == 0 {
            return id;
        }
        let kind = i32::from(mem.read::<i8>(src));
        if kind != 2 {
            return (kind - 2).cast_unsigned();
        }
        let track: u32 = mem.read(e + 8);
        let buf: u32 = mem.read(e + 4);
        let rects = src.wrapping_add(mem.read::<u32>(src.wrapping_add(4)));
        let rd16 = |mem: &M, a: u32| i32::from(mem.read::<i16>(a));
        let rand = |mem: &mut M, call: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32| {
            let f: u32 = mem.read(mem.rebase(RAND_SLOT));
            call(mem, f, &[])
        };
        if mem.read::<i32>(buf.wrapping_add(0x400)) <= 0 {
            return track;
        }
        let mut k = 0u32;
        while k.cast_signed() < mem.read::<i32>(buf.wrapping_add(0x400)) {
            let s = buf.wrapping_add(0x420 + 0x34 * k);
            if mem.read::<i8>(buf.wrapping_add(0x5a8 + k)) == -1 {
                k += 1;
                continue;
            }
            let ss = src
                .wrapping_add(mem.read::<u32>(src.wrapping_add(8)))
                .wrapping_add(0x90 * k);
            let list = ss.wrapping_add(track.wrapping_mul(8));
            let cnt = rd16(mem, list.wrapping_add(0x14));
            if cnt == 0 {
                mem.write(s.wrapping_add(6), 1u16);
                k += 1;
                continue;
            }
            let base = src.wrapping_add(mem.read::<u32>(list.wrapping_add(0x10)));
            let frame_at =
                |mem: &M| base.wrapping_add(mem.read::<u32>(s.wrapping_sub(0x14)).wrapping_mul(4));
            let mode = rd16(mem, s.wrapping_add(4));
            let fr = match mode {
                1 | 4 => frame_at(mem),
                2 => {
                    match rd16(mem, s.wrapping_add(2)) {
                        0 => {
                            let r = rand(mem, call).cast_signed();
                            mem.write(s.wrapping_add(2), 1u16);
                            mem.write(s.wrapping_sub(0x14), 0u32);
                            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                            mem.write(s, (r.wrapping_rem(0x5a) + 0x3c) as u16);
                        }
                        1 => {
                            let t = mem.read::<i16>(s).wrapping_sub(1);
                            mem.write(s, t);
                            if t < 0 {
                                mem.write(s, 0x0002_0004u32);
                            }
                        }
                        2 => {
                            let t = mem.read::<i16>(s).wrapping_sub(1);
                            mem.write(s, t);
                            if t < 0 {
                                let f = mem.read::<i32>(s.wrapping_sub(0x14)).wrapping_add(1);
                                mem.write(s.wrapping_sub(0x14), f);
                                if f < cnt {
                                    mem.write(s, 1u16);
                                } else {
                                    mem.write(s.wrapping_sub(0x14), f.wrapping_sub(1));
                                    mem.write(s, 0u32);
                                }
                            }
                        }
                        _ => {}
                    }
                    frame_at(mem)
                }
                _ => {
                    let mut fr = frame_at(mem);
                    if mem.read::<i16>(fr) >= 0 {
                        let c = mem.read::<i16>(s.wrapping_sub(4)).wrapping_add(1);
                        mem.write(s.wrapping_sub(4), c);
                        if rd16(mem, fr) + rd16(mem, s.wrapping_sub(2)) < i32::from(c) {
                            mem.write(s.wrapping_sub(4), 0u16);
                            let f = mem.read::<i32>(s.wrapping_sub(0x14)).wrapping_add(1);
                            mem.write(s.wrapping_sub(0x14), f);
                            if f >= cnt {
                                fr = base;
                                mem.write(s.wrapping_sub(0x14), 0u32);
                            }
                            let r = rand(mem, call) & 0xff;
                            let w =
                                r.wrapping_mul(u32::from(mem.read::<u8>(fr.wrapping_add(3)))) >> 8;
                            #[allow(clippy::cast_possible_truncation)]
                            mem.write(s.wrapping_sub(2), w as u16);
                        }
                    }
                    if mode == 3 {
                        #[allow(clippy::cast_possible_truncation)] // fstp m32
                        let add = |a: f32, b: f32| (f64::from(a) + f64::from(b)) as f32;
                        let x = add(
                            mem.read(s.wrapping_add(0xc)),
                            mem.read(s.wrapping_add(0x14)),
                        );
                        mem.write(s.wrapping_add(0x14), x);
                        let y = add(
                            mem.read(s.wrapping_add(0x10)),
                            mem.read(s.wrapping_add(0x18)),
                        );
                        mem.write(s.wrapping_add(0x18), y);
                        let r =
                            rects.wrapping_add(12 * u32::from(mem.read::<u8>(fr.wrapping_add(2))));
                        for (pos, dim) in [(0x14u32, 4u32), (0x18, 6)] {
                            let v = if pos == 0x14 { x } else { y };
                            if v < 0.0 {
                                let d = f64::from(rd16(mem, r.wrapping_add(dim)));
                                #[allow(clippy::cast_possible_truncation)]
                                mem.write(s.wrapping_add(pos), (f64::from(v) + d) as f32);
                            }
                        }
                        for (pos, dim) in [(0x14u32, 4u32), (0x18, 6)] {
                            let d = f64::from(rd16(mem, r.wrapping_add(dim)));
                            let v = f64::from(crate::x87::load(mem.read(s.wrapping_add(pos))));
                            if v >= d {
                                #[allow(clippy::cast_possible_truncation)]
                                mem.write(s.wrapping_add(pos), (v - d) as f32);
                            }
                        }
                    }
                    fr
                }
            };
            mem.write(s.wrapping_add(6), 0u16);
            let r = rects.wrapping_add(12 * u32::from(mem.read::<u8>(fr.wrapping_add(2))));
            let pixels = src.wrapping_add(mem.read::<u32>(r.wrapping_add(8)));
            for (to, from) in [
                (s.wrapping_sub(0x10), 8u32),
                (s.wrapping_sub(0xe), 0xa),
                (s + 8, 0xc),
            ] {
                let v: u16 = mem.read(ss.wrapping_add(from));
                mem.write(to, v);
            }
            for (to, from) in [(0xcu32, 4u32), (0xa, 6), (8, 0), (6, 2)] {
                let v: u16 = mem.read(r.wrapping_add(from));
                mem.write(s.wrapping_sub(to), v);
            }
            mem.write(s.wrapping_sub(0x18), pixels);
            if mode == 3 {
                let sl = rd16(mem, ss.wrapping_add(0xc)).cast_unsigned();
                let pitch = rd16(
                    mem,
                    buf.wrapping_add(0x5c8).wrapping_add(sl.wrapping_mul(2)),
                );
                let pos_y = f64::from(mem.read::<f32>(s.wrapping_add(0x18)));
                let v = ffx_bridge::x87::ftol(mem, pos_y).cast_signed();
                let dst = rd16(mem, s.wrapping_sub(0x10)).wrapping_add(
                    mem.read::<i32>(buf.wrapping_add(0x5b8).wrapping_add(sl.wrapping_mul(4))),
                );
                let memcpy: u32 = mem.read(mem.rebase(MEMCPY_SLOT));
                let mut row = 0i32;
                while row < rd16(mem, r.wrapping_add(6)) {
                    let Some(m) = v
                        .wrapping_add(row)
                        .checked_rem(rd16(mem, s.wrapping_sub(0xa)))
                    else {
                        return 0;
                    };
                    let w = rd16(mem, s.wrapping_sub(0xc));
                    let from = m
                        .wrapping_mul(w)
                        .wrapping_add(pixels.cast_signed())
                        .wrapping_add(0x20);
                    let to = rd16(mem, s.wrapping_sub(0xe))
                        .wrapping_add(row)
                        .wrapping_mul(pitch)
                        .wrapping_add(dst);
                    call(
                        mem,
                        memcpy,
                        &[to.cast_unsigned(), from.cast_unsigned(), w.cast_unsigned()],
                    );
                    row += 1;
                }
            }
            k += 1;
        }
        buf
    }

    /// `chr_texanim_main_sub_new`'s callees and constants: the `strstr`
    /// import slot, `"c043"`, `"s024"`, `"c001"` (`g_ChrStrC001`),
    /// `g_BtlLeaderAlive`, the current map, `TexAnim_SetMaterialsChar_0077fbf0`
    /// (12 words) and `graphicTextureAnimationSetMaterialUV_00644150`.
    pub const STRSTR_SLOT: u32 = 0x00b0_c378;
    pub const STR_C043: u32 = 0x00b5_461c;
    pub const STR_S024: u32 = 0x00b5_4624;
    pub const STR_C001: u32 = 0x00b4_86bc;
    pub const BTL_LEADER_ALIVE: Global<u32> = Global::new(0x00c4_21fc);
    pub const CURRENT_MAP: Global<u32> = Global::new(0x012f_bc40);
    pub const SET_MATERIALS_CHAR: u32 = 0x0077_fbf0;
    pub const SET_MATERIAL_UV: u32 = 0x0064_4150;

    /// A new-format work record (0x34 bytes from the work block's `+8`,
    /// `+0xd08 + k` its byte "unused" when -1): the pixel pointer, frame,
    /// placement (`u16` x, y), the rectangle (`u16` w, h, x, y), the step
    /// counter and random wait (`i16`), the blink timer and state, the
    /// mode, the done flag, the layer (`u16`), the scroll speed and
    /// position (`f32` x, y).
    mod rec {
        pub const PIXELS: u32 = 0x00;
        pub const FRAME: u32 = 0x04;
        pub const X: u32 = 0x08;
        pub const Y: u32 = 0x0a;
        pub const W: u32 = 0x0c;
        pub const H: u32 = 0x0e;
        pub const RX: u32 = 0x10;
        pub const RY: u32 = 0x12;
        pub const COUNT: u32 = 0x14;
        pub const WAIT: u32 = 0x16;
        pub const TIMER: u32 = 0x18;
        pub const STATE: u32 = 0x1a;
        pub const MODE: u32 = 0x1c;
        pub const DONE: u32 = 0x1e;
        pub const LAYER: u32 = 0x20;
        pub const SPEED_X: u32 = 0x24;
        pub const SPEED_Y: u32 = 0x28;
        pub const POS_X: u32 = 0x2c;
        pub const POS_Y: u32 = 0x30;
    }

    /// Whether the C strings at `a` and `b` are equal (the original's
    /// inlined `strcmp` two bytes a step; only `== 0` is used).
    fn c_str_eq(mem: &impl Mem, a: u32, b: u32) -> bool {
        let mut k = 0u32;
        loop {
            let (x, y) = (
                mem.read::<u8>(a.wrapping_add(k)),
                mem.read::<u8>(b.wrapping_add(k)),
            );
            if x != y {
                return false;
            }
            if x == 0 {
                return true;
            }
            k += 1;
        }
    }

    /// `chr_texanim_main_sub_new(i, name, draw)` (`0x0077f360`; caller
    /// `chr_texanim_main`): one step of new-format entry `i` of
    /// [`TEX_ANIMS`] (source kind byte 2). On map `0xe2` a name containing
    /// `"c043"` sets the entry's track (`+8`) to 3 first.
    ///
    /// Tracks are stepped in two passes: first 1, 2, 0x15 and the entry's
    /// own track (unless one of those), skipping a track whose flag byte
    /// (`+0xf + track`) has bit 1 and keeping one with bit 0 for the
    /// second pass. For a track, each work record not marked unused takes
    /// its frame list from the source's record (0x110 bytes at
    /// `[src+8]`: frames offset at `+0x10 + 8 track`, `i16` count `+0x14`;
    /// no frames: the done flag on the first track only). By mode (jump
    /// table `0x0077f960`):
    /// - 1, 4: frame 1 while the frame number is 0, else frame 0;
    /// - 2: a blink (state 0: wait 60 + `rand() % 90`; 1: count down by
    ///   the speed, then state 2 with 4; 2: count down, then the next
    ///   frame, or back to state 0 after the last);
    /// - 3 and others: the frame steps when the counter (plus the speed
    ///   each time) passes the frame's `i16` time plus the random wait
    ///   (`(rand() & 0xff) * frame byte 3 >> 8`, set then; mode 3 passes,
    ///   the others reach), the old frame showing this time (frame 0 when
    ///   it wraps); mode 3 also scrolls (`f32` through x87) wrapped into
    ///   the rectangle.
    ///
    /// The frame's rectangle (12 bytes at `[src+4]` by frame byte 2:
    /// words `a`, `b`, pixel offset) and the record's placement are stored.
    /// With `draw`, `TexAnim_SetMaterialsChar` gets them (unless
    /// `g_BtlLeaderAlive` is 0 for Tidus's (`"c001"`) record 0; the
    /// rectangle words as the original's frame holds them, split across
    /// words), twice more for `"s024"`'s layers 1 and 2, and mode 3 gives
    /// its v offset (`1 - y / h`) to `SetMaterialUV`. Counts are read again
    /// each time. No caller reads `eax`.
    #[allow(clippy::too_many_lines)]
    pub fn chr_texanim_main_sub_new<M: Mem>(
        mem: &mut M,
        i: u32,
        name: u32,
        draw: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_bridge::x87::Reg;
        let g = |mem: &M, f: u32| mem.rebase(f);
        let e = TEX_ANIMS
            .ptr(mem)
            .addr
            .wrapping_add(i.wrapping_mul(TEXANIM_SIZE));
        let rd16 = |mem: &M, a: u32| i32::from(mem.read::<i16>(a));
        let rand = |mem: &mut M, call: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32| {
            let f: u32 = mem.read(mem.rebase(RAND_SLOT));
            call(mem, f, &[])
        };
        if CURRENT_MAP.get(mem) == 0xe2 {
            let strstr: u32 = mem.read(g(mem, STRSTR_SLOT));
            if call(mem, strstr, &[name, g(mem, STR_C043)]) != 0 {
                mem.write(e.wrapping_add(8), 3u32);
            }
        }
        let src: u32 = mem.read(e);
        if src == 0 || mem.read::<i8>(src) != 2 {
            return;
        }
        let speed: i8 = mem.read(e.wrapping_add(0xe));
        #[allow(clippy::cast_sign_loss)] // the low word, as `sub word, ax`
        let speed16 = i16::from(speed) as u16;
        let own: u32 = mem.read(e.wrapping_add(8));
        #[allow(clippy::cast_possible_truncation)] // its low byte
        let own = if matches!(own, 1 | 2 | 0x15) {
            0xff
        } else {
            own as u8
        };
        let order = [1u8, 2, 0x15, own, 0xff, 0xff];
        let mut later = [0xffu8; 8];
        let mut n_later = 0usize;
        for pass in 0..2 {
            let mut j = 0usize;
            loop {
                let track = if pass == 0 {
                    let t = i32::from(order[j].cast_signed());
                    if t == -1 {
                        break;
                    }
                    let flags: u8 = mem.read(e.wrapping_add(0xf).wrapping_add(t.cast_unsigned()));
                    if flags & 2 != 0 {
                        j += 1;
                        continue;
                    }
                    if flags & 1 != 0 {
                        // (the list is all 0xff past its end, as the
                        // original writes after each)
                        later[n_later] = order[j];
                        n_later += 1;
                        j += 1;
                        continue;
                    }
                    t
                } else {
                    let t = i32::from(later[j].cast_signed());
                    if t == -1 {
                        break;
                    }
                    t
                };
                let tu = track.cast_unsigned();
                let rects = src.wrapping_add(mem.read::<u32>(src.wrapping_add(4)));
                let work: u32 = mem.read(e.wrapping_add(4));
                let mut k = 0u32;
                while k.cast_signed() < mem.read::<i32>(work) {
                    let r = work.wrapping_add(8 + 0x34 * k);
                    let at = |o: u32| r.wrapping_add(o);
                    let next = k + 1;
                    if mem.read::<u8>(work.wrapping_add(0xd08 + k)) == 0xff {
                        k = next;
                        continue;
                    }
                    let layout = src
                        .wrapping_add(mem.read::<u32>(src.wrapping_add(8)))
                        .wrapping_add(0x110 * k);
                    let list = layout.wrapping_add(tu.wrapping_mul(8));
                    let cnt = rd16(mem, list.wrapping_add(0x14));
                    if cnt == 0 {
                        if j == 0 {
                            mem.write(at(rec::DONE), 1u16);
                        }
                        k = next;
                        continue;
                    }
                    let base = src.wrapping_add(mem.read::<u32>(list.wrapping_add(0x10)));
                    let frame_at = |mem: &M| {
                        base.wrapping_add(mem.read::<u32>(at(rec::FRAME)).wrapping_mul(4))
                    };
                    let mode = rd16(mem, at(rec::MODE));
                    let fr = match mode {
                        1 | 4 => {
                            base.wrapping_add(4 * u32::from(mem.read::<u32>(at(rec::FRAME)) == 0))
                        }
                        2 => {
                            match rd16(mem, at(rec::STATE)) {
                                0 => {
                                    let n = rand(mem, call).cast_signed();
                                    mem.write(at(rec::STATE), 1u16);
                                    mem.write(at(rec::FRAME), 0u32);
                                    #[allow(
                                        clippy::cast_possible_truncation,
                                        clippy::cast_sign_loss
                                    )]
                                    mem.write(at(rec::TIMER), (n.wrapping_rem(0x5a) + 0x3c) as u16);
                                }
                                1 => {
                                    let t = mem.read::<u16>(at(rec::TIMER)).wrapping_sub(speed16);
                                    mem.write(at(rec::TIMER), t);
                                    if t.cast_signed() < 0 {
                                        mem.write(at(rec::TIMER), 0x0002_0004u32);
                                    }
                                }
                                2 => {
                                    let t = mem.read::<u16>(at(rec::TIMER)).wrapping_sub(speed16);
                                    mem.write(at(rec::TIMER), t);
                                    if t.cast_signed() < 0 {
                                        let f = mem.read::<i32>(at(rec::FRAME)).wrapping_add(1);
                                        mem.write(at(rec::FRAME), f);
                                        if f < cnt {
                                            mem.write(at(rec::TIMER), 1u16);
                                        } else {
                                            mem.write(at(rec::FRAME), f.wrapping_sub(1));
                                            mem.write(at(rec::TIMER), 0u32);
                                        }
                                    }
                                }
                                _ => {}
                            }
                            frame_at(mem)
                        }
                        _ => {
                            let mut fr = frame_at(mem);
                            let time = rd16(mem, fr);
                            if time >= 0 {
                                let c = mem.read::<u16>(at(rec::COUNT)).wrapping_add(speed16);
                                mem.write(at(rec::COUNT), c);
                                let due = rd16(mem, at(rec::WAIT)) + time;
                                let c = rd16(mem, at(rec::COUNT));
                                // mode 3 steps past the time, the others on it
                                if (mode == 3 && due < c) || (mode != 3 && due <= c) {
                                    mem.write(at(rec::COUNT), 0u16);
                                    let n = rand(mem, call) & 0xff;
                                    let f = mem.read::<i32>(at(rec::FRAME)).wrapping_add(1);
                                    mem.write(at(rec::FRAME), f);
                                    let w = n.wrapping_mul(u32::from(
                                        mem.read::<u8>(fr.wrapping_add(3)),
                                    )) >> 8;
                                    #[allow(clippy::cast_possible_truncation)]
                                    mem.write(at(rec::WAIT), w as u16);
                                    if mem.read::<i32>(at(rec::FRAME)) >= cnt {
                                        fr = base;
                                        mem.write(at(rec::FRAME), 0u32);
                                    }
                                }
                            }
                            if mode == 3 {
                                let x = Reg::load(mem.read(at(rec::SPEED_X)))
                                    .add_m32(mem.read(at(rec::POS_X)))
                                    .store();
                                mem.write(at(rec::POS_X), x);
                                let y = Reg::load(mem.read(at(rec::SPEED_Y)))
                                    .add_m32(mem.read(at(rec::POS_Y)))
                                    .store();
                                mem.write(at(rec::POS_Y), y);
                                let rect = rects.wrapping_add(
                                    12 * u32::from(mem.read::<u8>(fr.wrapping_add(2))),
                                );
                                let size: u32 = mem.read(rect.wrapping_add(4));
                                #[allow(clippy::cast_possible_truncation)] // the two halves
                                let (w, h) = (
                                    Reg::constant(f64::from(size as i16)),
                                    Reg::constant(f64::from((size >> 16) as i16)),
                                );
                                if x < 0.0 {
                                    mem.write(at(rec::POS_X), (Reg::load(x) + w).store());
                                }
                                if y < 0.0 {
                                    mem.write(at(rec::POS_Y), (Reg::load(y) + h).store());
                                }
                                for (pos, d) in [(rec::POS_X, w), (rec::POS_Y, h)] {
                                    let v = Reg::load(mem.read(at(pos)));
                                    if v.value() >= d.value() {
                                        mem.write(at(pos), (v - d).store());
                                    }
                                }
                            }
                            fr
                        }
                    };
                    mem.write(at(rec::DONE), 0u16);
                    let rect =
                        rects.wrapping_add(12 * u32::from(mem.read::<u8>(fr.wrapping_add(2))));
                    let b: u32 = mem.read(rect.wrapping_add(4));
                    let a: u32 = mem.read(rect);
                    let c: u32 = mem.read(rect.wrapping_add(8));
                    for (to, from) in [(rec::X, 8u32), (rec::Y, 0xa), (rec::LAYER, 0xc)] {
                        let v: u16 = mem.read(layout.wrapping_add(from));
                        mem.write(at(to), v);
                    }
                    #[allow(clippy::cast_possible_truncation)] // the halves
                    for (to, v) in [
                        (rec::W, b as u16),
                        (rec::H, (b >> 16) as u16),
                        (rec::RX, a as u16),
                        (rec::RY, (a >> 16) as u16),
                    ] {
                        mem.write(at(to), v);
                    }
                    mem.write(at(rec::PIXELS), c.wrapping_add(src));
                    if draw == 0 {
                        k = next;
                        continue;
                    }
                    let frame: u32 = mem.read(at(rec::FRAME));
                    let (x, y) = (
                        u32::from(mem.read::<u16>(at(rec::X))),
                        u32::from(mem.read::<u16>(at(rec::Y))),
                    );
                    if BTL_LEADER_ALIVE.get(mem) != 0
                        || !c_str_eq(mem, g(mem, STR_C001), name)
                        || k != 0
                    {
                        // the frame's dwords at ebp-0x38, -0x36, -0x3c, -0x3a
                        let words = [b, (b >> 16) | (c << 16), a, (a >> 16) | (b << 16)];
                        call(
                            mem,
                            g(mem, SET_MATERIALS_CHAR),
                            &[
                                k, tu, frame, x, y, words[0], words[1], words[2], words[3], name,
                                draw, 0,
                            ],
                        );
                    }
                    if name != 0 && c_str_eq(mem, name, g(mem, STR_S024)) {
                        for layer in [1u32, 2] {
                            let h16 = |o: u32| u32::from(mem.read::<u16>(at(o)));
                            let args = [
                                layer,
                                tu,
                                mem.read(at(rec::FRAME)),
                                h16(rec::X),
                                h16(rec::Y),
                                h16(rec::W),
                                h16(rec::H),
                                h16(rec::RX),
                                h16(rec::RY),
                                name,
                                draw,
                                0,
                            ];
                            call(mem, g(mem, SET_MATERIALS_CHAR), &args);
                        }
                    }
                    if mem.read::<i16>(at(rec::MODE)) == 3 {
                        let h = Reg::constant(f64::from(mem.read::<i16>(at(rec::H))));
                        let q = Reg::load((Reg::load(mem.read(at(rec::POS_Y))) / h).store());
                        let v = (Reg::ONE - q).store();
                        call(mem, g(mem, SET_MATERIAL_UV), &[k, tu, 0, v.to_bits(), draw]);
                    }
                    k = next;
                }
                j += 1;
            }
        }
    }

    /// `chr_texanim_set(def, handle)` (`0x0077c950`): the first free entry
    /// of [`TEX_ANIMS`] (none: `movie_err("i!=CHR_TEX_ANIM_MAX", ...,
    /// 0xb28)`, -1) takes `def`'s source (`+0x20`) and its kind (byte `+0`
    /// to `+0xc`) and form (byte `+2` to `+0xd`). Kind 1 marks
    /// [`TEX_ANIM_844`] with 1 and the entry; kind 2 sets up the old (form
    /// 0) or new (form 1) work block. Returns the entry.
    pub fn chr_texanim_set<M: Mem>(
        mem: &mut M,
        def: u32,
        handle: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let table = TEX_ANIMS.ptr(mem).addr;
        let Some(i) = (0..0x40u32).find(|&i| mem.read::<u32>(table + 0x30 * i) == 0) else {
            let err = mem.rebase(MOVIE_ERR);
            let (msg, file) = (mem.rebase(0x00b5_44b8), mem.rebase(0x00b5_40a8));
            call(mem, err, &[msg, file, 0xb28]);
            return u32::MAX;
        };
        let e = table + 0x30 * i;
        let src: u32 = mem.read(def.wrapping_add(0x20));
        mem.write(e, src);
        let src: u32 = mem.read(def.wrapping_add(0x20));
        let form: u8 = mem.read(src.wrapping_add(2));
        mem.write(e + 0xd, form);
        let kind: u8 = mem.read(src);
        mem.write(e + 0xc, kind);
        match kind {
            1 => {
                #[allow(clippy::cast_possible_truncation)] // below 0x40
                TEX_ANIM_844.set(mem, [1, i as u8]);
            }
            2 => match mem.read::<u8>(e + 0xd) {
                0 => {
                    chr_texanim_set_old(mem, i, handle, def, call);
                }
                1 => {
                    chr_texanim_set_new(mem, i, handle, def, call);
                }
                _ => {}
            },
            _ => {}
        }
        i
    }
}
