//! M5 `system-a` (round 3): the graphic UI object's constructor
//! (`0x0067ad30`, from `GraphicUi_CreateInstance_0067b6c0`): its record
//! arrays sized and filled from the buffers `GraphicUi_AllocBuffers`
//! makes; a shadow gradient mesh's vertex set-up (`0x006790c0`); and
//! the graphic object's destructor (`0x00634850`).

/// The originals over game memory and the original callees.
pub mod bridge {
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// `GraphicUi_AllocBuffers_0067b770` (thiscall).
    pub const ALLOC_BUFFERS: u32 = 0x0067_b770;
    /// `Phyre_PhyreMalloc_0042fb80(size, align)` and `Phyre_PhyreFree`
    /// (cdecl).
    pub const MALLOC_ALIGNED: u32 = 0x0042_fb80;
    pub const PHYRE_FREE: u32 = 0x0042_fb20;
    /// `MenuRecord_InitSingle_0067c2d0` / `MenuRecord_InitMulti_0067c030`
    /// (thiscall, `ret 4`).
    pub const INIT_SINGLE: u32 = 0x0067_c2d0;
    pub const INIT_MULTI: u32 = 0x0067_c030;
    /// `crt_memset_thunk` (cdecl).
    pub const MEMSET: u32 = 0x0094_964c;
    /// The pointer stored at `+0x5b88`.
    pub const DEFAULT_5B88: u32 = 0x00b4_5ba0;

