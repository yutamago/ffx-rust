//! The model cache (`Cache_*`): the PS2 game's cache of loaded character
//! and monster data, which the remaster extended with the render data of
//! each entry (`docs/formats/render.md`).
//!
//! `g_CacheSlots` holds 100 pointers to entries, least recently used
//! first (`Cache_ToTop` moves an entry to the end). An entry:
//!
//! | offset | type  | field                                          |
//! |--------|-------|------------------------------------------------|
//! | 0x00   | u32   | id (model number, `ch_create_name`'s argument) |
//! | 0x04   | i8    | kind (1 character, 2 monster, 3, 4)            |
//! | 0x05   | u8    | in use                                         |
//! | 0x06   | u8    | not read yet                                   |
//! | 0x07   | u8    | locked                                         |
//! | 0x08   | u8    | flags: bits 0-2 age (7 = pinned), bit 6 unused |
//! | 0x0c   | u32   | the data (0: a free entry)                     |
//! | 0x10   | u32   | render data: the character's model             |
//! | 0x14   | u32   | render data: its second model                  |
//! | 0x18   | u32   | cloth mesh data                                |
//! | 0x1c   | u32   | cloth mesh mapping data                        |
//! | 0x20   | u32   | cloth vertex buffer                            |
//! | 0x24   | u32   | release when read (`Cache_Clear`)              |

/// Slots in the table.
pub const SLOTS: u32 = 100;

/// The originals, over game memory.
pub mod bridge {
    use super::SLOTS;
    use crate::host::{Conv, Host};
    use ffx_bridge::{Global, Mem};

    /// `g_CacheSlots`: the 100 entry pointers.
    pub const TABLE: Global<u32> = Global::new(0x0130_2f98);

    /// `Sg_MemSetType(ptr, type)` (cdecl).
    pub const SG_MEM_SET_TYPE: u32 = 0x0083_c590;
    /// `Sg_MemDump(0)` (cdecl).
    pub const SG_MEM_DUMP: u32 = 0x0083_bb50;
    /// `ch_create_name(id)` (cdecl): the model's name.
    pub const CH_CREATE_NAME: u32 = 0x0083_8100;
    /// `yiAssert(0, title, format, ...)` (cdecl, varargs).
    pub const YI_ASSERT: u32 = 0x0090_ae50;
    /// `Cache_OverFlow()`: frees the oldest unused entry; 0 when it did.
    pub const OVERFLOW: u32 = 0x0084_09a0;

    /// `"Memory Cache"`, the assertions' title.
    pub const ASSERT_TITLE: u32 = 0x00b5_a5e0;
    /// `Cache_Exist`'s message (`"CacheData isn't red yet.\n..."`).
    pub const ASSERT_NOT_READ: u32 = 0x00b5_a5b0;
    /// `Cache_Entry_NotYet`'s message (`"Can't find cache yet=%d ..."`).
    pub const ASSERT_NOT_FOUND: u32 = 0x00b5_a620;
    /// The kind names `Cache_Entry_NotYet` passes to `ch_create_name`.
    pub const KIND_NAMES: [u32; 3] = [0x00b5_6a08, 0x00b5_a61c, 0x00b8_fbeb];

    /// Slot `i`'s address.
    fn slot(m: &impl Mem, i: u32) -> u32 {
        TABLE.ptr(m).at(i).addr
    }

    /// The entry in slot `i`.
    fn entry(m: &impl Mem, i: u32) -> u32 {
        m.read(slot(m, i))
    }

    /// Whether `e` is entry `(kind, id)`; `live` also asks for its data.
    fn matches(m: &impl Mem, e: u32, kind: i32, id: u32, live: bool) -> bool {
        (!live || m.read::<u32>(e + 0xc) != 0)
            && m.read::<u32>(e) == id
            && i32::from(m.read::<i8>(e + 4)) == kind
    }

