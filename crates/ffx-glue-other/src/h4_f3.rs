//! Continuation round 4 of list `h4-glue-other`, batch 3: the sound
//! effects' per-frame update and their event creation.
//!
//! Both run under the sound effects' mutex (`+0xcc`, `WaitForSingleObject`
//! and `ReleaseMutex` through their import slots) where they touch the
//! id tree at `+0x64`. Their growable arrays (count, capacity, entries)
//! grow to `n + n / 8 + 2` as the engine's `PArray` does.

use ffx_graphic::host::{Conv, Host};

use crate::frames::{OPERATOR_DELETE, PRINT_TO_OUTPUT, RELEASE_MUTEX, WAIT_FOR_SINGLE_OBJECT};
use crate::h4_c2::{FREE_LIST_DEALLOCATE, TREE_SUCCESSOR, USER_FREE_THUNK, tree_begin, walk_tree};
use crate::h4_d4::{MEMCPY, USER_MALLOC};
use crate::h4_e1::pass;
use crate::h4_f1::{OPERATOR_NEW, SFX_ANY_EVENT_FLAG, SFX_BANK_DTOR, SFX_BANK_PLAYING};

/// `SfxEvent_updateFades_0070f5c0(event; dt)` (thiscall).
pub const SFX_EVENT_UPDATE_FADES: u32 = 0x0070_f5c0;
/// `SfxEvent_dtor_0070e850(event)` (thiscall).
pub const SFX_EVENT_DTOR: u32 = 0x0070_e850;
/// `SfxEvent_ctor_0070e710(event; src, volume)` (thiscall).
pub const SFX_EVENT_CTOR: u32 = 0x0070_e710;
/// `Phyre_PRedBlackTree_remove(tree; node)` (thiscall).
pub const TREE_REMOVE: u32 = 0x0057_5000;
/// `SfxBank_appendEventById_0070fa80(bank; id, out)` (thiscall, `al`).
pub const BANK_APPEND_EVENT: u32 = 0x0070_fa80;
/// `SfxIdTree_insertArray_0070cde0(tree; &key, &array)` (thiscall).
pub const ID_TREE_INSERT: u32 = 0x0070_cde0;
/// The prefix and format [`sfx_create_event`] prints for a sound with too
/// many live events (`0x00b4c30c`, `0x00b4fc48`).
pub const TOO_MANY: [u32; 2] = [0x00b4_c30c, 0x00b4_fc48];

fn wait(h: &mut impl Host, this: u32) {
    let f: u32 = h.read(h.rebase(WAIT_FOR_SINGLE_OBJECT));
    let mutex: u32 = h.read(this.wrapping_add(0xcc));
    h.call(f, Conv::Stdcall, &[mutex, u32::MAX]);
}

fn release(h: &mut impl Host, this: u32) {
    let f: u32 = h.read(h.rebase(RELEASE_MUTEX));
    let mutex: u32 = h.read(this.wrapping_add(0xcc));
    h.call(f, Conv::Stdcall, &[mutex]);
}

/// The node of the `PRedBlackTree` at `tree` keyed `key` (`+0xc`, signed;
/// root `tree + 0xc`, children `+0` and `+4`), or `tree` when none.
fn tree_find(h: &impl Host, tree: u32, key: u32) -> u32 {
    let mut node: u32 = h.read(tree.wrapping_add(0xc));
    while node != tree {
        let k: u32 = h.read(node.wrapping_add(0xc));
        if k == key {
            return node;
        }
        let right = k.cast_signed() < key.cast_signed();
        node = h.read(node.wrapping_add(if right { 4 } else { 0 }));
    }
    tree
}

/// The capacity a full array of `count` entries grows to.
fn grown(count: u32) -> u32 {
    let n = count.cast_signed();
    n.wrapping_div(8)
        .wrapping_add(2)
        .wrapping_add(n)
        .cast_unsigned()
}

/// A growable array of words (count, capacity, entries), as the
/// engine's `PArray` lays it out.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Words {
    pub(crate) count: u32,
    pub(crate) cap: u32,
    pub(crate) data: u32,
}

