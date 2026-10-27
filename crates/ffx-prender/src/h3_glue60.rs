//! The render-side glue, sixtieth batch (M4 port round 17, list
//! `h3-glue-render`): a 2D quad of the sphere-grid screens into the
//! ability map's vertex buffers.

/// `g_GraphicSkipFrame`.
pub const SKIP_FRAME: u32 = 0x012f_b7d0;
/// `RemapButtonIcon_sGetInstance()` (cdecl) and
/// `RemapButtonIcon_remapButtonIcon(remap; &g_ButtonData[0],
/// &g_ButtonData[1], quad, &name, kind)` (`ret 0x14`).
pub const REMAP_INSTANCE: u32 = 0x006e_7710;
pub const REMAP_BUTTON_ICON: u32 = 0x006e_69e0;
pub const BUTTON_DATA: u32 = 0x00c5_955c;
/// `g_GraphicAbmap`, `AbmapManager_GetVertexInfo(manager; name, kind)`
/// (`ret 8`) and `AbmapManager_CheckWordVertex(manager; x, y2, y)` (`ret
/// 0xc`).
pub const ABMAP: u32 = 0x00cc_c838;
pub const VERTEX_INFO: u32 = 0x0068_4cd0;
pub const CHECK_WORD_VERTEX: u32 = 0x0068_43a0;
/// `strstr`'s import slot.
pub const STRSTR_SLOT: u32 = 0x00b0_c378;
/// `"NoTexture"` and the names the quad's texture is picked by: "face",
/// `"keyboard_icon"`, `"pad_icon"`, "icon", "stone", "mes".
pub const NO_TEXTURE: u32 = 0x00b4_1b50;
pub const NAMES: [u32; 6] = [
    0x00b4_1b5c,
    0x00b4_1b64,
    0x00b4_1b74,
    0x00b4_1b80,
    0x00b4_1b88,
    0x00b4_1b90,
];
/// The stone texture's corners (f32: 0.37890625, 0.4951172, 0.62890625,
/// 0.7451172).
pub const STONE_V0: u32 = 0x00b4_1ca0;
pub const STONE_U0: u32 = 0x00b4_1ca4;
pub const STONE_V1: u32 = 0x00b4_1cb0;
pub const STONE_U1: u32 = 0x00b4_1cb4;

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
    /// The "mes" texture's shift.
    const MES_DU: f64 = 0.534_179_687_5;
    const MES_DV: f64 = 0.156_25;

    fn f(h: &impl Host, at: u32) -> f64 {
        x87::load(h.read(at))
    }

    /// `graphicDrawUIAbmapElement(quad, name, kind)` (cdecl; `slots` =
    /// the address of `quad`'s argument word). Nothing on a skipped frame
    /// or when the button-icon remap draws it (the remap may replace
    /// `name` through its slot). Picks the vertex buffer: kinds 6 and 11
    /// by `name` (the texture's own, or 0xd8, 0x144, 0x6c on for "face",
    /// `"keyboard_icon"`, "icon"; "stone" checks the word vertex and sets
    /// the stone corners; "mes" shifts the texture), others `"NoTexture"`'s.
    /// Appends the quad's four vertices (0x20 bytes each: x, y, u, v,
    /// RGBA as ints): colours / 255, positions on the 1920x1080 screen
    /// centred, normals (0, 0, 1), (u, 1 - v), and two triangles' 16-bit
    /// indices at count * 1.5.
    pub fn draw_ui_abmap_element(h: &mut impl Host, slots: u32, quad: u32, kind: u32) {
        if h.read::<u32>(h.rebase(SKIP_FRAME)) != 0 {
            return;
        }
        let remap = h.call_sym(REMAP_INSTANCE, Conv::Cdecl, &[]);
        let buttons = h.rebase(BUTTON_DATA);
        let drawn = h.call_sym(
            REMAP_BUTTON_ICON,
            Conv::Thiscall(remap),
            &[buttons, buttons + 4, quad, slots + 4, kind],
        );
        if drawn != 0 {
            return;
        }
        let buffer = if kind == 6 || kind == 0xb {
            named_buffer(h, slots, quad, kind)
        } else {
            let abmap: u32 = h.read(h.rebase(ABMAP));
            let name = h.rebase(NO_TEXTURE);
            let info = h.call_sym(VERTEX_INFO, Conv::Thiscall(abmap), &[name, kind]);
            h.read(info.wrapping_add(0x94))
        };
        append(h, quad, buffer);
    }

    /// `strstr(name, NAMES[k]) != 0`.
    fn contains(h: &mut impl Host, slots: u32, k: usize) -> bool {
        let strstr: u32 = h.read(h.rebase(STRSTR_SLOT));
        let name: u32 = h.read(slots + 4);
        let needle = h.rebase(NAMES[k]);
        h.call(strstr, Conv::Cdecl, &[name, needle]) != 0
    }

    /// The vertex buffer of a named quad (kinds 6 and 11).
    fn named_buffer(h: &mut impl Host, slots: u32, quad: u32, kind: u32) -> u32 {
        let abmap: u32 = h.read(h.rebase(ABMAP));
        let name: u32 = h.read(slots + 4);
        let info = h.call_sym(VERTEX_INFO, Conv::Thiscall(abmap), &[name, kind]);
        let own: u32 = h.read(info.wrapping_add(0x94));
        if h.read::<u32>(slots + 4) == 0 {
            return own;
        }
        if contains(h, slots, 0) {
            return shifted(h, info, 0xd8);
        }
        if contains(h, slots, 1) {
            return shifted(h, info, 0x144);
        }
        if contains(h, slots, 2) {
            return own;
        }
        if contains(h, slots, 3) {
            return shifted(h, info, 0x6c);
        }
        if contains(h, slots, 4) {
            stone(h, quad);
        } else if contains(h, slots, 5) {
            mes(h, quad);
        }
        own
    }

    /// The vertex info's own buffer, `off` on.
    fn shifted(h: &impl Host, info: u32, off: u32) -> u32 {
        h.read::<u32>(info.wrapping_add(0x94)).wrapping_add(off)
    }

    /// The word vertex checked at the quad's first y, its third y and its
    /// second x on screen; the stone texture's corners.
    fn stone(h: &mut impl Host, quad: u32) {
        let y = x87::sub(x87::mul(f(h, quad + 4), SY), OY);
        let a = x87::ftol(h, y);
        let y2 = x87::sub(x87::mul(SY, f(h, quad + 0x24)), OY);
        let b = x87::ftol(h, y2);
        let x = x87::sub(x87::mul(f(h, quad + 0x20), SX), OX);
        let c = x87::ftol(h, x);
        let abmap: u32 = h.read(h.rebase(ABMAP));
        h.call_sym(CHECK_WORD_VERTEX, Conv::Thiscall(abmap), &[c, b, a]);
        let [u0, v0, u1, v1]: [u32; 4] =
            [STONE_U0, STONE_V0, STONE_U1, STONE_V1].map(|at| h.read(h.rebase(at)));
        for (off, v) in [
            (0x8, u0),
            (0xc, v0),
            (0x28, u1),
            (0x2c, v0),
            (0x48, u0),
            (0x4c, v1),
            (0x6c, v1),
            (0x68, u1),
        ] {
            h.write(quad + off, v);
        }
    }

    /// The "mes" texture: u + 0.5341797, v - 0.15625 at each corner.
    fn mes(h: &mut impl Host, quad: u32) {
        for k in 0..4 {
            let at = quad + 0x20 * k;
            let u = x87::add(f(h, at + 8), MES_DU);
            h.write(at + 8, x87::store(u));
            let v = x87::sub(f(h, at + 0xc), MES_DV);
            h.write(at + 0xc, x87::store(v));
        }
    }

    /// `fild` then `fstp dword`: rounded once to f32.
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_wrap)]
    fn int_f32(v: u32) -> f64 {
        f64::from(v as i32 as f32)
    }

    /// The quad's vertices appended to the buffer.
    fn append(h: &mut impl Host, quad: u32, buffer: u32) {
        let count: u32 = h.read(buffer.wrapping_add(4));
        // colours
        let at = count << 2;
        let mut k = 0u32;
        for vertex in 0..4u32 {
            for c in 0..4u32 {
                let v: u32 = h.read(quad + 0x20 * vertex + 0x10 + 4 * c);
                let v = x87::store(x87::div(int_f32(v), 255.0));
                let colours: u32 = h.read(buffer.wrapping_add(0x14));
                h.write(colours.wrapping_add(at.wrapping_add(k).wrapping_mul(4)), v);
                k += 1;
            }
        }
        // positions and normals
        let count: u32 = h.read(buffer.wrapping_add(4));
        let at = count.wrapping_mul(3);
        for vertex in 0..4u32 {
            let x = x87::sub(x87::mul(f(h, quad + 0x20 * vertex), SX), OX);
            let y = x87::sub(x87::mul(f(h, quad + 0x20 * vertex + 4), SY), OY);
            for (c, v) in [x87::store(x), x87::store(y), ONE].into_iter().enumerate() {
                let positions: u32 = h.read(buffer.wrapping_add(0xc));
                let word = at.wrapping_add(3 * vertex + c as u32);
                h.write(positions.wrapping_add(word.wrapping_mul(4)), v);
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
        // texture coordinates
        let count: u32 = h.read(buffer.wrapping_add(4));
        let at = count.wrapping_mul(2);
        for vertex in 0..4u32 {
            let u: u32 = h.read(quad + 0x20 * vertex + 8);
            let v = x87::sub(1.0, f(h, quad + 0x20 * vertex + 0xc));
            for (c, w) in [x87::store(x87::load(u)), x87::store(v)]
                .into_iter()
                .enumerate()
            {
                let uvs: u32 = h.read(buffer.wrapping_add(0x18));
                let word = at.wrapping_add(2 * vertex + c as u32);
                h.write(uvs.wrapping_add(word.wrapping_mul(4)), w);
            }
        }
        // indices: two triangles at count * 1.5
        let first: u32 = h.read(buffer.wrapping_add(4));
        #[allow(clippy::cast_precision_loss)] // fild, fstp dword: rounded once
        let n = f64::from(first as f32);
        let at = x87::ftol(h, x87::mul(n, 1.5));
        #[allow(clippy::cast_possible_truncation)] // the original stores 16 bits
        let base = first as u16;
        for (k, d) in [0u16, 1, 2, 2, 1, 3].into_iter().enumerate() {
            let indices: u32 = h.read(buffer.wrapping_add(0x1c));
            let off = at.wrapping_add(k as u32).wrapping_mul(2);
            h.write(indices.wrapping_add(off), base.wrapping_add(d));
        }
        let count: u32 = h.read(buffer.wrapping_add(4));
        h.write(buffer.wrapping_add(4), count.wrapping_add(4));
        let total: u32 = h.read(buffer.wrapping_add(8));
        h.write(buffer.wrapping_add(8), total.wrapping_add(6));
    }
}
