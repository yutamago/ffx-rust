//! Signals: the requests workers send each other (opcodes `36`-`3b`,
//! `45`-`53`), the replies that end them, and how a worker starts the one
//! at the head of its queue (`AtelDecodeSignal`) and returns from it
//! (`AtelExecReturn`). A signal is a `0x18`-byte node ([`sig`]) in one of
//! two doubly linked lists of the receiving worker: its queue (worker
//! `+0x80`, by level, highest first, replies at the end) and its free nodes
//! (worker `+0x88`). A node's `prev` points at the `next` field before it
//! (the previous node's, or the list head's), so unlinking is
//! `*prev = next; next.prev = prev`.

// w, l, s: the worker, its level and its stack, as everywhere in the bridge
#![allow(clippy::many_single_char_names)]

use ffx_bridge::{Host, Mem};

use super::{
    CONTEXT, CTRL_WORKS, ERROR, atel_get_basic_work, atel_get_real_var_adrs,
    atel_pop_stack_integer, atel_push_stack_integer, script_worker_level,
};
use crate::vm::{ctrl, is_small, level, level_index, worker};

/// The layout of a signal node.
pub mod sig {
    /// `u32`: the next node.
    pub const NEXT: u32 = 0x00;
    /// `u32`: the `next` field pointing at this node.
    pub const PREV: u32 = 0x04;
    /// `u16`: the entry point to run (or, in a reply, the one that ran).
    pub const ENTRY: u32 = 0x08;
    /// `u16`: the sending worker's index, `0xffff` none.
    pub const FROM: u32 = 0x0a;
    /// `u16`: the receiving worker's index.
    pub const TO: u32 = 0x0c;
    /// `u8`: bits 0-3 the level to run at, 4-6 the state (1 queued, 2
    /// running, set by [`super::atel_decode_signal`]), 7 new (cleared once
    /// the queue has been looked at).
    pub const LEVEL: u32 = 0x0e;
    /// `u8`: 0 request (`REQ`), 1 request the sender waits to start
    /// (`REQSW`), 2 request it waits to end (`REQEW`), 3 reply.
    pub const KIND: u32 = 0x0f;
    /// `u16`, `u16`, `u16`: the binary header index, the header table index
    /// and the ctrl work number the entry runs from (the level's `+0x3c`,
    /// `+0x3e`, `+0x40`).
    pub const HEADER: u32 = 0x10;
}

/// Worker fields of the signal lists.
pub mod sigwork {
    /// `u32`: the queue's first node.
    pub const QUEUE: u32 = 0x80;
    /// `u32`: the free nodes' first node.
    pub const FREE: u32 = 0x88;
    /// `u32`: the signal being run.
    pub const CURRENT: u32 = 0x90;
    /// `u16`: the entry point being run.
    pub const ENTRY: u32 = 0x2c;
    /// `u16` × 3: the header a request sent from level 0 carries (level 0
    /// `+0x3c`, `+0x3e`, `+0x40`).
    pub const HEADER: u32 = 0x168;
}

/// Level fields the signals use.
pub mod siglevel {
    /// `u8`: cleared when an entry starts.
    pub const RUNNING: u32 = 0x1e;
    /// `u16`, `u16`, `u16`: entry, worker and level the level waits on
    /// (opcodes `37`, `3a`, `77`).
    pub const WAIT_ENTRY: u32 = 0x2c;
    /// See [`WAIT_ENTRY`].
    pub const WAIT_WORKER: u32 = 0x2e;
    /// See [`WAIT_ENTRY`].
    pub const WAIT_LEVEL: u32 = 0x30;
    /// `u16`: the binary header index the level runs.
    pub const HEADER: u32 = 0x3c;
    /// `u16`: the header table index.
    pub const TABLE: u32 = 0x3e;
    /// `u16`: the ctrl work whose tables these index.
    pub const CTRL: u32 = 0x40;
    /// `u32`, `u32`: type 1 workers' move and turn blocks of the level.
    pub const MOVE: u32 = 0x44;
    /// See [`MOVE`].
    pub const TURN: u32 = 0x48;
}

/// A signal as the registering functions take it (on the stack in the
/// original): sender, receiver, kind, level, entry, then the header the
/// receiver will run (used only when the current ctrl work has header
/// tables, `+0x1f8`).
///
/// Every caller in the game fills the first five words and leaves the
/// header words as whatever was on its stack; the Rust callers pass 0
/// there ([`Request::new`]).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Request(pub [u32; 8]);

impl Request {
    /// A request without header words.
    #[must_use]
    pub const fn new(from: u32, to: u32, kind: u32, lvl: u32, entry: u32) -> Self {
        Self([from, to, kind, lvl, entry, 0, 0, 0])
    }

