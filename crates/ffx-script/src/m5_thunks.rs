//! M5 stage 1, list `script`: the Atel module's lone `jmp` thunks (five
//! bytes each, `jmp target`), which Ghidra left as labels. Each hands its
//! caller's words on to the target and returns the target's `EAX`; the
//! targets read no stack words except `AtelRemoveCtrlWork`'s index.

/// One thunk: `(address, target)`, both `ffx.sym` addresses.
pub const THUNKS: [(u32, u32); 11] = [
    (0x0086_a7f0, 0x0078_26a0), // FFXGetAsiaMode_007826a0
    (0x0086_bc70, 0x0088_d600), // TkGetLastEventJumpId
    (0x0086_bc80, 0x0088_d610), // TkGetLastEventJumpIndex
    (0x0086_c1c0, 0x0088_d660), // TkGetNowEventJumpId
    (0x0086_bca0, 0x0088_d630), // TkGetLastEventJumpMapNo
    (0x0086_bad0, 0x0078_51f0), // MsGetSaveConfigHiragana
    (0x0086_bbc0, 0x0078_51d0), // MsGetSaveConfigEnglish
    (0x0086_d940, 0x0078_0c90), // MsBattleCheck
    (0x0086_ed70, 0x0087_3d60), // the debug-list release at 0x00873d60
    (0x0087_4830, 0x0087_42b0), // AtelClearShape
    (0x0087_5ad0, 0x0086_ed40), // AtelRemoveCtrlWork(i)
];

/// The target of the thunk at `at` (an `ffx.sym` address), 0 for none.
#[must_use]
pub const fn target(at: u32) -> u32 {
    let mut i = 0;
    while i < THUNKS.len() {
        if THUNKS[i].0 == at {
            return THUNKS[i].1;
        }
        i += 1;
    }
    0
}

pub mod bridge {
    use ffx_bridge::Host;

    /// The thunk to `target` (an `ffx.sym` address): the target called
    /// with the caller's words `args`; its `EAX`.
    pub fn jmp_thunk(h: &mut impl Host, target: u32, args: &[u32]) -> u32 {
        let f = h.rebase(target);
        h.call(f, args) // call width: 1 (AtelRemoveCtrlWork's index at most)
    }
}
