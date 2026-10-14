//! `dxbc <file> [--dis]`: prints the reflection of every DXBC container in
//! a file (a `.dxbc`, a captured `.vs`/`.ps`, or a shader cluster), and on
//! Windows with `--dis` the disassembly (`D3DDisassemble` from
//! `d3dcompiler_47.dll`).

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: dxbc <file> [--dis]");
    let rest: Vec<String> = args.collect();
    let dis = rest.iter().any(|a| a == "--dis");
    let ops = rest.iter().any(|a| a == "--ops");
    let bytes = std::fs::read(&path).expect("read");
    for (at, blob) in ffx_dxbc::find_all(&bytes) {
        let c = ffx_dxbc::Container::parse(blob).expect("container");
        let (kind, major, minor) = c.program().unwrap_or(("?", 0, 0));
        println!(
            "== {at:#x}: {kind}_{major}_{minor}, {} bytes, hash {:016x}",
            blob.len(),
            ffx_dxbc::hash(blob)
        );
        for s in c.inputs().unwrap_or_default() {
            println!(
                "  in  {}{} r{} mask {:#x} sys {}",
                s.semantic, s.index, s.register, s.mask, s.system
            );
        }
        for s in c.outputs().unwrap_or_default() {
            println!(
                "  out {}{} r{} mask {:#x} sys {}",
                s.semantic, s.index, s.register, s.mask, s.system
            );
        }
        if let Ok(Some(r)) = c.reflection() {
            for b in &r.bindings {
                println!(
                    "  bind {:?} {} slot {} x{} dim {}",
                    b.kind, b.name, b.slot, b.count, b.dimension
                );
            }
            for cb in &r.buffers {
                println!("  cbuffer {} ({} bytes)", cb.name, cb.size);
                for v in &cb.variables {
                    println!(
                        "    {:4} {:4} {}{} {}x{}{}{}",
                        v.offset,
                        v.size,
                        v.name,
                        if v.elements > 0 {
                            format!("[{}]", v.elements)
                        } else {
                            String::new()
                        },
                        v.rows,
                        v.columns,
                        if v.class == 3 { " col-major" } else { "" },
                        if v.used { "" } else { " (unused)" }
                    );
                    print_members(&v.members, v.offset, 3);
                }
            }
        }
        if dis {
            disassemble(blob);
        }
        if ops {
            match c.program_code() {
                Ok(p) => {
                    for i in &p.code {
                        println!("{}", i.mnemonic());
                    }
                }
                Err(e) => println!("  decode error: {e}"),
            }
        }
    }
}

fn print_members(list: &[ffx_dxbc::Member], base: u32, indent: usize) {
    for m in list {
        println!(
            "{:indent$}{:4} {}{} {}x{}",
            "",
            base + m.offset,
            m.name,
            if m.elements > 0 {
                format!("[{}]", m.elements)
            } else {
                String::new()
            },
            m.rows,
            m.columns,
            indent = indent * 2
        );
        print_members(&m.members, base + m.offset, indent + 1);
    }
}

#[cfg(windows)]
fn disassemble(blob: &[u8]) {
    use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA};
    type Disassemble = unsafe extern "system" fn(
        *const u8,
        usize,
        u32,
        *const u8,
        *mut *mut std::ffi::c_void,
    ) -> i32;
    // SAFETY: loads the system's shader compiler and looks up a function
    // with the documented signature.
    let f: Disassemble = unsafe {
        let lib = LoadLibraryA(c"d3dcompiler_47.dll".as_ptr().cast());
        match GetProcAddress(lib, c"D3DDisassemble".as_ptr().cast()) {
            Some(f) => std::mem::transmute(f),
            None => {
                println!("  (no D3DDisassemble)");
                return;
            }
        }
    };
    // ID3DBlob: IUnknown, then GetBufferPointer (3) and GetBufferSize (4).
    let mut out: *mut std::ffi::c_void = std::ptr::null_mut();
    // SAFETY: a valid blob in, an ID3DBlob out.
    let hr = unsafe { f(blob.as_ptr(), blob.len(), 0, std::ptr::null(), &raw mut out) };
    if hr < 0 || out.is_null() {
        println!("  (D3DDisassemble failed: {hr:#x})");
        return;
    }
    // SAFETY: the blob's own vtable.
    unsafe {
        let vt = *(out as *const *const usize);
        let ptr: unsafe extern "system" fn(*mut std::ffi::c_void) -> *const u8 =
            std::mem::transmute(*vt.add(3));
        let len: unsafe extern "system" fn(*mut std::ffi::c_void) -> usize =
            std::mem::transmute(*vt.add(4));
        let text = std::slice::from_raw_parts(ptr(out), len(out));
        println!("{}", String::from_utf8_lossy(text).trim_end_matches('\0'));
        let release: unsafe extern "system" fn(*mut std::ffi::c_void) -> u32 =
            std::mem::transmute(*vt.add(2));
        release(out);
    }
}

#[cfg(not(windows))]
fn disassemble(_: &[u8]) {
    println!("  (disassembly needs the Windows build)");
}
