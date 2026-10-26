//! Round 3 of list `h4-glue-other` (M4 port round 1): the async cluster
//! loads of `TextUtil_init` and the font call of
//! `TOMkpCrossEasyStrFontSClut` (both wider than [`Host::call`]: the
//! bridge hands them to a `wide` cdecl caller, the hook's
//! `call_cdecl_words`), more create-once singletons, array helpers, a
//! tree erase, dlmalloc's `create_mspace_with_base`, the escape menu's
//! cancel key, menu textures, an `opus` size sum and `rcbgRestoreReg`.
//!
//! Bridges that hand callees the addresses of their own locals take
//! `frame` as in [`crate::frames`]: a block laid out as the original's
//! frame from its lowest local up.

use ffx_bridge::Global;
use ffx_graphic::host::{Conv, Host};

use crate::menu::CALL_AS3;
use crate::misc::OPERATOR_NEW;
use crate::sound::{USER_FREE, USER_MALLOC};

/// `AsyncLoadManager_sGetInstance()` (cdecl).
pub const ASYNC_LOAD_GET: u32 = 0x0069_1b40;
/// `AsyncLoadManager_loadCluster(mgr, a, name, callback, kind, lo, hi,
/// i, b)` (cdecl, nine words: wider than [`Host::call`]).
pub const ASYNC_LOAD_CLUSTER: u32 = 0x0069_1660;
/// The two cluster names `TextUtil_init` loads (`0x00c38e10`).
pub const TEXT_CLUSTERS: u32 = 0x00c3_8e10;
/// The load callback `TextUtil_init` passes (`0x006f63f0`).
pub const TEXT_CLUSTER_LOADED: u32 = 0x006f_63f0;

/// `TextUtil_init(n)` (`0x006f6370`; `n` in `ecx`, nothing on the stack):
/// both text clusters requested from the async loader, `n` passed as a
/// sign-extended 64-bit value, the index, zeros around them. Returns 0.
///
/// `wide(h, f, words)` calls the cdecl function at `f` (rebased) with
/// these nine words.
pub fn text_util_init<H: Host>(
    h: &mut H,
    n: u32,
    wide: &mut impl FnMut(&mut H, u32, &[u32]) -> u32,
) -> u32 {
    let hi = if n.cast_signed() < 0 { u32::MAX } else { 0 };
    let table = h.rebase(TEXT_CLUSTERS);
    let callback = h.rebase(TEXT_CLUSTER_LOADED);
    let load = h.rebase(ASYNC_LOAD_CLUSTER);
    for i in 0..2u32 {
        // the name is pushed before the manager is fetched
        let name = h.read::<u32>(table + 4 * i);
        let mgr = h.call_sym(ASYNC_LOAD_GET, Conv::Cdecl, &[]);
        wide(h, load, &[mgr, 0, name, callback, 2, n, hi, i, 0]);
    }
    0
}

/// `TOAdpCrossEasyStrFontSClutRGBA` (`0x008ec2f0`, cdecl, ten words).
pub const CROSS_EASY_RGBA: u32 = 0x008e_c2f0;

/// `TOMkpCrossEasyStrFontSClut(s, x, y, w, sx, sy)` (`0x00901660`,
/// cdecl): `TOAdpCrossEasyStrFontSClutRGBA(s, x, y, w, 0x80, 0x80, 0x80,
/// 0x80, sx, sy)`; the floats go through `fld`/`fstp` (a signalling NaN
/// comes out quiet). Returns the callee's `eax`.
pub fn mkp_cross_easy<H: Host>(
    h: &mut H,
    args: [u32; 6],
    wide: &mut impl FnMut(&mut H, u32, &[u32]) -> u32,
) -> u32 {
    let f = |bits: u32| ffx_bridge::x87::round_trip(f32::from_bits(bits)).to_bits();
    let [s, x, y, w, sx, sy] = args;
    let target = h.rebase(CROSS_EASY_RGBA);
    wide(
        h,
        target,
        &[s, f(x), f(y), w, 0x80, 0x80, 0x80, 0x80, f(sx), f(sy)],
    )
}

/// `AsyncLoadManager_createInstance_00691a80()`: global, size,
/// constructor.
pub const ASYNC_LOAD_MANAGER: (Global<u32>, u32, u32) =
    (Global::new(0x00cc_c88c), 0x4c, 0x0069_0830);
/// `PostProcessManager_createInstance_00692a90()`.
pub const POST_PROCESS_MANAGER: (Global<u32>, u32, u32) =
    (Global::new(0x00cc_c8a0), 0x1_240c, 0x0069_2020);
/// `PostProcessTweakUtil_createInstance_00692b10()`.
pub const POST_PROCESS_TWEAK: (Global<u32>, u32, u32) =
    (Global::new(0x00cc_c89c), 0x190, 0x0069_2410);

/// `g_TextureAnimationManager` (`0x00cccaa8`).
pub const TEXTURE_ANIMATION_MANAGER: Global<u32> = Global::new(0x00cc_caa8);
/// `TextureAnimationManager`'s constructor (`0x006af370`, thiscall).
pub const TEXTURE_ANIMATION_CTOR: u32 = 0x006a_f370;
/// Its setup `0x006b3f40` (thiscall, no stack words).
pub const TEXTURE_ANIMATION_SETUP: u32 = 0x006b_3f40;

/// `TextureAnimationManager_createInstance_006b00a0()`: unless the
/// global is set, the constructor on `operator_new(0x84)` (0 when that
/// fails) into it, then `0x006b3f40` on that (0 included). The SEH frame
/// is not kept. Callers read no result.
pub fn texture_animation_create(h: &mut impl Host) {
    if TEXTURE_ANIMATION_MANAGER.get(h) != 0 {
        return;
    }
    let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x84]);
    let v = if p == 0 {
        0
    } else {
        h.call_sym(TEXTURE_ANIMATION_CTOR, Conv::Thiscall(p), &[])
    };
    TEXTURE_ANIMATION_MANAGER.set(h, v);
    h.call_sym(TEXTURE_ANIMATION_SETUP, Conv::Thiscall(v), &[]);
}

/// `memcpy` (the import's thunk; cdecl).
pub const MEMCPY: u32 = crate::frames::MEMCPY;

