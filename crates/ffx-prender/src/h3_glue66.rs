//! The render-side glue, sixty-sixth batch (M4 port round 19, list
//! `h3-glue-render`): a 2D quad of the menus into the UI's vertex
//! buffers. The cookie is left out.

use ffx_bridge::Global;

pub use crate::h3_glue29::{CURRENT_BUFFER_ID, DYN_GEO_FILL};
pub use crate::h3_glue60::{
    ABMAP, BUTTON_DATA, NO_TEXTURE, REMAP_BUTTON_ICON, REMAP_INSTANCE, SKIP_FRAME,
};

/// `graphicDrawUIAbmapElement(quad, name, kind)` (cdecl).
pub const DRAW_UI_ABMAP_ELEMENT: u32 = 0x0063_e930;
/// `graphicUiElementRemap(quad, mode)` (cdecl).
pub const UI_ELEMENT_REMAP: u32 = 0x0064_4320;
/// `g_GraphicUi` and `MenuManager_GetMenuVertexInfo(ui; name, kind)`
/// (`ret 8`).
pub const GRAPHIC_UI: u32 = 0x00cc_c81c;
pub const MENU_VERTEX_INFO: u32 = 0x0067_ba50;
/// The ability map's untextured corners (f32: u 0.76953125 and
/// 0.8154297, v 0.5214844 and 0.58984375) and its colour scale (2).
pub const ABMAP_U0: Global<u32> = Global::new(0x00b4_1cb8);
pub const ABMAP_U1: Global<u32> = Global::new(0x00b4_1cbc);
pub const ABMAP_V0: Global<u32> = Global::new(0x00b4_1ca8);
pub const ABMAP_V1: Global<u32> = Global::new(0x00b4_1cac);
pub const ABMAP_COLOUR_SCALE: Global<f32> = Global::new(0x00b9_2268);

pub mod bridge {
    //! The original over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    const ONE: u32 = 0x3f80_0000;
    /// The screen scale (1920 / 512 and 1080 / 416) and offsets.
    const SX: f64 = 3.75;
    const SY: f64 = 2.596_153_736_114_502;
    const OX: f64 = 960.0;
    const OY: f64 = 540.0;

    /// The frame (`ebp -` these): the scratch qwords, the quad, the name
    /// (the remap's slot), the buffer (also a colour scratch), the f32
    /// scratch (first `kind`), and the local quad's four vertices
    /// (`0x20` bytes each from `ebp - 0x84`: x, y, u, 1 - v, RGBA).
    const SCRATCH_9C: u32 = 0x9c;
    const QUAD: u32 = 0x98;
    const NAME: u32 = 0x94;
    const SCRATCH_90: u32 = 0x90;
    const BUFFER: u32 = 0x8c;
    const SCRATCH_88: u32 = 0x88;
    const LOCAL: u32 = 0x84;

    fn f(h: &impl Host, at: u32) -> f64 {
        x87::load(h.read(at))
    }

    /// `fld`/`fstp` of a dword.
    fn copy(h: &mut impl Host, from: u32, to: u32) {
        let v = x87::store(f(h, from));
        h.write(to, v);
    }

    /// Word `field` of the local quad's vertex `k`.
    fn local(ebp: u32, k: u32, field: u32) -> u32 {
        ebp - LOCAL + 0x20 * k + 4 * field
    }

