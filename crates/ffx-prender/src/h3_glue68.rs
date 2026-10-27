//! The render-side glue, sixty-eighth batch (M4 port round 19, list
//! `h3-glue-render`): the scene's constructor. The exception frame and
//! cookie are left out, as in [`crate::glue10`].

/// `eh_vector_constructor_iterator(array, size, count, ctor, dtor)`
/// (stdcall), `memset` (cdecl), the `CreateMutexW` import slot (stdcall,
/// 3 words), `g_PhyreScene3D` (a byte), `Phyre_PhyreMalloc_0042fb80(size,
/// align)` and `Phyre_PhyreFree(p)` (cdecl),
/// `Phyre_PFramework_PApplication_GetApplication()` (cdecl).
pub const VECTOR_CTOR: u32 = 0x0094_966a;
pub const MEMSET: u32 = 0x0094_964c;
pub const CREATE_MUTEX_IAT: u32 = 0x00b0_c198;
pub const SCENE_3D_FLAG: u32 = 0x00cc_b9c0;
pub const PHYRE_MALLOC_ALIGNED: u32 = 0x0042_fb80;
pub const PHYRE_FREE: u32 = 0x0042_fb20;
pub const GET_APPLICATION: u32 = 0x0062_3b90;

pub mod bridge {
    //! The original over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_graphic::host::{Conv, Host};

