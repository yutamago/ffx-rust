//! h2-render-b round 3, batch 2: constructors and destructors of the
//! D3D11 render objects (buffers, textures, shader programs), the
//! instance list's buffer free, vector min/max, viewport and render
//! target colour setters, `PDynamicMeshInstance::Bind`.

use ffx_graphic::host::{Conv, Host};

use crate::array::{BORROWED, COUNT};
use crate::descriptor::PHYRE_FREE;
use crate::h2_more::MEMSET;

/// Releases the COM object in `slot` (method 2, `ecx` the object) and
/// clears the slot, when it is set.
fn release(h: &mut impl Host, slot: u32) {
    let obj: u32 = h.read(slot);
    if obj != 0 {
        let vt: u32 = h.read(obj);
        let f: u32 = h.read(vt + 8);
        h.call(f, Conv::Thiscall(obj), &[obj]);
        h.write(slot, 0u32);
    }
}

/// Frees the storage of the `PSharray` at `this + off` when it is owned
/// and not null (and, with `more`, holds more than one element), then
/// clears it.
fn free_sharray(h: &mut impl Host, this: u32, off: u32, more: bool) {
    let n: u32 = h.read(this + off);
    if n & BORROWED == 0 && (!more || n & COUNT > 1) {
        let data: u32 = h.read(this + off + 4);
        if data != 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
        }
    }
    h.write(this + off + 4, 0u32);
    h.write(this + off, 0u32);
}

pub mod bridge {
    //! The originals over game memory.

    use super::{BORROWED, COUNT, Conv, Host, MEMSET, PHYRE_FREE, free_sharray, release};
    use ffx_bridge::x87;

    /// The pattern of several render-object constructors: the base
    /// constructor `base(this)` (thiscall), `+flag = 0`, an empty
    /// `words`-word block at `+block` with `+flag = 1` (unless its
    /// address is null), the words at `zero`, then `memset(this + off, 0,
    /// n)` when given. Returns `this`.
    #[allow(clippy::too_many_arguments)]
    pub fn ctor_block(
        h: &mut impl Host,
        this: u32,
        base: u32,
        flag: u32,
        block: u32,
        words: u32,
        zero: &[u32],
        memset: Option<(u32, u32)>,
    ) -> u32 {
        h.call_sym(base, Conv::Thiscall(this), &[]);
        h.write(this + flag, 0u32);
        let b = this.wrapping_add(block);
        if b != 0 {
            for i in 0..words {
                h.write(b + 4 * i, 0u32);
            }
            h.write(this + flag, 1u32);
        }
        for &off in zero {
            h.write(this + off, 0u32);
        }
        if let Some((off, n)) = memset {
            h.call_sym(MEMSET, Conv::Cdecl, &[this + off, 0, n]);
        }
        this
    }

    /// `0x0055e060(this)`: the spin wait `0x00564680(this)`, then
    /// `0x005648c0(this)` when bit 0 of `+0x30` is set, then the
    /// `PSharray` at `+0x14` freed (owned, more than one element) and
    /// cleared.
    pub fn wait_and_clear(h: &mut impl Host, this: u32) {
        h.call_sym(0x0056_4680, Conv::Thiscall(this), &[]);
        if h.read::<u8>(this + 0x30) & 1 != 0 {
            h.call_sym(0x0056_48c0, Conv::Thiscall(this), &[]);
        }
        free_sharray(h, this, 0x14, true);
    }

    /// Releases the COM objects at `+0x10`, `+0xc` and `+8` of `p`, then
    /// `PRenderRingBuffer::~PRenderRingBuffer(p)`; returns its `eax`
    /// (the D3D11 buffer destructors).
    pub fn release_buffer(h: &mut impl Host, p: u32) -> u32 {
        release(h, p + 0x10);
        release(h, p + 0xc);
        release(h, p + 8);
        h.call_sym(0x0050_32c0, Conv::Thiscall(p), &[])
    }

    /// `destructInstance(p)` of the D3D11 buffers: [`release_buffer`];
    /// `al = 1`.
    pub fn destruct_buffer(h: &mut impl Host, p: u32) -> u8 {
        release_buffer(h, p);
        1
    }

