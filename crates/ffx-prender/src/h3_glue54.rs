//! The render-side glue, fifty-fourth batch (M4 port round 16, list
//! `h3-glue-render`): a VFX texture list loaded into the scene's texture
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

    /// The frame (below `ebp`): the list's text as a string, the scene,
    /// the line list (count, capacity, data), the list's directory, the
    /// file path, one line, the reader object.
    const TEXT: u32 = 0x328;
    const SCENE: u32 = 0x32c;
    const LINES: u32 = 0x344;
    const DIR: u32 = 0x310;
    const PATH: u32 = 0x210;
    const LINE: u32 = 0x110;
    const READER: u32 = 0x37c;
    /// Words used for several things in turn: the stream, then the
    /// tail's line pointer, then the path's copy (`- 0x330`); the name's
    /// `PString`, then the request list (`- 0x338`); the text's
    /// `PString`, then the line index (`- 0x334`); the line's copy
    /// (`- 0x348`); the cluster (`- 0x34c`).
    const W330: u32 = 0x330;
    const W334: u32 = 0x334;
    const W338: u32 = 0x338;
    const LINE_COPY: u32 = 0x348;
    const CLUSTER: u32 = 0x34c;

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

    /// The early ends: the line list's data freed (user heap), the text's
    /// buffer freed; `code`.
    fn finish(h: &mut impl Host, ebp: u32, code: u32) -> u32 {
        let data: u32 = h.read(ebp - LINES + 8);
        if data != 0 {
            h.call_sym(USER_FREE, Conv::Cdecl, &[data]);
        }
        free_buffer(h, ebp - TEXT);
        code
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
    #[allow(clippy::too_many_lines)] // one original, kept in its order
    pub fn load_vfx_texture(
        h: &mut impl Host,
        ebp: u32,
        this: u32,
        kind: u32,
        name: u32,
        async_load: u32,
    ) -> u32 {
        h.write(ebp - SCENE, this);
        let s = ebp - TEXT;
        clear_string(h, s);
        h.call_sym(STRING_ASSIGN, Conv::Thiscall(s), &[h.rebase(EMPTY), 0]);
        let found = h.call_sym(FIND_NAMED, Conv::Thiscall(this), &[name]);
        if found == 0 {
            if h.call_sym(FIOS_IS_FILE_EXIST, Conv::Cdecl, &[name]) == 0 {
                free_buffer(h, s);
                return 5;
            }
            let stream = h.call_sym(FIOS_OPEN, Conv::Cdecl, &[name, 1]);
            h.write(ebp - W330, stream);
            if stream == 0 {
                free_buffer(h, s);
                return 5;
            }
            let size = h.call_sym(FIOS_FILE_SIZE, Conv::Cdecl, &[stream]);
            let buf = h.call_sym(OPERATOR_NEW_ARRAY, Conv::Cdecl, &[size.wrapping_add(1)]);
            h.write(buf.wrapping_add(size), 0u8);
            let stream: u32 = h.read(ebp - W330);
            h.call_sym(FIOS_READ, Conv::Cdecl, &[stream, buf, size]);
            let n = strlen(h, buf);
            h.call_sym(STRING_APPEND, Conv::Thiscall(s), &[buf, n]);
            h.call_sym(OPERATOR_DELETE_ARRAY, Conv::Cdecl, &[buf]);
            h.call_sym(FIOS_CLOSE, Conv::Cdecl, &[stream]);
            h.call_sym(PSTRING, Conv::Thiscall(ebp - W338), &[name]);
            let t = text(h, s);
            h.call_sym(PSTRING, Conv::Thiscall(ebp - W334), &[t]);
            let this: u32 = h.read(ebp - SCENE);
            h.call_sym(
                INSERT_PAIR,
                Conv::Thiscall(this.wrapping_add(NAMED_TREE)),
                &[ebp - W338, ebp - W334],
            );
            free_pstring(h, ebp - W334);
            free_pstring(h, ebp - W338);
        } else {
            let n = strlen(h, found);
            h.call_sym(STRING_APPEND, Conv::Thiscall(s), &[found, n]);
        }
        // the name's directory, up to its last '/' (searched from its end
        // without a lower bound, as in the original)
        let dir = ebp - DIR;
        let mut end = strlen(h, name);
        strcpy(h, dir, name);
        while h.read::<u8>(dir.wrapping_add(end)) != b'/' {
            end = end.wrapping_sub(1);
        }
        end = end.wrapping_add(1);
        if end >= 0x100 {
            h.call_sym(RANGECHECK_FAILURE, Conv::Cdecl, &[]);
            return 0;
        }
        h.write(dir + end, 0u8);
        let count = split_lines(h, ebp);
        h.write(ebp - W338, 0u32);
        if async_load & 0xff != 0 {
            let manager = h.call_sym(ASYNC_INSTANCE, Conv::Cdecl, &[]);
            let list = h.call_sym(CHILDREN_REQUEST_LIST, Conv::Thiscall(manager), &[count]);
            h.write(ebp - W338, list);
        }
        let mut i = 0u32;
        h.write(ebp - W334, i);
        while i < count {
            match load_line(h, ebp, kind, async_load, i, count) {
                Line::Skipped | Line::Loaded => {}
                Line::Failed(code) => return finish(h, ebp, code),
                Line::Aborted => return 0,
            }
            i = h.read::<u32>(ebp - W334).wrapping_add(1);
            h.write(ebp - W334, i);
        }
        if count.cast_signed() < 0 && h.read::<u32>(ebp - LINES + 4).cast_signed() < 0 {
            h.call_sym(RESERVE_WORDS, Conv::Thiscall(ebp - LINES), &[0]);
        }
        finish(h, ebp, 0)
    }

    /// The text cut into the frame's line list: `find("\n", from, 1)` from
    /// 1 on, each hit ending a line that starts after the last one, up to
    /// no hit or an empty line; then the rest when it is inside the
    /// text and not empty. The list's count.
    fn split_lines(h: &mut impl Host, ebp: u32) -> u32 {
        let s = ebp - TEXT;
        let lines = ebp - LINES;
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
                h.write(ebp - W330, line);
                h.call_sym(PUSH_BACK, Conv::Thiscall(lines), &[ebp - W330]);
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
        /// The file did not open (5), or the fix-up failed (its code).
        Failed(u32),
        /// `report_rangecheckfailure` (which does not return).
        Aborted,
    }

    /// Line `i` of `count`: its file name into `ebp - 0x110`, its path
    /// into `ebp - 0x210`, the file opened, the scene's slot taken, the
    /// cluster loaded.
    fn load_line(
        h: &mut impl Host,
        ebp: u32,
        kind: u32,
        async_load: u32,
        i: u32,
        count: u32,
    ) -> Line {
        let data: u32 = h.read(ebp - LINES + 8);
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
        let reader = ebp - READER;
        h.call_sym(READER_CTOR, Conv::Thiscall(reader), &[path]);
        let open = h.call_sym(READER_IS_OPEN, Conv::Thiscall(reader), &[]);
        if open as u8 == 0 {
            h.call_sym(READER_DTOR, Conv::Thiscall(reader), &[]);
            return Line::Failed(5);
        }
        h.call_sym(READER_CLOSE, Conv::Thiscall(reader), &[]);
        let n = strlen(h, line);
        let copy = h.call_sym(OPERATOR_NEW_ARRAY, Conv::Cdecl, &[n.wrapping_add(1)]);
        h.write(ebp - LINE_COPY, copy);
        strcpy(h, copy, line);
        let n = strlen(h, path);
        let path_copy = h.call_sym(OPERATOR_NEW_ARRAY, Conv::Cdecl, &[n.wrapping_add(1)]);
        h.write(ebp - W330, path_copy);
        strcpy(h, path_copy, path);
        let slot = take_slot(h, ebp);
        if async_load & 0xff != 0 {
            let list: u32 = h.read(ebp - W338);
            let copy: u32 = h.read(ebp - LINE_COPY);
            let path_copy: u32 = h.read(ebp - W330);
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
            h.write(ebp - CLUSTER, 0u32);
            let manager = h.call_sym(CLUSTER_MANAGER, Conv::Cdecl, &[]);
            let cluster = h.call_sym(LOAD_PCLUSTER, Conv::Thiscall(manager), &[path]);
            h.write(ebp - CLUSTER, cluster);
            h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
            let failed = h.call_sym(FIXUP_CLUSTERS, Conv::Cdecl, &[ebp - CLUSTER, 1]);
            if failed != 0 {
                h.call_sym(READER_DTOR, Conv::Thiscall(reader), &[]);
                return Line::Failed(failed);
            }
            let this: u32 = h.read(ebp - SCENE);
            let copy: u32 = h.read(ebp - LINE_COPY);
            let path_copy: u32 = h.read(ebp - W330);
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
    fn take_slot(h: &mut impl Host, ebp: u32) -> u32 {
        let this: u32 = h.read(ebp - SCENE);
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
        let this: u32 = h.read(ebp - SCENE);
        let at = push(h, this.wrapping_add(0x4c), RESERVE_WORDS, 4);
        if at != 0 {
            h.write(at, empty);
        }
        let this: u32 = h.read(ebp - SCENE);
        let at = push(h, this.wrapping_add(0x58), RESERVE_FLAGS, 4);
        if at != 0 {
            h.write(at, 0u32);
        }
        let at = push(h, this.wrapping_add(0x40), RESERVE_LAYOUTS, 20);
        if at != 0 {
            // junk read: [ebp-0x360..-0x350] never written; 0 (brief 2026-10-24)
            for w in 0..5u32 {
                h.write(at + 4 * w, 0u32);
            }
        }
        h.read::<u32>(names).wrapping_sub(1)
    }
}
