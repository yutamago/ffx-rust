# The audio platform: what FFX.exe asks of FMOD

This is the census of M4's audio part (`docs/rewrite-plan.md`, M4: "Audio
(FSB/FEV decoding, mixer) ... rewritten in Rust in-process. FMOD ... no
longer used"). It lists every call `FFX.exe` makes into FMOD Ex
(`fmod_event.dll`, `fmodex.dll`), with its callers, its arguments and what
the game does with the answer, and what a Rust mixer has to reproduce. The
layers above FMOD (`Snd*`, `Sqs*`, the `i*` commands, the FMOD command
queue, `FfxFmod`'s layout) are in [`sound.md`](sound.md); this file starts
where `FfxFmod`, `FmodMusic`, `FmodVoice`, `FmodSfx`/`SfxBank`/`SfxEvent`,
`FmodShout` and the movie player call the libraries. Crate:
[`crates/ffx-audio-census`](../../crates/ffx-audio-census).

```bash
# the boundary from the image: 40 imports, their thunks and call sites
FFX_GAME_DIR=~/steam-games/FFX-X2-HD cargo run --release -p ffx-audio-census --example census
# a call log of a running game: original code, sound off, through the harness
FFX_TEST_VOLUME=0 FFX_FMOD_LOG=/mnt/c/.../fmod.log \
  target/release/examples/replay tools/replays/<replay>.txt '<trace>' original
# its totals per function (calls, callers, threads, common arguments, results)
cargo run --release -p ffx-audio-census --example census -- fmod.log
# the gate (skips without the variables)
FFX_GAME_DIR=~/steam-games/FFX-X2-HD FFX_FMOD_LOGS=<log>,<log> \
  cargo test --release -p ffx-audio-census
```

Sources: the disassembly and the dump of `FFX.exe`, the call logs below,
and `fmod_event.dll`'s export table (the names of its implementation
classes, `?getGroup@EventProjectI@FMOD@@...`). Where an FMOD number is given
a name (an event state, a mode bit, a callback type), the name is the FMOD
Ex 4.4x API's meaning of that number and is marked *(API)*; the number is
what the binary shows. Result codes match `FFX.exe`'s own
`FMOD_ErrorString` (`0x00707590`).

## The call log (`FFX_FMOD_LOG`)

`crates/ffx-hook/src/fmod_log.rs`. With `FFX_FMOD_LOG=<file>` set (a WSL
path; `tools/run-game.sh` hands it over with `/p`), the hook points every
`FFX.exe` import of the two DLLs (its import table slots) and every entry
of `fmod_event.dll`'s vtables that is a named export (211 entries in all:
the 40 imports and 171 `EventProjectI`, `EventGroupI`, `EventI`, ...
methods) at a logging stub. A call whose return address is in `FFX.exe` is
logged when it returns: the thread, the harness frame, the return address,
the arguments (as many words as the callee popped: the exact argument
count), the result, the word each pointer argument points at after the call
(out-values) and the string it pointed at before. FMOD's calls among its
own objects pass through unlogged. The stubs pass every register and stack
word through, so nothing else changes: the sound stays off with
`FFX_TEST_VOLUME=0`. Without the variable nothing is patched.

Write the log on the Windows disk (`%TEMP%`), not on the WSL share: on the
share a boot took 0.2 ms per call. The writer keeps a 1 MiB buffer and
flushes at most once a second, so the harness's `quit` (TerminateProcess)
loses up to the last second of calls.

Logs taken on 2026-10-25 (original code, `--hook-disable replacements`,
`FFX_TEST_VOLUME=0`; under `~/ffx-work/m4/captures/audio/`, never in git):

| Log | What ran | Calls |
|---|---|---|
| `battle-attack-a` | the boot without the graphics layer: test20, memochek, the attract demo (`loopdemo`), test20 again, 179,442 frames in fast mode (the replay itself failed: "the title scene did not come back", below) | 564,580 |
| `battle-attack-any-c` | the boot, the save in slot 1 (`mcfr0400`), a battle (field 3, group 0), two attacks: `battle-attack.txt` with the load line and the first `waitfor` relaxed (`~/ffx-work/m4/audio/replays/`), with `FFX_GFX=1` | 2,358 |

`event-besaid` could not run here: it loads save `ffx_003`, which this
machine's saves do not have, and without the graphics layer the boot
missed the title twice (`memochek` went on to `loopdemo`), with the log and
without it. See the open questions.

All the logged calls ran on one thread, the `FmodManager` thread (its start
address is the generic thread entry `Thread_Start_00679db0`).

