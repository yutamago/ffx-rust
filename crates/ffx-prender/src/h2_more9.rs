//! h2-render-b continuation round, batch 3: shader-pass and sprite
//! collection construction and destruction, the D3D11 sampler state's
//! creation, the dynamic buffers' map for writing and reading.
//!
//! COM methods stay calls through the object's vtable (`stdcall`, the
//! object first), `ecx` as the original leaves it.

use ffx_bridge::x87;
use ffx_graphic::host::{Conv, Host};

use crate::descriptor::PHYRE_FREE;

/// The render-thread device holder's guard word, the holder, its device
/// and context words.
const HOLDER_GUARD: u32 = 0x00c9_628c;
const HOLDER: u32 = 0x00c9_4f00;
const HOLDER_DEVICE: u32 = 0x00c9_4f6c;
const HOLDER_CONTEXT: u32 = 0x00c9_4f70;
const HOLDER_CTOR: u32 = 0x004f_ee70;
const HOLDER_DTOR: u32 = 0x00af_b710;
const ATEXIT: u32 = 0x0094_9207;
const SCOPE_ENTER: u32 = 0x0046_3d50;
const SCOPE_LEAVE: u32 = 0x0046_3e90;

/// The shader pass's callees.
const CLEAR_24: u32 = 0x004b_5050;
const STATE_BLOCK_CTOR: u32 = 0x0058_3e20;
const RELEASE_VIEWS: u32 = 0x0059_c950;
/// The sprite collection's callees.
const DELETE_OBJECT: u32 = 0x0044_1240;
const SAMPLER_DTOR: u32 = 0x0058_5230;
const WAIT_AND_CLEAR: u32 = 0x0055_e060;
const RELEASE40: u32 = 0x0043_3900;
/// The sampler creation's callees and constants.
const SAMPLER_DEFAULTS: u32 = 0x0058_3100;
const DEFAULTS_FLAG: u32 = 0x00b3_144c;
const MAX_ANISOTROPY: u32 = 0x00c2_5370;
const MAP_FILTER: u32 = 0x0058_aa10;
const MAP_MODE: u32 = 0x0058_ab10;
const COLOR_SCALE: u32 = 0x00b3_4258;
const NAME_GUID: u32 = 0x00b6_ae3c;
const SAMPLER_NAME: u32 = 0x00b1_d7e4;
/// `PRenderThread_mapResource(&out, resource, mode)` (cdecl).
const MAP_RESOURCE: u32 = 0x0056_0f90;

/// A COM method: `slot` (word index) of `obj`'s vtable, `stdcall` with
/// `obj` first, `ecx` as the original leaves it.
fn com(h: &mut impl Host, ecx: u32, obj: u32, slot: u32, args: &[u32]) -> u32 {
    let vt: u32 = h.read(obj);
    let f: u32 = h.read(vt + 4 * slot);
    let mut words = Vec::with_capacity(args.len() + 1);
    words.push(obj);
    words.extend_from_slice(args);
    h.call(f, Conv::Thiscall(ecx), &words) // call width: 7 (six arguments at most)
}

/// The holder of the render thread's device, built on first use (the
/// guard's bit 0; its destructor registered with `atexit`).
fn holder(h: &mut impl Host) {
    let g = h.rebase(HOLDER_GUARD);
    let v: u32 = h.read(g);
    if v & 1 == 0 {
        h.write(g, v | 1);
        let holder = h.rebase(HOLDER);
        h.call_sym(HOLDER_CTOR, Conv::Thiscall(holder), &[]);
        let dtor = h.rebase(HOLDER_DTOR);
        h.call_sym(ATEXIT, Conv::Cdecl, &[dtor]);
    }
}

/// An owned storage freed: the pointer at `ptr` unless the count word at
/// `count` has bit 31 (borrowed) or the pointer is null; both cleared.
fn free_owned(h: &mut impl Host, count: u32, ptr: u32) {
    if h.read::<u32>(count) >> 31 == 0 {
        let p: u32 = h.read(ptr);
        if p != 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p]);
        }
    }
    h.write(ptr, 0u32);
    h.write(count, 0u32);
}

/// The first word of the 20-byte entry array at `this + 0x18` (count,
/// then the inline entry or the storage pointer); 0 when empty.
fn entries(h: &mut impl Host, this: u32) -> u32 {
    match h.read::<u32>(this + 0x18) & 0x7fff_ffff {
        0 => 0,
        1 => this + 0x1c,
        _ => h.read(this + 0x1c),
    }
}

pub mod bridge {
    //! The originals over game memory.

    use super::{
        BIND_PASS_STATE, BLEND_NAME, BUFFER_COUNT, CONSTANT_NAME, DEPTH_NAME, FROM_AXES,
        JOINT_ROWS, MALLOC_ALIGNED, MATRIX_COPY, MEMCPY, RASTER_NAME, ROTATE3, TRANSFORM3, slots,
    };
    use super::{
        CLEAR_24, COLOR_SCALE, Conv, DEFAULTS_FLAG, DELETE_OBJECT, HOLDER_CONTEXT, HOLDER_DEVICE,
        Host, MAP_FILTER, MAP_MODE, MAP_RESOURCE, MAX_ANISOTROPY, NAME_GUID, PHYRE_FREE,
        RELEASE_VIEWS, RELEASE40, SAMPLER_DEFAULTS, SAMPLER_DTOR, SAMPLER_NAME, SCOPE_ENTER,
        SCOPE_LEAVE, STATE_BLOCK_CTOR, WAIT_AND_CLEAR, com, entries, free_owned, holder, x87,
    };

