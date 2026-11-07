//! M5 stage 1, round 9 (`tail9`, list `phyre-anim-left`):
//! `PAnimatableComponent_bindTargets_00536230`, which
//! `Phyre_PAnimation_PAnimatableComponent_bind_00541c00` runs: the
//! animation set's records sorted into binder lists by their kind, over
//! the component's (object, class) pairs, and the lists handed to
//! `setNetwork` as one array of binder pointers.
//!
//! The original keeps its seven lists in its frame (their addresses go to
//! the append helpers, the temporaries' to their destructor), so the
//! bridge keeps them at the original's `ebp` offsets ([`bridge::FRAME`]);
//! the pointer array `setNetwork` gets is an alloca (`_alloca_probe_16`),
//! so the bridge is split there ([`bridge::collect`], [`bridge::finish`]).
//! Callees stay calls to the originals.

/// The originals over game memory.
#[allow(clippy::many_single_char_names)] // the original's registers
pub mod bridge {
    use ffx_graphic::host::{Conv, Host};

    /// `PhyreMalloc(size, align)`, `PhyreFree(p)` (cdecl).
    pub const MALLOC: u32 = 0x0042_fb80;
    pub const FREE: u32 = 0x0042_fb20;
    /// The append helpers (thiscall `(storage, &value)`, `ret 8`):
    /// `Phyre_PArray_appendWord_0053b250` and the four 8-byte-object
    /// copies `Phyre_PAnimatableComponent_appendObject_0053b370/4a0/540/5e0`.
    pub const APPEND_WORD: u32 = 0x0053_b250;
    pub const APPEND_X: u32 = 0x0053_b370;
    pub const APPEND_S: u32 = 0x0053_b4a0;
    pub const APPEND_V: u32 = 0x0053_b540;
    pub const APPEND_Y: u32 = 0x0053_b5e0;
    /// The binder temporaries' destructor
    /// (`Phyre_PAnimation_PAnimationChannelTargetBind_dtor`, thiscall) and
    /// `Phyre_PAnimatableComponent_setNetwork_0053a2e0` (thiscall
    /// `(network, binders, n)`, `ret 0xc`).
    pub const TEMP_DTOR: u32 = 0x005c_db70;
    pub const SET_NETWORK: u32 = 0x0053_a2e0;
    /// The class descriptors the record kinds look for (a class matches
    /// when the descriptor is on its parent chain, `+0x40`): kind 1's
    /// record class and first match, kind 2's special class and kind 1's
    /// second match, kind 5's match.
    pub const CLASS_K1: u32 = 0x00ca_91f8;
    pub const CLASS_K2: u32 = 0x00ca_7228;
    pub const CLASS_K5: u32 = 0x00ca_5040;
    /// The binder vtables: the lists S (kind 1), V (kind 1's second), T
    /// (kind 2, 12-byte elements), U (kind 3, 12-byte), X (kind 4), Y
    /// (kind 5).
    pub const VT_S: u32 = 0x00b2_a358;
    pub const VT_V: u32 = 0x00b2_a364;
    pub const VT_T: u32 = 0x00b2_a328;
    pub const VT_U: u32 = 0x00b2_a34c;
    pub const VT_X: u32 = 0x00b2_a340;
    pub const VT_Y: u32 = 0x00b2_a334;

    /// The bytes of the original's frame below `ebp` the bridge uses (the
    /// lists, their counts and the temporaries, down to `ebp - 0xd8`).
    pub const FRAME: u32 = 0xd8;

    const MASK: u32 = 0x7fff_ffff;
    const OWNED_BIT: u32 = 0x8000_0000;

