//! M5 `system-a` (round 3): the archive (`BigFileStream`) name lookups
//! (`FileSys_Exists`, `BigFileStream_openFile`: the MD5 of the path below
//! the stream's root, through `CryptoAPI`, looked up in the five
//! archives), the synchronous read (`BigFileStream_readVFile`) and the
//! `VFile` pool of a `BigFileHandle` (`0x0061d9c0`).

/// `BigFileStream_sBigFileStreamInstacne` (the stream; 0 before set-up).
pub const STREAM_INSTANCE: u32 = 0x00cc_9c48;

/// The originals over game memory and the original callees.
pub mod bridge {
    use ffx_graphic::host::{Conv, Host};

    /// ADVAPI32 imports (stdcall): `CryptCreateHash`, `CryptHashData`,
    /// `CryptGetHashParam`, `CryptDestroyHash`.
    pub const CRYPT_CREATE_HASH_IAT: u32 = 0x00b0_c00c;
    pub const CRYPT_HASH_DATA_IAT: u32 = 0x00b0_c008;
    pub const CRYPT_GET_HASH_PARAM_IAT: u32 = 0x00b0_c010;
    pub const CRYPT_DESTROY_HASH_IAT: u32 = 0x00b0_c004;
    /// KERNEL32 imports (stdcall): `EnterCriticalSection`,
    /// `LeaveCriticalSection`, `ReleaseSemaphore`.
    pub const ENTER_CS_IAT: u32 = 0x00b0_c154;
    pub const LEAVE_CS_IAT: u32 = 0x00b0_c158;
    pub const RELEASE_SEMAPHORE_IAT: u32 = 0x00b0_c0f4;
    /// `Vbf_Contains` (thiscall, `ret 4`, `al`) and
    /// `BigFileHandle_getNewVFile` (thiscall, `ret 4`).
    pub const VBF_CONTAINS: u32 = 0x0061_dec0;
    pub const GET_NEW_VFILE: u32 = 0x0061_df10;
    /// `FileSys_NewReadRequest` (thiscall, `ret 0x18`),
    /// `BigFileStream_fileTaskRead` (stdcall, 3 words),
    /// `BigFileStream_fileTaskUncompress` (stdcall, 4 words),
    /// `Parser_Reset_0061c270` (thiscall).
    pub const NEW_READ_REQUEST: u32 = 0x0061_b7e0;
    pub const TASK_READ: u32 = 0x0061_b8d0;
    pub const TASK_UNCOMPRESS: u32 = 0x0061_ba80;
    pub const SLOT_RESET: u32 = 0x0061_c270;
    /// `VFile_assign` (thiscall, `ret 4`), `VFile_VFile` (thiscall),
    /// `VbfStream_Open` (thiscall, `ret 8`), `0x00949960` (cdecl: operator
    /// new with the `nothrow` tag at `0x0193e9e8`).
    pub const VFILE_ASSIGN: u32 = 0x0061_aa10;
    pub const VFILE_CTOR: u32 = 0x0061_a860;
    pub const VBF_STREAM_OPEN: u32 = 0x0061_acc0;
    pub const NEW_NOTHROW: u32 = 0x0094_9960;
    pub const NOTHROW: u32 = 0x0193_e9e8;

    fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Stdcall, args)
    }

    /// The frame of the name lookups: the hash size at `ebp - 0x1c`, the
    /// hash handle at `ebp - 0x18`, the digest at `ebp - 0x14` (16 bytes).
    pub const DIGEST_FRAME: u32 = 0x1c;

    /// The MD5 of `name + [this + 0x3c]` (the path below the stream's
    /// root) into the frame (`CryptCreateHash(provider, CALG_MD5)`,
    /// `CryptHashData`, `CryptGetHashParam(HP_HASHVAL)`); returns the
    /// digest's address and the hash handle.
    fn digest(h: &mut impl Host, ebp: u32, this: u32, name: u32) -> (u32, u32) {
        let size = ebp.wrapping_sub(0x1c);
        let handle = ebp.wrapping_sub(0x18);
        let md5 = ebp.wrapping_sub(0x14);
        let provider: u32 = h.read(this);
        h.write(size, 0x10u32);
        import(h, CRYPT_CREATE_HASH_IAT, &[provider, 0x8003, 0, 0, handle]);
        let path = name.wrapping_add(h.read(this.wrapping_add(0x3c)));
        let mut len = 0u32;
        while h.read::<u8>(path.wrapping_add(len)) != 0 {
            len = len.wrapping_add(1);
        }
        let hash: u32 = h.read(handle);
        import(h, CRYPT_HASH_DATA_IAT, &[hash, path, len, 0]);
        let hash: u32 = h.read(handle);
        import(h, CRYPT_GET_HASH_PARAM_IAT, &[hash, 2, md5, size, 0]);
        (md5, handle)
    }

    /// `FileSys_Exists(this; name)` (thiscall, `ret 4`) -> `al`: 0 before
    /// the stream exists; else whether one of the five archives (`+0x10`,
    /// the set ones) holds the name's digest (`Vbf_Contains`).
    pub fn exists(h: &mut impl Host, ebp: u32, this: u32, name: u32) -> u8 {
        if h.read::<u32>(h.rebase(super::STREAM_INSTANCE)) == 0 {
            return 0;
        }
        let (md5, handle) = digest(h, ebp, this, name);
        let mut found = 0;
        for k in 0..5u32 {
            let vbf: u32 = h.read(this.wrapping_add(0x10 + 4 * k));
            if vbf != 0 && h.call_sym(VBF_CONTAINS, Conv::Thiscall(vbf), &[md5]) & 0xff != 0 {
                found = 1;
                break;
            }
        }
        let hash: u32 = h.read(handle);
        import(h, CRYPT_DESTROY_HASH_IAT, &[hash]);
        found
    }

    /// `BigFileStream_openFile(this; name)` (thiscall, `ret 4`): 0 before
    /// the stream exists; else the first archive's
    /// `BigFileHandle_getNewVFile` for the name's digest that is not 0.
    pub fn open_file(h: &mut impl Host, ebp: u32, this: u32, name: u32) -> u32 {
        if h.read::<u32>(h.rebase(super::STREAM_INSTANCE)) == 0 {
            return 0;
        }
        let (md5, handle) = digest(h, ebp, this, name);
        let mut file = 0;
        for k in 0..5u32 {
            let vbf: u32 = h.read(this.wrapping_add(0x10 + 4 * k));
            if vbf != 0 {
                file = h.call_sym(GET_NEW_VFILE, Conv::Thiscall(vbf), &[md5]);
                if file != 0 {
                    break;
                }
            }
        }
        let hash: u32 = h.read(handle);
        import(h, CRYPT_DESTROY_HASH_IAT, &[hash]);
        file
    }

    /// `BigFileStream_readVFile(this; file, length, req, lo, hi, info)`
    /// (thiscall, `ret 0x18`): a read request (`FileSys_NewReadRequest`;
    /// none: 0), its read and uncompress tasks run here, then under the
    /// stream's lock (`+0x24`) the slot released (its use count down; at
    /// 0 reset, `+0x58` = 0 and the stream's semaphore (`+4`) released).
    /// Returns `length` when the request ended done (`+0x50` = 5), else 0.
    #[allow(clippy::too_many_arguments)] // the original's six words and `this`
    pub fn read_vfile(
        h: &mut impl Host,
        this: u32,
        file: u32,
        length: u32,
        req: u32,
        lo: u32,
        hi: u32,
        info: u32,
    ) -> u32 {
        let slot = h.call_sym(
            NEW_READ_REQUEST,
            Conv::Thiscall(this),
            &[file, length, req, lo, hi, info],
        );
        if slot == 0 {
            return 0;
        }
        h.call_sym(TASK_READ, Conv::Stdcall, &[0, slot, 0]);
        h.call_sym(TASK_UNCOMPRESS, Conv::Stdcall, &[0, slot, 0, 0]);
        let done = h.read::<u32>(slot.wrapping_add(0x50)) == 5;
        let lock = this.wrapping_add(0x24);
        import(h, ENTER_CS_IAT, &[lock]);
        let uses = h.read::<u32>(slot.wrapping_add(0x24)).wrapping_sub(1);
        h.write(slot.wrapping_add(0x24), uses);
        if uses == 0 {
            h.call_sym(SLOT_RESET, Conv::Thiscall(slot), &[]);
            h.write(slot.wrapping_add(0x58), 0u32);
            let sem: u32 = h.read(this.wrapping_add(4));
            import(h, RELEASE_SEMAPHORE_IAT, &[sem, 1, 0]);
        }
        import(h, LEAVE_CS_IAT, &[lock]);
        if done { length } else { 0 }
    }

    /// `0x0061d9c0(this; digest)` (thiscall, `ret 4`): under the handle's
    /// lock (`+0x44`), the first free one (byte `+0` clear) of the 15
    /// inline `VFile`s at `[+0x5c]` assigned the digest; none free: a new
    /// `VFile` (`operator new(0x20, nothrow)`, `VFile_VFile`) opened on
    /// the handle's stream (`VbfStream_Open(vf; [+0x40], this)`) and
    /// assigned, or 0 when the allocation fails.
    pub fn new_vfile(h: &mut impl Host, this: u32, digest: u32) -> u32 {
        let lock = this.wrapping_add(0x44);
        import(h, ENTER_CS_IAT, &[lock]);
        let files: u32 = h.read(this.wrapping_add(0x5c));
        let mut vf = 0;
        for i in 0..15u32 {
            let at = files.wrapping_add(i * 0x20);
            if h.read::<u8>(at) == 0 {
                h.call_sym(VFILE_ASSIGN, Conv::Thiscall(at), &[digest]);
                let files: u32 = h.read(this.wrapping_add(0x5c));
                vf = files.wrapping_add(i * 0x20);
                break;
            }
        }
        import(h, LEAVE_CS_IAT, &[lock]);
        if vf != 0 {
            return vf;
        }
        let tag = h.rebase(NOTHROW);
        let p = h.call_sym(NEW_NOTHROW, Conv::Cdecl, &[0x20, tag]);
        if p == 0 {
            return 0;
        }
        let vf = h.call_sym(VFILE_CTOR, Conv::Thiscall(p), &[]);
        if vf != 0 {
            let stream: u32 = h.read(this.wrapping_add(0x40));
            h.call_sym(VBF_STREAM_OPEN, Conv::Thiscall(vf), &[stream, this]);
            h.call_sym(VFILE_ASSIGN, Conv::Thiscall(vf), &[digest]);
        }
        vf
    }

    /// `MSVCR110!_fseeki64` / `MSVCR110!fread` (cdecl).
    pub const FSEEKI64_IAT: u32 = 0x00b0_c3f8;
    pub const FREAD_IAT: u32 = 0x00b0_c3f4;
    /// `operator new` / `operator_new_006304c0` / `operator delete`
    /// (cdecl).
    pub const OPERATOR_NEW: u32 = 0x0063_0310;
    pub const NEW_ARRAY: u32 = 0x0063_04c0;
    pub const OPERATOR_DELETE: u32 = 0x0063_0390;

    fn cdecl_import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Cdecl, args)
    }

    /// Block `i`'s packed size from the table (0 stands for 64 KB).
    fn block_size(h: &impl Host, sizes: u32, i: u32) -> u32 {
        let n = u32::from(h.read::<u16>(sizes.wrapping_add(i.wrapping_mul(2))));
        if n == 0 { 0x10000 } else { n }
    }

    /// `BigFileStream_fileTaskRead(_, slot, _)` (stdcall, 3 words): state
    /// 1; the file (`+0x44`) sought to the packed offset (`+0x28`, 64
    /// bits); then each block `fread` whole from the first (`+4`): the
    /// head record's (`+8`, when flagged) into its buffer, the body's
    /// (`+0xc`) into the slot's buffer (`+0x18`) at 64 KB steps with its
    /// last block into the body record's buffer, the tail's (`+0x10`)
    /// into its buffer (the next block, not counted); state 2.
    pub fn task_read(h: &mut impl Host, slot: u32) {
        let info: u32 = h.read(slot.wrapping_add(0x40));
        let mut block: u32 = h.read(slot.wrapping_add(4));
        h.write(slot.wrapping_add(0x50), 1u32);
        let sizes: u32 = h.read(info.wrapping_add(0x38));
        let len: u32 = h.read(slot.wrapping_add(0x38));
        let f: u32 = h.read(slot.wrapping_add(0x44));
        let lo: u32 = h.read(slot.wrapping_add(0x28));
        let hi: u32 = h.read(slot.wrapping_add(0x2c));
        cdecl_import(h, FSEEKI64_IAT, &[f, lo, hi, 0]);
        let head: u32 = h.read(slot.wrapping_add(8));
        let mut left = len;
        if h.read::<u8>(head) != 0 {
            let n = block_size(h, sizes, block);
            block = block.wrapping_add(1);
            let buf: u32 = h.read(head.wrapping_add(4));
            cdecl_import(h, FREAD_IAT, &[buf, n, 1, f]);
            let head: u32 = h.read(slot.wrapping_add(8));
            left = len.wrapping_sub(h.read(head.wrapping_add(8)));
        }
        let body: u32 = h.read(slot.wrapping_add(0xc));
        if h.read::<u8>(body) != 0 {
            let mut at = len.wrapping_sub(left).wrapping_add(0x10000);
            while len.wrapping_sub(at) >= 0x10000 {
                block = block.wrapping_add(1);
                let n = block_size(h, sizes, block.wrapping_sub(1));
                let base: u32 = h.read(slot.wrapping_add(0x18));
                cdecl_import(h, FREAD_IAT, &[base.wrapping_add(at), n, 1, f]);
                at = at.wrapping_add(0x10000);
            }
            block = block.wrapping_add(1);
            let n = block_size(h, sizes, block.wrapping_sub(1));
            let body: u32 = h.read(slot.wrapping_add(0xc));
            let buf: u32 = h.read(body.wrapping_add(4));
            cdecl_import(h, FREAD_IAT, &[buf, n, 1, f]);
        }
        let tail: u32 = h.read(slot.wrapping_add(0x10));
        if h.read::<u8>(tail) != 0 {
            let n = block_size(h, sizes, block);
            let buf: u32 = h.read(tail.wrapping_add(4));
            cdecl_import(h, FREAD_IAT, &[buf, n, 1, f]);
        }
        h.write(slot.wrapping_add(0x50), 2u32);
    }

    /// `0x0061add0(this)` (thiscall): a read slot's constructor: three
    /// buffer records (`+8`, `+0xc`, `+0x10`: 16 bytes from `operator
    /// new`, a 64 KB block at `+4`, byte `+0` clear; 0 when the record
    /// allocation fails), the slot's and the records' flags cleared, the
    /// counters `+0x14`, `+0x18`, `+0x1c`, `+0x24`, `+0x50` 0 (the buffer
    /// at `+0x20` is never freed here: `+0x1c` was just cleared). Returns
    /// `this`.
    pub fn slot_ctor(h: &mut impl Host, this: u32) -> u32 {
        h.write(this.wrapping_add(0x1c), 0u32);
        h.write(this.wrapping_add(0x24), 0u32);
        for off in [8u32, 0xc, 0x10] {
            let rec = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x10]);
            if rec != 0 {
                let block = h.call_sym(NEW_ARRAY, Conv::Cdecl, &[0x10000]);
                h.write(rec.wrapping_add(4), block);
                h.write(rec, 0u8);
            }
            h.write(this.wrapping_add(off), rec);
        }
        h.write(this, 0u8);
        for off in [8u32, 0xc, 0x10] {
            let rec: u32 = h.read(this.wrapping_add(off));
            h.write(rec, 0u8);
        }
        h.write(this.wrapping_add(0x14), 0u32);
        h.write(this.wrapping_add(0x18), 0u32);
        if h.read::<u32>(this.wrapping_add(0x1c)) != 0 {
            let p: u32 = h.read(this.wrapping_add(0x20));
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[p]);
        }
        h.write(this.wrapping_add(0x1c), 0u32);
        h.write(this.wrapping_add(0x50), 0u32);
        h.write(this.wrapping_add(0x24), 0u32);
        this
    }

    /// `fopen`, `fread`, `_fseeki64`, `fseek`, `fclose`, `printf` (cdecl);
    /// `CryptAcquireContextA`, `CryptReleaseContext` (stdcall).
    pub const FOPEN_IAT: u32 = 0x00b0_c3f0;
    pub const FSEEK_IAT: u32 = 0x00b0_c3fc;
    pub const FCLOSE_IAT: u32 = 0x00b0_c3ec;
    pub const PRINTF_IAT: u32 = 0x00b0_c380;
    pub const CRYPT_ACQUIRE_IAT: u32 = 0x00b0_c018;
    pub const CRYPT_RELEASE_IAT: u32 = 0x00b0_c014;
    /// The open mode (`"rb"`), the "not an archive" message, the magic
    /// (`SRYK`).
    pub const MODE_RB: u32 = 0x00b3_f69c;
    pub const MSG_BAD_MAGIC: u32 = 0x00b3_faf8;
    pub const MAGIC: u32 = 0x4b59_5253;
    /// `Md5Digest_Less` (thiscall, `ret 4`, `al`), `PRedBlackTree_reset_0061d8e0`
    /// (thiscall), `Vbf_InsertEntry` (thiscall, `ret 8`),
    /// `eh_vector_constructor_iterator` (stdcall, 5 words), `VFile_dtor_VFile`.
    pub const DIGEST_LESS: u32 = 0x0061_d6e0;
    pub const TREE_RESET: u32 = 0x0061_d8e0;
    pub const INSERT_ENTRY: u32 = 0x0061_de30;
    pub const VECTOR_CTOR: u32 = 0x0094_966a;
    pub const VFILE_DTOR: u32 = 0x0061_a960;

    /// The frame `BigFileHandle_initialize` needs: the stored digest at
    /// `ebp - 0x54`, the computed one at `ebp - 0x44`, the scratch words
    /// down to `ebp - 0x10` (the hash handle).
    pub const INIT_FRAME: u32 = 0x54;

    /// `BigFileHandle_initialize(this; path)` (thiscall, `ret 4`) -> `al`:
    /// the archive at `path` (copied to a new 0x104-byte `+0x40`) opened
    /// (`fopen`; 0 when it doesn't, or when its magic isn't `SRYK`, the
    /// file then left open with a message). Its header (the size word
    /// read into the caller's `path` slot, `slots`) read into a new block
    /// (`+0`), its MD5 (`CryptoAPI`, `ebp - 0x44`) checked against the
    /// one stored in the file's last 16 bytes (`ebp - 0x54`; differing:
    /// 0). Then the block-size table (`+0x38`), the block count (`+0x34`)
    /// and a stored-block flag per block (`+0x3c`: the size is not 0;
    /// cleared for each entry's last block when it ends on that block's
    /// stored size), every entry's name offset fixed up and its digest
    /// inserted (`Vbf_InsertEntry`), the 15 inline `VFile`s (`+0x5c`,
    /// after a count word) built and opened on the archive. Returns 1.
    #[allow(
        clippy::too_many_lines,
        clippy::many_single_char_names,
        clippy::cast_possible_truncation
    )] // the original's three tables; the sizes' block counts
    pub fn initialize(h: &mut impl Host, ebp: u32, this: u32, path: u32, slots: u32) -> u8 {
        let at = |off: u32| ebp.wrapping_sub(off);
        if path == 0 {
            return 0;
        }
        let copy = h.call_sym(NEW_ARRAY, Conv::Cdecl, &[0x104]);
        h.write(this.wrapping_add(0x40), copy);
        let mut k = 0;
        loop {
            let b: u8 = h.read(path.wrapping_add(k));
            h.write(copy.wrapping_add(k), b);
            k += 1;
            if b == 0 {
                break;
            }
        }
        let name: u32 = h.read(this.wrapping_add(0x40));
        let mode = h.rebase(MODE_RB);
        let f = cdecl_import(h, FOPEN_IAT, &[name, mode]);
        if f == 0 {
            return 0;
        }
        h.write(at(0x18), 0u32);
        h.write(slots, 0u32);
        cdecl_import(h, FREAD_IAT, &[at(0x18), 4, 1, f]);
        if h.read::<u32>(at(0x18)) != MAGIC {
            let msg = h.rebase(MSG_BAD_MAGIC);
            cdecl_import(h, PRINTF_IAT, &[msg]);
            return 0;
        }
        cdecl_import(h, FREAD_IAT, &[slots, 4, 1, f]);
        h.write(at(0x1c), 0x10u32);
        for k in 0..4u32 {
            h.write(at(0x54 - 4 * k), 0u32);
        }
        cdecl_import(h, FSEEKI64_IAT, &[f, 0xffff_fff0, 0xffff_ffff, 2]);
        let n: u32 = h.read(at(0x1c));
        cdecl_import(h, FREAD_IAT, &[at(0x54), n, 1, f]);
        let size: u32 = h.read(slots);
        let hdr = h.call_sym(NEW_ARRAY, Conv::Cdecl, &[size]);
        h.write(this, hdr);
        cdecl_import(h, FSEEK_IAT, &[f, 0, 0]);
        let (size, hdr): (u32, u32) = (h.read(slots), h.read(this));
        cdecl_import(h, FREAD_IAT, &[hdr, size, 1, f]);
        cdecl_import(h, FCLOSE_IAT, &[f]);
        for k in 0..4u32 {
            h.write(at(0x44 - 4 * k), 0u32);
        }
        import(h, CRYPT_ACQUIRE_IAT, &[at(0x20), 0, 0, 1, 0xf000_0000]);
        let prov: u32 = h.read(at(0x20));
        import(h, CRYPT_CREATE_HASH_IAT, &[prov, 0x8003, 0, 0, at(0x10)]);
        let (hash, size, hdr): (u32, u32, u32) = (h.read(at(0x10)), h.read(slots), h.read(this));
        import(h, CRYPT_HASH_DATA_IAT, &[hash, hdr, size, 0]);
        let hash: u32 = h.read(at(0x10));
        import(
            h,
            CRYPT_GET_HASH_PARAM_IAT,
            &[hash, 2, at(0x44), at(0x1c), 0],
        );
        let hash: u32 = h.read(at(0x10));
        import(h, CRYPT_DESTROY_HASH_IAT, &[hash]);
        let prov: u32 = h.read(at(0x20));
        import(h, CRYPT_RELEASE_IAT, &[prov, 0]);
        let less = |h: &mut _, a: u32, b: u32| {
            Host::call_sym(h, DIGEST_LESS, Conv::Thiscall(a), &[b]).to_le_bytes()[0] != 0
        };
        if less(h, at(0x44), at(0x54)) || less(h, at(0x54), at(0x44)) {
            return 0;
        }
        let hdr: u32 = h.read(this);
        h.write(at(0x2c), hdr);
        let n: u32 = h.read(hdr.wrapping_add(8));
        let digests = hdr.wrapping_add(0x10);
        h.write(at(0x14), digests);
        let records = digests.wrapping_add(n.wrapping_mul(16));
        let tail = records.wrapping_add(n.wrapping_mul(32));
        h.write(at(0x28), tail.wrapping_add(4));
        let sizes = tail.wrapping_add(h.read(tail));
        h.write(this.wrapping_add(0x38), sizes);
        let n: u32 = h.read(hdr.wrapping_add(8));
        h.write(at(0x24), records);
        let last = records.wrapping_add(n.wrapping_mul(32)).wrapping_sub(0x20);
        let blocks_of = |lo: u32, hi: u32| (u64::from(hi) << 32 | u64::from(lo)) >> 16;
        let (lo, hi): (u32, u32) = (h.read(last.wrapping_add(8)), h.read(last.wrapping_add(0xc)));
        let first: u32 = h.read(last);
        let mut count = first.wrapping_add(blocks_of(lo, hi) as u32);
        h.write(this.wrapping_add(0x34), count);
        if lo & 0xffff != 0 {
            count = count.wrapping_add(1);
            h.write(this.wrapping_add(0x34), count);
        }
        let flags = h.call_sym(NEW_ARRAY, Conv::Cdecl, &[count]);
        h.write(this.wrapping_add(0x3c), flags);
        let mut i = 0u32;
        while i < h.read::<u32>(this.wrapping_add(0x34)) {
            let sizes: u32 = h.read(this.wrapping_add(0x38));
            let stored = h.read::<u16>(sizes.wrapping_add(2 * i)) != 0;
            let flags: u32 = h.read(this.wrapping_add(0x3c));
            h.write(flags.wrapping_add(i), u8::from(stored));
            i += 1;
        }
        h.call_sym(TREE_RESET, Conv::Thiscall(this.wrapping_add(4)), &[]);
        let hdr: u32 = h.read(at(0x2c));
        let mut k = 0u32;
        while k < h.read::<u32>(hdr.wrapping_add(8)) {
            let rec = records.wrapping_add(32 * k);
            let fix: u32 = h.read(at(0x28));
            let name: u32 = h.read(rec.wrapping_add(0x18));
            h.write(rec.wrapping_add(0x18), name.wrapping_add(fix));
            let (lo, hi): (u32, u32) = (h.read(rec.wrapping_add(8)), h.read(rec.wrapping_add(0xc)));
            h.write(at(0x34), lo);
            let mut end = blocks_of(lo, hi) as u32;
            if lo & 0xffff != 0 {
                end = end.wrapping_add(1);
            }
            end = end.wrapping_add(h.read(rec));
            let sizes: u32 = h.read(this.wrapping_add(0x38));
            let last_size = u32::from(h.read::<u16>(sizes.wrapping_add(2 * end).wrapping_sub(2)));
            if last_size == lo & 0xffff {
                let flags: u32 = h.read(this.wrapping_add(0x3c));
                h.write(flags.wrapping_add(end).wrapping_sub(1), 0u8);
            }
            h.write(at(0x24), rec);
            let digest: u32 = h.read(at(0x14));
            h.call_sym(
                INSERT_ENTRY,
                Conv::Thiscall(this.wrapping_add(4)),
                &[digest, at(0x24)],
            );
            h.write(at(0x14), digest.wrapping_add(0x10));
            k += 1;
        }
        let block = h.call_sym(NEW_ARRAY, Conv::Cdecl, &[0x1e4]);
        h.write(at(0x2c), block);
        let files = if block == 0 {
            0
        } else {
            h.write(block, 0xfu32);
            let (ctor, dtor) = (h.rebase(VFILE_CTOR), h.rebase(VFILE_DTOR));
            let files = block.wrapping_add(4);
            h.call_sym(VECTOR_CTOR, Conv::Stdcall, &[files, 0x20, 0xf, ctor, dtor]);
            files
        };
        h.write(this.wrapping_add(0x5c), files);
        for k in 0..15u32 {
            let files: u32 = h.read(this.wrapping_add(0x5c));
            let stream: u32 = h.read(this.wrapping_add(0x40));
            h.call_sym(
                VBF_STREAM_OPEN,
                Conv::Thiscall(files.wrapping_add(0x20 * k)),
                &[stream, this],
            );
        }
        1
    }

    /// `zlib_uncompress(dst, &len, src, srclen)` (cdecl),
    /// `AtlFindStringResource_0061b270` (cdecl), `ResetToLauncher`
    /// (cdecl); `SysFreeString`, `SysAllocStringLen`, `ShowCursor`,
    /// `MessageBoxW` (stdcall); the message box's title.
    pub const UNCOMPRESS: u32 = 0x0040_5e20;
    /// `crt_memcpy_thunk` (cdecl).
    pub const MEMCPY: u32 = 0x0094_9256;
    pub const FIND_STRING: u32 = 0x0061_b270;
    pub const RESET_TO_LAUNCHER: u32 = 0x0042_f4d0;
    pub const SYS_FREE_STRING_IAT: u32 = 0x00b0_c61c;
    pub const SYS_ALLOC_STRING_LEN_IAT: u32 = 0x00b0_c620;
    pub const SHOW_CURSOR_IAT: u32 = 0x00b0_c688;
    pub const MESSAGE_BOX_IAT: u32 = 0x00b0_c684;
    pub const ERROR_TITLE: u32 = 0x00b3_f9fc;
    /// The frame of `BigFileStream_fileTaskUncompress`: the output length
    /// `zlib_uncompress` updates at `ebp - 0x10`.
    pub const UNCOMPRESS_FRAME: u32 = 0x10;

    /// One block into `dst`: inflated from `src` (`zlib_uncompress`, the
    /// length word `ebp - 0x10` set to 64 KB first and updated) when the
    /// block is stored compressed (`flags[block]`), else 64 KB copied.
    /// Returns the inflate's error (0 for a copy).
    fn block_into(
        h: &mut impl Host,
        ebp: u32,
        info: (u32, u32),
        block: u32,
        dst: u32,
        src: u32,
    ) -> u32 {
        let len = ebp.wrapping_sub(0x10);
        h.write(len, 0x10000u32);
        let (sizes, flags) = info;
        if h.read::<u8>(flags.wrapping_add(block)) != 0 {
            let packed = u32::from(h.read::<u16>(sizes.wrapping_add(2 * block)));
            h.call_sym(UNCOMPRESS, Conv::Cdecl, &[dst, len, src, packed])
        } else {
            h.call_sym(MEMCPY, Conv::Cdecl, &[dst, src, 0x10000]);
            0
        }
    }

    /// The slot's cache record (`[+0x14]`: its 64 KB block, the range
    /// `+0x10..+8` it holds) set to the block just inflated at the
    /// stream position `+0x30` (64 bits) and `len` bytes long.
    #[allow(clippy::cast_possible_truncation)] // the 64-bit range's words
    fn cache_block(h: &mut impl Host, slot: u32, len: u32) {
        let cache: u32 = h.read(slot.wrapping_add(0x14));
        let (lo, hi): (u32, u32) = (
            h.read(slot.wrapping_add(0x30)),
            h.read(slot.wrapping_add(0x34)),
        );
        h.write(cache.wrapping_add(0x10), lo);
        h.write(cache.wrapping_add(0x14), hi);
        let end = (u64::from(hi) << 32 | u64::from(lo)).wrapping_add(u64::from(len));
        h.write(cache.wrapping_add(8), end as u32);
        h.write(cache.wrapping_add(0xc), (end >> 32) as u32);
    }

    /// `BigFileStream_fileTaskUncompress(_, slot, _, _)` (stdcall, 4
    /// words): state 3; the blocks `fileTaskRead` read unpacked into the
    /// slot's buffer (`+0x18`): the head (inflated into the cache block,
    /// its part copied; or copied from the head record), the body in
    /// place (each block's packed bytes 64 KB on; the last from the body
    /// record), the tail (as the head); the stream position (`+0x30`)
    /// moved on. All `+0x38` bytes made and no inflate error: state 5;
    /// else the error box (string 0x7836 as a `BSTR` kept in the caller's
    /// third slot, `slots + 4`; `ShowCursor`, `MessageBoxW`), state 6 and
    /// `ResetToLauncher`.
    #[allow(clippy::too_many_lines, clippy::cast_possible_truncation)] // the three parts
    pub fn task_uncompress(h: &mut impl Host, ebp: u32, slot: u32, slots: u32) {
        let at = |off: u32| slot.wrapping_add(off);
        let len_at = ebp.wrapping_sub(0x10);
        let ret: u32 = h.read(at(0x40));
        let length: u32 = h.read(at(0x38));
        let mut block: u32 = h.read(at(4));
        h.write(at(0x50), 3u32);
        let info = (
            h.read::<u32>(ret.wrapping_add(0x38)),
            h.read::<u32>(ret.wrapping_add(0x3c)),
        );
        let mut left = length;
        let mut done = 0u64;
        let mut err = 0u32;
        h.write(len_at, 0u32);
        let head: u32 = h.read(at(8));
        if h.read::<u8>(head) != 0 {
            h.write(len_at, 0x10000u32);
            if h.read::<u8>(info.1.wrapping_add(block)) != 0 {
                let packed = u32::from(h.read::<u16>(info.0.wrapping_add(2 * block)));
                let src: u32 = h.read(head.wrapping_add(4));
                let cache: u32 = h.read(at(0x14));
                let dst: u32 = h.read(cache);
                err = h.call_sym(UNCOMPRESS, Conv::Cdecl, &[dst, len_at, src, packed]);
                let head: u32 = h.read(at(8));
                let cache: u32 = h.read(at(0x14));
                let src = h
                    .read::<u32>(head.wrapping_add(0xc))
                    .wrapping_add(h.read(cache));
                let (out, n): (u32, u32) = (h.read(at(0x18)), h.read(head.wrapping_add(8)));
                h.call_sym(MEMCPY, Conv::Cdecl, &[out, src, n]);
                let len: u32 = h.read(len_at);
                cache_block(h, slot, len);
                let pos = (u64::from(h.read::<u32>(at(0x34))) << 32
                    | u64::from(h.read::<u32>(at(0x30))))
                .wrapping_add(u64::from(len));
                h.write(at(0x30), pos as u32);
                h.write(at(0x34), (pos >> 32) as u32);
            } else {
                let src = h
                    .read::<u32>(head.wrapping_add(0xc))
                    .wrapping_add(h.read(head.wrapping_add(4)));
                let (out, n): (u32, u32) = (h.read(at(0x18)), h.read(head.wrapping_add(8)));
                h.call_sym(MEMCPY, Conv::Cdecl, &[out, src, n]);
            }
            let head: u32 = h.read(at(8));
            let n: u32 = h.read(head.wrapping_add(8));
            left = left.wrapping_sub(n);
            done = u64::from(n);
            block = block.wrapping_add(1);
        }
        let body: u32 = h.read(at(0xc));
        if h.read::<u8>(body) != 0 {
            let out: u32 = h.read(at(0x18));
            let mut dst = out.wrapping_add(done as u32);
            let blocks = left >> 16;
            let pos = (u64::from(h.read::<u32>(at(0x34))) << 32
                | u64::from(h.read::<u32>(at(0x30))))
            .wrapping_add(u64::from(blocks << 16));
            h.write(at(0x30), pos as u32);
            h.write(at(0x34), (pos >> 32) as u32);
            let mut count = blocks.wrapping_sub(1);
            while count != 0 {
                err |= block_into(h, ebp, info, block, dst, dst.wrapping_add(0x10000));
                done = done.wrapping_add(u64::from(h.read::<u32>(len_at)));
                dst = dst.wrapping_add(0x10000);
                block = block.wrapping_add(1);
                count = count.wrapping_sub(1);
            }
            let body: u32 = h.read(at(0xc));
            let src: u32 = h.read(body.wrapping_add(4));
            err |= block_into(h, ebp, info, block, dst, src);
            done = done.wrapping_add(u64::from(h.read::<u32>(len_at)));
            block = block.wrapping_add(1);
        }
        let tail: u32 = h.read(at(0x10));
        if h.read::<u8>(tail) != 0 {
            h.write(len_at, 0x10000u32);
            let out = h.read::<u32>(at(0x18)).wrapping_add(done as u32);
            if h.read::<u8>(info.1.wrapping_add(block)) != 0 {
                let packed = u32::from(h.read::<u16>(info.0.wrapping_add(2 * block)));
                let src: u32 = h.read(tail.wrapping_add(4));
                let cache: u32 = h.read(at(0x14));
                let scratch: u32 = h.read(cache);
                err |= h.call_sym(UNCOMPRESS, Conv::Cdecl, &[scratch, len_at, src, packed]);
                let (tail, cache): (u32, u32) = (h.read(at(0x10)), h.read(at(0x14)));
                let (scratch, n): (u32, u32) = (h.read(cache), h.read(tail.wrapping_add(8)));
                let out = h.read::<u32>(at(0x18)).wrapping_add(done as u32);
                h.call_sym(MEMCPY, Conv::Cdecl, &[out, scratch, n]);
                let len: u32 = h.read(len_at);
                cache_block(h, slot, len);
            } else {
                let (src, n): (u32, u32) =
                    (h.read(tail.wrapping_add(4)), h.read(tail.wrapping_add(8)));
                h.call_sym(MEMCPY, Conv::Cdecl, &[out, src, n]);
            }
            let tail: u32 = h.read(at(0x10));
            done = done.wrapping_add(u64::from(h.read::<u32>(tail.wrapping_add(8))));
        }
        if done == u64::from(h.read::<u32>(at(0x38))) && err == 0 {
            h.write(at(0x50), 5u32);
            return;
        }
        import(h, SYS_FREE_STRING_IAT, &[0]);
        let s = h.call_sym(FIND_STRING, Conv::Cdecl, &[0x7836]);
        let text = if s == 0 {
            0
        } else {
            let n = u32::from(h.read::<u16>(s));
            import(h, SYS_ALLOC_STRING_LEN_IAT, &[s.wrapping_add(2), n])
        };
        h.write(slots.wrapping_add(4), text);
        import(h, SHOW_CURSOR_IAT, &[1]);
        let title = h.rebase(ERROR_TITLE);
        import(h, MESSAGE_BOX_IAT, &[0, text, title, 0x10]);
        h.write(at(0x50), 6u32);
        h.call_sym(RESET_TO_LAUNCHER, Conv::Cdecl, &[]);
        import(h, SYS_FREE_STRING_IAT, &[text]);
    }

    /// `PhyreFIOS_LoadMagicfilePrx`'s callees: `PhyreFIOS_LoadsPrx`,
    /// `AtlFindStringResourceInstance_0064f970` (cdecl),
    /// `CStringW_LoadString_00652310`, the UTF-8 to UTF-16 conversion
    /// `0x00651800` and the string's `0x00654070` (thiscall, `ret 8`);
    /// the string manager; `free`; the format; the message id.
    pub const LOADS_PRX: u32 = 0x006f_9bc0;
    pub const FIND_STRING_INSTANCE: u32 = 0x0064_f970;
    pub const LOAD_STRING: u32 = 0x0065_2310;
    pub const TO_WIDE: u32 = 0x0065_1800;
    pub const STRING_INSERT: u32 = 0x0065_4070;
    pub const STRING_MANAGER: u32 = 0x00c6_4bf0;
    pub const FREE_IAT: u32 = 0x00b0_c370;
    pub const PRX_FORMAT: u32 = 0x00b4_2498;
    pub const MSG_PRX: u32 = 0x7833;
    /// The frame of `PhyreFIOS_LoadMagicfilePrx`: the message string at
    /// `ebp - 0x10c`, the conversion's pointer at `ebp - 0x108` and its
    /// inline buffer at `ebp - 0x104`.
    pub const PRX_FRAME: u32 = 0x10c;

    /// `PhyreFIOS_LoadMagicfilePrx(name, b, c)` (cdecl): the magic file's
    /// PRX loaded (`PhyreFIOS_LoadsPrx(name, b, c)`); when that fails its
    /// answer is returned at once. Else the error box: string 0x7833 (an
    /// empty `CStringW` from the string manager, loaded when the
    /// resource is found) with the file name (UTF-8 to UTF-16 into the
    /// frame, freed when it was allocated) put in by `0x00654070`,
    /// `ShowCursor`, `MessageBoxW`, `ResetToLauncher`, the string
    /// released (`lock xadd` of -1 on its count; freed through its manager
    /// at 0). Returns 0.
    pub fn load_magicfile_prx(h: &mut impl Host, ebp: u32, name: u32, b: u32, c: u32) -> u32 {
        let r = h.call_sym(LOADS_PRX, Conv::Cdecl, &[name, b, c]);
        if r != 0 {
            return r;
        }
        let text_at = ebp.wrapping_sub(0x10c);
        let conv_at = ebp.wrapping_sub(0x108);
        let inline = ebp.wrapping_sub(0x104);
        let manager = h.rebase(STRING_MANAGER);
        let nil = h.call_virtual(manager, 3, &[]);
        h.write(text_at, nil.wrapping_add(0x10));
        let inst = h.call_sym(FIND_STRING_INSTANCE, Conv::Cdecl, &[MSG_PRX, 0]);
        if inst != 0 {
            h.call_sym(LOAD_STRING, Conv::Thiscall(text_at), &[inst, MSG_PRX]);
        }
        h.write(conv_at, inline);
        h.call_sym(TO_WIDE, Conv::Thiscall(conv_at), &[name, 0xfde9]);
        let wide: u32 = h.read(conv_at);
        let fmt = h.rebase(PRX_FORMAT);
        h.call_sym(STRING_INSERT, Conv::Thiscall(text_at), &[fmt, wide]);
        let wide: u32 = h.read(conv_at);
        if wide != inline {
            cdecl_import(h, FREE_IAT, &[wide]);
        }
        import(h, SHOW_CURSOR_IAT, &[1]);
        let text: u32 = h.read(text_at);
        let title = h.rebase(ERROR_TITLE);
        import(h, MESSAGE_BOX_IAT, &[0, text, title, 0x10]);
        h.call_sym(RESET_TO_LAUNCHER, Conv::Cdecl, &[]);
        // ~CStringW: `lock xadd` of -1 on the reference count
        let data = text.wrapping_sub(0x10);
        let left = h.read::<u32>(data.wrapping_add(0xc)).wrapping_sub(1);
        h.write(data.wrapping_add(0xc), left);
        if left.cast_signed() <= 0 {
            let owner: u32 = h.read(data);
            let vtable: u32 = h.read(owner);
            let free: u32 = h.read(vtable.wrapping_add(4));
            h.call(free, Conv::Thiscall(owner), &[data]);
        }
        0
    }
}