    /// The eight words at `at`.
    pub fn read(mem: &impl Mem, at: u32) -> Self {
        Self(core::array::from_fn(|i| {
            mem.read(at + 4 * u32::try_from(i).expect("8"))
        }))
    }
}

fn unlink(mem: &mut impl Mem, n: u32) {
    let prev = mem.read::<u32>(n + sig::PREV);
    let next = mem.read::<u32>(n + sig::NEXT);
    if prev != 0 {
        mem.write(prev, next);
    }
    if next != 0 {
        mem.write(next + sig::PREV, prev);
    }
}

/// `AtelLinkSigWorkNext(at, n)` (`0x0086dc10`): links `n` after the node
/// (or list head) `at`.
pub fn atel_link_sig_work_next(mem: &mut impl Mem, at: u32, n: u32) {
    let next = mem.read::<u32>(at);
    if next != 0 {
        mem.write(next + sig::PREV, n);
    }
    mem.write(at, n);
    mem.write(n + sig::PREV, at);
    mem.write(n + sig::NEXT, next);
}

/// `AtelLinkSigWorkPrev(at, n)` (`0x0086dc30`): links `n` before node `at`.
pub fn atel_link_sig_work_prev(mem: &mut impl Mem, at: u32, n: u32) {
    let prev = mem.read::<u32>(at + sig::PREV);
    if prev != 0 {
        mem.write(prev, n);
    }
    mem.write(at + sig::PREV, n);
    mem.write(n + sig::PREV, prev);
    mem.write(n + sig::NEXT, at);
}

/// `AtelLinkSigWorkButtom(head, n)` (`0x0086dc50`): links `n` at the end of
/// the list whose head is `head`.
pub fn atel_link_sig_work_buttom(mem: &mut impl Mem, head: u32, n: u32) {
    let mut at = head;
    loop {
        let next = mem.read::<u32>(at);
        if next == 0 {
            break;
        }
        at = next;
    }
    atel_link_sig_work_next(mem, at, n);
}

/// Moves node `n` from wherever it is to the free list of worker `w`.
fn free(mem: &mut impl Mem, w: u32, n: u32) {
    unlink(mem, n);
    let f = mem.read::<u32>(w + sigwork::FREE);
    atel_link_sig_work_next(mem, f, n);
}

/// `AtelRegistSignalMain(request, -1)` (`0x0086ea00`): takes a free node of
/// the receiver, fills it and queues it (a reply at the end, a request
/// before the first reply or lower level). Returns 1, or 0 when the
/// receiver is `0xffff` or has no free node ([`ERROR`] zeroed). While the
/// event's ctrl work is current, a sender whose `+0x36` has `0x1000` marks
/// the receiver `0x200` (clearing `0x400`).
pub fn atel_regist_signal_main(mem: &mut impl Mem, r: Request) -> u32 {
    let [from, to, kind, lvl, entry, h0, h1, h2] = r.0;
    if to == 0xffff {
        return 0;
    }
    let w = atel_get_basic_work(mem, to.cast_signed());
    let small = is_small(mem.read::<u8>(mem.read::<u32>(w + worker::HEADER)));
    let lvl = if small && lvl.cast_signed() >= 2 {
        1
    } else {
        lvl
    };
    let n = mem.read::<u32>(w + sigwork::FREE);
    if n == 0 {
        ERROR.set(mem, 0);
        return 0;
    }
    unlink(mem, n);
    let c = CONTEXT.get(mem);
    let (hdr, table, ctl) = if mem.read::<u32>(c + 0x1f8) == 0 {
        (
            mem.read::<u16>(w + sigwork::HEADER),
            mem.read::<u16>(w + sigwork::HEADER + 2),
            mem.read::<u16>(w + sigwork::HEADER + 4),
        )
    } else {
        #[allow(clippy::cast_possible_truncation)] // 16-bit fields
        (h2 as u16, h1 as u16, h0 as u16)
    };
    #[allow(clippy::cast_possible_truncation)] // the fields' widths
    {
        mem.write(n + sig::ENTRY, entry as u16);
        mem.write(n + sig::FROM, from as u16);
        mem.write(n + sig::TO, to as u16);
        mem.write(n + sig::KIND, kind as u8);
        let b = (lvl as u8 & 0xf) | 0x90;
        mem.write(n + sig::HEADER + 2, table);
        mem.write(n + sig::LEVEL, b);
        mem.write(n + sig::HEADER + 4, ctl);
        mem.write(n + sig::HEADER, hdr);
        let head = w + sigwork::QUEUE;
        if kind == 3 {
            atel_link_sig_work_buttom(mem, head, n);
        } else {
            let mut at = mem.read::<u32>(head);
            if at == 0 {
                mem.write(head, n);
                mem.write(n + sig::PREV, head);
                mem.write(n + sig::NEXT, 0u32);
            } else {
                let b = b & 0xf;
                loop {
                    let next = mem.read::<u32>(at);
                    if mem.read::<u8>(at + sig::KIND) == 3
                        || mem.read::<u8>(at + sig::LEVEL) & 0xf < b
                    {
                        atel_link_sig_work_prev(mem, at, n);
                        break;
                    }
                    if next == 0 {
                        atel_link_sig_work_next(mem, at, n);
                        break;
                    }
                    at = next;
                }
            }
        }
    }
    let c = CONTEXT.get(mem);
    if mem.read::<u16>(c + ctrl::ID) == 0 && from != 0xffff {
        let sender = atel_get_basic_work(mem, from.cast_signed());
        if mem.read::<u16>(sender + worker::FLAGS2) & 0x1000 != 0 {
            let f = mem.read::<u16>(w + worker::FLAGS2) & 0xfbff | 0x200;
            mem.write(w + worker::FLAGS2, f);
        }
    }
    1
}

