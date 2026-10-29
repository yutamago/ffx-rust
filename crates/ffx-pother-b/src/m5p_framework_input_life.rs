//! Object lifecycles of list `phyre-framework-input` (batch 7):
//! `PApplication`'s destructor, `PInputDevicePad`'s constructor, and the
//! motion blur's render-target check.

use ffx_graphic::host::{Conv, Host};

/// `Phyre::PhyreFree(p)` (cdecl).
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// `PApplication`'s vtable.
pub const APPLICATION_VTABLE: u32 = 0x00b3_fe38;
/// `PInputDevice`'s and `PInputDevicePad`'s vtables.
pub const DEVICE_VTABLE: u32 = 0x00b3_fbf8;
pub const PAD_VTABLE: u32 = 0x00b3_fca4;
/// `PWorld`'s entry release (thiscall).
pub const WORLD_RELEASE: u32 = 0x0044_2a80;
/// `PRenderSurface::~PRenderSurface` (a bare `ret`).
pub const SURFACE_DTOR: u32 = 0x005b_3640;
/// The renderer's destructor body (thiscall).
pub const RENDERER_DTOR: u32 = 0x005b_49e0;
/// `eh vector destructor iterator` / `constructor iterator` (stdcall).
pub const EH_VECTOR_DTOR: u32 = 0x0094_96d2;
pub const EH_VECTOR_CTOR: u32 = 0x0094_966a;
/// The application's arrays (in the list): `+0x50` pair, `+0x48` 0xe4
/// elements.
pub const RELEASE_A4_9C: u32 = 0x0062_1230;
pub const RELEASE_E4: u32 = 0x0062_0990;
/// `PProcessorProfileTimelineBase`'s destructor (in the list).
pub const TIMELINE_DTOR: u32 = 0x0060_ec70;
/// `g_PInputDeviceList` (head `+0`, tail `+4`).
pub const DEVICE_LIST: u32 = 0x00c3_0fa8;
/// The pad slot's constructor and destructor.
pub const SLOT_CTOR: u32 = 0x0062_0540;
pub const SLOT_DTOR: u32 = 0x0062_1190;
/// `memset(p, v, n)` (cdecl).
pub const MEMSET: u32 = 0x0094_964c;
/// `PTexture2DD3D11::isBound` (thiscall, `al`), `unbind` (thiscall).
pub const TEXTURE_IS_BOUND: u32 = 0x0059_5f90;
pub const TEXTURE_UNBIND: u32 = 0x0059_cc40;
/// The render target's resize `(w, h, format, 0)` (thiscall, `ret 0x10`).
pub const TARGET_RESIZE: u32 = 0x004c_40c0;

pub mod bridge {
    use super::{
        APPLICATION_VTABLE, Conv, DEVICE_LIST, DEVICE_VTABLE, EH_VECTOR_CTOR, EH_VECTOR_DTOR, Host,
        MEMSET, PAD_VTABLE, PHYRE_FREE, RELEASE_A4_9C, RELEASE_E4, RENDERER_DTOR, SLOT_CTOR,
        SLOT_DTOR, SURFACE_DTOR, TARGET_RESIZE, TEXTURE_IS_BOUND, TEXTURE_UNBIND, TIMELINE_DTOR,
        WORLD_RELEASE,
    };

