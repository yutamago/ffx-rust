//! The per-thread Unicorn instance and the run loop.

use std::cell::Cell;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use unicorn_engine::{Arch, HookType, MemType, Mode, Prot, RegisterX86, Unicorn};

use crate::crt::{self, SENTINEL_OFFSET};
use crate::{Config, Error, Host, Loaded, Regs, config, registry};

/// What stopped Unicorn, as the fault hook saw it.
#[derive(Debug, Clone, Copy)]
enum Fault {
    /// An instruction fetch outside overlay code: a call-out, an import,
    /// or the return.
    Fetch(u32),
    /// A data access that couldn't be mapped.
    Data(u32, MemType),
}

/// One mapped range of this engine.
#[derive(Debug, Clone, Copy)]
struct Region {
    start: u64,
    end: u64,
    /// The overlay whose image it belongs to.
    image: Option<u64>,
    text: bool,
}

#[derive(Debug, Default)]
struct State {
    base: usize,
    regions: Vec<Region>,
    generation: u64,
    /// The overlays the regions were made for (kept alive while mapped:
    /// their code buffers back Unicorn memory).
    images: Vec<Arc<Loaded>>,
    fault: Option<Fault>,
}

struct Engine {
    uc: Unicorn<'static, State>,
    depth: u32,
}

thread_local! {
    static ENGINE: Cell<*mut Engine> = const { Cell::new(std::ptr::null_mut()) };
}

const PAGE: u64 = 0x1000;
/// The lowest address mapped on demand: the first 64 KiB stay unmapped,
/// so a null pointer faults as it would natively.
const LOWEST: u64 = 0x1_0000;
const CPU_CORE2DUO: i32 = 2;

fn overlaps(a: (u64, u64), b: (u64, u64)) -> bool {
    a.0 < b.1 && b.0 < a.1
}

/// Maps guest memory for a data access at `addr`: the overlay image
/// holding it (code from its private copy), else the 1 MiB block around
/// it, or the 64 KiB block when an image or an earlier mapping is in the
/// way. `true` when `addr` is mapped afterwards.
fn map_for(uc: &mut Unicorn<'_, State>, addr: u64) -> bool {
    if !(LOWEST..0x1_0000_0000).contains(&addr) {
        return false;
    }
    if uc
        .get_data()
        .regions
        .iter()
        .any(|r| addr >= r.start && addr < r.end)
    {
        return true;
    }
    let images: Vec<Arc<Loaded>> = registry().images.clone();
    if let Some(l) = images
        .iter()
        .find(|l| addr >= u64::from(l.base) && addr < u64::from(l.base) + u64::from(l.span))
    {
        return map_image(uc, l);
    }
    let taken = |uc: &Unicorn<'_, State>, r: (u64, u64)| {
        images.iter().any(|l| {
            overlaps(
                r,
                (u64::from(l.base), u64::from(l.base) + u64::from(l.span)),
            )
        }) || uc
            .get_data()
            .regions
            .iter()
            .any(|m| overlaps(r, (m.start, m.end)))
    };
    let mut block = (addr & !0xf_ffff, (addr & !0xf_ffff) + 0x10_0000);
    if block.0 < LOWEST || taken(uc, block) {
        block = (addr & !0xffff, (addr & !0xffff) + 0x1_0000);
        if taken(uc, block) {
            block = (addr & !(PAGE - 1), (addr & !(PAGE - 1)) + PAGE);
        }
    }
    map_identity(uc, block.0, block.1, None)
}

fn map_identity(uc: &mut Unicorn<'_, State>, start: u64, end: u64, image: Option<u64>) -> bool {
    if start >= end {
        return true;
    }
    let host = uc.get_data().base.wrapping_add(start as usize);
    // SAFETY: guest `start..end` is the embedder's memory at `base + start`
    // (the `configure` contract); Unicorn reaches it only through accesses
    // the overlay code makes, which native code would make the same way.
    let ok = unsafe {
        uc.mem_map_ptr(
            start,
            end - start,
            Prot::READ | Prot::WRITE,
            host as *mut std::ffi::c_void,
        )
    }
    .is_ok();
    if ok {
        uc.get_data_mut().regions.push(Region {
            start,
            end,
            image,
            text: false,
        });
    }
    ok
}

