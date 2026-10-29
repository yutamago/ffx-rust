//! M5 stage 1, list `small`, round 4: character leftovers: the texture
//! animation's GS upload packets, the model viewer's motion groups and
//! names, the cloth file task.

/// The originals over game memory.
#[allow(clippy::many_single_char_names)] // registers
pub mod bridge {
    use ffx_graphic::host::{Conv, Host};

    fn at(p: u32, o: u32) -> u32 {
        p.wrapping_add(o)
    }

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// The `i16` at `p`, sign-extended.
    fn sx16(h: &impl Host, p: u32) -> u32 {
        i32::from(h.read::<i16>(p)).cast_unsigned()
    }

    /// `TexAnim_SetGifTag(q, ...)` (9 words), `TexAnim_setTag(q, a, b, c,
    /// flag)`, the TRXREG, TRXPOS and TRXDIR quadwords (cdecl).
    pub const SET_GIF_TAG: u32 = 0x0077_ebe0;
    pub const SET_TAG: u32 = 0x0077_eb00;
    pub const TRXREG: u32 = 0x0077_f140;
    pub const TRXPOS: u32 = 0x0077_f100;
    pub const TRXDIR: u32 = 0x0077_f0e0;

    /// `setLoadImageTagsTexAnim(q, src, x, y, w, hgt, width, base)`
    /// (`0x0077eff0`, cdecl): a GS image upload packet at `q`: the DMA
    /// tag and NOPs (`0x10000006`, six zero words, `0x50000005`), the GIF
    /// tag, BITBLTBUF (`base`, the buffer width `(width + 63) / 64`, PSMT
    /// 0x13), TRXREG (`w`, `hgt`), TRXPOS (`x`, `y`), TRXDIR, then the
    /// image's DMA tag: `w * hgt / 16 + 2` quadwords (`0x30000000`) from
    /// `src & 0x7fffffff`, and two zero words. Returns the end (`q +
    /// 0x80`).
    #[allow(clippy::too_many_arguments)] // the original's eight words
    pub fn set_load_image_tags(
        h: &mut impl Host,
        q: u32,
        src: u32,
        x: u32,
        y: u32,
        w: u32,
        hgt: u32,
        width: u32,
        base: u32,
    ) -> u32 {
        h.write(q, 0x1000_0006u32);
        for i in 1..7 {
            h.write(at(q, 4 * i), 0u32);
        }
        h.write(at(q, 0x1c), 0x5000_0005u32);
        cdecl(h, SET_GIF_TAG, &[at(q, 0x20), 0xe, 0, 1, 0, 0, 0, 1, 4]);
        let blocks = width.cast_signed().wrapping_add(0x3f) / 0x40;
        cdecl(
            h,
            SET_TAG,
            &[at(q, 0x30), base, blocks.cast_unsigned(), 0x13, 0],
        );
        cdecl(h, TRXREG, &[at(q, 0x40), w, hgt]);
        cdecl(h, TRXPOS, &[at(q, 0x50), 0, x, y]);
        cdecl(h, TRXDIR, &[at(q, 0x60), 0]);
        // the original multiplies the `edx` that still holds `w` after the
        // calls (none of them writes it)
        let qwords = w.cast_signed().wrapping_mul(hgt.cast_signed()) / 16;
        h.write(
            at(q, 0x70),
            qwords.wrapping_add(2).cast_unsigned() | 0x3000_0000,
        );
        h.write(at(q, 0x74), src & 0x7fff_ffff);
        h.write(at(q, 0x78), 0u32);
        h.write(at(q, 0x7c), 0u32);
        at(q, 0x80)
    }

    /// `g_ChrTexAnim` (64 x 0x30: source `+0`, work `+4`, format `+0xd`),
    /// `Ch_TextureGetImageBase(chr, i)` (cdecl), `setLoadImageTagsTexAnim`.
    pub const TEX_ANIM: u32 = 0x0231_0000;
    pub const GET_IMAGE_BASE: u32 = 0x0083_0dc0;
    pub const SET_LOAD_IMAGE_TAGS: u32 = 0x0077_eff0;

