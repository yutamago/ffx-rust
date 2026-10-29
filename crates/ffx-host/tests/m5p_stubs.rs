//! The `host_only!` entries (M5 list `stubs`, `ffx_pcore::m5p_stubs`;
//! docs/porting.md, "Functions under five bytes") through ffx-host's
//! function table: the 32-bit game has no hook for them, so this is where
//! their table entries run. Every copy is called from Unicorn code the
//! way the game calls it (stack words pushed, `ecx` set, `call`, cdecl
//! callers pop), once with the ported table (the entry must run in Rust)
//! and once with no table (the original's bytes in Unicorn); `eax` (`al`
//! for the byte results), `ST0`, the caller's `ecx`/`edx` after the call
//! and the memory must agree, and the caller's stack must come back where
//! it was (a wrong pop count sends its `ret` astray). Needs
//! `FFX_GAME_DIR`; skips without it.

use std::path::PathBuf;

use ffx_bridge::Mem as _;
use ffx_host::exec::Exec;
use ffx_host::table::{Conv, Table};
use ffx_pcore::m5p_stubs::{Kind, STUBS, Stub};

fn exe() -> Option<PathBuf> {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return None;
    };
    Some(PathBuf::from(dir).join("FFX.exe"))
}

/// Calls copy `s` on `this` from code in the arena: `eax` preset to
/// `0xa5a5_a5a5`, `edx` to `0x5a5a_5a5a`; after the call `ecx` and `edx`
/// go to `out`, `out + 4`, `ST0` (float copies) to `out + 8`, and `esp`
/// minus its value before the pushes to `out + 16`. Returns `eax`.
fn call(exec: &mut Exec, s: &Stub, this: u32, out: u32) -> u32 {
    let at = exec.alloc_code(&[0; 128]).unwrap();
    let mut c = vec![0x89, 0x25]; // mov [out + 20], esp
    c.extend((out + 20).to_le_bytes());
    for i in (0..s.words).rev() {
        c.push(0x68); // push imm32
        c.extend((0x1357_9bd0 + i).to_le_bytes());
    }
    c.push(0xb8); // mov eax, imm32
    c.extend(0xa5a5_a5a5u32.to_le_bytes());
    c.push(0xba); // mov edx, imm32
    c.extend(0x5a5a_5a5au32.to_le_bytes());
    c.push(0xb9); // mov ecx, this
    c.extend(this.to_le_bytes());
    c.push(0xe8); // call rel32
    let next = at + u32::try_from(c.len()).unwrap() + 4;
    c.extend(s.addr.wrapping_sub(next).to_le_bytes());
    if !s.this && s.words > 0 {
        c.extend([0x83, 0xc4, u8::try_from(4 * s.words).unwrap()]); // add esp, n
    }
    c.extend([0x89, 0x0d]); // mov [out], ecx
    c.extend(out.to_le_bytes());
    c.extend([0x89, 0x15]); // mov [out + 4], edx
    c.extend((out + 4).to_le_bytes());
    if s.kind == Kind::Float {
        c.extend([0xdd, 0x1d]); // fstp qword [out + 8]
        c.extend((out + 8).to_le_bytes());
    }
    c.extend([0x89, 0x25]); // mov [out + 16], esp
    c.extend((out + 16).to_le_bytes());
    c.push(0xc3);
    exec.write_code(at, &c);
    match exec.call_cdecl(at, &[]) {
        Ok(eax) => eax,
        Err(e) => panic!("{:#x}: {e} ({:?})", s.addr, exec.stopped()),
    }
}

/// A block of known bytes and an `out` area, alike in both executors.
fn block(exec: &mut Exec) -> (u32, u32) {
    let bytes: Vec<u8> = (0u8..0x80).map(|i| i.wrapping_mul(37) ^ 0xa5).collect();
    let obj = exec.alloc_code(&bytes).unwrap();
    let out = exec.alloc_code(&[0; 32]).unwrap();
    (obj, out)
}

fn read(exec: &Exec, at: u32, n: usize) -> Vec<u8> {
    let mut b = vec![0; n];
    exec.mem().read_bytes(at, &mut b);
    b
}

#[test]
#[allow(clippy::many_single_char_names)] // the two runs and their results
fn m5p_stubs_run_from_the_table_as_the_original() {
    let Some(exe) = exe() else { return };
    let mut ported = ffx_host::boot_with(&exe, ffx_host::ports::ported()).unwrap();
    let mut original = ffx_host::boot_with(&exe, Table::new()).unwrap();
    let (obj, pout) = block(&mut ported);
    let (orig_obj, oout) = block(&mut original);
    assert_eq!(
        (obj, pout),
        (orig_obj, oout),
        "the two arenas allocate alike"
    );
    let mut runs = 0;
    for s in STUBS {
        let e = ported
            .table()
            .get(s.addr)
            .copied()
            .unwrap_or_else(|| panic!("{:#x} not in the table", s.addr));
        let conv = if s.this { Conv::Thiscall } else { Conv::Cdecl };
        assert_eq!((e.conv, e.words), (conv, s.words as usize), "{}", e.name);
        let thises: &[u32] = match s.kind {
            Kind::Word | Kind::Byte | Kind::Float => &[obj],
            _ => &[0, obj, u32::MAX, 0xffff_fff0],
        };
        for &this in thises {
            let before = ported.stats.ports.get(&s.addr).copied().unwrap_or(0);
            let a = call(&mut ported, s, this, pout);
            let b = call(&mut original, s, this, oout);
            assert_eq!(ported.stopped(), None, "{}", e.name);
            assert_eq!(original.stopped(), None, "{}", e.name);
            assert_eq!(
                ported.stats.ports.get(&s.addr).copied().unwrap_or(0),
                before + 1,
                "{} ran in Rust",
                e.name
            );
            match s.kind {
                Kind::False | Kind::True | Kind::Byte => {
                    assert_eq!(a & 0xff, b & 0xff, "{}: al", e.name);
                }
                // nothing returned: the caller's eax, kept
                _ => assert_eq!(a, b, "{}: eax", e.name),
            }
            let (x, y) = (read(&ported, pout, 32), read(&original, oout, 32));
            assert_eq!(x, y, "{} this {this:#x}: ecx, edx, ST0, esp", e.name);
            let esp = |v: &[u8], at: usize| u32::from_le_bytes(v[at..at + 4].try_into().unwrap());
            assert_eq!(esp(&x, 16), esp(&x, 20), "{}: the caller's esp", e.name);
            assert_eq!(
                read(&ported, obj, 0x80),
                read(&original, obj, 0x80),
                "{}: memory",
                e.name
            );
            runs += 1;
        }
    }
    assert!(original.stats.ports.is_empty());
    eprintln!("{runs} runs over {} host-only entries", STUBS.len());
}
