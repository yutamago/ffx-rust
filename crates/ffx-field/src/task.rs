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
/// `pppMemFree(heap, spare, p)` (`0x00716a90`).
pub const PPP_MEM_FREE: u32 = 0x0071_6a90;
/// `pppMemAlloc(heap, spare, size)` (`0x00716a30`).
pub const PPP_MEM_ALLOC: u32 = 0x0071_6a30;
/// `pppRunPart(part, arg)` (`0x00712250`).
pub const PPP_RUN_PART: u32 = 0x0071_2250;
/// `Ch_TextureSetClutBase(chr, a, b)` (`0x008313c0`).
pub const CH_TEXTURE_SET_CLUT_BASE: u32 = 0x0083_13c0;
/// `Ch_TextureSetImageBase(chr, a, b)` (`0x00831390`).
pub const CH_TEXTURE_SET_IMAGE_BASE: u32 = 0x0083_1390;
/// `Ch_DrawEnEffect(chr, on)` (`0x0082eb60`).
pub const CH_DRAW_EN_EFFECT: u32 = 0x0082_eb60;
/// `Sg_LightGetColorMatrixPtr()` (`0x008361b0`).
pub const LIGHT_COLOR_MATRIX: u32 = 0x0083_61b0;
/// `Sg_LightGetNormalMatrixPtr()` (`0x00836240`).
pub const LIGHT_NORMAL_MATRIX: u32 = 0x0083_6240;

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
    /// `+0x3c0`: the model (`rcefModelInit`, `0xd0` bytes).
    pub const MODEL: u32 = 0x3c0;
    /// Size.
    pub const SIZE: u32 = 0x490;
    /// `rcefObjFini`, its destructor.
    pub const FINI: u32 = 0x0092_fee0;
    /// `rcefObjProc`, its after-function.
    pub const PROC: u32 = 0x0093_0060;
    /// The floats `rcefObjInit` sets: scales and colours 1.0 (`+0x13c`,
    /// `+0x16c` 0.0, which clearing already did).
    pub const INIT: [(u32, f32); 12] = [
        (0x130, 1.0),
        (0x134, 1.0),
        (0x138, 1.0),
        (0x13c, 0.0),
        (0x16c, 0.0),
        (0x160, 1.0),
        (0x164, 1.0),
        (0x168, 1.0),
        (0x170, 1.0),
        (0x174, 1.0),
        (0x178, 1.0),
        (0x17c, 1.0),
    ];
    /// Its three matrices (identity after `rcefObjInit`).
    pub const MATRICES: [u32; 3] = [0x1b0, 0x1f0, 0x230];
}

/// The particle parts `rcefParCreate` remembers for entry 2, by magic
/// effect id (`0x0193c0d0`..).
pub const PAR_KEEP_2: [(u32, u32); 4] = [
    (0x20e, 0x0193_c0d0),
    (0x180, 0x0193_c0d8),
    (0x168, 0x0193_c0e0),
    (0x217, 0x0193_c0e8),
];
/// The same for entry 1.
pub const PAR_KEEP_1: [(u32, u32); 2] = [(0xbf, 0x0193_c0dc), (0x216, 0x0193_c0e4)];

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

/// π, the `.rdata` double `rcAngleDiff` compares with (`0x00b92288`).
pub const PI: ffx_bridge::Global<f64> = ffx_bridge::Global::new(0x00b9_2288);
/// 2π, the `.rdata` double it wraps with (`0x00b39378`).
pub const TWO_PI: ffx_bridge::Global<f64> = ffx_bridge::Global::new(0x00b3_9378);

/// The effect targets (`0x00924a20` fills them, up to 16): `0x24` bytes
/// each.
pub mod target {
    /// The table.
    pub const TABLE: ffx_bridge::Global<u8> = ffx_bridge::Global::new(0x025d_5c60);
    /// Bytes per entry.
    pub const SIZE: u32 = 0x24;
    /// `+0x0`: flags (`u16`): bit 0 reached (`0x009248c0`), bit 1 done.
    pub const FLAGS: u32 = 0x0;
    /// `+0x8`: horizontal distance to the target.
    pub const DIST: u32 = 0x8;
    /// `+0xc`: its direction, an angle.
    pub const ANGLE: u32 = 0xc;
    /// `+0x10`: its height difference.
    pub const HEIGHT: u32 = 0x10;
    /// The delay a reached target waits (`u16`, `rcefTargetSetDelay`).
    pub const DELAY: ffx_bridge::Global<u16> = ffx_bridge::Global::new(0x025d_5ea2);
}

