//! M5 stage 1, list `static-init-c`: the compiler-generated constructors
//! of globals and the destructors they register (run before `WinMain`, the
//! destructors at exit). Each shim calls its row of
//! `ffx_pload::m5_static_init_c` (the family tables), so one shim per
//! address; the parity tests are `crates/ffx-oracle/tests/m5_static_init_c.rs`.

use ffx_pload::m5_static_init_c as si;

use super::{StackFrame, mem, ported};

ported! {
    /// `StaticInit_Key_PageDown_00af6c50`: type-enum value `"Key_PageDown"`: when `0x00a2e780` finds no such name, the node `0x00c6be34` goes on the head of its list `0x01a85390`; `atexit(0x00b0a240)`.
    StaticInit_Key_PageDown_00af6c50 @ 0x00af_6c50 => extern "C" fn s_00af6c50() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[0])
    }
    /// `StaticInit_Key_PageUp_00af6c90`: type-enum value `"Key_PageUp"`: when `0x00a2e780` finds no such name, the node `0x00c6be44` goes on the head of its list `0x01a85390`; `atexit(0x00b0a260)`.
    StaticInit_Key_PageUp_00af6c90 @ 0x00af_6c90 => extern "C" fn s_00af6c90() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[1])
    }
    /// `StaticInit_Key_Period_00af6cd0`: type-enum value `"Key_Period"`: when `0x00a2e780` finds no such name, the node `0x00c6bec4` goes on the head of its list `0x01a85390`; `atexit(0x00b0a280)`.
    StaticInit_Key_Period_00af6cd0 @ 0x00af_6cd0 => extern "C" fn s_00af6cd0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[2])
    }
    /// `StaticInit_Key_Q_00af6d10`: type-enum value `"Key_Q"`: when `0x00a2e780` finds no such name, the node `0x00c6c194` goes on the head of its list `0x01a85390`; `atexit(0x00b0a2a0)`.
    StaticInit_Key_Q_00af6d10 @ 0x00af_6d10 => extern "C" fn s_00af6d10() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[3])
    }
    /// `StaticInit_Key_R_00af6d50`: type-enum value `"Key_R"`: when `0x00a2e780` finds no such name, the node `0x00c6c184` goes on the head of its list `0x01a85390`; `atexit(0x00b0a2c0)`.
    StaticInit_Key_R_00af6d50 @ 0x00af_6d50 => extern "C" fn s_00af6d50() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[4])
    }
    /// `StaticInit_Key_RightBracket_00af6d90`: type-enum value `"Key_RightBracket"`: when `0x00a2e780` finds no such name, the node `0x00c6bf14` goes on the head of its list `0x01a85390`; `atexit(0x00b0a2e0)`.
    StaticInit_Key_RightBracket_00af6d90 @ 0x00af_6d90 => extern "C" fn s_00af6d90() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[5])
    }
    /// `StaticInit_Key_Right_Alt_00af6dd0`: type-enum value `"Key_Right_Alt"`: when `0x00a2e780` finds no such name, the node `0x00c6bcb4` goes on the head of its list `0x01a85390`; `atexit(0x00b0a300)`.
    StaticInit_Key_Right_Alt_00af6dd0 @ 0x00af_6dd0 => extern "C" fn s_00af6dd0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[6])
    }
    /// `StaticInit_Key_Right_Control_00af6e10`: type-enum value `"Key_Right_Control"`: when `0x00a2e780` finds no such name, the node `0x00c6bcd4` goes on the head of its list `0x01a85390`; `atexit(0x00b0a320)`.
    StaticInit_Key_Right_Control_00af6e10 @ 0x00af_6e10 => extern "C" fn s_00af6e10() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[7])
    }
    /// `StaticInit_Key_Right_Shift_00af6e50`: type-enum value `"Key_Right_Shift"`: when `0x00a2e780` finds no such name, the node `0x00c6bcc4` goes on the head of its list `0x01a85390`; `atexit(0x00b0a340)`.
    StaticInit_Key_Right_Shift_00af6e50 @ 0x00af_6e50 => extern "C" fn s_00af6e50() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[8])
    }
    /// `StaticInit_Key_Right_00af6e90`: type-enum value `"Key_Right"`: when `0x00a2e780` finds no such name, the node `0x00c6be94` goes on the head of its list `0x01a85390`; `atexit(0x00b0a360)`.
    StaticInit_Key_Right_00af6e90 @ 0x00af_6e90 => extern "C" fn s_00af6e90() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[9])
    }
    /// `StaticInit_Key_S_00af6ed0`: type-enum value `"Key_S"`: when `0x00a2e780` finds no such name, the node `0x00c6c174` goes on the head of its list `0x01a85390`; `atexit(0x00b0a380)`.
    StaticInit_Key_S_00af6ed0 @ 0x00af_6ed0 => extern "C" fn s_00af6ed0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[10])
    }
    /// `StaticInit_Key_Semicolon_00af6f10`: type-enum value `"Key_Semicolon"`: when `0x00a2e780` finds no such name, the node `0x00c6bef4` goes on the head of its list `0x01a85390`; `atexit(0x00b0a3a0)`.
    StaticInit_Key_Semicolon_00af6f10 @ 0x00af_6f10 => extern "C" fn s_00af6f10() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[11])
    }
    /// `StaticInit_Key_Slash_00af6f50`: type-enum value `"Key_Slash"`: when `0x00a2e780` finds no such name, the node `0x00c6beb4` goes on the head of its list `0x01a85390`; `atexit(0x00b0a3c0)`.
    StaticInit_Key_Slash_00af6f50 @ 0x00af_6f50 => extern "C" fn s_00af6f50() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[12])
    }
    /// `StaticInit_Key_Space_00af6f90`: type-enum value `"Key_Space"`: when `0x00a2e780` finds no such name, the node `0x00c6bf54` goes on the head of its list `0x01a85390`; `atexit(0x00b0a3e0)`.
    StaticInit_Key_Space_00af6f90 @ 0x00af_6f90 => extern "C" fn s_00af6f90() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[13])
    }
    /// `StaticInit_Key_T_00af6fd0`: type-enum value `"Key_T"`: when `0x00a2e780` finds no such name, the node `0x00c6c164` goes on the head of its list `0x01a85390`; `atexit(0x00b0a400)`.
    StaticInit_Key_T_00af6fd0 @ 0x00af_6fd0 => extern "C" fn s_00af6fd0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[14])
    }
    /// `StaticInit_Key_Tab_00af7010`: type-enum value `"Key_Tab"`: when `0x00a2e780` finds no such name, the node `0x00c6bf64` goes on the head of its list `0x01a85390`; `atexit(0x00b0a420)`.
    StaticInit_Key_Tab_00af7010 @ 0x00af_7010 => extern "C" fn s_00af7010() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[15])
    }
    /// `StaticInit_Key_U_00af7050`: type-enum value `"Key_U"`: when `0x00a2e780` finds no such name, the node `0x00c6c154` goes on the head of its list `0x01a85390`; `atexit(0x00b0a440)`.
    StaticInit_Key_U_00af7050 @ 0x00af_7050 => extern "C" fn s_00af7050() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[16])
    }
    /// `StaticInit_Key_Up_00af7090`: type-enum value `"Key_Up"`: when `0x00a2e780` finds no such name, the node `0x00c6be84` goes on the head of its list `0x01a85390`; `atexit(0x00b0a460)`.
    StaticInit_Key_Up_00af7090 @ 0x00af_7090 => extern "C" fn s_00af7090() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[17])
    }
    /// `StaticInit_Key_V_00af70d0`: type-enum value `"Key_V"`: when `0x00a2e780` finds no such name, the node `0x00c6c144` goes on the head of its list `0x01a85390`; `atexit(0x00b0a480)`.
    StaticInit_Key_V_00af70d0 @ 0x00af_70d0 => extern "C" fn s_00af70d0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[18])
    }
    /// `StaticInit_Key_W_00af7110`: type-enum value `"Key_W"`: when `0x00a2e780` finds no such name, the node `0x00c6c134` goes on the head of its list `0x01a85390`; `atexit(0x00b0a4a0)`.
    StaticInit_Key_W_00af7110 @ 0x00af_7110 => extern "C" fn s_00af7110() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[19])
    }
    /// `StaticInit_Key_X_00af7150`: type-enum value `"Key_X"`: when `0x00a2e780` finds no such name, the node `0x00c6c124` goes on the head of its list `0x01a85390`; `atexit(0x00b0a4c0)`.
    StaticInit_Key_X_00af7150 @ 0x00af_7150 => extern "C" fn s_00af7150() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[20])
    }
    /// `StaticInit_Key_Y_00af7190`: type-enum value `"Key_Y"`: when `0x00a2e780` finds no such name, the node `0x00c6c114` goes on the head of its list `0x01a85390`; `atexit(0x00b0a4e0)`.
    StaticInit_Key_Y_00af7190 @ 0x00af_7190 => extern "C" fn s_00af7190() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[21])
    }
    /// `StaticInit_Key_Z_00af71d0`: type-enum value `"Key_Z"`: when `0x00a2e780` finds no such name, the node `0x00c6c104` goes on the head of its list `0x01a85390`; `atexit(0x00b0a500)`.
    StaticInit_Key_Z_00af71d0 @ 0x00af_71d0 => extern "C" fn s_00af71d0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[22])
    }
    /// `StaticInit_Button_LeftMB_00af7210`: type-enum value `"Button_LeftMB"`: when `0x00a2ebf0` finds no such name, the node `0x00c6c3ec` goes on the head of its list `0x01a853cc`; `atexit(0x00b0a530)`.
    StaticInit_Button_LeftMB_00af7210 @ 0x00af_7210 => extern "C" fn s_00af7210() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[23])
    }
    /// `StaticInit_Button_MiddleMB_00af7250`: type-enum value `"Button_MiddleMB"`: when `0x00a2ebf0` finds no such name, the node `0x00c6c3dc` goes on the head of its list `0x01a853cc`; `atexit(0x00b0a550)`.
    StaticInit_Button_MiddleMB_00af7250 @ 0x00af_7250 => extern "C" fn s_00af7250() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[24])
    }
    /// `StaticInit_Button_RightMB_00af7290`: type-enum value `"Button_RightMB"`: when `0x00a2ebf0` finds no such name, the node `0x00c6c3fc` goes on the head of its list `0x01a853cc`; `atexit(0x00b0a570)`.
    StaticInit_Button_RightMB_00af7290 @ 0x00af_7290 => extern "C" fn s_00af7290() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[25])
    }
    /// `StaticInit_Empty_00af72d0`: type-enum value `"Empty"`: when `0x00a2ebf0` finds no such name, the node `0x00c6c3cc` goes on the head of its list `0x01a853cc`; `atexit(0x00b0a590)`.
    StaticInit_Empty_00af72d0 @ 0x00af_72d0 => extern "C" fn s_00af72d0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[26])
    }
    /// `StaticInit_ZeroWords_00af7310`: clears the three words `0x01a85414`, `0x01a85418`, `0x01a8541c` (void: eax untouched).
    StaticInit_ZeroWords_00af7310 @ 0x00af_7310 => extern "C" fn s_00af7310() {
        si::zero_words(&mut mem(), &si::ZERO_WORDS[0]);
    }
    /// `StaticInit_POccluderGeometryInstance_00af7330`: class descriptor `"POccluderGeometryInstance"` (`cd_desc_set8`) on `0x01a85420`: constructor, buffers, set8; `atexit(0x00b0a5c0)`.
    StaticInit_POccluderGeometryInstance_00af7330 @ 0x00af_7330 => extern "C" fn s_00af7330() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[0])
    }
    /// `StaticInit_Button_0_00af73f0`: type-enum value `"Button_0"`: when `0x00a32560` finds no such name, the node `0x00c6d01c` goes on the head of its list `0x01a85578`; `atexit(0x00b0a640)`.
    StaticInit_Button_0_00af73f0 @ 0x00af_73f0 => extern "C" fn s_00af73f0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[27])
    }
    /// `StaticInit_Button_10_00af7430`: type-enum value `"Button_10"`: when `0x00a32560` finds no such name, the node `0x00c6ce1c` goes on the head of its list `0x01a85578`; `atexit(0x00b0a660)`.
    StaticInit_Button_10_00af7430 @ 0x00af_7430 => extern "C" fn s_00af7430() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[28])
    }
    /// `StaticInit_Button_11_00af7470`: type-enum value `"Button_11"`: when `0x00a32560` finds no such name, the node `0x00c6cdec` goes on the head of its list `0x01a85578`; `atexit(0x00b0a680)`.
    StaticInit_Button_11_00af7470 @ 0x00af_7470 => extern "C" fn s_00af7470() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[29])
    }
    /// `StaticInit_Button_12_00af74b0`: type-enum value `"Button_12"`: when `0x00a32560` finds no such name, the node `0x00c6cdbc` goes on the head of its list `0x01a85578`; `atexit(0x00b0a6a0)`.
    StaticInit_Button_12_00af74b0 @ 0x00af_74b0 => extern "C" fn s_00af74b0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[30])
    }
    /// `StaticInit_Button_13_00af74f0`: type-enum value `"Button_13"`: when `0x00a32560` finds no such name, the node `0x00c6cd9c` goes on the head of its list `0x01a85578`; `atexit(0x00b0a6c0)`.
    StaticInit_Button_13_00af74f0 @ 0x00af_74f0 => extern "C" fn s_00af74f0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[31])
    }
    /// `StaticInit_Button_14_00af7530`: type-enum value `"Button_14"`: when `0x00a32560` finds no such name, the node `0x00c6cd7c` goes on the head of its list `0x01a85578`; `atexit(0x00b0a6e0)`.
    StaticInit_Button_14_00af7530 @ 0x00af_7530 => extern "C" fn s_00af7530() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[32])
    }
    /// `StaticInit_Button_15_00af7570`: type-enum value `"Button_15"`: when `0x00a32560` finds no such name, the node `0x00c6cd5c` goes on the head of its list `0x01a85578`; `atexit(0x00b0a700)`.
    StaticInit_Button_15_00af7570 @ 0x00af_7570 => extern "C" fn s_00af7570() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[33])
    }
    /// `StaticInit_Button_1_00af75b0`: type-enum value `"Button_1"`: when `0x00a32560` finds no such name, the node `0x00c6cfdc` goes on the head of its list `0x01a85578`; `atexit(0x00b0a720)`.
    StaticInit_Button_1_00af75b0 @ 0x00af_75b0 => extern "C" fn s_00af75b0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[34])
    }
    /// `StaticInit_Button_2_00af75f0`: type-enum value `"Button_2"`: when `0x00a32560` finds no such name, the node `0x00c6cf9c` goes on the head of its list `0x01a85578`; `atexit(0x00b0a740)`.
    StaticInit_Button_2_00af75f0 @ 0x00af_75f0 => extern "C" fn s_00af75f0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[35])
    }
    /// `StaticInit_Button_3_00af7630`: type-enum value `"Button_3"`: when `0x00a32560` finds no such name, the node `0x00c6cf5c` goes on the head of its list `0x01a85578`; `atexit(0x00b0a760)`.
    StaticInit_Button_3_00af7630 @ 0x00af_7630 => extern "C" fn s_00af7630() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[36])
    }
    /// `StaticInit_Button_4_00af7670`: type-enum value `"Button_4"`: when `0x00a32560` finds no such name, the node `0x00c6cf2c` goes on the head of its list `0x01a85578`; `atexit(0x00b0a780)`.
    StaticInit_Button_4_00af7670 @ 0x00af_7670 => extern "C" fn s_00af7670() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[37])
    }
    /// `StaticInit_Button_5_00af76b0`: type-enum value `"Button_5"`: when `0x00a32560` finds no such name, the node `0x00c6cefc` goes on the head of its list `0x01a85578`; `atexit(0x00b0a7a0)`.
    StaticInit_Button_5_00af76b0 @ 0x00af_76b0 => extern "C" fn s_00af76b0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[38])
    }
    /// `StaticInit_Button_6_00af76f0`: type-enum value `"Button_6"`: when `0x00a32560` finds no such name, the node `0x00c6cecc` goes on the head of its list `0x01a85578`; `atexit(0x00b0a7c0)`.
    StaticInit_Button_6_00af76f0 @ 0x00af_76f0 => extern "C" fn s_00af76f0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[39])
    }
    /// `StaticInit_Button_7_00af7730`: type-enum value `"Button_7"`: when `0x00a32560` finds no such name, the node `0x00c6ce9c` goes on the head of its list `0x01a85578`; `atexit(0x00b0a7e0)`.
    StaticInit_Button_7_00af7730 @ 0x00af_7730 => extern "C" fn s_00af7730() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[40])
    }
    /// `StaticInit_Button_8_00af7770`: type-enum value `"Button_8"`: when `0x00a32560` finds no such name, the node `0x00c6ce6c` goes on the head of its list `0x01a85578`; `atexit(0x00b0a800)`.
    StaticInit_Button_8_00af7770 @ 0x00af_7770 => extern "C" fn s_00af7770() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[41])
    }
    /// `StaticInit_Button_9_00af77b0`: type-enum value `"Button_9"`: when `0x00a32560` finds no such name, the node `0x00c6ce3c` goes on the head of its list `0x01a85578`; `atexit(0x00b0a820)`.
    StaticInit_Button_9_00af77b0 @ 0x00af_77b0 => extern "C" fn s_00af77b0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[42])
    }
    /// `StaticInit_Button_A_00af77f0`: type-enum value `"Button_A"`: when `0x00a32560` finds no such name, the node `0x00c6cfbc` goes on the head of its list `0x01a85578`; `atexit(0x00b0a840)`.
    StaticInit_Button_A_00af77f0 @ 0x00af_77f0 => extern "C" fn s_00af77f0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[43])
    }
    /// `StaticInit_Button_B_00af7830`: type-enum value `"Button_B"`: when `0x00a32560` finds no such name, the node `0x00c6cf7c` goes on the head of its list `0x01a85578`; `atexit(0x00b0a860)`.
    StaticInit_Button_B_00af7830 @ 0x00af_7830 => extern "C" fn s_00af7830() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[44])
    }
    /// `StaticInit_Button_Back_00af7870`: type-enum value `"Button_Back"`: when `0x00a32560` finds no such name, the node `0x00c6ce4c` goes on the head of its list `0x01a85578`; `atexit(0x00b0a880)`.
    StaticInit_Button_Back_00af7870 @ 0x00af_7870 => extern "C" fn s_00af7870() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[45])
    }
    /// `StaticInit_Button_Circle_00af78b0`: type-enum value `"Button_Circle"`: when `0x00a32560` finds no such name, the node `0x00c6cf8c` goes on the head of its list `0x01a85578`; `atexit(0x00b0a8a0)`.
    StaticInit_Button_Circle_00af78b0 @ 0x00af_78b0 => extern "C" fn s_00af78b0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[46])
    }
    /// `StaticInit_Button_Cross_00af78f0`: type-enum value `"Button_Cross"`: when `0x00a32560` finds no such name, the node `0x00c6cfcc` goes on the head of its list `0x01a85578`; `atexit(0x00b0a8c0)`.
    StaticInit_Button_Cross_00af78f0 @ 0x00af_78f0 => extern "C" fn s_00af78f0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[47])
    }
    /// `StaticInit_Button_Down_00af7930`: type-enum value `"Button_Down"`: when `0x00a32560` finds no such name, the node `0x00c6cd6c` goes on the head of its list `0x01a85578`; `atexit(0x00b0a8e0)`.
    StaticInit_Button_Down_00af7930 @ 0x00af_7930 => extern "C" fn s_00af7930() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[48])
    }
    /// `StaticInit_Button_L1_00af7970`: type-enum value `"Button_L1"`: when `0x00a32560` finds no such name, the node `0x00c6cf1c` goes on the head of its list `0x01a85578`; `atexit(0x00b0a900)`.
    StaticInit_Button_L1_00af7970 @ 0x00af_7970 => extern "C" fn s_00af7970() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[49])
    }
    /// `StaticInit_Button_L2_00af79b0`: type-enum value `"Button_L2"`: when `0x00a32560` finds no such name, the node `0x00c6cebc` goes on the head of its list `0x01a85578`; `atexit(0x00b0a920)`.
    StaticInit_Button_L2_00af79b0 @ 0x00af_79b0 => extern "C" fn s_00af79b0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[50])
    }
    /// `StaticInit_Button_L3_00af79f0`: type-enum value `"Button_L3"`: when `0x00a32560` finds no such name, the node `0x00c6ce0c` goes on the head of its list `0x01a85578`; `atexit(0x00b0a940)`.
    StaticInit_Button_L3_00af79f0 @ 0x00af_79f0 => extern "C" fn s_00af79f0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[51])
    }
    /// `StaticInit_Button_LB_00af7a30`: type-enum value `"Button_LB"`: when `0x00a32560` finds no such name, the node `0x00c6cf0c` goes on the head of its list `0x01a85578`; `atexit(0x00b0a960)`.
    StaticInit_Button_LB_00af7a30 @ 0x00af_7a30 => extern "C" fn s_00af7a30() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[52])
    }
    /// `StaticInit_Button_LS_00af7a70`: type-enum value `"Button_LS"`: when `0x00a32560` finds no such name, the node `0x00c6cdfc` goes on the head of its list `0x01a85578`; `atexit(0x00b0a980)`.
    StaticInit_Button_LS_00af7a70 @ 0x00af_7a70 => extern "C" fn s_00af7a70() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[53])
    }
    /// `StaticInit_Button_LT_00af7ab0`: type-enum value `"Button_LT"`: when `0x00a32560` finds no such name, the node `0x00c6ceac` goes on the head of its list `0x01a85578`; `atexit(0x00b0a9a0)`.
    StaticInit_Button_LT_00af7ab0 @ 0x00af_7ab0 => extern "C" fn s_00af7ab0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[54])
    }
    /// `StaticInit_Button_LeftMB_00af7af0`: type-enum value `"Button_LeftMB"`: when `0x00a32560` finds no such name, the node `0x00c6cfec` goes on the head of its list `0x01a85578`; `atexit(0x00b0a9c0)`.
    StaticInit_Button_LeftMB_00af7af0 @ 0x00af_7af0 => extern "C" fn s_00af7af0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[55])
    }
    /// `StaticInit_Button_Left_00af7b30`: type-enum value `"Button_Left"`: when `0x00a32560` finds no such name, the node `0x00c6cd4c` goes on the head of its list `0x01a85578`; `atexit(0x00b0a9e0)`.
    StaticInit_Button_Left_00af7b30 @ 0x00af_7b30 => extern "C" fn s_00af7b30() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[56])
    }
    /// `StaticInit_Button_MiddleMB_00af7b70`: type-enum value `"Button_MiddleMB"`: when `0x00a32560` finds no such name, the node `0x00c6cfac` goes on the head of its list `0x01a85578`; `atexit(0x00b0aa00)`.
    StaticInit_Button_MiddleMB_00af7b70 @ 0x00af_7b70 => extern "C" fn s_00af7b70() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[57])
    }
    /// `StaticInit_Button_R1_00af7bb0`: type-enum value `"Button_R1"`: when `0x00a32560` finds no such name, the node `0x00c6ceec` goes on the head of its list `0x01a85578`; `atexit(0x00b0aa20)`.
    StaticInit_Button_R1_00af7bb0 @ 0x00af_7bb0 => extern "C" fn s_00af7bb0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[58])
    }
    /// `StaticInit_Button_R2_00af7bf0`: type-enum value `"Button_R2"`: when `0x00a32560` finds no such name, the node `0x00c6ce8c` goes on the head of its list `0x01a85578`; `atexit(0x00b0aa40)`.
    StaticInit_Button_R2_00af7bf0 @ 0x00af_7bf0 => extern "C" fn s_00af7bf0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[59])
    }
    /// `StaticInit_Button_R3_00af7c30`: type-enum value `"Button_R3"`: when `0x00a32560` finds no such name, the node `0x00c6cddc` goes on the head of its list `0x01a85578`; `atexit(0x00b0aa60)`.
    StaticInit_Button_R3_00af7c30 @ 0x00af_7c30 => extern "C" fn s_00af7c30() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[60])
    }
    /// `StaticInit_Button_RB_00af7c70`: type-enum value `"Button_RB"`: when `0x00a32560` finds no such name, the node `0x00c6cedc` goes on the head of its list `0x01a85578`; `atexit(0x00b0aa80)`.
    StaticInit_Button_RB_00af7c70 @ 0x00af_7c70 => extern "C" fn s_00af7c70() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[61])
    }
    /// `StaticInit_Button_RS_00af7cb0`: type-enum value `"Button_RS"`: when `0x00a32560` finds no such name, the node `0x00c6cdcc` goes on the head of its list `0x01a85578`; `atexit(0x00b0aaa0)`.
    StaticInit_Button_RS_00af7cb0 @ 0x00af_7cb0 => extern "C" fn s_00af7cb0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[62])
    }
    /// `StaticInit_Button_RT_00af7cf0`: type-enum value `"Button_RT"`: when `0x00a32560` finds no such name, the node `0x00c6ce7c` goes on the head of its list `0x01a85578`; `atexit(0x00b0aac0)`.
    StaticInit_Button_RT_00af7cf0 @ 0x00af_7cf0 => extern "C" fn s_00af7cf0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[63])
    }
    /// `StaticInit_Button_RightMB_00af7d30`: type-enum value `"Button_RightMB"`: when `0x00a32560` finds no such name, the node `0x00c6cf6c` goes on the head of its list `0x01a85578`; `atexit(0x00b0aae0)`.
    StaticInit_Button_RightMB_00af7d30 @ 0x00af_7d30 => extern "C" fn s_00af7d30() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[64])
    }
    /// `StaticInit_Button_Right_00af7d70`: type-enum value `"Button_Right"`: when `0x00a32560` finds no such name, the node `0x00c6cd8c` goes on the head of its list `0x01a85578`; `atexit(0x00b0ab00)`.
    StaticInit_Button_Right_00af7d70 @ 0x00af_7d70 => extern "C" fn s_00af7d70() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[65])
    }
    /// `StaticInit_Button_Select_00af7db0`: type-enum value `"Button_Select"`: when `0x00a32560` finds no such name, the node `0x00c6ce5c` goes on the head of its list `0x01a85578`; `atexit(0x00b0ab20)`.
    StaticInit_Button_Select_00af7db0 @ 0x00af_7db0 => extern "C" fn s_00af7db0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[66])
    }
    /// `StaticInit_Button_Square_00af7df0`: type-enum value `"Button_Square"`: when `0x00a32560` finds no such name, the node `0x00c6d00c` goes on the head of its list `0x01a85578`; `atexit(0x00b0ab40)`.
    StaticInit_Button_Square_00af7df0 @ 0x00af_7df0 => extern "C" fn s_00af7df0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[67])
    }
    /// `StaticInit_Button_Start_00af7e30`: type-enum value `"Button_Start"`: when `0x00a32560` finds no such name, the node `0x00c6ce2c` goes on the head of its list `0x01a85578`; `atexit(0x00b0ab60)`.
    StaticInit_Button_Start_00af7e30 @ 0x00af_7e30 => extern "C" fn s_00af7e30() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[68])
    }
    /// `StaticInit_Button_Triangle_00af7e70`: type-enum value `"Button_Triangle"`: when `0x00a32560` finds no such name, the node `0x00c6cf4c` goes on the head of its list `0x01a85578`; `atexit(0x00b0ab80)`.
    StaticInit_Button_Triangle_00af7e70 @ 0x00af_7e70 => extern "C" fn s_00af7e70() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[69])
    }
    /// `StaticInit_Button_Up_00af7eb0`: type-enum value `"Button_Up"`: when `0x00a32560` finds no such name, the node `0x00c6cdac` goes on the head of its list `0x01a85578`; `atexit(0x00b0aba0)`.
    StaticInit_Button_Up_00af7eb0 @ 0x00af_7eb0 => extern "C" fn s_00af7eb0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[70])
    }
    /// `StaticInit_Button_X_00af7ef0`: type-enum value `"Button_X"`: when `0x00a32560` finds no such name, the node `0x00c6cffc` goes on the head of its list `0x01a85578`; `atexit(0x00b0abc0)`.
    StaticInit_Button_X_00af7ef0 @ 0x00af_7ef0 => extern "C" fn s_00af7ef0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[71])
    }
    /// `StaticInit_Button_Y_00af7f30`: type-enum value `"Button_Y"`: when `0x00a32560` finds no such name, the node `0x00c6cf3c` goes on the head of its list `0x01a85578`; `atexit(0x00b0abe0)`.
    StaticInit_Button_Y_00af7f30 @ 0x00af_7f30 => extern "C" fn s_00af7f30() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[72])
    }
    /// `StaticInit_Empty_00af7f70`: type-enum value `"Empty"`: when `0x00a32560` finds no such name, the node `0x00c6d02c` goes on the head of its list `0x01a85578`; `atexit(0x00b0ac00)`.
    StaticInit_Empty_00af7f70 @ 0x00af_7f70 => extern "C" fn s_00af7f70() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[73])
    }
    /// `StaticInit_Key_0_00af7fb0`: type-enum value `"Key_0"`: when `0x00a32560` finds no such name, the node `0x00c6cb0c` goes on the head of its list `0x01a85578`; `atexit(0x00b0ac20)`.
    StaticInit_Key_0_00af7fb0 @ 0x00af_7fb0 => extern "C" fn s_00af7fb0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[74])
    }
    /// `StaticInit_Key_1_00af7ff0`: type-enum value `"Key_1"`: when `0x00a32560` finds no such name, the node `0x00c6cb9c` goes on the head of its list `0x01a85578`; `atexit(0x00b0ac40)`.
    StaticInit_Key_1_00af7ff0 @ 0x00af_7ff0 => extern "C" fn s_00af7ff0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[75])
    }
    /// `StaticInit_Key_2_00af8030`: type-enum value `"Key_2"`: when `0x00a32560` finds no such name, the node `0x00c6cb8c` goes on the head of its list `0x01a85578`; `atexit(0x00b0ac60)`.
    StaticInit_Key_2_00af8030 @ 0x00af_8030 => extern "C" fn s_00af8030() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[76])
    }
    /// `StaticInit_Key_3_00af8070`: type-enum value `"Key_3"`: when `0x00a32560` finds no such name, the node `0x00c6cb7c` goes on the head of its list `0x01a85578`; `atexit(0x00b0ac80)`.
    StaticInit_Key_3_00af8070 @ 0x00af_8070 => extern "C" fn s_00af8070() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[77])
    }
    /// `StaticInit_Key_4_00af80b0`: type-enum value `"Key_4"`: when `0x00a32560` finds no such name, the node `0x00c6cb6c` goes on the head of its list `0x01a85578`; `atexit(0x00b0aca0)`.
    StaticInit_Key_4_00af80b0 @ 0x00af_80b0 => extern "C" fn s_00af80b0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[78])
    }
    /// `StaticInit_Key_5_00af80f0`: type-enum value `"Key_5"`: when `0x00a32560` finds no such name, the node `0x00c6cb5c` goes on the head of its list `0x01a85578`; `atexit(0x00b0acc0)`.
    StaticInit_Key_5_00af80f0 @ 0x00af_80f0 => extern "C" fn s_00af80f0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[79])
    }
    /// `StaticInit_Key_6_00af8130`: type-enum value `"Key_6"`: when `0x00a32560` finds no such name, the node `0x00c6cb4c` goes on the head of its list `0x01a85578`; `atexit(0x00b0ace0)`.
    StaticInit_Key_6_00af8130 @ 0x00af_8130 => extern "C" fn s_00af8130() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[80])
    }
    /// `StaticInit_Key_7_00af8170`: type-enum value `"Key_7"`: when `0x00a32560` finds no such name, the node `0x00c6cb3c` goes on the head of its list `0x01a85578`; `atexit(0x00b0ad00)`.
    StaticInit_Key_7_00af8170 @ 0x00af_8170 => extern "C" fn s_00af8170() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[81])
    }
    /// `StaticInit_Key_8_00af81b0`: type-enum value `"Key_8"`: when `0x00a32560` finds no such name, the node `0x00c6cb2c` goes on the head of its list `0x01a85578`; `atexit(0x00b0ad20)`.
    StaticInit_Key_8_00af81b0 @ 0x00af_81b0 => extern "C" fn s_00af81b0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[82])
    }
    /// `StaticInit_Key_9_00af81f0`: type-enum value `"Key_9"`: when `0x00a32560` finds no such name, the node `0x00c6cb1c` goes on the head of its list `0x01a85578`; `atexit(0x00b0ad40)`.
    StaticInit_Key_9_00af81f0 @ 0x00af_81f0 => extern "C" fn s_00af81f0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[83])
    }
    /// `StaticInit_Key_A_00af8230`: type-enum value `"Key_A"`: when `0x00a32560` finds no such name, the node `0x00c6cd3c` goes on the head of its list `0x01a85578`; `atexit(0x00b0ad60)`.
    StaticInit_Key_A_00af8230 @ 0x00af_8230 => extern "C" fn s_00af8230() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[84])
    }
    /// `StaticInit_Key_AccentGrave_00af8270`: type-enum value `"Key_AccentGrave"`: when `0x00a32560` finds no such name, the node `0x00c6c74c` goes on the head of its list `0x01a85578`; `atexit(0x00b0ad80)`.
    StaticInit_Key_AccentGrave_00af8270 @ 0x00af_8270 => extern "C" fn s_00af8270() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[85])
    }
    /// `StaticInit_Key_Apostrophe_00af82b0`: type-enum value `"Key_Apostrophe"`: when `0x00a32560` finds no such name, the node `0x00c6c98c` goes on the head of its list `0x01a85578`; `atexit(0x00b0ada0)`.
    StaticInit_Key_Apostrophe_00af82b0 @ 0x00af_82b0 => extern "C" fn s_00af82b0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[86])
    }
    /// `StaticInit_Key_B_00af82f0`: type-enum value `"Key_B"`: when `0x00a32560` finds no such name, the node `0x00c6cd2c` goes on the head of its list `0x01a85578`; `atexit(0x00b0adc0)`.
    StaticInit_Key_B_00af82f0 @ 0x00af_82f0 => extern "C" fn s_00af82f0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[87])
    }
    /// `StaticInit_Key_Backslash_00af8330`: type-enum value `"Key_Backslash"`: when `0x00a32560` finds no such name, the node `0x00c6c9ac` goes on the head of its list `0x01a85578`; `atexit(0x00b0ade0)`.
    StaticInit_Key_Backslash_00af8330 @ 0x00af_8330 => extern "C" fn s_00af8330() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[88])
    }
    /// `StaticInit_Key_Backspace_00af8370`: type-enum value `"Key_Backspace"`: when `0x00a32560` finds no such name, the node `0x00c6ca1c` goes on the head of its list `0x01a85578`; `atexit(0x00b0ae00)`.
    StaticInit_Key_Backspace_00af8370 @ 0x00af_8370 => extern "C" fn s_00af8370() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[89])
    }
    /// `StaticInit_Key_C_00af83b0`: type-enum value `"Key_C"`: when `0x00a32560` finds no such name, the node `0x00c6cd1c` goes on the head of its list `0x01a85578`; `atexit(0x00b0ae20)`.
    StaticInit_Key_C_00af83b0 @ 0x00af_83b0 => extern "C" fn s_00af83b0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[90])
    }
    /// `StaticInit_Key_Comma_00af83f0`: type-enum value `"Key_Comma"`: when `0x00a32560` finds no such name, the node `0x00c6c97c` goes on the head of its list `0x01a85578`; `atexit(0x00b0ae40)`.
    StaticInit_Key_Comma_00af83f0 @ 0x00af_83f0 => extern "C" fn s_00af83f0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[91])
    }
    /// `StaticInit_Key_D_00af8430`: type-enum value `"Key_D"`: when `0x00a32560` finds no such name, the node `0x00c6cd0c` goes on the head of its list `0x01a85578`; `atexit(0x00b0ae60)`.
    StaticInit_Key_D_00af8430 @ 0x00af_8430 => extern "C" fn s_00af8430() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[92])
    }
    /// `StaticInit_Key_Delete_00af8470`: type-enum value `"Key_Delete"`: when `0x00a32560` finds no such name, the node `0x00c6c8bc` goes on the head of its list `0x01a85578`; `atexit(0x00b0ae80)`.
    StaticInit_Key_Delete_00af8470 @ 0x00af_8470 => extern "C" fn s_00af8470() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[93])
    }
    /// `StaticInit_Key_Down_00af84b0`: type-enum value `"Key_Down"`: when `0x00a32560` finds no such name, the node `0x00c6c91c` goes on the head of its list `0x01a85578`; `atexit(0x00b0aea0)`.
    StaticInit_Key_Down_00af84b0 @ 0x00af_84b0 => extern "C" fn s_00af84b0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[94])
    }
    /// `StaticInit_Key_E_00af84f0`: type-enum value `"Key_E"`: when `0x00a32560` finds no such name, the node `0x00c6ccfc` goes on the head of its list `0x01a85578`; `atexit(0x00b0aec0)`.
    StaticInit_Key_E_00af84f0 @ 0x00af_84f0 => extern "C" fn s_00af84f0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[95])
    }
    /// `StaticInit_Key_End_00af8530`: type-enum value `"Key_End"`: when `0x00a32560` finds no such name, the node `0x00c6c8fc` goes on the head of its list `0x01a85578`; `atexit(0x00b0aee0)`.
    StaticInit_Key_End_00af8530 @ 0x00af_8530 => extern "C" fn s_00af8530() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[96])
    }
    /// `StaticInit_Key_Enter_00af8570`: type-enum value `"Key_Enter"`: when `0x00a32560` finds no such name, the node `0x00c6ca2c` goes on the head of its list `0x01a85578`; `atexit(0x00b0af00)`.
    StaticInit_Key_Enter_00af8570 @ 0x00af_8570 => extern "C" fn s_00af8570() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[97])
    }
    /// `StaticInit_Key_Equals_00af85b0`: type-enum value `"Key_Equals"`: when `0x00a32560` finds no such name, the node `0x00c6c9dc` goes on the head of its list `0x01a85578`; `atexit(0x00b0af20)`.
    StaticInit_Key_Equals_00af85b0 @ 0x00af_85b0 => extern "C" fn s_00af85b0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[98])
    }
    /// `StaticInit_Key_Escape_00af85f0`: type-enum value `"Key_Escape"`: when `0x00a32560` finds no such name, the node `0x00c6cafc` goes on the head of its list `0x01a85578`; `atexit(0x00b0af40)`.
    StaticInit_Key_Escape_00af85f0 @ 0x00af_85f0 => extern "C" fn s_00af85f0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[99])
    }
    /// `StaticInit_Key_F10_00af8630`: type-enum value `"Key_F10"`: when `0x00a32560` finds no such name, the node `0x00c6ca5c` goes on the head of its list `0x01a85578`; `atexit(0x00b0af60)`.
    StaticInit_Key_F10_00af8630 @ 0x00af_8630 => extern "C" fn s_00af8630() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[100])
    }
    /// `StaticInit_Key_F11_00af8670`: type-enum value `"Key_F11"`: when `0x00a32560` finds no such name, the node `0x00c6ca4c` goes on the head of its list `0x01a85578`; `atexit(0x00b0af80)`.
    StaticInit_Key_F11_00af8670 @ 0x00af_8670 => extern "C" fn s_00af8670() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[101])
    }
    /// `StaticInit_Key_F12_00af86b0`: type-enum value `"Key_F12"`: when `0x00a32560` finds no such name, the node `0x00c6ca3c` goes on the head of its list `0x01a85578`; `atexit(0x00b0afa0)`.
    StaticInit_Key_F12_00af86b0 @ 0x00af_86b0 => extern "C" fn s_00af86b0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[102])
    }
    /// `StaticInit_Key_F1_00af86f0`: type-enum value `"Key_F1"`: when `0x00a32560` finds no such name, the node `0x00c6caec` goes on the head of its list `0x01a85578`; `atexit(0x00b0afc0)`.
    StaticInit_Key_F1_00af86f0 @ 0x00af_86f0 => extern "C" fn s_00af86f0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[103])
    }
    /// `StaticInit_Key_F2_00af8730`: type-enum value `"Key_F2"`: when `0x00a32560` finds no such name, the node `0x00c6cadc` goes on the head of its list `0x01a85578`; `atexit(0x00b0afe0)`.
    StaticInit_Key_F2_00af8730 @ 0x00af_8730 => extern "C" fn s_00af8730() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[104])
    }
    /// `StaticInit_Key_F3_00af8770`: type-enum value `"Key_F3"`: when `0x00a32560` finds no such name, the node `0x00c6cacc` goes on the head of its list `0x01a85578`; `atexit(0x00b0b000)`.
    StaticInit_Key_F3_00af8770 @ 0x00af_8770 => extern "C" fn s_00af8770() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[105])
    }
    /// `StaticInit_Key_F4_00af87b0`: type-enum value `"Key_F4"`: when `0x00a32560` finds no such name, the node `0x00c6cabc` goes on the head of its list `0x01a85578`; `atexit(0x00b0b020)`.
    StaticInit_Key_F4_00af87b0 @ 0x00af_87b0 => extern "C" fn s_00af87b0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[106])
    }
    /// `StaticInit_Key_F5_00af87f0`: type-enum value `"Key_F5"`: when `0x00a32560` finds no such name, the node `0x00c6caac` goes on the head of its list `0x01a85578`; `atexit(0x00b0b040)`.
    StaticInit_Key_F5_00af87f0 @ 0x00af_87f0 => extern "C" fn s_00af87f0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[107])
    }
    /// `StaticInit_Key_F6_00af8830`: type-enum value `"Key_F6"`: when `0x00a32560` finds no such name, the node `0x00c6ca9c` goes on the head of its list `0x01a85578`; `atexit(0x00b0b060)`.
    StaticInit_Key_F6_00af8830 @ 0x00af_8830 => extern "C" fn s_00af8830() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[108])
    }
    /// `StaticInit_Key_F7_00af8870`: type-enum value `"Key_F7"`: when `0x00a32560` finds no such name, the node `0x00c6ca8c` goes on the head of its list `0x01a85578`; `atexit(0x00b0b080)`.
    StaticInit_Key_F7_00af8870 @ 0x00af_8870 => extern "C" fn s_00af8870() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[109])
    }
    /// `StaticInit_Key_F8_00af88b0`: type-enum value `"Key_F8"`: when `0x00a32560` finds no such name, the node `0x00c6ca7c` goes on the head of its list `0x01a85578`; `atexit(0x00b0b0a0)`.
    StaticInit_Key_F8_00af88b0 @ 0x00af_88b0 => extern "C" fn s_00af88b0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[110])
    }
    /// `StaticInit_Key_F9_00af88f0`: type-enum value `"Key_F9"`: when `0x00a32560` finds no such name, the node `0x00c6ca6c` goes on the head of its list `0x01a85578`; `atexit(0x00b0b0c0)`.
    StaticInit_Key_F9_00af88f0 @ 0x00af_88f0 => extern "C" fn s_00af88f0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[111])
    }
    /// `StaticInit_Key_F_00af8930`: type-enum value `"Key_F"`: when `0x00a32560` finds no such name, the node `0x00c6ccec` goes on the head of its list `0x01a85578`; `atexit(0x00b0b0e0)`.
    StaticInit_Key_F_00af8930 @ 0x00af_8930 => extern "C" fn s_00af8930() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[112])
    }
    /// `StaticInit_Key_G_00af8970`: type-enum value `"Key_G"`: when `0x00a32560` finds no such name, the node `0x00c6ccdc` goes on the head of its list `0x01a85578`; `atexit(0x00b0b100)`.
    StaticInit_Key_G_00af8970 @ 0x00af_8970 => extern "C" fn s_00af8970() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[113])
    }
    /// `StaticInit_Key_H_00af89b0`: type-enum value `"Key_H"`: when `0x00a32560` finds no such name, the node `0x00c6cccc` goes on the head of its list `0x01a85578`; `atexit(0x00b0b120)`.
    StaticInit_Key_H_00af89b0 @ 0x00af_89b0 => extern "C" fn s_00af89b0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[114])
    }
    /// `StaticInit_Key_Home_00af89f0`: type-enum value `"Key_Home"`: when `0x00a32560` finds no such name, the node `0x00c6c90c` goes on the head of its list `0x01a85578`; `atexit(0x00b0b140)`.
    StaticInit_Key_Home_00af89f0 @ 0x00af_89f0 => extern "C" fn s_00af89f0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[115])
    }
    /// `StaticInit_Key_I_00af8a30`: type-enum value `"Key_I"`: when `0x00a32560` finds no such name, the node `0x00c6ccbc` goes on the head of its list `0x01a85578`; `atexit(0x00b0b160)`.
    StaticInit_Key_I_00af8a30 @ 0x00af_8a30 => extern "C" fn s_00af8a30() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[116])
    }
    /// `StaticInit_Key_Insert_00af8a70`: type-enum value `"Key_Insert"`: when `0x00a32560` finds no such name, the node `0x00c6c8cc` goes on the head of its list `0x01a85578`; `atexit(0x00b0b180)`.
    StaticInit_Key_Insert_00af8a70 @ 0x00af_8a70 => extern "C" fn s_00af8a70() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[117])
    }
    /// `StaticInit_Key_J_00af8ab0`: type-enum value `"Key_J"`: when `0x00a32560` finds no such name, the node `0x00c6ccac` goes on the head of its list `0x01a85578`; `atexit(0x00b0b1a0)`.
    StaticInit_Key_J_00af8ab0 @ 0x00af_8ab0 => extern "C" fn s_00af8ab0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[118])
    }
    /// `StaticInit_Key_K_00af8af0`: type-enum value `"Key_K"`: when `0x00a32560` finds no such name, the node `0x00c6cc9c` goes on the head of its list `0x01a85578`; `atexit(0x00b0b1c0)`.
    StaticInit_Key_K_00af8af0 @ 0x00af_8af0 => extern "C" fn s_00af8af0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[119])
    }
    /// `StaticInit_Key_L_00af8b30`: type-enum value `"Key_L"`: when `0x00a32560` finds no such name, the node `0x00c6cc8c` goes on the head of its list `0x01a85578`; `atexit(0x00b0b1e0)`.
    StaticInit_Key_L_00af8b30 @ 0x00af_8b30 => extern "C" fn s_00af8b30() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[120])
    }
    /// `StaticInit_Key_LeftBracket_00af8b70`: type-enum value `"Key_LeftBracket"`: when `0x00a32560` finds no such name, the node `0x00c6c9cc` goes on the head of its list `0x01a85578`; `atexit(0x00b0b200)`.
    StaticInit_Key_LeftBracket_00af8b70 @ 0x00af_8b70 => extern "C" fn s_00af8b70() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[121])
    }
    /// `StaticInit_Key_Left_Alt_00af8bb0`: type-enum value `"Key_Left_Alt"`: when `0x00a32560` finds no such name, the node `0x00c6c78c` goes on the head of its list `0x01a85578`; `atexit(0x00b0b220)`.
    StaticInit_Key_Left_Alt_00af8bb0 @ 0x00af_8bb0 => extern "C" fn s_00af8bb0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[122])
    }
    /// `StaticInit_Key_Left_Control_00af8bf0`: type-enum value `"Key_Left_Control"`: when `0x00a32560` finds no such name, the node `0x00c6c7ac` goes on the head of its list `0x01a85578`; `atexit(0x00b0b240)`.
    StaticInit_Key_Left_Control_00af8bf0 @ 0x00af_8bf0 => extern "C" fn s_00af8bf0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[123])
    }
    /// `StaticInit_Key_Left_Shift_00af8c30`: type-enum value `"Key_Left_Shift"`: when `0x00a32560` finds no such name, the node `0x00c6c79c` goes on the head of its list `0x01a85578`; `atexit(0x00b0b260)`.
    StaticInit_Key_Left_Shift_00af8c30 @ 0x00af_8c30 => extern "C" fn s_00af8c30() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[124])
    }
    /// `StaticInit_Key_Left_00af8c70`: type-enum value `"Key_Left"`: when `0x00a32560` finds no such name, the node `0x00c6c94c` goes on the head of its list `0x01a85578`; `atexit(0x00b0b280)`.
    StaticInit_Key_Left_00af8c70 @ 0x00af_8c70 => extern "C" fn s_00af8c70() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[125])
    }
    /// `StaticInit_Key_M_00af8cb0`: type-enum value `"Key_M"`: when `0x00a32560` finds no such name, the node `0x00c6cc7c` goes on the head of its list `0x01a85578`; `atexit(0x00b0b2a0)`.
    StaticInit_Key_M_00af8cb0 @ 0x00af_8cb0 => extern "C" fn s_00af8cb0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[126])
    }
    /// `StaticInit_Key_Minus_00af8cf0`: type-enum value `"Key_Minus"`: when `0x00a32560` finds no such name, the node `0x00c6c9ec` goes on the head of its list `0x01a85578`; `atexit(0x00b0b2c0)`.
    StaticInit_Key_Minus_00af8cf0 @ 0x00af_8cf0 => extern "C" fn s_00af8cf0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[127])
    }
    /// `StaticInit_Key_N_00af8d30`: type-enum value `"Key_N"`: when `0x00a32560` finds no such name, the node `0x00c6cc6c` goes on the head of its list `0x01a85578`; `atexit(0x00b0b2e0)`.
    StaticInit_Key_N_00af8d30 @ 0x00af_8d30 => extern "C" fn s_00af8d30() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[128])
    }
    /// `StaticInit_Key_Numpad_0_00af8d70`: type-enum value `"Key_Numpad_0"`: when `0x00a32560` finds no such name, the node `0x00c6c81c` goes on the head of its list `0x01a85578`; `atexit(0x00b0b300)`.
    StaticInit_Key_Numpad_0_00af8d70 @ 0x00af_8d70 => extern "C" fn s_00af8d70() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[129])
    }
    /// `StaticInit_Key_Numpad_1_00af8db0`: type-enum value `"Key_Numpad_1"`: when `0x00a32560` finds no such name, the node `0x00c6c8ac` goes on the head of its list `0x01a85578`; `atexit(0x00b0b320)`.
    StaticInit_Key_Numpad_1_00af8db0 @ 0x00af_8db0 => extern "C" fn s_00af8db0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[130])
    }
    /// `StaticInit_Key_Numpad_2_00af8df0`: type-enum value `"Key_Numpad_2"`: when `0x00a32560` finds no such name, the node `0x00c6c89c` goes on the head of its list `0x01a85578`; `atexit(0x00b0b340)`.
    StaticInit_Key_Numpad_2_00af8df0 @ 0x00af_8df0 => extern "C" fn s_00af8df0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[131])
    }
    /// `StaticInit_Key_Numpad_3_00af8e30`: type-enum value `"Key_Numpad_3"`: when `0x00a32560` finds no such name, the node `0x00c6c88c` goes on the head of its list `0x01a85578`; `atexit(0x00b0b360)`.
    StaticInit_Key_Numpad_3_00af8e30 @ 0x00af_8e30 => extern "C" fn s_00af8e30() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[132])
    }
    /// `StaticInit_Key_Numpad_4_00af8e70`: type-enum value `"Key_Numpad_4"`: when `0x00a32560` finds no such name, the node `0x00c6c87c` goes on the head of its list `0x01a85578`; `atexit(0x00b0b380)`.
    StaticInit_Key_Numpad_4_00af8e70 @ 0x00af_8e70 => extern "C" fn s_00af8e70() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[133])
    }
    /// `StaticInit_Key_Numpad_5_00af8eb0`: type-enum value `"Key_Numpad_5"`: when `0x00a32560` finds no such name, the node `0x00c6c86c` goes on the head of its list `0x01a85578`; `atexit(0x00b0b3a0)`.
    StaticInit_Key_Numpad_5_00af8eb0 @ 0x00af_8eb0 => extern "C" fn s_00af8eb0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[134])
    }
    /// `StaticInit_Key_Numpad_6_00af8ef0`: type-enum value `"Key_Numpad_6"`: when `0x00a32560` finds no such name, the node `0x00c6c85c` goes on the head of its list `0x01a85578`; `atexit(0x00b0b3c0)`.
    StaticInit_Key_Numpad_6_00af8ef0 @ 0x00af_8ef0 => extern "C" fn s_00af8ef0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[135])
    }
    /// `StaticInit_Key_Numpad_7_00af8f30`: type-enum value `"Key_Numpad_7"`: when `0x00a32560` finds no such name, the node `0x00c6c84c` goes on the head of its list `0x01a85578`; `atexit(0x00b0b3e0)`.
    StaticInit_Key_Numpad_7_00af8f30 @ 0x00af_8f30 => extern "C" fn s_00af8f30() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[136])
    }
    /// `StaticInit_Key_Numpad_8_00af8f70`: type-enum value `"Key_Numpad_8"`: when `0x00a32560` finds no such name, the node `0x00c6c83c` goes on the head of its list `0x01a85578`; `atexit(0x00b0b400)`.
    StaticInit_Key_Numpad_8_00af8f70 @ 0x00af_8f70 => extern "C" fn s_00af8f70() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[137])
    }
    /// `StaticInit_Key_Numpad_9_00af8fb0`: type-enum value `"Key_Numpad_9"`: when `0x00a32560` finds no such name, the node `0x00c6c82c` goes on the head of its list `0x01a85578`; `atexit(0x00b0b420)`.
    StaticInit_Key_Numpad_9_00af8fb0 @ 0x00af_8fb0 => extern "C" fn s_00af8fb0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[138])
    }
    /// `StaticInit_Key_Numpad_Asterisk_00af8ff0`: type-enum value `"Key_Numpad_Asterisk"`: when `0x00a32560` finds no such name, the node `0x00c6c7fc` goes on the head of its list `0x01a85578`; `atexit(0x00b0b440)`.
    StaticInit_Key_Numpad_Asterisk_00af8ff0 @ 0x00af_8ff0 => extern "C" fn s_00af8ff0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[139])
    }
    /// `StaticInit_Key_Numpad_Enter_00af9030`: type-enum value `"Key_Numpad_Enter"`: when `0x00a32560` finds no such name, the node `0x00c6c7cc` goes on the head of its list `0x01a85578`; `atexit(0x00b0b460)`.
    StaticInit_Key_Numpad_Enter_00af9030 @ 0x00af_9030 => extern "C" fn s_00af9030() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[140])
    }
    /// `StaticInit_Key_Numpad_Minus_00af9070`: type-enum value `"Key_Numpad_Minus"`: when `0x00a32560` finds no such name, the node `0x00c6c7ec` goes on the head of its list `0x01a85578`; `atexit(0x00b0b480)`.
    StaticInit_Key_Numpad_Minus_00af9070 @ 0x00af_9070 => extern "C" fn s_00af9070() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[141])
    }
    /// `StaticInit_Key_Numpad_Period_00af90b0`: type-enum value `"Key_Numpad_Period"`: when `0x00a32560` finds no such name, the node `0x00c6c7bc` goes on the head of its list `0x01a85578`; `atexit(0x00b0b4a0)`.
    StaticInit_Key_Numpad_Period_00af90b0 @ 0x00af_90b0 => extern "C" fn s_00af90b0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[142])
    }
    /// `StaticInit_Key_Numpad_Plus_00af90f0`: type-enum value `"Key_Numpad_Plus"`: when `0x00a32560` finds no such name, the node `0x00c6c7dc` goes on the head of its list `0x01a85578`; `atexit(0x00b0b4c0)`.
    StaticInit_Key_Numpad_Plus_00af90f0 @ 0x00af_90f0 => extern "C" fn s_00af90f0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[143])
    }
    /// `StaticInit_Key_Numpad_Slash_00af9130`: type-enum value `"Key_Numpad_Slash"`: when `0x00a32560` finds no such name, the node `0x00c6c80c` goes on the head of its list `0x01a85578`; `atexit(0x00b0b4e0)`.
    StaticInit_Key_Numpad_Slash_00af9130 @ 0x00af_9130 => extern "C" fn s_00af9130() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[144])
    }
    /// `StaticInit_Key_O_00af9170`: type-enum value `"Key_O"`: when `0x00a32560` finds no such name, the node `0x00c6cc5c` goes on the head of its list `0x01a85578`; `atexit(0x00b0b500)`.
    StaticInit_Key_O_00af9170 @ 0x00af_9170 => extern "C" fn s_00af9170() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[145])
    }
    /// `StaticInit_Key_P_00af91b0`: type-enum value `"Key_P"`: when `0x00a32560` finds no such name, the node `0x00c6cc4c` goes on the head of its list `0x01a85578`; `atexit(0x00b0b520)`.
    StaticInit_Key_P_00af91b0 @ 0x00af_91b0 => extern "C" fn s_00af91b0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[146])
    }
    /// `StaticInit_Key_PageDown_00af91f0`: type-enum value `"Key_PageDown"`: when `0x00a32560` finds no such name, the node `0x00c6c8dc` goes on the head of its list `0x01a85578`; `atexit(0x00b0b540)`.
    StaticInit_Key_PageDown_00af91f0 @ 0x00af_91f0 => extern "C" fn s_00af91f0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[147])
    }
    /// `StaticInit_Key_PageUp_00af9230`: type-enum value `"Key_PageUp"`: when `0x00a32560` finds no such name, the node `0x00c6c8ec` goes on the head of its list `0x01a85578`; `atexit(0x00b0b560)`.
    StaticInit_Key_PageUp_00af9230 @ 0x00af_9230 => extern "C" fn s_00af9230() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[148])
    }
    /// `StaticInit_Key_Period_00af9270`: type-enum value `"Key_Period"`: when `0x00a32560` finds no such name, the node `0x00c6c96c` goes on the head of its list `0x01a85578`; `atexit(0x00b0b580)`.
    StaticInit_Key_Period_00af9270 @ 0x00af_9270 => extern "C" fn s_00af9270() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[149])
    }
    /// `StaticInit_Key_Q_00af92b0`: type-enum value `"Key_Q"`: when `0x00a32560` finds no such name, the node `0x00c6cc3c` goes on the head of its list `0x01a85578`; `atexit(0x00b0b5a0)`.
    StaticInit_Key_Q_00af92b0 @ 0x00af_92b0 => extern "C" fn s_00af92b0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[150])
    }
    /// `StaticInit_Key_R_00af92f0`: type-enum value `"Key_R"`: when `0x00a32560` finds no such name, the node `0x00c6cc2c` goes on the head of its list `0x01a85578`; `atexit(0x00b0b5c0)`.
    StaticInit_Key_R_00af92f0 @ 0x00af_92f0 => extern "C" fn s_00af92f0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[151])
    }
    /// `StaticInit_Key_RightBracket_00af9330`: type-enum value `"Key_RightBracket"`: when `0x00a32560` finds no such name, the node `0x00c6c9bc` goes on the head of its list `0x01a85578`; `atexit(0x00b0b5e0)`.
    StaticInit_Key_RightBracket_00af9330 @ 0x00af_9330 => extern "C" fn s_00af9330() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[152])
    }
    /// `StaticInit_Key_Right_Alt_00af9370`: type-enum value `"Key_Right_Alt"`: when `0x00a32560` finds no such name, the node `0x00c6c75c` goes on the head of its list `0x01a85578`; `atexit(0x00b0b600)`.
    StaticInit_Key_Right_Alt_00af9370 @ 0x00af_9370 => extern "C" fn s_00af9370() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[153])
    }
    /// `StaticInit_Key_Right_Control_00af93b0`: type-enum value `"Key_Right_Control"`: when `0x00a32560` finds no such name, the node `0x00c6c77c` goes on the head of its list `0x01a85578`; `atexit(0x00b0b620)`.
    StaticInit_Key_Right_Control_00af93b0 @ 0x00af_93b0 => extern "C" fn s_00af93b0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[154])
    }
    /// `StaticInit_Key_Right_Shift_00af93f0`: type-enum value `"Key_Right_Shift"`: when `0x00a32560` finds no such name, the node `0x00c6c76c` goes on the head of its list `0x01a85578`; `atexit(0x00b0b640)`.
    StaticInit_Key_Right_Shift_00af93f0 @ 0x00af_93f0 => extern "C" fn s_00af93f0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[155])
    }
    /// `StaticInit_Key_Right_00af9430`: type-enum value `"Key_Right"`: when `0x00a32560` finds no such name, the node `0x00c6c93c` goes on the head of its list `0x01a85578`; `atexit(0x00b0b660)`.
    StaticInit_Key_Right_00af9430 @ 0x00af_9430 => extern "C" fn s_00af9430() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[156])
    }
    /// `StaticInit_Key_S_00af9470`: type-enum value `"Key_S"`: when `0x00a32560` finds no such name, the node `0x00c6cc1c` goes on the head of its list `0x01a85578`; `atexit(0x00b0b680)`.
    StaticInit_Key_S_00af9470 @ 0x00af_9470 => extern "C" fn s_00af9470() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[157])
    }
    /// `StaticInit_Key_Semicolon_00af94b0`: type-enum value `"Key_Semicolon"`: when `0x00a32560` finds no such name, the node `0x00c6c99c` goes on the head of its list `0x01a85578`; `atexit(0x00b0b6a0)`.
    StaticInit_Key_Semicolon_00af94b0 @ 0x00af_94b0 => extern "C" fn s_00af94b0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[158])
    }
    /// `StaticInit_Key_Slash_00af94f0`: type-enum value `"Key_Slash"`: when `0x00a32560` finds no such name, the node `0x00c6c95c` goes on the head of its list `0x01a85578`; `atexit(0x00b0b6c0)`.
    StaticInit_Key_Slash_00af94f0 @ 0x00af_94f0 => extern "C" fn s_00af94f0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[159])
    }
    /// `StaticInit_Key_Space_00af9530`: type-enum value `"Key_Space"`: when `0x00a32560` finds no such name, the node `0x00c6c9fc` goes on the head of its list `0x01a85578`; `atexit(0x00b0b6e0)`.
    StaticInit_Key_Space_00af9530 @ 0x00af_9530 => extern "C" fn s_00af9530() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[160])
    }
    /// `StaticInit_Key_T_00af9570`: type-enum value `"Key_T"`: when `0x00a32560` finds no such name, the node `0x00c6cc0c` goes on the head of its list `0x01a85578`; `atexit(0x00b0b700)`.
    StaticInit_Key_T_00af9570 @ 0x00af_9570 => extern "C" fn s_00af9570() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[161])
    }
    /// `StaticInit_Key_Tab_00af95b0`: type-enum value `"Key_Tab"`: when `0x00a32560` finds no such name, the node `0x00c6ca0c` goes on the head of its list `0x01a85578`; `atexit(0x00b0b720)`.
    StaticInit_Key_Tab_00af95b0 @ 0x00af_95b0 => extern "C" fn s_00af95b0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[162])
    }
    /// `StaticInit_Key_U_00af95f0`: type-enum value `"Key_U"`: when `0x00a32560` finds no such name, the node `0x00c6cbfc` goes on the head of its list `0x01a85578`; `atexit(0x00b0b740)`.
    StaticInit_Key_U_00af95f0 @ 0x00af_95f0 => extern "C" fn s_00af95f0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[163])
    }
    /// `StaticInit_Key_Up_00af9630`: type-enum value `"Key_Up"`: when `0x00a32560` finds no such name, the node `0x00c6c92c` goes on the head of its list `0x01a85578`; `atexit(0x00b0b760)`.
    StaticInit_Key_Up_00af9630 @ 0x00af_9630 => extern "C" fn s_00af9630() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[164])
    }
    /// `StaticInit_Key_V_00af9670`: type-enum value `"Key_V"`: when `0x00a32560` finds no such name, the node `0x00c6cbec` goes on the head of its list `0x01a85578`; `atexit(0x00b0b780)`.
    StaticInit_Key_V_00af9670 @ 0x00af_9670 => extern "C" fn s_00af9670() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[165])
    }
    /// `StaticInit_Key_W_00af96b0`: type-enum value `"Key_W"`: when `0x00a32560` finds no such name, the node `0x00c6cbdc` goes on the head of its list `0x01a85578`; `atexit(0x00b0b7a0)`.
    StaticInit_Key_W_00af96b0 @ 0x00af_96b0 => extern "C" fn s_00af96b0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[166])
    }
    /// `StaticInit_Key_X_00af96f0`: type-enum value `"Key_X"`: when `0x00a32560` finds no such name, the node `0x00c6cbcc` goes on the head of its list `0x01a85578`; `atexit(0x00b0b7c0)`.
    StaticInit_Key_X_00af96f0 @ 0x00af_96f0 => extern "C" fn s_00af96f0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[167])
    }
    /// `StaticInit_Key_Y_00af9730`: type-enum value `"Key_Y"`: when `0x00a32560` finds no such name, the node `0x00c6cbbc` goes on the head of its list `0x01a85578`; `atexit(0x00b0b7e0)`.
    StaticInit_Key_Y_00af9730 @ 0x00af_9730 => extern "C" fn s_00af9730() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[168])
    }
    /// `StaticInit_Key_Z_00af9770`: type-enum value `"Key_Z"`: when `0x00a32560` finds no such name, the node `0x00c6cbac` goes on the head of its list `0x01a85578`; `atexit(0x00b0b800)`.
    StaticInit_Key_Z_00af9770 @ 0x00af_9770 => extern "C" fn s_00af9770() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[169])
    }
    /// `StaticInit_PGameSettings_00af97b0`: class descriptor `"PGameSettings"` (`cd_desc_clear`) on `0x01a855b8`: constructor, buffers, clear; `atexit(0x00b0b830)`.
    StaticInit_PGameSettings_00af97b0 @ 0x00af_97b0 => extern "C" fn s_00af97b0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[1])
    }
    /// `StaticInit_CriticalSection_00af9870`: `InitializeCriticalSection(0x01a858cc)` through the import slot `0x00b0c150` (stdcall); `atexit(0x00b0b8c0)`.
    StaticInit_CriticalSection_00af9870 @ 0x00af_9870 => extern "C" fn s_00af9870() -> u32 {
        si::import_at_exit(&mut mem(), &si::IMPORT_AT_EXIT[0])
    }
    /// `StaticInit_ChainAppend_00af9890`: appends the object `0x00c85b34` (vtable `0x00b7aa0c`) to the chain at `0x00c901a8` (before the first block at least `0xff` full); `atexit(0x00b0b8d0)`.
    StaticInit_ChainAppend_00af9890 @ 0x00af_9890 => extern "C" fn s_00af9890() -> u32 {
        si::list_append(&mut mem(), &si::LIST_APPENDS[0])
    }
    /// `StaticInit_RenderText_AlphaBlend_00af98e0`: render pass type `"RenderText_AlphaBlend"` built on its global `0x01a85920` (name, flag 0); `atexit(0x00b0b920)`.
    StaticInit_RenderText_AlphaBlend_00af98e0 @ 0x00af_98e0 => extern "C" fn s_00af98e0() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[0])
    }
    /// `StaticInit_RenderText_AlphaTest_00af9900`: render pass type `"RenderText_AlphaTest"` built on its global `0x01a85930` (name, flag 0); `atexit(0x00b0b930)`.
    StaticInit_RenderText_AlphaTest_00af9900 @ 0x00af_9900 => extern "C" fn s_00af9900() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[1])
    }
    /// `StaticInit_RenderText_HardEdges_00af9920`: render pass type `"RenderText_HardEdges"` built on its global `0x01a85940` (name, flag 0); `atexit(0x00b0b940)`.
    StaticInit_RenderText_HardEdges_00af9920 @ 0x00af_9920 => extern "C" fn s_00af9920() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[2])
    }
    /// `StaticInit_RenderText_SoftEdgesAndGlow_00af9940`: render pass type `"RenderText_SoftEdgesAndGlow"` built on its global `0x01a85980` (name, flag 0); `atexit(0x00b0b950)`.
    StaticInit_RenderText_SoftEdgesAndGlow_00af9940 @ 0x00af_9940 => extern "C" fn s_00af9940() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[3])
    }
    /// `StaticInit_RenderText_SoftEdgesAndOutline_00af9960`: render pass type `"RenderText_SoftEdgesAndOutline"` built on its global `0x01a85960` (name, flag 0); `atexit(0x00b0b960)`.
    StaticInit_RenderText_SoftEdgesAndOutline_00af9960 @ 0x00af_9960 => extern "C" fn s_00af9960() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[4])
    }
    /// `StaticInit_RenderText_SoftEdgesAndShadow_00af9980`: render pass type `"RenderText_SoftEdgesAndShadow"` built on its global `0x01a85970` (name, flag 0); `atexit(0x00b0b970)`.
    StaticInit_RenderText_SoftEdgesAndShadow_00af9980 @ 0x00af_9980 => extern "C" fn s_00af9980() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[5])
    }
    /// `StaticInit_RenderText_SoftEdges_00af99a0`: render pass type `"RenderText_SoftEdges"` built on its global `0x01a85950` (name, flag 0); `atexit(0x00b0b980)`.
    StaticInit_RenderText_SoftEdges_00af99a0 @ 0x00af_99a0 => extern "C" fn s_00af99a0() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[6])
    }
    /// `StaticInit_Ctor_00af99c0`: no-argument constructor `0x00a3e730` on the global `0x01a85910` (thiscall), then `atexit(0x00b0b990)`.
    StaticInit_Ctor_00af99c0 @ 0x00af_99c0 => extern "C" fn s_00af99c0() -> u32 {
        si::ctor0_at_exit(&mut mem(), &si::CTOR0_AT_EXIT[0])
    }
    /// `StaticInit_POccluderGeometryObject_00af99e0`: class descriptor `"POccluderGeometryObject"` on `0x01a85990` (base from `GetClassDescriptor`): constructor, buffers; flags `(f & !2 | 0xc) & !8`, the function `0x00a41690` at `0x01a85a24`, the vtable `0x00b7ac34`; `atexit(0x00b0b9e0)`.
    StaticInit_POccluderGeometryObject_00af99e0 @ 0x00af_99e0 => extern "C" fn s_00af99e0() -> u32 {
        si::class_decl_fn(&mut mem(), &si::CLASS_DECLS_FN[0])
    }
    /// `StaticDtor_StoreVt_00af9b00`: atexit destructor: stores the vtable `0x00b0d9cc` in `0x00c8b438` (void: eax untouched).
    StaticDtor_StoreVt_00af9b00 @ 0x00af_9b00 => extern "C" fn s_00af9b00() {
        si::store_word(&mut mem(), &si::STORE_WORDS[0]);
    }
    /// `StaticDtor_StoreVt_00af9b10`: atexit destructor: stores the vtable `0x00b0d9cc` in `0x00c8b440` (void: eax untouched).
    StaticDtor_StoreVt_00af9b10 @ 0x00af_9b10 => extern "C" fn s_00af9b10() {
        si::store_word(&mut mem(), &si::STORE_WORDS[1]);
    }
    /// `StaticDtor_StoreVt_00af9b20`: atexit destructor: stores the vtable `0x00b0d9cc` in `0x00c8b43c` (void: eax untouched).
    StaticDtor_StoreVt_00af9b20 @ 0x00af_9b20 => extern "C" fn s_00af9b20() {
        si::store_word(&mut mem(), &si::STORE_WORDS[2]);
    }
    /// `StaticDtor_Free_00af9b60`: atexit destructor: when bit 0 of the word at `0x00c8ff30` is clear, `Phyre_PhyreFree(that word)` (cdecl) and its eax; else the word.
    StaticDtor_Free_00af9b60 @ 0x00af_9b60 => extern "C" fn s_00af9b60() -> u32 {
        si::free_unless_bit0(&mut mem(), &si::FREE_UNLESS_BIT0[0])
    }
    /// `StaticDtor_Free_00af9b80`: atexit destructor: when bit 0 of the word at `0x00c8ff2c` is clear, `Phyre_PhyreFree(that word)` (cdecl) and its eax; else the word.
    StaticDtor_Free_00af9b80 @ 0x00af_9b80 => extern "C" fn s_00af9b80() -> u32 {
        si::free_unless_bit0(&mut mem(), &si::FREE_UNLESS_BIT0[1])
    }
    /// `StaticDtor_Thiscall_00af9e10`: atexit destructor: ecx = `0x00c90288`, tail-jumps to `0x00433dd0` (thiscall); its eax.
    StaticDtor_Thiscall_00af9e10 @ 0x00af_9e10 => extern "C" fn s_00af9e10() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[0])
    }
    /// `StaticDtor_VtThiscall_00af9f00`: atexit destructor: stores the vtable `0x00b0e16c` in `0x00c902e8`, then ecx = it and tail-jumps to `Phyre_PClassDescriptor_Destruct` (thiscall); its eax.
    StaticDtor_VtThiscall_00af9f00 @ 0x00af_9f00 => extern "C" fn s_00af9f00() -> u32 {
        si::vt_tail(&mut mem(), &si::VT_TAILS[0])
    }
    /// `StaticDtor_VtThiscall_00afa040`: atexit destructor: stores the vtable `0x00b0eae4` in `0x00c90760`, then ecx = it and tail-jumps to `Phyre_PClassDescriptor_Destruct` (thiscall); its eax.
    StaticDtor_VtThiscall_00afa040 @ 0x00af_a040 => extern "C" fn s_00afa040() -> u32 {
        si::vt_tail(&mut mem(), &si::VT_TAILS[1])
    }
    /// `StaticDtor_Thiscall_00afa060`: atexit destructor: ecx = `0x00c907f4`, tail-jumps to `0x00575940` (thiscall); its eax.
    StaticDtor_Thiscall_00afa060 @ 0x00af_a060 => extern "C" fn s_00afa060() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[1])
    }
    /// `StaticDtor_VtThiscall_00afaa00`: atexit destructor: stores the vtable `0x00b10abc` in `0x00c922c0`, then ecx = it and tail-jumps to `Phyre_PClassDescriptor_Destruct` (thiscall); its eax.
    StaticDtor_VtThiscall_00afaa00 @ 0x00af_aa00 => extern "C" fn s_00afaa00() -> u32 {
        si::vt_tail(&mut mem(), &si::VT_TAILS[2])
    }
    /// `StaticDtor_Thiscall_00afaf60`: atexit destructor: ecx = `0x00c940c4`, tail-jumps to `0x0045c1d0` (thiscall); its eax.
    StaticDtor_Thiscall_00afaf60 @ 0x00af_af60 => extern "C" fn s_00afaf60() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[2])
    }
    /// `StaticDtor_Thiscall_00afc2e0`: atexit destructor: calls `0x0047fc10` on `0x00c98668` (thiscall, inside an ebp frame); its eax.
    StaticDtor_Thiscall_00afc2e0 @ 0x00af_c2e0 => extern "C" fn s_00afc2e0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::FRAME_THISCALLS[0])
    }
    /// `StaticDtor_Thiscall_00afc2f0`: atexit destructor: calls `0x0047fc10` on `0x00c986a4` (thiscall, inside an ebp frame); its eax.
    StaticDtor_Thiscall_00afc2f0 @ 0x00af_c2f0 => extern "C" fn s_00afc2f0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::FRAME_THISCALLS[1])
    }
    /// `StaticDtor_Thiscall_00afc300`: atexit destructor: calls `0x0047fc10` on `0x00c98650` (thiscall, inside an ebp frame); its eax.
    StaticDtor_Thiscall_00afc300 @ 0x00af_c300 => extern "C" fn s_00afc300() -> u32 {
        si::tail_thiscall(&mut mem(), &si::FRAME_THISCALLS[2])
    }
    /// `StaticDtor_Thiscall_00afc310`: atexit destructor: calls `0x0047fc10` on `0x00c98698` (thiscall, inside an ebp frame); its eax.
    StaticDtor_Thiscall_00afc310 @ 0x00af_c310 => extern "C" fn s_00afc310() -> u32 {
        si::tail_thiscall(&mut mem(), &si::FRAME_THISCALLS[3])
    }
    /// `StaticDtor_Thiscall_00afc320`: atexit destructor: calls `0x0047fc10` on `0x00c9868c` (thiscall, inside an ebp frame); its eax.
    StaticDtor_Thiscall_00afc320 @ 0x00af_c320 => extern "C" fn s_00afc320() -> u32 {
        si::tail_thiscall(&mut mem(), &si::FRAME_THISCALLS[4])
    }
    /// `StaticDtor_Thiscall_00afc330`: atexit destructor: calls `0x0047fc10` on `0x00c986c8` (thiscall, inside an ebp frame); its eax.
    StaticDtor_Thiscall_00afc330 @ 0x00af_c330 => extern "C" fn s_00afc330() -> u32 {
        si::tail_thiscall(&mut mem(), &si::FRAME_THISCALLS[5])
    }
    /// `StaticDtor_Thiscall_00afc340`: atexit destructor: calls `0x0047fc10` on `0x00c986d4` (thiscall, inside an ebp frame); its eax.
    StaticDtor_Thiscall_00afc340 @ 0x00af_c340 => extern "C" fn s_00afc340() -> u32 {
        si::tail_thiscall(&mut mem(), &si::FRAME_THISCALLS[6])
    }
    /// `StaticDtor_Thiscall_00afc350`: atexit destructor: calls `0x0047fc10` on `0x00c986e0` (thiscall, inside an ebp frame); its eax.
    StaticDtor_Thiscall_00afc350 @ 0x00af_c350 => extern "C" fn s_00afc350() -> u32 {
        si::tail_thiscall(&mut mem(), &si::FRAME_THISCALLS[7])
    }
    /// `StaticDtor_Thiscall_00afc360`: atexit destructor: calls `0x0047fc10` on `0x00c98644` (thiscall, inside an ebp frame); its eax.
    StaticDtor_Thiscall_00afc360 @ 0x00af_c360 => extern "C" fn s_00afc360() -> u32 {
        si::tail_thiscall(&mut mem(), &si::FRAME_THISCALLS[8])
    }
    /// `StaticDtor_Thiscall_00afc370`: atexit destructor: calls `0x0047fc10` on `0x00c986ec` (thiscall, inside an ebp frame); its eax.
    StaticDtor_Thiscall_00afc370 @ 0x00af_c370 => extern "C" fn s_00afc370() -> u32 {
        si::tail_thiscall(&mut mem(), &si::FRAME_THISCALLS[9])
    }
    /// `StaticDtor_Thiscall_00afc380`: atexit destructor: calls `0x0047fc10` on `0x00c9862c` (thiscall, inside an ebp frame); its eax.
    StaticDtor_Thiscall_00afc380 @ 0x00af_c380 => extern "C" fn s_00afc380() -> u32 {
        si::tail_thiscall(&mut mem(), &si::FRAME_THISCALLS[10])
    }
    /// `StaticDtor_Thiscall_00afc390`: atexit destructor: calls `0x0047fc10` on `0x00c98638` (thiscall, inside an ebp frame); its eax.
    StaticDtor_Thiscall_00afc390 @ 0x00af_c390 => extern "C" fn s_00afc390() -> u32 {
        si::tail_thiscall(&mut mem(), &si::FRAME_THISCALLS[11])
    }
    /// `StaticDtor_Thiscall_00afc3a0`: atexit destructor: calls `0x0047fc10` on `0x00c985b4` (thiscall, inside an ebp frame); its eax.
    StaticDtor_Thiscall_00afc3a0 @ 0x00af_c3a0 => extern "C" fn s_00afc3a0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::FRAME_THISCALLS[12])
    }
    /// `StaticDtor_Thiscall_00afc3b0`: atexit destructor: calls `0x0047fc10` on `0x00c98614` (thiscall, inside an ebp frame); its eax.
    StaticDtor_Thiscall_00afc3b0 @ 0x00af_c3b0 => extern "C" fn s_00afc3b0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::FRAME_THISCALLS[13])
    }
    /// `StaticDtor_Thiscall_00afc3c0`: atexit destructor: calls `0x0047fc10` on `0x00c98608` (thiscall, inside an ebp frame); its eax.
    StaticDtor_Thiscall_00afc3c0 @ 0x00af_c3c0 => extern "C" fn s_00afc3c0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::FRAME_THISCALLS[14])
    }
    /// `StaticDtor_Thiscall_00afc3d0`: atexit destructor: calls `0x0047fc10` on `0x00c985fc` (thiscall, inside an ebp frame); its eax.
    StaticDtor_Thiscall_00afc3d0 @ 0x00af_c3d0 => extern "C" fn s_00afc3d0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::FRAME_THISCALLS[15])
    }
    /// `StaticDtor_Thiscall_00afc3e0`: atexit destructor: calls `0x0047fc10` on `0x00c985e4` (thiscall, inside an ebp frame); its eax.
    StaticDtor_Thiscall_00afc3e0 @ 0x00af_c3e0 => extern "C" fn s_00afc3e0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::FRAME_THISCALLS[16])
    }
    /// `StaticDtor_Thiscall_00afc3f0`: atexit destructor: calls `0x0047fc10` on `0x00c985f0` (thiscall, inside an ebp frame); its eax.
    StaticDtor_Thiscall_00afc3f0 @ 0x00af_c3f0 => extern "C" fn s_00afc3f0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::FRAME_THISCALLS[17])
    }
    /// `StaticDtor_Thiscall_00afc400`: atexit destructor: calls `0x0047fc10` on `0x00c985d8` (thiscall, inside an ebp frame); its eax.
    StaticDtor_Thiscall_00afc400 @ 0x00af_c400 => extern "C" fn s_00afc400() -> u32 {
        si::tail_thiscall(&mut mem(), &si::FRAME_THISCALLS[18])
    }
    /// `StaticDtor_Thiscall_00afc410`: atexit destructor: calls `0x0047fc10` on `0x00c9865c` (thiscall, inside an ebp frame); its eax.
    StaticDtor_Thiscall_00afc410 @ 0x00af_c410 => extern "C" fn s_00afc410() -> u32 {
        si::tail_thiscall(&mut mem(), &si::FRAME_THISCALLS[19])
    }
    /// `StaticDtor_Thiscall_00afc420`: atexit destructor: calls `0x0047fc10` on `0x00c98680` (thiscall, inside an ebp frame); its eax.
    StaticDtor_Thiscall_00afc420 @ 0x00af_c420 => extern "C" fn s_00afc420() -> u32 {
        si::tail_thiscall(&mut mem(), &si::FRAME_THISCALLS[20])
    }
    /// `StaticDtor_Thiscall_00afc430`: atexit destructor: calls `0x0047fc10` on `0x00c98674` (thiscall, inside an ebp frame); its eax.
    StaticDtor_Thiscall_00afc430 @ 0x00af_c430 => extern "C" fn s_00afc430() -> u32 {
        si::tail_thiscall(&mut mem(), &si::FRAME_THISCALLS[21])
    }
    /// `StaticDtor_Thiscall_00afc440`: atexit destructor: calls `0x0047fc10` on `0x00c986bc` (thiscall, inside an ebp frame); its eax.
    StaticDtor_Thiscall_00afc440 @ 0x00af_c440 => extern "C" fn s_00afc440() -> u32 {
        si::tail_thiscall(&mut mem(), &si::FRAME_THISCALLS[22])
    }
    /// `StaticDtor_Thiscall_00afc450`: atexit destructor: calls `0x0047fc10` on `0x00c986b0` (thiscall, inside an ebp frame); its eax.
    StaticDtor_Thiscall_00afc450 @ 0x00af_c450 => extern "C" fn s_00afc450() -> u32 {
        si::tail_thiscall(&mut mem(), &si::FRAME_THISCALLS[23])
    }
    /// `StaticDtor_Thiscall_00afc460`: atexit destructor: calls `0x0047fc10` on `0x00c98620` (thiscall, inside an ebp frame); its eax.
    StaticDtor_Thiscall_00afc460 @ 0x00af_c460 => extern "C" fn s_00afc460() -> u32 {
        si::tail_thiscall(&mut mem(), &si::FRAME_THISCALLS[24])
    }
    /// `StaticDtor_Thiscall_00afc470`: atexit destructor: calls `0x0047fc10` on `0x00c985cc` (thiscall, inside an ebp frame); its eax.
    StaticDtor_Thiscall_00afc470 @ 0x00af_c470 => extern "C" fn s_00afc470() -> u32 {
        si::tail_thiscall(&mut mem(), &si::FRAME_THISCALLS[25])
    }
    /// `StaticDtor_Thiscall_00afc480`: atexit destructor: calls `0x0047fc10` on `0x00c985c0` (thiscall, inside an ebp frame); its eax.
    StaticDtor_Thiscall_00afc480 @ 0x00af_c480 => extern "C" fn s_00afc480() -> u32 {
        si::tail_thiscall(&mut mem(), &si::FRAME_THISCALLS[26])
    }
    /// `StaticDtor_Thiscall_00afe060`: atexit destructor: ecx = `0x00ca315c`, tail-jumps to `0x004c0ed0` (thiscall); its eax.
    StaticDtor_Thiscall_00afe060 @ 0x00af_e060 => extern "C" fn s_00afe060() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[3])
    }
    /// `StaticDtor_Thiscall_00afe090`: atexit destructor: ecx = `0x00ca3224`, tail-jumps to `0x004c0ed0` (thiscall); its eax.
    StaticDtor_Thiscall_00afe090 @ 0x00af_e090 => extern "C" fn s_00afe090() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[4])
    }
    /// `StaticDtor_Thiscall_00afe0a0`: atexit destructor: ecx = `0x00ca324c`, tail-jumps to `0x004c0ed0` (thiscall); its eax.
    StaticDtor_Thiscall_00afe0a0 @ 0x00af_e0a0 => extern "C" fn s_00afe0a0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[5])
    }
    /// `StaticDtor_Thiscall_00afe0f0`: atexit destructor: ecx = `0x00ca31ac`, tail-jumps to `0x004c0ed0` (thiscall); its eax.
    StaticDtor_Thiscall_00afe0f0 @ 0x00af_e0f0 => extern "C" fn s_00afe0f0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[6])
    }
    /// `StaticDtor_Thiscall_00afe100`: atexit destructor: ecx = `0x00ca31d4`, tail-jumps to `0x004c0ed0` (thiscall); its eax.
    StaticDtor_Thiscall_00afe100 @ 0x00af_e100 => extern "C" fn s_00afe100() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[7])
    }
    /// `StaticDtor_Thiscall_00afe110`: atexit destructor: ecx = `0x00ca31fc`, tail-jumps to `0x004c0ed0` (thiscall); its eax.
    StaticDtor_Thiscall_00afe110 @ 0x00af_e110 => extern "C" fn s_00afe110() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[8])
    }
    /// `StaticDtor_Thiscall_00afe180`: atexit destructor: ecx = `0x00ca33dc`, tail-jumps to `0x004c0ed0` (thiscall); its eax.
    StaticDtor_Thiscall_00afe180 @ 0x00af_e180 => extern "C" fn s_00afe180() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[9])
    }
    /// `StaticDtor_Thiscall_00afe1f0`: atexit destructor: ecx = `0x00ca33b4`, tail-jumps to `0x004c0ed0` (thiscall); its eax.
    StaticDtor_Thiscall_00afe1f0 @ 0x00af_e1f0 => extern "C" fn s_00afe1f0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[10])
    }
    /// `StaticDtor_Thiscall_00afe220`: atexit destructor: ecx = `0x00ca3184`, tail-jumps to `0x004c0ed0` (thiscall); its eax.
    StaticDtor_Thiscall_00afe220 @ 0x00af_e220 => extern "C" fn s_00afe220() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[11])
    }
    /// `StaticDtor_Thiscall_00afe230`: atexit destructor: ecx = `0x00ca329c`, tail-jumps to `0x004c0ed0` (thiscall); its eax.
    StaticDtor_Thiscall_00afe230 @ 0x00af_e230 => extern "C" fn s_00afe230() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[12])
    }
    /// `StaticDtor_Thiscall_00afe240`: atexit destructor: ecx = `0x00ca310c`, tail-jumps to `0x004c0ed0` (thiscall); its eax.
    StaticDtor_Thiscall_00afe240 @ 0x00af_e240 => extern "C" fn s_00afe240() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[13])
    }
    /// `StaticDtor_Thiscall_00afe250`: atexit destructor: ecx = `0x00ca34cc`, tail-jumps to `0x004c0ed0` (thiscall); its eax.
    StaticDtor_Thiscall_00afe250 @ 0x00af_e250 => extern "C" fn s_00afe250() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[14])
    }
    /// `StaticDtor_VtThiscall_00afe4c0`: atexit destructor: stores the vtable `0x00b1ec74` in `0x00ca4430`, then ecx = it and tail-jumps to `Phyre_PClassDescriptor_Destruct` (thiscall); its eax.
    StaticDtor_VtThiscall_00afe4c0 @ 0x00af_e4c0 => extern "C" fn s_00afe4c0() -> u32 {
        si::vt_tail(&mut mem(), &si::VT_TAILS[3])
    }
    /// `StaticDtor_Thiscall_00afe730`: atexit destructor: ecx = `0x00ca4edc`, tail-jumps to `Phyre_PFreeList_dtor_PFreeList` (thiscall); its eax.
    StaticDtor_Thiscall_00afe730 @ 0x00af_e730 => extern "C" fn s_00afe730() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[15])
    }
    /// `StaticDtor_Thiscall_00afe860`: atexit destructor: ecx = `0x00ca4ef4`, tail-jumps to `Phyre_PFreeList_dtor_PFreeList` (thiscall); its eax.
    StaticDtor_Thiscall_00afe860 @ 0x00af_e860 => extern "C" fn s_00afe860() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[16])
    }
    /// `StaticDtor_Thiscall_00afea50`: atexit destructor: ecx = `0x00ca54fc`, tail-jumps to `Phyre_PFreeList_dtor_PFreeList` (thiscall); its eax.
    StaticDtor_Thiscall_00afea50 @ 0x00af_ea50 => extern "C" fn s_00afea50() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[17])
    }
    /// `StaticDtor_Thiscall_00afea60`: atexit destructor: ecx = `0x00ca5514`, tail-jumps to `Phyre_PFreeList_dtor_PFreeList` (thiscall); its eax.
    StaticDtor_Thiscall_00afea60 @ 0x00af_ea60 => extern "C" fn s_00afea60() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[18])
    }
    /// `StaticDtor_Call_00afea70`: atexit destructor: `0x004327b0`(`0x00c169c4`) (cdecl); its eax.
    StaticDtor_Call_00afea70 @ 0x00af_ea70 => extern "C" fn s_00afea70() -> u32 {
        si::call_arg(&mut mem(), &si::CALL_ARGS[0])
    }
    /// `StaticDtor_Call_00afea80`: atexit destructor: `0x004327b0`(`0x00c16bb4`) (cdecl); its eax.
    StaticDtor_Call_00afea80 @ 0x00af_ea80 => extern "C" fn s_00afea80() -> u32 {
        si::call_arg(&mut mem(), &si::CALL_ARGS[1])
    }
    /// `StaticDtor_Call_00afea90`: atexit destructor: `0x004327b0`(`0x00c16bc4`) (cdecl); its eax.
    StaticDtor_Call_00afea90 @ 0x00af_ea90 => extern "C" fn s_00afea90() -> u32 {
        si::call_arg(&mut mem(), &si::CALL_ARGS[2])
    }
    /// `StaticDtor_Call_00afeaa0`: atexit destructor: `0x004327b0`(`0x00c16a84`) (cdecl); its eax.
    StaticDtor_Call_00afeaa0 @ 0x00af_eaa0 => extern "C" fn s_00afeaa0() -> u32 {
        si::call_arg(&mut mem(), &si::CALL_ARGS[3])
    }
    /// `StaticDtor_Call_00afeab0`: atexit destructor: `0x004327b0`(`0x00c16ba4`) (cdecl); its eax.
    StaticDtor_Call_00afeab0 @ 0x00af_eab0 => extern "C" fn s_00afeab0() -> u32 {
        si::call_arg(&mut mem(), &si::CALL_ARGS[4])
    }
    /// `StaticDtor_Call_00afeac0`: atexit destructor: `0x004327b0`(`0x00c16b94`) (cdecl); its eax.
    StaticDtor_Call_00afeac0 @ 0x00af_eac0 => extern "C" fn s_00afeac0() -> u32 {
        si::call_arg(&mut mem(), &si::CALL_ARGS[5])
    }
    /// `StaticDtor_Call_00afead0`: atexit destructor: `0x004327b0`(`0x00c16b24`) (cdecl); its eax.
    StaticDtor_Call_00afead0 @ 0x00af_ead0 => extern "C" fn s_00afead0() -> u32 {
        si::call_arg(&mut mem(), &si::CALL_ARGS[6])
    }
    /// `StaticDtor_Call_00afeae0`: atexit destructor: `0x004327b0`(`0x00c16b34`) (cdecl); its eax.
    StaticDtor_Call_00afeae0 @ 0x00af_eae0 => extern "C" fn s_00afeae0() -> u32 {
        si::call_arg(&mut mem(), &si::CALL_ARGS[7])
    }
    /// `StaticDtor_Call_00afeaf0`: atexit destructor: `0x004327b0`(`0x00c16b74`) (cdecl); its eax.
    StaticDtor_Call_00afeaf0 @ 0x00af_eaf0 => extern "C" fn s_00afeaf0() -> u32 {
        si::call_arg(&mut mem(), &si::CALL_ARGS[8])
    }
    /// `StaticDtor_Call_00afeb00`: atexit destructor: `0x004327b0`(`0x00c16b14`) (cdecl); its eax.
    StaticDtor_Call_00afeb00 @ 0x00af_eb00 => extern "C" fn s_00afeb00() -> u32 {
        si::call_arg(&mut mem(), &si::CALL_ARGS[9])
    }
    /// `StaticDtor_Call_00afeb10`: atexit destructor: `0x004327b0`(`0x00c16b44`) (cdecl); its eax.
    StaticDtor_Call_00afeb10 @ 0x00af_eb10 => extern "C" fn s_00afeb10() -> u32 {
        si::call_arg(&mut mem(), &si::CALL_ARGS[10])
    }
    /// `StaticDtor_Call_00afeb20`: atexit destructor: `0x004327b0`(`0x00c16bd4`) (cdecl); its eax.
    StaticDtor_Call_00afeb20 @ 0x00af_eb20 => extern "C" fn s_00afeb20() -> u32 {
        si::call_arg(&mut mem(), &si::CALL_ARGS[11])
    }
    /// `StaticDtor_Call_00afeb30`: atexit destructor: `0x004327b0`(`0x00c169e4`) (cdecl); its eax.
    StaticDtor_Call_00afeb30 @ 0x00af_eb30 => extern "C" fn s_00afeb30() -> u32 {
        si::call_arg(&mut mem(), &si::CALL_ARGS[12])
    }
    /// `StaticDtor_Call_00afeb40`: atexit destructor: `0x004327b0`(`0x00c169b4`) (cdecl); its eax.
    StaticDtor_Call_00afeb40 @ 0x00af_eb40 => extern "C" fn s_00afeb40() -> u32 {
        si::call_arg(&mut mem(), &si::CALL_ARGS[13])
    }
    /// `StaticDtor_Call_00afeb50`: atexit destructor: `0x004327b0`(`0x00c16b84`) (cdecl); its eax.
    StaticDtor_Call_00afeb50 @ 0x00af_eb50 => extern "C" fn s_00afeb50() -> u32 {
        si::call_arg(&mut mem(), &si::CALL_ARGS[14])
    }
    /// `StaticDtor_Call_00afeb60`: atexit destructor: `0x004327b0`(`0x00c16af4`) (cdecl); its eax.
    StaticDtor_Call_00afeb60 @ 0x00af_eb60 => extern "C" fn s_00afeb60() -> u32 {
        si::call_arg(&mut mem(), &si::CALL_ARGS[15])
    }
    /// `StaticDtor_Call_00afeb70`: atexit destructor: `0x004327b0`(`0x00c16b64`) (cdecl); its eax.
    StaticDtor_Call_00afeb70 @ 0x00af_eb70 => extern "C" fn s_00afeb70() -> u32 {
        si::call_arg(&mut mem(), &si::CALL_ARGS[16])
    }
    /// `StaticDtor_Call_00afeb80`: atexit destructor: `0x004327b0`(`0x00c16a64`) (cdecl); its eax.
    StaticDtor_Call_00afeb80 @ 0x00af_eb80 => extern "C" fn s_00afeb80() -> u32 {
        si::call_arg(&mut mem(), &si::CALL_ARGS[17])
    }
    /// `StaticDtor_Call_00afeb90`: atexit destructor: `0x004327b0`(`0x00c16a54`) (cdecl); its eax.
    StaticDtor_Call_00afeb90 @ 0x00af_eb90 => extern "C" fn s_00afeb90() -> u32 {
        si::call_arg(&mut mem(), &si::CALL_ARGS[18])
    }
    /// `StaticDtor_Call_00afeba0`: atexit destructor: `0x004327b0`(`0x00c16ae4`) (cdecl); its eax.
    StaticDtor_Call_00afeba0 @ 0x00af_eba0 => extern "C" fn s_00afeba0() -> u32 {
        si::call_arg(&mut mem(), &si::CALL_ARGS[19])
    }
    /// `StaticDtor_Call_00afebb0`: atexit destructor: `0x004327b0`(`0x00c16a94`) (cdecl); its eax.
    StaticDtor_Call_00afebb0 @ 0x00af_ebb0 => extern "C" fn s_00afebb0() -> u32 {
        si::call_arg(&mut mem(), &si::CALL_ARGS[20])
    }
    /// `StaticDtor_Call_00afebc0`: atexit destructor: `0x004327b0`(`0x00c16ac4`) (cdecl); its eax.
    StaticDtor_Call_00afebc0 @ 0x00af_ebc0 => extern "C" fn s_00afebc0() -> u32 {
        si::call_arg(&mut mem(), &si::CALL_ARGS[21])
    }
    /// `StaticDtor_Call_00afebd0`: atexit destructor: `0x004327b0`(`0x00c16ad4`) (cdecl); its eax.
    StaticDtor_Call_00afebd0 @ 0x00af_ebd0 => extern "C" fn s_00afebd0() -> u32 {
        si::call_arg(&mut mem(), &si::CALL_ARGS[22])
    }
    /// `StaticDtor_Call_00afebe0`: atexit destructor: `0x004327b0`(`0x00c16aa4`) (cdecl); its eax.
    StaticDtor_Call_00afebe0 @ 0x00af_ebe0 => extern "C" fn s_00afebe0() -> u32 {
        si::call_arg(&mut mem(), &si::CALL_ARGS[23])
    }
    /// `StaticDtor_Call_00afebf0`: atexit destructor: `0x004327b0`(`0x00c16ab4`) (cdecl); its eax.
    StaticDtor_Call_00afebf0 @ 0x00af_ebf0 => extern "C" fn s_00afebf0() -> u32 {
        si::call_arg(&mut mem(), &si::CALL_ARGS[24])
    }
    /// `StaticDtor_Call_00afec00`: atexit destructor: `0x004327b0`(`0x00c16b54`) (cdecl); its eax.
    StaticDtor_Call_00afec00 @ 0x00af_ec00 => extern "C" fn s_00afec00() -> u32 {
        si::call_arg(&mut mem(), &si::CALL_ARGS[25])
    }
    /// `StaticDtor_Call_00afec10`: atexit destructor: `0x004327b0`(`0x00c16b04`) (cdecl); its eax.
    StaticDtor_Call_00afec10 @ 0x00af_ec10 => extern "C" fn s_00afec10() -> u32 {
        si::call_arg(&mut mem(), &si::CALL_ARGS[26])
    }
    /// `StaticDtor_Call_00afec20`: atexit destructor: `0x004327b0`(`0x00c169f4`) (cdecl); its eax.
    StaticDtor_Call_00afec20 @ 0x00af_ec20 => extern "C" fn s_00afec20() -> u32 {
        si::call_arg(&mut mem(), &si::CALL_ARGS[27])
    }
    /// `StaticDtor_Call_00afec30`: atexit destructor: `0x004327b0`(`0x00c169d4`) (cdecl); its eax.
    StaticDtor_Call_00afec30 @ 0x00af_ec30 => extern "C" fn s_00afec30() -> u32 {
        si::call_arg(&mut mem(), &si::CALL_ARGS[28])
    }
    /// `StaticDtor_Call_00afec40`: atexit destructor: `0x004327b0`(`0x00c16a74`) (cdecl); its eax.
    StaticDtor_Call_00afec40 @ 0x00af_ec40 => extern "C" fn s_00afec40() -> u32 {
        si::call_arg(&mut mem(), &si::CALL_ARGS[29])
    }
    /// `StaticDtor_Call_00afec50`: atexit destructor: `0x004327b0`(`0x00c16a34`) (cdecl); its eax.
    StaticDtor_Call_00afec50 @ 0x00af_ec50 => extern "C" fn s_00afec50() -> u32 {
        si::call_arg(&mut mem(), &si::CALL_ARGS[30])
    }
    /// `StaticDtor_Call_00afec60`: atexit destructor: `0x004327b0`(`0x00c16a44`) (cdecl); its eax.
    StaticDtor_Call_00afec60 @ 0x00af_ec60 => extern "C" fn s_00afec60() -> u32 {
        si::call_arg(&mut mem(), &si::CALL_ARGS[31])
    }
    /// `StaticDtor_Call_00afec70`: atexit destructor: `0x004327b0`(`0x00c16a04`) (cdecl); its eax.
    StaticDtor_Call_00afec70 @ 0x00af_ec70 => extern "C" fn s_00afec70() -> u32 {
        si::call_arg(&mut mem(), &si::CALL_ARGS[32])
    }
    /// `StaticDtor_Call_00afec80`: atexit destructor: `0x004327b0`(`0x00c16a24`) (cdecl); its eax.
    StaticDtor_Call_00afec80 @ 0x00af_ec80 => extern "C" fn s_00afec80() -> u32 {
        si::call_arg(&mut mem(), &si::CALL_ARGS[33])
    }
    /// `StaticDtor_Call_00afec90`: atexit destructor: `0x004327b0`(`0x00c16a14`) (cdecl); its eax.
    StaticDtor_Call_00afec90 @ 0x00af_ec90 => extern "C" fn s_00afec90() -> u32 {
        si::call_arg(&mut mem(), &si::CALL_ARGS[34])
    }
    /// `StaticDtor_VtThiscall_00aff2d0`: atexit destructor: stores the vtable `0x00b22b0c` in `0x00ca7228`, then ecx = it and tail-jumps to `Phyre_PClassDescriptor_Destruct` (thiscall); its eax.
    StaticDtor_VtThiscall_00aff2d0 @ 0x00af_f2d0 => extern "C" fn s_00aff2d0() -> u32 {
        si::vt_tail(&mut mem(), &si::VT_TAILS[4])
    }
    /// `StaticDtor_Thiscall_00aff950`: atexit destructor: ecx = `0x00ca8998`, tail-jumps to `Phyre_PRendering_PRenderSurface_dtor_PRenderSurface` (thiscall); its eax.
    StaticDtor_Thiscall_00aff950 @ 0x00af_f950 => extern "C" fn s_00aff950() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[19])
    }
    /// `StaticDtor_Thiscall_00aff960`: atexit destructor: ecx = `0x00ca89a8`, tail-jumps to `Phyre_PRendering_PRenderSurface_dtor_PRenderSurface` (thiscall); its eax.
    StaticDtor_Thiscall_00aff960 @ 0x00af_f960 => extern "C" fn s_00aff960() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[20])
    }
    /// `StaticDtor_VtThiscall_00b00f70`: atexit destructor: stores the vtable `0x00b2a374` in `0x00cacd18`, then ecx = it and tail-jumps to `Phyre_PClassDescriptor_Destruct` (thiscall); its eax.
    StaticDtor_VtThiscall_00b00f70 @ 0x00b0_0f70 => extern "C" fn s_00b00f70() -> u32 {
        si::vt_tail(&mut mem(), &si::VT_TAILS[5])
    }
    /// `StaticDtor_VtCall_00b01940`: atexit destructor: stores the vtable `0x00b2be10` in `0x00c20868`, then `0x00545e80`(`0x00c20868`) (cdecl); its eax.
    StaticDtor_VtCall_00b01940 @ 0x00b0_1940 => extern "C" fn s_00b01940() -> u32 {
        si::vt_call(&mut mem(), &si::VT_CALLS[0])
    }
    /// `StaticDtor_VtCall_00b01960`: atexit destructor: stores the vtable `0x00b2be10` in `0x00c20874`, then `0x00545e80`(`0x00c20874`) (cdecl); its eax.
    StaticDtor_VtCall_00b01960 @ 0x00b0_1960 => extern "C" fn s_00b01960() -> u32 {
        si::vt_call(&mut mem(), &si::VT_CALLS[1])
    }
    /// `StaticDtor_VtCall_00b01980`: atexit destructor: stores the vtable `0x00b2be10` in `0x00c20880`, then `0x00545e80`(`0x00c20880`) (cdecl); its eax.
    StaticDtor_VtCall_00b01980 @ 0x00b0_1980 => extern "C" fn s_00b01980() -> u32 {
        si::vt_call(&mut mem(), &si::VT_CALLS[2])
    }
    /// `StaticDtor_Thiscall_00b01de0`: atexit destructor: ecx = `0x00caecec`, tail-jumps to `Phyre_PFreeList_dtor_PFreeList` (thiscall); its eax.
    StaticDtor_Thiscall_00b01de0 @ 0x00b0_1de0 => extern "C" fn s_00b01de0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[21])
    }
    /// `StaticDtor_Thiscall_00b01df0`: atexit destructor: ecx = `0x00caed04`, tail-jumps to `Phyre_PFreeList_dtor_PFreeList` (thiscall); its eax.
    StaticDtor_Thiscall_00b01df0 @ 0x00b0_1df0 => extern "C" fn s_00b01df0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[22])
    }
    /// `StaticDtor_Call_00b01e00`: atexit destructor: `0x00433550`(`0x00c21360`) (cdecl); its eax.
    StaticDtor_Call_00b01e00 @ 0x00b0_1e00 => extern "C" fn s_00b01e00() -> u32 {
        si::call_arg(&mut mem(), &si::CALL_ARGS[35])
    }
    /// `StaticDtor_VtThiscall_00b02c10`: atexit destructor: stores the vtable `0x00b2f85c` in `0x00cb1cf0`, then ecx = it and tail-jumps to `Phyre_PClassDescriptor_Destruct` (thiscall); its eax.
    StaticDtor_VtThiscall_00b02c10 @ 0x00b0_2c10 => extern "C" fn s_00b02c10() -> u32 {
        si::vt_tail(&mut mem(), &si::VT_TAILS[6])
    }
    /// `StaticDtor_Thiscall_00b03680`: atexit destructor: ecx = `0x00cb3c94`, tail-jumps to `0x00463e70` (thiscall); its eax.
    StaticDtor_Thiscall_00b03680 @ 0x00b0_3680 => extern "C" fn s_00b03680() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[23])
    }
    /// `StaticDtor_Thiscall_00b04760`: atexit destructor: ecx = `0x00c294d8`, tail-jumps to `0x005c82a0` (thiscall); its eax.
    StaticDtor_Thiscall_00b04760 @ 0x00b0_4760 => extern "C" fn s_00b04760() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[24])
    }
    /// `StaticDtor_Thiscall_00b04770`: atexit destructor: ecx = `0x00c29518`, tail-jumps to `0x005c82a0` (thiscall); its eax.
    StaticDtor_Thiscall_00b04770 @ 0x00b0_4770 => extern "C" fn s_00b04770() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[25])
    }
    /// `StaticDtor_Thiscall_00b04780`: atexit destructor: ecx = `0x00c29498`, tail-jumps to `0x005c82a0` (thiscall); its eax.
    StaticDtor_Thiscall_00b04780 @ 0x00b0_4780 => extern "C" fn s_00b04780() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[26])
    }
    /// `StaticDtor_Thiscall_00b04790`: atexit destructor: ecx = `0x00c29598`, tail-jumps to `0x005c82a0` (thiscall); its eax.
    StaticDtor_Thiscall_00b04790 @ 0x00b0_4790 => extern "C" fn s_00b04790() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[27])
    }
    /// `StaticDtor_Thiscall_00b047a0`: atexit destructor: ecx = `0x00c29558`, tail-jumps to `0x005c82a0` (thiscall); its eax.
    StaticDtor_Thiscall_00b047a0 @ 0x00b0_47a0 => extern "C" fn s_00b047a0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[28])
    }
    /// `StaticDtor_Thiscall_00b047d0`: atexit destructor: ecx = `0x00cbd428`, tail-jumps to `0x005c82a0` (thiscall); its eax.
    StaticDtor_Thiscall_00b047d0 @ 0x00b0_47d0 => extern "C" fn s_00b047d0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[29])
    }
    /// `StaticDtor_Thiscall_00b047e0`: atexit destructor: ecx = `0x00cbd5a8`, tail-jumps to `0x005c82a0` (thiscall); its eax.
    StaticDtor_Thiscall_00b047e0 @ 0x00b0_47e0 => extern "C" fn s_00b047e0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[30])
    }
    /// `StaticDtor_Thiscall_00b047f0`: atexit destructor: ecx = `0x00cbd728`, tail-jumps to `0x005c82a0` (thiscall); its eax.
    StaticDtor_Thiscall_00b047f0 @ 0x00b0_47f0 => extern "C" fn s_00b047f0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[31])
    }
    /// `StaticDtor_Thiscall_00b04800`: atexit destructor: ecx = `0x00cbd4a8`, tail-jumps to `0x005c82a0` (thiscall); its eax.
    StaticDtor_Thiscall_00b04800 @ 0x00b0_4800 => extern "C" fn s_00b04800() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[32])
    }
    /// `StaticDtor_Thiscall_00b04810`: atexit destructor: ecx = `0x00cbd628`, tail-jumps to `0x005c82a0` (thiscall); its eax.
    StaticDtor_Thiscall_00b04810 @ 0x00b0_4810 => extern "C" fn s_00b04810() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[33])
    }
    /// `StaticDtor_Thiscall_00b04820`: atexit destructor: ecx = `0x00cbd7a8`, tail-jumps to `0x005c82a0` (thiscall); its eax.
    StaticDtor_Thiscall_00b04820 @ 0x00b0_4820 => extern "C" fn s_00b04820() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[34])
    }
    /// `StaticDtor_Thiscall_00b04830`: atexit destructor: ecx = `0x00cbd528`, tail-jumps to `0x005c82a0` (thiscall); its eax.
    StaticDtor_Thiscall_00b04830 @ 0x00b0_4830 => extern "C" fn s_00b04830() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[35])
    }
    /// `StaticDtor_Thiscall_00b04840`: atexit destructor: ecx = `0x00cbd6a8`, tail-jumps to `0x005c82a0` (thiscall); its eax.
    StaticDtor_Thiscall_00b04840 @ 0x00b0_4840 => extern "C" fn s_00b04840() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[36])
    }
    /// `StaticDtor_Thiscall_00b04850`: atexit destructor: ecx = `0x00cbd828`, tail-jumps to `0x005c82a0` (thiscall); its eax.
    StaticDtor_Thiscall_00b04850 @ 0x00b0_4850 => extern "C" fn s_00b04850() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[37])
    }
    /// `StaticDtor_VtCall_00b04870`: atexit destructor: stores the vtable `0x00b38558` in `0x00c29794`, then `0x005cbf40`(`0x00c29794`) (cdecl); its eax.
    StaticDtor_VtCall_00b04870 @ 0x00b0_4870 => extern "C" fn s_00b04870() -> u32 {
        si::vt_call(&mut mem(), &si::VT_CALLS[3])
    }
    /// `StaticDtor_VtCall_00b04890`: atexit destructor: stores the vtable `0x00b38558` in `0x00c29788`, then `0x005cbf40`(`0x00c29788`) (cdecl); its eax.
    StaticDtor_VtCall_00b04890 @ 0x00b0_4890 => extern "C" fn s_00b04890() -> u32 {
        si::vt_call(&mut mem(), &si::VT_CALLS[4])
    }
    /// `StaticDtor_VtCall_00b048b0`: atexit destructor: stores the vtable `0x00b38558` in `0x00c297a0`, then `0x005cbf40`(`0x00c297a0`) (cdecl); its eax.
    StaticDtor_VtCall_00b048b0 @ 0x00b0_48b0 => extern "C" fn s_00b048b0() -> u32 {
        si::vt_call(&mut mem(), &si::VT_CALLS[5])
    }
    /// `StaticDtor_VtCall_00b048e0`: atexit destructor: stores the vtable `0x00b385d0` in `0x00c298bc`, then `0x005cc3a0`(`0x00c298bc`) (cdecl); its eax.
    StaticDtor_VtCall_00b048e0 @ 0x00b0_48e0 => extern "C" fn s_00b048e0() -> u32 {
        si::vt_call(&mut mem(), &si::VT_CALLS[6])
    }
    /// `StaticDtor_VtCall_00b04900`: atexit destructor: stores the vtable `0x00b385d0` in `0x00c298c8`, then `0x005cc3a0`(`0x00c298c8`) (cdecl); its eax.
    StaticDtor_VtCall_00b04900 @ 0x00b0_4900 => extern "C" fn s_00b04900() -> u32 {
        si::vt_call(&mut mem(), &si::VT_CALLS[7])
    }
    /// `StaticDtor_VtThiscall_00b04940`: atexit destructor: stores the vtable `0x00b389f4` in `0x00cbd9c8`, then ecx = it and tail-jumps to `Phyre_PClassDescriptor_Destruct` (thiscall); its eax.
    StaticDtor_VtThiscall_00b04940 @ 0x00b0_4940 => extern "C" fn s_00b04940() -> u32 {
        si::vt_tail(&mut mem(), &si::VT_TAILS[7])
    }
    /// `StaticDtor_VtThiscall_00b04cb0`: atexit destructor: stores the vtable `0x00b3afd4` in `0x00cbe940`, then ecx = it and tail-jumps to `Phyre_PClassDescriptor_Destruct` (thiscall); its eax.
    StaticDtor_VtThiscall_00b04cb0 @ 0x00b0_4cb0 => extern "C" fn s_00b04cb0() -> u32 {
        si::vt_tail(&mut mem(), &si::VT_TAILS[8])
    }
    /// `StaticDtor_VtThiscall_00b04cf0`: atexit destructor: stores the vtable `0x00b3b0cc` in `0x00cbe9d8`, then ecx = it and tail-jumps to `Phyre_PClassDescriptor_Destruct` (thiscall); its eax.
    StaticDtor_VtThiscall_00b04cf0 @ 0x00b0_4cf0 => extern "C" fn s_00b04cf0() -> u32 {
        si::vt_tail(&mut mem(), &si::VT_TAILS[9])
    }
    /// `StaticDtor_VtThiscall_00b04f10`: atexit destructor: stores the vtable `0x00b3abec` in `0x00cbf0f8`, then ecx = it and tail-jumps to `Phyre_PClassDescriptor_Destruct` (thiscall); its eax.
    StaticDtor_VtThiscall_00b04f10 @ 0x00b0_4f10 => extern "C" fn s_00b04f10() -> u32 {
        si::vt_tail(&mut mem(), &si::VT_TAILS[10])
    }
    /// `StaticDtor_VtThiscall_00b060d0`: atexit destructor: stores the vtable `0x00b3fdac` in `0x00cc9d20`, then ecx = it and tail-jumps to `Phyre_PClassDescriptor_Destruct` (thiscall); its eax.
    StaticDtor_VtThiscall_00b060d0 @ 0x00b0_60d0 => extern "C" fn s_00b060d0() -> u32 {
        si::vt_tail(&mut mem(), &si::VT_TAILS[11])
    }
    /// `StaticDtor_VtThiscall_00b06110`: atexit destructor: stores the vtable `0x00b3fcc4` in `0x00cc9e50`, then ecx = it and tail-jumps to `Phyre_PClassDescriptor_Destruct` (thiscall); its eax.
    StaticDtor_VtThiscall_00b06110 @ 0x00b0_6110 => extern "C" fn s_00b06110() -> u32 {
        si::vt_tail(&mut mem(), &si::VT_TAILS[12])
    }
    /// `StaticDtor_Free_00b06770`: atexit destructor: when bit 0 of the word at `0x00ccb6f0` is clear, `Phyre_PhyreFree(that word)` (cdecl) and its eax; else the word.
    StaticDtor_Free_00b06770 @ 0x00b0_6770 => extern "C" fn s_00b06770() -> u32 {
        si::free_unless_bit0(&mut mem(), &si::FREE_UNLESS_BIT0[2])
    }
    /// `StaticDtor_Free_00b06790`: atexit destructor: when bit 0 of the word at `0x00ccb6ec` is clear, `Phyre_PhyreFree(that word)` (cdecl) and its eax; else the word.
    StaticDtor_Free_00b06790 @ 0x00b0_6790 => extern "C" fn s_00b06790() -> u32 {
        si::free_unless_bit0(&mut mem(), &si::FREE_UNLESS_BIT0[3])
    }
    /// `StaticDtor_CloseHandle_00b06810`: atexit destructor: `CloseHandle([0x00ccb6d0])` through the import slot `0x00b0c0dc` (stdcall); its eax.
    StaticDtor_CloseHandle_00b06810 @ 0x00b0_6810 => extern "C" fn s_00b06810() -> u32 {
        si::import_word(&mut mem(), &si::IMPORT_WORDS[0])
    }
    /// `StaticDtor_Thiscall_00b06850`: atexit destructor: ecx = `0x00ccb9f8`, tail-jumps to `0x004dbab0` (thiscall); its eax.
    StaticDtor_Thiscall_00b06850 @ 0x00b0_6850 => extern "C" fn s_00b06850() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[38])
    }
    /// `StaticDtor_Free_00b06860`: atexit destructor: when bit 0 of the word at `0x00ccb9e4` is clear, `Phyre_PhyreFree(that word)` (cdecl) and its eax; else the word.
    StaticDtor_Free_00b06860 @ 0x00b0_6860 => extern "C" fn s_00b06860() -> u32 {
        si::free_unless_bit0(&mut mem(), &si::FREE_UNLESS_BIT0[4])
    }
    /// `StaticDtor_Free_00b06880`: atexit destructor: when bit 0 of the word at `0x00ccb9e0` is clear, `Phyre_PhyreFree(that word)` (cdecl) and its eax; else the word.
    StaticDtor_Free_00b06880 @ 0x00b0_6880 => extern "C" fn s_00b06880() -> u32 {
        si::free_unless_bit0(&mut mem(), &si::FREE_UNLESS_BIT0[5])
    }
    /// `StaticDtor_Thiscall_00b068a0`: atexit destructor: ecx = `0x00ccb9e8`, tail-jumps to `0x004dbab0` (thiscall); its eax.
    StaticDtor_Thiscall_00b068a0 @ 0x00b0_68a0 => extern "C" fn s_00b068a0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[39])
    }
    /// `StaticDtor_Free_00b06c10`: atexit destructor: when bit 0 of the word at `0x00ccc82c` is clear, `Phyre_PhyreFree(that word)` (cdecl) and its eax; else the word.
    StaticDtor_Free_00b06c10 @ 0x00b0_6c10 => extern "C" fn s_00b06c10() -> u32 {
        si::free_unless_bit0(&mut mem(), &si::FREE_UNLESS_BIT0[6])
    }
    /// `StaticDtor_Free_00b06c30`: atexit destructor: when bit 0 of the word at `0x00ccc828` is clear, `Phyre_PhyreFree(that word)` (cdecl) and its eax; else the word.
    StaticDtor_Free_00b06c30 @ 0x00b0_6c30 => extern "C" fn s_00b06c30() -> u32 {
        si::free_unless_bit0(&mut mem(), &si::FREE_UNLESS_BIT0[7])
    }
    /// `StaticDtor_Thiscall_00b06df0`: atexit destructor: ecx = `0x00ccca1c`, tail-jumps to `0x004dbab0` (thiscall); its eax.
    StaticDtor_Thiscall_00b06df0 @ 0x00b0_6df0 => extern "C" fn s_00b06df0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[40])
    }
    /// `StaticDtor_Thiscall_00b06e00`: atexit destructor: ecx = `0x00ccca3c`, tail-jumps to `0x004dbab0` (thiscall); its eax.
    StaticDtor_Thiscall_00b06e00 @ 0x00b0_6e00 => extern "C" fn s_00b06e00() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[41])
    }
    /// `StaticDtor_Thiscall_00b06e10`: atexit destructor: ecx = `0x00ccc8ac`, tail-jumps to `0x004dbab0` (thiscall); its eax.
    StaticDtor_Thiscall_00b06e10 @ 0x00b0_6e10 => extern "C" fn s_00b06e10() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[42])
    }
    /// `StaticDtor_Thiscall_00b06e20`: atexit destructor: ecx = `0x00ccc9fc`, tail-jumps to `0x004dbab0` (thiscall); its eax.
    StaticDtor_Thiscall_00b06e20 @ 0x00b0_6e20 => extern "C" fn s_00b06e20() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[43])
    }
    /// `StaticDtor_Thiscall_00b06e30`: atexit destructor: ecx = `0x00ccc8fc`, tail-jumps to `0x004dbab0` (thiscall); its eax.
    StaticDtor_Thiscall_00b06e30 @ 0x00b0_6e30 => extern "C" fn s_00b06e30() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[44])
    }
    /// `StaticDtor_Thiscall_00b06e40`: atexit destructor: ecx = `0x00ccc8dc`, tail-jumps to `0x004dbab0` (thiscall); its eax.
    StaticDtor_Thiscall_00b06e40 @ 0x00b0_6e40 => extern "C" fn s_00b06e40() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[45])
    }
    /// `StaticDtor_Thiscall_00b06e50`: atexit destructor: ecx = `0x00ccc8bc`, tail-jumps to `0x004dbab0` (thiscall); its eax.
    StaticDtor_Thiscall_00b06e50 @ 0x00b0_6e50 => extern "C" fn s_00b06e50() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[46])
    }
    /// `StaticDtor_Thiscall_00b06e60`: atexit destructor: ecx = `0x00ccc8cc`, tail-jumps to `0x004dbab0` (thiscall); its eax.
    StaticDtor_Thiscall_00b06e60 @ 0x00b0_6e60 => extern "C" fn s_00b06e60() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[47])
    }
    /// `StaticDtor_Thiscall_00b06e70`: atexit destructor: ecx = `0x00ccc90c`, tail-jumps to `0x004dbab0` (thiscall); its eax.
    StaticDtor_Thiscall_00b06e70 @ 0x00b0_6e70 => extern "C" fn s_00b06e70() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[48])
    }
    /// `StaticDtor_Thiscall_00b06e80`: atexit destructor: ecx = `0x00ccc8ec`, tail-jumps to `0x004dbab0` (thiscall); its eax.
    StaticDtor_Thiscall_00b06e80 @ 0x00b0_6e80 => extern "C" fn s_00b06e80() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[49])
    }
    /// `StaticDtor_Thiscall_00b06e90`: atexit destructor: ecx = `0x00ccc94c`, tail-jumps to `0x004dbab0` (thiscall); its eax.
    StaticDtor_Thiscall_00b06e90 @ 0x00b0_6e90 => extern "C" fn s_00b06e90() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[50])
    }
    /// `StaticDtor_Thiscall_00b06ea0`: atexit destructor: ecx = `0x00ccc9cc`, tail-jumps to `0x004dbab0` (thiscall); its eax.
    StaticDtor_Thiscall_00b06ea0 @ 0x00b0_6ea0 => extern "C" fn s_00b06ea0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[51])
    }
    /// `StaticDtor_Thiscall_00b06eb0`: atexit destructor: ecx = `0x00ccc99c`, tail-jumps to `0x004dbab0` (thiscall); its eax.
    StaticDtor_Thiscall_00b06eb0 @ 0x00b0_6eb0 => extern "C" fn s_00b06eb0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[52])
    }
    /// `StaticDtor_Thiscall_00b06ec0`: atexit destructor: ecx = `0x00ccc98c`, tail-jumps to `0x004dbab0` (thiscall); its eax.
    StaticDtor_Thiscall_00b06ec0 @ 0x00b0_6ec0 => extern "C" fn s_00b06ec0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[53])
    }
    /// `StaticDtor_Thiscall_00b06ed0`: atexit destructor: ecx = `0x00ccc9ac`, tail-jumps to `0x004dbab0` (thiscall); its eax.
    StaticDtor_Thiscall_00b06ed0 @ 0x00b0_6ed0 => extern "C" fn s_00b06ed0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[54])
    }
    /// `StaticDtor_Thiscall_00b06ee0`: atexit destructor: ecx = `0x00ccc9bc`, tail-jumps to `0x004dbab0` (thiscall); its eax.
    StaticDtor_Thiscall_00b06ee0 @ 0x00b0_6ee0 => extern "C" fn s_00b06ee0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[55])
    }
    /// `StaticDtor_Thiscall_00b06ef0`: atexit destructor: ecx = `0x00ccca0c`, tail-jumps to `0x004dbab0` (thiscall); its eax.
    StaticDtor_Thiscall_00b06ef0 @ 0x00b0_6ef0 => extern "C" fn s_00b06ef0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[56])
    }
    /// `StaticDtor_Thiscall_00b06f00`: atexit destructor: ecx = `0x00ccca2c`, tail-jumps to `0x004dbab0` (thiscall); its eax.
    StaticDtor_Thiscall_00b06f00 @ 0x00b0_6f00 => extern "C" fn s_00b06f00() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[57])
    }
    /// `StaticDtor_Thiscall_00b06f10`: atexit destructor: ecx = `0x00ccc91c`, tail-jumps to `0x004dbab0` (thiscall); its eax.
    StaticDtor_Thiscall_00b06f10 @ 0x00b0_6f10 => extern "C" fn s_00b06f10() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[58])
    }
    /// `StaticDtor_Thiscall_00b06f20`: atexit destructor: ecx = `0x00ccc92c`, tail-jumps to `0x004dbab0` (thiscall); its eax.
    StaticDtor_Thiscall_00b06f20 @ 0x00b0_6f20 => extern "C" fn s_00b06f20() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[59])
    }
    /// `StaticDtor_Thiscall_00b06f30`: atexit destructor: ecx = `0x00ccc93c`, tail-jumps to `0x004dbab0` (thiscall); its eax.
    StaticDtor_Thiscall_00b06f30 @ 0x00b0_6f30 => extern "C" fn s_00b06f30() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[60])
    }
    /// `StaticDtor_Thiscall_00b06f40`: atexit destructor: ecx = `0x00ccc96c`, tail-jumps to `0x004dbab0` (thiscall); its eax.
    StaticDtor_Thiscall_00b06f40 @ 0x00b0_6f40 => extern "C" fn s_00b06f40() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[61])
    }
    /// `StaticDtor_Thiscall_00b06f50`: atexit destructor: ecx = `0x00ccc97c`, tail-jumps to `0x004dbab0` (thiscall); its eax.
    StaticDtor_Thiscall_00b06f50 @ 0x00b0_6f50 => extern "C" fn s_00b06f50() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[62])
    }
    /// `StaticDtor_Thiscall_00b06f60`: atexit destructor: ecx = `0x00ccc95c`, tail-jumps to `0x004dbab0` (thiscall); its eax.
    StaticDtor_Thiscall_00b06f60 @ 0x00b0_6f60 => extern "C" fn s_00b06f60() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[63])
    }
    /// `StaticDtor_Thiscall_00b06f70`: atexit destructor: ecx = `0x00ccc9ec`, tail-jumps to `0x004dbab0` (thiscall); its eax.
    StaticDtor_Thiscall_00b06f70 @ 0x00b0_6f70 => extern "C" fn s_00b06f70() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[64])
    }
    /// `StaticDtor_Thiscall_00b06f80`: atexit destructor: ecx = `0x00ccc9dc`, tail-jumps to `0x004dbab0` (thiscall); its eax.
    StaticDtor_Thiscall_00b06f80 @ 0x00b0_6f80 => extern "C" fn s_00b06f80() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[65])
    }
    /// `StaticDtor_CloseHandle_00b06ff0`: atexit destructor: `CloseHandle([0x00ccca78])` through the import slot `0x00b0c0dc` (stdcall); its eax.
    StaticDtor_CloseHandle_00b06ff0 @ 0x00b0_6ff0 => extern "C" fn s_00b06ff0() -> u32 {
        si::import_word(&mut mem(), &si::IMPORT_WORDS[1])
    }
    /// `StaticDtor_CloseHandle_00b07080`: atexit destructor: `CloseHandle([0x00cccaa0])` through the import slot `0x00b0c0dc` (stdcall); its eax.
    StaticDtor_CloseHandle_00b07080 @ 0x00b0_7080 => extern "C" fn s_00b07080() -> u32 {
        si::import_word(&mut mem(), &si::IMPORT_WORDS[2])
    }
    /// `StaticDtor_Thiscall_00b071b0`: atexit destructor: ecx = `0x00cdeb90`, tail-jumps to `0x004dbab0` (thiscall); its eax.
    StaticDtor_Thiscall_00b071b0 @ 0x00b0_71b0 => extern "C" fn s_00b071b0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[66])
    }
    /// `StaticDtor_Thiscall_00b071c0`: atexit destructor: ecx = `0x00cdebd8`, tail-jumps to `0x004dbab0` (thiscall); its eax.
    StaticDtor_Thiscall_00b071c0 @ 0x00b0_71c0 => extern "C" fn s_00b071c0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[67])
    }
    /// `StaticDtor_Thiscall_00b071d0`: atexit destructor: ecx = `0x00cdebc8`, tail-jumps to `0x004dbab0` (thiscall); its eax.
    StaticDtor_Thiscall_00b071d0 @ 0x00b0_71d0 => extern "C" fn s_00b071d0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[68])
    }
    /// `StaticDtor_Thiscall_00b071e0`: atexit destructor: ecx = `0x00cdebe8`, tail-jumps to `0x004dbab0` (thiscall); its eax.
    StaticDtor_Thiscall_00b071e0 @ 0x00b0_71e0 => extern "C" fn s_00b071e0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[69])
    }
    /// `StaticDtor_Thiscall_00b071f0`: atexit destructor: ecx = `0x00cdebb8`, tail-jumps to `0x004dbab0` (thiscall); its eax.
    StaticDtor_Thiscall_00b071f0 @ 0x00b0_71f0 => extern "C" fn s_00b071f0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[70])
    }
    /// `StaticDtor_Thiscall_00b07200`: atexit destructor: ecx = `0x00cdeba8`, tail-jumps to `0x004dbab0` (thiscall); its eax.
    StaticDtor_Thiscall_00b07200 @ 0x00b0_7200 => extern "C" fn s_00b07200() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[71])
    }
    /// `StaticDtor_Thiscall_00b072f0`: atexit destructor: ecx = `0x00cdec68`, tail-jumps to `0x004dbab0` (thiscall); its eax.
    StaticDtor_Thiscall_00b072f0 @ 0x00b0_72f0 => extern "C" fn s_00b072f0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[72])
    }
    /// `StaticDtor_Thiscall_00b07300`: atexit destructor: ecx = `0x00cdec38`, tail-jumps to `0x004dbab0` (thiscall); its eax.
    StaticDtor_Thiscall_00b07300 @ 0x00b0_7300 => extern "C" fn s_00b07300() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[73])
    }
    /// `StaticDtor_Thiscall_00b07310`: atexit destructor: ecx = `0x00cdec58`, tail-jumps to `0x004dbab0` (thiscall); its eax.
    StaticDtor_Thiscall_00b07310 @ 0x00b0_7310 => extern "C" fn s_00b07310() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[74])
    }
    /// `StaticDtor_Thiscall_00b07320`: atexit destructor: ecx = `0x00cdec78`, tail-jumps to `0x004dbab0` (thiscall); its eax.
    StaticDtor_Thiscall_00b07320 @ 0x00b0_7320 => extern "C" fn s_00b07320() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[75])
    }
    /// `StaticDtor_Thiscall_00b07330`: atexit destructor: ecx = `0x00cdec48`, tail-jumps to `0x004dbab0` (thiscall); its eax.
    StaticDtor_Thiscall_00b07330 @ 0x00b0_7330 => extern "C" fn s_00b07330() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[76])
    }
    /// `StaticDtor_Thiscall_00b07370`: atexit destructor: ecx = `0x00cded10`, tail-jumps to `0x004dbab0` (thiscall); its eax.
    StaticDtor_Thiscall_00b07370 @ 0x00b0_7370 => extern "C" fn s_00b07370() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[77])
    }
    /// `StaticDtor_Thiscall_00b07380`: atexit destructor: ecx = `0x00cded00`, tail-jumps to `0x004dbab0` (thiscall); its eax.
    StaticDtor_Thiscall_00b07380 @ 0x00b0_7380 => extern "C" fn s_00b07380() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[78])
    }
    /// `StaticDtor_Thiscall_00b07390`: atexit destructor: ecx = `0x00cdecd0`, tail-jumps to `0x004dbab0` (thiscall); its eax.
    StaticDtor_Thiscall_00b07390 @ 0x00b0_7390 => extern "C" fn s_00b07390() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[79])
    }
    /// `StaticDtor_Thiscall_00b073a0`: atexit destructor: ecx = `0x00cdecb0`, tail-jumps to `0x004dbab0` (thiscall); its eax.
    StaticDtor_Thiscall_00b073a0 @ 0x00b0_73a0 => extern "C" fn s_00b073a0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[80])
    }
    /// `StaticDtor_Thiscall_00b073b0`: atexit destructor: ecx = `0x00cdecf0`, tail-jumps to `0x004dbab0` (thiscall); its eax.
    StaticDtor_Thiscall_00b073b0 @ 0x00b0_73b0 => extern "C" fn s_00b073b0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[81])
    }
    /// `FFXApplication_atexit_dtor`: atexit destructor: the static `FFXApplication`'s destructor (`0x00c8fb78`): its vtable, `WM_CLOSE` when `0x00c8fae4` is 1, tail-jumps to the `PApplication` destructor; its eax.
    FFXApplication_atexit_dtor @ 0x00af_9ba0 => extern "C" fn s_00af9ba0() -> u32 {
        si::ffx_application_dtor(&mut mem())
    }
    /// `StaticDtor_ChainUnlink_00af9c30`: atexit destructor: stores the vtable `0x00b0e078` in `0x00c0a2d0` and unlinks it from the chain at `0x00c901a8` (its link takes the word at `0x00c0a2d4`, which is cleared); the link address.
    StaticDtor_ChainUnlink_00af9c30 @ 0x00af_9c30 => extern "C" fn s_00af9c30() -> u32 {
        si::chain_unlink(&mut mem(), &si::CHAIN_UNLINKS[0])
    }
    /// `StaticDtor_ChainUnlink_00af9c80`: atexit destructor: stores the vtable `0x00b0e078` in `0x00c0a33c` and unlinks it from the chain at `0x00c901a8` (its link takes the word at `0x00c0a340`, which is cleared); the link address.
    StaticDtor_ChainUnlink_00af9c80 @ 0x00af_9c80 => extern "C" fn s_00af9c80() -> u32 {
        si::chain_unlink(&mut mem(), &si::CHAIN_UNLINKS[1])
    }
    /// `StaticDtor_ChainUnlink_00af9cd0`: atexit destructor: stores the vtable `0x00b0e078` in `0x00c0a37c` and unlinks it from the chain at `0x00c901a8` (its link takes the word at `0x00c0a380`, which is cleared); the link address.
    StaticDtor_ChainUnlink_00af9cd0 @ 0x00af_9cd0 => extern "C" fn s_00af9cd0() -> u32 {
        si::chain_unlink(&mut mem(), &si::CHAIN_UNLINKS[2])
    }
    /// `StaticDtor_ChainUnlink_00af9d20`: atexit destructor: stores the vtable `0x00b0e078` in `0x00c0a3c0` and unlinks it from the chain at `0x00c901a8` (its link takes the word at `0x00c0a3c4`, which is cleared); the link address.
    StaticDtor_ChainUnlink_00af9d20 @ 0x00af_9d20 => extern "C" fn s_00af9d20() -> u32 {
        si::chain_unlink(&mut mem(), &si::CHAIN_UNLINKS[3])
    }
    /// `StaticDtor_ChainUnlink_00af9d70`: atexit destructor: stores the vtable `0x00b0e078` in `0x00c0a3fc` and unlinks it from the chain at `0x00c901a8` (its link takes the word at `0x00c0a400`, which is cleared); the link address.
    StaticDtor_ChainUnlink_00af9d70 @ 0x00af_9d70 => extern "C" fn s_00af9d70() -> u32 {
        si::chain_unlink(&mut mem(), &si::CHAIN_UNLINKS[4])
    }
    /// `StaticDtor_ChainUnlink_00af9dc0`: atexit destructor: stores the vtable `0x00b0e078` in `0x00c0a440` and unlinks it from the chain at `0x00c901a8` (its link takes the word at `0x00c0a444`, which is cleared); the link address.
    StaticDtor_ChainUnlink_00af9dc0 @ 0x00af_9dc0 => extern "C" fn s_00af9dc0() -> u32 {
        si::chain_unlink(&mut mem(), &si::CHAIN_UNLINKS[5])
    }
    /// `StaticDtor_Seq_00afa690`: atexit destructor: thiscalls `Phyre_PRedBlackTree_clear` on `0x00c0bb70`, `Phyre_PRedBlackTree_clear` on `0x00c0bb70`; the last one's eax.
    StaticDtor_Seq_00afa690 @ 0x00af_a690 => extern "C" fn s_00afa690() -> u32 {
        si::thiscall_seq(&mut mem(), &si::THISCALL_SEQS[0])
    }
    /// `StaticDtor_Seq_00afb980`: atexit destructor: thiscalls `Phyre_PRedBlackTree_clear` on `0x00c0e2f0`, `Phyre_PRedBlackTree_clear` on `0x00c0e2f0`; the last one's eax.
    StaticDtor_Seq_00afb980 @ 0x00af_b980 => extern "C" fn s_00afb980() -> u32 {
        si::thiscall_seq(&mut mem(), &si::THISCALL_SEQS[1])
    }
    /// `StaticDtor_Detach_00afce00`: atexit destructor: stores the vtable `0x00b170e8` in `0x00c1083c`; when the word at `0x00c10848` is set, `0x0049e920`(`0x00c1083c`) (cdecl) and the entry [`0x00c1084c`] of the table `0x00c9af00` is cleared; that index.
    StaticDtor_Detach_00afce00 @ 0x00af_ce00 => extern "C" fn s_00afce00() -> u32 {
        si::cond_detach(&mut mem(), &si::COND_DETACHES[0])
    }
    /// `StaticDtor_Detach_00afce40`: atexit destructor: stores the vtable `0x00b170e8` in `0x00c10828`; when the word at `0x00c10834` is set, `0x0049e920`(`0x00c10828`) (cdecl) and the entry [`0x00c10838`] of the table `0x00c9af00` is cleared; that index.
    StaticDtor_Detach_00afce40 @ 0x00af_ce40 => extern "C" fn s_00afce40() -> u32 {
        si::cond_detach(&mut mem(), &si::COND_DETACHES[1])
    }
    /// `StaticDtor_Detach_00afce80`: atexit destructor: stores the vtable `0x00b170e8` in `0x00c10814`; when the word at `0x00c10820` is set, `0x0049e920`(`0x00c10814`) (cdecl) and the entry [`0x00c10824`] of the table `0x00c9af00` is cleared; that index.
    StaticDtor_Detach_00afce80 @ 0x00af_ce80 => extern "C" fn s_00afce80() -> u32 {
        si::cond_detach(&mut mem(), &si::COND_DETACHES[2])
    }
    /// `StaticDtor_Detach_00afcec0`: atexit destructor: stores the vtable `0x00b170e8` in `0x00c10850`; when the word at `0x00c1085c` is set, `0x0049e920`(`0x00c10850`) (cdecl) and the entry [`0x00c10860`] of the table `0x00c9af00` is cleared; that index.
    StaticDtor_Detach_00afcec0 @ 0x00af_cec0 => extern "C" fn s_00afcec0() -> u32 {
        si::cond_detach(&mut mem(), &si::COND_DETACHES[3])
    }
    /// `StaticDtor_Detach_00afe4e0`: atexit destructor: stores the vtable `0x00b1eb90` in `0x00c158d8`; when the word at `0x00c158e4` is set, `0x004cc050`(`0x00c158d8`) (cdecl) and the entry [`0x00c158e8`] of the table `0x00ca43b0` is cleared; that index.
    StaticDtor_Detach_00afe4e0 @ 0x00af_e4e0 => extern "C" fn s_00afe4e0() -> u32 {
        si::cond_detach(&mut mem(), &si::COND_DETACHES[4])
    }
    /// `StaticDtor_Detach_00afe520`: atexit destructor: stores the vtable `0x00b1eb90` in `0x00c158ec`; when the word at `0x00c158f8` is set, `0x004cc050`(`0x00c158ec`) (cdecl) and the entry [`0x00c158fc`] of the table `0x00ca43b0` is cleared; that index.
    StaticDtor_Detach_00afe520 @ 0x00af_e520 => extern "C" fn s_00afe520() -> u32 {
        si::cond_detach(&mut mem(), &si::COND_DETACHES[5])
    }
    /// `StaticDtor_Detach_00afe560`: atexit destructor: stores the vtable `0x00b1eb90` in `0x00c158b0`; when the word at `0x00c158bc` is set, `0x004cc050`(`0x00c158b0`) (cdecl) and the entry [`0x00c158c0`] of the table `0x00ca43b0` is cleared; that index.
    StaticDtor_Detach_00afe560 @ 0x00af_e560 => extern "C" fn s_00afe560() -> u32 {
        si::cond_detach(&mut mem(), &si::COND_DETACHES[6])
    }
    /// `StaticDtor_Detach_00afe5a0`: atexit destructor: stores the vtable `0x00b1eb90` in `0x00c158c4`; when the word at `0x00c158d0` is set, `0x004cc050`(`0x00c158c4`) (cdecl) and the entry [`0x00c158d4`] of the table `0x00ca43b0` is cleared; that index.
    StaticDtor_Detach_00afe5a0 @ 0x00af_e5a0 => extern "C" fn s_00afe5a0() -> u32 {
        si::cond_detach(&mut mem(), &si::COND_DETACHES[7])
    }
    /// `StaticDtor_SubCount_00afe740`: atexit destructor: stores the vtable `0x00b1f7ac` in `0x00c16108`, `0x00432760`(`0x00c16108`) (cdecl), then takes the word at `0x00c16118` off the total at `0x00ca4e20`; that word.
    StaticDtor_SubCount_00afe740 @ 0x00af_e740 => extern "C" fn s_00afe740() -> u32 {
        si::sub_count(&mut mem(), &si::SUB_COUNTS[0])
    }
    /// `StaticDtor_SubCount_00afe770`: atexit destructor: stores the vtable `0x00b1f7ac` in `0x00c160f0`, `0x00432760`(`0x00c160f0`) (cdecl), then takes the word at `0x00c16100` off the total at `0x00ca4e20`; that word.
    StaticDtor_SubCount_00afe770 @ 0x00af_e770 => extern "C" fn s_00afe770() -> u32 {
        si::sub_count(&mut mem(), &si::SUB_COUNTS[1])
    }
    /// `StaticDtor_SubCount_00afe7a0`: atexit destructor: stores the vtable `0x00b1f7ac` in `0x00ca4ec4`, `0x00432760`(`0x00ca4ec4`) (cdecl), then takes the word at `0x00ca4ed4` off the total at `0x00ca4e20`; that word.
    StaticDtor_SubCount_00afe7a0 @ 0x00af_e7a0 => extern "C" fn s_00afe7a0() -> u32 {
        si::sub_count(&mut mem(), &si::SUB_COUNTS[2])
    }
    /// `StaticDtor_SubCount_00afe7d0`: atexit destructor: stores the vtable `0x00b1f7ac` in `0x00c16150`, `0x00432760`(`0x00c16150`) (cdecl), then takes the word at `0x00c16160` off the total at `0x00ca4e20`; that word.
    StaticDtor_SubCount_00afe7d0 @ 0x00af_e7d0 => extern "C" fn s_00afe7d0() -> u32 {
        si::sub_count(&mut mem(), &si::SUB_COUNTS[3])
    }
    /// `StaticDtor_SubCount_00afe800`: atexit destructor: stores the vtable `0x00b1f7ac` in `0x00c16120`, `0x00432760`(`0x00c16120`) (cdecl), then takes the word at `0x00c16130` off the total at `0x00ca4e20`; that word.
    StaticDtor_SubCount_00afe800 @ 0x00af_e800 => extern "C" fn s_00afe800() -> u32 {
        si::sub_count(&mut mem(), &si::SUB_COUNTS[4])
    }
    /// `StaticDtor_SubCount_00afe830`: atexit destructor: stores the vtable `0x00b1f7ac` in `0x00c16138`, `0x00432760`(`0x00c16138`) (cdecl), then takes the word at `0x00c16148` off the total at `0x00ca4e20`; that word.
    StaticDtor_SubCount_00afe830 @ 0x00af_e830 => extern "C" fn s_00afe830() -> u32 {
        si::sub_count(&mut mem(), &si::SUB_COUNTS[5])
    }
    /// `StaticDtor_FreeOwned_00afe870`: atexit destructor: when bit 31 of the word at `0x00ca4ebc` is clear, `Phyre_PhyreFree(the block at 0x00ca4ec0)` unless it is 0 (cdecl); both words cleared.
    StaticDtor_FreeOwned_00afe870 @ 0x00af_e870 => extern "C" fn s_00afe870() -> u32 {
        si::free_if_owned(&mut mem(), &si::FREE_IF_OWNED[0])
    }
    /// `StaticDtor_Owned_00aff8a0`: atexit destructor: thiscalls `0x0045c1d0` on `0x00ca852c` and `0x004fd160` on `0x00ca8524`; when bit 31 of `0x00ca8510` is clear, `0x004ab4d0`([`0x00ca8514`], the word & `0x7fffffff`); both words cleared.
    StaticDtor_Owned_00aff8a0 @ 0x00af_f8a0 => extern "C" fn s_00aff8a0() -> u32 {
        si::dtor_00aff8a0(&mut mem())
    }
    /// `StaticDtor_EhVector_00aff930`: atexit destructor: `eh_vector_destructor_iterator(0x00ca8330, 0x10, 0x1e, 0x004a1770)`; its eax.
    StaticDtor_EhVector_00aff930 @ 0x00af_f930 => extern "C" fn s_00aff930() -> u32 {
        si::dtor_00aff930(&mut mem())
    }
    /// `StaticDtor_ChainUnlink_00affae0`: atexit destructor: stores the vtable `0x00b0e078` in `0x00c1a300` and unlinks it from the chain at `0x00c901a8` (its link takes the word at `0x00c1a304`, which is cleared); the link address.
    StaticDtor_ChainUnlink_00affae0 @ 0x00af_fae0 => extern "C" fn s_00affae0() -> u32 {
        si::chain_unlink(&mut mem(), &si::CHAIN_UNLINKS[6])
    }
    /// `StaticDtor_ChainUnlink_00affb30`: atexit destructor: stores the vtable `0x00b0e078` in `0x00c1a350` and unlinks it from the chain at `0x00c901a8` (its link takes the word at `0x00c1a354`, which is cleared); the link address.
    StaticDtor_ChainUnlink_00affb30 @ 0x00af_fb30 => extern "C" fn s_00affb30() -> u32 {
        si::chain_unlink(&mut mem(), &si::CHAIN_UNLINKS[7])
    }
    /// `StaticDtor_FreeOwned_00b00fd0`: atexit destructor: when bit 31 of the word at `0x00cacedc` is clear, `Phyre_PhyreFree(the block at 0x00cacee0)` unless it is 0 (cdecl); both words cleared.
    StaticDtor_FreeOwned_00b00fd0 @ 0x00b0_0fd0 => extern "C" fn s_00b00fd0() -> u32 {
        si::free_if_owned(&mut mem(), &si::FREE_IF_OWNED[1])
    }
    /// `StaticDtor_Seq_00b035b0`: atexit destructor: thiscalls `Phyre_PRedBlackTree_orphanAllContents` on `0x00c24f0c`, `Phyre_PFreeList_reset` on `0x00c24f24`, `Phyre_PFreeList_dtor_PFreeList` on `0x00c24f24`, `Phyre_PRedBlackTree_clear` on `0x00c24f0c`, `Phyre_PRedBlackTree_clear` on `0x00c24f0c`; the last one's eax.
    StaticDtor_Seq_00b035b0 @ 0x00b0_35b0 => extern "C" fn s_00b035b0() -> u32 {
        si::thiscall_seq(&mut mem(), &si::THISCALL_SEQS[2])
    }
    /// `StaticDtor_Seq_00b03690`: atexit destructor: thiscalls `Phyre_PRedBlackTree_orphanAllContents` on `0x00c25374`, `Phyre_PFreeList_reset` on `0x00c2538c`, `Phyre_PFreeList_dtor_PFreeList` on `0x00c2538c`, `Phyre_PRedBlackTree_clear` on `0x00c25374`, `Phyre_PRedBlackTree_clear` on `0x00c25374`; the last one's eax.
    StaticDtor_Seq_00b03690 @ 0x00b0_3690 => extern "C" fn s_00b03690() -> u32 {
        si::thiscall_seq(&mut mem(), &si::THISCALL_SEQS[3])
    }
    /// `StaticDtor_FreeOwned_00b04b40`: atexit destructor: when bit 31 of the word at `0x00cbe6d8` is clear, `Phyre_PhyreFree(the block at 0x00cbe6dc)` unless it is 0 (cdecl); both words cleared.
    StaticDtor_FreeOwned_00b04b40 @ 0x00b0_4b40 => extern "C" fn s_00b04b40() -> u32 {
        si::free_if_owned(&mut mem(), &si::FREE_IF_OWNED[2])
    }
    /// `StaticDtor_ChainUnlink_00b04b80`: atexit destructor: stores the vtable `0x00b0e078` in `0x00c2a054` and unlinks it from the chain at `0x00c901a8` (its link takes the word at `0x00c2a058`, which is cleared); the link address.
    StaticDtor_ChainUnlink_00b04b80 @ 0x00b0_4b80 => extern "C" fn s_00b04b80() -> u32 {
        si::chain_unlink(&mut mem(), &si::CHAIN_UNLINKS[8])
    }
    /// `StaticDtor_EhVector_00b050d0`: atexit destructor: frees the owned block `0x00cc1430` (flags `0x00cc142c`), `eh_vector_destructor_iterator(0x00cc13ec, 0x20, 2, 0x005e1e80)`, the vtable `0x00b3c60c` at `0x00cc13e8`; the iterator's eax.
    StaticDtor_EhVector_00b050d0 @ 0x00b0_50d0 => extern "C" fn s_00b050d0() -> u32 {
        si::dtor_00b050d0(&mut mem())
    }
    /// `StaticDtor_ChainUnlink_00b05ef0`: atexit destructor: stores the vtable `0x00b0e078` in `0x00c2a0f4` and unlinks it from the chain at `0x00c901a8` (its link takes the word at `0x00c2a0f8`, which is cleared); the link address.
    StaticDtor_ChainUnlink_00b05ef0 @ 0x00b0_5ef0 => extern "C" fn s_00b05ef0() -> u32 {
        si::chain_unlink(&mut mem(), &si::CHAIN_UNLINKS[9])
    }
    /// `StaticDtor_FreeOwned_00b05f50`: atexit destructor: when bit 31 of the word at `0x00cc96e0` is clear, `Phyre_PhyreFree(the block at 0x00cc96e4)` unless it is 0 (cdecl); both words cleared.
    StaticDtor_FreeOwned_00b05f50 @ 0x00b0_5f50 => extern "C" fn s_00b05f50() -> u32 {
        si::free_if_owned(&mut mem(), &si::FREE_IF_OWNED[3])
    }
    /// `StaticDtor_FreeOwned_00b06130`: atexit destructor: when bit 31 of the word at `0x00cc9d14` is clear, `Phyre_PhyreFree(the block at 0x00cc9d18)` unless it is 0 (cdecl); both words cleared.
    StaticDtor_FreeOwned_00b06130 @ 0x00b0_6130 => extern "C" fn s_00b06130() -> u32 {
        si::free_if_owned(&mut mem(), &si::FREE_IF_OWNED[4])
    }
    /// `StaticDtor_Seq_00b06630`: atexit destructor: thiscalls `Phyre_PRedBlackTree_orphanAllContents` on `0x00c31e9c`, `Phyre_PFreeList_reset` on `0x00c31eb4`, `Phyre_PFreeList_dtor_PFreeList` on `0x00c31eb4`, `Phyre_PRedBlackTree_clear` on `0x00c31e9c`, `Phyre_PRedBlackTree_clear` on `0x00c31e9c`; the last one's eax.
    StaticDtor_Seq_00b06630 @ 0x00b0_6630 => extern "C" fn s_00b06630() -> u32 {
        si::thiscall_seq(&mut mem(), &si::THISCALL_SEQS[4])
    }
    /// `StaticDtor_Shutdown_00b066e0`: atexit destructor: `0x00689830`(), `0x0068bba0`(), `0x00a259f0`(), then `user_free` the word at `0x00ccb214` when it is not 0.
    StaticDtor_Shutdown_00b066e0 @ 0x00b0_66e0 => extern "C" fn s_00b066e0() -> u32 {
        si::dtor_00b066e0(&mut mem())
    }
    /// `StaticDtor_FreeOwned_00b067b0`: atexit destructor: when bit 31 of the word at `0x00ccb6d4` is clear, `Phyre_PhyreFree(the block at 0x00ccb6d8)` (cdecl); both words cleared.
    StaticDtor_FreeOwned_00b067b0 @ 0x00b0_67b0 => extern "C" fn s_00b067b0() -> u32 {
        si::free_if_owned(&mut mem(), &si::FREE_IF_OWNED[5])
    }
    /// `StaticDtor_FreeNonzero_00b067f0`: atexit destructor: when the word at `0x00ccb6e8` is not 0, `user_free_thunk(it)` (cdecl) and its eax; else 0.
    StaticDtor_FreeNonzero_00b067f0 @ 0x00b0_67f0 => extern "C" fn s_00b067f0() -> u32 {
        si::free_if_nonzero(&mut mem(), &si::FREE_IF_NONZERO[0])
    }
    /// `StaticDtor_Seq_00b07000`: atexit destructor: thiscalls `Phyre_PRedBlackTree_orphanAllContents` on `0x00c3419c`, `Phyre_PFreeList_reset` on `0x00c341b4`, `Phyre_PFreeList_dtor_PFreeList` on `0x00c341b4`, `Phyre_PRedBlackTree_clear` on `0x00c3419c`, `Phyre_PRedBlackTree_clear` on `0x00c3419c`; the last one's eax.
    StaticDtor_Seq_00b07000 @ 0x00b0_7000 => extern "C" fn s_00b07000() -> u32 {
        si::thiscall_seq(&mut mem(), &si::THISCALL_SEQS[5])
    }
    /// `StaticDtor_Tree_00b06050`: atexit destructor: erases the tree at `0x00cc9ee4` (result iterator in a frame word) and deletes its head node; `operator_delete`'s eax.
    StaticDtor_Tree_00b06050 @ 0x00b0_6050 => extern "C" fn s_00b06050() -> u32 {
        let mut f = StackFrame::<2>::new();
        si::dtor_00b06050(&mut mem(), f.at())
    }
    /// `StaticDtor_StringList_00b06cb0`: atexit destructor: `StdStringList_Clear` on `0x00ccc840`, then deletes its head node; `operator_delete`'s eax.
    StaticDtor_StringList_00b06cb0 @ 0x00b0_6cb0 => extern "C" fn s_00b06cb0() -> u32 {
        si::dtor_00b06cb0(&mut mem())
    }
}
