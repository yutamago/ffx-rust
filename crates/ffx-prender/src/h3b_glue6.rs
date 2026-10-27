//! The render-side glue at 0x00690000 and up (M4, list `h3-glue-render`,
//! agent h3b, round 2): `TextureAnimationManager_addAnimatedMaterials`,
//! which collects a cluster range's animated textures and materials into
//! the texture-animation manager (Fahrenheit's
//! `TextureAnimationManager::addAnimatedMaterials`).

/// `Phyre_PhyreMalloc_0042fb80(size, align)`, `Phyre_PhyreFree(p)`.
pub const MALLOC: u32 = 0x0042_fb80;
pub const FREE: u32 = 0x0042_fb20;
/// `operator new`, `operator new[]`, `operator delete[]` (cdecl).
pub const NEW: u32 = 0x0063_0310;
pub const NEW_ARRAY: u32 = 0x0063_04c0;
pub const DELETE_ARRAY: u32 = 0x0063_0540;
/// The CRT through its thunks (cdecl) and import slots.
pub const MEMCPY: u32 = 0x0094_9256;
pub const MEMSET: u32 = 0x0094_964c;
pub const STRSTR: u32 = 0x00b0_c378;
pub const ATOI: u32 = 0x00b0_c40c;

/// The scene cursors: `Begin`/`Next` of the materials' walk (class
/// [`MATERIAL_CLASS`]), the textures' seek (class [`TEXTURE_CLASS`]) and
/// the animation cluster's `Begin`/`Next` (class [`ANIMATION_CLASS`]).
pub const CURSOR_BEGIN: u32 = 0x0063_7730;
pub const CURSOR_NEXT: u32 = 0x0063_77a0;
pub const ITER_SEEK: u32 = 0x0046_4f30;
pub const ANIM_BEGIN: u32 = 0x006b_2230;
pub const ANIM_NEXT: u32 = 0x006b_22e0;
pub const MATERIAL_CLASS: u32 = 0x00ca_7228;
pub const TEXTURE_CLASS: u32 = 0x00ca_37d0;
pub const ANIMATION_CLASS: u32 = 0x00cb_1eb8;
/// The class an animation object's chain (`+0x2c`, next `+0x40`) must hold
/// for a character animation (`TextureAnimationInfoChar`).
pub const CHAR_CLASS: u32 = 0x00cb_1f50;

/// The pushes: words (`PArray_PushInto_006b24e0`, `_006b2570`,
/// `_0065fac0`), `PArray_appendInto_006a25e0`, the 32-byte
/// `TextureAnimationInfo` and the 64-byte `TextureAnimationInfoChar`.
pub const PUSH_6B24E0: u32 = 0x006b_24e0;
pub const PUSH_6B2570: u32 = 0x006b_2570;
pub const PUSH_65FAC0: u32 = 0x0065_fac0;
pub const APPEND_6A25E0: u32 = 0x006a_25e0;
pub const PUSH_INFO: u32 = 0x006b_2730;
pub const PUSH_CHAR: u32 = 0x006b_2800;
/// `RenderTextureInfo_set_006b2400(this; &array, name, path, f0..f4)`.
pub const INFO_SET: u32 = 0x006b_2400;
/// The two vtables.
pub const INFO_VTABLE: u32 = 0x00b4_8420;
pub const CHAR_VTABLE: u32 = 0x00b4_842c;
/// `PArray_Data_004d45a0(array)`, `ShaderParams_FindByName(shader; name)`.
pub const ARRAY_DATA: u32 = 0x004d_45a0;
pub const FIND_PARAM: u32 = 0x0056_cbf0;
/// `ShadowManager_getInstance()`, `ShadowManager_getDummyShadowMap(this)`.
pub const SHADOW_INSTANCE: u32 = 0x006d_2390;
pub const DUMMY_SHADOW_MAP: u32 = 0x006d_2380;
/// `Render_SurfaceFormat_0066e4f0(buffer; parameter, &record)`.
pub const WRITE_PARAM: u32 = 0x0066_e4f0;
/// `TextureAnimationMaterial`: assign, copy an array, copy-construct,
/// destroy.
pub const MATERIAL_ASSIGN: u32 = 0x006a_faf0;
pub const MATERIAL_COPY_ARRAY: u32 = 0x006a_e730;
pub const MATERIAL_COPY: u32 = 0x006a_eee0;
pub const MATERIAL_DTOR: u32 = 0x006a_f690;