    /// The frame slots (offsets below `ebp`).
    mod slot {
        // the 8-byte-object lists: (array object, its storage, the count
        // copy the original keeps)
        pub const S: u32 = 0x84;
        pub const S_PTR: u32 = 0x80;
        pub const S_N: u32 = 0x4c;
        pub const V: u32 = 0x74;
        pub const V_PTR: u32 = 0x70;
        pub const V_N: u32 = 0x28;
        pub const X: u32 = 0x7c;
        pub const X_PTR: u32 = 0x78;
        pub const X_N: u32 = 0x44;
        pub const Y: u32 = 0x60;
        pub const Y_PTR: u32 = 0x5c;
        pub const Y_N: u32 = 0x18;
        // the class word list
        pub const W: u32 = 0x8c;
        pub const W_PTR: u32 = 0x88;
        pub const W_N: u32 = 0x68;
        pub const W_VALUE: u32 = 0x20;
        // the 12-byte lists grown in place: (count, storage, count with
        // the not-owned bit)
        pub const T_N2: u32 = 0x38;
        pub const T_PTR: u32 = 0x34;
        pub const T_N: u32 = 0x30;
        pub const U_N2: u32 = 0x58;
        pub const U_PTR: u32 = 0x54;
        pub const U_N: u32 = 0x40;
        // the temporaries
        pub const TEMP_S: u32 = 0xac;
        pub const TEMP_V: u32 = 0xa4;
        pub const TEMP_T: u32 = 0xc0;
        pub const TEMP_T2: u32 = 0xcc;
        pub const TEMP_U: u32 = 0xd8;
        pub const TEMP_X: u32 = 0xb4;
        pub const TEMP_Y0: u32 = 0x9c;
        pub const TEMP_Y: u32 = 0x94;
    }

    /// The list slots the original zeroes at its start.
    const ZEROED: [u32; 21] = [
        slot::S,
        slot::S_PTR,
        slot::S_N,
        slot::V,
        slot::V_PTR,
        slot::V_N,
        slot::X,
        slot::X_PTR,
        slot::X_N,
        slot::Y,
        slot::Y_PTR,
        slot::Y_N,
        slot::W,
        slot::W_PTR,
        slot::W_N,
        slot::T_N2,
        slot::T_PTR,
        slot::T_N,
        slot::U_N2,
        slot::U_PTR,
        slot::U_N,
    ];

    struct Frame(u32);

    impl Frame {
        fn at(&self, o: u32) -> u32 {
            self.0.wrapping_sub(o)
        }
        fn get(&self, h: &impl Host, o: u32) -> u32 {
            h.read(self.at(o))
        }
        fn set(&self, h: &mut impl Host, o: u32, v: u32) {
            h.write(self.at(o), v);
        }
    }

    /// Whether `class` or one of its parents (`+0x40`) is `target`.
    fn derives(h: &impl Host, class: u32, target: u32) -> bool {
        let mut c = class;
        while c != 0 {
            if c == target {
                return true;
            }
            c = h.read(c.wrapping_add(0x40));
        }
        false
    }

    /// One binder appended to an 8-byte-object list: the temporary
    /// `{vtable, value}` at `temp`, fresh storage for `count + 1`
    /// elements (`count` from the slot `count_from`), the append helper
    /// on the list, the temporary destroyed. Returns the helper's result.
    #[allow(clippy::too_many_arguments)]
    fn append_object(
        h: &mut impl Host,
        f: &Frame,
        temp: u32,
        vt: u32,
        value: u32,
        count_from: u32,
        list: u32,
        helper: u32,
    ) -> u32 {
        let vt = h.rebase(vt);
        f.set(h, temp, vt);
        f.set(h, temp - 4, value);
        let n = f.get(&*h, count_from) & MASK;
        let size = n.wrapping_mul(8).wrapping_add(8);
        let storage = h.call_sym(MALLOC, Conv::Cdecl, &[size, 4]);
        let r = h.call_sym(helper, Conv::Thiscall(f.at(list)), &[storage, f.at(temp)]);
        h.call_sym(TEMP_DTOR, Conv::Thiscall(f.at(temp)), &[]);
        r
    }

