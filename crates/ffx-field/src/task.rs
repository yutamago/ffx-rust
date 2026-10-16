//! `rcef`: the field effects' tasks and objects (`docs/formats/field.md`).
//!
//! A task runs a stack of sequence functions: each frame
//! `rcefTaskProc` calls the one on top (again while it sets the
//! re-run flag), then the task's after-function. A sequence calls a
//! sub-sequence (`rcefTaskCallSeq`), replaces itself (`rcefTaskSetSeq`) or
//! returns to its caller (`rcefTaskRetSeqCont`). Each stack level has 16
//! bytes of arguments (the wait counter of `rcefTaskWait`).
//!
//! A controller holds the tasks in a doubly linked list between a head
//! (priority 0) and a tail (priority `0xffff`), sorted by priority. An
//! effect object is a task with a transform, an attached character, a
//! particle part and an animation.

/// Offsets in a task (`rcefTaskInit`, `0xbc` bytes).
pub mod header {
    /// `+0x0`: the flags ([`super::ALIVE`], [`super::RERUN`], [`super::DEAD`]).
    pub const FLAGS: u32 = 0x0;
    /// `+0x4`: the previous task.
    pub const PREV: u32 = 0x4;
    /// `+0x8`: the next task.
    pub const NEXT: u32 = 0x8;
    /// `+0xc`: the sequence functions, one per stack level.
    pub const SEQ: u32 = 0xc;
    /// `+0x2c`: 16 bytes of arguments per stack level.
    pub const ARGS: u32 = 0x2c;
    /// Bytes of arguments per level.
    pub const ARGS_SIZE: u32 = 0x10;
    /// `+0xac`: the stack level (`i32`, `0..=MAX_LEVEL`).
    pub const LEVEL: u32 = 0xac;
    /// `+0xb0`: called after the sequences each frame unless the task is
    /// dead (`rcefObjProc` for objects).
    pub const AFTER: u32 = 0xb0;
    /// `+0xb4`: called when the task is killed (`rcefTaskKill`).
    pub const DESTRUCTOR: u32 = 0xb4;
    /// `+0xb8`: the priority (`i32`): the list is sorted by it.
    pub const PRIORITY: u32 = 0xb8;
    /// Size.
    pub const SIZE: u32 = 0xbc;
}

/// Flag bit 0: set by `rcefTaskInit`.
pub const ALIVE: u32 = 1;
/// Flag bit 1: run the sequence again this frame (`rcefTaskProc`).
pub const RERUN: u32 = 2;
/// Flag bit 2: kill the task after this frame (`rcefTaskCtrlProc`); also
/// skips its after-function.
pub const DEAD: u32 = 4;
/// The deepest stack level (`rcefTaskCallSeq`): eight levels.
pub const MAX_LEVEL: i32 = 7;
/// A new task's priority.
pub const DEFAULT_PRIORITY: u32 = 0x8000;
/// The original's empty destructor, a new task's (`0x0092ed00`).
pub const NO_DESTRUCTOR: u32 = 0x0092_ed00;
/// `rcefTaskWaitSeq` (`0x0092ee90`), the sequence `rcefTaskWait` calls.
pub const WAIT_SEQ: u32 = 0x0092_ee90;

/// Offsets in a controller (`rcefTaskCtrlInit`).
pub mod ctrl {
    /// `+0x0`: the head task (priority 0).
    pub const HEAD: u32 = 0x0;
    /// `+0xbc`: the tail task (priority `0xffff`).
    pub const TAIL: u32 = super::header::SIZE;
    /// The tail's priority.
    pub const TAIL_PRIORITY: u32 = 0xffff;
    /// `+0x178`: cleared by `rcefTaskCtrlInit`.
    pub const FIELD_178: u32 = 0x178;
}

