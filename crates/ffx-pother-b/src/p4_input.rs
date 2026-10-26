//! The input layer (`PInputDevice*`, `PInputMapper`'s mappings), the
//! application viewport and frame, and the scene's search-tree insert.
//!
//! A mapping (`0x30` bytes) is a list link (`+0`, `+4`), its device
//! (`+8`), its id (`+0xc`), scale (`+0x10`), offset (`+0x14`), deadzone
//! (`+0x18`), value (`+0x1c`), raw value (`+0x20`), state / pressed /
//! released bytes (`+0x24..0x26`), and the device byte (`+0x28`) or word
//! (`+0x2c`) it reads.

use ffx_bridge::x87;
use ffx_graphic::host::{Conv, Host};

/// `PApplicationViewport`'s vtable.
pub const VIEWPORT_VTABLE: u32 = 0x00b3_ff50;
/// `PInputDevice`'s vtable.
pub const DEVICE_VTABLE: u32 = 0x00b3_fbf8;
/// `PInputDeviceKeyboard`'s vtable.
pub const KEYBOARD_VTABLE: u32 = 0x00b3_fc30;
/// `g_PInputDeviceList`: the devices' list head (`+0` first, `+4` last).
pub const DEVICE_LIST: u32 = 0x00c3_0fa8;
/// `crt_memset_thunk` (cdecl `(p, value, n)`).
pub const MEMSET: u32 = 0x0094_964c;
/// `Phyre_PFramework_PInput_Update` (cdecl, `eax` an error code).
pub const INPUT_UPDATE: u32 = 0x0062_5770;
/// `SaveDataTtyWrite2` (cdecl `(level, fmt, ...)`).
pub const TTY_WRITE: u32 = 0x0064_8850;
/// `g_PhyreApplication`.
pub const G_APPLICATION: u32 = 0x00cc_9cd8;
/// `PInputMapper_update` (thiscall `(mapper)`).
pub const MAPPER_UPDATE: u32 = 0x0062_e040;
/// `"EXIT_GAME_DEBUG: PInput::Update() error! result=%d\n"`.
pub const MSG_INPUT: u32 = 0x00b4_0c54;
/// `"EXIT_GAME_DEBUG: PApplication::Update() error! result=%d\n"`.
pub const MSG_APP: u32 = 0x00b4_0c88;
/// `Distortion_GetInstance` (cdecl, the singleton in `eax`).
pub const DISTORTION_INSTANCE: u32 = 0x006c_2290;
/// The distortion getters `0x00675120` stores at `+0x10..0x24`, in order
/// (thiscall on the instance, no stack words).
pub const DISTORTION_GETTERS: [u32; 6] = [
    0x006c_26f0,
    0x006c_26b0,
    0x006c_26e0,
    0x006c_26c0,
    0x006c_2700,
    0x006c_26d0,
];

/// The device field an axis id 1-9 reads (`+0x10c` x, `+0x110` y, `+0x114`
/// the wheel; ids repeat every three), or `None`.
#[must_use]
pub fn mouse_axis_field(id: u32) -> Option<u32> {
    match id {
        1 | 4 | 7 => Some(0x10c),
        2 | 5 | 8 => Some(0x110),
        3 | 6 | 9 => Some(0x114),
        _ => None,
    }
}

/// `(f32)(int) / divisor` as the originals do it: `fild`, stored to `f32`,
/// divided by a `double`, stored to `f32`.
#[must_use]
pub fn scaled(v: u32, divisor: f64) -> f32 {
    // `fild` is exact; the `fstp m32` rounds to nearest, as `as` does
    #[allow(clippy::cast_precision_loss)]
    let f = v as i32 as f32;
    f32::from_bits(x87::store(x87::div(f64::from(f), divisor)))
}

