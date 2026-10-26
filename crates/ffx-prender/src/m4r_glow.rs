//! `PLegacyGlowGPUBase_render` (`0x00a1de20`, list p6-render-c;
//! `m4-rest`): the legacy glow's blur chain, drawn by
//! `PostProcessManager_render` (`docs/formats/postfx.md`).
//!
//! `thiscall(glow; renderer, target, source, _, _)`, `ret 0x14` -> 0, or
//! the first error: a callee's, 3 for a parameter that does not fit its
//! buffer (or sits below offset 4), 1 for a null gain. `min(+0x50, 3)`
//! levels: down, level `i` blurs its input (the source at level 0, else
//! `+0x84[i-1]`) horizontally into `+0x78[i]`, then that vertically into
//! `+0x84[i]`; up, from the last level to 0, `+0x84[i]` horizontally into
//! `+0x78[i]`, then that vertically and added into `+0x84[i]` (the target
//! at level 0, the back buffer when it is 0).
//!
//! The glow: `+0x24` the sampler state, `+0x48` the screen mesh
//! instance, `+0x4c` its parameters (`+4` the parameter buffer, `+0`
//! its size), `+0x54` the gain, `+0x58` the spread, `+0x5c` the up passes'
//! scale, `+0x60` the texture parameter, `+0x64` the sampler parameter,
//! `+0x68` `GlowGain`, `+0x6c` `GaussianOutputScale`, `+0x70`
//! `GaussianBlurBufferSize` (descriptors: `+8` the u16 offset, `+0xa` the
//! size in its low 13 bits), `+0x78[3]` and `+0x84[3]` the render targets
//! (`{texture, byte unbound}`; a texture's `+0x1c`/`+0x20` are its size).
//!
//! The frame: the callees get the original's locals by address (the
//! render surfaces, the texture records), and the parameters are copied
//! from it, so the bridge keeps them at the original's `ebp` offsets. A
//! parameter longer than its 16-byte block copies the words after it in
//! the frame: the port's frame holds the same blocks, the other words 0
//! (the brief's junk-read rule; the game's are 16 bytes). The surface
//! destructor (`0x005b3640`) is a bare `ret`: not called. No SEH frame.

use ffx_bridge::x87;
use ffx_graphic::host::{Conv, Host};

/// `Render_SurfaceFormat_0066e4f0` (thiscall `(buffer; parameter,
/// &record)`, `ret 8`) -> 0 or an error.
pub const SET_TEXTURE: u32 = 0x0066_e4f0;
/// `Phyre_PParameterBuffer_setValueDesc_0056e600` (thiscall `(buffer;
/// parameter, sampler)`, `ret 8`) -> 0 or 3.
pub const SET_SAMPLER: u32 = 0x0056_e600;
/// `PRenderSurface::PRenderSurface` (thiscall `(surface; target, x)`,
/// `ret 8`).
pub const SURFACE: u32 = 0x005b_35c0;
/// `PRenderer::setColorTarget` (thiscall `(renderer; &surface, slot)`,
/// `ret 8`).
pub const SET_COLOR: u32 = 0x005b_b130;
/// `PRenderer::setDepthStencilTarget` (thiscall `(renderer; target)`,
/// `ret 4`).
pub const SET_DEPTH: u32 = 0x005b_b5b0;
/// `PRenderer::beginScene` (thiscall `(renderer; x)`, `ret 4`).
pub const BEGIN_SCENE: u32 = 0x005b_5e30;
/// `PRendererBase_queueCommand18` (thiscall `(renderer; byte, a, b)`,
/// `ret 0xc`).
pub const QUEUE18: u32 = 0x005b_aa40;
/// `PRendererBase::renderMeshInstance` (thiscall `(renderer; instance,
/// contexts, x, y)`, `ret 0x10`).
pub const RENDER_MESH: u32 = 0x005b_9a90;
/// `PRenderer::endScene` (thiscall `(renderer; x)`, `ret 4`).
pub const END_SCENE: u32 = 0x005b_6950;
/// The passes the renderer draws with (`renderer +0x48`): horizontal,
/// vertical, vertical added.
pub const PASS_H: u32 = 0x0194_4e4c;
pub const PASS_V: u32 = 0x0194_4e5c;
pub const PASS_ADD: u32 = 0x0194_4e9c;

