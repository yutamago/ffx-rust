//! Shared helpers of round 4's own-list family tests (`magic_o4_*.rs`):
//! every copy of a family checked against its own DLL.
#![allow(dead_code)]

use crate::magic::Twin;

/// Every copy `(DLL, RVA)` of a family, each checked to bind to one of
/// the family's ports (named `"<label> NNNN@AAAA"`, the copy's own or,
/// for a copy whose code is the same as another's, that one's). Empty
/// without `FFX_GAME_DIR`.
pub fn copies(label: &str, list: &[(u16, u32)]) -> Vec<(u16, u32)> {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        return Vec::new();
    };
    let dir = std::path::PathBuf::from(dir).join("magicFiles/FFX");
    for &(n, rva) in list {
        let name = format!("magic_{n:04}.dll");
        let file = std::fs::read(dir.join(&name)).expect("the DLL");
        let pe = ffx_magic::pe::Pe::parse(&file).expect("a PE");
        let img = ffx_magic::Image::new(&name, &file, pe.preferred_base).expect("an image");
        let a = img.at(rva);
        let i = ffx_magic::bind(&img, a).unwrap_or_else(|| panic!("{n}: nothing binds at {a:#x}"));
        let name = ffx_magic::PORTS[i].0;
        assert!(
            name.strip_prefix(label)
                .and_then(|r| r.strip_prefix(' '))
                .is_some_and(|r| r.len() == 9 && r.as_bytes()[4] == b'@'),
            "{n}: {a:#x} binds to {name}, not {label}"
        );
    }
    list.to_vec()
}

/// The distinct DLLs of some copies, in order.
pub fn dlls(lists: &[&[(u16, u32)]]) -> Vec<u16> {
    let mut v: Vec<u16> = lists.iter().flat_map(|l| l.iter().map(|c| c.0)).collect();
    v.sort_unstable();
    v.dedup();
    v
}

/// `n` words of `t` at `at`, distinct and non-zero, from `seed`.
pub fn fill(t: &mut Twin, at: u32, n: u32, seed: u32) {
    for i in 0..n {
        t.set_u32(
            at + 4 * i,
            seed.wrapping_mul(0x9e37_79b9).wrapping_add(i) | 1,
        );
    }
}