    /// `PInstanceList::freeBuffer(this)`: when `+0x49` is set, a scene
    /// cursor over the list (`PhyreSceneCursor_SetNode(&cursor; this)`,
    /// the cursor at `frame`: slot, free slot, stride, count, offset)
    /// and the type's (`+0x2c`) virtual method 11 on every live slot
    /// (`slot + offset`; free slots skipped along the free chain); then
    /// the buffer at `+0x34` freed when `+0x48` is set, and `+0x34`,
    /// `+0x30`, `+0x3c`, `+0x40` cleared. `frame` is the original's
    /// `ebp - 0x1c` (the cursor; `this` at `+0x14`, the type at `+0x18`).
    pub fn free_buffer(h: &mut impl Host, this: u32, frame: u32) {
        h.write(frame + 0x14, this);
        if h.read::<u8>(this + 0x49) != 0 {
            let ty: u32 = h.read(this + 0x2c);
            h.write(frame + 0x18, ty);
            h.call_sym(0x0044_1dc0, Conv::Thiscall(frame), &[this]);
            let mut n: u32 = h.read(frame + 0xc);
            if n != 0 {
                let mut free: u32 = h.read(frame + 4);
                let mut slot: u32 = h.read(frame);
                'live: loop {
                    let off: u32 = h.read(frame + 0x10);
                    let vt: u32 = h.read(ty);
                    let f: u32 = h.read(vt + 0x2c);
                    h.call(f, Conv::Thiscall(ty), &[off.wrapping_add(slot)]);
                    let stride: u32 = h.read(frame + 8);
                    loop {
                        slot = slot.wrapping_add(stride);
                        n = n.wrapping_sub(1);
                        if free != slot {
                            break;
                        }
                        free = h.read(free);
                        if n == 0 {
                            break 'live;
                        }
                    }
                    if n == 0 {
                        break;
                    }
                }
            }
        }
        if h.read::<u8>(this + 0x48) != 0 {
            let p: u32 = h.read(this + 0x34);
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p]);
        }
        for off in [0x34, 0x30, 0x3c, 0x40] {
            h.write(this + off, 0u32);
        }
    }

    /// The scalar deleting destructor `(this; flags)` of a
    /// `PDynamicMeshInstance` subclass: vtable `0x00b23880`, the owned
    /// storage at `+0x10` (count `+0xc`) freed and cleared, the base
    /// destructor `0x004fa470`, `this` freed when bit 0 of `flags`;
    /// returns `this`.
    pub fn dynamic_dtor(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        let vt = h.rebase(0x00b2_3880);
        h.write(this, vt);
        if h.read::<u32>(this + 0xc) & BORROWED == 0 {
            let p: u32 = h.read(this + 0x10);
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p]);
        }
        h.write(this + 0x10, 0u32);
        h.write(this + 0xc, 0u32);
        h.call_sym(0x004f_a470, Conv::Thiscall(this), &[]);
        if flags as u8 & 1 != 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[this]);
        }
        this
    }

    /// `constructInstance(p)` of that class: the registry `0x00cbb300`'s
    /// lookup minus 4 (`al = 0` when that is null), then, when `p` is
    /// set, `PDynamicMeshInstance::PDynamicMeshInstance(p; type)`, vtable
    /// `0x00b23880`, `+0xc` and `+0x10` zero; `al = 1`.
    pub fn construct_dynamic(h: &mut impl Host, p: u32) -> u8 {
        let r = h.rebase(0x00cb_b300);
        let t = h.call_sym(crate::descriptor::REGISTRY_LOOKUP, Conv::Thiscall(r), &[]);
        if t == 0 || t == 4 {
            return 0;
        }
        if p != 0 {
            h.call_sym(0x004f_a420, Conv::Thiscall(p), &[t.wrapping_sub(4)]);
            let vt = h.rebase(0x00b2_3880);
            h.write(p, vt);
            h.write(p + 0xc, 0u32);
            h.write(p + 0x10, 0u32);
        }
        1
    }

    /// `0x004e4df0(items, n)` (cdecl): when `items` is set, the two
    /// words of each 8-byte element freed unless their bit 0 is set
    /// (`+4` first), then `items` freed.
    pub fn free_pairs(h: &mut impl Host, items: u32, n: u32) {
        if items == 0 {
            return;
        }
        for i in 0..n {
            let e = items.wrapping_add(8 * i);
            for off in [4, 0] {
                let w: u32 = h.read(e.wrapping_add(off));
                if w & 1 == 0 {
                    h.call_sym(PHYRE_FREE, Conv::Cdecl, &[w]);
                }
            }
        }
        h.call_sym(PHYRE_FREE, Conv::Cdecl, &[items]);
    }

    /// `0x004f84f0`/`0x004f8580(out, a, b)` (cdecl): the component-wise
    /// maximum (`max`) or minimum of two 3-vectors, x87: `b` unless `a`
    /// wins strictly (`b < a` for the maximum, `b > a` for the minimum),
    /// so an unordered pair gives `b`; each value through `fld`/`fstp`.
    /// Returns `out`.
    pub fn vec3_select(h: &mut impl Host, out: u32, a: u32, b: u32, max: bool) -> u32 {
        let mut r = [0u32; 3];
        for (i, slot) in (0u32..).zip(r.iter_mut()) {
            let x = x87::load(h.read(a + 4 * i));
            let y = x87::load(h.read(b + 4 * i));
            let a_wins = if max { y < x } else { y > x };
            *slot = x87::store(if a_wins { x } else { y });
        }
        for (i, w) in (0u32..).zip(r) {
            h.write(out + 4 * i, w);
        }
        out
    }

    /// `constructInstance(p)` of `PTexture2DD3D11`: its constructor's
    /// body (`crate::h2_more::bridge::texture2d_ctor`) when `p` is set;
    /// `al = 1`.
    pub fn construct_texture2d(h: &mut impl Host, p: u32) -> u8 {
        if p != 0 {
            crate::h2_more::bridge::texture2d_ctor(h, p);
        }
        1
    }

    /// `0x005856b0(this)`: `0x0059c9c0(this)` (the release), the owned
    /// buffers at `+0x4e0` (count, data `+0x4e4`) freed by
    /// `Phyre_FreeOwnedBuffers20(data, count)`, both cleared, then the
    /// tail call `0x00585610(this)`; returns its `eax`.
    pub fn program_dtor(h: &mut impl Host, this: u32) -> u32 {
        h.call_sym(0x0059_c9c0, Conv::Thiscall(this), &[]);
        let n: u32 = h.read(this + 0x4e0);
        if n & BORROWED == 0 {
            let data: u32 = h.read(this + 0x4e4);
            h.call_sym(0x0058_8190, Conv::Cdecl, &[data, n & COUNT]);
        }
        h.write(this + 0x4e4, 0u32);
        h.write(this + 0x4e0, 0u32);
        h.call_sym(0x0058_5610, Conv::Thiscall(this), &[])
    }

    /// `0x00585610(this)`: the owned `PSharray`s at `+0xc` and `+4` freed
    /// (when not null) and cleared.
    pub fn free_two_arrays(h: &mut impl Host, this: u32) {
        free_sharray(h, this, 0xc, false);
        free_sharray(h, this, 4, false);
    }

    /// `PDynamicMeshInstance::Bind()`: the class registered
    /// (`registerType(0x00ca7f28)`), its data member `m_dynamicMesh`
    /// built once (guard `0x00ca7fe8`: `PClassDataMember(0x00ca7fbc;
    /// class, type 0x00c9a7d0, name 0x00b23794, 4, 0xa, 0)`, atexit
    /// `0x00aff7f0`), then `addToNamespace(class)`; returns its `eax`.
    pub fn dynamic_mesh_bind(h: &mut impl Host) -> u32 {
        let class = h.rebase(0x00ca_7f28);
        h.call_sym(crate::declare::REGISTER_TYPE, Conv::Cdecl, &[class]);
        let g = h.rebase(0x00ca_7fe8);
        let v: u32 = h.read(g);
        if v & 1 == 0 {
            h.write(g, v | 1);
            let member = h.rebase(0x00ca_7fbc);
            let ty = h.rebase(0x00c9_a7d0);
            let name = h.rebase(0x00b2_3794);
            h.call_sym(
                crate::declare::DATA_MEMBER_CTOR,
                Conv::Thiscall(member),
                &[class, ty, name, 4, 0xa, 0],
            );
            let d = h.rebase(0x00af_f7f0);
            h.call_sym(crate::declare::ATEXIT, Conv::Cdecl, &[d]);
        }
        h.call_sym(crate::declare::ADD_TO_NAMESPACE, Conv::Thiscall(class), &[])
    }

    /// `PRendererD3D11::setViewport(this; x, y, w, h)`: a
    /// `D3D11_VIEWPORT` at `frame + 4` (x and the flipped y `[+0x1338] -
    /// y - h` as signed integers, w and h unsigned, depth 0..1; each
    /// `fild` rounded once to `f32`), the temporary `[frame] = h`, then
    /// `RSSetViewports(1, &vp)` on the context `+0x70` (method 44);
    /// returns its `eax`. `frame` is the original's `ebp - 0x20`.
    // the original's `fild`/`fstp` round each integer once, as `as f32` does
    #[allow(clippy::cast_precision_loss)]
    pub fn set_viewport_rect(h: &mut impl Host, this: u32, r: [u32; 4], frame: u32) -> u32 {
        let [x, y, w, hh] = r;
        let height: u32 = h.read(this + 0x1338);
        let flipped = height.wrapping_sub(y).wrapping_sub(hh);
        let vp = [
            (x as i32 as f32).to_bits(),
            (flipped as i32 as f32).to_bits(),
            (w as f32).to_bits(),
            (hh as f32).to_bits(),
            0,
            0x3f80_0000,
        ];
        h.write(frame, hh);
        for (i, v) in (0u32..).zip(vp) {
            h.write(frame + 4 + 4 * i, v);
        }
        let ctx: u32 = h.read(this + 0x70);
        let vt: u32 = h.read(ctx);
        let f: u32 = h.read(vt + 0xb0);
        h.call(f, Conv::Thiscall(vt), &[ctx, 1, frame + 4])
    }

    /// `0x00599f30(this; color, i)`: 5 for `i >= 8`; else the four words
    /// of `color` into render target `i`'s colour (`+0x228 + 0x10 i`),
    /// with no color those of the default local at `frame`
    /// (`0x005b3620(&local)` gives them, `0x005b3640(&local)` destroys
    /// it); 0.
    pub fn set_target_color(h: &mut impl Host, this: u32, color: u32, i: u32, frame: u32) -> u32 {
        if i >= 8 {
            return 5;
        }
        let dst = this + 0x228 + 0x10 * i;
        if color != 0 {
            for k in 0..4 {
                let w: u32 = h.read(color + 4 * k);
                h.write(dst + 4 * k, w);
            }
            return 0;
        }
        let p = h.call_sym(0x005b_3620, Conv::Thiscall(frame), &[]);
        for k in 0..4 {
            let w: u32 = h.read(p.wrapping_add(4 * k));
            h.write(dst + 4 * k, w);
        }
        h.call_sym(0x005b_3640, Conv::Thiscall(frame), &[]);
        0
    }

    /// `0x004f49d0()`: a new 0x3c-byte object (`PhyreMalloc`, null when
    /// that fails) built by `0x00583240`, `+0 = 0x3000`, its storage sized
    /// to 8 (`0x00599a80(p; 8)` unless the count is 8 already; `+4 = 0`
    /// when that returns 0), bound (`PTextureD3D11_bind`) unless
    /// `PConstantBufferNVN_isBound` says so (`al`); returns it.
    pub fn new_constant_buffer(h: &mut impl Host) -> u32 {
        let p = h.call_sym(0x0042_fb40, Conv::Cdecl, &[0x3c]);
        if p == 0 {
            return 0;
        }
        h.call_sym(0x0058_3240, Conv::Thiscall(p), &[]);
        let n = h.read::<u32>(p + 0xc) & COUNT;
        h.write(p, 0x3000u32);
        if n != 8 && h.call_sym(0x0059_9a80, Conv::Thiscall(p), &[8]) == 0 {
            h.write(p + 4, 0u32);
        }
        if h.call_sym(0x0059_5f70, Conv::Thiscall(p), &[]) as u8 == 0 {
            h.call_sym(0x0058_cf20, Conv::Thiscall(p), &[]);
        }
        p
    }
}