/// An overlay's image: data through the embedder's memory, code from the
/// private copy, read-execute.
fn map_image(uc: &mut Unicorn<'_, State>, l: &Arc<Loaded>) -> bool {
    let (b, t0, t1, e) = (
        u64::from(l.base),
        u64::from(l.text_start),
        u64::from(l.text_end),
        u64::from(l.base) + u64::from(l.span),
    );
    if !map_identity(uc, b, t0, Some(l.id)) || !map_identity(uc, t1, e, Some(l.id)) {
        return false;
    }
    // SAFETY: the buffer lives as long as `l`, which `State::images` keeps
    // until the region is unmapped.
    let ok = unsafe {
        uc.mem_map_ptr(
            t0,
            t1 - t0,
            Prot::READ | Prot::EXEC,
            l.text.ptr().cast::<std::ffi::c_void>(),
        )
    }
    .is_ok();
    if ok {
        let s = uc.get_data_mut();
        s.regions.push(Region {
            start: t0,
            end: t1,
            image: Some(l.id),
            text: true,
        });
        s.images.push(Arc::clone(l));
    }
    ok
}

/// Brings the mappings up to the registry: drops unloaded overlays'
/// regions and code, and any block a newly loaded overlay now covers.
fn sync(uc: &mut Unicorn<'_, State>) {
    let (generation, live): (u64, Vec<(u64, u64, u64)>) = {
        let r = registry();
        (
            r.generation,
            r.images
                .iter()
                .map(|l| {
                    (
                        l.id,
                        u64::from(l.base),
                        u64::from(l.base) + u64::from(l.span),
                    )
                })
                .collect(),
        )
    };
    if uc.get_data().generation == generation {
        return;
    }
    let regions = std::mem::take(&mut uc.get_data_mut().regions);
    let mut keep = Vec::with_capacity(regions.len());
    for r in regions {
        let stale = match r.image {
            Some(id) => !live.iter().any(|l| l.0 == id),
            None => live.iter().any(|l| overlaps((r.start, r.end), (l.1, l.2))),
        };
        if stale {
            if r.text {
                let _ = uc.ctl_remove_cache(r.start, r.end);
            }
            let _ = uc.mem_unmap(r.start, r.end - r.start);
        } else {
            keep.push(r);
        }
    }
    let s = uc.get_data_mut();
    s.regions = keep;
    s.images.retain(|l| live.iter().any(|x| x.0 == l.id));
    s.generation = generation;
}

fn on_fault(uc: &mut Unicorn<'_, State>, ty: MemType, addr: u64, size: usize, _v: i64) -> bool {
    match ty {
        MemType::READ_UNMAPPED | MemType::WRITE_UNMAPPED => {
            let last = addr + (size.max(1) as u64) - 1;
            if map_for(uc, addr) && map_for(uc, last) {
                return true;
            }
            uc.get_data_mut().fault = Some(Fault::Data(addr as u32, ty));
            false
        }
        MemType::FETCH_UNMAPPED => {
            // overlay code not mapped in this engine yet: its image
            let a = addr as u32;
            let image = registry()
                .images
                .iter()
                .find(|l| a >= l.text_start && a < l.text_end)
                .cloned();
            if let Some(l) = image
                && map_image(uc, &l)
            {
                return true;
            }
            uc.get_data_mut().fault = Some(Fault::Fetch(a));
            false
        }
        MemType::FETCH_PROT => {
            uc.get_data_mut().fault = Some(Fault::Fetch(addr as u32));
            false
        }
        _ => {
            uc.get_data_mut().fault = Some(Fault::Data(addr as u32, ty));
            false
        }
    }
}

