//! M5 `system-b`: `AsyncLoadManager`, the loader thread that runs cluster
//! loads and registered tasks off the main thread.
//!
//! The manager (`g_AsyncLoadManager`): `+0` the vtable, `+4` the thread,
//! `+8` the "work" semaphore, `+0xc` the "done" semaphore, `+0x10` the
//! mutex, `+0x14` running, `+0x18` the [`LoadClusterRequest`] pool
//! (`g_AsyncLoadClusterPool`), `+0x1c` the [`RegistTaskRequest`] pool
//! (`g_AsyncRegistTaskPool`), `+0x20` two request lists of 12 bytes (the
//! one at `+0x38` takes new requests while the thread runs the other),
//! `+0x3c` the children lists (a list of lists).
//!
//! A list (`{count, capacity, data}`, words) grows to
//! `count + count / 8 + 2` (signed) when a push passes its capacity. A pool: `+0` the
//! vtable, `+4` the requests, `+8` the used bytes, `+0xc` the capacity,
//! `+0x10` the first free index, `+0x14` its mutex.
//!
//! [`LoadClusterRequest`]: the 0x94-byte request (`+4` the name, inline at
//! `+0x10` below 0x84 bytes, `+8` a word, `+0xc` the argument words,
//! inline after the name while both fit).
//! [`RegistTaskRequest`]: the 0x20-byte request (`+4` the task, `+8` the
//! argument words, inline at `+0xc` up to 5 words).

#![allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]

/// `g_AsyncLoadManager`.
pub const MANAGER: u32 = 0x00cc_c88c;
/// `g_AsyncLoadClusterPool`, `g_AsyncRegistTaskPool`: the manager's two
/// pools.
pub const CLUSTER_POOL: u32 = 0x00cc_c890;
pub const TASK_POOL: u32 = 0x00cc_c894;
/// The vtables.
pub const MANAGER_VFTABLE: u32 = 0x00b4_6fa4;
pub const CLUSTER_POOL_VFTABLE: u32 = 0x00b4_6fac;
pub const TASK_POOL_VFTABLE: u32 = 0x00b4_6fb4;
pub const CLUSTER_REQUEST_VFTABLE: u32 = 0x00b4_6f84;
pub const TASK_REQUEST_VFTABLE: u32 = 0x00b4_6f94;
pub const REQUEST_VFTABLE: u32 = 0x00b4_6f74;
/// The thread's entry, `AsyncLoadManager_threadMain_00691360`.
pub const THREAD_MAIN: u32 = 0x0069_1360;
/// Its name, `"AsyncLoadManager"`.
pub const THREAD_NAME: u32 = 0x00b4_6fb8;
/// The pool-full message (`fprintf(stderr, ..., capacity)`).
pub const POOL_FULL: u32 = 0x00b4_6fcc;
/// A request list's constructor and destructor (`0x006906b0`, `0x00690a30`;
/// the vector iterators' callbacks).
pub const LIST_CTOR: u32 = 0x0069_06b0;
pub const LIST_DTOR: u32 = 0x0069_0a30;

/// `operator_new`, `operator_delete`, `operator new[]`, `operator
/// delete[]`, `user_malloc`, `user_free_thunk` (cdecl).
pub const OPERATOR_NEW: u32 = 0x0063_0310;
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
pub const NEW_ARRAY: u32 = 0x0063_04c0;
pub const DELETE_ARRAY: u32 = 0x0063_0540;
pub const USER_MALLOC: u32 = 0x0068_7020;
pub const USER_FREE: u32 = 0x0068_7010;
/// `memcpy` (`0x00949256`, `jmp [MSVCR110 memcpy]`) and `crt_memset_thunk`.
pub const MEMCPY: u32 = 0x0094_9256;
pub const MEMSET: u32 = 0x0094_964c;
/// `eh_vector_constructor_iterator` / `eh_vector_destructor_iterator`
/// (stdcall, 5 and 4 words).
pub const VECTOR_CTOR: u32 = 0x0094_966a;
pub const VECTOR_DTOR: u32 = 0x0094_96d2;
/// The PS2 kernel stand-ins (cdecl): `CreateSema_006f2b10(count)`,
/// `CreateThread_ps2(param, name)`, `StartThread_ps2(thread, arg)`,
/// `ChangeThreadPriority_006f3050(thread, priority)`,
/// `SignalSema(sema)`, `WaitSema(sema)`, `ExitDeleteThread_006f2f00(thread)`.
pub const CREATE_SEMA: u32 = 0x006f_2b10;
pub const CREATE_THREAD: u32 = 0x006f_2e10;
pub const START_THREAD: u32 = 0x006f_2d30;
pub const THREAD_PRIORITY: u32 = 0x006f_3050;
pub const SIGNAL_SEMA: u32 = 0x006f_2ce0;
pub const WAIT_SEMA: u32 = 0x006f_2d80;
pub const EXIT_THREAD: u32 = 0x006f_2f00;
/// Import slots: `KERNEL32` `CreateMutexW`, `CloseHandle`,
/// `WaitForSingleObject`, `ReleaseMutex` (stdcall); `MSVCR110` `memmove`,
/// `__iob_func`, `fprintf` (cdecl).
pub const CREATE_MUTEX_IAT: u32 = 0x00b0_c198;
pub const CLOSE_HANDLE_IAT: u32 = 0x00b0_c0dc;
pub const WAIT_IAT: u32 = 0x00b0_c0f8;
pub const RELEASE_MUTEX_IAT: u32 = 0x00b0_c1c0;
pub const MEMMOVE_IAT: u32 = 0x00b0_c34c;
pub const IOB_FUNC_IAT: u32 = 0x00b0_c350;
pub const FPRINTF_IAT: u32 = 0x00b0_c38c;

