//! The `MsGetSaveCommand(chr, cmd)` accessors in the function table at
//! `0x00c410c8` (`KernelSaveCommand_<cmd>_<addr>`): each is
//! `push cmd; push chr; call MsGetSaveCommand` and returns its `eax`. The
//! system module's copies of the same shape (`0x00771c30..`) go through
//! the same bridge, `ffx_system::tolib::bridge::kernel_save_command`.

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::Mem;

    /// Each accessor's address and the command it asks for.
    pub const SAVE_COMMANDS: [(u32, u32); 31] = [
        (0x0076_f180, 0x3056),
        (0x0076_f1d0, 0x3058),
        (0x0076_f220, 0x3059),
        (0x0076_f240, 0x305a),
        (0x0076_f290, 0x305b),
        (0x0076_f2e0, 0x305c),
        (0x0076_f330, 0x305d),
        (0x0076_f370, 0x305e),
        (0x0076_f450, 0x305f),
        (0x0077_0320, 0x3060),
        (0x0077_0950, 0x3061),
        (0x0077_0970, 0x3062),
        (0x0077_09b0, 0x3063),
        (0x0077_09f0, 0x3064),
        (0x0077_0a40, 0x3065),
        (0x0077_0a90, 0x3066),
        (0x0077_0ae0, 0x3067),
        (0x0077_0b30, 0x3068),
        (0x0077_0b80, 0x3069),
        (0x0077_0ba0, 0x306a),
        (0x0077_0bf0, 0x306b),
        (0x0077_0c40, 0x306c),
        (0x0077_0c90, 0x306d),
        (0x0077_0ce0, 0x306e),
        (0x0077_0d30, 0x306f),
        (0x0077_0d50, 0x3070),
        (0x0077_0da0, 0x3071),
        (0x0077_0df0, 0x3072),
        (0x0077_0e40, 0x3073),
        (0x0077_0e90, 0x3074),
        (0x0077_0ee0, 0x3075),
    ];

    /// The command the accessor at `at` asks for.
    #[must_use]
    pub fn command(at: u32) -> Option<u32> {
        SAVE_COMMANDS
            .iter()
            .find(|&&(addr, _)| addr == at)
            .map(|&(_, cmd)| cmd)
    }

    /// The accessor at `at` (one of [`SAVE_COMMANDS`]) for character
    /// `chr`: `MsGetSaveCommand(chr, cmd)` through `call`; 0 for an
    /// address that is none of them.
    pub fn kernel_save_command<M: Mem>(
        mem: &mut M,
        at: u32,
        chr: u32,
        call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        command(at).map_or(0, |cmd| {
            ffx_system::tolib::bridge::kernel_save_command(mem, chr, cmd, call)
        })
    }
}
