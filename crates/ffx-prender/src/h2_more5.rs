//! h2-render-b round 4, batch 1: array deleters, destructors and
//! constructors, the D3D11 renderer's shader-stage binds, constant-buffer
//! maps, the ring of constant buffers, the anisotropy setter, the
//! material creators.
//!
//! COM methods (`ID3D11Device`, `ID3D11DeviceContext`, resources) stay
//! calls through the object's vtable (`stdcall`, the object first); `ecx`
//! is passed as the original leaves it, which the methods never read.

use ffx_bridge::x87;
use ffx_graphic::host::{Conv, Host};

use crate::array::{BORROWED, COUNT};
use crate::descriptor::PHYRE_FREE;

/// `memset` (cdecl `(dst, byte, n)`).
const MEMSET: u32 = 0x0094_964c;
/// `memcpy` (cdecl `(dst, src, n)`).
const MEMCPY: u32 = 0x0094_9256;
/// `PhyrePrintf(level, format, ...)`.
const PRINTF: u32 = 0x0043_52f0;
/// `PhyreOS::Sleep(ms)`.
const SLEEP: u32 = 0x0043_0030;
/// The scoped lock's constructor and destructor (thiscall on the lock).
const LOCK: u32 = 0x0046_3d50;
const UNLOCK: u32 = 0x0046_3e90;
/// `PCluster::allocateInstanceList(cluster; 1, type, size)`.
const ALLOCATE_INSTANCE_LIST: u32 = 0x0044_0b70;
/// The render-thread device holder's guard word and the holder.
const HOLDER_GUARD: u32 = 0x00c9_628c;
const HOLDER: u32 = 0x00c9_4f00;
/// The holder's device-context word (`HOLDER + 0x70`).
const HOLDER_CONTEXT: u32 = 0x00c9_4f70;

/// The storage of the `PSharray` whose count word is `n` at `array`:
/// the data word for more than one element, the inline word for one,
/// null for none.
fn storage_of(h: &mut impl Host, array: u32, n: u32) -> u32 {
    match n & COUNT {
        0 => 0,
        1 => array + 4,
        _ => h.read(array + 4),
    }
}

/// [`storage_of`] with the count word read now.
fn storage(h: &mut impl Host, array: u32) -> u32 {
    let n: u32 = h.read(array);
    storage_of(h, array, n)
}

/// A COM method: `slot` (word index) of `obj`'s vtable, `stdcall` with
/// `obj` first, `ecx` as the original leaves it.
fn com(h: &mut impl Host, ecx: u32, obj: u32, slot: u32, args: &[u32]) -> u32 {
    let vt: u32 = h.read(obj);
    let f: u32 = h.read(vt + 4 * slot);
    let mut words = Vec::with_capacity(args.len() + 1);
    words.push(obj);
    words.extend_from_slice(args);
    h.call(f, Conv::Thiscall(ecx), &words) // call width: 8 (seven arguments at most)
}

/// `IUnknown::Release(obj)` with `ecx` as given.
fn release(h: &mut impl Host, ecx: u32, obj: u32) {
    com(h, ecx, obj, 2, &[]);
}

/// The holder of the render thread's device context, built on first use
/// (the guard's bit 0; its destructor registered with `atexit`).
fn holder(h: &mut impl Host) {
    let g = h.rebase(HOLDER_GUARD);
    let v: u32 = h.read(g);
    if v & 1 == 0 {
        h.write(g, v | 1);
        let holder = h.rebase(HOLDER);
        h.call_sym(0x004f_ee70, Conv::Thiscall(holder), &[]);
        let dtor = h.rebase(0x00af_b710);
        h.call_sym(0x0094_9207, Conv::Cdecl, &[dtor]);
    }
}

pub mod bridge {
    //! The originals over game memory.

    use super::{
        ALLOCATE_INSTANCE_LIST, BORROWED, COUNT, Conv, HOLDER_CONTEXT, Host, LOCK, MEMCPY, MEMSET,
        PHYRE_FREE, PRINTF, SLEEP, UNLOCK, com, holder, release, storage, storage_of, x87,
    };

    /// An array deleter `(data, n)` (cdecl): for each of the `n` elements
    /// of `stride` bytes, the member destructors `dtor(element + off)`
    /// (thiscall) in order, then `data` freed; nothing when `data` is
    /// null.
    pub fn delete_array(h: &mut impl Host, data: u32, n: u32, stride: u32, members: &[(u32, u32)]) {
        if data == 0 {
            return;
        }
        let mut e = data;
        for _ in 0..n {
            for &(off, dtor) in members {
                h.call_sym(dtor, Conv::Thiscall(e.wrapping_add(off)), &[]);
            }
            e = e.wrapping_add(stride);
        }
        h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
    }

    /// `0x004dd770(data, n)`: the 0x20-byte elements' four `PString`
    /// words (`+0xc`, `+8`, `+4`, `+0`) freed unless tagged static (bit
    /// 0), then `data` freed; nothing when `data` is null.
    pub fn delete_strings4(h: &mut impl Host, data: u32, n: u32) {
        if data == 0 {
            return;
        }
        let mut e = data;
        for _ in 0..n {
            for off in [0xc, 8, 4, 0] {
                let w: u32 = h.read(e.wrapping_add(off));
                if w & 1 == 0 {
                    h.call_sym(PHYRE_FREE, Conv::Cdecl, &[w]);
                }
            }
            e = e.wrapping_add(0x20);
        }
        h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
    }

