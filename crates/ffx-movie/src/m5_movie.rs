//! M5 stage 1, list `movie`: the FMV player manager's teardown helpers
//! and time queries, the PS2 movie layer's debug stubs, file opens and
//! small glue.
//!
//! The callees still in the game's code (the `PVideoPlaybackWin32`
//! clock, `std` containers, `dbgPrintf`, the PS2 file layer) are called
//! by address through `ffx_graphic::host::Host`.

pub use ffx_graphic::host::{Conv, Host};

/// The original functions, over game memory.
pub mod bridge {
    use ffx_bridge::{Global, x87};

    use super::{Conv, Host};

    /// `operator delete` (cdecl).
    pub const OPERATOR_DELETE: u32 = 0x0063_0390;
    /// `user_free_thunk` (cdecl).
    pub const USER_FREE: u32 = 0x0068_7010;
    /// `dbgPrintf(fmt, ...)` (cdecl, empty on PC).
    pub const DBG_PRINTF: u32 = 0x0062_f500;
    /// `PhyFMVPlayerManager_dtor` (`ecx`).
    pub const MANAGER_DTOR: u32 = 0x006d_6d00;
    /// `std_Tree_erase_range_006d8940` (thiscall `(tree; &out, first,
    /// last)` ret 0xc).
    pub const TREE_ERASE_RANGE: u32 = 0x006d_8940;
    /// `0x00654f00` (thiscall `(list; a, b)` ret 8): a new list head.
    pub const LIST_HEAD_NEW: u32 = 0x0065_4f00;
    /// `PVideoPlaybackWin32::getEndTime` / `getCurrentTime` (`ecx`) ->
    /// `edx:eax`.
    pub const VIDEO_END_TIME: u32 = 0x00a2_7c40;
    pub const VIDEO_CURRENT_TIME: u32 = 0x00a2_7bd0;
    /// `PhyFMVPlayerManager_processRequests` (`ecx`).
    pub const PROCESS_REQUESTS: u32 = 0x006d_8cb0;
    /// `TODebugGetPadPrintFlag()` (cdecl).
    pub const TO_DEBUG_PAD_PRINT_FLAG: u32 = 0x008a_bd20;
    /// `task_wait(n)` (cdecl).
    pub const TASK_WAIT: u32 = 0x0077_a4e0;
    /// `TkSyncV(n)` (cdecl).
    pub const TK_SYNC_V: u32 = 0x0088_7f00;
    /// `movie_csc()` (cdecl).
    pub const MOVIE_CSC: u32 = 0x0077_d5e0;
    /// `sceOpen(name, mode)` / `sceClose(fd)` (cdecl).
    pub const SCE_OPEN: u32 = 0x0062_fa30;
    pub const SCE_CLOSE: u32 = 0x0062_f610;
    /// `0x0077b060(pcm, b, work)`: the movie audio's channel mix (cdecl).
    pub const AUDIO_MIX_HALVES: u32 = 0x0077_b060;
    /// `Gu_WindowCreate(x, y, w, h, kind, draw)` (cdecl).
    pub const GU_WINDOW_CREATE: u32 = 0x0084_a920;
    /// `movie_view`'s window callback.
    pub const MOVIE_VIEW_DRAW: u32 = 0x0093_dc00;

