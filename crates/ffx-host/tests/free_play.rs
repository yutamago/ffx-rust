//! The imports the free play reached or can reach that had no Rust side
//! (WP7; docs/host.md, "Imports"): `OLEAUT32`'s `BSTR`s, the string
//! resources the error boxes load, `TerminateThread`. On the player's
//! `FFX.exe`; skips without `FFX_GAME_DIR`.

use std::path::PathBuf;
use std::time::Duration;

use ffx_bridge::Mem;
use ffx_host::exec::Exec;

fn exe() -> Option<PathBuf> {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return None;
    };
    Some(PathBuf::from(dir).join("FFX.exe"))
}

/// The address the IAT slot of `dll!name` holds (the import's stub).
fn import(exec: &Exec, dll: &str, name: &str) -> u32 {
    let slot = exec
        .image()
        .imports
        .iter()
        .find(|i| i.dll.eq_ignore_ascii_case(dll) && i.name == name)
        .unwrap_or_else(|| panic!("{dll}!{name} not imported"))
        .slot;
    exec.mem().read(slot)
}

/// `.rsrc`'s game addresses.
fn rsrc(exec: &Exec) -> std::ops::Range<u32> {
    let s = exec
        .image()
        .sections
        .iter()
        .find(|s| s.name == ".rsrc")
        .expect(".rsrc");
    s.start..s.start + s.size
}

#[test]
fn string_resources_and_bstrs_as_the_error_boxes_use_them() {
    let Some(exe) = exe() else { return };
    let mut exec = ffx_host::boot(&exe).unwrap();
    let base = exec.image().base;
    let k32 = "KERNEL32.dll";
    // FindResourceW(FFX.exe, block 0x784, RT_STRING): string 0x7836's
    // block (the uncompress error box's text)
    let res = exec
        .call_stdcall(import(&exec, k32, "FindResourceW"), &[base, 0x784, 6])
        .unwrap();
    assert!(rsrc(&exec).contains(&res), "{res:#x}");
    let data = exec
        .call_stdcall(import(&exec, k32, "LoadResource"), &[base, res])
        .unwrap();
    let locked = exec
        .call_stdcall(import(&exec, k32, "LockResource"), &[data])
        .unwrap();
    let size = exec
        .call_stdcall(import(&exec, k32, "SizeofResource"), &[base, res])
        .unwrap();
    assert_eq!(locked, data);
    assert!(rsrc(&exec).contains(&data) && rsrc(&exec).contains(&(data + size - 1)));
    // 16 length-prefixed strings fill the block exactly
    let mem = exec.mem();
    let mut at = data;
    let mut lengths = Vec::new();
    for _ in 0..16 {
        let units = u32::from(mem.read::<u16>(at));
        lengths.push(units);
        at += 2 + 2 * units;
    }
    assert_eq!(at, data + size, "string lengths {lengths:?}");
    // 0x7836 is the block's string 6: a BSTR of it, as the error box makes
    let text = (0..6).fold(data, |p, _| p + 2 + 2 * u32::from(mem.read::<u16>(p)));
    let units = u32::from(mem.read::<u16>(text));
    assert!(units > 0);
    // (the game's own loader, AtlFindStringResource_0061b270, walks the ATL
    // module list the C runtime's start-up fills; `boot` runs no start-up,
    // so it finds no module here: a run reaches it through this lookup)
    let ole = "OLEAUT32.dll";
    let bstr = exec
        .call_stdcall(import(&exec, ole, "ORDINAL 4"), &[text + 2, units])
        .unwrap();
    assert_ne!(bstr, 0);
    let mem = exec.mem();
    assert_eq!(mem.read::<u32>(bstr - 4), 2 * units);
    let mut want = vec![0u8; 2 * units as usize];
    mem.read_bytes(text + 2, &mut want);
    let mut got = vec![0u8; 2 * units as usize];
    mem.read_bytes(bstr, &mut got);
    assert_eq!(got, want);
    assert_eq!(mem.read::<u16>(bstr + 2 * units), 0);
    // SysAllocString of it: the same units, its length from the NUL
    let copy = exec
        .call_stdcall(import(&exec, ole, "ORDINAL 2"), &[bstr])
        .unwrap();
    assert_eq!(exec.mem().read::<u32>(copy - 4), 2 * units);
    assert_eq!(
        exec.call_stdcall(import(&exec, ole, "ORDINAL 2"), &[0])
            .unwrap(),
        0
    );
    // SysFreeString(0) does nothing; a freed block is the next one of
    // its size
    let free = import(&exec, ole, "ORDINAL 6");
    exec.call_stdcall(free, &[0]).unwrap();
    exec.call_stdcall(free, &[copy]).unwrap();
    let again = exec
        .call_stdcall(import(&exec, ole, "ORDINAL 4"), &[0, units])
        .unwrap();
    assert_eq!(again, copy);
    assert!((0..units).all(|i| exec.mem().read::<u16>(again + 2 * i) == 0));
}

/// Code that calls `[slot]` (`call dword [slot]`).
fn call(code: &mut Vec<u8>, slot: u32) {
    code.extend_from_slice(&[0xff, 0x15]);
    code.extend_from_slice(&slot.to_le_bytes());
}

fn push(code: &mut Vec<u8>, v: u32) {
    code.push(0x68);
    code.extend_from_slice(&v.to_le_bytes());
}