/// Offsets in the effect main object (`rcefMainInit`).
pub mod main {
    /// `+0x10`: its effect memory environment (the `ppp` allocator's).
    pub const ENV: u32 = 0x10;
    /// Where the current `ppp` environment pointer lives: `.data`
    /// `0x00c3a4cc` holds its address.
    pub const ENV_SLOT: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x00c3_a4cc);
}

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

/// `rcefTaskCtrlIsEnd(ctrl)` (`0x009302e0`).
pub const TASK_CTRL_IS_END: u32 = 0x0093_02e0;
/// `rcefMainCreateObj(main)` (`0x00925140`).
pub const MAIN_CREATE_OBJ: u32 = 0x0092_5140;
/// `rcefObjIsParEnd(obj)` (`0x00930010`).
pub const OBJ_IS_PAR_END: u32 = 0x0093_0010;
/// `rcefMainSetPar(main, obj, a, b)` (`0x009256d0`).
pub const MAIN_SET_PAR: u32 = 0x0092_56d0;
/// `rcefObjAttachChr(obj, chr, bone)` (`0x0092f970`).
pub const OBJ_ATTACH_CHR: u32 = 0x0092_f970;
/// `rcCopyQword_00924f90(dst, src)` (`0x00924f90`).
pub const COPY_QWORD: u32 = 0x0092_4f90;

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
    /// The pray effect `rcEffectTaskPrayWait` waits for.
    pub const PRAY_EFFECT: Global<u32> = Global::new(0x025d_5a24);
    /// What `rcEffectTaskFlagWait` waits to be 0 (read only there).
    pub const FLAG_WAIT: Global<u32> = Global::new(0x025d_5c10);
    /// The request `rcefPostRequest` stores: five words at `0x025d5c18`
    /// (the first argument, the float, the third, a zero, the fourth, the
    /// fifth; `+0x10` is the zero). Nothing in the PC build reads them.
    pub const REQUEST: u32 = 0x025d_5c18;
    /// Set by `rcefPostRequest`; `rcEffectTaskRequestWait` waits for it to
    /// be 0, and nothing in the PC build clears it.
    pub const REQUEST_PENDING: Global<u32> = Global::new(0x025d_5c34);
    /// The program index `pppRunPartStd` is running (`g_pppCurrentProgram`).
    pub const CURRENT_PROGRAM: Global<u32> = Global::new(0x0230_fd20);
    /// `graphicVFXDestroy(lo, hi)` (cdecl, a 64-bit instance handle).
    pub const VFX_DESTROY: u32 = 0x0064_5280;
    /// `graphicVFXGetRenderDataByInstance(lo, hi)` (cdecl).
    pub const VFX_GET_RENDER_DATA: u32 = 0x0064_53d0;

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

    /// `rcAngleDiff(a, b)` (`0x0092e400`): `|a - b|` as `f32`, wrapped to
    /// `2π - d` past π; floats as bits.
    pub fn rc_angle_diff(mem: &impl Mem, a: u32, b: u32) -> f32 {
        use ffx_bridge::x87;
        let d = x87::load(x87::store(x87::sub(x87::load(a), x87::load(b)).abs()));
        if d > super::PI.get(mem) {
            f32::from_bits(x87::store(x87::sub(super::TWO_PI.get(mem), d)))
        } else {
            f32::from_bits(x87::store(d))
        }
    }

    fn target_at(mem: &impl Mem, i: u32, offset: u32) -> u32 {
        super::target::TABLE
            .ptr(mem)
            .addr
            .wrapping_add(i.wrapping_mul(super::target::SIZE))
            .wrapping_add(offset)
    }

    /// `rcefTargetIsDone(i)` (`0x00924ba0`): flag bit 1 of target `i`.
    pub fn rcef_target_is_done(mem: &impl Mem, i: u32) -> u32 {
        u32::from(mem.read::<u8>(target_at(mem, i, super::target::FLAGS)) >> 1 & 1)
    }

    /// `rcefTargetClear(i)` (`0x00924bc0`): its flags.
    pub fn rcef_target_clear(mem: &mut impl Mem, i: u32) {
        let at = target_at(mem, i, super::target::FLAGS);
        mem.write(at, 0u16);
    }

    fn target_float(mem: &impl Mem, i: u32, offset: u32) -> f32 {
        f32::from_bits(mem.read(target_at(mem, i, offset)))
    }

    /// `rcefTargetGetDist(i)` (`0x00924a00`).
    pub fn rcef_target_get_dist(mem: &impl Mem, i: u32) -> f32 {
        target_float(mem, i, super::target::DIST)
    }

    /// `rcefTargetGetAngle(i)` (`0x009249e0`).
    pub fn rcef_target_get_angle(mem: &impl Mem, i: u32) -> f32 {
        target_float(mem, i, super::target::ANGLE)
    }

    /// `rcefTargetGetHeight(i)` (`0x009249c0`).
    pub fn rcef_target_get_height(mem: &impl Mem, i: u32) -> f32 {
        target_float(mem, i, super::target::HEIGHT)
    }

    /// `rcefTargetSetDelay(frames)` (`0x00924be0`).
    pub fn rcef_target_set_delay(mem: &mut impl Mem, frames: u16) {
        super::target::DELAY.set(mem, frames);
    }

    /// `rcefMainSetEnv(main)` (`0x009254f0`, an identical copy at
    /// `0x00925510`): makes the effect's memory environment current.
    pub fn rcef_main_set_env(mem: &mut impl Mem, m: u32) {
        let env = mem.read::<u32>(m + super::main::ENV);
        let slot = super::main::ENV_SLOT.get(mem);
        mem.write(slot, env);
    }

    /// `rcefObjLinkLast(obj, child)` (`0x0092f920`): `child` after the last
    /// object of `obj`'s chain (`+0xcc` next, `+0xc8` previous).
    pub fn rcef_obj_link_last(mem: &mut impl Mem, o: u32, child: u32) {
        let mut last = o;
        loop {
            let next = mem.read::<u32>(last + 0xcc);
            if next == 0 {
                break;
            }
            last = next;
        }
        mem.write(last + 0xcc, child);
        mem.write(child + 0xc8, last);
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

    // ---- round 21 ----

    /// `rcefPostRequest(a, f, b, c, d)` (`0x0092eba0`, an exported effect
    /// import; name ours): stores the request at [`REQUEST`] (the float
    /// copied through the x87 stack, so a signalling NaN turns quiet) and
    /// sets [`REQUEST_PENDING`]. Returns `d` (left in `eax`).
    pub fn rcef_post_request(mem: &mut impl Mem, args: [u32; 5]) -> u32 {
        let [a, f, b, c, d] = args;
        let f = ffx_bridge::x87::round_trip(f32::from_bits(f)).to_bits();
        let r = mem.rebase(REQUEST);
        mem.write(r, a);
        mem.write(r + 4, f);
        mem.write(r + 8, b);
        mem.write(r + 0x10, c);
        mem.write(r + 0xc, 0u32);
        mem.write(r + 0x14, d);
        REQUEST_PENDING.set(mem, 1);
        d
    }

    /// `rcEffectTaskRequestWait(task)` (`0x009304e0`, name ours): a
    /// sequence that returns once [`REQUEST_PENDING`] is 0.
    pub fn rc_effect_task_request_wait(mem: &mut impl Mem, t: u32) {
        if REQUEST_PENDING.get(mem) == 0 {
            rcef_task_ret_seq_cont(mem, t);
        }
    }

    /// `rcefAnimeFini(id)` (`0x0092f4a0`): `graphicVFXDestroy` of the
    /// handle `id << 32`. Returns its `eax`.
    pub fn rcef_anime_fini<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        id: u32,
        call: &mut C,
    ) -> u32 {
        let f = mem.rebase(VFX_DESTROY);
        call(mem, f, &[0, id])
    }

    /// `rcefModelFini(id)` (`0x0092f710`): the handle `(id << 32) +
    /// program` (the current program sign-extended) destroyed when the
    /// renderer has render data for it. Returns the last call's `eax`.
    pub fn rcef_model_fini<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        id: u32,
        call: &mut C,
    ) -> u32 {
        let p = CURRENT_PROGRAM.get(mem);
        let hi = id.wrapping_add(if p.cast_signed() < 0 { u32::MAX } else { 0 });
        let f = mem.rebase(VFX_GET_RENDER_DATA);
        let r = call(mem, f, &[p, hi]);
        if r == 0 {
            return 0;
        }
        let f = mem.rebase(VFX_DESTROY);
        call(mem, f, &[p, hi])
    }

    // ---- round 22 ----

    /// `rcefObjFini(obj)` (`0x0092fee0`, an object's finaliser
    /// [`obj::FINI`]): the particle part's finaliser (`0x0092efc0`, empty),
    /// [`rcef_anime_fini`] with the animation's address (`+0x330`) as the
    /// id, [`rcef_model_fini`] with the model's (`+0x3c0`), then the empty
    /// destructor. Returns `rcefModelFini`'s `eax`.
    pub fn rcef_obj_fini<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        o: u32,
        call: &mut C,
    ) -> u32 {
        rcef_anime_fini(mem, o.wrapping_add(obj::ANIME), call);
        rcef_model_fini(mem, o.wrapping_add(obj::MODEL), call)
    }

    // ---- round 4: running tasks, objects, particle parts ----

    /// `rcefTaskProc(task)` (`0x0092ed90`): the current level's sequence
    /// (`call(mem, function, task)`, again while it sets [`RERUN`]), then
    /// the after-function unless the task is dead. Returns the `eax` it
    /// ends with: 0 without an after-function, the function when dead,
    /// else its result.
    pub fn rcef_task_proc<M: Mem>(
        mem: &mut M,
        t: u32,
        mut call: impl FnMut(&mut M, u32, u32) -> u32,
    ) -> u32 {
        loop {
            let l = level(mem, t);
            let f = mem.read::<u32>(t + header::FLAGS);
            mem.write(t + header::FLAGS, f & !RERUN);
            let seq = mem.read::<u32>(seq_slot(t, l));
            if seq != 0 {
                call(mem, seq, t);
            }
            if mem.read::<u32>(t + header::FLAGS) & RERUN == 0 {
                break;
            }
        }
        let after = mem.read::<u32>(t + header::AFTER);
        if after == 0 || mem.read::<u32>(t + header::FLAGS) & super::DEAD != 0 {
            return after;
        }
        call(mem, after, t)
    }

    /// `rcefTaskDestruct(task)` (`0x0092ec90`): its destructor
    /// (`call(mem, function, task)`, a tail call: its result), or the task
    /// itself without one.
    pub fn rcef_task_destruct<M: Mem>(
        mem: &mut M,
        t: u32,
        call: impl FnOnce(&mut M, u32, u32) -> u32,
    ) -> u32 {
        match mem.read::<u32>(t + header::DESTRUCTOR) {
            0 => t,
            f => call(mem, f, t),
        }
    }

    /// `rcEffectTaskPrayWait(task)` (`0x009304b0`): returns to the caller
    /// sequence (`rcefTaskRetSeqCont`) once `op_pray_effect_check` of the
    /// pray effect (`0x025d5a24`; `check`) gives 0.
    pub fn rc_effect_task_pray_wait<M: Mem>(
        mem: &mut M,
        t: u32,
        check: impl FnOnce(&mut M, u32) -> i32,
    ) {
        let e = PRAY_EFFECT.get(mem);
        if check(mem, e) == 0 {
            rcef_task_ret_seq_cont(mem, t);
        }
    }

    /// `rcefObjInit(obj)` (`0x0092ff20`): cleared (`0x490` bytes), a task
    /// with `rcefObjFini` as destructor and `rcefObjProc` after, scales
    /// and colours 1.0, three identity matrices, the particle part, the
    /// animation and the model (`+0x3c0`); returns the model (the
    /// `memset` `rcefModelInit` tail-calls returns it).
    pub fn rcef_obj_init(mem: &mut impl Mem, o: u32) -> u32 {
        mem.write_bytes(o, &[0; obj::SIZE as usize]);
        rcef_task_init(mem, o);
        let fini = mem.rebase(obj::FINI);
        mem.write(o + header::DESTRUCTOR, fini);
        let proc = mem.rebase(obj::PROC);
        mem.write(o + header::AFTER, proc);
        for (at, v) in obj::INIT {
            mem.write(o + at, v);
        }
        for m in obj::MATRICES {
            ffx_ps2::vu0::bridge::matrix_identity(mem, o + m);
        }
        rcef_par_init(mem, o + obj::PAR);
        rcef_anime_init(mem, o + obj::ANIME);
        rcef_model_init(mem, o + obj::MODEL);
        o + obj::MODEL
    }

    /// `rcefParCreate(par, data, index, a, b, c)` (`0x0092eee0`): a running
    /// particle part on entry `index` of `data` (`0x50` bytes each from
    /// `+0x20`), scales 1.0 and the three arguments; parts of some magic
    /// effects are remembered (`MagicFile_GetMagicId`: index 2 of 0x20e,
    /// 0x180, 0x168, 0x217; index 1 of 0xbf, 0x216). Returns `c`.
    pub fn rcef_par_create(
        mem: &mut impl Mem,
        p: u32,
        data: u32,
        index: u32,
        a: u32,
        b: u32,
        c: u32,
    ) -> u32 {
        let id = ffx_system::instance::bridge::magic_id(mem);
        let table: &[(u32, u32)] = match index {
            2 => &super::PAR_KEEP_2,
            1 => &super::PAR_KEEP_1,
            _ => &[],
        };
        if let Some(&(_, at)) = table.iter().find(|&&(m, _)| m == id) {
            mem.write(mem.rebase(at), p);
        }
        mem.write(
            p + 0xc,
            index
                .wrapping_mul(0x50)
                .wrapping_add(0x20)
                .wrapping_add(data),
        );
        mem.write(p + 8, index);
        mem.write(p, 1u32);
        mem.write(p + 4, data);
        mem.write(p + 0x78, 1f32);
        mem.write(p + 0x74, 1f32);
        mem.write(p + 0x70, 1f32);
        mem.write(p + 0x90, a);
        mem.write(p + 0x94, b);
        mem.write(p + 0x98, c);
        c
    }

    /// `rcefMainSetPar(main, obj, data_index, index)` (`0x009256d0`):
    /// [`rcef_par_create`] on the object's particle part with the main
    /// data's entry (`+0x18` table), its matrix `+0x1f0`, scale `+0x160`
    /// and colour `+0x170`; returns the last.
    pub fn rcef_main_set_par(
        mem: &mut impl Mem,
        main: u32,
        o: u32,
        data_index: u32,
        index: u32,
    ) -> u32 {
        let table = mem.read::<u32>(main.wrapping_add(0x18));
        let data = mem.read::<u32>(table.wrapping_add(data_index.wrapping_mul(4)));
        rcef_par_create(
            mem,
            o.wrapping_add(obj::PAR),
            data,
            index,
            o.wrapping_add(0x1f0),
            o.wrapping_add(0x160),
            o.wrapping_add(0x170),
        )
    }

    /// `rcefTaskKill(t)` (`0x0092ed50`): `t` unlinked from its list, its
    /// destructor run ([`rcef_task_destruct`]) and its memory returned to
    /// the current effect heap (`pppMemFree(heap, heap + 0x10, t)`).
    pub fn rcef_task_kill<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        t: u32,
        call: &mut C,
    ) -> u32 {
        let (prev, next) = (mem.read::<u32>(t + 4), mem.read::<u32>(t + 8));
        mem.write(prev.wrapping_add(8), next);
        let (prev, next) = (mem.read::<u32>(t + 4), mem.read::<u32>(t + 8));
        mem.write(next.wrapping_add(4), prev);
        rcef_task_destruct(mem, t, |m, f, a| call(m, f, &[a]));
        let heap = mem.read::<u32>(super::main::ENV_SLOT.get(mem));
        let f = mem.rebase(super::PPP_MEM_FREE);
        call(mem, f, &[heap, heap.wrapping_add(0x10), t])
    }

    /// `rcefChrInit(c)` (`0x00930770`): an effect's character draw block:
    /// no character, a reference GIF tag (`+0x10` 0x1c, `+0x14` from
    /// `0x00c498dc`, `+0x18` 0x44), grey, its two matrices (`+0x80`,
    /// `+0xc0`, the identity) and the light's colour and normal matrices
    /// (`Sg_LightGet*MatrixPtr`). Returns the identity's `eax` (1.0).
    pub fn rcef_chr_init<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        c: u32,
        call: &mut C,
    ) -> u32 {
        mem.write(c, 0u32);
        mem.write(c + 4, 0u32);
        mem.write(c + 8, 0u32);
        mem.write_bytes(c + 0x10, &[0; 0x70]);
        mem.write(c + 0x10, 0x1cu32);
        let tag = mem.read::<u32>(mem.rebase(crate::model::REF_GIF_TAG));
        mem.write(c + 0x14, tag);
        mem.write(c + 0x18, 0x44u8);
        mem.write(c + 0x28, 0x8080_8080u32);
        mem.write(c + 0x30, c + 0x80);
        let f = mem.rebase(super::LIGHT_COLOR_MATRIX);
        let m = call(mem, f, &[]);
        mem.write(c + 0x34, m);
        let f = mem.rebase(super::LIGHT_NORMAL_MATRIX);
        let m = call(mem, f, &[]);
        mem.write(c + 0x38, m);
        mem.write(c + 0x3c, c + 0xc0);
        mem.write(c + 0x5c, 0x8080u16);
        ffx_ps2::vu0::bridge::matrix_identity(mem, c + 0x80);
        ffx_ps2::vu0::bridge::matrix_identity(mem, c + 0xc0)
    }

    /// `rcefWorkSwap(w)` (`0x00925530`): the quadwords at `+0x1c4` and
    /// `+0x1d4` swapped (`rcCopyQword` through `tmp`, the original's 16
    /// stack bytes), `+0x1e8` 0, `+0x1f0` 1.
    pub fn rcef_work_swap(mem: &mut impl Mem, w: u32, tmp: u32) {
        use crate::vu0::bridge::rc_copy_qword;
        rc_copy_qword(mem, tmp, w + 0x1d4);
        rc_copy_qword(mem, w + 0x1d4, w + 0x1c4);
        rc_copy_qword(mem, w + 0x1c4, tmp);
        mem.write(w + 0x1e8, 0u32);
        mem.write(w + 0x1f0, 1u32);
    }

    /// `rcefWorkSet(w, a, b, v)` (`0x00925630`): `+0x1c0` 1, `a` and `b`
    /// to `+0x1c4`/`+0x1d4` (`rcCopyQword`), `v` at `+0x1e4`, `+0x1e8` 0,
    /// `+0x1f0` 1.
    pub fn rcef_work_set(mem: &mut impl Mem, w: u32, a: u32, b: u32, v: u32) {
        use crate::vu0::bridge::rc_copy_qword;
        mem.write(w + 0x1c0, 1u32);
        rc_copy_qword(mem, w + 0x1c4, a);
        rc_copy_qword(mem, w + 0x1d4, b);
        mem.write(w + 0x1e4, v);
        mem.write(w + 0x1e8, 0u32);
        mem.write(w + 0x1f0, 1u32);
    }

    /// `rcefRunPart(env, part, arg)` (`0x0092eec0`): `env` made the current
    /// effect heap (`*g_pppEnvSlot`), then `pppRunPart(part, arg)`.
    pub fn rcef_run_part<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        env: u32,
        part: u32,
        arg: u32,
        call: &mut C,
    ) -> u32 {
        let slot = super::main::ENV_SLOT.get(mem);
        mem.write(slot, env);
        let f = mem.rebase(super::PPP_RUN_PART);
        call(mem, f, &[part, arg])
    }

    /// `rcefObjNew()` (`0x00930030`): 0x490 bytes from the current effect
    /// heap (`pppMemAlloc(heap, heap + 0x10, 0x490)`), [`rcef_obj_init`].
    pub fn rcef_obj_new<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) -> u32 {
        let heap = mem.read::<u32>(super::main::ENV_SLOT.get(mem));
        let f = mem.rebase(super::PPP_MEM_ALLOC);
        let o = call(mem, f, &[heap, heap.wrapping_add(0x10), 0x490]);
        rcef_obj_init(mem, o);
        o
    }

    /// `rcefTaskCtrlFini(ctrl)` (`0x00930260`): every task of the list
    /// (`+0xc0` the first, back at `ctrl` when empty) killed
    /// ([`rcef_task_kill`]).
    pub fn rcef_task_ctrl_fini<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        ctrl: u32,
        call: &mut C,
    ) {
        loop {
            let t = mem.read::<u32>(ctrl + 0xc0);
            if t == ctrl {
                break;
            }
            rcef_task_kill(mem, t, call);
        }
    }

    /// `rcefTaskCtrlProc(first)` (`0x00930300`): each task from `first`
    /// along `+0x8` run ([`rcef_task_proc`]), then the dead ones (flag 4)
    /// killed.
    pub fn rcef_task_ctrl_proc<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        first: u32,
        call: &mut C,
    ) {
        if first == 0 {
            return;
        }
        let mut t = first;
        while t != 0 {
            rcef_task_proc(mem, t, |m, f, a| call(m, f, &[a]));
            t = mem.read::<u32>(t + 8);
        }
        let mut t = first;
        while t != 0 {
            let next = mem.read::<u32>(t + 8);
            if mem.read::<u8>(t) & 4 != 0 {
                rcef_task_kill(mem, t, call);
            }
            t = next;
        }
    }

    /// `rcefChrFini(c)` (`0x00930710`): the character (`+0x4`) gets its
    /// CLUT and image bases back (`Ch_TextureSetClutBase`/`SetImageBase`
    /// with -1, -1) and effects off (`Ch_DrawEnEffect(chr, 0)`); the
    /// block at `+0x8` goes back to the effect heap (`pppMemFree`) and is
    /// cleared.
    pub fn rcef_chr_fini<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        c: u32,
        call: &mut C,
    ) {
        let chr = mem.read::<u32>(c + 4);
        let f = mem.rebase(super::CH_TEXTURE_SET_CLUT_BASE);
        call(mem, f, &[chr, u32::MAX, u32::MAX]);
        let chr = mem.read::<u32>(c + 4);
        let f = mem.rebase(super::CH_TEXTURE_SET_IMAGE_BASE);
        call(mem, f, &[chr, u32::MAX, u32::MAX]);
        let chr = mem.read::<u32>(c + 4);
        let f = mem.rebase(super::CH_DRAW_EN_EFFECT);
        call(mem, f, &[chr, 0]);
        let p = mem.read::<u32>(c + 8);
        if p != 0 {
            let heap = mem.read::<u32>(super::main::ENV_SLOT.get(mem));
            let f = mem.rebase(super::PPP_MEM_FREE);
            call(mem, f, &[heap, heap.wrapping_add(0x10), p]);
        }
        mem.write(c + 8, 0u32);
    }

    /// `rcefMainCreateObj(main)` (`0x00925140`): [`rcef_obj_new`], its init
    /// once more ([`rcef_obj_init`]), then added to the main's task list
    /// (`+0x1c`, [`rcef_task_ctrl_add`]). Returns the object.
    pub fn rcef_main_create_obj<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        m: u32,
        call: &mut C,
    ) -> u32 {
        let o = rcef_obj_new(mem, call);
        rcef_obj_init(mem, o);
        rcef_task_ctrl_add(mem, m + 0x1c, o);
        o
    }

    /// `rcefMainFini(main)` (`0x009251c0`): its environment current, the
    /// block at `+0x18` back to the heap (`pppMemFree`) and cleared, then
    /// [`rcef_task_ctrl_fini`] of its list (`+0x1c`).
    pub fn rcef_main_fini<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        m: u32,
        call: &mut C,
    ) {
        let slot = super::main::ENV_SLOT.get(mem);
        mem.write(slot, mem.read::<u32>(m + super::main::ENV));
        let p = mem.read::<u32>(m + 0x18);
        if p != 0 {
            let heap = mem.read::<u32>(super::main::ENV_SLOT.get(mem));
            let f = mem.rebase(super::PPP_MEM_FREE);
            call(mem, f, &[heap, heap.wrapping_add(0x10), p]);
            mem.write(m + 0x18, 0u32);
        }
        rcef_task_ctrl_fini(mem, m + 0x1c, call);
    }

    /// `rcefWorkSetFromVf00(w, b, v)` (`0x009255d0`): `+0x1c0` 1, `vf00` to
    /// `+0x1c4`, `b` to `+0x1d4` (`rcCopyQword`), `v` at `+0x1e4`, `+0x1e8`
    /// 0, `+0x1f0` 1, `+0x1f4` 0.
    pub fn rcef_work_set_from_vf00(mem: &mut impl Mem, w: u32, b: u32, v: u32) {
        mem.write(w + 0x1c0, 1u32);
        crate::vu0::bridge::rcef_get_vf00(mem, w + 0x1c4);
        crate::vu0::bridge::rc_copy_qword(mem, w + 0x1d4, b);
        mem.write(w + 0x1e4, v);
        mem.write(w + 0x1e8, 0u32);
        mem.write(w + 0x1f0, 1u32);
        mem.write(w + 0x1f4, 0u32);
    }

    // ---- round 13 ----

    /// `rcefMainKickPar(main, a, b, chr, bone, p0, p1, p2, flags, force)`
    /// (`0x009253a0`): a new object when the main's task has ended
    /// (`rcefMainCreateObj`), else its object (`+0x24`), unless that one's
    /// particles still run and `force` is 0 (returns 0 then); then
    /// `rcefMainSetPar(main, obj, a, b)`, `rcefObjAttachChr(obj, chr,
    /// bone)`, the three vectors to `+0xd0`, `+0x100`, `+0x130` and
    /// `flags` into bits 4..12 of `+0xbc`. Returns the new `+0xbc`.
    #[allow(clippy::many_single_char_names)] // the original's arguments
    pub fn rcef_main_kick_par<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        args: [u32; 10],
        call: &mut C,
    ) -> u32 {
        let [m, a, b, chr, bone, p0, p1, p2, flags, force] = args;
        let f = mem.rebase(super::TASK_CTRL_IS_END);
        let obj = if call(mem, f, &[m.wrapping_add(0x1c)]) != 0 {
            let f = mem.rebase(super::MAIN_CREATE_OBJ);
            call(mem, f, &[m])
        } else {
            let obj = mem.read::<u32>(m.wrapping_add(0x24));
            if force == 0 {
                let f = mem.rebase(super::OBJ_IS_PAR_END);
                if call(mem, f, &[obj]) == 0 {
                    return 0;
                }
            }
            obj
        };
        let f = mem.rebase(super::MAIN_SET_PAR);
        call(mem, f, &[m, obj, a, b]);
        let f = mem.rebase(super::OBJ_ATTACH_CHR);
        call(mem, f, &[obj, chr, bone]);
        let f = mem.rebase(super::COPY_QWORD);
        call(mem, f, &[obj.wrapping_add(0xd0), p0]);
        call(mem, f, &[obj.wrapping_add(0x100), p1]);
        call(mem, f, &[obj.wrapping_add(0x130), p2]);
        let at = obj.wrapping_add(0xbc);
        let v = (mem.read::<u32>(at) & 0xffff_e00f) | flags;
        mem.write(at, v);
        v
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