    /// One binder appended to a 12-byte list grown in place (the
    /// original's inline `PArray` growth): the temporary
    /// `{vtable, class, object}` at `temp`; storage for `count + 1`
    /// elements (0: the temporary destroyed, `Err(0xd)`); the old
    /// elements copied with the vtable, the new one last; the old storage's
    /// elements destroyed (virtual slot 0 with 0) and the storage freed
    /// when owned and not the new one; the counts set; the temporary
    /// destroyed.
    #[allow(clippy::too_many_arguments)]
    fn append_inline(
        h: &mut impl Host,
        f: &Frame,
        temp: u32,
        vt: u32,
        class: u32,
        object: u32,
        (count_slot, ptr_slot, mirror_slot): (u32, u32, u32),
    ) -> Result<(), u32> {
        let vt = h.rebase(vt);
        f.set(h, temp, vt);
        f.set(h, temp - 4, class);
        f.set(h, temp - 8, object);
        let n = f.get(&*h, count_slot) & MASK;
        let size = n.wrapping_add(1).wrapping_mul(3).wrapping_mul(4);
        let new = h.call_sym(MALLOC, Conv::Cdecl, &[size, 4]);
        if new == 0 {
            h.call_sym(TEMP_DTOR, Conv::Thiscall(f.at(temp)), &[]);
            return Err(0xd);
        }
        let old = f.get(&*h, ptr_slot);
        let mut src = old.wrapping_add(8);
        let mut dst = new;
        for _ in 0..n {
            let at = dst;
            dst = dst.wrapping_add(0xc);
            if at != 0 {
                h.write(at, vt);
                let a: u32 = h.read(src.wrapping_sub(4));
                h.write(at.wrapping_add(4), a);
                let b: u32 = h.read(src);
                h.write(at.wrapping_add(8), b);
                src = src.wrapping_add(0xc);
            }
        }
        let last = new.wrapping_add(n.wrapping_mul(0xc));
        if last != 0 {
            h.write(last, vt);
            let a = f.get(&*h, temp - 4);
            h.write(last.wrapping_add(4), a);
            let b = f.get(&*h, temp - 8);
            h.write(last.wrapping_add(8), b);
        }
        let old = f.get(&*h, ptr_slot);
        if old != new && f.get(&*h, count_slot) & OWNED_BIT == 0 && old != 0 {
            for k in 0..n {
                h.call_virtual(old.wrapping_add(k.wrapping_mul(0xc)), 0, &[0]);
            }
            let old = f.get(&*h, ptr_slot);
            h.call_sym(FREE, Conv::Cdecl, &[old]);
        }
        f.set(h, ptr_slot, new);
        f.set(h, count_slot, n.wrapping_add(1));
        f.set(h, mirror_slot, n.wrapping_add(1));
        h.call_sym(TEMP_DTOR, Conv::Thiscall(f.at(temp)), &[]);
        Ok(())
    }

    /// Kinds 2 (general) and 3: every pair whose object is set and whose
    /// class derives from the record's class appended (12-byte lists).
    fn append_matching(
        h: &mut impl Host,
        f: &Frame,
        pairs: (u32, u32),
        record_class: u32,
        temp: u32,
        vt: u32,
        slots: (u32, u32, u32),
    ) -> Result<(), u32> {
        let mut p = pairs.0;
        while p < pairs.1 {
            let object: u32 = h.read(p);
            let class: u32 = h.read(p.wrapping_add(4));
            if object != 0 && class != 0 && derives(&*h, class, record_class) {
                append_inline(h, f, temp, vt, record_class, object, slots)?;
            }
            p = p.wrapping_add(8);
        }
        Ok(())
    }

    /// The binder lists' state the original keeps in registers and in
    /// slots no callee sees.
    struct Kinds {
        /// Kind 1 ran (`dl`, `[ebp-0x11]`); kind 5 ran (`dh`, `[ebp-0x21]`).
        one: bool,
        five: bool,
        /// The count of V the cleanup releases (`[ebp-0x28]`, or V's own
        /// count on an error of its append).
        v_count: Option<u32>,
    }

