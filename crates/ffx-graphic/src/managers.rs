//! The render layer's managers (singletons the remaster keeps beside the
//! scene: post-processing, shadows, texture animation, dynamic geometry
//! memory, distortion and blur) and their one-line accessors.
//!
//! Each manager is a C++ object; these are the accessors whose logic is
//! the game's (fields, counters, singletons). Their virtual methods and
//! the `PhyreEngine` work behind them stay original.

/// The originals, over game memory.
pub mod bridge {
    use crate::host::{Conv, Host};
    use ffx_bridge::{Global, Mem, x87};

    pub use crate::scene::bridge::{embedded, field, set_byte_field, set_field, set_if_changed};

    /// `g_PostProcessManager` (`PostProcessManager_GetInstance`).
    pub const POST_PROCESS: Global<u32> = Global::new(0x00cc_c8a0);
    /// `g_PostProcessTweakUtil` (`PostProcessTweakUtil_GetInstance`).
    pub const POST_PROCESS_TWEAK: Global<u32> = Global::new(0x00cc_c89c);
    /// `g_TextureAnimationManager` (`TextureAnimationManager_GetInstance`).
    pub const TEXTURE_ANIMATION: Global<u32> = Global::new(0x00cc_caa8);
    /// `g_ShadowManager` (`ShadowManager_getInstance`).
    pub const SHADOW: Global<u32> = Global::new(0x00cd_ec28);
    /// `g_ShadowSize` (`ShadowManager_GetShadowSize`).
    pub const SHADOW_SIZE: Global<u32> = Global::new(0x00c3_4408);
    /// `g_SpecularMapParamsManager` (`SpecularMapParamsManager_GetInstance`).
    pub const SPECULAR_MAP_PARAMS: Global<u32> = Global::new(0x00cd_ec1c);
    /// `g_DebugMenuManager` (`DebugMenuManager_GetInstance`).
    pub const DEBUG_MENU: Global<u32> = Global::new(0x00cc_cab0);
    /// `g_GraphicPauseGame` (`graphicCheckPauseGame`), an object pointer.
    pub const PAUSE_GAME: Global<u32> = Global::new(0x00cc_cb00);
    /// `g_DynGeoFillBuffer`: the dynamic geometry fill buffer's state.
    pub const DYN_GEO_FILL_BUFFER: Global<u32> = Global::new(0x00cd_edf0);

    /// A singleton getter: the global's value.
    pub fn instance(m: &impl Mem, g: Global<u32>) -> u32 {
        g.get(m)
    }

    /// `PostProcessTweakUtil_Destroy` (`FUN_00692bc0`) and the other
    /// singleton deleters (`DebugMenuManager`, the pause object): the
    /// virtual destructor `(1)` (deleting), then the global is cleared.
    /// Returns `eax` when an object was deleted (its destructor's), else
    /// `None` (`eax` untouched).
    pub fn destroy_singleton(h: &mut impl Host, g: Global<u32>) -> Option<u32> {
        let obj = g.get(h);
        if obj == 0 {
            return None;
        }
        let r = h.call_virtual(obj, 0, &[1]);
        g.set(h, 0);
        Some(r)
    }

    /// `graphicDynGeoFillBufferIsValid()`: the fill buffer's first word is
    /// set.
    pub fn dyn_geo_fill_buffer_is_valid(m: &impl Mem) -> u32 {
        let p = DYN_GEO_FILL_BUFFER.get(m);
        u32::from(m.read::<u32>(p) != 0)
    }

    // ---- DynGeoMemManager: +0 the current buffer set ----
    // A buffer set: +0 the current buffer, +4 its index, +8 the count,
    // +0x10 the buffers' table, +0x14 the mutex; a buffer: +0 its size,
    // +4 its start, +8 its fill position (an address).

    /// MSVCR110's `printf`, `__iob_func` and `fprintf`, through the
    /// image's import slots.
    pub const PRINTF_SLOT: u32 = 0x00b0_c380;
    /// See [`PRINTF_SLOT`].
    pub const IOB_FUNC_SLOT: u32 = 0x00b0_c350;
    /// See [`PRINTF_SLOT`].
    pub const FPRINTF_SLOT: u32 = 0x00b0_c38c;
    /// `"No buffer memory in this manager.\n"`.
    pub const MSG_NO_BUFFER: u32 = 0x00b4_a6e8;
    /// `"Error! Do not have enough memory in dynamic geometry memory..."`.
    pub const MSG_FULL: u32 = 0x00b4_a658;

