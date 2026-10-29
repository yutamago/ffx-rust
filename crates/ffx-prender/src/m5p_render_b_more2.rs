//! M5 stage 1, list `phyre-render-b`, batch 6: the render thread's and
//! the renderer job's teardown, the command buffers' hand-off to the
//! preprocess ring, the shader programs' COM release, the render
//! interface's wide-string conversion, the dynamic mesh's modifier
//! packet, the character lighting's destructor and a vorbis lookup
//! table's release.

/// `Phyre_PhyreFree`, `Phyre_PhyreMalloc_0042fb80(size, align)` (cdecl).
pub const PHYRE_FREE: u32 = 0x0042_fb20;
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
/// `0x00430240(event)` (thiscall): an event's close.
pub const EVENT_CLOSE: u32 = 0x0043_0240;
/// The CRT's `eh vector destructor iterator(p, size, count, dtor)`
/// (stdcall) and the two element destructors the render thread uses.
pub const EH_VECTOR_DTOR: u32 = 0x0094_96d2;
pub const SLOT_DTOR: u32 = 0x0043_1740;
pub const RING_ELEMENT_DTOR: u32 = 0x0061_4210;
/// `0x0060ec70` (thiscall): the thread base's destructor body.
pub const THREAD_BASE_DTOR: u32 = 0x0060_ec70;
/// `Phyre_PPreprocessRingBuffer_GetSingleton` (cdecl) and `_allocate(ring;
/// size)` (thiscall); `crt_memcpy_thunk` (cdecl).
pub const RING_SINGLETON: u32 = 0x005c_d010;
pub const RING_ALLOCATE: u32 = 0x005c_d0f0;
pub const MEMCPY: u32 = 0x0094_9256;
/// `Phyre_PRedBlackTreeMap_removeAllDtor_006a68f0`,
/// `Phyre_PRedBlackTree_orphanAllContents`, `_clear`,
/// `Phyre_PFreeList_reset`, `_dtor_PFreeList`, `Phyre_PCluster_dtor_PCluster`
/// (thiscall).
pub const MAP_REMOVE_ALL: u32 = 0x006a_68f0;
pub const TREE_ORPHAN_ALL: u32 = 0x0057_4970;
pub const TREE_CLEAR: u32 = 0x0057_4760;
pub const FREE_LIST_RESET: u32 = 0x0049_cc10;
pub const FREE_LIST_DTOR: u32 = 0x0049_c5b0;
pub const CLUSTER_DTOR: u32 = 0x0043_fa30;
/// `PModifierNetwork_runPacket_005c1f90(packet)` (cdecl).
pub const RUN_PACKET: u32 = 0x005c_1f90;
/// `0x005cd3b0` (thiscall): the ring-buffered job's base destructor body.
pub const RING_JOB_DTOR: u32 = 0x005c_d3b0;
/// The vorbis allocator's free pointer (cdecl `(p)`) and `memset`.
pub const VORBIS_FREE: u32 = 0x00c8_b240;
pub const MEMSET: u32 = 0x0094_964c;

/// The render-target size check's callees (thiscall):
/// `Phyre_PRendering_PTexture2DD3D11_isBound` (`al`), `_unbind`,
/// `Phyre_PRendering_PTexture2DBase_setDimensions(texture; w, h, word 0, 0)`.
pub const IS_BOUND: u32 = 0x0059_5f90;
pub const UNBIND: u32 = 0x0059_cc40;
pub const SET_DIMENSIONS: u32 = 0x004c_40c0;
/// The render interface's teardown callees (thiscall): the scene
/// parameters' release `0x004fdca0`, `PShaderD3D11_release_0059c190`,
/// `0x0059c230`, `PRenderTargetD3D11_releaseViews_0059c650`,
/// `PShaderD3D11_releaseViews_0059c950`, `PSamplerStateD3D11_release_0059c770`,
/// `Phyre_PRendering_releaseComPairs_0059c330`,
/// `Phyre_PRendering_releaseComEight_0059c370`.
pub const SCENE_RELEASE: u32 = 0x004f_dca0;
pub const SHADERS_RELEASE: u32 = 0x0059_c190;
pub const PROGRAM_RELEASE: u32 = 0x0059_c230;
pub const TARGET_RELEASE_VIEWS: u32 = 0x0059_c650;
pub const SHADER_RELEASE_VIEWS: u32 = 0x0059_c950;
pub const SAMPLER_RELEASE: u32 = 0x0059_c770;
pub const COM_PAIRS: u32 = 0x0059_c330;
pub const COM_EIGHT: u32 = 0x0059_c370;
/// `PBitmapFontText`'s destructor callees (thiscall):
/// `PSpriteArrays_release_00495a10`, `Phyre_PRendering_PMeshInstance_dtor_PMeshInstance`,
/// `Phyre_WaitAndClear_0055e060`, `PArray_release40_00433900`,
/// `Phyre_PGeometry_PMesh_dtor_00433af0`.
pub const SPRITE_ARRAYS_RELEASE: u32 = 0x0049_5a10;
pub const MESH_INSTANCE_DTOR: u32 = 0x004f_2c50;
pub const WAIT_AND_CLEAR: u32 = 0x0055_e060;
pub const ARRAY40_RELEASE: u32 = 0x0043_3900;
pub const MESH_DTOR: u32 = 0x0043_3af0;