    /// The upload packet for record `rec` (0x34 bytes; the original's
    /// pointer is `rec + 0x1c`) of the texture `chr`.
    fn upload(h: &mut impl Host, q: u32, chr: u32, rec: u32) -> u32 {
        let base = cdecl(h, GET_IMAGE_BASE, &[chr, sx16(h, at(rec, 0x20))]);
        let args = [
            q,
            h.read(rec),
            sx16(h, at(rec, 8)),
            sx16(h, at(rec, 0xa)),
            sx16(h, at(rec, 0xc)),
            sx16(h, at(rec, 0xe)),
            sx16(h, at(rec, 0x10)),
            base,
        ];
        cdecl(h, SET_LOAD_IMAGE_TAGS, &args)
    }

    /// Whether record `k` (`rec`) uploads: its skip byte (`skips + k`)
    /// isn't 0xff, its `u16 +0x1e` is 0 and its `u16 +0x1c` isn't 3.
    fn uploads(h: &impl Host, skips: u32, k: u32, rec: u32) -> bool {
        h.read::<u8>(at(skips, k)) != 0xff
            && h.read::<u16>(at(rec, 0x1e)) == 0
            && h.read::<u16>(at(rec, 0x1c)) != 3
    }

    /// The source of animation `i` when it is a texture animation (its
    /// first byte 2): the work area.
    fn work(h: &impl Host, i: u32) -> Option<u32> {
        let e = at(h.rebase(TEX_ANIM), i.wrapping_mul(0x30));
        let src: u32 = h.read(e);
        if src == 0 || h.read::<u8>(src) != 2 {
            return None;
        }
        Some(h.read(at(e, 4)))
    }

    /// `chr_texanim_draw_sub_old(i, q)` (`0x0077fe50`, cdecl): the old
    /// format's uploads (count `+0x400`, texture `+0x404`, records from
    /// `+0x408`, skip bytes `+0x5a8`) appended at `q`. Returns the end.
    pub fn draw_sub_old(h: &mut impl Host, i: u32, q: u32) -> u32 {
        let Some(w) = work(h, i) else { return q };
        let mut q = q;
        let mut k = 0u32;
        while k.cast_signed() < h.read::<i32>(at(w, 0x400)) {
            let rec = at(w, 0x408 + 0x34 * k);
            if uploads(h, at(w, 0x5a8), k, rec) {
                let chr: u32 = h.read(at(w, 0x404));
                q = upload(h, q, chr, rec);
            }
            k += 1;
        }
        q
    }

    /// `dbgPrintf`, `TODebugGetPadPrintFlag()`,
    /// `Ch_TextureAnimGetHandle(i)`, `Ch_GetName(chr)` (cdecl) and the
    /// messages.
    pub const DBG_PRINTF: u32 = 0x0062_f500;
    pub const PAD_PRINT_FLAG: u32 = 0x008a_bd20;
    pub const ANIM_GET_HANDLE: u32 = 0x0083_5ed0;
    pub const GET_NAME: u32 = 0x0082_6b90;
    pub const AREA_OVER: u32 = 0x00b5_462c;
    pub const PKT_SIZE: u32 = 0x00b5_4648;
    pub const ERR_CHR: u32 = 0x00b5_465c;

    /// `chr_texanim_draw_sub_new(i, q)` (`0x0077f210`, cdecl): the new
    /// format's uploads (count `+0`, texture `+4`, records from `+8`, skip
    /// bytes `+0xd08`; a record needs a destination) appended at `q`,
    /// each checked against its area (`x + w` against `+0x18`, `y + h`
    /// against `+0x1a`; "area over" printed). With `+0xd48` set only
    /// `TODebugGetPadPrintFlag` runs. A packet of 0xa00 bytes or more is
    /// reported with the character's name. Returns the end.
    pub fn draw_sub_new(h: &mut impl Host, i: u32, q: u32) -> u32 {
        let start = q;
        let mut q = q;
        if let Some(w) = work(h, i) {
            if h.read::<u32>(at(w, 0xd48)) == 0 {
                let mut k = 0u32;
                while k.cast_signed() < h.read::<i32>(w) {
                    let rec = at(w, 8 + 0x34 * k);
                    if uploads(h, at(w, 0xd08), k, rec) && h.read::<u32>(rec) != 0 {
                        let chr: u32 = h.read(at(w, 4));
                        let base = cdecl(h, GET_IMAGE_BASE, &[chr, sx16(h, at(rec, 0x20))]);
                        for (a, b, limit) in [(0x8, 0xc, 0x10), (0xa, 0xe, 0x12)] {
                            let end = sx16(h, at(rec, a)).wrapping_add(sx16(h, at(rec, b)));
                            if end.cast_signed() > sx16(h, at(rec, limit)).cast_signed() {
                                let f = h.rebase(AREA_OVER);
                                cdecl(h, DBG_PRINTF, &[f]);
                            }
                        }
                        let args = [
                            q,
                            h.read(rec),
                            sx16(h, at(rec, 8)),
                            sx16(h, at(rec, 0xa)),
                            sx16(h, at(rec, 0xc)),
                            sx16(h, at(rec, 0xe)),
                            sx16(h, at(rec, 0x10)),
                            base,
                        ];
                        q = cdecl(h, SET_LOAD_IMAGE_TAGS, &args);
                    }
                    k += 1;
                }
            } else {
                cdecl(h, PAD_PRINT_FLAG, &[]);
            }
        }
        let size = q.wrapping_sub(start);
        if size.cast_signed() >= 0xa00 {
            let f = h.rebase(PKT_SIZE);
            cdecl(h, DBG_PRINTF, &[f, size]);
            let handle = cdecl(h, ANIM_GET_HANDLE, &[i]);
            let name = cdecl(h, GET_NAME, &[handle]);
            let f = h.rebase(ERR_CHR);
            cdecl(h, DBG_PRINTF, &[f, name]);
        }
        q
    }

