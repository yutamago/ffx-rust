# Lua in FFX.exe

FFX.exe links Lua 5.2.1 (about 686 functions) through PhyreEngine's
PScripting layer. The game never runs it: no Lua state is created in a
normal session, and nothing in the image can reach the code that would
create one except two PhyreEngine component classes that the game's data
never instantiates. The rumoured "scripting harness" is unused engine code.

## Static picture

`lua_newstate` (`0x009506d0`) has five callers, and every Lua state the
game could ever have goes through one of them:

| Caller | What it is | Referenced by |
|---|---|---|
| `luaL_newstate` `0x0094d5b0` | Stock lauxlib: `lua_newstate(0x0094c5f0, 0)` then `lua_atpanic(L, luaL_panic)`; `luaL_panic` `0x0094ddb0` prints "PANIC: unprotected error in call to Lua API (%s)" | nothing (no call, jump or pointer) |
| `PScripting_NewState` `0x005d5350` | state, `0x009540f0`, Phyre types (`0x00436f70`), `Phyre_PScripting_RegisterFunctions` | nothing |
| `PScripting_NewStateForScript` `0x005d53b0` | as above, plus global `phyre_pscript` = the PScript object, and loads its compiled chunk | `PScriptedComponent_createThread` `0x00554860` (`0x005548b4`) and `PScriptedComponent_update` `0x00555f20` (`0x00555f7c`) |
| `PScripting_CompileScriptFile` `0x005d6120` | reads a file with `PStreamReaderFile`, runs it in a fresh state, sizes the result with `PScriptingCompiler_CalculateSize` (the asset compiler's path) | nothing |
| `PScripting_BuildStateImage` `0x00618150` | state with `PScriptMemoryAnalyzer`/`PScriptMemoryRecorder` allocators, `luaopen_base`, `luaopen_table`, Phyre types, `RegisterFunctions`, `lua_atpanic(Phyre_PScripting_Panic)`, `lua_gc(COLLECT)`; snapshots the heap | `PScripting_PScheduler_initialize` `0x005d0f00`, which nothing references |

`PScriptedComponent_initialize` (`0x00554830`, calls
`Phyre_PGameplay_PScriptableComponent_initialize` then `0x00554860`) is slot
1 of `Phyre::PGameplay::PScriptedComponent::vftable` (`0x00b2d410`, entry at
`0x00b2d414`) and of `PCameraControllerComponent::vftable` (`0x00b2db30`,
entry `0x00b2db34`), which inherits it. Those are the only two
references. Such components exist only if a loaded Phyre cluster contains
one; the class descriptors are registered with every other Phyre class.

The other script entry points have no callers and no pointers:
`ExecuteScriptString` (`0x004362b0`), `Phyre_PScripting_ExecuteScript`
(`0x005d5620`), `Phyre_PScripting_PScheduler_addOneShotScript`
(`0x005d0350`). The 13 callers of `lua_pcallk` (`0x0094b4d0`) are these, the
PScheduler/PScript execute paths, and Lua's own library code.

What a state would expose: `luaopen_base`/`luaopen_table` (only in the
`0x00618150` image), a `Phyre` metatable namespace with the vector and
matrix types (`0x00436f90`, `Phyre_PScripting_RegisterVectorMatrixTypes`
`0x005721b0`), and four globals from `Phyre_PScripting_RegisterFunctions`
(`0x005d6070`): `require` (`0x005d63a0`), `unrequire` (`0x005d67b0`),
`profile` (`0x005d6050`), `print` (`0x005d5c00`). No FFX game function is
bound: no script opcode, battle, field or menu call.

Data: no `.lua` names in the strings, and none of the 72,129 files in
`FFX_Data.vbf` and `metamenu.vbf` is Lua (`ffx vbf list`; the types are
`.phyre`, `.bin`, `.mgrp`, `.txt`, ...).

## Runtime check (2026-10-16)

Temporary wrapper hooks (count, log first and power-of-two hits with a
backtrace, call the original) on `luaL_newstate`, `lua_newstate`,
`lua_pcallk`, `0x00436f70`, `0x00554830`, `0x00554860`, `0x00555f20`,
`0x005d53b0`, `0x005d5350`, `0x005d6120`, `0x00618150`, `0x005d0f00`,
`0x004362b0`, `0x005d5620`, `0x005d0350`, plus a control probe on
`Phyre_PhyreMalloc_0042fb80`. Runs: boot, title, load, then the
`field-walk`, `battle-attack` and `menu-browse` replays. All 15 probes
installed. The control fired more than 65,000 times per run; every Lua
probe fired zero times.

## Verdict

There is no scripting harness in the remaster. Lua is PhyreEngine baggage
linked in with PScripting and the PGameplay components; FFX's logic runs on
the PS2 event/AI script VM (`Script_*`), not Lua. A user cannot feed it a
script: no state exists, no file is read, and no entry point is reachable.

For modding it is not worth pursuing. Making it run would need injected
code to create a state and call it, and the state could reach only Phyre
vector maths and `print`; every binding to the game would have to be
written first. The hook DLL already runs Rust in the process with full
access, so Lua would add a language and nothing else.
