//! Small `TO` menu and text state: help modes, the ability map work area,
//! the item list of the ability map, pause, font width tables, the Al Bhed
//! table, key item names and help.

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    /// `TOMenuGetFirstReadEndFlag`'s flag.
    pub const FIRST_READ_END: Global<i8> = Global::new(0x0134_0859);
    /// The help mode `TOMenuHelpSetExtMode` sets.
    pub const HELP_EXT_MODE: Global<u32> = Global::new(0x0186_a228);
    /// The second value `TOMenuHelpSetExtMode` sets.
    pub const HELP_EXT_ARG: Global<u32> = Global::new(0x00c5_a088);
    /// The ability map's work area.
    pub const ABILITYMAP_WORK: Global<u8> = Global::new(0x0164_0870);
    /// Entries in the ability map's item list.
    pub const AMAP_ITEM_LIST_MAX: Global<u32> = Global::new(0x0186_a384);
    /// The ability map's item list: 4-byte entries, type (`u16`) first.
    pub const AMAP_ITEMS: Global<u32> = Global::new(0x0159_7770);
    /// `g_PauseEntered`.
    pub const PAUSE_ENTERED: Global<u32> = Global::new(0x0134_084c);
    /// `SetInvokedFromSelectCursor`'s value.
    pub const INVOKED_FROM_SELECT_CURSOR: Global<u32> = Global::new(0x0187_16f0);
    /// `TOGetMessageWindowMode`'s value.
    pub const MESSAGE_WINDOW_MODE: Global<u32> = Global::new(0x0186_59bc);
    /// The FFX-to-Al Bhed character table.
    pub const ALBHED_TABLE: Global<u8> = Global::new(0x0187_63e8);
    /// The font width table pointers, one per 16-byte font record.
    pub const FONT_WIDTH_TABLES: Global<u32> = Global::new(0x0184_1da4);

    /// `TOMenuGetFirstReadEndFlag()` (`0x008a8e50`).
    pub fn first_read_end(mem: &impl Mem) -> i32 {
        i32::from(FIRST_READ_END.get(mem))
    }

    /// `TOMenuHelpSetExtMode(mode, arg)` (`0x008bef10`).
    pub fn help_set_ext_mode(mem: &mut impl Mem, mode: u32, arg: u32) {
        HELP_EXT_MODE.set(mem, mode);
        HELP_EXT_ARG.set(mem, arg);
    }

    /// `TOMenuGetAbilitymapWorkAdrs()` (`0x008bea30`).
    pub fn abilitymap_work(mem: &impl Mem) -> u32 {
        ABILITYMAP_WORK.ptr(mem).addr
    }

    /// `TOAmapGetItemListMax()` (`0x008c8c30`).
    pub fn amap_item_list_max(mem: &impl Mem) -> u32 {
        AMAP_ITEM_LIST_MAX.get(mem)
    }

    /// `TOAmapGetItemType(i)` (`0x008c6710`).
    pub fn amap_item_type(mem: &impl Mem, i: i32) -> u16 {
        mem.read(
            AMAP_ITEMS
                .ptr(mem)
                .addr
                .wrapping_add(i.cast_unsigned().wrapping_mul(4)),
        )
    }

    /// `IsInTkMenuPause()` (`0x008a8e10`).
    pub fn is_in_pause(mem: &impl Mem) -> u32 {
        PAUSE_ENTERED.get(mem)
    }

    /// `SetInvokedFromSelectCursor(v)` (`0x008e89c0`).
    pub fn set_invoked_from_select_cursor(mem: &mut impl Mem, v: u32) {
        INVOKED_FROM_SELECT_CURSOR.set(mem, v);
    }

    /// `TOGetMessageWindowMode()` (`0x008ac7b0`).
    pub fn message_window_mode(mem: &impl Mem) -> u32 {
        MESSAGE_WINDOW_MODE.get(mem)
    }

    /// `TOGetFFXToAlbhedTblAdrs()` (`0x00906b20`).
    pub fn albhed_table(mem: &impl Mem) -> u32 {
        ALBHED_TABLE.ptr(mem).addr
    }

    /// `TOGetFontWidthTblAdrs(font)` (`0x008ac3d0`).
    pub fn font_width_table(mem: &impl Mem, font: i32) -> u32 {
        mem.read(
            FONT_WIDTH_TABLES
                .ptr(mem)
                .addr
                .wrapping_add(font.cast_unsigned().wrapping_mul(16)),
        )
    }

    /// `TOFFXToSJIS(text, out)` (`0x008b6200`): the PC port's stub, an
    /// empty string.
    pub fn ffx_to_sjis(mem: &mut impl Mem, out: u32) {
        mem.write(out, 0u8);
    }

    /// `TOMenuGetGetImportantName(id)` (`0x008ddee0`, `field` 0) and
    /// `TOMenuGetGetImportantHelp(id)` (`0x008ddea0`, `field` 8): the key
    /// item's name or help text, given `MsGetSaveConfigHiragana()` and
    /// `MsImportantPtr(id)` as (record, strings): the strings plus the
    /// record's `u16` offset (the second one, `+4`, with hiragana on).
    pub fn important_text(
        mem: &impl Mem,
        field: u32,
        hiragana: u32,
        record: u32,
        strings: u32,
    ) -> u32 {
        let off = field + if hiragana != 0 { 4 } else { 0 };
        strings.wrapping_add(u32::from(mem.read::<u16>(record.wrapping_add(off))))
    }
}
