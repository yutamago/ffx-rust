//! Small helpers of the render layer that decide results game code reads:
//! a clip-space outcode, `std` container steps the compiler instanced for
//! its maps and heaps, and copies.

/// The originals, over game memory.
#[allow(clippy::many_single_char_names)] // vector components, string bytes
pub mod bridge {
    use crate::host::{Conv, Host};
    use ffx_bridge::{Global, Mem, x87};

    /// The map id `graphicIsMapInList` (`0x00642610`) tests.
    pub const MAP_ID: Global<u32> = Global::new(0x0133_0844);

    /// The maps `graphicIsMapInList` lists.
    pub const LISTED_MAPS: [u32; 16] = [
        0x118, 0x52, 0x51, 0xde, 0x13c, 0x17d, 0x5d, 0x12, 0x15d, 0xf8, 0x99, 0xb3, 0x50, 0x137,
        0x16b, 0x84,
    ];

    /// `ClipOutcode(v)` (`0x00652470`): the outcode of a homogeneous
    /// point `(x, y, z, w)`: bits 0-2 for `x`, `y`, `z` above `w`, bits 3-5
    /// for them below `-w` (each difference or sum stored as a float
    /// first; a NaN sets nothing).
    pub fn clip_outcode(m: &impl Mem, v: u32) -> u32 {
        let f = |i: u32| x87::load(x87::store(x87::load(m.read(v + 4 * i))));
        let (x, y, z, w) = (f(0), f(1), f(2), f(3));
        let st = |a: f64| x87::load(x87::store(a));
        let above = [st(x87::sub(x, w)), st(x87::sub(y, w)), st(x87::sub(z, w))];
        let below = [st(x87::add(w, x)), st(x87::add(w, y)), st(x87::add(w, z))];
        let mut code = 0;
        for (i, d) in (0u32..).zip(above) {
            if d > 0.0 {
                code |= 1 << i;
            }
        }
        for (i, s) in (3u32..).zip(below) {
            if s < 0.0 {
                code |= 1 << i;
            }
        }
        code
    }

    /// `graphicIsMapInList()` (`0x00642610`): 1 when the current map
    /// (`0x01330844`) is one of [`LISTED_MAPS`], else 0.
    pub fn is_map_in_list(m: &impl Mem) -> u32 {
        let id = MAP_ID.get(m);
        u32::from(LISTED_MAPS.contains(&id))
    }

    /// `Vec3_UninitializedCopy(first, last, dest)` (`0x006b4f80`): copies
    /// the 12-byte elements `[first, last)` to `dest` (a null `dest`
    /// copies nothing but still advances). Returns the end of `dest`.
    pub fn vec3_uninitialized_copy(m: &mut impl Mem, first: u32, last: u32, dest: u32) -> u32 {
        let (mut from, mut to) = (first, dest);
        while from != last {
            if to != 0 {
                for k in 0..3 {
                    let w: u32 = m.read(from + 4 * k);
                    m.write(to + 4 * k, w);
                }
            }
            from = from.wrapping_add(12);
            to = to.wrapping_add(12);
        }
        to
    }

    /// `ShadowManager_GetCascadeMatrix(this, out, i)` (`0x006d23a0`): the
    /// 4x4 matrix at `this +0x210 + 64 i` to `out`. Returns `out`.
    pub fn shadow_get_cascade_matrix(m: &mut impl Mem, this: u32, out: u32, i: u32) -> u32 {
        let from = this.wrapping_add(0x210).wrapping_add(i << 6);
        for k in 0..16 {
            let w: u32 = m.read(from + 4 * k);
            m.write(out + 4 * k, w);
        }
        out
    }