    /// `graphicDrawUIElement(quad, name, screen, kind, vertex_kind)`
    /// (cdecl, void; frame from `ebp - 0x9c`). Nothing on a skipped frame.
    /// With the ability map up (`g_GraphicAbmap`'s `+4` below 2), the
    /// quad is reshaped into its two-vertex-row form ([`abmap`]) and
    /// drawn by `graphicDrawUIAbmapElement`. Otherwise nothing when the
    /// button-icon remap draws it (it may replace the name in its frame
    /// slot); the UI's vertex record for the name (or `"NoTexture"`) gets
    /// the name, whether it has one and `vertex_kind`; the quad is
    /// remapped and laid out by `kind` ([`layout`]) and appended
    /// ([`append`]).
    pub fn draw_ui_element(
        h: &mut impl Host,
        ebp: u32,
        quad: u32,
        name: u32,
        screen: u32,
        kind: u32,
        vertex_kind: u32,
    ) {
        h.write(ebp - QUAD, quad);
        h.write(ebp - NAME, name);
        h.write(ebp - SCRATCH_88, vertex_kind);
        if h.read::<u32>(h.rebase(SKIP_FRAME)) != 0 {
            return;
        }
        let abmap: u32 = h.read(h.rebase(ABMAP));
        if abmap != 0 && h.read::<i32>(abmap.wrapping_add(4)) < 2 {
            abmap_element(h, ebp, quad, name);
            return;
        }
        let remap = h.call_sym(REMAP_INSTANCE, Conv::Cdecl, &[]);
        let buttons = h.rebase(BUTTON_DATA);
        let drawn = h.call_sym(
            REMAP_BUTTON_ICON,
            Conv::Thiscall(remap),
            &[buttons, buttons + 4, quad, ebp - NAME, kind],
        );
        if drawn != 0 {
            return;
        }
        let name: u32 = h.read(ebp - NAME);
        let ui: u32 = h.read(h.rebase(GRAPHIC_UI));
        let buffer = if name == 0 {
            let none = h.rebase(NO_TEXTURE);
            let info = h.call_sym(MENU_VERTEX_INFO, Conv::Thiscall(ui), &[none, vertex_kind]);
            let buffer: u32 = h.read(info.wrapping_add(0x94));
            h.write(buffer.wrapping_add(0x20), none);
            h.write(buffer, 0_u32);
            buffer
        } else {
            let info = h.call_sym(MENU_VERTEX_INFO, Conv::Thiscall(ui), &[name, vertex_kind]);
            let buffer: u32 = h.read(info.wrapping_add(0x94));
            h.write(info.wrapping_add(0xbc), 0_u32);
            let name: u32 = h.read(ebp - NAME);
            h.write(buffer.wrapping_add(0x20), name);
            h.write(buffer, 1_u32);
            buffer
        };
        let vertex_kind: u32 = h.read(ebp - SCRATCH_88);
        h.write(buffer.wrapping_add(0x28), vertex_kind);
        h.write(ebp - BUFFER, buffer);
        layout(h, ebp, quad, kind);
        append(h, ebp, buffer, screen, kind);
    }

    /// The ability map's form: without a name, the untextured corners and
    /// the colours doubled (`fild`, f32, times 2, truncated; the f32 kept
    /// at `ebp - 0x8c`). Then the quad's rows moved: vertex 2 and 3 from
    /// vertex 1 (and vertex 0's y), vertex 1 from vertex 0's x and u, the
    /// colours shifted down; drawn as kind 6 (named) or 11.
    fn abmap_element(h: &mut impl Host, ebp: u32, quad: u32, name: u32) {
        if name == 0 {
            for (off, g) in [
                (8, ABMAP_U0),
                (0x28, ABMAP_U1),
                (0xc, ABMAP_V0),
                (0x2c, ABMAP_V1),
            ] {
                let v = x87::store(x87::load(g.get(h)));
                h.write(quad + off, v);
            }
            let scale = f64::from(ABMAP_COLOUR_SCALE.get(h));
            for off in [0x10, 0x14, 0x18, 0x1c, 0x30, 0x34, 0x38, 0x3c] {
                let v = int_f32(h.read(quad + off));
                #[allow(clippy::cast_possible_truncation)] // fild of an int: exact
                h.write(ebp - BUFFER, v as f32);
                let r = x87::ftol(h, x87::mul(v, scale));
                h.write(quad + off, r);
            }
        }
        for (from, to) in [
            (4, 0x44),
            (0x20, 0x40),
            (0x20, 0x60),
            (0x24, 0x64),
            (0, 0x20),
            (0xc, 0x4c),
            (0x28, 0x48),
            (0x28, 0x68),
            (0x2c, 0x6c),
            (8, 0x28),
        ] {
            copy(h, quad + from, quad + to);
        }
        for off in [0x30, 0x34, 0x38, 0x3c] {
            let v: u32 = h.read(quad + off);
            h.write(quad + off + 0x40, v);
            h.write(quad + off + 0x20, v);
        }
        for off in [0x10, 0x14, 0x18, 0x1c] {
            let v: u32 = h.read(quad + off);
            h.write(quad + off + 0x20, v);
        }
        let (name, kind) = if name == 0 { (0, 0xb) } else { (name, 6) };
        h.call_sym(DRAW_UI_ABMAP_ELEMENT, Conv::Cdecl, &[quad, name, kind]);
    }

