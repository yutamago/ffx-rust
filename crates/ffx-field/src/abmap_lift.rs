//! The ability map's float-heavy draws, lifted instruction by instruction
//! from `FFX.exe` into state machines over an emulated frame (field agent
//! r3): exact, not yet rewritten by hand.

use ffx_bridge::Mem;

use crate::lift::{Regs, fcom, ld32, set, st0, sti};

/// The emulated frame's size (`work`) of each function here.
pub const WORK: u32 = 0x400;

/// `AbmapDrawGridMarks()` (`0x00a49f10`, name ours; from the ability map's
/// draw `0x00a54660`): the marks of the ability map's 17 x 20 grid
/// (`g_Abmap + 0x12e60`, one byte a cell): a GS sprite packet per set cell
/// at `0x01a860ec`, then the pulsing frame (`g_Abmap + 0x11698`, 1/256 a
/// step) handed to `graphicDrawUIAbmapElement` twice. Lifted instruction
/// by instruction (state machine over an emulated frame, [`WORK`] bytes);
/// returns the original's `eax`.
#[allow(
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::float_cmp,
    clippy::cast_possible_truncation,
    clippy::unreadable_literal,
    clippy::match_same_arms,
    clippy::needless_late_init,
    clippy::eq_op,
    clippy::nonminimal_bool,
    clippy::if_not_else,
    unused_braces,
    unused_assignments
)]
pub fn abmap_draw_grid_marks<M, C>(mem: &mut M, work: u32, call: &mut C) -> u32
where
    M: Mem,
    C: FnMut(&mut M, u32, &[u32]) -> u32,
{
    let mut r = Regs {
        ebp: work.wrapping_add(0x2c0),
        esp: work.wrapping_add(0x2c0 - 0x258),
        cw: 0x027f,
        ..Regs::default()
    };
    let mut st: Vec<f64> = Vec::with_capacity(8);
    let mut pc: u32 = 0x00a4_9f2c;
    loop {
        match pc {
            0xa49f2c => {
                // 0xa49f2c: mov eax,ds:0xc613d8
                r.eax = mem.read::<u32>(mem.rebase(0xc613d8));
                // 0xa49f31: xor eax,ebp
                {
                    let v = r.eax ^ r.ebp;
                    r.eax = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0xa49f33: mov DWORD PTR [ebp-0x4],eax
                mem.write(r.ebp.wrapping_sub(0x4), r.eax);
                // 0xa49f36: push esi
                {
                    let v = r.esi;
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0xa49f37: push edi
                {
                    let v = r.edi;
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0xa49f38: mov eax,ds:0x2305834
                r.eax = mem.read::<u32>(mem.rebase(0x2305834));
                // 0xa49f3d: mov DWORD PTR [ebp-0x110],0x0
                mem.write(r.ebp.wrapping_sub(0x110), 0x0u32);
                // 0xa49f47: lea ecx,[eax+0x12e60]
                r.ecx = r.eax.wrapping_add(0x12e60);
                // 0xa49f4d: mov DWORD PTR [ebp-0x10c],0x0
                mem.write(r.ebp.wrapping_sub(0x10c), 0x0u32);
                // 0xa49f57: mov eax,DWORD PTR [eax+0x11698]
                r.eax = mem.read::<u32>(r.eax.wrapping_add(0x11698));
                // 0xa49f5d: and eax,0x1ff
                {
                    let v = r.eax & 0x1ffu32;
                    r.eax = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0xa49f62: mov DWORD PTR [ebp-0x1a8],eax
                mem.write(r.ebp.wrapping_sub(0x1a8), r.eax);
                // 0xa49f68: fild DWORD PTR [ebp-0x1a8]
                st.push(f64::from(mem.read::<i32>(r.ebp.wrapping_sub(0x1a8))));
                // 0xa49f6e: mov DWORD PTR [ebp-0x244],0x40
                mem.write(r.ebp.wrapping_sub(0x244), 0x40u32);
                // 0xa49f78: mov DWORD PTR [ebp-0x248],0x40
                mem.write(r.ebp.wrapping_sub(0x248), 0x40u32);
                // 0xa49f82: mov DWORD PTR [ebp-0x24c],0x40
                mem.write(r.ebp.wrapping_sub(0x24c), 0x40u32);
                // 0xa49f8c: mov DWORD PTR [ebp-0x250],0x40
                mem.write(r.ebp.wrapping_sub(0x250), 0x40u32);
                // 0xa49f96: fstp DWORD PTR [ebp-0x1a8]
                mem.write(r.ebp.wrapping_sub(0x1a8), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa49f9c: fld DWORD PTR [ebp-0x1a8]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x1a8)));
                // 0xa49fa2: fmul QWORD PTR ds:0xb50198
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * mem.read::<f64>(mem.rebase(0xb50198)));
                }
                // 0xa49fa8: fld1
                st.push(1.0);
                // 0xa49faa: fsub st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, x - y);
                }
                // 0xa49fac: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa49fae: fstp DWORD PTR [ebp-0x1b8]
                mem.write(r.ebp.wrapping_sub(0x1b8), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa49fb4: fld DWORD PTR [ebp-0x1b8]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x1b8)));
                // 0xa49fba: fst DWORD PTR [ebp-0x240]
                mem.write(r.ebp.wrapping_sub(0x240), crate::x87::store(st0(&st)));
                // 0xa49fc0: fst DWORD PTR [ebp-0x23c]
                mem.write(r.ebp.wrapping_sub(0x23c), crate::x87::store(st0(&st)));
                // 0xa49fc6: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa49fc8: fld QWORD PTR ds:0xb92270
                st.push(mem.read::<f64>(mem.rebase(0xb92270)));
                // 0xa49fce: fadd st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, x + y);
                }
                // 0xa49fd0: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa49fd2: fstp DWORD PTR [ebp-0x1a8]
                mem.write(r.ebp.wrapping_sub(0x1a8), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa49fd8: fld DWORD PTR [ebp-0x1a8]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x1a8)));
                // 0xa49fde: fst DWORD PTR [ebp-0x230]
                mem.write(r.ebp.wrapping_sub(0x230), crate::x87::store(st0(&st)));
                // 0xa49fe4: fst DWORD PTR [ebp-0x21c]
                mem.write(r.ebp.wrapping_sub(0x21c), crate::x87::store(st0(&st)));
                // 0xa49fea: fst DWORD PTR [ebp-0x210]
                mem.write(r.ebp.wrapping_sub(0x210), crate::x87::store(st0(&st)));
                // 0xa49ff0: fstp DWORD PTR [ebp-0x20c]
                mem.write(r.ebp.wrapping_sub(0x20c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa49ff6: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa49ff8: fst DWORD PTR [ebp-0x22c]
                mem.write(r.ebp.wrapping_sub(0x22c), crate::x87::store(st0(&st)));
                // 0xa49ffe: fst DWORD PTR [ebp-0x220]
                mem.write(r.ebp.wrapping_sub(0x220), crate::x87::store(st0(&st)));
                // 0xa4a004: fld1
                st.push(1.0);
                // 0xa4a006: fst DWORD PTR [ebp-0x208]
                mem.write(r.ebp.wrapping_sub(0x208), crate::x87::store(st0(&st)));
                // 0xa4a00c: fst DWORD PTR [ebp-0x218]
                mem.write(r.ebp.wrapping_sub(0x218), crate::x87::store(st0(&st)));
                // 0xa4a012: fst DWORD PTR [ebp-0x228]
                mem.write(r.ebp.wrapping_sub(0x228), crate::x87::store(st0(&st)));
                // 0xa4a018: fst DWORD PTR [ebp-0x238]
                mem.write(r.ebp.wrapping_sub(0x238), crate::x87::store(st0(&st)));
                // 0xa4a01e: fldz
                st.push(0.0);
                // 0xa4a020: fst DWORD PTR [ebp-0x204]
                mem.write(r.ebp.wrapping_sub(0x204), crate::x87::store(st0(&st)));
                // 0xa4a026: fst DWORD PTR [ebp-0x214]
                mem.write(r.ebp.wrapping_sub(0x214), crate::x87::store(st0(&st)));
                // 0xa4a02c: fst DWORD PTR [ebp-0x224]
                mem.write(r.ebp.wrapping_sub(0x224), crate::x87::store(st0(&st)));
                // 0xa4a032: fst DWORD PTR [ebp-0x234]
                mem.write(r.ebp.wrapping_sub(0x234), crate::x87::store(st0(&st)));
                // 0xa4a038: fld st(2)
                {
                    let x = sti(&st, 2);
                    st.push(x);
                }
                // 0xa4a03a: fsubp st(5),st
                {
                    let (x, y) = (sti(&st, 5), sti(&st, 0));
                    set(&mut st, 5, x - y);
                }
                st.pop();
                // 0xa4a03c: fxch st(4)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 4);
                }
                // 0xa4a03e: fstp DWORD PTR [ebp-0x1a8]
                mem.write(r.ebp.wrapping_sub(0x1a8), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a044: fld DWORD PTR [ebp-0x1a8]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x1a8)));
                // 0xa4a04a: fst DWORD PTR [ebp-0x1fc]
                mem.write(r.ebp.wrapping_sub(0x1fc), crate::x87::store(st0(&st)));
                // 0xa4a050: fstp DWORD PTR [ebp-0x200]
                mem.write(r.ebp.wrapping_sub(0x200), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a056: fld DWORD PTR [ebp-0x1fc]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x1fc)));
                // 0xa4a05c: fadd QWORD PTR ds:0xb92220
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + mem.read::<f64>(mem.rebase(0xb92220)));
                }
                // 0xa4a062: fstp DWORD PTR [ebp-0x1fc]
                mem.write(r.ebp.wrapping_sub(0x1fc), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a068: fld DWORD PTR [ebp-0x200]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x200)));
                // 0xa4a06e: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4a070: fadd st,st(4)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 4));
                    set(&mut st, 0, x + y);
                }
                // 0xa4a072: fstp DWORD PTR [ebp-0x1a8]
                mem.write(r.ebp.wrapping_sub(0x1a8), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a078: fld DWORD PTR [ebp-0x1a8]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x1a8)));
                // 0xa4a07e: fst DWORD PTR [ebp-0x1f0]
                mem.write(r.ebp.wrapping_sub(0x1f0), crate::x87::store(st0(&st)));
                // 0xa4a084: fld DWORD PTR [ebp-0x1fc]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x1fc)));
                // 0xa4a08a: fst DWORD PTR [ebp-0x1ec]
                mem.write(r.ebp.wrapping_sub(0x1ec), crate::x87::store(st0(&st)));
                // 0xa4a090: fxch st(2)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 2);
                }
                // 0xa4a092: fstp DWORD PTR [ebp-0x1e0]
                mem.write(r.ebp.wrapping_sub(0x1e0), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a098: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4a09a: faddp st(4),st
                {
                    let (x, y) = (sti(&st, 4), sti(&st, 0));
                    set(&mut st, 4, x + y);
                }
                st.pop();
                // 0xa4a09c: fxch st(3)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 3);
                }
                // 0xa4a09e: fstp DWORD PTR [ebp-0x1a8]
                mem.write(r.ebp.wrapping_sub(0x1a8), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a0a4: fld DWORD PTR [ebp-0x1a8]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x1a8)));
                // 0xa4a0aa: fst DWORD PTR [ebp-0x1dc]
                mem.write(r.ebp.wrapping_sub(0x1dc), crate::x87::store(st0(&st)));
                // 0xa4a0b0: fstp DWORD PTR [ebp-0x1cc]
                mem.write(r.ebp.wrapping_sub(0x1cc), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a0b6: fxch st(2)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 2);
                }
                // 0xa4a0b8: fstp DWORD PTR [ebp-0x1d0]
                mem.write(r.ebp.wrapping_sub(0x1d0), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a0be: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4a0c0: fst DWORD PTR [ebp-0x1c8]
                mem.write(r.ebp.wrapping_sub(0x1c8), crate::x87::store(st0(&st)));
                // 0xa4a0c6: fst DWORD PTR [ebp-0x1d8]
                mem.write(r.ebp.wrapping_sub(0x1d8), crate::x87::store(st0(&st)));
                // 0xa4a0cc: fst DWORD PTR [ebp-0x1e8]
                mem.write(r.ebp.wrapping_sub(0x1e8), crate::x87::store(st0(&st)));
                // 0xa4a0d2: fstp DWORD PTR [ebp-0x1f8]
                mem.write(r.ebp.wrapping_sub(0x1f8), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a0d8: mov eax,ds:0x2305834
                r.eax = mem.read::<u32>(mem.rebase(0x2305834));
                // 0xa4a0dd: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4a0df: fst DWORD PTR [ebp-0x1c4]
                mem.write(r.ebp.wrapping_sub(0x1c4), crate::x87::store(st0(&st)));
                // 0xa4a0e5: add eax,0x1182c
                {
                    let v = r.eax.wrapping_add(0x1182cu32);
                    r.eax = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0xa4a0ea: fst DWORD PTR [ebp-0x1d4]
                mem.write(r.ebp.wrapping_sub(0x1d4), crate::x87::store(st0(&st)));
                // 0xa4a0f0: mov DWORD PTR [ebp-0x1ac],eax
                mem.write(r.ebp.wrapping_sub(0x1ac), r.eax);
                // 0xa4a0f6: fst DWORD PTR [ebp-0x1e4]
                mem.write(r.ebp.wrapping_sub(0x1e4), crate::x87::store(st0(&st)));
                // 0xa4a0fc: mov DWORD PTR [ebp-0x1a8],0x11
                mem.write(r.ebp.wrapping_sub(0x1a8), 0x11u32);
                // 0xa4a106: fstp DWORD PTR [ebp-0x1f4]
                mem.write(r.ebp.wrapping_sub(0x1f4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a10c: fld QWORD PTR ds:0xb921e0
                st.push(mem.read::<f64>(mem.rebase(0xb921e0)));
                // 0xa4a112: fld QWORD PTR ds:0xb51bc0
                st.push(mem.read::<f64>(mem.rebase(0xb51bc0)));
                // 0xa4a118: fld QWORD PTR ds:0xb922f0
                st.push(mem.read::<f64>(mem.rebase(0xb922f0)));
                // 0xa4a11e: fld QWORD PTR ds:0xb41d38
                st.push(mem.read::<f64>(mem.rebase(0xb41d38)));
                pc = 0xa4a124;
            }
            0xa4a124 => {
                // 0xa4a124: mov edx,eax
                r.edx = r.eax;
                // 0xa4a126: mov esi,0x14
                r.esi = 0x14u32;
                // 0xa4a12b: mov DWORD PTR [ebp-0x1b4],edx
                mem.write(r.ebp.wrapping_sub(0x1b4), r.edx);
                // 0xa4a131: mov DWORD PTR [ebp-0x1b0],esi
                mem.write(r.ebp.wrapping_sub(0x1b0), r.esi);
                pc = 0xa4a137;
            }
            0xa4a137 => {
                // 0xa4a137: mov al,BYTE PTR [ecx]
                r.eax = (r.eax & 0xffff_ff00) | ((u32::from(mem.read::<u8>(r.ecx))) & 0xff);
                // 0xa4a139: inc ecx
                {
                    let v = r.ecx.wrapping_add(1);
                    r.ecx = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0xa4a13a: mov DWORD PTR [ebp-0x254],ecx
                mem.write(r.ebp.wrapping_sub(0x254), r.ecx);
                // 0xa4a140: test al,al
                r.flag = ((r.eax & 0xff) & (r.eax & 0xff), 0, false);
                r.cmp = false;
                // 0xa4a142: je 0xa4aba1
                pc = if r.zf() { 0xa4aba1 } else { 0xa4a148 };
            }
            0xa4a148 => {
                // 0xa4a148: mov edi,DWORD PTR ds:0x1a860ec
                r.edi = mem.read::<u32>(mem.rebase(0x1a860ec));
                // 0xa4a14e: mov DWORD PTR [edi],0x4
                mem.write(r.edi, 0x4u32);
                // 0xa4a154: mov eax,ds:0x1a860ec
                r.eax = mem.read::<u32>(mem.rebase(0x1a860ec));
                // 0xa4a159: mov DWORD PTR [edi+0x8],0x512
                mem.write(r.edi.wrapping_add(0x8), 0x512u32);
                // 0xa4a160: mov DWORD PTR [eax+0x4],0x302a4000
                mem.write(r.eax.wrapping_add(0x4), 0x302a4000u32);
                // 0xa4a167: mov DWORD PTR [edi+0xc],0x0
                mem.write(r.edi.wrapping_add(0xc), 0x0u32);
                // 0xa4a16e: mov DWORD PTR [edi+0xd0],0x8004
                mem.write(r.edi.wrapping_add(0xd0), 0x8004u32);
                // 0xa4a178: mov DWORD PTR [edi+0xd4],0x302a4000
                mem.write(r.edi.wrapping_add(0xd4), 0x302a4000u32);
                // 0xa4a182: mov DWORD PTR [edi+0xd8],0x512
                mem.write(r.edi.wrapping_add(0xd8), 0x512u32);
                // 0xa4a18c: mov DWORD PTR [edi+0xdc],0x0
                mem.write(r.edi.wrapping_add(0xdc), 0x0u32);
                // 0xa4a196: mov eax,DWORD PTR [edx-0x16c]
                r.eax = mem.read::<u32>(r.edx.wrapping_sub(0x16c));
                // 0xa4a19c: mov DWORD PTR [edi+0x100],eax
                mem.write(r.edi.wrapping_add(0x100), r.eax);
                // 0xa4a1a2: mov DWORD PTR [edi+0x30],eax
                mem.write(r.edi.wrapping_add(0x30), r.eax);
                // 0xa4a1a5: mov eax,DWORD PTR [edx-0x168]
                r.eax = mem.read::<u32>(r.edx.wrapping_sub(0x168));
                // 0xa4a1ab: mov DWORD PTR [edi+0x104],eax
                mem.write(r.edi.wrapping_add(0x104), r.eax);
                // 0xa4a1b1: mov DWORD PTR [edi+0x34],eax
                mem.write(r.edi.wrapping_add(0x34), r.eax);
                // 0xa4a1b4: mov DWORD PTR [edi+0x108],0x2000
                mem.write(r.edi.wrapping_add(0x108), 0x2000u32);
                // 0xa4a1be: mov DWORD PTR [edi+0x38],0x2000
                mem.write(r.edi.wrapping_add(0x38), 0x2000u32);
                // 0xa4a1c5: mov DWORD PTR [edi+0x10c],0x0
                mem.write(r.edi.wrapping_add(0x10c), 0x0u32);
                // 0xa4a1cf: mov DWORD PTR [edi+0x3c],0x0
                mem.write(r.edi.wrapping_add(0x3c), 0x0u32);
                // 0xa4a1d6: mov eax,DWORD PTR [edx-0x15c]
                r.eax = mem.read::<u32>(r.edx.wrapping_sub(0x15c));
                // 0xa4a1dc: mov DWORD PTR [edi+0x130],eax
                mem.write(r.edi.wrapping_add(0x130), r.eax);
                // 0xa4a1e2: mov DWORD PTR [edi+0x60],eax
                mem.write(r.edi.wrapping_add(0x60), r.eax);
                // 0xa4a1e5: mov eax,DWORD PTR [edx-0x158]
                r.eax = mem.read::<u32>(r.edx.wrapping_sub(0x158));
                // 0xa4a1eb: mov DWORD PTR [edi+0x134],eax
                mem.write(r.edi.wrapping_add(0x134), r.eax);
                // 0xa4a1f1: mov DWORD PTR [edi+0x64],eax
                mem.write(r.edi.wrapping_add(0x64), r.eax);
                // 0xa4a1f4: mov DWORD PTR [edi+0x138],0x2000
                mem.write(r.edi.wrapping_add(0x138), 0x2000u32);
                // 0xa4a1fe: mov DWORD PTR [edi+0x68],0x2000
                mem.write(r.edi.wrapping_add(0x68), 0x2000u32);
                // 0xa4a205: mov DWORD PTR [edi+0x13c],0x0
                mem.write(r.edi.wrapping_add(0x13c), 0x0u32);
                // 0xa4a20f: mov DWORD PTR [edi+0x6c],0x0
                mem.write(r.edi.wrapping_add(0x6c), 0x0u32);
                // 0xa4a216: mov eax,DWORD PTR [edx-0x1c]
                r.eax = mem.read::<u32>(r.edx.wrapping_sub(0x1c));
                // 0xa4a219: mov DWORD PTR [edi+0x160],eax
                mem.write(r.edi.wrapping_add(0x160), r.eax);
                // 0xa4a21f: mov DWORD PTR [edi+0x90],eax
                mem.write(r.edi.wrapping_add(0x90), r.eax);
                // 0xa4a225: mov eax,DWORD PTR [edx-0x18]
                r.eax = mem.read::<u32>(r.edx.wrapping_sub(0x18));
                // 0xa4a228: mov DWORD PTR [edi+0x164],eax
                mem.write(r.edi.wrapping_add(0x164), r.eax);
                // 0xa4a22e: mov DWORD PTR [edi+0x94],eax
                mem.write(r.edi.wrapping_add(0x94), r.eax);
                // 0xa4a234: mov DWORD PTR [edi+0x168],0x2000
                mem.write(r.edi.wrapping_add(0x168), 0x2000u32);
                // 0xa4a23e: mov DWORD PTR [edi+0x98],0x2000
                mem.write(r.edi.wrapping_add(0x98), 0x2000u32);
                // 0xa4a248: mov DWORD PTR [edi+0x16c],0x0
                mem.write(r.edi.wrapping_add(0x16c), 0x0u32);
                // 0xa4a252: mov DWORD PTR [edi+0x9c],0x0
                mem.write(r.edi.wrapping_add(0x9c), 0x0u32);
                // 0xa4a25c: mov eax,DWORD PTR [edx-0xc]
                r.eax = mem.read::<u32>(r.edx.wrapping_sub(0xc));
                // 0xa4a25f: mov DWORD PTR [edi+0x190],eax
                mem.write(r.edi.wrapping_add(0x190), r.eax);
                // 0xa4a265: mov DWORD PTR [edi+0xc0],eax
                mem.write(r.edi.wrapping_add(0xc0), r.eax);
                // 0xa4a26b: mov eax,DWORD PTR [edx-0x8]
                r.eax = mem.read::<u32>(r.edx.wrapping_sub(0x8));
                // 0xa4a26e: mov DWORD PTR [edi+0x194],eax
                mem.write(r.edi.wrapping_add(0x194), r.eax);
                // 0xa4a274: mov DWORD PTR [edi+0xc4],eax
                mem.write(r.edi.wrapping_add(0xc4), r.eax);
                // 0xa4a27a: mov eax,DWORD PTR [ebp-0x240]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x240));
                // 0xa4a280: mov DWORD PTR [edi+0x198],0x2000
                mem.write(r.edi.wrapping_add(0x198), 0x2000u32);
                // 0xa4a28a: mov DWORD PTR [edi+0xc8],0x2000
                mem.write(r.edi.wrapping_add(0xc8), 0x2000u32);
                // 0xa4a294: mov DWORD PTR [edi+0x19c],0x0
                mem.write(r.edi.wrapping_add(0x19c), 0x0u32);
                // 0xa4a29e: mov DWORD PTR [edi+0xcc],0x0
                mem.write(r.edi.wrapping_add(0xcc), 0x0u32);
                // 0xa4a2a8: mov ds:0xc8f7d0,eax
                mem.write(mem.rebase(0xc8f7d0), r.eax);
                // 0xa4a2ad: mov eax,DWORD PTR [ebp-0x23c]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x23c));
                // 0xa4a2b3: mov ds:0xc8f7d4,eax
                mem.write(mem.rebase(0xc8f7d4), r.eax);
                // 0xa4a2b8: mov eax,DWORD PTR [ebp-0x238]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x238));
                // 0xa4a2be: mov ds:0xc8f7d8,eax
                mem.write(mem.rebase(0xc8f7d8), r.eax);
                // 0xa4a2c3: mov eax,DWORD PTR [ebp-0x234]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x234));
                // 0xa4a2c9: mov ds:0xc8f7dc,eax
                mem.write(mem.rebase(0xc8f7dc), r.eax);
                // 0xa4a2ce: fld DWORD PTR [edx-0x160]
                st.push(ld32(mem, r.edx.wrapping_sub(0x160)));
                // 0xa4a2d4: fstp DWORD PTR ds:0xc8f7e0
                mem.write(mem.rebase(0xc8f7e0), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a2da: fld DWORD PTR ds:0xc8f7e0
                st.push(ld32(mem, mem.rebase(0xc8f7e0)));
                // 0xa4a2e0: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4a2e2: fmul st,st(6)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 6));
                    set(&mut st, 0, x * y);
                }
                // 0xa4a2e4: fstp DWORD PTR ds:0xc8f7d0
                mem.write(mem.rebase(0xc8f7d0), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a2ea: mov eax,ds:0xc8f7d0
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d0));
                // 0xa4a2ef: fld DWORD PTR ds:0xc8f7d4
                st.push(ld32(mem, mem.rebase(0xc8f7d4)));
                // 0xa4a2f5: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4a2f7: fstp DWORD PTR ds:0xc8f7d4
                mem.write(mem.rebase(0xc8f7d4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a2fd: fmul DWORD PTR ds:0xc8f7d8
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f7d8)));
                }
                // 0xa4a303: fstp DWORD PTR ds:0xc8f7d8
                mem.write(mem.rebase(0xc8f7d8), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a309: mov DWORD PTR [edi+0x10],eax
                mem.write(r.edi.wrapping_add(0x10), r.eax);
                // 0xa4a30c: mov eax,ds:0xc8f7d4
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d4));
                // 0xa4a311: mov DWORD PTR [edi+0x14],eax
                mem.write(r.edi.wrapping_add(0x14), r.eax);
                // 0xa4a314: mov eax,ds:0xc8f7d8
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d8));
                // 0xa4a319: mov DWORD PTR [edi+0x18],eax
                mem.write(r.edi.wrapping_add(0x18), r.eax);
                // 0xa4a31c: mov eax,ds:0xc8f7dc
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7dc));
                // 0xa4a321: mov DWORD PTR [edi+0x1c],eax
                mem.write(r.edi.wrapping_add(0x1c), r.eax);
                // 0xa4a324: mov eax,DWORD PTR [ebp-0x230]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x230));
                // 0xa4a32a: mov ds:0xc8f7d0,eax
                mem.write(mem.rebase(0xc8f7d0), r.eax);
                // 0xa4a32f: mov eax,DWORD PTR [ebp-0x22c]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x22c));
                // 0xa4a335: mov ds:0xc8f7d4,eax
                mem.write(mem.rebase(0xc8f7d4), r.eax);
                // 0xa4a33a: mov eax,DWORD PTR [ebp-0x228]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x228));
                // 0xa4a340: mov ds:0xc8f7d8,eax
                mem.write(mem.rebase(0xc8f7d8), r.eax);
                // 0xa4a345: mov eax,DWORD PTR [ebp-0x224]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x224));
                // 0xa4a34b: mov ds:0xc8f7dc,eax
                mem.write(mem.rebase(0xc8f7dc), r.eax);
                // 0xa4a350: fld DWORD PTR [edx-0x150]
                st.push(ld32(mem, r.edx.wrapping_sub(0x150)));
                // 0xa4a356: fstp DWORD PTR ds:0xc8f7e0
                mem.write(mem.rebase(0xc8f7e0), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a35c: fld DWORD PTR [ebp-0x230]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x230)));
                // 0xa4a362: fld DWORD PTR ds:0xc8f7e0
                st.push(ld32(mem, mem.rebase(0xc8f7e0)));
                // 0xa4a368: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4a36a: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0xa4a36c: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4a36e: fstp DWORD PTR ds:0xc8f7d0
                mem.write(mem.rebase(0xc8f7d0), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a374: mov eax,ds:0xc8f7d0
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d0));
                // 0xa4a379: fld DWORD PTR ds:0xc8f7d4
                st.push(ld32(mem, mem.rebase(0xc8f7d4)));
                // 0xa4a37f: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4a381: fstp DWORD PTR ds:0xc8f7d4
                mem.write(mem.rebase(0xc8f7d4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a387: fmul DWORD PTR ds:0xc8f7d8
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f7d8)));
                }
                // 0xa4a38d: fstp DWORD PTR ds:0xc8f7d8
                mem.write(mem.rebase(0xc8f7d8), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a393: mov DWORD PTR [edi+0x40],eax
                mem.write(r.edi.wrapping_add(0x40), r.eax);
                // 0xa4a396: mov eax,ds:0xc8f7d4
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d4));
                // 0xa4a39b: mov DWORD PTR [edi+0x44],eax
                mem.write(r.edi.wrapping_add(0x44), r.eax);
                // 0xa4a39e: mov eax,ds:0xc8f7d8
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d8));
                // 0xa4a3a3: mov DWORD PTR [edi+0x48],eax
                mem.write(r.edi.wrapping_add(0x48), r.eax);
                // 0xa4a3a6: mov eax,ds:0xc8f7dc
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7dc));
                // 0xa4a3ab: mov DWORD PTR [edi+0x4c],eax
                mem.write(r.edi.wrapping_add(0x4c), r.eax);
                // 0xa4a3ae: mov eax,DWORD PTR [ebp-0x220]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x220));
                // 0xa4a3b4: mov ds:0xc8f7d0,eax
                mem.write(mem.rebase(0xc8f7d0), r.eax);
                // 0xa4a3b9: mov eax,DWORD PTR [ebp-0x21c]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x21c));
                // 0xa4a3bf: mov ds:0xc8f7d4,eax
                mem.write(mem.rebase(0xc8f7d4), r.eax);
                // 0xa4a3c4: mov eax,DWORD PTR [ebp-0x218]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x218));
                // 0xa4a3ca: mov ds:0xc8f7d8,eax
                mem.write(mem.rebase(0xc8f7d8), r.eax);
                // 0xa4a3cf: mov eax,DWORD PTR [ebp-0x214]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x214));
                // 0xa4a3d5: mov ds:0xc8f7dc,eax
                mem.write(mem.rebase(0xc8f7dc), r.eax);
                // 0xa4a3da: fld DWORD PTR [edx-0x10]
                st.push(ld32(mem, r.edx.wrapping_sub(0x10)));
                // 0xa4a3dd: fstp DWORD PTR ds:0xc8f7e0
                mem.write(mem.rebase(0xc8f7e0), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a3e3: fld DWORD PTR ds:0xc8f7e0
                st.push(ld32(mem, mem.rebase(0xc8f7e0)));
                // 0xa4a3e9: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4a3eb: fmulp st(6),st
                {
                    let (x, y) = (sti(&st, 6), sti(&st, 0));
                    set(&mut st, 6, x * y);
                }
                st.pop();
                // 0xa4a3ed: fxch st(5)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 5);
                }
                // 0xa4a3ef: fstp DWORD PTR ds:0xc8f7d0
                mem.write(mem.rebase(0xc8f7d0), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a3f5: mov eax,ds:0xc8f7d0
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d0));
                // 0xa4a3fa: fld DWORD PTR ds:0xc8f7d4
                st.push(ld32(mem, mem.rebase(0xc8f7d4)));
                // 0xa4a400: fmul st,st(5)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 5));
                    set(&mut st, 0, x * y);
                }
                // 0xa4a402: fstp DWORD PTR ds:0xc8f7d4
                mem.write(mem.rebase(0xc8f7d4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a408: fld DWORD PTR ds:0xc8f7d8
                st.push(ld32(mem, mem.rebase(0xc8f7d8)));
                // 0xa4a40e: fmulp st(5),st
                {
                    let (x, y) = (sti(&st, 5), sti(&st, 0));
                    set(&mut st, 5, x * y);
                }
                st.pop();
                // 0xa4a410: fxch st(4)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 4);
                }
                // 0xa4a412: fstp DWORD PTR ds:0xc8f7d8
                mem.write(mem.rebase(0xc8f7d8), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a418: mov DWORD PTR [edi+0x70],eax
                mem.write(r.edi.wrapping_add(0x70), r.eax);
                // 0xa4a41b: mov eax,ds:0xc8f7d4
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d4));
                // 0xa4a420: mov DWORD PTR [edi+0x74],eax
                mem.write(r.edi.wrapping_add(0x74), r.eax);
                // 0xa4a423: mov eax,ds:0xc8f7d8
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d8));
                // 0xa4a428: mov DWORD PTR [edi+0x78],eax
                mem.write(r.edi.wrapping_add(0x78), r.eax);
                // 0xa4a42b: mov eax,ds:0xc8f7dc
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7dc));
                // 0xa4a430: mov DWORD PTR [edi+0x7c],eax
                mem.write(r.edi.wrapping_add(0x7c), r.eax);
                // 0xa4a433: mov eax,DWORD PTR [ebp-0x210]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x210));
                // 0xa4a439: mov ds:0xc8f7d0,eax
                mem.write(mem.rebase(0xc8f7d0), r.eax);
                // 0xa4a43e: mov eax,DWORD PTR [ebp-0x20c]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x20c));
                // 0xa4a444: mov ds:0xc8f7d4,eax
                mem.write(mem.rebase(0xc8f7d4), r.eax);
                // 0xa4a449: mov eax,DWORD PTR [ebp-0x208]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x208));
                // 0xa4a44f: mov ds:0xc8f7d8,eax
                mem.write(mem.rebase(0xc8f7d8), r.eax);
                // 0xa4a454: mov eax,DWORD PTR [ebp-0x204]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x204));
                // 0xa4a45a: mov ds:0xc8f7dc,eax
                mem.write(mem.rebase(0xc8f7dc), r.eax);
                // 0xa4a45f: fld DWORD PTR [edx]
                st.push(ld32(mem, r.edx));
                // 0xa4a461: fstp DWORD PTR ds:0xc8f7e0
                mem.write(mem.rebase(0xc8f7e0), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a467: fld DWORD PTR [ebp-0x210]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x210)));
                // 0xa4a46d: fld DWORD PTR ds:0xc8f7e0
                st.push(ld32(mem, mem.rebase(0xc8f7e0)));
                // 0xa4a473: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4a475: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0xa4a477: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4a479: fstp DWORD PTR ds:0xc8f7d0
                mem.write(mem.rebase(0xc8f7d0), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a47f: mov eax,ds:0xc8f7d0
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d0));
                // 0xa4a484: fld DWORD PTR ds:0xc8f7d4
                st.push(ld32(mem, mem.rebase(0xc8f7d4)));
                // 0xa4a48a: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4a48c: fstp DWORD PTR ds:0xc8f7d4
                mem.write(mem.rebase(0xc8f7d4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a492: fmul DWORD PTR ds:0xc8f7d8
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f7d8)));
                }
                // 0xa4a498: fstp DWORD PTR ds:0xc8f7d8
                mem.write(mem.rebase(0xc8f7d8), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a49e: mov DWORD PTR [edi+0xa0],eax
                mem.write(r.edi.wrapping_add(0xa0), r.eax);
                // 0xa4a4a4: mov eax,ds:0xc8f7d4
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d4));
                // 0xa4a4a9: mov DWORD PTR [edi+0xa4],eax
                mem.write(r.edi.wrapping_add(0xa4), r.eax);
                // 0xa4a4af: mov eax,ds:0xc8f7d8
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d8));
                // 0xa4a4b4: mov DWORD PTR [edi+0xa8],eax
                mem.write(r.edi.wrapping_add(0xa8), r.eax);
                // 0xa4a4ba: mov eax,ds:0xc8f7dc
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7dc));
                // 0xa4a4bf: mov DWORD PTR [edi+0xac],eax
                mem.write(r.edi.wrapping_add(0xac), r.eax);
                // 0xa4a4c5: mov eax,DWORD PTR [ebp-0x200]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x200));
                // 0xa4a4cb: mov ds:0xc8f7d0,eax
                mem.write(mem.rebase(0xc8f7d0), r.eax);
                // 0xa4a4d0: mov eax,DWORD PTR [ebp-0x1fc]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x1fc));
                // 0xa4a4d6: mov ds:0xc8f7d4,eax
                mem.write(mem.rebase(0xc8f7d4), r.eax);
                // 0xa4a4db: mov eax,DWORD PTR [ebp-0x1f8]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x1f8));
                // 0xa4a4e1: mov ds:0xc8f7d8,eax
                mem.write(mem.rebase(0xc8f7d8), r.eax);
                // 0xa4a4e6: mov eax,DWORD PTR [ebp-0x1f4]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x1f4));
                // 0xa4a4ec: mov ds:0xc8f7dc,eax
                mem.write(mem.rebase(0xc8f7dc), r.eax);
                // 0xa4a4f1: fld DWORD PTR [edx-0x160]
                st.push(ld32(mem, r.edx.wrapping_sub(0x160)));
                // 0xa4a4f7: fstp DWORD PTR ds:0xc8f7e0
                mem.write(mem.rebase(0xc8f7e0), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a4fd: fld DWORD PTR [ebp-0x200]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x200)));
                // 0xa4a503: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4a505: fld DWORD PTR ds:0xc8f7e0
                st.push(ld32(mem, mem.rebase(0xc8f7e0)));
                // 0xa4a50b: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4a50d: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0xa4a50f: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4a511: fstp DWORD PTR ds:0xc8f7d0
                mem.write(mem.rebase(0xc8f7d0), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a517: mov eax,ds:0xc8f7d0
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d0));
                // 0xa4a51c: fld DWORD PTR ds:0xc8f7d4
                st.push(ld32(mem, mem.rebase(0xc8f7d4)));
                // 0xa4a522: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4a524: fstp DWORD PTR ds:0xc8f7d4
                mem.write(mem.rebase(0xc8f7d4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a52a: fmul DWORD PTR ds:0xc8f7d8
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f7d8)));
                }
                // 0xa4a530: fstp DWORD PTR ds:0xc8f7d8
                mem.write(mem.rebase(0xc8f7d8), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a536: mov DWORD PTR [edi+0xe0],eax
                mem.write(r.edi.wrapping_add(0xe0), r.eax);
                // 0xa4a53c: mov eax,ds:0xc8f7d4
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d4));
                // 0xa4a541: mov DWORD PTR [edi+0xe4],eax
                mem.write(r.edi.wrapping_add(0xe4), r.eax);
                // 0xa4a547: mov eax,ds:0xc8f7d8
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d8));
                // 0xa4a54c: mov DWORD PTR [edi+0xe8],eax
                mem.write(r.edi.wrapping_add(0xe8), r.eax);
                // 0xa4a552: mov eax,ds:0xc8f7dc
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7dc));
                // 0xa4a557: mov DWORD PTR [edi+0xec],eax
                mem.write(r.edi.wrapping_add(0xec), r.eax);
                // 0xa4a55d: mov eax,DWORD PTR [ebp-0x1f0]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x1f0));
                // 0xa4a563: mov ds:0xc8f7d0,eax
                mem.write(mem.rebase(0xc8f7d0), r.eax);
                // 0xa4a568: mov eax,DWORD PTR [ebp-0x1ec]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x1ec));
                // 0xa4a56e: mov ds:0xc8f7d4,eax
                mem.write(mem.rebase(0xc8f7d4), r.eax);
                // 0xa4a573: mov eax,DWORD PTR [ebp-0x1e8]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x1e8));
                // 0xa4a579: mov ds:0xc8f7d8,eax
                mem.write(mem.rebase(0xc8f7d8), r.eax);
                // 0xa4a57e: mov eax,DWORD PTR [ebp-0x1e4]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x1e4));
                // 0xa4a584: mov ds:0xc8f7dc,eax
                mem.write(mem.rebase(0xc8f7dc), r.eax);
                // 0xa4a589: fld DWORD PTR [edx-0x150]
                st.push(ld32(mem, r.edx.wrapping_sub(0x150)));
                // 0xa4a58f: fstp DWORD PTR ds:0xc8f7e0
                mem.write(mem.rebase(0xc8f7e0), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a595: fld DWORD PTR [ebp-0x1f0]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x1f0)));
                // 0xa4a59b: fld DWORD PTR ds:0xc8f7e0
                st.push(ld32(mem, mem.rebase(0xc8f7e0)));
                // 0xa4a5a1: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4a5a3: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0xa4a5a5: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4a5a7: fstp DWORD PTR ds:0xc8f7d0
                mem.write(mem.rebase(0xc8f7d0), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a5ad: mov eax,ds:0xc8f7d0
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d0));
                // 0xa4a5b2: fld DWORD PTR ds:0xc8f7d4
                st.push(ld32(mem, mem.rebase(0xc8f7d4)));
                // 0xa4a5b8: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4a5ba: fstp DWORD PTR ds:0xc8f7d4
                mem.write(mem.rebase(0xc8f7d4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a5c0: fmul DWORD PTR ds:0xc8f7d8
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f7d8)));
                }
                // 0xa4a5c6: fstp DWORD PTR ds:0xc8f7d8
                mem.write(mem.rebase(0xc8f7d8), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a5cc: mov DWORD PTR [edi+0x110],eax
                mem.write(r.edi.wrapping_add(0x110), r.eax);
                // 0xa4a5d2: mov eax,ds:0xc8f7d4
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d4));
                // 0xa4a5d7: mov DWORD PTR [edi+0x114],eax
                mem.write(r.edi.wrapping_add(0x114), r.eax);
                // 0xa4a5dd: mov eax,ds:0xc8f7d8
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d8));
                // 0xa4a5e2: mov DWORD PTR [edi+0x118],eax
                mem.write(r.edi.wrapping_add(0x118), r.eax);
                // 0xa4a5e8: mov eax,ds:0xc8f7dc
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7dc));
                // 0xa4a5ed: mov DWORD PTR [edi+0x11c],eax
                mem.write(r.edi.wrapping_add(0x11c), r.eax);
                // 0xa4a5f3: mov eax,DWORD PTR [ebp-0x1e0]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x1e0));
                // 0xa4a5f9: mov ds:0xc8f7d0,eax
                mem.write(mem.rebase(0xc8f7d0), r.eax);
                // 0xa4a5fe: mov eax,DWORD PTR [ebp-0x1dc]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x1dc));
                // 0xa4a604: mov ds:0xc8f7d4,eax
                mem.write(mem.rebase(0xc8f7d4), r.eax);
                // 0xa4a609: mov eax,DWORD PTR [ebp-0x1d8]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x1d8));
                // 0xa4a60f: mov ds:0xc8f7d8,eax
                mem.write(mem.rebase(0xc8f7d8), r.eax);
                // 0xa4a614: mov eax,DWORD PTR [ebp-0x1d4]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x1d4));
                // 0xa4a61a: mov ds:0xc8f7dc,eax
                mem.write(mem.rebase(0xc8f7dc), r.eax);
                // 0xa4a61f: fld DWORD PTR [edx-0x10]
                st.push(ld32(mem, r.edx.wrapping_sub(0x10)));
                // 0xa4a622: fstp DWORD PTR ds:0xc8f7e0
                mem.write(mem.rebase(0xc8f7e0), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a628: fld DWORD PTR ds:0xc8f7e0
                st.push(ld32(mem, mem.rebase(0xc8f7e0)));
                // 0xa4a62e: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4a630: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0xa4a632: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4a634: fstp DWORD PTR ds:0xc8f7d0
                mem.write(mem.rebase(0xc8f7d0), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a63a: fld DWORD PTR ds:0xc8f7d4
                st.push(ld32(mem, mem.rebase(0xc8f7d4)));
                // 0xa4a640: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4a642: fstp DWORD PTR ds:0xc8f7d4
                mem.write(mem.rebase(0xc8f7d4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a648: fmul DWORD PTR ds:0xc8f7d8
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f7d8)));
                }
                // 0xa4a64e: mov eax,ds:0xc8f7d0
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d0));
                // 0xa4a653: fstp DWORD PTR ds:0xc8f7d8
                mem.write(mem.rebase(0xc8f7d8), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a659: mov DWORD PTR [edi+0x140],eax
                mem.write(r.edi.wrapping_add(0x140), r.eax);
                // 0xa4a65f: mov eax,ds:0xc8f7d4
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d4));
                // 0xa4a664: mov DWORD PTR [edi+0x144],eax
                mem.write(r.edi.wrapping_add(0x144), r.eax);
                // 0xa4a66a: mov eax,ds:0xc8f7d8
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d8));
                // 0xa4a66f: mov DWORD PTR [edi+0x148],eax
                mem.write(r.edi.wrapping_add(0x148), r.eax);
                // 0xa4a675: mov eax,ds:0xc8f7dc
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7dc));
                // 0xa4a67a: mov DWORD PTR [edi+0x14c],eax
                mem.write(r.edi.wrapping_add(0x14c), r.eax);
                // 0xa4a680: mov eax,DWORD PTR [ebp-0x1d0]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x1d0));
                // 0xa4a686: mov ds:0xc8f7d0,eax
                mem.write(mem.rebase(0xc8f7d0), r.eax);
                // 0xa4a68b: mov eax,DWORD PTR [ebp-0x1cc]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x1cc));
                // 0xa4a691: mov ds:0xc8f7d4,eax
                mem.write(mem.rebase(0xc8f7d4), r.eax);
                // 0xa4a696: mov eax,DWORD PTR [ebp-0x1c8]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x1c8));
                // 0xa4a69c: mov ds:0xc8f7d8,eax
                mem.write(mem.rebase(0xc8f7d8), r.eax);
                // 0xa4a6a1: mov eax,DWORD PTR [ebp-0x1c4]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x1c4));
                // 0xa4a6a7: mov ds:0xc8f7dc,eax
                mem.write(mem.rebase(0xc8f7dc), r.eax);
                // 0xa4a6ac: fld DWORD PTR [edx]
                st.push(ld32(mem, r.edx));
                // 0xa4a6ae: fstp DWORD PTR ds:0xc8f7e0
                mem.write(mem.rebase(0xc8f7e0), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a6b4: mov esi,DWORD PTR [ebp-0x250]
                r.esi = mem.read::<u32>(r.ebp.wrapping_sub(0x250));
                // 0xa4a6ba: fld DWORD PTR [ebp-0x1d0]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x1d0)));
                // 0xa4a6c0: mov edx,DWORD PTR [ebp-0x24c]
                r.edx = mem.read::<u32>(r.ebp.wrapping_sub(0x24c));
                // 0xa4a6c6: fld DWORD PTR ds:0xc8f7e0
                st.push(ld32(mem, mem.rebase(0xc8f7e0)));
                // 0xa4a6cc: mov ecx,DWORD PTR [ebp-0x248]
                r.ecx = mem.read::<u32>(r.ebp.wrapping_sub(0x248));
                // 0xa4a6d2: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4a6d4: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0xa4a6d6: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4a6d8: fstp DWORD PTR ds:0xc8f7d0
                mem.write(mem.rebase(0xc8f7d0), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a6de: mov eax,ds:0xc8f7d0
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d0));
                // 0xa4a6e3: fld DWORD PTR ds:0xc8f7d4
                st.push(ld32(mem, mem.rebase(0xc8f7d4)));
                // 0xa4a6e9: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4a6eb: fstp DWORD PTR ds:0xc8f7d4
                mem.write(mem.rebase(0xc8f7d4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a6f1: fmul DWORD PTR ds:0xc8f7d8
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f7d8)));
                }
                // 0xa4a6f7: fstp DWORD PTR ds:0xc8f7d8
                mem.write(mem.rebase(0xc8f7d8), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a6fd: mov DWORD PTR [edi+0x170],eax
                mem.write(r.edi.wrapping_add(0x170), r.eax);
                // 0xa4a703: mov eax,ds:0xc8f7d4
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d4));
                // 0xa4a708: fild DWORD PTR [edi+0x30]
                st.push(f64::from(mem.read::<i32>(r.edi.wrapping_add(0x30))));
                // 0xa4a70b: mov DWORD PTR [edi+0x174],eax
                mem.write(r.edi.wrapping_add(0x174), r.eax);
                // 0xa4a711: mov eax,ds:0xc8f7d8
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d8));
                // 0xa4a716: mov DWORD PTR [edi+0x178],eax
                mem.write(r.edi.wrapping_add(0x178), r.eax);
                // 0xa4a71c: mov eax,ds:0xc8f7dc
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7dc));
                // 0xa4a721: mov DWORD PTR [edi+0x180],esi
                mem.write(r.edi.wrapping_add(0x180), r.esi);
                // 0xa4a727: mov DWORD PTR [edi+0x150],esi
                mem.write(r.edi.wrapping_add(0x150), r.esi);
                // 0xa4a72d: mov DWORD PTR [edi+0x120],esi
                mem.write(r.edi.wrapping_add(0x120), r.esi);
                // 0xa4a733: mov DWORD PTR [edi+0xf0],esi
                mem.write(r.edi.wrapping_add(0xf0), r.esi);
                // 0xa4a739: mov DWORD PTR [edi+0xb0],esi
                mem.write(r.edi.wrapping_add(0xb0), r.esi);
                // 0xa4a73f: mov DWORD PTR [edi+0x80],esi
                mem.write(r.edi.wrapping_add(0x80), r.esi);
                // 0xa4a745: mov DWORD PTR [edi+0x50],esi
                mem.write(r.edi.wrapping_add(0x50), r.esi);
                // 0xa4a748: mov DWORD PTR [edi+0x20],esi
                mem.write(r.edi.wrapping_add(0x20), r.esi);
                // 0xa4a74b: mov DWORD PTR [edi+0x17c],eax
                mem.write(r.edi.wrapping_add(0x17c), r.eax);
                // 0xa4a751: mov eax,DWORD PTR [ebp-0x244]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x244));
                // 0xa4a757: mov DWORD PTR [edi+0x184],edx
                mem.write(r.edi.wrapping_add(0x184), r.edx);
                // 0xa4a75d: mov DWORD PTR [edi+0x154],edx
                mem.write(r.edi.wrapping_add(0x154), r.edx);
                // 0xa4a763: mov DWORD PTR [edi+0x124],edx
                mem.write(r.edi.wrapping_add(0x124), r.edx);
                // 0xa4a769: mov DWORD PTR [edi+0xf4],edx
                mem.write(r.edi.wrapping_add(0xf4), r.edx);
                // 0xa4a76f: mov DWORD PTR [edi+0xb4],edx
                mem.write(r.edi.wrapping_add(0xb4), r.edx);
                // 0xa4a775: mov DWORD PTR [edi+0x84],edx
                mem.write(r.edi.wrapping_add(0x84), r.edx);
                // 0xa4a77b: mov DWORD PTR [edi+0x54],edx
                mem.write(r.edi.wrapping_add(0x54), r.edx);
                // 0xa4a77e: mov DWORD PTR [edi+0x24],edx
                mem.write(r.edi.wrapping_add(0x24), r.edx);
                // 0xa4a781: mov DWORD PTR [edi+0x188],ecx
                mem.write(r.edi.wrapping_add(0x188), r.ecx);
                // 0xa4a787: mov DWORD PTR [edi+0x158],ecx
                mem.write(r.edi.wrapping_add(0x158), r.ecx);
                // 0xa4a78d: mov DWORD PTR [edi+0x128],ecx
                mem.write(r.edi.wrapping_add(0x128), r.ecx);
                // 0xa4a793: mov DWORD PTR [edi+0xf8],ecx
                mem.write(r.edi.wrapping_add(0xf8), r.ecx);
                // 0xa4a799: mov DWORD PTR [edi+0xb8],ecx
                mem.write(r.edi.wrapping_add(0xb8), r.ecx);
                // 0xa4a79f: mov DWORD PTR [edi+0x88],ecx
                mem.write(r.edi.wrapping_add(0x88), r.ecx);
                // 0xa4a7a5: mov DWORD PTR [edi+0x58],ecx
                mem.write(r.edi.wrapping_add(0x58), r.ecx);
                // 0xa4a7a8: mov DWORD PTR [edi+0x28],ecx
                mem.write(r.edi.wrapping_add(0x28), r.ecx);
                // 0xa4a7ab: mov DWORD PTR [edi+0x18c],eax
                mem.write(r.edi.wrapping_add(0x18c), r.eax);
                // 0xa4a7b1: mov DWORD PTR [edi+0x15c],eax
                mem.write(r.edi.wrapping_add(0x15c), r.eax);
                // 0xa4a7b7: mov DWORD PTR [edi+0x12c],eax
                mem.write(r.edi.wrapping_add(0x12c), r.eax);
                // 0xa4a7bd: mov DWORD PTR [edi+0xfc],eax
                mem.write(r.edi.wrapping_add(0xfc), r.eax);
                // 0xa4a7c3: mov DWORD PTR [edi+0xbc],eax
                mem.write(r.edi.wrapping_add(0xbc), r.eax);
                // 0xa4a7c9: mov DWORD PTR [edi+0x8c],eax
                mem.write(r.edi.wrapping_add(0x8c), r.eax);
                // 0xa4a7cf: mov DWORD PTR [edi+0x5c],eax
                mem.write(r.edi.wrapping_add(0x5c), r.eax);
                // 0xa4a7d2: mov DWORD PTR [edi+0x2c],eax
                mem.write(r.edi.wrapping_add(0x2c), r.eax);
                // 0xa4a7d5: fstp DWORD PTR [ebp-0x1a4]
                mem.write(r.ebp.wrapping_sub(0x1a4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a7db: fld DWORD PTR [ebp-0x1a4]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x1a4)));
                // 0xa4a7e1: fmul st,st(3)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 3));
                    set(&mut st, 0, x * y);
                }
                // 0xa4a7e3: fsub st,st(2)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 2));
                    set(&mut st, 0, x - y);
                }
                // 0xa4a7e5: fadd st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x + y);
                }
                // 0xa4a7e7: fstp DWORD PTR [ebp-0x1a0]
                mem.write(r.ebp.wrapping_sub(0x1a0), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a7ed: fild DWORD PTR [edi+0x34]
                st.push(f64::from(mem.read::<i32>(r.edi.wrapping_add(0x34))));
                // 0xa4a7f0: fstp DWORD PTR [ebp-0x1a4]
                mem.write(r.ebp.wrapping_sub(0x1a4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a7f6: fld DWORD PTR [ebp-0x1a4]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x1a4)));
                // 0xa4a7fc: fmul st,st(3)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 3));
                    set(&mut st, 0, x * y);
                }
                // 0xa4a7fe: fsub st,st(2)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 2));
                    set(&mut st, 0, x - y);
                }
                // 0xa4a800: fld QWORD PTR ds:0xb54c70
                st.push(mem.read::<f64>(mem.rebase(0xb54c70)));
                // 0xa4a806: fadd st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, x + y);
                }
                // 0xa4a808: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4a80a: fstp DWORD PTR [ebp-0x19c]
                mem.write(r.ebp.wrapping_sub(0x19c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a810: fild DWORD PTR [edi+0x60]
                st.push(f64::from(mem.read::<i32>(r.edi.wrapping_add(0x60))));
                // 0xa4a813: fstp DWORD PTR [ebp-0x1a4]
                mem.write(r.ebp.wrapping_sub(0x1a4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a819: fld DWORD PTR [ebp-0x1a4]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x1a4)));
                // 0xa4a81f: fmul st,st(4)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 4));
                    set(&mut st, 0, x * y);
                }
                // 0xa4a821: fsub st,st(3)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 3));
                    set(&mut st, 0, x - y);
                }
                // 0xa4a823: fadd st,st(2)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 2));
                    set(&mut st, 0, x + y);
                }
                // 0xa4a825: fstp DWORD PTR [ebp-0x180]
                mem.write(r.ebp.wrapping_sub(0x180), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a82b: fild DWORD PTR [edi+0x64]
                st.push(f64::from(mem.read::<i32>(r.edi.wrapping_add(0x64))));
                // 0xa4a82e: fstp DWORD PTR [ebp-0x1a4]
                mem.write(r.ebp.wrapping_sub(0x1a4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a834: fld DWORD PTR [ebp-0x1a4]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x1a4)));
                // 0xa4a83a: fmul st,st(4)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 4));
                    set(&mut st, 0, x * y);
                }
                // 0xa4a83c: fsub st,st(3)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 3));
                    set(&mut st, 0, x - y);
                }
                // 0xa4a83e: fadd st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x + y);
                }
                // 0xa4a840: fstp DWORD PTR [ebp-0x17c]
                mem.write(r.ebp.wrapping_sub(0x17c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a846: fild DWORD PTR [edi+0x90]
                st.push(f64::from(mem.read::<i32>(r.edi.wrapping_add(0x90))));
                // 0xa4a84c: fstp DWORD PTR [ebp-0x1a4]
                mem.write(r.ebp.wrapping_sub(0x1a4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a852: fld DWORD PTR [ebp-0x1a4]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x1a4)));
                // 0xa4a858: fmul st,st(4)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 4));
                    set(&mut st, 0, x * y);
                }
                // 0xa4a85a: fsub st,st(3)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 3));
                    set(&mut st, 0, x - y);
                }
                // 0xa4a85c: fadd st,st(2)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 2));
                    set(&mut st, 0, x + y);
                }
                // 0xa4a85e: fstp DWORD PTR [ebp-0x160]
                mem.write(r.ebp.wrapping_sub(0x160), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a864: fild DWORD PTR [edi+0x94]
                st.push(f64::from(mem.read::<i32>(r.edi.wrapping_add(0x94))));
                // 0xa4a86a: fstp DWORD PTR [ebp-0x1a4]
                mem.write(r.ebp.wrapping_sub(0x1a4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a870: fld DWORD PTR [ebp-0x1a4]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x1a4)));
                // 0xa4a876: fmul st,st(4)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 4));
                    set(&mut st, 0, x * y);
                }
                // 0xa4a878: fsub st,st(3)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 3));
                    set(&mut st, 0, x - y);
                }
                // 0xa4a87a: fadd st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x + y);
                }
                // 0xa4a87c: fstp DWORD PTR [ebp-0x15c]
                mem.write(r.ebp.wrapping_sub(0x15c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a882: fild DWORD PTR [edi+0xc0]
                st.push(f64::from(mem.read::<i32>(r.edi.wrapping_add(0xc0))));
                // 0xa4a888: fstp DWORD PTR [ebp-0x1a4]
                mem.write(r.ebp.wrapping_sub(0x1a4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a88e: fld DWORD PTR [ebp-0x1a4]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x1a4)));
                // 0xa4a894: fmul st,st(4)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 4));
                    set(&mut st, 0, x * y);
                }
                // 0xa4a896: fsub st,st(3)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 3));
                    set(&mut st, 0, x - y);
                }
                // 0xa4a898: faddp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x + y);
                }
                st.pop();
                // 0xa4a89a: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4a89c: fstp DWORD PTR [ebp-0x140]
                mem.write(r.ebp.wrapping_sub(0x140), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a8a2: fild DWORD PTR [edi+0xc4]
                st.push(f64::from(mem.read::<i32>(r.edi.wrapping_add(0xc4))));
                // 0xa4a8a8: fstp DWORD PTR [ebp-0x1a4]
                mem.write(r.ebp.wrapping_sub(0x1a4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a8ae: fld DWORD PTR [ebp-0x1a4]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x1a4)));
                // 0xa4a8b4: fmulp st(3),st
                {
                    let (x, y) = (sti(&st, 3), sti(&st, 0));
                    set(&mut st, 3, x * y);
                }
                st.pop();
                // 0xa4a8b6: fxch st(2)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 2);
                }
                // 0xa4a8b8: fsubrp st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, y - x);
                }
                st.pop();
                // 0xa4a8ba: faddp st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, x + y);
                }
                st.pop();
                // 0xa4a8bc: fstp DWORD PTR [ebp-0x13c]
                mem.write(r.ebp.wrapping_sub(0x13c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a8c2: fld DWORD PTR [edi+0x10]
                st.push(ld32(mem, r.edi.wrapping_add(0x10)));
                // 0xa4a8c5: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4a8c7: fstp DWORD PTR [ebp-0x198]
                mem.write(r.ebp.wrapping_sub(0x198), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a8cd: fld DWORD PTR [edi+0x14]
                st.push(ld32(mem, r.edi.wrapping_add(0x14)));
                // 0xa4a8d0: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4a8d2: fstp DWORD PTR [ebp-0x194]
                mem.write(r.ebp.wrapping_sub(0x194), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a8d8: fld DWORD PTR [edi+0x40]
                st.push(ld32(mem, r.edi.wrapping_add(0x40)));
                // 0xa4a8db: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4a8dd: fstp DWORD PTR [ebp-0x178]
                mem.write(r.ebp.wrapping_sub(0x178), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a8e3: fld DWORD PTR [edi+0x44]
                st.push(ld32(mem, r.edi.wrapping_add(0x44)));
                // 0xa4a8e6: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4a8e8: fstp DWORD PTR [ebp-0x174]
                mem.write(r.ebp.wrapping_sub(0x174), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a8ee: fld DWORD PTR [edi+0x70]
                st.push(ld32(mem, r.edi.wrapping_add(0x70)));
                // 0xa4a8f1: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4a8f3: fstp DWORD PTR [ebp-0x158]
                mem.write(r.ebp.wrapping_sub(0x158), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a8f9: fld DWORD PTR [edi+0x74]
                st.push(ld32(mem, r.edi.wrapping_add(0x74)));
                // 0xa4a8fc: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4a8fe: fstp DWORD PTR [ebp-0x154]
                mem.write(r.ebp.wrapping_sub(0x154), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a904: fld DWORD PTR [edi+0xa0]
                st.push(ld32(mem, r.edi.wrapping_add(0xa0)));
                // 0xa4a90a: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4a90c: fstp DWORD PTR [ebp-0x138]
                mem.write(r.ebp.wrapping_sub(0x138), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a912: fmul DWORD PTR [edi+0xa4]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, r.edi.wrapping_add(0xa4)));
                }
                // 0xa4a918: mov DWORD PTR [ebp-0x190],0x80
                mem.write(r.ebp.wrapping_sub(0x190), 0x80u32);
                // 0xa4a922: mov DWORD PTR [ebp-0x18c],0x80
                mem.write(r.ebp.wrapping_sub(0x18c), 0x80u32);
                // 0xa4a92c: mov DWORD PTR [ebp-0x188],0x80
                mem.write(r.ebp.wrapping_sub(0x188), 0x80u32);
                // 0xa4a936: fstp DWORD PTR [ebp-0x134]
                mem.write(r.ebp.wrapping_sub(0x134), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a93c: mov DWORD PTR [ebp-0x184],0x80
                mem.write(r.ebp.wrapping_sub(0x184), 0x80u32);
                // 0xa4a946: mov DWORD PTR [ebp-0x170],0x80
                mem.write(r.ebp.wrapping_sub(0x170), 0x80u32);
                // 0xa4a950: push 0xa
                {
                    let v = 0xau32;
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0xa4a952: lea eax,[ebp-0x108]
                r.eax = r.ebp.wrapping_sub(0x108);
                // 0xa4a958: push eax
                {
                    let v = r.eax;
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0xa4a959: lea eax,[ebp-0x1a0]
                r.eax = r.ebp.wrapping_sub(0x1a0);
                // 0xa4a95f: push eax
                {
                    let v = r.eax;
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0xa4a960: mov DWORD PTR [ebp-0x16c],0x80
                mem.write(r.ebp.wrapping_sub(0x16c), 0x80u32);
                // 0xa4a96a: mov DWORD PTR [ebp-0x168],0x80
                mem.write(r.ebp.wrapping_sub(0x168), 0x80u32);
                // 0xa4a974: mov DWORD PTR [ebp-0x164],0x80
                mem.write(r.ebp.wrapping_sub(0x164), 0x80u32);
                // 0xa4a97e: mov DWORD PTR [ebp-0x150],0x80
                mem.write(r.ebp.wrapping_sub(0x150), 0x80u32);
                // 0xa4a988: mov DWORD PTR [ebp-0x14c],0x80
                mem.write(r.ebp.wrapping_sub(0x14c), 0x80u32);
                // 0xa4a992: mov DWORD PTR [ebp-0x148],0x80
                mem.write(r.ebp.wrapping_sub(0x148), 0x80u32);
                // 0xa4a99c: mov DWORD PTR [ebp-0x144],0x80
                mem.write(r.ebp.wrapping_sub(0x144), 0x80u32);
                // 0xa4a9a6: mov DWORD PTR [ebp-0x130],0x80
                mem.write(r.ebp.wrapping_sub(0x130), 0x80u32);
                // 0xa4a9b0: mov DWORD PTR [ebp-0x12c],0x80
                mem.write(r.ebp.wrapping_sub(0x12c), 0x80u32);
                // 0xa4a9ba: mov DWORD PTR [ebp-0x128],0x80
                mem.write(r.ebp.wrapping_sub(0x128), 0x80u32);
                // 0xa4a9c4: mov DWORD PTR [ebp-0x124],0x80
                mem.write(r.ebp.wrapping_sub(0x124), 0x80u32);
                // 0xa4a9ce: call 0x63e930
                {
                    let a: Vec<u32> = (0..3)
                        .map(|k| mem.read::<u32>(r.esp.wrapping_add(4 * k)))
                        .collect();
                    let f = mem.rebase(0x63e930);
                    r.eax = call(mem, f, &a);
                }
                // 0xa4a9d3: fild DWORD PTR [edi+0x100]
                st.push(f64::from(mem.read::<i32>(r.edi.wrapping_add(0x100))));
                // 0xa4a9d9: fstp DWORD PTR [ebp-0x1a4]
                mem.write(r.ebp.wrapping_sub(0x1a4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4a9df: fld DWORD PTR [ebp-0x1a4]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x1a4)));
                // 0xa4a9e5: fld QWORD PTR ds:0xb921e0
                st.push(mem.read::<f64>(mem.rebase(0xb921e0)));
                // 0xa4a9eb: fmul st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, x * y);
                }
                // 0xa4a9ed: fld QWORD PTR ds:0xb51bc0
                st.push(mem.read::<f64>(mem.rebase(0xb51bc0)));
                // 0xa4a9f3: fsub st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x - y);
                }
                // 0xa4a9f5: fld QWORD PTR ds:0xb922f0
                st.push(mem.read::<f64>(mem.rebase(0xb922f0)));
                // 0xa4a9fb: fadd st(3),st
                {
                    let (x, y) = (sti(&st, 3), sti(&st, 0));
                    set(&mut st, 3, x + y);
                }
                // 0xa4a9fd: fxch st(3)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 3);
                }
                // 0xa4a9ff: fstp DWORD PTR [ebp-0x1a0]
                mem.write(r.ebp.wrapping_sub(0x1a0), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4aa05: fild DWORD PTR [edi+0x104]
                st.push(f64::from(mem.read::<i32>(r.edi.wrapping_add(0x104))));
                // 0xa4aa0b: fstp DWORD PTR [ebp-0x1a4]
                mem.write(r.ebp.wrapping_sub(0x1a4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4aa11: fld DWORD PTR [ebp-0x1a4]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x1a4)));
                // 0xa4aa17: fmul st,st(2)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 2));
                    set(&mut st, 0, x * y);
                }
                // 0xa4aa19: fsub st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x - y);
                }
                // 0xa4aa1b: fld QWORD PTR ds:0xb54c70
                st.push(mem.read::<f64>(mem.rebase(0xb54c70)));
                // 0xa4aa21: fadd st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, x + y);
                }
                // 0xa4aa23: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4aa25: fstp DWORD PTR [ebp-0x19c]
                mem.write(r.ebp.wrapping_sub(0x19c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4aa2b: fild DWORD PTR [edi+0x130]
                st.push(f64::from(mem.read::<i32>(r.edi.wrapping_add(0x130))));
                // 0xa4aa31: fstp DWORD PTR [ebp-0x1a4]
                mem.write(r.ebp.wrapping_sub(0x1a4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4aa37: fld DWORD PTR [ebp-0x1a4]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x1a4)));
                // 0xa4aa3d: fmul st,st(3)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 3));
                    set(&mut st, 0, x * y);
                }
                // 0xa4aa3f: fsub st,st(2)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 2));
                    set(&mut st, 0, x - y);
                }
                // 0xa4aa41: fadd st,st(4)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 4));
                    set(&mut st, 0, x + y);
                }
                // 0xa4aa43: fstp DWORD PTR [ebp-0x180]
                mem.write(r.ebp.wrapping_sub(0x180), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4aa49: fild DWORD PTR [edi+0x134]
                st.push(f64::from(mem.read::<i32>(r.edi.wrapping_add(0x134))));
                // 0xa4aa4f: fstp DWORD PTR [ebp-0x1a4]
                mem.write(r.ebp.wrapping_sub(0x1a4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4aa55: fld DWORD PTR [ebp-0x1a4]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x1a4)));
                // 0xa4aa5b: fmul st,st(3)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 3));
                    set(&mut st, 0, x * y);
                }
                // 0xa4aa5d: fsub st,st(2)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 2));
                    set(&mut st, 0, x - y);
                }
                // 0xa4aa5f: fadd st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x + y);
                }
                // 0xa4aa61: fstp DWORD PTR [ebp-0x17c]
                mem.write(r.ebp.wrapping_sub(0x17c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4aa67: fild DWORD PTR [edi+0x160]
                st.push(f64::from(mem.read::<i32>(r.edi.wrapping_add(0x160))));
                // 0xa4aa6d: fstp DWORD PTR [ebp-0x1a4]
                mem.write(r.ebp.wrapping_sub(0x1a4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4aa73: fld DWORD PTR [ebp-0x1a4]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x1a4)));
                // 0xa4aa79: fmul st,st(3)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 3));
                    set(&mut st, 0, x * y);
                }
                // 0xa4aa7b: fsub st,st(2)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 2));
                    set(&mut st, 0, x - y);
                }
                // 0xa4aa7d: fadd st,st(4)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 4));
                    set(&mut st, 0, x + y);
                }
                // 0xa4aa7f: fstp DWORD PTR [ebp-0x160]
                mem.write(r.ebp.wrapping_sub(0x160), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4aa85: fild DWORD PTR [edi+0x164]
                st.push(f64::from(mem.read::<i32>(r.edi.wrapping_add(0x164))));
                // 0xa4aa8b: fstp DWORD PTR [ebp-0x1a4]
                mem.write(r.ebp.wrapping_sub(0x1a4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4aa91: fld DWORD PTR [ebp-0x1a4]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x1a4)));
                // 0xa4aa97: fmul st,st(3)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 3));
                    set(&mut st, 0, x * y);
                }
                // 0xa4aa99: fsub st,st(2)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 2));
                    set(&mut st, 0, x - y);
                }
                // 0xa4aa9b: fadd st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x + y);
                }
                // 0xa4aa9d: fstp DWORD PTR [ebp-0x15c]
                mem.write(r.ebp.wrapping_sub(0x15c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4aaa3: fild DWORD PTR [edi+0x190]
                st.push(f64::from(mem.read::<i32>(r.edi.wrapping_add(0x190))));
                // 0xa4aaa9: fstp DWORD PTR [ebp-0x1a4]
                mem.write(r.ebp.wrapping_sub(0x1a4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4aaaf: fld DWORD PTR [ebp-0x1a4]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x1a4)));
                // 0xa4aab5: fmul st,st(3)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 3));
                    set(&mut st, 0, x * y);
                }
                // 0xa4aab7: fsub st,st(2)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 2));
                    set(&mut st, 0, x - y);
                }
                // 0xa4aab9: faddp st(4),st
                {
                    let (x, y) = (sti(&st, 4), sti(&st, 0));
                    set(&mut st, 4, x + y);
                }
                st.pop();
                // 0xa4aabb: fxch st(3)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 3);
                }
                // 0xa4aabd: fstp DWORD PTR [ebp-0x140]
                mem.write(r.ebp.wrapping_sub(0x140), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4aac3: fild DWORD PTR [edi+0x194]
                st.push(f64::from(mem.read::<i32>(r.edi.wrapping_add(0x194))));
                // 0xa4aac9: fstp DWORD PTR [ebp-0x1a4]
                mem.write(r.ebp.wrapping_sub(0x1a4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4aacf: fld DWORD PTR [ebp-0x1a4]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x1a4)));
                // 0xa4aad5: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0xa4aad7: fsubp st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, x - y);
                }
                st.pop();
                // 0xa4aad9: faddp st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, x + y);
                }
                st.pop();
                // 0xa4aadb: fstp DWORD PTR [ebp-0x13c]
                mem.write(r.ebp.wrapping_sub(0x13c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4aae1: fld DWORD PTR [edi+0xe0]
                st.push(ld32(mem, r.edi.wrapping_add(0xe0)));
                // 0xa4aae7: fld QWORD PTR ds:0xb41d38
                st.push(mem.read::<f64>(mem.rebase(0xb41d38)));
                // 0xa4aaed: push 0xa
                {
                    let v = 0xau32;
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0xa4aaef: fmul st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, x * y);
                }
                // 0xa4aaf1: lea eax,[ebp-0x108]
                r.eax = r.ebp.wrapping_sub(0x108);
                // 0xa4aaf7: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4aaf9: push eax
                {
                    let v = r.eax;
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0xa4aafa: lea eax,[ebp-0x1a0]
                r.eax = r.ebp.wrapping_sub(0x1a0);
                // 0xa4ab00: fstp DWORD PTR [ebp-0x198]
                mem.write(r.ebp.wrapping_sub(0x198), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4ab06: push eax
                {
                    let v = r.eax;
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0xa4ab07: fld DWORD PTR [edi+0xe4]
                st.push(ld32(mem, r.edi.wrapping_add(0xe4)));
                // 0xa4ab0d: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4ab0f: fstp DWORD PTR [ebp-0x194]
                mem.write(r.ebp.wrapping_sub(0x194), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4ab15: fld DWORD PTR [edi+0x110]
                st.push(ld32(mem, r.edi.wrapping_add(0x110)));
                // 0xa4ab1b: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4ab1d: fstp DWORD PTR [ebp-0x178]
                mem.write(r.ebp.wrapping_sub(0x178), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4ab23: fld DWORD PTR [edi+0x114]
                st.push(ld32(mem, r.edi.wrapping_add(0x114)));
                // 0xa4ab29: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4ab2b: fstp DWORD PTR [ebp-0x174]
                mem.write(r.ebp.wrapping_sub(0x174), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4ab31: fld DWORD PTR [edi+0x140]
                st.push(ld32(mem, r.edi.wrapping_add(0x140)));
                // 0xa4ab37: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4ab39: fstp DWORD PTR [ebp-0x158]
                mem.write(r.ebp.wrapping_sub(0x158), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4ab3f: fld DWORD PTR [edi+0x144]
                st.push(ld32(mem, r.edi.wrapping_add(0x144)));
                // 0xa4ab45: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4ab47: fstp DWORD PTR [ebp-0x154]
                mem.write(r.ebp.wrapping_sub(0x154), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4ab4d: fld DWORD PTR [edi+0x170]
                st.push(ld32(mem, r.edi.wrapping_add(0x170)));
                // 0xa4ab53: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4ab55: fstp DWORD PTR [ebp-0x138]
                mem.write(r.ebp.wrapping_sub(0x138), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4ab5b: fmul DWORD PTR [edi+0x174]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, r.edi.wrapping_add(0x174)));
                }
                // 0xa4ab61: fstp DWORD PTR [ebp-0x134]
                mem.write(r.ebp.wrapping_sub(0x134), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4ab67: call 0x63e930
                {
                    let a: Vec<u32> = (0..3)
                        .map(|k| mem.read::<u32>(r.esp.wrapping_add(4 * k)))
                        .collect();
                    let f = mem.rebase(0x63e930);
                    r.eax = call(mem, f, &a);
                }
                // 0xa4ab6c: fld DWORD PTR [ebp-0x1b8]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x1b8)));
                // 0xa4ab72: mov ecx,DWORD PTR [ebp-0x254]
                r.ecx = mem.read::<u32>(r.ebp.wrapping_sub(0x254));
                // 0xa4ab78: fld QWORD PTR ds:0xb921e0
                st.push(mem.read::<f64>(mem.rebase(0xb921e0)));
                // 0xa4ab7e: mov edx,DWORD PTR [ebp-0x1b4]
                r.edx = mem.read::<u32>(r.ebp.wrapping_sub(0x1b4));
                // 0xa4ab84: fld QWORD PTR ds:0xb51bc0
                st.push(mem.read::<f64>(mem.rebase(0xb51bc0)));
                // 0xa4ab8a: mov esi,DWORD PTR [ebp-0x1b0]
                r.esi = mem.read::<u32>(r.ebp.wrapping_sub(0x1b0));
                // 0xa4ab90: fld QWORD PTR ds:0xb41d38
                st.push(mem.read::<f64>(mem.rebase(0xb41d38)));
                // 0xa4ab96: add esp,0x18
                {
                    let v = r.esp.wrapping_add(0x18u32);
                    r.esp = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0xa4ab99: fld QWORD PTR ds:0xb922f0
                st.push(mem.read::<f64>(mem.rebase(0xb922f0)));
                // 0xa4ab9f: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                pc = 0xa4aba1;
            }
            0xa4aba1 => {
                // 0xa4aba1: add edx,0x10
                {
                    let v = r.edx.wrapping_add(0x10u32);
                    r.edx = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0xa4aba4: dec esi
                {
                    let v = r.esi.wrapping_sub(1);
                    r.esi = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0xa4aba5: mov DWORD PTR [ebp-0x1b4],edx
                mem.write(r.ebp.wrapping_sub(0x1b4), r.edx);
                // 0xa4abab: mov DWORD PTR [ebp-0x1b0],esi
                mem.write(r.ebp.wrapping_sub(0x1b0), r.esi);
                // 0xa4abb1: jne 0xa4a137
                pc = if !r.zf() { 0xa4a137 } else { 0xa4abb7 };
            }
            0xa4abb7 => {
                // 0xa4abb7: mov eax,DWORD PTR [ebp-0x1ac]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x1ac));
                // 0xa4abbd: add eax,0x150
                {
                    let v = r.eax.wrapping_add(0x150u32);
                    r.eax = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0xa4abc2: dec DWORD PTR [ebp-0x1a8]
                {
                    let v = mem.read::<u32>(r.ebp.wrapping_sub(0x1a8)).wrapping_sub(1);
                    mem.write(r.ebp.wrapping_sub(0x1a8), v);
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0xa4abc8: mov DWORD PTR [ebp-0x1ac],eax
                mem.write(r.ebp.wrapping_sub(0x1ac), r.eax);
                // 0xa4abce: jne 0xa4a124
                pc = if !r.zf() { 0xa4a124 } else { 0xa4abd4 };
            }
            0xa4abd4 => {
                // 0xa4abd4: mov ecx,DWORD PTR [ebp-0x4]
                r.ecx = mem.read::<u32>(r.ebp.wrapping_sub(0x4));
                // 0xa4abd7: fstp st(3)
                {
                    let x = st0(&st);
                    set(&mut st, 3, x);
                }
                st.pop();
                // 0xa4abd9: fstp st(1)
                {
                    let x = st0(&st);
                    set(&mut st, 1, x);
                }
                st.pop();
                // 0xa4abdb: pop edi
                {
                    r.edi = mem.read::<u32>(r.esp);
                    r.esp = r.esp.wrapping_add(4);
                }
                // 0xa4abdc: fstp st(2)
                {
                    let x = st0(&st);
                    set(&mut st, 2, x);
                }
                st.pop();
                // 0xa4abde: xor ecx,ebp
                {
                    let v = r.ecx ^ r.ebp;
                    r.ecx = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0xa4abe0: fstp st(0)
                {
                    let x = st0(&st);
                    set(&mut st, 0, x);
                }
                st.pop();
                // 0xa4abe2: pop esi
                {
                    r.esi = mem.read::<u32>(r.esp);
                    r.esp = r.esp.wrapping_add(4);
                }
                // 0xa4abe3: fstp st(0)
                {
                    let x = st0(&st);
                    set(&mut st, 0, x);
                }
                st.pop();
                // 0xa4abe5: call 0x94923a
                // security cookie check
                pc = 0xa4abea;
            }
            0xa4abea => {
                // 0xa4abea: epilogue
                break;
            }

            _ => break,
        }
    }
    r.eax
}
/// `AbmapDrawMap()` (`0x00a4ce20`, name ours; from the ability map's draw
/// `0x00a54660`): the ability map's main pass: the node matrices
/// (`AbmapMakeNodeMatrix`), the nodes, links and cursor as GS packets at
/// `0x01a860ec` through the emulated VU0 registers (`Vec2SubScalarsFlags`),
/// and the element draws (`graphicDrawUIAbmapElement`). Lifted
/// instruction by instruction (state machine over an emulated frame,
/// [`WORK`] bytes); returns the original's `eax`.
#[allow(
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::float_cmp,
    clippy::cast_possible_truncation,
    clippy::unreadable_literal,
    clippy::match_same_arms,
    clippy::needless_late_init,
    clippy::eq_op,
    clippy::nonminimal_bool,
    clippy::if_not_else,
    unused_braces,
    unused_assignments
)]
pub fn abmap_draw_map<M, C>(mem: &mut M, work: u32, call: &mut C) -> u32
where
    M: Mem,
    C: FnMut(&mut M, u32, &[u32]) -> u32,
{
    let mut r = Regs {
        ebp: work.wrapping_add(0x3c0),
        esp: work.wrapping_add(0x3c0 - 0x354),
        cw: 0x027f,
        ..Regs::default()
    };
    let mut st: Vec<f64> = Vec::with_capacity(8);
    let mut pc: u32 = 0x00a4_ce29;
    loop {
        match pc {
            0xa4ce29 => {
                // 0xa4ce29: mov eax,ds:0xc613d8
                r.eax = mem.read::<u32>(mem.rebase(0xc613d8));
                // 0xa4ce2e: xor eax,ebp
                {
                    let v = r.eax ^ r.ebp;
                    r.eax = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0xa4ce30: mov DWORD PTR [ebp-0x4],eax
                mem.write(r.ebp.wrapping_sub(0x4), r.eax);
                // 0xa4ce33: fld DWORD PTR ds:0xb7b6f0
                st.push(ld32(mem, mem.rebase(0xb7b6f0)));
                // 0xa4ce39: push ebx
                {
                    let v = r.ebx;
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0xa4ce3a: fstp DWORD PTR [ebp-0x294]
                mem.write(r.ebp.wrapping_sub(0x294), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4ce40: push esi
                {
                    let v = r.esi;
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0xa4ce41: fld DWORD PTR ds:0xb7b6f4
                st.push(ld32(mem, mem.rebase(0xb7b6f4)));
                // 0xa4ce47: mov esi,DWORD PTR ds:0x2305834
                r.esi = mem.read::<u32>(mem.rebase(0x2305834));
                // 0xa4ce4d: fstp DWORD PTR [ebp-0x290]
                mem.write(r.ebp.wrapping_sub(0x290), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4ce53: push edi
                {
                    let v = r.edi;
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0xa4ce54: fldz
                st.push(0.0);
                // 0xa4ce56: movsx eax,WORD PTR [esi]
                r.eax = i32::from(mem.read::<i16>(r.esi)).cast_unsigned();
                // 0xa4ce59: fst DWORD PTR [ebp-0x288]
                mem.write(r.ebp.wrapping_sub(0x288), crate::x87::store(st0(&st)));
                // 0xa4ce5f: lea ecx,[esi+0x8]
                r.ecx = r.esi.wrapping_add(0x8);
                // 0xa4ce62: fst DWORD PTR [ebp-0x28c]
                mem.write(r.ebp.wrapping_sub(0x28c), crate::x87::store(st0(&st)));
                // 0xa4ce68: mov DWORD PTR [ebp-0x1a0],ecx
                mem.write(r.ebp.wrapping_sub(0x1a0), r.ecx);
                // 0xa4ce6e: fld DWORD PTR ds:0xb7b700
                st.push(ld32(mem, mem.rebase(0xb7b700)));
                // 0xa4ce74: mov DWORD PTR [ebp-0xc],0x0
                mem.write(r.ebp.wrapping_sub(0xc), 0x0u32);
                // 0xa4ce7b: fstp DWORD PTR [ebp-0x228]
                mem.write(r.ebp.wrapping_sub(0x228), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4ce81: mov DWORD PTR [ebp-0x8],0x0
                mem.write(r.ebp.wrapping_sub(0x8), 0x0u32);
                // 0xa4ce88: fld DWORD PTR ds:0xb7b6fc
                st.push(ld32(mem, mem.rebase(0xb7b6fc)));
                // 0xa4ce8e: mov DWORD PTR [ebp-0x238],0x40
                mem.write(r.ebp.wrapping_sub(0x238), 0x40u32);
                // 0xa4ce98: fstp DWORD PTR [ebp-0x224]
                mem.write(r.ebp.wrapping_sub(0x224), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4ce9e: mov DWORD PTR [ebp-0x23c],0x40
                mem.write(r.ebp.wrapping_sub(0x23c), 0x40u32);
                // 0xa4cea8: mov DWORD PTR [ebp-0x240],0x40
                mem.write(r.ebp.wrapping_sub(0x240), 0x40u32);
                // 0xa4ceb2: mov DWORD PTR [ebp-0x244],0x40
                mem.write(r.ebp.wrapping_sub(0x244), 0x40u32);
                // 0xa4cebc: fst DWORD PTR [ebp-0x21c]
                mem.write(r.ebp.wrapping_sub(0x21c), crate::x87::store(st0(&st)));
                // 0xa4cec2: fstp DWORD PTR [ebp-0x220]
                mem.write(r.ebp.wrapping_sub(0x220), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4cec8: test eax,eax
                r.flag = (r.eax & r.eax, 0, false);
                r.cmp = false;
                // 0xa4ceca: je 0xa4f239
                pc = if r.zf() { 0xa4f239 } else { 0xa4ced0 };
            }
            0xa4ced0 => {
                // 0xa4ced0: fld1
                st.push(1.0);
                // 0xa4ced2: dec eax
                {
                    let v = r.eax.wrapping_sub(1);
                    r.eax = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0xa4ced3: mov DWORD PTR [ebp-0x254],eax
                mem.write(r.ebp.wrapping_sub(0x254), r.eax);
                // 0xa4ced9: movzx eax,WORD PTR [ecx+0x6]
                r.eax = u32::from(mem.read::<u16>(r.ecx.wrapping_add(0x6)));
                // 0xa4cedd: push ecx
                {
                    let v = r.ecx;
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0xa4cede: mov eax,DWORD PTR [eax*4+0x1a85f78]
                r.eax = mem.read::<u32>(mem.rebase(0x1a85f78).wrapping_add(r.eax.wrapping_mul(4)));
                // 0xa4cee5: fstp DWORD PTR [esp]
                mem.write(r.esp, crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4cee8: push ecx
                {
                    let v = r.ecx;
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0xa4cee9: movsx ebx,WORD PTR [eax+0x10]
                r.ebx = i32::from(mem.read::<i16>(r.eax.wrapping_add(0x10))).cast_unsigned();
                // 0xa4ceed: add ebx,eax
                {
                    let v = r.ebx.wrapping_add(r.eax);
                    r.ebx = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0xa4ceef: lea eax,[ebp-0x218]
                r.eax = r.ebp.wrapping_sub(0x218);
                // 0xa4cef5: push eax
                {
                    let v = r.eax;
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0xa4cef6: mov DWORD PTR [ebp-0x14c],ebx
                mem.write(r.ebp.wrapping_sub(0x14c), r.ebx);
                // 0xa4cefc: call 0xa5ad30
                {
                    let a: Vec<u32> = (0..3)
                        .map(|k| mem.read::<u32>(r.esp.wrapping_add(4 * k)))
                        .collect();
                    let f = mem.rebase(0xa5ad30);
                    r.eax = call(mem, f, &a);
                }
                // 0xa4cf01: mov eax,DWORD PTR [esi+0x113e0]
                r.eax = mem.read::<u32>(r.esi.wrapping_add(0x113e0));
                // 0xa4cf07: mov ds:0xc8f910,eax
                mem.write(mem.rebase(0xc8f910), r.eax);
                // 0xa4cf0c: fld DWORD PTR [ebp-0x218]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x218)));
                // 0xa4cf12: mov eax,DWORD PTR [esi+0x113e4]
                r.eax = mem.read::<u32>(r.esi.wrapping_add(0x113e4));
                // 0xa4cf18: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4cf1a: mov ds:0xc8f914,eax
                mem.write(mem.rebase(0xc8f914), r.eax);
                // 0xa4cf1f: mov eax,DWORD PTR [esi+0x113e8]
                r.eax = mem.read::<u32>(r.esi.wrapping_add(0x113e8));
                // 0xa4cf25: mov ds:0xc8f918,eax
                mem.write(mem.rebase(0xc8f918), r.eax);
                // 0xa4cf2a: mov eax,DWORD PTR [esi+0x113ec]
                r.eax = mem.read::<u32>(r.esi.wrapping_add(0x113ec));
                // 0xa4cf30: mov ds:0xc8f91c,eax
                mem.write(mem.rebase(0xc8f91c), r.eax);
                // 0xa4cf35: mov eax,DWORD PTR [esi+0x113f0]
                r.eax = mem.read::<u32>(r.esi.wrapping_add(0x113f0));
                // 0xa4cf3b: mov ds:0xc8f920,eax
                mem.write(mem.rebase(0xc8f920), r.eax);
                // 0xa4cf40: mov eax,DWORD PTR [esi+0x113f4]
                r.eax = mem.read::<u32>(r.esi.wrapping_add(0x113f4));
                // 0xa4cf46: mov ds:0xc8f924,eax
                mem.write(mem.rebase(0xc8f924), r.eax);
                // 0xa4cf4b: mov eax,DWORD PTR [esi+0x113f8]
                r.eax = mem.read::<u32>(r.esi.wrapping_add(0x113f8));
                // 0xa4cf51: mov ds:0xc8f928,eax
                mem.write(mem.rebase(0xc8f928), r.eax);
                // 0xa4cf56: mov eax,DWORD PTR [esi+0x113fc]
                r.eax = mem.read::<u32>(r.esi.wrapping_add(0x113fc));
                // 0xa4cf5c: mov ds:0xc8f92c,eax
                mem.write(mem.rebase(0xc8f92c), r.eax);
                // 0xa4cf61: mov eax,DWORD PTR [esi+0x11400]
                r.eax = mem.read::<u32>(r.esi.wrapping_add(0x11400));
                // 0xa4cf67: mov ds:0xc8f930,eax
                mem.write(mem.rebase(0xc8f930), r.eax);
                // 0xa4cf6c: mov eax,DWORD PTR [esi+0x11404]
                r.eax = mem.read::<u32>(r.esi.wrapping_add(0x11404));
                // 0xa4cf72: mov ds:0xc8f934,eax
                mem.write(mem.rebase(0xc8f934), r.eax);
                // 0xa4cf77: mov eax,DWORD PTR [esi+0x11408]
                r.eax = mem.read::<u32>(r.esi.wrapping_add(0x11408));
                // 0xa4cf7d: mov ds:0xc8f938,eax
                mem.write(mem.rebase(0xc8f938), r.eax);
                // 0xa4cf82: mov eax,DWORD PTR [esi+0x1140c]
                r.eax = mem.read::<u32>(r.esi.wrapping_add(0x1140c));
                // 0xa4cf88: mov ds:0xc8f93c,eax
                mem.write(mem.rebase(0xc8f93c), r.eax);
                // 0xa4cf8d: mov eax,DWORD PTR [esi+0x11410]
                r.eax = mem.read::<u32>(r.esi.wrapping_add(0x11410));
                // 0xa4cf93: mov ds:0xc8f940,eax
                mem.write(mem.rebase(0xc8f940), r.eax);
                // 0xa4cf98: mov eax,DWORD PTR [esi+0x11414]
                r.eax = mem.read::<u32>(r.esi.wrapping_add(0x11414));
                // 0xa4cf9e: mov ds:0xc8f944,eax
                mem.write(mem.rebase(0xc8f944), r.eax);
                // 0xa4cfa3: mov eax,DWORD PTR [esi+0x11418]
                r.eax = mem.read::<u32>(r.esi.wrapping_add(0x11418));
                // 0xa4cfa9: mov ds:0xc8f948,eax
                mem.write(mem.rebase(0xc8f948), r.eax);
                // 0xa4cfae: mov eax,DWORD PTR [esi+0x1141c]
                r.eax = mem.read::<u32>(r.esi.wrapping_add(0x1141c));
                // 0xa4cfb4: mov ds:0xc8f94c,eax
                mem.write(mem.rebase(0xc8f94c), r.eax);
                // 0xa4cfb9: mov eax,DWORD PTR [ebp-0x218]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x218));
                // 0xa4cfbf: mov ds:0xc8f950,eax
                mem.write(mem.rebase(0xc8f950), r.eax);
                // 0xa4cfc4: mov eax,DWORD PTR [ebp-0x214]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x214));
                // 0xa4cfca: mov ds:0xc8f954,eax
                mem.write(mem.rebase(0xc8f954), r.eax);
                // 0xa4cfcf: mov eax,DWORD PTR [ebp-0x210]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x210));
                // 0xa4cfd5: mov ds:0xc8f958,eax
                mem.write(mem.rebase(0xc8f958), r.eax);
                // 0xa4cfda: mov eax,DWORD PTR [ebp-0x20c]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x20c));
                // 0xa4cfe0: mov ds:0xc8f95c,eax
                mem.write(mem.rebase(0xc8f95c), r.eax);
                // 0xa4cfe5: mov eax,DWORD PTR [ebp-0x208]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x208));
                // 0xa4cfeb: mov ds:0xc8f960,eax
                mem.write(mem.rebase(0xc8f960), r.eax);
                // 0xa4cff0: mov eax,DWORD PTR [ebp-0x204]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x204));
                // 0xa4cff6: mov ds:0xc8f964,eax
                mem.write(mem.rebase(0xc8f964), r.eax);
                // 0xa4cffb: mov eax,DWORD PTR [ebp-0x200]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x200));
                // 0xa4d001: mov ds:0xc8f968,eax
                mem.write(mem.rebase(0xc8f968), r.eax);
                // 0xa4d006: mov eax,DWORD PTR [ebp-0x1fc]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x1fc));
                // 0xa4d00c: mov ds:0xc8f96c,eax
                mem.write(mem.rebase(0xc8f96c), r.eax);
                // 0xa4d011: mov eax,DWORD PTR [ebp-0x1f8]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x1f8));
                // 0xa4d017: mov ds:0xc8f970,eax
                mem.write(mem.rebase(0xc8f970), r.eax);
                // 0xa4d01c: mov eax,DWORD PTR [ebp-0x1f4]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x1f4));
                // 0xa4d022: mov ds:0xc8f974,eax
                mem.write(mem.rebase(0xc8f974), r.eax);
                // 0xa4d027: mov eax,DWORD PTR [ebp-0x1f0]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x1f0));
                // 0xa4d02d: mov ds:0xc8f978,eax
                mem.write(mem.rebase(0xc8f978), r.eax);
                // 0xa4d032: mov eax,DWORD PTR [ebp-0x1ec]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x1ec));
                // 0xa4d038: mov ds:0xc8f97c,eax
                mem.write(mem.rebase(0xc8f97c), r.eax);
                // 0xa4d03d: mov eax,DWORD PTR [ebp-0x1e8]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x1e8));
                // 0xa4d043: mov ds:0xc8f980,eax
                mem.write(mem.rebase(0xc8f980), r.eax);
                // 0xa4d048: mov eax,DWORD PTR [ebp-0x1e4]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x1e4));
                // 0xa4d04e: mov ds:0xc8f984,eax
                mem.write(mem.rebase(0xc8f984), r.eax);
                // 0xa4d053: mov eax,DWORD PTR [ebp-0x1e0]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x1e0));
                // 0xa4d059: mov ds:0xc8f988,eax
                mem.write(mem.rebase(0xc8f988), r.eax);
                // 0xa4d05e: mov eax,DWORD PTR [ebp-0x1dc]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x1dc));
                // 0xa4d064: add esp,0xc
                {
                    let v = r.esp.wrapping_add(0xcu32);
                    r.esp = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0xa4d067: mov ds:0xc8f98c,eax
                mem.write(mem.rebase(0xc8f98c), r.eax);
                // 0xa4d06c: fmul DWORD PTR ds:0xc8f910
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f910)));
                }
                // 0xa4d072: fstp DWORD PTR ds:0xc8f790
                mem.write(mem.rebase(0xc8f790), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d078: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4d07a: fmul DWORD PTR ds:0xc8f914
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f914)));
                }
                // 0xa4d080: fstp DWORD PTR ds:0xc8f794
                mem.write(mem.rebase(0xc8f794), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d086: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4d088: fmul DWORD PTR ds:0xc8f918
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f918)));
                }
                // 0xa4d08e: fstp DWORD PTR ds:0xc8f798
                mem.write(mem.rebase(0xc8f798), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d094: fmul DWORD PTR ds:0xc8f91c
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f91c)));
                }
                // 0xa4d09a: fstp DWORD PTR ds:0xc8f79c
                mem.write(mem.rebase(0xc8f79c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d0a0: fld DWORD PTR ds:0xc8f920
                st.push(ld32(mem, mem.rebase(0xc8f920)));
                // 0xa4d0a6: fld DWORD PTR [ebp-0x214]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x214)));
                // 0xa4d0ac: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4d0ae: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0xa4d0b0: fld DWORD PTR ds:0xc8f790
                st.push(ld32(mem, mem.rebase(0xc8f790)));
                // 0xa4d0b6: faddp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x + y);
                }
                st.pop();
                // 0xa4d0b8: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4d0ba: fstp DWORD PTR ds:0xc8f790
                mem.write(mem.rebase(0xc8f790), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d0c0: fld DWORD PTR ds:0xc8f924
                st.push(ld32(mem, mem.rebase(0xc8f924)));
                // 0xa4d0c6: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4d0c8: fadd DWORD PTR ds:0xc8f794
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f794)));
                }
                // 0xa4d0ce: fstp DWORD PTR ds:0xc8f794
                mem.write(mem.rebase(0xc8f794), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d0d4: fld DWORD PTR ds:0xc8f928
                st.push(ld32(mem, mem.rebase(0xc8f928)));
                // 0xa4d0da: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4d0dc: fadd DWORD PTR ds:0xc8f798
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f798)));
                }
                // 0xa4d0e2: fstp DWORD PTR ds:0xc8f798
                mem.write(mem.rebase(0xc8f798), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d0e8: fmul DWORD PTR ds:0xc8f92c
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f92c)));
                }
                // 0xa4d0ee: fadd DWORD PTR ds:0xc8f79c
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f79c)));
                }
                // 0xa4d0f4: fstp DWORD PTR ds:0xc8f79c
                mem.write(mem.rebase(0xc8f79c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d0fa: fld DWORD PTR ds:0xc8f930
                st.push(ld32(mem, mem.rebase(0xc8f930)));
                // 0xa4d100: fld DWORD PTR [ebp-0x210]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x210)));
                // 0xa4d106: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4d108: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0xa4d10a: fld DWORD PTR ds:0xc8f790
                st.push(ld32(mem, mem.rebase(0xc8f790)));
                // 0xa4d110: faddp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x + y);
                }
                st.pop();
                // 0xa4d112: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4d114: fstp DWORD PTR ds:0xc8f790
                mem.write(mem.rebase(0xc8f790), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d11a: fld DWORD PTR ds:0xc8f934
                st.push(ld32(mem, mem.rebase(0xc8f934)));
                // 0xa4d120: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4d122: fadd DWORD PTR ds:0xc8f794
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f794)));
                }
                // 0xa4d128: fstp DWORD PTR ds:0xc8f794
                mem.write(mem.rebase(0xc8f794), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d12e: fld DWORD PTR ds:0xc8f938
                st.push(ld32(mem, mem.rebase(0xc8f938)));
                // 0xa4d134: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4d136: fadd DWORD PTR ds:0xc8f798
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f798)));
                }
                // 0xa4d13c: fstp DWORD PTR ds:0xc8f798
                mem.write(mem.rebase(0xc8f798), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d142: fld DWORD PTR ds:0xc8f93c
                st.push(ld32(mem, mem.rebase(0xc8f93c)));
                // 0xa4d148: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4d14a: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0xa4d14c: fld DWORD PTR ds:0xc8f79c
                st.push(ld32(mem, mem.rebase(0xc8f79c)));
                // 0xa4d152: faddp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x + y);
                }
                st.pop();
                // 0xa4d154: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4d156: fstp DWORD PTR ds:0xc8f79c
                mem.write(mem.rebase(0xc8f79c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d15c: fld DWORD PTR ds:0xc8f940
                st.push(ld32(mem, mem.rebase(0xc8f940)));
                // 0xa4d162: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4d164: fld DWORD PTR [ebp-0x20c]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x20c)));
                // 0xa4d16a: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4d16c: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0xa4d16e: fld DWORD PTR ds:0xc8f790
                st.push(ld32(mem, mem.rebase(0xc8f790)));
                // 0xa4d174: faddp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x + y);
                }
                st.pop();
                // 0xa4d176: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4d178: fstp DWORD PTR ds:0xc8f950
                mem.write(mem.rebase(0xc8f950), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d17e: fld DWORD PTR ds:0xc8f944
                st.push(ld32(mem, mem.rebase(0xc8f944)));
                // 0xa4d184: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4d186: fmul st,st(2)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 2));
                    set(&mut st, 0, x * y);
                }
                // 0xa4d188: fadd DWORD PTR ds:0xc8f794
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f794)));
                }
                // 0xa4d18e: fstp DWORD PTR ds:0xc8f954
                mem.write(mem.rebase(0xc8f954), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d194: fld DWORD PTR ds:0xc8f948
                st.push(ld32(mem, mem.rebase(0xc8f948)));
                // 0xa4d19a: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4d19c: fmul st,st(3)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 3));
                    set(&mut st, 0, x * y);
                }
                // 0xa4d19e: fadd DWORD PTR ds:0xc8f798
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f798)));
                }
                // 0xa4d1a4: fstp DWORD PTR ds:0xc8f958
                mem.write(mem.rebase(0xc8f958), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d1aa: fld DWORD PTR ds:0xc8f94c
                st.push(ld32(mem, mem.rebase(0xc8f94c)));
                // 0xa4d1b0: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4d1b2: fmulp st(4),st
                {
                    let (x, y) = (sti(&st, 4), sti(&st, 0));
                    set(&mut st, 4, x * y);
                }
                st.pop();
                // 0xa4d1b4: fld DWORD PTR ds:0xc8f79c
                st.push(ld32(mem, mem.rebase(0xc8f79c)));
                // 0xa4d1ba: faddp st(4),st
                {
                    let (x, y) = (sti(&st, 4), sti(&st, 0));
                    set(&mut st, 4, x + y);
                }
                st.pop();
                // 0xa4d1bc: fxch st(3)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 3);
                }
                // 0xa4d1be: fstp DWORD PTR ds:0xc8f95c
                mem.write(mem.rebase(0xc8f95c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d1c4: fld DWORD PTR [ebp-0x208]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x208)));
                // 0xa4d1ca: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4d1cc: fmul DWORD PTR ds:0xc8f910
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f910)));
                }
                // 0xa4d1d2: fstp DWORD PTR ds:0xc8f790
                mem.write(mem.rebase(0xc8f790), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d1d8: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4d1da: fmul DWORD PTR ds:0xc8f914
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f914)));
                }
                // 0xa4d1e0: fstp DWORD PTR ds:0xc8f794
                mem.write(mem.rebase(0xc8f794), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d1e6: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4d1e8: fmul DWORD PTR ds:0xc8f918
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f918)));
                }
                // 0xa4d1ee: fstp DWORD PTR ds:0xc8f798
                mem.write(mem.rebase(0xc8f798), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d1f4: fmul DWORD PTR ds:0xc8f91c
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f91c)));
                }
                // 0xa4d1fa: fstp DWORD PTR ds:0xc8f79c
                mem.write(mem.rebase(0xc8f79c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d200: fld DWORD PTR ds:0xc8f920
                st.push(ld32(mem, mem.rebase(0xc8f920)));
                // 0xa4d206: fld DWORD PTR [ebp-0x204]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x204)));
                // 0xa4d20c: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4d20e: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0xa4d210: fld DWORD PTR ds:0xc8f790
                st.push(ld32(mem, mem.rebase(0xc8f790)));
                // 0xa4d216: faddp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x + y);
                }
                st.pop();
                // 0xa4d218: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4d21a: fstp DWORD PTR ds:0xc8f790
                mem.write(mem.rebase(0xc8f790), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d220: fld DWORD PTR ds:0xc8f924
                st.push(ld32(mem, mem.rebase(0xc8f924)));
                // 0xa4d226: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4d228: fadd DWORD PTR ds:0xc8f794
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f794)));
                }
                // 0xa4d22e: fstp DWORD PTR ds:0xc8f794
                mem.write(mem.rebase(0xc8f794), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d234: fld DWORD PTR ds:0xc8f928
                st.push(ld32(mem, mem.rebase(0xc8f928)));
                // 0xa4d23a: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4d23c: fadd DWORD PTR ds:0xc8f798
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f798)));
                }
                // 0xa4d242: fstp DWORD PTR ds:0xc8f798
                mem.write(mem.rebase(0xc8f798), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d248: fmul DWORD PTR ds:0xc8f92c
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f92c)));
                }
                // 0xa4d24e: fadd DWORD PTR ds:0xc8f79c
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f79c)));
                }
                // 0xa4d254: fstp DWORD PTR ds:0xc8f79c
                mem.write(mem.rebase(0xc8f79c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d25a: fld DWORD PTR ds:0xc8f930
                st.push(ld32(mem, mem.rebase(0xc8f930)));
                // 0xa4d260: fld DWORD PTR [ebp-0x200]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x200)));
                // 0xa4d266: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4d268: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0xa4d26a: fld DWORD PTR ds:0xc8f790
                st.push(ld32(mem, mem.rebase(0xc8f790)));
                // 0xa4d270: faddp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x + y);
                }
                st.pop();
                // 0xa4d272: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4d274: fstp DWORD PTR ds:0xc8f790
                mem.write(mem.rebase(0xc8f790), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d27a: fld DWORD PTR ds:0xc8f934
                st.push(ld32(mem, mem.rebase(0xc8f934)));
                // 0xa4d280: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4d282: fadd DWORD PTR ds:0xc8f794
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f794)));
                }
                // 0xa4d288: fstp DWORD PTR ds:0xc8f794
                mem.write(mem.rebase(0xc8f794), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d28e: fld DWORD PTR ds:0xc8f938
                st.push(ld32(mem, mem.rebase(0xc8f938)));
                // 0xa4d294: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4d296: fadd DWORD PTR ds:0xc8f798
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f798)));
                }
                // 0xa4d29c: fstp DWORD PTR ds:0xc8f798
                mem.write(mem.rebase(0xc8f798), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d2a2: fmul st,st(5)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 5));
                    set(&mut st, 0, x * y);
                }
                // 0xa4d2a4: fadd DWORD PTR ds:0xc8f79c
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f79c)));
                }
                // 0xa4d2aa: fstp DWORD PTR ds:0xc8f79c
                mem.write(mem.rebase(0xc8f79c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d2b0: fld st(3)
                {
                    let x = sti(&st, 3);
                    st.push(x);
                }
                // 0xa4d2b2: fld DWORD PTR [ebp-0x1fc]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x1fc)));
                // 0xa4d2b8: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4d2ba: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0xa4d2bc: fld DWORD PTR ds:0xc8f790
                st.push(ld32(mem, mem.rebase(0xc8f790)));
                // 0xa4d2c2: faddp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x + y);
                }
                st.pop();
                // 0xa4d2c4: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4d2c6: fstp DWORD PTR ds:0xc8f960
                mem.write(mem.rebase(0xc8f960), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d2cc: fld st(2)
                {
                    let x = sti(&st, 2);
                    st.push(x);
                }
                // 0xa4d2ce: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4d2d0: fadd DWORD PTR ds:0xc8f794
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f794)));
                }
                // 0xa4d2d6: fstp DWORD PTR ds:0xc8f964
                mem.write(mem.rebase(0xc8f964), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d2dc: fld st(1)
                {
                    let x = sti(&st, 1);
                    st.push(x);
                }
                // 0xa4d2de: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4d2e0: fadd DWORD PTR ds:0xc8f798
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f798)));
                }
                // 0xa4d2e6: fstp DWORD PTR ds:0xc8f968
                mem.write(mem.rebase(0xc8f968), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d2ec: fmul st,st(3)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 3));
                    set(&mut st, 0, x * y);
                }
                // 0xa4d2ee: fadd DWORD PTR ds:0xc8f79c
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f79c)));
                }
                // 0xa4d2f4: fstp DWORD PTR ds:0xc8f96c
                mem.write(mem.rebase(0xc8f96c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d2fa: fld DWORD PTR [ebp-0x1f8]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x1f8)));
                // 0xa4d300: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4d302: fmul DWORD PTR ds:0xc8f910
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f910)));
                }
                // 0xa4d308: fstp DWORD PTR ds:0xc8f790
                mem.write(mem.rebase(0xc8f790), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d30e: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4d310: fmul DWORD PTR ds:0xc8f914
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f914)));
                }
                // 0xa4d316: fstp DWORD PTR ds:0xc8f794
                mem.write(mem.rebase(0xc8f794), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d31c: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4d31e: fmul DWORD PTR ds:0xc8f918
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f918)));
                }
                // 0xa4d324: fstp DWORD PTR ds:0xc8f798
                mem.write(mem.rebase(0xc8f798), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d32a: fmul DWORD PTR ds:0xc8f91c
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f91c)));
                }
                // 0xa4d330: fstp DWORD PTR ds:0xc8f79c
                mem.write(mem.rebase(0xc8f79c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d336: fld DWORD PTR ds:0xc8f920
                st.push(ld32(mem, mem.rebase(0xc8f920)));
                // 0xa4d33c: fld DWORD PTR [ebp-0x1f4]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x1f4)));
                // 0xa4d342: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4d344: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0xa4d346: fld DWORD PTR ds:0xc8f790
                st.push(ld32(mem, mem.rebase(0xc8f790)));
                // 0xa4d34c: faddp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x + y);
                }
                st.pop();
                // 0xa4d34e: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4d350: fstp DWORD PTR ds:0xc8f790
                mem.write(mem.rebase(0xc8f790), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d356: fld DWORD PTR ds:0xc8f924
                st.push(ld32(mem, mem.rebase(0xc8f924)));
                // 0xa4d35c: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4d35e: fadd DWORD PTR ds:0xc8f794
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f794)));
                }
                // 0xa4d364: fstp DWORD PTR ds:0xc8f794
                mem.write(mem.rebase(0xc8f794), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d36a: fld DWORD PTR ds:0xc8f928
                st.push(ld32(mem, mem.rebase(0xc8f928)));
                // 0xa4d370: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4d372: fadd DWORD PTR ds:0xc8f798
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f798)));
                }
                // 0xa4d378: fstp DWORD PTR ds:0xc8f798
                mem.write(mem.rebase(0xc8f798), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d37e: fmul DWORD PTR ds:0xc8f92c
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f92c)));
                }
                // 0xa4d384: fadd DWORD PTR ds:0xc8f79c
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f79c)));
                }
                // 0xa4d38a: fstp DWORD PTR ds:0xc8f79c
                mem.write(mem.rebase(0xc8f79c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d390: fld DWORD PTR [ebp-0x1f0]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x1f0)));
                // 0xa4d396: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4d398: fmul DWORD PTR ds:0xc8f930
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f930)));
                }
                // 0xa4d39e: fadd DWORD PTR ds:0xc8f790
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f790)));
                }
                // 0xa4d3a4: fstp DWORD PTR ds:0xc8f790
                mem.write(mem.rebase(0xc8f790), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d3aa: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4d3ac: fmul DWORD PTR ds:0xc8f934
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f934)));
                }
                // 0xa4d3b2: fadd DWORD PTR ds:0xc8f794
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f794)));
                }
                // 0xa4d3b8: fstp DWORD PTR ds:0xc8f794
                mem.write(mem.rebase(0xc8f794), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d3be: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4d3c0: fmul DWORD PTR ds:0xc8f938
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f938)));
                }
                // 0xa4d3c6: fadd DWORD PTR ds:0xc8f798
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f798)));
                }
                // 0xa4d3cc: fstp DWORD PTR ds:0xc8f798
                mem.write(mem.rebase(0xc8f798), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d3d2: fmul st,st(5)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 5));
                    set(&mut st, 0, x * y);
                }
                // 0xa4d3d4: fadd DWORD PTR ds:0xc8f79c
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f79c)));
                }
                // 0xa4d3da: fstp DWORD PTR ds:0xc8f79c
                mem.write(mem.rebase(0xc8f79c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d3e0: fld DWORD PTR [ebp-0x1ec]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x1ec)));
                // 0xa4d3e6: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4d3e8: fmul st,st(5)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 5));
                    set(&mut st, 0, x * y);
                }
                // 0xa4d3ea: fadd DWORD PTR ds:0xc8f790
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f790)));
                }
                // 0xa4d3f0: fstp DWORD PTR ds:0xc8f970
                mem.write(mem.rebase(0xc8f970), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d3f6: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4d3f8: fmul st,st(3)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 3));
                    set(&mut st, 0, x * y);
                }
                // 0xa4d3fa: fadd DWORD PTR ds:0xc8f794
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f794)));
                }
                // 0xa4d400: fstp DWORD PTR ds:0xc8f974
                mem.write(mem.rebase(0xc8f974), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d406: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4d408: fmul st,st(2)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 2));
                    set(&mut st, 0, x * y);
                }
                // 0xa4d40a: fadd DWORD PTR ds:0xc8f798
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f798)));
                }
                // 0xa4d410: fstp DWORD PTR ds:0xc8f978
                mem.write(mem.rebase(0xc8f978), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d416: fmul st,st(3)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 3));
                    set(&mut st, 0, x * y);
                }
                // 0xa4d418: fadd DWORD PTR ds:0xc8f79c
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f79c)));
                }
                // 0xa4d41e: fstp DWORD PTR ds:0xc8f97c
                mem.write(mem.rebase(0xc8f97c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d424: fld DWORD PTR [ebp-0x1e8]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x1e8)));
                // 0xa4d42a: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4d42c: fmul DWORD PTR ds:0xc8f910
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f910)));
                }
                // 0xa4d432: fstp DWORD PTR ds:0xc8f790
                mem.write(mem.rebase(0xc8f790), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d438: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4d43a: fmul DWORD PTR ds:0xc8f914
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f914)));
                }
                // 0xa4d440: fstp DWORD PTR ds:0xc8f794
                mem.write(mem.rebase(0xc8f794), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d446: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4d448: fmul DWORD PTR ds:0xc8f918
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f918)));
                }
                // 0xa4d44e: fstp DWORD PTR ds:0xc8f798
                mem.write(mem.rebase(0xc8f798), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d454: fmul DWORD PTR ds:0xc8f91c
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f91c)));
                }
                // 0xa4d45a: fstp DWORD PTR ds:0xc8f79c
                mem.write(mem.rebase(0xc8f79c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d460: fld DWORD PTR [ebp-0x1e4]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x1e4)));
                // 0xa4d466: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4d468: fmul DWORD PTR ds:0xc8f920
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f920)));
                }
                // 0xa4d46e: fadd DWORD PTR ds:0xc8f790
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f790)));
                }
                // 0xa4d474: fstp DWORD PTR ds:0xc8f790
                mem.write(mem.rebase(0xc8f790), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d47a: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4d47c: fmul DWORD PTR ds:0xc8f924
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f924)));
                }
                // 0xa4d482: fadd DWORD PTR ds:0xc8f794
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f794)));
                }
                // 0xa4d488: fstp DWORD PTR ds:0xc8f794
                mem.write(mem.rebase(0xc8f794), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d48e: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4d490: fmul DWORD PTR ds:0xc8f928
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f928)));
                }
                // 0xa4d496: fadd DWORD PTR ds:0xc8f798
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f798)));
                }
                // 0xa4d49c: fstp DWORD PTR ds:0xc8f798
                mem.write(mem.rebase(0xc8f798), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d4a2: fmul DWORD PTR ds:0xc8f92c
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f92c)));
                }
                // 0xa4d4a8: fadd DWORD PTR ds:0xc8f79c
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f79c)));
                }
                // 0xa4d4ae: fstp DWORD PTR ds:0xc8f79c
                mem.write(mem.rebase(0xc8f79c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d4b4: fld DWORD PTR [ebp-0x1e0]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x1e0)));
                // 0xa4d4ba: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4d4bc: fmul DWORD PTR ds:0xc8f930
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f930)));
                }
                // 0xa4d4c2: fadd DWORD PTR ds:0xc8f790
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f790)));
                }
                // 0xa4d4c8: fstp DWORD PTR ds:0xc8f790
                mem.write(mem.rebase(0xc8f790), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d4ce: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4d4d0: fmul DWORD PTR ds:0xc8f934
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f934)));
                }
                // 0xa4d4d6: fadd DWORD PTR ds:0xc8f794
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f794)));
                }
                // 0xa4d4dc: fstp DWORD PTR ds:0xc8f794
                mem.write(mem.rebase(0xc8f794), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d4e2: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4d4e4: fmul DWORD PTR ds:0xc8f938
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f938)));
                }
                // 0xa4d4ea: fadd DWORD PTR ds:0xc8f798
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f798)));
                }
                // 0xa4d4f0: fstp DWORD PTR ds:0xc8f798
                mem.write(mem.rebase(0xc8f798), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d4f6: fmulp st(5),st
                {
                    let (x, y) = (sti(&st, 5), sti(&st, 0));
                    set(&mut st, 5, x * y);
                }
                st.pop();
                // 0xa4d4f8: fld DWORD PTR ds:0xc8f79c
                st.push(ld32(mem, mem.rebase(0xc8f79c)));
                // 0xa4d4fe: faddp st(5),st
                {
                    let (x, y) = (sti(&st, 5), sti(&st, 0));
                    set(&mut st, 5, x + y);
                }
                st.pop();
                // 0xa4d500: fxch st(4)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 4);
                }
                // 0xa4d502: fstp DWORD PTR ds:0xc8f79c
                mem.write(mem.rebase(0xc8f79c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d508: fld DWORD PTR [ebp-0x1dc]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x1dc)));
                // 0xa4d50e: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4d510: fmulp st(4),st
                {
                    let (x, y) = (sti(&st, 4), sti(&st, 0));
                    set(&mut st, 4, x * y);
                }
                st.pop();
                // 0xa4d512: fld DWORD PTR ds:0xc8f790
                st.push(ld32(mem, mem.rebase(0xc8f790)));
                // 0xa4d518: faddp st(4),st
                {
                    let (x, y) = (sti(&st, 4), sti(&st, 0));
                    set(&mut st, 4, x + y);
                }
                st.pop();
                // 0xa4d51a: fxch st(3)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 3);
                }
                // 0xa4d51c: fstp DWORD PTR ds:0xc8f980
                mem.write(mem.rebase(0xc8f980), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d522: fmul st,st(2)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 2));
                    set(&mut st, 0, x * y);
                }
                // 0xa4d524: fadd DWORD PTR ds:0xc8f794
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f794)));
                }
                // 0xa4d52a: fstp DWORD PTR ds:0xc8f984
                mem.write(mem.rebase(0xc8f984), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d530: fld st(1)
                {
                    let x = sti(&st, 1);
                    st.push(x);
                }
                // 0xa4d532: fmulp st(3),st
                {
                    let (x, y) = (sti(&st, 3), sti(&st, 0));
                    set(&mut st, 3, x * y);
                }
                st.pop();
                // 0xa4d534: fld DWORD PTR ds:0xc8f798
                st.push(ld32(mem, mem.rebase(0xc8f798)));
                // 0xa4d53a: faddp st(3),st
                {
                    let (x, y) = (sti(&st, 3), sti(&st, 0));
                    set(&mut st, 3, x + y);
                }
                st.pop();
                // 0xa4d53c: fxch st(2)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 2);
                }
                // 0xa4d53e: fstp DWORD PTR ds:0xc8f988
                mem.write(mem.rebase(0xc8f988), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d544: fmulp st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, x * y);
                }
                st.pop();
                // 0xa4d546: fadd DWORD PTR ds:0xc8f79c
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f79c)));
                }
                // 0xa4d54c: fstp DWORD PTR ds:0xc8f98c
                mem.write(mem.rebase(0xc8f98c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d552: movsx eax,WORD PTR [ebx+0x20]
                r.eax = i32::from(mem.read::<i16>(r.ebx.wrapping_add(0x20))).cast_unsigned();
                // 0xa4d556: cdq
                r.edx = if r.eax.cast_signed() < 0 { u32::MAX } else { 0 };
                // 0xa4d557: mov esi,eax
                r.esi = r.eax;
                // 0xa4d559: mov edi,edx
                r.edi = r.edx;
                // 0xa4d55b: mov DWORD PTR ds:0xc8f588,esi
                mem.write(mem.rebase(0xc8f588), r.esi);
                // 0xa4d561: mov DWORD PTR ds:0xc8f58c,edi
                mem.write(mem.rebase(0xc8f58c), r.edi);
                // 0xa4d567: movsx eax,WORD PTR [ebx+0x22]
                r.eax = i32::from(mem.read::<i16>(r.ebx.wrapping_add(0x22))).cast_unsigned();
                // 0xa4d56b: cdq
                r.edx = if r.eax.cast_signed() < 0 { u32::MAX } else { 0 };
                // 0xa4d56c: mov ecx,eax
                r.ecx = r.eax;
                // 0xa4d56e: mov eax,ds:0xc8f5a0
                r.eax = mem.read::<u32>(mem.rebase(0xc8f5a0));
                // 0xa4d573: mov DWORD PTR [ebp-0x29c],eax
                mem.write(r.ebp.wrapping_sub(0x29c), r.eax);
                // 0xa4d579: mov eax,ds:0xc8f5a4
                r.eax = mem.read::<u32>(mem.rebase(0xc8f5a4));
                // 0xa4d57e: mov DWORD PTR [ebp-0x298],eax
                mem.write(r.ebp.wrapping_sub(0x298), r.eax);
                // 0xa4d584: mov eax,ds:0xc8f590
                r.eax = mem.read::<u32>(mem.rebase(0xc8f590));
                // 0xa4d589: mov DWORD PTR [ebp-0x32c],eax
                mem.write(r.ebp.wrapping_sub(0x32c), r.eax);
                // 0xa4d58f: mov eax,ds:0xc8f594
                r.eax = mem.read::<u32>(mem.rebase(0xc8f594));
                // 0xa4d594: mov DWORD PTR ds:0xc8f588,esi
                mem.write(mem.rebase(0xc8f588), r.esi);
                // 0xa4d59a: mov DWORD PTR [ebp-0x328],eax
                mem.write(r.ebp.wrapping_sub(0x328), r.eax);
                // 0xa4d5a0: fld DWORD PTR ds:0xc8f588
                st.push(ld32(mem, mem.rebase(0xc8f588)));
                // 0xa4d5a6: mov DWORD PTR ds:0xc8f58c,ecx
                mem.write(mem.rebase(0xc8f58c), r.ecx);
                // 0xa4d5ac: fstp DWORD PTR ds:0xc8f7d0
                mem.write(mem.rebase(0xc8f7d0), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d5b2: mov eax,ds:0xc8f7d0
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d0));
                // 0xa4d5b7: fld DWORD PTR ds:0xc8f58c
                st.push(ld32(mem, mem.rebase(0xc8f58c)));
                // 0xa4d5bd: mov DWORD PTR [ebp-0xac],eax
                mem.write(r.ebp.wrapping_sub(0xac), r.eax);
                // 0xa4d5c3: fstp DWORD PTR ds:0xc8f7d4
                mem.write(mem.rebase(0xc8f7d4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d5c9: mov eax,ds:0xc8f7d4
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d4));
                // 0xa4d5ce: mov DWORD PTR ds:0xc8f590,edi
                mem.write(mem.rebase(0xc8f590), r.edi);
                // 0xa4d5d4: mov DWORD PTR ds:0xc8f594,edx
                mem.write(mem.rebase(0xc8f594), r.edx);
                // 0xa4d5da: fld DWORD PTR ds:0xc8f590
                st.push(ld32(mem, mem.rebase(0xc8f590)));
                // 0xa4d5e0: mov DWORD PTR [ebp-0xa8],eax
                mem.write(r.ebp.wrapping_sub(0xa8), r.eax);
                // 0xa4d5e6: fstp DWORD PTR ds:0xc8f7d8
                mem.write(mem.rebase(0xc8f7d8), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d5ec: mov eax,ds:0xc8f7d8
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d8));
                // 0xa4d5f1: fld DWORD PTR ds:0xc8f594
                st.push(ld32(mem, mem.rebase(0xc8f594)));
                // 0xa4d5f7: mov DWORD PTR [ebp-0xa4],eax
                mem.write(r.ebp.wrapping_sub(0xa4), r.eax);
                // 0xa4d5fd: fstp DWORD PTR ds:0xc8f7dc
                mem.write(mem.rebase(0xc8f7dc), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d603: mov eax,ds:0xc8f7dc
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7dc));
                // 0xa4d608: fild DWORD PTR [ebp-0xac]
                st.push(f64::from(mem.read::<i32>(r.ebp.wrapping_sub(0xac))));
                // 0xa4d60e: mov DWORD PTR ds:0xc8f598,ecx
                mem.write(mem.rebase(0xc8f598), r.ecx);
                // 0xa4d614: mov DWORD PTR ds:0xc8f59c,edx
                mem.write(mem.rebase(0xc8f59c), r.edx);
                // 0xa4d61a: mov DWORD PTR [ebp-0x2a4],ecx
                mem.write(r.ebp.wrapping_sub(0x2a4), r.ecx);
                // 0xa4d620: mov DWORD PTR [ebp-0x2a0],edx
                mem.write(r.ebp.wrapping_sub(0x2a0), r.edx);
                // 0xa4d626: mov DWORD PTR [ebp-0x334],esi
                mem.write(r.ebp.wrapping_sub(0x334), r.esi);
                // 0xa4d62c: mov DWORD PTR [ebp-0x330],edi
                mem.write(r.ebp.wrapping_sub(0x330), r.edi);
                // 0xa4d632: mov DWORD PTR [ebp-0xa0],eax
                mem.write(r.ebp.wrapping_sub(0xa0), r.eax);
                // 0xa4d638: fstp DWORD PTR [ebp-0x140]
                mem.write(r.ebp.wrapping_sub(0x140), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d63e: fld DWORD PTR [ebp-0x140]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x140)));
                // 0xa4d644: fld QWORD PTR ds:0xb921e0
                st.push(mem.read::<f64>(mem.rebase(0xb921e0)));
                // 0xa4d64a: fmul st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, x * y);
                }
                // 0xa4d64c: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4d64e: fstp DWORD PTR ds:0xc8f7d0
                mem.write(mem.rebase(0xc8f7d0), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d654: fild DWORD PTR [ebp-0xa8]
                st.push(f64::from(mem.read::<i32>(r.ebp.wrapping_sub(0xa8))));
                // 0xa4d65a: fstp DWORD PTR [ebp-0x140]
                mem.write(r.ebp.wrapping_sub(0x140), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d660: fld DWORD PTR [ebp-0x140]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x140)));
                // 0xa4d666: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4d668: fstp DWORD PTR ds:0xc8f7d4
                mem.write(mem.rebase(0xc8f7d4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d66e: fld DWORD PTR ds:0xc0a00c
                st.push(ld32(mem, mem.rebase(0xc0a00c)));
                // 0xa4d674: fst DWORD PTR ds:0xc8f7d8
                mem.write(mem.rebase(0xc8f7d8), crate::x87::store(st0(&st)));
                // 0xa4d67a: fld DWORD PTR ds:0xc0a010
                st.push(ld32(mem, mem.rebase(0xc0a010)));
                // 0xa4d680: fst DWORD PTR ds:0xc8f7dc
                mem.write(mem.rebase(0xc8f7dc), crate::x87::store(st0(&st)));
                // 0xa4d686: fld DWORD PTR ds:0xc8f950
                st.push(ld32(mem, mem.rebase(0xc8f950)));
                // 0xa4d68c: fld DWORD PTR ds:0xc8f7d0
                st.push(ld32(mem, mem.rebase(0xc8f7d0)));
                // 0xa4d692: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4d694: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0xa4d696: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4d698: fstp DWORD PTR ds:0xc8f790
                mem.write(mem.rebase(0xc8f790), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d69e: fld DWORD PTR ds:0xc8f954
                st.push(ld32(mem, mem.rebase(0xc8f954)));
                // 0xa4d6a4: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4d6a6: fstp DWORD PTR ds:0xc8f794
                mem.write(mem.rebase(0xc8f794), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d6ac: fld DWORD PTR ds:0xc8f958
                st.push(ld32(mem, mem.rebase(0xc8f958)));
                // 0xa4d6b2: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4d6b4: fstp DWORD PTR ds:0xc8f798
                mem.write(mem.rebase(0xc8f798), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d6ba: fmul DWORD PTR ds:0xc8f95c
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f95c)));
                }
                // 0xa4d6c0: fstp DWORD PTR ds:0xc8f79c
                mem.write(mem.rebase(0xc8f79c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d6c6: fld DWORD PTR ds:0xc8f960
                st.push(ld32(mem, mem.rebase(0xc8f960)));
                // 0xa4d6cc: fld DWORD PTR ds:0xc8f7d4
                st.push(ld32(mem, mem.rebase(0xc8f7d4)));
                // 0xa4d6d2: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4d6d4: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0xa4d6d6: fld DWORD PTR ds:0xc8f790
                st.push(ld32(mem, mem.rebase(0xc8f790)));
                // 0xa4d6dc: faddp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x + y);
                }
                st.pop();
                // 0xa4d6de: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4d6e0: fstp DWORD PTR ds:0xc8f790
                mem.write(mem.rebase(0xc8f790), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d6e6: fld DWORD PTR ds:0xc8f964
                st.push(ld32(mem, mem.rebase(0xc8f964)));
                // 0xa4d6ec: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4d6ee: fadd DWORD PTR ds:0xc8f794
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f794)));
                }
                // 0xa4d6f4: fstp DWORD PTR ds:0xc8f794
                mem.write(mem.rebase(0xc8f794), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d6fa: fld DWORD PTR ds:0xc8f968
                st.push(ld32(mem, mem.rebase(0xc8f968)));
                // 0xa4d700: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4d702: fadd DWORD PTR ds:0xc8f798
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f798)));
                }
                // 0xa4d708: fstp DWORD PTR ds:0xc8f798
                mem.write(mem.rebase(0xc8f798), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d70e: fmul DWORD PTR ds:0xc8f96c
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f96c)));
                }
                // 0xa4d714: fadd DWORD PTR ds:0xc8f79c
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f79c)));
                }
                // 0xa4d71a: fstp DWORD PTR ds:0xc8f79c
                mem.write(mem.rebase(0xc8f79c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d720: fld st(1)
                {
                    let x = sti(&st, 1);
                    st.push(x);
                }
                // 0xa4d722: fmul DWORD PTR ds:0xc8f970
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f970)));
                }
                // 0xa4d728: fld DWORD PTR ds:0xc8f790
                st.push(ld32(mem, mem.rebase(0xc8f790)));
                // 0xa4d72e: fadd st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x + y);
                }
                // 0xa4d730: fstp DWORD PTR ds:0xc8f790
                mem.write(mem.rebase(0xc8f790), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d736: fld st(2)
                {
                    let x = sti(&st, 2);
                    st.push(x);
                }
                // 0xa4d738: fmul DWORD PTR ds:0xc8f974
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f974)));
                }
                // 0xa4d73e: fld DWORD PTR ds:0xc8f794
                st.push(ld32(mem, mem.rebase(0xc8f794)));
                // 0xa4d744: fadd st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x + y);
                }
                // 0xa4d746: fstp DWORD PTR ds:0xc8f794
                mem.write(mem.rebase(0xc8f794), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d74c: fld st(3)
                {
                    let x = sti(&st, 3);
                    st.push(x);
                }
                // 0xa4d74e: fmul DWORD PTR ds:0xc8f978
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f978)));
                }
                // 0xa4d754: fst QWORD PTR [ebp-0x1d8]
                mem.write(r.ebp.wrapping_sub(0x1d8), st0(&st));
                // 0xa4d75a: fadd DWORD PTR ds:0xc8f798
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f798)));
                }
                // 0xa4d760: fstp DWORD PTR ds:0xc8f798
                mem.write(mem.rebase(0xc8f798), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d766: fld DWORD PTR ds:0xc8f97c
                st.push(ld32(mem, mem.rebase(0xc8f97c)));
                // 0xa4d76c: fmulp st(4),st
                {
                    let (x, y) = (sti(&st, 4), sti(&st, 0));
                    set(&mut st, 4, x * y);
                }
                st.pop();
                // 0xa4d76e: fxch st(3)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 3);
                }
                // 0xa4d770: fst QWORD PTR [ebp-0x148]
                mem.write(r.ebp.wrapping_sub(0x148), st0(&st));
                // 0xa4d776: fadd DWORD PTR ds:0xc8f79c
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f79c)));
                }
                // 0xa4d77c: fstp DWORD PTR ds:0xc8f79c
                mem.write(mem.rebase(0xc8f79c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d782: fld DWORD PTR ds:0xc8f980
                st.push(ld32(mem, mem.rebase(0xc8f980)));
                // 0xa4d788: fmul st,st(2)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 2));
                    set(&mut st, 0, x * y);
                }
                // 0xa4d78a: fst QWORD PTR [ebp-0x1d0]
                mem.write(r.ebp.wrapping_sub(0x1d0), st0(&st));
                // 0xa4d790: fadd DWORD PTR ds:0xc8f790
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f790)));
                }
                // 0xa4d796: fstp DWORD PTR ds:0xc8f940
                mem.write(mem.rebase(0xc8f940), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d79c: fld DWORD PTR ds:0xc8f984
                st.push(ld32(mem, mem.rebase(0xc8f984)));
                // 0xa4d7a2: fmul st,st(2)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 2));
                    set(&mut st, 0, x * y);
                }
                // 0xa4d7a4: fst QWORD PTR [ebp-0x1c8]
                mem.write(r.ebp.wrapping_sub(0x1c8), st0(&st));
                // 0xa4d7aa: fadd DWORD PTR ds:0xc8f794
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f794)));
                }
                // 0xa4d7b0: fstp DWORD PTR ds:0xc8f944
                mem.write(mem.rebase(0xc8f944), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d7b6: fld DWORD PTR ds:0xc8f988
                st.push(ld32(mem, mem.rebase(0xc8f988)));
                // 0xa4d7bc: fmul st,st(2)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 2));
                    set(&mut st, 0, x * y);
                }
                // 0xa4d7be: fst QWORD PTR [ebp-0x1a8]
                mem.write(r.ebp.wrapping_sub(0x1a8), st0(&st));
                // 0xa4d7c4: fadd DWORD PTR ds:0xc8f798
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f798)));
                }
                // 0xa4d7ca: fstp DWORD PTR ds:0xc8f948
                mem.write(mem.rebase(0xc8f948), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d7d0: fld DWORD PTR ds:0xc8f98c
                st.push(ld32(mem, mem.rebase(0xc8f98c)));
                // 0xa4d7d6: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0xa4d7d8: mov eax,ds:0xc8f940
                r.eax = mem.read::<u32>(mem.rebase(0xc8f940));
                // 0xa4d7dd: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4d7df: mov DWORD PTR [ebp-0x17c],eax
                mem.write(r.ebp.wrapping_sub(0x17c), r.eax);
                // 0xa4d7e5: fst QWORD PTR [ebp-0x1b0]
                mem.write(r.ebp.wrapping_sub(0x1b0), st0(&st));
                // 0xa4d7eb: mov eax,ds:0xc8f944
                r.eax = mem.read::<u32>(mem.rebase(0xc8f944));
                // 0xa4d7f0: fadd DWORD PTR ds:0xc8f79c
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f79c)));
                }
                // 0xa4d7f6: mov DWORD PTR [ebp-0x178],eax
                mem.write(r.ebp.wrapping_sub(0x178), r.eax);
                // 0xa4d7fc: mov eax,ds:0xc8f948
                r.eax = mem.read::<u32>(mem.rebase(0xc8f948));
                // 0xa4d801: fstp DWORD PTR ds:0xc8f94c
                mem.write(mem.rebase(0xc8f94c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d807: mov DWORD PTR [ebp-0x174],eax
                mem.write(r.ebp.wrapping_sub(0x174), r.eax);
                // 0xa4d80d: mov eax,ds:0xc8f94c
                r.eax = mem.read::<u32>(mem.rebase(0xc8f94c));
                // 0xa4d812: fldz
                st.push(0.0);
                // 0xa4d814: mov DWORD PTR [ebp-0x170],eax
                mem.write(r.ebp.wrapping_sub(0x170), r.eax);
                // 0xa4d81a: fcom DWORD PTR [ebp-0x170]
                r.sw = fcom(st0(&st), ld32(mem, r.ebp.wrapping_sub(0x170)));
                // 0xa4d820: fnstsw ax
                r.eax = (r.eax & 0xffff_00ff) | (u32::from(r.sw) << 8);
                // 0xa4d822: test ah,0x1
                r.flag = ((r.eax >> 8) & 0x1, 0, true);
                r.cmp = false;
                // 0xa4d825: je 0xa4f209
                pc = if r.zf() { 0xa4f209 } else { 0xa4d82b };
            }
            0xa4d82b => {
                // 0xa4d82b: movsx eax,WORD PTR [ebx+0x24]
                r.eax = i32::from(mem.read::<i16>(r.ebx.wrapping_add(0x24))).cast_unsigned();
                // 0xa4d82f: cdq
                r.edx = if r.eax.cast_signed() < 0 { u32::MAX } else { 0 };
                // 0xa4d830: mov esi,eax
                r.esi = r.eax;
                // 0xa4d832: mov edi,edx
                r.edi = r.edx;
                // 0xa4d834: mov DWORD PTR ds:0xc8f588,esi
                mem.write(mem.rebase(0xc8f588), r.esi);
                // 0xa4d83a: mov DWORD PTR ds:0xc8f58c,edi
                mem.write(mem.rebase(0xc8f58c), r.edi);
                // 0xa4d840: movsx eax,WORD PTR [ebx+0x26]
                r.eax = i32::from(mem.read::<i16>(r.ebx.wrapping_add(0x26))).cast_unsigned();
                // 0xa4d844: cdq
                r.edx = if r.eax.cast_signed() < 0 { u32::MAX } else { 0 };
                // 0xa4d845: mov ecx,eax
                r.ecx = r.eax;
                // 0xa4d847: mov eax,ds:0xc8f5a0
                r.eax = mem.read::<u32>(mem.rebase(0xc8f5a0));
                // 0xa4d84c: mov DWORD PTR [ebp-0x34c],eax
                mem.write(r.ebp.wrapping_sub(0x34c), r.eax);
                // 0xa4d852: mov eax,ds:0xc8f5a4
                r.eax = mem.read::<u32>(mem.rebase(0xc8f5a4));
                // 0xa4d857: mov DWORD PTR [ebp-0x348],eax
                mem.write(r.ebp.wrapping_sub(0x348), r.eax);
                // 0xa4d85d: mov eax,ds:0xc8f590
                r.eax = mem.read::<u32>(mem.rebase(0xc8f590));
                // 0xa4d862: mov DWORD PTR [ebp-0x2bc],eax
                mem.write(r.ebp.wrapping_sub(0x2bc), r.eax);
                // 0xa4d868: mov eax,ds:0xc8f594
                r.eax = mem.read::<u32>(mem.rebase(0xc8f594));
                // 0xa4d86d: mov DWORD PTR ds:0xc8f588,esi
                mem.write(mem.rebase(0xc8f588), r.esi);
                // 0xa4d873: mov DWORD PTR [ebp-0x2b8],eax
                mem.write(r.ebp.wrapping_sub(0x2b8), r.eax);
                // 0xa4d879: fld DWORD PTR ds:0xc8f588
                st.push(ld32(mem, mem.rebase(0xc8f588)));
                // 0xa4d87f: mov DWORD PTR ds:0xc8f58c,ecx
                mem.write(mem.rebase(0xc8f58c), r.ecx);
                // 0xa4d885: fstp DWORD PTR ds:0xc8f7d0
                mem.write(mem.rebase(0xc8f7d0), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d88b: mov eax,ds:0xc8f7d0
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d0));
                // 0xa4d890: fld DWORD PTR ds:0xc8f58c
                st.push(ld32(mem, mem.rebase(0xc8f58c)));
                // 0xa4d896: mov DWORD PTR [ebp-0xcc],eax
                mem.write(r.ebp.wrapping_sub(0xcc), r.eax);
                // 0xa4d89c: fstp DWORD PTR ds:0xc8f7d4
                mem.write(mem.rebase(0xc8f7d4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d8a2: mov eax,ds:0xc8f7d4
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d4));
                // 0xa4d8a7: mov DWORD PTR ds:0xc8f590,edi
                mem.write(mem.rebase(0xc8f590), r.edi);
                // 0xa4d8ad: mov DWORD PTR [ebp-0xc8],eax
                mem.write(r.ebp.wrapping_sub(0xc8), r.eax);
                // 0xa4d8b3: fld DWORD PTR ds:0xc8f590
                st.push(ld32(mem, mem.rebase(0xc8f590)));
                // 0xa4d8b9: mov DWORD PTR ds:0xc8f594,edx
                mem.write(mem.rebase(0xc8f594), r.edx);
                // 0xa4d8bf: fstp DWORD PTR ds:0xc8f7d8
                mem.write(mem.rebase(0xc8f7d8), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d8c5: mov eax,ds:0xc8f7d8
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d8));
                // 0xa4d8ca: fld DWORD PTR ds:0xc8f594
                st.push(ld32(mem, mem.rebase(0xc8f594)));
                // 0xa4d8d0: mov DWORD PTR [ebp-0xc4],eax
                mem.write(r.ebp.wrapping_sub(0xc4), r.eax);
                // 0xa4d8d6: fstp DWORD PTR ds:0xc8f7dc
                mem.write(mem.rebase(0xc8f7dc), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d8dc: mov eax,ds:0xc8f7dc
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7dc));
                // 0xa4d8e1: fild DWORD PTR [ebp-0xcc]
                st.push(f64::from(mem.read::<i32>(r.ebp.wrapping_sub(0xcc))));
                // 0xa4d8e7: mov DWORD PTR ds:0xc8f598,ecx
                mem.write(mem.rebase(0xc8f598), r.ecx);
                // 0xa4d8ed: mov DWORD PTR ds:0xc8f59c,edx
                mem.write(mem.rebase(0xc8f59c), r.edx);
                // 0xa4d8f3: mov DWORD PTR [ebp-0x354],ecx
                mem.write(r.ebp.wrapping_sub(0x354), r.ecx);
                // 0xa4d8f9: mov DWORD PTR [ebp-0x350],edx
                mem.write(r.ebp.wrapping_sub(0x350), r.edx);
                // 0xa4d8ff: mov DWORD PTR [ebp-0x2c4],esi
                mem.write(r.ebp.wrapping_sub(0x2c4), r.esi);
                // 0xa4d905: mov DWORD PTR [ebp-0x2c0],edi
                mem.write(r.ebp.wrapping_sub(0x2c0), r.edi);
                // 0xa4d90b: mov DWORD PTR [ebp-0xc0],eax
                mem.write(r.ebp.wrapping_sub(0xc0), r.eax);
                // 0xa4d911: fstp DWORD PTR [ebp-0x140]
                mem.write(r.ebp.wrapping_sub(0x140), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d917: fld DWORD PTR [ebp-0x140]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x140)));
                // 0xa4d91d: fmul st,st(4)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 4));
                    set(&mut st, 0, x * y);
                }
                // 0xa4d91f: fstp DWORD PTR ds:0xc8f7d0
                mem.write(mem.rebase(0xc8f7d0), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d925: fild DWORD PTR [ebp-0xc8]
                st.push(f64::from(mem.read::<i32>(r.ebp.wrapping_sub(0xc8))));
                // 0xa4d92b: fstp DWORD PTR [ebp-0x140]
                mem.write(r.ebp.wrapping_sub(0x140), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d931: fld DWORD PTR [ebp-0x140]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x140)));
                // 0xa4d937: fmul st,st(4)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 4));
                    set(&mut st, 0, x * y);
                }
                // 0xa4d939: fstp DWORD PTR ds:0xc8f7d4
                mem.write(mem.rebase(0xc8f7d4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d93f: fld DWORD PTR ds:0xc0a00c
                st.push(ld32(mem, mem.rebase(0xc0a00c)));
                // 0xa4d945: fstp DWORD PTR ds:0xc8f7d8
                mem.write(mem.rebase(0xc8f7d8), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d94b: fld DWORD PTR ds:0xc0a010
                st.push(ld32(mem, mem.rebase(0xc0a010)));
                // 0xa4d951: fstp DWORD PTR ds:0xc8f7dc
                mem.write(mem.rebase(0xc8f7dc), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d957: fld DWORD PTR ds:0xc8f950
                st.push(ld32(mem, mem.rebase(0xc8f950)));
                // 0xa4d95d: fld DWORD PTR ds:0xc8f7d0
                st.push(ld32(mem, mem.rebase(0xc8f7d0)));
                // 0xa4d963: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4d965: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0xa4d967: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4d969: fstp DWORD PTR ds:0xc8f790
                mem.write(mem.rebase(0xc8f790), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d96f: fld DWORD PTR ds:0xc8f954
                st.push(ld32(mem, mem.rebase(0xc8f954)));
                // 0xa4d975: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4d977: fstp DWORD PTR ds:0xc8f794
                mem.write(mem.rebase(0xc8f794), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d97d: fld DWORD PTR ds:0xc8f958
                st.push(ld32(mem, mem.rebase(0xc8f958)));
                // 0xa4d983: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4d985: fstp DWORD PTR ds:0xc8f798
                mem.write(mem.rebase(0xc8f798), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d98b: fmul DWORD PTR ds:0xc8f95c
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f95c)));
                }
                // 0xa4d991: fstp DWORD PTR ds:0xc8f79c
                mem.write(mem.rebase(0xc8f79c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d997: fld DWORD PTR ds:0xc8f960
                st.push(ld32(mem, mem.rebase(0xc8f960)));
                // 0xa4d99d: fld DWORD PTR ds:0xc8f7d4
                st.push(ld32(mem, mem.rebase(0xc8f7d4)));
                // 0xa4d9a3: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4d9a5: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0xa4d9a7: fld DWORD PTR ds:0xc8f790
                st.push(ld32(mem, mem.rebase(0xc8f790)));
                // 0xa4d9ad: faddp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x + y);
                }
                st.pop();
                // 0xa4d9af: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4d9b1: fstp DWORD PTR ds:0xc8f790
                mem.write(mem.rebase(0xc8f790), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d9b7: fld DWORD PTR ds:0xc8f964
                st.push(ld32(mem, mem.rebase(0xc8f964)));
                // 0xa4d9bd: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4d9bf: fadd DWORD PTR ds:0xc8f794
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f794)));
                }
                // 0xa4d9c5: fstp DWORD PTR ds:0xc8f794
                mem.write(mem.rebase(0xc8f794), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d9cb: fld DWORD PTR ds:0xc8f968
                st.push(ld32(mem, mem.rebase(0xc8f968)));
                // 0xa4d9d1: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4d9d3: fadd DWORD PTR ds:0xc8f798
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f798)));
                }
                // 0xa4d9d9: fstp DWORD PTR ds:0xc8f798
                mem.write(mem.rebase(0xc8f798), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d9df: fmul DWORD PTR ds:0xc8f96c
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f96c)));
                }
                // 0xa4d9e5: fadd DWORD PTR ds:0xc8f79c
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f79c)));
                }
                // 0xa4d9eb: fstp DWORD PTR ds:0xc8f79c
                mem.write(mem.rebase(0xc8f79c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d9f1: fld DWORD PTR ds:0xc8f790
                st.push(ld32(mem, mem.rebase(0xc8f790)));
                // 0xa4d9f7: fadd st,st(2)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 2));
                    set(&mut st, 0, x + y);
                }
                // 0xa4d9f9: fstp DWORD PTR ds:0xc8f790
                mem.write(mem.rebase(0xc8f790), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4d9ff: fld DWORD PTR ds:0xc8f794
                st.push(ld32(mem, mem.rebase(0xc8f794)));
                // 0xa4da05: fadd st,st(3)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 3));
                    set(&mut st, 0, x + y);
                }
                // 0xa4da07: fstp DWORD PTR ds:0xc8f794
                mem.write(mem.rebase(0xc8f794), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4da0d: fld DWORD PTR ds:0xc8f798
                st.push(ld32(mem, mem.rebase(0xc8f798)));
                // 0xa4da13: fld QWORD PTR [ebp-0x1d8]
                st.push(mem.read::<f64>(r.ebp.wrapping_sub(0x1d8)));
                // 0xa4da19: fadd st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, x + y);
                }
                // 0xa4da1b: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4da1d: fstp DWORD PTR ds:0xc8f798
                mem.write(mem.rebase(0xc8f798), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4da23: fld DWORD PTR ds:0xc8f79c
                st.push(ld32(mem, mem.rebase(0xc8f79c)));
                // 0xa4da29: fadd QWORD PTR [ebp-0x148]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + mem.read::<f64>(r.ebp.wrapping_sub(0x148)));
                }
                // 0xa4da2f: fstp DWORD PTR ds:0xc8f79c
                mem.write(mem.rebase(0xc8f79c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4da35: fld DWORD PTR ds:0xc8f790
                st.push(ld32(mem, mem.rebase(0xc8f790)));
                // 0xa4da3b: fadd QWORD PTR [ebp-0x1d0]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + mem.read::<f64>(r.ebp.wrapping_sub(0x1d0)));
                }
                // 0xa4da41: fstp DWORD PTR ds:0xc8f940
                mem.write(mem.rebase(0xc8f940), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4da47: mov eax,ds:0xc8f940
                r.eax = mem.read::<u32>(mem.rebase(0xc8f940));
                // 0xa4da4c: fld DWORD PTR ds:0xc8f794
                st.push(ld32(mem, mem.rebase(0xc8f794)));
                // 0xa4da52: mov DWORD PTR [ebp-0x16c],eax
                mem.write(r.ebp.wrapping_sub(0x16c), r.eax);
                // 0xa4da58: fadd QWORD PTR [ebp-0x1c8]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + mem.read::<f64>(r.ebp.wrapping_sub(0x1c8)));
                }
                // 0xa4da5e: fstp DWORD PTR ds:0xc8f944
                mem.write(mem.rebase(0xc8f944), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4da64: mov eax,ds:0xc8f944
                r.eax = mem.read::<u32>(mem.rebase(0xc8f944));
                // 0xa4da69: fld DWORD PTR ds:0xc8f798
                st.push(ld32(mem, mem.rebase(0xc8f798)));
                // 0xa4da6f: mov DWORD PTR [ebp-0x168],eax
                mem.write(r.ebp.wrapping_sub(0x168), r.eax);
                // 0xa4da75: fadd QWORD PTR [ebp-0x1a8]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + mem.read::<f64>(r.ebp.wrapping_sub(0x1a8)));
                }
                // 0xa4da7b: fstp DWORD PTR ds:0xc8f948
                mem.write(mem.rebase(0xc8f948), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4da81: mov eax,ds:0xc8f948
                r.eax = mem.read::<u32>(mem.rebase(0xc8f948));
                // 0xa4da86: fld DWORD PTR ds:0xc8f79c
                st.push(ld32(mem, mem.rebase(0xc8f79c)));
                // 0xa4da8c: mov DWORD PTR [ebp-0x164],eax
                mem.write(r.ebp.wrapping_sub(0x164), r.eax);
                // 0xa4da92: fadd QWORD PTR [ebp-0x1b0]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + mem.read::<f64>(r.ebp.wrapping_sub(0x1b0)));
                }
                // 0xa4da98: fstp DWORD PTR ds:0xc8f94c
                mem.write(mem.rebase(0xc8f94c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4da9e: mov eax,ds:0xc8f94c
                r.eax = mem.read::<u32>(mem.rebase(0xc8f94c));
                // 0xa4daa3: mov DWORD PTR [ebp-0x160],eax
                mem.write(r.ebp.wrapping_sub(0x160), r.eax);
                // 0xa4daa9: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4daab: fcom DWORD PTR [ebp-0x160]
                r.sw = fcom(st0(&st), ld32(mem, r.ebp.wrapping_sub(0x160)));
                // 0xa4dab1: fnstsw ax
                r.eax = (r.eax & 0xffff_00ff) | (u32::from(r.sw) << 8);
                // 0xa4dab3: test ah,0x1
                r.flag = ((r.eax >> 8) & 0x1, 0, true);
                r.cmp = false;
                // 0xa4dab6: je 0xa4f207
                pc = if r.zf() { 0xa4f207 } else { 0xa4dabc };
            }
            0xa4dabc => {
                // 0xa4dabc: movsx eax,WORD PTR [ebx+0x28]
                r.eax = i32::from(mem.read::<i16>(r.ebx.wrapping_add(0x28))).cast_unsigned();
                // 0xa4dac0: cdq
                r.edx = if r.eax.cast_signed() < 0 { u32::MAX } else { 0 };
                // 0xa4dac1: mov esi,eax
                r.esi = r.eax;
                // 0xa4dac3: mov edi,edx
                r.edi = r.edx;
                // 0xa4dac5: mov DWORD PTR ds:0xc8f588,esi
                mem.write(mem.rebase(0xc8f588), r.esi);
                // 0xa4dacb: mov DWORD PTR ds:0xc8f58c,edi
                mem.write(mem.rebase(0xc8f58c), r.edi);
                // 0xa4dad1: movsx eax,WORD PTR [ebx+0x2a]
                r.eax = i32::from(mem.read::<i16>(r.ebx.wrapping_add(0x2a))).cast_unsigned();
                // 0xa4dad5: cdq
                r.edx = if r.eax.cast_signed() < 0 { u32::MAX } else { 0 };
                // 0xa4dad6: mov ecx,eax
                r.ecx = r.eax;
                // 0xa4dad8: mov eax,ds:0xc8f5a0
                r.eax = mem.read::<u32>(mem.rebase(0xc8f5a0));
                // 0xa4dadd: mov DWORD PTR [ebp-0x33c],eax
                mem.write(r.ebp.wrapping_sub(0x33c), r.eax);
                // 0xa4dae3: mov eax,ds:0xc8f5a4
                r.eax = mem.read::<u32>(mem.rebase(0xc8f5a4));
                // 0xa4dae8: mov DWORD PTR [ebp-0x338],eax
                mem.write(r.ebp.wrapping_sub(0x338), r.eax);
                // 0xa4daee: mov eax,ds:0xc8f590
                r.eax = mem.read::<u32>(mem.rebase(0xc8f590));
                // 0xa4daf3: mov DWORD PTR [ebp-0x2dc],eax
                mem.write(r.ebp.wrapping_sub(0x2dc), r.eax);
                // 0xa4daf9: mov eax,ds:0xc8f594
                r.eax = mem.read::<u32>(mem.rebase(0xc8f594));
                // 0xa4dafe: mov DWORD PTR ds:0xc8f588,esi
                mem.write(mem.rebase(0xc8f588), r.esi);
                // 0xa4db04: mov DWORD PTR [ebp-0x2d8],eax
                mem.write(r.ebp.wrapping_sub(0x2d8), r.eax);
                // 0xa4db0a: fld DWORD PTR ds:0xc8f588
                st.push(ld32(mem, mem.rebase(0xc8f588)));
                // 0xa4db10: mov DWORD PTR ds:0xc8f58c,ecx
                mem.write(mem.rebase(0xc8f58c), r.ecx);
                // 0xa4db16: fstp DWORD PTR ds:0xc8f7d0
                mem.write(mem.rebase(0xc8f7d0), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4db1c: mov eax,ds:0xc8f7d0
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d0));
                // 0xa4db21: fld DWORD PTR ds:0xc8f58c
                st.push(ld32(mem, mem.rebase(0xc8f58c)));
                // 0xa4db27: mov DWORD PTR [ebp-0xdc],eax
                mem.write(r.ebp.wrapping_sub(0xdc), r.eax);
                // 0xa4db2d: fstp DWORD PTR ds:0xc8f7d4
                mem.write(mem.rebase(0xc8f7d4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4db33: mov eax,ds:0xc8f7d4
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d4));
                // 0xa4db38: mov DWORD PTR ds:0xc8f590,edi
                mem.write(mem.rebase(0xc8f590), r.edi);
                // 0xa4db3e: mov DWORD PTR [ebp-0xd8],eax
                mem.write(r.ebp.wrapping_sub(0xd8), r.eax);
                // 0xa4db44: fld DWORD PTR ds:0xc8f590
                st.push(ld32(mem, mem.rebase(0xc8f590)));
                // 0xa4db4a: mov DWORD PTR ds:0xc8f594,edx
                mem.write(mem.rebase(0xc8f594), r.edx);
                // 0xa4db50: fstp DWORD PTR ds:0xc8f7d8
                mem.write(mem.rebase(0xc8f7d8), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4db56: mov eax,ds:0xc8f7d8
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d8));
                // 0xa4db5b: fld DWORD PTR ds:0xc8f594
                st.push(ld32(mem, mem.rebase(0xc8f594)));
                // 0xa4db61: mov DWORD PTR [ebp-0xd4],eax
                mem.write(r.ebp.wrapping_sub(0xd4), r.eax);
                // 0xa4db67: fstp DWORD PTR ds:0xc8f7dc
                mem.write(mem.rebase(0xc8f7dc), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4db6d: mov eax,ds:0xc8f7dc
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7dc));
                // 0xa4db72: fild DWORD PTR [ebp-0xdc]
                st.push(f64::from(mem.read::<i32>(r.ebp.wrapping_sub(0xdc))));
                // 0xa4db78: mov DWORD PTR ds:0xc8f598,ecx
                mem.write(mem.rebase(0xc8f598), r.ecx);
                // 0xa4db7e: mov DWORD PTR ds:0xc8f59c,edx
                mem.write(mem.rebase(0xc8f59c), r.edx);
                // 0xa4db84: mov DWORD PTR [ebp-0x344],ecx
                mem.write(r.ebp.wrapping_sub(0x344), r.ecx);
                // 0xa4db8a: mov DWORD PTR [ebp-0x340],edx
                mem.write(r.ebp.wrapping_sub(0x340), r.edx);
                // 0xa4db90: mov DWORD PTR [ebp-0x2e4],esi
                mem.write(r.ebp.wrapping_sub(0x2e4), r.esi);
                // 0xa4db96: mov DWORD PTR [ebp-0x2e0],edi
                mem.write(r.ebp.wrapping_sub(0x2e0), r.edi);
                // 0xa4db9c: mov DWORD PTR [ebp-0xd0],eax
                mem.write(r.ebp.wrapping_sub(0xd0), r.eax);
                // 0xa4dba2: fstp DWORD PTR [ebp-0x140]
                mem.write(r.ebp.wrapping_sub(0x140), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4dba8: fld DWORD PTR [ebp-0x140]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x140)));
                // 0xa4dbae: fmul st,st(5)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 5));
                    set(&mut st, 0, x * y);
                }
                // 0xa4dbb0: fstp DWORD PTR ds:0xc8f7d0
                mem.write(mem.rebase(0xc8f7d0), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4dbb6: fild DWORD PTR [ebp-0xd8]
                st.push(f64::from(mem.read::<i32>(r.ebp.wrapping_sub(0xd8))));
                // 0xa4dbbc: fstp DWORD PTR [ebp-0x140]
                mem.write(r.ebp.wrapping_sub(0x140), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4dbc2: fld DWORD PTR [ebp-0x140]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x140)));
                // 0xa4dbc8: fmul st,st(5)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 5));
                    set(&mut st, 0, x * y);
                }
                // 0xa4dbca: fstp DWORD PTR ds:0xc8f7d4
                mem.write(mem.rebase(0xc8f7d4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4dbd0: fld DWORD PTR ds:0xc0a00c
                st.push(ld32(mem, mem.rebase(0xc0a00c)));
                // 0xa4dbd6: fstp DWORD PTR ds:0xc8f7d8
                mem.write(mem.rebase(0xc8f7d8), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4dbdc: fld DWORD PTR ds:0xc0a010
                st.push(ld32(mem, mem.rebase(0xc0a010)));
                // 0xa4dbe2: fstp DWORD PTR ds:0xc8f7dc
                mem.write(mem.rebase(0xc8f7dc), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4dbe8: fld DWORD PTR ds:0xc8f950
                st.push(ld32(mem, mem.rebase(0xc8f950)));
                // 0xa4dbee: fld DWORD PTR ds:0xc8f7d0
                st.push(ld32(mem, mem.rebase(0xc8f7d0)));
                // 0xa4dbf4: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4dbf6: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0xa4dbf8: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4dbfa: fstp DWORD PTR ds:0xc8f790
                mem.write(mem.rebase(0xc8f790), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4dc00: fld DWORD PTR ds:0xc8f954
                st.push(ld32(mem, mem.rebase(0xc8f954)));
                // 0xa4dc06: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4dc08: fstp DWORD PTR ds:0xc8f794
                mem.write(mem.rebase(0xc8f794), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4dc0e: fld DWORD PTR ds:0xc8f958
                st.push(ld32(mem, mem.rebase(0xc8f958)));
                // 0xa4dc14: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4dc16: fstp DWORD PTR ds:0xc8f798
                mem.write(mem.rebase(0xc8f798), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4dc1c: fmul DWORD PTR ds:0xc8f95c
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f95c)));
                }
                // 0xa4dc22: fstp DWORD PTR ds:0xc8f79c
                mem.write(mem.rebase(0xc8f79c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4dc28: fld DWORD PTR ds:0xc8f960
                st.push(ld32(mem, mem.rebase(0xc8f960)));
                // 0xa4dc2e: fld DWORD PTR ds:0xc8f7d4
                st.push(ld32(mem, mem.rebase(0xc8f7d4)));
                // 0xa4dc34: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4dc36: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0xa4dc38: fld DWORD PTR ds:0xc8f790
                st.push(ld32(mem, mem.rebase(0xc8f790)));
                // 0xa4dc3e: faddp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x + y);
                }
                st.pop();
                // 0xa4dc40: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4dc42: fstp DWORD PTR ds:0xc8f790
                mem.write(mem.rebase(0xc8f790), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4dc48: fld DWORD PTR ds:0xc8f964
                st.push(ld32(mem, mem.rebase(0xc8f964)));
                // 0xa4dc4e: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4dc50: fadd DWORD PTR ds:0xc8f794
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f794)));
                }
                // 0xa4dc56: fstp DWORD PTR ds:0xc8f794
                mem.write(mem.rebase(0xc8f794), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4dc5c: fld DWORD PTR ds:0xc8f968
                st.push(ld32(mem, mem.rebase(0xc8f968)));
                // 0xa4dc62: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4dc64: fadd DWORD PTR ds:0xc8f798
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f798)));
                }
                // 0xa4dc6a: fstp DWORD PTR ds:0xc8f798
                mem.write(mem.rebase(0xc8f798), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4dc70: fmul DWORD PTR ds:0xc8f96c
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f96c)));
                }
                // 0xa4dc76: fadd DWORD PTR ds:0xc8f79c
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f79c)));
                }
                // 0xa4dc7c: fstp DWORD PTR ds:0xc8f79c
                mem.write(mem.rebase(0xc8f79c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4dc82: fld DWORD PTR ds:0xc8f790
                st.push(ld32(mem, mem.rebase(0xc8f790)));
                // 0xa4dc88: fadd st,st(3)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 3));
                    set(&mut st, 0, x + y);
                }
                // 0xa4dc8a: fstp DWORD PTR ds:0xc8f790
                mem.write(mem.rebase(0xc8f790), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4dc90: fld DWORD PTR ds:0xc8f794
                st.push(ld32(mem, mem.rebase(0xc8f794)));
                // 0xa4dc96: fadd st,st(4)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 4));
                    set(&mut st, 0, x + y);
                }
                // 0xa4dc98: fstp DWORD PTR ds:0xc8f794
                mem.write(mem.rebase(0xc8f794), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4dc9e: fld DWORD PTR ds:0xc8f798
                st.push(ld32(mem, mem.rebase(0xc8f798)));
                // 0xa4dca4: faddp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x + y);
                }
                st.pop();
                // 0xa4dca6: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4dca8: fstp DWORD PTR ds:0xc8f798
                mem.write(mem.rebase(0xc8f798), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4dcae: fld DWORD PTR ds:0xc8f79c
                st.push(ld32(mem, mem.rebase(0xc8f79c)));
                // 0xa4dcb4: fadd QWORD PTR [ebp-0x148]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + mem.read::<f64>(r.ebp.wrapping_sub(0x148)));
                }
                // 0xa4dcba: fstp DWORD PTR ds:0xc8f79c
                mem.write(mem.rebase(0xc8f79c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4dcc0: fld DWORD PTR ds:0xc8f790
                st.push(ld32(mem, mem.rebase(0xc8f790)));
                // 0xa4dcc6: fadd QWORD PTR [ebp-0x1d0]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + mem.read::<f64>(r.ebp.wrapping_sub(0x1d0)));
                }
                // 0xa4dccc: fstp DWORD PTR ds:0xc8f940
                mem.write(mem.rebase(0xc8f940), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4dcd2: mov eax,ds:0xc8f940
                r.eax = mem.read::<u32>(mem.rebase(0xc8f940));
                // 0xa4dcd7: fld DWORD PTR ds:0xc8f794
                st.push(ld32(mem, mem.rebase(0xc8f794)));
                // 0xa4dcdd: mov DWORD PTR [ebp-0x15c],eax
                mem.write(r.ebp.wrapping_sub(0x15c), r.eax);
                // 0xa4dce3: fadd QWORD PTR [ebp-0x1c8]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + mem.read::<f64>(r.ebp.wrapping_sub(0x1c8)));
                }
                // 0xa4dce9: fstp DWORD PTR ds:0xc8f944
                mem.write(mem.rebase(0xc8f944), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4dcef: mov eax,ds:0xc8f944
                r.eax = mem.read::<u32>(mem.rebase(0xc8f944));
                // 0xa4dcf4: fld DWORD PTR ds:0xc8f798
                st.push(ld32(mem, mem.rebase(0xc8f798)));
                // 0xa4dcfa: mov DWORD PTR [ebp-0x158],eax
                mem.write(r.ebp.wrapping_sub(0x158), r.eax);
                // 0xa4dd00: fadd QWORD PTR [ebp-0x1a8]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + mem.read::<f64>(r.ebp.wrapping_sub(0x1a8)));
                }
                // 0xa4dd06: fstp DWORD PTR ds:0xc8f948
                mem.write(mem.rebase(0xc8f948), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4dd0c: mov eax,ds:0xc8f948
                r.eax = mem.read::<u32>(mem.rebase(0xc8f948));
                // 0xa4dd11: fld DWORD PTR ds:0xc8f79c
                st.push(ld32(mem, mem.rebase(0xc8f79c)));
                // 0xa4dd17: mov DWORD PTR [ebp-0x154],eax
                mem.write(r.ebp.wrapping_sub(0x154), r.eax);
                // 0xa4dd1d: fadd QWORD PTR [ebp-0x1b0]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + mem.read::<f64>(r.ebp.wrapping_sub(0x1b0)));
                }
                // 0xa4dd23: fstp DWORD PTR ds:0xc8f94c
                mem.write(mem.rebase(0xc8f94c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4dd29: mov eax,ds:0xc8f94c
                r.eax = mem.read::<u32>(mem.rebase(0xc8f94c));
                // 0xa4dd2e: mov DWORD PTR [ebp-0x150],eax
                mem.write(r.ebp.wrapping_sub(0x150), r.eax);
                // 0xa4dd34: fcom DWORD PTR [ebp-0x150]
                r.sw = fcom(st0(&st), ld32(mem, r.ebp.wrapping_sub(0x150)));
                // 0xa4dd3a: fnstsw ax
                r.eax = (r.eax & 0xffff_00ff) | (u32::from(r.sw) << 8);
                // 0xa4dd3c: test ah,0x1
                r.flag = ((r.eax >> 8) & 0x1, 0, true);
                r.cmp = false;
                // 0xa4dd3f: je 0xa4f209
                pc = if r.zf() { 0xa4f209 } else { 0xa4dd45 };
            }
            0xa4dd45 => {
                // 0xa4dd45: movsx eax,WORD PTR [ebx+0x2c]
                r.eax = i32::from(mem.read::<i16>(r.ebx.wrapping_add(0x2c))).cast_unsigned();
                // 0xa4dd49: cdq
                r.edx = if r.eax.cast_signed() < 0 { u32::MAX } else { 0 };
                // 0xa4dd4a: mov esi,eax
                r.esi = r.eax;
                // 0xa4dd4c: mov edi,edx
                r.edi = r.edx;
                // 0xa4dd4e: mov DWORD PTR ds:0xc8f588,esi
                mem.write(mem.rebase(0xc8f588), r.esi);
                // 0xa4dd54: mov DWORD PTR ds:0xc8f58c,edi
                mem.write(mem.rebase(0xc8f58c), r.edi);
                // 0xa4dd5a: movsx eax,WORD PTR [ebx+0x2e]
                r.eax = i32::from(mem.read::<i16>(r.ebx.wrapping_add(0x2e))).cast_unsigned();
                // 0xa4dd5e: cdq
                r.edx = if r.eax.cast_signed() < 0 { u32::MAX } else { 0 };
                // 0xa4dd5f: mov ecx,eax
                r.ecx = r.eax;
                // 0xa4dd61: mov eax,ds:0xc8f5a0
                r.eax = mem.read::<u32>(mem.rebase(0xc8f5a0));
                // 0xa4dd66: mov DWORD PTR [ebp-0x31c],eax
                mem.write(r.ebp.wrapping_sub(0x31c), r.eax);
                // 0xa4dd6c: mov eax,ds:0xc8f5a4
                r.eax = mem.read::<u32>(mem.rebase(0xc8f5a4));
                // 0xa4dd71: mov DWORD PTR [ebp-0x318],eax
                mem.write(r.ebp.wrapping_sub(0x318), r.eax);
                // 0xa4dd77: mov eax,ds:0xc8f590
                r.eax = mem.read::<u32>(mem.rebase(0xc8f590));
                // 0xa4dd7c: mov DWORD PTR [ebp-0x2fc],eax
                mem.write(r.ebp.wrapping_sub(0x2fc), r.eax);
                // 0xa4dd82: mov eax,ds:0xc8f594
                r.eax = mem.read::<u32>(mem.rebase(0xc8f594));
                // 0xa4dd87: mov DWORD PTR ds:0xc8f588,esi
                mem.write(mem.rebase(0xc8f588), r.esi);
                // 0xa4dd8d: mov DWORD PTR [ebp-0x2f8],eax
                mem.write(r.ebp.wrapping_sub(0x2f8), r.eax);
                // 0xa4dd93: fld DWORD PTR ds:0xc8f588
                st.push(ld32(mem, mem.rebase(0xc8f588)));
                // 0xa4dd99: mov DWORD PTR ds:0xc8f58c,ecx
                mem.write(mem.rebase(0xc8f58c), r.ecx);
                // 0xa4dd9f: fstp DWORD PTR ds:0xc8f7d0
                mem.write(mem.rebase(0xc8f7d0), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4dda5: mov eax,ds:0xc8f7d0
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d0));
                // 0xa4ddaa: fld DWORD PTR ds:0xc8f58c
                st.push(ld32(mem, mem.rebase(0xc8f58c)));
                // 0xa4ddb0: mov DWORD PTR [ebp-0xbc],eax
                mem.write(r.ebp.wrapping_sub(0xbc), r.eax);
                // 0xa4ddb6: fstp DWORD PTR ds:0xc8f7d4
                mem.write(mem.rebase(0xc8f7d4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4ddbc: mov eax,ds:0xc8f7d4
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d4));
                // 0xa4ddc1: mov DWORD PTR ds:0xc8f590,edi
                mem.write(mem.rebase(0xc8f590), r.edi);
                // 0xa4ddc7: mov DWORD PTR [ebp-0xb8],eax
                mem.write(r.ebp.wrapping_sub(0xb8), r.eax);
                // 0xa4ddcd: fld DWORD PTR ds:0xc8f590
                st.push(ld32(mem, mem.rebase(0xc8f590)));
                // 0xa4ddd3: mov DWORD PTR ds:0xc8f594,edx
                mem.write(mem.rebase(0xc8f594), r.edx);
                // 0xa4ddd9: fstp DWORD PTR ds:0xc8f7d8
                mem.write(mem.rebase(0xc8f7d8), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4dddf: mov eax,ds:0xc8f7d8
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d8));
                // 0xa4dde4: fld DWORD PTR ds:0xc8f594
                st.push(ld32(mem, mem.rebase(0xc8f594)));
                // 0xa4ddea: mov DWORD PTR [ebp-0xb4],eax
                mem.write(r.ebp.wrapping_sub(0xb4), r.eax);
                // 0xa4ddf0: fstp DWORD PTR ds:0xc8f7dc
                mem.write(mem.rebase(0xc8f7dc), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4ddf6: mov eax,ds:0xc8f7dc
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7dc));
                // 0xa4ddfb: fild DWORD PTR [ebp-0xbc]
                st.push(f64::from(mem.read::<i32>(r.ebp.wrapping_sub(0xbc))));
                // 0xa4de01: mov DWORD PTR ds:0xc8f598,ecx
                mem.write(mem.rebase(0xc8f598), r.ecx);
                // 0xa4de07: mov DWORD PTR ds:0xc8f59c,edx
                mem.write(mem.rebase(0xc8f59c), r.edx);
                // 0xa4de0d: mov DWORD PTR [ebp-0x324],ecx
                mem.write(r.ebp.wrapping_sub(0x324), r.ecx);
                // 0xa4de13: mov DWORD PTR [ebp-0x320],edx
                mem.write(r.ebp.wrapping_sub(0x320), r.edx);
                // 0xa4de19: mov DWORD PTR [ebp-0x304],esi
                mem.write(r.ebp.wrapping_sub(0x304), r.esi);
                // 0xa4de1f: mov DWORD PTR [ebp-0x300],edi
                mem.write(r.ebp.wrapping_sub(0x300), r.edi);
                // 0xa4de25: mov DWORD PTR [ebp-0xb0],eax
                mem.write(r.ebp.wrapping_sub(0xb0), r.eax);
                // 0xa4de2b: fstp DWORD PTR [ebp-0x140]
                mem.write(r.ebp.wrapping_sub(0x140), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4de31: fld DWORD PTR [ebp-0x140]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x140)));
                // 0xa4de37: fmul st,st(4)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 4));
                    set(&mut st, 0, x * y);
                }
                // 0xa4de39: fstp DWORD PTR ds:0xc8f7d0
                mem.write(mem.rebase(0xc8f7d0), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4de3f: fild DWORD PTR [ebp-0xb8]
                st.push(f64::from(mem.read::<i32>(r.ebp.wrapping_sub(0xb8))));
                // 0xa4de45: fstp DWORD PTR [ebp-0x140]
                mem.write(r.ebp.wrapping_sub(0x140), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4de4b: fld DWORD PTR [ebp-0x140]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x140)));
                // 0xa4de51: fmulp st(4),st
                {
                    let (x, y) = (sti(&st, 4), sti(&st, 0));
                    set(&mut st, 4, x * y);
                }
                st.pop();
                // 0xa4de53: fxch st(3)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 3);
                }
                // 0xa4de55: fstp DWORD PTR ds:0xc8f7d4
                mem.write(mem.rebase(0xc8f7d4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4de5b: fld DWORD PTR ds:0xc0a00c
                st.push(ld32(mem, mem.rebase(0xc0a00c)));
                // 0xa4de61: fstp DWORD PTR ds:0xc8f7d8
                mem.write(mem.rebase(0xc8f7d8), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4de67: fld DWORD PTR ds:0xc0a010
                st.push(ld32(mem, mem.rebase(0xc0a010)));
                // 0xa4de6d: fst DWORD PTR ds:0xc8f7dc
                mem.write(mem.rebase(0xc8f7dc), crate::x87::store(st0(&st)));
                // 0xa4de73: fld DWORD PTR ds:0xc8f950
                st.push(ld32(mem, mem.rebase(0xc8f950)));
                // 0xa4de79: fld DWORD PTR ds:0xc8f7d0
                st.push(ld32(mem, mem.rebase(0xc8f7d0)));
                // 0xa4de7f: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4de81: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0xa4de83: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4de85: fstp DWORD PTR ds:0xc8f790
                mem.write(mem.rebase(0xc8f790), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4de8b: fld DWORD PTR ds:0xc8f954
                st.push(ld32(mem, mem.rebase(0xc8f954)));
                // 0xa4de91: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4de93: fstp DWORD PTR ds:0xc8f794
                mem.write(mem.rebase(0xc8f794), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4de99: fld DWORD PTR ds:0xc8f958
                st.push(ld32(mem, mem.rebase(0xc8f958)));
                // 0xa4de9f: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4dea1: fstp DWORD PTR ds:0xc8f798
                mem.write(mem.rebase(0xc8f798), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4dea7: fmul DWORD PTR ds:0xc8f95c
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f95c)));
                }
                // 0xa4dead: fstp DWORD PTR ds:0xc8f79c
                mem.write(mem.rebase(0xc8f79c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4deb3: fld DWORD PTR ds:0xc8f960
                st.push(ld32(mem, mem.rebase(0xc8f960)));
                // 0xa4deb9: fld DWORD PTR ds:0xc8f7d4
                st.push(ld32(mem, mem.rebase(0xc8f7d4)));
                // 0xa4debf: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4dec1: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0xa4dec3: fld DWORD PTR ds:0xc8f790
                st.push(ld32(mem, mem.rebase(0xc8f790)));
                // 0xa4dec9: faddp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x + y);
                }
                st.pop();
                // 0xa4decb: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4decd: fstp DWORD PTR ds:0xc8f790
                mem.write(mem.rebase(0xc8f790), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4ded3: fld DWORD PTR ds:0xc8f964
                st.push(ld32(mem, mem.rebase(0xc8f964)));
                // 0xa4ded9: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4dedb: fadd DWORD PTR ds:0xc8f794
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f794)));
                }
                // 0xa4dee1: fstp DWORD PTR ds:0xc8f794
                mem.write(mem.rebase(0xc8f794), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4dee7: fld DWORD PTR ds:0xc8f968
                st.push(ld32(mem, mem.rebase(0xc8f968)));
                // 0xa4deed: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4deef: fadd DWORD PTR ds:0xc8f798
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f798)));
                }
                // 0xa4def5: fstp DWORD PTR ds:0xc8f798
                mem.write(mem.rebase(0xc8f798), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4defb: fmul DWORD PTR ds:0xc8f96c
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f96c)));
                }
                // 0xa4df01: fadd DWORD PTR ds:0xc8f79c
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xc8f79c)));
                }
                // 0xa4df07: fstp DWORD PTR ds:0xc8f79c
                mem.write(mem.rebase(0xc8f79c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4df0d: fld DWORD PTR ds:0xc8f790
                st.push(ld32(mem, mem.rebase(0xc8f790)));
                // 0xa4df13: faddp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x + y);
                }
                st.pop();
                // 0xa4df15: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4df17: fstp DWORD PTR ds:0xc8f790
                mem.write(mem.rebase(0xc8f790), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4df1d: fld DWORD PTR ds:0xc8f794
                st.push(ld32(mem, mem.rebase(0xc8f794)));
                // 0xa4df23: faddp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x + y);
                }
                st.pop();
                // 0xa4df25: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4df27: fstp DWORD PTR ds:0xc8f794
                mem.write(mem.rebase(0xc8f794), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4df2d: fld DWORD PTR ds:0xc8f798
                st.push(ld32(mem, mem.rebase(0xc8f798)));
                // 0xa4df33: fadd QWORD PTR [ebp-0x1d8]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + mem.read::<f64>(r.ebp.wrapping_sub(0x1d8)));
                }
                // 0xa4df39: fstp DWORD PTR ds:0xc8f798
                mem.write(mem.rebase(0xc8f798), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4df3f: fld DWORD PTR ds:0xc8f79c
                st.push(ld32(mem, mem.rebase(0xc8f79c)));
                // 0xa4df45: fadd QWORD PTR [ebp-0x148]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + mem.read::<f64>(r.ebp.wrapping_sub(0x148)));
                }
                // 0xa4df4b: fstp DWORD PTR ds:0xc8f79c
                mem.write(mem.rebase(0xc8f79c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4df51: fld DWORD PTR ds:0xc8f790
                st.push(ld32(mem, mem.rebase(0xc8f790)));
                // 0xa4df57: fadd QWORD PTR [ebp-0x1d0]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + mem.read::<f64>(r.ebp.wrapping_sub(0x1d0)));
                }
                // 0xa4df5d: fstp DWORD PTR ds:0xc8f940
                mem.write(mem.rebase(0xc8f940), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4df63: mov ecx,DWORD PTR ds:0xc8f940
                r.ecx = mem.read::<u32>(mem.rebase(0xc8f940));
                // 0xa4df69: fld DWORD PTR ds:0xc8f794
                st.push(ld32(mem, mem.rebase(0xc8f794)));
                // 0xa4df6f: mov DWORD PTR [ebp-0x19c],ecx
                mem.write(r.ebp.wrapping_sub(0x19c), r.ecx);
                // 0xa4df75: fadd QWORD PTR [ebp-0x1c8]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + mem.read::<f64>(r.ebp.wrapping_sub(0x1c8)));
                }
                // 0xa4df7b: fstp DWORD PTR ds:0xc8f944
                mem.write(mem.rebase(0xc8f944), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4df81: mov edx,DWORD PTR ds:0xc8f944
                r.edx = mem.read::<u32>(mem.rebase(0xc8f944));
                // 0xa4df87: fld DWORD PTR ds:0xc8f798
                st.push(ld32(mem, mem.rebase(0xc8f798)));
                // 0xa4df8d: mov DWORD PTR [ebp-0x198],edx
                mem.write(r.ebp.wrapping_sub(0x198), r.edx);
                // 0xa4df93: fadd QWORD PTR [ebp-0x1a8]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + mem.read::<f64>(r.ebp.wrapping_sub(0x1a8)));
                }
                // 0xa4df99: fstp DWORD PTR ds:0xc8f948
                mem.write(mem.rebase(0xc8f948), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4df9f: mov eax,ds:0xc8f948
                r.eax = mem.read::<u32>(mem.rebase(0xc8f948));
                // 0xa4dfa4: fld DWORD PTR ds:0xc8f79c
                st.push(ld32(mem, mem.rebase(0xc8f79c)));
                // 0xa4dfaa: mov DWORD PTR [ebp-0x194],eax
                mem.write(r.ebp.wrapping_sub(0x194), r.eax);
                // 0xa4dfb0: fadd QWORD PTR [ebp-0x1b0]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + mem.read::<f64>(r.ebp.wrapping_sub(0x1b0)));
                }
                // 0xa4dfb6: fstp DWORD PTR ds:0xc8f94c
                mem.write(mem.rebase(0xc8f94c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4dfbc: mov eax,ds:0xc8f94c
                r.eax = mem.read::<u32>(mem.rebase(0xc8f94c));
                // 0xa4dfc1: mov DWORD PTR [ebp-0x190],eax
                mem.write(r.ebp.wrapping_sub(0x190), r.eax);
                // 0xa4dfc7: fld DWORD PTR [ebp-0x190]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x190)));
                // 0xa4dfcd: fcom st(2)
                r.sw = fcom(st0(&st), sti(&st, 2));
                // 0xa4dfcf: fnstsw ax
                r.eax = (r.eax & 0xffff_00ff) | (u32::from(r.sw) << 8);
                // 0xa4dfd1: fstp st(2)
                {
                    let x = st0(&st);
                    set(&mut st, 2, x);
                }
                st.pop();
                // 0xa4dfd3: test ah,0x41
                r.flag = ((r.eax >> 8) & 0x41, 0, true);
                r.cmp = false;
                // 0xa4dfd6: jnp 0xa4f211
                pc = if !r.pf() { 0xa4f211 } else { 0xa4dfdc };
            }
            0xa4dfdc => {
                // 0xa4dfdc: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4dfde: mov eax,DWORD PTR [ebp-0x17c]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x17c));
                // 0xa4dfe4: fdiv DWORD PTR [ebp-0x170]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x / ld32(mem, r.ebp.wrapping_sub(0x170)));
                }
                // 0xa4dfea: mov ds:0xc8f7d0,eax
                mem.write(mem.rebase(0xc8f7d0), r.eax);
                // 0xa4dfef: mov eax,DWORD PTR [ebp-0x178]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x178));
                // 0xa4dff5: mov ds:0xc8f7d4,eax
                mem.write(mem.rebase(0xc8f7d4), r.eax);
                // 0xa4dffa: mov eax,DWORD PTR [ebp-0x174]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x174));
                // 0xa4e000: mov ds:0xc8f7d8,eax
                mem.write(mem.rebase(0xc8f7d8), r.eax);
                // 0xa4e005: mov eax,DWORD PTR [ebp-0x170]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x170));
                // 0xa4e00b: mov ds:0xc8f7dc,eax
                mem.write(mem.rebase(0xc8f7dc), r.eax);
                // 0xa4e010: fstp DWORD PTR ds:0xc8f788
                mem.write(mem.rebase(0xc8f788), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e016: fld DWORD PTR [ebp-0x17c]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x17c)));
                // 0xa4e01c: fld DWORD PTR ds:0xc8f788
                st.push(ld32(mem, mem.rebase(0xc8f788)));
                // 0xa4e022: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4e024: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0xa4e026: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4e028: fstp DWORD PTR ds:0xc8f7d0
                mem.write(mem.rebase(0xc8f7d0), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e02e: mov eax,ds:0xc8f7d0
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d0));
                // 0xa4e033: mov DWORD PTR [ebp-0x140],eax
                mem.write(r.ebp.wrapping_sub(0x140), r.eax);
                // 0xa4e039: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4e03b: mov DWORD PTR [ebp-0x17c],eax
                mem.write(r.ebp.wrapping_sub(0x17c), r.eax);
                // 0xa4e041: fmul DWORD PTR ds:0xc8f7d4
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f7d4)));
                }
                // 0xa4e047: mov eax,DWORD PTR [ebp-0x16c]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x16c));
                // 0xa4e04d: mov ds:0xc8f7d0,eax
                mem.write(mem.rebase(0xc8f7d0), r.eax);
                // 0xa4e052: mov eax,DWORD PTR [ebp-0x168]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x168));
                // 0xa4e058: fstp DWORD PTR ds:0xc8f7d4
                mem.write(mem.rebase(0xc8f7d4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e05e: mov ebx,DWORD PTR ds:0xc8f7d4
                r.ebx = mem.read::<u32>(mem.rebase(0xc8f7d4));
                // 0xa4e064: mov ds:0xc8f7d4,eax
                mem.write(mem.rebase(0xc8f7d4), r.eax);
                // 0xa4e069: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4e06b: mov eax,DWORD PTR [ebp-0x164]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x164));
                // 0xa4e071: fmul DWORD PTR ds:0xc8f7d8
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f7d8)));
                }
                // 0xa4e077: mov DWORD PTR [ebp-0x178],ebx
                mem.write(r.ebp.wrapping_sub(0x178), r.ebx);
                // 0xa4e07d: fstp DWORD PTR ds:0xc8f7d8
                mem.write(mem.rebase(0xc8f7d8), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e083: mov edi,DWORD PTR ds:0xc8f7d8
                r.edi = mem.read::<u32>(mem.rebase(0xc8f7d8));
                // 0xa4e089: mov ds:0xc8f7d8,eax
                mem.write(mem.rebase(0xc8f7d8), r.eax);
                // 0xa4e08e: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4e090: mov eax,DWORD PTR [ebp-0x160]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x160));
                // 0xa4e096: fmul DWORD PTR ds:0xc8f7dc
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f7dc)));
                }
                // 0xa4e09c: mov DWORD PTR [ebp-0x174],edi
                mem.write(r.ebp.wrapping_sub(0x174), r.edi);
                // 0xa4e0a2: fstp DWORD PTR ds:0xc8f7dc
                mem.write(mem.rebase(0xc8f7dc), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e0a8: mov esi,DWORD PTR ds:0xc8f7dc
                r.esi = mem.read::<u32>(mem.rebase(0xc8f7dc));
                // 0xa4e0ae: fld DWORD PTR ds:0xc0a004
                st.push(ld32(mem, mem.rebase(0xc0a004)));
                // 0xa4e0b4: mov ds:0xc8f7dc,eax
                mem.write(mem.rebase(0xc8f7dc), r.eax);
                // 0xa4e0b9: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4e0bb: mov DWORD PTR [ebp-0x170],esi
                mem.write(r.ebp.wrapping_sub(0x170), r.esi);
                // 0xa4e0c1: faddp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x + y);
                }
                st.pop();
                // 0xa4e0c3: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4e0c5: fstp DWORD PTR [ebp-0x1c4]
                mem.write(r.ebp.wrapping_sub(0x1c4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e0cb: fld st(1)
                {
                    let x = sti(&st, 1);
                    st.push(x);
                }
                // 0xa4e0cd: fdiv DWORD PTR [ebp-0x160]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x / ld32(mem, r.ebp.wrapping_sub(0x160)));
                }
                // 0xa4e0d3: fstp DWORD PTR ds:0xc8f788
                mem.write(mem.rebase(0xc8f788), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e0d9: fld DWORD PTR [ebp-0x16c]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x16c)));
                // 0xa4e0df: fld DWORD PTR ds:0xc8f788
                st.push(ld32(mem, mem.rebase(0xc8f788)));
                // 0xa4e0e5: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4e0e7: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0xa4e0e9: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4e0eb: fstp DWORD PTR ds:0xc8f7d0
                mem.write(mem.rebase(0xc8f7d0), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e0f1: mov eax,ds:0xc8f7d0
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d0));
                // 0xa4e0f6: mov DWORD PTR [ebp-0x248],eax
                mem.write(r.ebp.wrapping_sub(0x248), r.eax);
                // 0xa4e0fc: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4e0fe: mov DWORD PTR [ebp-0x16c],eax
                mem.write(r.ebp.wrapping_sub(0x16c), r.eax);
                // 0xa4e104: fmul DWORD PTR ds:0xc8f7d4
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f7d4)));
                }
                // 0xa4e10a: fstp DWORD PTR ds:0xc8f7d4
                mem.write(mem.rebase(0xc8f7d4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e110: mov eax,ds:0xc8f7d4
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d4));
                // 0xa4e115: mov DWORD PTR [ebp-0x250],eax
                mem.write(r.ebp.wrapping_sub(0x250), r.eax);
                // 0xa4e11b: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4e11d: mov DWORD PTR [ebp-0x168],eax
                mem.write(r.ebp.wrapping_sub(0x168), r.eax);
                // 0xa4e123: fmul DWORD PTR ds:0xc8f7d8
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f7d8)));
                }
                // 0xa4e129: fstp DWORD PTR ds:0xc8f7d8
                mem.write(mem.rebase(0xc8f7d8), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e12f: mov eax,ds:0xc8f7d8
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d8));
                // 0xa4e134: mov DWORD PTR [ebp-0x258],eax
                mem.write(r.ebp.wrapping_sub(0x258), r.eax);
                // 0xa4e13a: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4e13c: mov DWORD PTR [ebp-0x164],eax
                mem.write(r.ebp.wrapping_sub(0x164), r.eax);
                // 0xa4e142: fmul DWORD PTR ds:0xc8f7dc
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f7dc)));
                }
                // 0xa4e148: fstp DWORD PTR ds:0xc8f7dc
                mem.write(mem.rebase(0xc8f7dc), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e14e: mov eax,ds:0xc8f7dc
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7dc));
                // 0xa4e153: mov DWORD PTR [ebp-0x260],eax
                mem.write(r.ebp.wrapping_sub(0x260), r.eax);
                // 0xa4e159: fadd st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x + y);
                }
                // 0xa4e15b: mov DWORD PTR [ebp-0x160],eax
                mem.write(r.ebp.wrapping_sub(0x160), r.eax);
                // 0xa4e161: mov eax,DWORD PTR [ebp-0x15c]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x15c));
                // 0xa4e167: fstp DWORD PTR [ebp-0x1cc]
                mem.write(r.ebp.wrapping_sub(0x1cc), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e16d: fld st(1)
                {
                    let x = sti(&st, 1);
                    st.push(x);
                }
                // 0xa4e16f: mov ds:0xc8f7d0,eax
                mem.write(mem.rebase(0xc8f7d0), r.eax);
                // 0xa4e174: fdiv DWORD PTR [ebp-0x150]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x / ld32(mem, r.ebp.wrapping_sub(0x150)));
                }
                // 0xa4e17a: mov eax,DWORD PTR [ebp-0x158]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x158));
                // 0xa4e180: mov ds:0xc8f7d4,eax
                mem.write(mem.rebase(0xc8f7d4), r.eax);
                // 0xa4e185: mov eax,DWORD PTR [ebp-0x154]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x154));
                // 0xa4e18b: mov ds:0xc8f7d8,eax
                mem.write(mem.rebase(0xc8f7d8), r.eax);
                // 0xa4e190: mov eax,DWORD PTR [ebp-0x150]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x150));
                // 0xa4e196: mov ds:0xc8f7dc,eax
                mem.write(mem.rebase(0xc8f7dc), r.eax);
                // 0xa4e19b: fstp DWORD PTR ds:0xc8f788
                mem.write(mem.rebase(0xc8f788), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e1a1: fld DWORD PTR [ebp-0x15c]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x15c)));
                // 0xa4e1a7: fld DWORD PTR ds:0xc8f788
                st.push(ld32(mem, mem.rebase(0xc8f788)));
                // 0xa4e1ad: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4e1af: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0xa4e1b1: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4e1b3: fstp DWORD PTR ds:0xc8f7d0
                mem.write(mem.rebase(0xc8f7d0), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e1b9: mov eax,ds:0xc8f7d0
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d0));
                // 0xa4e1be: mov DWORD PTR [ebp-0x230],eax
                mem.write(r.ebp.wrapping_sub(0x230), r.eax);
                // 0xa4e1c4: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4e1c6: mov DWORD PTR [ebp-0x15c],eax
                mem.write(r.ebp.wrapping_sub(0x15c), r.eax);
                // 0xa4e1cc: fmul DWORD PTR ds:0xc8f7d4
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f7d4)));
                }
                // 0xa4e1d2: mov DWORD PTR ds:0xc8f7d0,ecx
                mem.write(mem.rebase(0xc8f7d0), r.ecx);
                // 0xa4e1d8: fstp DWORD PTR ds:0xc8f7d4
                mem.write(mem.rebase(0xc8f7d4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e1de: mov eax,ds:0xc8f7d4
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d4));
                // 0xa4e1e3: mov DWORD PTR [ebp-0x25c],eax
                mem.write(r.ebp.wrapping_sub(0x25c), r.eax);
                // 0xa4e1e9: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4e1eb: mov DWORD PTR [ebp-0x158],eax
                mem.write(r.ebp.wrapping_sub(0x158), r.eax);
                // 0xa4e1f1: fmul DWORD PTR ds:0xc8f7d8
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f7d8)));
                }
                // 0xa4e1f7: mov DWORD PTR ds:0xc8f7d4,edx
                mem.write(mem.rebase(0xc8f7d4), r.edx);
                // 0xa4e1fd: fstp DWORD PTR ds:0xc8f7d8
                mem.write(mem.rebase(0xc8f7d8), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e203: mov eax,ds:0xc8f7d8
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d8));
                // 0xa4e208: mov DWORD PTR [ebp-0x24c],eax
                mem.write(r.ebp.wrapping_sub(0x24c), r.eax);
                // 0xa4e20e: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4e210: mov DWORD PTR [ebp-0x154],eax
                mem.write(r.ebp.wrapping_sub(0x154), r.eax);
                // 0xa4e216: fmul DWORD PTR ds:0xc8f7dc
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f7dc)));
                }
                // 0xa4e21c: fstp DWORD PTR ds:0xc8f7dc
                mem.write(mem.rebase(0xc8f7dc), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e222: mov eax,ds:0xc8f7dc
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7dc));
                // 0xa4e227: mov DWORD PTR [ebp-0x22c],eax
                mem.write(r.ebp.wrapping_sub(0x22c), r.eax);
                // 0xa4e22d: fadd st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x + y);
                }
                // 0xa4e22f: mov DWORD PTR [ebp-0x150],eax
                mem.write(r.ebp.wrapping_sub(0x150), r.eax);
                // 0xa4e235: mov eax,ds:0xc8f948
                r.eax = mem.read::<u32>(mem.rebase(0xc8f948));
                // 0xa4e23a: mov ds:0xc8f7d8,eax
                mem.write(mem.rebase(0xc8f7d8), r.eax);
                // 0xa4e23f: fstp DWORD PTR [ebp-0x144]
                mem.write(r.ebp.wrapping_sub(0x144), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e245: mov eax,ds:0xc8f94c
                r.eax = mem.read::<u32>(mem.rebase(0xc8f94c));
                // 0xa4e24a: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4e24c: mov ds:0xc8f7dc,eax
                mem.write(mem.rebase(0xc8f7dc), r.eax);
                // 0xa4e251: fdivrp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, y / x);
                }
                st.pop();
                // 0xa4e253: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4e255: fstp DWORD PTR ds:0xc8f788
                mem.write(mem.rebase(0xc8f788), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e25b: fld DWORD PTR [ebp-0x19c]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x19c)));
                // 0xa4e261: fld DWORD PTR ds:0xc8f788
                st.push(ld32(mem, mem.rebase(0xc8f788)));
                // 0xa4e267: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4e269: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0xa4e26b: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4e26d: fstp DWORD PTR ds:0xc8f7d0
                mem.write(mem.rebase(0xc8f7d0), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e273: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4e275: fmul DWORD PTR ds:0xc8f7d4
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f7d4)));
                }
                // 0xa4e27b: fstp DWORD PTR ds:0xc8f7d4
                mem.write(mem.rebase(0xc8f7d4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e281: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4e283: fmul DWORD PTR ds:0xc8f7d8
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f7d8)));
                }
                // 0xa4e289: fstp DWORD PTR ds:0xc8f7d8
                mem.write(mem.rebase(0xc8f7d8), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e28f: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4e291: fmul DWORD PTR ds:0xc8f7dc
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f7dc)));
                }
                // 0xa4e297: fstp DWORD PTR ds:0xc8f7dc
                mem.write(mem.rebase(0xc8f7dc), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e29d: faddp st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, x + y);
                }
                st.pop();
                // 0xa4e29f: fstp DWORD PTR ds:0xc8f830
                mem.write(mem.rebase(0xc8f830), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e2a5: mov eax,ds:0xc8f830
                r.eax = mem.read::<u32>(mem.rebase(0xc8f830));
                // 0xa4e2aa: mov DWORD PTR [ebp-0x284],eax
                mem.write(r.ebp.wrapping_sub(0x284), r.eax);
                // 0xa4e2b0: mov eax,ds:0xc8f7d0
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d0));
                // 0xa4e2b5: mov DWORD PTR [ebp-0x234],eax
                mem.write(r.ebp.wrapping_sub(0x234), r.eax);
                // 0xa4e2bb: mov DWORD PTR [ebp-0x19c],eax
                mem.write(r.ebp.wrapping_sub(0x19c), r.eax);
                // 0xa4e2c1: mov eax,ds:0xc8f7d4
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d4));
                // 0xa4e2c6: mov DWORD PTR [ebp-0x1d4],eax
                mem.write(r.ebp.wrapping_sub(0x1d4), r.eax);
                // 0xa4e2cc: mov DWORD PTR [ebp-0x198],eax
                mem.write(r.ebp.wrapping_sub(0x198), r.eax);
                // 0xa4e2d2: mov eax,ds:0xc8f7d8
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d8));
                // 0xa4e2d7: mov DWORD PTR [ebp-0x1ac],eax
                mem.write(r.ebp.wrapping_sub(0x1ac), r.eax);
                // 0xa4e2dd: mov DWORD PTR [ebp-0x194],eax
                mem.write(r.ebp.wrapping_sub(0x194), r.eax);
                // 0xa4e2e3: mov eax,ds:0xc8f7dc
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7dc));
                // 0xa4e2e8: mov DWORD PTR [ebp-0x1a4],eax
                mem.write(r.ebp.wrapping_sub(0x1a4), r.eax);
                // 0xa4e2ee: mov DWORD PTR [ebp-0x190],eax
                mem.write(r.ebp.wrapping_sub(0x190), r.eax);
                // 0xa4e2f4: mov eax,DWORD PTR [ebp-0x294]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x294));
                // 0xa4e2fa: mov ds:0xc8f7d0,eax
                mem.write(mem.rebase(0xc8f7d0), r.eax);
                // 0xa4e2ff: mov eax,DWORD PTR [ebp-0x290]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x290));
                // 0xa4e305: mov edx,DWORD PTR [ebp-0x28c]
                r.edx = mem.read::<u32>(r.ebp.wrapping_sub(0x28c));
                // 0xa4e30b: mov ecx,DWORD PTR [ebp-0x288]
                r.ecx = mem.read::<u32>(r.ebp.wrapping_sub(0x288));
                // 0xa4e311: mov ds:0xc8f7d4,eax
                mem.write(mem.rebase(0xc8f7d4), r.eax);
                // 0xa4e316: mov eax,DWORD PTR [ebp-0x228]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x228));
                // 0xa4e31c: mov ds:0xc8f7e0,eax
                mem.write(mem.rebase(0xc8f7e0), r.eax);
                // 0xa4e321: mov eax,DWORD PTR [ebp-0x224]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x224));
                // 0xa4e327: mov ds:0xc8f7e4,eax
                mem.write(mem.rebase(0xc8f7e4), r.eax);
                // 0xa4e32c: mov eax,DWORD PTR [ebp-0x220]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x220));
                // 0xa4e332: mov ds:0xc8f7e8,eax
                mem.write(mem.rebase(0xc8f7e8), r.eax);
                // 0xa4e337: mov eax,DWORD PTR [ebp-0x21c]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x21c));
                // 0xa4e33d: mov ds:0xc8f7ec,eax
                mem.write(mem.rebase(0xc8f7ec), r.eax);
                // 0xa4e342: mov eax,DWORD PTR [ebp-0x140]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x140));
                // 0xa4e348: mov ds:0xc8f800,eax
                mem.write(mem.rebase(0xc8f800), r.eax);
                // 0xa4e34d: mov eax,ds:0xc8f508
                r.eax = mem.read::<u32>(mem.rebase(0xc8f508));
                // 0xa4e352: sub esp,0x10
                {
                    let v = r.esp.wrapping_sub(0x10u32);
                    r.esp = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0xa4e355: mov ds:0xc8fac4,eax
                mem.write(mem.rebase(0xc8fac4), r.eax);
                // 0xa4e35a: mov eax,esp
                r.eax = r.esp;
                // 0xa4e35c: push 0xc8f800
                {
                    let v = mem.rebase(0xc8f800);
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0xa4e361: mov DWORD PTR [eax],0x44bf0000
                mem.write(r.eax, 0x44bf0000u32);
                // 0xa4e367: mov DWORD PTR [eax+0x4],0x44c80000
                mem.write(r.eax.wrapping_add(0x4), 0x44c80000u32);
                // 0xa4e36e: mov DWORD PTR [eax+0x8],edx
                mem.write(r.eax.wrapping_add(0x8), r.edx);
                // 0xa4e371: push 0xc8f7f0
                {
                    let v = mem.rebase(0xc8f7f0);
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0xa4e376: mov DWORD PTR ds:0xc8f7d8,edx
                mem.write(mem.rebase(0xc8f7d8), r.edx);
                // 0xa4e37c: mov DWORD PTR ds:0xc8f7dc,ecx
                mem.write(mem.rebase(0xc8f7dc), r.ecx);
                // 0xa4e382: mov DWORD PTR ds:0xc8f7d4,0x44c80000
                mem.write(mem.rebase(0xc8f7d4), 0x44c80000u32);
                // 0xa4e38c: mov DWORD PTR ds:0xc8f7d0,0x44bf0000
                mem.write(mem.rebase(0xc8f7d0), 0x44bf0000u32);
                // 0xa4e396: mov DWORD PTR ds:0xc8f7e4,0x45100000
                mem.write(mem.rebase(0xc8f7e4), 0x45100000u32);
                // 0xa4e3a0: mov DWORD PTR ds:0xc8f7e0,0x45210000
                mem.write(mem.rebase(0xc8f7e0), 0x45210000u32);
                // 0xa4e3aa: mov DWORD PTR ds:0xc8f804,ebx
                mem.write(mem.rebase(0xc8f804), r.ebx);
                // 0xa4e3b0: mov DWORD PTR ds:0xc8f808,edi
                mem.write(mem.rebase(0xc8f808), r.edi);
                // 0xa4e3b6: mov DWORD PTR ds:0xc8f80c,esi
                mem.write(mem.rebase(0xc8f80c), r.esi);
                // 0xa4e3bc: mov DWORD PTR [eax+0xc],ecx
                mem.write(r.eax.wrapping_add(0xc), r.ecx);
                // 0xa4e3bf: call 0x75e870
                {
                    let a: Vec<u32> = (0..4)
                        .map(|k| mem.read::<u32>(r.esp.wrapping_add(4 * k)))
                        .collect();
                    let f = mem.rebase(0x75e870);
                    r.eax = call(mem, f, &a);
                }
                // 0xa4e3c4: mov eax,ds:0xc8f800
                r.eax = mem.read::<u32>(mem.rebase(0xc8f800));
                // 0xa4e3c9: add esp,0x8
                {
                    let v = r.esp.wrapping_add(0x8u32);
                    r.esp = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0xa4e3cc: mov ecx,esp
                r.ecx = r.esp;
                // 0xa4e3ce: push 0xc8f7e0
                {
                    let v = mem.rebase(0xc8f7e0);
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0xa4e3d3: mov DWORD PTR [ecx],eax
                mem.write(r.ecx, r.eax);
                // 0xa4e3d5: mov eax,ds:0xc8f804
                r.eax = mem.read::<u32>(mem.rebase(0xc8f804));
                // 0xa4e3da: mov DWORD PTR [ecx+0x4],eax
                mem.write(r.ecx.wrapping_add(0x4), r.eax);
                // 0xa4e3dd: mov eax,ds:0xc8f808
                r.eax = mem.read::<u32>(mem.rebase(0xc8f808));
                // 0xa4e3e2: mov DWORD PTR [ecx+0x8],eax
                mem.write(r.ecx.wrapping_add(0x8), r.eax);
                // 0xa4e3e5: mov eax,ds:0xc8f80c
                r.eax = mem.read::<u32>(mem.rebase(0xc8f80c));
                // 0xa4e3ea: push 0xc8f7f0
                {
                    let v = mem.rebase(0xc8f7f0);
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0xa4e3ef: mov DWORD PTR [ecx+0xc],eax
                mem.write(r.ecx.wrapping_add(0xc), r.eax);
                // 0xa4e3f2: call 0x75e870
                {
                    let a: Vec<u32> = (0..4)
                        .map(|k| mem.read::<u32>(r.esp.wrapping_add(4 * k)))
                        .collect();
                    let f = mem.rebase(0x75e870);
                    r.eax = call(mem, f, &a);
                }
                // 0xa4e3f7: add esp,0x18
                {
                    let v = r.esp.wrapping_add(0x18u32);
                    r.esp = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0xa4e3fa: test BYTE PTR ds:0xc8fac4,0xc0
                r.flag = (
                    u32::from(mem.read::<u8>(mem.rebase(0xc8fac4))) & 0xc0u32,
                    0,
                    false,
                );
                r.cmp = false;
                // 0xa4e401: je 0xa4e670
                pc = if r.zf() { 0xa4e670 } else { 0xa4e407 };
            }
            0xa4e407 => {
                // 0xa4e407: mov eax,DWORD PTR [ebp-0x248]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x248));
                // 0xa4e40d: mov ds:0xc8f800,eax
                mem.write(mem.rebase(0xc8f800), r.eax);
                // 0xa4e412: mov eax,DWORD PTR [ebp-0x250]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x250));
                // 0xa4e418: mov ds:0xc8f804,eax
                mem.write(mem.rebase(0xc8f804), r.eax);
                // 0xa4e41d: mov eax,DWORD PTR [ebp-0x258]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x258));
                // 0xa4e423: mov ds:0xc8f808,eax
                mem.write(mem.rebase(0xc8f808), r.eax);
                // 0xa4e428: mov eax,DWORD PTR [ebp-0x260]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x260));
                // 0xa4e42e: mov ds:0xc8f80c,eax
                mem.write(mem.rebase(0xc8f80c), r.eax);
                // 0xa4e433: mov eax,ds:0xc8f508
                r.eax = mem.read::<u32>(mem.rebase(0xc8f508));
                // 0xa4e438: sub esp,0x10
                {
                    let v = r.esp.wrapping_sub(0x10u32);
                    r.esp = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0xa4e43b: mov ecx,esp
                r.ecx = r.esp;
                // 0xa4e43d: mov ds:0xc8fac4,eax
                mem.write(mem.rebase(0xc8fac4), r.eax);
                // 0xa4e442: mov eax,ds:0xc8f7d8
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d8));
                // 0xa4e447: mov DWORD PTR [ecx],0x44bf0000
                mem.write(r.ecx, 0x44bf0000u32);
                // 0xa4e44d: mov DWORD PTR [ecx+0x4],0x44c80000
                mem.write(r.ecx.wrapping_add(0x4), 0x44c80000u32);
                // 0xa4e454: mov DWORD PTR [ecx+0x8],eax
                mem.write(r.ecx.wrapping_add(0x8), r.eax);
                // 0xa4e457: mov eax,ds:0xc8f7dc
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7dc));
                // 0xa4e45c: push 0xc8f800
                {
                    let v = mem.rebase(0xc8f800);
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0xa4e461: push 0xc8f7f0
                {
                    let v = mem.rebase(0xc8f7f0);
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0xa4e466: mov DWORD PTR ds:0xc8f7d4,0x44c80000
                mem.write(mem.rebase(0xc8f7d4), 0x44c80000u32);
                // 0xa4e470: mov DWORD PTR ds:0xc8f7d0,0x44bf0000
                mem.write(mem.rebase(0xc8f7d0), 0x44bf0000u32);
                // 0xa4e47a: mov DWORD PTR ds:0xc8f7e4,0x45100000
                mem.write(mem.rebase(0xc8f7e4), 0x45100000u32);
                // 0xa4e484: mov DWORD PTR ds:0xc8f7e0,0x45210000
                mem.write(mem.rebase(0xc8f7e0), 0x45210000u32);
                // 0xa4e48e: mov DWORD PTR [ecx+0xc],eax
                mem.write(r.ecx.wrapping_add(0xc), r.eax);
                // 0xa4e491: call 0x75e870
                {
                    let a: Vec<u32> = (0..4)
                        .map(|k| mem.read::<u32>(r.esp.wrapping_add(4 * k)))
                        .collect();
                    let f = mem.rebase(0x75e870);
                    r.eax = call(mem, f, &a);
                }
                // 0xa4e496: mov eax,ds:0xc8f800
                r.eax = mem.read::<u32>(mem.rebase(0xc8f800));
                // 0xa4e49b: add esp,0x8
                {
                    let v = r.esp.wrapping_add(0x8u32);
                    r.esp = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0xa4e49e: mov ecx,esp
                r.ecx = r.esp;
                // 0xa4e4a0: push 0xc8f7e0
                {
                    let v = mem.rebase(0xc8f7e0);
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0xa4e4a5: mov DWORD PTR [ecx],eax
                mem.write(r.ecx, r.eax);
                // 0xa4e4a7: mov eax,ds:0xc8f804
                r.eax = mem.read::<u32>(mem.rebase(0xc8f804));
                // 0xa4e4ac: mov DWORD PTR [ecx+0x4],eax
                mem.write(r.ecx.wrapping_add(0x4), r.eax);
                // 0xa4e4af: mov eax,ds:0xc8f808
                r.eax = mem.read::<u32>(mem.rebase(0xc8f808));
                // 0xa4e4b4: mov DWORD PTR [ecx+0x8],eax
                mem.write(r.ecx.wrapping_add(0x8), r.eax);
                // 0xa4e4b7: mov eax,ds:0xc8f80c
                r.eax = mem.read::<u32>(mem.rebase(0xc8f80c));
                // 0xa4e4bc: push 0xc8f7f0
                {
                    let v = mem.rebase(0xc8f7f0);
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0xa4e4c1: mov DWORD PTR [ecx+0xc],eax
                mem.write(r.ecx.wrapping_add(0xc), r.eax);
                // 0xa4e4c4: call 0x75e870
                {
                    let a: Vec<u32> = (0..4)
                        .map(|k| mem.read::<u32>(r.esp.wrapping_add(4 * k)))
                        .collect();
                    let f = mem.rebase(0x75e870);
                    r.eax = call(mem, f, &a);
                }
                // 0xa4e4c9: add esp,0x18
                {
                    let v = r.esp.wrapping_add(0x18u32);
                    r.esp = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0xa4e4cc: test BYTE PTR ds:0xc8fac4,0xc0
                r.flag = (
                    u32::from(mem.read::<u8>(mem.rebase(0xc8fac4))) & 0xc0u32,
                    0,
                    false,
                );
                r.cmp = false;
                // 0xa4e4d3: je 0xa4e670
                pc = if r.zf() { 0xa4e670 } else { 0xa4e4d9 };
            }
            0xa4e4d9 => {
                // 0xa4e4d9: mov eax,DWORD PTR [ebp-0x230]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x230));
                // 0xa4e4df: mov ds:0xc8f800,eax
                mem.write(mem.rebase(0xc8f800), r.eax);
                // 0xa4e4e4: mov eax,DWORD PTR [ebp-0x25c]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x25c));
                // 0xa4e4ea: mov ds:0xc8f804,eax
                mem.write(mem.rebase(0xc8f804), r.eax);
                // 0xa4e4ef: mov eax,DWORD PTR [ebp-0x24c]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x24c));
                // 0xa4e4f5: mov ds:0xc8f808,eax
                mem.write(mem.rebase(0xc8f808), r.eax);
                // 0xa4e4fa: mov eax,DWORD PTR [ebp-0x22c]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x22c));
                // 0xa4e500: mov ds:0xc8f80c,eax
                mem.write(mem.rebase(0xc8f80c), r.eax);
                // 0xa4e505: mov eax,ds:0xc8f508
                r.eax = mem.read::<u32>(mem.rebase(0xc8f508));
                // 0xa4e50a: sub esp,0x10
                {
                    let v = r.esp.wrapping_sub(0x10u32);
                    r.esp = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0xa4e50d: mov ecx,esp
                r.ecx = r.esp;
                // 0xa4e50f: mov ds:0xc8fac4,eax
                mem.write(mem.rebase(0xc8fac4), r.eax);
                // 0xa4e514: mov eax,ds:0xc8f7d8
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d8));
                // 0xa4e519: mov DWORD PTR [ecx],0x44bf0000
                mem.write(r.ecx, 0x44bf0000u32);
                // 0xa4e51f: mov DWORD PTR [ecx+0x4],0x44c80000
                mem.write(r.ecx.wrapping_add(0x4), 0x44c80000u32);
                // 0xa4e526: mov DWORD PTR [ecx+0x8],eax
                mem.write(r.ecx.wrapping_add(0x8), r.eax);
                // 0xa4e529: mov eax,ds:0xc8f7dc
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7dc));
                // 0xa4e52e: push 0xc8f800
                {
                    let v = mem.rebase(0xc8f800);
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0xa4e533: push 0xc8f7f0
                {
                    let v = mem.rebase(0xc8f7f0);
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0xa4e538: mov DWORD PTR ds:0xc8f7d4,0x44c80000
                mem.write(mem.rebase(0xc8f7d4), 0x44c80000u32);
                // 0xa4e542: mov DWORD PTR ds:0xc8f7d0,0x44bf0000
                mem.write(mem.rebase(0xc8f7d0), 0x44bf0000u32);
                // 0xa4e54c: mov DWORD PTR ds:0xc8f7e4,0x45100000
                mem.write(mem.rebase(0xc8f7e4), 0x45100000u32);
                // 0xa4e556: mov DWORD PTR ds:0xc8f7e0,0x45210000
                mem.write(mem.rebase(0xc8f7e0), 0x45210000u32);
                // 0xa4e560: mov DWORD PTR [ecx+0xc],eax
                mem.write(r.ecx.wrapping_add(0xc), r.eax);
                // 0xa4e563: call 0x75e870
                {
                    let a: Vec<u32> = (0..4)
                        .map(|k| mem.read::<u32>(r.esp.wrapping_add(4 * k)))
                        .collect();
                    let f = mem.rebase(0x75e870);
                    r.eax = call(mem, f, &a);
                }
                // 0xa4e568: mov eax,ds:0xc8f800
                r.eax = mem.read::<u32>(mem.rebase(0xc8f800));
                // 0xa4e56d: add esp,0x8
                {
                    let v = r.esp.wrapping_add(0x8u32);
                    r.esp = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0xa4e570: mov ecx,esp
                r.ecx = r.esp;
                // 0xa4e572: push 0xc8f7e0
                {
                    let v = mem.rebase(0xc8f7e0);
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0xa4e577: mov DWORD PTR [ecx],eax
                mem.write(r.ecx, r.eax);
                // 0xa4e579: mov eax,ds:0xc8f804
                r.eax = mem.read::<u32>(mem.rebase(0xc8f804));
                // 0xa4e57e: mov DWORD PTR [ecx+0x4],eax
                mem.write(r.ecx.wrapping_add(0x4), r.eax);
                // 0xa4e581: mov eax,ds:0xc8f808
                r.eax = mem.read::<u32>(mem.rebase(0xc8f808));
                // 0xa4e586: mov DWORD PTR [ecx+0x8],eax
                mem.write(r.ecx.wrapping_add(0x8), r.eax);
                // 0xa4e589: mov eax,ds:0xc8f80c
                r.eax = mem.read::<u32>(mem.rebase(0xc8f80c));
                // 0xa4e58e: push 0xc8f7f0
                {
                    let v = mem.rebase(0xc8f7f0);
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0xa4e593: mov DWORD PTR [ecx+0xc],eax
                mem.write(r.ecx.wrapping_add(0xc), r.eax);
                // 0xa4e596: call 0x75e870
                {
                    let a: Vec<u32> = (0..4)
                        .map(|k| mem.read::<u32>(r.esp.wrapping_add(4 * k)))
                        .collect();
                    let f = mem.rebase(0x75e870);
                    r.eax = call(mem, f, &a);
                }
                // 0xa4e59b: add esp,0x18
                {
                    let v = r.esp.wrapping_add(0x18u32);
                    r.esp = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0xa4e59e: test BYTE PTR ds:0xc8fac4,0xc0
                r.flag = (
                    u32::from(mem.read::<u8>(mem.rebase(0xc8fac4))) & 0xc0u32,
                    0,
                    false,
                );
                r.cmp = false;
                // 0xa4e5a5: je 0xa4e670
                pc = if r.zf() { 0xa4e670 } else { 0xa4e5ab };
            }
            0xa4e5ab => {
                // 0xa4e5ab: mov eax,DWORD PTR [ebp-0x234]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x234));
                // 0xa4e5b1: mov ds:0xc8f800,eax
                mem.write(mem.rebase(0xc8f800), r.eax);
                // 0xa4e5b6: mov eax,DWORD PTR [ebp-0x1d4]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x1d4));
                // 0xa4e5bc: mov ds:0xc8f804,eax
                mem.write(mem.rebase(0xc8f804), r.eax);
                // 0xa4e5c1: mov eax,DWORD PTR [ebp-0x1ac]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x1ac));
                // 0xa4e5c7: mov ds:0xc8f808,eax
                mem.write(mem.rebase(0xc8f808), r.eax);
                // 0xa4e5cc: mov eax,DWORD PTR [ebp-0x1a4]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x1a4));
                // 0xa4e5d2: mov ds:0xc8f80c,eax
                mem.write(mem.rebase(0xc8f80c), r.eax);
                // 0xa4e5d7: mov eax,ds:0xc8f508
                r.eax = mem.read::<u32>(mem.rebase(0xc8f508));
                // 0xa4e5dc: sub esp,0x10
                {
                    let v = r.esp.wrapping_sub(0x10u32);
                    r.esp = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0xa4e5df: mov ecx,esp
                r.ecx = r.esp;
                // 0xa4e5e1: mov ds:0xc8fac4,eax
                mem.write(mem.rebase(0xc8fac4), r.eax);
                // 0xa4e5e6: mov eax,ds:0xc8f7d8
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d8));
                // 0xa4e5eb: mov DWORD PTR [ecx],0x44bf0000
                mem.write(r.ecx, 0x44bf0000u32);
                // 0xa4e5f1: mov DWORD PTR [ecx+0x4],0x44c80000
                mem.write(r.ecx.wrapping_add(0x4), 0x44c80000u32);
                // 0xa4e5f8: mov DWORD PTR [ecx+0x8],eax
                mem.write(r.ecx.wrapping_add(0x8), r.eax);
                // 0xa4e5fb: mov eax,ds:0xc8f7dc
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7dc));
                // 0xa4e600: push 0xc8f800
                {
                    let v = mem.rebase(0xc8f800);
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0xa4e605: push 0xc8f7f0
                {
                    let v = mem.rebase(0xc8f7f0);
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0xa4e60a: mov DWORD PTR ds:0xc8f7d4,0x44c80000
                mem.write(mem.rebase(0xc8f7d4), 0x44c80000u32);
                // 0xa4e614: mov DWORD PTR ds:0xc8f7d0,0x44bf0000
                mem.write(mem.rebase(0xc8f7d0), 0x44bf0000u32);
                // 0xa4e61e: mov DWORD PTR ds:0xc8f7e4,0x45100000
                mem.write(mem.rebase(0xc8f7e4), 0x45100000u32);
                // 0xa4e628: mov DWORD PTR ds:0xc8f7e0,0x45210000
                mem.write(mem.rebase(0xc8f7e0), 0x45210000u32);
                // 0xa4e632: mov DWORD PTR [ecx+0xc],eax
                mem.write(r.ecx.wrapping_add(0xc), r.eax);
                // 0xa4e635: call 0x75e870
                {
                    let a: Vec<u32> = (0..4)
                        .map(|k| mem.read::<u32>(r.esp.wrapping_add(4 * k)))
                        .collect();
                    let f = mem.rebase(0x75e870);
                    r.eax = call(mem, f, &a);
                }
                // 0xa4e63a: mov eax,ds:0xc8f800
                r.eax = mem.read::<u32>(mem.rebase(0xc8f800));
                // 0xa4e63f: add esp,0x8
                {
                    let v = r.esp.wrapping_add(0x8u32);
                    r.esp = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0xa4e642: mov ecx,esp
                r.ecx = r.esp;
                // 0xa4e644: push 0xc8f7e0
                {
                    let v = mem.rebase(0xc8f7e0);
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0xa4e649: mov DWORD PTR [ecx],eax
                mem.write(r.ecx, r.eax);
                // 0xa4e64b: mov eax,ds:0xc8f804
                r.eax = mem.read::<u32>(mem.rebase(0xc8f804));
                // 0xa4e650: mov DWORD PTR [ecx+0x4],eax
                mem.write(r.ecx.wrapping_add(0x4), r.eax);
                // 0xa4e653: mov eax,ds:0xc8f808
                r.eax = mem.read::<u32>(mem.rebase(0xc8f808));
                // 0xa4e658: mov DWORD PTR [ecx+0x8],eax
                mem.write(r.ecx.wrapping_add(0x8), r.eax);
                // 0xa4e65b: mov eax,ds:0xc8f80c
                r.eax = mem.read::<u32>(mem.rebase(0xc8f80c));
                // 0xa4e660: push 0xc8f7f0
                {
                    let v = mem.rebase(0xc8f7f0);
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0xa4e665: mov DWORD PTR [ecx+0xc],eax
                mem.write(r.ecx.wrapping_add(0xc), r.eax);
                // 0xa4e668: call 0x75e870
                {
                    let a: Vec<u32> = (0..4)
                        .map(|k| mem.read::<u32>(r.esp.wrapping_add(4 * k)))
                        .collect();
                    let f = mem.rebase(0x75e870);
                    r.eax = call(mem, f, &a);
                }
                // 0xa4e66d: add esp,0x18
                {
                    let v = r.esp.wrapping_add(0x18u32);
                    r.esp = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                pc = 0xa4e670;
            }
            0xa4e670 => {
                // 0xa4e670: mov ebx,DWORD PTR [ebp-0x14c]
                r.ebx = mem.read::<u32>(r.ebp.wrapping_sub(0x14c));
                // 0xa4e676: movsx eax,WORD PTR [ebx+0x30]
                r.eax = i32::from(mem.read::<i16>(r.ebx.wrapping_add(0x30))).cast_unsigned();
                // 0xa4e67a: cdq
                r.edx = if r.eax.cast_signed() < 0 { u32::MAX } else { 0 };
                // 0xa4e67b: mov esi,eax
                r.esi = r.eax;
                // 0xa4e67d: mov edi,edx
                r.edi = r.edx;
                // 0xa4e67f: mov DWORD PTR ds:0xc8f588,esi
                mem.write(mem.rebase(0xc8f588), r.esi);
                // 0xa4e685: mov DWORD PTR ds:0xc8f58c,edi
                mem.write(mem.rebase(0xc8f58c), r.edi);
                // 0xa4e68b: movsx eax,WORD PTR [ebx+0x32]
                r.eax = i32::from(mem.read::<i16>(r.ebx.wrapping_add(0x32))).cast_unsigned();
                // 0xa4e68f: cdq
                r.edx = if r.eax.cast_signed() < 0 { u32::MAX } else { 0 };
                // 0xa4e690: mov ecx,eax
                r.ecx = r.eax;
                // 0xa4e692: mov eax,ds:0xc8f5a0
                r.eax = mem.read::<u32>(mem.rebase(0xc8f5a0));
                // 0xa4e697: mov DWORD PTR [ebp-0x2ac],eax
                mem.write(r.ebp.wrapping_sub(0x2ac), r.eax);
                // 0xa4e69d: mov eax,ds:0xc8f5a4
                r.eax = mem.read::<u32>(mem.rebase(0xc8f5a4));
                // 0xa4e6a2: mov DWORD PTR [ebp-0x2a8],eax
                mem.write(r.ebp.wrapping_sub(0x2a8), r.eax);
                // 0xa4e6a8: mov eax,ds:0xc8f590
                r.eax = mem.read::<u32>(mem.rebase(0xc8f590));
                // 0xa4e6ad: mov DWORD PTR [ebp-0x2cc],eax
                mem.write(r.ebp.wrapping_sub(0x2cc), r.eax);
                // 0xa4e6b3: mov eax,ds:0xc8f594
                r.eax = mem.read::<u32>(mem.rebase(0xc8f594));
                // 0xa4e6b8: mov DWORD PTR [ebp-0x2c8],eax
                mem.write(r.ebp.wrapping_sub(0x2c8), r.eax);
                // 0xa4e6be: mov DWORD PTR ds:0xc8f588,esi
                mem.write(mem.rebase(0xc8f588), r.esi);
                // 0xa4e6c4: mov DWORD PTR ds:0xc8f58c,ecx
                mem.write(mem.rebase(0xc8f58c), r.ecx);
                // 0xa4e6ca: fld DWORD PTR ds:0xc8f588
                st.push(ld32(mem, mem.rebase(0xc8f588)));
                // 0xa4e6d0: mov DWORD PTR ds:0xc8f590,edi
                mem.write(mem.rebase(0xc8f590), r.edi);
                // 0xa4e6d6: fstp DWORD PTR ds:0xc8f7d0
                mem.write(mem.rebase(0xc8f7d0), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e6dc: mov eax,ds:0xc8f7d0
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d0));
                // 0xa4e6e1: fld DWORD PTR ds:0xc8f58c
                st.push(ld32(mem, mem.rebase(0xc8f58c)));
                // 0xa4e6e7: mov DWORD PTR [ebp-0xfc],eax
                mem.write(r.ebp.wrapping_sub(0xfc), r.eax);
                // 0xa4e6ed: fstp DWORD PTR ds:0xc8f7d4
                mem.write(mem.rebase(0xc8f7d4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e6f3: mov eax,ds:0xc8f7d4
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d4));
                // 0xa4e6f8: fld DWORD PTR ds:0xc8f590
                st.push(ld32(mem, mem.rebase(0xc8f590)));
                // 0xa4e6fe: mov DWORD PTR [ebp-0xf8],eax
                mem.write(r.ebp.wrapping_sub(0xf8), r.eax);
                // 0xa4e704: fstp DWORD PTR ds:0xc8f7d8
                mem.write(mem.rebase(0xc8f7d8), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e70a: mov eax,ds:0xc8f7d8
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d8));
                // 0xa4e70f: mov DWORD PTR ds:0xc8f594,edx
                mem.write(mem.rebase(0xc8f594), r.edx);
                // 0xa4e715: mov DWORD PTR [ebp-0xf4],eax
                mem.write(r.ebp.wrapping_sub(0xf4), r.eax);
                // 0xa4e71b: fld DWORD PTR ds:0xc8f594
                st.push(ld32(mem, mem.rebase(0xc8f594)));
                // 0xa4e721: mov DWORD PTR ds:0xc8f59c,edx
                mem.write(mem.rebase(0xc8f59c), r.edx);
                // 0xa4e727: fstp DWORD PTR ds:0xc8f7dc
                mem.write(mem.rebase(0xc8f7dc), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e72d: mov eax,ds:0xc8f7dc
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7dc));
                // 0xa4e732: fild DWORD PTR [ebp-0xfc]
                st.push(f64::from(mem.read::<i32>(r.ebp.wrapping_sub(0xfc))));
                // 0xa4e738: mov DWORD PTR [ebp-0xf0],eax
                mem.write(r.ebp.wrapping_sub(0xf0), r.eax);
                // 0xa4e73e: mov DWORD PTR ds:0xc8f598,ecx
                mem.write(mem.rebase(0xc8f598), r.ecx);
                // 0xa4e744: mov DWORD PTR [ebp-0x2b0],edx
                mem.write(r.ebp.wrapping_sub(0x2b0), r.edx);
                // 0xa4e74a: mov DWORD PTR [ebp-0x2b4],ecx
                mem.write(r.ebp.wrapping_sub(0x2b4), r.ecx);
                // 0xa4e750: mov DWORD PTR [ebp-0x2d4],esi
                mem.write(r.ebp.wrapping_sub(0x2d4), r.esi);
                // 0xa4e756: mov DWORD PTR [ebp-0x2d0],edi
                mem.write(r.ebp.wrapping_sub(0x2d0), r.edi);
                // 0xa4e75c: fstp DWORD PTR [ebp-0x14c]
                mem.write(r.ebp.wrapping_sub(0x14c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e762: fld DWORD PTR [ebp-0x14c]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x14c)));
                // 0xa4e768: fld QWORD PTR ds:0xb50190
                st.push(mem.read::<f64>(mem.rebase(0xb50190)));
                // 0xa4e76e: fmul st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, x * y);
                }
                // 0xa4e770: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4e772: fstp DWORD PTR ds:0xc8f7d0
                mem.write(mem.rebase(0xc8f7d0), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e778: fild DWORD PTR [ebp-0xf8]
                st.push(f64::from(mem.read::<i32>(r.ebp.wrapping_sub(0xf8))));
                // 0xa4e77e: fstp DWORD PTR [ebp-0x14c]
                mem.write(r.ebp.wrapping_sub(0x14c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e784: fld DWORD PTR [ebp-0x14c]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x14c)));
                // 0xa4e78a: mov eax,ds:0xc8f7d0
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d0));
                // 0xa4e78f: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4e791: mov DWORD PTR [ebp-0x1c0],eax
                mem.write(r.ebp.wrapping_sub(0x1c0), r.eax);
                // 0xa4e797: fstp DWORD PTR ds:0xc8f7d4
                mem.write(mem.rebase(0xc8f7d4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e79d: mov eax,ds:0xc8f7d4
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d4));
                // 0xa4e7a2: fld DWORD PTR ds:0xc0a00c
                st.push(ld32(mem, mem.rebase(0xc0a00c)));
                // 0xa4e7a8: mov DWORD PTR [ebp-0x1bc],eax
                mem.write(r.ebp.wrapping_sub(0x1bc), r.eax);
                // 0xa4e7ae: fstp DWORD PTR ds:0xc8f7d8
                mem.write(mem.rebase(0xc8f7d8), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e7b4: mov eax,ds:0xc8f7d8
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d8));
                // 0xa4e7b9: fld DWORD PTR ds:0xc0a010
                st.push(ld32(mem, mem.rebase(0xc0a010)));
                // 0xa4e7bf: mov DWORD PTR [ebp-0x1b8],eax
                mem.write(r.ebp.wrapping_sub(0x1b8), r.eax);
                // 0xa4e7c5: fstp DWORD PTR ds:0xc8f7dc
                mem.write(mem.rebase(0xc8f7dc), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e7cb: mov eax,ds:0xc8f7dc
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7dc));
                // 0xa4e7d0: mov DWORD PTR [ebp-0x1b4],eax
                mem.write(r.ebp.wrapping_sub(0x1b4), r.eax);
                // 0xa4e7d6: movsx eax,WORD PTR [ebx+0x34]
                r.eax = i32::from(mem.read::<i16>(r.ebx.wrapping_add(0x34))).cast_unsigned();
                // 0xa4e7da: cdq
                r.edx = if r.eax.cast_signed() < 0 { u32::MAX } else { 0 };
                // 0xa4e7db: mov esi,eax
                r.esi = r.eax;
                // 0xa4e7dd: mov edi,edx
                r.edi = r.edx;
                // 0xa4e7df: mov DWORD PTR ds:0xc8f588,esi
                mem.write(mem.rebase(0xc8f588), r.esi);
                // 0xa4e7e5: mov DWORD PTR ds:0xc8f58c,edi
                mem.write(mem.rebase(0xc8f58c), r.edi);
                // 0xa4e7eb: movsx eax,WORD PTR [ebx+0x36]
                r.eax = i32::from(mem.read::<i16>(r.ebx.wrapping_add(0x36))).cast_unsigned();
                // 0xa4e7ef: cdq
                r.edx = if r.eax.cast_signed() < 0 { u32::MAX } else { 0 };
                // 0xa4e7f0: mov ecx,eax
                r.ecx = r.eax;
                // 0xa4e7f2: mov DWORD PTR ds:0xc8f598,ecx
                mem.write(mem.rebase(0xc8f598), r.ecx);
                // 0xa4e7f8: mov DWORD PTR ds:0xc8f59c,edx
                mem.write(mem.rebase(0xc8f59c), r.edx);
                // 0xa4e7fe: mov DWORD PTR [ebp-0x2f4],ecx
                mem.write(r.ebp.wrapping_sub(0x2f4), r.ecx);
                // 0xa4e804: mov DWORD PTR [ebp-0x2f0],edx
                mem.write(r.ebp.wrapping_sub(0x2f0), r.edx);
                // 0xa4e80a: mov eax,ds:0xc8f5a0
                r.eax = mem.read::<u32>(mem.rebase(0xc8f5a0));
                // 0xa4e80f: mov DWORD PTR [ebp-0x2ec],eax
                mem.write(r.ebp.wrapping_sub(0x2ec), r.eax);
                // 0xa4e815: mov eax,ds:0xc8f5a4
                r.eax = mem.read::<u32>(mem.rebase(0xc8f5a4));
                // 0xa4e81a: mov DWORD PTR [ebp-0x2e8],eax
                mem.write(r.ebp.wrapping_sub(0x2e8), r.eax);
                // 0xa4e820: mov eax,ds:0xc8f590
                r.eax = mem.read::<u32>(mem.rebase(0xc8f590));
                // 0xa4e825: mov DWORD PTR [ebp-0x30c],eax
                mem.write(r.ebp.wrapping_sub(0x30c), r.eax);
                // 0xa4e82b: mov eax,ds:0xc8f594
                r.eax = mem.read::<u32>(mem.rebase(0xc8f594));
                // 0xa4e830: mov DWORD PTR [ebp-0x308],eax
                mem.write(r.ebp.wrapping_sub(0x308), r.eax);
                // 0xa4e836: mov DWORD PTR ds:0xc8f588,esi
                mem.write(mem.rebase(0xc8f588), r.esi);
                // 0xa4e83c: fld DWORD PTR ds:0xc8f588
                st.push(ld32(mem, mem.rebase(0xc8f588)));
                // 0xa4e842: fstp DWORD PTR ds:0xc8f7d0
                mem.write(mem.rebase(0xc8f7d0), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e848: mov eax,ds:0xc8f7d0
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d0));
                // 0xa4e84d: mov DWORD PTR [ebp-0xec],eax
                mem.write(r.ebp.wrapping_sub(0xec), r.eax);
                // 0xa4e853: mov DWORD PTR ds:0xc8f58c,ecx
                mem.write(mem.rebase(0xc8f58c), r.ecx);
                // 0xa4e859: fld DWORD PTR ds:0xc8f58c
                st.push(ld32(mem, mem.rebase(0xc8f58c)));
                // 0xa4e85f: fstp DWORD PTR ds:0xc8f7d4
                mem.write(mem.rebase(0xc8f7d4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e865: mov eax,ds:0xc8f7d4
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d4));
                // 0xa4e86a: mov DWORD PTR ds:0xc8f590,edi
                mem.write(mem.rebase(0xc8f590), r.edi);
                // 0xa4e870: fld DWORD PTR ds:0xc8f590
                st.push(ld32(mem, mem.rebase(0xc8f590)));
                // 0xa4e876: fstp DWORD PTR ds:0xc8f7d8
                mem.write(mem.rebase(0xc8f7d8), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e87c: mov DWORD PTR [ebp-0xe8],eax
                mem.write(r.ebp.wrapping_sub(0xe8), r.eax);
                // 0xa4e882: mov eax,ds:0xc8f7d8
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d8));
                // 0xa4e887: mov DWORD PTR ds:0xc8f594,edx
                mem.write(mem.rebase(0xc8f594), r.edx);
                // 0xa4e88d: fld DWORD PTR ds:0xc8f594
                st.push(ld32(mem, mem.rebase(0xc8f594)));
                // 0xa4e893: fstp DWORD PTR ds:0xc8f7dc
                mem.write(mem.rebase(0xc8f7dc), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e899: mov DWORD PTR [ebp-0xe4],eax
                mem.write(r.ebp.wrapping_sub(0xe4), r.eax);
                // 0xa4e89f: mov eax,ds:0xc8f7dc
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7dc));
                // 0xa4e8a4: fild DWORD PTR [ebp-0xec]
                st.push(f64::from(mem.read::<i32>(r.ebp.wrapping_sub(0xec))));
                // 0xa4e8aa: mov DWORD PTR [ebp-0xe0],eax
                mem.write(r.ebp.wrapping_sub(0xe0), r.eax);
                // 0xa4e8b0: mov DWORD PTR [ebp-0x310],edi
                mem.write(r.ebp.wrapping_sub(0x310), r.edi);
                // 0xa4e8b6: mov DWORD PTR [ebp-0x314],esi
                mem.write(r.ebp.wrapping_sub(0x314), r.esi);
                // 0xa4e8bc: fstp DWORD PTR [ebp-0x14c]
                mem.write(r.ebp.wrapping_sub(0x14c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e8c2: fld DWORD PTR [ebp-0x14c]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x14c)));
                // 0xa4e8c8: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4e8ca: fstp DWORD PTR ds:0xc8f7d0
                mem.write(mem.rebase(0xc8f7d0), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e8d0: fild DWORD PTR [ebp-0xe8]
                st.push(f64::from(mem.read::<i32>(r.ebp.wrapping_sub(0xe8))));
                // 0xa4e8d6: fstp DWORD PTR [ebp-0x14c]
                mem.write(r.ebp.wrapping_sub(0x14c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e8dc: fmul DWORD PTR [ebp-0x14c]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, r.ebp.wrapping_sub(0x14c)));
                }
                // 0xa4e8e2: mov eax,ds:0xc8f7d0
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d0));
                // 0xa4e8e7: mov DWORD PTR [ebp-0x18c],eax
                mem.write(r.ebp.wrapping_sub(0x18c), r.eax);
                // 0xa4e8ed: mov edi,DWORD PTR ds:0x1a860ec
                r.edi = mem.read::<u32>(mem.rebase(0x1a860ec));
                // 0xa4e8f3: fstp DWORD PTR ds:0xc8f7d4
                mem.write(mem.rebase(0xc8f7d4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e8f9: mov eax,ds:0xc8f7d4
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d4));
                // 0xa4e8fe: fld DWORD PTR ds:0xc0a00c
                st.push(ld32(mem, mem.rebase(0xc0a00c)));
                // 0xa4e904: mov DWORD PTR [ebp-0x188],eax
                mem.write(r.ebp.wrapping_sub(0x188), r.eax);
                // 0xa4e90a: fstp DWORD PTR ds:0xc8f7d8
                mem.write(mem.rebase(0xc8f7d8), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e910: mov eax,ds:0xc8f7d8
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d8));
                // 0xa4e915: fld DWORD PTR ds:0xc0a010
                st.push(ld32(mem, mem.rebase(0xc0a010)));
                // 0xa4e91b: mov DWORD PTR [ebp-0x184],eax
                mem.write(r.ebp.wrapping_sub(0x184), r.eax);
                // 0xa4e921: fstp DWORD PTR ds:0xc8f7dc
                mem.write(mem.rebase(0xc8f7dc), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e927: mov eax,ds:0xc8f7dc
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7dc));
                // 0xa4e92c: fld1
                st.push(1.0);
                // 0xa4e92e: mov DWORD PTR [ebp-0x180],eax
                mem.write(r.ebp.wrapping_sub(0x180), r.eax);
                // 0xa4e934: fst DWORD PTR [ebp-0x184]
                mem.write(r.ebp.wrapping_sub(0x184), crate::x87::store(st0(&st)));
                // 0xa4e93a: mov DWORD PTR [edi],0x8001
                mem.write(r.edi, 0x8001u32);
                // 0xa4e940: fst DWORD PTR [ebp-0x268]
                mem.write(r.ebp.wrapping_sub(0x268), crate::x87::store(st0(&st)));
                // 0xa4e946: mov eax,ds:0x1a860ec
                r.eax = mem.read::<u32>(mem.rebase(0x1a860ec));
                // 0xa4e94b: fst DWORD PTR [ebp-0x278]
                mem.write(r.ebp.wrapping_sub(0x278), crate::x87::store(st0(&st)));
                // 0xa4e951: fstp DWORD PTR [ebp-0x1b8]
                mem.write(r.ebp.wrapping_sub(0x1b8), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e957: mov DWORD PTR [eax+0x4],0xd00a4000
                mem.write(r.eax.wrapping_add(0x4), 0xd00a4000u32);
                // 0xa4e95e: fldz
                st.push(0.0);
                // 0xa4e960: fst DWORD PTR [ebp-0x180]
                mem.write(r.ebp.wrapping_sub(0x180), crate::x87::store(st0(&st)));
                // 0xa4e966: fst DWORD PTR [ebp-0x264]
                mem.write(r.ebp.wrapping_sub(0x264), crate::x87::store(st0(&st)));
                // 0xa4e96c: fst DWORD PTR [ebp-0x274]
                mem.write(r.ebp.wrapping_sub(0x274), crate::x87::store(st0(&st)));
                // 0xa4e972: fstp DWORD PTR [ebp-0x1b4]
                mem.write(r.ebp.wrapping_sub(0x1b4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e978: fld DWORD PTR [ebp-0x1c0]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x1c0)));
                // 0xa4e97e: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4e980: fadd DWORD PTR [ebp-0x18c]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, r.ebp.wrapping_sub(0x18c)));
                }
                // 0xa4e986: fstp DWORD PTR [ebp-0x18c]
                mem.write(r.ebp.wrapping_sub(0x18c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e98c: fld DWORD PTR [ebp-0x1bc]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x1bc)));
                // 0xa4e992: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4e994: fadd DWORD PTR [ebp-0x188]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, r.ebp.wrapping_sub(0x188)));
                }
                // 0xa4e99a: fstp DWORD PTR [ebp-0x188]
                mem.write(r.ebp.wrapping_sub(0x188), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e9a0: fld DWORD PTR [ebp-0x18c]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x18c)));
                // 0xa4e9a6: fst DWORD PTR [ebp-0x280]
                mem.write(r.ebp.wrapping_sub(0x280), crate::x87::store(st0(&st)));
                // 0xa4e9ac: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4e9ae: fstp DWORD PTR [ebp-0x27c]
                mem.write(r.ebp.wrapping_sub(0x27c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e9b4: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4e9b6: fst DWORD PTR [ebp-0x270]
                mem.write(r.ebp.wrapping_sub(0x270), crate::x87::store(st0(&st)));
                // 0xa4e9bc: fld DWORD PTR [ebp-0x188]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x188)));
                // 0xa4e9c2: fstp DWORD PTR [ebp-0x26c]
                mem.write(r.ebp.wrapping_sub(0x26c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4e9c8: mov DWORD PTR [edi+0x8],0x2512512e
                mem.write(r.edi.wrapping_add(0x8), 0x2512512eu32);
                // 0xa4e9cf: mov DWORD PTR [edi+0xc],0x51251
                mem.write(r.edi.wrapping_add(0xc), 0x51251u32);
                // 0xa4e9d6: mov eax,DWORD PTR [ebx+0x10]
                r.eax = mem.read::<u32>(r.ebx.wrapping_add(0x10));
                // 0xa4e9d9: mov DWORD PTR [edi+0x10],eax
                mem.write(r.edi.wrapping_add(0x10), r.eax);
                // 0xa4e9dc: mov eax,DWORD PTR [ebx+0x14]
                r.eax = mem.read::<u32>(r.ebx.wrapping_add(0x14));
                // 0xa4e9df: mov DWORD PTR [edi+0x14],eax
                mem.write(r.edi.wrapping_add(0x14), r.eax);
                // 0xa4e9e2: mov DWORD PTR [edi+0x18],0x6
                mem.write(r.edi.wrapping_add(0x18), 0x6u32);
                // 0xa4e9e9: mov DWORD PTR [edi+0x1c],0x0
                mem.write(r.edi.wrapping_add(0x1c), 0x0u32);
                // 0xa4e9f0: mov ebx,DWORD PTR [ebx+0x10]
                r.ebx = mem.read::<u32>(r.ebx.wrapping_add(0x10));
                // 0xa4e9f3: and ebx,0x3fff
                {
                    let v = r.ebx & 0x3fffu32;
                    r.ebx = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0xa4e9f9: cmp ebx,0x3500
                r.flag = (r.ebx, 0x3500u32, false);
                r.cmp = true;
                // 0xa4e9ff: jne 0xa4ea06
                pc = if !r.zf() { 0xa4ea06 } else { 0xa4ea01 };
            }
            0xa4ea01 => {
                // 0xa4ea01: mov ebx,0x2710
                r.ebx = 0x2710u32;
                pc = 0xa4ea06;
            }
            0xa4ea06 => {
                // 0xa4ea06: mov eax,DWORD PTR [ebp-0x17c]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x17c));
                // 0xa4ea0c: fld QWORD PTR ds:0xb313a0
                st.push(mem.read::<f64>(mem.rebase(0xb313a0)));
                // 0xa4ea12: mov DWORD PTR [ebp-0x11c],eax
                mem.write(r.ebp.wrapping_sub(0x11c), r.eax);
                // 0xa4ea18: mov eax,DWORD PTR [ebp-0x178]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x178));
                // 0xa4ea1e: mov DWORD PTR [ebp-0x118],eax
                mem.write(r.ebp.wrapping_sub(0x118), r.eax);
                // 0xa4ea24: mov eax,DWORD PTR [ebp-0x174]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x174));
                // 0xa4ea2a: mov DWORD PTR [ebp-0x114],eax
                mem.write(r.ebp.wrapping_sub(0x114), r.eax);
                // 0xa4ea30: mov eax,DWORD PTR [ebp-0x170]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x170));
                // 0xa4ea36: mov DWORD PTR [ebp-0x110],eax
                mem.write(r.ebp.wrapping_sub(0x110), r.eax);
                // 0xa4ea3c: xor esi,esi
                {
                    let v = r.esi ^ r.esi;
                    r.esi = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                pc = 0xa4ea3e;
            }
            0xa4ea3e => {
                // 0xa4ea3e: fld DWORD PTR [ebp+esi*1-0x11c]
                st.push(ld32(
                    mem,
                    r.ebp
                        .wrapping_add(r.esi.wrapping_mul(1).wrapping_sub(0x11c)),
                ));
                // 0xa4ea45: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4ea47: call 0x9497e0
                {
                    let x = st0(&st);
                    st.pop();
                    r.eax = ffx_bridge::x87::ftol(&*mem, x);
                }
                // 0xa4ea4c: mov DWORD PTR [esi+0xc8f800],eax
                mem.write(mem.rebase(0xc8f800).wrapping_add(r.esi), r.eax);
                // 0xa4ea52: add esi,0x4
                {
                    let v = r.esi.wrapping_add(0x4u32);
                    r.esi = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0xa4ea55: cmp esi,0x8
                r.flag = (r.esi, 0x8u32, false);
                r.cmp = true;
                // 0xa4ea58: jl 0xa4ea3e
                pc = if r.lt() { 0xa4ea3e } else { 0xa4ea5a };
            }
            0xa4ea5a => {
                // 0xa4ea5a: mov eax,ds:0xc8f800
                r.eax = mem.read::<u32>(mem.rebase(0xc8f800));
                // 0xa4ea5f: mov DWORD PTR [edi+0x40],eax
                mem.write(r.edi.wrapping_add(0x40), r.eax);
                // 0xa4ea62: mov eax,ds:0xc8f804
                r.eax = mem.read::<u32>(mem.rebase(0xc8f804));
                // 0xa4ea67: mov DWORD PTR [edi+0x44],eax
                mem.write(r.edi.wrapping_add(0x44), r.eax);
                // 0xa4ea6a: mov eax,ds:0xc8f808
                r.eax = mem.read::<u32>(mem.rebase(0xc8f808));
                // 0xa4ea6f: mov DWORD PTR [edi+0x48],eax
                mem.write(r.edi.wrapping_add(0x48), r.eax);
                // 0xa4ea72: mov eax,DWORD PTR [ebp-0x16c]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x16c));
                // 0xa4ea78: mov DWORD PTR [ebp-0x13c],eax
                mem.write(r.ebp.wrapping_sub(0x13c), r.eax);
                // 0xa4ea7e: mov eax,DWORD PTR [ebp-0x168]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x168));
                // 0xa4ea84: mov DWORD PTR [ebp-0x138],eax
                mem.write(r.ebp.wrapping_sub(0x138), r.eax);
                // 0xa4ea8a: mov eax,DWORD PTR [ebp-0x164]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x164));
                // 0xa4ea90: mov DWORD PTR [ebp-0x134],eax
                mem.write(r.ebp.wrapping_sub(0x134), r.eax);
                // 0xa4ea96: mov eax,DWORD PTR [ebp-0x160]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x160));
                // 0xa4ea9c: mov DWORD PTR [edi+0x48],0x3000
                mem.write(r.edi.wrapping_add(0x48), 0x3000u32);
                // 0xa4eaa3: mov DWORD PTR [edi+0x4c],0x8000
                mem.write(r.edi.wrapping_add(0x4c), 0x8000u32);
                // 0xa4eaaa: mov DWORD PTR [ebp-0x130],eax
                mem.write(r.ebp.wrapping_sub(0x130), r.eax);
                // 0xa4eab0: xor esi,esi
                {
                    let v = r.esi ^ r.esi;
                    r.esi = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                pc = 0xa4eab2;
            }
            0xa4eab2 => {
                // 0xa4eab2: fld DWORD PTR [ebp+esi*1-0x13c]
                st.push(ld32(
                    mem,
                    r.ebp
                        .wrapping_add(r.esi.wrapping_mul(1).wrapping_sub(0x13c)),
                ));
                // 0xa4eab9: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4eabb: call 0x9497e0
                {
                    let x = st0(&st);
                    st.pop();
                    r.eax = ffx_bridge::x87::ftol(&*mem, x);
                }
                // 0xa4eac0: mov DWORD PTR [esi+0xc8f800],eax
                mem.write(mem.rebase(0xc8f800).wrapping_add(r.esi), r.eax);
                // 0xa4eac6: add esi,0x4
                {
                    let v = r.esi.wrapping_add(0x4u32);
                    r.esi = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0xa4eac9: cmp esi,0x8
                r.flag = (r.esi, 0x8u32, false);
                r.cmp = true;
                // 0xa4eacc: jl 0xa4eab2
                pc = if r.lt() { 0xa4eab2 } else { 0xa4eace };
            }
            0xa4eace => {
                // 0xa4eace: mov eax,ds:0xc8f800
                r.eax = mem.read::<u32>(mem.rebase(0xc8f800));
                // 0xa4ead3: mov DWORD PTR [edi+0x70],eax
                mem.write(r.edi.wrapping_add(0x70), r.eax);
                // 0xa4ead6: mov eax,ds:0xc8f804
                r.eax = mem.read::<u32>(mem.rebase(0xc8f804));
                // 0xa4eadb: mov DWORD PTR [edi+0x74],eax
                mem.write(r.edi.wrapping_add(0x74), r.eax);
                // 0xa4eade: mov eax,ds:0xc8f808
                r.eax = mem.read::<u32>(mem.rebase(0xc8f808));
                // 0xa4eae3: mov DWORD PTR [edi+0x78],eax
                mem.write(r.edi.wrapping_add(0x78), r.eax);
                // 0xa4eae6: mov eax,DWORD PTR [ebp-0x15c]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x15c));
                // 0xa4eaec: mov DWORD PTR [ebp-0x10c],eax
                mem.write(r.ebp.wrapping_sub(0x10c), r.eax);
                // 0xa4eaf2: mov eax,DWORD PTR [ebp-0x158]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x158));
                // 0xa4eaf8: mov DWORD PTR [ebp-0x108],eax
                mem.write(r.ebp.wrapping_sub(0x108), r.eax);
                // 0xa4eafe: mov eax,DWORD PTR [ebp-0x154]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x154));
                // 0xa4eb04: mov DWORD PTR [ebp-0x104],eax
                mem.write(r.ebp.wrapping_sub(0x104), r.eax);
                // 0xa4eb0a: mov eax,DWORD PTR [ebp-0x150]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x150));
                // 0xa4eb10: mov DWORD PTR [edi+0x78],0x3000
                mem.write(r.edi.wrapping_add(0x78), 0x3000u32);
                // 0xa4eb17: mov DWORD PTR [edi+0x7c],0x0
                mem.write(r.edi.wrapping_add(0x7c), 0x0u32);
                // 0xa4eb1e: mov DWORD PTR [ebp-0x100],eax
                mem.write(r.ebp.wrapping_sub(0x100), r.eax);
                // 0xa4eb24: xor esi,esi
                {
                    let v = r.esi ^ r.esi;
                    r.esi = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                pc = 0xa4eb26;
            }
            0xa4eb26 => {
                // 0xa4eb26: fld DWORD PTR [ebp+esi*1-0x10c]
                st.push(ld32(
                    mem,
                    r.ebp
                        .wrapping_add(r.esi.wrapping_mul(1).wrapping_sub(0x10c)),
                ));
                // 0xa4eb2d: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4eb2f: call 0x9497e0
                {
                    let x = st0(&st);
                    st.pop();
                    r.eax = ffx_bridge::x87::ftol(&*mem, x);
                }
                // 0xa4eb34: mov DWORD PTR [esi+0xc8f800],eax
                mem.write(mem.rebase(0xc8f800).wrapping_add(r.esi), r.eax);
                // 0xa4eb3a: add esi,0x4
                {
                    let v = r.esi.wrapping_add(0x4u32);
                    r.esi = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0xa4eb3d: cmp esi,0x8
                r.flag = (r.esi, 0x8u32, false);
                r.cmp = true;
                // 0xa4eb40: jl 0xa4eb26
                pc = if r.lt() { 0xa4eb26 } else { 0xa4eb42 };
            }
            0xa4eb42 => {
                // 0xa4eb42: mov eax,ds:0xc8f800
                r.eax = mem.read::<u32>(mem.rebase(0xc8f800));
                // 0xa4eb47: mov esi,DWORD PTR [ebp-0x19c]
                r.esi = mem.read::<u32>(r.ebp.wrapping_sub(0x19c));
                // 0xa4eb4d: mov edx,DWORD PTR [ebp-0x198]
                r.edx = mem.read::<u32>(r.ebp.wrapping_sub(0x198));
                // 0xa4eb53: mov ecx,DWORD PTR [ebp-0x194]
                r.ecx = mem.read::<u32>(r.ebp.wrapping_sub(0x194));
                // 0xa4eb59: mov DWORD PTR [edi+0xa0],eax
                mem.write(r.edi.wrapping_add(0xa0), r.eax);
                // 0xa4eb5f: mov eax,ds:0xc8f804
                r.eax = mem.read::<u32>(mem.rebase(0xc8f804));
                // 0xa4eb64: mov DWORD PTR [edi+0xa4],eax
                mem.write(r.edi.wrapping_add(0xa4), r.eax);
                // 0xa4eb6a: mov eax,ds:0xc8f808
                r.eax = mem.read::<u32>(mem.rebase(0xc8f808));
                // 0xa4eb6f: mov DWORD PTR [edi+0xa8],eax
                mem.write(r.edi.wrapping_add(0xa8), r.eax);
                // 0xa4eb75: mov eax,DWORD PTR [ebp-0x190]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x190));
                // 0xa4eb7b: mov DWORD PTR [edi+0xa8],0x3000
                mem.write(r.edi.wrapping_add(0xa8), 0x3000u32);
                // 0xa4eb85: mov DWORD PTR [edi+0xac],0x0
                mem.write(r.edi.wrapping_add(0xac), 0x0u32);
                // 0xa4eb8f: mov DWORD PTR ds:0xc8f810,esi
                mem.write(mem.rebase(0xc8f810), r.esi);
                // 0xa4eb95: mov DWORD PTR [ebp-0x12c],esi
                mem.write(r.ebp.wrapping_sub(0x12c), r.esi);
                // 0xa4eb9b: mov DWORD PTR ds:0xc8f814,edx
                mem.write(mem.rebase(0xc8f814), r.edx);
                // 0xa4eba1: mov DWORD PTR ds:0xc8f818,ecx
                mem.write(mem.rebase(0xc8f818), r.ecx);
                // 0xa4eba7: mov ds:0xc8f81c,eax
                mem.write(mem.rebase(0xc8f81c), r.eax);
                // 0xa4ebac: mov DWORD PTR [ebp-0x128],edx
                mem.write(r.ebp.wrapping_sub(0x128), r.edx);
                // 0xa4ebb2: mov DWORD PTR [ebp-0x124],ecx
                mem.write(r.ebp.wrapping_sub(0x124), r.ecx);
                // 0xa4ebb8: mov DWORD PTR [ebp-0x120],eax
                mem.write(r.ebp.wrapping_sub(0x120), r.eax);
                // 0xa4ebbe: xor esi,esi
                {
                    let v = r.esi ^ r.esi;
                    r.esi = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                pc = 0xa4ebc0;
            }
            0xa4ebc0 => {
                // 0xa4ebc0: fld DWORD PTR [ebp+esi*1-0x12c]
                st.push(ld32(
                    mem,
                    r.ebp
                        .wrapping_add(r.esi.wrapping_mul(1).wrapping_sub(0x12c)),
                ));
                // 0xa4ebc7: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4ebc9: call 0x9497e0
                {
                    let x = st0(&st);
                    st.pop();
                    r.eax = ffx_bridge::x87::ftol(&*mem, x);
                }
                // 0xa4ebce: mov DWORD PTR [esi+0xc8f800],eax
                mem.write(mem.rebase(0xc8f800).wrapping_add(r.esi), r.eax);
                // 0xa4ebd4: add esi,0x4
                {
                    let v = r.esi.wrapping_add(0x4u32);
                    r.esi = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0xa4ebd7: cmp esi,0x8
                r.flag = (r.esi, 0x8u32, false);
                r.cmp = true;
                // 0xa4ebda: jl 0xa4ebc0
                pc = if r.lt() { 0xa4ebc0 } else { 0xa4ebdc };
            }
            0xa4ebdc => {
                // 0xa4ebdc: mov eax,ds:0xc8f800
                r.eax = mem.read::<u32>(mem.rebase(0xc8f800));
                // 0xa4ebe1: fstp st(0)
                {
                    let x = st0(&st);
                    set(&mut st, 0, x);
                }
                st.pop();
                // 0xa4ebe3: mov DWORD PTR [edi+0xd0],eax
                mem.write(r.edi.wrapping_add(0xd0), r.eax);
                // 0xa4ebe9: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4ebeb: mov eax,ds:0xc8f804
                r.eax = mem.read::<u32>(mem.rebase(0xc8f804));
                // 0xa4ebf0: fld DWORD PTR [ebp-0x1c4]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x1c4)));
                // 0xa4ebf6: mov DWORD PTR [edi+0xd4],eax
                mem.write(r.edi.wrapping_add(0xd4), r.eax);
                // 0xa4ebfc: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4ebfe: mov eax,ds:0xc8f808
                r.eax = mem.read::<u32>(mem.rebase(0xc8f808));
                // 0xa4ec03: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0xa4ec05: mov DWORD PTR [edi+0xd8],eax
                mem.write(r.edi.wrapping_add(0xd8), r.eax);
                // 0xa4ec0b: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4ec0d: mov eax,DWORD PTR [ebp-0x1c0]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x1c0));
                // 0xa4ec13: mov DWORD PTR [edi+0xd8],0x3000
                mem.write(r.edi.wrapping_add(0xd8), 0x3000u32);
                // 0xa4ec1d: mov DWORD PTR [edi+0xdc],0x0
                mem.write(r.edi.wrapping_add(0xdc), 0x0u32);
                // 0xa4ec27: mov ds:0xc8f7d0,eax
                mem.write(mem.rebase(0xc8f7d0), r.eax);
                // 0xa4ec2c: fstp DWORD PTR ds:0xc8f7d0
                mem.write(mem.rebase(0xc8f7d0), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4ec32: mov eax,DWORD PTR [ebp-0x1bc]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x1bc));
                // 0xa4ec38: mov ds:0xc8f7d4,eax
                mem.write(mem.rebase(0xc8f7d4), r.eax);
                // 0xa4ec3d: fld DWORD PTR ds:0xc8f7d4
                st.push(ld32(mem, mem.rebase(0xc8f7d4)));
                // 0xa4ec43: mov eax,DWORD PTR [ebp-0x1b8]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x1b8));
                // 0xa4ec49: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4ec4b: mov ds:0xc8f7d8,eax
                mem.write(mem.rebase(0xc8f7d8), r.eax);
                // 0xa4ec50: mov eax,DWORD PTR [ebp-0x1b4]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x1b4));
                // 0xa4ec56: fstp DWORD PTR ds:0xc8f7d4
                mem.write(mem.rebase(0xc8f7d4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4ec5c: mov ds:0xc8f7dc,eax
                mem.write(mem.rebase(0xc8f7dc), r.eax);
                // 0xa4ec61: mov eax,ds:0xc8f7d0
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d0));
                // 0xa4ec66: fmul DWORD PTR ds:0xc8f7d8
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f7d8)));
                }
                // 0xa4ec6c: fstp DWORD PTR ds:0xc8f7d8
                mem.write(mem.rebase(0xc8f7d8), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4ec72: mov DWORD PTR [edi+0x20],eax
                mem.write(r.edi.wrapping_add(0x20), r.eax);
                // 0xa4ec75: mov eax,ds:0xc8f7d4
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d4));
                // 0xa4ec7a: fld st(1)
                {
                    let x = sti(&st, 1);
                    st.push(x);
                }
                // 0xa4ec7c: fld DWORD PTR [ebp-0x1cc]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x1cc)));
                // 0xa4ec82: mov DWORD PTR [edi+0x24],eax
                mem.write(r.edi.wrapping_add(0x24), r.eax);
                // 0xa4ec85: mov eax,ds:0xc8f7d8
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d8));
                // 0xa4ec8a: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4ec8c: mov DWORD PTR [edi+0x28],eax
                mem.write(r.edi.wrapping_add(0x28), r.eax);
                // 0xa4ec8f: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0xa4ec91: mov eax,ds:0xc8f7dc
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7dc));
                // 0xa4ec96: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4ec98: mov DWORD PTR [edi+0x2c],eax
                mem.write(r.edi.wrapping_add(0x2c), r.eax);
                // 0xa4ec9b: mov eax,DWORD PTR [ebp-0x280]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x280));
                // 0xa4eca1: mov ds:0xc8f7d0,eax
                mem.write(mem.rebase(0xc8f7d0), r.eax);
                // 0xa4eca6: fstp DWORD PTR ds:0xc8f7d0
                mem.write(mem.rebase(0xc8f7d0), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4ecac: mov eax,DWORD PTR [ebp-0x27c]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x27c));
                // 0xa4ecb2: mov ds:0xc8f7d4,eax
                mem.write(mem.rebase(0xc8f7d4), r.eax);
                // 0xa4ecb7: fld DWORD PTR ds:0xc8f7d4
                st.push(ld32(mem, mem.rebase(0xc8f7d4)));
                // 0xa4ecbd: mov eax,DWORD PTR [ebp-0x278]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x278));
                // 0xa4ecc3: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4ecc5: mov ds:0xc8f7d8,eax
                mem.write(mem.rebase(0xc8f7d8), r.eax);
                // 0xa4ecca: mov eax,DWORD PTR [ebp-0x274]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x274));
                // 0xa4ecd0: fstp DWORD PTR ds:0xc8f7d4
                mem.write(mem.rebase(0xc8f7d4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4ecd6: mov ds:0xc8f7dc,eax
                mem.write(mem.rebase(0xc8f7dc), r.eax);
                // 0xa4ecdb: mov eax,ds:0xc8f7d0
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d0));
                // 0xa4ece0: fmul DWORD PTR ds:0xc8f7d8
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f7d8)));
                }
                // 0xa4ece6: fstp DWORD PTR ds:0xc8f7d8
                mem.write(mem.rebase(0xc8f7d8), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4ecec: mov DWORD PTR [edi+0x50],eax
                mem.write(r.edi.wrapping_add(0x50), r.eax);
                // 0xa4ecef: mov eax,ds:0xc8f7d4
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d4));
                // 0xa4ecf4: fld DWORD PTR [ebp-0x144]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x144)));
                // 0xa4ecfa: mov DWORD PTR [edi+0x54],eax
                mem.write(r.edi.wrapping_add(0x54), r.eax);
                // 0xa4ecfd: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4ecff: mov eax,ds:0xc8f7d8
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d8));
                // 0xa4ed04: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0xa4ed06: mov DWORD PTR [edi+0x58],eax
                mem.write(r.edi.wrapping_add(0x58), r.eax);
                // 0xa4ed09: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4ed0b: mov eax,ds:0xc8f7dc
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7dc));
                // 0xa4ed10: mov DWORD PTR [edi+0x5c],eax
                mem.write(r.edi.wrapping_add(0x5c), r.eax);
                // 0xa4ed13: mov eax,DWORD PTR [ebp-0x270]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x270));
                // 0xa4ed19: mov ds:0xc8f7d0,eax
                mem.write(mem.rebase(0xc8f7d0), r.eax);
                // 0xa4ed1e: fstp DWORD PTR ds:0xc8f7d0
                mem.write(mem.rebase(0xc8f7d0), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4ed24: mov eax,DWORD PTR [ebp-0x26c]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x26c));
                // 0xa4ed2a: mov ds:0xc8f7d4,eax
                mem.write(mem.rebase(0xc8f7d4), r.eax);
                // 0xa4ed2f: fld DWORD PTR ds:0xc8f7d4
                st.push(ld32(mem, mem.rebase(0xc8f7d4)));
                // 0xa4ed35: mov eax,DWORD PTR [ebp-0x268]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x268));
                // 0xa4ed3b: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4ed3d: mov ds:0xc8f7d8,eax
                mem.write(mem.rebase(0xc8f7d8), r.eax);
                // 0xa4ed42: mov eax,DWORD PTR [ebp-0x264]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x264));
                // 0xa4ed48: fstp DWORD PTR ds:0xc8f7d4
                mem.write(mem.rebase(0xc8f7d4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4ed4e: mov ds:0xc8f7dc,eax
                mem.write(mem.rebase(0xc8f7dc), r.eax);
                // 0xa4ed53: fmul DWORD PTR ds:0xc8f7d8
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f7d8)));
                }
                // 0xa4ed59: fstp DWORD PTR ds:0xc8f7d8
                mem.write(mem.rebase(0xc8f7d8), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4ed5f: mov eax,ds:0xc8f7d0
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d0));
                // 0xa4ed64: fld DWORD PTR [ebp-0x284]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x284)));
                // 0xa4ed6a: mov DWORD PTR [edi+0x80],eax
                mem.write(r.edi.wrapping_add(0x80), r.eax);
                // 0xa4ed70: mov eax,ds:0xc8f7d4
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d4));
                // 0xa4ed75: mov DWORD PTR [edi+0x84],eax
                mem.write(r.edi.wrapping_add(0x84), r.eax);
                // 0xa4ed7b: mov eax,ds:0xc8f7d8
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d8));
                // 0xa4ed80: mov DWORD PTR [edi+0x88],eax
                mem.write(r.edi.wrapping_add(0x88), r.eax);
                // 0xa4ed86: mov eax,ds:0xc8f7dc
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7dc));
                // 0xa4ed8b: mov DWORD PTR [edi+0x8c],eax
                mem.write(r.edi.wrapping_add(0x8c), r.eax);
                // 0xa4ed91: fst DWORD PTR ds:0xc8f7e0
                mem.write(mem.rebase(0xc8f7e0), crate::x87::store(st0(&st)));
                // 0xa4ed97: mov eax,DWORD PTR [ebp-0x18c]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x18c));
                // 0xa4ed9d: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0xa4ed9f: mov ds:0xc8f7d0,eax
                mem.write(mem.rebase(0xc8f7d0), r.eax);
                // 0xa4eda4: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0xa4eda6: mov eax,DWORD PTR [ebp-0x188]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x188));
                // 0xa4edac: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4edae: mov ds:0xc8f7d4,eax
                mem.write(mem.rebase(0xc8f7d4), r.eax);
                // 0xa4edb3: fstp DWORD PTR ds:0xc8f7d0
                mem.write(mem.rebase(0xc8f7d0), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4edb9: mov eax,DWORD PTR [ebp-0x184]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x184));
                // 0xa4edbf: fld DWORD PTR ds:0xc8f7d4
                st.push(ld32(mem, mem.rebase(0xc8f7d4)));
                // 0xa4edc5: mov ds:0xc8f7d8,eax
                mem.write(mem.rebase(0xc8f7d8), r.eax);
                // 0xa4edca: mov eax,DWORD PTR [ebp-0x180]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x180));
                // 0xa4edd0: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4edd2: mov ds:0xc8f7dc,eax
                mem.write(mem.rebase(0xc8f7dc), r.eax);
                // 0xa4edd7: mov eax,ds:0xc8f7d0
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d0));
                // 0xa4eddc: fstp DWORD PTR ds:0xc8f7d4
                mem.write(mem.rebase(0xc8f7d4), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4ede2: mov esi,DWORD PTR [ebp-0x244]
                r.esi = mem.read::<u32>(r.ebp.wrapping_sub(0x244));
                // 0xa4ede8: mov edx,DWORD PTR [ebp-0x240]
                r.edx = mem.read::<u32>(r.ebp.wrapping_sub(0x240));
                // 0xa4edee: fmul DWORD PTR ds:0xc8f7d8
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, mem.rebase(0xc8f7d8)));
                }
                // 0xa4edf4: mov ecx,DWORD PTR [ebp-0x23c]
                r.ecx = mem.read::<u32>(r.ebp.wrapping_sub(0x23c));
                // 0xa4edfa: fstp DWORD PTR ds:0xc8f7d8
                mem.write(mem.rebase(0xc8f7d8), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4ee00: mov DWORD PTR [edi+0xb0],eax
                mem.write(r.edi.wrapping_add(0xb0), r.eax);
                // 0xa4ee06: mov eax,ds:0xc8f7d4
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d4));
                // 0xa4ee0b: fild DWORD PTR [edi+0x40]
                st.push(f64::from(mem.read::<i32>(r.edi.wrapping_add(0x40))));
                // 0xa4ee0e: mov DWORD PTR [edi+0xb4],eax
                mem.write(r.edi.wrapping_add(0xb4), r.eax);
                // 0xa4ee14: mov eax,ds:0xc8f7d8
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7d8));
                // 0xa4ee19: mov DWORD PTR [edi+0xb8],eax
                mem.write(r.edi.wrapping_add(0xb8), r.eax);
                // 0xa4ee1f: mov eax,ds:0xc8f7dc
                r.eax = mem.read::<u32>(mem.rebase(0xc8f7dc));
                // 0xa4ee24: mov DWORD PTR [edi+0xc0],esi
                mem.write(r.edi.wrapping_add(0xc0), r.esi);
                // 0xa4ee2a: mov DWORD PTR [edi+0x90],esi
                mem.write(r.edi.wrapping_add(0x90), r.esi);
                // 0xa4ee30: mov DWORD PTR [edi+0x60],esi
                mem.write(r.edi.wrapping_add(0x60), r.esi);
                // 0xa4ee33: mov DWORD PTR [edi+0x30],esi
                mem.write(r.edi.wrapping_add(0x30), r.esi);
                // 0xa4ee36: mov DWORD PTR [edi+0xbc],eax
                mem.write(r.edi.wrapping_add(0xbc), r.eax);
                // 0xa4ee3c: mov eax,DWORD PTR [ebp-0x238]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x238));
                // 0xa4ee42: mov DWORD PTR [edi+0xc4],edx
                mem.write(r.edi.wrapping_add(0xc4), r.edx);
                // 0xa4ee48: mov DWORD PTR [edi+0x94],edx
                mem.write(r.edi.wrapping_add(0x94), r.edx);
                // 0xa4ee4e: mov DWORD PTR [edi+0x64],edx
                mem.write(r.edi.wrapping_add(0x64), r.edx);
                // 0xa4ee51: mov DWORD PTR [edi+0x34],edx
                mem.write(r.edi.wrapping_add(0x34), r.edx);
                // 0xa4ee54: mov DWORD PTR [edi+0xc8],ecx
                mem.write(r.edi.wrapping_add(0xc8), r.ecx);
                // 0xa4ee5a: mov DWORD PTR [edi+0x98],ecx
                mem.write(r.edi.wrapping_add(0x98), r.ecx);
                // 0xa4ee60: mov DWORD PTR [edi+0x68],ecx
                mem.write(r.edi.wrapping_add(0x68), r.ecx);
                // 0xa4ee63: mov DWORD PTR [edi+0x38],ecx
                mem.write(r.edi.wrapping_add(0x38), r.ecx);
                // 0xa4ee66: mov DWORD PTR [edi+0xcc],eax
                mem.write(r.edi.wrapping_add(0xcc), r.eax);
                // 0xa4ee6c: mov DWORD PTR [edi+0x9c],eax
                mem.write(r.edi.wrapping_add(0x9c), r.eax);
                // 0xa4ee72: mov DWORD PTR [edi+0x6c],eax
                mem.write(r.edi.wrapping_add(0x6c), r.eax);
                // 0xa4ee75: mov DWORD PTR [edi+0x3c],eax
                mem.write(r.edi.wrapping_add(0x3c), r.eax);
                // 0xa4ee78: fstp DWORD PTR [ebp-0x144]
                mem.write(r.ebp.wrapping_sub(0x144), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4ee7e: fld DWORD PTR [ebp-0x144]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x144)));
                // 0xa4ee84: fld QWORD PTR ds:0xb921e0
                st.push(mem.read::<f64>(mem.rebase(0xb921e0)));
                // 0xa4ee8a: fmul st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, x * y);
                }
                // 0xa4ee8c: fld QWORD PTR ds:0xb51bc0
                st.push(mem.read::<f64>(mem.rebase(0xb51bc0)));
                // 0xa4ee92: fsub st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x - y);
                }
                // 0xa4ee94: fld QWORD PTR ds:0xb922f0
                st.push(mem.read::<f64>(mem.rebase(0xb922f0)));
                // 0xa4ee9a: fadd st(3),st
                {
                    let (x, y) = (sti(&st, 3), sti(&st, 0));
                    set(&mut st, 3, x + y);
                }
                // 0xa4ee9c: fxch st(3)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 3);
                }
                // 0xa4ee9e: fstp DWORD PTR [ebp-0x9c]
                mem.write(r.ebp.wrapping_sub(0x9c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4eea4: fild DWORD PTR [edi+0x44]
                st.push(f64::from(mem.read::<i32>(r.edi.wrapping_add(0x44))));
                // 0xa4eea7: fstp DWORD PTR [ebp-0x144]
                mem.write(r.ebp.wrapping_sub(0x144), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4eead: fld DWORD PTR [ebp-0x144]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x144)));
                // 0xa4eeb3: fmul st,st(2)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 2));
                    set(&mut st, 0, x * y);
                }
                // 0xa4eeb5: fsub st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x - y);
                }
                // 0xa4eeb7: fld QWORD PTR ds:0xb54c70
                st.push(mem.read::<f64>(mem.rebase(0xb54c70)));
                // 0xa4eebd: fadd st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, x + y);
                }
                // 0xa4eebf: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4eec1: fstp DWORD PTR [ebp-0x98]
                mem.write(r.ebp.wrapping_sub(0x98), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4eec7: fild DWORD PTR [edi+0x70]
                st.push(f64::from(mem.read::<i32>(r.edi.wrapping_add(0x70))));
                // 0xa4eeca: fstp DWORD PTR [ebp-0x144]
                mem.write(r.ebp.wrapping_sub(0x144), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4eed0: fld DWORD PTR [ebp-0x144]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x144)));
                // 0xa4eed6: fmul st,st(3)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 3));
                    set(&mut st, 0, x * y);
                }
                // 0xa4eed8: fsub st,st(2)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 2));
                    set(&mut st, 0, x - y);
                }
                // 0xa4eeda: fadd st,st(4)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 4));
                    set(&mut st, 0, x + y);
                }
                // 0xa4eedc: fstp DWORD PTR [ebp-0x7c]
                mem.write(r.ebp.wrapping_sub(0x7c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4eedf: fild DWORD PTR [edi+0x74]
                st.push(f64::from(mem.read::<i32>(r.edi.wrapping_add(0x74))));
                // 0xa4eee2: fstp DWORD PTR [ebp-0x144]
                mem.write(r.ebp.wrapping_sub(0x144), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4eee8: fld DWORD PTR [ebp-0x144]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x144)));
                // 0xa4eeee: fmul st,st(3)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 3));
                    set(&mut st, 0, x * y);
                }
                // 0xa4eef0: fsub st,st(2)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 2));
                    set(&mut st, 0, x - y);
                }
                // 0xa4eef2: fadd st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x + y);
                }
                // 0xa4eef4: fstp DWORD PTR [ebp-0x78]
                mem.write(r.ebp.wrapping_sub(0x78), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4eef7: fild DWORD PTR [edi+0xa0]
                st.push(f64::from(mem.read::<i32>(r.edi.wrapping_add(0xa0))));
                // 0xa4eefd: fstp DWORD PTR [ebp-0x144]
                mem.write(r.ebp.wrapping_sub(0x144), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4ef03: fld DWORD PTR [ebp-0x144]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x144)));
                // 0xa4ef09: fmul st,st(3)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 3));
                    set(&mut st, 0, x * y);
                }
                // 0xa4ef0b: fsub st,st(2)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 2));
                    set(&mut st, 0, x - y);
                }
                // 0xa4ef0d: fadd st,st(4)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 4));
                    set(&mut st, 0, x + y);
                }
                // 0xa4ef0f: fstp DWORD PTR [ebp-0x5c]
                mem.write(r.ebp.wrapping_sub(0x5c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4ef12: fild DWORD PTR [edi+0xa4]
                st.push(f64::from(mem.read::<i32>(r.edi.wrapping_add(0xa4))));
                // 0xa4ef18: fstp DWORD PTR [ebp-0x144]
                mem.write(r.ebp.wrapping_sub(0x144), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4ef1e: fld DWORD PTR [ebp-0x144]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x144)));
                // 0xa4ef24: fmul st,st(3)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 3));
                    set(&mut st, 0, x * y);
                }
                // 0xa4ef26: fsub st,st(2)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 2));
                    set(&mut st, 0, x - y);
                }
                // 0xa4ef28: fadd st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x + y);
                }
                // 0xa4ef2a: fstp DWORD PTR [ebp-0x58]
                mem.write(r.ebp.wrapping_sub(0x58), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4ef2d: fild DWORD PTR [edi+0xd0]
                st.push(f64::from(mem.read::<i32>(r.edi.wrapping_add(0xd0))));
                // 0xa4ef33: fstp DWORD PTR [ebp-0x144]
                mem.write(r.ebp.wrapping_sub(0x144), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4ef39: fld DWORD PTR [ebp-0x144]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x144)));
                // 0xa4ef3f: fmul st,st(3)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 3));
                    set(&mut st, 0, x * y);
                }
                // 0xa4ef41: fsub st,st(2)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 2));
                    set(&mut st, 0, x - y);
                }
                // 0xa4ef43: faddp st(4),st
                {
                    let (x, y) = (sti(&st, 4), sti(&st, 0));
                    set(&mut st, 4, x + y);
                }
                st.pop();
                // 0xa4ef45: fxch st(3)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 3);
                }
                // 0xa4ef47: fstp DWORD PTR [ebp-0x3c]
                mem.write(r.ebp.wrapping_sub(0x3c), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4ef4a: fild DWORD PTR [edi+0xd4]
                st.push(f64::from(mem.read::<i32>(r.edi.wrapping_add(0xd4))));
                // 0xa4ef50: fstp DWORD PTR [ebp-0x144]
                mem.write(r.ebp.wrapping_sub(0x144), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4ef56: fld DWORD PTR [ebp-0x144]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x144)));
                // 0xa4ef5c: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0xa4ef5e: fsubp st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, x - y);
                }
                st.pop();
                // 0xa4ef60: faddp st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, x + y);
                }
                st.pop();
                // 0xa4ef62: fstp DWORD PTR [ebp-0x38]
                mem.write(r.ebp.wrapping_sub(0x38), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4ef65: fld DWORD PTR [edi+0x20]
                st.push(ld32(mem, r.edi.wrapping_add(0x20)));
                // 0xa4ef68: fdiv DWORD PTR [edi+0x28]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x / ld32(mem, r.edi.wrapping_add(0x28)));
                }
                // 0xa4ef6b: fstp DWORD PTR [ebp-0x94]
                mem.write(r.ebp.wrapping_sub(0x94), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4ef71: fld DWORD PTR [edi+0x24]
                st.push(ld32(mem, r.edi.wrapping_add(0x24)));
                // 0xa4ef74: fdiv DWORD PTR [edi+0x28]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x / ld32(mem, r.edi.wrapping_add(0x28)));
                }
                // 0xa4ef77: fstp DWORD PTR [ebp-0x90]
                mem.write(r.ebp.wrapping_sub(0x90), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4ef7d: fld DWORD PTR [edi+0x50]
                st.push(ld32(mem, r.edi.wrapping_add(0x50)));
                // 0xa4ef80: fdiv DWORD PTR [edi+0x58]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x / ld32(mem, r.edi.wrapping_add(0x58)));
                }
                // 0xa4ef83: fstp DWORD PTR [ebp-0x74]
                mem.write(r.ebp.wrapping_sub(0x74), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4ef86: fld DWORD PTR [edi+0x54]
                st.push(ld32(mem, r.edi.wrapping_add(0x54)));
                // 0xa4ef89: fdiv DWORD PTR [edi+0x58]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x / ld32(mem, r.edi.wrapping_add(0x58)));
                }
                // 0xa4ef8c: fstp DWORD PTR [ebp-0x70]
                mem.write(r.ebp.wrapping_sub(0x70), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4ef8f: fld DWORD PTR [edi+0x80]
                st.push(ld32(mem, r.edi.wrapping_add(0x80)));
                // 0xa4ef95: fdiv DWORD PTR [edi+0x88]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x / ld32(mem, r.edi.wrapping_add(0x88)));
                }
                // 0xa4ef9b: fstp DWORD PTR [ebp-0x54]
                mem.write(r.ebp.wrapping_sub(0x54), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4ef9e: fld DWORD PTR [edi+0x84]
                st.push(ld32(mem, r.edi.wrapping_add(0x84)));
                // 0xa4efa4: fdiv DWORD PTR [edi+0x88]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x / ld32(mem, r.edi.wrapping_add(0x88)));
                }
                // 0xa4efaa: fstp DWORD PTR [ebp-0x50]
                mem.write(r.ebp.wrapping_sub(0x50), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4efad: fld DWORD PTR [edi+0xb0]
                st.push(ld32(mem, r.edi.wrapping_add(0xb0)));
                // 0xa4efb3: fdiv DWORD PTR [edi+0xb8]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x / ld32(mem, r.edi.wrapping_add(0xb8)));
                }
                // 0xa4efb9: fstp DWORD PTR [ebp-0x34]
                mem.write(r.ebp.wrapping_sub(0x34), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4efbc: fld DWORD PTR [edi+0xb4]
                st.push(ld32(mem, r.edi.wrapping_add(0xb4)));
                // 0xa4efc2: fdiv DWORD PTR [edi+0xb8]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x / ld32(mem, r.edi.wrapping_add(0xb8)));
                }
                // 0xa4efc8: fstp DWORD PTR [ebp-0x30]
                mem.write(r.ebp.wrapping_sub(0x30), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4efcb: cmp ebx,0x3000
                r.flag = (r.ebx, 0x3000u32, false);
                r.cmp = true;
                // 0xa4efd1: ja 0xa4f10e
                pc = if r.above() { 0xa4f10e } else { 0xa4efd7 };
            }
            0xa4efd7 => {
                // 0xa4efd7: je 0xa4f0a8
                pc = if r.zf() { 0xa4f0a8 } else { 0xa4efdd };
            }
            0xa4efdd => {
                // 0xa4efdd: cmp ebx,0x2710
                r.flag = (r.ebx, 0x2710u32, false);
                r.cmp = true;
                // 0xa4efe3: je 0xa4f02d
                pc = if r.zf() { 0xa4f02d } else { 0xa4efe5 };
            }
            0xa4efe5 => {
                // 0xa4efe5: cmp ebx,0x2f00
                r.flag = (r.ebx, 0x2f00u32, false);
                r.cmp = true;
                // 0xa4efeb: jne 0xa4f179
                pc = if !r.zf() { 0xa4f179 } else { 0xa4eff1 };
            }
            0xa4eff1 => {
                // 0xa4eff1: fld DWORD PTR [ebp-0x94]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x94)));
                // 0xa4eff7: fld QWORD PTR ds:0xb92220
                st.push(mem.read::<f64>(mem.rebase(0xb92220)));
                // 0xa4effd: fmul st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, x * y);
                }
                // 0xa4efff: fld QWORD PTR ds:0xb7b678
                st.push(mem.read::<f64>(mem.rebase(0xb7b678)));
                // 0xa4f005: fadd st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x + y);
                }
                // 0xa4f007: fxch st(2)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 2);
                }
                // 0xa4f009: fstp DWORD PTR [ebp-0x94]
                mem.write(r.ebp.wrapping_sub(0x94), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4f00f: fld DWORD PTR [ebp-0x74]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x74)));
                // 0xa4f012: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4f014: fadd st,st(2)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 2));
                    set(&mut st, 0, x + y);
                }
                // 0xa4f016: fstp DWORD PTR [ebp-0x74]
                mem.write(r.ebp.wrapping_sub(0x74), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4f019: fld DWORD PTR [ebp-0x54]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x54)));
                // 0xa4f01c: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4f01e: fadd st,st(2)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 2));
                    set(&mut st, 0, x + y);
                }
                // 0xa4f020: fstp DWORD PTR [ebp-0x54]
                mem.write(r.ebp.wrapping_sub(0x54), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4f023: fmul DWORD PTR [ebp-0x34]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, r.ebp.wrapping_sub(0x34)));
                }
                // 0xa4f026: faddp st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, x + y);
                }
                st.pop();
                // 0xa4f028: jmp 0xa4f176
                pc = 0xa4f176;
            }
            0xa4f02d => {
                // 0xa4f02d: fld DWORD PTR [ebp-0x94]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x94)));
                // 0xa4f033: fld QWORD PTR ds:0xb34f90
                st.push(mem.read::<f64>(mem.rebase(0xb34f90)));
                // 0xa4f039: fmul st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, x * y);
                }
                // 0xa4f03b: fld QWORD PTR ds:0xb51bd0
                st.push(mem.read::<f64>(mem.rebase(0xb51bd0)));
                // 0xa4f041: fadd st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x + y);
                }
                // 0xa4f043: fxch st(2)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 2);
                }
                // 0xa4f045: fstp DWORD PTR [ebp-0x94]
                mem.write(r.ebp.wrapping_sub(0x94), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4f04b: fld DWORD PTR [ebp-0x90]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x90)));
                // 0xa4f051: fld QWORD PTR ds:0xb92220
                st.push(mem.read::<f64>(mem.rebase(0xb92220)));
                pc = 0xa4f057;
            }
            0xa4f057 => {
                // 0xa4f057: fmul st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, x * y);
                }
                // 0xa4f059: fadd st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, x + y);
                }
                // 0xa4f05b: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4f05d: fstp DWORD PTR [ebp-0x90]
                mem.write(r.ebp.wrapping_sub(0x90), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4f063: fld DWORD PTR [ebp-0x74]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x74)));
                // 0xa4f066: fmul st,st(2)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 2));
                    set(&mut st, 0, x * y);
                }
                // 0xa4f068: fadd st,st(3)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 3));
                    set(&mut st, 0, x + y);
                }
                // 0xa4f06a: fstp DWORD PTR [ebp-0x74]
                mem.write(r.ebp.wrapping_sub(0x74), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4f06d: fld DWORD PTR [ebp-0x70]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x70)));
                // 0xa4f070: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4f072: fadd st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x + y);
                }
                // 0xa4f074: fstp DWORD PTR [ebp-0x70]
                mem.write(r.ebp.wrapping_sub(0x70), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4f077: fld DWORD PTR [ebp-0x54]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x54)));
                // 0xa4f07a: fmul st,st(2)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 2));
                    set(&mut st, 0, x * y);
                }
                // 0xa4f07c: fadd st,st(3)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 3));
                    set(&mut st, 0, x + y);
                }
                // 0xa4f07e: fstp DWORD PTR [ebp-0x54]
                mem.write(r.ebp.wrapping_sub(0x54), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4f081: fld DWORD PTR [ebp-0x50]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x50)));
                // 0xa4f084: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4f086: fadd st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x + y);
                }
                // 0xa4f088: fstp DWORD PTR [ebp-0x50]
                mem.write(r.ebp.wrapping_sub(0x50), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4f08b: fld DWORD PTR [ebp-0x34]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x34)));
                // 0xa4f08e: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0xa4f090: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4f092: faddp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x + y);
                }
                st.pop();
                // 0xa4f094: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4f096: fstp DWORD PTR [ebp-0x34]
                mem.write(r.ebp.wrapping_sub(0x34), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4f099: fld DWORD PTR [ebp-0x30]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x30)));
                // 0xa4f09c: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4f09e: faddp st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, x + y);
                }
                st.pop();
                // 0xa4f0a0: fstp DWORD PTR [ebp-0x30]
                mem.write(r.ebp.wrapping_sub(0x30), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4f0a3: jmp 0xa4f179
                pc = 0xa4f179;
            }
            0xa4f0a8 => {
                // 0xa4f0a8: fld DWORD PTR [ebp-0x94]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x94)));
                // 0xa4f0ae: fld QWORD PTR ds:0xb92220
                st.push(mem.read::<f64>(mem.rebase(0xb92220)));
                // 0xa4f0b4: fmul st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, x * y);
                }
                // 0xa4f0b6: fadd st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, x + y);
                }
                // 0xa4f0b8: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4f0ba: fstp DWORD PTR [ebp-0x94]
                mem.write(r.ebp.wrapping_sub(0x94), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4f0c0: fld DWORD PTR [ebp-0x90]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x90)));
                // 0xa4f0c6: fld QWORD PTR ds:0xb34f90
                st.push(mem.read::<f64>(mem.rebase(0xb34f90)));
                // 0xa4f0cc: fsub st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, x - y);
                }
                // 0xa4f0ce: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4f0d0: fstp DWORD PTR [ebp-0x90]
                mem.write(r.ebp.wrapping_sub(0x90), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4f0d6: fld DWORD PTR [ebp-0x74]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x74)));
                // 0xa4f0d9: fmul st,st(2)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 2));
                    set(&mut st, 0, x * y);
                }
                // 0xa4f0db: fadd st,st(2)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 2));
                    set(&mut st, 0, x + y);
                }
                // 0xa4f0dd: fstp DWORD PTR [ebp-0x74]
                mem.write(r.ebp.wrapping_sub(0x74), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4f0e0: fld DWORD PTR [ebp-0x70]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x70)));
                // 0xa4f0e3: fsub st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x - y);
                }
                // 0xa4f0e5: fstp DWORD PTR [ebp-0x70]
                mem.write(r.ebp.wrapping_sub(0x70), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4f0e8: fld DWORD PTR [ebp-0x54]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x54)));
                // 0xa4f0eb: fmul st,st(2)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 2));
                    set(&mut st, 0, x * y);
                }
                // 0xa4f0ed: fadd st,st(2)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 2));
                    set(&mut st, 0, x + y);
                }
                // 0xa4f0ef: fstp DWORD PTR [ebp-0x54]
                mem.write(r.ebp.wrapping_sub(0x54), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4f0f2: fld DWORD PTR [ebp-0x50]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x50)));
                // 0xa4f0f5: fsub st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x - y);
                }
                // 0xa4f0f7: fstp DWORD PTR [ebp-0x50]
                mem.write(r.ebp.wrapping_sub(0x50), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4f0fa: fld DWORD PTR [ebp-0x34]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x34)));
                // 0xa4f0fd: fmul st,st(2)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 2));
                    set(&mut st, 0, x * y);
                }
                // 0xa4f0ff: faddp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x + y);
                }
                st.pop();
                // 0xa4f101: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4f103: fstp DWORD PTR [ebp-0x34]
                mem.write(r.ebp.wrapping_sub(0x34), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4f106: fsubr DWORD PTR [ebp-0x30]
                {
                    let x = st0(&st);
                    set(&mut st, 0, ld32(mem, r.ebp.wrapping_sub(0x30)) - x);
                }
                // 0xa4f109: fstp DWORD PTR [ebp-0x30]
                mem.write(r.ebp.wrapping_sub(0x30), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4f10c: jmp 0xa4f179
                pc = 0xa4f179;
            }
            0xa4f10e => {
                // 0xa4f10e: cmp ebx,0x3200
                r.flag = (r.ebx, 0x3200u32, false);
                r.cmp = true;
                // 0xa4f114: je 0xa4f14d
                pc = if r.zf() { 0xa4f14d } else { 0xa4f116 };
            }
            0xa4f116 => {
                // 0xa4f116: cmp ebx,0x3600
                r.flag = (r.ebx, 0x3600u32, false);
                r.cmp = true;
                // 0xa4f11c: jne 0xa4f179
                pc = if !r.zf() { 0xa4f179 } else { 0xa4f11e };
            }
            0xa4f11e => {
                // 0xa4f11e: fld DWORD PTR [ebp-0x94]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x94)));
                // 0xa4f124: fld QWORD PTR ds:0xb92200
                st.push(mem.read::<f64>(mem.rebase(0xb92200)));
                // 0xa4f12a: fmul st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, x * y);
                }
                // 0xa4f12c: fld QWORD PTR ds:0xb43d58
                st.push(mem.read::<f64>(mem.rebase(0xb43d58)));
                // 0xa4f132: fadd st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x + y);
                }
                // 0xa4f134: fxch st(2)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 2);
                }
                // 0xa4f136: fstp DWORD PTR [ebp-0x94]
                mem.write(r.ebp.wrapping_sub(0x94), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4f13c: fld DWORD PTR [ebp-0x90]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x90)));
                // 0xa4f142: fld QWORD PTR ds:0xb34f90
                st.push(mem.read::<f64>(mem.rebase(0xb34f90)));
                // 0xa4f148: jmp 0xa4f057
                pc = 0xa4f057;
            }
            0xa4f14d => {
                // 0xa4f14d: fld DWORD PTR [ebp-0x94]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x94)));
                // 0xa4f153: fld QWORD PTR ds:0xb92220
                st.push(mem.read::<f64>(mem.rebase(0xb92220)));
                // 0xa4f159: fmul st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, x * y);
                }
                // 0xa4f15b: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0xa4f15d: fstp DWORD PTR [ebp-0x94]
                mem.write(r.ebp.wrapping_sub(0x94), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4f163: fld DWORD PTR [ebp-0x74]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x74)));
                // 0xa4f166: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4f168: fstp DWORD PTR [ebp-0x74]
                mem.write(r.ebp.wrapping_sub(0x74), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4f16b: fld DWORD PTR [ebp-0x54]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x54)));
                // 0xa4f16e: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0xa4f170: fstp DWORD PTR [ebp-0x54]
                mem.write(r.ebp.wrapping_sub(0x54), crate::x87::store(st0(&st)));
                st.pop();
                // 0xa4f173: fmul DWORD PTR [ebp-0x34]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, r.ebp.wrapping_sub(0x34)));
                }
                pc = 0xa4f176;
            }
            0xa4f176 => {
                // 0xa4f176: fstp DWORD PTR [ebp-0x34]
                mem.write(r.ebp.wrapping_sub(0x34), crate::x87::store(st0(&st)));
                st.pop();
                pc = 0xa4f179;
            }
            0xa4f179 => {
                // 0xa4f179: push 0x2
                {
                    let v = 0x2u32;
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0xa4f17b: lea eax,[ebp-0x9c]
                r.eax = r.ebp.wrapping_sub(0x9c);
                // 0xa4f181: push 0x0
                {
                    let v = 0x0u32;
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0xa4f183: push eax
                {
                    let v = r.eax;
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0xa4f184: mov DWORD PTR [ebp-0x8c],0x40
                mem.write(r.ebp.wrapping_sub(0x8c), 0x40u32);
                // 0xa4f18e: mov DWORD PTR [ebp-0x88],0x40
                mem.write(r.ebp.wrapping_sub(0x88), 0x40u32);
                // 0xa4f198: mov DWORD PTR [ebp-0x84],0x40
                mem.write(r.ebp.wrapping_sub(0x84), 0x40u32);
                // 0xa4f1a2: mov DWORD PTR [ebp-0x80],0x80
                mem.write(r.ebp.wrapping_sub(0x80), 0x80u32);
                // 0xa4f1a9: mov DWORD PTR [ebp-0x6c],0x40
                mem.write(r.ebp.wrapping_sub(0x6c), 0x40u32);
                // 0xa4f1b0: mov DWORD PTR [ebp-0x68],0x40
                mem.write(r.ebp.wrapping_sub(0x68), 0x40u32);
                // 0xa4f1b7: mov DWORD PTR [ebp-0x64],0x40
                mem.write(r.ebp.wrapping_sub(0x64), 0x40u32);
                // 0xa4f1be: mov DWORD PTR [ebp-0x60],0x80
                mem.write(r.ebp.wrapping_sub(0x60), 0x80u32);
                // 0xa4f1c5: mov DWORD PTR [ebp-0x4c],0x40
                mem.write(r.ebp.wrapping_sub(0x4c), 0x40u32);
                // 0xa4f1cc: mov DWORD PTR [ebp-0x48],0x40
                mem.write(r.ebp.wrapping_sub(0x48), 0x40u32);
                // 0xa4f1d3: mov DWORD PTR [ebp-0x44],0x40
                mem.write(r.ebp.wrapping_sub(0x44), 0x40u32);
                // 0xa4f1da: mov DWORD PTR [ebp-0x40],0x80
                mem.write(r.ebp.wrapping_sub(0x40), 0x80u32);
                // 0xa4f1e1: mov DWORD PTR [ebp-0x2c],0x40
                mem.write(r.ebp.wrapping_sub(0x2c), 0x40u32);
                // 0xa4f1e8: mov DWORD PTR [ebp-0x28],0x40
                mem.write(r.ebp.wrapping_sub(0x28), 0x40u32);
                // 0xa4f1ef: mov DWORD PTR [ebp-0x24],0x40
                mem.write(r.ebp.wrapping_sub(0x24), 0x40u32);
                // 0xa4f1f6: mov DWORD PTR [ebp-0x20],0x80
                mem.write(r.ebp.wrapping_sub(0x20), 0x80u32);
                // 0xa4f1fd: call 0x63e930
                {
                    let a: Vec<u32> = (0..3)
                        .map(|k| mem.read::<u32>(r.esp.wrapping_add(4 * k)))
                        .collect();
                    let f = mem.rebase(0x63e930);
                    r.eax = call(mem, f, &a);
                }
                // 0xa4f202: add esp,0xc
                {
                    let v = r.esp.wrapping_add(0xcu32);
                    r.esp = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0xa4f205: jmp 0xa4f215
                pc = 0xa4f215;
            }
            0xa4f207 => {
                // 0xa4f207: fstp st(0)
                {
                    let x = st0(&st);
                    set(&mut st, 0, x);
                }
                st.pop();
                pc = 0xa4f209;
            }
            0xa4f209 => {
                // 0xa4f209: fstp st(0)
                {
                    let x = st0(&st);
                    set(&mut st, 0, x);
                }
                st.pop();
                // 0xa4f20b: fstp st(2)
                {
                    let x = st0(&st);
                    set(&mut st, 2, x);
                }
                st.pop();
                // 0xa4f20d: fstp st(1)
                {
                    let x = st0(&st);
                    set(&mut st, 1, x);
                }
                st.pop();
                // 0xa4f20f: jmp 0xa4f213
                pc = 0xa4f213;
            }
            0xa4f211 => {
                // 0xa4f211: fstp st(0)
                {
                    let x = st0(&st);
                    set(&mut st, 0, x);
                }
                st.pop();
                pc = 0xa4f213;
            }
            0xa4f213 => {
                // 0xa4f213: fstp st(0)
                {
                    let x = st0(&st);
                    set(&mut st, 0, x);
                }
                st.pop();
                pc = 0xa4f215;
            }
            0xa4f215 => {
                // 0xa4f215: mov ecx,DWORD PTR [ebp-0x1a0]
                r.ecx = mem.read::<u32>(r.ebp.wrapping_sub(0x1a0));
                // 0xa4f21b: mov eax,DWORD PTR [ebp-0x254]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x254));
                // 0xa4f221: add ecx,0x10
                {
                    let v = r.ecx.wrapping_add(0x10u32);
                    r.ecx = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0xa4f224: mov DWORD PTR [ebp-0x1a0],ecx
                mem.write(r.ebp.wrapping_sub(0x1a0), r.ecx);
                // 0xa4f22a: test eax,eax
                r.flag = (r.eax & r.eax, 0, false);
                r.cmp = false;
                // 0xa4f22c: je 0xa4f239
                pc = if r.zf() { 0xa4f239 } else { 0xa4f22e };
            }
            0xa4f22e => {
                // 0xa4f22e: mov esi,DWORD PTR ds:0x2305834
                r.esi = mem.read::<u32>(mem.rebase(0x2305834));
                // 0xa4f234: jmp 0xa4ced0
                pc = 0xa4ced0;
            }
            0xa4f239 => {
                // 0xa4f239: mov ecx,DWORD PTR [ebp-0x4]
                r.ecx = mem.read::<u32>(r.ebp.wrapping_sub(0x4));
                // 0xa4f23c: pop edi
                {
                    r.edi = mem.read::<u32>(r.esp);
                    r.esp = r.esp.wrapping_add(4);
                }
                // 0xa4f23d: pop esi
                {
                    r.esi = mem.read::<u32>(r.esp);
                    r.esp = r.esp.wrapping_add(4);
                }
                // 0xa4f23e: xor ecx,ebp
                {
                    let v = r.ecx ^ r.ebp;
                    r.ecx = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0xa4f240: pop ebx
                {
                    r.ebx = mem.read::<u32>(r.esp);
                    r.esp = r.esp.wrapping_add(4);
                }
                // 0xa4f241: call 0x94923a
                // security cookie check
                // 0xa4f246: mov esp,ebp
                r.esp = r.ebp;
                // 0xa4f248: pop ebp
                {
                    r.ebp = mem.read::<u32>(r.esp);
                    r.esp = r.esp.wrapping_add(4);
                }
                // 0xa4f249: ret
                break;
            }

            _ => break,
        }
    }
    r.eax
}
