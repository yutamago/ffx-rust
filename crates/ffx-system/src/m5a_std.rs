//! M5 `system-a`: the C++ library pieces `FFX.exe` compiled into its own
//! code (the remaster's `MSVCP110` templates): the `std::error_category`
//! objects of `<system_error>` and `<ios>`, the locale facet release and
//! the string streams' destructors that exception unwinding runs.
//!
//! The DLL imports they call (`std::_Syserror_map`, the stream base
//! destructors) stay calls through `FFX.exe`'s import table; game code
//! they call (`operator delete`, the string assign, the string buffer's
//! destructor) stays a call to the original (`Host::call_sym`).
//!
//! Each bridge's doc names its original and the calling convention read
//! from the disassembly; `eax` is returned wherever the original defines
//! it, also where Ghidra says `void`.

/// `operator delete(p)` (`0x00630390`, cdecl).
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
/// `std::string::assign(this; s, n)` (`0x00404270`, thiscall, `ret 8`).
pub const STRING_ASSIGN: u32 = 0x0040_4270;
/// `std::basic_stringbuf<char>`'s destructor body (`0x00403010`, thiscall,
/// no stack words; ends in a tail jump to `MSVCP110`).
pub const STRINGBUF_DTOR: u32 = 0x0040_3010;
/// `std::basic_stringstream<char>`'s deleting destructor through its
/// `basic_ios` (`0x004038c0`, thiscall, `ret 4`).
pub const STRINGSTREAM_DELETING: u32 = 0x0040_38c0;

/// `std::error_category`'s vtable (`0x00b0d9cc`), which every category's
/// deleting destructor stores before it frees the object.
pub const ERROR_CATEGORY_VTABLE: u32 = 0x00b0_d9cc;
/// `"unknown error"` (`0x00b7c7a4`).
pub const UNKNOWN_ERROR: u32 = 0x00b7_c7a4;
/// `"iostream stream error"` (`0x00b7c7c0`, 21 characters).
pub const IOSTREAM_ERROR: u32 = 0x00b7_c7c0;
/// `std::_Generic_error_category`'s object pointer (`0x00c8b43c`).
pub const GENERIC_CATEGORY: u32 = 0x00c8_b43c;
/// `std::_System_error_category`'s object pointer (`0x00c8b438`).
pub const SYSTEM_CATEGORY: u32 = 0x00c8_b438;

/// Import slot of `MSVCP110!std::_Syserror_map(int)` (cdecl).
pub const SYSERROR_MAP_IAT: u32 = 0x00b0_c258;
/// Import slot of `MSVCP110!std::_Winerror_map(int)` (cdecl).
pub const WINERROR_MAP_IAT: u32 = 0x00b0_c254;
/// Import slot of `MSVCP110!std::basic_iostream<char>::~basic_iostream`.
pub const IOSTREAM_DTOR_IAT: u32 = 0x00b0_c290;
/// Import slot of `MSVCP110!std::basic_istream<char>::~basic_istream`.
pub const ISTREAM_DTOR_IAT: u32 = 0x00b0_c298;
/// Import slot of `MSVCP110!std::basic_ostream<char>::~basic_ostream`.
pub const OSTREAM_DTOR_IAT: u32 = 0x00b0_c2c0;
/// Import slot of `MSVCP110!std::basic_ios<char>::~basic_ios`.
pub const IOS_DTOR_IAT: u32 = 0x00b0_c2f8;
/// Import slot `0x00b0c294` (`MSVCP110`, a `basic_istream<char>` method
/// taking only `this`), where `0x00405a77`'s vtordisp thunk jumps.
pub const ISTREAM_VTORDISP_IAT: u32 = 0x00b0_c294;

/// A string stream's destructor that unwinding runs (three copies, each
/// `ecx` = the stream, no stack words): the stream's `basic_ios` sits at
/// `ios` and gets `vtable`, the vbase word before it the distance, then
/// the string buffer at `buf` and the stream base at `base` (through
/// `base_dtor`'s import slot) are destroyed, and the tail jump destroys
/// the `basic_ios` (`IOS_DTOR_IAT`).
#[derive(Debug, Clone, Copy)]
pub struct StreamDtor {
    pub ios: u32,
    pub vtable: u32,
    pub buf: u32,
    pub base: u32,
    pub base_dtor: u32,
}

/// `0x00401840`: a `std::stringstream` (`basic_iostream`).
pub const STRINGSTREAM_DTOR: StreamDtor = StreamDtor {
    ios: 0x68,
    vtable: 0x00b7_cb6c,
    buf: 0x18,
    base: 0x20,
    base_dtor: IOSTREAM_DTOR_IAT,
};
/// `0x004022b0`: a `std::istringstream`.
pub const ISTRINGSTREAM_DTOR: StreamDtor = StreamDtor {
    ios: 0x60,
    vtable: 0x00b8_f790,
    buf: 0x10,
    base: 0x18,
    base_dtor: ISTREAM_DTOR_IAT,
};
/// `0x004029c0`: a `std::ostringstream`.
pub const OSTRINGSTREAM_DTOR: StreamDtor = StreamDtor {
    ios: 0x50,
    vtable: 0x00b7_cb1c,
    buf: 0x04,
    base: 0x08,
    base_dtor: OSTREAM_DTOR_IAT,
};

