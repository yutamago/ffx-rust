# Scripts

Events, NPCs, story progression and monster AI all run on one bytecode
interpreter, `Script_Run` (`0x008641e0`). Scripts live in:

- event files, `ffx_ps2/ffx/master/jppc/event/**/*.ebp` (397), section 0;
- monster files, `ffx_ps2/ffx/master/jppc/battle/mon/_mNNN/mNNN.bin` (361),
  section 0 (registered by `Mon_LoadAll` through `0x00797a40`).

Disassembler/assembler: [`crates/ffx-script`](../../crates/ffx-script).

## Instructions

`Script_Fetch` (`0x00869d60`): a byte below `0x80` is an opcode alone; a
byte with the high bit set is opcode `byte & 0x7f` followed by a `u16`
operand. The program counter advances by `((byte | 0x40) >> 6)`, 1 or 3.
Opcodes reach their cases through the byte table at `0x0086536c`; Ghidra
resolves it, so its `case` labels are opcodes.

The machine is a stack machine with registers A and B, a float
accumulator, four integer and ten float registers, and per-worker
variables. The full table with the evidence for each opcode is
[`crates/ffx-script/src/opcode.rs`](../../crates/ffx-script/src/opcode.rs).
In summary:

| Opcodes | Meaning |
|---|---|
| `00`, `1d`, `1e`, `76` | no-op |
| `01`–`05` | logical or/and; bitwise or/xor/and |
| `06`–`0f` | comparisons: eq, ne, gt/ge/lt/le, signed and unsigned |
| `10`–`13` | bit test, inverted bit test, shift left, arithmetic shift right |
| `14`–`18`, `1a` | add, sub, mul, div, mod (int or float), float negate |
| `19`, `1c` | logical not, bitwise not |
| `1f`–`24`, `27` | variables: push, pop (two store modes), array element push/pop, element address |
| `25`, `26` | float accumulator pop/push |
| `28`, `29`, `2a`, `2c` | registers A, B push/pop |
| `2d`, `2e`, `2f` | push integer constant, 16-bit immediate, float constant |
| `30`, `31`, `32` | jump to label: always, if A ≠ 0, if A = 0 (`55`–`57`: pop into A first) |
| `33`, `34` | call another worker's first entry (3 deep), return |
| `35`, `58` | call a native function (`58` then pops the result into the float accumulator) |
| `36`–`38`, `45`–`53` | requests to other workers (three families of six) |
| `3c`–`3f`, `40`, `54` | end the entry (four variants), wait, halt |
| `59`–`74` | integer and float register pop/push |
| `75` | push global constant (table `0x00c52704`) |
| `1b`, `41`–`44`, `7b`–`7f` | invalid: stop with an error; never used |

### Comparisons

With `a` pushed first and `b` on top:

| | `==` | `!=` | `>` | `>=` | `<` | `<=` |
|---|---|---|---|---|---|---|
| signed | `06` | `07` | `0a` | `0e` | `0b` | `0f` |
| unsigned | | | `08` | `0c` | `09` | `0d` |

Floats use the same opcodes.

## Native functions

`func id` splits the ID into a group (`id >> 12`) and an index; groups
0, 1, 4–9, b, c, d have tables registered by `AtelInitTotal`
(`0x0086d6d0`). Each 16-byte entry holds four handlers: start (called once),
poll (called until done, so a call can span frames), float result and
integer result. The shipped scripts call 948 distinct native functions,
2.3 million times. `ffx script disasm` annotates each call with its handler's
address (and name, when `symbols/ffx.sym` has one); for example
`0x70ab` → `0x007a5360`, which sets a battle actor property
(`0x007b4b70`).

`ffx script natives` lists every native the shipped scripts call (event
scripts, monster AI and battle scripts), with its call count, the number
of scripts calling it, its four handler slots and its name. The 226 most
used natives, by calls and by scripts, are named in `symbols/ffx.sym`
(`Native_*`, with arguments in push order and the evidence), which covers
97 % of the 3.1 million calls. By group:

| Group | Area | Examples |
|---|---|---|
| 0 | workers, movement, triggers, messages, sound, party | `Native_MsgSetText` (`0x0064`: window, message), `Native_MsgOpen` (`0x006a`), `Native_MsgWait` (`0x007c`), `Native_MoveStart` (`0x0018`), `CT_0013_setPosition`, `Native_MapJump` (`0x010b`: map, entrance), `Native_WaitFrames_0000`, `Native_SpriteShow` (`0x011b`) |
| 1 | maths | `AtelCall_Math_Sqrt_RF` (`0x1006`) |
| 4 | screen | `Native_FadeIn` (`0x4004`), `Native_FadeOut` (`0x4005`), `Native_LightSet` (`0x4013`), `CT_401D_init` (`0x401d`) |
| 5 | field characters | `Native_ChrPlayMotion` (`0x501a`), `Native_ChrWaitMotion_501e`, `Native_ChrHide` (`0x5008`), `Native_ChrSetMotionBlend` (`0x5034`) |
| 6 | camera | `Native_CamSetPos` (`0x6002`), `Native_CamMove` (`0x6010`), `Native_CamSetScreenDepth` (`0x603b`) |
| 7 | battle | `Native_BtlUseCommand` (`0x700b`: targets, command), `Native_GetActorProperty` (`0x700f`), `Native_SetActorsProperty` (`0x7018`), `Native_ActorPlayMotion` (`0x7007`) |
| 8 | field effects, map parts | `Native_FpRun` (`0x8002`), `Native_FpBind` (`0x8009`), `Native_MapPartShow` (`0x800f`) |

Names marked `?` in their comments are inferred, not traced end to end.

## Script block

All offsets are relative to the block start (`B`). Code offsets in tables
are relative to the code start.

| Offset | Type | Meaning |
|---|---|---|
| `0x00` | `u32` | Code length |
| `0x10` | `u32` | Block length |
| `0x30` | `u32` | Code start; the only header field `Script_Run` reads |
| `0x34` | `u16` | Worker count |
| `0x38` | `u32` × *n* | Worker header offsets |

Other header words (`0x04`–`0x2c`) point to tables before and after the
code and are kept as they are.

Worker header (`0x34` bytes):

| Offset | Type | Meaning |
|---|---|---|
| `0x00` | `u16` | Worker type; `Script_Run` treats types 5 and 6 specially |
| `0x02` | `u16` | Variable count; table at `0x14`, 8 bytes each (`u32` type in the top nibble and location, `u16` array length) |
| `0x04` | `u16` | Integer constant count; table at `0x18` |
| `0x06` | `u16` | Float constant count; table at `0x1c`, ending at the code start |
| `0x08` | `u16` | Entry point count; table at `0x20` |
| `0x0a` | `u16` | Jump label count; table at `0x24` (`Script_Jump`, `0x008726f0`) |
| `0x30` | `u32` | End of the code, rounded up to 4 |

Every worker's entries and labels lie inside its own contiguous code range,
and the ranges follow worker order. This holds for all 758 shipped scripts,
which is what lets the disassembler name jump targets per worker.

## Assembly

```bash
ffx script list znkd                     # files with scripts, filtered
ffx script disasm m001 -o raldo.asm      # a monster's AI
ffx script disasm ffx_ps2/ffx/master/jppc/event/obj/zn/znkd0900/znkd0900.ebp
ffx script asm m001 raldo.asm --mod-dir mods/my-mod
```

```text
worker 1 type 2
e0:
    pushimm 158
    pushimm 16650
    func_facc 0x70ab         ; 0x007a5360
```

`eN:` and `lN:` place entry points and jump labels; jumps name labels
(`jmp l3`). Assembly applies onto the original script: constants,
variables and data stay as they are, and the original entry and label
counts must be kept. Code that fits in a worker's original range is written
there and padded with `nop`; longer code is appended to the end of the
block, and the containing file's later sections shift by a multiple of
`0x40`. That works because `Script_Run` only resolves code through the code
start at `+0x30`; nothing else in the block moves.

## Verification

`cargo test -p ffx-script` (with `FFX_GAME_DIR`) disassembles and
reassembles all 758 scripts, 17.3 million instructions, and checks that
script blocks and their files come back byte for byte. A second test grows
a worker of `m001` past its range and checks the relocated code, the
shifted sections and that the result round-trips again.

## The VM in memory

What the Atel accessors show of the running VM. The offsets are constants
in [`ffx_script::vm`](../../crates/ffx-script/src/vm/mod.rs), and
`ffx_script::vm::bridge` runs those accessors over game memory, checked
against the original on the field and battle snapshots
(`crates/ffx-oracle/tests/script.rs`).

### Ctrl works

A ctrl work is one running script block: seven of them, `0x238` bytes each,
at `0x01325ba0` (`AtelGetCtrlWork`, `0x0086afa0`). `g_ScriptContext`
(`0x01326b28`), the current one (`AtelGetCurCtrlWork`, `AtelSetCurrentCtrl`,
`AtelChangeCurrentCtrl`), follows the array. Ctrl work 0 runs the event; in
the battle snapshot ctrl works 2 (75 workers) and 3 (6) are in use too.

