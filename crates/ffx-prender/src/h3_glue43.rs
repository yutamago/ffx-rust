//! The render-side glue, forty-third batch (M4 port round 11, list
//! `h3-glue-render`): the scene's non-UI pre-render pass.

/// `Phyre_PRendering_PRendererBase_setCamera(renderer; camera)` (`ret
/// 4`), `Phyre_PCamera_updateViewMatrices(camera)`,
/// `PMatrix4_RowTimes_004c9c20(m; out, v)` (`ret 8`, `eax` the row
/// written), `PMatrix4_Copy_00453340(out; m)` (`ret 4`), the
/// application getter (cdecl) and `PApplication::renderWorld(app;
/// world, camera, pass)` (`ret 0xc`).
pub const SET_CAMERA: u32 = crate::h3_glue12::SET_CAMERA;
pub const UPDATE_VIEW_MATRICES: u32 = crate::h3_glue12::UPDATE_VIEW_MATRICES;
pub const ROW_TIMES: u32 = 0x004c_9c20;
pub const MATRIX_COPY: u32 = 0x0045_3340;
pub const GET_APPLICATION: u32 = crate::h3_glue12::GET_APPLICATION;
pub const RENDER_WORLD: u32 = crate::h3_glue12::RENDER_WORLD;

/// `g_PhyreScene3D` (a byte), `g_CurrentMapNo`, the techniques of the
/// pass (`preOpaque`, `preTransparent`, `preTransparentATC`), the
/// squeeze's `f32` y scale (4/3) and its `f64` offset (1/3).
pub const SCENE_3D: u32 = crate::h3_glue27::SCENE_3D;
pub const CURRENT_MAP: u32 = 0x012f_bc40;
pub const PRE_OPAQUE: u32 = 0x00c1_6a04;
pub const PRE_TRANSPARENT: u32 = 0x00c1_6a14;
pub const PRE_TRANSPARENT_ATC: u32 = 0x00c1_6a24;
pub const SQUEEZE_SCALE: u32 = 0x00b4_3d34;
pub const SQUEEZE_OFFSET: u32 = 0x00b4_3d40;

/// The maps whose transparent pre-pass is `preTransparent` (the others
/// take `preTransparentATC`).
pub const PLAIN_TRANSPARENT_MAPS: [u32; 4] = [0xa0, 0xa1, 0x101, 0x16c];

/// The 3D world pass: `PhyreScene_GetScreenAdjustedView_00651650(scene;
/// out)` (`ret 4`, `eax` the matrix), `setViewport(renderer; x, y, w,
/// h)` (`ret 0x10`), `PhyreScene_DrawMeshMatrices(scene; on)` (`ret 4`),
/// `PhyreScene_DrawZPrePass(scene)`; the techniques `Opaque`,
/// `OpaqueNoMSAA`, `Transparent`, `TransparentNoMSAA`.
pub const SCREEN_ADJUSTED_VIEW: u32 = 0x0065_1650;
pub const SET_VIEWPORT: u32 = 0x005b_c060;
pub const DRAW_MESH_MATRICES: u32 = 0x0066_b360;
pub const DRAW_Z_PREPASS: u32 = 0x0065_1070;
pub const OPAQUE: u32 = 0x00c1_69b4;
pub const OPAQUE_NO_MSAA: u32 = 0x00c1_69e4;
pub const TRANSPARENT: u32 = 0x00c1_69d4;
pub const TRANSPARENT_NO_MSAA: u32 = 0x00c1_69f4;