/// The originals over game memory and the original callees.
pub mod bridge {
    use super::{
        ERROR_CATEGORY_VTABLE, GENERIC_CATEGORY, IOS_DTOR_IAT, IOSTREAM_ERROR,
        ISTREAM_VTORDISP_IAT, OPERATOR_DELETE, STRING_ASSIGN, STRINGBUF_DTOR,
        STRINGSTREAM_DELETING, SYSERROR_MAP_IAT, SYSTEM_CATEGORY, StreamDtor, UNKNOWN_ERROR,
        WINERROR_MAP_IAT,
    };
    use ffx_graphic::host::{Conv, Host};

    /// Calls the import in `slot` (its `ffx.sym` address).
    fn import(h: &mut impl Host, slot: u32, conv: Conv, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, conv, args)
    }

    /// `0x004011d0` (`ecx` = the holder, no stack words; unwinding): the
    /// facet at `*holder`, when set, is released (`vtable[2]`, which
    /// returns the facet to delete or 0) and deleted (`vtable[0](1)`).
    pub fn release_facet(h: &mut impl Host, holder: u32) {
        let facet: u32 = h.read(holder);
        if facet == 0 {
            return;
        }
        let dead = h.call_virtual(facet, 2, &[]);
        if dead != 0 {
            h.call_virtual(dead, 0, &[1]);
        }
    }

    /// An error category's deleting destructor (`0x00401260`,
    /// `0x00401430`, `0x004014e0`, `0x00401600`; thiscall, `ret 4`):
    /// `*this` = `std::error_category`'s vtable, `operator delete(this)`
    /// when bit 0 of `flags` is set. Returns `this`.
    pub fn category_deleting_dtor(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        let vtable = h.rebase(ERROR_CATEGORY_VTABLE);
        h.write(this, vtable);
        if flags & 1 != 0 {
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[this]);
        }
        this
    }

    /// `error_category::equivalent(this; code, cond)` (`0x00401330`,
    /// thiscall, `ret 8`): `default_error_condition(&local, code)`
    /// (`vtable[3]`, thiscall with the result's address at `ebp - 8`)
    /// compared with the condition `{value, category}` at `cond`.
    /// Returns `eax` as the original leaves it: `al` the answer, the
    /// upper bytes those of the returned pointer (category mismatch) or
    /// of the value it read.
    pub fn equivalent_code(h: &mut impl Host, ebp: u32, this: u32, code: u32, cond: u32) -> u32 {
        let local = ebp.wrapping_sub(8);
        let got = h.call_virtual(this, 3, &[local, code]);
        let category: u32 = h.read(got.wrapping_add(4));
        let want: u32 = h.read(cond.wrapping_add(4));
        if category != want {
            return got & !0xff;
        }
        let value: u32 = h.read(got);
        let want: u32 = h.read(cond);
        (value & !0xff) | u32::from(value == want)
    }

    /// `error_category::equivalent(this; &code, cond)` (`0x00401370`,
    /// thiscall, `ret 8`): whether the error code `{value, category}` at
    /// `code` is `{cond, this}`. Returns `eax` as the original leaves it
    /// (`al` the answer; the upper bytes of `code`, or of the value read).
    pub fn equivalent_cond(h: &mut impl Host, this: u32, code: u32, cond: u32) -> u32 {
        let category: u32 = h.read(code.wrapping_add(4));
        if this != category {
            return code & !0xff;
        }
        let value: u32 = h.read(code);
        (value & !0xff) | u32::from(value == cond)
    }

    /// `strlen` as the originals inline it.
    fn strlen(h: &impl Host, s: u32) -> u32 {
        let mut n = 0u32;
        while h.read::<u8>(s.wrapping_add(n)) != 0 {
            n = n.wrapping_add(1);
        }
        n
    }

    /// An empty `std::string` at `out` (capacity 15, size 0) assigned
    /// `text`'s `len` bytes.
    fn set_string(h: &mut impl Host, out: u32, text: u32, len: u32) {
        h.write(out.wrapping_add(0x14), 0xfu32);
        h.write(out.wrapping_add(0x10), 0u32);
        h.write(out, 0u8);
        h.call_sym(STRING_ASSIGN, Conv::Thiscall(out), &[text, len]);
    }

    /// The message of `code` from the map in import slot `map`, or
    /// "unknown error", into the string at `out`. Returns `out`.
    fn message(h: &mut impl Host, map: u32, out: u32, code: u32) -> u32 {
        let s = import(h, map, Conv::Cdecl, &[code]);
        let text = if s == 0 { h.rebase(UNKNOWN_ERROR) } else { s };
        let len = strlen(h, text);
        set_string(h, out, text, len);
        out
    }

    /// `_Generic_error_category::message(out, code)` (`0x004013b0`;
    /// called as a thiscall, but `ecx` is unused: two stack words, `ret
    /// 8`): `std::_Syserror_map(code)`'s text. Returns `out`.
    pub fn generic_message(h: &mut impl Host, out: u32, code: u32) -> u32 {
        message(h, SYSERROR_MAP_IAT, out, code)
    }

    /// `_System_error_category::message(out, code)` (`0x00401540`, as
    /// [`generic_message`]): `std::_Winerror_map(code)`'s text.
    pub fn system_message(h: &mut impl Host, out: u32, code: u32) -> u32 {
        message(h, WINERROR_MAP_IAT, out, code)
    }

    /// `_Iostream_error_category::message(out, code)` (`0x00401490`, as
    /// [`generic_message`]): "iostream stream error" for 1, else the
    /// generic message.
    pub fn iostream_message(h: &mut impl Host, out: u32, code: u32) -> u32 {
        if code == 1 {
            let text = h.rebase(IOSTREAM_ERROR);
            set_string(h, out, text, 0x15);
            return out;
        }
        generic_message(h, out, code);
        out
    }

    /// `_System_error_category::default_error_condition(out, code)`
    /// (`0x004015c0`; `ecx` unused, `ret 8`): `{code, generic}` when
    /// `std::_Syserror_map` knows the code, else `{code, system}`.
    /// Returns `out`.
    pub fn default_condition(h: &mut impl Host, out: u32, code: u32) -> u32 {
        let known = import(h, SYSERROR_MAP_IAT, Conv::Cdecl, &[code]) != 0;
        h.write(out, code);
        let category = h.rebase(if known {
            GENERIC_CATEGORY
        } else {
            SYSTEM_CATEGORY
        });
        h.write(out.wrapping_add(4), category);
        out
    }

    /// `0x00401660` (`ecx` = the holder of an output stream, no stack
    /// words; unwinding, the stream sentry's end): the stream's buffer
    /// (`basic_ios` at the vbase distance, `rdbuf` at `+0x38`), when set,
    /// gets `vtable[2]` (`_Unlock`), which the original tail-jumps to.
    pub fn sentry_unlock(h: &mut impl Host, holder: u32) {
        let stream: u32 = h.read(holder);
        let vbtable: u32 = h.read(stream);
        let ios: u32 = h.read(vbtable.wrapping_add(4));
        let buf: u32 = h.read(ios.wrapping_add(stream).wrapping_add(0x38));
        if buf != 0 {
            h.call_virtual(buf, 2, &[]);
        }
    }

    /// A string stream's destructor (`0x00401840`, `0x004022b0`,
    /// `0x004029c0`; [`StreamDtor`]). Returns the tail-jumped
    /// `~basic_ios`'s `eax`.
    pub fn stream_dtor(h: &mut impl Host, this: u32, d: StreamDtor) -> u32 {
        let ios = this.wrapping_add(d.ios);
        let vbtable: u32 = h.read(this);
        let dist: u32 = h.read(vbtable.wrapping_add(4));
        let ios_vtable = h.rebase(d.vtable);
        h.write(dist.wrapping_add(this), ios_vtable);
        // read again: the store may have landed on the vbase pointer
        let vbtable: u32 = h.read(this);
        let dist: u32 = h.read(vbtable.wrapping_add(4));
        h.write(
            dist.wrapping_add(this).wrapping_sub(4),
            dist.wrapping_sub(d.ios),
        );
        h.call_sym(
            STRINGBUF_DTOR,
            Conv::Thiscall(this.wrapping_add(d.buf)),
            &[],
        );
        import(
            h,
            d.base_dtor,
            Conv::Thiscall(this.wrapping_add(d.base)),
            &[],
        );
        import(h, IOS_DTOR_IAT, Conv::Thiscall(ios), &[])
    }

    /// `0x00405a77` (a vtordisp thunk: `ecx` = the object, no stack
    /// words): `this - [this - 4] - 0x50` to the import in
    /// [`ISTREAM_VTORDISP_IAT`] (a tail jump). Returns its `eax`.
    pub fn vtordisp_istream(h: &mut impl Host, this: u32) -> u32 {
        let disp: u32 = h.read(this.wrapping_sub(4));
        let obj = this.wrapping_sub(disp).wrapping_sub(0x50);
        import(h, ISTREAM_VTORDISP_IAT, Conv::Thiscall(obj), &[])
    }

    /// `0x00405a83` (a vtordisp thunk, thiscall `(this; flags)`): `this -
    /// [this - 4]` to the string stream's deleting destructor
    /// `0x004038c0` (a tail jump, `ret 4`). Returns its `eax`.
    pub fn vtordisp_stringstream_delete(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        let disp: u32 = h.read(this.wrapping_sub(4));
        let obj = this.wrapping_sub(disp);
        h.call_sym(STRINGSTREAM_DELETING, Conv::Thiscall(obj), &[flags])
    }
}
