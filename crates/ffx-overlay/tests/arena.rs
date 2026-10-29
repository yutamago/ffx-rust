//! An overlay in a 4 GiB arena, as `ffx-host` will run it: the player's
//! `magic_*.dll` loaded as data, `DllMain`, `InitMagicPRX` with an import
//! table whose every slot traps into the test's dispatcher, and the
//! `GetEffectOverlayTable` entries. Needs `FFX_GAME_DIR`.

use std::path::PathBuf;

use ffx_overlay::{Config, Host, Overlay, OverlayFn, Pe, Regs};

const SERVICE: u32 = 0x7fff_0000;
/// The stand-in `g_MagicImports`: 744 words; each function slot holds
/// `TRAPS + 16 * slot`, which no code is mapped at.
const TABLE: u32 = 0x0080_0000;
const SLOTS: u32 = 744;
const TRAPS: u32 = 0x0090_0000;
/// The three data slots' targets (`+0xb94`, `+0xb98`, `+0xb9c`).
const DATA: [u32; 3] = [0x00a0_0000, 0x00a1_0000, 0x00a2_0000];
const HEAP: u32 = 0x2000_0000;
const STACK_TOP: u32 = 0x3000_0000;

struct Arena {
    base: *mut u8,
    next: u32,
    calls: Vec<(u32, Option<u32>)>,
}

impl Arena {
    fn word(&self, at: u32) -> u32 {
        // SAFETY: inside the 4 GiB arena.
        unsafe { self.base.add(at as usize).cast::<u32>().read_unaligned() }
    }

    fn set(&mut self, at: u32, v: u32) {
        // SAFETY: inside the 4 GiB arena.
        unsafe { self.base.add(at as usize).cast::<u32>().write_unaligned(v) }
    }

    fn bytes(&mut self, at: u32, b: &[u8]) {
        // SAFETY: inside the 4 GiB arena.
        unsafe { std::ptr::copy_nonoverlapping(b.as_ptr(), self.base.add(at as usize), b.len()) }
    }
}

impl Host for Arena {
    fn call_out(&mut self, target: u32, slot: Option<u32>, regs: &mut Regs) -> Result<(), String> {
        self.calls.push((target, slot));
        // every slot answers 0, cdecl
        regs.eax = 0;
        regs.esp += 4;
        Ok(())
    }

    fn import(&mut self, name: &str, regs: &mut Regs) -> Option<Result<(), String>> {
        // the game's MSVCR110 would answer these; a fixed value is enough
        // for start-up
        match name {
            "rand" => {
                regs.eax = 0x1234;
                regs.esp += 4;
                Some(Ok(()))
            }
            _ => None,
        }
    }

    fn alloc(&mut self, size: u32) -> Option<u32> {
        let at = self.next;
        self.next = (self.next + size + 15) & !15;
        Some(at)
    }

    fn free(&mut self, _addr: u32) {}
}

fn arena() -> Option<(Arena, PathBuf)> {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return None;
    };
    let layout = std::alloc::Layout::from_size_align(1 << 32, 1 << 16).unwrap();
    // SAFETY: a non-zero layout; the pages stay untouched until used.
    let base = unsafe { std::alloc::alloc_zeroed(layout) };
    assert!(!base.is_null());
    ffx_overlay::configure(Config {
        base: base as usize,
        service: SERVICE,
        import_table: Some((TABLE, SLOTS)),
    });
    let mut a = Arena {
        base,
        next: HEAP,
        calls: Vec::new(),
    };
    for i in 0..SLOTS - 3 {
        a.set(TABLE + 4 * i, TRAPS + 16 * i);
    }
    for (i, d) in (SLOTS - 3..).zip(DATA) {
        a.set(TABLE + 4 * i, d);
    }
    // +0xb98 is read through once more (the overlay keeps `*[+0xb98]`)
    a.set(DATA[1], 0x00a1_1000);
    Some((a, PathBuf::from(dir).join("magicFiles/FFX")))
}

fn load(a: &mut Arena, dir: &std::path::Path, name: &str, base: u32) -> Overlay {
    let file = std::fs::read(dir.join(name)).unwrap();
    let pe = Pe::parse(&file).unwrap();
    assert!(pe.span() <= 0x0800_0000);
    let _ = a;
    Overlay::load(name, &file, &pe, base).unwrap()
}

#[test]
fn overlays_start_and_hand_out_their_tables() {
    let Some((mut a, dir)) = arena() else { return };
    let mut bases = 0x1000_0000;
    for name in ["magic_0003.dll", "magic_0150.dll", "magic_0481.dll"] {
        let ov = load(&mut a, &dir, name, bases);
        bases += ov.span();
        let base = ov.base();
        a.bytes(STACK_TOP - 0x100, &[0; 0x100]);
        // DllMain(base, DLL_PROCESS_ATTACH, 0): stdcall, TRUE
        let sp = STACK_TOP - 16;
        a.set(sp, SERVICE + 0xfff0);
        a.set(sp + 4, base);
        a.set(sp + 8, 1);
        a.set(sp + 12, 0);
        let mut stops = 0;
        let r = ffx_overlay::call_with(&mut a, ov.entry_point(), &Regs::at(sp), |_| stops += 1)
            .unwrap();
        // the CRT start-up's imports, then the return
        assert!(stops > 1, "{name}: {stops} stops");
        assert_eq!(r.eax, 1, "{name}: DllMain");
        a.calls.clear();
        // InitMagicPRX(&g_MagicImports): the overlay keeps the table
        let init = ov.export("InitMagicPRX").unwrap();
        OverlayFn(init)
            .call(&mut a, STACK_TOP, 0, &[TABLE])
            .unwrap();
        // GetEffectOverlayTable(0): a table of the overlay's functions
        let get = ov.export("GetEffectOverlayTable").unwrap();
        let t = OverlayFn(get).call(&mut a, STACK_TOP, 0, &[0]).unwrap().eax;
        assert!(t >= base && t < base + ov.span(), "{name}: table {t:#x}");
        let entries = ov.table_entries(t).unwrap();
        assert!(entries.len() >= 4, "{name}: {} entries", entries.len());
        // every call-out went through a slot of the table
        assert!(a.calls.iter().all(|&(t, s)| s == Some((t - TRAPS) / 16)));
        eprintln!(
            "{name}: base {base:#x}, table {t:#x}, {} entries, {} call-outs in InitMagicPRX",
            entries.len(),
            a.calls.len()
        );
        assert_eq!(a.word(TABLE), TRAPS);
        // DllMain(base, DLL_PROCESS_DETACH, 0)
        let r = OverlayFn(ov.entry_point())
            .call(&mut a, STACK_TOP, 0, &[base, 0, 0])
            .unwrap();
        assert_eq!(r.eax, 1, "{name}: DllMain detach");
        ov.unload();
    }
}