impl Words {
    pub(crate) fn load(h: &impl Host, at: u32) -> Self {
        Self {
            count: h.read(at),
            cap: h.read(at.wrapping_add(4)),
            data: h.read(at.wrapping_add(8)),
        }
    }

    /// Counts one more entry and grows a full array (`user_malloc`, the
    /// entries copied with `memcpy` of the new count's bytes, the old
    /// freed); returns the new entry's slot (0 when the entries are
    /// missing).
    pub(crate) fn grow(&mut self, h: &mut impl Host) -> u32 {
        self.count = self.count.wrapping_add(1);
        if self.count.cast_signed() > self.cap.cast_signed() {
            let cap = grown(self.count);
            if self.data == 0 {
                self.data = h.call_sym(USER_MALLOC, Conv::Cdecl, &[cap.wrapping_mul(4)]);
            } else {
                let mut p = 0u32;
                if cap.cast_signed() > 0 {
                    p = h.call_sym(USER_MALLOC, Conv::Cdecl, &[cap.wrapping_mul(4)]);
                    if p != 0 {
                        let bytes = self.count.wrapping_mul(4);
                        h.call_sym(MEMCPY, Conv::Cdecl, &[p, self.data, bytes]);
                    }
                }
                h.call_sym(USER_FREE_THUNK, Conv::Cdecl, &[self.data]);
                self.data = p;
            }
            self.cap = cap;
        }
        self.data
            .wrapping_add(self.count.wrapping_mul(4))
            .wrapping_sub(4)
    }
}

/// [`walk_tree`] that stops at the node `visit` returns true for (no
/// successor taken after it, as the originals break out).
fn walk_while<H: Host>(h: &mut H, tree: u32, mut visit: impl FnMut(&mut H, u32) -> bool) {
    let mut node = tree_begin(h, tree);
    if node == tree {
        return;
    }
    let mut next = h.call_sym(TREE_SUCCESSOR, Conv::Thiscall(tree), &[node]);
    while node != tree {
        if visit(h, node) {
            return;
        }
        node = next;
        if next != tree {
            next = h.call_sym(TREE_SUCCESSOR, Conv::Thiscall(tree), &[next]);
        }
    }
}

/// Destroys and frees a sound event.
fn delete_event(h: &mut impl Host, event: u32) {
    h.call_sym(SFX_EVENT_DTOR, Conv::Thiscall(event), &[]);
    h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[event]);
}

/// Hands `event` its fades unless paused (byte `this + 0`) while one of
/// its instances is flagged; returns true (to be freed) when none is.
fn update_event(h: &mut impl Host, this: u32, event: u32, dt: u32) -> bool {
    if h.call_sym(SFX_ANY_EVENT_FLAG, Conv::Thiscall(event), &[]) as u8 != 0 {
        if h.read::<u8>(this) == 0 {
            h.call_sym(SFX_EVENT_UPDATE_FADES, Conv::Thiscall(event), &[pass(dt)]);
        }
        return false;
    }
    true
}

