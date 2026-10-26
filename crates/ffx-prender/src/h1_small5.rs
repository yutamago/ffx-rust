//! `PhyreEngine` render methods, the list's last ones (M4 port,
//! h1-render-a, done in the p3-other-a continuation d): the dynamic mesh's
//! stream binding (its `_alloca`'d pointer array handed to a virtual
//! method), the segment grouping of the renderer's dynamic streams, the
//! texture format registration and `PParameterBuffer::VisitChildren`.

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

/// `PShaderParameterDefinition_findIn_004d5f30` (thiscall `(ctx, data)`,
/// `ret 4`): the parameter buffer's offset table, or 0.
pub const PARAMETER_TABLE: u32 = 0x004d_5f30;
/// `Phyre_PhyrePrintf` and its format `"Unknown parameter type and element
/// count for %s\n"`.
pub const PHYRE_PRINTF: u32 = 0x0043_52f0;
pub const UNKNOWN_PARAMETER: u32 = 0x00b1_fd70;
/// The parameter kinds (`+2`) visited as one object of a class: kind and
/// class descriptor.
pub const OBJECT_KINDS: [(u8, u32); 11] = [
    (0x42, 0x00c9_bef8),
    (0x43, 0x00c9_bf90),
    (0x44, 0x00c9_c028),
    (0x47, 0x00c9_c0c0),
    (0x48, 0x00c9_c158),
    (0x49, 0x00c9_c1f0),
    (0x4b, 0x00c9_c288),
    (0x4c, 0x00c9_c320),
    (0x4a, 0x00c9_be60),
    (0x4d, 0x00c9_c4e8),
    (0x4e, 0x00c9_c580),
];
/// The element types (`+3`, the original's jump table at `0x004d6fe0`):
/// the type getter and the element count, or none (reported unknown).
pub const ELEMENT_TYPES: [Option<(u32, u32)>; 51] = {
    const F32: u32 = 0x0043_8060;
    const U16: u32 = 0x0043_7e70;
    const U32: u32 = 0x0043_7fc0;
    const U8: u32 = 0x0043_7d30;
    const I32: u32 = 0x0043_7f10;
    const I16: u32 = 0x0043_7dd0;
    const CHAR: u32 = 0x0043_7c90;
    [
        Some((F32, 1)),
        Some((F32, 2)),
        Some((F32, 3)),
        Some((F32, 4)),
        Some((U16, 1)),
        Some((U16, 2)),
        Some((U16, 3)),
        Some((U16, 4)),
        Some((U32, 1)),
        Some((U32, 2)),
        Some((U32, 3)),
        Some((U32, 4)),
        Some((U16, 1)),
        Some((U16, 2)),
        Some((U16, 3)),
        Some((U16, 4)),
        Some((U8, 1)),
        Some((U8, 2)),
        Some((U8, 3)),
        Some((U8, 4)),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some((I32, 1)),
        Some((I32, 2)),
        Some((I32, 3)),
        Some((I32, 4)),
        Some((I16, 1)),
        Some((I16, 2)),
        Some((I16, 3)),
        Some((I16, 4)),
        Some((CHAR, 1)),
        None,
        None,
        None,
        Some((U16, 1)),
        Some((U16, 2)),
        Some((U16, 3)),
        Some((U16, 4)),
        None,
        None,
        None,
        None,
        Some((F32, 12)),
        Some((F32, 16)),
        Some((U32, 1)),
    ]
};

pub mod visit {
    //! `0x004d6c00`, over game memory.

    use super::{ELEMENT_TYPES, OBJECT_KINDS, PARAMETER_TABLE, PHYRE_PRINTF, UNKNOWN_PARAMETER};
    use ffx_graphic::host::{Conv, Host};

    /// `Phyre_PRendering_PParameterBuffer_VisitChildren` (cdecl `(buffer,
    /// visitor)`): unless the visitor's slot 3 says stop (its `al`, its
    /// `eax` returned), each of the buffer's 16-byte parameters (`+0xc`,
    /// count at `+8`) is handed to the visitor's slot 2 `(data, count,
    /// type, 0, data2)`: data at the parameter's offset `+8` (0 when it
    /// runs past the buffer's size or the offset is under 4; unchecked
    /// with an offset table, which gives data2), kinds 0x42..0x4e as one
    /// object of their class, else the element type's getter and count
    /// (times the array size `+0` when non-zero); an unknown one is
    /// reported through `PhyrePrintf` with its name. `eax`: the offset
    /// table when there are no parameters, else the parameters' bytes.
    pub fn visit_children(h: &mut impl Host, buffer: u32, visitor: u32) -> u32 {
        let stop = h.call_virtual(visitor, 3, &[]);
        if stop & 0xff != 0 {
            return stop;
        }
        let ctx: u32 = h.read(visitor + 4);
        let count = h.read::<u32>(buffer + 8) & 0x7fff_ffff;
        let data: u32 = h.read(buffer + 0xc);
        let table = {
            let f = h.rebase(PARAMETER_TABLE);
            h.call(f, Conv::Thiscall(ctx), &[data])
        };
        if count == 0 {
            return table;
        }
        let mut off = 0u32;
        for _ in 0..count {
            let entry = h.read::<u32>(buffer + 0xc).wrapping_add(off);
            let offset = u32::from(h.read::<u16>(entry + 8));
            let end = (u32::from(h.read::<u16>(entry + 0xa)) & 0x1fff) + offset;
            let size: u32 = h.read(buffer);
            let mut p = if end > size || offset < 4 {
                0
            } else {
                buffer.wrapping_add(offset)
            };
            let kind: u8 = h.read(entry + 2);
            let q = if table == 0 {
                p
            } else {
                let q = u32::from(h.read::<u16>(table.wrapping_add(8).wrapping_add(off)));
                p = buffer.wrapping_add(offset);
                q.wrapping_add(buffer)
            };
            if let Some(&(_, desc)) = OBJECT_KINDS.iter().find(|&&(k, _)| k == kind) {
                let desc = h.rebase(desc);
                h.call_virtual(visitor, 2, &[p, 1, desc, 0, q]);
            } else {
                let element: u8 = h.read(entry + 3);
                let (ty, n) = match ELEMENT_TYPES.get(usize::from(element)).copied().flatten() {
                    Some((getter, n)) => {
                        let f = h.rebase(getter);
                        (h.call(f, Conv::Cdecl, &[]), n)
                    }
                    None => (0, 0),
                };
                let array = u32::from(h.read::<u16>(entry));
                let total = if array == 0 { n } else { array.wrapping_mul(n) };
                if ty == 0 {
                    let w: u32 = h.read(entry + 4);
                    let name = w.wrapping_sub(w & 1);
                    let (f, fmt) = (h.rebase(PHYRE_PRINTF), h.rebase(UNKNOWN_PARAMETER));
                    h.call(f, Conv::Cdecl, &[1, fmt, name]);
                } else {
                    h.call_virtual(visitor, 2, &[p, total, ty, 0, q]);
                }
            }
            off = off.wrapping_add(0x10);
        }
        off
    }
}
