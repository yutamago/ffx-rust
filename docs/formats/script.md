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
start in the dump and runs in Rust only through the dispatch.

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

#### The ported natives

Every native with a Rust handler, by ID: the arguments in push order
(the last pushed is popped first; `int`, `f` a float, `var` a variable
reference), the result (`EAX` for an integer handler, `ST0` for a float
one; a start/poll pair's poll returns 5, or for some 1, when done and 0 to
be called again next frame) and what a poll waits for. The modders' Lua
API is generated from this table. `-`: none.

| ID | Name | Arguments (push order) | Result | Waits | Handlers (module) |
|---|---|---|---|---|---|
| `0x0013` | `CT_0013_setPosition` | x f, y f, z f: the worker's position | 0 | - | int `0x0085faa0` (native) |
| `0x001a` | `Native_MoveWait` | - | 5 when done | until the move block's mode (+2 & 0x7f) is 0 (the move started by MoveStart ends) | poll `0x008579a0` (native) |
| `0x001b` | `Native_TurnWait` | - | 5 when done | until the turn block's mode (+2 & 0x1f) is 0 | poll `0x00857b30` (native) |
| `0x001c` | `Native_MoveSetFrames` | elapsed int, frames int: the move block's timer (+0xc, +8) | float: elapsed | - | float `0x00855f10` (native) |
| `0x001d` | `Native_TurnSetFrames` | elapsed int, frames int: the turn block's timer (frames <= 0 become 1) | float: elapsed | - | float `0x008560d0` (native) |
| `0x0028` | `Native_TurnSetYaw` | yaw f (radians) | float: the wrapped yaw (turn target +0x20, remaining +0x2c) | - | float `0x00859640` (native) |
| `0x0029` | `Native_TurnSetPitch` | angle f (radians, pitch) | float: the wrapped angle (+0x24, remaining +0x30) | - | float `0x00859880` (native) |
| `0x0033` | `Native_GetWorkerId` | worker int (-1 = self) | int: its id (+0x2e) | - | int `0x0085ba50` (native) |
| `0x0034` | `Native_EnableTrigger` | kind int: trigger bit (0 talk) | int: kind (worker +0xab gets the bit) | - | int `0x0085c060` (native) |
| `0x0035` | `Native_DisableTrigger` | kind int | 0 (clears worker +0xab's bit) | - | int `0x0085c280` (native) |
| `0x0038` | `Native_GetWorkerX` | worker int | float: its x | - | float `0x0085bb80` (native) |
| `0x0039` | `Native_GetWorkerY` | worker int | float: its y | - | float `0x0085bd40` (native) |
| `0x003a` | `Native_GetWorkerZ` | worker int | float: its z | - | float `0x0085bf10` (native) |
| `0x003f` | `Native_TurnGetField38` | - | float: the turn block's +0x38 | - | float `0x0085c590` (native) |
| `0x0046` | `Native_PadPressed` | - | int: buttons pressed this frame, remapped | - | int `0x0085d9b0` (native) |
| `0x0056` | `Native_TriggerSetActive` | on int | 1 for trigger workers (types 2, 3, 5, 6: +0x34 bit 7 = on), else 0 | - | int `0x0085fbf0` (native) |
| `0x005c` | `Native_TriggerSetActive2` | on int | as 0x0056 | - | int `0x008600c0` (native) |
| `0x005d` | `Native_ControlEnable` | - | 1 (player control on) | - | int `0x008566b0` (native) |
| `0x005e` | `Native_ControlDisable` | - | 0 (player control off) | - | int `0x008567c0` (native) |
| `0x005f` | `Native_WaitForever` | - | 0 | forever (until the entry is interrupted) | poll `0x0085c420` (native) |
| `0x0064` | `Native_MsgSetText` | window int, msg int: message index in the event's table | 0 (window text and flags set) | - | int `0x00857870` (native_msg) |
| `0x0065` | `Native_MsgSetPos` | window int, x int, y int, anchor int | 0 (three windows of maps 0x17, 0xab, 0xd4 are moved) | - | int `0x008580c0` (native_msg) |
| `0x0066` | `Native_MsgSetStyle` | window int, style int | int: window | - | int `0x00858360` (native) |
| `0x006b` | `Native_MsgClose` | window int | int: window (closes it) | - | int `0x00859060` (native_msg) |
| `0x0077` | `Native_WorkerStopMove` | worker int | 0 (stops its move) | - | int `0x0085b620` (native) |
| `0x0078` | `Native_WorkerStopTurn` | worker int | 0 (stops its turn) | - | int `0x0085b950` (native) |
| `0x008f` | `Native_MsgIsReady` | window int | 1 when closed, in state 4, or open with its text done; else 0 | - | int `0x0085de10` (native) |
| `0x0097` | `Native_MsgGetState` | window int | int: state (0 closed, 1 opening, 2 open, 3 closing, 4) | - | int `0x0085fa80` (native) |
| `0x009d` | `Native_MsgSetFlags` | window int, flags int | int: window | - | int `0x008584d0` (native) |
| `0x009e` | `Native_MsgSetValue` | window int, slot int, type int, value int: inline text variable | int: window | - | int `0x00855c70` (native_msg) |
| `0x00a6` | `Native_Random` | n int | int: random 0..n-1 | - | int `0x00857400` (native) |
| `0x00bb` | `Native_SetMapFadeFrames` | frames int: fade length for the next map jump | int: frames | - | int `0x008596b0` (native) |
| `0x00bc` | `Native_SetMapFadeType` | type int: fade kind for the next map jump (0, 1) | int: type | - | int `0x00859590` (native) |
| `0x00c5` | `Native_SetTurnMotions` | m0 int, m1 int, m2 int, m3 int: turn motions | 0 | - | int `0x0085a4e0` (native) |
| `0x00c6` | `Native_GetTurnKind` | - | int: bit 0 remaining yaw > 0, bit 1 at least the half-turn angle | - | int `0x0085a670` (native) |
| `0x00c7` | `Native_GetTurnMotion` | kind int (from 0x00c6) | int: the motion 0x00c5 set for it | - | int `0x0085abb0` (native) |
| `0x00d7` | `Native_VoiceStop` | - | 0 (stops the voice) | - | int `0x0085e090` (native) |
| `0x00d8` | `Native_VoiceIsPlaying` | - | 1 while the voice plays, else 0 | no (scripts loop on it) | int `0x0085e1d0` (native_msg) |
| `0x00d9` | `Native_SoundWait_Poll` | - | 1 when done | until the voice has stopped (or the voice-sync countdown runs out) | poll `0x0085e4a0` (native_msg) |
| `0x00df` | `Native_SfxPlay` | sfx int | int: sfx | - | int `0x0085f310` (native) |
| `0x00fd` | `Native_SfxPlayEx` | sfx int, pan int, volume int (0-127) | int: sfx | - | int `0x0085ff30` (native) |
| `0x0116` | `Native_WorkerSetFlag0200` | worker int | int: worker (+0x36: 0x400 cleared, 0x200 set) | - | int `0x0085b9b0` (native) |
| `0x011b` | `Native_SpriteShow` | sprite int (0-127), data var: variable reference | int: sprite | - | int `0x0085c370` (native) |
| `0x011c` | `Native_SpriteHide` | sprite int | int: sprite | - | int `0x0085c4c0` (native) |
| `0x0120` | `Native_SpriteSetScale` | sprite int, sx f, sy f (1.0 normal) | int: sprite | - | int `0x0085cde0` (native) |
| `0x0129` | `Native_SpriteSetColor` | sprite int, r int, b int, g int | int: sprite | - | int `0x0085c760` (native) |
| `0x013f` | `Native_MacroDicText` | dict int, entry int | int: text pointer | - | int `0x00856a00` (native_msg) |
| `0x0142` | `Native_BitIndex` | mask int, n int | int: index of the nth set bit, 0 if fewer | - | int `0x008578c0` (native_msg) |
| `0x0148` | `Native_MinimapSetMarker` | marker int (0 default), worker int | int: the stored value | - | int `0x008583b0` (native) |
| `0x0171` | `Native_CharRestoreHp` | chr int: party character | int (HP restored to the maximum) | - | int `0x0085c680` (native) |
| `0x0172` | `Native_CharRestoreMp` | chr int | int (MP restored to the maximum) | - | int `0x0085c860` (native) |
| `0x019c` | `Native_AlwaysTrue019c` | - | 1 | - | int `0x00856630` (native) |
| `0x01a5` | `Native_01a5` | a int, b int | 1 | - | int `0x008574c0` (native_map) |
| `0x01d1` | `Native_WaitFrames_01d1` | frames int | 1 when done | for frames frames (at once when skipping) | start `0x0085fd20`, poll `0x0085ff70` (native) |
| `0x01e4` | `Native_01e4` | x f, y f, z f, n int | int: the map module's result | - | int `0x00858c90` (native_map) |
| `0x01e6` | `Native_01e6` | n int, x f, y f | int: n | - | int `0x008591d0` (native_map) |
| `0x0241` | `CT_0241_ret` | on int: field state bit 3 | int: on != 0 | - | int `0x00856ad0` (native) |
| `0x5000` | `Native_ChrSetMotion_5000` | motion int | 0 | - | int `0x00a78770` (native) |
| `0x5003` | `Native_ChrWaitMotion_5003` | - | 5 when done | until the character's motion has ended (+0x736) | start `0x00a787d0`, poll `0x00a788a0` (native) |
| `0x5005` | `Native_ChrReleaseMotion` | - | 0 (automatic motions take over) | - | int `0x00a78940` (native) |
| `0x5008` | `Native_ChrHide` | hide int (0, 1) | 0 | - | int `0x00a78ac0` (native) |
| `0x5009` | `Native_ChrSetPushWeight` | weight f (push weight) | float: previous weight | - | float `0x00a78b30` (native) |
| `0x500e` | `Native_ChrSetGroundFollow` | on int: stick to the ground | 0 | - | int `0x00a78cb0` (native) |
| `0x500f` | `Native_ChrSetBodyHit` | on int: collide with characters | 0 | - | int `0x00a78da0` (native) |
| `0x5016` | `Native_ChrSetOffset` | x f, y f, z f: model offset | 0 | - | int `0x00a790b0` (native) |
| `0x5017` | `Native_ChrSetGroundMode` | mode int (0 clamp, 1 snap, 2 float) | 0 | - | int `0x00a79130` (native) |
| `0x501b` | `Native_ChrSetMotion_501b` | motion int | 0 | - | int `0x00a792d0` (native) |
| `0x501e` | `Native_ChrWaitMotion_501e` | - | 5 when done | until the scripted motion has ended | start `0x00a79370`, poll `0x00a793c0` (native) |
| `0x5020` | `Native_ChrReleaseMotion2` | - | 0 (as 0x5005) | - | int `0x00a794b0` (native) |
| `0x5021` | `Native_ChrSetMotionSpeed` | speed int (8.8, 256 = 1x) | 0 | - | int `0x00a79510` (native) |
| `0x5025` | `Native_ChrSetAutoMotion` | slot int, motion int (-1 default) | 0 | - | int `0x00a79670` (native) |
| `0x5029` | `Native_ChrSetShadowMode` | mode int | int: Ch_SetShadowMode's result | - | int `0x00a79a40` (native) |
| `0x502a` | `Native_ChrSetDistanceCull` | on int | int: previous | - | int `0x00a79af0` (native) |
| `0x5033` | `Native_ChrFadeAlpha` | alpha f, frames int (0 at once) | 0 | - | int `0x00a79ea0` (native) |
| `0x5034` | `Native_ChrSetMotionBlend` | frames int: blend into the next motion | 0 | - | int `0x00a79f40` (native) |
| `0x5051` | `Native_ChrGetMotionFrame` | - | int: motion frame (8.8) | - | int `0x00a78750` (native) |
| `0x5057` | `Native_ChrSetBoneUpdate` | worker int, on int | int: previous | - | int `0x00a78af0` (native) |
| `0x505c` | `Native_ChrSetNextMotionBlend` | frames int (-1 = 0, 0 = 1) | 0 | - | int `0x00a78d70` (native) |
| `0x505d` | `Native_ChrGetMotion` | - | int: motion id | - | int `0x00a78dd0` (native) |
| `0x5068` | `Native_WaitCdIdle` | - | 1 when done | until the CD reads are idle | poll `0x00a795d0` (native) |
| `0x5075` | `Native_ChrGetMotionFrameOf` | worker int | int: its motion frame | - | int `0x00a79dd0` (native) |
| `0x507e` | `CT_507E_ret` | worker int, on int: mouth flapping | 0 | - | int `0x00a7a210` (native) |
| `0x5087` | `Native_ChrRefreshAllBones` | - | 0 | - | int `0x00a7a6a0` (native) |
| `0x6000` | `Native_CamSuspend` | worker int | int: camera handle (channel suspended) | - | int `0x007b8ea0` (native_cam) |
| `0x6001` | `Native_CamResume` | worker int | int: camera handle (channel resumed) | - | int `0x007b8f30` (native_cam) |
| `0x6002` | `Native_CamSetPos` | x f, y f, z f: eye position | 0 | - | int `0x007b9220` (native_cam) |
| `0x6003` | `Native_CamGetPos` | x var, y var, z var: variable references | 0 (the eye position stored) | - | int `0x007b9260` (native_cam) |
| `0x6004` | `Native_CamSetPolar` | theta f (deg), phi f (deg), len f: eye around the reference | 0 | - | int `0x007b92e0` (native_cam) |
| `0x6005` | `Native_Cam_6005` | theta f (deg), phi f (deg), len f: added to the eye's polar position | 0 | - | int `0x007b9320` (native_cam) |
| `0x6006` | `Native_Cam_6006` | six floats a..f: eye half; the first three as theta, phi (deg), len (helper 0x007bb410 mode -1) | 0 | - | int `0x007b9360` (native_cam) |
| `0x6007` | `Native_Cam_6007` | six floats: as 0x6006, mode 0 | 0 | - | int `0x007b93a0` (native_cam) |
| `0x6008` | `Native_Cam_6008` | six floats: as 0x6006, mode 1 | 0 | - | int `0x007b93c0` (native_cam) |
| `0x600f` | `Native_Cam_600f` | - | int: camera handle (0x007bed10(cam, 1), eye half) | - | int `0x007b9520` (native_cam) |
| `0x6010` | `Native_CamMove` | frames int: eye moves to its set position, linear | 0 | no (0x601a waits) | int `0x007b96c0` (native_cam) |
| `0x6011` | `Native_Cam_6011` | frames int: eye move, mode 1, type 1 | 0 | - | int `0x007b9730` (native_cam) |
| `0x6012` | `Native_Cam_6012` | frames int: eye move, type 3 | 0 | - | int `0x007b9800` (native_cam) |
| `0x6014` | `Native_CamMoveEased` | accel int, hold int, decel int, curve int: eased eye move | 0 | - | int `0x007b9940` (native_cam) |
| `0x6015` | `Native_CamMoveEased1` | accel int, hold int, decel int, curve int: as 0x6014, mode 1 | 0 | - | int `0x007b9980` (native_cam) |
| `0x6017` | `Native_Cam_6017` | four floats: eye half (0x007bdde0) | 0 | - | int `0x007b9a00` (native_cam) |
| `0x6019` | `Native_Cam_6019` | - | int: camera handle (0x007bec90(cam, 1)) | - | int `0x007b9ac0` (native_cam) |
| `0x6020` | `Native_CamRefSetPos` | x f, y f, z f: reference (look-at) position | 0 | - | int `0x007b9d00` (native_cam) |
| `0x6021` | `Native_Cam_6021` | x var, y var, z var: variable references | 0 (the reference position stored) | - | int `0x007b9d40` (native_cam) |
| `0x6027` | `Native_CamRefSetTarget` | target int, bone int, y f: reference follows a target | 0 | - | int `0x007b9fe0` (native_cam) |
| `0x6028` | `Native_Cam_6028` | five floats: reference constraint, kind 0 | 0 | - | int `0x007ba0e0` (native_cam) |
| `0x6029` | `Native_CamRefAddLineArea` | x1 f, z1 f, x2 f, z2 f, r f: reference kept near a segment | 0 | - | int `0x007ba1d0` (native_cam) |
| `0x602b` | `Native_CamRefAddCircleArea` | x f, z f, r f: reference kept by a circle | 0 | - | int `0x007ba2d0` (native_cam) |
| `0x602d` | `Native_CamRefClearAreas` | - | int: camera handle (reference constraints cleared) | - | int `0x007ba2f0` (native_cam) |
| `0x602e` | `Native_CamFocusTimeLinear` | frames int: focus move, linear | 0 | - | int `0x007b7f70` (native_cam) |
| `0x6030` | `Native_Cam_6030` | frames int: focus move, type 3 | 0 | - | int `0x007b8050` (native_cam) |
| `0x6032` | `Native_CamFocusTimeEase` | accel int, hold int, decel int, curve int: eased focus move | 0 | - | int `0x007b8130` (native_cam) |
| `0x6034` | `Native_CamFocusCut` | - | int: camera handle (focus jumps to its target) | - | int `0x007b8210` (native_cam) |
| `0x6035` | `Native_Cam_6035` | four floats: focus half (0x007bdde0) | 0 | - | int `0x007b8170` (native_cam) |
| `0x6038` | `Native_CamFocusWait` | - | 5 when done | until the focus half stops moving | poll `0x007b8270` (native_cam) |
| `0x603a` | `Native_CamSetRoll` | degrees f: eye roll | int: camera handle | - | int `0x007b9010` (native_cam) |
| `0x603b` | `Native_CamSetScreenDepth` | depth f: screen depth (zoom) | int: camera handle | - | int `0x007b90b0` (native_cam) |
| `0x603c` | `Native_Cam_603c` | angle f (deg), three floats: eye half (0x007befc0) | 0 | - | int `0x007b9400` (native_cam) |
| `0x603f` | `Native_Cam_603f` | a int, b int, c f (/100): focus (0x007c5c60) | 0 | - | int `0x007b86f0` (native_cam) |
| `0x6040` | `Native_CamEyeOrbitScaled` | actor a int, actor b int, height f, yaw f (deg), pitch f (deg), distance f: eye orbit scaled by actor size | 0 | - | int `0x007b8710` (native_cam) |
| `0x6041` | `Native_CamFocusOrbitScaled` | as 0x6040 for the focus | 0 | - | int `0x007b8730` (native_cam) |
| `0x6043` | `Native_Cam_6043` | v int | int: camera handle (0x007bdf90(cam, 1, v ^ 0xffff)) | - | int `0x007b8660` (native_cam) |
| `0x6044` | `Native_CamEyeOrbit` | as 0x6040, distance absolute | 0 | - | int `0x007b8750` (native_cam) |
| `0x6045` | `Native_CamFocusOrbit` | as 0x6044 for the focus | 0 | - | int `0x007b8770` (native_cam) |
| `0x6046` | `Native_Cam_6046` | path int (0, 1), a f, b f, c int: eye along a camera path | int: camera handle | - | int `0x007b9d20` (native_cam) |
| `0x6047` | `Native_Cam_6047` | as 0x6046 for the focus | int: camera handle | - | int `0x007b9d60` (native_cam) |
| `0x6048` | `Native_Cam_6048` | - | int: camera handle (the eye path list is cleared) | - | int `0x007b9da0` (native_cam) |
| `0x6049` | `Native_CamPathAddEye` | - | int: camera handle (the eye appended to path list 1) | - | int `0x007b9e30` (native_cam) |
| `0x604a` | `Native_Cam_604a` | - | int: camera handle (the focus path list is cleared) | - | int `0x007b9eb0` (native_cam) |
| `0x604b` | `Native_CamPathAddFocus` | - | int: camera handle (the focus appended to path list 0) | - | int `0x007b9f40` (native_cam) |
| `0x604d` | `Native_Cam_604d` | as 0x6040, mode 4 | 0 | - | int `0x007b87b0` (native_cam) |
| `0x605c` | `Native_Cam_605c` | a int, b f, c int, d int, e int: focus (0x007c4fc0, 1) | int: camera handle | - | int `0x007b8e80` (native_cam) |
| `0x605d` | `Native_Cam_605d` | as 0x605c for the eye | int: camera handle | - | int `0x007b8f10` (native_cam) |
| `0x605f` | `Native_Cam_605f` | - | 0 (0x007c4f50(cam, 0)) | - | int `0x007b97a0` (native_cam) |
| `0x6060` | `Native_Cam_6060` | - | 0 (0x007c4f50(cam, 1)) | - | int `0x007b98e0` (native_cam) |
| `0x6062` | `Native_Cam_6062` | - | 5 when done | until 0x007c4f10(cam, 0) is 0 | poll `0x007b9a20` (native_cam) |
| `0x6063` | `Native_Cam_6063` | - | 5 when done | until 0x007c4f10(cam, 1) is 0 | poll `0x007b99a0` (native_cam) |
| `0x6064` | `Native_Cam_6064` | id int: camera channel (0-2) | 5 when done | until that channel is idle (0x007c5680) | start `0x007b9aa0`, poll `0x007b9b20` (native_cam) |
| `0x606b` | `Native_Cam_606b` | a int, b int, c f, d int, e int, f int: (0x007c56f0, 3) | 0 | - | int `0x007b94a0` (native_cam) |
| `0x6071` | `Native_Cam_6071` | as 0x606b, kind 5 | 0 | - | int `0x007b9580` (native_cam) |
| `0x6076` | `Native_Cam_6076` | - | 0 (the battle cameras freed) | - | int `0x007b8fe0` (native_cam) |
| `0x607d` | `Native_Cam_607d` | v int | int: camera handle (0x007bcdd0(cam, v)) | - | int `0x007b8840` (native_cam) |
| `0x6083` | `Native_Cam_6083` | as 0x605c, kind 8 | int: camera handle | - | int `0x007b9340` (native_cam) |
| `0x6084` | `Native_Cam_6084` | as 0x605d, kind 8 | int: camera handle | - | int `0x007b9380` (native_cam) |
| `0x6087` | `Native_Cam_6087` | - | int: camera handle (0x007beae0(cam, 0)) | - | int `0x007b8a10` (native_cam) |
| `0x6088` | `Native_Cam_6088` | - | int: camera handle (0x007beae0(cam, 1)) | - | int `0x007b8a70` (native_cam) |
| `0x8000` | `Native_FieldSetMaskBit` | bit int, on int | int: bit | - | int `0x0091c530` (native) |
| `0x8001` | `Native_8001` | on int | int: on | - | int `0x0091c5b0` (native_map) |
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
| `0x800c` | `Native_800c` | on int | int: on != 0 | - | int `0x0091b160` (native_map) |
| `0x800d` | `Native_FpShow` | fp int, show int | int: show | - | int `0x0091c740` (native) |
| `0x800f` | `Native_MapPartShow` | part int: map part | 1 | - | int `0x0091b1a0` (native_map) |
| `0x8010` | `Native_MapPartHide` | part int | 1 | - | int `0x0091b1e0` (native_map) |
| `0x8011` | `Native_8011` | a int, b int, c int | 1 | - | int `0x0091b200` (native_map) |
| `0x8014` | `Native_8014` | n int, x int | 1 | - | int `0x0091b280` (native_map) |
| `0x801d` | `Native_801d` | a int, b int | 1 | - | int `0x0091b400` (native_map) |
| `0x801e` | `Native_801e` | x int | 1 | - | int `0x0091b430` (native_map) |
| `0x801f` | `Native_801f` | x int | 1 | - | int `0x0091b450` (native_map) |
| `0x8020` | `Native_8020` | x int | 1 | - | int `0x0091b470` (native_map) |
| `0x8021` | `Native_8021` | x int | 1 | - | int `0x0091b490` (native_map) |
| `0x8022` | `Native_8022` | a int, b int | 1 | - | int `0x0091b4b0` (native_map) |
| `0x8026` | `Native_8026` | a int, x f, b int, c int, d int | 1 | - | int `0x0091b550` (native_map) |
| `0x802c` | `Native_802c` | - | 1 | - | int `0x0091b680` (native_map) |
| `0x802d` | `Native_802d` | - | 1 | - | int `0x0091b690` (native_map) |
| `0x802e` | `Native_802e` | a int, b int | 1 | - | int `0x0091b6a0` (native_map) |
| `0x802f` | `Native_802f` | x int | 1 | - | int `0x0091b6d0` (native_map) |
| `0x8030` | `Native_8030` | a int, b int | 1 | - | int `0x0091b6f0` (native_map) |
| `0x8032` | `Native_8032` | x int | 1 | - | int `0x0091b740` (native_map) |
| `0x8035` | `Native_8035` | on int | 1 | - | int `0x0091b790` (native_map) |
| `0x8036` | `Native_8036` | on int | 1 | - | int `0x0091b7d0` (native_map) |
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
| `0x804f` | `Native_804f` | a int, x f, b int | 1 | - | int `0x0091c150` (native_map) |
| `0x8059` | `Native_8059` | a int, b int | 1 | - | int `0x0091c3c0` (native_map) |
| `0x805b` | `Native_FpSetStickPos` | n int, x f, y f, z f | 1 | - | int `0x0091c410` (native_map) |
| `0x805c` | `Native_805c` | on int | 1 | - | int `0x0091c470` (native_map) |
| `0x805d` | `Native_805d` | - | 1 | - | int `0x0091c4a0` (native_map) |
| `0x805e` | `Native_805e` | map int: guide map number | 1 | - | int `0x0091c4d0` (native_map) |
| `0x805f` | `Native_805f` | x int | 1 | - | int `0x0091c4f0` (native_map) |
| `0x8060` | `Native_8060` | x int | 1 | - | int `0x0091c510` (native_map) |
| `0x8066` | `Native_8066` | x int | 1 | - | int `0x0091c7c0` (native_map) |
| `0x8067` | `Native_BltzSetObjModels` | worker a int, worker b int, mode int, unused int (Blitzball) | 1 | - | int `0x0091c810` (native_map) |
| `0x806a` | `Native_806a` | x int | 1 | - | int `0x0091af80` (native_map) |
| `0x806b` | `Native_806b` | on int | 1 | - | int `0x0091afc0` (native_map) |

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
returns 0) for the debug lines. `native_special_cases_match_the_original`
covers what small random arguments miss: `Native_MsgSetPos`'s moved
windows, the voice waits' countdowns and debug counting, `Native_MsgClose`
on each window state, `Native_CamSetScreenDepth` in debug mode. In the game those words hold whatever
earlier calls left at that depth, so the Rust interpreter (whose frames
differ) can store different garbage there; nothing reads it back as far as
the tests reach.

### Plan

1. Done: the accessors, the dispatch loop, every opcode, native dispatch,
   the signals (request families, `AtelExecReturn`, the waits, opcodes
   `79`/`7a`): `Script_Run` calls no original script code but natives.
2. Now: the natives, most used first. Done: the field module's ready
   ones (group 8, `0x01a5`, `0x01e4`, `0x01e6`), the sprite, sound
   effect and stop natives, 62 camera natives, the message setters and the
   voice waits. Next: `Native_MsgOpen` and the message waits, the move and
   turn starts, the camera natives with variable or path tables
   (`0x007ba020`, `0x007ba100`, `0x007ba210`, `Native_CamWait`).
3. Then a `Natives` implementation with no original code behind it, so
   the interpreter runs outside the game.

## Open questions

- The meaning of most native functions.
- Opcode `40` and what the `+0x2e4` table of opcode `7a` drives.
- The block header words `0x04`–`0x2c` and the tables they point to.
