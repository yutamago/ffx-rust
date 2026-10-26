//! The render-side glue, twenty-fourth batch (M4 port round 6, list
//! `h3-glue-render`): the colour-correction cluster lookup. The exception
//! frame and cookie are left out, as in [`crate::glue10`].

/// `std::string::assign(this, p, n)` (`0x00404270`), `append(this, p, n)`,
/// `Phyre_PhyrePrintf(level, text)` (cdecl), `AssetListReader_getSymbolId
/// (this, name)`, `PostProcessManager_SetFirstEntry(this, list)`,
/// `operator delete(p)` (cdecl), `"_ColorCorrection"` (16 bytes), `"No
/// models loaded."` and the empty string.
pub const STRING_ASSIGN: u32 = 0x0040_4270;
pub const STRING_APPEND: u32 = 0x0065_76d0;
pub const PHYRE_PRINTF: u32 = crate::glue10::PHYRE_PRINTF;
pub const GET_SYMBOL_ID: u32 = 0x006f_4350;
pub const SET_FIRST_ENTRY: u32 = 0x0069_8400;
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
pub const COLOR_CORRECTION: u32 = 0x00b4_233c;
pub const NO_MODELS: u32 = 0x00b4_2350;
pub const EMPTY: u32 = 0x00b8_fbeb;

/// The map path's parts: `"/bin"`, `"/2d"` (4 bytes), `".ahwin32"` (9
/// bytes), the folder format, `fiosUnifyFilename(path, out, size)` and
/// `sprintf`'s import slot (cdecl).
pub const BIN: u32 = 0x00b4_22d4;
pub const TWO_D: u32 = 0x00b4_22dc;
pub const AHWIN32: u32 = 0x00b4_22e0;
pub const DATA_FORMAT: u32 = 0x00b4_22ec;
pub const FIOS_UNIFY_FILENAME: u32 = 0x0067_9820;
pub const SPRINTF: u32 = crate::glue9::SPRINTF;

/// The dynamic-geometry memory pools: the pool release `0x006de860(this,
/// kind)`, `operator new(size)` (`0x00630310`, cdecl), the list
/// constructor `0x006de2c0(this)`, the block sizes per kind,
/// `CreateMutexW`'s import slot (stdcall), `operator_new_006304c0(size)`,
/// `Phyre_PhyreMalloc_0042fb80(size, align)` (cdecl) and
/// `PArray_PushInto_006dea60(this, storage, &item)`.
pub const POOL_RELEASE: u32 = 0x006d_e860;
pub const OPERATOR_NEW: u32 = 0x0063_0310;
pub const POOL_LIST_CTOR: u32 = 0x006d_e2c0;
pub const POOL_BLOCK_SIZES: u32 = 0x00b4_a650;
pub const CREATE_MUTEX: u32 = 0x00b0_c198;
pub const OPERATOR_NEW_6304C0: u32 = 0x0063_04c0;
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
pub const ARRAY_PUSH_INTO: u32 = 0x006d_ea60;

/// The mesh-matrix list: `PMatrix4_toWorldMatrix3x4(dst, m)` and
/// `renderMeshInstance(this, mesh, data, 0, context)`.
pub const TO_WORLD_MATRIX: u32 = 0x0045_1190;
pub const RENDER_MESH_INSTANCE: u32 = 0x005b_9a90;

