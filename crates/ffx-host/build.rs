//! Gives `ffx-rust.exe` its file icon (`res/ffx-rust.ico`, the project's
//! own artwork; the game's window shows `FFX.exe`'s icon at run time,
//! `win_forward::game_icon`). Windows targets only: the icon is compiled
//! with LLVM's resource compiler (`llvm-rc`, or the newest `llvm-rc-<n>` on
//! the PATH) and the `.res` goes to the linker. Without one the build goes
//! on with the default icon and a warning.

use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=res/ffx-rust.ico");
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let Some(rc) = resource_compiler() else {
        println!("cargo:warning=no llvm-rc on the PATH: ffx-rust.exe keeps the default file icon");
        return;
    };
    let dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("set by cargo"));
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("set by cargo"));
    let script = out.join("ffx-rust.rc");
    let icon = dir.join("res").join("ffx-rust.ico");
    let icon = icon.to_string_lossy().replace('\\', "/");
    std::fs::write(&script, format!("1 ICON \"{icon}\"\n")).expect("write the resource script");
    let res = out.join("ffx-rust.res");
    let ok = Command::new(&rc)
        .arg("/fo")
        .arg(&res)
        .arg(&script)
        .status()
        .is_ok_and(|s| s.success());
    if ok {
        println!("cargo:rustc-link-arg-bin=ffx-rust={}", res.display());
    } else {
        println!("cargo:warning={rc} failed: ffx-rust.exe keeps the default file icon");
    }
}

/// `llvm-rc`, else the newest `llvm-rc-<n>` in the PATH's folders.
fn resource_compiler() -> Option<String> {
    let path = std::env::var_os("PATH")?;
    let mut best: Option<(u32, String)> = None;
    for dir in std::env::split_paths(&path) {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let version = if name == "llvm-rc" || name == "llvm-rc.exe" {
                u32::MAX
            } else if let Some(n) = name.strip_prefix("llvm-rc-").and_then(|n| n.parse().ok()) {
                n
            } else {
                continue;
            };
            if best.as_ref().is_none_or(|(v, _)| version > *v) {
                best = Some((version, entry.path().to_string_lossy().into_owned()));
            }
        }
    }
    best.map(|(_, p)| p)
}
