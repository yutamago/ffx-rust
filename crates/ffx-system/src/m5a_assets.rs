//! M5 `system-a` (round 3): the scene's asset list reader (its copy
//! constructor and destructor), the Steam stats object's constructor
//! and the archive file read (`FileSys_Read`).

/// The originals over game memory and the original callees.
pub mod bridge {
    use ffx_graphic::host::{Conv, Host};

    /// The asset list reader's member copies (thiscall): the tree
    /// (`0x0064d050`, `ret 8`), the vector at `+8` (`0x0064d570`, `ret
    /// 4`), the vectors at `+0x14` and `+0x20` (`0x0064d730`, `ret 4`),
    /// the string at `+0x2c` (`StdString_assignSubstr_00403a10`, `ret
    /// 0xc`).
    pub const COPY_TREE: u32 = 0x0064_d050;
    pub const COPY_VECTOR8: u32 = 0x0064_d570;
    pub const COPY_VECTOR: u32 = 0x0064_d730;
    pub const ASSIGN_SUBSTR: u32 = 0x0040_3a10;
    /// The destructor's callees: `operator delete` (cdecl),
    /// `0x00635c40` (a range's destructor, stdcall `ret 8`),
    /// `std_Tree_erase_range_00636d60` (thiscall, `ret 0xc`).
    pub const OPERATOR_DELETE: u32 = 0x0063_0390;
    pub const DESTROY_RANGE: u32 = 0x0063_5c40;
    pub const TREE_ERASE: u32 = 0x0063_6d60;

    /// The frame of the copy: `this` at `ebp - 0x14` (for the unwinding),
    /// the allocator byte at `ebp - 0xd` (its address handed to the tree
    /// copy).
    pub const COPY_FRAME: u32 = 0x14;

    /// `AssetListReader_copy_0064d9e0(this; src)` (thiscall, `ret 4`):
    /// the tree (`+0`, with the allocator tag at `ebp - 0xd`), the
    /// vectors at `+8`, `+0x14`, `+0x20` copied, the string at `+0x2c`
    /// made empty and assigned all of `src`'s. Returns `this`.
    ///
    /// junk read: `[ebp-0xd]` is never written; the tree copy gets its
    /// address only (0 in the port's frame, brief 2026-10-24).
    pub fn copy(h: &mut impl Host, ebp: u32, this: u32, src: u32) -> u32 {
        // the unwinding's copy of `this`
        h.write(ebp.wrapping_sub(0x14), this);
        let tag = ebp.wrapping_sub(0xd);
        h.call_sym(COPY_TREE, Conv::Thiscall(this), &[src, tag]);
        let at = |base: u32, off: u32| base.wrapping_add(off);
        h.call_sym(COPY_VECTOR8, Conv::Thiscall(at(this, 8)), &[at(src, 8)]);
        h.call_sym(
            COPY_VECTOR,
            Conv::Thiscall(at(this, 0x14)),
            &[at(src, 0x14)],
        );
        h.call_sym(
            COPY_VECTOR,
            Conv::Thiscall(at(this, 0x20)),
            &[at(src, 0x20)],
        );
        let s = at(this, 0x2c);
        h.write(at(s, 0x14), 0xfu32);
        h.write(at(s, 0x10), 0u32);
        h.write(s, 0u8);
        h.call_sym(
            ASSIGN_SUBSTR,
            Conv::Thiscall(s),
            &[at(src, 0x2c), 0, u32::MAX],
        );
        this
    }

    /// The frame of the destructor: the erase's iterator at `ebp - 4`
    /// (first holding `this`, the pushed `ecx`).
    pub const DTOR_FRAME: u32 = 4;

