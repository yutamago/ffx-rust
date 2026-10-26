//! List p6-render-c, continuation round 2 (batch 14): owned-array
//! destructors, the material's buffer size count, the renderer's
//! initialisation and the application's input devices.
//!
//! Conventions are read from the disassembly; `eax` is returned wherever
//! the original defines it.

use ffx_graphic::host::{Conv, Host};

/// `Phyre_PhyreFree(p)` (cdecl) and the records' release
/// `0x005c1ef0(records, n)` (cdecl).
pub const FREE: u32 = 0x0042_fb20;
pub const FREE_RECORDS: u32 = 0x005c_1ef0;

/// How a destructor releases an owned `PArray` (count word, then data).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Owned {
    /// `PhyreFree(data)` when there is data.
    Free,
    /// `0x005c1ef0(data, count)`.
    Records,
}

/// The owned-array destructors: (address, arrays by their count offset).
pub const ARRAY_DTORS: &[(u32, &[(u32, Owned)])] = &[
    (
        0x005b_f7d0,
        &[
            (0x38, Owned::Free),
            (0x18, Owned::Free),
            (0x10, Owned::Free),
            (4, Owned::Records),
        ],
    ),
    (0x00a2_bbd0, &[(0x18, Owned::Free), (0x10, Owned::Free)]),
];

/// The arrays of the destructor at `addr` ([`ARRAY_DTORS`]).
#[must_use]
pub const fn array_dtor(addr: u32) -> &'static [(u32, Owned)] {
    let mut i = 0;
    while i < ARRAY_DTORS.len() {
        if ARRAY_DTORS[i].0 == addr {
            return ARRAY_DTORS[i].1;
        }
        i += 1;
    }
    panic!("no such destructor")
}

/// `PRendererBase::initialize`'s callees: the lock guard (thiscall ctor
/// and dtor on a frame word), the render interface singleton (`0x00c94f00`,
/// its guard bit `0x00c9628c`, constructor `0x004fee70`, `atexit`
/// destructor `0x00afb710`) and its `getViewportSize(&w, &h, &out)`
/// (thiscall), the semaphore's `initialize` and the rings' set-up.
pub const GUARD_CTOR: u32 = 0x0046_3d50;
pub const GUARD_DTOR: u32 = 0x0046_3e90;
pub const INTERFACE: u32 = 0x00c9_4f00;
pub const INTERFACE_GUARD: u32 = 0x00c9_628c;
pub const INTERFACE_CTOR: u32 = 0x004f_ee70;
pub const INTERFACE_ATEXIT: u32 = 0x00af_b710;
pub const ATEXIT: u32 = 0x0094_9207;
pub const VIEWPORT_SIZE: u32 = 0x0059_4f40;
pub const SEMAPHORE_INIT: u32 = 0x0043_0080;
pub const INIT_RINGS: u32 = 0x005b_8e80;

/// `0x00629870`'s callees: the input device's creation `0x00623c90(0, 0)`
/// (cdecl), the application's `0x00626aa0(this)` (thiscall), and the
/// input list's head.
pub const CREATE_DEVICE: u32 = 0x0062_3c90;
pub const APP_AFTER_DEVICE: u32 = 0x0062_6aa0;
pub const INPUT_LIST: u32 = 0x00c3_0fa8;
/// The device's five mappings (offset in the device, kind).
pub const MAPPINGS: [(u32, u32); 5] = [(0x18, 1), (0x48, 2), (0x78, 0xb), (0xa8, 0xc), (0xd8, 0xd)];

/// The morph weights' lookup `0x005b2c70(list; segment)` (thiscall).
pub const WEIGHTS_OF: u32 = 0x005b_2c70;

pub mod bridge {
    //! The originals over game memory.

    use super::WEIGHTS_OF;
    use super::{
        APP_AFTER_DEVICE, ATEXIT, CREATE_DEVICE, Conv, FREE, FREE_RECORDS, GUARD_CTOR, GUARD_DTOR,
        Host, INIT_RINGS, INPUT_LIST, INTERFACE, INTERFACE_ATEXIT, INTERFACE_CTOR, INTERFACE_GUARD,
        MAPPINGS, Owned, SEMAPHORE_INIT, VIEWPORT_SIZE,
    };
    use ffx_bridge::x87;

