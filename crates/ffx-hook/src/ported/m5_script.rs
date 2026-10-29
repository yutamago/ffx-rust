//! M5 stage 1, list `script`: Atel natives that already ran in Rust through
//! `Script_Run`'s dispatch (`ffx_script::vm::bridge::native*::find`), now
//! hooked at their own addresses as well. The dump has no function start
//! for most of them (Ghidra left them as table targets); `symbols/sizes.tsv`
//! holds their measured lengths. All are cdecl `(worker, args, stack)` and
//! leave `EAX` (or `ST0` for the float slot); `Native_b008_DebugPrint`
//! leaves the caller's `EAX` (an empty `dbgPrintf` before its `ret`), so
//! its shim returns nothing and the thunk keeps it.
//!
//! The Atel module's lone `jmp` thunks (`ffx_script::m5_thunks`) are here
//! too: each calls its target with the words the target reads.
//!
//! Threads: the main thread (the script VM); no state, no frames.

use ffx_script::m5_thunks as th;
use ffx_script::vm::bridge::{
    native_btl, native_btl2, native_chr, native_m5, native_mods, native_move, native_msg,
};

use super::{StackFrame, mem, ported};
use ffx_menu::m5m_l::bridge as l;
use ffx_menu::m5m_m::bridge as mm;

