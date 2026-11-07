//! M5 stage 1, round 9 (`tail9`, list `phyre-media`):
//! `PTexture2DD3D11_mapLevel_00596b90`, the level mapping
//! `PVideoPlaybackWin32_MapTexture` uses: under the render scope lock, a
//! CPU texture's level buffer (made on first use) laid out mip by mip,
//! or each mip of a D3D11 texture mapped through the device context's
//! `Map`, into the caller's list of (data, row pitch) pairs.

/// The originals over game memory.
#[allow(clippy::many_single_char_names)] // the original's registers
pub mod bridge {
    use ffx_graphic::host::{Conv, Host};

    /// `Phyre_PSharray_element28_00585c30(array; i)` (thiscall, `ret 4`),
    /// `Phyre_PhyrePrintf(1, text)` and its text (`0x00b33ac0`),
    /// `PhyreMalloc(size, align)` (cdecl), `PRenderScopeLock_enter/leave`
    /// (thiscall), the render interface's guard (`0x00c9628c` bit 0), its
    /// constructor `Phyre_PRendering_Construct583580_004fee70` (thiscall
    /// on `0x00c94f00`), `atexit` and its handler (`0x00afb710`), and the
    /// device context it holds (`0x00c94f70`).
    pub const ELEMENT: u32 = 0x0058_5c30;
    pub const PRINTF: u32 = 0x0043_52f0;
    pub const NOT_CPU_TEXT: u32 = 0x00b3_3ac0;
    pub const MALLOC: u32 = 0x0042_fb80;
    pub const LOCK_ENTER: u32 = 0x0046_3d50;
    pub const LOCK_LEAVE: u32 = 0x0046_3e90;
    pub const GUARD: u32 = 0x00c9_628c;
    pub const INTERFACE: u32 = 0x00c9_4f00;
    pub const INTERFACE_CTOR: u32 = 0x004f_ee70;
    pub const ATEXIT: u32 = 0x0094_9207;
    pub const INTERFACE_DTOR: u32 = 0x00af_b710;
    pub const CONTEXT: u32 = 0x00c9_4f70;

    /// The bytes of the original's frame below `ebp` the callees see: the
    /// scope lock (`ebp - 0x2c`) and `Map`'s `D3D11_MAPPED_SUBRESOURCE`
    /// (`ebp - 0x38`, 3 words).
    pub const FRAME: u32 = 0x38;

    /// A mip's width or height for the layout: 0 takes the level's own,
    /// block-compressed formats round up to 4.
    fn extent(v: u32, base: u32, block: bool) -> u32 {
        let v = if v == 0 { base } else { v };
        if block { v.wrapping_add(3) & !3 } else { v }
    }

    /// The next mip's extent (halved, at least 1).
    fn half(v: u32) -> u32 {
        let v = v >> 1;
        if v <= 1 { 1 } else { v }
    }

