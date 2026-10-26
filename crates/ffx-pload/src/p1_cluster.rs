//! List `p1-load`, round 4: `PSerialization::PBinary::loadCluster`
//! (`0x0057bce0`, cdecl `(stream, cluster, ctx, key)`, `eax` 0 or an
//! error; Ghidra shows it `void`, `PCluster::LoadAssetFile` reads the
//! result).
//!
//! It reads a `PBinary` cluster file:
//! 1. 0x15 unless serialization is initialised (`0x00432020`); 0xb with
//!    `stderr`'s "VIRTUOS ERROR" line unless the stream is open (its
//!    slot 2).
//! 2. The 0x54-byte header into the frame (`0x00469980` clears it
//!    first): magic `PHYR`, byte-swapped through the swapper at
//!    `0x00c96b40` when it isn't (then a swapping file), its version
//!    (`+0xc`, `0x00464710`) and platform (`+0x44`, `0x005f0490`, or 0)
//!    checked (6 otherwise); the namespace block (`+8` bytes,
//!    `PhyreMalloc(size, 4)`, 0xd without) read whole, or after the
//!    header's extra bytes (header size `+4` above 0x54: kept in an
//!    array, then `0x00579010`), or partly from the header itself (below
//!    0x54).
//! 3. The block checked (`0x0047f9d0`, 6), the namespace mapping built
//!    in the frame (`0x004787e0`, verbose unless the "Generic" platform
//!    `0x005787d0` matches the header's version), the cluster header
//!    (`0x00578830`), the missing dynamic classes
//!    (`allocateMissingClassDescriptors(cluster, 1, 1)`), the reader in
//!    the frame (`0x00577760`) and its body (`0x0057c130`).
//! 4. The texture pass: the loader registered under `key` (`0x00576ef0`
//!    on `0x00c24f0c`, given the key word's address), else a new one
//!    (`PhyreMalloc(0x20, 4)`, `0x00463d10`, `0x004647a0(cluster,
//!    &header, ctx, key)`); then `loadClusterTextures` (`0x00465aa0`), the
//!    loader freed and a registered key dropped (`0x0057cb70`).
//! 5. The reader's and mapping's destructors, the blocks freed when
//!    owned.
//!
//! Two words the original reads were only partly written: the swap
//! flag (`ebp - 0x3c`, a byte) and the verbose flag (`ebp - 0x40`, a
//! byte, the rest the partial copy's length when the header is short),
//! both passed on as words. The port has 0 in their unwritten bytes
//! (junk reads, brief 2026-10-24); the callees test the low byte.

use ffx_graphic::host::{Conv, Host};

const IS_INITIALIZED: u32 = 0x0043_2020;
const HEADER_CTOR: u32 = 0x0046_9980;
const READ_EXACT: u32 = 0x0057_dd30;
const SWAPPER: u32 = 0x00c9_6b40;
const SWAP: u32 = 0x0043_d6a0;
const VERSION: u32 = 0x0046_4710;
const PLATFORM: u32 = 0x005f_0490;
const MALLOC: u32 = 0x0042_fb80;
const FREE: u32 = 0x0042_fb20;
const SWAP_BLOCK: u32 = 0x0047_d680;
const RESIZE: u32 = 0x0044_a010;
const READ_REST: u32 = 0x0057_9010;
const MEMCPY: u32 = 0x0094_9256;
const CHECK_BLOCK: u32 = 0x0047_f9d0;
const FIND_PLATFORM: u32 = 0x0057_87d0;
const GENERIC: u32 = 0x00b3_1388;
const GLOBAL_NAMESPACE: u32 = 0x0043_e2f0;
const MAPPING_INIT: u32 = 0x0047_87e0;
const READ_HEADER: u32 = 0x0057_8830;
const ALLOCATE_MISSING: u32 = 0x0047_c190;
const READER_CTOR: u32 = 0x0057_7760;
const READ_BODY: u32 = 0x0057_c130;
const LOADERS: u32 = 0x00c2_4f0c;
const FIND_LOADER: u32 = 0x0057_6ef0;
const DROP_LOADER: u32 = 0x0057_cb70;
const LOADER_CTOR: u32 = 0x0046_3d10;
const LOADER_INIT: u32 = 0x0046_47a0;
const LOAD_TEXTURES: u32 = 0x0046_5aa0;
const READER_DTOR: u32 = 0x0057_7f60;
const MAPPING_DTOR: u32 = 0x0047_9930;
const IOB_FUNC: u32 = 0x00b0_c350;
const FPRINTF: u32 = 0x00b0_c38c;
const NOT_OPEN: u32 = 0x00b3_134c;
const UNSET_NAME: u32 = 0x00b3_1340;

/// `PHYR` as the file's first word.
const MAGIC: u32 = 0x5048_5952;
/// The header's bytes.
const HEADER: u32 = 0x54;

/// The errors.
const NOT_INITIALIZED: u32 = 0x15;
const NOT_OPEN_ERR: u32 = 0xb;
const BAD_FILE: u32 = 6;
const NO_BLOCK: u32 = 0xd;

