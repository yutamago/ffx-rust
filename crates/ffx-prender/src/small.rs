//! Small render methods of h2-render-b round 2: the D3D11 renderer's
//! state setters (scissor rectangles, shader resources), texture binds,
//! the modifier network's forwarders, element lookups.
//!
//! Several pass the addresses of their own locals to callees: their
//! bridges take that frame (`frame`, the original's `ebp - size`, laid out
//! at the original's offsets), which the shims allocate zeroed
//! (`StackFrame`) and the tests place at the original's address.
//!
//! COM calls (`ID3D11DeviceContext`, `ID3D11Resource`, ...) go through the
//! object's vtable, `stdcall` with the object first; `ecx` is what the
//! original leaves there (the vtable or the object), which the callee
//! never reads.

use ffx_bridge::x87;
use ffx_graphic::host::{Conv, Host};

use crate::array::COUNT;

/// The storage of the `PSharray` at `array`: the data word for more than
/// one element, the inline word for one, null for none.
fn storage(h: &mut impl Host, array: u32) -> u32 {
    match h.read::<u32>(array) & COUNT {
        0 => 0,
        1 => array + 4,
        _ => h.read(array + 4),
    }
}

/// A COM method: `slot` (word index) of `obj`'s vtable, `stdcall` with
/// `obj` first, `ecx` as the original leaves it.
fn com(h: &mut impl Host, ecx: u32, obj: u32, slot: u32, args: &[u32]) -> u32 {
    let vt: u32 = h.read(obj);
    let f: u32 = h.read(vt + 4 * slot);
    let mut words = Vec::with_capacity(args.len() + 1);
    words.push(obj);
    words.extend_from_slice(args);
    h.call(f, Conv::Thiscall(ecx), &words) // call width: 5 (four arguments at most)
}

pub mod bridge {
    //! The originals over game memory.

    use super::{COUNT, Conv, Host, com, storage, x87};

    /// `PModifierNetworkDynamicMeshInstance` vslot 1 `(this; ctx)`: a
    /// 0x10-byte frame `(ctx, this, [this+0x10][ctx.index], 0)` handed to
    /// `0x005ead0(network; ctx.index, &frame)` (thiscall on
    /// `[this + 4]`); returns its `eax`.
    pub fn network_forward1(h: &mut impl Host, this: u32, ctx: u32, frame: u32) -> u32 {
        let index: u32 = h.read(ctx + 0x20);
        let table: u32 = h.read(this + 0x10);
        let entry: u32 = h.read(table.wrapping_add(index.wrapping_mul(4)));
        h.write(frame, ctx);
        h.write(frame + 4, this);
        h.write(frame + 8, entry);
        h.write(frame + 0xc, 0u32);
        let network: u32 = h.read(this + 4);
        h.call_sym(0x005a_ead0, Conv::Thiscall(network), &[index, frame])
    }

    /// vslot 2 `(this; a, ctx)`: as [`network_forward1`], to
    /// `0x005aece0(network; ctx.index, a, &frame)`.
    pub fn network_forward2(h: &mut impl Host, this: u32, a: u32, ctx: u32, frame: u32) -> u32 {
        let index: u32 = h.read(ctx + 0x20);
        let table: u32 = h.read(this + 0x10);
        let entry: u32 = h.read(table.wrapping_add(index.wrapping_mul(4)));
        h.write(frame, ctx);
        h.write(frame + 4, this);
        h.write(frame + 8, entry);
        h.write(frame + 0xc, 0u32);
        let network: u32 = h.read(this + 4);
        h.call_sym(0x005a_ece0, Conv::Thiscall(network), &[index, a, frame])
    }

    /// `0x0058cf20(this)`: when the first word of the `PSharray` at
    /// `+0xc` is 0, `0x0058ee20(this; [this], 0)`; then `+0x38 = 1`;
    /// returns that call's `eax`, else 0. (An empty array reads address
    /// 0, as the original does.)
    pub fn bind_if_unset(h: &mut impl Host, this: u32) -> u32 {
        let s = storage(h, this + 0xc);
        let mut r = 0;
        if h.read::<u32>(s) == 0 {
            let first: u32 = h.read(this);
            r = h.call_sym(0x0058_ee20, Conv::Thiscall(this), &[first, 0]);
        }
        h.write(this + 0x38, 1u32);
        r
    }

    /// `PTexture2DD3D11::bind(this)`: when the first word of the
    /// `PSharray` at `+0x24` is 0, the device texture is created
    /// (`0x0058fc20(this; 0, 1, bit 10 of +0x14, 0)`); `+0x70 = 1`;
    /// returns 0.
    pub fn texture_bind(h: &mut impl Host, this: u32) -> u32 {
        let s = storage(h, this + 0x24);
        if h.read::<u32>(s) == 0 {
            let bit = (h.read::<u32>(this + 0x14) >> 10) & 1;
            h.call_sym(0x0058_fc20, Conv::Thiscall(this), &[0, 1, bit, 0]);
        }
        h.write(this + 0x70, 1u32);
        0
    }

