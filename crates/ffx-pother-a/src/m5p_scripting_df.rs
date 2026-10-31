//! List `phyre-scripting`'s two run-time frames (M5 stage 1, round
//! "design-frames"): the script loader's file name (`0x005d57b0`, the
//! name with `.lua` added in an `_alloca_probe_16` buffer, handed to the
//! file-loader callback) and the module name of a script path
//! (`0x005d6650`, the part between the last `/` and `.lua` in such a
//! buffer, copied into a `PString`).
//!
//! Each original sizes its buffer from the string and fills it with CRT
//! calls, so each bridge is split at the alloca (docs/porting.md,
//! "Run-time frames and varargs"): the part before it returns the size the
//! original asks `_alloca_probe_16` for, the shim carves the frame, and
//! the part after takes its address.

use ffx_graphic::host::{Conv, Host};

/// `".lua"` in `.rdata`.
pub const LUA_EXT: u32 = 0x00b3_9138;
/// `strstr`'s import slot (cdecl).
pub const STRSTR_SLOT: u32 = 0x00b0_c378;
/// `strncpy`'s import slot (cdecl).
pub const STRNCPY_SLOT: u32 = 0x00b0_c368;
/// `strncat`'s import slot (cdecl).
pub const STRNCAT_SLOT: u32 = 0x00b0_c3d0;
/// `crt_memset_thunk` (cdecl, `jmp [memset]`).
pub const MEMSET: u32 = 0x0094_964c;
/// `Phyre_PString_Assign(this; text)` (thiscall, ret 4).
pub const PSTRING_ASSIGN: u32 = 0x0044_5cf0;
/// The script file loader `(name, user)` the game registers through
/// `0x005d66d0` (a function pointer; the original calls it with two words
/// and reads its `eax`).
pub const FILE_LOADER: u32 = 0x00cb_e6bc;
/// The loader's second argument (stored beside it by `0x005d66d0`).
pub const FILE_LOADER_USER: u32 = 0x00cb_e6c0;

/// The originals over game memory.
pub mod bridge {
    use super::{
        Conv, FILE_LOADER, FILE_LOADER_USER, Host, LUA_EXT, MEMSET, PSTRING_ASSIGN, STRNCAT_SLOT,
        STRNCPY_SLOT, STRSTR_SLOT,
    };

    /// `strlen` as the originals inline it (`mov al, [esi]; inc esi; test
    /// al, al`).
    fn str_len(h: &impl Host, s: u32) -> u32 {
        let mut n = 0u32;
        while h.read::<u8>(s.wrapping_add(n)) != 0 {
            n = n.wrapping_add(1);
        }
        n
    }

    /// The load file name of `0x005d57b0(name)`, before its alloca:
    /// `strstr(name, ".lua")`; `None` when `name` has it (no buffer), else
    /// `(len, bytes)`: the name's length and the `len + 5` bytes the
    /// original's buffer takes.
    pub fn load_file_size(h: &mut impl Host, name: u32) -> Option<(u32, u32)> {
        let ext = h.rebase(LUA_EXT);
        let strstr: u32 = h.read(h.rebase(STRSTR_SLOT));
        if h.call(strstr, Conv::Cdecl, &[name, ext]) != 0 {
            return None;
        }
        let len = str_len(h, name);
        Some((len, len.wrapping_add(5)))
    }

    /// The rest of `0x005d57b0(name)` (cdecl) -> `eax`: with `buf` (the
    /// `(len, bytes)` of [`load_file_size`]), `memset(buf, 0, bytes)`,
    /// `strncpy(buf, name, len)`, `strncpy(buf + len, ".lua", 4)`; then
    /// when the file loader is set, its `eax` for `(buf or name,
    /// user)`, else 0.
    pub fn load_file(h: &mut impl Host, name: u32, buf: Option<(u32, u32, u32)>) -> u32 {
        let file = match buf {
            None => name,
            Some((at, len, bytes)) => {
                let strncpy: u32 = h.read(h.rebase(STRNCPY_SLOT));
                let ext = h.rebase(LUA_EXT);
                h.call_sym(MEMSET, Conv::Cdecl, &[at, 0, bytes]);
                h.call(strncpy, Conv::Cdecl, &[at, name, len]);
                h.call(strncpy, Conv::Cdecl, &[at.wrapping_add(len), ext, 4]);
                at
            }
        };
        let loader: u32 = h.read(h.rebase(FILE_LOADER));
        if loader == 0 {
            return 0;
        }
        let user: u32 = h.read(h.rebase(FILE_LOADER_USER));
        h.call(loader, Conv::Cdecl, &[file, user])
    }

    /// The module name of `0x005d6650(string, path)`, before its alloca:
    /// `strstr(path, ".lua")`; `None` without it, else `(start, len,
    /// bytes)`: the name starts after the last `/` before the extension
    /// (at `path` itself when the scan reaches it), runs `len` bytes to the
    /// extension, and the buffer takes `len + 1`.
    pub fn module_name_size(h: &mut impl Host, path: u32) -> Option<(u32, u32, u32)> {
        let ext = h.rebase(LUA_EXT);
        let strstr: u32 = h.read(h.rebase(STRSTR_SLOT));
        let end = h.call(strstr, Conv::Cdecl, &[path, ext]);
        if end == 0 {
            return None;
        }
        let mut start = end;
        while start > path && h.read::<u8>(start) != b'/' {
            start -= 1;
        }
        if start != path {
            start = start.wrapping_add(1);
        }
        let len = end.wrapping_sub(start);
        Some((start, len, len.wrapping_add(1)))
    }

    /// The rest of `0x005d6650(string, path)` (cdecl) -> `eax`: with `buf`
    /// (the `(start, len)` of [`module_name_size`]), `buf[0] = 0` and
    /// `strncat(buf, start, len)`; then `Phyre_PString_Assign(string;
    /// buf or path)`, its `eax`.
    pub fn module_name(
        h: &mut impl Host,
        string: u32,
        path: u32,
        buf: Option<(u32, u32, u32)>,
    ) -> u32 {
        let text = match buf {
            None => path,
            Some((at, start, len)) => {
                let strncat: u32 = h.read(h.rebase(STRNCAT_SLOT));
                h.write(at, 0u8);
                h.call(strncat, Conv::Cdecl, &[at, start, len]);
                at
            }
        };
        h.call_sym(PSTRING_ASSIGN, Conv::Thiscall(string), &[text])
    }
}
