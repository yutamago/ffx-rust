//! `PhyreEngine` render methods, the list's last ones (M4 port,
//! h1-render-a, done in the p3-other-a continuation d): the dynamic mesh's
//! stream binding (its `_alloca`'d pointer array handed to a virtual
//! method) and the segment grouping of the renderer's dynamic streams.

use ffx_graphic::host::Host;

/// `0x0058c0d0` (thiscall `(ctx, 0, data, count, b)`, `ret 0x10`): the
/// streams' check the binding makes first; non-zero fails.
pub const STREAM_CHECK: u32 = 0x0058_c0d0;
/// `PhyreFindEntry16_004bd770` (thiscall `(this, key)`, `ret 4`).
pub const FIND_ENTRY16: u32 = 0x004b_d770;
/// `Phyre_Ctor_0055dc40` (thiscall `(this)` -> `this`): a data block view.
pub const BLOCK_VIEW_CTOR: u32 = 0x0055_dc40;
/// `PDataBlock_setToExternalBuffer_00483810` (thiscall `(this, buffer,
/// count)`, `ret 8`).
pub const SET_TO_EXTERNAL_BUFFER: u32 = 0x0048_3810;
/// `Phyre_PGeometry_FormatSize_0055bc40` (cdecl `(format)`).
pub const FORMAT_SIZE: u32 = 0x0055_bc40;

pub mod bridge {
    //! The originals over game memory.

    use super::{
        BLOCK_VIEW_CTOR, FIND_ENTRY16, FORMAT_SIZE, Host, SET_TO_EXTERNAL_BUFFER, STREAM_CHECK,
    };
    use ffx_graphic::host::Conv;

    fn thiscall(h: &mut impl Host, sym: u32, this: u32, args: &[u32]) -> u32 {
        let f = h.rebase(sym);
        h.call(f, Conv::Thiscall(this), args)
    }

    /// The number of stream pointers [`bind_streams`] puts in its buffer
    /// (the original's `_alloca(n * 4)`): the streams array's count.
    #[must_use]
    pub fn stream_count(h: &impl Host, streams: u32) -> u32 {
        h.read::<u32>(streams) & 0x7fff_ffff
    }

    /// `0x00498d20` (thiscall `(mesh, ctx, _, &result, &segment_info,
    /// segment, a, streams, b)`, `ret 0x20`; `PDynamicMesh`'s and its
    /// derived classes' vtable slot 2): both outputs cleared; the streams
    /// (`{count | flag, data}` of 0x40-byte entries) checked through
    /// `ctx` (`0x0058c0d0`, its non-zero result returned); then `buffer`
    /// (the original's `_alloca`'d array, [`stream_count`] words) gets each
    /// stream with a non-zero `+0x38`, else 0, and the first of them whose
    /// segment record's byte `+8` is set is remembered; the mesh's slot 5
    /// `(segment, a, buffer, b)` result goes to `*result`, the remembered
    /// stream's `+4` to `*segment_info`. Returns 0.
    #[allow(clippy::too_many_arguments)]
    pub fn bind_streams(
        h: &mut impl Host,
        this: u32,
        ctx: u32,
        result: u32,
        info: u32,
        segment: u32,
        a: u32,
        streams: u32,
        b: u32,
        buffer: u32,
    ) -> u32 {
        h.write(result, 0u32);
        h.write(info, 0u32);
        let n = stream_count(h, streams);
        if n != 0 {
            let data: u32 = h.read(streams + 4);
            let r = thiscall(h, STREAM_CHECK, ctx, &[0, data, n, b]);
            if r != 0 {
                return r;
            }
        }
        let mut found = 0u32;
        let mut s: u32 = h.read(streams + 4);
        for i in 0..n {
            let slot = buffer.wrapping_add(4 * i);
            if h.read::<u32>(s + 0x38) == 0 {
                h.write(slot, 0u32);
            } else {
                h.write(slot, s);
                if found == 0 {
                    let firsts: u32 = h.read(this + 0x14);
                    let first: u32 = h.read(firsts.wrapping_add(segment.wrapping_mul(16)));
                    let k = first.wrapping_add(i);
                    let records: u32 = h.read(this + 0xc);
                    let rec: u32 = h.read(records.wrapping_add(k.wrapping_mul(12)).wrapping_add(4));
                    if h.read::<u8>(rec + 8) != 0 {
                        found = s;
                    }
                }
            }
            s = s.wrapping_add(0x40);
        }
        let r = h.call_virtual(this, 5, &[segment, a, buffer, b]);
        h.write(result, r);
        if found != 0 {
            let v: u32 = h.read(found + 4);
            h.write(info, v);
        }
        0
    }