    /// Calls the CRT function an import slot holds (cdecl).
    fn call_import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let at = h.rebase(slot);
        let f: u32 = h.read(at);
        h.call(f, Conv::Cdecl, args)
    }

    /// `DynGeoMemManager_getBufferFromDynGeoMemory(this, size)`: `size`
    /// bytes at the current buffer's fill position rounded up to 16; 0
    /// without a set, or (with a message on `stderr`, the fill position
    /// left rounded) when the buffer is full. A set without buffers prints
    /// its message and then faults, as the original.
    pub fn dyn_geo_get_buffer(h: &mut impl Host, this: u32, size: u32) -> u32 {
        let set: u32 = h.read(this);
        if set == 0 {
            return 0;
        }
        let buffer = if h.read::<u32>(set + 8) == 0 {
            let msg = h.rebase(MSG_NO_BUFFER);
            call_import(h, PRINTF_SLOT, &[msg]);
            0
        } else {
            let index: u32 = h.read(set + 4);
            let table: u32 = h.read(set + 0x10);
            h.read(table.wrapping_add(index.wrapping_mul(4)))
        };
        let fill: u32 = h.read(buffer + 8);
        let start = (fill.wrapping_sub(1) & !0xf).wrapping_add(0x10);
        let limit = h.read::<u32>(buffer + 4).wrapping_add(h.read(buffer));
        let end = size.wrapping_add(start);
        h.write(buffer + 8, end);
        if end > limit {
            let msg = h.rebase(MSG_FULL);
            let iob = call_import(h, IOB_FUNC_SLOT, &[]);
            call_import(h, FPRINTF_SLOT, &[iob.wrapping_add(0x40), msg]);
            h.write(buffer + 8, start);
            return 0;
        }
        start
    }

    /// `ffx-graphic`'s 16-byte round-up (`((x - 1) & ~0xf) + 0x10`; 0 stays 0).
    #[must_use]
    pub fn align16(x: u32) -> u32 {
        (x.wrapping_sub(1) & !0xf).wrapping_add(0x10)
    }

    /// `graphicDynGeoMemManagerGetBufferForSegment(data, segment, vertices,
    /// indices)`: one block from `g_DynGeoFillBuffer`'s manager cut into
    /// five arrays (12, 16, 12 and 8 bytes a vertex, 2 an index), each
    /// rounded to 16; their addresses go
    /// into the segment (`+0xc..+0x1c`) and into the current buffer's slot
    /// (`+0x2c + 0x1c * id ..`), with the counts. 0 when the memory is
    /// exhausted, else 1.
    pub fn get_buffer_for_segment<H: Host>(
        h: &mut H,
        data: u32,
        segment: u32,
        vertices: u32,
        indices: u32,
    ) -> u32 {
        let v12 = align16(vertices.wrapping_mul(12));
        let colours = align16(vertices.wrapping_mul(16));
        let uvs = align16(vertices.wrapping_mul(8));
        let size = align16(indices.wrapping_mul(2))
            .wrapping_add(uvs)
            .wrapping_add(v12)
            .wrapping_add(colours)
            .wrapping_add(v12);
        let manager = DYN_GEO_FILL_BUFFER.get(h);
        let base = dyn_geo_get_buffer(h, manager, size);
        if base == 0 {
            return 0;
        }
        let third = base.wrapping_add(colours).wrapping_add(v12);
        let fourth = base
            .wrapping_add(v12)
            .wrapping_add(colours)
            .wrapping_add(v12);
        let fifth = base
            .wrapping_add(uvs)
            .wrapping_add(v12)
            .wrapping_add(colours)
            .wrapping_add(v12);
        let manager = DYN_GEO_FILL_BUFFER.get(h);
        let id = dyn_geo_current_buffer_id(h, manager);
        let seg = segment.wrapping_mul(0x6c);
        let slot = seg.wrapping_add(id.wrapping_mul(0x1c));
        let put = |h: &mut H, offset: u32, v: u32| {
            let segs: u32 = h.read(data + 0x94);
            h.write(segs.wrapping_add(offset), v);
        };
        put(h, slot.wrapping_add(0x34), base);
        put(h, slot.wrapping_add(0x3c), base.wrapping_add(v12));
        put(h, slot.wrapping_add(0x38), third);
        put(h, slot.wrapping_add(0x40), fourth);
        put(h, slot.wrapping_add(0x44), fifth);
        put(h, seg + 0xc, base);
        put(h, seg + 0x14, base.wrapping_add(v12));
        put(h, seg + 0x10, third);
        put(h, seg + 0x18, fourth);
        put(h, seg + 0x1c, fifth);
        put(h, seg + 8, indices);
        put(h, seg + 4, vertices);
        put(h, slot.wrapping_add(0x30), indices);
        put(h, slot.wrapping_add(0x2c), vertices);
        1
    }

    /// `DynGeoMemManager_getCurrentBufferId(this)`: the set's index, -1
    /// without one.
    pub fn dyn_geo_current_buffer_id(m: &impl Mem, this: u32) -> u32 {
        match m.read::<u32>(this) {
            0 => u32::MAX,
            set => m.read(set + 4),
        }
    }

    /// `DynGeoMemManager_clearDynGeoMemory(this)`: the current buffer's
    /// fill position back to its start. Returns `eax` when it did
    /// (`None`: untouched).
    pub fn dyn_geo_clear(m: &mut impl Mem, this: u32) -> Option<u32> {
        let set: u32 = m.read(this);
        if set == 0 {
            return None;
        }
        let index: u32 = m.read(set + 4);
        let slot = m
            .read::<u32>(set + 0x10)
            .wrapping_add(index.wrapping_mul(4));
        let buffer: u32 = m.read(slot);
        let start: u32 = m.read(buffer + 4);
        m.write(set, start);
        let buffer: u32 = m.read(slot);
        let start: u32 = m.read(buffer + 4);
        m.write(buffer + 8, start);
        Some(start)
    }

    /// `DynGeoMemManager_selectBuffer(this, i)` (`FUN_006dec90`): the
    /// current word becomes word `i + 1`; returns it.
    pub fn dyn_geo_select(m: &mut impl Mem, this: u32, i: u32) -> u32 {
        let v: u32 = m.read(this.wrapping_add(i.wrapping_mul(4)).wrapping_add(4));
        m.write(this, v);
        v
    }

    // ---- TextureAnimationManager ----

    /// `TextureAnimationManager_getMaterialByLayNum(this, lay)`: material
    /// `+8 + +0xc + lay` of the table at `+0x78` (its count at `+0x74`,
    /// bit 31 a flag), 0 past the end.
    pub fn material_by_lay_num(m: &impl Mem, this: u32, lay: u32) -> u32 {
        let count = m.read::<u32>(this + 0x74) & 0x7fff_ffff;
        let i = m
            .read::<u32>(this + 0xc)
            .wrapping_add(m.read(this + 8))
            .wrapping_add(lay);
        if i < count {
            m.read(m.read::<u32>(this + 0x78).wrapping_add(i.wrapping_mul(4)))
        } else {
            0
        }
    }

    /// `TextureAnimation_next(this)` (`FUN_006b22c0`, with `step` 1) and
    /// `TextureAnimation_prev` (`FUN_006b2380`, `step` -1): the frame
    /// (`+0x18`) moves by `step` modulo the frame count (`+8`, bit 31 a
    /// flag); returns the quotient (`eax`; `edx` is the new frame). With a
    /// count of 0 the original faults (`div`); this leaves the frame and
    /// returns 0.
    pub fn step_frame(m: &mut impl Mem, this: u32, step: u32) -> u32 {
        let count = m.read::<u32>(this + 8) & 0x7fff_ffff;
        let v = m
            .read::<u32>(this + 0x18)
            .wrapping_add(step)
            .wrapping_add(count);
        let (Some(q), Some(r)) = (v.checked_div(count), v.checked_rem(count)) else {
            return 0;
        };
        m.write(this + 0x18, r);
        q
    }

    /// `TextureAnimationInfoChar_active(this)` (0) and `_deactive` (1):
    /// the byte `+0x34`.
    pub fn set_inactive(m: &mut impl Mem, this: u32, inactive: bool) {
        m.write(this + 0x34, u8::from(inactive));
    }

    /// The `thiscall` getters of two floats (`FUN_006b1cc0`: `+0x24`,
    /// `FUN_006b1ce0`: `+0x2c`) into `out`, through the x87. Returns `out`.
    pub fn get_float_pair(m: &mut impl Mem, this: u32, offset: u32, out: u32) -> u32 {
        for i in 0..2 {
            let v: u32 = m.read(this + offset + 4 * i);
            m.write(out + 4 * i, x87::round_trip(f32::from_bits(v)).to_bits());
        }
        out
    }

    /// The `thiscall` setters of one float (`fld`/`fstp`).
    pub fn set_float(m: &mut impl Mem, this: u32, offset: u32, v: f32) {
        m.write(this + offset, x87::round_trip(v).to_bits());
    }

    /// `Distortion_updateDistortionBlur(this, v)`: the float `+0x78`;
    /// returns 0.
    pub fn distortion_update_blur(m: &mut impl Mem, this: u32, v: f32) -> u32 {
        set_float(m, this, 0x78, v);
        0
    }

    /// `BlurEffect_activate(this)`: active (`+0xc`), weight 0 (`+0x84`),
    /// state 2 (`+0x10`).
    pub fn blur_activate(m: &mut impl Mem, this: u32) {
        m.write(this + 0xc, 1u8);
        m.write(this + 0x84, 0u32);
        m.write(this + 0x10, 2u32);
    }

    /// `BlurEffect_setTarget(this, v)` (`FUN_006c4a90`): `+0x2c` and the
    /// changed flag `+0x28`. Returns `v`.
    pub fn set_with_flag(m: &mut impl Mem, this: u32, v: u32) -> u32 {
        m.write(this + 0x2c, v);
        m.write(this + 0x28, 1u8);
        v
    }

    /// `ShadowManager_clearCascades(this)` (`FUN_006d1780`): 64 words from
    /// `+0xfe8` to 0. Returns 0 (`eax`).
    pub fn clear_words(m: &mut impl Mem, this: u32) -> u32 {
        m.write_bytes(this + 0xfe8, &[0; 0x100]);
        0
    }

    /// `ShadowManager_getCascade(this, i)` (`FUN_006d22e0`): word `i` from
    /// `+0x60`.
    pub fn word_at(m: &impl Mem, this: u32, base: u32, i: u32) -> u32 {
        m.read(this.wrapping_add(base).wrapping_add(i.wrapping_mul(4)))
    }

    /// `PostProcessManager_isEffectAllowed(id)` (`FUN_00698730`): false for
    /// the effects 0x17 and 0x15c; in `al`.
    #[must_use]
    pub fn effect_allowed(id: u32) -> u8 {
        u8::from(id != 0x17 && id != 0x15c)
    }

    /// `ShadowManager_RemoveFromShadowMap(this, obj)`: the first of the 8 x 8
    /// shadow-map slots (`+0xde8`) holding `obj` is freed: its word, its
    /// count (`+0x29e8`) and its flag byte (`+0x2ae8`); with a count above
    /// 1 also the flag bytes at `+1`, `+8` and `+9` from it. Returns 0
    /// (`al`).
    pub fn remove_from_shadow_map(m: &mut impl Mem, this: u32, obj: u32) -> u8 {
        for i in 0..64 {
            if m.read::<u32>(this + 0xde8 + 4 * i) == obj {
                if m.read::<i32>(this + 0x29e8 + 4 * i) > 1 {
                    m.write(this + 0x2ae9 + i, 0u8);
                    m.write(this + 0x2af0 + i, 0u8);
                    m.write(this + 0x2af1 + i, 0u8);
                }
                m.write(this + 0x2ae8 + i, 0u8);
                m.write(this + 0x29e8 + 4 * i, 0u32);
                m.write(this + 0xde8 + 4 * i, 0u32);
                return 0;
            }
        }
        0
    }

    /// `DynGeoTree_Find(this, &key)` (`FUN_006dedd0`): a search tree whose
    /// head is `this` (root at `+0xc`); nodes `+0` lower, `+4` higher,
    /// `+0xc` the key. The node, else 0.
    pub fn tree_find(m: &impl Mem, this: u32, key_ptr: u32) -> u32 {
        let key: u32 = m.read(key_ptr);
        let mut node: u32 = m.read(this + 0xc);
        while node != this {
            let k: u32 = m.read(node + 0xc);
            if k == key {
                return node;
            }
            node = m.read(node + if key < k { 0 } else { 4 });
        }
        0
    }

    /// `DynGeoTree_Insert(this, node)` (`FUN_006dee50`): links `node` (key
    /// at `+0xc`) under the tree whose head is `this`: its children become
    /// the head, its `+8` (parent, bit 0 a colour) gets bit 0 set, then the
    /// parent. Returns the node already holding the key (leaving the new
    /// one half-linked, as the original), else 0.
    pub fn tree_insert(m: &mut impl Mem, this: u32, node: u32) -> u32 {
        let tag: u32 = m.read(node + 8);
        m.write(node + 8, tag.wrapping_add(1 - (tag & 1)));
        m.write(node, this);
        m.write(node + 4, this);
        let key: u32 = m.read(node + 0xc);
        // an empty tree hangs the node from the root word itself
        let mut parent = this + 0xc;
        let mut right = false;
        let mut cur: u32 = m.read(this + 0xc);
        while cur != this {
            parent = cur;
            let k: u32 = m.read(cur + 0xc);
            if k == key {
                return cur;
            }
            right = key > k;
            cur = m.read(cur + if right { 4 } else { 0 });
        }
        let tag: u32 = m.read(node + 8);
        m.write(node + 8, (tag & 1).wrapping_add(parent));
        m.write(parent + if right { 4 } else { 0 }, node);
        0
    }

    /// Whether the C strings at `a` and `b` are equal (the inlined
    /// `strcmp` of the originals, two bytes at a time; only equality is
    /// used).
    #[allow(clippy::many_single_char_names)]
    pub fn str_eq(m: &impl Mem, a: u32, b: u32) -> bool {
        let mut i = 0u32;
        loop {
            let (x, y): (u8, u8) = (m.read(a.wrapping_add(i)), m.read(b.wrapping_add(i)));
            if x != y {
                return false;
            }
            if x == 0 {
                return true;
            }
            i += 1;
        }
    }

    /// The `PString` at `at`: its pointer without the flag bit.
    fn pstring(m: &impl Mem, at: u32) -> u32 {
        let p: u32 = m.read(at);
        p.wrapping_sub(p & 1)
    }

    /// `TextureAnimation_FindByName(this, name)` (`FUN_006b1d10`): the first
    /// of the `+0x7c` entries (`0x20` bytes from `+0x80`, the name a
    /// `PString` at `+4`) named `name` (the same pointer, or equal text
    /// when `name` is set); 0 when none is.
    pub fn find_by_name(m: &impl Mem, this: u32, name: u32) -> u32 {
        let count = m.read::<u32>(this + 0x7c) & 0x7fff_ffff;
        let base: u32 = m.read(this + 0x80);
        for i in 0..count {
            let entry = base.wrapping_add(i * 0x20);
            let own = pstring(m, entry + 4);
            if own != 0 && (own == name || (name != 0 && str_eq(m, own, name))) {
                return entry;
            }
        }
        0
    }

    /// `TextureAnimation_FindMaterial(this, list, index, name)`
    /// (`FUN_006b1e40`): slot `index` of `list` (`+8` count with a flag
    /// bit, `+0xc` the slots; -1 for none) looked up in the first of the
    /// `+0x64` texture sets (`0x20` bytes from `+0x68`: `+0` count, `+4`
    /// materials, `+0x18` the name) named `name`; 0 when anything is
    /// missing.
    pub fn find_material(m: &impl Mem, this: u32, list: u32, index: u32, name: u32) -> u32 {
        if list == 0 || index >= m.read::<u32>(list + 8) & 0x7fff_ffff {
            return 0;
        }
        let slot: u32 = m.read(
            m.read::<u32>(list + 0xc)
                .wrapping_add(index.wrapping_mul(4)),
        );
        if slot == u32::MAX {
            return 0;
        }
        let count = m.read::<u32>(this + 0x64) & 0x7fff_ffff;
        let base: u32 = m.read(this + 0x68);
        for i in 0..count {
            let set = base.wrapping_add(i * 0x20);
            let own = pstring(m, set + 0x18);
            if own != 0 && str_eq(m, own, name) {
                if slot >= m.read::<u32>(set) & 0x7fff_ffff {
                    return 0;
                }
                return m.read(m.read::<u32>(set + 4).wrapping_add(slot.wrapping_mul(4)));
            }
        }
        0
    }

    /// `PostProcessManager_setFocusDepth(this, which, depth)`: the near (0,
    /// `+0x123c8`) or far (1, `+0x123cc`) focus depth, through the x87.
    /// Returns `eax` (`which - 1`, 0 for 0).
    pub fn set_focus_depth(m: &mut impl Mem, this: u32, which: u32, depth: f32) -> u32 {
        match which {
            0 => set_float(m, this, 0x123c8, depth),
            1 => set_float(m, this, 0x123cc, depth),
            _ => {}
        }
        which.saturating_sub(1)
    }

    /// `SpecularMapParamsManager_needDrawSpecularPass(this)`: whether one of
    /// the four maps (`+0`) is set and not hidden (`+0x5b0 + i`).
    pub fn need_draw_specular_pass(m: &impl Mem, this: u32) -> u32 {
        u32::from(
            (0..4).any(|i| m.read::<u32>(this + 4 * i) != 0 && m.read::<u8>(this + 0x5b0 + i) == 0),
        )
    }

    /// `ShadowManager_AlreadyinShadowMap(this, obj, &col, &row)`: where
    /// `obj` sits among the 8 x 8 shadow-map slots; 1 (`al`) and the
    /// position when found, else 0.
    pub fn already_in_shadow_map(m: &mut impl Mem, this: u32, obj: u32, col: u32, row: u32) -> u8 {
        for r in 0..8u32 {
            for c in 0..8u32 {
                if m.read::<u32>(this + 0xde8 + 4 * (8 * r + c)) == obj {
                    m.write(row, r);
                    m.write(col, c);
                    return 1;
                }
            }
        }
        0
    }

    /// The cursor checks (`FUN_00660240`, `FUN_00660270`, `FUN_006602a0`,
    /// `FUN_006602d0`, `FUN_00660300`, `FUN_006b28f0`: copies of one
    /// template): a cursor (`this`) over a list (`+0x18`, first node
    /// `+0x2c`, nodes linked at `+0x40`) pointing at `+0x1c` is valid when
    /// it has no list, or when its node is still in the list and the list's
    /// `+0x40` is set. In `al`.
    pub fn cursor_valid(m: &impl Mem, this: u32) -> u8 {
        let list: u32 = m.read(this + 0x18);
        if list == 0 {
            return 1;
        }
        let at: u32 = m.read(this + 0x1c);
        let mut node: u32 = m.read(list + 0x2c);
        while node != 0 {
            if node == at {
                return u8::from(m.read::<u32>(list + 0x40) != 0);
            }
            node = m.read(node + 0x40);
        }
        0
    }

    /// `PostProcessManager_setAllSwitches(this, v)` (`FUN_006989c0`): the four
    /// bytes `+0x123b0..+0x123b3`. Returns `v` (`al`).
    pub fn set_all_switches(m: &mut impl Mem, this: u32, v: u32) -> u8 {
        #[allow(clippy::cast_possible_truncation)] // the byte argument
        let b = v as u8;
        for i in 0..4 {
            m.write(this + 0x123b0 + i, b);
        }
        b
    }

    /// `PostProcessManager_resetCrossFade(this)` (`FUN_00698f70`): unless
    /// `+0x123a0` is set, and while the manager's `+0x123a8` is 1, clears
    /// `+0x1239c` and the float `+0x12398`.
    pub fn reset_cross_fade(m: &mut impl Mem, this: u32) {
        if m.read::<u8>(this + 0x123a0) == 0 {
            let pp = POST_PROCESS.get(m);
            if m.read::<u8>(pp + 0x123a8) == 1 {
                m.write(this + 0x1239c, 0u32);
                m.write(this + 0x12398, 0u32);
            }
        }
    }

    /// `BlurEffect_SetFloats74(this, a, b, c, d)` (`FUN_006c46e0`): four
    /// floats from `+0x74`, through the x87.
    pub fn set_floats4(m: &mut impl Mem, this: u32, v: [f32; 4]) {
        for (i, x) in (0u32..).zip(v) {
            set_float(m, this, 0x74 + 4 * i, x);
        }
    }

    /// `Distortion_updateDistortionVertex(this, i, j, x, y, z, w, u, v)`:
    /// vertex `9i - j` (16 bytes from `*(+0x6c) + 0x80`) gets `x, y, z, w`
    /// and its UV (8 bytes from `*(+0x70) + 0x40`) `u, 1 - v`. Returns 0.
    #[allow(clippy::many_single_char_names)]
    pub fn distortion_update_vertex(
        m: &mut impl Mem,
        this: u32,
        i: u32,
        j: u32,
        f: [f32; 6],
    ) -> u32 {
        let k = i.wrapping_mul(9).wrapping_sub(j);
        let verts: u32 = m.read(this + 0x6c);
        let at = verts.wrapping_add(k.wrapping_add(8).wrapping_mul(16));
        for (n, x) in (0u32..).zip(&f[..4]) {
            m.write(at + 4 * n, x87::round_trip(*x).to_bits());
        }
        let uvs: u32 = m.read(this + 0x70);
        let at = uvs.wrapping_add(k.wrapping_mul(8)).wrapping_add(0x40);
        m.write(at, x87::round_trip(f[4]).to_bits());
        let v = x87::store(x87::sub(1.0, x87::load(f[5].to_bits())));
        m.write(at + 4, v);
        0
    }
}