/// The frame's size below `ebp`.
pub const FRAME: u32 = 0x148;
/// The texture records (`{0xff, 0, sampler, texture}`) at `ebp -` these:
/// down before the h pass, down before the v pass, up before the h pass,
/// up before the v pass.
pub const RECORDS: [u32; 4] = [0x98, 0x48, 0xe8, 0x68];
/// The sampler blocks (`{0x47, 0, sampler, 0}`): down, up h, up v.
pub const SAMPLERS: [u32; 3] = [0xa8, 0xb8, 0x88];
/// `GaussianOutputScale` down (`(s, s, s, 1)`), the ones of up level 0,
/// the up v scale.
pub const SCALE_DOWN: u32 = 0x78;
pub const ONES: u32 = 0xd8;
pub const SCALE_UP: u32 = 0x34;
/// `GaussianBlurBufferSize` (`(w, h, 0, 0)`): down, up.
pub const SIZE_DOWN: u32 = 0xc8;
pub const SIZE_UP: u32 = 0x58;
/// The render surfaces: down h, down v, up h, up v into the target, into
/// the back buffer, into a level.
pub const SURFACES: [u32; 6] = [0x118, 0xf8, 0x148, 0x138, 0x108, 0x128];
/// The size mask the gain's copy reads from the frame.
pub const MASK: u32 = 0x38;

/// `(f32)(unsigned)n` (`fild`, the 2^32 fix, `fstp dword`).
fn unsigned_f32(n: u32) -> u32 {
    x87::store(f64::from(n))
}

pub mod bridge {
    //! The original over game memory.

    use super::{
        BEGIN_SCENE, Conv, END_SCENE, Host, MASK, ONES, PASS_ADD, PASS_H, PASS_V, QUEUE18, RECORDS,
        RENDER_MESH, SAMPLERS, SCALE_DOWN, SCALE_UP, SET_COLOR, SET_DEPTH, SET_SAMPLER,
        SET_TEXTURE, SIZE_DOWN, SIZE_UP, SURFACE, SURFACES, unsigned_f32, x87,
    };

    /// The frame at the original's `ebp`.
    struct Frame(u32);

    impl Frame {
        fn at(&self, off: u32) -> u32 {
            self.0.wrapping_sub(off)
        }
    }

    /// The call's objects.
    struct Glow {
        this: u32,
        renderer: u32,
    }

    impl Glow {
        fn field(&self, h: &impl Host, off: u32) -> u32 {
            h.read(self.this.wrapping_add(off))
        }
        /// The parameter buffer (`[[glow + 0x4c] + 4]`).
        fn buffer(&self, h: &impl Host) -> u32 {
            h.read(self.field(h, 0x4c).wrapping_add(4))
        }
        fn sampler(&self) -> u32 {
            self.this.wrapping_add(0x24)
        }
        /// Whether the texture and sampler parameters are both set.
        fn has_texture(&self, h: &impl Host) -> bool {
            self.field(h, 0x60) != 0 && self.field(h, 0x64) != 0
        }
        /// Render target `k` of the table at `+base`.
        fn target(&self, h: &impl Host, base: u32, k: u32) -> u32 {
            self.field(h, base + 4 * k)
        }
    }

    /// A render target's texture: 0 when it is unbound (byte `+4`).
    fn texture(h: &impl Host, target: u32) -> u32 {
        if h.read::<u8>(target.wrapping_add(4)) == 0 {
            h.read(target)
        } else {
            0
        }
    }

