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
| `3c`–`3f`, `54` | end the entry (four variants), halt |
| `40` | a debugger breakpoint: runs the instruction it replaced, or stops here (`0x0086442e`) |
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
2.3 million times (event and monster scripts). The battle scripts
(`battle/btl`) add 111 natives and 0.8 million calls. `ffx script disasm`
annotates each call with its handler's
address (and name, when `symbols/ffx.sym` has one); for example
`0x70ab` → `0x007a5360`, which sets a battle actor property
(`0x007b4b70`).

`ffx script natives` lists every native the event, monster and battle scripts call, with its
call count, the number of scripts calling it, its four handler slots and
its name. 1056 of the 1059 are named in `symbols/ffx.sym` (by their result
handler, else their start, else their poll); 1029 of the names are
`Native_*`, with arguments in push order and the evidence. The names
cover 99.96 % of the 3.1 million calls; the three without a name are
`0x70a9`, `0x7043` and `0x706d` (1139 calls). By group:

| Group | Area | Examples |
|---|---|---|
| 0 | workers, movement, triggers, messages, sound, party | `Native_MsgSetText` (`0x0064`: window, message), `Native_MsgOpen` (`0x006a`), `Native_MsgWait_End` (`0x007c`), `Native_MoveStart` (`0x0018`), `CT_0013_setPosition`, `Native_MapJump` (`0x010b`: map, entrance), `Native_WaitFrames_0000`, `Native_SpriteShow` (`0x011b`) |
| 1 | maths | `AtelCall_Math_Sqrt_RF` (`0x1006`) |
| 4 | screen | `Native_FadeIn` (`0x4004`), `Native_FadeOut` (`0x4005`), `Native_LightSet` (`0x4013`), `Native_MenuOpen` (`0x401d`) |
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
ported, whose rows below say which slots remain in the original.

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
That table is a script variable's storage, installed by `Native_00f0`
through `Atel_SetLineTable` (`0x0086d190`): eight 8-byte `(key u32, value
u8)` slots, reset to `(0xffff, 3)` (`Atel_ResetLineTable`, `0x0086d110`),
then 16 signal nodes from `+0x40` that `Atel_FreeLineSignals`
(`0x0086d1c0`) pushes onto the worker's free list (`+0x88`).

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
eax; ret`) and run in Rust only that way. Of the 1586 handler addresses, 1370 are in the
hook table (the `ported!` entries in `crates/ffx-hook/src/ported/`); the
other 216 run in Rust only through the dispatch. The choice is by
static use in the shipped scripts (`cargo run -p ffx-script --example
native_counts` lists every native the event and monster scripts call, with
its call count and handlers) and
by whether every callee is ported. Float handlers return `ST0` as `f64`.
Where a Ghidra signature says `void`, the `EAX` the handler leaves is what
`Script_FinishNative` pushes (`Native_EnableTrigger` returns the kind it
popped, `Native_TurnSetFrames` the elapsed value).

`ffx_script::vm::bridge::native_map` has 59 of the group-8 natives (`0x8000`-`0x806b`,
62 handler addresses: map parts, the field programs, the map's sprite objects, fog and
back colours and parameters) and the field natives `0x01a5`, `0x01e4`,
`0x01e6` (3 handler addresses; 65 in all). Most pop their arguments, call the map module and return 1.
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

`ffx_script::vm::bridge::native_cam` has 70 camera natives of group 6 (71 handler addresses).
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

`ffx_script::vm::bridge::native_move` has 389 natives (411 handler addresses) of the move and
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

`ffx_script::vm::bridge::native_btl` has 138 battle natives (group 7; 144 handler addresses):
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
are marked in the table. `0x5065`'s `Ch_RomReadEventMGRP` asks the file
system for the group's size first (`*0x02310c54`), which hashes the path
with `ADVAPI32`'s `CALG_MD5` and opens the archive; the oracle now has
the `Crypt*` imports (a real MD5) and a `CreateFileW` that finds no file,
so the native completes on both snapshots.

`ffx_script::vm::bridge::helpers` (round 13) has VM helpers that are
not natives: `Script_StopTurn` (`0x00863fe0`, the turn-block twin of
`Script_StopMove`), the script block's header tables (`0x0086ac60`,
`0x0086ac30`, `0x0086ac90`: `+0x2c`, then `+0xc`, offsets relative to
the block), `AtelGetSnd3dWork`, `AtelSoundTableGet_00876860`, the
message-window status of the current bank (`0x0086be60`, `0x0086c8b0`),
`0x0086c530`, `0x0086f780`, `0x008712a0` and `AtelAllClearBindMask`.
Round 14 added 46 more: the effect, love-parameter and guide-map flags,
the script block's expansion getters (`+0x2c`: `+8` the special flags,
`+0x10` a name, `+0x18` a table, `+0x24`, `+0x28` the bind list, `+0x3c`
the sound event; the empty string `0x00b8fbeb` stands for none), the
sprite records (`0x01327148`, 0x1c bytes, 128), `AtelClearSnd3dWork`, two
string helpers and the level and turn-block float setters. Seven of
those (`0x00871500`..`0x008715a0`, `0x0086f300`) end with `fst`, not
`fstp`, so they return the value in `ST0`.
Round 15 added `AtelRequireCtrlWork` (`0x0086ee10`, the ctrl work's
setup for a script block: id, block, its worker/variable counts, the
callbacks, the flag words, scales of 1.0, the cleared slots and
`g_AtelBasicWorkCache` = 0), `AtelSetUpCallFunc`, `AtelGetSaveDic`,
`AtelGetSaveDicName`, `AtelUcOff` (the player's control taken from every
type-1 worker), `0x00877520` (a number as digits over the divisor table
`0x00c52b18`) and `0x0086db80`, and shims for `0x007a3ac0` and the four
group-8 natives the dump now has starts for.
Round 16 took the scheduler's callback queues (`g_AtelCallbackQue`
`0x01327f50`: per queue fifteen 16-byte entries, callback, word and a
7-character name; `AtelRegistCallbackQue2` fills the first free one and
clears the next entry's callback, `AtelExecCallbackQue` calls the first
with its word's address and moves the rest up when it returns non-zero),
`Native_PlayMotionCommon` (`0x00a7aae0`, now called in Rust by the motion
natives, with its per-map `+0x870` fixes), `Script_LineActorSide`,
`AtelGetMyBufferSize`, `AtelClearMesWinWork` (whose `0x01a00200` has no
relocation, so it stays the same when the image moves), `0x00877660`
(hex parsing), `0x0086f460` (one bind-mask bit) and `0x0086ab80`.
Round 17 ported the ctrl work's default start callback (`0x008683e0`,
`+0x50`): every worker runs (`Script_Run(worker, ctrl +8, 1)`) until its
`+0x34` bit 4 says its init is done, then every worker's signals are
decoded and the ctrl work's bit 3 is set. With it the per-frame script
path calls no original script code: what `Script_Run`, the signals and
the scheduler still call is `Mutex_Lock`/`Mutex_Unlock` (`0x006f2ed0`,
`0x006f2f50`, thin wrappers over `WaitForSingleObject`/`ReleaseMutex`),
the script debugger's hook `0x0067a9b0` (a forward to the debugger object
`*0x00ce9014`) and the ctrl works' own callbacks (`+0x28`, `+0x58`,
`+0x5c`, `+0x60`), which belong to their callers' modules.
Round 18 took the natives no shipped script calls, which the modders' API
can offer: `ffx_script::vm::bridge::native_btl2` has 73 more battle
natives of group 7 (78 handler addresses; weapon and item slots of the actor's `g_BtlItemMem2`
record, the angle and distance between two selected actors, model flags
through `0x00795080`, command records through `0x0078ce50`, the starts
and polls that keep the selected actor in the argument word), and
`native_sys` (96 natives, 99 handler addresses) has group 12's footstep sounds (`SndKickFoot` with its kind
per native), voice and shout streams, the effect flags, the CD
interface's methods (`GetCD`, `0x0076d0d0`, called through its function
table) and the peeks and pokes on the buffer `0x0088c830()`, group 8's
remaining map forwarders and the first group-0 ones. Copies the compiler
kept of an earlier native (`0x007a2c50`, `0x00856600`) and bare jumps to
one (`0x00855ec0`, `0x00856030`, the `0x013c` poll and result) reuse its
Rust handler; the jumps and `0x007a64c0` (no function start in the
dump) run in Rust only through the dispatch. A few return the
original's leftover `EAX` because their callers' dispatch ignores it:
`Native_7075`'s start returns the argument word's address. `Native_70e1`
compares a sign-extended byte with `0xff`, which never matches, so -1
reaches `MsGetChr`; `Native_7103` hands `0x008350a0` a vector whose
fourth word is uninitialised stack (the port passes 0); `Native_c029`
calls the angle wrap `0x0088bc20` and drops its result, and that wrap
never returns for an infinity, in the game as in the port. Debug natives
found on the way (`0x705f`, `0xc004`, `0xc005`, `0xc034`, `0xc05c`) are
marked in the table and not ported.

Round 19 began with the setup functions other modules wait on, in
`helpers`: the map jumps `AtelSetEventJump` (`0x0086fec0`) and
`AtelSetEventJump2` (`0x0086ff40`; with the ctrl work's bit 7, or
`FFXIsDebug`: the player's control taken, `TkSetEventJump`, the hook at
`0x01327070`, `MsBattleExeClear`, bit 7 cleared and `+0x210` bit 1 set, the
3D-sound ratio from the float at `0x00b5be88`, then a fade of 30 frames for
a jump to map `0x109` and none for the others, or with the third argument the save
record's frames `+0x12` and kind `+0x11`, 0 `Sg_FadeOut`, 1
`Sg_FadeOutW`), `AtelUcOn` (`0x00871ab0`, the player's control given to
every bound type-1 worker whose control bit is set: `+0x36` bit 3 when the
ctrl work's bit 5 is set, else `+0x34` bit 9; the one with the ctrl work's
`+0xa` id and a model becomes the player), `Script_StopMove`
(`0x00863e10`, the move blocks' counterpart of `Script_StopTurn`: own
blocks at `+0x5b8 + i*0x4c`, and the turn stopped first when the shared
block has `+2` bit 14), `AtelDecodeCommand`, `AtelSetSaveDicIndex` (the
save record's `+0xe` from the event block's `+0x1e`, or its table at
`+0x28` when bit 15 is set), `AtelSetUpCtrlWork` (bit 0, `g_AtelError`
cleared for a ctrl work without `+0x1c`, and the default callbacks),
`Event_GetEntrance` and `Event_GetEntranceAt` (`0x0086c020`; both take the
worker, whose `+4` is its script block: entrances at block `+4`, their end
at `+8`, 0x20 bytes each, the first past the end, `0x01325b50` without
any), `AtelGetEventActorPos` (`0x0086b1e0`), `AtelGetHelpEventFlag`,
`AtelCheckCaptMonster` (the monster list at `0x00c86708`, 8 bytes each:
mode 0 matches `+2`, 1 `+4`, 2 counts all), `AtelRequestTrigger_008764f0`
(a request for an enabled trigger at the level and entry the sound and
vibration tables give), `AtelBindEff2OffSome`, `AtelMapLoad`,
`AtelSkipReqLevel2`, `AtelGetBattleIdAdrs`, `AtelSetMoveHokanMode`,
`AtelGetEventCharHandleFromID`, `AtelGetCharHandleFromBindPC` and
`AtelGetSummonNameforFinalBattleSelector` (from id 15 on it asks
`TkMn2GetCommandName(0x30ff)`, whatever the id). `AtelPCOpen`
(`0x00879cf0`) and `AtelPCOpen2` open `host0:` files, the PS2 development
host: platform code, left to the original.

The natives no shipped script calls continue in
`ffx_script::vm::bridge::native_mods`, last in the dispatch chain. Group
1's maths: `0x100b` (`v << 15`), `0x100c` (`x * 32768.0`, the double at
`0x00b51588`, rounded to `f32` before `_ftol2_sse` truncates it: without
SSE2 the x87 path keeps the low word of a 64-bit truncation, where that
rounding shows for integers past 24 bits), the angle conversions
`0x100d`-`0x1010` and the sine/cosine tables `0x1008`/`0x1009`, `CItan`
(`0x1003`), `atan` (`0x1004`) through the game's own `_CIatan` on the
popped `ST0` (`NativeHost::call_intrinsic1`, the one-operand form of
`call_intrinsic2`), `1 / sqrt(v)` (`0x1007`, both steps rounded to `f32`),
and the vector natives `0x1011`-`0x1018` over `Math_Atan2XZ`,
`Math_YawPitchTo`, `Math_AngleStep`, `Math_InRadius`, `TkMtCalcSpline` and
`0x0088a3a0`, whose variable arguments go through `AtelGetRealVarAdrs`.

Group 4's map and light natives (`0x4010`-`0x403d`) forward to the `Sg_`
point lights (`Sg_PLight*`: enable, radius, light, attenuation, position
with its component mask, colour), `Sg_LightSetDirXYZ`, the camera follow
bit, the wind and the id tables; the getters that take popped words as
out pointers (`0x4027`, `0x4029`, `0x4037`) hand them on unchanged, so a
mod passes real addresses. `0x4000`, `0x4041` and `0x4042` are empty
handlers (`xor eax, eax; ret`) and, with `0x4031` and `0x4035`, run in
Rust only through the dispatch. `0x4038` (`Sg_MemDump`, a memory report)
and `0x401e` (`Sg_Test`, map-test setup and a scratch allocation) are
debug natives, listed in the table and not ported.

Group 5's character natives (each also reachable as its group-4 copy,
`0x4049`-`0x40d3` to `0x5002`-`0x508c`) take the worker's model
(`AtelGetCharHandle`) or worker i's, and do nothing (0) without one:
`Ch_SetSequence`, `Ch_ElemGetDirect`, `Ch_ModelGetHide`, `Ch_SetSysMotion`,
`Ch_TextureSetImageBase`, the point-light mask and the id getters
(`Ch_IdGetMap` `+0x828 & 0x7f`, `Ch_IdGetSound` bits 15-16). `0x5027` asks
`Ch_ElemGetPos` for an element's position with the vector `(x, y, z,
1.0)` in `NativeHost::scratch` and writes the result's three words into
three variables; `0x5038` copies the model's `+0x6b4`..`+0x6bc` into three.
The load pairs keep the selection in the args word: `0x5062` (`id`, `n`,
`keep` into `+0`, `+4`, `+8`) polls `ChN_ReadSystemMGRPsync` and `0x5064`
(`id`, `keep`) `Ch_DataReadSync`; with `keep` one call is enough and its
result is the int slot (`+0xc`), without it `0x5062` returns 5 and
`0x5064` 1 once the read returns 0. `0x5041`, `0x5043`, `0x5045`, `0x508a`
and `0x508c` are `Native_PlayMotionCommon` with fixed mode pairs.

Group 0's last 160 handlers (the general natives no shipped script
calls) are mostly forwarders: the sound setters (`SndSetFoot3dStatus`,
`SndSetFootStatus`, `SndSetKinuStatus` on the worker's party slot `+0x40`,
skipped for -1 except in `0x0150`/`0x0152`; the music and sound-effect
pitch natives with their clamps), the pad getters around `0x0086af30`
(low word, some through `Pad_MapButtons(0, 0, buttons)`, some one bit),
the message-window fields (`0x0067`, `0x0068`, `0x015a`), the save
block from `0x0088c830` (`0x012f`'s eight counters floored at 0,
`0x0131`/`0x0238` on its 20-word bit set), position-block setters and
getters (`AtelGetPosWorkAdrs`; the angle ones through `Math_WrapAngle`),
the worker flag words `+0x36`/`+0x38`, and the start/poll pairs `0x00f7`
(party worker and count), `0x0232` (a pending dummy-controller bit for
two frames), `0x022b` (ten frames, then `0x01a85f70`), `0x022d` (wait for
a button), `0x0258` (the English/international switch) and `0x013a`
(`0x0139`'s choice window with 8). `0x0228` measures the current turn
target against the yaw: wrapped, absolute, over one of two doubles,
capped at 1 (NaN passes the `fcom`), times a popped integer, each step
`f32`, then `_ftol2`. `0x00f7`'s start returns the args address when the
count is positive and the party worker otherwise. The original's locals
show through in three: `AtelGetPartyWorker` reads below its own for a
negative slot, and `0x0099`/`0x009c` pass an out vector `TkMtCalcSpline`
leaves untouched for an empty table; the oracle test gives them chosen
arguments instead of random stacks. `0x01ed` is a debug loop (the empty
`dbgPrintf`, then the popped value pushed back forever) and stays out.
35 handlers are bare jumps or missing from the dump and run in Rust only
through the dispatch; `0x000a`, `0x000b`, `0x022a`, `0x022b`'s start and
`0x0230` keep the auto-port's shim.

Group 6's last 39 camera and screen natives take the worker's camera
handle the way `native_cam` does (`AtelGetCameraWorkAdrs`'s `+0x3c` slot,
filled by `MsAtelGetCameraID(5, worker is the event map's camera actor,
0)`): `MsCameraMoveFrame` on side 0 with mode 1 (`0x602f`, `0x6031`),
`MsCameraResetDirVector`, `Cam_IsMoving`, `MsCameraPriority` and
`0x007bf1a0`'s two bits (`0x600e`, `0x602c`), these returning the handle.
The rest forward `(worker, args, stack)` with fixed words to the
camera-area, shake and eye/ref helpers (`MsAtelCameraShake` passes its
`EAX` on), drive `MsScreenMove`/`MsScreenMoveReset` with popped ids, and
`0x6089` pops eight values (two floats) for `MsScreenShake2` with a 9
between them. `0x607c` keeps the screen id in the args word and waits
until `MsScreenMoveCheck(id, 1)` is 0.

Script round 21 continued with groups b and 7. Group b's table
(`0x00c40e30`) has 16 entries: `AtelInitTotal` pushes it, and the next
address the code refers to, `0x00c40f30`, is the kernel menu's accessor
table (`0x00770540` stores it into `0x00c416d0`; its entries are
`KernelSaveCommand_*` and the like, `cdecl(chr)`). IDs `0xb010` on would
read that table as handler slots and are not natives. Of the 16, the
eight slots left (`0xb005`-`0xb008`, `0xb00e`, `0xb00f`) are the PC port's
stubs for the PS2 movie callbacks: a "Virtuos Warning: Movie on Windows,
PS3 & PS Vita is not impelemented yet!" line naming the callback
(`movie_pause_resulti`, `movie_ch_resulti`, `movie_vol_resulti`,
`movie_start_debug_init`/`_exec`/`_resulti`,
`movie_op_frame_draw_acc_path3_resulti`, `movie_frame_pal_resulti`)
through the empty `dbgPrintf`, then 0. `0xb008`'s poll returning 0 waits
forever; its start leaves `EAX` as it was.

Group 7's last 81 battle natives read the worker's actor
(`AtelGetBattleIdAdrs2`, 0xff for none) and most pick their target with
`Battle_FirstSelectedActor(actor, selector, 0, 1)`: effects
(`MsSetChrEffect_0079eb10`, whose start counts the actor's `+0xe12`,
`MsEtCriticalEffect`, `MsEffectSetGuard`), the sound registrations
(`MsRegSEplay2`/`3`, `MsRegSEfade`, `0x706c`'s per-actor sound table at
`+0xeac`, which skips the last one played, `0x0112c8e0`), the equipment
refresh `0x7079`, the battle text windows
(`0x7042`, `0x7072`, `0x70b7`, and `0x711f`/`0x7121` behind
`MsGetBattleMessageFlag` and `0x01841d20`: the window clamped by
`Clamp0To6_007a86e0`, `0x0112c250` = the text index, the text from the
table at `*0x0112a9b8` or `*0x0112a998`, two `u16` offsets per entry by
`MsGetSaveConfigHiragana`) and a run of byte getters and setters on the
battle globals around `0x0112c9xx`. `0x7045`, `0x70b6` and `0x7077` scale a
`MsSetMove`/`MsGetTargetDist` float and truncate it (`0x7077` capped by the
`f32` at `0x00b4ae10`, NaN passing the `fcom`). `0x703e`'s poll waits
while any of the 31 actors is busy or acting and records which in
`0x0112ca08`/`0x0112ca09` (0xdf, 0xdd; 0xff/0xde when the battle state
rules the check out). `0x7076` (the ball spline a shipped battle script
calls) stays with the original: `MsSetBallSplineInit` copies the out
vector `MsGetChrPos` may leave unwritten, and `MsSetBallSpline` positions
`Ch_OptposGet` may not fill, into the actor and spline records, so a Rust
caller one frame higher hands on different stack leftovers than the
original handler (the script entries test on battle.snap shows it); it
can move once `MsSetBallSpline`'s own port settles them. `0x70f0` is `Atel_GetBattleCountF`, which keeps its
shim. 34 of the 81 are bare jumps, global reads or writes, or missing from
the dump, and run in Rust only through the dispatch.

Group 4's table (`0x00c88d88`) has 71 entries (`0x4000`-`0x4046`) and ends
where group 5's (`0x00c891f8`) begins, so `0x4047`-`0x40d7` read group 5's
entries (`0x4047 + k` is `0x5000 + k`): the same slots, the same handler
addresses, hence the same Rust handlers through the dispatch. The table
lists those IDs with the group-5 row they alias.

The rest of round 21: group 8's map-part natives (`Yn_MapPartSet44` with
the popped float in one or both of `+0x44`/`+0x48`, the `Yn_Scene*` float
getters as jumps, `pppFpSetStickChr`/`pppFpSetStickMapDObj`); group 9's
`0x9000` (an "atel_test_print!" line, 0); group c's system natives
(`SqsSePlay`, the stream state, `AtelMapLoadNoCache`, the command-menu
panel bit, `movie_stop_prog` when a movie runs; `0xc02b` and `0xc04f` share
`0x0008`'s and `0x01d3`'s handlers' code); group d's `0xd000` (its
`abiritymap_debug` line dropped). `0xc000`, `0xc001`, `0xc006`, `0xc015`
and `0xc027` do their work only under `TkIsDebug` (the save block to and
from a PS2 development-host file, a debug sound upload, the collect bits)
and are listed as debug natives.

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
API is to be generated from this table (`docs/rewrite-plan.md`). `-`: none. A result starting with
"debug (mod-tool candidate)" marks a debug native that is not ported
("not ported" in the handler column); the line says what it does. A slot
marked "not ported" runs in the original, and so does the handler of
`0x7076`.

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
| `0x0008` | `Native_0008` | - | int: 0 (xor eax, eax; ret) | - | int `0x0085eef0` (native_mods) |
| `0x0009` | `Native_0009` | - | int: AtelGetCharHandle(worker) (0x0086ae80) | - | int `0x0085f160` (native_mods) |
| `0x000a` | `GetFieldU8aa_0085f2d0` | - | int: the worker's type (+0xaa; the auto-port's shim stays) | - | int `0x0085f2d0` (native_mods) |
| `0x000b` | `GetFieldU16a8_0085f360` | - | int: the worker's +0xa8 (u16; the auto-port's shim stays) | - | int `0x0085f360` (native_mods) |
| `0x000c` | `Native_000c` | i | int: Sg_SetPlayer(i) (0x0082dad0); i | - | int `0x0085f640` (native_mods) |
| `0x000d` | `Native_000d` | - | int: jumps to 0x0086bc70 | - | int `0x008565f0` (native_mods) |
| `0x000e` | `Native_000e` | - | int: jumps to 0x0086bc80 | - | int `0x008566a0` (native_mods) |
| `0x000f` | `Native_000f` | - | int: jumps to 0x0086c1c0 | - | int `0x00856790` (native_mods) |
| `0x0010` | `Native_0010` | as 0x0086c1d0 | as 0x0086c1d0 (a jump) | - | int `0x00856910` (native_move) |
| `0x0011` | `Native_MapChange` | map int, entrance int: map change | 0 | - | int `0x008581c0` (native_move) |
| `0x0012` | `Native_0012` | x f, z f: 0x00870970(worker, x, z, 0) | 0 | - | int `0x0085f800` (native_move) |
| `0x0013` | `CT_0013_setPosition` | x f, y f, z f: the worker's position | 0 | - | int `0x0085faa0` (native) |
| `0x0014` | `Native_0014` | b f, a f | int: 0x0086f750(worker, b, a); 0 | - | int `0x00860120` (native_mods) |
| `0x0015` | `Native_MoveSetTarget` | x f, y f, z f: move target | 0 | - | int `0x008602f0` (native_move) |
| `0x0016` | `Native_MoveSetSpeed` | speed f: move speed | float: speed | - | float `0x00857da0` (native_move) |
| `0x0017` | `Native_SetArriveRadius` | r f: arrive radius of the current level | float: r | - | float `0x0085f040` (native_move) |
| `0x0018` | `Native_MoveStart` | a int, mode int (low 7 bits: 5/6 timed, 7 from here, 8, 9 jump arc; 0x800 added when the block's +0x38 is 0), b int | 0 | no (MoveWait 0x001a waits) | int `0x00856c30` (native_move) |
| `0x0019` | `Native_TurnStart` | a int, mode int (low 5 bits: 2 timed turn, 0x800/0x400 wrap the turn), b int | 0 | no (TurnWait 0x001b waits) | int `0x00857520` (native_move) |
| `0x001a` | `Native_MoveWait` | - | 5 when done | until the move block's mode (+2 & 0x7f) is 0 (the move started by MoveStart ends) | poll `0x008579a0` (native) |
| `0x001b` | `Native_TurnWait` | - | 5 when done | until the turn block's mode (+2 & 0x1f) is 0 | poll `0x00857b30` (native) |
| `0x001c` | `Native_MoveSetFrames` | elapsed int, frames int: the move block's timer (+0xc, +8) | float: elapsed | - | float `0x00855f10` (native) |
| `0x001d` | `Native_TurnSetFrames` | elapsed int, frames int: the turn block's timer (frames <= 0 become 1) | float: elapsed | - | float `0x008560d0` (native) |
| `0x001e` | `Native_001e` | v f, _ f | float: v (the top is dropped) | - | float `0x00856490` (native_sys) |
| `0x001f` | `Native_GetYawToMoveTarget` | - | float: yaw from the worker to the move target | - | float `0x008587a0` (native_move) |
| `0x0020` | `Native_GetPitchToMoveTarget` | - | float: the pitch to the move target (second result of 0x0088b4d0) | - | float `0x00858d30` (native_move) |
| `0x0021` | `Native_0021` | yaw f (radians) | float: the wrapped yaw (move block +0x44) | - | float `0x00859430` (native_move) |
| `0x0022` | `Native_0022` | a f | float: the current level's turn block (+0x600 + level * 0x4c) +0 = Math_WrapAngle(a); that angle | - | float `0x00859510` (native_mods) |
| `0x0023` | `Native_0023` | v f | float: position block +0x20 = v; v | - | float `0x008591a0` (native_mods) |
| `0x0024` | `Native_0024` | v f | float: position block +0x24 = v; v | - | float `0x008592f0` (native_mods) |
| `0x0025` | `Native_0025` | yaw f (radians) | float: the wrapped yaw (position +0x34) | - | float `0x00859d80` (native_move) |
| `0x0026` | `Native_0026` | a f | float: position block +0x30 = the wrapped angle; that angle | - | float `0x00859e50` (native_mods) |
| `0x0027` | `Native_0027` | a f | float: position block +0x38 = the wrapped angle; that angle | - | float `0x00859f30` (native_mods) |
| `0x0028` | `Native_TurnSetYaw` | yaw f (radians) | float: the wrapped yaw (turn target +0x20, remaining +0x2c) | - | float `0x00859640` (native) |
| `0x0029` | `Native_TurnSetPitch` | angle f (radians, pitch) | float: the wrapped angle (+0x24, remaining +0x30) | - | float `0x00859880` (native) |
| `0x002a` | `Native_002a` | pitch f (radians) | float: the wrapped pitch (turn block +0x28; the rest to the position's pitch in +0x34) | - | float `0x00859bb0` (native_move) |
| `0x002b` | `Native_002b` | v f | float: 0x00871520(worker, level, v) | - | float `0x0085a0b0` (native_move) |
| `0x002c` | `Native_002c` | v f | float: 0x00871500(worker, level, v) | - | float `0x0085a1a0` (native_move) |
| `0x002d` | `Native_SetArriveRadiusAll` | r f: arrive radius of every level | float: r | - | float `0x00858020` (native_move) |
| `0x002e` | `Native_TurnSetSpeed` | speed f: turn block +0x10 | float: speed | - | float `0x0085a2a0` (native_move) |
| `0x002f` | `Native_TurnSetSpeed2` | speed f: turn block +0x18 | float: speed | - | float `0x0085a5c0` (native_move) |
| `0x0030` | `Native_0030` | v f | float: 0x00871540(worker, level, v) | - | float `0x0085a7c0` (native_move) |
| `0x0031` | `Native_0031` | - | int: Atel_SetWorkerFlag200(worker, 1) (0x008712a0), 0x0086a0e0(); 1 | - | int `0x00856ba0` (native_mods) |
| `0x0032` | `Native_0032` | - | int: Atel_SetWorkerFlag200(worker, 0), 0x0086a0e0(); 0 | - | int `0x00856da0` (native_mods) |
| `0x0033` | `Native_GetWorkerId` | worker int (-1 = self) | int: its id (+0x2e) | - | int `0x0085ba50` (native) |
| `0x0034` | `Native_EnableTrigger` | kind int: trigger bit (0 talk) | int: kind (worker +0xab gets the bit) | - | int `0x0085c060` (native) |
| `0x0035` | `Native_DisableTrigger` | kind int | 0 (clears worker +0xab's bit) | - | int `0x0085c280` (native) |
| `0x0036` | `Native_0036` | worker int | 0 (each level reset, 0x00863e10) | - | int `0x0085b4e0` (native_move) |
| `0x0037` | `Native_0037` | worker int | 0: 0x00863fe0 on its nine levels | - | int `0x0085b780` (native_move) |
| `0x0038` | `Native_GetWorkerX` | worker int | float: its x | - | float `0x0085bb80` (native) |
| `0x0039` | `Native_GetWorkerY` | worker int | float: its y | - | float `0x0085bd40` (native) |
| `0x003a` | `Native_GetWorkerZ` | worker int | float: its z | - | float `0x0085bf10` (native) |
| `0x003b` | `Native_003b` | v | int: 0x0086f940(worker, v); 0 | - | int `0x008604a0` (native_mods) |
| `0x003c` | `Native_003c` | v | int: 0x0086f8a0(worker, v); 0 | - | int `0x00855d50` (native_mods) |
| `0x003d` | `Native_003d` | - | 0 (0x0086db80(worker, 0xff)) | - | int `0x0085c3e0` (native_move) |
| `0x003e` | `Native_003e` | mask | int: Atel_CopyTurnAngles(worker, mask) (0x0086db80); 0 | - | int `0x0085c470` (native_mods) |
| `0x003f` | `Native_TurnGetField38` | - | float: the turn block's +0x38 | - | float `0x0085c590` (native) |
| `0x0040` | `Native_0040` | - | float: the current level's +0x48 block's +0x3c | - | float `0x0085c740` (native_mods) |
| `0x0041` | `Native_0041` | - | float: the current level's +0x48 block's +0x40 | - | float `0x0085c840` (native_mods) |
| `0x0042` | `Native_SetPartyIndex` | i int: party index (bound character for type-1 workers; slot, 0xf from 8) | int: i | - | int `0x0085cb40` (native_move) |
| `0x0043` | `Native_SetPlayer` | - | int: the worker id (the player is set to it, 0x0086f6a0) | - | int `0x0085c9a0` (native_move) |
| `0x0044` | `Native_0044` | - | int: the flag word 0x00888c40(0, 0, 0x0086af30()) | - | int `0x0085d4e0` (native_move) |
| `0x0045` | `Native_0045` | - | int: the buttons of 0x0086af50() through Pad_MapButtons | - | int `0x0085d760` (native_mods) |
| `0x0046` | `Native_PadPressed` | - | int: buttons pressed this frame, remapped | - | int `0x0085d9b0` (native) |
| `0x0047` | `Native_0047` | - | int: the flag word 0x00888c40(0, 0, 0x0086af80()) | - | int `0x0085db70` (native_move) |
| `0x0048` | `Native_0048` | - | int: AtelGetControlerT() (0x0086af30), the low word | - | int `0x0085dcc0` (native_mods) |
| `0x0049` | `Native_0049` | - | int: 0x0086af50(), the low word | - | int `0x0085ddc0` (native_mods) |
| `0x004a` | `Native_004a` | - | int: AtelGetControler() (0x0086af70), the low word | - | int `0x0085dff0` (native_mods) |
| `0x004b` | `Native_004b` | - | int: 0x0086af80(), the low word | - | int `0x0085e0d0` (native_mods) |
| `0x004c` | `Native_004c` | i int | int: bit i of the flag word for 0x0086af30() | - | int `0x0085e130` (native_move) |
| `0x004d` | `Native_004d` | i int | int: bit i of the flag word for 0x0086af70() (0 for worker 11 on story 0x31 with [0x0112d6c9] bit 5) | - | int `0x0085e820` (native_move) |
| `0x004e` | `Native_004e` | b | int: bit b of 0x0086af80()'s buttons through Pad_MapButtons | - | int `0x0085ea80` (native_mods) |
| `0x004f` | `Native_004f` | b | int: bit b of 0x0086af50()'s buttons through Pad_MapButtons | - | int `0x0085e3c0` (native_mods) |
| `0x0050` | `Native_0050` | i int | int: bit i of 0x0086af30() | - | int `0x0085ec60` (native_move) |
| `0x0051` | `Native_0051` | i int | int: bit i of 0x0086af70() | - | int `0x0085f0e0` (native_move) |
| `0x0052` | `Native_0052` | b | int: bit b of 0x0086af80() | - | int `0x0085f2a0` (native_mods) |
| `0x0053` | `Native_0053` | b | int: bit b of 0x0086af50() | - | int `0x0085eda0` (native_mods) |
| `0x0054` | `Native_TriggerSetLine` | x0 f, y0 f, z0 f, x1 f, y1 f, z1 f: the line ends (+0, +0x30) | int: 1 for line triggers (types 2, 5), else 0 | - | int `0x0085f370` (native_move) |
| `0x0055` | `Native_0055` | r f: position +0x1c | int: 1 for triggers (types 2, 3, 5, 6), else 0 | - | int `0x0085fa20` (native_move) |
| `0x0056` | `Native_TriggerSetActive` | on int | 1 for trigger workers (types 2, 3, 5, 6: +0x34 bit 7 = on), else 0 | - | int `0x0085fbf0` (native) |
| `0x0057` | `Native_TriggerSetPoint` | x f, y f, z f: the trigger point | int: 1 for point triggers (types 3, 6), else 0 | - | int `0x008603e0` (native_move) |
| `0x0058` | `Native_0058` | as 0x0059, after the float at 0x00b92340 is pushed | as 0x0059 (0x00855cd0) | - | int `0x00855f80` (native_move) |
| `0x0059` | `Native_0059` | z1 f, h f, r f: position +0x30, +0x38, +0x1c | int: 1 for point triggers (types 3, 6), else 0 | - | int `0x00855cd0` (native_move) |
| `0x005a` | `Native_005a` | yaw f: position +0x34 of point triggers (types 3, 6) | 0 | - | int `0x00856170` (native_move) |
| `0x005b` | `Native_SetTouchRange_copy_00856600` | as Native_SetTouchRange | as Native_SetTouchRange | - | int `0x00856600` (native_sys) |
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
| `0x0067` | `Native_0067` | i, v | int: v's low and high halves pushed back, then as 0x0068 (the window's +4 = low, +6 = high); 0 | - | int `0x00858850` (native_mods) |
| `0x0068` | `Native_0068` | i, b, a | int: message window i's +4 = b, +6 = a; 0 | - | int `0x00858640` (native_mods) |
| `0x0069` | `Native_0069` | window int, b int, a int: +0x1e = b, +0x1f = a (bytes) | int: window | - | int `0x008589d0` (native_move) |
| `0x006a` | `Native_MsgOpen` | window int, mode int (0, 2, 66: choice flags) | int: window (opened, state 1) | - | int `0x00858b50` (native_msg) |
| `0x006b` | `Native_MsgClose` | window int | int: window (closes it) | - | int `0x00859060` (native_msg) |
| `0x006c` | `Native_SetMoveSpeed` | speed f: worker move speed | float: speed | - | float `0x00857f40` (native_move) |
| `0x006d` | `Native_SetMoveParam3C` | a f: move block +0x3c of all levels | float: a | - | float `0x0085ab80` (native_move) |
| `0x006e` | `Native_SetMoveParam40` | a f: move block +0x40 of all levels | float: a | - | float `0x0085acd0` (native_move) |
| `0x006f` | `Native_SetTurnParam10` | a f: turn block +0x10 of all levels | float: a | - | float `0x0085af20` (native_move) |
| `0x0070` | `Native_SetTurnParam18` | a f: turn block +0x18 of all levels | float: a | - | float `0x0085b230` (native_move) |
| `0x0071` | `Native_SetTurnParam1C` | a f: turn block +0x1c of all levels | float: a | - | float `0x0085b480` (native_move) |
| `0x0072` | `Native_0072` | v | int: SetField_00870520(worker, level, v) (the level's move block +0x5bc word); v | - | int `0x0085bb50` (native_mods) |
| `0x0073` | `Native_0073` | v | int: SetField_00871170(worker, level, v) (the level's turn block +0x868 word); v | - | int `0x0085bc90` (native_mods) |
| `0x0074` | `Native_SetMoveMOT` | v int: move motion | int: v | - | int `0x0085bdc0` (native_move) |
| `0x0075` | `Native_0075` | v | int: SetFields_0086f120(worker, v); v | - | int `0x0085bf40` (native_mods) |
| `0x0076` | `Native_GetOwnWorkerType` | - | int: the worker's type (0x0086a7b0) | - | int `0x0085f5a0` (native_move) |
| `0x0077` | `Native_WorkerStopMove` | worker int | 0 (stops its move) | - | int `0x0085b620` (native) |
| `0x0078` | `Native_WorkerStopTurn` | worker int | 0 (stops its turn) | - | int `0x0085b950` (native) |
| `0x0079` | `Native_0079` | v | int: v (popped and returned) | - | int `0x0085c040` (native_mods) |
| `0x007a` | `Native_LineExtend` | back f, front f: how far the line ends move out | int: 1 for line triggers (types 2, 5), else 0 | - | int `0x0085c130` (native_move) |
| `0x007b` | `Native_007b` | i | int: ReturnConst_008b7cc0(text i) (0x008b7cc0), the text from Msg_Text1 (0x0086b170) when 0x0086bbc0() is set, else Msg_Text (0x0086bf30) | - | int `0x00857ea0` (native_mods) |
| `0x007c` | `Native_MsgWait_End` | window int | 1 when done; int slot 0 (closes it) | until the window is closed or in state 4 (open with its text shown and the ctrl work's bit 2: closed then) | start `0x0085b7c0`, poll `0x0085b980`, int `0x0085ba80` (native_msg) |
| `0x007d` | `Native_MsgSelect` | window int[, a int[, b int]]: choice window | 1 when done; int slot: the choice (-1 none) | until the window is closed or in state 4, or open with its input done (closed then) | start `0x00859710`, poll `0x00859c50`, int `0x0085a6f0` (native_msg) |
| `0x007e` | `Native_007e` | - | int: jumps to AtelGetNowCtrlTopId (0x0086c1a0) | - | int `0x0085c580` (native_mods) |
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
| `0x008a` | `Native_008a` | - | int: jumps to AtelGetEventMapCameraId (0x0086b5d0) | - | int `0x0085d3b0` (native_mods) |
| `0x008b` | `Native_LineGetCrossSide` | - | int: side of the crossed line (0x008632b0), else from worker +0x36 (-1 for 3) | - | int `0x0085d410` (native_move) |
| `0x008c` | `Native_008c` | - | int: the current ctrl work's +0x1f4 | - | int `0x0085d8b0` (native_mods) |
| `0x008d` | `Native_SetTYZAngle` | angle f: pos +0x4c | float: the angle | - | float `0x008577c0` (native_move) |
| `0x008e` | `Native_008e` | v f | float: position block +0x1c = v; v | - | float `0x0085db40` (native_mods) |
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
| `0x0099` | `Native_0099` | &points, n | int: 0x00862b20(&p, points, n), then AtelSetPos3d(worker, p, 0); 1 | - | int `0x0085fd80` (native_mods) |
| `0x009a` | `Native_009a` | v int: position +0x28 for types 5, 6 | int: v | - | int `0x00860390` (native_move) |
| `0x009b` | `Native_009b` | b var, a var: two vectors pushed as the line for 0x0054 | as 0x0054 | - | int `0x0085f750` (native_move) |
| `0x009c` | `Native_009c` | &points, n | int: 0x00862b70(&p, points, n), then AtelSetPos3d(worker, p, 0); 1 | - | int `0x00860000` (native_mods) |
| `0x009d` | `Native_MsgSetFlags` | window int, flags int | int: window | - | int `0x008584d0` (native) |
| `0x009e` | `Native_MsgSetValue` | window int, slot int, type int, value int: inline text variable | int: window | - | int `0x00855c70` (native_msg) |
| `0x009f` | `Native_009f` | win int, n int, v int | int: win; TOMacroSetValue(win, n, v) (0x008b8980) | - | int `0x00856090` (native_sys) |
| `0x00a0` | `Native_00a0` | win int, n int, t int | int: win; TOMacroSetType(win, n, t) | - | int `0x00855ed0` (native_sys) |
| `0x00a1` | `Native_00a1` | win int, v int | int: v; the message window's +0x10 = v | - | int `0x00856220` (native_sys) |
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
| `0x00ad` | `Native_00ad` | v | int: AtelSetSaveDicIndex(v) (0x00870ef0); v | - | int `0x008581f0` (native_mods) |
| `0x00ae` | `Native_00ae` | - | int: jumps to AtelGetSaveDic (0x0086c410) | - | int `0x00858320` (native_mods) |
| `0x00af` | `Native_00af` | v int | int: v; 0x0086f3c0(v) | - | int `0x00858440` (native_sys) |
| `0x00b0` | `Native_00b0` | - | int: jumps to AtelGetBattleSceneGroup (0x0086a9a0) | - | int `0x008585c0` (native_mods) |
| `0x00b1` | `Native_00b1` | v f: move block +0x18 | float: v | - | float `0x00856a50` (native_move) |
| `0x00b2` | `Native_00b2` | on int | int: 0x0083add0(on != 0) | - | int `0x008586b0` (native_move) |
| `0x00b3` | `Native_00b3` | v int: the first word of 0x0083adf0's set (0x0083b110) | int: v | - | int `0x00858810` (native_move) |
| `0x00b4` | `Native_00b4` | v int: the second word of 0x0083adf0's set (0x0083b110) | int: v | - | int `0x00858a20` (native_move) |
| `0x00b5` | `Native_00b5` | b | int: Sg_GetTkAntiXYB(&x, &y, &b) (0x0083adf0), then Sg_SetTkAntiXYB(x, y, b) (0x0083b110) with the popped b; b | - | int `0x00858cf0` (native_mods) |
| `0x00b6` | `Native_00b6` | - | int: the first of the three words 0x0083adf0 writes | - | int `0x00858e20` (native_sys) |
| `0x00b7` | `Native_00b7` | - | int: the second of the three words 0x0083adf0 writes | - | int `0x00859080` (native_sys) |
| `0x00b8` | `Native_00b8` | - | int: the third of the three words 0x0083adf0 writes | - | int `0x00859220` (native_sys) |
| `0x00b9` | `Native_00b9` | - | int: r = Sg_EnableTkAnti(0) (0x0083add0), then Sg_EnableTkAnti(r); r | - | int `0x008593e0` (native_mods) |
| `0x00ba` | `SetFieldConst44_008594a0` | - | 1: the worker's +0x44 = 0 | - | int `0x008594a0` (native_move) |
| `0x00bb` | `Native_SetMapFadeFrames` | frames int: fade length for the next map jump | int: frames | - | int `0x008596b0` (native) |
| `0x00bc` | `Native_SetMapFadeType` | type int: fade kind for the next map jump (0, 1) | int: type | - | int `0x00859590` (native) |
| `0x00bd` | `Native_00bd` | n int (at least 1): 0x00871730(worker id, n - 1) | int: n | - | int `0x00859840` (native_move) |
| `0x00be` | `Native_00be` | on int: the ctrl work's bit 7 | int: on != 0 | - | int `0x00859950` (native_move) |
| `0x00bf` | `Native_00bf` | i int | float: +0xc of 0x0086c020(worker, i) | - | float `0x00859d30` (native_move) |
| `0x00c0` | `Native_00c0` | i int | float: +0x10 of 0x0086c020(worker, i) | - | float `0x00859e00` (native_move) |
| `0x00c1` | `Native_00c1` | i int | float: +0x14 of 0x0086c020(worker, i) | - | float `0x00859ef0` (native_move) |
| `0x00c2` | `Native_00c2` | i int | float: +8 of 0x0086c020(worker, i) | - | float `0x0085a070` (native_move) |
| `0x00c3` | `Native_00c3` | - | int: 1 when the low five bits of the u16 at +2 of the current level's (Script_WorkerLevel) +0x48 block are set | - | int `0x0085a130` (native_mods) |
| `0x00c4` | `Native_00c4` | - | int: 1 while the move block runs (mode +2 & 0x7f) | - | int `0x0085a340` (native_move) |
| `0x00c5` | `Native_SetTurnMotions` | m0 int, m1 int, m2 int, m3 int: turn motions | 0 | - | int `0x0085a4e0` (native) |
| `0x00c6` | `Native_GetTurnKind` | - | int: bit 0 remaining yaw > 0, bit 1 at least the half-turn angle | - | int `0x0085a670` (native) |
| `0x00c7` | `Native_GetTurnMotion` | kind int (from 0x00c6) | int: the motion 0x00c5 set for it | - | int `0x0085abb0` (native) |
| `0x00c8` | `Native_GetTurnFrames` | mode int, frames2 int, frames int | int: the frames left of the turn: the remaining angle over pi (mode bit 1, eased) or [0x00b11338], clamped to 0..1, times frames (bit 1) or frames2 | - | int `0x0085ad00` (native_move) |
| `0x00c9` | `Native_00c9` | i int | float: the signed word +6 of 0x0086c020(worker, i) | - | float `0x0085b560` (native_move) |
| `0x00ca` | `Native_PartyAdd` | chr int: joins the party (0x007869b0(chr, 1)) | int: chr | - | int `0x0085b700` (native_move) |
| `0x00cb` | `Native_00cb` | chr int | int: chr (removed from the party slots) | - | int `0x0085b820` (native_move) |
| `0x00cc` | `Native_00cc` | chr int | int: 1 when 0x00785380(chr) is 1 | - | int `0x0085b9f0` (native_move) |
| `0x00cd` | `Native_00cd` | - | int: 1 when the worker's party slot (+0x40) is one of the three MsGetSavePartyMember (0x007852f0) gives and not 0xff, else 0 | - | int `0x0085bbe0` (native_mods) |
| `0x00ce` | `Native_00ce` | v int: worker +0xb16 | int: v | - | int `0x0085c2b0` (native_move) |
| `0x00cf` | `Native_00cf` | v int: worker +0xaf0 | int: v | - | int `0x0085c400` (native_move) |
| `0x00d0` | `GetFieldaf0_0085c4b0` | - | int: worker +0xaf0 | - | int `0x0085c4b0` (native_move) |
| `0x00d1` | `CT_01D1_ret` | - | 0: 0x00863e10 on the worker's nine levels | - | int `0x0085c5b0` (native_move) |
| `0x00d2` | `Native_00d2` | - | 0: 0x00863fe0 on the worker's nine levels | - | int `0x0085c6f0` (native_move) |
| `0x00d3` | `Native_00d3` | - | int: Script_StopMove(worker, -1) (0x00863e10) through its own index pushed and popped; 0 | - | int `0x0085c7d0` (native_mods) |
| `0x00d4` | `Native_00d4` | - | int: Script_StopTurn(worker, -1) (0x00863fe0) the same way; 0 | - | int `0x0085c910` (native_mods) |
| `0x00d5` | `Native_VoiceStandby_End` | voice int | the standby frames; int slot 0 (Voice_Prepare unless a voice is on standby) | until the standby countdown runs out, or no movie runs (and g_VoiceHold allows) | start `0x0085cd00`, poll `0x0085cec0`, int `0x0085d360` (native_msg) |
| `0x00d6` | `Native_VoiceStart_End` | - | the pending frames; int slot 0 (volume, play flag, sync countdown) | until the start countdown runs out, or the stream is ready and the frames counted down | start `0x0085d6b0`, poll `0x0085d9d0`, int `0x0085de80` (native_msg) |
| `0x00d7` | `Native_VoiceStop` | - | 0 (stops the voice) | - | int `0x0085e090` (native) |
| `0x00d8` | `Native_VoiceIsPlaying` | - | 1 while the voice plays, else 0 | no (scripts loop on it) | int `0x0085e1d0` (native_msg) |
| `0x00d9` | `Native_SoundWait_Poll` | - | 1 when done | until the voice has stopped (or the voice-sync countdown runs out) | poll `0x0085e4a0` (native_msg) |
| `0x00da` | `Native_00da` | as 0x0086a9b0 | as 0x0086a9b0 (a jump) | - | int `0x00857c30` (native_move) |
| `0x00db` | `Native_00db` | v int | 5 when done | until 0x00788bd0() is 3 (or 1 on map 0x10e), calling 0x00872470(v) each frame | start `0x0085e9e0`, poll `0x0085eaf0` (native_move) |
| `0x00dc` | `Native_00dc` | - | done when 0x008633e0(worker id) has bit 0 | until 0x008633e0(worker id) bit 0 | start `0x0085ee00`, poll `0x0085ee70` (native_move) |
| `0x00dd` | `Native_00dd` | - | int: MsEventEffectGetStat() (0x00788bd0): 0 and 6 give 0, 1, 3 and 5 give 1, the rest themselves | - | int `0x0085f070` (native_mods) |
| `0x00de` | `Native_00de_Poll` | - | 0x00863350(worker id) | until 0x00863350(worker id) (poll only) | poll `0x0085f280` (native_move) |
| `0x00df` | `Native_SfxPlay` | sfx int | int: sfx | - | int `0x0085f310` (native) |
| `0x00e0` | `Native_00e0` | v int: 0x00886eb0(v) | int: v | - | int `0x00855f60` (native_move) |
| `0x00e1` | `Native_00e1` | as 0x0086a9d0 | as 0x0086a9d0 (a jump) | - | int `0x00857fb0` (native_move) |
| `0x00e2` | `Native_00e2` | v int: 0x0086f430(v) | int: v | - | int `0x00857dd0` (native_move) |
| `0x00e3` | `Native_SetTurnSpeed` | speed f: turn block +0x14 | float: speed | - | float `0x0085a480` (native_move) |
| `0x00e4` | `Native_SetTurnSpeedAll` | speed f: the turn speed of all 9 slots (0x0086f300) | float: speed | - | float `0x0085b030` (native_move) |
| `0x00e5` | `Native_00e5` | on int | int: on != 0, stored as bit 3 of the current ctrl work's +1 | - | int `0x00856130` (native_sys) |
| `0x00e6` | `Native_00e6` | chr int | int: 1 when chr is in a party slot | - | int `0x0085bf90` (native_move) |
| `0x00e7` | `Native_PartySwapIn` | party b int, a int (-1 = 0xff) | int: a | - | int `0x0085bdf0` (native_move) |
| `0x00e8` | `Native_00e8` | c var, b var, a var: the three party slots (-1 for empty) | 1 | - | int `0x00856270` (native_move) |
| `0x00e9` | `Native_00e9` | on int: the ctrl work's +1 bit 4 | int: on != 0 | - | int `0x00856660` (native_move) |
| `0x00ea` | `Native_00ea` | - | int: bit 4 of the current ctrl work's byte +1 | - | int `0x008567d0` (native_mods) |
| `0x00eb` | `GetFieldU832_00856980` | - | int: the worker's level (+0x32) | - | int `0x00856980` (native_move) |
| `0x00ec` | `Native_00ec` | chr int | int: its party slot 0..2, -1 when absent | - | int `0x00856bc0` (native_move) |
| `0x00ed` | `GetField40_008571c0` | - | int: the worker's bound character (+0x40) | - | int `0x008571c0` (native_move) |
| `0x00ee` | `Native_00ee` | worker int | int: its +0x34 bit 5 | - | int `0x008572b0` (native_move) |
| `0x00ef` | `Native_00ef` | on int: +0x36 bit 11 | int: on != 0 | - | int `0x00857470` (native_move) |
| `0x00f0` | `Native_00f0` | &v | int: 0x0086d190(worker, variable address); 1 | - | int `0x008576a0` (native_mods) |
| `0x00f1` | `Native_00f1` | key | int: Script_LineActorSide(worker, key) (0x008632b0) when it is not negative; else, when key is the position block's +0x28 or that is 0xffff, the worker's +0x36 bits 1-2 (3 as -1); else the negative result | - | int `0x0085d620` (native_mods) |
| `0x00f2` | `Native_00f2` | v int | int: 0x00870190(v) | - | int `0x00857850` (native_move) |
| `0x00f3` | `Native_MinimapSetCorner` | corner int: minimap corner (0x00919e20) | int: corner | - | int `0x00857c40` (native_move) |
| `0x00f4` | `Native_AngleToPoint` | x f, y f (ignored), z f | float: yaw from the worker to (x, z) | - | float `0x00858970` (native_move) |
| `0x00f5` | `Native_00f5` | x f, y f, z f | float: 0x0088b4d0's second result from the worker's position | - | float `0x00858f40` (native_move) |
| `0x00f6` | `Native_00f6` | n int (at least 1), worker int | 5 when done | until 0x00863520(worker, n) is 0 (at once for a negative worker) | start `0x00857df0`, poll `0x00857ff0` (native_move) |
| `0x00f7` | `Native_00f7` | slot, n | args +0 = AtelGetPartyWorker(slot) (0x0086a5e0), +4 = n, at least 1; the args address, or the worker when n is not positive | - | start `0x00858290`, poll `0x00858400` (native_mods) |
| `0x00f8` | `Native_00f8` | var int, value int, count int | debug (mod-tool candidate): in a debug build (TkIsDebug) fills the variable with memset(&var, value, count); the release build only pops the three; returns count | - | int `0x00858520` (not ported) |
| `0x00f9` | `Native_00f9` | - | 1 (TkGetEventNewGame in a debug build) | - | int `0x00858780` (native_move) |
| `0x00fa` | `Native_00fa` | v int: 0x0088eb40(v) | int: v | - | int `0x00858920` (native_move) |
| `0x00fb` | `Native_00fb` | window int | int: 0x008b8010(window) | - | int `0x00858ac0` (native_move) |
| `0x00fc` | `Native_00fc` | v | int: 0x008700e0(v) | - | int `0x00858dd0` (native_mods) |
| `0x00fd` | `Native_SfxPlayEx` | sfx int, pan int, volume int (0-127) | int: sfx | - | int `0x0085ff30` (native) |
| `0x00fe` | `Native_00fe` | b int, a int: 0x00886f60(b, a) | int: a | - | int `0x00860090` (native_move) |
| `0x00ff` | `Native_00ff` | id, pan | int: SndSepPanDirect(id, pan) (0x00886df0); pan | - | int `0x008602c0` (native_mods) |
| `0x0100` | `Native_0100` | c int, b int, a int: 0x00886ff0(c, b, a) | int: a | - | int `0x00860460` (native_move) |
| `0x0101` | `Native_0101` | id int, pan int, frames int | int: frames; SndSepPanTranslate(id, pan, frames) | - | int `0x00855d90` (native_sys) |
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
| `0x0113` | `Native_0113` | - | int: jumps to AtelGetSave20 (0x0086d9d0) | - | int `0x0085b5c0` (native_mods) |
| `0x0114` | `Native_0114` | b int, a int: the ctrl work's +2 bit 2 = b, bit 1 = a | int: b != 0 | - | int `0x0085b670` (native_move) |
| `0x0115` | `Native_0115` | on int: the ctrl work's +2 bit 3 | int: on != 0 | - | int `0x0085b8c0` (native_move) |
| `0x0116` | `Native_WorkerSetFlag0200` | worker int | int: worker (+0x36: 0x400 cleared, 0x200 set) | - | int `0x0085b9b0` (native) |
| `0x0117` | `Native_0117` | worker int: its +0x36 bit 10 cleared, bits 9 and 12 set | int: worker | - | int `0x0085bc50` (native_move) |
| `0x0118` | `Native_0118` | i | int: worker i's +0x36 gets bit 10 and loses bit 9; i | - | int `0x0085bae0` (native_mods) |
| `0x0119` | `Native_0119` | on int (clamped 0..1) | int: 0x0086b150(bound character, on) | - | int `0x0085bd70` (native_move) |
| `0x011a` | `Native_011a` | c var, b int, a var: 0x00874b70(&c, b, &a) | 1 | - | int `0x0085c0a0` (native_move) |
| `0x011b` | `Native_SpriteShow` | sprite int (0-127), data var: variable reference | int: sprite | - | int `0x0085c370` (native) |
| `0x011c` | `Native_SpriteHide` | sprite int | int: sprite | - | int `0x0085c4c0` (native) |
| `0x011d` | `Native_011d` | b int, a int: 0x008749b0(b, a) | int: b | - | int `0x0085c650` (native_move) |
| `0x011e` | `Native_011e` | b, a | int: 0x00874ad0(b, a); b | - | int `0x0085caf0` (native_mods) |
| `0x011f` | `Native_SpriteSetPos` | sprite int, x int, y int | int: sprite | - | int `0x0085cc50` (native_move) |
| `0x0120` | `Native_SpriteSetScale` | sprite int, sx f, sy f (1.0 normal) | int: sprite | - | int `0x0085cde0` (native) |
| `0x0121` | `Native_0121` | i int (0..13) | int: the flag byte 0x00888c00(0, 0, 0x00888cf0(0, 0, i)) | - | int `0x0085d1d0` (native_move) |
| `0x0122` | `Native_0122` | i int (0..13) | int: the flag byte 0x00888c00(0, 0, i) | - | int `0x0085d310` (native_move) |
| `0x0123` | `Native_0123` | v | int: v (popped and returned) | - | int `0x0085d440` (native_mods) |
| `0x0124` | `Native_0124` | v | int: v (popped and returned) | - | int `0x0085d530` (native_mods) |
| `0x0125` | `Native_0125` | b f, a f | int: 0x00870970(worker, b, a, 1); 0 | - | int `0x0085fc50` (native_mods) |
| `0x0126` | `Native_SetPos_0126` | x f, y f, z f: 0x00870b20(worker, x, y, z, 1) | 0 | - | int `0x0085fea0` (native_move) |
| `0x0127` | `Native_0127` | worker int | as 0x0128 with the own touch range (position +0x48) as the angle | - | int `0x0085db10` (native_move) |
| `0x0128` | `Native_0128` | worker int, angle f | int: 0x0088b730(angle, the offset from that worker) | - | int `0x0085d7b0` (native_move) |
| `0x0129` | `Native_SpriteSetColor` | sprite int, r int, b int, g int | int: sprite | - | int `0x0085c760` (native) |
| `0x012a` | `Native_012a` | b int, a int: 0x00874970(b, a) | int: b | - | int `0x0085c890` (native_move) |
| `0x012b` | `Native_012b` | on int: the ctrl work's +2 bit 6 | int: on != 0 | - | int `0x0085dc50` (native_move) |
| `0x012c` | `Native_012c` | b, a | int: the current ctrl work's +4 = a, +6 = b (u16s); 1 | - | int `0x0085ddd0` (native_mods) |
| `0x012d` | `Native_012d` | i int (0..7) | int: word i of record 0x0088c830() +0x2c | - | int `0x0085df80` (native_move) |
| `0x012e` | `Native_012e` | i int (0..7), v int: added to word i of record +0x2c | int: the sum | - | int `0x0085e040` (native_move) |
| `0x012f` | `Native_012f` | i, n | int: the save block's (0x0088c830()) counter i (clamped to 0..7, +0x2c + 4 i) less n, 0 when that is negative; the new value | - | int `0x0085e170` (native_mods) |
| `0x0130` | `Native_0130` | i int (0..7), v int: word i of record +0x2c (0 when negative) | int: the clamped i | - | int `0x0085e410` (native_move) |
| `0x0131` | `Native_0131` | i | int: clears bit i of the save block's (0x0088c830()) 20-word bit set at +0x4c (nothing past it); 1 | - | int `0x0085e760` (native_mods) |
| `0x0132` | `Native_0132` | worker int, on int: +0x34 bit 7 of a trigger worker | int: 1 for triggers (types 2, 3, 5, 6), else 0 | - | int `0x0085fe30` (native_move) |
| `0x0133` | `Native_0133` | as 0x0085fe30 | as 0x0085fe30 | - | int `0x00860290` (native_move) |
| `0x0134` | `Native_ChrLoadAlias` | as Native_SetModel (0x0001) | SetModel's result | - | int `0x0085d3c0` (native_move) |
| `0x0135` | `Native_0135` | as 0x00784e80 | as 0x00784e80 (a jump) | - | int `0x0085ed00` (native_move) |
| `0x0136` | `Native_0136` | v int | int: 1 when 0x007859a0(-v) is not negative | - | int `0x0085ee40` (native_move) |
| `0x0137` | `Native_0137` | v int | int: 1 when 0x007859a0(v) is not negative | - | int `0x0085ef80` (native_move) |
| `0x0138` | `Native_0138` | v int: 0x00785c20(v) | as 0x00784e80 | - | int `0x0085f140` (native_move) |
| `0x0139` | `Native_0139` | as 0x00860b00(worker, args, stack, 0) | int: 0x008b7c40(the choice) after Native_MsgSelect_Result | as Native_MsgSelect_Poll | start `0x0085f730`, poll `0x0085f850`, int `0x0085f900` (native_move) |
| `0x013a` | `Native_013a` | - | 0x00860b00(worker, args, stack, 8)'s EAX (the choice window open, as 0x0139's start with 8) | - | start `0x0085fb30`, poll `0x0085fcd0`, int `0x0085fd40` (native_mods) |
| `0x013b` | `Native_MsgShowWait` | window int, message int, choice b int, choice a int, x int, y int, anchor int | 1 when done; int slot: the choice (-1 none) | as 0x007d | start `0x008602a0`, poll `0x008603d0`, int `0x00860760` (native_msg) |
| `0x013c` | `Native_013c` | as Msg_ShowSetup: window, message, b, a, x, y, anchor | Msg_ShowSetup with mode 8 (window, text, open, choice) | the choice, as Native_MsgSelect (poll 5) | start `0x00855d70`, poll `0x00855ec0`, int `0x00856030` (native_sys) |
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
| `0x0147` | `Native_0147` | v | int: the worker's +0xbc = v with bit 31 set (0 stays 0), +0xb4 = the f32 at 0x00b41db0; that word | - | int `0x00858250` (native_mods) |
| `0x0148` | `Native_MinimapSetMarker` | marker int (0 default), worker int | int: the stored value | - | int `0x008583b0` (native) |
| `0x0149` | `Native_0149` | on int: the ctrl work's +2 bit 7 | int: on != 0 | - | int `0x00858580` (native_move) |
| `0x014a` | `Native_014a` | c int, b int, a int: 0x0086fc20(c, b, a) | int: c | - | int `0x00858740` (native_move) |
| `0x014b` | `Native_014b` | on | int: SndSetFoot3dStatus(party slot, on) (0x008870b0) when the worker has a slot; on != 0 | - | int `0x008588e0` (native_mods) |
| `0x014c` | `Native_014c` | slot, on | int: SndSetFoot3dStatus(slot, on != 0) (0x008870b0); on != 0 | - | int `0x00858af0` (native_mods) |
| `0x014d` | `Native_014d` | as 0x0086bcc0 | as 0x0086bcc0 (a jump) | - | int `0x00858e10` (native_move) |
| `0x014e` | `Native_014e` | r int, y int, x int: stick position (0..255) | float: 0 inside radius r of (128, 128), else the angle 0x00889e70(x - 128, y - 128) (0 as [0x00b368a0]) | - | float `0x00858ea0` (native_move) |
| `0x014f` | `Native_014f` | c f, b f, a f | int: 1 when c > 0x0088b400(wrap(b - a), c) | - | int `0x00859120` (native_move) |
| `0x0150` | `Native_0150` | - | int: SndSetFoot3dStatus(party slot, 1); 1 | - | int `0x00859380` (native_mods) |
| `0x0151` | `Native_FootstepHookEnable` | v int | 1 (0x008870b0(v, 1)) | - | int `0x00859410` (native_move) |
| `0x0152` | `Native_0152` | - | int: SndSetFoot3dStatus(party slot, 0); 0 | - | int `0x008594f0` (native_mods) |
| `0x0153` | `Native_0153` | slot | int: SndSetFoot3dStatus(slot, 0); 0 | - | int `0x00859600` (native_mods) |
| `0x0154` | `Native_0154` | - | int: 1 after 0x00887100(bound character, 1), 0 without one | - | int `0x008597a0` (native_move) |
| `0x0155` | `Native_0155` | v int: 0x00887100(v, 1) | 1 | - | int `0x00859d00` (native_move) |
| `0x0156` | `Native_0156` | - | int: 1 after 0x00887100(bound character, 0), 0 without one | - | int `0x00859920` (native_move) |
| `0x0157` | `Native_0157` | v int: 0x00887100(v, 0) | 1 | - | int `0x00859dc0` (native_move) |
| `0x0158` | `Native_0158` | slot int: the worker's party slot | int: slot | - | int `0x0085cfa0` (native_move) |
| `0x0159` | `Native_0159` | worker int, slot int: that worker's party slot | int: slot | - | int `0x0085d290` (native_move) |
| `0x015a` | `Native_015a` | i, b, a | int: message window i's +0x16 = b (u16), +8 and +0xc = a; 0 | - | int `0x00857c60` (native_mods) |
| `0x015b` | `Native_015b` | window int, event int | the window record | until the window's message wait (Native_MsgWaitEx_Poll) is done, then one more frame (fanfare 0x10001 for a new primer); the result clears the ctrl work's +3 bit 2 and returns 1 | start `0x0085a8a0`, poll `0x0085af70`, int `0x0085b4c0` (native_move) |
| `0x015c` | `Native_015c` | on | int: TkSndSetFootKickStay(on != 0) (0x00887b40); on != 0 | - | int `0x0085b530` (native_mods) |
| `0x015d` | `Native_015d` | v int | int: 0x00870160(v) | - | int `0x00857a80` (native_move) |
| `0x015e` | `Native_015e` | v int | int: 0x008701c0(v) | - | int `0x00857980` (native_move) |
| `0x015f` | `Native_015f` | v int: 0x0088e790(v) | int: v | - | int `0x0085b650` (native_move) |
| `0x0160` | `CT_0160_ret_hasKeyItem` | v int | int: 0x00790800(v) | - | int `0x0085b900` (native_move) |
| `0x0161` | `Native_0161` | v int: 0x0088e780(v) | int: v | - | int `0x0085b760` (native_move) |
| `0x0162` | `Native_0162` | - | int: SndSetKinuStatus(party slot, 1); 1, 0 without a slot | - | int `0x00859e90` (native_mods) |
| `0x0163` | `Native_0163` | slot | int: SndSetKinuStatus(slot, 1); 1 | - | int `0x0085a0f0` (native_mods) |
| `0x0164` | `Native_0164` | - | int: SndSetKinuStatus(party slot, 0); 1, 0 without a slot | - | int `0x00859f70` (native_mods) |
| `0x0165` | `Native_0165` | slot | int: SndSetKinuStatus(slot, 0); 1 | - | int `0x0085a1d0` (native_mods) |
| `0x0166` | `Native_0166` | track int: 0x00886a20, current music = track | 1 | - | int `0x008598f0` (native_move) |
| `0x0167` | `Native_0167` | - | 1: 0x008869f0() | - | int `0x00859c40` (native_move) |
| `0x0168` | `Native_0168` | v | int: 0x00886950(v); v | - | int `0x0085b990` (native_mods) |
| `0x0169` | `Native_0169` | b, a | int: 0x008868e0(b, a); 1 | - | int `0x0085bab0` (native_mods) |
| `0x016a` | `Native_016a` | b int, a int: 0x00886ab0(b, a) | int: a | - | int `0x0085bbb0` (native_move) |
| `0x016b` | `Native_016b` | c int, b int, a int: 0x00886b40(c, b, a) | int: a | - | int `0x0085be70` (native_move) |
| `0x016c` | `Native_016c` | - | 1: the current music stopped (0x00886a60), current music -1 | - | int `0x008594c0` (native_move) |
| `0x016d` | `Native_016d` | track int: 0x00886a60 (the current music cleared when it is track) | 1 | - | int `0x008595b0` (native_move) |
| `0x016e` | `Native_SetMapMarkerPos` | x int, y int: map marker (record +0xb4, +0xb6) | 1 | - | int `0x0085c2d0` (native_move) |
| `0x016f` | `Native_016f` | v int: record 0x0088c830() +0xb8 | int: v | - | int `0x0085c430` (native_move) |
| `0x0170` | `Native_0170` | b, a | int: the save block's (0x0088c830()) +0xb8 = a, +0xba = b (u16s), TkBackupEventJump(block) (0x0088e800); a | - | int `0x0085c530` (native_mods) |
| `0x0171` | `Native_CharRestoreHp` | chr int: party character | int (HP restored to the maximum) | - | int `0x0085c680` (native) |
| `0x0172` | `Native_CharRestoreMp` | chr int | int (MP restored to the maximum) | - | int `0x0085c860` (native) |
| `0x0173` | `Native_0173` | - | float: the position block's +0x24 | - | float `0x0085e7a0` (native_mods) |
| `0x0174` | `Native_0174` | - | float: the position's +0x30 | - | float `0x0085e3a0` (native_move) |
| `0x0175` | `Native_0175` | i | float: worker i's position block +0x24 | - | float `0x0085eec0` (native_mods) |
| `0x0176` | `Native_0176` | i | float: worker i's position block +0x30 | - | float `0x0085ec30` (native_mods) |
| `0x0177` | `Native_0177` | on int: +0x36 bit 6 (and bit 15 when on) | int: on != 0 | - | int `0x0085c9c0` (native_move) |
| `0x0178` | `Native_0178` | i | int: AtelGetEffectFlag(i) (0x0086b030) | - | int `0x0085cc30` (native_mods) |
| `0x0179` | `Native_0179` | v int | int: v | until 0x0086b030(v) is set | start `0x0085cd90`, poll `0x0085cea0` (native_move) |
| `0x017a` | `Native_017a` | v int: 0x0086fa40(v) (map 0x5e room 0x186: 9/10 set/clear [0x012fb8b4]) | int: v | - | int `0x0085d210` (native_move) |
| `0x017b` | `Native_017b` | v int: 0x00863860(v) | int: v | - | int `0x0085d3d0` (native_move) |
| `0x017c` | `Native_017c` | track int: 0x008869e0, 0x008868b0, current music = track | 1 | - | int `0x0085d480` (native_move) |
| `0x017d` | `Native_017d` | id | int: SndMusicMultiStart(id) (0x008868b0), AtelRegistMusicNumber(id) (0x00873d00); 1 | - | int `0x0085d680` (native_mods) |
| `0x017e` | `Native_017e` | track int, v int: 0x00886830(track, v) (or 0x008869e0/0x00886a20), current music = track | 1 | - | int `0x0085d830` (native_move) |
| `0x017f` | `Native_017f` | v int: 0x00886ab0(current music, v) | int: the current music | - | int `0x0085bd10` (native_move) |
| `0x0180` | `Native_0180` | a int, b int | int: i = 0x00873b00(), with 0x00886b40(i, a, b) when i >= 0 | - | int `0x0085c000` (native_move) |
| `0x0181` | `Native_0181` | - | float: the position's +0x38 | - | float `0x0085e550` (native_move) |
| `0x0182` | `Native_0182` | i | float: worker i's position block +0x38 | - | float `0x0085ed10` (native_mods) |
| `0x0183` | `Native_0183` | a f | float: Script_StopTurn(worker, -1) (0x00863fe0), position block +0x38 = Math_WrapAngle(a), AtelSetMoveHokanMode(worker, 0) (0x008703b0), and AtelSetBindObject(worker) (0x0086f4c0) for types 4 and 5; that angle | - | float `0x0085f690` (native_mods) |
| `0x0184` | `Native_0184` | v int (-128..127): 0x00886a90(v) | 1 | - | int `0x0085dac0` (native_move) |
| `0x0185` | `Native_0185` | track int, v int (-128..127): 0x00886aa0(track, v) | 1 | - | int `0x0085dbb0` (native_move) |
| `0x0186` | `Native_0186` | pitch, b | int: SndMusicPitchDirect(pitch, b) (0x00886930), pitch clamped to -0x80..0x7f and b to 0..0xff; 1 | - | int `0x0085dd50` (native_mods) |
| `0x0187` | `Native_0187` | id, pitch, frames | int: SndMusicPitchTranslate(id, pitch, frames) (0x00886940), pitch clamped to -0x80..0x7f and frames to 0..0xff; id | - | int `0x0085df00` (native_mods) |
| `0x0188` | `Native_MusicFadeOut` | frames int: music fade-out (0 or less: stop at once) | int: frames | - | int `0x0085e290` (native_move) |
| `0x0189` | `Native_0189` | v int: 0x00873e60(v) | int: v | nothing (done at once) | start `0x0085e890`, poll `0x0085ea30` (native_move) |
| `0x018a` | `Native_018a` | v int: 0x008702f0(v) | int: v | - | int `0x0085ebb0` (native_move) |
| `0x018b` | `Native_018b` | v int: 0x00863af0(v) | int: v | - | int `0x0085ec90` (native_move) |
| `0x018c` | `Native_018c` | id, pitch, x | int: SndSePitchDirect(id, pitch, x) (0x00886d20); 1 | - | int `0x0085ed60` (native_mods) |
| `0x018d` | `Native_018d` | id, a, pitch, b | int: SndSePitchTranslate(id, a, pitch, b) (0x00886d50); 1 | - | int `0x0085ef00` (native_mods) |
| `0x018e` | `Native_018e` | v | int: 0x00873d20(v), SndVagStreamVolumeDirect(v) (0x00887990); 0 | - | int `0x0085f110` (native_mods) |
| `0x018f` | `Native_018f` | b int, a int: 0x00873d20(a), 0x008879b0(b, a) | 0 | - | int `0x0085f240` (native_move) |
| `0x0190` | `Native_0190` | v | int: 0x00873d10(v), 0x00887970(v); 0 | - | int `0x0085f330` (native_mods) |
| `0x0191` | `Native_0191` | b, a | int: 0x00873d10(a), 0x00887980(b, a); 0 | - | int `0x0085f5c0` (native_mods) |
| `0x0192` | `Native_0192` | v f: the ctrl work's +0x1f0 | float: v | - | float `0x0085f700` (native_move) |
| `0x0193` | `Native_0193` | track int: request or start (0x00886ff0(track, 0xc, 0x7f or 0x3f)) unless marked | int: track | - | int `0x0085f4e0` (native_move) |
| `0x0194` | `Native_0194` | v int: 0x0086f060(v) | int: v | - | int `0x0085f8b0` (native_move) |
| `0x0195` | `Native_0195_Poll` | - | 5 when done | until 0x0086b060() is 0 (poll only) | poll `0x0085fb00` (native_move) |
| `0x0196` | `Native_0196` | v int: 0x00862800(worker, v, +0xa8) for attached workers | 1 | - | int `0x0085fce0` (native_move) |
| `0x0197` | `Native_0197` | v int: +0xa8 (+0xaa = 5) | 0 | - | int `0x0085edd0` (native_move) |
| `0x0198` | `Native_0198` | c int, b int, a int: 0x00908330(c, b, a) for +0xaa 5 | int: 1, else 0 | - | int `0x0085ffa0` (native_move) |
| `0x0199` | `Native_0199` | b int, a int: 0x00908330(+0xa8, b, a) for +0xaa 5 | int: 1, else 0 | - | int `0x00860200` (native_move) |
| `0x019a` | `Native_019a` | obj int, _ int, slot int | 1; with a character, yiAttachChrDObj(obj, chr, slot) | - | int `0x00856040` (native_sys) |
| `0x019b` | `Native_019b` | b int, a int | 1; with a character, yiAttachChrDObj(+0xa8, chr, a) (through a push and pop of +0xa8, b, a) | - | int `0x008563e0` (native_sys) |
| `0x019c` | `Native_AlwaysTrue019c` | - | 1 | - | int `0x00856630` (native) |
| `0x019d` | `Worker_Detach` | v int: also release the model when nonzero | 0: detaches the worker (+0xaa, +0xa8 = 0) | - | int `0x0085ca00` (native_move) |
| `0x019e` | `Native_019e` | - | int: 0x0086c530(0) | - | int `0x00856950` (native_move) |
| `0x019f` | `Native_019f` | v int | int: 0x0086c530(v) | - | int `0x008567a0` (native_move) |
| `0x01a0` | `Native_01a0` | track int, b int, a int: as 0x0193 with volume a (halved when [record +0x28]) and 0x00886e20(track, 0xc, b) | int: track | - | int `0x0085f940` (native_move) |
| `0x01a1` | `Native_01a1` | i, a f, b f, c f, d f | int: yiSetMIMeParamDObj(i, &(a, b, c, d)) (0x0090a4d0); 1 | - | int `0x00856b10` (native_mods) |
| `0x01a2` | `Native_01a2` | - | int: pushes a 0 (the fourth float), then as 0x01a1; 1 | - | int `0x00856e90` (native_mods) |
| `0x01a3` | `Native_01a3` | - | int: pushes two 0s, then as 0x01a1; 1 | - | int `0x00857210` (native_mods) |
| `0x01a4` | `Native_01a4` | i int, x f: 0x0090a4d0(i, &[x, 0, 0, 0]) | 1 | - | int `0x00857360` (native_move) |
| `0x01a5` | `Native_01a5` | index int, frame int: texture swap frame (yiTexSwapSetFrame) | 1 | - | int `0x008574c0` (native_map) |
| `0x01a6` | `Native_01a6` | - | 1: fills the empty party slots with the lowest free characters 0..7 | - | int `0x008576d0` (native_move) |
| `0x01a7` | `Native_01a7` | v int: event | the args or 0x00798b60() | until a new primer's fanfare (0x10001) has played; the result runs 0x00798b60() for a new primer and returns 1 | start `0x00857b70`, poll `0x00857ef0`, int `0x00858230` (native_move) |
| `0x01a8` | `Native_01a8` | chr int | int: +0x54 of 0x00785330(chr) | - | int `0x00858390` (native_move) |
| `0x01a9` | `Native_01a9` | i int | int: the word +0x50 of the record 0x00785330(i) | - | int `0x00858500` (native_sys) |
| `0x01aa` | `Native_01aa` | i int | int: the word +0x58 of the record 0x00785330(i) | - | int `0x00858690` (native_sys) |
| `0x01ab` | `Native_01ab` | as 0x00784f50 | as 0x00784f50 (a jump) | - | int `0x00858800` (native_move) |
| `0x01ac` | `Native_01ac` | v int | int: 0x00790af0(v) | - | int `0x00858950` (native_move) |
| `0x01ad` | `Native_01ad` | v int | int: 0x00790b90(v, 1) | - | int `0x00858b30` (native_move) |
| `0x01ae` | `Native_01ae` | v int | int: 0x00790b90(v, -1) | - | int `0x00858df0` (native_sys) |
| `0x01af` | `Native_01af` | b int, a int | int: 0x00790b90(b, a) | - | int `0x00858e70` (native_move) |
| `0x01b0` | `Native_01b0` | b int, a int: 0x00873eb0(b, a) (the original's printf of both left out) | int: b | - | int `0x008590a0` (native_move) |
| `0x01b1` | `Native_01b1` | - | 1: 0x00873db0() | - | int `0x00859240` (native_move) |
| `0x01b2` | `Native_01b2` | v int | int: 0x007904b0(v) | - | int `0x00859360` (native_move) |
| `0x01b4` | `Native_01b4` | - | int: TkSndIsReadShoutVoice() != 0 (0x00887b00) | - | int `0x00859490` (native_mods) |
| `0x01b5` | `Native_01b5` | - | 0x00644280(1), then as 0x00874060 | - | int `0x00859580` (native_move) |
| `0x01b6` | `Native_01b6` | - | int: 0x007ad6e0() != 0 | - | int `0x00859630` (native_move) |
| `0x01b7` | `Native_01b7` | v int: 0x007ad640(v) when 0x007ad6e0() | int: 1, else 0 | - | int `0x00859800` (native_move) |
| `0x01b8` | `Native_01b8` | - | int: 0x007ad710(1) | - | int `0x00859ba0` (native_move) |
| `0x01b9` | `Native_01b9` | - | int: 0x007ad710(0)'s EAX | - | int `0x00859d20` (native_mods) |
| `0x01ba` | `Native_01ba` | worker int, on int (0..1) | int: 0x0086b150(its bound character, on), 0 for none | - | int `0x0085beb0` (native_move) |
| `0x01bb` | `Native_01bb` | - | int: 0x008863c0(0) | - | int `0x00859de0` (native_move) |
| `0x01bc` | `Native_01bc` | v int: 0x00870140(v) | 1 | - | int `0x00859ec0` (native_move) |
| `0x01bd` | `Native_01bd` | v int: 0x008715f0(v) | 1 | - | int `0x00859ff0` (native_move) |
| `0x01be` | `Native_01be` | - | int: jumps to AtelGetWaterBattleEffect (0x0086c8a0) | - | int `0x0085a0e0` (native_mods) |
| `0x01bf` | `Native_01bf` | - | 1: 0x008708d0() | - | int `0x0085a190` (native_move) |
| `0x01c0` | `Native_01c0` | i int (0..3), v int: 0x0086fa60(i, v) | int: v | - | int `0x0085a250` (native_move) |
| `0x01c1` | `Native_01c1` | - | int: 0x00873c40(); 1 | - | int `0x0085a420` (native_mods) |
| `0x01c2` | `Native_01c2` | - | 1: 0x00873d30() | - | int `0x0085a570` (native_move) |
| `0x01c3` | `Native_01c3` | v int: 0x00874030(v) | int: v | - | int `0x0085a750` (native_move) |
| `0x01c4` | `Native_01c4` | on int: 0x00873dd0(on != 0) | int: on != 0 | - | int `0x0085a840` (native_move) |
| `0x01c5` | `Native_01c5` | b int, a int: Al Bhed dictionary mode (args +0xa = b, +0xb = a) | int: the outcome (0, 2, 3, 4 or 6) | for worker +0xa8 0x5045: while 0x00822540() is -1 and 0x00648100() is 0 | start `0x0085b1a0`, poll `0x0085b590`, int `0x0085d8f0` (native_move) |
| `0x01c6` | `Native_01c6` | as 0x0088e5e0 | as 0x0088e5e0 (a jump) | - | int `0x0085de70` (native_move) |
| `0x01c7` | `Native_01c7` | - | int: jumps to 0x008741d0 | - | int `0x0085e000` (native_mods) |
| `0x01c8` | `Native_01c8` | v int | 1 when the worker is attached as kind 5 (+0xaa): yiDisposeAnimDObj(+0xa8, v) (through a push and pop of both); else 0 | - | int `0x00855e50` (native_sys) |
| `0x01c9` | `Native_01c9` | b int, a int: 0x009084b0(b, a) | 1 | - | int `0x00860730` (native_move) |
| `0x01ca` | `Native_01ca` | - | 1: the worker's +0x38 bit 2 | - | int `0x0085e340` (native_move) |
| `0x01cb` | `Native_01cb` | i | int: worker i's +0x38 gets bit 2; 1 | - | int `0x0085e0e0` (native_mods) |
| `0x01cc` | `Native_01cc` | as 0x008863f0 | as 0x008863f0 (a jump) | - | int `0x0085e400` (native_move) |
| `0x01cd` | `Native_01cd` | - | int: jumps to AtelGetMusicNumber (0x00873b00) | - | int `0x0085e5d0` (native_mods) |
| `0x01ce` | `Native_01ce` | - | 1: the character's 0x007fb290/0x007fbab0, +0x38 bit 4 | - | int `0x0085e7c0` (native_move) |
| `0x01cf` | `Native_01cf` | - | 1: the character's 0x007fba40, +0x38 bit 4 cleared | - | int `0x0085ea40` (native_move) |
| `0x01d0` | `Native_01d0` | - | 1: the worker's +0x38 bit 3 when it has a character | - | int `0x0085fbb0` (native_move) |
| `0x01d1` | `Native_WaitFrames_01d1` | frames int | 1 when done | for frames frames (at once when skipping) | start `0x0085fd20`, poll `0x0085ff70` (native) |
| `0x01d2` | `Native_01d2` | msg int | int: 1 when its text equals record 0x0088c830() +0xec | - | int `0x00860180` (native_move) |
| `0x01d3` | `Native_01d3` | - | int: 1 (mov eax, 1; ret) | - | int `0x00860720` (native_mods) |
| `0x01d4` | `Native_01d4` | slot int, on int: the slot model's 0x0082aa30 and +0xb08 bit 0 | int: on != 0 | - | int `0x00855dd0` (native_move) |
| `0x01d5` | `Native_01d5` | a int, b int | 1; GetCD()'s +0x88(a, b) | - | int `0x008561e0` (native_sys) |
| `0x01d6` | `Native_01d6` | id | int: voice id's stream position (SndStreamDVDPos, 0x00887790; 0 when it is -1) and the two words 0x00885ba0(id, &a, &b) gives through 0x00885bf0(0, &b, &a, id, pos) into the CD object's (GetCD, 0x0076d0d0) method +0x8c; that value | - | int `0x008564b0` (native_mods) |
| `0x01d7` | `Native_01d7` | v int | int: v; GetCD()'s +0x8c(v) | - | int `0x00856740` (native_sys) |
| `0x01d8` | `Native_01d8` | stream, entry, from, to, len | int: Vibrate_StartLevel(stream, entry, from, to, len) (0x00876050), the first four clamped to 0..7, 0..1, 0..0x7f, 0..0x7f; 1 | - | int `0x008567e0` (native_mods) |
| `0x01d9` | `Native_01d9` | - | 1: text 0x0078fcf0(3, 0x202c, ..) copied to 0x00784fb0(6) | - | int `0x008582e0` (native_move) |
| `0x01da` | `Native_01da` | d int, c int, b int, a int: 0x00876050(3, d, c, b, a) | 1 | - | int `0x00856a80` (native_move) |
| `0x01db` | `Native_01db` | v int | int: v; 0x00876290(v) | - | int `0x00856e00` (native_sys) |
| `0x01dc` | `Native_01dc` | - | 3: 0x00876290(3) | - | int `0x008571d0` (native_move) |
| `0x01dd` | `Native_01dd` | v int | v, kept in the argument word | as Native_01de_Poll (its poll 0x008577f0 is a copy) | start `0x00857660`, poll `0x008577f0` (native_sys) |
| `0x01de` | `Native_01de` | - | 3 | until 0x00875de0(3) is 0 | start `0x00857920`, poll `0x008579d0` (native_move) |
| `0x01df` | `Native_01df` | i | int: worker i's +0x38 gets bit 3 when it is a character (+0xaa 1); 1 | - | int `0x0085f8d0` (native_mods) |
| `0x01e0` | `Native_01e0` | v int: 0x0082a9d0(character, v) | int: 1 with a character, else 0 | - | int `0x00858490` (native_move) |
| `0x01e1` | `Native_01e1` | - | int: 0x0082a9a0(chr) for a worker attached as kind 1 (+0xaa) with a character, else 0 | - | int `0x008586e0` (native_sys) |
| `0x01e2` | `Native_01e2` | as 0x009085a0 | as 0x009085a0 (a jump) | - | int `0x00858940` (native_move) |
| `0x01e3` | `Native_01e3` | as 0x00908450 | as 0x00908450 (a jump) | - | int `0x00858ab0` (native_move) |
| `0x01e4` | `Native_01e4` | a f, b f, c f, n int: yiSetFieldVec3Int | int: 0 | - | int `0x00858c90` (native_map) |
| `0x01e5` | `Native_01e5` | c int, b int, a int | int: 0x0090a400(c, b, a) | - | int `0x00859000` (native_move) |
| `0x01e6` | `Native_01e6` | on int, a f, b f: yiSetFieldPair | int: on | - | int `0x008591d0` (native_map) |
| `0x01e7` | `Native_01e7` | track int, v int: 0x008869e0, 0x008868f0(track, v), current music = track | 1 | - | int `0x00859320` (native_move) |
| `0x01e8` | `Native_01e8` | v | int: SndSepClear(v) (0x00886d80); 1 | - | int `0x00859470` (native_mods) |
| `0x01e9` | `GetField0_008595f0` | window int, wait int | int: the window (0x008640f0(window) at the start) | while wait is set, until the window's state (+0x14) is 0 or 4 | start `0x008590f0`, poll `0x008592c0`, int `0x008595f0` (native_move) |
| `0x01ea` | `Native_01ea` | var int | 1; 0x008715e0(the variable's address) | - | int `0x00857aa0` (native_sys) |
| `0x01eb` | `Native_01eb` | v int: 0x00875fb0(1, v, 0x0088d590()) | 1 | - | int `0x00858070` (native_move) |
| `0x01ec` | `Native_01ec` | b, a | int: TkSetVibrateStream(b, a, stream) (0x00875fb0) with the current ctrl work's +0x44 when it is set; 1 | - | int `0x00857cf0` (native_mods) |
| `0x01ed` | `Native_01ed` | - | debug (mod-tool candidate): an endless loop: dbgPrintf, the top popped and pushed back with a 2 under it, forever | - | int `0x00857f70` (not ported) |
| `0x01ee` | `Native_01ee` | - | 1: 0x00876290(1) | - | int `0x00857440` (native_move) |
| `0x01ef` | `Native_01ef` | - | int: 2 pushed and popped back (the old top on a full stack), handed to 0x00876290 | - | int `0x00857320` (native_sys) |
| `0x01f0` | `Native_01f0` | v f: 0x00873e50(v) | float: v | - | float `0x00859550` (native_move) |
| `0x01f1` | `Native_01f1` | - | float: jumps to AtelGetHalfTurnAngle (0x00873ad0) | - | float `0x00859620` (native_mods) |
| `0x01f2` | `Native_01f2` | - | int: bits 8..15 for the characters 8..15 that 0x00785380 reports | - | int `0x008597d0` (native_move) |
| `0x01f3` | `Native_01f3` | enabled int, all int, dst var | int: k << 8 (k choices): a choice text of characters 8..15 | - | int `0x00859990` (native_move) |
| `0x01f4` | `Native_01f4` | n int, mask int | int: the n-th set bit of mask among bits 8..15, -1 when fewer | - | int `0x00859fa0` (native_move) |
| `0x01f5` | `Native_01f5` | on | int: TkMenuSetDummyControlerFlag(on != 0) (0x008ca570); on != 0 | - | int `0x0085a160` (native_mods) |
| `0x01f6` | `Native_01f6` | b | int: TkMenuSetDummyControlerPending(1 << b) (0x008ca560); b | - | int `0x0085a2d0` (native_mods) |
| `0x01f7` | `Native_01f7` | a, b, c | int: AtelRegistBattleEndMovieFadeout(a, b, c) (0x0086e840); 0 | - | int `0x0085a800` (native_mods) |
| `0x01f8` | `Native_01f8` | track int, frames int: fade (0 or less: stop the current music) | int: frames | - | int `0x0085e570` (native_move) |
| `0x01f9` | `Native_01f9` | chr int: Native_00cb on it, then 0x00786aa0(chr, 0) | 1 | - | int `0x0085ac50` (native_move) |
| `0x01fa` | `Native_01fa` | as 0x008bef00 | as 0x008bef00 (a jump) | - | int `0x0085af60` (native_move) |
| `0x01fb` | `Native_01fb` | as 0x008beef0 | as 0x008beef0 (a jump) | - | int `0x0085af10` (native_move) |
| `0x01fc` | `CT_01FC_ret_teachAbilityToPartyMemberSilently` | b int, a int | int: 0x00874200(b, a, 1) | - | int `0x0085b170` (native_move) |
| `0x01fd` | `Native_01fd` | b, a | int: MsSetSaveCommandWithPrefix(b, a, 0) (0x00874200) | - | int `0x0085ba20` (native_mods) |
| `0x01fe` | `Native_01fe` | b, a | int: MsGetSaveCommand(b, a) (0x00785020) | - | int `0x0085bb20` (native_mods) |
| `0x01ff` | `Native_01ff` | - | int: SndSetFootStatus(party slot, 1) (0x00887100) and SndSetKinuStatus(party slot, 1), each when the worker has a slot; 1 | - | int `0x0085a300` (native_mods) |
| `0x0200` | `Native_0200` | v int: 0x00887100(v, 1), 0x00887130(v, 1) | 1 | - | int `0x0085a430` (native_move) |
| `0x0201` | `Native_0201` | - | 1: 0x00887100/0x00887130(bound character, 0) | - | int `0x0085a580` (native_move) |
| `0x0202` | `Native_0202` | v int: 0x00887100(v, 0), 0x00887130(v, 0) | 1 | - | int `0x0085a770` (native_move) |
| `0x0203` | `Native_0203` | chr int | int: 0x0085d000's result after 0x00863650(worker id) | until 0x0086ade0() clears, the loader is ready, 0x00887b30()/0x0076f060() are 0, 0x00836a50(chr, 0) and 0x0082a040(chr) are not 1 | start `0x0085d460`, poll `0x0085d550`, int `0x0085dc10` (native_move) |
| `0x0204` | `Native_0204` | v f: position +0x2c | float: v | - | float `0x0085dc90` (native_move) |
| `0x0205` | `Native_0205` | - | 1: 0x00862700(worker, 0) with a character | - | int `0x0085ec00` (native_move) |
| `0x0206` | `Native_0206` | - | 1: 0x00862680(worker, 0) with a character | - | int `0x0085ee10` (native_move) |
| `0x0207` | `Native_0207` | - | 1: 0x00862700(worker, 1) with a character | - | int `0x0085f010` (native_move) |
| `0x0208` | `Native_0208` | - | int: AtelBindEff2OffSome(worker, 1) (0x00862680) for a character (+0xaa 1) with a model; 1 | - | int `0x0085f210` (native_mods) |
| `0x0209` | `Native_0209` | - | 1: 0x00862700(worker, 2) with a character | - | int `0x0085f430` (native_move) |
| `0x020a` | `Native_020a` | - | 1: 0x00862680(worker, 2) with a character | - | int `0x0085f660` (native_move) |
| `0x020b` | `Native_020b` | - | 1: the worker's +0x38 bits 8 and 2 | - | int `0x0085be30` (native_move) |
| `0x020c` | `Native_020c` | i | int: worker i's +0x38 gets bit 8; i pushed back and popped, and that worker's +0x38 gets bit 2; 1 | - | int `0x0085bcc0` (native_mods) |
| `0x020d` | `Native_020d` | v int: 0x00870fa0(v) | int: v | - | int `0x0085bf70` (native_move) |
| `0x020e` | `Native_020e` | - | int: AtelGetSphereMonitor() (0x0086c4b0), its low word | - | int `0x0085c090` (native_mods) |
| `0x020f` | `Native_020f` | v int: 0x00870130(v), then the ctrl work 0x0086c1f0() made current | int: the previous ctrl (also to 0x00780c70) | - | int `0x0085c100` (native_move) |
| `0x0210` | `Native_BitFlagSet` | bit int | int: bit (set) | - | int `0x0085c3b0` (native_move) |
| `0x0211` | `Native_0211` | i | int: AtelSetHelpEventFlag(i, 0) (0x008701f0); i | - | int `0x0085c810` (native_mods) |
| `0x0212` | `Native_0212` | on int, v f: the ctrl work's +0x208 = v, +2 bit 0 = on | int: on != 0 | - | int `0x0085c950` (native_move) |
| `0x0213` | `Native_ChrGetAttachBone` | bone int | int: 0x00835660(character, bone, 0), 0 without a character | - | int `0x0085ce30` (native_move) |
| `0x0214` | `Native_0214` | i, e | int: Ch_OptposGetElement(model, e, 0) (0x00835660) for worker i when it is a character (+0xaa 1) with a model (+0x9c), else 0 | - | int `0x0085cbe0` (native_mods) |
| `0x0215` | `Native_0215` | i int (0..17), v int (1..2) | int: 0x008c3170(i, v), 0 out of range | - | int `0x0085d170` (native_move) |
| `0x0216` | `CT_0216_ret_teachAbilityToPartyMemberWithMsg` | window int, item int, n int | the window record (or 0x00785020's refusal) | until the window's message wait is done, then one more frame; the result clears the ctrl work's +3 bit 2 and returns 0x00874200(item, n, 1) | start `0x0085b280`, poll `0x0085b5d0`, int `0x0085b920` (native_move) |
| `0x0217` | `Native_0217` | - | 1: 0x00798b60() | - | int `0x0085d3a0` (native_move) |
| `0x0218` | `Native_0218` | i | int: MsGetMonsterCapture(i) (0x00790af0) | - | int `0x0085d3f0` (native_mods) |
| `0x0219` | `Native_0219` | b, a | int: AddMonsterCapture(b, a) (0x00790b90) | - | int `0x0085d500` (native_mods) |
| `0x021a` | `Native_021a` | b int, a int | int: 0x00872aa0(0, b, a, 0) | - | int `0x0085d780` (native_move) |
| `0x021b` | `Native_021b` | b int, a int | int: 0x00872aa0(1, b, a, 0) | - | int `0x0085d8c0` (native_move) |
| `0x021c` | `Native_021c` | id | int: AtelCheckCaptMonster(2, 0, id, 0) (0x00872aa0) | - | int `0x0085db90` (native_mods) |
| `0x021d` | `Native_021d` | - | int: 1 when a primer flagged +3 bit 3 was handed out (0x007ab970, 0x008c30d0) | - | int `0x0085dcd0` (native_move) |
| `0x021e` | `Native_021e` | - | 1: 0x0090a420() | - | int `0x00858e40` (native_move) |
| `0x021f` | `Native_021f` | i | int: AtelSetLoveParamDisable(i, 0) (0x00870250); i | - | int `0x0085dfc0` (native_mods) |
| `0x0220` | `Native_0220` | v int: 0x00870250(v, 1) | int: v | - | int `0x0085e010` (native_move) |
| `0x0221` | `Native_0221` | i int | int: word i of the table at 0x00c86e28 | - | int `0x0085e110` (native_move) |
| `0x0222` | `Native_0222` | i | int: the word at 0x00c8756c + 4 i | - | int `0x0085e380` (native_mods) |
| `0x0223` | `Native_0223` | on | int: bit 4 of the save menu word (MsGetSaveMenu, 0x00785290) = on != 0, TkRefreshActuator() (0x00872480); on != 0 | - | int `0x0085e460` (native_mods) |
| `0x0224` | `Native_0224` | - | int: bit 4 of the save menu word | - | int `0x0085e800` (native_mods) |
| `0x0225` | `Native_0225` | v int: 0x00870f60(v) | int: v | - | int `0x0085ea10` (native_move) |
| `0x0226` | `Native_0226` | on int: 0x00729050(on != 0) | int: on != 0 | - | int `0x0085eb80` (native_move) |
| `0x0227` | `Native_0227` | on | int: 0x00873dd0(on != 0); on != 0 | - | int `0x0085ac20` (native_mods) |
| `0x0228` | `Native_0228` | flags, b, a | int: how far the current level's turn target (+0x884 + level * 0x44) is from the position block's yaw (+0x34), wrapped (Math_WrapAngle) and made absolute, over the double at 0x00b92288 (flags bit 1) or 0x00b11338, at most 1, times a (bit 1) or b, truncated; each step rounded to f32 | - | int `0x0085b060` (native_mods) |
| `0x0229` | `SetFieldBits38_0085ece0` | - | 1: the worker's +0x38 bit 9 | - | int `0x0085ece0` (native_move) |
| `0x022a` | `SetFieldBits38_0085ed40` | - | int: the worker's +0x38 gets bit 10; 1 (the auto-port's shim stays) | - | int `0x0085ed40` (native_mods) |
| `0x022b` | `SetFields_0085eea0` | - | args +0 = 0, +4 = 10 (the auto-port's shim stays); the args address | - | start `0x0085eea0`, poll `0x0085efb0` (native_mods) |
| `0x022c` | `Native_022c` | i, v | int: SetByte_0186a429_008ca370(i, v); 1 | - | int `0x0085f2e0` (native_mods) |
| `0x022d` | `Native_022d` | mask | args +0 = mask; mask | - | start `0x0085f4c0`, poll `0x0085f600` (native_mods) |
| `0x022e` | `Native_022e` | d int, c int, b int, a int: 0x00870280(d, c, b, a) | 1 | - | int `0x0085f860` (native_move) |
| `0x022f` | `Native_022f` | slot int, on int: the slot model's 0x0082b970 and +0xb08 bit 1 | int: on != 0 | - | int `0x00855fb0` (native_move) |
| `0x0230` | `SetFieldBits38_0085fb10` | - | int: the worker's +0x38 gets bit 11; 1 (the auto-port's shim stays) | - | int `0x0085fb10` (native_mods) |
| `0x0231` | `Native_0231` | worker int: its +0x38 bit 11 | 1 | - | int `0x0085fca0` (native_move) |
| `0x0232` | `Native_0232` | b | args +0 = 1 << b, +4 = ReturnConst2_00a54750() (2); that count | - | start `0x0085a4b0`, poll `0x0085a5f0` (native_mods) |
| `0x0233` | `Native_0233` | v | int: TkMn2SetMainFrameRatioDiff(v) (0x008ab260); v | - | int `0x0085fe10` (native_mods) |
| `0x0234` | `Native_0234` | - | 1: 0x0086a040() | - | int `0x0085fff0` (native_move) |
| `0x0235` | `Native_0235` | as 0x008729d0 | as 0x008729d0 (a jump) | - | int `0x00860170` (native_move) |
| `0x0236` | `Native_0236` | msg int | int: the message text (0x0086b170 or 0x0086bf30) | - | int `0x00860350` (native_move) |
| `0x0237` | `Native_0237` | n int, mask int, list var, dst var: list of n string pointers; mask bit i enables entry i | int: k << 8 (k choices): the choice text in dst (copied once more after its end) | - | int `0x008604c0` (native_move) |
| `0x0238` | `Native_0238` | i | int: bit i of the save block's bit set at +0x4c (0 past it) | - | int `0x0085eb30` (native_mods) |
| `0x0239` | `Native_0239` | i int (below 640): bit i of record 0x0088c830() +0x4c set | 1 | - | int `0x0085e8b0` (native_move) |
| `0x023a` | `Native_023a` | v int: 0x0086f9e0(v) | 1 | - | int `0x00856250` (native_move) |
| `0x023b` | `Native_023b` | v int | int: 0x0086b9e0(v) | - | int `0x0085c490` (native_move) |
| `0x023c` | `Native_023c` | b int, n int | int: 1 when 0x0086b9e0(b + i) is set for every i below n | - | int `0x0085c600` (native_move) |
| `0x023d` | `Native_023d` | - | 1: 0x00647e40, 0x00642390(7) when [0x025d5ebc], 0x00786b00, 0x00782050, 0x0088d1c0(0), 0x00643e70 | - | int `0x00856520` (native_move) |
| `0x023e` | `Native_023e` | id int | int: id; SndSepLock(id) (0x00886dd0) | - | int `0x00856640` (native_sys) |
| `0x023f` | `Native_023f` | v int: 0x008749e0(v) | int: v | - | int `0x00856770` (native_move) |
| `0x0240` | `Native_0240` | b int, on int: 0x007ad5e0(b, on != 0) | int: on != 0 | - | int `0x008568d0` (native_move) |
| `0x0241` | `CT_0241_ret` | on int: field state bit 3 | int: on != 0 | - | int `0x00856ad0` (native) |
| `0x0242` | `Native_0242` | as 0x00777e90 | as 0x00777e90 (a jump) | - | int `0x00857200` (native_move) |
| `0x0243` | `Native_0243` | as 0x007787b0 | as 0x007787b0 (a jump) | - | int `0x00857350` (native_move) |
| `0x0244` | `Native_0244` | - | 1: 0x007fb690() | - | int `0x008574b0` (native_move) |
| `0x0245` | `Native_0245` | - | 1: 0x007fb680() | - | int `0x00857690` (native_move) |
| `0x0246` | `Native_0246` | - | int: 0x007fb730(); 1 | - | int `0x00857810` (native_mods) |
| `0x0247` | `Native_0247` | - | int: 0x007fb720(); 1 | - | int `0x00857910` (native_mods) |
| `0x0248` | `Native_0248` | - | int: 0x007fb520(); 1 | - | int `0x008579f0` (native_mods) |
| `0x0249` | `Native_0249` | - | int: 0x007fb510(); 1 | - | int `0x00857b60` (native_mods) |
| `0x024a` | `Native_024a` | on int: bit 13 of *0x00785290(), then 0x008724a0() | int: on != 0 | - | int `0x00857cb0` (native_move) |
| `0x024b` | `Native_024b` | slot int, on int: 0x0082b2c0(model, on) for a used slot with a motion | int: on != 0 | - | int `0x00857e30` (native_move) |
| `0x024c` | `SetFieldBits38_008580a0` | - | 1: the worker's +0x38 bit 12 | - | int `0x008580a0` (native_move) |
| `0x024d` | `Native_024d` | on int: the ctrl work's +0x210 bit 6 | int: on != 0 | - | int `0x00856dc0` (native_move) |
| `0x024e` | `Native_024e` | v | int: the current ctrl work's +0x214 = v; v | - | int `0x0085d4b0` (native_mods) |
| `0x024f` | `Native_024f` | - | int: the buttons of 0x0086af60() through Pad_MapButtons | - | int `0x0085d890` (native_mods) |
| `0x0250` | `Native_0250` | - | int: 0x0086af60(), the low word | - | int `0x0085de60` (native_mods) |
| `0x0251` | `Native_0251` | i int | int: bit i of the flag word for 0x0086af60() | - | int `0x0085e5e0` (native_move) |
| `0x0252` | `Native_0252` | b | int: bit b of 0x0086af60() | - | int `0x0085ef50` (native_mods) |
| `0x0253` | `Native_LangGroupNotJpAsia` | - | int: 1 unless the language group (0x00641100) is 0, 3 or 4 | - | int `0x00858210` (native_move) |
| `0x0254` | `Native_0254` | v int: bits 14..15 of *0x00785290(), then 0x00a53de0(0x0112ec7c) | int: v | - | int `0x008585d0` (native_move) |
| `0x0255` | `Native_0255` | - | int: bits 14..15 of *0x00785290() | - | int `0x00858430` (native_move) |
| `0x0256` | `Native_0256` | window int: +0x28 = 0x8000 | int: window | - | int `0x00858710` (native_move) |
| `0x0257` | `Native_0257` | window int: +0x28 = 0 | int: window | - | int `0x008588b0` (native_move) |
| `0x0258` | `Native_0258` | on | when on != 0 differs from 0x0086bbc0(), args +0 = 0, MsSetSaveConfigEnglish(on) (0x00785ed0) and TOSwapInternationalEnvInit(0x0086bbc0()) (0x008b0920, its EAX); else args +0 = -1 | - | start `0x00858a60`, poll `0x00858da0` (native_mods) |
| `0x0259` | `Native_0259` | i int, v f: 0x00872530(i, v) | float: v | - | float `0x00858fc0` (native_move) |
| `0x025a` | `Native_025a` | - | int: jumps to 0x0086a7f0 | - | int `0x008590e0` (native_mods) |
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
| `0x1003` | `AtelCall_Math_Tan_RF` | v f | float: CItan(v) (0x0088c420) | - | float `0x00877ad0` (native_mods) |
| `0x1004` | `AtelCall_Math_ATan_RF` | v f | float: atan(v) through the game's _CIatan (import thunk 0x00949d5c), rounded to f32 | - | float `0x00877b00` (native_mods) |
| `0x1005` | `AtelCall_Math_ATan2_RF` | y f, x f | float: atan2(y, x) through the game's _CIatan2 (import thunk 0x009498bc), rounded to f32 | - | float `0x00877b30` (native_chr) |
| `0x1006` | `AtelCall_Math_Sqrt_RF` | x f | float: sqrt(x) as f32 | - | float `0x00877b70` (native_move) |
| `0x1007` | `AtelCall_Math_InvSqrt_RF` | v f | float: 1 / sqrt(v), the root and the quotient rounded to f32 | - | float `0x00877ba0` (native_mods) |
| `0x1008` | `Native_1008` | angle int | int: Math_SinTable2(angle) (0x0088c380) | - | int `0x00877bd0` (native_mods) |
| `0x1009` | `Native_1009` | angle int | int: Math_CosTable(angle) (0x0088c120) | - | int `0x00877bf0` (native_mods) |
| `0x100a` | `Native_100a` | v int | int: the popped value; int: the popped value | - | int `0x008778b0` (native_chr) |
| `0x100b` | `Native_100b` | v int | int: v << 15 | - | int `0x008778d0` (native_mods) |
| `0x100c` | `AtelCall_Math_Ftoul_RI` | x f | int: x times the double at 0x00b51588, rounded to f32, truncated (_ftol2_sse) | - | int `0x008778f0` (native_mods) |
| `0x100d` | `Native_100d` | angle f | int: Math_AngleTo15Bits(angle) (0x0088c040) | - | int `0x00877920` (native_mods) |
| `0x100e` | `Native_100e` | angle f | int: Math_AngleTo12Bits(angle) (0x0088bfe0) | - | int `0x00877950` (native_mods) |
| `0x100f` | `Native_100f` | a int | float: Math_AngleFrom15Bits(a) (0x0088c0e0) | - | float `0x00877980` (native_mods) |
| `0x1010` | `Native_1010` | a int | float: Math_AngleFrom12Bits(a) (0x0088c0a0) | - | float `0x008779a0` (native_mods) |
| `0x1011` | `Native_1011` | x0 f, z0 f, x1 f, z1 f | float: Math_Atan2XZ(x0, z0, x1, z1) (0x0088b430) | - | float `0x00877c10` (native_mods) |
| `0x1012` | `Native_1012` | yaw var, pitch var, x0 f, y0 f, z0 f, x1 f, y1 f, z1 f | float: the yaw; Math_YawPitchTo (0x0088b4d0) writes the yaw and pitch from the first point to the second into the variables | - | float `0x00877c80` (native_mods) |
| `0x1013` | `Native_1013` | v f | float: Math_WrapAngle(v) (0x0088bc20) | - | float `0x00877d30` (native_chr) |
| `0x1014` | `Native_1014` | cur f, target f, step f | float: Math_AngleStep(cur, target, step) (0x0088b570) | - | float `0x00877d60` (native_mods) |
| `0x1015` | `Native_1015` | - | six floats: 0x0088b780(f6, ..., f1)'s EAX | - | int `0x00877dc0` (native_chr) |
| `0x1016` | `Native_1016` | r f, x0 f, y0 f, z0 f, x1 f, y1 f, z1 f | int: Math_InRadius(r, x0 - x1, y0 - y1, z0 - z1) (0x0088b730), the differences rounded to f32 | - | int `0x00877e50` (native_mods) |
| `0x1017` | `Native_1017` | out var, points var, n int, t f | 0.0: TkMtCalcSpline(out, points, n, t) (0x0088a4a0) on the variables | - | float `0x00877ef0` (native_mods) |
| `0x1018` | `Native_1018` | out var, a f, b f, c f | 0.0: 0x0088a3a0(out, a, b, c) on the variable | - | float `0x00877f60` (native_mods) |
| `0x1019` | `AtelCall_Math_Abs_RF` | - | |v| rounded to f32 | - | float `0x00877fd0` (native_chr) |
| `0x101a` | `Native_101a` | x1 f, z1 f, x2 f, z2 f | float: sqrt((z2 - z1)^2 + (x2 - x1)^2), f32 steps | - | float `0x00877ff0` (native_chr) |
| `0x101b` | `Native_101b` | x1 f, y1 f, z1 f, x2 f, y2 f, z2 f | float: the 3D distance, as 0x101a | - | float `0x00878060` (native_chr) |
| `0x101c` | `Native_101c` | - | 0x00878140(worker, args, stack)'s record +4 = 0; 0 | - | int `0x008780f0` (native_chr) |
| `0x101d` | `Native_101d` | as 0x00878140 | 0: 0x00878140(worker, args, stack)'s record +4 = 1 (0x101c sets 0) | - | int `0x00878110` (native_mods) |
| `0x4000` | `Native_Empty_00a77d20` | - | 0 (an empty handler) | - | int `0x00a77d20` (native_mods) |
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
| `0x4010` | `Native_4010` | v int, a f, b f | 0 (the three popped and dropped) | - | int `0x00a77f40` (native_mods) |
| `0x4011` | `Native_4011` | i int, x f, y f, z f | 0: Sg_LightSetDirXYZ(i, x, y, z) (0x008364f0) | - | int `0x00a77f70` (native_mods) |
| `0x4012` | `Native_4012` | i int, x f, y f, z f | 0: 0x00836400(i, x, y, z) | - | int `0x00a77fd0` (native_mods) |
| `0x4013` | `Native_LightSet` | light int (0-3), r int, g int, b int, pitch int (deg), yaw int (deg) | 0 | - | int `0x00a78030` (native_move) |
| `0x4014` | `Native_4014` | v int | int: Sg_EnFocus(v) (0x0082cb60) | - | int `0x00a780a0` (native_chr) |
| `0x4015` | `Native_4015` | b int, a int | 0: Sg_SetFocus(b, a) (0x0082ce20) | - | int `0x00a780c0` (native_chr) |
| `0x4016` | `Native_4016` | i int, v f | 0: 0x0082d9e0(i, v) | - | int `0x00a780f0` (native_chr) |
| `0x4017` | `Native_4017` | b int, a int | 0: Sg_IdSetMapTable(b, a) (0x0083e9e0) | - | int `0x00a78130` (native_chr) |
| `0x4018` | `Native_4018` | i int | int: Sg_IdGetMapTable(i) (0x0083e960) | - | int `0x00a78160` (native_mods) |
| `0x4019` | `Native_4019` | v int | 0: 0x0082bdf0(v) | - | int `0x00a78180` (native_chr) |
| `0x401a` | `Native_401a` | b int, a int | 0: Sg_AccSetAlpha(b, a) (0x0082bd10) | - | int `0x00a781a0` (native_chr) |
| `0x401b` | `Native_401b` | v | Sg_AccSetDrawMap(v) (0x0082bcb0) | - | int `0x00a781d0` (native_chr) |
| `0x401c` | `Native_401c` | v | Sg_AccEnableBG(v) (0x0082be10) | - | int `0x00a781f0` (native_chr) |
| `0x401d` | `Native_MenuOpen` | menu int | the start's result; poll 1 when done; int slot the menu result (0x00821740) | one frame (g_MenuWaitFrames) | start `0x00a78210`, poll `0x00a78270`, int `0x00a782a0` (native_move) |
| `0x401e` | `Native_401e` | v int | debug (mod-tool candidate): Sg_Test(v) (0x008467b0: map-test setup, a sync print, a 64 KiB scratch allocation and its release); its result | - | int `0x00a782c0` (not ported) |
| `0x401f` | `Native_401f` | - | 0x00840e40() | - | int `0x00a782e0` (native_chr) |
| `0x4020` | `Native_4020` | - | 0: 0x00840e50() | - | int `0x00a782f0` (native_chr) |
| `0x4021` | `Native_4021` | v int | int: Sg_CacheDisposeChr(v) (0x00840e60) | - | int `0x00a78300` (native_mods) |
| `0x4022` | `Native_4022` | v | Sg_SetKeepFps(v) (0x00821ae0) | - | int `0x00a78320` (native_chr) |
| `0x4023` | `Native_4023` | v | Sg_PLightAllocate(v) (0x00831950) | - | int `0x00a78340` (native_chr) |
| `0x4024` | `Native_4024` | - | 0x00831a30() | - | int `0x00a78360` (native_chr) |
| `0x4025` | `Native_4025` | n, a f, b f, c f, d f, e, f, g, i | Sg_PLightDefine(n, a, b, c, d, e, f, g, i) (0x008319b0; the original's dbgPrintf is left out) | - | int `0x00a78370` (native_chr) |
| `0x4026` | `Native_4026` | i int, mask int, x f, y f, z f | 0: Sg_PLightSetPos(i, mask, x, y, z) (0x00831ca0) | - | int `0x00a78450` (native_mods) |
| `0x4027` | `Native_4027` | i int, x int, y int, z int | 0: Sg_PLightGetPos(i, x, y, z) (0x00831b30) with the popped words as they are | - | int `0x00a784c0` (native_mods) |
| `0x4028` | `Native_4028` | i int, r int, g int, b int | 0: Sg_PLightSetColor(i, r, g, b) (0x00831c10) | - | int `0x00a78510` (native_mods) |
| `0x4029` | `Native_4029` | i int, r int, g int, b int | 0: Sg_PLightGetColor(i, r, g, b) (0x00831a60) with the popped words as they are | - | int `0x00a78560` (native_mods) |
| `0x402a` | `Native_402a` | b, a | Sg_PLightSetEnable(b, a) (0x00831c60) | - | int `0x00a785b0` (native_chr) |
| `0x402b` | `Native_402b` | i int | int: Sg_PLightGetEnable(i) (0x00831ad0) | - | int `0x00a77880` (native_mods) |
| `0x402c` | `Native_402c` | i int, r f | 0: Sg_PLightSetRadius(i, r) (0x00831ce0) | - | int `0x00a778a0` (native_mods) |
| `0x402d` | `Native_402d` | i int | float: Sg_PLightGetRadius(i) (0x00831b80) | - | float `0x00a778e0` (native_mods) |
| `0x402e` | `Native_402e` | i int, light int | 0: Sg_PLightSetLight(i, light) (0x00831c80) | - | int `0x00a77900` (native_mods) |
| `0x402f` | `Native_402f` | i int | int: Sg_PLightGetLight(i) (0x00831b00) | - | int `0x00a77930` (native_mods) |
| `0x4030` | `Native_4030` | on int | 0: Sg_SetCameraFollow(on) (0x0082dab0) | - | int `0x00a77950` (native_mods) |
| `0x4031` | `Native_4031` | - | int: Sg_GetCameraFollow_0082d850() (a jump; bit 1 of g_CameraFollow 0x00c496d5) | - | int `0x00a77970` (native_mods) |
| `0x4032` | `Native_4032` | i int, a f | 0: Sg_PLightSetAttenuation(i, a) (0x00831bf0) | - | int `0x00a77980` (native_mods) |
| `0x4033` | `Native_4033` | i int | float: Sg_PLightGetAttenuation(i) (0x00831a40) | - | float `0x00a779c0` (native_mods) |
| `0x4034` | `Native_4034` | v | Sg_SetMapAlphaFast(v) (0x00821b00); 0.0 | - | float `0x00a779e0` (native_chr) |
| `0x4035` | `Native_4035` | - | 0.0 (Sg_GetMapAlphaFast() (0x008205d0) called, its value dropped) | - | float `0x00a77a00` (native_mods) |
| `0x4036` | `Native_4036` | x f, y f, z f | Sg_GlobalSetWind(x, y, z) (0x00824e30) | - | int `0x00a77a10` (native_chr) |
| `0x4037` | `Native_4037` | x int, y int, z int | 0: Sg_GlobalGetWind(x, y, z) (0x00824dd0) with the popped words as they are | - | int `0x00a77a70` (native_mods) |
| `0x4038` | `Native_4038` | - | debug (mod-tool candidate): Sg_MemDump() (0x0083bb50: prints the memory pools and Cache_Dump), with no argument (it takes the stack's leftover); 0 | - | int `0x00a77ab0` (not ported) |
| `0x4039` | `Native_4039` | d, c, b, a | Sg_AccSetClear(d, c, b, a) (0x0082bdc0); 0.0 | - | float `0x00a77ac0` (native_chr) |
| `0x403a` | `Native_403a` | v | Sg_AccSetDrawChr(v) (0x0082be00) | - | int `0x00a77b10` (native_chr) |
| `0x403b` | `Native_FootSetMaterial` | index int, material int | 0: Sg_IdSetEffectTable(index, material) (0x0083e9b0) | - | int `0x00a77b30` (native_chr) |
| `0x403c` | `Native_403c` | v f | Sg_SetWaterLimit(v) (0x00836660) | - | int `0x00a77b60` (native_chr) |
| `0x403d` | `Native_403d` | v int | 0: Sg_MemSetFast(v) (0x0083c520) | - | int `0x00a77bc0` (native_mods) |
| `0x403e` | `Native_403e` | i, v f | Sg_PlayerSetDirFix(i, v) (0x0082d990) | - | int `0x00a77be0` (native_chr) |
| `0x403f` | `Native_403f` | - | 0x0082d930() | - | int `0x00a77c20` (native_chr) |
| `0x4040` | `Native_SetControlAngle` | degrees f | 0: 0x0082d950(degrees) | - | int `0x00a77c30` (native_chr) |
| `0x4041` | `Native_Empty_00a77c60` | - | 0 (an empty handler) | - | int `0x00a77c60` (native_mods) |
| `0x4042` | `Native_Empty_00a77c70` | - | 0 (an empty handler) | - | int `0x00a77c70` (native_mods) |
| `0x4043` | `Native_4043` | b, a | Ch_DrawSetShadowDir(b, a) (0x00830120) | - | int `0x00a77c80` (native_chr) |
| `0x4044` | `Native_4044` | - | three popped and dropped; 0 | - | int `0x00a77cb0` (native_chr) |
| `0x4045` | `Native_4045` | v | 0x0082d9d0(v) | - | int `0x00a77ce0` (native_chr) |
| `0x4046` | `Native_4046` | v | Ch_DrawSetShadowSemi(v) (0x00821b20) | - | int `0x00a77d00` (native_chr) |
| `0x4047` | `Native_ChrSetMotion_5000` | motion int | 0 | - | int `0x00a78770` (native) (as `0x5000`: group 4's table runs into group 5's) |
| `0x4048` | `Native_ChrLoadMotionGroup` | n int | int: 0, Chr_LoadMotionGroup(n) (0x008375f0) | - (the poll returns 5) | start `0x00a799b0`, poll `0x00a79a10`, int `0x00a79a20` (native_chr) (as `0x5001`: group 4's table runs into group 5's) |
| `0x4049` | `Native_5002` | seq int | 0: Ch_SetSequence(model, seq) (0x00837ac0) for a model | - | int `0x00a787a0` (native_mods) |
| `0x404a` | `Native_ChrWaitMotion_5003` | - | 5 when done | until the character's motion has ended (+0x736) | start `0x00a787d0`, poll `0x00a788a0` (native) (as `0x5003`: group 4's table runs into group 5's) |
| `0x404b` | `Native_501f_copy_00a788d0` | - | int: 1 when the model's +0x736 is set, else 0 (also without a model) | - | int `0x00a788d0` (native_mods) |
| `0x404c` | `Native_ChrReleaseMotion` | - | 0 (automatic motions take over) | - | int `0x00a78940` (native) (as `0x5005`: group 4's table runs into group 5's) |
| `0x404d` | `Native_5006` | scale f | 0: Ch_SetScaleAll(model, scale) (0x0082b590) | - | int `0x00a78a40` (native_chr) (as `0x5006`: group 4's table runs into group 5's) |
| `0x404e` | `Native_5007` | n, x f, y f, z f | Ch_SetScale(model, n, x, y, z) (0x0082b560) | - | int `0x00a789a0` (native_chr) (as `0x5007`: group 4's table runs into group 5's) |
| `0x404f` | `Native_ChrHide` | hide int (0, 1) | 0 | - | int `0x00a78ac0` (native) (as `0x5008`: group 4's table runs into group 5's) |
| `0x4050` | `Native_ChrSetPushWeight` | weight f (push weight) | float: previous weight | - | float `0x00a78b30` (native) (as `0x5009`: group 4's table runs into group 5's) |
| `0x4051` | `Native_500a` | height f | 0: Ch_SetHitHeight(model, height) (0x0082b320) for a model | - | int `0x00a78c20` (native_mods) |
| `0x4052` | `Native_500b` | v f | 0: Ch_SetJimen(model, v) (0x0082b360) | - | int `0x00a78b90` (native_chr) (as `0x500b`: group 4's table runs into group 5's) |
| `0x4053` | `Native_500c` | v | popped and dropped (the original only prints a warning for a model); 0 | - | int `0x00a78d30` (native_chr) (as `0x500c`: group 4's table runs into group 5's) |
| `0x4054` | `Native_500d` | v int | 0: Ch_SetHideEvent(model, v) (0x0082b2c0) | - | int `0x00a78df0` (native_chr) (as `0x500d`: group 4's table runs into group 5's) |
| `0x4055` | `Native_ChrSetGroundFollow` | on int: stick to the ground | 0 | - | int `0x00a78cb0` (native) (as `0x500e`: group 4's table runs into group 5's) |
| `0x4056` | `Native_ChrSetBodyHit` | on int: collide with characters | 0 | - | int `0x00a78da0` (native) (as `0x500f`: group 4's table runs into group 5's) |
| `0x4057` | `Native_ChrLoadSystemMgrp` | chr id int, n int | start: the args words; int: 0, ChN_ReadSystemMGRP(id, n) (0x00836870) | - (the poll returns 5) | start `0x00a79820`, poll `0x00a79970`, int `0x00a79980` (native_chr) (as `0x5010`: group 4's table runs into group 5's) |
| `0x4058` | `Native_5011` | v int | 0: ChN_DisposeSystemMGRP(the model's u16 +0, v) (0x00836830) for a model | - | int `0x00a78e50` (native_mods) |
| `0x405a` | `Native_5013` | mode int | 0: Ch_SetFieldMode, Ch_SetMotionType(model, mode) (0x0082b1e0, 0x0082b3a0) | - | int `0x00a78ec0` (native_chr) (as `0x5013`: group 4's table runs into group 5's) |
| `0x405b` | `Native_5014` | v f, n | Ch_SetShade(model, v, n) (0x0082b5b0) | - | int `0x00a78f30` (native_chr) (as `0x5014`: group 4's table runs into group 5's) |
| `0x405c` | `Native_5015` | r f, g f, b f, n int | 0: Ch_SetShadeCol(model, r, g, b, n) (0x0082b5d0) | - | int `0x00a78fb0` (native_chr) (as `0x5015`: group 4's table runs into group 5's) |
| `0x405d` | `Native_ChrSetOffset` | x f, y f, z f: model offset | 0 | - | int `0x00a790b0` (native) (as `0x5016`: group 4's table runs into group 5's) |
| `0x405e` | `Native_ChrSetGroundMode` | mode int (0 clamp, 1 snap, 2 float) | 0 | - | int `0x00a79130` (native) (as `0x5017`: group 4's table runs into group 5's) |
| `0x405f` | `Native_5018` | v int | 0: Ch_SetMotionSpeed(model, v) (0x0082b380) | - | int `0x00a791e0` (native_chr) (as `0x5018`: group 4's table runs into group 5's) |
| `0x4060` | `Native_5019` | - | Native_PlayMotionCommon(worker, stack, 0, 0) | - | int `0x00a79220` (native_chr) (as `0x5019`: group 4's table runs into group 5's) |
| `0x4061` | `Native_ChrPlayMotion` | motion args as Native_PlayMotionCommon (stack) | 0 | - | int `0x00a79270` (native_move) (as `0x501a`: group 4's table runs into group 5's) |
| `0x4062` | `Native_ChrSetMotion_501b` | motion int | 0 | - | int `0x00a792d0` (native) (as `0x501b`: group 4's table runs into group 5's) |
| `0x4064` | `Native_501d` | seq int | 0: Ch_SetSequence(model, seq) (0x00837ac0) for a model (as 0x5002) | - | int `0x00a79310` (native_mods) |
| `0x4065` | `Native_ChrWaitMotion_501e` | - | 5 when done | until the scripted motion has ended | start `0x00a79370`, poll `0x00a793c0` (native) (as `0x501e`: group 4's table runs into group 5's) |
| `0x4066` | `Native_501f` | - | 1 while the worker's model has +0x736 set | - | int `0x00a79450` (native_chr) (as `0x501f`: group 4's table runs into group 5's) |
| `0x4067` | `Native_ChrReleaseMotion2` | - | 0 (as 0x5005) | - | int `0x00a794b0` (native) (as `0x5020`: group 4's table runs into group 5's) |
| `0x4068` | `Native_ChrSetMotionSpeed` | speed int (8.8, 256 = 1x) | 0 | - | int `0x00a79510` (native) (as `0x5021`: group 4's table runs into group 5's) |
| `0x4069` | `Native_ChrSetMotionSeqLevel` | level int | 0 | - | int `0x00a79550` (native_move) (as `0x5022`: group 4's table runs into group 5's) |
| `0x406a` | `Native_5023` | v int | 0 (the model asked for and v popped, both unused) | - | int `0x00a795b0` (native_mods) |
| `0x406b` | `Native_5024` | v int | 0 (as 0x5023) | - | int `0x00a79620` (native_mods) |
| `0x406c` | `Native_ChrSetAutoMotion` | slot int, motion int (-1 default) | 0 | - | int `0x00a79670` (native) (as `0x5025`: group 4's table runs into group 5's) |
| `0x406d` | `Native_5026` | worker, point, &x, &y, &z | Ch_ElemGetPos(model, point, 0, &v) (0x00833d10) into the three variables (null ones skipped); 0 | - | int `0x00a796e0` (native_chr) (as `0x5026`: group 4's table runs into group 5's) |
| `0x406e` | `Native_5027` | i int, element int, x f, y f, z f, ox var, oy var, oz var | 0: Ch_ElemGetPos(worker i's model, element, &(x, y, z, 1.0), &out) (0x00833d10) and out's three words into the variables (null ones skipped; nothing without a model) | - | int `0x00a79860` (native_mods) |
| `0x406f` | `Native_5028` | v f | Ch_MoveSetChgRun(model, v) (0x0082b040), 0.0 without a model | - | float `0x00a799d0` (native_chr) (as `0x5028`: group 4's table runs into group 5's) |
| `0x4070` | `Native_ChrSetShadowMode` | mode int | int: Ch_SetShadowMode's result | - | int `0x00a79a40` (native) (as `0x5029`: group 4's table runs into group 5's) |
| `0x4071` | `Native_ChrSetDistanceCull` | on int | int: previous | - | int `0x00a79af0` (native) (as `0x502a`: group 4's table runs into group 5's) |
| `0x4072` | `Native_502b` | - | 0 (the model asked for, unused) | - | int `0x00a79b70` (native_mods) |
| `0x4073` | `Native_502c` | worker, point | as Native_ChrAttach with Ch_BindToElemDirect (0x00832680) | - | int `0x00a79bc0` (native_chr) (as `0x502c`: group 4's table runs into group 5's) |
| `0x4074` | `Native_ChrAttach` | worker int (-1 detaches), point int | 0: Ch_BindToElem(model, that worker's model, point) (0x00832630) | - | int `0x00a79c50` (native_chr) (as `0x502d`: group 4's table runs into group 5's) |
| `0x4075` | `Native_502e` | - | int: Ch_IdGetMap(model) (0x0083d840), 0 without a model | - | int `0x00a79d10` (native_mods) |
| `0x4076` | `Native_502f` | i int | int: Ch_IdGetMap(worker i's model), 0 without a model | - | int `0x00a79d60` (native_mods) |
| `0x4077` | `Native_5030` | - | int: Ch_IdGetSound(model) (0x0083d900), 0 without a model | - | int `0x00a79db0` (native_mods) |
| `0x4078` | `Native_5031` | i int | int: Ch_IdGetSound(worker i's model), 0 without a model | - | int `0x00a79e00` (native_mods) |
| `0x4079` | `Native_ChrPlayDefaultMotion` | - | 0 (default motion) | - | int `0x00a79e60` (native_move) (as `0x5032`: group 4's table runs into group 5's) |
| `0x407a` | `Native_ChrFadeAlpha` | alpha f, frames int (0 at once) | 0 | - | int `0x00a79ea0` (native) (as `0x5033`: group 4's table runs into group 5's) |
| `0x407b` | `Native_ChrSetMotionBlend` | frames int: blend into the next motion | 0 | - | int `0x00a79f40` (native) (as `0x5034`: group 4's table runs into group 5's) |
| `0x407c` | `Native_5035` | - | Ch_GetSeqFrame(model) (0x00836af0), 0 without a model | - | int `0x00a79fa0` (native_chr) (as `0x5035`: group 4's table runs into group 5's) |
| `0x407d` | `Native_5036` | - | 0x00a7a980(worker, args, stack, 0) | - | int `0x00a7a120` (native_chr) (as `0x5036`: group 4's table runs into group 5's) |
| `0x407e` | `Native_5037` | worker int, x var, y var, z var | 0: the model's point 0 (Ch_OptposGet 0x008354f0) into the variables (null ones skipped) | - | int `0x00a7a140` (native_chr) (as `0x5037`: group 4's table runs into group 5's) |
| `0x407f` | `Native_5038` | i int, x var, y var, z var | 0: worker i's model's +0x6b4, +0x6b8, +0x6bc into the variables (null ones skipped) | - | int `0x00a7a250` (native_mods) |
| `0x4080` | `Native_ChrSetHitRadiusScale` | scale f | float: Ch_SetThickness(model, scale) (0x0082b880); 0.0 without a model | - | float `0x00a7a340` (native_chr) (as `0x5039`: group 4's table runs into group 5's) |
| `0x4081` | `Native_503a` | z f | float: Ch_SetClipZ(model, z) (0x0082b150); 0.0 without a model | - | float `0x00a7a3d0` (native_chr) (as `0x503a`: group 4's table runs into group 5's) |
| `0x4082` | `Native_503b` | b, a | ChN_DisposeSystemMGRP(b, a) (0x00836830) when the worker has a model | - | int `0x00a7a450` (native_chr) (as `0x503b`: group 4's table runs into group 5's) |
| `0x4083` | `Native_503c` | v int | 0: Ch_DisposeMGRP(v) (0x00836ad0) | - | int `0x00a7a4d0` (native_chr) (as `0x503c`: group 4's table runs into group 5's) |
| `0x4084` | `Native_503d` | v int | 0: ChN_DataDispose(v, 1) (0x00824f00) | - | int `0x00a7a520` (native_chr) (as `0x503d`: group 4's table runs into group 5's) |
| `0x4085` | `Native_503e` | b int, a int | 0: ChN_DataDispose(b, a) (0x00824f00) | - | int `0x00a7a570` (native_chr) (as `0x503e`: group 4's table runs into group 5's) |
| `0x4086` | `Native_503f` | v | Ch_SetSequence(model, v) (0x00837ac0) | - | int `0x00a7a5a0` (native_chr) (as `0x503f`: group 4's table runs into group 5's) |
| `0x4087` | `Native_ChrSetMotion2` | motion int | 0 | - | int `0x00a7a610` (native_move) (as `0x5040`: group 4's table runs into group 5's) |
| `0x4088` | `Native_5019_copy_00a7a740` | as Native_PlayMotionCommon | 0: Native_PlayMotionCommon(worker, stack, 0, 0) | - | int `0x00a7a740` (native_mods) |
| `0x4089` | `Native_ChrPlayMotion2` | as 0x501a | 0 | - | int `0x00a7a780` (native_move) (as `0x5042`: group 4's table runs into group 5's) |
| `0x408a` | `Native_5043` | as Native_PlayMotionCommon | 0: Native_PlayMotionCommon(worker, stack, 0, 1) | - | int `0x00a7a7c0` (native_mods) |
| `0x408b` | `Native_5044` | as Native_PlayMotionCommon | 0: Native_PlayMotionCommon(worker, stack, 1, 1) (0x00a7aae0) | - | int `0x00a7a810` (native_chr) (as `0x5044`: group 4's table runs into group 5's) |
| `0x408c` | `Native_5045` | as Native_PlayMotionCommon | 0: Native_PlayMotionCommon(worker, stack, 0, 1) (as 0x5043) | - | int `0x00a7a830` (native_mods) |
| `0x408d` | `Native_5046` | - | Native_PlayMotionCommon(worker, stack, 1, 1) | - | int `0x00a7a880` (native_chr) (as `0x5046`: group 4's table runs into group 5's) |
| `0x408e` | `Native_5047` | worker int, v int | 0: Ch_SetSequence(that worker's model, v) (0x00837ac0) | - | int `0x00a7a900` (native_chr) (as `0x5047`: group 4's table runs into group 5's) |
| `0x408f` | `Native_5048` | - | as 0x5047 | - | int `0x00a7a940` (native_chr) (as `0x5048`: group 4's table runs into group 5's) |
| `0x4090` | `Native_5049` | worker | start: the args word = that worker's model, also EAX; poll: 0 while the model in the args word has +0x736 set (or there is none), else 5 | see the poll | start `0x00a785f0`, poll `0x00a78620` (native_chr) (as `0x5049`: group 4's table runs into group 5's) |
| `0x4091` | `Native_504a` | worker | as 0x501f for that worker | - | int `0x00a78640` (native_chr) (as `0x504a`: group 4's table runs into group 5's) |
| `0x4092` | `Native_504b` | worker | Ch_ResetMotion (0x0082b080) on that worker's model | - | int `0x00a78670` (native_chr) (as `0x504b`: group 4's table runs into group 5's) |
| `0x4093` | `Native_504c` | worker int, v int | 0: Ch_SetMotionSpeed(that worker's model, v) (0x0082b380) | - | int `0x00a786a0` (native_chr) (as `0x504c`: group 4's table runs into group 5's) |
| `0x4094` | `Native_504d` | v f | Ch_AllSetThicknessEvent(v) (0x0082a8d0); 0.0 | - | float `0x00a786e0` (native_chr) (as `0x504d`: group 4's table runs into group 5's) |
| `0x4095` | `Native_ChrLoad` | chr int | int: 0x00829ef0's result | - | int `0x00a79800` (native_move) (as `0x504e`: group 4's table runs into group 5's) |
| `0x4096` | `Native_504f` | slot int | int: args +0xc, Ch_DataReadSync(slot)'s result (0x0082a040) | until Ch_DataReadSync(slot) is 0 (poll 1), or 5 with args +8 set | start `0x00a79340`, poll `0x00a79380`, int `0x00a79410` (native_chr) (as `0x504f`: group 4's table runs into group 5's) |
| `0x4097` | `Native_5050` | worker int, v int | 0: Ch_MotionSet737(that worker's model, v) (0x00837570) | - | int `0x00a78710` (native_chr) (as `0x5050`: group 4's table runs into group 5's) |
| `0x4098` | `Native_ChrGetMotionFrame` | - | int: motion frame (8.8) | - | int `0x00a78750` (native) (as `0x5051`: group 4's table runs into group 5's) |
| `0x4099` | `Native_5052` | worker, point, x f, y f, z f | Ch_BindToElemDirectOfs(model, that worker's model, point, x, y, z) (0x008326e0); worker -1 detaches with the offset | - | int `0x00a787e0` (native_chr) (as `0x5052`: group 4's table runs into group 5's) |
| `0x409a` | `Native_5053` | i int, id int | int: Ch_ElemGetDirect(worker i's model, id) (0x00833bf0), 0 without a model | - | int `0x00a78900` (native_mods) |
| `0x409b` | `Native_5054` | part int, on int | int: Ch_ModelSetHide(model, part, on) (0x0082aea0); 0 without a model | - | int `0x00a78960` (native_chr) (as `0x5054`: group 4's table runs into group 5's) |
| `0x409c` | `Native_5055` | v int | int: Ch_ModelGetHide(model, v) (0x0082add0), 0 without a model | - | int `0x00a78a10` (native_mods) |
| `0x409d` | `Native_5056` | v int | int: Ch_SetCalcNorm(model, v) (0x00835750); 0 without a model | - | int `0x00a78a80` (native_chr) (as `0x5056`: group 4's table runs into group 5's) |
| `0x409e` | `Native_ChrSetBoneUpdate` | worker int, on int | int: previous | - | int `0x00a78af0` (native) (as `0x5057`: group 4's table runs into group 5's) |
| `0x409f` | `Native_5058` | v int | 0: Ch_AllSetCalcNorm(v) (0x008325b0) | - | int `0x00a78b70` (native_chr) (as `0x5058`: group 4's table runs into group 5's) |
| `0x40a0` | `Native_5059` | slot int, motion int | 0: Ch_SetSysMotion(model, slot, motion) (0x00837d00) for a model | - | int `0x00a78bd0` (native_mods) |
| `0x40a1` | `Native_505a` | b, a | Ch_TextureSetClutBase(model, b, a) (0x008313c0), 0 without a model | - | int `0x00a78c60` (native_chr) (as `0x505a`: group 4's table runs into group 5's) |
| `0x40a2` | `Native_505b` | b int, a int | int: Ch_TextureSetImageBase(model, b, a) (0x00831390), 0 without a model | - | int `0x00a78ce0` (native_mods) |
| `0x40a3` | `Native_ChrSetNextMotionBlend` | frames int (-1 = 0, 0 = 1) | 0 | - | int `0x00a78d70` (native) (as `0x505c`: group 4's table runs into group 5's) |
| `0x40a4` | `Native_ChrGetMotion` | - | int: motion id | - | int `0x00a78dd0` (native) (as `0x505d`: group 4's table runs into group 5's) |
| `0x40a5` | `Native_505e` | - | Ch_GetSp(model) (0x0082ad60), 0.0 without a model | - | float `0x00a78e20` (native_chr) (as `0x505e`: group 4's table runs into group 5's) |
| `0x40a6` | `Native_505f` | v int | 0: Ch_MotionSetLoop(model, v) (0x00839960) | - | int `0x00a78e90` (native_chr) (as `0x505f`: group 4's table runs into group 5's) |
| `0x40a7` | `Native_5060` | v | Ch_IdEnableShade(model, v) (0x008340f0) | - | int `0x00a78f00` (native_chr) (as `0x5060`: group 4's table runs into group 5's) |
| `0x40a8` | `Native_5061` | b int, a int | 0: Ch_MgrpRead(b, a) (0x00836a40) | - | int `0x00a78f80` (native_chr) (as `0x5061`: group 4's table runs into group 5's) |
| `0x40a9` | `Native_5062` | id int, n int, keep int | the id; the poll 5 when done; int slot the args word +0xc (with keep, ChN_ReadSystemMGRPsync's result) | with keep set one call of ChN_ReadSystemMGRPsync(id, n) (0x00836a50), else until it returns 0 | start `0x00a79030`, poll `0x00a79070`, int `0x00a79120` (native_mods) |
| `0x40aa` | `Native_5063` | id int, n int | int: args +0xc, ChN_ReadSystemMGRPsync(id, n)'s result (0x00836a50) | as 0x504f | start `0x00a79160`, poll `0x00a791a0`, int `0x00a79210` (native_chr) (as `0x5063`: group 4's table runs into group 5's) |
| `0x40ab` | `Native_5064` | id int, keep int | the id; the poll 5 (with keep) or 1 when done; int slot the args word +0xc | with keep set one call of Ch_DataReadSync(id) (0x0082a040), else until it returns 0 | start `0x00a79240`, poll `0x00a79290`, int `0x00a79300` (native_mods) |
| `0x40ac` | `Native_5065` | v int | 0: 0x00837650(v) (the dbgPrintf left out) | - | int `0x00a79420` (native_chr) (as `0x5065`: group 4's table runs into group 5's) |
| `0x40ad` | `Native_5066` | n | start: args +0 = n, +0xc = 0; EAX n; poll: 1 once 0x00837670(n) is 0 (the dbgPrintf left out), else 0; int: args +0xc | see the poll | start `0x00a79480`, poll `0x00a794d0`, int `0x00a79540` (native_chr) (as `0x5066`: group 4's table runs into group 5's) |
| `0x40ae` | `Native_5067` | v | Ch_DrawSetCull(model, v) (0x008300b0) | - | int `0x00a79580` (native_chr) (as `0x5067`: group 4's table runs into group 5's) |
| `0x40af` | `Native_WaitCdIdle` | - | 1 when done | until the CD reads are idle | poll `0x00a795d0` (native) (as `0x5068`: group 4's table runs into group 5's) |
| `0x40b0` | `Native_5069` | v | Ch_SetKeepFps(model, v) (0x00839b50), 0 without a model | - | int `0x00a795e0` (native_chr) (as `0x5069`: group 4's table runs into group 5's) |
| `0x40b1` | `Native_506a` | v | Ch_PLightSetEnable(model, v) (0x00830ae0) | - | int `0x00a79640` (native_chr) (as `0x506a`: group 4's table runs into group 5's) |
| `0x40b2` | `Native_506b` | - | int: Ch_PLightGetEnable(model) (0x00830ad0), 0 without a model | - | int `0x00a796c0` (native_mods) |
| `0x40b3` | `Native_506c` | bits int, mask int | 0: Ch_PLightSetEnableMask(model, bits, mask) (0x00830b00) for a model | - | int `0x00a797b0` (native_mods) |
| `0x40b4` | `Native_506d` | a f, b f | Ch_SetRotX(model, wrap(b degrees), a) (0x008357f0) | - | int `0x00a79a80` (native_chr) (as `0x506d`: group 4's table runs into group 5's) |
| `0x40b5` | `Native_506e` | v f | Ch_BindSetScale(model, v) (0x00832610) | - | int `0x00a79b30` (native_chr) (as `0x506e`: group 4's table runs into group 5's) |
| `0x40b6` | `Native_506f` | v int (popped only for a model) | 0: Ch_SetHideCalc(model, v) (0x0082b280) | - | int `0x00a79b90` (native_chr) (as `0x506f`: group 4's table runs into group 5's) |
| `0x40b7` | `Native_5070` | - | int: Ch_GetHideCalc(model) (0x0082abe0), 0 without a model | - | int `0x00a79c30` (native_mods) |
| `0x40b8` | `Native_5071` | v | Ch_TextureLoadImage(model, v) (0x00831300) | - | int `0x00a79cc0` (native_chr) (as `0x5071`: group 4's table runs into group 5's) |
| `0x40b9` | `Native_5072` | - | Ch_IdGetPNum(model) (0x0082adc0), 0 without a model | - | int `0x00a79cf0` (native_chr) (as `0x5072`: group 4's table runs into group 5's) |
| `0x40ba` | `Native_5073` | v | Ch_SetOverlapHit(model, v) (0x0082b420), popped only for a model | - | int `0x00a79d30` (native_chr) (as `0x5073`: group 4's table runs into group 5's) |
| `0x40bb` | `Native_5074` | - | int: Ch_GetOverlapHit(model) (0x0082ac70), 0 without a model | - | int `0x00a79d90` (native_mods) |
| `0x40bc` | `Native_ChrGetMotionFrameOf` | worker int | int: its motion frame | - | int `0x00a79dd0` (native) (as `0x5075`: group 4's table runs into group 5's) |
| `0x40bd` | `Native_5076` | on int (popped only for a model) | 0: Ch_MouthSetAutoTalk(model, on) (0x00834770) | - | int `0x00a79e30` (native_chr) (as `0x5076`: group 4's table runs into group 5's) |
| `0x40be` | `Native_5077` | - | int: Ch_MouthGetAutoTalk(model) (0x00834740), 0 without a model | - | int `0x00a79e80` (native_mods) |
| `0x40bf` | `Native_5078` | b int, a int | 0: Ch_TextureSetAnim(model, b, a) (0x0083d040) | - | int `0x00a79ef0` (native_chr) (as `0x5078`: group 4's table runs into group 5's) |
| `0x40c0` | `Native_5079` | on int | 0: Ch_NeckSetEnable(model, on) (0x00834f60) | - | int `0x00a79f70` (native_chr) (as `0x5079`: group 4's table runs into group 5's) |
| `0x40c1` | `Native_507a` | worker int | 0: Ch_NeckSetTarget(model, that worker's model) (0x00835050) | - | int `0x00a79fc0` (native_chr) (as `0x507a`: group 4's table runs into group 5's) |
| `0x40c2` | `Native_507b` | x f, y f, z f | Ch_NeckSetTargetPos(model, &[x, y, z]) (0x008350a0) | - | int `0x00a7a000` (native_chr) (as `0x507b`: group 4's table runs into group 5's) |
| `0x40c3` | `Native_507c` | - | Ch_NeckSetTargetCamera(model) (0x00835080) | - | int `0x00a7a070` (native_chr) (as `0x507c`: group 4's table runs into group 5's) |
| `0x40c4` | `Native_507d` | b f, a f | degrees: Ch_NeckSetRot(model, wrap(b), wrap(a)) (0x00834ff0) | - | int `0x00a7a090` (native_chr) (as `0x507d`: group 4's table runs into group 5's) |
| `0x40c5` | `CT_507E_ret` | worker int, on int: mouth flapping | 0 | - | int `0x00a7a210` (native) (as `0x507e`: group 4's table runs into group 5's) |
| `0x40c6` | `Native_507f` | i int, v f | 0: 0x0082ae30(model, i, v) for a model | - | int `0x00a7a380` (native_mods) |
| `0x40c7` | `Native_5080` | v f | Ch_SetHiju(model, v) (0x0082b340) | - | int `0x00a7a410` (native_chr) (as `0x5080`: group 4's table runs into group 5's) |
| `0x40c8` | `Native_5081` | v | Ch_SetChocobo(model, v) (0x0082b130), popped only for a model | - | int `0x00a7a4a0` (native_chr) (as `0x5081`: group 4's table runs into group 5's) |
| `0x40c9` | `Native_5082` | v | Ch_ClothSetEnable(model, v) (0x0082a930), popped only for a model | - | int `0x00a7a4f0` (native_chr) (as `0x5082`: group 4's table runs into group 5's) |
| `0x40ca` | `Native_5083` | v | Ch_Vu1SetClipmode(model, v) (0x0082b970), popped only for a model | - | int `0x00a7a540` (native_chr) (as `0x5083`: group 4's table runs into group 5's) |
| `0x40cb` | `Native_5084` | v f | Ch_NeckSetSpeed(model, v) (0x00835020) | - | int `0x00a7a5d0` (native_chr) (as `0x5084`: group 4's table runs into group 5's) |
| `0x40cc` | `Native_5085` | v f | Ch_ShadowSetHeight(model, v) (0x00830cb0) | - | int `0x00a7a640` (native_chr) (as `0x5085`: group 4's table runs into group 5's) |
| `0x40cd` | `Native_5086` | - | Ch_ReserveCalcNorm(model) (0x00835710) | - | int `0x00a7a680` (native_chr) (as `0x5086`: group 4's table runs into group 5's) |
| `0x40ce` | `Native_ChrRefreshAllBones` | - | 0 | - | int `0x00a7a6a0` (native) (as `0x5087`: group 4's table runs into group 5's) |
| `0x40cf` | `Native_5088` | v f, n | Ch_SetShadeID(model, v, n) (0x0082b720) | - | int `0x00a7a6b0` (native_chr) (as `0x5088`: group 4's table runs into group 5's) |
| `0x40d0` | `Native_5089` | - | Native_PlayMotionCommon(worker, stack, 2, 0) | - | int `0x00a7a700` (native_chr) (as `0x5089`: group 4's table runs into group 5's) |
| `0x40d1` | `Native_508a` | as Native_PlayMotionCommon | 0: Native_PlayMotionCommon(worker, stack, 2, 1) | - | int `0x00a7a720` (native_mods) |
| `0x40d2` | `Native_508b` | - | the same as 0x5089 | - | int `0x00a7a760` (native_chr) (as `0x508b`: group 4's table runs into group 5's) |
| `0x40d3` | `Native_508c` | as Native_PlayMotionCommon | 0: Native_PlayMotionCommon(worker, stack, 2, 1) (as 0x508a) | - | int `0x00a7a7a0` (native_mods) |
| `0x40d4` | `Native_508d` | v | Ch_TextureSetAnimTimer(model, v) (0x0083d070) | - | int `0x00a7a7e0` (native_chr) (as `0x508d`: group 4's table runs into group 5's) |
| `0x40d5` | `Native_508e` | v | Ch_ClothEnableGrav(model, v) (0x008245f0) | - | int `0x00a7a850` (native_chr) (as `0x508e`: group 4's table runs into group 5's) |
| `0x40d6` | `Native_508f` | v int (popped only for a model) | 0: the model's +0x870 byte = v | - | int `0x00a7a8a0` (native_chr) (as `0x508f`: group 4's table runs into group 5's) |
| `0x40d7` | `Native_5090` | v | the model's +0x871 byte, popped only for a model | - | int `0x00a7a8d0` (native_chr) (as `0x5090`: group 4's table runs into group 5's) |
| `0x5000` | `Native_ChrSetMotion_5000` | motion int | 0 | - | int `0x00a78770` (native) |
| `0x5001` | `Native_ChrLoadMotionGroup` | n int | int: 0, Chr_LoadMotionGroup(n) (0x008375f0) | - (the poll returns 5) | start `0x00a799b0`, poll `0x00a79a10`, int `0x00a79a20` (native_chr) |
| `0x5002` | `Native_5002` | seq int | 0: Ch_SetSequence(model, seq) (0x00837ac0) for a model | - | int `0x00a787a0` (native_mods) |
| `0x5003` | `Native_ChrWaitMotion_5003` | - | 5 when done | until the character's motion has ended (+0x736) | start `0x00a787d0`, poll `0x00a788a0` (native) |
| `0x5004` | `Native_501f_copy_00a788d0` | - | int: 1 when the model's +0x736 is set, else 0 (also without a model) | - | int `0x00a788d0` (native_mods) |
| `0x5005` | `Native_ChrReleaseMotion` | - | 0 (automatic motions take over) | - | int `0x00a78940` (native) |
| `0x5006` | `Native_5006` | scale f | 0: Ch_SetScaleAll(model, scale) (0x0082b590) | - | int `0x00a78a40` (native_chr) |
| `0x5007` | `Native_5007` | n, x f, y f, z f | Ch_SetScale(model, n, x, y, z) (0x0082b560) | - | int `0x00a789a0` (native_chr) |
| `0x5008` | `Native_ChrHide` | hide int (0, 1) | 0 | - | int `0x00a78ac0` (native) |
| `0x5009` | `Native_ChrSetPushWeight` | weight f (push weight) | float: previous weight | - | float `0x00a78b30` (native) |
| `0x500a` | `Native_500a` | height f | 0: Ch_SetHitHeight(model, height) (0x0082b320) for a model | - | int `0x00a78c20` (native_mods) |
| `0x500b` | `Native_500b` | v f | 0: Ch_SetJimen(model, v) (0x0082b360) | - | int `0x00a78b90` (native_chr) |
| `0x500c` | `Native_500c` | v | popped and dropped (the original only prints a warning for a model); 0 | - | int `0x00a78d30` (native_chr) |
| `0x500d` | `Native_500d` | v int | 0: Ch_SetHideEvent(model, v) (0x0082b2c0) | - | int `0x00a78df0` (native_chr) |
| `0x500e` | `Native_ChrSetGroundFollow` | on int: stick to the ground | 0 | - | int `0x00a78cb0` (native) |
| `0x500f` | `Native_ChrSetBodyHit` | on int: collide with characters | 0 | - | int `0x00a78da0` (native) |
| `0x5010` | `Native_ChrLoadSystemMgrp` | chr id int, n int | start: the args words; int: 0, ChN_ReadSystemMGRP(id, n) (0x00836870) | - (the poll returns 5) | start `0x00a79820`, poll `0x00a79970`, int `0x00a79980` (native_chr) |
| `0x5011` | `Native_5011` | v int | 0: ChN_DisposeSystemMGRP(the model's u16 +0, v) (0x00836830) for a model | - | int `0x00a78e50` (native_mods) |
| `0x5013` | `Native_5013` | mode int | 0: Ch_SetFieldMode, Ch_SetMotionType(model, mode) (0x0082b1e0, 0x0082b3a0) | - | int `0x00a78ec0` (native_chr) |
| `0x5014` | `Native_5014` | v f, n | Ch_SetShade(model, v, n) (0x0082b5b0) | - | int `0x00a78f30` (native_chr) |
| `0x5015` | `Native_5015` | r f, g f, b f, n int | 0: Ch_SetShadeCol(model, r, g, b, n) (0x0082b5d0) | - | int `0x00a78fb0` (native_chr) |
| `0x5016` | `Native_ChrSetOffset` | x f, y f, z f: model offset | 0 | - | int `0x00a790b0` (native) |
| `0x5017` | `Native_ChrSetGroundMode` | mode int (0 clamp, 1 snap, 2 float) | 0 | - | int `0x00a79130` (native) |
| `0x5018` | `Native_5018` | v int | 0: Ch_SetMotionSpeed(model, v) (0x0082b380) | - | int `0x00a791e0` (native_chr) |
| `0x5019` | `Native_5019` | - | Native_PlayMotionCommon(worker, stack, 0, 0) | - | int `0x00a79220` (native_chr) |
| `0x501a` | `Native_ChrPlayMotion` | motion args as Native_PlayMotionCommon (stack) | 0 | - | int `0x00a79270` (native_move) |
| `0x501b` | `Native_ChrSetMotion_501b` | motion int | 0 | - | int `0x00a792d0` (native) |
| `0x501d` | `Native_501d` | seq int | 0: Ch_SetSequence(model, seq) (0x00837ac0) for a model (as 0x5002) | - | int `0x00a79310` (native_mods) |
| `0x501e` | `Native_ChrWaitMotion_501e` | - | 5 when done | until the scripted motion has ended | start `0x00a79370`, poll `0x00a793c0` (native) |
| `0x501f` | `Native_501f` | - | 1 while the worker's model has +0x736 set | - | int `0x00a79450` (native_chr) |
| `0x5020` | `Native_ChrReleaseMotion2` | - | 0 (as 0x5005) | - | int `0x00a794b0` (native) |
| `0x5021` | `Native_ChrSetMotionSpeed` | speed int (8.8, 256 = 1x) | 0 | - | int `0x00a79510` (native) |
| `0x5022` | `Native_ChrSetMotionSeqLevel` | level int | 0 | - | int `0x00a79550` (native_move) |
| `0x5023` | `Native_5023` | v int | 0 (the model asked for and v popped, both unused) | - | int `0x00a795b0` (native_mods) |
| `0x5024` | `Native_5024` | v int | 0 (as 0x5023) | - | int `0x00a79620` (native_mods) |
| `0x5025` | `Native_ChrSetAutoMotion` | slot int, motion int (-1 default) | 0 | - | int `0x00a79670` (native) |
| `0x5026` | `Native_5026` | worker, point, &x, &y, &z | Ch_ElemGetPos(model, point, 0, &v) (0x00833d10) into the three variables (null ones skipped); 0 | - | int `0x00a796e0` (native_chr) |
| `0x5027` | `Native_5027` | i int, element int, x f, y f, z f, ox var, oy var, oz var | 0: Ch_ElemGetPos(worker i's model, element, &(x, y, z, 1.0), &out) (0x00833d10) and out's three words into the variables (null ones skipped; nothing without a model) | - | int `0x00a79860` (native_mods) |
| `0x5028` | `Native_5028` | v f | Ch_MoveSetChgRun(model, v) (0x0082b040), 0.0 without a model | - | float `0x00a799d0` (native_chr) |
| `0x5029` | `Native_ChrSetShadowMode` | mode int | int: Ch_SetShadowMode's result | - | int `0x00a79a40` (native) |
| `0x502a` | `Native_ChrSetDistanceCull` | on int | int: previous | - | int `0x00a79af0` (native) |
| `0x502b` | `Native_502b` | - | 0 (the model asked for, unused) | - | int `0x00a79b70` (native_mods) |
| `0x502c` | `Native_502c` | worker, point | as Native_ChrAttach with Ch_BindToElemDirect (0x00832680) | - | int `0x00a79bc0` (native_chr) |
| `0x502d` | `Native_ChrAttach` | worker int (-1 detaches), point int | 0: Ch_BindToElem(model, that worker's model, point) (0x00832630) | - | int `0x00a79c50` (native_chr) |
| `0x502e` | `Native_502e` | - | int: Ch_IdGetMap(model) (0x0083d840), 0 without a model | - | int `0x00a79d10` (native_mods) |
| `0x502f` | `Native_502f` | i int | int: Ch_IdGetMap(worker i's model), 0 without a model | - | int `0x00a79d60` (native_mods) |
| `0x5030` | `Native_5030` | - | int: Ch_IdGetSound(model) (0x0083d900), 0 without a model | - | int `0x00a79db0` (native_mods) |
| `0x5031` | `Native_5031` | i int | int: Ch_IdGetSound(worker i's model), 0 without a model | - | int `0x00a79e00` (native_mods) |
| `0x5032` | `Native_ChrPlayDefaultMotion` | - | 0 (default motion) | - | int `0x00a79e60` (native_move) |
| `0x5033` | `Native_ChrFadeAlpha` | alpha f, frames int (0 at once) | 0 | - | int `0x00a79ea0` (native) |
| `0x5034` | `Native_ChrSetMotionBlend` | frames int: blend into the next motion | 0 | - | int `0x00a79f40` (native) |
| `0x5035` | `Native_5035` | - | Ch_GetSeqFrame(model) (0x00836af0), 0 without a model | - | int `0x00a79fa0` (native_chr) |
| `0x5036` | `Native_5036` | - | 0x00a7a980(worker, args, stack, 0) | - | int `0x00a7a120` (native_chr) |
| `0x5037` | `Native_5037` | worker int, x var, y var, z var | 0: the model's point 0 (Ch_OptposGet 0x008354f0) into the variables (null ones skipped) | - | int `0x00a7a140` (native_chr) |
| `0x5038` | `Native_5038` | i int, x var, y var, z var | 0: worker i's model's +0x6b4, +0x6b8, +0x6bc into the variables (null ones skipped) | - | int `0x00a7a250` (native_mods) |
| `0x5039` | `Native_ChrSetHitRadiusScale` | scale f | float: Ch_SetThickness(model, scale) (0x0082b880); 0.0 without a model | - | float `0x00a7a340` (native_chr) |
| `0x503a` | `Native_503a` | z f | float: Ch_SetClipZ(model, z) (0x0082b150); 0.0 without a model | - | float `0x00a7a3d0` (native_chr) |
| `0x503b` | `Native_503b` | b, a | ChN_DisposeSystemMGRP(b, a) (0x00836830) when the worker has a model | - | int `0x00a7a450` (native_chr) |
| `0x503c` | `Native_503c` | v int | 0: Ch_DisposeMGRP(v) (0x00836ad0) | - | int `0x00a7a4d0` (native_chr) |
| `0x503d` | `Native_503d` | v int | 0: ChN_DataDispose(v, 1) (0x00824f00) | - | int `0x00a7a520` (native_chr) |
| `0x503e` | `Native_503e` | b int, a int | 0: ChN_DataDispose(b, a) (0x00824f00) | - | int `0x00a7a570` (native_chr) |
| `0x503f` | `Native_503f` | v | Ch_SetSequence(model, v) (0x00837ac0) | - | int `0x00a7a5a0` (native_chr) |
| `0x5040` | `Native_ChrSetMotion2` | motion int | 0 | - | int `0x00a7a610` (native_move) |
| `0x5041` | `Native_5019_copy_00a7a740` | as Native_PlayMotionCommon | 0: Native_PlayMotionCommon(worker, stack, 0, 0) | - | int `0x00a7a740` (native_mods) |
| `0x5042` | `Native_ChrPlayMotion2` | as 0x501a | 0 | - | int `0x00a7a780` (native_move) |
| `0x5043` | `Native_5043` | as Native_PlayMotionCommon | 0: Native_PlayMotionCommon(worker, stack, 0, 1) | - | int `0x00a7a7c0` (native_mods) |
| `0x5044` | `Native_5044` | as Native_PlayMotionCommon | 0: Native_PlayMotionCommon(worker, stack, 1, 1) (0x00a7aae0) | - | int `0x00a7a810` (native_chr) |
| `0x5045` | `Native_5045` | as Native_PlayMotionCommon | 0: Native_PlayMotionCommon(worker, stack, 0, 1) (as 0x5043) | - | int `0x00a7a830` (native_mods) |
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
| `0x5053` | `Native_5053` | i int, id int | int: Ch_ElemGetDirect(worker i's model, id) (0x00833bf0), 0 without a model | - | int `0x00a78900` (native_mods) |
| `0x5054` | `Native_5054` | part int, on int | int: Ch_ModelSetHide(model, part, on) (0x0082aea0); 0 without a model | - | int `0x00a78960` (native_chr) |
| `0x5055` | `Native_5055` | v int | int: Ch_ModelGetHide(model, v) (0x0082add0), 0 without a model | - | int `0x00a78a10` (native_mods) |
| `0x5056` | `Native_5056` | v int | int: Ch_SetCalcNorm(model, v) (0x00835750); 0 without a model | - | int `0x00a78a80` (native_chr) |
| `0x5057` | `Native_ChrSetBoneUpdate` | worker int, on int | int: previous | - | int `0x00a78af0` (native) |
| `0x5058` | `Native_5058` | v int | 0: Ch_AllSetCalcNorm(v) (0x008325b0) | - | int `0x00a78b70` (native_chr) |
| `0x5059` | `Native_5059` | slot int, motion int | 0: Ch_SetSysMotion(model, slot, motion) (0x00837d00) for a model | - | int `0x00a78bd0` (native_mods) |
| `0x505a` | `Native_505a` | b, a | Ch_TextureSetClutBase(model, b, a) (0x008313c0), 0 without a model | - | int `0x00a78c60` (native_chr) |
| `0x505b` | `Native_505b` | b int, a int | int: Ch_TextureSetImageBase(model, b, a) (0x00831390), 0 without a model | - | int `0x00a78ce0` (native_mods) |
| `0x505c` | `Native_ChrSetNextMotionBlend` | frames int (-1 = 0, 0 = 1) | 0 | - | int `0x00a78d70` (native) |
| `0x505d` | `Native_ChrGetMotion` | - | int: motion id | - | int `0x00a78dd0` (native) |
| `0x505e` | `Native_505e` | - | Ch_GetSp(model) (0x0082ad60), 0.0 without a model | - | float `0x00a78e20` (native_chr) |
| `0x505f` | `Native_505f` | v int | 0: Ch_MotionSetLoop(model, v) (0x00839960) | - | int `0x00a78e90` (native_chr) |
| `0x5060` | `Native_5060` | v | Ch_IdEnableShade(model, v) (0x008340f0) | - | int `0x00a78f00` (native_chr) |
| `0x5061` | `Native_5061` | b int, a int | 0: Ch_MgrpRead(b, a) (0x00836a40) | - | int `0x00a78f80` (native_chr) |
| `0x5062` | `Native_5062` | id int, n int, keep int | the id; the poll 5 when done; int slot the args word +0xc (with keep, ChN_ReadSystemMGRPsync's result) | with keep set one call of ChN_ReadSystemMGRPsync(id, n) (0x00836a50), else until it returns 0 | start `0x00a79030`, poll `0x00a79070`, int `0x00a79120` (native_mods) |
| `0x5063` | `Native_5063` | id int, n int | int: args +0xc, ChN_ReadSystemMGRPsync(id, n)'s result (0x00836a50) | as 0x504f | start `0x00a79160`, poll `0x00a791a0`, int `0x00a79210` (native_chr) |
| `0x5064` | `Native_5064` | id int, keep int | the id; the poll 5 (with keep) or 1 when done; int slot the args word +0xc | with keep set one call of Ch_DataReadSync(id) (0x0082a040), else until it returns 0 | start `0x00a79240`, poll `0x00a79290`, int `0x00a79300` (native_mods) |
| `0x5065` | `Native_5065` | v int | 0: 0x00837650(v) (the dbgPrintf left out) | - | int `0x00a79420` (native_chr) |
| `0x5066` | `Native_5066` | n | start: args +0 = n, +0xc = 0; EAX n; poll: 1 once 0x00837670(n) is 0 (the dbgPrintf left out), else 0; int: args +0xc | see the poll | start `0x00a79480`, poll `0x00a794d0`, int `0x00a79540` (native_chr) |
| `0x5067` | `Native_5067` | v | Ch_DrawSetCull(model, v) (0x008300b0) | - | int `0x00a79580` (native_chr) |
| `0x5068` | `Native_WaitCdIdle` | - | 1 when done | until the CD reads are idle | poll `0x00a795d0` (native) |
| `0x5069` | `Native_5069` | v | Ch_SetKeepFps(model, v) (0x00839b50), 0 without a model | - | int `0x00a795e0` (native_chr) |
| `0x506a` | `Native_506a` | v | Ch_PLightSetEnable(model, v) (0x00830ae0) | - | int `0x00a79640` (native_chr) |
| `0x506b` | `Native_506b` | - | int: Ch_PLightGetEnable(model) (0x00830ad0), 0 without a model | - | int `0x00a796c0` (native_mods) |
| `0x506c` | `Native_506c` | bits int, mask int | 0: Ch_PLightSetEnableMask(model, bits, mask) (0x00830b00) for a model | - | int `0x00a797b0` (native_mods) |
| `0x506d` | `Native_506d` | a f, b f | Ch_SetRotX(model, wrap(b degrees), a) (0x008357f0) | - | int `0x00a79a80` (native_chr) |
| `0x506e` | `Native_506e` | v f | Ch_BindSetScale(model, v) (0x00832610) | - | int `0x00a79b30` (native_chr) |
| `0x506f` | `Native_506f` | v int (popped only for a model) | 0: Ch_SetHideCalc(model, v) (0x0082b280) | - | int `0x00a79b90` (native_chr) |
| `0x5070` | `Native_5070` | - | int: Ch_GetHideCalc(model) (0x0082abe0), 0 without a model | - | int `0x00a79c30` (native_mods) |
| `0x5071` | `Native_5071` | v | Ch_TextureLoadImage(model, v) (0x00831300) | - | int `0x00a79cc0` (native_chr) |
| `0x5072` | `Native_5072` | - | Ch_IdGetPNum(model) (0x0082adc0), 0 without a model | - | int `0x00a79cf0` (native_chr) |
| `0x5073` | `Native_5073` | v | Ch_SetOverlapHit(model, v) (0x0082b420), popped only for a model | - | int `0x00a79d30` (native_chr) |
| `0x5074` | `Native_5074` | - | int: Ch_GetOverlapHit(model) (0x0082ac70), 0 without a model | - | int `0x00a79d90` (native_mods) |
| `0x5075` | `Native_ChrGetMotionFrameOf` | worker int | int: its motion frame | - | int `0x00a79dd0` (native) |
| `0x5076` | `Native_5076` | on int (popped only for a model) | 0: Ch_MouthSetAutoTalk(model, on) (0x00834770) | - | int `0x00a79e30` (native_chr) |
| `0x5077` | `Native_5077` | - | int: Ch_MouthGetAutoTalk(model) (0x00834740), 0 without a model | - | int `0x00a79e80` (native_mods) |
| `0x5078` | `Native_5078` | b int, a int | 0: Ch_TextureSetAnim(model, b, a) (0x0083d040) | - | int `0x00a79ef0` (native_chr) |
| `0x5079` | `Native_5079` | on int | 0: Ch_NeckSetEnable(model, on) (0x00834f60) | - | int `0x00a79f70` (native_chr) |
| `0x507a` | `Native_507a` | worker int | 0: Ch_NeckSetTarget(model, that worker's model) (0x00835050) | - | int `0x00a79fc0` (native_chr) |
| `0x507b` | `Native_507b` | x f, y f, z f | Ch_NeckSetTargetPos(model, &[x, y, z]) (0x008350a0) | - | int `0x00a7a000` (native_chr) |
| `0x507c` | `Native_507c` | - | Ch_NeckSetTargetCamera(model) (0x00835080) | - | int `0x00a7a070` (native_chr) |
| `0x507d` | `Native_507d` | b f, a f | degrees: Ch_NeckSetRot(model, wrap(b), wrap(a)) (0x00834ff0) | - | int `0x00a7a090` (native_chr) |
| `0x507e` | `CT_507E_ret` | worker int, on int: mouth flapping | 0 | - | int `0x00a7a210` (native) |
| `0x507f` | `Native_507f` | i int, v f | 0: 0x0082ae30(model, i, v) for a model | - | int `0x00a7a380` (native_mods) |
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
| `0x508a` | `Native_508a` | as Native_PlayMotionCommon | 0: Native_PlayMotionCommon(worker, stack, 2, 1) | - | int `0x00a7a720` (native_mods) |
| `0x508b` | `Native_508b` | - | the same as 0x5089 | - | int `0x00a7a760` (native_chr) |
| `0x508c` | `Native_508c` | as Native_PlayMotionCommon | 0: Native_PlayMotionCommon(worker, stack, 2, 1) (as 0x508a) | - | int `0x00a7a7a0` (native_mods) |
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
| `0x6009` | `Native_6009` | - | int: 0x007bab90(worker, args, stack, 1); 0 | - | int `0x007b93e0` (native_mods) |
| `0x600a` | `Native_600a` | - | 0x007bb2b0(..., 1, 0); 0 | - | int `0x007b9440` (native_chr) |
| `0x600b` | `Native_600b` | - | int: 0x007bb2b0(worker, args, stack, 1, 1); 0 | - | int `0x007b9480` (native_mods) |
| `0x600c` | `Native_600c` | - | int: MsAtelCameraRefAddCircleArea_007bb370(worker, args, stack, 1, 0); 0 | - | int `0x007b94c0` (native_mods) |
| `0x600d` | `Native_600d` | - | int: MsAtelCameraRefAddCircleArea_007bb370(worker, args, stack, 1, 1); 0 | - | int `0x007b9500` (native_mods) |
| `0x600e` | `Native_600e` | on | int: 0x007bf1a0(cam, 1, on & 1); the camera handle | - | int `0x007b95a0` (native_mods) |
| `0x600f` | `Native_Cam_600f` | - | int: camera handle (0x007bed10(cam, 1), eye half) | - | int `0x007b9520` (native_cam) |
| `0x6010` | `Native_CamMove` | frames int: eye moves to its set position, linear | 0 | no (0x601a waits) | int `0x007b96c0` (native_cam) |
| `0x6011` | `Native_Cam_6011` | frames int: eye move, mode 1, type 1 | 0 | - | int `0x007b9730` (native_cam) |
| `0x6012` | `Native_Cam_6012` | frames int: eye move, type 3 | 0 | - | int `0x007b9800` (native_cam) |
| `0x6013` | `Native_6013` | frames | MsCameraMoveFrame(cam, 1, 1, frames, 3) (0x007bde50); 0 | - | int `0x007b9870` (native_chr) |
| `0x6014` | `Native_CamMoveEased` | accel int, hold int, decel int, curve int: eased eye move | 0 | - | int `0x007b9940` (native_cam) |
| `0x6015` | `Native_CamMoveEased1` | accel int, hold int, decel int, curve int: as 0x6014, mode 1 | 0 | - | int `0x007b9980` (native_cam) |
| `0x6016` | `Native_CamMoveFinish` | - | int: camera handle (eye transition applied now; with magic 0x92 the second call arms Native_BtlRandom) | - | int `0x007b9b40` (native_cam) |
| `0x6017` | `Native_Cam_6017` | a f, b f, c f, d f: eye channel words +0x130..+0x13c (0x007bdde0) | 0 | - | int `0x007b9a00` (native_cam) |
| `0x6018` | `Native_6018` | - | int: 0x007bb210(worker, args, stack, 1); 0 | - | int `0x007b9a80` (native_mods) |
| `0x6019` | `Native_Cam_6019` | - | int: camera handle (0x007bec90(cam, 1)) | - | int `0x007b9ac0` (native_cam) |
| `0x601a` | `Native_CamWait` | - | 5 when done | until the eye half stops moving | poll `0x007b9be0` (native_cam) |
| `0x601b` | `Native_601b` | - | Cam_IsMoving(cam, 1) (0x007bdd20) | - | int `0x007b9ca0` (native_chr) |
| `0x601c` | `Native_Cam_601c` | var int: variable reference, degrees f | int: camera handle (0x007bf520(cam, 1, 1, &var, radians)) | - | int `0x007b8330` (native_cam) |
| `0x601d` | `Native_Cam_601d` | var int, degrees f, y f, x f | int: camera handle (0x007bf600(cam, 1, 1, &var, radians, y, x)) | - | int `0x007b8470` (native_cam) |
| `0x601e` | `Native_601e` | var, deg f | 0x007bf520(cam, 1, 0, &var, deg * pi / 180); the camera | - | int `0x007b83d0` (native_chr) |
| `0x601f` | `Native_Cam_601f` | as 0x601d, mode 0 | int: camera handle | - | int `0x007b8530` (native_cam) |
| `0x6020` | `Native_CamRefSetPos` | x f, y f, z f: reference (look-at) position | 0 | - | int `0x007b9d00` (native_cam) |
| `0x6021` | `Native_Cam_6021` | x var, y var, z var: variable references | 0 (the reference position stored) | - | int `0x007b9d40` (native_cam) |
| `0x6022` | `Native_6022` | - | int: 0x007bb5d0(worker, args, stack, 0, 0); 0 | - | int `0x007b9e10` (native_mods) |
| `0x6023` | `Native_6023` | - | int: 0x007bb5d0(worker, args, stack, 0, -1); 0 | - | int `0x007b9e90` (native_mods) |
| `0x6024` | `Native_6024` | - | int: MsAtelCameraEyeAddArea_007bb410(worker, args, stack, 0, -1); 0 | - | int `0x007b9f20` (native_mods) |
| `0x6025` | `Native_6025` | - | int: MsAtelCameraEyeAddArea_007bb410(worker, args, stack, 0, 0); 0 | - | int `0x007b9fa0` (native_mods) |
| `0x6026` | `Native_6026` | - | int: MsAtelCameraEyeAddArea_007bb410(worker, args, stack, 0, 1); 0 | - | int `0x007b9fc0` (native_mods) |
| `0x6027` | `Native_CamRefSetTarget` | target int, bone int, y f: reference follows a target | 0 | - | int `0x007b9fe0` (native_cam) |
| `0x6028` | `Native_Cam_6028` | five floats: reference constraint, kind 0 | 0 | - | int `0x007ba0e0` (native_cam) |
| `0x6029` | `Native_CamRefAddLineArea` | x1 f, z1 f, x2 f, z2 f, r f: reference kept near a segment | 0 | - | int `0x007ba1d0` (native_cam) |
| `0x602a` | `Native_602a` | - | 0x007bb370(..., 0, 0); 0 | - | int `0x007ba1f0` (native_chr) |
| `0x602b` | `Native_CamRefAddCircleArea` | x f, z f, r f: reference kept by a circle | 0 | - | int `0x007ba2d0` (native_cam) |
| `0x602c` | `Native_602c` | on | int: 0x007bf1a0(cam, 0, on & 1); the camera handle | - | int `0x007ba350` (native_mods) |
| `0x602d` | `Native_CamRefClearAreas` | - | int: camera handle (reference constraints cleared) | - | int `0x007ba2f0` (native_cam) |
| `0x602e` | `Native_CamFocusTimeLinear` | frames int: focus move, linear | 0 | - | int `0x007b7f70` (native_cam) |
| `0x602f` | `Native_602f` | frames | int: MsCameraMoveFrame(cam, 0, 1, frames, 1); 0 | - | int `0x007b7fe0` (native_mods) |
| `0x6030` | `Native_Cam_6030` | frames int: focus move, type 3 | 0 | - | int `0x007b8050` (native_cam) |
| `0x6031` | `Native_6031` | frames | int: MsCameraMoveFrame(cam, 0, 1, frames, 3); 0 | - | int `0x007b80c0` (native_mods) |
| `0x6032` | `Native_CamFocusTimeEase` | accel int, hold int, decel int, curve int: eased focus move | 0 | - | int `0x007b8130` (native_cam) |
| `0x6033` | `Native_6033` | - | 0x007ba5b0(worker, args, stack, 0, 1); 0 | - | int `0x007b8150` (native_chr) |
| `0x6034` | `Native_CamFocusCut` | - | int: camera handle (focus jumps to its target) | - | int `0x007b8210` (native_cam) |
| `0x6035` | `Native_Cam_6035` | as 0x6017 for the focus half | 0 | - | int `0x007b8170` (native_cam) |
| `0x6036` | `Native_6036` | - | int: 0x007bb210(worker, args, stack, 0); 0 | - | int `0x007b8190` (native_mods) |
| `0x6037` | `Native_6037` | - | int: MsCameraResetDirVector(cam, 0) (0x007bec90); the camera handle | - | int `0x007b81b0` (native_mods) |
| `0x6038` | `Native_CamFocusWait` | - | 5 when done | until the focus half stops moving | poll `0x007b8270` (native_cam) |
| `0x6039` | `Native_6039` | - | int: Cam_IsMoving(cam, 0) (0x007bdd20) | - | int `0x007b82d0` (native_mods) |
| `0x603a` | `Native_CamSetRoll` | degrees f: eye roll | int: camera handle | - | int `0x007b9010` (native_cam) |
| `0x603b` | `Native_CamSetScreenDepth` | depth f: screen depth (zoom) | int: camera handle | - | int `0x007b90b0` (native_cam) |
| `0x603c` | `Native_Cam_603c` | angle f (deg), b f, c f, d f: eye, kind 10 (0x007befc0) | 0 | - | int `0x007b9400` (native_cam) |
| `0x603d` | `Native_603d` | - | int: 0x007bac40(worker, args, stack, 0); 0 | - | int `0x007ba000` (native_mods) |
| `0x603e` | `Native_603e` | - | int: 0x007bad10(worker, args, stack, 1); 0 | - | int `0x007b86d0` (native_mods) |
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
| `0x6053` | `Native_6053` | id, v | int: MsScreenMove(id, 0, v, 1); 0 | - | int `0x007b8c60` (native_mods) |
| `0x6054` | `Native_6054` | id, v | int: MsScreenMove(id, 0, v, 3); 0 | - | int `0x007b8c90` (native_mods) |
| `0x6055` | `Native_6055` | d, c, b, a | MsScreenMoveAcc(d, 0, c, b, a) (0x007bbeb0) | - | int `0x007b8cc0` (native_chr) |
| `0x6056` | `Native_6056` | id, v | int: MsScreenMove(id, 1, v, 1); 0 | - | int `0x007b8d10` (native_mods) |
| `0x6057` | `Native_6057` | id, v | int: MsScreenMove(id, 1, v, 3); 0 | - | int `0x007b8d40` (native_mods) |
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
| `0x6065` | `Native_6065` | p | int: MsCameraPriority(cam, p) (0x007be0a0); the camera handle | - | int `0x007b87d0` (native_mods) |
| `0x6066` | `Native_6066` | - | the shake with (0, 2) | - | int `0x007b8fa0` (native_chr) |
| `0x6067` | `Native_6067` | - | the shake with (1, 2) | - | int `0x007b8fc0` (native_chr) |
| `0x6068` | `Native_6068` | - | int: MsAtelScreenShake(worker, args, stack, 2) (0x007bb840); 0 | - | int `0x007b9460` (native_mods) |
| `0x6069` | `Native_6069` | - | MsAtelCameraShake(worker, args, stack, 0, 3) (0x007bb8c0) | - | int `0x007b8ff0` (native_chr) |
| `0x606a` | `Native_606a` | - | int: MsAtelCameraShake(worker, args, stack, 1, 3)'s EAX | - | int `0x007b9090` (native_mods) |
| `0x606b` | `Native_Cam_606b` | chan int (< 3), flags int, b f, c int, d int, e int: channel interpolation (0x007c56f0, kind 3) | 0 | - | int `0x007b94a0` (native_cam) |
| `0x606c` | `Native_606c` | - | the shake with (0, 4) | - | int `0x007b9150` (native_chr) |
| `0x606d` | `Native_606d` | - | int: MsAtelCameraShake(.., 1, 4)'s EAX | - | int `0x007b9170` (native_mods) |
| `0x606e` | `Native_606e` | - | 0x007bb840(..., 4); 0 | - | int `0x007b94e0` (native_chr) |
| `0x606f` | `Native_606f` | - | int: MsAtelCameraShake(.., 0, 5)'s EAX | - | int `0x007b91d0` (native_mods) |
| `0x6070` | `Native_6070` | - | int: MsAtelCameraShake(.., 1, 5)'s EAX | - | int `0x007b9200` (native_mods) |
| `0x6071` | `Native_Cam_6071` | as 0x606b, kind 5 | 0 | - | int `0x007b9580` (native_cam) |
| `0x6072` | `Native_6072` | - | int: MsAtelCameraShake(.., 0, 6)'s EAX | - | int `0x007b9240` (native_mods) |
| `0x6073` | `Native_6073` | - | int: MsAtelCameraShake(.., 1, 6)'s EAX | - | int `0x007b9280` (native_mods) |
| `0x6074` | `Native_6074` | - | 0x007bb840(worker, args, stack, 6); 0 | - | int `0x007b9610` (native_chr) |
| `0x6075` | `Native_6075` | b, a | int: 0x007bc870(b, a); 0 | - | int `0x007b8b50` (native_mods) |
| `0x6076` | `Native_Cam_6076` | - | 0 (the battle cameras freed) | - | int `0x007b8fe0` (native_cam) |
| `0x6077` | `Native_6077` | - | float: the camera's +0xc90 angle (0x007bd500) in degrees (MsNearRag(0, v) * 180 / pi) | - | float `0x007b9190` (native_chr) |
| `0x6078` | `Native_6078` | - | jumps to 0x007bd510(worker, args, stack) (ST0) | - | float `0x007b91f0` (native_chr) |
| `0x6079` | `Native_6079` | id | int: MsScreenMoveReset(id, 0) (0x007bc010); 0 | - | int `0x007b8dc0` (native_mods) |
| `0x607a` | `Native_607a` | id | int: MsScreenMoveReset(id, 1); 0 | - | int `0x007b8de0` (native_mods) |
| `0x607b` | `Native_607b` | screen | start: into the args word; EAX it; poll: 5 once MsScreenMoveCheck(screen, 0) (0x007bbfc0) is 0 | see the poll | start `0x007b8e40`, poll `0x007b8e60` (native_chr) |
| `0x607c` | `Native_607c` | id | AtelGetBattleIdAdrs2(worker) (0x0086a990, unused), args +0 = id; id | - | start `0x007b8e00`, poll `0x007b8e20` (native_mods) |
| `0x607d` | `Native_Cam_607d` | v int | int: camera handle (0x007bcdd0(cam, v)) | - | int `0x007b8840` (native_cam) |
| `0x607e` | `Native_CamSetDepthMaskMode` | flags | 0x007bcf50(cam, flags); the camera | - | int `0x007b88b0` (native_chr) |
| `0x607f` | `Native_CamSetDepthMask` | plane, z | 0x007bf280(cam, plane, z); the camera | - | int `0x007b8920` (native_chr) |
| `0x6080` | `Native_BtlRandom` | b int (bit 0 picks draw table 14 or 15) | int: the draw; with the latch armed 2 for odd, 0 for even | - | int `0x007b89a0` (native_cam) |
| `0x6081` | `Native_6081` | - | the shake with (0, 7) | - | int `0x007b92c0` (native_chr) |
| `0x6082` | `Native_6082` | - | the shake with (1, 7) | - | int `0x007b9300` (native_chr) |
| `0x6083` | `Native_Cam_6083` | as 0x605c, flags 8 | int: camera handle | - | int `0x007b9340` (native_cam) |
| `0x6084` | `Native_Cam_6084` | as 0x605d, flags 8 | int: camera handle | - | int `0x007b9380` (native_cam) |
| `0x6085` | `Native_6085` | - | int: 0x007ba460(worker, args, stack, 1); 0 | - | int `0x007b92a0` (native_mods) |
| `0x6086` | `Native_6086` | - | int: 0x007ba460(worker, args, stack, 0); 0 | - | int `0x007b9d80` (native_mods) |
| `0x6087` | `Native_Cam_6087` | - | int: camera handle (0x007beae0(cam, 0)) | - | int `0x007b8a10` (native_cam) |
| `0x6088` | `Native_Cam_6088` | - | int: camera handle (0x007beae0(cam, 1)) | - | int `0x007b8a70` (native_cam) |
| `0x6089` | `Native_6089` | p8, p7, p6, p5, p4 f, p3, p2 f, p1 | int: MsScreenShake2(p8, p7, 9, p6, p5, p4, p3, p2, p1) (0x007c57b0); 0 | - | int `0x007b9630` (native_mods) |
| `0x7000` | `Native_7000` | - | 0 (actor +0xdf2 = 0) | - | int `0x007a8620` (native_btl) |
| `0x7001` | `Native_BtlSetFlagBded` | v | int: the byte 0x0112bded = v & 1; 0 | - | int `0x007a39d0` (native_mods) |
| `0x7002` | `Native_7002` | a int, b int | the start's result; poll returns 1 at once | no | start `0x007a3530` (native_btl), poll `0x007a35c0` (not ported) |
| `0x7003` | `Native_7003` | a int, b int | 0 (0x00795eb0(actor, 2, 0, -1, b, a)) | - | int `0x007a3d90` (native_btl) |
| `0x7004` | `Native_7004` | v f | 0; the f32 0x0112bde4 = v | - | int `0x007a3aa0` (native_btl2) |
| `0x7005` | `Native_BtlIsUnderwater` | - | int: the i8 at 0x0112c9ce | - | int `0x007a4550` (native_mods) |
| `0x7006` | `Native_7006` | a int, b int | 0 (as 0x7003 with 1, when actor +0x41d) | - | int `0x007a3f50` (native_btl) |
| `0x7007` | `Native_ActorPlayMotion` | as 0x007a8fc0(worker, args, stack, 0) | its result | - | int `0x007a35a0` (native_move) |
| `0x7008` | `Native_ActorWaitMotion` | - | 5 when done | until the actor's motion ends (no actor/model: at once) | poll `0x007a3980` (native_btl) |
| `0x7009` | `Native_7009` | b int | 0 | - | int `0x007a3b20` (native_btl) |
| `0x700a` | `Native_700a` | b int, f f | 0 (actor +0x50c = b, +0x538 = f) | - | int `0x007a4100` (native_btl) |
| `0x700b` | `Native_BtlUseCommand` | targets int (selector), command int | 1 if used, else 0 (actor cannot act) | - | int `0x007a44b0` (native_btl) |
| `0x700c` | `Native_700c` | - | 0 (0x007a9150(worker, args, stack, 0.0)) | - | int `0x007a4310` (native_btl) |
| `0x700d` | `Native_700d` | mode int, k int | int: 0x007a87f0(&to, &from, actor, abs k, 0); with the actor locked (+0x44e), 1 for a negative k else 0 | - | int `0x007a63f0` (native_btl) |
| `0x700e` | `Native_700e` | sel | int: MsSetDamage(actor, sel, 0) (0x0078d980) for an actor; 0 | - | int `0x007a5690` (native_mods) |
| `0x700f` | `Native_GetActorProperty` | selector int, property int | int: MsGetChrStatInfo's value | - | int `0x007a4d50` (native_btl) |
| `0x7010` | `Native_BtlSelectTarget` | a int, b int, c int, d int | int: the selected target | - | int `0x007a5af0` (native_btl) |
| `0x7011` | `Native_7011` | v int | 0; the byte 0x0112a8fc = v & 1 | - | int `0x007a4280` (native_btl2) |
| `0x7012` | `Native_7012` | - | 0 | - | int `0x007a29a0` (native_btl) |
| `0x7013` | `Native_7013` | v f | 0; with a model, 0x0082b840(model, v) | - | int `0x007a5840` (native_btl2) |
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
| `0x7020` | `Native_7020` | - | int: 0x007a92c0(actor, its +0xdc0), 0 without an actor | - | int `0x007a53b0` (native_btl2) |
| `0x7021` | `Native_ActorGetId` | selector int | int: the picked actor's id, -1 for none | - | int `0x007a6a80` (native_btl) |
| `0x7022` | `Native_7022` | v int | 0: the byte 0x0112c9dc = v | - | int `0x007a4cd0` (native_btl) |
| `0x7023` | `Native_7023` | selector int | int: ftol of the mean of MsSetMove(i, 1) (0x007a9fc0) over the picked actors (f32 sum and mean) | - | int `0x007a4940` (native_btl) |
| `0x7024` | `Native_BtlGetBattleId` | - | int: MsGetBattleScene() | - | int `0x007a4e90` (native_btl) |
| `0x7025` | `Native_7025` | d int, c int, b int, a int | int: out of MsActTargetSearch(actor, d, c, b, 4, a, 0, &out) (0x007acd40), whose result goes to 0x00796430(actor, r) | - | int `0x007a5cf0` (native_btl) |
| `0x7026` | `Native_7026` | v int | 0 (actor +0x44f = v) | - | int `0x007a5d80` (native_btl) |
| `0x7027` | `Native_7027` | - | int: the actor's byte +0x44f, 0 without an actor | - | int `0x007a5e90` (native_btl2) |
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
| `0x703d` | `Native_703d` | - | int: the byte 0x0112a8e4 = 0; 0 | - | int `0x007a5980` (native_mods) |
| `0x703e` | `Native_703e_Poll` | - | with 0x0112a8e4 set, 0x0112a8e5 drops to 0 when it is 1. Unless the i8 at 0x0112c9e4 is negative and 0x0112c9d0, 0x0112c9d8, 0x0112bde1, 0x0112c944 are all 0, the pair at 0x0112ca08 = (0xff, 0xde) and it waits (0). Else the first of the 31 actors that is busy (+0xdc8 with +0x508, +0x432 or +0xdfb: kind 0xdf) or acting (+0x10 with +0x436 or +0xdf0: 0xdd) goes to 0x0112ca08 with its kind and it waits; none: done (5) | - | poll `0x007a5a10` (native_mods) |
| `0x703f` | `Native_BtlRunEvent` | event int, a int | the event handle; poll its 0x007a8700 | until the event ends | start `0x007a5df0`, poll `0x007a5ec0` (native_btl) |
| `0x7040` | `Native_7040` | v int | int: 0x0079e8c0(actor, v); 0 without an actor | - | int `0x007a5610` (native_btl) |
| `0x7041` | `Native_7041` | - | 0 | - | int `0x007a56d0` (native_btl) |
| `0x7042` | `Native_7042` | win, k, x, y, anchor | int: 0x0112c250 = k; TODrawAMES(win clamped 0..6, text k, x, y, anchor) (0x0089b140) from the table at *0x0112a9b8 when there is one; 0 | - | int `0x007a6060` (native_mods) |
| `0x7043` | `Native_7043_Poll` | - | 5 when done | until 0x008b7ca0(handle) is 0, or 0x008b8530(handle) is set (then 0x008ab8f0(handle)); the handle is the argument word | start `0x007a6a10` (not ported), poll `0x007a6a40` (native_btl2) |
| `0x7044` | `Native_7044` | win | int: TOCloseMessageWindow(win clamped 0..6) (0x008ab8f0); 0 | - | int `0x007a6e10` (native_mods) |
| `0x7045` | `Native_7045` | sel | int: MsSetMove(first of sel, 0xc) (0x007a9fc0, ST0) over the double at 0x00b43d70, rounded to f32, truncated | - | int `0x007a4da0` (native_mods) |
| `0x7046` | `Native_7046` | v int | 0: MsMoveSplineSet(v) (0x007ab140) | - | int `0x007a5220` (native_btl) |
| `0x7047` | `Native_7047` | i int, a int | 0 | - | int `0x007a5270` (native_btl) |
| `0x7048` | `Native_7048` | i int, x f, y f, z f | 0 | - | int `0x007a53f0` (native_btl) |
| `0x7049` | `Native_7049` | d int, c int, f f, a int | 0: 0x007ab010(d, actor, c, f, a) | - | int `0x007a55a0` (native_btl) |
| `0x704a` | `Native_704a` | selector int | int: picked actors with +0x415 set | - | int `0x007a58d0` (native_btl) |
| `0x704b` | `Native_704b` | selector int, motion int, b int | the args word: 1 when MsSetMotion(i, motion, b, 0, 1, worker, &out) (0x007ab360) set out for any picked actor | until 0x007a8700(args) is done | start `0x007a7640`, poll `0x007a7850` (native_btl) |
| `0x704c` | `Native_704c` | selector int | the argument word = MsGetChrBit(actor, selector, 0, 1), returned | - | start `0x007a7bf0` (native_btl2), poll `0x007a7cf0` (not ported) |
| `0x704d` | `Native_704d` | - | int: 0 (xor eax, eax; ret) | - | int `0x007a74e0` (native_mods) |
| `0x704e` | `Native_704e` | v int | 0; the actor's bytes +0x3f5 and +0x3f6 = v | - | int `0x007a7530` (native_btl2) |
| `0x704f` | `Native_704f` | selector int, v int | 0; every actor below 31 in MsGetChrBit(actor, selector, 0, 1) gets the byte +0xddd = v | - | int `0x007a70c0` (native_btl2) |
| `0x7050` | `Native_7050` | selector int | the args word: MsSetChrVisibleBtl2Event(worker, MsGetChrBit(actor, selector, 0, 1), -1) (0x007a89c0) | until 0x007a8700(args) is done | start `0x007a7400`, poll `0x007a74f0` (native_btl) |
| `0x7051` | `Native_7051` | - | 5 when done | until actor +0xe12 is 0 | poll `0x007a5890` (native_btl) |
| `0x7052` | `Native_7052` | a int: selector, b int: target selector, v int | 0 | - | int `0x007a7e10` (native_btl) |
| `0x7053` | `Native_7053` | f f | 0: 0x007a9150(worker, args, stack, f) | - | int `0x007a4470` (native_btl) |
| `0x7054` | `Native_7054` | to int, from int, point int | 0: the from actor's model point (Ch_OptposGet 0x008354f0) through 0x0079e1b0(0, &v) into MsSetChrPosReset(to, &v) (0x00796220) | - | int `0x007a7fe0` (native_btl) |
| `0x7055` | `Native_7055` | b int | 0 | - | int `0x007a3eb0` (native_btl) |
| `0x7056` | `Native_7056` | v | int: MsBattleSpecialInit(v) (0x0079de60); 0 | - | int `0x007a6e80` (native_mods) |
| `0x7057` | `Native_7057` | target int, f f (dropped), b int, a int | 0: MsSetChrDirTarget(actor, target, 0, -1, b, a) (0x00795eb0) | - | int `0x007a4220` (native_btl) |
| `0x7058` | `Native_7058` | selector int, motion int | int: 0 when the picked actor plays it, else -1 | - | int `0x007a8160` (native_btl) |
| `0x7059` | `Native_7059` | v int: target | float: distance (MsGetTargetDist mode 1) | - | float `0x007a6e40` (native_btl) |
| `0x705a` | `Native_705a` | targets int, command int | 1, 0 without an actor | - | int `0x007a49f0` (native_btl) |
| `0x705b` | `Native_705b` | selector int | float: 0x00794a70(selected actor), 0.0 for none | - | float `0x007a2d80` (native_btl2) |
| `0x705c` | `Native_705c` | selector int | float: the selected actor's 0x0082ab30(model) times its f32 +0x3a4 (rounded to f32); 0.0 without actor or model | - | float `0x007a2df0` (native_btl2) |
| `0x705d` | `Native_ActorSetFlag36` | a int, b int (dropped) | int: the actor index (+0x36 = 1) | - | int `0x007a51e0` (native_btl) |
| `0x705e` | `Native_705e` | - | 0; 0x00788e60(actor) | - | int `0x007a5310` (native_btl2) |
| `0x705f` | `Native_705f` | v f | debug (mod-tool candidate): dbgPrintf of v (as a double) and the actor (a bare ret in this build); 0 | - | int `0x007a4c30` (not ported) |
| `0x7060` | `Native_7060` | - | 0; 0x0079eff0(actor) with an actor | - | int `0x007a62a0` (native_btl2) |
| `0x7061` | `Native_7061` | - | 0; 0x0079f110(actor) with an actor | - | int `0x007a6300` (native_btl2) |
| `0x7062` | `Native_7062` | a int, b int | int: 0x0079eba0(actor, a, b, 1) | - | int `0x007a4580` (native_btl) |
| `0x7063` | `Native_7063` | - | 5 when the actor's +0xe13 is clear, else 0 | until +0xe13 is clear | poll `0x007a5dc0` (native_btl) |
| `0x7064` | `Native_7064` | v int | v, kept in the argument word and the word 0x0112c24c | until 0x007a8c20() is set and 0x007b2c10(1) is 0 (poll 5) | start `0x007a63d0`, poll `0x007a64c0`, int `0x007a65d0` (native_btl2) |
| `0x7065` | `Native_7065_Poll` | - | GetCD(); done (5) once 0x007a8c20() is set and msVoiceCheckStanby(0) (0x007b2c10) is 0 | - | poll `0x007a6760`, int `0x007a6950` (native_mods) |
| `0x7066` | `Native_7066` | - | int: jumps to 0x007b2c00 | - | int `0x007a6960` (native_mods) |
| `0x7067` | `Native_7067` | - | int: msVoiceIsStreaming() != 0 (0x007b2be0) | - | int `0x007a6a30` (native_mods) |
| `0x7068` | `Native_7068_Poll` | - | 1 once msVoiceIsStreaming() is 0, else 0 | - | poll `0x007a6ad0` (native_mods) |
| `0x7069` | `Native_BtlFindActor` | p4, p3, p2, p1 | int: MsActTargetSearch(0, p4, p3, p2, 7, p1, 0, &pick) (0x007acd40)'s pick (an out word, zeroed first) | - | int `0x007a6170` (native_mods) |
| `0x706a` | `Native_706a` | selector int | int: 0 when the selected actor's +0xdc0 is 1 << its index, else -1 | - | int `0x007a7580` (native_btl2) |
| `0x706b` | `Native_706b` | selector int, a int, b int | 0 | - | int `0x007a84c0` (native_btl) |
| `0x706c` | `Native_706c` | sel, k | int: for the first actor x of sel, the record of its +0x14 actor: with +0xf2c set and k at most 0x1f, sound k (+0xeac + 4 k) plays (MsSEP3Dregist(3, x, se, 0, 0), 0x007a0a30) unless it is 0 or the last one (0x0112c8e0, then updated); 0 | - | int `0x007a6bc0` (native_mods) |
| `0x706d` | `Native_706d` | b int, a int | b; the argument words = a, b | - | start `0x007a6d00` (native_btl2), poll `0x007a6db0` and int `0x007a6ea0` (not ported) |
| `0x706e` | `Native_706e` | selector int, voice int | the args word: 1 when MsSetVoice(i, voice, 1, worker, &out) (0x007a0720) set out for any picked actor | until 0x007a8700(args) is done | start `0x007a7020`, poll `0x007a71f0` (native_btl) |
| `0x706f` | `Native_706f` | as 0x007a8fc0(worker, args, stack, 1) | its result | - | int `0x007a36c0` (native_btl) |
| `0x7070` | `Native_7070` | - | int: SetByte_00c58ef4(1) (0x0089bc60); 0 | - | int `0x007a7010` (native_mods) |
| `0x7071` | `Native_7071` | - | 0: 0x0089bc60(0) | - | int `0x007a70b0` (native_btl) |
| `0x7072` | `Native_7072` | win, k | int: TODrawAMES_EVJIMAKU(win, text k) (0x0089b230) from *0x0112a9b8; 0 | - | int `0x007a6330` (native_mods) |
| `0x7073` | `Native_7073` | weapon int | int: MsAttachWeapon(actor, weapon)'s result | - | int `0x007a85f0` (native_btl2) |
| `0x7074` | `Native_7074` | v | int: MsDetachWeapon(actor, v) (0x00793f10) | - | int `0x007a29f0` (native_mods) |
| `0x7075` | `Native_7075` | selector int, b int, a int | the argument word = 1 when 0x007ab360(record +0xc, b, a, 0, 1, worker, &out) set out for any selected actor below 31 with a command record (0x00795bb0), else 0 | until 0x007a8700(args) (poll) | start `0x007a78f0`, poll `0x007a7aa0` (native_btl2) |
| `0x7076` | `Native_7076` | c int, f f, a int | 0: MsSetBallSplineInit(actor), the actor's +0x45a = c and +0x452 = 0, MsSetBallSpline(actor), MsMoveSpline(2, actor, c, f, a), +0x452 = 1 | - | int `0x007a56f0` (left to the original: its callees copy unwritten locals) |
| `0x7077` | `Native_7077` | sel, a | int: MsGetTargetDist(x, a, 0) (0x007a9a40, 0.0 when the first actor x of sel is 0xff) over the double at 0x00b49968 plus the one at 0x00b43d80, rounded to f32, at most the f32 at 0x00b4ae10, truncated | - | int `0x007a5450` (native_mods) |
| `0x7078` | `Native_7078` | selector int, b int, a int | int: MsGetComInfo(picked, b, a) (0x0078cd30) | - | int `0x007a6540` (native_btl) |
| `0x7079` | `Native_7079` | - | int: for an actor, the equipment refresh: MsResetAutoStatus(record), MsSetRamChrParam(actor), MsSetChrWeak(actor, -1), MsSetAutoStatus(record), MsStatusEffectCheck(actor), MsSetWeaponEffectAttribute(actor, record); 0 | - | int `0x007a7200` (native_mods) |
| `0x707a` | `Native_707a` | selector int | int: 0x0079a920's result | - | int `0x007a2a80` (native_btl) |
| `0x707b` | `Native_707b` | selector int, k int | int: 0x0079fd40(picked, k) | - | int `0x007a7470` (native_btl) |
| `0x707c` | `Native_707c_Poll` | - | 5 when done | until the actor's (or its +0x14 owner's, when +0x11 is clear) +0xf2c is set or +0xf2e is clear; 5 without an actor | poll `0x007a7260` (native_btl2) |
| `0x707d` | `Native_707d` | k int: battle flag, v int | int: v | - | int `0x007a81f0` (native_btl) |
| `0x707e` | `Native_707e` | k int: battle flag + 1 | int: the flag (0 for k without one) | - | int `0x007a27e0` (native_btl) |
| `0x707f` | `Native_707f` | selector int | 0 | - | int `0x007a5e50` (native_btl) |
| `0x7080` | `Native_7080` | v | int: 0x008720e0(handle, v) with MsGetChrHandle(2) (0x00795080) when there is one; 0 | - | int `0x007a2c70` (native_mods) |
| `0x7081` | `Native_7081_Poll` | - | done (5) when the byte 0x0112c022 is 1, else 0 | - | poll `0x007a5d60` (native_mods) |
| `0x7082` | `Native_7082` | b int, a int | int: 0x0079eb10(actor, b, 1, a, 1) | - | int `0x007a41e0` (native_btl) |
| `0x7083` | `Native_7083` | n int, _ int | 0: for n > 0 with +0xe15 set and no +0xea8, op_et_weapon_unit(model, 0, ...) (0x007fc9b0) into +0xea8 | - | int `0x007a42a0` (native_btl) |
| `0x7084` | `Native_7084` | - | 0 | - | int `0x007a4440` (native_btl) |
| `0x7085` | `Native_7085` | - | 0 (as 0x700c, record +0x418 = 1) | - | int `0x007a43a0` (native_btl) |
| `0x7086` | `Native_7086` | - | 0 | - | int `0x007a2ce0` (native_btl) |
| `0x7087` | `Native_7087` | selector int, b int, a int | int: 0x0079fcf0(picked, b, a); 0 for none | - | int `0x007a7880` (native_btl) |
| `0x7088` | `Native_7088` | selector int, se int | int: MsRegSEplay(picked, se) (0x007a0100); 0 for none | - | int `0x007a79c0` (native_btl) |
| `0x7089` | `Native_7089` | sel, se, frames | int: MsRegSEfade(first of sel, se, frames) (0x007a00d0), 0 for none | - | int `0x007a7c40` (native_mods) |
| `0x708a` | `Native_708a` | v int | 0: 0x00782b40(v) | - | int `0x007a3710` (native_btl) |
| `0x708b` | `Native_708b` | v int | int: 0x00782940(v) | - | int `0x007a3900` (native_btl) |
| `0x708c` | `Native_708c` | a int, b int | int: 0x00782ba0(a, b) | - | int `0x007a3920` (native_btl) |
| `0x708d` | `Native_BtlGetTurnActor` | - | int: the i8 at 0x0112c9e2 | - | int `0x007a76e0` (native_mods) |
| `0x708e` | `Native_708e` | - | int: the actor's signed +0x508 after MsMotionPosCheck (0x007aa100; +0x4f0 = 10.0, +0x34 = 1 around it when +0x608 and +0x505 are set) | - | int `0x007a5c20` (native_btl) |
| `0x708f` | `Native_708f` | v int | int: 0x007a8720(actor, v, 1) | - | int `0x007a6700` (native_btl) |
| `0x7090` | `Native_7090` | v | int: MsEtCriticalEffect(actor, v) (0x0079e590); 0 | - | int `0x007a49c0` (native_mods) |
| `0x7091` | `Native_7091` | to int, from int (selectors) | 0: the 40-byte name +0x540 and +0xdd5 copied from one picked actor to the other | - | int `0x007a77c0` (native_btl) |
| `0x7092` | `Native_7092` | selector int | int: (0x00794fb0(index) - the second float of 0x007956e0(index, &v)) rounded to f32, times 100, truncated | - | int `0x007a5f10` (native_btl2) |
| `0x7093` | `Native_7093` | - | int: 0x00791780(actor, record) | - | int `0x007a60f0` (native_btl) |
| `0x7094` | `Native_7094` | b f, a f, v | int: MsSetChrTransparentVisible(actor, b, a, v, record +0x423) (0x00796550) for an actor; 0 | - | int `0x007a72c0` (native_mods) |
| `0x7095` | `Native_7095` | - | int: MsGetChrMoveFrameRest(actor) (0x007a9320), 0 without an actor | - | int `0x007a61d0` (native_mods) |
| `0x7096` | `Native_7096` | - | int: the signed byte 0x0112c9fa | - | int `0x007a7b30` (native_btl) |
| `0x7097` | `Native_7097` | v int | the handle; poll 0x007a8700 | until it ends | start `0x007a5780`, poll `0x007a58c0` (native_btl) |
| `0x7098` | `Native_7098` | - | int: MsCheckDefenseMotion(MsGetChr(actor)) (0x007af2f0), 0 without an actor | - | int `0x007a2dc0` (native_mods) |
| `0x7099` | `Native_BtlSetPadOrientation` | v | int: the byte 0x0112c9cd = v & 3; that value | - | int `0x007a7340` (native_mods) |
| `0x709a` | `Native_709a` | - | int: 0 when the actor is the one in the signed byte 0x0112c9e4, else 1 | - | int `0x007a2eb0` (native_btl2) |
| `0x709b` | `Native_709b` | selector int | float: the second float of the vector 0x007956e0(selected actor, &v) writes | - | float `0x007a3170` (native_btl2) |
| `0x709c` | `Native_709c` | a int, b int (selectors) | float: as 0x70c2 with 0x007c4990's fourth result | - | float `0x007a3330` (native_btl2) |
| `0x709d` | `Native_709d` | - | 5 when there is no actor or its +0x425 is clear, else 0 | until +0x425 is clear | poll `0x007a3ae0` (native_btl) |
| `0x709e` | `Native_709e` | selector int, a int, b int | int: 0x007b74d0(picked, a, b) | - | int `0x007a2fe0` (native_btl) |
| `0x709f` | `Native_709f` | selector int | float: the selected actor's target direction (MsGetChrTargetDir 0x007958c0) minus its direction (MsGetChrDir 0x00794d20), in degrees | - | float `0x007a3460` (native_btl2) |
| `0x70a0` | `Native_70a0` | v int | int: the old signed byte 0x0112c9fc; it becomes v & 1 | - | int `0x007a73d0` (native_btl2) |
| `0x70a1` | `Native_ActorGetSlotRef` | selector int | int: the picked actor's +0xc, -6 - slot, or 0xff | - | int `0x007a6b40` (native_btl) |
| `0x70a2` | `Native_70a2` | v int | the start's result (poll not ported) | its own poll | start `0x007a7440` (native_btl), poll `0x007a7500` (not ported) |
| `0x70a3` | `Native_70a3` | - | int: Battle_ScriptPlayMotion(worker, args, stack, 0) (0x007a8fc0) with the actor's +0x38 byte set to 1 for the call and put back; +0x3f3 = 1 when +0x434 is set; the old byte (sign-extended) | - | int `0x007a3730` (native_mods) |
| `0x70a4` | `Native_70a4` | - | 0: 0x0079cca0() | - | int `0x007a7630` (native_btl) |
| `0x70a5` | `Native_70a5` | x f, y f, z f | 0 | - | int `0x007a76f0` (native_btl) |
| `0x70a6` | `Native_70a6` | v int | int: v; the byte 0x0112c9ff = v | - | int `0x007a77a0` (native_btl2) |
| `0x70a7` | `Native_70a7` | sel, a | float: 0x007b2c90(first of sel, a)'s ST0 | - | float `0x007a54f0` (native_mods) |
| `0x70a8` | `Native_70a8` | selector int, property int, value f | 0 | - | int `0x007a57e0` (native_btl) |
| `0x70a9` | `Native_70a9_Poll` | - | 5 when done | until 0x008b7ca0(handle) is 0, or 0x008b8530(handle) is 1, or 0x008864b0() is 0 with state 3 (then 0x008ab8f0(handle)) | start `0x007a6c50` (not ported), poll `0x007a6cb0` (native_btl2) |
| `0x70aa` | `Native_GetSelfProperty` | property int | int: the actor's own property | - | int `0x007a4ea0` (native_btl) |
| `0x70ab` | `Native_SetActorProperty` | property int, value int | int: 0x007b4b70(actor, property, -13, value) | - | int `0x007a5360` (native_move) |
| `0x70ac` | `Native_70ac` | i int | float: actor float property i | - | float `0x007a5650` (native_btl) |
| `0x70ad` | `Native_70ad` | - | int: jumps to 0x007ad6e0 | - | int `0x007a2cd0` (native_mods) |
| `0x70ae` | `Native_70ae` | - | int: the i8 at 0x0112c9e3 | - | int `0x007a7790` (native_mods) |
| `0x70af` | `Native_70af` | - | int: jumps to MsGetVoiceEventFlag (0x007986a0) | - | int `0x007a79a0` (native_mods) |
| `0x70b0` | `Native_70b0` | selector int | int: the steps the picked actor needs (from +0x40e, or the +0x4cc/+0x4d0 ramp over the distance) | - | int `0x007a4f80` (native_btl) |
| `0x70b1` | `Native_70b1` | v int | debug (mod-tool candidate): dbgPrintf("##### %d : %x : c%02d #####", v, v, actor) (a bare ret in this build); 0 | - | int `0x007a4a60` (not ported) |
| `0x70b2` | `Native_SetSelfFloatProperty` | property int, value f | 0 (MsSetChrMotionInfo) | - | int `0x007a5930` (native_btl) |
| `0x70b3` | `Native_70b3` | v int | int: 0x007b2b60(v) | - | int `0x007a7df0` (native_btl2) |
| `0x70b4` | `Native_70b4` | - | int: for an actor, +0xf65 = 8, MsResetWeaponEffectAttribute(actor, record) (0x0079ea90), then 0x007ad5b0(actor, +0xf64, +0xf65); 0 | - | int `0x007a7130` (native_mods) |
| `0x70b5` | `Native_70b5` | - | 0 | - | int `0x007a2f40` (native_btl) |
| `0x70b6` | `Native_70b6` | sel | int: MsSetMove(first of sel, 0xc) over 0x007a88e0(the actor's record) (both ST0, the divisor rounded to f32), rounded to f32, truncated | - | int `0x007a52b0` (native_mods) |
| `0x70b7` | `Native_70b7` | win, k | int: TODrawAMES_EVJIMAKU_ALBHED(win, text k) (0x0089b290) from *0x0112a9b8; 0 | - | int `0x007a6640` (native_mods) |
| `0x70b8` | `Native_70b8` | v int | int: v; the byte 0x0112ca00 = v | - | int `0x007a7860` (native_btl2) |
| `0x70b9` | `Native_70b9` | selector int | float: what 0x00795170(0xb, MsGetChrBit(actor, selector, 0, 1), 0, &out) writes | - | float `0x007a2ee0` (native_btl2) |
| `0x70ba` | `Native_70ba` | selector int | float: what 0x00795170(0xa, MsGetChrBit(actor, selector, 0, 1), 0, &out) writes | - | float `0x007a3040` (native_btl2) |
| `0x70bb` | `Native_70bb` | v int | 0: the actor's +0x415 = 0x10, +0x417 = 0, +0x452 = 0, +0x45a = v (u16) | - | int `0x007a4cf0` (native_btl) |
| `0x70bc` | `Native_70bc` | selector int | the selected actor's index (kept in the argument word) | until that actor's +0xe12 is clear (poll 5) | start `0x007a5990`, poll `0x007a5b90` (native_btl2) |
| `0x70bd` | `Native_70bd` | selector int | the selected actor's index (kept in the argument word) | until that actor's +0xe13 is clear (poll 5) | start `0x007a5ed0`, poll `0x007a6030` (native_btl2) |
| `0x70be` | `Native_70be` | selector int | float: the picked actor's direction (MsGetChrDir 0x00794d20) in degrees (MsNearRag(0, d) * 180 / pi) | - | float `0x007a3780` (native_btl) |
| `0x70bf` | `Native_70bf` | v f | 0; the actor's f32 +0x3ac = v and, with a model, 0x00832610(model, v) | - | int `0x007a6ae0` (native_btl2) |
| `0x70c0` | `Native_70c0` | sel | float: 0x00794990(first of sel)'s ST0 | - | float `0x007a6c70` (native_mods) |
| `0x70c1` | `Native_70c1` | b int, a int (selectors) | int: the distance between the two actors' positions (0x00795170 mode 1, 0x007c3030), truncated | - | int `0x007a4b90` (native_btl2) |
| `0x70c2` | `Native_70c2` | a int, b int (selectors) | float: the angle from a's position to b's (0x007c4990's third result; positions from 0x00795170 mode 5) in degrees | - | float `0x007a31d0` (native_btl2) |
| `0x70c3` | `Native_70c3` | selector int | float: MsGetChrTargetDir(picked, 0) (0x007958c0) in degrees, as 0x70be | - | float `0x007a3650` (native_btl) |
| `0x70c4` | `Native_70c4` | selector int | int: 1 when one of the selected actor's two weapon slots (+0x592) holds a weapon with bit 3 of its +3 (MsGetSaveWeapon 0x007abbd0), else 0 | - | int `0x007a2ba0` (native_btl2) |
| `0x70c5` | `Native_70c5` | - | int: 1 when the byte 0x0112c9f5 is set | - | int `0x007a7c20` (native_mods) |
| `0x70c6` | `Native_70c6` | v | int: 0x007b7ed0(v) | - | int `0x007a3950` (native_mods) |
| `0x70c7` | `Native_70c7` | v | float: 0x007b7ef0(v)'s ST0 | - | float `0x007a39f0` (native_mods) |
| `0x70c8` | `Native_70c8` | sel, a | int: 0x0079fda0(first of sel, a), 0 for none | - | int `0x007a75e0` (native_mods) |
| `0x70c9` | `Native_70c9` | selector int, k int | int: 0x0079fdf0(picked, k) | - | int `0x007a7740` (native_btl) |
| `0x70ca` | `Native_70ca` | sel, se | int: MsRegSEplay2(first of sel, se) (0x007a0140), 0 for none | - | int `0x007a7ab0` (native_mods) |
| `0x70cb` | `Native_70cb` | sel, se | int: MsRegSEplay3(first of sel, se) (0x007a0170), 0 for none | - | int `0x007a7b40` (native_mods) |
| `0x70cc` | `Native_70cc` | selector int | int: the set (actor +0xdc0 = it) | - | int `0x007a7cb0` (native_btl) |
| `0x70cd` | `Native_70cd` | selector int | int: the set (actor +0xdc0 |= it) | - | int `0x007a7d60` (native_btl) |
| `0x70ce` | `Native_70ce` | selector int | int: the set (cleared from actor +0xdc0) | - | int `0x007a7f20` (native_btl) |
| `0x70cf` | `Native_70cf` | - | int: the i8 at *0x0112a9c0 + 1 | - | int `0x007a7f70` (native_mods) |
| `0x70d0` | `Native_70d0` | v int | 0; the byte 0x0112c9de = v & 1 | - | int `0x007a4380` (native_btl2) |
| `0x70d1` | `Native_70d1` | selector int | int: the picked actor's +0xe12 | - | int `0x007a5cb0` (native_btl) |
| `0x70d2` | `Native_70d2` | selector int | int: the selected actor's byte +0xe13 | - | int `0x007a6130` (native_btl2) |
| `0x70d3` | `Native_70d3` | sel, b, a | int: MsSetChrEffect_0079eb10(x, b, 7, a, 1) for the first actor x of sel, whose +0xe12 counts up when it starts; 0 | - | int `0x007a4a90` (native_mods) |
| `0x70d4` | `Native_70d4` | sel, b, a | int: 0x0079eba0(first of sel, b, a, 1) | - | int `0x007a4f20` (native_mods) |
| `0x70d5` | `Native_70d5` | selector int | int: the picked index (actor +0x6d1) | - | int `0x007a7fa0` (native_btl) |
| `0x70d6` | `Native_70d6` | c int, b int, a int | int: 0x00789bf0(chr i, chr j, r, r +0x28, r +0x2a, 0, r +0x23, 0, 0, 0, 0) with i, j the actors selected by c, b and r = 0x0078ce50(i, 0, -1, 0, &a) | - | int `0x007a80b0` (native_btl2) |
| `0x70d7` | `Native_70d7` | selector int, motion int | 0: MsDamageSetMotion(picked, motion, picked != actor) (0x0078ca20) unless the picked actor's +0x422 actor has +0xdcc | - | int `0x007a30f0` (native_btl) |
| `0x70d8` | `Native_70d8` | v int | int: v; the byte 0x0112ca06 = v & 1 | - | int `0x007a3400` (native_btl2) |
| `0x70d9` | `Native_70d9` | - | int: Ch_ModelSetHide(model, 1, the i8 at 0x0112ca06) (0x0082aea0) for actor 13's model (MsGetChr(0xd) +0) when it has one; that byte, read again | - | int `0x007a3500` (native_mods) |
| `0x70da` | `Native_70da` | - | int: the i8 at 0x0112c9dc | - | int `0x007a4df0` (native_mods) |
| `0x70db` | `Native_70db` | - | int: the i8 at 0x0112ca06 | - | int `0x007a3450` (native_mods) |
| `0x70dc` | `Native_70dc` | selector int, i int | 0: 40 bytes of the name table *0x0112a9b8 (entry MsGetSaveConfigHiragana() + 2i) into the picked actor's +0x540 | - | int `0x007a7a10` (native_btl) |
| `0x70dd` | `Native_70dd` | v int | int: v; the word 0x0112c8fc = v | - | int `0x007a7f80` (native_btl2) |
| `0x70de` | `Native_70de` | - | int: jumps to 0x00785210 | - | int `0x007a79b0` (native_mods) |
| `0x70df` | `Native_70df` | v int | int: 1 when the byte 0x0112a8e0 is set and the i16 at *0x0112a9f0 is v >> 16, else 0 | - | int `0x007a4ee0` (native_btl2) |
| `0x70e0` | `Native_70e0` | - | int: 1 when the actor and its partner can act | - | int `0x007a8450` (native_btl) |
| `0x70e1` | `Native_70e1` | - | int: 1 when the actor named by the actor's signed byte +0xded has +0xdf1 clear, else 0 (0 without an actor) | - | int `0x007a85a0` (native_btl2) |
| `0x70e2` | `Native_70e2` | selector int, v int | 0 | - | int `0x007a32a0` (native_btl) |
| `0x70e3` | `Native_70e3` | v int | 0; once (word 0x0112be98 is 0): 0x0112be9c = v, 0x0083c4a0(0x40), 0x0112be98 = 0x00687020(v), 0x00799f00(it, v) | - | int `0x007a7da0` (native_btl2) |
| `0x70e4` | `Native_70e4` | b, a | int: MsGetDamageLimit(first of b, first of a) (0x0079a860, each through Battle_FirstSelectedActor) | - | int `0x007a2b50` (native_mods) |
| `0x70e5` | `Native_70e5` | sel, b, a | int: 0x0079eca0(first of sel, b, a); 0 | - | int `0x007a4c70` (native_mods) |
| `0x70e6` | `Native_70e6` | sel, b, a | int: 0x0079ec40(first of sel, b, a); 0 | - | int `0x007a4e00` (native_mods) |
| `0x70e7` | `Native_70e7` | command int | int: the actor's +0xdc0 = 0x007996d0(actor, command, bit 1 of record +0x1a clear, 1), record = 0x0078ce50(actor, 0, -1, 0, &command) | - | int `0x007a8680` (native_btl2) |
| `0x70e8` | `Native_70e8` | - | int: jumps to MsGetGIL (0x00784e80) | - | int `0x007a7a90` (native_mods) |
| `0x70e9` | `Native_70e9` | - | int: jumps to 0x007b2b30 | - | int `0x007a7b00` (native_mods) |
| `0x70ea` | `Native_70ea` | v int | int: v (MsSet01130838(v), 0x007b2b40) | - | int `0x007a7b10` (native_btl) |
| `0x70eb` | `Native_70eb` | - | int: brnd(0x11) (0x007988f0) | - | int `0x007a7d10` (native_mods) |
| `0x70ec` | `Native_01b2_copy_007a2c50` | v int | int: 0x007904b0(v), as Native_01b2 | - | int `0x007a2c50` (native_btl2) |
| `0x70ed` | `Native_70ed` | b int, a int | int: MsSaveItemUse(b, a) (0x00790550) | - | int `0x007a2ca0` (native_btl) |
| `0x70ee` | `Native_70ee` | b int, a int | int: by a + b: < 0x20 0x30de, < 0x30 0x30df, < 0x3f 0x30e0, < 0x50 0x30e1, else 0x30e2 | - | int `0x007a7b90` (native_btl2) |
| `0x70ef` | `Native_70ef` | selector int, signal int | 0: MsEffectSendSignal(picked, signal) (0x0079e4f0) | - | int `0x007a64f0` (native_btl) |
| `0x70f0` | `Atel_GetBattleCountF` | - | float: the i32 at 0x0112c900 as f32 (helpers::atel_get_battle_count_f, whose shim it is) | - | float `0x007a3ac0` (native_mods) |
| `0x70f1` | `Native_70f1` | - | 0: the actor's 20 item slots in g_BtlItemMem2 (+id*0x478) freed (0xff) | - | int `0x007a2d30` (native_btl2) |
| `0x70f2` | `Native_70f2` | item int | 0: the item put in the actor's first free slot of 20 (g_BtlItemMem2) | - | int `0x007a2e60` (native_btl2) |
| `0x70f3` | `Native_70f3` | b, a | int: 0x007a8ad0(b, a) | - | int `0x007a3300` (native_mods) |
| `0x70f4` | `Native_70f4` | b, a | int: 0x007a8b40(actor, b, a) | - | int `0x007a2ad0` (native_mods) |
| `0x70f5` | `Native_70f5` | command int, selector int | int: 0x0078c690(selected actor, 0x0079a4b0(command, 0), 0) | - | int `0x007a2f80` (native_btl2) |
| `0x70f6` | `Native_70f6` | - | int: the bytes 0x011363ec and 0x011363ed = 0 (the list 0x70f8 reads, emptied); 0 | - | int `0x007a34e0` (native_mods) |
| `0x70f7` | `Native_70f7` | v int | int: v; appended to the 16 u16 at 0x011363f0 (count: the byte 0x011363ec) while there is room | - | int `0x007a3560` (native_btl2) |
| `0x70f8` | `Native_70f8` | - | int: the next u16 of the list at 0x011363f0 (read index 0x011363ed, count 0x011363ec, the index advanced), 0 past the end | - | int `0x007a36e0` (native_mods) |
| `0x70f9` | `Native_70f9` | p5, p4, p3, p2, p1 | int: 0x007a8f60(actor, p5, p4, p3, p2, p1) | - | int `0x007a5fc0` (native_mods) |
| `0x70fa` | `Native_70fa` | v int | 0; the u16 0x01136410 = v | - | int `0x007a3420` (native_btl2) |
| `0x70fb` | `Native_70fb` | p5, p4, p3, p2, p1 | int: 0x007a8f60(actor, p5, -16, p3, 0, p2) when 0x007a8b40(actor, p5, p4) and 0x007a8ad0(p5, p1) both hold, else (or for 0xff) 0xff | - | int `0x007a37e0` (native_mods) |
| `0x70fc` | `Native_70fc` | - | int: the u16 at 0x01136410 | - | int `0x007a3440` (native_mods) |
| `0x70fd` | `Native_70fd` | b int, a int | int: the actor index; unless the u16 0x01136410 is set, the actor's +0x714 = 1, +0x726/+0x727 = a, +0x728/+0x729 = b | - | int `0x007a3a10` (native_btl2) |
| `0x70fe` | `Native_70fe` | e int, d int, b int, a int | 0; the actor's +0x714 = 1, +0x726 = a, +0x728 = b, +0x727 = d, +0x729 = e | - | int `0x007a3b80` (native_btl2) |
| `0x70ff` | `Native_70ff` | - | int: the actor's +0xdc0, also stored in the word 0x0112c8d0 | - | int `0x007a3d00` (native_btl2) |
| `0x7100` | `Native_7100` | sel | int: 0x007a8be0(MsGetChrBit(actor, sel, 0, 1)) (0x00794330) | - | int `0x007a7d20` (native_mods) |
| `0x7101` | `Native_7101` | - | int: the i8 at 0x0112a905 | - | int `0x007a7f60` (native_mods) |
| `0x7102` | `Native_7102` | a int, b int (selectors) | int: actor a's index (0xff for none), stored in actor b's +0x438 (+0x439 = 1) | - | int `0x007a35d0` (native_btl2) |
| `0x7103` | `Native_7103` | selector int, x f, y f, z f | 0: the selected actor's model (0x00795080) gets mode 3 and the vector at +0x4c0 (0x008350a0) | - | int `0x007a3880` (native_btl2) |
| `0x7104` | `Native_7104` | a int, b int, c int | int: 0x0079d2a0(a, b, c) | - | int `0x007a6600` (native_btl) |
| `0x7105` | `Native_7105_Poll` | - | 5 when done | until the actor's +0xe17 is set | poll `0x007a6730` (native_btl2) |
| `0x7106` | `CT_7106_ret_doesChrKnowCommand` | selector int, command int | int: MsGetSaveCommand(picked, command) (0x00785020) | - | int `0x007a30a0` (native_btl) |
| `0x7107` | `Native_7107` | n int | int: ftol((MsDirTurn(chr, yaw +0x514[+0x511]) - +0x490) / pi * n) (0x007a9650; f32 steps) | - | int `0x007a6800` (native_btl) |
| `0x7108` | `Native_7108` | a int, b int | 0 | - | int `0x007a40a0` (native_btl) |
| `0x7109` | `Native_7109` | selector int, reset int, f f | int: the number of picked actors; each with a model: +0x407 = 1, with reset 0x00796070(i, 0), +0x505 = 1, +0x50c = 0; +0x43d = 0, +0x538 = f | - | int `0x007a3df0` (native_btl) |
| `0x710a` | `Native_710a` | - | int: the i8 at 0x0112ca10, which becomes 1 | - | int `0x007a3970` (native_mods) |
| `0x710b` | `Native_710b_Poll` | - | 5 when done | until the byte 0x0112ca10 is set | poll `0x007a3a80` (native_btl2) |
| `0x710c` | `Native_710c` | - | int: the signed byte 0x0112ca0d, first counted down by 1 while above 0 (only with an actor) | - | int `0x007a2c10` (native_btl2) |
| `0x710d` | `Native_710d` | - | 0; the actor's +0x411 = 0 | - | int `0x007a3c20` (native_btl2) |
| `0x710e` | `Native_710e` | - | int: the first set byte +0 of the four 0x007b19d0(k) records, 0 for none | - | int `0x007a3c00` (native_mods) |
| `0x710f` | `Native_710f` | - | int: that record's byte +1, 0 for none | - | int `0x007a3c90` (native_mods) |
| `0x7110` | `Native_7110` | selector int (5 is 0 while the word 0x011363bc is set) | int: the selected actor's index; its model's +0x4d4 = 1 (0x00834770) | - | int `0x007a3d30` (native_btl2) |
| `0x7111` | `Native_7111` | selector int (5 is 0 while the word 0x011363bc is set) | int: the selected actor's index; its model's +0x4d4 = 0 | - | int `0x007a3ff0` (native_btl2) |
| `0x7112` | `Native_7112` | c int, _ f (dropped), b int, a int | 0; 0x00795eb0(actor, c, 1, -1, b, a) | - | int `0x007a43d0` (native_btl2) |
| `0x7113` | `Native_7113` | - | int: the byte 0x0112c9e5 = 1; 0 | - | int `0x007a8090` (native_mods) |
| `0x7114` | `Native_7114` | - | int: the byte 0x0112c9e5 = 2; 0 | - | int `0x007a80a0` (native_mods) |
| `0x7115` | `Native_7115` | - | 0: MsSetChrDirLeave(actor) (0x00795e70) | - | int `0x007a4560` (native_btl) |
| `0x7116` | `Native_7116` | - | int: MsEffectSetGuard(actor) (0x0079e510) | - | int `0x007a6930` (native_mods) |
| `0x7117` | `Native_7117` | targets int, command int | 1: Battle_QueueCommand(actor, command, MsGetChrBit(actor, targets, 0, 1, 1, 0x40)) after MsMotionPosCheck when +0x3f9 is set; 0 without an actor | - | int `0x007a4b00` (native_btl) |
| `0x7118` | `Native_7118` | v int | int: v; the byte 0x0112ca11 = v | - | int `0x007a81d0` (native_btl) |
| `0x7119` | `Native_7119` | v int | int: v; the byte 0x0112ca12 = v | - | int `0x007a8430` (native_btl2) |
| `0x711a` | `Native_711a` | selector int, a f, b f, c f | 0; the selected actor's model (0x00795080) gets 0x00824c90(model, c, b, a) | - | int `0x007a4160` (native_btl2) |
| `0x711b` | `Native_711b` | v int | int: v & 1, stored in the byte 0x0112ca13 | - | int `0x007a3dd0` (native_btl2) |
| `0x711c` | `Native_711c` | v int | int: v; the u16 0x0112c94e = v | - | int `0x007a8530` (native_btl) |
| `0x711d` | `Native_711d` | height f | 0: Ch_ShadowSetHeight(MsGetChrHandle(actor), height) for a model (0x00795080, 0x00830cb0) | - | int `0x007a4330` (native_btl) |
| `0x711e` | `Native_711e` | - | int: the i8 at 0x0112ca15 | - | int `0x007a3f40` (native_mods) |
| `0x711f` | `Native_711f` | - | int: as 0x7042 from the table at *0x0112a998, only when MsGetBattleMessageFlag() (0x00784e40) is set and the word at 0x01841d20 is not 0; 0 | - | int `0x007a6200` (native_mods) |
| `0x7120` | `Native_7120` | b, a | int: 0x0079d090(a), b dropped; 0 | - | int `0x007a65a0` (native_mods) |
| `0x7121` | `Native_7121` | win, k | int: as 0x70b7 from *0x0112a998 with 0x711f's checks; 0 | - | int `0x007a6790` (native_mods) |
| `0x7122` | `Native_7122` | selector int | int: the selected actor's index; its model's +0x4d4 = 1 | - | int `0x007a3ef0` (native_btl2) |
| `0x7123` | `Native_7123` | weapon int | int: MsAttachWeapon(5, weapon)'s result | - | int `0x007a2980` (native_btl2) |
| `0x7124` | `Native_7124` | id | int: 0x0112c250 = id; MsSetVoiceId(id) (0x007b2b50) when MsGetBattleMessageFlag(); id | - | int `0x007a7ee0` (native_mods) |
| `0x7125` | `Native_7125` | - | 0; MsAttachWeapon(4, 0) when the worker's actor is 4 | - | int `0x007a8650` (native_btl2) |
| `0x7126` | `Native_7126` | i int | int: i (0xff for a negative i); actor i's +0x3e = 1 and its two parts' (0x00795b10(i, k)) +0x3e = 1 | - | int `0x007a8550` (native_btl2) |
| `0x7127` | `Native_7127` | b, a | int: 0x007969a0(b, a) | - | int `0x007a5240` (native_mods) |
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
| `0x800e` | `Native_800e` | - | int: jumps to Yn_MapPartCount (0x0091a3d0) | - | int `0x0091b190` (native_mods) |
| `0x800f` | `Native_MapPartShow` | part int: map part | 1 | - | int `0x0091b1a0` (native_map) |
| `0x8010` | `Native_MapPartHide` | part int | 1 | - | int `0x0091b1e0` (native_map) |
| `0x8011` | `Native_8011` | a int, b int, c int | 1 | - | int `0x0091b200` (native_map) |
| `0x8012` | `Native_8012` | v int | int: 0x0091a440(v) | - | int `0x0091b240` (native_sys) |
| `0x8013` | `Native_8013` | v int | int: 0x0091a490(v) | - | int `0x0091b260` (native_sys) |
| `0x8014` | `Native_8014` | part int, v f: map part +0x14 (Yn_MapPartSet14) | 1 | - | int `0x0091b280` (native_map) |
| `0x8015` | `Native_8015` | v int | float: 0x0091a670(v) | - | float `0x0091b2c0` (native_sys) |
| `0x8016` | `Native_8016` | a int, b int, c int | 1; 0x0091ac40(a, b, c) | - | int `0x0091b2e0` (native_sys) |
| `0x8017` | `Native_8017` | v int | int: 0x0091a580(v) | - | int `0x0091b320` (native_sys) |
| `0x8018` | `Native_8018` | v int | int: 0x0091a5d0(v) | - | int `0x0091b340` (native_sys) |
| `0x8019` | `Native_8019` | a int, b int, c int | 1; 0x0091ac80(a, b, c) | - | int `0x0091b360` (native_sys) |
| `0x801a` | `Native_801a` | v int | int: 0x0091a620(v) | - | int `0x0091b3a0` (native_sys) |
| `0x801b` | `Native_801b` | v int | int: 0x0091a3f0(v) | - | int `0x0091b3c0` (native_sys) |
| `0x801c` | `Native_801c` | v int | int: 0x0091a2a0(v) | - | int `0x0091b3e0` (native_sys) |
| `0x801d` | `Native_801d` | part int, frame int: map part animation frame | 1 | - | int `0x0091b400` (native_map) |
| `0x801e` | `Native_801e` | part int: animation flag 0 on | 1 | - | int `0x0091b430` (native_map) |
| `0x801f` | `Native_801f` | part int: animation flag 0 off | 1 | - | int `0x0091b450` (native_map) |
| `0x8020` | `Native_8020` | part int: animation on | 1 | - | int `0x0091b470` (native_map) |
| `0x8021` | `Native_8021` | part int: animation off | 1 | - | int `0x0091b490` (native_map) |
| `0x8022` | `Native_8022` | part int, bits int: animation mode (bits 8-11) | 1 | - | int `0x0091b4b0` (native_map) |
| `0x8023` | `Native_8023` | a f, b f | 1; 0x0091ad40(a, b) | - | int `0x0091b4e0` (native_sys) |
| `0x8024` | `Native_8024` | - | float: jumps to Yn_SceneGetB8 (0x0091a700) | - | float `0x0091b530` (native_mods) |
| `0x8025` | `Native_8025` | - | float: jumps to Yn_SceneGetBC (0x0091a730) | - | float `0x0091b540` (native_mods) |
| `0x8026` | `Native_8026` | a int, b f, c int, d int, e int: scene +0xc0..+0xd0 (Yn_SceneSetC0) | 1 | - | int `0x0091b550` (native_map) |
| `0x8027` | `Native_8027` | - | int: 0x0091a900(); 1 | - | int `0x0091b5c0` (native_mods) |
| `0x8028` | `Native_8028` | v f | 1; 0x0091ad10(v) | - | int `0x0091b5d0` (native_sys) |
| `0x8029` | `Native_8029` | - | float: jumps to Yn_SceneGetAC (0x0091a6d0) | - | float `0x0091b600` (native_mods) |
| `0x802a` | `Native_802a` | a int, b f, c int, d f | 1; 0x0091aa00(a, b, c, d) | - | int `0x0091b610` (native_sys) |
| `0x802b` | `Native_802b` | - | int: 0x0091a8d0(); 1 | - | int `0x0091b670` (native_mods) |
| `0x802c` | `Native_802c` | - | 1 | - | int `0x0091b680` (native_map) |
| `0x802d` | `Native_802d` | - | 1 | - | int `0x0091b690` (native_map) |
| `0x802e` | `Native_802e` | part int, rate int: animation rate | 1 | - | int `0x0091b6a0` (native_map) |
| `0x802f` | `Native_802f` | part int: animation to its last frame | 1 | - | int `0x0091b6d0` (native_map) |
| `0x8030` | `Native_8030` | a int, b int | 1 | - | int `0x0091b6f0` (native_map) |
| `0x8031` | `Native_8031` | v int | int: 0x0091a2e0(v) | - | int `0x0091b720` (native_sys) |
| `0x8032` | `Native_8032` | limit f: guide map limit | 1 | - | int `0x0091b740` (native_map) |
| `0x8033` | `Native_8033` | - | float: jumps to Yn_GuideMapGetLimit (0x00919ca0) | - | float `0x0091b770` (native_mods) |
| `0x8034` | `Native_8034` | - | int: 0x00919b70(); 1 | - | int `0x0091b780` (native_mods) |
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
| `0x8046` | `Native_8046` | v int | 1; a record from 0x009086d0 with word 0 = v, passed to 0x0090a040 | - | int `0x0091bf10` (native_sys) |
| `0x8047` | `Native_8047` | v int | 1; as 0x8046 with word 1 | - | int `0x0091bf60` (native_sys) |
| `0x8048` | `Native_8048` | v int | 1; as 0x8046 with word 2 | - | int `0x0091bfb0` (native_sys) |
| `0x8049` | `Native_BackGetRed` | - | int: back red | - | int `0x0091c000` (native_map) |
| `0x804a` | `Native_BackGetGreen` | - | int: back green | - | int `0x0091c030` (native_map) |
| `0x804b` | `Native_BackGetBlue` | - | int: back blue | - | int `0x0091c060` (native_map) |
| `0x804c` | `Native_804c` | part, v f | int: Yn_MapPartSet44_0091abc0(part, v, v); 1 | - | int `0x0091c090` (native_mods) |
| `0x804d` | `Native_804d` | part, v f | int: Yn_MapPartSet44_0091abc0(part, v, 0.0); 1 | - | int `0x0091c0d0` (native_mods) |
| `0x804e` | `Native_804e` | part, v f | int: Yn_MapPartSet44_0091abc0(part, 0.0, v); 1 | - | int `0x0091c110` (native_mods) |
| `0x804f` | `Native_804f` | part int, s f, frames int: map part scaled to (s, s) over frames | 1 | - | int `0x0091c150` (native_map) |
| `0x8050` | `Native_8050` | v int | float: 0x0091a530(v) | - | float `0x0091c1a0` (native_sys) |
| `0x8051` | `Native_8051` | v int | float: 0x0091a4e0(v) | - | float `0x0091c1c0` (native_sys) |
| `0x8052` | `Native_8052` | a int, b f, c f | 1; 0x0091aae0(a, b, c) | - | int `0x0091c1e0` (native_sys) |
| `0x8053` | `Native_8053` | i int, v f | 1; 0x0091aae0(i, v, 0x0091a380(i) rounded to f32) | - | int `0x0091c230` (native_sys) |
| `0x8054` | `Native_8054` | i int, v f | 1; 0x0091aae0(i, 0x0091a330(i, v) rounded to f32, v) | - | int `0x0091c280` (native_sys) |
| `0x8055` | `Native_8055` | a int, b f, c f, d int | 1; 0x0091a0f0(a, b, c, d) | - | int `0x0091c2d0` (native_sys) |
| `0x8056` | `Native_8056` | v int | float: 0x0091a330(v) | - | float `0x0091c330` (native_sys) |
| `0x8057` | `Native_8057` | v int | float: 0x0091a380(v) | - | float `0x0091c350` (native_sys) |
| `0x8058` | `Native_8058` | pc, b, a | int: pppFpSetStickChr(a, AtelGetCharHandleFromBindPC(pc), b) (0x0072a530, 0x0086ae90); 1 | - | int `0x0091c370` (native_mods) |
| `0x8059` | `Native_8059` | a int, b int: pppFpSetStickMap(b, a) | 1 | - | int `0x0091c3c0` (native_map) |
| `0x805a` | `Native_805a` | v int | 1; 0x0072a120(v) | - | int `0x0091c3f0` (native_sys) |
| `0x805b` | `Native_FpSetStickPos` | n int, x f, y f, z f | 1 | - | int `0x0091c410` (native_map) |
| `0x805c` | `Native_805c` | on int | 1 | - | int `0x0091c470` (native_map) |
| `0x805d` | `Native_805d` | - | 1 | - | int `0x0091c4a0` (native_map) |
| `0x805e` | `Native_805e` | map int: guide map number (Yn_GuideMapSetMapNo) | 1 | - | int `0x0091c4d0` (native_map) |
| `0x805f` | `Native_805f` | x int | 1 | - | int `0x0091c4f0` (native_map) |
| `0x8060` | `Native_8060` | x int | 1 | - | int `0x0091c510` (native_map) |
| `0x8061` | `Native_8061` | v int | 1; 0x00908580(v) | - | int `0x0091c580` (native_sys) |
| `0x8062` | `Native_8062` | - | int: 0x00908440(); 1 | - | int `0x0091c5a0` (native_mods) |
| `0x8063` | `Native_8063` | a f, b f, c f, d f | 1; 0x0090a200(a, b, c, d) | - | int `0x0091c600` (native_sys) |
| `0x8064` | `Native_8064` | six floats | 1; 0x0090a230(the six floats) | - | int `0x0091c6b0` (native_sys) |
| `0x8065` | `Native_8065` | v int | int: 0x009086b0(v) | - | int `0x0091c780` (native_sys) |
| `0x8066` | `Native_8066` | fp int: pppFpStopStatus | 1 | - | int `0x0091c7c0` (native_map) |
| `0x8067` | `Native_BltzSetObjModels` | worker a int, worker b int, mode int, unused int (Blitzball) | 1 | - | int `0x0091c810` (native_map) |
| `0x8068` | `Native_8068` | - | int: Yn_FPm173End() (0x00919ab0); 1 | - | int `0x0091c890` (native_mods) |
| `0x8069` | `Native_8069` | b, a | int: pppFpSetStickMapDObj(a, b) (0x0072a5e0); 1 | - | int `0x0091af30` (native_mods) |
| `0x806a` | `Native_806a` | v int: scene +0x104 | 1 | - | int `0x0091af80` (native_map) |
| `0x806b` | `Native_806b` | on int | 1 | - | int `0x0091afc0` (native_map) |
| `0x9000` | `Native_9000` | - | int: an "atel_test_print!" line through the empty dbgPrintf; 0 | - | int `0x00907e60` (native_mods) |
| `0xb000` | `Native_b000` | movie int, flags int | start: args = flags, movie; graphicInitFMVPlayer(movie, 0), 0x006427c0(1), flags 0x2000 pauses at the end; int: 0, movie_start_prog(movie, flags) (0x0076f080) | - (the poll returns 5) | start `0x0076e910`, poll `0x0076e980`, int `0x0076e990` (native_chr) |
| `0xb001` | `Native_b001` | - | int: 0; the movie camera off (movie_have_camera), movie -1 | while graphicIsVideoPlaying() or 0x00641ce0(); then graphicDestroyFmv() | start `0x0076e9b0`, poll `0x0076e9c0`, int `0x0076e9f0` (native_chr) |
| `0xb002` | `Native_b002` | - | debug (mod-tool candidate): a dbgPrintf line only; 0 | - | int `0x0076ea30` (not ported) |
| `0xb003` | `Native_b003` | - | int: graphicGetCurrentFrame() (0x006410d0, a jump) | - | int `0x0076ea40` (native_chr) |
| `0xb004` | `Native_b004` | - | start: graphicSetPauseVideoAtEndFrame(0); poll: 0x00643e50(), the movie camera off, movie -1, g_WaitFramesByTime 0, 1; int: graphicDestroyFmv(), 0 | - | start `0x0076ea50`, poll `0x0076ea60`, int `0x0076eac0` (native_chr) |
| `0xb005` | `Native_b005` | - | int: a "Virtuos Warning: Movie on Windows, PS3 & PS Vita is not impelemented yet!" line naming the PS2 callback through the empty dbgPrintf; 0 (the poll keeps waiting) | - | int `0x0076ead0` (native_mods) |
| `0xb006` | `Native_b006` | - | int: a "Virtuos Warning: Movie on Windows, PS3 & PS Vita is not impelemented yet!" line naming the PS2 callback through the empty dbgPrintf; 0 (the poll keeps waiting) | - | int `0x0076eae0` (native_mods) |
| `0xb007` | `Native_b007` | - | int: a "Virtuos Warning: Movie on Windows, PS3 & PS Vita is not impelemented yet!" line naming the PS2 callback through the empty dbgPrintf; 0 (the poll keeps waiting) | - | int `0x0076eaf0` (native_mods) |
| `0xb008` | `Native_b008` | - | the same warning for movie_start_debug_init; dbgPrintf leaves EAX as it was, which the dispatch ignores (0 here) | - | start `0x0076eb00`, poll `0x0076eb10`, int `0x0076eb20` (native_mods) |
| `0xb009` | `Native_b009` | - | int: 0; g_WaitFramesByTime 1, 0x006427c0(0), 0x0076edc0() | until 0x00641ce0() is 0 and 0x00645dc0() is not | start `0x0076eb30`, poll `0x0076eb40`, int `0x0076eb60` (native_chr) |
| `0xb00a` | `Native_b00a` | movie, flags | int: 0xc024 (0x00878c60, poll), 0xb00a (0x0076ec10, int) and the like: 1; int: 0; start: the movie = movie, graphicInitFMVPlayer(movie, flags & 0x80); when it starts, 0x006427c0(0), g_WaitFramesByTime, the done flag and 0x0112a00c = 1. EAX the last call's; poll: 5 unless 0x0112a00c is set while 0x00641ce0() or not 0x00645dc0(); int: 0 | see the poll | start `0x0076eb80`, poll `0x0076ebe0`, int `0x0076ec10`, int `0x0076ec10`, int `0x0076ec10` (native_chr) |
| `0xb00b` | `Native_b00b` | - | poll: 5 when 0x00645e50() is not 0; int: graphicDestroyFmv() for movie 0x34 (the dbgPrintf left out), then 0xb001's int; start: a bare ret | see the poll | start `0x0076ec20`, poll `0x0076ec30`, int `0x0076ec40` (native_chr) |
| `0xb00c` | `Native_b00c` | - | jumps to 0x00641140 (after a dbgPrintf, left out) | - | int `0x0076ec90` (native_chr) |
| `0xb00d` | `Native_b00d` | - | the movie done flag = 0; 0 | - | int `0x0076ecb0` (native_chr) |
| `0xb00e` | `Native_b00e` | - | int: a "Virtuos Warning: Movie on Windows, PS3 & PS Vita is not impelemented yet!" line naming the PS2 callback through the empty dbgPrintf; 0 (the poll keeps waiting) | - | int `0x0076ecc0` (native_mods) |
| `0xb00f` | `Native_b00f` | - | int: a "Virtuos Warning: Movie on Windows, PS3 & PS Vita is not impelemented yet!" line naming the PS2 callback through the empty dbgPrintf; 0 (the poll keeps waiting) | - | int `0x0076ecd0` (native_mods) |
| `0xc000` | `Native_c000` | - | debug (mod-tool candidate): under TkIsDebug only: the save event block (TkGetSaveEventAddress) written to a PS2 development-host file (AtelPCCreate/AtelPCWrite 0x21ec bytes, name at 0x00c52dc0); 1, else 0 | - | int `0x00878420` (not ported) |
| `0xc001` | `Native_c001` | - | debug (mod-tool candidate): under TkIsDebug only: the save event block read back from that host file (AtelPCOpen/AtelPCRead); 1, else 0 | - | int `0x00878490` (not ported) |
| `0xc002` | `Native_c002` | on | MsSetSaveConfigHiragana(on != 0) (0x00785f40); 0 | - | int `0x008784e0` (native_chr) |
| `0xc003` | `Native_c003` | map | AtelMapLoad(map) (0x0086dca0) | - | int `0x00878510` (native_chr) |
| `0xc004` | `Native_c004` | a int, b int | debug (mod-tool candidate): under TkIsDebug (0 in this build) 0x0083a4f0(b, a); 1 | - | int `0x00878570` (not ported) |
| `0xc005` | `Native_c005` | v int | debug (mod-tool candidate): under TkIsDebug (0 in this build) 0x0083a580(v); 1 | - | int `0x008785b0` (not ported) |
| `0xc006` | `Native_c006` | - | debug (mod-tool candidate): under TkIsDebug only: 0x0083a570(); 1 | - | int `0x008785e0` (not ported) |
| `0xc007` | `Native_c007` | map, index | TkSetStartMapIndex(map, index) (0x0088ebf0); map | - | int `0x00878600` (native_chr) |
| `0xc008` | `Native_c008` | var int | 1 (the variable's address is computed and dropped) | - | int `0x00878390` (native_sys) |
| `0xc009` | `Native_c009` | mode | AtelSetEventMode(mode) (0x00873e40); mode | - | int `0x00878630` (native_chr) |
| `0xc00b` | `Native_c00b` | - | debug (mod-tool candidate): in a debug build yiLoadBattleFieldParticleNo(v); 1 in release | - | int `0x00878690` (not ported) |
| `0xc00c` | `Native_c00c` | - | debug (mod-tool candidate): in a debug build (TkIsDebug) yiLoadFieldParticleNo(v); 1 in release | - | int `0x00878650` (not ported) |
| `0xc00d` | `Native_c00d` | on | MsSetSaveConfigCreditName(on != 0) (0x00785e90); on != 0 | - | int `0x008786d0` (native_chr) |
| `0xc00e` | `Native_c00e` | id, b, c, d | int: SqsSePlay(id, b, c, d) (0x0081e4e0); id | - | int `0x00878700` (native_mods) |
| `0xc00f` | `Native_c00f` | var int | 0 (as 0xc008) | - | int `0x00878750` (native_sys) |
| `0xc010` | `Native_c010` | voice int | 1; SndStandbyStream(voice) (0x00887570) | - | int `0x00878770` (native_sys) |
| `0xc011` | `Native_c011` | - | int: SndStartStream() (0x00887630); 1 | - | int `0x00878790` (native_mods) |
| `0xc012` | `Native_c012` | - | int: SndGetStreamState() != 0 (0x008864b0) | - | int `0x008787a0` (native_mods) |
| `0xc013` | `Native_c013` | - | 1 when MsGetSaveConfigHiragana() (0x007851f0) is 1 | - | int `0x008787b0` (native_chr) |
| `0xc014` | `Native_c014` | - | 1 when 0x007851b0() is not 0 | - | int `0x008787c0` (native_chr) |
| `0xc015` | `Native_c015` | &name int | debug (mod-tool candidate): the variable's address taken, then under TkIsDebug only: a host file read into a TkMalloc buffer and sent with SndSepTrans; 1 | - | int `0x008787d0` (not ported) |
| `0xc016` | `Native_c016` | v int | 1; 0x00886e80(v) | - | int `0x00878860` (native_sys) |
| `0xc017` | `Native_c017` | v int | 1; 0x00886eb0(v) | - | int `0x00878880` (native_sys) |
| `0xc018` | `Native_c018` | - | debug (mod-tool candidate): in a debug build SqsSetProjectBase(&var); 1 in release | - | int `0x008788a0` (not ported) |
| `0xc019` | `Native_c019` | map | int: AtelMapLoadNoCache(map) (0x0086dd30); 0 | - | int `0x00878550` (native_mods) |
| `0xc01a` | `Native_c01a` | party int, kick int, variation int | 1; SndKickFoot(1, party, kick, variation) | - | int `0x008788e0` (native_sys) |
| `0xc01b` | `Native_c01b` | party int, kick int, variation int | 1; SndKickFoot(0, party, kick, variation) | - | int `0x00878920` (native_sys) |
| `0xc01c` | `Native_c01c` | party int, kick int | 1; SndKickFoot(5, party, kick, 0) | - | int `0x00878960` (native_sys) |
| `0xc01d` | `Native_c01d` | party int, kick int, variation int | 1; SndKickFoot(2, party, kick, variation) | - | int `0x008789a0` (native_sys) |
| `0xc01e` | `Native_c01e` | party int, kick int, variation int | 1; SndKickFoot(3, party, kick, variation) | - | int `0x008789e0` (native_sys) |
| `0xc01f` | `Native_c01f` | party int, kick int, variation int | 1; SndKickFoot(4, party, kick, variation) | - | int `0x00878a20` (native_sys) |
| `0xc020` | `Native_c020` | v int | int: v (popped and returned) | - | int `0x00878ac0` (native_sys) |
| `0xc021` | `Native_c021` | v int | int: v (popped and returned) | - | int `0x00878ae0` (native_sys) |
| `0xc022` | `Native_c022` | - | debug (mod-tool candidate): in a debug build Sg_EnableDrawGui(v); 1 in release | - | int `0x00878b00` (not ported) |
| `0xc023` | `Native_c023` | worker int, kind int, x var, y var, z var | 1; the given worker's character position (0x00833c30(chr, kind, &matrix) translation) into the three variables, 0.0 each without a character | - | int `0x00878b40` (native_sys) |
| `0xc024` | `Native_c024` | _, v | start: the battle end (AtelEventSetUpForBattleMusic, AtelEventRemoveForBattle for the event jump map, AtelBattleEndCallback(1, v, map)); its EAX; poll: 0xc024 (0x00878c60, poll), 0xb00a (0x0076ec10, int) and the like: 1; poll: 1 | see the poll | start `0x00878c10`, poll `0x00878c60`, poll `0x00878c60` (native_chr) |
| `0xc025` | `Native_c025` | msg int | int: 1 when the message text (Msg_Text) fits and is copied over the player's name (MsGetSaveChrName(0), shorter than MsGetSaveChrNameSize(0)), else 0 | - | int `0x00878c70` (native_chr) |
| `0xc026` | `Native_c026` | var int, v int | 1; 0x00886a00(AtelGetRealVarAdrs(worker, var), v) | - | int `0x00878ce0` (native_sys) |
| `0xc027` | `Native_c027` | - | debug (mod-tool candidate): under TkIsDebug only: TOSetCollectBit(-1) and 0x0088e780(0xa004 + k) for k < 26; 0 | - | int `0x00878d20` (not ported) |
| `0xc028` | `Native_c028` | - | debug (mod-tool candidate): in a debug build 0x0088e790(0xa004 + i) for 26 i; 1 | - | int `0x00878d60` (not ported) |
| `0xc029` | `Native_c029` | flags int, b int, a int | int: a when bit 1 of flags is set, else b (the level's +0x884 minus the yaw is wrapped by 0x0088bc20 and dropped) | - | int `0x00878d90` (native_sys) |
| `0xc02a` | `Native_c02a` | - | debug (mod-tool candidate): in a debug build TkSetOutputEncount(v) and AtelOutputEncountStart(); v | - | int `0x00878e10` (not ported) |
| `0xc02b` | `Native_c02b` | - | int: 0 (xor eax, eax; ret) | - | int `0x00878e50` (native_mods) |
| `0xc02c` | `Native_c02c` | v int | int: the popped value; int: the popped value | - | int `0x00878e60` (native_chr) |
| `0xc02d` | `Native_c02d` | party int | 1; SndKickFoot(6, party, 0, 0) | - | int `0x00878a60` (native_sys) |
| `0xc02e` | `Native_c02e` | party int | 1; SndKickFoot(7, party, 0, 0) | - | int `0x00878a90` (native_sys) |
| `0xc02f` | `Native_c02f` | on, v f | the current ctrl work's +0x208 = v and +2 bit 0 = on != 0; EAX on != 0 | - | int `0x00878e80` (native_chr) |
| `0xc030` | `Native_c030` | - | TkIsDebug(), then the current ctrl work's +0x1f4 | - | int `0x00878ed0` (native_chr) |
| `0xc031` | `Native_c031` | v | 0x0086dcd0(v) | - | int `0x00878530` (native_chr) |
| `0xc032` | `Native_c032` | - | 1 (Script_WorkerLevel of the running level, unused) | - | int `0x00878ef0` (native_sys) |
| `0xc033` | `Native_c033` | - | int: jumps to Ch_EnableGetTexture (0x00830320) | - | int `0x00878f10` (native_mods) |
| `0xc034` | `Native_c034` | v int | debug (mod-tool candidate): pops v; under TkIsDebug (0 in this build) 0x00830330(1); 1 | - | int `0x00878f20` (not ported) |
| `0xc035` | `Native_c035` | pc | int: AtelGetCharHandleFromBindPC(pc) (0x0086ae90) | - | int `0x00878f50` (native_mods) |
| `0xc036` | `Native_c036` | on | pppFpSetDisp(on != 0) (0x00729050); on != 0 | - | int `0x00878f70` (native_chr) |
| `0xc037` | `Native_c037` | i int | int: the string table 0x008b9130()'s base plus its u16 offset 2i (2i + 1 when 0x0086bad0() is set) | - | int `0x00878fa0` (native_sys) |
| `0xc038` | `Native_c038` | c int, b int, var int | 1; GetCD()'s +0x10(c), then its +0(b, the variable's address, 0, 0) | - | int `0x00878fe0` (native_sys) |
| `0xc039` | `Native_c039` | v int | int: GetCD()'s +8(v) | - | int `0x00879040` (native_sys) |
| `0xc03a` | `Native_c03a` | a int, b int, volume int | 1; SndStartShoutStream(a, b, volume) (0x008875d0) | - | int `0x00879070` (native_sys) |
| `0xc03b` | `Native_c03b` | - | debug (mod-tool candidate): MsDebugFullItem (0x007849d0), the debug cheat that gives every item; 1 | - | int `0x008790b0` (not ported) |
| `0xc03c` | `Native_c03c` | - | debug (mod-tool candidate): MsDebugFullCommand (0x007848b0), the debug cheat that teaches every command; 1 | - | int `0x008790c0` (not ported) |
| `0xc03d` | `Native_c03d` | bit int, on int | int: on; the effect flag bit set (AtelSetEffectFlag) or cleared (0x00863860) | - | int `0x008790d0` (native_sys) |
| `0xc03e` | `Native_c03e` | v int | int: 0x00886680(v) | - | int `0x00879110` (native_sys) |
| `0xc03f` | `Native_c03f` | v int | int: v; 0x00887180(v) | - | int `0x00879130` (native_sys) |
| `0xc040` | `Native_c040` | - | int: jumps to 0x00886430 | - | int `0x00879150` (native_mods) |
| `0xc041` | `Native_c041` | v int | int: v; 0x008871c0(v) | - | int `0x00879160` (native_sys) |
| `0xc042` | `Native_c042` | - | int: jumps to SndGetSoundType (0x00886440) | - | int `0x00879180` (native_mods) |
| `0xc043` | `Native_c043` | - | int: SndSepClearAll() (0x00886db0); 1 | - | int `0x00879190` (native_mods) |
| `0xc044` | `Native_c044` | v int | 0; 0x0088c7f0(v) | - | int `0x008791a0` (native_sys) |
| `0xc045` | `Native_c045` | v int | int: 0x0088c800(v) | - | int `0x008791c0` (native_sys) |
| `0xc046` | `Native_c046` | v int | 0; 0x0088c820(v) | - | int `0x008791e0` (native_sys) |
| `0xc047` | `Native_c047` | offset int | float: the f32 at the buffer 0x0088c830() plus offset | - | float `0x00879200` (native_sys) |
| `0xc048` | `Native_c048` | offset int, v f | float: v; the f32 at 0x0088c830() plus offset = v | - | float `0x00879280` (native_sys) |
| `0xc049` | `Native_c049` | offset int | int: the word at 0x0088c830() plus offset | - | int `0x00879220` (native_sys) |
| `0xc04a` | `Native_c04a` | offset int, v f | int: v truncated; stored as a word at 0x0088c830() plus offset | - | int `0x008792c0` (native_sys) |
| `0xc04b` | `Native_c04b` | offset int | int: the i16 at 0x0088c830() plus offset, sign-extended | - | int `0x00879240` (native_sys) |
| `0xc04c` | `Native_c04c` | offset int, v f | int: v truncated; its low 16 bits stored at 0x0088c830() plus offset | - | int `0x00879300` (native_sys) |
| `0xc04d` | `Native_c04d` | offset int | int: the i8 at 0x0088c830() plus offset, sign-extended | - | int `0x00879260` (native_sys) |
| `0xc04e` | `Native_c04e` | offset int, v f | int: v truncated; its low byte stored at 0x0088c830() plus offset | - | int `0x00879340` (native_sys) |
| `0xc04f` | `Native_c04f` | - | int: 1 (mov eax, 1; ret) | - | int `0x00879380` (native_mods) |
| `0xc050` | `Native_c050` | - | int: movie_stop_prog() (0x0076f090) when movie_run_check() (0x0076f060); 1 | - | int `0x00879390` (native_mods) |
| `0xc051` | `Native_c051` | on | TkSetTGS(on != 0) (0x0088ec10); on != 0 | - | int `0x008793b0` (native_chr) |
| `0xc052` | `Native_c052` | - | jumps to 0x0088d6d0 | - | int `0x008793e0` (native_chr) |
| `0xc053` | `Native_c053` | - | four popped and dropped; 0 | - | int `0x008793f0` (native_chr) |
| `0xc054` | `Native_c054` | n | TkMalloc(n) (0x0088e650) | - | int `0x008781b0` (native_chr) |
| `0xc055` | `Native_c055` | p | TkFree(p) (0x0088d520); 1 | - | int `0x008781d0` (native_chr) |
| `0xc056` | `Native_c056` | - | debug (mod-tool candidate): TkCdRead(group, index, adrs) with a dbgPrintf (a debug file read); 1 | - | int `0x00878230` (not ported) |
| `0xc057` | `Native_c057` | - | debug (mod-tool candidate): copies 0x68c0 bytes from the address + 0x40 into g_SaveData and TkFlushSyncH ('copy to saveram'); 1 | - | int `0x008781f0` (not ported) |
| `0xc058` | `Native_c058` | v | AtelGetRealVarAdrs(worker, v) | - | int `0x00878280` (native_chr) |
| `0xc059` | `Native_c059` | i, on | int: MsGetSaveCommandMenuPanel(i) (0x007850b0) before MsSetSaveCommandMenuPanel(i, on != 0) (0x00785d40) | - | int `0x008782a0` (native_mods) |
| `0xc05a` | `Native_c05a` | event int, variation int | 1; SndKickFoot(0xff, event, 0, variation) (kind 0xff plays a sound event) | - | int `0x008782f0` (native_sys) |
| `0xc05b` | `Native_c05b` | v int | debug (mod-tool candidate): AtelPs2DebugString2("DEBUG:%d/%d" of the worker id +0x2e and v) (0x00873c80); v | - | int `0x00878330` (not ported) |
| `0xc05c` | `Native_c05c` | v int | debug (mod-tool candidate): AtelPs2DebugString2(sprintf("DEBUG:%d/0x%x", worker id, v)); v | - | int `0x008783c0` (not ported) |
| `0xc05d` | `Native_c05d` | v int | int: v; 0x00887ca0(v) | - | int `0x00878470` (native_sys) |
| `0xd000` | `Native_d000` | v | int: popped for an "abiritymap_debug(%d)" line through the empty dbgPrintf; 0 | - | int `0x00a445f0` (native_mods) |

### Helpers (script round 22)

`ffx_script::vm::bridge::helpers` has, besides the setup functions of
earlier rounds: `Atel_ClearBindMask` (`0x00863650`, the counterpart of
`Atel_SetBindMask`), the event sprites' alpha and colour setters
(`0x00874ad0`, `0x008749f0`: the slot clamped to 0-127, the values to
0-255; `EAX` is slot * 7), `Atel_GetVoiceStreamParams` (`0x00885ba0`:
digits 4-5 of `(id >> 12)` in decimal below 21 give `(0x19, 1)`, else
`(0x1b, 0)`), the character-name converters `setDefaultPlayerName` uses for
language 1 (`0x00875220`) and languages 9 and 10 (`0x00875370`, the
inverse), and the line table above. `native_btl::atel_btl_move_target_dist`
is `0x007a9150`, the battle move set-up behind `0x700c`, `0x7085` and
`0x7053`: it pops `c`, `b`, `a`, brake, accel, `n`, speed and kind (floats
through `fstp m32`) and, when the worker has a battle actor and `n` > 0,
calls `MsGetMoveTargetDist` (`0x007ab540`) with the actor's `+0x416` and
returns the actor record (0 otherwise). `0x00a7ae20` is a copy of
`Angle_Wrap` (`ST0` result) and is hooked with `ffx_chara`'s port. The
natives that called these through the original now call the Rust ones.
`round22_helpers_match_the_original` checks them on the image and on each
snapshot. On the snapshots' `0xcd`-filled event workers (battle id from
`0x5d` up, `MsGetChr`'s spare record) `MsGetMoveTargetDist` copies a
position `MsGetChrPos` never writes (`[ebp-0x14]`) into the record's
`+0x390`; with the pops in Rust, the leftovers there differ from the
original's, so a sixth uninitialised-stack stub zeroes its frame
(`0x007ab546`, as `MsGetTargetDist`'s). Move kinds 2 and 10 read other
actors through that record's garbage, where the original stops; the test
leaves them out there.

### Helpers (script round 23)

The move and turn blocks: each level block (`+0x12c + level*0x4c`) points
at a move block (`+0x44`) and a turn block (`+0x48`); a level's own blocks
are `+0x5b8 + level*0x4c` and `+0x864 + level*0x44`, and levels sharing one
move or turn together. `Atel_StartMoveBlock` (`0x008703e0`) and
`Atel_StartTurnBlock` (`0x00871030`), the tails of `Native_MoveStart` and
`Native_TurnStart`, write the current level's own block `+0` = param, `+2`
= mode, `+6` = 0 (16 bits each) and hand it to `Atel_ShareMoveBlock`
(`0x00870440`) or `Atel_ShareTurnBlock` (`0x00871090`): the level's own
block (the level unclamped) becomes the clamped level's block (0-1 for
types 5 and 6, 0-8 otherwise), and so for every level `i` from 0 to the
level (to 7 with mode bit `0x20`) whose bit is in `mask | 1 << level`;
with mode bit `0x100` only for levels whose current block is idle (`+2 &
0xff80` for moves, `& 0xffe0` for turns). All four return the `EAX` the
loop leaves. `Atel_AddSepLoad` (`0x0086e250`) appends `(id, flags |
0x8000)` to the sound-separate load list (`g_AtelSepLoadList`,
`0x01327030`, 8 entries, count `0x01325b78`) when the flags' group bits
(`0x7e0000`, none = group 0 at bit 16) meet `(g_AtelSepGroupMask |
1) << 16` (`0x01326bdc`): 0 added, -1 already listed, 1 full or not
selected. `0x00866520` fills it from the event header's separates table
(`+0x2c`, then `+0xc`: a count and `(id, flags)` pairs), and `0x008638c0`
queues the pending ones with a fade of 0xf0 frames (0x78 with flag
`0x2000`, 0x168 with `0x4000`). `round23_helpers_match_the_original`
checks them on synthetic workers of every type and on the snapshots' real
workers (those whose level blocks point at their own blocks).

`AtelSetBindObject` (`0x0086f4c0`) moves the map object a worker is bound
to (`+0xaa` 4: the group `+0xa8`, `yiSetRotTransGroup`; 5: the object
`+0xa8`, `yiSetRotTransDObj`) to its position block (`+0x284` for types 5
and 6, `+0x558` otherwise): rotation `+0x30`..`+0x38` with w 0, translation
`+0`..`+8` with w 1, both built in the original's frame. Its `EAX` is that
call's (a pointer into `yiSetRotTransGroup`'s own frame), else the block
pointer with `+0xaa` in the low byte; type 4 workers have no block and the
original reads through 0. `AtelSetPos3d` (`0x00870b20`, `(worker, x, y, z,
snap)`, returns `snap`) writes the block's position, then by `+0xaa`: 1 the
character (`+0x9c`) `Ch_SetPos3` and `Ch_ClothReset`, 2 the camera
`MsCameraSetRect(+0x3c, 1, &(x, y, z, 1))`. For the player worker (`+0x2e`
= the ctrl work's `+0xa`) the ctrl work's position `+0x218` (w `+0x224`)
moves to `+0x228` and becomes the block's (w 1), and the save event's
encounter byte (`+0x10`) and `AtelSetSaveDicIndex` take the character's
PID fields (`Ch_IdGetEncount`, `Ch_IdGetDictionary`; 0 without one).
Without the worker's `+0x34` bit 4, or with `snap`, the previous position
(`+0x228`, w 1) and the block's `+0x10`..`+0x18` follow the new one. The
ctrl work's `+2` bit 4 is set. The natives that called both through the
original call the Rust ones, with the native scratch as the frame.
`round23b_helpers_match_the_original` covers them (character, map-group
and camera bindings come from the snapshots).

The rest of round 23: `Atel_UcResume` (`0x0086a130`) runs `AtelUcOff`,
then `AtelUcOn` when the ctrl work had given the player control (`+0`
bits `0x22`, read first) and its `+1` bit 2 and `+3` bits `0x84` are clear;
`Atel_UcRestore` (`0x0086a0e0`) does the same and then puts `+0` bit 1
back as it was. `Atel_GetWorkerRotY` (`0x0086a6e0`) returns the position
block's `+0x34` of a worker index in `ST0`; `Atel_SyncChrDir`
(`0x00869df0`) stores `Ch_GetDir` of the worker's character into the
block's `+0x20` when bound to it (`+0xaa` 1). `AtelBindEff2On`
(`0x00862700`) switches the character's bind effect 2 on (`op_et_bindeff2`,
`_draw_on`) and one of its parts (kind 0 walk, 1 breath, 2 water), marking
it in the worker's `+0x38` (bits 5, 6, 7). `Atel_QueueSepLoads`
(`0x008638c0`) turns the pending entries of `g_AtelSepLoadList` that the
event header's table does not list into the separates queue
(`g_AtelSepQueueCount`, `0x0132707c`, entries from 1: ids at
`0x01327080`, fades at `0x013270a0`). `round23c_helpers_match_the_original`
covers them; `op_et_bindeff2_draw_on` finds no bind-effect slot on either
snapshot's characters, so its call is not observed there.

### Helpers (script round 23, second machine's round 23 d)

The battle bridges (`MsBtlBridgeBtl2Event`, `MsBtlBridgeEvent2Btl`) carry
the event's workers through a battle with a set of `...FromID` accessors.
Each makes ctrl work 0 current for the `AtelGetBasicWork` call and puts the
old one back. Worker fields they use: `+0x33` the link byte
(`AtelGetEventActorLinkWork` `0x0086b1b0` / `AtelSetEventActorLinkWork`
`0x0086fc40`; the getter's `mov al` leaves the worker's upper bytes in
`EAX`), `+0xaa` the binding (`AtelGetEventBindIdFromID` `0x0086b370`),
`+0xa8` the character id (`AtelGetEventCharIDFromID` `0x0086b3d0`), and
for the two weapon elements k: `+0xaf4 + 4k` the model
(`AtelGetEventElemHandleFromID` `0x0086b440` /
`AtelSetEventElemHandleFromID` `0x0086fde0`), `+0xafc + 4k` the weapon
record's `+0xd` byte (`AtelGetEventElemPosFromID` `0x0086b480` /
`AtelSetEventElemPosFromID` `0x0086fe20`), `+0xb04 + 2k` the element's
character id (`0x0086b400` / `0x0086fda0`), and `+0xb14` a 16-bit mask of
the elements in use (`AtelGetEventElemStatus` `0x0086b4c0`, bit k mod 32;
`AtelSetEventElemStatus` `0x0086fe60`, a 16-bit `shl`, so k mod 32 from 16
up is no bit). Fahrenheit names `0x0086b480` the handle getter (a BSim
match). The getter at `0x0086b440` is the one that reads what the handle
setter stores, and `MsBtlBridgeEvent2Btl` puts its result into the weapon
record's model word, so the names here follow the stores.
`AtelSetEventCharHandleFromID` (`0x0086fd60`) stores the model `+0x9c`
only for a character binding (`+0xaa` 1). `AtelGetEventActorType`
(`0x0086b320`) is the type byte, or 7 outside ctrl work 0's actor count
(`+0xc`, not the worker count).

Without the context switch: `AtelGetHideStatusFromID` (`0x0086ba40`,
`+0xa0`), `AtelGetKeepBattlePartyFromID` (`0x0086bbf0`, bit 8 of `+0x38`),
`AtelGetRequestLevel` (`0x0086c3f0`, `+0x32`), `AtelGetMyWorkSize`
(`0x0086c160`: the header's `+0x10` rounded up to 16 with `cdq`, so
towards zero below -15) and `AtelSetMyWorkAdrs` (`0x00870870`: `+0xb8` =
the address, then the unrounded size in bytes cleared at it, or, without
one, at the binary `+4` plus the header's `+0x2c` when that offset is set;
`EAX` 0 after a clear, else the worker). `AtelGetElemId` (`0x0086b150`) is
`MsGetSaveChrArmorModel` for kind 1, else `MsGetSaveChrWeaponModel`;
`Native_0119` and `Native_01ba` call the Rust version.
`round23d_event_accessors_match_the_original` covers them: synthetic ctrl
works on the image (indices past the end, fewer actors than workers,
negative sizes, a binary plus offset that wraps to 0), then the event's
workers from every ctrl work in use on the snapshots.

The save event record setters behind the natives (round 23 e) each fetch
the record (`TkGetSaveEventAddress`) and leave it in `EAX`:
`AtelSetWaterBattleEffect` (`0x008715f0`, `+0xd7` = on != 0),
`Atel_SetSaveEventD1` (`0x0086f060`, `+0xd1`), `Atel_SetSaveEvent13Bit1`
(`0x0086f430`, bit 1 of `+0x13`), `AtelSetForcePlaceId` (`0x00870140`,
`+0xd4` the place and `+0xd6` = 1), the `u16` stores
`Atel_SetSaveDicIndexValue` (`0x00870ed0`, `+0xe`, the field
`AtelSetSaveDicIndex` computes), `Atel_SetSaveEventE8` (`0x00873dd0`) and
`Atel_SetSaveEventEA` (`0x00870fa0`), and `Atel_SetSaveEventE4`
(`0x00874030`). `AtelSetHelpEventFlag` (`0x008701f0`) sets or clears the
bit `AtelGetHelpEventFlag` reads. `AtelInitSaveWork` (`0x0086d4e0`) clears
the record's script variables (`+0x1ec`, 0x1000 bytes) and the separates
load list (`g_AtelSepLoadCount`, `g_AtelSepGroupMask`), then tail-jumps to
`NkzVersion`, so `EAX` is the build date string. `AtelSetDolby`
(`0x0086f9e0`) stores bit 7 of the save menu word, then sets the sound
mode (0 Dolby, 1 when bit 0, else 2) and the pad vibration (bit 4).
`Atel_JumpToEntrance` (`0x00874000`, behind `0x8006`) marks
`g_AtelMapIdPage` (1, the entrance, `0xff`) and jumps to that entrance of
the current map. `AtelMapLoadNoCache` (`0x0086dd30`) is `AtelMapLoad`'s
code and runs its port; `AtelMapLoadBtl` (`0x0086dcd0`) reads a battle
map instead. The natives that called these call the Rust versions now.
`round23e_save_setters_match_the_original` covers them. Its map loads
compare only maps 0 and -1, because a positive map makes the original
read the map file, which the oracle cannot do.

Positions and polls (round 23 f). `AtelGetActorStartPos` (`0x0086a720`),
`AtelGetActorEndPos` (`0x0086a390`) and `Script_GetWorkerPos`
(`0x0086a2f0`) copy a worker's position block (`+0x284` for types 5 and
6, address 0 for type 4, else `+0x558`) through the x87 into three out
words. Two-point workers (types 2 and 5) have a second point at the
block's `+0x30`: the start getter reads the first point, the end getter
the second, and `Script_GetWorkerPos` gives `(b + a) * 0.5` per axis
(the constant at `0x00b92220`). These three read y and z before they
store x, so outs that overlap the block show the order. `Event_SetPlayer`
(`0x0086f6a0`) stores the player id (`+0xa`) and, unless the 32-bit id is
`0xffff`, copies that worker's position into the ctrl work's `+0x218`
and `+0x228`. `AtelIsBind` (`0x0086d950`) is a character binding
(`+0xaa` 1) with a model. `Atel_HasSignalFromLevel` (`0x00863520`, behind
the `0x00f6`/`0x00f7` polls) looks for a queued signal entry (`+0x80`
list) of a kind other than 3 at or above a level. The effect polls switch
on `MsEventEffectGetStat` through jump tables. `Atel_EffectStartPoll`
(`0x008633e0`, `0x00dc`) starts the effect in state 3, and in state 5
clears `g_AtelEffectState` and reports done. `Atel_EffectIdlePoll`
(`0x008633a0`) is done in state 0. `Atel_EffectEndPoll` (`0x00863350`,
`0x00de`) is done in states 0 and 6, where it also sets
`g_AtelOverlayStatus` to 4. `Atel_SetLevelVectorsXZ` (`0x0086f750`,
`0x0014`) calls `Atel_SetLevelVectors` with a middle 0.
`Atel_GetWorkerBits10_12` (`0x0086a440`) reads `+0x34` bits 10..12, at
most 5. `AtelRecoverPreBgPage` (`0x00873cc0`) puts `g_AtelMapIdPage`'s
saved BG and ID pages back. `AtelExecReturn2` (`0x00869350`) tail-jumps
to `AtelExecReturn` with the caller's three words when the worker has a
request level. `AtelGetCharHandleFromBindPC2` (`0x0086aee0`) is
`AtelGetCharHandleFromBindPC`'s code and runs its port.
`round23f_positions_and_polls_match_the_original` covers them. In it the
effect's handler table points at a stand-in that only returns.

Event set-up helpers (round 23 g). `AtelInitSourceCodeDebugWork`
(`0x0086d670`) copies up to eight words of a source record (count at
`+0`, entries every 12 bytes from `+4`) into the worker's `+0xc`.
`AtelEventSetupGuideMap` (`0x00866490`) and `AtelEventRemoveGuideMap`
(`0x008661a0`) set and clear ctrl work 0's guide map flag (`+1` bit 7).
On PC `Yn_GuideMapInit` and `Yn_GuideMapEnd` are empty, so only the flag
and `Yn_GuideMapSetMode(0)` remain. The event header's sound table (its
`+0x2c` offset) gives `AtelEventSetUpFoot3d` (`0x00866210`) the foot
table for `SndEventSetFoot3dCtrlp`, and gives `AtelEventLoadMusic`
(`0x00865a60`) a music word (`+0x38`). Only a negative music word is
read (`SndMusicRead` of its low half). The `SndSyncMusicStandby(1)` after
the read returns 0 on PC without looking at its argument, and the
`AtelPs2DebugString` calls around it are a bare `ret`.
`AtelInitCalcSound` (`0x00877420`) clears the 3D sound work and copies
16 bytes from `0x00c0a004` to `0x01328278`. `AtelGetGuideMapAngEx`
(`0x0086b9a0`) returns ctrl work 0's `Atel_GetBinHdrFloat` in `ST0`, or 0
when that work is unused or has no event block.
`AtelInitInternalActorWork` (`0x0086d260`) zeroes a worker's
`+0x48..+0x7f`. `AtelSetOnMemoryMovieFileWait` (`0x00873f70`) sets the
texture video info and records the movie in the save event record
(`+0xd8` movie, `+0xdc` flags, `+0xe0` = 1). `AtelRecoverFromMenuCallback`
(`0x0086e700`) resets the menu bank, the guide map display, the forced
place and the seven vibration masks, and sets the overlay status to 6.
`round23g_setup_helpers_match_the_original` covers them. On the image it
needs a Phyre scene (`0x00ccb9d8`) and an event block with an expansion
float record.

FP bindings (round 23 h). A type-1 worker keeps four FP (effect point)
slots at `+0xb18`, 0x10 bytes each: `+0` holds the FP id in its low 15
bits and sets bit 15 once a position is set, `+2` holds the bone (or the
map group or object), and `+4..+0xc` hold the position. `AtelBindFp`
(`0x00862800`, behind `0x8009` and `0x0196`) takes the first free slot
and sticks the FP to whatever the worker is bound to: the character
model (`pppFpSetStickChr`, binding 1 with a model), the map group (4) or
the map object (5). `AtelBindFpOff` (`0x008628a0`, `0x800b`) frees the
slots holding an id, or every used slot for id 0 (each through
`pppFpResetStick(0)`). `AtelBindFpPos` (`0x00862920`, `0x800a`) stores
the position and bit 15 and passes `(x, y, z, 1.0)` in a stack local to
`pppFpSetStickPos`, which reads only x, y and z. After a battle,
`AtelRecoverFPFromBattle` (`0x0086e4c0`) sticks every used slot again
and puts its position back. With any other binding it returns at the
first used slot. `round23h_fp_bindings_match_the_original` covers them
on synthetic workers (types 0, 1, 2, every binding) and on the
snapshots' type-1 workers. The ports get a scratch frame for the vector.
`pppFpSetStickChr` does nothing for model 0, because `Ch_CheckHandle(0)`
is -1.

Radius tests, map index, music and the map camera (round 23 i).
`Atel_InRadiusXZ` (`0x008634b0`) and `Atel_InRadiusXYZ` (`0x00863440`)
call `Math_InRadius` with the point's offset from the worker's position
block; the XZ form passes 0 for y. Both write the differences back into
their own argument slots. `AtelGetMapIndex` (`0x0086bd90`) returns -1
without an entrance table. With a language table in the sound table's
map record it returns that table's word for the current language
(`TkGetLang`, 0 outside 0..16; `MsGetSaveConfigEnglish` is 0 on PC).
Otherwise it returns entrance i's map. `AtelEventPlayMusic`
(`0x00865ac0`) works only on a negative music word: a number from
`0x100` up fades the playing music out, a smaller one starts or
cross-fades unless it is already playing, and either way the number is
registered (-1 for the fade). `AtelRequestMapCamera` (`0x0086ed80`)
sends the camera worker signal (`0xffff`, worker, 0, level 1, the
current entrance's camera). Its entrance table falls back to its first
record when the index runs past the end, and to `0x01325b50` when
there is no table. `round23i_radius_music_camera_match_the_original`
covers them; the music calls run only on the snapshots, because the image
has no sound context.

### Helpers (script round 24)

`getEventLocTexListPath(out)` (`0x00874c50`) builds the event's texture
list path: `/FFX_Data/GameData/PS3Data/event/<dir>/tex/TexList.txt`, with
`<dir>` the language prefix (`g_EventLangDirs`, `0x00c52768`: `""` but
`fr/`, `sp/`, `de/`, `it/`; `""` for names holding `sins05`) + `obj/` +
the event name (`g_EventName`, `0x023cbc60`), or empty past language 10.
Names holding `bltz` skip the prefix; for the five tutorial events
`bltz0000`, `bltz0005`, `bltz0006`, `bltz0200`, `bltz0201` the language
directory of the jump table at `0x00874f84` (0 `""`, 1 and 6-8 `us/`, 2
`fr/`, 3 `sp/`, 4 `de/`, 5 `it/`, 9 `kr/`, 10 `ch/`) goes in seven bytes
before the name. `fiosUnifyFilename` turns it into the PC path through a
256-byte local, copied back with `sprintf_s`; `EAX` is its length.
Language 11 on map `0x79` points `g_EventLangDirs[11]` at `cn/` first. A
path longer than 256 bytes ends the game (`sprintf_s`), so the tests stay
below that.

`AtelStopSoundAll` (`0x00871890`) stops sound separates 1-0x77, the music
(twice) and FMOD's music, with `TkSyncV(0)` + `TkSndUpdate` pairs between,
and ends in `SqsAllVoiceTerminate` (through its thunk `0x008872e0`); `EAX` is the last
`TkSndUpdate`'s. `AtelGetHyoukaMenuEnable` (`0x0086bae0`) is ctrl work 0's
`+1` bit 4, cleared by `TkGetTGS`, battle, an Eternal Calm scene, ctrl
work 0's state bits (`+0` bits 0-1 needed; `+1` bit 2, `+3` `0x87`, `+2`
bit 5 block), a sound read, a movie or `g_AtelOverlayStatus` 1; it makes
ctrl work 0 current while it asks. `Atel_SetRunInBattle_008630c0` (name
ours, from `AtelBattleEndCallback`) sets `+0x34` bit 2 on every actor of
ctrl work 0 when its `+0` bit 0 is set, through `AtelGetBasicWork`'s
lookup inlined on ctrl work 0 (`atel_get_basic_work_in`).
`round24a_tex_list_path_matches_the_original` and
`round24a_sound_menu_battle_match_the_original` cover them; the sound
calls are checked against logging stand-ins (`r24_log_stubs`: every
call, its arguments and order land in a buffer the memory comparison
sees), the menu check's callees against fixed results.

`Atel_InitFreeSignals_0086d510` (name ours, from `AtelInitBasicWork`,
which empties `+0x88` first) pushes the worker's own signal nodes onto
its free list: types 5 and 6 clear the line table word `+0x2e4` and push
the eight nodes from `+0x1c4`, the others the 16 from `+0x3d8`.
`Atel_ResetMsgWindowBank_00863c90` (from `AtelEnterMenuCallback`) resets
one bank's eight message windows as `AtelClearMesWinWork` resets all 16.
`Atel_LoadEventSeps_00866520(_, mode, group)` (from `0x00862470`,
`AtelEventRemoveSep`, `0x00866500`) empties `g_AtelSepLoadList` and
refills it from the separates table of ctrl work 0's script block: with
`mode` 0, entries with flags bit 1 and not bit 2, or bit 3 and not bit 1;
with `mode` set and `group` selected (`(g_AtelSepGroupMask | 1)`, bit
`group`), none, but the entries with none of the group's bits (bit
`16 + group`) are kept; otherwise entries with bit 0. Entries added and
not refused (1: full or unselected) are kept too, and the first eight
kept ids stay loaded (`SqsSeSeparateMaskClear` through its thunk
`0x00886dc0`) or, with none, every separate is cleared
(`SndSepClearAll`); the group mask ends 0.
`round24b_signals_windows_seps_match_the_original` covers them (the two
sound calls logged).

Round 24 c: `destroyMoviePicture` (`0x0076ece0`) clears a movie picture
(`g_MoviePictureOn`, `0x0112a00c`; the time-counted frame wait off,
`0x00c40e2c` = 1) and tail-jumps to `graphicDestroyFmv`.
`Movie_IsBusy_0076ed10` gives 0 only while a video plays
(`graphicIsVideoPlaying` or `0x00641ce0`) without `movie_with3DScene` or
a movie picture. `TkMenuSaveLoad_PollCard_008b3d20` steps the save
screen's card poll (`0x0186626b`: 0 to 1, 1 asks `TkMcGetInfo`, 2 ends)
and, when done, sets the save choice, resets the step and flips
`g_TkMenuSaveLoadToggles`. `Atel_SetVarSlot_00878140`, the core of
`Native_101c`/`101d` (which now call it in Rust), pops a variable word
and a slot (0-7) and points the current ctrl work's `+0x1a4 + slot*8` at
the variable. `Atel_GetEventActorRot_0086b270` is
`AtelGetEventActorPos`'s twin for the rotation (`+0x30`..`+0x38`;
x, z, then y after the ctrl work goes back). `Atel_TransSepGroup_00871780`
waits for `SndSyncTrans`, queues the pending separates for a negative
group, and sends group `group` of a separates block (a head word with
bit 30 set and bit 31 clear: the sizes, then the groups from `+0x20`;
the earlier sizes are summed in pairs) with `SndSepTransSepOnly` or
`SndSepTransNonblock`, then selects the group in `g_AtelSepGroupMask`.
`AtelGetGuideMapActorInfo` (`0x0086b740`) fills a guide-map record for
marker `i` of ctrl work 0's script block (jump table `0x0086b8fc` on the
worker type for the direction and the two-point flag); the y of the
start and end positions go to its own argument slot (a frame word in
Rust). `round24c_movie_menu_seps_markers_match_the_original` covers them,
the callees as stand-ins (`r24_log_stubs`, `r24_countdown_stub` for the
transfer wait).

Round 24 d: the separate wrappers `AtelSndTransSepSPKDataGroup`
(`0x00871750`: `Atel_TransSepGroup_00871780(hdr, data, 1, group, 0)`,
then `SndSyncTrans(1)`), `AtelEventSetUpSep` (`0x00866430`: group -1,
`data` kept in ctrl work 0's `+0x204` when `sep_only` is 1, then the 3D
sound ratios 0 and 1/60) and `AtelEventRemoveSep` (`0x008661d0`:
`SndStopStream`, then `Atel_LoadEventSeps_00866520(block, mode, -1)` with
ctrl work 0 current; without a script block ctrl work 0 stays current);
`AtelEnterMenuCallback` (`0x008655e0`: guide map off, vibration off for
pads 1-7, `MsEventEffectFree`, message bank 1 reset,
`TkMenuClearDummyControler`, overlay status 3) and its jump twin
`AtelGetGideMapActorInfo` (`0x0086b700`).
`Atel_RequestBattleEndEntries_0086e8d0(tag)` walks the script block's
battle-end table (`+0x24`: a count, then 16-byte entries from `+4`: tag,
worker, entry) and signals each entry of `tag` (every entry while the
save's `+0xe8` says always) to its worker at level 8.
`Atel_SetLevelTarget_0086f940(worker, i)` writes `i` into the level's
turn block and the target's position into the level vectors.
`Atel_CamGetVars_007ba3c0` and its copy `Atel_CamGetRefVars_007ba460`
store a camera's side point (`Cam_GetPos`, `Cam_GetRefPos_007bd4b0`) into
three popped script variables, looking the worker's camera up first
(`MsAtelGetCameraID(5, is the event map camera, 0)`).
`Vibrate_UpdateVolume_00875b40` ends finished pads and keeps the
channel maxima in `g_TkVibrateVolume`. `round24d_seps_menu_vibrate_cam_match_the_original`
covers them; `0x0086e8d0` runs in the script runs' set-up
(`fresh_run_pair`), where signals have free nodes.

Round 24 e: `Atel_RequestOwnerTrigger_008764a0(owner, kind)` finds the
worker bound to `owner` in ctrl work 0 (`Atel_FindWorkerByOwner`) and
requests its trigger. Two save-screen windows,
`TkMenuSaveLoad_OpenWindow_008b3c20` and `TkMenuSaveLoad_OpenChoice_008b3c70`,
are near-copies of the menu's `MenuNameEntry_RegistWindow` and
`MenuSave_OpenWindow2` with the save screen's callbacks;
`TkMenuSaveLoad_ClearLoadPos_008b3cd0` resets a card slot's load position.
`Atel_InitMovieState_00873c00` clears the save event's `+0xe0` and the
movie buffer pointers; `Atel_QueueBattleEndEffect_00861550(kind, v)`
queues the battle end's `"EFF"` callback (`0x00861f90`) with `kind` and
`v` in `0x01326b40`/`0x01326b44`; `Atel_FadeOutQueuedSeps_00868460`
fades the queued separates out and empties the queue;
`Atel_ShowWaterFps_008682a0(on)` shows or hides the four water effects;
`Atel_SetSaveEventBit_008700e0(n)` sets bit `n` (below 64) of the save
event's flag words at `+0x18`. `round24e_save_movie_effects_match_the_original`
covers them.

### Helpers (script round 25)

Round 25 a: `AtelExecSoftWareReset` (`0x008738d0`) checks, with ctrl work
0 current, that a soft reset is allowed (`+0` bit 0 set, `+3` bits `0x83`
and `+0x210` bit 2 clear, `+0x210` bit `0x40` set, no movie
(`0x0086d940` 0), map not `0x15c` (`0x0086c1c0`), no voice on standby or
loading) and runs `TkSoftWareReset` when `g_AtelResetRequest`
(`0x01327108`) is 2, or when allowed and `TkCheckSoftWareReset` agrees.
`AtelClearShape` (`0x008742b0`) resets the 128 event sprites and frees
their six effect handles each (`graphicVFXDestroy(i + k*10000, sprite)`,
the sprite's address as the id's high word), then unloads the event's
texture list (`g_AtelEventTexListLoaded`, `0x01327140`) by its path
(`getEventLocTexListPath`). `Atel_TranslateBGSeps_00861020(volume,
frames, all)` and `AtelEventKickBGSep(header, fade)` (`0x008658d0`) walk
the event header's separates table (`+0x2c` -> `+0xc`: a count and
`(id, flags)` pairs): bit 2 marks a background separate, `0x8` one that
the first also translates with `all`, `0x30` silent (volume 0), `0x80` one
that starts later (kicked without `fade`, flag 4) and keeps its volume
unless `0x40`, `0x300` never kicked, `0x1c00` a fade-in (`0x400` 0xf0,
`0x800` 0x78, `0x1000` 0x168 frames) from 0; the save event's `+0x28`
halves the volume (`0x3f` for `0x7f`). A separate pending in
`g_AtelSepLoadList` under the same id is not kicked.
`Atel_SetWorkerPosXZ_00870970(worker, x, z, keep)` (`Native_0012`,
`Native_0125`) puts a worker at `(x, 0, z)` (kind 5/6 at `+0x284`, others
at `+0x558`, kind 4 has none and the original writes through null); for
the player worker the ctx's camera target `+0x218` takes it (the old one
to `+0x228`) and the save event's encounter byte and dictionary index
follow its character. `AtelExecCtrlEventInit_008677a0(ctrl, battle)` runs
each worker's init (`Script_Run(w, 0, 1)`) until all have `+0x34` bit
`0x10`, then decodes their signals; its debug strings go to
`AtelPs2DebugString`, a bare `ret`. `AtelEventSetUpForBattleMusic`
(`0x00866240`) prepares ctrl work 0 for a battle (field and gravity
modes, swap works, FP effects of type-`0x5001` workers hidden, UC off and
back on for the old ctrl work, guide map closed).
`AtelEventRegistFootInfo` (`0x00865b80`) hands the header's four footstep
tables (`+0x2c` sub-header `+0x1c`, `+0x20`, `+0x30`, `+0x34`; 16 `u16`
offsets each) to `SndSetFootSep`. `round25a_reset_shapes_seps_setup_match_the_original`
covers them (only the workers of the first, `0xb58`-byte class have FP
slots, so the test fills those only there).

Round 25 b: `AtelAllClearCallbackQue` (`0x008b3cf0`) resets both card
slots' load positions; `AtelExecVibrate(update)` (`0x008739a0`) sends
twice the two vibration volumes to pad 0's actuators;
`Atel_SetChrAltParts_008720e0(chr, on)` swaps a character's model parts 1
and 2; `Atel_ApplyChrDir_00870360` and `Atel_SyncChrRot_00869f90` move a
direction between a bound worker's position block (`+0x20`, `+0x34`
wrapped by `Math_WrapAngle`) and its character;
`Atel_SetSpriteScale_00874a80(i, x, y)` stores an event sprite's scale
times 256 (`ftol2_sse`, low 16 bits); `Atel_StepSpriteAnim_00874bf0`
advances a sprite's frame timer by `0x100` (its animation, from
`get_ptr32_func(+0x14)`: frame count at `+4`, 8-byte frames whose length
is at `+0x12` and loop flag `0x80` at `+0x14`); `Atel_LoadEventTexList_00874b70`
loads the event's texture list. `Atel_SaveLights_0086db20` /
`Atel_RestoreLights_0086e770` keep the four light colours and three
directions in `g_AtelSavedLights` (`0x01326d00`) across a battle;
`Atel_ResetTextLanguage_0086ece0` sets the text language back to the
build's (`FFXGetAsiaMode`) at a game over and swaps the international data
when that changed it. `round25b_small_helpers_match_the_original` covers
them (the light functions on the snapshots only: the image has no scene).
`SignalSema_copy_006f3000` (an exact copy of the platform `SignalSema`)
and `0x006db800` / `0x006db8a0` (an 8-slot ring buffer per channel,
`0x1e8` bytes apart, guarded by Win32 semaphores: push and pop for the
thread `0x00703a20` serves) are left as platform code.

Round 25 c: `AtelBGSepFade(volume, frames)` (`0x00861000`) is
`Atel_TranslateBGSeps_00861020` without `all`. `Atel_StepSprites_008743e0`
steps every active event sprite while they are shown (`0x01327144`,
`0x01327f4c`) and no movie plays. `AtelDrawSemiTrans` (`0x00872ca0`)
draws the guide map (`Yn_GuideMapProcEx(show)`): show for ctrl work 0's
`+3` bit `0x20` without `0x10`, or `0x40`, when the save menu's first
byte has `0x40`, or for `+3` bit 8; never while `+0x1f4` (unsigned) is
below 3. `Atel_SetPartyMask_008623b0` keeps the three party members' bits
in `g_AtelPartyMask` (`0x01326b7c`). `Atel_RegistVibrateSignal_00876590`
sends a worker's sound and vibration curve as a signal when its `+0xab`
allows sound `k`. `Native_DrawBarSegment_00855a30` draws one bar segment
from the cursor `0x013259f4` (width `table(kind) * 0x013259f0 /
0x013259e8`; the original traps on a zero divisor, the port gives 0).
`Atel_BattleEndMapStart_008615b0(map)` queues the `"MAPR"` callback when
the battle ended on another map (`g_AtelBattleEndOtherMap`, `0x01325b88`),
else restores the map's render, guide map and FP state and ends the
battle (`Yn_BattleEnd`). `Atel_SetMovieTexture_00873eb0(id, flags)`
drives the save event's movie state (`+0xe0`: 1 set up, 3 new flags; 2..5
busy, another id waits through `AtelSetOnMemoryMovieFileWait`).
`round25c_fade_sprites_guide_party_bar_movie_match_the_original` covers
them. `AtelOutputEncountStart` (`0x00885a80`) writes a separator line to a
host-PC log (`AtelPCOpen2`/`AtelPCWrite`): debug; `0x006f3130` creates the
Iggy pause menu (`FFIggyPauseMenu`) once: PC UI.

Round 25 d: `Atel_RegistVibrateSignalWith_008766d0` is the round 25 c
vibration signal with a given curve (the worker's own for a negative
one) and `AtelRegistSignal`. `AtelSetSndCameraPos` (`0x00877470`) makes
the sound listener matrix `0x01328288`: the camera matrix with the
player's position as translation (two `FFXVu0InversMatrix` through a
local), or the camera matrix itself without a player.
`Atel_TurnToPoint_0086f7f0` stores `Math_YawPitchTo`'s yaw and pitch
from the worker to a point in its turn block (`+0x174` of its level,
clamped to 1 for types 5/6, 8 else). `Atel_ShowSprite_008748a0` turns an
event sprite on with a texture. `AtelUpdateWaterStatus` (`0x00871e50`)
keeps `g_AtelInWater` (`0x01326b83`): the map's flag (save event `+0x29`
from `yiGetMapStatus` outside battles), or the camera's y against ctrl
work 0's water level `+0x208` (level <= y; NaN counts as above).
`AtelExec_Internal_871d10` (`0x00871d70`) merges the pad history of the
frames since the last sync into `g_AtelPadHeld` and the words after it.
`Atel_SetEventPathLang_00879430` points `/ffx/proj/event/xx/` at the text
language's directory (jump table `0x00879508` with the byte index table
`0x00879524`: 1 us, 2 fr, 3 es, 4 de, 5 it, 18 uk, else jp).
`Atel_AddLoveParam_00865650` adds an event's affection (ctx `+4` for
women, `+6` for men) to a party member once per event jump id (bits from
the save event's `+0x4c`). `Atel_FindVarSlot_0087bc90` resolves a
variable reference to its table entry and element.
`round25d_turn_sprite_water_pad_lang_love_vars_match_the_original` covers
them. `0x00879b60` reads a host-PC file into a source buffer
(`AtelPCOpen`, `sceRead`; caller `AtelLoadSourceData`): debug.

Round 25 e: `Atel_TurnToWorker_0086f8a0(worker, target)` records the
target in the current turn block and turns to it;
`Atel_SyncTurnEnd_00870dc0(worker, level)` gives the character the
worker's rotation and ends the level's turn (`+0x48`, mode `u16` at `+2`,
low 5 bits; mode 5 never ends here) once its direction is the target
(exact `f32` compares, NaN never equal) or, for modes 3/4, its `+0xc`
fell below `+8`. `Atel_StreamSeek_00885bf0` points the CD (`GetCD`, its
`+0x10` select and `+4` seek) at a sound stream's sectors.
`AtelJumpGameOver` (`0x0086da10`) resets music, sound, filters and the text
language and jumps to map `0x17`. `Atel_KickBGSepsOnFlag_00865780(header,
on)` stops or plays the delayed background separates and fades the others
(`0x3f` with `on`, `0x7f` without, 12 frames). `Native_GetChrOptPos_00a7a980`
(`Native_5036`, mode 0) stores a character's attachment point into three
script variables. `Atel_ApplyPolarMove_008629c0` adds a polar step
(`Math_PolarToVector`) to a bound worker's position and moves its camera
(type 2) or re-binds it (types 4 and 5).
`round25e_turns_stream_game_over_seps_match_the_original` covers them (the
variables there avoid storage 5, the worker itself, and the field
snapshot's workers with the debug heap's `0xcdcdcdcd` as character handle
get one of the test's own).

Round 25 f: `Atel_SetEventWaterFlag_00865740(on)` updates the save
event's water byte `+0x28` and, on a change, the delayed separates and
water effects. `AtelUpdateWindowStatus` (`0x00871f30`) copies the window
system's statuses into the message windows (4 reads as 0; the status word
is the window's `+0x14`, the mode flags `+0x1d`), keeps the ctx's busy bit
(`+1` bit 4), counts the window countdown `+0x1ea` down and turns the UC
off and on again when either changes. `AtelUpdateBattleStatus`
(`0x00871bc0`) leaves ctrl work 0 current, measures the camera target's
step (`CIsqrt(Math_LengthSq3)`), feeds it to the walk abilities and, with
encounters allowed, to `MsBattleEncountExe`. `Atel_InitWorkerLevels_0086d300`
sets every level's three words and resets it; type-1 workers get move and
turn blocks (speed `0x5a`, pi, pi/64). `Atel_CheckLineCross_008680a0`
fires trigger 4 (or a vibration signal) when a target's step crosses the
worker's line at a small enough height gap.
`round25f_water_windows_battle_levels_lines_match_the_original` covers
them.

Round 26 a: `AtelBattleEndCallback` (`0x00861160`) brings the field back
after a battle. End type 6 resets the renderer (`PostReset(1)`) and fades
to opaque black; end type 1 without `g_AtelBattleEndStatus` bit 9 jumps
to the game over; anything else queues the return callbacks (`CAMERA`
unless status bit 14 in battle mode 1, then by the status's low byte,
jump table `0x00861408`: 1 nothing, 2 `MOVIE` with `+8`/`+0xc` copied to
`0x01326b38`, 3/4 the end effect `0x00861550`; then `SNDCLR`, `SNDREG`,
the map start `0x008615b0`, `CHRREG`, `EFFDONE` for kind 3, `DONE`, and
`DONE2` or, outside kinds 1..4 or mode 1, the menu reserve `0x40010002`
with message area 2). The status is copied to `+4` first and reset to
`0x10001` last; ctrl work 0 is current throughout and the caller's comes
back. `Atel_RunCtrlWorkers_00867950(event, input, clear)` runs every
worker of the current ctrl work once (`Script_Run(w, event +8, input)`;
without input only workers with `+0x36` bits 7-8), handles the talk
choice from the pad (`+0x1e4`, `+0x1e6`, `+0x1e8`; affection and trigger
for a chosen worker), decodes the signals (moving pending levels `+0x36`
bits 9-10 down to 7-8, or counting 7-8 down) and repeats while a result
has a bit outside 14-15; bit 15 sets ctx `+8` = 3. `AtelUpdateFilePath`
(`0x00879f00`) picks the event directory by `AtelGetEventMode` (jump
table `0x0087a2b8`) and `TkGetLang` (byte tables `0x0087a2f0`.. into
word tables `0x0087a2d0`..; the language order is jp, us, fr, es, de, it,
`0x12` uk, while the strings sit jp, us, de, fr, it, es, uk).
`round26a_battle_end_ctrl_pass_file_path_match_the_original` covers them
with every callee a logged stand-in; some stand-ins switch the ctrl work
or move the choice to show which reads the original repeats.

Round 26 b: `Atel_CheckPause_00873620` (caller `0x00872bd0`) decides, with
ctrl work 0 current, whether START opens the pause menu: the delay word
after `g_AtelPauseFlag` counts down, START held and pressed (`0x800` in
both pad words) with the delay at 0 sets the flag, and each blocker (ctx
bits 0 and 3, a movie, `0x0086d940` or ctx `+3` bits 0-1, a busy effect,
ctx `+3` bit 7 or `+1` bit 2, no actors, overlay status 1..3, `+0x210`
bits 4, 3 and 1) clears the flag and restarts the delay at 12 (at 0 for no
actors and `+0x210` bit 3). An HDD error with nothing blocking pauses and
sets the error flag. With `Movie_IsBusy` and no TGS flag a set flag
becomes `Sg_MenuSetReserve(0x41000000 | hdd)`, `+0x210` bit 2 and
`0x013270f4` = 1. `AtelInitTotal` (`0x0086d6d0`) resets the script
globals and registers the native tables (groups 1, 0, 12, 6, 9, 5, 4, 11,
7, 8, 13 after clearing 0..15). `Atel_MoveStep_00870540` (caller
`0x008666e0`) runs one frame of a level's move for the bound character:
the speed (`Ch_SetSp`) after a vertical part (`Ch_SetVelY` from `_CIsin`
/ `_CIcos` of the position block's `+0x24`; jump table `0x00870830` on
the move kind minus 2 in field mode 0) or a position (`TkCh_SetPos`),
then the move's end by kind (byte table `0x00870860` into `0x00870850`):
time below length for kinds 5-6 and 7-9, the radius `+0x38` otherwise.
Its `EAX` is a leftover (the x87 status word on some paths) that the
caller does not read. `round26b_pause_init_total_move_step_match_the_original`
covers them. `0x00882d50` and `0x00884d90` draw lines of the script
debugger's pages (`TkPutDebugFontString*`); they are reached only from
`AtelDebugDisplay_00883040`, which nothing calls, so they stay unported.

Round 26 c: `AtelBindElemOff` (`0x00862780`) frees bound element `i`
(`+0xaf4 + 4i`, `Ch_Dispose` when `Ch_CheckHandle` returns 0), shows the
character's model part 1 again for element 0 of a binding-7 worker and
clears bit `i` of `+0xb14` with a 16-bit shift (no bit from 16 up).
`Atel_CalcSpline_00862b20` and `Atel_CalcSpline3_00862b70` clamp `t` to
0..1 (NaN passes) and call `TkMtCalcSpline`/`TkMtCalcSpline3` on the
curve's points (`+0x10`, count `+0`). `Atel_ExecCtrlWorks_008688f0(from,
to)` (from `AtelExec` and `AtelMenuExec`) makes each ctrl work in turn
current (index in `0x01326b30`), runs the `+0x4c` callback of works in
state 0..2 with bit 0, counts `+0x1f4` for bit 3 and moves states 1 and 2
to 3. `Atel_CheckLineCrosses_00868020` runs the line check for the
worker's own target and its eight line records (`+0x2e4`).
`AtelEventGetFilePath2` (`0x008795b0`) builds `<event dir><sub>/xy/<base>/
<name><ext>` in `0x01328858` (`xy` the first two characters of the base,
`alt` cut at its `.` or else the cut name), or just `<name><ext>`.
`round26c_bind_splines_ctrl_works_lines_file_path_match_the_original`
covers them.

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

The oracle reports a call that runs out of its instruction budget
(200,000,000) as a runaway with the address it stopped at; it used to
show up as "EBX not preserved", because the registers had not been
restored. That was `ctrl_start_matches_the_original` on the battle
snapshot: worker 18 of ctrl work `0xf45ba0` is a field worker that
`Script_Run` skips in battle (`+0x34` bit 2 and `+0x36` bit 6 clear,
`MsBattleCheck` set), so it never sets its init bit and the start
callback's loop spins on it for ever, in the game as in the oracle. A
snapshot's ctrl works are past their start; the test now gives such
workers the init bit in both runs and compares the rest.

### Plan

1. Done: the accessors, the dispatch loop, every opcode, native dispatch,
   the signals (request families, `AtelExecReturn`, the waits, opcodes
   `79`/`7a`): `Script_Run` calls no original script code but natives.
2. Done for 1037 of the 1059 natives that `ffx script natives` lists. Not
   ported: 15 debug natives (marked in the table) and `0x7076`, which waits
   on `MsSetBallSpline`'s port. Six more run one or two slots in the original
   (their rows say which). The group-0 table at `0x00c50050` has 180 more
   entries that no script calls (179 with a Rust handler). Now: the VM's
   other helpers, by blocker count (`tools/worklist.py --module script`).
3. Then a `Natives` implementation with no original code behind it, so
   the interpreter runs outside the game.

## Open questions

- What calls `Script_LineActorSide` (`0x008632b0`), the reader of the
  `+0x2e4` table that opcode `7a` writes.
- The block header words `0x04`–`0x2c` and the tables they point to.
