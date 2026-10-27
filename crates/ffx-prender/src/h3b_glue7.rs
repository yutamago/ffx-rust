//! The render-side glue at 0x00690000 and up (M4, list `h3-glue-render`,
//! agent h3b, round 2): `ShadowManager_renderCharacterTiles_006caee0`,
//! the shadow manager's per-character shadow tiles (`PhyreScene_RenderShadow`
//! for every solution but 4): each 8 x 8 cell with casters is drawn from
//! its own camera into a tile of the atlas, the tiles are blurred into a
//! second target and copied back, and the tiles' rectangles are kept for
//! the next frame's clear.

/// The renderer's methods (thiscall on the renderer).
pub const SET_COLOR_TARGET: u32 = 0x005b_b130;
pub const SET_DEPTH_TARGET: u32 = 0x005b_b5b0;
pub const BEGIN_SCENE: u32 = 0x005b_5e30;
pub const END_SCENE: u32 = 0x005b_6950;
pub const SET_CLEAR_COLOR: u32 = 0x005b_ada0;
pub const SET_CLEAR_DEPTH: u32 = 0x005b_af10;
pub const SET_VIEWPORT: u32 = 0x005b_c060;
pub const SET_SCISSOR: u32 = 0x005b_bdc0;
pub const CLEAR_SCENE: u32 = 0x005b_6330;
pub const SET_CAMERA: u32 = 0x005b_abd0;
pub const RENDER_INSTANCE: u32 = 0x005b_9a90;
pub const RENDER_FOR_PASS: u32 = 0x005b_9a70;
/// `PRenderSurface::PRenderSurface(surface; texture, 0)` (its destructor is
/// a bare `ret`, left out).
pub const SURFACE_CTOR: u32 = 0x005b_35c0;
/// `PMatrix4_toWorldMatrix3x4(out; matrix)`.
pub const TO_WORLD: u32 = 0x0045_1190;
/// `Phyre_PCamera_updateViewMatrices(camera + 4)`.
pub const UPDATE_VIEW: u32 = 0x0067_20c0;
/// `ShaderParams_FindByName(shader; name)`,
/// `Render_SurfaceFormat_0066e4f0(buffer; parameter, &record)`,
/// `ShaderParamBuffer_SetValue(buffer; parameter, &value)`.
pub const FIND_PARAM: u32 = 0x0056_cbf0;
pub const WRITE_PARAM: u32 = 0x0066_e4f0;
pub const SET_VALUE: u32 = 0x0063_8780;
/// `PArray16_pushBack_006d17d0(array; &rect)`, `Phyre_PhyreFree`.
pub const PUSH_RECT: u32 = 0x006d_17d0;
pub const FREE: u32 = 0x0042_fb20;
/// `PostProcessManager_GetInstance()`.
pub const POST_PROCESS: u32 = 0x0069_2be0;
/// `atexit(handler)`, `memset`, `memcpy` (cdecl).
pub const ATEXIT: u32 = 0x0094_9207;
pub const ATEXIT_HANDLER: u32 = 0x00b0_72b0;
pub const MEMSET: u32 = 0x0094_964c;
pub const MEMCPY: u32 = 0x0094_9256;

/// The last frame's tile rectangles: a static `PArray` of 16-byte
/// `{x0, y0, x1, y1}` (count `+0`, top bit not owned; data `+4`) and its
/// guard word (bit 0: constructed, `atexit` registered).
pub const RECTS: u32 = 0x00cd_ec88;
pub const RECTS_GUARD: u32 = 0x00cd_ec90;
/// `g_ShadowSize`: the atlas size (8 tiles a side).
pub const SHADOW_SIZE: u32 = 0x00c3_4408;
/// The pass descriptors stored at the renderer's `+0x48` for the casters,
/// the blur and the copy back.
pub const PASS_CASTERS: u32 = 0x00c1_6a54;
pub const PASS_BLUR: u32 = 0x00cd_ec38;
pub const PASS_COPY: u32 = 0x00cd_ec48;
/// Constants: the camera's near plane (f32 0.1), 10.0 and 0.5 (f64).
pub const NEAR: u32 = 0x00b1_0f38;
pub const TEN: u32 = 0x00b9_22b0;
pub const HALF: u32 = 0x00b9_2220;
/// `"GaussianBlurBufferSize"`, `"OutputScale"`, `"UvScaleBias"`,
/// `"ColorBuffer"`.
pub const BLUR_SIZE: u32 = 0x00b4_94d8;
pub const OUTPUT_SCALE: u32 = 0x00b4_948c;
pub const UV_SCALE_BIAS: u32 = 0x00b4_25e4;
pub const COLOR_BUFFER: u32 = 0x00b4_25d8;

