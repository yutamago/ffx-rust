//! List `p1-load`, continuation round 3, batch 1: the D3D11 objects a
//! loaded cluster's resources create on first use: the shader programs'
//! pixel and vertex shaders and input layout, the programs' constant
//! buffer, the shader pass state's rasterizer, blend and depth-stencil
//! states, a texture's shader-resource views.
//!
//! Each runs under the render scope lock (`0x00463d50` / `0x00463e90`,
//! thiscall on a frame word) and builds the render-thread device holder
//! `0x00c94f00` on first use (guard bit 0 of `0x00c9628c`, its
//! destructor registered with `atexit`), whose `+0x6c` is the device.
//! COM methods stay calls through the object's vtable (`stdcall`, the
//! object first), `ecx` as the original leaves it (the methods never
//! read it). The originals' SEH frames only unwind the scope lock, and
//! nothing they call throws, so the ports have none; their frames are
//! taken as an argument where a callee sees a frame address (the lock,
//! the descriptions), laid out at the original's `ebp` offsets.

use ffx_graphic::host::{Conv, Host};

/// The render scope lock's enter and leave (thiscall on the lock word).
const SCOPE_ENTER: u32 = 0x0046_3d50;
const SCOPE_LEAVE: u32 = 0x0046_3e90;
/// The holder's guard word, the holder, its constructor and the
/// destructor `atexit` gets.
const HOLDER_GUARD: u32 = 0x00c9_628c;
const HOLDER: u32 = 0x00c9_4f00;
const HOLDER_CTOR: u32 = 0x004f_ee70;
const HOLDER_DTOR: u32 = 0x00af_b710;
const ATEXIT: u32 = 0x0094_9207;
/// The holder's device word (`HOLDER + 0x6c`).
const HOLDER_DEVICE: u32 = 0x00c9_4f6c;
/// The holder's default sampler words (`HOLDER + 0x1318`, 16 of them).
const HOLDER_SAMPLERS: u32 = 0x00c9_6218;
/// `PRendererContext_instance` and `GetRenderStateCache` (cdecl `()`,
/// both the holder, made on first use).
const RENDERER_CONTEXT: u32 = 0x0046_49d0;
const RENDER_STATE_CACHE: u32 = 0x004f_d8b0;
/// `PhyrePrintf(level, format, ...)`.
const PRINTF: u32 = 0x0043_52f0;
/// `WKPDID_D3DDebugObjectName`, the GUID `SetPrivateData` names with.
const NAME_GUID: u32 = 0x00b6_ae3c;
/// `PArray<u8>::resize(n)` (thiscall).
const BYTES_RESIZE: u32 = 0x0044_a010;
/// The ports' own originals, called at their addresses.
pub const CONSTANT_BUFFER_CREATE: u32 = 0x0058_f490;
pub const INPUT_LAYOUT_CREATE: u32 = 0x0058_d900;
pub const CUBE_VIEWS: u32 = 0x0058_f9f0;

/// `PhyreEngine`'s error codes these return.
pub mod error {
    /// The device refused the constant buffer: 5.
    pub const BUFFER: u32 = 5;
    /// The device refused an object: 9.
    pub const CREATE: u32 = 9;
}

/// A COM method: `slot` (word index) of `obj`'s vtable, `stdcall` with
/// `obj` first, `ecx` as the original leaves it.
fn com(h: &mut impl Host, ecx: u32, obj: u32, slot: u32, args: &[u32]) -> u32 {
    let vt: u32 = h.read(obj);
    let f: u32 = h.read(vt.wrapping_add(4 * slot));
    let mut words = Vec::with_capacity(args.len() + 1);
    words.push(obj);
    words.extend_from_slice(args);
    h.call(f, Conv::Thiscall(ecx), &words) // call width: 6 (five arguments at most)
}

/// `obj`'s debug name set: `SetPrivateData(obj, guid, len, name)`
/// (slot 5), `ecx` as the original leaves it.
fn set_name(h: &mut impl Host, ecx: u32, obj: u32, len: u32, name: u32) {
    let guid = h.rebase(NAME_GUID);
    let name = h.rebase(name);
    com(h, ecx, obj, 5, &[guid, len, name]);
}