    /// `0x00592b60(this; primitive, start, count)`: no vertices give
    /// `al = 1` (the rest of `eax` is the caller's). Else the primitive's
    /// D3D11 topology (`0x0058ab60`, cdecl): none gives `eax = 0`; then on
    /// the context at `+0x70` `IASetPrimitiveTopology(topology)` (method
    /// 24) and `Draw(count, start)` (method 13); `al = 1`.
    pub fn draw(h: &mut impl Host, this: u32, primitive: u32, start: u32, count: u32) -> u8 {
        if count == 0 {
            return 1;
        }
        let topology = h.call_sym(0x0058_ab60, Conv::Cdecl, &[primitive]);
        if topology == 0 {
            return 0;
        }
        let ctx: u32 = h.read(this + 0x70);
        let vt: u32 = h.read(ctx);
        com(h, vt, ctx, 24, &[topology]);
        let ctx: u32 = h.read(this + 0x70);
        let vt: u32 = h.read(ctx);
        com(h, vt, ctx, 13, &[count, start]);
        1
    }

    /// `0x005008c0(this; a, b, c)`: the vectors at `this` and `this + 0x10`
    /// (three floats each, copied through x87) into the frame at `+0` and
    /// `+0x10`, then `0x004ffe40(&first, &second, a, b, c)` (cdecl);
    /// returns its `eax`. The frame's fourth words are never written
    /// (0 in the shim's frame).
    pub fn bounds_forward(h: &mut impl Host, this: u32, a: u32, b: u32, c: u32, frame: u32) -> u32 {
        for (src, dst) in [(this + 0x10, frame + 0x10), (this, frame)] {
            for i in 0..3 {
                let v: u32 = h.read(src + 4 * i);
                h.write(dst + 4 * i, x87::store(x87::load(v)));
            }
        }
        h.call_sym(0x004f_fe40, Conv::Cdecl, &[frame, frame + 0x10, a, b, c])
    }

    /// `0x00593e30(this; i)`: for `i` below the count of the `PSharray`
    /// at `+0x20`, word 1 of its 0x14-byte element `i`; else 0.
    pub fn element_word1(h: &mut impl Host, this: u32, i: u32) -> u32 {
        let n = h.read::<u32>(this + 0x20) & COUNT;
        if i >= n {
            return 0;
        }
        let s = storage(h, this + 0x20);
        h.read(s.wrapping_add(i.wrapping_mul(0x14)) + 4)
    }

    /// `0x0059aaf0(this; x, y, w, h)`: one scissor rectangle `(x, y, x +
    /// w, y + h)` in the frame (its cookie word above), set on the
    /// context at `+0x70` (method 45, `RSSetScissorRects(1, &rect)`);
    /// returns its `eax` (the cookie check keeps it).
    pub fn set_scissor(h: &mut impl Host, this: u32, rect: [u32; 4], frame: u32) -> u32 {
        let [x, y, w, hh] = rect;
        h.write(frame, x);
        h.write(frame + 4, y);
        h.write(frame + 8, w.wrapping_add(x));
        h.write(frame + 0xc, hh.wrapping_add(y));
        let ctx: u32 = h.read(this + 0x70);
        let vt: u32 = h.read(ctx);
        com(h, vt, ctx, 45, &[1, frame])
    }

    /// `0x00598a80(holder, shader, slot)` (stdcall): for each of the four
    /// tables at `+0x10..+0x1c` that is set, `0x00598b50(table; shader,
    /// slot)`; returns 0.
    pub fn unbind_all(h: &mut impl Host, holder: u32, shader: u32, slot: u32) -> u32 {
        for off in [0x10, 0x14, 0x18, 0x1c] {
            let t: u32 = h.read(holder + off);
            if t != 0 {
                h.call_sym(0x0059_8b50, Conv::Thiscall(t), &[shader, slot]);
            }
        }
        0
    }

    /// `0x00598b50(this; _, slot)`: the 0x80 resource words at `+0x1c`
    /// and the 0x10 at `+0x21c` whose owner word (0x280 bytes above) is
    /// `slot` are cleared (the others are rewritten as they are); returns
    /// `al = 1` over the last address the original computed.
    pub fn unbind_table(h: &mut impl Host, this: u32, slot: u32) -> u32 {
        for (base, n) in [(this + 0x1c, 0x80), (this + 0x21c, 0x10)] {
            for i in 0..n {
                let p = base + 4 * i;
                let v = if h.read::<u32>(p + 0x280) == slot {
                    0
                } else {
                    h.read::<u32>(p)
                };
                h.write(p, v);
            }
        }
        ((this + 0x25c) & !0xff) | 1
    }