/// A thread block where `fs` points, as on Windows: `fs:[0]` the
/// exception chain (-1, empty; overlay code links its frames there and
/// never into the real thread's), `fs:[0x18]` the block itself. Page one
/// holds a descriptor table (entry 1 `fs`, 2 flat code, 3 flat data:
/// Unicorn ignores a 32-bit `FS_BASE` write).
fn thread_block(uc: &mut Unicorn<'_, State>, cfg: &Config, at: u32) -> Result<(), Error> {
    if !map_for(uc, u64::from(at)) || !map_for(uc, u64::from(at) + 0x1fff) {
        return Err(Error::Host(format!("thread block {at:#x} can't be mapped")));
    }
    let teb = at + 0x1000;
    // SAFETY: `at .. at + 0x2000` came from `Host::alloc`.
    unsafe {
        crt::write_u32(cfg, teb, u32::MAX);
        crt::write_u32(cfg, teb + 4, 0xffff_0000);
        crt::write_u32(cfg, teb + 8, 0x0001_0000);
        crt::write_u32(cfg, teb + 0x18, teb);
        crt::write_u32(cfg, teb + 0x24, 1);
    }
    let descriptor = |base: u64, limit: u64, access: u64, flags: u64| -> u64 {
        (limit & 0xffff)
            | ((base & 0xff_ffff) << 16)
            | (access << 40)
            | (((limit >> 16) & 0xf) << 48)
            | (flags << 52)
            | ((base >> 24) << 56)
    };
    let table = [
        0,
        descriptor(u64::from(teb), 0xfff, 0xf3, 0x4),
        descriptor(0, 0xf_ffff, 0x9b, 0xc),
        descriptor(0, 0xf_ffff, 0x93, 0xc),
    ];
    for (i, d) in (0u32..).zip(table) {
        // SAFETY: as above.
        unsafe { crt::write_bytes(cfg, at + 8 * i, &d.to_le_bytes()) };
    }
    // uc_x86_mmr: selector u16, (pad), base u64, limit u32, flags u32
    let mut gdtr = [0u8; 24];
    gdtr[8..16].copy_from_slice(&u64::from(at).to_le_bytes());
    gdtr[16..20].copy_from_slice(&0x1fu32.to_le_bytes());
    uc.reg_write_long(RegisterX86::GDTR, &gdtr)?;
    let cr0 = uc.reg_read(RegisterX86::CR0)?;
    uc.reg_write(RegisterX86::CR0, cr0 | 1)?;
    // SSE on, as Windows enables it (OSFXSR, OSXMMEXCPT)
    let cr4 = uc.reg_read(RegisterX86::CR4)?;
    uc.reg_write(RegisterX86::CR4, cr4 | 0x600)?;
    uc.reg_write(RegisterX86::MXCSR, 0x1f80)?;
    uc.reg_write(RegisterX86::CS, 0x10)?;
    for seg in [RegisterX86::SS, RegisterX86::DS, RegisterX86::ES] {
        uc.reg_write(seg, 0x18)?;
    }
    uc.reg_write(RegisterX86::FS, 0x0b)?;
    // x87: empty stack, the game's control word
    uc.reg_write(RegisterX86::FPTAG, 0xffff)?;
    uc.reg_write(RegisterX86::FPSW, 0)?;
    uc.reg_write(RegisterX86::FPCW, 0x027f)?;
    Ok(())
}