/// `FmodSfx_update(this; dt)` (thiscall, ret 4; without its SEH frame).
/// Under the mutex every sound of the id tree (`+0x64`) runs its events
/// (`+0x18`, count `+0x10`, unsigned): flagged ones get their fades
/// (unless paused, byte `+0`), the others are destroyed and cleared; a
/// sound whose every slot is empty has its id (`+0xc`) noted, and each
/// noted sound is then removed from the tree (`PRedBlackTree_remove`, its
/// array `+0x18` freed, the node given back to the free list `+0x7c`).
/// After the mutex the first bank of the tree at `+0x94` with no playing
/// events is destroyed, its node removed and given back (`+0xac`). The
/// stream event (`+0xc8`) is run as an event (cleared when destroyed).
/// Its callers read no result.
pub fn sfx_update(h: &mut impl Host, this: u32, dt: u32) {
    wait(h, this);
    let mut ids = Words::default();
    let tree = this.wrapping_add(0x64);
    walk_tree(h, tree, |h, node| {
        let mut empty = 0u32;
        let mut i = 0u32;
        while i < h.read::<u32>(node.wrapping_add(0x10)) {
            let events: u32 = h.read(node.wrapping_add(0x18));
            let event: u32 = h.read(events.wrapping_add(4 * i));
            if event == 0 {
                empty = empty.wrapping_add(1);
            } else if update_event(h, this, event, dt) {
                empty = empty.wrapping_add(1);
                let events: u32 = h.read(node.wrapping_add(0x18));
                let event: u32 = h.read(events.wrapping_add(4 * i));
                if event != 0 {
                    delete_event(h, event);
                }
                let events: u32 = h.read(node.wrapping_add(0x18));
                h.write(events.wrapping_add(4 * i), 0u32);
            }
            i += 1;
        }
        if h.read::<u32>(node.wrapping_add(0x10)) == empty {
            let slot = ids.grow(h);
            if slot != 0 {
                let id: u32 = h.read(node.wrapping_add(0xc));
                h.write(slot, id);
            }
        }
    });
    let mut k = 0u32;
    while k < ids.count {
        let id: u32 = h.read(ids.data.wrapping_add(4 * k));
        let node = tree_find(h, tree, id);
        if node != tree {
            h.call_sym(TREE_REMOVE, Conv::Thiscall(tree), &[node]);
            let array: u32 = h.read(node.wrapping_add(0x18));
            if array != 0 {
                h.call_sym(USER_FREE_THUNK, Conv::Cdecl, &[array]);
            }
            h.call_sym(
                FREE_LIST_DEALLOCATE,
                Conv::Thiscall(tree.wrapping_add(0x18)),
                &[node],
            );
        }
        k += 1;
    }
    release(h, this);
    // the first bank with nothing playing
    let banks = this.wrapping_add(0x94);
    let mut dead: Option<u32> = None;
    walk_while(h, banks, |h, node| {
        let bank: u32 = h.read(node.wrapping_add(0x10));
        if h.call_sym(SFX_BANK_PLAYING, Conv::Thiscall(bank), &[]) != 0 {
            return false;
        }
        let keys = h.call_sym(USER_MALLOC, Conv::Cdecl, &[0xc]);
        if keys != 0 {
            let key: u32 = h.read(node.wrapping_add(0xc));
            h.write(keys, key);
        }
        if bank != 0 {
            h.call_sym(SFX_BANK_DTOR, Conv::Thiscall(bank), &[]);
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[bank]);
        }
        dead = Some(keys);
        true
    });
    if let Some(keys) = dead {
        let id: u32 = h.read(keys);
        let node = tree_find(h, banks, id);
        if node != banks {
            h.call_sym(TREE_REMOVE, Conv::Thiscall(banks), &[node]);
            h.call_sym(
                FREE_LIST_DEALLOCATE,
                Conv::Thiscall(banks.wrapping_add(0x18)),
                &[node],
            );
        }
        if keys != 0 {
            h.call_sym(USER_FREE_THUNK, Conv::Cdecl, &[keys]);
        }
    }
    let stream: u32 = h.read(this.wrapping_add(0xc8));
    if stream != 0 && update_event(h, this, stream, dt) {
        let stream: u32 = h.read(this.wrapping_add(0xc8));
        if stream != 0 {
            delete_event(h, stream);
        }
        h.write(this.wrapping_add(0xc8), 0u32);
    }
    if ids.data != 0 {
        h.call_sym(USER_FREE_THUNK, Conv::Cdecl, &[ids.data]);
    }
}

/// Bytes of [`sfx_create_event`]'s frame, from `ebp - 0x2c`: the new id
/// list (`+0`: count, capacity, entries), the event list the banks fill
/// (`+0xc`), the id argument word (`+0x34`, whose address the id tree's
/// insert takes).
pub const CREATE_FRAME: u32 = 0x38;