    /// The 4-byte array at `this + at` (`+0` its count, its top bit set
    /// for a borrowed block; `+4` the block) resized to `n` elements: a
    /// zeroed aligned block (none when that fails), the old one freed
    /// when owned and different.
    fn resize(h: &mut impl Host, this: u32, at: u32, n: u32) {
        let count_at = this.wrapping_add(at);
        if h.read::<u32>(count_at) == n {
            return;
        }
        let p = h.call_sym(MALLOC_ALIGNED, Conv::Cdecl, &[n * 4, 4]);
        if p == 0 {
            return;
        }
        for k in 0..n {
            h.write(p.wrapping_add(4 * k), 0u32);
        }
        let old: u32 = h.read(count_at.wrapping_add(4));
        if old != p && h.read::<u32>(count_at) >> 31 == 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
        }
        h.write(count_at.wrapping_add(4), p);
        h.write(count_at, n);
    }

    /// The array at `this + at` filled: element `i` = `init(this;
    /// buffer)`, the buffer moving on by the word at `this + step` after
    /// each. Returns the buffer after the last.
    fn fill(
        h: &mut impl Host,
        this: u32,
        at: u32,
        n: u32,
        init: u32,
        step: u32,
        mut buf: u32,
    ) -> u32 {
        for k in 0..n {
            let block: u32 = h.read(this.wrapping_add(at + 4));
            let slot = block.wrapping_add(4 * k);
            let v = h.call_sym(init, Conv::Thiscall(this), &[buf]);
            h.write(slot, v);
            buf = buf.wrapping_add(h.read(this.wrapping_add(step)));
        }
        buf
    }

    /// `0x0067ad30(this)` (thiscall): the four array headers (`+0x15e0`,
    /// `+0x2bd0`, `+0x4238`, `+0x5828`, four words each) and `+0x2ec30`
    /// cleared, the buffers allocated (`GraphicUi_AllocBuffers`), then
    /// from the first buffer (`+0x5b80`) 600 single records into the
    /// array at `+0x15e0`, four more into `+0x2be4..=+0x2bf0`, 800 multi
    /// records into `+0x15e8`, and from the second (`+0x5b84`) 600 single
    /// and 800 multi into `+0x4238` and `+0x4240` (the record sizes
    /// `+0x5b94` and `+0x5b98`); the rest of the state reset (`+0x5848`
    /// 100 words, `+0..+0x15e0` zeroed, `+0x5b7c` 1). Returns `this`.
    pub fn ctor(h: &mut impl Host, this: u32) -> u32 {
        let at = |off: u32| this.wrapping_add(off);
        for base in [0x15e0u32, 0x2bd0, 0x4238, 0x5828] {
            for k in 0..4 {
                h.write(at(base + 4 * k), 0u32);
            }
        }
        h.write(at(0x2ec30), 0u32);
        h.call_sym(ALLOC_BUFFERS, Conv::Thiscall(this), &[]);
        let buf: u32 = h.read(at(0x5b80));
        h.write(at(0x2be0), 0u32);
        resize(h, this, 0x15e0, 0x258);
        let mut buf = fill(h, this, 0x15e0, 0x258, INIT_SINGLE, 0x5b94, buf);
        for off in [0x2be4u32, 0x2be8, 0x2bec, 0x2bf0] {
            let v = h.call_sym(INIT_SINGLE, Conv::Thiscall(this), &[buf]);
            buf = buf.wrapping_add(h.read(at(0x5b94)));
            h.write(at(off), v);
        }
        resize(h, this, 0x15e8, 0x320);
        fill(h, this, 0x15e8, 0x320, INIT_MULTI, 0x5b98, buf);
        let buf: u32 = h.read(at(0x5b84));
        resize(h, this, 0x4238, 0x258);
        let buf = fill(h, this, 0x4238, 0x258, INIT_SINGLE, 0x5b94, buf);
        resize(h, this, 0x4240, 0x320);
        fill(h, this, 0x4240, 0x320, INIT_MULTI, 0x5b98, buf);
        for k in 0..0x64u32 {
            h.write(at(0x5848 + 4 * k), 0u32);
        }
        let p = h.rebase(DEFAULT_5B88);
        h.write(at(0x5b88), p);
        for off in [
            0x2c50u32, 0x5b78, 0x2bf8, 0x2bfc, 0x5840, 0x5838, 0x583c, 0x5b68,
        ] {
            h.write(at(off), 0u32);
        }
        h.call_sym(MEMSET, Conv::Cdecl, &[this, 0, 0x15e0]);
        h.write(at(0x5ba0), 0u32);
        h.write(at(0x5b74), 0u8);
        h.write(at(0x5b6c), 0u32);
        h.write(at(0x5b7c), 1u32);
        h.write(at(0x5b8c), 0u32);
        h.write(at(0x5844), 0u32);
        h.write(at(0x5b9c), 0u16);
        this
    }

    /// The three constants `0x006790c0` loads: an `f32` (-1.0, the cloth
    /// flip), an `f32`, and a double subtracted from the doubled offset.
    pub const GRAD_K1: u32 = 0x00b9_2340;
    pub const GRAD_K2: u32 = 0x00b9_2194;
    pub const GRAD_D: u32 = 0x00b9_2248;
    /// The frame of `0x006790c0`: its five factors at `ebp - 0x14..-4`.
    pub const GRAD_FRAME: u32 = 0x14;

    /// `0x006790c0(this; a, mode, c, d)` (thiscall, `ret 0x10`; from
    /// `ShadowManager_setupGradMeshesB`): a gradient mesh's three vertex
    /// blocks and index list. By `mode` the offset `q` and the four
    /// scales `s1..s4` (all rounded to `f32` in the frame): 1: `q = (a -
    /// c) / (d - c)`, `s1 = c / a`; 2: `s3 = c / a`, `s4 = d / a`; 3: `q =
    /// -c / (d - c)`, `s1 = s2 = s3 = 0`, `s4 = d / a`; otherwise `q = 0`,
    /// the scales 1. The positions (`+0x1c`, 24 floats) take the two
    /// constants and `r = 2q - D` (rounded into the caller's `c` slot,
    /// `slots + 8`), the second block (`+0x17c`, 16) `q`, the third
    /// (`+0xfc`, 24) the scales; the 12 indices at `+0x1bc`. Returns
    /// `this + 0x1c`, as `eax` ends.
    #[allow(clippy::many_single_char_names)] // the original's operands
    pub fn grad_mesh(h: &mut impl Host, ebp: u32, this: u32, mode: u32, slots: u32) -> u32 {
        let at = |off: u32| ebp.wrapping_sub(off);
        let arg = |k: u32| slots.wrapping_add(4 * k);
        let a = x87::load(h.read(arg(0)));
        let c = x87::load(h.read(arg(2)));
        let d = x87::load(h.read(arg(3)));
        let (mut q, mut s) = (0.0, [1.0; 4]);
        match mode {
            1 => {
                q = x87::div(x87::sub(a, c), x87::sub(d, c));
                s[0] = x87::div(c, a);
            }
            3 => {
                q = x87::div(x87::sub(0.0, c), x87::sub(d, c));
                s = [0.0, 0.0, 0.0, x87::div(d, a)];
            }
            2 => {
                s[2] = x87::div(c, a);
                s[3] = x87::div(d, a);
            }
            _ => {}
        }
        // the frame: q at -4, s1 -0x10, s2 -0x14, s3 -8, s4 -0xc
        h.write(at(4), x87::store(q));
        for (off, v) in [0x10u32, 0x14, 8, 0xc].into_iter().zip(s) {
            h.write(at(off), x87::store(v));
        }
        let q_bits: u32 = h.read(at(4));
        let q = x87::load(q_bits);
        let k1: u32 = h.read(h.rebase(GRAD_K1));
        let k2: u32 = h.read(h.rebase(GRAD_K2));
        let dd: f64 = h.read(h.rebase(GRAD_D));
        let r_bits = x87::store(x87::sub(x87::add(q, q), dd));
        h.write(slots.wrapping_add(8), r_bits);
        let (one, zero) = (1f32.to_bits(), 0f32.to_bits());
        let (k1, k2) = (x87::store(x87::load(k1)), x87::store(x87::load(k2)));
        let r = x87::store(x87::load(r_bits));
        let positions = [
            k1, k1, k2, one, k1, k2, k1, r, k2, one, r, k2, k1, r, k2, one, r, k2, k1, one, k2,
            one, one, k2,
        ];
        let q = q_bits;
        let second = [
            zero, zero, one, zero, zero, q, one, q, zero, q, one, q, zero, one, one, one,
        ];
        let s: [u32; 4] = [0x10u32, 0x14, 8, 0xc].map(|off| h.read(at(off)));
        let mut third = Vec::with_capacity(24);
        for sk in s {
            third.extend([one, sk, one, one, sk, one]);
        }
        for (block, values) in [
            (0x1cu32, &positions[..]),
            (0x17c, &second[..]),
            (0xfc, &third[..]),
        ] {
            for (k, v) in (0u32..).zip(values) {
                h.write(this.wrapping_add(block + 4 * k), *v);
            }
        }
        for (k, v) in (0u32..).zip([0u32, 1, 2, 3, 2, 1, 4, 5, 6, 7, 6, 5]) {
            h.write(this.wrapping_add(0x1bc + 4 * k), v);
        }
        this.wrapping_add(0x1c)
    }

    /// The graphic object's destructor's callees: `Phyre_PhyreFree`,
    /// `user_free_thunk`, `operator delete` (cdecl); member destructors
    /// (thiscall): `0x00636970`, `Phyre_PFreeList_dtor_PFreeList`,
    /// `Phyre_PFreeList_reset`, `Phyre_PRedBlackTree_clear`,
    /// `Phyre_PRedBlackTree_orphanAllContents`, `0x00585230`,
    /// `BufferTree_dtor_00633fe0`, `Phyre_PCluster_dtor_PCluster`,
    /// `0x006370d0` (a tree's erase, `ret 0xc`), `AssetListReader_dtor`,
    /// `PVideoPlaybackWin32_dtor_00a26f30`, `Phyre_PWorld_releaseEntries`,
    /// `ListVector_dtor_00634220`; `eh_vector_destructor_iterator`
    /// (stdcall, 4 words); `CloseHandle` (stdcall); the base vtable the
    /// six texture holders get back (`0x00b10f5c`) and `0x004be960`, an
    /// element destructor.
    pub const DTOR_636970: u32 = 0x0063_6970;
    pub const FREE_LIST_DTOR: u32 = 0x0049_c5b0;
    pub const FREE_LIST_RESET: u32 = 0x0049_cc10;
    pub const TREE_CLEAR: u32 = 0x0057_4760;
    pub const TREE_ORPHAN_ALL: u32 = 0x0057_4970;
    pub const DTOR_585230: u32 = 0x0058_5230;
    pub const BUFFER_TREE_DTOR: u32 = 0x0063_3fe0;
    pub const CLUSTER_DTOR: u32 = 0x0043_fa30;
    pub const TREE_ERASE: u32 = 0x0063_70d0;
    pub const ASSET_LIST_DTOR: u32 = 0x0063_4610;
    pub const VIDEO_DTOR: u32 = 0x00a2_6f30;
    pub const WORLD_RELEASE: u32 = 0x0044_2a80;
    pub const LIST_VECTOR_DTOR: u32 = 0x0063_4220;
    pub const VECTOR_DTOR_ITERATOR: u32 = 0x0094_96d2;
    pub const CLOSE_HANDLE_IAT: u32 = 0x00b0_c0dc;
    pub const HOLDER_VTABLE: u32 = 0x00b1_0f5c;
    pub const ELEMENT_DTOR_4BE960: u32 = 0x004b_e960;
    pub const PHYRE_FREE_FN: u32 = 0x0042_fb20;
    pub const USER_FREE_THUNK: u32 = 0x0068_7010;
    pub const OPERATOR_DELETE: u32 = 0x0063_0390;
    /// The frame of `0x00634850`: `this` at `ebp - 0x10`, the scratch
    /// word at `ebp - 0x14` (the trees' addresses and the erase's
    /// iterator).
    pub const FINAL_FRAME: u32 = 0x14;

    /// One step of the graphic object's member-wise destructor.
    #[derive(Debug, Clone, Copy)]
    enum Step {
        /// A `PArray` (`+0` the count, its top bit set for a borrowed
        /// block; `+4` the block): freed (`Phyre_PhyreFree`) when owned
        /// and, with `check`, set; both words cleared.
        Array { at: u32, check: bool },
        /// A tree with a pooled allocator (the `0x00636970` set or, with
        /// `orphan`, the orphaning one), its address in the frame.
        Pooled { at: u32, orphan: bool },
        /// `user_free_thunk` of the word when set.
        UserFree(u32),
        /// `Phyre_PhyreFree` of a tagged word (bit 0 set: inline).
        Tagged(u32),
        /// A member destructor (thiscall).
        Dtor(u32, u32),
        /// `eh_vector_destructor_iterator(at, size, count, dtor)`.
        Elements {
            at: u32,
            size: u32,
            count: u32,
            dtor: u32,
        },
        /// `CloseHandle` of the word.
        Close(u32),
        /// A vector freed when set (`operator delete`), its words cleared.
        Vector(u32),
        /// A `std` tree erased (`0x006370d0`, iterator in the frame) and
        /// its head deleted.
        Tree(u32),
        /// A texture holder (`at`: its vtable back to the base) and its
        /// tagged name word.
        Holder(u32, u32),
    }

    /// `0x00634850`'s steps, in its order.
    const FINAL_STEPS: &[Step] = &[
        Step::Array {
            at: 0x10d84,
            check: true,
        },
        Step::Pooled {
            at: 0x10d54,
            orphan: false,
        },
        Step::UserFree(0xfbf8),
        Step::UserFree(0xfbe0),
        Step::UserFree(0xfbc4),
        Step::UserFree(0xfbb0),
        Step::UserFree(0xfb9c),
        Step::UserFree(0xfb88),
        Step::UserFree(0xfb5c),
        Step::UserFree(0xfb50),
        Step::Close(0xfb3c),
        Step::UserFree(0xfb38),
        Step::UserFree(0xfb2c),
        Step::UserFree(0xfb20),
        Step::Vector(0xfb08),
        Step::Tagged(0xfb00),
        Step::Tagged(0xfafc),
        Step::Tagged(0xfaf0),
        Step::Dtor(DTOR_585230, 0xfacc),
        Step::Dtor(DTOR_585230, 0xfaa8),
        Step::Tagged(0xfa6c),
        Step::Dtor(BUFFER_TREE_DTOR, 0xf9d4),
        Step::Dtor(CLUSTER_DTOR, 0xf96c),
        Step::Array {
            at: 0x38c4,
            check: false,
        },
        Step::Array {
            at: 0x38bc,
            check: false,
        },
        Step::Array {
            at: 0x38b4,
            check: false,
        },
        Step::Array {
            at: 0x38ac,
            check: false,
        },
        Step::Array {
            at: 0x38a4,
            check: false,
        },
        Step::Array {
            at: 0x3898,
            check: false,
        },
        Step::Array {
            at: 0x3890,
            check: false,
        },
        Step::Array {
            at: 0x3884,
            check: false,
        },
        Step::UserFree(0x3880),
        Step::UserFree(0x3874),
        Step::UserFree(0x3868),
        Step::UserFree(0x385c),
        Step::Elements {
            at: 0x3314,
            size: 0x50,
            count: 0x10,
            dtor: CLUSTER_DTOR,
        },
        Step::Elements {
            at: 0x2e14,
            size: 0x50,
            count: 0x10,
            dtor: CLUSTER_DTOR,
        },
        Step::Elements {
            at: 0x2914,
            size: 0x50,
            count: 0x10,
            dtor: CLUSTER_DTOR,
        },
        Step::Dtor(CLUSTER_DTOR, 0x28c4),
        Step::Dtor(CLUSTER_DTOR, 0x2874),
        Step::Elements {
            at: 0x1374,
            size: 0x50,
            count: 0x40,
            dtor: CLUSTER_DTOR,
        },
        Step::Dtor(CLUSTER_DTOR, 0x1324),
        Step::Dtor(CLUSTER_DTOR, 0x12d4),
        Step::Dtor(CLUSTER_DTOR, 0x1284),
        Step::Dtor(CLUSTER_DTOR, 0x1234),
        Step::Dtor(CLUSTER_DTOR, 0x11e4),
        Step::Tree(0x11dc),
        Step::UserFree(0x11d0),
        Step::Pooled {
            at: 0x1198,
            orphan: true,
        },
        Step::UserFree(0x1190),
        Step::Dtor(ASSET_LIST_DTOR, 0x1144),
        Step::UserFree(0x1140),
        Step::UserFree(0x1134),
        Step::Dtor(ASSET_LIST_DTOR, 0x10e4),
        Step::UserFree(0x10e0),
        Step::Dtor(VIDEO_DTOR, 0xde8),
        Step::Holder(0xbdc, 0xcac),
        Step::Holder(0xa84, 0xb54),
        Step::Elements {
            at: 0x964,
            size: 0x24,
            count: 8,
            dtor: ELEMENT_DTOR_4BE960,
        },
        Step::Dtor(DTOR_585230, 0x940),
        Step::Holder(0x7e4, 0x8b4),
        Step::Holder(0x658, 0x728),
        Step::Holder(0x4fc, 0x5cc),
        Step::Holder(0x398, 0x468),
        Step::Holder(0x23c, 0x30c),
        Step::Holder(0xe0, 0x1b0),
        Step::Dtor(WORLD_RELEASE, 0xbc),
        Step::Dtor(WORLD_RELEASE, 0xac),
        Step::Dtor(WORLD_RELEASE, 0x9c),
        Step::Dtor(WORLD_RELEASE, 0x8c),
        Step::Dtor(WORLD_RELEASE, 0x7c),
        Step::UserFree(0x60),
        Step::UserFree(0x54),
        Step::UserFree(0x48),
        Step::UserFree(0x3c),
        Step::UserFree(0x30),
    ];

    fn tagged_free(h: &mut impl Host, at: u32) {
        let w: u32 = h.read(at);
        if w & 1 == 0 {
            h.call_sym(PHYRE_FREE_FN, Conv::Cdecl, &[w]);
        }
    }

    /// `0x00634850(this)` (thiscall; from `graphicFinalize`): the graphic
    /// object's members destroyed in reverse order ([`FINAL_STEPS`]), the
    /// last the list vector at `+4` (`ListVector_dtor_00634220`), whose
    /// `eax` it returns.
    pub fn graphic_final(h: &mut impl Host, ebp: u32, this: u32) -> u32 {
        let scratch = ebp.wrapping_sub(0x14);
        h.write(ebp.wrapping_sub(0x10), this);
        let at = |off: u32| this.wrapping_add(off);
        for &step in FINAL_STEPS {
            match step {
                Step::Array { at: off, check } => {
                    let count: u32 = h.read(at(off));
                    let block: u32 = h.read(at(off + 4));
                    if count >> 31 == 0 && (!check || block != 0) {
                        h.call_sym(PHYRE_FREE_FN, Conv::Cdecl, &[block]);
                    }
                    h.write(at(off + 4), 0u32);
                    h.write(at(off), 0u32);
                }
                Step::Pooled { at: off, orphan } => {
                    let tree = at(off);
                    h.write(scratch, tree);
                    let pool = tree.wrapping_add(0x18);
                    if orphan {
                        h.call_sym(TREE_ORPHAN_ALL, Conv::Thiscall(tree), &[]);
                        h.call_sym(FREE_LIST_RESET, Conv::Thiscall(pool), &[]);
                    } else {
                        h.call_sym(DTOR_636970, Conv::Thiscall(tree), &[]);
                    }
                    h.call_sym(FREE_LIST_DTOR, Conv::Thiscall(pool), &[]);
                    h.call_sym(TREE_CLEAR, Conv::Thiscall(tree), &[]);
                    h.call_sym(TREE_CLEAR, Conv::Thiscall(tree), &[]);
                }
                Step::UserFree(off) => {
                    let p: u32 = h.read(at(off));
                    if p != 0 {
                        h.call_sym(USER_FREE_THUNK, Conv::Cdecl, &[p]);
                    }
                }
                Step::Tagged(off) => tagged_free(h, at(off)),
                Step::Dtor(f, off) => {
                    h.call_sym(f, Conv::Thiscall(at(off)), &[]);
                }
                Step::Elements {
                    at: off,
                    size,
                    count,
                    dtor,
                } => {
                    let dtor = h.rebase(dtor);
                    h.call_sym(
                        VECTOR_DTOR_ITERATOR,
                        Conv::Stdcall,
                        &[at(off), size, count, dtor],
                    );
                }
                Step::Close(off) => {
                    let handle: u32 = h.read(at(off));
                    let close: u32 = h.read(h.rebase(CLOSE_HANDLE_IAT));
                    h.call(close, Conv::Stdcall, &[handle]);
                }
                Step::Vector(off) => {
                    let first: u32 = h.read(at(off));
                    if first != 0 {
                        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[first]);
                        for k in 0..3 {
                            h.write(at(off + 4 * k), 0u32);
                        }
                    }
                }
                Step::Tree(off) => {
                    let head: u32 = h.read(at(off));
                    let left: u32 = h.read(head);
                    h.call_sym(TREE_ERASE, Conv::Thiscall(at(off)), &[scratch, left, head]);
                    let head: u32 = h.read(at(off));
                    h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[head]);
                }
                Step::Holder(vt, name) => {
                    let base = h.rebase(HOLDER_VTABLE);
                    h.write(at(vt), base);
                    tagged_free(h, at(name));
                }
            }
        }
        h.call_sym(LIST_VECTOR_DTOR, Conv::Thiscall(at(4)), &[])
    }
}
