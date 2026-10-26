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
