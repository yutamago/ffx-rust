//! The ability map's float-heavy draws, lifted instruction by instruction
//! from `FFX.exe` into state machines over an emulated frame (field agent
//! r3): exact, not yet rewritten by hand.

use ffx_bridge::Mem;

use crate::lift::{Regs, ld32, set, st0, sti};

/// The emulated frame's size (`work`) of each function here.
pub const WORK: u32 = 0x300;

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
