//! The render-side glue at 0x00690000 and up (M4, list `h3-glue-render`,
//! agent h3b, round 2): `ShadowManager_renderCascades_006c9e50`, the
//! shadow manager's cascaded shadow maps (`PhyreScene_RenderShadow` for
//! solutions 2, 3 and 4): the cascades' caster rectangles, each cascade's
//! casters drawn from its light camera into its map, blurred through a
//! second target (the grad meshes of solution 4) and kept for the next
//! frame's clear.

pub use crate::h3b_glue7::{
    BEGIN_SCENE, BLUR_SIZE, CLEAR_SCENE, COLOR_BUFFER, END_SCENE, FIND_PARAM, FREE, MEMCPY, NEAR,
    OUTPUT_SCALE, PASS_BLUR, PASS_CASTERS, PASS_COPY, POST_PROCESS, RENDER_INSTANCE, SET_CAMERA,
    SET_CLEAR_COLOR, SET_CLEAR_DEPTH, SET_COLOR_TARGET, SET_DEPTH_TARGET, SET_SCISSOR,
    SET_VIEWPORT, SHADOW_SIZE, SURFACE_CTOR, TO_WORLD, UV_SCALE_BIAS, WRITE_PARAM,
};

/// `PMatrix4_Copy_00453340(out; m)`.
pub const MATRIX_COPY: u32 = 0x0045_3340;
/// `ShadowManager_syncCascadeMatrices_006c9780(this)`.
pub const SYNC_CASCADES: u32 = 0x006c_9780;
/// `ShadowManager_casterCrossesEdge_006c8f50(this; view 4x4 by value,
/// flag, ratio, scale_x, scale_y)` (`ret 0x50`, twenty words): `al`.
pub const CROSSES_EDGE: u32 = 0x006c_8f50;
/// `ShadowManager_buildCasterRects_006c88a0(this; i, scale_x, scale_y)`.
pub const BUILD_RECTS: u32 = 0x006c_88a0;
/// `ShadowManager_setupGradMeshesA/B` (`this; i, meshes, extras, scale,
/// unused, depth`).
pub const GRAD_MESHES_A: u32 = 0x006d_12f0;
pub const GRAD_MESHES_B: u32 = 0x006d_1530;
/// `ShadowCascadeView_init_006c7cc0(view; m3x4, matrix, near, far)`.
pub const VIEW_INIT: u32 = 0x006c_7cc0;
/// The grad-mesh pool's current set (toggled 0/1 for solution 4).
pub const GRAD_MESH_SET: u32 = 0x00cd_ec24;
/// Ratios handed to the edge test and the grad meshes (f32 globals and
/// the 0.95 constant), the light camera's far plane (f32 60.0).
pub const RATIO_A: u32 = 0x00cd_ec2c;
pub const RATIO_B: u32 = 0x00b4_9954;
pub const RATIO_C: u32 = 0x00cd_ec34;
pub const FAR: u32 = 0x00b3_eb9c;
/// The pass descriptor of solution 4's grad meshes.
pub const PASS_GRAD: u32 = 0x00cd_ec58;
/// The x87 control word the game runs with (53-bit, round to nearest):
/// the original keeps `cw | 0xc00` (chop) in its frame around `fistp`.
pub const GAME_CW: u32 = 0x027f;