    /// `PApplication::~PApplication` (`0x00620bc0`): its vtable; the name
    /// `+0x398` freed unless borrowed; the world `+0x2c4` released; the
    /// viewport array (`+0x2bc`, owned data freed even when null) cleared;
    /// the renderer `+0x124`'s four surfaces (`+0x20c`, 0x10 each, and the
    /// one at `+0x24c`) and its body; the arrays at `+0x50` and `+0x48`;
    /// the array at `+0x40` (owned non-null data freed) cleared; the
    /// profile timeline `+0xc`. `eax`: that last array release's
    /// (`PhyreFree`'s, 0 or `0xfe`).
    pub fn application_dtor(h: &mut impl Host, this: u32) -> u32 {
        let vt = h.rebase(APPLICATION_VTABLE);
        h.write(this, vt);
        let name: u32 = h.read(this + 0x398);
        if name & 1 == 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[name]);
        }
        h.call_sym(WORLD_RELEASE, Conv::Thiscall(this + 0x2c4), &[]);
        let count: u32 = h.read(this + 0x2bc);
        if count & 0x8000_0000 == 0 {
            let data: u32 = h.read(this + 0x2c0);
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
        }
        h.write(this + 0x2c0, 0u32);
        h.write(this + 0x2bc, 0u32);
        let renderer = this + 0x124;
        let surfaces = renderer + 0xe8;
        h.call_sym(SURFACE_DTOR, Conv::Thiscall(surfaces + 0x40), &[]);
        let dtor = h.rebase(SURFACE_DTOR);
        h.call_sym(EH_VECTOR_DTOR, Conv::Stdcall, &[surfaces, 0x10, 4, dtor]);
        h.call_sym(RENDERER_DTOR, Conv::Thiscall(renderer), &[]);
        h.call_sym(RELEASE_A4_9C, Conv::Thiscall(this + 0x50), &[]);
        let arr = this + 0x40;
        h.call_sym(RELEASE_E4, Conv::Thiscall(arr + 8), &[]);
        let count: u32 = h.read(arr);
        let data: u32 = h.read(arr + 4);
        let eax = if count & 0x8000_0000 != 0 {
            0xfe
        } else if data == 0 {
            0
        } else {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data])
        };
        h.write(arr + 4, 0u32);
        h.write(arr, 0u32);
        // the timeline's destructor only stores its vtable: `eax` stays
        h.call_sym(TIMELINE_DTOR, Conv::Thiscall(this + 0xc), &[]);
        eax
    }

    /// `PInputDevicePad::PInputDevicePad(device)` (`0x006201a0`, `ret 4`):
    /// the device's two empty lists (`+4` then linked at the tail of the
    /// global device list, `+0xc` the mappings), the `DirectInput` device
    /// `+0x14`, the pad's vtable, four 0x30-byte slots at `+0x6c`
    /// constructed, the axes, buttons, vibration and state cleared, the
    /// 0x244-byte device instance at `+0x12c` zeroed; `this`.
    pub fn pad_ctor(h: &mut impl Host, this: u32, device: u32) -> u32 {
        let node = this + 4;
        h.write(node, node);
        h.write(node + 4, node);
        let vt = h.rebase(DEVICE_VTABLE);
        h.write(this, vt);
        h.write(this + 0xc, this + 0xc);
        h.write(this + 0x10, this + 0xc);
        h.write(this + 0x14, device);
        let head = h.rebase(DEVICE_LIST);
        let tail: u32 = h.read(head + 4);
        h.write(node, head);
        h.write(node + 4, tail);
        h.write(tail, node);
        h.write(head + 4, node);
        let vt = h.rebase(PAD_VTABLE);
        h.write(this, vt);
        let ctor = h.rebase(SLOT_CTOR);
        let dtor = h.rebase(SLOT_DTOR);
        h.call_sym(
            EH_VECTOR_CTOR,
            Conv::Stdcall,
            &[this + 0x6c, 0x30, 4, ctor, dtor],
        );
        for off in [
            0x3ac, 0x378, 0x374, 0x370, 0x384, 0x380, 0x37c, 0x390, 0x38c, 0x388, 0x394,
        ] {
            h.write(this + off, 0u32);
        }
        h.write(this + 0x69, 0u8);
        for off in [0x398, 0x39c, 0x3a0, 0x3a4] {
            h.write(this + off, 0u32);
        }
        h.write_bytes(this + 0x18, &[0; 0x20]);
        for off in [0x48, 0x38, 0x4c, 0x3c, 0x50, 0x40, 0x54, 0x44] {
            h.write(this + off, 0u32);
        }
        h.write(this + 0x58, 0u16);
        h.write(this + 0x5a, 0u8);
        for off in [0x5c, 0x60, 0x64] {
            h.write(this + off, 0f32.to_bits());
        }
        h.write(this + 0x68, 0u8);
        h.call_sym(MEMSET, Conv::Cdecl, &[this + 0x12c, 0, 0x244]);
        this
    }

    /// One render target of the blur: `Some(eax)` to return, `None` to go
    /// on. A target already `(w, h)` goes on; else a bound texture is
    /// unbound (non-zero returned), then the target resized to `(w, h,
    /// its format, 0)` (non-zero returned).
    fn check_target(h: &mut impl Host, target: u32, w: u32, ht: u32) -> Option<u32> {
        let cw: u32 = h.read(target + 0x1c);
        let ch: u32 = h.read(target + 0x20);
        if cw == w && ch == ht {
            return None;
        }
        if h.call_sym(TEXTURE_IS_BOUND, Conv::Thiscall(target), &[]) as u8 != 0 {
            let r = h.call_sym(TEXTURE_UNBIND, Conv::Thiscall(target), &[]);
            if r != 0 {
                return Some(r);
            }
        }
        let format: u32 = h.read(target);
        let r = h.call_sym(TARGET_RESIZE, Conv::Thiscall(target), &[w, ht, format, 0]);
        if r != 0 { Some(r) } else { None }
    }

    /// `PMotionBlur`'s target check (`0x00a235f0`, thiscall `(w, h)`,
    /// `ret 8`): the two targets (`+0xcd8`, `+0xd4c`: a holder whose byte
    /// `+4` is clear and whose `+0` is the target) sized to `(w, h)`; 1
    /// for a missing or flagged holder or a missing target, a failing
    /// step's `eax`, else 0.
    pub fn check_targets(h: &mut impl Host, this: u32, w: u32, ht: u32) -> u32 {
        for off in [0xcd8, 0xd4c] {
            let holder: u32 = h.read(this + off);
            if holder == 0 || h.read::<u8>(holder + 4) != 0 {
                return 1;
            }
            let target: u32 = h.read(holder);
            if target == 0 {
                return 1;
            }
            if let Some(r) = check_target(h, target, w, ht) {
                return r;
            }
        }
        0
    }
}
