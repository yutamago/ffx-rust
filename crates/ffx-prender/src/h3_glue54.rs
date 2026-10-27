//! The render-side glue, fifty-fourth batch (M4 port round 16, list
//! `h3-glue-render`): VFX texture lists loaded into the scene's texture
//! table. The exception frame and cookie are left out, as in
//! [`crate::glue10`].

/// The MSVC string's `assign(this; text, n)` (`ret 8`), `append(this;
/// text, n)` (`ret 8`) and `find(this; text, pos, n)` (`ret 0xc`, -1
/// none).
pub const STRING_ASSIGN: u32 = 0x0040_4270;
pub const STRING_APPEND: u32 = 0x0065_76d0;
pub const STRING_FIND: u32 = 0x0040_3bd0;
/// `PhyreScene_findNamedTexture_00660420(scene; name)` (`ret 4`): the
/// text of a list already read, or 0.
pub const FIND_NAMED: u32 = 0x0066_0420;
/// `fiosIsFileExist(name)`, `fiosOpen(name, mode)`, `fiosFileSize(stream)`,
/// `fiosRead(stream, buf, n)` and `fiosClose(stream)` (all cdecl).
pub const FIOS_IS_FILE_EXIST: u32 = 0x0067_96f0;
pub const FIOS_OPEN: u32 = 0x0067_9730;
pub const FIOS_FILE_SIZE: u32 = 0x0067_9570;
pub const FIOS_READ: u32 = 0x0067_9800;
pub const FIOS_CLOSE: u32 = 0x0067_9510;
/// `operator new[]`, `operator delete[]`, `operator delete` (cdecl) and
/// the user heap's `free` (cdecl).
pub const OPERATOR_NEW_ARRAY: u32 = 0x0063_04c0;
pub const OPERATOR_DELETE_ARRAY: u32 = 0x0063_0540;
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
pub const USER_FREE: u32 = 0x0068_7010;
/// `Phyre_PString_PString(this; text)` (`ret 4`; one tagged word: a heap
/// pointer when its bit 0 is clear) and `Phyre_PhyreFree(p)` (cdecl).
pub const PSTRING: u32 = 0x0042_f090;
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// The named-texture tree's `insertPair(tree; &name, &text)` (`ret 8`) at
/// scene `+0x10d54`.
pub const INSERT_PAIR: u32 = 0x0065_fdd0;
pub const NAMED_TREE: u32 = 0x10d54;
/// The arrays' `reserve(array; n)` (`ret 4`): word arrays (the frame's
/// line list, the scene's names and paths), the scene's flags, the
/// scene's 20-byte layouts; the line list's `push_back(array; &p)`
/// (`ret 4`).
pub const RESERVE_WORDS: u32 = 0x0066_7e80;
pub const RESERVE_FLAGS: u32 = 0x0066_7e00;
pub const RESERVE_LAYOUTS: u32 = 0x0066_8200;
pub const PUSH_BACK: u32 = 0x0065_60c0;
/// `AsyncLoadManager_sGetInstance()` (cdecl),
/// `createChildrenRequestList(manager; n)` (`ret 4`) and
/// `loadCluster(manager, list, path, callback, n, n 64-bit words)`
/// (cdecl, variadic: thirteen words here).
pub const ASYNC_INSTANCE: u32 = 0x0069_1b40;
pub const CHILDREN_REQUEST_LIST: u32 = 0x0069_0f70;
pub const LOAD_CLUSTER: u32 = 0x0069_1660;
/// `PhyreScene_onVfxTextureLoaded_00666880`, the asynchronous load's
/// callback.
pub const ON_LOADED: u32 = 0x0066_6880;
/// `ClusterManager_GetInstance()` (cdecl), `loadPCluster(manager; path)`
/// (`ret 4`), `Phyre_PFramework_PApplication_GetApplication()` (cdecl; the
/// result is not used) and `PApplication_FixupClusters(clusters, n)`
/// (cdecl, 0 on success).
pub const CLUSTER_MANAGER: u32 = 0x0069_a1a0;
pub const LOAD_PCLUSTER: u32 = 0x0069_b940;
pub const GET_APPLICATION: u32 = 0x0062_3b90;
pub const FIXUP_CLUSTERS: u32 = 0x0062_3580;
/// `PhyreScene_setVFXTextureLayout_00656ae0(id, kind, name)` (stdcall,
/// `ret 0xc`).
pub const SET_LAYOUT: u32 = 0x0065_6ae0;
/// `PStreamReaderFile`'s constructor `(this; path)` (`ret 4`), `isOpen`,
/// `close` and destructor (thiscall, no arguments).
pub const READER_CTOR: u32 = 0x0057_66d0;
pub const READER_IS_OPEN: u32 = 0x0057_6980;
pub const READER_CLOSE: u32 = 0x0057_6930;
pub const READER_DTOR: u32 = 0x0057_67c0;
/// The MSVC string's destructor (`std_string_dtor`, thiscall), the
/// in-image `memset` thunk (cdecl) and `PArray_pushBack20_006562f0(array;
/// &element)` (`ret 4`, a 20-byte element).
pub const STRING_TIDY: u32 = 0x0040_2da0;
pub const MEMSET: u32 = 0x0094_964c;
pub const PUSH_BACK20: u32 = 0x0065_62f0;
/// `report_rangecheckfailure()` (cdecl, never returns).
pub const RANGECHECK_FAILURE: u32 = 0x0094_9b02;
/// The import slots of `strstr`, `strncpy` and `sprintf` (cdecl).
pub const STRSTR: u32 = 0x00b0_c378;
pub const STRNCPY: u32 = 0x00b0_c368;
pub const SPRINTF: u32 = 0x00b0_c3dc;
/// The image's `""`, `"\n"`, `"D3D11/"` and `"%s%s%s"`.
pub const EMPTY: u32 = 0x00b8_fbeb;
pub const NEWLINE: u32 = 0x00b7_cae0;
pub const D3D11: u32 = 0x00b4_2a34;
pub const PATH_FORMAT: u32 = 0x00b4_2a3c;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use crate::h3_glue49::bridge::{clear_string, text};
    use ffx_graphic::host::{Conv, Host};

    /// The C string's length.
    pub(crate) fn strlen(h: &impl Host, s: u32) -> u32 {
        let mut n = 0u32;
        while h.read::<u8>(s.wrapping_add(n)) != 0 {
            n = n.wrapping_add(1);
        }
        n
    }

    /// The C string at `from` copied to `to`, its 0 included.
    pub(crate) fn strcpy(h: &mut impl Host, to: u32, from: u32) {
        let mut k = 0u32;
        loop {
            let c: u8 = h.read(from.wrapping_add(k));
            h.write(to.wrapping_add(k), c);
            if c == 0 {
                break;
            }
            k = k.wrapping_add(1);
        }
    }

    /// A cdecl call through the import slot `slot`.
    pub(crate) fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let at = h.rebase(slot);
        let f: u32 = h.read(at);
        h.call(f, Conv::Cdecl, args)
    }

    /// The string's heap buffer freed (capacity 16 or more).
    pub(crate) fn free_buffer(h: &mut impl Host, s: u32) {
        if h.read::<u32>(s + 0x14) >= 0x10 {
            let p: u32 = h.read(s);
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[p]);
        }
    }

    /// A `PString` word freed when it holds a heap pointer (bit 0 clear).
    pub(crate) fn free_pstring(h: &mut impl Host, at: u32) {
        let v: u32 = h.read(at);
        if v & 1 == 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[v]);
        }
    }

    /// The new size of a grown array of `count` (`count + 2 + count / 8`,
    /// the division rounding to 0).
    pub(crate) fn grown(count: u32) -> u32 {
        count
            .wrapping_add(2)
            .wrapping_add((count.cast_signed() / 8).cast_unsigned())
    }

    /// One element appended to the array at `arr` (count, capacity, data):
    /// the count raised, the array grown with `reserve` when the count
    /// passes the capacity (signed), the new last element's address (0
    /// when the data is null and it is the first) returned.
    pub(crate) fn push(h: &mut impl Host, arr: u32, reserve: u32, size: u32) -> u32 {
        let count = h.read::<u32>(arr).wrapping_add(1);
        h.write(arr, count);
        let cap: u32 = h.read(arr + 4);
        if count.cast_signed() > cap.cast_signed() {
            h.call_sym(reserve, Conv::Thiscall(arr), &[grown(count)]);
        }
        let count: u32 = h.read(arr);
        let data: u32 = h.read(arr + 8);
        data.wrapping_add(count.wrapping_mul(size))
            .wrapping_sub(size)
    }

    /// Where a loader keeps its things below `ebp` (offsets): the list's
    /// text as a string, the scene, the line list (count, capacity,
    /// data), the tail line's pointer (later the path's copy), the
    /// request list, the line index, the line's copy, the cluster, the
    /// reader object. Both share the directory (`- 0x310`), the path
    /// (`- 0x210`) and the line (`- 0x110`).
    struct Frame {
        ebp: u32,
        text: u32,
        scene: u32,
        lines: u32,
        tail: u32,
        list: u32,
        index: u32,
        line_copy: u32,
        cluster: u32,
        reader: u32,
        /// `loadMagicFileVFXTexture`: the `_` directory's fallback, the
        /// layout pushed through `PArray_pushBack20`, the string freed
        /// with its destructor when a file is missing.
        magic: bool,
    }

    const DIR: u32 = 0x310;
    const PATH: u32 = 0x210;
    const LINE: u32 = 0x110;
    /// `loadMagicFileVFXTexture`'s fallback directory (0x100 bytes), its
    /// second reader and the layout it pushes (20 bytes inside the second
    /// reader's room).
    const ALT_DIR: u32 = 0x410;
    const READER2: u32 = 0x884;
    const LAYOUT: u32 = 0x87c;
    /// `loadMagicFileVFXTexture`'s read buffer (0x400 bytes).
    const CHUNK: u32 = 0x810;

    impl Frame {
        fn at(&self, off: u32) -> u32 {
            self.ebp - off
        }
    }

    /// The early ends: the line list's data freed (user heap), the text's
    /// buffer freed (with the string's destructor when `tidy`); `code`.
    fn finish(h: &mut impl Host, f: &Frame, code: u32, tidy: bool) -> u32 {
        let data: u32 = h.read(f.at(f.lines) + 8);
        if data != 0 {
            h.call_sym(USER_FREE, Conv::Cdecl, &[data]);
        }
        if tidy {
            h.call_sym(STRING_TIDY, Conv::Thiscall(f.at(f.text)), &[]);
        } else {
            free_buffer(h, f.at(f.text));
        }
        code
    }

    /// The name's directory into `ebp - 0x310`: the name copied, cut after
    /// its last `/` (searched from its end without a lower bound, as in
    /// the original). The cut's index, or `None` after
    /// `report_rangecheckfailure` (past 0xff).
    fn directory(h: &mut impl Host, ebp: u32, name: u32) -> Option<u32> {
        let dir = ebp - DIR;
        let mut end = strlen(h, name);
        strcpy(h, dir, name);
        while h.read::<u8>(dir.wrapping_add(end)) != b'/' {
            end = end.wrapping_sub(1);
        }
        end = end.wrapping_add(1);
        if end >= 0x100 {
            h.call_sym(RANGECHECK_FAILURE, Conv::Cdecl, &[]);
            return None;
        }
        h.write(dir + end, 0u8);
        Some(end)
    }

    /// The lines loaded: the request list (with `async`), each line, the
    /// list's data and the text freed; 0 or the first failure's code.
    /// The magic loader marks the scene's `+0x24` byte after the last.
    fn load_lines(h: &mut impl Host, f: &Frame, kind: u32, async_load: u32) -> u32 {
        let count = split_lines(h, f);
        h.write(f.at(f.list), 0u32);
        if async_load & 0xff != 0 {
            let manager = h.call_sym(ASYNC_INSTANCE, Conv::Cdecl, &[]);
            let list = h.call_sym(CHILDREN_REQUEST_LIST, Conv::Thiscall(manager), &[count]);
            h.write(f.at(f.list), list);
        }
        let mut i = 0u32;
        h.write(f.at(f.index), i);
        while i < count {
            match load_line(h, f, kind, async_load, i, count) {
                Line::Skipped | Line::Loaded => {}
                Line::Missing => return finish(h, f, 5, f.magic),
                Line::Failed(code) => return finish(h, f, code, false),
                Line::Aborted => return 0,
            }
            i = h.read::<u32>(f.at(f.index)).wrapping_add(1);
            h.write(f.at(f.index), i);
        }
        if f.magic {
            let this: u32 = h.read(f.at(f.scene));
            h.write(this.wrapping_add(0x24), 1u8);
        }
        if count.cast_signed() < 0 && h.read::<u32>(f.at(f.lines) + 4).cast_signed() < 0 {
            h.call_sym(RESERVE_WORDS, Conv::Thiscall(f.at(f.lines)), &[0]);
        }
        finish(h, f, 0, false)
    }

    /// `PhyreScene_loadVFXTexture(this; kind, name, async)` (thiscall,
    /// `ret 0xc`; frame from `ebp - 0x37c`) -> 0, 5 when the list or one
    /// of its files is missing, or the cluster fix-up's error.
    ///
    /// The list `name` (a text of file names, one per line) is the
    /// scene's named text when it has one; else the file is read (5 when
    /// it doesn't exist or open) and its text entered into the scene's
    /// named-texture tree (`+0x10d54`) under the name. The text is cut at
    /// its newlines (from position 1, up to an empty line); each line
    /// (the last may lack its newline; a line's last byte, its `\r`, is
    /// dropped) is a file of the name's directory's `D3D11/`. The file
    /// must open (5 otherwise); its name and path are copied to the heap
    /// and take the first free slot of the scene's table (a null name at
    /// `+0x3c`: name and path `""`, flag 0) or a new one (`""`, `""`, 0
    /// and a 20-byte layout of the never-written words at `ebp - 0x360`,
    /// 0 here). With `async` (its low byte) the cluster loads through
    /// the async manager into a request list made for the lines, with
    /// `PhyreScene_onVfxTextureLoaded` called back with (slot, name,
    /// path, kind); else it loads and is fixed up at once (its error
    /// returned), and the slot gets flag 1, the name, the path and its
    /// layout.
    pub fn load_vfx_texture(
        h: &mut impl Host,
        ebp: u32,
        this: u32,
        kind: u32,
        name: u32,
        async_load: u32,
    ) -> u32 {
        let f = Frame {
            ebp,
            text: 0x328,
            scene: 0x32c,
            lines: 0x344,
            tail: 0x330,
            list: 0x338,
            index: 0x334,
            line_copy: 0x348,
            cluster: 0x34c,
            reader: 0x37c,
            magic: false,
        };
        h.write(f.at(f.scene), this);
        let s = f.at(f.text);
        clear_string(h, s);
        h.call_sym(STRING_ASSIGN, Conv::Thiscall(s), &[h.rebase(EMPTY), 0]);
        let found = h.call_sym(FIND_NAMED, Conv::Thiscall(this), &[name]);
        if found == 0 {
            if h.call_sym(FIOS_IS_FILE_EXIST, Conv::Cdecl, &[name]) == 0 {
                free_buffer(h, s);
                return 5;
            }
            // the stream is kept where the tail line goes later
            let stream = h.call_sym(FIOS_OPEN, Conv::Cdecl, &[name, 1]);
            h.write(f.at(f.tail), stream);
            if stream == 0 {
                free_buffer(h, s);
                return 5;
            }
            let size = h.call_sym(FIOS_FILE_SIZE, Conv::Cdecl, &[stream]);
            let buf = h.call_sym(OPERATOR_NEW_ARRAY, Conv::Cdecl, &[size.wrapping_add(1)]);
            h.write(buf.wrapping_add(size), 0u8);
            let stream: u32 = h.read(f.at(f.tail));
            h.call_sym(FIOS_READ, Conv::Cdecl, &[stream, buf, size]);
            let n = strlen(h, buf);
            h.call_sym(STRING_APPEND, Conv::Thiscall(s), &[buf, n]);
            h.call_sym(OPERATOR_DELETE_ARRAY, Conv::Cdecl, &[buf]);
            h.call_sym(FIOS_CLOSE, Conv::Cdecl, &[stream]);
            // the two `PString`s sit where the list and the index go later
            h.call_sym(PSTRING, Conv::Thiscall(f.at(f.list)), &[name]);
            let t = text(h, s);
            h.call_sym(PSTRING, Conv::Thiscall(f.at(f.index)), &[t]);
            let this: u32 = h.read(f.at(f.scene));
            h.call_sym(
                INSERT_PAIR,
                Conv::Thiscall(this.wrapping_add(NAMED_TREE)),
                &[f.at(f.list), f.at(f.index)],
            );
            free_pstring(h, f.at(f.index));
            free_pstring(h, f.at(f.list));
        } else {
            let n = strlen(h, found);
            h.call_sym(STRING_APPEND, Conv::Thiscall(s), &[found, n]);
        }
        if directory(h, ebp, name).is_none() {
            return 0;
        }
        load_lines(h, &f, kind, async_load)
    }

    /// `PhyreScene_loadMagicFileVFXTexture(this; kind, name, async)`
    /// (thiscall, `ret 0xc`; frame from `ebp - 0x884`) -> 0, 5 when the
    /// list or one of its files is missing, or the cluster fix-up's
    /// error.
    ///
    /// As [`load_vfx_texture`], with these differences: the list is
    /// always the file (read in chunks of up to 0x3ff bytes into `ebp -
    /// 0x810`; a missing or unopened list marks the scene's `+0x24` byte),
    /// never entered into the named-texture tree. When the list's
    /// directory ends in `_` and two more bytes before its `/` (`x_ab/`),
    /// a line's file that does not open is looked for in the directory
    /// without them (`x/`, `ebp - 0x410`; 5 when it has no such
    /// directory or the file doesn't open there either). A new slot's
    /// layout is pushed with `PArray_pushBack20` from `ebp - 0x87c`
    /// (inside the second reader's room: its bytes when that reader ran,
    /// else never written; 0 in the port's frame). A missing line file
    /// frees the text with the string's destructor. After the last line
    /// the scene's `+0x24` byte is set.
    pub fn load_magic_file_vfx_texture(
        h: &mut impl Host,
        ebp: u32,
        this: u32,
        kind: u32,
        name: u32,
        async_load: u32,
    ) -> u32 {
        let f = Frame {
            ebp,
            text: 0x828,
            scene: 0x82c,
            lines: 0x838,
            tail: 0x83c,
            list: 0x84c,
            index: 0x840,
            line_copy: 0x844,
            cluster: 0x848,
            reader: 0x868,
            magic: true,
        };
        h.write(f.at(f.scene), this);
        if h.call_sym(FIOS_IS_FILE_EXIST, Conv::Cdecl, &[name]) == 0 {
            h.write(this.wrapping_add(0x24), 1u8);
            return 5;
        }
        let stream = h.call_sym(FIOS_OPEN, Conv::Cdecl, &[name, 1]);
        if stream == 0 {
            h.write(this.wrapping_add(0x24), 1u8);
            return 5;
        }
        let s = f.at(f.text);
        clear_string(h, s);
        h.call_sym(STRING_ASSIGN, Conv::Thiscall(s), &[h.rebase(EMPTY), 0]);
        let chunk = ebp - CHUNK;
        loop {
            let n = h.call_sym(FIOS_READ, Conv::Cdecl, &[stream, chunk, 0x3ff]);
            if n.cast_signed() <= 0 {
                break;
            }
            if n >= 0x400 {
                h.call_sym(RANGECHECK_FAILURE, Conv::Cdecl, &[]);
                return 0;
            }
            h.write(chunk + n, 0u8);
            let len = strlen(h, chunk);
            h.call_sym(STRING_APPEND, Conv::Thiscall(s), &[chunk, len]);
        }
        h.call_sym(FIOS_CLOSE, Conv::Cdecl, &[stream]);
        let Some(end) = directory(h, ebp, name) else {
            return 0;
        };
        // the fallback directory: "x_ab/" -> "x/"
        let alt = ebp - ALT_DIR;
        h.write(alt, 0u8);
        let memset = h.rebase(MEMSET);
        h.call(memset, Conv::Cdecl, &[alt + 1, 0, 0xff]);
        let slash = end.wrapping_sub(1);
        let dir = ebp - DIR;
        if h.read::<u8>(dir.wrapping_add(slash).wrapping_sub(3)) == b'_' {
            let n = slash.wrapping_sub(3);
            import(h, STRNCPY, &[alt, dir, n]);
            h.write(alt.wrapping_add(n), b'/');
        }
        load_lines(h, &f, kind, async_load)
    }

    /// The text cut into the frame's line list: `find("\n", from, 1)` from
    /// 1 on, each hit ending a line that starts after the last one, up to
    /// no hit or an empty line; then the rest when it is inside the
    /// text and not empty. The list's count.
    fn split_lines(h: &mut impl Host, f: &Frame) -> u32 {
        let s = f.at(f.text);
        let lines = f.at(f.lines);
        h.write(lines + 4, 0u32);
        h.write(lines + 8, 0u32);
        h.write(lines, 0u32);
        let mut start = 0u32;
        let mut at = 0u32;
        loop {
            at = h.call_sym(
                STRING_FIND,
                Conv::Thiscall(s),
                &[h.rebase(NEWLINE), at.wrapping_add(1), 1],
            );
            if at == u32::MAX || at == start {
                break;
            }
            let line = text(h, s).wrapping_add(start);
            let slot = push(h, lines, RESERVE_WORDS, 4);
            if slot != 0 {
                h.write(slot, line);
            }
            start = at.wrapping_add(1);
        }
        let size: u32 = h.read(s + 0x10);
        if start < size {
            let line = text(h, s).wrapping_add(start);
            if h.read::<u8>(line) != 0 {
                h.write(f.at(f.tail), line);
                h.call_sym(PUSH_BACK, Conv::Thiscall(lines), &[f.at(f.tail)]);
            }
        }
        h.read(lines)
    }

    /// How one line ended.
    enum Line {
        /// No newline and not the last line: nothing done.
        Skipped,
        /// Loaded (or its load started).
        Loaded,
        /// The file did not open (5).
        Missing,
        /// The fix-up failed (its code).
        Failed(u32),
        /// `report_rangecheckfailure` (which does not return).
        Aborted,
    }

    /// The reader at `reader` made for the path; whether it opened (then
    /// closed).
    fn opens(h: &mut impl Host, reader: u32, path: u32) -> bool {
        h.call_sym(READER_CTOR, Conv::Thiscall(reader), &[path]);
        let open = h.call_sym(READER_IS_OPEN, Conv::Thiscall(reader), &[]);
        if open as u8 == 0 {
            return false;
        }
        h.call_sym(READER_CLOSE, Conv::Thiscall(reader), &[]);
        true
    }

    /// Line `i` of `count`: its file name into `ebp - 0x110`, its path
    /// into `ebp - 0x210`, the file opened, the scene's slot taken, the
    /// cluster loaded.
    #[allow(clippy::too_many_lines)] // one original's loop body, kept in its order
    fn load_line(
        h: &mut impl Host,
        f: &Frame,
        kind: u32,
        async_load: u32,
        i: u32,
        count: u32,
    ) -> Line {
        let ebp = f.ebp;
        let data: u32 = h.read(f.at(f.lines) + 8);
        let p: u32 = h.read(data.wrapping_add(i.wrapping_mul(4)));
        let newline = h.rebase(NEWLINE);
        let hit = import(h, STRSTR, &[p, newline]);
        let line = ebp - LINE;
        if hit == 0 {
            if i != count.wrapping_sub(1) {
                return Line::Skipped;
            }
            let n = strlen(h, p);
            import(h, STRNCPY, &[line, p, n]);
            let n = strlen(h, p);
            if n >= 0x100 {
                h.call_sym(RANGECHECK_FAILURE, Conv::Cdecl, &[]);
                return Line::Aborted;
            }
            h.write(line + n, 0u8);
        } else {
            let n = hit.wrapping_sub(p);
            import(h, STRNCPY, &[line, p, n]);
            let last = n.wrapping_sub(1);
            if last >= 0x100 {
                h.call_sym(RANGECHECK_FAILURE, Conv::Cdecl, &[]);
                return Line::Aborted;
            }
            h.write(line + last, 0u8);
        }
        let path = ebp - PATH;
        let format = h.rebase(PATH_FORMAT);
        let d3d11 = h.rebase(D3D11);
        import(h, SPRINTF, &[path, format, ebp - DIR, d3d11, line]);
        let reader = f.at(f.reader);
        if !opens(h, reader, path) {
            let found = f.magic && h.read::<u8>(ebp - ALT_DIR) != 0 && {
                import(h, SPRINTF, &[path, format, ebp - ALT_DIR, d3d11, line]);
                let reader2 = ebp - READER2;
                let found = opens(h, reader2, path);
                h.call_sym(READER_DTOR, Conv::Thiscall(reader2), &[]);
                found
            };
            if !found {
                h.call_sym(READER_DTOR, Conv::Thiscall(reader), &[]);
                return Line::Missing;
            }
        }
        let n = strlen(h, line);
        let copy = h.call_sym(OPERATOR_NEW_ARRAY, Conv::Cdecl, &[n.wrapping_add(1)]);
        h.write(f.at(f.line_copy), copy);
        strcpy(h, copy, line);
        let n = strlen(h, path);
        let path_copy = h.call_sym(OPERATOR_NEW_ARRAY, Conv::Cdecl, &[n.wrapping_add(1)]);
        h.write(f.at(f.tail), path_copy);
        strcpy(h, path_copy, path);
        let slot = take_slot(h, f);
        if async_load & 0xff != 0 {
            let list: u32 = h.read(f.at(f.list));
            let copy: u32 = h.read(f.at(f.line_copy));
            let path_copy: u32 = h.read(f.at(f.tail));
            let sign = |v: u32| (v.cast_signed() >> 31).cast_unsigned();
            let manager = h.call_sym(ASYNC_INSTANCE, Conv::Cdecl, &[]);
            let callback = h.rebase(ON_LOADED);
            h.call_sym_wide(
                LOAD_CLUSTER,
                &[
                    manager,
                    list,
                    path,
                    callback,
                    4,
                    slot,
                    sign(slot),
                    copy,
                    sign(copy),
                    path_copy,
                    sign(path_copy),
                    kind,
                    sign(kind),
                ],
            );
        } else {
            h.write(f.at(f.cluster), 0u32);
            let manager = h.call_sym(CLUSTER_MANAGER, Conv::Cdecl, &[]);
            let cluster = h.call_sym(LOAD_PCLUSTER, Conv::Thiscall(manager), &[path]);
            h.write(f.at(f.cluster), cluster);
            h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
            let failed = h.call_sym(FIXUP_CLUSTERS, Conv::Cdecl, &[f.at(f.cluster), 1]);
            if failed != 0 {
                h.call_sym(READER_DTOR, Conv::Thiscall(reader), &[]);
                return Line::Failed(failed);
            }
            let this: u32 = h.read(f.at(f.scene));
            let copy: u32 = h.read(f.at(f.line_copy));
            let path_copy: u32 = h.read(f.at(f.tail));
            let at = slot.wrapping_mul(4);
            let flags: u32 = h.read(this.wrapping_add(0x60));
            h.write(flags.wrapping_add(at), 1u32);
            let names: u32 = h.read(this.wrapping_add(0x3c));
            h.write(names.wrapping_add(at), copy);
            let paths: u32 = h.read(this.wrapping_add(0x54));
            h.write(paths.wrapping_add(at), path_copy);
            h.call_sym(SET_LAYOUT, Conv::Stdcall, &[slot, kind, copy]);
        }
        h.call_sym(READER_DTOR, Conv::Thiscall(reader), &[]);
        Line::Loaded
    }

    /// The scene's first free texture slot (a null name at `+0x3c`; its
    /// name and path set to `""`, its flag to 0), or a new slot at the
    /// end of the four arrays (names `+0x34`, paths `+0x4c`, flags
    /// `+0x58`, 20-byte layouts `+0x40`).
    fn take_slot(h: &mut impl Host, f: &Frame) -> u32 {
        let this: u32 = h.read(f.at(f.scene));
        let empty = h.rebase(EMPTY);
        let n: u32 = h.read(this.wrapping_add(0x34));
        let names: u32 = h.read(this.wrapping_add(0x3c));
        let mut k = 0u32;
        while k < n {
            if h.read::<u32>(names.wrapping_add(k.wrapping_mul(4))) == 0 {
                h.write(names.wrapping_add(k.wrapping_mul(4)), empty);
                let paths: u32 = h.read(this.wrapping_add(0x54));
                h.write(paths.wrapping_add(k.wrapping_mul(4)), empty);
                let flags: u32 = h.read(this.wrapping_add(0x60));
                h.write(flags.wrapping_add(k.wrapping_mul(4)), 0u32);
                if k != u32::MAX {
                    return k;
                }
                break;
            }
            k = k.wrapping_add(1);
        }
        let names = this.wrapping_add(0x34);
        let at = push(h, names, RESERVE_WORDS, 4);
        if at != 0 {
            h.write(at, empty);
        }
        let this: u32 = h.read(f.at(f.scene));
        let at = push(h, this.wrapping_add(0x4c), RESERVE_WORDS, 4);
        if at != 0 {
            h.write(at, empty);
        }
        let this: u32 = h.read(f.at(f.scene));
        let at = push(h, this.wrapping_add(0x58), RESERVE_FLAGS, 4);
        if at != 0 {
            h.write(at, 0u32);
        }
        if f.magic {
            // the layout from ebp - 0x87c: the second reader's bytes when
            // it ran; else never written, 0 in the port's frame
            // junk read: [ebp-0x87c..-0x86c] never written; 0 (brief 2026-10-24)
            h.call_sym(
                PUSH_BACK20,
                Conv::Thiscall(this.wrapping_add(0x40)),
                &[f.ebp - LAYOUT],
            );
        } else {
            let at = push(h, this.wrapping_add(0x40), RESERVE_LAYOUTS, 20);
            if at != 0 {
                // junk read: [ebp-0x360..-0x350] never written; 0 (brief 2026-10-24)
                for w in 0..5u32 {
                    h.write(at + 4 * w, 0u32);
                }
            }
        }
        h.read::<u32>(names).wrapping_sub(1)
    }
}
