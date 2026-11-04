//! The render-side glue, twenty-seventh batch (M4 port round 7, list
//! `h3-glue-render`): the glow map's preparation, the broken-screen
//! pass's vertex layout and the character's screen height. The exception frame and
//! cookie are left out, as in [`crate::glue10`]; the render surface in the
//! frame is destroyed on every path, as the original's unwinding does.

pub use crate::h3_glue26::{
    BEGIN_SCENE, BIND_TEXTURE, COLOR_BUFFER, END_SCENE, FIND_PARAMETER, MEMCPY,
    RENDER_MESH_INSTANCE, SET_COLOR_TARGET, SET_DEPTH_STENCIL_TARGET, SURFACE_CTOR, SURFACE_DTOR,
    UV_SCALE_BIAS,
};

pub use crate::h3_glue25::{
    FORMAT_SIZE, INDEX_IS_BOUND, INDEX_RELEASE, MALLOC_ALIGNED, SEGMENTS_CONSTRUCT,
    SEGMENTS_DESTROY, STREAM_CTOR, STREAMS_SET,
};

/// `BrokenScreenEffect_fillFloats_006d4150(this)` and
/// `BrokenScreenEffect_fillIndices_006d41b0(this)` (thiscall).
pub const FILL_COLOURS: u32 = 0x006d_4150;
pub const FILL_INDICES: u32 = 0x006d_41b0;

/// The character's screen height: `getScenerioFlag()`,
/// `MsGetBattleScene()` (cdecl), `PhyreScene_Project3DPointOntoScreen(this,
/// out, p, mode)` (`ret 0xc`, a float left in `ST0`),
/// `PApplication::GetApplication`, the gates `0x01325b40` and
/// `0x0193bf68`, `g_CurrentMapNo`, `g_PhyreScene3D` (byte), the `f32`
/// heights (event, map 0x133, lowest, highest) and the `f64` 0.5.
pub const SCENARIO_FLAG: u32 = 0x0078_7360;
pub const BATTLE_SCENE: u32 = 0x0078_2970;
pub const PROJECT_POINT: u32 = 0x0065_26a0;
pub const GET_APPLICATION: u32 = crate::glue::GET_APPLICATION;
pub const SCENARIO_GATE: u32 = 0x0132_5b40;
pub const LOW_GATE: u32 = 0x0193_bf68;
pub const CURRENT_MAP_NO: u32 = 0x012f_bc40;
pub const SCENE_3D: u32 = 0x00cc_b9c0;
pub const EVENT_HEIGHT: u32 = 0x00b4_3cfc;
pub const MAP_HEIGHT: u32 = 0x00b4_12c4;
pub const MIN_HEIGHT: u32 = 0x00b3_eb40;
pub const MAX_HEIGHT: u32 = 0x00b4_3d00;
pub const HALF: u32 = 0x00b9_2220;

