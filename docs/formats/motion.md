# Skeletons and motions

The remaster draws the new PhyreEngine models (`docs/formats/phyre.md`) but
still animates them with the PS2 skeletons and motions:

- `ffx_ps2/ffx/master/jppc/chr/<kind>/<name>/mdl/<name>.chr`: the model
  block with the bind pose;
- `…/chr/<kind>/<name>/mot/resident0`–`3.mgrp`: the character's own motion
  groups (16-byte files are empty);
- `…/event/obj/**.mgrp` and `battle/mot/regmot.mgrp`: motions loaded for an
  event or for battle.

Reader/writer: [`crates/ffx-motion`](../../crates/ffx-motion). All 3,156
motion groups (21,165 clips) parse and are rebuilt byte for byte, also
through the JSON form (`ffx motion export/import`). `ffx model export` adds
a character's clips to its glTF as animations; all 857 `chr/` models
validate.

## Characters

A character id is `kind << 12 | number` (`sg_getdbtype`, `0x00829e10`;
`ch_create_chrname`, `0x00838060`):

| Kind | Letter | Folder |
|---|---|---|
| 0 | c | `pc` |
| 1 | m | `mon` |
| 2 | n | `npc` |
| 3 | s | `sum` |
| 4 | w | `wep` |
| 5 | f | `obj` |
| 6 | k | `skl` |

A motion id is `character << 16 | number`. `Ch_SetSequence` (`0x00837ac0`)
looks the number up in the sequences of the id's character, which need not
be the actor's own: 166 of the 242 monsters without motions of their own
have the same skeleton as one that has them, and the `.chr` files of 178 of
them hold motion ids of exactly one other monster (m001 names `0x1002….`).
The exporter uses the named character's motions for models without their
own (inferred from the data; the table holding these ids in the `.chr`
header is not decoded yet).

## Skeleton (`.chr`)

`+0x10` is the offset of the model block (0 in three `chr/skl` files). In
it, `+0x0a` (`u16`) is the bone count and `+0x1c` the offset of the bone
table, relative to the block. `SG_InitBones` (`0x008277f0`) reads 20 bytes
per bone:

| Offset | Type | Meaning |
|---|---|---|
| `0x00` | `u16` | Parent bone; a root is its own parent |
| `0x02` | `i16` × 3 | Rotation x, y, z, 1/100 degree |
| `0x08` | `i16` × 3 | Translation, 1/1000 unit |
| `0x0e` | `i16` × 3 | Scale, 1/4096 (the game keeps the integer part) |

PS2 bone *i* is the remastered model's node `SkeletonNode_i`, whose local
matrix equals this bind pose (checked on c001). A bone's local transform is
`T · Rz · Ry · Rx · S`: the x axis of c001's bone 11 (24.57°, 8.92°,
15.55°) is (0.952, 0.265, −0.155) in both.

The node and the bone agree for every bone that skins a vertex, but other
HD nodes can differ: c001's bones 103-135 are identity in the HD model,
c101's base bone 150 has a zero matrix, and c041 lacks 30 nodes and
differs on bones 87-91 (ffx-demaster `docs/models.md`). A model's stored
vertices are the bind pose in its base bone's frame: posed by the skin
lists' bones and weights, Σ w · bone · p matches base · vertex to within
0.01-0.04 units, so the skin lists (`docs/formats/main.md`,
`sg_calcenvelope_asm`) only add bones and weights. For the kept monsters,
NPCs and weapons, an HD vertex × 1000 equals the PS2 `i16` bit for bit
(ffx-demaster `docs/research/hd-vs-ps2-assets.md`). Party models carry up
to 8 bones per vertex, m112 up to 14.

## Motion group (`.mgrp`)

`SG_AddMotionGroup` (`0x00836c90`) registers a group and
`SG_RelocateGroup` (`0x00836f70`) turns its offsets into pointers.