    /// `0x0058ab60(primitive)`: the D3D11 primitive topology of the
    /// engine's primitive types 0..5 (points 1, lines 2, triangles 4,
    /// strips 5, fans as strips after a one-time `FAN_NOT_SUPPORTED`
    /// message, 1), 0 for the others.
    pub fn topology(h: &mut impl Host, primitive: u32) -> u32 {
        match primitive {
            0 | 5 => 1,
            1 => 2,
            2 => 4,
            3 => 5,
            4 => {
                let once = h.rebase(0x00cb_4bcc);
                if h.read::<u8>(once) == 0 {
                    h.write(once, 1u8);
                    let s = h.rebase(0x00b3_3bf8);
                    h.call_sym(0x0043_52f0, Conv::Cdecl, &[1, s]);
                }
                5
            }
            _ => 0,
        }
    }

    /// `0x0058a8f0(factor)`: the engine's blend factor as a `D3D11_BLEND`
    /// (the table at `0x0058a960`; 2, `D3D11_BLEND_ONE`, outside 0..14).
    #[must_use]
    pub fn blend_factor(factor: u32) -> u32 {
        const TABLE: [u32; 15] = [1, 2, 3, 4, 9, 0xa, 5, 6, 7, 8, 0xe, 0xf, 2, 2, 0xb];
        TABLE.get(factor as usize).copied().unwrap_or(2)
    }

    /// `0x005998f0(this; mode, a, b)`: `+0x5d4 = 0`, `+0x5d8 = mode`
    /// (its low byte), `+0x5dc`/`+0x5e0` the two values mapped by
    /// `0x0058a8f0` (cdecl), `+0x5e8 = 5`, `+0x5ec = 6`; returns 0.
    pub fn set_blend_pair(h: &mut impl Host, this: u32, mode: u32, a: u32, b: u32) -> u32 {
        h.write(this + 0x5d4, 0u32);
        h.write(this + 0x5d8, mode & 0xff);
        let x = h.call_sym(0x0058_a8f0, Conv::Cdecl, &[a]);
        h.write(this + 0x5dc, x);
        let y = h.call_sym(0x0058_a8f0, Conv::Cdecl, &[b]);
        h.write(this + 0x5e0, y);
        h.write(this + 0x5e8, 5u32);
        h.write(this + 0x5ec, 6u32);
        0
    }

    /// `0x005940d0(this; count, modes)`: the DXGI factory at `+0x133c`
    /// (when set): `EnumAdapters(0, &adapter)` (method 7), the adapter's
    /// `EnumOutputs(0, &output)` (method 7), the output's
    /// `GetDisplayModeList(R8G8B8A8_UNORM, 0, count, modes)` (method 8);
    /// returns 0. Nothing is released, as in the original. The two
    /// out-pointers are the frame's words (`output` at `+0`, `adapter` at
    /// `+4`), zeroed first.
    pub fn query_and_set(h: &mut impl Host, this: u32, a: u32, b: u32, frame: u32) -> u32 {
        let obj: u32 = h.read(this + 0x133c);
        h.write(frame + 4, 0u32);
        h.write(frame, 0u32);
        if obj == 0 {
            return 0;
        }
        com(h, obj, obj, 7, &[0, frame + 4]);
        let first: u32 = h.read(frame + 4);
        if first != 0 {
            com(h, first, first, 7, &[0, frame]);
        }
        let second: u32 = h.read(frame);
        if second != 0 {
            com(h, second, second, 8, &[0x1c, 0, a, b]);
        }
        0
    }

    /// `0x00563900(this; i, out)`: element `i` (0x14 bytes) of the
    /// `PSharray` at `+0x14` gives a key; `0x00560e90(&offset, key, 2)`
    /// (cdecl, `offset` the frame's word, zeroed first); on 0, `out =
    /// ([this+0x2c] + offset, [this+0x34])`. Returns that call's `eax`.
    pub fn element_offset(h: &mut impl Host, this: u32, i: u32, out: u32, frame: u32) -> u32 {
        h.write(frame, 0u32);
        let s = storage(h, this + 0x14);
        let key: u32 = h.read(s.wrapping_add(i.wrapping_mul(0x14)));
        let r = h.call_sym(0x0056_0e90, Conv::Cdecl, &[frame, key, 2]);
        if r != 0 {
            return r;
        }
        let base: u32 = h.read(this + 0x2c);
        let offset: u32 = h.read(frame);
        h.write(out, base.wrapping_add(offset));
        let v: u32 = h.read(this + 0x34);
        h.write(out + 4, v);
        0
    }
}
