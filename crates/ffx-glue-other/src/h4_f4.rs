//! Continuation round 4 of list `h4-glue-other`, batch 4: the FMOD event
//! system's setup, a sound bank's constructor and its event tree's load.

use ffx_graphic::host::{Conv, Host};

use crate::frames::{ERRCHECK, PRINT_TO_OUTPUT};
use crate::h4_c2::{FREE_LIST_ALLOCATE_BLOCK, FREE_LIST_DEALLOCATE, TREE_POST_INSERT};
use crate::h4_d1::FIOS_EXISTS;
use crate::h4_d4::{FIOS_CLOSE, FIOS_OPEN, FIOS_READ, FIOS_SIZE, MEMCPY};
use crate::h4_e5::OPERATOR_DELETE_ARRAY;
use crate::h4_f1::{FFX_FMOD_FILE, OPERATOR_NEW};
use crate::h4_f3::{MEMSET, SPRINTF_IAT, TOO_MANY};
use crate::misc::OPERATOR_NEW_2;

// ---- the event system

/// `FMOD_Memory_Initialize`'s thunk (stdcall).
pub const MEMORY_INITIALIZE: u32 = 0x0094_8cd6;
/// `FMOD_EventSystem_Create`'s thunk (stdcall).
pub const EVENT_SYSTEM_CREATE: u32 = 0x0094_8c70;
/// `FMOD::EventSystem::getSystemObject`'s thunk (stdcall).
pub const GET_SYSTEM_OBJECT: u32 = 0x0094_8c88;
/// `FMOD::System::setStreamBufferSize`'s thunk (stdcall).
pub const SET_STREAM_BUFFER_SIZE: u32 = 0x0094_8ce8;
/// `FMOD::EventSystem::init`'s thunk (stdcall).
pub const EVENT_SYSTEM_INIT: u32 = 0x0094_8c76;
/// `FMOD::System::setFileSystem`'s thunk (stdcall).
pub const SET_FILE_SYSTEM: u32 = 0x0094_8ce2;
/// `FfxFmod_SetGlobal00c3a480_00708610()`.
pub const CLEAR_GLOBAL: u32 = 0x0070_8610;
/// The memory callbacks: `FmodMemory_alloc`, `FmodMemory_realloc`,
/// `FmodMemory_free`.
pub const MEMORY_CALLBACKS: [u32; 3] = [0x0070_66d0, 0x0070_6730, 0x0070_6710];
/// The file callbacks: `FmodFile_open`, `FmodFile_close`, `FmodFile_read`,
/// `FmodFile_seek`.
pub const FILE_CALLBACKS: [u32; 4] = [0x0070_8510, 0x0070_84d0, 0x0070_85a0, 0x0070_85e0];
/// `"EventSystem create failed!\n"` (`0x00b4d280`).
pub const CREATE_FAILED: u32 = 0x00b4_d280;

fn errcheck(h: &mut impl Host, r: u32, line: u32) -> u32 {
    let file = h.rebase(FFX_FMOD_FILE);
    h.call_sym(ERRCHECK, Conv::Cdecl, &[r, file, line])
}

/// `FfxFmod_initEventSystem(this)` (thiscall): FMOD's memory goes through
/// the game's callbacks (`FMOD_Memory_Initialize(0, 0, alloc, realloc,
/// free, -1)`); the event system is created into `+4` (line 0x9b; none:
/// `"EventSystem create failed!"` printed, its `eax` returned), its
/// system object fetched into `+8` (0xa4), stream buffers 0x10000 bytes
/// (unit 8, 0xa7), the event system started (0x80 channels, 0xab), the
/// global `0x00c3a480` cleared, the file callbacks installed (block
/// 0x800, 0xb1). Returns the last `ERRCHECK`'s `eax`.
pub fn init_event_system(h: &mut impl Host, this: u32) -> u32 {
    let [alloc, realloc, free] = MEMORY_CALLBACKS.map(|f| h.rebase(f));
    h.call_sym(
        MEMORY_INITIALIZE,
        Conv::Stdcall,
        &[0, 0, alloc, realloc, free, u32::MAX],
    );
    let system_at = this.wrapping_add(4);
    let r = h.call_sym(EVENT_SYSTEM_CREATE, Conv::Stdcall, &[system_at]);
    errcheck(h, r, 0x9b);
    let system: u32 = h.read(system_at);
    if system == 0 {
        let prefix = h.rebase(TOO_MANY[0]);
        let message = h.rebase(CREATE_FAILED);
        return h.call_sym(PRINT_TO_OUTPUT, Conv::Cdecl, &[prefix, message]);
    }
    let object_at = this.wrapping_add(8);
    let r = h.call_sym(GET_SYSTEM_OBJECT, Conv::Stdcall, &[system, object_at]);
    errcheck(h, r, 0xa4);
    let object: u32 = h.read(object_at);
    let r = h.call_sym(SET_STREAM_BUFFER_SIZE, Conv::Stdcall, &[object, 0x10000, 8]);
    errcheck(h, r, 0xa7);
    let system: u32 = h.read(system_at);
    let r = h.call_sym(EVENT_SYSTEM_INIT, Conv::Stdcall, &[system, 0x80, 0, 0, 0]);
    errcheck(h, r, 0xab);
    h.call_sym(CLEAR_GLOBAL, Conv::Cdecl, &[]);
    let [open, close, read, seek] = FILE_CALLBACKS.map(|f| h.rebase(f));
    let object: u32 = h.read(object_at);
    let r = h.call_sym(
        SET_FILE_SYSTEM,
        Conv::Stdcall,
        &[object, open, close, read, seek, 0, 0, 0x800],
    );
    errcheck(h, r, 0xb1)
}

