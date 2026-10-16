//! `rcbgScene`: a map's scene, its layers (the "map parts" of the script
//! natives) and objects, and the Yn data chunk tags (`docs/formats/field.md`).

use ffx_bridge::Global;

/// Offsets in a scene (`rcbgSceneInit`, `0x110` bytes).
pub mod layout {
    /// Size.
    pub const SIZE: u32 = 0x110;
    /// `+0x0`: 1 after `rcbgSceneInit`.
    pub const KIND: u32 = 0x0;
    /// `+0x4`: layers (map parts), `i32` (`rcbgSceneFini`).
    pub const LAYER_COUNT: u32 = 0x4;
    /// `+0x8`: the layers, [`super::layer::SIZE`] bytes each.
    pub const LAYERS: u32 = 0x8;
    /// `+0xc`: objects, `i32` (`rcbgSceneAddObject`).
    pub const OBJECT_COUNT: u32 = 0xc;
    /// `+0x10`: the objects, one pointer each; an object's first word is
    /// its type (`0x20002` transfer, `0x20004` particle simulation, ...).
    pub const OBJECTS: u32 = 0x10;
    /// `+0x18`: guide map models, `i16` (`Yn_GuideMapSetMapNo` bound).
    pub const GUIDE_COUNT: u32 = 0x18;
    /// `+0x1a`: the guide map shown, `i16` (`Yn_GuideMapGetMapNo`).
    pub const GUIDE_CURRENT: u32 = 0x1a;
    /// `+0x1c`: the guide map models.
    pub const GUIDE_MODELS: u32 = 0x1c;
    /// `+0xf4`: the minimap's world units per pixel, 10.0 after
    /// `rcbgSceneInit` (`rcbgGuideUpdateEx` scales by its inverse).
    pub const GUIDE_SCALE: u32 = 0xf4;
    /// 10.0, as `rcbgSceneInit` stores it (an `.rdata` float, `0x00b92318`).
    pub const GUIDE_SCALE_INIT: u32 = 0x4120_0000;
}

/// Offsets in a layer (`rcbgLayerInit`, `0xc0` bytes).
pub mod layer {
    /// Size.
    pub const SIZE: u32 = 0xc0;
    /// `+0x90`: its animation (`rcbgAnimeInit`); bit 1 of the first word
    /// is switched by script natives `0x0091b470`/`0x0091b490`.
    pub const ANIME: u32 = 0x90;
    /// `+0xb0`: its palette (`rcbgPalInit`).
    pub const PAL: u32 = 0xb0;
    /// The words `rcbgLayerInit` sets after clearing: two 1.0 scales, two
    /// `0x100`, `0x44`, 0 and a grey `0x80808080` colour.
    pub const INIT: [(u32, u32); 7] = [
        (0x48, 0x3f80_0000),
        (0x44, 0x3f80_0000),
        (0x30, 0x100),
        (0x2c, 0x100),
        (0x38, 0x44),
        (0x3c, 0),
        (0x40, 0x8080_8080),
    ];
}

/// The Yn data chunk tags (`rcbgChunkTag`): pointers to four-character
/// strings in `.rdata` ("YNDT", "YNPR", "YNSC", "YNTX", "YNTM", "YNSM",
/// "YNST", "YNSP", "YNTT", "YNSE", "YNFT", "YNGM", "YNED").
pub const CHUNK_TAGS: Global<u32> = Global::new(0x00c5_e480);

/// The scene where the game keeps it, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::Mem;

    use super::{CHUNK_TAGS, layer, layout};
    use crate::model;

    /// `rcbgSceneInit(scene)` (`0x009209e0`).
    pub fn rcbg_scene_init(mem: &mut impl Mem, s: u32) {
        mem.write_bytes(s, &[0; layout::SIZE as usize]);
        mem.write(s + layout::GUIDE_SCALE, layout::GUIDE_SCALE_INIT);
        mem.write(s + layout::KIND, 1u32);
    }

    /// `rcbgSceneGetObject(scene, kind, n)` (`0x00920950`): the `n`-th
    /// object of type `kind`, or null.
    pub fn rcbg_scene_get_object(mem: &impl Mem, s: u32, kind: u32, n: u32) -> u32 {
        let count = mem.read::<i32>(s + layout::OBJECT_COUNT);
        let list = mem.read::<u32>(s + layout::OBJECTS);
        let mut seen = 0u32;
        for i in 0..u32::try_from(count.max(0)).unwrap_or(0) {
            let o = mem.read::<u32>(list + 4 * i);
            if mem.read::<u32>(o) == kind {
                if seen == n {
                    return o;
                }
                seen = seen.wrapping_add(1);
            }
        }
        0
    }

    /// `rcbgSceneCountObjects(scene, kind)` (`0x009209a0`).
    pub fn rcbg_scene_count_objects(mem: &impl Mem, s: u32, kind: u32) -> u32 {
        let count = mem.read::<i32>(s + layout::OBJECT_COUNT);
        let list = mem.read::<u32>(s + layout::OBJECTS);
        (0..u32::try_from(count.max(0)).unwrap_or(0))
            .filter(|&i| mem.read::<u32>(mem.read::<u32>(list + 4 * i)) == kind)
            .fold(0, |n: u32, _| n.wrapping_add(1))
    }

    /// `rcbgChunkTag(i)` (`0x00921b90`): the tag string of chunk kind `i`
    /// (unchecked).
    pub fn rcbg_chunk_tag(mem: &impl Mem, i: u32) -> u32 {
        CHUNK_TAGS.ptr(mem).at(i).get(mem)
    }

    /// `rcbgLayerInit(layer)` (`0x0092ad20`).
    pub fn rcbg_layer_init(mem: &mut impl Mem, l: u32) {
        mem.write_bytes(l, &[0; layer::SIZE as usize]);
        model::bridge::rcbg_anime_init(mem, l + layer::ANIME);
        model::bridge::rcbg_pal_init(mem, l + layer::PAL);
        for (o, v) in layer::INIT {
            mem.write(l + o, v);
        }
    }
}