/// `FmodSfx_createSfxEvent(this; id)` (thiscall, ret 4; `frame` see
/// [`CREATE_FRAME`]; without its SEH frame): the first bank of the tree
/// at `+0x34`, else of the tree at `+4`, that has the event `id`
/// (`SfxBank_appendEventById(id, &list)`, `al`) gives a new `SfxEvent`
/// (`operator new(0x28)`, `SfxEvent_ctor(&list, volume +0xd0)`); a
/// failed one in the first tree goes on to the second. A new event is
/// registered under the mutex in the id tree (`+0x64`): appended to its
/// sound's array (`+0x10` of the node; more than 5 live ones, 0x1e for
/// sound 0x72, prints a warning first) or inserted as a new sound with a
/// one-entry array (`SfxIdTree_insertArray(&id, &array)`). The list's
/// entries are freed. Returns the event or 0.
pub fn sfx_create_event(h: &mut impl Host, frame: u32, this: u32, id: u32) -> u32 {
    let tmp = frame;
    let list = frame.wrapping_add(0xc);
    let key = frame.wrapping_add(0x34);
    h.write(key, id);
    for off in [0u32, 4, 8] {
        h.write(list.wrapping_add(off), 0u32);
    }
    let mut event = 0u32;
    for (n, tree) in [this.wrapping_add(0x34), this.wrapping_add(4)]
        .into_iter()
        .enumerate()
    {
        let mut matched = false;
        walk_while(h, tree, |h, node| {
            let bank: u32 = h.read(node.wrapping_add(0x10));
            matched = h.call_sym(BANK_APPEND_EVENT, Conv::Thiscall(bank), &[id, list]) as u8 != 0;
            matched
        });
        if !matched {
            continue;
        }
        let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x28]);
        event = if p == 0 {
            0
        } else {
            let volume = pass(h.read(this.wrapping_add(0xd0)));
            h.call_sym(SFX_EVENT_CTOR, Conv::Thiscall(p), &[list, volume])
        };
        if event != 0 {
            register(h, this, tmp, key, id, event);
            break;
        }
        if n == 1 {
            break;
        }
    }
    let data: u32 = h.read(list.wrapping_add(8));
    if data != 0 {
        h.call_sym(USER_FREE_THUNK, Conv::Cdecl, &[data]);
    }
    event
}

/// [`sfx_create_event`]'s registration of `event` under the mutex.
fn register(h: &mut impl Host, this: u32, tmp: u32, key: u32, id: u32, event: u32) {
    wait(h, this);
    let tree = this.wrapping_add(0x64);
    let node = tree_find(h, tree, id);
    let array = node.wrapping_add(0x10);
    if node != tree && array != 0 {
        let n: u32 = h.read(array);
        let data: u32 = h.read(array.wrapping_add(8));
        let mut live = 0u32;
        for k in 0..n {
            if h.read::<u32>(data.wrapping_add(4 * k)) != 0 {
                live = live.wrapping_add(1);
            }
        }
        let limit = if id == 0x72 { 0x1e } else { 5 };
        if live.cast_signed() > limit {
            let prefix = h.rebase(TOO_MANY[0]);
            let format = h.rebase(TOO_MANY[1]);
            h.call_sym(PRINT_TO_OUTPUT, Conv::Cdecl, &[prefix, format, id]);
        }
        let mut words = Words::load(h, array);
        let slot = words.grow(h);
        h.write(array, words.count);
        h.write(array.wrapping_add(4), words.cap);
        h.write(array.wrapping_add(8), words.data);
        if slot != 0 {
            h.write(slot, event);
        }
    } else {
        h.write(tmp, 1u32);
        let data = h.call_sym(USER_MALLOC, Conv::Cdecl, &[0xc]);
        h.write(tmp.wrapping_add(8), data);
        h.write(tmp.wrapping_add(4), 3u32);
        if data != 0 {
            h.write(data, event);
        }
        h.call_sym(ID_TREE_INSERT, Conv::Thiscall(tree), &[key, tmp]);
        if data != 0 {
            h.call_sym(USER_FREE_THUNK, Conv::Cdecl, &[data]);
        }
    }
    release(h, this);
}