// ---- the sound bank

/// `FMOD::EventSystem::load`'s thunk (stdcall).
pub const EVENT_SYSTEM_LOAD: u32 = 0x0094_8c94;
/// `FmodEventTree_ctor_0070ff50(tree; project, path)` (thiscall).
pub const EVENT_TREE_CTOR: u32 = 0x0070_ff50;
/// `SfxEventGroup_ctor_00710780(group; project, path)` (thiscall).
pub const EVENT_GROUP_CTOR: u32 = 0x0071_0780;
/// `report_rangecheckfailure()` (never returns).
pub const RANGE_CHECK_FAILURE: u32 = 0x0094_9b02;
/// The load messages: `"[Fmod Log]"` (`0x00b4f190`), `"this=0x%p, Load
/// Fmod Project '%s'\n"` (`0x00b4fdf4`), `"FMOD Error! Fail to create
/// SFX project '%s'\n"` (`0x00b4fe34`).
pub const LOAD_MESSAGES: [u32; 3] = [0x00b4_f190, 0x00b4_fdf4, 0x00b4_fe34];
/// The event list names: `"%s_common.txt"` (`0x00b4fe18`),
/// `"%s_loop.txt"` (`0x00b4fe28`).
pub const LIST_FORMATS: [u32; 2] = [0x00b4_fe18, 0x00b4_fe28];
/// Bytes of [`sfx_bank_ctor`]'s frame, from `ebp - 0x210`: the loop
/// list's name (`+0`) and the common list's (`+0x100`), 0x100 bytes each.
pub const BANK_FRAME: u32 = 0x200;

/// `strlen` of the C string at `s`.
fn strlen(h: &impl Host, s: u32) -> u32 {
    let mut n = 0u32;
    while h.read::<u8>(s.wrapping_add(n)) != 0 {
        n = n.wrapping_add(1);
    }
    n
}

/// `name` = `path` without its last four bytes (the extension), in a
/// zeroed 0x100-byte buffer, then `sprintf(name, format, name)`.
fn list_name(h: &mut impl Host, name: u32, path: u32, len: u32, format: u32) {
    h.write(name, 0u8);
    h.call_sym(MEMSET, Conv::Cdecl, &[name.wrapping_add(1), 0, 0xff]);
    h.call_sym(MEMCPY, Conv::Cdecl, &[name, path, len]);
    h.write(name.wrapping_add(len).wrapping_sub(4), 0u8);
    let sprintf: u32 = h.read(h.rebase(SPRINTF_IAT));
    let format = h.rebase(format);
    h.call(sprintf, Conv::Cdecl, &[name, format, name]);
}

/// A failed bank: `+8`, `+4`, `+0` cleared.
fn clear_bank(h: &mut impl Host, this: u32) {
    for off in [8u32, 4, 0] {
        h.write(this.wrapping_add(off), 0u32);
    }
}