/// `Array8_reserve_0070edb0(this; n)` (thiscall): an array of 8-byte
/// elements (`+0` count, `+4` capacity, `+8` storage), as
/// [`crate::sound::sfx_id_array_reserve`] with `8n` bytes: without
/// storage `user_malloc(8n)`; with it a new block for a positive `n` with
/// the count's elements copied, the old one freed. Returns the
/// allocation's `eax` on the first path, `user_free`'s on the second.
pub fn array8_reserve(h: &mut impl Host, this: u32, n: u32) -> u32 {
    if h.read::<u32>(this + 8) == 0 {
        let p = h.call_sym(USER_MALLOC, Conv::Cdecl, &[n.wrapping_mul(8)]);
        h.write(this + 4, n);
        h.write(this + 8, p);
        return p;
    }
    let mut p = 0;
    if n.cast_signed() > 0 {
        p = h.call_sym(USER_MALLOC, Conv::Cdecl, &[n.wrapping_mul(8)]);
        if p != 0 {
            let bytes = h.read::<u32>(this).wrapping_shl(3);
            let old = h.read::<u32>(this + 8);
            h.call_sym(MEMCPY, Conv::Cdecl, &[p, old, bytes]);
        }
    }
    let old = h.read::<u32>(this + 8);
    let r = h.call_sym(USER_FREE, Conv::Cdecl, &[old]);
    h.write(this + 4, n);
    h.write(this + 8, p);
    r
}

/// The aligned allocator `0x0042fb80(size, align)` (cdecl).
pub const ALIGNED_ALLOC: u32 = 0x0042_fb80;
/// `0x006aeb20(dst, src, n)` (cdecl): copy-constructs `n` 0x40-byte
/// elements.
pub const COPY_ELEMENTS40: u32 = 0x006a_eb20;
/// `PArray_destroy_006b0230(p, n)` (cdecl).
pub const DESTROY_ELEMENTS40: u32 = 0x006b_0230;

/// `PArray40_assign_006afa70(this; src)` (thiscall, `args` the address
/// of its argument word): `src`'s count (`+0`, top bit masked) of 0x40-byte
/// elements (`+4`) copied into a new aligned block (none for 0); the old
/// storage destroyed unless it is the new one or the count's top bit
/// (not owned) is set; then the block and the count. The original keeps
/// `src`'s storage pointer in its argument slot; so does the port.
/// Returns `this`.
pub fn parray40_assign(h: &mut impl Host, this: u32, args: u32) -> u32 {
    let src: u32 = h.read(args);
    let n = h.read::<u32>(src) & 0x7fff_ffff;
    let data: u32 = h.read(src + 4);
    h.write(args, data);
    let mut p = 0;
    if n != 0 {
        p = h.call_sym(ALIGNED_ALLOC, Conv::Cdecl, &[n.wrapping_shl(6), 4]);
        if p != 0 {
            let data: u32 = h.read(args);
            h.call_sym(COPY_ELEMENTS40, Conv::Cdecl, &[p, data, n]);
        }
    }
    let old: u32 = h.read(this + 4);
    if old != p {
        let word: u32 = h.read(this);
        if word >> 31 == 0 {
            h.call_sym(DESTROY_ELEMENTS40, Conv::Cdecl, &[old, word & 0x7fff_ffff]);
        }
    }
    h.write(this + 4, p);
    h.write(this, n);
    this
}

/// `0x006a1720(this; &out, node)` (thiscall): the tree's erase, its
/// iterator written to `out`.
pub const TREE_ERASE: u32 = 0x006a_1720;

/// `Tree_eraseValue_006a1d10(this; value)` (thiscall, `args` the address
/// of its argument word): an in-order walk of the `std::map` at `*this`
/// (node: `+0` left, `+4` parent, `+8` right, `+0xd` nil, `+0x28` the
/// mapped word) to the first node whose `+0x28` is `value`, which
/// `0x006a1720(this; args, node)` erases (the iterator lands in the
/// argument slot); returns that call's `eax`, or the head when no node
/// matches.
pub fn tree_erase_value(h: &mut impl Host, this: u32, args: u32) -> u32 {
    fn nil(h: &impl Host, node: u32) -> bool {
        h.read::<u8>(node + 0xd) != 0
    }
    let head: u32 = h.read(this);
    let value: u32 = h.read(args);
    let mut node: u32 = h.read(head);
    while node != head {
        if h.read::<u32>(node + 0x28) == value {
            return h.call_sym(TREE_ERASE, Conv::Thiscall(this), &[args, node]);
        }
        if nil(h, node) {
            continue;
        }
        let right: u32 = h.read(node + 8);
        if nil(h, right) {
            // up while coming from the right
            let mut child = node;
            let mut parent: u32 = h.read(node + 4);
            while !nil(h, parent) && child == h.read::<u32>(parent + 8) {
                child = parent;
                parent = h.read(parent + 4);
            }
            node = parent;
        } else {
            // the leftmost node of the right subtree
            node = right;
            loop {
                let left: u32 = h.read(node);
                if nil(h, left) {
                    break;
                }
                node = left;
            }
        }
    }
    head
}

/// dlmalloc's `mparams.magic` (`0x00ce6ec4`).
pub const MPARAMS_MAGIC: u32 = 0x00ce_6ec4;
/// dlmalloc's `mparams.page_size` (`0x00ce6ec8`).
pub const MPARAMS_PAGE_SIZE: u32 = 0x00ce_6ec8;
/// dlmalloc's `init_mparams()` (`0x006ea050`, cdecl).
pub const INIT_MPARAMS: u32 = 0x006e_a050;
/// dlmalloc's `init_user_mstate(base, size)` (`0x006ea180`, cdecl).
pub const INIT_USER_MSTATE: u32 = 0x006e_a180;

/// `create_mspace_with_base(base, capacity, locked)` (`0x006ebd30`,
/// cdecl, dlmalloc): `init_mparams` once; a capacity above 0x210 and
/// below `-(0x210 + page_size)` gets `init_user_mstate(base, capacity)`
/// with `seg.sflags` (`+0x1d4`) = 8 (extern) and the lock bit (bit 1 of
/// `mflags`, `+0x1b8`) from `locked`; returns that state, else 0.
pub fn create_mspace_with_base(h: &mut impl Host, base: u32, capacity: u32, locked: u32) -> u32 {
    if h.read::<u32>(h.rebase(MPARAMS_MAGIC)) == 0 {
        h.call_sym(INIT_MPARAMS, Conv::Cdecl, &[]);
    }
    if capacity <= 0x210 {
        return 0;
    }
    let page: u32 = h.read(h.rebase(MPARAMS_PAGE_SIZE));
    if capacity >= 0xffff_fdf0u32.wrapping_sub(page) {
        return 0;
    }
    let m = h.call_sym(INIT_USER_MSTATE, Conv::Cdecl, &[base, capacity]);
    let flags: u32 = h.read(m + 0x1b8);
    h.write(m + 0x1d4, 8u32);
    let flags = if locked != 0 { flags | 2 } else { flags & !2 };
    h.write(m + 0x1b8, flags);
    m
}