    /// The local quad from the remapped quad: kind 1 (or 6, 11, ..., as
    /// `kind % 5 == 1`) remaps with 1 and takes vertices 0, 1, 2, 2;
    /// kind 4 remaps with 4 and takes all four; others remap with 0 and
    /// take the corners of vertices 0 and 1 (x and u: 0, 1, 1, 0; y and
    /// v: 1, 1, 0, 0; colours 0, 1, 1, 0, or 1, 1, 0, 0 for kind 2).
    /// `v` is stored as `1 - v`.
    fn layout(h: &mut impl Host, ebp: u32, quad: u32, kind: u32) {
        #[allow(clippy::cast_possible_wrap)] // idiv
        let (mode, xu, yv, colours) = if (kind as i32) % 5 == 1 {
            (1, [0, 1, 2, 2], [0, 1, 2, 2], [0, 1, 2, 2])
        } else if kind == 4 {
            (4, [0, 1, 2, 3], [0, 1, 2, 3], [0, 1, 2, 3])
        } else if kind == 2 {
            (0, [0, 1, 1, 0], [1, 1, 0, 0], [1, 1, 0, 0])
        } else {
            (0, [0, 1, 1, 0], [1, 1, 0, 0], [0, 1, 1, 0])
        };
        h.call_sym(UI_ELEMENT_REMAP, Conv::Cdecl, &[quad, mode]);
        let q = |k: u32, field: u32| quad + 0x20 * k + 4 * field;
        for k in 0..4u32 {
            let (a, b, c) = (xu[k as usize], yv[k as usize], colours[k as usize]);
            copy(h, q(a, 0), local(ebp, k, 0));
            copy(h, q(b, 1), local(ebp, k, 1));
            copy(h, q(a, 2), local(ebp, k, 2));
            let v = x87::store(x87::sub(1.0, f(h, q(b, 3))));
            h.write(local(ebp, k, 3), v);
            for c4 in 0..4 {
                let w: u32 = h.read(q(c, 4 + c4));
                h.write(local(ebp, k, 4 + c4), w);
            }
        }
    }

