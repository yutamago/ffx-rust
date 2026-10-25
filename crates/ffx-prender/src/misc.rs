//! Small `PhyreEngine` functions of list p6-render-c (M4 port round 1):
//! accessors, constructors, the render ring buffers' allocators, the
//! render thread's signal and wait wrappers, and the registration and
//! `Bind` drivers. Each bridge names its original; callees stay original
//! and are called at their addresses through [`Host`].

use ffx_graphic::host::{Conv, Host};

/// `Phyre_PhyreFree(ptr)` (cdecl).
pub const PHYRE_FREE: u32 = crate::descriptor::PHYRE_FREE;
/// `Phyre_PhyreMalloc(size, align)` (cdecl).
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
/// `g_PhyreApplication`.
pub const APPLICATION: u32 = 0x00cc_9cd8;
/// `Phyre_PRendering_PRenderThread_GetInstance()` (cdecl).
pub const RENDER_THREAD_INSTANCE: u32 = 0x0061_4410;
/// `Phyre_PRendering_PRenderThread_sync(thread)` (thiscall).
pub const RENDER_THREAD_SYNC: u32 = 0x0061_5ea0;
/// `Phyre_PRendering_PRenderThread_threadEntry(thread)` (thiscall).
pub const RENDER_THREAD_ENTRY: u32 = 0x0061_6010;
/// `Phyre_PEventFlagWin32_signal(flag)` (thiscall).
pub const EVENT_SIGNAL: u32 = 0x0043_0200;
/// `Phyre_PEventFlagWin32_PEventFlagWin32(flag)` (thiscall).
pub const EVENT_FLAG_CTOR: u32 = 0x0042_fc80;
/// `Phyre_PSemaphoreWin32_wait(sem)` (thiscall).
pub const SEMAPHORE_WAIT: u32 = 0x0043_02b0;
/// `Phyre_PCriticalSectionBase_initialize(cs)` (thiscall).
pub const CRITICAL_SECTION_INIT: u32 = 0x0043_17f0;
/// `Phyre_PTypeDispenser_RegisterType(descriptor)` (cdecl).
pub const REGISTER_TYPE: u32 = 0x0043_95b0;
/// `Phyre_PClassDescriptor_addToNamespace(descriptor)` (thiscall).
pub const ADD_TO_NAMESPACE: u32 = 0x0043_c140;
/// `ClassDynamicMesh`'s class descriptor (registered by
/// [`bridge::register_dynamic_mesh`]).
pub const DYNAMIC_MESH_DESCRIPTOR: u32 = 0x00cc_ba18;
/// `Phyre_PRendering_PRendererBase_renderMeshInstance(renderer, instance,
/// node, 0, pass)` (thiscall, `ret 0x10`).
pub const RENDER_MESH_INSTANCE: u32 = 0x005b_9a90;
/// The `Bind` functions `PApplication::prePhyreInit` runs (cdecl).
pub const APPLICATION_BINDS: [u32; 3] = [0x0062_2050, 0x0062_27b0, 0x0062_2b60];
/// The `Bind` functions `PUtilityText::initialize` runs (cdecl).
pub const TEXT_BINDS: [u32; 2] = [0x00a2_c020, 0x00a2_c350];
/// `PWindowWin32Base`'s flip interval (1 or 2).
pub const FLIP_INTERVAL: u32 = 0x00c3_0e88;
/// `operator_new(size)` (cdecl).
pub const OPERATOR_NEW: u32 = 0x0063_0310;
/// The import slot `0x00625850` jumps through when `operator_new` fails
/// (the bad-alloc throw).
pub const BAD_ALLOC_SLOT: u32 = 0x00b0_c2a0;
/// The image address `0x005b3580` stores at `+0x18` (a default object).
pub const DEFAULT_0018: u32 = 0x00ca_34cc;
/// The head of the named list `0x005c84f0` searches (`{name, next}`
/// nodes; the modifier types' semantic names).
pub const NAMED_LIST: u32 = 0x00cb_d348;
/// `Phyre_PString_Assign(string, text)` (thiscall).
pub const PSTRING_ASSIGN: u32 = 0x0044_5cf0;
/// `g_hWnd`.
pub const HWND: u32 = 0x00cc_9ce8;
/// `USER32!SetWindowTextA`'s import slot.
pub const SET_WINDOW_TEXT_SLOT: u32 = 0x00b0_c6d4;
/// `PUtilityText`'s render pass per technique (cached).
pub const TEXT_PASS_CACHE: u32 = 0x01a8_58f4;
/// The technique names `GetRenderPassForTechnique` looks up (8-byte entries).
pub const TEXT_PASS_NAMES: u32 = 0x00b7_ab00;
/// The render pass lookup by name `0x004dd860` (cdecl).
pub const FIND_RENDER_PASS: u32 = 0x004d_d860;
/// `Phyre_PRendering_PRenderThread_queueFrame_00615210` (thiscall).
pub const QUEUE_FRAME: u32 = 0x0061_5210;
/// `Phyre_PRendering_PRenderRingBufferSegmented_nextSegment_00613eb0` (thiscall).
pub const RING_NEXT_SEGMENT: u32 = 0x0061_3eb0;