/// `0x00630df0(out, a, b, c)` (cdecl): fills the 12-byte `out`, returns it.
pub const MAKE_EVENT: u32 = 0x0063_0df0;
/// The `ActionScript` function the escape menu calls on cancel
/// (`"onCancelKey"`, `0x00b46c7c`).
pub const ON_CANCEL_KEY: u32 = 0x00b4_6c7c;

/// `FFEscMenu_onCancelInput_007018a0(this; input, b)` (thiscall, ret 8;
/// `frame` the 12-byte local at `ebp - 0xc`): with the menu's cancel flag
/// (`+0xfc`) clear, 1; else for an input kind (`*input`) of 1, 3, 5 or 8
/// the flag is cleared, `0x00630df0(frame, 0, 0, 0)` and
/// `IggyMenu_callIggyAS3External(this; 0, "onCancelKey", 0, 0, 1)` run
/// and the result is 1; any other kind gives `kind == 7`. Returns `al`.
pub fn esc_on_cancel_input(h: &mut impl Host, frame: u32, this: u32, input: u32) -> u8 {
    if h.read::<u8>(this + 0xfc) == 0 {
        return 1;
    }
    let kind: u32 = h.read(input);
    if !matches!(kind, 1 | 3 | 5 | 8) {
        return u8::from(kind == 7);
    }
    h.write(this + 0xfc, 0u8);
    h.call_sym(MAKE_EVENT, Conv::Cdecl, &[frame, 0, 0, 0]);
    let name = h.rebase(ON_CANCEL_KEY);
    h.call_sym(CALL_AS3, Conv::Thiscall(this), &[0, name, 0, 0, 1]);
    1
}

/// `g_PhyreScene` (`0x00ccb9d8`).
pub const PHYRE_SCENE: Global<u32> = Global::new(0x00cc_b9d8);
/// `PhyreScene_loadMenuTexture(scene; id)` (thiscall).
pub const LOAD_MENU_TEXTURE: u32 = 0x0066_51d0;
/// The main menu's wave textures (`0x00c59c40`..`0x00c59d58`, 70 ids).
pub const WAVE_TEXTURES: (u32, u32) = (0x00c5_9c40, 0x00c5_9d58);

/// `MenuTextureManager_LoadMainMenuWaveTexture()` (`0x0067cba0`): each id
/// of [`WAVE_TEXTURES`] loaded through `g_PhyreScene` (read again for
/// each). Returns the last call's `eax`.
pub fn load_main_menu_wave_texture(h: &mut impl Host) -> u32 {
    let (start, end) = WAVE_TEXTURES;
    let mut r = 0;
    for at in (start..end).step_by(4) {
        let id: u32 = h.read(h.rebase(at));
        let scene = PHYRE_SCENE.get(h);
        r = h.call_sym(LOAD_MENU_TEXTURE, Conv::Thiscall(scene), &[id]);
    }
    r
}

/// `0x006393a0(n)` (cdecl): a header size for `n` entries.
pub const ACC_HEADER_SIZE: u32 = 0x0063_93a0;
/// `0x007e3810(base, kind, i)` (cdecl): entry `i` of kind `kind`.
pub const OPUS_ENTRY: u32 = 0x007e_3810;
/// `0x006393c0(v)` (cdecl): an entry's size.
pub const ACC_ENTRY_SIZE: u32 = 0x0063_93c0;

/// `opus_0706_acc_draw(rec, base, out)` (`0x007f7360`, cdecl): `*out` =
/// the header size for `rec`'s count (`+6`, u16) plus, for each of those
/// entries from `rec`'s first index (`+4`, u16), the size of the kind-4
/// entry's signed `+0x12` (the count reread each time); then
/// `user_malloc(*out)`, whose `eax` it returns.
pub fn opus_0706_acc_draw(h: &mut impl Host, rec: u32, base: u32, out: u32) -> u32 {
    h.write(out, 0u32);
    let n = u32::from(h.read::<u16>(rec + 6));
    let size = h.call_sym(ACC_HEADER_SIZE, Conv::Cdecl, &[n]);
    let total = h.read::<u32>(out).wrapping_add(size);
    h.write(out, total);
    let mut index = u32::from(h.read::<u16>(rec + 4));
    if h.read::<u16>(rec + 6) != 0 {
        let mut i = 0u32;
        loop {
            let entry = h.call_sym(OPUS_ENTRY, Conv::Cdecl, &[base, 4, index]);
            let v = i32::from(h.read::<i16>(entry + 0x12)) as u32;
            let size = h.call_sym(ACC_ENTRY_SIZE, Conv::Cdecl, &[v]);
            let total = h.read::<u32>(out).wrapping_add(size);
            h.write(out, total);
            i += 1;
            index = index.wrapping_add(1);
            if i.cast_signed() >= i32::from(h.read::<u16>(rec + 6)) {
                break;
            }
        }
    }
    let total: u32 = h.read(out);
    h.call_sym(USER_MALLOC, Conv::Cdecl, &[total])
}

/// `rcPacketDataInit(p)` (cdecl).
pub const RC_PACKET_INIT: u32 = 0x0092_7a20;
/// `0x00927a70(p, buffer)` (cdecl).
pub const RC_PACKET_SET_BUFFER: u32 = 0x0092_7a70;
/// `0x009202a0(v)` (cdecl): returns `v`.
pub const RC_IDENTITY: u32 = 0x0092_02a0;
/// `0x009275b0(p)` (cdecl).
pub const RC_PACKET_STEP: u32 = 0x0092_75b0;
/// `0x00927600(p)` (cdecl).
pub const RC_PACKET_SEND: u32 = 0x0092_7600;
/// The packet buffer `Sg_InitAll` allocates (`0x00c498dc`).
pub const RC_PACKET_BUFFER: Global<u32> = Global::new(0x00c4_98dc);

/// `rcbgRestoreReg()` (`0x00920250`; `frame` the 8-byte packet at
/// `ebp - 8`): `rcPacketDataInit(frame)`, `0x00927a70(frame,
/// *0x00c498dc)`, `+4` passed through `0x009202a0`, then `0x009275b0` and
/// `0x00927600` on it. Returns the last call's `eax`.
pub fn rcbg_restore_reg(h: &mut impl Host, frame: u32) -> u32 {
    h.call_sym(RC_PACKET_INIT, Conv::Cdecl, &[frame]);
    let buffer = RC_PACKET_BUFFER.get(h);
    h.call_sym(RC_PACKET_SET_BUFFER, Conv::Cdecl, &[frame, buffer]);
    let v: u32 = h.read(frame + 4);
    let v = h.call_sym(RC_IDENTITY, Conv::Cdecl, &[v]);
    h.write(frame + 4, v);
    h.call_sym(RC_PACKET_STEP, Conv::Cdecl, &[frame]);
    h.call_sym(RC_PACKET_SEND, Conv::Cdecl, &[frame])
}