fn engine(host: &mut impl Host) -> Result<*mut Engine, Error> {
    let e = ENGINE.with(Cell::get);
    if !e.is_null() {
        return Ok(e);
    }
    let cfg = config()?;
    let mut uc = Unicorn::new_with_data(
        Arch::X86,
        Mode::MODE_32,
        State {
            base: cfg.base,
            ..State::default()
        },
    )?;
    // SSE2 and SSE3 as the overlays were built for; no SSE4.2 (Unicorn's
    // string instructions are wrong, ffx-oracle)
    uc.ctl_set_cpu_model(CPU_CORE2DUO)?;
    uc.add_mem_hook(
        HookType::MEM_READ_UNMAPPED
            | HookType::MEM_WRITE_UNMAPPED
            | HookType::MEM_FETCH_UNMAPPED
            | HookType::MEM_READ_PROT
            | HookType::MEM_WRITE_PROT
            | HookType::MEM_FETCH_PROT,
        1,
        0,
        on_fault,
    )?;
    let block = host
        .alloc(0x2000)
        .ok_or_else(|| Error::Host("no memory for the thread block".into()))?;
    thread_block(&mut uc, cfg, block)?;
    let e = Box::into_raw(Box::new(Engine { uc, depth: 0 }));
    ENGINE.with(|c| c.set(e));
    Ok(e)
}

/// `g_MagicImports` by address: the first slot holding each.
fn slot_of(cfg: &Config, target: u32) -> Option<u32> {
    static SLOTS: OnceLock<HashMap<u32, u32>> = OnceLock::new();
    let (table, n) = cfg.import_table?;
    SLOTS
        .get_or_init(|| {
            let mut m = HashMap::new();
            for i in 0..n {
                // SAFETY: the embedder's table, `n` words.
                let f = unsafe { crt::read_u32(cfg, table + 4 * i) };
                m.entry(f).or_insert(i);
            }
            m
        })
        .get(&target)
        .copied()
}

fn read_st(uc: &mut Unicorn<'_, State>, regs: &mut Regs) -> Result<(), Error> {
    let sw = uc.reg_read(RegisterX86::FPSW)? as u16;
    let tag = uc.reg_read(RegisterX86::FPTAG)? as u16;
    let top = (sw >> 11) & 7;
    let mut n = 0u16;
    while n < 8 && (tag >> (2 * ((top + n) & 7))) & 3 != 3 {
        n += 1;
    }
    if n > 2 {
        return Err(Error::Host(format!(
            "{n} values on the x87 stack at a call"
        )));
    }
    for i in 0..n {
        let v = uc.reg_read_long(RegisterX86::ST0 as i32 + i32::from(i))?;
        regs.st[usize::from(i)].copy_from_slice(&v[..10]);
    }
    regs.st_len = n as u8;
    if n > 0 {
        // pop them all: tags empty, top moves up
        let mut t = tag;
        for i in 0..n {
            t |= 3 << (2 * ((top + i) & 7));
        }
        uc.reg_write(RegisterX86::FPTAG, u64::from(t))?;
        let ntop = (top + n) & 7;
        uc.reg_write(RegisterX86::FPSW, u64::from((sw & !0x3800) | (ntop << 11)))?;
    }
    Ok(())
}

fn push_st(uc: &mut Unicorn<'_, State>, value: &[u8; 10]) -> Result<(), Error> {
    let sw = uc.reg_read(RegisterX86::FPSW)? as u16;
    let tag = uc.reg_read(RegisterX86::FPTAG)? as u16;
    let top = ((sw >> 11) & 7).wrapping_sub(1) & 7;
    uc.reg_write(RegisterX86::FPSW, u64::from((sw & !0x3800) | (top << 11)))?;
    uc.reg_write(RegisterX86::FPTAG, u64::from(tag & !(3 << (2 * top))))?;
    uc.reg_write_long(RegisterX86::ST0, value)?;
    Ok(())
}

const GPRS: [RegisterX86; 9] = [
    RegisterX86::EAX,
    RegisterX86::ECX,
    RegisterX86::EDX,
    RegisterX86::EBX,
    RegisterX86::ESP,
    RegisterX86::EBP,
    RegisterX86::ESI,
    RegisterX86::EDI,
    RegisterX86::EFLAGS,
];