    fn write_words(h: &mut impl Host, at: u32, words: [u32; 4]) {
        for (k, w) in (0_u32..).zip(words) {
            h.write(at.wrapping_add(4 * k), w);
        }
    }

    /// The inlined `PParameterBuffer::setValue`: `[desc + 0xa] & mask`
    /// bytes from `src` to the buffer at the u16 offset `[desc + 8]`; 3
    /// when they do not fit (`[buffer]` is its size) or the offset is
    /// below 4.
    fn copy_parameter(
        h: &mut impl Host,
        g: &Glow,
        desc: u32,
        src: u32,
        mask: u16,
    ) -> Result<(), u32> {
        let buffer = g.buffer(h);
        let offset = u32::from(h.read::<u16>(desc.wrapping_add(8)));
        let len = u32::from(h.read::<u16>(desc.wrapping_add(0xa)) & mask);
        if len.wrapping_add(offset) > h.read::<u32>(buffer) || offset < 4 {
            return Err(3);
        }
        let mut bytes = vec![0_u8; len as usize];
        h.read_bytes(src, &mut bytes);
        h.write_bytes(buffer.wrapping_add(offset), &bytes);
        Ok(())
    }

    fn ok(r: u32) -> Result<(), u32> {
        if r == 0 { Ok(()) } else { Err(r) }
    }

    /// The texture parameter set to `texture` through the record at
    /// `ebp - record`.
    fn set_texture(
        h: &mut impl Host,
        f: &Frame,
        g: &Glow,
        record: u32,
        texture: u32,
    ) -> Result<(), u32> {
        write_words(h, f.at(record), [0xff, 0, g.sampler(), texture]);
        let (buffer, desc) = (g.buffer(h), g.field(h, 0x60));
        ok(h.call_sym(SET_TEXTURE, Conv::Thiscall(buffer), &[desc, f.at(record)]))
    }

    /// The sampler parameter copied from the block at `ebp - block`.
    fn set_sampler(h: &mut impl Host, f: &Frame, g: &Glow, block: u32) -> Result<(), u32> {
        write_words(h, f.at(block), [0x47, 0, g.sampler(), 0]);
        let desc = g.field(h, 0x64);
        copy_parameter(h, g, desc, f.at(block), 0x1fff)
    }

    /// The parameter at `+field` (when set) copied from `ebp - block`.
    fn set_value(
        h: &mut impl Host,
        f: &Frame,
        g: &Glow,
        field: u32,
        block: u32,
    ) -> Result<(), u32> {
        let desc = g.field(h, field);
        if desc != 0 {
            copy_parameter(h, g, desc, f.at(block), 0x1fff)?;
        }
        Ok(())
    }

    /// `GaussianBlurBufferSize` = (w, h, 0, 0) of `target`'s texture at
    /// `ebp - at` (its last two words are set once).
    fn buffer_size(h: &mut impl Host, f: &Frame, target: u32, at: u32) {
        let tex = texture(h, target);
        let height: u32 = h.read(tex.wrapping_add(0x20));
        let width: u32 = h.read(tex.wrapping_add(0x1c));
        h.write(f.at(at), unsigned_f32(width));
        h.write(f.at(at - 4), unsigned_f32(height));
    }

    /// One blur pass into `target` (the surface at `ebp - surface`) with
    /// `pass`: the color target, no depth, the scene, command 0x18, the
    /// mesh.
    fn blur(
        h: &mut impl Host,
        f: &Frame,
        g: &Glow,
        target: u32,
        surface: u32,
        pass: u32,
    ) -> Result<(), u32> {
        let r = g.renderer;
        h.call_sym(SURFACE, Conv::Thiscall(f.at(surface)), &[target, 0]);
        ok(h.call_sym(SET_COLOR, Conv::Thiscall(r), &[f.at(surface), 0]))?;
        ok(h.call_sym(SET_DEPTH, Conv::Thiscall(r), &[0]))?;
        ok(h.call_sym(BEGIN_SCENE, Conv::Thiscall(r), &[0]))?;
        h.call_sym(QUEUE18, Conv::Thiscall(r), &[0, 6, 7]);
        draw(h, g, pass)?;
        ok(h.call_sym(END_SCENE, Conv::Thiscall(r), &[1]))
    }