/// The ports over game memory.
pub mod bridge {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// The frame's bytes the port keeps at the original's offsets: from
    /// `ebp - 0xdc` (the mapping, the header, the arrays, the reader)
    /// up to and with the key's argument word at `ebp + 0x14`.
    pub const FRAME: u32 = 0xf4;
    /// Where `ebp` is in it.
    const EBP: u32 = 0xdc;

    /// A callee's `al`.
    fn al(eax: u32) -> bool {
        eax & 0xff != 0
    }

    /// Calls the CRT import in `slot` (cdecl).
    fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let slot = h.rebase(slot);
        let f: u32 = h.read(slot);
        h.call(f, Conv::Cdecl, args)
    }

    /// `loadCluster(stream, cluster, ctx, key)`: 0 or the error.
    /// `frame` is [`FRAME`] bytes at the original's `ebp - 0xdc`.
    pub fn load_cluster(h: &mut impl Host, args: [u32; 4], frame: u32) -> u32 {
        let [stream, cluster, ctx, key] = args;
        let ebp = frame + EBP;
        h.write(ebp + 0x14, key);
        if !al(h.call_sym(IS_INITIALIZED, Conv::Cdecl, &[])) {
            return NOT_INITIALIZED;
        }
        if !al(h.call_virtual(stream, 2, &[])) {
            let word: u32 = h.read(stream + 8);
            let low = u32::from(h.read::<u8>(stream + 8) & 1);
            let mut name = word.wrapping_sub(low);
            if name == 0 {
                name = h.rebase(UNSET_NAME);
            }
            let format = h.rebase(NOT_OPEN);
            let iob = import(h, IOB_FUNC, &[]);
            import(h, FPRINTF, &[iob.wrapping_add(0x40), format, name]);
            return NOT_OPEN_ERR;
        }
        let header = ebp - 0xbc;
        h.call_sym(HEADER_CTOR, Conv::Thiscall(header), &[]);
        // junk read: [ebp-0x3c] bytes 1-3 never written; 0 (brief 2026-10-24)
        h.write(ebp - 0x3c, 0u8);
        for off in [0x64, 0x54, 0x50, 0x5c, 0x58] {
            h.write(ebp - off, 0u32);
        }
        let r = read_and_map(h, (stream, cluster, ctx), ebp);
        // the extra header bytes and the namespace block, when owned
        let count: u32 = h.read(ebp - 0x5c);
        let extra: u32 = h.read(ebp - 0x58);
        if count >> 31 == 0 && extra != 0 {
            h.call_sym(FREE, Conv::Cdecl, &[extra]);
        }
        let size: u32 = h.read(ebp - 0x64);
        let data: u32 = h.read(ebp - 0x50);
        if size >> 31 == 0 && data != 0 {
            h.call_sym(FREE, Conv::Cdecl, &[data]);
        }
        r
    }

    /// Steps 2 to 4 (and the destructors): 0 or the error.
    fn read_and_map(h: &mut impl Host, (stream, cluster, ctx): (u32, u32, u32), ebp: u32) -> u32 {
        let header = ebp - 0xbc;
        let r = h.call_sym(READ_EXACT, Conv::Thiscall(stream), &[header, HEADER]);
        if r != 0 {
            return r;
        }
        h.write(ebp - 0x3c, 0u8);
        let swapper = h.rebase(SWAPPER);
        if h.read::<u32>(header) != MAGIC {
            h.call_sym(SWAP, Conv::Thiscall(swapper), &[header, 1]);
            if h.read::<u32>(header) != MAGIC {
                return BAD_FILE;
            }
            h.write(ebp - 0x3c, 1u8);
        }
        let version = h.call_sym(VERSION, Conv::Cdecl, &[]);
        if h.read::<u32>(ebp - 0xb0) != version {
            return BAD_FILE;
        }
        let platform = h.call_sym(PLATFORM, Conv::Cdecl, &[]);
        let file_platform: u32 = h.read(ebp - 0x78);
        if file_platform != platform && file_platform != 0 {
            return BAD_FILE;
        }
        let (data, header_size, swap_word, mut verbose_word) =
            match read_block(h, stream, ebp, swapper) {
                Ok(x) => x,
                Err(r) => return r,
            };
        if !al(h.call_sym(CHECK_BLOCK, Conv::Thiscall(data), &[])) {
            return BAD_FILE;
        }
        let generic = h.rebase(GENERIC);
        let platform = h.call_sym(FIND_PLATFORM, Conv::Cdecl, &[generic]);
        let quiet = platform != 0 && {
            let ids: u32 = h.read(platform + 0xc);
            h.read::<u32>(ebp - 0xb0) == h.read::<u32>(ids)
        };
        verbose_word = (verbose_word & !0xff) | u32::from(!quiet);
        h.write(ebp - 0x40, verbose_word);
        let ns = h.call_sym(GLOBAL_NAMESPACE, Conv::Cdecl, &[]);
        let mapping = ebp - 0xdc;
        h.call_sym(
            MAPPING_INIT,
            Conv::Thiscall(mapping),
            &[data, ns, verbose_word],
        );
        let mut r = h.call_sym(
            READ_HEADER,
            Conv::Cdecl,
            &[header, header_size, ebp - 0x5c, mapping, swap_word],
        );
        if r == 0 {
            r = h.call_sym(ALLOCATE_MISSING, Conv::Thiscall(mapping), &[cluster, 1, 1]);
        }
        if r == 0 {
            let reader = ebp - 0x38;
            h.call_sym(
                READER_CTOR,
                Conv::Thiscall(reader),
                &[cluster, mapping, header, swap_word],
            );
            r = h.call_sym(READ_BODY, Conv::Thiscall(reader), &[stream]);
            if r == 0 {
                r = textures(h, (stream, cluster, ctx), ebp);
            }
            h.call_sym(READER_DTOR, Conv::Thiscall(reader), &[]);
        }
        h.call_sym(MAPPING_DTOR, Conv::Thiscall(mapping), &[]);
        r
    }

    /// The namespace block (step 2's end): `(block, header size, swap
    /// word, verbose word)`, or the error.
    fn read_block(
        h: &mut impl Host,
        stream: u32,
        ebp: u32,
        swapper: u32,
    ) -> Result<(u32, u32, u32, u32), u32> {
        let header = ebp - 0xbc;
        let size: u32 = h.read(ebp - 0xb4);
        if size == 0 {
            return Err(BAD_FILE);
        }
        let data = h.call_sym(MALLOC, Conv::Cdecl, &[size, 4]);
        if data == 0 {
            return Err(NO_BLOCK);
        }
        let header_size: u32 = h.read(ebp - 0xb8);
        h.write(ebp - 0x50, data);
        h.write(ebp - 0x64, size);
        h.write(ebp - 0x54, size);
        let swap_word: u32 = h.read(ebp - 0x3c);
        let swap = swap_word & 0xff != 0;
        // [ebp-0x40]: the partial copy's length when the header is short
        // junk read: [ebp-0x40] bytes 1-3 never written otherwise; 0 (brief 2026-10-24)
        let mut verbose_word = 0u32;
        let r = if header_size == HEADER {
            let r = h.call_sym(READ_EXACT, Conv::Thiscall(stream), &[data, size]);
            if swap {
                h.call_sym(SWAP_BLOCK, Conv::Thiscall(data), &[]);
            }
            r
        } else {
            if swap {
                h.call_sym(SWAP, Conv::Thiscall(swapper), &[header, 1]);
            }
            if header_size > HEADER {
                if h.call_sym(RESIZE, Conv::Thiscall(ebp - 0x5c), &[header_size]) != 0 {
                    return Err(NO_BLOCK);
                }
                let extra: u32 = h.read(ebp - 0x58);
                for k in 0..21 {
                    let w: u32 = h.read(header + 4 * k);
                    h.write(extra + 4 * k, w);
                }
                let r = h.call_sym(
                    READ_EXACT,
                    Conv::Thiscall(stream),
                    &[extra + HEADER, header_size.wrapping_sub(HEADER)],
                );
                if r != 0 {
                    return Err(r);
                }
                h.call_sym(READ_REST, Conv::Cdecl, &[stream, swap_word, data, size])
            } else {
                let avail = HEADER - header_size;
                let n = avail.min(size);
                verbose_word = n;
                if avail != n {
                    return Err(BAD_FILE);
                }
                h.call_sym(MEMCPY, Conv::Cdecl, &[data, header + header_size, n]);
                h.call_sym(
                    READ_REST,
                    Conv::Cdecl,
                    &[stream, swap_word, data.wrapping_add(n), size - n],
                )
            }
        };
        if r != 0 {
            return Err(r);
        }
        Ok((data, header_size, swap_word, verbose_word))
    }

    /// Step 4: 0 or the error.
    fn textures(h: &mut impl Host, (stream, cluster, ctx): (u32, u32, u32), ebp: u32) -> u32 {
        let loaders = h.rebase(LOADERS);
        let found = h.call_sym(FIND_LOADER, Conv::Thiscall(loaders), &[ebp + 0x14]);
        let slot = if found == 0 {
            0
        } else {
            found.wrapping_add(0x10)
        };
        let loader = if slot != 0 {
            h.read(slot)
        } else {
            let block = h.call_sym(MALLOC, Conv::Cdecl, &[0x20, 4]);
            h.write(ebp - 0x40, block);
            h.write(ebp - 0x44, block);
            if block != 0 {
                h.call_sym(LOADER_CTOR, Conv::Thiscall(block), &[]);
            }
            let key: u32 = h.read(ebp + 0x14);
            let r = h.call_sym(
                LOADER_INIT,
                Conv::Thiscall(block),
                &[cluster, ebp - 0xbc, ctx, key],
            );
            if r != 0 {
                return r;
            }
            block
        };
        let r = h.call_sym(
            LOAD_TEXTURES,
            Conv::Thiscall(loader),
            &[stream, cluster, ctx],
        );
        if r != 0 {
            return r;
        }
        h.call_sym(FREE, Conv::Cdecl, &[loader]);
        if slot != 0 {
            h.call_sym(DROP_LOADER, Conv::Thiscall(loaders), &[ebp + 0x14]);
        }
        0
    }
}
