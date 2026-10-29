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

    /// `WaitForSingleObject(h, ms)` and `ReleaseMutex(h)`'s import slots
    /// (stdcall), `Phyre_PhyreMalloc(size, align)`, `PArray_PushInto(array;
    /// dst, value)` (thiscall, `ret 8`), `operator delete(p)`,
    /// `ClusterMap_eraseSubtree(map; node)` (thiscall, `ret 4`),
    /// `PCluster` destructor (thiscall), `Phyre_PhyreFree(p)`.
    pub const WAIT_FOR_SINGLE_OBJECT: u32 = 0x00b0_c0f8;
    pub const RELEASE_MUTEX: u32 = 0x00b0_c1c0;
    pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
    pub const PARRAY_PUSH_INTO: u32 = 0x0069_b650;
    pub const OPERATOR_DELETE: u32 = 0x0063_0390;
    pub const ERASE_SUBTREE: u32 = 0x0069_a2d0;
    pub const CLUSTER_DTOR: u32 = 0x0043_fa30;
    pub const PHYRE_FREE: u32 = 0x0042_fb20;

    /// The in-order successor of `node` in an MSVC `std::map` (nil byte
    /// `+0xd`, left `+0`, parent `+4`, right `+8`); a nil node stays.
    fn successor(h: &impl Host, node: u32) -> u32 {
        let nil = |n: u32| -> bool { h.read::<u8>(at(n, 0xd)) != 0 };
        if nil(node) {
            return node;
        }
        let right: u32 = h.read(at(node, 8));
        if !nil(right) {
            let mut n = right;
            loop {
                let left: u32 = h.read(n);
                if nil(left) {
                    return n;
                }
                n = left;
            }
        }
        let mut n = node;
        let mut p: u32 = h.read(at(n, 4));
        while !nil(p) && n == h.read::<u32>(at(p, 8)) {
            n = p;
            p = h.read(at(p, 4));
        }
        p
    }

    /// `FUN_0069a9c0(cache)` (thiscall): the cluster cache emptied at
    /// shutdown. Under its mutex (`[[+0x18]]`): every entry of the map at
    /// `+0x10` gathers its value in a local `PArray` (`arr`, the
    /// original's `ebp - 0x20`: count, data), and the entry's slot is
    /// handed back to the pool (`+0x1c`, 0x18 bytes each, 0x400 of them:
    /// its in-use byte `+0x601c` cleared, the lowest free index `+0x641c`
    /// lowered) or deleted; the map is erased. Then each gathered cluster
    /// is destroyed and freed, and the array's storage freed when owned.
    /// Returns `Phyre_PhyreFree`'s `eax`, or what the original's last
    /// instructions leave (the count, or the data pointer).
    pub fn cluster_cache_clear(h: &mut impl Host, cache: u32, arr: u32) -> u32 {
        h.write(arr, 0u32);
        h.write(at(arr, 4), 0u32);
        let wait: u32 = h.read(h.rebase(WAIT_FOR_SINGLE_OBJECT));
        let mutex: u32 = h.read(h.read::<u32>(at(cache, 0x18)));
        h.call(wait, Conv::Stdcall, &[mutex, u32::MAX]);
        let map = at(cache, 0x10);
        let mut node: u32 = h.read(h.read::<u32>(map));
        let pool = at(cache, 0x1c);
        let pool_end = at(cache, 0x601c);
        while node != h.read::<u32>(map) {
            let value: u32 = h.read(at(node, 0x14));
            let count: u32 = h.read(arr);
            let dst = cdecl(
                h,
                PHYRE_MALLOC,
                &[(count & 0x7fff_ffff).wrapping_mul(4).wrapping_add(4), 4],
            );
            h.call_sym(PARRAY_PUSH_INTO, Conv::Thiscall(arr), &[dst, value]);
            let slot: u32 = h.read(at(node, 0x14));
            if slot >= pool && slot < pool_end {
                let idx = slot.wrapping_sub(cache).wrapping_sub(0x1c).cast_signed() / 0x18;
                h.write(at(pool_end, idx.cast_unsigned()), 0u8);
                if idx < h.read::<i32>(at(cache, 0x641c)) {
                    h.write(at(cache, 0x641c), idx);
                }
            } else {
                cdecl(h, OPERATOR_DELETE, &[slot]);
            }
            node = successor(h, node);
        }
        let head: u32 = h.read(map);
        let mut n: u32 = h.read(at(head, 4));
        while h.read::<u8>(at(n, 0xd)) == 0 {
            let right: u32 = h.read(at(n, 8));
            h.call_sym(ERASE_SUBTREE, Conv::Thiscall(map), &[right]);
            let left: u32 = h.read(n);
            cdecl(h, OPERATOR_DELETE, &[n]);
            n = left;
        }
        let head: u32 = h.read(map);
        h.write(at(head, 4), head);
        let head: u32 = h.read(map);
        h.write(head, head);
        let head: u32 = h.read(map);
        h.write(at(head, 8), head);
        h.write(at(map, 4), 0u32);
        let release: u32 = h.read(h.rebase(RELEASE_MUTEX));
        let mutex: u32 = h.read(h.read::<u32>(at(cache, 0x18)));
        h.call(release, Conv::Stdcall, &[mutex]);
        let count: u32 = h.read(arr);
        let n = count & 0x7fff_ffff;
        let mut r = n;
        for i in 0..n {
            let data: u32 = h.read(at(arr, 4));
            let e: u32 = h.read(at(data, 4 * i));
            if e != 0 {
                h.call_sym(CLUSTER_DTOR, Conv::Thiscall(e), &[]);
                cdecl(h, PHYRE_FREE, &[e]);
            }
            r = h.read(at(arr, 4));
        }
        if count & 0x8000_0000 == 0 {
            let data: u32 = h.read(at(arr, 4));
            r = cdecl(h, PHYRE_FREE, &[data]);
        }
        r
    }

    /// CRT import slots (cdecl): `memset`'s thunk, `strstr`, `__iob_func`,
    /// `fprintf`, `fflush`, `sprintf`; `Ch_AnalysisChrNum(src, dst)`,
    /// `fiosUnifyFilename(src, out, size)`, `graphicCharacterLoad(name,
    /// id)`, `graphicTryAsyncLoadIfAvailable(f, arg)` (cdecl); the strings;
    /// the 50 cloth file loads (`g_ChrClothLoads`, 0x110 bytes each from
    /// `0x012fc598`: the path, `+0x100/+0x104` the outputs, `+0x108` busy,
    /// `+0x10c` the character); `Ch_LoadClothFileTask`.
    pub const MEMSET: u32 = 0x0094_964c;
    pub const STRSTR: u32 = 0x00b0_c378;
    pub const IOB_FUNC: u32 = 0x00b0_c350;
    pub const FPRINTF: u32 = 0x00b0_c38c;
    pub const FFLUSH: u32 = 0x00b0_c3bc;
    pub const SPRINTF: u32 = 0x00b0_c3dc;
    pub const ANALYSIS_CHR_NUM: u32 = 0x0082_9e60;
    pub const UNIFY_FILENAME: u32 = 0x0067_9820;
    pub const CHARACTER_LOAD: u32 = 0x0063_d140;
    pub const TRY_ASYNC_LOAD: u32 = 0x0064_42f0;
    pub const STR_CHR: u32 = 0x00b4_7d84;
    pub const STR_MAGENTA: u32 = 0x00b5_967c;
    pub const STR_LOAD_CHARACTER: u32 = 0x00b5_9684;
    pub const STR_RESET: u32 = 0x00b1_f854;
    pub const STR_DATA_DIR: u32 = 0x00b4_3b18;
    pub const STR_PATH_FMT: u32 = 0x00b5_85c8;
    pub const STR_AHWIN32: u32 = 0x00b5_9698;
    pub const CLOTH_LOADS: u32 = 0x012f_c598;
    pub const LOAD_CLOTH_FILE_TASK: u32 = 0x0082_a140;

    fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Cdecl, args)
    }

    /// `fprintf(stderr, ...)` through `__iob_func() + 0x20`.
    fn eprint(h: &mut impl Host, args: &[u32]) {
        let err = import(h, IOB_FUNC, &[]).wrapping_add(0x20);
        let mut w = [err, 0, 0];
        w[1..=args.len()].copy_from_slice(args);
        import(h, FPRINTF, &w[..=args.len()]);
    }

    /// `Ch_LoadHdRenderData(path, cloth_a, cloth_b, id)` (`0x0082a1c0`,
    /// cdecl). `frame` is the original's three buffers (`ebp - 0x284`:
    /// the data directory, 0x100 bytes; then the file path, 0x100; then
    /// the character's name, 0x80): the name taken from `path` after
    /// "chr" (`Ch_AnalysisChrNum`), printed to stderr in magenta, the
    /// path `<data dir>/<name>` with its last two characters and on
    /// replaced by "ahwin32" loaded (`graphicCharacterLoad(path, id)`);
    /// with both cloth outputs given, a free cloth load slot gets the path
    /// and the outputs and `Ch_LoadClothFileTask` is queued on it. Returns
    /// `graphicCharacterLoad`'s `eax`.
    pub fn load_hd_render_data(
        h: &mut impl Host,
        path: u32,
        a: u32,
        b: u32,
        id: u32,
        frame: u32,
    ) -> u32 {
        let (dir, full, name) = (frame, at(frame, 0x100), at(frame, 0x200));
        cdecl(h, MEMSET, &[name, 0, 0x80]);
        cdecl(h, MEMSET, &[full, 0, 0x100]);
        let chr = h.rebase(STR_CHR);
        let p = import(h, STRSTR, &[path, chr]);
        if p != 0 {
            cdecl(h, ANALYSIS_CHR_NUM, &[p, name]);
        }
        let s = h.rebase(STR_MAGENTA);
        eprint(h, &[s]);
        let s = h.rebase(STR_LOAD_CHARACTER);
        eprint(h, &[s, name]);
        let s = h.rebase(STR_RESET);
        eprint(h, &[s]);
        let err = import(h, IOB_FUNC, &[]).wrapping_add(0x20);
        import(h, FFLUSH, &[err]);
        let s = h.rebase(STR_DATA_DIR);
        cdecl(h, UNIFY_FILENAME, &[s, dir, 0x100]);
        let s = h.rebase(STR_PATH_FMT);
        import(h, SPRINTF, &[full, s, dir, name]);
        let mut len = 0u32;
        while h.read::<u8>(at(full, len)) != 0 {
            len += 1;
        }
        let s = h.rebase(STR_AHWIN32);
        import(h, STRNCPY, &[at(full, len).wrapping_sub(2), s, 7]);
        let r = cdecl(h, CHARACTER_LOAD, &[full, id]);
        if a == 0 || b == 0 {
            return r;
        }
        let loads = h.rebase(CLOTH_LOADS);
        let Some(slot) = (0..50u32)
            .map(|i| at(loads, i * 0x110))
            .find(|&s| h.read::<u8>(at(s, 0x108)) == 0)
        else {
            return r;
        };
        h.write(at(slot, 0x108), 1u8);
        h.write(at(slot, 0x10c), r);
        import(h, STRNCPY, &[slot, full, 0x100]);
        h.write(at(slot, 0x100), a);
        h.write(at(slot, 0x104), b);
        let task = h.rebase(LOAD_CLOTH_FILE_TASK);
        cdecl(h, TRY_ASYNC_LOAD, &[task, slot]);
        r
    }

    /// `ch_create_name(id)`, `ch_create_chrpath(id)`, `Sg_PcReadD(path)`,
    /// `Sg_PcReadGetLen()` (cdecl); `FFXIsDebug`; the two name buffers;
    /// the 50 motion table entries (16 bytes from `0x01301438`: data,
    /// used, count, id); the formats.
    pub const CREATE_NAME: u32 = 0x0083_8100;
    pub const CREATE_CHRPATH: u32 = 0x0083_80b0;
    pub const PC_READ_D: u32 = 0x0083_ae50;
    pub const PC_READ_GET_LEN: u32 = 0x0083_ae70;
    pub const IS_DEBUG: u32 = 0x0133_c910;
    pub const MOTION_NAME: u32 = 0x0130_17b8;
    pub const MOTION_NAME2: u32 = 0x0130_1778;
    pub const MOTION_TABLES: u32 = 0x0130_1438;
    pub const FMT_NAME_KEY: u32 = 0x00b5_9adc;
    pub const FMT_MOTION_TBL: u32 = 0x00b5_9ae4;
    pub const FMT_NAME_NAME: u32 = 0x00b4_a5dc;

    /// `FUN_008371f0(key)` (cdecl): the motion `key` (`id << 16 |
    /// motion`) as text: "name:key" in `0x013017b8`; in a debug build for
    /// a non-viewer character, the character's `inf/motion.tbl` (loaded
    /// once into a free table entry through `path`, the original's 64-byte
    /// local at `ebp - 0x44`; no free entry ends the search) gives the
    /// motion's own name, `name_motion` in `0x01301778`. Returns the
    /// buffer used.
    pub fn motion_name(h: &mut impl Host, key: u32, path: u32) -> u32 {
        let id = (key.cast_signed() >> 16).cast_unsigned() & 0xffff;
        let buf = h.rebase(MOTION_NAME);
        let name = cdecl(h, CREATE_NAME, &[id]);
        let f = h.rebase(FMT_NAME_KEY);
        import(h, SPRINTF, &[buf, f, name, key]);
        if h.read::<u8>(h.rebase(IS_DEBUG)) == 0 || id & 0xf000 == 0xf000 {
            return buf;
        }
        let motion = key & 0xffff;
        let tables = h.rebase(MOTION_TABLES);
        let (mut free, mut found) = (0u32, 0u32);
        for i in 0..50 {
            let e = at(tables, 16 * i);
            if h.read::<u32>(at(e, 4)) == 0 {
                if free == 0 {
                    free = e;
                }
            } else if h.read::<u32>(at(e, 0xc)) == id {
                found = e;
            }
        }
        if free == 0 {
            return buf;
        }
        if found == 0 {
            let dir = cdecl(h, CREATE_CHRPATH, &[id]);
            let f = h.rebase(FMT_MOTION_TBL);
            import(h, SPRINTF, &[path, f, dir]);
            let data = cdecl(h, PC_READ_D, &[path]);
            h.write(free, data);
            h.write(at(free, 0xc), id);
            if data != 0 {
                let len = cdecl(h, PC_READ_GET_LEN, &[]);
                h.write(at(free, 8), len.wrapping_sub(0x10) >> 6);
            }
            found = free;
            h.write(at(free, 4), 1u32);
        }
        let data: u32 = h.read(found);
        if data == 0 {
            return buf;
        }
        let n: i32 = h.read(at(found, 8));
        let mut p = at(data, 0x10);
        for _ in 0..n.max(0) {
            if u32::from(h.read::<u16>(p)) == motion {
                let name = cdecl(h, CREATE_NAME, &[id]);
                let buf2 = h.rebase(MOTION_NAME2);
                let f = h.rebase(FMT_NAME_NAME);
                import(h, SPRINTF, &[buf2, f, name, at(p, 0x20)]);
                return buf2;
            }
            p = at(p, 0x40);
        }
        buf
    }

    /// `Ch_TextureSetAnim(chr, a, b)`, `Ch_SetSequence(chr, id)` (cdecl),
    /// the system motion sets (`i8` per set), the search counter.
    pub const TEXTURE_SET_ANIM: u32 = 0x0083_d040;
    pub const SET_SEQUENCE: u32 = 0x0083_7ac0;
    pub const SYS_MOTION_SETS: u32 = 0x00c4_9784;
    pub const SEARCH_COUNT: u32 = 0x0130_09f0;

    /// The next index from `i` in direction `dir` (up when set) over
    /// `0..=max`, wrapping.
    fn step(i: i32, max: i32, dir: u32) -> i32 {
        if dir != 0 {
            let up = i.wrapping_add(1);
            if up > max { 0 } else { up }
        } else {
            let down = i.wrapping_sub(1);
            if down < 0 { max } else { down }
        }
    }

    /// `Ch_SetSysMotion_ViewerAnimation(chr, set, idx, max, dir)`
    /// (`0x00837e10`, cdecl): the model viewer's next motion of `set`
    /// from `idx` on, in direction `dir`, trying at most `max` + 1 (the
    /// shared counter `0x013009f0`) indices. A definition with flag 1
    /// (`[chr + 0x1c4] + 0x70`) looks in its system motion table (a
    /// missing row gives 0 at once; the texture animation 4 set for set 1
    /// index 0x10 on characters with flag 0x200), else in its motion list
    /// `+0x108` (entries of 8 bytes: count `+8`, `(index, motion)` pairs
    /// `+0xc`; set 0 tries only `idx`). The motion found is set
    /// (`Ch_SetSequence`) and its index returned; 0 when none is.
    pub fn set_sys_motion_viewer(
        h: &mut impl Host,
        chr: u32,
        set: u32,
        idx: u32,
        max: u32,
        dir: u32,
    ) -> u32 {
        let counter = h.rebase(SEARCH_COUNT);
        let def: u32 = h.read(at(chr, 0x1c4));
        let mut i = idx.cast_signed();
        let max_s = max.cast_signed();
        let mut g = h.read::<u32>(counter).wrapping_add(1);
        if h.read::<u8>(at(def, 0x70)) & 1 != 0 {
            h.write(counter, g);
            while g.cast_signed() <= max_s {
                let k = i32::from(h.read::<i8>(at(h.rebase(SYS_MOTION_SETS), set)));
                let def: u32 = h.read(at(chr, 0x1c4));
                let tbl: u32 = h.read(at(def, 0x6c));
                let row: u32 = h.read(at(
                    tbl,
                    k.cast_unsigned().wrapping_mul(8).wrapping_add(0x10),
                ));
                if row == 0 {
                    return 0;
                }
                if h.read::<u32>(at(chr, 0x198)) & 0x200 != 0 && set == 1 && i == 0x10 {
                    cdecl(h, TEXTURE_SET_ANIM, &[chr, 4, set]);
                    g = h.read(counter);
                }
                let m: u32 = h.read(at(row, i.cast_unsigned().wrapping_mul(4)));
                if m != 0 {
                    cdecl(h, SET_SEQUENCE, &[chr, m]);
                    h.write(counter, 0u32);
                    return i.cast_unsigned();
                }
                i = step(i, max_s, dir);
                g = g.wrapping_add(1);
                h.write(counter, g);
            }
            h.write(counter, 0u32);
            return 0;
        }
        let list: u32 = h.read(at(def, 0x108));
        if list == 0 {
            return 0;
        }
        h.write(counter, g);
        let ent = at(list, set.wrapping_mul(8));
        while g.cast_signed() <= max_s {
            let pairs: u32 = h.read(at(ent, 0xc));
            let n: u32 = h.read(at(ent, 8));
            for j in 0..n {
                let p = at(pairs, j * 4);
                if i.cast_unsigned() == u32::from(h.read::<u16>(p)) {
                    let motion = u32::from(h.read::<u16>(at(p, 2)));
                    let def: u32 = h.read(at(chr, 0x1c4));
                    let id: u32 = h.read(def);
                    cdecl(h, SET_SEQUENCE, &[chr, motion | id << 16]);
                    return i.cast_unsigned();
                }
            }
            if set == 0 {
                return 0;
            }
            i = step(i, max_s, dir);
            g = g.wrapping_add(1);
            h.write(counter, g);
        }
        h.write(counter, 0u32);
        0
    }

    /// The `Gu` window calls (cdecl): `Gu_WindowGetArg(w)`,
    /// `Gu_WindowSetArg(w, a)`, `Gu_WindowSetTitle(w, s)`,
    /// `Gu_WindowAddButton(w, id, text, x, y, wd, ht)`,
    /// `Gu_ButtonSetToggle(w, id, on)`, `Gu_ButtonSetToggleValue(w, id,
    /// on)`, `Gu_WindowCloseTree(w)`; `Ch_TextureGetAnimId(chr)`,
    /// `tex_anim_no_set(i, a, b, c, d)`, `user_malloc(n)`,
    /// `user_free_thunk(p)`, `Ch_CheckHandle(chr)`; the window's character
    /// (`0x0193c148`), its title format, note and `"%d"`.
    pub const WINDOW_GET_ARG: u32 = 0x0084_aec0;
    pub const WINDOW_SET_ARG: u32 = 0x0084_b560;
    pub const WINDOW_SET_TITLE: u32 = 0x0084_b6a0;
    pub const WINDOW_ADD_BUTTON: u32 = 0x0084_a470;
    pub const BUTTON_SET_TOGGLE: u32 = 0x0084_9380;
    pub const BUTTON_SET_TOGGLE_VALUE: u32 = 0x0084_93e0;
    pub const WINDOW_CLOSE_TREE: u32 = 0x0084_ab80;
    pub const TEXTURE_GET_ANIM_ID: u32 = 0x0082_b960;
    pub const TEX_ANIM_NO_SET: u32 = 0x0077_e940;
    pub const USER_MALLOC: u32 = 0x0068_7020;
    pub const USER_FREE: u32 = 0x0068_7010;
    pub const CHECK_HANDLE: u32 = 0x0082_61b0;
    pub const ANIM_WIN_CHR: u32 = 0x0193_c148;
    pub const FMT_TEX_ANIM: u32 = 0x00b6_8f90;
    pub const ANIM_WIN_FMT: u32 = 0x00b6_2d20;
    pub const ANIM_WIN_TEXT: u32 = 0x00b6_8f7c;
    pub const FMT_D: u32 = 0x00b0_f604;

    /// `FUN_0093e370(window, msg, id)` (cdecl): the texture animation
    /// window's procedure (`chr_anim_win`'s). `buf` is the original's
    /// 128-byte local (`ebp - 0x84`). On create (0x8001) its state (8
    /// bytes: the character, the pressed button) is made, the title set
    /// and 32 toggle buttons laid out, button `id` pressed and animation
    /// 0 shown; on close (0x8003) the state is freed; a button (0x8007,
    /// `id` below 32) shows that animation and moves the pressed toggle.
    /// The window closes when its character's handle is gone. Returns 0.
    pub fn anim_window_proc(h: &mut impl Host, window: u32, msg: u32, id: u32, buf: u32) -> u32 {
        let mut st = cdecl(h, WINDOW_GET_ARG, &[window]);
        match msg {
            0x8001 => {
                st = cdecl(h, USER_MALLOC, &[8]);
                let chr: u32 = h.read(h.rebase(ANIM_WIN_CHR));
                h.write(st, chr);
                cdecl(h, WINDOW_SET_ARG, &[window, st]);
                let chr: u32 = h.read(h.rebase(ANIM_WIN_CHR));
                let name = cdecl(h, GET_NAME, &[chr]);
                let f = h.rebase(FMT_TEX_ANIM);
                import(h, SPRINTF, &[buf, f, name]);
                cdecl(h, WINDOW_SET_TITLE, &[window, buf]);
                let chr: u32 = h.read(st);
                let anim = cdecl(h, TEXTURE_GET_ANIM_ID, &[chr]);
                let (f, t) = (h.rebase(ANIM_WIN_FMT), h.rebase(ANIM_WIN_TEXT));
                cdecl(h, DBG_PRINTF, &[f, t]);
                for i in 0..32u32 {
                    let f = h.rebase(FMT_D);
                    import(h, SPRINTF, &[buf, f, i]);
                    let (x, y) = ((i & 7) * 20, (i >> 3) * 20 + 4);
                    cdecl(h, WINDOW_ADD_BUTTON, &[window, i, buf, x, y, 0x11, 0xe]);
                    cdecl(h, BUTTON_SET_TOGGLE, &[window, i, 1]);
                }
                cdecl(h, BUTTON_SET_TOGGLE_VALUE, &[window, id, 1]);
                h.write(at(st, 4), 0u32);
                cdecl(h, TEX_ANIM_NO_SET, &[anim, 4, 0, 0, 0]);
            }
            0x8003 => {
                cdecl(h, USER_FREE, &[st]);
                return 0;
            }
            0x8007 if id <= 0x1f => {
                let chr: u32 = h.read(st);
                let anim = cdecl(h, TEXTURE_GET_ANIM_ID, &[chr]);
                cdecl(h, TEX_ANIM_NO_SET, &[anim, 4, id, 0, 0]);
                let pressed: u32 = h.read(at(st, 4));
                if pressed & 0x8000_0000 == 0 {
                    cdecl(h, BUTTON_SET_TOGGLE_VALUE, &[window, pressed, 0]);
                }
                cdecl(h, BUTTON_SET_TOGGLE_VALUE, &[window, id, 1]);
                h.write(at(st, 4), id);
            }
            _ => {}
        }
        if st != 0 {
            let chr: u32 = h.read(st);
            if cdecl(h, CHECK_HANDLE, &[chr]) & 0x8000_0000 != 0 {
                cdecl(h, WINDOW_CLOSE_TREE, &[window]);
            }
        }
        0
    }

    /// `Sg_PcRead(path)`, `Ch_SetMotionSeq(chr, clip, loops, start, end)`,
    /// `sg_getdbnum(c)` (cdecl), `atoi`'s import slot (cdecl), the
    /// formats.
    pub const PC_READ: u32 = 0x0083_ae30;
    pub const SET_MOTION_SEQ: u32 = 0x0083_9b80;
    pub const GET_DB_NUM: u32 = 0x0082_9d80;
    pub const ATOI: u32 = 0x00b0_c40c;
    pub const FMT_MTYPE: u32 = 0x00b5_9464;
    pub const FMT_ANM_LOCAL: u32 = 0x00b5_9484;
    pub const FMT_ANM_PROJ: u32 = 0x00b5_9494;
    pub const MSG_MOTRELTAB: u32 = 0x00b5_94bc;

    /// `Ch_ReadMotionViewer(chr, dir, sub, name)` (`0x008283c0`, cdecl):
    /// the model viewer's motion file. `buf` is the original's 128-byte
    /// path (`ebp - 0x84`). Unless the definition (`[chr + 0x1c4]`)
    /// holds its motions (`+0xf8`), the `.anm` is read (viewer models
    /// from their own directory `+0xb8`, others from the project tree),
    /// kept in the definition's slot `+0xfc` (the old one freed, the name
    /// copied to `+0x104`) and started (a clip for kinds above 1, else
    /// the pose `+0x754` = 0x100); the relative motion table `+0x114`
    /// (count `+0x110`) gives `+0x758` for `atoi(sub + 1) | db(sub[0]) <<
    /// 12`; the motion slot `+0x800` and mode `+0x752` = 4 set. Returns
    /// 4, 0 when the file doesn't read, or `dir` when the definition
    /// holds its motions.
    pub fn read_motion_viewer(
        h: &mut impl Host,
        chr: u32,
        dir: u32,
        sub: u32,
        name: u32,
        buf: u32,
    ) -> u32 {
        let def: u32 = h.read(at(chr, 0x1c4));
        let flags: u32 = h.read(at(chr, 0x194));
        h.write(at(chr, 0x194), flags & 0x7fff_ffff);
        if h.read::<u8>(at(def, 0xf8)) != 0 {
            return dir;
        }
        let f = h.rebase(FMT_MTYPE);
        cdecl(h, DBG_PRINTF, &[f, dir, sub, name]);
        if h.read::<u16>(chr) & 0xf000 == 0xf000 {
            let f = h.rebase(FMT_ANM_LOCAL);
            import(h, SPRINTF, &[buf, f, at(def, 0xb8), name]);
        } else {
            let f = h.rebase(FMT_ANM_PROJ);
            import(h, SPRINTF, &[buf, f, dir, sub, name]);
        }
        let data = cdecl(h, PC_READ, &[buf]);
        if data == 0 {
            return 0;
        }
        let e = at(def, 0xfc);
        let old: u32 = h.read(at(e, 4));
        if old != 0 {
            cdecl(h, USER_FREE, &[old]);
            h.write(at(e, 4), 0u32);
        }
        h.write(at(e, 4), data);
        let copy = import(h, STRDUP, &[name]);
        h.write(at(e, 8), copy);
        let kind: u16 = h.read(at(data, 8));
        if kind == 0 || kind == 1 {
            let pose: u16 = h.read(data);
            h.write(e, pose);
            let flags: u32 = h.read(at(chr, 0x194));
            h.write(at(chr, 0x194), flags | 0x8000_0000);
            h.write(at(chr, 0x740), 0u32);
            h.write(at(chr, 0x754), 0x100u16);
        } else {
            let clip: u32 = h.read(at(e, 4));
            cdecl(h, SET_MOTION_SEQ, &[chr, clip, 0, 0, u32::MAX]);
        }
        let n = import(h, ATOI, &[at(sub, 1)]);
        let c = i32::from(h.read::<i8>(sub)).cast_unsigned();
        let key = n | cdecl(h, GET_DB_NUM, &[c]) << 12;
        h.write(at(chr, 0x758), 0u32);
        let list: u32 = h.read(at(def, 0x114));
        if list != 0 {
            let count = i32::from(h.read::<i16>(at(def, 0x110)));
            for i in 0..count.max(0).cast_unsigned() {
                let entry: u32 = h.read(at(list, 4 * i));
                if i32::from(h.read::<i16>(entry)).cast_unsigned() == key {
                    h.write(at(chr, 0x758), entry);
                    let f = h.rebase(MSG_MOTRELTAB);
                    cdecl(h, DBG_PRINTF, &[f]);
                    break;
                }
            }
        }
        let flags: u32 = h.read(at(chr, 0x194));
        h.write(at(chr, 0x194), flags | 0x40);
        h.write(at(chr, 0x800), e);
        h.write(at(chr, 0x752), 4u16);
        4
    }
}