pub mod bridge {
    //! The originals over game memory.

    use super::{
        ADD_TO_NAMESPACE, APPLICATION, APPLICATION_BINDS, BAD_ALLOC_SLOT, CRITICAL_SECTION_INIT,
        Conv, DYNAMIC_MESH_DESCRIPTOR, EVENT_FLAG_CTOR, EVENT_SIGNAL, FLIP_INTERVAL, Host,
        OPERATOR_NEW, PHYRE_FREE, PHYRE_MALLOC, REGISTER_TYPE, RENDER_MESH_INSTANCE,
        RENDER_THREAD_ENTRY, RENDER_THREAD_INSTANCE, RENDER_THREAD_SYNC, SEMAPHORE_WAIT,
        TEXT_BINDS,
    };

    /// `Phyre_PFramework_PApplication_GetApplication()` -> `g_PhyreApplication`.
    pub fn get_application(h: &mut impl Host) -> u32 {
        h.read(h.rebase(APPLICATION))
    }

    /// `PApplication::prePhyreInit()` -> 0: the three class `Bind`s.
    pub fn pre_phyre_init(h: &mut impl Host) -> u32 {
        for f in APPLICATION_BINDS {
            h.call_sym(f, Conv::Cdecl, &[]);
        }
        0
    }

    /// `PUtilityText::initialize()` -> 0: the font classes' `Bind`s.
    pub fn text_initialize(h: &mut impl Host) -> u32 {
        for f in TEXT_BINDS {
            h.call_sym(f, Conv::Cdecl, &[]);
        }
        0
    }

    /// `PApplication::setWindowIconID(app, id)` -> `id` (left in `eax`).
    pub fn set_window_icon_id(h: &mut impl Host, app: u32, id: u32) -> u32 {
        h.write(app.wrapping_add(0x39c), id);
        id
    }

    /// `PRenderer::syncRender()`: `PRenderThread::sync` on the render
    /// thread's instance; its `eax`.
    pub fn sync_render(h: &mut impl Host) -> u32 {
        let thread = h.call_sym(RENDER_THREAD_INSTANCE, Conv::Cdecl, &[]);
        h.call_sym(RENDER_THREAD_SYNC, Conv::Thiscall(thread), &[])
    }

    /// `PRenderThread::StaticThreadEntry(thread)` (cdecl): the render
    /// thread's body, `threadEntry` on `thread`; its `eax`.
    pub fn static_thread_entry(h: &mut impl Host, thread: u32) -> u32 {
        h.call_sym(RENDER_THREAD_ENTRY, Conv::Thiscall(thread), &[])
    }

    /// `PRenderThread::SignalSynchronizationEventFlagRenderCallback(cmd)`
    /// (cdecl): signals the event flag at `cmd +4`; its `eax` (0).
    pub fn signal_sync_flag(h: &mut impl Host, cmd: u32) -> u32 {
        let flag: u32 = h.read(cmd.wrapping_add(4));
        h.call_sym(EVENT_SIGNAL, Conv::Thiscall(flag), &[])
    }

    /// `PRenderThread::getFlipSlot(thread)` -> 0: waits on the semaphore
    /// at `thread +0x164`.
    pub fn get_flip_slot(h: &mut impl Host, thread: u32) -> u32 {
        h.call_sym(
            SEMAPHORE_WAIT,
            Conv::Thiscall(thread.wrapping_add(0x164)),
            &[],
        );
        0
    }

    /// `0x00672790()`: registers `ClassDynamicMesh`'s descriptor with the
    /// type dispenser, then adds it to its namespace (a tail call); its
    /// `eax`.
    pub fn register_dynamic_mesh(h: &mut impl Host) -> u32 {
        let d = h.rebase(DYNAMIC_MESH_DESCRIPTOR);
        h.call_sym(REGISTER_TYPE, Conv::Cdecl, &[d]);
        h.call_sym(ADD_TO_NAMESPACE, Conv::Thiscall(d), &[])
    }

    /// `PWorldCursor` run length (`0x009f3810`, thiscall): the entries
    /// left between `+0xc` and `+0x10` in strides of `+0x14` (unsigned),
    /// else the count at `+0x18`. A zero stride faults in the original
    /// (`div`); the port returns 0 there, never panicking.
    pub fn cursor_run_length(h: &mut impl Host, cursor: u32) -> u32 {
        let at: u32 = h.read(cursor.wrapping_add(0xc));
        let end: u32 = h.read(cursor.wrapping_add(0x10));
        if at < end {
            let stride: u32 = h.read(cursor.wrapping_add(0x14));
            (end - at).checked_div(stride).unwrap_or(0)
        } else {
            h.read(cursor.wrapping_add(0x18))
        }
    }