/// The device holder built on first use (the guard's bit 0; its
/// destructor registered with `atexit`).
fn holder(h: &mut impl Host) {
    let g = h.rebase(HOLDER_GUARD);
    let v: u32 = h.read(g);
    if v & 1 == 0 {
        h.write(g, v | 1);
        let holder = h.rebase(HOLDER);
        h.call_sym(HOLDER_CTOR, Conv::Thiscall(holder), &[]);
        let dtor = h.rebase(HOLDER_DTOR);
        h.call_sym(ATEXIT, Conv::Cdecl, &[dtor]);
    }
}

fn enter(h: &mut impl Host, lock: u32) {
    h.call_sym(SCOPE_ENTER, Conv::Thiscall(lock), &[]);
}

fn leave(h: &mut impl Host, lock: u32) {
    h.call_sym(SCOPE_LEAVE, Conv::Thiscall(lock), &[]);
}

/// The holder's device word.
fn device(h: &impl Host) -> u32 {
    h.read(h.rebase(HOLDER_DEVICE))
}

/// The storage of the `PSharray` at `array` (count word, then the data
/// word or the one inline element): the data word for more than one
/// element, the inline slot for one, null for none.
fn storage(h: &impl Host, array: u32) -> u32 {
    let n = h.read::<u32>(array) & 0x7fff_ffff;
    match n {
        0 => 0,
        1 => array.wrapping_add(4),
        _ => h.read(array.wrapping_add(4)),
    }
}

/// `0x00593bd0`: the views' count at `+0x20`, at least 1.
fn view_count(h: &impl Host, this: u32) -> u32 {
    let n = h.read::<u32>(this.wrapping_add(0x20)) & 0x7fff_ffff;
    n.max(1)
}

/// A shader program kind: where its object goes, the profiles it
/// checks, its `ID3D11Device` method and its message.
struct Shader {
    out: u32,
    profile_50: u32,
    profile_41: u32,
    slot: u32,
    message: u32,
}

/// The two shader programs' first half: under the lock, when the program
/// has no shader (`this + out`) and the device exists, the profile
/// checked against the context's feature level (`+0xb4`: at least
/// 0xb000 for the 5.0 profile, 0xa100 for 4.1; else a message), then the
/// shader created from the bytecode (`+0x10`, its length `+0xc` low 31
/// bits) into `this + out`. False when there is no device (the original
/// returns 0 at once).
fn create_shader(h: &mut impl Host, this: u32, lock: u32, k: &Shader) -> bool {
    let out = this.wrapping_add(k.out);
    if h.read::<u32>(out) != 0 {
        return true;
    }
    enter(h, lock);
    holder(h);
    if device(h) == 0 {
        leave(h, lock);
        return false;
    }
    let profile: u32 = h.read(this.wrapping_add(0x4dc));
    // the 4.1 check reads the profile again (and is not reached when a
    // 5.0 profile fails)
    let ok = !((profile == k.profile_50 && level(h) < 0xb000)
        || (h.read::<u32>(this.wrapping_add(0x4dc)) == k.profile_41 && level(h) < 0xa100));
    if ok {
        let ctx = h.call_sym(RENDERER_CONTEXT, Conv::Cdecl, &[]);
        let dev: u32 = h.read(ctx.wrapping_add(0x6c));
        let vt: u32 = h.read(dev);
        let len = h.read::<u32>(this.wrapping_add(0xc)) & 0x7fff_ffff;
        let code: u32 = h.read(this.wrapping_add(0x10));
        com(h, vt, dev, k.slot, &[code, len, 0, out]);
    } else {
        let msg = h.rebase(k.message);
        h.call_sym(PRINTF, Conv::Cdecl, &[1, msg]);
    }
    leave(h, lock);
    true
}

/// The renderer context's feature level (`+0xb4`, signed).
fn level(h: &mut impl Host) -> i32 {
    let ctx = h.call_sym(RENDERER_CONTEXT, Conv::Cdecl, &[]);
    h.read(ctx.wrapping_add(0xb4))
}

