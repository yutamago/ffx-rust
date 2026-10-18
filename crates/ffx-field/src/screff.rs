//! `yiScreff`: the field's screen distortion (heat haze, under water), the
//! per-frame update of its 9 x 9 vertex grid. Lifted instruction by
//! instruction from `FFX.exe` (`0x00913200`) into a state machine over an
//! emulated frame (field agent r3): exact, not yet rewritten by hand; the
//! x87 stack is modelled at the game's 53-bit precision.

use ffx_bridge::Mem;

use crate::camera::FArgs;

use crate::lift::{Regs, fcom, fist64, ld32, set, st0, sti};

/// The emulated frame's size (`work`): the original's 0x250 bytes of
/// locals, its saved registers, the outgoing arguments and its own
/// argument.
pub const WORK: u32 = 0x300;

/// `yiScreff_draw(scene)` (`0x00913200`, name ours; `yiScreff_entry`
/// calls it): with distortion on (`+0xbe078` bit 0) the fade line from the
/// camera (`+0x67f78` inverted, the depth `+0xbe07c`), the blur level
/// `+0xbe0a0` / 128 (`graphicSetDistortionBlur`), then for each of the 9 x
/// 9 grid vertices the wave offset (`_CIfmod`/`_CIsin` of the phases
/// `+0xbe080`/`+0xbe084` and the speed `+0xbe094`, amplitude `+0xbe098`)
/// clamped to 0..1 and the colour `+0xbe0a4..` faded toward grey,
/// handed to `graphicUpdateDistortionVertex`; the frame count `+0xbe0a8`
/// and the phases (`+= +0xbe090` x 1.5 and x 1) move on. `work` is
/// [`WORK`] bytes. Returns the last `graphicUpdateDistortionVertex`'s
/// `eax`, as the original leaves it.
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
pub fn yi_screff_draw<M, C, F>(
    mem: &mut M,
    scene: u32,
    work: u32,
    call: &mut C,
    fcall: &mut F,
) -> u32
where
    M: Mem,
    C: FnMut(&mut M, u32, &[u32]) -> u32,
    F: FnMut(&mut M, u32, FArgs<'_>) -> f64,
{
    let mut r = Regs {
        ebp: work.wrapping_add(0x2a0),
        esp: work.wrapping_add(0x2a0 - 0x250),
        cw: 0x027f,
        ..Regs::default()
    };
    mem.write(r.ebp.wrapping_add(8), scene);
    let mut st: Vec<f64> = Vec::with_capacity(8);
    let mut pc: u32 = 0x0091_3209;
    loop {
        match pc {
            0x913209 => {
                // 0x913209: mov eax,ds:0xc613d8
                r.eax = mem.read::<u32>(mem.rebase(0xc613d8));
                // 0x91320e: xor eax,ebp
                {
                    let v = r.eax ^ r.ebp;
                    r.eax = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0x913210: mov DWORD PTR [ebp-0x4],eax
                mem.write(r.ebp.wrapping_sub(0x4), r.eax);
                // 0x913213: fldz
                st.push(0.0);
                // 0x913215: push ebx
                {
                    let v = r.ebx;
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0x913216: mov ebx,DWORD PTR [ebp+0x8]
                r.ebx = mem.read::<u32>(r.ebp.wrapping_add(0x8));
                // 0x913219: fst DWORD PTR [ebp-0x24c]
                mem.write(r.ebp.wrapping_sub(0x24c), crate::x87::store(st0(&st)));
                // 0x91321f: fst DWORD PTR [ebp-0x230]
                mem.write(r.ebp.wrapping_sub(0x230), crate::x87::store(st0(&st)));
                // 0x913225: push esi
                {
                    let v = r.esi;
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0x913226: test BYTE PTR [ebx+0xbe078],0x1
                r.flag = (
                    u32::from(mem.read::<u8>(r.ebx.wrapping_add(0xbe078))) & 0x1u32,
                    0,
                    false,
                );
                r.cmp = false;
                // 0x91322d: fst DWORD PTR [ebp-0x234]
                mem.write(r.ebp.wrapping_sub(0x234), crate::x87::store(st0(&st)));
                // 0x913233: fld1
                st.push(1.0);
                // 0x913235: push edi
                {
                    let v = r.edi;
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0x913236: je 0x9132b3
                pc = if r.zf() { 0x9132b3 } else { 0x913238 };
            }
            0x913238 => {
                // 0x913238: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0x91323a: lea eax,[ebp-0x64]
                r.eax = r.ebp.wrapping_sub(0x64);
                // 0x91323d: fstp DWORD PTR [ebp-0x14]
                mem.write(r.ebp.wrapping_sub(0x14), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913240: push eax
                {
                    let v = r.eax;
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0x913241: lea esi,[ebx+0x67f78]
                r.esi = r.ebx.wrapping_add(0x67f78);
                // 0x913247: fst DWORD PTR [ebp-0xc]
                mem.write(r.ebp.wrapping_sub(0xc), crate::x87::store(st0(&st)));
                // 0x91324a: mov ecx,0x10
                r.ecx = 0x10u32;
                // 0x91324f: fstp DWORD PTR [ebp-0x8]
                mem.write(r.ebp.wrapping_sub(0x8), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913252: lea edi,[ebp-0x64]
                r.edi = r.ebp.wrapping_sub(0x64);
                // 0x913255: fld DWORD PTR [ebx+0xbe07c]
                st.push(ld32(mem, r.ebx.wrapping_add(0xbe07c)));
                // 0x91325b: push eax
                {
                    let v = r.eax;
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0x91325c: fchs
                {
                    let x = st0(&st);
                    set(&mut st, 0, -x);
                }
                // 0x91325e: rep_movs
                while r.ecx != 0 {
                    let v = mem.read::<u32>(r.esi);
                    mem.write(r.edi, v);
                    r.esi = r.esi.wrapping_add(4);
                    r.edi = r.edi.wrapping_add(4);
                    r.ecx -= 1;
                }
                // 0x913260: fstp DWORD PTR [ebp-0x10]
                mem.write(r.ebp.wrapping_sub(0x10), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913263: call 0x6ed450
                {
                    let a: Vec<u32> = (0..2)
                        .map(|k| mem.read::<u32>(r.esp.wrapping_add(4 * k)))
                        .collect();
                    let f = mem.rebase(0x6ed450);
                    r.eax = call(mem, f, &a);
                }
                // 0x913268: lea eax,[ebp-0x14]
                r.eax = r.ebp.wrapping_sub(0x14);
                // 0x91326b: push eax
                {
                    let v = r.eax;
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0x91326c: lea eax,[ebp-0x64]
                r.eax = r.ebp.wrapping_sub(0x64);
                // 0x91326f: push eax
                {
                    let v = r.eax;
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0x913270: lea eax,[ebp-0x24]
                r.eax = r.ebp.wrapping_sub(0x24);
                // 0x913273: push eax
                {
                    let v = r.eax;
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0x913274: call 0x6ed2e0
                {
                    let a: Vec<u32> = (0..3)
                        .map(|k| mem.read::<u32>(r.esp.wrapping_add(4 * k)))
                        .collect();
                    let f = mem.rebase(0x6ed2e0);
                    r.eax = call(mem, f, &a);
                }
                // 0x913279: fld DWORD PTR [ebp-0x20]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x20)));
                // 0x91327c: add esp,0x14
                {
                    let v = r.esp.wrapping_add(0x14u32);
                    r.esp = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0x91327f: fld DWORD PTR [ebx+0x68360]
                st.push(ld32(mem, r.ebx.wrapping_add(0x68360)));
                // 0x913285: fld QWORD PTR ds:0xb54c70
                st.push(mem.read::<f64>(mem.rebase(0xb54c70)));
                // 0x91328b: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0x91328d: fdivrp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, y / x);
                }
                st.pop();
                // 0x91328f: fld QWORD PTR ds:0xb41308
                st.push(mem.read::<f64>(mem.rebase(0xb41308)));
                // 0x913295: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0x913297: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0x913299: fstp DWORD PTR [ebp-0x24c]
                mem.write(r.ebp.wrapping_sub(0x24c), crate::x87::store(st0(&st)));
                st.pop();
                // 0x91329f: fld DWORD PTR [ebp-0x24c]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x24c)));
                // 0x9132a5: fdivp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x / y);
                }
                st.pop();
                // 0x9132a7: fmul st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, x * y);
                }
                // 0x9132a9: fsubp st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, x - y);
                }
                st.pop();
                // 0x9132ab: fstp DWORD PTR [ebp-0x24c]
                mem.write(r.ebp.wrapping_sub(0x24c), crate::x87::store(st0(&st)));
                st.pop();
                // 0x9132b1: jmp 0x9132b7
                pc = 0x9132b7;
            }
            0x9132b3 => {
                // 0x9132b3: fstp st(0)
                {
                    let x = st0(&st);
                    set(&mut st, 0, x);
                }
                st.pop();
                // 0x9132b5: fstp st(0)
                {
                    let x = st0(&st);
                    set(&mut st, 0, x);
                }
                st.pop();
                pc = 0x9132b7;
            }
            0x9132b7 => {
                // 0x9132b7: fild DWORD PTR [ebx+0xbe0a0]
                st.push(f64::from(mem.read::<i32>(r.ebx.wrapping_add(0xbe0a0))));
                // 0x9132bd: push ecx
                {
                    let v = r.ecx;
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0x9132be: fstp DWORD PTR [ebp-0x220]
                mem.write(r.ebp.wrapping_sub(0x220), crate::x87::store(st0(&st)));
                st.pop();
                // 0x9132c4: fld DWORD PTR [ebp-0x220]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x220)));
                // 0x9132ca: fmul QWORD PTR ds:0xb170b8
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * mem.read::<f64>(mem.rebase(0xb170b8)));
                }
                // 0x9132d0: fstp DWORD PTR [ebp-0x220]
                mem.write(r.ebp.wrapping_sub(0x220), crate::x87::store(st0(&st)));
                st.pop();
                // 0x9132d6: fld DWORD PTR [ebp-0x220]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x220)));
                // 0x9132dc: fstp DWORD PTR [esp]
                mem.write(r.esp, crate::x87::store(st0(&st)));
                st.pop();
                // 0x9132df: call 0x642f30
                {
                    let a: Vec<u32> = (0..1)
                        .map(|k| mem.read::<u32>(r.esp.wrapping_add(4 * k)))
                        .collect();
                    let f = mem.rebase(0x642f30);
                    r.eax = call(mem, f, &a);
                }
                // 0x9132e4: fld DWORD PTR [ebx+0xbe094]
                st.push(ld32(mem, r.ebx.wrapping_add(0xbe094)));
                // 0x9132ea: add esp,0x4
                {
                    let v = r.esp.wrapping_add(0x4u32);
                    r.esp = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0x9132ed: fmul QWORD PTR ds:0xb41308
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * mem.read::<f64>(mem.rebase(0xb41308)));
                }
                // 0x9132f3: xor edi,edi
                {
                    let v = r.edi ^ r.edi;
                    r.edi = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0x9132f5: mov DWORD PTR [ebp-0x248],edi
                mem.write(r.ebp.wrapping_sub(0x248), r.edi);
                // 0x9132fb: fstp DWORD PTR [ebp-0x250]
                mem.write(r.ebp.wrapping_sub(0x250), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913301: fld QWORD PTR ds:0xb67128
                st.push(mem.read::<f64>(mem.rebase(0xb67128)));
                pc = 0x913307;
            }
            0x913307 => {
                // 0x913307: mov esi,edi
                r.esi = r.edi;
                // 0x913309: shl esi,0x6
                {
                    let v = r.esi << 0x6u32;
                    r.esi = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0x91330c: mov DWORD PTR [ebp-0x220],esi
                mem.write(r.ebp.wrapping_sub(0x220), r.esi);
                // 0x913312: fild DWORD PTR [ebp-0x220]
                st.push(f64::from(mem.read::<i32>(r.ebp.wrapping_sub(0x220))));
                // 0x913318: fstp DWORD PTR [ebp-0x220]
                mem.write(r.ebp.wrapping_sub(0x220), crate::x87::store(st0(&st)));
                st.pop();
                // 0x91331e: fld DWORD PTR [ebp-0x220]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x220)));
                // 0x913324: fstp DWORD PTR [ebp-0x238]
                mem.write(r.ebp.wrapping_sub(0x238), crate::x87::store(st0(&st)));
                st.pop();
                // 0x91332a: fld DWORD PTR [ebx+0xbe094]
                st.push(ld32(mem, r.ebx.wrapping_add(0xbe094)));
                // 0x913330: fmul DWORD PTR [ebp-0x238]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, r.ebp.wrapping_sub(0x238)));
                }
                // 0x913336: fadd DWORD PTR [ebx+0xbe080]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, r.ebx.wrapping_add(0xbe080)));
                }
                // 0x91333c: fstp DWORD PTR [ebp-0x220]
                mem.write(r.ebp.wrapping_sub(0x220), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913342: fld DWORD PTR [ebp-0x220]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x220)));
                // 0x913348: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0x91334a: call 0x9498e0
                {
                    let (y, x) = (st0(&st), sti(&st, 1));
                    st.pop();
                    st.pop();
                    let f = mem.rebase(0x9498e0);
                    let v = fcall(mem, f, FArgs::St01(x, y));
                    st.push(v);
                }
                // 0x91334f: fstp DWORD PTR [ebp-0x220]
                mem.write(r.ebp.wrapping_sub(0x220), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913355: fld DWORD PTR [ebp-0x220]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x220)));
                // 0x91335b: fstp DWORD PTR [ebp-0x23c]
                mem.write(r.ebp.wrapping_sub(0x23c), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913361: fld DWORD PTR [ebx+0xbe094]
                st.push(ld32(mem, r.ebx.wrapping_add(0xbe094)));
                // 0x913367: fld QWORD PTR ds:0xb2c5c8
                st.push(mem.read::<f64>(mem.rebase(0xb2c5c8)));
                // 0x91336d: fmul st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, x * y);
                }
                // 0x91336f: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0x913371: fst QWORD PTR [ebp-0x228]
                mem.write(r.ebp.wrapping_sub(0x228), st0(&st));
                // 0x913377: fld DWORD PTR [ebx+0xbe080]
                st.push(ld32(mem, r.ebx.wrapping_add(0xbe080)));
                // 0x91337d: fchs
                {
                    let x = st0(&st);
                    set(&mut st, 0, -x);
                }
                // 0x91337f: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0x913381: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0x913383: fst QWORD PTR [ebp-0x244]
                mem.write(r.ebp.wrapping_sub(0x244), st0(&st));
                // 0x913389: fld DWORD PTR [ebp-0x238]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x238)));
                // 0x91338f: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0x913391: faddp st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, x + y);
                }
                st.pop();
                // 0x913393: fstp DWORD PTR [ebp-0x220]
                mem.write(r.ebp.wrapping_sub(0x220), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913399: fld DWORD PTR [ebp-0x220]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x220)));
                // 0x91339f: fld QWORD PTR ds:0xb67128
                st.push(mem.read::<f64>(mem.rebase(0xb67128)));
                // 0x9133a5: call 0x9498e0
                {
                    let (y, x) = (st0(&st), sti(&st, 1));
                    st.pop();
                    st.pop();
                    let f = mem.rebase(0x9498e0);
                    let v = fcall(mem, f, FArgs::St01(x, y));
                    st.push(v);
                }
                // 0x9133aa: fstp DWORD PTR [ebp-0x220]
                mem.write(r.ebp.wrapping_sub(0x220), crate::x87::store(st0(&st)));
                st.pop();
                // 0x9133b0: fld DWORD PTR [ebp-0x220]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x220)));
                // 0x9133b6: fstp DWORD PTR [ebp-0x22c]
                mem.write(r.ebp.wrapping_sub(0x22c), crate::x87::store(st0(&st)));
                st.pop();
                // 0x9133bc: test edi,edi
                r.flag = (r.edi & r.edi, 0, false);
                r.cmp = false;
                // 0x9133be: jne 0x91388e
                pc = if !r.zf() { 0x91388e } else { 0x9133c4 };
            }
            0x9133c4 => {
                // 0x9133c4: fld DWORD PTR [ebp-0x23c]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x23c)));
                // 0x9133ca: mov DWORD PTR [ebp-0x218],edi
                mem.write(r.ebp.wrapping_sub(0x218), r.edi);
                // 0x9133d0: call 0x9498c8
                {
                    let x = st0(&st);
                    st.pop();
                    let f = mem.rebase(0x9498c8);
                    let v = fcall(mem, f, FArgs::St0(x));
                    st.push(v);
                }
                // 0x9133d5: fstp DWORD PTR [ebp-0x23c]
                mem.write(r.ebp.wrapping_sub(0x23c), crate::x87::store(st0(&st)));
                st.pop();
                // 0x9133db: fldz
                st.push(0.0);
                // 0x9133dd: fmul QWORD PTR ds:0xb92200
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * mem.read::<f64>(mem.rebase(0xb92200)));
                }
                // 0x9133e3: fstp DWORD PTR [ebp-0x220]
                mem.write(r.ebp.wrapping_sub(0x220), crate::x87::store(st0(&st)));
                st.pop();
                // 0x9133e9: fld DWORD PTR [ebp-0x22c]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x22c)));
                // 0x9133ef: call 0x9498c8
                {
                    let x = st0(&st);
                    st.pop();
                    let f = mem.rebase(0x9498c8);
                    let v = fcall(mem, f, FArgs::St0(x));
                    st.push(v);
                }
                // 0x9133f4: fstp DWORD PTR [ebp-0x238]
                mem.write(r.ebp.wrapping_sub(0x238), crate::x87::store(st0(&st)));
                st.pop();
                // 0x9133fa: fld DWORD PTR [ebp-0x23c]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x23c)));
                // 0x913400: mov DWORD PTR [ebp-0x240],edi
                mem.write(r.ebp.wrapping_sub(0x240), r.edi);
                // 0x913406: fld DWORD PTR [ebp-0x220]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x220)));
                // 0x91340c: lea esi,[ebp-0x208]
                r.esi = r.ebp.wrapping_sub(0x208);
                pc = 0x913412;
            }
            0x913412 => {
                // 0x913412: fld DWORD PTR [ebx+0xbe098]
                st.push(ld32(mem, r.ebx.wrapping_add(0xbe098)));
                // 0x913418: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0x91341a: fld DWORD PTR [ebp-0x230]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x230)));
                // 0x913420: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0x913422: faddp st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, x + y);
                }
                st.pop();
                // 0x913424: fstp DWORD PTR [ebp-0x21c]
                mem.write(r.ebp.wrapping_sub(0x21c), crate::x87::store(st0(&st)));
                st.pop();
                // 0x91342a: fld DWORD PTR [ebp-0x234]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x234)));
                // 0x913430: call 0x9498c8
                {
                    let x = st0(&st);
                    st.pop();
                    let f = mem.rebase(0x9498c8);
                    let v = fcall(mem, f, FArgs::St0(x));
                    st.push(v);
                }
                // 0x913435: fstp DWORD PTR [ebp-0x22c]
                mem.write(r.ebp.wrapping_sub(0x22c), crate::x87::store(st0(&st)));
                st.pop();
                // 0x91343b: fld DWORD PTR [ebp-0x22c]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x22c)));
                // 0x913441: fmul DWORD PTR [ebx+0xbe098]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, r.ebx.wrapping_add(0xbe098)));
                }
                // 0x913447: fld DWORD PTR [ebp-0x230]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x230)));
                // 0x91344d: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0x91344f: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0x913451: fild DWORD PTR [ebp-0x218]
                st.push(f64::from(mem.read::<i32>(r.ebp.wrapping_sub(0x218))));
                // 0x913457: fstp DWORD PTR [ebp-0x218]
                mem.write(r.ebp.wrapping_sub(0x218), crate::x87::store(st0(&st)));
                st.pop();
                // 0x91345d: fld DWORD PTR [ebp-0x218]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x218)));
                // 0x913463: fmul QWORD PTR ds:0xb92200
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * mem.read::<f64>(mem.rebase(0xb92200)));
                }
                // 0x913469: fstp DWORD PTR [ebp-0x218]
                mem.write(r.ebp.wrapping_sub(0x218), crate::x87::store(st0(&st)));
                st.pop();
                // 0x91346f: fld DWORD PTR [ebp-0x218]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x218)));
                // 0x913475: faddp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x + y);
                }
                st.pop();
                // 0x913477: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0x913479: fstp DWORD PTR [ebp-0x22c]
                mem.write(r.ebp.wrapping_sub(0x22c), crate::x87::store(st0(&st)));
                st.pop();
                // 0x91347f: fld DWORD PTR [ebp-0x250]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x250)));
                // 0x913485: fadd DWORD PTR [ebp-0x234]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, r.ebp.wrapping_sub(0x234)));
                }
                // 0x91348b: fstp DWORD PTR [ebp-0x234]
                mem.write(r.ebp.wrapping_sub(0x234), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913491: fld DWORD PTR [ebp-0x234]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x234)));
                // 0x913497: fld QWORD PTR ds:0xb67118
                st.push(mem.read::<f64>(mem.rebase(0xb67118)));
                // 0x91349d: fcomp st(1)
                r.sw = fcom(st0(&st), sti(&st, 1));
                st.pop();
                // 0x91349f: fnstsw ax
                r.eax = (r.eax & 0xffff_00ff) | (u32::from(r.sw) << 8);
                // 0x9134a1: test ah,0x5
                r.flag = ((r.eax >> 8) & 0x5, 0, true);
                r.cmp = false;
                // 0x9134a4: jp 0x9134b4
                pc = if r.pf() { 0x9134b4 } else { 0x9134a6 };
            }
            0x9134a6 => {
                // 0x9134a6: fsub QWORD PTR ds:0xb67128
                {
                    let x = st0(&st);
                    set(&mut st, 0, x - mem.read::<f64>(mem.rebase(0xb67128)));
                }
                // 0x9134ac: fstp DWORD PTR [ebp-0x234]
                mem.write(r.ebp.wrapping_sub(0x234), crate::x87::store(st0(&st)));
                st.pop();
                // 0x9134b2: jmp 0x9134b6
                pc = 0x9134b6;
            }
            0x9134b4 => {
                // 0x9134b4: fstp st(0)
                {
                    let x = st0(&st);
                    set(&mut st, 0, x);
                }
                st.pop();
                pc = 0x9134b6;
            }
            0x9134b6 => {
                // 0x9134b6: fld DWORD PTR [ebx+0xbe098]
                st.push(ld32(mem, r.ebx.wrapping_add(0xbe098)));
                // 0x9134bc: fmul DWORD PTR [ebp-0x238]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, r.ebp.wrapping_sub(0x238)));
                }
                // 0x9134c2: fld QWORD PTR ds:0xb2c5c8
                st.push(mem.read::<f64>(mem.rebase(0xb2c5c8)));
                // 0x9134c8: fdiv st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, x / y);
                }
                // 0x9134ca: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0x9134cc: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0x9134ce: fld DWORD PTR [ebp-0x21c]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x21c)));
                // 0x9134d4: faddp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x + y);
                }
                st.pop();
                // 0x9134d6: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0x9134d8: fstp DWORD PTR [ebp-0x21c]
                mem.write(r.ebp.wrapping_sub(0x21c), crate::x87::store(st0(&st)));
                st.pop();
                // 0x9134de: fld DWORD PTR [ebx+0xbe094]
                st.push(ld32(mem, r.ebx.wrapping_add(0xbe094)));
                // 0x9134e4: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0x9134e6: fild DWORD PTR [ebp-0x240]
                st.push(f64::from(mem.read::<i32>(r.ebp.wrapping_sub(0x240))));
                // 0x9134ec: fstp DWORD PTR [ebp-0x218]
                mem.write(r.ebp.wrapping_sub(0x218), crate::x87::store(st0(&st)));
                st.pop();
                // 0x9134f2: fmul DWORD PTR [ebp-0x218]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, r.ebp.wrapping_sub(0x218)));
                }
                // 0x9134f8: fld DWORD PTR [ebx+0xbe084]
                st.push(ld32(mem, r.ebx.wrapping_add(0xbe084)));
                // 0x9134fe: fchs
                {
                    let x = st0(&st);
                    set(&mut st, 0, -x);
                }
                // 0x913500: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0x913502: faddp st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, x + y);
                }
                st.pop();
                // 0x913504: fstp DWORD PTR [ebp-0x218]
                mem.write(r.ebp.wrapping_sub(0x218), crate::x87::store(st0(&st)));
                st.pop();
                // 0x91350a: fld DWORD PTR [ebp-0x218]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x218)));
                // 0x913510: fld QWORD PTR ds:0xb67128
                st.push(mem.read::<f64>(mem.rebase(0xb67128)));
                // 0x913516: call 0x9498e0
                {
                    let (y, x) = (st0(&st), sti(&st, 1));
                    st.pop();
                    st.pop();
                    let f = mem.rebase(0x9498e0);
                    let v = fcall(mem, f, FArgs::St01(x, y));
                    st.push(v);
                }
                // 0x91351b: fstp DWORD PTR [ebp-0x218]
                mem.write(r.ebp.wrapping_sub(0x218), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913521: fld DWORD PTR [ebp-0x218]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x218)));
                // 0x913527: call 0x9498c8
                {
                    let x = st0(&st);
                    st.pop();
                    let f = mem.rebase(0x9498c8);
                    let v = fcall(mem, f, FArgs::St0(x));
                    st.push(v);
                }
                // 0x91352c: fstp DWORD PTR [ebp-0x218]
                mem.write(r.ebp.wrapping_sub(0x218), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913532: fld DWORD PTR [ebp-0x218]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x218)));
                // 0x913538: fmul DWORD PTR [ebx+0xbe098]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, r.ebx.wrapping_add(0xbe098)));
                }
                // 0x91353e: fdiv QWORD PTR ds:0xb2c5c8
                {
                    let x = st0(&st);
                    set(&mut st, 0, x / mem.read::<f64>(mem.rebase(0xb2c5c8)));
                }
                // 0x913544: fld DWORD PTR [ebp-0x230]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x230)));
                // 0x91354a: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0x91354c: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0x91354e: fld DWORD PTR [ebp-0x22c]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x22c)));
                // 0x913554: faddp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x + y);
                }
                st.pop();
                // 0x913556: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0x913558: fstp DWORD PTR [ebp-0x22c]
                mem.write(r.ebp.wrapping_sub(0x22c), crate::x87::store(st0(&st)));
                st.pop();
                // 0x91355e: fld DWORD PTR [ebp-0x21c]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x21c)));
                // 0x913564: fldz
                st.push(0.0);
                // 0x913566: fcom st(1)
                r.sw = fcom(st0(&st), sti(&st, 1));
                // 0x913568: fnstsw ax
                r.eax = (r.eax & 0xffff_00ff) | (u32::from(r.sw) << 8);
                // 0x91356a: test ah,0x41
                r.flag = ((r.eax >> 8) & 0x41, 0, true);
                r.cmp = false;
                // 0x91356d: jne 0x91357f
                pc = if !r.zf() { 0x91357f } else { 0x91356f };
            }
            0x91356f => {
                // 0x91356f: fstp st(1)
                {
                    let x = st0(&st);
                    set(&mut st, 1, x);
                }
                st.pop();
                // 0x913571: fst DWORD PTR [ebp-0x21c]
                mem.write(r.ebp.wrapping_sub(0x21c), crate::x87::store(st0(&st)));
                // 0x913577: fld DWORD PTR [ebp-0x21c]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x21c)));
                // 0x91357d: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                pc = 0x91357f;
            }
            0x91357f => {
                // 0x91357f: fld DWORD PTR [ebp-0x22c]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x22c)));
                // 0x913585: fcom st(1)
                r.sw = fcom(st0(&st), sti(&st, 1));
                // 0x913587: fnstsw ax
                r.eax = (r.eax & 0xffff_00ff) | (u32::from(r.sw) << 8);
                // 0x913589: test ah,0x5
                r.flag = ((r.eax >> 8) & 0x5, 0, true);
                r.cmp = false;
                // 0x91358c: jp 0x91359e
                pc = if r.pf() { 0x91359e } else { 0x91358e };
            }
            0x91358e => {
                // 0x91358e: fstp st(0)
                {
                    let x = st0(&st);
                    set(&mut st, 0, x);
                }
                st.pop();
                // 0x913590: fstp DWORD PTR [ebp-0x22c]
                mem.write(r.ebp.wrapping_sub(0x22c), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913596: fld DWORD PTR [ebp-0x22c]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x22c)));
                // 0x91359c: jmp 0x9135a0
                pc = 0x9135a0;
            }
            0x91359e => {
                // 0x91359e: fstp st(1)
                {
                    let x = st0(&st);
                    set(&mut st, 1, x);
                }
                st.pop();
                pc = 0x9135a0;
            }
            0x9135a0 => {
                // 0x9135a0: fld1
                st.push(1.0);
                // 0x9135a2: fcom st(2)
                r.sw = fcom(st0(&st), sti(&st, 2));
                // 0x9135a4: fnstsw ax
                r.eax = (r.eax & 0xffff_00ff) | (u32::from(r.sw) << 8);
                // 0x9135a6: test ah,0x5
                r.flag = ((r.eax >> 8) & 0x5, 0, true);
                r.cmp = false;
                // 0x9135a9: jp 0x9135bf
                pc = if r.pf() { 0x9135bf } else { 0x9135ab };
            }
            0x9135ab => {
                // 0x9135ab: fstp st(2)
                {
                    let x = st0(&st);
                    set(&mut st, 2, x);
                }
                st.pop();
                // 0x9135ad: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0x9135af: fst DWORD PTR [ebp-0x21c]
                mem.write(r.ebp.wrapping_sub(0x21c), crate::x87::store(st0(&st)));
                // 0x9135b5: fld DWORD PTR [ebp-0x21c]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x21c)));
                // 0x9135bb: fxch st(2)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 2);
                }
                // 0x9135bd: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                pc = 0x9135bf;
            }
            0x9135bf => {
                // 0x9135bf: fcom st(1)
                r.sw = fcom(st0(&st), sti(&st, 1));
                // 0x9135c1: fnstsw ax
                r.eax = (r.eax & 0xffff_00ff) | (u32::from(r.sw) << 8);
                // 0x9135c3: test ah,0x5
                r.flag = ((r.eax >> 8) & 0x5, 0, true);
                r.cmp = false;
                // 0x9135c6: jp 0x9135d8
                pc = if r.pf() { 0x9135d8 } else { 0x9135c8 };
            }
            0x9135c8 => {
                // 0x9135c8: fstp st(1)
                {
                    let x = st0(&st);
                    set(&mut st, 1, x);
                }
                st.pop();
                // 0x9135ca: fst DWORD PTR [ebp-0x22c]
                mem.write(r.ebp.wrapping_sub(0x22c), crate::x87::store(st0(&st)));
                // 0x9135d0: fld DWORD PTR [ebp-0x22c]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x22c)));
                // 0x9135d6: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                pc = 0x9135d8;
            }
            0x9135d8 => {
                // 0x9135d8: test BYTE PTR [ebx+0xbe078],0x1
                r.flag = (
                    u32::from(mem.read::<u8>(r.ebx.wrapping_add(0xbe078))) & 0x1u32,
                    0,
                    false,
                );
                r.cmp = false;
                // 0x9135df: mov eax,DWORD PTR [ebp-0x21c]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x21c));
                // 0x9135e5: mov DWORD PTR [esi+0x84],eax
                mem.write(r.esi.wrapping_add(0x84), r.eax);
                // 0x9135eb: mov eax,DWORD PTR [ebp-0x22c]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x22c));
                // 0x9135f1: mov DWORD PTR [esi+0x88],eax
                mem.write(r.esi.wrapping_add(0x88), r.eax);
                // 0x9135f7: mov DWORD PTR [esi+0x8c],0x3f800000
                mem.write(r.esi.wrapping_add(0x8c), 0x3f800000u32);
                // 0x913601: je 0x91375e
                pc = if r.zf() { 0x91375e } else { 0x913607 };
            }
            0x913607 => {
                // 0x913607: fcomp st(3)
                r.sw = fcom(st0(&st), sti(&st, 3));
                st.pop();
                // 0x913609: fnstsw ax
                r.eax = (r.eax & 0xffff_00ff) | (u32::from(r.sw) << 8);
                // 0x91360b: test ah,0x41
                r.flag = ((r.eax >> 8) & 0x41, 0, true);
                r.cmp = false;
                // 0x91360e: jne 0x913760
                pc = if !r.zf() { 0x913760 } else { 0x913614 };
            }
            0x913614 => {
                // 0x913614: movzx eax,BYTE PTR [ebx+0xbe0a4]
                r.eax = u32::from(mem.read::<u8>(r.ebx.wrapping_add(0xbe0a4)));
                // 0x91361b: fnstcw WORD PTR [ebp-0x21a]
                mem.write(r.ebp.wrapping_sub(0x21a), r.cw);
                // 0x913621: mov DWORD PTR [ebp-0x218],eax
                mem.write(r.ebp.wrapping_sub(0x218), r.eax);
                // 0x913627: fild DWORD PTR [ebp-0x218]
                st.push(f64::from(mem.read::<i32>(r.ebp.wrapping_sub(0x218))));
                // 0x91362d: fstp QWORD PTR [ebp-0x228]
                mem.write(r.ebp.wrapping_sub(0x228), st0(&st));
                st.pop();
                // 0x913633: fld QWORD PTR [ebp-0x228]
                st.push(mem.read::<f64>(r.ebp.wrapping_sub(0x228)));
                // 0x913639: movzx eax,WORD PTR [ebp-0x21a]
                r.eax = u32::from(mem.read::<u16>(r.ebp.wrapping_sub(0x21a)));
                // 0x913640: fld QWORD PTR ds:0xb4c170
                st.push(mem.read::<f64>(mem.rebase(0xb4c170)));
                // 0x913646: or eax,0xc00
                {
                    let v = r.eax | 0xc00u32;
                    r.eax = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0x91364b: fsub st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, x - y);
                }
                // 0x91364d: mov DWORD PTR [ebp-0x218],eax
                mem.write(r.ebp.wrapping_sub(0x218), r.eax);
                // 0x913653: fld st(4)
                {
                    let x = sti(&st, 4);
                    st.push(x);
                }
                // 0x913655: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0x913657: fadd st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, x + y);
                }
                // 0x913659: fldcw WORD PTR [ebp-0x218]
                r.cw = mem.read::<u16>(r.ebp.wrapping_sub(0x218));
                // 0x91365f: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0x913661: fistp QWORD PTR [ebp-0x228]
                {
                    let v = fist64(st0(&st), r.cw);
                    mem.write(r.ebp.wrapping_sub(0x228), v);
                    st.pop();
                }
                // 0x913667: mov eax,DWORD PTR [ebp-0x228]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x228));
                // 0x91366d: mov DWORD PTR [esi-0xc],eax
                mem.write(r.esi.wrapping_sub(0xc), r.eax);
                // 0x913670: movzx eax,BYTE PTR [ebx+0xbe0a5]
                r.eax = u32::from(mem.read::<u8>(r.ebx.wrapping_add(0xbe0a5)));
                // 0x913677: fldcw WORD PTR [ebp-0x21a]
                r.cw = mem.read::<u16>(r.ebp.wrapping_sub(0x21a));
                // 0x91367d: mov DWORD PTR [ebp-0x218],eax
                mem.write(r.ebp.wrapping_sub(0x218), r.eax);
                // 0x913683: fild DWORD PTR [ebp-0x218]
                st.push(f64::from(mem.read::<i32>(r.ebp.wrapping_sub(0x218))));
                // 0x913689: fnstcw WORD PTR [ebp-0x21a]
                mem.write(r.ebp.wrapping_sub(0x21a), r.cw);
                // 0x91368f: fstp QWORD PTR [ebp-0x228]
                mem.write(r.ebp.wrapping_sub(0x228), st0(&st));
                st.pop();
                // 0x913695: fld QWORD PTR [ebp-0x228]
                st.push(mem.read::<f64>(r.ebp.wrapping_sub(0x228)));
                // 0x91369b: fsub st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x - y);
                }
                // 0x91369d: movzx eax,WORD PTR [ebp-0x21a]
                r.eax = u32::from(mem.read::<u16>(r.ebp.wrapping_sub(0x21a)));
                // 0x9136a4: or eax,0xc00
                {
                    let v = r.eax | 0xc00u32;
                    r.eax = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0x9136a9: mov DWORD PTR [ebp-0x218],eax
                mem.write(r.ebp.wrapping_sub(0x218), r.eax);
                // 0x9136af: fmul st,st(4)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 4));
                    set(&mut st, 0, x * y);
                }
                // 0x9136b1: fadd st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x + y);
                }
                // 0x9136b3: fldcw WORD PTR [ebp-0x218]
                r.cw = mem.read::<u16>(r.ebp.wrapping_sub(0x218));
                // 0x9136b9: fistp QWORD PTR [ebp-0x228]
                {
                    let v = fist64(st0(&st), r.cw);
                    mem.write(r.ebp.wrapping_sub(0x228), v);
                    st.pop();
                }
                // 0x9136bf: mov eax,DWORD PTR [ebp-0x228]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x228));
                // 0x9136c5: mov DWORD PTR [esi-0x8],eax
                mem.write(r.esi.wrapping_sub(0x8), r.eax);
                // 0x9136c8: movzx eax,BYTE PTR [ebx+0xbe0a6]
                r.eax = u32::from(mem.read::<u8>(r.ebx.wrapping_add(0xbe0a6)));
                // 0x9136cf: fldcw WORD PTR [ebp-0x21a]
                r.cw = mem.read::<u16>(r.ebp.wrapping_sub(0x21a));
                // 0x9136d5: mov DWORD PTR [ebp-0x218],eax
                mem.write(r.ebp.wrapping_sub(0x218), r.eax);
                // 0x9136db: fild DWORD PTR [ebp-0x218]
                st.push(f64::from(mem.read::<i32>(r.ebp.wrapping_sub(0x218))));
                // 0x9136e1: fnstcw WORD PTR [ebp-0x21a]
                mem.write(r.ebp.wrapping_sub(0x21a), r.cw);
                // 0x9136e7: fstp QWORD PTR [ebp-0x228]
                mem.write(r.ebp.wrapping_sub(0x228), st0(&st));
                st.pop();
                // 0x9136ed: fld QWORD PTR [ebp-0x228]
                st.push(mem.read::<f64>(r.ebp.wrapping_sub(0x228)));
                // 0x9136f3: fsub st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x - y);
                }
                // 0x9136f5: movzx eax,WORD PTR [ebp-0x21a]
                r.eax = u32::from(mem.read::<u16>(r.ebp.wrapping_sub(0x21a)));
                // 0x9136fc: or eax,0xc00
                {
                    let v = r.eax | 0xc00u32;
                    r.eax = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0x913701: mov DWORD PTR [ebp-0x218],eax
                mem.write(r.ebp.wrapping_sub(0x218), r.eax);
                // 0x913707: fmul st,st(4)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 4));
                    set(&mut st, 0, x * y);
                }
                // 0x913709: fadd st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x + y);
                }
                // 0x91370b: fldcw WORD PTR [ebp-0x218]
                r.cw = mem.read::<u16>(r.ebp.wrapping_sub(0x218));
                // 0x913711: fistp QWORD PTR [ebp-0x228]
                {
                    let v = fist64(st0(&st), r.cw);
                    mem.write(r.ebp.wrapping_sub(0x228), v);
                    st.pop();
                }
                // 0x913717: mov eax,DWORD PTR [ebp-0x228]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x228));
                // 0x91371d: mov DWORD PTR [esi-0x4],eax
                mem.write(r.esi.wrapping_sub(0x4), r.eax);
                // 0x913720: fldcw WORD PTR [ebp-0x21a]
                r.cw = mem.read::<u16>(r.ebp.wrapping_sub(0x21a));
                // 0x913726: fnstcw WORD PTR [ebp-0x21a]
                mem.write(r.ebp.wrapping_sub(0x21a), r.cw);
                // 0x91372c: fmulp st(3),st
                {
                    let (x, y) = (sti(&st, 3), sti(&st, 0));
                    set(&mut st, 3, x * y);
                }
                st.pop();
                // 0x91372e: movzx eax,WORD PTR [ebp-0x21a]
                r.eax = u32::from(mem.read::<u16>(r.ebp.wrapping_sub(0x21a)));
                // 0x913735: fxch st(2)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 2);
                }
                // 0x913737: or eax,0xc00
                {
                    let v = r.eax | 0xc00u32;
                    r.eax = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0x91373c: mov DWORD PTR [ebp-0x218],eax
                mem.write(r.ebp.wrapping_sub(0x218), r.eax);
                // 0x913742: fldcw WORD PTR [ebp-0x218]
                r.cw = mem.read::<u16>(r.ebp.wrapping_sub(0x218));
                // 0x913748: fistp QWORD PTR [ebp-0x228]
                {
                    let v = fist64(st0(&st), r.cw);
                    mem.write(r.ebp.wrapping_sub(0x228), v);
                    st.pop();
                }
                // 0x91374e: mov eax,DWORD PTR [ebp-0x228]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x228));
                // 0x913754: mov DWORD PTR [esi],eax
                mem.write(r.esi, r.eax);
                // 0x913756: fldcw WORD PTR [ebp-0x21a]
                r.cw = mem.read::<u16>(r.ebp.wrapping_sub(0x21a));
                // 0x91375c: jmp 0x913786
                pc = 0x913786;
            }
            0x91375e => {
                // 0x91375e: fstp st(0)
                {
                    let x = st0(&st);
                    set(&mut st, 0, x);
                }
                st.pop();
                pc = 0x913760;
            }
            0x913760 => {
                // 0x913760: movzx eax,BYTE PTR [ebx+0xbe0a4]
                r.eax = u32::from(mem.read::<u8>(r.ebx.wrapping_add(0xbe0a4)));
                // 0x913767: fstp st(2)
                {
                    let x = st0(&st);
                    set(&mut st, 2, x);
                }
                st.pop();
                // 0x913769: mov DWORD PTR [esi-0xc],eax
                mem.write(r.esi.wrapping_sub(0xc), r.eax);
                // 0x91376c: movzx eax,BYTE PTR [ebx+0xbe0a5]
                r.eax = u32::from(mem.read::<u8>(r.ebx.wrapping_add(0xbe0a5)));
                // 0x913773: mov DWORD PTR [esi-0x8],eax
                mem.write(r.esi.wrapping_sub(0x8), r.eax);
                // 0x913776: movzx eax,BYTE PTR [ebx+0xbe0a6]
                r.eax = u32::from(mem.read::<u8>(r.ebx.wrapping_add(0xbe0a6)));
                // 0x91377d: mov DWORD PTR [esi-0x4],eax
                mem.write(r.esi.wrapping_sub(0x4), r.eax);
                // 0x913780: mov DWORD PTR [esi],0x80
                mem.write(r.esi, 0x80u32);
                pc = 0x913786;
            }
            0x913786 => {
                // 0x913786: mov eax,DWORD PTR [esi]
                r.eax = mem.read::<u32>(r.esi);
                // 0x913788: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0x91378a: sub esp,0x18
                {
                    let v = r.esp.wrapping_sub(0x18u32);
                    r.esp = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0x91378d: fstp DWORD PTR [esp+0x14]
                mem.write(r.esp.wrapping_add(0x14), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913791: fstp DWORD PTR [esp+0x10]
                mem.write(r.esp.wrapping_add(0x10), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913795: fild DWORD PTR [esi]
                st.push(f64::from(mem.read::<i32>(r.esi)));
                // 0x913797: test eax,eax
                r.flag = (r.eax & r.eax, 0, false);
                r.cmp = false;
                // 0x913799: jns 0x9137a1
                pc = if !r.sf() { 0x9137a1 } else { 0x91379b };
            }
            0x91379b => {
                // 0x91379b: fadd DWORD PTR ds:0xb11f14
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xb11f14)));
                }
                pc = 0x9137a1;
            }
            0x9137a1 => {
                // 0x9137a1: fstp DWORD PTR [ebp-0x218]
                mem.write(r.ebp.wrapping_sub(0x218), crate::x87::store(st0(&st)));
                st.pop();
                // 0x9137a7: fld DWORD PTR [ebp-0x218]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x218)));
                // 0x9137ad: mov eax,DWORD PTR [esi-0x4]
                r.eax = mem.read::<u32>(r.esi.wrapping_sub(0x4));
                // 0x9137b0: fld QWORD PTR ds:0xb170b8
                st.push(mem.read::<f64>(mem.rebase(0xb170b8)));
                // 0x9137b6: fmul st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, x * y);
                }
                // 0x9137b8: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0x9137ba: fstp DWORD PTR [ebp-0x218]
                mem.write(r.ebp.wrapping_sub(0x218), crate::x87::store(st0(&st)));
                st.pop();
                // 0x9137c0: fld DWORD PTR [ebp-0x218]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x218)));
                // 0x9137c6: fstp DWORD PTR [esp+0xc]
                mem.write(r.esp.wrapping_add(0xc), crate::x87::store(st0(&st)));
                st.pop();
                // 0x9137ca: fild DWORD PTR [esi-0x4]
                st.push(f64::from(mem.read::<i32>(r.esi.wrapping_sub(0x4))));
                // 0x9137cd: test eax,eax
                r.flag = (r.eax & r.eax, 0, false);
                r.cmp = false;
                // 0x9137cf: jns 0x9137d7
                pc = if !r.sf() { 0x9137d7 } else { 0x9137d1 };
            }
            0x9137d1 => {
                // 0x9137d1: fadd DWORD PTR ds:0xb11f14
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xb11f14)));
                }
                pc = 0x9137d7;
            }
            0x9137d7 => {
                // 0x9137d7: fstp DWORD PTR [ebp-0x218]
                mem.write(r.ebp.wrapping_sub(0x218), crate::x87::store(st0(&st)));
                st.pop();
                // 0x9137dd: fld DWORD PTR [ebp-0x218]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x218)));
                // 0x9137e3: mov eax,DWORD PTR [esi-0x8]
                r.eax = mem.read::<u32>(r.esi.wrapping_sub(0x8));
                // 0x9137e6: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0x9137e8: fstp DWORD PTR [ebp-0x218]
                mem.write(r.ebp.wrapping_sub(0x218), crate::x87::store(st0(&st)));
                st.pop();
                // 0x9137ee: fld DWORD PTR [ebp-0x218]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x218)));
                // 0x9137f4: fstp DWORD PTR [esp+0x8]
                mem.write(r.esp.wrapping_add(0x8), crate::x87::store(st0(&st)));
                st.pop();
                // 0x9137f8: fild DWORD PTR [esi-0x8]
                st.push(f64::from(mem.read::<i32>(r.esi.wrapping_sub(0x8))));
                // 0x9137fb: test eax,eax
                r.flag = (r.eax & r.eax, 0, false);
                r.cmp = false;
                // 0x9137fd: jns 0x913805
                pc = if !r.sf() { 0x913805 } else { 0x9137ff };
            }
            0x9137ff => {
                // 0x9137ff: fadd DWORD PTR ds:0xb11f14
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xb11f14)));
                }
                pc = 0x913805;
            }
            0x913805 => {
                // 0x913805: fstp DWORD PTR [ebp-0x218]
                mem.write(r.ebp.wrapping_sub(0x218), crate::x87::store(st0(&st)));
                st.pop();
                // 0x91380b: fld DWORD PTR [ebp-0x218]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x218)));
                // 0x913811: mov eax,DWORD PTR [esi-0xc]
                r.eax = mem.read::<u32>(r.esi.wrapping_sub(0xc));
                // 0x913814: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0x913816: fstp DWORD PTR [ebp-0x218]
                mem.write(r.ebp.wrapping_sub(0x218), crate::x87::store(st0(&st)));
                st.pop();
                // 0x91381c: fld DWORD PTR [ebp-0x218]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x218)));
                // 0x913822: fstp DWORD PTR [esp+0x4]
                mem.write(r.esp.wrapping_add(0x4), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913826: fild DWORD PTR [esi-0xc]
                st.push(f64::from(mem.read::<i32>(r.esi.wrapping_sub(0xc))));
                // 0x913829: test eax,eax
                r.flag = (r.eax & r.eax, 0, false);
                r.cmp = false;
                // 0x91382b: jns 0x913833
                pc = if !r.sf() { 0x913833 } else { 0x91382d };
            }
            0x91382d => {
                // 0x91382d: fadd DWORD PTR ds:0xb11f14
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xb11f14)));
                }
                pc = 0x913833;
            }
            0x913833 => {
                // 0x913833: fstp DWORD PTR [ebp-0x218]
                mem.write(r.ebp.wrapping_sub(0x218), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913839: fmul DWORD PTR [ebp-0x218]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, r.ebp.wrapping_sub(0x218)));
                }
                // 0x91383f: fstp DWORD PTR [ebp-0x218]
                mem.write(r.ebp.wrapping_sub(0x218), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913845: fld DWORD PTR [ebp-0x218]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x218)));
                // 0x91384b: fstp DWORD PTR [esp]
                mem.write(r.esp, crate::x87::store(st0(&st)));
                st.pop();
                // 0x91384e: push edi
                {
                    let v = r.edi;
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0x91384f: push 0x0
                {
                    let v = 0x0u32;
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0x913851: call 0x644940
                {
                    let a: Vec<u32> = (0..8)
                        .map(|k| mem.read::<u32>(r.esp.wrapping_add(4 * k)))
                        .collect();
                    let f = mem.rebase(0x644940);
                    r.eax = call(mem, f, &a);
                }
                // 0x913856: fld DWORD PTR [ebp-0x23c]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x23c)));
                // 0x91385c: mov eax,DWORD PTR [ebp-0x240]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x240));
                // 0x913862: fld DWORD PTR [ebp-0x220]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x220)));
                // 0x913868: add eax,0x34
                {
                    let v = r.eax.wrapping_add(0x34u32);
                    r.eax = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0x91386b: inc edi
                {
                    let v = r.edi.wrapping_add(1);
                    r.edi = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0x91386c: add esp,0x20
                {
                    let v = r.esp.wrapping_add(0x20u32);
                    r.esp = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0x91386f: add esi,0x10
                {
                    let v = r.esi.wrapping_add(0x10u32);
                    r.esi = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0x913872: mov DWORD PTR [ebp-0x218],edi
                mem.write(r.ebp.wrapping_sub(0x218), r.edi);
                // 0x913878: mov DWORD PTR [ebp-0x240],eax
                mem.write(r.ebp.wrapping_sub(0x240), r.eax);
                // 0x91387e: cmp eax,0x1d4
                r.flag = (r.eax, 0x1d4u32, false);
                r.cmp = true;
                // 0x913883: jl 0x913412
                pc = if r.lt() { 0x913412 } else { 0x913889 };
            }
            0x913889 => {
                // 0x913889: jmp 0x913ec9
                pc = 0x913ec9;
            }
            0x91388e => {
                // 0x91388e: lea eax,[esi+0x40]
                r.eax = r.esi.wrapping_add(0x40);
                // 0x913891: mov DWORD PTR [ebp-0x220],eax
                mem.write(r.ebp.wrapping_sub(0x220), r.eax);
                // 0x913897: fild DWORD PTR [ebp-0x220]
                st.push(f64::from(mem.read::<i32>(r.ebp.wrapping_sub(0x220))));
                // 0x91389d: fstp DWORD PTR [ebp-0x220]
                mem.write(r.ebp.wrapping_sub(0x220), crate::x87::store(st0(&st)));
                st.pop();
                // 0x9138a3: fld DWORD PTR [ebp-0x220]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x220)));
                // 0x9138a9: fstp DWORD PTR [ebp-0x220]
                mem.write(r.ebp.wrapping_sub(0x220), crate::x87::store(st0(&st)));
                st.pop();
                // 0x9138af: fld DWORD PTR [ebx+0xbe094]
                st.push(ld32(mem, r.ebx.wrapping_add(0xbe094)));
                // 0x9138b5: fmul DWORD PTR [ebp-0x220]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, r.ebp.wrapping_sub(0x220)));
                }
                // 0x9138bb: fadd DWORD PTR [ebx+0xbe080]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, r.ebx.wrapping_add(0xbe080)));
                }
                // 0x9138c1: fstp DWORD PTR [ebp-0x238]
                mem.write(r.ebp.wrapping_sub(0x238), crate::x87::store(st0(&st)));
                st.pop();
                // 0x9138c7: fld DWORD PTR [ebp-0x238]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x238)));
                // 0x9138cd: fld QWORD PTR ds:0xb67128
                st.push(mem.read::<f64>(mem.rebase(0xb67128)));
                // 0x9138d3: call 0x9498e0
                {
                    let (y, x) = (st0(&st), sti(&st, 1));
                    st.pop();
                    st.pop();
                    let f = mem.rebase(0x9498e0);
                    let v = fcall(mem, f, FArgs::St01(x, y));
                    st.push(v);
                }
                // 0x9138d8: fstp DWORD PTR [ebp-0x238]
                mem.write(r.ebp.wrapping_sub(0x238), crate::x87::store(st0(&st)));
                st.pop();
                // 0x9138de: fld DWORD PTR [ebp-0x238]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x238)));
                // 0x9138e4: fstp DWORD PTR [ebp-0x23c]
                mem.write(r.ebp.wrapping_sub(0x23c), crate::x87::store(st0(&st)));
                st.pop();
                // 0x9138ea: fld DWORD PTR [ebp-0x220]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x220)));
                // 0x9138f0: fmul QWORD PTR [ebp-0x228]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * mem.read::<f64>(r.ebp.wrapping_sub(0x228)));
                }
                // 0x9138f6: fadd QWORD PTR [ebp-0x244]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + mem.read::<f64>(r.ebp.wrapping_sub(0x244)));
                }
                // 0x9138fc: fstp DWORD PTR [ebp-0x220]
                mem.write(r.ebp.wrapping_sub(0x220), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913902: fld DWORD PTR [ebp-0x220]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x220)));
                // 0x913908: fld QWORD PTR ds:0xb67128
                st.push(mem.read::<f64>(mem.rebase(0xb67128)));
                // 0x91390e: call 0x9498e0
                {
                    let (y, x) = (st0(&st), sti(&st, 1));
                    st.pop();
                    st.pop();
                    let f = mem.rebase(0x9498e0);
                    let v = fcall(mem, f, FArgs::St01(x, y));
                    st.push(v);
                }
                // 0x913913: fstp DWORD PTR [ebp-0x220]
                mem.write(r.ebp.wrapping_sub(0x220), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913919: fld DWORD PTR [ebp-0x220]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x220)));
                // 0x91391f: fstp DWORD PTR [ebp-0x22c]
                mem.write(r.ebp.wrapping_sub(0x22c), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913925: fld DWORD PTR [ebx+0xbe084]
                st.push(ld32(mem, r.ebx.wrapping_add(0xbe084)));
                // 0x91392b: fld QWORD PTR ds:0xb67128
                st.push(mem.read::<f64>(mem.rebase(0xb67128)));
                // 0x913931: call 0x9498e0
                {
                    let (y, x) = (st0(&st), sti(&st, 1));
                    st.pop();
                    st.pop();
                    let f = mem.rebase(0x9498e0);
                    let v = fcall(mem, f, FArgs::St01(x, y));
                    st.push(v);
                }
                // 0x913936: fstp DWORD PTR [ebp-0x220]
                mem.write(r.ebp.wrapping_sub(0x220), crate::x87::store(st0(&st)));
                st.pop();
                // 0x91393c: fld DWORD PTR [ebp-0x220]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x220)));
                // 0x913942: fstp DWORD PTR [ebp-0x234]
                mem.write(r.ebp.wrapping_sub(0x234), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913948: fld DWORD PTR [ebx+0xbe084]
                st.push(ld32(mem, r.ebx.wrapping_add(0xbe084)));
                // 0x91394e: fchs
                {
                    let x = st0(&st);
                    set(&mut st, 0, -x);
                }
                // 0x913950: fmul QWORD PTR ds:0xb2c5c8
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * mem.read::<f64>(mem.rebase(0xb2c5c8)));
                }
                // 0x913956: fstp DWORD PTR [ebp-0x220]
                mem.write(r.ebp.wrapping_sub(0x220), crate::x87::store(st0(&st)));
                st.pop();
                // 0x91395c: fld DWORD PTR [ebp-0x220]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x220)));
                // 0x913962: fld QWORD PTR ds:0xb67128
                st.push(mem.read::<f64>(mem.rebase(0xb67128)));
                // 0x913968: call 0x9498e0
                {
                    let (y, x) = (st0(&st), sti(&st, 1));
                    st.pop();
                    st.pop();
                    let f = mem.rebase(0x9498e0);
                    let v = fcall(mem, f, FArgs::St01(x, y));
                    st.push(v);
                }
                // 0x91396d: fstp DWORD PTR [ebp-0x220]
                mem.write(r.ebp.wrapping_sub(0x220), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913973: fld DWORD PTR [ebp-0x220]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x220)));
                // 0x913979: mov DWORD PTR [ebp-0x218],0x0
                mem.write(r.ebp.wrapping_sub(0x218), 0x0u32);
                // 0x913983: fstp DWORD PTR [ebp-0x240]
                mem.write(r.ebp.wrapping_sub(0x240), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913989: fld DWORD PTR [ebp-0x23c]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x23c)));
                // 0x91398f: call 0x9498c8
                {
                    let x = st0(&st);
                    st.pop();
                    let f = mem.rebase(0x9498c8);
                    let v = fcall(mem, f, FArgs::St0(x));
                    st.push(v);
                }
                // 0x913994: fstp DWORD PTR [ebp-0x23c]
                mem.write(r.ebp.wrapping_sub(0x23c), crate::x87::store(st0(&st)));
                st.pop();
                // 0x91399a: fild DWORD PTR [ebp-0x248]
                st.push(f64::from(mem.read::<i32>(r.ebp.wrapping_sub(0x248))));
                // 0x9139a0: fstp DWORD PTR [ebp-0x220]
                mem.write(r.ebp.wrapping_sub(0x220), crate::x87::store(st0(&st)));
                st.pop();
                // 0x9139a6: fld DWORD PTR [ebp-0x220]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x220)));
                // 0x9139ac: fmul QWORD PTR ds:0xb92200
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * mem.read::<f64>(mem.rebase(0xb92200)));
                }
                // 0x9139b2: fstp DWORD PTR [ebp-0x220]
                mem.write(r.ebp.wrapping_sub(0x220), crate::x87::store(st0(&st)));
                st.pop();
                // 0x9139b8: fld DWORD PTR [ebp-0x22c]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x22c)));
                // 0x9139be: call 0x9498c8
                {
                    let x = st0(&st);
                    st.pop();
                    let f = mem.rebase(0x9498c8);
                    let v = fcall(mem, f, FArgs::St0(x));
                    st.push(v);
                }
                // 0x9139c3: fstp DWORD PTR [ebp-0x238]
                mem.write(r.ebp.wrapping_sub(0x238), crate::x87::store(st0(&st)));
                st.pop();
                // 0x9139c9: fld DWORD PTR [ebp-0x23c]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x23c)));
                // 0x9139cf: lea esi,[ebp-0x208]
                r.esi = r.ebp.wrapping_sub(0x208);
                // 0x9139d5: fld DWORD PTR [ebp-0x220]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x220)));
                pc = 0x9139db;
            }
            0x9139db => {
                // 0x9139db: mov eax,DWORD PTR [ebx+0xbe078]
                r.eax = mem.read::<u32>(r.ebx.wrapping_add(0xbe078));
                // 0x9139e1: fld DWORD PTR [ebp-0x234]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x234)));
                // 0x9139e7: mov edi,eax
                r.edi = r.eax;
                // 0x9139e9: and edi,0x1
                {
                    let v = r.edi & 0x1u32;
                    r.edi = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0x9139ec: je 0x913a6a
                pc = if r.zf() { 0x913a6a } else { 0x9139ee };
            }
            0x9139ee => {
                // 0x9139ee: test al,0x2
                r.flag = ((r.eax & 0xff) & 0x2u32, 0, false);
                r.cmp = false;
                // 0x9139f0: jne 0x913a2c
                pc = if !r.zf() { 0x913a2c } else { 0x9139f2 };
            }
            0x9139f2 => {
                // 0x9139f2: fild DWORD PTR [ebp-0x218]
                st.push(f64::from(mem.read::<i32>(r.ebp.wrapping_sub(0x218))));
                // 0x9139f8: fstp DWORD PTR [ebp-0x230]
                mem.write(r.ebp.wrapping_sub(0x230), crate::x87::store(st0(&st)));
                st.pop();
                // 0x9139fe: fld DWORD PTR [ebp-0x230]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x230)));
                // 0x913a04: fmul QWORD PTR ds:0xb5f1e8
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * mem.read::<f64>(mem.rebase(0xb5f1e8)));
                }
                // 0x913a0a: fstp DWORD PTR [ebp-0x230]
                mem.write(r.ebp.wrapping_sub(0x230), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913a10: fld DWORD PTR [ebp-0x230]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x230)));
                // 0x913a16: fld DWORD PTR [ebp-0x24c]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x24c)));
                // 0x913a1c: fchs
                {
                    let x = st0(&st);
                    set(&mut st, 0, -x);
                }
                // 0x913a1e: fsubp st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, x - y);
                }
                st.pop();
                // 0x913a20: fmul QWORD PTR ds:0xb170b8
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * mem.read::<f64>(mem.rebase(0xb170b8)));
                }
                // 0x913a26: fstp DWORD PTR [ebp-0x230]
                mem.write(r.ebp.wrapping_sub(0x230), crate::x87::store(st0(&st)));
                st.pop();
                pc = 0x913a2c;
            }
            0x913a2c => {
                // 0x913a2c: fld DWORD PTR [ebp-0x230]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x230)));
                // 0x913a32: fldz
                st.push(0.0);
                // 0x913a34: fcom st(1)
                r.sw = fcom(st0(&st), sti(&st, 1));
                // 0x913a36: fnstsw ax
                r.eax = (r.eax & 0xffff_00ff) | (u32::from(r.sw) << 8);
                // 0x913a38: test ah,0x41
                r.flag = ((r.eax >> 8) & 0x41, 0, true);
                r.cmp = false;
                // 0x913a3b: jne 0x913a4d
                pc = if !r.zf() { 0x913a4d } else { 0x913a3d };
            }
            0x913a3d => {
                // 0x913a3d: fstp st(1)
                {
                    let x = st0(&st);
                    set(&mut st, 1, x);
                }
                st.pop();
                // 0x913a3f: fstp DWORD PTR [ebp-0x230]
                mem.write(r.ebp.wrapping_sub(0x230), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913a45: fld DWORD PTR [ebp-0x230]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x230)));
                // 0x913a4b: jmp 0x913a7c
                pc = 0x913a7c;
            }
            0x913a4d => {
                // 0x913a4d: fstp st(0)
                {
                    let x = st0(&st);
                    set(&mut st, 0, x);
                }
                st.pop();
                // 0x913a4f: fld1
                st.push(1.0);
                // 0x913a51: fcom st(1)
                r.sw = fcom(st0(&st), sti(&st, 1));
                // 0x913a53: fnstsw ax
                r.eax = (r.eax & 0xffff_00ff) | (u32::from(r.sw) << 8);
                // 0x913a55: test ah,0x5
                r.flag = ((r.eax >> 8) & 0x5, 0, true);
                r.cmp = false;
                // 0x913a58: jp 0x913a7a
                pc = if r.pf() { 0x913a7a } else { 0x913a5a };
            }
            0x913a5a => {
                // 0x913a5a: fstp st(1)
                {
                    let x = st0(&st);
                    set(&mut st, 1, x);
                }
                st.pop();
                // 0x913a5c: fstp DWORD PTR [ebp-0x230]
                mem.write(r.ebp.wrapping_sub(0x230), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913a62: fld DWORD PTR [ebp-0x230]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x230)));
                // 0x913a68: jmp 0x913a7c
                pc = 0x913a7c;
            }
            0x913a6a => {
                // 0x913a6a: fld1
                st.push(1.0);
                // 0x913a6c: fstp DWORD PTR [ebp-0x230]
                mem.write(r.ebp.wrapping_sub(0x230), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913a72: fld DWORD PTR [ebp-0x230]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x230)));
                // 0x913a78: jmp 0x913a7c
                pc = 0x913a7c;
            }
            0x913a7a => {
                // 0x913a7a: fstp st(0)
                {
                    let x = st0(&st);
                    set(&mut st, 0, x);
                }
                st.pop();
                pc = 0x913a7c;
            }
            0x913a7c => {
                // 0x913a7c: fld DWORD PTR [ebx+0xbe098]
                st.push(ld32(mem, r.ebx.wrapping_add(0xbe098)));
                // 0x913a82: fmulp st(4),st
                {
                    let (x, y) = (sti(&st, 4), sti(&st, 0));
                    set(&mut st, 4, x * y);
                }
                st.pop();
                // 0x913a84: fmulp st(3),st
                {
                    let (x, y) = (sti(&st, 3), sti(&st, 0));
                    set(&mut st, 3, x * y);
                }
                st.pop();
                // 0x913a86: fxch st(2)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 2);
                }
                // 0x913a88: faddp st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, x + y);
                }
                st.pop();
                // 0x913a8a: fstp DWORD PTR [ebp-0x21c]
                mem.write(r.ebp.wrapping_sub(0x21c), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913a90: call 0x9498c8
                {
                    let x = st0(&st);
                    st.pop();
                    let f = mem.rebase(0x9498c8);
                    let v = fcall(mem, f, FArgs::St0(x));
                    st.push(v);
                }
                // 0x913a95: fstp DWORD PTR [ebp-0x22c]
                mem.write(r.ebp.wrapping_sub(0x22c), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913a9b: fld DWORD PTR [ebp-0x22c]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x22c)));
                // 0x913aa1: fmul DWORD PTR [ebx+0xbe098]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, r.ebx.wrapping_add(0xbe098)));
                }
                // 0x913aa7: fld DWORD PTR [ebp-0x230]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x230)));
                // 0x913aad: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0x913aaf: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0x913ab1: fild DWORD PTR [ebp-0x218]
                st.push(f64::from(mem.read::<i32>(r.ebp.wrapping_sub(0x218))));
                // 0x913ab7: fstp DWORD PTR [ebp-0x22c]
                mem.write(r.ebp.wrapping_sub(0x22c), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913abd: fld DWORD PTR [ebp-0x22c]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x22c)));
                // 0x913ac3: fmul QWORD PTR ds:0xb92200
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * mem.read::<f64>(mem.rebase(0xb92200)));
                }
                // 0x913ac9: fstp DWORD PTR [ebp-0x22c]
                mem.write(r.ebp.wrapping_sub(0x22c), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913acf: fld DWORD PTR [ebp-0x22c]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x22c)));
                // 0x913ad5: faddp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x + y);
                }
                st.pop();
                // 0x913ad7: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0x913ad9: fstp DWORD PTR [ebp-0x22c]
                mem.write(r.ebp.wrapping_sub(0x22c), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913adf: fld DWORD PTR [ebp-0x250]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x250)));
                // 0x913ae5: fadd DWORD PTR [ebp-0x234]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, r.ebp.wrapping_sub(0x234)));
                }
                // 0x913aeb: fstp DWORD PTR [ebp-0x234]
                mem.write(r.ebp.wrapping_sub(0x234), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913af1: fld DWORD PTR [ebp-0x234]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x234)));
                // 0x913af7: fld QWORD PTR ds:0xb67118
                st.push(mem.read::<f64>(mem.rebase(0xb67118)));
                // 0x913afd: fcomp st(1)
                r.sw = fcom(st0(&st), sti(&st, 1));
                st.pop();
                // 0x913aff: fnstsw ax
                r.eax = (r.eax & 0xffff_00ff) | (u32::from(r.sw) << 8);
                // 0x913b01: test ah,0x5
                r.flag = ((r.eax >> 8) & 0x5, 0, true);
                r.cmp = false;
                // 0x913b04: jp 0x913b14
                pc = if r.pf() { 0x913b14 } else { 0x913b06 };
            }
            0x913b06 => {
                // 0x913b06: fsub QWORD PTR ds:0xb67128
                {
                    let x = st0(&st);
                    set(&mut st, 0, x - mem.read::<f64>(mem.rebase(0xb67128)));
                }
                // 0x913b0c: fstp DWORD PTR [ebp-0x234]
                mem.write(r.ebp.wrapping_sub(0x234), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913b12: jmp 0x913b16
                pc = 0x913b16;
            }
            0x913b14 => {
                // 0x913b14: fstp st(0)
                {
                    let x = st0(&st);
                    set(&mut st, 0, x);
                }
                st.pop();
                pc = 0x913b16;
            }
            0x913b16 => {
                // 0x913b16: fld DWORD PTR [ebx+0xbe098]
                st.push(ld32(mem, r.ebx.wrapping_add(0xbe098)));
                // 0x913b1c: fmul DWORD PTR [ebp-0x238]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, r.ebp.wrapping_sub(0x238)));
                }
                // 0x913b22: fdiv QWORD PTR ds:0xb2c5c8
                {
                    let x = st0(&st);
                    set(&mut st, 0, x / mem.read::<f64>(mem.rebase(0xb2c5c8)));
                }
                // 0x913b28: fmulp st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, x * y);
                }
                st.pop();
                // 0x913b2a: fadd DWORD PTR [ebp-0x21c]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, r.ebp.wrapping_sub(0x21c)));
                }
                // 0x913b30: fstp DWORD PTR [ebp-0x21c]
                mem.write(r.ebp.wrapping_sub(0x21c), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913b36: fld DWORD PTR [ebp-0x240]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x240)));
                // 0x913b3c: call 0x9498c8
                {
                    let x = st0(&st);
                    st.pop();
                    let f = mem.rebase(0x9498c8);
                    let v = fcall(mem, f, FArgs::St0(x));
                    st.push(v);
                }
                // 0x913b41: fstp DWORD PTR [ebp-0x224]
                mem.write(r.ebp.wrapping_sub(0x224), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913b47: fld DWORD PTR [ebp-0x224]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x224)));
                // 0x913b4d: fmul DWORD PTR [ebx+0xbe098]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, r.ebx.wrapping_add(0xbe098)));
                }
                // 0x913b53: fdiv QWORD PTR ds:0xb2c5c8
                {
                    let x = st0(&st);
                    set(&mut st, 0, x / mem.read::<f64>(mem.rebase(0xb2c5c8)));
                }
                // 0x913b59: fld DWORD PTR [ebp-0x230]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x230)));
                // 0x913b5f: fld st(0)
                {
                    let x = sti(&st, 0);
                    st.push(x);
                }
                // 0x913b61: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0x913b63: fld DWORD PTR [ebp-0x22c]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x22c)));
                // 0x913b69: faddp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x + y);
                }
                st.pop();
                // 0x913b6b: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0x913b6d: fstp DWORD PTR [ebp-0x22c]
                mem.write(r.ebp.wrapping_sub(0x22c), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913b73: fld DWORD PTR [ebp-0x250]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x250)));
                // 0x913b79: fadd DWORD PTR [ebp-0x240]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, r.ebp.wrapping_sub(0x240)));
                }
                // 0x913b7f: fstp DWORD PTR [ebp-0x240]
                mem.write(r.ebp.wrapping_sub(0x240), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913b85: fld DWORD PTR [ebp-0x240]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x240)));
                // 0x913b8b: fld QWORD PTR ds:0xb67118
                st.push(mem.read::<f64>(mem.rebase(0xb67118)));
                // 0x913b91: fcomp st(1)
                r.sw = fcom(st0(&st), sti(&st, 1));
                st.pop();
                // 0x913b93: fnstsw ax
                r.eax = (r.eax & 0xffff_00ff) | (u32::from(r.sw) << 8);
                // 0x913b95: test ah,0x5
                r.flag = ((r.eax >> 8) & 0x5, 0, true);
                r.cmp = false;
                // 0x913b98: jp 0x913ba8
                pc = if r.pf() { 0x913ba8 } else { 0x913b9a };
            }
            0x913b9a => {
                // 0x913b9a: fsub QWORD PTR ds:0xb67128
                {
                    let x = st0(&st);
                    set(&mut st, 0, x - mem.read::<f64>(mem.rebase(0xb67128)));
                }
                // 0x913ba0: fstp DWORD PTR [ebp-0x240]
                mem.write(r.ebp.wrapping_sub(0x240), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913ba6: jmp 0x913baa
                pc = 0x913baa;
            }
            0x913ba8 => {
                // 0x913ba8: fstp st(0)
                {
                    let x = st0(&st);
                    set(&mut st, 0, x);
                }
                st.pop();
                pc = 0x913baa;
            }
            0x913baa => {
                // 0x913baa: fld DWORD PTR [ebp-0x21c]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x21c)));
                // 0x913bb0: fldz
                st.push(0.0);
                // 0x913bb2: fcom st(1)
                r.sw = fcom(st0(&st), sti(&st, 1));
                // 0x913bb4: fnstsw ax
                r.eax = (r.eax & 0xffff_00ff) | (u32::from(r.sw) << 8);
                // 0x913bb6: test ah,0x41
                r.flag = ((r.eax >> 8) & 0x41, 0, true);
                r.cmp = false;
                // 0x913bb9: jne 0x913bcb
                pc = if !r.zf() { 0x913bcb } else { 0x913bbb };
            }
            0x913bbb => {
                // 0x913bbb: fstp st(1)
                {
                    let x = st0(&st);
                    set(&mut st, 1, x);
                }
                st.pop();
                // 0x913bbd: fst DWORD PTR [ebp-0x21c]
                mem.write(r.ebp.wrapping_sub(0x21c), crate::x87::store(st0(&st)));
                // 0x913bc3: fld DWORD PTR [ebp-0x21c]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x21c)));
                // 0x913bc9: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                pc = 0x913bcb;
            }
            0x913bcb => {
                // 0x913bcb: fld DWORD PTR [ebp-0x22c]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x22c)));
                // 0x913bd1: fcom st(1)
                r.sw = fcom(st0(&st), sti(&st, 1));
                // 0x913bd3: fnstsw ax
                r.eax = (r.eax & 0xffff_00ff) | (u32::from(r.sw) << 8);
                // 0x913bd5: test ah,0x5
                r.flag = ((r.eax >> 8) & 0x5, 0, true);
                r.cmp = false;
                // 0x913bd8: jp 0x913bea
                pc = if r.pf() { 0x913bea } else { 0x913bda };
            }
            0x913bda => {
                // 0x913bda: fstp st(0)
                {
                    let x = st0(&st);
                    set(&mut st, 0, x);
                }
                st.pop();
                // 0x913bdc: fstp DWORD PTR [ebp-0x22c]
                mem.write(r.ebp.wrapping_sub(0x22c), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913be2: fld DWORD PTR [ebp-0x22c]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x22c)));
                // 0x913be8: jmp 0x913bec
                pc = 0x913bec;
            }
            0x913bea => {
                // 0x913bea: fstp st(1)
                {
                    let x = st0(&st);
                    set(&mut st, 1, x);
                }
                st.pop();
                pc = 0x913bec;
            }
            0x913bec => {
                // 0x913bec: fld1
                st.push(1.0);
                // 0x913bee: fcom st(2)
                r.sw = fcom(st0(&st), sti(&st, 2));
                // 0x913bf0: fnstsw ax
                r.eax = (r.eax & 0xffff_00ff) | (u32::from(r.sw) << 8);
                // 0x913bf2: test ah,0x5
                r.flag = ((r.eax >> 8) & 0x5, 0, true);
                r.cmp = false;
                // 0x913bf5: jp 0x913c0b
                pc = if r.pf() { 0x913c0b } else { 0x913bf7 };
            }
            0x913bf7 => {
                // 0x913bf7: fstp st(2)
                {
                    let x = st0(&st);
                    set(&mut st, 2, x);
                }
                st.pop();
                // 0x913bf9: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0x913bfb: fst DWORD PTR [ebp-0x21c]
                mem.write(r.ebp.wrapping_sub(0x21c), crate::x87::store(st0(&st)));
                // 0x913c01: fld DWORD PTR [ebp-0x21c]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x21c)));
                // 0x913c07: fxch st(2)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 2);
                }
                // 0x913c09: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                pc = 0x913c0b;
            }
            0x913c0b => {
                // 0x913c0b: fcom st(1)
                r.sw = fcom(st0(&st), sti(&st, 1));
                // 0x913c0d: fnstsw ax
                r.eax = (r.eax & 0xffff_00ff) | (u32::from(r.sw) << 8);
                // 0x913c0f: test ah,0x5
                r.flag = ((r.eax >> 8) & 0x5, 0, true);
                r.cmp = false;
                // 0x913c12: jp 0x913c24
                pc = if r.pf() { 0x913c24 } else { 0x913c14 };
            }
            0x913c14 => {
                // 0x913c14: fstp st(1)
                {
                    let x = st0(&st);
                    set(&mut st, 1, x);
                }
                st.pop();
                // 0x913c16: fst DWORD PTR [ebp-0x22c]
                mem.write(r.ebp.wrapping_sub(0x22c), crate::x87::store(st0(&st)));
                // 0x913c1c: fld DWORD PTR [ebp-0x22c]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x22c)));
                // 0x913c22: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                pc = 0x913c24;
            }
            0x913c24 => {
                // 0x913c24: mov eax,DWORD PTR [ebp-0x21c]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x21c));
                // 0x913c2a: mov DWORD PTR [esi+0x84],eax
                mem.write(r.esi.wrapping_add(0x84), r.eax);
                // 0x913c30: mov eax,DWORD PTR [ebp-0x22c]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x22c));
                // 0x913c36: mov DWORD PTR [esi+0x88],eax
                mem.write(r.esi.wrapping_add(0x88), r.eax);
                // 0x913c3c: mov DWORD PTR [esi+0x8c],0x3f800000
                mem.write(r.esi.wrapping_add(0x8c), 0x3f800000u32);
                // 0x913c46: test edi,edi
                r.flag = (r.edi & r.edi, 0, false);
                r.cmp = false;
                // 0x913c48: je 0x913da5
                pc = if r.zf() { 0x913da5 } else { 0x913c4e };
            }
            0x913c4e => {
                // 0x913c4e: fcomp st(3)
                r.sw = fcom(st0(&st), sti(&st, 3));
                st.pop();
                // 0x913c50: fnstsw ax
                r.eax = (r.eax & 0xffff_00ff) | (u32::from(r.sw) << 8);
                // 0x913c52: test ah,0x41
                r.flag = ((r.eax >> 8) & 0x41, 0, true);
                r.cmp = false;
                // 0x913c55: jne 0x913da7
                pc = if !r.zf() { 0x913da7 } else { 0x913c5b };
            }
            0x913c5b => {
                // 0x913c5b: movzx eax,BYTE PTR [ebx+0xbe0a4]
                r.eax = u32::from(mem.read::<u8>(r.ebx.wrapping_add(0xbe0a4)));
                // 0x913c62: fnstcw WORD PTR [ebp-0x21a]
                mem.write(r.ebp.wrapping_sub(0x21a), r.cw);
                // 0x913c68: mov DWORD PTR [ebp-0x224],eax
                mem.write(r.ebp.wrapping_sub(0x224), r.eax);
                // 0x913c6e: fild DWORD PTR [ebp-0x224]
                st.push(f64::from(mem.read::<i32>(r.ebp.wrapping_sub(0x224))));
                // 0x913c74: fstp QWORD PTR [ebp-0x228]
                mem.write(r.ebp.wrapping_sub(0x228), st0(&st));
                st.pop();
                // 0x913c7a: fld QWORD PTR [ebp-0x228]
                st.push(mem.read::<f64>(r.ebp.wrapping_sub(0x228)));
                // 0x913c80: movzx eax,WORD PTR [ebp-0x21a]
                r.eax = u32::from(mem.read::<u16>(r.ebp.wrapping_sub(0x21a)));
                // 0x913c87: fld QWORD PTR ds:0xb4c170
                st.push(mem.read::<f64>(mem.rebase(0xb4c170)));
                // 0x913c8d: or eax,0xc00
                {
                    let v = r.eax | 0xc00u32;
                    r.eax = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0x913c92: fsub st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, x - y);
                }
                // 0x913c94: mov DWORD PTR [ebp-0x224],eax
                mem.write(r.ebp.wrapping_sub(0x224), r.eax);
                // 0x913c9a: fld st(4)
                {
                    let x = sti(&st, 4);
                    st.push(x);
                }
                // 0x913c9c: fmulp st(2),st
                {
                    let (x, y) = (sti(&st, 2), sti(&st, 0));
                    set(&mut st, 2, x * y);
                }
                st.pop();
                // 0x913c9e: fadd st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, x + y);
                }
                // 0x913ca0: fldcw WORD PTR [ebp-0x224]
                r.cw = mem.read::<u16>(r.ebp.wrapping_sub(0x224));
                // 0x913ca6: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0x913ca8: fistp QWORD PTR [ebp-0x228]
                {
                    let v = fist64(st0(&st), r.cw);
                    mem.write(r.ebp.wrapping_sub(0x228), v);
                    st.pop();
                }
                // 0x913cae: mov eax,DWORD PTR [ebp-0x228]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x228));
                // 0x913cb4: mov DWORD PTR [esi-0xc],eax
                mem.write(r.esi.wrapping_sub(0xc), r.eax);
                // 0x913cb7: movzx eax,BYTE PTR [ebx+0xbe0a5]
                r.eax = u32::from(mem.read::<u8>(r.ebx.wrapping_add(0xbe0a5)));
                // 0x913cbe: fldcw WORD PTR [ebp-0x21a]
                r.cw = mem.read::<u16>(r.ebp.wrapping_sub(0x21a));
                // 0x913cc4: mov DWORD PTR [ebp-0x224],eax
                mem.write(r.ebp.wrapping_sub(0x224), r.eax);
                // 0x913cca: fild DWORD PTR [ebp-0x224]
                st.push(f64::from(mem.read::<i32>(r.ebp.wrapping_sub(0x224))));
                // 0x913cd0: fnstcw WORD PTR [ebp-0x21a]
                mem.write(r.ebp.wrapping_sub(0x21a), r.cw);
                // 0x913cd6: fstp QWORD PTR [ebp-0x228]
                mem.write(r.ebp.wrapping_sub(0x228), st0(&st));
                st.pop();
                // 0x913cdc: fld QWORD PTR [ebp-0x228]
                st.push(mem.read::<f64>(r.ebp.wrapping_sub(0x228)));
                // 0x913ce2: fsub st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x - y);
                }
                // 0x913ce4: movzx eax,WORD PTR [ebp-0x21a]
                r.eax = u32::from(mem.read::<u16>(r.ebp.wrapping_sub(0x21a)));
                // 0x913ceb: or eax,0xc00
                {
                    let v = r.eax | 0xc00u32;
                    r.eax = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0x913cf0: mov DWORD PTR [ebp-0x224],eax
                mem.write(r.ebp.wrapping_sub(0x224), r.eax);
                // 0x913cf6: fmul st,st(4)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 4));
                    set(&mut st, 0, x * y);
                }
                // 0x913cf8: fadd st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x + y);
                }
                // 0x913cfa: fldcw WORD PTR [ebp-0x224]
                r.cw = mem.read::<u16>(r.ebp.wrapping_sub(0x224));
                // 0x913d00: fistp QWORD PTR [ebp-0x228]
                {
                    let v = fist64(st0(&st), r.cw);
                    mem.write(r.ebp.wrapping_sub(0x228), v);
                    st.pop();
                }
                // 0x913d06: mov eax,DWORD PTR [ebp-0x228]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x228));
                // 0x913d0c: mov DWORD PTR [esi-0x8],eax
                mem.write(r.esi.wrapping_sub(0x8), r.eax);
                // 0x913d0f: movzx eax,BYTE PTR [ebx+0xbe0a6]
                r.eax = u32::from(mem.read::<u8>(r.ebx.wrapping_add(0xbe0a6)));
                // 0x913d16: fldcw WORD PTR [ebp-0x21a]
                r.cw = mem.read::<u16>(r.ebp.wrapping_sub(0x21a));
                // 0x913d1c: mov DWORD PTR [ebp-0x224],eax
                mem.write(r.ebp.wrapping_sub(0x224), r.eax);
                // 0x913d22: fild DWORD PTR [ebp-0x224]
                st.push(f64::from(mem.read::<i32>(r.ebp.wrapping_sub(0x224))));
                // 0x913d28: fnstcw WORD PTR [ebp-0x21a]
                mem.write(r.ebp.wrapping_sub(0x21a), r.cw);
                // 0x913d2e: fstp QWORD PTR [ebp-0x228]
                mem.write(r.ebp.wrapping_sub(0x228), st0(&st));
                st.pop();
                // 0x913d34: fld QWORD PTR [ebp-0x228]
                st.push(mem.read::<f64>(r.ebp.wrapping_sub(0x228)));
                // 0x913d3a: fsub st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x - y);
                }
                // 0x913d3c: movzx eax,WORD PTR [ebp-0x21a]
                r.eax = u32::from(mem.read::<u16>(r.ebp.wrapping_sub(0x21a)));
                // 0x913d43: or eax,0xc00
                {
                    let v = r.eax | 0xc00u32;
                    r.eax = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0x913d48: mov DWORD PTR [ebp-0x224],eax
                mem.write(r.ebp.wrapping_sub(0x224), r.eax);
                // 0x913d4e: fmul st,st(4)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 4));
                    set(&mut st, 0, x * y);
                }
                // 0x913d50: fadd st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x + y);
                }
                // 0x913d52: fldcw WORD PTR [ebp-0x224]
                r.cw = mem.read::<u16>(r.ebp.wrapping_sub(0x224));
                // 0x913d58: fistp QWORD PTR [ebp-0x228]
                {
                    let v = fist64(st0(&st), r.cw);
                    mem.write(r.ebp.wrapping_sub(0x228), v);
                    st.pop();
                }
                // 0x913d5e: mov eax,DWORD PTR [ebp-0x228]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x228));
                // 0x913d64: mov DWORD PTR [esi-0x4],eax
                mem.write(r.esi.wrapping_sub(0x4), r.eax);
                // 0x913d67: fldcw WORD PTR [ebp-0x21a]
                r.cw = mem.read::<u16>(r.ebp.wrapping_sub(0x21a));
                // 0x913d6d: fnstcw WORD PTR [ebp-0x21a]
                mem.write(r.ebp.wrapping_sub(0x21a), r.cw);
                // 0x913d73: fmulp st(3),st
                {
                    let (x, y) = (sti(&st, 3), sti(&st, 0));
                    set(&mut st, 3, x * y);
                }
                st.pop();
                // 0x913d75: movzx eax,WORD PTR [ebp-0x21a]
                r.eax = u32::from(mem.read::<u16>(r.ebp.wrapping_sub(0x21a)));
                // 0x913d7c: fxch st(2)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 2);
                }
                // 0x913d7e: or eax,0xc00
                {
                    let v = r.eax | 0xc00u32;
                    r.eax = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0x913d83: mov DWORD PTR [ebp-0x224],eax
                mem.write(r.ebp.wrapping_sub(0x224), r.eax);
                // 0x913d89: fldcw WORD PTR [ebp-0x224]
                r.cw = mem.read::<u16>(r.ebp.wrapping_sub(0x224));
                // 0x913d8f: fistp QWORD PTR [ebp-0x228]
                {
                    let v = fist64(st0(&st), r.cw);
                    mem.write(r.ebp.wrapping_sub(0x228), v);
                    st.pop();
                }
                // 0x913d95: mov eax,DWORD PTR [ebp-0x228]
                r.eax = mem.read::<u32>(r.ebp.wrapping_sub(0x228));
                // 0x913d9b: mov DWORD PTR [esi],eax
                mem.write(r.esi, r.eax);
                // 0x913d9d: fldcw WORD PTR [ebp-0x21a]
                r.cw = mem.read::<u16>(r.ebp.wrapping_sub(0x21a));
                // 0x913da3: jmp 0x913dcd
                pc = 0x913dcd;
            }
            0x913da5 => {
                // 0x913da5: fstp st(0)
                {
                    let x = st0(&st);
                    set(&mut st, 0, x);
                }
                st.pop();
                pc = 0x913da7;
            }
            0x913da7 => {
                // 0x913da7: movzx eax,BYTE PTR [ebx+0xbe0a4]
                r.eax = u32::from(mem.read::<u8>(r.ebx.wrapping_add(0xbe0a4)));
                // 0x913dae: fstp st(2)
                {
                    let x = st0(&st);
                    set(&mut st, 2, x);
                }
                st.pop();
                // 0x913db0: mov DWORD PTR [esi-0xc],eax
                mem.write(r.esi.wrapping_sub(0xc), r.eax);
                // 0x913db3: movzx eax,BYTE PTR [ebx+0xbe0a5]
                r.eax = u32::from(mem.read::<u8>(r.ebx.wrapping_add(0xbe0a5)));
                // 0x913dba: mov DWORD PTR [esi-0x8],eax
                mem.write(r.esi.wrapping_sub(0x8), r.eax);
                // 0x913dbd: movzx eax,BYTE PTR [ebx+0xbe0a6]
                r.eax = u32::from(mem.read::<u8>(r.ebx.wrapping_add(0xbe0a6)));
                // 0x913dc4: mov DWORD PTR [esi-0x4],eax
                mem.write(r.esi.wrapping_sub(0x4), r.eax);
                // 0x913dc7: mov DWORD PTR [esi],0x80
                mem.write(r.esi, 0x80u32);
                pc = 0x913dcd;
            }
            0x913dcd => {
                // 0x913dcd: mov eax,DWORD PTR [esi]
                r.eax = mem.read::<u32>(r.esi);
                // 0x913dcf: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0x913dd1: sub esp,0x18
                {
                    let v = r.esp.wrapping_sub(0x18u32);
                    r.esp = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0x913dd4: fstp DWORD PTR [esp+0x14]
                mem.write(r.esp.wrapping_add(0x14), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913dd8: fstp DWORD PTR [esp+0x10]
                mem.write(r.esp.wrapping_add(0x10), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913ddc: fild DWORD PTR [esi]
                st.push(f64::from(mem.read::<i32>(r.esi)));
                // 0x913dde: test eax,eax
                r.flag = (r.eax & r.eax, 0, false);
                r.cmp = false;
                // 0x913de0: jns 0x913de8
                pc = if !r.sf() { 0x913de8 } else { 0x913de2 };
            }
            0x913de2 => {
                // 0x913de2: fadd DWORD PTR ds:0xb11f14
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xb11f14)));
                }
                pc = 0x913de8;
            }
            0x913de8 => {
                // 0x913de8: fstp DWORD PTR [ebp-0x224]
                mem.write(r.ebp.wrapping_sub(0x224), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913dee: fld DWORD PTR [ebp-0x224]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x224)));
                // 0x913df4: mov eax,DWORD PTR [esi-0x4]
                r.eax = mem.read::<u32>(r.esi.wrapping_sub(0x4));
                // 0x913df7: fld QWORD PTR ds:0xb170b8
                st.push(mem.read::<f64>(mem.rebase(0xb170b8)));
                // 0x913dfd: fmul st(1),st
                {
                    let (x, y) = (sti(&st, 1), sti(&st, 0));
                    set(&mut st, 1, x * y);
                }
                // 0x913dff: fxch st(1)
                {
                    let n = st.len();
                    st.swap(n - 1, n - 1 - 1);
                }
                // 0x913e01: fstp DWORD PTR [ebp-0x224]
                mem.write(r.ebp.wrapping_sub(0x224), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913e07: fld DWORD PTR [ebp-0x224]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x224)));
                // 0x913e0d: fstp DWORD PTR [esp+0xc]
                mem.write(r.esp.wrapping_add(0xc), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913e11: fild DWORD PTR [esi-0x4]
                st.push(f64::from(mem.read::<i32>(r.esi.wrapping_sub(0x4))));
                // 0x913e14: test eax,eax
                r.flag = (r.eax & r.eax, 0, false);
                r.cmp = false;
                // 0x913e16: jns 0x913e1e
                pc = if !r.sf() { 0x913e1e } else { 0x913e18 };
            }
            0x913e18 => {
                // 0x913e18: fadd DWORD PTR ds:0xb11f14
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xb11f14)));
                }
                pc = 0x913e1e;
            }
            0x913e1e => {
                // 0x913e1e: fstp DWORD PTR [ebp-0x224]
                mem.write(r.ebp.wrapping_sub(0x224), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913e24: fld DWORD PTR [ebp-0x224]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x224)));
                // 0x913e2a: mov eax,DWORD PTR [esi-0x8]
                r.eax = mem.read::<u32>(r.esi.wrapping_sub(0x8));
                // 0x913e2d: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0x913e2f: fstp DWORD PTR [ebp-0x224]
                mem.write(r.ebp.wrapping_sub(0x224), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913e35: fld DWORD PTR [ebp-0x224]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x224)));
                // 0x913e3b: fstp DWORD PTR [esp+0x8]
                mem.write(r.esp.wrapping_add(0x8), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913e3f: fild DWORD PTR [esi-0x8]
                st.push(f64::from(mem.read::<i32>(r.esi.wrapping_sub(0x8))));
                // 0x913e42: test eax,eax
                r.flag = (r.eax & r.eax, 0, false);
                r.cmp = false;
                // 0x913e44: jns 0x913e4c
                pc = if !r.sf() { 0x913e4c } else { 0x913e46 };
            }
            0x913e46 => {
                // 0x913e46: fadd DWORD PTR ds:0xb11f14
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xb11f14)));
                }
                pc = 0x913e4c;
            }
            0x913e4c => {
                // 0x913e4c: fstp DWORD PTR [ebp-0x224]
                mem.write(r.ebp.wrapping_sub(0x224), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913e52: fld DWORD PTR [ebp-0x224]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x224)));
                // 0x913e58: mov eax,DWORD PTR [esi-0xc]
                r.eax = mem.read::<u32>(r.esi.wrapping_sub(0xc));
                // 0x913e5b: fmul st,st(1)
                {
                    let (x, y) = (sti(&st, 0), sti(&st, 1));
                    set(&mut st, 0, x * y);
                }
                // 0x913e5d: fstp DWORD PTR [ebp-0x224]
                mem.write(r.ebp.wrapping_sub(0x224), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913e63: fld DWORD PTR [ebp-0x224]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x224)));
                // 0x913e69: fstp DWORD PTR [esp+0x4]
                mem.write(r.esp.wrapping_add(0x4), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913e6d: fild DWORD PTR [esi-0xc]
                st.push(f64::from(mem.read::<i32>(r.esi.wrapping_sub(0xc))));
                // 0x913e70: test eax,eax
                r.flag = (r.eax & r.eax, 0, false);
                r.cmp = false;
                // 0x913e72: jns 0x913e7a
                pc = if !r.sf() { 0x913e7a } else { 0x913e74 };
            }
            0x913e74 => {
                // 0x913e74: fadd DWORD PTR ds:0xb11f14
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, mem.rebase(0xb11f14)));
                }
                pc = 0x913e7a;
            }
            0x913e7a => {
                // 0x913e7a: mov edi,DWORD PTR [ebp-0x218]
                r.edi = mem.read::<u32>(r.ebp.wrapping_sub(0x218));
                // 0x913e80: fstp DWORD PTR [ebp-0x224]
                mem.write(r.ebp.wrapping_sub(0x224), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913e86: fmul DWORD PTR [ebp-0x224]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * ld32(mem, r.ebp.wrapping_sub(0x224)));
                }
                // 0x913e8c: fstp DWORD PTR [ebp-0x224]
                mem.write(r.ebp.wrapping_sub(0x224), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913e92: fld DWORD PTR [ebp-0x224]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x224)));
                // 0x913e98: fstp DWORD PTR [esp]
                mem.write(r.esp, crate::x87::store(st0(&st)));
                st.pop();
                // 0x913e9b: push edi
                {
                    let v = r.edi;
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0x913e9c: push DWORD PTR [ebp-0x248]
                {
                    let v = mem.read::<u32>(r.ebp.wrapping_sub(0x248));
                    r.esp = r.esp.wrapping_sub(4);
                    mem.write(r.esp, v);
                }
                // 0x913ea2: call 0x644940
                {
                    let a: Vec<u32> = (0..8)
                        .map(|k| mem.read::<u32>(r.esp.wrapping_add(4 * k)))
                        .collect();
                    let f = mem.rebase(0x644940);
                    r.eax = call(mem, f, &a);
                }
                // 0x913ea7: fld DWORD PTR [ebp-0x23c]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x23c)));
                // 0x913ead: fld DWORD PTR [ebp-0x220]
                st.push(ld32(mem, r.ebp.wrapping_sub(0x220)));
                // 0x913eb3: inc edi
                {
                    let v = r.edi.wrapping_add(1);
                    r.edi = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0x913eb4: add esp,0x20
                {
                    let v = r.esp.wrapping_add(0x20u32);
                    r.esp = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0x913eb7: add esi,0x10
                {
                    let v = r.esi.wrapping_add(0x10u32);
                    r.esi = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0x913eba: mov DWORD PTR [ebp-0x218],edi
                mem.write(r.ebp.wrapping_sub(0x218), r.edi);
                // 0x913ec0: cmp edi,0x9
                r.flag = (r.edi, 0x9u32, false);
                r.cmp = true;
                // 0x913ec3: jl 0x9139db
                pc = if r.lt() { 0x9139db } else { 0x913ec9 };
            }
            0x913ec9 => {
                // 0x913ec9: mov edi,DWORD PTR [ebp-0x248]
                r.edi = mem.read::<u32>(r.ebp.wrapping_sub(0x248));
                // 0x913ecf: fstp st(1)
                {
                    let x = st0(&st);
                    set(&mut st, 1, x);
                }
                st.pop();
                // 0x913ed1: inc edi
                {
                    let v = r.edi.wrapping_add(1);
                    r.edi = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0x913ed2: fstp st(0)
                {
                    let x = st0(&st);
                    set(&mut st, 0, x);
                }
                st.pop();
                // 0x913ed4: fld QWORD PTR ds:0xb67128
                st.push(mem.read::<f64>(mem.rebase(0xb67128)));
                // 0x913eda: mov DWORD PTR [ebp-0x248],edi
                mem.write(r.ebp.wrapping_sub(0x248), r.edi);
                // 0x913ee0: cmp edi,0x9
                r.flag = (r.edi, 0x9u32, false);
                r.cmp = true;
                // 0x913ee3: jl 0x913307
                pc = if r.lt() { 0x913307 } else { 0x913ee9 };
            }
            0x913ee9 => {
                // 0x913ee9: mov ecx,DWORD PTR [ebp-0x4]
                r.ecx = mem.read::<u32>(r.ebp.wrapping_sub(0x4));
                // 0x913eec: fstp st(0)
                {
                    let x = st0(&st);
                    set(&mut st, 0, x);
                }
                st.pop();
                // 0x913eee: fld DWORD PTR [ebx+0xbe090]
                st.push(ld32(mem, r.ebx.wrapping_add(0xbe090)));
                // 0x913ef4: inc DWORD PTR [ebx+0xbe0a8]
                {
                    let v = mem.read::<u32>(r.ebx.wrapping_add(0xbe0a8)).wrapping_add(1);
                    mem.write(r.ebx.wrapping_add(0xbe0a8), v);
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0x913efa: fmul QWORD PTR ds:0xb41d00
                {
                    let x = st0(&st);
                    set(&mut st, 0, x * mem.read::<f64>(mem.rebase(0xb41d00)));
                }
                // 0x913f00: pop edi
                {
                    r.edi = mem.read::<u32>(r.esp);
                    r.esp = r.esp.wrapping_add(4);
                }
                // 0x913f01: pop esi
                {
                    r.esi = mem.read::<u32>(r.esp);
                    r.esp = r.esp.wrapping_add(4);
                }
                // 0x913f02: xor ecx,ebp
                {
                    let v = r.ecx ^ r.ebp;
                    r.ecx = v;
                    r.flag = (v, 0, false);
                    r.cmp = false;
                }
                // 0x913f04: fadd DWORD PTR [ebx+0xbe080]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, r.ebx.wrapping_add(0xbe080)));
                }
                // 0x913f0a: fstp DWORD PTR [ebx+0xbe080]
                mem.write(r.ebx.wrapping_add(0xbe080), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913f10: fld DWORD PTR [ebx+0xbe084]
                st.push(ld32(mem, r.ebx.wrapping_add(0xbe084)));
                // 0x913f16: fadd DWORD PTR [ebx+0xbe090]
                {
                    let x = st0(&st);
                    set(&mut st, 0, x + ld32(mem, r.ebx.wrapping_add(0xbe090)));
                }
                // 0x913f1c: fstp DWORD PTR [ebx+0xbe084]
                mem.write(r.ebx.wrapping_add(0xbe084), crate::x87::store(st0(&st)));
                st.pop();
                // 0x913f22: pop ebx
                {
                    r.ebx = mem.read::<u32>(r.esp);
                    r.esp = r.esp.wrapping_add(4);
                }
                // 0x913f23: call 0x94923a
                // security cookie check
                // 0x913f28: mov esp,ebp
                r.esp = r.ebp;
                // 0x913f2a: pop ebp
                {
                    r.ebp = mem.read::<u32>(r.esp);
                    r.esp = r.esp.wrapping_add(4);
                }
                // 0x913f2b: ret
                break;
            }

            _ => break,
        }
    }
    r.eax
}
