//! h2-render-b, the rest (forked share): the renderer's render-target
//! binding (`0x0058c830`) and the shader program binding's texture
//! parameters (`setTextureParameters`, `0x0059b3d0`).

use ffx_graphic::host::{Conv, Host};

/// `PRenderTargetD3D11::createViews` (thiscall `(target)` -> 0 on
/// success).
const CREATE_VIEWS: u32 = 0x0059_1460;
/// `PRendererD3D11_clear_0058e3d0` (thiscall `(this; flags)`).
const CLEAR: u32 = 0x0058_e3d0;
/// `crt_memset_thunk` (cdecl `(p, value, n)`).
const MEMSET: u32 = 0x0094_964c;
/// `Phyre_PhyrePrintf` (cdecl `(level, format, ...)`).
const PRINTF: u32 = 0x0043_52f0;
/// `0x0060fdd0` (thiscall `(owner; index, entry)`): the parameter's name
/// object, or 0.
const PARAMETER_NAME: u32 = 0x0060_fdd0;

/// The messages `setTextureParameters` prints (`.rdata`).
const MSG_LOCATION: u32 = 0x00b3_3798;
const MSG_DATA_BLOCK_NAMED: u32 = 0x00b3_37c0;
const MSG_DATA_BLOCK: u32 = 0x00b3_3808;
const MSG_INDEX_BLOCK_NAMED: u32 = 0x00b3_3848;
const MSG_INDEX_BLOCK: u32 = 0x00b3_38a0;
const MSG_UNKNOWN: u32 = 0x00b3_38f0;

/// A COM method: `slot` (word index) of `obj`'s vtable, `stdcall` with
/// `obj` first, `ecx` as the original leaves it.
fn com(h: &mut impl Host, ecx: u32, obj: u32, slot: u32, args: &[u32]) -> u32 {
    let vt: u32 = h.read(obj);
    let f: u32 = h.read(vt.wrapping_add(4 * slot));
    let mut words = Vec::with_capacity(args.len() + 1);
    words.push(obj);
    words.extend_from_slice(args);
    h.call(f, Conv::Thiscall(ecx), &words) // call width: 4 (three arguments at most)
}

/// `fild` of a word and `fstp dword`: one rounding to `f32`.
#[allow(clippy::cast_possible_truncation)]
fn int_to_f32(v: i32) -> u32 {
    (f64::from(v) as f32).to_bits()
}

/// `fild`, plus 2^32 when negative (the word as unsigned), `fstp dword`.
#[allow(clippy::cast_possible_truncation)]
fn uint_to_f32(v: u32) -> u32 {
    (f64::from(v) as f32).to_bits()
}

pub mod bridge {
    //! The originals over game memory.

    use super::{
        CLEAR, CREATE_VIEWS, Conv, Host, MEMSET, MSG_DATA_BLOCK, MSG_DATA_BLOCK_NAMED,
        MSG_INDEX_BLOCK, MSG_INDEX_BLOCK_NAMED, MSG_LOCATION, MSG_UNKNOWN, PARAMETER_NAME, PRINTF,
        com, int_to_f32, uint_to_f32,
    };

    /// The bytes of `bind_targets`'s frame callees see, from `ebp -
    /// 0x23c` (the 0x200-byte null-view array) to `ebp - 0x3c`; the
    /// viewport (`ebp - 0x1c`, six floats) is at `frame + 0x220`.
    pub const BIND_FRAME: u32 = 0x200;
    /// The words between that part and `ebp`.
    pub const BIND_GAP: u32 = 0x3c;

    /// What the colour targets agree on so far: width, height and sample
    /// byte, once one is bound.
    struct Size {
        any: bool,
        width: u32,
        height: u32,
        samples: u32,
    }

    impl Size {
        /// Takes `(w, h, s)` for the first target, else checks it.
        fn agree(&mut self, w: u32, ht: u32, s: u32) -> bool {
            if self.any {
                self.width == w && self.height == ht && self.samples == s
            } else {
                self.width = w;
                self.height = ht;
                self.samples = s;
                true
            }
        }
    }

