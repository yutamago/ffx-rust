//! The runtime every overlay links (docs/formats/magic.md, "Porting the
//! overlays"): the MSVC 11 DLL start-up, `InitMagicPRX`,
//! `GetEffectOverlayTable` and the slot thunks. In 100 to 581 DLLs each,
//! the same code with the DLL's own global addresses, which the ports
//! read from the instructions' operands.

use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `g_MagicImports`'s address, as `InitMagicPRX` stored it.
#[must_use]
pub fn table_global(img: &Image) -> Option<u32> {
    // InitMagicPRX +0x2f: mov ds:G, eax (the table argument)
    img.export("InitMagicPRX").map(|f| img.word(f + 0x30))
}

/// `[[G] + 4 * slot]`: the function a slot thunk jumps to, read through
/// the global as the thunk reads it.
pub fn slot_target<G: Game>(g: &G, table_global: u32, slot: u32) -> u32 {
    let table = g.read::<u32>(table_global);
    g.read::<u32>(table.wrapping_add(4 * slot))
}

/// `_DllMainCRTStartup(hinst, reason, reserved)`, stdcall: the CRT's
/// `__security_init_cookie` (attach), `__DllMainCRTStartup` with
/// `_CRT_INIT` (the `_onexit` table, `_initterm_e`/`_initterm` over the
/// CRT's own initialisers, the `atexit` list at detach) and the DLL's
/// `DllMain` (`DisableThreadLibraryCalls`, TRUE).
///
/// The port keeps none of that state: it is the CRT's private bookkeeping
/// (the cookie, the `onexit` list, the attach count), which no overlay
/// function the game reaches reads (the parity test checks that the words
/// the original writes are read by no ported function). The initialiser
/// tables hold only the CRT's own functions in the DLLs this binds in
/// (`runtime.rs`'s test lists them). TRUE for every reason.
pub fn dll_main<G: Game>(_g: &mut G, _img: &Image, _f: &Frame) -> Ret {
    Ret {
        eax: 1,
        pops: 12,
        ..Ret::default()
    }
}

/// `_DllMainCRTStartup`: `push ebp; mov ebp, esp; cmp [ebp+0xc], 1;
/// jne; call __security_init_cookie; push x3; call __DllMainCRTStartup;
/// add esp, 0xc; pop ebp; ret 0xc`.
pub const DLL_MAIN: Sig = Sig {
    len: 35,
    hash: 0x2004_0159_9bc6_33c8,
    calls: &[(0xa, Callee::Any), (0x18, Callee::Any)],
    imports: &[],
};

/// `GetEffectOverlayTable(which)`: the overlay table for 0, else 0.
pub fn get_effect_overlay_table<G: Game>(g: &mut G, img: &Image, f: &Frame) -> Ret {
    let which = f.arg(g, 0);
    // +0x8: mov eax, TABLE
    let table = img.word(img.export("GetEffectOverlayTable").unwrap_or(0) + 9);
    // xor ecx, ecx; cmp [ebp+8], ecx; cmovne eax, ecx: ecx is 0 on the way out
    Ret::eax(if which == 0 { table } else { 0 })
}

/// `push ebp; mov ebp, esp; xor ecx, ecx; cmp [ebp+8], ecx; mov eax,
/// TABLE; cmovne eax, ecx; pop ebp; ret`.
pub const GET_EFFECT_OVERLAY_TABLE: Sig = Sig {
    len: 18,
    hash: 0xb8ed_330b_f27b_c0dc,
    calls: &[],
    imports: &[],
};

/// `InitMagicPRX`'s copies: (offset in `g_MagicImports`, offset of the
/// destination global's operand in the function). `TABLE` is the table's
/// own address; `DEREF` a word read through the slot (`*[table +
/// 0xb98]`).
const TABLE: u32 = u32::MAX;
const DEREF: u32 = 1 << 31;
const INIT_STORES: [(u32, u32); 33] = [
    (0x0b94, 0x010),
    (TABLE, 0x030),
    (DEREF | 0x0b98, 0x036),
    (0x0ba0, 0x09d),
    (0x0ba4, 0x0a9),
    (0x0ba8, 0x0b5),
    (0x0bb4, 0x0c1),
    (0x0bac, 0x0cd),
    (0x0bb0, 0x0d9),
    (0x0bb8, 0x0e5),
    (0x0bbc, 0x0f1),
    (0x0bc0, 0x0fd),
    (0x0dc0, 0x109),
    (0x0bc4, 0x115),
    (0x0bc8, 0x121),
    (0x0bcc, 0x12d),
    (0x0bd0, 0x139),
    (0x0bd4, 0x145),
    (0x0bd8, 0x151),
    (0x0bdc, 0x15d),
    (0x0be0, 0x169),
    (0x0be4, 0x175),
    (0x0be8, 0x181),
    (0x0bec, 0x18d),
    (0x0bf0, 0x199),
    (0x0bf4, 0x1a5),
    (0x0c04, 0x1b1),
    (0x0cec, 0x1bd),
    (0x0d48, 0x1c9),
    (0x0e5c, 0x1d5),
    (0x0e64, 0x1e2),
    (0x0e9c, 0x1f5),
    (0x0ea0, 0x1fa),
];