/// `getFullFilename(out, name)` (cdecl).
pub const FULL_FILENAME: u32 = 0x0070_7aa0;
/// `memset`'s thunk.
pub const MEMSET: u32 = 0x0094_964c;
/// `SfxBank_ctor_0070f760(bank; a, path)` (thiscall).
pub const SFX_BANK_CTOR: u32 = 0x0070_f760;
/// `SfxEvent_stop(event; immediate)` (thiscall).
pub const SFX_EVENT_STOP: u32 = 0x0070_f490;
/// `SfxEvent_start(event; volume, pan)` (thiscall).
pub const SFX_EVENT_START: u32 = 0x0070_f310;
/// The battle stream's project, English and Japanese
/// (`"SFX/US/9999.fev"`, `"SFX/JP/9999.fev"`).
pub const STREAM_PROJECTS: [u32; 2] = [0x00b4_fc90, 0x00b4_fca0];
/// The message for a missing stream event (`0x00b4fcb0`).
pub const NO_STREAM_EVENT: u32 = 0x00b4_fcb0;
/// Bytes of [`sfx_play_battle_streaming`]'s frame, from `ebp - 0x11c`:
/// the event list (`+0`) and the project path (`+0xc`, 0x100 bytes).
pub const STREAM_FRAME: u32 = 0x10c;

/// `FmodSfx_playBattleStreaming(this; a, id, volume, pan)` (thiscall, ret
/// 0x10; `frame` see [`STREAM_FRAME`]; without its SEH frame): nothing
/// while paused (byte `+0`). Without the stream bank (`+0xc4`) one is
/// made (`operator new(0xc)`, `SfxBank_ctor(a, path)`; 0 when none) from
/// the stream project (byte `+2` set: the English one) through
/// `getFullFilename`, into a zeroed path. The old stream event (`+0xc8`)
/// is stopped (immediate 0), destroyed and cleared. The bank's event `id`
/// (`SfxBank_appendEventById`) becomes the new stream event
/// (`operator new(0x28)`, `SfxEvent_ctor(&list, volume +0xd0)`; started
/// with `volume`, `pan`); a missing one is reported (`printToOutput`).
/// The list's entries are freed. Its callers read no result.
pub fn sfx_play_battle_streaming(
    h: &mut impl Host,
    frame: u32,
    this: u32,
    [a, id, volume, pan]: [u32; 4],
) {
    if h.read::<u8>(this) != 0 {
        return;
    }
    let list = frame;
    let path = frame.wrapping_add(0xc);
    if h.read::<u32>(this.wrapping_add(0xc4)) == 0 {
        h.write(path, 0u8);
        h.call_sym(MEMSET, Conv::Cdecl, &[path.wrapping_add(1), 0, 0xff]);
        let english = h.read::<u8>(this.wrapping_add(2)) != 0;
        let project = h.rebase(if english {
            STREAM_PROJECTS[0]
        } else {
            STREAM_PROJECTS[1]
        });
        h.call_sym(FULL_FILENAME, Conv::Cdecl, &[path, project]);
        let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0xc]);
        let bank = if p == 0 {
            0
        } else {
            h.call_sym(SFX_BANK_CTOR, Conv::Thiscall(p), &[a, path])
        };
        h.write(this.wrapping_add(0xc4), bank);
    }
    let stream: u32 = h.read(this.wrapping_add(0xc8));
    if stream != 0 {
        h.call_sym(SFX_EVENT_STOP, Conv::Thiscall(stream), &[0]);
        let stream: u32 = h.read(this.wrapping_add(0xc8));
        if stream != 0 {
            delete_event(h, stream);
        }
        h.write(this.wrapping_add(0xc8), 0u32);
    }
    for off in [0u32, 4, 8] {
        h.write(list.wrapping_add(off), 0u32);
    }
    let bank: u32 = h.read(this.wrapping_add(0xc4));
    if h.call_sym(BANK_APPEND_EVENT, Conv::Thiscall(bank), &[id, list]) as u8 != 0 {
        let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x28]);
        let event = if p == 0 {
            0
        } else {
            let v = pass(h.read(this.wrapping_add(0xd0)));
            h.call_sym(SFX_EVENT_CTOR, Conv::Thiscall(p), &[list, v])
        };
        h.write(this.wrapping_add(0xc8), event);
        h.call_sym(SFX_EVENT_START, Conv::Thiscall(event), &[volume, pan]);
    } else {
        let prefix = h.rebase(TOO_MANY[0]);
        let message = h.rebase(NO_STREAM_EVENT);
        h.call_sym(PRINT_TO_OUTPUT, Conv::Cdecl, &[prefix, message, id]);
    }
    let data: u32 = h.read(list.wrapping_add(8));
    if data != 0 {
        h.call_sym(USER_FREE_THUNK, Conv::Cdecl, &[data]);
    }
}