Totals of `battle-attack-any-c` (1,705 frames; "/s" per second of the
log's wall time, in fast mode):

| function | calls | /s | frames | callers (return address xN) |
|---|---|---|---|---|
| `FMOD_Memory_GetStats` | 1125 | 121.1 | 0-1705 | 00707a1b x1125 (ERRCHECK) |
| `EventSystem::update` | 379 | 40.8 | 0-1704 | 00707170 x379 |
| `EventProjectI::getInfo` | 336 | 36.2 | 401-1705 | 0070fb81 x336 |
| `Event::setVolume` | 135 | 14.5 | 351-1705 | 00709e80 x65, 0070f182 x70 |
| `Event::getState` | 70 | 7.5 | 347-728 | 00708ec1 x61, 007097fd x4, 00709a18 x2, 00709b8a, 00709ee8, 0070a075 |
| `Event::getChannelGroup` | 44 | 4.7 | 381-1705 | 0070ec6b x1, 0070eeea x43 |
| `ChannelGroup::getNumChannels` | 44 | 4.7 | 381-1705 | 0070ec85 x1, 0070ef04 x43 |
| `ChannelGroup::getChannel` | 43 | 4.6 | 381-1702 | 0070ecb7 x1, 0070ef37 x42 |
| `Channel::setPan` | 42 | 4.5 | 381-1703 | 0070ef59 x42 |
| `EventProjectI::getGroup` | 36 | 3.9 | 0-1521 | 00708e32 x2, 0070a86e, 0070ab5b, 0070ffde x16, 007107fa x16 |
| `EventSystem::load` | 21 | 2.3 | 0-1520 | 00708ded x2, 0070a823, 0070ab11, 0070f7a8 x17 |
| `Event::setCallback` | 18 | 1.9 | 349-1679 | 0070982e x4, 00709ad6, 00709c33, 0070e89f x2, 0070f39f x10 |
| `Event::start` | 14 | 1.5 | 351-1679 | 0070984c x4, 0070f3bd x10 |
| `EventGroupI::getEventByIndex` | 14 | 1.5 | 345-1678 | 00708a9d, 0070915b x3, 007102c7 x5, 00710b41 x5 |
| `EventProjectI::release` | 8 | 0.9 | 401-451 | 0070f97d x8 |
| `EventProjectI::cancelAllLoads` | 8 | 0.9 | 401-449 | 0070f95f x8 |
| `Event::getPaused` | 4 | 0.4 | 381-729 | 0070921b, 00709a56 x2, 00709bc8 |
| `Event::getVolume` | 2 | 0.2 | 397-399 | 007091f9, 0070f2a7 |
| `Event::stop` | 2 | 0.2 | 381-730 | 00709af6, 00709c53 (both `false`) |
| `EventGroupI::freeEventData` | 2 | 0.2 | 381-734 | 00708786, 00708b4d (both `wait = true`) |
| `Event::getInfo` | 1 | | 1621 | 0070ed18 |
| `Event::setPaused` | 1 | | 399 | 00709a87 (`true`) |
| `Channel::getLoopCount` | 1 | | 1621 | 0070ecd6 |
| the start-up calls, once each | | | 0 | `FMOD_Memory_Initialize`, `FMOD_EventSystem_Create`, `EventSystem::getSystemObject`, `System::setStreamBufferSize` (0x10000, 8), `EventSystem::init` (128, 0, NULL, 0), `System::setFileSystem`, `System::createSound` (the Shout FSB, mode 0x200), `Sound::getNumSubSounds` |

What the logs show beyond the static census:

- **Every result was 0** (`FMOD_OK`) in both logs, apart from one
  `EventSystem::load` of an SFX project answering 0x17 (file not found)
  in each (`0x0070f7a8`, during the first frames): the game asks for an SFX
  project that is not in the data and prints "Fail to create SFX
  project".
- **The steady calls.** Each FMOD tick runs `EventSystem::update`, and each
  `ERRCHECK` (`0x007079d0`) calls `FMOD_Memory_GetStats` (to log a new
  maximum), so GetStats runs three times per update; each tick also asks
  every loaded `SfxBank` for its playing events
  (`SfxBank_getNumPlayingEvents` `0x0070fb20`, `EventProject::getInfo`).
- **The FMOD tick follows the game clock.** `FmodManager_tick`
  (`0x006fa700`) measures its 33.333 ms periods with
  `graphicGetTimeInMicroseconds`, the clock the harness makes virtual: in
  fast mode the boot log ran 94,032 updates in 123 s of wall time (771 a
  second), about one per 1.9 frames. With the virtual clock the FMOD side
  steps with the frames, not with the wall.
- **Volumes are all 0** with the sound off: all 135 `Event::setVolume`
  calls passed 0.0 (master 0 x anything). The volume formulas below are
  from the code, not from the log.
- **Music**: `getEventByIndex` at `0x0070915b` (`FmodMusic_read`) with
  the event indexes 0 (boot), 0x15 and 0x1a (the field and the battle), and
  6 at `0x00708a9d` (`FmodMusic_dataSend`), then `getState` polls (`== 1`, then
  `& 8`; seen state values 0x18 = PLAYING | CHANNELSPLAYING *(API)*) and a
  `setVolume` per tick while a translation runs (`0x00708ec1`/`0x00709e80`,
  61 and 64 calls).
- **SFX**: banks load as `SFX/US/<nnnn>.fev`; each sound starts as
  `getEventByIndex` + `setCallback(eventcallback)` + `start` per event of the
  sequence (a looping sound is several events), then `setVolume` and the
  channel pan (`getChannelGroup`, `getNumChannels`, `getChannel(0)`,
  `setPan`). Pans seen: -0.11811024 = (56/127)*2 - 1 and -0.007874016 =
  (63/127)*2 - 1, the game's 0..127 pan.
- **Out-values**: `EventProject::getGroup` hands back the group, `load` the
  project, `getState` the state; event handles are small integers with the
  low bit set (`0x540001`, `0x2a480001`), not pointers.

## Threads

| Thread | FMOD calls | Evidence |
|---|---|---|
| **FmodManager** | every FMOD Designer call and the game's own fmodex calls (the Shout FSB), start-up and shut-down included | created by the FmodManager ctor (`0x006f9de0`; the free-standing `FmodManager_createThread` `0x006fa130` has no caller), which first queues command 0x2a (`initEventSystem`) and 0x2b, so even `FMOD_Memory_Initialize` runs on it. `0x006f2e10(params, "FmodManager")` -> `0x00679cb0`: CreateThread, stack 0x10000, suspended, named by exception 0x406d1388; ResumeThread; `SetThreadPriority(h, 300)` (`0x006f3050`; 300 is no `THREAD_PRIORITY_*` value, so Windows refuses it and the thread stays at normal priority: from the Win32 rules, not checked). Body `FmodManager_threadMain` (`0x006fa1b0`): `while (mgr+0x4c) FmodManager_tick`, then a last queue drain, `FfxFmod_shutDown`, `mgr+0x4d = 1`. The tick waits on the event `mgr+0x18` (alertable) for the rest of a 33.333 ms period of game time, then runs the queue; once a period has passed it first runs `FfxFmod_soundLoop` (`EventSystem::update` when more than 33,000 us passed since `0x00cec168`, the objects' updates, `FmodSfx_update(seconds)`). Every call in both logs ran here |
| callers of the sync commands (0x11-0x13, 0x16, 0x19, 0x41, `sound.md`) | none | they read fields only: `0x007074e0` (`FmodVoice +0x44 | +0x40`), `0x007074b0` (`+0x54`), `0x007067d0` (`FmodMusic +0x18`), `0x00709060`, `0x00706b30`, `0x00706e10`. Sync 0x14 (`syncStandby`, `Event::getState`) runs on the FMOD thread |
| SFX loading | on the FMOD thread | `iSfxLoadData` queues 0x30 -> `FfxFmod_sfxLoad` `0x00706e20` -> `FmodSfx_loadProject` `0x0070cf10` -> `SfxBank` `0x0070f760`: a blocking `EventSystem::load`. There is no loader thread and no non-blocking mode |
| the movie player's open, pause, volume and close | fmodex only: `createSound`, `playSound`, `Channel::setPriority`, `Sound::setUserData`, `Channel::stop/setPaused/setVolume`, `Sound::getOpenState`, `Sound::release` | the game's side (`PhyFMVPlayerManager` `0x006d8fc0`/`0x006da9b0`, `PhyreScene` `0x0066fb70`), not the FMOD thread; on the same `System` as the FMOD thread, with no lock of the game's (FMOD Ex's own thread safety *(API)*). No movie ran in the logs |
| FMOD's own threads (mixer, stream, file) | none | they call back into the game: the file callbacks (streams and loads) and the movie's PCM read callback (`0x00a27ee0`, on FMOD's stream thread *(API)*) |

## The imports

`FFX.exe` imports 17 functions of `fmod_event.dll` and 23 of `fmodex.dll`
(import slots `0x00b0c734..0x00b0c7d4`). Every one goes through one
`jmp dword ptr [slot]` thunk (`0x00948c70..0x00948d36`, and
`0x00a7c8f2..0x00a7c910` for six the movie player uses); `refs.py` and
`ffx_audio_census::boundary` find one thunk per slot and no other use of a
slot. All are `__stdcall`, members with `this` as the first stack word.
"line N" is `ERRCHECK(result, file, N)` (`0x007079d0`: prints
`FMOD error! (%d) %s` for a non-zero result and logs a new
`FMOD_Memory_GetStats` maximum); the files are `FfxFmod.cpp`,
`FmodMusic.cpp`, `FmodVoice.cpp`, `FmodShout.cpp`, `SfxEvent.cpp`,
`SfxProject.cpp`. An event *state* is the `u32` of `Event::getState`; the
game tests `& 8` (PLAYING *(API)*), `== 1` (READY *(API)*: loaded, not
playing) and `== 2` (LOADING *(API)*).

### fmod_event.dll

| Import | Signature | Call sites | Arguments | The answer, and what the call means |
|---|---|---|---|---|
| `FMOD_EventSystem_Create` | `(EventSystem**)` | `0070684c` FfxFmod_initEventSystem; `007066a6` (wrapper `0x007066a0`, no callers) | `&FfxFmod+4` | creates the event system; line 0x9b; a NULL system prints "EventSystem create failed!" and stops the init |
| `EventSystem::init` | `(int maxchannels, uint flags, void* extradriverdata, uint eventflags)` | `007068ca` | `(0x80, 0, NULL, 0)`: 128 channels, default flags *(API)* | line 0xab. Initialises the low-level System too (no `System::init` import) |
| `EventSystem::getSystemObject` | `(System**)` | `00706884` | `&FfxFmod+8` | the low-level System, also the movie player's; line 0xa4 |
| `EventSystem::release` | `()` | `00707058` FfxFmod_shutDown | after `unload`, 100 ms sleeps around it (`0x006f2d20`) | line 0xcd |
| `EventSystem::unload` | `()` | `00707036` FfxFmod_shutDown | after the voice, music and SFX shut-downs, each followed by 100 ms | line 0xc9 |
| `EventSystem::update` | `()` | `0070716b` FfxFmod_soundLoop; `00707432` FfxFmod_soundUpdate (command 2) | only with `FfxFmod+4` set; the loop only after 33,000 us since `0x00cec168` | advances FMOD: event state, fades, stream feeding, callbacks; line 0x121 |
| `EventSystem::load` | `(const char* name, FMOD_EVENT_LOADINFO*, EventProject**)` | `00708de8` FmodMusic_loadEventProject; `0070a81e` FmodVoice_dataChange; `0070ab0c` FmodVoice_initData; `0070f7a3` SfxBank ctor `0x0070f760` | the name from `getFullFilename` (`0x00707aa0`): `../../../FFX_Data/GameData/PS3Data/Sound_PC/` + the project (logged); load info always NULL. Music `Music/ffx_music.fev` -> `FmodMusic+8`, `Music_PS2/ffx_music_PS2.fev` -> `+0xc` (`0x00708c00`); voice `Voice/US|JP/<project>.fev` -> `projects[i]` (`FmodVoice+0x18`, 21 slots), the battle voices `Voice/US|JP/ffx_us|jp_voice_btl.fev` -> `+0x20`; SFX `SFX/US|JP/%04d.fev` -> `SfxBank+0` | loads an FEV project (blocking; through the file callbacks below). Music line 0x76 ("NULL == mEventProject!"), voice 0x22f/0x148; SFX unchecked: 0 logs "Load Fmod Project", else "Fail to create SFX project" (0x17 seen in the log) |
| `Event::start` | `()` | music `0070931c` restart, `00709847`, `007098c9` start; voice `0070b251` restart, `0070b633` start; SFX `0070f3b8` SfxEvent_start | always right after `setCallback(eventcallback, &state word)`; music and voice only in state `== 1` | starts the event; lines 0x22c, 0x1a8, 0x1c5, 0x357, 0x386, 0x88 |
| `Event::stop` | `(bool immediate)` | music `00709af1` stop, `00709c4e` stopAll; voice `0070b700`; SFX `0070f518` SfxEvent_stop | music and voice `false` (fade by the event's fade-out *(API)*), after `setCallback(NULL, NULL)`; SFX `true` when the caller asks or the first instance's volume is 0 (`ftol(getVolume) == 0`, `0x0070f4dc`), else `false`. SfxEvent_stop's callers: `0x0070d190` playBattleStreaming 0, `0x0070d780` shutdown 1, `0x0070d8f0` stop 0, `0x0070d930` stopAllNonBlocking (argument), `0x0070d9e0` stopAllNonloop 1, `0x0070da90` stopAllSound (argument) | lines 0x20d, 0x2b7, 0xac, 0xb1 |
| `Event::getState` | `(uint* state)` | music `00708ebc` loopVolumeTranslate, `007092cb` restart, `007097f8` start, `00709a13` stop, `00709b85` stopAll, `00709d97` syncStandby, `00709ee3` volumeFadeOut, `0070a070` volumeTranslate; voice `0070a4b8` (`0x0070a4a0`), `0070ad2e` loopVolumeTranslate, `0070b182` restart, `0070b397` setPaused, `0070b5ea` start, `0070b6d1` stop, `0070b80d` volumeDirect, `0070b8d9` volumeTranslate | | `& 8` gates fades, stops and pauses; music start: `== 1` starts, else `& 8` with `FmodMusic+0x28 == 0x79` restarts, else start pending (`+0x1c`); `syncStandby` (command 0x14) answers 1 for `r == 0 && state == 2` (music 0x10 still loading). Voice uses the result: non-zero (the event is gone) frees the event data and reads a pending voice again; restart retries up to 5 times. Logged states: 0x18 |
| `Event::setCallback` | `(FMOD_EVENT_CALLBACK, void* userdata)` | music `007092fe`, `00709829`, `007098ab` -> `(eventcallback, &record+0x20)`, `00709ad1`, `00709c2e` -> `(NULL, NULL)`; voice `0070b236`, `0070b618` -> `(eventcallback, &FmodVoice+0x40)`, `0070b6ec` -> `(NULL, NULL)`; SFX `0070f39a` -> `(eventcallback, &instance+4)`, `0070e89a` (SfxEvent release `0x0070e850`) -> `(NULL, NULL)` per instance | `eventcallback` `0x00707a50` | the only way the game learns that an event ended (below); lines 0x229, 0x1a5, 0x1c2, 0x20c, 0x354, 0x383, 0x87 |
| `Event::setVolume` | `(float)` | music `007094c4` setEntireVolume, `007095bc` setMute, `00709e7b` volumeDirect, `00709fb4` volumeReset; voice `0070b867` volumeDirect; SFX `0070f17d` SfxEvent_setVolume `0x0070f140` | music `record+0x38 x FmodMusic+0x30`, `record+0x38 = v/127` (0 when muted, `+0x24 == 1`); voice `(FmodVoice+0x80 == 1 ? 0 : v) / 127.0 x +0x88` (f64 127 at `0x00b922e8`), only in state `& 8`; SFX `SfxEvent+0x24` (entire, `0x0070ee90`) `x +0x20` (requested 0..1) per instance with its flag set | the event's volume, 0..1; lines 0x33a, 0x326, 0x12e |
| `Event::getVolume` | `(float*)` | `007091f4` FmodMusic_releaseOldMusic; `0070b917` FmodVoice_volumeTranslate (-> `+0x38`); SFX `0070ec22` (`0x0070ec00`, no callers), `0070f092` setPan2, `0070f2a2` setVolume2, `0070f4b9` stop | first instance only for SFX | the start of a translation; releaseOldMusic fades out (0x12 frames) every other playing, unpaused music with volume > 0; setPan2 drops it (the pan translation starts from 0.0, `0x0070f000`); lines 0x259, 0x307, 0x10f |
| `Event::setPaused` | `(bool)` | `00708974` crossPlay `false`; `00708feb` pause, `0070903f` pauseAll `arg != 0`; `00709a82`, `00709bf0` stop/stopAll `true` (a replay point `record+4 == 1` and not paused: pause instead of stop); `00709aaf`, `00709c0c` `false` (unpause before stopping); `0070b3cc` FmodVoice_setPaused `x == 1` (when x changed and state `& 8`); `0070f123` SfxEvent_setPaused `0x0070f100` (`FmodSfx+0` byte, every instance) | | pauses keep the position (the replay point); lines 0x278, 0x3b7, 0x201, 0x20a, 0x27d, 0x282, 0xbd |
| `Event::getPaused` | `(bool*)` | `00709216` releaseOldMusic; `00709a51` stop; `00709bc3` stopAll | | lines 0x25b, 0x1fa |
| `Event::getInfo` | `(int* index, char** name, FMOD_EVENT_INFO*)` | `0070b06d` FmodVoice_read; `0070eb67` (`0x0070eb40`, no callers); `0070ed13` SfxEvent_isPlaying | `(NULL, NULL, info)`, info 13 zeroed words (`0x0070a140`) | voice: `info+8` (lengthms *(API)*) -> `FmodVoice+0x6c`, the timeout of `0x0070b750` (elapsed ms > length clears the voice), line 0x429; SFX: `info+0x28 == 1` (numinstances *(API)*) |
| `Event::getChannelGroup` | `(ChannelGroup**)` | `0070ebb4` (`0x0070eb90`, SfxEvent getPan, no callers); `0070ec66` isPlaying; `0070eee5` SfxEvent_applyPan `0x0070eeb0` | first instance / each instance with its flag set | the event's channels, for the pan; line 0x11b |

### fmodex.dll

| Import | Signature | Call sites | Arguments | The answer, and what the call means |
|---|---|---|---|---|
| `FMOD_Memory_Initialize` | `(void* pool, int poollen, alloc, realloc, free, FMOD_MEMORY_TYPE)` | `00706839` initEventSystem (the first FMOD call); `007066c5` (wrapper `0x007066b0`, no callers) | `(NULL, 0, FmodMemory_alloc 0x007066d0, FmodMemory_realloc 0x00706730, FmodMemory_free 0x00706710, 0xffffffff)` | FMOD allocates from the game's user heap (below); unchecked |
| `FMOD_Memory_GetStats` | `(int* current, int* max, bool blocking)` | `00707a16` ERRCHECK; `0070680e` (`0x007067f0`), `00707a8e` fmodMemStats (no callers) | `(&cur, &max, false)` | ERRCHECK logs a new maximum (`0x00cec17c`); a debug statistic, three calls per FMOD tick |
| `System::setFileSystem` | `(open, close, read, seek, asyncread, asynccancel, int blockalign)` | `00706906` | `(FmodFile_open 0x00708510, FmodFile_close 0x007084d0, FmodFile_read 0x007085a0, FmodFile_seek 0x007085e0, NULL, NULL, 0x800)` | every FMOD file goes through the game's file layer (below); line 0xb1 |
| `System::setStreamBufferSize` | `(uint size, FMOD_TIMEUNIT)` | `007068a5` (before `init`) | `(0x10000, 8)`: 64 KiB, unit RAWBYTES *(API)* | line 0xa7 |
| `System::createSound` | `(const char*, FMOD_MODE, FMOD_CREATESOUNDEXINFO*, Sound**)` | `0070e38c` FmodShout_initData; `00a275f9` movie `PVideoPlaybackWin32_AudioInit_00a273b0` | Shout: `(Voice/US|JP/ffx_us|jp_voice_btl_iop_bank00.fsb, 0x200, NULL, &shout+0)`, 0x200 = CREATECOMPRESSEDSAMPLE *(API)* (logged); an older one is released first (line 0x3b). Movie: `(NULL, 0x4c8, &exinfo, &player+0x29c)`, 0x4c8 = OPENUSER | CREATESTREAM | SOFTWARE | 2D *(API)* | Shout line 0x4a ("NULL == mShoutFsb!"); movie: non-zero frees the packet ring and fails |
| `System::playSound` | `(FMOD_CHANNELINDEX, Sound*, bool paused, Channel**)` | `0070e632` FmodShout_start; `00a2761d` movie AudioInit | Shout `(-2, subsound shout+4, false, &shout+8)`, -2 = CHANNEL_REUSE *(API)*; movie `(-1, sound, false, &player+0x2a0)`, -1 = CHANNEL_FREE *(API)* | Shout line 0xcc; movie: 0 -> `setPriority` |
| `Sound::release` | `()` | `0070e322` FmodShout_initData (reload); `0070e461` (`0x0070e450`, FmodShout release, from FmodVoice_shutdown `0x0070b480`); `00a2793f` PVideoPlaybackWin32_AudioShutdown | movie: after `getOpenState` answers READY | lines 0x3b, 0x85 |
| `Sound::getSubSound` | `(int, Sound**)` | `0070e57e` FmodShout_start | `(i, &shout+4)` for each of the `shout+0xc` subsounds | line 0xb5 |
| `Sound::getName` | `(char*, int)` | `0070e5a4` FmodShout_start | `(buffer, 0x100)` | compared with the ML table's name for the shout id; the match is played; line 0xb8 |
| `Sound::getNumSubSounds` | `(int*)` | `0070e3cb` FmodShout_initData | `&shout+0xc` | line 0x54 |
| `Sound::getOpenState` | `(FMOD_OPENSTATE*, uint*, bool*, bool*)` | `00a27925` movie stop; `00a27c6f` (`0x00a27c50`); `00a27dcf` (`0x00a27db0`) | `(&state, NULL, NULL, NULL)` | stop spins (`Sleep(1)`) until `r == 0 && state == 0` (READY *(API)*); the other two count READY (or no sound) as "audio done", then ask whether the packet ring is empty (`0x00a3dde0`) |
| `Sound::setUserData` | `(void*)` | `00a27605` movie AudioInit | the player | unchecked |
| `Sound::getUserData` | `(void**)` | `00a27ef2` movie PCM read callback | | 0 and a player: decode |
| `Channel::setVolume` | `(float)` | `0070e436` FmodShout_setVolume; `00a28340` PVideoPlaybackWin32_setVolume | Shout: the voice entire volume `FmodVoice+0x88` (from `0x0070b2b0`, `0x0070b430`); movie: its f32 argument (callers `0x006d7430`, `0x006d8fc0` and `0x006da9b0` with 0, `0x006dabd0`, `PhyFMVPlayerManager_SetVolume_006dabf0` with iSoundUpdate's master 0..1) | Shout line 0xd9; the movie returns `r == 0` |
| `Channel::getVolume` | `(float*)` | `0070e419` FmodShout_setVolume | the value is dropped: a liveness probe, `setVolume` follows only for 0 | |
| `Channel::setPan` | `(float)` | `0070ef54` SfxEvent_applyPan | the SFX pan -1..1 (`v/127 + v/127 - 1` clamped, `0x0070ef80`) on every channel of the event's ChannelGroup | line 0x146 |
| `Channel::getPan` | `(float*)` | `0070ebea` (`0x0070eb90`, no callers) | channel 0 | |
| `Channel::getLoopCount` | `(int*)` | `0070ecd1` SfxEvent_isPlaying | | line 0x160; `== -1` (loops forever) |
| `Channel::setPaused` | `(bool)` | `00a27eb2` `true` and `00a27ed6` `false` (PVideoPlaybackWin32_togglePause, state `+0xe4` 1 -> 2 and 2 -> 1); `00a28307` `false` (resume `0x00a282f0`, outside every dump function) | | unchecked |
| `Channel::stop` | `()` | `00a2790a` movie stop | `player+0x2a0` | unchecked |
| `Channel::setPriority` | `(int)` | `00a27629` movie AudioInit | 0 (playSound's FMOD_OK in eax): the highest priority *(API)* | unchecked |
| `ChannelGroup::getNumChannels` | `(int*)` | `0070ec80` isPlaying; `0070eeff` SfxEvent_applyPan | | lines 0x158, 0x13f |
| `ChannelGroup::getChannel` | `(int, Channel**)` | `0070ebd5` (index 0); `0070ecb2`, `0070ef32` (each index) | logged: always index 0 (the SFX events have one channel each) | lines 0x11e, 0x15d, 0x144 |

`SfxEvent_isPlaying` (`0x0070ec40`) answers 1 only for an event whose
`getInfo` says one instance and whose channel loops forever
(`getLoopCount == -1`), and the value is 1.

## Virtual calls

The game calls two FMOD interfaces through their vtables, `EventProject`
and `EventGroup`; everything else goes through the imports. No import
returns an `EventCategory`, `EventParameter`, `EventReverb`, `EventQueue`
or `MusicSystem`, and the game has none. The slots are confirmed in
`fmod_event.dll`'s `.rdata`, whose runs of pointers to the exported
`?...@<Class>I@FMOD@@` methods are the vtables (`EventProjectI` at
`0x100508c0`, `EventGroupI` at `0x100500c8`; the call log resolves each
patched slot to its export's name), and by the logs.

- `EventProjectI`: +0 `release`, +4 `getInfo`, +8 `getGroup`, +0xc
  `getGroupByIndex`, +0x10 `getNumGroups`, +0x14 `getEvent`, +0x18
  `getEventByProjectID`, +0x1c `getNumEvents`, +0x20 `loadSampleData`,
  +0x24 `stopAllEvents`, +0x28 `cancelAllLoads`, +0x2c `setUserData`, +0x30
  `getUserData`, +0x34 `getMemoryInfo`.
- `EventGroupI`: +0 `getInfo`, +4 `loadEventData`, +8 `freeEventData`, +0xc
  `getGroup`, +0x10 `getGroupByIndex`, +0x14 `getParentGroup`, +0x18
  `getParentProject`, +0x1c `getNumGroups`, +0x20 `getEvent`, +0x24
  `getEventByIndex`, +0x28 `getNumEvents`, +0x2c `getProperty`, ... +0x44
  `getMemoryInfo`.

| Method | Sites | Arguments | What it means |
|---|---|---|---|
| `EventProjectI::getGroup` | `00708e2f` FmodMusic_loadEventProject; `0070a86b` FmodVoice_dataChange; `0070ab58` FmodVoice_initData; `0070ffdb` (`0x0070ff50`, SfxBank nonloop map); `007107f7` (`0x00710780`, SfxBank loop map) | `("music", false, &FmodMusic+0 / +4)`; `("voice", false, &groups[i])`; `("voice", false, &FmodVoice+0x1c)`; `("nonloop", true, &map)`; `("loop", true, &map)` (all logged) | the group by name; cacheevents only for SFX. SFX: a failure maps group 0, else reads `%04d_common.txt` (`0x00710370`) / `%04d_loop.txt` (`0x00710c50`); lines 0x80, 0x238, 0x152 |
| `EventProjectI::getInfo` | `0070fb7e` SfxBank_getNumPlayingEvents `0x0070fb20` | `(&info)`, 0x11c bytes, `+0` and `+0x104..+0x118` zeroed | answers `info+0x114` (numplayingevents *(API)*); unloadProjectBySeq `0x0070dbb0`, unloadProjects `0x0070dd10` and FmodSfx_update `0x0070dec0` delete a bank only at 0. Twice per tick per loaded bank in the logs; line 0x74 |
| `EventProjectI::stopAllEvents` | `0070fcdf` SfxBank_stopAllEvents `0x0070fcd0` | `(immediate)`: 1 from FmodSfx_shutdown `0x0070d780`, 0 from unloadProjectBySeq | unchecked |
| `EventProjectI::cancelAllLoads` | `00708d78` FmodMusic_loadEventProject (old project); `00709657`, `007096a7` FmodMusic_shutdown; `0070aa86` FmodVoice_initData (reload); `0070b0ee` FmodVoice_releaseData; `0070b4b7`, `0070b529` FmodVoice_shutdown; `0070f95c` SfxBank dtor `0x0070f930` | `()` | always before `release`, 100 ms apart; lines 0x6d, 0xbb, 0xc8, 0x137, 0x25d, 0x1b6, 0x1c6, 0x48 |
| `EventProjectI::release` | `00708d90`; `00709681`, `007096d1`; `0070aaa1`; `0070b127`; `0070b4ee`, `0070b549`; `0070f97b` | `()` | unloads the project; lines 0x6e, 0xc2, 0xcf, 0x138, 0x265, 0x1be, 0x1cc, 0x4c |
| `EventGroupI::getEventByIndex` | `00708a9a` FmodMusic_dataSend; `00709158` FmodMusic_read; `007102c4` (`0x00710290`, SFX nonloop); `00710b3e` (`0x00710ae0`, SFX loop) | music `(6, 0, &record[0x10].event)` and `(table[id], 0, &record[id].event)` (index from `0x00708be0`, table `0x00b4f1c8`; skipped while `0x00cec174 == 1`); SFX `(index from the map, 0, &event)`, every index of a loop record | an event instance, mode 0 (default, blocking *(API)*); music line 0x114, 0x59 (EVENT_NOTFOUND) prints "mEvent[%d] not find!"; SFX unchecked |
| `EventGroupI::getEvent` | `0070af27` FmodVoice_read (group `k` from the project name's last digits); `0070af81` (battle group `+0x1c`: ids from 0x33000001, and 0x8341504, 0xc545504, 0x28745504, 0x28746504, 0x504) | `(name, 0, &FmodVoice+0x14)`, the voice's event name from `VoiceIDMapper.txt` | lines 0x40a, 0x3f5; 0x59 prints "voiceRead: mVoiceEvent not find!" |
| `EventGroupI::loadEventData` | `0070a9af` FmodVoice_dataChange | `(1, 0)`: resource STREAMS *(API)*, mode default | pre-opens the streams of three projects only: change 0x170 with `ffx_jp_voice01`/`ffx_us_voice01`, 0x10e with `ffx_jp/us_voice270`, 0x7f with `ffx_us_voice08` (`0x0070a876..`); line 0x23f |
| `EventGroupI::freeEventData` | `00708783` FmodMusic_clear, `00708873` clearAll, `00708b4a` forceClearAll; `0070a4f9` (`0x0070a4a0`, when getState fails); `0070a5bb` FmodVoice_clear (up to 6 tries); `0070b1c9` FmodVoice_restart (after 5 failed getStates) | `(event, true)` (logged) | frees one event's instance and data, waiting until ready; lines 0x15d, 0x355, 0xb4, 0x297, 0x33c |

`FmodMusic`: `+0` remastered group, `+4` PS2 group, `+8` `ffx_music`,
`+0xc` `ffx_music_PS2`, `+0x2c` BGM type (1 picks `+4`/`+0xc`; `switchBGM`
`0x00709c80` stops, clears, reads and restarts). `FmodVoice`: `+0xc` the
group array and `+0x18` the project array (21 each), `+0x1c`/`+0x20` the
battle group and project, `+0x68` its `FMOD_EVENT_INFO`.

## Callbacks into the game

**Memory** (`FMOD_Memory_Initialize`, all `__stdcall`):
`FmodMemory_alloc` `0x007066d0(size, type, source)` takes `user_malloc(size
+ 16)` (`0x00687020`, the user heap), aligns it to 16 and keeps the pad at
`ptr-1` (size 0: NULL); `FmodMemory_realloc` `0x00706730` makes a new block
and copies `min(size, old size - pad)`; `FmodMemory_free` `0x00706710`
frees `ptr - ptr[-1]`.

**Files** (`System::setFileSystem`, block align 0x800): every file FMOD
opens (FEV, FSB, the streams, the Shout FSB) goes through the game's Phyre
file layer and so the VBF archive (and the hook's mods).
`FmodFile_open` `0x00708510(name, unicode, filesize*, handle*,
userdata*)`: a PStreamFile on `name + g_00c3a480` (`0x00607bc0`; the
offset is 0, set by `0x00708610` at init), `*filesize` from
`PhyreFIOSStreamFile_getFileSize`, 0x17 (file not found) when it does not
open; `FmodFile_close` `0x007084d0`; `FmodFile_read` `0x007085a0`, 0x16
(end of file) on a short read; `FmodFile_seek` `0x007085e0`, from the
start. A NULL handle answers 0x25. No asynchronous reads.

**Event end** (`eventcallback` `0x00707a50(event, type, p1, p2,
userdata)`): for type 2 or 4 (SOUNDDEF_END, EVENTFINISHED *(API)*) it
writes `*(u32*)userdata = 0`, and always answers 0. The words are the
music record's play state (`+0x20`), the voice's play status
(`FmodVoice+0x40`) and an SFX instance's active byte (`+4`): a Rust mixer
must clear them at the same point of the same tick the event ends at.

**Movie audio** (PVideoPlaybackWin32, Vorbis in WebM; `ffx_pcui::video`
already ports the decoder side, `PVideoPlaybackWin32_AudioRead_00a27170`):
`AudioInit` (`0x00a273b0`) fills an `FMOD_CREATESOUNDEXINFO` (0x88 bytes,
zeroed): `cbsize` 0x88, `length` = round(duration ns x 1e-9 x rate x
channels x 2) bytes, `numchannels`, `defaultfrequency` (the track's),
`format` 2 (PCM16), `decodebuffersize` 0x1000, `pcmreadcallback`
`0x00a27ee0`; then `createSound(NULL, 0x4c8, ...)`, `setUserData`,
`playSound(CHANNEL_FREE, ..., false)`, `setPriority(0)`. The read callback
ignores the length FMOD asks for and always writes 0x1000 frames of
16-bit PCM (`floor(x * 32767 + 0.5)`, clamped). The movie shares the game's
System (`player+0x298` from `FfxFmod+8`).

## What the Rust audio platform must do

What the game sees of FMOD is small: events created by index or name from
loaded projects, started, stopped (now or by fade), paused, given a volume
(0..1) and, for SFX, a pan per channel (-1..1); their state polled
(`& 8`, `== 1`, `== 2`); their end reported through one callback. Plus one
compressed sample bank with named subsounds (the Shout FSB) played on one
reused channel, and one user PCM stream (movies). No 3D, pitch, reverb,
DSP, event parameters, categories or music system: positional sound is
the game's own pan and volume (`Snd3dFootRequest`, `sound.md`), and the
master, music, voice and SFX volumes are multiplied in by the game.

To give the original's output:

- **Data.** The FSB5 banks (`sound.md`): IMA ADPCM, Xbox layout,
  everywhere. On the US data: music 89 tracks, all stereo, 82 at 44,100 Hz
  and 7 at 48,000, 76 with a loop chunk whose end is not the last frame
  (real loop points, e.g. `1_09_battle_theme` 1014349-4468084 of 4527424
  frames); voice 8,839 samples, 8,733 mono 48,000 Hz, 100 mono 32,000, 6
  stereo 44,100, 8,621 with a loop chunk (nearly all whole-sample);
  SFX 7,620 samples at 48,000 Hz, 2,371 mono and 5,249 stereo, 767 with a
  loop chunk. The FEV projects decide which of these loop, stream or are
  loaded whole, the events' volumes, fades and instance limits: the
  `LGCY` body (`sound.md`) must be decoded for that (open question 1).
- **Voices.** `EventSystem::init` asks for 128 channels; the game never
  sets priorities on events. FMOD's voice stealing beyond 128 is part of
  the output only if the game ever plays that many (it limits SFX to 5
  instances a sequence, 30 for sequence 114).
- **Streams.** The stream buffer is 64 KiB; voice projects pre-open their
  streams for three projects only. Music and voice events are read
  (`getEventByIndex`/`getEvent`) and freed per id. The game polls state
  `== 2` (loading) only in `syncStandby`; a Rust stream that is never
  "loading" changes when that answers 1, and so the script's waits.
- **Timing.** Event state changes become visible to the game at
  `EventSystem::update`, once per FMOD tick (33.333 ms of game time, on
  the FmodManager thread), and the end callback runs inside it. The voice
  scenes keep time with the voice stream (`docs/harness.md`, event-cliff),
  and the boot's opening with its music: their timing is the mixer's
  playback position, so a Rust mixer has to advance on the same clock
  (the game's) at the output rate FMOD uses.
- **Volumes and pans** as the floats the game passes (0..1, -1..1);
  `Channel::setPan` per channel of the event. FMOD's pan law and the
  resampler (44,100/32,000 Hz into the output rate) shape the samples:
  bit-identical output needs FMOD Ex's mixer behaviour, which is not in
  `FFX.exe` (open question 2).
- **Files** through the game's file layer (VBF, mods), as the callbacks do.

## The offline engine (M5 stage 2)

`crates/ffx-sound`: `fev` (the projects, `sound.md`), `mixer` (voices and
the mix), `replay` (an FMOD call log driving the mixer) and the example
`fmod-mix`:

```bash
# an original-code capture and its call log, in one run (background, game slot)
FFX_AUDIO_WAV=<win temp>/r.wav FFX_FMOD_LOG=<win temp>/r.fmodlog \
  target/release/examples/replay tools/replays/<replay>.txt '<trace>' original
# the same log mixed by the Rust engine, then compared between the marks
FFX_GAME_DIR=~/steam-games/FFX-X2-HD cargo run --release -p ffx-sound \
  --example fmod-mix -- r.fmodlog mine.wav
python3 -I tools/wav-compare.py r.wav mine.wav
```

With both variables set the log also gets one line `# update <n> <dsp
clock> <result>` after each `EventSystem::update` (the capture's wrapper
makes the update, so it is not a logged call from `FFX.exe`), flushed
every tick so `quit` loses nothing. The engine applies the calls logged
before an update note, then mixes one block: update `n` writes WAV frames
`(n - 1) x 1600 .. n x 1600` (the DSP clock after it is `n x 1600`). It
follows `load`, `getGroup`, `getEventByIndex`/`getEvent`, `start`, `stop`,
`setVolume`, `setPaused`, the pan chain (`getChannelGroup` ->
`getChannel(0)` -> `Channel::setPan`) and the Shout bank (`createSound`,
`getSubSound`, `playSound`, `Channel::setVolume`/`setPaused`); the movie
stream is left out. A voice's gain is the event's FEV volume times the
game's `setVolume` (1 until set), in `f32`.

Captures and logs of this round (original code, all four agreed with the
2026-10-26 references sample for sample, except the third battle-attack
run, which differed from them by 0.0168 at most: its own capture is the
comparison): `~/ffx-work/m5/mixer/out/<replay>.{wav,wav.marks,fmodlog}`;
the exploration scripts are in `~/ffx-work/m5/mixer/`.

## FMOD Ex's mixer (measured)

FMOD Ex 4.44.32 as the capture runs it (`docs/harness.md`, "Audio
capture"): software mixer, 48,000 Hz, stereo, `PCMFLOAT`, 1,600-frame
blocks, one per `EventSystem::update`. Round 1 measured from the game's
captures; round 2 (2026-10-27) with controlled tests through the game's
own `fmod_event.dll`/`fmodex.dll`:

```bash
# 32-bit Windows console program: no game, no window, no device (WAV writer, NRT)
PATH=tools/shims:$PATH XWIN_ARCH=x86,x86_64 cargo xwin build --release \
  --target i686-pc-windows-msvc -p ffx-sound --example fmod-probe
fmod-probe.exe <game dir> <media dir with the .fev/.fsb> <script> <out.wav>
```

`crates/ffx-sound/examples/fmod-probe.rs` sets FMOD up as the capture
does (stream buffer, `setOutput(5)`, `PCMFLOAT`, 1,600-frame blocks,
`init(128, STREAM_FROM_UPDATE, file)`), then runs a script: load projects,
create events by index with the end callback, `start`, `stop`, `setPaused`,
`setVolume`, the game's pan chain, the Shout calls, `update`. It prints
each update's DSP clock, every callback with its update and every state
change. The scripts and the analysis are in `~/ffx-work/m5/mixer/probe/`
and `~/ffx-work/m5/mixer/ana_*.py`; the waves come from banks `1110`
(mono and stereo 48 kHz one-shots, a mono loop), `2515` (the loops
`50027_0..3`) and the PS2 music bank.

- **Decoding.** IMA ADPCM, Xbox layout, 64 samples a block: the header's
  predictor first, then 63 codes (`sound.md`). PCM to float is x 1/32768.
- **Resampler.** Linear interpolation `a + (b - a) x frac` in `f32`, the
  position a 32.32 fixed-point number from 0 stepping `(rate << 32) /
  48000` (44,100 Hz: 0.91875 a frame). 48,000 Hz waves are copied.
- **Start.** A voice starts at the first frame of the block of the update
  after its `start`, its gain ramped from 0: frame k of the block has gain
  k x delta, delta = target / 64, added after each frame (accumulated in
  `f32`), then exactly the target from frame 64 on (keeping the
  accumulated value instead made every voice one ulp off).
- **Volume changes.** The same 64-frame ramp from the old gain to the new,
  in the block after `setVolume`, mono and stereo alike (steps 0.5, 0.25,
  0.75, 0.125, 1 every tick: every block exact). **But** a change is taken
  only when the voice's level changes (left plus right) come to at least
  0.00064: below that the old gain stays and the next block compares
  again. Mono 0.00063 a tick updates every other block, 0.00064 every
  block; stereo 0.000315 a side every other, 0.00032 every block; the same
  at a base volume of 0.5. This was round 1's "mono every second block":
  the game's loop fade-in steps 0.000417 a tick.
- **Silence.** A change too small to take, to a volume below about 2e-5
  (1.704e-5 silent, 2.32e-5 not), silences the voice at once, without a
  ramp; it keeps its place. A larger change ramps as usual, to 0 too
  (battle-attack's music fade from 0.0042 to 0).
- **Pan.** None: the event system ignores `Channel::setPan`. Pans -1,
  -0.5, 0, 0.26, 1 on a mono wave and -1, -0.5, 0.5, 1 on a stereo wave
  all gave gain exactly 1 on the same sides. **Mono waves play on the left
  only**, the right stays 0; stereo waves on both sides.
- **Stop, pause.** `stop(false)` and `stop(true)` alike: silent from the
  next block's first frame, no ramp (no event has a fade-out). A pause is
  silent from the next block and the voice resumes where it stopped, at
  its old gain without a ramp.
- **Loops.** On whole IMA blocks. The first pass ends at the end point
  rounded up to a block; every later pass starts at the start point
  rounded up and lasts the loop's length rounded up: `50027_0`, loop
  119648..=737699, plays 0..737728, then 119680..737792 again and again.
  Points on block boundaries (`35110_1`, 90176..=110143) play as they are.
- **Streams.** The PS2 victory fanfare (streamed, 44.1 kHz) played alone
  for 300 blocks equals the decoded wave resampled as above, from its
  first frame: no start latency, no difference from a sample in memory.
- **End callback.** For a once-only wave whose last frame falls in the
  block of update n, the callback says SOUNDDEF_END (2) in update n + 1
  and EVENTFINISHED (4) in update n + 2; `start` itself calls back types
  9, 8 and 1 at once. The game clears its play word on 2 or 4, so on 2.
  `getState` reads 0x18 while it plays, 0x8 after update n (the channel
  done) and 1 (READY) after update n + 2.
- **Sum order.** Voices summed in start order. fmod-probe found no case
  where the order changes the sum (three voices at 0.3, 0.011 and 0.97,
  both start orders, all six orders exact), and ordering by FMOD's channel
  index or by the event instance changed nothing or made it worse on the
  captures.

## Residue (2026-10-27, second round)

The Rust engine against the original capture of the same run, between
`audiomark start` and `end` (`tools/wav-compare.py`'s window):

| Replay | First differing sample | Max error | RMS error (signal RMS) | Blocks exact / within 1e-5 of the peak / larger | Cause, as far as known |
|---|---|---|---|---|---|
| event-besaid | frame 457600 (start + 1600, tick 1), off by 2.9e-11 | 1.86e-9 | 8.2e-11 (0.00201: 4.1e-8) | 40 / 720 / 0 | float rounding only: one ulp of the sum in places |
| battle-attack | frame 254414 (start + 6414, tick 4), off by 3.6e-12 | 3.73e-9 | 1.15e-10 (0.00298: 3.9e-8) | 66 / 923 / 0 | float rounding only |
| menu-browse | frame 435236 (start + 6436, tick 4), off by 2.3e-10 | 2.33e-10 | 4.2e-12 (0.00102: 4.1e-9) | 295 / 116 / 0 | float rounding only |
| battle-ifrit | frame 499203 (start + 3, tick 0), off by 4.7e-10 | 7.45e-9 | 1.7e-10 (0.00678: 2.6e-8) | 0 / 550 / 0 | float rounding only; no block exact (nine voices from the start, two of them 44.1 kHz) |

Round 1's table, for comparison: max errors 0.00129, 0.00326, 4.98e-5 and
6.68e-4, RMS 3.9 %, 3.6 %, 0.2 % and 0.8 % of the signal. Every
structural difference is gone; what is left is at most a few ulps of a
`f32` sum, about 160 dB under the signal. Nothing is accepted as a
tolerance; that is the user's decision.

What is left:
- The last ulps: where FMOD's float operations differ from the engine's
  (the order or grouping of the sum, or the interpolation's or the gain's
  rounding when many voices play). The probe cases so far always summed
  exactly; a probe replaying one of the game's logs (battle-ifrit's
  first blocks) against the engine would find the operation.
- FMOD's channel indices: the engine's model (the highest free of 128)
  matches every `getChannel` handle in menu-browse (19 of 19) but not
  elsewhere (50 of 224, 389 of 852, 238 of 779): FMOD keeps channels
  longer than the engine's voices. It doesn't change the mix.
- Voice streams (32 and 48 kHz; none of the four logs plays a voice event:
  event-besaid's log has no `getEvent`), the Shout channel (the probe has the calls; no log has a
  Shout yet: record a battle with a voiced shout), `maxplaybacks`
  stealing (the game's own limits never reach it in the logs) and the
  movie user stream.

## Open questions

1. *(Answered 2026-10-27.)* The FEV `LGCY` body: `sound.md`, "FMOD
   Designer event projects". Volume, max playbacks, loop or once, stream
   or sample per bank; no fades, pitch, randomisation or 3D in any event.
2. *(Answered 2026-10-27, second round.)* What "identical output" means
   for audio: the capture (`docs/harness.md`, "Audio capture") gives
   FMOD's own mix; the measurements and the residue (float rounding only)
   are above.
3. The boot without the graphics layer missed the title twice here
   (`memochek` -> `loopdemo`, with and without the call log, and once with
   it on): is that the sound being off (master 0 makes every event's
   volume 0) or the harness's press timing? It matters for audio because
   the opening follows its music.
4. `event-besaid` needs save `ffx_003` (not on this machine); a voiced
   event log (voice `getEvent`, `loadEventData`, the Shout FSB's
   `getSubSound`/`getName`/`playSound`) is still to take, and a movie (the
   fmodex user stream and its read callback).
5. `SetThreadPriority(h, 300)`: confirm at run time that the FmodManager
   thread runs at normal priority.
6. The call log keeps 8 words per call; `System::setFileSystem` (8 words
   with `this`) is the longest, so every argument is in the logs.