    /// `g_FMVPlayerManager`.
    pub const FMV_PLAYER_MANAGER: Global<u32> = Global::new(0x00cd_ed3c);
    /// `g_FMVSkipped` (byte).
    pub const FMV_SKIPPED: Global<u8> = Global::new(0x00cd_ed31);
    /// The movie audio work area's pointer.
    pub const AUDIO_WORK: Global<u32> = Global::new(0x00c4_21b0);
    /// `movie_view`'s "window open" byte.
    pub const MOVIE_VIEW_OPEN: Global<u8> = Global::new(0x0193_c151);
    /// `29.97` (f64) and `0.001` (f64): frames per second, ms to s.
    pub const FPS: u32 = 0x00b4_a178;
    pub const MS_TO_S: u32 = 0x00b4_7c68;

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    fn this_call(h: &mut impl Host, sym: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Thiscall(this), args)
    }

    fn dbg(h: &mut impl Host, msg: u32, args: &[u32]) {
        let mut a = vec![h.rebase(msg)];
        a.extend_from_slice(args);
        cdecl(h, DBG_PRINTF, &a);
    }

    fn f64_at(h: &mut impl Host, sym: u32) -> f64 {
        let at = h.rebase(sym);
        f64::from_bits(h.read::<u64>(at))
    }

    /// `PhyFMVPlayerManager_DestroyInstance()` (0x006d7370): the
    /// manager torn down, deleted and cleared.
    pub fn destroy_instance(h: &mut impl Host) {
        let m = FMV_PLAYER_MANAGER.get(h);
        if m != 0 {
            this_call(h, MANAGER_DTOR, m, &[]);
            cdecl(h, OPERATOR_DELETE, &[m]);
            FMV_PLAYER_MANAGER.set(h, 0);
        }
    }

    /// A `std::list`'s destruction (`list` = the list object, its head
    /// node pointer at `+0` and the size at `+4`): the head relinked to
    /// itself, the size 0, every node deleted, then the head.
    fn list_destroy(h: &mut impl Host, list: u32) {
        let head: u32 = h.read(list);
        let mut node: u32 = h.read(head);
        h.write(head, head);
        let head: u32 = h.read(list);
        h.write(head + 4, head);
        h.write(list + 4, 0u32);
        while node != h.read::<u32>(list) {
            let next: u32 = h.read(node);
            cdecl(h, OPERATOR_DELETE, &[node]);
            node = next;
        }
        let head: u32 = h.read(list);
        cdecl(h, OPERATOR_DELETE, &[head]);
    }

    /// `0x006d6c30` (`ecx` = a `std::list`): its destructor.
    pub fn list_dtor(h: &mut impl Host, list: u32) {
        list_destroy(h, list);
    }

    /// `0x006d6c70` (`ecx` = a `std::map`): every node erased
    /// (`erase(begin, end)`, the returned iterator in the original's frame
    /// at `ebp - 4`, `frame` its `ebp`), then the head deleted.
    pub fn map_dtor(h: &mut impl Host, frame: u32, map: u32) {
        let head: u32 = h.read(map);
        let first: u32 = h.read(head);
        this_call(h, TREE_ERASE_RANGE, map, &[frame - 4, first, head]);
        let head: u32 = h.read(map);
        cdecl(h, OPERATOR_DELETE, &[head]);
    }

    /// `0x006d6cd0` (`ecx`): `user_free` of `+0x18` and `+0xc` when set;
    /// the `eax` the original leaves.
    pub fn free_buffers(h: &mut impl Host, this: u32) -> u32 {
        let a: u32 = h.read(this + 0x18);
        if a != 0 {
            cdecl(h, USER_FREE, &[a]);
        }
        let b: u32 = h.read(this + 0xc);
        if b == 0 { 0 } else { cdecl(h, USER_FREE, &[b]) }
    }

    /// `0x006daf30` (`ecx`): `+0x14` deleted and cleared, the list at
    /// `+0xc` destroyed.
    pub fn request_queue_dtor(h: &mut impl Host, this: u32) {
        let p: u32 = h.read(this + 0x14);
        if p != 0 {
            cdecl(h, OPERATOR_DELETE, &[p]);
            h.write(this + 0x14, 0u32);
        }
        list_destroy(h, this + 0xc);
    }

    /// `0x006daee0` (`ecx`) -> `this`: zeroed, with a new list head at
    /// `+0xc` (`0x00654f00(0, 0)`).
    pub fn request_queue_ctor(h: &mut impl Host, this: u32) -> u32 {
        for off in [0, 4, 8, 0xc, 0x10] {
            h.write(this + off, 0u32);
        }
        let head = this_call(h, LIST_HEAD_NEW, this + 0xc, &[0, 0]);
        h.write(this + 0xc, head);
        h.write(this + 0x14, 0u32);
        h.write(this + 0x18, 0u32);
        this
    }

    /// `PhyFMVPlayerManager_getEndTimeMs(this)` (0x006d91d0): the
    /// playback's end time (`+0x38`, a `u64`) rounded to f32, times
    /// 29.97 and 0.001, to an integer (`ftol`).
    pub fn get_end_time_ms(h: &mut impl Host, this: u32) -> u32 {
        let end = h.call_sym_u64(VIDEO_END_TIME, Conv::Thiscall(this + 0x38), &[]);
        #[allow(clippy::cast_precision_loss)] // the original's two roundings
        let end = x87::load(x87::store(end as f64));
        let fps = f64_at(h, FPS);
        let ms = f64_at(h, MS_TO_S);
        let v = x87::mul(x87::mul(end, fps), ms);
        x87::ftol(h, v)
    }

    /// `PhyFMVPlayerManager_isVideoAtEndFrame(this)` (0x006da240): false
    /// unless playing (`+0x6d2`); true once skipped (`g_FMVSkipped`) or
    /// without a video (`+0x6d0`); without the end flag (`+0x6e4`) false;
    /// else whether the current time reached the end time (`u64`).
    pub fn is_video_at_end_frame(h: &mut impl Host, this: u32) -> bool {
        if h.read::<u8>(this + 0x6d2) == 0 {
            return false;
        }
        if FMV_SKIPPED.get(h) != 0 || h.read::<u8>(this + 0x6d0) == 0 {
            return true;
        }
        if h.read::<u8>(this + 0x6e4) == 0 {
            return false;
        }
        let now = h.call_sym_u64(VIDEO_CURRENT_TIME, Conv::Thiscall(this + 0x38), &[]);
        let end = h.call_sym_u64(VIDEO_END_TIME, Conv::Thiscall(this + 0x38), &[]);
        now >= end
    }

    /// The PS2 movie layer's empty debug hooks (`code_0076ed80`,
    /// `movie_frame_prog`, ...): `dbgPrintf(msg)`.
    pub fn debug_stub(h: &mut impl Host, msg: u32) {
        dbg(h, msg, &[]);
    }

    /// `movie_wait()` (0x0077daa0): `task_wait(1)`'s `eax`.
    pub fn movie_wait(h: &mut impl Host) -> u32 {
        cdecl(h, TASK_WAIT, &[1])
    }

    /// A 5-byte `jmp` to a cdecl `target`: its `eax`.
    pub fn jmp_thunk(h: &mut impl Host, target: u32, args: &[u32]) -> u32 {
        cdecl(h, target, args)
    }

    /// `code_006dadf0(this)`: `jmp PhyFMVPlayerManager_processRequests`.
    pub fn process_requests_thunk(h: &mut impl Host, this: u32) -> u32 {
        this_call(h, PROCESS_REQUESTS, this, &[])
    }

    /// `movieOpen(name)` (0x0093da30) -> `sceOpen(name, 0x8001)`.
    pub fn movie_open(h: &mut impl Host, name: u32) -> u32 {
        let fd = cdecl(h, SCE_OPEN, &[name, 0x8001]);
        dbg(h, 0x00b6_8ec8, &[name, fd]);
        fd
    }

    /// `movieClose(fd)` (0x0093da00) -> 1.
    pub fn movie_close(h: &mut impl Host, fd: u32) -> u32 {
        dbg(h, 0x00b6_8ee4, &[fd]);
        cdecl(h, SCE_CLOSE, &[fd]);
        1
    }

    /// `0x0077dc50(convert)`: 0: `TkSyncV(0)`; else `movie_csc()` then
    /// `task_wait(1)`; both tail jumps through its own argument word
    /// (`slots`). Returns the tail call's `eax`.
    pub fn movie_frame_wait(h: &mut impl Host, convert: u32, slots: u32) -> u32 {
        if convert == 0 {
            h.write(slots, 0u32);
            return cdecl(h, TK_SYNC_V, &[0]);
        }
        cdecl(h, MOVIE_CSC, &[]);
        h.write(slots, 1u32);
        cdecl(h, TASK_WAIT, &[1])
    }

    /// `code_0077de40()`: the audio work's decoded block mixed
    /// (`0x0077b060`) by its channel layout (`+0x506c8`: 0 one channel,
    /// 1 two, the second after `+0x50758` bytes).
    pub fn movie_audio_mix(h: &mut impl Host) {
        let w = AUDIO_WORK.get(h);
        let block: u32 = h.read(w + 0x5059c);
        match h.read::<u32>(w + 0x506c8) {
            0 => {
                cdecl(h, AUDIO_MIX_HALVES, &[block + 0x10, 0, w]);
            }
            1 => {
                let size: u32 = h.read(w + 0x50758);
                cdecl(h, AUDIO_MIX_HALVES, &[block + 0x10, size + 0x10 + block, w]);
            }
            _ => {}
        }
    }

    /// `movie_view()` (0x00941830): the movie debug window (`Gu_WindowCreate`)
    /// unless open.
    pub fn movie_view(h: &mut impl Host) {
        if MOVIE_VIEW_OPEN.get(h) == 0 {
            let draw = h.rebase(MOVIE_VIEW_DRAW);
            cdecl(h, GU_WINDOW_CREATE, &[0x64, 0x1e, 0xdc, 0xbe, 0xe, draw]);
        }
    }
}
