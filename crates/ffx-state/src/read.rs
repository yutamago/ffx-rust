//! A [`Snapshot`] read from the game's memory. Only reads; every read goes
//! through [`Reader`], which answers `false` where nothing is mapped (the
//! value then reads as 0), so a snapshot can be taken at any time, before
//! the game set anything up too.
//!
//! The addresses are `FFX.exe`'s, with their sources in `symbols/ffx.sym`
//! and docs/formats (chara.md for the characters, save.md for
//! `g_SaveData`); the character record's offsets are `ffx_chara::layout`'s.

use ffx_chara::layout as l;

use crate::{Actor, Bone, Camera, Input, Snapshot, World};

/// The game's memory, read only.
pub trait Reader {
    /// Reads `out.len()` bytes at `at`; false (and `out` untouched) where
    /// they are not all mapped.
    fn read_bytes(&self, at: u32, out: &mut [u8]) -> bool;
}

impl<F: Fn(u32, &mut [u8]) -> bool> Reader for F {
    fn read_bytes(&self, at: u32, out: &mut [u8]) -> bool {
        self(at, out)
    }
}

/// `g_SaveData`, the game state block (docs/formats/save.md).
const SAVE_DATA: u32 = 0x0112_ca90;
const SAVE_MAP: u32 = 0;
const SAVE_PREVIOUS_MAP: u32 = 2;
const SAVE_ENTRANCE: u32 = 0xc;
const SAVE_STORY: u32 = 0xbec;
/// `g_EventName`: the running event's name (`char[]`).
const EVENT_NAME: u32 = 0x023c_bc60;
/// Control work 0 (7 works of 0x238 bytes): `+0` flags (bit 1: the player
/// has control, `AtelUcOn`), `+0x1f4` the event's frame count.
const CTRL_WORK: u32 = 0x0132_5ba0;
const CTRL_CONTROL: u8 = 0x02;
const EVENT_FRAME: u32 = 0x0132_5d94;
/// `g_BtlStatus` (`i8`, nonzero in battle).
const BTL_STATUS: u32 = 0x0112_a8e0;
/// `g_PauseOn`.
const PAUSE_ON: u32 = 0x0134_0828;
/// `g_FadeActive` (a bit per fade slot).
const FADE_ACTIVE: u32 = 0x0130_0704;

/// `g_Camera`: the camera channel (`MsCameraGetPosNow`); `+0x50` the point looked at,
/// `+0x61c + 0x50` the eye (`MsCameraGetPosNow`).
const G_CAMERA: u32 = 0x0231_1430;
const CAMERA_TARGET: u32 = 0x50;
const CAMERA_EYE: u32 = 0x61c + 0x50;
/// `g_PhyreScene`: the scene object; `+0xdc` its main camera (a
/// `PCamera`: `+0xbc` far, `+0x150` aspect, `+0x154` vertical FOV),
/// `+0xfa04` the camera's world 3x4 (docs/formats/render.md, "Cameras").
const G_PHYRE_SCENE: u32 = 0x00cc_b9d8;
const SCENE_CAMERA: u32 = 0xdc;
const SCENE_CAMERA_WORLD: u32 = 0xfa04;
const PCAMERA_FAR: u32 = 0xbc;
const PCAMERA_ASPECT: u32 = 0x150;
const PCAMERA_FOV_Y: u32 = 0x154;

/// `g_PadHeld`, the field movement's pad (`Sg_ControlMain`); the stick's
/// y and x bytes just below it.
const PAD_HELD: u32 = 0x023c_44ac;
const STICK_Y: u32 = 0x023c_44a4;
const STICK_X: u32 = 0x023c_44a5;
/// `g_PadPressed` (`AtelGetControler`).
const PAD_PRESSED: u32 = 0x0132_70d0;

