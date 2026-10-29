//! M5 `system-b`: the crash reporter's `StackWalker` (the open-source
//! class, linked into `FFX.exe`): its constructor and destructors, and the
//! four report printers (`OnSymInit`, `OnLoadModule`, `OnCallstackEntry`,
//! `OnDbgHelpErr`) that format a line into a 1 KB frame buffer with
//! `vsnprintf1K_00687070` and hand it to `OnOutput` (vtable `+0x14`).
//!
//! The object: `+0` the vtable, `+4` the 0x44-byte helper (`+4` the
//! dbghelp module, `+8` the process, `+0xc` a `malloc`ed string, `+0x10`
//! `SymCleanup`), `+8` the process, `+0xc` the process id, `+0x10` 0,
//! `+0x14` the `_strdup`ed symbol path, `+0x18` the options (`| 0x10` with
//! a path), `+0x1c` 1000 (the maximum name length).
//!
//! A callstack entry (`OnCallstackEntry`'s second argument): `+0` the
//! offset (64 bits), `+8` the name, `+0x408` the undecorated name,
//! `+0x808` the full undecorated name, `+0xc14` the line number, `+0xc18`
//! the file name, `+0x1020` the module name (1 KB strings).

/// `StackWalker_vftable`.
pub const VFTABLE: u32 = 0x00b4_65e0;
/// `vsnprintf1K_00687070(buffer, 0x400, format, ...)` (cdecl).
pub const VSNPRINTF_1K: u32 = 0x0068_7070;
/// `Init68_00687190(this; walker, process)` (thiscall, ret 8).
pub const INIT_HELPER: u32 = 0x0068_7190;
/// `operator_new` / `operator_delete` (cdecl).
pub const OPERATOR_NEW: u32 = 0x0063_0310;
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
/// `crt_memset_thunk` (cdecl).
pub const MEMSET: u32 = 0x0094_964c;
/// Import slots: `MSVCR110` `_strdup`, `free`, `strcpy_s`, `strncpy_s`
/// (cdecl); `KERNEL32` `FreeLibrary`, `GetVersionExA` (stdcall).
pub const STRDUP_IAT: u32 = 0x00b0_c45c;
pub const FREE_IAT: u32 = 0x00b0_c370;
pub const STRCPY_S_IAT: u32 = 0x00b0_c454;
pub const STRNCPY_S_IAT: u32 = 0x00b0_c60c;
pub const FREE_LIBRARY_IAT: u32 = 0x00b0_c120;
pub const GET_VERSION_EX_IAT: u32 = 0x00b0_c0b4;
/// The strings and formats.
pub const NO_FUNCTION_NAME: u32 = 0x00b4_6ac0;
pub const NO_FILE_NAME: u32 = 0x00b4_6ae0;
pub const NO_MODULE_NAME: u32 = 0x00b4_6afc;
pub const FMT_ENTRY: u32 = 0x00b4_6b18;
pub const FMT_ENTRY_LINE: u32 = 0x00b4_6b2c;
pub const FMT_DBGHELP_ERR: u32 = 0x00b4_6b3c;
pub const FMT_MODULE: u32 = 0x00b4_6a28;
pub const FMT_MODULE_VERSION: u32 = 0x00b4_6a68;
pub const FMT_SYM_INIT: u32 = 0x00b4_6b68;
pub const FMT_OS_VERSION: u32 = 0x00b4_6bac;

/// The report buffer: `ebp - 0x404`, 0x400 bytes.
pub const BUFFER: u32 = 0x404;
/// `OnSymInit`'s `OSVERSIONINFOEXA` at `ebp - 0x4a0` (0x9c bytes).
pub const VERSION_INFO: u32 = 0x4a0;

/// The originals over game memory.
pub mod bridge {
    use super::{
        BUFFER, FMT_DBGHELP_ERR, FMT_ENTRY, FMT_ENTRY_LINE, FMT_MODULE, FMT_MODULE_VERSION,
        FMT_OS_VERSION, FMT_SYM_INIT, FREE_IAT, FREE_LIBRARY_IAT, GET_VERSION_EX_IAT, INIT_HELPER,
        MEMSET, NO_FILE_NAME, NO_FUNCTION_NAME, NO_MODULE_NAME, OPERATOR_DELETE, OPERATOR_NEW,
        STRCPY_S_IAT, STRDUP_IAT, STRNCPY_S_IAT, VERSION_INFO, VFTABLE, VSNPRINTF_1K,
    };
    use ffx_graphic::host::{Conv, Host};