/// `createThread`'s thread parameters at `ebp - 0x30` (12 words; the
/// entry at `+4`, the stack size at `+0xc`, the priority at `+0x14`,
/// `+0x20` 0; the others are never written and never read).
pub const THREAD_PARAM: u32 = 0x30;
/// The argument array the variadic requests build (`_alloca`): a count,
/// then one word per argument. The original's loop counter is a byte, so
/// no more than 256 words are ever written.
pub const ARGS_WORDS: u32 = 256;

/// The signed growth of a list past its capacity.
#[must_use]
pub fn grown(count: u32) -> u32 {
    let c = count as i32;
    (c / 8).wrapping_add(c).wrapping_add(2) as u32
}

/// The originals over game memory.
pub mod bridge {
    use super::{
        CLOSE_HANDLE_IAT, CLUSTER_POOL, CLUSTER_POOL_VFTABLE, CLUSTER_REQUEST_VFTABLE,
        CREATE_MUTEX_IAT, CREATE_SEMA, CREATE_THREAD, DELETE_ARRAY, EXIT_THREAD, FPRINTF_IAT,
        IOB_FUNC_IAT, LIST_CTOR, LIST_DTOR, MANAGER, MANAGER_VFTABLE, MEMCPY, MEMMOVE_IAT, MEMSET,
        NEW_ARRAY, OPERATOR_DELETE, OPERATOR_NEW, POOL_FULL, RELEASE_MUTEX_IAT, REQUEST_VFTABLE,
        SIGNAL_SEMA, START_THREAD, TASK_POOL, TASK_POOL_VFTABLE, TASK_REQUEST_VFTABLE, THREAD_MAIN,
        THREAD_NAME, THREAD_PARAM, THREAD_PRIORITY, USER_FREE, USER_MALLOC, VECTOR_CTOR,
        VECTOR_DTOR, WAIT_IAT, WAIT_SEMA, grown,
    };
    use ffx_graphic::host::{Conv, Host};

