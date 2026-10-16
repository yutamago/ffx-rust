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

/// The parts and objects where the game keeps them.
pub mod bridge {
    use ffx_bridge::Mem;
    use ffx_system::stub::{is_ptr32_null, ptr32};

    use crate::heap::bridge::{ENV_SLOT, MEM_ALLOC_FAILED, ppp_alloc};

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
        call: &mut impl FnMut(&mut M, u32, &[u32]),
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
        call: &mut impl FnMut(&mut M, u32, &[u32]),
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
        call: &mut impl FnMut(&mut M, u32, &[u32]),
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
        call: &mut impl FnMut(&mut M, u32, &[u32]),
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
}