fn slot(exec: &Exec, name: &str) -> u32 {
    exec.image()
        .imports
        .iter()
        .find(|i| i.dll.eq_ignore_ascii_case("KERNEL32.dll") && i.name == name)
        .unwrap_or_else(|| panic!("{name} not imported"))
        .slot
}

#[test]
fn terminate_thread_ends_waiting_spinning_suspended_and_own_threads() {
    let Some(exe) = exe() else { return };
    let mut exec = ffx_host::boot(&exe).unwrap();
    exec.set_budget(Duration::from_secs(60));
    let [create, wait, terminate, code_of, resume, event] = [
        "CreateThread",
        "WaitForSingleObject",
        "TerminateThread",
        "GetExitCodeThread",
        "ResumeThread",
        "CreateEventA",
    ]
    .map(|n| slot(&exec, n));
    let cell = exec.alloc_code(&[0; 0x40]).unwrap();
    // waits(event): WaitForSingleObject(event, INFINITE); ret 4 with 1
    // push INFINITE; push dword [esp+8] (the parameter, past the push)
    let mut r = vec![0x6a, 0xff, 0xff, 0x74, 0x24, 0x08];
    call(&mut r, wait);
    r.extend_from_slice(&[0xb8, 1, 0, 0, 0, 0xc2, 0x04, 0x00]);
    let waits = exec.alloc_code(&r).unwrap();
    // spins: jmp $
    let spins = exec.alloc_code(&[0xeb, 0xfe]).unwrap();
    // ends_itself: TerminateThread(GetCurrentThread() = -2, 0x66); then
    // a word that shows it went on
    let mut r = Vec::new();
    push(&mut r, 0x66);
    push(&mut r, 0xffff_fffe);
    call(&mut r, terminate);
    r.extend_from_slice(&[0xc7, 0x05]); // mov dword [cell + 0x20], 1
    r.extend_from_slice(&(cell + 0x20).to_le_bytes());
    r.extend_from_slice(&1u32.to_le_bytes());
    r.extend_from_slice(&[0x33, 0xc0, 0xc2, 0x04, 0x00]);
    let ends_itself = exec.alloc_code(&r).unwrap();
    // main: an event; four threads (the last suspended); each but the
    // self-ending one terminated with its own code; every handle waited
    // for; the exit codes into cell + 0..
    let mut m = vec![0x56, 0x57, 0x53]; // push esi; push edi; push ebx
    for v in [0, 0, 1, 0] {
        push(&mut m, v); // CreateEventA(0, manual, unset, 0): name, initial, manual, attributes pushed
    }
    call(&mut m, event);
    m.extend_from_slice(&[0x8b, 0xd8]); // mov ebx, eax
    let mut handles = Vec::new();
    for (i, (routine, flags)) in [(waits, 0u32), (spins, 0), (ends_itself, 0), (spins, 4)]
        .into_iter()
        .enumerate()
    {
        push(&mut m, 0);
        push(&mut m, flags);
        m.push(0x53); // push ebx (the parameter)
        push(&mut m, routine);
        push(&mut m, 0);
        push(&mut m, 0);
        call(&mut m, create);
        let at = cell + 0x10 + 4 * u32::try_from(i).unwrap();
        m.push(0xa3); // mov [at], eax
        m.extend_from_slice(&at.to_le_bytes());
        handles.push(at);
    }
    // let the running ones get going (WaitForSingleObject(ebx, 50))
    push(&mut m, 50);
    m.push(0x53);
    call(&mut m, wait);
    for (i, &h) in handles.iter().enumerate() {
        if i == 2 {
            continue;
        }
        push(&mut m, 0x50 + u32::try_from(i).unwrap());
        m.extend_from_slice(&[0xff, 0x35]); // push dword [h]
        m.extend_from_slice(&h.to_le_bytes());
        call(&mut m, terminate);
    }
    // the suspended one, resumed now, never runs
    m.extend_from_slice(&[0xff, 0x35]);
    m.extend_from_slice(&handles[3].to_le_bytes());
    call(&mut m, resume);
    for (i, &h) in handles.iter().enumerate() {
        push(&mut m, 0xffff_ffff);
        m.extend_from_slice(&[0xff, 0x35]);
        m.extend_from_slice(&h.to_le_bytes());
        call(&mut m, wait);
        push(&mut m, cell + 4 * u32::try_from(i).unwrap());
        m.extend_from_slice(&[0xff, 0x35]);
        m.extend_from_slice(&h.to_le_bytes());
        call(&mut m, code_of);
    }
    m.extend_from_slice(&[0x5b, 0x5f, 0x5e, 0x33, 0xc0, 0xc3]);
    let main = exec.alloc_code(&m).unwrap();
    exec.call_cdecl(main, &[]).unwrap();
    let mem = exec.mem();
    let codes: [u32; 4] = mem.read(cell);
    assert_eq!(codes, [0x50, 0x51, 0x66, 0x53]);
    assert_eq!(
        mem.read::<u32>(cell + 0x20),
        0,
        "ran on after ending itself"
    );
    // a handle that is no thread
    assert_eq!(
        exec.call_stdcall(exec.mem().read(terminate), &[0x1234_5678, 0])
            .unwrap(),
        0
    );
}