    /// The texture of a 2D (kind 0 or 1) or cube (kind 2) target bindable
    /// as a render target: its descriptor's flags (`[[tex] + 0x10]`) have
    /// `bit` set and bit 1 (not a target) clear.
    fn bindable(h: &impl Host, tex: u32, bit: u32) -> bool {
        if tex == 0 {
            return false;
        }
        let flags: u32 = h.read(h.read::<u32>(tex) + 0x10);
        flags >> bit & 1 != 0 && flags >> 1 & 1 == 0
    }

    /// `PRendererD3D11::bindRenderTargets(this, flags)` (`0x0058c830`,
    /// thiscall, `ret 4`) -> `al` 1, or 0 when a target's views fail or
    /// the targets disagree. The eight colour slots (`+0x228`, 16 bytes:
    /// target, face) give their views to `+0x2b8..0x2d4` (2D: `+0x10`;
    /// cube: `[+0x1c][face]`) and their count (the last bound slot + 1)
    /// to `+0x2fc`; the depth target (`+0x2a8`, face `+0x2ac`) its view to
    /// `+0x2d8`. Without colour targets the back buffer's (`+0x1b0`,
    /// `+0x1dc`, the window's size at `+0x1a0` unless `+0x1a4`, the
    /// viewport at `+0x1324..0x1330`). Then every stage's shader
    /// resources 0..0x80 are unbound (vtable `+0x20`, `+0x64`, `+0x7c`,
    /// `+0x10c`, `+0xec`, `+0xfc` with a zeroed array at `ebp - 0x23c`),
    /// the targets set (`+0x84`), the viewport set (`+0xb0`: x, the
    /// height minus y and h, w, h, 0, 1 as floats at `ebp - 0x1c`) and
    /// the clear made (`0x0058e3d0(this, flags)`). `frame` is `ebp -
    /// 0x23c`.
    #[allow(clippy::too_many_lines)]
    pub fn bind_targets(h: &mut impl Host, frame: u32, this: u32, flags: u32) -> u8 {
        h.write(this + 0x2fc, 0u32);
        h.write(this + 0x300, 0u32);
        let mut size = Size {
            any: false,
            width: 0,
            height: 0,
            samples: 0,
        };
        let mut x = 0u32;
        let mut y = 0u32;
        for k in 0..8u32 {
            let slot = this + 0x228 + 16 * k;
            let out = this + 0x2b8 + 4 * k;
            let t: u32 = h.read(slot);
            if t == 0 {
                h.write(out, 0u32);
                continue;
            }
            if h.call_sym(CREATE_VIEWS, Conv::Thiscall(t), &[]) != 0 {
                return 0;
            }
            let kind: u8 = h.read(t + 4);
            let view: u32 = match kind {
                0 | 1 => {
                    let tex: u32 = h.read(t);
                    if !bindable(h, tex, 6) {
                        return 0;
                    }
                    let (w, ht) = (h.read(tex + 0x1c), h.read(tex + 0x20));
                    if !size.agree(w, ht, h.read::<u8>(t + 5).into()) {
                        return 0;
                    }
                    h.read(t + 0x10)
                }
                2 => {
                    let tex: u32 = h.read(t);
                    if !bindable(h, tex, 8) {
                        return 0;
                    }
                    // a cube face is square: its width is both sides
                    let w: u32 = h.read(tex + 0x1c);
                    if !size.agree(w, w, h.read::<u8>(t + 5).into()) {
                        return 0;
                    }
                    let face: u32 = h.read(slot + 4);
                    h.read(h.read::<u32>(t + 0x1c).wrapping_add(face.wrapping_mul(4)))
                }
                _ => continue,
            };
            size.any = true;
            h.write(out, view);
            h.write(this + 0x2fc, k + 1);
        }
        let depth: u32 = h.read(this + 0x2a8);
        let mut bound = false;
        if depth == 0 {
            h.write(this + 0x2d8, 0u32);
        } else {
            if h.call_sym(CREATE_VIEWS, Conv::Thiscall(depth), &[]) != 0 {
                return 0;
            }
            let kind: u8 = h.read(depth + 4);
            let tex: u32 = h.read(depth);
            match kind {
                0 | 1 => {
                    let (w, ht) = (h.read(tex + 0x1c), h.read(tex + 0x20));
                    if !size.agree(w, ht, h.read::<u8>(depth + 5).into()) {
                        return 0;
                    }
                    let view: u32 = h.read(depth + 0x14);
                    h.write(this + 0x2d8, view);
                    bound = true;
                }
                2 => {
                    let w: u32 = h.read(tex + 0x1c);
                    if !size.agree(w, w, h.read::<u8>(depth + 5).into()) {
                        return 0;
                    }
                    let face: u32 = h.read(this + 0x2ac);
                    let view: u32 = h.read(
                        h.read::<u32>(depth + 0x24)
                            .wrapping_add(face.wrapping_mul(4)),
                    );
                    h.write(this + 0x2d8, view);
                    bound = true;
                }
                _ => {}
            }
        }
        if bound || size.any {
            h.write(this + 0x1334, size.width);
            h.write(this + 0x1338, size.height);
        } else {
            let back: u32 = h.read(this + 0x1b0);
            h.write(this + 0x2b8, back);
            h.write(this + 0x2fc, 1u32);
            let back_depth: u32 = h.read(this + 0x1dc);
            h.write(this + 0x2d8, back_depth);
            for (field, off) in [(0x1334, 0x1c), (0x1338, 0x20)] {
                let window = if h.read::<u8>(this + 0x1a4) == 0 {
                    h.read(this + 0x1a0)
                } else {
                    0u32
                };
                let v: u32 = h.read(window.wrapping_add(off));
                h.write(this + field, v);
            }
            size.width = h.read(this + 0x132c);
            x = h.read(this + 0x1324);
            y = h.read(this + 0x1328);
            size.height = h.read(this + 0x1330);
        }
        // the null shader-resource views
        h.write(frame, 0u32);
        h.call_sym(MEMSET, Conv::Cdecl, &[frame + 4, 0, 0x1fc]);
        for slot in [0x20, 0x64, 0x7c, 0x10c, 0xec, 0xfc] {
            let ctx: u32 = h.read(this + 0x70);
            let vt: u32 = h.read(ctx);
            com(h, vt, ctx, slot / 4, &[0, 0x80, frame]);
        }
        let n: u32 = h.read(this + 0x2fc);
        let depth_view: u32 = h.read(this + 0x2d8);
        let views = if n == 0 { 0 } else { this + 0x2b8 };
        let ctx: u32 = h.read(this + 0x70);
        let vt: u32 = h.read(ctx);
        com(h, vt, ctx, 0x84 / 4, &[n, views, depth_view]);
        let viewport = frame + 0x220;
        let full: u32 = h.read(this + 0x1338);
        let top = full.wrapping_sub(y).wrapping_sub(size.height);
        let words = [
            int_to_f32(x as i32),
            int_to_f32(top as i32),
            uint_to_f32(size.width),
            uint_to_f32(size.height),
            0,
            0x3f80_0000,
        ];
        for (k, w) in (0u32..).zip(words) {
            h.write(viewport + 4 * k, w);
        }
        let ctx: u32 = h.read(this + 0x70);
        com(h, viewport, ctx, 0xb0 / 4, &[1, viewport]);
        h.call_sym(CLEAR, Conv::Thiscall(this), &[flags]);
        1
    }

