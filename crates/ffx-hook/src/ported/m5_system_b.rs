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
use ffx_system::m5b_hdd::bridge as hd;
use ffx_system::m5b_menu::bridge as mn;
use ffx_system::m5b_misc::{self as ms, bridge as mb};
use ffx_system::m5b_movie::bridge as mv;
use ffx_system::m5b_pc::bridge as pc;
use ffx_system::m5b_stackwalker::bridge as sw;
use ffx_system::m5b_steam::{self as steam, bridge as sm};
use ffx_system::m5b_stl::{self as stl, bridge as st};
use ffx_system::m5b_sync::{self as sy, bridge as sb3};

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
    // ---- std::_Tree, std::string, std::vector ----
    /// `std::_Tree::_Insert_at(this; &out, addLeft, where, value, node)`.
    StdTree_insertAt_006993d0 @ 0x0069_93d0 => extern "thiscall" fn tree_insert_at(this: u32, out: u32, add_left: u32, at: u32, _value: u32, node: u32) -> u32 {
        st::tree_insert_at(&mut mem(), this, out, add_left, at, node)
    }
    /// `std::_Tree::erase(this; &out, where)`: advances its own `where`
    /// slot, as the original.
    ClusterMap_erase_0069b010 @ 0x0069_b010 => extern "thiscall" fn tree_erase(this: u32, out: u32, _at: u32; slots: u32) -> u32 {
        st::tree_erase(&mut mem(), this, out, slots.wrapping_add(4))
    }
    /// `(this; name)`: the hash set holds `DJB_hash(name)`.
    HashSet_containsName_0069b7c0 @ 0x0069_b7c0 => extern "thiscall" fn hash_set_contains(this: u32, name: u32) -> u32 {
        st::hash_set_contains(&mut mem(), this, name)
    }
    /// `(this; src, _)`: a string move plus the word after it.
    StdString_moveWithWord_0069c200 @ 0x0069_c200 => extern "thiscall" fn string_move_with_word(this: u32, src: u32, _unused: u32) -> u32 {
        st::string_move_with_word(&mut mem(), this, src)
    }
    /// `()`: queue 0 of the job scheduler (the code of
    /// `JobScheduleWait_Ch_Cloth_PerCharacter`).
    JobScheduleWait_pppPart_fillBuffer @ 0x0071_e810 => extern "C" fn wait_ppp_part() {
        ffx_system::m5a_platform::bridge::wait_cloth(&mut mem());
    }
    /// `LocalizationManager::readLocalizationKit(this)`: frame `ebp - 0x304`.
    LocalizationManager_readLocalizationKit @ 0x006d_b2f0 => extern "thiscall" fn read_localization_kit(this: u32) {
        let mut f = StackFrame::<{ (ms::LOC_KIT_FRAME / 4) as usize }>::new();
        mb::read_localization_kit(&mut mem(), f.at() + ms::LOC_KIT_FRAME, this);
    }
    /// `DepthBiasHooks_onSceneLoaded(_, a, b)`: the two 0x50-byte objects
    /// are the caller's argument words; the callee destroys them.
    DepthBiasHooks_onSceneLoaded @ 0x006e_4470 => extern "C" fn depth_bias_on_scene_loaded(_word: u32; slots: u32) {
        mb::depth_bias_on_scene_loaded(&mut mem(), slots);
    }

    // ---- Steam callbacks ----
    /// `CCallback<UserAchievementStored_t>`'s destructor.
    CCallback_UserAchievementStored_dtor @ 0x006f_1310 => extern "thiscall" fn cb_achievement_stored_dtor(this: u32) {
        sm::callback_dtor(&mut mem(), this, steam::ACHIEVEMENT_STORED_VFTABLE);
    }
    /// `CCallback<UserStatsReceived_t>`'s destructor.
    CCallback_UserStatsReceived_dtor @ 0x006f_1330 => extern "thiscall" fn cb_stats_received_dtor(this: u32) {
        sm::callback_dtor(&mut mem(), this, steam::STATS_RECEIVED_VFTABLE);
    }
    /// `CCallback<UserStatsStored_t>`'s destructor.
    CCallback_UserStatsStored_dtor @ 0x006f_1350 => extern "thiscall" fn cb_stats_stored_dtor(this: u32) {
        sm::callback_dtor(&mut mem(), this, steam::STATS_STORED_VFTABLE);
    }
    /// `StatsAndAchievements::OnUserStatsReceived(this; cb)`.
    StatsAndAchievements_OnUserStatsReceived @ 0x006f_15f0 => extern "thiscall" fn on_user_stats_received(this: u32, cb: u32) {
        sm::on_user_stats_received(&mut mem(), this, cb);
    }
    /// `CCallback<UserAchievementStored_t>::Run(this; param)`.
    CCallback_UserAchievementStored_run_006f1820 @ 0x006f_1820 => extern "thiscall" fn cb_run_006f1820(this: u32, param: u32) -> u32 {
        sm::callback_run(&mut mem(), this, param)
    }
    /// `CCallback<UserAchievementStored_t>::Run(this; param, ioFailure, call)`.
    CCallback_UserAchievementStored_runCall_006f1830 @ 0x006f_1830 => extern "thiscall" fn cb_run_call_006f1830(this: u32, param: u32, _io: u32, _call_lo: u32, _call_hi: u32) -> u32 {
        sm::callback_run_call(&mut mem(), this, param)
    }
    /// `CCallback<UserStatsReceived_t>::Run(this; param)`.
    CCallback_UserStatsReceived_run_006f1850 @ 0x006f_1850 => extern "thiscall" fn cb_run_006f1850(this: u32, param: u32) -> u32 {
        sm::callback_run(&mut mem(), this, param)
    }
    /// `CCallback<UserStatsReceived_t>::Run(this; param, ioFailure, call)`.
    CCallback_UserStatsReceived_runCall_006f1860 @ 0x006f_1860 => extern "thiscall" fn cb_run_call_006f1860(this: u32, param: u32, _io: u32, _call_lo: u32, _call_hi: u32) -> u32 {
        sm::callback_run_call(&mut mem(), this, param)
    }
    /// `CCallback<UserStatsStored_t>::Run(this; param)`.
    CCallback_UserStatsStored_run_006f1880 @ 0x006f_1880 => extern "thiscall" fn cb_run_006f1880(this: u32, param: u32) -> u32 {
        sm::callback_run(&mut mem(), this, param)
    }
    /// `CCallback<UserStatsStored_t>::Run(this; param, ioFailure, call)`.
    CCallback_UserStatsStored_runCall_006f1890 @ 0x006f_1890 => extern "thiscall" fn cb_run_call_006f1890(this: u32, param: u32, _io: u32, _call_lo: u32, _call_hi: u32) -> u32 {
        sm::callback_run_call(&mut mem(), this, param)
    }

    // ---- AssetListReader's symbol map ----
    /// `std::map<std::string, ...>::equal_range(this; &out, key)`.
    AssetListReader_equalRange_006f3c60 @ 0x006f_3c60 => extern "thiscall" fn string_map_equal_range(this: u32, out: u32, key: u32) -> u32 {
        st::string_map_equal_range(&mut mem(), this, out, key)
    }
    /// `std::vector<word>::_Reallocate(this; capacity)`.
    StdVector_reallocate_006f3ee0 @ 0x006f_3ee0 => extern "thiscall" fn vector_reallocate(this: u32, capacity: u32) -> u32 {
        st::vector_reallocate(&mut mem(), this, capacity)
    }
    /// `std::vector<word>::_Reserve(this; more)`.
    StdVector_reserve_006f3f60 @ 0x006f_3f60 => extern "thiscall" fn vector_reserve(this: u32, more: u32) -> u32 {
        st::vector_reserve(&mut mem(), this, more)
    }
    /// `AssetListReader::clear(this)`.
    AssetListReader_clear @ 0x006f_4030 => extern "thiscall" fn asset_list_clear(this: u32) -> u32 {
        st::asset_list_clear(&mut mem(), this)
    }
    /// `erase(this; key)`: the range (`ebp - 8`) and the erase's result
    /// (`ebp - 4`, over the range's second word) in a frame,
    /// the count in its own argument slot as the original.
    AssetListReader_eraseKey_006f40e0 @ 0x006f_40e0 => extern "thiscall" fn string_map_erase_key(this: u32, key: u32; slots: u32) -> u32 {
        let mut f = StackFrame::<2>::new();
        st::string_map_erase_key(&mut mem(), f.at(), f.at() + 4, slots, this, key)
    }
    /// `hasKey(this; name)`: frame `ebp - 0x2c`.
    AssetListReader_hasKey_006f4410 @ 0x006f_4410 => extern "thiscall" fn string_map_has_key(this: u32, name: u32) -> u8 {
        let mut f = StackFrame::<{ (stl::HAS_KEY_FRAME / 4) as usize }>::new();
        st::string_map_has_key(&mut mem(), f.at() + stl::HAS_KEY_FRAME, this, name)
    }
    /// `AssetListReader::getSymbolId(this; name)`: its frame and
    /// `hasKey`'s below it.
    AssetListReader_getSymbolId @ 0x006f_4350 => extern "thiscall" fn get_symbol_id(this: u32, name: u32) -> u32 {
        let mut f = StackFrame::<{ ((stl::SYMBOL_HAS_KEY_EBP + stl::HAS_KEY_FRAME) / 4) as usize }>::new();
        st::get_symbol_id(&mut mem(), f.at() + stl::SYMBOL_HAS_KEY_EBP + stl::HAS_KEY_FRAME, this, name)
    }
    /// `std::vector<word>::push_back(this; &value)`.
    StdVector_pushBack_006f4890 @ 0x006f_4890 => extern "thiscall" fn vector_push_back(this: u32, value: u32) {
        st::vector_push_back(&mut mem(), this, value);
    }
    // ---- sound objects, the FMOD ring, SyncManager ----
    /// The sound objects' constructor `(this)`.
    FfxSound_ctor_00706490 @ 0x0070_6490 => extern "thiscall" fn fmod_objects_ctor(this: u32) -> u32 {
        sb3::fmod_objects_ctor(&mut mem(), this)
    }
    /// `FmodManager`'s command flush `(this)`.
    FmodManager_flushCommands_006fa7e0 @ 0x006f_a7e0 => extern "thiscall" fn fmod_flush(this: u32) -> u32 {
        sb3::fmod_flush(&mut mem(), this)
    }
    /// `SyncManager::initSyncFileList(this)`: frame `ebp - 0x214`.
    SyncManager_initSyncFileList @ 0x006f_aa30 => extern "thiscall" fn init_sync_file_list(this: u32) -> u32 {
        let mut f = StackFrame::<{ (sy::LIST_FRAME / 4) as usize }>::new();
        sb3::init_sync_file_list(&mut mem(), f.at() + sy::LIST_FRAME, this)
    }
    /// `SyncManager::initSyncVoiceList(this)`: frame `ebp - 0x210`.
    SyncManager_initSyncVoiceList @ 0x006f_aba0 => extern "thiscall" fn init_sync_voice_list(this: u32) -> u32 {
        let mut f = StackFrame::<{ (sy::LIST_FRAME / 4) as usize }>::new();
        sb3::init_sync_voice_list(&mut mem(), f.at() + sy::LIST_FRAME, this)
    }
    /// `SyncManager::releaseSyncData(this)`.
    SyncManager_releaseSyncData @ 0x006f_aee0 => extern "thiscall" fn release_sync_data(this: u32) -> u32 {
        sb3::release_sync_data(&mut mem(), this)
    }
    /// `SyncManager::syncDataChange(this; _)`: its frame and the list
    /// readers' below it.
    SyncManager_syncDataChange @ 0x006f_af90 => extern "thiscall" fn sync_data_change(this: u32, _unused: u32) -> u32 {
        let mut f = StackFrame::<{ ((sy::CHANGE_LIST_EBP + sy::LIST_FRAME) / 4) as usize }>::new();
        sb3::sync_data_change(&mut mem(), f.at() + sy::CHANGE_LIST_EBP + sy::LIST_FRAME, this)
    }
    /// `SyncManager_syncDataChangeAsyncTask(args)` (an async task).
    SyncManager_syncDataChangeAsyncTask @ 0x006f_b130 => extern "C" fn sync_data_change_task(args: u32) -> u32 {
        let mut f = StackFrame::<{ ((sy::CHANGE_LIST_EBP + sy::LIST_FRAME) / 4) as usize }>::new();
        sb3::sync_data_change_task(&mut mem(), f.at() + sy::CHANGE_LIST_EBP + sy::LIST_FRAME, args)
    }

    // ---- RadialBlur's parameters, Sys::Thread ----
    /// `RadialBlur_setTextureParam_00703610(this; sampler, texture)`:
    /// the value block in a frame.
    RadialBlur_setTextureParam_00703610 @ 0x0070_3610 => extern "thiscall" fn radial_set_texture_param(this: u32, sampler: u32, texture: u32) -> u32 {
        let mut f = StackFrame::<4>::new();
        mb::post_set_texture_param(&mut mem(), f.at(), this, 0x60, 0x2c, sampler, texture)
    }
    /// `RadialBlur_setParam58_00703780(this; x, y, z, w)`: copies from its
    /// own argument words.
    RadialBlur_setParam58_00703780 @ 0x0070_3780 => extern "thiscall" fn radial_set_param58(this: u32, _x: u32, _y: u32, _z: u32, _w: u32; args: u32) -> u32 {
        ffx_glue_other::frames::post_set_param(&mut mem(), ffx_glue_other::frames::ParamSlot { guard: 0x58, param: 0x58, holder: 0x2c }, this, args)
    }
    /// `RadialBlur_setParam5c_00703820(this; x, y, z, w)`.
    RadialBlur_setParam5c_00703820 @ 0x0070_3820 => extern "thiscall" fn radial_set_param5c(this: u32, _x: u32, _y: u32, _z: u32, _w: u32; args: u32) -> u32 {
        ffx_glue_other::frames::post_set_param(&mut mem(), ffx_glue_other::frames::ParamSlot { guard: 0x60, param: 0x5c, holder: 0x2c }, this, args)
    }
    /// `Sys::Thread::~Thread(this)`.
    SysThread_dtor_00703990 @ 0x0070_3990 => extern "thiscall" fn thread_dtor(this: u32) {
        mb::thread_destruct(&mut mem(), this);
    }
    /// `Sys::Thread`'s deleting destructor `(this; flags)`.
    SysThread_deletingDtor_007039d0 @ 0x0070_39d0 => extern "thiscall" fn thread_delete(this: u32, flags: u32) -> u32 {
        mb::thread_deleting_dtor(&mut mem(), this, flags)
    }
    /// `JobWorker_start_00703a50(this; queue, arg)`.
    JobWorker_start_00703a50 @ 0x0070_3a50 => extern "thiscall" fn job_worker_start(this: u32, queue: u32, arg: u32) -> u32 {
        mb::job_worker_start(&mut mem(), this, queue, arg)
    }

    // ---- round 2: the movie frame queue and its reader ----
    /// `SetFields_00a3dd00(this; a, b, c, d)`.
    SetFields_00a3dd00 @ 0x00a3_dd00 => extern "thiscall" fn set_fields_00a3dd00(this: u32, a: u32, b: u32, c: u32, d: u32) -> u32 {
        mv::set_fields(&mut mem(), this, [a, b, c, d])
    }
    /// `CDataFrameQueue::CDataFrameQueue(this)`.
    CDataFrameQueue_ctor @ 0x00a3_dd20 => extern "thiscall" fn data_frame_queue_ctor(this: u32) -> u32 {
        mv::queue_construct(&mut mem(), this)
    }
    /// The deleting destructor `(this; flags)`.
    CDataFrameQueue_deletingDtor_00a3dd60 @ 0x00a3_dd60 => extern "thiscall" fn data_frame_queue_delete(this: u32, flags: u32) -> u32 {
        mv::queue_deleting_dtor(&mut mem(), this, flags)
    }
    /// `Front(this)`.
    CDataFrameQueue_Front @ 0x00a3_dd90 => extern "thiscall" fn data_frame_queue_front(this: u32) -> u32 {
        mv::queue_front(&mut mem(), this)
    }
    /// `Pop(this)`: `eax` untouched when no writer is woken.
    CDataFrameQueue_Pop @ 0x00a3_ddc0 => extern "thiscall" fn data_frame_queue_pop(this: u32) {
        let _ = mv::queue_pop(&mut mem(), this);
    }
    /// `Count(this)`.
    CDataFrameQueue_Count @ 0x00a3_dde0 => extern "thiscall" fn data_frame_queue_count(this: u32) -> u32 {
        mv::queue_count(&mut mem(), this)
    }
    /// `Init(this; capacity, extra, alloc, free)`.
    CDataFrameQueue_Init @ 0x00a3_ddf0 => extern "thiscall" fn data_frame_queue_init(this: u32, capacity: u32, extra: u32, alloc: u32, free: u32) -> u32 {
        mv::queue_init(&mut mem(), this, capacity, extra, alloc, free)
    }
    /// `Close(this)`.
    CDataFrameQueue_Close @ 0x00a3_dea0 => extern "thiscall" fn data_frame_queue_close(this: u32) -> u32 {
        mv::queue_close(&mut mem(), this)
    }
    /// `Reserve(this; len)`.
    CDataFrameQueue_Reserve @ 0x00a3_dec0 => extern "thiscall" fn data_frame_queue_reserve(this: u32, len: u32) -> u32 {
        mv::queue_reserve(&mut mem(), this, len)
    }
    /// `Commit(this)`.
    CDataFrameQueue_Commit @ 0x00a3_df00 => extern "thiscall" fn data_frame_queue_commit(this: u32) -> u32 {
        mv::queue_commit(&mut mem(), this)
    }
    /// `Destroy(this)`: `eax` untouched when closed already.
    CDataFrameQueue_Destroy @ 0x00a3_df20 => extern "thiscall" fn data_frame_queue_destroy(this: u32) {
        let _ = mv::queue_destroy(&mut mem(), this);
    }
    /// `0x00a3dfb0(this; out, start, len, limit, event)`.
    CDataFrameQueue_waitSpace_00a3dfb0 @ 0x00a3_dfb0 => extern "thiscall" fn data_frame_queue_wait_space(this: u32, out: u32, start: u32, len: u32, limit: u32, event: u32) -> u32 {
        mv::queue_wait_space(&mut mem(), this, out, start, len, limit, event)
    }
    /// `VbfMkvReader::VbfMkvReader(this)`.
    VbfMkvReader_ctor @ 0x00a3_e030 => extern "thiscall" fn vbf_mkv_reader_ctor(this: u32) -> u32 {
        mv::reader_construct(&mut mem(), this)
    }
    /// The deleting destructor `(this; flags)`.
    VbfMkvReader_deletingDtor_00a3e0c0 @ 0x00a3_e0c0 => extern "thiscall" fn vbf_mkv_reader_delete(this: u32, flags: u32) -> u32 {
        mv::reader_deleting_dtor(&mut mem(), this, flags)
    }
    /// `Close(this)`.
    VbfMkvReader_Close @ 0x00a3_e150 => extern "thiscall" fn vbf_mkv_reader_close(this: u32) {
        mv::reader_close(&mut mem(), this);
    }
    /// `Length(this; total, available)`.
    VbfMkvReader_Length @ 0x00a3_e1b0 => extern "thiscall" fn vbf_mkv_reader_length(this: u32, total: u32, available: u32) -> u32 {
        mv::reader_length(&mut mem(), this, total, available)
    }
    /// `Open(this; name)`.
    VbfMkvReader_Open @ 0x00a3_e1f0 => extern "thiscall" fn vbf_mkv_reader_open(this: u32, name: u32) -> u32 {
        mv::reader_open(&mut mem(), this, name)
    }

    // ---- round 2: the occluder class descriptors' methods (`this` unused) ----
    /// Slot 5 `(value, out)`.
    OccluderDescriptor_get_00a424b0 @ 0x00a4_24b0 => extern "stdcall" fn occluder_get(value: u32, out: u32) -> u32 {
        pc::occluder_get(&mut mem(), value, out)
    }
    /// The abstract descriptor's slot 3 `(a, b)`: a message only.
    OccluderDescriptor_notSupported_00a424e0 @ 0x00a4_24e0 => extern "stdcall" fn occluder_not_supported(_a: u32, _b: u32) -> u32 {
        pc::not_supported(&mut mem())
    }
    /// Slot 6 `(L, slot)`.
    OccluderDescriptor_pushSlot_00a42700 @ 0x00a4_2700 => extern "stdcall" fn occluder_push_slot(l: u32, slot: u32) -> u32 {
        pc::push_slot(&mut mem(), l, slot)
    }
    /// The abstract descriptor's slot 4 `(L, value)`.
    OccluderDescriptor_push_00a42740 @ 0x00a4_2740 => extern "stdcall" fn occluder_push_00a42740(l: u32, v: u32) -> u32 {
        pc::push_value(&mut mem(), l, v)
    }
    /// The header descriptors' slot 4 `(L, value)`.
    OccluderDescriptor_push_00a42760 @ 0x00a4_2760 => extern "stdcall" fn occluder_push_00a42760(l: u32, v: u32) -> u32 {
        pc::push_value(&mut mem(), l, v)
    }
    /// Slot 0 `(a, flag)`.
    OccluderDescriptor_registrySet_00a427e0 @ 0x00a4_27e0 => extern "stdcall" fn occluder_registry_set(a: u32, flag: u32) -> u32 {
        pc::registry_set(&mut mem(), a, flag)
    }
    /// The header descriptors' slot 15 `(this; a, b)`.
    OccluderDescriptor_callField_00a42810 @ 0x00a4_2810 => extern "thiscall" fn occluder_call_field(this: u32, a: u32, b: u32) -> u32 {
        pc::call_field(&mut mem(), this, a, b)
    }

    // ---- round 2: the PC read hooks, the focus handler, jobs ----
    /// `PcReadFunc_set`'s sector read: a tail jump to
    /// `CDfileReadSector_CD`.
    PcRead_readSector_00a42870 @ 0x00a4_2870 => extern "C" fn pc_read_sector(sector: u32, buffer: u32, size: u32, callback: u32, opts: u32) -> u32 {
        pc::read_sector(&mut mem(), sector, buffer, size, callback, opts)
    }
    /// `PcReadFunc_set`'s file sector `(file)`: rebased into its slot.
    PcRead_fileSector_00a42880 @ 0x00a4_2880 => extern "C" fn pc_file_sector(file: u32; slots: u32) -> u32 {
        pc::file_sector(&mut mem(), file, slots)
    }
    /// `pc_ReadWholeFile(name, size_out)`.
    pc_ReadWholeFile_00a42a00 @ 0x00a4_2a00 => extern "C" fn pc_read_whole_file(name: u32, size_out: u32) -> u32 {
        pc::read_whole_file(&mut mem(), name, size_out)
    }
    /// `CWindowEventHandler` slot 0 `(this)`: the focus lost.
    WindowEventHandler_focusLost_00a44890 @ 0x00a4_4890 => extern "thiscall" fn focus_lost(this: u32) -> u32 {
        pc::focus_lost(&mut mem(), this)
    }
    /// `CWindowEventHandler` slot 1 `(this)`: the focus back.
    WindowEventHandler_focusGained_00a44960 @ 0x00a4_4960 => extern "thiscall" fn focus_gained(this: u32) -> u32 {
        pc::focus_gained(&mut mem(), this)
    }
    /// `()`: the focus handler registered.
    WindowEventHandler_Register_00a44a30 @ 0x00a4_4a30 => extern "C" fn register_focus_handler() -> u32 {
        pc::register_focus_handler(&mut mem())
    }
    /// `(this; func, a, b, queue)`: a two-word job pushed.
    JobScheduler_pushJob2_00a44a50 @ 0x00a4_4a50 => extern "thiscall" fn push_job2(this: u32, func: u32, a: u32, b: u32, queue: u32; slots: u32) -> u32 {
        pc::push_job(&mut mem(), this, func, &[a, b], queue, slots.wrapping_add(12))
    }
    /// `(chars, count)`.
    JobSchedule_Ch_Cloth_PerCharacter @ 0x00a4_4b40 => extern "C" fn schedule_cloth(chars: u32, count: u32) -> u32 {
        let mut f = StackFrame::<1>::new();
        let mut m = mem();
        // the job as the original pushes it: the harness may have pointed
        // that operand at its seeded wrapper
        let job: u32 = ffx_bridge::Mem::read(&m, ffx_bridge::Mem::rebase(&m, ffx_system::m5b_pc::CLOTH_JOB_PUSH));
        pc::schedule_cloth(&mut m, chars, count, job, f.at())
    }
    /// `()`: the same code as `JobScheduleWait_Ch_Cloth_PerCharacter`.
    JobScheduleWait_Ch_CalcMiddle_Batch @ 0x00a4_4b80 => extern "C" fn wait_calc_middle() {
        ffx_system::m5a_platform::bridge::wait_cloth(&mut mem());
    }
    /// `(chars, pass, count)`.
    JobSchedule_Ch_CalcMiddle_Batch @ 0x00a4_4b90 => extern "C" fn schedule_calc_middle(chars: u32, pass: u32, count: u32) -> u32 {
        let mut f = StackFrame::<1>::new();
        let mut m = mem();
        // the job as the original pushes it (see `schedule_cloth`)
        let job: u32 = ffx_bridge::Mem::read(&m, ffx_bridge::Mem::rebase(&m, ffx_system::m5b_pc::CALC_MIDDLE_JOB_PUSH));
        pc::schedule_calc_middle(&mut m, [chars, pass, count], job, f.at())
    }
    /// `(this; func, a, b, c, queue)`: a three-word job pushed.
    JobScheduler_pushJob3_0071e720 @ 0x0071_e720 => extern "thiscall" fn push_job3(this: u32, func: u32, a: u32, b: u32, c: u32, queue: u32; slots: u32) -> u32 {
        pc::push_job(&mut mem(), this, func, &[a, b, c], queue, slots.wrapping_add(16))
    }
    /// `str_replace(s, what, with)`.
    str_replace @ 0x00a4_4bd0 => extern "C" fn str_replace(s: u32, what: u32, with: u32) -> u32 {
        pc::str_replace(&mut mem(), s, what, with)
    }

    // ---- round 2: the HDD layer and one-jump thunks ----
    /// `()`: `jmp AtelGetScenarioFlag`.
    Thunk_AtelGetScenarioFlag_007726d0 @ 0x0077_26d0 => extern "C" fn scenario_flag() -> u32 {
        hd::scenario_flag(&mut mem())
    }
    /// `()`: the sphere monitor word, zero-extended.
    Thunk_AtelGetSphereMonitor_00772730 @ 0x0077_2730 => extern "C" fn sphere_monitor() -> u32 {
        hd::sphere_monitor(&mut mem())
    }
    /// `(i)`: `jmp hdd_rmode_get`.
    Thunk_hdd_rmode_get_007727f0 @ 0x0077_27f0 => extern "C" fn rmode_get(i: u32) -> u32 {
        hd::rmode_get(&mut mem(), i)
    }
    /// `()`: bit 5 of the buttons byte.
    PadButtonBit5_00772800 @ 0x0077_2800 => extern "C" fn pad_bit5() -> u32 {
        hd::pad_bit(&mut mem(), 5)
    }
    /// `()`: bits 12..15 of the buttons or held word, pad on.
    PadShoulderHeld_00772860 @ 0x0077_2860 => extern "C" fn pad_shoulder() -> u32 {
        hd::pad_any_shoulder(&mut mem())
    }
    /// `()`: bit 6 of the buttons byte.
    PadButtonBit6_00772970 @ 0x0077_2970 => extern "C" fn pad_bit6() -> u32 {
        hd::pad_bit(&mut mem(), 6)
    }
    /// `()`: the word at `0x0112a078`.
    GetGlobal_0112a078_007729d0 @ 0x0077_29d0 => extern "C" fn pad_word() -> u32 {
        hd::pad_word(&mut mem())
    }
    /// `(i)`: `jmp hdd_head_sectors`.
    Thunk_hdd_head_sectors_00773530 @ 0x0077_3530 => extern "C" fn head_sectors(i: u32) -> u32 {
        hd::head_sectors(&mut mem(), i)
    }
    /// `(i, v)`: `jmp SetTableEntry_00779590`.
    Thunk_SetTableEntry_00773940 @ 0x0077_3940 => extern "C" fn set_rmode(i: u32, v: u32) -> u32 {
        hd::set_rmode(&mut mem(), i, v)
    }
    /// `(n)`: `jmp MsPayGIL`.
    Thunk_MsPayGIL_00773990 @ 0x0077_3990 => extern "C" fn pay_gil(n: u32) -> u32 {
        hd::pay_gil(&mut mem(), n)
    }
    /// `()`: `jmp get_hdd_status`.
    Thunk_get_hdd_status_00773b80 @ 0x0077_3b80 => extern "C" fn hdd_status() -> u32 {
        hd::hdd_status(&mut mem())
    }
    /// `(on)`.
    hdd_SetMode_00773c60 @ 0x0077_3c60 => extern "C" fn set_hdd_mode(on: u32) -> u32 {
        hd::set_hdd_mode(&mut mem(), on)
    }
    /// `()`.
    hdd_head_init @ 0x0077_89d0 => extern "C" fn head_init() -> u32 {
        hd::head_init(&mut mem())
    }
    /// `()`.
    hdd_HeadsAssert_00778980 @ 0x0077_8980 => extern "C" fn heads_assert() -> u32 {
        hd::heads_assert(&mut mem())
    }
    /// `(module, n)`.
    cd_QueueModuleRead_0076cf60 @ 0x0076_cf60 => extern "C" fn queue_module_read(module: u32, n: u32) -> u32 {
        hd::queue_module_read(&mut mem(), module, n)
    }
    /// `()`.
    hdd_unmount @ 0x0077_9730 => extern "C" fn hdd_unmount() -> u32 {
        hd::unmount(&mut mem())
    }
    /// `()`.
    hdd_InstalledMb_00778f60 @ 0x0077_8f60 => extern "C" fn installed_mb() -> u32 {
        hd::installed_mb(&mut mem(), 1)
    }
    /// `()`.
    hdd_InstalledMb2_00778fb0 @ 0x0077_8fb0 => extern "C" fn installed_mb2() -> u32 {
        hd::installed_mb(&mut mem(), 2)
    }
    /// `()`.
    hdd_InstallPercent_00779290 @ 0x0077_9290 => extern "C" fn install_percent() -> u32 {
        hd::install_percent(&mut mem())
    }
    /// `()`.
    hdd_install_end @ 0x0077_92c0 => extern "C" fn install_end() -> u32 {
        hd::install_end(&mut mem())
    }
    /// `()`: the word at `0x0112a518`.
    GetGlobal_0112a518_007793a0 @ 0x0077_93a0 => extern "C" fn word_a518() -> u32 {
        hd::word_a518(&mut mem())
    }
    /// `()`.
    hdd_InstallResult_00779880 @ 0x0077_9880 => extern "C" fn install_result() -> u32 {
        hd::install_result(&mut mem())
    }
    /// `()`.
    hdd_DrawProgress_00779450 @ 0x0077_9450 => extern "C" fn draw_progress() -> u32 {
        hd::draw_progress(&mut mem())
    }
    /// `()`.
    hdd_Format_007789a0 @ 0x0077_89a0 => extern "C" fn hdd_format() -> u32 {
        hd::format(&mut mem())
    }
    /// `()`.
    TrophyManager_DestroyInstance @ 0x007a_0cb0 => extern "C" fn trophy_destroy() -> u32 {
        hd::trophy_destroy(&mut mem())
    }
    /// `(lang)`: `jmp FFXSetLang`.
    Thunk_FFXSetLang_00887ca0 @ 0x0088_7ca0 => extern "C" fn set_lang(lang: u32) -> u32 {
        hd::set_lang(&mut mem(), lang)
    }
    /// `()`: `jmp SaveDataIsAlbhedLoaded`.
    Thunk_SaveDataIsAlbhedLoaded_00822540 @ 0x0082_2540 => extern "C" fn albhed_loaded() -> u32 {
        hd::albhed_loaded(&mut mem())
    }
    /// `()`: `jmp SaveDataIsBusy`.
    SaveDataIsBusy_thunk @ 0x0082_2560 => extern "C" fn save_busy() -> u32 {
        hd::save_busy(&mut mem())
    }
    /// `(_, buf, size, head, file)`.
    hd_file_read_no_wait @ 0x0077_85e0 => extern "C" fn read_no_wait(_a: u32, buf: u32, size: u32, head: u32, file: u32) -> u32 {
        hd::read_no_wait(&mut mem(), buf, size, head, file)
    }
    /// `()`.
    cd_CheckDisk_0076d5a0 @ 0x0076_d5a0 => extern "C" fn check_disk() -> u32 {
        hd::check_disk(&mut mem())
    }
    /// `()`.
    cd_AllocReadBuffer_0076dc70 @ 0x0076_dc70 => extern "C" fn alloc_read_buffer() -> u32 {
        hd::alloc_read_buffer(&mut mem())
    }
    /// `()`.
    hdd_DrawBarSprite_007766e0 @ 0x0077_66e0 => extern "C" fn draw_bar_sprite() -> u32 {
        hd::draw_background(&mut mem())
    }
    /// `()`.
    hdd_DrawWallSprite_00776740 @ 0x0077_6740 => extern "C" fn draw_wall_sprite() -> u32 {
        hd::draw_wall(&mut mem())
    }
    /// `TkyiAssert(msg)`: `eax` untouched.
    TkyiAssert @ 0x0077_7680 => extern "C" fn tkyi_assert(msg: u32) {
        hd::assert(&mut mem(), msg);
    }
    /// `(n)`: the slot decremented, the block's flag set.
    cd_SetBlockFlag_0076cf80 @ 0x0076_cf80 => extern "C" fn set_block_flag(n: u32; slots: u32) -> u32 {
        hd::set_block_flag(&mut mem(), n, slots)
    }
    /// `()`: started already, the original leaves `eax` untouched; its
    /// callers reach it through `call eax` (`MsgExpandCodes_00776940`'s
    /// `[0x00c416dc]` table), so that `eax` is its own address.
    hdd_InstallStart_00779120 @ 0x0077_9120 => extern "C" fn install_start() -> u32 {
        let mut m = mem();
        let own = ffx_bridge::Mem::rebase(&m, 0x0077_9120);
        hd::install_start(&mut m).unwrap_or(own)
    }

    // ---- round 3: menu wrappers and table entries ----
    /// `(a, b, c, d)`.
    TOMkpValueFontMClutRight @ 0x0090_42e0 => extern "C" fn value_font_m_clut_right(a: u32, b: u32, c: u32, d: u32) -> u32 {
        mn::value_font_m_clut_right(&mut mem(), [a, b, c, d])
    }
    /// `(a, b, c, d)`.
    TOMkpFont_00902810 @ 0x0090_2810 => extern "C" fn font_902810(a: u32, b: u32, c: u32, d: u32) -> u32 {
        mn::font_902810(&mut mem(), [a, b, c, d])
    }
    /// `(id, x, y, w, h, c0, c1)`.
    TOMkpShapeXYWHC2 @ 0x0090_3b70 => extern "C" fn shape_xywhc2(id: u32, x: u32, y: u32, w: u32, h: u32, c0: u32, c1: u32) -> u32 {
        mn::shape_xywhc2(&mut mem(), [id, x, y, w, h, c0, c1])
    }
    /// `(text, x, y, colour)`.
    TOMkpBaseMesFont_00901cb0 @ 0x0090_1cb0 => extern "C" fn base_mes_font(text: u32, x: u32, y: u32, colour: u32) -> u32 {
        mn::base_mes_font(&mut mem(), [text, x, y, colour])
    }
    /// `(a, b, c, d, e)`.
    TOMkpMesFont_00901d10 @ 0x0090_1d10 => extern "C" fn mes_font_901d10(a: u32, b: u32, c: u32, d: u32, e: u32) -> u32 {
        mn::mes_font_901d10(&mut mem(), [a, b, c, d, e])
    }
    /// `(text, out, x, y)`.
    TOMkpStrFontM_00901f80 @ 0x0090_1f80 => extern "C" fn str_font_m4(a: u32, b: u32, c: u32, d: u32) -> u32 {
        mn::str_font_m4(&mut mem(), [a, b, c, d])
    }
    /// `(text, x, y)`.
    TOMkpEasyStrFontM @ 0x0090_1f40 => extern "C" fn easy_str_font_m(text: u32, x: u32, y: u32) -> u32 {
        mn::easy_str_font_m(&mut mem(), text, x, y)
    }
    /// `(prim)`.
    TOMakePktSeqPrimFunc @ 0x008f_e040 => extern "C" fn seq_prim_func(prim: u32) -> u32 {
        mn::seq_prim_func(&mut mem(), prim)
    }
    /// `(prim)`.
    TOMakePktSeqPrimMes @ 0x008f_e080 => extern "C" fn seq_prim_mes(prim: u32) -> u32 {
        mn::seq_prim_mes(&mut mem(), prim)
    }
    /// `()`.
    TOMenuTransSmallFacePlyTex @ 0x0090_1160 => extern "C" fn trans_small_face() -> u32 {
        mn::trans_small_face(&mut mem())
    }
    /// `(cursor, text, out, a, b, c, d, e, f)`.
    TOAdpStrFontM_008edfc0 @ 0x008e_dfc0 => extern "C" fn str_font_m(cursor: u32, text: u32, out: u32, a: u32, b: u32, c: u32, d: u32, e: u32, f: u32) -> u32 {
        mn::str_font_m(&mut mem(), cursor, text, out, [a, b, c, d, e, f])
    }
    /// `()`: a constant.
    ConstB62cf4_008ac7a0 @ 0x008a_c7a0 => extern "C" fn const_8ac7a0() -> u32 {
        mn::constant(&mut mem(), ffx_system::m5b_menu::STR_B62CF4)
    }
    /// `()`: a constant.
    ConstB62cf4_008b7c90 @ 0x008b_7c90 => extern "C" fn const_8b7c90() -> u32 {
        mn::constant(&mut mem(), ffx_system::m5b_menu::STR_B62CF4)
    }
    /// `()`: a constant.
    ConstB62cf4_008df680 @ 0x008d_f680 => extern "C" fn const_8df680() -> u32 {
        mn::constant(&mut mem(), ffx_system::m5b_menu::STR_B62CF4)
    }
    /// `()`: a constant.
    ConstB66b30_008fa4e0 @ 0x008f_a4e0 => extern "C" fn const_8fa4e0() -> u32 {
        mn::constant(&mut mem(), ffx_system::m5b_menu::STR_B66B30)
    }
    /// `()`: a constant.
    ConstB66b30_00906c10 @ 0x0090_6c10 => extern "C" fn const_906c10() -> u32 {
        mn::constant(&mut mem(), ffx_system::m5b_menu::STR_B66B30)
    }
    /// `()`.
    TkMenuUpdateSaveNeed_008b34f0 @ 0x008b_34f0 => extern "C" fn update_save_need() -> u32 {
        mn::update_save_need(&mut mem())
    }
    /// `()`.
    TkMcUnlock_008b3600 @ 0x008b_3600 => extern "C" fn mc_unlock() -> u32 {
        mn::mc_unlock(&mut mem())
    }
    /// `()`: `jmp 0x008fd500` (no stack words; the target's code inline).
    TOMkpPkt8eb4e0_008be250 @ 0x008b_e250 => extern "C" fn pkt_8eb4e0() -> u32 {
        mn::pkt_8eb4e0(&mut mem())
    }
    /// `()`.
    HddMenuEnter_008bf050 @ 0x008b_f050 => extern "C" fn hdd_menu_enter() -> u32 {
        mn::hdd_menu_enter(&mut mem())
    }
    /// `()`.
    HddMenuFunc_008bf0a0 @ 0x008b_f0a0 => extern "C" fn hdd_menu_func() -> u32 {
        mn::hdd_menu_func(&mut mem())
    }
    /// `()`.
    TkMenuCloseItemWindows_008c66c0 @ 0x008c_66c0 => extern "C" fn close_item_windows() -> u32 {
        mn::close_item_windows(&mut mem())
    }
    /// `()`: `jmp TOAmapGetItemListMax`.
    Thunk_TOAmapGetItemListMax_008c6700 @ 0x008c_6700 => extern "C" fn item_list_max() -> u32 {
        mn::item_list_max(&mut mem())
    }
    /// `()`.
    TkMenuOptionEnter_008e1910 @ 0x008e_1910 => extern "C" fn option_enter() -> u32 {
        mn::option_enter(&mut mem())
    }
    /// `()`.
    TkMenuOptionOpen_008e1950 @ 0x008e_1950 => extern "C" fn option_open() -> u32 {
        mn::option_open(&mut mem())
    }
    /// `()`.
    TkMenuOptionClose_008e19b0 @ 0x008e_19b0 => extern "C" fn option_close() -> u32 {
        mn::option_close(&mut mem())
    }
    /// `()`.
    TkMenuOptionLeave_008e19d0 @ 0x008e_19d0 => extern "C" fn option_leave() -> u32 {
        mn::option_leave(&mut mem())
    }
    /// `()`.
    TkMenuAbmExit_008e27b0 @ 0x008e_27b0 => extern "C" fn abm_exit() -> u32 {
        mn::abm_exit(&mut mem())
    }
    /// `()`: `jmp 0x00a56030` (the target's code inline).
    TkMenuAbmEnter_008e27c0 @ 0x008e_27c0 => extern "C" fn abm_enter() -> u32 {
        mn::abm_enter(&mut mem())
    }
    /// `()`: `jmp 0x00a54760` (the target's code inline).
    TkMenuAbmLeave_008e27d0 @ 0x008e_27d0 => extern "C" fn abm_leave() -> u32 {
        mn::abm_leave(&mut mem())
    }
    /// `(a, b)`: `jmp TOMakePktFont1612Edge`.
    Thunk_TOMakePktFont1612Edge_00906260 @ 0x0090_6260 => extern "C" fn font_1612_edge(a: u32, b: u32) -> u32 {
        mn::font_1612_edge(&mut mem(), a, b)
    }
    /// `()`.
    PrintB53060_0076ef90 @ 0x0076_ef90 => extern "C" fn print_b53060() -> u32 {
        mn::print_b53060(&mut mem())
    }
    /// `()`: `jmp TOGetFFXLangJmp`.
    TOGetFFXLang_008dfe10 @ 0x008d_fe10 => extern "C" fn get_ffx_lang() -> u32 {
        mn::get_ffx_lang(&mut mem())
    }
    /// `()`.
    CallbackOne_008abb00 @ 0x008a_bb00 => extern "C" fn callback_one() -> u32 {
        mn::callback_one(&mut mem())
    }
    /// `(cursor, a, b, c, d, e)`.
    TOAdpMesFont_008ed120 @ 0x008e_d120 => extern "C" fn mes_font_8ed120(cursor: u32, a: u32, b: u32, c: u32, d: u32, e: u32) -> u32 {
        mn::mes_font_8ed120(&mut mem(), [cursor, a, b, c, d, e])
    }
    /// `(a, b, c)`.
    TOMkpFont_008f8870 @ 0x008f_8870 => extern "C" fn font_8f8870(a: u32, b: u32, c: u32) -> u32 {
        mn::font_8f8870(&mut mem(), a, b, c)
    }
    /// `()`: with no names `eax` untouched.
    SetWakkalimitMagicFileTexturesUnloaded @ 0x008e_89d0 => extern "C" fn wakka_textures_unloaded() {
        let _ = mn::wakka_textures_unloaded(&mut mem());
    }
    /// `(pos, glyph)`.
    TOGlyphAdvance_008fc770 @ 0x008f_c770 => extern "C" fn glyph_advance(pos: u32, glyph: u32) -> u32 {
        mn::glyph_advance(&mut mem(), pos, glyph)
    }
    /// `(cursor, x, y, w, c, c1)`.
    TOAdpShapeDaDb_008f1a90 @ 0x008f_1a90 => extern "C" fn shape_da_db_8f1a90(cursor: u32, x: u32, y: u32, w: u32, c: u32, c1: u32) -> u32 {
        mn::shape_da_db(&mut mem(), [cursor, x, y, w, c, c1])
    }
    /// `(cursor, x, y, w, c, c1)`: the same code.
    TOAdpShapeDaDb_008f1af0 @ 0x008f_1af0 => extern "C" fn shape_da_db_8f1af0(cursor: u32, x: u32, y: u32, w: u32, c: u32, c1: u32) -> u32 {
        mn::shape_da_db(&mut mem(), [cursor, x, y, w, c, c1])
    }
    /// `(cursor, id1, id2, x, y, w, c, c1)`.
    TOAdpShapePair_008f0990 @ 0x008f_0990 => extern "C" fn shape_ids(cursor: u32, id1: u32, id2: u32, x: u32, y: u32, w: u32, c: u32, c1: u32) -> u32 {
        mn::shape_ids(&mut mem(), [cursor, id1, id2, x, y, w, c, c1])
    }

    // ---- round 3: magic files, VFX vector cells ----
    /// `(magic)`: off, `eax` untouched.
    MagicFile_clearAllPreloadedPrx @ 0x009d_aca0 => extern "C" fn clear_all_preloaded(magic: u32) {
        let _ = pc::clear_all_preloaded(&mut mem(), magic);
    }
    /// `(magic, file)`: off, `eax` untouched.
    MagicFile_ClearPreloadedPrx_009dacc0 @ 0x009d_acc0 => extern "C" fn clear_preloaded(magic: u32, file: u32) {
        let _ = pc::clear_preloaded(&mut mem(), magic, file);
    }
    /// `(module)`.
    onMagicDLLLoaded @ 0x009d_b110 => extern "C" fn on_magic_dll_loaded(module: u32) -> u32 {
        pc::on_magic_dll_loaded(&mut mem(), module)
    }
    /// `(dst)`.
    VFX_CopyC0a004_0092cc20 @ 0x0092_cc20 => extern "C" fn copy_c0a004(dst: u32) -> u32 {
        pc::copy_c0a004(&mut mem(), dst)
    }
    /// `(dst, src)`.
    VFX_CopyViaC8f980_0092cbf0 @ 0x0092_cbf0 => extern "C" fn copy_via_c8f980(dst: u32, src: u32) -> u32 {
        pc::copy_via_c8f980(&mut mem(), dst, src)
    }

    // ---- round 3, batch 2: packet builders and menu draws ----
    /// `(cursor)`.
    TOAdpTexFlushAlpha_008eb4e0 @ 0x008e_b4e0 => extern "C" fn tex_flush_alpha(cursor: u32) -> u32 {
        mn::tex_flush_alpha(&mut mem(), cursor)
    }
    /// `()`.
    TOMkpPlainDrawRegs_009046e0 @ 0x0090_46e0 => extern "C" fn plain_draw_regs() -> u32 {
        mn::plain_draw_regs(&mut mem())
    }
    /// `(cursor, x, y, w, c, c1)`.
    TOAdpShapeDaDbIcon10b_008f1990 @ 0x008f_1990 => extern "C" fn shape_da_db_10b(cursor: u32, x: u32, y: u32, w: u32, c: u32, c1: u32) -> u32 {
        mn::shape_da_db_icon(&mut mem(), [cursor, x, y, w, c, c1], 0x10b)
    }
    /// `(cursor, x, y, w, c, c1)`.
    TOAdpShapeDaDbIcon10a_008f1a10 @ 0x008f_1a10 => extern "C" fn shape_da_db_10a(cursor: u32, x: u32, y: u32, w: u32, c: u32, c1: u32) -> u32 {
        mn::shape_da_db_icon(&mut mem(), [cursor, x, y, w, c, c1], 0x10a)
    }
    /// `(a, b, x, y, w, c, c1)`.
    TOMkpShapeLeftRight_008f5e90 @ 0x008f_5e90 => extern "C" fn shape_left_right(a: u32, b: u32, x: u32, y: u32, w: u32, c: u32, c1: u32) -> u32 {
        mn::shape_left_right(&mut mem(), [a, b, x, y, w, c, c1])
    }
    /// `(x, y, w, h, c, c1)`: `w` clamped in its argument slot.
    TOMkpWindowFrame_008f9b60 @ 0x008f_9b60 => extern "C" fn window_frame(x: u32, y: u32, w: u32, h: u32, c: u32, c1: u32; slots: u32) -> u32 {
        mn::window_frame(&mut mem(), [x, y, w, h, c, c1], slots)
    }
    /// `(x, y, w, h)`: `w` less the corner in its argument slot.
    TOMkpWindowPlain_008f9a40 @ 0x008f_9a40 => extern "C" fn window_plain(x: u32, y: u32, w: u32, h: u32; slots: u32) -> u32 {
        mn::window_plain(&mut mem(), [x, y, w, h], slots)
    }
    /// `(x0, y0, x1, y1)`.
    TOMakePktSampleBlueWindowXY2 @ 0x008f_de50 => extern "C" fn sample_blue_window(x0: u32, y0: u32, x1: u32, y1: u32) -> u32 {
        mn::sample_blue_window(&mut mem(), [x0, y0, x1, y1])
    }
    /// `(chr, x, y)`.
    TOBtlDrawChangeSeqAnm0 @ 0x0089_28b0 => extern "C" fn change_seq_anm(chr: u32, x: u32, y: u32) -> u32 {
        mn::change_seq_anm(&mut mem(), chr, x, y)
    }
    /// `(save, x, y)`: the info kept in the first argument slot.
    TkMenuDrawSavePartyFaces_008b37d0 @ 0x008b_37d0 => extern "C" fn save_party_faces(save: u32, x: u32, y: u32; slots: u32) -> u32 {
        mn::save_party_faces(&mut mem(), save, x, y, slots)
    }
    /// `(p, slot, x, y, c0, c1, c2, c3)`: the cell's `u` in the slot's
    /// argument slot.
    TOAdpFaceSprite_008f1880 @ 0x008f_1880 => extern "C" fn face_sprite(p: u32, slot: u32, x: u32, y: u32, c0: u32, c1: u32, c2: u32, c3: u32; slots: u32) -> u32 {
        mn::face_sprite(&mut mem(), [p, slot, x, y, c0, c1, c2, c3], slots)
    }
    /// `(cursor, value, x, a, b)`: frame `ebp - 0x24`.
    TOAdpValueFontMClutRight_008f3280 @ 0x008f_3280 => extern "C" fn value_font_m(cursor: u32, value: u32, x: u32, a: u32, b: u32) -> u32 {
        let mut f = StackFrame::<9>::new();
        mn::value_font_m(&mut mem(), f.at() + 0x24, [cursor, value, x, a, b])
    }
    /// `(cursor, value, x, a, b)`: frame `ebp - 0x24`.
    TOAdpValueFont3d80_008f3390 @ 0x008f_3390 => extern "C" fn value_font_3d80(cursor: u32, value: u32, x: u32, a: u32, b: u32) -> u32 {
        let mut f = StackFrame::<9>::new();
        mn::value_font_3d80(&mut mem(), f.at() + 0x24, [cursor, value, x, a, b])
    }
    /// `(value, x, y, c)`: frame `ebp - 0xc`.
    TOMkpBtlLimitLuluMidareCounter @ 0x0090_1280 => extern "C" fn lulu_counter(value: u32, x: u32, y: u32, c: u32) -> u32 {
        let mut f = StackFrame::<3>::new();
        mn::lulu_counter(&mut mem(), f.at() + 0xc, [value, x, y, c])
    }
    /// `(cursor, value, x, a, b)`: frame `ebp - 0x24`.
    TOAdpValueFontLClutRight @ 0x008f_30b0 => extern "C" fn value_font_l_clut_right(cursor: u32, value: u32, x: u32, a: u32, b: u32) -> u32 {
        let mut f = StackFrame::<9>::new();
        mn::value_font_l_clut_right(&mut mem(), f.at() + 0x24, [cursor, value, x, a, b])
    }
    /// `(text, x, y)`: `f32 x` left in the `y` argument slot.
    TOMkpEasyStrFontS @ 0x0090_2040 => extern "C" fn easy_str_font_s(text: u32, x: u32, y: u32; slots: u32) -> u32 {
        mn::easy_str_font_s(&mut mem(), text, x, y, slots)
    }
    /// `(text, x, y)`: frame `ebp - 8`; `f32 x` left in its argument slot.
    TOMkpEasyMesFontLCenter @ 0x0090_1c50 => extern "C" fn easy_mes_font_l_center(text: u32, x: u32, y: u32; slots: u32) -> u32 {
        let mut f = StackFrame::<2>::new();
        mn::easy_mes_font_l_center(&mut mem(), f.at() + 8, [text, x, y], slots)
    }
    /// `(path)`: frame `ebp - 0x204`.
    TOInitBattleFontPath @ 0x008a_d780 => extern "C" fn init_battle_font_path(path: u32) -> u32 {
        let mut f = StackFrame::<0x81>::new();
        mn::init_font_path(&mut mem(), f.at() + 0x204, path, &ffx_system::m5b_menu::BATTLE_FONTS)
    }
    /// `(path)`: frame `ebp - 0x204`.
    TOInitEventFontPath @ 0x008a_d840 => extern "C" fn init_event_font_path(path: u32) -> u32 {
        let mut f = StackFrame::<0x81>::new();
        mn::init_font_path(&mut mem(), f.at() + 0x204, path, &ffx_system::m5b_menu::EVENT_FONTS)
    }

    // ---- round 3, batch 3: glyph packets, UI quads ----
    /// `(pos, glyph)`.
    TOAdpGlyphEdge_00905ef0 @ 0x0090_5ef0 => extern "C" fn glyph_edge(pos: u32, glyph: u32) -> u32 {
        mn::glyph_edge(&mut mem(), pos, glyph)
    }
    /// `(pos, glyph)`.
    TOAdpGlyphFill_009060f0 @ 0x0090_60f0 => extern "C" fn glyph_fill(pos: u32, glyph: u32) -> u32 {
        mn::glyph_fill(&mut mem(), pos, glyph)
    }
    /// `(pos, glyph)`: frame `ebp - 0x1c`.
    TOAdpGlyphDraw_00906050 @ 0x0090_6050 => extern "C" fn glyph_draw(pos: u32, glyph: u32) -> u32 {
        let mut f = StackFrame::<7>::new();
        mn::glyph_draw(&mut mem(), f.at() + 0x1c, pos, glyph)
    }
    /// `(pos, glyph)`: frame `ebp - 0x1c`.
    TOMakePktMovieFont1612 @ 0x008f_d960 => extern "C" fn movie_font_1612(pos: u32, glyph: u32) -> u32 {
        let mut f = StackFrame::<7>::new();
        mn::movie_font_1612(&mut mem(), f.at() + 0x1c, pos, glyph)
    }
    /// `(x, y, w, h, c0, c1)`: frame `ebp - 0xa0`.
    TODrawUIBox_008fd390 @ 0x008f_d390 => extern "C" fn ui_box(x: u32, y: u32, w: u32, h: u32, c0: u32, c1: u32) -> u32 {
        let mut f = StackFrame::<0x28>::new();
        mn::ui_box(&mut mem(), f.at() + 0xa0, [x, y, w, h, c0, c1])
    }
    /// `(x0, y0, x1, y1, r, g, b, a)`: frame `ebp - 0x9c`.
    TODrawUIRect_008fb090 @ 0x008f_b090 => extern "C" fn ui_rect(x0: u32, y0: u32, x1: u32, y1: u32, r: u32, g: u32, b: u32, a: u32) -> u32 {
        let mut f = StackFrame::<0x27>::new();
        mn::ui_rect(&mut mem(), f.at() + 0x9c, [x0, y0, x1, y1, r, g, b, a])
    }
    /// `(cursor, x, y, c0, c1)`: frame `ebp - 0xa8`.
    TODrawUIPicture_008f2bf0 @ 0x008f_2bf0 => extern "C" fn ui_picture(cursor: u32, x: u32, y: u32, c0: u32, c1: u32) -> u32 {
        let mut f = StackFrame::<0x2a>::new();
        mn::ui_picture(&mut mem(), f.at() + 0xa8, [cursor, x, y, c0, c1])
    }
    /// `(x, y, w, h, first, count, total)`: `x + w - 1` left in the `h`
    /// argument slot.
    TODrawScrollBar_00902700 @ 0x0090_2700 => extern "C" fn scroll_bar(x: u32, y: u32, w: u32, h: u32, first: u32, count: u32, total: u32; slots: u32) -> u32 {
        mn::scroll_bar(&mut mem(), [x, y, w, h, first, count, total], slots)
    }
    /// `(dst, src, v0, v1, v2, v3)`: the vector is read in its argument
    /// slots.
    VFX_MulXByValue_0092cc40 @ 0x0092_cc40 => extern "C" fn vec_mul_x(dst: u32, src: u32, _v0: u32, _v1: u32, _v2: u32, _v3: u32; slots: u32) -> u32 {
        mn::vec_mul_x(&mut mem(), dst, src, slots)
    }

    // ---- round 3, batch 3: the HDD reader's requests ----
    /// `(sector, buf, size, head)`.
    hd_file_read_th @ 0x0077_8610 => extern "C" fn read_th(sector: u32, buf: u32, size: u32, head: u32) -> u32 {
        hd::read_th(&mut mem(), sector, buf, size, head)
    }
    /// `(path, buf, size, offset)`.
    hd_file_read_ex_th_main @ 0x0077_8550 => extern "C" fn read_ex_th(path: u32, buf: u32, size: u32, offset: u32) -> u32 {
        hd::read_ex_th(&mut mem(), path, buf, size, offset)
    }
    /// `()`: the background thread's read.
    hd_file_read_th_main @ 0x0077_86f0 => extern "C" fn read_th_main() -> u32 {
        hd::read_th_main(&mut mem())
    }
    /// `(device, need)`: frame `ebp - 4`.
    hdd_FreeSpaceOk_00777610 @ 0x0077_7610 => extern "C" fn free_space_ok(device: u32, need: u32) -> u32 {
        let mut f = StackFrame::<1>::new();
        hd::free_space_ok(&mut mem(), f.at() + 4, device, need)
    }
    /// `(cd)`.
    CdSetReadFuncs_0076cff0 @ 0x0076_cff0 => extern "C" fn set_read_funcs(cd: u32) -> u32 {
        hd::set_read_funcs(&mut mem(), cd)
    }

    // ---- round 3, batch 3: RadialBlur's draws, the sphere grid's item check ----
    /// `(this; texture, _)` (`ret 8`): frame `ebp - 0x2c`.
    RadialBlur_draw_007030e0 @ 0x0070_30e0 => extern "thiscall" fn radial_blur_draw(this: u32, texture: u32, _unused: u32) -> u32 {
        let mut f = StackFrame::<0xb>::new();
        pc::blur_draw(&mut mem(), f.at() + 0x2c, this, texture)
    }
    /// `(this; target)` (`ret 4`): frame `ebp - 0x3c`.
    RadialBlur_drawScaled_00703480 @ 0x0070_3480 => extern "thiscall" fn radial_blur_draw_scaled(this: u32, target: u32) -> u32 {
        let mut f = StackFrame::<0xf>::new();
        pc::blur_draw_scaled(&mut mem(), f.at() + 0x3c, this, target)
    }
    /// `()`: frame `ebp - 0x24`.
    AbmapItemUsable_00a44d30 @ 0x00a4_4d30 => extern "C" fn abmap_item_usable() -> u32 {
        let mut f = StackFrame::<9>::new();
        pc::abmap_item_usable(&mut mem(), f.at() + 0x24)
    }
    /// `()`: not running, `eax` untouched.
    AbmapRefreshItems_00a44ea0 @ 0x00a4_4ea0 => extern "C" fn abmap_refresh_items() {
        let mut f = StackFrame::<9>::new();
        let _ = pc::abmap_refresh_items(&mut mem(), f.at() + 0x24);
    }

    // ---- round 3, batch 4: small menu wrappers ----
    /// `(win)`.
    TOWaitBtlWinStart_008a8750 @ 0x008a_8750 => extern "C" fn wait_win_start(win: u32) -> u32 {
        mn::wait_win_start(&mut mem(), win)
    }
    /// `(a, b, c)`.
    TOMkpMenuIcon_008fd4d0 @ 0x008f_d4d0 => extern "C" fn menu_icon(a: u32, b: u32, c: u32) -> u32 {
        mn::menu_icon(&mut mem(), [a, b, c])
    }
    /// `(a, b, c, d, e)`.
    TOMkpMovieMessageImage @ 0x0090_2840 => extern "C" fn movie_message_image(a: u32, b: u32, c: u32, d: u32, e: u32) -> u32 {
        mn::movie_message_image(&mut mem(), [a, b, c, d, e])
    }
    /// `(a, b, c, d)`.
    TOAdpFont8ebc00_008ebbd0 @ 0x008e_bbd0 => extern "C" fn font_8ebc00(a: u32, b: u32, c: u32, d: u32) -> u32 {
        mn::font_8ebc00(&mut mem(), [a, b, c, d])
    }
    /// `(cursor, text, x, y)`.
    TOAdpMesFont1612_008e9fd0 @ 0x008e_9fd0 => extern "C" fn mes_font_1612(a: u32, b: u32, c: u32, d: u32) -> u32 {
        mn::mes_font_1612(&mut mem(), [a, b, c, d])
    }
    /// `(a, b, c, d, e)`.
    TOAdpFont8ea680_008ea5c0 @ 0x008e_a5c0 => extern "C" fn font_8ea680(a: u32, b: u32, c: u32, d: u32, e: u32) -> u32 {
        mn::font_8ea680(&mut mem(), [a, b, c, d, e])
    }
    /// `(a, text)`: frame `ebp - 8`.
    TOGetWidthExtMes1612 @ 0x008b_8070 => extern "C" fn width_ext_mes_1612(a: u32, text: u32) -> u32 {
        let mut f = StackFrame::<2>::new();
        mn::width_ext_mes_1612(&mut mem(), f.at() + 8, a, text)
    }
    /// `(fmt, ...)`: the variable words stay the caller's (`vsprintf`
    /// reads them from `slots + 4`); frame `ebp - 0x104`.
    TODapsPrintf @ 0x0089_dde0 => extern "C" fn daps_printf(fmt: u32; slots: u32) -> u32 {
        let mut f = StackFrame::<0x41>::new();
        mn::daps_printf(&mut mem(), f.at() + 0x104, fmt, slots.wrapping_add(4))
    }
    /// `(x, y, w, h, r, g, b, a)`.
    TOMkpFlatRect_008fc6a0 @ 0x008f_c6a0 => extern "C" fn flat_rect(x: u32, y: u32, w: u32, h: u32, r: u32, g: u32, b: u32, a: u32) -> u32 {
        mn::flat_rect(&mut mem(), [x, y, w, h, r, g, b, a])
    }
    /// `(page, a1, a2, a3, a4, a5, a6)`: frame `ebp - 0x70`.
    TOTransImageLH @ 0x008b_0da0 => extern "C" fn trans_image_lh(page: u32, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32) -> u32 {
        let mut f = StackFrame::<0x1c>::new();
        mn::trans_image_lh(&mut mem(), f.at() + 0x70, [page, a1, a2, a3, a4, a5, a6])
    }

    // ---- round 3, batch 4: the disk checks, sector reads, the install wall ----
    /// `(ctx)`.
    hdd_DiskChangedCached_007775e0 @ 0x0077_75e0 => extern "C" fn disk_changed_cached(ctx: u32) -> u32 {
        hd::disk_changed_cached(&mut mem(), ctx)
    }
    /// `(offset, buf, skip, ctx)`.
    hdd_StreamRead_00777380 @ 0x0077_7380 => extern "C" fn stream_read(offset: u32, buf: u32, skip: u32, ctx: u32) -> u32 {
        hd::stream_read(&mut mem(), [offset, buf, skip, ctx])
    }
    /// `()`: `jmp hdd_ReadModuleFiles_00777f00`.
    Thunk_hdd_ReadModuleFiles_00777970 @ 0x0077_7970 => extern "C" fn read_module_files() -> u32 {
        hd::read_module_files(&mut mem())
    }
    /// `()`.
    hdd_kabe_disp @ 0x0077_93c0 => extern "C" fn kabe_disp() -> u32 {
        hd::kabe_disp(&mut mem())
    }

    // ---- round 3, batch 4: the frame-buffer quad, the HDD install mode and mount, main ----
    /// `()`: frame `ebp - 0x9c`.
    TODrawRealFrameBuffer_008a92a0 @ 0x008a_92a0 => extern "C" fn real_frame_buffer() -> u32 {
        let mut f = StackFrame::<0x27>::new();
        mn::real_frame_buffer(&mut mem(), f.at() + 0x9c)
    }
    /// `()`.
    hdd_install_init @ 0x0077_92e0 => extern "C" fn install_init() -> u32 {
        hd::install_init(&mut mem())
    }
    /// `()`: frame `ebp - 0x84`.
    hdd_mount @ 0x0077_9490 => extern "C" fn mount() -> u32 {
        let mut f = StackFrame::<0x21>::new();
        hd::mount(&mut mem(), f.at() + 0x84)
    }
    /// `(argc, argv)`: the game's `main`.
    GameMain_00822490 @ 0x0082_2490 => extern "C" fn game_main(argc: u32, argv: u32) -> u32 {
        pc::game_main(&mut mem(), argc, argv)
    }

    // ---- round 3, batch 5: the screen capture, the CD offsets, the install check ----
    /// `(x, y)`.
    TOCaptureScreen_008adaf0 @ 0x008a_daf0 => extern "C" fn capture_screen(x: u32, y: u32) -> u32 {
        mn::capture_screen(&mut mem(), x, y)
    }
    /// `(names)`: frame `ebp - 0x28`; without names `eax` untouched.
    file_offset_get @ 0x0076_db40 => extern "C" fn file_offset_get(names: u32) {
        let mut f = StackFrame::<0xa>::new();
        let _ = hd::file_offset_get(&mut mem(), f.at() + 0x28, names);
    }
    /// `()`.
    hdd_VerifyFiles_007787c0 @ 0x0077_87c0 => extern "C" fn verify_files() -> u32 {
        hd::verify_files(&mut mem())
    }
    /// `(arg)`.
    hdd_install_check @ 0x0077_91a0 => extern "C" fn install_check(arg: u32) -> u32 {
        hd::install_check(&mut mem(), arg)
    }

    // ---- round 3, batch 5: the partition size, the help picture, the map-object reload ----
    /// `()`: frame `ebp - 0x44`.
    hdd_OpenFfxPartition_00779000 @ 0x0077_9000 => extern "C" fn open_ffx_partition() -> u32 {
        let mut f = StackFrame::<0x11>::new();
        hd::open_ffx_partition(&mut mem(), f.at() + 0x44)
    }
    /// `(page, clut)`: frame `ebp - 0x68` (16-byte aligned).
    hdd_HelpPicture_00779ad0 @ 0x0077_9ad0 => extern "C" fn help_picture(page: u32, clut: u32) -> u32 {
        let mut f = StackFrame::<0x1a>::new();
        hd::help_picture(&mut mem(), f.at() + 0x68, page, clut)
    }
    /// `()`: frame `ebp - 0x10c`.
    rcReloadMapObjects_0092cdd0 @ 0x0092_cdd0 => extern "C" fn reload_map_objects() -> u32 {
        let mut f = StackFrame::<0x43>::new();
        pc::reload_map_objects(&mut mem(), f.at() + 0x10c)
    }

    // ---- round 3, batch 5: printf2, the install thread, the disc wait ----
    /// `(fmt, args)`: frame `ebp - 0x204`.
    printf2 @ 0x0076_dd10 => extern "C" fn printf2(fmt: u32, args: u32) -> u32 {
        let mut f = StackFrame::<0x81>::new();
        mn::printf2(&mut mem(), f.at() + 0x204, fmt, args)
    }
    /// `()`: the install thread; frame `ebp - 0x44`.
    hdd_thread_main @ 0x0077_95b0 => extern "C" fn install_thread() -> u32 {
        let mut f = StackFrame::<0x11>::new();
        hd::install_thread(&mut mem(), f.at() + 0x44)
    }
    /// `()`.
    CdWaitDisc_0076ccb0 @ 0x0076_ccb0 => extern "C" fn wait_disc() -> u32 {
        hd::wait_disc(&mut mem())
    }

    // ---- round 3, batch 6: the gradient box, the textured quad, the wall frame ----
    /// `(x, y, w, h, c0, c1, full)`: frame `ebp - 0xa4`.
    TODrawBoxXYWHC2FW @ 0x008f_3eb0 => extern "C" fn box_fill_width(x: u32, y: u32, w: u32, h: u32, c0: u32, c1: u32, full: u32) -> u32 {
        let mut f = StackFrame::<0x29>::new();
        mn::box_fill_width(&mut mem(), f.at() + 0xa4, [x, y, w, h, c0, c1, full])
    }
    /// `(x, y, w, h, texture)`: frame `ebp - 0xa0`.
    TODrawUITextured_00774520 @ 0x0077_4520 => extern "C" fn ui_textured(x: u32, y: u32, w: u32, h: u32, texture: u32) -> u32 {
        let mut f = StackFrame::<0x28>::new();
        mn::ui_textured(&mut mem(), f.at() + 0xa0, [x, y, w, h, texture])
    }
    /// `(frame, clut)`: frame `ebp - 0x6c` (16-byte aligned).
    hdd_WallFrame_007798b0 @ 0x0077_98b0 => extern "C" fn wall_frame(frame: u32, clut: u32) -> u32 {
        let mut f = StackFrame::<0x1b>::new();
        hd::wall_frame(&mut mem(), f.at() + 0x6c, frame, clut)
    }

    /// `(chr, x, y)`: 16-bit sums in the argument slots; without a bar
    /// `eax` is the FPU status word (the shim leaves the caller's).
    TOBtlDrawLimitGauge @ 0x0089_5480 => extern "C" fn limit_gauge(chr: u32, x: u32, y: u32; slots: u32) {
        let _ = mn::limit_gauge(&mut mem(), chr, x, y, slots);
    }
    /// `(chr, x, y)`: the HP's `y` in the high half of `y`'s slot.
    TOBtlDrawParamPlate @ 0x0089_5940 => extern "C" fn param_plate(chr: u32, x: u32, y: u32; slots: u32) -> u32 {
        mn::param_plate(&mut mem(), chr, x, y, slots)
    }

    /// `(cursor, page, x, y, w, h)`.
    TOAdpMovieImage_008f2e10 @ 0x008f_2e10 => extern "C" fn movie_image(cursor: u32, page: u32, x: u32, y: u32, w: u32, h: u32) -> u32 {
        mn::movie_image(&mut mem(), [cursor, page, x, y, w, h])
    }
    /// `(cursor, x, y, w, r, g, b, a)`: the colour bytes and `y - 3` left in
    /// the argument slots.
    TOAdpBarCaps_008ebc00 @ 0x008e_bc00 => extern "C" fn bar_caps(cursor: u32, x: u32, y: u32, w: u32, r: u32, g: u32, b: u32, a: u32; slots: u32) -> u32 {
        mn::bar_caps(&mut mem(), [cursor, x, y, w, r, g, b, a], slots)
    }

    /// `(cursor, text, x, y, flag, r, g, b, a, kind, scale)`: `f32 scale`
    /// left in the cursor's argument slot.
    TOAdpEasyMesFontLClutRGBAInterMode @ 0x008e_d190 => extern "C" fn mes_font_inter(cursor: u32, text: u32, x: u32, y: u32, flag: u32, r: u32, g: u32, b: u32, a: u32, kind: u32, scale: u32; slots: u32) -> u32 {
        mn::mes_font_inter(&mut mem(), [cursor, text, x, y, flag, r, g, b, a, kind, scale], slots)
    }

    /// `(pos, glyph)`: the font page left in `pos`'s argument slot.
    TOMakePktMovieFont1612Font @ 0x008f_dc00 => extern "C" fn movie_font_glyph(pos: u32, glyph: u32; slots: u32) -> u32 {
        mn::movie_font_glyph(&mut mem(), pos, glyph, slots)
    }

    /// `(cursor, x, y, kind)`.
    TOAdpShapeByKind_008f1b50 @ 0x008f_1b50 => extern "C" fn shape_by_kind(cursor: u32, x: u32, y: u32, kind: u32) -> u32 {
        mn::shape_by_kind(&mut mem(), [cursor, x, y, kind])
    }

    /// `(x, y, w, h, c0, c1, full)` (`x`..`h` floats): frame `ebp - 0xb0`.
    TODrawCrossBoxXYWHC2FW @ 0x008f_4c40 => extern "C" fn cross_box_fill_width(x: u32, y: u32, w: u32, h: u32, c0: u32, c1: u32, full: u32) -> u32 {
        let mut f = StackFrame::<0x2c>::new();
        mn::cross_box_fill_width(&mut mem(), f.at() + 0xb0, [x, y, w, h, c0, c1, full])
    }

    /// `(item, x, y, colour)` (`x`, `y` floats): frame `ebp - 8`; the `y`
    /// argument slot ends holding the name's `x`.
    TODrawCrossMenuIconImportantName @ 0x008f_50f0 => extern "C" fn important_icon_name(item: u32, x: u32, y: u32, colour: u32; slots: u32) -> u32 {
        let mut f = StackFrame::<2>::new();
        mn::important_icon_name(&mut mem(), f.at() + 8, [item, x, y, colour], slots)
    }

    /// `(list)`: frame `ebp - 0x120`.
    hdd_ins_check @ 0x0077_8d70 => extern "C" fn ins_check(list: u32) -> u32 {
        let mut f = StackFrame::<0x48>::new();
        hd::ins_check(&mut mem(), f.at() + 0x120, list)
    }

    /// `()`.
    TODrawWakkaLimitFrame_008a1e50 @ 0x008a_1e50 => extern "C" fn wakka_frame() -> u32 {
        mn::wakka_frame(&mut mem())
    }

    /// `(win, macro, pos)`: the step left in `win`'s argument slot.
    TODrawMovieChangeMacro2 @ 0x008b_5ea0 => extern "C" fn movie_change_macro(win: u32, macro_id: u32, pos: u32; slots: u32) -> u32 {
        mn::movie_change_macro(&mut mem(), win, macro_id, pos, slots)
    }

    /// `(cursor, key, text, x, y, flag, r, g, b, a)`: `f32 x` kept in
    /// `cursor`'s argument slot.
    TOAdpFont1612Text_008ea680 @ 0x008e_a680 => extern "C" fn font_1612_text(cursor: u32, key: u32, text: u32, x: u32, y: u32, flag: u32, r: u32, g: u32, b: u32, a: u32; slots: u32) -> u32 {
        mn::font_1612_text(&mut mem(), [cursor, key, text, x, y, flag, r, g, b, a], slots)
    }

    // ---- round 3, batch 11: JobWorker's thread loop, FileReadMain ----
    /// `(this)`: never returns.
    ThreadLoop_00703a20 @ 0x0070_3a20 => extern "thiscall" fn job_worker_loop(this: u32) {
        pc::job_worker_loop(&mut mem(), this);
    }
    /// `()`: `jmp [[0x01128adc]]`.
    FileReadMain @ 0x0076_cfd0 => extern "C" fn file_read_main() -> u32 {
        pc::file_read_main(&mut mem())
    }

    // ---- round 3, batch 13: the movie message's end, the movie disc wait ----
    /// `()`: frame `ebp - 8` (never written: 0).
    TOMovieMessageEnd_008ab860 @ 0x008a_b860 => extern "C" fn movie_message_end() -> u32 {
        let mut f = StackFrame::<2>::new();
        mn::movie_message_end(&mut mem(), f.at() + 8)
    }
    /// `(play, ctx)`: the ioctl's result goes into `ctx`'s argument slot.
    hdd_MovieDiscWait_00777440 @ 0x0077_7440 => extern "C" fn movie_disc_wait(play: u32, ctx: u32; slots: u32) -> u32 {
        hd::movie_disc_wait(&mut mem(), play, ctx, slots)
    }

    /// `(lo, hi, uv, colour)`: frame `ebp - 0x10`; `uv` advanced in its
    /// argument slot.
    updateVFXRenderData_AkazaScrFlt @ 0x0071_ce20 => extern "C" fn akaza_scr_flt(lo: u32, hi: u32, uv: u32, colour: u32; slots: u32) -> u32 {
        let mut f = StackFrame::<4>::new();
        pc::akaza_scr_flt(&mut mem(), f.at() + 0x10, [lo, hi, uv, colour], slots)
    }

    /// `(out, chr)`: frame `ebp - 0x25c`.
    rcbgGuideMarker_0092c740 @ 0x0092_c740 => extern "C" fn guide_marker(out: u32, chr: u32) -> u32 {
        let mut f = StackFrame::<0x97>::new();
        pc::guide_marker(&mut mem(), f.at() + 0x25c, out, chr)
    }

    /// `(pos, glyph)`: frame `ebp - 0x28`.
    TOMakePktMovieFont1612Edge @ 0x008f_d9f0 => extern "C" fn movie_font_edge(pos: u32, glyph: u32) -> u32 {
        let mut f = StackFrame::<0xa>::new();
        mn::movie_font_edge(&mut mem(), f.at() + 0x28, pos, glyph)
    }

    /// `(alpha)`: frame `ebp - 0x9c`.
    TODrawAlphaBG @ 0x008f_3a50 => extern "C" fn alpha_bg(alpha: u32) -> u32 {
        let mut f = StackFrame::<0x27>::new();
        mn::alpha_bg(&mut mem(), f.at() + 0x9c, alpha)
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

/// Whether the port named `name` is on (`FFX_HOOK_DISABLE` with it or
/// `replacements` turns it off; `FFX_HOOK_ONLY` without it too).
fn wrapped_port_on(name: &str) -> bool {
    let off = std::env::var("FFX_HOOK_DISABLE").unwrap_or_default();
    let only = crate::hook_only().is_none_or(|names| names.iter().any(|n| n == name));
    only && !off
        .split(',')
        .map(str::trim)
        .any(|n| n == "replacements" || n == name)
}

/// The names that turn the PS2 file-size ports off or on alone: the mod
/// wrappers (`mods.rs`) own `CDfileSize_CD`'s and `CDfileSize_PC`'s
/// addresses and call these ports instead of their trampolines.
pub(crate) const CD_FILE_SIZE_CD_NAME: &str = "CDfileSize_CD_port";
pub(crate) const CD_FILE_SIZE_PC_NAME: &str = "CDfileSize_PC_port";

/// `CDfileSize_CD(number)` in Rust (`ffx_system::m5b_hdd::bridge::
/// file_size_cd`), or `None` when turned off.
pub(crate) fn cd_file_size_cd_port(number: u32) -> Option<u32> {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if !*ON.get_or_init(|| wrapped_port_on(CD_FILE_SIZE_CD_NAME)) {
        return None;
    }
    Some(hd::file_size_cd(&mut mem(), number))
}

/// `CDfileSize_PC(number)` in Rust (`ffx_system::m5b_hdd::bridge::
/// file_size_pc`), or `None` when turned off.
pub(crate) fn cd_file_size_pc_port(number: u32) -> Option<u32> {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if !*ON.get_or_init(|| wrapped_port_on(CD_FILE_SIZE_PC_NAME)) {
        return None;
    }
    Some(hd::file_size_pc(&mut mem(), number))
}
