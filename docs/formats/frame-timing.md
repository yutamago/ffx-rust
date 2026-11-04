# Frame timing

How often FFX.exe runs its world update and its scripts, what counts in world ticks and what in real
time, and what a mod that decouples the frame rate (draws at the monitor's rate) or fast-forwards
(several ticks per drawn frame) has to deal with. Static research (objdump, `tools/refs.py`, the
ports in `crates/ffx-sg/src/m5main_loop.rs`, `crates/ffx-sg/src/state.rs` `frame_sync` and
`crates/ffx-script/src/vm/bridge/run.rs`); the game was not run for it. Atel# writes waits in
seconds from it (`Wait.Seconds`, 30 frames a second; yutamago/ffx-atel-sharp
`docs/research/frame-timing.md`).

In short: the world (field, events, battle) ticks 30 times a second. The tick is tied to the drawn
frame: `Sg_FrameSync` turns each frame's real time into a number of ticks, one at 30 fps, more to
catch up or with the PC booster. The main menu draws at 60 fps but stops the world. A script frame
(`WaitFrames`) is one tick, so no script can wait less than one.

## The loop

| Step | Where | What |
|---|---|---|
| Frame limiter | `Game_Main` `0x0062ee50`, `Sleep` at `0x0062f3bb` | A frame (`App_Frame` `0x0062e340`) every `g_FrameVsyncs` (`0x00c30e88`) × 1/59.94 s of `Phyre_PTimerWin32_GetTime`; sleeps away the rest. No vsync wait of its own; this is the cap |
| Frame interval | `graphicSetFlipVsnc` `0x006430e0` → `PWindowWin32Base::SetFlipVSyncInterval` `0x00625090` (clamps to 1..2) | Called with 2 by `Sg_MainInit` (`0x00820865`), with 1 when the main menu opens and 2 when it closes (`TkMenuMainLoop` `0x008a9dac`, `0x008a9fe8`), 2 when the sphere grid opens and 1 when it closes (`eiAbmStart` `0x00a56002`, `eiAbmExit` `0x00a5473d`). These five are the only callers |
| Update | `FFXApplication::animate` `0x0042f520` → `updateFFX` `0x00822840` | `updateFFX` calls `Sg_MainLoop(dt)` (`0x00820ae0`) again while `g_VoiceHold` (`0x012fb808`) is not 0, each call taking one off |
| Speed-up booster | `SpdCtrl_ScaleElapsedTime` `0x006f7340` | `dt` × `g_SpdCtrlScales[level]` (`0x00c38e1c`: 1, 2, 4; F1 steps the level) where the booster is allowed; mutes the sound while on |
| Tick count | `Sg_FrameSync(dt)` `0x00821e80` | `dt` capped at 0.3 s (`0x00b3eb44`). Ticks due = (carry + `dt`) × 60. Fewer than 2 due, or `Sg_CanSpeedUp` (`0x0081fd30`) 0: `g_VoiceHold` = 1, carries cleared. Otherwise `g_VoiceHold` = whole part of (carry + `dt`) × 30, and the fraction carries over. A voice-synced cut scene (`g_SyncOn`) counts from the sync clock instead (below) |
| World tick | `Sg_MainLoop`'s update loop (port: `m5main_loop.rs` `update`) | `Sg_GetCurExecFrames()` (`0x0082d7e0`: `0x00c496d4`, 1 by default) times: `Sg_ControlPlayer`, `MsBattleProcess`, `TkUpdateEventJump` (the scripts, below), `TOControlMessageWindow`, effects, `MsCameraProcess`, field, `Ch_CalcMain` (characters, motions). Then once per `Sg_MainLoop`: `Ch_CalcMiddle`, the field animation, the draw |
| Draw | `Sg_MainLoop`'s draw, then Phyre's frame | Each `Sg_MainLoop`, repeats included, builds its draw. Phyre queues one flip per `App_Frame` (`PRenderer_flip` `0x005b6af0`), which the render thread (`0x00616010`) presents one update behind the main thread ([harness.md](../harness.md), "Frames and threads") |

At the 30 fps interval (2 vsyncs, `dt` = 1/29.97 s), each frame has (carry + `dt`) × 60 = 2.002 ticks
due, and the hold is ⌊1.001⌋ = 1. The 0.001 left over adds up to a second tick in about one frame in
1,000 ([harness.md](../harness.md), "Pinned state", sees this as `g_VoiceHold` 2), so the world runs at 30.0 ticks a
real second, a little more than the 29.97 frames drawn. With `Sg_CanSpeedUp` 0 the carries are
dropped and it is exactly one tick per frame: 29.97 a second. When a frame is late, the next one
catches up with more ticks, at most ⌊0.3 × 30⌋ = 9.

At the 60 fps interval (main menu), only 1 tick is due per frame, so the hold is 1 and there is one
`Sg_MainLoop` per frame: 60 a second. But the main menu stops the world update: `Sg_MainLoop` takes
the menu path (`Sg_MainCheckMenu` `0x00820750`) and goes straight to the frame's end. Scripts, motions and the field
do not tick behind the menu. The same holds for the pause, the save screen and scene loads (`stop`).
So **scripts tick only in the 30 fps modes**, at 30 a second.

## What a script frame is

`TkUpdateEventJump` (`0x0088d3d0`) runs once per world tick ("every frame in `Sg_MainLoop`'s loop",
ffx.sym). Through `Atel_EventFrame` and the ctrl work's callback `AtelExecCtrlEvent` (`0x00867740`),
it runs three passes of `Atel_RunCtrlWorkers` (`0x00867950`). The first pass runs `Script_Run`
(`0x008641e0`) on every worker. The other two run only workers with a level that a request just
started (`+0x36` bits `0x180`), so a new request begins in the same tick. Every caller of `Script_Run`
is in these passes, or is the debugger (`AtelDecodeCommand`, `Atel_CtrlStart`,
`AtelExecCtrlEventInit`).

A native that polls ends the worker's run for that tick: opcode `35`/`58` (`run.rs`) starts the
native and polls it once, and a poll without the "run on" bit (4) leaves the run with an empty
stack. `Native_WaitFrames_0000` (`0x0085c4e0`) stores n, and its poll (`0x0085c6b0`) subtracts 1
and is done at 0 or below. So `WaitFrames(n)` started in tick t ends in tick t + n − 1, and the
next statement runs in tick t + n. `WaitFrames(1)` and `WaitFrames(0)` both resume in the next tick.

Two exceptions:

- **Movies with a camera.** `Native_b00a_MovieStartPicture` sets `g_WaitFramesByTime`
  (`0x0112a008`), and `destroyMoviePicture` clears it. While it is set and `movie_have_camera`
  (`0x0076efc0`: movies `0x20`, `0x2e`, `0x46`, `0x48`) holds, the poll subtracts
  `graphicVideoGetElapsedFrame()` (`0x00645d90`: the movie's frames since the last update) instead
  of 1. The wait then counts the movie's own frames, 29.97 a second ([platform.md](platform.md), "Movies"). `Atel_MoveUpdate` (`0x00868990`) does the same for moves.
- **The timing patch.** `WaitFrames(10)` in battle scene `0x610000` while magic `0x290` is loaded
  waits 40 frames (`Native_WaitFrames_0000`'s start slot). Any wait written as 10 frames there is.

`Script_Run` also has a per-worker speed: `+0x94` × `g_GameSpeed` (`0x01326bd8`), added to
`+0x98` (`0x0086426b`..`0x008642cc`). Below 1 the worker's run stops at once. At 1 or more it runs
normally. Nothing makes it run more than once per tick. `g_GameSpeed` is 1.0 (`AtelInitTotal`), and
its only setter (`0x00871020`) has no callers. This is not a rate control.

## Frames and real time

| Counts world ticks (speeds up with them) | Counts real time or its own clock |
|---|---|
| `WaitFrames`, the poll natives (motion, move, turn, camera, message waits) | Movies: `updateTexture` takes one decoded frame per update (or per render command for `0x20`, `0x2e`, `0x48` and loops). The sound is an FMOD stream in real time, with no A/V clock ([platform.md](platform.md)) |
| Motions (`SG_UpdateMotion`, in `Ch_CalcMain`), moves and turns (`Atel_MoveUpdate`), `Sg_AnimValF` eases | FMOD: music, voices and sounds mix on FMOD's threads in real time. A sound starts in a tick but plays at its own rate |
| The camera (`MsCameraProcess`), the effects' update half (`MsEffectProcess(0)`), the field (`Yn_FieldProc_0091a8c0`) | Voice-synced cut scenes: with `g_SyncOn` (`0x012fb8a0`, set by `Sg_SyncCutScene` `0x00821d60` from `iSyncGetData`, the remaster's `SyncData` tables), `Sg_FrameSync` steps ticks while the sync clock (`graphicGetTimeInMicroseconds`) is behind and spins while it is 0.1 s or more ahead, so the scene keeps pace with the voices in real time |
| Battle (`MsBattleProcess`): turn order, counters, the battle's waits | Loading: the asset loader thread reads in real time, and the world update stops (`Sg_IsSceneLoading` `0x0081fcd0`) until it is done |
| Message windows (`TOControlMessageWindow`), the pad's press edges as scripts see them | Input: the pad word is built once per drawn frame (`Virtuos_InputManager_updateInput` `0x00632a30`); hold repeats after 0.2333 s, then every 0.1 s |
| Once per `Sg_MainLoop`, not per tick: `Ch_CalcMiddle`, the field animation (`Yi`), the v-blank counters (+2), `Sg_MainCalcRate`'s rate | The Iggy (Flash) PC menus at their own 30 fps; the speed-up booster's icon |

`Sg_SetKeepFps` (native `0x4022`, set by battle magic) has motions step by `Sg_MainCalcRate`'s rate
(`0x023cbbec`, 0x100 = one tick). That is the v-blank count since the last call, which is 2 per
`Sg_MainLoop`, or the sync data's frame in a synced scene. It is the PS2's frame-drop compensation
and stays in ticks.

## (a) Decoupled drawing at the monitor's rate

Raising the rate the easy way changes `g_FrameVsyncs`, or replaces the limiter's interval with the
monitor's. Each frame then runs `Sg_MainLoop` with one tick (fewer than 2 due), so the game runs
twice as fast at 60 fps and faster still above. The tick has to be separated from the drawn frame.
A Fahrenheit mod could do it like this:

1. **Limiter.** Hook `Game_Main`'s `Sleep` (`0x0062f3bb`) or `Phyre_PTimerWin32_GetTime`. Run
   `App_Frame` at the monitor's rate (from the swap chain's refresh rate, or with no cap), and
   present with vsync or without.
2. **Tick accumulator.** Keep the game's own `dt` (the frame's real time) and add it to a 30 Hz
   accumulator. When a tick (or several) is due, let `Sg_MainLoop` run as now, passing it the
   accumulated time as `dt`, so `Sg_FrameSync` sees the 30 fps it expects. When none is due, draw
   without updating.
