//! `PRendererBase::generateNodeParameterCaptureBuffer` (`0x005b7fe0`,
//! list p6-render-c; `m4-rest`): the per-node shader parameters of one
//! mesh instance written into a parameter capture buffer.
//!
//! `thiscall(renderer; node)`, `ret 4` -> the buffer, or 0 without a
//! camera (`renderer +0x5c`) or when the ring (`renderer +0x1c`) has no
//! room. The node: `+0x10` the mesh instance (`+0` its mesh, `+0x7c` its
//! textures), `+0x14` the skin's bind matrices, `+0x1c` the skin (`+8`
//! the bone count, `+0xc` the `(bind, joint)` u16 pairs), `+0x20` a
//! texture index, `+0x24` the world matrix, `+0x28` the node's matrix
//! block (`this` of its light lookups), `+0x68` the parameter layout
//! (`+0x28` the count, `+0x2c` 16-byte entries: `+2` the semantic
//! `0x80..0x91`, `+3` the skin format, `+8` the offset, `+0xa` the size;
//! `+0x32` the buffer's size).
//!
//! The frame: the bridge takes the original's `ebp` and builds every
//! matrix where the original does (the callees get those addresses).

use ffx_bridge::x87;
use ffx_graphic::host::{Conv, Host};

/// `Phyre_PCamera_ConvertProjectionMatrixDepthRangeToZeroOne` (cdecl
/// `(m, near, far)`, floats as words).
pub const DEPTH_TO_ZERO_ONE: u32 = 0x0045_4260;
/// `PMatrix4_FromMatrix3x4_00451080` (thiscall `(out, m)`, `ret 4`).
pub const FROM_3X4: u32 = 0x0045_1080;
/// `PMatrix4_Multiply_004ca360` (cdecl `(out, a, b)`) -> out.
pub const MULTIPLY: u32 = 0x004c_a360;
/// `PMatrix4_MulThis_004c9da0` (thiscall `(a, out, b)`, `ret 8`) -> out.
pub const MUL_THIS: u32 = 0x004c_9da0;
/// `PMatrix4_Copy_00453340` (thiscall `(out, m)`, `ret 4`) -> out.
pub const COPY: u32 = 0x0045_3340;
/// `PMatrix4_Transpose_004f93f0` (cdecl `(out, m)`) -> out.
pub const TRANSPOSE: u32 = 0x004f_93f0;
/// `sce_Vectormath_Scalar_Aos_inverse` (cdecl `(out, m)`) -> out.
pub const INVERSE: u32 = 0x0057_3d00;
/// The node block's light transform (thiscall `(block, out, v)`, `ret 8`)
/// -> the result's address.
pub const LIGHT_VECTOR: u32 = 0x005b_4c00;
/// A segmented ring's `allocate` (thiscall `(ring, size)`, `ret 4`).
pub const ALLOCATE: u32 = 0x0061_3e10;
/// `PParameterCaptureBuffer_setValue` (thiscall `(buffer, entry, value)`,
/// `ret 8`).
pub const SET_VALUE: u32 = 0x004f_edc0;
/// `Phyre_PRendering_bufferSlice_005b8b00` (thiscall `(buffer, entry)`,
/// `ret 4`) -> the entry's place in the buffer.
pub const SLICE: u32 = 0x005b_8b00;
/// The instance's texture lookup (thiscall `(instance, index)`, `ret 4`).
pub const TEXTURE: u32 = 0x005b_8c30;
/// `Phyre_PhyrePrintf` (cdecl, varargs).
pub const PRINTF: u32 = 0x0043_52f0;
/// The "unsupported skin format" warning's once-flag (byte) and format.
pub const WARNED: u32 = 0x00cb_bf5c;
pub const SKIN_FORMAT_WARNING: u32 = 0x00b3_66a4;
/// The frame's size below `ebp`.
pub const FRAME: u32 = 0x60c;

/// A float copied through the x87 stack (`fld dword`, `fstp dword`): a
/// signalling NaN comes out quiet.
#[must_use]
pub fn through_x87(bits: u32) -> u32 {
    x87::store(x87::load(bits))
}

/// The 3x4 a skin matrix's column-major 4x4 `m` stores (its rows 0..2,
/// rows read down the columns): `m0 m1 m2 m4 | m5 m6 m8 m9 | m10 m12 m13
/// m14`, the original's order.
#[must_use]
pub fn rows_3x4(m: &[u32; 16]) -> [u32; 12] {
    [
        m[0], m[1], m[2], m[4], m[5], m[6], m[8], m[9], m[10], m[12], m[13], m[14],
    ]
}