    /// `chr_texanim_draw_sub_old` and `_new`.
    pub const DRAW_SUB_OLD: u32 = 0x0077_fe50;
    pub const DRAW_SUB_NEW: u32 = 0x0077_f210;

    /// `chr_texanim_draw(i, q)` (`0x0077c830`, cdecl): by animation `i`'s
    /// format byte (`+0xd`, signed), a tail jump to the old (0) or new (1)
    /// packet builder; any other format returns `q`.
    pub fn draw(h: &mut impl Host, i: u32, q: u32) -> u32 {
        let e = at(h.rebase(TEX_ANIM), i.wrapping_mul(0x30));
        match h.read::<u8>(at(e, 0xd)) {
            0 => cdecl(h, DRAW_SUB_OLD, &[i, q]),
            1 => cdecl(h, DRAW_SUB_NEW, &[i, q]),
            _ => q,
        }
    }

    /// `SG_FindMotionGroup(a, b, key)`, `Cache_Exist(kind, id)`,
    /// `Cache_CheckYet(kind, id)`, `Ch_MgrpRead(id, n)`, `Ch_ReadSync()`,
    /// `Cache_Use(kind, id)`, `SG_AddMotionGroup(group, a, b, key)`
    /// (cdecl).
    pub const FIND_MOTION_GROUP: u32 = 0x0083_6f10;
    pub const CACHE_EXIST: u32 = 0x0084_06f0;
    pub const CACHE_CHECK_YET: u32 = 0x0083_ffe0;
    pub const MGRP_READ: u32 = 0x0082_a4c0;
    pub const READ_SYNC: u32 = 0x0082_a5e0;
    pub const CACHE_USE: u32 = 0x0084_0dd0;
    pub const ADD_MOTION_GROUP: u32 = 0x0083_6c90;

    /// `ChN_ReadSystemMGRP_ViewerAnimation(id, n)` (`0x008369a0`, cdecl):
    /// the motion group `n << 16 | id` (cache kind 2), read and synced
    /// when the cache doesn't hold it yet (state 2: `Ch_MgrpRead`, -1 when
    /// that fails; state 1: only the sync) and added to the viewer's
    /// groups. Returns its `i16` at `[entry + 0xc] + 8`.
    pub fn read_mgrp_viewer(h: &mut impl Host, id: u32, n: u32) -> u32 {
        let key = n << 16 | id;
        if cdecl(h, FIND_MOTION_GROUP, &[0, n, id]) != 0 {
            let e = cdecl(h, CACHE_EXIST, &[2, key]);
            let data: u32 = h.read(at(e, 0xc));
            return sx16(h, at(data, 8));
        }
        match cdecl(h, CACHE_CHECK_YET, &[2, key]) {
            2 => {
                if cdecl(h, MGRP_READ, &[id, n]) != 0 {
                    return u32::MAX;
                }
                cdecl(h, READ_SYNC, &[0]);
            }
            1 => {
                cdecl(h, READ_SYNC, &[0]);
            }
            _ => {}
        }
        let e = cdecl(h, CACHE_EXIST, &[2, key]);
        cdecl(h, CACHE_USE, &[2, key]);
        cdecl(h, ADD_MOTION_GROUP, &[e, 0, n, id]);
        let data: u32 = h.read(at(e, 0xc));
        sx16(h, at(data, 8))
    }

