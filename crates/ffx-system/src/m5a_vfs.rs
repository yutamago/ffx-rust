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
}
