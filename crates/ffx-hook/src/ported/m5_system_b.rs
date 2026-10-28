//! M5 stage 1, list `system-b`: the crash reporter's `StackWalker`
//! (`ffx_system::m5b_stackwalker`), `AsyncLoadManager` and its request
//! pools (`ffx_system::m5b_async`), and small PC glue
//! (`ffx_system::m5b_misc`).
//!
//! Threads: `AsyncLoadManager`'s thread runs `threadMain` and the
//! requests (32 KB of stack: the shims' frames stay small); the pools
//! take their mutexes as the originals do. Frames are per call.

use super::{StackFrame, mem, ported};
use ffx_system::m5b_async::{self as al, bridge as ab};
use ffx_system::m5b_misc::bridge as mb;
use ffx_system::m5b_stackwalker::bridge as sw;

ported! {
    // ---- StackWalker ----
    /// `StackWalker::StackWalker(this; options, symPath, processId, process)`.
    StackWalker_ctor_006870b0 @ 0x0068_70b0 => extern "thiscall" fn stack_walker_ctor(this: u32, options: u32, sym_path: u32, process_id: u32, process: u32) -> u32 {
        sw::construct(&mut mem(), this, options, sym_path, process_id, process)
    }
    /// `StackWalker::~StackWalker(this)`.
    StackWalker_dtor_00687210 @ 0x0068_7210 => extern "thiscall" fn stack_walker_dtor(this: u32) {
        sw::destruct(&mut mem(), this);
    }
    /// The deleting destructor `(this; flags)`.
    StackWalker_deletingDtor_006872e0 @ 0x0068_72e0 => extern "thiscall" fn stack_walker_delete(this: u32, flags: u32) -> u32 {
        sw::deleting_dtor(&mut mem(), this, flags)
    }
    /// `OnCallstackEntry(this; eType, entry)`: frame `ebp - 0x40c`.
    StackWalker_OnCallstackEntry_006882a0 @ 0x0068_82a0 => extern "thiscall" fn on_callstack_entry(this: u32, kind: u32, entry: u32) {
        let mut f = StackFrame::<0x103>::new();
        sw::on_callstack_entry(&mut mem(), f.at() + 0x40c, this, kind, entry);
    }
    /// `OnDbgHelpErr(this; funcName, gle, addr)`: frame `ebp - 0x40c`.
    StackWalker_OnDbgHelpErr_00688470 @ 0x0068_8470 => extern "thiscall" fn on_dbghelp_err(this: u32, func: u32, gle: u32, lo: u32, hi: u32) {
        let mut f = StackFrame::<0x103>::new();
        sw::on_dbghelp_err(&mut mem(), f.at() + 0x40c, this, func, gle, lo, hi);
    }
    /// `OnLoadModule(this; img, mod, base, size, result, symType, pdb,
    /// fileVersion)`: frame `ebp - 0x414`.
    StackWalker_OnLoadModule_006884e0 @ 0x0068_84e0 => extern "thiscall" fn on_load_module(this: u32, image: u32, module: u32, base_lo: u32, base_hi: u32, size: u32, result: u32, sym_type: u32, pdb: u32, version_lo: u32, version_hi: u32) {
        let mut f = StackFrame::<0x105>::new();
        let m = sw::Module { image, module, base_lo, base_hi, size, result, sym_type, pdb, version_lo, version_hi };
        sw::on_load_module(&mut mem(), f.at() + 0x414, this, &m);
    }
    /// `OnSymInit(this; searchPath, symOptions, userName)`: frame `ebp -
    /// 0x4a0`.
    StackWalker_OnSymInit_006885f0 @ 0x0068_85f0 => extern "thiscall" fn on_sym_init(this: u32, search_path: u32, options: u32, user: u32) {
        let mut f = StackFrame::<0x128>::new();
        sw::on_sym_init(&mut mem(), f.at() + 0x4a0, this, search_path, options, user);
    }

    // ---- FFEscMenu ----
    /// `FFEscMenu::~FFEscMenu(this)`.
    FFEscMenu_dtor_0068e1f0 @ 0x0068_e1f0 => extern "thiscall" fn esc_menu_dtor(this: u32) -> u32 {
        mb::esc_menu_destruct(&mut mem(), this)
    }

    // ---- AsyncLoadManager ----
    /// `AssetListReader_getPlatformSpecificPath(out, std::string path)`:
    /// the string's six words are the caller's argument slots.
    AssetListReader_getPlatformSpecificPath @ 0x0069_0590 => extern "C" fn platform_specific_path(out: u32, _p0: u32, _p1: u32, _p2: u32, _p3: u32, _size: u32, _capacity: u32; slots: u32) -> u32 {
        ab::platform_specific_path(&mut mem(), out, slots.wrapping_add(4))
    }
    /// The children list's `reserve(this; capacity)`.
    AsyncList_reserve_00691770 @ 0x0069_1770 => extern "thiscall" fn list_reserve_00691770(this: u32, capacity: u32) {
        ab::list_reserve(&mut mem(), this, capacity);
    }
    /// A request list's `reserve(this; capacity)`.
    AsyncList_reserve_006917f0 @ 0x0069_17f0 => extern "thiscall" fn list_reserve_006917f0(this: u32, capacity: u32) {
        ab::list_reserve(&mut mem(), this, capacity);
    }
    /// A list's constructor `(this; capacity)`.
    AsyncList_ctor_00690680 @ 0x0069_0680 => extern "thiscall" fn list_ctor(this: u32, capacity: u32) -> u32 {
        ab::list_construct(&mut mem(), this, capacity)
    }
    /// `RequestPool<LoadClusterRequest>::RequestPool(this; capacity)`.
    AsyncLoadManager_ClusterPool_ctor_006906d0 @ 0x0069_06d0 => extern "thiscall" fn cluster_pool_ctor(this: u32, capacity: u32) -> u32 {
        ab::cluster_pool_construct(&mut mem(), this, capacity)
    }
    /// `AsyncLoadManager::createThread(this)`: frame `ebp - 0x30`.
    AsyncLoadManager_createThread @ 0x0069_1180 => extern "thiscall" fn create_thread(this: u32) {
        let mut f = StackFrame::<{ (al::THREAD_PARAM / 4) as usize }>::new();
        ab::create_thread(&mut mem(), f.at() + al::THREAD_PARAM, this);
    }
    /// `AsyncLoadManager::AsyncLoadManager(this)`: `createThread`'s frame
    /// `ebp - 0x30`.
    AsyncLoadManager_ctor_00690830 @ 0x0069_0830 => extern "thiscall" fn manager_ctor(this: u32) -> u32 {
        let mut f = StackFrame::<{ (al::THREAD_PARAM / 4) as usize }>::new();
        ab::construct(&mut mem(), f.at() + al::THREAD_PARAM, this)
    }
    /// `AsyncLoadManager::~AsyncLoadManager(this)`.
    AsyncLoadManager_dtor_00690ac0 @ 0x0069_0ac0 => extern "thiscall" fn manager_dtor(this: u32) -> u32 {
        ab::destruct(&mut mem(), this)
    }
    /// The cluster pool's deleting destructor `(this; flags)`.
    AsyncLoadManager_ClusterPool_deletingDtor_00690c40 @ 0x0069_0c40 => extern "thiscall" fn cluster_pool_delete(this: u32, flags: u32) -> u32 {
        ab::pool_deleting_dtor(&mut mem(), this, flags, al::CLUSTER_POOL_VFTABLE)
    }
    /// The task pool's deleting destructor `(this; flags)`.
    AsyncLoadManager_TaskPool_deletingDtor_00690c80 @ 0x0069_0c80 => extern "thiscall" fn task_pool_delete(this: u32, flags: u32) -> u32 {
        ab::pool_deleting_dtor(&mut mem(), this, flags, al::TASK_POOL_VFTABLE)
    }
    /// The manager's deleting destructor `(this; flags)`.
    AsyncLoadManager_deletingDtor_00690cc0 @ 0x0069_0cc0 => extern "thiscall" fn manager_delete(this: u32, flags: u32) -> u32 {
        ab::deleting_dtor(&mut mem(), this, flags)
    }
    /// `LoadClusterRequest`'s deleting destructor `(this; flags)`.
    AsyncLoadManager_LoadClusterRequest_deletingDtor_00690cf0 @ 0x0069_0cf0 => extern "thiscall" fn cluster_request_delete(this: u32, flags: u32) -> u32 {
        ab::cluster_request_deleting_dtor(&mut mem(), this, flags)
    }
    /// `RegistTaskRequest`'s deleting destructor `(this; flags)`.
    AsyncLoadManager_RegistTaskRequest_deletingDtor_00690d50 @ 0x0069_0d50 => extern "thiscall" fn task_request_delete(this: u32, flags: u32) -> u32 {
        ab::task_request_deleting_dtor(&mut mem(), this, flags)
    }
    /// `Request`'s deleting destructor `(this; flags)`.
    AsyncLoadManager_Request_deletingDtor_00690d90 @ 0x0069_0d90 => extern "thiscall" fn request_delete(this: u32, flags: u32) -> u32 {
        ab::request_deleting_dtor(&mut mem(), this, flags)
    }
    /// `queue(this; list, request)`.
    AsyncLoadManager_queue_00690e60 @ 0x0069_0e60 => extern "thiscall" fn queue(this: u32, list: u32, request: u32) -> u32 {
        ab::queue(&mut mem(), this, list, request)
    }
    /// `AsyncLoadManager::createChildrenRequestList(this; capacity)`.
    AsyncLoadManager_createChildrenRequestList @ 0x0069_0f70 => extern "thiscall" fn create_children_list(this: u32, capacity: u32) -> u32 {
        ab::create_children_list(&mut mem(), this, capacity)
    }
    /// The cluster pool's `alloc(this)`.
    AsyncLoadManager_ClusterPool_alloc_00690ff0 @ 0x0069_0ff0 => extern "thiscall" fn cluster_pool_alloc(this: u32) -> u32 {
        ab::cluster_pool_alloc(&mut mem(), this)
    }
    /// The task pool's `alloc(this)`.
    AsyncLoadManager_TaskPool_alloc_006910c0 @ 0x0069_10c0 => extern "thiscall" fn task_pool_alloc(this: u32) -> u32 {
        ab::task_pool_alloc(&mut mem(), this)
    }
    /// The cluster pool's `free(this; request)`.
    AsyncLoadManager_ClusterPool_free_006911f0 @ 0x0069_11f0 => extern "thiscall" fn cluster_pool_free(this: u32, request: u32) -> u32 {
        ab::cluster_pool_free(&mut mem(), this, request)
    }
    /// `LoadClusterRequest`'s release (`ecx` = the request).
    AsyncLoadManager_LoadClusterRequest_release_006912c0 @ 0x0069_12c0 => extern "thiscall" fn cluster_request_release(request: u32) -> u32 {
        ab::free_cluster_request(&mut mem(), request)
    }
    /// `RegistTaskRequest`'s release (`ecx` = the request).
    AsyncLoadManager_RegistTaskRequest_release_006912d0 @ 0x0069_12d0 => extern "thiscall" fn task_request_release(request: u32) {
        let _ = ab::free_task_request(&mut mem(), request);
    }
    /// `LoadClusterRequest::init(this; name, word, args)`.
    AsyncLoadManager_LoadClusterRequest_init_00691550 @ 0x0069_1550 => extern "thiscall" fn cluster_request_init(this: u32, name: u32, word: u32, args: u32) {
        ab::cluster_request_init(&mut mem(), this, name, word, args);
    }
    /// `AsyncLoadManager_loadCluster(manager, list, name, word, count,
    /// ...)`: the varargs are the caller's; the argument array in a frame.
    AsyncLoadManager_loadCluster @ 0x0069_1660 => extern "C" fn load_cluster(manager: u32, list: u32, name: u32, word: u32, count: u32; slots: u32) -> u32 {
        let mut f = StackFrame::<{ al::ARGS_WORDS as usize }>::new();
        ab::load_cluster(&mut mem(), f.at(), manager, list, name, word, count, slots.wrapping_add(20))
    }
    /// `AsyncLoadManager_registAsyncLoadTask(manager, task, count, ...)`.
    AsyncLoadManager_registAsyncLoadTask @ 0x0069_18b0 => extern "C" fn regist_task(manager: u32, task: u32, count: u32; slots: u32) -> u32 {
        let mut f = StackFrame::<{ al::ARGS_WORDS as usize }>::new();
        ab::regist_task(&mut mem(), f.at(), manager, task, count, slots.wrapping_add(12))
    }
}

/// The name that turns [`loader_thread_main`] off (`FFX_HOOK_DISABLE`) or
/// on alone (`FFX_HOOK_ONLY`): the harness's loader-turn wrapper owns
/// `AsyncLoadManager_threadMain_00691360`'s address (`loader_turn.rs`) and
/// calls the port instead of its trampoline (one hook per address).
pub(crate) const THREAD_MAIN_NAME: &str = "AsyncLoadManager_threadMain_00691360_port";

/// Whether the loader thread runs [`loader_thread_main`].
pub(crate) fn loader_thread_main_on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| {
        let off = std::env::var("FFX_HOOK_DISABLE").unwrap_or_default();
        let only =
            crate::hook_only().is_none_or(|names| names.iter().any(|n| n == THREAD_MAIN_NAME));
        only && !off
            .split(',')
            .map(str::trim)
            .any(|n| n == "replacements" || n == THREAD_MAIN_NAME)
    })
}

/// `AsyncLoadManager_threadMain_00691360()` (cdecl, the loader thread's
/// entry) in Rust (`ffx_system::m5b_async::bridge::thread_main`).
pub(crate) fn loader_thread_main() -> u32 {
    ab::thread_main(&mut mem())
}