pub mod bridge {
    //! The original over game memory.

    use super::{
        ALLOCATE, COPY, Conv, DEPTH_TO_ZERO_ONE, FROM_3X4, Host, INVERSE, LIGHT_VECTOR, MUL_THIS,
        MULTIPLY, PRINTF, SET_VALUE, SKIN_FORMAT_WARNING, SLICE, TEXTURE, TRANSPOSE, WARNED,
        rows_3x4, through_x87,
    };

    /// The frame at the original's `ebp`.
    struct Frame(u32);

    impl Frame {
        fn at(&self, off: u32) -> u32 {
            self.0.wrapping_sub(off)
        }
    }

    /// What the per-node values are built from.
    struct Node {
        renderer: u32,
        /// The node itself.
        this: u32,
        /// `[node + 0x24]`: the world matrix.
        world: u32,
        /// `node + 0x28`: the node's matrix block.
        block: u32,
        /// `[node + 0x10]`: the instance.
        instance: u32,
        buffer: u32,
    }

    /// `out = copy(m)` into the frame's result matrix (`ebp - 0xa4`);
    /// returns its address.
    fn into_result(h: &mut impl Host, f: &Frame, m: u32) -> u32 {
        let result = f.at(0xa4);
        h.call_sym(COPY, Conv::Thiscall(result), &[m]);
        result
    }

    fn multiply(h: &mut impl Host, out: u32, a: u32, b: u32) -> u32 {
        h.call_sym(MULTIPLY, Conv::Cdecl, &[out, a, b])
    }

    fn transpose(h: &mut impl Host, out: u32, m: u32) -> u32 {
        h.call_sym(TRANSPOSE, Conv::Cdecl, &[out, m])
    }

    fn mul_this(h: &mut impl Host, a: u32, out: u32, b: u32) -> u32 {
        h.call_sym(MUL_THIS, Conv::Thiscall(a), &[out, b])
    }

    /// A light vector (4 words of the renderer's `+0x60` block at `from`)
    /// through the node block, copied through the x87 stack to `ebp -
    /// 0x54`; returns that address.
    fn light_vector(h: &mut impl Host, f: &Frame, n: &Node, from: u32, tmp: u32, out: u32) -> u32 {
        for k in 0..4 {
            let w: u32 = h.read(n.renderer + 0x60 + from + 4 * k);
            h.write(f.at(tmp) + 4 * k, w);
        }
        let v = h.call_sym(
            LIGHT_VECTOR,
            Conv::Thiscall(n.block),
            &[f.at(out), f.at(tmp)],
        );
        for k in 0..4 {
            let w: u32 = h.read(v + 4 * k);
            h.write(f.at(0x54) + 4 * k, through_x87(w));
        }
        f.at(0x54)
    }

    /// The skin's sources: the `(bind, joint)` u16 pairs and the two
    /// 64-byte matrix arrays.
    struct Skin {
        pairs: u32,
        binds: u32,
        joints: u32,
    }

    impl Skin {
        /// `bind[pair.0] . joint[pair.1]` into the frame at `ebp - out`.
        fn product(&self, h: &mut impl Host, f: &Frame, i: u32, out: u32) -> u32 {
            let bind = u32::from(h.read::<u16>(self.pairs + 4 * i));
            let joint = u32::from(h.read::<u16>(self.pairs + 4 * i + 2));
            mul_this(
                h,
                (bind << 6).wrapping_add(self.binds),
                f.at(out),
                (joint << 6).wrapping_add(self.joints),
            )
        }
    }

