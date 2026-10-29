//! M5 stage 1, list `static-init-a`: the compiler-generated constructors of
//! globals (`static` objects, id counters, copies of pointers from the
//! image) and the `atexit` destructors they register. The CRT runs them
//! before `WinMain`, from its initialiser tables, so the game reaches each
//! one through its address.
//!
//! Each shape is one function here over the game's memory. The constants of
//! each original are a row in [`rows`], keyed by its address: a shim passes
//! its own address and the function reads the row. The parity tests
//! (`ffx-oracle/tests/m5_static_init_a.rs`) run every row against its
//! original and compare the globals and the `atexit` calls.
//!
//! Addresses in the tables are `ffx.sym` addresses; `Mem::rebase` makes
//! them real in the process (the image may be relocated). A function the
//! row table lacks does nothing and returns 0: the shims only pass rows that
//! exist (a unit test checks the table).

pub mod rows;

use ffx_graphic::host::{Conv, Host};

/// `atexit(f)` (cdecl): 0 on success.
pub const ATEXIT: u32 = 0x0094_9207;
/// `Phyre_PRendering_PSceneRenderPassType_find_004dd860` (cdecl `(name)`):
/// the pass type in the list whose name is `name`, else 0.
pub const FIND_PASS: u32 = 0x004d_d860;
/// `Phyre_PRendering_Ctor8_005844a0` (thiscall, eight stack words).
pub const CTOR8: u32 = 0x0058_44a0;
/// `PhyreStaticEntry_ctor_0047fbe0` (thiscall, two stack words).
pub const STATIC_ENTRY: u32 = 0x0047_fbe0;
/// `Phyre_PNamespace_GetGlobalNamespace` (cdecl `()`).
pub const GET_NS: u32 = 0x0043_e2f0;
/// `Phyre_PClassDescriptor_PClassDescriptor_0043b0a0` (thiscall, six words).
pub const PCD_CTOR: u32 = 0x0043_b0a0;
/// `Phyre_PClassDescriptor_setDefaultBuffer` (thiscall, one word).
pub const SET_DEF: u32 = 0x0043_d510;
/// `Phyre_PClassDescriptor_setValidationBuffer` (thiscall, one word).
pub const SET_VAL: u32 = 0x0043_d5b0;
/// `Phyre_PClassDescriptor_setWriteMaskBuffer` (thiscall, one word).
pub const SET_WM: u32 = 0x0043_d5c0;

/// Calls the original at `sym` as a cdecl function with `args`; its `eax`.
pub fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
    let f = h.rebase(sym);
    h.call(f, Conv::Cdecl, args)
}

/// Calls the original at `sym` as a thiscall (`this` in `ecx`) with `args`;
/// its `eax`.
pub fn thiscall(h: &mut impl Host, sym: u32, this: u32, args: &[u32]) -> u32 {
    let f = h.rebase(sym);
    h.call(f, Conv::Thiscall(this), args)
}

/// The words of a row, with the image addresses (bit `i` of `addr` set)
/// rebased.
fn words_for<const N: usize>(h: &impl Host, words: [u32; N], addr: u8) -> [u32; N] {
    let mut out = words;
    for (i, w) in out.iter_mut().enumerate() {
        if ((addr >> i) & 1) == 1 {
            *w = h.rebase(*w);
        }
    }
    out
}

/// The `atexit`-only initialisers ([`rows::DTOR_ONLY`]): `atexit(dtor)`,
/// and the `atexit` result, which the original leaves in `eax`.
pub fn dtor_only(h: &mut impl Host, at: u32) -> u32 {
    let Some(row) = rows::DTOR_ONLY.iter().find(|r| r.at == at) else {
        return 0;
    };
    let dtor = h.rebase(row.dtor);
    cdecl(h, ATEXIT, &[dtor])
}

/// The counter initialisers ([`rows::NEXT_ID`]): the counter's value goes to
/// `dst`, the counter is stored back plus one, then `atexit(dtor)`. Returns
/// the `atexit` result.
pub fn next_id(h: &mut impl Host, at: u32) -> u32 {
    let Some(row) = rows::NEXT_ID.iter().find(|r| r.at == at) else {
        return 0;
    };
    let counter = h.rebase(row.counter);
    let dst = h.rebase(row.dst);
    let value: u32 = h.read(counter);
    h.write(dst, value);
    h.write(counter, value.wrapping_add(1));
    let dtor = h.rebase(row.dtor);
    cdecl(h, ATEXIT, &[dtor])
}

/// The word-clearing initialisers ([`rows::ZERO_WORD`]).
pub fn zero_word(h: &mut impl Host, at: u32) {
    let Some(row) = rows::ZERO_WORD.iter().find(|r| r.at == at) else {
        return;
    };
    let g = h.rebase(row.g);
    h.write(g, 0u32);
}

/// The word-copying initialisers ([`rows::COPY_WORD`]); returns the copied
/// word, which the original leaves in `eax`.
pub fn copy_word(h: &mut impl Host, at: u32) -> u32 {
    let Some(row) = rows::COPY_WORD.iter().find(|r| r.at == at) else {
        return 0;
    };
    let src = h.rebase(row.src);
    let dst = h.rebase(row.dst);
    let value: u32 = h.read(src);
    h.write(dst, value);
    value
}