/// The 16 default sampler words at `this + 0x21c` from `src`.
fn copy_samplers(h: &mut impl Host, this: u32, src: u32) {
    for i in 0..16 {
        let v: u32 = h.read(src);
        h.write(this.wrapping_add(0x21c + 4 * i), v);
    }
}

/// The ports over game memory.
pub mod bridge {
    use super::{
        BYTES_RESIZE, CONSTANT_BUFFER_CREATE, CUBE_VIEWS, Conv, HOLDER_SAMPLERS, Host,
        INPUT_LAYOUT_CREATE, PRINTF, RENDER_STATE_CACHE, RENDERER_CONTEXT, Shader, com,
        copy_samplers, create_shader, device, enter, error, holder, leave, set_name, storage,
        view_count,
    };

    /// `PShaderFragmentProgramD3D11` create (`0x0058d1c0`, thiscall):
    /// the pixel shader (`+0x4e0`, `CreatePixelShader`, device slot 15;
    /// profiles `ps50`/`ps41`), then the constant buffer
    /// (`0x0058f490`, its result dropped) and the 16 sampler words from
    /// the holder's. 0. `lock` is the original's `ebp - 0x10`.
    pub fn create_pixel_shader(h: &mut impl Host, this: u32, lock: u32) -> u32 {
        let k = Shader {
            out: 0x4e0,
            profile_50: 0x7073_3530,
            profile_41: 0x7073_3431,
            slot: 15,
            message: 0x00b3_398c,
        };
        if !create_shader(h, this, lock, &k) {
            return 0;
        }
        h.call_sym(CONSTANT_BUFFER_CREATE, Conv::Thiscall(this), &[]);
        holder(h);
        let src = h.rebase(HOLDER_SAMPLERS);
        copy_samplers(h, this, src);
        0
    }

    /// `PShaderVertexProgramD3D11` create (`0x0058d780`, thiscall): the
    /// vertex shader (`+0x4ec`, `CreateVertexShader`, device slot 12;
    /// profiles `vs50`/`vs41`), then the constant buffer
    /// (`0x0058f490`) and the input layout of the stream description at
    /// `+0x4e0` (`0x0058d900`): the first non-zero result of those, else
    /// 0 after the 16 sampler words from `GetRenderStateCache() +
    /// 0x1318`. `lock` is the original's `ebp - 0x10`.
    pub fn create_vertex_shader(h: &mut impl Host, this: u32, lock: u32) -> u32 {
        let k = Shader {
            out: 0x4ec,
            profile_50: 0x7673_3530,
            profile_41: 0x7673_3431,
            slot: 12,
            message: 0x00b3_3930,
        };
        if !create_shader(h, this, lock, &k) {
            return 0;
        }
        let r = h.call_sym(CONSTANT_BUFFER_CREATE, Conv::Thiscall(this), &[]);
        if r != 0 {
            return r;
        }
        let r = h.call_sym(
            INPUT_LAYOUT_CREATE,
            Conv::Thiscall(this.wrapping_add(0x4e0)),
            &[this],
        );
        if r != 0 {
            return r;
        }
        let cache = h.call_sym(RENDER_STATE_CACHE, Conv::Cdecl, &[]);
        copy_samplers(h, this, cache.wrapping_add(0x1318));
        0
    }

    /// The bytes of the input layout's frame below its cookie
    /// (`ebp - 0x39c` up to `ebp - 0x10`).
    pub const LAYOUT_FRAME: u32 = 0x38c;
    /// The most elements its frame holds (the original has no check: more
    /// would overwrite its cookie and end the process).
    pub const LAYOUT_ELEMENTS: u32 = 32;

