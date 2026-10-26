//! List p6-render-c, continuation round 4: `PRendererBase::
//! renderMeshInstance` (`0x005b9a90`), the per-instance draw the render
//! chain's mesh lists run.
//!
//! Read from the disassembly (Ghidra says `void`; the result is in `eax`
//! and `renderMeshInstanceList` reads it).

use ffx_bridge::x87;
use ffx_graphic::host::{Conv, Host};

/// The preprocess ring (`GetSingleton()` cdecl, `allocate(ring; bytes)`
/// thiscall) for matrix arrays above 0x80 entries.
pub const RING_SINGLETON: u32 = 0x005c_d010;
pub const RING_ALLOCATE: u32 = 0x005c_d0f0;
/// The instance's joint matrices and skin upload (thiscall on the
/// instance).
pub const COMPUTE_JOINTS: u32 = 0x004f_6ef0;
pub const UPLOAD_SKIN: u32 = 0x004f_98b0;
/// The draw context's constructor `0x005b44c0(ctx; a, b, c, d, e)`, the
/// command header's `0x005b4560(obj)` (thiscall).
pub const CONTEXT_CTOR: u32 = 0x005b_44c0;
pub const HEADER_CTOR: u32 = 0x005b_4560;
/// The 4x4 inverse (cdecl `(out, m)`) and copy (thiscall `(out; m)`).
pub const INVERSE: u32 = 0x004c_c0c0;
pub const COPY: u32 = 0x004c_9430;
/// The per-segment steps (thiscall on the renderer unless said):
/// `selectTechniquePass` (8 words), `meshInstanceSizes` (cdecl, 9
/// words), `hasRoom(bytes, commands)` (`al`), the flush (virtual slot
/// 2), `allocateAndFillParameterBuffers` (8 words), the data ring's
/// `allocate(+0x1c; bytes)` and `queueMeshCommands` (8 words).
pub const SELECT_PASS: u32 = 0x005b_a690;
pub const INSTANCE_SIZES: u32 = 0x005b_5150;
pub const HAS_ROOM: u32 = 0x005b_6a70;
pub const FILL_BUFFERS: u32 = 0x005b_5a50;
pub const DATA_ALLOCATE: u32 = 0x0061_3e10;
pub const QUEUE_COMMANDS: u32 = 0x005b_a380;
/// The effect and context words that skip a segment together.
pub const SKIP_EFFECT: u32 = 0x00c1_69b4;
pub const SKIP_CONTEXT: u32 = 0x00c1_69d4;

/// The original's frame from `ebp - 0x188` (`FRAME` bytes): every local
/// it writes, at its `ebp` offset (`at(-0x12c)` is `ebp - 0x12c`).
pub const FRAME: u32 = 0x178;
#[must_use]
pub const fn at(frame: u32, off: i32) -> u32 {
    frame.wrapping_add((0x188 + off) as u32)
}

/// The matrix array the original takes with `_alloca` for `n` matrices
/// (16-byte aligned, `n <= 0x80`): its base, from the shim or the test.
pub trait Alloca {
    fn alloca(&mut self, n: u32) -> u32;
}

impl<F: FnMut(u32) -> u32> Alloca for F {
    fn alloca(&mut self, n: u32) -> u32 {
        self(n)
    }
}

pub mod bridge {
    use super::{
        Alloca, COMPUTE_JOINTS, CONTEXT_CTOR, COPY, Conv, DATA_ALLOCATE, FILL_BUFFERS, HAS_ROOM,
        HEADER_CTOR, Host, INSTANCE_SIZES, INVERSE, QUEUE_COMMANDS, RING_ALLOCATE, RING_SINGLETON,
        SELECT_PASS, SKIP_CONTEXT, SKIP_EFFECT, UPLOAD_SKIN, at, x87,
    };

