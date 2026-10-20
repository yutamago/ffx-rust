//! Reimplementations of `FFX.exe` functions, written so they can be tested on
//! any platform. Each function documents the original it replaces; the FFI
//! shims that patch them into the game live in `ffx-hook`, and `ffx-oracle`
//! checks them against the original machine code.

pub mod path;
pub mod zlib;

/// Whether mutant `id` is the active one. `tools/mutants.py` rewrites each
/// mutation site into `if ffx_core::mutant(id) { mutated } else { original }`
/// so one build serves every mutant; a run picks one with `FFX_MUTANT=id`
/// (read once; ids start at 1). Without the `mutants` feature this is a
/// constant `false` and the branch folds away.
#[cfg(feature = "mutants")]
#[must_use]
pub fn mutant(id: u32) -> bool {
    static ACTIVE: std::sync::OnceLock<u32> = std::sync::OnceLock::new();
    id != 0
        && *ACTIVE.get_or_init(|| {
            std::env::var("FFX_MUTANT")
                .ok()
                .and_then(|v| v.trim().parse().ok())
                .unwrap_or(0)
        }) == id
}

/// Whether mutant `id` is the active one: never, without the `mutants`
/// feature.
#[cfg(not(feature = "mutants"))]
#[inline]
#[must_use]
pub const fn mutant(_id: u32) -> bool {
    false
}

#[cfg(test)]
mod tests {
    #[test]
    fn no_mutant_is_active_without_ffx_mutant() {
        // 0 is never a mutant id
        assert!(!super::mutant(0));
        if std::env::var_os("FFX_MUTANT").is_none() {
            assert!(!super::mutant(1));
        }
    }
}