    /// `PStreamInputLayoutD3D11` create (`0x0058d900`, thiscall
    /// `(layout; program)`, `ret 4`): when the layout (`+8`) is not made,
    /// under the lock, one `D3D11_INPUT_ELEMENT_DESC` (7 words at
    /// `ebp - 0x390`) per element of the array at `+0`/`+4` (0x14 bytes:
    /// the name word, colour bit cleared, skipped when null; `+8`, `+0xc`,
    /// `+0x10`; offset 0; per-instance class and step rate 1 when `+4`
    /// points at an object whose byte `+8` is set), then
    /// `CreateInputLayout` (device slot 11) with the program's bytecode
    /// (`+0x10`, length `+0xc` low 31 bits) into `ebp - 0x398`, stored at
    /// `+8` when made. 0, or 9 when the device fails or makes none.
    /// `frame` is the original's `ebp - 0x39c` (the lock, the layout
    /// word, the descriptions).
    pub fn create_input_layout(h: &mut impl Host, this: u32, program: u32, frame: u32) -> u32 {
        if h.read::<u32>(this.wrapping_add(8)) != 0 {
            return 0;
        }
        let lock = frame;
        enter(h, lock);
        let n = h.read::<u32>(this) & 0x7fff_ffff;
        let descs = frame + 0xc;
        let mut src: u32 = h.read(this.wrapping_add(4));
        let mut count = 0u32;
        for _ in 0..n {
            let name = h.read::<u32>(src) & !1;
            if name != 0 {
                let step: u32 = h.read(src.wrapping_add(4));
                let instanced = u32::from(step != 0 && h.read::<u8>(step.wrapping_add(8)) != 0);
                // the original keeps the flag in its frame word `ebp - 0x394`
                h.write(frame + 8, instanced);
                if count < super::bridge::LAYOUT_ELEMENTS {
                    let d = descs + 0x1c * count;
                    let semantic: u32 = h.read(src.wrapping_add(8));
                    let format: u32 = h.read(src.wrapping_add(0xc));
                    let slot: u32 = h.read(src.wrapping_add(0x10));
                    for (k, v) in [name, semantic, format, slot, 0, instanced, instanced]
                        .into_iter()
                        .enumerate()
                    {
                        h.write(d + 4 * k as u32, v);
                    }
                }
                count += 1;
            }
            src = src.wrapping_add(0x14);
        }
        let out = frame + 4;
        h.write(out, 0u32);
        holder(h);
        let dev = device(h);
        let vt: u32 = h.read(dev);
        let len = h.read::<u32>(program.wrapping_add(0xc)) & 0x7fff_ffff;
        let code: u32 = h.read(program.wrapping_add(0x10));
        let hr = com(h, vt, dev, 11, &[descs, count, code, len, out]);
        let layout: u32 = h.read(out);
        if (hr as i32) < 0 || layout == 0 {
            leave(h, lock);
            return error::CREATE;
        }
        h.write(this.wrapping_add(8), layout);
        leave(h, lock);
        0
    }

    /// The bytes of the constant buffer's frame below its cookie
    /// (`ebp - 0x2c` up to `ebp - 0x10`).
    pub const BUFFER_FRAME: u32 = 0x1c;

    /// `PShaderProgramConstantBuffer` create (`0x0058f490`, thiscall):
    /// when there is no buffer (`+0`) and a size (`+0x14`), under the
    /// lock, `CreateBuffer` (device slot 3) with a description at `ebp -
    /// 0x28` (the size, default usage, constant-buffer binding) into
    /// `+0`, named, and the shadow bytes at `+4` resized to the size.
    /// 0, or 5 when the device fails (with a message). `frame` is the
    /// original's `ebp - 0x2c` (the lock, the description).
    pub fn create_constant_buffer(h: &mut impl Host, this: u32, frame: u32) -> u32 {
        if h.read::<u32>(this) != 0 || h.read::<u32>(this.wrapping_add(0x14)) == 0 {
            return 0;
        }
        let lock = frame;
        enter(h, lock);
        let desc = frame + 4;
        let size: u32 = h.read(this.wrapping_add(0x14));
        for (k, v) in [size, 0, 4, 0, 0, 0].into_iter().enumerate() {
            h.write(desc + 4 * k as u32, v);
        }
        holder(h);
        let dev = device(h);
        let vt: u32 = h.read(dev);
        if (com(h, vt, dev, 3, &[desc, 0, this]) as i32) < 0 {
            let msg = h.rebase(0x00b3_3758);
            h.call_sym(PRINTF, Conv::Cdecl, &[1, msg]);
            leave(h, lock);
            return error::BUFFER;
        }
        let buffer: u32 = h.read(this);
        let vt: u32 = h.read(buffer);
        set_name(h, vt, buffer, 0x1c, 0x00b3_3778);
        let size: u32 = h.read(this.wrapping_add(0x14));
        h.call_sym(BYTES_RESIZE, Conv::Thiscall(this.wrapping_add(4)), &[size]);
        leave(h, lock);
        0
    }

