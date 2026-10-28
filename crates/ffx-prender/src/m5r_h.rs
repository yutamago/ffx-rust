//! M5 stage 1, list `render`, batch 8: scene and manager helpers (the
//! loading cluster's move into the 3D world, the mesh matrix push, the
//! dyn-geo buffer swap, the water texture unload, the texture video
//! callback, the instance list visibility pass, a word array resize),
//! 0x20-byte element arrays, the float heap's sort loop, the character
//! table push under its mutex, a shadow-mesh script copy, and three
//! small PS2 movie / cache helpers.
//!
//! Twins of functions ported earlier run those bridges (see the shims).

use crate::m5r_c::quiet;
use ffx_bridge::Global;

/// `g_PhyreScene`.
pub const SCENE: Global<u32> = Global::new(0x00cc_b9d8);
/// The character instance list's mutex handle.
pub const CHARACTER_MUTEX: Global<u32> = Global::new(0x00cc_ca78);
/// `g_PrxState`, `g_PrxModule`.
pub const PRX_STATE: Global<u32> = Global::new(0x00ce_83c4);
pub const PRX_MODULE: Global<u32> = Global::new(0x00ce_83c0);
/// `g_GsRegisterSink`: a write-only port the PS2 GS register writes go
/// to.
pub const GS_SINK: Global<u32> = Global::new(0x0230_56e8);
/// `g_CacheSlots`: 0x64 cache words.
pub const CACHE_SLOTS: u32 = 0x0130_2f98;

pub const WAIT_FOR_SINGLE_OBJECT: u32 = 0x00b0_c0f8;
pub const RELEASE_MUTEX: u32 = 0x00b0_c1c0;
pub const PRINTF: u32 = 0x00b0_c380;
pub const LOAD_LIBRARY_A: u32 = 0x00b0_c07c;