/// Offsets in an effect object (`rcefObjInit`, `0x490` bytes).
pub mod obj {
    /// `+0xbc`: the mode flags: bit 1 attached to a character
    /// (`rcefObjAttachChr`, which clears bit 2), bits 4..=11 the mode
    /// (`rcefObjSetMode`).
    pub const MODE: u32 = 0xbc;
    /// Mode bit 1: attached to a character.
    pub const ATTACHED: u32 = 0x2;
    /// Mode bit 2: cleared by `rcefObjAttachChr`.
    pub const DETACHED_BIT: u32 = 0x4;
    /// Bits `rcefObjClearMode` clears.
    pub const MODE_MASK: u32 = 0x0000_0ff0;
    /// `+0x270`: the attached character.
    pub const CHR: u32 = 0x270;
    /// `+0x274`: the element (bone) of the character.
    pub const CHR_ELEMENT: u32 = 0x274;
    /// `+0x280`: the particle part (`rcefParInit`, `0xb0` bytes); its first
    /// byte's bit 0 while it runs.
    pub const PAR: u32 = 0x280;
    /// `+0x330`: the animation (`rcefAnimeInit`, `0x90` bytes); its first
    /// byte's bit 0 while it runs.
    pub const ANIME: u32 = 0x330;
}

/// Offsets in a particle part (`rcefParInit`).
pub mod par {
    /// Size.
    pub const SIZE: u32 = 0xb0;
    /// The words `rcefParInit` sets after clearing it: four to 1.0, one to
    /// the integer 1, one to 0.0 (which clearing already did).
    pub const INIT: [(u32, u32); 6] = [
        (0x5c, 0x3f80_0000),
        (0x9c, 0x3f80_0000),
        (0x6c, 1),
        (0xa0, 0x3f80_0000),
        (0xa4, 0x3f80_0000),
        (0xa8, 0),
    ];
}

/// Offsets in an animation (`rcefAnimeInit`).
pub mod anime {
    /// Size.
    pub const SIZE: u32 = 0x90;
    /// `+0x0`: 1 while it runs (`rcefAnimeStart`).
    pub const RUNNING: u32 = 0x0;
    /// `+0x4`: the animation data (`rcefAnimeStart`).
    pub const DATA: u32 = 0x4;
    /// `+0x20`: a pointer to [`WORK`].
    pub const CURSOR: u32 = 0x20;
    /// `+0x44`: `u16`, `0x200` after `rcefAnimeInit`.
    pub const RATE: u32 = 0x44;
    /// `+0x48`, `+0x4c`: the two outputs `rcefAnimeStart` is given (the
    /// object's matrices at `+0x1f0` and `+0x170` in `0x00925590`).
    pub const OUT: [u32; 2] = [0x48, 0x4c];
    /// `+0x50`: its work area.
    pub const WORK: u32 = 0x50;
}

/// `3π/2`, the original's `.rdata` double (`0x00b67e18`; the `f32`
/// nearest it, widened).
pub const THREE_HALF_PI: ffx_bridge::Global<f64> = ffx_bridge::Global::new(0x00b6_7e18);

/// The stack level after `rcefTaskCallSeq`, or `None` when the stack is
/// full (nothing changes).
#[must_use]
pub fn call_level(level: i32) -> Option<i32> {
    (level < MAX_LEVEL).then(|| level + 1)
}

/// The stack level after a return (`rcefTaskRetSeqCont_Inner`): one less,
/// never below 0.
#[must_use]
pub fn ret_level(level: i32) -> i32 {
    if level > 0 { level - 1 } else { level }
}

/// Where a level's sequence function is.
#[must_use]
#[allow(clippy::cast_sign_loss)] // the original indexes with the raw level
pub fn seq_slot(t: u32, level: i32) -> u32 {
    t.wrapping_add(header::SEQ)
        .wrapping_add((level as u32).wrapping_mul(4))
}

/// Where a level's arguments are.
#[must_use]
#[allow(clippy::cast_sign_loss)] // as above
pub fn args_slot(t: u32, level: i32) -> u32 {
    t.wrapping_add(header::ARGS)
        .wrapping_add((level as u32).wrapping_mul(header::ARGS_SIZE))
}