    /// The skin's matrices (semantic `0x8f`): `bind[pair.0] . joint
    /// [pair.1]` for each bone the entry has room for, as 4x4s (format
    /// `0x31`) or 3x4s (`0x30`); a short list ends with `0xffffffff`.
    /// Other formats warn once.
    fn skin(h: &mut impl Host, f: &Frame, n: &Node, entry: u32) {
        // the frame's words, read where the original reads them
        let mesh: u32 = h.read(f.at(0x34));
        let joints: u32 = h.read(mesh + 0xc);
        h.write(f.at(0x3c), joints);
        let format = u32::from(h.read::<u8>(entry + 3));
        let skin: u32 = h.read(n.this + 0x1c);
        let mut bones = h.read::<u32>(skin + 8) & 0x7fff_ffff;
        let pairs: u32 = h.read(skin + 0xc);
        h.write(f.at(0x14), pairs);
        let binds: u32 = h.read(f.at(0x2c));
        let size = u32::from(h.read::<u16>(entry + 0xa));
        let skin = Skin {
            pairs,
            binds,
            joints,
        };
        match format {
            0x31 => {
                let room = (size >> 6) & 0x7f;
                h.write(f.at(0xc), room);
                bones = bones.min(room);
                let dst = h.call_sym(SLICE, Conv::Thiscall(n.buffer), &[entry]);
                h.write(f.at(0x10), dst);
                for i in 0..bones {
                    let m = skin.product(h, f, i, 0x58c);
                    h.call_sym(COPY, Conv::Thiscall(dst.wrapping_add(i << 6)), &[m]);
                }
                if bones < room {
                    h.write(dst.wrapping_add(bones << 6), u32::MAX);
                }
            }
            0x30 => {
                let room = (size & 0x1fff) / 48;
                h.write(f.at(0x28), room);
                bones = bones.min(room);
                let dst = h.call_sym(SLICE, Conv::Thiscall(n.buffer), &[entry]);
                h.write(f.at(0xc4), dst);
                h.write(f.at(0x14), 0_u32);
                for i in 0..bones {
                    let m = skin.product(h, f, i, 0x60c);
                    let r = into_result(h, f, m);
                    let mut w = [0_u32; 16];
                    for (k, x) in w.iter_mut().enumerate() {
                        *x = through_x87(h.read(r + 4 * k as u32));
                    }
                    let rows = rows_3x4(&w);
                    let out = dst.wrapping_add(i * 0x30);
                    for (k, &x) in rows.iter().enumerate() {
                        h.write(out + 4 * k as u32, x);
                    }
                    // the original's frame copies on the way
                    for (off, x) in [
                        (0xac, w[9]),
                        (0xb0, w[8]),
                        (0xb8, w[6]),
                        (0xcc, w[5]),
                        (0xa8, w[14]),
                        (0xb4, w[13]),
                        (0xc0, w[12]),
                        (0xbc, w[10]),
                        (0x24, w[12]),
                        (0xc, w[13]),
                        (0x10, w[14]),
                    ] {
                        h.write(f.at(off), x);
                    }
                    h.write(f.at(0x14), i + 1);
                }
                if bones < room {
                    h.write(dst.wrapping_add(bones * 0x30), u32::MAX);
                }
            }
            _ => {
                let warned = h.rebase(WARNED);
                if h.read::<u8>(warned) == 0 {
                    h.write(warned, 1_u8);
                    let text = h.rebase(SKIN_FORMAT_WARNING);
                    h.call_sym(PRINTF, Conv::Cdecl, &[1, text, format]);
                }
            }
        }
    }

    /// One parameter entry: its value's address, or `None` when the
    /// original writes nothing (a missing source, the two semantics whose
    /// value it builds but never stores, the skin, an unknown semantic).
    fn value(h: &mut impl Host, f: &Frame, n: &Node, entry: u32) -> Option<u32> {
        let semantic = u32::from(h.read::<u8>(entry + 2));
        let lights = n.renderer + 0xa0;
        let view_proj = f.at(0x12c);
        let world = n.world;
        let block = n.block;
        Some(match semantic.wrapping_sub(0x80) {
            0 => light_vector(h, f, n, 0x20, 0xdc, 0x13c),
            1 => light_vector(h, f, n, 0x30, 0xec, 0x14c),
            2 => {
                if world == 0 {
                    return None;
                }
                world
            }
            3 => {
                let m = mul_this(h, lights, f.at(0x3cc), world);
                into_result(h, f, m)
            }
            4 => {
                let m = multiply(h, f.at(0x24c), view_proj, world);
                into_result(h, f, m)
            }
            5 => {
                if block == 0 {
                    return None;
                }
                block
            }
            6 => {
                let m = mul_this(h, block, f.at(0x44c), n.renderer + 0x60);
                into_result(h, f, m)
            }
            7 => {
                let m = multiply(h, f.at(0x2cc), view_proj, world);
                let m = h.call_sym(INVERSE, Conv::Cdecl, &[f.at(0x54c), m]);
                into_result(h, f, m)
            }
            8 => {
                let m = transpose(h, f.at(0x34c), block);
                into_result(h, f, m)
            }
            9 => {
                let m = mul_this(h, block, f.at(0x4cc), n.renderer + 0x60);
                let m = transpose(h, f.at(0x1cc), m);
                into_result(h, f, m)
            }
            10 => {
                let m = multiply(h, f.at(0x5cc), view_proj, world);
                let m = h.call_sym(INVERSE, Conv::Cdecl, &[f.at(0x20c), m]);
                let m = transpose(h, f.at(0x28c), m);
                into_result(h, f, m);
                return None;
            }
            11 => {
                let m = transpose(h, f.at(0x30c), world);
                into_result(h, f, m)
            }
            12 => {
                let m = mul_this(h, lights, f.at(0x38c), world);
                let m = transpose(h, f.at(0x40c), m);
                into_result(h, f, m)
            }
            13 => {
                let m = multiply(h, f.at(0x48c), view_proj, world);
                let m = transpose(h, f.at(0x50c), m);
                into_result(h, f, m);
                return None;
            }
            14 => {
                let mesh: u32 = h.read(f.at(0x34));
                let count = h.read::<u32>(mesh + 8) & 0x7fff_ffff;
                h.write(f.at(0x28), count);
                // fild (exact: bit 31 is clear), fstp dword: one rounding
                #[allow(clippy::cast_precision_loss)]
                let value = count as f32;
                h.write(f.at(0xc8), value.to_bits());
                f.at(0xc8)
            }
            15 => {
                skin(h, f, n, entry);
                return None;
            }
            16 | 17 => {
                let index: u32 = h.read(n.this + 0x20);
                let texture = if semantic == 0x90 {
                    h.call_sym(TEXTURE, Conv::Thiscall(n.instance), &[index])
                } else {
                    let textures: u32 = h.read(n.instance + 0x7c);
                    h.read(textures.wrapping_add(index << 2))
                };
                h.write(f.at(0x58), texture);
                if texture != 0 {
                    let w: u32 = h.read(texture + 4);
                    h.write(f.at(0x60), w);
                }
                f.at(0x64)
            }
            _ => return None,
        })
    }