/// `SfxBank_ctor_0070f760(this; system, path)` (thiscall, ret 8; `frame`
/// see [`BANK_FRAME`]; without its SEH frame): the project `path` loaded
/// into `+0` (`EventSystem::load(system, path, 0, this)`); without a
/// system or on failure (printed) `+8`, `+4`, `+0` are cleared. Else,
/// logged, the event tree (`+4`, `operator new(0x34)`,
/// `FmodEventTree_ctor(project, "<path>_common.txt")`) and the event
/// group (`+8`, `SfxEventGroup_ctor(project, "<path>_loop.txt")`; 0 when
/// `new` fails) are built, `<path>` the path without its extension (a
/// path shorter than 4 bytes or of 0x104 and more fails the range check,
/// which never returns). Returns `this`.
pub fn sfx_bank_ctor(h: &mut impl Host, frame: u32, this: u32, system: u32, path: u32) -> u32 {
    if system == 0 {
        clear_bank(h, this);
        return this;
    }
    let r = h.call_sym(EVENT_SYSTEM_LOAD, Conv::Stdcall, &[system, path, 0, this]);
    if r != 0 {
        let prefix = h.rebase(TOO_MANY[0]);
        let message = h.rebase(LOAD_MESSAGES[2]);
        h.call_sym(PRINT_TO_OUTPUT, Conv::Cdecl, &[prefix, message, path]);
        clear_bank(h, this);
        return this;
    }
    let prefix = h.rebase(LOAD_MESSAGES[0]);
    let message = h.rebase(LOAD_MESSAGES[1]);
    h.call_sym(PRINT_TO_OUTPUT, Conv::Cdecl, &[prefix, message, this, path]);
    let len = strlen(h, path);
    let common = frame.wrapping_add(0x100);
    if len.wrapping_sub(4) >= 0x100 {
        // the original's buffer index check: it never returns
        h.write(common, 0u8);
        h.call_sym(MEMSET, Conv::Cdecl, &[common.wrapping_add(1), 0, 0xff]);
        h.call_sym(MEMCPY, Conv::Cdecl, &[common, path, len]);
        return h.call_sym(RANGE_CHECK_FAILURE, Conv::Cdecl, &[]);
    }
    list_name(h, common, path, len, LIST_FORMATS[0]);
    let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x34]);
    let tree = if p == 0 {
        0
    } else {
        let project: u32 = h.read(this);
        h.call_sym(EVENT_TREE_CTOR, Conv::Thiscall(p), &[project, common])
    };
    h.write(this.wrapping_add(4), tree);
    let looped = frame;
    list_name(h, looped, path, len, LIST_FORMATS[1]);
    let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x34]);
    let group = if p == 0 {
        0
    } else {
        let project: u32 = h.read(this);
        h.call_sym(EVENT_GROUP_CTOR, Conv::Thiscall(p), &[project, looped])
    };
    h.write(this.wrapping_add(8), group);
    this
}

// ---- the event tree

/// `vfxIntTree_Insert(tree; node)` (thiscall): 0 when inserted.
pub const INT_TREE_INSERT: u32 = 0x0070_fda0;

/// `FmodEventTree_load_00710370(this; path)` (thiscall, ret 4): an
/// existing file (`fiosIsFileExist`, `fiosOpen(path, 1)`; else 0) is read
/// whole into a new buffer (`operator new` of its size plus a 0 byte;
/// `fiosRead`, `fiosClose`); each of its 8-byte records (key, value)
/// becomes a node of the tree at `+4` (from the free list `+0x1c`,
/// refilled once by `allocateNewBlock(+0x24)` when empty, a record
/// skipped when still empty): inserted (`vfxIntTree_Insert`, then
/// `postInsertFinalize`) or given back when the key is there. The buffer
/// is freed (`operator delete[]`, its `eax` returned).
pub fn event_tree_load(h: &mut impl Host, this: u32, path: u32) -> u32 {
    if h.call_sym(FIOS_EXISTS, Conv::Cdecl, &[path]) == 0 {
        return 0;
    }
    let file = h.call_sym(FIOS_OPEN, Conv::Cdecl, &[path, 1]);
    if file == 0 {
        return 0;
    }
    let size = h.call_sym(FIOS_SIZE, Conv::Cdecl, &[file]);
    let buf = h.call_sym(OPERATOR_NEW_2, Conv::Cdecl, &[size.wrapping_add(1)]);
    h.write(buf.wrapping_add(size), 0u8);
    h.call_sym(FIOS_READ, Conv::Cdecl, &[file, buf, size]);
    h.call_sym(FIOS_CLOSE, Conv::Cdecl, &[file]);
    let n = size >> 3;
    let tree = this.wrapping_add(4);
    let list = this.wrapping_add(0x1c);
    for i in 0..n {
        let mut node: u32 = h.read(list);
        if node == 0 {
            let block: u32 = h.read(this.wrapping_add(0x24));
            h.call_sym(FREE_LIST_ALLOCATE_BLOCK, Conv::Thiscall(list), &[block]);
            node = h.read(list);
            if node == 0 {
                continue;
            }
        }
        let next: u32 = h.read(node);
        h.write(list, next);
        for k in 0..3u32 {
            h.write(node.wrapping_add(4 * k), node);
        }
        let key: u32 = h.read(buf.wrapping_add(8 * i));
        h.write(node.wrapping_add(0xc), key);
        let value: u32 = h.read(buf.wrapping_add(8 * i + 4));
        h.write(node.wrapping_add(0x10), value);
        if h.call_sym(INT_TREE_INSERT, Conv::Thiscall(tree), &[node]) == 0 {
            h.call_sym(TREE_POST_INSERT, Conv::Thiscall(tree), &[node]);
        } else {
            h.call_sym(FREE_LIST_DEALLOCATE, Conv::Thiscall(list), &[node]);
        }
    }
    h.call_sym(OPERATOR_DELETE_ARRAY, Conv::Cdecl, &[buf])
}
