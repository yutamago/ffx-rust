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

/// The map part override table of the field object (`+0x2c00`, two
/// entries of ten words: a map id and nine parts).
pub const LANG_TABLE: u32 = 0x2c00;
/// Bytes per entry.
pub const LANG_ENTRY: u32 = 0x28;
/// What `MapPart_InitLangOverrides` writes: map `0xcbc` (`titl00`, the
/// title) and `0x199f` (`titl02`, the logos), each with the part per
/// language slot (0..=5 Japanese, English, French, Spanish, German,
/// Italian; 6, 7 Korean and Chinese; 8 unused).
pub const LANG_OVERRIDES: [(u32, [u32; 9]); 2] = [
    (0xcbc, [0, 0xd, 7, 0xb, 9, 5, 0xd, 3, 0xd]),
    (0x199f, [2, 0, 6, 8, 7, 5, 2, 2, 8]),
];

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

    // ---- round 4: language overrides of map parts ----

    /// `MapPart_InitLangOverrides(this)` (`0x0067c620`, `this` in ECX): the
    /// two entries of the override table at `+0x2c00`: a map id and nine
    /// parts by language slot ([`super::LANG_OVERRIDES`]). Returns `this`.
    pub fn map_part_init_lang_overrides(mem: &mut impl Mem, this: u32) -> u32 {
        for (i, (map, parts)) in (0u32..).zip(super::LANG_OVERRIDES) {
            let e = this.wrapping_add(super::LANG_TABLE + i * super::LANG_ENTRY);
            mem.write(e.wrapping_add(4), parts);
            mem.write(e, map);
        }
        this
    }

    /// `MapPart_LangOverride(this, lang, map)` (`0x0067fb30`, thiscall):
    /// the part to show for game language `lang` (0..=5 the slots 0..=5,
    /// 9 and 10 the slots 6 and 7) when `map` has an entry, else -1. Other
    /// languages print "Warning: uncorrect `LanguageType`" (`warn`) and give
    /// -1.
    pub fn map_part_lang_override<M: Mem>(
        mem: &mut M,
        this: u32,
        lang: u32,
        map: u32,
        warn: impl FnOnce(&mut M),
    ) -> u32 {
        let Some(i) = (0..2u32).find(|&i| {
            mem.read::<u32>(this.wrapping_add(super::LANG_TABLE + i * super::LANG_ENTRY)) == map
        }) else {
            return u32::MAX;
        };
        let slot = if lang <= 5 {
            lang
        } else if lang.wrapping_sub(9) <= 1 {
            lang - 3
        } else {
            warn(mem);
            return u32::MAX;
        };
        mem.read(this.wrapping_add(super::LANG_TABLE + 4 + 4 * (slot + i * 10)))
    }

    /// `rcbgLayerFini(layer)` (`0x0092ac90`): deletes its two transfers
    /// (`+0xa0`, `+0xbc`), its animation block (`+0x90`), then
    /// `rcbgPalFini(+0xb0)` (`0x00929980`, an empty function). Returns
    /// `layer + 0xb0`.
    pub fn rcbg_layer_fini<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        l: u32,
        call: &mut C,
    ) -> u32 {
        for off in [0xa0u32, 0xbc] {
            let t = mem.read::<u32>(l.wrapping_add(off));
            if t != 0 {
                model::bridge::rcbg_trx_delete(mem, t, call);
                mem.write(l.wrapping_add(off), 0u32);
            }
        }
        model::bridge::rcbg_layer_anime_fini(mem, l.wrapping_add(0x90), call);
        l.wrapping_add(0xb0)
    }

    /// `rcbgSceneFini(scene)` (`0x00920870`): the layers finished and
    /// freed, each object's destructor (`+4` of the object, when set)
    /// called and the object table freed, `+0x14` freed, the models
    /// (`+0x1c`, `i16` count `+0x18`) deleted. Returns 0.
    pub fn rcbg_scene_fini<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        s: u32,
        call: &mut C,
    ) -> u32 {
        let mut i = 0;
        while i < mem.read::<i32>(s.wrapping_add(4)) {
            let l = mem.read::<u32>(s.wrapping_add(8));
            rcbg_layer_fini(
                mem,
                l.wrapping_add(i.cast_unsigned().wrapping_mul(0xc0)),
                call,
            );
            i += 1;
        }
        mem.write(s.wrapping_add(4), 0u32);
        model::bridge::free_field(mem, call, s.wrapping_add(8));
        let mut i = 0;
        while i < mem.read::<i32>(s.wrapping_add(0xc)) {
            let table = mem.read::<u32>(s.wrapping_add(0x10));
            let o = mem.read::<u32>(table.wrapping_add(i.cast_unsigned().wrapping_mul(4)));
            let f = mem.read::<u32>(o.wrapping_add(4));
            if f != 0 {
                call(mem, f, &[o]);
            }
            i += 1;
        }
        mem.write(s.wrapping_add(0xc), 0u32);
        model::bridge::free_field(mem, call, s.wrapping_add(0x10));
        model::bridge::free_field(mem, call, s.wrapping_add(0x14));
        let mut i = 0i32;
        while i < i32::from(mem.read::<i16>(s.wrapping_add(0x18))) {
            let at = s
                .wrapping_add(0x1c)
                .wrapping_add(i.cast_unsigned().wrapping_mul(4));
            let m = mem.read::<u32>(at);
            if m != 0 {
                model::bridge::rcbg_model_delete(mem, m, call);
                mem.write(at, 0u32);
            }
            i += 1;
        }
        mem.write(s.wrapping_add(0x18), 0u16);
        0
    }
}