    /// The screen mesh drawn with `pass`.
    fn draw(h: &mut impl Host, g: &Glow, pass: u32) -> Result<(), u32> {
        let r = g.renderer;
        let contexts: u32 = h.read(r.wrapping_add(0x13c));
        let pass = h.rebase(pass);
        h.write(r.wrapping_add(0x48), pass);
        let mesh = g.field(h, 0x48);
        let instance: u32 = h.read(mesh.wrapping_add(0x10));
        ok(h.call_sym(
            RENDER_MESH,
            Conv::Thiscall(r),
            &[mesh, instance, 0, contexts],
        ))
    }

    /// `PLegacyGlowGPUBase_render(glow; renderer, target, source)` with
    /// the frame at `ebp`.
    pub fn render(
        h: &mut impl Host,
        glow: u32,
        renderer: u32,
        target: u32,
        source: u32,
        ebp: u32,
    ) -> u32 {
        let f = Frame(ebp);
        let g = Glow {
            this: glow,
            renderer,
        };
        let levels = g.field(h, 0x50).min(3);
        h.write(f.at(MASK), 0x1fff_u32);
        match down(h, &f, &g, levels, source).and_then(|()| up(h, &f, &g, levels, target)) {
            Ok(()) => 0,
            Err(r) => r,
        }
    }

    /// Down: each level blurred h into `+0x78[i]`, v into `+0x84[i]`.
    fn down(h: &mut impl Host, f: &Frame, g: &Glow, levels: u32, source: u32) -> Result<(), u32> {
        if levels == 0 {
            return Ok(());
        }
        h.write(f.at(SIZE_DOWN - 8), 0_u32);
        h.write(f.at(SIZE_DOWN - 12), 0_u32);
        h.write(f.at(SCALE_DOWN - 12), 1.0_f32.to_bits());
        for i in 0..levels {
            let h_target = g.target(h, 0x78, i);
            buffer_size(h, f, h_target, SIZE_DOWN);
            // GaussianOutputScale = (s, s, s, 1), s = (i + 1) spread + 1
            let spread = g.field(h, 0x58);
            let s = x87::Reg::constant(f64::from(i + 1))
                .mul_m32(f32::from_bits(spread))
                .value();
            let s = x87::store(x87::add(s, 1.0));
            for k in 0..3 {
                h.write(f.at(SCALE_DOWN - 4 * k), s);
            }
            let input = if i == 0 {
                source
            } else {
                texture(h, g.target(h, 0x80, i))
            };
            if g.has_texture(h) {
                set_texture(h, f, g, RECORDS[0], input)?;
                set_sampler(h, f, g, SAMPLERS[0])?;
            }
            set_value(h, f, g, 0x6c, SCALE_DOWN)?;
            set_value(h, f, g, 0x70, SIZE_DOWN)?;
            blur(h, f, g, h_target, SURFACES[0], PASS_H)?;

            if g.has_texture(h) {
                let tex = texture(h, g.target(h, 0x78, i));
                set_texture(h, f, g, RECORDS[1], tex)?;
                let (buffer, desc) = (g.buffer(h), g.field(h, 0x64));
                ok(h.call_sym(SET_SAMPLER, Conv::Thiscall(buffer), &[desc, g.sampler()]))?;
            }
            let v_target = g.target(h, 0x84, i);
            blur(h, f, g, v_target, SURFACES[1], PASS_V)?;
        }
        Ok(())
    }