    /// The bytes of the shader pass state's frame below its cookie
    /// (`ebp - 0x40` up to `ebp - 0x10`).
    pub const STATE_FRAME: u32 = 0x30;

    /// `PShaderPassStateD3D11` create (`0x0058d4e0`, thiscall): each
    /// state not made yet, under the lock: the rasterizer state (`+0x168`,
    /// device slot 22, from the description at `+4`; named), its
    /// wireframe copy (`+0x16c`: the description copied to `ebp - 0x38`
    /// with fill mode 2; not named), the blend state (`+0x174`, slot 20,
    /// from `+0x60`; named), the depth-stencil state (`+0x170`, slot 21
    /// of the context's device, from `+0x2c`; named, under a second lock
    /// word). 0, or 9 at the first the device fails or makes none.
    /// `frame` is the original's `ebp - 0x40` (the two lock words, the
    /// copy).
    pub fn create_pass_state(h: &mut impl Host, this: u32, frame: u32) -> u32 {
        let lock = frame + 4;
        let solid = this.wrapping_add(0x168);
        if h.read::<u32>(solid) == 0 {
            enter(h, lock);
            holder(h);
            let dev = device(h);
            let hr = com(h, dev, dev, 22, &[this.wrapping_add(4), solid]);
            let obj: u32 = h.read(solid);
            if (hr as i32) < 0 || obj == 0 {
                leave(h, lock);
                return error::CREATE;
            }
            set_name(h, obj, obj, 0xb, 0x00b3_3654);
            leave(h, lock);
        }
        let wire = this.wrapping_add(0x16c);
        if h.read::<u32>(wire) == 0 {
            enter(h, lock);
            let desc = frame + 8;
            for k in 0..10 {
                let v: u32 = h.read(this.wrapping_add(4 + 4 * k));
                h.write(desc + 4 * k, v);
            }
            h.write(desc, 2u32);
            holder(h);
            let dev = device(h);
            let vt: u32 = h.read(dev);
            let hr = com(h, vt, dev, 22, &[desc, wire]);
            if (hr as i32) < 0 || h.read::<u32>(wire) == 0 {
                leave(h, lock);
                return error::CREATE;
            }
            leave(h, lock);
        }
        let blend = this.wrapping_add(0x174);
        if h.read::<u32>(blend) == 0 {
            enter(h, lock);
            holder(h);
            let dev = device(h);
            let hr = com(h, dev, dev, 20, &[this.wrapping_add(0x60), blend]);
            let obj: u32 = h.read(blend);
            if (hr as i32) < 0 || obj == 0 {
                leave(h, lock);
                return error::CREATE;
            }
            set_name(h, obj, obj, 10, 0x00b3_3660);
            leave(h, lock);
        }
        let depth = this.wrapping_add(0x170);
        if h.read::<u32>(depth) == 0 {
            let lock = frame;
            enter(h, lock);
            let ctx = h.call_sym(RENDERER_CONTEXT, Conv::Cdecl, &[]);
            let dev: u32 = h.read(ctx.wrapping_add(0x6c));
            let hr = com(h, dev, dev, 21, &[this.wrapping_add(0x2c), depth]);
            let obj: u32 = h.read(depth);
            if (hr as i32) < 0 || obj == 0 {
                leave(h, lock);
                return error::CREATE;
            }
            set_name(h, obj, obj, 0x11, 0x00b3_366c);
            leave(h, lock);
        }
        0
    }

    /// The bytes of the views' frame below its cookie (`ebp - 0x30` up
    /// to `ebp - 0x10`).
    pub const VIEWS_FRAME: u32 = 0x20;

