//! M5 finish, WP5: originals the ports still called in Unicorn, by the
//! blocks gate run 1 emulated for them (ffx-rust's profile; docs/host.md,
//! "Rust and Unicorn"). Each follows its original's machine code
//! (`objdump -d FFX.exe`).

use ffx_bridge::Mem;

/// The render thread's job list (`PRenderThread_processRenderCommands`
/// builds two with `eh vector constructor iterator`, element size
/// `0x200c`): a 16-bit count at `+0`, a word at `+4`, 1024 two-word
/// entries from `+8`, a word at `+0x2008`.
pub const JOB_LIST_ENTRIES: u32 = 0x400;

/// `PRenderThread_JobList_ctor_00613f70(this)` (thiscall, no stack words):
/// the count (`u16` at `+0`, the two bytes after it untouched), the word
/// at `+4`, every entry and the word at `+0x2008` zeroed; `this` in
/// `eax`. 107 M of the emulated blocks of gate run 1 (8 replays), the most
/// of any `FFX.exe` function the ports call.
pub fn job_list_ctor(m: &mut impl Mem, this: u32) -> u32 {
    m.write(this, 0u16);
    m.write(this + 4, 0u32);
    let entries = vec![0u8; (8 * JOB_LIST_ENTRIES) as usize];
    m.write_bytes(this + 8, &entries);
    m.write(this + 8 + 8 * JOB_LIST_ENTRIES, 0u32);
    this
}

/// `batchSetClothVertexBuffers` (`0x006365f0`) and its callees.
pub mod cloth {
    use ffx_graphic::host::{Conv, Host};

    /// The cloth characters' table (`graphicCharacterInstanceSetClothVertexBuffers`'s).
    pub const TABLE: u32 = 0x00cc_b254;
    /// A cloth instance's vertex-buffer object: its segment-indexed array
    /// (`thiscall(obj) -> array`).
    pub const SEGMENT_ARRAY: u32 = 0x004d_45a0;
    /// `thiscall(obj; type) -> the cloth view of that type, or 0`, with
    /// the type descriptor at `0x00b41974`.
    pub const FIND_VIEW: u32 = 0x0056_cbf0;
    pub const VIEW_TYPE: u32 = 0x00b4_1974;
    /// `thiscall(target; view, cloth)`: binds the cloth object to the view.
    pub const BIND_VIEW: u32 = 0x0063_8720;
    /// The stream's map and unmap (`thiscall(stream; &out)` fills
    /// `{data, bytes}`; `thiscall(stream; next, &out) -> 0 when done`).
    pub const MAP_STREAM: u32 = 0x0043_4c50;
    pub const UNMAP_STREAM: u32 = 0x0056_4cb0;
    /// `crt_memcpy_thunk` (cdecl).
    pub const MEMCPY: u32 = 0x0094_9256;
    /// The mesh buffer's position and range updates for a list with a
    /// buffer per instance (`thiscall(buffer; ...)`).
    pub const SET_POSITIONS: u32 = 0x006e_2b10;
    pub const SET_RANGE: u32 = 0x006e_2ac0;
    /// A stream item whose `+4` is this semantic gets the cloth's vertices.
    pub const CLOTH_SEMANTIC: u32 = 0x00c9_8650;

    /// Bytes per segment, per stream and per stream item.
    const SEGMENT: u32 = 0x6c;
    const STREAM: u32 = 0x40;
    const ITEM: u32 = 0xc;
    const COUNT_MASK: u32 = 0x7fff_ffff;

    fn count(h: &impl Host, at: u32) -> u32 {
        h.read::<u32>(at) & COUNT_MASK
    }

    /// `batchSetClothVertexBuffers(_, &first, &last)` (cdecl; `out` is
    /// the original's two-word local at `ebp - 0x4c` the stream map fills):
    /// for each cloth character `i` from `*first` to `*last` (signed,
    /// `*last` read again after each) whose table entry is set, with its
    /// instance list (`+0`) and parallel list (`+4`) set, `+8` 0 and `+0xc`
    /// 2: for each of its `+0x14` instances `k` with a cloth table
    /// (`entry + 0x30 + 4k`) and a parallel entry (`parallel + 4 + 4k`):
    /// for each segment `s` of the instance's mesh whose `+0x20` and
    /// `+0x24` halves are not -1 and that has a cloth object (`table + 4 +
    /// 4s`) and a parallel word (`parallel_k + 4 + 4s`): the view of
    /// [`SEGMENT_ARRAY`]`[segment index]` gets the cloth object
    /// ([`FIND_VIEW`], [`BIND_VIEW`]); then each stream of the segment
    /// (`+0x2c`, `+0x28` of them) and each of its items with the
    /// [`CLOTH_SEMANTIC`]: with the instance list's count (`+8`) at most
    /// `k`, the cloth's vertices (`cloth + 0x4c`, `bytes`) copied into the
    /// mapped stream and the stream unmapped at `+0x10 + 1` (0 when that
    /// reaches `+0x18`), the stream's `+0x10`/`+0x14` set when the unmap
    /// says done; with more, the instance's buffer (`list + 0xc`, `+0x10`)
    /// gets the positions and the range (`cloth + 0x44` vertices of 16
    /// bytes) for segment `s`. Each character done has its `+0x10` cleared.
    /// `eax` is `last`.
    #[allow(clippy::too_many_lines)] // the original's five nested loops
    pub fn batch_set_cloth_vertex_buffers(
        h: &mut impl Host,
        first: u32,
        last: u32,
        out: u32,
    ) -> u32 {
        let table = h.rebase(TABLE);
        let semantic = h.rebase(CLOTH_SEMANTIC);
        let mut i: i32 = h.read(first);
        while i <= h.read::<i32>(last) {
            let slot = table.wrapping_add(i.cast_unsigned().wrapping_mul(4));
            let entry: u32 = h.read(slot);
            i = i.wrapping_add(1);
            if entry == 0 {
                continue;
            }
            let list: u32 = h.read(entry);
            if list == 0 || h.read::<u32>(entry + 8) != 0 || h.read::<u32>(entry + 0xc) != 2 {
                continue;
            }
            let parallel: u32 = h.read(entry + 4);
            if parallel == 0 {
                continue;
            }
            let instances: i32 = h.read(entry + 0x14);
            for k in 0..instances.max(0).cast_unsigned() {
                set_instance(h, entry, list, parallel, k, semantic, out);
            }
            let entry: u32 = h.read(slot);
            h.write(entry + 0x10, 0u32);
        }
        last
    }