/// `AtelRegistSignal(from, to, kind, level, entry)` (`0x0086e990`).
pub fn atel_regist_signal(mem: &mut impl Mem, r: Request) -> u32 {
    if r.0[1] == 0xffff {
        return 0;
    }
    atel_regist_signal_main(mem, r)
}

/// `AtelRegistSignal2(request)` (`0x0086e9e0`): the same from a request in
/// memory.
pub fn atel_regist_signal2(mem: &mut impl Mem, at: u32) -> u32 {
    let r = Request::read(mem, at);
    atel_regist_signal(mem, r)
}

/// `AtelRegistSignalWithoutSameLevel2(request)` (`0x0086ebe0`): registers
/// unless the receiver has a request queued at that level already.
pub fn atel_regist_signal_without_same_level(mem: &mut impl Mem, r: Request) -> u32 {
    without(mem, r, |mem, n| {
        u32::from(mem.read::<u8>(n + sig::LEVEL) & 0xf) == r.0[3]
    })
}

/// `AtelRegistSignalWithoutSameTag2(request)` (`0x0086ec80`): registers
/// unless the receiver has a request for that entry queued already.
pub fn atel_regist_signal_without_same_tag(mem: &mut impl Mem, r: Request) -> u32 {
    without(mem, r, |mem, n| {
        u32::from(mem.read::<u16>(n + sig::ENTRY)) == r.0[4]
    })
}

fn without<M: Mem>(mem: &mut M, r: Request, same: impl Fn(&M, u32) -> bool) -> u32 {
    if r.0[1] == 0xffff {
        return 0;
    }
    let w = atel_get_basic_work(mem, r.0[1].cast_signed());
    let mut n = mem.read::<u32>(w + sigwork::QUEUE);
    while n != 0 {
        if mem.read::<u8>(n + sig::KIND) != 3 && same(mem, n) {
            return 0;
        }
        n = mem.read(n);
    }
    atel_regist_signal_main(mem, r)
}

/// A reply from worker `w` to the sender of node `n`, for its `level` and
/// entry (none when it had no sender).
fn reply(mem: &mut impl Mem, w: u32, n: u32, lvl: u32) {
    let from = u32::from(mem.read::<u16>(w + worker::ID));
    let to = u32::from(mem.read::<u16>(n + sig::FROM));
    let entry = u32::from(mem.read::<u16>(n + sig::ENTRY));
    if to != 0xffff {
        atel_regist_signal_main(mem, Request::new(from, to, 3, lvl, entry));
    }
}

/// `AtelCancelSignals(worker, level, mode)` (`0x00872860`): drops the
/// queued signals above `level` (with `mode` 2 only those already looked
/// at), replying where the sender waits: a `REQSW` sender once its request
/// has started (state 1), a `REQEW` one unless `mode` is 1. With `mode` 1
/// a running (state 2) signal stays.
pub fn atel_cancel_signals(mem: &mut impl Mem, w: u32, lvl: i32, mode: u32) {
    let mut n = mem.read::<u32>(w + sigwork::QUEUE);
    while n != 0 {
        let next = mem.read::<u32>(n);
        let b = mem.read::<u8>(n + sig::LEVEL);
        let nl = i32::from(b & 0xf);
        if lvl.wrapping_add(1) <= nl && !(mode == 2 && b & 0x80 != 0) {
            let st = b >> 4 & 7;
            let kind = mem.read::<u8>(n + sig::KIND);
            if st == 1 && kind == 1 {
                reply(mem, w, n, nl.cast_unsigned());
            }
            if (st == 1 || st == 2) && mode != 1 && kind == 2 {
                reply(mem, w, n, nl.cast_unsigned());
            }
            if !(mode == 1 && st == 2) {
                free(mem, w, n);
                if mem.read::<u32>(w + sigwork::CURRENT) == n {
                    mem.write(w + sigwork::CURRENT, 0u32);
                }
            }
        }
        n = next;
    }
}