    /// An owned-array destructor (`this`): each `PArray` (count at
    /// `off`, bit 31 a view; data at `off + 4`) released when owned and
    /// cleared (data, then count). `eax`: the last array's: 0xfe for a
    /// view, 0 for no data, else the release's.
    pub fn array_dtor(h: &mut impl Host, this: u32, arrays: &[(u32, Owned)]) -> u32 {
        let mut eax = 0;
        for &(off, how) in arrays {
            let count: u32 = h.read(this + off);
            if count >> 31 != 0 {
                eax = 0xfe;
            } else {
                let data: u32 = h.read(this + off + 4);
                eax = match how {
                    Owned::Free if data == 0 => 0,
                    Owned::Free => h.call_sym(FREE, Conv::Cdecl, &[data]),
                    Owned::Records => {
                        h.call_sym(FREE_RECORDS, Conv::Cdecl, &[data, count & 0x7fff_ffff])
                    }
                };
            }
            h.write(this + off + 4, 0u32);
            h.write(this + off, 0u32);
        }
        eax
    }

    /// `0x005c6680(material; ctx)` (thiscall, `ret 4`): the material's
    /// buffer size: the header (`2 n + 0x33` rounded down to 16, `n` the
    /// parameters `+4`), then each parameter (20-byte entries at `[+8]`;
    /// `ctx+0xc` = its index): with a sizer at `+0x1c`, 16 more and its
    /// `(0, ctx, &tmp)` (cdecl; `tmp` 16 bytes in the frame, `frame` the
    /// original's `ebp - 0x14`), else `+0x20`'s `(0, ctx)`, each result
    /// rounded up to 16; plus 4 per word of `+0x18`.
    pub fn material_size(h: &mut impl Host, material: u32, frame: u32, ctx: u32) -> u32 {
        let n = h.read::<u32>(material + 4) & 0x7fff_ffff;
        let words = h.read::<u32>(material + 0x18) & 0x7fff_ffff;
        let mut size = n.wrapping_mul(2).wrapping_add(0x33) & !0xf;
        for i in 0..n {
            h.write(ctx + 0xc, i);
            let params: u32 = h.read(material + 8);
            let e: u32 = h.read(params.wrapping_add(i.wrapping_mul(0x14)));
            let sizer: u32 = h.read(e + 0x1c);
            let r = if sizer != 0 {
                size = size.wrapping_add(0x10);
                h.call(sizer, Conv::Cdecl, &[0, ctx, frame])
            } else {
                let f: u32 = h.read(e + 0x20);
                if f == 0 {
                    0
                } else {
                    h.call(f, Conv::Cdecl, &[0, ctx])
                }
            };
            size = size.wrapping_add(r.wrapping_add(0xf) & !0xf);
        }
        size.wrapping_add(words.wrapping_mul(4))
    }

    /// `PRendererBase::initialize(this; base, size, command_size)`
    /// (`0x005b8da0`, `ret 0xc`): under the lock guard (a frame word,
    /// `frame + 4`, the original's `ebp - 0x10`), the render interface
    /// singleton (built on first use) asked the viewport size into
    /// `+0x10`/`+0x14` (its third word through `frame`, `ebp - 0x14`);
    /// then the semaphore `+0xe0` (16/16; byte `+0xe4` set) and, when it
    /// is up, the rings over `base` (`size` rounded down to 16 for the
    /// data, the rest for commands, 16 segments). `eax`: the rings' or
    /// the semaphore's result.
    pub fn renderer_initialize(
        h: &mut impl Host,
        this: u32,
        frame: u32,
        base: u32,
        size: u32,
        command_size: u32,
    ) -> u32 {
        let lock = frame + 4;
        h.call_sym(GUARD_CTOR, Conv::Thiscall(lock), &[]);
        let guard = h.rebase(INTERFACE_GUARD);
        let g: u32 = h.read(guard);
        let iface = h.rebase(INTERFACE);
        if g & 1 == 0 {
            h.write(guard, g | 1);
            h.call_sym(INTERFACE_CTOR, Conv::Thiscall(iface), &[]);
            let f = h.rebase(INTERFACE_ATEXIT);
            h.call_sym(ATEXIT, Conv::Cdecl, &[f]);
        }
        h.call_sym(
            VIEWPORT_SIZE,
            Conv::Thiscall(iface),
            &[this + 0x10, this + 0x14, frame],
        );
        h.call_sym(GUARD_DTOR, Conv::Thiscall(lock), &[]);
        let data = size & !0xf;
        let sem = this + 0xe0;
        let r = h.call_sym(SEMAPHORE_INIT, Conv::Thiscall(sem), &[0x10, 0x10]);
        h.write(sem + 4, 1u8);
        if r != 0 {
            return r;
        }
        h.write(this + 0x44, sem);
        h.call_sym(
            INIT_RINGS,
            Conv::Thiscall(this),
            &[base, base.wrapping_add(data), data, command_size, 0x10],
        )
    }