/// The value `PInputMapping_update` (`0x006263e0`) stores: `(value at
/// +0x1c, raw at +0x20)` for a device reading `raw`.
#[must_use]
pub fn mapping_value(id: u32, raw: u32, scale: u32, offset: u32, deadzone: u32) -> (u32, u32) {
    let raw_i = raw as i32;
    // ids 1, 2, 4, 5 keep any value; 3, 6, the 10-bit ids 7-10 and
    // unknown ids go through the deadzone
    let (norm, half, stick) = match id {
        1 | 2 | 4 | 5 => (scaled(raw, 255.0), raw_i.wrapping_add(0xff) / 2, true),
        3 | 6 => (scaled(raw, 255.0), raw_i.wrapping_add(0xff) / 2, false),
        7..=10 => (scaled(raw, 1023.0), raw_i.wrapping_add(0x3ff) / 2, false),
        _ => (0.0, 0, false),
    };
    let v = x87::add(
        x87::mul(x87::load(scale), f64::from(norm)),
        x87::load(offset),
    );
    let v = x87::store(v);
    if !stick {
        // the stick ids 1, 2, 4, 5 skip the deadzone; for the others a
        // magnitude under it gives 0 (a NaN on either side keeps `v`)
        let mag = x87::load(v).abs();
        let dz = x87::load(deadzone);
        if dz > mag {
            return (0, half as u32);
        }
    }
    (v, half as u32)
}

pub mod bridge {
    //! The originals over game memory.

    use super::{
        Conv, DEVICE_LIST, DEVICE_VTABLE, DISTORTION_GETTERS, DISTORTION_INSTANCE, G_APPLICATION,
        Host, INPUT_UPDATE, KEYBOARD_VTABLE, MAPPER_UPDATE, MEMSET, MSG_APP, MSG_INPUT, TTY_WRITE,
        VIEWPORT_VTABLE, mapping_value, mouse_axis_field, scaled,
    };

    /// `PApplicationViewport::PApplicationViewport(vp, app)` (`0x0061fdd0`,
    /// `ret 4`) -> `vp`: the app's `+0x40`, `+0x50`, `+0x124` blocks, scale
    /// 1.0, size from the app's `+0x26c`/`+0x270` (after 640x480).
    pub fn viewport_ctor(h: &mut impl Host, vp: u32, app: u32) -> u32 {
        h.write(vp + 0x1c, 0x280u32);
        h.write(vp + 0x10, 1f32);
        h.write(vp + 4, app.wrapping_add(0x40));
        h.write(vp + 8, app.wrapping_add(0x50));
        h.write(vp + 0xc, app.wrapping_add(0x124));
        h.write(vp + 0x20, 0x1e0u32);
        let vt = h.rebase(VIEWPORT_VTABLE);
        h.write(vp, vt);
        h.write(vp + 0x14, 0u32);
        h.write(vp + 0x18, 0u32);
        h.write(vp + 0x24, 0u32);
        let w: u32 = h.read(app.wrapping_add(0x26c));
        h.write(vp + 0x1c, w);
        let hh: u32 = h.read(app.wrapping_add(0x270));
        h.write(vp + 0x20, hh);
        vp
    }

    /// The `PInputDevice` base: empty lists at `+4` and `+0xc`, the state
    /// word at `+0x14`, linked at the tail of `g_PInputDeviceList`.
    fn device_base(h: &mut impl Host, dev: u32, state: u32) {
        let link = dev + 4;
        let maps = dev + 0xc;
        h.write(link, link);
        h.write(link + 4, link);
        let vt = h.rebase(DEVICE_VTABLE);
        h.write(dev, vt);
        h.write(maps, maps);
        h.write(maps + 4, maps);
        h.write(dev + 0x14, state);
        let head = h.rebase(DEVICE_LIST);
        let last: u32 = h.read(head + 4);
        h.write(link + 4, last);
        h.write(link, head);
        h.write(last, link);
        h.write(head + 4, link);
    }

    /// `PInputDeviceKeyboard::PInputDeviceKeyboard(dev, state)`
    /// (`0x0061fff0`, `ret 4`) -> `dev`: the device base, then the three
    /// 256-byte key arrays `+0x218`, `+0x18`, `+0x118` cleared.
    pub fn keyboard_ctor(h: &mut impl Host, dev: u32, state: u32) -> u32 {
        device_base(h, dev, state);
        let vt = h.rebase(KEYBOARD_VTABLE);
        h.write(dev, vt);
        let f = h.rebase(MEMSET);
        for at in [0x218, 0x18, 0x118] {
            h.call(f, Conv::Cdecl, &[dev + at, 0, 0x100]);
        }
        dev
    }