    /// The part of `bindTargets(this, network, set, pairs)` before the
    /// alloca: the frame at `ebp` (the original's offsets, [`FRAME`]
    /// bytes below it) set up, every record of the set (`+8` count,
    /// `+0xc` records of 0x1c bytes: class `+0`, kind `+0x10`) sorted
    /// into the lists over the pairs (`pairs`: count `+0`, `(object,
    /// class)` words from `+4`):
    /// - kind 1 (record class `0x00ca91f8`, once): pairs deriving from
    ///   `0x00ca91f8` with an object past 4 into S (the object - 4), else
    ///   pairs deriving from `0x00ca7228` with an object into V;
    /// - kind 2: record class `0x00ca7228`: pairs deriving from it whose
    ///   object has `+0xc` into T; another class: as kind 3 into T;
    /// - kind 3: pairs with an object deriving from the record's class
    ///   into U;
    /// - kind 4: a class not in W yet into W and X;
    /// - kind 5 (once): a null binder into Y, then pairs deriving from
    ///   `0x00ca5040` with an object.
    ///
    /// Returns `Ok(total)` (the binders, the alloca is `4 * total` bytes)
    /// or the original's return value after the cleanup.
    ///
    /// # Errors
    /// An append's error, or 0xd when there is no storage for an element.
    pub fn collect(h: &mut impl Host, ebp: u32, set: u32, pairs: u32) -> Result<u32, u32> {
        let f = Frame(ebp);
        let begin: u32 = h.read(pairs.wrapping_add(4));
        let n_pairs = h.read::<u32>(pairs) & MASK;
        let end = begin.wrapping_add(n_pairs.wrapping_mul(8));
        let n = h.read::<u32>(set.wrapping_add(8)) & MASK;
        let mut record: u32 = if n != 0 {
            h.read(set.wrapping_add(0xc))
        } else {
            0
        };
        for o in ZEROED {
            f.set(h, o, 0);
        }
        let mut k = Kinds {
            one: false,
            five: false,
            v_count: None,
        };
        let (k1, k2, k5) = (h.rebase(CLASS_K1), h.rebase(CLASS_K2), h.rebase(CLASS_K5));
        let t_slots = (slot::T_N, slot::T_PTR, slot::T_N2);
        let u_slots = (slot::U_N, slot::U_PTR, slot::U_N2);
        for _ in 0..n {
            let class: u32 = h.read(record);
            f.set(h, slot::W_VALUE, class);
            let kind = h.read::<u32>(record.wrapping_add(0x10));
            let r = match kind {
                1 if class == k1 && !k.one => {
                    let r = kind1(h, &f, (begin, end), k1, k2, &mut k);
                    k.one = true;
                    r
                }
                2 if class == k2 => kind2(h, &f, (begin, end), k2, t_slots),
                2 => append_matching(h, &f, (begin, end), class, slot::TEMP_T2, VT_T, t_slots),
                3 => append_matching(h, &f, (begin, end), class, slot::TEMP_U, VT_U, u_slots),
                4 => kind4(h, &f, class),
                5 if !k.five => {
                    let r = kind5(h, &f, (begin, end), k5);
                    k.five = true;
                    r
                }
                _ => Ok(()),
            };
            if let Err(code) = r {
                // kind 1's V append failing: the cleanup takes V's own count
                return Err(cleanup(h, &f, code, k.v_count));
            }
            record = record.wrapping_add(0x1c);
        }
        let total = [
            slot::V_N,
            slot::S_N,
            slot::Y_N,
            slot::X_N,
            slot::U_N,
            slot::T_N,
        ]
        .iter()
        .fold(0_u32, |t, &o| t.wrapping_add(f.get(&*h, o) & MASK));
        Ok(total)
    }