    /// One texture-parameter getter: `getter(obj; arg)` when `obj` is
    /// set, else 0.
    fn get(h: &mut impl Host, getter: u32, obj: u32, arg: u32) -> u32 {
        if obj == 0 {
            0
        } else {
            h.call_sym(getter, Conv::Thiscall(obj), &[arg])
        }
    }

    /// A data-block parameter with no block: the slot cleared and an
    /// error printed, with the parameter's name when `0x0060fdd0` finds
    /// one.
    fn missing_block(
        h: &mut impl Host,
        this: u32,
        owner: u32,
        index: u32,
        entry: u32,
        loc: u32,
        msgs: (u32, u32),
    ) {
        h.write(
            this.wrapping_add(0x1c).wrapping_add(loc.wrapping_mul(4)),
            0u32,
        );
        let name = h.call_sym(PARAMETER_NAME, Conv::Thiscall(owner), &[index, entry]);
        if name == 0 {
            let msg = h.rebase(msgs.1);
            h.call_sym(PRINTF, Conv::Cdecl, &[1, msg]);
        } else {
            let w: u32 = h.read(name + 4);
            let msg = h.rebase(msgs.0);
            h.call_sym(PRINTF, Conv::Cdecl, &[1, msg, w.wrapping_sub(w & 1)]);
        }
    }