/// `AtelSignalReply(worker, node)` (`0x00871650`): the reply a `REQSW`
/// sender waits for, once its request starts.
pub fn atel_signal_reply(mem: &mut impl Mem, w: u32, n: u32) {
    if n != 0 && mem.read::<u8>(n + sig::KIND) == 1 {
        let lvl = u32::from(mem.read::<u8>(n + sig::LEVEL) & 0xf);
        reply(mem, w, n, lvl);
    }
}

/// `AtelSearchReplySignal(worker, level)` (`0x008635c0`): the last reply in
/// worker `w`'s queue to what `level` waits on (its `+0x2c` entry, `+0x2e`
/// worker and `+0x30` level), or 0.
pub fn atel_search_reply_signal(mem: &impl Mem, w: u32, l: u32) -> u32 {
    let entry = mem.read::<u16>(l + siglevel::WAIT_ENTRY);
    let from = mem.read::<u16>(l + siglevel::WAIT_WORKER);
    let lvl = mem.read::<u16>(l + siglevel::WAIT_LEVEL);
    let mut found = 0;
    let mut n = mem.read::<u32>(w + sigwork::QUEUE);
    while n != 0 {
        if mem.read::<u8>(n + sig::KIND) == 3
            && mem.read::<u16>(n + sig::FROM) == from
            && mem.read::<u16>(n + sig::ENTRY) == entry
            && u16::from(mem.read::<u8>(n + sig::LEVEL) & 0xf) == lvl
        {
            found = n;
        }
        n = mem.read(n);
    }
    found
}

/// `AtelFreeSignal(worker index, node)` (`0x008654c0`).
pub fn atel_free_signal(mem: &mut impl Mem, id: u32, n: u32) {
    let w = atel_get_basic_work(mem, id.cast_signed());
    if n != 0 {
        free(mem, w, n);
    }
}

/// `AtelIsSignalQueued(worker index, entry)` (`0x00863570`): 1 when a
/// request for `entry` is queued.
pub fn atel_is_signal_queued(mem: &mut impl Mem, id: u32, entry: u32) -> u32 {
    let w = atel_get_basic_work(mem, id.cast_signed());
    let mut n = mem.read::<u32>(w + sigwork::QUEUE);
    while n != 0 {
        let next = mem.read::<u32>(n);
        if mem.read::<u8>(n + sig::KIND) != 3 && u32::from(mem.read::<u16>(n + sig::ENTRY)) == entry
        {
            return 1;
        }
        n = next;
    }
    0
}

/// `AtelChangeBinaryHeader(worker index, block, b, header)` (`0x00862cc0`):
/// points the worker at header `header` of `block` and clears its next
/// eight words.
pub fn atel_change_binary_header(mem: &mut impl Mem, id: u32, block: u32, b: u32, hdr: u32) {
    let w = atel_get_basic_work(mem, id.cast_signed());
    let off = mem.read::<u32>(block.wrapping_add(0x38).wrapping_add(hdr.wrapping_mul(4)));
    mem.write(w + worker::BLOCK, block);
    mem.write(w + 8, b);
    mem.write(w + worker::HEADER, off.wrapping_add(block));
    for i in 3..11 {
        mem.write(w + 4 * i, 0u32);
    }
}

/// The level's header from its ctrl work's tables, when that ctrl work has
/// them (`+0x1f8`, `+0x1fc`).
fn level_header(mem: &mut impl Mem, w: u32, l: u32) {
    let c = CTRL_WORKS.ptr(mem).addr + u32::from(mem.read::<u16>(l + siglevel::CTRL)) * ctrl::SIZE;
    let blocks = mem.read::<u32>(c + 0x1f8);
    if blocks == 0 {
        return;
    }
    let t = u32::from(mem.read::<u16>(l + siglevel::TABLE));
    let block = mem.read::<u32>(blocks + t * 4);
    let second = mem.read::<u32>(c + 0x1fc);
    let b = if second == 0 {
        0
    } else {
        mem.read(second + t * 4)
    };
    let id = u32::from(mem.read::<u16>(w + worker::ID));
    let hdr = u32::from(mem.read::<u16>(l + siglevel::HEADER));
    atel_change_binary_header(mem, id, block, b, hdr);
}