| Offset | Type | Meaning |
|---|---|---|
| `0x00` | `u32` | 0 (the load address once relocated) |
| `0x04` | `u32` | Sequence count |
| `0x08` | `u32` | 0 |
| `0x0c` | `u32` | Offset of the sequence table, which ends the file |

Sequence, 20 bytes (`SG_AddSequence`, `0x00836d30`):

| Offset | Type | Meaning |
|---|---|---|
| `0x00` | `u32` | Flags |
| `0x04` | `u16` | Character |
| `0x06` | `u16` | 0 |
| `0x08` | `u16` | Motion count |
| `0x0a` | `u16` | Clip count |
| `0x0c` | `u32` | Offset of the motion table |
| `0x10` | `u32` | Offset of the clip table |

Motion, 16 bytes: `u16` number (the low half of a motion id), `u16`, `u16`
label count, `u16` script length, `u32` offset of the labels (`u16` script
offsets), `u32` offset of the script.

Clip, 16 bytes: `u32` 0, `u32` mark count, `u32` offset of the marks (`u16`
frames: 0, the frame count, then loop and cue points), `u32` offset of the
clip header.

Each sequence's data is laid out as: for each clip, its marks (4-byte
aligned), then its header (8-byte aligned); then the clip table; then each
motion's labels and script (4-byte aligned); then the motion table. Padding
bytes are `0x77`.

### Motion scripts

`SG_SeqStep` (`0x00837900`) runs a motion's script. Arguments are `u16`,
unaligned.

| Opcode | Arguments | Meaning |
|---|---|---|
| 0 | | End |
| 1 | clip, loops, start mark, end mark | Play a clip |
| 2 | | Wait for the clip |
| 3 | count | Wait `count` frames |
| 4 | label | Jump |
| 5 | | Stop |
| 6 | value | Continue once a counter reaches `value` |

### Clip

`Ch_MotionSetupFcurve` (`0x00839980`) reads the header:

| Offset | Type | Meaning |
|---|---|---|
| `0x00` | `u16` | Frame count |
| `0x02` | `u16` | Bone count (bones past the skeleton's are ignored) |
| `0x04` | `u16` | Frame rate × 256 (`0x1e00`) |
| `0x06` | `u16` | Event count |
| `0x08` | `u32` | Offset of the channel types; the header ends there (0x10 or 0x18) |
| `0x0c` | `u32` | Offset of the values |
| `0x10` | `u32` | Offset of the events (8 bytes each: four `u16`, meaning unknown) |
| `0x14` | `u32` | 0 |

Every bone has nine channels: rotation x, y, z in 1/4096 turn, translation
x, y, z in 1/100 unit (`SG_ApplyBones`, `0x00838f70`, scales by the
character definition's `+0x38` × 0.001; the factor 0.01 matches the bind
poses of c001 and m002), and scale x, y, z in 1/4096.

The types are packed two bits per channel, lowest first:

| Type | Value |
|---|---|
| 0 | 0 |
| 1 | 4096 |
| 2 | A constant `i16` from the values |
| 3 | A key stream from the values: `u16` byte length (including itself, even), then the bytes |

`SG_StepChannels` (`0x008394d0`) decodes a stream one frame at a time. The
value starts at 0 and each frame adds a delta:

| Bytes | Delta |
|---|---|
| `0xxxxxxx` | 7-bit signed |
| `11xxxxxx yyyyyyyy` | 14-bit signed, `y` the high byte |
| `10nnnnnn` | The previous delta, for this frame and `n` more |

The shipped encoder never follows a full 64-frame run with another run; it
writes the delta again first. `SG_SampleChannels` (`0x008393c0`)
interpolates between frames, rotations the short way round.

One clip (w001's `resident1.mgrp`) is uncompressed (`SG_ApplyRawClip`,
`0x00838750`): `u16` frames, `u16` bones, `f32` frame rate, 8 zero bytes,
then per bone and channel one `i16` per frame.

## Open questions

- The events (footsteps, effects?) and the sequence flags.
- Where the `.chr` header keeps the motion ids that name another
  character's motions.
- What sets the character definition's `+0x38` (taken as 10).