/// `g_GraphicUi` (`0x00ccc81c`).
pub const GRAPHIC_UI: Global<u32> = Global::new(0x00cc_c81c);
/// `0x0067e4f0(ui; id)` (thiscall).
pub const UI_RELEASE_TEXTURE: u32 = 0x0067_e4f0;
/// `0x00679820(id, buf, size)` (cdecl): the texture's name.
pub const TEXTURE_NAME: u32 = 0x0067_9820;
/// `MenuDynamicGeometry_setTextureInMaterialUnloaded(geometry; name)`.
pub const SET_TEXTURE_UNLOADED: u32 = 0x006a_3740;
/// `0x00671120(scene; name)` (thiscall).
pub const SCENE_UNLOAD_TEXTURE: u32 = 0x0067_1120;
/// The bytes of `MenuTextureManager_MenuUnloadTexture`'s frame: a
/// 0x100-byte name and the cookie.
pub const UNLOAD_TEXTURE_FRAME: u32 = 0x104;

/// `MenuTextureManager_MenuUnloadTexture(id)` (`0x0067dbf0`, stdcall;
/// `frame` the name buffer at `ebp - 0x104`): for a non-zero id,
/// `0x0067e4f0(g_GraphicUi; id)`, the texture's name into the buffer,
/// then the name handed to the UI's dynamic geometry
/// (`**(g_GraphicUi + 0x2bd4)`) and to `g_PhyreScene`'s
/// `0x00671120`. Returns the last call's `eax` (for id 0 the original's
/// is junk; the port's 0).
pub fn menu_unload_texture(h: &mut impl Host, frame: u32, id: u32) -> u32 {
    if id == 0 {
        return 0;
    }
    let ui = GRAPHIC_UI.get(h);
    h.call_sym(UI_RELEASE_TEXTURE, Conv::Thiscall(ui), &[id]);
    h.call_sym(TEXTURE_NAME, Conv::Cdecl, &[id, frame, 0x100]);
    let ui = GRAPHIC_UI.get(h);
    let holder: u32 = h.read(ui + 0x2bd4);
    let geometry: u32 = h.read(holder);
    h.call_sym(SET_TEXTURE_UNLOADED, Conv::Thiscall(geometry), &[frame]);
    let scene = PHYRE_SCENE.get(h);
    h.call_sym(SCENE_UNLOAD_TEXTURE, Conv::Thiscall(scene), &[frame])
}

/// `BlurEffect`'s create-once singleton (`0x006c34d0`; the global
/// `BlurEffect_GetInstance` reads): global, size, constructor.
pub const BLUR_EFFECT_NEW: (Global<u32>, u32, u32) = (Global::new(0x00cd_eba0), 0x88, 0x006c_32d0);
/// `MotionBlur`'s (`0x006c5030`; `MotionBlur_GetInstance`'s global).
pub const MOTION_BLUR_NEW: (Global<u32>, u32, u32) = (Global::new(0x00cd_ec04), 0x8c, 0x006c_4ec0);

/// `g_GraphicAbmap` (`0x00ccc838`).
pub const GRAPHIC_ABMAP: Global<u32> = Global::new(0x00cc_c838);
/// The sphere grid's constructor `0x00680970(this; arg)` (thiscall).
pub const ABMAP_CTOR: u32 = 0x0068_0970;

/// `GraphicAbmap_createInstance_00684a60(arg)` (cdecl): unless
/// `g_GraphicAbmap` is set, `0x00680970(operator_new(0xe8); arg)` into it
/// (0 when the allocation fails). The SEH frame is not kept. Callers read
/// no result.
pub fn abmap_create(h: &mut impl Host, arg: u32) {
    if GRAPHIC_ABMAP.get(h) != 0 {
        return;
    }
    let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0xe8]);
    let v = if p == 0 {
        0
    } else {
        h.call_sym(ABMAP_CTOR, Conv::Thiscall(p), &[arg])
    };
    GRAPHIC_ABMAP.set(h, v);
}

/// The post effect base constructor `0x00576630(this)` (thiscall).
pub const POST_EFFECT_BASE_CTOR: u32 = 0x0057_6630;
/// `0x00583b20(this)` (thiscall): the member at `+0x68`'s constructor.
pub const MEMBER68_CTOR: u32 = 0x0058_3b20;
/// The float `MotionBlur`'s constructor stores at `+0x5c` (`0x00b495e4`).
pub const MOTION_BLUR_5C: u32 = 0x00b4_95e4;

/// `MotionBlur_ctor_006c4ec0(this)` (thiscall): the base constructor,
/// an empty list head at `+0x48` (both links to itself), `+0x5c` from
/// [`MOTION_BLUR_5C`] (through `fld`/`fstp`), `+0x54` 0, the half-word
/// `+0x58` 0x100, `+0x60` and `+0x64` 0, then `+0x68`'s constructor.
/// The SEH frame is not kept. Returns `this`.
pub fn motion_blur_ctor(h: &mut impl Host, this: u32) -> u32 {
    h.call_sym(POST_EFFECT_BASE_CTOR, Conv::Thiscall(this), &[]);
    let head = this + 0x48;
    h.write(head, head);
    h.write(head + 4, head);
    let v: u32 = h.read(h.rebase(MOTION_BLUR_5C));
    let v = ffx_bridge::x87::round_trip(f32::from_bits(v)).to_bits();
    h.write(this + 0x5c, v);
    h.write(this + 0x54, 0u32);
    h.write(this + 0x58, 0x100u16);
    h.write(this + 0x60, 0u32);
    h.write(this + 0x64, 0u32);
    h.call_sym(MEMBER68_CTOR, Conv::Thiscall(this + 0x68), &[]);
    this
}

/// `InitVFXRenderDataLine`'s callees (cdecl): `0x00714b60(a, b, 0x9c4,
/// d)`, `0x006453d0(a, b)` (the render data), `0x00645780(a, b)`,
/// `0x0071a0c0(a, b, c)`, `0x006458a0(a, b)`, `0x00645280(a, b)`.
pub const VFX_LINE_CALLS: [u32; 6] = [
    0x0071_4b60,
    0x0064_53d0,
    0x0064_5780,
    0x0071_a0c0,
    0x0064_58a0,
    0x0064_5280,
];