ported! {
    /// `Native_b000_MovieStart_Poll(worker, args, stack)`.
    Native_b000_MovieStart_Poll @ 0x0076_e980 => extern "C" fn n_76e980(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_done(&mut mem(), w, a, s)
    }
    /// `Native_b001_MovieWait_Poll(worker, args, stack)`.
    Native_b001_MovieWait_Poll @ 0x0076_e9c0 => extern "C" fn n_76e9c0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_b001_poll(&mut mem(), w, a, s)
    }
    /// `Native_b003_MovieGetFrame(worker, args, stack)`.
    Native_b003_MovieGetFrame @ 0x0076_ea40 => extern "C" fn n_76ea40(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_b003(&mut mem(), w, a, s)
    }
    /// `Native_b004_MovieClose(worker, args, stack)`.
    Native_b004_MovieClose @ 0x0076_ea50 => extern "C" fn n_76ea50(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_b004(&mut mem(), w, a, s)
    }
    /// `Native_b004_MovieClose_Poll(worker, args, stack)`.
    Native_b004_MovieClose_Poll @ 0x0076_ea60 => extern "C" fn n_76ea60(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_b004_poll(&mut mem(), w, a, s)
    }
    /// `Native_b004_MovieClose_Result(worker, args, stack)`.
    Native_b004_MovieClose_Result @ 0x0076_eac0 => extern "C" fn n_76eac0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_b004_result(&mut mem(), w, a, s)
    }
    /// `Native_b005_DebugPrint(worker, args, stack)`.
    Native_b005_DebugPrint @ 0x0076_ead0 => extern "C" fn n_76ead0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_b005(&mut mem(), w, a, s)
    }
    /// `Native_b006_DebugPrint(worker, args, stack)`.
    Native_b006_DebugPrint @ 0x0076_eae0 => extern "C" fn n_76eae0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_b005(&mut mem(), w, a, s)
    }
    /// `Native_b007_DebugPrint(worker, args, stack)`.
    Native_b007_DebugPrint @ 0x0076_eaf0 => extern "C" fn n_76eaf0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_b005(&mut mem(), w, a, s)
    }
    /// `Native_b008_DebugPrint(worker, args, stack)`.
    Native_b008_DebugPrint @ 0x0076_eb00 => extern "C" fn n_76eb00(w: u32, a: u32, s: u32) {
        native_mods::native_b008(&mut mem(), w, a, s);
    }
    /// `Native_b008_DebugPrint_Poll(worker, args, stack)`.
    Native_b008_DebugPrint_Poll @ 0x0076_eb10 => extern "C" fn n_76eb10(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_b005(&mut mem(), w, a, s)
    }
    /// `Native_b008_DebugPrint_Result(worker, args, stack)`.
    Native_b008_DebugPrint_Result @ 0x0076_eb20 => extern "C" fn n_76eb20(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_b005(&mut mem(), w, a, s)
    }
    /// `Native_b009_MovieCameraWait_Poll(worker, args, stack)`.
    Native_b009_MovieCameraWait_Poll @ 0x0076_eb40 => extern "C" fn n_76eb40(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_b009_poll(&mut mem(), w, a, s)
    }
    /// `Native_b009_MovieCameraWait_Result(worker, args, stack)`.
    Native_b009_MovieCameraWait_Result @ 0x0076_eb60 => extern "C" fn n_76eb60(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_b009_result(&mut mem(), w, a, s)
    }
    /// `Native_b00a_MovieStartPicture_Poll(worker, args, stack)`.
    Native_b00a_MovieStartPicture_Poll @ 0x0076_ebe0 => extern "C" fn n_76ebe0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_b00a_poll(&mut mem(), w, a, s)
    }
    /// `Native_b00b_MovieDestroyWait_Poll(worker, args, stack)`.
    Native_b00b_MovieDestroyWait_Poll @ 0x0076_ec30 => extern "C" fn n_76ec30(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_b00b_poll(&mut mem(), w, a, s)
    }
    /// `Native_b00c_MovieGetManager(worker, args, stack)`.
    Native_b00c_MovieGetManager @ 0x0076_ec90 => extern "C" fn n_76ec90(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_b00c(&mut mem(), w, a, s)
    }
    /// `Native_b00d_MovieClearDone(worker, args, stack)`.
    Native_b00d_MovieClearDone @ 0x0076_ecb0 => extern "C" fn n_76ecb0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_b00d(&mut mem(), w, a, s)
    }
    /// `Native_b00e_DebugPrint(worker, args, stack)`.
    Native_b00e_DebugPrint @ 0x0076_ecc0 => extern "C" fn n_76ecc0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_b005(&mut mem(), w, a, s)
    }
    /// `Native_b00f_DebugPrint(worker, args, stack)`.
    Native_b00f_DebugPrint @ 0x0076_ecd0 => extern "C" fn n_76ecd0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_b005(&mut mem(), w, a, s)
    }
    /// `Native_70ad_EquipmentHasUnmarked(worker, args, stack)`.
    Native_70ad_EquipmentHasUnmarked @ 0x007a_2cd0 => extern "C" fn n_7a2cd0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_70ad(&mut mem(), w, a, s)
    }
    /// `Native_70fc_BtlGetRandFixed(worker, args, stack)`.
    Native_70fc_BtlGetRandFixed @ 0x007a_3440 => extern "C" fn n_7a3440(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_70fc(&mut mem(), w, a, s)
    }
    /// `Native_70db_BtlGetByteCa06(worker, args, stack)`.
    Native_70db_BtlGetByteCa06 @ 0x007a_3450 => extern "C" fn n_7a3450(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_70db(&mut mem(), w, a, s)
    }
    /// `Native_70f6_BtlListClear(worker, args, stack)`.
    Native_70f6_BtlListClear @ 0x007a_34e0 => extern "C" fn n_7a34e0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_70f6(&mut mem(), w, a, s)
    }
    /// `Native_70d9_ChrModelSetHide13(worker, args, stack)`.
    Native_70d9_ChrModelSetHide13 @ 0x007a_3500 => extern "C" fn n_7a3500(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_70d9(&mut mem(), w, a, s)
    }
    /// `Native_7002_BtlLabelExe_Poll(worker, args, stack)`.
    Native_7002_BtlLabelExe_Poll @ 0x007a_35c0 => extern "C" fn n_7a35c0(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_7002_poll(&mut mem(), w, a, s)
    }
    /// `Native_70f8_BtlListNext(worker, args, stack)`.
    Native_70f8_BtlListNext @ 0x007a_36e0 => extern "C" fn n_7a36e0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_70f8(&mut mem(), w, a, s)
    }
    /// `Native_710a_BtlSetByteCa10(worker, args, stack)`.
    Native_710a_BtlSetByteCa10 @ 0x007a_3970 => extern "C" fn n_7a3970(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_710a(&mut mem(), w, a, s)
    }
    /// `Native_710e_ReelFirstSetByte(worker, args, stack)`.
    Native_710e_ReelFirstSetByte @ 0x007a_3c00 => extern "C" fn n_7a3c00(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_710e(&mut mem(), w, a, s)
    }
    /// `Native_710f_ReelGetByte1(worker, args, stack)`.
    Native_710f_ReelGetByte1 @ 0x007a_3c90 => extern "C" fn n_7a3c90(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_710f(&mut mem(), w, a, s)
    }
    /// `Native_711e_BtlGetCommand3029Count(worker, args, stack)`.
    Native_711e_BtlGetCommand3029Count @ 0x007a_3f40 => extern "C" fn n_7a3f40(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_711e(&mut mem(), w, a, s)
    }
    /// `Native_7014_BtlGetParsedCommand(worker, args, stack)`.
    Native_7014_BtlGetParsedCommand @ 0x007a_4430 => extern "C" fn n_7a4430(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7014(&mut mem(), w, a, s)
    }
    /// `Native_70da_BtlGetFirstAttack(worker, args, stack)`.
    Native_70da_BtlGetFirstAttack @ 0x007a_4df0 => extern "C" fn n_7a4df0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_70da(&mut mem(), w, a, s)
    }
    /// `Native_7035_BtlGetEndForced(worker, args, stack)`.
    Native_7035_BtlGetEndForced @ 0x007a_53a0 => extern "C" fn n_7a53a0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7035(&mut mem(), w, a, s)
    }
    /// `Native_703d_BtlCameraStateClear(worker, args, stack)`.
    Native_703d_BtlCameraStateClear @ 0x007a_5980 => extern "C" fn n_7a5980(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_703d(&mut mem(), w, a, s)
    }
    /// `Native_703e_WaitBtlIdle_Poll(worker, args, stack)`.
    Native_703e_WaitBtlIdle_Poll @ 0x007a_5a10 => extern "C" fn n_7a5a10(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_703e_poll(&mut mem(), w, a, s)
    }
    /// `Native_7081_WaitLimitTypeCue_Poll(worker, args, stack)`.
    Native_7081_WaitLimitTypeCue_Poll @ 0x007a_5d60 => extern "C" fn n_7a5d60(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_7081_poll(&mut mem(), w, a, s)
    }
    /// `Native_7064_BtlStreamStandby_Poll(worker, args, stack)`.
    Native_7064_BtlStreamStandby_Poll @ 0x007a_64c0 => extern "C" fn n_7a64c0(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_7064_poll(&mut mem(), w, a, s)
    }
    /// `Native_7065_BtlStreamStart_Poll(worker, args, stack)`.
    Native_7065_BtlStreamStart_Poll @ 0x007a_6760 => extern "C" fn n_7a6760(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_7065_poll(&mut mem(), w, a, s)
    }
    /// `Native_7065_BtlStreamStart(worker, args, stack)`.
    Native_7065_BtlStreamStart @ 0x007a_6950 => extern "C" fn n_7a6950(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_7065(&mut mem(), w, a, s)
    }
    /// `Native_7066_SndStopStream(worker, args, stack)`.
    Native_7066_SndStopStream @ 0x007a_6960 => extern "C" fn n_7a6960(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_7066(&mut mem(), w, a, s)
    }
    /// `Native_7067_VoiceIsStreaming(worker, args, stack)`.
    Native_7067_VoiceIsStreaming @ 0x007a_6a30 => extern "C" fn n_7a6a30(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_7067(&mut mem(), w, a, s)
    }
    /// `Native_7068_VoiceStreamWait_Poll(worker, args, stack)`.
    Native_7068_VoiceStreamWait_Poll @ 0x007a_6ad0 => extern "C" fn n_7a6ad0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_7068_poll(&mut mem(), w, a, s)
    }
    /// `Native_706d_SoundStreamPlay_Poll(worker, args, stack)`.
    Native_706d_SoundStreamPlay_Poll @ 0x007a_6db0 => extern "C" fn n_7a6db0(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_706d_poll(&mut mem(), w, a, s)
    }
    /// `Native_7070_SetByte00c58ef4(worker, args, stack)`.
    Native_7070_SetByte00c58ef4 @ 0x007a_7010 => extern "C" fn n_7a7010(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_7070(&mut mem(), w, a, s)
    }
    /// `Native_7071_ClearByte00c58ef4(worker, args, stack)`.
    Native_7071_ClearByte00c58ef4 @ 0x007a_70b0 => extern "C" fn n_7a70b0(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7071(&mut mem(), w, a, s)
    }
    /// `Native_70a2_RegMotionRead_Poll(worker, args, stack)`.
    Native_70a2_RegMotionRead_Poll @ 0x007a_7500 => extern "C" fn n_7a7500(w: u32, a: u32, s: u32) -> u32 {
        native_btl2::native_70a2_poll(&mut mem(), w, a, s)
    }
    /// `Native_70a4_ChnSystemMgrpReset(worker, args, stack)`.
    Native_70a4_ChnSystemMgrpReset @ 0x007a_7630 => extern "C" fn n_7a7630(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_70a4(&mut mem(), w, a, s)
    }
    /// `Native_70ae_BtlGetDying(worker, args, stack)`.
    Native_70ae_BtlGetDying @ 0x007a_7790 => extern "C" fn n_7a7790(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_70ae(&mut mem(), w, a, s)
    }
    /// `Native_70af_GetVoiceEventFlag(worker, args, stack)`.
    Native_70af_GetVoiceEventFlag @ 0x007a_79a0 => extern "C" fn n_7a79a0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_70af(&mut mem(), w, a, s)
    }
    /// `Native_70de_GlobalBit9Guarded(worker, args, stack)`.
    Native_70de_GlobalBit9Guarded @ 0x007a_79b0 => extern "C" fn n_7a79b0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_70de(&mut mem(), w, a, s)
    }
    /// `Native_70e8_GetGil(worker, args, stack)`.
    Native_70e8_GetGil @ 0x007a_7a90 => extern "C" fn n_7a7a90(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_70e8(&mut mem(), w, a, s)
    }
    /// `Native_70e9_GetGlobal01130838(worker, args, stack)`.
    Native_70e9_GetGlobal01130838 @ 0x007a_7b00 => extern "C" fn n_7a7b00(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_70e9(&mut mem(), w, a, s)
    }
    /// `Native_7096_BtlGetReflected(worker, args, stack)`.
    Native_7096_BtlGetReflected @ 0x007a_7b30 => extern "C" fn n_7a7b30(w: u32, a: u32, s: u32) -> u32 {
        native_btl::native_7096(&mut mem(), w, a, s)
    }
    /// `Native_70c5_BtlGetPartyLeaves(worker, args, stack)`.
    Native_70c5_BtlGetPartyLeaves @ 0x007a_7c20 => extern "C" fn n_7a7c20(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_70c5(&mut mem(), w, a, s)
    }
    /// `Native_70eb_BtlRandom(worker, args, stack)`.
    Native_70eb_BtlRandom @ 0x007a_7d10 => extern "C" fn n_7a7d10(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_70eb(&mut mem(), w, a, s)
    }
    /// `Native_7101_BtlGetDebugFull(worker, args, stack)`.
    Native_7101_BtlGetDebugFull @ 0x007a_7f60 => extern "C" fn n_7a7f60(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_7101(&mut mem(), w, a, s)
    }
    /// `Native_70cf_BtlGetVoiceEvent(worker, args, stack)`.
    Native_70cf_BtlGetVoiceEvent @ 0x007a_7f70 => extern "C" fn n_7a7f70(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_70cf(&mut mem(), w, a, s)
    }
    /// `Native_7113_BtlSetStealRule(worker, args, stack)`.
    Native_7113_BtlSetStealRule @ 0x007a_8090 => extern "C" fn n_7a8090(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_7113(&mut mem(), w, a, s)
    }
    /// `Native_7114_BtlSetStealRule(worker, args, stack)`.
    Native_7114_BtlSetStealRule @ 0x007a_80a0 => extern "C" fn n_7a80a0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_7114(&mut mem(), w, a, s)
    }
    /// `Native_6078_CamGetScreenDepth(worker, args, stack)`.
    Native_6078_CamGetScreenDepth @ 0x007b_91f0 => extern "C" fn n_7b91f0(w: u32, a: u32, s: u32) -> f64 {
        native_chr::native_6078(&mut mem(), w, a, s)
    }
    /// `Native_013c_MsgShowWaitEx_Poll(worker, args, stack)`.
    Native_013c_MsgShowWaitEx_Poll @ 0x0085_5ec0 => extern "C" fn n_855ec0(w: u32, a: u32, s: u32) -> u32 {
        native_msg::native_msg_select_poll(&mut mem(), w, a, s)
    }
    /// `Native_013c_MsgShowWaitEx_Result(worker, args, stack)`.
    Native_013c_MsgShowWaitEx_Result @ 0x0085_6030 => extern "C" fn n_856030(w: u32, a: u32, s: u32) -> u32 {
        native_msg::native_msg_select_result(&mut mem(), w, a, s)
    }
    /// `Native_023d(worker, args, stack)`.
    Native_023d @ 0x0085_6520 => extern "C" fn n_856520(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_023d(&mut mem(), w, a, s)
    }
    /// `Native_000d_GetLastEventJumpId(worker, args, stack)`.
    Native_000d_GetLastEventJumpId @ 0x0085_65f0 => extern "C" fn n_8565f0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_000d(&mut mem(), w, a, s)
    }
    /// `Native_000e_GetLastEventJumpIndex(worker, args, stack)`.
    Native_000e_GetLastEventJumpIndex @ 0x0085_66a0 => extern "C" fn n_8566a0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_000e(&mut mem(), w, a, s)
    }
    /// `Native_000f_GetNowEventJumpId(worker, args, stack)`.
    Native_000f_GetNowEventJumpId @ 0x0085_6790 => extern "C" fn n_856790(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_000f(&mut mem(), w, a, s)
    }
    /// `Native_00ea_CtrlGetByte1Bit4(worker, args, stack)`.
    Native_00ea_CtrlGetByte1Bit4 @ 0x0085_67d0 => extern "C" fn n_8567d0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_00ea(&mut mem(), w, a, s)
    }
    /// `Native_0010_GetNowEventJumpIndex(worker, args, stack)`.
    Native_0010_GetNowEventJumpIndex @ 0x0085_6910 => extern "C" fn n_856910(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0010(&mut mem(), w, a, s)
    }
    /// `Native_0242_GetHddStatus(worker, args, stack)`.
    Native_0242_GetHddStatus @ 0x0085_7200 => extern "C" fn n_857200(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0242(&mut mem(), w, a, s)
    }
    /// `Native_0243_GetHddWord518(worker, args, stack)`.
    Native_0243_GetHddWord518 @ 0x0085_7350 => extern "C" fn n_857350(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0243(&mut mem(), w, a, s)
    }
    /// `Native_0244(worker, args, stack)`.
    Native_0244 @ 0x0085_74b0 => extern "C" fn n_8574b0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0244(&mut mem(), w, a, s)
    }
    /// `Native_0245_SetGlobal011a0080Bit11(worker, args, stack)`.
    Native_0245_SetGlobal011a0080Bit11 @ 0x0085_7690 => extern "C" fn n_857690(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0245(&mut mem(), w, a, s)
    }
    /// `Native_0246_ClearGlobal011a0080Bit13(worker, args, stack)`.
    Native_0246_ClearGlobal011a0080Bit13 @ 0x0085_7810 => extern "C" fn n_857810(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0246(&mut mem(), w, a, s)
    }
    /// `Native_0247_SetGlobal011a0080Bit13(worker, args, stack)`.
    Native_0247_SetGlobal011a0080Bit13 @ 0x0085_7910 => extern "C" fn n_857910(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0247(&mut mem(), w, a, s)
    }
    /// `Native_0248_ClearGlobal011a0080Bit12(worker, args, stack)`.
    Native_0248_ClearGlobal011a0080Bit12 @ 0x0085_79f0 => extern "C" fn n_8579f0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0248(&mut mem(), w, a, s)
    }
    /// `Native_0249_SetGlobal011a0080Bit12(worker, args, stack)`.
    Native_0249_SetGlobal011a0080Bit12 @ 0x0085_7b60 => extern "C" fn n_857b60(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0249(&mut mem(), w, a, s)
    }
    /// `Native_00da_GetBattleStatus(worker, args, stack)`.
    Native_00da_GetBattleStatus @ 0x0085_7c30 => extern "C" fn n_857c30(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00da(&mut mem(), w, a, s)
    }
    /// `Native_00e1_GetBattleStatusSys(worker, args, stack)`.
    Native_00e1_GetBattleStatusSys @ 0x0085_7fb0 => extern "C" fn n_857fb0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00e1(&mut mem(), w, a, s)
    }
    /// `Native_01d9_SetChr6Name(worker, args, stack)`.
    Native_01d9_SetChr6Name @ 0x0085_82e0 => extern "C" fn n_8582e0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01d9(&mut mem(), w, a, s)
    }
    /// `Native_00ae_GetSaveDic(worker, args, stack)`.
    Native_00ae_GetSaveDic @ 0x0085_8320 => extern "C" fn n_858320(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_00ae(&mut mem(), w, a, s)
    }
    /// `Native_0255_SaveMenuGetBits1415(worker, args, stack)`.
    Native_0255_SaveMenuGetBits1415 @ 0x0085_8430 => extern "C" fn n_858430(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0255(&mut mem(), w, a, s)
    }
    /// `Native_00b0_GetBattleSceneGroup(worker, args, stack)`.
    Native_00b0_GetBattleSceneGroup @ 0x0085_85c0 => extern "C" fn n_8585c0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_00b0(&mut mem(), w, a, s)
    }
    /// `Native_00f9_AlwaysTrue(worker, args, stack)`.
    Native_00f9_AlwaysTrue @ 0x0085_8780 => extern "C" fn n_858780(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_00f9(&mut mem(), w, a, s)
    }
    /// `Native_01ab_GetBattleCount(worker, args, stack)`.
    Native_01ab_GetBattleCount @ 0x0085_8800 => extern "C" fn n_858800(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01ab(&mut mem(), w, a, s)
    }
    /// `Native_01e2_SetGlobal018ded08Bit24(worker, args, stack)`.
    Native_01e2_SetGlobal018ded08Bit24 @ 0x0085_8940 => extern "C" fn n_858940(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01e2(&mut mem(), w, a, s)
    }
    /// `Native_01e3_DisableHamonEffect(worker, args, stack)`.
    Native_01e3_DisableHamonEffect @ 0x0085_8ab0 => extern "C" fn n_858ab0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01e3(&mut mem(), w, a, s)
    }
    /// `Native_014d_SaveEventMaxIndex(worker, args, stack)`.
    Native_014d_SaveEventMaxIndex @ 0x0085_8e10 => extern "C" fn n_858e10(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_014d(&mut mem(), w, a, s)
    }
    /// `Native_021e_RenderInitFog(worker, args, stack)`.
    Native_021e_RenderInitFog @ 0x0085_8e40 => extern "C" fn n_858e40(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_021e(&mut mem(), w, a, s)
    }
    /// `Native_025a_GetAsiaMode(worker, args, stack)`.
    Native_025a_GetAsiaMode @ 0x0085_90e0 => extern "C" fn n_8590e0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_025a(&mut mem(), w, a, s)
    }
    /// `Native_01b1_SaveEventSetE0(worker, args, stack)`.
    Native_01b1_SaveEventSetE0 @ 0x0085_9240 => extern "C" fn n_859240(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01b1(&mut mem(), w, a, s)
    }
    /// `Native_00b9_SgTkAntiEnabled(worker, args, stack)`.
    Native_00b9_SgTkAntiEnabled @ 0x0085_93e0 => extern "C" fn n_8593e0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_00b9(&mut mem(), w, a, s)
    }
    /// `Native_025c_NoOp(worker, args, stack)`.
    Native_025c_NoOp @ 0x0085_9400 => extern "C" fn n_859400(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_025c(&mut mem(), w, a, s)
    }
    /// `Native_01b4_IsShoutVoiceRead(worker, args, stack)`.
    Native_01b4_IsShoutVoiceRead @ 0x0085_9490 => extern "C" fn n_859490(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_01b4(&mut mem(), w, a, s)
    }
    /// `Native_016c_MusicStopCurrent(worker, args, stack)`.
    Native_016c_MusicStopCurrent @ 0x0085_94c0 => extern "C" fn n_8594c0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_016c(&mut mem(), w, a, s)
    }
    /// `Native_01b5_TextureVideoPlay(worker, args, stack)`.
    Native_01b5_TextureVideoPlay @ 0x0085_9580 => extern "C" fn n_859580(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01b5(&mut mem(), w, a, s)
    }
    /// `Native_01f1_GetHalfTurnAngle(worker, args, stack)`.
    Native_01f1_GetHalfTurnAngle @ 0x0085_9620 => extern "C" fn n_859620(w: u32, a: u32, s: u32) -> f64 {
        native_mods::native_01f1(&mut mem(), w, a, s)
    }
    /// `Native_01b6_EquipHasUnmarked(worker, args, stack)`.
    Native_01b6_EquipHasUnmarked @ 0x0085_9630 => extern "C" fn n_859630(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01b6(&mut mem(), w, a, s)
    }
    /// `Native_01b8_EquipMarkFirst(worker, args, stack)`.
    Native_01b8_EquipMarkFirst @ 0x0085_9ba0 => extern "C" fn n_859ba0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01b8(&mut mem(), w, a, s)
    }
    /// `Native_0167_MusicSetReplayPoint(worker, args, stack)`.
    Native_0167_MusicSetReplayPoint @ 0x0085_9c40 => extern "C" fn n_859c40(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0167(&mut mem(), w, a, s)
    }
    /// `Native_01b9_EquipUnmarkFirst(worker, args, stack)`.
    Native_01b9_EquipUnmarkFirst @ 0x0085_9d20 => extern "C" fn n_859d20(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_01b9(&mut mem(), w, a, s)
    }
    /// `Native_01bb_GetMusicPlayState(worker, args, stack)`.
    Native_01bb_GetMusicPlayState @ 0x0085_9de0 => extern "C" fn n_859de0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01bb(&mut mem(), w, a, s)
    }
    /// `Native_MusicWaitLoaded_Poll(worker, args, stack)`.
    Native_MusicWaitLoaded_Poll @ 0x0085_9df0 => extern "C" fn n_859df0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_music_wait_loaded_poll(&mut mem(), w, a, s)
    }
    /// `Native_01be_GetWaterBattleEffect(worker, args, stack)`.
    Native_01be_GetWaterBattleEffect @ 0x0085_a0e0 => extern "C" fn n_85a0e0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_01be(&mut mem(), w, a, s)
    }
    /// `Native_0262_AlwaysTrue(worker, args, stack)`.
    Native_0262_AlwaysTrue @ 0x0085_a110 => extern "C" fn n_85a110(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0262(&mut mem(), w, a, s)
    }
    /// `Native_0263_AlwaysTrue(worker, args, stack)`.
    Native_0263_AlwaysTrue @ 0x0085_a120 => extern "C" fn n_85a120(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0263(&mut mem(), w, a, s)
    }
    /// `Native_01bf_SetGlobal01326b81(worker, args, stack)`.
    Native_01bf_SetGlobal01326b81 @ 0x0085_a190 => extern "C" fn n_85a190(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01bf(&mut mem(), w, a, s)
    }
    /// `Native_0266_SaveEventGetBit5ec(worker, args, stack)`.
    Native_0266_SaveEventGetBit5ec @ 0x0085_a410 => extern "C" fn n_85a410(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0266(&mut mem(), w, a, s)
    }
    /// `Native_01c1_SaveEventStepE0To4(worker, args, stack)`.
    Native_01c1_SaveEventStepE0To4 @ 0x0085_a420 => extern "C" fn n_85a420(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_01c1(&mut mem(), w, a, s)
    }
    /// `Native_01c2_SaveEventStepE0To1(worker, args, stack)`.
    Native_01c2_SaveEventStepE0To1 @ 0x0085_a570 => extern "C" fn n_85a570(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01c2(&mut mem(), w, a, s)
    }
    /// `Native_0265_GetGlobal00cccacc(worker, args, stack)`.
    Native_0265_GetGlobal00cccacc @ 0x0085_a7f0 => extern "C" fn n_85a7f0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0265(&mut mem(), w, a, s)
    }
    /// `Native_01fb_GetGlobal0186a228(worker, args, stack)`.
    Native_01fb_GetGlobal0186a228 @ 0x0085_af10 => extern "C" fn n_85af10(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01fb(&mut mem(), w, a, s)
    }
    /// `Native_010e_GetGlobal011307a0(worker, args, stack)`.
    Native_010e_GetGlobal011307a0 @ 0x0085_af50 => extern "C" fn n_85af50(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_010e(&mut mem(), w, a, s)
    }
    /// `Native_01fa_GetGlobal00c5a088(worker, args, stack)`.
    Native_01fa_GetGlobal00c5a088 @ 0x0085_af60 => extern "C" fn n_85af60(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01fa(&mut mem(), w, a, s)
    }
    /// `Native_0111_PartyPushMembers(worker, args, stack)`.
    Native_0111_PartyPushMembers @ 0x0085_b4b0 => extern "C" fn n_85b4b0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0111(&mut mem(), w, a, s)
    }
    /// `Native_015b_GiveEventReward_Result(worker, args, stack)`.
    Native_015b_GiveEventReward_Result @ 0x0085_b4c0 => extern "C" fn n_85b4c0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_015b_result(&mut mem(), w, a, s)
    }
    /// `Native_0112_PartyPopMembers(worker, args, stack)`.
    Native_0112_PartyPopMembers @ 0x0085_b520 => extern "C" fn n_85b520(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0112(&mut mem(), w, a, s)
    }
    /// `Native_0113_GetSave20(worker, args, stack)`.
    Native_0113_GetSave20 @ 0x0085_b5c0 => extern "C" fn n_85b5c0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0113(&mut mem(), w, a, s)
    }
    /// `Native_020e_GetSphereMonitor(worker, args, stack)`.
    Native_020e_GetSphereMonitor @ 0x0085_c090 => extern "C" fn n_85c090(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_020e(&mut mem(), w, a, s)
    }
    /// `Native_007e_GetCtrlTopId(worker, args, stack)`.
    Native_007e_GetCtrlTopId @ 0x0085_c580 => extern "C" fn n_85c580(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_007e(&mut mem(), w, a, s)
    }
    /// `Native_0086_GetEventJumpMap(worker, args, stack)`.
    Native_0086_GetEventJumpMap @ 0x0085_cdb0 => extern "C" fn n_85cdb0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0086(&mut mem(), w, a, s)
    }
    /// `Native_0087_GetNowEventJumpId(worker, args, stack)`.
    Native_0087_GetNowEventJumpId @ 0x0085_cf90 => extern "C" fn n_85cf90(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0087(&mut mem(), w, a, s)
    }
    /// `Native_0088_GetLastEventJumpMapNo(worker, args, stack)`.
    Native_0088_GetLastEventJumpMapNo @ 0x0085_d160 => extern "C" fn n_85d160(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0088(&mut mem(), w, a, s)
    }
    /// `Native_0089_GetLastEventJumpId(worker, args, stack)`.
    Native_0089_GetLastEventJumpId @ 0x0085_d280 => extern "C" fn n_85d280(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0089(&mut mem(), w, a, s)
    }
    /// `Native_0217_BtlGetInit(worker, args, stack)`.
    Native_0217_BtlGetInit @ 0x0085_d3a0 => extern "C" fn n_85d3a0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0217(&mut mem(), w, a, s)
    }
    /// `Native_008a_GetMapCameraId(worker, args, stack)`.
    Native_008a_GetMapCameraId @ 0x0085_d3b0 => extern "C" fn n_85d3b0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_008a(&mut mem(), w, a, s)
    }
    /// `Native_0044_PadHeld(worker, args, stack)`.
    Native_0044_PadHeld @ 0x0085_d4e0 => extern "C" fn n_85d4e0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0044(&mut mem(), w, a, s)
    }
    /// `Native_0045_PadRepeat(worker, args, stack)`.
    Native_0045_PadRepeat @ 0x0085_d760 => extern "C" fn n_85d760(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0045(&mut mem(), w, a, s)
    }
    /// `Native_024f_PadMapButtonsHist1c(worker, args, stack)`.
    Native_024f_PadMapButtonsHist1c @ 0x0085_d890 => extern "C" fn n_85d890(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_024f(&mut mem(), w, a, s)
    }
    /// `Native_008c_CtrlGetRemoveClears(worker, args, stack)`.
    Native_008c_CtrlGetRemoveClears @ 0x0085_d8b0 => extern "C" fn n_85d8b0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_008c(&mut mem(), w, a, s)
    }
    /// `Native_0047_PadMapButtons(worker, args, stack)`.
    Native_0047_PadMapButtons @ 0x0085_db70 => extern "C" fn n_85db70(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0047(&mut mem(), w, a, s)
    }
    /// `Native_0048_PadHeldRaw(worker, args, stack)`.
    Native_0048_PadHeldRaw @ 0x0085_dcc0 => extern "C" fn n_85dcc0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0048(&mut mem(), w, a, s)
    }
    /// `Native_0049_PadRepeatRaw(worker, args, stack)`.
    Native_0049_PadRepeatRaw @ 0x0085_ddc0 => extern "C" fn n_85ddc0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0049(&mut mem(), w, a, s)
    }
    /// `Native_0250_PadGetButtonsLow1c(worker, args, stack)`.
    Native_0250_PadGetButtonsLow1c @ 0x0085_de60 => extern "C" fn n_85de60(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0250(&mut mem(), w, a, s)
    }
    /// `Native_01c6_GetGlobal00c58e89(worker, args, stack)`.
    Native_01c6_GetGlobal00c58e89 @ 0x0085_de70 => extern "C" fn n_85de70(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01c6(&mut mem(), w, a, s)
    }
    /// `Native_004a_PadPressedRaw(worker, args, stack)`.
    Native_004a_PadPressedRaw @ 0x0085_dff0 => extern "C" fn n_85dff0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_004a(&mut mem(), w, a, s)
    }
    /// `Native_01c7_GetCdSizeKb(worker, args, stack)`.
    Native_01c7_GetCdSizeKb @ 0x0085_e000 => extern "C" fn n_85e000(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_01c7(&mut mem(), w, a, s)
    }
    /// `Native_004b_PadGetButtonsLow(worker, args, stack)`.
    Native_004b_PadGetButtonsLow @ 0x0085_e0d0 => extern "C" fn n_85e0d0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_004b(&mut mem(), w, a, s)
    }
    /// `Native_01cc_GetNowPlayMusic(worker, args, stack)`.
    Native_01cc_GetNowPlayMusic @ 0x0085_e400 => extern "C" fn n_85e400(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_01cc(&mut mem(), w, a, s)
    }
    /// `Native_01cd_GetMusicNumber(worker, args, stack)`.
    Native_01cd_GetMusicNumber @ 0x0085_e5d0 => extern "C" fn n_85e5d0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_01cd(&mut mem(), w, a, s)
    }
    /// `Native_0224_SaveMenuGetBit4(worker, args, stack)`.
    Native_0224_SaveMenuGetBit4 @ 0x0085_e800 => extern "C" fn n_85e800(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_0224(&mut mem(), w, a, s)
    }
    /// `Native_0189_SetMapIdPage_Poll(worker, args, stack)`.
    Native_0189_SetMapIdPage_Poll @ 0x0085_ea30 => extern "C" fn n_85ea30(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0189_poll(&mut mem(), w, a, s)
    }
    /// `Native_0135_GetGil(worker, args, stack)`.
    Native_0135_GetGil @ 0x0085_ed00 => extern "C" fn n_85ed00(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0135(&mut mem(), w, a, s)
    }
    /// `Native_00dd_EventEffectGetStat(worker, args, stack)`.
    Native_00dd_EventEffectGetStat @ 0x0085_f070 => extern "C" fn n_85f070(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_00dd(&mut mem(), w, a, s)
    }
    /// `Native_0195_WaitGlobal01326b58_Poll(worker, args, stack)`.
    Native_0195_WaitGlobal01326b58_Poll @ 0x0085_fb00 => extern "C" fn n_85fb00(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0195_poll(&mut mem(), w, a, s)
    }
    /// `Native_0234_FlushAlbhedDic(worker, args, stack)`.
    Native_0234_FlushAlbhedDic @ 0x0085_fff0 => extern "C" fn n_85fff0(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0234(&mut mem(), w, a, s)
    }
    /// `Native_0235_GetGlobal01325b8c(worker, args, stack)`.
    Native_0235_GetGlobal01325b8c @ 0x0086_0170 => extern "C" fn n_860170(w: u32, a: u32, s: u32) -> u32 {
        native_move::native_0235(&mut mem(), w, a, s)
    }
    /// `Native_01d3_AlwaysTrue(worker, args, stack)`.
    Native_01d3_AlwaysTrue @ 0x0086_0720 => extern "C" fn n_860720(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_01d3(&mut mem(), w, a, s)
    }
    /// `Native_c011_StreamStart(worker, args, stack)`.
    Native_c011_StreamStart @ 0x0087_8790 => extern "C" fn n_878790(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_c011(&mut mem(), w, a, s)
    }
    /// `Native_c012_StreamGetState(worker, args, stack)`.
    Native_c012_StreamGetState @ 0x0087_87a0 => extern "C" fn n_8787a0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_c012(&mut mem(), w, a, s)
    }
    /// `Native_c013_SaveConfigHiragana(worker, args, stack)`.
    Native_c013_SaveConfigHiragana @ 0x0087_87b0 => extern "C" fn n_8787b0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_c013(&mut mem(), w, a, s)
    }
    /// `Native_c014_GlobalBitTest(worker, args, stack)`.
    Native_c014_GlobalBitTest @ 0x0087_87c0 => extern "C" fn n_8787c0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_c014(&mut mem(), w, a, s)
    }
    /// `Native_c024_BtlEnd_Poll(worker, args, stack)`.
    Native_c024_BtlEnd_Poll @ 0x0087_8c60 => extern "C" fn n_878c60(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_one(&mut mem(), w, a, s)
    }
    /// `Native_c030_CtrlGetRemoveClearsDebug(worker, args, stack)`.
    Native_c030_CtrlGetRemoveClearsDebug @ 0x0087_8ed0 => extern "C" fn n_878ed0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_c030(&mut mem(), w, a, s)
    }
    /// `Native_c033_EnableGetTexture(worker, args, stack)`.
    Native_c033_EnableGetTexture @ 0x0087_8f10 => extern "C" fn n_878f10(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_c033(&mut mem(), w, a, s)
    }
    /// `Native_c040_GetGlobal01330092(worker, args, stack)`.
    Native_c040_GetGlobal01330092 @ 0x0087_9150 => extern "C" fn n_879150(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_c040(&mut mem(), w, a, s)
    }
    /// `Native_c042_GetSoundType(worker, args, stack)`.
    Native_c042_GetSoundType @ 0x0087_9180 => extern "C" fn n_879180(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_c042(&mut mem(), w, a, s)
    }
    /// `Native_c043_SepClearAll(worker, args, stack)`.
    Native_c043_SepClearAll @ 0x0087_9190 => extern "C" fn n_879190(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_c043(&mut mem(), w, a, s)
    }
    /// `Native_c04f_AlwaysTrue(worker, args, stack)`.
    Native_c04f_AlwaysTrue @ 0x0087_9380 => extern "C" fn n_879380(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_01d3(&mut mem(), w, a, s)
    }
    /// `Native_c050_MovieStop(worker, args, stack)`.
    Native_c050_MovieStop @ 0x0087_9390 => extern "C" fn n_879390(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_c050(&mut mem(), w, a, s)
    }
    /// `Native_c052_GetTGS(worker, args, stack)`.
    Native_c052_GetTGS @ 0x0087_93e0 => extern "C" fn n_8793e0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_c052(&mut mem(), w, a, s)
    }
    /// `Native_9000_DebugPrint(worker, args, stack)`.
    Native_9000_DebugPrint @ 0x0090_7e60 => extern "C" fn n_907e60(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_9000(&mut mem(), w, a, s)
    }
    /// `Native_800e_MapPartGetCount(worker, args, stack)`.
    Native_800e_MapPartGetCount @ 0x0091_b190 => extern "C" fn n_91b190(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_800e(&mut mem(), w, a, s)
    }
    /// `Native_8024_SceneGetB8(worker, args, stack)`.
    Native_8024_SceneGetB8 @ 0x0091_b530 => extern "C" fn n_91b530(w: u32, a: u32, s: u32) -> f64 {
        native_mods::native_8024(&mut mem(), w, a, s)
    }
    /// `Native_8025_SceneGetBC(worker, args, stack)`.
    Native_8025_SceneGetBC @ 0x0091_b540 => extern "C" fn n_91b540(w: u32, a: u32, s: u32) -> f64 {
        native_mods::native_8025(&mut mem(), w, a, s)
    }
    /// `Native_8027_SceneWaveC0Stop(worker, args, stack)`.
    Native_8027_SceneWaveC0Stop @ 0x0091_b5c0 => extern "C" fn n_91b5c0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_8027(&mut mem(), w, a, s)
    }
    /// `Native_8029_SceneGetAC(worker, args, stack)`.
    Native_8029_SceneGetAC @ 0x0091_b600 => extern "C" fn n_91b600(w: u32, a: u32, s: u32) -> f64 {
        native_mods::native_8029(&mut mem(), w, a, s)
    }
    /// `Native_802b_ResetChain193bf80(worker, args, stack)`.
    Native_802b_ResetChain193bf80 @ 0x0091_b670 => extern "C" fn n_91b670(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_802b(&mut mem(), w, a, s)
    }
    /// `Native_8033_GuideMapGetLimit(worker, args, stack)`.
    Native_8033_GuideMapGetLimit @ 0x0091_b770 => extern "C" fn n_91b770(w: u32, a: u32, s: u32) -> f64 {
        native_mods::native_8033(&mut mem(), w, a, s)
    }
    /// `Native_8034_FpDispDebug(worker, args, stack)`.
    Native_8034_FpDispDebug @ 0x0091_b780 => extern "C" fn n_91b780(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_8034(&mut mem(), w, a, s)
    }
    /// `Native_8062_ClearGlobal018ded08Bit8(worker, args, stack)`.
    Native_8062_ClearGlobal018ded08Bit8 @ 0x0091_c5a0 => extern "C" fn n_91c5a0(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_8062(&mut mem(), w, a, s)
    }
    /// `Native_8068_FpM173End(worker, args, stack)`.
    Native_8068_FpM173End @ 0x0091_c890 => extern "C" fn n_91c890(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_8068(&mut mem(), w, a, s)
    }
    /// `Native_4031_GetCameraFollow(worker, args, stack)`.
    Native_4031_GetCameraFollow @ 0x00a7_7970 => extern "C" fn n_a77970(w: u32, a: u32, s: u32) -> u32 {
        native_mods::native_4031(&mut mem(), w, a, s)
    }
    /// `Native_4035_GetMapAlphaDropped(worker, args, stack)`.
    Native_4035_GetMapAlphaDropped @ 0x00a7_7a00 => extern "C" fn n_a77a00(w: u32, a: u32, s: u32) -> f64 {
        native_mods::native_4035(&mut mem(), w, a, s)
    }
    /// `Native_403f_CameraGetYaw(worker, args, stack)`.
    Native_403f_CameraGetYaw @ 0x00a7_7c20 => extern "C" fn n_a77c20(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_403f(&mut mem(), w, a, s)
    }
    /// `Native_400b_ClearGlobal0130074c(worker, args, stack)`.
    Native_400b_ClearGlobal0130074c @ 0x00a7_7e20 => extern "C" fn n_a77e20(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_400b(&mut mem(), w, a, s)
    }
    /// `Native_400c_FilterReset(worker, args, stack)`.
    Native_400c_FilterReset @ 0x00a7_7ed0 => extern "C" fn n_a77ed0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_400c(&mut mem(), w, a, s)
    }
    /// `Native_401f_CacheClear(worker, args, stack)`.
    Native_401f_CacheClear @ 0x00a7_82e0 => extern "C" fn n_a782e0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_401f(&mut mem(), w, a, s)
    }
    /// `Native_4020_CacheClearFast(worker, args, stack)`.
    Native_4020_CacheClearFast @ 0x00a7_82f0 => extern "C" fn n_a782f0(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_4020(&mut mem(), w, a, s)
    }
    /// `Native_4024_PLightDispose(worker, args, stack)`.
    Native_4024_PLightDispose @ 0x00a7_8360 => extern "C" fn n_a78360(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_4024(&mut mem(), w, a, s)
    }
    /// `Native_ChrLoadMotionGroup_Poll(worker, args, stack)`.
    Native_ChrLoadMotionGroup_Poll @ 0x00a7_9a10 => extern "C" fn n_a79a10(w: u32, a: u32, s: u32) -> u32 {
        native_chr::native_done(&mut mem(), w, a, s)
    }
    // ---- the jmp thunks (ffx_script::m5_thunks) ----
    /// `jmp FFXGetAsiaMode_007826a0` (`0x007826a0`): its `EAX`.
    FFXGetAsiaMode_Jmp_0086a7f0 @ 0x0086_a7f0 => extern "C" fn t_86a7f0() -> u32 {
        th::bridge::jmp_thunk(&mut mem(), const { th::target(0x0086_a7f0) }, &[])
    }
    /// `jmp TkGetLastEventJumpId` (`0x0088d600`): its `EAX`.
    TkGetLastEventJumpId_thunk_0086bc70 @ 0x0086_bc70 => extern "C" fn t_86bc70() -> u32 {
        th::bridge::jmp_thunk(&mut mem(), const { th::target(0x0086_bc70) }, &[])
    }
    /// `jmp TkGetLastEventJumpIndex` (`0x0088d610`): its `EAX`.
    TkGetLastEventJumpIndex_thunk_0086bc80 @ 0x0086_bc80 => extern "C" fn t_86bc80() -> u32 {
        th::bridge::jmp_thunk(&mut mem(), const { th::target(0x0086_bc80) }, &[])
    }
    /// `jmp TkGetNowEventJumpId` (`0x0088d660`): its `EAX`.
    TkGetNowEventJumpId_thunk_0086c1c0 @ 0x0086_c1c0 => extern "C" fn t_86c1c0() -> u32 {
        th::bridge::jmp_thunk(&mut mem(), const { th::target(0x0086_c1c0) }, &[])
    }
    /// `jmp TkGetLastEventJumpMapNo` (`0x0088d630`): its `EAX`.
    TkGetLastEventJumpMapNo_thunk_0086bca0 @ 0x0086_bca0 => extern "C" fn t_86bca0() -> u32 {
        th::bridge::jmp_thunk(&mut mem(), const { th::target(0x0086_bca0) }, &[])
    }
    /// `jmp MsGetSaveConfigHiragana` (`0x007851f0`): its `EAX`.
    MsGetSaveConfigHiragana_thunk_0086bad0 @ 0x0086_bad0 => extern "C" fn t_86bad0() -> u32 {
        th::bridge::jmp_thunk(&mut mem(), const { th::target(0x0086_bad0) }, &[])
    }
    /// `jmp MsGetSaveConfigEnglish` (`0x007851d0`): its `EAX`.
    MsGetSaveConfigEnglish_thunk_0086bbc0 @ 0x0086_bbc0 => extern "C" fn t_86bbc0() -> u32 {
        th::bridge::jmp_thunk(&mut mem(), const { th::target(0x0086_bbc0) }, &[])
    }
    /// `jmp MsBattleCheck` (`0x00780c90`): its `EAX`.
    MsBattleCheck_thunk_0086d940 @ 0x0086_d940 => extern "C" fn t_86d940() -> u32 {
        th::bridge::jmp_thunk(&mut mem(), const { th::target(0x0086_d940) }, &[])
    }
    /// `jmp FUN_00873d60` (`0x00873d60`): its `EAX`.
    FUN_00873d60_thunk_0086ed70 @ 0x0086_ed70 => extern "C" fn t_86ed70() -> u32 {
        th::bridge::jmp_thunk(&mut mem(), const { th::target(0x0086_ed70) }, &[])
    }
    /// `jmp AtelClearShape` (`0x008742b0`): its `EAX`.
    AtelClearShape_thunk_00874830 @ 0x0087_4830 => extern "C" fn t_874830() -> u32 {
        th::bridge::jmp_thunk(&mut mem(), const { th::target(0x0087_4830) }, &[])
    }
    /// `jmp AtelRemoveCtrlWork` (`0x0086ed40`): its `EAX`.
    AtelRemoveCtrlWork_thunk_00875ad0 @ 0x0087_5ad0 => extern "C" fn t_875ad0(a0: u32) -> u32 {
        th::bridge::jmp_thunk(&mut mem(), const { th::target(0x0087_5ad0) }, &[a0])
    }
    // ---- the debug natives (ffx_script::vm::bridge::native_m5) ----
    /// `Native_b002_DebugPrint(worker, args, stack)`.
    Native_b002_DebugPrint @ 0x0076_ea30 => extern "C" fn n_76ea30(w: u32, a: u32, s: u32) -> u32 {
        native_m5::native_b002(&mut mem(), w, a, s)
    }
    /// `Native_70b1_DebugPrint(worker, args, stack)`.
    Native_70b1_DebugPrint @ 0x007a_4a60 => extern "C" fn n_7a4a60(w: u32, a: u32, s: u32) -> u32 {
        native_m5::native_70b1(&mut mem(), w, a, s)
    }
    /// `Native_705f_DebugPrint(worker, args, stack)`.
    Native_705f_DebugPrint @ 0x007a_4c30 => extern "C" fn n_7a4c30(w: u32, a: u32, s: u32) -> u32 {
        native_m5::native_705f(&mut mem(), w, a, s)
    }
    /// `Native_00f8_DebugVarMemset(worker, args, stack)`.
    Native_00f8_DebugVarMemset @ 0x0085_8520 => extern "C" fn n_858520(w: u32, a: u32, s: u32) -> u32 {
        native_m5::native_00f8(&mut mem(), w, a, s)
    }
    /// `Native_c006_DebugClearGlobal01301804(worker, args, stack)`.
    Native_c006_DebugClearGlobal01301804 @ 0x0087_85e0 => extern "C" fn n_8785e0(w: u32, a: u32, s: u32) -> u32 {
        native_m5::native_c006(&mut mem(), w, a, s)
    }
    /// `Native_c027_DebugCollectBits(worker, args, stack)`.
    Native_c027_DebugCollectBits @ 0x0087_8d20 => extern "C" fn n_878d20(w: u32, a: u32, s: u32) -> u32 {
        native_m5::native_c027(&mut mem(), w, a, s)
    }
    /// `Native_c028_ImportantSetAll(worker, args, stack)`.
    Native_c028_ImportantSetAll @ 0x0087_8d60 => extern "C" fn n_878d60(w: u32, a: u32, s: u32) -> u32 {
        native_m5::native_c028(&mut mem(), w, a, s)
    }
    /// `Native_c03b_FullItem(worker, args, stack)`.
    Native_c03b_FullItem @ 0x0087_90b0 => extern "C" fn n_8790b0(w: u32, a: u32, s: u32) -> u32 {
        native_m5::native_c03b(&mut mem(), w, a, s)
    }
    /// `Native_c03c_FullCommand(worker, args, stack)`.
    Native_c03c_FullCommand @ 0x0087_90c0 => extern "C" fn n_8790c0(w: u32, a: u32, s: u32) -> u32 {
        native_m5::native_c03c(&mut mem(), w, a, s)
    }
    /// `Native_4038_MemDump(worker, args, stack)`.
    Native_4038_MemDump @ 0x00a7_7ab0 => extern "C" fn n_a77ab0(w: u32, a: u32, s: u32) -> u32 {
        native_m5::native_4038(&mut mem(), w, a, s)
    }
    /// `Native_401e_SgTest(worker, args, stack)`.
    Native_401e_SgTest @ 0x00a7_82c0 => extern "C" fn n_a782c0(w: u32, a: u32, s: u32) -> u32 {
        native_m5::native_401e(&mut mem(), w, a, s)
    }
    /// `Native_c034_EnableSetTexture(worker, args, stack)`.
    Native_c034_EnableSetTexture @ 0x0087_8f20 => extern "C" fn n_878f20(w: u32, a: u32, s: u32) -> u32 {
        native_m5::native_c034(&mut mem(), w, a, s)
    }
    /// `Native_c022_EnableDrawGui(worker, args, stack)`.
    Native_c022_EnableDrawGui @ 0x0087_8b00 => extern "C" fn n_878b00(w: u32, a: u32, s: u32) -> u32 {
        native_m5::native_c022(&mut mem(), w, a, s)
    }
    // ---- round 4 (menu-battle-script 2): host-file and capture natives ----
    /// `Native_c057_CopyToSaveRam(worker, args, stack)`.
    Native_c057_CopyToSaveRam @ 0x0087_81f0 => extern "C" fn n_8781f0(w: u32, a: u32, s: u32) -> u32 {
        native_m5::native_c057(&mut mem(), w, a, s)
    }
    /// `Native_c056_CdRead(worker, args, stack)`.
    Native_c056_CdRead @ 0x0087_8230 => extern "C" fn n_878230(w: u32, a: u32, s: u32) -> u32 {
        native_m5::native_c056(&mut mem(), w, a, s)
    }
    /// `Native_c05b_DebugPs2String(worker, args, stack)`.
    Native_c05b_DebugPs2String @ 0x0087_8330 => extern "C" fn n_878330(w: u32, a: u32, s: u32) -> u32 {
        native_m5::native_c05b(&mut mem(), w, a, s)
    }
    /// `Native_c05c_DebugPs2String(worker, args, stack)`.
    Native_c05c_DebugPs2String @ 0x0087_83c0 => extern "C" fn n_8783c0(w: u32, a: u32, s: u32) -> u32 {
        native_m5::native_c05c(&mut mem(), w, a, s)
    }
    /// `Native_c000_SaveEventToHost(worker, args, stack)`.
    Native_c000_SaveEventToHost @ 0x0087_8420 => extern "C" fn n_878420(w: u32, a: u32, s: u32) -> u32 {
        native_m5::native_c000(&mut mem(), w, a, s)
    }
    /// `Native_c001_SaveEventFromHost(worker, args, stack)`.
    Native_c001_SaveEventFromHost @ 0x0087_8490 => extern "C" fn n_878490(w: u32, a: u32, s: u32) -> u32 {
        native_m5::native_c001(&mut mem(), w, a, s)
    }
    /// `Native_c004_CaptureEnableKari(worker, args, stack)`.
    Native_c004_CaptureEnableKari @ 0x0087_8570 => extern "C" fn n_878570(w: u32, a: u32, s: u32) -> u32 {
        native_m5::native_c004(&mut mem(), w, a, s)
    }
    /// `Native_c005_CaptureEnableKariOn(worker, args, stack)`.
    Native_c005_CaptureEnableKariOn @ 0x0087_85b0 => extern "C" fn n_8785b0(w: u32, a: u32, s: u32) -> u32 {
        native_m5::native_c005(&mut mem(), w, a, s)
    }
    /// `Native_c00c_LoadFieldParticle(worker, args, stack)`.
    Native_c00c_LoadFieldParticle @ 0x0087_8650 => extern "C" fn n_878650(w: u32, a: u32, s: u32) -> u32 {
        native_m5::native_c00c(&mut mem(), w, a, s)
    }
    /// `Native_c00b_LoadBtlFieldParticle(worker, args, stack)`.
    Native_c00b_LoadBtlFieldParticle @ 0x0087_8690 => extern "C" fn n_878690(w: u32, a: u32, s: u32) -> u32 {
        native_m5::native_c00b(&mut mem(), w, a, s)
    }
    /// `Native_c015_SepTransFromHost(worker, args, stack)`.
    Native_c015_SepTransFromHost @ 0x0087_87d0 => extern "C" fn n_8787d0(w: u32, a: u32, s: u32) -> u32 {
        native_m5::native_c015(&mut mem(), w, a, s)
    }
    /// `Native_c018_SqsSetProjectBase(worker, args, stack)`.
    Native_c018_SqsSetProjectBase @ 0x0087_88a0 => extern "C" fn n_8788a0(w: u32, a: u32, s: u32) -> u32 {
        native_m5::native_c018(&mut mem(), w, a, s)
    }
    /// `Native_c02a_OutputEncountStart(worker, args, stack)`.
    Native_c02a_OutputEncountStart @ 0x0087_8e10 => extern "C" fn n_878e10(w: u32, a: u32, s: u32) -> u32 {
        native_m5::native_c02a(&mut mem(), w, a, s)
    }
    // ---- round 4 (menu-battle-script 2), batch 3 (ffx_menu::m5m_l, m5m_m) ----
    /// `(worker)`: the motion number, a 0x24-byte frame.
    AtelDebug_PrintMotionNo_0087b0c0 @ 0x0087_b0c0 => extern "C" fn print_motion_no(w: u32) -> u32 {
        let mut fr = StackFrame::<9>::new();
        let ebp = fr.at() + 0x24;
        l::print_motion_no(&mut mem(), ebp, w)
    }
    /// `(worker)`: the state.
    AtelDebug_PrintState_0087b170 @ 0x0087_b170 => extern "C" fn print_state(w: u32) -> u32 {
        let mut fr = StackFrame::<9>::new();
        let ebp = fr.at() + 0x24;
        l::print_state(&mut mem(), ebp, w)
    }
    /// `(worker)`: the byte `+0xaf`.
    AtelDebug_PrintByteAF_0087b360 @ 0x0087_b360 => extern "C" fn print_byte_af(w: u32) -> u32 {
        let mut fr = StackFrame::<9>::new();
        let ebp = fr.at() + 0x24;
        l::print_byte_af(&mut mem(), ebp, w)
    }
    /// `(worker, i)`: slot `i`.
    AtelDebug_PrintSlot_0087ee10 @ 0x0087_ee10 => extern "C" fn print_slot(w: u32, i: u32) -> u32 {
        let mut fr = StackFrame::<9>::new();
        let ebp = fr.at() + 0x24;
        l::print_slot(&mut mem(), ebp, w, i)
    }
    /// `(worker)`: slot 0.
    AtelDebug_PrintSlot0_0087b3e0 @ 0x0087_b3e0 => extern "C" fn print_slot0(w: u32) -> u32 {
        l::print_slot_n(&mut mem(), w, 0)
    }
    /// `(worker)`: slot 1.
    AtelDebug_PrintSlot1_0087b400 @ 0x0087_b400 => extern "C" fn print_slot1(w: u32) -> u32 {
        l::print_slot_n(&mut mem(), w, 1)
    }
    /// `(worker)`: the body hit.
    AtelDebug_PrintBodyHit_0087b420 @ 0x0087_b420 => extern "C" fn print_body_hit(w: u32) -> u32 {
        l::print_body_hit(&mut mem(), w)
    }
    /// `(worker)`: the thickness, a 0x28-byte frame.
    AtelDebug_PrintThickness_0087b490 @ 0x0087_b490 => extern "C" fn print_thickness(w: u32) -> u32 {
        let mut fr = StackFrame::<10>::new();
        let ebp = fr.at() + 0x28;
        l::print_thickness(&mut mem(), ebp, w)
    }
    /// `(worker)`: the position work's `+0x2c`.
    AtelDebug_PrintPos2C_0087b520 @ 0x0087_b520 => extern "C" fn print_pos_2c(w: u32) -> u32 {
        let mut fr = StackFrame::<9>::new();
        let ebp = fr.at() + 0x24;
        l::print_pos_2c(&mut mem(), ebp, w)
    }
    /// `(worker)`: flag `+0x38` bit 5.
    AtelDebug_PrintFlag38_20_0087b580 @ 0x0087_b580 => extern "C" fn print_flag_b580(w: u32) -> u32 {
        l::print_flag(&mut mem(), ffx_menu::m5m_l::FLAGS[0].1, w)
    }
    /// `(worker)`: flag `+0x38` bit 6.
    AtelDebug_PrintFlag38_40_0087b5b0 @ 0x0087_b5b0 => extern "C" fn print_flag_b5b0(w: u32) -> u32 {
        l::print_flag(&mut mem(), ffx_menu::m5m_l::FLAGS[1].1, w)
    }
    /// `(worker)`: flag `+0x38` bit 7.
    AtelDebug_PrintFlag38_80_0087b5e0 @ 0x0087_b5e0 => extern "C" fn print_flag_b5e0(w: u32) -> u32 {
        l::print_flag(&mut mem(), ffx_menu::m5m_l::FLAGS[2].1, w)
    }
    /// `(worker)`: the level byte.
    AtelDebug_PrintLevel_0087b610 @ 0x0087_b610 => extern "C" fn print_level(w: u32) -> u32 {
        let mut fr = StackFrame::<9>::new();
        let ebp = fr.at() + 0x24;
        l::print_level(&mut mem(), ebp, w)
    }
    /// `()`: the control work's player.
    AtelDebug_PrintCtrlPlayer_0087b660 @ 0x0087_b660 => extern "C" fn print_ctrl_player() -> u32 {
        l::print_ctrl_player(&mut mem())
    }
    /// `()`: control work `+0x218`.
    AtelDebug_PrintCtrl218_0087b6b0 @ 0x0087_b6b0 => extern "C" fn print_ctrl_218() -> u32 {
        let mut fr = StackFrame::<9>::new();
        let ebp = fr.at() + 0x24;
        l::print_ctrl_float(&mut mem(), ebp, 0x218)
    }
    /// `()`: control work `+0x21c`.
    AtelDebug_PrintCtrl21C_0087b710 @ 0x0087_b710 => extern "C" fn print_ctrl_21c() -> u32 {
        let mut fr = StackFrame::<9>::new();
        let ebp = fr.at() + 0x24;
        l::print_ctrl_float(&mut mem(), ebp, 0x21c)
    }
    /// `()`: control work `+0x220`.
    AtelDebug_PrintCtrl220_0087b770 @ 0x0087_b770 => extern "C" fn print_ctrl_220() -> u32 {
        let mut fr = StackFrame::<9>::new();
        let ebp = fr.at() + 0x24;
        l::print_ctrl_float(&mut mem(), ebp, 0x220)
    }
    /// `()`: control work `+0x228`.
    AtelDebug_PrintCtrl228_0087b7d0 @ 0x0087_b7d0 => extern "C" fn print_ctrl_228() -> u32 {
        let mut fr = StackFrame::<9>::new();
        let ebp = fr.at() + 0x24;
        l::print_ctrl_float(&mut mem(), ebp, 0x228)
    }
    /// `()`: control work `+0x22c`.
    AtelDebug_PrintCtrl22C_0087b830 @ 0x0087_b830 => extern "C" fn print_ctrl_22c() -> u32 {
        let mut fr = StackFrame::<9>::new();
        let ebp = fr.at() + 0x24;
        l::print_ctrl_float(&mut mem(), ebp, 0x22c)
    }
    /// `()`: control work `+0x230`.
    AtelDebug_PrintCtrl230_0087b890 @ 0x0087_b890 => extern "C" fn print_ctrl_230() -> u32 {
        let mut fr = StackFrame::<9>::new();
        let ebp = fr.at() + 0x24;
        l::print_ctrl_float(&mut mem(), ebp, 0x230)
    }
    /// `(worker)`: the worker bits' name.
    AtelDebug_PrintBitsName_0087b8f0 @ 0x0087_b8f0 => extern "C" fn print_bits_name(w: u32) -> u32 {
        l::print_bits_name(&mut mem(), w)
    }
    /// `(worker)`: the `+0x34` flags.
    AtelDebug_PrintFlags34_0087b920 @ 0x0087_b920 => extern "C" fn print_flags_34(w: u32) -> u32 {
        l::print_flags_34(&mut mem(), w)
    }
    /// `(worker)`: the `+0x38` flags.
    AtelDebug_PrintFlags38_0087b980 @ 0x0087_b980 => extern "C" fn print_flags_38(w: u32) -> u32 {
        l::print_flags_38(&mut mem(), w)
    }
    /// `(worker)`: the program counter.
    AtelDebug_PrintPc_0087b9c0 @ 0x0087_b9c0 => extern "C" fn print_pc(w: u32) -> u32 {
        let mut fr = StackFrame::<9>::new();
        let ebp = fr.at() + 0x24;
        l::print_pc(&mut mem(), ebp, w)
    }
    /// `(worker)`: flag `+0x34` bit 1, colour 0.
    AtelDebug_PrintFlag34_02_0087ba70 @ 0x0087_ba70 => extern "C" fn print_flag_ba70(w: u32) -> u32 {
        l::print_flag(&mut mem(), ffx_menu::m5m_l::FLAGS[3].1, w)
    }
    /// `(worker)`: the byte `+0x33`.
    AtelDebug_PrintByte33_0087baa0 @ 0x0087_baa0 => extern "C" fn print_byte_33(w: u32) -> u32 {
        let mut fr = StackFrame::<9>::new();
        let ebp = fr.at() + 0x24;
        l::print_byte_33(&mut mem(), ebp, w)
    }
    /// `(worker)`: two flags, colour 0.
    AtelDebug_PrintFlagsCtrl_0087baf0 @ 0x0087_baf0 => extern "C" fn print_flags_ctrl(w: u32) -> u32 {
        l::print_flags_ctrl(&mut mem(), w)
    }
    /// `(worker)`: the word `+0xc4`.
    AtelDebug_PrintWordC4_0087bb40 @ 0x0087_bb40 => extern "C" fn print_word_c4(w: u32) -> u32 {
        let mut fr = StackFrame::<9>::new();
        let ebp = fr.at() + 0x24;
        l::print_word_c4(&mut mem(), ebp, w)
    }
    /// `(worker)`: the list's length.
    AtelDebug_PrintListLen_0087bb90 @ 0x0087_bb90 => extern "C" fn print_list_len(w: u32) -> u32 {
        let mut fr = StackFrame::<9>::new();
        let ebp = fr.at() + 0x24;
        l::print_list_len(&mut mem(), ebp, w)
    }
    /// `(worker)`: flag `+0x34` bit 7.
    AtelDebug_PrintFlag34_80_0087bc20 @ 0x0087_bc20 => extern "C" fn print_flag_bc20(w: u32) -> u32 {
        l::print_flag(&mut mem(), ffx_menu::m5m_l::FLAGS[4].1, w)
    }
    /// `(s)`: right aligned.
    AtelDebug_PrintRight_0087e6d0 @ 0x0087_e6d0 => extern "C" fn print_right(s: u32) -> u32 {
        l::print_right(&mut mem(), s)
    }
    /// `AtelSetUpSourceDebug(src, len, on, page, size)`.
    AtelSetUpSourceDebug @ 0x0088_5b00 => extern "C" fn atel_set_up_source_debug(src: u32, len: u32, on: u32, page: u32, size: u32) -> u32 {
        l::set_up_source_debug(&mut mem(), src, len, on, page, size)
    }
    /// `FFXUsbMouseSync(mode)`: mode 1 writes the caller's argument word.
    FFXUsbMouseSync @ 0x0085_5bf0 => extern "C" fn ffx_usb_mouse_sync(mode: u32; slots: u32) -> u32 {
        mm::usb_mouse_sync(&mut mem(), mode, slots)
    }
    /// `FFXUsbMouse_IopMemGet(dst)`.
    FFXUsbMouse_IopMemGet @ 0x0085_5c40 => extern "C" fn ffx_usb_mouse_iop_mem_get(dst: u32) -> u32 {
        mm::usb_mouse_iop_mem_get(&mut mem(), dst)
    }
    /// `FFXUsbMouseRead2(arg, dst)`.
    FFXUsbMouseRead2 @ 0x0085_5bc0 => extern "C" fn ffx_usb_mouse_read2(arg: u32, dst: u32) -> u32 {
        mm::usb_mouse_read2(&mut mem(), arg, dst)
    }
    /// `(this)`: the job queues' destructor.
    AsyncJobQueue_dtor_006db730 @ 0x006d_b730 => extern "thiscall" fn async_job_queue_dtor(this: u32) -> u32 {
        mm::job_queue_dtor(&mut mem(), this)
    }
    /// `StartThread_ps2(thread, arg)`.
    StartThread_ps2 @ 0x006f_2d30 => extern "C" fn start_thread_ps2(t: u32, arg: u32) -> u32 {
        mm::start_thread(&mut mem(), t, arg)
    }
    /// `CreateThread_ps2(params, name)`.
    CreateThread_ps2 @ 0x006f_2e10 => extern "C" fn create_thread_ps2(params: u32, name: u32) -> u32 {
        mm::create_thread(&mut mem(), params, name)
    }
    /// `(handle)`.
    DeleteThread_ps2_006f2f00 @ 0x006f_2f00 => extern "C" fn delete_thread_ps2(handle: u32) -> u32 {
        mm::delete_thread(&mut mem(), handle)
    }
    /// `(count)`.
    CreateSema_ps2_006f2b10 @ 0x006f_2b10 => extern "C" fn create_sema_ps2(count: u32) -> u32 {
        mm::create_sema(&mut mem(), count)
    }
    /// `(handle, prio)`.
    ChangeThreadPriority_ps2_006f3050 @ 0x006f_3050 => extern "C" fn change_thread_priority_ps2(handle: u32, prio: u32) -> u32 {
        mm::set_thread_priority(&mut mem(), handle, prio)
    }
    /// `(this; job, q)`.
    AsyncJobQueue_push_006db800 @ 0x006d_b800 => extern "thiscall" fn async_job_queue_push(this: u32, job: u32, q: u32) -> u32 {
        mm::job_push(&mut mem(), this, job, q)
    }
    /// `(this; q)`.
    AsyncJobQueue_pop_006db8a0 @ 0x006d_b8a0 => extern "thiscall" fn async_job_queue_pop(this: u32, q: u32) -> u32 {
        mm::job_pop(&mut mem(), this, q)
    }
    /// `(this; flags)`.
    Ps2Thread_dtor_006f29a0 @ 0x006f_29a0 => extern "thiscall" fn ps2_thread_dtor(this: u32, flags: u32) -> u32 {
        mm::thread_dtor(&mut mem(), this, flags)
    }
    /// `(handle)`.
    PollSema_ps2_006f2d50 @ 0x006f_2d50 => extern "C" fn poll_sema_ps2(handle: u32) -> u32 {
        mm::poll_sema(&mut mem(), handle)
    }
    /// `Mutex_Create()`.
    Mutex_Create @ 0x006f_2de0 => extern "C" fn mutex_create() -> u32 {
        mm::mutex_create(&mut mem())
    }
    /// `Mutex_Lock(m)`.
    Mutex_Lock @ 0x006f_2ed0 => extern "C" fn mutex_lock(m: u32) -> u32 {
        mm::mutex_lock(&mut mem(), m)
    }
    /// `Mutex_Unlock(m)`.
    Mutex_Unlock @ 0x006f_2f50 => extern "C" fn mutex_unlock(m: u32) -> u32 {
        mm::mutex_unlock(&mut mem(), m)
    }
    /// `(handle)`.
    SignalSema_copy_006f3000 @ 0x006f_3000 => extern "C" fn signal_sema_copy(handle: u32) -> u32 {
        mm::signal_sema(&mut mem(), handle)
    }
    /// `(path)`: the overlay menu, once.
    OverlayMenu_create_006f3130 @ 0x006f_3130 => extern "C" fn overlay_menu_create(path: u32) {
        mm::overlay_menu_create(&mut mem(), path);
    }
    /// `(buf, size)`.
    printf2_dump_0076e890 @ 0x0076_e890 => extern "C" fn printf2_dump(buf: u32, size: u32) -> u32 {
        mm::dump_write(&mut mem(), buf, size)
    }
    /// `AtelSetCtrlChar(id)`.
    AtelSetCtrlChar @ 0x0086_f580 => extern "C" fn atel_set_ctrl_char(id: u32) -> u32 {
        mm::set_ctrl_char(&mut mem(), id)
    }
    /// `()`.
    SaveEvent_SetStep6_00873db0 @ 0x0087_3db0 => extern "C" fn save_event_step6() -> u32 {
        mm::save_event_step6(&mut mem())
    }
    /// `(worker, i)`.
    Atel_GetWorkerTableWord_0086c250 @ 0x0086_c250 => extern "C" fn worker_table_word(w: u32, i: u32) -> u32 {
        mm::worker_table_word(&mut mem(), w, i)
    }
    /// `()`: byte `0x01326b81` = 1.
    AtelSetGlobal01326b81_008708d0 @ 0x0087_08d0 => extern "C" fn set_global_01326b81() {
        mm::set_global_01326b81(&mut mem());
    }
    /// `()`: step 4.
    SaveEvent_SetStep4_00873c40 @ 0x0087_3c40 => extern "C" fn save_event_step4() -> u32 {
        mm::save_event_step(&mut mem(), 4)
    }
    /// `AtelRemovePS2Debug()`.
    AtelRemovePS2Debug @ 0x0087_9e70 => extern "C" fn atel_remove_ps2_debug() -> u32 {
        mm::remove_ps2_debug(&mut mem())
    }
    /// `()`: step 1.
    SaveEvent_SetStep1_00873d30 @ 0x0087_3d30 => extern "C" fn save_event_step1() -> u32 {
        mm::save_event_step(&mut mem(), 1)
    }
    /// `()`: the word at `0x01326b58`.
    AtelGetGlobal01326b58_0086b060 @ 0x0086_b060 => extern "C" fn global_01326b58() -> u32 {
        mm::global_01326b58(&mem())
    }
    /// `()`: an empty string's address.
    AtelGetBuildDate_00860770 @ 0x0086_0770 => extern "C" fn empty_string_00860770() -> u32 {
        mm::rdata_address(&mem(), 0x00b5_b9fc)
    }
    /// `(keep)`.
    AtelRegistDebugCallbacks_00861440 @ 0x0086_1440 => extern "C" fn regist_debug_callbacks(keep: u32) -> u32 {
        mm::regist_debug_callbacks(&mut mem(), keep)
    }
    /// `()`.
    AtelClearDebugger_0087f1c0 @ 0x0087_f1c0 => extern "C" fn atel_clear_debugger() -> u32 {
        mm::clear_debugger(&mut mem())
    }
    /// `()`.
    AtelInitDebugger_0087f0d0 @ 0x0087_f0d0 => extern "C" fn atel_init_debugger() -> u32 {
        mm::init_debugger(&mut mem())
    }
    /// `AtelInitPS2Debug()`.
    AtelInitPS2Debug @ 0x0087_98f0 => extern "C" fn atel_init_ps2_debug() -> u32 {
        mm::init_ps2_debug(&mut mem())
    }
    /// `()`.
    AtelInitTotalDebug_00873ba0 @ 0x0087_3ba0 => extern "C" fn atel_init_total_debug() -> u32 {
        mm::init_total_debug(&mut mem())
    }
    /// `()`: an empty string's address.
    AtelGetBuildDate_00878130 @ 0x0087_8130 => extern "C" fn empty_string_00878130() -> u32 {
        mm::rdata_address(&mem(), 0x00b5_b9fc)
    }
    /// `AtelPCClose(fd)`: writes the caller's argument word.
    AtelPCClose @ 0x0087_9c30 => extern "C" fn atel_pc_close(fd: u32; slots: u32) -> u32 {
        mm::pc_close(&mut mem(), fd, slots)
    }
    /// `AtelPCCreate(name)`: the path in a frame.
    AtelPCCreate @ 0x0087_9c60 => extern "C" fn atel_pc_create(name: u32) -> u32 {
        let mut fr = StackFrame::<{ 0x104 / 4 }>::new();
        let buf = fr.at();
        mm::pc_create(&mut mem(), buf, name)
    }
    /// `AtelPCWrite(fd, buf, n)`.
    AtelPCWrite @ 0x0087_9e50 => extern "C" fn atel_pc_write(fd: u32, buf: u32, n: u32) -> u32 {
        mm::pc_forward(&mut mem(), mm::SCE_WRITE, fd, buf, n)
    }
    /// `AtelPCOpen(name)`: the path in a frame.
    AtelPCOpen @ 0x0087_9cf0 => extern "C" fn atel_pc_open(name: u32) -> u32 {
        let mut fr = StackFrame::<{ 0x104 / 4 }>::new();
        let buf = fr.at();
        mm::pc_open(&mut mem(), buf, name)
    }
    /// `AtelPCRead(fd, buf, n)`.
    AtelPCRead @ 0x0087_9de0 => extern "C" fn atel_pc_read(fd: u32, buf: u32, n: u32) -> u32 {
        mm::pc_read(&mut mem(), fd, buf, n)
    }
    /// `AtelPCSize(fd)`.
    AtelPCSize @ 0x0087_9e10 => extern "C" fn atel_pc_size(fd: u32) -> u32 {
        mm::pc_size(&mut mem(), fd)
    }
    /// `AtelPCOpen2(name)`: the path in a frame.
    AtelPCOpen2 @ 0x0087_9d60 => extern "C" fn atel_pc_open2(name: u32) -> u32 {
        let mut fr = StackFrame::<{ 0x104 / 4 }>::new();
        let buf = fr.at();
        mm::pc_open2(&mut mem(), buf, name)
    }
    /// `AtelPClseek(fd, off, whence)`.
    AtelPClseek @ 0x0087_9e60 => extern "C" fn atel_pc_lseek(fd: u32, off: u32, whence: u32) -> u32 {
        mm::pc_forward(&mut mem(), mm::SCE_LSEEK, fd, off, whence)
    }
    /// `AtelOutputEncountStart()`: the line in a frame.
    AtelOutputEncountStart @ 0x0088_5a80 => extern "C" fn atel_output_encount_start() -> u32 {
        let mut fr = StackFrame::<{ 0x404 / 4 }>::new();
        let buf = fr.at();
        mm::output_encount_start(&mut mem(), buf)
    }
    /// `()`: an empty string's address.
    AtelGetBuildDate_00879420 @ 0x0087_9420 => extern "C" fn empty_string_00879420() -> u32 {
        mm::rdata_address(&mem(), 0x00b5_b9fc)
    }
    /// `AtelGetEventName(i)`: the index entry in a frame.
    AtelGetEventName @ 0x0087_9760 => extern "C" fn atel_get_event_name(i: u32) -> u32 {
        let mut fr = StackFrame::<2>::new();
        let entry = fr.at();
        mm::get_event_name(&mut mem(), entry, i)
    }
    /// `AtelLoadDebugData(slot, event)`.
    AtelLoadDebugData @ 0x0087_99b0 => extern "C" fn atel_load_debug_data(slot: u32, event: u32) -> u32 {
        mm::load_debug_data(&mut mem(), slot, event)
    }
    /// `(a, _, b, name)`.
    Atel_LoadSourceFile_00879b60 @ 0x0087_9b60 => extern "C" fn atel_load_source_file(a: u32, _x: u32, b: u32, name: u32) -> u32 {
        mm::load_source_file(&mut mem(), a, b, name)
    }
}