    /// `PRendererBase::renderMeshInstance(renderer; instance, contexts,
    /// x, y)` (thiscall, `ret 0x10`) -> 0 or an error: 0x16 when the
    /// renderer is not in a frame (bit 0 of `+4`), 0x15 without an effect
    /// (`+0x48`), 1 without contexts (a `PSharray`), 7 for a segment
    /// whose context is missing, or a callee's.
    ///
    /// `x` (0: the instance's `+0x14`) goes into the draw context, `y`
    /// into the instance's `+0x80` (a change re-uploads its skin). The
    /// joint matrices (`[mesh+0x18]` + 1) go in `alloca`'s array (up to
    /// 0x80) or the preprocess ring (released after, `+4 = 0`). Then for
    /// passes 0..=3, each segment of that pass (`+0x1c`): its context
    /// (by `+0`), the inverse of its matrix (`+4`) copied into the draw
    /// context, the technique pass, and unless none (or the segment's
    /// byte `+0x14`) the sizes, room (else flush), the parameter
    /// buffers, the data and the commands.
    #[allow(clippy::too_many_lines)]
    pub fn render_mesh_instance(
        h: &mut impl Host,
        r: u32,
        frame: u32,
        args: [u32; 4],
        alloca: &mut impl Alloca,
    ) -> u32 {
        let [inst, contexts, x, y] = args;
        let l = |off: i32| at(frame, off);
        h.write(l(-0x44), r);
        if h.read::<u32>(r.wrapping_add(4)) & 1 == 0 {
            return 0x16;
        }
        let effect: u32 = h.read(r.wrapping_add(0x48));
        h.write(l(-0x3c), effect);
        if effect == 0 {
            return 0x15;
        }
        let x = if x == 0 {
            h.read(inst.wrapping_add(0x14))
        } else {
            x
        };
        let n = h.read::<u32>(contexts) & 0x7fff_ffff;
        let list = if n > 1 {
            h.read(contexts.wrapping_add(4))
        } else if n != 0 {
            contexts.wrapping_add(4)
        } else {
            0
        };
        let mesh: u32 = h.read(inst);
        let extra: u32 = h.read(inst.wrapping_add(0x18));
        h.write(l(-0x28), list);
        h.write(l(-0x18), mesh);
        h.write(l(-0x54), extra);
        if list == 0 {
            return 1;
        }
        let count = (h.read::<u32>(mesh.wrapping_add(0x18)) & 0x7fff_ffff).wrapping_add(1);
        h.write(l(-0x34), 0u32);
        let mut block = 0;
        let base = if count > 0x80 {
            let ring = h.call_sym(RING_SINGLETON, Conv::Cdecl, &[]);
            block = h.call_sym(RING_ALLOCATE, Conv::Thiscall(ring), &[count << 6]);
            h.write(l(-0x34), block);
            block.wrapping_add(0x10)
        } else {
            alloca.alloca(count)
        };
        h.write(l(-0xbc), block);
        let mats = base.wrapping_add(0x40);
        h.write(l(-0x20), mats);
        h.call_sym(COMPUTE_JOINTS, Conv::Thiscall(inst), &[mats]);
        for off in [-0x78, -0x74, -0x70, -0x6c, -0x68] {
            h.write(l(off), 0u32);
        }
        let ctx = l(-0x12c);
        let a: u32 = h.read(r.wrapping_add(0x5c));
        h.call_sym(
            CONTEXT_CTOR,
            Conv::Thiscall(ctx),
            &[a, r.wrapping_add(0x60), r.wrapping_add(0xa0), inst, mats],
        );
        h.write(l(-0x114), x);
        let e = run(h, r, frame, inst, y, mats);
        if block != 0 {
            h.write(block.wrapping_add(4), 0u32);
        }
        e
    }

    /// The skin upload and the passes; the caller releases the ring
    /// block.
    #[allow(clippy::too_many_lines)]
    fn run(h: &mut impl Host, r: u32, frame: u32, inst: u32, y: u32, mats: u32) -> u32 {
        let l = |off: i32| at(frame, off);
        if h.read::<u32>(inst.wrapping_add(0x80)) != y {
            h.write(inst.wrapping_add(0x80), y);
            let e = h.call_sym(UPLOAD_SKIN, Conv::Thiscall(inst), &[mats]);
            if e != 0 {
                return e;
            }
        }
        let mesh: u32 = h.read(l(-0x18));
        let list: u32 = h.read(l(-0x28));
        let effect: u32 = h.read(l(-0x3c));
        let extra: u32 = h.read(l(-0x54));
        let count = h.read::<u32>(mesh) & 0x7fff_ffff;
        h.write(l(-0x58), count);
        let skip_effect = h.rebase(SKIP_EFFECT);
        let skip_context = h.rebase(SKIP_CONTEXT);
        h.write(l(-0x1c), 0u32);
        for pass in 0..=3u32 {
            h.write(l(-0x14), 0u32);
            if count == 0 {
                h.write(l(-0x1c), pass + 1);
                continue;
            }
            h.write(l(-0x24), 0u32);
            h.write(l(-0x2c), 0u32);
            for i in 0..count {
                let data = h
                    .read::<u32>(inst.wrapping_add(0x28))
                    .wrapping_add(0x24 * i);
                let seg = h.read::<u32>(mesh.wrapping_add(4)).wrapping_add(0x6c * i);
                if h.read::<u32>(seg.wrapping_add(0x1c)) == pass {
                    let index: u32 = h.read(seg);
                    let ctx: u32 = h.read(list.wrapping_add(index.wrapping_mul(4)));
                    if ctx == 0 {
                        return 7;
                    }
                    let mat: u32 = h.read(ctx);
                    h.write(l(-0x30), mat);
                    let skip = effect == skip_effect
                        && h.read::<u32>(ctx.wrapping_add(8)) == skip_context
                        && h.read::<u32>(ctx.wrapping_add(0xc)) != skip_context;
                    if !skip {
                        let e = draw(h, r, frame, [inst, data, seg, ctx, mat, i], mats, extra);
                        if e != 0 {
                            return e;
                        }
                    }
                }
                h.write(l(-0x2c), 0x24 * (i + 1));
                h.write(l(-0x24), 0x6c * (i + 1));
                h.write(l(-0x14), i + 1);
            }
            h.write(l(-0x1c), pass + 1);
        }
        0
    }