/// The VFX table's clear by type: the map head maker `0x006a0390()`,
/// the map lock `0x006a0fe0(this)`, `__iob_func` / `fprintf` (import
/// slots), the two messages, the kept map's find-or-make
/// `0x0069c630(this, &key)` and insert `0x0069cfe0(this, &out, 0, &value,
/// node)`, `ClassVFXRenderDataTable_InsertToDeletePool(this, a, b, kind,
/// effect)`, `VfxRenderDataMap_eraseRange_0069d990(this, first, last)`,
/// `std_Tree_erase_range_006a19d0(this, &out, first, last)` and
/// `operator delete(p)` (cdecl).
pub const MAP_HEAD: u32 = 0x006a_0390;
pub const MAP_LOCK: u32 = 0x006a_0fe0;
pub const IOB_FUNC: u32 = crate::glue4::IOB_FUNC;
pub const FPRINTF: u32 = crate::glue4::FPRINTF;
pub const CLEAR_SIZE_MESSAGE: u32 = 0x00b4_801c;
pub const CLEAR_REMAINING_MESSAGE: u32 = 0x00b4_8060;
pub const KEPT_FIND: u32 = 0x0069_c630;
pub const KEPT_INSERT: u32 = 0x0069_cfe0;
pub const INSERT_TO_DELETE_POOL: u32 = 0x0069_f900;
pub const ERASE_RANGE: u32 = 0x0069_d990;
pub const TREE_ERASE_RANGE: u32 = 0x006a_19d0;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// An empty MSVC `std::string` at `s` (16-byte buffer, size `+0x10`,
    /// capacity `+0x14`).
    fn string_empty(h: &mut impl Host, s: u32) {
        h.write(s + 0x14, 0xfu32);
        h.write(s + 0x10, 0u32);
        h.write(s, 0u8);
    }

    /// The string's text: the heap pointer from capacity 16 on, else the
    /// inline buffer.
    fn string_text(h: &impl Host, s: u32) -> u32 {
        if h.read::<u32>(s + 0x14) >= 0x10 {
            h.read(s)
        } else {
            s
        }
    }

    /// The heap text freed from capacity 16 on.
    fn string_free(h: &mut impl Host, s: u32) {
        if h.read::<u32>(s + 0x14) >= 0x10 {
            let p: u32 = h.read(s);
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[p]);
        }
    }

    /// `PhyreScene_setupColorCorrection(this)` (`ecx`; frame: the name at
    /// `ebp - 0x28`, an empty string at `ebp - 0x40`): the map's name
    /// (`+0xd8`) with `"_ColorCorrection"` is looked up among the clusters
    /// (`+0x10e4`, their lists `+0x10e0`), then the shared ones (`+0x1144`,
    /// `+0x1134`), and the post-process manager (`+0x10cc`) takes the
    /// list found (or none). Without a name it only prints. Returns 0.
    pub fn scene_setup_color_correction(h: &mut impl Host, ebp: u32, this: u32) -> u32 {
        let name = ebp - 0x28;
        let empty = ebp - 0x40;
        string_empty(h, name);
        let map: u32 = h.read(this + 0xd8);
        if map == 0 {
            let text = h.rebase(NO_MODELS);
            h.call_sym(PHYRE_PRINTF, Conv::Cdecl, &[1, text]);
        } else {
            let mut len = 0u32;
            while h.read::<u8>(map.wrapping_add(len)) != 0 {
                len += 1;
            }
            h.call_sym(STRING_ASSIGN, Conv::Thiscall(name), &[map, len]);
            let suffix = h.rebase(COLOR_CORRECTION);
            h.call_sym(STRING_APPEND, Conv::Thiscall(name), &[suffix, 0x10]);
        }
        string_empty(h, empty);
        let text = h.rebase(EMPTY);
        h.call_sym(STRING_ASSIGN, Conv::Thiscall(empty), &[text, 0]);
        let mut list = 0;
        for (reader, lists) in [(0x10e4, 0x10e0), (0x1144, 0x1134)] {
            let key = string_text(h, name);
            let id = h.call_sym(GET_SYMBOL_ID, Conv::Thiscall(this + reader), &[key]);
            if id != u32::MAX {
                let base: u32 = h.read(this + lists);
                list = h.read(base.wrapping_add(id.wrapping_mul(4)));
                break;
            }
        }
        let post: u32 = h.read(this + 0x10cc);
        h.call_sym(SET_FIRST_ENTRY, Conv::Thiscall(post), &[list]);
        string_free(h, empty);
        string_empty(h, empty);
        string_free(h, name);
        0
    }

    fn strlen(h: &impl Host, s: u32) -> u32 {
        let mut n = 0u32;
        while h.read::<u8>(s.wrapping_add(n)) != 0 {
            n += 1;
        }
        n
    }

    /// C `strstr`.
    fn strstr(h: &impl Host, hay: u32, needle: u32) -> u32 {
        let mut at = hay;
        loop {
            let mut k = 0u32;
            loop {
                let n: u8 = h.read(needle.wrapping_add(k));
                if n == 0 {
                    return at;
                }
                if h.read::<u8>(at.wrapping_add(k)) != n {
                    break;
                }
                k += 1;
            }
            if h.read::<u8>(at) == 0 {
                return 0;
            }
            at = at.wrapping_add(1);
        }
    }

    /// C `strrchr`.
    fn strrchr(h: &impl Host, s: u32, c: u8) -> u32 {
        let mut found = 0;
        let mut k = 0u32;
        loop {
            let b: u8 = h.read(s.wrapping_add(k));
            if b == c {
                found = s.wrapping_add(k);
            }
            if b == 0 {
                return found;
            }
            k += 1;
        }
    }

    /// C `strncpy`: up to `n` bytes of `src`, the rest of `n` zeros.
    fn strncpy(h: &mut impl Host, dst: u32, src: u32, n: u32) {
        let mut ended = false;
        for k in 0..n {
            let b: u8 = if ended {
                0
            } else {
                h.read(src.wrapping_add(k))
            };
            ended |= b == 0;
            h.write(dst.wrapping_add(k), b);
        }
    }

    /// `PhyreScene_convertMapPath(asset, out, keep2d)` (stdcall; frame:
    /// the map folder at `ebp - 0x84`, the unified format at `ebp -
    /// 0x184`, `out` at `- 0x188`, the last `/` at `- 0x18c`): the asset's
    /// path (`+0x17`) up to `"/bin"`, its last segment repeated (after
    /// `"/2d"` unless `keep2d`), with `".ahwin32"`, put into the game's
    /// data folder (`out`, through `sprintf`). `eax`: `sprintf`'s.
    pub fn scene_convert_map_path(
        h: &mut impl Host,
        ebp: u32,
        asset: u32,
        out: u32,
        keep2d: u32,
    ) -> u32 {
        let folder = ebp - 0x84;
        let format = ebp - 0x184;
        h.write(ebp - 0x188, out);
        h.write(out, 0u32);
        h.write_bytes(folder, &[0; 0x80]);
        let src = asset.wrapping_add(0x17);
        let bin = h.rebase(BIN);
        let mut n = strstr(h, src, bin).wrapping_sub(src);
        strncpy(h, folder, src, n);
        let last = strrchr(h, folder, b'/');
        h.write(ebp - 0x18c, last);
        let len = strlen(h, last);
        if keep2d as u8 == 0 {
            let end = folder + strlen(h, folder);
            let two_d: u32 = h.read(h.rebase(TWO_D));
            h.write(end, two_d);
            n = n.wrapping_add(3);
        }
        strncpy(h, folder.wrapping_add(n), last, len);
        let end = folder + strlen(h, folder);
        let mut suffix = [0u8; 9];
        h.read_bytes(h.rebase(AHWIN32), &mut suffix);
        h.write_bytes(end, &suffix);
        let data = h.rebase(DATA_FORMAT);
        h.call_sym(FIOS_UNIFY_FILENAME, Conv::Cdecl, &[data, format, 0x100]);
        let slot = h.rebase(SPRINTF);
        let sprintf: u32 = h.read(slot);
        h.call(sprintf, Conv::Cdecl, &[out, format, folder])
    }

    /// `DynGeoMemManager_createPool_006de720(this, kind; slots)` (`ret 4`;
    /// frame: `kind * 4` at `ebp - 0x14`, the new block at `ebp - 0x10`;
    /// the list, then its slot's address, over `kind`'s argument slot):
    /// the kind's pool is released and replaced by a new list
    /// (`this + 4 + 4 kind`) of two blocks, each with a mutex and the
    /// kind's size allocated (`+0` size, `+4`/`+8` data, `+0xc` mutex);
    /// the first block becomes the list's current one (`+0`, with `+4`
    /// -1 -> 0), and the count `+8` grows.
    pub fn dyn_geo_create_pool(h: &mut impl Host, ebp: u32, slots: u32, this: u32, kind: u32) {
        h.call_sym(POOL_RELEASE, Conv::Thiscall(this), &[kind]);
        let list = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x18]);
        h.write(slots, list);
        let list = if list == 0 {
            0
        } else {
            h.call_sym(POOL_LIST_CTOR, Conv::Thiscall(list), &[])
        };
        let offset = kind.wrapping_mul(4);
        let slot = this.wrapping_add(4).wrapping_add(offset);
        h.write(ebp - 0x14, offset);
        h.write(slots, slot);
        h.write(slot, list);
        for _ in 0..2 {
            let block = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x10]);
            h.write(ebp - 0x10, block);
            let block = if block == 0 {
                0
            } else {
                let sizes = h.rebase(POOL_BLOCK_SIZES);
                let size: u32 = h.read(sizes.wrapping_add(offset));
                let at = h.rebase(CREATE_MUTEX);
                let create: u32 = h.read(at);
                let mutex = h.call(create, Conv::Stdcall, &[0, 0, 0]);
                h.write(block + 0xc, mutex);
                h.write(block + 4, 0u32);
                h.write(block + 8, 0u32);
                let data = h.call_sym(OPERATOR_NEW_6304C0, Conv::Cdecl, &[size]);
                h.write(block + 4, data);
                h.write(block + 8, data);
                h.write(block, size);
                block
            };
            let slot: u32 = h.read(slots);
            let list: u32 = h.read(slot);
            h.write(ebp - 0x10, block);
            let count = h.read::<u32>(list.wrapping_add(0xc)) & 0x7fff_ffff;
            let storage = h.call_sym(
                PHYRE_MALLOC,
                Conv::Cdecl,
                &[count.wrapping_add(1).wrapping_mul(4), 4],
            );
            h.call_sym(
                ARRAY_PUSH_INTO,
                Conv::Thiscall(list.wrapping_add(0xc)),
                &[storage, ebp - 0x10],
            );
            if h.read::<u32>(list.wrapping_add(4)) == u32::MAX {
                h.write(list.wrapping_add(4), 0u32);
                let blocks: u32 = h.read(list.wrapping_add(0x10));
                let first: u32 = h.read(blocks);
                let data: u32 = h.read(first.wrapping_add(4));
                h.write(list, data);
            }
            let n = h.read::<u32>(list.wrapping_add(8)).wrapping_add(1);
            h.write(list.wrapping_add(8), n);
        }
    }

    /// `fld dword` + `fstp dword`.
    fn quiet(word: u32) -> u32 {
        x87::store(x87::load(word))
    }

    /// `PhyreScene_DrawMeshMatrices(this, restore)` (`ret 4`; frame: the
    /// 4x4 at `ebp - 0x84`, the saved 3x4 at `ebp - 0x44`, the mesh at
    /// `ebp - 0x14`, a row at `ebp - 0x10`): each entry of the
    /// `graphicAddMeshMatrix` list (`+0x10d88`, 0x44 bytes: the mesh, then
    /// a 4x4 whose rows' x and y are negated) becomes the mesh's world
    /// matrix (`[mesh + 4]`, saved first) and the mesh is drawn; with
    /// `restore` the saved matrix is put back. Every float goes through
    /// the x87.
    pub fn scene_draw_mesh_matrices(h: &mut impl Host, ebp: u32, this: u32, restore: u32) {
        let matrix = ebp - 0x84;
        let saved = ebp - 0x44;
        let row = ebp - 0x10;
        let mut i = 0u32;
        while i < h.read::<u32>(this + 0x1_0d84) & 0x7fff_ffff {
            let list: u32 = h.read(this + 0x1_0d88);
            let entry = list.wrapping_add(0x44 * i);
            let mesh: u32 = h.read(entry);
            h.write(ebp - 0x14, mesh);
            let world: u32 = h.read(mesh.wrapping_add(4));
            for r in 0..4 {
                let from = entry + 4 + 0x10 * r;
                for c in 0..4 {
                    let word: u32 = h.read(from + 4 * c);
                    let v = if c < 2 {
                        x87::store(-x87::load(word))
                    } else {
                        quiet(word)
                    };
                    h.write(row + 4 * c, v);
                }
                for c in 0..4 {
                    let word: u32 = h.read(row + 4 * c);
                    h.write(matrix + 0x10 * r + 4 * c, quiet(word));
                }
            }
            for k in 0..12 {
                let word: u32 = h.read(world.wrapping_add(4 * k));
                h.write(saved + 4 * k, quiet(word));
            }
            h.call_sym(TO_WORLD_MATRIX, Conv::Thiscall(world), &[matrix]);
            let renderer: u32 = h.read(this + 0xd0);
            let context: u32 = h.read(renderer.wrapping_add(0x13c));
            let mesh: u32 = h.read(ebp - 0x14);
            let data: u32 = h.read(mesh.wrapping_add(0x10));
            h.call_sym(
                RENDER_MESH_INSTANCE,
                Conv::Thiscall(renderer),
                &[mesh, data, 0, context],
            );
            if restore as u8 != 0 {
                let mesh: u32 = h.read(ebp - 0x14);
                let world: u32 = h.read(mesh.wrapping_add(4));
                for k in 0..12 {
                    let word: u32 = h.read(saved + 4 * k);
                    h.write(world.wrapping_add(4 * k), quiet(word));
                }
            }
            i += 1;
        }
    }

    fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let at = h.rebase(slot);
        let f: u32 = h.read(at);
        h.call(f, Conv::Cdecl, args)
    }

    /// `ClassVFXRenderDataTable_ClearByType(this, mask; slots)` (`ret 4`;
    /// frame: the key record `{a, b, effect}` at `ebp - 0x34`, the insert's
    /// result at `ebp - 0x24`, the kept map `{head, size}` at `ebp - 0x1c`,
    /// `b` at `ebp - 0x14`, `this` at `ebp - 0x10`): the effects of the
    /// map (in order) with a key whose type byte (`effect + 0x51`) meets
    /// `mask` go into a new map; the others are stopped (virtual `+0x10`)
    /// and sent to the delete pool (kind 3). The sizes are printed to
    /// `stdout`; then the table's map is emptied and the kept map erased
    /// (`mask`'s slot takes the erase's result) and its head freed.
    pub fn vfx_table_clear_by_type(h: &mut impl Host, ebp: u32, slots: u32, this: u32, mask: u32) {
        h.write(ebp - 0x10, this);
        let table_head: u32 = h.read(this);
        let mut node: u32 = h.read(table_head);
        let kept = ebp - 0x1c;
        h.write(kept, 0u32);
        h.write(kept + 4, 0u32);
        let head = h.call_sym(MAP_HEAD, Conv::Thiscall(kept), &[]);
        h.write(kept, head);
        h.call_sym(MAP_LOCK, Conv::Thiscall(kept), &[]);
        let size: u32 = h.read(this + 4);
        let text = h.rebase(CLEAR_SIZE_MESSAGE);
        let iob = import(h, IOB_FUNC, &[]);
        import(h, FPRINTF, &[iob.wrapping_add(0x20), text, size]);
        while node != h.read::<u32>(this) {
            let a: u32 = h.read(node.wrapping_add(0x10));
            let b: u32 = h.read(node.wrapping_add(0x14));
            h.write(ebp - 0x14, b);
            if a | b != 0 {
                let effect: u32 = h.read(node.wrapping_add(0x18));
                if (mask as u8) & h.read::<u8>(effect.wrapping_add(0x51)) != 0 {
                    let key = ebp - 0x34;
                    h.write(key + 4, b);
                    h.write(key, a);
                    h.write(key + 8, effect);
                    let found = h.call_sym(KEPT_FIND, Conv::Thiscall(kept), &[key]);
                    h.call_sym(
                        KEPT_INSERT,
                        Conv::Thiscall(kept),
                        &[ebp - 0x24, 0, found.wrapping_add(0x10), found],
                    );
                } else {
                    h.call_virtual(effect, 4, &[]);
                    let this: u32 = h.read(ebp - 0x10);
                    let b: u32 = h.read(ebp - 0x14);
                    h.call_sym(
                        INSERT_TO_DELETE_POOL,
                        Conv::Thiscall(this),
                        &[a, b, 3, effect],
                    );
                }
            }
            node = crate::glue10::bridge::successor(h, node);
        }
        let size: u32 = h.read(kept + 4);
        let text = h.rebase(CLEAR_REMAINING_MESSAGE);
        let iob = import(h, IOB_FUNC, &[]);
        import(h, FPRINTF, &[iob.wrapping_add(0x20), text, size]);
        let this: u32 = h.read(ebp - 0x10);
        h.call_sym(MAP_LOCK, Conv::Thiscall(this), &[]);
        let head: u32 = h.read(kept);
        let first: u32 = h.read(head);
        h.call_sym(ERASE_RANGE, Conv::Thiscall(this), &[first, head]);
        h.call_sym(MAP_LOCK, Conv::Thiscall(kept), &[]);
        let head: u32 = h.read(kept);
        let first: u32 = h.read(head);
        h.call_sym(
            TREE_ERASE_RANGE,
            Conv::Thiscall(kept),
            &[slots, first, head],
        );
        let head: u32 = h.read(kept);
        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[head]);
    }
}