    /// `0x00629870(app)` (thiscall): the input device (`0x00623c90(0,
    /// 0)`, at `+0x9c`): its flag `+0x108` from the byte `+0xa6`, its five
    /// mappings (`+4..+0x14`, 0x30 bytes apart from `+0x18`) set up
    /// (virtual slot 2: `(mapping, kind)`), or the five cleared without
    /// one; `0x00626aa0(app)`; then for each of 7 device kinds `k` the
    /// `k`-th input of the list whose slot 1 answers 1 (or 0) at `+0x80 +
    /// 4 k`. `eax`: `app + 0x9c`.
    pub fn setup_inputs(h: &mut impl Host, app: u32) -> u32 {
        let dev = h.call_sym(CREATE_DEVICE, Conv::Cdecl, &[0, 0]);
        h.write(app + 0x9c, dev);
        if dev == 0 {
            for off in [0x14, 0x10, 0xc, 8, 4] {
                h.write(app + off, 0u32);
            }
        } else {
            let b: u8 = h.read(app + 0xa6);
            h.write(dev + 0x108, b);
            let dev: u32 = h.read(app + 0x9c);
            for (k, &(off, _)) in MAPPINGS.iter().enumerate() {
                h.write(app + 4 + 4 * k as u32, dev.wrapping_add(off));
            }
            for (k, &(_, kind)) in MAPPINGS.iter().enumerate() {
                let d: u32 = h.read(app + 0x9c);
                let m: u32 = h.read(app + 4 + 4 * k as u32);
                h.call_virtual(d, 2, &[m, kind]);
            }
        }
        h.call_sym(APP_AFTER_DEVICE, Conv::Thiscall(app), &[]);
        let head = h.rebase(INPUT_LIST);
        let mut slot = app + 0x80;
        for k in 0..7u32 {
            let mut found = 0;
            let mut seen = 0u32;
            let mut link: u32 = h.read(head);
            while link != head && link != 0 && link != 4 {
                let input = link - 4;
                if h.call_virtual(input, 1, &[]) == 1 {
                    if seen == k {
                        found = input;
                        break;
                    }
                    seen += 1;
                }
                link = h.read(input + 4);
            }
            h.write(slot, found);
            slot += 4;
        }
        slot
    }

    /// `0x005cb750(out, ctx)` (cdecl): the morph weights of the context's
    /// segment (`ctx+0xc`) of the modifier (`[[ctx+8]]`; its segments
    /// `[+8]`, 20 bytes, count `+4`): 0 without one. With `out`, each of
    /// the segment's `n` targets (`[seg+8]`) gets 1.0 unless its entry
    /// (8 bytes from the segment record's `+0x10`) is negative: then the
    /// weight of its record (`[+0x14] + 20 id`, id below `+0x10`'s count;
    /// else record 0) from the weights array (`0x005b2c70`; 0.0 past its
    /// end), each through `fld`/`fstp`; no weights give all 1.0. Returns
    /// `4 n`.
    pub fn morph_weights(h: &mut impl Host, out: u32, ctx: u32) -> u32 {
        let list: u32 = h.read(ctx + 8);
        let seg: u32 = h.read(ctx + 0xc);
        let mw: u32 = h.read(list);
        if mw == 0 {
            return 0;
        }
        if seg >= h.read::<u32>(mw + 4) & 0x7fff_ffff {
            return 0;
        }
        let segoff = seg.wrapping_mul(20);
        let segs: u32 = h.read(mw + 8);
        let record: u32 = h.read(segs.wrapping_add(segoff));
        if record == 0 {
            return 0;
        }
        let n = h.read::<u32>(record + 8) & 0x7fff_ffff;
        if out == 0 {
            return n.wrapping_mul(4);
        }
        let weights = h.call_sym(WEIGHTS_OF, Conv::Thiscall(list), &[seg]);
        if weights == 0 {
            for k in 0..n {
                h.write(out.wrapping_add(4 * k), 1.0f32);
            }
            return n.wrapping_mul(4);
        }
        for k in 0..n {
            let segs: u32 = h.read(mw + 8);
            let entries: u32 = h.read(segs.wrapping_add(segoff).wrapping_add(0x10));
            let e = entries.wrapping_add(k.wrapping_mul(8));
            let mut v = 1.0f32;
            if h.read::<i32>(e) < 0 {
                let id: u32 = h.read(e + 4);
                let rec = if id < h.read::<u32>(mw + 0x10) & 0x7fff_ffff {
                    h.read::<u32>(mw + 0x14).wrapping_add(id.wrapping_mul(20))
                } else {
                    0
                };
                let r: u32 = h.read(rec.wrapping_add(0x10));
                v = if r < h.read::<u32>(weights) & 0x7fff_ffff {
                    let data: u32 = h.read(weights + 4);
                    x87::round_trip(h.read(data.wrapping_add(r.wrapping_mul(4))))
                } else {
                    0.0
                };
            }
            h.write(out.wrapping_add(4 * k), x87::round_trip(v));
        }
        n.wrapping_mul(4)
    }
}
