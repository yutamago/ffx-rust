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
        pc::schedule_cloth(&mut mem(), chars, count, f.at())
    }
    /// `()`: the same code as `JobScheduleWait_Ch_Cloth_PerCharacter`.
    JobScheduleWait_Ch_CalcMiddle_Batch @ 0x00a4_4b80 => extern "C" fn wait_calc_middle() {
        ffx_system::m5a_platform::bridge::wait_cloth(&mut mem());
    }
    /// `(chars, pass, count)`.
    JobSchedule_Ch_CalcMiddle_Batch @ 0x00a4_4b90 => extern "C" fn schedule_calc_middle(chars: u32, pass: u32, count: u32) -> u32 {
        let mut f = StackFrame::<1>::new();
        pc::schedule_calc_middle(&mut mem(), chars, pass, count, f.at())
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
