//! M5 stage 1, list `render-pcui` (round 3), batch 4: the pc-ui side's
//! array releases, `GDraw` resource teardowns and keyed callbacks, the
//! Iggy player's render-thread callbacks, the page heap's lock sweeps,
//! bucket setup and hash lookup, a video plane copy and two Iggy value
//! readers.

/// The originals over game memory.
#[allow(
    clippy::many_single_char_names, // the originals' argument words
    clippy::cast_possible_wrap,     // the originals' signed compares and idiv
    clippy::cast_sign_loss
)]
pub mod bridge {
    use ffx_bridge::Mem;
    use ffx_graphic::host::{Conv, Host};

    pub const PHYRE_FREE: u32 = 0x0042_fb20;
    /// `g_GDraw` (a pointer): its pool at `+0x290`, two objects at
    /// `+0x788`, `+0x794`, the slot table `+0x800..+0x980`.
    pub const GDRAW: u32 = 0x01a8_58b4;
    pub const GDRAW_RECT: u32 = 0x00a3_6090;
    pub const GDRAW_POOL_FREE_ALL: u32 = 0x00a3_b380;
    pub const GDRAW_CALLBACK: u32 = 0x00a3_b480;
    pub const NODE_REQUEUE: u32 = 0x00a3_9f00;
    pub const QUEUE_DATA_CALLBACK: u32 = 0x005b_5940;
    pub const SYNC_RENDER: u32 = 0x005b_c3f0;
    /// The render-thread callbacks: `Callback_invoke_00a3d6d0` and the
    /// pair's functions (`IggyPlayerHolder_destroy_00a3d780`, the draw
    /// `0x00a3d9f0`).
    pub const CALLBACK_INVOKE: u32 = 0x00a3_d6d0;
    pub const HOLDER_DESTROY: u32 = 0x00a3_d780;
    pub const HOLDER_DRAW: u32 = 0x00a3_d9f0;
    pub const PAGE_BUCKET_FILL: u32 = 0x0094_6b20;
    pub const DECOMMIT: u32 = 0x0094_38e0;
    pub const RELEASE: u32 = 0x0094_3940;
    pub const PAGE_BITS_INIT: u32 = 0x0094_8750;
    pub const MEMSET_THUNK: u32 = 0x0094_964c;
    pub const MEMCPY_THUNK: u32 = 0x0094_9256;
    /// `KERNEL32!EnterCriticalSection`, `LeaveCriticalSection`.
    pub const ENTER_CRITICAL_SECTION_IAT: u32 = 0x00b0_c154;
    pub const LEAVE_CRITICAL_SECTION_IAT: u32 = 0x00b0_c158;
    pub const GET_CLASS_DESCRIPTOR: u32 = 0x0043_5740;
    /// The class descriptor `0x00a258c0` compares with (or 1).
    pub const VALUE_CLASS: u32 = 0x00c9_0760;

    fn word(h: &impl Mem, at: u32) -> u32 {
        h.read(at)
    }