    /// `PInputDeviceMouse::getAxis(dev, id)` (`0x00628a40`, vtable `+0x10`,
    /// `ret 4`) -> `ST0`: the axis id's field `/ 255`, 0 for other ids.
    pub fn mouse_get_axis(h: &mut impl Host, dev: u32, id: u32) -> f32 {
        match mouse_axis_field(id) {
            Some(at) => scaled(h.read(dev + at), 255.0),
            None => 0.0,
        }
    }

    /// `PInputDeviceMouse::bind(dev, mapping, id)` (`0x00625d10`, vtable
    /// `+8`, `ret 8`) -> 0, 0x12 (already bound) or 5 (unknown id): ids
    /// 0xb-0xd read the byte `+0x10d + id`, axis ids 1-9 the word field;
    /// the mapping is linked at the tail of the device's list `+0xc`.
    pub fn mouse_bind(h: &mut impl Host, dev: u32, mapping: u32, id: u32) -> u32 {
        let (byte, word) = if id.wrapping_sub(0xb) < 3 {
            (dev.wrapping_add(0x10d).wrapping_add(id), 0)
        } else {
            match mouse_axis_field(id) {
                Some(at) => (0, dev + at),
                None => return 5,
            }
        };
        if h.read::<u32>(mapping + 8) != 0 {
            return 0x12;
        }
        if byte != 0 {
            h.write(mapping + 0x2c, 0u32);
            h.write(mapping + 0x28, byte);
        } else {
            h.write(mapping + 0x28, 0u32);
            h.write(mapping + 0x2c, word);
        }
        h.write(mapping + 8, dev);
        let list = dev + 0xc;
        h.write(mapping + 0xc, id);
        let last: u32 = h.read(list + 4);
        h.write(mapping + 4, last);
        h.write(mapping, list);
        h.write(last, mapping);
        h.write(list + 4, mapping);
        0
    }

    /// `PInputMapping_update(mapping, raw)` (`0x006263e0`, `ret 4`).
    pub fn mapping_update(h: &mut impl Host, mapping: u32, raw: u32) {
        let id: u32 = h.read(mapping + 0xc);
        let (v, half) = mapping_value(
            id,
            raw,
            h.read(mapping + 0x10),
            h.read(mapping + 0x14),
            h.read(mapping + 0x18),
        );
        h.write(mapping + 0x20, half);
        h.write(mapping + 0x1c, v);
    }

    /// `PInputDevice::updateMappings(dev)` (`0x0062d550`, fastcall): each
    /// mapping of the list `+0xc`: a byte mapping latches its state with
    /// the pressed (`+0x25`) and released (`+0x26`) edges, a word mapping
    /// runs [`mapping_update`] on the word.
    pub fn update_mappings(h: &mut impl Host, dev: u32) {
        let list = dev + 0xc;
        let mut m: u32 = h.read(list);
        while m != list && m != 0 {
            let byte: u32 = h.read(m + 0x28);
            if byte == 0 {
                let word: u32 = h.read(m + 0x2c);
                if word != 0 {
                    let raw: u32 = h.read(word);
                    mapping_update(h, m, raw);
                }
            } else {
                let old: u8 = h.read(m + 0x24);
                let now: u8 = h.read(byte);
                h.write(m + 0x25, u8::from(old == 0 && now == 1));
                h.write(m + 0x26, u8::from(old == 1 && now == 0));
                h.write(m + 0x24, now);
            }
            m = h.read(m);
        }
    }

    /// `App_Frame()` (`0x0062e340`) -> 0, or 9 when `PInput::Update` fails
    /// or the application quits (`+0x39`), each with its debug line; else
    /// the mapper (`app + 0x2b8`) update.
    pub fn app_frame(h: &mut impl Host) -> u32 {
        let f = h.rebase(INPUT_UPDATE);
        let r = h.call(f, Conv::Cdecl, &[]);
        let tty = h.rebase(TTY_WRITE);
        if r != 0 {
            let msg = h.rebase(MSG_INPUT);
            h.call(tty, Conv::Cdecl, &[0xc, msg, r]);
            return 9;
        }
        let app: u32 = h.read(h.rebase(G_APPLICATION));
        if h.read::<u8>(app + 0x39) != 0 {
            let msg = h.rebase(MSG_APP);
            h.call(tty, Conv::Cdecl, &[0xc, msg, 9]);
            return 9;
        }
        let mapper: u32 = h.read(app + 0x2b8);
        let f = h.rebase(MAPPER_UPDATE);
        h.call(f, Conv::Thiscall(mapper), &[]);
        0
    }

