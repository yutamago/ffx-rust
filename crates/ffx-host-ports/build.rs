//! Compiles ffx-hook's shims with `cfg(ffx_host)` (src/lib.rs). rustc's
//! dependency info tracks the shim files themselves.

fn main() {
    println!("cargo:rustc-cfg=ffx_host");
    println!("cargo:rerun-if-changed=build.rs");
}