/// `g_ChrCount` (`i32`, slots) and `g_ChrArray` (the records).
const CHR_COUNT: u32 = 0x023c_44e0;
const CHR_ARRAY: u32 = 0x023c_44e4;
/// At most this many slots are read (the game makes a few dozen).
const MAX_SLOTS: u32 = 256;
/// `g_Player`: the player character's record, 0 none.
const PLAYER: u32 = 0x0130_0788;
/// The root's world matrix in a record (`Ch_CalcElement`).
const WORLD_MATRIX: u32 = 0x1d0;
/// The model header's element count (`u16` at `+0xa`, `Ch_GetNbElem`).
const HEADER_ELEMENTS: u32 = 0xa;
/// An element's global matrix (relative to the character).
const ELEM_GLOBAL: u32 = 0x88;
/// At most this many bones a character.
const MAX_BONES: u32 = 512;

struct Mem<'a>(&'a dyn Reader);

impl Mem<'_> {
    fn bytes<const N: usize>(&self, at: u32) -> [u8; N] {
        let mut b = [0u8; N];
        if !self.0.read_bytes(at, &mut b) {
            b = [0; N];
        }
        b
    }
    fn u8(&self, at: u32) -> u8 {
        self.bytes::<1>(at)[0]
    }
    fn u16(&self, at: u32) -> u16 {
        u16::from_le_bytes(self.bytes(at))
    }
    fn i16(&self, at: u32) -> i16 {
        i16::from_le_bytes(self.bytes(at))
    }
    fn u32(&self, at: u32) -> u32 {
        u32::from_le_bytes(self.bytes(at))
    }
    fn i32(&self, at: u32) -> i32 {
        i32::from_le_bytes(self.bytes(at))
    }
    fn f32(&self, at: u32) -> f32 {
        let v = f32::from_le_bytes(self.bytes(at));
        if v.is_finite() { v } else { 0.0 }
    }
    /// `N` floats (at most 16) in one read; non-finite ones read as 0.
    fn floats<const N: usize>(&self, at: u32) -> [f32; N] {
        const { assert!(N <= 16) };
        let mut b = [0u8; 64];
        let n = 4 * N.min(16);
        if !self.0.read_bytes(at, &mut b[..n]) {
            return [0.0; N];
        }
        std::array::from_fn(|i| {
            let v = f32::from_le_bytes([b[4 * i], b[4 * i + 1], b[4 * i + 2], b[4 * i + 3]]);
            if v.is_finite() { v } else { 0.0 }
        })
    }
    /// Whether `LEN` (at most 64) bytes at `at` are mapped.
    fn mapped<const LEN: usize>(&self, at: u32) -> bool {
        const { assert!(LEN <= 64) };
        let mut b = [0u8; LEN];
        self.0.read_bytes(at, &mut b)
    }
    /// A NUL-terminated string of at most `max` bytes (Latin-1 as UTF-8).
    fn c_string(&self, at: u32, max: usize) -> String {
        if at == 0 {
            return String::new();
        }
        let mut s = String::new();
        for i in 0..max {
            let b = self.u8(at.wrapping_add(u32::try_from(i).unwrap_or(0)));
            if b == 0 {
                break;
            }
            s.push(char::from(b));
        }
        s
    }
}

/// The snapshot now: `update` and `game_time` are the host's (the clock's
/// updates run and its time), the rest is read from the game.
#[must_use]
pub fn read(mem: &dyn Reader, update: u32, game_time: f64) -> Snapshot {
    let m = Mem(mem);
    Snapshot {
        update,
        game_time,
        world: world(&m),
        camera: camera(&m),
        input: Input {
            held: m.u32(PAD_HELD),
            pressed: m.u16(PAD_PRESSED),
            stick_x: m.u8(STICK_X),
            stick_y: m.u8(STICK_Y),
        },
        actors: actors(&m),
    }
}

fn world(m: &Mem<'_>) -> World {
    World {
        map: m.u16(SAVE_DATA + SAVE_MAP),
        previous_map: m.u16(SAVE_DATA + SAVE_PREVIOUS_MAP),
        entrance: m.u8(SAVE_DATA + SAVE_ENTRANCE),
        story: m.u16(SAVE_DATA + SAVE_STORY),
        event: m.c_string(EVENT_NAME, 32),
        event_frame: m.u32(EVENT_FRAME),
        battle: m.u8(BTL_STATUS) != 0,
        player_control: m.u8(CTRL_WORK) & CTRL_CONTROL != 0,
        paused: m.u8(PAUSE_ON) != 0,
        fading: m.u32(FADE_ACTIVE) != 0,
    }
}

