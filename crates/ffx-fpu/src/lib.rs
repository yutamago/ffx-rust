//! The x87 instructions Unicorn (QEMU) computes through host `double`
//! functions, redone exactly as an Intel FPU does them ([`x87`]), and where
//! they are in the player's binaries ([`sites`]). Shared by `ffx-oracle`
//! (parity tests) and `ffx-host` (the development-only Unicorn fallback),
//! so original code computes the same bits in both, and as on the CPU.
//!
//! With the `unicorn` feature, [`unicorn::step`] runs one of them on an
//! emulated CPU's registers.

pub mod x87;

#[cfg(feature = "unicorn")]
pub mod unicorn;

use x87::Op;

/// A build whose instruction starts are known from a linear disassembly
/// (`objdump -d`), told apart by its PE header's `TimeDateStamp` and
/// `CheckSum`.
struct Known {
    what: &'static str,
    stamp: u32,
    check_sum: u32,
    /// RVAs of the first byte (`0xd9`) of every [`Op`] instruction.
    rvas: &'static [u32],
}

const KNOWN: [Known; 3] = [
    Known {
        what: "MSVCR110.dll 11.0.51106.1",
        stamp: 0x5098_858e,
        check_sum: 0x000e_396c,
        rvas: &[
            0x323f5, 0x32538, 0x3266d, 0x32793, 0x327c0, 0x327ca, 0x328ed, 0x329bb, 0x32b0a,
            0x32c80, 0x32ecc, 0x32ef9, 0x32f03, 0x330ae, 0x330dd, 0x330e7, 0x3854a, 0x424e2,
            0x447ff, 0x45e51, 0x46dda, 0xb182a,
        ],
    },
    // 22 `0xd9 xx` pairs in .text, none an instruction start (the one
    // objdump shows, `fcos` at RVA 0x46f0c, sits in a table of words).
    Known {
        what: "MSVCP110.dll 11.0.51106.1",
        stamp: 0x5098_8595,
        check_sum: 0x0008_ea5b,
        rvas: &[],
    },
    // 798 `0xd9 xx` pairs in .text, none an instruction start: the game's
    // own maths calls the C runtime (`_CIsin`, ...) or uses SSE.
    Known {
        what: "FFX.exe (Steam, 2026-09-10)",
        stamp: 0x6aa2_219c,
        check_sum: 0x00a3_2338,
        rvas: &[],
    },
];

/// Where the [`Op`] instructions of one module are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sites {
    /// The module's build, when known.
    pub build: Option<&'static str>,
    /// `(rva, op)`. For a known build these are exactly the instruction
    /// starts (safe to patch to `int3`); otherwise every `0xd9 xx` pair in
    /// executable code that could be one (watch them with a code hook,
    /// which fires only where an instruction really starts; never patch
    /// them).
    pub at: Vec<(u32, Op)>,
}

impl Sites {
    /// Whether [`Sites::at`] are exact instruction starts.
    #[must_use]
    pub fn exact(&self) -> bool {
        self.build.is_some()
    }
}

/// The [`Op`] sites of a module laid out as in memory (`image`, offsets
/// are RVAs) with the PE header's `stamp` and `check_sum`, `code` its
/// executable sections as `(rva, size)`.
///
/// A known build's listed RVAs are checked against `image` (each must
/// decode to the op): a mismatch drops back to the candidates.
#[must_use]
pub fn sites(image: &[u8], stamp: u32, check_sum: u32, code: &[(u32, u32)]) -> Sites {
    let op_at = |rva: u32| -> Option<Op> {
        let i = usize::try_from(rva).ok()?;
        if *image.get(i)? != 0xd9 {
            return None;
        }
        Op::decode(*image.get(i + 1)?)
    };
    if let Some(k) = KNOWN
        .iter()
        .find(|k| k.stamp == stamp && k.check_sum == check_sum)
    {
        let listed: Option<Vec<(u32, Op)>> =
            k.rvas.iter().map(|&r| op_at(r).map(|op| (r, op))).collect();
        if let Some(at) = listed {
            return Sites {
                build: Some(k.what),
                at,
            };
        }
    }
    let mut at = Vec::new();
    for &(start, size) in code {
        let end = start.saturating_add(size);
        for rva in start..end.saturating_sub(1) {
            if let Some(op) = op_at(rva) {
                at.push((rva, op));
            }
        }
    }
    Sites { build: None, at }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_builds_list_every_candidate_pair() {
        let image = [0x90, 0xd9, 0xfe, 0xd9, 0xd9, 0xf3, 0xd9];
        let s = sites(&image, 1, 2, &[(0, 7)]);
        assert!(!s.exact());
        assert_eq!(s.at, vec![(1, Op::Fsin), (4, Op::Fpatan)]);
        // outside the code ranges: nothing
        assert_eq!(sites(&image, 1, 2, &[(0, 1)]).at, vec![]);
    }

    #[test]
    fn a_known_build_whose_bytes_differ_falls_back() {
        let k = &KNOWN[0];
        let s = sites(&[0xd9, 0xfe], k.stamp, k.check_sum, &[(0, 2)]);
        assert!(!s.exact());
        assert_eq!(s.at, vec![(0, Op::Fsin)]);
        let k = &KNOWN[1];
        let s = sites(&[0xd9, 0xfe], k.stamp, k.check_sum, &[(0, 2)]);
        assert!(s.exact());
        assert_eq!(s.at, vec![]);
    }
}