    /// `PShaderProgramBindingD3D11::setTextureParameters(this, owner,
    /// program, index, entries, unused, count)` (`0x0059b3d0`, thiscall,
    /// `ret 0x18`) -> `al` 1. With the program's parameter block
    /// (`[program + 0x20 + 4 index]`, and `program + 0x14` set) and
    /// `count` (a `u16`) entries (16 bytes: `u16` offset into the block,
    /// location; -1 skips), each parameter (type word, argument,
    /// sampler/texture/buffer objects at `+8..0x14`) by its type: the
    /// textures 0x42..0x44, 0x48/0x4b into `+0x1c + 4 loc` (the index
    /// into `+0x29c + 4 loc`), samplers 0x47 into `+0x21c` (index at
    /// `+0x49c`), buffers 0x49, 0x4c..0x4e into `+0x25c`, constant
    /// buffers 0x4a into `+0x27c` (locations under 8), data blocks 0xde /
    /// 0xe2; other non-zero types print an error.
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    pub fn set_texture_parameters(
        h: &mut impl Host,
        this: u32,
        owner: u32,
        program: u32,
        index: u32,
        entries: u32,
        count: u16,
    ) -> u8 {
        let block: u32 = h.read(
            program
                .wrapping_add(0x20)
                .wrapping_add(index.wrapping_mul(4)),
        );
        if block == 0 || h.read::<u32>(program + 0x14) == 0 {
            return 1;
        }
        if entries == 0 || count == 0 {
            return 1;
        }
        let at = |field: u32, loc: u32| this.wrapping_add(field).wrapping_add(loc.wrapping_mul(4));
        for k in 0..u32::from(count) {
            let entry = entries.wrapping_add(16 * k);
            let loc: u32 = h.read(entry.wrapping_add(4));
            let p = u32::from(h.read::<u16>(entry)).wrapping_add(block);
            if loc == u32::MAX {
                continue;
            }
            let kind: u32 = h.read(p);
            let arg: u32 = h.read(p + 4);
            match kind {
                0x42..=0x44 | 0x48 | 0x4b => {
                    let obj: u32 = h.read(p + 0xc);
                    let v = match kind {
                        0x42 => get(h, 0x0059_3db0, obj, arg),
                        0x43 => get(h, 0x0059_3de0, obj, arg),
                        0x44 => get(h, 0x0059_3e30, obj, arg),
                        _ => {
                            let other: u32 = h.read(p + 0x10);
                            if obj == 0 {
                                get(h, 0x0059_4730, other, arg)
                            } else {
                                get(h, 0x0059_3d60, obj, arg)
                            }
                        }
                    };
                    h.write(at(0x1c, loc), v);
                    let loc2: u32 = h.read(entry + 4);
                    h.write(at(0x29c, loc2), index);
                }
                0x47 => {
                    let sampler: u32 = h.read(p + 8);
                    let v = if sampler == 0 {
                        0
                    } else {
                        h.read(sampler + 0x20)
                    };
                    h.write(at(0x21c, loc), v);
                    let loc2: u32 = h.read(entry + 4);
                    h.write(at(0x49c, loc2), index);
                }
                0x49 => {
                    let obj: u32 = h.read(p + 0xc);
                    let other: u32 = h.read(p + 0x10);
                    let v = if obj == 0 {
                        get(h, 0x0059_4830, other, arg)
                    } else {
                        get(h, 0x0059_3f90, obj, arg)
                    };
                    h.write(at(0x25c, loc), v);
                }
                0x4a => {
                    let obj: u32 = h.read(p + 0xc);
                    let v = if obj == 0 {
                        0
                    } else {
                        h.call_sym(0x0059_37a0, Conv::Thiscall(obj), &[arg]);
                        h.call_sym(0x0059_3ca0, Conv::Thiscall(obj), &[arg])
                    };
                    let loc2: u32 = h.read(entry + 4);
                    if loc2 < 8 {
                        h.write(at(0x27c, loc2), v);
                    } else {
                        let msg = h.rebase(MSG_LOCATION);
                        h.call_sym(PRINTF, Conv::Cdecl, &[1, msg, loc2]);
                    }
                }
                0x4c => {
                    let obj: u32 = h.read(p + 0xc);
                    let other: u32 = h.read(p + 0x10);
                    let captured: u32 = h.read(p + 0x14);
                    let v = if obj != 0 {
                        h.call_sym(0x0059_3f90, Conv::Thiscall(obj), &[arg])
                    } else if captured != 0 {
                        h.write(captured + 0x14, 1u8);
                        h.read(captured + 0x10)
                    } else {
                        get(h, 0x0059_4830, other, arg)
                    };
                    let loc2: u32 = h.read(entry + 4);
                    h.write(at(0x25c, loc2), v);
                }
                0x4d | 0x4e => {
                    let obj: u32 = h.read(p + 0xc);
                    let getter = if kind == 0x4d {
                        0x0059_3fe0
                    } else {
                        0x0059_4010
                    };
                    let v = get(h, getter, obj, arg);
                    h.write(at(0x25c, loc), v);
                }
                0xde | 0xe2 => {
                    let obj: u32 = h.read(p + 0xc);
                    let (getter, msgs) = if kind == 0xde {
                        (0x0059_4730, (MSG_DATA_BLOCK_NAMED, MSG_DATA_BLOCK))
                    } else {
                        (0x0059_4780, (MSG_INDEX_BLOCK_NAMED, MSG_INDEX_BLOCK))
                    };
                    if obj == 0 {
                        missing_block(h, this, owner, index, entry, loc, msgs);
                    } else {
                        let v = h.call_sym(getter, Conv::Thiscall(obj), &[0]);
                        h.write(at(0x1c, loc), v);
                    }
                }
                0 => {}
                _ => {
                    let msg = h.rebase(MSG_UNKNOWN);
                    h.call_sym(PRINTF, Conv::Cdecl, &[1, msg, kind]);
                }
            }
        }
        1
    }
}

