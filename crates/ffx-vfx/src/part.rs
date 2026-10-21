//! Particle parts and their objects: the data set-up (`pppInitData`,
//! `pppInitPdt`), object creation (`pppCreatePObject`) and the loops that
//! call each program's set-ups.
//!
//! A part (`g_pppCurrentPart` while it runs) has its frame at `+8`, the
//! number of its programs at `+0x14` (their table records listed at
//! `+0x34`), the number of its object births at `+0x18` (16-byte records
//! at `+0x3c`: the object definition, `+8` its birth frame) and the head of
//! its object list at `+0x30`. An object definition starts with its
//! program list (`+0x20` the size of an object's work areas, `+0x24` the
//! drawing priority, `+0x26` the number of programs, 16-byte entries from
//! `+0x28`: the program table record, `+8` the program's work area
//! offset), then the last object made from it (`+4`) and a count (`+0xc`).
//!
//! Pointers are stored through `set_ptr32` and read through
//! `get_ptr32_func` ([`ffx_system::stub`], the identity on PC). Where the
//! originals call a program's functions through the program table
//! (`+0x10`..`+0x20` of a record), the bridges take a `call` callback
//! (function address, cdecl arguments) that the hook implements with the
//! function pointer and the oracle with the original machine code.

// b, c, d, e, n, o, p, r, w as in the descriptions
#![allow(clippy::many_single_char_names)]

/// The parts and objects where the game keeps them.
pub mod bridge {
    use ffx_bridge::{Global, Mem};
    use ffx_system::stub::{is_ptr32_null, ptr32};

    use crate::heap::bridge::{ENV_SLOT, MEM_ALLOC_FAILED, ppp_alloc, ppp_free};
    use crate::ppp::bridge::STOP;
    use crate::prog::bridge::CURRENT_PART;

    fn rd(mem: &impl Mem, at: u32) -> u32 {
        mem.read(at)
    }

    /// `pppInitPdt(pdt, programs)` (`0x007120f0`): relocates a part data
    /// block in place: its chain of object definitions (from `+0x10`,
    /// each linked by its first word; a definition whose successor's link is
    /// null ends the chain, and its own link is cleared), each
    /// definition's program entries (`+0x28`: the record index becomes a
    /// pointer into `programs`, 0x28 bytes each; `+8` and `+0xc` become
    /// pointers into the block), the record index list (`+8`, counted) and
    /// the offset list (`+0xc`, counted).
    pub fn ppp_init_pdt(mem: &mut impl Mem, pdt: u32, programs: u32) {
        let base = ptr32(pdt);
        let list8 = rd(mem, base.wrapping_add(8)).wrapping_add(base);
        let n8 = rd(mem, list8).cast_signed();
        let list9 = rd(mem, base.wrapping_add(0xc)).wrapping_add(base);
        let n9 = rd(mem, list9).cast_signed();
        let mut node = base.wrapping_add(0x10);
        if is_ptr32_null(rd(mem, node)) != 0 {
            return;
        }
        loop {
            let v = ptr32(ptr32(rd(mem, node)).wrapping_add(base));
            mem.write(node, v);
            let mut i = 0i32;
            while i < i32::from(mem.read::<i16>(node.wrapping_add(0x26))) {
                let e = node.wrapping_add(0x28 + 0x10 * i.cast_unsigned());
                let idx = ptr32(rd(mem, e));
                mem.write(e, ptr32(programs.wrapping_add(idx.wrapping_mul(0x28))));
                for at in [8, 0xc] {
                    let v = ptr32(ptr32(rd(mem, e + at)).wrapping_add(base));
                    mem.write(e + at, v);
                }
                i += 1;
            }
            let next = ptr32(rd(mem, node));
            if is_ptr32_null(rd(mem, next)) != 0 {
                break;
            }
            node = next;
        }
        mem.write(node, ptr32(0));
        for i in 0..n8.max(0).cast_unsigned() {
            let at = list8.wrapping_add(4 + 4 * i);
            let v = ptr32(programs.wrapping_add(rd(mem, at).wrapping_mul(0x28)));
            mem.write(at, v);
        }
        for i in 0..n9.max(0).cast_unsigned() {
            let at = list9.wrapping_add(4 + 4 * i);
            let v = ptr32(rd(mem, at).wrapping_add(base));
            mem.write(at, v);
        }
    }

    /// `pppInitData(data, programs, _)` (`0x00711fa0`): relocates an effect
    /// data block: its four tables (`+0x10..0x1c`); each part data block of
    /// the first (`u16` count at `+6`, through [`ppp_init_pdt`]); the three
    /// pointers of each 0x20-byte record of the second (`+8` count;
    /// `+0x14..0x1c`); the words of the third (`+0xa`); the `+4` of each
    /// 8-byte record of the fourth (`+0xc`).
    pub fn ppp_init_data(mem: &mut impl Mem, data: u32, programs: u32) {
        for at in [0x10, 0x14, 0x18, 0x1c] {
            let p = data.wrapping_add(at);
            let v = ptr32(ptr32(rd(mem, p)).wrapping_add(data));
            mem.write(p, v);
        }
        let n = mem.read::<u16>(data.wrapping_add(6));
        let mut list = ptr32(rd(mem, data.wrapping_add(0x10)));
        for _ in 0..n {
            let pdt = ptr32(rd(mem, list).wrapping_add(data));
            mem.write(list, pdt);
            ppp_init_pdt(mem, pdt, programs);
            list = list.wrapping_add(4);
        }
        let n = mem.read::<u16>(data.wrapping_add(8));
        let recs = ptr32(rd(mem, data.wrapping_add(0x14)));
        for r in 0..u32::from(n) {
            for at in [0x14, 0x18, 0x1c] {
                let p = recs.wrapping_add(0x20 * r + at);
                let v = ptr32(ptr32(rd(mem, p)).wrapping_add(data));
                mem.write(p, v);
            }
        }
        let n = mem.read::<u16>(data.wrapping_add(0xa));
        let words = ptr32(rd(mem, data.wrapping_add(0x18)));
        for i in 0..u32::from(n) {
            let p = words.wrapping_add(4 * i);
            let v = ptr32(ptr32(rd(mem, p)).wrapping_add(data));
            mem.write(p, v);
        }
        let n = mem.read::<u16>(data.wrapping_add(0xc));
        let recs = ptr32(rd(mem, data.wrapping_add(0x1c)));
        for r in 0..u32::from(n) {
            let p = recs.wrapping_add(8 * r + 4);
            let v = ptr32(ptr32(rd(mem, p)).wrapping_add(data));
            mem.write(p, v);
        }
    }

    /// `pppOtFunc_Reg(list, node, a, b)` (`0x00716be0`): appends `node`
    /// (`+8` a, `+0xc` b, `+0` the list) to the ordering list (`+4` its
    /// last node, `u16` count at `+0xc`).
    pub fn ppp_ot_func_reg(mem: &mut impl Mem, list: u32, node: u32, a: u32, b: u32) {
        mem.write(node.wrapping_add(8), ptr32(a));
        mem.write(node.wrapping_add(0xc), ptr32(b));
        mem.write(node, ptr32(list));
        let tail = ptr32(rd(mem, list.wrapping_add(4)));
        mem.write(tail, ptr32(node));
        let prev = rd(mem, list.wrapping_add(4));
        mem.write(node.wrapping_add(4), prev);
        let n = mem.read::<u16>(list.wrapping_add(0xc)).wrapping_add(1);
        mem.write(list.wrapping_add(0xc), n);
        mem.write(list.wrapping_add(4), ptr32(node));
    }

    /// `g_pppOtDrawing` (`0x00cec1a4`): 1 while `pppOtFunc_Control` runs
    /// the list's draws (the `KeLns` draws test it).
    pub const OT_DRAWING: Global<u32> = Global::new(0x00ce_c1a4);
    /// `MsCopyScreenMatrix(out)` (`0x007bba50`), called through `call`.
    const MS_COPY_SCREEN_MATRIX: Global<u32> = Global::new(0x007b_ba50);
    /// `MsCopyCameraMatrix(out)` (`0x007c0070`), called through `call`.
    const MS_COPY_CAMERA_MATRIX: Global<u32> = Global::new(0x007c_0070);
    /// `g_pppScreenMatrix` (`0x0230fe60`).
    const SCREEN_MATRIX: Global<u32> = Global::new(0x0230_fe60);
    /// `g_pppCameraMatrix` (`0x0230ff60`).
    const CAMERA_MATRIX: Global<u32> = Global::new(0x0230_ff60);

    /// `pppOtFunc_Control(list)` (`0x00716ac0`): with `g_pppOtDrawing` set,
    /// the screen and camera matrices copied (through `call`), then each
    /// node's draw (`+8`) called as `f(data, i, frame)` (`+0xc`; `i` its
    /// place in the list, `frame` the list's `+0xe` on entry), and the
    /// list's `+0xe` advanced.
    pub fn ppp_ot_func_control<M: Mem>(
        mem: &mut M,
        list: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        OT_DRAWING.set(mem, 1);
        let (f, out) = (
            MS_COPY_SCREEN_MATRIX.ptr(mem).addr,
            SCREEN_MATRIX.ptr(mem).addr,
        );
        call(mem, f, &[out]);
        let (f, out) = (
            MS_COPY_CAMERA_MATRIX.ptr(mem).addr,
            CAMERA_MATRIX.ptr(mem).addr,
        );
        call(mem, f, &[out]);
        let mut node = ptr32(rd(mem, list));
        let frame = u32::from(mem.read::<u16>(list.wrapping_add(0xe)));
        let mut i = 0u32;
        while i < u32::from(mem.read::<u16>(list.wrapping_add(0xc))) {
            let f = ptr32(rd(mem, node.wrapping_add(8)));
            let data = ptr32(rd(mem, node.wrapping_add(0xc)));
            call(mem, f, &[data, i, frame]);
            i += 1;
            node = ptr32(rd(mem, node));
        }
        let n = mem.read::<u16>(list.wrapping_add(0xe)).wrapping_add(1);
        mem.write(list.wrapping_add(0xe), n);
        OT_DRAWING.set(mem, 0);
    }

    /// `pppSetPartMatrix(camera, screen)` (`0x00762550`):
    /// `g_pppCameraMatrix` and `g_pppScreenMatrix` copied from the two
    /// matrices (`FFXVu0CopyMatrix`); returns the second copy's eax.
    pub fn ppp_set_part_matrix(mem: &mut impl Mem, camera: u32, screen: u32) -> u32 {
        use ffx_ps2::vu0::bridge as vu0;
        let cam = CAMERA_MATRIX.ptr(mem).addr;
        vu0::matrix_copy(mem, cam, camera);
        let scr = SCREEN_MATRIX.ptr(mem).addr;
        vu0::matrix_copy(mem, scr, screen)
    }

    /// `pppOtFunc_InitEnv(list)` (`0x00716b70`): an empty OT list: both
    /// links (`+4`, then `+0`) `set_ptr32(list)` (through `call`), the
    /// count `+0xc` 0. Returns 0.
    pub fn ppp_ot_func_init_env<M: Mem>(
        mem: &mut M,
        list: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let set = Global::<u32>::new(0x00a4_4700).ptr(mem).addr;
        let p = call(mem, set, &[list]);
        mem.write(list.wrapping_add(4), p);
        let p = call(mem, set, &[list]);
        mem.write(list, p);
        mem.write(list.wrapping_add(0xc), 0u32);
        0
    }

    /// `pppGetPartMatrix(camera, screen)` (`0x00761c40`): the camera and
    /// screen matrices into the two buffers: with the word at
    /// `0x00d4b7f8` = 1 (the battle's) `MsCopyScreenMatrix(screen)` and
    /// `MsCopyCameraMatrix(camera)` (through `call`), else copies of
    /// `g_pppCameraMatrix` and `g_pppScreenMatrix` (`FFXVu0CopyMatrix`).
    /// Returns the last callee's eax.
    pub fn ppp_get_part_matrix<M: Mem>(
        mem: &mut M,
        camera: u32,
        screen: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_ps2::vu0::bridge as vu0;
        let g = |mem: &M, a: u32| Global::<u32>::new(a).ptr(mem).addr;
        if mem.read::<u32>(g(mem, 0x00d4_b7f8)) == 1 {
            call(mem, g(mem, 0x007b_ba50), &[screen]);
            return call(mem, g(mem, 0x007c_0070), &[camera]);
        }
        let cam = CAMERA_MATRIX.ptr(mem).addr;
        vu0::matrix_copy(mem, camera, cam);
        let scr = SCREEN_MATRIX.ptr(mem).addr;
        vu0::matrix_copy(mem, screen, scr)
    }