    /// `0x005b9a70(renderer, instance)` (thiscall): `renderMeshInstance(
    /// instance, instance +0x10's node, 0, renderer +0x13c)`; its result.
    pub fn render_instance(h: &mut impl Host, renderer: u32, instance: u32) -> u32 {
        let node: u32 = h.read(instance.wrapping_add(0x10));
        let pass: u32 = h.read(renderer.wrapping_add(0x13c));
        h.call_sym(
            RENDER_MESH_INSTANCE,
            Conv::Thiscall(renderer),
            &[instance, node, 0, pass],
        )
    }

    /// `0x005b2c70(array, i)` (thiscall): element `i` of a `PArray` of
    /// words (`+4` count, bit 31 the not-owned flag; `+8` data), 0 past
    /// the end.
    pub fn array_at(h: &mut impl Host, array: u32, i: u32) -> u32 {
        let n = h.read::<u32>(array.wrapping_add(4)) & 0x7fff_ffff;
        if i >= n {
            return 0;
        }
        let data: u32 = h.read(array.wrapping_add(8));
        h.read(data.wrapping_add(i.wrapping_mul(4)))
    }

    /// `PRenderSurface::PRenderSurface(surface, a, b)` -> `surface`:
    /// `{a, 0, b, 0}`.
    pub fn render_surface_ctor(h: &mut impl Host, surface: u32, a: u32, b: u32) -> u32 {
        h.write(surface, a);
        h.write(surface.wrapping_add(8), b);
        h.write(surface.wrapping_add(4), 0u32);
        h.write(surface.wrapping_add(0xc), 0u32);
        surface
    }

    /// `0x005b3620(obj)` (thiscall) -> `obj`: four zero words.
    pub fn zero4(h: &mut impl Host, obj: u32) -> u32 {
        for i in 0..4 {
            h.write(obj.wrapping_add(4 * i), 0u32);
        }
        obj
    }

    /// `0x005b3660(obj, a, b, c)` (thiscall): `+0x10..0x18 = a, b, c`;
    /// `eax` is `c`.
    pub fn set_three(h: &mut impl Host, obj: u32, a: u32, b: u32, c: u32) -> u32 {
        h.write(obj.wrapping_add(0x10), a);
        h.write(obj.wrapping_add(0x14), b);
        h.write(obj.wrapping_add(0x18), c);
        c
    }

    /// `0x0061c890(handle)` (thiscall): a pooled handle `{owner, slot}`:
    /// when `owner` is set, `0x004ba540(owner +0x14's pool; &handle.slot,
    /// owner)` gives the slot back (thiscall, `ret 8`); then `owner` = 0.
    pub fn release_handle(h: &mut impl Host, release: u32, handle: u32) {
        let owner: u32 = h.read(handle);
        if owner != 0 {
            let pool: u32 = h.read(owner.wrapping_add(0x14));
            h.call_sym(
                release,
                Conv::Thiscall(pool),
                &[handle.wrapping_add(4), owner],
            );
        }
        h.write(handle, 0u32);
    }

    /// `0x005c86e0(out, x)` (cdecl): `*out = lookup(x)` (`0x005c84f0`,
    /// cdecl); 0 when found, 0x13 when not.
    pub fn lookup_into(h: &mut impl Host, lookup: u32, out: u32, x: u32) -> u32 {
        let r = h.call_sym(lookup, Conv::Cdecl, &[x]);
        h.write(out, r);
        if r == 0 { 0x13 } else { 0 }
    }

    /// `0x005aead0` / `0x005aece0(obj, i, args...)` (thiscall): the entry
    /// `i` of the 16-byte table at `obj +0x38`: 0 when its first word is
    /// null, else `f(entry, args...)` (thiscall).
    pub fn table_call(h: &mut impl Host, f: u32, obj: u32, i: u32, args: &[u32]) -> u32 {
        let table: u32 = h.read(obj.wrapping_add(0x38));
        let entry: u32 = h.read(table.wrapping_add(i.wrapping_mul(16)));
        if entry == 0 {
            return 0;
        }
        h.call_sym(f, Conv::Thiscall(entry), args)
    }

    /// `0x00629840(lock, base, size)` (thiscall) -> 0: a range `{+0x18
    /// base, +0x1c end, +0x20 size, +0x24 and +0x28 cursors at base}`,
    /// then `PCriticalSectionBase::initialize(lock)`.
    pub fn range_init(h: &mut impl Host, lock: u32, base: u32, size: u32) -> u32 {
        h.write(lock.wrapping_add(0x20), size);
        h.write(lock.wrapping_add(0x18), base);
        h.write(lock.wrapping_add(0x1c), size.wrapping_add(base));
        h.write(lock.wrapping_add(0x24), base);
        h.write(lock.wrapping_add(0x28), base);
        h.call_sym(CRITICAL_SECTION_INIT, Conv::Thiscall(lock), &[]);
        0
    }