    /// `PShaderPassD3D11::PShaderPassD3D11(this)` (`0x00583c00`,
    /// thiscall): the base (24 zero bytes), the render-state block at
    /// `+0x18`, then the eight 24-byte resource tables at `+0x194`
    /// zeroed. Returns `this`.
    pub fn shader_pass_ctor(h: &mut impl Host, this: u32) -> u32 {
        h.call_sym(CLEAR_24, Conv::Thiscall(this), &[]);
        h.call_sym(STATE_BLOCK_CTOR, Conv::Thiscall(this + 0x18), &[]);
        for k in 0..0x30 {
            h.write(this + 0x194 + 4 * k, 0u32);
        }
        this
    }

    /// `PShaderPassD3D11::~PShaderPassD3D11(this)` (`0x005853c0`,
    /// thiscall): the views released (`+0x18`, `0x0059c950`), then the
    /// eight resource tables' storages (`+0x24c` down to `+0x1a4`) and
    /// the one at `+0x10` freed when owned, their words cleared.
    pub fn shader_pass_dtor(h: &mut impl Host, this: u32) {
        h.call_sym(RELEASE_VIEWS, Conv::Thiscall(this + 0x18), &[]);
        for count in [0x24c, 0x234, 0x21c, 0x204, 0x1ec, 0x1d4, 0x1bc, 0x1a4, 0x10] {
            free_owned(h, this + count, this + count + 4);
        }
    }

