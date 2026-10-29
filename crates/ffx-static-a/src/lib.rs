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
        let n = all.len();
        all.sort_unstable();
        all.dedup();
        assert_eq!(all.len(), n);
    }
}