    /// `0x005a04d0(format)` (cdecl): whether `format` is 0x4a, 0x13, 0x90
    /// or 0x91, in `al` over the rest of `format`.
    #[must_use]
    pub fn format_in_set(format: u32) -> u32 {
        let yes = matches!(format, 0x4a | 0x13 | 0x90 | 0x91);
        (format & !0xff) | u32::from(yes)
    }

    /// `0x00613e80(ring, base, size, extra)` (thiscall) -> 0: the words
    /// `{base, base + size, base, 0, extra}`.
    pub fn ring_init(h: &mut impl Host, ring: u32, base: u32, size: u32, extra: u32) -> u32 {
        h.write(ring.wrapping_add(4), size.wrapping_add(base));
        h.write(ring.wrapping_add(0x10), extra);
        h.write(ring, base);
        h.write(ring.wrapping_add(8), base);
        h.write(ring.wrapping_add(0xc), 0u32);
        0
    }

    /// `PRenderRingBufferSegmented::allocate(ring, size)`: `size` bytes
    /// at the cursor `+8`, wrapping to the start `+0` when they pass the
    /// end `+4`; the block, or 0 (the cursor left at the wrapped start)
    /// when even that does not fit.
    pub fn segmented_allocate(h: &mut impl Host, ring: u32, size: u32) -> u32 {
        let end_of_ring: u32 = h.read(ring.wrapping_add(4));
        let mut at: u32 = h.read(ring.wrapping_add(8));
        let mut end = at.wrapping_add(size);
        if end > end_of_ring {
            at = h.read(ring);
            end = at.wrapping_add(size);
        }
        if end <= end_of_ring {
            h.write(ring.wrapping_add(8), end);
            at
        } else {
            h.write(ring.wrapping_add(8), at);
            0
        }
    }

    /// `PRenderRingBuffer::stepForAllocation(ring, cursor, size, out)`:
    /// the block for `size` bytes at `cursor`, wrapping to the start
    /// `+0` past the end `+4`; `*out` = the block (0 when it does not
    /// fit); returns the cursor after it, or the block's start when it
    /// does not fit.
    pub fn step_for_allocation(
        h: &mut impl Host,
        ring: u32,
        cursor: u32,
        size: u32,
        out: u32,
    ) -> u32 {
        let end_of_ring: u32 = h.read(ring.wrapping_add(4));
        let mut at = cursor;
        let mut end = at.wrapping_add(size);
        if end > end_of_ring {
            at = h.read(ring);
            end = at.wrapping_add(size);
        }
        h.write(out, 0u32);
        if end <= end_of_ring {
            h.write(out, at);
            end
        } else {
            at
        }
    }

    /// `PKick::PKick(kick)` -> `kick`: two zero words, the event flag at
    /// `+8`, the byte `+0xc` and the word `+0x10` zeroed.
    pub fn kick_ctor(h: &mut impl Host, kick: u32) -> u32 {
        h.write(kick, 0u32);
        h.write(kick.wrapping_add(4), 0u32);
        h.call_sym(EVENT_FLAG_CTOR, Conv::Thiscall(kick.wrapping_add(8)), &[]);
        h.write(kick.wrapping_add(0xc), 0u8);
        h.write(kick.wrapping_add(0x10), 0u32);
        kick
    }

    /// `PWindowWin32Base::SetFlipVSyncInterval(n)` (cdecl): the interval
    /// clamped to 1..=2; `eax` is 2 for `n >= 2`, else `n` (0 stays 0
    /// while the interval becomes 1).
    pub fn set_flip_interval(h: &mut impl Host, n: u32) -> u32 {
        let at = h.rebase(FLIP_INTERVAL);
        if n >= 2 {
            h.write(at, 2u32);
            2
        } else {
            h.write(at, 1u32);
            n
        }
    }

    /// A constructor that runs its base's (thiscall) and then stores its
    /// own vtable and, where given, the word at `+4` (image addresses);
    /// returns `obj` (`0x00672630`, `0x009fc140`, `0x009fc7c0`).
    pub fn derived_ctor(
        h: &mut impl Host,
        obj: u32,
        base: u32,
        vtable: u32,
        word4: Option<u32>,
    ) -> u32 {
        h.call_sym(base, Conv::Thiscall(obj), &[]);
        let v = h.rebase(vtable);
        h.write(obj, v);
        if let Some(w) = word4 {
            let w = h.rebase(w);
            h.write(obj.wrapping_add(4), w);
        }
        obj
    }