    /// `PSpriteCollection::~PSpriteCollection(this)` (`0x005676c0`,
    /// thiscall): its two cluster objects (`+0`, `+0x10`) deleted from
    /// their cluster (`+8`), the three arrays at `+0xc4..+0xd8` freed
    /// when owned, the sampler state (`+0x94`) destroyed, the instance
    /// list at `+0x24` (its job, its 40-byte array, its storage), then
    /// the array at `+0x14` (freed only when owned and over one entry).
    pub fn sprite_collection_dtor(h: &mut impl Host, this: u32) {
        for at in [0, 0x10] {
            let obj: u32 = h.read(this + at);
            if obj != 0 {
                let cluster: u32 = h.read(this + 8);
                h.call_sym(DELETE_OBJECT, Conv::Thiscall(cluster), &[obj]);
            }
        }
        for count in [0xd4, 0xcc, 0xc4] {
            free_owned(h, this + count, this + count + 4);
        }
        h.call_sym(SAMPLER_DTOR, Conv::Thiscall(this + 0x94), &[]);
        let list = this + 0x24;
        h.call_sym(WAIT_AND_CLEAR, Conv::Thiscall(list + 0x30), &[]);
        h.call_sym(RELEASE40, Conv::Thiscall(list + 0x28), &[]);
        free_owned(h, list + 8, list + 0xc);
        let c: u32 = h.read(this + 0x14);
        if c >> 31 == 0 && c & 0x7fff_ffff > 1 {
            let p: u32 = h.read(this + 0x18);
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p]);
        }
        h.write(this + 0x18, 0u32);
        h.write(this + 0x14, 0u32);
    }

    /// The bytes of `createSamplerState`'s frame below its cookie
    /// (`ebp - 0x54` up to `ebp - 0x10`).
    pub const SAMPLER_FRAME: u32 = 0x44;

    /// `PSamplerStateD3D11::create(this)` (`0x00595bf0`, thiscall): 0x16
    /// when it has its state object (`+0x20`) already; else, under the
    /// render scope lock and with the device holder, when the device
    /// exists: the description at `ebp - 0x44` (defaults by
    /// `0x00583100`, the filter from the bytes `+0`, `+1` and bit 6 of
    /// `+0x1c`, the address modes from `+2..+4`, the bias `+8`, the
    /// anisotropy `+0xc` (first raised to the integer part of the global
    /// maximum `0x00c25370` when that is over 1), the comparison 4 or 1,
    /// the border colour from the bytes `+0x10..+0x13` scaled by 1/255),
    /// `CreateSamplerState` and the object named. Returns 0, or 0xe when
    /// the creation fails. `frame` is the original's `ebp - 0x54` (the
    /// lock, the x87 control-word temporaries and the description).
    #[allow(clippy::too_many_lines)]
    pub fn create_sampler_state(h: &mut impl Host, this: u32, frame: u32) -> u32 {
        let local = |off: u32| frame + 0x54 - off;
        let out = this + 0x20;
        if h.read::<u32>(out) != 0 {
            return 0x16;
        }
        let lock = local(0x54);
        h.call_sym(SCOPE_ENTER, Conv::Thiscall(lock), &[]);
        holder(h);
        let device_at = h.rebase(HOLDER_DEVICE);
        if h.read::<u32>(device_at) != 0 {
            let flag: u8 = h.read(h.rebase(DEFAULTS_FLAG));
            let desc = local(0x44);
            h.call_sym(SAMPLER_DEFAULTS, Conv::Thiscall(desc), &[u32::from(flag)]);
            // the anisotropy raised to the global maximum's integer part
            // (`fistp qword` toward zero, the low word as unsigned)
            let max = x87::load(h.read(h.rebase(MAX_ANISOTROPY)));
            if max > 1.0 {
                let n = x87::ftol2(max);
                h.write(this + 0xc, x87::store(f64::from(n)));
            }
            let comparison = (h.read::<u32>(this + 0x1c) >> 6) & 0xffff_ff01;
            let [min, mag]: [u8; 2] = [h.read(this), h.read(this + 1)];
            let filter = h.call_sym(
                MAP_FILTER,
                Conv::Cdecl,
                &[u32::from(min), u32::from(mag), comparison],
            );
            h.write(desc, filter);
            for k in 0..3 {
                let mode: u8 = h.read(this + 2 + k);
                let m = h.call_sym(MAP_MODE, Conv::Cdecl, &[u32::from(mode)]);
                h.write(desc + 4 + 4 * k, m);
            }
            let bias: u32 = h.read(this + 8);
            h.write(local(0x34), x87::store(x87::load(bias)));
            let aniso = x87::ftol2(x87::load(h.read(this + 0xc)));
            h.write(local(0x30), aniso);
            // the border colour: bytes times 1/255 (an f64 constant)
            let scale = f64::from_bits(h.read(h.rebase(COLOR_SCALE)));
            let bytes: [u8; 4] = [
                h.read(this + 0x10),
                h.read(this + 0x11),
                h.read(this + 0x12),
                h.read(this + 0x13),
            ];
            for (k, b) in (0u32..).zip(bytes) {
                let c = x87::mul(f64::from(b), scale);
                h.write(local(0x28 - 4 * k), x87::store(c));
            }
            // the temporaries as the conversions leave them: the control
            // word (the game's 0x027f) and the last byte as a double
            h.write(local(0x4e), 0x027fu16);
            h.write(local(0x4c), f64::from(bytes[3]).to_bits());
            let cmp = if comparison as u8 != 0 { 4u32 } else { 1 };
            h.write(local(0x2c), cmp);
            holder(h);
            let device: u32 = h.read(device_at);
            let vt: u32 = h.read(device);
            if (com(h, vt, device, 23, &[desc, out]) as i32) < 0 {
                h.call_sym(SCOPE_LEAVE, Conv::Thiscall(lock), &[]);
                return 0xe;
            }
            let state: u32 = h.read(out);
            if state == 0 {
                h.call_sym(SCOPE_LEAVE, Conv::Thiscall(lock), &[]);
                return 0xe;
            }
            let guid = h.rebase(NAME_GUID);
            let name = h.rebase(SAMPLER_NAME);
            com(h, state, state, 5, &[guid, 0xd, name]);
        }
        h.call_sym(SCOPE_LEAVE, Conv::Thiscall(lock), &[]);
        0
    }

    /// The bytes of `mapForWrite`'s frame below its cookie (`ebp - 0x3c`
    /// up to `ebp - 0x10`).
    pub const MAP_WRITE_FRAME: u32 = 0x2c;

    /// `PDynamicBufferD3D11::mapForWrite(this; i, out)` (`0x00563290`,
    /// thiscall, ret 8): entry `i` of the 20-byte entries at `+0x18`
    /// (`GetDesc` of its buffer): a CPU-writable buffer is mapped by
    /// `PRenderThread_mapResource` (mode 2); else, under the render scope
    /// lock, through its staging copy (`+0xc`, created on first use:
    /// staging, CPU read and write) mapped for writing. `out` gets the
    /// data pointer plus `+0x30` and the word `+0x38`. Returns 0, 9 when
    /// D3D11 fails, or the map call's result. `frame` is the original's
    /// `ebp - 0x3c`.
    pub fn map_for_write(h: &mut impl Host, this: u32, i: u32, out: u32, frame: u32) -> u32 {
        let local = |off: u32| frame + 0x3c - off;
        h.write(local(0x2c), 0u32);
        let row = i.wrapping_mul(20);
        let base = entries(h, this);
        let buffer: u32 = h.read(base.wrapping_add(row));
        let desc = local(0x28);
        let vt: u32 = h.read(buffer);
        com(h, vt, buffer, 10, &[desc]);
        let data = if h.read::<u32>(local(0x1c)) & 0x1_0000 != 0 {
            let base = entries(h, this);
            let resource: u32 = h.read(base.wrapping_add(row));
            let r = h.call_sym(MAP_RESOURCE, Conv::Cdecl, &[local(0x2c), resource, 2]);
            if r != 0 {
                return r;
            }
            h.read::<u32>(local(0x2c))
        } else {
            let lock = local(0x30);
            h.call_sym(SCOPE_ENTER, Conv::Thiscall(lock), &[]);
            holder(h);
            let base = entries(h, this);
            let staging = base.wrapping_add(row) + 0xc;
            if h.read::<u32>(staging) == 0 {
                let device: u32 = h.read(h.rebase(HOLDER_DEVICE));
                h.write(local(0x24), 3u32);
                h.write(local(0x20), 0u32);
                h.write(local(0x1c), 0x3_0000u32);
                h.write(local(0x18), 0u32);
                let vt: u32 = h.read(device);
                if com(h, vt, device, 3, &[desc, 0, staging]) != 0 {
                    h.call_sym(SCOPE_LEAVE, Conv::Thiscall(lock), &[]);
                    return super::FAILED;
                }
            }
            let context: u32 = h.read(h.rebase(HOLDER_CONTEXT));
            let base = entries(h, this);
            let copy: u32 = h.read(base.wrapping_add(row) + 0xc);
            let mapped = local(0x3c);
            let r = com(h, base, context, 14, &[copy, 0, 2, 0, mapped]);
            let p: u32 = h.read(mapped);
            if (r as i32) < 0 || p == 0 {
                h.call_sym(SCOPE_LEAVE, Conv::Thiscall(lock), &[]);
                return super::FAILED;
            }
            h.call_sym(SCOPE_LEAVE, Conv::Thiscall(lock), &[]);
            p
        };
        let offset: u32 = h.read(this + 0x30);
        h.write(out, offset.wrapping_add(data));
        let word: u32 = h.read(this + 0x38);
        h.write(out + 4, word);
        0
    }

    /// The bytes of `mapForRead`'s frame below its cookie (`ebp - 0x48`
    /// up to `ebp - 0x10`).
    pub const MAP_READ_FRAME: u32 = 0x38;

    /// `PDynamicBufferD3D11::mapForRead(this; i, out)` (`0x00562e00`,
    /// thiscall, ret 8): as [`map_for_write`] for reading: a CPU-readable
    /// buffer is mapped directly (1 when it has none; the lock at
    /// `ebp - 0x2c`); else its staging copy is created when missing,
    /// filled by `CopyResource` and mapped. Returns 0, 1 or 9. `frame` is
    /// the original's `ebp - 0x48`.
    pub fn map_for_read(h: &mut impl Host, this: u32, i: u32, out: u32, frame: u32) -> u32 {
        let local = |off: u32| frame + 0x48 - off;
        h.write(local(0x2c), out);
        let row = i.wrapping_mul(20);
        let base = entries(h, this);
        let buffer: u32 = h.read(base.wrapping_add(row));
        let desc = local(0x28);
        let vt: u32 = h.read(buffer);
        com(h, vt, buffer, 10, &[desc]);
        let (data, out) = if h.read::<u32>(local(0x1c)) & 0x2_0000 != 0 {
            let base = entries(h, this);
            let resource: u32 = h.read(base.wrapping_add(row));
            if resource == 0 {
                return 1;
            }
            let lock = local(0x2c);
            h.call_sym(SCOPE_ENTER, Conv::Thiscall(lock), &[]);
            holder(h);
            let context: u32 = h.read(h.rebase(HOLDER_CONTEXT));
            let vt: u32 = h.read(context);
            let mapped = local(0x48);
            let r = com(h, vt, context, 14, &[resource, 0, 1, 0, mapped]);
            let p: u32 = h.read(mapped);
            if (r as i32) < 0 || p == 0 {
                h.call_sym(SCOPE_LEAVE, Conv::Thiscall(lock), &[]);
                return super::FAILED;
            }
            h.call_sym(SCOPE_LEAVE, Conv::Thiscall(lock), &[]);
            (p, out)
        } else {
            let lock = local(0x30);
            h.call_sym(SCOPE_ENTER, Conv::Thiscall(lock), &[]);
            holder(h);
            let base = entries(h, this);
            let staging = base.wrapping_add(row) + 0xc;
            if h.read::<u32>(staging) == 0 {
                let device: u32 = h.read(h.rebase(HOLDER_DEVICE));
                h.write(local(0x24), 3u32);
                h.write(local(0x20), 0u32);
                h.write(local(0x1c), 0x3_0000u32);
                h.write(local(0x18), 0u32);
                let vt: u32 = h.read(device);
                if com(h, vt, device, 3, &[desc, 0, staging]) != 0 {
                    h.call_sym(SCOPE_LEAVE, Conv::Thiscall(lock), &[]);
                    return super::FAILED;
                }
            }
            let context: u32 = h.read(h.rebase(HOLDER_CONTEXT));
            let base = entries(h, this);
            let src: u32 = h.read(base.wrapping_add(row));
            let copy: u32 = h.read(base.wrapping_add(row) + 0xc);
            com(h, base, context, 47, &[copy, src]);
            let base = entries(h, this);
            let copy: u32 = h.read(base.wrapping_add(row) + 0xc);
            let mapped = local(0x3c);
            let r = com(h, base, context, 14, &[copy, 0, 1, 0, mapped]);
            let p: u32 = h.read(mapped);
            if (r as i32) < 0 || p == 0 {
                h.call_sym(SCOPE_LEAVE, Conv::Thiscall(lock), &[]);
                return super::FAILED;
            }
            h.call_sym(SCOPE_LEAVE, Conv::Thiscall(lock), &[]);
            // `out` from the frame word (the read path's lock lives there)
            (p, h.read(local(0x2c)))
        };
        let offset: u32 = h.read(this + 0x30);
        h.write(out, offset.wrapping_add(data));
        let word: u32 = h.read(this + 0x38);
        h.write(out + 4, word);
        0
    }

    /// `PRendererD3D11::setUnorderedAccessViews(this; views, count,
    /// initial)` (`0x0058dbe0`, thiscall, ret 12): the eight UAV slots
    /// after the bound render targets (`+0x2fc` of them): slot `k` gets
    /// `views[targets + k]` while `targets + k < count` (at most 8), else
    /// null. When the count differs from `+0x300`, a slot differs from
    /// `+0x2dc`, or an initial count is not -1 (read only where the slot
    /// matches), the slots are stored and
    /// `OMSetRenderTargetsAndUnorderedAccessViews` (the targets `+0x2b8`
    /// when there are any, the depth view `+0x2d8`) is called and the
    /// count kept. Returns 0.
    pub fn set_unordered_access_views(
        h: &mut impl Host,
        this: u32,
        views: u32,
        count: u32,
        initial: u32,
    ) -> u32 {
        let n = count.min(8);
        let targets: u32 = h.read(this + 0x2fc);
        let mut w = [0u32; 8];
        for (k, slot) in (0u32..).zip(w.iter_mut()) {
            let at = targets.wrapping_add(k);
            if at < n {
                *slot = h.read(views.wrapping_add(at.wrapping_mul(4)));
            }
        }
        let mut changed = n != h.read::<u32>(this + 0x300);
        if !changed {
            for (k, &v) in (0u32..).zip(w.iter()) {
                if h.read::<u32>(this + 0x2dc + 4 * k) != v
                    || h.read::<u32>(initial + 4 * k) != u32::MAX
                {
                    changed = true;
                }
            }
        }
        if !changed {
            return 0;
        }
        for (k, &v) in (0u32..).zip(w.iter()) {
            h.write(this + 0x2dc + 4 * k, v);
        }
        let targets_now: u32 = h.read(this + 0x2fc);
        let rtvs = if targets_now != 0 { this + 0x2b8 } else { 0 };
        let context: u32 = h.read(this + 0x70);
        let depth: u32 = h.read(this + 0x2d8);
        let uavs = n.wrapping_sub(h.read(this + 0x2fc));
        let targets_arg: u32 = h.read(this + 0x2fc);
        com(
            h,
            context,
            context,
            34,
            &[
                targets_arg,
                rtvs,
                depth,
                targets,
                uavs,
                this + 0x2dc,
                initial,
            ],
        );
        h.write(this + 0x300, n);
        0
    }

    /// The bytes of `createConstantBuffers`' frame below its cookie
    /// (`ebp - 0x48` up to `ebp - 0x10`).
    pub const CONSTANT_FRAME: u32 = 0x38;

    /// `PConstantBufferD3D11::create(this; size, data)` (`0x0058ee20`,
    /// thiscall, ret 8): the size rounded up past a multiple of 16
    /// (`(size & ~15) + 16`, at `+0`); under the render scope lock and
    /// with the device holder, one dynamic constant buffer per ring slot
    /// (`getBufferCount`, `0x004f7d50`, asked again after each): created
    /// from `data` when given, its description read into `+0x20`, named,
    /// a 16-aligned CPU copy of the size, the slot's third word cleared.
    /// The slots are the 16-byte entries at `+0xc`. Returns 0, or 9
    /// without a device or when a creation fails. `frame` is the
    /// original's `ebp - 0x48`.
    pub fn create_constant_buffers(
        h: &mut impl Host,
        this: u32,
        size: u32,
        data: u32,
        frame: u32,
    ) -> u32 {
        let local = |off: u32| frame + 0x48 - off;
        let size = (size & !0xf).wrapping_add(0x10);
        h.write(local(0x38), size);
        h.write(this, size);
        let lock = local(0x34);
        h.call_sym(SCOPE_ENTER, Conv::Thiscall(lock), &[]);
        holder(h);
        let device: u32 = h.read(h.rebase(HOLDER_DEVICE));
        h.write(local(0x3c), device);
        let r = if device == 0 {
            super::FAILED
        } else {
            let desc = local(0x28);
            for (k, w) in [size, 2, 4, 0x1_0000, 0, 0].into_iter().enumerate() {
                h.write(desc + 4 * k as u32, w);
            }
            let init = local(0x48);
            h.write(init, data);
            h.write(init + 4, 0u32);
            h.write(init + 8, 0u32);
            h.write(local(0x2c), 0u32);
            let mut r = 0;
            if h.call_sym(BUFFER_COUNT, Conv::Thiscall(this), &[]) != 0 {
                let init = if data != 0 { init } else { 0 };
                h.write(local(0x30), init);
                let mut off = 0u32;
                loop {
                    let base = slots(h, this);
                    let vt: u32 = h.read(device);
                    if com(h, vt, device, 3, &[desc, init, base.wrapping_add(off)]) != 0 {
                        r = super::FAILED;
                        break;
                    }
                    let base = slots(h, this);
                    let buffer: u32 = h.read(base.wrapping_add(off));
                    com(h, buffer, buffer, 10, &[this + 0x20]);
                    let base = slots(h, this);
                    let buffer: u32 = h.read(base.wrapping_add(off));
                    let vt: u32 = h.read(buffer);
                    let guid = h.rebase(NAME_GUID);
                    let name = h.rebase(CONSTANT_NAME);
                    com(h, vt, buffer, 5, &[guid, 0xf, name]);
                    let base = slots(h, this);
                    let size: u32 = h.read(local(0x38));
                    let copy = h.call_sym(MALLOC_ALIGNED, Conv::Cdecl, &[size, 0x10]);
                    h.write(base.wrapping_add(off) + 4, copy);
                    let base = slots(h, this);
                    h.write(base.wrapping_add(off) + 8, 0u32);
                    let i = h.read::<u32>(local(0x2c)).wrapping_add(1);
                    h.write(local(0x2c), i);
                    off = off.wrapping_add(0x10);
                    if i >= h.call_sym(BUFFER_COUNT, Conv::Thiscall(this), &[]) {
                        break;
                    }
                }
            }
            r
        };
        h.call_sym(SCOPE_LEAVE, Conv::Thiscall(lock), &[]);
        r
    }

    /// The bytes of `applyPassState`'s frame below its cookie
    /// (`ebp - 0x40` up to `ebp - 0x10`).
    pub const PASS_STATE_FRAME: u32 = 0x30;

    /// One state object `slot` of the device (`CreateRasterizerState` 22,
    /// `CreateBlendState` 20, `CreateDepthStencilState` 21) from `desc`
    /// into `out` under the lock at `lock`, named; false when it fails
    /// (the lock left).
    fn make_state(
        h: &mut impl Host,
        lock: u32,
        slot: u32,
        desc: u32,
        out: u32,
        name: (u32, u32),
    ) -> bool {
        holder(h);
        let device: u32 = h.read(h.rebase(HOLDER_DEVICE));
        let ok = (com(h, device, device, slot, &[desc, out]) as i32) >= 0 && {
            let state: u32 = h.read(out);
            if state != 0 {
                let guid = h.rebase(NAME_GUID);
                let n = h.rebase(name.0);
                com(h, state, state, 5, &[guid, name.1, n]);
            }
            state != 0
        };
        h.call_sym(SCOPE_LEAVE, Conv::Thiscall(lock), &[]);
        ok
    }

    /// `PShaderPassStateD3D11::apply(this; key)` (`0x0059ad20`,
    /// thiscall, ret 4; the state block at `this + 0x18`): creates what
    /// is missing under the render scope lock, each named: the
    /// rasterizer state (`+0x168` from the description `+4`), a
    /// wireframe copy of it (`+0x16c`, fill mode 2, not named), the blend
    /// state (`+0x174` from `+0x60`), the depth-stencil state (`+0x170`
    /// from `+0x2c`); then binds the block (`0x0058c6c0` with `*key`) and
    /// returns its result, or 9 when a creation fails. `frame` is the
    /// original's `ebp - 0x40`.
    pub fn apply_pass_state(h: &mut impl Host, this: u32, key: u32, frame: u32) -> u32 {
        let local = |off: u32| frame + 0x40 - off;
        let block = this + 0x18;
        h.write(local(0x40), key);
        if h.read::<u32>(block + 0x168) == 0 {
            let lock = local(0x3c);
            h.call_sym(SCOPE_ENTER, Conv::Thiscall(lock), &[]);
            if !make_state(h, lock, 22, block + 4, block + 0x168, (RASTER_NAME, 0xb)) {
                return super::FAILED;
            }
        }
        if h.read::<u32>(block + 0x16c) == 0 {
            let lock = local(0x3c);
            h.call_sym(SCOPE_ENTER, Conv::Thiscall(lock), &[]);
            let copy = local(0x38);
            for k in 0..10 {
                let w: u32 = h.read(block + 4 + 4 * k);
                h.write(copy + 4 * k, w);
            }
            h.write(copy, 2u32);
            holder(h);
            let device: u32 = h.read(h.rebase(HOLDER_DEVICE));
            let vt: u32 = h.read(device);
            let out = block + 0x16c;
            let r = com(h, vt, device, 22, &[copy, out]);
            if (r as i32) < 0 || h.read::<u32>(out) == 0 {
                h.call_sym(SCOPE_LEAVE, Conv::Thiscall(lock), &[]);
                return super::FAILED;
            }
            h.call_sym(SCOPE_LEAVE, Conv::Thiscall(lock), &[]);
        }
        if h.read::<u32>(block + 0x174) == 0 {
            let lock = local(0x3c);
            h.call_sym(SCOPE_ENTER, Conv::Thiscall(lock), &[]);
            if !make_state(h, lock, 20, block + 0x60, block + 0x174, (BLEND_NAME, 0xa)) {
                return super::FAILED;
            }
        }
        if h.read::<u32>(block + 0x170) == 0 {
            let lock = local(0x40);
            h.call_sym(SCOPE_ENTER, Conv::Thiscall(lock), &[]);
            if !make_state(h, lock, 21, block + 0x2c, block + 0x170, (DEPTH_NAME, 0x11)) {
                return super::FAILED;
            }
        }
        let k: u32 = h.read(key);
        h.call_sym(BIND_PASS_STATE, Conv::Thiscall(block), &[k])
    }

    /// `PSceneWideParametersKey::operator=(this; src)` (`0x004fd2a0`,
    /// thiscall, ret 4): the two matrices (`PMatrix4` copies at `+0` and
    /// `+0x40`), then the 36 floats `+0x80..+0x10c` one by one through
    /// the FPU (a signalling NaN comes out quiet). Returns `this`.
    pub fn scene_key_assign(h: &mut impl Host, this: u32, src: u32) -> u32 {
        h.call_sym(MATRIX_COPY, Conv::Thiscall(this), &[src]);
        h.call_sym(MATRIX_COPY, Conv::Thiscall(this + 0x40), &[src + 0x40]);
        for k in 0..36 {
            let off = 0x80 + 4 * k;
            let v: u32 = h.read(src + off);
            h.write(this + off, x87::store(x87::load(v)));
        }
        this
    }

    /// The bytes of `copyConstantBuffer`'s frame (`ebp - 0x44` up to
    /// `ebp`: the transposed matrix, `this` at `ebp - 4`).
    pub const COPY_FRAME: u32 = 0x44;

    /// Transposes a matrix of 4 rows of `n` floats at `src` (`n` 4 or 3)
    /// into `n` rows of 4 at the frame words `tmp` (each through the FPU,
    /// in the original's order) and copies them to `dst`.
    fn transpose(h: &mut impl Host, src: u32, dst: u32, n: u32, tmp: u32) {
        for r in 0..n {
            for c in 0..4 {
                let v: u32 = h.read(src + 4 * (c * n + r));
                h.write(tmp + 4 * (r * 4 + c), x87::store(x87::load(v)));
            }
        }
        for j in 0..4 * n {
            let v: u32 = h.read(tmp + 4 * j);
            h.write(dst + 4 * j, v);
        }
    }

    /// `PRenderThread::copyConstantBuffer(this; buf, slot, entries, _,
    /// count)` (`0x0059a650`, thiscall, ret 0x14): with a used buffer
    /// (`+0x14` of `buf`), its data `buf.data[slot]` (`+0x20`) and
    /// `count` (a 16-bit word) entries of 16 bytes (source offset u16,
    /// destination offset, size, kind byte at `+0xc`), each whose
    /// destination is inside this buffer (`+0x14`, `+4`) is copied to
    /// `+8`: kind `'1'` as 4x4 matrices transposed, `'0'` as 4x3 ones
    /// (48 bytes each) transposed to 3x4, a matrix with a first word of
    /// -1 ending the entry; any other kind by `memcpy`. Returns `al` 0.
    /// `frame` is the original's `ebp - 0x44`.
    pub fn copy_constant_buffer(
        h: &mut impl Host,
        this: u32,
        buf: u32,
        slot: u32,
        entries: u32,
        count: u32,
        frame: u32,
    ) -> u8 {
        let local = |off: u32| frame + 0x44 - off;
        h.write(local(4), this);
        if h.read::<u32>(buf + 0x14) == 0 {
            return 0;
        }
        let data: u32 = h.read(buf.wrapping_add(slot.wrapping_mul(4)) + 0x20);
        let count = count & 0xffff;
        if count == 0 || data == 0 || entries == 0 {
            return 0;
        }
        for e in 0..count {
            let entry = entries + 0x10 * e;
            let from: u16 = h.read(entry);
            let src = data.wrapping_add(u32::from(from));
            let at: u32 = h.read(entry + 4);
            if at >= h.read::<u32>(this + 0x14) || at >= h.read::<u32>(this + 4) & 0x7fff_ffff {
                continue;
            }
            let size: u32 = h.read(entry + 8);
            let kind: u8 = h.read(entry + 0xc);
            let base: u32 = h.read(this + 8);
            let mut dst = base.wrapping_add(at);
            let (n_rows, step, tmp) = match kind {
                0x30 => (3, 0x30, local(0x34)),
                0x31 => (4, 0x40, local(0x44)),
                _ => {
                    h.call_sym(MEMCPY, Conv::Cdecl, &[dst, src, size]);
                    continue;
                }
            };
            let n = size / step;
            let mut src = src;
            for _ in 0..n {
                if h.read::<u32>(src) == u32::MAX {
                    break;
                }
                transpose(h, src, dst, n_rows, tmp);
                src = src.wrapping_add(step);
                dst = dst.wrapping_add(step);
            }
        }
        0
    }

    /// The bytes of `computeJointMatrix`'s frame (`ebp - 0x10c` up to
    /// `ebp`).
    pub const JOINT_FRAME: u32 = 0x10c;

    /// `PMeshInstance::computeJointMatrix(this; out)` (`0x004f6fc0`,
    /// thiscall, ret 4): for each joint of the mesh (`[this]`: count
    /// `+0x18`, parent indices `+0x2c`) with local matrices at `+0xc`,
    /// joint `i` of `out` = its parent's matrix (`out[parent[i]]`)
    /// applied to the local matrix: the translation row by
    /// `transformPoint3`, the three axes by `rotate3`, the result built by
    /// `PMatrix4::fromAxes` in the frame and copied out. The local rows
    /// are copied through the FPU first (each word raw at its slot, then
    /// rounded to the callee's input). `frame` is the original's
    /// `ebp - 0x10c`.
    pub fn compute_joint_matrix(h: &mut impl Host, this: u32, out: u32, frame: u32) {
        let local = |off: u32| frame + 0x10c - off;
        let mesh: u32 = h.read(this);
        let mut local_m: u32 = h.read(this + 0xc);
        let mut count = h.read::<u32>(mesh + 0x18) & 0x7fff_ffff;
        let mut parents: u32 = h.read(mesh + 0x2c);
        h.write(local(4), local_m);
        h.write(local(8), parents);
        if local_m == 0 || parents == 0 || count == 0 {
            return;
        }
        let mut dst = out;
        loop {
            h.write(local(0xc), local_m + 0x20);
            let parent_index: u32 = h.read(parents);
            for (row, raw, rounded) in JOINT_ROWS {
                for k in 0..4u32 {
                    let w: u32 = h.read(local_m + row + 4 * k);
                    h.write(local(raw - 4 * k), w);
                    if k < 3 {
                        h.write(local(rounded - 4 * k), x87::store(x87::load(w)));
                    }
                }
            }
            let parent = (parent_index << 6).wrapping_add(out);
            let t = h.call_sym(
                TRANSFORM3,
                Conv::Thiscall(parent),
                &[local(0x10c), local(0x9c)],
            );
            let z = h.call_sym(ROTATE3, Conv::Thiscall(parent), &[local(0xdc), local(0xbc)]);
            let y = h.call_sym(ROTATE3, Conv::Thiscall(parent), &[local(0xfc), local(0xcc)]);
            let x = h.call_sym(ROTATE3, Conv::Thiscall(parent), &[local(0xec), local(0xac)]);
            let m = local(0x8c);
            h.call_sym(FROM_AXES, Conv::Thiscall(m), &[x, y, z, t]);
            for k in 0..16 {
                let w: u32 = h.read(m + 4 * k);
                h.write(dst + 4 * k, x87::store(x87::load(w)));
            }
            parents = parents.wrapping_add(4);
            local_m = local_m.wrapping_add(0x40);
            dst = dst.wrapping_add(0x40);
            h.write(local(8), parents);
            h.write(local(4), local_m);
            count -= 1;
            if count == 0 {
                h.write(local(0xc), local_m + 0x20);
                return;
            }
        }
    }
}