    /// `PTexture2DD3D11_mapLevel_00596b90(texture; level, out)` (thiscall,
    /// `ret 8`) -> 0, 3 (no such level), 0x15 (the level has no resource)
    /// or 9 (a `Map` failed or gave no data). `out`: `+0` set to 0, then
    /// (data, row pitch) pairs from `+4`, one per mip (`+0xc` + 1 of them;
    /// one when flag 2 of `+0x14` is set). `ebp`: the frame ([`FRAME`]
    /// bytes below it).
    /// A texture without flag 0x10000 of `+0x68` (CPU) prints a note and
    /// lays its mips out in one buffer (the level element's `+0x10`, made
    /// with `PhyreMalloc` on first use), slices `bpp x w x h / 8` and rows
    /// `bpp x w x (4 for block formats, else 1) / 8`; with it each mip `i`
    /// of the element's resource is mapped (`Map(resource, i, 2 or 4 by
    /// flag 2 of +0x60, 0, &mapped)`, context slot 14).
    #[allow(clippy::too_many_lines)]
    pub fn map_level(h: &mut impl Host, ebp: u32, tex: u32, level: u32, out: u32) -> u32 {
        let at = |o: u32| tex.wrapping_add(o);
        let levels = at(0x24);
        let mut count = h.read::<u32>(levels) & 0x7fff_ffff;
        if count == 0 {
            count = 1;
        }
        if level >= count {
            return 3;
        }
        let e = h.call_sym(ELEMENT, Conv::Thiscall(levels), &[level]);
        if h.read::<u32>(e) == 0 {
            return 0x15;
        }
        let cpu = h.read::<u32>(at(0x68)) & 0x1_0000 == 0;
        if cpu {
            let text = h.rebase(NOT_CPU_TEXT);
            h.call_sym(PRINTF, Conv::Cdecl, &[1, text]);
        }
        let lock = ebp.wrapping_sub(0x2c);
        h.call_sym(LOCK_ENTER, Conv::Thiscall(lock), &[]);
        let guard = h.rebase(GUARD);
        let g: u32 = h.read(guard);
        if g & 1 == 0 {
            h.write(guard, g | 1);
            let iface = h.rebase(INTERFACE);
            h.call_sym(INTERFACE_CTOR, Conv::Thiscall(iface), &[]);
            let dtor = h.rebase(INTERFACE_DTOR);
            h.call_sym(ATEXIT, Conv::Cdecl, &[dtor]);
        }
        let ctx: u32 = h.read(h.rebase(CONTEXT));
        h.write(out, 0_u32);
        let mips = if (h.read::<u32>(at(0x14)) >> 1) & 1 != 0 {
            0
        } else {
            h.read::<u32>(at(0xc))
        };
        let map_type = if h.read::<u8>(at(0x60)) & 2 != 0 {
            4
        } else {
            2
        };
        let mut entry = out.wrapping_add(4);
        let result = if cpu {
            let e = h.call_sym(ELEMENT, Conv::Thiscall(levels), &[level]);
            if h.read::<u32>(e.wrapping_add(0x10)) == 0 {
                let (w0, h0): (u32, u32) = (h.read(at(0x1c)), h.read(at(0x20)));
                let fmt: u32 = h.read(tex);
                let block = h.read::<u8>(fmt.wrapping_add(0x10)) & 1 != 0;
                let (mut w, mut hh) = (w0, h0);
                let mut total = 0_u32;
                for _ in 0..mips.wrapping_add(1) {
                    let ew = extent(w, w0, block);
                    let eh = extent(hh, h0, block);
                    let bpp: u32 = h.read(fmt.wrapping_add(0xc));
                    total = total.wrapping_add(bpp.wrapping_mul(ew).wrapping_mul(eh) >> 3);
                    w = half(w);
                    hh = half(hh);
                }
                let buffer = h.call_sym(MALLOC, Conv::Cdecl, &[total, 1]);
                let e = h.call_sym(ELEMENT, Conv::Thiscall(levels), &[level]);
                h.write(e.wrapping_add(0x10), buffer);
            }
            let e = h.call_sym(ELEMENT, Conv::Thiscall(levels), &[level]);
            let mut data: u32 = h.read(e.wrapping_add(0x10));
            let (mut w, mut hh): (u32, u32) = (h.read(at(0x1c)), h.read(at(0x20)));
            for _ in 0..mips.wrapping_add(1) {
                let (w0, h0): (u32, u32) = (h.read(at(0x1c)), h.read(at(0x20)));
                let fmt: u32 = h.read(tex);
                let block = h.read::<u8>(fmt.wrapping_add(0x10)) & 1 != 0;
                let ew = extent(w, w0, block);
                let eh = extent(hh, h0, block);
                let bpp: u32 = h.read(fmt.wrapping_add(0xc));
                let slice = bpp.wrapping_mul(ew).wrapping_mul(eh) >> 3;
                h.write(entry, data);
                let rows = if block { 4 } else { 1 };
                let pitch = bpp.wrapping_mul(ew).wrapping_mul(rows) >> 3;
                data = data.wrapping_add(slice);
                h.write(entry.wrapping_add(4), pitch);
                entry = entry.wrapping_add(8);
                w = half(w);
                hh = half(hh);
            }
            0
        } else {
            let mapped = ebp.wrapping_sub(0x38);
            let mut i = 0_u32;
            loop {
                for o in [0, 4, 8] {
                    h.write(mapped.wrapping_add(o), 0_u32);
                }
                let n = h.read::<u32>(levels) & 0x7fff_ffff;
                let base = if n > 1 {
                    h.read(levels.wrapping_add(4))
                } else if n != 0 {
                    levels.wrapping_add(4)
                } else {
                    0
                };
                let resource: u32 = h.read(base.wrapping_add(level.wrapping_mul(0x1c)));
                let vt: u32 = h.read(ctx);
                let map: u32 = h.read(vt.wrapping_add(0x38));
                let r = h.call(map, Conv::Stdcall, &[ctx, resource, i, map_type, 0, mapped]);
                let data: u32 = h.read(mapped);
                if r.cast_signed() < 0 || data == 0 {
                    break 9;
                }
                h.write(entry, data);
                let pitch: u32 = h.read(mapped.wrapping_add(4));
                h.write(entry.wrapping_add(4), pitch);
                i += 1;
                entry = entry.wrapping_add(8);
                if i > mips {
                    break 0;
                }
            }
        };
        h.call_sym(LOCK_LEAVE, Conv::Thiscall(lock), &[]);
        result
    }
}