/// Strings: `"Mat_CTA"`, `""`, `"CHR_TEXTURE_ANIMATION_UV"`, `":"`,
/// `"\r\n"`, `","`.
pub const MAT_CTA: u32 = 0x00b4_843c;
pub const EMPTY: u32 = 0x00b8_fbeb;
pub const CHR_UV: u32 = 0x00b4_85b0;
pub const COLON: u32 = 0x00b4_85cc;
pub const NEWLINE: u32 = 0x00b4_85d0;
pub const COMMA: u32 = 0x00b8_f884;
/// `"AnimSampler"` .. `"AnimSampler6"`.
pub const SAMPLERS: [u32; 6] = [
    0x00b4_8444,
    0x00b4_8450,
    0x00b4_8460,
    0x00b4_8470,
    0x00b4_8480,
    0x00b4_8490,
];
/// `"layer1"` .. `"layer6"`.
pub const LAYERS: [u32; 6] = [
    0x00b4_1724,
    0x00b4_172c,
    0x00b4_1734,
    0x00b4_173c,
    0x00b4_1744,
    0x00b4_174c,
];
/// `"AnimTranslate_GAME"` .. `"AnimScale_MAYA"` (into `+0x38` ..).
pub const TRANSFORMS: [u32; 14] = [
    0x00b4_84a0,
    0x00b4_84b4,
    0x00b4_84c8,
    0x00b4_84dc,
    0x00b4_84f0,
    0x00b4_8504,
    0x00b4_8518,
    0x00b4_8528,
    0x00b4_8538,
    0x00b4_8548,
    0x00b4_8558,
    0x00b4_8568,
    0x00b4_8578,
    0x00b4_858c,
];
/// `"AnimSamplerState"`.
pub const SAMPLER_STATE: u32 = 0x00b4_859c;

/// The frame (`ebp` offsets, below `ebp`): the bytes the port uses.
pub const FRAME: u32 = 0x140;
/// The materials' cursor (`- 0xf8`, eight words), the textures' and
/// animations' cursor (`- 0x78`), the stack `TextureAnimationMaterial`
/// (`- 0x98`, 0x20 bytes: words `+0`, Char objects `+8`, words `+0x10`,
/// names `+0x18`/`+0x1c`), the stack `TextureAnimationInfoChar`
/// (`- 0xd8`, 0x40 bytes) and `TextureAnimationInfo` (`- 0xb8`, 0x20).
pub const CURSOR_A: u32 = 0xf8;
pub const CURSOR_B: u32 = 0x78;
pub const MATERIAL: u32 = 0x98;
pub const CHAR_INFO: u32 = 0xd8;
pub const INFO: u32 = 0xb8;
/// The sampler records (four words each) of `AnimSampler` .. `6`.
pub const SAMPLER_RECORDS: [u32; 6] = [0x20, 0x30, 0x120, 0x130, 0xa8, 0x68];
/// The zero floats copied into `layer1` .. `6`.
pub const LAYER_SLOTS: [u32; 6] = [0x108, 0x10c, 0xfc, 0x100, 0x104, 0x110];
/// The sampler-state record (`0x47, 0, this + 0x38, 0`).
pub const STATE_RECORD: u32 = 0x140;
/// The number buffers of the shadow table's parse (16 bytes each).
pub const KEY_TEXT: u32 = 0x30;
pub const VALUE_TEXT: u32 = 0x20;

pub mod bridge {
    //! The original over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    const MASK: u32 = 0x7fff_ffff;

    /// A tagged pointer word (bit 0: not owned) as the address.
    fn untag(v: u32) -> u32 {
        v.wrapping_sub(v & 1)
    }

    fn strlen(h: &impl Host, p: u32) -> u32 {
        let mut n = 0u32;
        while h.read::<u8>(p.wrapping_add(n)) != 0 {
            n = n.wrapping_add(1);
        }
        n
    }

    /// The inlined `strcmp(a, b) == 0`.
    fn streq(h: &impl Host, a: u32, b: u32) -> bool {
        let mut k = 0u32;
        loop {
            let x: u8 = h.read(a.wrapping_add(k));
            if x != h.read::<u8>(b.wrapping_add(k)) {
                return false;
            }
            if x == 0 {
                return true;
            }
            k = k.wrapping_add(1);
        }
    }