    /// A search tree's `insert(tree, node)` (`ret 4`): the node's tag `+8`
    /// gets its low bit set, its links `+0`, `+4` point at the sentinel
    /// `tree`; from the root `[tree + 0xc]`, a greater key goes to `[n]`,
    /// a smaller to `[n + 4]`. An equal key returns that node; else the
    /// node is hung at the empty side, its `+8` gets the parent (with its
    /// tag bit), and 0. `signed` picks the comparison (`0x0064bfa0`
    /// signed, `0x0064c020` and `0x006a5fc0` unsigned).
    pub fn tree_insert(h: &mut impl Host, tree: u32, node: u32, signed: bool) -> u32 {
        let tag: u32 = h.read(node + 8);
        let low = u32::from(h.read::<u8>(node + 8) & 1);
        h.write(node + 8, tag.wrapping_add(1 - low));
        h.write(node, tree);
        h.write(node + 4, tree);
        let mut parent = tree + 0xc;
        let mut side = 0u32;
        let mut n: u32 = h.read(tree + 0xc);
        if n != tree {
            let key: u32 = h.read(node + 0xc);
            loop {
                parent = n;
                let k: u32 = h.read(n + 0xc);
                let greater = if signed {
                    (k as i32) > (key as i32)
                } else {
                    k > key
                };
                if !greater && k == key {
                    return n;
                }
                side = u32::from(!greater);
                n = h.read(n + 4 * side);
                if n == tree {
                    break;
                }
            }
        }
        let low = u32::from(h.read::<u8>(node + 8) & 1);
        h.write(node + 8, low.wrapping_add(parent));
        h.write(parent + 4 * side, node);
        0
    }

    /// `0x00675120(params, x)` (`ret 8`) -> 0: the distortion singleton's
    /// six settings into `params + 0x10..0x24`.
    pub fn distortion_params(h: &mut impl Host, params: u32) -> u32 {
        for (k, getter) in DISTORTION_GETTERS.into_iter().enumerate() {
            let f = h.rebase(DISTORTION_INSTANCE);
            let inst = h.call(f, Conv::Cdecl, &[]);
            let f = h.rebase(getter);
            let v = h.call(f, Conv::Thiscall(inst), &[]);
            h.write(params + 0x10 + 4 * k as u32, v);
        }
        0
    }

    /// `PInputMapper::latchButtons(mapper)` (`0x00628320`, thiscall): for
    /// the 7 keyboards (`+0x80`) keys `0x1c..0x11c` and the 18 pads
    /// (`+0x38`) buttons `0xb..0x1b`, each device's `vtable[3](id)`; a
    /// value that differs from the cached one (keyboard `+0x118 + i`, pad
    /// `+0x28 + i`) is latched at `+0x18 + i`, then cached.
    pub fn latch_buttons(h: &mut impl Host, mapper: u32) {
        for (list, devices, first, cache, keys) in [
            (0x80u32, 7u32, 0x1cu32, 0x118u32, 0x100u32),
            (0x38, 18, 0xb, 0x28, 0x10),
        ] {
            for k in 0..devices {
                let dev: u32 = h.read(mapper + list + 4 * k);
                if dev == 0 {
                    continue;
                }
                for i in 0..keys {
                    let v = h.call_virtual(dev, 3, &[first + i]) as u8;
                    if v != h.read::<u8>(dev + cache + i) {
                        h.write(dev + 0x18 + i, v);
                    }
                    h.write(dev + cache + i, v);
                }
            }
        }
    }
}