    /// `FloatHeap_AdjustHeap(base, hole, len, &value, less)` (`0x006c7250`):
    /// `std::_Adjust_heap` on floats with a `bool less(a, b)` (cdecl):
    /// sifts the hole down to a leaf, then pushes `value` up from there.
    /// Returns `&value`.
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_wrap)] // signed indices
    pub fn float_adjust_heap(
        h: &mut impl Host,
        base: u32,
        hole: u32,
        len: u32,
        value: u32,
        less: u32,
    ) -> u32 {
        let at = |i: i32| base.wrapping_add((i as u32).wrapping_mul(4));
        let top = hole as i32;
        let len = len as i32;
        let mut hole = top;
        let mut child = hole.wrapping_mul(2).wrapping_add(2);
        while child < len {
            let a = rt(h.read(at(child)));
            let b = rt(h.read(at(child - 1)));
            if less_than(h, less, a, b) {
                child -= 1;
            }
            let v = rt(h.read(at(child)));
            h.write(at(hole), v);
            hole = child;
            child = child.wrapping_mul(2).wrapping_add(2);
        }
        if child == len {
            let v = rt(h.read(at(len - 1)));
            h.write(at(hole), v);
            hole = len - 1;
        }
        let mut parent = (hole - 1) / 2;
        while top < hole {
            let a = rt(h.read(at(parent)));
            let b = rt(h.read(value));
            if !less_than(h, less, a, b) {
                break;
            }
            let v = rt(h.read(at(parent)));
            h.write(at(hole), v);
            hole = parent;
            parent = (hole - 1) / 2;
        }
        let v = rt(h.read(value));
        h.write(at(hole), v);
        value
    }

    /// The heap's `bool less(a, b)`: its `al`.
    fn less_than(h: &mut impl Host, less: u32, a: u32, b: u32) -> bool {
        h.call(less, Conv::Cdecl, &[a, b]).to_le_bytes()[0] != 0
    }

    /// A float as `fld`/`fstp` moves it.
    fn rt(bits: u32) -> u32 {
        x87::round_trip(f32::from_bits(bits)).to_bits()
    }

    /// Whether the `std::map` node is the head (its `+0xd` byte).
    fn is_nil(m: &impl Mem, node: u32) -> bool {
        m.read::<u8>(node + 0xd) != 0
    }

    /// `MapIterator_Decrement(it)` (`0x0064f360`, fastcall): `--it` of a
    /// `std::map` iterator (nodes `+0` left, `+4` parent, `+8` right):
    /// from the head to the last node, else the left subtree's last node,
    /// else up past the parents it is the left child of. Returns `it`.
    pub fn map_iterator_decrement(m: &mut impl Mem, it: u32) -> u32 {
        let node: u32 = m.read(it);
        if is_nil(m, node) {
            let last: u32 = m.read(node + 8);
            m.write(it, last);
            return it;
        }
        let left: u32 = m.read(node);
        if !is_nil(m, left) {
            let mut n = left;
            loop {
                let r: u32 = m.read(n + 8);
                if is_nil(m, r) {
                    break;
                }
                n = r;
            }
            m.write(it, n);
            return it;
        }
        let mut p: u32 = m.read(node + 4);
        while !is_nil(m, p) && m.read::<u32>(it) == m.read::<u32>(p) {
            m.write(it, p);
            p = m.read(p + 4);
        }
        let cur: u32 = m.read(it);
        if !is_nil(m, cur) {
            m.write(it, p);
        }
        it
    }

    /// A `PString`'s text: the pointer with its flag bit 0 cleared.
    fn pstring_text(m: &impl Mem, at: u32) -> u32 {
        let p: u32 = m.read(at);
        p.wrapping_sub(p & 1)
    }

    /// `a < b` for two `PString` texts: by pointer when either is null,
    /// else as `strcmp`.
    fn pstring_less(m: &impl Mem, a: u32, b: u32) -> bool {
        if a == 0 || b == 0 {
            return a < b;
        }
        let mut i = 0u32;
        loop {
            let (x, y) = (m.read::<u8>(a + i), m.read::<u8>(b + i));
            if x != y {
                return x < y;
            }
            if x == 0 {
                return false;
            }
            i += 1;
        }
    }

    /// `PStringTree_Find(this, &key)` (`0x006be750`): a search tree under
    /// the head `this` (root `+0xc`; nodes `+0` lower, `+4` higher, `+0xc`
    /// a `PString` key) by text. The node, else 0.
    pub fn pstring_tree_find(m: &impl Mem, this: u32, key: u32) -> u32 {
        let k = pstring_text(m, key);
        let mut node: u32 = m.read(this + 0xc);
        while node != this {
            let nk = pstring_text(m, node + 0xc);
            node = if pstring_less(m, k, nk) {
                m.read(node)
            } else if pstring_less(m, nk, k) {
                m.read(node + 4)
            } else {
                return node;
            };
        }
        0
    }

    // ---- round 7 ----

    /// `ShadowManager_SetQuality(this, q)` (`0x006ceaa0`): `+0x4060 = q`,
    /// the changed byte `+0x4064`, and `g_ShadowSize` 4096, 2048 or 1024
    /// for quality 1, 2, 3. Returns `q - 3` for another quality, else 0.
    pub fn shadow_set_quality(m: &mut impl Mem, this: u32, q: u32) -> u32 {
        use crate::managers::bridge::SHADOW_SIZE;
        m.write(this + 0x4060, q);
        m.write(this + 0x4064, 1u8);
        let size = match q {
            1 => 0x1000,
            2 => 0x800,
            3 => 0x400,
            _ => return q.wrapping_sub(3),
        };
        SHADOW_SIZE.set(m, size);
        0
    }

    /// `BlurEffectEx_SetPositiveParams(this, a, b, c, d)` (`0x006c4620`): each
    /// float above 0 to `+0x5c` .. `+0x68`.
    pub fn set_positive_floats(m: &mut impl Mem, this: u32, v: [f32; 4]) {
        for (i, x) in (0u32..).zip(v) {
            let b = x87::load(x.to_bits());
            if b > 0.0 {
                m.write(this + 0x5c + 4 * i, x87::store(b));
            }
        }
    }

    /// `SetFields_006c47a0(this)` (`0x006c47a0`): clears the byte `+0x58`
    /// and the words `+0..+0x20` and `+0x48..+0x54`. Returns 0.
    pub fn render_params_reset(m: &mut impl Mem, this: u32) -> u32 {
        m.write(this + 0x58, 0u8);
        for off in [
            0, 4, 8, 0xc, 0x14, 0x20, 0x18, 0x10, 0x1c, 0x48, 0x4c, 0x50, 0x54,
        ] {
            m.write(this + off, 0u32);
        }
        0
    }

    /// `FloatHeap_PushHeap(base, hole, top, &value, less)` (`0x006c7730`):
    /// `std::_Push_heap` on floats: moves parents down while `less(parent,
    /// value)`, then stores `value`. Returns `&value`, or `base` when
    /// `hole <= top` from the start.
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_wrap)] // signed indices
    pub fn float_push_heap(
        h: &mut impl Host,
        base: u32,
        hole: u32,
        top: u32,
        value: u32,
        less: u32,
    ) -> u32 {
        let at = |i: i32| base.wrapping_add((i as u32).wrapping_mul(4));
        let (mut hole, top) = (hole as i32, top as i32);
        let mut parent = (hole - 1) / 2;
        if top >= hole {
            let v = rt(h.read(value));
            h.write(at(hole), v);
            return base;
        }
        loop {
            let a = rt(h.read(at(parent)));
            let b = rt(h.read(value));
            if !less_than(h, less, a, b) {
                break;
            }
            let v = rt(h.read(at(parent)));
            h.write(at(hole), v);
            hole = parent;
            parent = (hole - 1) / 2;
            if top >= hole {
                break;
            }
        }
        let v = rt(h.read(value));
        h.write(at(hole), v);
        value
    }

    /// `FloatSort_Median3(a, b, c, less)` (`0x006c74d0`): orders the three
    /// floats so `*a <= *b <= *c` by `less` (swaps through `fld`/`fstp`).
    /// Returns the last `less` call's `eax`.
    pub fn float_median3(h: &mut impl Host, a: u32, b: u32, c: u32, less: u32) -> u32 {
        let mut r = h.call(less, Conv::Cdecl, &[rt(h.read(b)), rt(h.read(a))]);
        if r.to_le_bytes()[0] != 0 {
            let t = rt(h.read(b));
            let v = rt(h.read(a));
            h.write(b, v);
            h.write(a, t);
        }
        let x = rt(h.read(c));
        let y = rt(h.read(b));
        r = h.call(less, Conv::Cdecl, &[x, y]);
        if r.to_le_bytes()[0] != 0 {
            let t = rt(h.read(c));
            let v = rt(h.read(b));
            h.write(c, v);
            let t = rt(t);
            h.write(b, t);
            let av = rt(h.read(a));
            r = h.call(less, Conv::Cdecl, &[t, av]);
            if r.to_le_bytes()[0] != 0 {
                let t = rt(h.read(b));
                let v = rt(h.read(a));
                h.write(b, v);
                h.write(a, t);
            }
        }
        r
    }

    /// `g_ChrTexAnim` entry `i`'s work animation `which` (`0x34` bytes)
    /// restarted at `pos` (`u16 +0x24`): `+0xc`, `+0x1c`, `+0x20` 0 and
    /// the floats `+0x2c` .. `+0x38` 0; in the new-format half (`base` 0,
    /// `TexAnim_StartNew` `0x0077fc90`) or the old one (`base` 0x400,
    /// `TexAnim_StartOld` `0x00780750`). Returns 0, or `i` for an unloaded
    /// entry.
    pub fn tex_anim_start(m: &mut impl Mem, base: u32, i: u32, which: u32, pos: u32) -> u32 {
        use crate::texanim::bridge::CHR_TEXANIM;
        let e = CHR_TEXANIM.ptr(m).addr.wrapping_add(i.wrapping_mul(0x30));
        if m.read::<u32>(e) == 0 {
            return i;
        }
        let work: u32 = m.read(e + 4);
        let a = work
            .wrapping_add(which.wrapping_mul(0x34))
            .wrapping_add(base);
        #[allow(clippy::cast_possible_truncation)] // the u16 store
        m.write(a + 0x24, pos as u16);
        for off in [0x2c, 0x30, 0xc, 0x34, 0x1c, 0x38, 0x20] {
            m.write(a + off, 0u32);
        }
        0
    }

    /// `SetFields_006f7700(this)` (`0x006f7700`, fastcall): the
    /// cheat-overlay icons' Iggy movie paths (`freecamera.swf`,
    /// `time_stop.swf`, `speed_2/4.swf`, `invincible.swf`, `enemy_0/1.swf`,
    /// `autobattle.swf`, `autobattle_disable.swf`) and their byte table.
    /// Returns `this`.
    pub fn init_flash_table(m: &mut impl Mem, this: u32) -> u32 {
        for off in (0..0x1c).step_by(4) {
            m.write(this + off, 0u32);
        }
        for (off, s) in [
            (0x3c, 0x00b4_bff8),
            (0x40, 0x00b4_c028),
            (0x1c, 0x00b4_bf98),
            (0x20, 0x00b4_bfc8),
            (0x24, 0x00b4_c058),
            (0x28, 0x00b4_c088),
            (0x2c, 0x00b4_c0b8),
            (0x30, 0x00b4_c0e8),
            (0x34, 0x00b4_c118),
        ] {
            let p = m.rebase(s);
            m.write(this + off, p);
        }
        m.write(this + 0x38, 0x0201_0000u32);
        m.write(this + 0x3c, 0x0103_0302u32);
        m.write(this + 0x40, 0x504u16);
        this
    }

    /// `ShadowEntry_Init(this, owner)` (`0x006d3560`): owner at `+0`,
    /// kind 2 at `+4`, a scale of 1 at `+0x30` .. `+0x3c`, the rest
    /// cleared. Returns `this`.
    pub fn shadow_entry_init(m: &mut impl Mem, this: u32, owner: u32) -> u32 {
        for off in (0xc..0x30).step_by(4) {
            m.write(this + off, 0u32);
        }
        for off in [0x30, 0x34, 0x38, 0x3c] {
            m.write(this + off, 1f32.to_bits());
        }
        m.write(this, owner);
        m.write(this + 0xa0, 0u32);
        m.write(this + 0xa4, 0u32);
        m.write(this + 4, 2u32);
        for off in [0x40, 0x44, 0x48, 0x4c, 0x50] {
            m.write(this + off, 0u32);
        }
        this
    }

    /// `strcmp(a, b) == 0` as the inlined loops do (no null checks).
    fn str_eq(m: &impl Mem, a: u32, b: u32) -> bool {
        let mut i = 0u32;
        loop {
            let (x, y) = (m.read::<u8>(a + i), m.read::<u8>(b + i));
            if x != y {
                return false;
            }
            if x == 0 {
                return true;
            }
            i += 1;
        }
    }

    /// `TextureAnimationManager_FindMaterialIndex(this, name)`
    /// (`0x006b20a0`): the index of the first non-null material (`+0x78`,
    /// `+0x74` of them, bit 31 a flag) whose `PString` name (`+0x10`) is
    /// `name`, else -1.
    pub fn tex_anim_material_index(m: &impl Mem, this: u32, name: u32) -> u32 {
        let n = m.read::<u32>(this + 0x74) & 0x7fff_ffff;
        let list: u32 = m.read(this + 0x78);
        for i in 0..n {
            let mat: u32 = m.read(list + 4 * i);
            if mat != 0 && str_eq(m, pstring_text(m, mat + 0x10), name) {
                return i;
            }
        }
        u32::MAX
    }

    /// `TextureAnimationManager_FindAnimation(this, name)` (`0x006b1ef0`):
    /// the first animation (`+0x68`, 0x20 bytes each, `+0x64` of them, bit
    /// 31 a flag) whose `PString` name (`+0x18`, skipped when null) is
    /// `name`, else 0 (also for a null `name`).
    pub fn tex_anim_find_animation(m: &impl Mem, this: u32, name: u32) -> u32 {
        if name == 0 {
            return 0;
        }
        let n = m.read::<u32>(this + 0x64) & 0x7fff_ffff;
        let list: u32 = m.read(this + 0x68);
        for i in 0..n {
            let text = pstring_text(m, list + 0x20 * i + 0x18);
            if text != 0 && str_eq(m, text, name) {
                let list: u32 = m.read(this + 0x68);
                return list + 0x20 * i;
            }
        }
        0
    }

    /// `ParamList_SetAllFloat(this, v)` (`0x006b47e0`): each parameter of
    /// `+0x80` (0x20 bytes each, `+0x7c` of them, bit 31 a flag) gets
    /// `v` through its virtual 1.
    pub fn set_all_params(h: &mut impl Host, this: u32, v: f32) {
        let v = rt(v.to_bits());
        let mut i = 0;
        while i < h.read::<u32>(this + 0x7c) & 0x7fff_ffff {
            let list: u32 = h.read(this + 0x80);
            h.call_virtual(list + 0x20 * i, 1, &[v]);
            i += 1;
        }
    }

    /// `ReturnConst180_006c26d0()` and `ReturnConst51_006c2700()`
    /// : constants the distortion set-up asks for.
    pub const RENDER_CONST_180: u32 = 0x180;
    pub const RENDER_CONST_51: u32 = 0x51;

    /// The byte `ReturnGlobalSx8_006bc960` (`0x006bc960`) returns sign-extended.
    pub const RENDER_FLAG: Global<u8> = Global::new(0x00cc_cab8);

    /// `ReturnGlobalSx8_006bc960()`.
    pub fn render_flag(m: &impl Mem) -> u32 {
        #[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)] // movsx
        let v = i32::from(RENDER_FLAG.get(m) as i8) as u32;
        v
    }

    /// `LoadCompute_006f7bd0(this)` (and its copy `LoadCompute_006f7c00`;
    /// fastcall): `(+0x30 + +0x34) * 4`.
    pub fn buffer_size_bytes(m: &impl Mem, this: u32) -> u32 {
        let a: u32 = m.read(this + 0x34);
        let b: u32 = m.read(this + 0x30);
        a.wrapping_add(b).wrapping_mul(4)
    }

    /// `g_PrxState`: the PRX loader's state (1 loading, 2 loading
    /// asynchronously, 3 done, 0 unloaded).
    pub const PRX_STATE: Global<u32> = Global::new(0x00ce_83c4);
    /// `g_PrxModule`: the loaded PRX library (`HMODULE`).
    pub const PRX_MODULE: Global<u32> = Global::new(0x00ce_83c0);

    /// `Compare_006f9b70()`: the PRX load completed.
    pub fn prx_complete(m: &impl Mem) -> u32 {
        u32::from(PRX_STATE.get(m) == 3)
    }

    /// `PhyreFIOS_LoadPrxIdle()` (`0x006f9b80`): complete and no
    /// library held.
    pub fn prx_idle(m: &impl Mem) -> u32 {
        u32::from(PRX_STATE.get(m) == 3 && PRX_MODULE.get(m) == 0)
    }

    // ---- round 9 ----

    /// The game's import thunks of the CRT's `_CIsin` and `_CIcos`.
    pub const CI_SIN: u32 = 0x0094_98c8;
    pub const CI_COS: u32 = 0x0094_98c2;

    /// `sin` and `cos` of the float angle at `angle` as the originals take
    /// them: `fld m32`, the intrinsic, `fstp m32`, and once more through
    /// `fld`/`fstp`.
    fn sin_cos(h: &mut impl Host, angle: u32) -> (u32, u32) {
        let a = x87::load(angle);
        let sin = h.rebase(CI_SIN);
        let s = rt(h.call_intrinsic(sin, a).to_bits());
        let cos = h.rebase(CI_COS);
        let c = rt(h.call_intrinsic(cos, a).to_bits());
        (s, c)
    }

    /// `Matrix_RotationX(out, angle)` (`0x0066c8a0`) and
    /// `Matrix_RotationY(out, angle)` (`0x0066c920`): the 4x4 rotation about
    /// x (rows `[1 0 0 0] [0 c s 0] [0 -s c 0] [0 0 0 1]`) or y (`[c 0 -s 0]
    /// [0 1 0 0] [s 0 c 0] [0 0 0 1]`). Returns `out`.
    pub fn rotation(h: &mut impl Host, out: u32, angle: u32, y_axis: bool) -> u32 {
        let (s, c) = sin_cos(h, angle);
        let ns = s ^ 0x8000_0000;
        let (one, z) = (1f32.to_bits(), 0u32);
        let m = if y_axis {
            [c, z, ns, z, z, one, z, z, s, z, c, z, z, z, z, one]
        } else {
            [one, z, z, z, z, c, s, z, z, ns, c, z, z, z, z, one]
        };
        for (i, v) in (0u32..).zip(m) {
            h.write(out + 4 * i, v);
        }
        out
    }

    /// `PhyreSceneCursor_SetNode(cursor, node)` (`0x00441dc0`): the cursor
    /// from the node: `+8` the stride (`*(node +0x2c) +0x1c`), `+0` the
    /// position (`node +0x34`), `+0xc` the count (`node +0x3c`), `+0x10`
    /// (`*(node +0x2c) +0x8c`), `+4` the next link (`node +8`); then while
    /// counted and the link equals the position, the position steps by the
    /// stride and the link follows itself. Returns the last value moved
    /// into `eax`.
    pub fn cursor_set_node(m: &mut impl Mem, cur: u32, node: u32) -> u32 {
        let obj: u32 = m.read(node + 0x2c);
        let stride: u32 = m.read(obj + 0x1c);
        m.write(cur + 8, stride);
        let pos: u32 = m.read(node + 0x34);
        m.write(cur, pos);
        let count: u32 = m.read(node + 0x3c);
        m.write(cur + 0xc, count);
        let zero = m.read::<u32>(cur + 0xc) == 0;
        let v: u32 = m.read(obj + 0x8c);
        m.write(cur + 0x10, v);
        let mut eax: u32 = m.read(node + 8);
        m.write(cur + 4, eax);
        if zero {
            return eax;
        }
        loop {
            let link: u32 = m.read(cur + 4);
            let pos: u32 = m.read(cur);
            if link != pos {
                return eax;
            }
            eax = m.read::<u32>(cur + 8).wrapping_add(pos);
            let n = m.read::<u32>(cur + 0xc).wrapping_sub(1);
            m.write(cur + 0xc, n);
            m.write(cur, eax);
            eax = m.read(link);
            m.write(cur + 4, eax);
            if n == 0 {
                return eax;
            }
        }
    }

    /// `ResetAndConstruct_006c40c0(this)` (`0x006c40c0`): the fields reset
    /// (`SetFields_006c47a0`), then the member at `+0x24` set up
    /// (`0x00585230`, a tail call: its `eax`).
    pub fn reset_and_construct(h: &mut impl Host, this: u32) -> u32 {
        h.call_sym(RESET_FIELDS, Conv::Thiscall(this), &[]);
        h.call_sym(MEMBER_INIT_585230, Conv::Thiscall(this + 0x24), &[])
    }

    /// `SetFields_006c47a0`.
    pub const RESET_FIELDS: u32 = 0x006c_47a0;
    /// The `+0x24` member's set-up `0x00585230`.
    pub const MEMBER_INIT_585230: u32 = 0x0058_5230;
    /// The list cursor's advance `0x00464f30` (fastcall).
    pub const CURSOR_NEXT_464F30: u32 = 0x0046_4f30;

    /// `PostProcessManager_SetFirstEntry(this, list)` (`0x00698400`):
    /// `+0x1200` cleared, then with a list, a cursor on it (its node
    /// `list +0x1c`, the end `*(list +0x1c)` unless that is the node itself)
    /// is advanced (`0x00464f30`) and, when it has a count, `+0x1200` gets
    /// its position plus its first word. Returns 0.
    pub fn post_process_set_first_entry(h: &mut impl Host, this: u32, list: u32) -> u32 {
        h.write(this + 0x1200, 0u32);
        if list == 0 {
            return 0;
        }
        let node = list + 0x1c;
        let next: u32 = h.read(node);
        let end = if next == node { 0 } else { next };
        let cur = h.scratch();
        let tag = h.rebase(0x00ca_37d0);
        for (i, v) in (0u32..).zip([0, 0, 0, 0, 0, node, end, tag]) {
            h.write(cur + 4 * i, v);
        }
        h.call_sym(CURSOR_NEXT_464F30, Conv::Fastcall, &[cur]);
        if h.read::<u32>(cur + 0xc) != 0 {
            let v = h.read::<u32>(cur + 0x10).wrapping_add(h.read(cur));
            h.write(this + 0x1200, v);
        }
        0
    }

    /// `TextureAnimationManager_SetFrame(this, i, frame, absolute)`
    /// (`0x006b42c0`): material `i` (offset by `+8 + +0xc` unless
    /// `absolute`) names a texture animation (`TextureAnimation_FindByName`),
    /// which is unpaused (`+0x1c`) and, when `frame` is below its frame
    /// count (`+8`, bit 31 a flag), set to it (`+0x18`). Returns `eax`: 1
    /// in `al` (0 when the name finds no animation), over the frame count,
    /// the material count, or 0.
    pub fn tex_anim_set_frame(
        m: &mut impl Mem,
        this: u32,
        i: u32,
        frame: u32,
        absolute: u32,
    ) -> u32 {
        use crate::managers::bridge::find_by_name;
        let i = if absolute == 0 {
            let a: u32 = m.read(this + 0xc);
            let b: u32 = m.read(this + 8);
            i.wrapping_add(a.wrapping_add(b))
        } else {
            i
        };
        let count = m.read::<u32>(this + 0x74) & 0x7fff_ffff;
        if i >= count {
            return (count & !0xff) | 1;
        }
        let list: u32 = m.read(this + 0x78);
        let mat: u32 = m.read(list + 4 * i);
        if mat == 0 {
            return 1;
        }
        let name = pstring_text(m, mat + 0x10);
        let anim = find_by_name(m, this, name);
        if anim == 0 {
            return 0;
        }
        let n = m.read::<u32>(anim + 8) & 0x7fff_ffff;
        m.write(anim + 0x1c, 0u8);
        if frame < n {
            m.write(anim + 0x18, frame);
        }
        (n & !0xff) | 1
    }

    // ---- round 10 ----

    /// `FloatHeap_MakeHeap(first, last, less)` (`0x006c7410`):
    /// `std::_Make_heap` on floats: from the last parent down, each value
    /// is sifted to a leaf and pushed back up (`FloatHeap_PushHeap`).
    /// Returns `eax`: the last parent index (0 after any work).
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_wrap)] // signed indices
    pub fn float_make_heap(h: &mut impl Host, first: u32, last: u32, less: u32) -> u32 {
        let at = |i: i32| first.wrapping_add((i as u32).wrapping_mul(4));
        let n = (last.wrapping_sub(first) as i32) >> 2;
        let mut half = n / 2;
        if half <= 0 {
            return half as u32;
        }
        let mut edx = half * 2 + 2;
        let value = h.scratch() + 0x100;
        while half > 0 {
            let v = rt(h.read(at(half - 1)));
            h.write(value, v);
            half -= 1;
            edx -= 2;
            let mut hole = half;
            let mut child = edx;
            while child < n {
                let a = rt(h.read(at(child)));
                let b = rt(h.read(at(child - 1)));
                if less_than(h, less, a, b) {
                    child -= 1;
                }
                let v = rt(h.read(at(child)));
                h.write(at(hole), v);
                hole = child;
                child = child * 2 + 2;
            }
            if child == n {
                let v = rt(h.read(at(n - 1)));
                h.write(at(hole), v);
                hole = n - 1;
            }
            float_push_heap(h, first, hole as u32, half as u32, value, less);
        }
        half as u32
    }

    /// `FloatSort_Insertion(first, last, less)` (`0x006c7320`):
    /// `std::_Insertion_sort` on floats: a value less than the first moves
    /// the run up by one with `memmove` (bytes unchanged) and goes first;
    /// otherwise it sinks past larger neighbours (moved through
    /// `fld`/`fstp`).
    pub fn float_insertion_sort(h: &mut impl Host, first: u32, last: u32, less: u32) {
        if first == last {
            return;
        }
        let mut cur = first + 4;
        while cur != last {
            let v = rt(h.read(cur));
            let f = rt(h.read(first));
            if less_than(h, less, v, f) {
                let n = cur - first;
                let mut run = vec![0u8; n as usize];
                h.read_bytes(first, &mut run);
                h.write_bytes(first + 4, &run);
                h.write(first, rt(v));
            } else {
                let mut hole = cur;
                let mut prev = cur - 4;
                loop {
                    let p = rt(h.read(prev));
                    if !less_than(h, less, v, p) {
                        break;
                    }
                    let p = rt(h.read(prev));
                    h.write(hole, p);
                    hole = prev;
                    prev -= 4;
                }
                h.write(hole, rt(v));
            }
            cur += 4;
        }
    }

    /// The `PArray` growths `0x0066c700` and `0x0066c690` (`thiscall(array,
    /// capacity)`).
    pub const ARRAY_GROW_3890: u32 = 0x0066_c700;
    pub const ARRAY_GROW_3898: u32 = 0x0066_c690;

    /// `PhyreScene_PushPair3890(this, a, b)` (`0x00656b90`): `a` and `b`
    /// appended at the count `+0x38a0` to the arrays `+0x3894` and
    /// `+0x389c`; when the first array (`+0x3890` capacity, bit 31 a flag)
    /// is full both grow by 0x400. Returns the second growth's `eax`, or
    /// the first array's capacity.
    pub fn scene_push_pair(h: &mut impl Host, this: u32, a: u32, b: u32) -> u32 {
        let n: u32 = h.read(this + 0x38a0);
        let arr: u32 = h.read(this + 0x3894);
        h.write(arr + 4 * n, a);
        let n: u32 = h.read(this + 0x38a0);
        let arr: u32 = h.read(this + 0x389c);
        h.write(arr + 4 * n, b);
        let cap = h.read::<u32>(this + 0x3890) & 0x7fff_ffff;
        let n = h.read::<u32>(this + 0x38a0).wrapping_add(1);
        h.write(this + 0x38a0, n);
        if n < cap {
            return cap;
        }
        h.call_sym(
            ARRAY_GROW_3890,
            Conv::Thiscall(this + 0x3890),
            &[cap + 0x400],
        );
        let cap2 = h.read::<u32>(this + 0x3898) & 0x7fff_ffff;
        h.call_sym(
            ARRAY_GROW_3898,
            Conv::Thiscall(this + 0x3898),
            &[cap2 + 0x400],
        )
    }

    /// The grad-mesh pool's current set (`0x00cdec24`).
    pub const GRAD_MESH_SET: Global<u32> = Global::new(0x00cd_ec24);
    /// The CRT `printf` import slot.
    pub const PRINTF_SLOT: u32 = 0x00b0_c380;

    /// `GetASpareGradMeshInstance(this, &extra)` (`0x006cc980`): the first
    /// free one of the current set's 16 slots (bytes `+0xdc4`, 16 per set)
    /// is taken; `extra` gets its `+0xd44` word and its `+0xcc4` word is
    /// returned. With none free, a message (`GetASpareGradMeshInstance() , Shouldn't
    /// get here, contact cg please`) is printed and 0 returned.
    pub fn get_spare_grad_mesh(h: &mut impl Host, this: u32, out: u32) -> u32 {
        let set = GRAD_MESH_SET.get(h);
        let base = this.wrapping_add(set.wrapping_mul(16));
        for i in 0..16 {
            if h.read::<u8>(base + 0xdc4 + i) == 0 {
                h.write(base + 0xdc4 + i, 1u8);
                let set = GRAD_MESH_SET.get(h);
                let k = set.wrapping_mul(16).wrapping_add(i);
                let extra: u32 = h.read(this.wrapping_add(4 * k) + 0xd44);
                h.write(out, extra);
                let set = GRAD_MESH_SET.get(h);
                let k = set.wrapping_mul(16).wrapping_add(i);
                return h.read(this.wrapping_add(4 * k) + 0xcc4);
            }
        }
        let slot = h.rebase(PRINTF_SLOT);
        let printf: u32 = h.read(slot);
        let msg = h.rebase(0x00b4_97c0);
        h.call(printf, Conv::Cdecl, &[msg]);
        0
    }

    // ---- round 11 ----

    /// `TkMenuGetCurrentPlayerPos` and `TkMenuGetPlayerFromIndex2_008a9b00`.
    pub const MENU_CURRENT_PLAYER_POS: u32 = 0x008a_9870;
    pub const MENU_PLAYER_FROM_INDEX: u32 = 0x008a_9b00;

    /// `graphicGetCurrentFrame()` (`0x008e1c50`): the menu's player at its
    /// current position. Returns its `eax`.
    pub fn get_current_frame(h: &mut impl Host) -> u32 {
        let pos = h.call_sym(MENU_CURRENT_PLAYER_POS, Conv::Cdecl, &[]);
        h.call_sym(MENU_PLAYER_FROM_INDEX, Conv::Cdecl, &[pos])
    }

    /// `TextureAnimationManager_GetInstance` and `TextureAnimationManager_SetFrame`.
    pub const TEX_ANIM_GET_INSTANCE: u32 = 0x006b_02e0;
    pub const TEX_ANIM_SET_FRAME: u32 = 0x006b_42c0;

    /// `graphicTextureAnimationSetFrame(i, frame)` (`0x00643a20`):
    /// `TextureAnimationManager_SetFrame(i, frame, 0)` on the manager.
    pub fn graphic_tex_anim_set_frame(h: &mut impl Host, i: u32, frame: u32) -> u32 {
        let mgr = h.call_sym(TEX_ANIM_GET_INSTANCE, Conv::Cdecl, &[]);
        h.call_sym(TEX_ANIM_SET_FRAME, Conv::Thiscall(mgr), &[i, frame, 0])
    }

    /// `TexAnim_ResetPos` and `tex_anim_pos_set_old`.
    pub const TEX_ANIM_RESET_POS: u32 = 0x0077_fda0;
    pub const TEX_ANIM_POS_SET_OLD: u32 = 0x0078_08f0;

    /// `TexAnim_PosSet(i, which, v)` (`0x0077e9a0`): by `g_ChrTexAnim[i]`'s
    /// format (`+0xd`): old (0) `tex_anim_pos_set_old`, new (1)
    /// `TexAnim_ResetPos` (tail calls: their `eax`); else nothing
    /// (`eax` the format - 1).
    pub fn tex_anim_pos_set(h: &mut impl Host, i: u32, which: u32, v: u32) -> u32 {
        use crate::texanim::bridge::CHR_TEXANIM;
        let e = CHR_TEXANIM.ptr(h).addr.wrapping_add(i.wrapping_mul(0x30));
        #[allow(clippy::cast_sign_loss)] // movsx
        let fmt = i32::from(h.read::<i8>(e + 0xd)) as u32;
        match fmt {
            0 => h.call_sym(TEX_ANIM_POS_SET_OLD, Conv::Cdecl, &[i, which, v]),
            1 => h.call_sym(TEX_ANIM_RESET_POS, Conv::Cdecl, &[i, which, v]),
            _ => fmt.wrapping_sub(1),
        }
    }

    /// The CRT `sprintf` import slot.
    pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;

    /// `PhyreScene_SetNumberText(this, n)` (`0x0066e5e0`): with `+0xfc00`
    /// set, `+0xfc64 = n` and its decimal text (`sprintf "%d"`) at `+0xfc68`.
    pub fn scene_set_number_text(h: &mut impl Host, scene: u32, n: u32) {
        if h.read::<u8>(scene + 0xfc00) == 0 {
            return;
        }
        h.write(scene + 0xfc64, n);
        let slot = h.rebase(SPRINTF_SLOT);
        let sprintf: u32 = h.read(slot);
        let fmt = h.rebase(0x00b0_f604);
        h.call(sprintf, Conv::Cdecl, &[scene + 0xfc68, fmt, n]);
    }

    /// `TextureAnimationManager_SetPaused(this, i, paused)` (`0x006b3600`):
    /// the animation named by material `i + +8 + +0xc` gets its `+0x1c`
    /// byte.
    pub fn tex_anim_set_paused(m: &mut impl Mem, this: u32, i: u32, paused: u32) {
        use crate::managers::bridge::find_by_name;
        let a: u32 = m.read(this + 0xc);
        let b: u32 = m.read(this + 8);
        let i = i.wrapping_add(a.wrapping_add(b));
        if i >= m.read::<u32>(this + 0x74) & 0x7fff_ffff {
            return;
        }
        let list: u32 = m.read(this + 0x78);
        let mat: u32 = m.read(list + 4 * i);
        if mat == 0 {
            return;
        }
        let name = pstring_text(m, mat + 0x10);
        let anim = find_by_name(m, this, name);
        if anim != 0 {
            #[allow(clippy::cast_possible_truncation)] // the byte argument
            m.write(anim + 0x1c, paused as u8);
        }
    }

    /// `BrokenScreenEffect_GetInstance`.
    pub const BROKEN_SCREEN_GET_INSTANCE: u32 = 0x006d_4de0;

    /// `BrokenScreenEffect_BufferSize(n)` (`0x006393c0`): the vertex bytes
    /// for `n` pieces, `((8n-1)/16 + 5 + (24n-49)/16 + 2*((12n-1)/16) +
    /// (16n-1)/16) * 32` in unsigned arithmetic (after the instance getter).
    pub fn broken_screen_buffer_size(h: &mut impl Host, n: u32) -> u32 {
        h.call_sym(BROKEN_SCREEN_GET_INSTANCE, Conv::Cdecl, &[]);
        let a = n.wrapping_mul(24).wrapping_sub(0x31) >> 4;
        let b = n.wrapping_mul(12).wrapping_sub(1) >> 4;
        let c = n.wrapping_mul(16).wrapping_sub(1) >> 4;
        let d = (n.wrapping_mul(8).wrapping_sub(1) >> 4).wrapping_add(5);
        let sum = a
            .wrapping_add(b.wrapping_mul(2))
            .wrapping_add(c)
            .wrapping_add(d);
        sum << 5
    }

    /// The scene's draw check `0x006721b0` (`thiscall(scene)`).
    pub const SCENE_CHECK_6721B0: u32 = 0x0067_21b0;

    /// `PhyreScene_AddDrawEntry(this, e, kind)` (`0x00656c10`): up to 0xc00
    /// entries each: with `kind`, `e`'s `+0xc` to the list `+0x38cc` (count
    /// `+0xf8cc`); else `e` to `+0x68cc` (count `+0xf8d0`), and a set
    /// `0x006721b0` check sets the byte `+0x10d2c`.
    #[allow(clippy::cast_possible_wrap)] // signed counts
    pub fn scene_add_draw_entry(h: &mut impl Host, scene: u32, e: u32, kind: u32) {
        if kind & 0xff != 0 {
            let n: u32 = h.read(scene + 0xf8cc);
            if (n as i32) < 0xc00 {
                let v: u32 = h.read(e + 0xc);
                h.write(scene.wrapping_add(n.wrapping_mul(4)) + 0x38cc, v);
                h.write(scene + 0xf8cc, n.wrapping_add(1));
            }
            return;
        }
        let n: u32 = h.read(scene + 0xf8d0);
        if (n as i32) >= 0xc00 {
            return;
        }
        h.write(scene.wrapping_add(n.wrapping_mul(4)) + 0x68cc, e);
        let n = h.read::<u32>(scene + 0xf8d0).wrapping_add(1);
        h.write(scene + 0xf8d0, n);
        if h.call_sym(SCENE_CHECK_6721B0, Conv::Thiscall(scene), &[]) != 0 {
            h.write(scene + 0x10d2c, 1u8);
        }
    }

    /// The scene calls of `ShadowManager_ResetSolution`.
    pub const SCENE_UPDATE_SHADOW_CLUSTERS: u32 = 0x0065_7920;
    pub const SCENE_FORCE_SET_SHADOW_SOLUTION: u32 = 0x0065_13f0;

    /// `ShadowManager_ResetSolution(this, keep)` (`0x006ceb10`): the shadow
    /// state reset (`+0x403c` from `0x00c34400`, flags `+0x4030`/`+0x4044`/
    /// `+0x404c`/`+0x4054`, the compare callback `+0x4048` =
    /// `Compute_006cc8b0`, `+0x4058`/`+0x405c` 0); without `keep` also
    /// `+0x4034` = -10, `+0x400c` set and `+0x403c` again. Then the scene's
    /// shadow clusters and `PhyreScene_ForceSetShadowSolution(keep ? 0 :
    /// 2)` (a tail call: its `eax`).
    pub fn shadow_reset_solution(h: &mut impl Host, this: u32, keep: u32) -> u32 {
        let d = rt(h.read(h.rebase(0x00c3_4400)));
        h.write(this + 0x403c, d);
        h.write(this + 0x4030, 0u8);
        h.write(this + 0x4044, 1u8);
        let cb = h.rebase(0x006c_c8b0);
        h.write(this + 0x4048, cb);
        h.write(this + 0x404c, 1u8);
        h.write(this + 0x4054, 0u8);
        h.write(this + 0x4058, 0u32);
        h.write(this + 0x405c, 0u32);
        let arg = if keep != 0 {
            0
        } else {
            h.write(this + 0x404c, 1u8);
            let k = rt(h.read(h.rebase(0x00b3_04fc)));
            h.write(this + 0x4034, k);
            let d = rt(h.read(h.rebase(0x00c3_4400)));
            h.write(this + 0x400c, 1u8);
            h.write(this + 0x403c, d);
            2
        };
        let scene = crate::scene::bridge::SCENE.get(h);
        h.call_sym(SCENE_UPDATE_SHADOW_CLUSTERS, Conv::Thiscall(scene), &[]);
        let scene = crate::scene::bridge::SCENE.get(h);
        h.call_sym(
            SCENE_FORCE_SET_SHADOW_SOLUTION,
            Conv::Thiscall(scene),
            &[arg],
        )
    }

    // ---- round 12 ----

    /// `PhyreFIOS_LoadPrxInProgress()` (`0x006f9ba0`): the PRX loader's
    /// state is 1 (a synchronous load) or 2 (an asynchronous one).
    pub fn prx_in_progress(m: &impl Mem) -> u32 {
        u32::from(matches!(PRX_STATE.get(m), 1 | 2))
    }

    /// `TextureAnimationInfo_update(this, dt)` (`0x006b4450`, slot 1 of
    /// `TextureAnimationInfo_vftable` and `..Char_vftable`): while playing
    /// (byte `+0x1c`), the timer `+0x14` grows by `dt` (stored as a float);
    /// once it reaches the period `+0x10` (an ordered compare) it restarts
    /// at 0 and the frame `+0x18` steps by one modulo the count (`+8`,
    /// bit 31 a flag; `(frame + 1 + count) % count`, unsigned). With a
    /// count of 0 the original faults (`div`); this leaves the frame.
    pub fn tex_anim_info_update(m: &mut impl Mem, this: u32, dt: f32) {
        if m.read::<u8>(this + 0x1c) == 0 {
            return;
        }
        let t = x87::Reg::load(dt).add_m32(m.read(this + 0x14)).store();
        m.write(this + 0x14, t);
        let period: f32 = m.read(this + 0x10);
        let reached = matches!(
            x87::load(period.to_bits()).partial_cmp(&x87::load(t.to_bits())),
            Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
        );
        if !reached {
            return;
        }
        m.write(this + 0x14, 0u32);
        let count = m.read::<u32>(this + 8) & 0x7fff_ffff;
        let v = m
            .read::<u32>(this + 0x18)
            .wrapping_add(1)
            .wrapping_add(count);
        if let Some(r) = v.checked_rem(count) {
            m.write(this + 0x18, r);
        }
    }

    /// `TextureAnimationManager_GetByIndexList(this, list, i, absolute)`
    /// (`0x006b2140`): entry `i` of the index list (`+8` count, bit 31 a
    /// flag; `+0xc` the indices), unless -1, plus `+0x10` unless
    /// `absolute`, picks a word of `+0x60` (`+0x5c` count, bit 31 a flag);
    /// 0 for a null list or any index out of range.
    pub fn tex_anim_by_index_list(
        m: &impl Mem,
        this: u32,
        list: u32,
        i: u32,
        absolute: u32,
    ) -> u32 {
        if list == 0 || i >= m.read::<u32>(list + 8) & 0x7fff_ffff {
            return 0;
        }
        let indices: u32 = m.read(list + 0xc);
        let mut k: u32 = m.read(indices.wrapping_add(i.wrapping_mul(4)));
        if k == u32::MAX {
            return 0;
        }
        if absolute == 0 {
            k = k.wrapping_add(m.read(this + 0x10));
        }
        if k >= m.read::<u32>(this + 0x5c) & 0x7fff_ffff {
            return 0;
        }
        let words: u32 = m.read(this + 0x60);
        m.read(words.wrapping_add(k.wrapping_mul(4)))
    }

    /// `tex_anim_no_set_old(i, a, v)` (`0x007807c0`): for `a == 4` and a
    /// loaded `g_ChrTexAnim[i]` (`+0` set), `+8 = v` and the eight old-format
    /// animations of its work buffer (`0x34` bytes each from `+0x400`) get
    /// `+0xc`, `+0x1c`, `+0x20`, `+0x34` and `+0x38` zeroed. Returns its
    /// `eax`: the work buffer then, `i` for an empty entry, and for
    /// `a != 4` the `eax` it was called with, which its one caller
    /// (`tex_anim_no_set`) sets to `v` before the call.
    pub fn tex_anim_no_set_old(m: &mut impl Mem, i: u32, a: u32, v: u32) -> u32 {
        use crate::texanim::bridge::CHR_TEXANIM;
        if a != 4 {
            return v;
        }
        let e = CHR_TEXANIM.ptr(m).addr.wrapping_add(i.wrapping_mul(0x30));
        if m.read::<u32>(e) == 0 {
            return i;
        }
        m.write(e + 8, v);
        let work: u32 = m.read(e + 4);
        for k in 0..8u32 {
            let at = work.wrapping_add(0x400 + 0x34 * k);
            for off in [0xc, 0x1c, 0x20, 0x34, 0x38] {
                m.write(at.wrapping_add(off), 0u32);
            }
        }
        work
    }

    /// `dbgPrintf` and the format `texanim_malloc_memory_set` passes it.
    pub const DBG_PRINTF: u32 = 0x0062_f500;
    pub const MALLOC_MOVE_FORMAT: u32 = 0x00b5_4480;

    /// `texanim_malloc_memory_set(i, delta)` (`0x0077ea10`): prints the
    /// delta, then moves `g_ChrTexAnim[i]`'s work buffer (`+4`) by it.
    /// Returns `i * 0x30` (`eax`).
    pub fn texanim_malloc_memory_set(h: &mut impl Host, i: u32, delta: u32) -> u32 {
        use crate::texanim::bridge::CHR_TEXANIM;
        let fmt = h.rebase(MALLOC_MOVE_FORMAT);
        h.call_sym(DBG_PRINTF, Conv::Cdecl, &[fmt, delta]);
        let off = i.wrapping_mul(0x30);
        let at = CHR_TEXANIM.ptr(h).addr.wrapping_add(off) + 4;
        let v = h.read::<u32>(at).wrapping_add(delta);
        h.write(at, v);
        off
    }

    /// `Sg_SetMuteSound`.
    pub const SG_SET_MUTE_SOUND: u32 = 0x0082_1c60;
    /// The speed control's state (`SpdCtrl_Init`): three words, a byte
    /// flag, a word.
    pub const SPD_CTRL_B4: Global<u32> = Global::new(0x00ce_82b4);
    pub const SPD_CTRL_B8: Global<u32> = Global::new(0x00ce_82b8);
    pub const SPD_CTRL_ENABLED: Global<u8> = Global::new(0x00ce_82bc);
    pub const SPD_CTRL_C0: Global<u32> = Global::new(0x00ce_82c0);

    /// `SpdCtrl_Init()` (`0x006f72f0`): the speed control's state cleared,
    /// the sound unmuted (`Sg_SetMuteSound(0)`), then `+0xc0` cleared again
    /// and the enabled byte set. Returns the mute call's `eax`.
    pub fn spd_ctrl_init(h: &mut impl Host) -> u32 {
        SPD_CTRL_C0.set(h, 0);
        SPD_CTRL_B8.set(h, 0);
        SPD_CTRL_B4.set(h, 0);
        let r = h.call_sym(SG_SET_MUTE_SOUND, Conv::Cdecl, &[0]);
        SPD_CTRL_C0.set(h, 0);
        SPD_CTRL_ENABLED.set(h, 1);
        r
    }

    /// `inputIsPressButton`.
    pub const INPUT_IS_PRESS_BUTTON: u32 = 0x0063_0cf0;
    /// `CheckAutobtlKey`'s and `CheckInvincibleKey`'s flags and buttons.
    pub const AUTOBTL_KEY: Global<u32> = Global::new(0x00ce_82c8);
    pub const INVINCIBLE_KEY: Global<u32> = Global::new(0x00ce_82cc);
    pub const AUTOBTL_BUTTON: u32 = 0x2_0000;
    pub const INVINCIBLE_BUTTON: u32 = 0x20_0000;

    /// `CheckAutobtlKey()` (`0x006f7580`) and `CheckInvincibleKey()`
    /// (`0x006f76c0`): the flag is cleared, then set (and 1 returned)
    /// while `button` is pressed; else the flag (0).
    pub fn check_key(h: &mut impl Host, flag: Global<u32>, button: u32) -> u32 {
        flag.set(h, 0);
        if h.call_sym(INPUT_IS_PRESS_BUTTON, Conv::Cdecl, &[button]) != 0 {
            flag.set(h, 1);
            return 1;
        }
        flag.get(h)
    }

    /// `setLuluODKeyboardInput(left, right)` (`0x00645eb0`): with exactly
    /// one of buttons 0x8000 (to `*left`) and 0x2000 (to `*right`)
    /// pressed, that word becomes 1.
    pub fn set_lulu_od_keyboard_input(h: &mut impl Host, left: u32, right: u32) {
        let a = h.call_sym(INPUT_IS_PRESS_BUTTON, Conv::Cdecl, &[0x8000]) != 0;
        let b = h.call_sym(INPUT_IS_PRESS_BUTTON, Conv::Cdecl, &[0x2000]) != 0;
        match (a, b) {
            (true, false) => h.write(left, 1u32),
            (false, true) => h.write(right, 1u32),
            _ => {}
        }
    }

    /// `PhyreScene_GetSplitScreenStat`, `..SetSplitScreenStat`,
    /// `..SetSplitScreenType`.
    pub const SCENE_GET_SPLIT_SCREEN_STAT: u32 = 0x0065_17d0;
    pub const SCENE_SET_SPLIT_SCREEN_STAT: u32 = 0x0065_4550;
    pub const SCENE_SET_SPLIT_SCREEN_TYPE: u32 = 0x0065_4570;

    /// `SplitScreenStart(kind)` (`0x00638d40`): when the scene is not split
    /// yet, it becomes split (stat 1) of type 1 for `kind == 1`, else 0.
    /// Returns the last call's `eax`.
    pub fn split_screen_start(h: &mut impl Host, kind: u32) -> u32 {
        use crate::scene::bridge::SCENE;
        let scene = SCENE.get(h);
        let stat = h.call_sym(SCENE_GET_SPLIT_SCREEN_STAT, Conv::Thiscall(scene), &[]);
        if stat != 0 {
            return stat;
        }
        let scene = SCENE.get(h);
        h.call_sym(SCENE_SET_SPLIT_SCREEN_STAT, Conv::Thiscall(scene), &[1]);
        let scene = SCENE.get(h);
        h.call_sym(
            SCENE_SET_SPLIT_SCREEN_TYPE,
            Conv::Thiscall(scene),
            &[u32::from(kind == 1)],
        )
    }

    /// `PhyreScene_SetByte03`.
    pub const SCENE_SET_BYTE_03: u32 = 0x0066_d1b0;

    /// `graphicMagic0146SetKey(v)` (`0x00641e40`): the scene's byte `+3`
    /// (`PhyreScene_SetByte03`) and `g_GraphicLevelFilter` get `v`'s low
    /// byte. Returns the call's `eax`.
    pub fn magic_0146_set_key(h: &mut impl Host, v: u32) -> u32 {
        use crate::scene::bridge::SCENE;
        let scene = SCENE.get(h);
        let r = h.call_sym(SCENE_SET_BYTE_03, Conv::Thiscall(scene), &[v]);
        #[allow(clippy::cast_possible_truncation)] // the byte store
        crate::vfx::bridge::LEVEL_FILTER.set(h, v as u8);
        r
    }

    /// `g_LoadingScreen` and its `setParameterManually` and `Reset_006e4fd0`.
    pub const LOADING_SCREEN: Global<u32> = Global::new(0x00ce_517c);
    pub const LOADING_SCREEN_SET_PARAMETER: u32 = 0x006e_4ff0;
    pub const LOADING_SCREEN_RESET: u32 = 0x006e_4fd0;

    /// `graphicResetLoadingScreen()` (`0x00642c80`): by the loading
    /// screen's state (`+0x10`): 3 becomes 1 (`setParameterManually(null,
    /// &1, null)`), 0 resets it; others stay.
    pub fn reset_loading_screen(h: &mut impl Host) {
        let ls = LOADING_SCREEN.get(h);
        match h.read::<u32>(ls + 0x10) {
            3 => {
                let one = h.scratch();
                h.write(one, 1u32);
                h.call_sym(
                    LOADING_SCREEN_SET_PARAMETER,
                    Conv::Thiscall(ls),
                    &[0, one, 0],
                );
            }
            0 => {
                h.call_sym(LOADING_SCREEN_RESET, Conv::Thiscall(ls), &[]);
            }
            _ => {}
        }
    }

    /// `PostProcessManager_renderScreenFade` and the byte it gets last.
    pub const POST_PROCESS_RENDER_SCREEN_FADE: u32 = 0x0069_7490;
    pub const SCREEN_FADE_BYTE: Global<u8> = Global::new(0x00cc_caa4);

    /// `ScreenFadeDelegations_processRenderDelegation(d)` (`0x006ae5b0`):
    /// for a delegation `d` (words `+0..+8`, floats `+0xc..+0x18`, a word
    /// `+0x1c`), unless the post-process's colour is locked (byte
    /// `+0x123a0`): `+0x1c` to its `+0x1239c` and the four floats (through
    /// the x87) to `+0x1238c..+0x12398`; then
    /// `renderScreenFade(d[0], d[1], d[2], g_ScreenFadeByte)`.
    pub fn screen_fade_process(h: &mut impl Host, d: u32) {
        use crate::managers::bridge::POST_PROCESS;
        if d == 0 {
            return;
        }
        let mut floats = [0u32; 4];
        for (k, f) in (0u32..).zip(floats.iter_mut()) {
            *f = rt(h.read(d + 0xc + 4 * k));
        }
        let (a, b, c, last): (u32, u32, u32, u32) =
            (h.read(d), h.read(d + 4), h.read(d + 8), h.read(d + 0x1c));
        let pp = POST_PROCESS.get(h);
        if h.read::<u8>(pp + 0x123a0) == 0 {
            h.write(pp + 0x1239c, last);
        }
        let pp = POST_PROCESS.get(h);
        if h.read::<u8>(pp + 0x123a0) == 0 {
            for (k, f) in (0u32..).zip(floats) {
                h.write(pp + 0x1238c + 4 * k, f);
            }
        }
        let byte = u32::from(SCREEN_FADE_BYTE.get(h));
        let pp = POST_PROCESS.get(h);
        h.call_sym(
            POST_PROCESS_RENDER_SCREEN_FADE,
            Conv::Thiscall(pp),
            &[a, b, c, byte],
        );
    }
}