/// `AtelChangeLevelHeader(worker, level)` (`0x00862c60`).
pub fn atel_change_level_header(mem: &mut impl Mem, w: u32, l: u32) {
    level_header(mem, w, l);
}

/// The code address of entry point `entry` of worker `w`.
fn entry_pc(mem: &impl Mem, w: u32, entry: u32) -> u32 {
    let h = mem.read::<u32>(w + worker::HEADER);
    let block = mem.read::<u32>(w + worker::BLOCK);
    let table = mem.read::<u32>(h + 0x20);
    let off = mem.read::<u32>(table.wrapping_add(entry * 4).wrapping_add(block));
    mem.read::<u32>(block + 0x30)
        .wrapping_add(block)
        .wrapping_add(off)
}

fn kind_of(mem: &impl Mem, w: u32) -> u8 {
    mem.read(mem.read::<u32>(w + worker::HEADER))
}

/// `AtelInitLevelMove(worker, level, n)` (`0x00863750`): for type 1
/// workers, points the level at its move and turn blocks and resets them
/// from the worker's position.
pub fn atel_init_level_move(mem: &mut impl Mem, w: u32, l: u32, n: u32) {
    let k = kind_of(mem, w);
    let pos = if k == 4 {
        0
    } else if is_small(k) {
        w + worker::POS_SMALL
    } else {
        w + worker::POS
    };
    if k != 1 {
        return;
    }
    let mv = (w + 0x5b8).wrapping_add(n.wrapping_mul(level::SIZE));
    let turn = w.wrapping_add(n.wrapping_mul(0x11).wrapping_add(0x219).wrapping_mul(4));
    mem.write(l + siglevel::TURN, turn);
    mem.write(l + siglevel::MOVE, mv);
    mem.write(mv, 0u16);
    mem.write(turn, 0u16);
    mem.write(mv + 2, 0u16);
    mem.write(turn + 2, 0u16);
    for off in [8, 0xc, 0x14] {
        mem.write(mv + off, 0u32);
    }
    // fld m32 / fstp m32 of the position: a signalling NaN comes out quiet
    let p: [u32; 3] = core::array::from_fn(|i| {
        crate::vm::through_x87(mem.read(pos + 4 * u32::try_from(i).expect("3")))
    });
    for (i, &v) in (0..).zip(&p) {
        mem.write(mv + 0x1c + 4 * i, v);
        mem.write(mv + 0x28 + 4 * i, v);
    }
    for off in [0x34, 0x18, 0x44, 0x48] {
        mem.write(mv + off, 0u32);
    }
    for off in [0x20, 0x24, 0x28, 0x2c, 0x30, 0x34] {
        mem.write(turn + off, 0u32);
    }
}

/// `AtelResetLevel(worker, level)` (`0x008639e0`): level `level` back to
/// entry 0 of its header: call depth, PC, running flag and state cleared,
/// the float accumulator 0.0, type 1 workers' move blocks reset.
pub fn atel_reset_level(mem: &mut impl Mem, w: u32, n: u32) {
    let k = kind_of(mem, w);
    let l = w + worker::LEVELS + level_index(k, n.cast_signed()) * level::SIZE;
    mem.write(l + level::CALL_DEPTH, 0u8);
    level_header(mem, w, l);
    let pc = entry_pc(mem, w, 0);
    mem.write(l + siglevel::RUNNING, 0u8);
    mem.write(l + level::PC, pc);
    mem.write(l + level::STATE, 0u8);
    if kind_of(mem, w) == 1 {
        atel_init_level_move(mem, w, l, n);
    }
    mem.write(l + level::FACC, 0u32);
}

/// `AtelStartSignal(worker, node)` (`0x008690a0`): starts the request in
/// node `n` at its level: the worker's entry and level, the level's call
/// depth, header (from the signal), move blocks and PC.
pub fn atel_start_signal(mem: &mut impl Mem, w: u32, n: u32) {
    if n == 0 {
        return;
    }
    let entry = mem.read::<u16>(n + sig::ENTRY);
    let lv = mem.read::<u8>(n + sig::LEVEL) & 0xf;
    let k = kind_of(mem, w);
    let l = w + worker::LEVELS + level_index(k, i32::from(lv)) * level::SIZE;
    mem.write(w + sigwork::ENTRY, entry);
    mem.write(w + worker::LEVEL, lv);
    mem.write(l + level::CALL_DEPTH, 0u8);
    let table = mem.read::<u16>(n + sig::HEADER + 2);
    mem.write(l + siglevel::TABLE, table);
    let hdr = mem.read::<u16>(n + sig::HEADER);
    mem.write(l + siglevel::HEADER, hdr);
    let ctl = mem.read::<u16>(n + sig::HEADER + 4);
    mem.write(l + siglevel::CTRL, ctl);
    atel_init_level_move(mem, w, l, u32::from(lv));
    level_header(mem, w, l);
    let pc = entry_pc(mem, w, u32::from(entry));
    mem.write(l + siglevel::RUNNING, 0u8);
    mem.write(l + level::PC, pc);
}