pub const WORLD_REMOVE_CLUSTER: u32 = 0x0044_3b70;
pub const WORLD_ADD_CLUSTER: u32 = 0x0044_3250;
pub const PHYRE_MALLOC_ALIGNED: u32 = 0x0042_fb80;
pub const PHYRE_FREE: u32 = 0x0042_fb20;
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
pub const MESH_MATRIX_PUSH: u32 = 0x0065_fbe0;
/// The swap's "no buffers" message.
pub const NO_BUFFERS: u32 = 0x00b4_a6e8;
pub const UNIFY_FILENAME: u32 = 0x0067_9820;
pub const SCENE_UNLOAD_VFX_TEXTURE: u32 = 0x0067_1300;
pub const WATER_TEXTURES: u32 = 0x00b4_3ba4;
pub const VIDEO_UPDATE_TEXTURE: u32 = 0x00a2_84a0;
pub const WORD_ARRAY_ALLOC: u32 = 0x0064_f7f0;
pub const ELEMENT20_DTOR: u32 = 0x006a_f690;
pub const FLOAT_HEAP_ADJUST: u32 = 0x006c_7250;
pub const CHARACTER_TABLE_PUSH: u32 = 0x006a_2b20;
pub const SHADOW_GET: u32 = 0x0067_8580;
pub const SHADOW_COPY: u32 = 0x0067_8310;
pub const STR_A: u32 = 0x0077_75e0;
pub const STR_CHECK_HDD: u32 = 0x0077_e8e0;
pub const SCE_IOCTL: u32 = 0x0062_f8a0;
pub const DBG_PRINTF: u32 = 0x0062_f500;
pub const CACHE_DUMP_SLOT: u32 = 0x0083_ff20;
pub const CACHE_DUMP_HEAD: u32 = 0x00b5_a534;
pub const CACHE_DUMP_TAIL: u32 = 0x00b5_a544;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::Mem;
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, f: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Cdecl, args)
    }

    fn method(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    fn word(h: &impl Mem, at: u32) -> u32 {
        h.read(at)
    }

    fn byte(h: &impl Mem, at: u32) -> u8 {
        h.read(at)
    }

    fn import(h: &mut impl Host, slot: u32, conv: Conv, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, conv, args)
    }

    /// `PhyreScene_moveLoadingCluster_00657920(scene)`: the loading
    /// cluster `+0x28c4` into the world `+0x7c`: with `+0xf9d1` set
    /// (cleared) only added there; else unless already there (`+0xf9d0`)
    /// moved from `+0xbc`; `+0xf9d0` set.
    pub fn move_loading_cluster(h: &mut impl Host, scene: u32) {
        let c = scene + 0x28c4;
        if byte(h, scene + 0xf9d1) != 0 {
            h.write(scene + 0xf9d1, 0_u8);
            method(h, WORLD_ADD_CLUSTER, scene + 0x7c, &[c]);
            h.write(scene + 0xf9d0, 1_u8);
            return;
        }
        if byte(h, scene + 0xf9d0) != 0 {
            return;
        }
        method(h, WORLD_REMOVE_CLUSTER, scene + 0xbc, &[c]);
        method(h, WORLD_ADD_CLUSTER, scene + 0x7c, &[c]);
        h.write(scene + 0xf9d0, 1_u8);
    }

    /// `PhyreScene_PushMeshMatrix(scene; mesh, matrix)` (`ret 8`; frame:
    /// the 0x44-byte record at `ebp - 0x44`, the mesh then the 16 matrix
    /// words): a block for one more record (`Phyre_PhyreMalloc_0042fb80(
    /// (n + 1) * 0x44, 4)`, `n` the list's count `+0x10d84`) and
    /// `0x0065fbe0(list; block, record)`'s `eax`.
    pub fn push_mesh_matrix(
        h: &mut impl Host,
        ebp: u32,
        scene: u32,
        mesh: u32,
        matrix: u32,
    ) -> u32 {
        let rec = ebp - 0x44;
        h.write(rec, mesh);
        for k in 0..16 {
            let v = word(h, matrix + 4 * k);
            h.write(rec + 4 + 4 * k, v);
        }
        let list = scene + 0x1_0d84;
        let n = (word(h, list) & 0x7fff_ffff).wrapping_add(1);
        let p = cdecl(h, PHYRE_MALLOC_ALIGNED, &[n.wrapping_mul(0x44), 4]);
        method(h, MESH_MATRIX_PUSH, list, &[p, rec])
    }

    /// `DynGeoMemManager_swapBuffer_006dece0(this)`: without buffers
    /// (`+8`), a message (`printf`); otherwise under the mutex `+0x14` the
    /// index `+4` steps to `(i + 1) % n` (signed), and the current buffer
    /// `[+0x10][i]`'s data `+4` becomes `+0` and its own `+8`; `eax` that
    /// data, or `printf`'s.
    pub fn swap_buffer(h: &mut impl Host, this: u32) -> u32 {
        let n = word(h, this + 8);
        if n == 0 {
            let text = h.rebase(NO_BUFFERS);
            return import(h, PRINTF, Conv::Cdecl, &[text]);
        }
        let m = word(h, this + 0x14);
        import(h, WAIT_FOR_SINGLE_OBJECT, Conv::Stdcall, &[m, 0xffff_ffff]);
        let i = word(h, this + 4).wrapping_add(1).cast_signed();
        let n = word(h, this + 8).cast_signed();
        let next = if n == 0 { 0 } else { i.wrapping_rem(n) };
        h.write(this + 4, next.cast_unsigned());
        let m = word(h, this + 0x14);
        import(h, RELEASE_MUTEX, Conv::Stdcall, &[m]);
        let at = word(h, this + 0x10).wrapping_add(word(h, this + 4).wrapping_mul(4));
        let buf = word(h, at);
        let data = word(h, buf + 4);
        h.write(this, data);
        let buf = word(h, at);
        let data = word(h, buf + 4);
        h.write(buf + 8, data);
        data
    }

    /// `PhyreScene_unloadWaterEffectTexture(scene)` (frame: the name at
    /// `ebp - 0x104`): the water list unified and unloaded (`-1`).
    pub fn unload_water_texture(h: &mut impl Host, ebp: u32, scene: u32) -> u32 {
        let buf = ebp - 0x104;
        let name = h.rebase(WATER_TEXTURES);
        cdecl(h, UNIFY_FILENAME, &[name, buf, 0x100]);
        method(h, SCENE_UNLOAD_VFX_TEXTURE, scene, &[buf, 0xffff_ffff])
    }

    /// `PhyreScene_updateTextureVideoCallback(job)`: the request `[job +
    /// 4]` (video `+0`, three words `+4..+0xc`, flags `+0x10`, busy
    /// `+0x14`): busy = 1; with `[+0x10]` set and the video's
    /// `updateTexture` true the scene's `+0xdd1` is set; then busy = 0 and
    /// the request deleted (`operator_delete`'s `eax`).
    pub fn update_texture_video_callback(h: &mut impl Host, job: u32) -> u32 {
        let r = word(h, job + 4);
        let busy = word(h, r + 0x14);
        h.write(busy, 1_u32);
        let flag = word(h, r + 0x10);
        if word(h, flag) != 0 {
            let video = word(h, r);
            let (a, b, c) = (word(h, r + 4), word(h, r + 8), word(h, r + 0xc));
            let ok = method(h, VIDEO_UPDATE_TEXTURE, video, &[a, b, c]);
            if ok.to_le_bytes()[0] != 0 {
                let scene = SCENE.get(h);
                h.write(scene + 0xdd1, 1_u8);
            }
        }
        let busy = word(h, r + 0x14);
        h.write(busy, 0_u32);
        cdecl(h, OPERATOR_DELETE, &[r])
    }

    /// `PArray_resizeWords_0066c690`/`_0066c700(array; n)` (`ret 4`) ->
    /// 0 (0xd when the block can't be had): a different count gets a new
    /// block of `n` words keeping `min(n, count)` of them (`alloc`), the
    /// old one freed when owned and not the new one.
    pub fn resize_words(h: &mut impl Host, this: u32, n: u32, alloc: u32) -> u32 {
        let c = word(h, this);
        if n == c {
            return 0;
        }
        let keep = (c & 0x7fff_ffff).min(n);
        let data = word(h, this + 4);
        let p = cdecl(h, alloc, &[n, data, keep]);
        if p == 0 && n != 0 {
            return 0xd;
        }
        let old = word(h, this + 4);
        if old != p && word(h, this) >> 31 == 0 {
            cdecl(h, PHYRE_FREE, &[old]);
        }
        h.write(this + 4, p);
        h.write(this, n);
        0
    }

    /// `PhyreScene_setInstanceListOwners_0066c770(scene)`: for every
    /// instance list (`+0xfb48` count, `+0xfb50` data; the list's hidden
    /// byte `+0x14`), each mesh instance's draw record `+0x1c` points back
    /// at it, or 0 when hidden.
    pub fn set_instance_list_owners(h: &mut impl Host, scene: u32) {
        let mut k = 0;
        while k < word(h, scene + 0xfb48) {
            let list = word(h, word(h, scene + 0xfb50) + 4 * k);
            if list != 0 {
                let shown = byte(h, list + 0x14) != 0;
                let mut i = 0;
                while i < word(h, list) & 0x7fff_ffff {
                    let list = word(h, word(h, scene + 0xfb50) + 4 * k);
                    let inst = word(h, word(h, list + 4) + 4 * i);
                    if inst != 0 {
                        let rec = word(h, inst + 0x1c);
                        if rec != 0 {
                            h.write(rec + 0x1c, if shown { inst } else { 0 });
                        }
                    }
                    i += 1;
                }
            }
            k += 1;
        }
    }

    /// The 0x20-byte element arrays' release (`0x006af510`: each
    /// element's `dtor`; `0x006af5d0`: each element's virtual 0 with 0):
    /// an owned array with data has its elements destroyed and is freed;
    /// both words cleared.
    pub fn release_array20(h: &mut impl Host, this: u32, dtor: Option<u32>) {
        let count = word(h, this);
        if count >> 31 == 0 {
            let data = word(h, this + 4);
            if data != 0 {
                let mut e = data;
                for _ in 0..count & 0x7fff_ffff {
                    match dtor {
                        Some(f) => {
                            method(h, f, e, &[]);
                        }
                        None => {
                            h.call_virtual(e, 0, &[0]);
                        }
                    }
                    e = e.wrapping_add(0x20);
                }
                cdecl(h, PHYRE_FREE, &[data]);
            }
        }
        h.write(this + 4, 0_u32);
        h.write(this, 0_u32);
    }

    /// `FloatHeap_sortHeap_006c7890(first, last, pred)`: while more than
    /// one float is left, the front is swapped to the back (through the
    /// x87, the back value parked in its own argument slot `slot`) and
    /// `FloatHeap_AdjustHeap(first, 0, n, &slot, pred)` restores the heap.
    pub fn sort_heap(h: &mut impl Host, slot: u32, first: u32, last: u32, pred: u32) -> u32 {
        let mut bytes = last.wrapping_sub(first);
        let mut r = bytes & !3;
        while (bytes & !3).cast_signed() > 4 {
            let back = first.wrapping_add(bytes).wrapping_sub(4);
            let v = word(h, back);
            h.write(slot, quiet(v));
            let f = word(h, first);
            h.write(back, quiet(f));
            bytes = bytes.wrapping_sub(4);
            let n = (bytes.cast_signed() >> 2).cast_unsigned();
            cdecl(h, FLOAT_HEAP_ADJUST, &[first, 0, n, slot, pred]);
            r = bytes & !3;
        }
        r
    }

    /// `ClassCharacter_pushInstance_006a0d00(this; inst)` (`ret 4`): under
    /// the character mutex, `0x006a2b20(this + 8; &inst)` with its own
    /// argument slot `slot`; `ReleaseMutex`'s result.
    pub fn push_instance(h: &mut impl Host, this: u32, slot: u32) -> u32 {
        let m = CHARACTER_MUTEX.get(h);
        import(h, WAIT_FOR_SINGLE_OBJECT, Conv::Stdcall, &[m, 0xffff_ffff]);
        method(h, CHARACTER_TABLE_PUSH, this + 8, &[slot]);
        let m = CHARACTER_MUTEX.get(h);
        import(h, RELEASE_MUTEX, Conv::Stdcall, &[m])
    }

    /// `TaggedBuffer_free_006c7ec0(this)`: the buffer `+0xcc` freed unless
    /// its tag bit is set (an unwind funclet; no result).
    pub fn tagged_free(h: &mut impl Host, this: u32) {
        if byte(h, this + 0xcc) & 1 == 0 {
            let p = word(h, this + 0xcc);
            cdecl(h, PHYRE_FREE, &[p]);
        }
    }

    /// `ShadowDynamicMesh_copyOut_00678ba0(L, out)` (stdcall): the
    /// accessor's object copied into `out` (`0x00678310(out; object)`),
    /// whose 0x7b words the original then reads (`rep lods`); `eax` the
    /// last of them.
    pub fn shadow_copy_out(h: &mut impl Host, l: u32, out: u32) -> u32 {
        let obj = cdecl(h, SHADOW_GET, &[l]);
        let r = method(h, SHADOW_COPY, out, &[obj]);
        word(h, r.wrapping_add(0x7a * 4))
    }

    /// `MagicDLLAsyncLoad(job)`: `g_PrxState` 2, `LoadLibraryA(job)` into
    /// `g_PrxModule`, the job's callback `+0x84` with it, state 3; the
    /// callback's `eax`.
    pub fn magic_dll_async_load(h: &mut impl Host, job: u32) -> u32 {
        PRX_STATE.set(h, 2);
        let m = import(h, LOAD_LIBRARY_A, Conv::Stdcall, &[job]);
        PRX_MODULE.set(h, m);
        let cb = word(h, job + 0x84);
        let r = h.call(cb, Conv::Cdecl, &[m]);
        PRX_STATE.set(h, 3);
        r
    }

    /// `syncStrCheckPC_HDD(stream)` -> whether `sceIoctl(fd, 1, &word)`
    /// set the frame word (`ebp - 4`, -1 first) to something other than 0;
    /// 0 for a negative fd (`+0x50544`).
    pub fn sync_str_check_hdd(h: &mut impl Host, ebp: u32, stream: u32) -> u32 {
        let w = ebp - 4;
        h.write(w, 0xffff_ffff_u32);
        let fd = word(h, stream + 0x5_0544);
        if fd.cast_signed() < 0 {
            return 0;
        }
        cdecl(h, SCE_IOCTL, &[fd, 1, w]);
        u32::from(word(h, w) != 0)
    }

    /// `syncStrCheck_0077e8b0(stream)`: a tail jump to `0x007775e0` with
    /// the stream's byte `+0x5075c`, else to [`sync_str_check_hdd`].
    pub fn sync_str_check(h: &mut impl Host, stream: u32) -> u32 {
        let f = if byte(h, stream + 0x5_075c) == 0 {
            STR_CHECK_HDD
        } else {
            STR_A
        };
        cdecl(h, f, &[stream])
    }

    /// `GsSetFrameRegs_0077e830(a, b, c)`: the register writes the PS2 code
    /// made, into the sink in order (`0xff0000`, `& 0x7cf`, `b`, `a`, `c /
    /// 16` signed, `0x101`); the last word.
    pub fn gs_set_frame_regs(h: &mut impl Host, a: u32, b: u32, c: u32) -> u32 {
        GS_SINK.set(h, 0x00ff_0000);
        let v = GS_SINK.get(h) & 0x7cf;
        GS_SINK.set(h, v);
        GS_SINK.set(h, b);
        GS_SINK.set(h, a);
        GS_SINK.set(h, (c.cast_signed() / 16).cast_unsigned());
        GS_SINK.set(h, 0x101);
        (c.cast_signed() / 16).cast_unsigned()
    }

    /// `Cache_Dump(tag)`: `dbgPrintf(head, tag)`, every cache slot dumped
    /// (`0x0083ff20`), then a tail jump to `dbgPrintf(tail)` with the
    /// message stored into its own argument slot `slot`.
    pub fn cache_dump(h: &mut impl Host, slot: u32, tag: u32) -> u32 {
        let head = h.rebase(CACHE_DUMP_HEAD);
        cdecl(h, DBG_PRINTF, &[head, tag]);
        let base = h.rebase(CACHE_SLOTS);
        for k in 0..0x64 {
            let s = word(h, base + 4 * k);
            cdecl(h, CACHE_DUMP_SLOT, &[s]);
        }
        let tail = h.rebase(CACHE_DUMP_TAIL);
        h.write(slot, tail);
        cdecl(h, DBG_PRINTF, &[tail])
    }
}