fn get_regs(uc: &mut Unicorn<'_, State>) -> Result<Regs, Error> {
    let mut v = [0u32; 9];
    for (o, r) in v.iter_mut().zip(GPRS) {
        *o = uc.reg_read(r)? as u32;
    }
    let mut regs = Regs {
        eax: v[0],
        ecx: v[1],
        edx: v[2],
        ebx: v[3],
        esp: v[4],
        ebp: v[5],
        esi: v[6],
        edi: v[7],
        eflags: v[8],
        fpu_cw: uc.reg_read(RegisterX86::FPCW)? as u16,
        mxcsr: uc.reg_read(RegisterX86::MXCSR)? as u32,
        ..Regs::default()
    };
    regs.xmm0
        .copy_from_slice(&uc.reg_read_long(RegisterX86::XMM0)?[..16]);
    regs.xmm1
        .copy_from_slice(&uc.reg_read_long(RegisterX86::XMM1)?[..16]);
    read_st(uc, &mut regs)?;
    Ok(regs)
}

fn put_regs(uc: &mut Unicorn<'_, State>, regs: &Regs) -> Result<(), Error> {
    let v = [
        regs.eax,
        regs.ecx,
        regs.edx,
        regs.ebx,
        regs.esp,
        regs.ebp,
        regs.esi,
        regs.edi,
        regs.eflags,
    ];
    for (r, x) in GPRS.into_iter().zip(v) {
        uc.reg_write(r, u64::from(x))?;
    }
    uc.reg_write(RegisterX86::FPCW, u64::from(regs.fpu_cw))?;
    uc.reg_write(RegisterX86::MXCSR, u64::from(regs.mxcsr))?;
    uc.reg_write_long(RegisterX86::XMM0, &regs.xmm0)?;
    uc.reg_write_long(RegisterX86::XMM1, &regs.xmm1)?;
    for i in (0..usize::from(regs.st_len.min(2))).rev() {
        push_st(uc, &regs.st[i])?;
    }
    Ok(())
}

fn in_text(uc: &Unicorn<'_, State>, addr: u32) -> bool {
    let a = u64::from(addr);
    uc.get_data()
        .regions
        .iter()
        .any(|r| r.text && a >= r.start && a < r.end)
        || registry()
            .images
            .iter()
            .any(|l| addr >= l.text_start && addr < l.text_end)
}

/// Runs overlay code at `pc` with `regs` (`esp` at the return address,
/// arguments above it) until it returns there: the registers after its
/// `ret` (`st_len` 1 for a float result). Nests: may be called again from
/// inside a [`Host::call_out`] on the same thread.
///
/// # Errors
/// The emulation failed, an import has no stand-in, or a call-out failed.
pub fn call(host: &mut impl Host, pc: u32, regs: &Regs) -> Result<Regs, Error> {
    call_with(host, pc, regs, |_| {})
}

/// [`call`], timing aside: `on_stop` sees every stop of Unicorn (call-outs,
/// imports, the return) with the target address, for profiling.
///
/// # Errors
/// As [`call`].
pub fn call_with(
    host: &mut impl Host,
    pc: u32,
    regs: &Regs,
    mut on_stop: impl FnMut(u32),
) -> Result<Regs, Error> {
    let e = engine(host)?;
    // SAFETY: the engine is this thread's; no reference to it outlives a
    // statement that doesn't call back into the embedder.
    let nested = unsafe { (*e).depth } > 0;
    let saved = if nested {
        // SAFETY: as above.
        Some(unsafe { (*e).uc.context_init()? })
    } else {
        None
    };
    // SAFETY: as above.
    unsafe { (*e).depth += 1 };
    let r = drive(e, host, pc, regs, &mut on_stop);
    // SAFETY: as above.
    unsafe {
        (*e).depth -= 1;
        if let Some(c) = saved {
            (*e).uc.context_restore(&c)?;
        }
    }
    r
}

