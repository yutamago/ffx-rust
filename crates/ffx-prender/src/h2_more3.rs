//! h2-render-b round 3, batch 3: destructors that free tagged words and
//! owned arrays, the camera-controller-like component's constructor and
//! destructor, the instance list's `freeObject`, the indexed draw, the
//! texture registry insert under its critical section.

use ffx_graphic::host::{Conv, Host};

use crate::array::{BORROWED, COUNT};
use crate::descriptor::PHYRE_FREE;

/// Frees the word at `p` unless its bit 0 is set (a tagged pointer: set
/// means not owned), with no null check.
fn free_tagged(h: &mut impl Host, p: u32) {
    let w: u32 = h.read(p);
    if w & 1 == 0 {
        h.call_sym(PHYRE_FREE, Conv::Cdecl, &[w]);
    }
}

/// Frees an array's storage (`count` at `at`, data at `at + 4`) when it
/// is owned (with `null_check`, also only when not null), then clears
/// both words (data first).
fn free_array(h: &mut impl Host, at: u32, null_check: bool) {
    if h.read::<u32>(at) & BORROWED == 0 {
        let data: u32 = h.read(at + 4);
        if !null_check || data != 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
        }
    }
    h.write(at + 4, 0u32);
    h.write(at, 0u32);
}

pub mod bridge {
    //! The originals over game memory.

    use super::{COUNT, Conv, Host, free_array, free_tagged};

    /// `0x004dbaf0(this)`: the tagged words at `+0xc`, `+8`, `+4`, `+0`
    /// freed unless their bit 0 is set.
    pub fn free_tagged4(h: &mut impl Host, this: u32) {
        for off in [0xc, 8, 4, 0] {
            free_tagged(h, this + off);
        }
    }

    /// `0x00567620(this)`: the tagged word at `+0x14`, the owned array at
    /// `+0xc` (data `+0x10`, when not null), the tagged word at `+0`.
    pub fn free_567620(h: &mut impl Host, this: u32) {
        free_tagged(h, this + 0x14);
        free_array(h, this + 0xc, true);
        free_tagged(h, this);
    }

    /// `0x00585180(this)`: `0x0059c650(this)`, the owned arrays at
    /// `+0x20` and `+0x18` freed (no null check) and cleared, then the
    /// tail call `0x004c79f0(this)`; returns its `eax`.
    pub fn dtor_585180(h: &mut impl Host, this: u32) -> u32 {
        h.call_sym(0x0059_c650, Conv::Thiscall(this), &[]);
        free_array(h, this + 0x20, false);
        free_array(h, this + 0x18, false);
        h.call_sym(0x004c_79f0, Conv::Thiscall(this), &[])
    }

    /// `0x005572e0(this)`: the base `0x00553980(this; 0x00caff10)`, vtable
    /// `0x00b2db30`, `+0x30 = 0`, `0x005d6cc0(+0x34)`, the half-word
    /// `+0xb0` and words `+0xb4`, `+0xb8` zero, `0x005dff30(+0xbc)`,
    /// `+0x2bc = 0`; returns `this`.
    pub fn component_ctor(h: &mut impl Host, this: u32) -> u32 {
        let d = h.rebase(0x00ca_ff10);
        h.call_sym(0x0055_3980, Conv::Thiscall(this), &[d]);
        let vt = h.rebase(0x00b2_db30);
        h.write(this, vt);
        h.write(this + 0x30, 0u32);
        h.call_sym(0x005d_6cc0, Conv::Thiscall(this + 0x34), &[]);
        h.write(this + 0xb0, 0u16);
        h.write(this + 0xb4, 0u32);
        h.write(this + 0xb8, 0u32);
        h.call_sym(0x005d_ff30, Conv::Thiscall(this + 0xbc), &[]);
        h.write(this + 0x2bc, 0u32);
        this
    }

