//! M5 stage 1, list `small`, round 4: the movie player manager's
//! destructor, request queue and video task callback
//! (`PhyFMVPlayerManager`).

/// The originals over game memory.
#[allow(clippy::many_single_char_names)] // registers and indices
pub mod bridge {
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    fn at(p: u32, o: u32) -> u32 {
        p.wrapping_add(o)
    }

    /// `PhyFMVPlayerManager_DestroyFMVPlayer(mgr)` (thiscall).
    pub const DESTROY_FMV_PLAYER: u32 = 0x006d_71b0;
    /// `operator delete(p)`, `Phyre_PhyreFree(p)`, `user_free_thunk(p)`
    /// (cdecl).
    pub const OPERATOR_DELETE: u32 = 0x0063_0390;
    pub const PHYRE_FREE: u32 = 0x0042_fb20;
    pub const USER_FREE: u32 = 0x0068_7010;
    /// `std_Tree_erase_range(map; &out, first, last)` (thiscall, `ret 0xc`).
    pub const TREE_ERASE_RANGE: u32 = 0x006d_8940;
    /// Member destructors (thiscall): the buffers' release, the sampler
    /// state, the cluster, the video playback.
    pub const BUFFER_RELEASE: u32 = 0x004c_3010;
    pub const SAMPLER_DTOR: u32 = 0x0058_5230;
    pub const CLUSTER_DTOR: u32 = 0x0043_fa30;
    pub const PLAYBACK_DTOR: u32 = 0x00a2_6f30;
    /// `Phyre::PCameraProjection`'s vtable.
    pub const CAMERA_PROJECTION_VTABLE: u32 = 0x00b1_0f5c;