    /// `fild` then `fstp dword`: rounded once to f32.
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_wrap)]
    fn int_f32(v: u32) -> f64 {
        f64::from(v as i32 as f32)
    }

    /// The local quad's vertices appended to the buffer: colours / 255,
    /// positions by `screen` and `kind` ([`position`]), normals (0, 0,
    /// 1), (u, 1 - v), six 16-bit indices (0, 1, 3, 3, 1, 2 on the count)
    /// at count * 1.5 (the count unsigned, f32, kept at `ebp - 0x88`);
    /// count += 4, total += 6, and both kept in the dynamic geometry's
    /// current buffer's slot.
    fn append(h: &mut impl Host, ebp: u32, buffer: u32, screen: u32, kind: u32) {
        let count: u32 = h.read(buffer.wrapping_add(4));
        let at = count << 2;
        for k in 0..4u32 {
            for c in 0..4u32 {
                let v = int_f32(h.read(local(ebp, k, 4 + c)));
                let v = x87::store(x87::div(v, 255.0));
                h.write(ebp - SCRATCH_88, v);
                let colours: u32 = h.read(buffer.wrapping_add(0x14));
                let word = at.wrapping_add(4 * k + c);
                h.write(colours.wrapping_add(word.wrapping_mul(4)), v);
            }
        }
        let count: u32 = h.read(buffer.wrapping_add(4));
        let at = count.wrapping_mul(3);
        let positions = position(h, ebp, screen, kind);
        for (k, (x, y)) in (0u32..).zip(positions) {
            for (c, v) in (0u32..).zip([x, y, ONE]) {
                let base: u32 = h.read(buffer.wrapping_add(0xc));
                let word = at.wrapping_add(3 * k + c);
                h.write(base.wrapping_add(word.wrapping_mul(4)), v);
            }
        }
        for word in 0..12u32 {
            let normals: u32 = h.read(buffer.wrapping_add(0x10));
            let v = if word % 3 == 2 { ONE } else { 0 };
            h.write(
                normals.wrapping_add(at.wrapping_add(word).wrapping_mul(4)),
                v,
            );
        }
        let count: u32 = h.read(buffer.wrapping_add(4));
        let at = count.wrapping_mul(2);
        for k in 0..4u32 {
            for c in 0..2u32 {
                let v = x87::store(f(h, local(ebp, k, 2 + c)));
                let uvs: u32 = h.read(buffer.wrapping_add(0x18));
                let word = at.wrapping_add(2 * k + c);
                h.write(uvs.wrapping_add(word.wrapping_mul(4)), v);
            }
        }
        let first: u32 = h.read(buffer.wrapping_add(4));
        #[allow(clippy::cast_precision_loss)] // fild (+ 2^32), fstp dword
        let n = first as f32;
        h.write(ebp - SCRATCH_88, n);
        let at = x87::ftol(h, x87::mul(f64::from(n), 1.5));
        #[allow(clippy::cast_possible_truncation)] // the original stores 16 bits
        let base = first as u16;
        for (k, d) in (0u32..).zip([0u16, 1, 3, 3, 1, 2]) {
            let indices: u32 = h.read(buffer.wrapping_add(0x1c));
            let off = at.wrapping_add(k).wrapping_mul(2);
            h.write(indices.wrapping_add(off), base.wrapping_add(d));
        }
        let count: u32 = h.read(buffer.wrapping_add(4));
        h.write(buffer.wrapping_add(4), count.wrapping_add(4));
        let total: u32 = h.read(buffer.wrapping_add(8));
        h.write(buffer.wrapping_add(8), total.wrapping_add(6));
        let fill: u32 = h.read(h.rebase(DYN_GEO_FILL));
        let id = h.call_sym(CURRENT_BUFFER_ID, Conv::Thiscall(fill), &[]);
        let slot = buffer.wrapping_add(id.wrapping_mul(28));
        let count: u32 = h.read(buffer.wrapping_add(4));
        h.write(slot.wrapping_add(0x2c), count);
        let total: u32 = h.read(buffer.wrapping_add(8));
        h.write(slot.wrapping_add(0x30), total);
    }

    /// The four screen positions (f32 bits; each also spilled to `ebp -
    /// 0x88`): with `screen`, x * 3.75 - 960 and y * 2.5961537 - 540;
    /// else kind 1 as they are, kind 3 less (960, 540), others scaled
    /// about their centres (x pairs 0-1 and 2-3, y pairs 0-3 and 1-2;
    /// the 0-3 pair's centre and half extent kept as qwords at `ebp -
    /// 0x90` and `- 0x9c`).
    fn position<H: Host>(h: &mut H, ebp: u32, screen: u32, kind: u32) -> [(u32, u32); 4] {
        let x = |h: &H, k| f(h, local(ebp, k, 0));
        let y = |h: &H, k| f(h, local(ebp, k, 1));
        let spill = |h: &mut H, v: f64| {
            let v = x87::store(v);
            h.write(ebp - SCRATCH_88, v);
            v
        };
        let mut out = [(0, 0); 4];
        if screen != 0 || kind == 3 {
            let (sx, sy) = if screen != 0 { (SX, SY) } else { (1.0, 1.0) };
            for (k, p) in (0u32..).zip(out.iter_mut()) {
                let xv = x(h, k);
                let xv = if screen != 0 { x87::mul(xv, sx) } else { xv };
                let px = spill(h, x87::sub(xv, OX));
                let yv = y(h, k);
                let yv = if screen != 0 { x87::mul(yv, sy) } else { yv };
                let py = spill(h, x87::sub(yv, OY));
                *p = (px, py);
            }
            return out;
        }
        if kind == 1 {
            for (k, p) in (0u32..).zip(out.iter_mut()) {
                let px = x87::store(x(h, k));
                let py = x87::store(y(h, k));
                *p = (px, py);
            }
            return out;
        }
        let half = |a: f64, b: f64| x87::mul(x87::add(a, b), 0.5);
        let extent = |a: f64, b: f64, s: f64| x87::mul(x87::mul(x87::sub(a, b), s), 0.5);
        let (x0, x1) = (x(h, 0), x(h, 1));
        let mx = half(x1, x0);
        let dx = extent(x1, x0, SX);
        out[0].0 = spill(h, x87::sub(mx, dx));
        let (y0, y3) = (y(h, 0), y(h, 3));
        let my = half(y3, y0);
        h.write(ebp - SCRATCH_90, my);
        let dy = extent(y3, y0, SY);
        h.write(ebp - SCRATCH_9C, dy);
        out[0].1 = spill(h, x87::sub(my, dy));
        out[1].0 = spill(h, x87::add(mx, dx));
        let (y1, y2) = (y(h, 1), y(h, 2));
        let my2 = half(y1, y2);
        let dy2 = extent(y1, y2, SY);
        out[1].1 = spill(h, x87::add(dy2, my2));
        let (x2, x3) = (x(h, 2), x(h, 3));
        let mx2 = half(x3, x2);
        let dx2 = extent(x3, x2, SX);
        out[2].0 = spill(h, x87::sub(mx2, dx2));
        out[2].1 = spill(h, x87::sub(my2, dy2));
        out[3].0 = spill(h, x87::add(mx2, dx2));
        let dy: f64 = h.read(ebp - SCRATCH_9C);
        let my: f64 = h.read(ebp - SCRATCH_90);
        out[3].1 = spill(h, x87::add(dy, my));
        out
    }
}