    /// Up, from the last level: `+0x84[i]` blurred h into `+0x78[i]`,
    /// then v and added into `+0x84[i]`, or the target at level 0.
    fn up(h: &mut impl Host, f: &Frame, g: &Glow, levels: u32, target: u32) -> Result<(), u32> {
        if levels == 0 {
            return Ok(());
        }
        h.write(f.at(SIZE_UP - 8), 0_u32);
        h.write(f.at(SIZE_UP - 12), 0_u32);
        for i in (0..levels).rev() {
            let h_target = g.target(h, 0x78, i);
            buffer_size(h, f, h_target, SIZE_UP);
            if g.has_texture(h) {
                let tex = texture(h, g.target(h, 0x84, i));
                set_texture(h, f, g, RECORDS[2], tex)?;
                set_sampler(h, f, g, SAMPLERS[1])?;
            }
            if i == 0 && g.field(h, 0x6c) != 0 {
                write_words(h, f.at(ONES), [1.0_f32.to_bits(); 4]);
                set_value(h, f, g, 0x6c, ONES)?;
            }
            set_value(h, f, g, 0x70, SIZE_UP)?;
            blur(h, f, g, h_target, SURFACES[2], PASS_H)?;

            if g.has_texture(h) {
                let tex = texture(h, g.target(h, 0x78, i));
                set_texture(h, f, g, RECORDS[3], tex)?;
                set_sampler(h, f, g, SAMPLERS[2])?;
            }
            if g.field(h, 0x6c) != 0 {
                let scale = up_scale(h, g, i);
                write_words(h, f.at(SCALE_UP), [scale, scale, scale, 1.0_f32.to_bits()]);
                set_value(h, f, g, 0x6c, SCALE_UP)?;
            }
            let desc = g.field(h, 0x68);
            if desc != 0 {
                if g.this.wrapping_add(0x54) == 0 {
                    return Err(1);
                }
                let mask: u16 = h.read(f.at(MASK));
                copy_parameter(h, g, desc, g.this.wrapping_add(0x54), mask)?;
            }
            add_pass(h, f, g, i, target)?;
        }
        Ok(())
    }

    /// The up v pass's `GaussianOutputScale` word: `+0x5c / (2 spread +
    /// 1)` (3 from level 3 on, which `min(+0x50, 3)` never reaches), 1 at
    /// level 0.
    fn up_scale(h: &impl Host, g: &Glow, i: u32) -> u32 {
        if i == 0 {
            return 1.0_f32.to_bits();
        }
        let q = if i >= 3 {
            3.0
        } else {
            let spread = x87::load(g.field(h, 0x58));
            x87::div(1.0, x87::add(x87::mul(spread, 2.0), 1.0))
        };
        let q = x87::store(q);
        let gain = x87::Reg::load(f32::from_bits(g.field(h, 0x5c)));
        x87::store(gain.mul_m32(f32::from_bits(q)).value())
    }

    /// The up v pass, added into `+0x84[i]` (the target at level 0, the
    /// back buffer when the target is 0).
    fn add_pass(h: &mut impl Host, f: &Frame, g: &Glow, i: u32, target: u32) -> Result<(), u32> {
        let r = g.renderer;
        ok(h.call_sym(SET_DEPTH, Conv::Thiscall(r), &[0]))?;
        let (into, surface) = match (i, target) {
            (0, 0) => (0, SURFACES[4]),
            (0, t) => (t, SURFACES[3]),
            _ => (g.target(h, 0x84, i), SURFACES[5]),
        };
        h.call_sym(SURFACE, Conv::Thiscall(f.at(surface)), &[into, 0]);
        ok(h.call_sym(SET_COLOR, Conv::Thiscall(r), &[f.at(surface), 0]))?;
        ok(h.call_sym(BEGIN_SCENE, Conv::Thiscall(r), &[0]))?;
        draw(h, g, PASS_ADD)?;
        h.call_sym(QUEUE18, Conv::Thiscall(r), &[0, 6, 7]);
        ok(h.call_sym(END_SCENE, Conv::Thiscall(r), &[1]))
    }
}