    /// `PhyFMVPlayerManager_dtor(mgr)` (`0x006d6d00`, thiscall). `out` is
    /// the original's local (`ebp - 0x14`) that gets the map erase's
    /// result iterator.
    ///
    /// The player destroyed; the list at `+0x724` (sentinel node, size
    /// `+0x728`) emptied and its nodes and sentinel deleted; the map at
    /// `+0x6e8` erased and its head deleted; the camera at `+0x574` back
    /// to `PCameraProjection`; the arrays at `+0x644` and `+0x4e8/+0x4ec`
    /// freed when they own memory; the members destroyed in reverse
    /// order; `+0x18` and `+0xc` freed. Returns the last call's `eax`, or
    /// 0.
    pub fn dtor(h: &mut impl Host, mgr: u32, out: u32) -> u32 {
        h.call_sym(DESTROY_FMV_PLAYER, Conv::Thiscall(mgr), &[]);
        let list: u32 = h.read(at(mgr, 0x724));
        let mut node: u32 = h.read(list);
        h.write(list, list);
        let list: u32 = h.read(at(mgr, 0x724));
        h.write(at(list, 4), list);
        h.write(at(mgr, 0x728), 0u32);
        while node != h.read::<u32>(at(mgr, 0x724)) {
            let next: u32 = h.read(node);
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[node]);
            node = next;
        }
        let list: u32 = h.read(at(mgr, 0x724));
        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[list]);
        let map = at(mgr, 0x6e8);
        let head: u32 = h.read(map);
        let first: u32 = h.read(head);
        h.call_sym(TREE_ERASE_RANGE, Conv::Thiscall(map), &[out, first, head]);
        let head: u32 = h.read(map);
        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[head]);
        let vt = h.rebase(CAMERA_PROJECTION_VTABLE);
        h.write(at(mgr, 0x574), vt);
        if h.read::<u8>(at(mgr, 0x644)) & 1 == 0 {
            let p: u32 = h.read(at(mgr, 0x644));
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p]);
        }
        let n: u32 = h.read(at(mgr, 0x4e8));
        if n & 0x8000_0000 == 0 && n & 0x7fff_ffff > 1 {
            let p: u32 = h.read(at(mgr, 0x4ec));
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p]);
        }
        h.write(at(mgr, 0x4ec), 0u32);
        h.write(at(mgr, 0x4e8), 0u32);
        for o in [0x474, 0x400, 0x38c] {
            h.call_sym(BUFFER_RELEASE, Conv::Thiscall(at(mgr, o)), &[]);
        }
        h.call_sym(SAMPLER_DTOR, Conv::Thiscall(at(mgr, 0x364)), &[]);
        h.call_sym(CLUSTER_DTOR, Conv::Thiscall(at(mgr, 0x310)), &[]);
        h.call_sym(PLAYBACK_DTOR, Conv::Thiscall(at(mgr, 0x38)), &[]);
        let mut r = 0;
        for o in [0x18, 0xc] {
            let p: u32 = h.read(at(mgr, o));
            r = p;
            if p != 0 {
                r = h.call_sym(USER_FREE, Conv::Cdecl, &[p]);
            }
        }
        r
    }

    /// `g_FMVPlayerManager`, `g_DynGeoFillBuffer`, `g_RemapButtonIcon`.
    pub const FMV_PLAYER_MANAGER: u32 = 0x00cd_ed3c;
    pub const DYN_GEO_FILL_BUFFER: u32 = 0x00cd_edf0;
    pub const REMAP_BUTTON_ICON: u32 = 0x00ce_63c4;
    /// `Phyre_PhyrePrintf(level, fmt)` (cdecl) and the "singleton is not
    /// created yet" message.
    pub const PHYRE_PRINTF: u32 = 0x0043_52f0;
    pub const NO_SINGLETON: u32 = 0x00b4_9b98;
    /// `PhyFMVPlayerManager_PlayVideo(mgr; flag)` (thiscall, `ret 4`).
    pub const PLAY_VIDEO: u32 = 0x006d_7430;
    /// `Array_Reserve(array; n)` (thiscall, `ret 4`).
    pub const ARRAY_RESERVE: u32 = 0x0066_7e00;
    /// `IggyMenu_callIggyAS3External(a, b)` (cdecl).
    pub const CALL_IGGY_AS3: u32 = 0x00a2_8310;
    /// `JobScheduleWait_pppPart_fillBuffer()` (cdecl).
    pub const WAIT_FILL_BUFFER: u32 = 0x0071_e810;
    /// `PVideoPlayback::setMemoryAccessor(x)` (`ret 4`; called with
    /// `ecx` = `g_RemapButtonIcon`) and the two 0.0 constants it gets.
    pub const SET_MEMORY_ACCESSOR: u32 = 0x006e_5740;
    pub const ZERO_A: u32 = 0x00b4_b3a4;
    pub const ZERO_B: u32 = 0x00b4_b3a8;
    /// `DynGeoMemManager_createPool(mgr; n)`, `_selectBuffer(mgr; i)`,
    /// `_releasePool(mgr; flag)` (thiscall, `ret 4`).
    pub const DYN_GEO_CREATE_POOL: u32 = 0x006d_e720;
    pub const DYN_GEO_SELECT_BUFFER: u32 = 0x006d_ec90;
    pub const DYN_GEO_RELEASE_POOL: u32 = 0x006d_e860;
    /// `FUN_006d7d90(queue; a, b)` (thiscall, `ret 8`).
    pub const QUEUE_PUSH: u32 = 0x006d_7d90;

    /// `PhyFMVPlayerManager_processRequests(q)` (`0x006d8cb0`, thiscall).
    ///
    /// The four counters `+0x1c..+0x28` set to 5; each request (`+4`
    /// count, kinds `+0xc`, flags `+0x18`) handled by kind: 1 plays the
    /// video (unless `+0x2c` >= 2 and the fill buffer's `+4` is set),
    /// 3 stops it once the counter `+0x24` >= 4, the manager's four
    /// words at `+0x700` are all set and its byte `+0x6d2` is clear
    /// (Iggy told at level >= 1; at >= 2 the fill buffer waited for, the
    /// accessor reset, the pool created and selected and request 4
    /// queued), 2 and 4 release the pool once `+0x28` >= 4. A handled
    /// request becomes kind 0; when every request is of another kind,
    /// both arrays (`+4`, `+0x10`) are emptied.
    pub fn process_requests(h: &mut impl Host, q: u32) {
        for o in [0x1c, 0x24, 0x20, 0x28] {
            h.write(at(q, o), 5u32);
        }
        let mut mgr: u32 = h.read(h.rebase(FMV_PLAYER_MANAGER));
        if mgr == 0 {
            let f = h.rebase(NO_SINGLETON);
            h.call_sym(PHYRE_PRINTF, Conv::Cdecl, &[1, f]);
            mgr = h.read(h.rebase(FMV_PLAYER_MANAGER));
        }
        let mut other = 0u32;
        let mut i = 0u32;
        if h.read::<u32>(at(q, 4)) == 0 {
            return;
        }
        loop {
            let kinds: u32 = h.read(at(q, 0xc));
            let kind: u32 = h.read(at(kinds, 4 * i));
            let level: i32 = h.read(at(q, 0x2c));
            let fill: u32 = h.read(h.rebase(DYN_GEO_FILL_BUFFER));
            match kind {
                1 => {
                    if level < 2 || h.read::<u32>(at(fill, 4)) == 0 {
                        let flags: u32 = h.read(at(q, 0x18));
                        let flag = u32::from(h.read::<u32>(at(flags, 4 * i)) != 0);
                        h.call_sym(PLAY_VIDEO, Conv::Thiscall(mgr), &[flag]);
                        clear(h, q, i);
                    }
                }
                3 => {
                    let mut any_zero = false;
                    for k in 0..4 {
                        any_zero = any_zero || h.read::<u32>(at(mgr, 0x700 + 4 * k)) == 0;
                    }
                    if h.read::<i32>(at(q, 0x24)) >= 4
                        && !any_zero
                        && h.read::<u8>(at(mgr, 0x6d2)) == 0
                    {
                        if h.read::<i32>(at(q, 0x2c)) >= 1 {
                            h.call_sym(CALL_IGGY_AS3, Conv::Cdecl, &[0, 0]);
                        }
                        if h.read::<i32>(at(q, 0x2c)) >= 2 {
                            h.call_sym(WAIT_FILL_BUFFER, Conv::Cdecl, &[]);
                            set_memory_accessor(h, ZERO_B);
                            h.write(at(q, 0x28), 0u32);
                            let fill: u32 = h.read(h.rebase(DYN_GEO_FILL_BUFFER));
                            h.call_sym(DYN_GEO_CREATE_POOL, Conv::Thiscall(fill), &[0]);
                            let fill: u32 = h.read(h.rebase(DYN_GEO_FILL_BUFFER));
                            h.call_sym(DYN_GEO_SELECT_BUFFER, Conv::Thiscall(fill), &[0]);
                            h.call_sym(QUEUE_PUSH, Conv::Thiscall(q), &[4, 0]);
                        }
                        clear(h, q, i);
                    }
                }
                2 | 4 => {
                    if h.read::<i32>(at(q, 0x28)) >= 4 {
                        if level >= 2 {
                            h.call_sym(
                                DYN_GEO_RELEASE_POOL,
                                Conv::Thiscall(fill),
                                &[u32::from(kind == 4)],
                            );
                            if kind == 2 {
                                set_memory_accessor(h, ZERO_A);
                            }
                        }
                        clear(h, q, i);
                    }
                }
                _ => other += 1,
            }
            i += 1;
            if i >= h.read::<u32>(at(q, 4)) {
                break;
            }
        }
        if other == 0 || other != h.read::<u32>(at(q, 4)) {
            return;
        }
        for o in [4, 0x10] {
            if h.read::<i32>(at(q, o)) < 0 && h.read::<i32>(at(q, o + 4)) < 0 {
                h.call_sym(ARRAY_RESERVE, Conv::Thiscall(at(q, o)), &[0]);
            }
            h.write(at(q, o), 0u32);
        }
    }

    fn clear(h: &mut impl Host, q: u32, i: u32) {
        let kinds: u32 = h.read(at(q, 0xc));
        h.write(at(kinds, 4 * i), 0u32);
    }

    fn set_memory_accessor(h: &mut impl Host, zero: u32) {
        let x: u32 = h.read(h.rebase(zero));
        let icon: u32 = h.read(h.rebase(REMAP_BUTTON_ICON));
        h.call_sym(SET_MEMORY_ACCESSOR, Conv::Thiscall(icon), &[x]);
    }

    /// `InterlockedCompareExchange`'s import slot (stdcall, 3 words).
    pub const INTERLOCKED_COMPARE_EXCHANGE: u32 = 0x00b0_c12c;
    /// `PVideoPlaybackWin32::updateTexture(video; a, b, c)` (thiscall,
    /// `ret 0xc`) -> `al`; `::getCurrentTime(video)` (thiscall) ->
    /// `edx:eax`.
    pub const UPDATE_TEXTURE: u32 = 0x00a2_84a0;
    pub const GET_CURRENT_TIME: u32 = 0x00a2_7bd0;
    /// The frame rate 29.97 and 0.001 (`f64`).
    pub const FPS: u32 = 0x00b4_a178;
    pub const MILLI: u32 = 0x00b4_7c68;

    /// `PhyFMVPlayerManager_updateVideoCallback(task)` (`0x006dae00`,
    /// cdecl). The task's job (`task + 4`): `+0` the video, `+4..+0xc`
    /// the texture arguments, `+0x10` the frame word, `+0x14` the last
    /// frame, `+0x18` the "updated" byte, `+0x1c` the "visible" byte,
    /// `+0x20` the lock word.
    ///
    /// Under the lock (taken with `InterlockedCompareExchange(lock, 1,
    /// 0)`, released with `(lock, 0, 1)`): a visible video's texture
    /// updated; when it was and the frame word is below 0xfffe, the old
    /// frame saved and the new one computed from the playback time (an
    /// unsigned 64-bit count, through `f32`, times 29.97 and 0.001,
    /// truncated); the updated byte set. The job is deleted. Returns
    /// `operator delete`'s `eax`.
    pub fn update_video_callback(h: &mut impl Host, task: u32) -> u32 {
        let job: u32 = h.read(at(task, 4));
        let ice: u32 = h.read(h.rebase(INTERLOCKED_COMPARE_EXCHANGE));
        if job != 0 {
            let lock: u32 = h.read(at(job, 0x20));
            h.call(ice, Conv::Stdcall, &[lock, 1, 0]);
            let visible: u32 = h.read(at(job, 0x1c));
            if h.read::<u8>(visible) != 0 {
                let video: u32 = h.read(job);
                let args = [
                    h.read::<u32>(at(job, 4)),
                    h.read::<u32>(at(job, 8)),
                    h.read::<u32>(at(job, 0xc)),
                ];
                if h.call_sym(UPDATE_TEXTURE, Conv::Thiscall(video), &args) & 0xff != 0 {
                    let frame_at: u32 = h.read(at(job, 0x10));
                    let frame: i32 = h.read(frame_at);
                    if frame < 0xfffe {
                        let last: u32 = h.read(at(job, 0x14));
                        h.write(last, frame);
                        let video: u32 = h.read(job);
                        let t = h.call_sym_u64(GET_CURRENT_TIME, Conv::Thiscall(video), &[]);
                        let frame = frame_of(h, t);
                        let frame_at: u32 = h.read(at(job, 0x10));
                        h.write(frame_at, frame);
                    }
                    let updated: u32 = h.read(at(job, 0x18));
                    h.write(updated, 1u8);
                }
            }
        }
        let lock: u32 = h.read(at(job, 0x20));
        h.call(ice, Conv::Stdcall, &[lock, 0, 1]);
        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[job])
    }

    /// The frame for playback time `t`: `(f32)t * 29.97 * 0.001`, toward
    /// zero (`ftol2_sse`).
    fn frame_of(h: &impl Host, t: u64) -> u32 {
        // fild of the low 63 bits plus 2^63 for the top bit: the unsigned
        // value, rounded once to 53 bits by the fadd
        #[allow(clippy::cast_precision_loss)] // the x87's rounding
        let wide = t as f64;
        #[allow(clippy::cast_possible_truncation)] // fstp dword
        let seconds = wide as f32;
        let fps = f64::from_bits(h.read::<u64>(h.rebase(FPS)));
        let milli = f64::from_bits(h.read::<u64>(h.rebase(MILLI)));
        x87::ftol(h, f64::from(seconds) * fps * milli)
    }

    /// `dbgPrintf(fmt, ...)` (cdecl, varargs).
    pub const DBG_PRINTF: u32 = 0x0062_f500;
    /// `sceOpen(name, mode)`, `sceClose(fd)`, `sceRead(fd, buf, n)`,
    /// `sceWrite(fd, buf, n)`, `sceLseek(fd, off, whence)`,
    /// `sceLseek64(fd, off, whence)`, `sceIoctl(fd, req, arg)`,
    /// `FlushCache(n)` (cdecl).
    pub const SCE_OPEN: u32 = 0x0062_fa30;
    pub const SCE_CLOSE: u32 = 0x0062_f610;
    pub const SCE_READ: u32 = 0x0062_fc00;
    pub const SCE_WRITE: u32 = 0x0062_fdc0;
    pub const SCE_LSEEK: u32 = 0x0062_f8e0;
    pub const SCE_LSEEK64: u32 = 0x0062_f8f0;
    pub const SCE_IOCTL: u32 = 0x0062_f8a0;
    pub const FLUSH_CACHE: u32 = 0x0062_f4c0;
    /// `memset`'s and `memcpy`'s thunks (cdecl) and `strncpy`'s import
    /// slot (cdecl).
    pub const MEMSET: u32 = 0x0094_964c;
    pub const MEMCPY: u32 = 0x0094_9256;
    pub const STRNCPY: u32 = 0x00b0_c368;

    /// `dbgPrintf(fmt, args...)` with up to three arguments.
    fn printf(h: &mut impl Host, fmt: u32, args: &[u32]) -> u32 {
        let mut w = [h.rebase(fmt), 0, 0, 0];
        let n = args.len().min(3);
        w[1..=n].copy_from_slice(&args[..n]);
        h.call_sym(DBG_PRINTF, Conv::Cdecl, &w[..=n])
    }

    fn negative(x: u32) -> bool {
        x & 0x8000_0000 != 0
    }

    /// The resource file's pieces: the dash line, the title, `"%s"`, the
    /// `PS2ICON3D` magic, the error and progress formats; the section
    /// buffer pointer, the sizes and sources of the three sections, the
    /// icon title and its 31-byte name.
    pub const DASHES: u32 = 0x00b5_345c;
    pub const RESOURCE_TITLE: u32 = 0x00b5_347c;
    pub const PERCENT_S: u32 = 0x00b0_e160;
    pub const ICON_MAGIC: u32 = 0x00b5_3c98;
    pub const WRITE_ERR1: u32 = 0x00b5_3ca4;
    pub const WRITE_ERR2: u32 = 0x00b5_3cc8;
    pub const WRITE_ERR3: u32 = 0x00b5_3cec;
    pub const WRITE_DONE: u32 = 0x00b5_3d10;
    pub const RESOURCE_BUFFER: u32 = 0x0112_a50c;
    pub const SYSTEM_CNF: u32 = 0x0112_a064;
    pub const SYSTEM_CNF_SIZE: u32 = 0x0112_a068;
    pub const ICON_DATA: u32 = 0x0112_a050;
    pub const ICON_SIZE: u32 = 0x0231_0c20;
    pub const ICON_TITLE: u32 = 0x00c4_1790;
    pub const ICON_NAME: u32 = 0x00c4_1770;

    /// `(x + 0x1ff) / 0x200` (signed, toward zero, as the original's
    /// `cdq`/`sar`).
    fn sector_count(x: u32) -> i32 {
        x.cast_signed().wrapping_add(0x1ff) / 0x200
    }

    /// `x` rounded up to whole sectors.
    fn sectors(x: u32) -> u32 {
        (sector_count(x) << 9).cast_unsigned()
    }

    /// `FUN_00779d20(fd)` (cdecl): writes the HDD resource file. `head`
    /// is the original's 512-byte header (`ebp - 0x204`): `PS2ICON3D`
    /// (`strncpy`, 9 bytes), bytes 9..16 zero, then three `(offset,
    /// size)` pairs from `+0x10` (the `SYSTEM.CNF` text at 0x200, the
    /// icon title, the icon data; each offset past the sectors before),
    /// the icon pair again at `+0x28`, the rest zeroed (`memset`). The
    /// header is written, then each section: the buffer at `0x0112a50c`
    /// cleared to whole sectors, filled (the title with the icon's
    /// 31-byte name at `+0xe`), sought to and written. Returns 0, or the
    /// failing call's negative result.
    pub fn resource_write(h: &mut impl Host, fd: u32, head: u32) -> u32 {
        let buffer: u32 = h.read(h.rebase(RESOURCE_BUFFER));
        printf(h, DASHES, &[]);
        printf(h, RESOURCE_TITLE, &[]);
        let cnf: u32 = h.read(h.rebase(SYSTEM_CNF));
        printf(h, PERCENT_S, &[cnf]);
        printf(h, DASHES, &[]);
        let strncpy: u32 = h.read(h.rebase(STRNCPY));
        let magic = h.rebase(ICON_MAGIC);
        h.call(strncpy, Conv::Cdecl, &[head, magic, 9]);
        h.write(at(head, 9), 0u16);
        h.write(at(head, 0xb), 0u8);
        h.write(at(head, 0xc), 0u32);
        let size0: u32 = h.read(h.rebase(SYSTEM_CNF_SIZE));
        h.write(at(head, 0x14), size0);
        let off1 = (sector_count(size0).wrapping_add(1) << 9).cast_unsigned();
        h.write(at(head, 0x10), 0x200u32);
        h.write(at(head, 0x18), off1);
        let title = h.rebase(ICON_TITLE);
        let mut len = 0u32;
        while h.read::<u8>(at(title, len)) != 0 {
            len += 1;
        }
        let off2 = sectors(len).wrapping_add(off1);
        h.write(at(head, 0x20), off2);
        h.write(at(head, 0x28), off2);
        h.write(at(head, 0x1c), len);
        let icon_size: u32 = h.read(h.rebase(ICON_SIZE));
        h.write(at(head, 0x24), icon_size);
        h.write(at(head, 0x2c), icon_size);
        h.call_sym(MEMSET, Conv::Cdecl, &[at(head, 0x30), 0, 0x1d0]);
        let r = h.call_sym(SCE_WRITE, Conv::Cdecl, &[fd, head, 0x200]);
        if negative(r) {
            printf(h, WRITE_ERR1, &[]);
            return r;
        }
        for k in 0..3u32 {
            let size: u32 = h.read(at(head, 0x14 + 8 * k));
            let n = sectors(size);
            h.call_sym(MEMSET, Conv::Cdecl, &[buffer, 0, n]);
            match k {
                0 => {
                    let src: u32 = h.read(h.rebase(SYSTEM_CNF));
                    h.call_sym(MEMCPY, Conv::Cdecl, &[buffer, src, size]);
                }
                1 => {
                    let src = h.rebase(ICON_TITLE);
                    h.call_sym(MEMCPY, Conv::Cdecl, &[buffer, src, size]);
                    let name = h.rebase(ICON_NAME);
                    for i in 0..31 {
                        let b: u8 = h.read(at(name, i));
                        h.write(at(buffer, 0xe + i), b);
                    }
                }
                _ => {
                    let src: u32 = h.read(h.rebase(ICON_DATA));
                    h.call_sym(MEMCPY, Conv::Cdecl, &[buffer, src, size]);
                }
            }
            let off: u32 = h.read(at(head, 0x10 + 8 * k));
            let r = h.call_sym(SCE_LSEEK, Conv::Cdecl, &[fd, off, 0]);
            if negative(r) {
                printf(h, WRITE_ERR2, &[]);
                return r;
            }
            let r = h.call_sym(SCE_WRITE, Conv::Cdecl, &[fd, buffer, n]);
            if negative(r) {
                printf(h, WRITE_ERR3, &[]);
                return r;
            }
            printf(h, WRITE_DONE, &[k]);
        }
        0
    }

    /// `movieOpenCd`'s format, the HDD read state it clears and sets,
    /// `hdd_state_clear()` (cdecl), `movie_hdd_file_open`.
    pub const OPEN_CD_FMT: u32 = 0x00b5_3554;
    pub const HDD_STATE: u32 = 0x0112_a4b4;
    pub const HDD_STATE_CLEAR: u32 = 0x0077_7420;
    pub const HDD_FILE_OPEN: u32 = 0x0077_9be0;

    fn sign_extend(b: u8) -> u32 {
        u32::from(b) | if b & 0x80 != 0 { 0xffff_ff00 } else { 0 }
    }

    /// `movieOpenCd(viewer)` (`0x00777240`, cdecl): prints the HDD mode
    /// (`+0x5075d`, signed), clears the read state (`0x0112a4d8..+0x14`
    /// words; the byte `0x0112a4ec` set), `hdd_state_clear`, copies the
    /// file position `+0x506e4/+0x506e8` to `0x0112a4b4/+4`; in HDD mode
    /// returns `movie_hdd_file_open(+0x50764 (signed), position)`, else
    /// 1.
    pub fn open_cd(h: &mut impl Host, viewer: u32) -> u32 {
        let mode = sign_extend(h.read(at(viewer, 0x5_075d)));
        printf(h, OPEN_CD_FMT, &[mode]);
        let st = h.rebase(HDD_STATE);
        for o in [0x24, 0x28, 0x2c, 0x30, 0x34] {
            h.write(at(st, o), 0u32);
        }
        h.write(at(st, 0x38), 1u8);
        h.call_sym(HDD_STATE_CLEAR, Conv::Cdecl, &[]);
        let pos: u32 = h.read(at(viewer, 0x5_06e4));
        let pos_hi: u32 = h.read(at(viewer, 0x5_06e8));
        h.write(st, pos);
        h.write(at(st, 4), pos_hi);
        if h.read::<u8>(at(viewer, 0x5_075d)) == 0 {
            return 1;
        }
        let file = sign_extend(h.read(at(viewer, 0x5_0764)));
        h.call_sym(HDD_FILE_OPEN, Conv::Cdecl, &[file, pos])
    }

    /// `g_Hdd_Heads` (12-byte entries: the first sector, "present"),
    /// `get_hdd_name(n)`, `task_wait(n)` (cdecl), the open error formats.
    pub const HDD_HEADS: u32 = 0x0112_a528;
    pub const GET_HDD_NAME: u32 = 0x0077_7eb0;
    pub const TASK_WAIT: u32 = 0x0077_a4e0;
    pub const OPEN_ERR: u32 = 0x00b5_3730;
    pub const OPEN_ERR1: u32 = 0x00b5_3748;

    /// `movie_hdd_file_open(n, sector)` (`0x00779be0`, cdecl). `slot` is
    /// the original's own `n` slot (`ebp + 8`), which `sceIoctl(fd, 1,
    /// slot)` overwrites with the busy count.
    ///
    /// -1 when file `n` is absent; else the HDD file opened (`0x8001`),
    /// sought to `(sector - first) << 11` and polled until idle
    /// (`task_wait(1)` between polls). Returns the descriptor, or -1
    /// after printing which step failed.
    pub fn hdd_file_open(h: &mut impl Host, n: u32, sector: u32, slot: u32) -> u32 {
        let heads = h.rebase(HDD_HEADS);
        let e = at(heads, n.wrapping_mul(12));
        if h.read::<u32>(at(e, 4)) == 0 {
            return u32::MAX;
        }
        let name = h.call_sym(GET_HDD_NAME, Conv::Cdecl, &[n]);
        let off = sector.wrapping_sub(h.read::<u32>(e)) << 11;
        let fd = h.call_sym(SCE_OPEN, Conv::Cdecl, &[name, 0x8001]);
        if negative(fd) {
            printf(h, OPEN_ERR, &[name]);
            return u32::MAX;
        }
        if negative(h.call_sym(SCE_LSEEK64, Conv::Cdecl, &[fd, off, 0])) {
            printf(h, OPEN_ERR1, &[name]);
            return u32::MAX;
        }
        h.write(slot, n);
        h.call_sym(SCE_IOCTL, Conv::Cdecl, &[fd, 1, slot]);
        while h.read::<u32>(slot) != 0 {
            h.call_sym(TASK_WAIT, Conv::Cdecl, &[1]);
            h.call_sym(SCE_IOCTL, Conv::Cdecl, &[fd, 1, slot]);
        }
        fd
    }

    /// `readSizeFile`'s formats and `movie_wait()` (cdecl).
    pub const CANNOT_OPEN: u32 = 0x00b5_43d0;
    pub const LSEEK_FAILS: u32 = 0x00b6_8e74;
    pub const READ_WAIT: u32 = 0x00b6_8e8c;
    pub const CANNOT_READ: u32 = 0x00b6_8eac;
    pub const READ_END: u32 = 0x00b6_8ebc;
    pub const MOVIE_WAIT: u32 = 0x0077_daa0;

    /// `readSizeFile(name, buf)` (`0x0093da60`, cdecl): the whole file
    /// read into `buf` (its size from `sceLseek(fd, 0, 2)`, then back to
    /// 0, `FlushCache(0)`, one `sceRead`). Returns 1, or 0 after printing
    /// why (and closing it). A file that doesn't open is printed and
    /// waits forever (`movie_wait()` in a loop), as in the original.
    pub fn read_size_file(h: &mut impl Host, name: u32, buf: u32) -> u32 {
        let fd = h.call_sym(SCE_OPEN, Conv::Cdecl, &[name, 1]);
        if negative(fd) {
            printf(h, CANNOT_OPEN, &[name]);
            loop {
                h.call_sym(MOVIE_WAIT, Conv::Cdecl, &[]);
            }
        }
        let size = h.call_sym(SCE_LSEEK, Conv::Cdecl, &[fd, 0, 2]);
        if negative(size) || negative(h.call_sym(SCE_LSEEK, Conv::Cdecl, &[fd, 0, 0])) {
            return read_fail(h, LSEEK_FAILS, name, fd);
        }
        printf(h, READ_WAIT, &[name, size]);
        h.call_sym(FLUSH_CACHE, Conv::Cdecl, &[0]);
        if h.call_sym(SCE_READ, Conv::Cdecl, &[fd, buf, size]) != size {
            return read_fail(h, CANNOT_READ, name, fd);
        }
        h.call_sym(SCE_CLOSE, Conv::Cdecl, &[fd]);
        printf(h, READ_END, &[]);
        1
    }

    fn read_fail(h: &mut impl Host, fmt: u32, name: u32, fd: u32) -> u32 {
        printf(h, fmt, &[name]);
        h.call_sym(SCE_CLOSE, Conv::Cdecl, &[fd]);
        0
    }
}