    /// `0x00557410(this)`: vtable `0x00b2db30`; when the object at
    /// `+0x2bc` is set and `+0xac` points at its `+4`, `+0xa8` moves to
    /// its `+0xb4` and `+0xa8`/`+0xac` are cleared; `0x005e20c0(+0xbc)`,
    /// `0x005d6d10(+0x34)`, then the base destructor `0x00553a70(this)`;
    /// returns its `eax`.
    pub fn component_dtor(h: &mut impl Host, this: u32) -> u32 {
        let vt = h.rebase(0x00b2_db30);
        h.write(this, vt);
        let c: u32 = h.read(this + 0x2bc);
        if c != 0 && h.read::<u32>(this + 0xac) == c.wrapping_add(4) {
            let w: u32 = h.read(this + 0xa8);
            h.write(c + 0xb4, w);
            h.write(this + 0xa8, 0u32);
            h.write(this + 0xac, 0u32);
        }
        h.call_sym(0x005e_20c0, Conv::Thiscall(this + 0xbc), &[]);
        h.call_sym(0x005d_6d10, Conv::Thiscall(this + 0x34), &[]);
        h.call_sym(0x0055_3a70, Conv::Thiscall(this), &[])
    }

    /// `0x004fee80(this)`: byte `+0 = 1`, `+4 = 0`, four 0x10-byte
    /// elements at `+8` built by `eh_vector_constructor_iterator` (ctor
    /// `0x005b3620`, dtor `0x005b3640`), a fifth at `+0x48`
    /// (`0x005b3620`), the half-word `+0x58` zero, `+0x5c..+0x68` zero
    /// and then `+0x64 = 0x100000`, `+0x68 = 0x40000`; returns `this`.
    pub fn ctor_4fee80(h: &mut impl Host, this: u32) -> u32 {
        let items = this + 8;
        h.write(this, 1u8);
        h.write(this + 4, 0u32);
        let ctor = h.rebase(0x005b_3620);
        let dtor = h.rebase(0x005b_3640);
        h.call_sym(0x0094_966a, Conv::Stdcall, &[items, 0x10, 4, ctor, dtor]);
        h.call_sym(0x005b_3620, Conv::Thiscall(items + 0x40), &[]);
        h.write(items + 0x50, 0u16);
        for off in [0x60, 0x64, 0x68, 0x5c, 0x60] {
            h.write(this + off, 0u32);
        }
        h.write(this + 0x64, 0x0010_0000u32);
        h.write(this + 0x68, 0x0004_0000u32);
        this
    }

    /// `0x0058cf80(this)`: under the critical section `0x00cb3c94`
    /// (initialized first when its first byte is 0; entered by
    /// `0x00431840`, left by `0x004319d0`), `this` is added to the tree
    /// `0x00c25374` (`0x00595ee0(tree; &key)`, the key a local at
    /// `frame + 4`, the section's address at `frame`); then, when `+0x20`
    /// is 0, `0x00595bf0(this)`: its non-zero result is returned, else
    /// `+0x1c |= 0x80` when `+0x20` became set. Returns 0.
    pub fn register_texture(h: &mut impl Host, this: u32, frame: u32) -> u32 {
        let cs = h.rebase(0x00cb_3c94);
        if h.read::<u8>(cs) == 0 {
            h.call_sym(0x0043_17f0, Conv::Thiscall(cs), &[]);
        }
        h.write(frame, cs);
        h.call_sym(0x0043_1840, Conv::Thiscall(cs), &[]);
        h.write(frame + 4, this);
        let tree = h.rebase(0x00c2_5374);
        h.call_sym(0x0059_5ee0, Conv::Thiscall(tree), &[frame + 4]);
        h.call_sym(0x0043_19d0, Conv::Thiscall(cs), &[]);
        if h.read::<u32>(this + 0x20) == 0 {
            let r = h.call_sym(0x0059_5bf0, Conv::Thiscall(this), &[]);
            if r != 0 {
                return r;
            }
            if h.read::<u32>(this + 0x20) != 0 {
                let f: u32 = h.read(this + 0x1c);
                h.write(this + 0x1c, f | 0x80);
            }
        }
        0
    }