/// The texture-animation characters: `CharacterTextureAnimationTweakUtil
/// _GetInstance()`, `Phyre_PhyrePrintf(level, fmt, ...)` and its format,
/// the skip list `g_TexAnimSkip` (count, then 1-based characters),
/// `TextureAnimation_FindMaterial(this; anim, index, material)` (`ret
/// 0xc`), `ParallelArray_erase_006b2ca0(this; value)` (`ret 4`), the
/// `atoi` import slot and the target update `0x006b3660(this; value,
/// slot)` (`ret 8`).
pub const TWEAK_UTIL: u32 = 0x006b_6340;
pub const PHYRE_PRINTF: u32 = crate::glue5::PHYRE_PRINTF;
pub const SET_CHAR_FORMAT: u32 = 0x00b4_85d4;
pub const TEX_ANIM_SKIP: u32 = 0x00cc_b230;
pub const FIND_MATERIAL: u32 = 0x006b_1e40;
pub const ERASE: u32 = 0x006b_2ca0;
pub const ATOI_SLOT: u32 = crate::h3_glue33::ATOI;
pub const UPDATE_TARGET: u32 = 0x006b_3660;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use crate::h3_glue12::bridge::str_eq;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// An `fld dword` / `fstp dword` copy of one word.
    fn copy(h: &mut impl Host, to: u32, from: u32) {
        let v: u32 = h.read(from);
        h.write(to, x87::store(x87::load(v)));
    }

    /// `n` words copied one by one through the x87 stack.
    fn copy_words(h: &mut impl Host, to: u32, from: u32, n: u32) {
        for i in 0..n {
            copy(h, to + 4 * i, from + 4 * i);
        }
    }

    fn update_view(h: &mut impl Host, this: u32) {
        let cam: u32 = h.read(this + 0x7e0);
        h.call_sym(
            UPDATE_VIEW_MATRICES,
            Conv::Thiscall(cam.wrapping_add(4)),
            &[],
        );
    }

    /// `PMatrix4_RowTimes(m; out, v)`: the row's address.
    fn row_times(h: &mut impl Host, m: u32, out: u32, v: u32) -> u32 {
        h.call_sym(ROW_TIMES, Conv::Thiscall(m), &[out, v])
    }

    /// The matrix at `m` written as the original's x87 stack does: 1 on
    /// the diagonal except `y` (`m[5]`), 0 elsewhere but row 3's
    /// translation, stored in its order.
    fn diagonal(h: &mut impl Host, m: u32, y: u32) {
        let (zero, one) = (0u32, 1.0f32.to_bits());
        let mut words = [zero; 16];
        words[0] = one;
        words[5] = y;
        words[10] = one;
        words[15] = one;
        for (i, w) in words.iter().enumerate().take(12) {
            h.write(m + 4 * i as u32, *w);
        }
    }

    /// The camera's matrix squeezed: y scaled by 4/3 and moved by 1/3 -
    /// 2 `+0xf9bc`, row by row through the frame, copied back through
    /// `PMatrix4_Copy`; the view updated.
    fn squeeze(h: &mut impl Host, ebp: u32, this: u32) {
        let m = ebp - 0x44;
        let rows = h.read::<u32>(this + 0x7e0).wrapping_add(0x34);
        let scale = x87::store(x87::load(h.read(h.rebase(SQUEEZE_SCALE))));
        diagonal(h, m, scale);
        let s = x87::load(h.read(this + 0xf9bc));
        let offset = f64::from_bits(h.read(h.rebase(SQUEEZE_OFFSET)));
        let y = x87::store(x87::sub(offset, x87::add(s, s)));
        h.write(ebp - 0x54, y);
        h.write(m + 0x30, 0u32);
        copy(h, m + 0x34, ebp - 0x54);
        h.write(m + 0x38, 0u32);
        h.write(m + 0x3c, 1.0f32.to_bits());
        let p3 = row_times(h, m, ebp - 0xf8, rows.wrapping_add(0x30));
        h.write(ebp - 4, p3);
        let p2 = row_times(h, m, ebp - 0xe8, rows.wrapping_add(0x20));
        let p1 = row_times(h, m, ebp - 0x108, rows.wrapping_add(0x10));
        let p0 = row_times(h, m, ebp - 0x58, rows);
        copy_words(h, ebp - 0xd8, p0, 4);
        copy_words(h, ebp - 0xc8, p1, 4);
        copy_words(h, ebp - 0xb8, p2, 4);
        let p3: u32 = h.read(ebp - 4);
        copy_words(h, ebp - 0xa8, p3, 4);
        h.call_sym(MATRIX_COPY, Conv::Thiscall(m), &[ebp - 0xd8]);
        let this: u32 = h.read(ebp - 0x48);
        let cam: u32 = h.read(this + 0x7e0);
        copy_words(h, cam.wrapping_add(0x34), m, 16);
        h.call_sym(
            UPDATE_VIEW_MATRICES,
            Conv::Thiscall(cam.wrapping_add(4)),
            &[],
        );
    }

    /// `PhyreScene_DrawNonUIPrerenderScene(this)` (`ecx` only; frame at
    /// the original's offsets below `ebp`, which the matrix calls see:
    /// the matrix at `- 0x44`, the camera / last row at `- 4`, `this` at
    /// `- 0x48`, the rows at `- 0x58`, `- 0xe8`, `- 0xf8`, `- 0x108`, the
    /// saved camera matrix at `- 0x98`, the squeezed rows at `- 0xd8`).
    ///
    /// The camera (`+0x7e0`) goes to the renderer (`+0xd0`) and its view
    /// is updated; its matrix (`+0x34`, saved) is translated by
    /// (`+0xfa34`, `+0xfa38`) row by row; unless `+0xf9c7` or
    /// `g_PhyreScene3D`, squeezed again (y scale 4/3, y offset 1/3 - 2
    /// `+0xf9bc`) and copied through `PMatrix4_Copy`. World `+0x8c` is
    /// rendered with `preOpaque` (pass 1), then `preTransparent` or
    /// `preTransparentATC` by map (pass -1); the saved matrix is put
    /// back and the renderer gets camera `+0xdc`. Returns 0.
    pub fn draw_non_ui_prerender_scene(h: &mut impl Host, ebp: u32, this: u32) -> u32 {
        let m = ebp - 0x44;
        h.write(ebp - 0x48, this);
        let renderer: u32 = h.read(this + 0xd0);
        let cam4 = camera_at(h, this, 0x7e0);
        h.call_sym(SET_CAMERA, Conv::Thiscall(renderer), &[cam4]);
        update_view(h, this);

        // translate
        let cam: u32 = h.read(this + 0x7e0);
        h.write(ebp - 4, cam);
        diagonal(h, m, 1.0f32.to_bits());
        for i in 0..16 {
            let w: u32 = h.read(cam + 0x34 + 4 * i);
            h.write(ebp - 0x98 + 4 * i, w);
        }
        copy(h, ebp - 0x58, this + 0xfa34);
        copy(h, ebp - 0x54, this + 0xfa38);
        copy(h, m + 0x30, ebp - 0x58);
        copy(h, m + 0x34, ebp - 0x54);
        h.write(m + 0x38, 0u32);
        h.write(m + 0x3c, 1.0f32.to_bits());
        let r3 = row_times(h, m, ebp - 0x58, cam + 0x64);
        let cam: u32 = h.read(ebp - 4);
        let r2 = row_times(h, m, ebp - 0x108, cam + 0x54);
        let r1 = row_times(h, m, ebp - 0xe8, cam + 0x44);
        let cam: u32 = h.read(ebp - 4);
        let r0 = row_times(h, m, ebp - 0xf8, cam + 0x34);
        for (row, from) in [r0, r1, r2, r3].into_iter().enumerate() {
            copy_words(h, m + 0x10 * row as u32, from, 4);
        }
        let cam: u32 = h.read(ebp - 4);
        copy_words(h, cam + 0x34, m, 16);
        h.call_sym(
            UPDATE_VIEW_MATRICES,
            Conv::Thiscall(cam.wrapping_add(4)),
            &[],
        );

        let this: u32 = h.read(ebp - 0x48);
        let scene_3d: u8 = h.read(h.rebase(SCENE_3D));
        if h.read::<u8>(this + 0xf9c7) == 0 && scene_3d == 0 {
            squeeze(h, ebp, this);
        }

        // the two passes
        let this: u32 = h.read(ebp - 0x48);
        let renderer: u32 = h.read(this + 0xd0);
        let t = h.rebase(PRE_OPAQUE);
        h.write(renderer + 0x48, t);
        let cam4 = camera_at(h, this, 0x7e0);
        let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        h.call_sym(RENDER_WORLD, Conv::Thiscall(app), &[this + 0x8c, cam4, 1]);
        let map: u32 = h.read(h.rebase(CURRENT_MAP));
        let technique = if PLAIN_TRANSPARENT_MAPS.contains(&map) {
            PRE_TRANSPARENT
        } else {
            PRE_TRANSPARENT_ATC
        };
        let renderer: u32 = h.read(this + 0xd0);
        let t = h.rebase(technique);
        h.write(renderer + 0x48, t);
        let cam4 = camera_at(h, this, 0x7e0);
        let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        h.call_sym(
            RENDER_WORLD,
            Conv::Thiscall(app),
            &[this + 0x8c, cam4, u32::MAX],
        );

        // put the camera back
        let cam: u32 = h.read(this + 0x7e0);
        copy_words(h, cam.wrapping_add(0x34), ebp - 0x98, 16);
        h.call_sym(
            UPDATE_VIEW_MATRICES,
            Conv::Thiscall(cam.wrapping_add(4)),
            &[],
        );
        let next: u32 = h.read(this + 0xdc);
        let renderer: u32 = h.read(this + 0xd0);
        let next4 = if next == 0 { 0 } else { next + 4 };
        h.call_sym(SET_CAMERA, Conv::Thiscall(renderer), &[next4]);
        0
    }

    /// The camera at `this + field` as the renderer and the worlds take
    /// it (`+4`), or 0.
    fn camera_at(h: &impl Host, this: u32, field: u32) -> u32 {
        let cam: u32 = h.read(this + field);
        if cam == 0 { 0 } else { cam + 4 }
    }

    /// The renderer's (`+0xd0`) technique word `+0x48`.
    fn technique(h: &mut impl Host, this: u32, sym: u32) {
        let renderer: u32 = h.read(this + 0xd0);
        let t = h.rebase(sym);
        h.write(renderer + 0x48, t);
    }

    /// With MSAA on (the post-process manager's `+0x122b4`) `on`, else
    /// `off`.
    fn msaa_technique(h: &mut impl Host, this: u32, on: u32, off: u32) {
        let post: u32 = h.read(this + 0x10cc);
        let msaa: u8 = h.read(post + 0x122b4);
        technique(h, this, if msaa == 0 { off } else { on });
    }

    /// `PApplication::renderWorld(app; this + world, camera, pass)`.
    fn render_world(h: &mut impl Host, this: u32, world: u32, field: u32, pass: u32) {
        let cam = camera_at(h, this, field);
        let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        h.call_sym(
            RENDER_WORLD,
            Conv::Thiscall(app),
            &[this + world, cam, pass],
        );
    }

    /// The overlay world (`+0xbc`) is drawn in modes 1 and 3 (`+0xf9cc`).
    fn overlay(h: &mut impl Host, this: u32, field: u32) {
        let mode: u32 = h.read(this + 0xf9cc);
        if mode == 1 || mode == 3 {
            render_world(h, this, 0xbc, field, u32::MAX);
        }
    }

    /// `+0x7c`'s transparent pass, after the depth pre-pass and the
    /// technique when not split.
    fn back(h: &mut impl Host, this: u32, field: u32, pre: Option<bool>) {
        if pre == Some(true) {
            h.call_sym(DRAW_Z_PREPASS, Conv::Thiscall(this), &[]);
        }
        if pre.is_some() {
            technique(h, this, TRANSPARENT);
        }
        render_world(h, this, 0x7c, field, u32::MAX);
    }

    /// The worlds `+0xac` and `+0x7c` opaque (pass 1), then transparent
    /// (pass -1) with the overlay `+0xbc`, `+0xac` first when `+0xd4` is
    /// set, through the camera at `this + field`. `pre`: `None` for a
    /// split-screen half; else the mesh matrices are drawn on between
    /// the two and the depth pre-pass runs (when `Some(true)`) before
    /// `+0x7c`'s transparent pass.
    fn passes(h: &mut impl Host, this: u32, field: u32, pre: Option<bool>) {
        render_world(h, this, 0xac, field, 1);
        technique(h, this, OPAQUE);
        render_world(h, this, 0x7c, field, 1);
        match pre {
            None => technique(h, this, TRANSPARENT),
            Some(_) => {
                h.call_sym(DRAW_MESH_MATRICES, Conv::Thiscall(this), &[1]);
            }
        }
        if h.read::<u8>(this + 0xd4) == 0 {
            back(h, this, field, pre);
            msaa_technique(h, this, TRANSPARENT, TRANSPARENT_NO_MSAA);
            render_world(h, this, 0xac, field, u32::MAX);
            technique(h, this, TRANSPARENT);
            overlay(h, this, field);
        } else {
            msaa_technique(h, this, TRANSPARENT, TRANSPARENT_NO_MSAA);
            render_world(h, this, 0xac, field, u32::MAX);
            technique(h, this, TRANSPARENT);
            overlay(h, this, field);
            back(h, this, field, pre);
        }
    }

    /// `PhyreScene_Draw3DWorld(this, pre)` (`ret 4`; frame: the view at
    /// `ebp - 0x8c`, the matrix at `- 0x4c`, the viewport's x, y, w at
    /// `- 8`, `- 4`, `- 0xc` and its h in `pre`'s slot `+ 8`).
    ///
    /// The renderer (`+0xd0`) gets camera `+0xdc`; unless `+0xf9c7` or
    /// `g_PhyreScene3D`, the camera's matrix becomes the screen-adjusted
    /// view (`PMatrix4_Copy` into the frame, then copied). Split screen
    /// (`+0x10d8c` 1): each half of the renderer's viewport (`+8..+0x14`)
    /// draws [`passes`] with camera `+0xdc`, then `+0x238`, and the
    /// viewport is put back. Else the passes with the mesh matrices on
    /// (off at the end) and the depth pre-pass when `pre`'s low byte is
    /// set. Returns 0.
    pub fn draw_3d_world(h: &mut impl Host, ebp: u32, this: u32, pre: u32) -> u32 {
        let renderer: u32 = h.read(this + 0xd0);
        let cam = camera_at(h, this, 0xdc);
        h.call_sym(SET_CAMERA, Conv::Thiscall(renderer), &[cam]);
        let scene_3d: u8 = h.read(h.rebase(SCENE_3D));
        if h.read::<u8>(this + 0xf9c7) == 0 && scene_3d == 0 {
            let m = ebp - 0x4c;
            let view = h.call_sym(SCREEN_ADJUSTED_VIEW, Conv::Thiscall(this), &[ebp - 0x8c]);
            h.call_sym(MATRIX_COPY, Conv::Thiscall(m), &[view]);
            let cam = h.read::<u32>(this + 0xdc).wrapping_add(4);
            copy_words(h, cam.wrapping_add(0x30), m, 16);
            h.call_sym(UPDATE_VIEW_MATRICES, Conv::Thiscall(cam), &[]);
        }
        if h.read::<u32>(this + 0x10d8c) != 1 {
            msaa_technique(h, this, OPAQUE, OPAQUE_NO_MSAA);
            passes(h, this, 0xdc, Some(pre & 0xff != 0));
            h.call_sym(DRAW_MESH_MATRICES, Conv::Thiscall(this), &[0]);
            return 0;
        }
        let renderer: u32 = h.read(this + 0xd0);
        let x: u32 = h.read(renderer + 8);
        let y: u32 = h.read(renderer + 0xc);
        let w: u32 = h.read(renderer + 0x10);
        let ht: u32 = h.read(renderer + 0x14);
        h.write(ebp - 4, y);
        h.write(ebp + 8, ht);
        h.write(ebp - 8, x);
        h.write(ebp - 0xc, w);
        let half = w >> 1;
        for (field, left) in [(0xdc, x), (0x238, x.wrapping_add(half))] {
            let cam = camera_at(h, this, field);
            let renderer: u32 = h.read(this + 0xd0);
            h.call_sym(SET_CAMERA, Conv::Thiscall(renderer), &[cam]);
            let renderer: u32 = h.read(this + 0xd0);
            let (top, bottom): (u32, u32) = (h.read(ebp - 4), h.read(ebp + 8));
            h.call_sym(
                SET_VIEWPORT,
                Conv::Thiscall(renderer),
                &[left, top, half, bottom],
            );
            msaa_technique(h, this, OPAQUE, OPAQUE_NO_MSAA);
            passes(h, this, field, None);
        }
        let renderer: u32 = h.read(this + 0xd0);
        let (x, y, w, ht): (u32, u32, u32, u32) = (
            h.read(ebp - 8),
            h.read(ebp - 4),
            h.read(ebp - 0xc),
            h.read(ebp + 8),
        );
        h.call_sym(SET_VIEWPORT, Conv::Thiscall(renderer), &[x, y, w, ht]);
        0
    }

    /// A tagged string pointer (bit 0 a flag) matches `s`: the same
    /// pointer, or both set and equal.
    fn tagged_matches(h: &impl Host, field: u32, s: u32) -> bool {
        let word: u32 = h.read(field);
        let p = word - u32::from(h.read::<u8>(field) & 1);
        p == s || (p != 0 && s != 0 && str_eq(h, p, s))
    }

    /// The elements of a `PArray` at `a`: `+4`'s pointer for more than
    /// one, inline for one, none for 0.
    fn elements(h: &impl Host, a: u32) -> u32 {
        let n = h.read::<u32>(a) & 0x7fff_ffff;
        if n > 1 {
            h.read(a + 4)
        } else if n == 0 {
            0
        } else {
            a + 4
        }
    }

    /// `TextureAnimationManager_setAnimatedMaterialsChar(this, name,
    /// part, index, _, _, _, _, material, chr)` (`ret 0x24`; the four
    /// words between `index` and `material` are never read).
    ///
    /// Logs its call when the tweak utility's `+0x11` is 1. Nothing for
    /// a character in the skip list (`g_TexAnimSkip`, count first, 1-based)
    /// or not registered (`+0x70`'s 12-byte records, `+0x6c` of them,
    /// the character at `+4`). In the record's animation set (`+0x68`,
    /// 32-byte entries) the first 0x40-byte animation whose tagged name
    /// (`+0x20`) and part (`+4`) match is marked (`+0x34`); for each of
    /// its targets (`+0x38`) with mesh `+0x78` and material `+0x74` that
    /// the character has and `TextureAnimation_FindMaterial` finds, the
    /// record's matching target slot (`+8`, 8-byte entries) is cleared of
    /// the animation's frames (`+0x3c`) and of the frame number in
    /// `name + 1`, gets that number and a 0x14-byte entry (the material,
    /// `+0x2c`, `+0x30`, `+0x24`, `+0x28`), and `0x006b3660` updates it.
    pub fn set_animated_materials_char(
        h: &mut impl Host,
        this: u32,
        name: u32,
        part: u32,
        index: u32,
        material: u32,
        chr: u32,
    ) {
        let tweak = h.call_sym(TWEAK_UTIL, Conv::Cdecl, &[]);
        if h.read::<u8>(tweak + 0x11) == 1 {
            let fmt = h.rebase(SET_CHAR_FORMAT);
            h.call_sym(
                PHYRE_PRINTF,
                Conv::Cdecl,
                &[2, fmt, material, chr, name, part, index],
            );
        }
        let skip = h.rebase(TEX_ANIM_SKIP);
        let n: i32 = h.read(skip);
        let mut i = 1;
        while i <= n {
            if h.read::<u32>(skip + 4 * i as u32) == chr {
                return;
            }
            i += 1;
        }
        let count = h.read::<u32>(this + 0x6c) & 0x7fff_ffff;
        let list: u32 = h.read(this + 0x70);
        let Some(k) = (0..count).find(|&k| h.read::<u32>(list + 12 * k + 4) == chr) else {
            return;
        };
        let record = list + 12 * k;
        let slot = h.read::<u32>(record) << 5;
        let set = h.read::<u32>(this + 0x68).wrapping_add(slot);
        let anims = h.read::<u32>(set + 8) & 0x7fff_ffff;
        let first: u32 = h.read(set + 0xc);
        let Some(anim) = (0..anims)
            .map(|j| first.wrapping_add(0x40 * j))
            .find(|&e| tagged_matches(h, e + 0x20, name) && tagged_matches(h, e + 4, part))
        else {
            return;
        };
        h.write(anim + 0x34, 1u8);
        let slot = h.read::<u32>(record) << 5;
        if slot.wrapping_add(h.read(this + 0x68)) == 0 {
            return;
        }
        let mut t = 0u32;
        while t < h.read::<u32>(h.read::<u32>(anim + 0x38)) & 0x7fff_ffff {
            target(h, this, record, anim, t, (name, index, material, chr));
            t += 1;
        }
    }

    /// One target `t` of the animation (see
    /// [`set_animated_materials_char`]).
    fn target(
        h: &mut impl Host,
        this: u32,
        record: u32,
        anim: u32,
        t: u32,
        args: (u32, u32, u32, u32),
    ) {
        let (name, index, material, chr) = args;
        let targets: u32 = h.read(anim + 0x38);
        let item: u32 = h.read(h.read::<u32>(targets + 4) + 4 * t);
        let mat: u32 = h.read(item + 0x74);
        let mesh: u32 = h.read(item + 0x78);
        if (mat as i32) < 0 || (mesh as i32) < 0 {
            return;
        }
        let m: u32 = h.read(h.read::<u32>(chr + 4).wrapping_add(4 * mesh));
        let list = elements(h, h.read(m + 0x10));
        let value: u32 = h.read(list.wrapping_add(4 * mat));
        if value == 0 {
            return;
        }
        let found = h.call_sym(
            FIND_MATERIAL,
            Conv::Thiscall(this),
            &[anim, index, material],
        );
        if found == 0 {
            return;
        }
        let slots: u32 = h.read(record + 8);
        let n = h.read::<u32>(slots) & 0x7fff_ffff;
        let entries: u32 = h.read(slots + 4);
        let Some(s) = (0..n).find(|&s| {
            let e: u32 = h.read(entries + 8 * s);
            h.read::<u32>(e + 0x78) == mesh && h.read::<u32>(e + 0x74) == mat
        }) else {
            return;
        };
        let words = [0x2c, 0x30, 0x24, 0x28].map(|o| x87::store(x87::load(h.read(anim + o))));
        let mut f = 0u32;
        loop {
            let frames: u32 = h.read(anim + 0x3c);
            if f >= h.read::<u32>(frames) & 0x7fff_ffff {
                break;
            }
            let frame: u32 = h.read(h.read::<u32>(frames + 4) + 4 * f);
            let entries: u32 = h.read(h.read::<u32>(record + 8) + 4);
            let target: u32 = h.read(entries + 8 * s + 4);
            h.call_sym(ERASE, Conv::Thiscall(target), &[frame]);
            f += 1;
        }
        let entries: u32 = h.read(h.read::<u32>(record + 8) + 4);
        let atoi: u32 = h.read(h.rebase(ATOI_SLOT));
        let number = h.call(atoi, Conv::Cdecl, &[name.wrapping_add(1)]);
        let target: u32 = h.read(entries + 8 * s + 4);
        h.call_sym(ERASE, Conv::Thiscall(target), &[number]);
        let begin: u32 = h.read(target);
        let end: u32 = h.read(target + 8);
        if (end.wrapping_sub(begin) as i32 >> 2) >= h.read::<i32>(target + 0x10) {
            let oldest: u32 = h.read(begin);
            h.call_sym(ERASE, Conv::Thiscall(target), &[oldest]);
        }
        let end: u32 = h.read(target + 8);
        h.write(end, number);
        let rec: u32 = h.read(target + 0xc);
        let end: u32 = h.read(target + 8);
        h.write(target + 8, end.wrapping_add(4));
        h.write(rec, found);
        for (i, w) in words.into_iter().enumerate() {
            h.write(rec + 4 + 4 * i as u32, w);
        }
        let rec: u32 = h.read(target + 0xc);
        h.write(target + 0xc, rec.wrapping_add(0x14));
        let entries: u32 = h.read(h.read::<u32>(record + 8) + 4);
        h.call_sym(
            UPDATE_TARGET,
            Conv::Thiscall(this),
            &[value, entries + 8 * s],
        );
    }
}