/// `SfxEventTree_Find_0070ba20(tree; &key)` (thiscall): the node or 0.
pub const TREE_FIND: u32 = 0x0070_ba20;
/// `vfxIntTree_add_0070cd60(tree; &key, &value)` (thiscall).
pub const TREE_ADD: u32 = 0x0070_cd60;
/// `sprintf`'s import slot.
pub const SPRINTF_IAT: u32 = 0x00b0_c3dc;
/// The project name formats, English and Japanese (`"SFX/US/%04d.fev"`,
/// `"SFX/JP/%04d.fev"`).
pub const PROJECT_FORMATS: [u32; 2] = [0x00b4_fb60, 0x00b4_fb70];
/// Bytes of [`sfx_load_project`]'s frame, from `ebp - 0x214`: the new
/// bank (`+0`, whose address the tree's add takes), the full path
/// (`+4`, 0x100 bytes), the project name (`+0x104`, 0x100 bytes), the id
/// argument word (`+0x220`).
pub const LOAD_FRAME: u32 = 0x224;

/// `FmodSfx_loadProject(this; a, id, resident)` (thiscall, ret 0xc;
/// `frame` see [`LOAD_FRAME`]; without its SEH frame): nothing for `a`
/// 0. A project of the kept tree (`+0x94`) moves back to the tree at
/// `+0x34` (`vfxIntTree_add`; its node removed and given back to `+0xac`).
/// Else, unless the tree at `+4` or `+0x34` has it, a new bank
/// (`operator new(0xc)`, `SfxBank_ctor(a, path)`; 0 when none) is made
/// from `"SFX/US/%04d.fev"` (byte `+2` set, else `"SFX/JP/..."`) through
/// `sprintf` and `getFullFilename` into zeroed buffers, and added to the
/// tree at `+4` (low byte of `resident` set) or `+0x34`. Its callers
/// read no result.
pub fn sfx_load_project(h: &mut impl Host, frame: u32, this: u32, [a, id, resident]: [u32; 3]) {
    let slot = frame;
    let path = frame.wrapping_add(4);
    let name = frame.wrapping_add(0x104);
    let key = frame.wrapping_add(0x220);
    h.write(key, id);
    if a == 0 {
        return;
    }
    let kept = this.wrapping_add(0x94);
    let f = h.call_sym(TREE_FIND, Conv::Thiscall(kept), &[key]);
    if f != 0 && f.wrapping_add(0x10) != 0 {
        h.call_sym(
            TREE_ADD,
            Conv::Thiscall(this.wrapping_add(0x34)),
            &[key, f.wrapping_add(0x10)],
        );
        let g = h.call_sym(TREE_FIND, Conv::Thiscall(kept), &[key]);
        if g != 0 {
            h.call_sym(TREE_REMOVE, Conv::Thiscall(kept), &[g]);
            h.call_sym(
                FREE_LIST_DEALLOCATE,
                Conv::Thiscall(kept.wrapping_add(0x18)),
                &[g],
            );
        }
        return;
    }
    for tree in [this.wrapping_add(4), this.wrapping_add(0x34)] {
        let r = h.call_sym(TREE_FIND, Conv::Thiscall(tree), &[key]);
        if r != 0 && r.wrapping_add(0x10) != 0 {
            return;
        }
    }
    h.write(name, 0u8);
    h.call_sym(MEMSET, Conv::Cdecl, &[name.wrapping_add(1), 0, 0xff]);
    let format = h.rebase(if h.read::<u8>(this.wrapping_add(2)) != 0 {
        PROJECT_FORMATS[0]
    } else {
        PROJECT_FORMATS[1]
    });
    let id_now: u32 = h.read(key);
    let sprintf: u32 = h.read(h.rebase(SPRINTF_IAT));
    h.call(sprintf, Conv::Cdecl, &[name, format, id_now]);
    h.write(path, 0u8);
    h.call_sym(MEMSET, Conv::Cdecl, &[path.wrapping_add(1), 0, 0xff]);
    h.call_sym(FULL_FILENAME, Conv::Cdecl, &[path, name]);
    let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0xc]);
    let bank = if p == 0 {
        0
    } else {
        h.call_sym(SFX_BANK_CTOR, Conv::Thiscall(p), &[a, path])
    };
    h.write(slot, bank);
    let tree = if resident as u8 != 0 {
        this.wrapping_add(4)
    } else {
        this.wrapping_add(0x34)
    };
    h.call_sym(TREE_ADD, Conv::Thiscall(tree), &[key, slot]);
}