    /// A cdecl import through its slot.
    fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Cdecl, args)
    }

    fn malloc(h: &mut impl Host, size: u32, align: u32) -> u32 {
        h.call_sym(MALLOC, Conv::Cdecl, &[size, align])
    }

    fn free(h: &mut impl Host, p: u32) {
        h.call_sym(FREE, Conv::Cdecl, &[p]);
    }

    fn memcpy(h: &mut impl Host, dst: u32, src: u32, n: u32) {
        h.call_sym(MEMCPY, Conv::Cdecl, &[dst, src, n]);
    }

    /// A word pushed onto `array` (count `+0`): a block of `count + 1`
    /// words, then `push(array; block, value_at)`.
    fn push_word(h: &mut impl Host, push: u32, array: u32, value_at: u32) {
        let n = h.read::<u32>(array) & MASK;
        let block = malloc(h, n.wrapping_mul(4).wrapping_add(4), 4);
        h.call_sym(push, Conv::Thiscall(array), &[block, value_at]);
    }

    /// A scene cursor over the list `list` of objects of `class`.
    fn cursor_init(h: &mut impl Host, at: u32, list: u32, class: u32) {
        let first: u32 = h.read(list);
        for k in 0..5 {
            h.write(at + 4 * k, 0u32);
        }
        h.write(at + 0x14, list);
        h.write(at + 0x18, if first == list { 0 } else { first });
        h.write(at + 0x1c, h.rebase(class));
    }

    /// A float copied through the x87 (`fld`/`fstp dword`).
    fn copy_float(h: &impl Host, at: u32) -> u32 {
        x87::store(x87::load(h.read(at)))
    }

    /// A string assigned (`PString`-like, tagged word at `slot`): a copy
    /// of `src` (`PhyreMalloc(len + 1, 2)`), the old one freed when owned;
    /// nothing when the allocation fails.
    fn assign_copy(h: &mut impl Host, ebp: u32, slot: u32, src: u32) {
        let s = if src == 0 {
            0
        } else {
            let n = strlen(h, src).wrapping_add(1);
            h.write(ebp - 0x34, n);
            let s = malloc(h, n, 2);
            if s == 0 {
                return;
            }
            memcpy(h, s, src, n);
            s
        };
        let old: u32 = h.read(slot);
        if old & 1 == 0 {
            free(h, old);
        }
        h.write(slot, s);
    }

    /// `TextureAnimationManager_addAnimatedMaterials(this; scene,
    /// clusters, first, last, name, unused, table)` (`ret 0x1c`; frame
    /// at `ebp`, see the module's offsets):
    /// - the first texture of each cluster `first..last` (`clusters + 8`)
    ///   into the manager's list `+0x5c` (no `name`; `+0x10` its first
    ///   count) or the stack material's words;
    /// - each animation object of cluster `last` as a
    ///   `TextureAnimationInfoChar` into the stack material (its chain
    ///   holds [`CHAR_CLASS`]) or a `TextureAnimationInfo` into `+0x7c`;
    /// - each named material of `scene`: a `Mat_CTA` one gets a 0x7c-byte
    ///   record (samplers bound to the dummy shadow map, layers zeroed,
    ///   transforms, the sampler state) in the stack material's `+0x10`,
    ///   the others go to `+0x74` (`+0xc` its first count);
    /// - with `name`: the shadow table (`table`, rows `key:v!,v?,...\r\n`)
    ///   links the Char objects (`+0x3c`) and the records by name
    ///   (`+0x38`), the material takes `name` twice and goes into the
    ///   first empty slot of `+0x64`/`+0x68`, or a new one;
    /// - `+4` = 1 when `+0x7c` has entries.
    #[allow(clippy::too_many_lines, clippy::too_many_arguments)] // the original's sequence
    pub fn add_animated_materials(
        h: &mut impl Host,
        ebp: u32,
        this: u32,
        scene: u32,
        clusters: u32,
        first: u32,
        last: u32,
        name: u32,
        table: u32,
    ) {
        h.write(ebp - 0x40, this);
        h.write(ebp - 0x58, name);
        h.write(ebp - 0x38, table);
        let cursor_a = ebp - CURSOR_A;
        cursor_init(h, cursor_a, scene.wrapping_add(0x1c), MATERIAL_CLASS);
        h.call_sym(CURSOR_BEGIN, Conv::Thiscall(cursor_a), &[]);
        let material = ebp - MATERIAL;
        for k in 0..8 {
            h.write(material + 4 * k, 0u32);
        }
        h.write(ebp - 0x50, 0u32);
        h.write(ebp - 0x44, 1u32);

        // the first texture of each cluster before the last
        let cursor = ebp - CURSOR_B;
        let mut count_first = true;
        let mut i = first;
        while (i as i32) < (last as i32) {
            let data: u32 = h.read(clusters.wrapping_add(8));
            let list = h
                .read::<u32>(data.wrapping_add(i.wrapping_mul(4)))
                .wrapping_add(0x1c);
            cursor_init(h, cursor, list, TEXTURE_CLASS);
            h.call_sym(ITER_SEEK, Conv::Thiscall(cursor), &[]);
            let object = h
                .read::<u32>(ebp - 0x68)
                .wrapping_add(h.read(ebp - CURSOR_B));
            h.write(ebp - 0x3c, object);
            let array = if name == 0 {
                if count_first {
                    let n = h.read::<u32>(this + 0x5c) & MASK;
                    h.write(this + 0x10, n);
                    count_first = false;
                }
                this + 0x5c
            } else {
                material
            };
            push_word(h, PUSH_6B24E0, array, ebp - 0x3c);
            i = i.wrapping_add(1);
        }

        // the last cluster's animation objects
        let data: u32 = h.read(clusters.wrapping_add(8));
        let list = h
            .read::<u32>(data.wrapping_add(last.wrapping_mul(4)))
            .wrapping_add(0x1c);
        cursor_init(h, cursor, list, ANIMATION_CLASS);
        h.call_sym(ANIM_BEGIN, Conv::Thiscall(cursor), &[]);
        'objects: loop {
            let mut count: u32 = h.read(ebp - 0x6c);
            if count == 0 {
                break;
            }
            let mut at: u32 = h.read(ebp - CURSOR_B);
            loop {
                animation_object(h, ebp, this, at);
                let mut block: u32 = h.read(ebp - 0x74);
                let stride: u32 = h.read(ebp - 0x70);
                loop {
                    at = at.wrapping_add(stride);
                    count = count.wrapping_sub(1);
                    h.write(ebp - CURSOR_B, at);
                    h.write(ebp - 0x6c, count);
                    if block != at {
                        break;
                    }
                    block = h.read(block);
                    h.write(ebp - 0x74, block);
                    if count == 0 {
                        break;
                    }
                }
                if count == 0 {
                    h.call_sym(ANIM_NEXT, Conv::Thiscall(cursor), &[]);
                    continue 'objects;
                }
            }
        }

        // the scene's materials
        if h.read::<u32>(ebp - 0xec) != 0 {
            loop {
                let object = h
                    .read::<u32>(ebp - 0xe8)
                    .wrapping_add(h.read(ebp - CURSOR_A));
                if object != 0 {
                    let params: u32 = h.read(object + 0x10);
                    h.write(ebp - 0x54, params);
                    h.write(ebp - 0x48, 0u32);
                    if h.read::<u32>(params) & MASK != 0 {
                        let mut k = 0u32;
                        loop {
                            scene_material(h, ebp, this, params, k);
                            k = k.wrapping_add(1);
                            h.write(ebp - 0x48, k);
                            if k >= h.read::<u32>(params) & MASK {
                                break;
                            }
                        }
                    }
                }
                let mut at: u32 = h.read(ebp - CURSOR_A);
                let mut count: u32 = h.read(ebp - 0xec);
                if count != 0 {
                    let mut block: u32 = h.read(ebp - 0xf4);
                    let stride: u32 = h.read(ebp - 0xf0);
                    loop {
                        at = at.wrapping_add(stride);
                        count = count.wrapping_sub(1);
                        h.write(ebp - CURSOR_A, at);
                        h.write(ebp - 0xec, count);
                        if block != at {
                            break;
                        }
                        block = h.read(block);
                        h.write(ebp - 0xf4, block);
                        if count == 0 {
                            break;
                        }
                    }
                    if count == 0 {
                        h.call_sym(CURSOR_NEXT, Conv::Thiscall(cursor_a), &[]);
                        count = h.read(ebp - 0xec);
                    }
                }
                let n: u32 = h.read(ebp - 0x50);
                h.write(ebp - 0x50, n.wrapping_add(1));
                if count == 0 {
                    break;
                }
            }
        }

        if name != 0 {
            shadow_table(h, ebp);
            assign_copies(h, ebp, name);
            store_material(h, ebp, this);
        }
        if h.read::<u32>(this + 0x7c) & MASK != 0 {
            h.write(this + 4, 1u32);
        }
        h.call_sym(MATERIAL_DTOR, Conv::Thiscall(material), &[]);
    }

    /// One animation object (the cursor's element at `at`): a
    /// `TextureAnimationInfoChar` into the stack material's `+8` when its
    /// chain holds [`CHAR_CLASS`], else a `TextureAnimationInfo` into
    /// `this + 0x7c`; the frame object destroyed after.
    fn animation_object(h: &mut impl Host, ebp: u32, this: u32, at: u32) {
        let node: u32 = h.read(ebp - 0x60);
        let mut class: u32 = h.read(node.wrapping_add(0x2c));
        let char_class = h.rebase(CHAR_CLASS);
        while class != 0 && class != char_class {
            class = h.read(class + 0x40);
        }
        let record = h.read::<u32>(ebp - 0x68).wrapping_add(at);
        if class == char_class {
            char_object(h, ebp, record);
        } else {
            info_object(h, ebp, this, record);
        }
    }

    /// A `TextureAnimationInfoChar` built in the frame from `record`.
    fn char_object(h: &mut impl Host, ebp: u32, record: u32) {
        let o = ebp - CHAR_INFO;
        for off in [4, 8, 0xc, 0x10, 0x14, 0x18, 0x20, 0x24, 0x28, 0x2c, 0x30] {
            h.write(o + off, 0u32);
        }
        h.write(o + 0x1c, 1u8);
        h.write(o + 0x34, 0u8);
        h.write(o, h.rebase(CHAR_VTABLE));
        let floats = [0x18, 0x1c, 0x20, 0x24, 8].map(|off| copy_float(h, record + off));
        let first = untag(h.read(record));
        let second = untag(h.read(record + 0x14));
        h.call_sym(
            INFO_SET,
            Conv::Thiscall(o),
            &[
                record + 0xc,
                first,
                second,
                floats[0],
                floats[1],
                floats[2],
                floats[3],
                floats[4],
            ],
        );
        let n = h.read::<u32>(ebp - 0x90) & MASK;
        let block = malloc(h, n.wrapping_add(1) << 6, 4);
        h.call_sym(PUSH_CHAR, Conv::Thiscall(ebp - 0x90), &[block, o]);
        h.write(o, h.rebase(CHAR_VTABLE));
        let path: u32 = h.read(o + 0x20);
        if path & 1 == 0 {
            free(h, path);
        }
        h.write(o, h.rebase(INFO_VTABLE));
        let data: u32 = h.read(o + 0xc);
        if h.read::<u32>(o + 8) >> 31 == 0 && data != 0 {
            free(h, data);
        }
        let first: u32 = h.read(o + 4);
        if first & 1 == 0 {
            free(h, first);
        }
    }

    /// A `TextureAnimationInfo` built in the frame from `record`.
    fn info_object(h: &mut impl Host, ebp: u32, this: u32, record: u32) {
        let o = ebp - INFO;
        h.write(o, h.rebase(INFO_VTABLE));
        for off in [4, 8, 0xc, 0x10, 0x14, 0x18] {
            h.write(o + off, 0u32);
        }
        h.write(o + 0x1c, 1u8);
        let scale = copy_float(h, record + 8);
        h.write(ebp - 0x34, scale);
        let text = untag(h.read(record));
        h.write(ebp - 0x3c, text);
        let n = h.read::<u32>(record + 0xc) & MASK;
        let mut src: u32 = h.read(record + 0x10);
        h.write(ebp - 0x4c, n);
        let mut data = 0;
        if n != 0 {
            data = malloc(h, n << 2, 4);
            h.write(ebp - 0x54, data);
            if data != 0 {
                let mut p = data;
                let mut left = n;
                while left != 0 {
                    h.write(ebp - 0x48, left);
                    if p != 0 {
                        let v: u32 = h.read(src);
                        h.write(p, v);
                        src = src.wrapping_add(4);
                    }
                    p = p.wrapping_add(4);
                    left -= 1;
                    h.write(ebp - 0x48, left);
                }
            }
        }
        h.write(o + 8, n);
        h.write(o + 0xc, data);
        if text == 0 {
            free(h, 0);
            h.write(o + 4, 0u32);
        } else {
            let len = strlen(h, text).wrapping_add(1);
            h.write(ebp - 0x48, len);
            let s = malloc(h, len, 2);
            if s != 0 {
                memcpy(h, s, text, len);
                free(h, 0);
                h.write(o + 4, s);
            }
        }
        let scale = copy_float(h, ebp - 0x34);
        h.write(o + 0x10, scale);
        let n = h.read::<u32>(this + 0x7c) & MASK;
        let block = malloc(h, n.wrapping_add(1) << 5, 4);
        h.call_sym(PUSH_INFO, Conv::Thiscall(this + 0x7c), &[block, o]);
        h.write(o, h.rebase(INFO_VTABLE));
        let data: u32 = h.read(o + 0xc);
        if h.read::<u32>(o + 8) >> 31 == 0 && data != 0 {
            free(h, data);
        }
        let text: u32 = h.read(o + 4);
        if text & 1 == 0 {
            free(h, text);
        }
    }

    /// Material `k` of the parameter array `params`: unnamed ones are
    /// skipped; a `Mat_CTA` one gets its record, the others go to
    /// `this + 0x74`.
    fn scene_material(h: &mut impl Host, ebp: u32, this: u32, params: u32, k: u32) {
        let data = h.call_sym(ARRAY_DATA, Conv::Thiscall(params), &[]);
        let mat: u32 = h.read(data.wrapping_add(k.wrapping_mul(4)));
        h.write(ebp - 0x34, mat);
        let text = untag(h.read(mat + 0x10));
        let empty = h.rebase(EMPTY);
        if streq(h, text, empty) {
            return;
        }
        let cta = h.rebase(MAT_CTA);
        if import(h, STRSTR, &[text, cta]) == 0 {
            if h.read::<u32>(ebp - 0x44) != 0 {
                let n = h.read::<u32>(this + 0x74) & MASK;
                h.write(this + 0xc, n);
                h.write(ebp - 0x44, 0u32);
            }
            push_word(h, PUSH_65FAC0, this + 0x74, ebp - 0x34);
            return;
        }
        let o = h.call_sym(NEW, Conv::Cdecl, &[0x7c]);
        if o != 0 {
            h.call_sym(MEMSET, Conv::Cdecl, &[o, 0, 0x7c]);
        }
        h.write(o, mat);
        h.write(o + 0x74, k);
        let index: u32 = h.read(ebp - 0x50);
        h.write(o + 0x78, index);
        h.write(o + 0x70, 0u16);
        h.write(ebp - 0x3c, o);
        for (i, s) in (0..).zip(SAMPLERS) {
            let p = find(h, mat, s);
            h.write(o + 4 + 4 * i, p);
        }
        for (i, rec) in (0..).zip(SAMPLER_RECORDS) {
            let p: u32 = h.read(o + 4 + 4 * i);
            if p == 0 {
                continue;
            }
            let rec = ebp - rec;
            h.write(rec, 0xffu32);
            let shadows = h.call_sym(SHADOW_INSTANCE, Conv::Cdecl, &[]);
            let map = h.call_sym(DUMMY_SHADOW_MAP, Conv::Thiscall(shadows), &[]);
            let texture = if h.read::<u8>(map + 4) == 0 {
                h.read(map)
            } else {
                0
            };
            h.write(rec + 0xc, texture);
            h.write(rec + 8, this + 0x38);
            h.write(rec + 4, 0u32);
            let buffer: u32 = h.read(mat + 4);
            h.call_sym(WRITE_PARAM, Conv::Thiscall(buffer), &[p, rec]);
        }
        for (i, s) in (0..).zip(LAYERS) {
            let p = find(h, mat, s);
            h.write(o + 0x20 + 4 * i, p);
        }
        for (i, slot) in (0..).zip(LAYER_SLOTS) {
            let p: u32 = h.read(o + 0x20 + 4 * i);
            if p != 0 {
                h.write(ebp - slot, 0u32);
                set_parameter(h, ebp, mat, p, ebp - slot);
            }
        }
        for (i, s) in (0..).zip(TRANSFORMS) {
            let p = find(h, mat, s);
            h.write(o + 0x38 + 4 * i, p);
        }
        let p = find(h, mat, SAMPLER_STATE);
        h.write(o + 0x1c, p);
        if p != 0 {
            let rec = ebp - STATE_RECORD;
            h.write(rec + 8, this + 0x38);
            h.write(rec, 0x47u32);
            h.write(rec + 4, 0u32);
            h.write(rec + 0xc, 0u32);
            set_parameter(h, ebp, mat, p, rec);
        }
        // a UV-animated character material: the shader's
        // CHR_TEXTURE_ANIMATION_UV pair with a value other than ""
        let shader: u32 = h.read(mat);
        let n = h.read::<u32>(shader + 4) & MASK;
        h.write(ebp - 0x34, n);
        if n != 0 {
            let pairs: u32 = h.read(shader + 8);
            let uv = h.rebase(CHR_UV);
            for j in 0..n {
                let pair = pairs.wrapping_add(8 * j);
                let key = untag(h.read(pair));
                if key != uv && (key == 0 || !streq(h, key, uv)) {
                    continue;
                }
                let value = untag(h.read(pair.wrapping_add(4)));
                if value == empty || value != 0 && streq(h, value, empty) {
                    continue;
                }
                h.write(o + 0x70, 1u8);
                break;
            }
        }
        h.write(o + 0x71, 1u8);
        push_word(h, PUSH_6B2570, ebp - 0x88, ebp - 0x3c);
    }

    /// `ShaderParams_FindByName` on the material's shader (`+0`).
    fn find(h: &mut impl Host, mat: u32, name: u32) -> u32 {
        let shader: u32 = h.read(mat);
        let name = h.rebase(name);
        h.call_sym(FIND_PARAM, Conv::Thiscall(shader), &[name])
    }

    /// A parameter's bytes (`+8` offset, `+0xa` size & 0x1fff) copied
    /// from `from` into the material's buffer (`mat + 4`, its size at
    /// `+0`), when they fit and the offset is at least 4.
    fn set_parameter(h: &mut impl Host, ebp: u32, mat: u32, p: u32, from: u32) {
        let offset = u32::from(h.read::<u16>(p + 8));
        let size = u32::from(h.read::<u16>(p + 0xa) & 0x1fff);
        let buffer: u32 = h.read(mat + 4);
        h.write(ebp - 0x34, offset);
        if offset + size <= h.read::<u32>(buffer) && offset >= 4 {
            memcpy(h, buffer.wrapping_add(offset), from, size);
        }
    }

    /// The shadow table: an `n` x `n` word table (`n` the Char objects;
    /// columns 1.. -1) filled from the text (row `key:`, then `v!,` or
    /// `v?,` until `\r\n`); each Char object (`+0x20` its name, `#id`)
    /// takes its row's non-negative values (`+0x3c`) and the records whose
    /// material name holds its name followed by nothing or `G` (`+0x38`).
    #[allow(clippy::too_many_lines)] // the original's sequence
    fn shadow_table(h: &mut impl Host, ebp: u32) {
        let n = h.read::<u32>(ebp - 0x90) & MASK;
        let bytes = u64::from(n.wrapping_mul(n)) * 4;
        let size = if bytes >> 32 == 0 {
            bytes as u32
        } else {
            u32::MAX
        };
        let table = h.call_sym(NEW_ARRAY, Conv::Cdecl, &[size]);
        h.write(ebp - 0x4c, table);
        let cell = |row: u32, col: u32| -> u32 {
            table.wrapping_add(n.wrapping_mul(row).wrapping_add(col).wrapping_mul(4))
        };
        for row in 0..n {
            for col in 1..n {
                h.write(cell(row, col), u32::MAX);
            }
        }
        let mut text: u32 = h.read(ebp - 0x38);
        if text != 0 {
            h.write(ebp - 0x50, 0u32);
        }
        if text != 0 && n != 0 {
            let colon_s = h.rebase(COLON);
            let newline_s = h.rebase(NEWLINE);
            let comma_s = h.rebase(COMMA);
            for row in 0..n {
                let colon = import(h, STRSTR, &[text, colon_s]);
                for k in 0..4 {
                    h.write(ebp - KEY_TEXT + 4 * k, 0u32);
                }
                memcpy(h, ebp - KEY_TEXT, text, colon.wrapping_sub(text));
                let key = import(h, ATOI, &[ebp - KEY_TEXT]);
                h.write(cell(row, 0), key);
                text = colon.wrapping_add(1);
                h.write(ebp - 0x38, text);
                let end = import(h, STRSTR, &[colon, newline_s]);
                h.write(ebp - 0x34, end);
                let mut col = 1u32;
                while text != end {
                    let comma = import(h, STRSTR, &[text, comma_s]);
                    let mark: u8 = h.read(comma.wrapping_sub(1));
                    if mark == b'!' || mark == b'?' {
                        for k in 0..4 {
                            h.write(ebp - VALUE_TEXT + 4 * k, 0u32);
                        }
                        let len = comma.wrapping_sub(text).wrapping_sub(1);
                        memcpy(h, ebp - VALUE_TEXT, text, len);
                        let v = import(h, ATOI, &[ebp - VALUE_TEXT]);
                        h.write(cell(row, col), v);
                        text = comma.wrapping_add(1);
                        h.write(ebp - 0x38, text);
                        col = col.wrapping_add(1);
                    }
                }
                h.write(ebp - 0x50, row + 1);
            }
        }
        h.write(ebp - 0x3c, 0u32);
        if n != 0 {
            h.write(ebp - 0x38, 0u32);
            for i in 0..n {
                let objects: u32 = h.read(ebp - 0x8c);
                h.write(ebp - 0x44, objects);
                let object = objects.wrapping_add(0x40 * i);
                let text = untag(h.read(object.wrapping_add(0x20)));
                h.write(ebp - 0x48, text);
                let id = import(h, ATOI, &[text.wrapping_add(1)]);
                h.write(ebp - 0x34, id);
                if let Some(row) = (0..n).find(|&row| h.read::<u32>(cell(row, 0)) == id) {
                    for col in 1..n {
                        let v: u32 = h.read(cell(row, col));
                        if (v as i32) >= 0 {
                            h.write(ebp - 0x34, v);
                            let array: u32 = h.read(object.wrapping_add(0x3c));
                            push_word(h, APPEND_6A25E0, array, ebp - 0x34);
                        }
                    }
                }
                let records = h.read::<u32>(ebp - 0x88) & MASK;
                for j in 0..records {
                    let list: u32 = h.read(ebp - 0x84);
                    let record: u32 = h.read(list.wrapping_add(4 * j));
                    let mat: u32 = h.read(record);
                    let mat_name = untag(h.read(mat.wrapping_add(0x10)));
                    let hit = import(h, STRSTR, &[mat_name, text]);
                    if hit == 0 {
                        continue;
                    }
                    h.write(ebp - 0x34, text.wrapping_add(1));
                    let len = strlen(h, text);
                    let after: u8 = h.read(hit.wrapping_add(len));
                    if after == 0 || after == b'G' {
                        h.write(ebp - 0x34, record);
                        let array: u32 = h.read(object.wrapping_add(0x38));
                        push_word(h, PUSH_6B2570, array, ebp - 0x34);
                    }
                }
                h.write(ebp - 0x3c, i + 1);
                h.write(ebp - 0x38, 0x40 * (i + 1));
            }
        }
        h.call_sym(DELETE_ARRAY, Conv::Cdecl, &[table]);
    }

    /// The stack material's two names (`+0x18`, `+0x1c`) set to copies of
    /// `name` (each through a temporary copy).
    fn assign_copies(h: &mut impl Host, ebp: u32, name: u32) {
        for slot in [0x80, 0x7c] {
            let n = strlen(h, name).wrapping_add(1);
            let copy = malloc(h, n, 2);
            if copy != 0 {
                memcpy(h, copy, name, n);
            }
            h.write(ebp - 0x44, copy);
            assign_copy(h, ebp, ebp - slot, untag(copy));
            if copy & 1 == 0 {
                free(h, copy);
            }
        }
    }

    /// The stack material into the first empty material of `this + 0x64`
    /// (count) / `+0x68` (0x20-byte elements), else appended (the array
    /// copied into a new block, the old one destroyed when owned).
    fn store_material(h: &mut impl Host, ebp: u32, this: u32) {
        let material = ebp - MATERIAL;
        let n = h.read::<u32>(this + 0x64) & MASK;
        let data: u32 = h.read(this + 0x68);
        for k in 0..n {
            let e = data.wrapping_add(0x20 * k);
            let empty = h.read::<u32>(e + 0x10) & MASK == 0
                && h.read::<u32>(e + 8) & MASK == 0
                && h.read::<u32>(e) & MASK == 0
                && untag(h.read(e + 0x18)) == 0
                && untag(h.read(e + 0x1c)) == 0;
            if empty {
                h.call_sym(MATERIAL_ASSIGN, Conv::Thiscall(e), &[material]);
                return;
            }
        }
        let block = malloc(h, n.wrapping_add(1) << 5, 4);
        h.write(ebp - 0x34, block);
        if block == 0 {
            return;
        }
        let n = h.read::<u32>(this + 0x64) & MASK;
        h.write(ebp - 0x44, n);
        let old: u32 = h.read(this + 0x68);
        h.call_sym(MATERIAL_COPY_ARRAY, Conv::Cdecl, &[block, old, n]);
        let slot = block.wrapping_add(n << 5);
        h.write(ebp - 0x3c, slot);
        if slot != 0 {
            h.call_sym(MATERIAL_COPY, Conv::Thiscall(slot), &[material]);
        }
        let old: u32 = h.read(this + 0x68);
        let word: u32 = h.read(this + 0x64);
        h.write(ebp - 0x44, old);
        if old != block && word >> 31 == 0 && old != 0 {
            for k in 0..word & MASK {
                h.call_sym(
                    MATERIAL_DTOR,
                    Conv::Thiscall(old.wrapping_add(0x20 * k)),
                    &[],
                );
            }
            free(h, old);
        }
        h.write(this + 0x68, block);
        h.write(this + 0x64, n + 1);
    }
}