/// `InitVFXRenderDataLine(a, b, c, d)` (`0x007112d0`, cdecl): unless
/// `0x00714b60(a, b, 0x9c4, d)` fails, the render data's `+0xb8` set to
/// 1 (when there is one), `0x00645780(a, b)`, the render data again, and
/// when that `0x00645780` result was non-zero `0x0071a0c0(a, b, c)` and
/// `0x006458a0(a, b)`; any failure ends in `0x00645280(a, b)`. Returns
/// the last call's `eax`.
pub fn init_vfx_render_data_line(h: &mut impl Host, a: u32, b: u32, c: u32, d: u32) -> u32 {
    let [create, data, begin, fill, end, fail] = VFX_LINE_CALLS;
    if h.call_sym(create, Conv::Cdecl, &[a, b, 0x9c4, d]) != 0 {
        let p = h.call_sym(data, Conv::Cdecl, &[a, b]);
        if p != 0 {
            h.write(p + 0xb8, 1u32);
        }
        let ok = h.call_sym(begin, Conv::Cdecl, &[a, b]);
        h.call_sym(data, Conv::Cdecl, &[a, b]);
        if ok != 0 {
            h.call_sym(fill, Conv::Cdecl, &[a, b, c]);
            return h.call_sym(end, Conv::Cdecl, &[a, b]);
        }
    }
    h.call_sym(fail, Conv::Cdecl, &[a, b])
}

/// The node pool's refill `0x0049c8d0(pool; n)` (thiscall).
pub const POOL_REFILL: u32 = 0x0049_c8d0;
/// The node pool's give-back `0x0049c980(pool; node)` (thiscall).
pub const POOL_GIVE: u32 = 0x0049_c980;
/// The tree's rebalance after an insert `0x00574980(this; node)`
/// (thiscall).
pub const TREE_FIXUP: u32 = 0x0057_4980;

/// The tagged-tree inserts `TaggedTree_add_006a2830` and
/// `vfxIntTree_add_0070cd60` (thiscall(this; &key, &value), ret 8): a
/// node from the pool at `+0x18` (refilled with `+0x20` nodes when empty;
/// still empty gives 0xe), its links to itself, key and value at
/// `+0xc`/`+0x10`; `insert(this; node)` (`insert`, the one difference)
/// refusing it gives the node back and 0x12, else the rebalance and 0.
pub fn tree_add(h: &mut impl Host, insert: u32, this: u32, key: u32, value: u32) -> u32 {
    let pool = this + 0x18;
    let mut node: u32 = h.read(pool);
    if node == 0 {
        let n: u32 = h.read(this + 0x20);
        h.call_sym(POOL_REFILL, Conv::Thiscall(pool), &[n]);
        node = h.read(pool);
        if node == 0 {
            return 0xe;
        }
    }
    let next: u32 = h.read(node);
    h.write(pool, next);
    h.write(node, node);
    h.write(node + 4, node);
    h.write(node + 8, node);
    let k: u32 = h.read(key);
    h.write(node + 0xc, k);
    let v: u32 = h.read(value);
    h.write(node + 0x10, v);
    if h.call_sym(insert, Conv::Thiscall(this), &[node]) != 0 {
        h.call_sym(POOL_GIVE, Conv::Thiscall(pool), &[node]);
        return 0x12;
    }
    h.call_sym(TREE_FIXUP, Conv::Thiscall(this), &[node]);
    0
}

/// `TaggedTreeInsert_0069dae0(head; node)`: [`tree_add`]'s insert at
/// `0x006a2830`.
pub const TAGGED_TREE_INSERT: u32 = 0x0069_dae0;
/// `vfxIntTree_Insert_0070bb20(tree; node)`: at `0x0070cd60`.
pub const VFX_INT_TREE_INSERT: u32 = 0x0070_bb20;

/// The motion blur shader cluster's path (`0x00b4958c`).
pub const MOTION_BLUR_SHADER: u32 = 0x00b4_958c;
/// The motion blur's cluster-loaded callback (`0x006c53e0`).
pub const MOTION_BLUR_LOADED: u32 = 0x006c_53e0;
/// The bytes of `MotionBlur_initialize`'s frame: the 0x100-byte path and
/// the cookie.
pub const MOTION_BLUR_FRAME: u32 = 0x104;

/// `MotionBlur_initialize(this; a, b, c)` (`0x006c5360`, thiscall, ret
/// 0xc; `frame` the path at `ebp - 0x104`): the shader's data path into
/// the buffer (`0x00679820`), then `AsyncLoadManager_loadCluster(
/// sGetInstance(), 0, path, 0x006c53e0, 3, (i64)this, (i64)b, (i64)c)`
/// (eleven words, through `wide`); `a` is unused. Returns 0.
pub fn motion_blur_initialize<H: Host>(
    h: &mut H,
    frame: u32,
    this: u32,
    [b, c]: [u32; 2],
    wide: &mut impl FnMut(&mut H, u32, &[u32]) -> u32,
) -> u32 {
    let sign = |v: u32| if v.cast_signed() < 0 { u32::MAX } else { 0 };
    let path = h.rebase(MOTION_BLUR_SHADER);
    h.call_sym(TEXTURE_NAME, Conv::Cdecl, &[path, frame, 0x100]);
    let callback = h.rebase(MOTION_BLUR_LOADED);
    let mgr = h.call_sym(ASYNC_LOAD_GET, Conv::Cdecl, &[]);
    let load = h.rebase(ASYNC_LOAD_CLUSTER);
    let words = [
        mgr,
        0,
        frame,
        callback,
        3,
        this,
        sign(this),
        b,
        sign(b),
        c,
        sign(c),
    ];
    wide(h, load, &words);
    0
}

/// `iggyDataValueSet_00a28db0(value, b)` (cdecl): a boolean Iggy value
/// (writes only its `+0` and `+8`).
pub const DATA_VALUE_SET_BOOL: u32 = crate::menu::DATA_VALUE_SET_BOOL;
/// `IggyMenu_callFlashFunction_1(menu, name, value)` (cdecl, the 16-byte
/// value by value).
pub const CALL_FLASH_1: u32 = 0x0068_dd40;
/// `"setEnableChangePara"` (`0x00b4c4d8`) and `"setEnableSpecialFeature"`
/// (`0x00b4c4ec`).
pub const ENABLE_FUNCTIONS: [u32; 2] = [0x00b4_c4d8, 0x00b4_c4ec];