/// `Phyre_PFreeList_PFreeList(list; size, per_block, align, name)`
/// (thiscall).
pub const FREE_LIST_CTOR: u32 = crate::h4_c3::FREE_LIST_CTOR;
/// `"PMapPair"` (`0x00b13bc8`), the free lists' name.
pub const MAP_PAIR: u32 = 0x00b1_3bc8;
/// `CreateMutexW`'s import slot.
pub const CREATE_MUTEX_IAT: u32 = 0x00b0_c198;
/// The four trees of `FmodSfx` and their nodes' sizes.
pub const SFX_TREES: [(u32, u32); 4] = [(4, 0x14), (0x34, 0x14), (0x64, 0x1c), (0x94, 0x14)];

/// `FmodSfx_ctor_0070bfc0(this)` (thiscall; without its SEH frame): the
/// pause word (`+0`, 16 bits) and byte `+2` cleared; four empty
/// `PRedBlackTree`s (`+4`, `+0x34`, `+0x64`, `+0x94`: their six links
/// the tree itself) with free lists (`+0x18` of each: node size 0x14,
/// 0x1c for the id tree, 8 per block, align 4, `"PMapPair"`); the stream
/// bank and event (`+0xc4`, `+0xc8`) 0, the mutex (`+0xcc`,
/// `CreateMutexW(0, 0, 0)`), the whole volume (`+0xd0`) 1.0. Returns
/// `this`.
pub fn sfx_ctor(h: &mut impl Host, this: u32) -> u32 {
    h.write(this, 0u16);
    h.write(this.wrapping_add(2), 0u8);
    for (off, size) in SFX_TREES {
        let tree = this.wrapping_add(off);
        for k in 0..6u32 {
            h.write(tree.wrapping_add(4 * k), tree);
        }
        let name = h.rebase(MAP_PAIR);
        h.call_sym(
            FREE_LIST_CTOR,
            Conv::Thiscall(tree.wrapping_add(0x18)),
            &[size, 8, 4, name],
        );
    }
    h.write(this.wrapping_add(0xc4), 0u32);
    h.write(this.wrapping_add(0xc8), 0u32);
    let create: u32 = h.read(h.rebase(CREATE_MUTEX_IAT));
    let mutex = h.call(create, Conv::Stdcall, &[0, 0, 0]);
    h.write(this.wrapping_add(0xcc), mutex);
    h.write(this.wrapping_add(0xd0), 1.0f32.to_bits());
    this
}
