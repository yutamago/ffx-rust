//! On 32-bit Windows the arena is the process's own address space
//! (docs/host.md, "The arena"): `ffx-rust.exe` links high, so FFX.exe's
//! range (0x00400000..0x0277d000, an i686 executable's default base)
//! stays free, and large-address-aware, for the arena's regions above
//! 2 GB.

fn main() {
    let target = std::env::var("TARGET").unwrap_or_default();
    if target == "i686-pc-windows-msvc" {
        println!("cargo:rustc-link-arg-bins=/BASE:0x70000000");
        println!("cargo:rustc-link-arg-bins=/LARGEADDRESSAWARE");
    }
}
