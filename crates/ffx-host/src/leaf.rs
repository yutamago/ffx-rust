//! `FFX.exe`'s one-instruction functions, run in Rust when a port calls
//! one (docs/host.md, "Leaf functions"). The PS2 leftovers and empty
//! debug hooks (`FlushCache`, `sceDmaSend`, `sceGsSyncPath`,
//! `FFXVpu0Reset`, `MyProfilerAddMarker`, ...) and the accessors
//! (`CharacterTextureAnimationTweakUtil_GetInstance`: `mov eax, [g]`)
//! are a `ret` and at most one instruction before it; gate run 1 entered
//! Unicorn about 1.5 M times for them from ports. Each shape here is
//! exactly what the instruction does to `eax`; a call from Unicorn code
//! still runs the original (one block, no emulator stop).

use ffx_bridge::Mem;

use crate::table::Ret;

/// What a leaf leaves in `eax`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Leaf {
    /// `ret` / `ret n`: nothing (the caller's `eax`).
    Void,
    /// `xor eax, eax`, `or eax, -1`, `mov eax, imm32`, `mov al, imm8`
    /// (the upper bytes the caller's: 0 from Rust): a constant.
    Const(u32),
    /// `mov eax, [imm32]`: a global.
    Global(u32),
    /// `mov eax, [ecx + d8]`: a word of `this` (`d8` sign-extended).
    ThisWord(i8),
    /// `mov al, [ecx + d8]`: a byte of `this` (upper bytes 0 from Rust).
    ThisByte(i8),
    /// `mov eax, ecx`, `lea eax, [ecx + d8]`, `lea eax, [ecx + d32]`:
    /// `this` plus an offset (a member's address; m5-wp5e).
    ThisPlus(i32),
    /// `push ebp; mov ebp, esp; mov eax, [ebp+8]; (add eax, imm8 /
    /// imm32;) pop ebp; ret (n)`: the first stack word plus an offset
    /// (about 370 accessors in `FFX.exe`; m5-wp5e).
    ArgPlus(i32),
}

/// The leaf at `addr`, if its code is one of [`Leaf`]'s shapes.
#[must_use]
pub fn decode(mem: &impl Mem, addr: u32) -> Option<Leaf> {
    let mut b = [0u8; 8];
    mem.read_bytes(addr, &mut b);
    let imm = |i: usize| u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
    // `ret` or `ret n` after `len` bytes
    let ret_at = |i: usize| b[i] == 0xc3 || b[i] == 0xc2;
    Some(match b {
        [0xc3 | 0xc2, ..] => Leaf::Void,
        [0x33, 0xc0, ..] if ret_at(2) => Leaf::Const(0),
        [0x83, 0xc8, 0xff, ..] if ret_at(3) => Leaf::Const(u32::MAX),
        [0xb0, v, ..] if ret_at(2) => Leaf::Const(u32::from(v)),
        [0xb8, ..] if ret_at(5) => Leaf::Const(imm(1)),
        [0xa1, ..] if ret_at(5) => Leaf::Global(imm(1)),
        [0x8b, 0x01, ..] if ret_at(2) => Leaf::ThisWord(0),
        [0x8b, 0x41, d, ..] if ret_at(3) => Leaf::ThisWord(d.cast_signed()),
        [0x8a, 0x01, ..] if ret_at(2) => Leaf::ThisByte(0),
        [0x8a, 0x41, d, ..] if ret_at(3) => Leaf::ThisByte(d.cast_signed()),
        [0x8b, 0xc1, ..] if ret_at(2) => Leaf::ThisPlus(0),
        [0x8d, 0x41, d, ..] if ret_at(3) => Leaf::ThisPlus(i32::from(d.cast_signed())),
        [0x8d, 0x81, ..] if ret_at(6) => Leaf::ThisPlus(imm(2).cast_signed()),
        _ => return arg_plus(mem, addr),
    })
}

/// [`Leaf::ArgPlus`]'s shapes, after the frame's `push ebp; mov ebp,
/// esp; mov eax, [ebp+8]`.
fn arg_plus(mem: &impl Mem, addr: u32) -> Option<Leaf> {
    let mut b = [0u8; 16];
    mem.read_bytes(addr, &mut b);
    if b[..6] != [0x55, 0x8b, 0xec, 0x8b, 0x45, 0x08] {
        return None;
    }
    let ret_at = |i: usize| b[i] == 0x5d && (b[i + 1] == 0xc3 || b[i + 1] == 0xc2);
    Some(match b[6..] {
        [0x5d, ..] if ret_at(6) => Leaf::ArgPlus(0),
        [0x83, 0xc0, d, ..] if ret_at(9) => Leaf::ArgPlus(i32::from(d.cast_signed())),
        [0x05, a, b1, c, d, ..] if ret_at(11) => Leaf::ArgPlus(i32::from_le_bytes([a, b1, c, d])),
        _ => return None,
    })
}

/// The target `X` of a store wrapper, `push ebp; mov ebp, esp; push
/// [ebp+8]; call X; mov ecx, [ebp+0xc]; add esp, 4; mov [ecx], eax; pop
/// ebp; ret 8`: `*out = X(a)`, `X`'s `eax` (the Phyre class descriptors'
/// `getPointerFromScriptContext`, about 410 of them; m5-wp5e). A call
/// from Rust calls `X` (cdecl, one word) and stores its `eax`.
#[must_use]
pub fn store_wrapper(mem: &impl Mem, addr: u32) -> Option<u32> {
    let mut b = [0u8; 23];
    mem.read_bytes(addr, &mut b);
    let shape = b[..8] == [0x55, 0x8b, 0xec, 0xff, 0x75, 0x08, 0xe8, b[7]]
        && b[11..]
            == [
                0x8b, 0x4d, 0x0c, 0x83, 0xc4, 0x04, 0x89, 0x01, 0x5d, 0xc2, 0x08, 0x00,
            ];
    shape.then(|| {
        let rel = i32::from_le_bytes([b[7], b[8], b[9], b[10]]);
        addr.wrapping_add(11).wrapping_add_signed(rel)
    })
}

/// Runs `leaf` for a call with `ecx` and the first stack word `arg0` (0
/// when the caller passed none): what the original returns.
#[must_use]
pub fn run(mem: &impl Mem, leaf: Leaf, ecx: u32, arg0: u32) -> Ret {
    match leaf {
        Leaf::Void => Ret::VOID,
        Leaf::Const(v) => Ret::eax(v),
        Leaf::Global(at) => Ret::eax(mem.read(at)),
        Leaf::ThisWord(d) => Ret::eax(mem.read(ecx.wrapping_add_signed(i32::from(d)))),
        Leaf::ThisByte(d) => Ret::eax(u32::from(
            mem.read::<u8>(ecx.wrapping_add_signed(i32::from(d))),
        )),
        Leaf::ThisPlus(d) => Ret::eax(ecx.wrapping_add_signed(d)),
        Leaf::ArgPlus(d) => Ret::eax(arg0.wrapping_add_signed(d)),
    }
}
