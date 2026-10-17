//! The PC port's speed-up booster (`SpdCtrl_ScaleElapsedTime`,
//! `0x006f7340`): where it stays off.

/// Magic (`MagicFile_GetMagicId`) whose effect turns the booster off while
/// the battle window lets the player act; from the two jump tables of
/// `0x006f6ff0`, with `0x1b7..=0x1bd` checked before them.
pub const BLOCKING_MAGIC: [u32; 33] = [
    0x15b, 0x186, 0x187, 0x188, 0x189, 0x18a, 0x18b, 0x18c, 0x18d, 0x19b, 0x19c, 0x19d, 0x19e,
    0x1a9, 0x1b7, 0x1b8, 0x1b9, 0x1ba, 0x1bb, 0x1bc, 0x1bd, 0x1bf, 0x1c0, 0x1c1, 0x1c2, 0x1eb,
    0x1ec, 0x1ed, 0x1ee, 0x1ef, 0x1f4, 0x2b9, 0x2ba,
];

/// Whether map `map` keeps the booster off: one of the three maps of the
/// table at `0x00b4bf74`, or map 0x118 outside battle while the story is at
/// 0x71c..=0x721 (`scenario` is only asked then).
pub fn map_excluded(
    map: u32,
    in_battle: bool,
    table: [u32; 3],
    scenario: impl FnOnce() -> u16,
) -> bool {
    if map == 0x118 && !in_battle && scenario().wrapping_sub(0x71c) <= 5 {
        return true;
    }
    table.contains(&map)
}

/// What `0x006f6ff0` leaves in `eax` (its result is `al`): for a magic
/// `id` (`None`: no magic file loaded), with the battle window state and
/// `TOGetBtlWindowControlEnable` (asked only when the window is open).
/// `al` is 1 when the magic keeps the booster off: one of
/// [`BLOCKING_MAGIC`] while the window is closed or does not have
/// control. The upper bytes are what the original computed on its way.
pub fn magic_excluded_eax(
    id: Option<u32>,
    window_open: bool,
    control: impl FnOnce() -> u32,
) -> u32 {
    let Some(id) = id else {
        return 0xffff_ff00;
    };
    // (blocking, the eax the switch leaves before al is set)
    #[allow(clippy::cast_possible_wrap)] // signed compares
    let (blocking, eax) = if id as i32 > 0x1bd {
        let e = id.wrapping_sub(0x1bf);
        (
            e <= 0xfb && BLOCKING_MAGIC.contains(&id),
            u32::from(e > 0xfb) * e,
        )
    } else if id as i32 >= 0x1b7 {
        (true, id)
    } else {
        let e = id.wrapping_sub(0x15b);
        (
            e <= 0x4e && BLOCKING_MAGIC.contains(&id),
            u32::from(e > 0x4e) * e,
        )
    };
    if !blocking {
        return eax & !0xff;
    }
    if !window_open {
        return (eax & !0xff) | 1;
    }
    match control() {
        1 => 0,
        r => (r & !0xff) | 1,
    }
}

/// The originals, where the game keeps their state.
pub mod bridge {
    use ffx_bridge::Global;
    use ffx_graphic::host::{Conv, Host};

    /// `MsBattleGetStatus()`: 1 in battle.
    pub const MS_BATTLE_GET_STATUS: u32 = 0x0078_16f0;
    /// `getScenerioFlag()`: the story progress (`u16`).
    pub const GET_SCENERIO_FLAG: u32 = 0x0078_7360;
    /// `MagicFile_GetHandle()`: -1 with no magic file.
    pub const MAGIC_FILE_GET_HANDLE: u32 = 0x009d_a390;
    /// `MagicFile_GetMagicId()`.
    pub const MAGIC_FILE_GET_MAGIC_ID: u32 = 0x009d_a3b0;
    /// `TOGetBtlWindowControlEnable()`.
    pub const TO_GET_BTL_WINDOW_CONTROL_ENABLE: u32 = 0x0089_b590;
    /// `g_BtlWindowOpen`.
    pub const BTL_WINDOW_OPEN: Global<u32> = Global::new(0x023c_c08c);
    /// The maps that keep the booster off (`g_SpdCtrlExcludedMaps`).
    pub const EXCLUDED_MAPS: Global<[u32; 3]> = Global::new(0x00b4_bf74);

    /// `SpdCtrl_IsMapExcluded(map)` (`0x006f6fa0`): [`super::map_excluded`]
    /// as 0 or 1.
    pub fn is_map_excluded(mem: &mut impl Host, map: u32) -> u32 {
        let status = mem.call_sym(MS_BATTLE_GET_STATUS, Conv::Cdecl, &[]);
        let table = EXCLUDED_MAPS.get(mem);
        u32::from(super::map_excluded(map, status == 1, table, || {
            #[allow(clippy::cast_possible_truncation)] // a 16-bit result
            let s = mem.call_sym(GET_SCENERIO_FLAG, Conv::Cdecl, &[]) as u16;
            s
        }))
    }

    /// `SpdCtrl_IsMagicExcluded()` (`0x006f6ff0`): the magic being shown
    /// keeps the booster off; returns [`super::magic_excluded_eax`].
    pub fn is_magic_excluded(mem: &mut impl Host) -> u32 {
        let handle = mem.call_sym(MAGIC_FILE_GET_HANDLE, Conv::Cdecl, &[]);
        let id =
            (handle != u32::MAX).then(|| mem.call_sym(MAGIC_FILE_GET_MAGIC_ID, Conv::Cdecl, &[]));
        let open = id.is_some_and(|_| BTL_WINDOW_OPEN.get(mem) != 0);
        super::magic_excluded_eax(id, open, || {
            mem.call_sym(TO_GET_BTL_WINDOW_CONTROL_ENABLE, Conv::Cdecl, &[])
        })
    }
}