    /// A texture's shader-resource views (`0x0058f9f0`, thiscall): when
    /// its flags (`+0x58`) have bit 3, under the lock, one
    /// `CreateShaderResourceView` (device slot 7) per element of the
    /// array at `+0x20` (0x14 bytes: the resource word, the view at `+4`;
    /// at least one, the count asked again after each) with the
    /// description at `ebp - 0x30` (the format `[+0] + 0x20`, dimension
    /// 9, the mip count `+0x40`). 0, or 9 when one fails. `frame` is the
    /// original's `ebp - 0x30` (the description, the lock).
    pub fn create_views(h: &mut impl Host, this: u32, frame: u32) -> u32 {
        if h.read::<u8>(this.wrapping_add(0x58)) & 8 == 0 {
            return 0;
        }
        let lock = frame + 0x18;
        enter(h, lock);
        holder(h);
        let dev = device(h);
        let desc = frame;
        let base: u32 = h.read(this);
        let format: u32 = h.read(base.wrapping_add(0x20));
        let mips: u32 = h.read(this.wrapping_add(0x40));
        for (k, v) in [format, 9, 0, mips, 0, 0].into_iter().enumerate() {
            h.write(desc + 4 * k as u32, v);
        }
        let mut i = 0u32;
        let mut off = 0u32;
        while i < view_count(h, this) {
            let views = storage(h, this.wrapping_add(0x20));
            let resource: u32 = h.read(views.wrapping_add(off));
            let vt: u32 = h.read(dev);
            let hr = com(
                h,
                vt,
                dev,
                7,
                &[resource, desc, views.wrapping_add(off + 4)],
            );
            if (hr as i32) < 0 {
                leave(h, lock);
                return error::CREATE;
            }
            off = off.wrapping_add(0x14);
            i += 1;
        }
        leave(h, lock);
        0
    }

    /// The bytes of the cube map's frame below its cookie (`ebp - 0x650`
    /// up to `ebp - 0x10`).
    pub const CUBE_FRAME: u32 = 0x640;
    /// The subresource entries its frame holds (`ebp - 0x610` up to
    /// `ebp - 0x10`, 12 bytes each; the original has no check, more would
    /// overwrite its cookie and end the process).
    pub const CUBE_SUBRESOURCES: u32 = 128;