fn camera(m: &Mem<'_>) -> Camera {
    let scene = m.u32(G_PHYRE_SCENE);
    let pcamera = if scene == 0 {
        0
    } else {
        m.u32(scene.wrapping_add(SCENE_CAMERA))
    };
    let world_at = scene.wrapping_add(SCENE_CAMERA_WORLD);
    let valid = pcamera != 0 && m.mapped::<48>(world_at);
    let channel = m.u32(G_CAMERA);
    let (eye, target) = if channel == 0 {
        ([0.0; 3], [0.0; 3])
    } else {
        (
            m.floats(channel.wrapping_add(CAMERA_EYE)),
            m.floats(channel.wrapping_add(CAMERA_TARGET)),
        )
    };
    let at = |o: u32| m.f32(pcamera.wrapping_add(o));
    Camera {
        valid,
        world: if valid { m.floats(world_at) } else { [0.0; 12] },
        fov_y: if valid { at(PCAMERA_FOV_Y) } else { 0.0 },
        aspect: if valid { at(PCAMERA_ASPECT) } else { 0.0 },
        far: if valid { at(PCAMERA_FAR) } else { 0.0 },
        eye,
        target,
    }
}

fn actors(m: &Mem<'_>) -> Vec<Actor> {
    let base = m.u32(CHR_ARRAY);
    let count = u32::try_from(m.i32(CHR_COUNT)).unwrap_or(0).min(MAX_SLOTS);
    if base == 0 || count == 0 {
        return Vec::new();
    }
    let player = m.u32(PLAYER);
    let slot_of = |handle: u32| -> i16 {
        if handle < base || !(handle - base).is_multiple_of(l::SIZE) {
            return -1;
        }
        let i = (handle - base) / l::SIZE;
        if i < count {
            i16::try_from(i).unwrap_or(-1)
        } else {
            -1
        }
    };
    (0..count)
        .filter_map(|slot| {
            let at = base.wrapping_add(slot * l::SIZE);
            let f = |o: u32| at.wrapping_add(o);
            if m.u8(f(l::ACTIVE)) == 0 {
                return None;
            }
            let id = m.u16(f(l::ID));
            let frame = m.u32(f(l::MOTION_FRAME));
            #[allow(clippy::cast_precision_loss)] // 8.8 fixed frames
            let motion_frame = frame as f32 / 256.0;
            Some(Actor {
                slot: u16::try_from(slot).unwrap_or(0),
                id,
                kind: u8::try_from(id >> 12).unwrap_or(0),
                model: m.c_string(m.u32(f(l::NAME)), 16),
                position: m.floats(f(l::POS)),
                yaw: m.f32(f(l::ROT)),
                scale: m.floats(f(l::SCALE)),
                hidden: m.u8(f(l::HIDE)),
                alpha: m.f32(f(l::FADE_ALPHA)),
                motion: m.u32(f(l::MOTION_ID)),
                motion_frame,
                motion_speed: f32::from(m.i16(f(l::MOTION_SPEED))) / 256.0,
                parent: slot_of(m.u32(f(l::BIND_PARENT))),
                parent_bone: u16::try_from(m.u32(f(l::BIND_ELEM))).unwrap_or(0),
                player: at == player,
                world: m.floats(f(WORLD_MATRIX)),
                bones: bones(m, at),
            })
        })
        .collect()
}