/// `MsBattleCheck` (`0x00780c90`), owned by the battle module.
pub const MS_BATTLE_CHECK: u32 = 0x0078_0c90;

/// `AtelDecodeSignal(worker index, level)` (`0x008653f0`): when the worker
/// runs (flags as `Script_Run` checks them) and takes signals (`+0x34` bit
/// 3), starts the head of its queue if it is a request above `level` (a
/// negative `level` is the worker's own, past 9 none): a queued one through
/// [`atel_start_signal`] (replying to a `REQSW` sender, state 2), a running
/// one by switching to its level. Then marks every queued node seen.
pub fn atel_decode_signal(host: &mut impl Host, id: u32, lvl: i32) {
    let w = atel_get_basic_work(host, id.cast_signed());
    let f = host.read::<u16>(w + worker::FLAGS);
    if f & 2 == 0 {
        return;
    }
    if f & 4 == 0 && host.read::<u8>(w + worker::FLAGS2) & 0x40 == 0 {
        let b = host.rebase(MS_BATTLE_CHECK);
        if host.call(b, &[]) != 0 {
            return;
        }
    }
    if host.read::<u8>(w + worker::FLAGS) & 8 == 0 {
        return;
    }
    let head = host.read::<u32>(w + sigwork::QUEUE);
    let mut lvl = if lvl < 0 {
        i32::from(host.read::<u8>(w + worker::LEVEL))
    } else {
        lvl
    };
    if lvl > 9 {
        lvl = -1;
    }
    if head != 0 && host.read::<u8>(head + sig::KIND) != 3 {
        let b = host.read::<u8>(head + sig::LEVEL);
        if lvl < i32::from(b & 0xf) {
            if b & 0x70 == 0x10 {
                atel_start_signal(host, w, head);
                atel_signal_reply(host, w, head);
                let b = host.read::<u8>(head + sig::LEVEL) & 0xaf | 0x20;
                host.write(head + sig::LEVEL, b);
            } else {
                host.write(w + worker::LEVEL, b & 0xf);
                let l = script_worker_level(host, w, i32::from(b & 0xf));
                atel_change_level_header(host, w, l);
            }
            host.write(w + sigwork::CURRENT, head);
        }
    }
    let mut n = host.read::<u32>(w + sigwork::QUEUE);
    while n != 0 {
        let b = host.read::<u8>(n + sig::LEVEL) & 0x7f;
        host.write(n + sig::LEVEL, b);
        n = host.read(n);
    }
}

/// `AtelExecReturn(worker index, level, reset)` (`0x008691d0`): the end of
/// the request a worker runs: replies to a `REQEW` sender, frees the node,
/// drops the signals above `level` (when not negative), starts the next
/// (from level 10, so any), and with `reset` puts the worker's level back
/// to entry 0. Sets worker `+0x34` bit 4.
pub fn atel_exec_return(host: &mut impl Host, id: u32, lvl: u32, reset: u32) {
    let w = atel_get_basic_work(host, id.cast_signed());
    let cur = host.read::<u32>(w + sigwork::CURRENT);
    if cur != 0 && host.read::<u8>(cur + sig::KIND) == 2 {
        let l = u32::from(host.read::<u8>(cur + sig::LEVEL) & 0xf);
        reply(host, w, cur, l);
    }
    let cur = host.read::<u32>(w + sigwork::CURRENT);
    if cur != 0 {
        free(host, w, cur);
    }
    host.write(w + sigwork::CURRENT, 0u32);
    if lvl.cast_signed() >= 0 {
        atel_cancel_signals(host, w, lvl.cast_signed(), 2);
    }
    atel_decode_signal(host, id, 10);
    if reset != 0 {
        let n = host.read::<u8>(w + worker::LEVEL);
        let k = kind_of(host, w);
        let l = w + worker::LEVELS + level_index(k, i32::from(n)) * level::SIZE;
        atel_reset_level(host, w, u32::from(n));
        level_header(host, w, l);
        let pc = entry_pc(host, w, 1);
        host.write(l + siglevel::RUNNING, 0u8);
        host.write(l + level::PC, pc);
    }
    let f = host.read::<u16>(w + worker::FLAGS) | 0x10;
    host.write(w + worker::FLAGS, f);
}