/// `FFEscMenu_setEnableParaAndSpecial_006fca50(this; on)` (thiscall,
/// ret 4; `frame` the 16-byte Iggy value at `ebp - 0x10`): for each of
/// [`ENABLE_FUNCTIONS`], the value set to the boolean `on` and passed by
/// value to `IggyMenu_callFlashFunction_1(this, name, value)`. Returns
/// that call's `eax`.
pub fn esc_set_enable_para_and_special(h: &mut impl Host, frame: u32, this: u32, on: u32) -> u32 {
    let mut r = 0;
    for name in ENABLE_FUNCTIONS {
        h.call_sym(DATA_VALUE_SET_BOOL, Conv::Cdecl, &[frame, on]);
        let kind: u32 = h.read(frame);
        let flag: u32 = h.read(frame + 8);
        let name = h.rebase(name);
        // junk read: [ebp-0xc] and [ebp-0x4] never written; 0 (brief 2026-10-24)
        r = h.call_sym(CALL_FLASH_1, Conv::Cdecl, &[this, name, kind, 0, flag, 0]);
    }
    r
}

/// The `GDraw` state (`0x01a858b4`).
pub const GDRAW_STATE: u32 = crate::frames::GDRAW_STATE;
/// `"GDraw dynamic vertex buffer usage of %d bytes in one call larger
/// than buffer size %d"` (`0x00b7a530`).
pub const GDRAW_TOO_LARGE: u32 = 0x00b7_a530;
/// `gdraw_mapResource_00a3c070(ctx, resource, discard)` (cdecl).
pub const GDRAW_MAP_RESOURCE: u32 = 0x00a3_c070;

/// `gdraw_lockDynamicBuffer_00a3cf40(buffer, n)` (cdecl): a request
/// above the buffer's size (`+4`) warns and gives 0. Else the state's
/// peak (`+0x9b8`) rises to `n`, the write offset (`+8`) is the cursor
/// (`+0xc`) when `n` bytes still fit after it, else 0 (and then the map
/// discards), the resource (`+0`) is mapped on the state's context
/// (`+4`); on success the cursor moves past the `n` bytes and the
/// mapping plus the offset is returned (0 on failure).
pub fn gdraw_lock_dynamic(h: &mut impl Host, buffer: u32, n: u32) -> u32 {
    let size: u32 = h.read(buffer + 4);
    if n > size {
        let warn: u32 = h.read(h.rebase(crate::frames::GDRAW_SEND_WARNING));
        let text = h.rebase(GDRAW_TOO_LARGE);
        h.call(warn, Conv::Cdecl, &[0, text, n, size]);
        return 0;
    }
    let state: u32 = h.read(h.rebase(GDRAW_STATE));
    let peak: u32 = h.read(state + 0x9b8);
    h.write(state + 0x9b8, peak.max(n));
    let cursor: u32 = h.read(buffer + 0xc);
    let free = h.read::<u32>(buffer + 4).wrapping_sub(cursor);
    let offset = if free >= n { cursor } else { 0 };
    h.write(buffer + 8, offset);
    let resource: u32 = h.read(buffer);
    let ctx: u32 = h.read(state + 4);
    let discard = u32::from(offset == 0);
    let p = h.call_sym(GDRAW_MAP_RESOURCE, Conv::Cdecl, &[ctx, resource, discard]);
    if p == 0 {
        return 0;
    }
    let offset: u32 = h.read(buffer + 8);
    h.write(buffer + 0xc, offset.wrapping_add(n));
    p.wrapping_add(offset)
}

/// `RemapButtonIcon_sGetInstance()` (cdecl).
pub const REMAP_GET: u32 = 0x006e_7710;
/// `RemapButtonIcon_getPcButtonUv(icon; id, &u, &v)` (thiscall).
pub const REMAP_PC_UV: u32 = 0x006e_6670;
/// `RemapButtonIcon_getJoypadUV(icon; id, &u, &v)` (thiscall).
pub const REMAP_PAD_UV: u32 = 0x006e_6460;
/// `0x006e81f0(this; quad, 8, 0xc, u, v)` (thiscall): sets a quad's UVs.
pub const SET_QUAD_UV: u32 = 0x006e_81f0;

/// `ButtonIcon_setUvs_006e8020(this; id)` (thiscall, ret 4; `frame` the
/// two floats at `ebp - 8`): the PC button's UV (`getPcButtonUv(id,
/// frame, frame + 4)`) on the quad at `+0x2c`, the pad's
/// (`getJoypadUV(id, frame + 4, frame)`) on the one at `+0x48`, each
/// through `0x006e81f0(this; quad, 8, 0xc, first, second)` (the floats
/// through `fld`/`fstp`). Returns that call's `eax`.
pub fn button_icon_set_uvs(h: &mut impl Host, frame: u32, this: u32, id: u32) -> u32 {
    let f = |bits: u32| ffx_bridge::x87::round_trip(f32::from_bits(bits)).to_bits();
    let icon = h.call_sym(REMAP_GET, Conv::Cdecl, &[]);
    h.call_sym(REMAP_PC_UV, Conv::Thiscall(icon), &[id, frame, frame + 4]);
    let (u, v): (u32, u32) = (h.read(frame), h.read(frame + 4));
    let quad = [this + 0x2c, 8, 0xc, f(u), f(v)];
    h.call_sym(SET_QUAD_UV, Conv::Thiscall(this), &quad);
    let icon = h.call_sym(REMAP_GET, Conv::Cdecl, &[]);
    h.call_sym(REMAP_PAD_UV, Conv::Thiscall(icon), &[id, frame + 4, frame]);
    let (u, v): (u32, u32) = (h.read(frame + 4), h.read(frame));
    let quad = [this + 0x48, 8, 0xc, f(u), f(v)];
    h.call_sym(SET_QUAD_UV, Conv::Thiscall(this), &quad)
}

/// `DJB_hash(s)` (cdecl).
pub const DJB_HASH: u32 = 0x0067_b7d0;

/// `ClusterManager_getPClusterByName(this; name)` (`0x0069b4b0`,
/// thiscall, ret 4; `args` the address of its argument word): under the
/// manager's mutex (`**(this + 0x18)`), the cluster map at `+0x10` (key
/// `+0x10`, value `+0x14`) searched for the name's `DJB_hash` (lower
/// bound, then an exact match); the iterator is left in the argument slot.
/// Returns the entry's first word when its count (`+4`, signed) is at
/// least 1, else 0.
pub fn get_pcluster_by_name(h: &mut impl Host, this: u32, args: u32) -> u32 {
    let name: u32 = h.read(args);
    let hash = h.call_sym(DJB_HASH, Conv::Cdecl, &[name]);
    let holder: u32 = h.read(this + 0x18);
    let mutex: u32 = h.read(holder);
    let wait = crate::menu::WAIT_FOR_SINGLE_OBJECT.get(h);
    h.call(wait, Conv::Stdcall, &[mutex, u32::MAX]);
    let head: u32 = h.read(this + 0x10);
    let mut bound = head;
    let mut node: u32 = h.read(head + 4);
    while h.read::<u8>(node + 0xd) == 0 {
        if h.read::<u32>(node + 0x10) < hash {
            node = h.read(node + 8);
        } else {
            bound = node;
            node = h.read(node);
        }
    }
    let it = if bound == head || hash < h.read::<u32>(bound + 0x10) {
        head
    } else {
        bound
    };
    h.write(args, it);
    let it: u32 = h.read(args);
    let mut r = 0;
    if it != head {
        let entry: u32 = h.read(it + 0x14);
        if h.read::<u32>(entry + 4).cast_signed() >= 1 {
            r = h.read(entry);
        }
    }
    let holder: u32 = h.read(this + 0x18);
    let mutex: u32 = h.read(holder);
    let release = crate::menu::RELEASE_MUTEX.get(h);
    h.call(release, Conv::Stdcall, &[mutex]);
    r
}