    /// The owned `PArray` at `p + off` (count, data) freed when its data
    /// is not null, both words cleared.
    fn free_owned(h: &mut impl Host, p: u32, off: u32) {
        let n: u32 = h.read(p + off);
        if n & BORROWED == 0 {
            let d: u32 = h.read(p + off + 4);
            if d != 0 {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[d]);
            }
        }
        h.write(p + off + 4, 0u32);
        h.write(p + off, 0u32);
    }

    /// `0x0056cab0(p)` (`destructInstance`): the owned array at `+0xc`
    /// freed and cleared, the name at `+0` freed unless tagged static;
    /// `al = 1`.
    pub fn destruct_array_name(h: &mut impl Host, p: u32) -> u8 {
        free_owned(h, p, 0xc);
        let name: u32 = h.read(p);
        if name & 1 == 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[name]);
        }
        1
    }

    /// `0x00562410(p)` (`destructInstance`): `0x0055e060(p + 0x30)`,
    /// `0x00433900(p + 0x28)` (thiscall), the owned array at `+8` freed
    /// and cleared; `al = 1`.
    pub fn destruct_562410(h: &mut impl Host, p: u32) -> u8 {
        h.call_sym(0x0055_e060, Conv::Thiscall(p + 0x30), &[]);
        h.call_sym(0x0043_3900, Conv::Thiscall(p + 0x28), &[]);
        free_owned(h, p, 8);
        1
    }

    /// `0x0058fb90(this; texture, b, update, _)`: 5 for a null texture;
    /// else the texture into the first element of the `PSharray` at
    /// `+0x24`, its `GetDesc` into `+0x44`, `b` at `+0`, the width and
    /// height (`+0x44`/`+0x48`) at `+0x1c`/`+0x20`; with `update` (a
    /// byte) `0x0058f710(this)`'s non-zero result; else 0.
    pub fn set_texture(h: &mut impl Host, this: u32, tex: u32, b: u32, update: u32) -> u32 {
        if tex == 0 {
            return 5;
        }
        let s = storage(h, this + 0x24);
        h.write(s, tex);
        let s = storage(h, this + 0x24);
        let obj: u32 = h.read(s);
        com(h, obj, obj, 10, &[this + 0x44]);
        h.write(this, b);
        let w: u32 = h.read(this + 0x44);
        h.write(this + 0x1c, w);
        let w: u32 = h.read(this + 0x48);
        h.write(this + 0x20, w);
        if update & 0xff != 0 {
            let r = h.call_sym(0x0058_f710, Conv::Thiscall(this), &[]);
            if r != 0 {
                return r;
            }
        }
        0
    }

    /// `0x004ff800(this; node)`: `+0xc = node +4`, `+0x1c = node`; the
    /// two vectors of `node +0x1c` (`+0..+8`, `+0x10..+0x18`, through the
    /// x87) or zeros; `node +0x1c = this`; returns `this`.
    pub fn link_bounds(h: &mut impl Host, this: u32, node: u32) -> u32 {
        let w: u32 = h.read(node + 4);
        h.write(this + 0xc, w);
        h.write(this + 0x1c, node);
        let src: u32 = h.read(node + 0x1c);
        if src == 0 {
            for (i, off) in [0x10, 0, 0x14, 4, 0x18, 8].into_iter().enumerate() {
                h.write(this + off, 0u32);
                if i == 0 {
                    h.write(node + 0x1c, this);
                }
            }
            return this;
        }
        let mut v = [0u32; 6];
        for (k, off) in [0x10, 0x14, 0x18, 0, 4, 8].into_iter().enumerate() {
            let bits: u32 = h.read(src + off);
            v[k] = x87::store(x87::load(bits));
        }
        for (k, off) in [0, 4, 8, 0x10, 0x14, 0x18].into_iter().enumerate() {
            h.write(this + off, v[(k + 3) % 6]);
        }
        h.write(node + 0x1c, this);
        this
    }

    /// `0x004f8460(this; arg)`: unless `0x00595f70(this)` (a byte) says
    /// bound, `0x0058cf20(this)`'s non-zero result; else the next slot
    /// of the ring (`+4` plus one, wrapping at the `PSharray` count at
    /// `+0xc`, at least one), `Sleep(1)` while its 16-byte entry's word
    /// 3 is positive, then `0x00596020(this; slot, arg)`, which on 0 sets
    /// the byte `+8`; returns that result.
    pub fn next_ring_slot(h: &mut impl Host, this: u32, arg: u32) -> u32 {
        if h.call_sym(0x0059_5f70, Conv::Thiscall(this), &[]) as u8 == 0 {
            let r = h.call_sym(0x0058_cf20, Conv::Thiscall(this), &[]);
            if r != 0 {
                return r;
            }
        }
        let next = h.read::<u32>(this + 4).wrapping_add(1);
        let n: u32 = h.read::<u32>(this + 0xc) & COUNT;
        let wrap = if n == 0 { 1 } else { n };
        let slot = if next == wrap { 0 } else { next };
        let s = storage_of(h, this + 0xc, n);
        let entry = s.wrapping_add(slot << 4);
        while h.read::<i32>(entry + 0xc) > 0 {
            h.call_sym(SLEEP, Conv::Cdecl, &[1]);
        }
        let r = h.call_sym(0x0059_6020, Conv::Thiscall(this), &[slot, arg]);
        if r == 0 {
            h.write(this + 8, 1u8);
        }
        r
    }

    /// `0x00596020(this; slot, out)`: 3 for a slot past the ring's
    /// count (at least one), 0x15 when its entry is empty; else a warning
    /// unless bit 16 of `+0x2c` is set, the entry's word 1 into `out`;
    /// 0.
    pub fn ring_entry(h: &mut impl Host, this: u32, slot: u32, out: u32) -> u32 {
        let n = h.read::<u32>(this + 0xc) & COUNT;
        let wrap = if n == 0 { 1 } else { n };
        if slot >= wrap {
            return 3;
        }
        let s = storage_of(h, this + 0xc, n);
        let at = slot.wrapping_mul(16);
        if h.read::<u32>(s.wrapping_add(at)) == 0 {
            return 0x15;
        }
        if h.read::<u32>(this + 0x2c) & 0x1_0000 == 0 {
            let msg = h.rebase(0x00b3_3b50);
            h.call_sym(PRINTF, Conv::Cdecl, &[1, msg]);
        }
        let s = storage(h, this + 0xc);
        let w: u32 = h.read(s.wrapping_add(at).wrapping_add(4));
        h.write(out, w);
        0
    }

    /// `PEffectVariant::createMaterialWithPreallocatedBuffer(this;
    /// cluster, buffer)`: the buffer cleared (`memset`, the half-word
    /// size at `+0x30`), the parameter buffer built in it
    /// (`0x004d6180(buffer; this)`, 0 for a null buffer), then with a
    /// cluster the material `0x004d7b10(cluster; this, params)`'s result;
    /// else 0.
    pub fn create_material_prealloc(h: &mut impl Host, this: u32, cluster: u32, buf: u32) -> u32 {
        let size = u32::from(h.read::<u16>(this + 0x30));
        h.call_sym(MEMSET, Conv::Cdecl, &[buf, 0, size]);
        let params = if buf != 0 {
            h.call_sym(0x004d_6180, Conv::Thiscall(buf), &[this])
        } else {
            0
        };
        if cluster == 0 {
            return 0;
        }
        h.call_sym(0x004d_7b10, Conv::Thiscall(cluster), &[this, params])
    }

    /// `PEffectVariant::createMaterial(this; cluster, out_material,
    /// out_buffer)`: the parameter buffer's instance list from the
    /// cluster (its size the half-word at `+0x30` less 0x10, 0 for 0)
    /// into `out_buffer`; its storage cleared and built
    /// (`0x004d6180`), the material's instance list into `out_material`;
    /// returns the material built by `PMaterial::PMaterial(storage;
    /// this, params)`. Without one, the parameter buffer deleted
    /// (`PCluster::deleteObject`) and 0; 0 without a buffer.
    pub fn create_material(
        h: &mut impl Host,
        this: u32,
        cluster: u32,
        out_mat: u32,
        out_buf: u32,
    ) -> u32 {
        let size = u32::from(h.read::<u16>(this + 0x30));
        let arg = if size == 0 {
            0
        } else {
            size.wrapping_sub(0x10)
        };
        let ty = h.rebase(0x00ca_5040);
        let list = h.call_sym(
            ALLOCATE_INSTANCE_LIST,
            Conv::Thiscall(cluster),
            &[1, ty, arg],
        );
        if out_buf != 0 {
            h.write(out_buf, list);
        }
        if list == 0 {
            return 0;
        }
        let buf: u32 = h.read(list + 0x34);
        if buf == 0 {
            return 0;
        }
        h.call_sym(MEMSET, Conv::Cdecl, &[buf, 0, size]);
        let params = h.call_sym(0x004d_6180, Conv::Thiscall(buf), &[this]);
        let ty = h.rebase(0x00ca_5268);
        let list = h.call_sym(ALLOCATE_INSTANCE_LIST, Conv::Thiscall(cluster), &[1, ty, 0]);
        if out_mat != 0 {
            h.write(out_mat, list);
        }
        if list != 0 {
            let mat: u32 = h.read(list + 0x34);
            if mat != 0 {
                return h.call_sym(0x004d_7b10, Conv::Thiscall(mat), &[this, params]);
            }
        }
        h.call_sym(0x0044_1240, Conv::Thiscall(cluster), &[params]);
        0
    }

    /// `0x00593930(this; context, k)`: the two constant buffers'
    /// uploads. For each of the sizes at `+0x10` and `+0x14` that is not
    /// 0: `Map` (write-discard) of the buffer at `this + 4k` (the second
    /// at `this + 4(k + 2)`) into the frame's mapped subresource, the
    /// bytes of `+0x20` (`+0x24`) copied, `Unmap`; 9 when a map fails or
    /// gives no pointer, else 0. `frame`: the original's `ebp - 0xc`.
    pub fn upload_constants(h: &mut impl Host, this: u32, ctx: u32, k: u32, frame: u32) -> u32 {
        for (size_off, src_off, idx) in [(0x10, 0x20, k), (0x14, 0x24, k.wrapping_add(2))] {
            let size: u32 = h.read(this + size_off);
            if size == 0 {
                continue;
            }
            let slot = this.wrapping_add(idx.wrapping_mul(4));
            let res: u32 = h.read(slot);
            let r = com(h, slot, ctx, 14, &[res, 0, 4, 0, frame]);
            if (r as i32) < 0 {
                return 9;
            }
            let p: u32 = h.read(frame);
            if p == 0 {
                return 9;
            }
            let src: u32 = h.read(this + src_off);
            h.call_sym(MEMCPY, Conv::Cdecl, &[p, src, size]);
            let res: u32 = h.read(slot);
            com(h, slot, ctx, 15, &[res, 0]);
        }
        0
    }

    /// Takes a node from the free list whose head is at `head` (refilled
    /// by `PFreeList::allocateNewBlock(head; [head + 8])` when empty);
    /// null when none.
    fn free_list_take(h: &mut impl Host, head: u32) -> u32 {
        let mut n: u32 = h.read(head);
        if n == 0 {
            let w: u32 = h.read(head + 8);
            h.call_sym(0x0049_c8d0, Conv::Thiscall(head), &[w]);
            n = h.read(head);
            if n == 0 {
                return 0;
            }
        }
        let next: u32 = h.read(n);
        h.write(head, next);
        n
    }

    /// `0x004dd910(name)`: the pass type named `name`
    /// (`0x004dd8c0(name)`); for a missing one a warning, a node from
    /// each of two free lists, the name copied into the second and the
    /// first built as the pass type (`0x004db280(node; 0, name copy)`).
    /// Returns the type found, the new one, or the first node when the
    /// second list gave none.
    pub fn find_or_add_pass_type(h: &mut impl Host, name: u32) -> u32 {
        let r = h.call_sym(0x004d_d8c0, Conv::Cdecl, &[name]);
        if name == 0 || r != 0 {
            return r;
        }
        let msg = h.rebase(0x00b1_fa50);
        h.call_sym(PRINTF, Conv::Cdecl, &[1, msg, name]);
        let node = free_list_take(h, h.rebase(0x00ca_54fc));
        let copy = free_list_take(h, h.rebase(0x00ca_5514));
        if node == 0 || copy == 0 {
            return node;
        }
        let mut len = 0u32;
        while h.read::<u8>(name.wrapping_add(len)) != 0 {
            len += 1;
        }
        h.call_sym(MEMCPY, Conv::Cdecl, &[copy, name, len + 1]);
        h.call_sym(0x004d_b280, Conv::Thiscall(node), &[0, copy])
    }

    /// `0x0059c770(this)`: under the sampler cache's critical section
    /// (`0x00cb3c94`, initialised on first use), the cache entry for
    /// `this` (`0x00581670(0x00c25374; &this)`) removed from the tree and
    /// its node returned to the free list; then the sampler state at
    /// `+0x20` released, bit 7 of `+0x1c` cleared. Returns 0. `frame`:
    /// the original's `ebp - 0x14` (the section pointer, then `this`).
    pub fn release_sampler(h: &mut impl Host, this: u32, frame: u32) -> u32 {
        let cs = h.rebase(0x00cb_3c94);
        if h.read::<u8>(cs) == 0 {
            h.call_sym(0x0043_17f0, Conv::Thiscall(cs), &[]);
        }
        h.write(frame, cs);
        h.call_sym(0x0043_1840, Conv::Thiscall(cs), &[]);
        h.write(frame + 4, this);
        let tree = h.rebase(0x00c2_5374);
        let node = h.call_sym(0x0058_1670, Conv::Thiscall(tree), &[frame + 4]);
        if node != 0 {
            h.call_sym(0x0057_5000, Conv::Thiscall(tree), &[node]);
            let list = h.rebase(0x00c2_538c);
            h.call_sym(0x0049_c980, Conv::Thiscall(list), &[node]);
        }
        h.call_sym(0x0043_19d0, Conv::Thiscall(cs), &[]);
        let state: u32 = h.read(this + 0x20);
        if state != 0 {
            release(h, state, state);
            let f: u32 = h.read(this + 0x1c);
            h.write(this + 0x1c, f & !0x80);
            h.write(this + 0x20, 0u32);
        }
        0
    }

    /// The `PString` at `s` (tagged: bit 0 marks a static string) equals
    /// the NUL-terminated `name`.
    fn same_name(h: &mut impl Host, s: u32, name: u32) -> bool {
        let p = s - (s & 1);
        let mut i = 0u32;
        loop {
            let a: u8 = h.read(name.wrapping_add(i));
            let b: u8 = h.read(p.wrapping_add(i));
            if a != b {
                return false;
            }
            if a == 0 {
                return true;
            }
            i += 1;
        }
    }

    /// `ShaderParams_FindByName(shader; name)`: the first 16-byte
    /// parameter of the array at `+0x20` (count) / `+0x24`, then of the
    /// one at `+0x28` / `+0x2c`, whose name (`+4`) is `name`; 0 for none
    /// or a null name.
    pub fn find_param(h: &mut impl Host, this: u32, name: u32) -> u32 {
        if name == 0 {
            return 0;
        }
        for off in [0x20, 0x28] {
            let n = h.read::<u32>(this + off) & COUNT;
            let mut p: u32 = h.read(this + off + 4);
            for _ in 0..n {
                let s: u32 = h.read(p.wrapping_add(4));
                if same_name(h, s, name) {
                    return p;
                }
                p = p.wrapping_add(0x10);
            }
        }
        0
    }

    /// `0x00593010(this)`: the eight render targets at `+0x228` (16-byte
    /// entries): each target's `0x00591460` update (non-zero: `al = 0`);
    /// a multisampled one (bit 0 of `+8`, byte `+5`) that is not already
    /// resolved (byte `+4`, else `al = 0`) is resolved
    /// (`ResolveSubresource` of `0x00593e80(texture; 0)` from
    /// `0x00594060(texture; 0)`, the format `+0x54`) when its texture has
    /// a view; a target without one gives `al = 0`. `al = 1` after all
    /// eight.
    pub fn resolve_targets(h: &mut impl Host, this: u32) -> u8 {
        for i in 0..8u32 {
            let e: u32 = h.read(this + 0x228 + 0x10 * i);
            if e == 0 {
                continue;
            }
            if h.call_sym(0x0059_1460, Conv::Thiscall(e), &[]) != 0 {
                return 0;
            }
            if h.read::<u8>(e + 8) & 1 == 0 || h.read::<u8>(e + 5) == 0 {
                continue;
            }
            if h.read::<u8>(e + 4) != 0 {
                return 0;
            }
            let tex: u32 = h.read(e);
            if tex == 0 {
                return 0;
            }
            let n: u32 = h.read(tex + 0x24);
            if n & COUNT == 0 {
                return 0;
            }
            let s = storage_of(h, tex + 0x24, n);
            if h.read::<u32>(s + 4) == 0 {
                return 0;
            }
            let format: u32 = h.read(tex + 0x54);
            let ctx: u32 = h.read(this + 0x70);
            let src = h.call_sym(0x0059_4060, Conv::Thiscall(tex), &[0]);
            let dst = h.call_sym(0x0059_3e80, Conv::Thiscall(tex), &[0]);
            com(h, tex, ctx, 57, &[dst, 0, src, 0, format]);
        }
        1
    }

    /// The constant buffer update both shader binds share: with a slot
    /// (`+0x18` not -1), the buffer `+0` (when set) updated from `+8`
    /// (`UpdateSubresource`, pitch the count at `+4`) and recorded in the
    /// slot table at `+0x27c`.
    fn update_slot(h: &mut impl Host, this: u32, dev_ptr: u32) {
        let slot: u32 = h.read(this + 0x18);
        if slot == u32::MAX {
            return;
        }
        let buf: u32 = h.read(this);
        if buf != 0 {
            let dev: u32 = h.read(dev_ptr);
            let ctx: u32 = h.read(dev + 0x70);
            let vt: u32 = h.read(ctx);
            let n = h.read::<u32>(this + 4) & COUNT;
            let data: u32 = h.read(this + 8);
            com(h, vt, ctx, 48, &[buf, 0, 0, data, n, n]);
        }
        let slot: u32 = h.read(this + 0x18);
        let buf: u32 = h.read(this);
        h.write(
            this.wrapping_add(slot.wrapping_mul(4)).wrapping_add(0x27c),
            buf,
        );
    }

    /// `0x005932b0(this; &device, _)`: the pixel shader's bind. The
    /// sampler states (`0x0058dbe0(device; +0x25c, 8 from feature level
    /// 0xb000 up else 0, name)`), the constant buffer slot, then
    /// `PSSetShader` (`+0x4e0`), `PSSetConstantBuffers` (8 from
    /// `+0x27c`), `PSSetShaderResources` (0x80 from `+0x1c`),
    /// `PSSetSamplers` (0x10 from `+0x21c`). `al = 1`.
    pub fn bind_pixel_shader(h: &mut impl Host, this: u32, dev_ptr: u32) -> u8 {
        let dev: u32 = h.read(dev_ptr);
        let n = if h.read::<i32>(dev + 0xb4) < 0xb000 {
            0
        } else {
            8
        };
        let name = h.rebase(0x00b3_39c8);
        h.call_sym(0x0058_dbe0, Conv::Thiscall(dev), &[this + 0x25c, n, name]);
        update_slot(h, this, dev_ptr);
        let ctx: u32 = h.read(dev + 0x70);
        let vt: u32 = h.read(ctx);
        let shader: u32 = h.read(this + 0x4e0);
        com(h, vt, ctx, 9, &[shader, 0, 0]);
        for (slot, n, off) in [(16, 8, 0x27c), (8, 0x80, 0x1c), (10, 0x10, 0x21c)] {
            let ctx: u32 = h.read(dev + 0x70);
            com(h, ctx, ctx, slot, &[0, n, this + off]);
        }
        1
    }

    /// `0x00593510(this; &device, _)`: the vertex shader's bind. No
    /// unordered-access views (`CSSetUnorderedAccessViews` of the frame's
    /// eight null words, 8 from feature level 0xb000 up else 1, the
    /// initial counts at `0x00b3396c`), the constant buffer slot, then
    /// `VSSetShader` (`+0x4ec`), `VSSetConstantBuffers`,
    /// `VSSetShaderResources`, `VSSetSamplers` as the pixel shader's.
    /// `al = 1`. `frame`: the original's `ebp - 0x20`.
    pub fn bind_vertex_shader(h: &mut impl Host, this: u32, dev_ptr: u32, frame: u32) -> u8 {
        let dev: u32 = h.read(dev_ptr);
        for i in 0..8 {
            h.write(frame + 4 * i, 0u32);
        }
        let n = if h.read::<i32>(dev + 0xb4) >= 0xb000 {
            8
        } else {
            1
        };
        let ctx: u32 = h.read(dev + 0x70);
        let counts = h.rebase(0x00b3_396c);
        com(h, ctx, ctx, 68, &[0, n, frame, counts]);
        update_slot(h, this, dev_ptr);
        let ctx: u32 = h.read(dev + 0x70);
        let vt: u32 = h.read(ctx);
        let shader: u32 = h.read(this + 0x4ec);
        com(h, vt, ctx, 11, &[shader, 0, 0]);
        for (slot, n, off) in [(7, 8, 0x27c), (25, 0x80, 0x1c), (26, 0x10, 0x21c)] {
            let ctx: u32 = h.read(dev + 0x70);
            com(h, ctx, ctx, slot, &[0, n, this + off]);
        }
        1
    }

    /// `0x00582f50(this; _)`: two zero words, then eight copies of the
    /// 32-byte default block (`0, 2, 1, 1, 2, 1, 1` and the byte 0xf) at
    /// `+8`, built in the frame; returns `this`. The block's last three
    /// bytes are never written: junk read, `[ebp-0x3..-0x1]` (brief
    /// 2026-10-24): the shim's zeroed frame gives 0. `frame`: the
    /// original's `ebp - 0x20`.
    pub fn default_blocks(h: &mut impl Host, this: u32, frame: u32) -> u32 {
        h.write(this, 0u32);
        h.write(this + 4, 0u32);
        for (i, v) in [0u32, 2, 1, 1, 2, 1, 1].into_iter().enumerate() {
            h.write(frame + 4 * i as u32, v);
        }
        h.write(frame + 0x1c, 0xfu8);
        let mut block = [0u8; 0x20];
        h.read_bytes(frame, &mut block);
        for k in 0..8 {
            h.write_bytes(this + 8 + 0x20 * k, &block);
        }
        this
    }

    /// `Renderer_SetAnisotropy(level)`: the level as a float; when it
    /// differs from the global at `0x00c25370` (unordered counts as
    /// different), stored there and every cached sampler (the tree at
    /// `0x00c25374`, in order) released and rebuilt
    /// (`0x00595bf0(sampler)`). Returns 0.
    pub fn set_anisotropy(h: &mut impl Host, level: u32) -> u32 {
        let v = x87::store(f64::from(level as i32));
        let g = h.rebase(0x00c2_5370);
        let old: u32 = h.read(g);
        // fucomp / `test ah, 0x44` / jnp: only ordered-equal skips
        #[allow(clippy::float_cmp)]
        let same = x87::load(v) == x87::load(old);
        if same {
            return 0;
        }
        h.write(g, v);
        let head = h.rebase(0x00c2_5374);
        let root: u32 = h.read(head + 0xc);
        let mut node = root;
        if root != head && root != 0 {
            let mut n = root;
            while n != head {
                node = n;
                n = h.read(n);
            }
        }
        if node == head {
            return 0;
        }
        let mut next = h.call_sym(0x0057_51d0, Conv::Thiscall(head), &[node]);
        loop {
            let s: u32 = h.read(node + 0xc);
            let state: u32 = h.read(s + 0x20);
            if state != 0 {
                release(h, state, state);
                h.write(s + 0x20, 0u32);
                h.call_sym(0x0059_5bf0, Conv::Thiscall(s), &[]);
            }
            node = next;
            if next != head {
                next = h.call_sym(0x0057_51d0, Conv::Thiscall(head), &[next]);
            }
            if node == head {
                return 0;
            }
        }
    }

    /// `0x00599460(array; n)`: the `PSharray` of 16-byte elements at
    /// `array` resized to `n` (a count word equal to `n` keeps it):
    /// storage allocated for more than one (0xd when that fails), the
    /// kept elements copied (`0x0049f450`), the new ones built
    /// (`0x0058af90`), the old storage freed when owned and apart; 0.
    pub fn resize16(h: &mut impl Host, array: u32, n: u32) -> u32 {
        let word: u32 = h.read(array);
        if n == word {
            return 0;
        }
        let cur = word & COUNT;
        let old = storage_of(h, array, cur);
        let keep = n.min(cur);
        let mut new = array + 4;
        if n > 1 {
            new = h.call_sym(0x0042_fb80, Conv::Cdecl, &[n << 4, 4]);
        }
        if new == 0 {
            if n != 0 {
                return 0xd;
            }
        } else {
            h.call_sym(0x0049_f450, Conv::Cdecl, &[new, old, keep]);
            h.call_sym(
                0x0058_af90,
                Conv::Cdecl,
                &[new.wrapping_add(keep << 4), n - keep],
            );
        }
        if old != new && old != array + 4 && h.read::<u32>(array) & BORROWED == 0 && old != 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
        }
        if new != array + 4 {
            h.write(array + 4, new);
        }
        h.write(array, n);
        0
    }

    /// `0x004db9d0(this)`: the arrays at `+0x1c` (`0x00434360(data,
    /// n)`), `+0x14` (freed) and `+0xc` (`0x004dd770(data, n)`) released
    /// when owned and cleared, then `0x004db710(this + 4)`; returns its
    /// `eax`.
    pub fn destroy_4db9d0(h: &mut impl Host, this: u32) -> u32 {
        let n: u32 = h.read(this + 0x1c);
        if n & BORROWED == 0 {
            let d: u32 = h.read(this + 0x20);
            h.call_sym(0x0043_4360, Conv::Cdecl, &[d, n & COUNT]);
        }
        h.write(this + 0x20, 0u32);
        h.write(this + 0x1c, 0u32);
        free_owned(h, this, 0x14);
        let n: u32 = h.read(this + 0xc);
        if n & BORROWED == 0 {
            let d: u32 = h.read(this + 0x10);
            h.call_sym(0x004d_d770, Conv::Cdecl, &[d, n & COUNT]);
        }
        h.write(this + 0x10, 0u32);
        h.write(this + 0xc, 0u32);
        h.call_sym(0x004d_b710, Conv::Thiscall(this + 4), &[])
    }

    /// `0x00565000(this; i, _)`: when entry `i` (20 bytes) of the
    /// `PSharray` at `+0x14` holds a buffer, under the scoped lock (the
    /// original's argument slot, `lock`, holding `i`) the render thread's
    /// context unmaps it (`Unmap(buffer, 0)`). Returns 0.
    pub fn unmap_entry(h: &mut impl Host, this: u32, i: u32, lock: u32) -> u32 {
        let at = i.wrapping_mul(20);
        let s = storage(h, this + 0x14);
        if h.read::<u32>(s.wrapping_add(at)) == 0 {
            return 0;
        }
        h.write(lock, i);
        h.call_sym(LOCK, Conv::Thiscall(lock), &[]);
        holder(h);
        let s = storage(h, this + 0x14);
        let ctx: u32 = h.read(h.rebase(HOLDER_CONTEXT));
        let buf: u32 = h.read(s.wrapping_add(at));
        com(h, ctx, ctx, 15, &[buf, 0]);
        h.call_sym(UNLOCK, Conv::Thiscall(lock), &[]);
        0
    }

    /// `0x00560e90(out, resource, mode)` (and its copy `0x00560f90`): 1
    /// for a null resource; else `out` cleared and, under the scoped lock
    /// (frame `+0xc`), the resource mapped by the render thread's
    /// context with `mode` (2 becomes 4, write-no-overwrite, for a
    /// dynamic buffer: bit 1 of its `GetDesc` usage, frame `+0x14`); the
    /// mapped pointer (frame `+0`) into `out` and 0, or 9 when the map
    /// fails or gives none. `frame`: the original's `ebp - 0x38`.
    pub fn map_resource(h: &mut impl Host, out: u32, res: u32, mode: u32, frame: u32) -> u32 {
        if res == 0 {
            return 1;
        }
        h.write(out, 0u32);
        let lock = frame + 0xc;
        h.call_sym(LOCK, Conv::Thiscall(lock), &[]);
        holder(h);
        let mut m = mode;
        if m == 2 {
            com(h, frame + 0x10, res, 10, &[frame + 0x10]);
            if h.read::<u8>(frame + 0x14) & 2 != 0 {
                m = 4;
            }
        }
        let ctx: u32 = h.read(h.rebase(HOLDER_CONTEXT));
        let vt: u32 = h.read(ctx);
        let r = com(h, vt, ctx, 14, &[res, 0, m, 0, frame]);
        let result = if (r as i32) >= 0 && h.read::<u32>(frame) != 0 {
            let p: u32 = h.read(frame);
            h.write(out, p);
            0
        } else {
            9
        };
        h.call_sym(UNLOCK, Conv::Thiscall(lock), &[]);
        result
    }

    /// `0x0059ac40(this; state)`: the four shaders of the program
    /// (`+0` vertex, `+4` pixel, `+8` geometry, `+0xc` compute) into the
    /// render state (`+0x10..+0x1c`, the program at `+0xc`), each made
    /// ready (`0x0058d780`/`0x0058d1c0`/`0x0058d350`/`0x0058d030`) and
    /// set on the device context (`VSSetShader`, `GSSetShader`,
    /// `PSSetShader`, `CSSetShader`; null for none). Returns 0.
    pub fn set_program(h: &mut impl Host, this: u32, state: u32) -> u32 {
        let vs: u32 = h.read(this);
        let cs: u32 = h.read(this + 0xc);
        let gs: u32 = h.read(this + 8);
        let ps: u32 = h.read(this + 4);
        h.write(state + 0x10, vs);
        h.write(state + 0x14, ps);
        h.write(state + 0x18, gs);
        h.write(state + 0x1c, cs);
        h.write(state + 0xc, this);
        for (sh, prep, off, slot) in [
            (vs, 0x0058_d780, 0x4ec, 11),
            (gs, 0x0058_d350, 0x4e0, 23),
            (ps, 0x0058_d1c0, 0x4e0, 9),
            (cs, 0x0058_d030, 0x4e0, 69),
        ] {
            let obj = if sh != 0 {
                h.call_sym(prep, Conv::Thiscall(sh), &[]);
                h.read(sh + off)
            } else {
                0
            };
            let dev: u32 = h.read(state);
            let ctx: u32 = h.read(dev + 0x70);
            let vt: u32 = h.read(ctx);
            com(h, vt, ctx, slot, &[obj, 0, 0]);
        }
        0
    }

    /// The constant-table binds `0x0059ab50` / `0x0059b010` `(tables,
    /// program, stage)` (stdcall): for each of the four tables at
    /// `tables + 0x10..+0x1c` and the program's layout (`program +
    /// 0x14`, when both are set) the half-words `first` and `count` of
    /// `stage` at the layout's offsets, and `bind(table; [prefix..],
    /// program, stage, layout [+base] + 16 first, first, count)`.
    /// Returns 0.
    pub fn bind_tables(
        h: &mut impl Host,
        tables: u32,
        program: u32,
        stage: u32,
        bind: u32,
        with_tables: bool,
        offs: &[(u32, u32, u32); 4],
    ) -> u32 {
        for (k, &(first_off, count_off, base_off)) in offs.iter().enumerate() {
            let t: u32 = h.read(tables + 0x10 + 4 * k as u32);
            if t == 0 {
                continue;
            }
            let layout: u32 = h.read(program + 0x14);
            if layout == 0 {
                continue;
            }
            let at = layout.wrapping_add(stage.wrapping_mul(2));
            let first = u32::from(h.read::<u16>(at.wrapping_add(first_off)));
            let count = u32::from(h.read::<u16>(at.wrapping_add(count_off)));
            let layout: u32 = h.read(program + 0x14);
            let base: u32 = h.read(layout + base_off);
            let p = (first << 4).wrapping_add(base);
            if with_tables {
                h.call_sym(
                    bind,
                    Conv::Thiscall(t),
                    &[tables, program, stage, p, first, count],
                );
            } else {
                h.call_sym(bind, Conv::Thiscall(t), &[program, stage, p, first, count]);
            }
        }
        0
    }

    /// `0x00567110(this)`: two words cleared, the members at `+0x24`
    /// (`0x004337d0`) and `+0x94` (`0x00583b20`) built, the rest
    /// zeroed; returns `this`.
    pub fn ctor_567110(h: &mut impl Host, this: u32) -> u32 {
        h.write(this + 0x14, 0u32);
        h.write(this + 0x18, 0u32);
        h.call_sym(0x0043_37d0, Conv::Thiscall(this + 0x24), &[]);
        h.call_sym(0x0058_3b20, Conv::Thiscall(this + 0x94), &[]);
        for off in [0xc4, 0xc8, 0xcc, 0xd0, 0xd4, 0xd8] {
            h.write(this + off, 0u32);
        }
        h.write(this + 4, 0u16);
        for off in [0xc0, 0x20, 0x1c, 0xb8, 0x90, 0, 0xbc, 0xc] {
            h.write(this + off, 0u32);
        }
        h.write(this + 6, 0u8);
        h.write(this + 0x10, 0u32);
        h.write(this + 8, 0u32);
        this
    }

    /// `0x0059c540(this)`: `+0x38` cleared; for each of the
    /// `0x004f7d50(this)` entries (16 bytes) of the `PSharray` at
    /// `+0xc` (the count asked again after each), the object in word 0
    /// released and the buffer in word 1 freed, both cleared. Returns 0.
    pub fn release_entries(h: &mut impl Host, this: u32) -> u32 {
        h.write(this + 0x38, 0u32);
        let arr = this + 0xc;
        let mut n = h.call_sym(0x004f_7d50, Conv::Thiscall(this), &[]);
        let mut i = 0u32;
        let mut off = 0u32;
        while i < n {
            let c: u32 = h.read(arr);
            let s = storage_of(h, arr, c);
            if h.read::<u32>(s.wrapping_add(off)) != 0 {
                let obj: u32 = h.read(s.wrapping_add(off));
                let vt: u32 = h.read(obj);
                release(h, vt, obj);
                let s = storage(h, arr);
                h.write(s.wrapping_add(off), 0u32);
            }
            let c: u32 = h.read(arr);
            let s = storage_of(h, arr, c);
            if h.read::<u32>(s.wrapping_add(off + 4)) != 0 {
                let p: u32 = h.read(s.wrapping_add(off + 4));
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p]);
                let s = storage(h, arr);
                h.write(s.wrapping_add(off + 4), 0u32);
            }
            i += 1;
            off = off.wrapping_add(16);
            n = h.call_sym(0x004f_7d50, Conv::Thiscall(this), &[]);
        }
        0
    }

    /// `0x004e39b0(this)`: the arrays at `+0x28` and `+0x20`
    /// (`0x004ab4d0(data, n)`), `+0x14` (freed, null or not) released
    /// when owned and cleared, `0x004e3830(this + 0xc)`, the array at
    /// `+4` (`0x004e4df0(data, n)`).
    pub fn destroy_4e39b0(h: &mut impl Host, this: u32) {
        for off in [0x28, 0x20] {
            let n: u32 = h.read(this + off);
            if n & BORROWED == 0 {
                let d: u32 = h.read(this + off + 4);
                h.call_sym(0x004a_b4d0, Conv::Cdecl, &[d, n & COUNT]);
            }
            h.write(this + off + 4, 0u32);
            h.write(this + off, 0u32);
        }
        if h.read::<u32>(this + 0x14) & BORROWED == 0 {
            let d: u32 = h.read(this + 0x18);
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[d]);
        }
        h.write(this + 0x18, 0u32);
        h.write(this + 0x14, 0u32);
        h.call_sym(0x004e_3830, Conv::Thiscall(this + 0xc), &[]);
        let n: u32 = h.read(this + 4);
        if n & BORROWED == 0 {
            let d: u32 = h.read(this + 8);
            h.call_sym(0x004e_4df0, Conv::Cdecl, &[d, n & COUNT]);
        }
        h.write(this + 4, 0u32);
        h.write(this + 8, 0u32);
    }

    /// `0x00595620(this; device, size, size2, usage, bind, cpu)`: the two
    /// pairs of buffers (`this + 0/4` of `size`, 0x100000 for 0, and
    /// `this + 8/0xc` of `size2 + 0x10`, 0x40000 for 0) created by
    /// `ID3D11Device::CreateBuffer` when missing, from the descriptions
    /// built in the frame (usage, `bind | 1` / `bind | 2`, the CPU access
    /// word, misc 0x20 with a bind word); the sizes kept at
    /// `+0x18`/`+0x1c`. 9 when a creation fails, else 0. `frame`: the
    /// original's `ebp - 0x34`.
    #[allow(clippy::too_many_arguments)] // the original's
    pub fn create_buffers(
        h: &mut impl Host,
        this: u32,
        dev: u32,
        size: u32,
        size2: u32,
        usage: u32,
        bind: u32,
        cpu: u32,
        frame: u32,
    ) -> u32 {
        let size = if size == 0 { 0x10_0000 } else { size };
        let size2 = if size2 == 0 { 0x4_0000 } else { size2 };
        let d1 = frame + 0x18;
        for i in 0..6 {
            h.write(d1 + 4 * i, 0u32);
        }
        h.write(this + 0x18, size);
        h.write(d1, size);
        h.write(d1 + 8, bind | 1);
        h.write(this + 0x1c, size2);
        h.write(d1 + 4, usage);
        h.write(d1 + 0xc, cpu);
        if bind != 0 {
            h.write(d1 + 0x10, 0x20u32);
        }
        let d2 = frame;
        for i in 0..6 {
            h.write(d2 + 4 * i, 0u32);
        }
        h.write(d2, size2.wrapping_add(0x10));
        h.write(d2 + 8, bind | 2);
        h.write(d2 + 4, usage);
        h.write(d2 + 0xc, cpu);
        if bind != 0 {
            h.write(d2 + 0x10, 0x20u32);
        }
        for k in 0..2 {
            let p = this + 4 * k;
            if h.read::<u32>(p) == 0 {
                let r = com(h, d1, dev, 3, &[d1, 0, p]);
                if (r as i32) < 0 {
                    return 9;
                }
            }
            if h.read::<u32>(p + 8) == 0 {
                let r = com(h, d2, dev, 3, &[d2, 0, p + 8]);
                if (r as i32) < 0 {
                    return 9;
                }
            }
        }
        0
    }

    /// The `PArray` of COM objects at `this + off` (count, data):
    /// each released and nulled, then the storage freed when owned and
    /// not null, both words cleared when the count word is not 0.
    fn release_array(h: &mut impl Host, this: u32, off: u32) {
        let mut i = 0u32;
        while i < h.read::<u32>(this + off) & COUNT {
            let data: u32 = h.read(this + off + 4);
            let obj: u32 = h.read(data.wrapping_add(4 * i));
            if obj != 0 {
                release(h, obj, obj);
                let data: u32 = h.read(this + off + 4);
                h.write(data.wrapping_add(4 * i), 0u32);
            }
            i += 1;
        }
        let n: u32 = h.read(this + off);
        if n != 0 {
            let data: u32 = h.read(this + off + 4);
            if data != 0 && n & BORROWED == 0 {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
            }
            h.write(this + off + 4, 0u32);
            h.write(this + off, 0u32);
        }
    }

    /// `0x0059c650(this)`: the view arrays at `+0x18` and `+0x20` and
    /// the objects at `+0x10`/`+0x14` released; unless the byte `+4` is
    /// set, a texture at `+0` whose `+0x70` is set is unbound
    /// (`PTexture2DD3D11::unbind`), its non-zero result returned. Else 0.
    pub fn release_views2(h: &mut impl Host, this: u32) -> u32 {
        release_array(h, this, 0x18);
        release_array(h, this, 0x20);
        for off in [0x10, 0x14] {
            let obj: u32 = h.read(this + off);
            if obj != 0 {
                release(h, obj, obj);
                h.write(this + off, 0u32);
            }
        }
        if h.read::<u8>(this + 4) == 0 {
            let t: u32 = h.read(this);
            if t != 0 && h.read::<u32>(t + 0x70) != 0 {
                let r = h.call_sym(0x0059_cc40, Conv::Thiscall(t), &[]);
                if r != 0 {
                    return r;
                }
            }
        }
        0
    }
}
