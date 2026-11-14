//! M5 finish, WP5 part 5 (wp5f): small `FFX.exe` functions ported code
//! still called in the original: the graphics teardown's singleton frees
//! (`graphicFinalize`), the debug tweak texts' helpers, the opus tree's
//! clear, the facet list's free, the movie reader's base vtable store,
//! `SqsSePlayState`, and a few one-liners. Read from the machine code
//! (several are not in the dump).

pub mod bridge {
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    fn at(p: u32, o: u32) -> u32 {
        p.wrapping_add(o)
    }

    pub const OPERATOR_DELETE: u32 = 0x0063_0390;
    pub const PHYRE_FREE: u32 = 0x0042_fb20;
    /// `TextUtilEntry_dtor_00692950` (thiscall): the teardowns' member
    /// destructor.
    pub const TEXT_UTIL_ENTRY_DTOR: u32 = 0x0069_2950;
    pub const INPUT_SHUTDOWN_DEVICE: u32 = 0x0063_0b10;

    fn global(h: &impl Host, g: u32) -> u32 {
        h.read(h.rebase(g))
    }

    fn set_global(h: &mut impl Host, g: u32, v: u32) {
        let p = h.rebase(g);
        h.write(p, v);
    }

    /// One of `graphicFinalize`'s singleton frees (`0x006b6180` and six
    /// more): the object in `g`, when there is one, has the member at
    /// `member` destroyed ([`TEXT_UTIL_ENTRY_DTOR`]), is deleted and `g`
    /// cleared.
    #[derive(Clone, Copy, Debug)]
    pub struct Singleton {
        pub global: u32,
        pub member: u32,
    }

    /// `(function, global, member offset)` of the seven frees.
    pub const SINGLETONS: [(u32, Singleton); 7] = [
        (
            0x006b_6180,
            Singleton {
                global: 0x00cc_cae0,
                member: 0x20,
            },
        ),
        (
            0x006b_61b0,
            Singleton {
                global: 0x00cc_cad8,
                member: 0x114,
            },
        ),
        (
            0x006b_61e0,
            Singleton {
                global: 0x00cc_cadc,
                member: 0x18,
            },
        ),
        (
            0x006b_6210,
            Singleton {
                global: 0x00cc_cac4,
                member: 4,
            },
        ),
        (
            0x006b_6240,
            Singleton {
                global: 0x00cc_cad4,
                member: 0x20,
            },
        ),
        (
            0x006b_6270,
            Singleton {
                global: 0x00cc_cae4,
                member: 0x14,
            },
        ),
        (
            0x006b_62a0,
            Singleton {
                global: 0x00cc_cac8,
                member: 4,
            },
        ),
    ];