/// The frame (`ebp` offsets, below `ebp`): the bytes the port uses.
pub const FRAME: u32 = 0x188;
/// The three render surfaces (16 bytes each): the atlas, the blur
/// target, the atlas again.
pub const SURFACE_ATLAS: u32 = 0x188;
pub const SURFACE_BLUR: u32 = 0x168;
pub const SURFACE_BACK: u32 = 0x178;
/// The 3x4 world matrix `PMatrix4_toWorldMatrix3x4` fills.
pub const WORLD: u32 = 0x158;
/// The 64 drawn-tile flags (`ebp - 0x50 + cell`).
pub const DRAWN: u32 = 0x50;

pub mod bridge {
    //! The original over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    const MASK: u32 = 0x7fff_ffff;
    const ONE: u32 = 0x3f80_0000;

    fn call(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    /// `g_ShadowSize / 8` (towards zero): a tile's side.
    fn tile(h: &impl Host) -> i32 {
        h.read::<i32>(h.rebase(SHADOW_SIZE)) / 8
    }

    /// `fild` then `fstp dword`.
    fn int_f32(v: i32) -> u32 {
        x87::store(f64::from(v))
    }

    fn f32_at(h: &impl Host, at: u32) -> f64 {
        x87::load(h.read(at))
    }

    fn f64_at(h: &impl Host, at: u32) -> f64 {
        f64::from_bits(h.read(h.rebase(at)))
    }

    /// A parameter's bytes (`+8` offset, `+0xa` size & 0x1fff) copied from
    /// `from` into `buffer` (its size at `+0`) when they fit and the
    /// offset is at least 4; `false` when they don't. `ebp - 0x54` (or
    /// `slot`) keeps the offset.
    fn set_parameter(h: &mut impl Host, buffer: u32, p: u32, from: u32) -> bool {
        let offset = u32::from(h.read::<u16>(p + 8));
        let size = u32::from(h.read::<u16>(p + 0xa) & 0x1fff);
        if offset + size > h.read::<u32>(buffer) || offset < 4 {
            return false;
        }
        h.call_sym(
            MEMCPY,
            Conv::Cdecl,
            &[buffer.wrapping_add(offset), from, size],
        );
        true
    }

    /// A texture binding record `{0xff, 0, this + 0x3fc4, texture}` at
    /// `rec` for the surface object `surface` (`{texture, flag}`: none
    /// when the flag is set), written into the buffer.
    fn bind(h: &mut impl Host, this: u32, buffer: u32, p: u32, rec: u32, surface: u32) -> u32 {
        h.write(rec, 0xffu32);
        let texture = if h.read::<u8>(surface + 4) == 0 {
            h.read(surface)
        } else {
            0
        };
        h.write(rec + 8, this + 0x3fc4);
        h.write(rec + 0xc, texture);
        h.write(rec + 4, 0u32);
        call(h, WRITE_PARAM, buffer, &[p, rec])
    }

    /// Rectangle `r` of the list as `x, y, width, height`.
    fn rect(h: &impl Host, rects: u32, r: u32) -> [u32; 4] {
        let data: u32 = h.read(rects + 4);
        let at = data.wrapping_add(16 * r);
        let [x0, y0, x1, y1] = [0, 4, 8, 12].map(|k| h.read::<u32>(at.wrapping_add(k)));
        [x0, y0, x1.wrapping_sub(x0), y1.wrapping_sub(y0)]
    }

    /// Viewport and scissor rectangles of the last frame's tiles cleared,
    /// then the list emptied (its storage freed when owned).
    fn clear_rects(h: &mut impl Host, ebp: u32, renderer: u32, offset_slot: bool) -> u32 {
        let rects = h.rebase(RECTS);
        h.write(ebp - 0x68, 0u32);
        if h.read::<u32>(rects) & MASK != 0 {
            if offset_slot {
                h.write(ebp - 0x60, 0u32);
            }
            let mut r = 0u32;
            loop {
                let args = rect(h, rects, r);
                let err = call(h, SET_VIEWPORT, renderer, &args);
                if err != 0 {
                    return err;
                }
                let args = rect(h, rects, r);
                let err = call(h, SET_SCISSOR, renderer, &args);
                if err != 0 {
                    return err;
                }
                call(h, CLEAR_SCENE, renderer, &[1]);
                r += 1;
                h.write(ebp - 0x68, r);
                if offset_slot {
                    h.write(ebp - 0x60, 16 * r);
                }
                if r >= h.read::<u32>(rects) & MASK {
                    break;
                }
            }
        }
        let word = h.read::<u32>(rects) & 0x8000_0000;
        h.write(rects, word);
        if word == 0 {
            let data: u32 = h.read(rects + 4);
            h.call_sym(FREE, Conv::Cdecl, &[data]);
            h.write(rects + 4, 0u32);
        }
        0
    }

    /// `ShadowManager_renderCharacterTiles_006caee0(this; renderer,
    /// solution, keep)` (`ret 0xc`; frame at `ebp`, see the module's
    /// offsets): 0 at once while the Shadow setting (`+0x4060`) is 0.
    /// Without a cell in use (list `+0xde8` set, `+0xfe8`/`+0x10e8` 1, for
    /// solutions 2 and 3 cascade byte `+0x2b28` clear) only last frame's
    /// tiles are cleared. Else each such cell gets its camera (`+0x84`,
    /// from matrix `+0x17e8`; scaled by `+0x28e8`, near/far from
    /// `+0x6b8`/`+0x7b8`) and its casters (`+0xee8` instead unless
    /// `keep`) drawn into its tile; the tiles are blurred into `+0x70`'s
    /// surface (mesh `+0x8b8`) and copied back into `+0x5c`'s, recording
    /// their rectangles. The first renderer error, 3 when a blur parameter
    /// doesn't fit, or the final scissor's result.
    #[allow(clippy::too_many_lines)] // the original's sequence
    pub fn render_character_tiles(
        h: &mut impl Host,
        ebp: u32,
        this: u32,
        renderer: u32,
        solution: u32,
        keep: bool,
    ) -> u32 {
        h.write(ebp - 0x54, this);
        h.write(ebp - 0x6c, renderer);
        if h.read::<u32>(this + 0x4060) == 0 {
            return 0;
        }
        let guard = h.rebase(RECTS_GUARD);
        let word: u32 = h.read(guard);
        if word & 1 == 0 {
            h.write(guard, word | 1);
            let rects = h.rebase(RECTS);
            h.write(rects, 0u32);
            h.write(rects + 4, 0u32);
            let handler = h.rebase(ATEXIT_HANDLER);
            h.call_sym(ATEXIT, Conv::Cdecl, &[handler]);
        }
        h.write(ebp - DRAWN, 0u8);
        h.call_sym(MEMSET, Conv::Cdecl, &[ebp - DRAWN + 1, 0, 0x3f]);
        let atlas = ebp - SURFACE_ATLAS;
        let texture: u32 = h.read(this + 0x58);
        call(h, SURFACE_CTOR, atlas, &[texture, 0]);
        let err = call(h, SET_COLOR_TARGET, renderer, &[atlas, 0]);
        if err != 0 {
            return err;
        }
        let err = call(h, SET_DEPTH_TARGET, renderer, &[0]);
        if err != 0 {
            return err;
        }
        h.write(ebp - 0x71, 0u8);
        h.write(ebp - 0x88, this + 0xde8);
        let twin = solution == 2 || solution == 3;
        let mut any = false;
        for i in 0..8 {
            for j in 0..8 {
                let k = 8 * i + j;
                if h.read::<u32>(this + 0xde8 + 4 * k) != 0
                    && h.read::<u32>(this + 0xfe8 + 4 * k) == 1
                    && h.read::<u32>(this + 0x10e8 + 4 * k) == 1
                    && (!twin || h.read::<u8>(this + 0x2b28 + k) == 0)
                {
                    any = true;
                    h.write(ebp - 0x71, 1u8);
                    break;
                }
            }
        }
        if !any {
            if h.read::<u32>(h.rebase(RECTS)) & MASK == 0 {
                return 0;
            }
            let err = call(h, BEGIN_SCENE, renderer, &[0]);
            if err != 0 {
                return err;
            }
            call(h, SET_CLEAR_COLOR, renderer, &[0; 4]);
            let err = clear_rects(h, ebp, renderer, false);
            if err != 0 {
                return err;
            }
            return call(h, END_SCENE, renderer, &[1]);
        }
        call(h, SET_CLEAR_DEPTH, renderer, &[ONE]);
        let err = call(h, BEGIN_SCENE, renderer, &[0]);
        if err != 0 {
            return err;
        }
        call(h, SET_CLEAR_COLOR, renderer, &[0; 4]);
        let err = clear_rects(h, ebp, renderer, true);
        if err != 0 {
            return err;
        }
        let err = draw_casters(h, ebp, this, renderer, twin, keep);
        if err != 0 {
            return err;
        }
        let err = call(h, END_SCENE, renderer, &[1]);
        if err != 0 {
            return err;
        }
        blur(h, ebp, this, renderer)
    }

    /// Each cell's camera and casters into its tile (the tile flagged
    /// drawn).
    #[allow(clippy::too_many_lines)] // the original's sequence
    fn draw_casters(
        h: &mut impl Host,
        ebp: u32,
        this: u32,
        renderer: u32,
        twin: bool,
        keep: bool,
    ) -> u32 {
        h.write(ebp - 0x80, this + 0x17e8);
        h.write(ebp - 0x70, (ebp - DRAWN).wrapping_sub(0x3fa));
        h.write(ebp - 0x58, 0u32);
        h.write(ebp - 0x84, this + 0xde8);
        for i in 0..8u32 {
            h.write(ebp - 0x78, 0u32);
            h.write(ebp - 0x7c, this + 0xde8 + 0x20 * i);
            h.write(ebp - 0x64, this + 0x17e8 + 0x200 * i);
            for j in 0..8u32 {
                let k = 8 * i + j;
                let err = draw_cell(h, ebp, this, renderer, twin, keep, i, j, k);
                if err != 0 {
                    return err;
                }
                h.write(ebp - 0x78, j + 1);
                h.write(ebp - 0x7c, this + 0xde8 + 0x20 * i + 4 * (j + 1));
                h.write(ebp - 0x64, this + 0x17e8 + 0x200 * i + 0x40 * (j + 1));
            }
            h.write(ebp - 0x58, i + 1);
            h.write(ebp - 0x84, this + 0xde8 + 0x20 * (i + 1));
            h.write(ebp - 0x80, this + 0x17e8 + 0x200 * (i + 1));
        }
        0
    }

    /// Cell `k` (row `i`, column `j`) of [`draw_casters`].
    #[allow(clippy::too_many_arguments)] // the cell's coordinates
    fn draw_cell(
        h: &mut impl Host,
        ebp: u32,
        this: u32,
        renderer: u32,
        twin: bool,
        keep: bool,
        i: u32,
        j: u32,
        k: u32,
    ) -> u32 {
        let mut list: u32 = h.read(this + 0xde8 + 4 * k);
        h.write(ebp - 0x60, list);
        if list == 0
            || h.read::<u32>(this + 0xfe8 + 4 * k) != 1
            || h.read::<u32>(this + 0x10e8 + 4 * k) != 1
        {
            return 0;
        }
        let alt: u32 = h.read(this + 0xee8 + 4 * k);
        if alt != 0 && !keep {
            list = alt;
            h.write(ebp - 0x60, alt);
        }
        if h.read::<u32>(list) & MASK == 0 {
            return 0;
        }
        // the camera from the cell's matrix
        let data: u32 = h.read(list + 4);
        let first: u32 = h.read(data);
        let object: u32 = h.read(first.wrapping_add(0x1c));
        let world = ebp - WORLD;
        let matrix = this + 0x17e8 + 0x200 * i + 0x40 * j;
        call(h, TO_WORLD, world, &[matrix]);
        let to: u32 = h.read(this + 0x88);
        for w in 0..12 {
            let v = x87::store(f32_at(h, world + 4 * w));
            h.write(to.wrapping_add(4 * w), v);
        }
        let camera: u32 = h.read(this + 0x84);
        h.write(camera.wrapping_add(0xb4), to);
        let near = x87::store(f32_at(h, h.rebase(NEAR)));
        h.write(camera.wrapping_add(0xb8), near);
        changed(h, camera);
        let scale = x87::store(f32_at(h, object.wrapping_add(0x14)));
        h.write(ebp - 0x6c, scale);
        let v = f32_at(h, this + 0x28e8 + 4 * k);
        let t = x87::mul(x87::mul(v, v), f64_at(h, TEN));
        let t = x87::mul(x87::add(t, 1.0), x87::load(scale));
        let far = x87::store(t);
        h.write(ebp - 0x6c, far);
        let camera: u32 = h.read(this + 0x84);
        h.write(camera.wrapping_add(0xbc), x87::store(x87::load(far)));
        changed(h, camera);
        h.write(ebp - 0x68, k);
        let depth = f32_at(h, this + 0x7b8 + 4 * k);
        #[allow(clippy::float_cmp)] // fucompp against 0: only equality skips
        if depth != 0.0 {
            let ratio = x87::store(x87::div(f32_at(h, this + 0x6b8 + 4 * k), depth));
            h.write(ebp - 0x6c, ratio);
            let camera: u32 = h.read(this + 0x84);
            h.write(camera.wrapping_add(0x150), x87::store(x87::load(ratio)));
            changed(h, camera);
            let half = x87::store(x87::mul(depth, f64_at(h, HALF)));
            h.write(ebp - 0x6c, half);
            let camera: u32 = h.read(this + 0x84);
            h.write(camera.wrapping_add(0x154), x87::store(x87::load(half)));
            changed(h, camera);
            let camera: u32 = h.read(this + 0x84);
            call(h, UPDATE_VIEW, camera.wrapping_add(4), &[]);
        }
        if twin && h.read::<u8>(this + 0x2b28 + k) != 0 {
            return 0;
        }
        // the tile: viewport, scissor, camera, casters
        h.write((ebp - DRAWN).wrapping_add(k), 1u8);
        h.write(ebp - 0x68, k);
        h.write(ebp - 0x6c, k);
        for f in [SET_VIEWPORT, SET_SCISSOR] {
            let size = tile(h);
            let x = size.wrapping_mul(j as i32).wrapping_add(2) as u32;
            let y = size.wrapping_mul(i as i32).wrapping_add(2) as u32;
            let w: u32 = h.read(this + 0x4b8 + 4 * k);
            let hh: u32 = h.read(this + 0x5b8 + 4 * k);
            let err = call(h, f, renderer, &[x, y, w, hh]);
            if err != 0 {
                return err;
            }
        }
        let camera: u32 = h.read(this + 0x84);
        let view = if camera == 0 { 0 } else { camera + 4 };
        call(h, SET_CAMERA, renderer, &[view]);
        h.write(renderer + 0x48, h.rebase(PASS_CASTERS));
        let mut m = 0u32;
        while m < h.read::<u32>(list) & MASK {
            let data: u32 = h.read(list + 4);
            let instance: u32 = h.read(data.wrapping_add(4 * m));
            let materials: u32 = h.read(instance.wrapping_add(0x10));
            let pass: u32 = h.read(renderer + 0x13c);
            call(
                h,
                RENDER_INSTANCE,
                renderer,
                &[instance, materials, 0, pass],
            );
            m += 1;
        }
        0
    }

    /// The camera's virtual slot 1 (its `changed` notification).
    fn changed(h: &mut impl Host, camera: u32) {
        let vtable: u32 = h.read(camera);
        let f: u32 = h.read(vtable.wrapping_add(4));
        h.call(f, Conv::Thiscall(camera), &[]);
    }

    /// `ShaderParams_FindByName(shader; name)`.
    fn find(h: &mut impl Host, shader: u32, name: u32) -> u32 {
        let name = h.rebase(name);
        call(h, FIND_PARAM, shader, &[name])
    }

    /// The post-process manager's target texture (`+0x8c`: none when its
    /// flag is set).
    fn post_target(h: &mut impl Host) -> u32 {
        let post = h.call_sym(POST_PROCESS, Conv::Cdecl, &[]);
        let t: u32 = h.read(post.wrapping_add(0x8c));
        if h.read::<u8>(t.wrapping_add(4)) == 0 {
            h.read(t)
        } else {
            0
        }
    }

    /// The material of the blur mesh (`+0x8b8`): its first material
    /// (`PArray` at the mesh's `+0x10`).
    fn first_material(h: &impl Host, this: u32) -> u32 {
        let mesh: u32 = h.read(this + 0x8b8);
        let array: u32 = h.read(mesh.wrapping_add(0x10));
        let n = h.read::<u32>(array) & MASK;
        let data = match n {
            0 => 0,
            1 => array + 4,
            _ => h.read(array + 4),
        };
        h.read(data)
    }

    /// The blur parameter `GaussianBlurBufferSize`: the atlas size (a
    /// multiple of 8) twice, then 0, 0, at `rec` (the scratch words
    /// `- 0x54`, `- 0x64`, `- 0x70` as the original leaves them).
    fn blur_size(h: &mut impl Host, ebp: u32, buffer: u32, p: u32, rec: u32) -> bool {
        let size = h.read::<i32>(h.rebase(SHADOW_SIZE)) / 8 * 8;
        let f = int_f32(size);
        h.write(ebp - 0x64, f);
        h.write(ebp - 0x70, f);
        h.write(rec, f);
        h.write(rec + 4, f);
        let offset = u32::from(h.read::<u16>(p + 8));
        h.write(ebp - 0x54, offset);
        h.write(rec + 8, 0u32);
        h.write(rec + 0xc, 0u32);
        set_parameter(h, buffer, p, rec)
    }

    /// The tile's `UvScaleBias`: `{n / tiles, n / tiles, x / atlas, y /
    /// atlas}` (`n` the cell's size in tiles `+0x29e8`), each an x87
    /// division of the `f32` values.
    fn uv_scale_bias(size: i32, cells: i32, i: u32, j: u32) -> [u32; 3] {
        let atlas = x87::load(int_f32(size.wrapping_mul(8)));
        let span = x87::load(int_f32(cells.wrapping_mul(size)));
        let scale = x87::store(x87::div(span, atlas));
        let x = x87::load(int_f32(size.wrapping_mul(j as i32)));
        let y = x87::load(int_f32(size.wrapping_mul(i as i32)));
        let bias_x = x87::store(x87::div(x, atlas));
        let bias_y = x87::store(x87::div(y, atlas));
        [scale, bias_x, bias_y]
    }

    /// The blur into `+0x70`'s surface and the copy back into `+0x5c`'s
    /// (the drawn tiles only), the rectangles recorded; then the full
    /// screen.
    #[allow(clippy::too_many_lines)] // the original's sequence
    fn blur(h: &mut impl Host, ebp: u32, this: u32, renderer: u32) -> u32 {
        let material = first_material(h, this);
        let shader: u32 = h.read(material);
        h.write(ebp - 0x60, shader);
        let buffer: u32 = h.read(material.wrapping_add(4));
        h.write(ebp - 0x84, buffer);
        let size_p = find(h, shader, BLUR_SIZE);
        h.write(ebp - 0x7c, size_p);
        if size_p != 0 && !blur_size(h, ebp, buffer, size_p, ebp - 0x98) {
            return 3;
        }
        let p = find(h, shader, OUTPUT_SCALE);
        if p != 0 {
            for k in 0..4 {
                h.write(ebp - 0x128 + 4 * k, ONE);
            }
            let offset = u32::from(h.read::<u16>(p + 8));
            h.write(ebp - 0x54, offset);
            if !set_parameter(h, buffer, p, ebp - 0x128) {
                return 3;
            }
        }
        let uv_p = find(h, shader, UV_SCALE_BIAS);
        h.write(ebp - 0x78, uv_p);
        if uv_p != 0 {
            for (k, v) in [ONE, ONE, 0, 0].into_iter().enumerate() {
                h.write(ebp - 0xb8 + 4 * k as u32, v);
            }
            let offset = u32::from(h.read::<u16>(uv_p + 8));
            h.write(ebp - 0x54, offset);
            if !set_parameter(h, buffer, uv_p, ebp - 0xb8) {
                return 3;
            }
        }
        let color_p = find(h, shader, COLOR_BUFFER);
        h.write(ebp - 0x5c, color_p);
        if color_p != 0 {
            let surface: u32 = h.read(this + 0x58);
            let err = bind(h, this, buffer, color_p, ebp - 0xe8, surface);
            if err != 0 {
                return err;
            }
        }
        call(h, SET_CLEAR_COLOR, renderer, &[0; 4]);

        // the blur: each drawn tile through the blur mesh
        let target = ebp - SURFACE_BLUR;
        let texture: u32 = h.read(this + 0x70);
        call(h, SURFACE_CTOR, target, &[texture, 0]);
        let err = call(h, SET_COLOR_TARGET, renderer, &[target, 0]);
        if err != 0 {
            return err;
        }
        let err = call(h, SET_DEPTH_TARGET, renderer, &[0]);
        if err != 0 {
            return err;
        }
        let err = call(h, BEGIN_SCENE, renderer, &[0]);
        if err != 0 {
            return err;
        }
        h.write(ebp - 0x58, 0u32);
        h.write(ebp - 0x68, this + 0xde8);
        h.write(ebp - 0x6c, (ebp - DRAWN).wrapping_sub(0xa7a));
        for i in 0..8u32 {
            h.write(ebp - 0x80, 0u32);
            h.write(ebp - 0x60, this + 0xde8 + 0x20 * i);
            for j in 0..8u32 {
                let k = 8 * i + j;
                if h.read::<u32>(this + 0xde8 + 4 * k) != 0 {
                    h.write(ebp - 0x70, k);
                    let size = tile(h);
                    let cells: i32 = h.read(this + 0x29e8 + 4 * k);
                    let side = cells.wrapping_mul(size) as u32;
                    let x = size.wrapping_mul(j as i32) as u32;
                    let y = size.wrapping_mul(i as i32) as u32;
                    let err = call(h, SET_VIEWPORT, renderer, &[x, y, side, side]);
                    if err != 0 {
                        return err;
                    }
                    if h.read::<u8>((ebp - DRAWN).wrapping_add(k)) != 0 {
                        if uv_p != 0 {
                            let size = tile(h);
                            let cells: i32 = h.read(this + 0x29e8 + 4 * k);
                            let [scale, bias_x, bias_y] = uv_scale_bias(size, cells, i, j);
                            h.write(ebp - 0x64, bias_x);
                            h.write(ebp - 0x80, scale);
                            let rec = ebp - 0xd8;
                            for (w, v) in [scale, scale, bias_x, bias_y].into_iter().enumerate() {
                                h.write(rec + 4 * w as u32, v);
                            }
                            let offset = u32::from(h.read::<u16>(uv_p + 8));
                            h.write(ebp - 0x70, offset);
                            let n = u32::from(h.read::<u16>(uv_p + 0xa) & 0x1fff);
                            h.write(ebp - 0x54, n);
                            if !set_parameter(h, buffer, uv_p, rec) {
                                return 3;
                            }
                        }
                        h.write(renderer + 0x48, h.rebase(PASS_BLUR));
                        let mesh: u32 = h.read(this + 0x8b8);
                        let materials: u32 = h.read(mesh.wrapping_add(0x10));
                        let pass: u32 = h.read(renderer + 0x13c);
                        call(h, RENDER_INSTANCE, renderer, &[mesh, materials, 0, pass]);
                    }
                }
                h.write(ebp - 0x80, j + 1);
                h.write(ebp - 0x60, this + 0xde8 + 0x20 * i + 4 * (j + 1));
            }
            h.write(ebp - 0x58, i + 1);
            h.write(ebp - 0x68, this + 0xde8 + 0x20 * (i + 1));
        }
        let err = call(h, END_SCENE, renderer, &[1]);
        if err != 0 {
            return err;
        }

        // the copy back, the blurred target bound
        if size_p != 0 && !blur_size(h, ebp, buffer, size_p, ebp - 0xf8) {
            return 3;
        }
        if color_p != 0 {
            let surface: u32 = h.read(this + 0x70);
            let err = bind(h, this, buffer, color_p, ebp - 0x118, surface);
            if err != 0 {
                return err;
            }
        }
        if uv_p != 0 {
            for (k, v) in [ONE, ONE, 0, 0].into_iter().enumerate() {
                h.write(ebp - 0x108 + 4 * k as u32, v);
            }
            let err = call(h, SET_VALUE, buffer, &[uv_p, ebp - 0x108]);
            if err != 0 {
                return err;
            }
        }
        call(h, SET_CLEAR_COLOR, renderer, &[0; 4]);
        let back = ebp - SURFACE_BACK;
        let texture: u32 = h.read(this + 0x5c);
        call(h, SURFACE_CTOR, back, &[texture, 0]);
        let err = call(h, SET_COLOR_TARGET, renderer, &[back, 0]);
        if err != 0 {
            return err;
        }
        let err = call(h, SET_DEPTH_TARGET, renderer, &[0]);
        if err != 0 {
            return err;
        }
        let err = call(h, BEGIN_SCENE, renderer, &[0]);
        if err != 0 {
            return err;
        }
        let err = copy_back(h, ebp, this, renderer, buffer, uv_p);
        if err != 0 {
            return err;
        }
        let err = call(h, END_SCENE, renderer, &[1]);
        if err != 0 {
            return err;
        }

        // the full screen again
        let err = call(h, SET_COLOR_TARGET, renderer, &[0, 0]);
        if err != 0 {
            return err;
        }
        let err = call(h, SET_VIEWPORT, renderer, &[0, 0, 0x500, 0x2d0]);
        if err != 0 {
            return err;
        }
        let texture = post_target(h);
        let height: u32 = h.read(texture.wrapping_add(0x20));
        let texture = post_target(h);
        let width: u32 = h.read(texture.wrapping_add(0x1c));
        call(h, SET_SCISSOR, renderer, &[0, 0, width, height])
    }

    /// The copy back: each drawn tile from the blurred target, its
    /// rectangle pushed onto the static list.
    fn copy_back(
        h: &mut impl Host,
        ebp: u32,
        this: u32,
        renderer: u32,
        buffer: u32,
        uv_p: u32,
    ) -> u32 {
        h.write(ebp - 0x58, 0u32);
        for i in 0..8u32 {
            h.write(ebp - 0x60, 0u32);
            h.write(ebp - 0x68, this + 0xde8 + 0x20 * i);
            for j in 0..8u32 {
                let k = 8 * i + j;
                if h.read::<u32>(this + 0xde8 + 4 * k) != 0 {
                    h.write(ebp - 0x7c, k);
                    let size = tile(h);
                    let cells: i32 = h.read(this + 0x29e8 + 4 * k);
                    let side = (size / 2).wrapping_mul(cells) as u32;
                    let x = (j as i32).wrapping_mul(size) as u32;
                    let y = (i as i32).wrapping_mul(size) as u32;
                    let err = call(h, SET_VIEWPORT, renderer, &[x, y, side, side]);
                    if err != 0 {
                        return err;
                    }
                    if h.read::<u8>((ebp - DRAWN).wrapping_add(k)) != 0 {
                        if uv_p != 0 {
                            let size = tile(h);
                            let cells: i32 = h.read(this + 0x29e8 + 4 * k);
                            let [scale, bias_x, bias_y] = uv_scale_bias(size, cells, i, j);
                            h.write(ebp - 0x64, int_f32(size.wrapping_mul(8)));
                            h.write(ebp - 0x54, scale);
                            h.write(ebp - 0x70, scale);
                            h.write(ebp - 0x80, bias_x);
                            h.write(ebp - 0x5c, bias_y);
                            let rec = ebp - 0xa8;
                            for (w, v) in [scale, scale, bias_x, bias_y].into_iter().enumerate() {
                                h.write(rec + 4 * w as u32, v);
                            }
                            let err = call(h, SET_VALUE, buffer, &[uv_p, rec]);
                            if err != 0 {
                                return err;
                            }
                        }
                        h.write(renderer + 0x48, h.rebase(PASS_COPY));
                        let mesh: u32 = h.read(this + 0x8b8);
                        call(h, RENDER_FOR_PASS, renderer, &[mesh]);
                        let size = tile(h);
                        let cells: i32 = h.read(this + 0x29e8 + 4 * k);
                        let (i, j) = (i as i32, j as i32);
                        let rect = [
                            j.wrapping_mul(size),
                            i.wrapping_mul(size),
                            cells.wrapping_add(j).wrapping_mul(size),
                            i.wrapping_add(cells).wrapping_mul(size),
                        ];
                        for (w, v) in rect.into_iter().enumerate() {
                            h.write(ebp - 0xc8 + 4 * w as u32, v as u32);
                        }
                        let rects = h.rebase(RECTS);
                        call(h, PUSH_RECT, rects, &[ebp - 0xc8]);
                    }
                }
                h.write(ebp - 0x60, j + 1);
                h.write(ebp - 0x68, this + 0xde8 + 0x20 * i + 4 * (j + 1));
            }
            h.write(ebp - 0x58, i + 1);
            let row: u32 = h.read(ebp - 0x88);
            h.write(ebp - 0x88, row.wrapping_add(0x20));
        }
        0
    }
}