/// The frame (`ebp` offsets, below `ebp`): the bytes the port uses.
pub const FRAME: u32 = 0x290;
/// The cascade's light view (`ShadowCascadeView`; its `+0xcc` word is
/// freed after each cascade unless bit 0 is set).
pub const VIEW: u32 = 0x290;
/// The cascade's 3x4 world matrix.
pub const WORLD: u32 = 0x144;
/// The surfaces: the clear path's, the cascade's map, its blur target's
/// copy and the blur target.
pub const SURFACE_CLEAR: u32 = 0x114;
pub const SURFACE_MAP: u32 = 0x104;
pub const SURFACE_BACK: u32 = 0xf4;
pub const SURFACE_BLUR: u32 = 0xe4;

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

    fn int_f32(v: i32) -> u32 {
        x87::store(f64::from(v))
    }

    fn copy_f32(h: &impl Host, at: u32) -> u32 {
        x87::store(x87::load(h.read(at)))
    }

    /// `g_ShadowSize - 64` as an f32 (`fild`, `fstp dword`), left in
    /// `ebp - 0x14` and `ebp - 0x10` as the original does before each
    /// call that takes it.
    fn inner_size(h: &mut impl Host, ebp: u32) -> u32 {
        let size = h.read::<i32>(h.rebase(SHADOW_SIZE)).wrapping_sub(0x40);
        let f = int_f32(size);
        h.write(ebp - 0x14, f);
        h.write(ebp - 0x10, f);
        f
    }

    /// A parameter's place: (`+8` offset, `+0xa` size & 0x1fff).
    fn place(h: &impl Host, p: u32) -> (u32, u32) {
        let offset = u32::from(h.read::<u16>(p + 8));
        (offset, u32::from(h.read::<u16>(p + 0xa) & 0x1fff))
    }

    /// The parameter's bytes copied from `from` into `buffer` (its size
    /// at `+0`) when they fit and the offset is at least 4; `false` when
    /// they don't. The callers store the frame words the original stores
    /// before its `memcpy` first: the copy may read them.
    fn copy_parameter(
        h: &mut impl Host,
        buffer: u32,
        (offset, size): (u32, u32),
        from: u32,
    ) -> bool {
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

    /// A texture binding record `{0xff, 0, this + 0x3fe8, texture}` at
    /// `rec` for the surface object `surface`, written into the buffer.
    fn bind(h: &mut impl Host, this: u32, buffer: u32, p: u32, rec: u32, surface: u32) -> u32 {
        h.write(rec, 0xffu32);
        let texture = if h.read::<u8>(surface + 4) == 0 {
            h.read(surface)
        } else {
            0
        };
        h.write(rec + 0xc, texture);
        h.write(rec + 8, this + 0x3fe8);
        h.write(rec + 4, 0u32);
        call(h, WRITE_PARAM, buffer, &[p, rec])
    }

    /// The edge test of view matrix `m` (copied through the x87, as the
    /// by-value argument is built), `flag` and the ratio global.
    fn crosses<H: Host>(
        h: &mut H,
        ebp: u32,
        this: u32,
        m: u32,
        flag: u32,
        ratio: u32,
        crosses_edge: &mut impl FnMut(&mut H, u32, &[u32; 20]) -> u32,
    ) -> bool {
        let size = inner_size(h, ebp);
        let ratio = copy_f32(h, h.rebase(ratio));
        let mut words = [0u32; 20];
        for (k, w) in words.iter_mut().take(16).enumerate() {
            *w = copy_f32(h, this + m + 4 * k as u32);
        }
        words[16] = flag;
        words[17] = ratio;
        words[18] = size;
        words[19] = size;
        crosses_edge(h, this, &words) & 0xff != 0
    }

    /// `ShadowManager_renderCascades_006c9e50(this; renderer, solution,
    /// keep)` (`ret 0xc`; frame at `ebp`, see the module's offsets; it
    /// writes its `renderer` slot). 0 at once while the Shadow setting
    /// (`+0x4060`) is 0. With `+0x400c` set only the map (`+0x5c`) is
    /// cleared. Else (without `keep`) the cascades are checked against
    /// their edges (solution 4; `crosses_edge` makes the twenty-word call)
    /// or synced, their matrices saved when `+0x404c` says so, their caster
    /// rectangles built (and solution 4's grad meshes set up) and each
    /// cascade drawn, blurred and copied back. The first renderer error,
    /// 3 when a blur parameter doesn't fit, or the final scissor's result.
    pub fn render_cascades<H: Host>(
        h: &mut H,
        ebp: u32,
        this: u32,
        renderer: u32,
        solution: u32,
        keep: u8,
        crosses_edge: &mut impl FnMut(&mut H, u32, &[u32; 20]) -> u32,
    ) -> u32 {
        h.write(ebp - 0x48, this);
        if h.read::<u32>(this + 0x4060) == 0 {
            return 0;
        }
        if h.read::<u8>(this + 0x400c) != 0 {
            h.write(this + 0x400c, 0u8);
            return clear_map(h, ebp, this, renderer);
        }
        if keep == 0 && h.read::<u8>(this + 0x404c) == 0 {
            // solution 4: the cascades' views against their edges
            let hit = solution == 4
                && [
                    (0x210, 0, RATIO_A),
                    (0x250, 0, RATIO_B),
                    (0x430, 1, RATIO_A),
                    (0x470, 1, RATIO_B),
                ]
                .into_iter()
                .any(|(m, flag, ratio)| crosses(h, ebp, this, m, flag, ratio, crosses_edge));
            if hit {
                h.write(this + 0x404c, 1u8);
            } else {
                call(h, SYNC_CASCADES, this, &[]);
            }
        }
        if h.read::<u8>(this + 0x404c) != 0 {
            h.write(this + 0x404c, 0u8);
            for t in 0..2 {
                let at = this + 0x2b0 + 0x40 * t;
                for (to, from) in [(0, 0x220), (0x80, 0x1a0), (0x100, 0x120), (0x180, 0xa0)] {
                    call(h, MATRIX_COPY, at + to, &[at - from]);
                }
            }
        }
        let size = inner_size(h, ebp);
        call(h, BUILD_RECTS, this, &[0, size, size]);
        let mut cascades = 1;
        h.write(ebp - 0x50, 1u32);
        if solution == 4 && keep == 0 {
            let set = h.rebase(GRAD_MESH_SET);
            let flip = u32::from(h.read::<u32>(set) == 0);
            h.write(ebp - 0x50, 2u32);
            cascades = 2;
            h.write(set, flip);
            for block in [0x8bc, 0x9bc] {
                for k in 0..16 {
                    h.write(this + block + 4 * k, 0u32);
                }
            }
            let size = inner_size(h, ebp);
            call(h, BUILD_RECTS, this, &[1, size, size]);
            for (f, i, meshes, extras, ratio) in [
                (GRAD_MESHES_A, 0, 0x8bc, 0xabc, RATIO_A),
                (GRAD_MESHES_A, 1, 0x9bc, 0xbbc, RATIO_B),
                (GRAD_MESHES_B, 1, 0x9bc, 0xbbc, RATIO_C),
            ] {
                let size = inner_size(h, ebp);
                let ratio = copy_f32(h, h.rebase(ratio));
                call(
                    h,
                    f,
                    this,
                    &[i, this + meshes, this + extras, ratio, size, size],
                );
            }
        }
        h.write(ebp - 0x24, 0u32);
        for c in 0..cascades {
            let err = cascade(h, ebp, this, renderer, solution, keep, c);
            let view_text: u32 = h.read(ebp - VIEW + 0xcc);
            if view_text & 1 == 0 {
                h.call_sym(FREE, Conv::Cdecl, &[view_text]);
            }
            if err != 0 {
                return err;
            }
            h.write(ebp - 0x24, c + 1);
        }
        for k in 0..16 {
            h.write(this + 0xdc4 + k, 0u8);
            h.write(this + 0xdd4 + k, 0u8);
        }
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

    /// The `+0x400c` path: the map's surface cleared once.
    fn clear_map(h: &mut impl Host, ebp: u32, this: u32, renderer: u32) -> u32 {
        call(h, SET_CLEAR_COLOR, renderer, &[0; 4]);
        let surface = ebp - SURFACE_CLEAR;
        let texture: u32 = h.read(this + 0x5c);
        call(h, SURFACE_CTOR, surface, &[texture, 0]);
        let err = call(h, SET_COLOR_TARGET, renderer, &[surface, 0]);
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
        call(h, CLEAR_SCENE, renderer, &[1]);
        let err = call(h, END_SCENE, renderer, &[1]);
        if err != 0 {
            return err;
        }
        call(h, SET_COLOR_TARGET, renderer, &[0, 0])
    }

    /// The post-process manager's target texture (`+0x8c`).
    fn post_target(h: &mut impl Host) -> u32 {
        let post = h.call_sym(POST_PROCESS, Conv::Cdecl, &[]);
        let t: u32 = h.read(post.wrapping_add(0x8c));
        if h.read::<u8>(t.wrapping_add(4)) == 0 {
            h.read(t)
        } else {
            0
        }
    }

    /// Last frame's rectangles of cascade `c` (`+0x37b0 + 0x400 c`, count
    /// `+0x3fb0`) cleared; `count_at` keeps the count, `ptr_at` the walk.
    fn clear_rects(
        h: &mut impl Host,
        this: u32,
        renderer: u32,
        c: u32,
        count_at: u32,
        ptr_at: u32,
    ) -> u32 {
        let mut n = 0i32;
        let mut rect = this + 0x37b4 + 0x400 * c;
        if h.read::<i32>(this + 0x3fb0 + 4 * c) <= 0 {
            return 0;
        }
        h.write(ptr_at, rect);
        loop {
            let [x0, y0, x1, y1] =
                [-4i32, 0, 4, 8].map(|k| h.read::<u32>(rect.wrapping_add(k as u32)));
            let args = [
                x0.wrapping_add(0x20),
                y0.wrapping_add(0x20),
                x1.wrapping_sub(x0),
                y1.wrapping_sub(y0),
            ];
            let err = call(h, SET_VIEWPORT, renderer, &args);
            if err != 0 {
                return err;
            }
            call(h, CLEAR_SCENE, renderer, &[1]);
            n += 1;
            rect += 0x10;
            h.write(count_at, n as u32);
            h.write(ptr_at, rect);
            if n >= h.read::<i32>(this + 0x3fb0 + 4 * c) {
                return 0;
            }
        }
    }

    /// The map's material (the blur mesh `+0x8b8`'s first): (shader,
    /// parameter buffer).
    fn blur_material(h: &impl Host, this: u32) -> (u32, u32) {
        let mesh: u32 = h.read(this + 0x8b8);
        let array: u32 = h.read(mesh.wrapping_add(0x10));
        let n = h.read::<u32>(array) & MASK;
        let first = match n {
            0 => 0,
            1 => array + 4,
            _ => h.read(array + 4),
        };
        let material: u32 = h.read(first);
        let shader: u32 = h.read(material);
        (shader, h.read(material.wrapping_add(4)))
    }

    /// `ShaderParams_FindByName(shader; name)`.
    fn find(h: &mut impl Host, shader: u32, name: u32) -> u32 {
        let name = h.rebase(name);
        call(h, FIND_PARAM, shader, &[name])
    }

    /// A rectangle's `UvScaleBias`: `{w / a, h / b, x / a, y / b}`, each an
    /// x87 division of the `f32` values (`x`, `y` past the 32-texel
    /// border).
    fn uv(a: u32, b: u32, [x, y, w, hh]: [u32; 4]) -> [u32; 4] {
        let (a, b) = (x87::load(a), x87::load(b));
        let f = |v: u32| x87::load(int_f32(v as i32));
        let x = x87::store(x87::div(f(x.wrapping_add(0x20)), a));
        let y = x87::store(x87::div(f(y.wrapping_add(0x20)), b));
        let w = x87::store(x87::div(f(w), a));
        let hh = x87::store(x87::div(f(hh), b));
        [w, hh, x, y]
    }

    /// The renderer's colour target `surface`, no depth target, a scene
    /// begun: the first error.
    fn target(h: &mut impl Host, renderer: u32, surface: u32) -> u32 {
        let err = call(h, SET_COLOR_TARGET, renderer, &[surface, 0]);
        if err != 0 {
            return err;
        }
        let err = call(h, SET_DEPTH_TARGET, renderer, &[0]);
        if err != 0 {
            return err;
        }
        call(h, BEGIN_SCENE, renderer, &[0])
    }

    /// `renderMeshInstance(renderer; mesh, its materials, 0, the pass)`.
    fn render_mesh(h: &mut impl Host, renderer: u32, mesh: u32) {
        let pass: u32 = h.read(renderer + 0x13c);
        let materials: u32 = h.read(mesh.wrapping_add(0x10));
        call(h, RENDER_INSTANCE, renderer, &[mesh, materials, 0, pass]);
    }

    /// The cascade's casters: each cell in use whose cascade byte
    /// (`+0x2b28 + 0x40 c`) is set, drawn from the light view.
    fn draw_casters(h: &mut impl Host, ebp: u32, this: u32, renderer: u32, keep: u8, c: u32) {
        h.write(ebp - 0x10, 0u32);
        for i in 0..8u32 {
            h.write(ebp + 8, 0u32);
            h.write(ebp - 0x2c, this + 0xde8 + 0x20 * i);
            for j in 0..8u32 {
                let k = 8 * i + j;
                let mut list: u32 = h.read(this + 0xde8 + 4 * k);
                if list != 0
                    && h.read::<u32>(this + 0xfe8 + 4 * k) == 1
                    && h.read::<u32>(this + 0x10e8 + 4 * k) == 1
                    && h.read::<u8>(this + 0x2b28 + 0x40 * c + k) != 0
                {
                    let alt: u32 = h.read(this + 0xee8 + 4 * k);
                    if alt != 0 && keep == 0 {
                        list = alt;
                    }
                    if h.read::<u32>(list) & MASK != 0 {
                        call(h, SET_CAMERA, renderer, &[ebp - VIEW]);
                        h.write(renderer + 0x48, h.rebase(PASS_CASTERS));
                        let mut m = 0;
                        loop {
                            let data: u32 = h.read(list + 4);
                            let instance: u32 = h.read(data.wrapping_add(4 * m));
                            render_mesh(h, renderer, instance);
                            m += 1;
                            if m >= h.read::<u32>(list) & MASK {
                                break;
                            }
                        }
                    }
                }
                h.write(ebp + 8, j + 1);
                h.write(ebp - 0x2c, this + 0xde8 + 4 * (k + 1));
            }
            h.write(ebp - 0x10, i + 1);
        }
    }

    /// Cascade `c`: its light view, its casters into its map (`+0x60 +
    /// 4c`), the blur into `+0x6c + 4c` and back, the rectangles kept.
    #[allow(clippy::too_many_lines)] // the original's sequence
    fn cascade(
        h: &mut impl Host,
        ebp: u32,
        this: u32,
        renderer: u32,
        solution: u32,
        keep: u8,
        c: u32,
    ) -> u32 {
        let world = ebp - WORLD;
        call(h, TO_WORLD, world, &[this + 0x90]);
        let far = copy_f32(h, h.rebase(FAR));
        let near = copy_f32(h, h.rebase(NEAR));
        h.write(ebp - 0x4c, c << 6);
        let view = ebp - VIEW;
        call(
            h,
            VIEW_INIT,
            view,
            &[world, this + 0x110 + 0x40 * c, near, far],
        );
        let map = ebp - SURFACE_MAP;
        let texture: u32 = h.read(this + 0x60 + 4 * c);
        call(h, SURFACE_CTOR, map, &[texture, 0]);
        let err = call(h, SET_COLOR_TARGET, renderer, &[map, 0]);
        if err != 0 {
            return err;
        }
        let err = call(h, SET_DEPTH_TARGET, renderer, &[0]);
        if err != 0 {
            return err;
        }
        call(h, SET_CLEAR_COLOR, renderer, &[0; 4]);
        call(h, SET_CLEAR_DEPTH, renderer, &[ONE]);
        let err = call(h, BEGIN_SCENE, renderer, &[0]);
        if err != 0 {
            return err;
        }
        // the `renderer` slot counts, then holds the sizes
        h.write(ebp + 8, 0u32);
        let err = clear_rects(h, this, renderer, c, ebp + 8, ebp - 0x28);
        if err != 0 {
            return err;
        }
        let atlas = int_f32(h.read(h.rebase(SHADOW_SIZE)));
        h.write(ebp - 0x34, atlas);
        h.write(ebp - 0x28, atlas);
        let inner = h.read::<i32>(h.rebase(SHADOW_SIZE)).wrapping_sub(0x40);
        let f = int_f32(inner);
        h.write(ebp - 0x10, f);
        // fistp under the chop control word: the f32 truncated to 64 bits
        #[allow(clippy::cast_possible_truncation)] // an integral f32
        let side = f64::from(f32::from_bits(f)) as i64;
        h.write(ebp - 0x20, side as u32);
        h.write(ebp - 0x1c, (side >> 32) as u32);
        h.write(ebp - 0x44, side as u32);
        h.write(ebp - 0x14, GAME_CW | 0xc00);
        h.write(ebp + 8, side as u32);
        let side = side as u32;
        for f in [SET_VIEWPORT, SET_SCISSOR] {
            let err = call(h, f, renderer, &[0x20, 0x20, side, side]);
            if err != 0 {
                return err;
            }
        }
        draw_casters(h, ebp, this, renderer, keep, c);
        let err = call(h, END_SCENE, renderer, &[1]);
        if err != 0 {
            return err;
        }

        // the blur's parameters
        let (shader, buffer) = blur_material(h, this);
        h.write(ebp - 0x10, shader);
        h.write(ebp + 8, buffer);
        let size_p = find(h, shader, BLUR_SIZE);
        h.write(ebp - 0x44, size_p);
        let scale_p = find(h, shader, OUTPUT_SCALE);
        h.write(ebp - 0x38, scale_p);
        let uv_p = find(h, shader, UV_SCALE_BIAS);
        h.write(ebp - 0x14, uv_p);
        let color_p = find(h, shader, COLOR_BUFFER);
        h.write(ebp - 0x3c, color_p);
        if scale_p != 0 {
            for k in 0..4 {
                h.write(ebp - 0x64 + 4 * k, ONE);
            }
            let place = place(h, scale_p);
            h.write(ebp - 0x30, place.0);
            if !copy_parameter(h, buffer, place, ebp - 0x64) {
                return 3;
            }
        }
        if color_p != 0 {
            let surface: u32 = h.read(this + 0x60 + 4 * c);
            let err = bind(h, this, buffer, color_p, ebp - 0x84, surface);
            if err != 0 {
                return err;
            }
        }
        let back = ebp - SURFACE_BACK;
        let texture: u32 = h.read(this + 0x60 + 4 * c);
        call(h, SURFACE_CTOR, back, &[texture, 0]);
        call(h, SET_CLEAR_COLOR, renderer, &[0; 4]);
        let blur = ebp - SURFACE_BLUR;
        let texture: u32 = h.read(this + 0x6c + 4 * c);
        call(h, SURFACE_CTOR, blur, &[texture, 0]);
        let err = target(h, renderer, blur);
        if err != 0 {
            return err;
        }
        h.write(ebp - 0x2c, 0u32);
        let err = clear_rects(h, this, renderer, c, ebp - 0x2c, ebp - 0x10);
        if err != 0 {
            return err;
        }

        // the blur: each caster rectangle through the blur mesh
        let a = copy_f32(h, ebp - 0x34);
        let b = copy_f32(h, ebp - 0x28);
        h.write(ebp - 0x54, 0u32);
        let rects = this + 0x2fa8 + 0x400 * c;
        let count_at = this + 0x37a8 + 4 * c;
        if h.read::<i32>(count_at) > 0 {
            let mut n = 0u32;
            h.write(ebp - 0x30, rects + 4);
            loop {
                let rect = rects + 16 * n;
                let [x0, y0, x1, y1] = [0, 4, 8, 12].map(|k| h.read::<u32>(rect + k));
                let (w, hh) = (x1.wrapping_sub(x0), y1.wrapping_sub(y0));
                h.write(ebp - 0x10, w);
                h.write(ebp - 0x2c, hh);
                if size_p != 0 {
                    for (k, v) in (0..).zip([a, b, 0, 0]) {
                        h.write(ebp - 0xd4 + 4 * k, v);
                    }
                    let place = place(h, size_p);
                    h.write(ebp - 0x40, place.1);
                    h.write(ebp - 0x18, place.0);
                    if !copy_parameter(h, buffer, place, ebp - 0xd4) {
                        return 3;
                    }
                }
                if uv_p != 0 {
                    let [sw, sh, sx, sy] = uv(a, b, [x0, y0, w, hh]);
                    let place = place(h, uv_p);
                    h.write(ebp - 0x40, sx);
                    h.write(ebp - 0x18, place.0);
                    h.write(ebp - 0x1c, place.1);
                    for (k, v) in (0..).zip([sw, sh, sx, sy]) {
                        h.write(ebp - 0xa4 + 4 * k, v);
                    }
                    if !copy_parameter(h, buffer, place, ebp - 0xa4) {
                        return 3;
                    }
                }
                let args = [x0.wrapping_add(0x20), y0.wrapping_add(0x20), w, hh];
                let err = call(h, SET_VIEWPORT, renderer, &args);
                if err != 0 {
                    return err;
                }
                h.write(renderer + 0x48, h.rebase(PASS_BLUR));
                let mesh: u32 = h.read(this + 0x8b8);
                render_mesh(h, renderer, mesh);
                n += 1;
                h.write(ebp - 0x54, n);
                h.write(ebp - 0x30, rect + 0x14);
                if n as i32 >= h.read::<i32>(count_at) {
                    break;
                }
            }
        }
        let err = call(h, END_SCENE, renderer, &[1]);
        if err != 0 {
            return err;
        }

        // the copy back into the map
        if scale_p != 0 {
            for k in 0..4 {
                h.write(ebp - 0xc4 + 4 * k, ONE);
            }
            let place = place(h, scale_p);
            h.write(ebp - 0x1c, place.0);
            if !copy_parameter(h, buffer, place, ebp - 0xc4) {
                return 3;
            }
        }
        if color_p != 0 {
            let surface: u32 = h.read(this + 0x6c + 4 * c);
            let err = bind(h, this, buffer, color_p, ebp - 0x94, surface);
            if err != 0 {
                return err;
            }
        }
        call(h, SET_CLEAR_COLOR, renderer, &[0; 4]);
        let err = target(h, renderer, back);
        if err != 0 {
            return err;
        }
        h.write(ebp - 0x10, 0u32);
        if h.read::<i32>(count_at) > 0 {
            let mut n = 0u32;
            h.write(ebp - 0x30, rects);
            loop {
                let rect = rects + 16 * n;
                let [x0, y0, x1, y1] = [0, 4, 8, 12].map(|k| h.read::<u32>(rect + k));
                let (w, hh) = (x1.wrapping_sub(x0), y1.wrapping_sub(y0));
                h.write(ebp - 0x38, w);
                h.write(ebp - 0x3c, hh);
                if size_p != 0 {
                    for (k, v) in (0..).zip([a, b, 0, 0]) {
                        h.write(ebp - 0xb4 + 4 * k, v);
                    }
                    let place = place(h, size_p);
                    h.write(ebp - 0x1c, place.1);
                    h.write(ebp - 0x18, place.0);
                    if !copy_parameter(h, buffer, place, ebp - 0xb4) {
                        return 3;
                    }
                }
                if uv_p != 0 {
                    let [sw, sh, sx, sy] = uv(a, b, [x0, y0, w, hh]);
                    let place = place(h, uv_p);
                    h.write(ebp - 0x40, sy);
                    h.write(ebp - 0x18, place.0);
                    h.write(ebp - 0x1c, place.1);
                    for (k, v) in (0..).zip([sw, sh, sx, sy]) {
                        h.write(ebp - 0x74 + 4 * k, v);
                    }
                    if !copy_parameter(h, buffer, place, ebp - 0x74) {
                        return 3;
                    }
                }
                let args = [x0.wrapping_add(0x20), y0.wrapping_add(0x20), w, hh];
                let err = call(h, SET_VIEWPORT, renderer, &args);
                if err != 0 {
                    return err;
                }
                let default: u32 = h.read(this + 0x8b8);
                let grad: u32 = h.read(this + 0x8bc + 4 * (0x40 * c + n));
                let (pass, mesh) = if solution == 4 && keep == 0 && grad != default {
                    (PASS_GRAD, grad)
                } else {
                    (PASS_COPY, default)
                };
                h.write(renderer + 0x48, h.rebase(pass));
                render_mesh(h, renderer, mesh);
                let kept = this + 0x37b0 + 0x400 * c + 16 * n;
                for k in (0..16).step_by(4) {
                    let v: u32 = h.read(rect + k);
                    h.write(kept + k, v);
                }
                n += 1;
                h.write(ebp - 0x10, n);
                h.write(ebp - 0x30, rect + 0x10);
                if n as i32 >= h.read::<i32>(count_at) {
                    break;
                }
            }
        }
        let n: u32 = h.read(count_at);
        h.write(this + 0x3fb0 + 4 * c, n);
        call(h, END_SCENE, renderer, &[1])
    }
}