    /// `pppOtFunc_Lev(list, node)` (`0x00716ba0`): `node` unlinked (its
    /// neighbours, through `get_ptr32`, linked to each other) and the
    /// list's count (`+0xc`) one less.
    pub fn ppp_ot_func_lev(mem: &mut impl Mem, list: u32, node: u32) {
        let next = rd(mem, node);
        let prev = rd(mem, node.wrapping_add(4));
        mem.write(ptr32(prev), next);
        let next = rd(mem, node);
        mem.write(ptr32(next).wrapping_add(4), rd(mem, node.wrapping_add(4)));
        let n = mem.read::<u16>(list.wrapping_add(0xc)).wrapping_sub(1);
        mem.write(list.wrapping_add(0xc), n);
    }

    /// `pppPObjectCon` (`0x00712d90`, `at` 0x1c) and `pppPObjectCon2`
    /// (`0x00712d20`, `at` 0x20) `(obj)`: each of the object's programs
    /// gets its work area offset (into the offsets at `obj + [+0x20]`) and
    /// its set-up called as `f(obj, entry)`.
    pub fn ppp_pobject_con<M: Mem>(
        mem: &mut M,
        obj: u32,
        at: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let def = ptr32(rd(mem, obj.wrapping_add(8)));
        let prog = ptr32(rd(mem, def));
        let mut slot = rd(mem, prog.wrapping_add(0x20)).wrapping_add(obj);
        let mut i = 0i32;
        while i < i32::from(mem.read::<i16>(prog.wrapping_add(0x26))) {
            let e = prog.wrapping_add(0x28 + 0x10 * i.cast_unsigned());
            let rec = ptr32(rd(mem, e));
            let off = rd(mem, e + 8);
            mem.write(slot, off);
            slot = slot.wrapping_add(4);
            if rec != 0 {
                let f = rd(mem, rec.wrapping_add(at));
                if f != 0 {
                    call(mem, f, &[obj, e]);
                }
            }
            i += 1;
        }
    }

    /// `0x00712e40` (`at` 0x10), `0x00712e80` (`at` 0x14) and `0x00712e00`
    /// (`at` 0x18) `(part)`: each of the part's programs (`+0x34`, `+0x14`
    /// of them) that has a function there called as `f(part)`.
    pub fn ppp_part_call<M: Mem>(
        mem: &mut M,
        part: u32,
        at: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let mut i = 0i32;
        while i < rd(mem, part.wrapping_add(0x14)).cast_signed() {
            let list = rd(mem, part.wrapping_add(0x34));
            let rec = ptr32(rd(mem, list.wrapping_add(4 * i.cast_unsigned())));
            let f = rd(mem, rec.wrapping_add(at));
            if f != 0 {
                call(mem, f, &[part]);
            }
            i += 1;
        }
    }