    /// The first slot holding `(kind, id)`.
    fn find(m: &impl Mem, kind: i32, id: u32, live: bool) -> Option<(u32, u32)> {
        (0..SLOTS).find_map(|i| {
            let e = entry(m, i);
            matches(m, e, kind, id, live).then_some((i, e))
        })
    }

    /// The first live entry `(kind, id)`.
    pub fn find_live(m: &impl Mem, kind: i32, id: u32) -> Option<u32> {
        find(m, kind, id, true).map(|(_, e)| e)
    }

    /// `Cache_CheckYet(kind, id)`: 2 when absent, else whether it is not
    /// read yet.
    pub fn check_yet(m: &impl Mem, kind: i32, id: u32) -> u32 {
        match find_live(m, kind, id) {
            None => 2,
            Some(e) => u32::from(m.read::<u8>(e + 6) != 0),
        }
    }

    /// `Cache_Exist3` (`+0xc`), `Cache_Exist_ClothMeshData` (`+0x18`),
    /// `Cache_Exist_ClothMeshMappingData` (`+0x1c`),
    /// `Cache_Exist_ClothVertexBuff` (`+0x20`): the field of the first
    /// entry `(kind, id)` whose field is set, else 0.
    pub fn exist_field(m: &impl Mem, kind: i32, id: u32, offset: u32) -> u32 {
        (0..SLOTS)
            .map(|i| entry(m, i))
            .find(|&e| m.read::<u32>(e + offset) != 0 && matches(m, e, kind, id, false))
            .map_or(0, |e| m.read(e + offset))
    }

    /// `Cache_Exist(kind, id)`: the entry's data, 0 when absent; asserts
    /// when it is not read yet.
    pub fn exist(h: &mut impl Host, kind: i32, id: u32) -> u32 {
        let Some(e) = find_live(h, kind, id) else {
            return 0;
        };
        if h.read::<u8>(e + 6) != 0 {
            let data: u32 = h.read(e + 0xc);
            let (title, msg) = (h.rebase(ASSERT_TITLE), h.rebase(ASSERT_NOT_READ));
            #[allow(clippy::cast_sign_loss)] // pushed as a word
            h.call_sym(
                YI_ASSERT,
                Conv::Cdecl,
                &[0, title, msg, kind as u32, id, data],
            );
        }
        h.read(e + 0xc)
    }

    /// `Cache_Entry_ClothVertexBuff(kind, id, buffer)`: on the first entry
    /// `(kind, id)`, live or not.
    pub fn entry_cloth_vertex_buff(m: &mut impl Mem, kind: i32, id: u32, buffer: u32) {
        if let Some((_, e)) = find(m, kind, id, false) {
            m.write(e + 0x20, buffer);
        }
    }

    /// `Cache_Entry_RenderData(kind, id, &a, &b, &c, &d)`: clears the
    /// entry's render-data fields and hands out their addresses (`a`
    /// `+0x10`, `b` `+0x14`, `c` `+0x18`, `d` `+0x1c`); clears `+0x20`.
    pub fn entry_render_data(m: &mut impl Mem, kind: i32, id: u32, out: [u32; 4]) {
        let Some((_, e)) = find(m, kind, id, false) else {
            return;
        };
        // the original's order
        for (field, at) in [
            (0x10, out[0]),
            (0x18, out[2]),
            (0x1c, out[3]),
            (0x14, out[1]),
        ] {
            m.write(e + field, 0u32);
            m.write(at, e + field);
        }
        m.write(e + 0x20, 0u32);
    }

    /// `Cache_SetLock(kind, id, lock)`.
    pub fn set_lock(m: &mut impl Mem, kind: i32, id: u32, lock: u32) {
        if let Some(e) = find_live(m, kind, id) {
            #[allow(clippy::cast_possible_truncation)] // the byte argument
            m.write(e + 7, lock as u8);
        }
    }