    /// Kind 1 over the pairs.
    fn kind1(
        h: &mut impl Host,
        f: &Frame,
        (begin, end): (u32, u32),
        k1: u32,
        k2: u32,
        k: &mut Kinds,
    ) -> Result<(), u32> {
        let mut p = begin;
        while p < end {
            let class: u32 = h.read(p.wrapping_add(4));
            let mut done = false;
            if derives(&*h, class, k1) {
                let object: u32 = h.read(p);
                if object != 0 && object.wrapping_sub(4) != 0 {
                    let r = append_object(
                        h,
                        f,
                        slot::TEMP_S,
                        VT_S,
                        object.wrapping_sub(4),
                        slot::S_N,
                        slot::S,
                        APPEND_S,
                    );
                    let count = f.get(&*h, slot::S);
                    f.set(h, slot::S_N, count);
                    if r != 0 {
                        return Err(r);
                    }
                    done = true;
                }
            }
            if !done && class != 0 && derives(&*h, class, k2) {
                let object: u32 = h.read(p);
                if object != 0 {
                    let r = append_object(
                        h,
                        f,
                        slot::TEMP_V,
                        VT_V,
                        object,
                        slot::V_N,
                        slot::V,
                        APPEND_V,
                    );
                    if r != 0 {
                        k.v_count = Some(f.get(&*h, slot::V));
                        return Err(r);
                    }
                    let count = f.get(&*h, slot::V);
                    f.set(h, slot::V_N, count);
                }
            }
            p = p.wrapping_add(8);
        }
        Ok(())
    }

    /// Kind 2 with the record class `0x00ca7228`.
    fn kind2(
        h: &mut impl Host,
        f: &Frame,
        (begin, end): (u32, u32),
        k2: u32,
        t_slots: (u32, u32, u32),
    ) -> Result<(), u32> {
        let mut p = begin;
        while p < end {
            let class: u32 = h.read(p.wrapping_add(4));
            let object: u32 = if derives(&*h, class, k2) {
                h.read(p)
            } else {
                0
            };
            if object != 0 && h.read::<u32>(object.wrapping_add(0xc)) != 0 {
                append_inline(h, f, slot::TEMP_T, VT_T, k2, object, t_slots)?;
            }
            p = p.wrapping_add(8);
        }
        Ok(())
    }

    /// Kind 4: the record's class into W and a binder for it into X,
    /// unless W has it.
    fn kind4(h: &mut impl Host, f: &Frame, class: u32) -> Result<(), u32> {
        let n = f.get(&*h, slot::W_N) & MASK;
        let words = f.get(&*h, slot::W_PTR);
        if (0..n).any(|i| h.read::<u32>(words.wrapping_add(4 * i)) == class) {
            return Ok(());
        }
        let storage = h.call_sym(MALLOC, Conv::Cdecl, &[n.wrapping_mul(4).wrapping_add(4), 4]);
        h.call_sym(
            APPEND_WORD,
            Conv::Thiscall(f.at(slot::W)),
            &[storage, f.at(slot::W_VALUE)],
        );
        let r = append_object(
            h,
            f,
            slot::TEMP_X,
            VT_X,
            class,
            slot::X_N,
            slot::X,
            APPEND_X,
        );
        let count = f.get(&*h, slot::X);
        f.set(h, slot::X_N, count);
        let words = f.get(&*h, slot::W);
        f.set(h, slot::W_N, words);
        if r != 0 { Err(r) } else { Ok(()) }
    }