    /// `0x00498a30` (thiscall `(mesh, finder, segment, views, out)`,
    /// `ret 0x10`; called by `PRendererBase::populateContextParameter
    /// CaptureBufferWithDynamicStreams`): the segment's records (`{count
    /// first, n, stride}` at `+0x14`, 12-byte records at `+0xc`) split
    /// into runs that end before a zero first word; for each run a data
    /// block view constructed in the next 0x40 bytes of `views` and set
    /// to the run's records, and, when one of the run's keys (`+4`) is
    /// found by `finder` (`0x004bd770`), sized from the run's last record
    /// (`(first + FormatSize(format)) * stride`, kind 4); `out[k]` gets the
    /// view, or 0 when no key was found. Returns the number of runs.
    pub fn group_segment(
        h: &mut impl Host,
        this: u32,
        finder: u32,
        segment: u32,
        views: u32,
        out: u32,
    ) -> u32 {
        let seg: u32 = h.read::<u32>(this + 0x14).wrapping_add(segment << 4);
        let total = h.read::<u32>(this + 8) & 0x7fff_ffff;
        let first: u32 = h.read(seg);
        let n: u32 = h.read(seg + 4);
        let base = if first < total {
            h.read::<u32>(this + 0xc)
                .wrapping_add(first.wrapping_mul(12))
        } else {
            0
        };
        let stride: u32 = h.read(seg + 8);
        let record = |i: u32| base.wrapping_add(i.wrapping_mul(12));
        let mut runs = 0u32;
        let mut views = views;
        let mut i = 0u32;
        if n == 0 {
            return 0;
        }
        loop {
            let start = i;
            i += 1;
            while i < n && h.read::<u32>(record(i)) != 0 {
                i += 1;
            }
            let len = i - start;
            let mut found = false;
            for k in start..i {
                if !found {
                    let key: u32 = h.read(record(k) + 4);
                    found = thiscall(h, FIND_ENTRY16, finder, &[key]) != 0;
                }
            }
            let place = views;
            views = views.wrapping_add(0x40);
            let view = if place == 0 {
                0
            } else {
                thiscall(h, BLOCK_VIEW_CTOR, place, &[])
            };
            h.write(view + 0x15, 2u8);
            thiscall(h, SET_TO_EXTERNAL_BUFFER, view, &[record(start), len]);
            let view = if found {
                let last = record(i).wrapping_sub(12);
                let w0: u32 = h.read(last);
                let format: u8 = h.read(last + 8);
                let size = {
                    let f = h.rebase(FORMAT_SIZE);
                    h.call(f, Conv::Cdecl, &[u32::from(format)])
                };
                let count = w0.wrapping_add(size);
                h.write(view + 0x38, count.wrapping_mul(stride));
                h.write(view + 0x3c, 0u32);
                h.write(view + 0x14, 4u8);
                h.write(view, count);
                h.write(view + 4, stride);
                view
            } else {
                0
            };
            h.write(out.wrapping_add(runs * 4), view);
            runs += 1;
            if i >= n {
                return runs;
            }
        }
    }
}

/// `PType_find_004c0f10` (cdecl `(name)`) and `PType_register_004c1000`
/// (cdecl `(entry)`), the texture format list's lookup and registration.
pub const FORMAT_FIND: u32 = 0x004c_0f10;
pub const FORMAT_REGISTER: u32 = 0x004c_1000;
/// The list head and the table by index (`0x00ca2f08[entry+8]`).
pub const FORMAT_LIST: u32 = 0x00ca_2f04;
pub const FORMAT_TABLE: u32 = 0x00ca_2f08;
/// The texture format entries `{name, next, index, ...}` (0x28 bytes):
/// the first six registered inline, the other 27 through
/// [`FORMAT_REGISTER`].
pub const FORMAT_FIRST: u32 = 0x00ca_2fcc;
pub const FORMAT_ENTRY: u32 = 0x28;
pub const FORMAT_INLINE: u32 = 6;
pub const FORMAT_COUNT: u32 = 33;

pub mod formats {
    //! `0x004c1050`, over game memory.

    use super::{
        FORMAT_COUNT, FORMAT_ENTRY, FORMAT_FIND, FORMAT_FIRST, FORMAT_INLINE, FORMAT_LIST,
        FORMAT_REGISTER, FORMAT_TABLE,
    };
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        let f = h.rebase(sym);
        h.call(f, Conv::Cdecl, args)
    }

    /// `0x004c1050` (cdecl `()`, called by `PUtilityRendering::
    /// initialize`): the texture format entries registered in order: each
    /// stored at the table by its index, then, unless its name is null
    /// (0x11) or already listed (0x12, `0x004c0f10`), pushed at the head of
    /// the list. The first six are `PType_register` inlined with the
    /// compiler's interleaving (the next entry's index read and the
    /// previous link stores between); the others call it. Returns 0 or
    /// the failing code.
    pub fn register_all(h: &mut impl Host) -> u32 {
        let first = h.rebase(FORMAT_FIRST);
        let entry = |k: u32| first + k * FORMAT_ENTRY;
        let list = h.rebase(FORMAT_LIST);
        let table = h.rebase(FORMAT_TABLE);
        for k in 0..FORMAT_INLINE {
            let e = entry(k);
            let index: u32 = h.read(e + 8);
            match k {
                0 => {}
                1 => h.write(list, entry(0)),
                _ => h.write(entry(k - 1) + 4, entry(k - 2)),
            }
            h.write(table.wrapping_add(index.wrapping_mul(4)), e);
            let name: u32 = h.read(e);
            if k >= 2 {
                h.write(list, entry(k - 1));
            }
            if name == 0 {
                return 0x11;
            }
            if cdecl(h, FORMAT_FIND, &[name]) != 0 {
                return 0x12;
            }
            if k == 0 {
                let head: u32 = h.read(list);
                h.write(e + 4, head);
            }
        }
        let last = entry(FORMAT_INLINE - 1);
        h.write(last + 4, entry(FORMAT_INLINE - 2));
        h.write(list, last);
        let mut eax = 0;
        for k in FORMAT_INLINE..FORMAT_COUNT {
            eax = cdecl(h, FORMAT_REGISTER, &[entry(k)]);
            if eax != 0 {
                return eax;
            }
        }
        eax
    }
}