/// The tasks and objects where the game keeps them, through
/// [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    use super::{
        ALIVE, DEFAULT_PRIORITY, NO_DESTRUCTOR, RERUN, WAIT_SEQ, anime, args_slot, call_level,
        ctrl, header, obj, par, ret_level, seq_slot,
    };

    /// The first task `rcEffectTaskTaskWait` compares with (read only there).
    pub const TASK_WAIT_A: Global<u32> = Global::new(0x025d_5a44);
    /// The second one.
    pub const TASK_WAIT_B: Global<u32> = Global::new(0x025d_5afc);
    /// What `rcEffectTaskFlagWait` waits to be 0 (read only there).
    pub const FLAG_WAIT: Global<u32> = Global::new(0x025d_5c10);

    fn level(mem: &impl Mem, t: u32) -> i32 {
        mem.read(t + header::LEVEL)
    }

    fn set_flags(mem: &mut impl Mem, t: u32, bits: u32) {
        let f = mem.read::<u32>(t + header::FLAGS);
        mem.write(t + header::FLAGS, f | bits);
    }

    /// `rcefTaskInit(task)` (`0x0092ed10`): cleared, alive, the empty
    /// destructor and the default priority.
    pub fn rcef_task_init(mem: &mut impl Mem, t: u32) {
        mem.write_bytes(t, &[0; header::SIZE as usize]);
        mem.write(t + header::FLAGS, ALIVE);
        let none = mem.rebase(NO_DESTRUCTOR);
        mem.write(t + header::DESTRUCTOR, none);
        mem.write(t + header::PRIORITY, DEFAULT_PRIORITY);
    }

    /// `rcefTaskSetSeq(task, seq)` (`0x0092ee20`): replaces the current
    /// level's sequence.
    pub fn rcef_task_set_seq(mem: &mut impl Mem, t: u32, seq: u32) {
        let l = level(mem, t);
        mem.write(seq_slot(t, l), seq);
    }

    /// `rcefTaskSetSeqCont(task, seq)` (`0x0092ee40`): replaces it and runs
    /// the new one this frame.
    pub fn rcef_task_set_seq_cont(mem: &mut impl Mem, t: u32, seq: u32) {
        rcef_task_set_seq(mem, t, seq);
        set_flags(mem, t, RERUN);
    }

    /// `rcefTaskCallSeq(task, seq)` (`0x0092ecb0`): pushes a sequence
    /// (nothing when all eight levels are taken).
    pub fn rcef_task_call_seq(mem: &mut impl Mem, t: u32, seq: u32) {
        if let Some(l) = call_level(level(mem, t)) {
            mem.write(t + header::LEVEL, l);
            mem.write(seq_slot(t, l), seq);
        }
    }

    /// `rcefTaskRetSeqCont_Inner(task)` (`0x0092ede0`): pops a level.
    pub fn rcef_task_ret_seq_cont_inner(mem: &mut impl Mem, t: u32) {
        let l = level(mem, t);
        if l > 0 {
            mem.write(t + header::LEVEL, ret_level(l));
        }
    }

    /// `rcefTaskRetSeqCont(task)` (`0x0092ee00`): pops a level and runs the
    /// caller this frame.
    pub fn rcef_task_ret_seq_cont(mem: &mut impl Mem, t: u32) {
        rcef_task_ret_seq_cont_inner(mem, t);
        set_flags(mem, t, RERUN);
    }

    /// `rcefTaskWait(task, frames)` (`0x0092ee60`): calls `rcefTaskWaitSeq`
    /// with the count in the new level's arguments (none for 0 or less).
    pub fn rcef_task_wait(mem: &mut impl Mem, t: u32, frames: i32) {
        if frames > 0 {
            let seq = mem.rebase(WAIT_SEQ);
            rcef_task_call_seq(mem, t, seq);
            let l = level(mem, t);
            mem.write(args_slot(t, l), frames);
        }
    }

    /// `rcefTaskWaitSeq(task)` (`0x0092ee90`): counts a frame down; returns
    /// at 0.
    pub fn rcef_task_wait_seq(mem: &mut impl Mem, t: u32) {
        let at = args_slot(t, level(mem, t));
        let n = mem.read::<i32>(at).wrapping_sub(1);
        mem.write(at, n);
        // read again, level included, as the original does
        if mem.read::<i32>(args_slot(t, level(mem, t))) < 1 {
            rcef_task_ret_seq_cont(mem, t);
        }
    }

    /// `rcefTaskCtrlInit(ctrl)` (`0x00930290`): head and tail linked.
    pub fn rcef_task_ctrl_init(mem: &mut impl Mem, c: u32) {
        let (head, tail) = (c + ctrl::HEAD, c + ctrl::TAIL);
        rcef_task_init(mem, head);
        rcef_task_init(mem, tail);
        mem.write(head + header::NEXT, tail);
        mem.write(head + header::PRIORITY, 0u32);
        mem.write(tail + header::PRIORITY, ctrl::TAIL_PRIORITY);
        mem.write(tail + header::PREV, head);
        mem.write(c + ctrl::FIELD_178, 0u32);
    }

    /// `rcefTaskCtrlIsEnd(ctrl)` (`0x009302e0`): whether no task is left
    /// between head and tail.
    pub fn rcef_task_ctrl_is_end(mem: &impl Mem, c: u32) -> bool {
        mem.read::<u32>(c + ctrl::HEAD + header::NEXT) == c + ctrl::TAIL
    }

    /// `rcefTaskCtrlAdd(first, task)` (`0x00930220`): links `task` before
    /// the first task from `first` on with a greater priority (nothing when
    /// there is none, or `first` is null).
    pub fn rcef_task_ctrl_add(mem: &mut impl Mem, first: u32, t: u32) {
        let mut at = first;
        if at == 0 {
            return;
        }
        let p = mem.read::<i32>(t + header::PRIORITY);
        while mem.read::<i32>(at + header::PRIORITY) <= p {
            at = mem.read(at + header::NEXT);
            if at == 0 {
                return;
            }
        }
        let prev = mem.read::<u32>(at + header::PREV);
        mem.write(prev + header::NEXT, t);
        let prev = mem.read::<u32>(at + header::PREV);
        mem.write(t + header::PREV, prev);
        mem.write(at + header::PREV, t);
        mem.write(t + header::NEXT, at);
    }

    /// `rcefObjIsParEnd(obj)` (`0x00930010`): whether its particle part
    /// stopped.
    pub fn rcef_obj_is_par_end(mem: &impl Mem, o: u32) -> bool {
        mem.read::<u8>(o + obj::PAR) & 1 == 0
    }

    /// `rcefObjIsAnimeEnd(obj)` (`0x0092fff0`): whether its animation
    /// stopped.
    pub fn rcef_obj_is_anime_end(mem: &impl Mem, o: u32) -> bool {
        mem.read::<u8>(o + obj::ANIME) & 1 == 0
    }

    /// `rcefObjAttachChr(obj, chr, element)` (`0x0092f970`).
    pub fn rcef_obj_attach_chr(mem: &mut impl Mem, o: u32, chr: u32, element: u32) {
        let m = mem.read::<u32>(o + obj::MODE);
        mem.write(o + obj::MODE, m & !obj::DETACHED_BIT | obj::ATTACHED);
        mem.write(o + obj::CHR, chr);
        mem.write(o + obj::CHR_ELEMENT, element);
    }

    /// `rcefObjClearMode(obj)` (`0x009301e0`).
    pub fn rcef_obj_clear_mode(mem: &mut impl Mem, o: u32) {
        let m = mem.read::<u32>(o + obj::MODE);
        mem.write(o + obj::MODE, m & !obj::MODE_MASK);
    }

    /// `rcefObjSetMode(obj, mode)` (`0x00930200`): the mode bits replaced
    /// (`mode` is or-ed in, unmasked).
    pub fn rcef_obj_set_mode(mem: &mut impl Mem, o: u32, mode: u32) {
        rcef_obj_clear_mode(mem, o);
        let m = mem.read::<u32>(o + obj::MODE);
        mem.write(o + obj::MODE, m | mode);
    }

    /// `rcefParInit(par)` (`0x0092efd0`).
    pub fn rcef_par_init(mem: &mut impl Mem, p: u32) {
        mem.write_bytes(p, &[0; par::SIZE as usize]);
        for (o, v) in par::INIT {
            mem.write(p + o, v);
        }
    }

    /// `rcefAnimeInit(anime)` (`0x0092f4c0`).
    pub fn rcef_anime_init(mem: &mut impl Mem, a: u32) {
        mem.write_bytes(a, &[0; anime::SIZE as usize]);
        mem.write(a + anime::RATE, 0x200u16);
    }

    /// `rcefAnimeStart(anime, data, out0, out1)` (`0x0092f470`).
    pub fn rcef_anime_start(mem: &mut impl Mem, a: u32, data: u32, out0: u32, out1: u32) {
        mem.write(a + anime::DATA, data);
        mem.write(a + anime::OUT[0], out0);
        mem.write(a + anime::OUT[1], out1);
        mem.write(a + anime::RUNNING, 1u32);
        mem.write(a + anime::CURSOR, a + anime::WORK);
    }

    /// `rcefModelInit(model)` (`0x0092f750`): an object's model part,
    /// `0xd0` bytes, cleared.
    pub fn rcef_model_init(mem: &mut impl Mem, m: u32) {
        mem.write_bytes(m, &[0; 0xd0]);
    }

    /// `rcefIsBattle()` (`0x0092e620`): whether a battle runs
    /// (`MsBattleGetStatus` non-zero).
    pub fn rcef_is_battle(mem: &impl Mem) -> bool {
        ffx_battle::state::bridge::ms_battle_get_status(mem) != 0
    }

    /// `rcefDirToAngle(dir)` (`0x0092de30`): `3π/2 - dir`, as `f32`; the
    /// float as bits.
    pub fn rcef_dir_to_angle(mem: &impl Mem, dir: u32) -> f32 {
        let k = super::THREE_HALF_PI.get(mem);
        f32::from_bits(crate::x87::store(k - crate::x87::load(dir)))
    }

    /// `rcEffectTaskParWait(obj)` (`0x00930490`): a sequence that returns
    /// once the particle part stopped.
    pub fn rc_effect_task_par_wait(mem: &mut impl Mem, o: u32) {
        if rcef_obj_is_par_end(mem, o) {
            rcef_task_ret_seq_cont(mem, o);
        }
    }

    /// `rcEffectTaskAnimeWait(obj)` (`0x00930350`): returns once the
    /// animation stopped.
    pub fn rc_effect_task_anime_wait(mem: &mut impl Mem, o: u32) {
        if rcef_obj_is_anime_end(mem, o) {
            rcef_task_ret_seq_cont(mem, o);
        }
    }

    /// `rcEffectTaskTaskWait(task)` (`0x00930520`): returns once both
    /// [`TASK_WAIT_A`] and [`TASK_WAIT_B`] are this task.
    pub fn rc_effect_task_task_wait(mem: &mut impl Mem, t: u32) {
        if TASK_WAIT_A.get(mem) == t && TASK_WAIT_B.get(mem) == t {
            rcef_task_ret_seq_cont(mem, t);
        }
    }

    /// `rcEffectTaskFlagWait(task)` (`0x009303c0`): returns once
    /// [`FLAG_WAIT`] is 0.
    pub fn rc_effect_task_flag_wait(mem: &mut impl Mem, t: u32) {
        if FLAG_WAIT.get(mem) == 0 {
            rcef_task_ret_seq_cont(mem, t);
        }
    }
}

