//! List `p5-core`, continuation round: two of Iggy's `GDraw` D3D11 helpers
//! the game links (`0x00a34bc0` unbinds the vertex input and frees a
//! texture's views, `0x00a35670` makes a blend state). D3D11 calls stay
//! calls through the objects' vtables (stdcall, `this` first).

use ffx_graphic::host::{Conv, Host};

/// The `GDraw` state pointer (its `+4` is the immediate context).
pub const STATE: u32 = 0x01a8_58b4;
/// The null shader-resource views `PSSetShaderResources` binds (three).
pub const NULL_VIEWS: u32 = 0x01a8_58b8;
/// The CRT `memset` thunk (cdecl).
pub const MEMSET: u32 = 0x0094_964c;
/// `GDraw`'s warning callback (an import slot, cdecl `(0, format, what,
/// suffix[, hr])`) and its strings.
pub const WARN_SLOT: u32 = 0x00b0_c874;
pub const OUT_OF_MEMORY: u32 = 0x00b7_a4ec;
pub const ERROR_FORMAT: u32 = 0x00b7_a50c;
pub const CREATE_BLEND_STATE: u32 = 0x00b7_a414;
pub const EMPTY: u32 = 0x00b8_fbeb;
/// `E_OUTOFMEMORY`.
pub const E_OUTOFMEMORY: u32 = 0x8007_000e;

pub mod bridge {
    //! The originals over game memory.

    use super::{
        CREATE_BLEND_STATE, Conv, E_OUTOFMEMORY, EMPTY, ERROR_FORMAT, Host, MEMSET, NULL_VIEWS,
        OUT_OF_MEMORY, STATE, WARN_SLOT,
    };

    /// A COM method: slot `off` of `obj`'s vtable, `obj` first (stdcall).
    fn com(h: &mut impl Host, obj: u32, off: u32, args: &[u32]) -> u32 {
        let vt: u32 = h.read(obj);
        let f: u32 = h.read(vt.wrapping_add(off));
        let mut words = [0u32; 6];
        words[0] = obj;
        words[1..=args.len()].copy_from_slice(args);
        h.call(f, Conv::Stdcall, &words[..=args.len()])
    }

    /// `Release` of the object at `at` when there is one, the word then
    /// 0. `eax` as the original leaves it: `Release`'s, else `eax`.
    fn release(h: &mut impl Host, at: u32, eax: u32) -> u32 {
        let obj: u32 = h.read(at);
        if obj == 0 {
            return eax;
        }
        let r = com(h, obj, 8, &[]);
        h.write(at, 0u32);
        r
    }

    /// `0x00a34bc0(texture)` (cdecl): on the immediate context, three
    /// null pixel-shader views (slot 0), one null vertex buffer (stride
    /// and offset 0, `frame`'s two words), no index buffer; then, unless
    /// the texture's `+0x30` type nibble is 5, its views released: `+0`
    /// then `+4` when bit 0 of `[[+0x18] + 0xc]` is set, else `+4`, `+8`
    /// and `+0`. `eax` as the original leaves it.
    pub fn unbind(h: &mut impl Host, texture: u32, frame: u32) -> u32 {
        let state: u32 = h.read(h.rebase(STATE));
        let ctx: u32 = h.read(state + 4);
        h.write(frame, 0u32);
        h.write(frame + 4, 0u32);
        let views = h.rebase(NULL_VIEWS);
        com(h, ctx, 0x20, &[0, 3, views]);
        com(h, ctx, 0x48, &[0, 1, frame, frame + 4, frame + 4]);
        com(h, ctx, 0x4c, &[0, 0, 0]);
        let kind = h.read::<u32>(texture + 0x30) & 0xf000_0000;
        if kind == 0x5000_0000 {
            return kind;
        }
        let info: u32 = h.read(texture + 0x18);
        let mut eax = info;
        if h.read::<u8>(info.wrapping_add(0xc)) & 1 == 0 {
            for off in [4, 8, 0] {
                eax = release(h, texture + off, eax);
            }
        } else {
            for off in [0, 4] {
                eax = release(h, texture + off, eax);
            }
        }
        eax
    }

    /// `0x00a35670(device, a, b, c)` (cdecl): a blend-state description
    /// (0x108 bytes at `frame + 4`, zeroed by `memset`: `+8` `a`, `+0xc`
    /// `b`, `+0x10` `c`, `+0x14` 1, `+0x18` `b` (9 becomes 7), `+0x1c`
    /// `c`, `+0x20` 1, the byte `+0x24` 0xf) passed to the device's slot
    /// `+0x50` with `frame` for the result: the state, or 0 after `GDraw`'s
    /// warning (out of memory, or the error code).
    pub fn create_blend_state(
        h: &mut impl Host,
        dev: u32,
        a: u32,
        b: u32,
        c: u32,
        frame: u32,
    ) -> u32 {
        let desc = frame + 4;
        h.call_sym(MEMSET, Conv::Cdecl, &[desc, 0, 0x108]);
        h.write(desc + 8, a);
        h.write(desc + 0xc, b);
        h.write(desc + 0x10, c);
        h.write(desc + 0x14, 1u32);
        h.write(desc + 0x18, if b == 9 { 7 } else { b });
        h.write(desc + 0x1c, c);
        h.write(desc + 0x20, 1u32);
        h.write(desc + 0x24, 0xfu8);
        let hr = com(h, dev, 0x50, &[desc, frame]);
        if hr & 0x8000_0000 == 0 {
            return h.read(frame);
        }
        let warn: u32 = h.read(h.rebase(WARN_SLOT));
        let what = h.rebase(CREATE_BLEND_STATE);
        let empty = h.rebase(EMPTY);
        if hr == E_OUTOFMEMORY {
            let f = h.rebase(OUT_OF_MEMORY);
            h.call(warn, Conv::Cdecl, &[0, f, what, empty]);
        } else {
            let f = h.rebase(ERROR_FORMAT);
            h.call(warn, Conv::Cdecl, &[0, f, what, empty, hr]);
        }
        0
    }
}
