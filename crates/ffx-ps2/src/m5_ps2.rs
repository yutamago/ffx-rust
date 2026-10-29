//! M5 stage 1, list `ps2-emu`: the PS2 layer's jump thunks to the `cdc`
//! matrix code and the allocator, a getter, the input arrays' class
//! descriptor singletons, and the Ogg stream's end-of-page check.
//!
//! The callees still in the game's code are called by address through
//! `ffx_graphic::host::Host`.

pub use ffx_graphic::host::{Conv, Host};

/// The original functions, over game memory.
pub mod bridge {
    use ffx_bridge::Global;

    use super::{Conv, Host};

    /// The thunks' targets (cdecl).
    pub const CDC_MUL4X4: u32 = 0x0070_59b0;
    pub const CDC_ORTHONORMAL_INVERSE: u32 = 0x0070_5db0;
    pub const CDC_TRANSPOSE: u32 = 0x0070_63f0;
    pub const FFX_VU0_MUL_MATRIX: u32 = 0x006e_d5c0;
    pub const ALIGNED_FREE_THUNK: u32 = 0x0068_6fe0;
    pub const USER_ALIGNED_MALLOC: u32 = 0x0068_6ff0;
    /// `memcpy` (cdecl thunk).
    pub const MEMCPY: u32 = 0x0094_9256;
    /// `atexit(f)` (cdecl).
    pub const ATEXIT: u32 = 0x0094_9207;
    /// `Phyre_PClassDescriptor_getDefaultInstance_0043cbb0` (`ecx`).
    pub const GET_DEFAULT_INSTANCE: u32 = 0x0043_cbb0;
    /// The Ogg reader's page step (thiscall `(state; arg)` ret 4,
    /// `0x0042d470`).
    pub const OGG_PAGE_STEP: u32 = 0x0042_d470;
    /// The word `code_009db170` returns.
    pub const WORD_012FBC40: Global<u32> = Global::new(0x012f_bc40);

    /// A class descriptor singleton: its guard word and bit, the object,
    /// its constructor, its concrete vtable, its `atexit` destructor.
    #[derive(Debug, Clone, Copy)]
    pub struct Descriptor {
        pub guard: u32,
        pub object: u32,
        pub ctor: u32,
        pub vtable: u32,
        pub dtor: u32,
    }

    /// `PClassDescriptorConcrete<PArray<PInputAction>>` (`0x009dc580`).
    pub const INPUT_ACTION_ARRAY: Descriptor = Descriptor {
        guard: 0x0194_1f2c,
        object: 0x0194_1e98,
        ctor: 0x009d_fb30,
        vtable: 0x00b7_3abc,
        dtor: 0x00b0_8120,
    };
    /// `PClassDescriptorConcrete<PArray<PInputSource>>` (`0x009dc610`).
    pub const INPUT_SOURCE_ARRAY: Descriptor = Descriptor {
        guard: 0x0194_1e0c,
        object: 0x0194_1d78,
        ctor: 0x009d_fbd0,
        vtable: 0x00b7_3984,
        dtor: 0x00b0_8140,
    };

    /// A 5-byte `jmp` to a cdecl `target`: its `eax`.
    pub fn jmp_thunk(h: &mut impl Host, target: u32, args: &[u32]) -> u32 {
        h.call_sym(target, Conv::Cdecl, args)
    }

    /// `code_009db170()`: the word at `0x012fbc40`.
    pub fn get_word_012fbc40(h: &mut impl Host) -> u32 {
        WORD_012FBC40.get(h)
    }

    /// The descriptor's accessor (`0x009dc580`, `0x009dc610`): built once
    /// (guard bit 1: constructor, the `+0x90` flag bit 1 cleared, the
    /// concrete vtable, the destructor registered with `atexit`), then its
    /// default instance (`getDefaultInstance`'s `eax`).
    pub fn descriptor(h: &mut impl Host, d: Descriptor) -> u32 {
        let guard = h.rebase(d.guard);
        let object = h.rebase(d.object);
        let g: u32 = h.read(guard);
        if g & 1 == 0 {
            h.write(guard, g | 1);
            h.call_sym(d.ctor, Conv::Thiscall(object), &[]);
            let flags: u32 = h.read(object + 0x90);
            h.write(object + 0x90, flags & !2);
            let vt = h.rebase(d.vtable);
            h.write(object, vt);
            let dtor = h.rebase(d.dtor);
            h.call_sym(ATEXIT, Conv::Cdecl, &[dtor]);
        }
        h.call_sym(GET_DEFAULT_INSTANCE, Conv::Thiscall(object), &[])
    }

    /// `0x0042d370(state)`: the page step over `+0x10` with `+4`; when it
    /// returns 0, the end flag (`+0x2a` bit 7) set when the granule
    /// position (`+0x40`, i64) is positive and the remaining length
    /// (`+0x48`, i64) is not; then 0. A nonzero step result is returned.
    pub fn ogg_page_end(h: &mut impl Host, this: u32) -> u32 {
        let arg: u32 = h.read(this + 4);
        let r = h.call_sym(OGG_PAGE_STEP, Conv::Thiscall(this + 0x10), &[arg]);
        if r != 0 {
            return r;
        }
        let pos = h.read::<u64>(this + 0x40).cast_signed();
        let left = h.read::<u64>(this + 0x48).cast_signed();
        let flags: u8 = h.read(this + 0x2a);
        if pos > 0 && left <= 0 {
            h.write(this + 0x2a, flags | 0x80);
        } else {
            h.write(this + 0x2a, flags & 0x7f);
        }
        0
    }
}