    /// `Cache_ToTop(kind, id)` (`FUN_00840cf0`): moves the entry to the
    /// last slot (most recently used) and ages it to 5 unless pinned; 1
    /// when absent, else 0.
    pub fn touch(m: &mut impl Mem, kind: i32, id: u32) -> u32 {
        let Some((i, e)) = find(m, kind, id, true) else {
            return 1;
        };
        for j in i..SLOTS - 1 {
            let next = entry(m, j + 1);
            let at = slot(m, j);
            m.write(at, next);
        }
        let last = slot(m, SLOTS - 1);
        m.write(last, e);
        let flags: u8 = m.read(e + 8);
        if flags & 7 != 7 {
            m.write(e + 8, (flags & 0xfd) | 5);
        }
        0
    }

    /// `Cache_Use(kind, id)`: in use, touched, and its memory typed 2
    /// (characters) or 3 (monsters).
    pub fn use_entry(h: &mut impl Host, kind: i32, id: u32) {
        let Some(e) = find_live(h, kind, id) else {
            return;
        };
        h.write(e + 5, 1u8);
        touch(h, kind, id);
        let mem_type = match h.read::<u8>(e + 4) {
            1 => 2,
            2 => 3,
            _ => return,
        };
        let data: u32 = h.read(e + 0xc);
        h.call_sym(SG_MEM_SET_TYPE, Conv::Cdecl, &[data, mem_type]);
    }

    /// `Cache_Unuse(kind, id)`: not in use, flagged unused (bit 6), its
    /// memory typed 4.
    pub fn unuse(h: &mut impl Host, kind: i32, id: u32) {
        let Some(e) = find_live(h, kind, id) else {
            return;
        };
        let flags: u8 = h.read(e + 8);
        h.write(e + 5, 0u8);
        h.write(e + 8, (flags & 7) | 0x40);
        let data: u32 = h.read(e + 0xc);
        h.call_sym(SG_MEM_SET_TYPE, Conv::Cdecl, &[data, 4]);
    }

    /// `Cache_Entry(kind, id, data)`: takes the first free slot (freeing
    /// the oldest entry while there is none and `Cache_OverFlow` manages
    /// to), with age 5; types the data's memory 4.
    pub fn entry_add(h: &mut impl Host, kind: u32, id: u32, data: u32) {
        loop {
            if let Some(e) = (0..SLOTS)
                .map(|i| entry(h, i))
                .find(|&e| h.read::<u32>(e + 0xc) == 0)
            {
                h.write(e, id);
                h.write(e + 0xc, data);
                h.write(e + 5, 0x0500_0000u32);
                #[allow(clippy::cast_possible_truncation)] // the byte argument
                h.write(e + 4, kind as u8);
                h.call_sym(SG_MEM_SET_TYPE, Conv::Cdecl, &[data, 4]);
                return;
            }
            if h.call_sym(OVERFLOW, Conv::Cdecl, &[]) != 0 {
                return;
            }
        }
    }

    /// `Cache_Entry_NotYet(kind, id, yet)`: the entry's not-read flag;
    /// dumps the memory map and asserts when the entry is absent.
    pub fn entry_not_yet(h: &mut impl Host, kind: i32, id: u32, yet: u32) {
        if let Some(e) = find_live(h, kind, id) {
            #[allow(clippy::cast_possible_truncation)] // the byte argument
            h.write(e + 6, yet as u8);
            return;
        }
        h.call_sym(SG_MEM_DUMP, Conv::Cdecl, &[0]);
        let names = match kind {
            1 => KIND_NAMES[0],
            2 => KIND_NAMES[1],
            _ => KIND_NAMES[2],
        };
        let names = h.rebase(names);
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_wrap)] // sar, as pushed
        let hi = ((id as i32) >> 16) as u32;
        // the original pushes `names, hi` for the assertion before the
        // name call, which takes only the id
        let name = h.call_sym(CH_CREATE_NAME, Conv::Cdecl, &[id]);
        let (title, msg) = (h.rebase(ASSERT_TITLE), h.rebase(ASSERT_NOT_FOUND));
        h.call_sym(
            YI_ASSERT,
            Conv::Cdecl,
            &[0, title, msg, yet, name, hi, names],
        );
    }
}
