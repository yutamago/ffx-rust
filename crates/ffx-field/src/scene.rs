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

/// `rcbgTrxDraw` (`0x0092d230`): sends a transfer's image to GS memory.
pub const RCBG_TRX_DRAW: u32 = 0x0092_d230;
/// `rcbgSceneSetMatrix` (`0x00920c00`).
pub const SCENE_SET_MATRIX: u32 = 0x0092_0c00;

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

    /// `rcbgSceneAddObject(scene, obj)` (`0x00920540`): the object table
    /// (`+0x10`, count `+0xc`) grown by one ([`model::bridge::rc_realloc`])
    /// and `obj` appended.
    pub fn rcbg_scene_add_object<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        s: u32,
        obj: u32,
        call: &mut C,
    ) {
        let n = mem.read::<u32>(s.wrapping_add(0xc)).wrapping_add(1);
        mem.write(s.wrapping_add(0xc), n);
        let old = mem.read::<u32>(s.wrapping_add(0x10));
        let t = model::bridge::rc_realloc(mem, call, old, n.wrapping_mul(4));
        mem.write(s.wrapping_add(0x10), t);
        let n = mem.read::<u32>(s.wrapping_add(0xc));
        mem.write(t.wrapping_add(n.wrapping_mul(4)).wrapping_sub(4), obj);
    }

    /// `rcbgSceneModelLoad(scene, parse)` (`0x0092b6b0`): a model loaded
    /// and added to the scene.
    pub fn rcbg_scene_model_load<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        s: u32,
        p: u32,
        call: &mut C,
    ) {
        let m = model::bridge::rcbg_model_new(mem, call);
        model::bridge::rcbg_model_load(mem, m, p, call);
        rcbg_scene_add_object(mem, s, m, call);
    }

    /// `rcbgSceneTrxLoad(scene, parse)` (`0x0092b6e0`): a transfer loaded
    /// and added to the scene.
    pub fn rcbg_scene_trx_load<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        s: u32,
        p: u32,
        call: &mut C,
    ) {
        let t = model::bridge::rcbg_trx_new(mem, call);
        model::bridge::rcbg_trx_load(mem, t, p);
        rcbg_scene_add_object(mem, s, t, call);
    }

    /// `rcbgLayerLoad(layer, parse)` (`0x0092b1a0`): the layer
    /// ([`rcbg_layer_init`], its first `0x90` bytes), the animation frames
    /// when it has any (`+0x90`: [`model::bridge::rcbg_anime_set_data_cnt`],
    /// the words padded to 16 bytes, a transfer at `+0xa0`), two bytes
    /// (`+0xba`, `+0xbb`), 14 skipped, the image transfer `+0xbc`, the
    /// centre.
    pub fn rcbg_layer_load<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        l: u32,
        p: u32,
        call: &mut C,
    ) {
        use crate::rcdt::bridge::{rcdt_parse_advance, rcdt_parse_get_bin};
        rcbg_layer_init(mem, l);
        rcdt_parse_get_bin(mem, p, l, 0x90);
        let at = mem.read::<u32>(p.wrapping_add(4));
        let n = mem.read::<i32>(at);
        rcdt_parse_advance(mem, p, 4);
        rcdt_parse_advance(mem, p, 0xc);
        if n > 0 {
            model::bridge::rcbg_anime_set_data_cnt(mem, l.wrapping_add(0x90), n, call);
            let frames = mem.read::<u32>(l.wrapping_add(0x9c));
            let count = mem.read::<u32>(l.wrapping_add(0x98));
            rcdt_parse_get_bin(mem, p, frames, count.wrapping_shl(2));
            let rest = mem.read::<u32>(l.wrapping_add(0x98)) & 3;
            if rest != 0 {
                rcdt_parse_advance(mem, p, 0x10u32.wrapping_sub(rest.wrapping_mul(4)));
            }
            let t = model::bridge::rcbg_trx_new(mem, call);
            mem.write(l.wrapping_add(0xa0), t);
            model::bridge::rcbg_trx_load(mem, t, p);
        }
        rcdt_parse_get_bin(mem, p, l.wrapping_add(0xba), 1);
        rcdt_parse_get_bin(mem, p, l.wrapping_add(0xbb), 1);
        rcdt_parse_advance(mem, p, 0xe);
        let t = model::bridge::rcbg_trx_new(mem, call);
        mem.write(l.wrapping_add(0xbc), t);
        model::bridge::rcbg_trx_load(mem, t, p);
        model::bridge::rcbg_layer_load_calc_center(mem, l);
    }

    /// `rcbgTrxDrawPending(trx)` (`0x0092d470`): a transfer with an entry
    /// pending (`+0x3c` not -1) drawn (`rcbgTrxDraw`, through `call`) and
    /// marked done.
    pub fn rcbg_trx_draw_pending<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        t: u32,
        call: &mut C,
    ) {
        if mem.read::<u32>(t.wrapping_add(0x3c)) != u32::MAX {
            let f = mem.rebase(super::RCBG_TRX_DRAW);
            call(mem, f, &[t]);
            mem.write(t.wrapping_add(0x3c), u32::MAX);
        }
    }

    /// `rcbgSceneChkSortLayer(scene)` (`0x0092b460`): GS memory for the
    /// scene's image layers (flag 0x10): each layer's image at `+0x34`
    /// (in 64-word pages, the size of its 128x64-rounded image, a quarter
    /// once past `0x68000`, where the images restart at page 0x2f00 and
    /// `0xbc000`), then each CLUT at `+0x36` (`0x100` words each, from
    /// `0xbc000` unless the images moved there); the two transfers
    /// (`+0xa0`, `+0xbc`) get their `BITBLTBUF` words, entry 0, and are
    /// drawn when pending ([`rcbg_trx_draw_pending`]).
    pub fn rcbg_scene_chk_sort_layer<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        s: u32,
        call: &mut C,
    ) {
        let layers = mem.read::<u32>(s.wrapping_add(8));
        let count = mem.read::<i32>(s.wrapping_add(4));
        let mut words = 0i32;
        let mut moved = false;
        let mut l = layers;
        for _ in 0..count.max(0) {
            if mem.read::<u8>(l) & 0x10 != 0 {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // the u16 page
                mem.write(l.wrapping_add(0x34), (words / 64) as u16);
                let w = mem.read::<i32>(l.wrapping_add(4)).wrapping_add(0x7f) & !0x7f;
                let h = mem.read::<i32>(l.wrapping_add(8)).wrapping_add(0x3f) & !0x3f;
                let mut size = w.wrapping_mul(h);
                if moved {
                    size /= 4;
                }
                words = words.wrapping_add(size);
                if !moved && words >= 0x68000 {
                    mem.write(l.wrapping_add(0x34), 0x2f00u16);
                    words = 0xbc000;
                    moved = true;
                }
            }
            l = l.wrapping_add(0xc0);
        }
        if !moved {
            words = 0xbc000;
        }
        let mut l = mem.read::<u32>(s.wrapping_add(8));
        for _ in 0..count.max(0) {
            if mem.read::<u8>(l) & 0x10 != 0 {
                let t0 = mem.read::<u32>(l.wrapping_add(0xa0));
                let t1 = mem.read::<u32>(l.wrapping_add(0xbc));
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // the u16 page
                mem.write(l.wrapping_add(0x36), (words / 64) as u16);
                words = words.wrapping_add(0x100);
                let n = mem.read::<i32>(l.wrapping_add(4)).wrapping_add(0x3f);
                let mut tbw = n.wrapping_add((n >> 31) & 0x3f) >> 6;
                if tbw & 1 != 0 {
                    tbw = tbw.wrapping_add(1);
                }
                let page = u32::from(mem.read::<u16>(l.wrapping_add(0x34)));
                mem.write(t0.wrapping_add(0x18), 0u32);
                mem.write(
                    t0.wrapping_add(0x1c),
                    tbw.cast_unsigned().wrapping_shl(16) | page | 0x1b00_0000,
                );
                let clut = u32::from(mem.read::<u16>(l.wrapping_add(0x36)));
                mem.write(t1.wrapping_add(0x18), 0u32);
                mem.write(t1.wrapping_add(0x1c), clut | 0x1_0000);
                model::bridge::rcbg_trx_set_no(mem, t0, 0);
                model::bridge::rcbg_pal_set_no(mem, l.wrapping_add(0xb0), 0);
                rcbg_trx_draw_pending(mem, t0, call);
                rcbg_trx_draw_pending(mem, t1, call);
            }
            l = l.wrapping_add(0xc0);
        }
    }

    /// `rcbgSceneChkLayers(scene)` (`0x0092b400`): `+0xf8` set when a
    /// layer is 448 or more high (`+8`), `+0xfc` when one has an image
    /// (flag 0x10); then [`rcbg_scene_chk_sort_layer`] (a too-high layer
    /// only prints in debug builds).
    pub fn rcbg_scene_chk_layers<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        s: u32,
        call: &mut C,
    ) {
        let n = mem.read::<i32>(s.wrapping_add(4));
        let mut l = mem.read::<u32>(s.wrapping_add(8));
        for _ in 0..n.max(0) {
            if mem.read::<i32>(l.wrapping_add(8)) >= 0x1c0 {
                mem.write(s.wrapping_add(0xf8), 1u32);
            }
            if mem.read::<u8>(l) & 0x10 != 0 {
                mem.write(s.wrapping_add(0xfc), 1u32);
            }
            l = l.wrapping_add(0xc0);
        }
        rcbg_scene_chk_sort_layer(mem, s, call);
    }

    /// `rcbgSceneLoad(scene, parse)` (`0x0092b5e0`): the layer count
    /// (`+4`), 12 bytes skipped, the layers (`0xc0` bytes each,
    /// [`rcbg_layer_load`]), the colours and fog (`+0x68`, `+0x78`,
    /// `+0x88`, `+0xa8`, `+0x64`), [`rcbg_scene_chk_layers`] and
    /// `rcbgSceneSetMatrix` (through `call`).
    pub fn rcbg_scene_load<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        s: u32,
        p: u32,
        call: &mut C,
    ) {
        use crate::rcdt::bridge::{rcdt_parse_advance, rcdt_parse_get_bin};
        rcdt_parse_get_bin(mem, p, s.wrapping_add(4), 4);
        rcdt_parse_advance(mem, p, 0xc);
        let n = mem.read::<i32>(s.wrapping_add(4));
        if n > 0 {
            let t = model::bridge::rc_alloc(mem, call, n.cast_unsigned().wrapping_mul(0xc0));
            mem.write(s.wrapping_add(8), t);
            let mut i = 0;
            while i < mem.read::<i32>(s.wrapping_add(4)) {
                let l = mem
                    .read::<u32>(s.wrapping_add(8))
                    .wrapping_add(i.cast_unsigned().wrapping_mul(0xc0));
                rcbg_layer_load(mem, l, p, call);
                i += 1;
            }
        }
        for (off, n) in [
            (0x68u32, 0x10u32),
            (0x78, 0x10),
            (0x88, 0x10),
            (0xa8, 4),
            (0x64, 4),
        ] {
            rcdt_parse_get_bin(mem, p, s.wrapping_add(off), n);
        }
        rcbg_scene_chk_layers(mem, s, call);
        let f = mem.rebase(super::SCENE_SET_MATRIX);
        call(mem, f, &[s]);
    }

    /// `rcbgTrxTexDraw(tex)` (`0x00922100`): its CLUT transfer (`+0x60`)
    /// then image transfer (`+0x20`) drawn when pending
    /// ([`rcbg_trx_draw_pending`]).
    pub fn rcbg_trx_tex_draw<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        t: u32,
        call: &mut C,
    ) {
        rcbg_trx_draw_pending(mem, t.wrapping_add(0x60), call);
        rcbg_trx_draw_pending(mem, t.wrapping_add(0x20), call);
    }
}