    /// `PTextureCubeMapD3D11` create (`0x00590800`, thiscall `(texture;
    /// data, views, unused, skip)`, `ret 0x10`): 5 at once when the byte
    /// `skip` is set. Else under the lock (`ebp - 0x650`) with the device
    /// (0x17 when there is none): the `D3D11_TEXTURE2D_DESC` at `ebp -
    /// 0x63c` (the size `+0x1c` square, the mip count `+0xc` plus 1, or 1
    /// when `+0x10` is 0 and `+0x14` has no bit 1; six faces, the format
    /// `[+0] + 0x18`, one sample, cube flag 4); 5 when `+0x18` points at an
    /// object whose byte `+5` is set; else the bind flags (8 when the
    /// byte `views` is set; with bit 8 of `+0x14`, 0x40 or 0x20 by bit 1
    /// of `[+0] + 0x10`) and, from `data` on, one subresource per face and
    /// mip at `ebp - 0x610` (`[+0] + 0xc` bits per texel; the size
    /// halving, 0 standing for the full size; rounded up to 4 and pitched
    /// in rows of 4 when bit 0 of `[+0] + 0x10` says block-compressed).
    /// Then `CreateTexture2D` (device slot 5, with the subresources when
    /// `data` is set) into each 0x14-byte element of the array at `+0x20`
    /// (at least one; 9 when it returns anything but 0), each one's
    /// `GetDesc` (slot 10) into `+0x38`. After the lock, with `views`,
    /// `0x0058f9f0`'s result when not 0. Else 0. `frame` is the
    /// original's `ebp - 0x650`.
    #[allow(clippy::too_many_lines)]
    pub fn create_cube_map(
        h: &mut impl Host,
        this: u32,
        data: u32,
        views: u8,
        skip: u8,
        frame: u32,
    ) -> u32 {
        let local = |off: u32| frame + 0x650 - off;
        let srgb_bind = (h.read::<u32>(this.wrapping_add(0x14)) >> 8) & 1 != 0;
        if skip != 0 {
            return 5;
        }
        let base: u32 = h.read(this);
        let lock = local(0x650);
        enter(h, lock);
        holder(h);
        let dev = device(h);
        if dev == 0 {
            leave(h, lock);
            return 0x17;
        }
        let desc = local(0x63c);
        for k in 0..11 {
            h.write(desc + 4 * k, 0u32);
        }
        let size: u32 = h.read(this.wrapping_add(0x1c));
        let mut mips = h.read::<u32>(this.wrapping_add(0xc)).wrapping_add(1);
        h.write(desc, size);
        h.write(desc + 4, size);
        h.write(desc + 8, mips);
        if h.read::<u32>(this.wrapping_add(0x10)) == 0
            && (h.read::<u32>(this.wrapping_add(0x14)) >> 1) & 1 == 0
        {
            mips = 1;
            h.write(desc + 8, mips);
        }
        h.write(desc + 0xc, 6u32);
        let format: u32 = h.read(base.wrapping_add(0x18));
        h.write(desc + 0x10, format);
        h.write(desc + 0x14, 1u32);
        h.write(desc + 0x18, 0u32);
        h.write(desc + 0x28, 4u32);
        let owner: u32 = h.read(this.wrapping_add(0x18));
        if owner != 0 && h.read::<u8>(owner.wrapping_add(5)) != 0 {
            leave(h, lock);
            return 5;
        }
        let mut bind = if views != 0 { 8u32 } else { 0 };
        if srgb_bind {
            bind |= if (h.read::<u32>(base.wrapping_add(0x10)) >> 1) & 1 != 0 {
                0x40
            } else {
                0x20
            };
        }
        h.write(desc + 0x20, bind);
        h.write(desc + 0x1c, 0u32);
        h.write(desc + 0x24, 0u32);
        // the subresources: per face, per mip
        let subs = local(0x610);
        let mut cursor = data;
        let mut entry = 0u32;
        for _ in 0..6 {
            if mips == 0 {
                continue;
            }
            let block = h.read::<u8>(base.wrapping_add(0x10)) & 1 != 0;
            let mut dim = size;
            for _ in 0..mips {
                let mut w = if dim == 0 { size } else { dim };
                if h.read::<u8>(base.wrapping_add(0x10)) & 1 != 0 {
                    w = w.wrapping_add(3) & !3;
                }
                let bits: u32 = h.read(base.wrapping_add(0xc));
                let slice = bits.wrapping_mul(w).wrapping_mul(w) >> 3;
                let (row, rows) = if block {
                    (dim.wrapping_add(3) & !3, 4u32)
                } else {
                    (dim, 1)
                };
                let pitch: u32 = h.read::<u32>(base.wrapping_add(0xc));
                let pitch = pitch.wrapping_mul(row).wrapping_mul(rows) >> 3;
                dim >>= 1;
                if entry < CUBE_SUBRESOURCES {
                    let e = subs + 0xc * entry;
                    h.write(e, cursor);
                    h.write(e + 4, pitch);
                    h.write(e + 8, 0u32);
                }
                entry += 1;
                cursor = cursor.wrapping_add(slice);
            }
        }
        let init = if data != 0 { subs } else { 0 };
        let mut i = 0u32;
        let mut off = 0u32;
        while i < view_count(h, this) {
            let textures = storage(h, this.wrapping_add(0x20));
            let vt: u32 = h.read(dev);
            let hr = com(h, vt, dev, 5, &[desc, init, textures.wrapping_add(off)]);
            if hr != 0 {
                leave(h, lock);
                return error::CREATE;
            }
            let textures = storage(h, this.wrapping_add(0x20));
            let tex: u32 = h.read(textures.wrapping_add(off));
            com(h, tex, tex, 10, &[this.wrapping_add(0x38)]);
            i += 1;
            off = off.wrapping_add(0x14);
        }
        leave(h, lock);
        if views != 0 {
            let r = h.call_sym(CUBE_VIEWS, Conv::Thiscall(this), &[]);
            if r != 0 {
                return r;
            }
        }
        0
    }
}