/// The data directory prefix `FmodFile_open` adds to names
/// (`0x00c3a480`, a pointer-sized offset).
pub const FMOD_FILE_PREFIX: u32 = 0x00c3_a480;
/// `Phyre_PSerialization_PStreamFile_ctor(file; name, a, b, c, d)`.
pub const PSTREAM_FILE_CTOR: u32 = 0x0060_7bc0;
/// `Phyre_PSerialization_PStreamFileWin32_isOpen(file)` (`al`).
pub const PSTREAM_FILE_IS_OPEN: u32 = 0x0060_7f20;
/// `PhyreFIOSStreamFile_getFileSize(file)` (thiscall).
pub const PSTREAM_FILE_SIZE: u32 = 0x006f_9d50;
/// The stream file's destructor `0x00607c10(file)` (a bare `ret`).
pub const PSTREAM_FILE_DTOR: u32 = 0x0060_7c10;

/// `FmodFile_open(name, unicode, filesize, handle, userdata)`
/// (`0x00708510`, stdcall, FMOD's open callback): no name gives 0x17
/// (`FMOD_ERR_FILE_NOTFOUND`); else a stream file (`operator_new(8)`, 0
/// when that fails) on the prefixed name (`*0x00c3a480 + name`, mode 1, 0,
/// 0, 1); open, it goes to `*handle` and its size to `*filesize` (0
/// returned); not open, it is destroyed and freed (when there is one) and
/// 0x17 returned.
pub fn fmod_file_open(h: &mut impl Host, name: u32, filesize: u32, handle: u32) -> u32 {
    if name == 0 {
        return 0x17;
    }
    let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[8]);
    let file = if p == 0 {
        0
    } else {
        let prefix: u32 = h.read(h.rebase(FMOD_FILE_PREFIX));
        let path = prefix.wrapping_add(name);
        h.call_sym(PSTREAM_FILE_CTOR, Conv::Thiscall(p), &[path, 1, 0, 0, 1]);
        p
    };
    if h.call_sym(PSTREAM_FILE_IS_OPEN, Conv::Thiscall(file), &[]) as u8 != 0 {
        h.write(handle, file);
        let size = h.call_sym(PSTREAM_FILE_SIZE, Conv::Thiscall(file), &[]);
        h.write(filesize, size);
        return 0;
    }
    if file != 0 {
        h.call_sym(PSTREAM_FILE_DTOR, Conv::Thiscall(file), &[]);
        h.call_sym(crate::sound::OPERATOR_DELETE, Conv::Cdecl, &[file]);
    }
    0x17
}

/// The intrusive list's unlink `0x00575000(list; node)` (thiscall).
pub const LIST_UNLINK: u32 = 0x0057_5000;
/// The node pool's release `0x0049cc10(pool)` (thiscall).
pub const POOL_RELEASE: u32 = 0x0049_cc10;
/// The list's own teardown `0x007108b0(list)` (thiscall).
pub const LIST_TEARDOWN: u32 = 0x0071_08b0;

/// `SfxBank_destroyEvents_00710960(this)` (thiscall, `SfxBank`'s
/// destructor's helper): each node of the list at `+4` (its first at
/// `+4 + 0xc`, until the list itself or 0) unlinked and its data
/// (`+0x18`) freed with `user_free`; then the list's pool (`+0x1c`) and the
/// list torn down. The SEH frame is not kept. Returns the last call's
/// `eax`.
pub fn sfx_bank_destroy_events(h: &mut impl Host, this: u32) -> u32 {
    let list = this + 4;
    loop {
        let node: u32 = h.read(list + 0xc);
        if node == list || node == 0 {
            break;
        }
        h.call_sym(LIST_UNLINK, Conv::Thiscall(list), &[node]);
        let data: u32 = h.read(node + 0x18);
        if data != 0 {
            h.call_sym(USER_FREE, Conv::Cdecl, &[data]);
        }
    }
    h.call_sym(POOL_RELEASE, Conv::Thiscall(list + 0x18), &[]);
    h.call_sym(LIST_TEARDOWN, Conv::Thiscall(list), &[])
}

/// `TreeNode_newWithKey_0069c580(this; key)` (thiscall).
pub const TREE_NODE_NEW: u32 = 0x0069_c580;
/// `0x0069caf0(this; &out, head, key, node)` (thiscall): the map's
/// erase-by-key into `out`.
pub const MAP_ERASE_KEY: u32 = 0x0069_caf0;

/// The in-order successor of `node` in an MSVC tree (`+0` left, `+4`
/// parent, `+8` right, `+0xd` nil).
fn successor(h: &impl Host, node: u32) -> u32 {
    let nil = |n: u32| h.read::<u8>(n + 0xd) != 0;
    let right: u32 = h.read(node + 8);
    if nil(right) {
        let mut child = node;
        let mut parent: u32 = h.read(node + 4);
        while !nil(parent) && child == h.read::<u32>(parent + 8) {
            child = parent;
            parent = h.read(parent + 4);
        }
        parent
    } else {
        let mut n = right;
        loop {
            let left: u32 = h.read(n);
            if nil(left) {
                return n;
            }
            n = left;
        }
    }
}

/// `VfxRenderDataMap_eraseRange_0069d990(this; first, last)` (thiscall,
/// ret 8; `args` the address of its two argument words, `frame` the word
/// at `ebp - 4`): from `first` to `last`, each node's key (`+0x10`) gets a
/// new node (`0x0069c580`) whose key and itself go to
/// `0x0069caf0(this; frame, *this, node + 0x10, node)`; the iterator in
/// the `first` slot then steps to its successor (a nil node stays). Returns
/// the iterator (`last` at the end).
pub fn vfx_map_erase_range(h: &mut impl Host, frame: u32, this: u32, args: u32) -> u32 {
    // `push ecx` makes the word: `this` until the erase writes it
    h.write(frame, this);
    let mut it: u32 = h.read(args);
    while it != h.read::<u32>(args + 4) {
        let head: u32 = h.read(this);
        let node = h.call_sym(TREE_NODE_NEW, Conv::Thiscall(this), &[it + 0x10]);
        h.call_sym(
            MAP_ERASE_KEY,
            Conv::Thiscall(this),
            &[frame, head, node + 0x10, node],
        );
        it = h.read(args);
        if h.read::<u8>(it + 0xd) == 0 {
            it = successor(h, it);
            h.write(args, it);
        }
    }
    it
}

