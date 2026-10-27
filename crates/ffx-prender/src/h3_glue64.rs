//! The render-side glue, sixty-fourth batch (M4 port round 18, list
//! `h3-glue-render`): the field map's asset callback, which keeps the
//! loaded assets and places the map model's instances.

/// `Array_Reserve_00668080(array; n)` (`ret 4`).
pub const ARRAY_RESERVE: u32 = crate::glue4::ARRAY_RESERVE_668080;
/// `AssetListReader_isModel(reader; id)` (`ret 4`, `al`) and
/// `AssetListReader_getSymbolById(reader; id)` (`ret 4`).
pub const IS_MODEL: u32 = 0x006f_46b0;
pub const GET_SYMBOL_BY_ID: u32 = 0x006f_42d0;
/// The cluster's instance cursors: `PhyreSceneCursor_ctor_00633880` and
/// `PhyreSceneCursor_ctorSeek2_0064cda0` (`cursor; cluster`, `ret 4`),
/// `PhyreSceneCursor_Next_006377a0` and `PhyreSceneCursor_Next3`
/// (thiscall).
pub const CURSOR_CTOR: u32 = crate::h3_glue21::CURSOR_CTOR;
pub const CURSOR_NEXT: u32 = crate::h3_glue21::CURSOR_NEXT;
pub const CURSOR_CTOR_SEEK2: u32 = 0x0064_cda0;
pub const CURSOR_NEXT3: u32 = 0x0065_ede0;
/// The matrices: `PMatrix4_FromMatrix3x4_00451080(out; m)`,
/// `PMatrix4_Copy_004c9430(out; m)` (`ret 4`),
/// `PMatrix4_MulThis_004c9da0(a; out, b)` (`ret 8`) and
/// `PMatrix4_toWorldMatrix3x4(dst; m)` (`ret 4`).
pub const FROM_3X4: u32 = crate::glue5::MATRIX4_FROM_3X4;
pub const COPY: u32 = crate::h3_glue63::COPY;
pub const MUL_THIS: u32 = crate::h3_glue63::MUL_THIS;
pub const TO_WORLD_3X4: u32 = crate::h3_glue63::TO_WORLD_3X4;
/// `GetClassDescriptor<PMatrix4x3>()` (cdecl) and
/// `Phyre_PCluster_allocateInstanceList(cluster; n, type, 0)` (`ret
/// 0xc`).
pub const MATRIX4X3_TYPE: u32 = 0x0046_2aa0;
pub const ALLOCATE_INSTANCE_LIST: u32 = crate::p6_quads::ALLOCATE_INSTANCE_LIST;
/// The image's constants: the map scale -2.5 (f32), the offsets -256
/// and -224 (f32), the 10 (f32) of the scaled map, and 10 (f64).
pub const MAP_SCALE: u32 = 0x00b4_3df0;
pub const OFFSET_X: u32 = 0x00b4_3df8;
pub const OFFSET_Y: u32 = 0x00b4_3df4;
pub const SCALE_TEN: u32 = 0x00b9_2318;
pub const TEN: u32 = crate::h3_glue63::TEN;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use crate::h3_glue14::bridge::walk;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// The frame (`ebp -` these): the instance's 4x4, the scale
    /// temporaries, the placed matrix, the four matrices it is built
    /// from, and the cursor.
    const INSTANCE: u32 = 0x1c0;
    const SCALED: u32 = 0x180;
    const PLACED: u32 = 0x160;
    const M120: u32 = 0x120;
    const ME0: u32 = 0xe0;
    const MA0: u32 = 0xa0;
    const M60: u32 = 0x60;
    const CURSOR: u32 = 0x20;

    const ONE: u32 = 0x3f80_0000;

    /// A word through the x87 (`fld dword` / `fstp dword`).
    fn through(w: u32) -> u32 {
        x87::store(x87::load(w))
    }

    /// Writes the 16 words of a matrix: `value(k)` at `at + 4k`.
    fn fill(h: &mut impl Host, at: u32, value: impl Fn(u32) -> u32) {
        for k in 0..16 {
            h.write(at + 4 * k, value(k));
        }
    }

    /// Copies 16 words from `from` to `to` through the x87.
    fn copy16(h: &mut impl Host, to: u32, from: u32) {
        for k in 0..16 {
            let w: u32 = h.read(from.wrapping_add(4 * k));
            h.write(to.wrapping_add(4 * k), through(w));
        }
    }

    /// `PhyreScene_doOnFieldMapAssetLoaded(scene; asset, unused, &model,
    /// &assets, reader, id, scaled)` (thiscall, `ret 0x1c`; frame from
    /// `ebp - 0x1c0`) -> 0. `asset` is appended to `assets` (a word
    /// array `{count, capacity, data}`, grown to count + count / 8 + 2
    /// when full; a null slot address skips the store). For the first
    /// model asset (the reader's `isModel`, the scene's `+0xd8` still 0):
    /// `model` = asset, `+0xd8` = its symbol; each instance of the model
    /// cluster (`[base + offset + 4]`, the cursor at `ebp - 0x20`) gets
    /// its matrix times the map's placement (scaled by -2.5 and moved by
    /// (-256, -224), or with `scaled` by 10), written back as a 3x4
    /// through a new `PMatrix4x3` instance of the cluster, the matrix's
    /// owner (`+0x1c`) pointing at it (`+0xc`). With `scaled`, a second
    /// walk multiplies each instance's `+0xb0` vector (`+0x20..`) by 10.
    #[allow(clippy::too_many_arguments)] // the original's
    pub fn do_on_field_map_asset_loaded(
        h: &mut impl Host,
        ebp: u32,
        scene: u32,
        asset: u32,
        model: u32,
        assets: u32,
        reader: u32,
        id: u32,
        scaled: u32,
    ) -> u32 {
        let count = h.read::<u32>(assets).wrapping_add(1);
        h.write(assets, count);
        if (count as i32) > h.read::<i32>(assets.wrapping_add(4)) {
            let n = ((count as i32) / 8) as u32;
            let n = n.wrapping_add(2).wrapping_add(count);
            h.call_sym(ARRAY_RESERVE, Conv::Thiscall(assets), &[n]);
        }
        let count: u32 = h.read(assets);
        let data: u32 = h.read(assets.wrapping_add(8));
        let slot = data.wrapping_add(count.wrapping_mul(4)).wrapping_sub(4);
        if slot != 0 {
            h.write(slot, asset);
        }
        if asset == 0 {
            return 0;
        }
        if h.call_sym(IS_MODEL, Conv::Thiscall(reader), &[id]) as u8 == 0 {
            return 0;
        }
        if h.read::<u32>(scene + 0xd8) != 0 {
            return 0;
        }
        h.write(model, asset);
        let symbol = h.call_sym(GET_SYMBOL_BY_ID, Conv::Thiscall(reader), &[id]);
        h.write(scene + 0xd8, symbol);
        let scaled = scaled as u8 != 0;
        let cursor = ebp - CURSOR;
        let cluster: u32 = h.read(model);
        h.call_sym(CURSOR_CTOR, Conv::Thiscall(cursor), &[cluster]);
        walk(h, cursor, CURSOR_NEXT, |h, base| {
            place_instance(h, ebp, model, base, scaled);
        });
        let cluster: u32 = h.read(model);
        h.call_sym(CURSOR_CTOR_SEEK2, Conv::Thiscall(cursor), &[cluster]);
        let ten = f64::from_bits(h.read::<u64>(h.rebase(TEN)));
        walk(h, cursor, CURSOR_NEXT3, |h, base| {
            if scaled {
                scale_vector(h, ebp, base, ten);
            }
        });
        0
    }

    /// One instance of the first walk.
    fn place_instance(h: &mut impl Host, ebp: u32, model: u32, base: u32, scaled: bool) {
        let offset: u32 = h.read(ebp - CURSOR + 0x10);
        let matrix: u32 = h.read(base.wrapping_add(offset).wrapping_add(4));
        h.call_sym(FROM_3X4, Conv::Thiscall(ebp - INSTANCE), &[matrix]);
        let product = if scaled {
            // diag(10, 10, 10, 1)
            let ten = through(h.read(h.rebase(SCALE_TEN)));
            fill(h, ebp - MA0, |k| match k {
                0 | 5 | 10 => ten,
                15 => ONE,
                _ => 0,
            });
            h.call_sym(COPY, Conv::Thiscall(ebp - M60), &[ebp - MA0]);
            copy16(h, ebp - M120, ebp - M60);
            h.call_sym(
                MUL_THIS,
                Conv::Thiscall(ebp - INSTANCE),
                &[ebp - MA0, ebp - M120],
            )
        } else {
            // diag(-2.5, -2.5, 1, 1), then a move by (-256, -224)
            let scale = through(h.read(h.rebase(MAP_SCALE)));
            fill(h, ebp - ME0, |k| match k {
                0 | 5 => scale,
                10 | 15 => ONE,
                _ => 0,
            });
            h.call_sym(COPY, Conv::Thiscall(ebp - M60), &[ebp - ME0]);
            copy16(h, ebp - M120, ebp - M60);
            let x = through(h.read(h.rebase(OFFSET_X)));
            let y = through(h.read(h.rebase(OFFSET_Y)));
            fill(h, ebp - M60, |k| match k {
                0 | 5 | 10 | 15 => ONE,
                12 => x,
                13 => y,
                _ => 0,
            });
            h.call_sym(COPY, Conv::Thiscall(ebp - ME0), &[ebp - M60]);
            copy16(h, ebp - MA0, ebp - ME0);
            let first = h.call_sym(
                MUL_THIS,
                Conv::Thiscall(ebp - INSTANCE),
                &[ebp - ME0, ebp - M120],
            );
            h.call_sym(MUL_THIS, Conv::Thiscall(first), &[ebp - M60, ebp - MA0])
        };
        copy16(h, ebp - PLACED, product);
        let cluster: u32 = h.read(model);
        let kind = h.call_sym(MATRIX4X3_TYPE, Conv::Cdecl, &[]);
        let list = h.call_sym(
            ALLOCATE_INSTANCE_LIST,
            Conv::Thiscall(cluster),
            &[1, kind, 0],
        );
        let placed = if list == 0 {
            0
        } else {
            let placed: u32 = h.read(list + 0x34);
            if placed != 0 {
                h.call_sym(TO_WORLD_3X4, Conv::Thiscall(placed), &[ebp - PLACED]);
            }
            placed
        };
        // a missing instance list reads address 0, as the original does
        let first: u32 = h.read(placed);
        let offset: u32 = h.read(ebp - CURSOR + 0x10);
        let object = base.wrapping_add(offset);
        let target: u32 = h.read(object.wrapping_add(4));
        let owner: u32 = h.read(object.wrapping_add(0x1c));
        h.write(target, through(first));
        let second: u32 = h.read(placed.wrapping_add(4));
        h.write(owner.wrapping_add(0xc), target);
        h.write(target.wrapping_add(4), through(second));
        for k in 2..12 {
            let w: u32 = h.read(placed.wrapping_add(4 * k));
            h.write(target.wrapping_add(4 * k), through(w));
        }
    }

    /// One instance of the second walk: its `+0xb0` vector (`+0x20`,
    /// `+0x24`, `+0x28`) times 10, through the frame (`ebp - 0x180..`,
    /// `- 0x170..`).
    fn scale_vector(h: &mut impl Host, ebp: u32, base: u32, ten: f64) {
        let offset: u32 = h.read(ebp - CURSOR + 0x10);
        let vector: u32 = h.read(base.wrapping_add(offset).wrapping_add(0xb0));
        for k in 0..3 {
            let w: u32 = h.read(vector.wrapping_add(0x20 + 4 * k));
            h.write(ebp - SCALED + 4 * k, through(w));
        }
        for k in 0..3 {
            let w: u32 = h.read(ebp - SCALED + 4 * k);
            let v = x87::store(x87::mul(x87::load(w), ten));
            h.write(ebp - SCALED + 0x10 + 4 * k, v);
        }
        for k in 0..3 {
            let w: u32 = h.read(ebp - SCALED + 0x10 + 4 * k);
            h.write(vector.wrapping_add(0x20 + 4 * k), through(w));
        }
    }
}