/// The scene render pass type initialisers ([`rows::FIND_LINK`]): when
/// `FIND_PASS(name)` finds nothing, the list head goes to `dst` and the head
/// becomes the new node; then `atexit(dtor)`. Returns the `atexit` result.
pub fn find_link(h: &mut impl Host, at: u32) -> u32 {
    let Some(row) = rows::FIND_LINK.iter().find(|r| r.at == at) else {
        return 0;
    };
    let name = h.rebase(row.name);
    if cdecl(h, FIND_PASS, &[name]) == 0 {
        let head = h.rebase(row.head);
        let dst = h.rebase(row.dst);
        let node = h.rebase(row.node);
        let old: u32 = h.read(head);
        h.write(dst, old);
        h.write(head, node);
    }
    let dtor = h.rebase(row.dtor);
    cdecl(h, ATEXIT, &[dtor])
}

/// The constructor-then-destructor initialisers ([`rows::CALL_DTOR`]): the
/// callee is called as a thiscall on the row's object, with its one argument
/// when the row has one, then `atexit(dtor)`. Returns the `atexit` result.
pub fn call_dtor(h: &mut impl Host, at: u32) -> u32 {
    let Some(row) = rows::CALL_DTOR.iter().find(|r| r.at == at) else {
        return 0;
    };
    let this = h.rebase(row.this);
    let arg = row.arg.map(|a| h.rebase(a));
    if let Some(a) = arg {
        let _ = thiscall(h, row.callee, this, &[a]);
    } else {
        let _ = thiscall(h, row.callee, this, &[]);
    }
    let dtor = h.rebase(row.dtor);
    cdecl(h, ATEXIT, &[dtor])
}

/// The eight-word constructor initialisers ([`rows::CTOR8`]): `Ctor8(this,
/// words)` as a thiscall (the callee pops the eight words), then
/// `atexit(dtor)`. Returns the `atexit` result.
pub fn ctor8(h: &mut impl Host, at: u32) -> u32 {
    let Some(row) = rows::CTOR8.iter().find(|r| r.at == at) else {
        return 0;
    };
    let this = h.rebase(row.this);
    let words = words_for(h, row.words, row.addr);
    let _ = thiscall(h, CTOR8, this, &words); // call width: 8
    let dtor = h.rebase(row.dtor);
    cdecl(h, ATEXIT, &[dtor])
}

/// The two-word static entry initialisers ([`rows::STATIC_ENTRY`]):
/// `PhyreStaticEntry(this, a, b)` as a thiscall (the callee pops the two
/// words), then `atexit(dtor)`. Returns the `atexit` result.
pub fn static_entry(h: &mut impl Host, at: u32) -> u32 {
    let Some(row) = rows::STATIC_ENTRY.iter().find(|r| r.at == at) else {
        return 0;
    };
    let this = h.rebase(row.this);
    let words = words_for(h, row.words, row.addr);
    let _ = thiscall(h, STATIC_ENTRY, this, &words); // call width: 2
    let dtor = h.rebase(row.dtor);
    cdecl(h, ATEXIT, &[dtor])
}

/// The class-descriptor initialisers ([`rows::CLASS_DESC`]): the descriptor's
/// constructor on its global (six words, the namespace from `GetNamespace`),
/// the flag word `|= 2`, the vtable, the three buffer setters, the flag tail
/// and `atexit(dtor)`. Returns the `atexit` result.
pub fn class_desc(h: &mut impl Host, at: u32) -> u32 {
    let Some(row) = rows::CLASS_DESC.iter().find(|r| r.at == at) else {
        return 0;
    };
    let this = h.rebase(row.this);
    let flags = h.rebase(row.flags);
    let base = h.rebase(row.base);
    let name = h.rebase(row.name);
    let ns = cdecl(h, GET_NS, &[]);
    let _ = thiscall(
        h,
        PCD_CTOR,
        this,
        &[ns, name, row.size, row.align, base, row.c],
    );
    let f: u32 = h.read(flags);
    h.write(flags, f | 2);
    let vt1 = h.rebase(row.vt1);
    h.write(this, vt1);
    let def = h.rebase(row.def);
    let _ = thiscall(h, SET_DEF, this, &[def]);
    let val = h.rebase(row.val);
    let _ = thiscall(h, SET_VAL, this, &[val]);
    let wm = h.rebase(row.wm);
    let _ = thiscall(h, SET_WM, this, &[wm]);
    match row.tail {
        rows::FlagTail::And { m, vt2 } => {
            let f: u32 = h.read(flags);
            h.write(flags, f & m);
            let vt2 = h.rebase(vt2);
            h.write(this, vt2);
        }
        rows::FlagTail::AndOr { m1, m2, vt2 } => {
            let f: u32 = h.read(flags);
            let vt2 = h.rebase(vt2);
            h.write(this, vt2);
            h.write(flags, (f & m1) | m2);
        }
        rows::FlagTail::Keep => {}
    }
    let dtor = h.rebase(row.dtor);
    cdecl(h, ATEXIT, &[dtor])
}

#[cfg(test)]
mod tests {
    use super::rows;

    #[test]
    fn every_row_address_is_unique() {
        let mut all: Vec<u32> = rows::DTOR_ONLY.iter().map(|r| r.at).collect();
        all.extend(rows::NEXT_ID.iter().map(|r| r.at));
        all.extend(rows::ZERO_WORD.iter().map(|r| r.at));
        all.extend(rows::COPY_WORD.iter().map(|r| r.at));
        all.extend(rows::FIND_LINK.iter().map(|r| r.at));
        all.extend(rows::CALL_DTOR.iter().map(|r| r.at));
        all.extend(rows::CTOR8.iter().map(|r| r.at));
        all.extend(rows::STATIC_ENTRY.iter().map(|r| r.at));
        all.extend(rows::CLASS_DESC.iter().map(|r| r.at));
        let n = all.len();
        all.sort_unstable();
        all.dedup();
        assert_eq!(all.len(), n);
    }
}