/// `"FullGlowThreshold"`, `"NoGlowThreshold"` and the glow pass entry.
pub const FULL_GLOW_THRESHOLD: u32 = 0x00b4_7a60;
pub const NO_GLOW_THRESHOLD: u32 = 0x00b4_7a74;
pub const GLOW_PASS: u32 = 0x00cc_c94c;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// `PArray`-style element pointer: the data `+4` with a count over 1,
    /// the inline element `+4` with 1, none with 0.
    fn array_data(h: &impl Host, array: u32) -> u32 {
        match h.read::<u32>(array) & 0x7fff_ffff {
            0 => 0,
            1 => array + 4,
            _ => h.read(array + 4),
        }
    }

    /// A parameter's offset and size in its buffer (`u16`s at `+8` and
    /// `+0xa`, the size's low 13 bits).
    fn place(h: &impl Host, parameter: u32) -> (u32, u32) {
        let offset = u32::from(h.read::<u16>(parameter + 8));
        let size = u32::from(h.read::<u16>(parameter + 0xa) & 0x1fff);
        (offset, size)
    }

    /// Whether `size` bytes at `offset` fit the buffer (its first word the
    /// size) after its 4-byte header.
    fn fits(h: &impl Host, buffer: u32, offset: u32, size: u32) -> bool {
        offset + size <= h.read::<u32>(buffer) && offset >= 4
    }

    /// `PostProcessManager_prepareGlowMap(this, renderer, target, texture;
    /// slots)` (`ret 0xc`; frame: the surface at `ebp - 0x4c`, the scale
    /// record at `ebp - 0x3c`, the texture record at `ebp - 0x2c`, `this`
    /// at `ebp - 0x1c`, the three parameters found at `- 0x18`, `- 0x14`,
    /// `- 0x10`): the glow mesh's (`+0x1210`) first material gets `texture`
    /// as `"ColorBuffer"`, `{1, 1, 0, 0}` as `"UvScaleBias"` and the
    /// thresholds at `*this + 0x80` / `+ 0x84` (each copied when it fits;
    /// 3 when not; a null source 1); then a surface over `target`, no depth
    /// target, a scene begun with 0, the mesh drawn with the glow pass;
    /// `eax` the first error or `endScene`'s. `texture`'s slot takes each
    /// copy's offset or size as the original keeps them there.
    pub fn post_prepare_glow_map(
        h: &mut impl Host,
        ebp: u32,
        slots: u32,
        this: u32,
        renderer: u32,
        target: u32,
        texture: u32,
    ) -> u32 {
        h.write(ebp - 0x1c, this);
        let mesh: u32 = h.read(this + 0x1210);
        let materials: u32 = h.read(mesh + 0x10);
        let material: u32 = h.read(array_data(h, materials));
        let shader: u32 = h.read(material);
        let material: u32 = h.read(array_data(h, materials));
        let buffer: u32 = h.read(material + 4);
        let mut found = [0u32; 4];
        for (k, name) in [
            COLOR_BUFFER,
            UV_SCALE_BIAS,
            FULL_GLOW_THRESHOLD,
            NO_GLOW_THRESHOLD,
        ]
        .into_iter()
        .enumerate()
        {
            let name = h.rebase(name);
            found[k] = h.call_sym(FIND_PARAMETER, Conv::Thiscall(shader), &[name]);
            if k < 3 {
                h.write(ebp - 0x10 - 4 * k as u32, found[k]);
            }
        }
        let [colour, uv, full, none] = found;
        if colour != 0 {
            let record = ebp - 0x2c;
            h.write(ebp - 0x20, texture);
            h.write(record, 0xffu32);
            h.write(ebp - 0x24, 0u32);
            h.write(ebp - 0x28, 0u32);
            let error = h.call_sym(BIND_TEXTURE, Conv::Thiscall(buffer), &[colour, record]);
            if error != 0 {
                return error;
            }
        }
        if uv != 0 {
            let (offset, size) = place(h, uv);
            let record = ebp - 0x3c;
            let one = x87::store(x87::load(1.0f32.to_bits()));
            h.write(record, one);
            h.write(record + 4, one);
            h.write(record + 8, 0u32);
            h.write(record + 0xc, 0u32);
            h.write(slots + 8, offset);
            if !fits(h, buffer, offset, size) {
                return 3;
            }
            h.call_sym(
                MEMCPY,
                Conv::Cdecl,
                &[buffer.wrapping_add(offset), record, size],
            );
        }
        if full != 0 {
            let source = h.read::<u32>(this).wrapping_add(0x80);
            if source == 0 {
                return 1;
            }
            let (offset, size) = place(h, full);
            h.write(slots + 8, size);
            h.write(ebp - 0x18, offset);
            if !fits(h, buffer, offset, size) {
                return 3;
            }
            h.call_sym(
                MEMCPY,
                Conv::Cdecl,
                &[buffer.wrapping_add(offset), source, size],
            );
        }
        if none != 0 {
            let source = h.read::<u32>(this).wrapping_add(0x84);
            if source == 0 {
                return 1;
            }
            let (offset, size) = place(h, none);
            h.write(slots + 8, offset);
            if !fits(h, buffer, offset, size) {
                return 3;
            }
            h.call_sym(
                MEMCPY,
                Conv::Cdecl,
                &[buffer.wrapping_add(offset), source, size],
            );
        }
        let surface = ebp - 0x4c;
        h.call_sym(SURFACE_CTOR, Conv::Thiscall(surface), &[target, 0]);
        let result = glow_pass(h, this, renderer, surface);
        h.call_sym(SURFACE_DTOR, Conv::Thiscall(surface), &[]);
        result
    }

    /// The glow pass's draw into `surface`: the first error, or
    /// `endScene`'s (the draw's own result is not checked).
    fn glow_pass(h: &mut impl Host, this: u32, renderer: u32, surface: u32) -> u32 {
        let error = h.call_sym(SET_COLOR_TARGET, Conv::Thiscall(renderer), &[surface, 0]);
        if error != 0 {
            return error;
        }
        let error = h.call_sym(SET_DEPTH_STENCIL_TARGET, Conv::Thiscall(renderer), &[0]);
        if error != 0 {
            return error;
        }
        let error = h.call_sym(BEGIN_SCENE, Conv::Thiscall(renderer), &[0]);
        if error != 0 {
            return error;
        }
        let context: u32 = h.read(renderer + 0x13c);
        let pass = h.rebase(GLOW_PASS);
        h.write(renderer + 0x48, pass);
        let mesh: u32 = h.read(this + 0x1210);
        let data: u32 = h.read(mesh + 0x10);
        h.call_sym(
            RENDER_MESH_INSTANCE,
            Conv::Thiscall(renderer),
            &[mesh, data, 0, context],
        );
        h.call_sym(END_SCENE, Conv::Thiscall(renderer), &[1])
    }
    /// `0x006d4780(this, n, buffer; slots)` (thiscall, `ret 8`): a
    /// broken-screen pass's vertex layout. `n` goes to `+8`; the mesh
    /// (`+0x20`) gets one segment (as [`crate::h3_glue25`]'s shadow mesh)
    /// with `+4`'s primitive type and three streams; `buffer` is carved
    /// into four 16-byte-aligned arrays (`+0x40`: 3n words of 0, `+0x44`:
    /// n `{0, 0, 1}` triples, `+0x48`: 4n words filled by `0x006d4150`,
    /// `+0x4c`: 2n words of 0) and `+0x50`'s index array (filled by
    /// `0x006d41b0`); the index block (`+0x30` of the segment) gets the
    /// pass kind's count (6 gives 0x18, 8 gives 0x24, others 0), `{0, n -
    /// 1, count, format 8}` and its size. Returns the end of the index
    /// array's space; `n`'s slot takes the segment, then that end.
    pub fn broken_pass_layout(
        h: &mut impl Host,
        slots: u32,
        this: u32,
        n: u32,
        buffer: u32,
    ) -> u32 {
        h.write(this + 8, n);
        let mesh: u32 = h.read(this + 0x20);
        crate::h3_glue25::bridge::one_segment(h, mesh);
        let mesh: u32 = h.read(this + 0x20);
        let segment: u32 = h.read(mesh + 4);
        h.write(slots, segment);
        h.write(segment, 0u32);
        let primitive: u32 = h.read(this + 4);
        h.write(segment.wrapping_add(0x10), primitive);
        crate::h3_glue25::bridge::ensure_streams(h, segment, 3);
        let n: u32 = h.read(this + 8);
        h.write(this + 0x40, buffer);
        let positions = align16(n.wrapping_mul(12));
        let triples = buffer.wrapping_add(positions);
        h.write(this + 0x44, triples);
        let quads = triples.wrapping_add(positions);
        h.write(this + 0x48, quads);
        let pairs = quads.wrapping_add(align16(n << 4));
        h.write(this + 0x4c, pairs);
        let indices = pairs.wrapping_add(align16(n.wrapping_mul(8)));
        h.write(this + 0x50, indices);
        let end = indices.wrapping_add(align16(index_count(n).wrapping_mul(4)));
        h.write(slots, end);
        let mut i = 0u32;
        while i < h.read::<u32>(this + 8).wrapping_mul(3) {
            let at: u32 = h.read(this + 0x40);
            h.write(at.wrapping_add(4 * i), 0u32);
            i += 1;
        }
        let mut j = 0u32;
        while j < h.read::<u32>(this + 8) {
            for (k, value) in [0u32, 0, 1.0f32.to_bits()].into_iter().enumerate() {
                let at: u32 = h.read(this + 0x44);
                h.write(at.wrapping_add(12 * j + 4 * k as u32), value);
            }
            j += 1;
        }
        h.call_sym(FILL_COLOURS, Conv::Thiscall(this), &[]);
        let mut i = 0u32;
        while i < h.read::<u32>(this + 8).wrapping_mul(2) {
            let at: u32 = h.read(this + 0x4c);
            h.write(at.wrapping_add(4 * i), 0u32);
            i += 1;
        }
        h.call_sym(FILL_INDICES, Conv::Thiscall(this), &[]);
        let n: u32 = h.read(this + 8);
        let count = index_count(n);
        let block = segment.wrapping_add(0x30);
        h.write(block + 8, count);
        h.write(block + 0xc, 8u8);
        h.write(block, 0u32);
        h.write(block + 4, n.wrapping_sub(1));
        let size = h
            .call_sym(FORMAT_SIZE, Conv::Cdecl, &[8])
            .wrapping_mul(count);
        if h.read::<u32>(block + 0x34) != size {
            if h.call_sym(INDEX_IS_BOUND, Conv::Thiscall(block), &[]) as u8 != 0 {
                h.call_sym(INDEX_RELEASE, Conv::Thiscall(block), &[]);
            }
            h.write(block + 0x34, size);
        }
        h.read(slots)
    }

    /// `(x - 1) & !0xf` plus 16: `x` rounded up to 16 bytes (0 stays 0).
    fn align16(x: u32) -> u32 {
        (x.wrapping_sub(1) & 0xffff_fff0).wrapping_add(0x10)
    }

    /// The pass kind's index count: 6 gives 0x18, 8 gives 0x24, others 0.
    fn index_count(n: u32) -> u32 {
        match n {
            6 => 0x18,
            8 => 0x24,
            _ => 0,
        }
    }
    /// `PhyreScene_getChaPos(this, p; slots)` (thiscall, `ret 4`; frame:
    /// the screen point `{x, y}` at `ebp - 8`): the character's height on
    /// screen as a fraction (`+0xf9bc`). Fixed values for some events: the
    /// level (`+0xfaf4`) 0x220, 0x11ee with scenario 0x1c7 (while
    /// `0x01325b40` is set), 0x1e8 or 0x1ea, 0x1e63 with scenario 0x194
    /// give `0x00b43cfc`; 0x226d on map 0x133 gives `0x00b412c4`; 0x159c
    /// with scenario 0x7e4 and `0x0193bf68` set, `+0xf9c4` clear (set by
    /// the constructor and `PhyreScene_change2DSceneNum(1)`) or a 3D scene
    /// (`g_PhyreScene3D` set) give `0x00b3eb40`. Else `p` is projected and `(h / 2 -
    /// (h >> 3) - y) / h` (the window height `+0x270`, each term rounded to
    /// `f32` through `p`'s slot) is clamped to `[0x00b3eb40, 0x00b43d00]`,
    /// 0 in battle scene 0x2050000, and the levels 0x5c8 and 0x2ed add
    /// `+0xfa38`.
    pub fn scene_get_cha_pos(h: &mut impl Host, ebp: u32, slots: u32, this: u32, p: u32) {
        let out = this + 0xf9bc;
        let level: u32 = h.read(this + 0xfaf4);
        let event = level == 0x220
            || (level == 0x11ee
                && ((h.read::<u32>(h.rebase(SCENARIO_GATE)) != 0 && scenario(h) == 0x1c7)
                    || scenario(h) == 0x1e8
                    || scenario(h) == 0x1ea))
            || (h.read::<u32>(this + 0xfaf4) == 0x1e63 && scenario(h) == 0x194);
        if event {
            copy_constant(h, EVENT_HEIGHT, out);
            return;
        }
        let level: u32 = h.read(this + 0xfaf4);
        if level == 0x226d && h.read::<u32>(h.rebase(CURRENT_MAP_NO)) == 0x133 {
            copy_constant(h, MAP_HEIGHT, out);
            return;
        }
        let low =
            (level == 0x159c && scenario(h) == 0x7e4 && h.read::<u32>(h.rebase(LOW_GATE)) != 0)
                || h.read::<u8>(this + 0xf9c4) == 0
                || h.read::<u8>(h.rebase(SCENE_3D)) != 0;
        if low {
            copy_constant(h, MIN_HEIGHT, out);
            return;
        }
        let point = ebp - 8;
        h.call_sym_st0(PROJECT_POINT, Conv::Thiscall(this), &[point, p, 0]);
        let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        let eighth: u32 = h.read(app + 0x270);
        let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        let half: u32 = h.read(app + 0x270);
        let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        // unsigned to f32 through the slot (fild, + 2^32 when negative)
        let half = x87::load(x87::store(f64::from(half)));
        let at = h.rebase(HALF);
        let half = x87::mul(half, f64::from_bits(h.read::<u64>(at)));
        let eighth = x87::load(x87::store(f64::from(eighth >> 3)));
        let y = x87::load(x87::store(f64::from(h.read::<u32>(ebp - 4) as i32)));
        let height: u32 = h.read(app + 0x270);
        let height_f = x87::store(f64::from(height));
        let value = x87::div(x87::sub(x87::sub(half, eighth), y), x87::load(height_f));
        let value = x87::store(value);
        h.write(slots, value);
        h.write(out, value);
        let at = h.rebase(MIN_HEIGHT);
        let min = x87::load(h.read(at));
        if x87::load(value) < min {
            h.write(out, x87::store(min));
        }
        let at = h.rebase(MAX_HEIGHT);
        let max = x87::load(h.read(at));
        if max < x87::load(h.read(out)) {
            h.write(out, x87::store(max));
        }
        if h.call_sym(BATTLE_SCENE, Conv::Cdecl, &[]) == 0x0205_0000 {
            h.write(out, 0u32);
        }
        let level: u32 = h.read(this + 0xfaf4);
        if level == 0x5c8 || level == 0x2ed {
            let offset = x87::load(h.read(this + 0xfa38));
            let sum = x87::add(offset, x87::load(h.read(out)));
            h.write(out, x87::store(sum));
        }
    }

    /// `getScenerioFlag()`'s `ax`.
    fn scenario(h: &mut impl Host) -> u16 {
        h.call_sym(SCENARIO_FLAG, Conv::Cdecl, &[]) as u16
    }

    /// The `f32` constant at `sym` stored at `to` through the x87.
    fn copy_constant(h: &mut impl Host, sym: u32, to: u32) {
        let at = h.rebase(sym);
        let bits: u32 = h.read(at);
        h.write(to, x87::store(x87::load(bits)));
    }
}