    /// `0x00a3e5a0(user, size, _)` (stdcall): Iggy's allocation callback,
    /// `PhyreMalloc(size, 1)`.
    pub fn iggy_alloc(h: &mut impl Host, size: u32) -> u32 {
        h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[size, 1])
    }

    /// `0x00a3e5c0(user, ptr)` (stdcall): Iggy's free callback,
    /// `PhyreFree(ptr)`; its `eax`.
    pub fn iggy_free(h: &mut impl Host, ptr: u32) -> u32 {
        h.call_sym(PHYRE_FREE, Conv::Cdecl, &[ptr])
    }

    /// `0x005c7f50(header, a, b)` (thiscall): the header's callback at
    /// `+0x94` as `callback(a, b)` (cdecl); its `eax`.
    pub fn header_call(h: &mut impl Host, header: u32, a: u32, b: u32) -> u32 {
        let f: u32 = h.read(header.wrapping_add(0x94));
        h.call(f, Conv::Cdecl, &[a, b])
    }

    /// `0x00625850()`: a `std` tree's head node from `operator_new(0x14)`:
    /// its three links at itself, the colour and nil bytes `0x101`; when
    /// the allocation fails, the bad-alloc throw through
    /// [`BAD_ALLOC_SLOT`] (which does not return).
    pub fn tree_head(h: &mut impl Host) -> u32 {
        let node = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x14]);
        if node == 0 {
            let throw: u32 = h.read(h.rebase(BAD_ALLOC_SLOT));
            return h.call(throw, Conv::Cdecl, &[]);
        }
        h.write(node, node);
        h.write(node.wrapping_add(4), node);
        h.write(node.wrapping_add(8), node);
        h.write(node.wrapping_add(0xc), 0x101u16);
        node
    }

    /// A constructor that zeroes the first `words` words of `obj`; returns
    /// `obj` (`0x00613d90`: 5, `0x005b4560`: 7).
    pub fn zero_words(h: &mut impl Host, obj: u32, words: u32) -> u32 {
        for i in 0..words {
            h.write(obj.wrapping_add(4 * i), 0u32);
        }
        obj
    }

    /// `0x005b3580(obj)` (thiscall) -> `obj`: six zero words, then the
    /// image address `0x00ca34cc` at `+0x18`.
    pub fn zero6_with_default(h: &mut impl Host, obj: u32) -> u32 {
        zero_words(h, obj, 6);
        let d = h.rebase(super::DEFAULT_0018);
        h.write(obj.wrapping_add(0x18), d);
        obj
    }

    /// `0x005bf340(obj)` (thiscall) -> `obj`: the words `+0..+0x30`
    /// and `+0x38`, `+0x3c` zeroed, the byte `+0x34` too.
    pub fn zero_modifier(h: &mut impl Host, obj: u32) -> u32 {
        for at in (0..0x34).step_by(4).chain([0x38, 0x3c]) {
            h.write(obj.wrapping_add(at), 0u32);
        }
        h.write(obj.wrapping_add(0x34), 0u8);
        obj
    }

    /// `0x005b8b00(buffer, desc)` (thiscall): the bytes `desc` names in
    /// `buffer` (offset `u16 +8`, length `u16 +0xa & 0x1fff`): `buffer +
    /// offset` when the range ends within the size at `buffer +0` and
    /// starts past that 4-byte header, else 0.
    pub fn buffer_slice(h: &mut impl Host, buffer: u32, desc: u32) -> u32 {
        let offset = u32::from(h.read::<u16>(desc.wrapping_add(8)));
        let len = u32::from(h.read::<u16>(desc.wrapping_add(0xa))) & 0x1fff;
        let size: u32 = h.read(buffer);
        if len + offset > size || offset < 4 {
            return 0;
        }
        offset.wrapping_add(buffer)
    }

    /// `0x005b88e0(obj, i)` (thiscall): the address of the 20-byte record
    /// `i`'s `+0x10` in `obj`'s array (`+0x20` count, bit 31 the
    /// not-owned flag; `+0x24` the data, or the one record inline at
    /// `+0x24` for a count of 1; 0 for none).
    pub fn record_address(h: &mut impl Host, obj: u32, i: u32) -> u32 {
        let n = h.read::<u32>(obj.wrapping_add(0x20)) & 0x7fff_ffff;
        let data = match n {
            0 => 0,
            1 => obj.wrapping_add(0x24),
            _ => h.read(obj.wrapping_add(0x24)),
        };
        data.wrapping_add(i.wrapping_mul(5).wrapping_add(4).wrapping_mul(4))
    }

    /// `0x005b44c0(obj, a, b, c, d, e)` (thiscall, `ret 0x14`) -> `obj`:
    /// `{a, b, c, 0, d, e, 0, 0, 0, 0}` and `+0x68`, `+0x6c` = 0.
    pub fn stream_ctor(h: &mut impl Host, obj: u32, args: [u32; 5]) -> u32 {
        let [a, b, c, d, e] = args;
        for (at, v) in [(0, a), (4, b), (8, c), (0xc, 0), (0x10, d), (0x14, e)] {
            h.write(obj.wrapping_add(at), v);
        }
        for at in [0x18, 0x1c, 0x20, 0x24, 0x68, 0x6c] {
            h.write(obj.wrapping_add(at), 0u32);
        }
        obj
    }

    /// `0x005c8100(obj, a..j)` (thiscall, `ret 0x28`) -> `obj`: a
    /// modifier-network record from ten words (`args[0]` at `+0`, the
    /// arrays at `+8` and `+0x10` marked not owned, bit 31), the rest
    /// zeroed and `+0x3c` = -1. `args[8]` is not read.
    pub fn network_record_ctor(h: &mut impl Host, obj: u32, args: [u32; 10]) -> u32 {
        let w = |i: usize| args[i];
        h.write(obj, w(0));
        h.write(obj.wrapping_add(4), 0u32);
        h.write(obj.wrapping_add(0xc), w(5));
        h.write(obj.wrapping_add(8), w(3) | 0x8000_0000);
        h.write(obj.wrapping_add(0x14), w(6));
        h.write(obj.wrapping_add(0x10), w(4) | 0x8000_0000);
        h.write(obj.wrapping_add(0x2c), w(7));
        h.write(obj.wrapping_add(0x28), w(9));
        h.write(obj.wrapping_add(0x20), w(1));
        h.write(obj.wrapping_add(0x24), w(2));
        for at in [0x18, 0x1c, 0x34, 0x30, 0x38] {
            h.write(obj.wrapping_add(at), 0u32);
        }
        h.write(obj.wrapping_add(0x3c), u32::MAX);
        obj
    }

    /// `0x005c84f0(name)` (cdecl): the node of the list at
    /// [`super::NAMED_LIST`] (`{name, next}`) whose name equals `name`
    /// (byte for byte, up to the terminator), or 0 (also for a null
    /// `name`).
    pub fn find_named(h: &mut impl Host, name: u32) -> u32 {
        if name == 0 {
            return 0;
        }
        let mut node: u32 = h.read(h.rebase(super::NAMED_LIST));
        while node != 0 {
            let s: u32 = h.read(node);
            if same_string(h, s, name) {
                return node;
            }
            node = h.read(node.wrapping_add(4));
        }
        0
    }

    /// Whether the strings at `a` and `b` are equal (the inlined `strcmp`
    /// of `0x005c84f0`, which only tests for 0).
    fn same_string(h: &impl Host, a: u32, b: u32) -> bool {
        let mut i = 0u32;
        loop {
            let x: u8 = h.read(a.wrapping_add(i));
            if x != h.read::<u8>(b.wrapping_add(i)) {
                return false;
            }
            if x == 0 {
                return true;
            }
            i = i.wrapping_add(1);
        }
    }

    /// `0x005b52b0(array)` (cdecl): how many of the array's 16-byte
    /// entries (`+0` count, bit 31 the not-owned flag; `+4` data) have a
    /// kind byte (`+2`) of 0x12, 0x42, 0x43, 0x44 or 0xd9..=0xdc.
    pub fn count_kinds(h: &mut impl Host, array: u32) -> u32 {
        let n = h.read::<u32>(array) & 0x7fff_ffff;
        let data: u32 = h.read(array.wrapping_add(4));
        let mut count = 0u32;
        for i in 0..n {
            let kind: u8 = h.read(data.wrapping_add(2).wrapping_add(i.wrapping_mul(16)));
            if matches!(kind, 0x42 | 0xd9..=0xdc | 0x12 | 0x43 | 0x44) {
                count = count.wrapping_add(1);
            }
        }
        count
    }

    /// `0x005b5370(base, list, out)` (cdecl) -> the end of `out`: for each
    /// of `list`'s 16-byte entries (`+0` count & 0x7fffffff, `+4` data),
    /// the slot at `base + u16 entry+8`: when its object (`slot +0`) is
    /// set, the address of the object's 20-byte record `slot +8` plus
    /// 0x10 (the object's array: `+0x18` count, bit 31 the not-owned flag;
    /// `+0x1c` the data, inline for a count of 1) goes to `out`.
    pub fn gather_records(h: &mut impl Host, base: u32, list: u32, out: u32) -> u32 {
        let n = h.read::<u32>(list) & 0x7fff_ffff;
        let mut out = out;
        for i in 0..n {
            let entries: u32 = h.read(list.wrapping_add(4));
            let off =
                u32::from(h.read::<u16>(entries.wrapping_add(i.wrapping_mul(16)).wrapping_add(8)));
            let slot = off.wrapping_add(base);
            let obj: u32 = h.read(slot);
            if obj == 0 {
                continue;
            }
            let count = h.read::<u32>(obj.wrapping_add(0x18)) & 0x7fff_ffff;
            let data = match count {
                0 => 0,
                1 => obj.wrapping_add(0x1c),
                _ => h.read(obj.wrapping_add(0x1c)),
            };
            let index: u32 = h.read(slot.wrapping_add(8));
            h.write(
                out,
                data.wrapping_add(index.wrapping_mul(20)).wrapping_add(0x10),
            );
            out = out.wrapping_add(4);
        }
        out
    }

    /// `0x00613eb0(ring)` (thiscall): steps the segmented ring (`+0`
    /// start, `+4` end, `+8` cursor, `+0xc` the next segment's end,
    /// `+0x10` the segment size) to its next segment: the cursor moves
    /// to the pending end when there is one, the next end is the cursor
    /// plus a segment, wrapped past the end; a next end before the cursor
    /// restarts both at the start.
    pub fn ring_next_segment(h: &mut impl Host, ring: u32) {
        let pending: u32 = h.read(ring.wrapping_add(0xc));
        if pending != 0 {
            h.write(ring.wrapping_add(8), pending);
        }
        let seg: u32 = h.read(ring.wrapping_add(0x10));
        let end: u32 = h.read(ring.wrapping_add(4));
        let cursor: u32 = h.read(ring.wrapping_add(8));
        let start: u32 = h.read(ring);
        let wrap = |e: u32| {
            if e > end {
                e.wrapping_add(start.wrapping_sub(end))
            } else {
                e
            }
        };
        let next = wrap(cursor.wrapping_add(seg));
        h.write(ring.wrapping_add(0xc), next);
        if next < cursor {
            h.write(ring.wrapping_add(8), start);
            h.write(ring.wrapping_add(0xc), wrap(start.wrapping_add(seg)));
        }
    }

    /// How a `PArray`'s `setView` callback releases the old buffer.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Release {
        /// `PhyreFree(old)`, a null one too (65 bytes).
        Free,
        /// `PhyreFree(old)` when it is set (69 bytes).
        FreeSet,
        /// `free(old, count & 0x7fffffff)` (cdecl; the elements' own
        /// release, 74 bytes).
        With(u32),
    }

    /// A `PArray`'s `setView(arr, data, count)` callback (thiscall, `ret
    /// 8`; the member's `+0x3c` callback `member_call` runs): for a
    /// non-negative `count`, an owned old buffer (bit 31 of `arr +0`
    /// clear) other than `data` is released, then `arr = {count | bit
    /// 31, data}`, a view the array does not own. Returns 0.
    pub fn array_set_view(
        h: &mut impl Host,
        release: Release,
        arr: u32,
        data: u32,
        count: u32,
    ) -> u32 {
        if count & 0x8000_0000 != 0 {
            return 0;
        }
        let old: u32 = h.read(arr.wrapping_add(4));
        let header: u32 = h.read(arr);
        if old != data && header & 0x8000_0000 == 0 {
            match release {
                Release::Free => {
                    h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
                }
                Release::FreeSet if old != 0 => {
                    h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
                }
                Release::FreeSet => {}
                Release::With(f) => {
                    h.call_sym(f, Conv::Cdecl, &[old, header & 0x7fff_ffff]);
                }
            }
        }
        h.write(arr.wrapping_add(4), data);
        h.write(arr, count | 0x8000_0000);
        0
    }

    /// `0x005c1ef0(records, n)` (cdecl): frees an array of `n` 20-byte
    /// records, each holding a `PArray` at `+0xc` (its owned, set buffer
    /// freed, then zeroed), then the records themselves; nothing for null
    /// `records`. Returns `PhyreFree`'s `eax`, 0 for null `records` (the
    /// original leaves the caller's `eax` there; its callers, the
    /// destroy and `setView` callbacks, drop it).
    pub fn free_records(h: &mut impl Host, records: u32, n: u32) -> u32 {
        if records == 0 {
            return 0;
        }
        for i in 0..n {
            let arr = records.wrapping_add(0xc).wrapping_add(i.wrapping_mul(20));
            let count: u32 = h.read(arr);
            if count & 0x8000_0000 == 0 {
                let data: u32 = h.read(arr.wrapping_add(4));
                if data != 0 {
                    h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
                }
            }
            h.write(arr.wrapping_add(4), 0u32);
            h.write(arr, 0u32);
        }
        h.call_sym(PHYRE_FREE, Conv::Cdecl, &[records])
    }

    /// `0x005c8660(node)` (cdecl): adds a `{name, next}` node to the named
    /// list ([`super::NAMED_LIST`]) at its head: 0x11 for a null name,
    /// 0x12 when the name is already there ([`find_named`]), else 0.
    pub fn add_named(h: &mut impl Host, node: u32) -> u32 {
        let name: u32 = h.read(node);
        if name == 0 {
            return 0x11;
        }
        if find_named(h, name) != 0 {
            return 0x12;
        }
        let head = h.rebase(super::NAMED_LIST);
        let first: u32 = h.read(head);
        h.write(node.wrapping_add(4), first);
        h.write(head, node);
        0
    }

    /// A derived constructor (`0x009fc2b0`, `0x009fc480`, `0x009fc4e0`,
    /// `0x009fc650`): the base constructor (thiscall), the words at
    /// `zeros` zeroed, the class's vtable and the word at `+4` where given
    /// (image addresses). Returns `obj`.
    pub fn derived_ctor_zeroed(
        h: &mut impl Host,
        obj: u32,
        base: u32,
        vtable: Option<u32>,
        word4: Option<u32>,
        zeros: &[u32],
    ) -> u32 {
        h.call_sym(base, Conv::Thiscall(obj), &[]);
        for &at in zeros {
            h.write(obj.wrapping_add(at), 0u32);
        }
        if let Some(v) = vtable {
            let v = h.rebase(v);
            h.write(obj, v);
        }
        if let Some(w) = word4 {
            let w = h.rebase(w);
            h.write(obj.wrapping_add(4), w);
        }
        obj
    }

    /// `PApplication::setWindowTitle(app, title)`: the title into the
    /// `PString` at `app +0x398` (`Phyre_PString_Assign`, thiscall), then
    /// `SetWindowTextA(g_hWnd, text)` with the string's pointer (its
    /// first word less its low flag bit); `SetWindowTextA`'s result.
    pub fn set_window_title(h: &mut impl Host, app: u32, title: u32) -> u32 {
        let s = app.wrapping_add(0x398);
        h.call_sym(super::PSTRING_ASSIGN, Conv::Thiscall(s), &[title]);
        let word: u32 = h.read(s);
        let text = word.wrapping_sub(word & 1);
        let hwnd: u32 = h.read(h.rebase(super::HWND));
        let f: u32 = h.read(h.rebase(super::SET_WINDOW_TEXT_SLOT));
        h.call(f, Conv::Stdcall, &[hwnd, text])
    }

    /// `PUtilityText::GetRenderPassForTechnique(i)` (cdecl): the cached
    /// pass of technique `i` (`0x01a858f4[i]`), looked up once by its name
    /// (`0x00b7ab00[i]`, 8-byte entries) through `0x004dd860` (cdecl).
    pub fn text_render_pass(h: &mut impl Host, i: u32) -> u32 {
        let slot = h
            .rebase(super::TEXT_PASS_CACHE)
            .wrapping_add(i.wrapping_mul(4));
        if h.read::<u32>(slot) == 0 {
            let name: u32 = h.read(
                h.rebase(super::TEXT_PASS_NAMES)
                    .wrapping_add(i.wrapping_mul(8)),
            );
            let pass = h.call_sym(super::FIND_RENDER_PASS, Conv::Cdecl, &[name]);
            h.write(slot, pass);
        }
        h.read(slot)
    }

    /// `0x00615210(thread, a, b, c)` (thiscall, `ret 0xc`) -> 0: waits for
    /// a free slot (the semaphore at `+0x15c`), fills the next of the 16
    /// 20-byte frame slots (`+0x154` the index; slot `+0x10` a, `+0x14` b,
    /// `+0x20` c), advances the index and signals the slot's event flag
    /// (`+0x18`).
    pub fn queue_frame(h: &mut impl Host, thread: u32, a: u32, b: u32, c: u32) -> u32 {
        h.call_sym(
            SEMAPHORE_WAIT,
            Conv::Thiscall(thread.wrapping_add(0x15c)),
            &[],
        );
        let i: u32 = h.read(thread.wrapping_add(0x154));
        let slot = thread.wrapping_add(i.wrapping_mul(20));
        h.write(slot.wrapping_add(0x10), a);
        h.write(slot.wrapping_add(0x14), b);
        h.write(slot.wrapping_add(0x20), c);
        let next = i.wrapping_add(1);
        h.write(
            thread.wrapping_add(0x154),
            if next == 16 { 0 } else { next },
        );
        h.call_sym(EVENT_SIGNAL, Conv::Thiscall(slot.wrapping_add(0x18)), &[]);
        0
    }

    /// `PRendererBase` vtable slot 2 (`0x005b6c50`, thiscall): hands the
    /// frame (`+0x4c`, `+0x50`, the semaphore `+0x44`) to the render
    /// thread ([`queue_frame`] on its instance); on success waits on that
    /// semaphore when set, clears `+0x4c`/`+0x50` and steps both ring
    /// buffers (`+0x1c`, `+0x30`, [`ring_next_segment`]). Returns the
    /// queue's result (0).
    pub fn renderer_submit_frame(h: &mut impl Host, renderer: u32) -> u32 {
        let a: u32 = h.read(renderer.wrapping_add(0x4c));
        let b: u32 = h.read(renderer.wrapping_add(0x50));
        let c: u32 = h.read(renderer.wrapping_add(0x44));
        let thread = h.call_sym(RENDER_THREAD_INSTANCE, Conv::Cdecl, &[]);
        let r = h.call_sym(super::QUEUE_FRAME, Conv::Thiscall(thread), &[a, b, c]);
        if r != 0 {
            return r;
        }
        let sem: u32 = h.read(renderer.wrapping_add(0x44));
        if sem != 0 {
            h.call_sym(SEMAPHORE_WAIT, Conv::Thiscall(sem), &[]);
        }
        h.write(renderer.wrapping_add(0x4c), 0u32);
        h.write(renderer.wrapping_add(0x50), 0u32);
        h.call_sym(
            super::RING_NEXT_SEGMENT,
            Conv::Thiscall(renderer.wrapping_add(0x1c)),
            &[],
        );
        h.call_sym(
            super::RING_NEXT_SEGMENT,
            Conv::Thiscall(renderer.wrapping_add(0x30)),
            &[],
        );
        0
    }
}