/// `PRenderScopeLock_enter_00463d50` / `_leave_00463e90` (thiscall on a
/// lock object in the caller's frame).
const LOCK_ENTER: u32 = 0x0046_3d50;
const LOCK_LEAVE: u32 = 0x0046_3e90;
/// The renderer singleton's guard bits, the singleton, its constructor
/// and its `atexit` destructor; the device pointer in it.
const RENDERER_GUARD: u32 = 0x00c9_628c;
const RENDERER: u32 = 0x00c9_4f00;
const RENDERER_CTOR: u32 = 0x004f_ee70;
const RENDERER_DTOR: u32 = 0x00af_b710;
const ATEXIT: u32 = 0x0094_9207;
const DEVICE: u32 = 0x00c9_4f6c;
/// `Phyre_PRendering_CountOrOne24_00593bb0` (thiscall `(texture)`).
const SURFACE_COUNT: u32 = 0x0059_3bb0;
/// `PTexture2DD3D11_createViews_0058f710` (thiscall `(texture)`).
const CREATE_TEXTURE_VIEWS: u32 = 0x0058_f710;
/// `WKPDID_D3DDebugObjectName` and the two debug names (`.rdata`).
const DEBUG_NAME_GUID: u32 = 0x00b6_ae3c;
const NAME_TEXTURE: u32 = 0x00b3_3aa0;
const NAME_TEXTURE_MSAA: u32 = 0x00b3_3aac;

