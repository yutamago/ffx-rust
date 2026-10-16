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
    const GRAPHIC_VFX_DESTROY: Global<u32> = Global::new(0x0064_5280);
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
    /// the object's (`+0xc`), then `f(obj, key, entry)`.
    #[allow(clippy::too_many_arguments)]
    fn run_program<M: Mem>(
        mem: &mut M,
        prog: u32,
        c: u32,
        e: u32,
        first: u32,
        count: u32,
        fs: [u32; 2],
        draw_too: bool,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
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
            let next = ptr32(rd(mem, o));
            if fs[0] != 0 {
                call(mem, fs[0], &[o, key, e]);
                if draw_too {
                    call(mem, fs[1], &[o, key, e]);
                }
            } else {
                call(mem, fs[1], &[o, key, e]);
            }
            // the draw-only loop moves on after its call (the same unless
            // the draw relinks the object)
            o = if fs[0] != 0 { next } else { ptr32(rd(mem, o)) };
            n = n.wrapping_sub(1);
            if n == 0 {
                break;
            }
        }
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
    #[allow(clippy::too_many_lines)]
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
        let mut b = 0i32;
        while b < rd(mem, part.wrapping_add(0x18)).cast_signed() {
            let def = rd(mem, part.wrapping_add(0x3c)).wrapping_add(0x10 * b.cast_unsigned());
            b += 1;
            if def == 0 {
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
                        if step == 0 {
                            if draw != 0 && !hidden {
                                run_program(mem, prog, cc, e, first, n, [0, draw], false, call);
                            }
                        } else {
                            let draw_too = draw != 0 && !hidden;
                            run_program(mem, prog, cc, e, first, n, [step, draw], draw_too, call);
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
                return;
            }
        }
        if is_ptr32_null(rd(mem, chain)) == 0 {
            mem.write(w(0xb), 0u32);
        }
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
        if rd(mem, w(6)) == 0x33 && lang == 0xb && id == 0x296 {
            let a = ptr32(rd(mem, rd(mem, w(0xe)).wrapping_add(0x38)));
            let b = ptr32(rd(mem, a.wrapping_add(0x70)));
            mem.write(b.wrapping_add(0x10), 0xc37a_0000u32);
            mem.write(b.wrapping_add(0x14), 0x4150_0000u32);
        }
    }
}