/// `computeJointMatrix`'s local rows 3, 2, 1, 0: (row offset, raw frame
/// slot, rounded frame slot, as `ebp` offsets).
const JOINT_ROWS: [(u32, u32, u32); 4] = [
    (0x30, 0x1c, 0x9c),
    (0x20, 0x4c, 0xbc),
    (0x10, 0x2c, 0xcc),
    (0, 0x3c, 0xac),
];

/// The joint matrix's callees (thiscall on the parent matrix: out, in).
const TRANSFORM3: u32 = 0x004c_9eb0;
const ROTATE3: u32 = 0x004c_a140;
const FROM_AXES: u32 = 0x0045_0de0;

/// `memcpy` (the CRT import thunk, cdecl).
const MEMCPY: u32 = 0x0094_9256;

/// `PMatrix4` copy assignment (thiscall, the source).
const MATRIX_COPY: u32 = 0x0045_3340;

/// The pass states' debug names and the block's bind.
const RASTER_NAME: u32 = 0x00b3_3654;
const BLEND_NAME: u32 = 0x00b3_3660;
const DEPTH_NAME: u32 = 0x00b3_366c;
const BIND_PASS_STATE: u32 = 0x0058_c6c0;

/// The first word of the 16-byte slot array at `this + 0xc` (count, then
/// the inline slot or the storage pointer); 0 when empty.
fn slots(h: &mut impl Host, this: u32) -> u32 {
    match h.read::<u32>(this + 0xc) & 0x7fff_ffff {
        0 => 0,
        1 => this + 0x10,
        _ => h.read(this + 0x10),
    }
}

/// `getBufferCount` (thiscall), the constant buffers' debug name and the
/// aligned allocation.
const BUFFER_COUNT: u32 = 0x004f_7d50;
const CONSTANT_NAME: u32 = 0x00b1_76c4;
const MALLOC_ALIGNED: u32 = 0x0042_fb80;

/// What the maps return when D3D11 fails.
const FAILED: u32 = 9;