    fn cdecl(h: &mut impl Host, f: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Cdecl, args)
    }

    /// A COM-style release: the object's method `+8`, stdcall with the
    /// object first.
    fn release_object(h: &mut impl Host, obj: u32) -> u32 {
        let vt = word(h, obj);
        let f = word(h, vt.wrapping_add(8));
        h.call(f, Conv::Stdcall, &[obj])
    }

    /// `PArray_release_00a25e50(array)` (and with `null_check`,
    /// `0x00a25e80`, `0x00a3d3d0`): the data `+4` freed when the owner bit
    /// of `+0` is clear (and the data set), both words cleared.
    pub fn array_release(h: &mut impl Host, this: u32, null_check: bool) {
        let data = word(h, this + 4);
        if word(h, this) >> 31 == 0 && (!null_check || data != 0) {
            cdecl(h, PHYRE_FREE, &[data]);
        }
        h.write(this, 0_u32);
        h.write(this + 4, 0_u32);
    }

    /// `GDraw_rectCall_00a36040(rect, a, b)` (`slots`: the argument
    /// slots; the original converts each of the rectangle's four integers
    /// through `rect`'s slot, which keeps the first): `0x00a36090(a, b,
    /// x0, y0, x1, y1)` with the integers as floats -> its `eax`.
    pub fn rect_call(h: &mut impl Host, slots: u32, rect: u32, a: u32, b: u32) -> u32 {
        let mut f = [0; 4];
        for k in (0..4).rev() {
            #[allow(clippy::cast_precision_loss)] // fild, fstp dword: one rounding
            let x = (word(h, rect.wrapping_add(4 * k)) as i32 as f32).to_bits();
            h.write(slots, x);
            f[k as usize] = x;
        }
        cdecl(h, GDRAW_RECT, &[a, b, f[0], f[1], f[2], f[3]])
    }

    /// Queues `Callback_invoke_00a3d6d0` with an 8-byte block on the
    /// renderer (`local`: the original's `ebp - 4`, where the block's
    /// address comes back) and, when it was queued, fills the block with
    /// `(this, f)`. -> the queue's result.
    fn queue(h: &mut impl Host, local: u32, this: u32, renderer: u32, f: u32) -> u32 {
        h.write(local, 0_u32);
        let invoke = h.rebase(CALLBACK_INVOKE);
        let r = h.call_sym(
            QUEUE_DATA_CALLBACK,
            Conv::Thiscall(renderer),
            &[invoke, 8, local],
        );
        if r == 0 {
            let block = word(h, local);
            h.write(block, this);
            let f = h.rebase(f);
            h.write(block.wrapping_add(4), f);
        }
        r
    }

    /// `IggyPlayerHolder_queueDestroy_00a3d720(this; renderer)` (`ret 4`):
    /// the holder's destroy queued on the render thread, then the
    /// renderer's method `+8` and, when that gives 0, `syncRender`: the
    /// first nonzero of the two, else the queue's result.
    pub fn queue_destroy(h: &mut impl Host, local: u32, this: u32, renderer: u32) -> u32 {
        let queued = queue(h, local, this, renderer, HOLDER_DESTROY);
        let vt = word(h, renderer);
        let flush = word(h, vt.wrapping_add(8));
        let flushed = h.call(flush, Conv::Thiscall(renderer), &[]);
        if flushed != 0 {
            return flushed;
        }
        let synced = h.call_sym(SYNC_RENDER, Conv::Thiscall(renderer), &[]);
        if synced != 0 {
            return synced;
        }
        queued
    }

    /// `IggyPlayer_QueueDraw_00a3d9b0(this; renderer)` (`ret 4`): the
    /// holder's draw queued on the render thread -> 0, or the queue's
    /// error.
    pub fn queue_draw(h: &mut impl Host, local: u32, this: u32, renderer: u32) -> u32 {
        queue(h, local, this, renderer, HOLDER_DRAW)
    }

    /// `GDraw_releaseIf_00a398a0(node, key, x)` (stdcall): when key is 0
    /// or the node's `+0x14`, a node of the pool (`+0x18` the pool at
    /// `g_GDraw + 0x290`) is requeued, any other gets `0x00a3b480(node,
    /// x)` -> its `eax`; else `key`.
    pub fn release_if(h: &mut impl Host, node: u32, key: u32, x: u32) -> u32 {
        if word(h, node.wrapping_add(0x14)) != key && key != 0 {
            return key;
        }
        let g = word(h, h.rebase(GDRAW));
        if word(h, node.wrapping_add(0x18)) == g.wrapping_add(0x290) {
            return cdecl(h, NODE_REQUEUE, &[node]);
        }
        cdecl(h, GDRAW_CALLBACK, &[node, x])
    }

    /// `PageHeap_fillBuckets_00946100(heap)`: `0x00946b20(heap + 0x4ec;
    /// p, 0x10)` for each `p` from `[+8] + 0x10` up to `[+0x14]` (signed)
    /// in steps of 0x10.
    pub fn fill_buckets(h: &mut impl Host, this: u32) {
        let mut p = word(h, this + 8).wrapping_add(0x10);
        while (p as i32) <= (word(h, this + 0x14) as i32) {
            h.call_sym(PAGE_BUCKET_FILL, Conv::Thiscall(this + 0x4ec), &[p, 0x10]);
            p = p.wrapping_add(0x10);
        }
    }

    /// `PageList_free_009470f0(list)`: each page of the chain at `+4`
    /// (next at `+0`) decommitted and released (0x1000 bytes).
    pub fn free_pages(h: &mut impl Host, this: u32) {
        let mut page = word(h, this + 4);
        while page != 0 {
            let next = word(h, page);
            cdecl(h, DECOMMIT, &[page, 0x1000]);
            cdecl(h, RELEASE, &[page, 0x1000]);
            page = next;
        }
    }

    /// `PageBits_ctor_00946520(this; owner, n)` (`ret 8`): `+0` owner,
    /// `+0x404` n, the bit set `+0x408` initialised (`0x00948750(0x400,
    /// n)`), the 0x400 bytes at `+4` cleared -> `this`.
    pub fn page_bits_ctor(h: &mut impl Host, this: u32, owner: u32, n: u32) -> u32 {
        h.write(this, owner);
        h.write(this + 0x404, n);
        h.call_sym(PAGE_BITS_INIT, Conv::Thiscall(this + 0x408), &[0x400, n]);
        cdecl(h, MEMSET_THUNK, &[this + 4, 0, 0x400]);
        this
    }

    /// `PVideoPlaybackWin32_CopyPlane_00a26fe0(dst, dpitch, src, spitch,
    /// w, rows)`: nothing without rows or width; one `memcpy` of
    /// `spitch * rows` when the pitches match, else `w` bytes per row ->
    /// the last `memcpy`'s `eax` (0 when none ran).
    pub fn copy_plane(
        h: &mut impl Host,
        dst: u32,
        dpitch: u32,
        src: u32,
        spitch: u32,
        w: u32,
        rows: u32,
    ) -> u32 {
        if rows == 0 || w == 0 {
            return 0;
        }
        if spitch == dpitch {
            return cdecl(h, MEMCPY_THUNK, &[dst, src, spitch.wrapping_mul(rows)]);
        }
        let (mut d, mut s, mut r) = (dst, src, 0);
        for _ in 0..rows {
            r = cdecl(h, MEMCPY_THUNK, &[d, s, w]);
            d = d.wrapping_add(dpitch);
            s = s.wrapping_add(spitch);
        }
        r
    }

    /// `GDraw_freeResources_00a36300(stats)`: the pool freed
    /// (`GDraw_pool_freeAll_00a3b380(g_GDraw + 0x290, stats)`), then the
    /// objects `+0x788` and `+0x794` released and cleared (`g_GDraw` read
    /// again after the pool and after the first release).
    pub fn free_resources(h: &mut impl Host, stats: u32) {
        let at = h.rebase(GDRAW);
        let g = word(h, at);
        cdecl(h, GDRAW_POOL_FREE_ALL, &[g.wrapping_add(0x290), stats]);
        let mut g = word(h, at);
        let o = word(h, g.wrapping_add(0x788));
        if o != 0 {
            release_object(h, o);
            h.write(g.wrapping_add(0x788), 0_u32);
            g = word(h, at);
        }
        let o = word(h, g.wrapping_add(0x794));
        if o != 0 {
            release_object(h, o);
            h.write(g.wrapping_add(0x794), 0_u32);
        }
    }

    /// `GDraw_releaseSlots_00a3cfc0()`: for each of the 32 slots `+0x880 +
    /// 4 k`: `+0x800 + 4 k` cleared, the object released and cleared,
    /// `+0x900 + 4 k` cleared; then `+0x980`. `g_GDraw` is read again
    /// after each release (the slot itself is cleared at the old base).
    pub fn release_slots(h: &mut impl Host) {
        let at = h.rebase(GDRAW);
        let mut g = word(h, at);
        for off in (0x880..0x900).step_by(4) {
            h.write(g.wrapping_add(off - 0x80), 0_u32);
            let slot = g.wrapping_add(off);
            let o = word(h, slot);
            if o != 0 {
                release_object(h, o);
                g = word(h, at);
                h.write(slot, 0_u32);
            }
            h.write(g.wrapping_add(off + 0x80), 0_u32);
        }
        h.write(g.wrapping_add(0x980), 0_u32);
    }

    /// `PageHeap_lockAll_00946c40` / `_unlockAll_00946cc0(heap)`: for
    /// each of the 256 buckets whose bit is set in `[+0x40c]`, the import
    /// at `iat` (Enter/LeaveCriticalSection) on the bucket's lock
    /// (`[+4 + 4 k] + 0x18`) -> `heap`.
    pub fn lock_all(h: &mut impl Host, this: u32, iat: u32) -> u32 {
        let f = word(h, h.rebase(iat));
        for k in 0..0x100_u32 {
            let bits = word(h, this + 0x40c);
            if word(h, bits.wrapping_add((k >> 5) * 4)) & (1 << (k & 31)) != 0 {
                let b = word(h, this + 4 + 4 * k);
                h.call(f, Conv::Stdcall, &[b.wrapping_add(0x18)]);
            }
        }
        this
    }

    /// `PageBucket_init_00948900(this; size, align, heap)` (`ret 0xc`):
    /// `+0` size, `+0xc` align, `+0x10` heap, `+0x14` 0, `+4` the size
    /// rounded up to the alignment, `+8` how many fit in the heap's
    /// `+0x18` after its header (`(15 / align + 1) * align`); signed
    /// divisions -> `this`. A zero divisor (a divide fault in the
    /// original) gives 0 here.
    pub fn bucket_init(h: &mut impl Host, this: u32, size: u32, align: u32, heap: u32) -> u32 {
        let (s, a) = (size as i32, align as i32);
        h.write(this, size);
        let q = s.wrapping_sub(1).checked_div(a).unwrap_or(0);
        h.write(this + 0xc, align);
        h.write(this + 0x10, heap);
        h.write(this + 0x14, 0_u32);
        let rounded = q.wrapping_add(1).wrapping_mul(a);
        h.write(this + 4, rounded as u32);
        let header = 15_i32
            .checked_div(a)
            .unwrap_or(0)
            .wrapping_add(1)
            .wrapping_mul(a);
        let room = (word(h, heap.wrapping_add(0x18)) as i32).wrapping_sub(header);
        let n = room.checked_div(rounded).unwrap_or(0);
        h.write(this + 8, n as u32);
        this
    }

    /// `PageHash_find_009455d0(table; key, out)` (`ret 8`) -> `al`: the
    /// open-addressed table (`+0` its power-of-two size, `+4` the slots)
    /// probed from `key[1] & (size - 1)` until an empty slot or an entry
    /// whose `+0` is `key[0]`; a found entry's `+8` goes to `out`.
    pub fn hash_find(h: &mut impl Host, this: u32, key: u32, out: u32) -> bool {
        let slots = word(h, this + 4);
        if slots == 0 {
            return false;
        }
        let mask = word(h, this).wrapping_sub(1);
        let mut i = word(h, key.wrapping_add(4)) & mask;
        if word(h, slots.wrapping_add(4 * i)) != 0 {
            let k = word(h, key);
            loop {
                let e = word(h, slots.wrapping_add(4 * i));
                if word(h, e) == k {
                    break;
                }
                i = i.wrapping_add(1) & mask;
                if word(h, slots.wrapping_add(4 * i)) == 0 {
                    break;
                }
            }
        }
        let e = word(h, slots.wrapping_add(4 * i));
        if e == 0 {
            return false;
        }
        let v = word(h, e.wrapping_add(8));
        h.write(out, v);
        true
    }

    /// `IggyValue_read_00a25860` / `_00a258c0(reader; v, class, out,
    /// cursor)` (`ret 0x10`): 0 without `v`; else the next word of the
    /// cursor (`*cursor += 4`) through the reader's function `+4`
    /// (`ecx` `v`, the callee pops) and, with `out` and `class` the
    /// expected descriptor or 1 (`Phyre_PBase_GetClassDescriptor()` for
    /// `class_call`, else `0x00c90760`), `out` = (0, the result): 1;
    /// else 0.
    pub fn value_read(
        h: &mut impl Host,
        this: u32,
        v: u32,
        class: u32,
        out: u32,
        cursor: u32,
        class_call: bool,
    ) -> u32 {
        if v == 0 {
            return 0;
        }
        let c = word(h, cursor).wrapping_add(4);
        h.write(cursor, c);
        let w = word(h, c.wrapping_sub(4));
        let f = word(h, this + 4);
        let r = h.call(f, Conv::Thiscall(v), &[w]);
        if out == 0 {
            return 0;
        }
        let want = if class_call {
            cdecl(h, GET_CLASS_DESCRIPTOR, &[])
        } else {
            h.rebase(VALUE_CLASS)
        } | 1;
        if class != want {
            return 0;
        }
        h.write(out, 0_u32);
        h.write(out + 4, r);
        1
    }
}