| Offset | Type | Meaning | Evidence |
|---|---|---|---|
| `0x00` | `u8` | bit 0: in use | `AtelGetEventActorTotal` (`0x0086b300`), `AtelRemoveCtrlWork` (`0x0086ed40`) |
| `0x03` | `u8` | bits 0-1: run mode (1 skips all workers, 2 all but flag `0x8000`) | `Script_Run` |
| `0x0a` | `u16` | the player's worker, `0xffff` none | `AtelGetNowCtrlTopId` (`0x0086c1a0`) |
| `0x0c` | `u16` | workers with a full work area (actors) | `AtelGetActorType` (`0x0086a7b0`), `AtelGetCtrlworkActorTotal` |
| `0x10` | `u16` | all workers | `AtelGetBasicWork` (`0x0086a830`) |
| `0x14`, `0x16`, `0x18` | `u16` | workers of the `0x5b8`, `0x558` and `0x2e8` classes | `AtelGetBasicWork`, `AtelGetTotalActorNumber2` (`0x0086c510`) |
| `0x1a` | `u16` | ctrl work number (0 = the event's) | `AtelGetBasicWork`'s cache, `AtelGetRealVarAdrs` |
| `0x1c` | `u32` | the first worker | `AtelGetBasicWork` |
| `0x20` | `u32` | `&AtelGetBasicWork` (field snapshot) | |
| `0x2c`, `0x30` | `u32` | bases of variable storage 0 and 1 (storage 0 is in the save data in the field) | `AtelGetRealVarAdrs` (`0x0086c2e0`) |
| `0x34`, `0x38` | `u32` | the event's message tables | `AtelActorSetUp2`, `Msg_Text` |
| `0x44` | `u32` | `AtelSetVibrateStreamExt`'s value | `0x008715e0` |
| `0x48` | `u32` | a map record; `+0x18` the map camera (ctrl work 0's) | `AtelGetEventMapCameraId` (`0x0086b5d0`) |
| `0x4c` | `u32` | `AtelExecCtrlEvent` for the event | `AtelActorSetUp2` (`0x00875510`) |
| `0x50` | `u32` | callback `(ctrl)` | `AtelInitActor` (`0x0086cae0`) |
| `0x54` | `u32` | callback `(worker, offset) -> address` for storage 3; null in both snapshots | `AtelGetRealVarAdrs` |
| `0x58`, `0x5c` | `u32` | callbacks `(worker, level)` before and after a worker runs | `Script_Run` |
| `0x1f4` | `u32` | cleared when the ctrl work is freed | `AtelRemoveCtrlWork` |

### Workers

Workers lie back to back from `ctrl + 0x1c`, in five size classes, in this
order (`AtelGetBasicWork`): `actors - n558 - n2e8 - n5b8` of `0xb58` bytes,
then `n558` of `0x558`, `n5b8` of `0x5b8`, `n2e8` of `0x2e8`, and the rest
(up to the count at `+0x10`) of `0x30`. The block header has the same counts at `+0x34`
(all), `+0x36` (actors), `+0x14`, `+0x16`, `+0x1c`
(`AtelGetActorBufferSize`, `0x0086a290`). `AtelGetBasicWork` maps an index
outside `0..0x10` to worker 0 and caches its last result
(`g_AtelBasicWorkCache`, `0x013270e4`) keyed on the ctrl number and the
index (`0x013270dc`, `0x013270e0`); `AtelRemoveCtrlWork` drops the cache.

| Offset | Type | Meaning | Evidence |
|---|---|---|---|
| `0x00` | `u32` | worker header in the block; its first byte is the type | `AtelGetPosWorkAdrs`, `AtelGetActorType` |
| `0x04` | `u32` | the script block | `AtelGetRealVarAdrs` |
| `0x2e` | `u16` | worker index | `AtelGetActorId` (`0x0086a5d0`) |
| `0x32` | `u8` | level running | `AtelPopStackFloat`'s error report |
| `0x34`, `0x36` | `u16` | flags `Script_Run` tests | `Script_Run` |
| `0x3c` | | camera handle cache | `AtelGetCameraWorkAdrs` (`0x0086ae50`) |
| `0x48` | | storage 5 variables | `AtelGetRealVarAdrs` |
| `0x9c` | `u32` | field character | `AtelGetCharHandle` (`0x0086ae80`) |
| `0xc0` | | | `AtelGetBattleIdAdrs2` (`0x0086a990`) |
| `0xc4` | | the value stack (below) | `Script_Run` passes it to every opcode |
| `0x12c` | `0x4c` × 9 | levels (below); types 5 and 6 have two | `Script_WorkerLevel` (`0x00872650`) |
| `0x284` / `0x558` | | position block of types 5 and 6 / of the others (type 4 has none) | `AtelGetPosWorkAdrs` (`0x0086c2b0`) |

A level (one request priority): `+0x18` program counter, `+0x1d` call depth
(three return addresses from `+0`, worker IDs from `+0x10`), `+0x1f` request
state, `+0x20` and `+0x24` registers A and B, `+0x28` the float accumulator.

### The stack

19 tagged slots (`AtelPushStackInteger`, `0x0086e3f0`;
`AtelPushStackFloat`, `0x0086e3c0`): `+0x00` depth (`i32`), `+0x04` the
values (`u32` × 19), `+0x54` their kinds (`u8` × 19: 1 integer, 2 float).
A push onto a full stack is dropped; a pop from an empty one gives 0 (or
0.0); both set `g_AtelError` (`0x013270d8`) to 0. A negative depth is not
checked. Popping converts: `AtelPopStackInteger` (`0x0086df00`) truncates a
non-integer slot with `_ftol2_sse` (`cvttsd2si`: `0x80000000` for NaN and
out of range); `AtelPopStackFloat` (`0x0086de50`) converts a non-float slot
from `i32`. `Script_TopInt` (`0x0086df40`) reads the top and pops it only
when asked; `AtelFetchStackInteger` (`0x0086a000`) is the interpreter's
pop-an-integer.

### Variables

The worker header's `+0x14` (relative to the block) points to 8-byte
descriptors: a location word and, at `+4`, the array length (`u16`; `+6`
another `u16`, `AtelGetVarArrayRange`). The word holds the type in bits
28-31 (`u8`, `i8`, `u16`, `i16`, `u32`, `i32`, `f32`; past 6 nothing is
read or stored), the storage area in bits 25-27 and the offset in bits 0-23
(`AtelGetRealVarAdrs`):

| Storage | Area |
|---|---|
| 0, 1 | ctrl work `+0x2c`, `+0x30` |
| 2, 3, 4 | worker header `+0x28`, `+0x2c`, `+0x30` (offsets in the block; 4 is the data after the worker's code); 3 goes through the ctrl work's `+0x54` callback when it is set |
| 5 | inside the worker, from `+0x48` |
| 6 | block `+0x20` (shared by the block's workers) |
| 7 | none (address 0) |

An element index is clamped to `0..length-1` (`AtelGetValueAdrs`,
`0x0086c570`; element sizes 1, 2 or 4, `AtelGetValueIncValue`).
`AtelGetValueInteger`/`Float` (`0x0086c730`, `0x0086c610`) read by type;
`AtelSetValueInteger`/`Float` (`0x00871400`, `0x008712e0`) store, with
`clamp` saturating the narrow types (the opcodes' "store mode 1"). The
opcodes go through `Script_PushVar` (`1f`), `Script_StoreVar` (`20`, `21`,
`23`, `24`; bit `0x80` pops an index below the value), `Script_PushVarElem`
(`22`) and `Script_PushVarRef` (`27`, which pushes the location word
advanced to the element, not an address).

Floats pass through the x87 (`fld`/`fstp`), which quiets signalling NaNs
on hardware (the game masks the invalid exception); Unicorn, which the
oracle runs on, keeps them signalling, so the parity tests leave sNaNs out
of stored values.

### Globals and the save data

The small accessors (`ffx_script::vm::bridge::globals`) are one load or
store each. The ones on the Atel globals: the music number, the effect
flags and parameters (`AtelSetEffectParam`, `0x0086fa60`, has no bound on
the index), the 3D sound ratios, the battle end status (`0x01326b48`: `+0`
the status, whose bits 8, 9 and 16 the `AtelIs...Clear`/`AtelGetStatusWinFadeOut`
getters read inverted; `+4` the keep-play status, bits 8, 11, 12, 13 and
17), the map id page and event mode, the menu mask, the voice pan
(`0x01327135`) and VAG stream volume (`0x01327134`) that an event set-up
resets to `0x3f` and `0x7f`, the half-turn angle (`0x01327124`, an `f32` in
degrees that `Native_GetTurnKind` compares against), and the pad words the
interpreter's update ORs over the frames since the last one (`0x013270c0`,
six `u16` four bytes apart: held, pad 1 held, repeat, two history words,
pressed). The pad getters load `AX` only (the rest of `EAX` is the
caller's); `AtelGetSphereMonitor` (`0x0086c4b0`) does too, so its `EAX`
keeps the upper half of `&g_SaveData`.

Others read `g_SaveData` (`0x0112ca90`, through `MsGetSaveEventAddress`'s
thunk at `0x0088c830`):

| Offset | Type | Accessors |
|---|---|---|
| `0x10` | `u8` | `AtelGet`/`SetBattleSceneGroup` |
| `0x11`, `0x12` | `u8` | `AtelSetMapFadeType`, `AtelSetMapFadeFrames` (`0x008700c0`, `0x008700d0`) |
| `0x13` | `u8` | battle status: bit 1 `AtelGetBattleStatusSys`, bit 2 `AtelGetBattleStatusDummyEnc` |
| `0x20` | `u8` | `AtelGetSave20` |
| `0xb4`, `0xb6` | `u16` | the map marker (`AtelGetMapMarkerX`/`Y`, `0x00873ae0`/`0x00873af0`) |
| `0xd1` | `u8` | `AtelGetAlbhedRikku` |
| `0xd6` | `u8` | `AtelClearForcePlaceId` |
| `0xd7` | `u8` | `AtelGetWaterBattleEffect` |
| `0xd8`, `0xdc` | `u32` | read by `0x008807b0` |
| `0xe0` | `u32` | `AtelGetOnMemoryMovieFileNo`, `AtelSetOnMemoryMovie` |
| `0xe4` | `u32` | `AtelGetRandomEncountStatus` |
| `0xe8`, `0xea` | `u16` | `AtelGetBattleEndTagAlways`, `AtelGetSphereMonitor` |
| `0xbec` | `u16` | `AtelGetScenarioFlag` |

## The interpreter

`Script_Run(worker, mode, callbacks)` (`0x008641e0`) runs in Rust
(`ffx_script::vm::bridge::run`, the arithmetic in `ffx_script::vm::exec`).
It takes the run lock (`g_ScriptRunMutex`, `0x00c52740`, created on first
use), skips workers whose flags say so (worker `+0x34` bit 1 clear; bit 2
clear with `+0x36` bit `0x40` clear while `MsBattleCheck`; the ctrl work's
run mode), adds speed × `g_GameSpeed` to the worker's accumulator (`+0x94`;
the result's bit 0 says it reached 1), checks the level's wait states, then
fetches and executes instructions until one yields, ends the level or the
stack empties.

Every opcode handler is ported: integer arithmetic and comparisons, float
arithmetic at the x87's precision with its NaN rules (the larger
significand wins, not the first operand as in SSE), pushes and pops,
registers A, B and the float accumulator, variables (`1f`-`27`), jumps,
calls and returns between levels (the three return slots at level `+0`),
request opcodes `39`-`3b`, the ends, breakpoints and the debugger's step
mode. Natives (`35`, `58`) go through the `Natives` trait:
`OriginalNatives` dispatches in Rust (`Script_CallNative` `0x008777a0`,
`Script_PollNative` `0x00877730`, `Script_FinishNative` `0x008777f0`,
`Script_NativeEntry` `0x00877770` over `g_NativeTables`, 16-byte entries of
start, poll, float result and int result) and calls the natives
themselves in the original. A camera native (group 6) sets worker `+0x36`
bit `0x20` unless `0x10` is set; a missing poll means "done, run on" (5);
a float result is stored as `f32` (`fstp m32`) before it is pushed.

What stays original, called through `ffx_bridge::Host` (a cdecl call
into the game, so the hooked Rust version where another module has one):
the run lock (`Mutex_*`), `MsBattleCheck`, the ctrl work's callbacks
(`+0x28` end, `+0x58`/`+0x5c` around a run, `+0x60` the `REQ` gate), the
debug hook `0x0067a9b0` that `REQ` calls first, and the natives not yet
ported (below).

### Signals

Workers ask each other to run entry points through signals
(`ffx_script::vm::bridge::signal`): 24-byte nodes in two doubly linked
lists of the receiving worker, its queue (`+0x80`) and its free nodes
(`+0x88`); `+0x90` is the one it runs. A node's `prev` points at the
`next` field before it (another node's or the list head), so unlinking is
`*prev = next; next.prev = prev`.

| Offset | Type | Meaning |
|---|---|---|
| `0x00`, `0x04` | `u32` | next, prev |
| `0x08` | `u16` | the entry point to run (in a reply: the one that ran) |
| `0x0a`, `0x0c` | `u16` | sender and receiver index (`0xffff` none) |
| `0x0e` | `u8` | bits 0-3 the level, 4-6 state (1 queued, 2 running), 7 new |
| `0x0f` | `u8` | 0 `REQ`, 1 `REQSW` (the sender waits for it to start), 2 `REQEW` (to end), 3 reply |
| `0x10`-`0x14` | `u16` × 3 | the header the entry runs from: binary header index, table index, ctrl work |

`AtelRegistSignalMain` (`0x0086ea00`) takes a free node, fills it and
queues it before the first reply or lower level (replies last). The
header words come from the sender's level 0 (`+0x168`), or from the
request while the current ctrl work has header tables (`+0x1f8`, battle
ctrl works 2 and 3); every caller in the game leaves those three request
words unwritten on its stack, so the original stores stack garbage there.
The Rust callers pass 0 (the oracle tests patch the three loads to 0 on
both sides). `REQ`, `REQSW` and `REQEW` (`Script_OpReq`/`Sw`/`Ew`,
`0x00867230`, `0x00867570`, `0x008673d0`) pop entry, worker and level and
register through one of three forms by opcode: any, without the same tag
(entry), without the same level; `PREQ` (`39`-`3b`) goes through
`AtelGetPartyWorker` and the same-level form. A `REQSW`/`REQEW` level then
waits (`+0x1f` 1) for the reply (`AtelSearchReplySignal`); opcodes
`77`/`78` wait (`+0x1f` 2) while a request is queued
(`AtelIsSignalQueued`).

`AtelDecodeSignal` (`0x008653f0`) starts the head of the queue when it is
a request above the running level: a queued one through
`AtelStartSignal` (`0x008690a0`: entry, level, header, PC; a `REQSW`
sender gets its reply), a running one by switching back to its level.
`AtelExecReturn` (`0x008691d0`, the ends `3c`-`3f`, `54`) replies to a
`REQEW` sender, frees the node, cancels what was queued above the given
level (`AtelCancelSignals`, `0x00872860`), decodes the next and, for
`3e`/`3f`, resets the level to entry 1.

Opcode `79` (`0x00870d30`) writes the worker's vibration curve, eight
`u16` in a variable (`+0x44` points at it, filled from
`TkGetVibrateDefaultCurve` the first time); `7a` (`0x0086fab0`) sets a
`(key, value)` slot in the table at `+0x2e4` of types 5 and 6.

### Natives in Rust

`ffx_script::vm::bridge::native` has the natives that only pop arguments,
touch the worker, the VM's globals and the save data, and call code kept
elsewhere: 79 handlers (message windows, character motion and display
calls, worker ids and positions, turn and move blocks and their waits,
triggers, the minimap marker, random, the pad, fades, HP/MP restore, the
field programs `Fp_*`, ...). `native::find` maps a handler address to its
Rust version; `OriginalNatives`' dispatch uses it before calling the
original, so `Script_Run` in Rust runs them in Rust. Nine of them are
under the five bytes a hook jump needs (`Native_WaitForever` is `xor eax,
eax; ret`) and run in Rust only that way; the rest are in the hook table. The choice is by
static use in the shipped scripts (`cargo run -p ffx-script --example
native_counts` lists every native with its call count and handlers) and
by whether every callee is ported. Float handlers return `ST0` as `f64`.
Where a Ghidra signature says `void`, the `EAX` the handler leaves is what
`Script_FinishNative` pushes (`Native_EnableTrigger` returns the kind it
popped, `Native_TurnSetFrames` the elapsed value).

`ffx_script::vm::bridge::native_map` has group 8 (`0x8000`-`0x806b`, 65
handlers: map parts, the field programs, the map's sprite objects, fog and
back colours and parameters) and the field natives `0x01a5`, `0x01e4`,
`0x01e6`. Most pop their arguments, call the map module and return 1.
The colour and fog ones hand the map module the address of a stack
buffer (`yiGetFogColor(&rgba)`); the Rust ones use `NativeHost::scratch`,
a 64-byte buffer of the game thread (in the oracle, the bottom of the
emulated stack). On stage `0x140` setting a fog colour also feeds the
renderer the colour over 255, each component truncated by `_ftol2_sse`
and passed back as a float; the fog setters skip maps `0xca2`, `0x1e38`
and `0x699`. `Native_FpBindPos` truncates its fp id from `ST0` as
`AtelPopStackFloat` leaves it (an integer slot exactly), and its
`dbgPrintf` is an empty function. Six of them (`0x0091b040`, `0x0091b050`,
`0x0091b060`, `0x0091b680`, `0x0091b690`, `0x0091c4a0`) have no function
start in the Ghidra dump the hook checks use and run in Rust only through
the dispatch.

`ffx_script::vm::bridge::native_cam` has 62 camera handlers of group 6.
Most hand `(worker, args, stack)` to a camera helper (`0x007bb6a0`,
`0x007badb0`, ...) with one or two constants: the half (0 the focus or
"ref" point, 1 the eye) and a mode; the helper pops the arguments. The
others first take the worker's camera handle, cached in the worker's
`+0x3c` and otherwise asked of `MsAtelGetCameraID(5, worker is the event
map's camera actor, 0)` (`native_cam::cam`), then pop what they need.
`Native_CamSetRoll` converts degrees with the `f32` pi as a double
(`(deg * pi) / 180`, rounded to `f32` once). `Native_CamSetScreenDepth`
prints its depth through the `printf` import in debug mode
(`g_DebugMode` 1). `0x6048`/`0x604a` clear `g_BtlSplines[1]`/`[0]` only
when `MsCameraGetID(cam)` is the handle itself. The polls (`0x6038`,
`0x6062`, `0x6063`, `0x6064`) return 5 once their test is 0
(`~-(v != 0) & 5`). `Native_Cam_6076` (`0x007b8fe0`) has no function
start in the dump and runs in Rust only through the dispatch. Round 7 added
`Native_CamMoveFinish` (`0x6016`; with magic `0x92` its second call arms
`g_BtlRandomLatch`, which makes the next `Native_BtlRandom` `0x6080` give 2
for an odd draw and 0 for an even one), `Native_CamWait` (`0x601a`, debug
frame counting like the voice waits), the path natives `0x6059`, `0x605a`,
`0x605b` (path 0 and 1 are the battle spline lists after `g_BtlSplines`,
used only with at least three points; any other path is a script
variable's address) and the variable natives `0x601c`, `0x601d`,
`0x601f`.

`ffx_script::vm::bridge::native_msg` has `Native_MsgSetText`,
`Native_MsgSetPos`, `Native_MsgSetValue`, `Native_MsgClose`,
`Native_MacroDicText`, `Native_BitIndex` (`0x0142`) and the voice waits
`Native_VoiceIsPlaying` (`0x00d8`) / `Native_SoundWait_Poll` (`0x00d9`),
with `Msg_Text` (`0x0086bf30`). `Native_MsgSetPos` moves three windows on
three maps (map `0x17` at (0x1e6, 0x104) to (0x1c7, 0xc7); map `0xab` at
(0x100, 0x14f) to y 0x140 when `0x008ac2f0()` is 2 or 3; map `0xd4` at
(0x16, 0xdc) to y 0x82 when it is 4). `Msg_CloseWindow` (the menu
module's) passes `TOCloseMessageWindow` the window number unclamped, so a
number above 7 closes past the windows when the window is open. The voice
waits share a shape: in debug mode the first call prints a start line and
every call counts; with the flag (`0x01325ae4`/`0x01325aec`) set, a
countdown (`0x01325ae0`/`0x01325ae8`) decides, else `SndGetStreamState`;
when done, debug mode prints the count. The dump gives both one byte, so
they run in Rust only through the dispatch.

`ffx_script::vm::bridge::native_move` has 42 handlers of the move and
turn blocks, workers, character motion, map jumps and fades. The starts
reproduce the x87 sequences exactly: `Native_MoveStart` mode 9 sets the
jump arc's `+0x34`/`+0x18` from the height to go (`d = +0x2c - +0x20`,
`t = ±+0x18`, `u = 2(d - t)`, and for `u < 0`: `-u` and `d - u/2`), each
step rounded to `f32` where the original stores; `Native_TurnStart` mode 2
truncates the remaining angle to frames (`0x800` takes 2π off a positive
count, `0x400` adds it to a negative) and sets the rate to
`|speed × 0.8333 / frames|`. The turn speed setters (`0x002e`, `0x002f`,
`0x00e3`) are float handlers whose `ST0` is what their callee leaves (the
speed). `native_msg` now also has `Native_MsgOpen`, the message waits
(`0x007c`, `0x0084`: start, poll, int), the choice window `0x007d`
(`Native_MsgSelect`, its poll and result, which `Native_MsgShowWait`
`0x013b` shares) and `Msg_ShowSetup` (`0x008607a0`), which pops seven
arguments and re-pushes them for `MsgSetPos`, `MsgSetText`, `MsgOpen` and
`MsgSelect` in turn.

`ffx_script::vm::bridge::native_btl` has 38 battle natives (group 7):
actor properties through `MsGetChrStatInfo`/`MsSetChrStatInfo`, commands,
targets, motions and counts on the worker's battle actor (its index in the
battle id byte, `0xff` for none). Where the original passes the address of
a stack local for a result (`&out`), the Rust version uses
`NativeHost::scratch`, zeroed first; a callee that writes nothing leaves 0
there instead of the original's stack leftovers. Calls with more than six
arguments (`0x007acd40`, `0x00797bc0`, `0x007bf600`) go through
`NativeHost::call_n` (12 words in the game); round 7's `Native_Cam_601d`
called the six-argument `Host::call` with seven, which would have
panicked in the game. Round 8 also added the 0x0267 choice poll
(`Msg_ChoicePoll` `0x00860990`, which calls a handler for status 1),
`Native_VoiceStandby` and `Native_VoiceStart` with their polls and ends,
`Native_SpriteSetPos`, `Native_LightSet` and `Native_SetModel`. Round 9 added 44 more battle natives
(83 handlers in `native_btl`), `Native_0006` (a model attached to the
worker's character in a slot), `0x00f5`, `0x00cb` and the menu open
`CT_401D_init` with its poll and result.
Round 10 added the larger battle natives (`0x702c` the actor's jump to
its target, `0x70b0`, `0x7028`, the battle-flag pair `0x707d`/`0x707e`,
`0x700d`, `0x7032`, `0x7052`; 91 handlers in `native_btl`) and 34 more
group-0 natives: the entrance getters, the move and turn block setters
(`0x006d`..`0x0071`, `0x00e4`, `0x002b`, `0x002c`, `0x0021`), the
position-block setters (`0x0062`, `0x0063`, `0x0085`, `0x008d`),
`Native_SetTriggerMode`, `Native_SetPlayer`, `Native_MusicPlay` and smaller
forwarders. `0x702c` and `0x700d` hand `0x007a87f0` two stack vectors it
reads before writing; the oracle test zeroes them in both runs.
Round 11 finished group 0: 325 more handlers in `native_move` (411 in
all), from the trigger setters (`0x0054` line, `0x0057` point, `0x007a`
line extension), the party natives (`0x0042`, `0x00ca`, `0x00ec`,
`0x01a6`), the camera attach/detach pair (`0x0002`, `Worker_Detach`
`0x019d`), the music natives (`0x0188` fade-out, `0x0193`/`0x01a0`, the
loads) and the reward windows (`0x015b`, `0x0216`, `0x01a7`) to the
choice-text builders (`0x0237`, `0x0140`, `0x01f3`) and the many small
forwarders and jumps. 55 of them are too small for the hook's jump or
missing from the dump (`ret`, `mov eax, 1; ret`, bare `jmp`s); they have
a Rust handler but no shim. Seven more (`0x00856980`, `0x008571c0`,
`0x008580a0`, `0x008594a0`, `0x008595f0`, `0x0085c4b0`, `0x0085ece0`)
keep the auto-port's shim and name on main. Starts that are a bare `ret`
(`Native_MusicWaitLoaded`, `Native_00dc`) leave `EAX` as it was, which
the dispatch ignores.

Round 12 finished group 7 (50 more handlers in `native_btl`: the voice
and motion starts `0x706e`/`0x704b`/`0x7050`, whose polls are
`0x007a8700`; the picked-actor getters and setters through
`Battle_FirstSelectedActor`; `0x7107`, `0x7023`, `0x70be`, `0x70c3`
with their `f32` steps) and added `native_chr` for the other groups the
shipped scripts call: math (group 1), `Sg_*` scene effects (4), `Ch_*`
model natives (5), the rest of the camera and screen natives (6), the
movie player (11) and group 12's system natives. Natives that build an
out local with `push ecx` (`0x70be`'s `MsGetChrDir` writes nothing
without a model) read the caller's `ECX` there; the ports zero it, and
the oracle test calls the originals with `ECX` = 0. Round 13 ported
the last four: `0x1005` calls the game's own `_CIatan2` through its
import thunk (`0x009498bc`, `NativeHost::call_intrinsic2`, `y` in `ST1`
and `x` in `ST0`), so it is bit-exact with the CRT; `0x6077` reads the
camera's `+0xc90` (`0x007bd500`, called with no arguments); `0x5037`
and `0xc025`. `0xc03b` and `0xc03c` turned out to be the debug cheats
`MsDebugFullItem`/`MsDebugFullCommand` and are marked instead. The
debug ones (`0x70b1`, `0xb002`, `0xc00b`, `0xc00c`, `0xc018`, `0xc022`,
`0xc028`, `0xc02a`, `0xc03b`, `0xc03c`, `0xc056`, `0xc057`, `0xc05b`)
are marked in the table. `0x5065` stops in most oracle calls (all on
the field snapshot): its `Ch_RomReadEventMGRP` asks the file system for
the group's size first (`*0x02310c54`, whatever the input), which
hashes the name through `ADVAPI32!CryptCreateHash`, an import the
emulator does not stub; on the battle snapshot some calls get past it.

`ffx_script::vm::bridge::helpers` (round 13) has VM helpers that are
not natives: `Script_StopTurn` (`0x00863fe0`, the turn-block twin of
`Script_StopMove`), the script block's header tables (`0x0086ac60`,
`0x0086ac30`, `0x0086ac90`: `+0x2c`, then `+0xc`, offsets relative to
the block), `AtelGetSnd3dWork`, `AtelSoundTableGet_00876860`, the
message-window status of the current bank (`0x0086be60`, `0x0086c8b0`),
`0x0086c530`, `0x0086f780`, `0x008712a0` and `AtelAllClearBindMask`.

Debug natives and debug-only code are not ported. In this build
`dbgPrintf` (`0x0062f500`) is a bare `ret` and `TkIsDebug` (`0x00887d10`)
returns 0, so leaving them out changes nothing: the round-11 ports drop
the `dbgPrintf` lines, `0x01b0` drops its C-runtime `printf`, and
`0x00f9` is ported as it is (`TkIsDebug`, then 1 in this build).
`0x00f8`, whose only work happens under `TkIsDebug`, is listed in the
table as "debug (mod-tool candidate)" without a Rust handler; the
modders' API may offer such natives later.

#### The ported natives

Every native with a Rust handler, by ID: the arguments in push order
(the last pushed is popped first; `int`, `f` a float, `var` a variable
reference), the result (`EAX` for an integer handler, `ST0` for a float
one; a start/poll pair's poll returns 5, or for some 1, when done and 0 to
be called again next frame) and what a poll waits for. The modders' Lua
API is generated from this table. `-`: none. A result starting with
"debug (mod-tool candidate)" marks a debug native that is not ported
("not ported" in the handler column); the line says what it does.

| ID | Name | Arguments (push order) | Result | Waits | Handlers (module) |
|---|---|---|---|---|---|
| `0x0000` | `Native_WaitFrames_0000` | frames int (10 becomes 40 in battle scene 0x610000 with magic 0x290) | 1 when done | the frames (counted by real frames when g_WaitFramesByTime is set) | start `0x0085c4e0`, poll `0x0085c6b0` (native_move) |
| `0x0001` | `Native_SetModel` | model int | 0 (the worker's character allocated and placed) | - | int `0x0085d000` (native_move) |
| `0x0002` | `Native_AttachCamera` | camera int, mode int, _ int | 0: attaches the worker to camera (+0xaa 2, 0x007ba9a0) after detaching it | - | int `0x0085e920` (native_move) |
| `0x0003` | `Native_0003` | v int: +0xa8 (+0xaa = 3) | 0 | - | int `0x0085ebd0` (native_move) |
| `0x0004` | `Native_0004` | v int: worker +0xa8 (+0xaa = 4) | 0 | - | int `0x0085ecb0` (native_move) |
| `0x0005` | `Native_WorkerDetach` | pushes 0, then as Worker_Detach (0x019d) | Worker_Detach's result | - | int `0x0085cdc0` (native_move) |
| `0x0006` | `Native_0006` | slot int, model int, motion int | 1 (model attached in the slot), 0 without a character or model | - | int `0x0085e620` (native_move) |
| `0x0007` | `Native_0007` | slot int: 0x00862780(worker, slot) | 1 | - | int `0x0085c8c0` (native_move) |
| `0x0010` | `Native_0010` | as 0x0086c1d0 | as 0x0086c1d0 (a jump) | - | int `0x00856910` (native_move) |
| `0x0011` | `Native_MapChange` | map int, entrance int: map change | 0 | - | int `0x008581c0` (native_move) |
| `0x0012` | `Native_0012` | x f, z f: 0x00870970(worker, x, z, 0) | 0 | - | int `0x0085f800` (native_move) |
| `0x0013` | `CT_0013_setPosition` | x f, y f, z f: the worker's position | 0 | - | int `0x0085faa0` (native) |
| `0x0015` | `Native_MoveSetTarget` | x f, y f, z f: move target | 0 | - | int `0x008602f0` (native_move) |
| `0x0016` | `Native_MoveSetSpeed` | speed f: move speed | float: speed | - | float `0x00857da0` (native_move) |
| `0x0017` | `Native_SetArriveRadius` | r f: arrive radius of the current level | float: r | - | float `0x0085f040` (native_move) |
| `0x0018` | `Native_MoveStart` | a int, mode int (low 7 bits: 5/6 timed, 7 from here, 8, 9 jump arc; 0x800 added when the block's +0x38 is 0), b int | 0 | no (MoveWait 0x001a waits) | int `0x00856c30` (native_move) |
| `0x0019` | `Native_TurnStart` | a int, mode int (low 5 bits: 2 timed turn, 0x800/0x400 wrap the turn), b int | 0 | no (TurnWait 0x001b waits) | int `0x00857520` (native_move) |
| `0x001a` | `Native_MoveWait` | - | 5 when done | until the move block's mode (+2 & 0x7f) is 0 (the move started by MoveStart ends) | poll `0x008579a0` (native) |
| `0x001b` | `Native_TurnWait` | - | 5 when done | until the turn block's mode (+2 & 0x1f) is 0 | poll `0x00857b30` (native) |
| `0x001c` | `Native_MoveSetFrames` | elapsed int, frames int: the move block's timer (+0xc, +8) | float: elapsed | - | float `0x00855f10` (native) |
| `0x001d` | `Native_TurnSetFrames` | elapsed int, frames int: the turn block's timer (frames <= 0 become 1) | float: elapsed | - | float `0x008560d0` (native) |
| `0x001f` | `Native_GetYawToMoveTarget` | - | float: yaw from the worker to the move target | - | float `0x008587a0` (native_move) |
| `0x0020` | `Native_GetPitchToMoveTarget` | - | float: the pitch to the move target (second result of 0x0088b4d0) | - | float `0x00858d30` (native_move) |
| `0x0021` | `Native_0021` | yaw f (radians) | float: the wrapped yaw (move block +0x44) | - | float `0x00859430` (native_move) |
| `0x0025` | `Native_0025` | yaw f (radians) | float: the wrapped yaw (position +0x34) | - | float `0x00859d80` (native_move) |
| `0x0028` | `Native_TurnSetYaw` | yaw f (radians) | float: the wrapped yaw (turn target +0x20, remaining +0x2c) | - | float `0x00859640` (native) |
| `0x0029` | `Native_TurnSetPitch` | angle f (radians, pitch) | float: the wrapped angle (+0x24, remaining +0x30) | - | float `0x00859880` (native) |
| `0x002a` | `Native_002a` | pitch f (radians) | float: the wrapped pitch (turn block +0x28; the rest to the position's pitch in +0x34) | - | float `0x00859bb0` (native_move) |
| `0x002b` | `Native_002b` | v f | float: 0x00871520(worker, level, v) | - | float `0x0085a0b0` (native_move) |
| `0x002c` | `Native_002c` | v f | float: 0x00871500(worker, level, v) | - | float `0x0085a1a0` (native_move) |
| `0x002d` | `Native_SetArriveRadiusAll` | r f: arrive radius of every level | float: r | - | float `0x00858020` (native_move) |
| `0x002e` | `Native_TurnSetSpeed` | speed f: turn block +0x10 | float: speed | - | float `0x0085a2a0` (native_move) |
| `0x002f` | `Native_TurnSetSpeed2` | speed f: turn block +0x18 | float: speed | - | float `0x0085a5c0` (native_move) |
| `0x0030` | `Native_0030` | v f | float: 0x00871540(worker, level, v) | - | float `0x0085a7c0` (native_move) |
| `0x0033` | `Native_GetWorkerId` | worker int (-1 = self) | int: its id (+0x2e) | - | int `0x0085ba50` (native) |
| `0x0034` | `Native_EnableTrigger` | kind int: trigger bit (0 talk) | int: kind (worker +0xab gets the bit) | - | int `0x0085c060` (native) |
| `0x0035` | `Native_DisableTrigger` | kind int | 0 (clears worker +0xab's bit) | - | int `0x0085c280` (native) |
| `0x0036` | `Native_0036` | worker int | 0 (each level reset, 0x00863e10) | - | int `0x0085b4e0` (native_move) |
| `0x0037` | `Native_0037` | worker int | 0: 0x00863fe0 on its nine levels | - | int `0x0085b780` (native_move) |
| `0x0038` | `Native_GetWorkerX` | worker int | float: its x | - | float `0x0085bb80` (native) |
| `0x0039` | `Native_GetWorkerY` | worker int | float: its y | - | float `0x0085bd40` (native) |
| `0x003a` | `Native_GetWorkerZ` | worker int | float: its z | - | float `0x0085bf10` (native) |
| `0x003d` | `Native_003d` | - | 0 (0x0086db80(worker, 0xff)) | - | int `0x0085c3e0` (native_move) |
| `0x003f` | `Native_TurnGetField38` | - | float: the turn block's +0x38 | - | float `0x0085c590` (native) |
| `0x0042` | `Native_SetPartyIndex` | i int: party index (bound character for type-1 workers; slot, 0xf from 8) | int: i | - | int `0x0085cb40` (native_move) |
| `0x0043` | `Native_SetPlayer` | - | int: the worker id (the player is set to it, 0x0086f6a0) | - | int `0x0085c9a0` (native_move) |
| `0x0044` | `Native_0044` | - | int: the flag word 0x00888c40(0, 0, 0x0086af30()) | - | int `0x0085d4e0` (native_move) |
| `0x0046` | `Native_PadPressed` | - | int: buttons pressed this frame, remapped | - | int `0x0085d9b0` (native) |
| `0x0047` | `Native_0047` | - | int: the flag word 0x00888c40(0, 0, 0x0086af80()) | - | int `0x0085db70` (native_move) |
| `0x004c` | `Native_004c` | i int | int: bit i of the flag word for 0x0086af30() | - | int `0x0085e130` (native_move) |
| `0x004d` | `Native_004d` | i int | int: bit i of the flag word for 0x0086af70() (0 for worker 11 on story 0x31 with [0x0112d6c9] bit 5) | - | int `0x0085e820` (native_move) |
| `0x0050` | `Native_0050` | i int | int: bit i of 0x0086af30() | - | int `0x0085ec60` (native_move) |
| `0x0051` | `Native_0051` | i int | int: bit i of 0x0086af70() | - | int `0x0085f0e0` (native_move) |
| `0x0054` | `Native_TriggerSetLine` | x0 f, y0 f, z0 f, x1 f, y1 f, z1 f: the line ends (+0, +0x30) | int: 1 for line triggers (types 2, 5), else 0 | - | int `0x0085f370` (native_move) |
| `0x0055` | `Native_0055` | r f: position +0x1c | int: 1 for triggers (types 2, 3, 5, 6), else 0 | - | int `0x0085fa20` (native_move) |
| `0x0056` | `Native_TriggerSetActive` | on int | 1 for trigger workers (types 2, 3, 5, 6: +0x34 bit 7 = on), else 0 | - | int `0x0085fbf0` (native) |
| `0x0057` | `Native_TriggerSetPoint` | x f, y f, z f: the trigger point | int: 1 for point triggers (types 3, 6), else 0 | - | int `0x008603e0` (native_move) |
| `0x0058` | `Native_0058` | as 0x0059, after the float at 0x00b92340 is pushed | as 0x0059 (0x00855cd0) | - | int `0x00855f80` (native_move) |
| `0x0059` | `Native_0059` | z1 f, h f, r f: position +0x30, +0x38, +0x1c | int: 1 for point triggers (types 3, 6), else 0 | - | int `0x00855cd0` (native_move) |
| `0x005a` | `Native_005a` | yaw f: position +0x34 of point triggers (types 3, 6) | 0 | - | int `0x00856170` (native_move) |
| `0x005c` | `Native_TriggerSetActive2` | on int | as 0x0056 | - | int `0x008600c0` (native) |
| `0x005d` | `Native_ControlEnable` | - | 1 (player control on) | - | int `0x008566b0` (native) |
| `0x005e` | `Native_ControlDisable` | - | 0 (player control off) | - | int `0x008567c0` (native) |
| `0x005f` | `Native_WaitForever` | - | 0 | forever (until the entry is interrupted) | poll `0x0085c420` (native) |
| `0x0060` | `CT_0060_ret` | v int | int: 0x008712a0(worker, v) | - | int `0x00856920` (native_move) |
| `0x0061` | `Native_SetTalkRange` | r f: talk range (truncated) | float: the stored range | - | float `0x00857180` (native_move) |
| `0x0062` | `Native_SetRadius44` | r f (truncated): pos +0x44 | float: the stored radius | - | float `0x008572e0` (native_move) |
| `0x0063` | `Native_SetTalkAngle` | angle f (radians): talk angle (pos +0x3c) | float: the angle | - | float `0x008574f0` (native_move) |
| `0x0064` | `Native_MsgSetText` | window int, msg int: message index in the event's table | 0 (window text and flags set) | - | int `0x00857870` (native_msg) |
| `0x0065` | `Native_MsgSetPos` | window int, x int, y int, anchor int | 0 (three windows of maps 0x17, 0xab, 0xd4 are moved) | - | int `0x008580c0` (native_msg) |
| `0x0066` | `Native_MsgSetStyle` | window int, style int | int: window | - | int `0x00858360` (native) |
| `0x0069` | `Native_0069` | window int, b int, a int: +0x1e = b, +0x1f = a (bytes) | int: window | - | int `0x008589d0` (native_move) |
| `0x006a` | `Native_MsgOpen` | window int, mode int (0, 2, 66: choice flags) | int: window (opened, state 1) | - | int `0x00858b50` (native_msg) |
| `0x006b` | `Native_MsgClose` | window int | int: window (closes it) | - | int `0x00859060` (native_msg) |
| `0x006c` | `Native_SetMoveSpeed` | speed f: worker move speed | float: speed | - | float `0x00857f40` (native_move) |
| `0x006d` | `Native_SetMoveParam3C` | a f: move block +0x3c of all levels | float: a | - | float `0x0085ab80` (native_move) |
| `0x006e` | `Native_SetMoveParam40` | a f: move block +0x40 of all levels | float: a | - | float `0x0085acd0` (native_move) |
| `0x006f` | `Native_SetTurnParam10` | a f: turn block +0x10 of all levels | float: a | - | float `0x0085af20` (native_move) |
| `0x0070` | `Native_SetTurnParam18` | a f: turn block +0x18 of all levels | float: a | - | float `0x0085b230` (native_move) |
| `0x0071` | `Native_SetTurnParam1C` | a f: turn block +0x1c of all levels | float: a | - | float `0x0085b480` (native_move) |
| `0x0074` | `Native_SetMoveMOT` | v int: move motion | int: v | - | int `0x0085bdc0` (native_move) |
| `0x0076` | `Native_GetOwnWorkerType` | - | int: the worker's type (0x0086a7b0) | - | int `0x0085f5a0` (native_move) |
| `0x0077` | `Native_WorkerStopMove` | worker int | 0 (stops its move) | - | int `0x0085b620` (native) |
| `0x0078` | `Native_WorkerStopTurn` | worker int | 0 (stops its turn) | - | int `0x0085b950` (native) |
| `0x007a` | `Native_LineExtend` | back f, front f: how far the line ends move out | int: 1 for line triggers (types 2, 5), else 0 | - | int `0x0085c130` (native_move) |
| `0x007c` | `Native_MsgWait_End` | window int | 1 when done; int slot 0 (closes it) | until the window is closed or in state 4 (open with its text shown and the ctrl work's bit 2: closed then) | start `0x0085b7c0`, poll `0x0085b980`, int `0x0085ba80` (native_msg) |
| `0x007d` | `Native_MsgSelect` | window int[, a int[, b int]]: choice window | 1 when done; int slot: the choice (-1 none) | until the window is closed or in state 4, or open with its input done (closed then) | start `0x00859710`, poll `0x00859c50`, int `0x0085a6f0` (native_msg) |
| `0x007f` | `Native_SetTriggerMode` | mode int: trigger mode (bits 13-14 of +0x34) | int: 1 for trigger workers (types 2, 3, 5, 6), else 0 | - | int `0x0085c320` (native_move) |
| `0x0080` | `Native_GetEntranceX` | - | float: the entrance x (+0xc) | - | float `0x0085c7b0` (native_move) |
| `0x0081` | `Native_GetEntranceY` | - | float: the entrance y (+0x10) | - | float `0x0085c8f0` (native_move) |
| `0x0082` | `Native_GetEntranceZ` | - | float: entrance z | - | float `0x0085cb20` (native_move) |
| `0x0083` | `Native_GetEntranceYaw` | - | float: entrance yaw | - | float `0x0085cce0` (native_move) |
| `0x0084` | `Native_MsgWaitEx_End` | window int, x int | 1 when done; int slot 0 (closes it) | as 0x007c | start `0x0085abd0`, poll `0x0085ae90`, int `0x0085b6d0` (native_msg) |
| `0x0085` | `Native_SetTouchRange` | r f: touch range (pos +0x48) | 1 | - | int `0x00856460` (native_move) |
| `0x0086` | `Native_0086` | as 0x0086c1f0 | as 0x0086c1f0 (a jump) | - | int `0x0085cdb0` (native_move) |
| `0x0087` | `Native_0087` | as 0x0086c1c0 | as 0x0086c1c0 (a jump) | - | int `0x0085cf90` (native_move) |
| `0x0088` | `Native_0088` | as 0x0086bca0 | as 0x0086bca0 (a jump) | - | int `0x0085d160` (native_move) |
| `0x0089` | `Native_0089` | as 0x0086bc70 | as 0x0086bc70 (a jump) | - | int `0x0085d280` (native_move) |
| `0x008b` | `Native_LineGetCrossSide` | - | int: side of the crossed line (0x008632b0), else from worker +0x36 (-1 for 3) | - | int `0x0085d410` (native_move) |
| `0x008d` | `Native_SetTYZAngle` | angle f: pos +0x4c | float: the angle | - | float `0x008577c0` (native_move) |
| `0x008f` | `Native_MsgIsReady` | window int | 1 when closed, in state 4, or open with its text done; else 0 | - | int `0x0085de10` (native) |
| `0x0090` | `Native_0090` | - | float: the position's +0x20 | - | float `0x0085e270` (native_move) |
| `0x0091` | `Native_0091` | - | float: the position's yaw (+0x34) | - | float `0x0085e0b0` (native_move) |
| `0x0092` | `Native_0092` | worker int | float: its position's +0x20 | - | float `0x0085eac0` (native_move) |
| `0x0093` | `Native_0093` | worker int | float: its yaw (position +0x34) | - | float `0x0085e8f0` (native_move) |
| `0x0094` | `Native_0094` | v f: 0x0086f040(worker, v) | float: v | - | float `0x0085f0b0` (native_move) |
| `0x0095` | `Native_SetYaw` | yaw f | float: the wrapped yaw (turn stopped, character turned) | - | float `0x0085f170` (native_move) |
| `0x0096` | `Native_SetPitch` | pitch f (radians) | float: the wrapped pitch (position +0x24, +0x30) | - | float `0x0085f460` (native_move) |
| `0x0097` | `Native_MsgGetState` | window int | int: state (0 closed, 1 opening, 2 open, 3 closing, 4) | - | int `0x0085fa80` (native) |
| `0x0098` | `Native_0098` | var int, n int: 0x00870fc0(worker, &var), 0x00871610(worker, n), 0x00871630(worker, 0) | 1 | - | int `0x0085fb50` (native_move) |
| `0x009a` | `Native_009a` | v int: position +0x28 for types 5, 6 | int: v | - | int `0x00860390` (native_move) |
| `0x009b` | `Native_009b` | b var, a var: two vectors pushed as the line for 0x0054 | as 0x0054 | - | int `0x0085f750` (native_move) |
| `0x009d` | `Native_MsgSetFlags` | window int, flags int | int: window | - | int `0x008584d0` (native) |
| `0x009e` | `Native_MsgSetValue` | window int, slot int, type int, value int: inline text variable | int: window | - | int `0x00855c70` (native_msg) |
| `0x00a2` | `Native_00a2` | window int, d int, c int, b int, a int: bytes | int: the colour word a << 24 | d << 16 | c << 8 | b (window +0x10) | - | int `0x00856570` (native_move) |
| `0x00a3` | `Native_GetWorkerPos` | worker int, x var, y var, z var | 0 (its position stored) | - | int `0x00856e20` (native_move) |
| `0x00a4` | `Native_00a4` | b int, a int: workers | float: the distance between them | - | float `0x008566c0` (native_move) |
| `0x00a5` | `Native_00a5` | b int, a int: workers | float: the x-z distance between them | - | float `0x00856990` (native_move) |
| `0x00a6` | `Native_Random` | n int | int: random 0..n-1 | - | int `0x00857400` (native) |
| `0x00a7` | `Native_00a7` | worker int: its +0x34 bit 1 cleared | 0 | - | int `0x00857820` (native_move) |
| `0x00a8` | `Native_00a8` | worker int: its +0x34 bit 1 set | 1 | - | int `0x00857950` (native_move) |
| `0x00a9` | `Native_Random16` | - | int: a random 16-bit number (0x007988f0(2)) | - | int `0x00857680` (native_move) |
| `0x00aa` | `Native_00aa` | v int: 0x0086f3d0(v) | int: v | - | int `0x00857a60` (native_move) |
| `0x00ab` | `Native_00ab` | map int, entrance int: AtelSetEventJump2(map, entrance, 1) | 0 | - | int `0x00858330` (native_move) |
| `0x00ac` | `Native_00ac` | v int: 0x00870ed0(v) | int: v | - | int `0x00858050` (native_move) |
| `0x00b1` | `Native_00b1` | v f: move block +0x18 | float: v | - | float `0x00856a50` (native_move) |
| `0x00b2` | `Native_00b2` | on int | int: 0x0083add0(on != 0) | - | int `0x008586b0` (native_move) |
| `0x00b3` | `Native_00b3` | v int: the first word of 0x0083adf0's set (0x0083b110) | int: v | - | int `0x00858810` (native_move) |
| `0x00b4` | `Native_00b4` | v int: the second word of 0x0083adf0's set (0x0083b110) | int: v | - | int `0x00858a20` (native_move) |
| `0x00ba` | `SetFieldConst44_008594a0` | - | 1: the worker's +0x44 = 0 | - | int `0x008594a0` (native_move) |
| `0x00bb` | `Native_SetMapFadeFrames` | frames int: fade length for the next map jump | int: frames | - | int `0x008596b0` (native) |
| `0x00bc` | `Native_SetMapFadeType` | type int: fade kind for the next map jump (0, 1) | int: type | - | int `0x00859590` (native) |
| `0x00bd` | `Native_00bd` | n int (at least 1): 0x00871730(worker id, n - 1) | int: n | - | int `0x00859840` (native_move) |
| `0x00be` | `Native_00be` | on int: the ctrl work's bit 7 | int: on != 0 | - | int `0x00859950` (native_move) |
| `0x00bf` | `Native_00bf` | i int | float: +0xc of 0x0086c020(worker, i) | - | float `0x00859d30` (native_move) |
| `0x00c0` | `Native_00c0` | i int | float: +0x10 of 0x0086c020(worker, i) | - | float `0x00859e00` (native_move) |
| `0x00c1` | `Native_00c1` | i int | float: +0x14 of 0x0086c020(worker, i) | - | float `0x00859ef0` (native_move) |
| `0x00c2` | `Native_00c2` | i int | float: +8 of 0x0086c020(worker, i) | - | float `0x0085a070` (native_move) |
| `0x00c4` | `Native_00c4` | - | int: 1 while the move block runs (mode +2 & 0x7f) | - | int `0x0085a340` (native_move) |
| `0x00c5` | `Native_SetTurnMotions` | m0 int, m1 int, m2 int, m3 int: turn motions | 0 | - | int `0x0085a4e0` (native) |
| `0x00c6` | `Native_GetTurnKind` | - | int: bit 0 remaining yaw > 0, bit 1 at least the half-turn angle | - | int `0x0085a670` (native) |
| `0x00c7` | `Native_GetTurnMotion` | kind int (from 0x00c6) | int: the motion 0x00c5 set for it | - | int `0x0085abb0` (native) |
| `0x00c8` | `Native_GetTurnFrames` | mode int, frames2 int, frames int | int: the frames left of the turn: the remaining angle over pi (mode bit 1, eased) or [0x00b11338], clamped to 0..1, times frames (bit 1) or frames2 | - | int `0x0085ad00` (native_move) |
| `0x00c9` | `Native_00c9` | i int | float: the signed word +6 of 0x0086c020(worker, i) | - | float `0x0085b560` (native_move) |
| `0x00ca` | `Native_PartyAdd` | chr int: joins the party (0x007869b0(chr, 1)) | int: chr | - | int `0x0085b700` (native_move) |
| `0x00cb` | `Native_00cb` | chr int | int: chr (removed from the party slots) | - | int `0x0085b820` (native_move) |
| `0x00cc` | `Native_00cc` | chr int | int: 1 when 0x00785380(chr) is 1 | - | int `0x0085b9f0` (native_move) |
| `0x00ce` | `Native_00ce` | v int: worker +0xb16 | int: v | - | int `0x0085c2b0` (native_move) |
| `0x00cf` | `Native_00cf` | v int: worker +0xaf0 | int: v | - | int `0x0085c400` (native_move) |
| `0x00d0` | `GetFieldaf0_0085c4b0` | - | int: worker +0xaf0 | - | int `0x0085c4b0` (native_move) |
| `0x00d1` | `CT_01D1_ret` | - | 0: 0x00863e10 on the worker's nine levels | - | int `0x0085c5b0` (native_move) |
| `0x00d2` | `Native_00d2` | - | 0: 0x00863fe0 on the worker's nine levels | - | int `0x0085c6f0` (native_move) |
| `0x00d5` | `Native_VoiceStandby_End` | voice int | the standby frames; int slot 0 (Voice_Prepare unless a voice is on standby) | until the standby countdown runs out, or no movie runs (and g_VoiceHold allows) | start `0x0085cd00`, poll `0x0085cec0`, int `0x0085d360` (native_msg) |
| `0x00d6` | `Native_VoiceStart_End` | - | the pending frames; int slot 0 (volume, play flag, sync countdown) | until the start countdown runs out, or the stream is ready and the frames counted down | start `0x0085d6b0`, poll `0x0085d9d0`, int `0x0085de80` (native_msg) |
| `0x00d7` | `Native_VoiceStop` | - | 0 (stops the voice) | - | int `0x0085e090` (native) |
| `0x00d8` | `Native_VoiceIsPlaying` | - | 1 while the voice plays, else 0 | no (scripts loop on it) | int `0x0085e1d0` (native_msg) |
| `0x00d9` | `Native_SoundWait_Poll` | - | 1 when done | until the voice has stopped (or the voice-sync countdown runs out) | poll `0x0085e4a0` (native_msg) |
| `0x00da` | `Native_00da` | as 0x0086a9b0 | as 0x0086a9b0 (a jump) | - | int `0x00857c30` (native_move) |
| `0x00db` | `Native_00db` | v int | 5 when done | until 0x00788bd0() is 3 (or 1 on map 0x10e), calling 0x00872470(v) each frame | start `0x0085e9e0`, poll `0x0085eaf0` (native_move) |
| `0x00dc` | `Native_00dc` | - | done when 0x008633e0(worker id) has bit 0 | until 0x008633e0(worker id) bit 0 | start `0x0085ee00`, poll `0x0085ee70` (native_move) |
| `0x00de` | `Native_00de_Poll` | - | 0x00863350(worker id) | until 0x00863350(worker id) (poll only) | poll `0x0085f280` (native_move) |
| `0x00df` | `Native_SfxPlay` | sfx int | int: sfx | - | int `0x0085f310` (native) |
| `0x00e0` | `Native_00e0` | v int: 0x00886eb0(v) | int: v | - | int `0x00855f60` (native_move) |
| `0x00e1` | `Native_00e1` | as 0x0086a9d0 | as 0x0086a9d0 (a jump) | - | int `0x00857fb0` (native_move) |
| `0x00e2` | `Native_00e2` | v int: 0x0086f430(v) | int: v | - | int `0x00857dd0` (native_move) |
| `0x00e3` | `Native_SetTurnSpeed` | speed f: turn block +0x14 | float: speed | - | float `0x0085a480` (native_move) |
| `0x00e4` | `Native_SetTurnSpeedAll` | speed f: the turn speed of all 9 slots (0x0086f300) | float: speed | - | float `0x0085b030` (native_move) |
| `0x00e6` | `Native_00e6` | chr int | int: 1 when chr is in a party slot | - | int `0x0085bf90` (native_move) |
| `0x00e7` | `Native_PartySwapIn` | party b int, a int (-1 = 0xff) | int: a | - | int `0x0085bdf0` (native_move) |
| `0x00e8` | `Native_00e8` | c var, b var, a var: the three party slots (-1 for empty) | 1 | - | int `0x00856270` (native_move) |
| `0x00e9` | `Native_00e9` | on int: the ctrl work's +1 bit 4 | int: on != 0 | - | int `0x00856660` (native_move) |
| `0x00eb` | `GetFieldU832_00856980` | - | int: the worker's level (+0x32) | - | int `0x00856980` (native_move) |
| `0x00ec` | `Native_00ec` | chr int | int: its party slot 0..2, -1 when absent | - | int `0x00856bc0` (native_move) |
| `0x00ed` | `GetField40_008571c0` | - | int: the worker's bound character (+0x40) | - | int `0x008571c0` (native_move) |
| `0x00ee` | `Native_00ee` | worker int | int: its +0x34 bit 5 | - | int `0x008572b0` (native_move) |
| `0x00ef` | `Native_00ef` | on int: +0x36 bit 11 | int: on != 0 | - | int `0x00857470` (native_move) |
| `0x00f2` | `Native_00f2` | v int | int: 0x00870190(v) | - | int `0x00857850` (native_move) |
| `0x00f3` | `Native_MinimapSetCorner` | corner int: minimap corner (0x00919e20) | int: corner | - | int `0x00857c40` (native_move) |
| `0x00f4` | `Native_AngleToPoint` | x f, y f (ignored), z f | float: yaw from the worker to (x, z) | - | float `0x00858970` (native_move) |
| `0x00f5` | `Native_00f5` | x f, y f, z f | float: 0x0088b4d0's second result from the worker's position | - | float `0x00858f40` (native_move) |
| `0x00f6` | `Native_00f6` | n int (at least 1), worker int | 5 when done | until 0x00863520(worker, n) is 0 (at once for a negative worker) | start `0x00857df0`, poll `0x00857ff0` (native_move) |
| `0x00f8` | `Native_00f8` | var int, value int, count int | debug (mod-tool candidate): in a debug build (TkIsDebug) fills the variable with memset(&var, value, count); the release build only pops the three; returns count | - | int `0x00858520` (not ported) |
| `0x00f9` | `Native_00f9` | - | 1 (TkGetEventNewGame in a debug build) | - | int `0x00858780` (native_move) |
| `0x00fa` | `Native_00fa` | v int: 0x0088eb40(v) | int: v | - | int `0x00858920` (native_move) |
| `0x00fb` | `Native_00fb` | window int | int: 0x008b8010(window) | - | int `0x00858ac0` (native_move) |
| `0x00fd` | `Native_SfxPlayEx` | sfx int, pan int, volume int (0-127) | int: sfx | - | int `0x0085ff30` (native) |
| `0x00fe` | `Native_00fe` | b int, a int: 0x00886f60(b, a) | int: a | - | int `0x00860090` (native_move) |
| `0x0100` | `Native_0100` | c int, b int, a int: 0x00886ff0(c, b, a) | int: a | - | int `0x00860460` (native_move) |
| `0x0102` | `Native_MusicLoad` | track int: music to load | int: the track (0x00886960) | until [0x012fb808] is 1 or less | start `0x00858e50`, poll `0x00859040`, int `0x00859250` (native_move) |
| `0x0103` | `Native_0103` | track int: 0x00886800 (the current music cleared when it is track) | 1 | - | int `0x008593a0` (native_move) |
| `0x0104` | `Native_MusicPlay` | track int: music to play | 1 | - | int `0x008596d0` (native_move) |
| `0x0105` | `Native_MusicWaitLoaded` | - | 1 | nothing (done at once) | start `0x00859d70`, poll `0x00859df0`, int `0x00859ee0` (native_move) |
| `0x0106` | `Native_0106` | i int, b int, var int: record 0x008773f0(i) +0 = b, +4 = 0x00a44700(&var) | 1 | - | int `0x0085a010` (native_move) |
| `0x0107` | `Native_0107` | i int, v int: record 0x008773f0(i) +8 = v (its object's bit 1 set) | 1 | - | int `0x0085a1f0` (native_move) |
| `0x0108` | `Native_0108` | i int, second int, x f, y f, z f: record 0x008773f0(i) point +0x14 (second) or +8 | 1 | - | int `0x0085a370` (native_move) |
| `0x0109` | `Native_0109` | i int: 0x00876930(record, 1) when set | 1 | - | int `0x0085a870` (native_move) |
| `0x010a` | `Native_010a` | i int: 0x00886eb0 and 0x008772f0 on record 0x008773f0(i) | 1 | - | int `0x0085a630` (native_move) |
| `0x010b` | `Native_MapJump` | map int, entrance int: map jump | 0 | - | int `0x00858460` (native_move) |
| `0x010c` | `Native_010c` | map int, entrance int: AtelSetEventJump2(map, entrance, 0) | 0 | - | int `0x00858610` (native_move) |
| `0x010d` | `Native_010d` | i int (up to 0xfe): 0x0088e790(0xa004 + i) | int: 1, else 0 | - | int `0x0085ac90` (native_move) |
| `0x010e` | `Native_010e` | as 0x00906b10 | as 0x00906b10 (a jump) | - | int `0x0085af50` (native_move) |
| `0x010f` | `Native_010f` | window int, c int, b int, a int: +0x22 = c, +0x23 = b (bytes), +0x24 = a | int: window | - | int `0x0085afd0` (native_move) |
| `0x0110` | `Native_0110` | v int | int: 0x008b7c40(v) | - | int `0x0085b260` (native_move) |
| `0x0111` | `Native_0111` | as 0x0086e310 | as 0x0086e310 (a jump) | - | int `0x0085b4b0` (native_move) |
| `0x0112` | `Native_0112` | as 0x0086ddb0 | as 0x0086ddb0 (a jump) | - | int `0x0085b520` (native_move) |
| `0x0114` | `Native_0114` | b int, a int: the ctrl work's +2 bit 2 = b, bit 1 = a | int: b != 0 | - | int `0x0085b670` (native_move) |
| `0x0115` | `Native_0115` | on int: the ctrl work's +2 bit 3 | int: on != 0 | - | int `0x0085b8c0` (native_move) |
| `0x0116` | `Native_WorkerSetFlag0200` | worker int | int: worker (+0x36: 0x400 cleared, 0x200 set) | - | int `0x0085b9b0` (native) |
| `0x0117` | `Native_0117` | worker int: its +0x36 bit 10 cleared, bits 9 and 12 set | int: worker | - | int `0x0085bc50` (native_move) |
| `0x0119` | `Native_0119` | on int (clamped 0..1) | int: 0x0086b150(bound character, on) | - | int `0x0085bd70` (native_move) |
| `0x011a` | `Native_011a` | c var, b int, a var: 0x00874b70(&c, b, &a) | 1 | - | int `0x0085c0a0` (native_move) |
| `0x011b` | `Native_SpriteShow` | sprite int (0-127), data var: variable reference | int: sprite | - | int `0x0085c370` (native) |
| `0x011c` | `Native_SpriteHide` | sprite int | int: sprite | - | int `0x0085c4c0` (native) |
| `0x011d` | `Native_011d` | b int, a int: 0x008749b0(b, a) | int: b | - | int `0x0085c650` (native_move) |
| `0x011f` | `Native_SpriteSetPos` | sprite int, x int, y int | int: sprite | - | int `0x0085cc50` (native_move) |
| `0x0120` | `Native_SpriteSetScale` | sprite int, sx f, sy f (1.0 normal) | int: sprite | - | int `0x0085cde0` (native) |
| `0x0121` | `Native_0121` | i int (0..13) | int: the flag byte 0x00888c00(0, 0, 0x00888cf0(0, 0, i)) | - | int `0x0085d1d0` (native_move) |
| `0x0122` | `Native_0122` | i int (0..13) | int: the flag byte 0x00888c00(0, 0, i) | - | int `0x0085d310` (native_move) |
| `0x0126` | `Native_SetPos_0126` | x f, y f, z f: 0x00870b20(worker, x, y, z, 1) | 0 | - | int `0x0085fea0` (native_move) |
| `0x0127` | `Native_0127` | worker int | as 0x0128 with the own touch range (position +0x48) as the angle | - | int `0x0085db10` (native_move) |
| `0x0128` | `Native_0128` | worker int, angle f | int: 0x0088b730(angle, the offset from that worker) | - | int `0x0085d7b0` (native_move) |
| `0x0129` | `Native_SpriteSetColor` | sprite int, r int, b int, g int | int: sprite | - | int `0x0085c760` (native) |
| `0x012a` | `Native_012a` | b int, a int: 0x00874970(b, a) | int: b | - | int `0x0085c890` (native_move) |
| `0x012b` | `Native_012b` | on int: the ctrl work's +2 bit 6 | int: on != 0 | - | int `0x0085dc50` (native_move) |
| `0x012d` | `Native_012d` | i int (0..7) | int: word i of record 0x0088c830() +0x2c | - | int `0x0085df80` (native_move) |
| `0x012e` | `Native_012e` | i int (0..7), v int: added to word i of record +0x2c | int: the sum | - | int `0x0085e040` (native_move) |
| `0x0130` | `Native_0130` | i int (0..7), v int: word i of record +0x2c (0 when negative) | int: the clamped i | - | int `0x0085e410` (native_move) |
| `0x0132` | `Native_0132` | worker int, on int: +0x34 bit 7 of a trigger worker | int: 1 for triggers (types 2, 3, 5, 6), else 0 | - | int `0x0085fe30` (native_move) |
| `0x0133` | `Native_0133` | as 0x0085fe30 | as 0x0085fe30 | - | int `0x00860290` (native_move) |
| `0x0134` | `Native_ChrLoadAlias` | as Native_SetModel (0x0001) | SetModel's result | - | int `0x0085d3c0` (native_move) |
| `0x0135` | `Native_0135` | as 0x00784e80 | as 0x00784e80 (a jump) | - | int `0x0085ed00` (native_move) |
| `0x0136` | `Native_0136` | v int | int: 1 when 0x007859a0(-v) is not negative | - | int `0x0085ee40` (native_move) |
| `0x0137` | `Native_0137` | v int | int: 1 when 0x007859a0(v) is not negative | - | int `0x0085ef80` (native_move) |
| `0x0138` | `Native_0138` | v int: 0x00785c20(v) | as 0x00784e80 | - | int `0x0085f140` (native_move) |
| `0x0139` | `Native_0139` | as 0x00860b00(worker, args, stack, 0) | int: 0x008b7c40(the choice) after Native_MsgSelect_Result | as Native_MsgSelect_Poll | start `0x0085f730`, poll `0x0085f850`, int `0x0085f900` (native_move) |
| `0x013b` | `Native_MsgShowWait` | window int, message int, choice b int, choice a int, x int, y int, anchor int | 1 when done; int slot: the choice (-1 none) | as 0x007d | start `0x008602a0`, poll `0x008603d0`, int `0x00860760` (native_msg) |
| `0x013d` | `Native_013d` | v int | 1 when done | until 0x00886630() clears and the loader is ready (then 0x00871750(ctrl +0x48, ctrl +0x204, v)), 0x00887550(), 0x00886630() again and two more frames | start `0x008561b0`, poll `0x00856300` (native_move) |
| `0x013e` | `Native_013e` | on int: +0x36 bit 15 | int: on != 0 | - | int `0x00856890` (native_move) |
| `0x013f` | `Native_MacroDicText` | dict int, entry int | int: text pointer | - | int `0x00856a00` (native_msg) |
| `0x0140` | `Native_0140` | msg int, _ int, n int, mask int, list var, dst var | int: k << 8 (k choices): the choice text from message table msg | - | int `0x00856f20` (native_move) |
| `0x0141` | `Native_0141` | window int, v int, var int: +8 = +0xc = &var, +0x16 = v | 0 | - | int `0x00857a00` (native_move) |
| `0x0142` | `Native_BitIndex` | mask int, n int | int: index of the nth set bit, 0 if fewer | - | int `0x008578c0` (native_msg) |
| `0x0143` | `Native_0143` | frames int | int: frames (0x00877450 with [0x00b92350] / (frames * [0x00b92220]), 0x00861000(frames, 0)) | - | int `0x00857ad0` (native_move) |
| `0x0144` | `Native_0144` | frames int | int: frames (0x00877450 with 1 / (frames * [0x00b92220]), 0x00861000(frames, 0x7f)) | - | int `0x00857d40` (native_move) |
| `0x0145` | `Native_0145` | b int, a int: 0x00919e50(b, a) | 1 | - | int `0x00857fc0` (native_move) |
| `0x0146` | `Native_0146` | v int: 0x00919e80(v) | int: v | - | int `0x008581a0` (native_move) |
| `0x0148` | `Native_MinimapSetMarker` | marker int (0 default), worker int | int: the stored value | - | int `0x008583b0` (native) |
| `0x0149` | `Native_0149` | on int: the ctrl work's +2 bit 7 | int: on != 0 | - | int `0x00858580` (native_move) |
| `0x014a` | `Native_014a` | c int, b int, a int: 0x0086fc20(c, b, a) | int: c | - | int `0x00858740` (native_move) |
| `0x014d` | `Native_014d` | as 0x0086bcc0 | as 0x0086bcc0 (a jump) | - | int `0x00858e10` (native_move) |
| `0x014e` | `Native_014e` | r int, y int, x int: stick position (0..255) | float: 0 inside radius r of (128, 128), else the angle 0x00889e70(x - 128, y - 128) (0 as [0x00b368a0]) | - | float `0x00858ea0` (native_move) |
| `0x014f` | `Native_014f` | c f, b f, a f | int: 1 when c > 0x0088b400(wrap(b - a), c) | - | int `0x00859120` (native_move) |
| `0x0151` | `Native_FootstepHookEnable` | v int | 1 (0x008870b0(v, 1)) | - | int `0x00859410` (native_move) |
| `0x0154` | `Native_0154` | - | int: 1 after 0x00887100(bound character, 1), 0 without one | - | int `0x008597a0` (native_move) |
| `0x0155` | `Native_0155` | v int: 0x00887100(v, 1) | 1 | - | int `0x00859d00` (native_move) |
| `0x0156` | `Native_0156` | - | int: 1 after 0x00887100(bound character, 0), 0 without one | - | int `0x00859920` (native_move) |
| `0x0157` | `Native_0157` | v int: 0x00887100(v, 0) | 1 | - | int `0x00859dc0` (native_move) |
| `0x0158` | `Native_0158` | slot int: the worker's party slot | int: slot | - | int `0x0085cfa0` (native_move) |
| `0x0159` | `Native_0159` | worker int, slot int: that worker's party slot | int: slot | - | int `0x0085d290` (native_move) |
| `0x015b` | `Native_015b` | window int, event int | the window record | until the window's message wait (Native_MsgWaitEx_Poll) is done, then one more frame (fanfare 0x10001 for a new primer); the result clears the ctrl work's +3 bit 2 and returns 1 | start `0x0085a8a0`, poll `0x0085af70`, int `0x0085b4c0` (native_move) |
| `0x015d` | `Native_015d` | v int | int: 0x00870160(v) | - | int `0x00857a80` (native_move) |
| `0x015e` | `Native_015e` | v int | int: 0x008701c0(v) | - | int `0x00857980` (native_move) |
| `0x015f` | `Native_015f` | v int: 0x0088e790(v) | int: v | - | int `0x0085b650` (native_move) |
| `0x0160` | `CT_0160_ret_hasKeyItem` | v int | int: 0x00790800(v) | - | int `0x0085b900` (native_move) |
| `0x0161` | `Native_0161` | v int: 0x0088e780(v) | int: v | - | int `0x0085b760` (native_move) |
| `0x0166` | `Native_0166` | track int: 0x00886a20, current music = track | 1 | - | int `0x008598f0` (native_move) |
| `0x0167` | `Native_0167` | - | 1: 0x008869f0() | - | int `0x00859c40` (native_move) |
| `0x016a` | `Native_016a` | b int, a int: 0x00886ab0(b, a) | int: a | - | int `0x0085bbb0` (native_move) |
| `0x016b` | `Native_016b` | c int, b int, a int: 0x00886b40(c, b, a) | int: a | - | int `0x0085be70` (native_move) |
| `0x016c` | `Native_016c` | - | 1: the current music stopped (0x00886a60), current music -1 | - | int `0x008594c0` (native_move) |
| `0x016d` | `Native_016d` | track int: 0x00886a60 (the current music cleared when it is track) | 1 | - | int `0x008595b0` (native_move) |
| `0x016e` | `Native_SetMapMarkerPos` | x int, y int: map marker (record +0xb4, +0xb6) | 1 | - | int `0x0085c2d0` (native_move) |
| `0x016f` | `Native_016f` | v int: record 0x0088c830() +0xb8 | int: v | - | int `0x0085c430` (native_move) |
| `0x0171` | `Native_CharRestoreHp` | chr int: party character | int (HP restored to the maximum) | - | int `0x0085c680` (native) |
| `0x0172` | `Native_CharRestoreMp` | chr int | int (MP restored to the maximum) | - | int `0x0085c860` (native) |
| `0x0174` | `Native_0174` | - | float: the position's +0x30 | - | float `0x0085e3a0` (native_move) |
| `0x0177` | `Native_0177` | on int: +0x36 bit 6 (and bit 15 when on) | int: on != 0 | - | int `0x0085c9c0` (native_move) |
| `0x0179` | `Native_0179` | v int | int: v | until 0x0086b030(v) is set | start `0x0085cd90`, poll `0x0085cea0` (native_move) |
| `0x017a` | `Native_017a` | v int: 0x0086fa40(v) (map 0x5e room 0x186: 9/10 set/clear [0x012fb8b4]) | int: v | - | int `0x0085d210` (native_move) |
| `0x017b` | `Native_017b` | v int: 0x00863860(v) | int: v | - | int `0x0085d3d0` (native_move) |
| `0x017c` | `Native_017c` | track int: 0x008869e0, 0x008868b0, current music = track | 1 | - | int `0x0085d480` (native_move) |
| `0x017e` | `Native_017e` | track int, v int: 0x00886830(track, v) (or 0x008869e0/0x00886a20), current music = track | 1 | - | int `0x0085d830` (native_move) |
| `0x017f` | `Native_017f` | v int: 0x00886ab0(current music, v) | int: the current music | - | int `0x0085bd10` (native_move) |
| `0x0180` | `Native_0180` | a int, b int | int: i = 0x00873b00(), with 0x00886b40(i, a, b) when i >= 0 | - | int `0x0085c000` (native_move) |
| `0x0181` | `Native_0181` | - | float: the position's +0x38 | - | float `0x0085e550` (native_move) |
| `0x0184` | `Native_0184` | v int (-128..127): 0x00886a90(v) | 1 | - | int `0x0085dac0` (native_move) |
| `0x0185` | `Native_0185` | track int, v int (-128..127): 0x00886aa0(track, v) | 1 | - | int `0x0085dbb0` (native_move) |
| `0x0188` | `Native_MusicFadeOut` | frames int: music fade-out (0 or less: stop at once) | int: frames | - | int `0x0085e290` (native_move) |
| `0x0189` | `Native_0189` | v int: 0x00873e60(v) | int: v | nothing (done at once) | start `0x0085e890`, poll `0x0085ea30` (native_move) |
| `0x018a` | `Native_018a` | v int: 0x008702f0(v) | int: v | - | int `0x0085ebb0` (native_move) |
| `0x018b` | `Native_018b` | v int: 0x00863af0(v) | int: v | - | int `0x0085ec90` (native_move) |
| `0x018f` | `Native_018f` | b int, a int: 0x00873d20(a), 0x008879b0(b, a) | 0 | - | int `0x0085f240` (native_move) |
| `0x0192` | `Native_0192` | v f: the ctrl work's +0x1f0 | float: v | - | float `0x0085f700` (native_move) |
| `0x0193` | `Native_0193` | track int: request or start (0x00886ff0(track, 0xc, 0x7f or 0x3f)) unless marked | int: track | - | int `0x0085f4e0` (native_move) |
| `0x0194` | `Native_0194` | v int: 0x0086f060(v) | int: v | - | int `0x0085f8b0` (native_move) |
| `0x0195` | `Native_0195_Poll` | - | 5 when done | until 0x0086b060() is 0 (poll only) | poll `0x0085fb00` (native_move) |
| `0x0196` | `Native_0196` | v int: 0x00862800(worker, v, +0xa8) for attached workers | 1 | - | int `0x0085fce0` (native_move) |
| `0x0197` | `Native_0197` | v int: +0xa8 (+0xaa = 5) | 0 | - | int `0x0085edd0` (native_move) |
| `0x0198` | `Native_0198` | c int, b int, a int: 0x00908330(c, b, a) for +0xaa 5 | int: 1, else 0 | - | int `0x0085ffa0` (native_move) |
| `0x0199` | `Native_0199` | b int, a int: 0x00908330(+0xa8, b, a) for +0xaa 5 | int: 1, else 0 | - | int `0x00860200` (native_move) |
| `0x019c` | `Native_AlwaysTrue019c` | - | 1 | - | int `0x00856630` (native) |
| `0x019d` | `Worker_Detach` | v int: also release the model when nonzero | 0: detaches the worker (+0xaa, +0xa8 = 0) | - | int `0x0085ca00` (native_move) |
| `0x019e` | `Native_019e` | - | int: 0x0086c530(0) | - | int `0x00856950` (native_move) |
| `0x019f` | `Native_019f` | v int | int: 0x0086c530(v) | - | int `0x008567a0` (native_move) |
| `0x01a0` | `Native_01a0` | track int, b int, a int: as 0x0193 with volume a (halved when [record +0x28]) and 0x00886e20(track, 0xc, b) | int: track | - | int `0x0085f940` (native_move) |
| `0x01a4` | `Native_01a4` | i int, x f: 0x0090a4d0(i, &[x, 0, 0, 0]) | 1 | - | int `0x00857360` (native_move) |
| `0x01a5` | `Native_01a5` | index int, frame int: texture swap frame (yiTexSwapSetFrame) | 1 | - | int `0x008574c0` (native_map) |
| `0x01a6` | `Native_01a6` | - | 1: fills the empty party slots with the lowest free characters 0..7 | - | int `0x008576d0` (native_move) |
| `0x01a7` | `Native_01a7` | v int: event | the args or 0x00798b60() | until a new primer's fanfare (0x10001) has played; the result runs 0x00798b60() for a new primer and returns 1 | start `0x00857b70`, poll `0x00857ef0`, int `0x00858230` (native_move) |
| `0x01a8` | `Native_01a8` | chr int | int: +0x54 of 0x00785330(chr) | - | int `0x00858390` (native_move) |
| `0x01ab` | `Native_01ab` | as 0x00784f50 | as 0x00784f50 (a jump) | - | int `0x00858800` (native_move) |
| `0x01ac` | `Native_01ac` | v int | int: 0x00790af0(v) | - | int `0x00858950` (native_move) |
| `0x01ad` | `Native_01ad` | v int | int: 0x00790b90(v, 1) | - | int `0x00858b30` (native_move) |
| `0x01af` | `Native_01af` | b int, a int | int: 0x00790b90(b, a) | - | int `0x00858e70` (native_move) |
| `0x01b0` | `Native_01b0` | b int, a int: 0x00873eb0(b, a) (the original's printf of both left out) | int: b | - | int `0x008590a0` (native_move) |
| `0x01b1` | `Native_01b1` | - | 1: 0x00873db0() | - | int `0x00859240` (native_move) |
| `0x01b2` | `Native_01b2` | v int | int: 0x007904b0(v) | - | int `0x00859360` (native_move) |
| `0x01b5` | `Native_01b5` | - | 0x00644280(1), then as 0x00874060 | - | int `0x00859580` (native_move) |
| `0x01b6` | `Native_01b6` | - | int: 0x007ad6e0() != 0 | - | int `0x00859630` (native_move) |
| `0x01b7` | `Native_01b7` | v int: 0x007ad640(v) when 0x007ad6e0() | int: 1, else 0 | - | int `0x00859800` (native_move) |
| `0x01b8` | `Native_01b8` | - | int: 0x007ad710(1) | - | int `0x00859ba0` (native_move) |
| `0x01ba` | `Native_01ba` | worker int, on int (0..1) | int: 0x0086b150(its bound character, on), 0 for none | - | int `0x0085beb0` (native_move) |
| `0x01bb` | `Native_01bb` | - | int: 0x008863c0(0) | - | int `0x00859de0` (native_move) |
| `0x01bc` | `Native_01bc` | v int: 0x00870140(v) | 1 | - | int `0x00859ec0` (native_move) |
| `0x01bd` | `Native_01bd` | v int: 0x008715f0(v) | 1 | - | int `0x00859ff0` (native_move) |
| `0x01bf` | `Native_01bf` | - | 1: 0x008708d0() | - | int `0x0085a190` (native_move) |
| `0x01c0` | `Native_01c0` | i int (0..3), v int: 0x0086fa60(i, v) | int: v | - | int `0x0085a250` (native_move) |
| `0x01c2` | `Native_01c2` | - | 1: 0x00873d30() | - | int `0x0085a570` (native_move) |
| `0x01c3` | `Native_01c3` | v int: 0x00874030(v) | int: v | - | int `0x0085a750` (native_move) |
| `0x01c4` | `Native_01c4` | on int: 0x00873dd0(on != 0) | int: on != 0 | - | int `0x0085a840` (native_move) |
| `0x01c5` | `Native_01c5` | b int, a int: Al Bhed dictionary mode (args +0xa = b, +0xb = a) | int: the outcome (0, 2, 3, 4 or 6) | for worker +0xa8 0x5045: while 0x00822540() is -1 and 0x00648100() is 0 | start `0x0085b1a0`, poll `0x0085b590`, int `0x0085d8f0` (native_move) |
| `0x01c6` | `Native_01c6` | as 0x0088e5e0 | as 0x0088e5e0 (a jump) | - | int `0x0085de70` (native_move) |
| `0x01c9` | `Native_01c9` | b int, a int: 0x009084b0(b, a) | 1 | - | int `0x00860730` (native_move) |
| `0x01ca` | `Native_01ca` | - | 1: the worker's +0x38 bit 2 | - | int `0x0085e340` (native_move) |
| `0x01cc` | `Native_01cc` | as 0x008863f0 | as 0x008863f0 (a jump) | - | int `0x0085e400` (native_move) |
| `0x01ce` | `Native_01ce` | - | 1: the character's 0x007fb290/0x007fbab0, +0x38 bit 4 | - | int `0x0085e7c0` (native_move) |
| `0x01cf` | `Native_01cf` | - | 1: the character's 0x007fba40, +0x38 bit 4 cleared | - | int `0x0085ea40` (native_move) |
| `0x01d0` | `Native_01d0` | - | 1: the worker's +0x38 bit 3 when it has a character | - | int `0x0085fbb0` (native_move) |
| `0x01d1` | `Native_WaitFrames_01d1` | frames int | 1 when done | for frames frames (at once when skipping) | start `0x0085fd20`, poll `0x0085ff70` (native) |
| `0x01d2` | `Native_01d2` | msg int | int: 1 when its text equals record 0x0088c830() +0xec | - | int `0x00860180` (native_move) |
| `0x01d4` | `Native_01d4` | slot int, on int: the slot model's 0x0082aa30 and +0xb08 bit 0 | int: on != 0 | - | int `0x00855dd0` (native_move) |
| `0x01d9` | `Native_01d9` | - | 1: text 0x0078fcf0(3, 0x202c, ..) copied to 0x00784fb0(6) | - | int `0x008582e0` (native_move) |
| `0x01da` | `Native_01da` | d int, c int, b int, a int: 0x00876050(3, d, c, b, a) | 1 | - | int `0x00856a80` (native_move) |
| `0x01dc` | `Native_01dc` | - | 3: 0x00876290(3) | - | int `0x008571d0` (native_move) |
| `0x01de` | `Native_01de` | - | 3 | until 0x00875de0(3) is 0 | start `0x00857920`, poll `0x008579d0` (native_move) |
| `0x01e0` | `Native_01e0` | v int: 0x0082a9d0(character, v) | int: 1 with a character, else 0 | - | int `0x00858490` (native_move) |
| `0x01e2` | `Native_01e2` | as 0x009085a0 | as 0x009085a0 (a jump) | - | int `0x00858940` (native_move) |
| `0x01e3` | `Native_01e3` | as 0x00908450 | as 0x00908450 (a jump) | - | int `0x00858ab0` (native_move) |
| `0x01e4` | `Native_01e4` | a f, b f, c f, n int: yiSetFieldVec3Int | int: 0 | - | int `0x00858c90` (native_map) |
| `0x01e5` | `Native_01e5` | c int, b int, a int | int: 0x0090a400(c, b, a) | - | int `0x00859000` (native_move) |
| `0x01e6` | `Native_01e6` | on int, a f, b f: yiSetFieldPair | int: on | - | int `0x008591d0` (native_map) |
| `0x01e7` | `Native_01e7` | track int, v int: 0x008869e0, 0x008868f0(track, v), current music = track | 1 | - | int `0x00859320` (native_move) |
| `0x01e9` | `GetField0_008595f0` | window int, wait int | int: the window (0x008640f0(window) at the start) | while wait is set, until the window's state (+0x14) is 0 or 4 | start `0x008590f0`, poll `0x008592c0`, int `0x008595f0` (native_move) |
| `0x01eb` | `Native_01eb` | v int: 0x00875fb0(1, v, 0x0088d590()) | 1 | - | int `0x00858070` (native_move) |
| `0x01ee` | `Native_01ee` | - | 1: 0x00876290(1) | - | int `0x00857440` (native_move) |
| `0x01f0` | `Native_01f0` | v f: 0x00873e50(v) | float: v | - | float `0x00859550` (native_move) |
| `0x01f2` | `Native_01f2` | - | int: bits 8..15 for the characters 8..15 that 0x00785380 reports | - | int `0x008597d0` (native_move) |
| `0x01f3` | `Native_01f3` | enabled int, all int, dst var | int: k << 8 (k choices): a choice text of characters 8..15 | - | int `0x00859990` (native_move) |
| `0x01f4` | `Native_01f4` | n int, mask int | int: the n-th set bit of mask among bits 8..15, -1 when fewer | - | int `0x00859fa0` (native_move) |
| `0x01f8` | `Native_01f8` | track int, frames int: fade (0 or less: stop the current music) | int: frames | - | int `0x0085e570` (native_move) |
| `0x01f9` | `Native_01f9` | chr int: Native_00cb on it, then 0x00786aa0(chr, 0) | 1 | - | int `0x0085ac50` (native_move) |
| `0x01fa` | `Native_01fa` | as 0x008bef00 | as 0x008bef00 (a jump) | - | int `0x0085af60` (native_move) |
| `0x01fb` | `Native_01fb` | as 0x008beef0 | as 0x008beef0 (a jump) | - | int `0x0085af10` (native_move) |
| `0x01fc` | `CT_01FC_ret_teachAbilityToPartyMemberSilently` | b int, a int | int: 0x00874200(b, a, 1) | - | int `0x0085b170` (native_move) |
| `0x0200` | `Native_0200` | v int: 0x00887100(v, 1), 0x00887130(v, 1) | 1 | - | int `0x0085a430` (native_move) |
| `0x0201` | `Native_0201` | - | 1: 0x00887100/0x00887130(bound character, 0) | - | int `0x0085a580` (native_move) |
| `0x0202` | `Native_0202` | v int: 0x00887100(v, 0), 0x00887130(v, 0) | 1 | - | int `0x0085a770` (native_move) |
| `0x0203` | `Native_0203` | chr int | int: 0x0085d000's result after 0x00863650(worker id) | until 0x0086ade0() clears, the loader is ready, 0x00887b30()/0x0076f060() are 0, 0x00836a50(chr, 0) and 0x0082a040(chr) are not 1 | start `0x0085d460`, poll `0x0085d550`, int `0x0085dc10` (native_move) |
| `0x0204` | `Native_0204` | v f: position +0x2c | float: v | - | float `0x0085dc90` (native_move) |
| `0x0205` | `Native_0205` | - | 1: 0x00862700(worker, 0) with a character | - | int `0x0085ec00` (native_move) |
| `0x0206` | `Native_0206` | - | 1: 0x00862680(worker, 0) with a character | - | int `0x0085ee10` (native_move) |
| `0x0207` | `Native_0207` | - | 1: 0x00862700(worker, 1) with a character | - | int `0x0085f010` (native_move) |
| `0x0209` | `Native_0209` | - | 1: 0x00862700(worker, 2) with a character | - | int `0x0085f430` (native_move) |
| `0x020a` | `Native_020a` | - | 1: 0x00862680(worker, 2) with a character | - | int `0x0085f660` (native_move) |
| `0x020b` | `Native_020b` | - | 1: the worker's +0x38 bits 8 and 2 | - | int `0x0085be30` (native_move) |
| `0x020d` | `Native_020d` | v int: 0x00870fa0(v) | int: v | - | int `0x0085bf70` (native_move) |
| `0x020f` | `Native_020f` | v int: 0x00870130(v), then the ctrl work 0x0086c1f0() made current | int: the previous ctrl (also to 0x00780c70) | - | int `0x0085c100` (native_move) |
| `0x0210` | `Native_BitFlagSet` | bit int | int: bit (set) | - | int `0x0085c3b0` (native_move) |
| `0x0212` | `Native_0212` | on int, v f: the ctrl work's +0x208 = v, +2 bit 0 = on | int: on != 0 | - | int `0x0085c950` (native_move) |
| `0x0213` | `Native_ChrGetAttachBone` | bone int | int: 0x00835660(character, bone, 0), 0 without a character | - | int `0x0085ce30` (native_move) |
| `0x0215` | `Native_0215` | i int (0..17), v int (1..2) | int: 0x008c3170(i, v), 0 out of range | - | int `0x0085d170` (native_move) |
| `0x0216` | `CT_0216_ret_teachAbilityToPartyMemberWithMsg` | window int, item int, n int | the window record (or 0x00785020's refusal) | until the window's message wait is done, then one more frame; the result clears the ctrl work's +3 bit 2 and returns 0x00874200(item, n, 1) | start `0x0085b280`, poll `0x0085b5d0`, int `0x0085b920` (native_move) |
| `0x0217` | `Native_0217` | - | 1: 0x00798b60() | - | int `0x0085d3a0` (native_move) |
| `0x021a` | `Native_021a` | b int, a int | int: 0x00872aa0(0, b, a, 0) | - | int `0x0085d780` (native_move) |
| `0x021b` | `Native_021b` | b int, a int | int: 0x00872aa0(1, b, a, 0) | - | int `0x0085d8c0` (native_move) |
| `0x021d` | `Native_021d` | - | int: 1 when a primer flagged +3 bit 3 was handed out (0x007ab970, 0x008c30d0) | - | int `0x0085dcd0` (native_move) |
| `0x021e` | `Native_021e` | - | 1: 0x0090a420() | - | int `0x00858e40` (native_move) |
| `0x0220` | `Native_0220` | v int: 0x00870250(v, 1) | int: v | - | int `0x0085e010` (native_move) |
| `0x0221` | `Native_0221` | i int | int: word i of the table at 0x00c86e28 | - | int `0x0085e110` (native_move) |
| `0x0225` | `Native_0225` | v int: 0x00870f60(v) | int: v | - | int `0x0085ea10` (native_move) |
| `0x0226` | `Native_0226` | on int: 0x00729050(on != 0) | int: on != 0 | - | int `0x0085eb80` (native_move) |
| `0x0229` | `SetFieldBits38_0085ece0` | - | 1: the worker's +0x38 bit 9 | - | int `0x0085ece0` (native_move) |
| `0x022e` | `Native_022e` | d int, c int, b int, a int: 0x00870280(d, c, b, a) | 1 | - | int `0x0085f860` (native_move) |
| `0x022f` | `Native_022f` | slot int, on int: the slot model's 0x0082b970 and +0xb08 bit 1 | int: on != 0 | - | int `0x00855fb0` (native_move) |
| `0x0231` | `Native_0231` | worker int: its +0x38 bit 11 | 1 | - | int `0x0085fca0` (native_move) |
| `0x0234` | `Native_0234` | - | 1: 0x0086a040() | - | int `0x0085fff0` (native_move) |
| `0x0235` | `Native_0235` | as 0x008729d0 | as 0x008729d0 (a jump) | - | int `0x00860170` (native_move) |
| `0x0236` | `Native_0236` | msg int | int: the message text (0x0086b170 or 0x0086bf30) | - | int `0x00860350` (native_move) |
| `0x0237` | `Native_0237` | n int, mask int, list var, dst var: list of n string pointers; mask bit i enables entry i | int: k << 8 (k choices): the choice text in dst (copied once more after its end) | - | int `0x008604c0` (native_move) |
| `0x0239` | `Native_0239` | i int (below 640): bit i of record 0x0088c830() +0x4c set | 1 | - | int `0x0085e8b0` (native_move) |
| `0x023a` | `Native_023a` | v int: 0x0086f9e0(v) | 1 | - | int `0x00856250` (native_move) |
| `0x023b` | `Native_023b` | v int | int: 0x0086b9e0(v) | - | int `0x0085c490` (native_move) |
| `0x023c` | `Native_023c` | b int, n int | int: 1 when 0x0086b9e0(b + i) is set for every i below n | - | int `0x0085c600` (native_move) |
| `0x023d` | `Native_023d` | - | 1: 0x00647e40, 0x00642390(7) when [0x025d5ebc], 0x00786b00, 0x00782050, 0x0088d1c0(0), 0x00643e70 | - | int `0x00856520` (native_move) |
| `0x023f` | `Native_023f` | v int: 0x008749e0(v) | int: v | - | int `0x00856770` (native_move) |
| `0x0240` | `Native_0240` | b int, on int: 0x007ad5e0(b, on != 0) | int: on != 0 | - | int `0x008568d0` (native_move) |
| `0x0241` | `CT_0241_ret` | on int: field state bit 3 | int: on != 0 | - | int `0x00856ad0` (native) |
| `0x0242` | `Native_0242` | as 0x00777e90 | as 0x00777e90 (a jump) | - | int `0x00857200` (native_move) |
| `0x0243` | `Native_0243` | as 0x007787b0 | as 0x007787b0 (a jump) | - | int `0x00857350` (native_move) |
| `0x0244` | `Native_0244` | - | 1: 0x007fb690() | - | int `0x008574b0` (native_move) |
| `0x0245` | `Native_0245` | - | 1: 0x007fb680() | - | int `0x00857690` (native_move) |
| `0x024a` | `Native_024a` | on int: bit 13 of *0x00785290(), then 0x008724a0() | int: on != 0 | - | int `0x00857cb0` (native_move) |
| `0x024b` | `Native_024b` | slot int, on int: 0x0082b2c0(model, on) for a used slot with a motion | int: on != 0 | - | int `0x00857e30` (native_move) |
| `0x024c` | `SetFieldBits38_008580a0` | - | 1: the worker's +0x38 bit 12 | - | int `0x008580a0` (native_move) |
| `0x024d` | `Native_024d` | on int: the ctrl work's +0x210 bit 6 | int: on != 0 | - | int `0x00856dc0` (native_move) |
| `0x0251` | `Native_0251` | i int | int: bit i of the flag word for 0x0086af60() | - | int `0x0085e5e0` (native_move) |
| `0x0253` | `Native_LangGroupNotJpAsia` | - | int: 1 unless the language group (0x00641100) is 0, 3 or 4 | - | int `0x00858210` (native_move) |
| `0x0254` | `Native_0254` | v int: bits 14..15 of *0x00785290(), then 0x00a53de0(0x0112ec7c) | int: v | - | int `0x008585d0` (native_move) |
| `0x0255` | `Native_0255` | - | int: bits 14..15 of *0x00785290() | - | int `0x00858430` (native_move) |
| `0x0256` | `Native_0256` | window int: +0x28 = 0x8000 | int: window | - | int `0x00858710` (native_move) |
| `0x0257` | `Native_0257` | window int: +0x28 = 0 | int: window | - | int `0x008588b0` (native_move) |
| `0x0259` | `Native_0259` | i int, v f: 0x00872530(i, v) | float: v | - | float `0x00858fc0` (native_move) |
| `0x025b` | `Native_025b` | v int: 0x00822380 (0x17 as 0x16, 0x19 as 0x18; 0x16 nothing) | int: the value used | - | int `0x00859270` (native_move) |
| `0x025c` | `Native_025c` | as 0x0086c270 | as 0x0086c270 (a jump) | - | int `0x00859400` (native_move) |
| `0x025d` | `Native_025d` | v int: [0x01325ac4] | int: v | - | int `0x00859c20` (native_move) |
| `0x025e` | `Native_025e` | v int | int: 0x00643e80(v) | - | int `0x00859d50` (native_move) |
| `0x025f` | `Native_025f` | b int, a int: 0x00643500(b, a) | 0 | - | int `0x00859e20` (native_move) |
| `0x0260` | `Native_0260` | _ int | 1 | - | int `0x00859f10` (native_move) |
| `0x0261` | `Native_0261` | _ int | 1 | - | int `0x0085a090` (native_move) |
| `0x0262` | `Native_0262` | - | 1 | - | int `0x0085a110` (native_move) |
| `0x0263` | `Native_0263` | - | 1 | - | int `0x0085a120` (native_move) |
| `0x0264` | `Native_0264` | v int: 0x008cc150(v) | 0 | - | int `0x0085a230` (native_move) |
| `0x0265` | `Native_0265` | - | int: [0x00cccacc] | - | int `0x0085a7f0` (native_move) |
| `0x0266` | `Native_0266` | as 0x008cc140 | as 0x008cc140 (a jump) | - | int `0x0085a410` (native_move) |
| `0x0267` | `Native_0267` | as 0x013b | 1 when done; int slot: the choice (-1 none) | as 0x013b; a choice with status 1 calls 0x008cc150 | start `0x008602a0`, poll `0x0085a540`, int `0x0085a620` (native_msg) |
| `0x1000` | `Native_1000` | slot int, a int, b int, v int | 1: ctrl work +0xa4 slot (0..31) = a/8 | 0x80, b/4, the worker's +0x2e, v (cleared for negative a or b) | - | int `0x008779c0` (native_chr) |
| `0x1001` | `AtelCall_Math_Sin_RF` | v f | float: CIsin(v) (0x0088c3a0) | - | float `0x00877a70` (native_chr) |
| `0x1002` | `AtelCall_Math_Cos_RF` | v f | float: CIcos(v) (0x0088b3e0) | - | float `0x00877aa0` (native_chr) |
| `0x1005` | `AtelCall_Math_ATan2_RF` | y f, x f | float: atan2(y, x) through the game's _CIatan2 (import thunk 0x009498bc), rounded to f32 | - | float `0x00877b30` (native_chr) |
| `0x1006` | `AtelCall_Math_Sqrt_RF` | x f | float: sqrt(x) as f32 | - | float `0x00877b70` (native_move) |
| `0x100a` | `Native_100a` | v int | int: the popped value; int: the popped value | - | int `0x008778b0` (native_chr) |
| `0x1013` | `Native_1013` | v f | float: Math_WrapAngle(v) (0x0088bc20) | - | float `0x00877d30` (native_chr) |
| `0x1015` | `Native_1015` | - | six floats: 0x0088b780(f6, ..., f1)'s EAX | - | int `0x00877dc0` (native_chr) |
| `0x1019` | `AtelCall_Math_Abs_RF` | - | |v| rounded to f32 | - | float `0x00877fd0` (native_chr) |
| `0x101a` | `Native_101a` | x1 f, z1 f, x2 f, z2 f | float: sqrt((z2 - z1)^2 + (x2 - x1)^2), f32 steps | - | float `0x00877ff0` (native_chr) |
| `0x101b` | `Native_101b` | x1 f, y1 f, z1 f, x2 f, y2 f, z2 f | float: the 3D distance, as 0x101a | - | float `0x00878060` (native_chr) |
| `0x101c` | `Native_101c` | - | 0x00878140(worker, args, stack)'s record +4 = 0; 0 | - | int `0x008780f0` (native_chr) |
| `0x4001` | `Native_4001` | v f | 0: Sg_SetSuimen(v) (0x00836640) | - | int `0x00a77b90` (native_chr) |
| `0x4003` | `Atel_Sg_Flash` | d int, c int, b int, a int | 0: Sg_Flash(d, c, b, a) (0x0082cca0) | - | int `0x00a77d30` (native_chr) |
| `0x4004` | `Native_FadeIn` | frames int | 0 (fade in) | - | int `0x00a77d80` (native_move) |
| `0x4005` | `Native_FadeOut` | frames int | 0 (fade out) | - | int `0x00a77da0` (native_move) |
| `0x4006` | `Native_4006` | v int | 0: Sg_Fade_Common(v, 1, 1) | - | int `0x00a77dc0` (native_chr) |
| `0x4007` | `Native_4007` | v int | 0: Sg_Fade_Common(v, 0, 1) (0x0082cdc0) | - | int `0x00a77de0` (native_chr) |
| `0x4008` | `Native_4008` | d, c, b, a | Sg_FilterAdd(d, c, b, a) (0x0082cc20) | - | int `0x00a77e30` (native_chr) |
| `0x4009` | `Native_4009` | d, c, b, a | Sg_FilterSub(d, c, b, a) (0x0082cc80) | - | int `0x00a77e80` (native_chr) |
| `0x400a` | `Native_MotionBlur` | v int | 0 (motion blur) | - | int `0x00a77e00` (native_move) |
| `0x400b` | `Native_400b` | - | 0x0082cc00() | - | int `0x00a77e20` (native_chr) |
| `0x400c` | `Native_400c` | - | 0: 0x0082cc70() | - | int `0x00a77ed0` (native_chr) |
| `0x400d` | `Native_FadeWait` | - | - (start: a bare ret) | until 0x0082cc10() is 0 | start `0x00a77ee0`, poll `0x00a77ef0` (native_chr) |
| `0x400e` | `Native_400e` | v | Sg_EnableCalcMap(v) (0x00820440) | - | int `0x00a77f00` (native_chr) |
| `0x400f` | `Native_400f` | v | Sg_EnableDrawMap(v) (0x00820400) | - | int `0x00a77f20` (native_chr) |
| `0x4013` | `Native_LightSet` | light int (0-3), r int, g int, b int, pitch int (deg), yaw int (deg) | 0 | - | int `0x00a78030` (native_move) |
| `0x4014` | `Native_4014` | v int | int: Sg_EnFocus(v) (0x0082cb60) | - | int `0x00a780a0` (native_chr) |
| `0x4015` | `Native_4015` | b int, a int | 0: Sg_SetFocus(b, a) (0x0082ce20) | - | int `0x00a780c0` (native_chr) |
| `0x4016` | `Native_4016` | i int, v f | 0: 0x0082d9e0(i, v) | - | int `0x00a780f0` (native_chr) |
| `0x4017` | `Native_4017` | b int, a int | 0: Sg_IdSetMapTable(b, a) (0x0083e9e0) | - | int `0x00a78130` (native_chr) |
| `0x4019` | `Native_4019` | v int | 0: 0x0082bdf0(v) | - | int `0x00a78180` (native_chr) |
| `0x401a` | `Native_401a` | b int, a int | 0: Sg_AccSetAlpha(b, a) (0x0082bd10) | - | int `0x00a781a0` (native_chr) |
| `0x401b` | `Native_401b` | v | Sg_AccSetDrawMap(v) (0x0082bcb0) | - | int `0x00a781d0` (native_chr) |
| `0x401c` | `Native_401c` | v | Sg_AccEnableBG(v) (0x0082be10) | - | int `0x00a781f0` (native_chr) |
| `0x401d` | `Native_MenuOpen` | menu int | the start's result; poll 1 when done; int slot the menu result (0x00821740) | one frame (g_MenuWaitFrames) | start `0x00a78210`, poll `0x00a78270`, int `0x00a782a0` (native_move) |
| `0x401f` | `Native_401f` | - | 0x00840e40() | - | int `0x00a782e0` (native_chr) |
| `0x4020` | `Native_4020` | - | 0: 0x00840e50() | - | int `0x00a782f0` (native_chr) |
| `0x4022` | `Native_4022` | v | Sg_SetKeepFps(v) (0x00821ae0) | - | int `0x00a78320` (native_chr) |
| `0x4023` | `Native_4023` | v | Sg_PLightAllocate(v) (0x00831950) | - | int `0x00a78340` (native_chr) |
| `0x4024` | `Native_4024` | - | 0x00831a30() | - | int `0x00a78360` (native_chr) |
| `0x4025` | `Native_4025` | n, a f, b f, c f, d f, e, f, g, i | Sg_PLightDefine(n, a, b, c, d, e, f, g, i) (0x008319b0; the original's dbgPrintf is left out) | - | int `0x00a78370` (native_chr) |
| `0x402a` | `Native_402a` | b, a | Sg_PLightSetEnable(b, a) (0x00831c60) | - | int `0x00a785b0` (native_chr) |
| `0x4034` | `Native_4034` | v | Sg_SetMapAlphaFast(v) (0x00821b00); 0.0 | - | float `0x00a779e0` (native_chr) |
| `0x4036` | `Native_4036` | x f, y f, z f | Sg_GlobalSetWind(x, y, z) (0x00824e30) | - | int `0x00a77a10` (native_chr) |
| `0x4039` | `Native_4039` | d, c, b, a | Sg_AccSetClear(d, c, b, a) (0x0082bdc0); 0.0 | - | float `0x00a77ac0` (native_chr) |
| `0x403a` | `Native_403a` | v | Sg_AccSetDrawChr(v) (0x0082be00) | - | int `0x00a77b10` (native_chr) |
| `0x403b` | `Native_FootSetMaterial` | index int, material int | 0: Sg_IdSetEffectTable(index, material) (0x0083e9b0) | - | int `0x00a77b30` (native_chr) |
| `0x403c` | `Native_403c` | v f | Sg_SetWaterLimit(v) (0x00836660) | - | int `0x00a77b60` (native_chr) |
| `0x403e` | `Native_403e` | i, v f | Sg_PlayerSetDirFix(i, v) (0x0082d990) | - | int `0x00a77be0` (native_chr) |
| `0x403f` | `Native_403f` | - | 0x0082d930() | - | int `0x00a77c20` (native_chr) |
| `0x4040` | `Native_SetControlAngle` | degrees f | 0: 0x0082d950(degrees) | - | int `0x00a77c30` (native_chr) |
| `0x4043` | `Native_4043` | b, a | Ch_DrawSetShadowDir(b, a) (0x00830120) | - | int `0x00a77c80` (native_chr) |
| `0x4044` | `Native_4044` | - | three popped and dropped; 0 | - | int `0x00a77cb0` (native_chr) |
| `0x4045` | `Native_4045` | v | 0x0082d9d0(v) | - | int `0x00a77ce0` (native_chr) |
| `0x4046` | `Native_4046` | v | Ch_DrawSetShadowSemi(v) (0x00821b20) | - | int `0x00a77d00` (native_chr) |
| `0x5000` | `Native_ChrSetMotion_5000` | motion int | 0 | - | int `0x00a78770` (native) |
| `0x5001` | `Native_ChrLoadMotionGroup` | n int | int: 0, Chr_LoadMotionGroup(n) (0x008375f0) | - (the poll returns 5) | start `0x00a799b0`, poll `0x00a79a10`, int `0x00a79a20` (native_chr) |
| `0x5003` | `Native_ChrWaitMotion_5003` | - | 5 when done | until the character's motion has ended (+0x736) | start `0x00a787d0`, poll `0x00a788a0` (native) |
| `0x5005` | `Native_ChrReleaseMotion` | - | 0 (automatic motions take over) | - | int `0x00a78940` (native) |
| `0x5006` | `Native_5006` | scale f | 0: Ch_SetScaleAll(model, scale) (0x0082b590) | - | int `0x00a78a40` (native_chr) |
| `0x5007` | `Native_5007` | n, x f, y f, z f | Ch_SetScale(model, n, x, y, z) (0x0082b560) | - | int `0x00a789a0` (native_chr) |
| `0x5008` | `Native_ChrHide` | hide int (0, 1) | 0 | - | int `0x00a78ac0` (native) |
| `0x5009` | `Native_ChrSetPushWeight` | weight f (push weight) | float: previous weight | - | float `0x00a78b30` (native) |
| `0x500b` | `Native_500b` | v f | 0: Ch_SetJimen(model, v) (0x0082b360) | - | int `0x00a78b90` (native_chr) |
| `0x500c` | `Native_500c` | v | popped and dropped (the original only prints a warning for a model); 0 | - | int `0x00a78d30` (native_chr) |
| `0x500d` | `Native_500d` | v int | 0: Ch_SetHideEvent(model, v) (0x0082b2c0) | - | int `0x00a78df0` (native_chr) |
| `0x500e` | `Native_ChrSetGroundFollow` | on int: stick to the ground | 0 | - | int `0x00a78cb0` (native) |
| `0x500f` | `Native_ChrSetBodyHit` | on int: collide with characters | 0 | - | int `0x00a78da0` (native) |
| `0x5010` | `Native_ChrLoadSystemMgrp` | chr id int, n int | start: the args words; int: 0, ChN_ReadSystemMGRP(id, n) (0x00836870) | - (the poll returns 5) | start `0x00a79820`, poll `0x00a79970`, int `0x00a79980` (native_chr) |
| `0x5013` | `Native_5013` | mode int | 0: Ch_SetFieldMode, Ch_SetMotionType(model, mode) (0x0082b1e0, 0x0082b3a0) | - | int `0x00a78ec0` (native_chr) |
| `0x5014` | `Native_5014` | v f, n | Ch_SetShade(model, v, n) (0x0082b5b0) | - | int `0x00a78f30` (native_chr) |
| `0x5015` | `Native_5015` | r f, g f, b f, n int | 0: Ch_SetShadeCol(model, r, g, b, n) (0x0082b5d0) | - | int `0x00a78fb0` (native_chr) |
| `0x5016` | `Native_ChrSetOffset` | x f, y f, z f: model offset | 0 | - | int `0x00a790b0` (native) |
| `0x5017` | `Native_ChrSetGroundMode` | mode int (0 clamp, 1 snap, 2 float) | 0 | - | int `0x00a79130` (native) |
| `0x5018` | `Native_5018` | v int | 0: Ch_SetMotionSpeed(model, v) (0x0082b380) | - | int `0x00a791e0` (native_chr) |
| `0x5019` | `Native_5019` | - | Native_PlayMotionCommon(worker, stack, 0, 0) | - | int `0x00a79220` (native_chr) |
| `0x501a` | `Native_ChrPlayMotion` | motion args as Native_PlayMotionCommon (stack) | 0 | - | int `0x00a79270` (native_move) |
| `0x501b` | `Native_ChrSetMotion_501b` | motion int | 0 | - | int `0x00a792d0` (native) |
| `0x501e` | `Native_ChrWaitMotion_501e` | - | 5 when done | until the scripted motion has ended | start `0x00a79370`, poll `0x00a793c0` (native) |
| `0x501f` | `Native_501f` | - | 1 while the worker's model has +0x736 set | - | int `0x00a79450` (native_chr) |
| `0x5020` | `Native_ChrReleaseMotion2` | - | 0 (as 0x5005) | - | int `0x00a794b0` (native) |
| `0x5021` | `Native_ChrSetMotionSpeed` | speed int (8.8, 256 = 1x) | 0 | - | int `0x00a79510` (native) |
| `0x5022` | `Native_ChrSetMotionSeqLevel` | level int | 0 | - | int `0x00a79550` (native_move) |
| `0x5025` | `Native_ChrSetAutoMotion` | slot int, motion int (-1 default) | 0 | - | int `0x00a79670` (native) |
| `0x5026` | `Native_5026` | worker, point, &x, &y, &z | Ch_ElemGetPos(model, point, 0, &v) (0x00833d10) into the three variables (null ones skipped); 0 | - | int `0x00a796e0` (native_chr) |
| `0x5028` | `Native_5028` | v f | Ch_MoveSetChgRun(model, v) (0x0082b040), 0.0 without a model | - | float `0x00a799d0` (native_chr) |
| `0x5029` | `Native_ChrSetShadowMode` | mode int | int: Ch_SetShadowMode's result | - | int `0x00a79a40` (native) |
| `0x502a` | `Native_ChrSetDistanceCull` | on int | int: previous | - | int `0x00a79af0` (native) |
| `0x502c` | `Native_502c` | worker, point | as Native_ChrAttach with Ch_BindToElemDirect (0x00832680) | - | int `0x00a79bc0` (native_chr) |
| `0x502d` | `Native_ChrAttach` | worker int (-1 detaches), point int | 0: Ch_BindToElem(model, that worker's model, point) (0x00832630) | - | int `0x00a79c50` (native_chr) |
| `0x5032` | `Native_ChrPlayDefaultMotion` | - | 0 (default motion) | - | int `0x00a79e60` (native_move) |
| `0x5033` | `Native_ChrFadeAlpha` | alpha f, frames int (0 at once) | 0 | - | int `0x00a79ea0` (native) |
| `0x5034` | `Native_ChrSetMotionBlend` | frames int: blend into the next motion | 0 | - | int `0x00a79f40` (native) |
| `0x5035` | `Native_5035` | - | Ch_GetSeqFrame(model) (0x00836af0), 0 without a model | - | int `0x00a79fa0` (native_chr) |
| `0x5036` | `Native_5036` | - | 0x00a7a980(worker, args, stack, 0) | - | int `0x00a7a120` (native_chr) |
| `0x5037` | `Native_5037` | worker int, x var, y var, z var | 0: the model's point 0 (Ch_OptposGet 0x008354f0) into the variables (null ones skipped) | - | int `0x00a7a140` (native_chr) |
| `0x5039` | `Native_ChrSetHitRadiusScale` | scale f | float: Ch_SetThickness(model, scale) (0x0082b880); 0.0 without a model | - | float `0x00a7a340` (native_chr) |
| `0x503a` | `Native_503a` | z f | float: Ch_SetClipZ(model, z) (0x0082b150); 0.0 without a model | - | float `0x00a7a3d0` (native_chr) |
| `0x503b` | `Native_503b` | b, a | ChN_DisposeSystemMGRP(b, a) (0x00836830) when the worker has a model | - | int `0x00a7a450` (native_chr) |
| `0x503c` | `Native_503c` | v int | 0: Ch_DisposeMGRP(v) (0x00836ad0) | - | int `0x00a7a4d0` (native_chr) |
| `0x503d` | `Native_503d` | v int | 0: ChN_DataDispose(v, 1) (0x00824f00) | - | int `0x00a7a520` (native_chr) |
| `0x503e` | `Native_503e` | b int, a int | 0: ChN_DataDispose(b, a) (0x00824f00) | - | int `0x00a7a570` (native_chr) |
| `0x503f` | `Native_503f` | v | Ch_SetSequence(model, v) (0x00837ac0) | - | int `0x00a7a5a0` (native_chr) |
| `0x5040` | `Native_ChrSetMotion2` | motion int | 0 | - | int `0x00a7a610` (native_move) |
| `0x5042` | `Native_ChrPlayMotion2` | as 0x501a | 0 | - | int `0x00a7a780` (native_move) |
| `0x5044` | `Native_5044` | as Native_PlayMotionCommon | 0: Native_PlayMotionCommon(worker, stack, 1, 1) (0x00a7aae0) | - | int `0x00a7a810` (native_chr) |
| `0x5046` | `Native_5046` | - | Native_PlayMotionCommon(worker, stack, 1, 1) | - | int `0x00a7a880` (native_chr) |
| `0x5047` | `Native_5047` | worker int, v int | 0: Ch_SetSequence(that worker's model, v) (0x00837ac0) | - | int `0x00a7a900` (native_chr) |
| `0x5048` | `Native_5048` | - | as 0x5047 | - | int `0x00a7a940` (native_chr) |
| `0x5049` | `Native_5049` | worker | start: the args word = that worker's model, also EAX; poll: 0 while the model in the args word has +0x736 set (or there is none), else 5 | see the poll | start `0x00a785f0`, poll `0x00a78620` (native_chr) |
| `0x504a` | `Native_504a` | worker | as 0x501f for that worker | - | int `0x00a78640` (native_chr) |
| `0x504b` | `Native_504b` | worker | Ch_ResetMotion (0x0082b080) on that worker's model | - | int `0x00a78670` (native_chr) |
| `0x504c` | `Native_504c` | worker int, v int | 0: Ch_SetMotionSpeed(that worker's model, v) (0x0082b380) | - | int `0x00a786a0` (native_chr) |
| `0x504d` | `Native_504d` | v f | Ch_AllSetThicknessEvent(v) (0x0082a8d0); 0.0 | - | float `0x00a786e0` (native_chr) |
| `0x504e` | `Native_ChrLoad` | chr int | int: 0x00829ef0's result | - | int `0x00a79800` (native_move) |
| `0x504f` | `Native_504f` | slot int | int: args +0xc, Ch_DataReadSync(slot)'s result (0x0082a040) | until Ch_DataReadSync(slot) is 0 (poll 1), or 5 with args +8 set | start `0x00a79340`, poll `0x00a79380`, int `0x00a79410` (native_chr) |
| `0x5050` | `Native_5050` | worker int, v int | 0: Ch_MotionSet737(that worker's model, v) (0x00837570) | - | int `0x00a78710` (native_chr) |
| `0x5051` | `Native_ChrGetMotionFrame` | - | int: motion frame (8.8) | - | int `0x00a78750` (native) |
| `0x5052` | `Native_5052` | worker, point, x f, y f, z f | Ch_BindToElemDirectOfs(model, that worker's model, point, x, y, z) (0x008326e0); worker -1 detaches with the offset | - | int `0x00a787e0` (native_chr) |
| `0x5054` | `Native_5054` | part int, on int | int: Ch_ModelSetHide(model, part, on) (0x0082aea0); 0 without a model | - | int `0x00a78960` (native_chr) |
| `0x5056` | `Native_5056` | v int | int: Ch_SetCalcNorm(model, v) (0x00835750); 0 without a model | - | int `0x00a78a80` (native_chr) |
| `0x5057` | `Native_ChrSetBoneUpdate` | worker int, on int | int: previous | - | int `0x00a78af0` (native) |
| `0x5058` | `Native_5058` | v int | 0: Ch_AllSetCalcNorm(v) (0x008325b0) | - | int `0x00a78b70` (native_chr) |
| `0x505a` | `Native_505a` | b, a | Ch_TextureSetClutBase(model, b, a) (0x008313c0), 0 without a model | - | int `0x00a78c60` (native_chr) |
| `0x505c` | `Native_ChrSetNextMotionBlend` | frames int (-1 = 0, 0 = 1) | 0 | - | int `0x00a78d70` (native) |
| `0x505d` | `Native_ChrGetMotion` | - | int: motion id | - | int `0x00a78dd0` (native) |
| `0x505e` | `Native_505e` | - | Ch_GetSp(model) (0x0082ad60), 0.0 without a model | - | float `0x00a78e20` (native_chr) |
| `0x505f` | `Native_505f` | v int | 0: Ch_MotionSetLoop(model, v) (0x00839960) | - | int `0x00a78e90` (native_chr) |
| `0x5060` | `Native_5060` | v | Ch_IdEnableShade(model, v) (0x008340f0) | - | int `0x00a78f00` (native_chr) |
| `0x5061` | `Native_5061` | b int, a int | 0: Ch_MgrpRead(b, a) (0x00836a40) | - | int `0x00a78f80` (native_chr) |
| `0x5063` | `Native_5063` | id int, n int | int: args +0xc, ChN_ReadSystemMGRPsync(id, n)'s result (0x00836a50) | as 0x504f | start `0x00a79160`, poll `0x00a791a0`, int `0x00a79210` (native_chr) |
| `0x5065` | `Native_5065` | v int | 0: 0x00837650(v) (the dbgPrintf left out) | - | int `0x00a79420` (native_chr) |
| `0x5066` | `Native_5066` | n | start: args +0 = n, +0xc = 0; EAX n; poll: 1 once 0x00837670(n) is 0 (the dbgPrintf left out), else 0; int: args +0xc | see the poll | start `0x00a79480`, poll `0x00a794d0`, int `0x00a79540` (native_chr) |
| `0x5067` | `Native_5067` | v | Ch_DrawSetCull(model, v) (0x008300b0) | - | int `0x00a79580` (native_chr) |
| `0x5068` | `Native_WaitCdIdle` | - | 1 when done | until the CD reads are idle | poll `0x00a795d0` (native) |
| `0x5069` | `Native_5069` | v | Ch_SetKeepFps(model, v) (0x00839b50), 0 without a model | - | int `0x00a795e0` (native_chr) |
| `0x506a` | `Native_506a` | v | Ch_PLightSetEnable(model, v) (0x00830ae0) | - | int `0x00a79640` (native_chr) |
| `0x506d` | `Native_506d` | a f, b f | Ch_SetRotX(model, wrap(b degrees), a) (0x008357f0) | - | int `0x00a79a80` (native_chr) |
| `0x506e` | `Native_506e` | v f | Ch_BindSetScale(model, v) (0x00832610) | - | int `0x00a79b30` (native_chr) |
| `0x506f` | `Native_506f` | v int (popped only for a model) | 0: Ch_SetHideCalc(model, v) (0x0082b280) | - | int `0x00a79b90` (native_chr) |
| `0x5071` | `Native_5071` | v | Ch_TextureLoadImage(model, v) (0x00831300) | - | int `0x00a79cc0` (native_chr) |
| `0x5072` | `Native_5072` | - | Ch_IdGetPNum(model) (0x0082adc0), 0 without a model | - | int `0x00a79cf0` (native_chr) |
| `0x5073` | `Native_5073` | v | Ch_SetOverlapHit(model, v) (0x0082b420), popped only for a model | - | int `0x00a79d30` (native_chr) |
| `0x5075` | `Native_ChrGetMotionFrameOf` | worker int | int: its motion frame | - | int `0x00a79dd0` (native) |
| `0x5076` | `Native_5076` | on int (popped only for a model) | 0: Ch_MouthSetAutoTalk(model, on) (0x00834770) | - | int `0x00a79e30` (native_chr) |
| `0x5078` | `Native_5078` | b int, a int | 0: Ch_TextureSetAnim(model, b, a) (0x0083d040) | - | int `0x00a79ef0` (native_chr) |
| `0x5079` | `Native_5079` | on int | 0: Ch_NeckSetEnable(model, on) (0x00834f60) | - | int `0x00a79f70` (native_chr) |
| `0x507a` | `Native_507a` | worker int | 0: Ch_NeckSetTarget(model, that worker's model) (0x00835050) | - | int `0x00a79fc0` (native_chr) |
| `0x507b` | `Native_507b` | x f, y f, z f | Ch_NeckSetTargetPos(model, &[x, y, z]) (0x008350a0) | - | int `0x00a7a000` (native_chr) |
| `0x507c` | `Native_507c` | - | Ch_NeckSetTargetCamera(model) (0x00835080) | - | int `0x00a7a070` (native_chr) |
| `0x507d` | `Native_507d` | b f, a f | degrees: Ch_NeckSetRot(model, wrap(b), wrap(a)) (0x00834ff0) | - | int `0x00a7a090` (native_chr) |
| `0x507e` | `CT_507E_ret` | worker int, on int: mouth flapping | 0 | - | int `0x00a7a210` (native) |
| `0x5080` | `Native_5080` | v f | Ch_SetHiju(model, v) (0x0082b340) | - | int `0x00a7a410` (native_chr) |
| `0x5081` | `Native_5081` | v | Ch_SetChocobo(model, v) (0x0082b130), popped only for a model | - | int `0x00a7a4a0` (native_chr) |
| `0x5082` | `Native_5082` | v | Ch_ClothSetEnable(model, v) (0x0082a930), popped only for a model | - | int `0x00a7a4f0` (native_chr) |
| `0x5083` | `Native_5083` | v | Ch_Vu1SetClipmode(model, v) (0x0082b970), popped only for a model | - | int `0x00a7a540` (native_chr) |
| `0x5084` | `Native_5084` | v f | Ch_NeckSetSpeed(model, v) (0x00835020) | - | int `0x00a7a5d0` (native_chr) |
| `0x5085` | `Native_5085` | v f | Ch_ShadowSetHeight(model, v) (0x00830cb0) | - | int `0x00a7a640` (native_chr) |
| `0x5086` | `Native_5086` | - | Ch_ReserveCalcNorm(model) (0x00835710) | - | int `0x00a7a680` (native_chr) |
| `0x5087` | `Native_ChrRefreshAllBones` | - | 0 | - | int `0x00a7a6a0` (native) |
| `0x5088` | `Native_5088` | v f, n | Ch_SetShadeID(model, v, n) (0x0082b720) | - | int `0x00a7a6b0` (native_chr) |
| `0x5089` | `Native_5089` | - | Native_PlayMotionCommon(worker, stack, 2, 0) | - | int `0x00a7a700` (native_chr) |
| `0x508b` | `Native_508b` | - | the same as 0x5089 | - | int `0x00a7a760` (native_chr) |
| `0x508d` | `Native_508d` | v | Ch_TextureSetAnimTimer(model, v) (0x0083d070) | - | int `0x00a7a7e0` (native_chr) |
| `0x508e` | `Native_508e` | v | Ch_ClothEnableGrav(model, v) (0x008245f0) | - | int `0x00a7a850` (native_chr) |
| `0x508f` | `Native_508f` | v int (popped only for a model) | 0: the model's +0x870 byte = v | - | int `0x00a7a8a0` (native_chr) |
| `0x5090` | `Native_5090` | v | the model's +0x871 byte, popped only for a model | - | int `0x00a7a8d0` (native_chr) |
| `0x6000` | `Native_CamSuspend` | worker int | int: camera handle (channel suspended) | - | int `0x007b8ea0` (native_cam) |
| `0x6001` | `Native_CamResume` | worker int | int: camera handle (channel resumed) | - | int `0x007b8f30` (native_cam) |
| `0x6002` | `Native_CamSetPos` | x f, y f, z f: eye position | 0 | - | int `0x007b9220` (native_cam) |
| `0x6003` | `Native_CamGetPos` | x var, y var, z var: variable references | 0 (the eye position stored) | - | int `0x007b9260` (native_cam) |
| `0x6004` | `Native_CamSetPolar` | theta f (deg), phi f (deg), len f: eye around the reference | 0 | - | int `0x007b92e0` (native_cam) |
| `0x6005` | `Native_Cam_6005` | theta f (deg), phi f (deg), len f: added to the eye's polar position | 0 | - | int `0x007b9320` (native_cam) |
| `0x6006` | `Native_Cam_6006` | a f, b f, angle f (deg), d f, e f, f f: eye; mode -1: (a, b, angle) only, kind 4 (0x007bf330) | 0 | - | int `0x007b9360` (native_cam) |
| `0x6007` | `Native_Cam_6007` | a f, b f, c f, d f, e f, f f: eye, kind 5 (0x007bf420 mode 0) | 0 | - | int `0x007b93a0` (native_cam) |
| `0x6008` | `Native_Cam_6008` | as 0x6007, kind 6 (mode 1) | 0 | - | int `0x007b93c0` (native_cam) |
| `0x600a` | `Native_600a` | - | 0x007bb2b0(..., 1, 0); 0 | - | int `0x007b9440` (native_chr) |
| `0x600f` | `Native_Cam_600f` | - | int: camera handle (0x007bed10(cam, 1), eye half) | - | int `0x007b9520` (native_cam) |
| `0x6010` | `Native_CamMove` | frames int: eye moves to its set position, linear | 0 | no (0x601a waits) | int `0x007b96c0` (native_cam) |
| `0x6011` | `Native_Cam_6011` | frames int: eye move, mode 1, type 1 | 0 | - | int `0x007b9730` (native_cam) |
| `0x6012` | `Native_Cam_6012` | frames int: eye move, type 3 | 0 | - | int `0x007b9800` (native_cam) |
| `0x6013` | `Native_6013` | frames | MsCameraMoveFrame(cam, 1, 1, frames, 3) (0x007bde50); 0 | - | int `0x007b9870` (native_chr) |
| `0x6014` | `Native_CamMoveEased` | accel int, hold int, decel int, curve int: eased eye move | 0 | - | int `0x007b9940` (native_cam) |
| `0x6015` | `Native_CamMoveEased1` | accel int, hold int, decel int, curve int: as 0x6014, mode 1 | 0 | - | int `0x007b9980` (native_cam) |
| `0x6016` | `Native_CamMoveFinish` | - | int: camera handle (eye transition applied now; with magic 0x92 the second call arms Native_BtlRandom) | - | int `0x007b9b40` (native_cam) |
| `0x6017` | `Native_Cam_6017` | a f, b f, c f, d f: eye channel words +0x130..+0x13c (0x007bdde0) | 0 | - | int `0x007b9a00` (native_cam) |
| `0x6019` | `Native_Cam_6019` | - | int: camera handle (0x007bec90(cam, 1)) | - | int `0x007b9ac0` (native_cam) |
| `0x601a` | `Native_CamWait` | - | 5 when done | until the eye half stops moving | poll `0x007b9be0` (native_cam) |
| `0x601b` | `Native_601b` | - | Cam_IsMoving(cam, 1) (0x007bdd20) | - | int `0x007b9ca0` (native_chr) |
| `0x601c` | `Native_Cam_601c` | var int: variable reference, degrees f | int: camera handle (0x007bf520(cam, 1, 1, &var, radians)) | - | int `0x007b8330` (native_cam) |
| `0x601d` | `Native_Cam_601d` | var int, degrees f, y f, x f | int: camera handle (0x007bf600(cam, 1, 1, &var, radians, y, x)) | - | int `0x007b8470` (native_cam) |
| `0x601e` | `Native_601e` | var, deg f | 0x007bf520(cam, 1, 0, &var, deg * pi / 180); the camera | - | int `0x007b83d0` (native_chr) |
| `0x601f` | `Native_Cam_601f` | as 0x601d, mode 0 | int: camera handle | - | int `0x007b8530` (native_cam) |
| `0x6020` | `Native_CamRefSetPos` | x f, y f, z f: reference (look-at) position | 0 | - | int `0x007b9d00` (native_cam) |
| `0x6021` | `Native_Cam_6021` | x var, y var, z var: variable references | 0 (the reference position stored) | - | int `0x007b9d40` (native_cam) |
| `0x6027` | `Native_CamRefSetTarget` | target int, bone int, y f: reference follows a target | 0 | - | int `0x007b9fe0` (native_cam) |
| `0x6028` | `Native_Cam_6028` | five floats: reference constraint, kind 0 | 0 | - | int `0x007ba0e0` (native_cam) |
| `0x6029` | `Native_CamRefAddLineArea` | x1 f, z1 f, x2 f, z2 f, r f: reference kept near a segment | 0 | - | int `0x007ba1d0` (native_cam) |
| `0x602a` | `Native_602a` | - | 0x007bb370(..., 0, 0); 0 | - | int `0x007ba1f0` (native_chr) |
| `0x602b` | `Native_CamRefAddCircleArea` | x f, z f, r f: reference kept by a circle | 0 | - | int `0x007ba2d0` (native_cam) |
| `0x602d` | `Native_CamRefClearAreas` | - | int: camera handle (reference constraints cleared) | - | int `0x007ba2f0` (native_cam) |
| `0x602e` | `Native_CamFocusTimeLinear` | frames int: focus move, linear | 0 | - | int `0x007b7f70` (native_cam) |
| `0x6030` | `Native_Cam_6030` | frames int: focus move, type 3 | 0 | - | int `0x007b8050` (native_cam) |
| `0x6032` | `Native_CamFocusTimeEase` | accel int, hold int, decel int, curve int: eased focus move | 0 | - | int `0x007b8130` (native_cam) |
| `0x6033` | `Native_6033` | - | 0x007ba5b0(worker, args, stack, 0, 1); 0 | - | int `0x007b8150` (native_chr) |
| `0x6034` | `Native_CamFocusCut` | - | int: camera handle (focus jumps to its target) | - | int `0x007b8210` (native_cam) |
| `0x6035` | `Native_Cam_6035` | as 0x6017 for the focus half | 0 | - | int `0x007b8170` (native_cam) |
| `0x6038` | `Native_CamFocusWait` | - | 5 when done | until the focus half stops moving | poll `0x007b8270` (native_cam) |
| `0x603a` | `Native_CamSetRoll` | degrees f: eye roll | int: camera handle | - | int `0x007b9010` (native_cam) |
| `0x603b` | `Native_CamSetScreenDepth` | depth f: screen depth (zoom) | int: camera handle | - | int `0x007b90b0` (native_cam) |
| `0x603c` | `Native_Cam_603c` | angle f (deg), b f, c f, d f: eye, kind 10 (0x007befc0) | 0 | - | int `0x007b9400` (native_cam) |
| `0x603f` | `Native_Cam_603f` | a int, b int, c f (/100): focus (0x007c5c60) | 0 | - | int `0x007b86f0` (native_cam) |
| `0x6040` | `Native_CamEyeOrbitScaled` | actor a int, actor b int, height f, yaw f (deg), pitch f (deg), distance f: eye orbit scaled by actor size | 0 | - | int `0x007b8710` (native_cam) |
| `0x6041` | `Native_CamFocusOrbitScaled` | as 0x6040 for the focus | 0 | - | int `0x007b8730` (native_cam) |
| `0x6042` | `Native_6042` | v | 0x007bdf90(cam, 0, v ^ 0xffff); the camera | - | int `0x007b85f0` (native_chr) |
| `0x6043` | `Native_Cam_6043` | v int | int: camera handle (0x007bdf90(cam, 1, v ^ 0xffff)) | - | int `0x007b8660` (native_cam) |
| `0x6044` | `Native_CamEyeOrbit` | as 0x6040, distance absolute | 0 | - | int `0x007b8750` (native_cam) |
| `0x6045` | `Native_CamFocusOrbit` | as 0x6044 for the focus | 0 | - | int `0x007b8770` (native_cam) |
| `0x6046` | `Native_Cam_6046` | path int (0, 1), a f, b f, c int: eye along a camera path | int: camera handle | - | int `0x007b9d20` (native_cam) |
| `0x6047` | `Native_Cam_6047` | as 0x6046 for the focus | int: camera handle | - | int `0x007b9d60` (native_cam) |
| `0x6048` | `Native_Cam_6048` | - | int: camera handle (the eye path list is cleared) | - | int `0x007b9da0` (native_cam) |
| `0x6049` | `Native_CamPathAddEye` | - | int: camera handle (the eye appended to path list 1) | - | int `0x007b9e30` (native_cam) |
| `0x604a` | `Native_Cam_604a` | - | int: camera handle (the focus path list is cleared) | - | int `0x007b9eb0` (native_cam) |
| `0x604b` | `Native_CamPathAddFocus` | - | int: camera handle (the focus appended to path list 0) | - | int `0x007b9f40` (native_cam) |
| `0x604c` | `Native_604c` | - | 0x007badb0(..., 1, 3); 0 | - | int `0x007b8790` (native_chr) |
| `0x604d` | `Native_Cam_604d` | as 0x6040, mode 4 | 0 | - | int `0x007b87b0` (native_cam) |
| `0x604e` | `Native_604e` | - | six: 0x007bc930(f, 1), 0x007bc890(f, e, d, c, b, a, 0); 0 | - | int `0x007b8ad0` (native_chr) |
| `0x604f` | `Native_604f` | v | 0x007bc950(v, 0, v) | - | int `0x007b8b80` (native_chr) |
| `0x6050` | `Native_6050` | - | five: 0x007bc750(e, d, c, b, a); 0 | - | int `0x007b8ba0` (native_chr) |
| `0x6051` | `Native_6051` | b, a | MsScreenLinkSet(b, 1, a) (0x007bbdd0) | - | int `0x007b8c30` (native_chr) |
| `0x6052` | `Native_6052` | b, a | MsScreenLinkSet(b, 0, a) | - | int `0x007b8c00` (native_chr) |
| `0x6055` | `Native_6055` | d, c, b, a | MsScreenMoveAcc(d, 0, c, b, a) (0x007bbeb0) | - | int `0x007b8cc0` (native_chr) |
| `0x6058` | `Native_6058` | - | as 0x6055 with mode 1 | - | int `0x007b8d70` (native_chr) |
| `0x6059` | `Native_Cam_6059` | path int (0, 1 battle spline lists, else a variable), a int | 0 (focus along the path, from 0) | - | int `0x007ba020` (native_cam) |
| `0x605a` | `Native_Cam_605a` | path int, t f, a int | 0 (focus along the path, from t) | - | int `0x007ba100` (native_cam) |
| `0x605b` | `Native_Cam_605b` | path int, a int | 0 (eye along the path) | - | int `0x007ba210` (native_cam) |
| `0x605c` | `Native_Cam_605c` | a int, b f, c int, d int, e int: focus interpolation (0x007c4fc0 flags 1: curve byte a, value b, frames c/d, e) | int: camera handle | - | int `0x007b8e80` (native_cam) |
| `0x605d` | `Native_Cam_605d` | as 0x605c for the eye | int: camera handle | - | int `0x007b8f10` (native_cam) |
| `0x605e` | `Native_605e` | - | 0x007bb840(..., 1); 0 | - | int `0x007b9420` (native_chr) |
| `0x605f` | `Native_Cam_605f` | - | 0 (0x007c4f50(cam, 0)) | - | int `0x007b97a0` (native_cam) |
| `0x6060` | `Native_Cam_6060` | - | 0 (0x007c4f50(cam, 1)) | - | int `0x007b98e0` (native_cam) |
| `0x6061` | `Native_6061` | v | MsScreenResetShake(v) (0x007c56c0) | - | int `0x007b9960` (native_chr) |
| `0x6062` | `Native_Cam_6062` | - | 5 when done | until 0x007c4f10(cam, 0) is 0 | poll `0x007b9a20` (native_cam) |
| `0x6063` | `Native_Cam_6063` | - | 5 when done | until 0x007c4f10(cam, 1) is 0 | poll `0x007b99a0` (native_cam) |
| `0x6064` | `Native_Cam_6064` | id int: camera channel (0-2) | 5 when done | until that channel is idle (0x007c5680) | start `0x007b9aa0`, poll `0x007b9b20` (native_cam) |
| `0x6066` | `Native_6066` | - | the shake with (0, 2) | - | int `0x007b8fa0` (native_chr) |
| `0x6067` | `Native_6067` | - | the shake with (1, 2) | - | int `0x007b8fc0` (native_chr) |
| `0x6069` | `Native_6069` | - | MsAtelCameraShake(worker, args, stack, 0, 3) (0x007bb8c0) | - | int `0x007b8ff0` (native_chr) |
| `0x606b` | `Native_Cam_606b` | chan int (< 3), flags int, b f, c int, d int, e int: channel interpolation (0x007c56f0, kind 3) | 0 | - | int `0x007b94a0` (native_cam) |
| `0x606c` | `Native_606c` | - | the shake with (0, 4) | - | int `0x007b9150` (native_chr) |
| `0x606e` | `Native_606e` | - | 0x007bb840(..., 4); 0 | - | int `0x007b94e0` (native_chr) |
| `0x6071` | `Native_Cam_6071` | as 0x606b, kind 5 | 0 | - | int `0x007b9580` (native_cam) |
| `0x6074` | `Native_6074` | - | 0x007bb840(worker, args, stack, 6); 0 | - | int `0x007b9610` (native_chr) |
| `0x6076` | `Native_Cam_6076` | - | 0 (the battle cameras freed) | - | int `0x007b8fe0` (native_cam) |
| `0x6077` | `Native_6077` | - | float: the camera's +0xc90 angle (0x007bd500) in degrees (MsNearRag(0, v) * 180 / pi) | - | float `0x007b9190` (native_chr) |
| `0x6078` | `Native_6078` | - | jumps to 0x007bd510(worker, args, stack) (ST0) | - | float `0x007b91f0` (native_chr) |
| `0x607b` | `Native_607b` | screen | start: into the args word; EAX it; poll: 5 once MsScreenMoveCheck(screen, 0) (0x007bbfc0) is 0 | see the poll | start `0x007b8e40`, poll `0x007b8e60` (native_chr) |
| `0x607d` | `Native_Cam_607d` | v int | int: camera handle (0x007bcdd0(cam, v)) | - | int `0x007b8840` (native_cam) |
| `0x607e` | `Native_CamSetDepthMaskMode` | flags | 0x007bcf50(cam, flags); the camera | - | int `0x007b88b0` (native_chr) |
| `0x607f` | `Native_CamSetDepthMask` | plane, z | 0x007bf280(cam, plane, z); the camera | - | int `0x007b8920` (native_chr) |
| `0x6080` | `Native_BtlRandom` | b int (bit 0 picks draw table 14 or 15) | int: the draw; with the latch armed 2 for odd, 0 for even | - | int `0x007b89a0` (native_cam) |
| `0x6081` | `Native_6081` | - | the shake with (0, 7) | - | int `0x007b92c0` (native_chr) |
| `0x6082` | `Native_6082` | - | the shake with (1, 7) | - | int `0x007b9300` (native_chr) |
| `0x6083` | `Native_Cam_6083` | as 0x605c, flags 8 | int: camera handle | - | int `0x007b9340` (native_cam) |
| `0x6084` | `Native_Cam_6084` | as 0x605d, flags 8 | int: camera handle | - | int `0x007b9380` (native_cam) |
| `0x6087` | `Native_Cam_6087` | - | int: camera handle (0x007beae0(cam, 0)) | - | int `0x007b8a10` (native_cam) |
| `0x6088` | `Native_Cam_6088` | - | int: camera handle (0x007beae0(cam, 1)) | - | int `0x007b8a70` (native_cam) |
| `0x7000` | `Native_7000` | - | 0 (actor +0xdf2 = 0) | - | int `0x007a8620` (native_btl) |
| `0x7002` | `Native_7002` | a int, b int | the start's result; poll returns 1 at once | no | start `0x007a3530`, poll `0x007a35c0` (native_btl) |
| `0x7003` | `Native_7003` | a int, b int | 0 (0x00795eb0(actor, 2, 0, -1, b, a)) | - | int `0x007a3d90` (native_btl) |
| `0x7006` | `Native_7006` | a int, b int | 0 (as 0x7003 with 1, when actor +0x41d) | - | int `0x007a3f50` (native_btl) |
| `0x7007` | `Native_ActorPlayMotion` | as 0x007a8fc0(worker, args, stack, 0) | its result | - | int `0x007a35a0` (native_move) |
| `0x7008` | `Native_ActorWaitMotion` | - | 5 when done | until the actor's motion ends (no actor/model: at once) | poll `0x007a3980` (native_btl) |
| `0x7009` | `Native_7009` | b int | 0 | - | int `0x007a3b20` (native_btl) |
| `0x700a` | `Native_700a` | b int, f f | 0 (actor +0x50c = b, +0x538 = f) | - | int `0x007a4100` (native_btl) |
| `0x700b` | `Native_BtlUseCommand` | targets int (selector), command int | 1 if used, else 0 (actor cannot act) | - | int `0x007a44b0` (native_btl) |
| `0x700c` | `Native_700c` | - | 0 (0x007a9150(worker, args, stack, 0.0)) | - | int `0x007a4310` (native_btl) |
| `0x700d` | `Native_700d` | mode int, k int | int: 0x007a87f0(&to, &from, actor, abs k, 0); with the actor locked (+0x44e), 1 for a negative k else 0 | - | int `0x007a63f0` (native_btl) |
| `0x700f` | `Native_GetActorProperty` | selector int, property int | int: MsGetChrStatInfo's value | - | int `0x007a4d50` (native_btl) |
| `0x7010` | `Native_BtlSelectTarget` | a int, b int, c int, d int | int: the selected target | - | int `0x007a5af0` (native_btl) |
| `0x7012` | `Native_7012` | - | 0 | - | int `0x007a29a0` (native_btl) |
| `0x7014` | `Native_7014` | - | int: the word 0x0112c8d8 | - | int `0x007a4430` (native_btl) |
| `0x7015` | `Native_DebugPrint` | v int | 0 (debug print) | - | int `0x007a4910` (native_btl) |
| `0x7016` | `Native_ActorQueueMotion` | motion int | 0 (queued on the actor) | - | int `0x007a2a20` (native_btl) |
| `0x7017` | `Native_7017` | a int, b int | 0 | - | int `0x007a4050` (native_btl) |
| `0x7018` | `Native_SetActorsProperty` | selector int, property int, value int | int: MsSetChrStatInfo's result | - | int `0x007a50c0` (native_btl) |
| `0x7019` | `Native_7019` | - | int: 0x007acc90(actor) | - | int `0x007a62d0` (native_btl) |
| `0x701a` | `Native_701a` | a int, b int | int: 0x0078cd30(actor, a, b) | - | int `0x007a6390` (native_btl) |
| `0x701b` | `Native_701b` | targets int, v int | 0: 0x007ac940(actor, v, MsGetChrBit(actor, targets, 0, 1)) | - | int `0x007a66a0` (native_btl) |
| `0x701c` | `Native_701c` | v int | 0 (actor +0x420 = v) | - | int `0x007a59d0` (native_btl) |
| `0x701d` | `Native_701d` | - | int: actor +0x420 (signed) | - | int `0x007a5bf0` (native_btl) |
| `0x701e` | `Native_BtlCountActorsInBoth` | a int, b int: selectors | int: actors both pick | - | int `0x007a6870` (native_btl) |
| `0x701f` | `Native_701f` | v int | 0: the actor's +0x3f1 = v | - | int `0x007a5f80` (native_btl) |
| `0x7021` | `Native_ActorGetId` | selector int | int: the picked actor's id, -1 for none | - | int `0x007a6a80` (native_btl) |
| `0x7022` | `Native_7022` | v int | 0: the byte 0x0112c9dc = v | - | int `0x007a4cd0` (native_btl) |
| `0x7023` | `Native_7023` | selector int | int: ftol of the mean of MsSetMove(i, 1) (0x007a9fc0) over the picked actors (f32 sum and mean) | - | int `0x007a4940` (native_btl) |
| `0x7024` | `Native_BtlGetBattleId` | - | int: MsGetBattleScene() | - | int `0x007a4e90` (native_btl) |
| `0x7025` | `Native_7025` | d int, c int, b int, a int | int: out of MsActTargetSearch(actor, d, c, b, 4, a, 0, &out) (0x007acd40), whose result goes to 0x00796430(actor, r) | - | int `0x007a5cf0` (native_btl) |
| `0x7026` | `Native_7026` | v int | 0 (actor +0x44f = v) | - | int `0x007a5d80` (native_btl) |
| `0x7028` | `Native_7028` | x f, y f, z f: the actor's +0x3a0..+0x3a8 and its model's 0x0082b560(model, 7, ..) | 0 | - | int `0x007a6970` (native_btl) |
| `0x7029` | `Native_7029` | b int | 0 (actor +0x505 = b & 1) | - | int `0x007a3fb0` (native_btl) |
| `0x702a` | `Native_702a` | - | int: actor +0x508 | - | int `0x007a5bc0` (native_btl) |
| `0x702b` | `Native_702b` | - | int: -1 while the actor's model is busy, else 0 | - | int `0x007a3c50` (native_btl) |
| `0x702c` | `Native_702c` | mode int, p8 f (speed), p7 f, p6 f (acceleration), p5 f, p4 f, p3 f, p2 f, p1 f | int: 0x007a87f0(&to, &from, actor, 0x10, 1) (0 without an actor) | - | int `0x007a45c0` (native_btl) |
| `0x702d` | `Native_702d` | - | 0 | - | int `0x007a4e60` (native_btl) |
| `0x702e` | `Native_702e` | v int: target | float: distance to the target (MsGetTargetDist) | - | float `0x007a6d70` (native_btl) |
| `0x702f` | `Native_702f` | keep int | 0: resets the actor (+0x508 = 0, MsStatusEffectClear, 0x0079ea90, the system motion again unless +0xdce, MsSetChrVisibleAppear(actor, 0)); with keep +0xdc8 = 0 | - | int `0x007a5140` (native_btl) |
| `0x7030` | `Native_7030` | - | int: actor +0x412 | - | int `0x007a5090` (native_btl) |
| `0x7031` | `Native_7031` | - | 0 (actor +0x432 = 1) | - | int `0x007a2d00` (native_btl) |
| `0x7032` | `Native_7032` | selector int, angle f (degrees) | 0 | - | int `0x007a6f70` (native_btl) |
| `0x7033` | `Native_7033` | b int | 0 (actor +0x32 = b & 1) | - | int `0x007a3cc0` (native_btl) |
| `0x7034` | `Native_7034` | v int | int: v | - | int `0x007a5330` (native_btl) |
| `0x7035` | `Native_7035` | - | int: the byte 0x0112c9f1 | - | int `0x007a53a0` (native_btl) |
| `0x7036` | `Native_7036` | a f, b f, n int | 0: MsSetChrTransparentVisible(actor, a, b, n, 1) (0x00796550) | - | int `0x007a7180` (native_btl) |
| `0x7037` | `Native_7037` | b int, a int | int: 0x007ac840(actor, a, b, 1) | - | int `0x007a6d30` (native_btl) |
| `0x7038` | `Native_7038` | b int, a int | int: 0x007acb40(actor, a, b) | - | int `0x007a6dd0` (native_btl) |
| `0x7039` | `Native_7039` | - | 0 | - | int `0x007a2b10` (native_btl) |
| `0x703a` | `Native_703a` | v int | 0: the actor's +0x3fc (u16) = v through f32, truncated | - | int `0x007a7360` (native_btl) |
| `0x703b` | `CT_703B_ret_setCommandDisabled` | selector int, a int, b int | int: actors handled | - | int `0x007a6ef0` (native_btl) |
| `0x703c` | `Native_703c` | v int | the request handle; poll 0x007a8700 | until it ends | start `0x007a5540`, poll `0x007a5680` (native_btl) |
| `0x703f` | `Native_BtlRunEvent` | event int, a int | the event handle; poll its 0x007a8700 | until the event ends | start `0x007a5df0`, poll `0x007a5ec0` (native_btl) |
| `0x7040` | `Native_7040` | v int | int: 0x0079e8c0(actor, v); 0 without an actor | - | int `0x007a5610` (native_btl) |
| `0x7041` | `Native_7041` | - | 0 | - | int `0x007a56d0` (native_btl) |
| `0x7046` | `Native_7046` | v int | 0: MsMoveSplineSet(v) (0x007ab140) | - | int `0x007a5220` (native_btl) |
| `0x7047` | `Native_7047` | i int, a int | 0 | - | int `0x007a5270` (native_btl) |
| `0x7048` | `Native_7048` | i int, x f, y f, z f | 0 | - | int `0x007a53f0` (native_btl) |
| `0x7049` | `Native_7049` | d int, c int, f f, a int | 0: 0x007ab010(d, actor, c, f, a) | - | int `0x007a55a0` (native_btl) |
| `0x704a` | `Native_704a` | selector int | int: picked actors with +0x415 set | - | int `0x007a58d0` (native_btl) |
| `0x704b` | `Native_704b` | selector int, motion int, b int | the args word: 1 when MsSetMotion(i, motion, b, 0, 1, worker, &out) (0x007ab360) set out for any picked actor | until 0x007a8700(args) is done | start `0x007a7640`, poll `0x007a7850` (native_btl) |
| `0x7050` | `Native_7050` | selector int | the args word: MsSetChrVisibleBtl2Event(worker, MsGetChrBit(actor, selector, 0, 1), -1) (0x007a89c0) | until 0x007a8700(args) is done | start `0x007a7400`, poll `0x007a74f0` (native_btl) |
| `0x7051` | `Native_7051` | - | 5 when done | until actor +0xe12 is 0 | poll `0x007a5890` (native_btl) |
| `0x7052` | `Native_7052` | a int: selector, b int: target selector, v int | 0 | - | int `0x007a7e10` (native_btl) |
| `0x7053` | `Native_7053` | f f | 0: 0x007a9150(worker, args, stack, f) | - | int `0x007a4470` (native_btl) |
| `0x7054` | `Native_7054` | to int, from int, point int | 0: the from actor's model point (Ch_OptposGet 0x008354f0) through 0x0079e1b0(0, &v) into MsSetChrPosReset(to, &v) (0x00796220) | - | int `0x007a7fe0` (native_btl) |
| `0x7055` | `Native_7055` | b int | 0 | - | int `0x007a3eb0` (native_btl) |
| `0x7057` | `Native_7057` | target int, f f (dropped), b int, a int | 0: MsSetChrDirTarget(actor, target, 0, -1, b, a) (0x00795eb0) | - | int `0x007a4220` (native_btl) |
| `0x7058` | `Native_7058` | selector int, motion int | int: 0 when the picked actor plays it, else -1 | - | int `0x007a8160` (native_btl) |
| `0x7059` | `Native_7059` | v int: target | float: distance (MsGetTargetDist mode 1) | - | float `0x007a6e40` (native_btl) |
| `0x705a` | `Native_705a` | targets int, command int | 1, 0 without an actor | - | int `0x007a49f0` (native_btl) |
| `0x705d` | `Native_ActorSetFlag36` | a int, b int (dropped) | int: the actor index (+0x36 = 1) | - | int `0x007a51e0` (native_btl) |
| `0x7062` | `Native_7062` | a int, b int | int: 0x0079eba0(actor, a, b, 1) | - | int `0x007a4580` (native_btl) |
| `0x7063` | `Native_7063` | - | 5 when the actor's +0xe13 is clear, else 0 | until +0xe13 is clear | poll `0x007a5dc0` (native_btl) |
| `0x706b` | `Native_706b` | selector int, a int, b int | 0 | - | int `0x007a84c0` (native_btl) |
| `0x706e` | `Native_706e` | selector int, voice int | the args word: 1 when MsSetVoice(i, voice, 1, worker, &out) (0x007a0720) set out for any picked actor | until 0x007a8700(args) is done | start `0x007a7020`, poll `0x007a71f0` (native_btl) |
| `0x706f` | `Native_706f` | as 0x007a8fc0(worker, args, stack, 1) | its result | - | int `0x007a36c0` (native_btl) |
| `0x7071` | `Native_7071` | - | 0: 0x0089bc60(0) | - | int `0x007a70b0` (native_btl) |
| `0x7078` | `Native_7078` | selector int, b int, a int | int: MsGetComInfo(picked, b, a) (0x0078cd30) | - | int `0x007a6540` (native_btl) |
| `0x707a` | `Native_707a` | selector int | int: 0x0079a920's result | - | int `0x007a2a80` (native_btl) |
| `0x707b` | `Native_707b` | selector int, k int | int: 0x0079fd40(picked, k) | - | int `0x007a7470` (native_btl) |
| `0x707d` | `Native_707d` | k int: battle flag, v int | int: v | - | int `0x007a81f0` (native_btl) |
| `0x707e` | `Native_707e` | k int: battle flag + 1 | int: the flag (0 for k without one) | - | int `0x007a27e0` (native_btl) |
| `0x707f` | `Native_707f` | selector int | 0 | - | int `0x007a5e50` (native_btl) |
| `0x7082` | `Native_7082` | b int, a int | int: 0x0079eb10(actor, b, 1, a, 1) | - | int `0x007a41e0` (native_btl) |
| `0x7083` | `Native_7083` | n int, _ int | 0: for n > 0 with +0xe15 set and no +0xea8, op_et_weapon_unit(model, 0, ...) (0x007fc9b0) into +0xea8 | - | int `0x007a42a0` (native_btl) |
| `0x7084` | `Native_7084` | - | 0 | - | int `0x007a4440` (native_btl) |
| `0x7085` | `Native_7085` | - | 0 (as 0x700c, record +0x418 = 1) | - | int `0x007a43a0` (native_btl) |
| `0x7086` | `Native_7086` | - | 0 | - | int `0x007a2ce0` (native_btl) |
| `0x7087` | `Native_7087` | selector int, b int, a int | int: 0x0079fcf0(picked, b, a); 0 for none | - | int `0x007a7880` (native_btl) |
| `0x7088` | `Native_7088` | selector int, se int | int: MsRegSEplay(picked, se) (0x007a0100); 0 for none | - | int `0x007a79c0` (native_btl) |
| `0x708a` | `Native_708a` | v int | 0: 0x00782b40(v) | - | int `0x007a3710` (native_btl) |
| `0x708b` | `Native_708b` | v int | int: 0x00782940(v) | - | int `0x007a3900` (native_btl) |
| `0x708c` | `Native_708c` | a int, b int | int: 0x00782ba0(a, b) | - | int `0x007a3920` (native_btl) |
| `0x708e` | `Native_708e` | - | int: the actor's signed +0x508 after MsMotionPosCheck (0x007aa100; +0x4f0 = 10.0, +0x34 = 1 around it when +0x608 and +0x505 are set) | - | int `0x007a5c20` (native_btl) |
| `0x708f` | `Native_708f` | v int | int: 0x007a8720(actor, v, 1) | - | int `0x007a6700` (native_btl) |
| `0x7091` | `Native_7091` | to int, from int (selectors) | 0: the 40-byte name +0x540 and +0xdd5 copied from one picked actor to the other | - | int `0x007a77c0` (native_btl) |
| `0x7093` | `Native_7093` | - | int: 0x00791780(actor, record) | - | int `0x007a60f0` (native_btl) |
| `0x7096` | `Native_7096` | - | int: the signed byte 0x0112c9fa | - | int `0x007a7b30` (native_btl) |
| `0x7097` | `Native_7097` | v int | the handle; poll 0x007a8700 | until it ends | start `0x007a5780`, poll `0x007a58c0` (native_btl) |
| `0x709d` | `Native_709d` | - | 5 when there is no actor or its +0x425 is clear, else 0 | until +0x425 is clear | poll `0x007a3ae0` (native_btl) |
| `0x709e` | `Native_709e` | selector int, a int, b int | int: 0x007b74d0(picked, a, b) | - | int `0x007a2fe0` (native_btl) |
| `0x70a1` | `Native_ActorGetSlotRef` | selector int | int: the picked actor's +0xc, -6 - slot, or 0xff | - | int `0x007a6b40` (native_btl) |
| `0x70a2` | `Native_70a2` | v int | the start's result (poll not ported) | its own poll | start `0x007a7440`, poll `0x007a7500` (native_btl) |
| `0x70a4` | `Native_70a4` | - | 0: 0x0079cca0() | - | int `0x007a7630` (native_btl) |
| `0x70a5` | `Native_70a5` | x f, y f, z f | 0 | - | int `0x007a76f0` (native_btl) |
| `0x70a8` | `Native_70a8` | selector int, property int, value f | 0 | - | int `0x007a57e0` (native_btl) |
| `0x70aa` | `Native_GetSelfProperty` | property int | int: the actor's own property | - | int `0x007a4ea0` (native_btl) |
| `0x70ab` | `Native_SetActorProperty` | property int, value int | int: 0x007b4b70(actor, property, -13, value) | - | int `0x007a5360` (native_move) |
| `0x70ac` | `Native_70ac` | i int | float: actor float property i | - | float `0x007a5650` (native_btl) |
| `0x70b0` | `Native_70b0` | selector int | int: the steps the picked actor needs (from +0x40e, or the +0x4cc/+0x4d0 ramp over the distance) | - | int `0x007a4f80` (native_btl) |
| `0x70b1` | `Native_70b1` | v int | debug (mod-tool candidate): dbgPrintf("##### %d : %x : c%02d #####", v, v, actor) (a bare ret in this build); 0 | - | int `0x007a4a60` (not ported) |
| `0x70b2` | `Native_SetSelfFloatProperty` | property int, value f | 0 (MsSetChrMotionInfo) | - | int `0x007a5930` (native_btl) |
| `0x70b5` | `Native_70b5` | - | 0 | - | int `0x007a2f40` (native_btl) |
| `0x70bb` | `Native_70bb` | v int | 0: the actor's +0x415 = 0x10, +0x417 = 0, +0x452 = 0, +0x45a = v (u16) | - | int `0x007a4cf0` (native_btl) |
| `0x70be` | `Native_70be` | selector int | float: the picked actor's direction (MsGetChrDir 0x00794d20) in degrees (MsNearRag(0, d) * 180 / pi) | - | float `0x007a3780` (native_btl) |
| `0x70c3` | `Native_70c3` | selector int | float: MsGetChrTargetDir(picked, 0) (0x007958c0) in degrees, as 0x70be | - | float `0x007a3650` (native_btl) |
| `0x70c9` | `Native_70c9` | selector int, k int | int: 0x0079fdf0(picked, k) | - | int `0x007a7740` (native_btl) |
| `0x70cc` | `Native_70cc` | selector int | int: the set (actor +0xdc0 = it) | - | int `0x007a7cb0` (native_btl) |
| `0x70cd` | `Native_70cd` | selector int | int: the set (actor +0xdc0 |= it) | - | int `0x007a7d60` (native_btl) |
| `0x70ce` | `Native_70ce` | selector int | int: the set (cleared from actor +0xdc0) | - | int `0x007a7f20` (native_btl) |
| `0x70d1` | `Native_70d1` | selector int | int: the picked actor's +0xe12 | - | int `0x007a5cb0` (native_btl) |
| `0x70d5` | `Native_70d5` | selector int | int: the picked index (actor +0x6d1) | - | int `0x007a7fa0` (native_btl) |
| `0x70d7` | `Native_70d7` | selector int, motion int | 0: MsDamageSetMotion(picked, motion, picked != actor) (0x0078ca20) unless the picked actor's +0x422 actor has +0xdcc | - | int `0x007a30f0` (native_btl) |
| `0x70dc` | `Native_70dc` | selector int, i int | 0: 40 bytes of the name table *0x0112a9b8 (entry MsGetSaveConfigHiragana() + 2i) into the picked actor's +0x540 | - | int `0x007a7a10` (native_btl) |
| `0x70e0` | `Native_70e0` | - | int: 1 when the actor and its partner can act | - | int `0x007a8450` (native_btl) |
| `0x70e2` | `Native_70e2` | selector int, v int | 0 | - | int `0x007a32a0` (native_btl) |
| `0x70ea` | `Native_70ea` | v int | int: v (MsSet01130838(v), 0x007b2b40) | - | int `0x007a7b10` (native_btl) |
| `0x70ed` | `Native_70ed` | b int, a int | int: MsSaveItemUse(b, a) (0x00790550) | - | int `0x007a2ca0` (native_btl) |
| `0x70ef` | `Native_70ef` | selector int, signal int | 0: MsEffectSendSignal(picked, signal) (0x0079e4f0) | - | int `0x007a64f0` (native_btl) |
| `0x7104` | `Native_7104` | a int, b int, c int | int: 0x0079d2a0(a, b, c) | - | int `0x007a6600` (native_btl) |
| `0x7106` | `CT_7106_ret_doesChrKnowCommand` | selector int, command int | int: MsGetSaveCommand(picked, command) (0x00785020) | - | int `0x007a30a0` (native_btl) |
| `0x7107` | `Native_7107` | n int | int: ftol((MsDirTurn(chr, yaw +0x514[+0x511]) - +0x490) / pi * n) (0x007a9650; f32 steps) | - | int `0x007a6800` (native_btl) |
| `0x7108` | `Native_7108` | a int, b int | 0 | - | int `0x007a40a0` (native_btl) |
| `0x7109` | `Native_7109` | selector int, reset int, f f | int: the number of picked actors; each with a model: +0x407 = 1, with reset 0x00796070(i, 0), +0x505 = 1, +0x50c = 0; +0x43d = 0, +0x538 = f | - | int `0x007a3df0` (native_btl) |
| `0x7115` | `Native_7115` | - | 0: MsSetChrDirLeave(actor) (0x00795e70) | - | int `0x007a4560` (native_btl) |
| `0x7117` | `Native_7117` | targets int, command int | 1: Battle_QueueCommand(actor, command, MsGetChrBit(actor, targets, 0, 1, 1, 0x40)) after MsMotionPosCheck when +0x3f9 is set; 0 without an actor | - | int `0x007a4b00` (native_btl) |
| `0x7118` | `Native_7118` | v int | int: v; the byte 0x0112ca11 = v | - | int `0x007a81d0` (native_btl) |
| `0x711c` | `Native_711c` | v int | int: v; the u16 0x0112c94e = v | - | int `0x007a8530` (native_btl) |
| `0x711d` | `Native_711d` | height f | 0: Ch_ShadowSetHeight(MsGetChrHandle(actor), height) for a model (0x00795080, 0x00830cb0) | - | int `0x007a4330` (native_btl) |
| `0x8000` | `Native_FieldSetMaskBit` | group int, on int: map draw group (TkSetMapGroupDrawStatus) | int: group | - | int `0x0091c530` (native) |
| `0x8001` | `Native_8001` | on int: rendering on/off (yiRenderEnable bit 0x10, 0x00643df0) | int: on | - | int `0x0091c5b0` (native_map) |
| `0x8002` | `Native_FpRun` | fp int: field program, restart int (0 ends it) | int: restart | - | int `0x0091c670` (native) |
| `0x8003` | `Native_FpRunMode2` | fp int | 1 | - | int `0x0091c7a0` (native) |
| `0x8004` | `Native_MapSpriteWait` | sprite int: map sprite object | 5 when done | until the object is done | start `0x0091af60`, poll `0x0091afa0` (native_map) |
| `0x8005` | `Native_MapSpriteWaitEnd` | sprite int, keep int | 5 when done | until the object is done (state 2 counts unless keep) | start `0x0091c7e0`, poll `0x0091c860` (native_map) |
| `0x8006` | `Native_8006` | n int | int | no (the poll is done at once) | start `0x0091aff0`, poll `0x0091b040` (native_map) |
| `0x8007` | `Native_8007` | - | 1 | - | int `0x0091b050` (native_map) |
| `0x8008` | `Native_8008` | - | 1 | - | int `0x0091b060` (native_map) |
| `0x8009` | `Native_FpBind` | fp int, bone int: bind to the worker | 1 | - | int `0x0091b070` (native_map) |
| `0x800a` | `Native_FpBindPos` | fp int, x f, y f, z f: bind to a position | 1 | - | int `0x0091b0a0` (native_map) |
| `0x800b` | `Native_FpUnbind` | fp int (<= 0 all) | 1 | - | int `0x0091b130` (native_map) |
| `0x800c` | `Native_800c` | on int: field programs shown (pppFpSetDisp) | int: on != 0 | - | int `0x0091b160` (native_map) |
| `0x800d` | `Native_FpShow` | fp int, show int | int: show | - | int `0x0091c740` (native) |
| `0x800f` | `Native_MapPartShow` | part int: map part | 1 | - | int `0x0091b1a0` (native_map) |
| `0x8010` | `Native_MapPartHide` | part int | 1 | - | int `0x0091b1e0` (native_map) |
| `0x8011` | `Native_8011` | a int, b int, c int | 1 | - | int `0x0091b200` (native_map) |
| `0x8014` | `Native_8014` | part int, v f: map part +0x14 (Yn_MapPartSet14) | 1 | - | int `0x0091b280` (native_map) |
| `0x801d` | `Native_801d` | part int, frame int: map part animation frame | 1 | - | int `0x0091b400` (native_map) |
| `0x801e` | `Native_801e` | part int: animation flag 0 on | 1 | - | int `0x0091b430` (native_map) |
| `0x801f` | `Native_801f` | part int: animation flag 0 off | 1 | - | int `0x0091b450` (native_map) |
| `0x8020` | `Native_8020` | part int: animation on | 1 | - | int `0x0091b470` (native_map) |
| `0x8021` | `Native_8021` | part int: animation off | 1 | - | int `0x0091b490` (native_map) |
| `0x8022` | `Native_8022` | part int, bits int: animation mode (bits 8-11) | 1 | - | int `0x0091b4b0` (native_map) |
| `0x8026` | `Native_8026` | a int, b f, c int, d int, e int: scene +0xc0..+0xd0 (Yn_SceneSetC0) | 1 | - | int `0x0091b550` (native_map) |
| `0x802c` | `Native_802c` | - | 1 | - | int `0x0091b680` (native_map) |
| `0x802d` | `Native_802d` | - | 1 | - | int `0x0091b690` (native_map) |
| `0x802e` | `Native_802e` | part int, rate int: animation rate | 1 | - | int `0x0091b6a0` (native_map) |
| `0x802f` | `Native_802f` | part int: animation to its last frame | 1 | - | int `0x0091b6d0` (native_map) |
| `0x8030` | `Native_8030` | a int, b int | 1 | - | int `0x0091b6f0` (native_map) |
| `0x8032` | `Native_8032` | limit f: guide map limit | 1 | - | int `0x0091b740` (native_map) |
| `0x8035` | `Native_8035` | group int, on int: the group's field programs restarted (Fp_RestartGroup) or ended (Fp_EndGroup) | int: on | - | int `0x0091b790` (native_map) |
| `0x8036` | `Native_8036` | id int, on int: 0x00729440 or 0x00729400 | int: on | - | int `0x0091b7d0` (native_map) |
| `0x8037` | `Native_FogSetParams` | a f, b f, c f: fog parameters | 1 | - | int `0x0091b810` (native_map) |
| `0x8038` | `Native_FogSetParam0` | x f: fog parameter 0 | 1 | - | int `0x0091b8b0` (native_map) |
| `0x8039` | `Native_FogGetParam0` | - | float: fog parameter 0 | - | float `0x0091b930` (native_map) |
| `0x803a` | `Native_FogSetParam1` | x f: fog parameter 1 | 1 | - | int `0x0091b960` (native_map) |
| `0x803b` | `Native_FogGetParam1` | - | float: fog parameter 1 | - | float `0x0091b9e0` (native_map) |
| `0x803c` | `Native_FogSetParam2` | x f: fog parameter 2 | 1 | - | int `0x0091ba10` (native_map) |
| `0x803d` | `Native_FogGetParam2` | - | float: fog parameter 2 | - | float `0x0091baa0` (native_map) |
| `0x803e` | `Native_FogSetColor` | r int, g int, b int: fog colour | 1 | - | int `0x0091bad0` (native_map) |
| `0x803f` | `Native_FogSetRed` | r int | 1 | - | int `0x0091bbb0` (native_map) |
| `0x8040` | `Native_FogSetGreen` | g int | 1 | - | int `0x0091bc80` (native_map) |
| `0x8041` | `Native_FogSetBlue` | b int | 1 | - | int `0x0091bd50` (native_map) |
| `0x8042` | `Native_FogGetRed` | - | int: fog red | - | int `0x0091be20` (native_map) |
| `0x8043` | `Native_FogGetGreen` | - | int: fog green | - | int `0x0091be50` (native_map) |
| `0x8044` | `Native_FogGetBlue` | - | int: fog blue | - | int `0x0091be80` (native_map) |
| `0x8045` | `Native_BackSetColor` | r int, g int, b int: back colour | 1 | - | int `0x0091beb0` (native_map) |
| `0x8049` | `Native_BackGetRed` | - | int: back red | - | int `0x0091c000` (native_map) |
| `0x804a` | `Native_BackGetGreen` | - | int: back green | - | int `0x0091c030` (native_map) |
| `0x804b` | `Native_BackGetBlue` | - | int: back blue | - | int `0x0091c060` (native_map) |
| `0x804f` | `Native_804f` | part int, s f, frames int: map part scaled to (s, s) over frames | 1 | - | int `0x0091c150` (native_map) |
| `0x8059` | `Native_8059` | a int, b int: pppFpSetStickMap(b, a) | 1 | - | int `0x0091c3c0` (native_map) |
| `0x805b` | `Native_FpSetStickPos` | n int, x f, y f, z f | 1 | - | int `0x0091c410` (native_map) |
| `0x805c` | `Native_805c` | on int | 1 | - | int `0x0091c470` (native_map) |
| `0x805d` | `Native_805d` | - | 1 | - | int `0x0091c4a0` (native_map) |
| `0x805e` | `Native_805e` | map int: guide map number (Yn_GuideMapSetMapNo) | 1 | - | int `0x0091c4d0` (native_map) |
| `0x805f` | `Native_805f` | x int | 1 | - | int `0x0091c4f0` (native_map) |
| `0x8060` | `Native_8060` | x int | 1 | - | int `0x0091c510` (native_map) |
| `0x8066` | `Native_8066` | fp int: pppFpStopStatus | 1 | - | int `0x0091c7c0` (native_map) |
| `0x8067` | `Native_BltzSetObjModels` | worker a int, worker b int, mode int, unused int (Blitzball) | 1 | - | int `0x0091c810` (native_map) |
| `0x806a` | `Native_806a` | v int: scene +0x104 | 1 | - | int `0x0091af80` (native_map) |
| `0x806b` | `Native_806b` | on int | 1 | - | int `0x0091afc0` (native_map) |
| `0xb000` | `Native_b000` | movie int, flags int | start: args = flags, movie; graphicInitFMVPlayer(movie, 0), 0x006427c0(1), flags 0x2000 pauses at the end; int: 0, movie_start_prog(movie, flags) (0x0076f080) | - (the poll returns 5) | start `0x0076e910`, poll `0x0076e980`, int `0x0076e990` (native_chr) |
| `0xb001` | `Native_b001` | - | int: 0; the movie camera off (movie_have_camera), movie -1 | while graphicIsVideoPlaying() or 0x00641ce0(); then graphicDestroyFmv() | start `0x0076e9b0`, poll `0x0076e9c0`, int `0x0076e9f0` (native_chr) |
| `0xb002` | `Native_b002` | - | debug (mod-tool candidate): a dbgPrintf line only; 0 | - | int `0x0076ea30` (not ported) |
| `0xb003` | `Native_b003` | - | int: graphicGetCurrentFrame() (0x006410d0, a jump) | - | int `0x0076ea40` (native_chr) |
| `0xb004` | `Native_b004` | - | start: graphicSetPauseVideoAtEndFrame(0); poll: 0x00643e50(), the movie camera off, movie -1, g_WaitFramesByTime 0, 1; int: graphicDestroyFmv(), 0 | - | start `0x0076ea50`, poll `0x0076ea60`, int `0x0076eac0` (native_chr) |
| `0xb009` | `Native_b009` | - | int: 0; g_WaitFramesByTime 1, 0x006427c0(0), 0x0076edc0() | until 0x00641ce0() is 0 and 0x00645dc0() is not | start `0x0076eb30`, poll `0x0076eb40`, int `0x0076eb60` (native_chr) |
| `0xb00a` | `Native_b00a` | movie, flags | int: 0xc024 (0x00878c60, poll), 0xb00a (0x0076ec10, int) and the like: 1; int: 0; start: the movie = movie, graphicInitFMVPlayer(movie, flags & 0x80); when it starts, 0x006427c0(0), g_WaitFramesByTime, the done flag and 0x0112a00c = 1. EAX the last call's; poll: 5 unless 0x0112a00c is set while 0x00641ce0() or not 0x00645dc0(); int: 0 | see the poll | start `0x0076eb80`, poll `0x0076ebe0`, int `0x0076ec10`, int `0x0076ec10`, int `0x0076ec10` (native_chr) |
| `0xb00b` | `Native_b00b` | - | poll: 5 when 0x00645e50() is not 0; int: graphicDestroyFmv() for movie 0x34 (the dbgPrintf left out), then 0xb001's int; start: a bare ret | see the poll | start `0x0076ec20`, poll `0x0076ec30`, int `0x0076ec40` (native_chr) |
| `0xb00c` | `Native_b00c` | - | jumps to 0x00641140 (after a dbgPrintf, left out) | - | int `0x0076ec90` (native_chr) |
| `0xb00d` | `Native_b00d` | - | the movie done flag = 0; 0 | - | int `0x0076ecb0` (native_chr) |
| `0xc002` | `Native_c002` | on | MsSetSaveConfigHiragana(on != 0) (0x00785f40); 0 | - | int `0x008784e0` (native_chr) |
| `0xc003` | `Native_c003` | map | AtelMapLoad(map) (0x0086dca0) | - | int `0x00878510` (native_chr) |
| `0xc007` | `Native_c007` | map, index | TkSetStartMapIndex(map, index) (0x0088ebf0); map | - | int `0x00878600` (native_chr) |
| `0xc009` | `Native_c009` | mode | AtelSetEventMode(mode) (0x00873e40); mode | - | int `0x00878630` (native_chr) |
| `0xc00b` | `Native_c00b` | - | debug (mod-tool candidate): in a debug build yiLoadBattleFieldParticleNo(v); 1 in release | - | int `0x00878690` (not ported) |
| `0xc00c` | `Native_c00c` | - | debug (mod-tool candidate): in a debug build (TkIsDebug) yiLoadFieldParticleNo(v); 1 in release | - | int `0x00878650` (not ported) |
| `0xc00d` | `Native_c00d` | on | MsSetSaveConfigCreditName(on != 0) (0x00785e90); on != 0 | - | int `0x008786d0` (native_chr) |
| `0xc013` | `Native_c013` | - | 1 when MsGetSaveConfigHiragana() (0x007851f0) is 1 | - | int `0x008787b0` (native_chr) |
| `0xc014` | `Native_c014` | - | 1 when 0x007851b0() is not 0 | - | int `0x008787c0` (native_chr) |
| `0xc018` | `Native_c018` | - | debug (mod-tool candidate): in a debug build SqsSetProjectBase(&var); 1 in release | - | int `0x008788a0` (not ported) |
| `0xc022` | `Native_c022` | - | debug (mod-tool candidate): in a debug build Sg_EnableDrawGui(v); 1 in release | - | int `0x00878b00` (not ported) |
| `0xc024` | `Native_c024` | _, v | start: the battle end (AtelEventSetUpForBattleMusic, AtelEventRemoveForBattle for the event jump map, AtelBattleEndCallback(1, v, map)); its EAX; poll: 0xc024 (0x00878c60, poll), 0xb00a (0x0076ec10, int) and the like: 1; poll: 1 | see the poll | start `0x00878c10`, poll `0x00878c60`, poll `0x00878c60` (native_chr) |
| `0xc025` | `Native_c025` | msg int | int: 1 when the message text (Msg_Text) fits and is copied over the player's name (MsGetSaveChrName(0), shorter than MsGetSaveChrNameSize(0)), else 0 | - | int `0x00878c70` (native_chr) |
| `0xc028` | `Native_c028` | - | debug (mod-tool candidate): in a debug build 0x0088e790(0xa004 + i) for 26 i; 1 | - | int `0x00878d60` (not ported) |
| `0xc02a` | `Native_c02a` | - | debug (mod-tool candidate): in a debug build TkSetOutputEncount(v) and AtelOutputEncountStart(); v | - | int `0x00878e10` (not ported) |
| `0xc02c` | `Native_c02c` | v int | int: the popped value; int: the popped value | - | int `0x00878e60` (native_chr) |
| `0xc02f` | `Native_c02f` | on, v f | the current ctrl work's +0x208 = v and +2 bit 0 = on != 0; EAX on != 0 | - | int `0x00878e80` (native_chr) |
| `0xc030` | `Native_c030` | - | TkIsDebug(), then the current ctrl work's +0x1f4 | - | int `0x00878ed0` (native_chr) |
| `0xc031` | `Native_c031` | v | 0x0086dcd0(v) | - | int `0x00878530` (native_chr) |
| `0xc036` | `Native_c036` | on | pppFpSetDisp(on != 0) (0x00729050); on != 0 | - | int `0x00878f70` (native_chr) |
| `0xc03b` | `Native_c03b` | - | debug (mod-tool candidate): MsDebugFullItem (0x007849d0), the debug cheat that gives every item; 1 | - | int `0x008790b0` (not ported) |
| `0xc03c` | `Native_c03c` | - | debug (mod-tool candidate): MsDebugFullCommand (0x007848b0), the debug cheat that teaches every command; 1 | - | int `0x008790c0` (not ported) |
| `0xc051` | `Native_c051` | on | TkSetTGS(on != 0) (0x0088ec10); on != 0 | - | int `0x008793b0` (native_chr) |
| `0xc052` | `Native_c052` | - | jumps to 0x0088d6d0 | - | int `0x008793e0` (native_chr) |
| `0xc053` | `Native_c053` | - | four popped and dropped; 0 | - | int `0x008793f0` (native_chr) |
| `0xc054` | `Native_c054` | n | TkMalloc(n) (0x0088e650) | - | int `0x008781b0` (native_chr) |
| `0xc055` | `Native_c055` | p | TkFree(p) (0x0088d520); 1 | - | int `0x008781d0` (native_chr) |
| `0xc056` | `Native_c056` | - | debug (mod-tool candidate): TkCdRead(group, index, adrs) with a dbgPrintf (a debug file read); 1 | - | int `0x00878230` (not ported) |
| `0xc057` | `Native_c057` | - | debug (mod-tool candidate): copies 0x68c0 bytes from the address + 0x40 into g_SaveData and TkFlushSyncH ('copy to saveram'); 1 | - | int `0x008781f0` (not ported) |
| `0xc058` | `Native_c058` | v | AtelGetRealVarAdrs(worker, v) | - | int `0x00878280` (native_chr) |
| `0xc05b` | `Native_c05b` | v int | debug (mod-tool candidate): AtelPs2DebugString2("DEBUG:%d/%d" of the worker id +0x2e and v) (0x00873c80); v | - | int `0x00878330` (not ported) |

### Verification

`crates/ffx-oracle/tests/script.rs` runs the original `Script_Run` and the
Rust one on two copies of each snapshot and compares the result and every
byte written: every worker as it stands, every entry point of every actor
for several frames (about 400,000 instructions in the battle snapshot),
400 random programs over 97 opcodes with arguments kept in range (in the
battle on ctrl work 2, whose header tables take the signals' other path),
invalid opcodes and breakpoints. `natives_match_the_original` calls every
Rust native on each actor with small arguments on its stack, and
`synthetic_script_matches_the_original` runs a hand-built worker (block,
header, variables, constants, a counting loop, the end) in an image-only
oracle, so the interpreter is checked without a snapshot too. Two oracle stubs make the comparison possible:
the run lock and the CRT and Win32 imports natives reach are short x86
stubs written into `int3` padding of `.text` (the test asserts every one
found room), and two natives read stack words nothing wrote, which both
oracles zero: `0x007bb6a0` (`Native_CamSetPos`) hands `Cam_SetPos` a
vector whose `w` is uninitialised, as does `Light_Set` (`0x00836520`) to
`Sg_LightSetColor`/`Sg_LightSetDir`, and `0x007bab90`
(`Native_CamRefSetTarget`) to `0x007beeb0`. `printf` is stubbed (it
returns 0) for the debug lines. Before each native call the natives tests
zero the top 64 KiB of both stacks: the test calls a native, the Rust
version calls that native's helper one frame higher, so a word the
original reads before writing would hold different leftovers (the camera
path start `0x007bcb80` uses a spline position and tangent that
`TkMtCalcSpline` leaves unwritten for paths of fewer than three points). `native_special_cases_match_the_original`
covers what small random arguments miss: `Native_MsgSetPos`'s moved
windows, the voice waits' countdowns and debug counting, `Native_MsgClose`
on each window state, `Native_CamSetScreenDepth` in debug mode. `move_and_message_cases_match_the_original` runs the move and turn starts on
every mode with prepared blocks, the message opens and the three waits on
each window state, `Msg_ShowSetup`, the time-counted frame wait, the
camera paths from the battle lists with 0, 3 and more points,
`Native_CamWait` in debug mode and `Native_BtlRandom` with its latch. The natives tests leave the
emulated VU0 registers (`0x00c8f788`-`0x00c8f890`) out of the memory
comparison: battle helpers load vectors with an unwritten `w` into them
(`0x007a9150`). Three more uninitialised-stack stubs zero the frames of
`MsGetTargetDist` (`0x007a9a40`), `Native_702c` (`0x007a45c0`) and
`MsSetChrDirTarget` (`0x00795eb0`): each copies a position or angle that
`0x00794d20` and friends leave unwritten for some actors into the actor
(`+0x490`). Once the Rust battle natives ran in `Script_Run`, the
leftovers there came from different frames than the original's. A fifth stub zeroes `0x007a8720`'s two
locals (`Native_708f`): the angle `0x00794d20` leaves unwritten. The
natives tests and `orig` fail when an original an integer port stands for
leaves a float in `ST0` (a dropped float result). In the game those words hold whatever
earlier calls left at that depth, so the Rust interpreter (whose frames
differ) can store different garbage there; nothing reads it back as far as
the tests reach.

### Plan

1. Done: the accessors, the dispatch loop, every opcode, native dispatch,
   the signals (request families, `AtelExecReturn`, the waits, opcodes
   `79`/`7a`): `Script_Run` calls no original script code but natives.
2. Done: every native the shipped scripts call (rounds 1-13; the
   group-0 table at `0x00c50050` has 180 more entries no shipped script
   calls) but the debug ones, which are marked. Now: the VM's other
   helpers, by blocker count (`tools/worklist.py --module script`).
3. Then a `Natives` implementation with no original code behind it, so
   the interpreter runs outside the game.

## Open questions

- The meaning of most native functions.
- Opcode `40` and what the `+0x2e4` table of opcode `7a` drives.
- The block header words `0x04`–`0x2c` and the tables they point to.