    /// `PRendererD3D11::drawIndexed(this; primitive, count, indices, i)`:
    /// `al = 0` when the primitive has no topology (`0x0058ab60`); else
    /// `IASetPrimitiveTopology`, `IASetIndexBuffer` (the buffer of
    /// element `i` of the index object's views when bit 0 of `+0x30` is
    /// set, else none; `R32_UINT` when the index size `+0xc` is 8, else
    /// `R16_UINT`; offset `+0x2c`) and `DrawIndexed(count, 0, 0)` on
    /// the context `+0x70`; `al = 1`.
    pub fn draw_indexed(
        h: &mut impl Host,
        this: u32,
        prim: u32,
        count: u32,
        ix: u32,
        i: u32,
    ) -> u8 {
        let topo = h.call_sym(0x0058_ab60, Conv::Cdecl, &[prim]);
        if topo == 0 {
            return 0;
        }
        let ctx: u32 = h.read(this + 0x70);
        let vt: u32 = h.read(ctx);
        let f: u32 = h.read(vt + 0x60);
        h.call(f, Conv::Thiscall(vt), &[ctx, topo]);
        let ctx: u32 = h.read(this + 0x70);
        let offset: u32 = h.read(ix + 0x2c);
        let format: u32 = if h.read::<u8>(ix + 0xc) == 8 {
            0x2a
        } else {
            0x39
        };
        let buffer: u32 = if h.read::<u8>(ix + 0x30) & 1 == 0 {
            0
        } else {
            let n = h.read::<u32>(ix + 0x14) & COUNT;
            let base = match n {
                0 => 0,
                1 => ix + 0x18,
                _ => h.read(ix + 0x18),
            };
            h.read(base.wrapping_add(i.wrapping_mul(20)))
        };
        let vt: u32 = h.read(ctx);
        let f: u32 = h.read(vt + 0x4c);
        h.call(f, Conv::Thiscall(format), &[ctx, buffer, format, offset]);
        let ctx: u32 = h.read(this + 0x70);
        let vt: u32 = h.read(ctx);
        let f: u32 = h.read(vt + 0x30);
        h.call(f, Conv::Thiscall(vt), &[ctx, count, 0, 0]);
        1
    }

    /// `PInstanceList::freeObject(this; obj)`: 1 for null; 5 when the
    /// object's slot (`obj - type +0x8c`) lies outside the buffer (`+0x34`,
    /// `+0x3c` slots of the type's stride `+0x1c`), 0x14 off the stride,
    /// 0x12 when already free (the sorted free chain at `+8`); else the
    /// type's virtual method 11 on it (when `+0x49`),
    /// `PFreeList::addItemToFreeList(+8; slot, prev)`, `+0x40` counted
    /// down, and 0.
    pub fn free_object(h: &mut impl Host, this: u32, obj: u32) -> u32 {
        if obj == 0 {
            return 1;
        }
        let ty: u32 = h.read(this + 0x2c);
        let buf: u32 = h.read(this + 0x34);
        let stride: u32 = h.read(ty + 0x1c);
        let slot = obj.wrapping_sub(h.read(ty + 0x8c));
        if slot < buf {
            return 5;
        }
        let end = h
            .read::<u32>(this + 0x3c)
            .wrapping_mul(stride)
            .wrapping_add(buf);
        if slot > end {
            return 5;
        }
        // the original's `div` faults on a zero stride; the port says 0x14
        if (slot - buf).checked_rem(stride).unwrap_or(1) != 0 {
            return 0x14;
        }
        let mut prev = this + 8;
        let mut x: u32 = h.read(prev);
        while x != 0 {
            if x == slot {
                return 0x12;
            }
            if x > slot {
                break;
            }
            prev = x;
            x = h.read(x);
        }
        if h.read::<u8>(this + 0x49) != 0 {
            let ty: u32 = h.read(this + 0x2c);
            let vt: u32 = h.read(ty);
            let f: u32 = h.read(vt + 0x2c);
            h.call(f, Conv::Thiscall(ty), &[obj]);
        }
        h.call_sym(0x0049_c7a0, Conv::Thiscall(this + 8), &[slot, prev]);
        let n: u32 = h.read(this + 0x40);
        h.write(this + 0x40, n.wrapping_sub(1));
        0
    }

    /// `0x0055dc40(this)`: zero words `+0..+0x10`, half-word `+0x14`, an
    /// empty block at `+0x1c` (5 words) with `+0x18 = 1` (unless its
    /// address is null), zero `+0x30..+0x3c`; returns `this`.
    pub fn ctor_55dc40(h: &mut impl Host, this: u32) -> u32 {
        for off in (0..=0x10).step_by(4) {
            h.write(this + off, 0u32);
        }
        h.write(this + 0x14, 0u16);
        h.write(this + 0x18, 0u32);
        let b = this.wrapping_add(0x1c);
        if b != 0 {
            for i in 0..5 {
                h.write(b + 4 * i, 0u32);
            }
            h.write(this + 0x18, 1u32);
        }
        for off in [0x30, 0x34, 0x38, 0x3c] {
            h.write(this + off, 0u32);
        }
        this
    }
}