/// The record at `at`'s elements, posed.
fn bones(m: &Mem<'_>, at: u32) -> Vec<Bone> {
    let header = m.u32(at.wrapping_add(l::MODEL_HEADER));
    let elems = m.u32(at.wrapping_add(l::ELEMS));
    if header == 0 || elems == 0 {
        return Vec::new();
    }
    let n = u32::from(m.u16(header.wrapping_add(HEADER_ELEMENTS))).min(MAX_BONES);
    (0..n)
        .map(|i| Bone {
            matrix: m.floats(elems.wrapping_add(i * l::ELEM_SIZE + ELEM_GLOBAL)),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// A sparse memory: only the bytes written are mapped.
    struct Sparse(HashMap<u32, u8>);

    impl Sparse {
        fn put(&mut self, at: u32, b: &[u8]) {
            for (i, &v) in b.iter().enumerate() {
                self.0.insert(at.wrapping_add(u32::try_from(i).unwrap()), v);
            }
        }
    }

    impl Reader for Sparse {
        fn read_bytes(&self, at: u32, out: &mut [u8]) -> bool {
            let mut tmp = vec![0u8; out.len()];
            for (i, b) in tmp.iter_mut().enumerate() {
                match self.0.get(&at.wrapping_add(u32::try_from(i).unwrap())) {
                    Some(&v) => *b = v,
                    None => return false,
                }
            }
            out.copy_from_slice(&tmp);
            true
        }
    }

    #[test]
    fn nothing_mapped_reads_as_an_empty_world() {
        let s = read(&Sparse(HashMap::new()), 5, 2.0);
        assert_eq!(s.update, 5);
        assert_eq!(s.world, World::default());
        assert!(!s.camera.valid);
        assert_eq!(s.actors.len(), 0);
    }

    #[test]
    fn garbage_pointers_read_as_nothing() {
        // pointers near the top of the address space: every offset wraps
        let mut mem = Sparse(HashMap::new());
        let top: u32 = 0xffff_fff0;
        mem.put(CHR_ARRAY, &top.to_le_bytes());
        mem.put(CHR_COUNT, &i32::MAX.to_le_bytes());
        mem.put(G_PHYRE_SCENE, &top.to_le_bytes());
        mem.put(G_CAMERA, &top.to_le_bytes());
        mem.put(top, &[1; 16]);
        let s = read(&mem, 0, 0.0);
        assert!(!s.camera.valid);
        // the slot at the top reads as in use; its fields and bones wrap to nothing
        assert!(s.actors.iter().all(|a| a.bones.is_empty()));
    }

    #[test]
    fn the_live_characters_by_slot() {
        let mut mem = Sparse(HashMap::new());
        let base: u32 = 0x0400_0000;
        mem.put(CHR_ARRAY, &base.to_le_bytes());
        mem.put(CHR_COUNT, &3i32.to_le_bytes());
        mem.put(PLAYER, &(base + 2 * l::SIZE).to_le_bytes());
        // three slots of zeros, the second and third in use
        mem.put(base, &vec![0u8; 3 * l::SIZE as usize]);
        for slot in [1u32, 2] {
            let at = base + slot * l::SIZE;
            mem.put(at + l::ACTIVE, &[1]);
            mem.put(
                at + l::ID,
                &(0x0001u16 + u16::try_from(slot).unwrap()).to_le_bytes(),
            );
            mem.put(
                at + l::POS,
                &[1.0f32, -2.0, 3.0].map(f32::to_le_bytes).concat(),
            );
            mem.put(at + l::MOTION_FRAME, &(10 * 256 + 128u32).to_le_bytes());
        }
        // slot 1 is bound to slot 2 (a weapon to its holder)
        mem.put(
            base + l::SIZE + l::BIND_PARENT,
            &(base + 2 * l::SIZE).to_le_bytes(),
        );
        let s = read(&mem, 0, 0.0);
        assert_eq!(s.actors.len(), 2);
        let (a, b) = (&s.actors[0], &s.actors[1]);
        assert_eq!((a.slot, a.id, a.kind), (1, 2, 0));
        assert_eq!(a.position, [1.0, -2.0, 3.0]);
        assert!((a.motion_frame - 10.5).abs() < 1e-6);
        assert_eq!(a.parent, 2);
        assert!(!a.player);
        assert!(b.player);
        assert_eq!(b.parent, -1);
    }
}
