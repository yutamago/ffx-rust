//! The C runtime helpers in ffx-host's function table (`crate::crt`)
//! against `FFX.exe`'s originals in Unicorn, called from emulated code and
//! from Rust. Needs `FFX_GAME_DIR`; skips without it.

use std::path::PathBuf;

use ffx_bridge::Mem as _;
use ffx_host::exec::Exec;
use ffx_host::table::Table;

fn exe() -> Option<PathBuf> {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return None;
    };
    Some(PathBuf::from(dir).join("FFX.exe"))
}

/// The ported table and the original one (no ports at all).
fn pair() -> Option<(Exec, Exec)> {
    let exe = exe()?;
    let ported = ffx_host::boot(&exe).unwrap();
    let original = ffx_host::boot_with(&exe, Table::new()).unwrap();
    Some((ported, original))
}

/// A thiscall "constructor": `[ecx] = ++[counter]; [ecx + 4] = ecx`, so
/// the element order and each `this` show in memory. Returns (code,
/// counter).
fn marker(exec: &mut Exec) -> (u32, u32) {
    let counter = exec.alloc(16);
    let mut code = vec![0xa1]; // mov eax, [counter]
    code.extend_from_slice(&counter.to_le_bytes());
    code.push(0x40); // inc eax
    code.push(0xa3); // mov [counter], eax
    code.extend_from_slice(&counter.to_le_bytes());
    code.extend_from_slice(&[0x89, 0x01, 0x89, 0x49, 0x04, 0xc3]); // mov [ecx], eax; mov [ecx+4], ecx; ret
    (exec.alloc_code(&code).unwrap(), counter)
}

/// Code that calls `addr` with `words` pushed (stdcall/cdecl by `pops`),
/// returning `eax`.
fn call_from_unicorn(exec: &mut Exec, addr: u32, words: &[u32], cdecl: bool) -> u32 {
    let at = exec.alloc_code(&[0; 64]).unwrap();
    let mut code = Vec::new();
    for &w in words.iter().rev() {
        code.push(0x68);
        code.extend_from_slice(&w.to_le_bytes());
    }
    code.push(0xe8);
    let next = at + u32::try_from(code.len()).unwrap() + 4;
    code.extend_from_slice(&addr.wrapping_sub(next).to_le_bytes());
    if cdecl && !words.is_empty() {
        code.extend_from_slice(&[0x83, 0xc4, u8::try_from(4 * words.len()).unwrap()]);
    }
    code.push(0xc3);
    exec.write_code(at, &code);
    exec.call_cdecl(at, &[]).unwrap()
}

/// What one array call leaves: `eax`, the elements, the counter.
fn array_run(
    exec: &mut Exec,
    addr: u32,
    size: u32,
    count: u32,
    from_rust: bool,
) -> (u32, Vec<u8>, u32) {
    let (f, counter) = marker(exec);
    let n = size * count.cast_signed().max(0).cast_unsigned();
    let ptr = exec.alloc(n.max(4) + 8);
    exec.mem()
        .write_bytes(ptr, &vec![0xee; (n.max(4) + 8) as usize]);
    // the constructor iterator's fifth word is the unwind destructor
    let words: Vec<u32> = if addr == ffx_host::crt::EH_VECTOR_CTOR {
        vec![ptr, size, count, f, 0]
    } else {
        vec![ptr, size, count, f]
    };
    let eax = if from_rust {
        exec.call_stdcall(addr, &words).unwrap()
    } else {
        call_from_unicorn(exec, addr, &words, false)
    };
    let mut bytes = vec![0u8; (n.max(4) + 8) as usize];
    exec.mem().read_bytes(ptr, &mut bytes);
    // `this` in the elements is an address of this executor's: as offsets
    for chunk in bytes.chunks_mut(4) {
        if chunk.len() == 4 {
            let w = u32::from_le_bytes(chunk.try_into().unwrap());
            if (ptr..ptr + n.max(4)).contains(&w) {
                chunk.copy_from_slice(&(w - ptr).to_le_bytes());
            }
        }
    }
    let c: u32 = exec.mem().read(counter);
    (eax, bytes, c)
}

#[test]
fn the_array_loops_match_the_originals() {
    let Some((mut ported, mut original)) = pair() else {
        return;
    };
    for addr in [ffx_host::crt::EH_VECTOR_CTOR, ffx_host::crt::EH_VECTOR_DTOR] {
        for (size, count) in [(8, 5), (12, 1), (16, 0), (8, u32::MAX), (24, 3)] {
            for from_rust in [false, true] {
                let a = array_run(&mut ported, addr, size, count, from_rust);
                let b = array_run(&mut original, addr, size, count, from_rust);
                assert_eq!(
                    a, b,
                    "{addr:#x} size {size} count {count} from Rust {from_rust}"
                );
            }
        }
    }
    let n = |e: &Exec, a| e.stats.ports.get(&a).copied().unwrap_or(0);
    assert_eq!(n(&ported, ffx_host::crt::EH_VECTOR_CTOR), 10);
    assert_eq!(n(&ported, ffx_host::crt::EH_VECTOR_DTOR), 10);
    assert_eq!(n(&original, ffx_host::crt::EH_VECTOR_CTOR), 0);
}

