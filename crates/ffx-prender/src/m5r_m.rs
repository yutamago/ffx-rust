//! M5 stage 1, list `render-pcui` (round 3), batch 2: the specular-map
//! parameter manager's event and slot setters, the scene's VFX event and
//! after-post lists, two `PArray` grow-and-push helpers, the field map's
//! object reset walk, a shadow mesh instance factory, the texture
//! animation manager's destructor body, a tree pair's clear, the
//! texture-animation tag packer and the float introsort loop.

use crate::m5r_c::quiet;
use ffx_bridge::Global;

/// `g_PhyreScene`.
pub const SCENE: Global<u32> = Global::new(0x00cc_b9d8);
/// `g_GraphicSkipFrame`: set, the specular setters do nothing.
pub const SKIP_FRAME: Global<u32> = Global::new(0x012f_b7d0);

pub const CURSOR_BEGIN: u32 = 0x0063_7730;
pub const CURSOR_NEXT: u32 = 0x0063_77a0;
/// `g_ClassDesc_00ca7228`: the class the field map's walk selects.
pub const MAP_OBJECT_CLASS: u32 = 0x00ca_7228;

pub const MAGIC_ID: u32 = 0x009d_a3b0;
pub const ADD_VFX_DRAW_EVENT: u32 = 0x0065_6940;
/// The magic files whose specular events are left out.
pub const NO_SPECULAR_MAGIC: [u32; 4] = [0x245, 0x295, 0x289, 0x242];
pub const ADD_SPECULAR_EVENT: u32 = 0x006c_56f0;
pub const SET_SPECULAR_PARAMS: u32 = 0x006c_6390;
pub const SHADER_PARAM_WRITE: u32 = 0x0064_26e0;
/// `"ffxLvMatrix"`, `"ffxLcMatrix"`.
pub const LV_MATRIX: u32 = 0x00b4_1c24;
pub const LC_MATRIX: u32 = 0x00b4_1c30;

pub const RESERVE_16: u32 = 0x0066_8280;
pub const WORD_ARRAY_ALLOC: u32 = 0x0064_f860;
pub const PHYRE_FREE: u32 = 0x0042_fb20;

pub const ALLOCATE_INSTANCE_LIST: u32 = 0x0044_0b70;
/// The instance list class `FUN_00678940` allocates from.
pub const SHADOW_INSTANCE_CLASS: u32 = 0x00ce_82f8;
pub const SHADOW_INSTANCE_CTOR: u32 = 0x006f_9490;

pub const TREE_CLEAR: u32 = 0x0057_4760;

/// `TextureAnimationManager_vftable`.
pub const VT_TEXANIM_MANAGER: u32 = 0x00b4_8438;
pub const DESTROY_OBJECTS_20: u32 = 0x006a_f5d0;
pub const RELEASE_20: u32 = 0x006a_f510;
pub const SAMPLER_DTOR: u32 = 0x0058_5230;

pub const TEXANIM_SET_QWORD: u32 = 0x0077_ebc0;