/// `AtelFindActorByChar(chr)` (`0x0086a470`): the first actor of the
/// current ctrl work bound to character `chr` (worker `+0x40`), or -1.
pub fn atel_find_actor_by_char(mem: &mut impl Mem, chr: u32) -> u32 {
    let c = CONTEXT.get(mem);
    let actors = i32::from(mem.read::<u16>(c + ctrl::ACTORS));
    for i in 0..actors {
        let w = atel_get_basic_work(mem, i);
        if mem.read::<u32>(w + worker::BIND_PC) == chr {
            return i.cast_unsigned();
        }
    }
    u32::MAX
}

/// `g_SaveInParty` (`MsGetSavePartyMember`): the battle party's three
/// characters, `0xff` none.
pub const SAVE_IN_PARTY: ffx_bridge::Global<u8> = ffx_bridge::Global::new(0x0113_07e8);

/// `AtelGetPartyWorker(slot)` (`0x0086a5e0`): the actor of party slot
/// `slot`, -1 for an empty slot or `slot` 3 and up. A negative slot makes
/// the original read below its local array; here it is empty.
pub fn atel_get_party_worker(mem: &mut impl Mem, slot: u32) -> u32 {
    let s = slot.cast_signed();
    if !(0..3).contains(&s) {
        return u32::MAX;
    }
    let chr = SAVE_IN_PARTY.ptr(mem).at(slot).get(mem);
    if chr == 0xff {
        return u32::MAX;
    }
    atel_find_actor_by_char(mem, u32::from(chr))
}

/// `AtelRegistSignalWithoutSameLevel(from, to, kind, level, entry)`
/// (`0x0086eba0`), as opcodes `39`-`3b` call it.
pub fn atel_request(mem: &mut impl Mem, r: Request) -> u32 {
    atel_regist_signal_without_same_level(mem, r)
}

/// `FUN_0067a9b0(scenario, entry, worker, level)`, which `REQ` (and only
/// it) calls before registering: a debug hook on a sound object that can
/// spin waiting for its thread. Kept original.
pub const REQ_HOOK: u32 = 0x0067_a9b0;
/// `g_SaveScenario`, `REQ_HOOK`'s first argument.
pub const SAVE_SCENARIO: ffx_bridge::Global<u16> = ffx_bridge::Global::new(0x0112_d67c);

/// The three pops `REQ`, `REQSW` and `REQEW` start with: entry, worker,
/// level.
fn pop3(mem: &mut impl Mem, s: u32) -> (u32, u32, u32) {
    let entry = atel_pop_stack_integer(mem, s);
    let to = atel_pop_stack_integer(mem, s);
    let lvl = atel_pop_stack_integer(mem, s);
    (entry, to, lvl)
}

/// The ctrl work's `+0x60` callback, which gates the `REQ` forms `47`-`49`,
/// `4c`-`4e` and `51`-`53`.
fn gate(host: &mut impl Host, op: u8, base: u8) -> u32 {
    if op.wrapping_sub(base) < 3 {
        let f = host.read::<u32>(CONTEXT.get(host) + 0x60);
        host.call(f, &[])
    } else {
        1
    }
}

/// The registration by opcode form: `+0` any level, `+3` without the same
/// tag, else without the same level.
fn register(mem: &mut impl Mem, op: u8, forms: [u8; 4], r: Request) -> u32 {
    if op == forms[0] || op == forms[2] {
        atel_regist_signal(mem, r)
    } else if op == forms[1] || op == forms[3] {
        atel_regist_signal_without_same_tag(mem, r)
    } else {
        atel_regist_signal_without_same_level(mem, r)
    }
}

/// `Script_OpReq(op, level, worker, stack)` (`0x00867230`): opcodes `36`
/// and `45`-`49`, `REQ`: pops entry, worker and level and sends the worker
/// a request; pushes 1 when it was queued.
pub fn script_op_req(host: &mut impl Host, op: u8, w: u32, s: u32) {
    let ok = gate(host, op, 0x47);
    let (entry, to, lvl) = pop3(host, s);
    let f = host.rebase(REQ_HOOK);
    let scenario = u32::from(SAVE_SCENARIO.get(host));
    host.call(f, &[scenario, entry, to, lvl]);
    let v = if ok == 0 || entry == 0 {
        0
    } else {
        let id = u32::from(host.read::<u16>(w + worker::ID));
        register(
            host,
            op,
            [0x45, 0x46, 0x48, 0x49],
            Request::new(id, to, 0, lvl, entry),
        )
    };
    atel_push_stack_integer(host, s, v);
}