    /// A stdcall through the import slot `slot`.
    fn import_stdcall(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Stdcall, args)
    }

    /// `PhyreScene_PhyreScene(scene)` (thiscall, plain `ret`; frame from
    /// `ebp - 0x58`; Ghidra: fastcall) -> `scene`. Field by field as the
    /// original: the members' constructors (maps, worlds, cameras, the
    /// orthographic cameras' vtables, clusters, free lists of map pairs,
    /// the texture video, post passes), identity matrices built in the
    /// frame (`ebp - 0x58`) for `toWorldMatrix3x4`, a mutex, the free list
    /// reset; three 0x400-word buffers ([`buffer`]), the application's
    /// screen size (`+0x26c`, `+0x270`) and the record block `+0xd68`
    /// cleared.
    #[allow(clippy::too_many_lines)] // one store per member
    pub fn phyre_scene(h: &mut impl Host, ebp: u32, this: u32) -> u32 {
        h.write(ebp - 0x14, this);
        h.write(this, 0_u32);
        h.call_sym(
            0x0064_cec0,
            Conv::Thiscall(this + 0x4),
            &[ebp - 0xd, ebp - 0xd],
        );
        h.write(this + 0x24, 0_u16);
        h.write(this + 0x28, 0_u32);
        h.write(this + 0x2c, 0_u32);
        h.write(this + 0x30, 0_u32);
        h.write(this + 0x34, 0_u32);
        h.write(this + 0x38, 0_u32);
        h.write(this + 0x3c, 0_u32);
        h.write(this + 0x40, 0_u32);
        h.write(this + 0x44, 0_u32);
        h.write(this + 0x48, 0_u32);
        h.write(this + 0x4c, 0_u32);
        h.write(this + 0x50, 0_u32);
        h.write(this + 0x54, 0_u32);
        h.write(this + 0x58, 0_u32);
        h.write(this + 0x5c, 0_u32);
        h.write(this + 0x60, 0_u32);
        h.write(this + 0x64, 0_u32);
        h.write(this + 0x68, 0_u32);
        h.write(this + 0x6c, 0_u32);
        h.write(this + 0x74, 0_u32);
        h.write(this + 0x78, 0_u8);
        h.call_sym(0x0044_29e0, Conv::Thiscall(this + 0x7c), &[]);
        h.call_sym(0x0044_29e0, Conv::Thiscall(this + 0x8c), &[]);
        h.call_sym(0x0044_29e0, Conv::Thiscall(this + 0x9c), &[]);
        h.call_sym(0x0044_29e0, Conv::Thiscall(this + 0xac), &[]);
        h.call_sym(0x0044_29e0, Conv::Thiscall(this + 0xbc), &[]);
        h.write(this + 0xdc, 0_u32);
        h.call_sym(0x0045_6370, Conv::Thiscall(this + 0xe0), &[]);
        h.write(this + 0x238, 0_u32);
        h.call_sym(0x0045_6370, Conv::Thiscall(this + 0x23c), &[]);
        h.call_sym(0x0045_6370, Conv::Thiscall(this + 0x398), &[]);
        h.write(this + 0x4f0, 1f32);
        h.write(this + 0x4f4, 0_u8);
        h.write(this + 0x4f8, 0_u32);
        h.call_sym(0x0045_30a0, Conv::Thiscall(this + 0x500), &[]);
        h.write(this + 0x64c, 1f32);
        h.write(this + 0x650, 1f32);
        h.write(this + 0x4fc, h.rebase(0x00b1_1040));
        h.call_sym(0x0045_5fe0, Conv::Thiscall(this + 0x4fc), &[]);
        h.write(this + 0x654, 0_u32);
        h.call_sym(0x0045_6370, Conv::Thiscall(this + 0x658), &[]);
        h.write(ebp - 0x58, 1f32);
        h.write(ebp - 0x54, 0f32);
        h.write(ebp - 0x50, 0f32);
        h.write(ebp - 0x4c, 0f32);
        h.write(ebp - 0x48, 0f32);
        h.write(ebp - 0x40, 0f32);
        h.write(ebp - 0x3c, 0f32);
        h.write(ebp - 0x38, 0f32);
        h.write(ebp - 0x34, 0f32);
        h.write(ebp - 0x2c, 0f32);
        h.write(ebp - 0x28, 0f32);
        h.write(ebp - 0x24, 0f32);
        h.write(ebp - 0x20, 0f32);
        h.write(ebp - 0x44, 1f32);
        h.write(ebp - 0x30, 1f32);
        h.write(ebp - 0x1c, 1f32);
        h.call_sym(0x0045_1190, Conv::Thiscall(this + 0x7b0), &[ebp - 0x58]);
        h.write(this + 0x7e0, 0_u32);
        h.call_sym(0x0045_30a0, Conv::Thiscall(this + 0x7e8), &[]);
        h.write(this + 0x934, 1f32);
        h.write(this + 0x938, 1f32);
        h.write(this + 0x7e4, h.rebase(0x00b1_1040));
        h.call_sym(0x0045_5fe0, Conv::Thiscall(this + 0x7e4), &[]);
        h.write(this + 0x93c, 0_u16);
        h.write(this + 0x93e, 0_u8);
        h.call_sym(0x0058_3b20, Conv::Thiscall(this + 0x940), &[]);
        h.call_sym(
            VECTOR_CTOR,
            Conv::Stdcall,
            &[
                this + 0x964,
                0x24,
                0x8,
                h.rebase(0x004b_e860),
                h.rebase(0x004b_e960),
            ],
        );
        h.call_sym(0x0045_30a0, Conv::Thiscall(this + 0xa88), &[]);
        h.write(this + 0xbd4, 1f32);
        h.write(this + 0xbd8, 1f32);
        h.write(this + 0xa84, h.rebase(0x00b1_1040));
        h.call_sym(0x0045_5fe0, Conv::Thiscall(this + 0xa84), &[]);
        h.call_sym(0x0045_6370, Conv::Thiscall(this + 0xbdc), &[]);
        h.write(ebp - 0x58, 1f32);
        h.write(ebp - 0x54, 0f32);
        h.write(ebp - 0x50, 0f32);
        h.write(ebp - 0x4c, 0f32);
        h.write(ebp - 0x48, 0f32);
        h.write(ebp - 0x40, 0f32);
        h.write(ebp - 0x3c, 0f32);
        h.write(ebp - 0x38, 0f32);
        h.write(ebp - 0x34, 0f32);
        h.write(ebp - 0x2c, 0f32);
        h.write(ebp - 0x28, 0f32);
        h.write(ebp - 0x24, 0f32);
        h.write(ebp - 0x20, 0f32);
        h.write(ebp - 0x44, 1f32);
        h.write(ebp - 0x30, 1f32);
        h.write(ebp - 0x1c, 1f32);
        h.call_sym(0x0045_1190, Conv::Thiscall(this + 0xd34), &[ebp - 0x58]);
        h.write(this + 0xd64, 0x1_u8);
        h.write(this + 0xdc8, 0_u32);
        h.write(this + 0xdcc, 0_u32);
        h.write(this + 0xdd0, 0_u16);
        h.write(this + 0xdd8, 0_u32);
        h.write(this + 0xddc, 0_u32);
        h.write(this + 0xde0, 0_u32);
        h.call_sym(0x00a2_6e70, Conv::Thiscall(this + 0xde8), &[]);
        h.write(this + 0x10c0, 0_u32);
        h.write(this + 0x10c4, u32::MAX);
        h.write(this + 0x10c8, 0_u16);
        h.write(this + 0x10d0, 0_u16);
        h.write(this + 0x10d4, 0_u32);
        h.write(this + 0x10d8, 0_u32);
        h.write(this + 0x10dc, 0_u32);
        h.write(this + 0x10e0, 0_u32);
        h.call_sym(0x0069_04c0, Conv::Thiscall(this + 0x10e4), &[]);
        h.write(this + 0x1128, 0_u32);
        h.write(this + 0x112c, 0_u32);
        h.write(this + 0x1130, 0_u32);
        h.write(this + 0x1134, 0_u32);
        h.write(this + 0x1138, 0_u32);
        h.write(this + 0x113c, 0_u32);
        h.write(this + 0x1140, 0_u32);
        h.call_sym(0x0069_04c0, Conv::Thiscall(this + 0x1144), &[]);
        h.write(this + 0x1188, 0_u32);
        h.write(this + 0x118c, 0_u32);
        h.write(this + 0x1190, 0_u32);
        h.write(this + 0x1194, 0_u32);
        h.write(this + 0x1198, this + 0x1198);
        h.write(this + 0x119c, this + 0x1198);
        h.write(this + 0x11a0, this + 0x1198);
        h.write(this + 0x11ac, this + 0x1198);
        h.write(ebp - 0x18, this + 0x1198);
        h.write(this + 0x11a8, this + 0x1198);
        h.write(this + 0x11a4, this + 0x1198);
        h.call_sym(
            0x0049_c480,
            Conv::Thiscall(this + 0x11b0),
            &[0x14, 0x8, 0x4, h.rebase(0x00b1_3bc8)],
        );
        h.write(this + 0x11c8, 0_u32);
        h.write(this + 0x11cc, 0_u32);
        h.write(this + 0x11d0, 0_u32);
        h.write(this + 0x11d8, 0_u32);
        h.write(this + 0x11dc, 0_u32);
        h.write(this + 0x11e0, 0_u32);
        let ret = h.call_sym(0x0065_4ed0, Conv::Thiscall(this + 0x11dc), &[]);
        h.write(this + 0x11dc, ret);
        h.call_sym(0x0057_6630, Conv::Thiscall(this + 0x11e4), &[]);
        h.write(this + 0x122c, this + 0x122c);
        h.write(this + 0x1230, this + 0x122c);
        h.call_sym(0x0057_6630, Conv::Thiscall(this + 0x1234), &[]);
        h.write(this + 0x127c, this + 0x127c);
        h.write(this + 0x1280, this + 0x127c);
        h.call_sym(0x0057_6630, Conv::Thiscall(this + 0x1284), &[]);
        h.write(this + 0x12cc, this + 0x12cc);
        h.write(this + 0x12d0, this + 0x12cc);
        h.call_sym(0x0057_6630, Conv::Thiscall(this + 0x12d4), &[]);
        h.write(this + 0x131c, this + 0x131c);
        h.write(this + 0x1320, this + 0x131c);
        h.call_sym(0x0057_6630, Conv::Thiscall(this + 0x1324), &[]);
        h.write(this + 0x136c, this + 0x136c);
        h.write(this + 0x1370, this + 0x136c);
        h.call_sym(
            VECTOR_CTOR,
            Conv::Stdcall,
            &[
                this + 0x1374,
                0x50,
                0x40,
                h.rebase(0x0043_f800),
                h.rebase(0x0043_fa30),
            ],
        );
        h.call_sym(0x0057_6630, Conv::Thiscall(this + 0x2874), &[]);
        h.write(this + 0x28bc, this + 0x28bc);
        h.write(this + 0x28c0, this + 0x28bc);
        h.call_sym(0x0057_6630, Conv::Thiscall(this + 0x28c4), &[]);
        h.write(this + 0x290c, this + 0x290c);
        h.write(this + 0x2910, this + 0x290c);
        h.call_sym(
            VECTOR_CTOR,
            Conv::Stdcall,
            &[
                this + 0x2914,
                0x50,
                0x10,
                h.rebase(0x0043_f800),
                h.rebase(0x0043_fa30),
            ],
        );
        h.call_sym(
            VECTOR_CTOR,
            Conv::Stdcall,
            &[
                this + 0x2e14,
                0x50,
                0x10,
                h.rebase(0x0043_f800),
                h.rebase(0x0043_fa30),
            ],
        );
        h.call_sym(
            VECTOR_CTOR,
            Conv::Stdcall,
            &[
                this + 0x3314,
                0x50,
                0x10,
                h.rebase(0x0043_f800),
                h.rebase(0x0043_fa30),
            ],
        );
        h.write(this + 0x3854, 0_u32);
        h.write(this + 0x3858, 0_u32);
        h.write(this + 0x385c, 0_u32);
        h.write(this + 0x3860, 0_u32);
        h.write(this + 0x3864, 0_u32);
        h.write(this + 0x3868, 0_u32);
        h.write(this + 0x386c, 0_u32);
        h.write(this + 0x3870, 0_u32);
        h.write(this + 0x3874, 0_u32);
        h.write(this + 0x3878, 0_u32);
        h.write(this + 0x387c, 0_u32);
        h.write(this + 0x3880, 0_u32);
        h.write(this + 0x3884, 0_u32);
        h.write(this + 0x3888, 0_u32);
        h.write(this + 0x3890, 0_u32);
        h.write(this + 0x3894, 0_u32);
        h.write(this + 0x3898, 0_u32);
        h.write(this + 0x389c, 0_u32);
        h.write(this + 0x38a4, 0_u32);
        h.write(this + 0x38a8, 0_u32);
        h.write(this + 0x38ac, 0_u32);
        h.write(this + 0x38b0, 0_u32);
        h.write(this + 0x38b4, 0_u32);
        h.write(this + 0x38b8, 0_u32);
        h.write(this + 0x38bc, 0_u32);
        h.write(this + 0x38c0, 0_u32);
        h.write(this + 0x38c4, 0_u32);
        h.write(this + 0x38c8, 0_u32);
        h.write(this + 0xf8cc, 0_u32);
        h.write(this + 0xf8d0, 0_u32);
        h.write(this + 0xf954, 0_u32);
        h.write(this + 0xf958, 0_u8);
        h.call_sym(0x0057_6630, Conv::Thiscall(this + 0xf96c), &[]);
        h.write(this + 0xf9b4, this + 0xf9b4);
        h.write(this + 0xf9b8, this + 0xf9b4);
        h.write(this + 0xf9bc, 0f32);
        h.write(this + 0xf9c0, 0f32);
        h.write(this + 0xf9c4, 0x101_u32);
        h.write(this + 0xf9c8, 0_u8);
        h.write(this + 0xf9d4, this + 0xf9d4);
        h.write(this + 0xf9d8, this + 0xf9d4);
        h.write(this + 0xf9dc, this + 0xf9d4);
        h.write(this + 0xf9e8, this + 0xf9d4);
        h.write(ebp - 0x18, this + 0xf9d4);
        h.write(this + 0xf9e4, this + 0xf9d4);
        h.write(this + 0xf9e0, this + 0xf9d4);
        h.call_sym(
            0x0049_c480,
            Conv::Thiscall(this + 0xf9ec),
            &[0x1c, 0x8, 0x4, h.rebase(0x00b1_3bc8)],
        );
        h.write(ebp - 0x58, 1f32);
        h.write(ebp - 0x54, 0f32);
        h.write(ebp - 0x50, 0f32);
        h.write(ebp - 0x4c, 0f32);
        h.write(ebp - 0x48, 0f32);
        h.write(ebp - 0x40, 0f32);
        h.write(ebp - 0x3c, 0f32);
        h.write(ebp - 0x38, 0f32);
        h.write(ebp - 0x34, 0f32);
        h.write(ebp - 0x2c, 0f32);
        h.write(ebp - 0x28, 0f32);
        h.write(ebp - 0x24, 0f32);
        h.write(ebp - 0x20, 0f32);
        h.write(ebp - 0x44, 1f32);
        h.write(ebp - 0x30, 1f32);
        h.write(ebp - 0x1c, 1f32);
        h.call_sym(0x0045_1190, Conv::Thiscall(this + 0xfa04), &[ebp - 0x58]);
        h.write(this + 0xfa34, 0f32);
        h.write(this + 0xfa38, 0f32);
        h.write(ebp - 0x58, 1f32);
        h.write(ebp - 0x44, 1f32);
        h.write(ebp - 0x30, 1f32);
        h.write(ebp - 0x1c, 1f32);
        h.write(ebp - 0x54, 0f32);
        h.write(ebp - 0x50, 0f32);
        h.write(ebp - 0x4c, 0f32);
        h.write(ebp - 0x48, 0f32);
        h.write(ebp - 0x40, 0f32);
        h.write(ebp - 0x3c, 0f32);
        h.write(ebp - 0x38, 0f32);
        h.write(ebp - 0x34, 0f32);
        h.write(ebp - 0x2c, 0f32);
        h.write(ebp - 0x28, 0f32);
        h.write(ebp - 0x24, 0f32);
        h.write(ebp - 0x20, 0f32);
        h.call_sym(0x0045_1190, Conv::Thiscall(this + 0xfa3c), &[ebp - 0x58]);
        h.write(this + 0xfa6c, 0_u32);
        h.write(this + 0xfa94, 0_u32);
        h.write(this + 0xfa98, 0_u32);
        h.write(this + 0xfa9c, 0_u16);
        h.write(this + 0xfa9f, 0_u32);
        h.call_sym(0x0058_3b20, Conv::Thiscall(this + 0xfaa8), &[]);
        h.call_sym(0x0058_3b20, Conv::Thiscall(this + 0xfacc), &[]);
        h.write(this + 0xfaf0, 0_u32);
        h.write(this + 0xfafc, 0_u32);
        h.write(this + 0xfb00, 0_u32);
        h.write(this + 0xfb04, 0_u8);
        h.write(this + 0xfb08, 0_u32);
        h.write(this + 0xfb0c, 0_u32);
        h.write(this + 0xfb10, 0_u32);
        h.write(this + 0xfb14, 0_u32);
        h.write(this + 0xfb18, 0_u32);
        h.write(this + 0xfb1c, 0_u32);
        h.write(this + 0xfb20, 0_u32);
        h.write(this + 0xfb24, 0_u32);
        h.write(this + 0xfb28, 0_u32);
        h.write(this + 0xfb2c, 0_u32);
        h.write(this + 0xfb30, 0_u32);
        h.write(this + 0xfb34, 0_u32);
        h.write(this + 0xfb38, 0_u32);
        let ret = import_stdcall(h, CREATE_MUTEX_IAT, &[0x0, 0x0, 0x0]);
        h.write(this + 0xfb3c, ret);
        h.write(this + 0xfb44, 0x1_u16);
        h.write(this + 0xfb48, 0_u32);
        h.write(this + 0xfb4c, 0_u32);
        h.write(this + 0xfb50, 0_u32);
        h.write(this + 0xfb54, 0_u32);
        h.write(this + 0xfb58, 0_u32);
        h.write(this + 0xfb5c, 0_u32);
        h.write(this + 0xfb80, 0_u32);
        h.write(this + 0xfb84, 0_u32);
        h.write(this + 0xfb88, 0_u32);
        h.write(this + 0xfb94, 0_u32);
        h.write(this + 0xfb98, 0_u32);
        h.write(this + 0xfb9c, 0_u32);
        h.write(this + 0xfba8, 0_u32);
        h.write(this + 0xfbac, 0_u32);
        h.write(this + 0xfbb0, 0_u32);
        h.write(this + 0xfbbc, 0_u32);
        h.write(this + 0xfbc0, 0_u32);
        h.write(this + 0xfbc4, 0_u32);
        h.write(this + 0xfbd8, 0_u32);
        h.write(this + 0xfbdc, 0_u32);
        h.write(this + 0xfbe0, 0_u32);
        h.write(this + 0xfbf0, 0_u32);
        h.write(this + 0xfbf4, 0_u32);
        h.write(this + 0xfbf8, 0_u32);
        h.write(this + 0xfc00, 0_u16);
        h.write(this + 0xfc70, 0_u32);
        h.write(this + 0xfc74, 0_u32);
        h.write(this + 0xfc78, 1f32);
        h.write(this + 0xfc7c, 1f32);
        h.write(this + 0xfc80, 1f32);
        h.write(ebp - 0x18, this + 0x10d54);
        h.write(this + 0xfc84, 1f32);
        h.write(this + 0xfc88, 0_u8);
        h.write(this + 0xfc8c, 1f32);
        h.write(this + 0xfc90, 1f32);
        h.write(this + 0xfc94, 1f32);
        h.write(this + 0xfc98, 1f32);
        h.write(this + 0xfc9c, 0f32);
        h.write(this + 0xfca0, 0f32);
        h.write(this + 0xfca4, 0f32);
        h.write(this + 0xfca8, 0f32);
        h.write(this + 0xfcac, 1f32);
        h.write(this + 0xfcb0, 1f32);
        h.write(this + 0xfcb4, 1f32);
        h.write(this + 0xfcb8, 1f32);
        h.write(this + 0xfcbc, 0f32);
        h.write(this + 0xfcc0, 0f32);
        h.write(this + 0xfcc4, 0f32);
        h.write(this + 0xfcc8, 0f32);
        h.write(this + 0x10d2c, 0_u16);
        h.write(this + 0x10d2e, 0_u8);
        h.write(this + 0x10d30, 0f32);
        h.write(this + 0x10d34, 0f32);
        h.write(this + 0x10d38, 0f32);
        h.write(this + 0x10d50, 0_u8);
        h.write(this + 0x10d54, this + 0x10d54);
        h.write(this + 0x10d58, this + 0x10d54);
        h.write(this + 0x10d5c, this + 0x10d54);
        h.write(this + 0x10d68, this + 0x10d54);
        h.write(this + 0x10d64, this + 0x10d54);
        h.write(this + 0x10d60, this + 0x10d54);
        h.call_sym(
            0x0049_c480,
            Conv::Thiscall(this + 0x10d6c),
            &[0x14, 0x8, 0x4, h.rebase(0x00b1_3bc8)],
        );
        h.write(this + 0x10d84, 0_u32);
        h.write(this + 0x10d88, 0_u32);
        h.write(ebp - 0x58, 1f32);
        h.write(ebp - 0x54, 0f32);
        h.write(ebp - 0x50, 0f32);
        h.write(ebp - 0x4c, 0f32);
        h.write(this + 0x10d8c, 0_u32);
        h.write(ebp - 0x48, 0f32);
        h.write(this + 0x10d90, 0_u32);
        h.write(ebp - 0x40, 0f32);
        h.write(ebp - 0x3c, 0f32);
        h.write(ebp - 0x38, 0f32);
        h.write(ebp - 0x34, 0f32);
        h.write(ebp - 0x2c, 0f32);
        h.write(ebp - 0x28, 0f32);
        h.write(ebp - 0x24, 0f32);
        h.write(ebp - 0x20, 0f32);
        h.write(ebp - 0x44, 1f32);
        h.write(ebp - 0x30, 1f32);
        h.write(ebp - 0x1c, 1f32);
        h.call_sym(0x0045_1190, Conv::Thiscall(this + 0x10d94), &[ebp - 0x58]);
        h.write(this + 0x10dc4, 0_u32);
        h.write(this + 0x10dd0, 0_u32);
        h.write(this + 0x10dd4, 0_u32);
        h.write(this + 0x10dd8, 0x2_u32);
        h.write(this + 0x10de0, 0_u32);
        h.write(h.rebase(SCENE_3D_FLAG), 0_u8);
        h.write(this + 0x10ddc, 0_u32);
        h.call_sym(0x0057_4970, Conv::Thiscall(this + 0x1198), &[]);
        h.call_sym(0x0049_cc10, Conv::Thiscall(this + 0x11b0), &[]);
        h.call_sym(MEMSET, Conv::Cdecl, &[this + 0xfccc, 0x0, 0x1060]);
        h.call_sym(MEMSET, Conv::Cdecl, &[this + 0x3814, 0x0, 0x40]);
        h.call_sym(MEMSET, Conv::Cdecl, &[this + 0x2774, 0x0, 0x100]);
        buffer(h, this, 0x3884);
        h.write(this + 0x388c, 0_u32);
        buffer(h, this, 0x3890);
        let p: u32 = h.read(this + 0x3894);
        h.call_sym(MEMSET, Conv::Cdecl, &[p, 0, 0x1000]);
        buffer(h, this, 0x3898);
        let p: u32 = h.read(this + 0x389c);
        h.call_sym(MEMSET, Conv::Cdecl, &[p, 0, 0x1000]);
        h.write(this + 0x38a0, 0_u32);
        h.write(this + 0x10d40, 0_u32);
        h.write(this + 0x10d44, 0_u32);
        let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        let width: u32 = h.read(app.wrapping_add(0x26c));
        h.write(this + 0x10d48, width);
        let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        let height: u32 = h.read(app.wrapping_add(0x270));
        h.write(this + 0x10d4c, height);
        h.call_sym(MEMSET, Conv::Cdecl, &[this + 0xd68, 0, 0x60]);
        this
    }

    /// The buffer whose capacity is at `+cap` and pointer at `+cap + 4`,
    /// sized 0x400 words unless it is: a new 0x1000 bytes (align 4),
    /// cleared; the old one freed unless it is the same or the
    /// capacity's top bit (not owned) is set. Nothing when the allocation
    /// fails.
    fn buffer(h: &mut impl Host, this: u32, cap: u32) {
        if h.read::<u32>(this + cap) == 0x400 {
            return;
        }
        let p = h.call_sym(PHYRE_MALLOC_ALIGNED, Conv::Cdecl, &[0x1000, 4]);
        if p == 0 {
            return;
        }
        for k in 0..0x400 {
            h.write(p + 4 * k, 0_u32);
        }
        let old: u32 = h.read(this + cap + 4);
        if old != p && h.read::<u32>(this + cap) >> 31 == 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
        }
        h.write(this + cap + 4, p);
        h.write(this + cap, 0x400_u32);
    }
}