    /// Calls the import in `slot` (its `ffx.sym` address).
    fn import(h: &mut impl Host, slot: u32, conv: Conv, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, conv, args)
    }

    fn lock(h: &mut impl Host, mutex: u32) {
        import(h, WAIT_IAT, Conv::Stdcall, &[mutex, 0xffff_ffff]);
    }

    fn unlock(h: &mut impl Host, mutex: u32) -> u32 {
        import(h, RELEASE_MUTEX_IAT, Conv::Stdcall, &[mutex])
    }

    fn create_mutex(h: &mut impl Host) -> u32 {
        import(h, CREATE_MUTEX_IAT, Conv::Stdcall, &[0, 0, 0])
    }

    fn close_handle(h: &mut impl Host, handle: u32) -> u32 {
        import(h, CLOSE_HANDLE_IAT, Conv::Stdcall, &[handle])
    }

    /// `0x006917f0` / `0x00691770` (thiscall, ret 4): a list's
    /// `reserve(capacity)`. Without data: `user_malloc(4 * capacity)`.
    /// Else a new block for a positive capacity (the old count copied in
    /// when the allocation worked), the old one freed. Both set the
    /// capacity and the data.
    pub fn list_reserve(h: &mut impl Host, list: u32, capacity: u32) {
        let data: u32 = h.read(list.wrapping_add(8));
        if data == 0 {
            let p = h.call_sym(USER_MALLOC, Conv::Cdecl, &[capacity.wrapping_mul(4)]);
            h.write(list.wrapping_add(4), capacity);
            h.write(list.wrapping_add(8), p);
            return;
        }
        let mut p = 0;
        if (capacity as i32) > 0 {
            p = h.call_sym(USER_MALLOC, Conv::Cdecl, &[capacity.wrapping_mul(4)]);
            if p != 0 {
                let count: u32 = h.read(list);
                h.call_sym(MEMCPY, Conv::Cdecl, &[p, data, count.wrapping_shl(2)]);
            }
        }
        h.call_sym(USER_FREE, Conv::Cdecl, &[data]);
        h.write(list.wrapping_add(4), capacity);
        h.write(list.wrapping_add(8), p);
    }

    /// Appends `value` to the list at `list` (growing it past its
    /// capacity); the slot is written when its address is not 0. Returns
    /// what the original leaves in `eax`: `value`, or the data pointer
    /// when the slot's address is 0.
    fn push(h: &mut impl Host, list: u32, value: u32) -> u32 {
        let count = h.read::<u32>(list).wrapping_add(1);
        h.write(list, count);
        let capacity: u32 = h.read(list.wrapping_add(4));
        if (count as i32) > (capacity as i32) {
            list_reserve(h, list, grown(count));
        }
        let count: u32 = h.read(list);
        let data: u32 = h.read(list.wrapping_add(8));
        let slot = data.wrapping_add(count.wrapping_sub(1).wrapping_mul(4));
        if slot == 0 {
            return data;
        }
        h.write(slot, value);
        value
    }

    /// `0x00690680(this; capacity)` (thiscall, ret 4): an empty list,
    /// reserved when `capacity > 0` (signed); `this`.
    pub fn list_construct(h: &mut impl Host, this: u32, capacity: u32) -> u32 {
        h.write(this, 0u32);
        h.write(this.wrapping_add(4), 0u32);
        h.write(this.wrapping_add(8), 0u32);
        if (capacity as i32) > 0 {
            list_reserve(h, this, capacity);
        }
        this
    }

    /// `RequestPool<LoadClusterRequest>::RequestPool(this; capacity)`
    /// (`0x006906d0`, thiscall, ret 4; without its SEH frame): the vtable,
    /// the capacity, index 0, a mutex, `user_malloc(capacity * 0x94)`
    /// requests and `new[](capacity)` used bytes, cleared; `this`.
    pub fn cluster_pool_construct(h: &mut impl Host, this: u32, capacity: u32) -> u32 {
        pool_construct(h, this, CLUSTER_POOL_VFTABLE, capacity, 0x94);
        this
    }

    fn pool_construct(h: &mut impl Host, this: u32, vftable: u32, capacity: u32, size: u32) {
        h.write(this, h.rebase(vftable));
        h.write(this.wrapping_add(0xc), capacity);
        h.write(this.wrapping_add(0x10), 0u32);
        let mutex = create_mutex(h);
        h.write(this.wrapping_add(0x14), mutex);
        let requests = h.call_sym(USER_MALLOC, Conv::Cdecl, &[capacity.wrapping_mul(size)]);
        h.write(this.wrapping_add(4), requests);
        let used = h.call_sym(NEW_ARRAY, Conv::Cdecl, &[capacity]);
        h.write(this.wrapping_add(8), used);
        h.call_sym(MEMSET, Conv::Cdecl, &[used, 0, capacity]);
    }

    /// `AsyncLoadManager::AsyncLoadManager(this)` (`0x00690830`, thiscall;
    /// without its SEH frame): the vtable, `+4..+0xc` 0, the mutex,
    /// running, the two request lists (the vector iterator), `+0x38..+0x48`
    /// 0, `g_AsyncLoadManager`; a cluster pool of 200 and a task pool of
    /// 150 (`operator new(0x18)` each, 0 when it fails), the two pool
    /// globals, then [`create_thread`] (its frame at `thread_ebp`). Returns
    /// `this`.
    pub fn construct(h: &mut impl Host, thread_ebp: u32, this: u32) -> u32 {
        h.write(this, h.rebase(MANAGER_VFTABLE));
        for off in [4u32, 8, 0xc] {
            h.write(this.wrapping_add(off), 0u32);
        }
        let mutex = create_mutex(h);
        h.write(this.wrapping_add(0x10), mutex);
        h.write(this.wrapping_add(0x14), 1u32);
        let (ctor, dtor) = (h.rebase(LIST_CTOR), h.rebase(LIST_DTOR));
        h.call_sym(
            VECTOR_CTOR,
            Conv::Stdcall,
            &[this.wrapping_add(0x20), 0xc, 2, ctor, dtor],
        );
        for off in [0x38u32, 0x3c, 0x40, 0x44, 0x48] {
            h.write(this.wrapping_add(off), 0u32);
        }
        h.write(h.rebase(MANAGER), this);
        let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x18]);
        let clusters = if p == 0 {
            0
        } else {
            cluster_pool_construct(h, p, 200)
        };
        h.write(this.wrapping_add(0x18), clusters);
        let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x18]);
        if p != 0 {
            pool_construct(h, p, TASK_POOL_VFTABLE, 150, 0x20);
        }
        h.write(this.wrapping_add(0x1c), p);
        let clusters: u32 = h.read(this.wrapping_add(0x18));
        h.write(h.rebase(CLUSTER_POOL), clusters);
        let tasks: u32 = h.read(this.wrapping_add(0x1c));
        h.write(h.rebase(TASK_POOL), tasks);
        create_thread(h, thread_ebp, this);
        this
    }

    /// `AsyncLoadManager::createThread(this)` (`0x00691180`, thiscall;
    /// `ebp` is the original's frame pointer, the parameters at
    /// [`THREAD_PARAM`] below it): the two semaphores (count 1), the
    /// parameters (entry `threadMain`, stack 0x8000, priority 999, `+0x20`
    /// 0), `CreateThread_ps2(param, "AsyncLoadManager")` into `+4`; when it
    /// worked `StartThread_ps2(thread, this + 8)` and priority 1. Callers
    /// read no result.
    pub fn create_thread(h: &mut impl Host, ebp: u32, this: u32) {
        let a = h.call_sym(CREATE_SEMA, Conv::Cdecl, &[1]);
        h.write(this.wrapping_add(8), a);
        let b = h.call_sym(CREATE_SEMA, Conv::Cdecl, &[1]);
        h.write(this.wrapping_add(0xc), b);
        let param = ebp.wrapping_sub(THREAD_PARAM);
        h.write(param.wrapping_add(4), h.rebase(THREAD_MAIN));
        h.write(param.wrapping_add(0x14), 0x3e7u32);
        h.write(param.wrapping_add(0xc), 0x8000u32);
        h.write(param.wrapping_add(0x20), 0u32);
        let name = h.rebase(THREAD_NAME);
        let thread = h.call_sym(CREATE_THREAD, Conv::Cdecl, &[param, name]);
        h.write(this.wrapping_add(4), thread);
        if thread != 0 {
            h.call_sym(START_THREAD, Conv::Cdecl, &[thread, this.wrapping_add(8)]);
            let thread: u32 = h.read(this.wrapping_add(4));
            h.call_sym(THREAD_PRIORITY, Conv::Cdecl, &[thread, 1]);
        }
    }

    /// `AsyncLoadManager::~AsyncLoadManager(this)` (`0x00690ac0`,
    /// thiscall; without its SEH frame): the vtable; the pools' deleting
    /// destructors (slot 0, flag 1) when set; the children's data freed;
    /// the request lists' destructors (the vector iterator);
    /// `CloseHandle(mutex)`, whose result is left in `eax`.
    pub fn destruct(h: &mut impl Host, this: u32) -> u32 {
        h.write(this, h.rebase(MANAGER_VFTABLE));
        for off in [0x18u32, 0x1c] {
            let pool: u32 = h.read(this.wrapping_add(off));
            if pool != 0 {
                h.call_virtual(pool, 0, &[1]);
            }
        }
        let children: u32 = h.read(this.wrapping_add(0x44));
        if children != 0 {
            h.call_sym(USER_FREE, Conv::Cdecl, &[children]);
        }
        let dtor = h.rebase(LIST_DTOR);
        h.call_sym(
            VECTOR_DTOR,
            Conv::Stdcall,
            &[this.wrapping_add(0x20), 0xc, 2, dtor],
        );
        let mutex: u32 = h.read(this.wrapping_add(0x10));
        close_handle(h, mutex)
    }

    /// The manager's deleting destructor `(this; flags)` (`0x00690cc0`):
    /// [`destruct`], `operator delete(this)` when `flags & 1`; `this`.
    pub fn deleting_dtor(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        destruct(h, this);
        delete_if(h, this, flags)
    }

    fn delete_if(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        if flags & 1 != 0 {
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[this]);
        }
        this
    }

    /// A pool's deleting destructor `(this; flags)` (`0x00690c40` for the
    /// cluster pool, `0x00690c80` for the task pool; `vftable` its own):
    /// the vtable, `user_free(requests)`, `delete[](used)`,
    /// `CloseHandle(mutex)`, `operator delete(this)` when `flags & 1`;
    /// `this`.
    pub fn pool_deleting_dtor(h: &mut impl Host, this: u32, flags: u32, vftable: u32) -> u32 {
        let requests: u32 = h.read(this.wrapping_add(4));
        h.write(this, h.rebase(vftable));
        h.call_sym(USER_FREE, Conv::Cdecl, &[requests]);
        let used: u32 = h.read(this.wrapping_add(8));
        h.call_sym(DELETE_ARRAY, Conv::Cdecl, &[used]);
        let mutex: u32 = h.read(this.wrapping_add(0x14));
        close_handle(h, mutex);
        delete_if(h, this, flags)
    }

    /// `LoadClusterRequest`'s deleting destructor `(this; flags)`
    /// (`0x00690cf0`): the name freed (`delete[]`) unless inline at
    /// `+0x10`, the arguments unless inside `+0x10..=+0x94`; the base
    /// vtable; `this`.
    pub fn cluster_request_deleting_dtor(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        let name: u32 = h.read(this.wrapping_add(4));
        let inline = this.wrapping_add(0x10);
        h.write(this, h.rebase(CLUSTER_REQUEST_VFTABLE));
        if name != inline {
            h.call_sym(DELETE_ARRAY, Conv::Cdecl, &[name]);
        }
        let args: u32 = h.read(this.wrapping_add(0xc));
        if args < inline || args > this.wrapping_add(0x94) {
            h.call_sym(DELETE_ARRAY, Conv::Cdecl, &[args]);
        }
        h.write(this, h.rebase(REQUEST_VFTABLE));
        delete_if(h, this, flags)
    }

    /// `RegistTaskRequest`'s deleting destructor `(this; flags)`
    /// (`0x00690d50`): the arguments freed (`delete[]`) unless inline at
    /// `+0xc`; the base vtable; `this`.
    pub fn task_request_deleting_dtor(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        let args: u32 = h.read(this.wrapping_add(8));
        h.write(this, h.rebase(TASK_REQUEST_VFTABLE));
        if args != this.wrapping_add(0xc) {
            h.call_sym(DELETE_ARRAY, Conv::Cdecl, &[args]);
        }
        h.write(this, h.rebase(REQUEST_VFTABLE));
        delete_if(h, this, flags)
    }

    /// `Request`'s deleting destructor `(this; flags)` (`0x00690d90`).
    pub fn request_deleting_dtor(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        h.write(this, h.rebase(REQUEST_VFTABLE));
        delete_if(h, this, flags)
    }

    /// `0x00690e60(this; list, request)` (thiscall, ret 8): pushes
    /// `request` on `list`, or with no list on the manager's current list
    /// under the mutex and signals the work semaphore. Returns the
    /// original's `eax` (see [`push`]; `SignalSema`'s for the manager's
    /// list).
    pub fn queue(h: &mut impl Host, this: u32, list: u32, request: u32) -> u32 {
        if list != 0 {
            return push(h, list, request);
        }
        let mutex: u32 = h.read(this.wrapping_add(0x10));
        lock(h, mutex);
        let cur: u32 = h.read(this.wrapping_add(0x38));
        let list = this.wrapping_add(0x20).wrapping_add(cur.wrapping_mul(0xc));
        push(h, list, request);
        let mutex: u32 = h.read(this.wrapping_add(0x10));
        unlock(h, mutex);
        let sema: u32 = h.read(this.wrapping_add(8));
        h.call_sym(SIGNAL_SEMA, Conv::Cdecl, &[sema])
    }

    /// `AsyncLoadManager::createChildrenRequestList(this; capacity)`
    /// (`0x00690f70`, thiscall, ret 4): a new list (`operator new(0xc)`,
    /// [`list_construct`]; 0 when it fails) pushed on the children;
    /// returns it.
    pub fn create_children_list(h: &mut impl Host, this: u32, capacity: u32) -> u32 {
        let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0xc]);
        let list = if p == 0 {
            0
        } else {
            list_construct(h, p, capacity)
        };
        push(h, this.wrapping_add(0x3c), list);
        list
    }

    /// A pool's `alloc(this)` up to its initialisation, under its mutex
    /// (left locked): the first free request (marked used, the free index
    /// moved to the next unused one); when the pool is full, the message
    /// on `stderr` and an `operator new`ed one (0 when that fails).
    fn pool_take(h: &mut impl Host, this: u32, size: u32) -> u32 {
        let mutex: u32 = h.read(this.wrapping_add(0x14));
        lock(h, mutex);
        let index: u32 = h.read(this.wrapping_add(0x10));
        let capacity: u32 = h.read(this.wrapping_add(0xc));
        if index == capacity {
            let iob = import(h, IOB_FUNC_IAT, Conv::Cdecl, &[]);
            let msg = h.rebase(POOL_FULL);
            import(
                h,
                FPRINTF_IAT,
                Conv::Cdecl,
                &[iob.wrapping_add(0x40), msg, capacity],
            );
            return h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[size]);
        }
        let used: u32 = h.read(this.wrapping_add(8));
        let requests: u32 = h.read(this.wrapping_add(4));
        let p = index.wrapping_mul(size).wrapping_add(requests);
        h.write(index.wrapping_add(used), 1u8);
        let mut i: u32 = h.read(this.wrapping_add(0x10));
        let capacity: u32 = h.read(this.wrapping_add(0xc));
        if i < capacity {
            let used: u32 = h.read(this.wrapping_add(8));
            while h.read::<u8>(used.wrapping_add(i)) != 0 {
                i = i.wrapping_add(1);
                if i >= capacity {
                    break;
                }
            }
        }
        h.write(this.wrapping_add(0x10), i);
        p
    }

    /// `RequestPool<LoadClusterRequest>::alloc(this)` (`0x00690ff0`,
    /// thiscall): see the pools above; the request gets its vtable,
    /// `+4..+0xc` 0 and `+0x10..+0x94` cleared (`memset`).
    pub fn cluster_pool_alloc(h: &mut impl Host, this: u32) -> u32 {
        let p = pool_take(h, this, 0x94);
        if p != 0 {
            h.write(p, h.rebase(CLUSTER_REQUEST_VFTABLE));
            for off in [4u32, 8, 0xc] {
                h.write(p.wrapping_add(off), 0u32);
            }
            h.call_sym(MEMSET, Conv::Cdecl, &[p.wrapping_add(0x10), 0, 0x84]);
        }
        let mutex: u32 = h.read(this.wrapping_add(0x14));
        unlock(h, mutex);
        p
    }

    /// `RequestPool<RegistTaskRequest>::alloc(this)` (`0x006910c0`,
    /// thiscall): the request gets its vtable and `+4..+0x1c` 0.
    pub fn task_pool_alloc(h: &mut impl Host, this: u32) -> u32 {
        let p = pool_take(h, this, 0x20);
        if p != 0 {
            h.write(p, h.rebase(TASK_REQUEST_VFTABLE));
            for off in (4u32..0x20).step_by(4) {
                h.write(p.wrapping_add(off), 0u32);
            }
        }
        let mutex: u32 = h.read(this.wrapping_add(0x14));
        unlock(h, mutex);
        p
    }

    /// A pool's `free(this; request)` for `index` (the request's index,
    /// computed by the caller as the original does): inside the pool, its
    /// destructor (slot 0, flag 0) and, under the mutex, the slot freed
    /// and the free index lowered to it (`ReleaseMutex`'s result); outside
    /// it, a non-null request's deleting destructor (flag 1). `None` when
    /// the request is null and outside.
    fn pool_free(h: &mut impl Host, pool: u32, request: u32, index: u32) -> Option<u32> {
        let capacity: u32 = h.read(pool.wrapping_add(0xc));
        if index >= capacity {
            if request == 0 {
                return None;
            }
            return Some(h.call_virtual(request, 0, &[1]));
        }
        h.call_virtual(request, 0, &[0]);
        let mutex: u32 = h.read(pool.wrapping_add(0x14));
        lock(h, mutex);
        let first: u32 = h.read(pool.wrapping_add(0x10));
        if index < first {
            h.write(pool.wrapping_add(0x10), index);
        }
        let used: u32 = h.read(pool.wrapping_add(8));
        h.write(index.wrapping_add(used), 0u8);
        let mutex: u32 = h.read(pool.wrapping_add(0x14));
        Some(unlock(h, mutex))
    }

    /// `RequestPool<LoadClusterRequest>::free(this; request)`
    /// (`0x006911f0`, thiscall, ret 4): the index is `(request -
    /// requests) / 0x94` (signed); see [`pool_free`]. A null request
    /// outside leaves the low word of the division's multiply in `eax`.
    pub fn cluster_pool_free(h: &mut impl Host, this: u32, request: u32) -> u32 {
        let requests: u32 = h.read(this.wrapping_add(4));
        let diff = request.wrapping_sub(requests);
        let index = ((diff as i32) / 0x94) as u32;
        pool_free(h, this, request, index).unwrap_or(diff.wrapping_mul(0xdd67_c8a7))
    }

    /// `0x006912c0(request)` (`ecx` = the request): [`cluster_pool_free`]
    /// on `g_AsyncLoadClusterPool`.
    pub fn free_cluster_request(h: &mut impl Host, request: u32) -> u32 {
        let pool: u32 = h.read(h.rebase(CLUSTER_POOL));
        cluster_pool_free(h, pool, request)
    }

    /// `0x006912d0(request)` (`ecx` = the request): the task pool's free
    /// on `g_AsyncRegistTaskPool`, the index `(request - requests) >> 5`
    /// (arithmetic). A null request outside leaves `eax` alone (`None`).
    pub fn free_task_request(h: &mut impl Host, request: u32) -> Option<u32> {
        let pool: u32 = h.read(h.rebase(TASK_POOL));
        let requests: u32 = h.read(pool.wrapping_add(4));
        let index = ((request.wrapping_sub(requests) as i32) >> 5) as u32;
        pool_free(h, pool, request, index)
    }

    /// The thread's "clear" of a list: with a negative count and capacity
    /// (signed) the data is freed (or, when null, `user_malloc(0)` taken)
    /// and the capacity cleared; the count becomes 0.
    fn shrink_if_negative(h: &mut impl Host, list: u32) {
        let count: u32 = h.read(list);
        let capacity: u32 = h.read(list.wrapping_add(4));
        if (count as i32) < 0 && (capacity as i32) < 0 {
            let data: u32 = h.read(list.wrapping_add(8));
            if data == 0 {
                let p = h.call_sym(USER_MALLOC, Conv::Cdecl, &[0]);
                h.write(list.wrapping_add(8), p);
            } else {
                h.call_sym(USER_FREE, Conv::Cdecl, &[data]);
                h.write(list.wrapping_add(8), 0u32);
            }
            h.write(list.wrapping_add(4), 0u32);
        }
        h.write(list, 0u32);
    }

    /// Runs a request: slots 1 and 2.
    fn run(h: &mut impl Host, request: u32) {
        h.call_virtual(request, 1, &[]);
        h.call_virtual(request, 2, &[]);
    }

    /// `AsyncLoadManager_threadMain_00691360()` (cdecl, the thread's
    /// entry): while running, swaps the request lists under the mutex
    /// and runs every request of the old one (slots 1 and 2), each
    /// followed by the requests of the first children list, which is then
    /// freed and the children cleared; the list is cleared, and this
    /// repeats while the current list has requests. Then it signals the
    /// "done" semaphore and waits on the "work" one. At the end,
    /// `ExitDeleteThread(thread)`, whose result is returned.
    pub fn thread_main(h: &mut impl Host) -> u32 {
        let g = h.rebase(MANAGER);
        let mut m: u32 = h.read(g);
        if h.read::<u32>(m.wrapping_add(0x14)) != 0 {
            loop {
                m = h.read(g);
                let cur: u32 = h.read(m.wrapping_add(0x38));
                let at = m.wrapping_add(0x20).wrapping_add(cur.wrapping_mul(0xc));
                if h.read::<u32>(at) != 0 {
                    loop {
                        run_list(h, m);
                        let cur: u32 = h.read(m.wrapping_add(0x38));
                        let at = m.wrapping_add(0x20).wrapping_add(cur.wrapping_mul(0xc));
                        if h.read::<u32>(at) == 0 {
                            break;
                        }
                    }
                    m = h.read(g);
                }
                let done: u32 = h.read(m.wrapping_add(0xc));
                h.call_sym(SIGNAL_SEMA, Conv::Cdecl, &[done]);
                let m2: u32 = h.read(g);
                let work: u32 = h.read(m2.wrapping_add(8));
                h.call_sym(WAIT_SEMA, Conv::Cdecl, &[work]);
                let m3: u32 = h.read(g);
                if h.read::<u32>(m3.wrapping_add(0x14)) == 0 {
                    break;
                }
            }
        }
        let m: u32 = h.read(g);
        let thread: u32 = h.read(m.wrapping_add(4));
        h.call_sym(EXIT_THREAD, Conv::Cdecl, &[thread])
    }

    /// One pass of [`thread_main`] over the list it swaps out.
    fn run_list(h: &mut impl Host, m: u32) {
        let mutex: u32 = h.read(m.wrapping_add(0x10));
        lock(h, mutex);
        let old: u32 = h.read(m.wrapping_add(0x38));
        h.write(m.wrapping_add(0x38), old ^ 1);
        let mutex: u32 = h.read(m.wrapping_add(0x10));
        unlock(h, mutex);
        let list = m.wrapping_add(0x20).wrapping_add(old.wrapping_mul(0xc));
        let mut i = 0u32;
        while i < h.read::<u32>(list) {
            let data: u32 = h.read(list.wrapping_add(8));
            let request: u32 = h.read(data.wrapping_add(i.wrapping_mul(4)));
            run(h, request);
            let children = m.wrapping_add(0x3c);
            if h.read::<u32>(children) != 0 {
                let mut j = 0u32;
                loop {
                    let heads: u32 = h.read(m.wrapping_add(0x44));
                    let head: u32 = h.read(heads);
                    if j >= h.read::<u32>(head) {
                        break;
                    }
                    let data: u32 = h.read(head.wrapping_add(8));
                    let child: u32 = h.read(data.wrapping_add(j.wrapping_mul(4)));
                    run(h, child);
                    j = j.wrapping_add(1);
                }
                let heads: u32 = h.read(m.wrapping_add(0x44));
                let head: u32 = h.read(heads);
                if head != 0 {
                    let data: u32 = h.read(head.wrapping_add(8));
                    if data != 0 {
                        h.call_sym(USER_FREE, Conv::Cdecl, &[data]);
                    }
                    h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[head]);
                }
                shrink_if_negative(h, children);
            }
            i = i.wrapping_add(1);
        }
        shrink_if_negative(h, list);
    }

    /// `LoadClusterRequest::init(this; name, word, args)` (`0x00691550`,
    /// thiscall, ret 0xc): `+8` = `word`; the name copied inline at
    /// `+0x10` below 0x84 characters (signed), else into `new[](len +
    /// 1)`; the `args[0]` argument words after `args` copied (`memcpy`)
    /// inline after the name's NUL while name and words stay below 0x84
    /// bytes, else into `new[](4 * args[0])`; `+0xc` = where.
    pub fn cluster_request_init(h: &mut impl Host, this: u32, name: u32, word: u32, args: u32) {
        h.write(this.wrapping_add(8), word);
        let mut len = 0u32;
        while h.read::<u8>(name.wrapping_add(len)) != 0 {
            len = len.wrapping_add(1);
        }
        let dst = if (len as i32) < 0x84 {
            this.wrapping_add(0x10)
        } else {
            h.call_sym(NEW_ARRAY, Conv::Cdecl, &[len.wrapping_add(1)])
        };
        h.write(this.wrapping_add(4), dst);
        let mut k = 0u32;
        loop {
            let c: u8 = h.read(name.wrapping_add(k));
            h.write(dst.wrapping_add(k), c);
            k = k.wrapping_add(1);
            if c == 0 {
                break;
            }
        }
        let n = h.read::<u32>(args).wrapping_shl(2);
        let words = if (n.wrapping_add(len) as i32) < 0x84 {
            this.wrapping_add(len).wrapping_add(0x11)
        } else {
            h.call_sym(NEW_ARRAY, Conv::Cdecl, &[n])
        };
        h.call_sym(MEMCPY, Conv::Cdecl, &[words, args.wrapping_add(4), n]);
        h.write(this.wrapping_add(0xc), words);
    }

    /// Builds the variadic request's argument array at `array`: the
    /// count, then the low word of each 8-byte argument from `varargs` on.
    /// The original counts in a byte: past 254 arguments its index wraps
    /// and the loop never ends, and so does this one.
    fn gather(h: &mut impl Host, array: u32, count: u32, varargs: u32) {
        h.write(array, count);
        if (count as i32) < 1 {
            return;
        }
        let mut i: u8 = 1;
        let mut at = varargs;
        loop {
            let v: u32 = h.read(at);
            at = at.wrapping_add(8);
            h.write(array.wrapping_add(4 * u32::from(i)), v);
            i = i.wrapping_add(1);
            if i32::from(i) > count as i32 {
                break;
            }
        }
    }

    /// `AsyncLoadManager_loadCluster(manager, list, name, word, count,
    /// ...)` (`0x00691660`, cdecl; `varargs` the address of the first
    /// variadic word, `array` room for [`super::ARGS_WORDS`] words): a
    /// request from the cluster pool, [`cluster_request_init`] with the
    /// argument array, then [`queue`] on `list`; its `eax`.
    #[allow(clippy::too_many_arguments)]
    pub fn load_cluster(
        h: &mut impl Host,
        array: u32,
        manager: u32,
        list: u32,
        name: u32,
        word: u32,
        count: u32,
        varargs: u32,
    ) -> u32 {
        gather(h, array, count, varargs);
        let pool: u32 = h.read(manager.wrapping_add(0x18));
        let request = cluster_pool_alloc(h, pool);
        cluster_request_init(h, request, name, word, array);
        queue(h, manager, list, request)
    }

    /// `AsyncLoadManager_registAsyncLoadTask(manager, task, count, ...)`
    /// (`0x006918b0`, cdecl): a request from the task pool, `+4` = `task`,
    /// the argument words (inline at `+0xc` up to 5) at `+8`; pushed on the
    /// current list under the mutex, then `SignalSema(work)`, whose result
    /// is returned.
    pub fn regist_task(
        h: &mut impl Host,
        array: u32,
        manager: u32,
        task: u32,
        count: u32,
        varargs: u32,
    ) -> u32 {
        let pool: u32 = h.read(manager.wrapping_add(0x1c));
        let request = task_pool_alloc(h, pool);
        gather(h, array, count, varargs);
        h.write(request.wrapping_add(4), task);
        let n = h.read::<u32>(array).wrapping_shl(2);
        let words = if (n as i32) > 0x14 {
            h.call_sym(NEW_ARRAY, Conv::Cdecl, &[n])
        } else {
            request.wrapping_add(0xc)
        };
        h.call_sym(MEMCPY, Conv::Cdecl, &[words, array.wrapping_add(4), n]);
        h.write(request.wrapping_add(8), words);
        let mutex: u32 = h.read(manager.wrapping_add(0x10));
        lock(h, mutex);
        let cur: u32 = h.read(manager.wrapping_add(0x38));
        let list = manager
            .wrapping_add(0x20)
            .wrapping_add(cur.wrapping_mul(0xc));
        push(h, list, request);
        let mutex: u32 = h.read(manager.wrapping_add(0x10));
        unlock(h, mutex);
        let sema: u32 = h.read(manager.wrapping_add(8));
        h.call_sym(SIGNAL_SEMA, Conv::Cdecl, &[sema])
    }

    /// `AssetListReader_getPlatformSpecificPath(out, path)` (`0x00690590`,
    /// cdecl; `path` a `std::string` by value in the six words at `arg`,
    /// the caller's argument slots): `out` takes the string over: with a
    /// heap buffer (capacity 16 and up) its pointer, else `memmove` of
    /// `size + 1` bytes (none when that is 0); size and capacity. Returns
    /// `out`. The argument is not destroyed.
    pub fn platform_specific_path(h: &mut impl Host, out: u32, arg: u32) -> u32 {
        h.write(out.wrapping_add(0x14), 0xfu32);
        h.write(out.wrapping_add(0x10), 0u32);
        h.write(out, 0u8);
        let size: u32 = h.read(arg.wrapping_add(0x10));
        let capacity: u32 = h.read(arg.wrapping_add(0x14));
        if capacity >= 0x10 {
            let p: u32 = h.read(arg);
            h.write(out, p);
        } else if size.wrapping_add(1) != 0 {
            import(
                h,
                MEMMOVE_IAT,
                Conv::Cdecl,
                &[out, arg, size.wrapping_add(1)],
            );
        }
        let size: u32 = h.read(arg.wrapping_add(0x10));
        let capacity: u32 = h.read(arg.wrapping_add(0x14));
        h.write(out.wrapping_add(0x10), size);
        h.write(out.wrapping_add(0x14), capacity);
        out
    }
}