#[cfg(test)]
mod tests {
    use ffx_bridge::{Mem, VecMem};

    use super::*;

    #[test]
    fn the_stack_holds_eight_levels() {
        let mut m = VecMem::new(0x1000, 0x1000);
        let t = 0x1000;
        bridge::rcef_task_init(&mut m, t);
        for i in 0..10 {
            bridge::rcef_task_call_seq(&mut m, t, 0x100 + i);
        }
        assert_eq!(m.read::<i32>(t + header::LEVEL), MAX_LEVEL);
        assert_eq!(m.read::<u32>(seq_slot(t, 7)), 0x106);
        bridge::rcef_task_ret_seq_cont(&mut m, t);
        assert_eq!(m.read::<i32>(t + header::LEVEL), 6);
        assert_eq!(m.read::<u32>(t) & RERUN, RERUN);
    }

    #[test]
    fn tasks_sort_by_priority() {
        let mut m = VecMem::new(0x1000, 0x2000);
        let c = 0x1000;
        bridge::rcef_task_ctrl_init(&mut m, c);
        assert!(bridge::rcef_task_ctrl_is_end(&m, c));
        let (t1, t2) = (0x1400, 0x1600);
        bridge::rcef_task_init(&mut m, t1);
        bridge::rcef_task_init(&mut m, t2);
        m.write(t2 + header::PRIORITY, 0x10u32);
        bridge::rcef_task_ctrl_add(&mut m, c, t1);
        bridge::rcef_task_ctrl_add(&mut m, c, t2);
        assert_eq!(m.read::<u32>(c + header::NEXT), t2);
        assert_eq!(m.read::<u32>(t2 + header::NEXT), t1);
        assert_eq!(m.read::<u32>(t1 + header::NEXT), c + ctrl::TAIL);
        assert!(!bridge::rcef_task_ctrl_is_end(&m, c));
    }
}