    /// Kind 5: the null binder (its temporary alive over the pairs), then
    /// the pairs deriving from `0x00ca5040` with an object.
    fn kind5(h: &mut impl Host, f: &Frame, (begin, end): (u32, u32), k5: u32) -> Result<(), u32> {
        let vt = h.rebase(VT_Y);
        f.set(h, slot::TEMP_Y0, vt);
        f.set(h, slot::TEMP_Y0 - 4, 0);
        let n = f.get(&*h, slot::Y_N) & MASK;
        let storage = h.call_sym(MALLOC, Conv::Cdecl, &[n.wrapping_mul(8).wrapping_add(8), 4]);
        h.call_sym(
            APPEND_Y,
            Conv::Thiscall(f.at(slot::Y)),
            &[storage, f.at(slot::TEMP_Y0)],
        );
        let mut p = begin;
        while p < end {
            let class: u32 = h.read(p.wrapping_add(4));
            if derives(&*h, class, k5) {
                let object: u32 = h.read(p);
                if object != 0 {
                    let r =
                        append_object(h, f, slot::TEMP_Y, VT_Y, object, slot::Y, slot::Y, APPEND_Y);
                    if r != 0 {
                        h.call_sym(TEMP_DTOR, Conv::Thiscall(f.at(slot::TEMP_Y0)), &[]);
                        let count = f.get(&*h, slot::Y);
                        f.set(h, slot::Y_N, count);
                        return Err(r);
                    }
                }
            }
            p = p.wrapping_add(8);
        }
        h.call_sym(TEMP_DTOR, Conv::Thiscall(f.at(slot::TEMP_Y0)), &[]);
        let count = f.get(&*h, slot::Y);
        f.set(h, slot::Y_N, count);
        Ok(())
    }

    /// The part after the alloca: the binder pointers into `binders`
    /// (S, T, Y, X, U, V: each element's address), `setNetwork(network,
    /// binders, total)` on `this`, the cleanup. Returns `setNetwork`'s
    /// result.
    pub fn finish(
        h: &mut impl Host,
        ebp: u32,
        binders: u32,
        this: u32,
        network: u32,
        total: u32,
    ) -> u32 {
        let f = Frame(ebp);
        let mut out = binders;
        for (n_slot, ptr_slot, stride) in [
            (slot::S_N, slot::S_PTR, 8),
            (slot::T_N, slot::T_PTR, 0xc),
            (slot::Y_N, slot::Y_PTR, 8),
            (slot::X_N, slot::X_PTR, 8),
            (slot::U_N, slot::U_PTR, 0xc),
            (slot::V_N, slot::V_PTR, 8),
        ] {
            let n = f.get(&*h, n_slot) & MASK;
            let mut p = f.get(&*h, ptr_slot);
            for _ in 0..n {
                h.write(out, p);
                p = p.wrapping_add(stride);
                out = out.wrapping_add(4);
            }
        }
        let r = h.call_sym(
            SET_NETWORK,
            Conv::Thiscall(this),
            &[network, binders, total],
        );
        cleanup(h, &f, r, None)
    }

    /// The lists released in the original's order (W's storage when
    /// owned; V, U, X, Y, T, S: each element's virtual slot 0 with 0, then
    /// the storage, when owned and set). Returns `code`.
    fn cleanup(h: &mut impl Host, f: &Frame, code: u32, v_count: Option<u32>) -> u32 {
        if f.get(&*h, slot::W_N) & OWNED_BIT == 0 {
            let words = f.get(&*h, slot::W_PTR);
            h.call_sym(FREE, Conv::Cdecl, &[words]);
        }
        let v = v_count.unwrap_or_else(|| f.get(&*h, slot::V_N));
        for (count, ptr_slot, stride) in [
            (v, slot::V_PTR, 8),
            (f.get(&*h, slot::U_N), slot::U_PTR, 0xc),
            (f.get(&*h, slot::X_N), slot::X_PTR, 8),
            (f.get(&*h, slot::Y_N), slot::Y_PTR, 8),
            (f.get(&*h, slot::T_N), slot::T_PTR, 0xc),
            (f.get(&*h, slot::S_N), slot::S_PTR, 8),
        ] {
            if count & OWNED_BIT != 0 {
                continue;
            }
            let p = f.get(&*h, ptr_slot);
            if p == 0 {
                continue;
            }
            for k in 0..(count & MASK) {
                h.call_virtual(p.wrapping_add(k.wrapping_mul(stride)), 0, &[0]);
            }
            let p = f.get(&*h, ptr_slot);
            h.call_sym(FREE, Conv::Cdecl, &[p]);
        }
        code
    }
}
