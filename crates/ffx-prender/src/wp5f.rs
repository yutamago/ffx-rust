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
}