pub mod bridge2 {
    //! The originals over game memory.

    use super::{
        ATEXIT, CREATE_TEXTURE_VIEWS, Conv, DEBUG_NAME_GUID, DEVICE, Host, LOCK_ENTER, LOCK_LEAVE,
        MEMSET, NAME_TEXTURE, NAME_TEXTURE_MSAA, RENDERER, RENDERER_CTOR, RENDERER_DTOR,
        RENDERER_GUARD, SURFACE_COUNT, com,
    };

    /// The bytes of `create_device_texture`'s frame below `ebp`.
    pub const FRAME: u32 = 0x230;

    /// The renderer singleton made once (guard bit 0), then its device.
    fn device(h: &mut impl Host) -> u32 {
        let guard = h.rebase(RENDERER_GUARD);
        let g: u32 = h.read(guard);
        if g & 1 == 0 {
            h.write(guard, g | 1);
            let renderer = h.rebase(RENDERER);
            h.call_sym(RENDERER_CTOR, Conv::Thiscall(renderer), &[]);
            let dtor = h.rebase(RENDERER_DTOR);
            h.call_sym(ATEXIT, Conv::Cdecl, &[dtor]);
        }
        h.read(h.rebase(DEVICE))
    }

    /// The surfaces' array (`PSharray` at `+0x24`, 0x1c-byte entries:
    /// texture, its MSAA texture, its view, ...): inline for one, none
    /// for zero.
    fn surfaces(h: &impl Host, this: u32) -> u32 {
        let c = h.read::<u32>(this + 0x24) & 0x7fff_ffff;
        if c > 1 {
            h.read(this + 0x28)
        } else if c != 0 {
            this + 0x28
        } else {
            0
        }
    }