/// `PInputMapper::getStickAxis` (`0x00628470`)'s value from its deadzone
/// word `dz` and the axis mapping's value word (`None`: no mapping): the
/// value (through `fld`/`fstp`) unless its magnitude is under the
/// deadzone, then 0.
#[must_use]
pub fn stick_axis(dz: u32, own: Option<u32>) -> f32 {
    let x = own.map_or(0, crate::methods::quiet);
    if x87::load(dz) > x87::load(x).abs() {
        0.0
    } else {
        f32::from_bits(x)
    }
}

/// The pad deadzone `PInputMapper::attachPads` sets (`0.2f`, `0x00b412c4`).
pub const PAD_THRESHOLD: u32 = 0x3e4c_cccd;

pub mod bridge4 {
    //! The originals over game memory.

    use super::{DEVICE_LIST, Host, PAD_THRESHOLD, stick_axis};

    /// `PInputMapper::getStickAxis(mapper, axis, pad)` (`0x00628470`, `ret
    /// 8`) -> `ST0`. The pad (`[mapper + 0x38 + 4 pad]`; none gives 0)
    /// has its stick mappings at `+0x38..0x44` (left x, left y, right x,
    /// right y; value `+0x1c`, threshold `+0x18`). The cross deadzone:
    /// axis 0 takes its own threshold while axis 1's magnitude is under
    /// axis 1's threshold (a NaN is not under), else 0; 1 against 0, 2
    /// against 3, 3 against 2; other axes 0. The axis's mapping (`+0x38 +
    /// 4 axis`, read for any axis) gives the value.
    pub fn get_stick_axis(h: &mut impl Host, mapper: u32, axis: u32, pad: u32) -> f32 {
        let dev: u32 = h.read(mapper.wrapping_add(pad.wrapping_mul(4)).wrapping_add(0x38));
        if dev == 0 {
            return 0.0;
        }
        let dz = if axis <= 3 {
            let other: u32 = h.read(dev + 0x38 + 4 * (axis ^ 1));
            let v: u32 = h.read(other + 0x1c);
            let t: u32 = h.read(other + 0x18);
            if ffx_bridge::x87::load(t) > ffx_bridge::x87::load(v).abs() {
                let own: u32 = h.read(dev + 0x38 + 4 * axis);
                h.read(own + 0x18)
            } else {
                0
            }
        } else {
            0
        };
        let m: u32 = h.read(dev.wrapping_add(axis.wrapping_mul(4)).wrapping_add(0x38));
        let own = if m == 0 { None } else { Some(h.read(m + 0x1c)) };
        stick_axis(dz, own)
    }

    /// `PInputMapper::attachPads(mapper)` (`0x00626aa0`, thiscall): for
    /// pads 0..17, the pad-th device of kind 2 (`vtable[1]()`) in
    /// `g_PInputDeviceList` (the walk ends the whole call when it runs
    /// out); an empty slot `+0x38 + 4 pad` takes it, with its four stick
    /// mappings at `+0x6c + 0x30 k` (`+0x38..0x44`) bound (`vtable[2](m,
    /// id)`, ids 1, 2, 4, 5) and their thresholds 0.2.
    pub fn attach_pads(h: &mut impl Host, mapper: u32) {
        let head = h.rebase(DEVICE_LIST);
        for pad in 0..18u32 {
            let mut link: u32 = h.read(head);
            let mut seen = 0u32;
            let dev = loop {
                if link == head || link == 0 || link == 4 {
                    return;
                }
                let dev = link - 4;
                if h.call_virtual(dev, 1, &[]) == 2 {
                    if seen == pad {
                        break dev;
                    }
                    seen += 1;
                }
                link = h.read(dev + 4);
            };
            let slot = mapper + 0x38 + 4 * pad;
            if h.read::<u32>(slot) != 0 {
                continue;
            }
            h.write(slot, dev);
            for k in 0..4 {
                h.write(dev + 0x38 + 4 * k, dev + 0x6c + 0x30 * k);
            }
            for (k, id) in [1u32, 2, 4, 5].into_iter().enumerate() {
                let m: u32 = h.read(dev + 0x38 + 4 * k as u32);
                h.call_virtual(dev, 2, &[m, id]);
            }
            for k in 0..4 {
                let m: u32 = h.read(dev + 0x38 + 4 * k);
                h.write(m + 0x18, PAD_THRESHOLD);
            }
        }
    }
}