    /// A vector at `v` destroyed when set: its range (`0x00635c40(first,
    /// last)`), the block deleted, the three words cleared.
    fn vector_dtor(h: &mut impl Host, v: u32, range: bool) {
        let first: u32 = h.read(v);
        if first == 0 {
            return;
        }
        if range {
            let last: u32 = h.read(v.wrapping_add(4));
            h.call_sym(DESTROY_RANGE, Conv::Stdcall, &[first, last]);
        }
        let first: u32 = h.read(v);
        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[first]);
        for off in [0u32, 4, 8] {
            h.write(v.wrapping_add(off), 0u32);
        }
    }

    /// `AssetListReader_dtor_AssetListReader(this)` (thiscall): the
    /// string at `+0x2c` freed (when long) and emptied, the vectors at
    /// `+0x20` and `+0x14` (their elements destroyed) and `+8` freed, the
    /// tree erased (`erase(this; &it, head->left, head)`) and its head
    /// deleted.
    pub fn dtor(h: &mut impl Host, ebp: u32, this: u32) {
        let it = ebp.wrapping_sub(DTOR_FRAME);
        h.write(it, this);
        if h.read::<u32>(this.wrapping_add(0x40)) >= 0x10 {
            let p: u32 = h.read(this.wrapping_add(0x2c));
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[p]);
        }
        h.write(this.wrapping_add(0x40), 0xfu32);
        h.write(this.wrapping_add(0x3c), 0u32);
        h.write(this.wrapping_add(0x2c), 0u8);
        vector_dtor(h, this.wrapping_add(0x20), true);
        vector_dtor(h, this.wrapping_add(0x14), true);
        vector_dtor(h, this.wrapping_add(8), false);
        let head: u32 = h.read(this);
        let left: u32 = h.read(head);
        h.call_sym(TREE_ERASE, Conv::Thiscall(this), &[it, left, head]);
        let head: u32 = h.read(this);
        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[head]);
    }

    /// `steam_api` imports (cdecl): `SteamAPI_RegisterCallback`,
    /// `SteamUtils`, `SteamUserStats`, `SteamUser`.
    pub const REGISTER_CALLBACK_IAT: u32 = 0x00b0_c8b4;
    pub const STEAM_UTILS_IAT: u32 = 0x00b0_c89c;
    pub const STEAM_USER_STATS_IAT: u32 = 0x00b0_c898;
    pub const STEAM_USER_IAT: u32 = 0x00b0_c8a4;
    /// The three callbacks `(vtable, handler, id)`: user stats received,
    /// stored, achievement stored.
    pub const CALLBACKS: [(u32, u32, u32); 3] = [
        (0x00b4_b9f0, 0x006f_15f0, 0x44d),
        (0x00b4_ba00, 0x006f_16d0, 0x44e),
        (0x00b4_ba10, 0x006f_15e0, 0x44f),
    ];
    /// `g_AchievementRecords`' first word's address (`+0x44`) and count.
    pub const RECORDS: u32 = 0x00c3_5790;

    fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Cdecl, args)
    }

    /// `StatsAndAchievements_ctor(this)` (thiscall): the three 0x14-byte
    /// Steam callback objects at `+0`, `+0x14`, `+0x28` (vtable, flags
    /// byte `+4` and `+8` clear, `+0xc` this, `+0x10` the handler)
    /// registered (`SteamAPI_RegisterCallback(cb, id)`), the game id
    /// (`+0x3c`, `+0x40`) from `SteamUtils()->GetAppID()` (vtable `+0x24`;
    /// its low 24 bits), `+0x4c` = 0x100 (u16), `+0x44` the records,
    /// `+0x48` 0x22; when `SteamUserStats()` and a logged-on
    /// `SteamUser()` exist, `RequestCurrentStats()`. Returns `this`.
    pub fn stats_ctor(h: &mut impl Host, this: u32) -> u32 {
        for (k, (vt, handler, id)) in (0u32..).zip(CALLBACKS) {
            let cb = this.wrapping_add(0x14 * k);
            h.write(cb.wrapping_add(4), 0u8);
            h.write(cb.wrapping_add(8), 0u32);
            let vt = h.rebase(vt);
            h.write(cb, vt);
            let handler = h.rebase(handler);
            h.write(cb.wrapping_add(0xc), this);
            h.write(cb.wrapping_add(0x10), handler);
            import(h, REGISTER_CALLBACK_IAT, &[cb, id]);
        }
        let utils = import(h, STEAM_UTILS_IAT, &[]);
        let app = h.call_virtual(utils, 0x24 / 4, &[]);
        h.write(this.wrapping_add(0x3c), app & 0xff_ffff);
        h.write(this.wrapping_add(0x40), 0u32);
        h.write(this.wrapping_add(0x4c), 0x100u16);
        let records = h.rebase(RECORDS);
        h.write(this.wrapping_add(0x44), records);
        h.write(this.wrapping_add(0x48), 0x22u32);
        if import(h, STEAM_USER_STATS_IAT, &[]) != 0 && import(h, STEAM_USER_IAT, &[]) != 0 {
            let user = import(h, STEAM_USER_IAT, &[]);
            if h.call_virtual(user, 1, &[]).to_le_bytes()[0] != 0 {
                let stats = import(h, STEAM_USER_STATS_IAT, &[]);
                h.call_virtual(stats, 0, &[]);
            }
        }
        this
    }

    /// `BigFileStream_get` (cdecl), `BigFileStream_readVFile` (thiscall,
    /// `ret 0x18`), `crt_memcpy_thunk` (cdecl).
    pub const STREAM_GET: u32 = 0x0061_bdb0;
    pub const READ_VFILE: u32 = 0x0061_bfe0;
    pub const MEMCPY: u32 = 0x0094_9256;

    fn word64(h: &impl Host, at: u32) -> u64 {
        u64::from(h.read::<u32>(at.wrapping_add(4))) << 32 | u64::from(h.read::<u32>(at))
    }

    /// `FileSys_Read(vfile; dst, len)` (thiscall, `ret 8`): a read of
    /// `len` bytes at the file's position (`+0x10`, 64 bits), clipped to
    /// the entry's size (`[+0xc] + 8`, 64 bits; nothing left: 0). Bytes
    /// the file's buffer record (`[+0x18]`: data `+0`, its end `+8` and
    /// start `+0x10`, 64 bits each) holds are copied from it: all of
    /// them (returning `len`), a head starting inside it, or a tail
    /// ending inside it; the rest is read from the archive
    /// (`BigFileStream_readVFile(stream; dst, n, vfile, pos, [+8])`).
    /// Returns that read's count plus the bytes taken from the buffer.
    #[allow(clippy::cast_possible_truncation)] // the 64-bit positions' low words
    pub fn file_read(h: &mut impl Host, vfile: u32, dst: u32, len: u32) -> u32 {
        let pos = word64(h, vfile.wrapping_add(0x10));
        let entry: u32 = h.read(vfile.wrapping_add(0xc));
        let mut len = len;
        if pos.wrapping_add(u64::from(len)) > word64(h, entry.wrapping_add(8)) {
            let size_lo: u32 = h.read(entry.wrapping_add(8));
            len = size_lo.wrapping_sub(h.read(vfile.wrapping_add(0x10)));
            if len == 0 {
                return 0;
            }
        }
        let end = pos.wrapping_add(u64::from(len));
        let rec: u32 = h.read(vfile.wrapping_add(0x18));
        let data: u32 = h.read(rec);
        let rec_end = word64(h, rec.wrapping_add(8));
        let start = word64(h, rec.wrapping_add(0x10));
        let (pos_lo, start_lo) = (pos as u32, start as u32);
        let (mut to, mut n, mut at, mut taken) = (dst, len, pos, 0u32);
        if pos >= start && end <= rec_end {
            let src = data.wrapping_add(pos_lo.wrapping_sub(start_lo));
            h.call_sym(MEMCPY, Conv::Cdecl, &[dst, src, len]);
            return len;
        }
        if pos >= start && pos < rec_end {
            taken = (rec_end as u32).wrapping_sub(pos_lo);
            let src = data.wrapping_add(pos_lo.wrapping_sub(start_lo));
            h.call_sym(MEMCPY, Conv::Cdecl, &[dst, src, taken]);
            to = dst.wrapping_add(taken);
            at = rec_end;
            n = len.wrapping_sub(taken);
        } else if pos <= start && end > start && end <= rec_end {
            taken = (end as u32).wrapping_sub(start_lo);
            let out = dst.wrapping_sub(taken).wrapping_add(len);
            h.call_sym(MEMCPY, Conv::Cdecl, &[out, data, taken]);
            n = len.wrapping_sub(taken);
        }
        let stream = h.call_sym(STREAM_GET, Conv::Cdecl, &[]);
        let info: u32 = h.read(vfile.wrapping_add(8));
        let read = h.call_sym(
            READ_VFILE,
            Conv::Thiscall(stream),
            &[to, n, vfile, at as u32, (at >> 32) as u32, info],
        );
        read.wrapping_add(taken)
    }
}
