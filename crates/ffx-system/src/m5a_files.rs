//! M5 `system-a` (round 3): the archive streams' read-request set-up
//! (`FileSys_NewReadRequest` and the slot set-up `0x0061b740`), the
//! `VFile` constructor, the string-resource search over the ATL modules,
//! the member-lookup caches of the cluster fixups, two sort predicates,
//! unwinding destructors of the red-black-tree sets, the free-camera
//! world matrix's constructor and its normalised axis, and the Lua vector
//! negation.

/// The originals over game memory and the original callees.
pub mod bridge {
    #![allow(clippy::many_single_char_names)] // the originals' key words a, b, c
    #![allow(clippy::cast_possible_truncation)] // 64-bit offsets split into their words

    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// `operator new` / `operator_new_006304c0` / `operator delete` /
    /// `operator delete[]` (cdecl).
    pub const OPERATOR_NEW: u32 = 0x0063_0310;
    pub const NEW_ARRAY: u32 = 0x0063_04c0;
    pub const OPERATOR_DELETE: u32 = 0x0063_0390;
    pub const DELETE_ARRAY: u32 = 0x0063_0540;
    /// `Phyre_PhyreFree` (cdecl).
    pub const PHYRE_FREE: u32 = 0x0042_fb20;
    /// `Phyre_PRedBlackTree_clear`, `Phyre_PRedBlackTree_orphanAllContents`,
    /// `Phyre_PFreeList_reset`, `Phyre_PFreeList_dtor_PFreeList` (thiscall).
    pub const TREE_CLEAR: u32 = 0x0057_4760;
    pub const TREE_ORPHAN_ALL: u32 = 0x0057_4970;
    pub const FREE_LIST_RESET: u32 = 0x0049_cc10;
    pub const FREE_LIST_DTOR: u32 = 0x0049_c5b0;
    /// `Phyre_PClassView_findMember_0047d3e0` and `Class_FindMemberByOffset`
    /// (thiscall, `ret 0x10`).
    pub const FIND_MEMBER: u32 = 0x0047_d3e0;
    pub const FIND_MEMBER_BY_OFFSET: u32 = 0x0047_d5f0;
    /// `BigFileStream_claimSlot_0061b4a0` (thiscall, `ret 4`).
    pub const CLAIM_SLOT: u32 = 0x0061_b4a0;
    /// `0x0061b740` (thiscall, `ret 0x10`).
    pub const SET_UP_SLOT: u32 = 0x0061_b740;
    /// `0x009d9db5(module_list; i)` (thiscall, `ret 4`): the `i`-th ATL
    /// resource module, 0 past the last; the list is `0x0193fcc4`.
    pub const RESOURCE_MODULE: u32 = 0x009d_9db5;
    pub const ATL_MODULES: u32 = 0x0193_fcc4;
    /// `KERNEL32!FindResourceW` (stdcall).
    pub const FIND_RESOURCE_IAT: u32 = 0x00b0_c17c;
    /// `AtlStringResourceImage_0061b3c0` (cdecl).
    pub const STRING_RESOURCE: u32 = 0x0061_b3c0;
    /// `PMatrix4_toWorldMatrix3x4` / `PMatrix4_FromMatrix3x4_00451080`
    /// (thiscall, `ret 4`).
    pub const TO_WORLD_MATRIX: u32 = 0x0045_1190;
    pub const FROM_MATRIX3X4: u32 = 0x0045_1080;
    /// The world-matrix controller's vtable.
    pub const CONTROLLER_VTABLE: u32 = 0x00b3_f674;
    /// Lua: `lua_touserdata`, `0x0094b0d0` (a type test), `lua_getfield`,
    /// `0x0094b8b0` (a comparison), `lua_settop`, `lua_newuserdata`,
    /// `0x0094bce0` (`lua_setmetatable`), all cdecl.
    pub const LUA_TOUSERDATA: u32 = 0x0094_c1b0;
    pub const LUA_IS_USERDATA: u32 = 0x0094_b0d0;
    pub const LUA_GETFIELD: u32 = 0x0094_b030;
    pub const LUA_COMPARE: u32 = 0x0094_b8b0;
    pub const LUA_SETTOP: u32 = 0x0094_bdc0;
    pub const LUA_NEWUSERDATA: u32 = 0x0094_b450;
    pub const LUA_SETMETATABLE: u32 = 0x0094_bce0;
    /// The registry index and the vector type's metatable name.
    pub const LUA_REGISTRY: u32 = 0xfff0_b9d8;
    pub const VECTOR4_NAME: u32 = 0x00b3_07b4;

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    fn this_call(h: &mut impl Host, sym: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Thiscall(this), args)
    }

    /// `0x00573b70(L)` (cdecl, a Lua C function): the vector at stack
    /// index 1, when that is userdata of the vector type (its metatable
    /// equals the registry's), pushed negated as a new vector userdata
    /// (16 bytes, the type's metatable set); 1. Else 0. Each component
    /// goes through `fld`/`fchs`/`fstp` (a signalling NaN comes out
    /// quiet).
    pub fn lua_vector_negate(h: &mut impl Host, lua: u32) -> u32 {
        let v = cdecl(h, LUA_TOUSERDATA, &[lua, 1]);
        if v == 0 || cdecl(h, LUA_IS_USERDATA, &[lua, 1]) == 0 {
            return 0;
        }
        let name = h.rebase(VECTOR4_NAME);
        cdecl(h, LUA_GETFIELD, &[lua, LUA_REGISTRY, name]);
        let same = cdecl(h, LUA_COMPARE, &[lua, 0xffff_ffff, 0xffff_fffe]) != 0;
        cdecl(h, LUA_SETTOP, &[lua, 0xffff_fffd]);
        if !same {
            return 0;
        }
        let out = cdecl(h, LUA_NEWUSERDATA, &[lua, 0x10]);
        if out != 0 {
            cdecl(h, LUA_GETFIELD, &[lua, LUA_REGISTRY, name]);
            cdecl(h, LUA_SETMETATABLE, &[lua, 0xffff_fffe]);
        }
        let mut neg = [0u32; 4];
        for (k, n) in (0u32..).zip(neg.iter_mut()) {
            *n = x87::store(-x87::load(h.read(v.wrapping_add(4 * k))));
        }
        for (k, n) in (0u32..).zip(neg) {
            h.write(out.wrapping_add(4 * k), n);
        }
        1
    }

    /// A member-lookup cache (`+0` the class view, `+4`/`+8`/`+0xc` the
    /// last key, `+0x10` the member, `+0x14` the cursor step, `+0x1c`
    /// hits, `+0x20` misses).
    ///
    /// `0x0057a200(this; &cursor, a, b, c)` (thiscall, `ret 0x10`): a hit
    /// (`+4` = `b`, `+0xc` = `c`, `+8` = `a`) moves the cursor on by the
    /// step and counts; a miss asks `findMember(view; b, &cursor, a, c)`
    /// and, when it finds one, caches it with the step the lookup moved
    /// the cursor. Returns the member, or 0.
    pub fn cached_find_member(
        h: &mut impl Host,
        this: u32,
        cursor: u32,
        a: u32,
        b: u32,
        c: u32,
    ) -> u32 {
        let at = |off: u32| this.wrapping_add(off);
        if h.read::<u32>(at(4)) == b && h.read::<u32>(at(0xc)) == c && h.read::<u32>(at(8)) == a {
            let step: u32 = h.read(at(0x14));
            let p: u32 = h.read(cursor);
            h.write(cursor, p.wrapping_add(step));
            let hits: u32 = h.read(at(0x1c));
            h.write(at(0x1c), hits.wrapping_add(1));
            return h.read(at(0x10));
        }
        let old: u32 = h.read(cursor);
        let view: u32 = h.read(this);
        let m = this_call(h, FIND_MEMBER, view, &[b, cursor, a, c]);
        if m == 0 {
            return 0;
        }
        h.write(at(0x10), m);
        h.write(at(4), b);
        h.write(at(0xc), c);
        h.write(at(8), a);
        let p: u32 = h.read(cursor);
        let misses: u32 = h.read(at(0x20));
        h.write(at(0x20), misses.wrapping_add(1));
        h.write(at(0x14), p.wrapping_sub(old));
        h.read(at(0x10))
    }

    /// `Fixup_FindMember(this; index, &cursor, a, &element, b)` (thiscall,
    /// `ret 0x14`): an index below the class's member count gives that
    /// 32-byte member entry (when not null); else the cache (`+4` = `b`,
    /// `+8` = `a`: the element (`+0x18`) out and the step); a miss asks
    /// `Class_FindMemberByOffset(view; b, &cursor, a, &element)` and
    /// caches the member, its element and step. Returns the member, or 0.
    pub fn fixup_find_member(
        h: &mut impl Host,
        this: u32,
        index: u32,
        cursor: u32,
        a: u32,
        element: u32,
        b: u32,
    ) -> u32 {
        let at = |off: u32| this.wrapping_add(off);
        let view: u32 = h.read(this);
        let class: u32 = h.read(view);
        if index < h.read::<u32>(class.wrapping_add(0x10)) {
            let members: u32 = h.read(view.wrapping_add(0x18));
            let m = (index << 5).wrapping_add(members);
            if m != 0 {
                return m;
            }
        }
        if h.read::<u32>(at(4)) == b && h.read::<u32>(at(8)) == a {
            let e: u32 = h.read(at(0x18));
            h.write(element, e);
            let step: u32 = h.read(at(0x14));
            let p: u32 = h.read(cursor);
            h.write(cursor, p.wrapping_add(step));
            let hits: u32 = h.read(at(0x1c));
            h.write(at(0x1c), hits.wrapping_add(1));
            return h.read(at(0x10));
        }
        let old: u32 = h.read(cursor);
        let m = this_call(h, FIND_MEMBER_BY_OFFSET, view, &[b, cursor, a, element]);
        if m == 0 {
            return 0;
        }
        h.write(at(0x10), m);
        h.write(at(4), b);
        let e: u32 = h.read(element);
        h.write(at(0x18), e);
        h.write(at(8), a);
        let p: u32 = h.read(cursor);
        let misses: u32 = h.read(at(0x20));
        h.write(at(0x20), misses.wrapping_add(1));
        h.write(at(0x14), p.wrapping_sub(old));
        h.read(at(0x10))
    }

    /// `0x0060ac50` / `0x0060ad00(a, b)` (cdecl sort predicates): whether
    /// record `a` sorts before `b`: the words at `fields` compared in
    /// order, unsigned (the originals split the first by its top bit,
    /// which orders the same). 1 or 0.
    pub fn record_less(h: &impl Host, fields: &[u32], a: u32, b: u32) -> u32 {
        for &off in fields {
            let x: u32 = h.read(a.wrapping_add(off));
            let y: u32 = h.read(b.wrapping_add(off));
            if x != y {
                return u32::from(x < y);
            }
        }
        0
    }

    /// `0x0060ac50`'s fields: `+0`, `+0xc`, `+8`, `+4`.
    pub const LESS_60AC50: [u32; 4] = [0, 0xc, 8, 4];
    /// `0x0060ad00`'s fields: `+0`, `+8` .. `+0x18`, `+4`.
    pub const LESS_60AD00: [u32; 7] = [0, 8, 0xc, 0x10, 0x14, 0x18, 4];

    /// `0x0060d460(this)` (`ecx`; unwinding): `Phyre_PRedBlackTree_clear`
    /// twice (the derived and the base destructor).
    pub fn tree_dtor(h: &mut impl Host, this: u32) {
        this_call(h, TREE_CLEAR, this, &[]);
        this_call(h, TREE_CLEAR, this, &[]);
    }

    /// `0x0060d550(this)` (`ecx`; unwinding): the tree's contents orphaned,
    /// its free list (`+0x18`) reset and destroyed, the tree cleared
    /// twice.
    pub fn pooled_tree_dtor(h: &mut impl Host, this: u32) {
        this_call(h, TREE_ORPHAN_ALL, this, &[]);
        let pool = this.wrapping_add(0x18);
        this_call(h, FREE_LIST_RESET, pool, &[]);
        this_call(h, FREE_LIST_DTOR, pool, &[]);
        tree_dtor(h, this);
    }

    /// `0x0060d5d0(this)` (`ecx`; unwinding): [`pooled_tree_dtor`] of the
    /// tree at `+0x14`, then the block at `+0x10` freed
    /// (`Phyre_PhyreFree`) when set and `+0xc`'s top bit is clear (an
    /// owned array); `+0x10`, `+0xc` cleared.
    pub fn owned_array_dtor(h: &mut impl Host, this: u32) {
        pooled_tree_dtor(h, this.wrapping_add(0x14));
        if h.read::<u32>(this.wrapping_add(0xc)) >> 31 == 0 {
            let p: u32 = h.read(this.wrapping_add(0x10));
            if p != 0 {
                cdecl(h, PHYRE_FREE, &[p]);
            }
        }
        h.write(this.wrapping_add(0x10), 0u32);
        h.write(this.wrapping_add(0xc), 0u32);
    }

    /// The frame `0x0061a690` needs: an identity matrix at `ebp - 0x40`.
    pub const MATRIX_FRAME: u32 = 0x40;

    /// `0x0061a690(this)` (thiscall): the world-matrix controller's
    /// constructor: the vtable, `+4` the identity (`PMatrix4_toWorldMatrix3x4`
    /// of the identity built at `ebp - 0x40`), `+0x34..=+0x70` the
    /// identity 4x4 (floats; `+0x34`, `+0x48`, `+0x5c`, `+0x70` 1.0),
    /// `+0x74`, `+0x78` 0. Returns `this`.
    pub fn controller_ctor(h: &mut impl Host, ebp: u32, this: u32) -> u32 {
        let m = ebp.wrapping_sub(MATRIX_FRAME);
        for k in 0..16u32 {
            let one = k % 5 == 0;
            h.write(m.wrapping_add(4 * k), if one { 1f32 } else { 0f32 });
        }
        let vt = h.rebase(CONTROLLER_VTABLE);
        h.write(this, vt);
        this_call(h, TO_WORLD_MATRIX, this.wrapping_add(4), &[m]);
        h.write(this.wrapping_add(0x74), 0u32);
        h.write(this.wrapping_add(0x78), 0u32);
        for k in 0..16u32 {
            let one = k % 5 == 0;
            h.write(
                this.wrapping_add(0x34 + 4 * k),
                if one { 1f32 } else { 0f32 },
            );
        }
        this
    }

    /// The frame `0x0061a770` needs: a `PMatrix4` at `ebp - 0x44`.
    pub const AXIS_FRAME: u32 = 0x44;

    /// `0x0061a770(this; out)` (thiscall, `ret 4`): the 4x4 of the 3x4 at
    /// `this + 4` (`PMatrix4_FromMatrix3x4` into `ebp - 0x44`), its third
    /// row's `x, y, z` normalised into `out` (each square, the running
    /// sum, the root and its inverse rounded to `f32`, as the original
    /// stores them). Returns `out`.
    pub fn controller_axis(h: &mut impl Host, ebp: u32, this: u32, out: u32) -> u32 {
        let m = ebp.wrapping_sub(AXIS_FRAME);
        this_call(h, FROM_MATRIX3X4, m, &[this.wrapping_add(4)]);
        let x = x87::load(h.read(m.wrapping_add(0x20)));
        let y = x87::load(h.read(m.wrapping_add(0x24)));
        let z = x87::load(h.read(m.wrapping_add(0x28)));
        let f = |v: f64| x87::load(x87::store(v));
        let sum = f(x87::mul(x, x));
        let sum = f(x87::add(sum, x87::mul(y, y)));
        let sum = f(x87::add(sum, x87::mul(z, z)));
        let len = f(x87::sqrt(sum));
        let inv = f(x87::div(1.0, len));
        h.write(out, x87::store(x87::mul(x, inv)));
        h.write(out.wrapping_add(4), x87::store(x87::mul(y, inv)));
        h.write(out.wrapping_add(8), x87::store(x87::mul(inv, z)));
        out
    }

    /// `VFile_VFile(this)` (thiscall): byte `+0` and `+8..=+0x14`
    /// cleared, `+0x18` a new 24-byte buffer record (a 64 KB block, its
    /// counters 0), or 0 when that allocation fails. Returns `this`.
    pub fn vfile_ctor(h: &mut impl Host, this: u32) -> u32 {
        h.write(this, 0u8);
        for off in [8u32, 0xc, 0x10, 0x14] {
            h.write(this.wrapping_add(off), 0u32);
        }
        let rec = cdecl(h, OPERATOR_NEW, &[0x18]);
        if rec == 0 {
            h.write(this.wrapping_add(0x18), 0u32);
            return this;
        }
        let block = cdecl(h, NEW_ARRAY, &[0x10000]);
        h.write(rec, block);
        for off in [0x10u32, 0x14, 8, 0xc] {
            h.write(rec.wrapping_add(off), 0u32);
        }
        h.write(this.wrapping_add(0x18), rec);
        this
    }

    /// `0x0061b030(this)` (`ecx`): the three buffer records at `+8`,
    /// `+0xc`, `+0x10`, each when set: its block (`+4`) freed with
    /// `operator delete[]` and cleared, then the record deleted.
    pub fn vfile_free_buffers(h: &mut impl Host, this: u32) {
        for off in [8u32, 0xc, 0x10] {
            let rec: u32 = h.read(this.wrapping_add(off));
            if rec == 0 {
                continue;
            }
            let block: u32 = h.read(rec.wrapping_add(4));
            if block != 0 {
                cdecl(h, DELETE_ARRAY, &[block]);
                h.write(rec.wrapping_add(4), 0u32);
            }
            cdecl(h, OPERATOR_DELETE, &[rec]);
        }
    }

    /// `0x0061b270(id)` (cdecl): string `id` from the first ATL resource
    /// module that has its block (`FindResourceW(module, id / 16 + 1,
    /// RT_STRING)` and `AtlStringResourceImage_0061b3c0`), or 0.
    pub fn find_string_resource(h: &mut impl Host, id: u32) -> u32 {
        let list = h.rebase(ATL_MODULES);
        let mut found = 0;
        let mut i = 0u32;
        let mut module = this_call(h, RESOURCE_MODULE, list, &[i]);
        while module != 0 && found == 0 {
            let block = ((id >> 4).wrapping_add(1)) & 0xffff;
            let f: u32 = h.read(h.rebase(FIND_RESOURCE_IAT));
            let info = h.call(f, Conv::Stdcall, &[module, block, 6]);
            if info != 0 {
                found = cdecl(h, STRING_RESOURCE, &[module, info, id]);
            }
            i = i.wrapping_add(1);
            module = this_call(h, RESOURCE_MODULE, list, &[i]);
        }
        found
    }

    /// `0x0061b740(this; lo, hi, size, total)` (thiscall, `ret 0x10`): a
    /// read slot claimed (`claimSlot(this + 4; -1)`) and set up for a read
    /// of `total` bytes in `size`-byte blocks starting `hi:lo` into the
    /// first: its use count up, `+0x50` = 0; with an offset, the head
    /// record (`+8`: flag, length, offset) gets the head's length (`size`
    /// less the offset, at most `total`); the body record (`+0xc`) is
    /// flagged when at least a block is left, the tail record (`+0x10`)
    /// gets the rest. Returns the slot. (A `size` of 0, which the original
    /// divides by, gives no tail here; every caller passes 0x10000.)
    pub fn set_up_slot(
        h: &mut impl Host,
        this: u32,
        lo: u32,
        hi: u32,
        size: u32,
        total: u32,
    ) -> u32 {
        let slot = this_call(h, CLAIM_SLOT, this.wrapping_add(4), &[0xffff_ffff]);
        let uses: u32 = h.read(slot.wrapping_add(0x24));
        h.write(slot.wrapping_add(0x24), uses.wrapping_add(1));
        h.write(slot.wrapping_add(0x50), 0u32);
        let mut left = total;
        if hi != 0 || lo != 0 {
            let head: u32 = h.read(slot.wrapping_add(8));
            h.write(head, 1u8);
            let d = u64::from(size).wrapping_sub(u64::from(hi) << 32 | u64::from(lo));
            let first = if d >> 32 != 0 || (d as u32) > total {
                total
            } else {
                d as u32
            };
            let head: u32 = h.read(slot.wrapping_add(8));
            h.write(head.wrapping_add(8), first);
            let head: u32 = h.read(slot.wrapping_add(8));
            h.write(head.wrapping_add(0xc), lo);
            let head: u32 = h.read(slot.wrapping_add(8));
            left = total.wrapping_sub(h.read(head.wrapping_add(8)));
        }
        if left >= size {
            let body: u32 = h.read(slot.wrapping_add(0xc));
            h.write(body, 1u8);
        }
        let rest = left.checked_rem(size).unwrap_or(0);
        if rest.cast_signed() > 0 {
            let tail: u32 = h.read(slot.wrapping_add(0x10));
            h.write(tail, 1u8);
            let tail: u32 = h.read(slot.wrapping_add(0x10));
            h.write(tail.wrapping_add(8), rest);
        }
        slot
    }

    /// `FileSys_NewReadRequest(this; file, length, req, lo, hi, info)`
    /// (thiscall, `ret 0x18`): the read of `length` bytes at `hi:lo` in
    /// the entry (`req + 0xc`): whole 64 KB blocks skipped from the
    /// entry's first block (`+0`) and stored offset (`+0x10`, 64 bits),
    /// each block's packed size from `info + 0x38` (u16, 0 = 64 KB); then
    /// a slot set up ([`set_up_slot`] with the offset left) and filled in:
    /// `+0x18` the file, `+0x3c` the entry, `+0x40` `info`, `+4` the
    /// block, `+0x14` `[req + 0x18]`, `+0x30` the skipped bytes (64 bits),
    /// `+0x38` `length`, `+0x28` the packed offset (64 bits), `+0x44`
    /// `[req + 4]`. Returns the slot.
    #[allow(clippy::too_many_arguments)] // the original's six words and `this`
    pub fn new_read_request(
        h: &mut impl Host,
        this: u32,
        file: u32,
        length: u32,
        req: u32,
        lo: u32,
        hi: u32,
        info: u32,
    ) -> u32 {
        let entry: u32 = h.read(req.wrapping_add(0xc));
        let mut packed = u64::from(h.read::<u32>(entry.wrapping_add(0x14))) << 32
            | u64::from(h.read::<u32>(entry.wrapping_add(0x10)));
        let mut block: u32 = h.read(entry);
        let mut offset = u64::from(hi) << 32 | u64::from(lo);
        if offset >= 0x10000 {
            let sizes: u32 = h.read(info.wrapping_add(0x38));
            let mut at = sizes.wrapping_add(block.wrapping_mul(2));
            while offset >= 0x10000 {
                let n: u16 = h.read(at);
                offset = offset.wrapping_sub(0x10000);
                packed = packed.wrapping_add(if n == 0 { 0x10000 } else { u64::from(n) });
                block = block.wrapping_add(1);
                at = at.wrapping_add(2);
            }
        }
        let (rest_lo, rest_hi) = (offset as u32, (offset >> 32) as u32);
        let slot = this_call(h, SET_UP_SLOT, this, &[rest_lo, rest_hi, 0x10000, length]);
        h.write(slot.wrapping_add(0x18), file);
        let r18: u32 = h.read(req.wrapping_add(0x18));
        h.write(slot.wrapping_add(0x3c), entry);
        h.write(slot.wrapping_add(0x40), info);
        h.write(slot.wrapping_add(4), block);
        let skipped = (u64::from(hi) << 32 | u64::from(lo)).wrapping_sub(offset);
        h.write(slot.wrapping_add(0x14), r18);
        h.write(slot.wrapping_add(0x30), skipped as u32);
        h.write(slot.wrapping_add(0x38), length);
        h.write(slot.wrapping_add(0x28), packed as u32);
        h.write(slot.wrapping_add(0x2c), (packed >> 32) as u32);
        h.write(slot.wrapping_add(0x34), (skipped >> 32) as u32);
        let r4: u32 = h.read(req.wrapping_add(4));
        h.write(slot.wrapping_add(0x44), r4);
        slot
    }
}
