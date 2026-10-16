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
elsewhere: 66 handlers (message windows, character motion and display
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
`Sg_LightSetColor`/`Sg_LightSetDir`. In the game those words hold whatever
earlier calls left at that depth, so the Rust interpreter (whose frames
differ) can store different garbage there; nothing reads it back as far as
the tests reach.

### Plan

1. Done: the accessors, the dispatch loop, every opcode, native dispatch,
   the signals (request families, `AtelExecReturn`, the waits, opcodes
   `79`/`7a`): `Script_Run` calls no original script code but natives.
2. Now: the natives, most used first. Done: the field module's ready
   ones (group 8, `0x01a5`, `0x01e4`, `0x01e6`) and the sprite, sound
   effect and stop natives. Next: the message natives (`Native_MsgOpen`,
   `Native_MsgSetText`, the waits), the camera group 6 (each starts by
   resolving the worker's camera handle like `Native_CamSetPos`), the
   move and turn starts, and `Native_SoundWait_Poll` /
   `Native_VoiceIsPlaying` once the oracle stubs `printf`.
3. Then a `Natives` implementation with no original code behind it, so
   the interpreter runs outside the game.

## Open questions

- The meaning of most native functions.
- Opcode `40` and what the `+0x2e4` table of opcode `7a` drives.
- The block header words `0x04`–`0x2c` and the tables they point to.