/// The aligned allocator's free `0x0042fb20(p)` (cdecl).
pub const ALIGNED_FREE: u32 = 0x0042_fb20;

/// `PArray_appendInto_006a25e0(this; dst, &value)` (thiscall, ret 8):
/// the array's (`+0` count, top bit "not owned"; `+4` storage) words
/// copied into `dst` (a destination word at address 0 is skipped and the
/// source does not advance), `*value` appended, the old storage freed
/// (`0x0042fb20`) unless it is `dst`, not owned or null; then `dst` and
/// the count plus one (the top bit dropped). Returns 0, or 0xd for a null
/// `dst`.
pub fn parray_append_into(h: &mut impl Host, this: u32, dst: u32, value: u32) -> u32 {
    if dst == 0 {
        return 0xd;
    }
    let n = h.read::<u32>(this) & 0x7fff_ffff;
    let mut src: u32 = h.read(this + 4);
    let mut at = dst;
    for _ in 0..n {
        let d = at;
        at = at.wrapping_add(4);
        if d != 0 {
            let v: u32 = h.read(src);
            h.write(d, v);
            src = src.wrapping_add(4);
        }
    }
    let end = dst.wrapping_add(n.wrapping_mul(4));
    if end != 0 {
        let v: u32 = h.read(value);
        h.write(end, v);
    }
    let old: u32 = h.read(this + 4);
    if old != dst && h.read::<u32>(this) >> 31 == 0 && old != 0 {
        h.call_sym(ALIGNED_FREE, Conv::Cdecl, &[old]);
    }
    h.write(this + 4, dst);
    h.write(this, n + 1);
    0
}

/// `MenuManager_TakeMultiRecord_0067c170(this; chain)` (thiscall, ret
/// 4): the busy byte `+0x5b74` cleared; chain 0 takes the next record of
/// the pool at `+0x15e8`/`+0x15ec` (used `+0x2bfc`) into the list at `+0`
/// (count `+0x2c50`), chain 1 the pool at `+0x4240`/`+0x4244` (used
/// `+0x583c`) into the list at `+0x2c58` (count `+0x5840`). Returns the
/// record, 0 when the pool is used up or the chain is another.
pub fn take_multi_record(h: &mut impl Host, this: u32, chain: u32) -> u32 {
    h.write(this + 0x5b74, 0u8);
    let (cap, pool, used, list, count) = match chain {
        0 => (0x15e8, 0x15ec, 0x2bfc, 0, 0x2c50),
        1 => (0x4240, 0x4244, 0x583c, 0x2c58, 0x5840),
        _ => return 0,
    };
    let taken: u32 = h.read(this + used);
    if taken >= h.read::<u32>(this + cap) & 0x7fff_ffff {
        return 0;
    }
    let records: u32 = h.read(this + pool);
    let record: u32 = h.read(records.wrapping_add(taken.wrapping_mul(4)));
    let k: u32 = h.read(this + count);
    h.write(
        this.wrapping_add(list).wrapping_add(k.wrapping_mul(4)),
        record,
    );
    let v: u32 = h.read(this + used);
    h.write(this + used, v.wrapping_add(1));
    let v: u32 = h.read(this + count);
    h.write(this + count, v.wrapping_add(1));
    record
}

/// `PhyreScene_setMeshInstanceWorldMatrix(scene; mesh, instance)`.
pub const SET_MESH_WORLD: u32 = 0x0066_d810;
/// `Phyre_PRendering_PMeshInstanceBounds_setMinAndMax(bounds; &min,
/// &max)`.
pub const SET_MIN_AND_MAX: u32 = 0x0050_0bd0;
/// The bytes of `MenuDynamicGeometry_setMeshWorldMatrix`'s frame.
pub const MESH_WORLD_FRAME: u32 = 0x28;

/// `MenuDynamicGeometry_setMeshWorldMatrix(this)` (`0x006a2e60`,
/// thiscall; `frame` its locals at `ebp - 0x28`): with an instance
/// (`+0x24`) and a mesh (`+0xc`), `g_PhyreScene`'s
/// `setMeshInstanceWorldMatrix(mesh, instance)`; then, when the mesh has
/// bounds (`+0x1c`), the instance's minimum (`+0x98..`) and maximum
/// (`+0xa4..`) copied as floats (`fld`/`fstp`) into the frame (through
/// the original's scratch words at `ebp - 8`/`ebp - 4`) and handed to
/// `setMinAndMax(bounds; &min, &max)`. Returns what the original leaves
/// in `eax`: 0, the instance or the mesh where it stops, else that call's.
pub fn set_mesh_world_matrix(h: &mut impl Host, frame: u32, this: u32) -> u32 {
    let instance: u32 = h.read(this + 0x24);
    if instance == 0 {
        return 0;
    }
    let mesh: u32 = h.read(this + 0xc);
    if mesh == 0 {
        return instance;
    }
    let scene = PHYRE_SCENE.get(h);
    h.call_sym(SET_MESH_WORLD, Conv::Thiscall(scene), &[mesh, instance]);
    let mesh: u32 = h.read(this + 0xc);
    let bounds: u32 = h.read(mesh + 0x1c);
    if bounds == 0 {
        return mesh;
    }
    let instance: u32 = h.read(this + 0x24);
    // each store is an `fld`/`fstp` pair, in the original's order: (frame
    // offset, source)
    let steps: [(u32, u32); 10] = [
        (0x24, instance + 0xa8),
        (0x20, instance + 0xac),
        (0x10, instance + 0xa4),
        (0x14, frame + 0x24),
        (0x18, frame + 0x20),
        (0x20, instance + 0x9c),
        (0x24, instance + 0xa0),
        (0, instance + 0x98),
        (4, frame + 0x20),
        (8, frame + 0x24),
    ];
    for (to, from) in steps {
        let bits: u32 = h.read(from);
        let v = ffx_bridge::x87::round_trip(f32::from_bits(bits)).to_bits();
        h.write(frame + to, v);
    }
    h.call_sym(
        SET_MIN_AND_MAX,
        Conv::Thiscall(bounds),
        &[frame, frame + 0x10],
    )
}