#[test]
fn atexit_registers_as_the_original() {
    let Some((mut ported, mut original)) = pair() else {
        return;
    };
    let table = [0x025d_61ac, 0x025d_61b0];
    for (i, f) in [0x0040_1000u32, 0x0040_1090, 0].into_iter().enumerate() {
        let mut got = Vec::new();
        for e in [&mut ported, &mut original] {
            let before = e.shared().at_exit.lock().unwrap().len();
            let (addr, from_rust) = if i % 2 == 0 {
                (ffx_host::crt::ATEXIT, false)
            } else {
                (ffx_host::crt::ONEXIT, true)
            };
            let eax = if from_rust {
                e.call_cdecl(addr, &[f]).unwrap()
            } else {
                call_from_unicorn(e, addr, &[f], true)
            };
            let added: Vec<u32> = e.shared().at_exit.lock().unwrap()[before..].to_vec();
            let words: Vec<u32> = table.iter().map(|&a| e.mem().read(a)).collect();
            got.push((eax, added, words));
        }
        assert_eq!(got[0], got[1], "f {f:#x}");
    }
}

/// The one-instruction functions (`ffx_host::leaf`) called from Rust
/// against the same calls into Unicorn with no table: each shape on a
/// real function of `FFX.exe`, `eax` compared (the void ones leave the
/// caller's `eax`, garbage: only that nothing else changed).
#[test]
fn one_instruction_functions_match_unicorn() {
    use ffx_host::leaf::{self, Leaf};
    let Some((mut ported, mut original)) = pair() else {
        return;
    };
    let this = ported.alloc(0x20);
    let other = original.alloc(0x20);
    assert_eq!(this, other, "both heaps allocate alike");
    for e in [&mut ported, &mut original] {
        let bytes: Vec<u8> = (0x80u8..0xa0).collect();
        e.mem().write_bytes(this, &bytes);
        // the global CharacterTextureAnimationTweakUtil_GetInstance reads
        e.mem().write(0x00cc_cad4, 0x1234_5678u32);
    }
    let cases: [(u32, Leaf); 8] = [
        (0x0062_f4c0, Leaf::Void),            // FlushCache: ret
        (0x0069_3530, Leaf::Void),            // degradeResolution: ret 4
        (0x0062_f6e0, Leaf::Const(0)),        // sceDmaGetChan: xor eax, eax
        (0x00a4_4770, Leaf::Const(u32::MAX)), // sceSifDmaStat: or eax, -1
        (0x0067_acc0, Leaf::Const(1)),        // TrophyManager_IsInstallCompleted: mov al, 1
        (0x006b_6340, Leaf::Global(0x00cc_cad4)),
        (0x0063_12e0, Leaf::ThisWord(0)), // getButtonInfo: mov eax, [ecx]
        (0x006d_5670, Leaf::ThisByte(5)), // isActivatedForCurrentFrame: mov al, [ecx+5]
    ];
    for &(addr, want) in &cases {
        assert_eq!(leaf::decode(&ported.mem(), addr), Some(want), "{addr:#x}");
        let args = ffx_host::table::Args {
            ecx: this,
            stack: vec![7],
            ..Default::default()
        };
        let conv = ffx_host::table::Conv::Thiscall;
        let calls = ported.stats.unicorn_calls;
        let a = ported.call(addr, conv, &args).unwrap();
        assert_eq!(
            ported.stats.unicorn_calls, calls,
            "{addr:#x} ran in Unicorn"
        );
        let b = original.call(addr, conv, &args).unwrap();
        if want != Leaf::Void {
            // `mov al`: the upper bytes are the caller's in Unicorn
            let mask = if matches!(want, Leaf::ThisByte(_) | Leaf::Const(1)) {
                0xff
            } else {
                u32::MAX
            };
            assert_eq!(a.eax & mask, b.eax & mask, "{addr:#x}");
        }
    }
    // not leaves: a real function, alloca_probe's register dance
    assert_eq!(leaf::decode(&ported.mem(), 0x0094_9620), None);
    assert_eq!(leaf::decode(&ported.mem(), 0x0094_966a), None);
    assert_eq!(ported.stats.leaf_calls, 8);
}