    /// `strncpy`'s import slot (cdecl), the two cloth extensions,
    /// `ClusterManager_createPreallocateDataList(path)` (cdecl).
    pub const STRNCPY: u32 = 0x00b0_c368;
    pub const EXT_A: u32 = 0x00b5_9674;
    pub const EXT_B: u32 = 0x00b5_9678;
    pub const CREATE_PREALLOCATE: u32 = 0x0063_d260;

    /// `Ch_LoadClothFileTask(task)` (`0x0082a140`, cdecl): the path at
    /// `task` gets each extension in turn over its last 7 characters
    /// (4 bytes from `len - 7`), each loaded into the word `[task +
    /// 0x100]` resp. `[task + 0x104]` points at; the done byte `+0x108`
    /// and the word `+0x10c` cleared. Returns the second load's `eax`.
    pub fn load_cloth_file_task(h: &mut impl Host, task: u32) -> u32 {
        let out_a: u32 = h.read(at(task, 0x100));
        let out_b: u32 = h.read(at(task, 0x104));
        let mut len = 0u32;
        while h.read::<u8>(at(task, len)) != 0 {
            len += 1;
        }
        let ext_at = at(task, len).wrapping_sub(7);
        let strncpy: u32 = h.read(h.rebase(STRNCPY));
        let mut r = 0;
        for (ext, out) in [(EXT_A, out_a), (EXT_B, out_b)] {
            let e = h.rebase(ext);
            h.call(strncpy, Conv::Cdecl, &[ext_at, e, 4]);
            r = cdecl(h, CREATE_PREALLOCATE, &[task]);
            h.write(out, r);
        }
        h.write(at(task, 0x108), 0u8);
        h.write(at(task, 0x10c), 0u32);
        r
    }

    /// The viewer's model table (`g_ChrViewerModels`, 12 bytes: name,
    /// copy, directory) and its count (`0x012fbca4`); `_strdup`'s import
    /// slot (cdecl).
    pub const VIEWER_MODELS: u32 = 0x012f_bca8;
    pub const VIEWER_MODEL_COUNT: u32 = 0x012f_bca4;
    pub const STRDUP: u32 = 0x00b0_c45c;

    fn same_string(h: &impl Host, a: u32, b: u32) -> bool {
        let mut i = 0;
        loop {
            let (x, y) = (h.read::<u8>(at(a, i)), h.read::<u8>(at(b, i)));
            if x != y {
                return false;
            }
            if x == 0 {
                return true;
            }
            i += 1;
        }
    }

    /// `Ch_IdGetFromName(name, dir)` (`0x00826dc0`, cdecl): the viewer key
    /// `0xf000 + i` of the entry whose `+0` string is `dir` and `+8`
    /// string is `name`; a new entry otherwise (`dir` copied to `+0` and,
    /// with its first letter upper-cased, to `+4`; `name` to `+8`).
    pub fn id_get_from_name(h: &mut impl Host, name: u32, dir: u32) -> u32 {
        let count_at = h.rebase(VIEWER_MODEL_COUNT);
        let count: i32 = h.read(count_at);
        let models = h.rebase(VIEWER_MODELS);
        for i in 0..count.max(0).cast_unsigned() {
            let e = at(models, 12 * i);
            let (a, b) = (h.read::<u32>(e), h.read::<u32>(at(e, 8)));
            if same_string(h, a, dir) && same_string(h, b, name) {
                return 0xf000 + i;
            }
        }
        let i = count.cast_unsigned();
        h.write(count_at, i.wrapping_add(1));
        let strdup: u32 = h.read(h.rebase(STRDUP));
        let e = at(models, i.wrapping_mul(12));
        let p = h.call(strdup, Conv::Cdecl, &[dir]);
        h.write(e, p);
        let p = h.call(strdup, Conv::Cdecl, &[dir]);
        h.write(at(e, 4), p);
        let c: u8 = h.read(p);
        if c.is_ascii_lowercase() {
            h.write(p, c.wrapping_sub(0x20));
        }
        let p = h.call(strdup, Conv::Cdecl, &[name]);
        h.write(at(e, 8), p);
        i.wrapping_add(0xf000)
    }
}