    /// `generateNodeParameterCaptureBuffer(renderer; node)` with the
    /// original's frame at `ebp`: see the module.
    pub fn generate(h: &mut impl Host, renderer: u32, node: u32, ebp: u32) -> u32 {
        let f = Frame(ebp);
        let layout: u32 = h.read(node + 0x68);
        h.write(f.at(0x44), layout);
        let instance: u32 = h.read(node + 0x10);
        let world: u32 = h.read(node + 0x24);
        h.write(f.at(4), world);
        let binds: u32 = h.read(node + 0x14);
        h.write(f.at(0x2c), binds);
        let mesh: u32 = h.read(instance);
        h.write(f.at(0x38), instance);
        h.write(f.at(0x20), renderer);
        h.write(f.at(8), node.wrapping_add(0x28));
        h.write(f.at(0x34), mesh);
        let camera: u32 = h.read(renderer + 0x5c);
        if camera == 0 {
            return 0;
        }
        h.write(f.at(0x1c), renderer.wrapping_add(0x60));
        h.write(f.at(0x30), renderer.wrapping_add(0xa0));
        // the projection with depth 0..1, the camera's 4x4, their product
        let projection = f.at(0x18c);
        for k in 0..16 {
            let w: u32 = h.read(camera + 0x30 + 4 * k);
            h.write(projection + 4 * k, w);
        }
        let near = through_x87(h.read(camera + 0xb4));
        let far = through_x87(h.read(camera + 0xb8));
        h.call_sym(DEPTH_TO_ZERO_ONE, Conv::Cdecl, &[projection, near, far]);
        h.call_sym(FROM_3X4, Conv::Thiscall(f.at(0xa4)), &[camera]);
        multiply(h, f.at(0x12c), projection, f.at(0xa4));
        let size = u32::from(h.read::<u16>(layout + 0x32));
        let buffer = h.call_sym(ALLOCATE, Conv::Thiscall(renderer + 0x1c), &[size]);
        h.write(f.at(0x18), buffer);
        if buffer == 0 {
            return 0;
        }
        h.write(buffer, size);
        let count = h.read::<u32>(layout + 0x28) & 0x7fff_ffff;
        h.write(f.at(0x20), count);
        let n = Node {
            renderer,
            this: node,
            world,
            block: node + 0x28,
            instance,
            buffer,
        };
        let mut offset = 0_u32;
        h.write(f.at(0x40), offset);
        for left in (0..count).rev() {
            let entries: u32 = h.read(h.read::<u32>(f.at(0x44)) + 0x2c);
            let entry = entries.wrapping_add(offset);
            for (off, w) in [(0x64, 0x4a_u32), (0x60, 0), (0x5c, 0), (0x58, 0)] {
                h.write(f.at(off), w);
            }
            if let Some(v) = value(h, &f, &n, entry) {
                h.call_sym(SET_VALUE, Conv::Thiscall(buffer), &[entry, v]);
            }
            offset += 0x10;
            h.write(f.at(0x40), offset);
            h.write(f.at(0x20), left);
        }
        buffer
    }
}
