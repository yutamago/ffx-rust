//! `PRenderer`'s command encoders of list p6-render-c (continuation
//! round): each makes room in the renderer's two rings, takes a 0x48-byte
//! command from the command ring and writes its opcode and payload.
//!
//! The renderer (`this`): the data ring at `+0x1c` (cursor `+0x24`, end
//! `+0x28`), the command ring at `+0x30` (cursor `+0x38`, end `+0x3c`),
//! the frame's first command `+0x4c` and its end `+0x50`. A command:
//! opcode `+0`, payload from `+8`.
//!
//! Frame (the originals' `sub esp, 8`): the rings' check writes its
//! wrap-around cursor through a pointer to a local, `ebp - 4` for the
//! data ring and `ebp - 8` for the command ring; the bridges take that
//! frame's address (`ebp - 8`).

use ffx_bridge::x87;
use ffx_graphic::host::{Conv, Host};

/// `PRenderRingBuffer::check(ring; cursor, size, &wrapped)` (thiscall,
/// `ret 0xc`): the cursor after `size` more bytes; the wrapped-around
/// cursor through the pointer.
pub const RING_CHECK: u32 = 0x0061_3f00;
/// `PRenderRingBuffer::allocate(ring; size)` (thiscall, `ret 4`): the
/// block, or 0.
pub const RING_ALLOCATE: u32 = 0x0061_3e10;
/// A command's size.
pub const COMMAND_SIZE: u32 = 0x48;
/// The result when the command ring is out of room.
pub const OUT_OF_MEMORY: u32 = 0xd;
/// The result of a scene call out of order (`beginScene` in a scene,
/// `endScene`/`clearScene` outside one).
pub const BAD_STATE: u32 = 0x16;

/// The encoders with a plain payload: (original, opcode, payload).
pub const ENCODERS: &[(u32, u32, Payload)] = &[
    (0x005b_ada0, 6, Payload::Floats(4)),
    (0x005b_af10, 9, Payload::Floats(1)),
    (0x005b_afc0, 0xa, Payload::Words(1)),
    (0x005b_bdc0, 5, Payload::Words(4)),
    (0x005b_aa40, 0x18, Payload::ByteWords(2)),
    (0x005b_b390, 0x1e, Payload::ByteWords(1)),
];

/// The scene encoders (one word of payload): (original, opcode, what
/// they do with the in-scene bit 0 of `+4`).
pub const SCENE: &[(u32, u32, Scene)] = &[
    (0x005b_5e30, 0x13, Scene::Begin),
    (0x005b_6950, 0x15, Scene::End),
    (0x005b_6330, 0x14, Scene::Clear),
];

/// `PRenderCommandQueue::queueCallback(f, data)` (`0x005b5880`).
pub const QUEUE_CALLBACK: u32 = 0x005b_5880;
/// Its opcode.
pub const CALLBACK_OPCODE: u32 = 0x2c;

/// A payload at `+8`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Payload {
    /// `n` floats, each through `fld`/`fstp` (a signalling NaN quieted).
    Floats(u32),
    /// `n` words.
    Words(u32),
    /// A byte (the first argument's low byte) at `+8`, then `n` words
    /// from `+0xc`.
    ByteWords(u32),
}

impl Payload {
    /// The arguments the encoder takes (stack words after `this`).
    #[must_use]
    pub const fn words(self) -> u32 {
        match self {
            Payload::Floats(n) | Payload::Words(n) => n,
            Payload::ByteWords(n) => n + 1,
        }
    }
}

/// A scene encoder's state rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scene {
    /// Fails in a scene; else enters it (bit set before the command).
    Begin,
    /// Fails outside a scene; else leaves it.
    End,
    /// Fails outside a scene.
    Clear,
}

/// The encoder at `addr` ([`ENCODERS`]; a compile error when there is
/// none: the shims look theirs up in a `const` block).
#[must_use]
pub const fn encoder(addr: u32) -> (u32, Payload) {
    let mut i = 0;
    while i < ENCODERS.len() {
        if ENCODERS[i].0 == addr {
            return (ENCODERS[i].1, ENCODERS[i].2);
        }
        i += 1;
    }
    panic!("no such encoder")
}

/// The scene encoder at `addr` ([`SCENE`]), as [`encoder`].
#[must_use]
pub const fn scene(addr: u32) -> (u32, Scene) {
    let mut i = 0;
    while i < SCENE.len() {
        if SCENE[i].0 == addr {
            return (SCENE[i].1, SCENE[i].2);
        }
        i += 1;
    }
    panic!("no such scene encoder")
}

pub mod bridge {
    //! The originals over game memory.