/// `InitMagicPRX(table)`: keeps the table and the words of it the DLL
/// uses as data (pointers to game variables) in its globals, and copies
/// 16 words of `*[table + 0xb9c]` (`g_OpusBattle`'s first 64 bytes) into
/// a 16-word global. Returns the last word it loaded (`[table + 0xea0]`
/// is left in eax).
pub fn init_magic_prx<G: Game>(g: &mut G, img: &Image, f: &Frame) -> Ret {
    let table = f.arg(g, 0);
    let at = img.export("InitMagicPRX").unwrap_or(0);
    let mut eax = 0;
    for (k, &(off, op)) in INIT_STORES.iter().enumerate() {
        let v = if off == TABLE {
            table
        } else if off & DEREF != 0 {
            let p = g.read::<u32>(table.wrapping_add(off & !DEREF));
            g.read::<u32>(p)
        } else {
            g.read::<u32>(table.wrapping_add(off))
        };
        g.write::<u32>(img.word(at + op), v);
        if k == 2 {
            // the 16-word copy runs after the third store: mov [esi + G], ecx
            let dst = img.word(at + 0x51);
            for i in 0..16 {
                let src = g.read::<u32>(table.wrapping_add(0xb9c));
                let w = g.read::<u32>(src.wrapping_add(4 * i));
                g.write::<u32>(dst.wrapping_add(4 * i), w);
            }
        }
        eax = v;
    }
    Ret::eax(eax)
}

/// `InitMagicPRX` (538 bytes).
pub const INIT_MAGIC_PRX: Sig = Sig {
    len: 513,
    hash: 0xa5e0_4d82_b122_3da5,
    calls: &[],
    imports: &[],
};

/// A slot thunk: `push ebp; mov ebp, esp; mov eax, [G]; mov eax, [eax +
/// 4 * slot]; pop ebp; jmp eax` (17 bytes), or the same without the frame
/// (`mov eax, [G]; mov eax, [eax + 4 * slot]; jmp eax`, 13 bytes;
/// `magic_0275`, `magic_0297` and others). The game stores these as the
/// particle programs' step functions; the overlay calls them for its slot
/// calls. The port jumps to the slot's function with the caller's stack
/// as it is (eax holds the target, as in the original).
pub fn slot_thunk<G: Game>(g: &mut G, img: &Image, f: &Frame) -> Ret {
    let table = table_global(img).unwrap_or(0);
    let slot = slot_of_thunk(img, f.at, table).unwrap_or(0);
    Ret::jump(slot_target(g, table, slot))
}

/// The thunk's signature (the slot offset at +9 is an immediate and part
/// of the bytes, so each slot is its own signature: [`slot_of_thunk`]
/// checks the shape instead).
pub const SLOT_THUNK: Sig = Sig {
    len: 17,
    hash: 0,
    calls: &[],
    imports: &[],
};

/// The slot a thunk at `addr` jumps through, when the code there is a
/// slot thunk over `table_global` (either shape of [`slot_thunk`]).
#[must_use]
pub fn slot_of_thunk(img: &Image, addr: u32, table_global: u32) -> Option<u32> {
    let b = |o: u32| img.word(addr + o).to_le_bytes()[0];
    // 55 8b ec a1 G 8b 80 off 5d ff e0
    let framed = [b(0), b(1), b(2), b(3)] == [0x55, 0x8b, 0xec, 0xa1]
        && img.word(addr + 4) == table_global
        && [b(8), b(9), b(14), b(15), b(16)] == [0x8b, 0x80, 0x5d, 0xff, 0xe0];
    // a1 G 8b 80 off ff e0
    let bare = b(0) == 0xa1
        && img.word(addr + 1) == table_global
        && [b(5), b(6), b(11), b(12)] == [0x8b, 0x80, 0xff, 0xe0];
    let off = if framed {
        img.word(addr + 10)
    } else if bare {
        img.word(addr + 7)
    } else {
        return None;
    };
    (off.is_multiple_of(4) && off < 0x1000).then_some(off / 4)
}

/// `xor eax, eax; ret` (402 overlays): returns 0.
pub fn zero_ret<G: Game>(_g: &mut G, _img: &Image, _fr: &Frame) -> Ret {
    Ret::eax(0)
}

/// `xor eax, eax; ret`.
pub const ZERO_RET: Sig = Sig {
    len: 3,
    hash: 0x5838_9c18_2330_78ff,
    calls: &[],
    imports: &[],
};