    /// One segment's draw (`refs`: instance, its segment data, the
    /// segment, its context, material, index).
    fn draw(h: &mut impl Host, r: u32, frame: u32, refs: [u32; 6], mats: u32, extra: u32) -> u32 {
        let [inst, data, seg, ctx, mat, i] = refs;
        let l = |off: i32| at(frame, off);
        let index: u32 = h.read(seg.wrapping_add(4));
        let m = mats.wrapping_add(index << 6);
        h.write(l(-0x108), m);
        h.write(l(-0x110), seg);
        h.write(l(-0x10c), i);
        let inv = h.call_sym(INVERSE, Conv::Cdecl, &[l(-0x188), m]);
        h.call_sym(COPY, Conv::Thiscall(l(-0xb8)), &[inv]);
        for k in 0..16u32 {
            let v: f32 = h.read(l(-0xb8) + 4 * k);
            h.write(l(-0x104) + 4 * k, x87::round_trip(v));
        }
        let own = if h.read::<u32>(data.wrapping_add(0x18)) == mat {
            data
        } else {
            l(-0x78)
        };
        h.write(l(-0x40), own);
        let effect: u32 = h.read(l(-0x3c));
        h.call_sym(
            SELECT_PASS,
            Conv::Thiscall(r),
            &[l(-0xc4), l(-0x78), inst, mat, ctx, effect, l(-0x12c), data],
        );
        let pass: u32 = h.read(l(-0xc0));
        h.write(l(-0x64), pass);
        if pass == 0 || h.read::<u8>(seg.wrapping_add(0x14)) != 0 {
            return 0;
        }
        let technique: u32 = h.read(pass);
        let commands = h.read::<u32>(pass.wrapping_add(4)) & 0x7fff_ffff;
        let shared: u32 = h.read(ctx.wrapping_add(4));
        h.write(l(-0x60), technique);
        h.write(l(-0x5c), shared);
        for off in [-0x4c, -0x38, -0x48] {
            h.write(l(off), 0u32);
        }
        h.call_sym(
            INSTANCE_SIZES,
            Conv::Cdecl,
            &[
                l(-0x4c),
                l(-0x38),
                l(-0x48),
                technique,
                shared,
                mat,
                pass,
                i,
                extra,
            ],
        );
        let bytes: u32 = h.read(l(-0x4c));
        if h.call_sym(HAS_ROOM, Conv::Thiscall(r), &[bytes, commands]) as u8 == 0 {
            h.call_virtual(r, 2, &[]);
        }
        h.call_sym(HEADER_CTOR, Conv::Thiscall(l(-0x148)), &[]);
        h.write(l(-0x50), 0u32);
        let own: u32 = h.read(l(-0x40));
        let parameters: u32 = h.read(l(-0x38));
        let e = h.call_sym(
            FILL_BUFFERS,
            Conv::Thiscall(r),
            &[
                l(-0x148),
                technique,
                shared,
                l(-0x12c),
                l(-0x50),
                own,
                parameters,
                extra,
            ],
        );
        if e != 0 {
            return e;
        }
        let data_bytes: u32 = h.read(l(-0x48));
        let a = h.call_sym(
            DATA_ALLOCATE,
            Conv::Thiscall(r.wrapping_add(0x1c)),
            &[data_bytes],
        );
        let words: u32 = h.read(l(-0x50));
        h.call_sym(
            QUEUE_COMMANDS,
            Conv::Thiscall(r),
            &[mat, pass, extra, seg, i, l(-0x148), a, words],
        );
        0
    }
}