/// `Script_OpReqSw` (`0x00867570`, opcodes `37`, `4a`-`4e`, kind 1) and
/// `Script_OpReqEw` (`0x008673d0`, `38`, `4f`-`53`, kind 2): `REQ`, and the
/// level then waits (`+0x1f` 1) for the reply to the request it queued.
/// Returns 0.
pub fn script_op_req_wait(host: &mut impl Host, op: u8, w: u32, l: u32, s: u32) -> u32 {
    let (kind, base) = if op == 0x37 || (0x4a..=0x4e).contains(&op) {
        (1, 0x4a)
    } else {
        (2, 0x4f)
    };
    let ok = gate(host, op, base + 2);
    let (entry, to, lvl) = pop3(host, s);
    let v = if ok == 0 || entry == 0 {
        0
    } else {
        let id = u32::from(host.read::<u16>(w + worker::ID));
        register(
            host,
            op,
            [base, base + 1, base + 3, base + 4],
            Request::new(id, to, kind, lvl, entry),
        )
    };
    atel_push_stack_integer(host, s, v);
    if v != 0 {
        #[allow(clippy::cast_possible_truncation)] // 16-bit fields
        {
            host.write(l + siglevel::WAIT_ENTRY, entry as u16);
            host.write(l + siglevel::WAIT_WORKER, to as u16);
            host.write(l + siglevel::WAIT_LEVEL, lvl as u16);
        }
        host.write(l + level::STATE, 1u8);
    }
    0
}

/// `TkGetVibrateDefaultCurve(type)` (`0x00876390`): the default vibration
/// curve (eight `u16`) by worker type: types 2 and 3, 5, 6, and the rest.
#[must_use]
pub const fn tk_get_vibrate_default_curve(kind: u32) -> u32 {
    match kind {
        2 | 3 => 0x00c5_2ae8,
        5 => 0x00c5_2af8,
        6 => 0x00c5_2b08,
        _ => 0x00c5_2ad8,
    }
}

/// `Script_Op79(worker, var, index, value)` (`0x00870d30`): the worker's
/// vibration curve: stores the `u16` `value` at `index` (at most 7; a
/// negative one goes below) of the eight-`u16` variable `var`, which the
/// worker's `+0x44` then points at; the first time (`+0x44` null) the
/// variable is filled from its type's default curve.
pub fn script_op79(host: &mut impl Host, w: u32, var: u32, index: u32, value: u32) {
    let at = atel_get_real_var_adrs(host, w, var);
    if host.read::<u32>(w + 0x44) == 0 {
        let k = u32::from(kind_of(host, w));
        let d = host.rebase(tk_get_vibrate_default_curve(k));
        let v: [u16; 8] = host.read(d);
        host.write(at, v);
    }
    let i = if index.cast_signed() >= 8 { 7 } else { index };
    #[allow(clippy::cast_possible_truncation)] // a u16 store
    host.write(at.wrapping_add(i.wrapping_mul(2)), value as u16);
    host.write(w + 0x44, at);
}

/// `Script_Op7a(worker, key, value)` (`0x0086fab0`): types 5 and 6 only:
/// in the eight `(key, value, 3)` slots of the worker's `+0x2e4` table,
/// replaces the slot holding `key` (the last match), else takes the last
/// free one (key `0xffff` or value 0); returns 1, or 0 when there is no
/// table or no room.
pub fn script_op7a(mem: &mut impl Mem, w: u32, key: u32, value: u32) -> u32 {
    if !is_small(kind_of(mem, w)) {
        return 0;
    }
    let t = mem.read::<u32>(w + 0x2e4);
    if t == 0 {
        return 0;
    }
    let k = |i: u32| u32::from(mem.read::<u16>(t + i * 8));
    let mut i: i32 = if k(0) == key { 0 } else { -1 };
    for j in 1..7 {
        if k(j) == key {
            i = j.cast_signed();
        }
    }
    let i = if k(7) == key {
        7
    } else if i >= 0 {
        i
    } else {
        let free =
            |j: u32| mem.read::<u16>(t + j * 8) == 0xffff || mem.read::<u16>(t + j * 8 + 2) == 0;
        for j in 0..7 {
            if free(j) {
                i = j.cast_signed();
            }
        }
        if free(7) {
            7
        } else if i < 0 {
            return 0;
        } else {
            i
        }
    };
    let at = t + i.cast_unsigned() * 8;
    #[allow(clippy::cast_possible_truncation)] // u16 fields
    {
        mem.write(at, key as u16);
        mem.write(at + 2, value as u16);
    }
    mem.write(at + 4, 3u8);
    1
}
