//! M5 stage 1, list `render-pcui` (round 3), batch 5: the debug GUI's
//! character selector handler, the `PUtility` destructor, the page
//! heap's locked bucket call, lock constructor and statistics pass.

/// The originals over game memory.
pub mod bridge {
    use ffx_bridge::Mem;
    use ffx_graphic::host::{Conv, Host};

    pub const SELECTOR_GET_ITEM_VALUE: u32 = 0x0084_9d10;
    pub const WINDOW_SET_ARG: u32 = 0x0084_b560;
    pub const WINDOW_CONTROL_SET_TEXT: u32 = 0x0084_a710;
    pub const WINDOW_SET_WORD30: u32 = 0x0084_b7e0;
    /// `g_GuChrSelector` and the window it feeds (`0x0132591c`).
    pub const CHR_SELECTOR: u32 = 0x0132_5918;
    pub const CHR_WINDOW: u32 = 0x0132_591c;
    /// `Phyre_PUtility_vftable` and the utilities' list head.
    pub const VT_PUTILITY: u32 = 0x00b0_e078;
    pub const UTILITY_LIST: u32 = 0x00c9_01a8;
    pub const OPERATOR_DELETE: u32 = 0x0063_0390;
    /// `KERNEL32!EnterCriticalSection`, `LeaveCriticalSection`,
    /// `InitializeCriticalSection`.
    pub const ENTER_CRITICAL_SECTION_IAT: u32 = 0x00b0_c154;
    pub const LEAVE_CRITICAL_SECTION_IAT: u32 = 0x00b0_c158;
    pub const INIT_CRITICAL_SECTION_IAT: u32 = 0x00b0_c150;
    pub const BUCKET_CALL: u32 = 0x0094_3e30;
    pub const STORE_COMPUTED: u32 = 0x0094_39d0;
    pub const COMMITTED_BYTES: u32 = 0x0094_37f0;
    pub const RESERVED_BYTES: u32 = 0x0094_3800;
    pub const PAGE_MANAGER: u32 = 0x00c6_13b0;
    pub const PAGE_MANAGER_STATS: u32 = 0x0094_6f50;

    fn word(h: &impl Mem, at: u32) -> u32 {
        h.read(at)
    }

    fn cdecl(h: &mut impl Host, f: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Cdecl, args)
    }

    fn import(h: &mut impl Host, iat: u32, args: &[u32]) -> u32 {
        let f = word(h, h.rebase(iat));
        h.call(f, Conv::Stdcall, args)
    }

    /// `Gu_ChrSelectorProc_00853d40(window, message)` -> 0: on message
    /// 0x8011 the character selector's chosen item becomes the character
    /// window's argument, its name (`+4`) the window's text 0x4d1 (when
    /// an item is chosen), and `window`'s word `+0x30` 1.
    pub fn chr_selector_proc(h: &mut impl Host, window: u32, message: u32) -> u32 {
        if message != 0x8011 {
            return 0;
        }
        let sel = word(h, h.rebase(CHR_SELECTOR));
        let item = cdecl(h, SELECTOR_GET_ITEM_VALUE, &[sel]);
        let w = word(h, h.rebase(CHR_WINDOW));
        cdecl(h, WINDOW_SET_ARG, &[w, item]);
        if item != 0 {
            let w = word(h, h.rebase(CHR_WINDOW));
            let name = word(h, item.wrapping_add(4));
            cdecl(h, WINDOW_CONTROL_SET_TEXT, &[w, 0x4d1, name]);
        }
        cdecl(h, WINDOW_SET_WORD30, &[window, 1]);
        0
    }

    /// `PUtility_scalarDeletingDtor_00a3e3e0(this; flags)` (`ret 4`): the
    /// base vtable stored, the utility unlinked from the list at
    /// `0x00c901a8` (links at `+4`) when found, deleted with flag bit 0
    /// -> `this`.
    pub fn utility_delete(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        let vt = h.rebase(VT_PUTILITY);
        h.write(this, vt);
        let head = h.rebase(UTILITY_LIST);
        if word(h, head) != 0 {
            let mut link = head;
            loop {
                let node = word(h, link);
                if node == this {
                    break;
                }
                link = node.wrapping_add(4);
                if word(h, link) == 0 {
                    break;
                }
            }
            if word(h, link) != 0 {
                let next = word(h, this + 4);
                h.write(link, next);
                h.write(this + 4, 0_u32);
            }
        }
        if flags & 1 != 0 {
            cdecl(h, OPERATOR_DELETE, &[this]);
        }
        this
    }

    /// `PageBucket_lockedCall_00948820(this)`: under the lock `[+0x10]`,
    /// `FUN_00943e30(lock + 0x18; [+4])` -> `LeaveCriticalSection`'s `eax`.
    pub fn locked_call(h: &mut impl Host, this: u32) -> u32 {
        let cs = word(h, this + 0x10);
        let x = word(h, this + 4);
        import(h, ENTER_CRITICAL_SECTION_IAT, &[cs]);
        h.call_sym(BUCKET_CALL, Conv::Thiscall(cs.wrapping_add(0x18)), &[x]);
        import(h, LEAVE_CRITICAL_SECTION_IAT, &[cs])
    }

    /// `PageLock_ctor_009488a0(lock)`: `InitializeCriticalSection(lock)`,
    /// then `StoreComputed_009439d0(lock + 0x18; 0x4000)` -> `lock`.
    pub fn lock_ctor(h: &mut impl Host, this: u32) -> u32 {
        import(h, INIT_CRITICAL_SECTION_IAT, &[this]);
        h.call_sym(STORE_COMPUTED, Conv::Thiscall(this + 0x18), &[0x4000]);
        this
    }

    /// `PageStats_update_00947cc0()`: the committed and reserved byte
    /// getters (their results unused), then `0x00946f50(g_PageManager)`
    /// when there is one -> its `eax`, else the reserved bytes.
    pub fn stats_update(h: &mut impl Host) -> u32 {
        cdecl(h, COMMITTED_BYTES, &[]);
        let r = cdecl(h, RESERVED_BYTES, &[]);
        let pm = word(h, h.rebase(PAGE_MANAGER));
        if pm == 0 {
            return r;
        }
        h.call_sym(PAGE_MANAGER_STATS, Conv::Thiscall(pm), &[])
    }
}