    /// `pppCreatePObject(part, def)` (`0x00716770`): a new object of `def`
    /// from the current environment's heaps (its program list's work size
    /// plus a word per program; 0 and `g_pppMemAllocFailed` when neither
    /// heap has room), linked into the part's list after the last object
    /// of the same definition, else before the first object of equal or
    /// higher priority (the definition then remembers it), else at the end;
    /// counted, then set up ([`ppp_pobject_con`]). Returns the object.
    pub fn ppp_create_pobject<M: Mem>(
        mem: &mut M,
        part: u32,
        def: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let prog = ptr32(rd(mem, def));
        let n = i32::from(mem.read::<i16>(prog.wrapping_add(0x26)));
        let size = rd(mem, prog.wrapping_add(0x20)).wrapping_add((n * 4).cast_unsigned());
        let env = rd(mem, ENV_SLOT.get(mem));
        let spare = env.wrapping_add(0x10);
        let mut obj = ppp_alloc(mem, env, size);
        MEM_ALLOC_FAILED.set(mem, 0);
        if obj == 0 {
            if rd(mem, spare.wrapping_add(4)) != 0 {
                obj = ppp_alloc(mem, spare, size);
            }
            if obj == 0 {
                MEM_ALLOC_FAILED.set(mem, 1);
                return 0;
            }
        }
        mem.write(obj.wrapping_add(0xc), 0u32);
        mem.write(obj.wrapping_add(0x90), ptr32(0));
        mem.write(obj.wrapping_add(0x94), ptr32(0));
        mem.write(obj.wrapping_add(8), ptr32(def));
        let mut link = part.wrapping_add(0x30);
        let mut cur = ptr32(rd(mem, link));
        let next = 'place: {
            if cur != 0 {
                if is_ptr32_null(rd(mem, def.wrapping_add(4))) == 0 {
                    let last = ptr32(rd(mem, def.wrapping_add(4)));
                    let after = rd(mem, last);
                    mem.write(obj, after);
                    let last = ptr32(rd(mem, def.wrapping_add(4)));
                    mem.write(last, ptr32(obj));
                    break 'place None;
                }
                let prio = mem.read::<i16>(prog.wrapping_add(0x24));
                loop {
                    let d = ptr32(rd(mem, cur.wrapping_add(8)));
                    let p = ptr32(rd(mem, d));
                    if mem.read::<i16>(p.wrapping_add(0x24)) >= prio {
                        break 'place Some(cur);
                    }
                    link = cur;
                    cur = ptr32(rd(mem, cur));
                    if cur == 0 {
                        break 'place Some(0);
                    }
                }
            }
            Some(0)
        };
        if let Some(next) = next {
            mem.write(def.wrapping_add(4), ptr32(obj));
            mem.write(link, ptr32(obj));
            mem.write(obj, ptr32(next));
        }
        let c = mem.read::<u16>(def.wrapping_add(0xc)).wrapping_add(1);
        mem.write(def.wrapping_add(0xc), c);
        ppp_pobject_con(mem, obj, 0x1c, call);
        obj
    }

    /// `pppBornPObjects(part)` (`0x00712b80`): each birth record (`+0x3c`,
    /// `+0x18` of them) whose frame (`+8`) has come (at or before the
    /// part's `+8`) is marked done (`0x7fffffff`) and makes an object;
    /// stops when one cannot be made.
    pub fn ppp_born_pobjects<M: Mem>(
        mem: &mut M,
        part: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let mut i = 0i32;
        while i < rd(mem, part.wrapping_add(0x18)).cast_signed() {
            let births = rd(mem, part.wrapping_add(0x3c));
            let rec = births.wrapping_add(0x10 * i.cast_unsigned());
            if births != 0
                && rec != 0
                && rd(mem, rec + 8).cast_signed() <= rd(mem, part.wrapping_add(8)).cast_signed()
            {
                mem.write(rec + 8, 0x7fff_ffffu32);
                if ppp_create_pobject(mem, part, rec, call) == 0 {
                    return;
                }
            }
            i += 1;
        }
    }

    /// `graphicVFXDestroy(lo, hi)` (`0x00645280`): the renderer's side of
    /// an effect instance, called through `call`.
    pub(crate) const GRAPHIC_VFX_DESTROY: Global<u32> = Global::new(0x0064_5280);
    /// `TOGetFFXLang()` (`0x006410e0`), called through `call`.
    const TO_GET_FFX_LANG: Global<u32> = Global::new(0x0064_10e0);
    /// `g_pppUserFuncs`: the user function table `pppCallUsrFunc` indexes
    /// (`pppAllFreePObject` clears it).
    pub const USER_FUNCS: Global<u32> = Global::new(0x0230_fd30);
    /// `g_pppObjectCount`: `pppRunPartStd` adds each definition's object
    /// count.
    pub const OBJECT_COUNT: Global<u32> = Global::new(0x0230_fd24);
    /// `g_pppCurrentProgram`: the program index `pppRunPartStd` is
    /// running (0 after).
    pub const CURRENT_PROGRAM: Global<u32> = Global::new(0x0230_fd20);
    /// `g_pppSpecialPart`: set by `pppRunPartStd` for the 0x33-birth part
    /// or spell 0x296.
    pub const SPECIAL_PART: Global<u32> = Global::new(0x00cc_b9cc);

    /// Frees `data` back to the current environment: its heap when `data`
    /// is at or below the heap's last block (`+8`), else its spare heap.
    fn env_free(mem: &mut impl Mem, data: u32) {
        let env = rd(mem, ENV_SLOT.get(mem));
        if data > rd(mem, env.wrapping_add(8)) {
            ppp_free(mem, env.wrapping_add(0x10), data);
        } else {
            ppp_free(mem, env, data);
        }
    }

    /// `pppDeletePObject(obj, _)` (`0x007168f0`): each program's destructor
    /// (`+0x24`, as `f(obj, entry)`) and `graphicVFXDestroy(index, obj +
    /// (index >> 31))` (the 64-bit instance `obj:index`) for each program
    /// with a record; the definition's count down (its last object cleared
    /// at 0, else moved on to the next when it was this one); the object
    /// back to the environment's heaps.
    pub fn ppp_delete_pobject<M: Mem>(
        mem: &mut M,
        obj: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let def = ptr32(rd(mem, obj.wrapping_add(8)));
        let prog = ptr32(rd(mem, def));
        let mut i = 0i32;
        while i < i32::from(mem.read::<i16>(prog.wrapping_add(0x26))) {
            let e = prog.wrapping_add(0x28 + 0x10 * i.cast_unsigned());
            let rec = ptr32(rd(mem, e));
            if rec != 0 {
                let f = rd(mem, rec.wrapping_add(0x24));
                if f != 0 {
                    call(mem, f, &[obj, e]);
                }
                let destroy = GRAPHIC_VFX_DESTROY.ptr(mem).addr;
                let hi = obj.wrapping_add((i >> 31).cast_unsigned());
                call(mem, destroy, &[i.cast_unsigned(), hi]);
            }
            i += 1;
        }
        let n = mem.read::<u16>(def.wrapping_add(0xc)).wrapping_sub(1);
        mem.write(def.wrapping_add(0xc), n);
        if n == 0 {
            mem.write(def.wrapping_add(4), ptr32(0));
        } else if ptr32(rd(mem, def.wrapping_add(4))) == obj {
            let next = rd(mem, obj);
            mem.write(def.wrapping_add(4), next);
        }
        env_free(mem, obj);
    }

    /// `pppAllFreePObject(part)` (`0x00711f20`): every object deleted, the
    /// list emptied, the birth records (`+0x3c`) freed, the user function
    /// table cleared.
    pub fn ppp_all_free_pobject<M: Mem>(
        mem: &mut M,
        part: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let mut o = ptr32(rd(mem, part.wrapping_add(0x30)));
        while o != 0 {
            let next = ptr32(rd(mem, o));
            ppp_delete_pobject(mem, o, call);
            o = next;
        }
        let births = rd(mem, part.wrapping_add(0x3c));
        mem.write(part.wrapping_add(0x30), ptr32(0));
        if births != 0 {
            env_free(mem, births);
            mem.write(part.wrapping_add(0x3c), 0u32);
        }
        USER_FUNCS.set(mem, 0);
    }

    /// The step or draw of one program over a definition's objects: each
    /// object's key pointer (its slot at `obj + [prog+0x20] + 4 c`) moves on
    /// by the entry's key size (`u16` at `+4`) when the next key's frame is
    /// the object's (`+0xc`), then `f(obj, key, entry)`. With a step
    /// (`fs[0]`) the draw (`fs[1]`, when set) follows it while the program
    /// list's `+0xc` bit 0 is clear (read for each object). With `checked`
    /// the loop stops after any call that leaves `g_pppMemAllocFailed` set,
    /// and returns true.
    #[allow(clippy::too_many_arguments)]
    fn run_program<M: Mem>(
        mem: &mut M,
        prog: u32,
        c: u32,
        e: u32,
        first: u32,
        count: u32,
        fs: [u32; 2],
        checked: bool,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> bool {
        let failed = |mem: &M| checked && MEM_ALLOC_FAILED.get(mem) != 0;
        let mut o = first;
        let mut n = count;
        loop {
            let slot = rd(mem, prog.wrapping_add(0x20))
                .wrapping_add(4 * c)
                .wrapping_add(o);
            let mut key = ptr32(rd(mem, slot));
            let next_key = u32::from(mem.read::<u16>(e.wrapping_add(4))).wrapping_add(key);
            if rd(mem, next_key) == rd(mem, o.wrapping_add(0xc)) {
                mem.write(slot, ptr32(next_key));
                key = next_key;
            }
            if fs[0] != 0 {
                let next = ptr32(rd(mem, o));
                call(mem, fs[0], &[o, key, e]);
                if failed(mem) {
                    return true;
                }
                if fs[1] != 0 && mem.read::<u8>(prog.wrapping_add(0xc)) & 1 == 0 {
                    call(mem, fs[1], &[o, key, e]);
                    if failed(mem) {
                        return true;
                    }
                }
                o = next;
            } else {
                call(mem, fs[1], &[o, key, e]);
                if failed(mem) {
                    return true;
                }
                // the draw-only loop moves on after its call (the same
                // unless the draw relinks the object)
                o = ptr32(rd(mem, o));
            }
            n = n.wrapping_sub(1);
            if n == 0 {
                return false;
            }
        }
    }

    /// `pppDrawShape0(obj, key, entry)` (`0x00740a70`), the shape draw.
    pub const DRAW_SHAPE0: u32 = 0x0074_0a70;
    /// `pppDrawShapeX`'s draw (`0x00740f60`).
    pub const DRAW_SHAPE_X0: u32 = 0x0074_0f60;

    /// `pppDrawShape0(obj, key, entry)` (`0x00740a70`), the shape draw of
    /// `pppDrawShape`: unless the key's shape (`+4`) is `0xffff`, a render
    /// request on the stack (`frame - 0x60`, `frame` the original's
    /// aligned `ebp`): flags `+2` = 0x80, the colour at `+4` (through
    /// `g_pppColorConShift`; the object's colour from its second work area
    /// `[[entry+0xc]+4]`, [`crate::mdl::bridge::ee_colour_at`]), `+8` = 0,
    /// the object's matrix `obj+0x50` at `+0x10` and `+0x1c`,
    /// `g_PppPacketPos` at `+0x14`, byte `+0x18` = 0 (`+0` and `+0xc`
    /// left as they were). The shape set (`get_ptr32(env[+0x24][shape])`)
    /// gives the frame record `8 (frame + 2)` (frame = `u16` 1 of the
    /// first work area's `+0xa0` pair) and its data (`+ i16 rec[0]`).
    /// Unless the renderer skips the frame, the instance's render data is
    /// updated (`updateVFXRenderDataShp(inst, slot, req, frame)`, then
    /// `graphicVFXUpdate`) or created (`InitVFXRenderDataShp(inst, slot,
    /// req, frame, calculateVFXSP_AlphaFunction(byte data+[data+4]+4))`,
    /// that byte forced to 0x48 in magic file 499); the path-3 variants
    /// follow request flag 4, which 0x80 never has. Then (unless stopped)
    /// the pair's time (`u16` 0) advances by `i16 key+8`; at the record's
    /// `i16 +2` it moves to the next frame, and past the set's last
    /// (`i16 +6`) loops (record byte `+4` bit 7: both 0) or holds the last.
    pub fn ppp_draw_shape0<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        entry: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let g = |mem: &M, a: u32| Global::<u32>::new(a).ptr(mem).addr;
        let offs: u32 = rd(mem, entry.wrapping_add(0xc));
        let pair = rd(mem, offs).wrapping_add(0xa0).wrapping_add(obj);
        let area = rd(mem, offs.wrapping_add(4)).wrapping_add(obj);
        let shape = rd(mem, key.wrapping_add(4));
        if shape == 0xffff {
            return;
        }
        let env = rd(mem, ENV_SLOT.get(mem));
        let slot = rd(mem, env.wrapping_add(0x24)).wrapping_add(shape.wrapping_mul(4));
        let req = frame.wrapping_sub(0x60);
        let r = |k: u32| req.wrapping_add(k);
        mem.write(r(8), 0u32);
        mem.write(r(0x1c), obj.wrapping_add(0x50));
        mem.write(r(0x10), obj.wrapping_add(0x50));
        mem.write(r(2), 0x80u16);
        mem.write(r(0x18), 0u8);
        let colour = crate::mdl::bridge::ee_colour_at(mem, area, false, call);
        let shift = g(mem, 0x00c8_b500);
        let sh: u32 = mem.read(shift);
        mem.write(r(4).wrapping_add(sh), colour);
        mem.write(shift, 0u32);
        let set = ptr32(rd(mem, slot));
        let rec = set.wrapping_add((u32::from(mem.read::<u16>(pair.wrapping_add(2))) + 2) * 8);
        let data = set.wrapping_add(i32::from(mem.read::<i16>(rec)).cast_unsigned());
        let packet = rd(mem, g(mem, 0x0230_fd2c));
        mem.write(r(0x14), packet);
        let inst = |mem: &M| crate::draw::bridge::instance(mem, obj);
        let path3 = |mem: &M| mem.read::<u16>(r(2)) & 4 != 0;
        if rd(mem, g(mem, 0x012f_b7d0)) == 0 {
            let [lo, hi] = inst(mem);
            let n = u32::from(mem.read::<u16>(pair.wrapping_add(2)));
            if call(mem, g(mem, 0x0064_53d0), &[lo, hi]) == 0 {
                let d = data.wrapping_add(rd(mem, data.wrapping_add(4)));
                if call(mem, g(mem, 0x009d_a3b0), &[]) == 499 {
                    mem.write(d.wrapping_add(4), 0x48u8);
                }
                let blend = u32::from(mem.read::<u8>(d.wrapping_add(4)));
                let alpha = call(mem, g(mem, 0x0071_2c30), &[blend]);
                let [lo, hi] = inst(mem);
                let create = if path3(mem) { 0x0071_1030 } else { 0x0071_1460 };
                call(mem, g(mem, create), &[lo, hi, slot, req, n, alpha]);
            } else {
                let [lo, hi] = inst(mem);
                call(mem, g(mem, 0x0071_b8a0), &[lo, hi, slot, req, n]);
                let [lo, hi] = inst(mem);
                let update = if path3(mem) { 0x0064_5600 } else { 0x0064_58a0 };
                call(mem, g(mem, update), &[lo, hi]);
            }
        }
        advance_shape(mem, pair, key, set, rec);
    }

    /// The end of [`ppp_draw_shape0`] and [`ppp_draw_shape_x0`]: unless
    /// stopped, the time/frame pair at `pair` moves on by `i16 key+8`
    /// through the shape set's frame records.
    fn advance_shape(mem: &mut impl Mem, pair: u32, key: u32, set: u32, rec: u32) {
        if STOP.get(mem) != 0 {
            return;
        }
        let t = mem
            .read::<u16>(pair)
            .wrapping_add(mem.read::<u16>(key.wrapping_add(8)));
        mem.write(pair, t);
        let span = mem.read::<u16>(rec.wrapping_add(2));
        if i32::from(t) < i32::from(span.cast_signed()) {
            return;
        }
        let f = mem.read::<u16>(pair.wrapping_add(2)).wrapping_add(1);
        mem.write(pair.wrapping_add(2), f);
        mem.write(pair, t.wrapping_sub(span));
        let f = mem.read::<u16>(pair.wrapping_add(2));
        if i32::from(f) < i32::from(mem.read::<i16>(set.wrapping_add(6))) {
            return;
        }
        if mem.read::<u8>(rec.wrapping_add(4)) & 0x80 != 0 {
            mem.write(pair, 0u32);
        } else {
            mem.write(pair, 0u16);
            mem.write(pair.wrapping_add(2), f.wrapping_sub(1));
        }
    }

    /// `pppDrawShapeX0(obj, key, entry)` (`0x00740f60`), `pppDrawShapeX`'s
    /// draw: [`ppp_draw_shape0`] with the key's options and a few spells'
    /// exceptions. `MagicFile_GetMagicId` first (even for shape
    /// `0xffff`); the request's flags 0x80, 0x880 with key byte `+0xc`,
    /// `| 0x400` with `+0xd`; `+0x1c` = the third work area's `+0xa0`
    /// with `+0xe`, else 0; `+0xc` = 0; the colour from the second work
    /// area. Unless the renderer skips the frame: `+0x1c` = 0 for spell
    /// 0x241 program 9, 0x251/0x267 program 13, and 0x1e6/0x1dd; then the
    /// render data is updated (`graphicVFXUpdate`, no path 3) or created
    /// with the blend byte 0x44 turned 0x48 in spell 0x1a8, not at all on
    /// map 0x6c in field effect 0x4d program 8, and alpha `| 0x800` on map
    /// 0x137 in spell 0x13c or for spell 0x241 program 9.
    #[allow(clippy::too_many_lines)]
    pub fn ppp_draw_shape_x0<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        entry: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let g = |mem: &M, a: u32| Global::<u32>::new(a).ptr(mem).addr;
        let offs: u32 = rd(mem, entry.wrapping_add(0xc));
        let pair = rd(mem, offs).wrapping_add(0xa0).wrapping_add(obj);
        let area = rd(mem, offs.wrapping_add(4)).wrapping_add(obj);
        let magic = call(mem, g(mem, 0x009d_a3b0), &[]);
        let shape = rd(mem, key.wrapping_add(4));
        if shape == 0xffff {
            return;
        }
        let env = rd(mem, ENV_SLOT.get(mem));
        let slot = rd(mem, env.wrapping_add(0x24)).wrapping_add(shape.wrapping_mul(4));
        let req = frame.wrapping_sub(0x60);
        let r = |k: u32| req.wrapping_add(k);
        let opt = |mem: &M, k: u32| mem.read::<u8>(key.wrapping_add(k)) != 0;
        let mut flags: u16 = if opt(mem, 0xc) { 0x880 } else { 0x80 };
        if opt(mem, 0xd) {
            flags |= 0x400;
        }
        mem.write(r(2), flags);
        let third = if opt(mem, 0xe) {
            let offs: u32 = rd(mem, entry.wrapping_add(0xc));
            rd(mem, offs.wrapping_add(0xc))
                .wrapping_add(0xa0)
                .wrapping_add(obj)
        } else {
            0
        };
        mem.write(r(0x1c), third);
        mem.write(r(0x10), obj.wrapping_add(0x50));
        mem.write(r(8), 0u32);
        mem.write(r(0xc), 0u32);
        mem.write(r(0x18), 0u8);
        let colour = crate::mdl::bridge::ee_colour_at(mem, area, false, call);
        let shift = g(mem, 0x00c8_b500);
        let sh: u32 = mem.read(shift);
        mem.write(r(4).wrapping_add(sh), colour);
        mem.write(shift, 0u32);
        let set = ptr32(rd(mem, slot));
        let rec = set.wrapping_add((u32::from(mem.read::<u16>(pair.wrapping_add(2))) + 2) * 8);
        let data = set.wrapping_add(i32::from(mem.read::<i16>(rec)).cast_unsigned());
        let packet = rd(mem, g(mem, 0x0230_fd2c));
        mem.write(r(0x14), packet);
        if rd(mem, g(mem, 0x012f_b7d0)) == 0 {
            let prog = rd(mem, g(mem, 0x0230_fd20));
            let unlinked = if magic == 0x241 {
                prog == 9
            } else {
                ((magic == 0x251 || magic == 0x267) && prog == 0xd)
                    || magic == 0x1e6
                    || magic == 0x1dd
            };
            if unlinked {
                mem.write(r(0x1c), 0u32);
            }
            let inst = |mem: &M| crate::draw::bridge::instance(mem, obj);
            let [lo, hi] = inst(mem);
            if call(mem, g(mem, 0x0064_53d0), &[lo, hi]) == 0 {
                let d = data.wrapping_add(rd(mem, data.wrapping_add(4)));
                if magic == 0x1a8 && mem.read::<u8>(d.wrapping_add(4)) == 0x44 {
                    mem.write(d.wrapping_add(4), 0x48u8);
                }
                let blend = u32::from(mem.read::<u8>(d.wrapping_add(4)));
                let mut alpha = call(mem, g(mem, 0x0071_2c30), &[blend]);
                let map = rd(mem, g(mem, 0x012f_bc40));
                let prog = rd(mem, g(mem, 0x0230_fd20));
                let skip = map == 0x6c && rd(mem, g(mem, 0x00c3_a500)) == 0x4d && prog == 8;
                if !skip {
                    if (map == 0x137 && magic == 0x13c) || (magic == 0x241 && prog == 9) {
                        alpha |= 0x800;
                    }
                    let [lo, hi] = inst(mem);
                    let n = u32::from(mem.read::<u16>(pair.wrapping_add(2)));
                    call(mem, g(mem, 0x0071_1460), &[lo, hi, slot, req, n, alpha]);
                }
            } else {
                let n = u32::from(mem.read::<u16>(pair.wrapping_add(2)));
                let [lo, hi] = inst(mem);
                call(mem, g(mem, 0x0071_b8a0), &[lo, hi, slot, req, n]);
                let [lo, hi] = inst(mem);
                call(mem, g(mem, 0x0064_58a0), &[lo, hi]);
            }
        }
        advance_shape(mem, pair, key, set, rec);
    }

    /// `pppDrawShape` (`0x007409e0`, with [`DRAW_SHAPE0`]) and
    /// `pppDrawShapeX` (`0x00740ed0`, with [`DRAW_SHAPE_X0`]) `(def, prog,
    /// entry, c)`, the programs' runners: unless the program list's `+0xc`
    /// bit 0 is set, each of the definition's objects (`+4`, `+0xc` of
    /// them; at least one) gets the key advance and the draw called
    /// (through `call`) as `draw(obj, key, entry)`. Returns what the
    /// original leaves in `eax` (overlays return it): `4 c`, or the first
    /// object when the bit is set.
    pub fn ppp_draw_shape<M: Mem>(
        mem: &mut M,
        draw: u32,
        def: u32,
        prog: u32,
        e: u32,
        c: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let first = ptr32(rd(mem, def.wrapping_add(4)));
        if mem.read::<u8>(prog.wrapping_add(0xc)) & 1 != 0 {
            return first;
        }
        let count = u32::from(mem.read::<u16>(def.wrapping_add(0xc)));
        let draw = Global::<u32>::new(draw).ptr(mem).addr;
        run_program(mem, prog, c, e, first, count, [0, draw], false, call);
        c.wrapping_mul(4)
    }

    /// `pppRunPartStd(part)` (`0x00716c40`): one frame of a part's
    /// objects. For each birth record with objects, each program runs its
    /// own runner (`+4` of the record, as `f(def, prog, entry, c)`) or its
    /// step (`+8`) and draw (`+0xc`, not when the definition's `+0xc` bit 0
    /// is set) over the definition's objects; then (unless stopped) their
    /// frames advance by 0x1000. Then each object past its last frame
    /// (`+0x1c` of its definition) restarts at `+0x18` with its second
    /// set-ups, or (when the part is ending, `+0x1d`) is deleted past its
    /// `+0x14`; an ending part whose objects all end at 0x70000000 deletes
    /// them all.
    pub fn ppp_run_part_std<M: Mem>(
        mem: &mut M,
        part: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        if (part != 0 && rd(mem, part.wrapping_add(0x18)) == 0x33)
            || ffx_system::instance::bridge::magic_id(mem) == 0x296
        {
            SPECIAL_PART.set(mem, 1);
        }
        run_births(mem, part, false, call);
        end_objects(mem, part, call);
    }

    /// `pppRunPartFpObjects_00717010(part)` (`0x00717010`), the objects'
    /// frame of `pppRunPartFp`: [`ppp_run_part_std`] without the special
    /// part flag, skipping a null birth list, and returning at once (the
    /// current program left set) when a program's step or draw leaves
    /// `g_pppMemAllocFailed` set. (Its "ERROR!!!! prog=NULL" `dbgPrintf`
    /// for a missing program record is a bare `ret` on PC and left out,
    /// as in `pppRunPartStd`.)
    pub fn ppp_run_part_fp_objects<M: Mem>(
        mem: &mut M,
        part: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        if !run_births(mem, part, true, call) {
            end_objects(mem, part, call);
        }
    }

    /// The births of [`ppp_run_part_std`] (`checked`: of
    /// [`ppp_run_part_fp_objects`], which also skips a null birth list and
    /// returns true when a call failed an allocation).
    fn run_births<M: Mem>(
        mem: &mut M,
        part: u32,
        checked: bool,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> bool {
        let mut b = 0i32;
        while b < rd(mem, part.wrapping_add(0x18)).cast_signed() {
            let list = rd(mem, part.wrapping_add(0x3c));
            let def = list.wrapping_add(0x10 * b.cast_unsigned());
            b += 1;
            if def == 0 || (checked && list == 0) {
                continue;
            }
            let prog = ptr32(rd(mem, def));
            if prog == 0 {
                continue;
            }
            let objects = u32::from(mem.read::<u16>(def.wrapping_add(0xc)));
            let total = OBJECT_COUNT.get(mem).wrapping_add(objects);
            OBJECT_COUNT.set(mem, total);
            if objects == 0 {
                continue;
            }
            let mut c = 0i32;
            while c < i32::from(mem.read::<i16>(prog.wrapping_add(0x26))) {
                let e = prog.wrapping_add(0x28 + 0x10 * c.cast_unsigned());
                let rec = ptr32(rd(mem, e));
                CURRENT_PROGRAM.set(mem, c.cast_unsigned());
                if rec != 0 {
                    let runner = rd(mem, rec.wrapping_add(4));
                    if runner == 0 {
                        let first = ptr32(rd(mem, def.wrapping_add(4)));
                        let step = rd(mem, rec.wrapping_add(8));
                        let draw = rd(mem, rec.wrapping_add(0xc));
                        let hidden = mem.read::<u8>(prog.wrapping_add(0xc)) & 1 != 0;
                        let n = u32::from(mem.read::<u16>(def.wrapping_add(0xc)));
                        let cc = c.cast_unsigned();
                        let fs = [step, draw];
                        if (step != 0 || (draw != 0 && !hidden))
                            && run_program(mem, prog, cc, e, first, n, fs, checked, call)
                        {
                            return true;
                        }
                    } else {
                        call(mem, runner, &[def, prog, e, c.cast_unsigned()]);
                    }
                }
                c += 1;
            }
            if STOP.get(mem) == 0 {
                let mut o = ptr32(rd(mem, def.wrapping_add(4)));
                let mut n = u32::from(mem.read::<u16>(def.wrapping_add(0xc)));
                loop {
                    let f = rd(mem, o.wrapping_add(0xc)).wrapping_add(0x1000);
                    mem.write(o.wrapping_add(0xc), f);
                    o = ptr32(rd(mem, o));
                    n = n.wrapping_sub(1);
                    if n == 0 {
                        break;
                    }
                }
            }
        }
        false
    }

    /// The end of [`ppp_run_part_std`]'s frame: (unless stopped) the
    /// restarts and deletions of the part's objects; the current program
    /// back to 0.
    fn end_objects<M: Mem>(
        mem: &mut M,
        part: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        if STOP.get(mem) == 0 {
            let ending = mem.read::<u8>(part.wrapping_add(0x1d)) != 0;
            let mut ends = 0u32;
            let mut link = part.wrapping_add(0x30);
            let mut o = ptr32(rd(mem, link));
            while o != 0 {
                let d = ptr32(rd(mem, o.wrapping_add(8)));
                let p = ptr32(rd(mem, d));
                let next = ptr32(rd(mem, o));
                let frame = rd(mem, o.wrapping_add(0xc));
                if !ending && rd(mem, p.wrapping_add(0x1c)).cast_signed() <= frame.cast_signed() {
                    let restart = rd(mem, p.wrapping_add(0x18));
                    mem.write(o.wrapping_add(0xc), restart);
                    ppp_pobject_con(mem, o, 0x20, call);
                    link = o;
                } else {
                    let end = rd(mem, p.wrapping_add(0x14));
                    ends |= end;
                    if end <= frame {
                        mem.write(link, ptr32(next));
                        ppp_delete_pobject(mem, o, call);
                    } else {
                        link = o;
                    }
                }
                o = next;
            }
            if ending && ends == 0x7000_0000 {
                let mut o = ptr32(rd(mem, part.wrapping_add(0x30)));
                while o != 0 {
                    let next = ptr32(rd(mem, o));
                    ppp_delete_pobject(mem, o, call);
                    o = next;
                }
                mem.write(part.wrapping_add(0x30), ptr32(0));
            }
        }
        CURRENT_PROGRAM.set(mem, 0);
    }

    /// `pppRunPart(part, flags)` (`0x00712250`): makes `part` current; with
    /// flag 0x10 the programs' `+0x14` functions, then the births and the
    /// frame ([`ppp_run_part_std`]), with 0x20 their `+0x18`; then (unless
    /// stopped) the part's frame (`+8`, the previous at `+0xc`) advances
    /// by its rate (`+0x10`). Returns 1 when a part without a loop (`+0x1c`
    /// 0) has reached its length (`+4`), or an ending part has no objects
    /// left; a looping part's frame stays below 0x70000000.
    pub fn ppp_run_part<M: Mem>(
        mem: &mut M,
        part: u32,
        flags: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        CURRENT_PART.set(mem, part);
        if flags & 0x10 != 0 {
            ppp_part_call(mem, part, 0x14, call);
        }
        ppp_born_pobjects(mem, part, call);
        ppp_run_part_std(mem, part, call);
        if flags & 0x20 != 0 {
            ppp_part_call(mem, part, 0x18, call);
        }
        if STOP.get(mem) != 0 {
            return 0;
        }
        let frame = rd(mem, part.wrapping_add(8));
        mem.write(part.wrapping_add(0xc), frame);
        let next = rd(mem, part.wrapping_add(0x10)).wrapping_add(frame);
        mem.write(part.wrapping_add(8), next);
        if mem.read::<u8>(part.wrapping_add(0x1c)) == 0 {
            return u32::from(rd(mem, part.wrapping_add(4)).cast_signed() <= next.cast_signed());
        }
        if mem.read::<u8>(part.wrapping_add(0x1d)) != 0 {
            return is_ptr32_null(rd(mem, part.wrapping_add(0x30)));
        }
        if next.cast_signed() > 0x6fff_ffff {
            let rate = rd(mem, part.wrapping_add(0x10));
            mem.write(part.wrapping_add(0xc), 0x7000_0000u32.wrapping_sub(rate));
            mem.write(part.wrapping_add(8), 0x7000_0000u32);
        }
        0
    }

    /// `pppStartPart(part, rate, data, call_set_ups)` (`0x007123c0`): makes
    /// `part` current and starts it from its data block: length, loop
    /// flag, programs (`+8`, counted) and births (`+0xc`, counted; 16-byte
    /// records from the environment's heaps, one per object definition of
    /// the chain at `+0x10`); with `call_set_ups` the programs' `+0x10`
    /// functions. A part with 0x33 births in the language 0xb during spell
    /// 0x296 has one value patched (the original's localisation fix).
    /// Allocation failure leaves `part` at `0xfffff000`.
    pub fn ppp_start_part<M: Mem>(
        mem: &mut M,
        part: u32,
        rate: u32,
        data: u32,
        call_set_ups: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let d = ptr32(data);
        let lang_fn = TO_GET_FFX_LANG.ptr(mem).addr;
        let lang = call(mem, lang_fn, &[]);
        let id = ffx_system::instance::bridge::magic_id(mem);
        if !start_part(mem, part, rate, d, call_set_ups, call) {
            return;
        }
        let w = |i: u32| part.wrapping_add(4 * i);
        if rd(mem, w(6)) == 0x33 && lang == 0xb && id == 0x296 {
            let a = ptr32(rd(mem, rd(mem, w(0xe)).wrapping_add(0x38)));
            let b = ptr32(rd(mem, a.wrapping_add(0x70)));
            mem.write(b.wrapping_add(0x10), 0xc37a_0000u32);
            mem.write(b.wrapping_add(0x14), 0x4150_0000u32);
        }
    }

    /// `pppStartPartFromYonish(part, rate, data, call_set_ups)`
    /// (`0x00712560`): [`ppp_start_part`] on a data block given by its
    /// address rather than a `ptr32`, without the localisation patch.
    pub fn ppp_start_part_from_yonish<M: Mem>(
        mem: &mut M,
        part: u32,
        rate: u32,
        data: u32,
        call_set_ups: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        start_part(mem, part, rate, data, call_set_ups, call);
    }

    /// The body [`ppp_start_part`] and [`ppp_start_part_from_yonish`]
    /// share, on the data block at `d`; false when the birth records
    /// could not be allocated. A block without births faults in the
    /// originals (the record loop writes through the null records
    /// pointer); the port writes at 0 then.
    fn start_part<M: Mem>(
        mem: &mut M,
        part: u32,
        rate: u32,
        d: u32,
        call_set_ups: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> bool {
        CURRENT_PART.set(mem, part);
        let w = |i: u32| part.wrapping_add(4 * i);
        mem.write(w(0x1e), 0u32);
        mem.write(w(0x1c), 0x1000_1000u32);
        mem.write(w(0x1d), 0x1000_1000u32);
        let len = rd(mem, d);
        mem.write(w(1), len);
        let looping: u8 = mem.read(d.wrapping_add(4));
        mem.write(w(7), looping);
        let progs = rd(mem, d.wrapping_add(8)).wrapping_add(d);
        let births = rd(mem, d.wrapping_add(0xc)).wrapping_add(d);
        let n = rd(mem, progs);
        mem.write(w(5), n);
        mem.write(w(0xd), progs.wrapping_add(4));
        let nb = rd(mem, births);
        mem.write(w(6), nb);
        mem.write(w(0xe), births.wrapping_add(4));
        let chain = d.wrapping_add(0x10);
        if nb == 0 {
            mem.write(w(0xf), 0u32);
        } else {
            let env = rd(mem, ENV_SLOT.get(mem));
            let recs =
                crate::heap::bridge::ppp_mem_alloc(mem, env, env.wrapping_add(0x10), nb << 4);
            mem.write(w(0xf), recs);
            if recs == 0 {
                mem.write(part, 0xffff_f000u32);
                return false;
            }
        }
        // (the original first clears w(0xb) when the chain's first link is
        // set: a dead store, overwritten below before any read)
        mem.write(w(2), 0u32);
        mem.write(part.wrapping_add(0x1d), 0u8);
        let nb = rd(mem, w(6));
        mem.write(w(0xb), if nb != 0 { chain } else { 0 });
        mem.write(w(0xc), ptr32(0));
        let mut r = rd(mem, w(0xf));
        mem.write(w(4), rate);
        let mut node = chain;
        let mut k = 0u8;
        loop {
            mem.write(r, ptr32(node));
            let frame = rd(mem, node.wrapping_add(0x10));
            mem.write(r + 8, frame);
            mem.write(r + 4, ptr32(0));
            mem.write(r + 0xc, 0u16);
            mem.write(r + 0xe, k);
            k = k.wrapping_add(1);
            r = r.wrapping_add(0x10);
            node = ptr32(rd(mem, node));
            if node == 0 {
                break;
            }
        }
        if call_set_ups != 0 {
            ppp_part_call(mem, part, 0x10, call);
        }
        true
    }

    /// `pppCallUsrFunc(part, table, arg)` (`0x00716710`): for each of the
    /// part's objects, `table[k](obj, arg)` where `k` is its birth record's
    /// index byte (`+0xe`), when that entry is set.
    pub fn ppp_call_usr_func<M: Mem>(
        mem: &mut M,
        part: u32,
        table: u32,
        arg: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let mut o = ptr32(rd(mem, part.wrapping_add(0x30)));
        while o != 0 {
            let def = ptr32(rd(mem, o.wrapping_add(8)));
            let k = u32::from(mem.read::<u8>(def.wrapping_add(0xe)));
            let next = ptr32(rd(mem, o));
            let f = rd(mem, table.wrapping_add(4 * k));
            if f != 0 {
                call(mem, f, &[o, arg]);
            }
            o = next;
        }
    }

    /// Work area `i` of `obj` (`obj + offsets[i]`, the area's fields from
    /// `+0xa0`).
    fn area(mem: &impl Mem, obj: u32, ctrl: u32, i: u32) -> u32 {
        let offsets = rd(mem, ctrl.wrapping_add(0xc));
        obj.wrapping_add(rd(mem, offsets.wrapping_add(4 * i)))
    }

    /// `pppDrawHookDraw(obj, key, ctrl)` (`0x00757370`): area 0 takes the key's
    /// `+4`, and the user function of the object's birth record (when
    /// `g_pppUserFuncs` is set) is called as `f(obj, that)`.
    pub fn ppp_draw_hook<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        ctrl: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let w = area(mem, obj, ctrl, 0);
        let def = ptr32(rd(mem, obj.wrapping_add(8)));
        let v = rd(mem, key.wrapping_add(4));
        mem.write(w.wrapping_add(0xa0), v);
        let table = USER_FUNCS.get(mem);
        if table != 0 {
            let k = u32::from(mem.read::<u8>(def.wrapping_add(0xe)));
            let f = rd(mem, table.wrapping_add(4 * k));
            if f != 0 {
                let v = rd(mem, key.wrapping_add(4));
                call(mem, f, &[obj, v]);
            }
        }
    }

    /// A child object of the current part from birth record `index`
    /// (`None` when it cannot be made; `Some(0)` for a null record, as the
    /// originals write through it then), its parent `obj`.
    fn born_child<M: Mem>(
        mem: &mut M,
        obj: u32,
        index: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> Option<u32> {
        let part = CURRENT_PART.get(mem);
        let rec = index
            .wrapping_mul(0x10)
            .wrapping_add(rd(mem, part.wrapping_add(0x3c)));
        if rec == 0 {
            return Some(0);
        }
        let child = ppp_create_pobject(mem, part, rec, call);
        if child == 0 {
            return None;
        }
        mem.write(child.wrapping_add(4), ptr32(obj));
        Some(child)
    }

    /// `pppPointAp(obj, key, ctrl)` (`0x007573c0`): every `+0xc` frames
    /// (area 1's countdown byte `+0xa1`), a child object (birth record key
    /// `+4`, none for -1) at area 0's position (into the child's field at
    /// key `+8`).
    pub fn ppp_point_ap<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        ctrl: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let a0 = area(mem, obj, ctrl, 0);
        let a1 = area(mem, obj, ctrl, 1);
        if STOP.get(mem) != 0 {
            return;
        }
        let count = a1.wrapping_add(0xa1);
        if mem.read::<u8>(count) == 0 {
            let index = rd(mem, key.wrapping_add(4));
            if index == u32::MAX {
                return;
            }
            let Some(child) = born_child(mem, obj, index, call) else {
                return;
            };
            let at = rd(mem, key.wrapping_add(8)).wrapping_add(child);
            for i in 0..3 {
                let v = rd(mem, a0.wrapping_add(0xa0 + 4 * i));
                mem.write(at.wrapping_add(0xa0 + 4 * i), v);
            }
            let n: u8 = mem.read(key.wrapping_add(0xc));
            mem.write(count, n);
        }
        let n = mem.read::<u8>(count).wrapping_sub(1);
        mem.write(count, n);
    }

    /// `pppBornShort(obj, key, v)` (`0x00758270`): a child object (birth
    /// record key `+0xc`, none for -1) whose `i16` at key `+0x10` is `v`.
    pub fn ppp_born_short<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        v: u16,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let index = rd(mem, key.wrapping_add(0xc));
        if index == u32::MAX {
            return;
        }
        let Some(child) = born_child(mem, obj, index, call) else {
            return;
        };
        let at = rd(mem, key.wrapping_add(0x10))
            .wrapping_add(0xa0)
            .wrapping_add(child);
        mem.write(at, v);
    }

    /// `pppBornVertex` (`0x00757770`, `relative` false) and
    /// `pppBornVertexRel` (`0x00757b50`, true) `(obj, key, v)`: a child
    /// object (birth record key `+0xc`, none for -1) at the vertex `v`
    /// (three `i16`) through the object's matrix (`+0x10`), less the
    /// world matrix's translation when `relative`, into its field at key
    /// `+0x10`.
    pub fn ppp_born_vertex<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        v: u32,
        relative: bool,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let v = vertex(mem, v);
        born_vertex(mem, obj, key, v, relative, call);
    }

    /// Three `i16` at `at`.
    fn vertex(mem: &impl Mem, at: u32) -> [i16; 3] {
        [0, 2, 4].map(|i| mem.read::<i16>(at.wrapping_add(i)))
    }

    fn born_vertex<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        v: [i16; 3],
        relative: bool,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_bridge::x87::Reg;
        let index = rd(mem, key.wrapping_add(0xc));
        if index == u32::MAX {
            return;
        }
        let Some(child) = born_child(mem, obj, index, call) else {
            return;
        };
        let p: [f32; 4] = [f32::from(v[0]), f32::from(v[1]), f32::from(v[2]), 1.0];
        let m = obj.wrapping_add(0x10);
        let q: [f32; 3] = std::array::from_fn(|c| {
            #[allow(clippy::cast_possible_truncation)] // 0..3
            let c = c as u32;
            let col = [0, 4, 8, 12].map(|r| mem.read::<f32>(m.wrapping_add(4 * (r + c))));
            ffx_system::math::mul4x4_component(c, p, col)
        });
        let at = rd(mem, key.wrapping_add(0x10))
            .wrapping_add(0xa0)
            .wrapping_add(child);
        let t = crate::mat::bridge::WORLD.ptr(mem).addr.wrapping_add(0x30);
        for i in 0..3u32 {
            let x = if relative {
                Reg::load(q[i as usize])
                    .sub_m32(mem.read(t.wrapping_add(4 * i)))
                    .store()
            } else {
                q[i as usize]
            };
            mem.write(at.wrapping_add(4 * i), x);
        }
    }

    /// `pppPointRAp(obj, key, ctrl)` (`0x00757490`): every `+0x1c` frames
    /// (area 1's countdown byte `+0xa1`), a child object (birth record key
    /// `+0xc`) on a random direction: two rand-program draws `d` give the
    /// angles `ftol(d 32768 - 16384)` and `ftol(2 d 32768)` (sine table
    /// steps); the child's position (its field at key `+0x10`) is area 0's
    /// plus the direction times key `+4`, its velocity (field at key
    /// `+0x18`) the same direction times key `+8`.
    pub fn ppp_point_r_ap<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        ctrl: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        srand: &mut impl FnMut(&mut M, u32),
    ) {
        use crate::x87::{add, mul};
        let a1 = area(mem, obj, ctrl, 1);
        if STOP.get(mem) != 0 {
            return;
        }
        let count = a1.wrapping_add(0xa1);
        if mem.read::<u8>(count) == 0 {
            let a0 = area(mem, obj, ctrl, 0);
            let index = rd(mem, key.wrapping_add(0xc));
            if index == u32::MAX {
                return;
            }
            let Some(child) = born_child(mem, obj, index, call) else {
                return;
            };
            let sin = |mem: &M, i: u32| -> f32 {
                let t = crate::ppp::bridge::SIN.ptr(mem);
                f32::from_bits(t.at(i).get(mem))
            };
            let idx = |a: u32| (a.cast_signed() >> 4).cast_unsigned() & 0xfff;
            let f = |mem: &M, at: u32| mem.read::<f32>(key.wrapping_add(at));
            let d = crate::rand::bridge::draw(mem, srand);
            let a = ffx_bridge::x87::ftol(mem, f64::from(d) * 32768.0 - 16384.0);
            let r = f(mem, 4);
            let c4 = mul(sin(mem, idx(a.wrapping_add(0x4000))), r);
            let s5 = mul(sin(mem, idx(a)), r);
            let d = crate::rand::bridge::draw(mem, srand);
            let b = ffx_bridge::x87::ftol(mem, f64::from(d) * 32768.0 * 2.0);
            let v6 = mul(sin(mem, idx(b)), c4);
            let v4 = mul(c4, sin(mem, idx(b.wrapping_add(0x4000))));
            let pos = rd(mem, key.wrapping_add(0x10)).wrapping_add(child);
            let vel = rd(mem, key.wrapping_add(0x18)).wrapping_add(child);
            let dir = [v6, s5, v4];
            for i in 0..3u32 {
                let p0: f32 = mem.read(a0.wrapping_add(0xa0 + 4 * i));
                mem.write(pos.wrapping_add(0xa0 + 4 * i), add(p0, dir[i as usize]));
            }
            let k = f(mem, 8);
            for i in 0..3u32 {
                mem.write(vel.wrapping_add(0xa0 + 4 * i), mul(dir[i as usize], k));
            }
            let n: u8 = mem.read(key.wrapping_add(0x1c));
            mem.write(count, n);
        }
        let n = mem.read::<u8>(count).wrapping_sub(1);
        mem.write(count, n);
    }

    /// `pppKeHitBorn(obj, key, ctrl)` (`0x00759d80`): with its key, area 1's
    /// flag bit 1 (`+0xa0`) from the key's byte `+0xc`. Unless stopped or
    /// the key's birth record (`+4`) is -1: for each hit node of the list
    /// area 0 points at (`+0xa0`, linked by the first word) with flag 0x800
    /// (`u16` at `+0x3e`; without the bit, only those also without 0x40,
    /// and nothing during spell 0xcb), a child object at the node's point
    /// (`+0x40 + 0x30 [+0x36]`, into its field at key `+8`).
    pub fn ppp_ke_hit_born<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        ctrl: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let a1 = area(mem, obj, ctrl, 1).wrapping_add(0xa0);
        if rd(mem, obj.wrapping_add(0xc)) == rd(mem, key) {
            let b: u8 = mem.read(a1);
            let on = mem.read::<u8>(key.wrapping_add(0xc)) != 0;
            mem.write(a1, if on { b | 2 } else { b & 0xfd });
        }
        let index = rd(mem, key.wrapping_add(4));
        if STOP.get(mem) != 0 || index == u32::MAX {
            return;
        }
        let a0 = area(mem, obj, ctrl, 0);
        let mut node = ptr32(rd(mem, a0.wrapping_add(0xa0)));
        let all = mem.read::<u8>(a1) & 2 != 0;
        while node != 0 {
            let flags = mem.read::<u16>(node.wrapping_add(0x3e));
            if flags & 0x800 != 0 && (all || flags & 0x40 == 0) {
                if !all && ffx_system::instance::bridge::magic_id(mem) == 0xcb {
                    return;
                }
                let Some(child) = born_child(mem, obj, index, call) else {
                    return;
                };
                let n = u32::from(mem.read::<u8>(node.wrapping_add(0x36)));
                let dst = rd(mem, key.wrapping_add(8))
                    .wrapping_add(0xa0)
                    .wrapping_add(child);
                let src = node.wrapping_add(0x40).wrapping_add(0x30 * n);
                ffx_field::vu0::bridge::ke_vu0_copy_vec_xyz(mem, dst, src);
            }
            node = ptr32(rd(mem, node));
        }
    }

    /// `graphicLevelGetNameHash()` (`0x00641da0`), called through `call`.
    const GRAPHIC_LEVEL_GET_NAME_HASH: Global<u32> = Global::new(0x0064_1da0);
    /// `getScenerioFlag()` (`0x00787360`), called through `call`.
    const GET_SCENERIO_FLAG: Global<u32> = Global::new(0x0078_7360);
    /// `AtelGetEffectFlag(bit)` (`0x0086b030`), called through `call`.
    const ATEL_GET_EFFECT_FLAG: Global<u32> = Global::new(0x0086_b030);
    /// `g_CurrentMapNo`.
    const CURRENT_MAP_NO: Global<u32> = Global::new(0x012f_bc40);
    /// The two words `pppBornVertexLc`'s map rule tests (`0x0113335c` is 0,
    /// `0x00c3a504` is not -1).
    const LC_RULE: [Global<u32>; 2] = [Global::new(0x0113_335c), Global::new(0x00c3_a504)];

    fn call0<M: Mem>(
        mem: &mut M,
        g: Global<u32>,
        args: &[u32],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = g.ptr(mem).addr;
        call(mem, f, args)
    }

    /// `pppBornVertexLc(obj, key, v)` (`0x00757ed0`): a child object
    /// (birth record key `+0xc`) at the vertex `v` as it is (floats, into
    /// its field at key `+0x10`); on level 0x18cd or map 0x10e (with
    /// `0x0113335c` 0 and `0x00c3a504` not -1), when the parent has no more
    /// frames left than the child (in 0x1000 steps), the parent's
    /// program's end (`+0x14`) moves on by 0x1000.
    pub fn ppp_born_vertex_lc<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        v: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let v = vertex(mem, v);
        born_lc(mem, obj, key, v, call);
    }

    fn born_lc<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        v: [i16; 3],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let index = rd(mem, key.wrapping_add(0xc));
        if index == u32::MAX {
            return;
        }
        let Some(child) = born_child(mem, obj, index, call) else {
            return;
        };
        let hash = call0(mem, GRAPHIC_LEVEL_GET_NAME_HASH, &[], call);
        if (hash == 0x18cd || CURRENT_MAP_NO.get(mem) == 0x10e)
            && LC_RULE[0].get(mem) == 0
            && LC_RULE[1].get(mem) != u32::MAX
        {
            let left = |mem: &M, o: u32| {
                let d = ptr32(rd(mem, o.wrapping_add(8)));
                let p = ptr32(rd(mem, d));
                let n = rd(mem, p.wrapping_add(0x14))
                    .wrapping_sub(rd(mem, o.wrapping_add(0xc)))
                    .cast_signed();
                // the original's rounding toward zero of n / 0x1000
                (n + ((n >> 31) & 0xfff)) >> 12
            };
            let c = left(mem, child);
            if left(mem, obj) <= c {
                let d = ptr32(rd(mem, obj.wrapping_add(8)));
                let p = ptr32(rd(mem, d));
                let end = rd(mem, p.wrapping_add(0x14)).wrapping_add(0x1000);
                mem.write(p.wrapping_add(0x14), end);
            }
        }
        let at = rd(mem, key.wrapping_add(0x10))
            .wrapping_add(0xa0)
            .wrapping_add(child);
        for (i, x) in v.into_iter().enumerate() {
            #[allow(clippy::cast_possible_truncation)] // 0..3
            mem.write(at.wrapping_add(4 * i as u32), f32::from(x));
        }
    }

    /// The forms of the `pppVertexAp` steps.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum VertexAp {
        /// `pppVertexAp` (`0x00757840`): children through the object's
        /// matrix ([`ppp_born_vertex`]), and the localisation rule.
        Ap,
        /// `pppVertexApDisPos` (`0x00757c30`): less the world translation.
        DisPos,
        /// `pppVertexApLc` (`0x00757ff0`): the vertex itself
        /// ([`ppp_born_vertex_lc`]).
        Lc,
        /// `pppVertexApAt` (`0x007582e0`): the vertex index
        /// ([`ppp_born_short`]).
        At,
    }

    /// The `pppVertexAp` steps `(obj, key, ctrl)`: unless stopped or the
    /// key's shape (`i16` at `+4`) is negative, every key `+7` frames
    /// (area 0's countdown `i16` at `+0xa2`), key `+6` children at vertices
    /// of the shape (the current environment's shape tables, as
    /// `pppVertexAttend` reads them; the object's own vertices at `+0x90`
    /// when it has them): in order (area 0's cursor `u16` at `+0xa0`,
    /// wrapping at the list's length), or with key `+8` = 1 at random
    /// (`ftol(length d)` of a rand-program draw `d`).
    #[allow(clippy::too_many_lines)]
    pub fn ppp_vertex_ap<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        ctrl: u32,
        form: VertexAp,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        srand: &mut impl FnMut(&mut M, u32),
    ) {
        let a0 = area(mem, obj, ctrl, 0);
        let shape = mem.read::<i16>(key.wrapping_add(4));
        if STOP.get(mem) != 0 || shape < 0 {
            return;
        }
        let countdown = a0.wrapping_add(0xa2);
        if mem.read::<u16>(countdown) == 0 {
            let env = rd(mem, ENV_SLOT.get(mem));
            let records = rd(mem, env.wrapping_add(0x28));
            let rec = records.wrapping_add((i32::from(shape) * 8).cast_unsigned());
            let verts = if form == VertexAp::At {
                0
            } else {
                let own = ptr32(rd(mem, obj.wrapping_add(0x90)));
                if own != 0 {
                    own
                } else {
                    let block = i32::from(mem.read::<i16>(rec));
                    let blocks = rd(mem, env.wrapping_add(0x20));
                    ptr32(rd(
                        mem,
                        blocks
                            .wrapping_add((block * 0x20).cast_unsigned())
                            .wrapping_add(0x14),
                    ))
                }
            };
            let n = u32::from(mem.read::<u8>(key.wrapping_add(6)));
            let random = match mem.read::<u8>(key.wrapping_add(8)) {
                0 => false,
                1 => true,
                _ => {
                    finish(mem, obj, key, countdown, form, call);
                    return;
                }
            };
            let cursor = a0.wrapping_add(0xa0);
            for _ in 0..n {
                let (index, v) = if random {
                    let d = crate::rand::bridge::draw(mem, srand);
                    let list = ptr32(rd(mem, rec.wrapping_add(4)));
                    let len = mem.read::<i16>(rec.wrapping_add(2));
                    let r = ffx_bridge::x87::ftol(mem, f64::from(len) * f64::from(d));
                    if form == VertexAp::At {
                        #[allow(clippy::cast_possible_truncation)] // the low half
                        (r as u16, [0; 3])
                    } else {
                        let k = u32::from(mem.read::<u16>(list.wrapping_add(r.wrapping_mul(2))));
                        (0, vertex(mem, verts.wrapping_add(6 * k)))
                    }
                } else {
                    let len = i32::from(mem.read::<i16>(rec.wrapping_add(2)));
                    if len <= i32::from(mem.read::<u16>(cursor)) {
                        mem.write(cursor, 0u16);
                    }
                    let c = mem.read::<u16>(cursor);
                    if form == VertexAp::At {
                        mem.write(cursor, c.wrapping_add(1));
                        (c, [0; 3])
                    } else {
                        let list = ptr32(rd(mem, rec.wrapping_add(4)));
                        let k = u32::from(mem.read::<u16>(list.wrapping_add(2 * u32::from(c))));
                        let v = vertex(mem, verts.wrapping_add(6 * k));
                        mem.write(cursor, c.wrapping_add(1));
                        (0, v)
                    }
                };
                match form {
                    VertexAp::Ap => born_vertex(mem, obj, key, v, false, call),
                    VertexAp::DisPos => born_vertex(mem, obj, key, v, true, call),
                    VertexAp::Lc => born_lc(mem, obj, key, v, call),
                    VertexAp::At => ppp_born_short(mem, obj, key, index, call),
                }
            }
            finish(mem, obj, key, countdown, form, call);
            return;
        }
        let c = mem.read::<u16>(countdown).wrapping_sub(1);
        mem.write(countdown, c);
    }

    /// The end of a `pppVertexAp` frame that made children: the countdown
    /// from key `+7` (times 1.8 for `pppVertexAp` on level 0xe6c at story
    /// 0xcc6 during a spell with effect flag 5, 6 or 7), then one frame off.
    fn finish<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        countdown: u32,
        form: VertexAp,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let _ = obj;
        mem.write(countdown, u16::from(mem.read::<u8>(key.wrapping_add(7))));
        if form == VertexAp::Ap
            && call0(mem, GRAPHIC_LEVEL_GET_NAME_HASH, &[], call) == 0xe6c
            && call0(mem, GET_SCENERIO_FLAG, &[], call) & 0xffff == 0xcc6
            && ffx_system::instance::bridge::magic_id(mem) != 0
            && (call0(mem, ATEL_GET_EFFECT_FLAG, &[5], call) != 0
                || call0(mem, ATEL_GET_EFFECT_FLAG, &[6], call) != 0
                || call0(mem, ATEL_GET_EFFECT_FLAG, &[7], call) != 0)
        {
            let n = f64::from(mem.read::<u16>(countdown));
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            // fistp with truncation, the low half
            let v = (n * f64::from(1.8f32)).trunc() as i32 as u16;
            mem.write(countdown, v);
        }
        let c = mem.read::<u16>(countdown).wrapping_sub(1);
        mem.write(countdown, c);
    }

    /// `ppp_screen_to_fixed_00761170(out, v)` (`0x00761170`): VU0's
    /// `VFTOI4` on `x`, `y` and `VFTOI0` on `z`, `w`: `v` is copied to
    /// `0x00c8f810`, then `x * 16`, `y * 16`, `z` and `w` (at x87
    /// precision) through `ftol2_sse` into `0x00c8f800..` and `out`.
    /// Returns `out.w` (left in `eax`).
    pub fn ppp_screen_to_fixed(mem: &mut impl Mem, out: u32, v: u32) -> u32 {
        let src: [u32; 4] = mem.read(v);
        mem.write(Global::<u32>::new(0x00c8_f810).ptr(mem).addr, src);
        let dst = Global::<u32>::new(0x00c8_f800).ptr(mem).addr;
        let mut fixed = [0u32; 4];
        for (i, word) in fixed.iter_mut().enumerate() {
            let x = f64::from(crate::x87::load_f32(f32::from_bits(src[i])));
            // x and y times the image's 16.0 (`0x00b313a0`), exact
            let x = if i < 2 { x * 16.0 } else { x };
            *word = ffx_bridge::x87::ftol(mem, x);
        }
        mem.write(dst, fixed);
        mem.write(out, fixed);
        fixed[3]
    }

    /// `ppp_screen_to_world_007626d0(out, p)` (`0x007626d0`): the screen
    /// point `p` back through `g_pppScreenMatrix` (`0x0230fe60`): `s = m[3][2]
    /// / (p.z - m[2][2])` (rounded to `f32`), view `(p.x / m[0][0] * s,
    /// p.y / m[1][1] * s, s, 1)` at `frame + 0x40`, `FFXVu0InversMatrix(frame,
    /// g_pppCameraMatrix)` into the 0x40 bytes at `frame`, then
    /// `FFXVu0ApplyMatrix(out, frame, frame + 0x40)`; callees through `call`.
    /// `frame` stands for the original's locals (0x50 bytes).
    pub fn ppp_screen_to_world<M: Mem>(
        mem: &mut M,
        out: u32,
        p: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_bridge::x87::{div, load, mul, store, sub};
        let g = |mem: &M, a: u32| Global::<u32>::new(a).ptr(mem).addr;
        let screen = g(mem, 0x0230_fe60);
        let m = |mem: &M, at: u32| load(mem.read::<u32>(screen + at));
        let pv: [u32; 3] = mem.read(p);
        let s = store(div(m(mem, 0x38), sub(load(pv[2]), m(mem, 0x28))));
        let x = store(mul(div(load(pv[0]), m(mem, 0)), load(s)));
        let y = store(mul(div(load(pv[1]), m(mem, 0x14)), load(s)));
        let view = frame.wrapping_add(0x40);
        mem.write(view, [x, y, s, 1.0f32.to_bits()]);
        let camera = g(mem, 0x0230_ff60);
        call(mem, g(mem, 0x006e_d450), &[frame, camera]);
        call(mem, g(mem, 0x006e_d2e0), &[out, frame, view]);
    }

    /// `0x0074a760(rot, pos, r)` and its copies `0x0074ad80`, `0x0074b540`:
    /// `r` = (object index, offset) into [`CURRENT_PART`]'s objects (`+0x3c`,
    /// 16-byte records, `+4` the data through `get_ptr32_func`); index -1
    /// gives `rot` (0, 0, _, 1) and `pos` (0, 0, _, _). Else, with `d` the
    /// data plus the offset, `rot` = the three floats at `ptr32(d + 0xac)`
    /// and the one at `d + 0xa8`, `pos` = the four `i16` at `ptr32(d +
    /// 0xb0)` times the `f64` at `0x00b51550` (each rounded to `f32`).
    /// Returns the original's eax: `pos` for index -1, else the fourth
    /// `i16` sign-extended.
    pub fn ppp_obj_rot_pos(mem: &mut impl Mem, rot: u32, pos: u32, r: u32) -> u32 {
        use ffx_bridge::x87::{load, mul, store};
        let id: u32 = mem.read(r);
        if id == u32::MAX {
            mem.write(rot + 0xc, 1.0f32.to_bits());
            mem.write(rot, 0u32);
            mem.write(rot + 4, 0u32);
            mem.write(pos, 0u32);
            mem.write(pos + 4, 0u32);
            return pos;
        }
        let part = CURRENT_PART.get(mem);
        let objs: u32 = mem.read(part.wrapping_add(0x3c));
        let base = ptr32(mem.read(objs.wrapping_add(id.wrapping_mul(16)).wrapping_add(4)));
        let off: u32 = mem.read(r + 4);
        let d = off.wrapping_add(base);
        let p = ptr32(mem.read(d.wrapping_add(0xac)));
        let q = ptr32(mem.read(d.wrapping_add(0xb0)));
        for i in 0..3u32 {
            let v: u32 = mem.read(p.wrapping_add(4 * i));
            mem.write(rot + 4 * i, store(load(v)));
        }
        let w: u32 = mem.read(d.wrapping_add(0xa8));
        mem.write(rot + 0xc, store(load(w)));
        let k: f64 = mem.read(Global::<u32>::new(0x00b5_1550).ptr(mem).addr);
        let mut last = 0i16;
        for i in 0..4u32 {
            last = mem.read(q.wrapping_add(2 * i));
            #[allow(clippy::cast_precision_loss)] // an i16: exact
            let f = f32::from(last);
            mem.write(pos + 4 * i, store(mul(f64::from(f), k)));
        }
        i32::from(last).cast_unsigned()
    }

    /// `0x0074b9c0` (`pppNeiPointLightAdd`, ours) `(obj, r, acc)`: one
    /// point light of `pppNeiDrawShapePointLightDraw` added to the colour
    /// `acc` (4 `f32`). `r` = (object index, offset) as in
    /// [`ppp_obj_rot_pos`]; index -1 adds nothing. With `d` the data plus
    /// the offset plus 0xa0: the light at `ptr32(d +0xc)` (3 `f32`), its
    /// colour the 4 `i16` at `ptr32(d +0x10)`, its radius `f32 d +8`.
    /// Through the emulated VU0: `0x00c8f820` = (the light, `stale`: the
    /// original's uninitialised local), `0x00c8f830` = `obj +0x40..+0x48`
    /// minus the light (`obj +0x4c` kept), `restoreVf00Register`, then the
    /// squares in `0x00c8f830`, their running sum in its x, `Q` = the
    /// square root of its absolute value and `x` = `vf00.x + Q`, the
    /// distance `s`. Each colour word / 64 is added when the radius is not
    /// below `s` (or either is a NaN), else times `f32(radius / s / 64)`.
    #[allow(clippy::many_single_char_names)]
    pub fn ppp_nei_point_light_add<M: Mem>(mem: &mut M, obj: u32, r: u32, acc: u32, stale: u32) {
        use ffx_bridge::x87::{Reg, round_trip};
        let id: u32 = mem.read(r);
        if id == u32::MAX {
            return;
        }
        let part = CURRENT_PART.get(mem);
        let objs: u32 = mem.read(part.wrapping_add(0x3c));
        let base = ptr32(mem.read(objs.wrapping_add(id.wrapping_mul(16)).wrapping_add(4)));
        let off: u32 = mem.read(r + 4);
        let d = off.wrapping_add(0xa0).wrapping_add(base);
        let light = ptr32(mem.read(d.wrapping_add(0xc)));
        let col = ptr32(mem.read(d.wrapping_add(0x10)));
        let f = |mem: &M, at: u32| f32::from_bits(mem.read(at));
        let l = [0, 4, 8].map(|i| round_trip(f(mem, light.wrapping_add(i))));
        let vf = |mem: &M, at: u32| Global::<u32>::new(at).ptr(mem).addr;
        let (acc_v, len_v) = (vf(mem, 0x00c8_f820), vf(mem, 0x00c8_f830));
        mem.write(acc_v, l);
        mem.write(acc_v + 0xc, stale);
        let mut v = [0f32; 3];
        for (i, x) in (0u32..).zip(v.iter_mut()) {
            *x = Reg::load(f(mem, obj.wrapping_add(0x40 + 4 * i)))
                .sub_m32(l[i as usize])
                .store();
        }
        let w: u32 = mem.read(obj.wrapping_add(0x4c));
        mem.write(len_v, v);
        mem.write(len_v + 0xc, w);
        ffx_ps2::ee::bridge::restore_vf00(mem);
        let xx = (Reg::load(v[0]) * Reg::load(v[0])).store();
        let yy = Reg::load(v[1]).mul_m32(v[1]).store();
        let zz = Reg::load(v[2]).mul_m32(v[2]).store();
        let s1 = Reg::load(yy).add_m32(xx).store();
        let s2 = Reg::load(zz).add_m32(s1).store();
        mem.write(len_v + 4, yy);
        mem.write(len_v + 8, zz);
        let q = Reg::load(f32::from_bits(s2.to_bits() & 0x7fff_ffff))
            .sqrt()
            .store();
        mem.write(vf(mem, 0x00c8_f788), q);
        let vf00x = f32::from_bits(crate::prog::bridge::VF00.get(mem)[0]);
        let dist = Reg::load(vf00x).add_m32(q).store();
        mem.write(len_v, dist);
        let radius = f(mem, d.wrapping_add(8));
        let c = |mem: &M, i: u32| Reg::load(f32::from(mem.read::<i16>(col.wrapping_add(2 * i))));
        let a = |mem: &M, i: u32| f(mem, acc.wrapping_add(4 * i));
        let k = if radius < dist {
            Reg::load((Reg::load(dist).divr_m32(radius) * Reg::constant(0.015_625)).store())
        } else {
            Reg::constant(0.015_625)
        };
        // each sum stored before the next word is read
        let x = (c(mem, 0) * k + Reg::load(a(mem, 0))).store();
        mem.write(acc, x);
        for i in 1..3 {
            let x = (c(mem, i) * k).add_m32(a(mem, i)).store();
            mem.write(acc.wrapping_add(4 * i), x);
        }
        let x = k.mul_m32(c(mem, 3).store()).add_m32(a(mem, 3)).store();
        mem.write(acc.wrapping_add(0xc), x);
    }

    /// `hideDisplayOffParticle(part)` (`0x00728e40`): for each of the
    /// part's objects (`+0x18` of them, signed; `+0x3c` the 16-byte records,
    /// read again each turn) whose data `+4` is set: `a` = that data, `c` =
    /// `ptr32(ptr32(a + 8))` (`get_ptr32_func`, the identity on PC), and
    /// each of `c`'s `i16 +0x26` entries (16 bytes from `+0x28`) holding a
    /// pointer gets `graphicVFXSetVisible(k, a, 0)` (a 64-bit id `a:k`;
    /// through `call`).
    pub fn hide_display_off_particle<M: Mem>(
        mem: &mut M,
        part: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let set_visible = Global::<u32>::new(0x0064_5710).ptr(mem).addr;
        let mut i = 0u32;
        while i.cast_signed() < mem.read::<u32>(part.wrapping_add(0x18)).cast_signed() {
            let objs: u32 = mem.read(part.wrapping_add(0x3c));
            let rec = objs.wrapping_add(0x10 * i);
            if objs != 0 && rec != 0 {
                let data: u32 = mem.read(rec.wrapping_add(4));
                if is_ptr32_null(data) == 0 {
                    let a = ptr32(data);
                    let b = ptr32(mem.read(a.wrapping_add(8)));
                    let c = ptr32(mem.read(b));
                    let mut k = 0u32;
                    while k.cast_signed() < i32::from(mem.read::<i16>(c.wrapping_add(0x26))) {
                        let e = ptr32(mem.read(c.wrapping_add(0x28 + 0x10 * k)));
                        if e != 0 {
                            call(mem, set_visible, &[k, a, 0]);
                        }
                        k += 1;
                    }
                }
            }
            i += 1;
        }
    }

    /// `0x00761350(m, a, b, p4, p5)` (`pppLinePoints`, ours; called by
    /// `0x007625a0`): nine points from `a` toward `b` in steps of `(b - a)
    /// x [0x00b92200]` (1/8, `f32` each), each through `m`
    /// (`FFXVu0ApplyMatrix`, `w` 1). A point in front (`w > 0`) is divided
    /// by its `w` (`FFXVu0DivVectorXYZ`) and, unless `0x0062fc90(v)` (a
    /// PS2 check, a stub giving 0 on PC) says otherwise, turned into screen
    /// words (`ppp_screen_to_fixed`) in a 0x20-byte record (`+0x10` the
    /// `w`, byte `+0x14` 0; 1 for a point not drawn). From the second
    /// point on, two drawn neighbours go to `0x00760ce0(prev +0, prev +4,
    /// cur +0, cur +4, p4, p5)`. The original's locals at `frame` (its
    /// `ebp`: the step at `-0x38`, the point `-0x14`, the transformed one
    /// `-0x24`, the records `-0x58` and `-0x78`). Callees through `call`.
    #[allow(clippy::many_single_char_names)]
    pub fn ppp_line_points<M: Mem>(
        mem: &mut M,
        [m, a, b, p4, p5]: [u32; 5],
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use crate::x87::{ADD, MUL, Reg, SUB};
        let local = |o: u32| frame.wrapping_sub(o);
        let (step, pos, v, cur, prev) = (
            local(0x38),
            local(0x14),
            local(0x24),
            local(0x58),
            local(0x78),
        );
        let k: f64 = mem.read(Global::<u32>::new(0x00b9_2200).ptr(mem).addr);
        for i in 0..3u32 {
            let bi: u32 = mem.read(b.wrapping_add(4 * i));
            let ai: f32 = mem.read(a.wrapping_add(4 * i));
            let d = Reg::load(bi).mem(SUB, ai).constant(MUL, k).store();
            mem.write(step.wrapping_add(4 * i), d);
        }
        for i in 0..3u32 {
            let ai: u32 = mem.read(a.wrapping_add(4 * i));
            mem.write(pos.wrapping_add(4 * i), crate::x87::load(ai));
        }
        mem.write(pos.wrapping_add(0xc), 1f32);
        let sym = |mem: &M, at: u32| Global::<u32>::new(at).ptr(mem).addr;
        for n in 0..9 {
            call(mem, sym(mem, 0x006e_d2e0), &[v, m, pos]);
            let w: u32 = mem.read(v.wrapping_add(0xc));
            let mut flag = 1u8;
            if Reg::load(w).value() > 0.0 {
                call(mem, sym(mem, 0x006e_d420), &[v, v, crate::x87::load(w)]);
                if call(mem, sym(mem, 0x0062_fc90), &[v]) == 0 {
                    call(mem, sym(mem, 0x0076_1170), &[cur, v]);
                    let w: u32 = mem.read(v.wrapping_add(0xc));
                    mem.write(cur.wrapping_add(0x10), crate::x87::load(w));
                    flag = 0;
                }
            }
            mem.write(cur.wrapping_add(0x14), flag);
            if n != 0 && flag == 0 && mem.read::<u8>(prev.wrapping_add(0x14)) == 0 {
                let args = [
                    mem.read(prev),
                    mem.read(prev.wrapping_add(4)),
                    mem.read(cur),
                    mem.read(cur.wrapping_add(4)),
                    p4,
                    p5,
                ];
                call(mem, sym(mem, 0x0076_0ce0), &args);
            }
            for i in 0..3u32 {
                let p: u32 = mem.read(pos.wrapping_add(4 * i));
                let d: f32 = mem.read(step.wrapping_add(4 * i));
                mem.write(pos.wrapping_add(4 * i), Reg::load(p).mem(ADD, d).store());
            }
            let record: [u32; 8] = mem.read(cur);
            mem.write(prev, record);
        }
    }

    // ---- round 35 c ----

    /// `0x007625a0` (`pppDrawDebugAxes`, ours; `()`, called by
    /// `pppPartDraw` and `pppPartLoop`): three lines of
    /// [`ppp_line_points`] through `m = g_pppScreenMatrix x
    /// g_pppCameraMatrix` (`FFXVu0MulMatrix`), each 200 long and centred on
    /// the point `(x, y, z)` at `0x00d4b4d8`: along x (colour 0xff8080ff),
    /// along y (0xff80ff80) and along z (0xff80ffff), scale 1.0; the ends
    /// are `x -/+ [0x00b922e0]` (100, `f32`) and so on. The matrix and the
    /// six ends (three words each) live in the original's locals at
    /// `frame` (its `ebp`): `m` at `-0x44`, the ends at `-0xa4`/`-0x94`,
    /// `-0x54`/`-0x74` and `-0x84`/`-0x64`. Callees through `call`.
    pub fn ppp_draw_debug_axes<M: Mem>(
        mem: &mut M,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_bridge::x87::{add, load, store, sub};
        let g = |mem: &M, a: u32| Global::<u32>::new(a).ptr(mem).addr;
        let at = |off: u32| frame.wrapping_sub(off);
        let centre = g(mem, 0x00d4_b4d8);
        let [x, y, z] = [0, 4, 8].map(|k| load(mem.read(centre + k)));
        let d: f64 = mem.read(g(mem, 0x00b9_22e0));
        let (cx, cy, cz) = (store(x), store(y), store(z));
        // (start, end) of each axis, the moving coordinate first
        for (start, end, k, v) in [(0xa4, 0x94, 0, x), (0x54, 0x74, 1, y), (0x84, 0x64, 2, z)] {
            for (off, w) in [(start, sub(v, d)), (end, add(v, d))] {
                let mut p = [cx, cy, cz];
                p[k] = store(w);
                mem.write(at(off), p);
            }
        }
        let m = at(0x44);
        let (screen, camera) = (g(mem, 0x0230_fe60), g(mem, 0x0230_ff60));
        call(mem, g(mem, 0x006e_d5c0), &[m, screen, camera]);
        let line = g(mem, 0x0076_1350);
        for (a, b, colour) in [
            (0xa4, 0x94, 0xff80_80ff),
            (0x54, 0x74, 0xff80_ff80),
            (0x84, 0x64, 0xff80_ffff),
        ] {
            call(mem, line, &[m, at(a), at(b), colour, 0x3f80_0000]);
        }
    }

    /// `0x007372d0` (`pppPObjPosRot`, ours; `(pos, rot, target)`, called
    /// by `pppNeiChrPointLightDraw`): for a target `(n, field)` (`n` -1 for
    /// none: `pos` `x`, `y` = 0 and `w` = 1.0, `rot` `x`, `y` = 0, the
    /// rest untouched), `t = part object n + field`: `pos` = the three
    /// floats at `get_ptr32_func(t +0xac)` with 1.0 through
    /// `g_pppWorldMatrix` (`FFXVu0ApplyMatrix`; the vector in the
    /// original's locals at `frame - 0x14`, `frame` its `ebp`), then `pos
    /// +0xc` = the current part's `f32 +0x60 x t +0xa8`; `rot` = the four
    /// `i16` at `get_ptr32_func(t +0xb0)` times `[0x00b51550]` (1/16384),
    /// each `f32`. Callees through `call`.
    pub fn ppp_pobj_pos_rot<M: Mem>(
        mem: &mut M,
        pos: u32,
        rot: u32,
        target: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_bridge::x87::{load, mul, store};
        use ffx_system::stub::ptr32;
        let g = |mem: &M, a: u32| Global::<u32>::new(a).ptr(mem).addr;
        let n: u32 = mem.read(target);
        if n == u32::MAX {
            mem.write(pos, 0u32);
            mem.write(pos.wrapping_add(0xc), 1f32);
            mem.write(pos.wrapping_add(4), 0u32);
            mem.write(rot, [0u32; 2]);
            return;
        }
        let part = mem.read::<u32>(g(mem, 0x0230_ffe4));
        let records: u32 = mem.read(part.wrapping_add(0x3c));
        let o = ptr32(mem.read(records.wrapping_add(4).wrapping_add(n.wrapping_mul(16))));
        let t = mem.read::<u32>(target.wrapping_add(4)).wrapping_add(o);
        let src = ptr32(mem.read(t.wrapping_add(0xac)));
        let angles = ptr32(mem.read(t.wrapping_add(0xb0)));
        let v = frame.wrapping_sub(0x14);
        for k in 0..3 {
            let x = store(load(mem.read(src.wrapping_add(4 * k))));
            mem.write(v + 4 * k, x);
        }
        mem.write(v + 0xc, 1f32);
        let part = mem.read::<u32>(g(mem, 0x0230_ffe4));
        call(mem, g(mem, 0x006e_d2e0), &[pos, g(mem, 0x0230_ffa0), v]);
        let s = mul(
            load(mem.read(part.wrapping_add(0x60))),
            load(mem.read(t.wrapping_add(0xa8))),
        );
        mem.write(pos.wrapping_add(0xc), store(s));
        let k: f64 = mem.read(g(mem, 0x00b5_1550));
        for i in 0..4 {
            let a = f64::from(mem.read::<i16>(angles.wrapping_add(2 * i)));
            mem.write(rot.wrapping_add(4 * i), store(mul(a, k)));
        }
    }

    /// `pppNeiChrPointLightDraw(obj, key, ctrl)` (`0x007371c0`): for a
    /// target (`key +0x28` = `n`, not -1) and an area `a = obj +0xa0 +
    /// *ctrl[+0xc]` whose word `+0x80` is set: `t = part object n + key
    /// +0x2c`; bytes `a +0x88`, `+0x89` = `t +0xa8`, `t +0xb4` x 128 as
    /// `fistp` truncates them (the low byte; NaN and out of range give 0),
    /// then `pppPObjPosRot(a + 0x40 + 0x10 k, a + 0x10 k, key + 8 + 8 k)`
    /// for `k` 0..4 (through `call`).
    pub fn ppp_nei_chr_point_light_draw<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        ctrl: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_bridge::x87::{cvttsd2si, load, mul};
        let n: u32 = rd(mem, key.wrapping_add(0x28));
        if n == u32::MAX {
            return;
        }
        let off = rd(mem, rd(mem, ctrl.wrapping_add(0xc)));
        let a = obj.wrapping_add(0xa0).wrapping_add(off);
        if rd(mem, a.wrapping_add(0x80)) == 0 {
            return;
        }
        let part = rd(mem, CURRENT_PART.ptr(mem).addr);
        let records = rd(mem, part.wrapping_add(0x3c));
        let o = ptr32(rd(
            mem,
            records.wrapping_add(4).wrapping_add(n.wrapping_mul(16)),
        ));
        let t = rd(mem, key.wrapping_add(0x2c)).wrapping_add(o);
        let k: f64 = mem.read(Global::<u32>::new(0x00b4_c170).ptr(mem).addr);
        for (src, dst) in [(0xa8u32, 0x88u32), (0xb4, 0x89)] {
            let v = mul(load(rd(mem, t.wrapping_add(src))), k);
            mem.write(a.wrapping_add(dst), cvttsd2si(v).to_le_bytes()[0]);
        }
        let f = Global::<u32>::new(0x0073_72d0).ptr(mem).addr;
        for i in 0..4u32 {
            let args = [
                a.wrapping_add(0x40 + 0x10 * i),
                a.wrapping_add(0x10 * i),
                key.wrapping_add(8 + 8 * i),
            ];
            call(mem, f, &args);
        }
    }
}