    /// Calls the import in `slot` (its `ffx.sym` address).
    fn import(h: &mut impl Host, slot: u32, conv: Conv, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, conv, args)
    }

    /// `strlen` as the originals' byte loops count.
    fn strlen(h: &impl Host, s: u32) -> u32 {
        let mut n = 0u32;
        while h.read::<u8>(s.wrapping_add(n)) != 0 {
            n = n.wrapping_add(1);
        }
        n
    }

    /// `OnOutput(buffer)`: vtable `+0x14`.
    fn output(h: &mut impl Host, this: u32, buffer: u32) {
        h.call_virtual(this, 5, &[buffer]);
    }

    /// `StackWalker::StackWalker(this; options, symPath, processId,
    /// process)` (`0x006870b0`, thiscall, ret 0x10) -> `this`.
    pub fn construct(
        h: &mut impl Host,
        this: u32,
        options: u32,
        sym_path: u32,
        process_id: u32,
        process: u32,
    ) -> u32 {
        h.write(this.wrapping_add(0x18), options);
        h.write(this, h.rebase(VFTABLE));
        h.write(this.wrapping_add(0x10), 0u32);
        h.write(this.wrapping_add(8), process);
        let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x44]);
        let helper = if p == 0 {
            0
        } else {
            let process: u32 = h.read(this.wrapping_add(8));
            h.call_sym(INIT_HELPER, Conv::Thiscall(p), &[this, process])
        };
        h.write(this.wrapping_add(4), helper);
        h.write(this.wrapping_add(0xc), process_id);
        if sym_path == 0 {
            h.write(this.wrapping_add(0x14), 0u32);
        } else {
            let s = import(h, STRDUP_IAT, Conv::Cdecl, &[sym_path]);
            let options: u32 = h.read(this.wrapping_add(0x18));
            h.write(this.wrapping_add(0x18), options | 0x10);
            h.write(this.wrapping_add(0x14), s);
        }
        h.write(this.wrapping_add(0x1c), 1000u32);
        this
    }

    /// `StackWalker::~StackWalker(this)` (`0x00687210`, thiscall): the
    /// vtable back; `free(+0x14)`, `+0x14` = 0; the helper: its
    /// `SymCleanup(process)` (stdcall, when set), `FreeLibrary(module)`
    /// (when set), `+4` and `+0` = 0, `free(+0xc)` (when set), `+0xc` = 0,
    /// `operator delete`; `+4` = 0. Callers read no result.
    pub fn destruct(h: &mut impl Host, this: u32) {
        h.write(this, h.rebase(VFTABLE));
        let path: u32 = h.read(this.wrapping_add(0x14));
        if path != 0 {
            import(h, FREE_IAT, Conv::Cdecl, &[path]);
        }
        let helper: u32 = h.read(this.wrapping_add(4));
        h.write(this.wrapping_add(0x14), 0u32);
        if helper != 0 {
            let cleanup: u32 = h.read(helper.wrapping_add(0x10));
            if cleanup != 0 {
                let process: u32 = h.read(helper.wrapping_add(8));
                h.call(cleanup, Conv::Stdcall, &[process]);
            }
            let module: u32 = h.read(helper.wrapping_add(4));
            if module != 0 {
                import(h, FREE_LIBRARY_IAT, Conv::Stdcall, &[module]);
            }
            let name: u32 = h.read(helper.wrapping_add(0xc));
            h.write(helper.wrapping_add(4), 0u32);
            h.write(helper, 0u32);
            if name != 0 {
                import(h, FREE_IAT, Conv::Cdecl, &[name]);
            }
            h.write(helper.wrapping_add(0xc), 0u32);
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[helper]);
        }
        h.write(this.wrapping_add(4), 0u32);
    }

    /// The deleting destructor `(this; flags)` (`0x006872e0`, thiscall,
    /// ret 4): [`destruct`], `operator delete(this)` when `flags & 1`;
    /// `this`.
    pub fn deleting_dtor(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        destruct(h, this);
        if flags & 1 != 0 {
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[this]);
        }
        this
    }

    /// Copies the 1 KB string at `src` over `dst` when `src` is not empty
    /// (`strcpy_s`, or `strncpy_s` and a NUL at `+0x3ff` from 0x400
    /// characters on).
    fn copy_name(h: &mut impl Host, dst: u32, src: u32) {
        if h.read::<u8>(src) == 0 {
            return;
        }
        if strlen(h, src) < 0x400 {
            import(h, STRCPY_S_IAT, Conv::Cdecl, &[dst, 0x400, src]);
        } else {
            import(h, STRNCPY_S_IAT, Conv::Cdecl, &[dst, 0x400, src, 0x400]);
            h.write(dst.wrapping_add(0x3ff), 0u8);
        }
    }

    /// `StackWalker::OnCallstackEntry(this; eType, entry)` (`0x006882a0`,
    /// thiscall, ret 8; the buffer at `ebp - 0x404`, the cookie left out).
    /// Nothing for the last entry (`eType` 2) or a zero offset. Else the
    /// entry's empty name becomes "(function-name not available)", the
    /// undecorated and the full undecorated names (when not empty) are
    /// copied over it; without a file name it gets "(filename not
    /// available)", the module (when empty) "(module-name not available)",
    /// and the line is `"%p (%s): %s: %s\n"` (offset, module, file, name);
    /// with one, `"%s (%d): %s\n"` (file, line, name). The buffer's last
    /// byte is cleared, then `OnOutput`.
    pub fn on_callstack_entry(h: &mut impl Host, ebp: u32, this: u32, kind: u32, entry: u32) {
        if kind == 2 {
            return;
        }
        let lo: u32 = h.read(entry);
        let hi: u32 = h.read(entry.wrapping_add(4));
        if lo | hi == 0 {
            return;
        }
        let name = entry.wrapping_add(8);
        if h.read::<u8>(name) == 0 {
            let s = h.rebase(NO_FUNCTION_NAME);
            import(h, STRCPY_S_IAT, Conv::Cdecl, &[name, 0x400, s]);
        }
        copy_name(h, name, entry.wrapping_add(0x408));
        copy_name(h, name, entry.wrapping_add(0x808));
        let buffer = ebp.wrapping_sub(BUFFER);
        let file = entry.wrapping_add(0xc18);
        if h.read::<u8>(file) == 0 {
            let s = h.rebase(NO_FILE_NAME);
            import(h, STRCPY_S_IAT, Conv::Cdecl, &[file, 0x400, s]);
            let module = entry.wrapping_add(0x1020);
            if h.read::<u8>(module) == 0 {
                let s = h.rebase(NO_MODULE_NAME);
                import(h, STRCPY_S_IAT, Conv::Cdecl, &[module, 0x400, s]);
            }
            let fmt = h.rebase(FMT_ENTRY);
            let offset: u32 = h.read(entry);
            h.call_sym(
                VSNPRINTF_1K,
                Conv::Cdecl,
                &[buffer, 0x400, fmt, offset, module, file, name],
            );
        } else {
            let fmt = h.rebase(FMT_ENTRY_LINE);
            let line: u32 = h.read(entry.wrapping_add(0xc14));
            h.call_sym(
                VSNPRINTF_1K,
                Conv::Cdecl,
                &[buffer, 0x400, fmt, file, line, name],
            );
        }
        h.write(buffer.wrapping_add(0x3ff), 0u8);
        output(h, this, buffer);
    }

    /// `StackWalker::OnDbgHelpErr(this; funcName, gle, addr)`
    /// (`0x00688470`, thiscall, ret 0x10: the address is 64 bits, its
    /// high word only stored in the frame): `"ERROR: %s, GetLastError: %d
    /// (Address: %p)\n"`, then `OnOutput`.
    pub fn on_dbghelp_err(
        h: &mut impl Host,
        ebp: u32,
        this: u32,
        func: u32,
        gle: u32,
        addr_lo: u32,
        _addr_hi: u32,
    ) {
        let buffer = ebp.wrapping_sub(BUFFER);
        let fmt = h.rebase(FMT_DBGHELP_ERR);
        h.call_sym(
            VSNPRINTF_1K,
            Conv::Cdecl,
            &[buffer, 0x400, fmt, func, gle, addr_lo],
        );
        output(h, this, buffer);
    }

    /// `OnLoadModule`'s arguments after `this` (ten words: the base
    /// address and the file version are 64 bits).
    #[derive(Debug, Clone, Copy)]
    pub struct Module {
        pub image: u32,
        pub module: u32,
        pub base_lo: u32,
        pub base_hi: u32,
        pub size: u32,
        pub result: u32,
        pub sym_type: u32,
        pub pdb: u32,
        pub version_lo: u32,
        pub version_hi: u32,
    }

    /// `StackWalker::OnLoadModule(this; img, mod, base, size, result,
    /// symType, pdbName, fileVersion)` (`0x006884e0`, thiscall, ret
    /// 0x28): the base's low word as `%p`; without a version the short
    /// line, else the four 16-bit parts high first; then `OnOutput`.
    pub fn on_load_module(h: &mut impl Host, ebp: u32, this: u32, m: &Module) {
        let buffer = ebp.wrapping_sub(BUFFER);
        if m.version_lo | m.version_hi == 0 {
            let fmt = h.rebase(FMT_MODULE);
            h.call_sym(
                VSNPRINTF_1K,
                Conv::Cdecl,
                &[
                    buffer, 0x400, fmt, m.image, m.module, m.base_lo, m.size, m.result, m.sym_type,
                    m.pdb,
                ],
            );
        } else {
            let fmt = h.rebase(FMT_MODULE_VERSION);
            h.call_sym_wide(
                VSNPRINTF_1K,
                &[
                    buffer,
                    0x400,
                    fmt,
                    m.image,
                    m.module,
                    m.base_lo,
                    m.size,
                    m.result,
                    m.sym_type,
                    m.pdb,
                    m.version_hi >> 16,
                    m.version_hi & 0xffff,
                    m.version_lo >> 16,
                    m.version_lo & 0xffff,
                ],
            );
        }
        output(h, this, buffer);
    }

    /// `StackWalker::OnSymInit(this; searchPath, symOptions, userName)`
    /// (`0x006885f0`, thiscall, ret 0xc; the version info at `ebp -
    /// 0x4a0`): the search-path line and `OnOutput`; the info cleared
    /// (`memset`, 0x9c bytes), its size set, and when `GetVersionExA`
    /// succeeds the OS line (major, minor, build, the service-pack text,
    /// the suite mask and the product type) and `OnOutput`.
    pub fn on_sym_init(
        h: &mut impl Host,
        ebp: u32,
        this: u32,
        search_path: u32,
        options: u32,
        user: u32,
    ) {
        let buffer = ebp.wrapping_sub(BUFFER);
        let fmt = h.rebase(FMT_SYM_INIT);
        h.call_sym(
            VSNPRINTF_1K,
            Conv::Cdecl,
            &[buffer, 0x400, fmt, search_path, options, user],
        );
        output(h, this, buffer);
        let info = ebp.wrapping_sub(VERSION_INFO);
        h.call_sym(MEMSET, Conv::Cdecl, &[info, 0, 0x9c]);
        h.write(info, 0x9cu32);
        if import(h, GET_VERSION_EX_IAT, Conv::Stdcall, &[info]) == 0 {
            return;
        }
        let product: u8 = h.read(info.wrapping_add(0x9a));
        let suite: u16 = h.read(info.wrapping_add(0x98));
        let build: u32 = h.read(info.wrapping_add(0xc));
        let minor: u32 = h.read(info.wrapping_add(8));
        let major: u32 = h.read(info.wrapping_add(4));
        let fmt = h.rebase(FMT_OS_VERSION);
        h.call_sym(
            VSNPRINTF_1K,
            Conv::Cdecl,
            &[
                buffer,
                0x400,
                fmt,
                major,
                minor,
                build,
                info.wrapping_add(0x14),
                u32::from(suite),
                u32::from(product),
            ],
        );
        output(h, this, buffer);
    }
}