pub const FLOAT_PARTITION: u32 = 0x006c_7900;
pub const FLOAT_INTROSORT: u32 = 0x006c_77a0;
pub const FLOAT_INSERTION: u32 = 0x006c_7320;
pub const FLOAT_MAKE_HEAP: u32 = 0x006c_7410;
pub const FLOAT_SORT_HEAP: u32 = 0x006c_7890;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use crate::h3_glue14::bridge::{cursor_over, walk};
    use ffx_bridge::Mem;
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, f: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Cdecl, args)
    }

    fn word(h: &impl Mem, at: u32) -> u32 {
        h.read(at)
    }

    /// `FUN_0065f0e0(scene)` (frame from `ebp - 0x24`: the cursor): with
    /// the field map flag (`+0x93c`) and a map (`+0x1128`), each of the
    /// `+0x112c` objects of `+0x1134` has its objects of class
    /// `0x00ca7228` walked (cursor over the object's list `+0x1c`), and
    /// each one's object (`[offset + element + 0x1c]`, when set) gets
    /// `+0x1c` cleared.
    pub fn map_objects_reset(h: &mut impl Host, ebp: u32, scene: u32) {
        if h.read::<u8>(scene + 0x93c) == 0 || word(h, scene + 0x1128) == 0 {
            return;
        }
        let mut i = 0u32;
        while i < word(h, scene + 0x112c) {
            let objects = word(h, scene + 0x1134);
            let object = word(h, objects.wrapping_add(4 * i));
            let cursor = ebp - 0x24;
            cursor_over(h, cursor, object, MAP_OBJECT_CLASS);
            h.call_sym(CURSOR_BEGIN, Conv::Thiscall(cursor), &[]);
            walk(h, cursor, CURSOR_NEXT, |h, base| {
                let offset = word(h, cursor + 0x10);
                let o = word(h, offset.wrapping_add(base).wrapping_add(0x1c));
                if o != 0 {
                    h.write(o.wrapping_add(0x1c), 0_u32);
                }
            });
            i += 1;
        }
    }

    /// `SpecularMapParamsManager_AddSpecularEvent(x)` (stdcall; the
    /// event at `ev`, the original's `ebp - 0x14`): unless the current
    /// magic file is 0x245, 0x295, 0x289 or 0x242 (`MagicFile_GetMagicId`
    /// asked each time), a type-4 event with `x` at `+8` goes to the
    /// scene's `addVFXDrawEvent`; -> the last callee's `eax`.
    pub fn add_specular_event(h: &mut impl Host, ev: u32, x: u32) -> u32 {
        for id in NO_SPECULAR_MAGIC {
            let m = cdecl(h, MAGIC_ID, &[]);
            if m == id {
                return m;
            }
        }
        let scene = SCENE.get(h);
        h.write(ev + 8, x);
        h.write(ev, 4_u32);
        // junk read: [ebp-0x10] and [ebp-0x8] never written; 0 (brief 2026-10-24)
        h.write(ev + 4, 0_u32);
        h.write(ev + 0xc, 0_u32);
        h.call_sym(ADD_VFX_DRAW_EVENT, Conv::Thiscall(scene), &[ev])
    }

    /// The first free one of the manager's four slots (`+0`), or `None`.
    fn free_slot(h: &impl Mem, this: u32) -> Option<u32> {
        (0..4).find(|&k| word(h, this + 4 * k) == 0)
    }

    /// The slot record at `this + 0x10 + 0x164 k`: a 4x4 block of 0x40
    /// words from `a`, 0x10 from `b`, four floats (`fld`/`fstp`), the
    /// four words at `v` and `last` at `+0x160`.
    #[allow(clippy::too_many_arguments)] // the record's parts
    fn fill_slot(
        h: &mut impl Host,
        this: u32,
        k: u32,
        a: u32,
        b: u32,
        f: [u32; 4],
        v: u32,
        last: u32,
    ) {
        let at = this + k * 0x164;
        for w in 0..0x40 {
            let x = word(h, a.wrapping_add(4 * w));
            h.write(at + 0x10 + 4 * w, x);
        }
        for w in 0..0x10 {
            let x = word(h, b.wrapping_add(4 * w));
            h.write(at + 0x110 + 4 * w, x);
        }
        for (w, x) in (0u32..).zip(f) {
            h.write(at + 0x150 + 4 * w, quiet(x));
        }
        for w in 0..4 {
            let x = word(h, v.wrapping_add(4 * w));
            h.write(at + 0x160 + 4 * w, x);
        }
        h.write(at + 0x170, last);
    }

    /// `FUN_006c5b80(this; id, a, b, f0..f3, v, kind)` (`ret 0x24`): unless
    /// the frame is skipped, `id` takes the first free slot (state 4 for
    /// kind 0x10, else 3), its specular event is added, and the slot's
    /// record is filled.
    #[allow(clippy::too_many_arguments)] // the original's ten
    pub fn set_akaza_no_light(
        h: &mut impl Host,
        this: u32,
        id: u32,
        a: u32,
        b: u32,
        f: [u32; 4],
        v: u32,
        kind: u32,
    ) {
        if SKIP_FRAME.get(h) != 0 {
            return;
        }
        let slot = free_slot(h, this);
        if let Some(k) = slot {
            let state = if kind == 0x10 { 4_u32 } else { 3 };
            h.write(this + 0x5a0 + 4 * k, state);
            h.write(this + 4 * k, id);
        }
        h.call_sym(ADD_SPECULAR_EVENT, Conv::Stdcall, &[id]);
        if let Some(k) = slot {
            fill_slot(h, this, k, a, b, f, v, kind);
        }
    }

    /// `FUN_006c5c90(this; id, a, b, f0..f3, v, last, lv, lc, event)`
    /// (`ret 0x30`): unless the frame is skipped, `id` takes the first
    /// free slot (state 5), its specular event is added when `event` is
    /// set, and with a slot the record is filled and the shader
    /// parameters `ffxLvMatrix` (`lv`) and `ffxLcMatrix` (`lc`) written.
    #[allow(clippy::too_many_arguments)] // the original's thirteen
    pub fn set_akaza(
        h: &mut impl Host,
        this: u32,
        id: u32,
        a: u32,
        b: u32,
        f: [u32; 4],
        v: u32,
        last: u32,
        lv: u32,
        lc: u32,
        event: u32,
    ) {
        if SKIP_FRAME.get(h) != 0 {
            return;
        }
        let slot = free_slot(h, this);
        if let Some(k) = slot {
            h.write(this + 4 * k, id);
            h.write(this + 0x5a0 + 4 * k, 5_u32);
        }
        if event != 0 {
            h.call_sym(ADD_SPECULAR_EVENT, Conv::Stdcall, &[id]);
        }
        if let Some(k) = slot {
            fill_slot(h, this, k, a, b, f, v, last);
            let name = h.rebase(LV_MATRIX);
            cdecl(h, SHADER_PARAM_WRITE, &[id, name, lv]);
            let name = h.rebase(LC_MATRIX);
            cdecl(h, SHADER_PARAM_WRITE, &[id, name, lc]);
        }
    }

    /// `FUN_006c5e30(this; id, w0, f0, f1, w1, f2, w2, f3, f4, f5)` (`ret
    /// 0x28`): unless the frame is skipped, an `id` not yet in the four
    /// slots takes the first free one (its state `+0x5a0` cleared) and
    /// gets its specular event; then `FUN_006c6390` with the same words
    /// (the floats through the x87).
    pub fn set_no_light(h: &mut impl Host, this: u32, w: [u32; 10]) {
        let id = w[0];
        if SKIP_FRAME.get(h) != 0 {
            return;
        }
        if !(0..4).any(|k| word(h, this + 4 * k) == id) {
            if let Some(k) = free_slot(h, this) {
                h.write(this + 4 * k, id);
                h.write(this + 0x5a0 + 4 * k, 0_u32);
            }
            h.call_sym(ADD_SPECULAR_EVENT, Conv::Stdcall, &[id]);
        }
        let mut args = w;
        for (k, a) in args.iter_mut().enumerate() {
            // `id` and the words at [ebp+0xc], +0x18 and +0x20 pass as they are
            if !matches!(k, 0 | 1 | 4 | 6) {
                *a = quiet(*a);
            }
        }
        h.call_sym(SET_SPECULAR_PARAMS, Conv::Thiscall(this), &args);
    }

    /// `FUN_006568a0(scene; b0, b1, x, h)` (`ret 0x10`): a copy-buffer
    /// event (type 1, the order key `+0xf8cc` plus the current layer's
    /// offset when the layer `+0xf954` is above 0, the word `b0 | b1 << 8
    /// | h << 16`, `x`) pushed onto the event array `+0xfb54` (grown to
    /// `n + n / 8 + 2` when the count passes the capacity), counted in
    /// `+0xfb64`. `eax`: `x`, or the slot address when it is 0.
    pub fn add_copy_buffer_event(
        h: &mut impl Host,
        this: u32,
        b0: u32,
        b1: u32,
        x: u32,
        hw: u32,
    ) -> u32 {
        let mut key = word(h, this + 0xf8cc);
        let layer = word(h, this + 0xf954) as i32;
        if layer > 0 {
            key = key.wrapping_add(word(
                h,
                this.wrapping_add((layer as u32).wrapping_mul(4)) + 0xf910,
            ));
        }
        let arr = this + 0xfb54;
        let n = word(h, arr).wrapping_add(1);
        h.write(arr, n);
        let packed = (b0 & 0xff) | (b1 & 0xff) << 8 | (hw & 0xffff) << 16;
        if (n as i32) > (word(h, arr + 4) as i32) {
            let m = n as i32;
            let grow = (m / 8).wrapping_add(m.wrapping_add(2));
            h.call_sym(RESERVE_16, Conv::Thiscall(arr), &[grow as u32]);
        }
        let n = word(h, arr);
        let data = word(h, arr + 8).wrapping_sub(0x10);
        let e = n.wrapping_shl(4).wrapping_add(data);
        let ret = if e == 0 {
            data
        } else {
            h.write(e, 1_u32);
            h.write(e + 4, key);
            h.write(e + 8, packed);
            h.write(e + 0xc, x);
            x
        };
        let c = word(h, this + 0xfb64);
        h.write(this + 0xfb64, c.wrapping_add(1));
        ret
    }

    /// `PhyreScene_AddVFXToAfterPostList(scene; vfx)` (`ret 4`): a `vfx`
    /// not yet in the after-post list (`+0x3884` capacity with its owner
    /// bit, `+0x3888` data, `+0x388c` count) is appended; when the count
    /// reaches the capacity it grows by 0x400 (`WordArray_alloc_0064f860`,
    /// the old block freed when owned and moved).
    pub fn add_vfx_after_post(h: &mut impl Host, this: u32, vfx: u32) {
        let count = word(h, this + 0x388c);
        let data = word(h, this + 0x3888);
        if (count as i32) > 0 && (0..count).any(|k| word(h, data.wrapping_add(4 * k)) == vfx) {
            return;
        }
        h.write(data.wrapping_add(count.wrapping_mul(4)), vfx);
        let raw = word(h, this + 0x3884);
        let n = word(h, this + 0x388c).wrapping_add(1);
        h.write(this + 0x388c, n);
        let cap = raw & 0x7fff_ffff;
        if n < cap {
            return;
        }
        let new = cap + 0x400;
        if new == raw {
            return;
        }
        let keep = if new < cap { new } else { cap };
        let old = word(h, this + 0x3888);
        let block = cdecl(h, WORD_ARRAY_ALLOC, &[new, old, keep]);
        if block == 0 && new != 0 {
            return;
        }
        let old = word(h, this + 0x3888);
        if old != block && word(h, this + 0x3884) >> 31 == 0 {
            cdecl(h, PHYRE_FREE, &[old]);
        }
        h.write(this + 0x3888, block);
        h.write(this + 0x3884, new);
    }

    /// The `PArray` push into a new block: the `count` (`+0` without its
    /// owner bit) elements of `words` words at `+4` copied into `block`
    /// (an element whose new address is 0 is skipped and the source
    /// stays), `item` appended (unless its address is 0), the old block
    /// freed when it moves and is owned (and, with `null_check`, set),
    /// `+0` the count + 1, `+4` the block. 0, or 0xd without a block.
    fn push_into(
        h: &mut impl Host,
        this: u32,
        block: u32,
        item: u32,
        words: u32,
        null_check: bool,
    ) -> u32 {
        if block == 0 {
            return 0xd;
        }
        let size = words * 4;
        let mut src = word(h, this + 4);
        let n = word(h, this) & 0x7fff_ffff;
        let mut dst = block;
        for _ in 0..n {
            if dst != 0 {
                for w in 0..words {
                    let x = word(h, src.wrapping_add(4 * w));
                    h.write(dst.wrapping_add(4 * w), x);
                }
                src = src.wrapping_add(size);
            }
            dst = dst.wrapping_add(size);
        }
        let end = block.wrapping_add(n.wrapping_mul(size));
        if end != 0 {
            for w in 0..words {
                let x = word(h, item.wrapping_add(4 * w));
                h.write(end.wrapping_add(4 * w), x);
            }
        }
        let old = word(h, this + 4);
        if old != block && word(h, this) >> 31 == 0 && (!null_check || old != 0) {
            cdecl(h, PHYRE_FREE, &[old]);
        }
        h.write(this, n.wrapping_add(1));
        h.write(this + 4, block);
        0
    }

    /// `FUN_0065fbe0(array; block, item)` (`ret 8`): [`push_into`] for
    /// 0x44-byte elements (the mesh matrix list).
    pub fn push_into_44(h: &mut impl Host, this: u32, block: u32, item: u32) -> u32 {
        push_into(h, this, block, item, 0x11, true)
    }

    /// `FUN_006add90(array; block, item)` (`ret 8`): [`push_into`] for
    /// words; the old block is freed even when 0.
    pub fn push_into_4(h: &mut impl Host, this: u32, block: u32, item: u32) -> u32 {
        push_into(h, this, block, item, 1, false)
    }

    /// `FUN_00678940(this; cluster, existing)` (`ret 8`; `slots`: the
    /// argument slots, both of which it overwrites with the new
    /// instance): `existing` when set; else an instance list of the
    /// shadow mesh class allocated in `cluster`, and when it has an
    /// instance (`+0x34`) that instance constructed for `this` (its
    /// `eax`); 0 otherwise.
    pub fn shadow_instance(
        h: &mut impl Host,
        slots: u32,
        this: u32,
        cluster: u32,
        existing: u32,
    ) -> u32 {
        if existing != 0 {
            return existing;
        }
        let class = h.rebase(SHADOW_INSTANCE_CLASS);
        let list = h.call_sym(
            ALLOCATE_INSTANCE_LIST,
            Conv::Thiscall(cluster),
            &[1, class, 0],
        );
        if list == 0 {
            return 0;
        }
        let inst = word(h, list.wrapping_add(0x34));
        h.write(slots + 4, inst);
        if inst == 0 {
            return 0;
        }
        h.write(slots, inst);
        h.call_sym(SHADOW_INSTANCE_CTOR, Conv::Thiscall(inst), &[this])
    }

    /// `FUN_006df080(this)`: `Phyre_PRedBlackTree_clear(this)` twice ->
    /// the second's `eax`.
    pub fn tree_clear_twice(h: &mut impl Host, this: u32) -> u32 {
        h.call_sym(TREE_CLEAR, Conv::Thiscall(this), &[]);
        h.call_sym(TREE_CLEAR, Conv::Thiscall(this), &[])
    }

    /// The owner-bit release of the `PArray` at `this + off` (count,
    /// data): the data freed when owned (and, with `null_check`, set),
    /// both words cleared.
    fn release(h: &mut impl Host, this: u32, off: u32, null_check: bool) {
        let data = word(h, this + off + 4);
        if word(h, this + off) >> 31 == 0 && (!null_check || data != 0) {
            cdecl(h, PHYRE_FREE, &[data]);
        }
        h.write(this + off + 4, 0_u32);
        h.write(this + off, 0_u32);
    }

    /// `FUN_006af8a0(manager)`: the texture animation manager's body
    /// destroyed: its vtable, the objects at `+0x7c`, the arrays `+0x74`,
    /// `+0x6c` (data checked), `+0x64` (`PArray_release20`), `+0x5c`, and
    /// the two sampler states `+0x38` and `+0x14` -> the last `eax`.
    pub fn texanim_manager_dtor(h: &mut impl Host, this: u32) -> u32 {
        let vt = h.rebase(VT_TEXANIM_MANAGER);
        h.write(this, vt);
        h.call_sym(DESTROY_OBJECTS_20, Conv::Thiscall(this + 0x7c), &[]);
        release(h, this, 0x74, false);
        release(h, this, 0x6c, true);
        h.call_sym(RELEASE_20, Conv::Thiscall(this + 0x64), &[]);
        release(h, this, 0x5c, false);
        h.call_sym(SAMPLER_DTOR, Conv::Thiscall(this + 0x38), &[]);
        h.call_sym(SAMPLER_DTOR, Conv::Thiscall(this + 0x14), &[])
    }

    /// `FUN_0077eb00(q, a, b, c, flag)`: `TexAnim_SetQword(q, 0x50, lo,
    /// hi)` of the tag `((c << 8 | b) << 16 | a)` (64-bit shifts): with
    /// `flag` 0 that shifted on as `((v << 8 | c) << 24)` (`lo`, `hi` its
    /// halves), else `lo` 0 and `hi` the tag's low word -> its `eax`.
    pub fn set_tag(h: &mut impl Host, q: u32, a: u32, b: u32, c: u32, flag: u32) -> u32 {
        let v = ((u64::from(c) << 8 | u64::from(b)) << 16) | u64::from(a);
        let (lo, hi) = if flag == 0 {
            let v = ((v << 8) | u64::from(c)) << 24;
            (v as u32, (v >> 32) as u32)
        } else {
            (0, v as u32)
        };
        cdecl(h, TEXANIM_SET_QWORD, &[q, 0x50, lo, hi])
    }

    /// `FUN_006c77a0(first, last, depth, less)` (cdecl; `slots`: the
    /// argument slots, `first`'s and `last`'s get each partition's
    /// bounds; `out`: the pair `FUN_006c7900` writes, the original's
    /// `ebp - 8`): the introsort loop over floats. While more than 32
    /// remain: with depth left, partition, recurse into the smaller side
    /// (depth `d / 2 + d / 4`) and loop on the other; without, heap sort
    /// (make heap only past one element). Then the insertion sort for
    /// more than one.
    pub fn introsort(
        h: &mut impl Host,
        slots: u32,
        out: u32,
        first: u32,
        last: u32,
        depth: u32,
        less: u32,
    ) {
        let (mut first, mut last, mut depth) = (first, last, depth as i32);
        let mut n = (last.wrapping_sub(first) as i32) >> 2;
        while n > 0x20 {
            if depth <= 0 {
                if (last.wrapping_sub(first) & !3) as i32 > 4 {
                    cdecl(h, FLOAT_MAKE_HEAP, &[first, last, less, 0, 0]);
                }
                cdecl(h, FLOAT_SORT_HEAP, &[first, last, less]);
                return;
            }
            cdecl(h, FLOAT_PARTITION, &[out, first, last, less]);
            let lo = word(h, out);
            let hi = word(h, out + 4);
            h.write(slots + 4, hi);
            h.write(slots, lo);
            let half = depth / 2;
            depth = half + half / 2;
            if ((lo.wrapping_sub(first) & !3) as i32) < (last.wrapping_sub(hi) & !3) as i32 {
                cdecl(h, FLOAT_INTROSORT, &[first, lo, depth as u32, less]);
                first = hi;
            } else {
                cdecl(h, FLOAT_INTROSORT, &[hi, last, depth as u32, less]);
                last = lo;
            }
            n = (last.wrapping_sub(first) as i32) >> 2;
        }
        if n > 1 {
            cdecl(h, FLOAT_INSERTION, &[first, last, less, 0]);
        }
    }
}