3. **Drawing without a tick.** `Sg_MainLoop` both updates and builds the draw, so a frame without a
   tick needs a path that only draws. `Sg_SetExecFrames(0)` (the PS2 debug pause) is the nearest:
   the update loop runs no tick. It still runs `MsEffectProcess(0)` and the camera, and once per
   call `Ch_CalcMiddle`, the field animation, the effects' draw half (`op_loop_ffx_after`,
   `MsEffectProcess(1)`), `Sg_FrameSync`'s pad scan and v-blank count, `graphicVideoUpdate`, the
   fades and the menu update. Each of these would have to be held on frames without a tick (hooks
   that return early, or their state saved and restored), or they run at the drawn rate.
4. **Interpolation.** Save, for each tick, the state the draw reads: the camera and screen
   matrices (`MsScreenProcess`, `MsCopyCameraMatrix` `0x007c0070`, `MsCopyScreenMatrix` `0x007bba50`), each character's
   root and bone matrices (`Ch_CalcMiddle`'s output), and the field's moving parts. Before drawing a
   frame between ticks, write the lerp (slerp for rotations) of the last two ticks at the
   accumulator's fraction, and put the latest values back after. That shows the world one tick late,
   about 33 ms. Effects and particles (the VU-emulated `ppp` programs), message windows and the PS2
   2D layers have no clean state to interpolate. They would stay at 30 Hz unless each is handled.
5. **Render thread.** Phyre already draws update n while the main thread computes n + 1. A drawn
   frame without a tick is another `App_Frame` with a flip, so the handshake stays as it is (one
   flip per `App_Frame`). The cloth's staging ([host.md](../host.md), "The cloth race") expects
   one stage per submit and needs checking.

Feasibility: steps 1-3 are a few hooks. Camera interpolation is moderate work and covers most of the
perceived smoothness on the field. Full interpolation of characters, effects and UI touches many
systems and is a large project. The 30 Hz simulation itself stays exact, and with it every frame
count, scripts included.

## (b) Fast-forward: N ticks per drawn frame

The game has three ways to run several ticks per drawn frame:

| Way | How | Limit | Draws per tick |
|---|---|---|---|
| Booster | `g_SpdCtrlScales` (`0x00c38e1c`) × `dt` → `Sg_FrameSync` → `g_VoiceHold` → `updateFFX` repeats `Sg_MainLoop` | `dt` capped at 0.3 s: at most 9 ticks per frame, about ×9 at 30 fps. Only where `SpdCtrl_ScaleElapsedTime` and `Sg_CanSpeedUp` allow (not in movies, synced scenes, the sphere grid, the excluded maps and spells) | Each repeat builds a full draw (the harness sees a double tick draw the effects twice) |
| Exec frames | `Sg_SetExecFrames(n)` (`0x0082da60`, `0x00c496d4`, clamped to 0..99): the update loop runs n ticks per `Sg_MainLoop` | 99 per `Sg_MainLoop` | One draw; the effects' draw half runs n times |
| Both | exec frames × booster hold | 99 × 9 | |

A mod for ×100 would set exec frames (for example 50) and the booster scale (×2, with the 0.3 s cap
in mind), or call `Sg_MainLoop`'s update part in a loop of its own. Some of it does not follow:

- **What runs once per `Sg_MainLoop`.** With exec frames, `Ch_CalcMiddle`, the field animation, the
  fades, `Sg_FrameSync` (pad scan, v-blank count), `graphicVideoUpdate` and the menus run once per
  drawn frame, while the world runs n ticks. Field animations and movies stay at ×1.
  `Sg_MainCalcRate` forces the motion rate to 0x100 when exec frames is not 1. The booster repeats
  all of `Sg_MainLoop`, so it has none of these gaps, but it builds a draw per tick.
- **Audio.** FMOD plays in real time. Sounds triggered per tick pile up ×N, and music and voices
  don't speed up. The booster mutes the sound (`Sg_SetMuteSound`) and mutes the battle subtitles.
- **Movies.** The video advances one frame per update and the sound plays in real time, so a faster
  update desyncs them. `Sg_CanSpeedUp` refuses a video without a movie camera, and the booster stays
  off while `g_WaitFramesByTime` is set.
- **Voice-synced cut scenes.** `g_SyncOn` paces the ticks to the sync clock (real time), so the
  booster does nothing there. Exec frames would run n ticks per synced step and leave the voices
  behind.
- **Input.** The pad word is read once per drawn frame, and the press edges once per `Sg_MainLoop`
  (`Sg_ControlMain`'s `0x023c44a8`). With exec frames, all n ticks see the same buttons. Whether a
  press then counts once or n times in each reader (menus, scripts' `Pad` natives) is not checked.
- **Loading.** Real time; the world update waits for it.
- **Render thread.** One flip per `App_Frame` either way. Exec frames draws once, so the GPU sees no
  extra work. The booster's repeats build extra draws, and the main thread does all their work.
- **CPU.** Every tick is simulated. The hook harness's `--fast` mode ran 461 whole frames (update,
  draw, present) a second on its test machine ([harness.md](../harness.md), "How it gets in"), about ×15 in full
  frames. A tick without its draw is cheaper, but this has not been measured. ×100 would be 3,000
  ticks a second, which is likely beyond one core in heavy scenes.

## (c) Waits shorter than a frame

None. A worker runs once per tick, and the next tick is the next time anything in the world
changes. At a
decoupled draw rate (a), a script still sees only ticks. A mod that wanted finer steps would have
to raise the tick rate itself, and that changes the meaning of every frame count in the game's
scripts and code (all of them written for 30), so it is not a mod-sized change.