    /// One instance `k` of a character (the original's middle loops).
    fn set_instance(
        h: &mut impl Host,
        entry: u32,
        list: u32,
        parallel: u32,
        k: u32,
        semantic: u32,
        out: u32,
    ) {
        let instances: u32 = h.read(list + 4);
        let instance: u32 = h.read(instances.wrapping_add(4 * k));
        let mesh: u32 = h.read(instance);
        let buffer: u32 = h.read(instance + 0x10);
        let segments = count(h, mesh);
        let cloths: u32 = h.read(entry + 0x30 + 4 * k);
        let words: u32 = h.read(parallel + 4 + 4 * k);
        if cloths == 0 || words == 0 || segments.cast_signed() <= 0 {
            return;
        }
        for s in 0..segments {
            let at: u32 = h.read(mesh + 4);
            let segment = at.wrapping_add(SEGMENT * s);
            let streams = count(h, segment + 0x28);
            if h.read::<u16>(segment + 0x20) == 0xffff || h.read::<u16>(segment + 0x24) == 0xffff {
                continue;
            }
            let cloth_at = cloths + 4 + 4 * s;
            let cloth: u32 = h.read(cloth_at);
            if cloth == 0 || h.read::<u32>(words + 4 + 4 * s) == 0 {
                continue;
            }
            let array = h.call_sym(SEGMENT_ARRAY, Conv::Thiscall(buffer), &[]);
            let index: u32 = h.read(segment);
            let view_of: u32 = h.read(array.wrapping_add(index.wrapping_mul(4)));
            if view_of != 0 {
                let of: u32 = h.read(view_of);
                let view_type = h.rebase(VIEW_TYPE);
                let view = h.call_sym(FIND_VIEW, Conv::Thiscall(of), &[view_type]);
                if view != 0 {
                    let target: u32 = h.read(view_of + 4);
                    h.call_sym(BIND_VIEW, Conv::Thiscall(target), &[view, cloth]);
                }
            }
            if streams.cast_signed() <= 0 {
                continue;
            }
            for n in 0..streams {
                let first: u32 = h.read(segment + 0x2c);
                let stream = first.wrapping_add(STREAM * n);
                let items = count(h, stream + 8);
                for t in 0..items {
                    let at: u32 = h.read(stream + 0xc);
                    if h.read::<u32>(at.wrapping_add(ITEM * t) + 4) != semantic {
                        continue;
                    }
                    let held = count(h, list + 8);
                    if held > k {
                        set_buffer(h, list, k, cloth_at, s);
                    } else {
                        fill_stream(h, stream, cloth_at, out);
                    }
                }
            }
        }
    }

    /// The copy into a mapped stream (the instance list holds at most `k`).
    fn fill_stream(h: &mut impl Host, stream: u32, cloth_at: u32, out: u32) {
        h.call_sym(MAP_STREAM, Conv::Thiscall(stream), &[out]);
        let (data, bytes): (u32, u32) = (h.read(out), h.read(out + 4));
        let cloth: u32 = h.read(cloth_at);
        let vertices: u32 = h.read(cloth + 0x4c);
        h.call_sym(MEMCPY, Conv::Cdecl, &[data, vertices, bytes]);
        let next = h.read::<u32>(stream + 0x10).wrapping_add(1);
        let end = count(h, stream + 0x18);
        let next = if next == end { 0 } else { next };
        if h.call_sym(UNMAP_STREAM, Conv::Thiscall(stream), &[next, out]) == 0 {
            h.write(stream + 0x10, next);
            h.write(stream + 0x14, 0u8);
        }
    }

    /// The instance's own buffer (the list holds more than `k`).
    fn set_buffer(h: &mut impl Host, list: u32, k: u32, cloth_at: u32, s: u32) {
        let buffers: u32 = h.read(list + 0xc);
        let owner: u32 = h.read(buffers.wrapping_add(4 * k));
        let cloth: u32 = h.read(cloth_at);
        let target: u32 = h.read(owner + 0x10);
        h.call_sym(SET_POSITIONS, Conv::Thiscall(target), &[cloth + 0x48, s]);
        let cloth: u32 = h.read(cloth_at);
        let vertices: u32 = h.read(cloth + 0x4c);
        let bytes = h.read::<u32>(cloth + 0x44) << 4;
        let target: u32 = h.read(owner + 0x10);
        h.call_sym(SET_RANGE, Conv::Thiscall(target), &[vertices, bytes, s]);
    }
}
