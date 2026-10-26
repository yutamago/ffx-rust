//! `ClassDynamicMesh`'s `fillDynamicBlocks` (vtable slot 5 at
//! `0x00b43eec`, `0x00672b80`; list p6-render-c, `m4-rest`): copies one
//! source record's vertex streams and indices into the data blocks the
//! renderer bound.
//!
//! Layouts used:
//! - the instance (`ClassDynamicMeshInstance_populatePreprocessData`): `+4` cleared
//!   on every path, `+0x10` the source (`+0x90` the record count, `+0x94`
//!   the 0x6c-byte records), `+0x14` the dynamic buffer id
//!   (`graphicDynGeoMemManagerGetCurrentBufferId`, -1 when off);
//! - a record: `+4` the vertex count, `+8` the index count, `+0xc`,
//!   `+0x10`, `+0x14`, `+0x18` the four vertex streams (12, 12, 16 and 8
//!   bytes per vertex), `+0x1c` the 16-bit indices; then one 28-byte
//!   override per dynamic buffer at `+0x2c + 0x1c id`, the same seven
//!   words (`+8`, its stream 0, set means the buffer has data);
//! - a data block (the four in `streams`): `+4` its element count, `+0x3c`
//!   its storage; the index block: `+8` its length, `+0xc` the format
//!   byte (8: 32-bit), `+0x38` its storage.

/// `memcpy` (cdecl `(dst, src, n)`).
pub const MEMCPY: u32 = 0x0094_9256;
/// `g_DynGeoBuffersOn` (`0x00c32694`, read by `0x006732a0` too): the
/// dynamic buffers are in use.
pub const BUFFERS_ON: u32 = 0x00c3_2694;
/// `__iob_func`'s and `fprintf`'s import slots (cdecl).
pub const IOB_FUNC: u32 = 0x00b0_c350;
pub const FPRINTF: u32 = 0x00b0_c38c;
/// `"indexDataBlock == NULL in fillDynamicBlocks.  VFX Setup error.\n"`.
pub const NO_INDEX_BLOCK: u32 = 0x00b4_3f10;
/// `"indicesCount <= indexBufferLength in fillDynamicBlocks.  VFX Setup
/// error.\n"`.
pub const TOO_MANY_INDICES: u32 = 0x00b4_3f50;

/// What one record (or its buffer override) hands over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Source {
    pub vertices: u32,
    pub indices: u32,
    /// The four vertex streams' sources.
    pub streams: [u32; 4],
    /// The 16-bit indices.
    pub index_data: u32,
}

impl Source {
    /// The record's own data (`words` the record's 27 words).
    #[must_use]
    pub fn of_record(words: &[u32; 27]) -> Self {
        Self {
            vertices: words[1],
            indices: words[2],
            streams: [words[3], words[4], words[5], words[6]],
            index_data: words[7],
        }
    }

    /// Buffer `id`'s override when it has data (its third word set): the
    /// override's 7 words at record word `11 + 7 id`.
    ///
    /// Only ids 0 and 1 lie inside the record (the manager has two
    /// buffers). For an id of 2 or more the original reads past its copy
    /// of the record, into its own frame (the stack cookie, the saved
    /// `ebp`, the return address, its arguments) and would copy from
    /// there; the port reads zeros instead (no override), the
    /// never-written-word rule's 0 (brief 2026-10-24).
    #[must_use]
    pub fn of_buffer(words: &[u32; 27], id: u32) -> Option<Self> {
        if id > 1 {
            return None;
        }
        let at = 11 + 7 * id as usize;
        let w = |k: usize| words[at + k];
        (w(2) != 0).then(|| Self {
            vertices: w(0),
            indices: w(1),
            streams: [w(2), w(3), w(4), w(5)],
            index_data: w(6),
        })
    }
}

pub mod bridge {
    //! The original over game memory.

    use ffx_graphic::host::{Conv, Host};

    use super::{BUFFERS_ON, FPRINTF, IOB_FUNC, MEMCPY, NO_INDEX_BLOCK, Source, TOO_MANY_INDICES};

    fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Cdecl, args)
    }

    /// The four blocks' counts (`+4`), each block that is set.
    fn set_counts(h: &mut impl Host, streams: u32, value: u32) {
        for k in 0..4 {
            let block: u32 = h.read(streams + 4 * k);
            if block != 0 {
                h.write(block + 4, value);
            }
        }
    }

    fn warn(h: &mut impl Host, format: u32) {
        let iob = import(h, IOB_FUNC, &[]);
        let format = h.rebase(format);
        import(h, FPRINTF, &[iob.wrapping_add(0x40), format]);
    }

    /// `ClassDynamicMesh_fillDynamicBlocks_00672b80` (`(index, inst,
    /// streams, index_block)`, `ret 0x10`; `ecx` unused) -> 0.
    ///
    /// With a source, `index` below its record count, a buffer id of 0 or
    /// more and the dynamic buffers on: the record's data (or the current
    /// buffer's override) is copied into the four blocks' storage
    /// (`+0x3c`; skipped where either side is null) and their counts set
    /// to the vertex count; the indices go into the index block (16 to 32
    /// bits for format 8, else copied as bytes) whose length becomes the
    /// index count. No index block, or more indices than it holds,
    /// clears the counts and prints a warning to `stderr`. Otherwise
    /// every count is cleared (the index block's length too). The
    /// instance's `+4` is cleared on every path.
    pub fn fill_dynamic_blocks(
        h: &mut impl Host,
        index: u32,
        inst: u32,
        streams: u32,
        index_block: u32,
    ) -> u32 {
        let id: u32 = h.read(inst + 0x14);
        let src: u32 = h.read(inst + 0x10);
        let usable = src != 0
            && {
                let n: i32 = h.read(src + 0x90);
                n >= 1 && index < n as u32
            }
            && h.read::<i32>(inst + 0x14) >= 0
            && h.read::<u32>(h.rebase(BUFFERS_ON)) != 0;
        if !usable {
            set_counts(h, streams, 0);
            h.write(index_block + 8, 0_u32);
            h.write(inst + 4, 0_u32);
            return 0;
        }
        let mut dst = [0_u32; 4];
        for (k, d) in dst.iter_mut().enumerate() {
            let block: u32 = h.read(streams + 4 * k as u32);
            if block != 0 {
                *d = h.read(block + 0x3c);
            }
        }
        let records: u32 = h.read(src + 0x94);
        let record = index.wrapping_mul(0x6c).wrapping_add(records);
        let mut words = [0_u32; 27];
        for (k, w) in words.iter_mut().enumerate() {
            *w = h.read(record + 4 * k as u32);
        }
        let s = Source::of_buffer(&words, id).unwrap_or_else(|| Source::of_record(&words));
        if index_block == 0 {
            set_counts(h, streams, 0);
            h.write(inst + 4, 0_u32);
            warn(h, NO_INDEX_BLOCK);
            return 0;
        }
        if s.indices > h.read::<u32>(index_block + 8) {
            set_counts(h, streams, 0);
            h.write(index_block + 8, 0_u32);
            h.write(inst + 4, 0_u32);
            warn(h, TOO_MANY_INDICES);
            return 0;
        }
        let n = s.vertices;
        let sizes = [n.wrapping_mul(12), n.wrapping_mul(12), n << 4, n << 3];
        for k in 0..4 {
            if dst[k] != 0 && s.streams[k] != 0 {
                h.call_sym(MEMCPY, Conv::Cdecl, &[dst[k], s.streams[k], sizes[k]]);
            }
        }
        set_counts(h, streams, n);
        if s.indices > h.read::<u32>(index_block + 8) {
            // the original's assertion: a store to address 0
            h.write(0, 0_u32);
        }
        let to: u32 = h.read(index_block + 0x38);
        if h.read::<u8>(index_block + 0xc) == 8 {
            for i in 0..s.indices {
                let v = u32::from(h.read::<u16>(s.index_data.wrapping_add(i << 1)));
                h.write(to.wrapping_add(i << 2), v);
            }
        } else {
            h.call_sym(MEMCPY, Conv::Cdecl, &[to, s.index_data, s.indices]);
        }
        h.write(index_block + 8, s.indices);
        h.write(inst + 4, 0_u32);
        0
    }
}