    use super::{
        BAD_STATE, CALLBACK_OPCODE, COMMAND_SIZE, Conv, Host, OUT_OF_MEMORY, Payload,
        RING_ALLOCATE, RING_CHECK, Scene, x87,
    };

    /// Whether a ring (`ring`; cursor and end read before the check) has
    /// no room for `size` bytes: the check's new cursor `r` and wrapped
    /// cursor `o` (written to `out`): with the cursor not past the end,
    /// a wrap (`o` below the cursor) or `r` past the end.
    pub fn full(h: &mut impl Host, ring: u32, size: u32, out: u32) -> bool {
        let cur: u32 = h.read(ring + 8);
        let end: u32 = h.read(ring + 0xc);
        let r = h.call_sym(RING_CHECK, Conv::Thiscall(ring), &[cur, size, out]);
        let o: u32 = h.read(out);
        cur <= end && (o < cur || r > end)
    }

    /// Room for one command: when either ring is full (the data ring for
    /// nothing more, the command ring for a command), the frame goes to
    /// the render thread (virtual slot 2, `submitFrame`); then the
    /// command from the command ring, recorded as the frame's first when
    /// there is none, and as its end. 0 when the ring has no room.
    /// `frame` is the original's `ebp - 8`.
    pub fn begin_command(h: &mut impl Host, renderer: u32, frame: u32) -> u32 {
        if full(h, renderer + 0x1c, 0, frame + 4) || full(h, renderer + 0x30, COMMAND_SIZE, frame) {
            h.call_virtual(renderer, 2, &[]);
        }
        let cmd = h.call_sym(
            RING_ALLOCATE,
            Conv::Thiscall(renderer + 0x30),
            &[COMMAND_SIZE],
        );
        if cmd != 0 {
            if h.read::<u32>(renderer + 0x4c) == 0 {
                h.write(renderer + 0x4c, cmd);
            }
            h.write(renderer + 0x50, cmd.wrapping_add(COMMAND_SIZE));
        }
        cmd
    }

    /// A plain encoder (`thiscall(renderer; args)`): the command with
    /// `opcode` and `payload` from `args`; 0, or [`OUT_OF_MEMORY`].
    pub fn encode(
        h: &mut impl Host,
        renderer: u32,
        frame: u32,
        opcode: u32,
        payload: Payload,
        args: &[u32],
    ) -> u32 {
        let cmd = begin_command(h, renderer, frame);
        if cmd == 0 {
            return OUT_OF_MEMORY;
        }
        match payload {
            Payload::Floats(_) => {
                for (k, &a) in args.iter().enumerate() {
                    let v = x87::round_trip(f32::from_bits(a));
                    h.write(cmd + 8 + 4 * k as u32, v);
                }
            }
            Payload::Words(_) => {
                for (k, &a) in args.iter().enumerate() {
                    h.write(cmd + 8 + 4 * k as u32, a);
                }
            }
            Payload::ByteWords(_) => {
                h.write(cmd + 8, args[0] as u8);
                for (k, &a) in args[1..].iter().enumerate() {
                    h.write(cmd + 0xc + 4 * k as u32, a);
                }
            }
        }
        h.write(cmd, opcode);
        0
    }

    /// A scene encoder (`thiscall(renderer; word)`): [`BAD_STATE`] when
    /// the in-scene bit (bit 0 of `+4`) is wrong for `rule`; else the bit
    /// updated and the command with `opcode` and the word.
    pub fn encode_scene(
        h: &mut impl Host,
        renderer: u32,
        frame: u32,
        opcode: u32,
        rule: Scene,
        word: u32,
    ) -> u32 {
        let state: u32 = h.read(renderer + 4);
        let in_scene = state & 1 != 0;
        match rule {
            Scene::Begin if in_scene => return BAD_STATE,
            Scene::End | Scene::Clear if !in_scene => return BAD_STATE,
            Scene::Begin => h.write(renderer + 4, state | 1),
            Scene::End => h.write(renderer + 4, state & !1),
            Scene::Clear => {}
        }
        encode(h, renderer, frame, opcode, Payload::Words(1), &[word])
    }

    /// `PRenderCommandQueue::queueCallback(f, data)` (`0x005b5880`,
    /// thiscall on the renderer): 1 for no `f`; else the command
    /// `0x2c {f, data}`.
    pub fn queue_callback(h: &mut impl Host, renderer: u32, frame: u32, f: u32, data: u32) -> u32 {
        if f == 0 {
            return 1;
        }
        encode(
            h,
            renderer,
            frame,
            CALLBACK_OPCODE,
            Payload::Words(2),
            &[f, data],
        )
    }
}