/// The colour scale (a `double`, 1/255).
pub const COLOUR_SCALE: u32 = 0x00b3_4258;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, f: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Cdecl, args)
    }

    fn this_call(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    fn release_com(h: &mut impl Host, at: u32) {
        let obj: u32 = h.read(at);
        if obj != 0 {
            let vt: u32 = h.read(obj);
            let f: u32 = h.read(vt + 8);
            h.call(f, Conv::Stdcall, &[obj]);
            h.write(at, 0u32);
        }
    }

    /// A `PArray`'s owned non-null buffer freed, both words cleared.
    fn release_array(h: &mut impl Host, at: u32) {
        let count: u32 = h.read(at);
        let buffer: u32 = h.read(at + 4);
        if count & 0x8000_0000 == 0 && buffer != 0 {
            cdecl(h, PHYRE_FREE, &[buffer]);
        }
        h.write(at + 4, 0u32);
        h.write(at, 0u32);
    }

    /// `0x00614220(thread)`: the render thread's destructor body: the
    /// four events at `+0x16c` closed where open (their byte `+4`
    /// cleared), `+0x18c` = 0, the eight-byte slots and the sixteen
    /// 0x14-byte commands destroyed through the CRT's vector destructor,
    /// then the thread base (`+8`).
    pub fn render_thread_dtor(h: &mut impl Host, this: u32) {
        for k in 0..4 {
            let flag = this + 0x170 + 8 * k;
            if h.read::<u8>(flag) != 0 {
                this_call(h, EVENT_CLOSE, flag - 4, &[]);
                h.write(flag, 0u8);
            }
        }
        h.write(this + 0x18c, 0u32);
        let slot = h.rebase(SLOT_DTOR);
        h.call_sym(EH_VECTOR_DTOR, Conv::Stdcall, &[this + 0x16c, 8, 4, slot]);
        let ring = h.rebase(RING_ELEMENT_DTOR);
        h.call_sym(
            EH_VECTOR_DTOR,
            Conv::Stdcall,
            &[this + 0x10, 0x14, 0x10, ring],
        );
        this_call(h, THREAD_BASE_DTOR, this + 8, &[]);
    }

    /// `0x005b99a0(renderer; job)`: the renderer's records (`+0xc`,
    /// `[+0x2318]` 0x20-byte records) and its command bytes (`+0x200c` up
    /// to `[+0x2310]`) copied into one preprocess-ring block (`job +
    /// 0x14`); for a command count (`+0x2314`) of 0 or more the job's
    /// `PArray` at `+0x2c` points at the copied commands (external, the
    /// old owned buffer freed).
    pub fn hand_off(h: &mut impl Host, this: u32, job: u32) {
        let records = h.read::<u32>(this + 0x2318) << 5;
        let commands = h
            .read::<u32>(this + 0x2310)
            .wrapping_sub(this)
            .wrapping_sub(0x200c);
        let ring = cdecl(h, RING_SINGLETON, &[]);
        let block = this_call(h, RING_ALLOCATE, ring, &[records.wrapping_add(commands)]);
        let data = block.wrapping_add(0x10);
        cdecl(h, MEMCPY, &[data, this + 0xc, records]);
        let cmds = data.wrapping_add(records);
        cdecl(h, MEMCPY, &[cmds, this + 0x200c, commands]);
        h.write(job + 0x14, block);
        let n: u32 = h.read(this + 0x2314);
        if (n as i32) < 0 {
            return;
        }
        let old: u32 = h.read(job + 0x30);
        if old != cmds && h.read::<u32>(job + 0x2c) & 0x8000_0000 == 0 && old != 0 {
            cdecl(h, PHYRE_FREE, &[old]);
        }
        h.write(job + 0x30, cmds);
        h.write(job + 0x2c, n | 0x8000_0000);
    }

    /// `0x006a69b0(lighting)`: `CharacterLighting`'s destructor body: the
    /// map at `+0x1f0` emptied, the tree at `+0x158` orphaned with its free
    /// list (`+0x170`) reset and destroyed and the tree cleared twice, the
    /// clusters at `+0xa0`, `+0x50`, `+0` destroyed.
    pub fn character_lighting_dtor(h: &mut impl Host, this: u32) {
        this_call(h, MAP_REMOVE_ALL, this + 0x1f0, &[]);
        let tree = this + 0x158;
        this_call(h, TREE_ORPHAN_ALL, tree, &[]);
        this_call(h, FREE_LIST_RESET, tree + 0x18, &[]);
        this_call(h, FREE_LIST_DTOR, tree + 0x18, &[]);
        this_call(h, TREE_CLEAR, tree, &[]);
        this_call(h, TREE_CLEAR, tree, &[]);
        for at in [0xa0, 0x50, 0] {
            this_call(h, CLUSTER_DTOR, this + at, &[]);
        }
    }

    /// `0x0059c230(program)`: the eight stages' COM objects (from `+0x194`,
    /// 0x17c bytes apart: `+0`, `+4`, `+0xc`, `+8` that order), then
    /// `+0`, `+8`, the eight at `+0xc..0x28` and `+4` released and
    /// cleared; 0.
    pub fn program_release(h: &mut impl Host, this: u32) -> u32 {
        for k in 0..8 {
            let at = this + 0x194 + 0x17c * k;
            for off in [0, 4, 0xc, 8] {
                release_com(h, at + off);
            }
        }
        release_com(h, this);
        release_com(h, this + 8);
        for k in 0..8 {
            release_com(h, this + 0xc + 4 * k);
        }
        release_com(h, this + 4);
        0
    }

    /// `0x00597f20`, `0x00597ff0(state; s)`: 1 for a null string; else the
    /// string widened into a `PhyreMalloc` buffer (2 bytes a character,
    /// the terminator too) that is freed again at once (its use was
    /// compiled out); 0, or 0xd when the allocation fails.
    pub fn widen(h: &mut impl Host, s: u32) -> u32 {
        if s == 0 {
            return 1;
        }
        let mut len = 0u32;
        while h.read::<u8>(s.wrapping_add(len)) != 0 {
            len = len.wrapping_add(1);
        }
        let n = len.wrapping_add(1);
        let buf = if n == 0 {
            0
        } else {
            cdecl(h, PHYRE_MALLOC, &[n.wrapping_mul(2), 2])
        };
        if buf == 0 {
            return 0xd;
        }
        let mut k = 0u32;
        loop {
            let c: u8 = h.read(s.wrapping_add(k));
            h.write(buf.wrapping_add(2 * k), i16::from(c as i8) as u16);
            if c == 0 {
                break;
            }
            k = k.wrapping_add(1);
        }
        if n & 0x8000_0000 == 0 {
            cdecl(h, PHYRE_FREE, &[buf]);
        }
        0
    }

    /// `0x005ae1d0(mesh; i, packet, inputs, _, info)`: modifier `i`'s
    /// packet: nothing without its record (`[+0x38] + 16 i`); else the
    /// info's two words to `packet + 0x28` and `packet + 0x2c`, the
    /// inputs' buffers (`+0x3c`, 0 for none) for each of the record's
    /// `[+0x18]` slots whose index (`[+0x30]`) is not negative, the
    /// stride, the stream and the output, then the packet run; its
    /// `eax`.
    pub fn modifier_packet(
        h: &mut impl Host,
        this: u32,
        i: u32,
        packet: u32,
        inputs: u32,
        info: u32,
    ) -> u32 {
        let rec = h.read::<u32>(this + 0x38).wrapping_add(i << 4);
        let code: u32 = h.read(rec);
        if code == 0 {
            return 0;
        }
        let p = packet + 0x10;
        let v: u32 = h.read(info);
        h.write(packet + 0x28, v);
        let v: u32 = h.read(info + 4);
        h.write(p + 0x1c, v);
        let out = h.read::<u32>(this + 0x14).wrapping_add(i << 4);
        let n = h.read::<u32>(code + 0x18) & 0x7fff_ffff;
        let desc: u32 = h.read(p);
        let dst = p.wrapping_add(u32::from(h.read::<u16>(p + 0x12)));
        for j in 0..n {
            let first: u32 = h.read(rec + 8);
            let table: u32 = h.read(this + 0x30);
            let idx: u32 = h.read(table.wrapping_add(first.wrapping_add(j).wrapping_mul(4)));
            if (idx as i32) < 0 {
                continue;
            }
            let input: u32 = h.read(inputs.wrapping_add(idx.wrapping_mul(4)));
            let v = if input == 0 { 0 } else { h.read(input + 0x3c) };
            h.write(dst.wrapping_add(4 * j), v);
        }
        let stride = h.read::<u16>(desc + 0xc) << 2;
        h.write(p + 0xe, stride);
        let first: u32 = h.read(rec + 4);
        let streams: u32 = h.read(this + 0x20);
        h.write(p + 4, streams.wrapping_add(first.wrapping_mul(4)));
        let o: u32 = h.read(out + 8);
        h.write(p + 0x14, o);
        cdecl(h, RUN_PACKET, &[packet])
    }

    /// `0x005b4aa0(job)`: the renderer job's destructor body: its arrays
    /// at `+0x2c`, `+0x24`, `+0x1c` released, then the ring-buffered
    /// job's.
    pub fn renderer_job_dtor(h: &mut impl Host, this: u32) {
        for at in [0x2c, 0x24, 0x1c] {
            release_array(h, this + at);
        }
        this_call(h, RING_JOB_DTOR, this, &[]);
    }

    /// `0x00a97330(lookup)`: vorbis's lookup tables freed through the
    /// allocator (`+0x10`, `+0x14`, `+0x18` when set; the 17 rows of 8 at
    /// `+8` and the table; the 3 at `+0xc` and the table), then the
    /// 0x34-byte record zeroed; nothing for null.
    pub fn vorbis_lookup_free<H: Host>(h: &mut H, this: u32) {
        if this == 0 {
            return;
        }
        let free_at = h.rebase(VORBIS_FREE);
        let free = |h: &mut H, p: u32| {
            let f: u32 = h.read(free_at);
            h.call(f, Conv::Cdecl, &[p]);
        };
        for at in [0x10, 0x14, 0x18] {
            let p: u32 = h.read(this + at);
            if p != 0 {
                free(h, p);
            }
        }
        if h.read::<u32>(this + 8) != 0 {
            for row in 0..17u32 {
                for k in 0..8u32 {
                    let rows: u32 = h.read(this + 8);
                    let r: u32 = h.read(rows + 4 * row);
                    let p: u32 = h.read(r + 4 * k);
                    free(h, p);
                }
                let rows: u32 = h.read(this + 8);
                let r: u32 = h.read(rows + 4 * row);
                free(h, r);
            }
            let rows: u32 = h.read(this + 8);
            free(h, rows);
        }
        if h.read::<u32>(this + 0xc) != 0 {
            for k in 0..3u32 {
                let t: u32 = h.read(this + 0xc);
                let p: u32 = h.read(t + 4 * k);
                free(h, p);
            }
            let t: u32 = h.read(this + 0xc);
            free(h, t);
        }
        cdecl(h, MEMSET, &[this, 0, 0x34]);
    }

    /// A render target's size check (as `m5p_render_b_post2`'s): 1 for a
    /// kind without a texture, an unbind or resize error, else `None`.
    fn resize_handle(h: &mut impl Host, handle: u32, w: u32, hgt: u32) -> Option<u32> {
        if h.read::<u8>(handle + 4) != 0 {
            return Some(1);
        }
        let tex: u32 = h.read(handle);
        if tex == 0 {
            return Some(1);
        }
        if h.read::<u32>(tex + 0x1c) == w && h.read::<u32>(tex + 0x20) == hgt {
            return None;
        }
        if this_call(h, IS_BOUND, tex, &[]) & 0xff != 0 {
            let r = this_call(h, UNBIND, tex, &[]);
            if r != 0 {
                return Some(r);
            }
        }
        let word0: u32 = h.read(tex);
        let r = this_call(h, SET_DIMENSIONS, tex, &[w, hgt, word0, 0]);
        if r != 0 { Some(r) } else { None }
    }

    /// `0x00a233f0(glow; w, h)`: the glow's three levels of two targets
    /// each (`+0x78`, `+0x84` and on) resized from a quarter of the size,
    /// halved per level; 1 at a missing handle or texture, an error, or 0.
    pub fn glow_resize(h: &mut impl Host, this: u32, w: u32, hgt: u32) -> u32 {
        let (mut w, mut hgt) = (w >> 2, hgt >> 2);
        for k in 0..3 {
            for at in [0x78, 0x84] {
                let handle: u32 = h.read(this + at + 4 * k);
                if handle == 0 {
                    return 1;
                }
                if let Some(r) = resize_handle(h, handle, w, hgt) {
                    return r;
                }
            }
            w >>= 1;
            hgt >>= 1;
        }
        0
    }

    /// `0x00a23780(ssr; w, h)`: the reflection's targets at `+0x68`,
    /// `+0x70`, `+0x6c` (each skipped when null) resized to half the size;
    /// 1 for a kind without a texture, an error, or 0.
    pub fn ssr_resize(h: &mut impl Host, this: u32, w: u32, hgt: u32) -> u32 {
        for at in [0x68, 0x70, 0x6c] {
            let handle: u32 = h.read(this + at);
            if handle == 0 {
                continue;
            }
            if let Some(r) = resize_handle(h, handle, w >> 1, hgt >> 1) {
                return r;
            }
        }
        0
    }

    /// `0x005bc500(dst, src, n)` (cdecl): `n` 0x14-byte `PModifierNetwork`
    /// records copied (each where the destination is not null; the source
    /// advances only then): three words, and the `PArray` of 8-byte
    /// elements at `+0xc` deep-copied into a new `PhyreMalloc` buffer (an
    /// empty array when there is none or the allocation fails).
    pub fn copy_networks(h: &mut impl Host, dst: u32, src: u32, n: u32) {
        let mut s = src;
        for i in 0..n {
            let d = dst.wrapping_add(i.wrapping_mul(0x14));
            if d == 0 {
                continue;
            }
            let head: [u32; 3] = h.read(s);
            h.write(d, head);
            h.write(d + 0xc, 0u32);
            let count = h.read::<u32>(s + 0xc) & 0x7fff_ffff;
            let from: u32 = h.read(s + 0x10);
            let buf = if count == 0 {
                0
            } else {
                cdecl(h, PHYRE_MALLOC, &[count << 3, 4])
            };
            if buf != 0 {
                let mut f = from;
                for k in 0..count {
                    let e = buf.wrapping_add(8 * k);
                    if e != 0 {
                        let v: [u32; 2] = h.read(f);
                        h.write(e, v);
                        f = f.wrapping_add(8);
                    }
                }
            }
            h.write(d + 0x10, buf);
            h.write(d + 0xc, if buf == 0 { 0 } else { count });
            s = s.wrapping_add(0x14);
        }
    }

    /// `0x0059c400(interface)`: the render interface's teardown: its
    /// scene parameters (`0x004fdca0`), the context's state cleared
    /// (methods `+0x84(0, 0, 0)`, `+0x1b8`, `+0x1bc`), the shaders, the
    /// program, the two targets, the views, the sampler, the two buffers
    /// (`+0x340`, `+0x344`) freed, the COM pairs, the query `+0x378` and
    /// the swap chain `+0x74` released, the context cleared again and
    /// released, the device (`+0x6c`) released.
    pub fn interface_release(h: &mut impl Host, this: u32) {
        this_call(h, SCENE_RELEASE, this, &[]);
        let ctx: u32 = h.read(this + 0x70);
        if ctx != 0 {
            com(h, this + 0x70, 0x84, &[0, 0, 0]);
            com(h, this + 0x70, 0x1b8, &[]);
            com(h, this + 0x70, 0x1bc, &[]);
        }
        this_call(h, SHADERS_RELEASE, this + 0x394, &[]);
        this_call(h, PROGRAM_RELEASE, this + 0x6ec, &[]);
        this_call(h, TARGET_RELEASE_VIEWS, this + 0x1a0, &[]);
        this_call(h, TARGET_RELEASE_VIEWS, this + 0x1c8, &[]);
        this_call(h, SHADER_RELEASE_VIEWS, this + 0x570, &[]);
        this_call(h, SAMPLER_RELEASE, this + 0x12f8, &[]);
        for at in [0x340, 0x344] {
            let p: u32 = h.read(this + at);
            if p != 0 {
                cdecl(h, PHYRE_FREE, &[p]);
                h.write(this + at, 0u32);
            }
        }
        this_call(h, COM_PAIRS, this + 0x320, &[]);
        this_call(h, COM_EIGHT, this + 0x348, &[]);
        release_com(h, this + 0x378);
        release_com(h, this + 0x74);
        let ctx: u32 = h.read(this + 0x70);
        if ctx != 0 {
            com(h, this + 0x70, 0x1b8, &[]);
            com(h, this + 0x70, 0x1bc, &[]);
            com(h, this + 0x70, 8, &[]);
            h.write(this + 0x70, 0u32);
        }
        release_com(h, this + 0x6c);
    }

    /// COM method `off` of the object in `at` (read again for the call).
    fn com(h: &mut impl Host, at: u32, off: u32, args: &[u32]) {
        let obj: u32 = h.read(at);
        let vt: u32 = h.read(obj);
        let f: u32 = h.read(vt + off);
        let mut words = [0u32; 4];
        words[0] = obj;
        words[1..=args.len()].copy_from_slice(args);
        h.call(f, Conv::Stdcall, &words[..=args.len()]);
    }

    /// `0x006abc20(this)`: the four `PArray`s at `+0x18`, `+0x10`, `+8`,
    /// `+0` released.
    pub fn release_four(h: &mut impl Host, this: u32) {
        for at in [0x18, 0x10, 8, 0] {
            release_array(h, this + at);
        }
    }

    /// `0x00a295a0(text)`: `PBitmapFontText`'s destructor body: its sprite
    /// arrays released, the object at `+0x164` destroyed (slot 2), the
    /// char buffer (`+0x15c`) released, the mesh instance (`+0xa4`), the
    /// wait-and-clear at `+0x68`, the 0x40-byte array at `+0x60`, the
    /// `PArray` at `+0x40`, then the mesh base.
    pub fn text_dtor(h: &mut impl Host, this: u32) {
        this_call(h, SPRITE_ARRAYS_RELEASE, this, &[]);
        let obj: u32 = h.read(this + 0x164);
        if obj != 0 {
            h.call_virtual(obj, 2, &[]);
        }
        release_array(h, this + 0x15c);
        this_call(h, MESH_INSTANCE_DTOR, this + 0xa4, &[]);
        this_call(h, WAIT_AND_CLEAR, this + 0x68, &[]);
        this_call(h, ARRAY40_RELEASE, this + 0x60, &[]);
        release_array(h, this + 0x40);
        this_call(h, MESH_DTOR, this, &[]);
    }

    /// `0x00599d80(state; colour as a 64-bit value)`: as
    /// `0x00599cc0` on the low word (the bytes are taken from the 64-bit
    /// value: bits 16, 8, 0 and 24).
    pub fn set_colour64(h: &mut impl Host, this: u32, lo: u32, hi: u32) {
        let v = u64::from(hi) << 32 | u64::from(lo);
        let scale: f64 = h.read(h.rebase(COLOUR_SCALE));
        for (at, shift) in [(0x37c, 16), (0x380, 8), (0x384, 0), (0x388, 24)] {
            let b = f64::from(((v >> shift) & 0xff) as u32);
            h.write(this + at, (b * scale) as f32);
        }
    }
}
