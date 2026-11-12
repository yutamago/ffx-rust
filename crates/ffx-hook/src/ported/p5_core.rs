//! `ffx-pcore`: `PhyreEngine`'s core (M4 port round 1, list `p5-core`):
//! the matrix helpers, the thread primitives and pool, the free lists,
//! the strings and the class descriptors' little members.
//!
//! Threads: the thread primitives and the free lists run on every thread
//! (the job workers, the loader, the render thread). Shims keep their
//! frames on their own stack (`StackFrame`, at most 16 words); nothing is
//! shared.

use super::{StackFrame, mem, ported};

ported! {
    /// `Phyre_PString_PString` `(this, s)` -> `this`.
    Phyre_PString_PString @ 0x0042_f090 => extern "thiscall" fn p5_0042f090(this: u32, s: u32) -> u32 {
        ffx_pcore::string::bridge::new(&mut mem(), this, s)
    }
    /// `Phyre_PhyreMallocArray_0042fae0` `(n, size)` -> the block or 0.
    Phyre_PhyreMallocArray_0042fae0 @ 0x0042_fae0 => extern "C" fn p5_0042fae0(n: u32, size: u32) -> u32 {
        ffx_pcore::string::bridge::calloc_aligned(&mut mem(), n, size)
    }
    /// `Phyre_PhyreRealloc_0042fbd0` `(p, size)` -> the block or 0.
    Phyre_PhyreRealloc_0042fbd0 @ 0x0042_fbd0 => extern "C" fn p5_0042fbd0(p: u32, size: u32) -> u32 {
        ffx_pcore::string::bridge::realloc_aligned(&mut mem(), p, size)
    }
    /// `Phyre_PEventFlagWin32_PEventFlagWin32` `(this)`: no handle; `this`.
    Phyre_PEventFlagWin32_PEventFlagWin32 @ 0x0042_fc80 => extern "thiscall" fn p5_0042fc80(this: u32) -> u32 {
        ffx_pcore::sync::bridge::handle_init(&mut mem(), this)
    }
    /// `Phyre_PSemaphoreWin32_PSemaphoreWin32` `(this)`: no handle; `this`.
    Phyre_PSemaphoreWin32_PSemaphoreWin32 @ 0x0042_fc90 => extern "thiscall" fn p5_0042fc90(this: u32) -> u32 {
        ffx_pcore::sync::bridge::handle_init(&mut mem(), this)
    }
    /// `Phyre_PThreadInitializationWin32_PThreadInitializationWin32` `(this, entry)` -> `this`.
    Phyre_PThreadInitializationWin32_PThreadInitializationWin32 @ 0x0042_fca0 => extern "thiscall" fn p5_0042fca0(this: u32, entry: u32) -> u32 {
        ffx_pcore::sync::bridge::thread_initialization(&mut mem(), this, entry)
    }
    /// `Phyre_PEventFlagWin32_initialize` `(this, flags)` -> 0.
    Phyre_PEventFlagWin32_initialize @ 0x0043_0050 => extern "thiscall" fn p5_00430050(this: u32, flags: u32) -> u32 {
        ffx_pcore::sync::bridge::event_initialize(&mut mem(), this, flags)
    }
    /// `Phyre_PSemaphoreWin32_initialize` `(this, initial, max)` -> 0.
    Phyre_PSemaphoreWin32_initialize @ 0x0043_0080 => extern "thiscall" fn p5_00430080(this: u32, initial: u32, max: u32) -> u32 {
        ffx_pcore::sync::bridge::semaphore_initialize(&mut mem(), this, initial, max)
    }
    /// `Phyre_PEventFlagWin32_reset` `(this)` -> 0.
    Phyre_PEventFlagWin32_reset @ 0x0043_01e0 => extern "thiscall" fn p5_004301e0(this: u32) -> u32 {
        ffx_pcore::sync::bridge::event_reset(&mut mem(), this)
    }
    /// `Phyre_PEventFlagWin32_signal` `(this)` -> 0.
    Phyre_PEventFlagWin32_signal @ 0x0043_0200 => extern "thiscall" fn p5_00430200(this: u32) -> u32 {
        ffx_pcore::sync::bridge::event_signal(&mut mem(), this)
    }
    /// `Phyre_PSemaphoreWin32_signal` `(this, count, previous)` -> 0; the previous count lands in a frame word first.
    Phyre_PSemaphoreWin32_signal @ 0x0043_0210 => extern "thiscall" fn p5_00430210(this: u32, count: u32, previous: u32) -> u32 {
        let mut local = StackFrame::<1>::new();
        ffx_pcore::sync::bridge::semaphore_signal(&mut mem(), local.at(), this, count, previous)
    }
    /// `Phyre_PEventFlagWin32_waitTimeout_00430280` `(this, ms)` -> 0.
    Phyre_PEventFlagWin32_waitTimeout_00430280 @ 0x0043_0280 => extern "thiscall" fn p5_00430280(this: u32, ms: u32) -> u32 {
        ffx_pcore::sync::bridge::wait_for(&mut mem(), this, ms)
    }
    /// `Phyre_PEventFlagWin32_wait` `(this)` -> 0.
    Phyre_PEventFlagWin32_wait @ 0x0043_02a0 => extern "thiscall" fn p5_004302a0(this: u32) -> u32 {
        ffx_pcore::sync::bridge::wait(&mut mem(), this)
    }
    /// `Phyre_PSemaphoreWin32_wait` `(this)` -> 0.
    Phyre_PSemaphoreWin32_wait @ 0x0043_02b0 => extern "thiscall" fn p5_004302b0(this: u32) -> u32 {
        ffx_pcore::sync::bridge::wait(&mut mem(), this)
    }
    /// `Phyre_PCriticalSectionSimple_PCriticalSectionSimple` `(this)` -> `this`.
    Phyre_PCriticalSectionSimple_PCriticalSectionSimple @ 0x0043_1690 => extern "thiscall" fn p5_00431690(this: u32) -> u32 {
        ffx_pcore::sync::bridge::critical_section_simple(&mut mem(), this)
    }
    /// `Phyre_PEventFlagWithByte_init_004316d0` `(this)` -> `this`.
    Phyre_PEventFlagWithByte_init_004316d0 @ 0x0043_16d0 => extern "thiscall" fn p5_004316d0(this: u32) -> u32 {
        ffx_pcore::sync::bridge::event_flag_with_byte(&mut mem(), this)
    }
    /// `Phyre_PThreadInitializationBase_PThreadInitializationBase` `(this, entry)` -> `this`.
    Phyre_PThreadInitializationBase_PThreadInitializationBase @ 0x0043_16f0 => extern "thiscall" fn p5_004316f0(this: u32, entry: u32) -> u32 {
        ffx_pcore::sync::bridge::thread_initialization(&mut mem(), this, entry)
    }
    /// `Phyre_PCriticalSectionBase_initialize` `(this)` -> 0 or the event's result.
    Phyre_PCriticalSectionBase_initialize @ 0x0043_17f0 => extern "thiscall" fn p5_004317f0(this: u32) -> u32 {
        ffx_pcore::sync::bridge::critical_section_initialize(&mut mem(), this)
    }
    /// `Phyre_PCriticalSectionSimple_lockWait_004318c0` `(this, lock)` -> `InterlockedExchangeAdd`'s result.
    Phyre_PCriticalSectionSimple_lockWait_004318c0 @ 0x0043_18c0 => extern "thiscall" fn p5_004318c0(this: u32, lock: u32) -> u32 {
        ffx_pcore::sync::bridge::lock_wait(&mut mem(), this, lock)
    }
    /// `Phyre_PCriticalSectionSimple_lock` `(this)` -> the waiter count before the decrement.
    Phyre_PCriticalSectionSimple_lock @ 0x0043_1920 => extern "thiscall" fn p5_00431920(this: u32) -> u32 {
        ffx_pcore::sync::bridge::critical_section_lock(&mut mem(), this)
    }
    /// `Phyre_PCriticalSectionBase_release_00431950` `(this)`.
    Phyre_PCriticalSectionBase_release_00431950 @ 0x0043_1950 => extern "thiscall" fn p5_00431950(this: u32) {
        ffx_pcore::sync::bridge::critical_section_release(&mut mem(), this);
    }
    /// `Phyre_PCriticalSectionSimple_unlock` `(this)` -> 0 (signalled) or the exchange's result.
    Phyre_PCriticalSectionSimple_unlock @ 0x0043_1a80 => extern "thiscall" fn p5_00431a80(this: u32) -> u32 {
        ffx_pcore::sync::bridge::critical_section_unlock(&mut mem(), this)
    }
    /// `Phyre_PStringBuilder_operator` `(builder, s)` (append's `eax`, unread).
    Phyre_PStringBuilder_operator @ 0x0044_4bf0 => extern "thiscall" fn p5_00444bf0(this: u32, s: u32) {
        ffx_pcore::string::bridge::append_str(&mut mem(), this, s);
    }
    /// `Phyre_AllocCopy_00444c50` `(n, src, count)` -> the block or 0.
    Phyre_AllocCopy_00444c50 @ 0x0044_4c50 => extern "C" fn p5_00444c50(n: u32, src: u32, count: u32) -> u32 {
        ffx_pcore::string::bridge::alloc_copy(&mut mem(), n, src, count)
    }
    /// `Phyre_PStringBuilder_append_00444ce0` `(builder, s, n)`: its `eax` is unread (memchr's pointer or the length; the caller's on the early return).
    Phyre_PStringBuilder_append_00444ce0 @ 0x0044_4ce0 => extern "thiscall" fn p5_00444ce0(this: u32, s: u32, n: u32) {
        ffx_pcore::string::bridge::append(&mut mem(), this, s, n);
    }
    /// `PClassDesc_Construct1_004459e0` `(p)`: 1 word(s) clear when `p` is set; `eax` `p` with `al = 1`.
    PClassDesc_Construct1_004459e0 @ 0x0044_59e0 => extern "stdcall" fn p5_004459e0(p: u32) -> u32 {
        ffx_pcore::small::bridge::clear_words(&mut mem(), p, 1)
    }
    /// `PClassDesc_DestroyPString_00445a00` `(p)` -> `al`.
    PClassDesc_DestroyPString_00445a00 @ 0x0044_5a00 => extern "stdcall" fn p5_00445a00(p: u32) -> u8 {
        ffx_pcore::small::bridge::free_string(&mut mem(), p)
    }
    /// `PClassDesc_Return0_00445a20` `(p)` -> 0.
    PClassDesc_Return0_00445a20 @ 0x0044_5a20 => extern "stdcall" fn p5_00445a20(_p: u32) -> u32 {
        0
    }
    /// `Phyre_PString_Assign` `(this, s)` -> 0 or `0xd`.
    Phyre_PString_Assign @ 0x0044_5cf0 => extern "thiscall" fn p5_00445cf0(this: u32, s: u32) -> u32 {
        ffx_pcore::string::bridge::assign(&mut mem(), this, s)
    }
    /// `PClassDesc_Callback3c_00445d60` `(this, object, a, b)` -> the callback's `eax` or `0x15`.
    PClassDesc_Callback3c_00445d60 @ 0x0044_5d60 => extern "thiscall" fn p5_00445d60(this: u32, object: u32, a: u32, b: u32) -> u32 {
        ffx_pcore::small::bridge::callback_3c(&mut mem(), this, object, a, b)
    }
    /// `Phyre_PString_fixup_00445d90` `(this, offset, unused)` -> 0 or 5.
    Phyre_PString_fixup_00445d90 @ 0x0044_5d90 => extern "thiscall" fn p5_00445d90(this: u32, offset: u32, _unused: u32) -> u32 {
        ffx_pcore::string::bridge::fixup(&mut mem(), this, offset)
    }
    /// `PClassDesc_Construct2_0044c2b0` `(p)`: 2 word(s) clear when `p` is set; `eax` `p` with `al = 1`.
    PClassDesc_Construct2_0044c2b0 @ 0x0044_c2b0 => extern "stdcall" fn p5_0044c2b0(p: u32) -> u32 {
        ffx_pcore::small::bridge::clear_words(&mut mem(), p, 2)
    }
    /// `PClassDesc_Construct2_0044c2d0` `(p)`: 2 word(s) clear when `p` is set; `eax` `p` with `al = 1`.
    PClassDesc_Construct2_0044c2d0 @ 0x0044_c2d0 => extern "stdcall" fn p5_0044c2d0(p: u32) -> u32 {
        ffx_pcore::small::bridge::clear_words(&mut mem(), p, 2)
    }
    /// `PClassDesc_DestroyForward_0044c2f0` `(p)` -> `al`.
    PClassDesc_DestroyForward_0044c2f0 @ 0x0044_c2f0 => extern "stdcall" fn p5_0044c2f0(p: u32) -> u8 {
        ffx_pcore::small::bridge::forward(&mut mem(), ffx_pcore::small::DESTROY_44AF00, p, false)
    }
    /// `PClassDesc_DestroyForward_0044c310` `(p)` -> `al`.
    PClassDesc_DestroyForward_0044c310 @ 0x0044_c310 => extern "stdcall" fn p5_0044c310(p: u32) -> u8 {
        ffx_pcore::small::bridge::forward(&mut mem(), ffx_pcore::small::DESTROY_44AF00, p, false)
    }
    /// `Phyre_PThreadPool_PThreadPool` `(this, a, b)` -> `this`.
    Phyre_PThreadPool_PThreadPool @ 0x0044_cbe0 => extern "thiscall" fn p5_0044cbe0(this: u32, a: u32, b: u32) -> u32 {
        ffx_pcore::sync::bridge::thread_pool(&mut mem(), this, a, b)
    }
    /// `Phyre_PThreadPool_GetThreadCount` `()` -> the thread count.
    Phyre_PThreadPool_GetThreadCount @ 0x0044_cde0 => extern "C" fn p5_0044cde0() -> u32 {
        ffx_pcore::sync::bridge::thread_count(&mut mem())
    }
    /// `Phyre_PThreadPool_SetThreadCount_0044ce10` `(n)` -> `n`.
    Phyre_PThreadPool_SetThreadCount_0044ce10 @ 0x0044_ce10 => extern "C" fn p5_0044ce10(n: u32) -> u32 {
        ffx_pcore::sync::bridge::set_thread_count(&mut mem(), n)
    }
    /// `Phyre_PThreadPool_SetStackSize` `(n)` -> `n`.
    Phyre_PThreadPool_SetStackSize @ 0x0044_ce20 => extern "C" fn p5_0044ce20(n: u32) -> u32 {
        ffx_pcore::sync::bridge::set_stack_size(&mut mem(), n)
    }
    /// `PMatrix4_FromRows_00450d40` `(this, r0, r1, r2, r3)` -> `this`.
    PMatrix4_FromRows_00450d40 @ 0x0045_0d40 => extern "thiscall" fn p5_00450d40(this: u32, r0: u32, r1: u32, r2: u32, r3: u32) -> u32 {
        ffx_pcore::matrix::bridge::from_rows(&mut mem(), this, [r0, r1, r2, r3])
    }
    /// `PMatrix4_FromAxes_00450de0` `(this, a, b, c, d)` -> `this`.
    PMatrix4_FromAxes_00450de0 @ 0x0045_0de0 => extern "thiscall" fn p5_00450de0(this: u32, a: u32, b: u32, c: u32, d: u32) -> u32 {
        ffx_pcore::matrix::bridge::from_axes(&mut mem(), this, [a, b, c, d])
    }
    /// `PMatrix4_FromMatrix3x4_00451080` `(this, m)` -> `this`.
    PMatrix4_FromMatrix3x4_00451080 @ 0x0045_1080 => extern "thiscall" fn p5_00451080(this: u32, m: u32) -> u32 {
        ffx_pcore::matrix::bridge::from_3x4(&mut mem(), this, m)
    }
    /// `PMatrix4_toWorldMatrix3x4` `(dst, m)` -> `dst`.
    PMatrix4_toWorldMatrix3x4 @ 0x0045_1190 => extern "thiscall" fn p5_00451190(this: u32, m: u32) -> u32 {
        ffx_pcore::matrix::bridge::to_world_3x4(&mut mem(), this, m)
    }
    /// `PMatrix4_Identity_00451bb0` `(out)` -> `out`.
    PMatrix4_Identity_00451bb0 @ 0x0045_1bb0 => extern "C" fn p5_00451bb0(out: u32) -> u32 {
        ffx_pcore::matrix::bridge::identity(&mut mem(), out)
    }
    /// `PClassDesc_ConstructWorldMatrix_00451fe0` `(p)` -> `al` (vtable +0x28: its dispatchers test `al`).
    PClassDesc_ConstructWorldMatrix_00451fe0 @ 0x0045_1fe0 => extern "stdcall" fn p5_00451fe0(p: u32) -> u8 {
        let mut m = StackFrame::<16>::new();
        ffx_pcore::matrix::bridge::world_identity(&mut mem(), m.at(), p)
    }
    /// `PClassDesc_Return1_00452080` `(p)` -> `al = 1` (the rest of `eax` is the caller's; the vtable +0x2c dispatchers test `al` or drop it).
    PClassDesc_Return1_00452080 @ 0x0045_2080 => extern "stdcall" fn p5_00452080(_p: u32) -> u8 {
        1
    }
    /// `PClassDesc_Return0_00452090` `(p)` -> 0.
    PClassDesc_Return0_00452090 @ 0x0045_2090 => extern "stdcall" fn p5_00452090(_p: u32) -> u32 {
        0
    }
    /// `PObject_InitDefaults_004530a0` `(this)` -> `this`.
    PObject_InitDefaults_004530a0 @ 0x0045_30a0 => extern "thiscall" fn p5_004530a0(this: u32) -> u32 {
        let mut m = StackFrame::<16>::new();
        ffx_pcore::matrix::bridge::init_4530a0(&mut mem(), m.at(), this)
    }
    /// `PMatrix4_Copy_00453340` `(this, m)` -> `this`.
    PMatrix4_Copy_00453340 @ 0x0045_3340 => extern "thiscall" fn p5_00453340(this: u32, m: u32) -> u32 {
        ffx_pcore::matrix::bridge::copy(&mut mem(), this, m, false)
    }
    /// `PClassDesc_Construct2_0049bb00` `(p)`: 2 word(s) clear when `p` is set; `eax` `p` with `al = 1`.
    PClassDesc_Construct2_0049bb00 @ 0x0049_bb00 => extern "stdcall" fn p5_0049bb00(p: u32) -> u32 {
        ffx_pcore::small::bridge::clear_words(&mut mem(), p, 2)
    }
    /// `PClassDesc_Construct6_0049bb20` `(p)`.
    PClassDesc_Construct6_0049bb20 @ 0x0049_bb20 => extern "stdcall" fn p5_0049bb20(p: u32) -> u32 {
        ffx_pcore::small::bridge::clear_49bb20(&mut mem(), p)
    }
    /// `PClassDesc_DestroyArray_0049bb60` `(p)` -> `al` (the class descriptor dispatchers test `al` or drop it).
    PClassDesc_DestroyArray_0049bb60 @ 0x0049_bb60 => extern "stdcall" fn p5_0049bb60(p: u32) -> u8 {
        ffx_pcore::small::bridge::free_array(&mut mem(), p, true)
    }
    /// `PClassDesc_DestroyForward_0049bba0` `(p)` -> `al`.
    PClassDesc_DestroyForward_0049bba0 @ 0x0049_bba0 => extern "stdcall" fn p5_0049bba0(p: u32) -> u8 {
        ffx_pcore::small::bridge::forward(&mut mem(), ffx_pcore::small::DESTROY_499DC0, p, false)
    }
    /// `Phyre_PFreeList_PFreeList` `(this, size, count, align, d)` -> `this`.
    Phyre_PFreeList_PFreeList @ 0x0049_c480 => extern "thiscall" fn p5_0049c480(this: u32, size: u32, count: u32, align: u32, d: u32) -> u32 {
        ffx_pcore::freelist::bridge::new(&mut mem(), this, size, count, align, d)
    }
    /// `Phyre_PFreeList_dtor_PFreeList` `(this)`: frees the blocks.
    Phyre_PFreeList_dtor_PFreeList @ 0x0049_c5b0 => extern "thiscall" fn p5_0049c5b0(this: u32) {
        ffx_pcore::freelist::bridge::free_blocks(&mut mem(), this);
    }
    /// `Phyre_PFreeList_addItemToFreeList` `(item, head)` -> the old head (no `this`: `ret 8`).
    Phyre_PFreeList_addItemToFreeList @ 0x0049_c7a0 => extern "stdcall" fn p5_0049c7a0(item: u32, head: u32) -> u32 {
        ffx_pcore::freelist::bridge::add_item(&mut mem(), item, head)
    }
    /// `Phyre_PFreeList_linkItems_0049c7c0` `(this, first, n)` -> 0, or 1 for a null `first`.
    Phyre_PFreeList_linkItems_0049c7c0 @ 0x0049_c7c0 => extern "thiscall" fn p5_0049c7c0(this: u32, first: u32, n: u32) -> u32 {
        ffx_pcore::freelist::bridge::link_items(&mut mem(), this, first, n)
    }
    /// `Phyre_PFreeList_allocateNewBlock` `(this, n)` -> `al` (its 72 call sites never read `eax`).
    Phyre_PFreeList_allocateNewBlock @ 0x0049_c8d0 => extern "thiscall" fn p5_0049c8d0(this: u32, n: u32) -> u8 {
        ffx_pcore::freelist::bridge::allocate_block(&mut mem(), this, n)
    }
    /// `Phyre_PFreeList_deallocate` `(this, item)` -> the old head.
    Phyre_PFreeList_deallocate @ 0x0049_c980 => extern "thiscall" fn p5_0049c980(this: u32, item: u32) -> u32 {
        ffx_pcore::freelist::bridge::deallocate(&mut mem(), this, item)
    }
    /// `Phyre_PFreeList_reset` `(this)`: frees the blocks.
    Phyre_PFreeList_reset @ 0x0049_cc10 => extern "thiscall" fn p5_0049cc10(this: u32) {
        ffx_pcore::freelist::bridge::free_blocks(&mut mem(), this);
    }
    /// `PMatrix4_SetAffine_004c9360` `(this, m)` -> `this`.
    PMatrix4_SetAffine_004c9360 @ 0x004c_9360 => extern "thiscall" fn p5_004c9360(this: u32, m: u32) -> u32 {
        ffx_pcore::matrix::bridge::affine(&mut mem(), this, m)
    }
    /// `PMatrix4_Copy_004c9430` `(this, m)` -> `this`.
    PMatrix4_Copy_004c9430 @ 0x004c_9430 => extern "thiscall" fn p5_004c9430(this: u32, m: u32) -> u32 {
        ffx_pcore::matrix::bridge::copy(&mut mem(), this, m, true)
    }
    /// `PMatrix4_TimesRows_004c9bc0` `(m, out, b)` -> `out`.
    PMatrix4_TimesRows_004c9bc0 @ 0x004c_9bc0 => extern "thiscall" fn p5_004c9bc0(m: u32, out: u32, b: u32) -> u32 {
        let mut rows = StackFrame::<16>::new();
        ffx_pcore::matrix::bridge::times_rows(&mut mem(), rows.at(), m, out, b)
    }
    /// `PMatrix4_RowTimes_004c9c20` `(m, out, v)` -> `out`.
    PMatrix4_RowTimes_004c9c20 @ 0x004c_9c20 => extern "thiscall" fn p5_004c9c20(m: u32, out: u32, v: u32) -> u32 {
        ffx_pcore::matrix::bridge::row_times_vector(&mut mem(), m, out, v)
    }
    /// `PMatrix4_TransformPoint_004c9cb0` `(m, out, p)` -> `out`.
    PMatrix4_TransformPoint_004c9cb0 @ 0x004c_9cb0 => extern "thiscall" fn p5_004c9cb0(m: u32, out: u32, p: u32) -> u32 {
        ffx_pcore::matrix::bridge::transform_point(&mut mem(), m, out, p)
    }
    /// `PMatrix4_MulThis_004c9da0` `(a, out, b)` -> `out`.
    PMatrix4_MulThis_004c9da0 @ 0x004c_9da0 => extern "thiscall" fn p5_004c9da0(a: u32, out: u32, b: u32) -> u32 {
        ffx_pcore::matrix::bridge::mul_affine(&mut mem(), a, out, b)
    }
    /// `PMatrix4_TransformPoint3_004c9eb0` `(m, out, p)` -> `out`.
    PMatrix4_TransformPoint3_004c9eb0 @ 0x004c_9eb0 => extern "thiscall" fn p5_004c9eb0(m: u32, out: u32, p: u32) -> u32 {
        ffx_pcore::matrix::bridge::transform3(&mut mem(), m, out, p)
    }
    /// `PMatrix4_Rotate3_004ca140` `(m, out, v)` -> `out`.
    PMatrix4_Rotate3_004ca140 @ 0x004c_a140 => extern "thiscall" fn p5_004ca140(m: u32, out: u32, v: u32) -> u32 {
        ffx_pcore::matrix::bridge::rotate3(&mut mem(), m, out, v)
    }
    /// `PMatrix4_Multiply_004ca360` `(out, a, b)` -> `out`.
    PMatrix4_Multiply_004ca360 @ 0x004c_a360 => extern "C" fn p5_004ca360(out: u32, a: u32, b: u32) -> u32 {
        ffx_pcore::matrix::bridge::multiply(&mut mem(), out, a, b)
    }
    /// `PObject_MakeInto_004cc030` `(out, x)` -> 0 or `0x13`.
    PObject_MakeInto_004cc030 @ 0x004c_c030 => extern "C" fn p5_004cc030(out: u32, x: u32) -> u32 {
        ffx_pcore::array::bridge::make_into(&mut mem(), ffx_pcore::array::MAKE_4CB090, out, x)
    }
    /// `PClassDesc_Construct2_004cc1d0` `(p)`: 2 word(s) clear when `p` is set; `eax` `p` with `al = 1`.
    PClassDesc_Construct2_004cc1d0 @ 0x004c_c1d0 => extern "stdcall" fn p5_004cc1d0(p: u32) -> u32 {
        ffx_pcore::small::bridge::clear_words(&mut mem(), p, 2)
    }
    /// `PClassDesc_Construct_004cc240` `(p)`.
    PClassDesc_Construct_004cc240 @ 0x004c_c240 => extern "stdcall" fn p5_004cc240(p: u32) -> u32 {
        ffx_pcore::small::bridge::init_4cc240(&mut mem(), p)
    }
    /// `PClassDesc_DestroyArray_004cc2a0` `(p)` -> `al` (the class descriptor dispatchers test `al` or drop it).
    PClassDesc_DestroyArray_004cc2a0 @ 0x004c_c2a0 => extern "stdcall" fn p5_004cc2a0(p: u32) -> u8 {
        ffx_pcore::small::bridge::free_array(&mut mem(), p, true)
    }
    /// `PClassDesc_Return1_004cc360` `(p)` -> `al = 1` (the rest of `eax` is the caller's; the vtable +0x2c dispatchers test `al` or drop it).
    PClassDesc_Return1_004cc360 @ 0x004c_c360 => extern "stdcall" fn p5_004cc360(_p: u32) -> u8 {
        1
    }
    /// `PMatrix4_RotationZ_004cd1f0` `(out, angle)` -> `out`.
    PMatrix4_RotationZ_004cd1f0 @ 0x004c_d1f0 => extern "C" fn p5_004cd1f0(out: u32, angle: u32) -> u32 {
        ffx_pcore::matrix::bridge::rotation_z(&mut mem(), out, angle)
    }
    /// `PMatrix4_Scale_004cd270` `(out, v)` -> `out`.
    PMatrix4_Scale_004cd270 @ 0x004c_d270 => extern "C" fn p5_004cd270(out: u32, v: u32) -> u32 {
        ffx_pcore::matrix::bridge::scale(&mut mem(), out, v)
    }
    /// `PObject_MakeInto_004d3d10` `(out, x)` -> 0 or `0x13`.
    PObject_MakeInto_004d3d10 @ 0x004d_3d10 => extern "C" fn p5_004d3d10(out: u32, x: u32) -> u32 {
        ffx_pcore::array::bridge::make_into(&mut mem(), ffx_pcore::array::MAKE_4D3830, out, x)
    }
    /// `PClassDesc_Construct5_004d3eb0` `(p)`.
    PClassDesc_Construct5_004d3eb0 @ 0x004d_3eb0 => extern "stdcall" fn p5_004d3eb0(p: u32) -> u32 {
        ffx_pcore::small::bridge::clear_4d3eb0(&mut mem(), p)
    }
    /// `PClassDesc_Return1_004d3ee0` `(p)` -> `al = 1` (the rest of `eax` is the caller's; the vtable +0x2c dispatchers test `al` or drop it).
    PClassDesc_Return1_004d3ee0 @ 0x004d_3ee0 => extern "stdcall" fn p5_004d3ee0(_p: u32) -> u8 {
        1
    }
    /// `PArray_Data_004d45a0` `(array)` -> the elements.
    PArray_Data_004d45a0 @ 0x004d_45a0 => extern "thiscall" fn p5_004d45a0(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PBitArray_setBits_004d55d0` `(bits, value, bit, count)` (its `eax`, the last shift, is unread).
    PBitArray_setBits_004d55d0 @ 0x004d_55d0 => extern "thiscall" fn p5_004d55d0(this: u32, value: u32, bit: u32, count: u32) {
        ffx_pcore::array::bridge::set_bits(&mut mem(), this, value, bit, count);
    }
    /// `PSharray_compareWords` `(array, words, count)` -> -1, 0 or 1.
    PSharray_compareWords @ 0x004e_ea10 => extern "thiscall" fn p5_004eea10(this: u32, words: u32, count: u32) -> u32 {
        ffx_pcore::array::bridge::compare_words(&mut mem(), this, words, count)
    }
    /// `PClassDesc_Construct2_004eea90` `(p)`: 2 word(s) clear when `p` is set; `eax` `p` with `al = 1`.
    PClassDesc_Construct2_004eea90 @ 0x004e_ea90 => extern "stdcall" fn p5_004eea90(p: u32) -> u32 {
        ffx_pcore::small::bridge::clear_words(&mut mem(), p, 2)
    }
    /// `PClassDesc_Construct2_004eeab0` `(p)`: 2 word(s) clear when `p` is set; `eax` `p` with `al = 1`.
    PClassDesc_Construct2_004eeab0 @ 0x004e_eab0 => extern "stdcall" fn p5_004eeab0(p: u32) -> u32 {
        ffx_pcore::small::bridge::clear_words(&mut mem(), p, 2)
    }
    /// `PClassDesc_Construct2_004eead0` `(p)`: 2 word(s) clear when `p` is set; `eax` `p` with `al = 1`.
    PClassDesc_Construct2_004eead0 @ 0x004e_ead0 => extern "stdcall" fn p5_004eead0(p: u32) -> u32 {
        ffx_pcore::small::bridge::clear_words(&mut mem(), p, 2)
    }
    /// `PClassDesc_Construct2_004eeaf0` `(p)`: 2 word(s) clear when `p` is set; `eax` `p` with `al = 1`.
    PClassDesc_Construct2_004eeaf0 @ 0x004e_eaf0 => extern "stdcall" fn p5_004eeaf0(p: u32) -> u32 {
        ffx_pcore::small::bridge::clear_words(&mut mem(), p, 2)
    }
    /// `PClassDesc_Construct2_004eeb10` `(p)`: 2 word(s) clear when `p` is set; `eax` `p` with `al = 1`.
    PClassDesc_Construct2_004eeb10 @ 0x004e_eb10 => extern "stdcall" fn p5_004eeb10(p: u32) -> u32 {
        ffx_pcore::small::bridge::clear_words(&mut mem(), p, 2)
    }
    /// `PClassDesc_ConstructForward_004eeb30` `(p)` -> `al`.
    PClassDesc_ConstructForward_004eeb30 @ 0x004e_eb30 => extern "stdcall" fn p5_004eeb30(p: u32) -> u8 {
        ffx_pcore::small::bridge::forward(&mut mem(), ffx_pcore::small::DESTROY_4E9F70, p, true)
    }
    /// `PClassDesc_DestroyArray_004eeb50` `(p)` -> `al` (the class descriptor dispatchers test `al` or drop it).
    PClassDesc_DestroyArray_004eeb50 @ 0x004e_eb50 => extern "stdcall" fn p5_004eeb50(p: u32) -> u8 {
        ffx_pcore::small::bridge::free_array(&mut mem(), p, false)
    }
    /// `PClassDesc_DestroyArray_004eeb90` `(p)` -> `al` (the class descriptor dispatchers test `al` or drop it).
    PClassDesc_DestroyArray_004eeb90 @ 0x004e_eb90 => extern "stdcall" fn p5_004eeb90(p: u32) -> u8 {
        ffx_pcore::small::bridge::free_array(&mut mem(), p, false)
    }
    /// `PClassDesc_DestroyArray_004eebd0` `(p)` -> `al` (the class descriptor dispatchers test `al` or drop it).
    PClassDesc_DestroyArray_004eebd0 @ 0x004e_ebd0 => extern "stdcall" fn p5_004eebd0(p: u32) -> u8 {
        ffx_pcore::small::bridge::free_array(&mut mem(), p, false)
    }
    /// `PClassDesc_DestroyArray_004eec10` `(p)` -> `al` (the class descriptor dispatchers test `al` or drop it).
    PClassDesc_DestroyArray_004eec10 @ 0x004e_ec10 => extern "stdcall" fn p5_004eec10(p: u32) -> u8 {
        ffx_pcore::small::bridge::free_array(&mut mem(), p, false)
    }
    /// `PClassDesc_DestroyArrayElements_004eec50` `(p)` -> `al`.
    PClassDesc_DestroyArrayElements_004eec50 @ 0x004e_ec50 => extern "stdcall" fn p5_004eec50(p: u32) -> u8 {
        ffx_pcore::small::bridge::destroy_array(&mut mem(), p)
    }
    /// `PClassDesc_DestroyForward_004eec90` `(p)` -> `al`.
    PClassDesc_DestroyForward_004eec90 @ 0x004e_ec90 => extern "stdcall" fn p5_004eec90(p: u32) -> u8 {
        ffx_pcore::small::bridge::forward(&mut mem(), ffx_pcore::small::DESTROY_4EA2D0, p, false)
    }
    /// `PClassDesc_Return0_004eed00` `(p)` -> 0.
    PClassDesc_Return0_004eed00 @ 0x004e_ed00 => extern "stdcall" fn p5_004eed00(_p: u32) -> u32 {
        0
    }
    /// `PClassDesc_Callback38_004eee60` `(this, i)` -> the callback's `eax` or the table word.
    PClassDesc_Callback38_004eee60 @ 0x004e_ee60 => extern "thiscall" fn p5_004eee60(this: u32, i: u32) -> u32 {
        ffx_pcore::small::bridge::callback_38(&mut mem(), this, i)
    }
    /// `PClassDesc_Callback38_004eeec0` `(this, i)` -> the callback's `eax` or the table word.
    PClassDesc_Callback38_004eeec0 @ 0x004e_eec0 => extern "thiscall" fn p5_004eeec0(this: u32, i: u32) -> u32 {
        ffx_pcore::small::bridge::callback_38(&mut mem(), this, i)
    }
    /// `PArray_assign_004f9020` `(array, src, n)` -> 0 or 5.
    PArray_assign_004f9020 @ 0x004f_9020 => extern "thiscall" fn p5_004f9020(this: u32, src: u32, n: u32) -> u32 {
        ffx_pcore::array::bridge::assign(&mut mem(), this, src, n)
    }
    /// `PArray_borrow_004f90b0` `(array, elements, n)` -> 0.
    PArray_borrow_004f90b0 @ 0x004f_90b0 => extern "thiscall" fn p5_004f90b0(this: u32, elements: u32, n: u32) -> u32 {
        ffx_pcore::array::bridge::borrow(&mut mem(), this, elements, n)
    }
    /// `PArray_assignOrStore_004f9100` `(array, src, n)` -> 0.
    PArray_assignOrStore_004f9100 @ 0x004f_9100 => extern "thiscall" fn p5_004f9100(this: u32, src: u32, n: u32) -> u32 {
        ffx_pcore::array::bridge::assign_or_store(&mut mem(), this, src, n)
    }
    /// `PClassDesc_Callback3c_004f9150` `(this, object, a, b)` -> the callback's `eax` or `0x15`.
    PClassDesc_Callback3c_004f9150 @ 0x004f_9150 => extern "thiscall" fn p5_004f9150(this: u32, object: u32, a: u32, b: u32) -> u32 {
        ffx_pcore::small::bridge::callback_3c(&mut mem(), this, object, a, b)
    }
    /// `PClassDesc_Callback3c_004f9180` `(this, object, a, b)` -> the callback's `eax` or `0x15`.
    PClassDesc_Callback3c_004f9180 @ 0x004f_9180 => extern "thiscall" fn p5_004f9180(this: u32, object: u32, a: u32, b: u32) -> u32 {
        ffx_pcore::small::bridge::callback_3c(&mut mem(), this, object, a, b)
    }
    /// `PThreadPool_forward_0057cfe0` `(this, object)` -> the call's `eax`.
    PThreadPool_forward_0057cfe0 @ 0x0057_cfe0 => extern "thiscall" fn p5_0057cfe0(this: u32, object: u32) -> u32 {
        ffx_pcore::array::bridge::forward_57cfe0(&mut mem(), this, object)
    }
    /// `PThreadPool_allocStack_0057d000` `(this)` -> 0 or `0xd`.
    PThreadPool_allocStack_0057d000 @ 0x0057_d000 => extern "thiscall" fn p5_0057d000(this: u32) -> u32 {
        ffx_pcore::array::bridge::alloc_stack(&mut mem(), this)
    }
    /// `Phyre_PPreprocessRingBuffer_allocate` `(ring, size)` -> the block.
    Phyre_PPreprocessRingBuffer_allocate @ 0x005c_d0f0 => extern "thiscall" fn p5_005cd0f0(this: u32, size: u32) -> u32 {
        ffx_pcore::small::bridge::ring_allocate(&mut mem(), this, size)
    }
    /// `PRingBuffer_fits_005cd340` `(ring, n)` -> `al` over the word `+0x28`.
    PRingBuffer_fits_005cd340 @ 0x005c_d340 => extern "thiscall" fn p5_005cd340(this: u32, n: u32) -> u32 {
        ffx_pcore::small::bridge::ring_fits(&mut mem(), this, n)
    }
    /// `PClassDesc_Construct2_00a34270` `(p)`: 2 word(s) clear when `p` is set; `eax` `p` with `al = 1`.
    PClassDesc_Construct2_00a34270 @ 0x00a3_4270 => extern "stdcall" fn p5_00a34270(p: u32) -> u32 {
        ffx_pcore::small::bridge::clear_words(&mut mem(), p, 2)
    }
    /// `PClassDesc_Construct3_00a34290` `(p)`: 3 word(s) clear when `p` is set; `eax` `p` with `al = 1`.
    PClassDesc_Construct3_00a34290 @ 0x00a3_4290 => extern "stdcall" fn p5_00a34290(p: u32) -> u32 {
        ffx_pcore::small::bridge::clear_words(&mut mem(), p, 3)
    }
    /// `PClassDesc_DestroyArray_00a342c0` `(p)` -> `al` (the class descriptor dispatchers test `al` or drop it).
    PClassDesc_DestroyArray_00a342c0 @ 0x00a3_42c0 => extern "stdcall" fn p5_00a342c0(p: u32) -> u8 {
        ffx_pcore::small::bridge::free_array(&mut mem(), p, false)
    }
    /// `PClassDesc_DestroyArray_00a34300` `(p)` -> `al` (the class descriptor dispatchers test `al` or drop it).
    PClassDesc_DestroyArray_00a34300 @ 0x00a3_4300 => extern "stdcall" fn p5_00a34300(p: u32) -> u8 {
        ffx_pcore::small::bridge::free_array(&mut mem(), p, false)
    }
    /// `ComRelease_Slot_00a34910` `(slot)`.
    ComRelease_Slot_00a34910 @ 0x00a3_4910 => extern "C" fn p5_00a34910(slot: u32) {
        ffx_pcore::small::bridge::release(&mut mem(), slot);
    }
    /// `Pair_Clear_00a34a30` `(this)` -> `this`.
    Pair_Clear_00a34a30 @ 0x00a3_4a30 => extern "thiscall" fn p5_00a34a30(this: u32) -> u32 {
        ffx_pcore::small::bridge::clear_pair(&mut mem(), this)
    }
    /// `Config_InitDefaults_00a34a60` `(this)` -> `this`.
    Config_InitDefaults_00a34a60 @ 0x00a3_4a60 => extern "thiscall" fn p5_00a34a60(this: u32) -> u32 {
        ffx_pcore::small::bridge::init_a34a60(&mut mem(), this)
    }
    /// `Call_00a3baf0_00a34ae0` `(p)` -> the call's result.
    Call_00a3baf0_00a34ae0 @ 0x00a3_4ae0 => extern "C" fn p5_00a34ae0(p: u32) -> u32 {
        ffx_pcore::small::bridge::call_a34ae0(&mut mem(), p)
    }
    /// `State_InitBuffers_00a34b10` `(n)` -> the state pointer, `al` whether both buffers exist.
    State_InitBuffers_00a34b10 @ 0x00a3_4b10 => extern "C" fn p5_00a34b10(n: u32) -> u32 {
        ffx_pcore::small::bridge::init_a34b10(&mut mem(), n)
    }
    // ---- round 4 (p5_lights) ----
    /// `Phyre_packInstanceLights_004d4140` `(masks, segments, instance, &words, out, _, mode)` -> 0; the pairs (the original's `_alloca`, 8 bytes a light) in an `AllocaFrame`, the bit writer's word in this frame.
    Phyre_packInstanceLights_004d4140 @ 0x004d_4140 => extern "stdcall" fn p5_004d4140(masks: u32, segments: u32, instance: u32, words: u32, out: u32, unused: u32, mode: u32) -> u32 {
        use ffx_pcore::p5_lights::bridge as lights;
        let mut m = mem();
        let len = lights::pairs_len(&mut m, instance);
        let mut pairs = super::AllocaFrame::new();
        let at = pairs.alloc((len & !3).wrapping_add(8));
        let mut cursor = StackFrame::<1>::new();
        lights::pack_lights(&mut m, [masks, segments, instance, words, out, unused, mode], at, cursor.at())
    }
}