    /// See [`Singleton`].
    pub fn free_singleton(h: &mut impl Host, s: Singleton) {
        let obj = global(h, s.global);
        if obj == 0 {
            return;
        }
        h.call_sym(TEXT_UTIL_ENTRY_DTOR, Conv::Thiscall(at(obj, s.member)), &[]);
        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[obj]);
        set_global(h, s.global, 0);
    }

    /// `0x006b60f0()`: the singleton at `0x00cccad0`: its member at
    /// `+0x34` destroyed, its vector at `+0x1c` freed (the three words
    /// cleared), deleted, the global cleared.
    pub fn free_singleton_cccad0(h: &mut impl Host) {
        let obj = global(h, 0x00cc_cad0);
        if obj == 0 {
            return;
        }
        h.call_sym(TEXT_UTIL_ENTRY_DTOR, Conv::Thiscall(at(obj, 0x34)), &[]);
        let v: u32 = h.read(at(obj, 0x1c));
        if v != 0 {
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[v]);
            for o in [0x1c, 0x20, 0x24] {
                h.write(at(obj, o), 0u32);
            }
        }
        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[obj]);
        set_global(h, 0x00cc_cad0, 0);
    }

    /// `0x006b62d0()`: the object at `0x00cccac0` deleted through its
    /// vtable (slot 0, `1`) and the global cleared.
    pub fn free_object_cccac0(h: &mut impl Host) {
        let obj = global(h, 0x00cc_cac0);
        if obj == 0 {
            return;
        }
        let vt: u32 = h.read(obj);
        let f: u32 = h.read(vt);
        h.call(f, Conv::Thiscall(obj), &[1]);
        set_global(h, 0x00cc_cac0, 0);
    }

    /// `0x006dc870()`: the block at `0x00cdede8` freed (`PhyreFree`), the
    /// word cleared, then a tail jump to `Input_ShutdownDevice` (its
    /// `eax`); nothing (eax unchanged: 0 here) without a block.
    pub fn free_block_cdede8(h: &mut impl Host) -> u32 {
        let p = global(h, 0x00cd_ede8);
        if p == 0 {
            return 0;
        }
        h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p]);
        set_global(h, 0x00cd_ede8, 0);
        h.call_sym(INPUT_SHUTDOWN_DEVICE, Conv::Cdecl, &[])
    }

    /// `0x0041e6a0(this)` (thiscall): `mkvparser::IMkvReader`'s vtable
    /// stored (the reader's base destructor).
    pub fn mkv_reader_base_dtor(h: &mut impl Host, this: u32) {
        let vt = h.rebase(0x00b7_a9ec);
        h.write(this, vt);
    }

    /// `SqsSePlayState()` (`0x0081e530`) -> 0: "SFX `SqsSePlayState`:
    /// function is not implemented yet!!!" to `stderr` (`__iob_func() +
    /// 0x40`, `fprintf`).
    pub fn sqs_se_play_state(h: &mut impl Host) -> u32 {
        let msg = h.rebase(0x00b5_8ee4);
        let iob = import(h, 0x00b0_c350, &[]);
        import(h, 0x00b0_c38c, &[iob.wrapping_add(0x40), msg]);
        0
    }

    fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Cdecl, args)
    }

    /// `0x005f0950(array, n)`: words 8 and 9 of each of `n` 40-byte
    /// records cleared (a record at address 0 skipped, as the original
    /// tests).
    pub fn clear_records(h: &mut impl Host, array: u32, n: u32) {
        let mut p = array;
        for _ in 0..n {
            if p != 0 {
                h.write(at(p, 0x20), 0u32);
                h.write(at(p, 0x24), 0u32);
            }
            p = at(p, 0x28);
        }
    }

    /// `0x006058e0`, the callee of [`set_position_then`].
    pub const SET_AFTER_POSITION: u32 = 0x0060_58e0;

    /// `0x006058b0(this; a, xyz, f)` (thiscall, `ret 0xc`): three floats
    /// from `xyz` to `+0x60..+0x68` (`fld`/`fstp`: a signalling NaN comes
    /// out quiet), then a tail call of `0x006058e0(this; a, f)` (its
    /// `eax`).
    pub fn set_position_then(h: &mut impl Host, this: u32, a: u32, xyz: u32, f: u32) -> u32 {
        for i in 0..3 {
            let v = f32::from_bits(h.read(at(xyz, 4 * i)));
            h.write(at(this, 0x60 + 4 * i), x87::round_trip(v).to_bits());
        }
        let f = x87::round_trip(f32::from_bits(f)).to_bits();
        h.call_sym(SET_AFTER_POSITION, Conv::Thiscall(this), &[a, f])
    }

    /// `std::vector<12-byte T>::_Reallocate` (`0x006b6410`, thiscall, `ret 4`).
    pub const SECTION_VECTOR_REALLOCATE: u32 = 0x006b_6410;
    pub const XLENGTH_ERROR: u32 = 0x00b0_c29c;

    /// `SectionVector_grow_006b6500(this; n)` (thiscall, `ret 4`): room for
    /// `n` more 12-byte elements: nothing when the spare capacity has it
    /// (the spare count returned);
    /// `_Xlength_error("vector<T> too long")` past the maximum; else a tail
    /// jump to `_Reallocate` with the capacity grown by half (0 when that
    /// would pass the maximum) or to `size + n` when that is more. The
    /// counts are signed divisions by 12, as the original's.
    pub fn section_vector_grow(h: &mut impl Host, this: u32, n: u32) -> u32 {
        let first: u32 = h.read(this);
        let last: u32 = h.read(at(this, 4));
        let end: u32 = h.read(at(this, 8));
        let count = |a: u32, b: u32| (b.wrapping_sub(a).cast_signed() / 12).cast_unsigned();
        let spare = count(last, end);
        if spare >= n {
            return spare;
        }
        let size = count(first, last);
        if 0x1555_5555u32.wrapping_sub(size) < n {
            let t = h.rebase(0x00b3_4498);
            import(h, XLENGTH_ERROR, &[t]);
        }
        let cap = count(first, end);
        let want = size.wrapping_add(n);
        let half = cap >> 1;
        let grown = if 0x1555_5555u32.wrapping_sub(half) < cap {
            0
        } else {
            cap.wrapping_add(half)
        };
        let new = if grown < want { want } else { grown };
        h.call_sym(SECTION_VECTOR_REALLOCATE, Conv::Thiscall(this), &[new])
    }

    /// `memchr` (MSVCR110, cdecl).
    pub const MEMCHR: u32 = 0x00b0_c390;

    /// `std_string_findFirstOf_006b68c0(this; set, pos, n)` (thiscall,
    /// `ret 0xc`): the first character from `pos` that is one of the `n`
    /// bytes at `set` (`memchr`), or -1 (also for `n` 0 or `pos` past the
    /// size). (The original keeps the end pointer in its `n` slot; no
    /// caller reads it.)
    pub fn find_first_of(h: &mut impl Host, this: u32, set: u32, pos: u32, n: u32) -> u32 {
        let size: u32 = h.read(at(this, 0x10));
        if n == 0 || pos >= size {
            return u32::MAX;
        }
        let chars = if h.read::<u32>(at(this, 0x14)) >= 0x10 {
            h.read(this)
        } else {
            this
        };
        let end = at(chars, size);
        let mut p = at(chars, pos);
        while p < end {
            let c = i32::from(h.read::<i8>(p)).cast_unsigned();
            if import(h, MEMCHR, &[set, c, n]) != 0 {
                return p.wrapping_sub(chars);
            }
            p = at(p, 1);
        }
        u32::MAX
    }

    /// `sprintf_s` (MSVCR110) and `mapChange`.
    pub const SPRINTF_S: u32 = 0x00b0_c394;
    pub const MAP_CHANGE: u32 = 0x0090_8100;

    /// `0x006b6730(this; name)` (thiscall, `ret 4`): when `name` differs
    /// from the 20-byte text at `+0x30` (`strcmp`), it is copied there
    /// (`sprintf_s(text, 0x14, name)`) and the map changed to it
    /// (`mapChange(name)`).
    pub fn set_map_name(h: &mut impl Host, this: u32, name: u32) {
        let text = at(this, 0x30);
        let mut i = 0u32;
        let same = loop {
            let a: u8 = h.read(at(text, i));
            if a != h.read::<u8>(at(name, i)) {
                break false;
            }
            if a == 0 {
                break true;
            }
            i += 1;
        };
        if !same {
            import(h, SPRINTF_S, &[text, 0x14, name]);
            h.call_sym(MAP_CHANGE, Conv::Cdecl, &[name]);
        }
    }

    /// `opusTree_FreeSubtree_00806bb0` (thiscall `(tree; node)`).
    pub const OPUS_FREE_SUBTREE: u32 = 0x0080_6bb0;

    /// `0x00807120(tree)` (thiscall): the tree's `clear()`: down the
    /// root's left spine, each node's right subtree freed
    /// ([`OPUS_FREE_SUBTREE`]) and the node deleted; the head's links back
    /// to itself, the size 0.
    pub fn opus_tree_clear(h: &mut impl Host, tree: u32) {
        let head: u32 = h.read(tree);
        let mut node: u32 = h.read(at(head, 4));
        while h.read::<u8>(at(node, 0xd)) == 0 {
            let right: u32 = h.read(at(node, 8));
            h.call_sym(OPUS_FREE_SUBTREE, Conv::Thiscall(tree), &[right]);
            let next: u32 = h.read(node);
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[node]);
            node = next;
        }
        let head: u32 = h.read(tree);
        h.write(at(head, 4), head);
        let head: u32 = h.read(tree);
        h.write(head, head);
        let head: u32 = h.read(tree);
        h.write(at(head, 8), head);
        h.write(at(tree, 4), 0u32);
    }

    /// The facet list's head and `_Fac_node`'s destructor (thiscall).
    pub const FACET_LIST: u32 = 0x0193_ed28;
    pub const FAC_NODE_DTOR: u32 = 0x0094_a62c;

    /// `0x0094a641()`: the facet list freed node by node (the head moved
    /// on first, `~_Fac_node`, `operator delete`).
    pub fn facet_list_free(h: &mut impl Host) {
        loop {
            let node = global(h, FACET_LIST);
            if node == 0 {
                return;
            }
            let next: u32 = h.read(node);
            set_global(h, FACET_LIST, next);
            h.call_sym(FAC_NODE_DTOR, Conv::Thiscall(node), &[]);
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[node]);
        }
    }

    /// `graphicGetTime()` (cdecl, `ST0`: an `f32`-exact value) and
    /// `g_PhyreScene`.
    pub const GRAPHIC_GET_TIME: u32 = 0x0064_1410;
    pub const PHYRE_SCENE: u32 = 0x00cc_b9d8;

    /// `graphicGetTime()` as the `ST0` value it leaves (`fld dword`).
    fn now(h: &mut impl Host) -> f64 {
        f64::from(h.call_sym_st0(GRAPHIC_GET_TIME, Conv::Cdecl, &[]))
    }

    fn word(h: &impl Host, p: u32) -> f64 {
        x87::load(h.read(p))
    }

    /// `SkipPrompt_update_006e8540(prompt)` (thiscall) -> `al`: whether the
    /// movie's skip prompt still shows. Showing (`+0x15`): the fade-in
    /// `(now - shown +4) / length +0xc` (the difference stored as `f32`
    /// first), at most 1, in `+0x10`; past the hold (`+0` above the
    /// elapsed time) but within its last `length` (hold - length below
    /// it), the prompt starts hiding (`+0x15` 0, hide time `+8` = now,
    /// `+0x16` 1); 1. Hiding (`+0x16`): the fade-out `1 - (now - hidden) /
    /// length` in `+0x10`; once `length` has passed (or the compare is
    /// unordered) `+0x16` and `+0x14` are cleared and 0 is returned, else
    /// one. Neither: nothing; the original returns the `eax` its caller left
    /// (`PhyFMVPlayerManager_Render`'s, its only caller: `g_PhyreScene`'s
    /// value), which the port returns. Only `al` is defined (`ah` holds
    /// the FPU status word in the original).
    pub fn skip_prompt_update(h: &mut impl Host, p: u32) -> u32 {
        let length = word(h, at(p, 0xc));
        if h.read::<u8>(at(p, 0x15)) != 0 {
            let t = now(h);
            let d = x87::load(x87::store(x87::sub(t, word(h, at(p, 4)))));
            let q = x87::div(d, length);
            // fcom 1.0, q: 1.0 only when below q (a NaN q stays)
            let fade = if 1.0 < q { 1.0 } else { q };
            h.write(at(p, 0x10), x87::store(fade));
            let hold = word(h, p);
            if hold > d && x87::sub(hold, length) < d {
                h.write(at(p, 0x15), 0u8);
                let t = now(h);
                h.write(at(p, 8), x87::store(t));
                h.write(at(p, 0x16), 1u8);
            }
            return 1;
        }
        if h.read::<u8>(at(p, 0x16)) == 0 {
            return h.read(h.rebase(PHYRE_SCENE));
        }
        let t = now(h);
        let d = x87::load(x87::store(x87::sub(t, word(h, at(p, 8)))));
        let q = x87::div(d, length);
        h.write(at(p, 0x10), x87::store(x87::sub(1.0, q)));
        if length > d {
            return 1;
        }
        h.write(at(p, 0x16), 0u8);
        h.write(at(p, 0x14), 0u8);
        0
    }

    /// `Phyre_PhyreMalloc_0042fb80(size, align)` and the `memcpy` thunk.
    pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
    pub const MEMCPY: u32 = 0x0094_9256;

    /// `0x005130f0(this; src)` (thiscall, `ret 4`): a 28-byte record
    /// copied: five words, the name (`+0x14`, a pointer tagged in bit 0)
    /// duplicated untagged (`PhyreMalloc(len + 1, 2)`, `memcpy`; 0 when
    /// there is none or the allocation fails), the last word. Returns
    /// `this`. (The original keeps `len + 1` in its `src` slot; no caller
    /// reads it.)
    pub fn copy_named_record(h: &mut impl Host, this: u32, src: u32) -> u32 {
        for o in [0, 4, 8, 0xc, 0x10] {
            let w: u32 = h.read(at(src, o));
            h.write(at(this, o), w);
        }
        let tagged: u32 = h.read(at(src, 0x14));
        let name = tagged.wrapping_sub(u32::from(h.read::<u8>(at(src, 0x14)) & 1));
        let mut copy = 0;
        if name != 0 {
            let mut len = 0u32;
            while h.read::<u8>(at(name, len)) != 0 {
                len = len.wrapping_add(1);
            }
            let n = len.wrapping_add(1);
            copy = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[n, 2]);
            if copy != 0 {
                h.call_sym(MEMCPY, Conv::Cdecl, &[copy, name, n]);
            }
        }
        h.write(at(this, 0x14), copy);
        let w: u32 = h.read(at(src, 0x18));
        h.write(at(this, 0x18), w);
        this
    }

    /// `0x006258e0(this; out, key)` (thiscall, `ret 8`): `equal_range` of
    /// the `u32` key `*key` in a map whose nodes keep it at `+0x10`
    /// (left `+0`, parent `+4`, right `+8`, nil flag `+0xd`): the lower
    /// bound (the walk also notes the first node above the key while the
    /// upper bound is still the head) and the upper bound, searched again
    /// from that node's left child (else the root). `out` = {lower,
    /// upper}; returns `out`.
    pub fn equal_range_u32(h: &mut impl Host, this: u32, out: u32, key: u32) -> u32 {
        let head: u32 = h.read(this);
        let root: u32 = h.read(at(head, 4));
        let mut lower = head;
        let mut upper = head;
        let mut p = root;
        if h.read::<u8>(at(p, 0xd)) == 0 {
            let k: u32 = h.read(key);
            loop {
                let pk: u32 = h.read(at(p, 0x10));
                if pk < k {
                    p = h.read(at(p, 8));
                } else {
                    if h.read::<u8>(at(upper, 0xd)) != 0 && k < pk {
                        upper = p;
                    }
                    lower = p;
                    p = h.read(p);
                }
                if h.read::<u8>(at(p, 0xd)) != 0 {
                    break;
                }
            }
        }
        let mut p = if h.read::<u8>(at(upper, 0xd)) == 0 {
            h.read(upper)
        } else {
            root
        };
        while h.read::<u8>(at(p, 0xd)) == 0 {
            if h.read::<u32>(key) < h.read::<u32>(at(p, 0x10)) {
                upper = p;
                p = h.read(p);
            } else {
                p = h.read(at(p, 8));
            }
        }
        h.write(out, lower);
        h.write(at(out, 4), upper);
        out
    }

    /// `CloseHandle` (stdcall), the cluster map's range erase
    /// (`0x0069af70`, thiscall `(map; &out, first, last)`) and
    /// `FreeOwnedArray_0069a130(p, n)`.
    pub const CLOSE_HANDLE: u32 = 0x00b0_c0dc;
    pub const CLUSTER_MAP_ERASE: u32 = 0x0069_af70;
    pub const FREE_OWNED_ARRAY: u32 = 0x0069_a130;

    /// `0x00699d60(this)` (thiscall): the cluster manager's teardown: the
    /// event `+0x18` closed (`CloseHandle` of its first word) and deleted,
    /// the map at `+0x10` erased whole (into `slot`, the original's local
    /// at `ebp - 4`) and its head deleted, the array `+0xc` freed when
    /// owned (`+8` bit 31 clear; its count the low 31 bits), `+8`/`+0xc`
    /// cleared.
    pub fn cluster_manager_shutdown(h: &mut impl Host, this: u32, slot: u32) {
        let ev: u32 = h.read(at(this, 0x18));
        if ev != 0 {
            let handle: u32 = h.read(ev);
            let f: u32 = h.read(h.rebase(CLOSE_HANDLE));
            h.call(f, Conv::Stdcall, &[handle]);
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[ev]);
        }
        let head: u32 = h.read(at(this, 0x10));
        let first: u32 = h.read(head);
        h.call_sym(
            CLUSTER_MAP_ERASE,
            Conv::Thiscall(at(this, 0x10)),
            &[slot, first, head],
        );
        let head: u32 = h.read(at(this, 0x10));
        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[head]);
        let n: u32 = h.read(at(this, 8));
        if n >> 31 == 0 {
            let p: u32 = h.read(at(this, 0xc));
            h.call_sym(FREE_OWNED_ARRAY, Conv::Cdecl, &[p, n & 0x7fff_ffff]);
        }
        h.write(at(this, 8), 0u32);
        h.write(at(this, 0xc), 0u32);
    }

    /// `AtlThrow(hr)` (`0x0064fa50`, does not return), MSVCR110's
    /// `_recalloc`, `calloc` and `free` (cdecl).
    pub const ATL_THROW: u32 = 0x0064_fa50;
    pub const RECALLOC: u32 = 0x00b0_c440;
    pub const CALLOC: u32 = 0x00b0_c43c;
    pub const FREE: u32 = 0x00b0_c370;

    /// `0x006496d0(p, n, fixed, fixed_n)`: ATL's `AtlConvAllocMemory` for
    /// `n` wide characters: `*p` stays the caller's fixed buffer when `n`
    /// fits its `fixed_n` (a heap block it had is freed), else grows
    /// (`_recalloc` of a heap block, `calloc` instead of the fixed one).
    /// `E_INVALIDARG` for a null `p`/`fixed` or a negative `n`,
    /// `E_OUTOFMEMORY` when nothing came back (`AtlThrow`, which does not
    /// return; the port returns after it).
    pub fn conv_alloc_memory(h: &mut impl Host, p: u32, n: u32, fixed: u32, fixed_n: u32) {
        if p == 0 || n.cast_signed() < 0 || fixed == 0 {
            h.call_sym(ATL_THROW, Conv::Cdecl, &[0x8007_0057]);
            return;
        }
        let cur: u32 = h.read(p);
        let fits = n.cast_signed() <= fixed_n.cast_signed();
        if cur == fixed {
            if fits {
                h.write(p, fixed);
            } else {
                let f: u32 = h.read(h.rebase(CALLOC));
                let new = h.call(f, Conv::Cdecl, &[n, 2]);
                h.write(p, new);
            }
        } else if fits {
            let f: u32 = h.read(h.rebase(FREE));
            h.call(f, Conv::Cdecl, &[cur]);
            h.write(p, fixed);
        } else {
            let f: u32 = h.read(h.rebase(RECALLOC));
            let new = h.call(f, Conv::Cdecl, &[cur, n, 2]);
            if new == 0 {
                h.call_sym(ATL_THROW, Conv::Cdecl, &[0x8007_000e]);
                return;
            }
            h.write(p, new);
        }
        if h.read::<u32>(p) == 0 {
            h.call_sym(ATL_THROW, Conv::Cdecl, &[0x8007_000e]);
        }
    }

    /// Bullet's `btAlignedFreeInternal(ptr)` (cdecl): the aligned-free
    /// count (`0x0193fc88`) and the free hook (`0x00c64be4`, cdecl `(ptr)`),
    /// the `btSimulationIslandManager` vtable and `btUnionFind`'s
    /// destructor (thiscall).
    pub const BT_ALIGNED_FREE: u32 = 0x009d_4d50;
    pub const BT_FREE_COUNT: u32 = 0x0193_fc88;
    pub const BT_FREE_FUNC: u32 = 0x00c6_4be4;
    pub const BT_ISLAND_MANAGER_VTABLE: u32 = 0x00b7_c78c;
    pub const BT_ISLAND_MANAGER_DTOR: u32 = 0x00a8_da90;
    pub const BT_UNION_FIND_DTOR: u32 = 0x00a8_e680;

    /// `btAlignedFreeInternal(ptr)` (`0x009d4d50`) -> the free hook's
    /// result: a null `ptr` gives 0; else the count goes up and the hook
    /// frees it (the original tail-jumps there).
    pub fn bt_aligned_free(h: &mut impl Host, ptr: u32) -> u32 {
        if ptr == 0 {
            return 0;
        }
        let at_count = h.rebase(BT_FREE_COUNT);
        let n = h.read::<u32>(at_count).wrapping_add(1);
        h.write(at_count, n);
        let f: u32 = h.read(h.rebase(BT_FREE_FUNC));
        h.call(f, Conv::Cdecl, &[ptr])
    }

    /// A `btAlignedObjectArray` at `arr` (size `+0`, capacity `+4`, data
    /// `+8`, owns-memory byte `+0xc`) cleared as its `clear()`: owned data
    /// freed ([`BT_ALIGNED_FREE`]), the data cleared, owning again, size
    /// and capacity 0.
    fn bt_array_clear(h: &mut impl Host, arr: u32) {
        let data: u32 = h.read(arr.wrapping_add(8));
        if data != 0 {
            if h.read::<u8>(arr.wrapping_add(0xc)) != 0 {
                h.call_sym(BT_ALIGNED_FREE, Conv::Cdecl, &[data]);
            }
            h.write(arr.wrapping_add(8), 0u32);
        }
        h.write(arr.wrapping_add(0xc), 1u8);
        h.write(arr.wrapping_add(8), 0u32);
        h.write(arr, 0u32);
        h.write(arr.wrapping_add(4), 0u32);
    }

    /// `btSimulationIslandManager::~btSimulationIslandManager(this)`
    /// (`0x00a8da90`, thiscall): its vtable set, its two arrays (`+0x30`,
    /// `+0x1c`) cleared, then its union-find (`+4`) destroyed
    /// ([`BT_UNION_FIND_DTOR`]). (The original's exception frame has
    /// nothing to unwind here.)
    pub fn bt_island_manager_dtor(h: &mut impl Host, this: u32) {
        let vt = h.rebase(BT_ISLAND_MANAGER_VTABLE);
        h.write(this, vt);
        bt_array_clear(h, this.wrapping_add(0x30));
        bt_array_clear(h, this.wrapping_add(0x1c));
        h.call_sym(
            BT_UNION_FIND_DTOR,
            Conv::Thiscall(this.wrapping_add(4)),
            &[],
        );
    }

    /// `btUnionFind::~btUnionFind(this)` (`0x00a8e680`, thiscall): its
    /// element array (`+4`) cleared.
    pub fn bt_union_find_dtor(h: &mut impl Host, this: u32) {
        bt_array_clear(h, this.wrapping_add(4));
    }

    /// ATL's module pieces: the `memset` and `free` thunks,
    /// `Enter`/`Leave`/`DeleteCriticalSection`, `HeapDestroy` and
    /// `RaiseException` (stdcall imports), the `CWin32Heap` vtable.
    pub const CRT_MEMSET: u32 = 0x0094_964c;
    pub const CRT_FREE: u32 = 0x0094_95f2;
    pub const ENTER_CRITICAL_SECTION: u32 = 0x00b0_c154;
    pub const LEAVE_CRITICAL_SECTION: u32 = 0x00b0_c158;
    pub const DELETE_CRITICAL_SECTION: u32 = 0x00b0_c15c;
    pub const HEAP_DESTROY: u32 = 0x00b0_c04c;
    pub const RAISE_EXCEPTION: u32 = 0x00b0_c0e0;
    pub const WIN32_HEAP_VTABLE: u32 = 0x00b7_02a0;
    pub const ATL_BASE_MODULE_INIT: u32 = 0x009d_9c82;
    pub const ATL_BASE_MODULE_DTOR: u32 = 0x009d_9cc1;
    pub const ATL_GET_HINSTANCE_AT: u32 = 0x009d_9db5;
    pub const ATL_SIMPLE_ARRAY_AT: u32 = 0x009d_9ce0;
    pub const ATL_SIMPLE_ARRAY_DTOR: u32 = 0x009d_9e5e;
    pub const ATL_WIN32_HEAP_DTOR: u32 = 0x009d_a05c;
    pub const ATL_STRING_ADD_REF: u32 = 0x009d_a169;

    fn import_stdcall(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Stdcall, args)
    }

    /// `_ATL_BASE_MODULE70`'s constructor (`0x009d9c82`, thiscall) ->
    /// `this`: its critical section (`+0x14`, 0x18 bytes) zeroed
    /// (`memset`) and its resource-instance array (`+0x2c..+0x38`) empty.
    pub fn atl_base_module_init(h: &mut impl Host, this: u32) -> u32 {
        h.call_sym(CRT_MEMSET, Conv::Cdecl, &[this.wrapping_add(0x14), 0, 0x18]);
        for o in [0x2c, 0x30, 0x34] {
            h.write(this.wrapping_add(o), 0u32);
        }
        this
    }

    /// `CAtlBaseModule::~CAtlBaseModule(this)` (`0x009d9cc1`, thiscall):
    /// its critical section (`+0x14`) deleted, then its resource-instance
    /// array (`+0x2c`) destroyed (the original tail-jumps there).
    pub fn atl_base_module_dtor(h: &mut impl Host, this: u32) {
        import_stdcall(h, DELETE_CRITICAL_SECTION, &[this.wrapping_add(0x14)]);
        h.call_sym(
            ATL_SIMPLE_ARRAY_DTOR,
            Conv::Thiscall(this.wrapping_add(0x2c)),
            &[],
        );
    }

    /// `CSimpleArray<HINSTANCE>::~CSimpleArray(this)` (`0x009d9e5e`,
    /// thiscall): its data (`+0`) freed and cleared, size and capacity 0.
    pub fn atl_simple_array_dtor(h: &mut impl Host, this: u32) {
        let data: u32 = h.read(this);
        if data != 0 {
            h.call_sym(CRT_FREE, Conv::Cdecl, &[data]);
            h.write(this, 0u32);
        }
        h.write(this.wrapping_add(4), 0u32);
        h.write(this.wrapping_add(8), 0u32);
    }

    /// `CSimpleArray<HINSTANCE>::operator[](this; i)` (`0x009d9ce0`,
    /// thiscall, `ret 4`) -> `&data[i]`; an index outside `0..size`
    /// raises `EXCEPTION_ARRAY_BOUNDS_EXCEEDED` (non-continuable; the
    /// original does not come back).
    pub fn atl_simple_array_at(h: &mut impl Host, this: u32, i: i32) -> u32 {
        if i < 0 || i >= h.read::<i32>(this.wrapping_add(4)) {
            import_stdcall(h, RAISE_EXCEPTION, &[0xc000_008c, 1, 0, 0]);
            return 0;
        }
        let data: u32 = h.read(this);
        data.wrapping_add(i.cast_unsigned().wrapping_mul(4))
    }

    /// `CAtlBaseModule::GetHInstanceAt(this; i)` (`0x009d9db5`, thiscall,
    /// `ret 4`) -> the `i`-th resource instance under the module's lock
    /// (`+0x14`): the module's own (`+8`) at the array's size (`+0x30`),
    /// the array's entry below it, 0 outside.
    pub fn atl_get_hinstance_at(h: &mut impl Host, this: u32, i: i32) -> u32 {
        let cs = this.wrapping_add(0x14);
        import_stdcall(h, ENTER_CRITICAL_SECTION, &[cs]);
        let n: i32 = h.read(this.wrapping_add(0x30));
        let r = if i > n || i < 0 {
            0
        } else if i == n {
            h.read(this.wrapping_add(8))
        } else {
            let p = h.call_sym(
                ATL_SIMPLE_ARRAY_AT,
                Conv::Thiscall(this.wrapping_add(0x2c)),
                &[i.cast_unsigned()],
            );
            h.read(p)
        };
        import_stdcall(h, LEAVE_CRITICAL_SECTION, &[cs]);
        r
    }

    /// `CWin32Heap::~CWin32Heap(this)` (`0x009da05c`, thiscall): its
    /// vtable set; an owned (`+8` byte) heap (`+4`) destroyed.
    pub fn atl_win32_heap_dtor(h: &mut impl Host, this: u32) {
        let vt = h.rebase(WIN32_HEAP_VTABLE);
        h.write(this, vt);
        if h.read::<u8>(this.wrapping_add(8)) != 0 {
            let heap: u32 = h.read(this.wrapping_add(4));
            if heap != 0 {
                import_stdcall(h, HEAP_DESTROY, &[heap]);
            }
        }
    }

    /// The string data's `AddRef` (`0x009da169`, thiscall) -> its
    /// characters (`this + 8`): the reference count (`+0x14`) one more
    /// (the original's interlocked add).
    pub fn atl_string_add_ref(h: &mut impl Host, this: u32) -> u32 {
        let rc = this.wrapping_add(0x14);
        let n = h.read::<u32>(rc).wrapping_add(1);
        h.write(rc, n);
        this.wrapping_add(8)
    }

    /// `ButtonIcon_setUvs`' quad helper's callees: `PhyreMalloc(size,
    /// align)` (cdecl), the vertex-block constructor, `PArray::setBuffer`,
    /// the vertex buffer's set-up, `isBound`, `release`, the ring buffer's
    /// `submit` and `unmapCopy` (thiscall), and the vertex format the
    /// buffer's description gets.
    pub const BUTTON_QUAD: u32 = 0x006e_81f0;
    pub const BLOCK_CTOR: u32 = 0x0055_dc40;
    pub const ARRAY_SET_BUFFER: u32 = 0x0043_4e40;
    pub const VB_SETUP: u32 = 0x0048_35d0;
    pub const VB_IS_BOUND: u32 = 0x0056_2da0;
    pub const VB_RELEASE: u32 = 0x0056_46d0;
    pub const RING_SUBMIT: u32 = 0x0043_4c50;
    pub const RING_UNMAP_COPY: u32 = 0x0056_4cb0;
    pub const QUAD_FORMAT: u32 = 0x00c9_8638;

    /// The quad helper of `ButtonIcon_setUvs` (`0x006e81f0`, stdcall,
    /// `ret 0x14`: `(&object, w, h, u, v)`): the icon's mesh (`*object`
    /// `+4`) gets mode 2 (`+0x10`) and, unless it has them (`+0x28` count
    /// 2), two vertex blocks (0x80 bytes from `PhyreMalloc`, each
    /// constructed, set as its array); its vertex buffer (`+0x2c` `+0x40`)
    /// is set up for 8-byte vertices (6, 1), re-created when its size
    /// changed (released when bound), its description pointed at the quad
    /// format; then six vertices are written into the ring buffer's next
    /// slot (`submit` into `frame`): the cell `u..u + 1/h` by `top..top +
    /// 1/w` with `top = 1 - v - 1/w`, computed as the x87 does (53 bits,
    /// rounded at each store); the slot is copied (`unmapCopy`) and, when
    /// that succeeds, becomes the current one (`+0x10`, flag `+0x14`
    /// cleared). `frame` is the original's `ebp - 0x1c` (the slot).
    pub fn button_quad(
        h: &mut impl Host,
        object: u32,
        w: i32,
        hh: i32,
        u: f32,
        v: f32,
        frame: u32,
    ) {
        use x87::Reg;
        // the reciprocals and the top edge, as the original rounds them
        let hf = f32::from_bits(x87::store(f64::from(hh)));
        let inv_h = (Reg::load(1.0) / Reg::load(hf)).store();
        let wf = f32::from_bits(x87::store(f64::from(w)));
        let inv_w = (Reg::load(1.0) / Reg::load(wf)).store();
        let top = Reg::load(1.0).sub_m32(v).sub_m32(inv_w).store();
        let obj: u32 = h.read(object);
        let mesh: u32 = h.read(at(obj, 4));
        h.write(at(mesh, 0x10), 2u32);
        if h.read::<u32>(at(mesh, 0x28)) != 2 {
            let p = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[0x80, 4]);
            if p != 0 {
                for i in 0..2u32 {
                    let e = at(p, i * 0x40);
                    if e != 0 {
                        h.call_sym(BLOCK_CTOR, Conv::Thiscall(e), &[]);
                    }
                }
                h.call_sym(ARRAY_SET_BUFFER, Conv::Thiscall(at(mesh, 0x28)), &[p, 2]);
            }
        }
        let vb = at(h.read::<u32>(at(mesh, 0x2c)), 0x40);
        h.call_sym(VB_SETUP, Conv::Thiscall(vb), &[8, 6, 1]);
        let size = h.read::<u32>(at(vb, 4)).wrapping_mul(h.read::<u32>(vb));
        if h.read::<u32>(at(vb, 0x38)) != size {
            if h.call_sym(VB_IS_BOUND, Conv::Thiscall(vb), &[]) as u8 != 0 {
                h.call_sym(VB_RELEASE, Conv::Thiscall(vb), &[]);
            }
            h.write(at(vb, 0x38), size);
        }
        let desc: u32 = h.read(at(vb, 0xc));
        h.write(at(desc, 8), 1u8);
        let format = h.rebase(QUAD_FORMAT);
        h.write(at(desc, 4), format);
        h.write(desc, 0u32);
        h.call_sym(RING_SUBMIT, Conv::Thiscall(vb), &[frame]);
        let out: u32 = h.read(frame);
        let u = x87::round_trip(u);
        let y1 = (Reg::load(inv_w) + Reg::load(top)).store();
        let x1 = (Reg::load(inv_h) + Reg::load(u)).store();
        for (i, (x, y)) in [(u, y1), (u, top), (x1, top), (x1, top), (x1, y1), (u, y1)]
            .into_iter()
            .enumerate()
        {
            let i = u32::try_from(i).unwrap_or(0);
            h.write(at(out, 8 * i), x.to_bits());
            h.write(at(out, 8 * i + 4), y.to_bits());
        }
        let next = h.read::<u32>(at(vb, 0x10)).wrapping_add(1);
        let wrap = h.read::<u32>(at(vb, 0x18)) & 0x7fff_ffff;
        let slot = if next == wrap { 0 } else { next };
        let r = h.call_sym(RING_UNMAP_COPY, Conv::Thiscall(vb), &[slot, frame]);
        if r == 0 {
            h.write(at(vb, 0x10), slot);
            h.write(at(vb, 0x14), 0u8);
        }
    }

    /// The input devices' shared state (`0x00ccb15c`, its mutex `+0xa8`),
    /// the device map's sentinel (`0x00c31e9c`; its leftmost link `+0xc`)
    /// and node free list (`0x00c31eb4`); `WaitForSingleObject` and
    /// `ReleaseMutex` (stdcall imports), the tree's successor
    /// (thiscall `(tree; node)`, `ret 4`) and `orphanAllContents`, the
    /// free list's `reset` (thiscall), `user_free` (cdecl).
    pub const INPUT_RESET_DEVICES: u32 = 0x0063_0730;
    pub const INPUT_SHARED: u32 = 0x00cc_b15c;
    pub const INPUT_MAP: u32 = 0x00c3_1e9c;
    pub const INPUT_MAP_NODES: u32 = 0x00c3_1eb4;
    pub const WAIT_FOR_SINGLE_OBJECT: u32 = 0x00b0_c0f8;
    pub const RELEASE_MUTEX: u32 = 0x00b0_c1c0;
    pub const TREE_SUCCESSOR: u32 = 0x0057_5160;
    pub const TREE_ORPHAN_ALL: u32 = 0x0057_4970;
    pub const FREE_LIST_RESET: u32 = 0x0049_cc10;
    pub const USER_FREE: u32 = 0x0068_7010;

    /// The input shutdown's device-map reset (`0x00630730`, no arguments):
    /// with the shared state there, under its mutex: from the leftmost
    /// node (down the `+0` links from the sentinel's `+0xc`) every node
    /// up to the sentinel is freed (`user_free`), each successor taken
    /// before; then the tree's contents orphaned and its node free list
    /// reset. (The original's exception frame has nothing to unwind.)
    pub fn input_reset_devices(h: &mut impl Host) {
        let shared: u32 = h.read(h.rebase(INPUT_SHARED));
        if shared == 0 {
            return;
        }
        let mutex: u32 = h.read(at(shared, 0xa8));
        let f: u32 = h.read(h.rebase(WAIT_FOR_SINGLE_OBJECT));
        h.call(f, Conv::Stdcall, &[mutex, u32::MAX]);
        let head = h.rebase(INPUT_MAP);
        let mut node: u32 = h.read(at(head, 0xc));
        if node != head && node != 0 {
            loop {
                let next: u32 = h.read(node);
                if next == head {
                    break;
                }
                node = next;
            }
        }
        if node != head {
            let mut next = h.call_sym(TREE_SUCCESSOR, Conv::Thiscall(head), &[node]);
            loop {
                h.call_sym(USER_FREE, Conv::Cdecl, &[node]);
                node = next;
                if next != head {
                    next = h.call_sym(TREE_SUCCESSOR, Conv::Thiscall(head), &[next]);
                }
                if node == head {
                    break;
                }
            }
        }
        h.call_sym(TREE_ORPHAN_ALL, Conv::Thiscall(head), &[]);
        let nodes = h.rebase(INPUT_MAP_NODES);
        h.call_sym(FREE_LIST_RESET, Conv::Thiscall(nodes), &[]);
        let f: u32 = h.read(h.rebase(RELEASE_MUTEX));
        h.call(f, Conv::Stdcall, &[mutex]);
    }

    /// The script heap's realloc (`0x0061d030`), `memcpy` and
    /// `PhyrePrintf(level, format, ...)` (cdecl) with its out-of-memory
    /// format.
    pub const SCRIPT_REALLOC: u32 = 0x0061_d030;
    pub const PHYRE_PRINTF: u32 = 0x0043_52f0;
    pub const SCRIPT_OOM_FORMAT: u32 = 0x00b3_fa68;

    /// The script heap's realloc (`0x0061d030`, thiscall `(heap; p, old,
    /// new)`, `ret 0xc`) -> the block: `p` when `new` is 0 or not above
    /// `old`; else `new` rounded up to 16 bytes taken from the bump pointer
    /// (`+0`) once aligned to 16 up to the end (`+4`), the old bytes copied
    /// to the bump pointer as it was (not the aligned one, as the original)
    /// and that returned; past the end the failure is printed (level 1,
    /// the rounded size) and 0 returned.
    pub fn script_realloc(h: &mut impl Host, heap: u32, p: u32, old: u32, new: u32) -> u32 {
        if new == 0 || new <= old {
            return p;
        }
        let cur: u32 = h.read(heap);
        let size = new.wrapping_add(0xf) & !0xf;
        let end = (cur.wrapping_neg() & 0xf)
            .wrapping_add(cur)
            .wrapping_add(size);
        if end > h.read::<u32>(at(heap, 4)) {
            let format = h.rebase(SCRIPT_OOM_FORMAT);
            h.call_sym(PHYRE_PRINTF, Conv::Cdecl, &[1, format, size]);
            return 0;
        }
        h.write(heap, end);
        if old != 0 {
            h.call_sym(MEMCPY, Conv::Cdecl, &[cur, p, old]);
        }
        cur
    }

    /// The physics worlds' fix-up after loading (`0x005eb3c0`): its
    /// static header (`0x00cc0598`, enabled byte `+0`; guard bit 0 of
    /// `0x00cc05e4`, its constructor, the destructor `atexit` gets) and the
    /// per-world steps (thiscall).
    pub const PHYSICS_FIX_WORLDS: u32 = 0x005e_b3c0;
    pub const PHYSICS_HEADER: u32 = 0x00cc_0598;
    pub const PHYSICS_HEADER_GUARD: u32 = 0x00cc_05e4;
    pub const PHYSICS_HEADER_CTOR: u32 = 0x005e_0330;
    pub const PHYSICS_HEADER_DTOR: u32 = 0x00b0_5d10;
    pub const ATEXIT: u32 = 0x0094_9207;
    pub const PHYSICS_WORLD_FIX_A: u32 = 0x005f_56a0;
    pub const PHYSICS_WORLD_FIX_B: u32 = 0x005f_35b0;

    /// The physics worlds' fix-up after loading (`0x005eb3c0`, cdecl
    /// `(array)`) -> 0: the static physics header built on first use (its
    /// guard bit set, constructed, its destructor registered with
    /// `atexit`); when it is enabled, each of the array's worlds (`+0x3c`
    /// of them from `+0x30`, `+0x2c`'s `+0x1c` bytes apart) gets both
    /// steps. (The original's exception frame has nothing to unwind.)
    pub fn physics_fix_worlds(h: &mut impl Host, array: u32) -> u32 {
        let guard_at = h.rebase(PHYSICS_HEADER_GUARD);
        let guard: u32 = h.read(guard_at);
        let header = h.rebase(PHYSICS_HEADER);
        if guard & 1 == 0 {
            h.write(guard_at, guard | 1);
            h.call_sym(PHYSICS_HEADER_CTOR, Conv::Thiscall(header), &[]);
            let dtor = h.rebase(PHYSICS_HEADER_DTOR);
            h.call_sym(ATEXIT, Conv::Cdecl, &[dtor]);
        }
        if h.read::<u8>(header) != 0 {
            let mut n: u32 = h.read(at(array, 0x3c));
            let mut world: u32 = h.read(at(array, 0x30));
            let desc: u32 = h.read(at(array, 0x2c));
            let stride: u32 = h.read(at(desc, 0x1c));
            while n != 0 {
                h.call_sym(PHYSICS_WORLD_FIX_A, Conv::Thiscall(world), &[]);
                h.call_sym(PHYSICS_WORLD_FIX_B, Conv::Thiscall(world), &[]);
                world = world.wrapping_add(stride);
                n -= 1;
            }
        }
        0
    }
}