fn drive(
    e: *mut Engine,
    host: &mut impl Host,
    mut pc: u32,
    regs: &Regs,
    on_stop: &mut impl FnMut(u32),
) -> Result<Regs, Error> {
    let cfg = config()?;
    // SAFETY: the caller's stack: the return address it pushed.
    let ret_addr = unsafe { crt::read_u32(cfg, regs.esp) };
    let floor = regs.esp;
    {
        // SAFETY: this thread's engine, used only inside this block.
        let uc = unsafe { &mut (*e).uc };
        sync(uc);
        put_regs(uc, regs)?;
    }
    loop {
        let (target, esp) = {
            // SAFETY: as above.
            let uc = unsafe { &mut (*e).uc };
            sync(uc);
            uc.get_data_mut().fault = None;
            let r = uc.emu_start(u64::from(pc), 0, 0, 0);
            let fault = uc.get_data_mut().fault.take();
            match fault {
                Some(Fault::Fetch(t)) => (t, uc.reg_read(RegisterX86::ESP)? as u32),
                other => {
                    let at = uc.reg_read(RegisterX86::EIP).unwrap_or(0) as u32;
                    return Err(Error::Emulation {
                        pc: at,
                        err: r.err(),
                        detail: match other {
                            Some(Fault::Data(a, ty)) => format!(", data {ty:?} at {a:#010x}"),
                            _ => String::new(),
                        },
                    });
                }
            }
        };
        on_stop(target);
        if target == ret_addr && esp > floor {
            // SAFETY: as above.
            return get_regs(unsafe { &mut (*e).uc });
        }
        // SAFETY: as above.
        let mut r = get_regs(unsafe { &mut (*e).uc })?;
        // SAFETY: the overlay code's stack: its return address.
        let ret = unsafe { crt::read_u32(cfg, esp) };
        if let Some(key) = crt::trap_name(cfg, target) {
            crt::stand_in(host, cfg, &key, &mut r)?;
        } else if target == cfg.service + SENTINEL_OFFSET {
            return Err(Error::Emulation {
                pc: target,
                err: None,
                detail: ", returned to a sentinel it wasn't called from".into(),
            });
        } else {
            host.call_out(target, slot_of(cfg, target), &mut r)
                .map_err(Error::Host)?;
        }
        {
            // SAFETY: as above.
            let uc = unsafe { &mut (*e).uc };
            put_regs(uc, &r)?;
            if !in_text(uc, ret) {
                if ret == ret_addr && r.esp > floor {
                    on_stop(ret);
                    return get_regs(uc);
                }
                return Err(Error::Emulation {
                    pc: ret,
                    err: None,
                    detail: format!(", returned outside overlay code (from {target:#010x})"),
                });
            }
        }
        pc = ret;
    }
}

/// A call from a stand-in back into overlay code (`_initterm`'s
/// initializers): on the overlay's stack below `at.esp`, returning to the
/// service range's sentinel.
pub(crate) fn call_nested(host: &mut impl Host, f: u32, at: &Regs) -> Result<Regs, Error> {
    let cfg = config()?;
    let mut r = *at;
    r.st_len = 0;
    r.esp = (at.esp - 0x40) & !0xf;
    // SAFETY: the overlay's own stack, below its current top.
    unsafe { crt::write_u32(cfg, r.esp, cfg.service + SENTINEL_OFFSET) };
    let out = call(host, f, &r)?;
    Ok(out)
}

/// [`crate::OverlayFn::call`]: `args` pushed below `stack_top`, then a
/// return address in the service range.
pub(crate) fn call_rust(
    host: &mut impl Host,
    f: u32,
    stack_top: u32,
    ecx: u32,
    args: &[u32],
) -> Result<Regs, Error> {
    let cfg = config()?;
    let mut sp = (stack_top & !0xf) - 4 * args.len() as u32;
    for (i, a) in (0u32..).zip(args) {
        // SAFETY: the caller's stack room.
        unsafe { crt::write_u32(cfg, sp + 4 * i, *a) };
    }
    sp -= 4;
    // SAFETY: as above.
    unsafe { crt::write_u32(cfg, sp, cfg.service + SENTINEL_OFFSET) };
    let mut r = Regs::at(sp);
    r.ecx = ecx;
    call(host, f, &r)
}
