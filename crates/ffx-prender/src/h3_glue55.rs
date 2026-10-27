//! The render-side glue, fifty-fifth batch (M4 port round 16, list
//! `h3-glue-render`): a VFX texture list unloaded from the scene's
//! texture table. The exception frame and cookie are left out, as in
//! [`crate::glue10`].

pub use crate::h3_glue54::{
    CLUSTER_MANAGER, D3D11, EMPTY, FIND_NAMED, FIOS_CLOSE, FIOS_FILE_SIZE, FIOS_IS_FILE_EXIST,
    FIOS_OPEN, FIOS_READ, NEWLINE, OPERATOR_DELETE, OPERATOR_DELETE_ARRAY, OPERATOR_NEW_ARRAY,
    PATH_FORMAT, PUSH_BACK, RANGECHECK_FAILURE, RESERVE_WORDS, SPRINTF, STRING_APPEND,
    STRING_ASSIGN, STRING_FIND, STRING_TIDY, STRNCPY, STRSTR, USER_FREE,
};
/// `Phyre_PRendering_PRenderer_syncRender` (called with the renderer in
/// `ecx`, which it ignores).
pub const SYNC_RENDER: u32 = 0x005b_c3f0;
/// `ClusterManager_releasePCluster_0069be90(manager; path)` (`ret 4`).
pub const RELEASE_PCLUSTER: u32 = 0x0069_be90;
/// The string's `+= text` (`0x0064f5e0`, `this; text`, `ret 4`).
pub const STRING_APPEND_TEXT: u32 = 0x0064_f5e0;
/// `graphicCharacterRestoreSummonTexture()` (cdecl).
pub const RESTORE_SUMMON_TEXTURE: u32 = 0x0063_d550;
/// `g_CurrentMapNo`.
pub const CURRENT_MAP_NO: u32 = 0x012f_bc40;
/// The image's three summon texture names
/// (`"0000N_19_0_0_1_1.dds.phyre"`), which mode 0x276..0x27d keeps.
pub const SUMMON_TEXTURES: [u32; 3] = [0x00b4_2a44, 0x00b4_2a60, 0x00b4_2a7c];

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use crate::h3_glue49::bridge::{clear_string, text};
    use crate::h3_glue54::bridge::{free_buffer, import, push, strcpy, strlen};
    use ffx_graphic::host::{Conv, Host};

    /// The frame (below `ebp`): `mode - 0x276`, the name, the scene (later
    /// the tail line's pointer), the line list (count, capacity, data),
    /// a word used in turn as the layout offset, the stream and the line
    /// index; the list's text as a string; the path; the directory; one
    /// line.
    const MODE: u32 = 0x344;
    const NAME: u32 = 0x340;
    const SCENE: u32 = 0x33c;
    const LINES: u32 = 0x338;
    const W32C: u32 = 0x32c;
    const TEXT: u32 = 0x328;
    const PATH: u32 = 0x310;
    const DIR: u32 = 0x210;
    const LINE: u32 = 0x110;

    /// The two strings equal (`strcmp(a, b) == 0`).
    fn same(h: &impl Host, a: u32, b: u32) -> bool {
        let mut k = 0u32;
        loop {
            let x: u8 = h.read(a.wrapping_add(k));
            if x != h.read::<u8>(b.wrapping_add(k)) {
                return false;
            }
            if x == 0 {
                return true;
            }
            k = k.wrapping_add(1);
        }
    }

    /// Slot `k`'s word of the scene's array whose data pointer is at
    /// `this + field`.
    fn slot(h: &impl Host, this: u32, field: u32, k: u32) -> u32 {
        let data: u32 = h.read(this.wrapping_add(field));
        data.wrapping_add(k.wrapping_mul(4))
    }

    /// Slot `k`'s name and path freed and cleared, its flag cleared.
    fn clear_slot(h: &mut impl Host, this: u32, k: u32) {
        let name: u32 = h.read(slot(h, this, 0x3c, k));
        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[name]);
        let path: u32 = h.read(slot(h, this, 0x54, k));
        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[path]);
        h.write(slot(h, this, 0x3c, k), 0u32);
        h.write(slot(h, this, 0x54, k), 0u32);
        h.write(slot(h, this, 0x60, k), 0u32);
    }

    /// Slot `k`'s cluster released by its path, then the slot cleared.
    fn release_slot(h: &mut impl Host, this: u32, k: u32) {
        let path: u32 = h.read(slot(h, this, 0x54, k));
        let manager = h.call_sym(CLUSTER_MANAGER, Conv::Cdecl, &[]);
        h.call_sym(RELEASE_PCLUSTER, Conv::Thiscall(manager), &[path]);
        clear_slot(h, this, k);
    }

    /// Whether slot `k`'s path holds one of the summon texture names
    /// (`strstr` through its slot, the path re-read for each).
    fn summon_path(h: &mut impl Host, this: u32, k: u32) -> bool {
        for name in SUMMON_TEXTURES {
            let path: u32 = h.read(slot(h, this, 0x54, k));
            let name = h.rebase(name);
            if import(h, STRSTR, &[path, name]) != 0 {
                return true;
            }
        }
        false
    }

    /// The loaded slots of layout kind `kind` (the 20-byte layouts at
    /// `+0x48`, as many as `+0x40` counts) released, the frame's word
    /// `ebp - 0x32c` stepping through the layouts. With `keep_summons`
    /// (kind 2), while the scene's summon byte `+0xfb04` is set, slots
    /// whose path holds a summon texture name stay.
    fn release_kind(h: &mut impl Host, ebp: u32, this: u32, kind: u32, keep_summons: bool) {
        if h.read::<u32>(this.wrapping_add(0x4c)) == 0 {
            return;
        }
        let mut k = 0u32;
        let mut off = 0u32;
        h.write(ebp - W32C, off);
        loop {
            let kept = keep_summons
                && h.read::<u8>(this.wrapping_add(0xfb04)) != 0
                && h.read::<u32>(slot(h, this, 0x54, k)) != 0
                && summon_path(h, this, k);
            if !kept
                && h.read::<u32>(slot(h, this, 0x60, k)) != 0
                && k < h.read::<u32>(this.wrapping_add(0x40))
                && h.read::<u32>(h.read::<u32>(this.wrapping_add(0x48)).wrapping_add(off)) == kind
            {
                release_slot(h, this, k);
                off = h.read(ebp - W32C);
            }
            k = k.wrapping_add(1);
            off = off.wrapping_add(0x14);
            h.write(ebp - W32C, off);
            if k >= h.read::<u32>(this.wrapping_add(0x4c)) {
                return;
            }
        }
    }

    /// `PhyreScene_unloadVFXTexture(this; name, mode)` (thiscall, `ret 8`;
    /// frame from `ebp - 0x344`) -> 0, or 5 when the list is missing.
    ///
    /// The renderer's method 2 and `syncRender` first. Modes 0x276..0x27d
    /// set the scene's summon byte `+0xfb04`, 0x26e..0x275 clear it. Mode
    /// 0x400 releases every loaded slot of layout kind 1; modes other
    /// than -1 and 0x26e..0x27d every loaded slot of kind 2 (but summon
    /// textures while the summon byte is set). Modes -1 and 0x26e..0x27d
    /// unload the list `name` (the scene's named text, else the file:
    /// 5 when it doesn't exist or open): its lines (as in
    /// `PhyreScene_loadVFXTexture`) give paths under the name's
    /// directory's `D3D11/`, whose clusters are released and whose slots
    /// are cleared (summon textures kept for modes 0x276..0x27d). Modes
    /// 0x26e..0x275 then release every slot with a summon texture and
    /// restore the characters' summon textures.
    #[allow(clippy::too_many_lines)] // one original, kept in its order
    pub fn unload_vfx_texture(h: &mut impl Host, ebp: u32, this: u32, name: u32, mode: u32) -> u32 {
        h.write(ebp - SCENE, this);
        h.write(ebp - NAME, name);
        let renderer: u32 = h.read(this.wrapping_add(0xd0));
        h.call_virtual(renderer, 2, &[]);
        let renderer: u32 = h.read(this.wrapping_add(0xd0));
        h.call_sym(SYNC_RENDER, Conv::Thiscall(renderer), &[]);
        let summon_mode = mode.wrapping_sub(0x276);
        h.write(ebp - MODE, summon_mode);
        if summon_mode <= 7 {
            h.write(this.wrapping_add(0xfb04), 1u8);
        }
        if mode.wrapping_sub(0x26e) <= 7 {
            h.write(this.wrapping_add(0xfb04), 0u8);
        }
        let signed = mode.cast_signed();
        if mode == 0x400 {
            release_kind(h, ebp, this, 1, false);
            return 0;
        }
        if signed != -1 && !(0x26e..=0x27d).contains(&signed) {
            release_kind(h, ebp, this, 2, true);
            return 0;
        }
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
            h.write(ebp - W32C, stream);
            if stream == 0 {
                h.call_sym(STRING_TIDY, Conv::Thiscall(s), &[]);
                return 5;
            }
            let size = h.call_sym(FIOS_FILE_SIZE, Conv::Cdecl, &[stream]);
            let buf = h.call_sym(OPERATOR_NEW_ARRAY, Conv::Cdecl, &[size.wrapping_add(1)]);
            h.write(buf.wrapping_add(size), 0u8);
            let stream: u32 = h.read(ebp - W32C);
            h.call_sym(FIOS_READ, Conv::Cdecl, &[stream, buf, size]);
            h.call_sym(STRING_APPEND_TEXT, Conv::Thiscall(s), &[buf]);
            h.call_sym(OPERATOR_DELETE_ARRAY, Conv::Cdecl, &[buf]);
            h.call_sym(FIOS_CLOSE, Conv::Cdecl, &[stream]);
        } else {
            let n = strlen(h, found);
            h.call_sym(STRING_APPEND, Conv::Thiscall(s), &[found, n]);
        }
        let name: u32 = h.read(ebp - NAME);
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
        let mut i = 0u32;
        h.write(ebp - W32C, i);
        while i < count {
            if !unload_line(h, ebp, this, i) {
                return 0;
            }
            i = h.read::<u32>(ebp - W32C).wrapping_add(1);
            h.write(ebp - W32C, i);
        }
        if count.cast_signed() < 0 && h.read::<u32>(ebp - LINES + 4).cast_signed() < 0 {
            h.call_sym(RESERVE_WORDS, Conv::Thiscall(ebp - LINES), &[0]);
        }
        h.write(ebp - LINES, 0u32);
        let map: u32 = h.read(h.rebase(CURRENT_MAP_NO));
        let map_summon = (mode == 0x29e || mode == 0x29f) && map == 0x146;
        if (0x26e..=0x275).contains(&signed) || map_summon {
            let mut k = 0u32;
            while k < h.read::<u32>(this.wrapping_add(0x4c)) {
                if h.read::<u32>(slot(h, this, 0x54, k)) != 0 && summon_path(h, this, k) {
                    release_slot(h, this, k);
                }
                k = k.wrapping_add(1);
            }
            h.call_sym(RESTORE_SUMMON_TEXTURE, Conv::Cdecl, &[]);
            let map: u32 = h.read(h.rebase(CURRENT_MAP_NO));
            if (mode == 0x29e || mode == 0x29f) && map == 0x146 {
                h.write(this.wrapping_add(0xfb04), 0u8);
            }
        }
        let data: u32 = h.read(ebp - LINES + 8);
        if data != 0 {
            h.call_sym(USER_FREE, Conv::Cdecl, &[data]);
        }
        free_buffer(h, s);
        0
    }

    /// The text cut into the frame's line list, as in
    /// `PhyreScene_loadVFXTexture` (the tail line's pointer at `ebp -
    /// 0x33c`, over the scene's copy). The list's count.
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
                h.write(ebp - SCENE, line);
                h.call_sym(PUSH_BACK, Conv::Thiscall(lines), &[ebp - SCENE]);
            }
        }
        h.read(lines)
    }

    /// Line `i`: its file name into `ebp - 0x110` (as the loader cuts
    /// it); unless it is a summon texture kept in modes 0x276..0x27d, its
    /// path (`ebp - 0x310`) released and every slot with that path
    /// cleared. False after `report_rangecheckfailure`.
    fn unload_line(h: &mut impl Host, ebp: u32, this: u32, i: u32) -> bool {
        let data: u32 = h.read(ebp - LINES + 8);
        let p: u32 = h.read(data.wrapping_add(i.wrapping_mul(4)));
        let newline = h.rebase(NEWLINE);
        let hit = import(h, STRSTR, &[p, newline]);
        let line = ebp - LINE;
        if hit == 0 {
            let count: u32 = h.read(ebp - LINES);
            if i != count.wrapping_sub(1) {
                return true;
            }
            let n = strlen(h, p);
            import(h, STRNCPY, &[line, p, n]);
            let n = strlen(h, p);
            if n >= 0x100 {
                h.call_sym(RANGECHECK_FAILURE, Conv::Cdecl, &[]);
                return false;
            }
            h.write(line + n, 0u8);
        } else {
            let n = hit.wrapping_sub(p);
            import(h, STRNCPY, &[line, p, n]);
            let last = n.wrapping_sub(1);
            if last >= 0x100 {
                h.call_sym(RANGECHECK_FAILURE, Conv::Cdecl, &[]);
                return false;
            }
            h.write(line + last, 0u8);
        }
        if h.read::<u32>(ebp - MODE) <= 7 {
            for name in SUMMON_TEXTURES {
                let name = h.rebase(name);
                if same(h, line, name) {
                    return true;
                }
            }
        }
        let path = ebp - PATH;
        let format = h.rebase(PATH_FORMAT);
        let d3d11 = h.rebase(D3D11);
        import(h, SPRINTF, &[path, format, ebp - DIR, d3d11, line]);
        let manager = h.call_sym(CLUSTER_MANAGER, Conv::Cdecl, &[]);
        h.call_sym(RELEASE_PCLUSTER, Conv::Thiscall(manager), &[path]);
        let mut k = 0u32;
        while k < h.read::<u32>(this.wrapping_add(0x4c)) {
            let p: u32 = h.read(slot(h, this, 0x54, k));
            if p != 0 && same(h, p, path) {
                clear_slot(h, this, k);
            }
            k = k.wrapping_add(1);
        }
        true
    }
}