    /// `PTexture2DD3D11::createDeviceTexture(this, data, bind, unused,
    /// map)` (`0x0058fc20`, thiscall, `ret 0x10`) -> 0, 5 (a map of a
    /// texture whose `+0x15` bit 0 is set), 0x17 (no device) or 9 (a
    /// failed creation), or `createViews`' result. Under the render
    /// scope lock (`ebp - 0x1f8`): the `D3D11_TEXTURE2D_DESC` at `ebp -
    /// 0x1bc` (size `+0x1c`/`+0x20`, levels `+0xc + 1`, or 1 without
    /// `+0x10` and `+0x14` bit 1; format `[[this] + 0x18]`; the sample
    /// count from `[this + 0x18] + 5` with the device's feature level
    /// (`vtable[0x25]`, at least 0xa100) deciding a resolvable MSAA
    /// texture; usage, bind and CPU flags), per level the
    /// `D3D11_SUBRESOURCE_DATA` at `ebp - 0x190` from `data` (level k
    /// at the sum of the earlier levels' `bpp * w * h / 8`, halving w
    /// and h without a floor, 0 taken as the top level's, both rounded
    /// up to 4 for block formats; the pitch from the same width), then
    /// for each surface `CreateTexture2D` (`vtable[5]`), its private
    /// data (`+0x28` with `this + 0x44`), its debug name, and with
    /// `[this + 0x18] + 8` bit 0 an MSAA twin from the copy at `ebp -
    /// 0x1e8`. Afterwards either the MSAA views (`+0x64` bit 3, the
    /// shader-resource desc at `ebp - 0x230`, `vtable[7]`, under a
    /// second lock at `ebp - 0x1f0`) or, when bound as a resource,
    /// `createViews`. `ebp` is the original's frame base.
    #[allow(clippy::too_many_lines, clippy::too_many_arguments)]
    pub fn create_device_texture(
        h: &mut impl Host,
        ebp: u32,
        this: u32,
        data: u32,
        bind: u8,
        map: u8,
    ) -> u32 {
        let local = |off: u32| ebp.wrapping_sub(off);
        let target = (h.read::<u32>(this + 0x14) >> 8) & 1 != 0;
        let mut dynamic = if map == 0 {
            data == 0 && !target
        } else {
            if target {
                return 5;
            }
            true
        };
        let tex: u32 = h.read(this);
        let lock = local(0x1f8);
        h.call_sym(LOCK_ENTER, Conv::Thiscall(lock), &[]);
        let dev = device(h);
        if dev == 0 {
            h.call_sym(LOCK_LEAVE, Conv::Thiscall(lock), &[]);
            return 0x17;
        }
        let desc = local(0x1bc);
        h.call_sym(MEMSET, Conv::Cdecl, &[desc, 0, 0x2c]);
        let width: u32 = h.read(this + 0x1c);
        let height: u32 = h.read(this + 0x20);
        h.write(desc, width);
        h.write(desc + 4, height);
        let mut levels = h.read::<u32>(this + 0xc).wrapping_add(1);
        if h.read::<u32>(this + 0x10) == 0 && (h.read::<u32>(this + 0x14) >> 1) & 1 == 0 {
            levels = 1;
        }
        h.write(desc + 8, levels);
        let format: u32 = h.read(tex + 0x18);
        h.write(desc + 0x10, format);
        if dynamic && levels > 1 {
            dynamic = false;
        }
        h.write(desc + 0xc, 1u32);
        h.write(desc + 0x14, 1u32);
        h.write(desc + 0x18, 0u32);
        let mut msaa = false;
        let mut resolvable = false;
        let mut bind = bind;
        let ms: u32 = h.read(this + 0x18);
        let samples = if ms == 0 {
            0
        } else {
            u32::from(h.read::<u8>(ms + 5))
        };
        if samples != 0 {
            h.write(desc + 0x14, samples);
        }
        if samples > 1 {
            if (h.read::<u32>(tex + 0x10) >> 1) & 1 != 0 {
                // `ecx` is the sample count at this call
                let level = com(h, samples, dev, 0x94 / 4, &[]);
                if (level as i32) < 0xa100 {
                    bind = 0;
                } else {
                    resolvable = true;
                }
            }
            if h.read::<u8>(h.read::<u32>(this + 0x18) + 8) & 1 != 0 {
                msaa = true;
            }
        }
        let mut flags = if bind == 0 { 0 } else { 8 };
        if target {
            flags |= if (h.read::<u32>(tex + 0x10) >> 1) & 1 == 0 {
                0x20
            } else {
                0x40
            };
        }
        h.write(desc + 0x20, flags);
        h.write(desc + 0x1c, if dynamic { 2u32 } else { 0 });
        h.write(desc + 0x24, if dynamic { 0x1_0000u32 } else { 0 });
        if levels != 0 {
            let tex: u32 = h.read(this);
            let mut at = data;
            let (mut w, mut ht) = (width, height);
            for k in 0..levels {
                let block = h.read::<u8>(tex + 0x10) & 1 != 0;
                let bpp: u32 = h.read(tex + 0xc);
                let mut cw = if w == 0 { width } else { w };
                let mut ch = if ht == 0 { height } else { ht };
                if block {
                    cw = (cw + 3) & !3;
                    ch = (ch + 3) & !3;
                }
                let size = bpp.wrapping_mul(cw).wrapping_mul(ch) >> 3;
                let mut pw = if w == 0 { width } else { w };
                let mut mul = 1;
                if block {
                    pw = (pw + 3) & !3;
                    mul = 4;
                }
                let pitch = bpp.wrapping_mul(pw).wrapping_mul(mul) >> 3;
                let entry = local(0x190).wrapping_add(k.wrapping_mul(12));
                h.write(entry, at);
                h.write(entry + 4, pitch);
                h.write(entry + 8, 0u32);
                at = at.wrapping_add(size);
                ht >>= 1;
                w >>= 1;
            }
        }
        let copy = local(0x1e8);
        for k in 0..11 {
            let v: u32 = h.read(desc + 4 * k);
            h.write(copy + 4 * k, v);
        }
        if msaa {
            h.write(desc + 0x14, 1u32);
        }
        let init = if data == 0 { 0 } else { local(0x190) };
        let mut i = 0u32;
        while i < h.call_sym(SURFACE_COUNT, Conv::Thiscall(this), &[]) {
            let off = i.wrapping_mul(0x1c);
            let slot = surfaces(h, this).wrapping_add(off);
            let vt: u32 = h.read(dev);
            if com(h, vt, dev, 5, &[desc, init, slot]) != 0 {
                h.call_sym(LOCK_LEAVE, Conv::Thiscall(lock), &[]);
                return 9;
            }
            let obj: u32 = h.read(surfaces(h, this).wrapping_add(off));
            com(h, obj, obj, 0x28 / 4, &[this + 0x44]);
            let obj: u32 = h.read(surfaces(h, this).wrapping_add(off));
            let vt: u32 = h.read(obj);
            let guid = h.rebase(DEBUG_NAME_GUID);
            let name = h.rebase(NAME_TEXTURE);
            com(h, vt, obj, 5, &[guid, 8, name]);
            if msaa {
                let slot = surfaces(h, this).wrapping_add(off).wrapping_add(4);
                let vt: u32 = h.read(dev);
                if com(h, vt, dev, 5, &[copy, 0, slot]) != 0 {
                    h.call_sym(LOCK_LEAVE, Conv::Thiscall(lock), &[]);
                    return 9;
                }
                let obj: u32 = h.read(surfaces(h, this).wrapping_add(off).wrapping_add(4));
                let vt: u32 = h.read(obj);
                let name = h.rebase(NAME_TEXTURE_MSAA);
                com(h, vt, obj, 5, &[guid, 0xd, name]);
            }
            i += 1;
        }
        h.call_sym(LOCK_LEAVE, Conv::Thiscall(lock), &[]);
        if !resolvable {
            if bind != 0 {
                return h.call_sym(CREATE_TEXTURE_VIEWS, Conv::Thiscall(this), &[]);
            }
            return 0;
        }
        if h.read::<u8>(this + 0x64) & 8 == 0 {
            return 0;
        }
        let lock2 = local(0x1f0);
        h.call_sym(LOCK_ENTER, Conv::Thiscall(lock2), &[]);
        let dev = device(h);
        h.write(local(0x1f8), dev);
        let view = local(0x230);
        let format: u32 = h.read(h.read::<u32>(this) + 0x20);
        let first: u32 = h.read(this + 0x4c);
        for (k, w) in (0u32..).zip([format, 6, 0, first, 0, 0]) {
            h.write(view + 4 * k, w);
        }
        let mut i = 0u32;
        while i < h.call_sym(SURFACE_COUNT, Conv::Thiscall(this), &[]) {
            let off = i.wrapping_mul(0x1c);
            let base = surfaces(h, this);
            let res: u32 = h.read(base.wrapping_add(off).wrapping_add(4));
            let vt: u32 = h.read(dev);
            let r = com(
                h,
                vt,
                dev,
                7,
                &[res, view, base.wrapping_add(off).wrapping_add(8)],
            );
            if (r as i32) < 0 {
                h.call_sym(LOCK_LEAVE, Conv::Thiscall(lock2), &[]);
                return 9;
            }
            i += 1;
        }
        h.call_sym(LOCK_LEAVE, Conv::Thiscall(lock2), &[]);
        0
    }
}
