//! `PhyreEngine`'s scripting support outside the script layer (M5 stage 1,
//! list `phyre-scripting`, batch 7): the scheduler's runs: a frame's tick
//! (the entries due this frame), a list of one-shot entries, and one
//! entry's named function now. Each run goes through the persistent
//! state of the entry (`executePersistent`) or a slot of the scheduler's
//! state pool (`execute` on the state the slot hands out).

use ffx_graphic::host::{Conv, Host};

use crate::m5p_scripting_mid::bridge as mid;

/// The scheduler running its update (`0x17` refuses a nested one).
pub const SCHEDULER_RUNNING: u32 = 0x00cb_d9b8;
/// The entry being run.
pub const CURRENT_ENTRY: u32 = 0x00cb_d9bc;
/// `PScheduledScript::executePersistent(entry; function)` (thiscall).
pub const EXECUTE_PERSISTENT: u32 = 0x0061_8490;
/// `PScheduledScript::execute(entry; L, function)` (thiscall).
pub const EXECUTE: u32 = 0x0061_8330;
/// `0x00618130` (thiscall on a pool slot): the slot's state.
pub const SLOT_STATE: u32 = 0x0061_8130;
/// `Phyre_PFreeList_deallocate(list; p)` (thiscall).
pub const FREELIST_DEALLOCATE: u32 = 0x0049_c980;
/// `0x0054dcd0` (thiscall on a name table, `&name`, 0): the name's index.
pub const NAME_INDEX: u32 = 0x0054_dcd0;

/// The ports over game memory.
pub mod bridge {
    use super::{
        CURRENT_ENTRY, Conv, EXECUTE, EXECUTE_PERSISTENT, FREELIST_DEALLOCATE, Host, NAME_INDEX,
        SCHEDULER_RUNNING, SLOT_STATE, mid,
    };

    /// Entry `e`'s function `function` run: in its own state (`+0x1c`)
    /// when it has one, else in a state from the scheduler's pool (20-byte
    /// slots at `+0x2c`, capacity `+0x28`, in use `+0x30`; the slot is
    /// given back when it is still the last). `None` when the pool is
    /// full.
    fn run_entry(h: &mut impl Host, this: u32, e: u32, function: u32) -> Option<u32> {
        let cur = h.rebase(CURRENT_ENTRY);
        if h.read::<u32>(e.wrapping_add(0x1c)) != 0 {
            h.write(cur, e);
            let r = h.call_sym(EXECUTE_PERSISTENT, Conv::Thiscall(e), &[function]);
            h.write(cur, 0u32);
            return Some(r);
        }
        let cap: u32 = h.read(this.wrapping_add(0x28));
        let used: u32 = h.read(this.wrapping_add(0x30));
        if used >= cap & 0x7fff_ffff {
            return None;
        }
        h.write(this.wrapping_add(0x30), used.wrapping_add(1));
        let pool: u32 = h.read(this.wrapping_add(0x2c));
        let slot = pool.wrapping_add(used.wrapping_mul(20));
        if slot == 0 {
            return None;
        }
        let l = h.call_sym(SLOT_STATE, Conv::Thiscall(slot), &[]);
        h.write(cur, e);
        let r = h.call_sym(EXECUTE, Conv::Thiscall(e), &[l, function]);
        h.write(cur, 0u32);
        let used: u32 = h.read(this.wrapping_add(0x30));
        if used != 0 {
            let pool: u32 = h.read(this.wrapping_add(0x2c));
            if pool.wrapping_add(used.wrapping_mul(20)).wrapping_sub(20) == slot {
                h.write(this.wrapping_add(0x30), used.wrapping_sub(1));
            }
        }
        Some(r)
    }

    /// `0x005d0910` (thiscall `(frame)`): the scheduler's tick: each live
    /// entry (state `+0x18` not 1) with a period (`+0x14`) whose phase
    /// (`+0x16`) is due this frame runs; one that fails loses its period.
    /// Then the finished entries are reaped. 0x17 while already running,
    /// else 0.
    pub fn tick(h: &mut impl Host, this: u32, frame: u32) -> u32 {
        let running = h.rebase(SCHEDULER_RUNNING);
        if h.read::<u32>(running) != 0 {
            return 0x17;
        }
        h.write(running, this);
        let mut e: u32 = h.read(this.wrapping_add(0x24));
        while e != 0 {
            let step: u16 = h.read(e.wrapping_add(0x14));
            let phase: u16 = h.read(e.wrapping_add(0x16));
            if h.read::<u32>(e.wrapping_add(0x18)) != 1
                && step != 0
                && u32::from(phase).wrapping_add(frame) % u32::from(step) == 0
            {
                let function: u32 = h.read(e.wrapping_add(0x10));
                if let Some(r) = run_entry(h, this, e, function)
                    && r != 0
                {
                    h.write(e.wrapping_add(0x14), 0u16);
                }
            }
            e = h.read(e);
        }
        mid::reap_finished(h, this);
        h.write(running, 0u32);
        0
    }

    /// `0x005d0670` (thiscall `(&list)`): each entry of the one-shot list
    /// run, then unlinked and freed. 0x17 while already running, else 0.
    pub fn run_list(h: &mut impl Host, this: u32, list: u32) -> u32 {
        let running = h.rebase(SCHEDULER_RUNNING);
        if h.read::<u32>(running) != 0 {
            return 0x17;
        }
        h.write(running, this);
        loop {
            let e: u32 = h.read(list);
            if e == 0 {
                break;
            }
            let function: u32 = h.read(e.wrapping_add(0x10));
            let _ = run_entry(h, this, e, function);
            let mut cell = list;
            let mut x: u32 = h.read(cell);
            while x != 0 {
                if x == e {
                    let n: u32 = h.read(x);
                    h.write(cell, n);
                    h.write(x, 0u32);
                    break;
                }
                cell = x;
                x = h.read(x);
            }
            h.write(e.wrapping_add(0x10), u32::MAX);
            h.call_sym(
                FREELIST_DEALLOCATE,
                Conv::Thiscall(this.wrapping_add(4)),
                &[e],
            );
        }
        mid::reap_finished(h, this);
        h.write(running, 0u32);
        0
    }

    /// `0x005d07d0` (thiscall `(owner, key, name)`): the owner's entry of
    /// that key runs its function `name` (the owner's name table) now;
    /// the run's result, or 0x13 when there is no entry, no such name or
    /// no free state. `cell` is the name word the original hands the
    /// lookup (its argument slot).
    pub fn run_now(
        h: &mut impl Host,
        this: u32,
        owner: u32,
        key: u32,
        name: u32,
        cell: u32,
    ) -> u32 {
        let name = if name != 0 && h.read::<u8>(name) == 0 {
            0
        } else {
            name
        };
        let mut e: u32 = h.read(this.wrapping_add(0x24));
        loop {
            if e == 0 {
                return 0x13;
            }
            if h.read::<u32>(e.wrapping_add(4)) == owner && h.read::<u32>(e.wrapping_add(8)) == key
            {
                break;
            }
            e = h.read(e);
        }
        h.write(cell, name);
        let idx = h.call_sym(
            NAME_INDEX,
            Conv::Thiscall(owner.wrapping_add(8)),
            &[cell, 0],
        );
        if (idx as i32) < 0 {
            return 0x13;
        }
        run_entry(h, this, e, idx).unwrap_or(0x13)
    }
}
