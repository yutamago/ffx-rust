//! The menu plates: `TODrawMenuPlateXYWHType` (`0x008f5f70`), the framed
//! backgrounds of the menus and windows.
//!
//! A plate of type 1-14 is a stretched sheet image (shape -4, three
//! `TOMkpShapeXYWHUVC2` pieces: two caps and the middle) or a single
//! shape (`TOMkpShapeXYWHUV`), framed by `TODrawCrossBoxXYWHC2` corner and
//! edge pieces, with a few types adding an ability-map panel
//! (`graphicGetAbmap`, `0x008f4df0`) or the `0x008f9c20` frame. Every size
//! and offset goes through the HD UI remap (`graphicUiRemapX2`/`Y2`).
//!
//! The function is ten kilobytes of straight-line x87 code: each type's
//! arm below follows its instructions one for one (`f64` for the x87
//! registers at the game's 53-bit precision, `f32` where the original
//! stores a dword, constants read from the image), so the pieces get the
//! original's exact coordinates. The draw calls themselves stay with their
//! callers' packet code.

/// The bridges over game memory.
pub mod bridge {
    use ffx_bridge::Mem;

    /// An f32 constant of the image.
    fn image_f32<M: Mem>(mem: &M, a: u32) -> f32 {
        mem.read(mem.rebase(a))
    }

    /// An f64 constant of the image.
    fn img64<M: Mem>(mem: &M, a: u32) -> f64 {
        mem.read(mem.rebase(a))
    }

    /// `TODrawMenuPlateXYWHType(x, y, w, h, kind)` (`0x008f5f70`): draws
    /// the menu plate `kind` (1-14) over the box `x, y, w, h` (f32 bits).
    /// Remaps go through `remap(y_axis, v)` (`graphicUiRemapX2`/`Y2`),
    /// the shape, frame and panel calls through `call`.
    #[allow(
        clippy::too_many_lines,
        clippy::cast_possible_truncation,
        clippy::similar_names,
        clippy::many_single_char_names,
        clippy::unreadable_literal
    )]
    pub fn to_draw_menu_plate_xywh_type<M: Mem>(
        mem: &mut M,
        args: [u32; 5],
        mut remap: impl FnMut(&mut M, bool, f32) -> f32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let [x, y, w, h, kind] = args;
        let (x, y, w, h) = (
            f32::from_bits(x),
            f32::from_bits(y),
            f32::from_bits(w),
            f32::from_bits(h),
        );
        match kind {
            1 => {
                let x1: f64 = img64(mem, 0x00b4_1d58) * f64::from(h);
                let x2: f64 = x1 / img64(mem, 0x00b4_1d30);
                let v3 = x2 as f32;
                let v4 = image_f32(mem, 0x00b4_bf08);
                let v5 = 0.0f32;
                let v6 = 0.0f32;
                let v7 = image_f32(mem, 0x00b4_bf08);
                let r8 = remap(mem, false, v7);
                call(
                    mem,
                    0x0090_3ee0,
                    &[
                        0xffff_fffc,
                        x.to_bits(),
                        y.to_bits(),
                        r8.to_bits(),
                        h.to_bits(),
                        v6.to_bits(),
                        v5.to_bits(),
                        v4.to_bits(),
                        v3.to_bits(),
                        0x0080_8080,
                        0x8080_8080,
                    ],
                );
                let x9: f64 = img64(mem, 0x00b4_1d68) * f64::from(w);
                let x10: f64 = x9 * img64(mem, 0x00b4_1cc0);
                let x11: f64 = x10 - img64(mem, 0x00b5_f270);
                let v12 = x11 as f32;
                let v13 = 0.0f32;
                let v14 = image_f32(mem, 0x00b4_bf08);
                let d15: f64 = f64::from(w);
                let v16 = image_f32(mem, 0x00b6_5c0c);
                let r17 = remap(mem, false, v16);
                let x18: f64 = d15 - f64::from(r17);
                let v19 = x18 as f32;
                let v20 = image_f32(mem, 0x00b4_bf08);
                let r21 = remap(mem, false, v20);
                let x22: f64 = f64::from(r21) + f64::from(x);
                let v23 = x22 as f32;
                call(
                    mem,
                    0x0090_3ee0,
                    &[
                        0xffff_fffc,
                        v23.to_bits(),
                        y.to_bits(),
                        v19.to_bits(),
                        h.to_bits(),
                        v14.to_bits(),
                        v13.to_bits(),
                        v12.to_bits(),
                        v3.to_bits(),
                        0x8080_8080,
                        0x8080_8080,
                    ],
                );
                let x24: f64 = f64::from(x) + f64::from(w);
                let v25 = x10 as f32;
                let v26 = 0.0f32;
                let v27 = image_f32(mem, 0x00b4_bf08);
                let r28 = remap(mem, false, v27);
                let v29 = image_f32(mem, 0x00b4_bf08);
                let r30 = remap(mem, false, v29);
                let x31: f64 = x24 - f64::from(r30);
                let v32 = x31 as f32;
                call(
                    mem,
                    0x0090_3ee0,
                    &[
                        0xffff_fffc,
                        v32.to_bits(),
                        y.to_bits(),
                        r28.to_bits(),
                        h.to_bits(),
                        v12.to_bits(),
                        v26.to_bits(),
                        v25.to_bits(),
                        v3.to_bits(),
                        0x8080_8080,
                        0x0080_8080,
                    ],
                );
                let x33: f64 = f64::from(y) + f64::from(h);
                let v34 = x33 as f32;
                let v35 = image_f32(mem, 0x00b9_2308);
                let r36 = remap(mem, true, v35);
                let v37 = image_f32(mem, 0x00b4_bf08);
                let r38 = remap(mem, false, v37);
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        x.to_bits(),
                        v34.to_bits(),
                        r38.to_bits(),
                        r36.to_bits(),
                        0x0000_0000,
                        0x2600_0000,
                    ],
                );
                let v39 = image_f32(mem, 0x00b9_2308);
                let r40 = remap(mem, true, v39);
                let d41: f64 = f64::from(w);
                let v42 = image_f32(mem, 0x00b6_5c0c);
                let r43 = remap(mem, false, v42);
                let x44: f64 = d41 - f64::from(r43);
                let v45 = x44 as f32;
                let v46 = image_f32(mem, 0x00b4_bf08);
                let r47 = remap(mem, false, v46);
                let x48: f64 = f64::from(r47) + f64::from(x);
                let v49 = x48 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        v49.to_bits(),
                        v34.to_bits(),
                        v45.to_bits(),
                        r40.to_bits(),
                        0x2600_0000,
                        0x2600_0000,
                    ],
                );
                let v50 = image_f32(mem, 0x00b9_2308);
                let r51 = remap(mem, true, v50);
                let v52 = image_f32(mem, 0x00b4_bf08);
                let r53 = remap(mem, false, v52);
                let v54 = image_f32(mem, 0x00b4_bf08);
                let r55 = remap(mem, false, v54);
                let x56: f64 = x24 - f64::from(r55);
                let v57 = x56 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        v57.to_bits(),
                        v34.to_bits(),
                        r53.to_bits(),
                        r51.to_bits(),
                        0x2600_0000,
                        0x0000_0000,
                    ],
                );
                let v58 = image_f32(mem, 0x00b4_bf08);
                let r59 = remap(mem, false, v58);
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        x.to_bits(),
                        y.to_bits(),
                        r59.to_bits(),
                        h.to_bits(),
                        0x0000_0000,
                        0x5a00_0000,
                    ],
                );
                let d60: f64 = f64::from(w);
                let v61 = image_f32(mem, 0x00b6_5c0c);
                let r62 = remap(mem, false, v61);
                let x63: f64 = d60 - f64::from(r62);
                let v64 = x63 as f32;
                let v65 = image_f32(mem, 0x00b4_bf08);
                let r66 = remap(mem, false, v65);
                let x67: f64 = f64::from(r66) + f64::from(x);
                let v68 = x67 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        v68.to_bits(),
                        y.to_bits(),
                        v64.to_bits(),
                        h.to_bits(),
                        0x5a00_0000,
                        0x5a00_0000,
                    ],
                );
                let v69 = image_f32(mem, 0x00b4_bf08);
                let r70 = remap(mem, false, v69);
                let v71 = image_f32(mem, 0x00b4_bf08);
                let r72 = remap(mem, false, v71);
                let x73: f64 = x24 - f64::from(r72);
                let v74 = x73 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        v74.to_bits(),
                        y.to_bits(),
                        r70.to_bits(),
                        h.to_bits(),
                        0x5a00_0000,
                        0x0000_0000,
                    ],
                );
                let v75 = image_f32(mem, 0x00b9_2268);
                let r76 = remap(mem, true, v75);
                let v77 = image_f32(mem, 0x00b4_bf08);
                let r78 = remap(mem, false, v77);
                let v79 = 1.0f32;
                let r80 = remap(mem, true, v79);
                let x81: f64 = f64::from(r80) + f64::from(y);
                let v82 = x81 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        x.to_bits(),
                        v82.to_bits(),
                        r78.to_bits(),
                        r76.to_bits(),
                        0x00ff_ffff,
                        0x5aff_ffff,
                    ],
                );
                let v83 = image_f32(mem, 0x00b9_2268);
                let r84 = remap(mem, true, v83);
                let d85: f64 = f64::from(w);
                let v86 = image_f32(mem, 0x00b6_5c0c);
                let r87 = remap(mem, false, v86);
                let x88: f64 = d85 - f64::from(r87);
                let v89 = x88 as f32;
                let v90 = 1.0f32;
                let r91 = remap(mem, true, v90);
                let x92: f64 = f64::from(r91) + f64::from(y);
                let v93 = x92 as f32;
                let v94 = image_f32(mem, 0x00b4_bf08);
                let r95 = remap(mem, false, v94);
                let x96: f64 = f64::from(r95) + f64::from(x);
                let v97 = x96 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        v97.to_bits(),
                        v93.to_bits(),
                        v89.to_bits(),
                        r84.to_bits(),
                        0x5aff_ffff,
                        0x5aff_ffff,
                    ],
                );
                let v98 = image_f32(mem, 0x00b9_2268);
                let r99 = remap(mem, true, v98);
                let v100 = image_f32(mem, 0x00b4_bf08);
                let r101 = remap(mem, false, v100);
                let v102 = 1.0f32;
                let r103 = remap(mem, true, v102);
                let x104: f64 = f64::from(r103) + f64::from(y);
                let v105 = x104 as f32;
                let v106 = image_f32(mem, 0x00b4_bf08);
                let r107 = remap(mem, false, v106);
                let x108: f64 = x24 - f64::from(r107);
                let v109 = x108 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        v109.to_bits(),
                        v105.to_bits(),
                        r101.to_bits(),
                        r99.to_bits(),
                        0x5aff_ffff,
                        0x00ff_ffff,
                    ],
                );
                let v110 = image_f32(mem, 0x00b3_eb94);
                let r111 = remap(mem, true, v110);
                let v112 = image_f32(mem, 0x00b4_bf08);
                let r113 = remap(mem, false, v112);
                let v114 = image_f32(mem, 0x00b3_eb94);
                let r115 = remap(mem, true, v114);
                let x116: f64 = x33 - f64::from(r115);
                let v117 = x116 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        x.to_bits(),
                        v117.to_bits(),
                        r113.to_bits(),
                        r111.to_bits(),
                        0x0000_0000,
                        0x8000_0000,
                    ],
                );
                let v118 = image_f32(mem, 0x00b3_eb94);
                let r119 = remap(mem, true, v118);
                let d120: f64 = f64::from(w);
                let v121 = image_f32(mem, 0x00b6_5c0c);
                let r122 = remap(mem, false, v121);
                let x123: f64 = d120 - f64::from(r122);
                let v124 = x123 as f32;
                let v125 = image_f32(mem, 0x00b3_eb94);
                let r126 = remap(mem, true, v125);
                let x127: f64 = x33 - f64::from(r126);
                let v128 = x127 as f32;
                let v129 = image_f32(mem, 0x00b4_bf08);
                let r130 = remap(mem, false, v129);
                let x131: f64 = f64::from(r130) + f64::from(x);
                let v132 = x131 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        v132.to_bits(),
                        v128.to_bits(),
                        v124.to_bits(),
                        r119.to_bits(),
                        0x8000_0000,
                        0x8000_0000,
                    ],
                );
                let v133 = image_f32(mem, 0x00b3_eb94);
                let r134 = remap(mem, true, v133);
                let v135 = image_f32(mem, 0x00b4_bf08);
                let r136 = remap(mem, false, v135);
                let v137 = image_f32(mem, 0x00b3_eb94);
                let r138 = remap(mem, true, v137);
                let x139: f64 = x33 - f64::from(r138);
                let v140 = x139 as f32;
                let v141 = image_f32(mem, 0x00b4_bf08);
                let r142 = remap(mem, false, v141);
                let x143: f64 = x24 - f64::from(r142);
                let v144 = x143 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        v144.to_bits(),
                        v140.to_bits(),
                        r136.to_bits(),
                        r134.to_bits(),
                        0x8000_0000,
                        0x0000_0000,
                    ],
                );
            }
            2 => {
                let x145: f64 = img64(mem, 0x00b4_1d58) * f64::from(h);
                let x146: f64 = x145 / img64(mem, 0x00b4_1d30);
                let v147 = x146 as f32;
                let v148 = image_f32(mem, 0x00b4_1d8c);
                let v149 = 0.0f32;
                let v150 = 0.0f32;
                let v151 = image_f32(mem, 0x00b4_1d8c);
                let r152 = remap(mem, false, v151);
                call(
                    mem,
                    0x0090_3ee0,
                    &[
                        0xffff_fffc,
                        x.to_bits(),
                        y.to_bits(),
                        r152.to_bits(),
                        h.to_bits(),
                        v150.to_bits(),
                        v149.to_bits(),
                        v148.to_bits(),
                        v147.to_bits(),
                        0x0080_8080,
                        0x8080_8080,
                    ],
                );
                let x153: f64 = img64(mem, 0x00b4_1d68) * f64::from(w);
                let x154: f64 = x153 * img64(mem, 0x00b4_1cc0);
                let x155: f64 = x154 - img64(mem, 0x00b5_4d60);
                let v156 = x155 as f32;
                let v157 = 0.0f32;
                let v158 = image_f32(mem, 0x00b4_1d8c);
                let d159: f64 = f64::from(w);
                let v160 = image_f32(mem, 0x00b5_f384);
                let r161 = remap(mem, false, v160);
                let x162: f64 = d159 - f64::from(r161);
                let v163 = x162 as f32;
                let v164 = image_f32(mem, 0x00b4_1d8c);
                let r165 = remap(mem, false, v164);
                let x166: f64 = f64::from(r165) + f64::from(x);
                let v167 = x166 as f32;
                call(
                    mem,
                    0x0090_3ee0,
                    &[
                        0xffff_fffc,
                        v167.to_bits(),
                        y.to_bits(),
                        v163.to_bits(),
                        h.to_bits(),
                        v158.to_bits(),
                        v157.to_bits(),
                        v156.to_bits(),
                        v147.to_bits(),
                        0x8080_8080,
                        0x8080_8080,
                    ],
                );
                let x168: f64 = f64::from(x) + f64::from(w);
                let v169 = x154 as f32;
                let v170 = 0.0f32;
                let v171 = image_f32(mem, 0x00b4_1d8c);
                let r172 = remap(mem, false, v171);
                let v173 = image_f32(mem, 0x00b4_1d8c);
                let r174 = remap(mem, false, v173);
                let x175: f64 = x168 - f64::from(r174);
                let v176 = x175 as f32;
                call(
                    mem,
                    0x0090_3ee0,
                    &[
                        0xffff_fffc,
                        v176.to_bits(),
                        y.to_bits(),
                        r172.to_bits(),
                        h.to_bits(),
                        v156.to_bits(),
                        v170.to_bits(),
                        v169.to_bits(),
                        v147.to_bits(),
                        0x8080_8080,
                        0x0080_8080,
                    ],
                );
                let x177: f64 = f64::from(y) + f64::from(h);
                let v178 = x177 as f32;
                let v179 = image_f32(mem, 0x00b9_2308);
                let r180 = remap(mem, true, v179);
                let v181 = image_f32(mem, 0x00b4_1d8c);
                let r182 = remap(mem, false, v181);
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        x.to_bits(),
                        v178.to_bits(),
                        r182.to_bits(),
                        r180.to_bits(),
                        0x0000_0000,
                        0x2600_0000,
                    ],
                );
                let v183 = image_f32(mem, 0x00b9_2308);
                let r184 = remap(mem, true, v183);
                let d185: f64 = f64::from(w);
                let v186 = image_f32(mem, 0x00b5_f384);
                let r187 = remap(mem, false, v186);
                let x188: f64 = d185 - f64::from(r187);
                let v189 = x188 as f32;
                let v190 = image_f32(mem, 0x00b4_1d8c);
                let r191 = remap(mem, false, v190);
                let x192: f64 = f64::from(r191) + f64::from(x);
                let v193 = x192 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        v193.to_bits(),
                        v178.to_bits(),
                        v189.to_bits(),
                        r184.to_bits(),
                        0x2600_0000,
                        0x2600_0000,
                    ],
                );
                let v194 = image_f32(mem, 0x00b9_2308);
                let r195 = remap(mem, true, v194);
                let v196 = image_f32(mem, 0x00b4_1d8c);
                let r197 = remap(mem, false, v196);
                let v198 = image_f32(mem, 0x00b4_1d8c);
                let r199 = remap(mem, false, v198);
                let x200: f64 = x168 - f64::from(r199);
                let v201 = x200 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        v201.to_bits(),
                        v178.to_bits(),
                        r197.to_bits(),
                        r195.to_bits(),
                        0x2600_0000,
                        0x0000_0000,
                    ],
                );
                let v202 = image_f32(mem, 0x00b4_1d8c);
                let r203 = remap(mem, false, v202);
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        x.to_bits(),
                        y.to_bits(),
                        r203.to_bits(),
                        h.to_bits(),
                        0x0000_0000,
                        0x6600_0000,
                    ],
                );
                let d204: f64 = f64::from(w);
                let v205 = image_f32(mem, 0x00b5_f384);
                let r206 = remap(mem, false, v205);
                let x207: f64 = d204 - f64::from(r206);
                let v208 = x207 as f32;
                let v209 = image_f32(mem, 0x00b4_1d8c);
                let r210 = remap(mem, false, v209);
                let x211: f64 = f64::from(r210) + f64::from(x);
                let v212 = x211 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        v212.to_bits(),
                        y.to_bits(),
                        v208.to_bits(),
                        h.to_bits(),
                        0x6600_0000,
                        0x6600_0000,
                    ],
                );
                let v213 = image_f32(mem, 0x00b4_1d8c);
                let r214 = remap(mem, false, v213);
                let v215 = image_f32(mem, 0x00b4_1d8c);
                let r216 = remap(mem, false, v215);
                let x217: f64 = x168 - f64::from(r216);
                let v218 = x217 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        v218.to_bits(),
                        y.to_bits(),
                        r214.to_bits(),
                        h.to_bits(),
                        0x6600_0000,
                        0x0000_0000,
                    ],
                );
                let v219 = image_f32(mem, 0x00b9_2268);
                let r220 = remap(mem, true, v219);
                let v221 = image_f32(mem, 0x00b4_1d8c);
                let r222 = remap(mem, false, v221);
                let v223 = 1.0f32;
                let r224 = remap(mem, true, v223);
                let x225: f64 = f64::from(r224) + f64::from(y);
                let v226 = x225 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        x.to_bits(),
                        v226.to_bits(),
                        r222.to_bits(),
                        r220.to_bits(),
                        0x00ff_ffff,
                        0x5aff_ffff,
                    ],
                );
                let v227 = image_f32(mem, 0x00b9_2268);
                let r228 = remap(mem, true, v227);
                let d229: f64 = f64::from(w);
                let v230 = image_f32(mem, 0x00b5_f384);
                let r231 = remap(mem, false, v230);
                let x232: f64 = d229 - f64::from(r231);
                let v233 = x232 as f32;
                let v234 = 1.0f32;
                let r235 = remap(mem, true, v234);
                let x236: f64 = f64::from(r235) + f64::from(y);
                let v237 = x236 as f32;
                let v238 = image_f32(mem, 0x00b4_1d8c);
                let r239 = remap(mem, false, v238);
                let x240: f64 = f64::from(r239) + f64::from(x);
                let v241 = x240 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        v241.to_bits(),
                        v237.to_bits(),
                        v233.to_bits(),
                        r228.to_bits(),
                        0x5aff_ffff,
                        0x5aff_ffff,
                    ],
                );
                let v242 = image_f32(mem, 0x00b9_2268);
                let r243 = remap(mem, true, v242);
                let v244 = image_f32(mem, 0x00b4_1d8c);
                let r245 = remap(mem, false, v244);
                let v246 = 1.0f32;
                let r247 = remap(mem, true, v246);
                let x248: f64 = f64::from(r247) + f64::from(y);
                let v249 = x248 as f32;
                let v250 = image_f32(mem, 0x00b4_1d8c);
                let r251 = remap(mem, false, v250);
                let x252: f64 = x168 - f64::from(r251);
                let v253 = x252 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        v253.to_bits(),
                        v249.to_bits(),
                        r245.to_bits(),
                        r243.to_bits(),
                        0x5aff_ffff,
                        0x00ff_ffff,
                    ],
                );
                let v254 = image_f32(mem, 0x00b3_eb94);
                let r255 = remap(mem, true, v254);
                let v256 = image_f32(mem, 0x00b4_1d8c);
                let r257 = remap(mem, false, v256);
                let v258 = image_f32(mem, 0x00b3_eb94);
                let r259 = remap(mem, true, v258);
                let x260: f64 = x177 - f64::from(r259);
                let v261 = x260 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        x.to_bits(),
                        v261.to_bits(),
                        r257.to_bits(),
                        r255.to_bits(),
                        0x0000_0000,
                        0x8000_0000,
                    ],
                );
                let v262 = image_f32(mem, 0x00b3_eb94);
                let r263 = remap(mem, true, v262);
                let d264: f64 = f64::from(w);
                let v265 = image_f32(mem, 0x00b5_f384);
                let r266 = remap(mem, false, v265);
                let x267: f64 = d264 - f64::from(r266);
                let v268 = x267 as f32;
                let v269 = image_f32(mem, 0x00b3_eb94);
                let r270 = remap(mem, true, v269);
                let x271: f64 = x177 - f64::from(r270);
                let v272 = x271 as f32;
                let v273 = image_f32(mem, 0x00b4_1d8c);
                let r274 = remap(mem, false, v273);
                let x275: f64 = f64::from(r274) + f64::from(x);
                let v276 = x275 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        v276.to_bits(),
                        v272.to_bits(),
                        v268.to_bits(),
                        r263.to_bits(),
                        0x8000_0000,
                        0x8000_0000,
                    ],
                );
                let v277 = image_f32(mem, 0x00b3_eb94);
                let r278 = remap(mem, true, v277);
                let v279 = image_f32(mem, 0x00b4_1d8c);
                let r280 = remap(mem, false, v279);
                let v281 = image_f32(mem, 0x00b3_eb94);
                let r282 = remap(mem, true, v281);
                let x283: f64 = x177 - f64::from(r282);
                let v284 = x283 as f32;
                let v285 = image_f32(mem, 0x00b4_1d8c);
                let r286 = remap(mem, false, v285);
                let x287: f64 = x168 - f64::from(r286);
                let v288 = x287 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        v288.to_bits(),
                        v284.to_bits(),
                        r280.to_bits(),
                        r278.to_bits(),
                        0x8000_0000,
                        0x0000_0000,
                    ],
                );
            }
            3 => {
                let v289 = image_f32(mem, 0x00b9_2308);
                let r290 = remap(mem, true, v289);
                let x291: f64 = f64::from(r290) + f64::from(y);
                let v292 = x291 as f32;
                let v293 = image_f32(mem, 0x00b9_2308);
                let r294 = remap(mem, false, v293);
                let x295: f64 = f64::from(r294) + f64::from(x);
                let v296 = x295 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        v296.to_bits(),
                        v292.to_bits(),
                        w.to_bits(),
                        h.to_bits(),
                        0x2400_0000,
                        0x2400_0000,
                    ],
                );
                call(
                    mem,
                    0x008f_9c20,
                    &[
                        x.to_bits(),
                        y.to_bits(),
                        w.to_bits(),
                        h.to_bits(),
                        0x0000_0000,
                    ],
                );
                let v297 = image_f32(mem, 0x00b6_561c);
                let v298 = image_f32(mem, 0x00b6_6bec);
                let v299 = image_f32(mem, 0x00b6_5610);
                let v300 = image_f32(mem, 0x00b6_6be8);
                let d301: f64 = f64::from(h);
                let v302 = image_f32(mem, 0x00b9_2308);
                let r303 = remap(mem, true, v302);
                let x304: f64 = d301 - f64::from(r303);
                let v305 = x304 as f32;
                let d306: f64 = f64::from(w);
                let v307 = image_f32(mem, 0x00b9_2308);
                let r308 = remap(mem, false, v307);
                let x309: f64 = d306 - f64::from(r308);
                let v310 = x309 as f32;
                let v311 = image_f32(mem, 0x00b3_eb94);
                let r312 = remap(mem, true, v311);
                let x313: f64 = f64::from(r312) + f64::from(y);
                let v314 = x313 as f32;
                let v315 = image_f32(mem, 0x00b3_eb94);
                let r316 = remap(mem, false, v315);
                let x317: f64 = f64::from(r316) + f64::from(x);
                let v318 = x317 as f32;
                call(
                    mem,
                    0x0090_3ee0,
                    &[
                        0x0000_00c8,
                        v318.to_bits(),
                        v314.to_bits(),
                        v310.to_bits(),
                        v305.to_bits(),
                        v300.to_bits(),
                        v299.to_bits(),
                        v298.to_bits(),
                        v297.to_bits(),
                        0x6480_8080,
                        0x6480_8080,
                    ],
                );
            }
            4 | 5 => {
                let x319: f64 = f64::from(y) + f64::from(h);
                let v320 = x319 as f32;
                let v321 = image_f32(mem, 0x00b9_2308);
                let r322 = remap(mem, true, v321);
                let v323 = image_f32(mem, 0x00b5_4c78);
                let r324 = remap(mem, false, v323);
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        x.to_bits(),
                        v320.to_bits(),
                        r324.to_bits(),
                        r322.to_bits(),
                        0x0000_0000,
                        0x2600_0000,
                    ],
                );
                let v325 = image_f32(mem, 0x00b9_2308);
                let r326 = remap(mem, true, v325);
                let d327: f64 = f64::from(w);
                let v328 = image_f32(mem, 0x00b5_ef4c);
                let r329 = remap(mem, false, v328);
                let x330: f64 = d327 - f64::from(r329);
                let v331 = x330 as f32;
                let v332 = image_f32(mem, 0x00b5_4c78);
                let r333 = remap(mem, false, v332);
                let x334: f64 = f64::from(r333) + f64::from(x);
                let v335 = x334 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        v335.to_bits(),
                        v320.to_bits(),
                        v331.to_bits(),
                        r326.to_bits(),
                        0x2600_0000,
                        0x2600_0000,
                    ],
                );
                let x336: f64 = f64::from(x) + f64::from(w);
                let v337 = image_f32(mem, 0x00b9_2308);
                let r338 = remap(mem, true, v337);
                let v339 = image_f32(mem, 0x00b5_4c78);
                let r340 = remap(mem, false, v339);
                let v341 = image_f32(mem, 0x00b5_4c78);
                let r342 = remap(mem, false, v341);
                let x343: f64 = x336 - f64::from(r342);
                let v344 = x343 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        v344.to_bits(),
                        v320.to_bits(),
                        r340.to_bits(),
                        r338.to_bits(),
                        0x2600_0000,
                        0x0000_0000,
                    ],
                );
                let v345 = image_f32(mem, 0x00b5_4c78);
                let r346 = remap(mem, false, v345);
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        x.to_bits(),
                        y.to_bits(),
                        r346.to_bits(),
                        h.to_bits(),
                        0x007a_5255,
                        0x407d_4036,
                    ],
                );
                let d347: f64 = f64::from(w);
                let v348 = image_f32(mem, 0x00b5_ef4c);
                let r349 = remap(mem, false, v348);
                let x350: f64 = d347 - f64::from(r349);
                let v351 = x350 as f32;
                let v352 = image_f32(mem, 0x00b5_4c78);
                let r353 = remap(mem, false, v352);
                let x354: f64 = f64::from(r353) + f64::from(x);
                let v355 = x354 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        v355.to_bits(),
                        y.to_bits(),
                        v351.to_bits(),
                        h.to_bits(),
                        0x407d_4036,
                        0x407d_4036,
                    ],
                );
                let v356 = image_f32(mem, 0x00b5_4c78);
                let r357 = remap(mem, false, v356);
                let v358 = image_f32(mem, 0x00b5_4c78);
                let r359 = remap(mem, false, v358);
                let x360: f64 = x336 - f64::from(r359);
                let v361 = x360 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        v361.to_bits(),
                        y.to_bits(),
                        r357.to_bits(),
                        h.to_bits(),
                        0x407d_4036,
                        0x007d_4036,
                    ],
                );
                let v362 = image_f32(mem, 0x00b9_2268);
                let r363 = remap(mem, true, v362);
                let v364 = image_f32(mem, 0x00b5_4c78);
                let r365 = remap(mem, false, v364);
                let v366 = 1.0f32;
                let r367 = remap(mem, true, v366);
                let x368: f64 = f64::from(r367) + f64::from(y);
                let v369 = x368 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        x.to_bits(),
                        v369.to_bits(),
                        r365.to_bits(),
                        r363.to_bits(),
                        0x00ff_ffff,
                        0x5aff_ffff,
                    ],
                );
                let v370 = image_f32(mem, 0x00b9_2268);
                let r371 = remap(mem, true, v370);
                let d372: f64 = f64::from(w);
                let v373 = image_f32(mem, 0x00b5_ef4c);
                let r374 = remap(mem, false, v373);
                let x375: f64 = d372 - f64::from(r374);
                let v376 = x375 as f32;
                let v377 = 1.0f32;
                let r378 = remap(mem, true, v377);
                let x379: f64 = f64::from(r378) + f64::from(y);
                let v380 = x379 as f32;
                let v381 = image_f32(mem, 0x00b5_4c78);
                let r382 = remap(mem, false, v381);
                let x383: f64 = f64::from(r382) + f64::from(x);
                let v384 = x383 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        v384.to_bits(),
                        v380.to_bits(),
                        v376.to_bits(),
                        r371.to_bits(),
                        0x5aff_ffff,
                        0x5aff_ffff,
                    ],
                );
                let v385 = image_f32(mem, 0x00b9_2268);
                let r386 = remap(mem, true, v385);
                let v387 = image_f32(mem, 0x00b5_4c78);
                let r388 = remap(mem, false, v387);
                let v389 = 1.0f32;
                let r390 = remap(mem, true, v389);
                let x391: f64 = f64::from(r390) + f64::from(y);
                let v392 = x391 as f32;
                let v393 = image_f32(mem, 0x00b5_4c78);
                let r394 = remap(mem, false, v393);
                let x395: f64 = x336 - f64::from(r394);
                let v396 = x395 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        v396.to_bits(),
                        v392.to_bits(),
                        r388.to_bits(),
                        r386.to_bits(),
                        0x5aff_ffff,
                        0x00ff_ffff,
                    ],
                );
                let v397 = image_f32(mem, 0x00b3_eb94);
                let r398 = remap(mem, true, v397);
                let v399 = image_f32(mem, 0x00b5_4c78);
                let r400 = remap(mem, false, v399);
                let v401 = image_f32(mem, 0x00b3_eb94);
                let r402 = remap(mem, true, v401);
                let x403: f64 = x319 - f64::from(r402);
                let v404 = x403 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        x.to_bits(),
                        v404.to_bits(),
                        r400.to_bits(),
                        r398.to_bits(),
                        0x0000_0000,
                        0x8000_0000,
                    ],
                );
                let v405 = image_f32(mem, 0x00b3_eb94);
                let r406 = remap(mem, true, v405);
                let d407: f64 = f64::from(w);
                let v408 = image_f32(mem, 0x00b5_ef4c);
                let r409 = remap(mem, false, v408);
                let x410: f64 = d407 - f64::from(r409);
                let v411 = x410 as f32;
                let v412 = image_f32(mem, 0x00b3_eb94);
                let r413 = remap(mem, true, v412);
                let x414: f64 = x319 - f64::from(r413);
                let v415 = x414 as f32;
                let v416 = image_f32(mem, 0x00b5_4c78);
                let r417 = remap(mem, false, v416);
                let x418: f64 = f64::from(r417) + f64::from(x);
                let v419 = x418 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        v419.to_bits(),
                        v415.to_bits(),
                        v411.to_bits(),
                        r406.to_bits(),
                        0x8000_0000,
                        0x8000_0000,
                    ],
                );
                let v420 = image_f32(mem, 0x00b3_eb94);
                let r421 = remap(mem, true, v420);
                let v422 = image_f32(mem, 0x00b5_4c78);
                let r423 = remap(mem, false, v422);
                let v424 = image_f32(mem, 0x00b3_eb94);
                let r425 = remap(mem, true, v424);
                let x426: f64 = x319 - f64::from(r425);
                let v427 = x426 as f32;
                let v428 = image_f32(mem, 0x00b5_4c78);
                let r429 = remap(mem, false, v428);
                let x430: f64 = x336 - f64::from(r429);
                let v431 = x430 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        v431.to_bits(),
                        v427.to_bits(),
                        r423.to_bits(),
                        r421.to_bits(),
                        0x8000_0000,
                        0x0000_0000,
                    ],
                );
                if kind == 4 {
                    let v432 = image_f32(mem, 0x00b6_5638);
                    let v433 = image_f32(mem, 0x00b5_edd8);
                    let v434 = image_f32(mem, 0x00b6_562c);
                    let v435 = image_f32(mem, 0x00b6_5628);
                    let v436 = image_f32(mem, 0x00b5_eec0);
                    let r437 = remap(mem, true, v436);
                    let v438 = image_f32(mem, 0x00b5_eec0);
                    let r439 = remap(mem, false, v438);
                    let d440: f64 = f64::from(y);
                    let v441 = image_f32(mem, 0x00b9_2268);
                    let r442 = remap(mem, true, v441);
                    let x443: f64 = d440 - f64::from(r442);
                    let v444 = x443 as f32;
                    let v445 = image_f32(mem, 0x00b6_5b20);
                    let r446 = remap(mem, false, v445);
                    let x447: f64 = f64::from(r446) + f64::from(x);
                    let v448 = x447 as f32;
                    call(
                        mem,
                        0x0090_3bb0,
                        &[
                            0x0000_00c8,
                            v448.to_bits(),
                            v444.to_bits(),
                            r439.to_bits(),
                            r437.to_bits(),
                            v435.to_bits(),
                            v434.to_bits(),
                            v433.to_bits(),
                            v432.to_bits(),
                        ],
                    );
                }
            }
            6 => {
                call(
                    mem,
                    0x008f_9c20,
                    &[
                        x.to_bits(),
                        y.to_bits(),
                        w.to_bits(),
                        h.to_bits(),
                        0x0000_0004,
                    ],
                );
                let v449 = image_f32(mem, 0x00b6_6b94);
                let v450 = image_f32(mem, 0x00b6_6c2c);
                let v451 = image_f32(mem, 0x00b6_6b8c);
                let v452 = image_f32(mem, 0x00b6_6bb4);
                let d453: f64 = f64::from(h);
                let v454 = image_f32(mem, 0x00b9_2318);
                let r455 = remap(mem, true, v454);
                let x456: f64 = d453 - f64::from(r455);
                let v457 = x456 as f32;
                let v458 = image_f32(mem, 0x00b6_6dcc);
                let r459 = remap(mem, false, v458);
                let v460 = image_f32(mem, 0x00b3_eb98);
                let r461 = remap(mem, true, v460);
                let x462: f64 = f64::from(r461) + f64::from(y);
                let v463 = x462 as f32;
                let v464 = image_f32(mem, 0x00b9_2308);
                let r465 = remap(mem, false, v464);
                let x466: f64 = f64::from(r465) + f64::from(x);
                let v467 = x466 as f32;
                call(
                    mem,
                    0x0090_3bb0,
                    &[
                        0x0000_00c8,
                        v467.to_bits(),
                        v463.to_bits(),
                        r459.to_bits(),
                        v457.to_bits(),
                        v452.to_bits(),
                        v451.to_bits(),
                        v450.to_bits(),
                        v449.to_bits(),
                    ],
                );
            }
            7 | 14 => {
                let x468: f64 = f64::from(y) + f64::from(h);
                let v469 = x468 as f32;
                let v470 = image_f32(mem, 0x00b9_22f8);
                let r471 = remap(mem, true, v470);
                let v472 = image_f32(mem, 0x00b4_1d8c);
                let r473 = remap(mem, false, v472);
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        x.to_bits(),
                        v469.to_bits(),
                        r473.to_bits(),
                        r471.to_bits(),
                        0x0000_0000,
                        0x2600_0000,
                    ],
                );
                let v474 = image_f32(mem, 0x00b9_22f8);
                let r475 = remap(mem, true, v474);
                let d476: f64 = f64::from(w);
                let v477 = image_f32(mem, 0x00b5_f384);
                let r478 = remap(mem, false, v477);
                let x479: f64 = d476 - f64::from(r478);
                let v480 = x479 as f32;
                let v481 = image_f32(mem, 0x00b4_1d8c);
                let r482 = remap(mem, false, v481);
                let x483: f64 = f64::from(r482) + f64::from(x);
                let v484 = x483 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        v484.to_bits(),
                        v469.to_bits(),
                        v480.to_bits(),
                        r475.to_bits(),
                        0x2600_0000,
                        0x2600_0000,
                    ],
                );
                let x485: f64 = f64::from(x) + f64::from(w);
                let v486 = image_f32(mem, 0x00b9_22f8);
                let r487 = remap(mem, true, v486);
                let v488 = image_f32(mem, 0x00b4_1d8c);
                let r489 = remap(mem, false, v488);
                let v490 = image_f32(mem, 0x00b4_1d8c);
                let r491 = remap(mem, false, v490);
                let x492: f64 = x485 - f64::from(r491);
                let v493 = x492 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        v493.to_bits(),
                        v469.to_bits(),
                        r489.to_bits(),
                        r487.to_bits(),
                        0x2600_0000,
                        0x0000_0000,
                    ],
                );
                let v494 = image_f32(mem, 0x00b4_1d8c);
                let r495 = remap(mem, false, v494);
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        x.to_bits(),
                        y.to_bits(),
                        r495.to_bits(),
                        h.to_bits(),
                        0x0070_3140,
                        0x4070_3140,
                    ],
                );
                let d496: f64 = f64::from(w);
                let v497 = image_f32(mem, 0x00b5_f384);
                let r498 = remap(mem, false, v497);
                let x499: f64 = d496 - f64::from(r498);
                let v500 = x499 as f32;
                let v501 = image_f32(mem, 0x00b4_1d8c);
                let r502 = remap(mem, false, v501);
                let x503: f64 = f64::from(r502) + f64::from(x);
                let v504 = x503 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        v504.to_bits(),
                        y.to_bits(),
                        v500.to_bits(),
                        h.to_bits(),
                        0x4070_3140,
                        0x4070_3140,
                    ],
                );
                let v505 = image_f32(mem, 0x00b4_1d8c);
                let r506 = remap(mem, false, v505);
                let v507 = image_f32(mem, 0x00b4_1d8c);
                let r508 = remap(mem, false, v507);
                let x509: f64 = x485 - f64::from(r508);
                let v510 = x509 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        v510.to_bits(),
                        y.to_bits(),
                        r506.to_bits(),
                        h.to_bits(),
                        0x4070_3140,
                        0x0070_3140,
                    ],
                );
                if kind == 7 {
                    let v581 = image_f32(mem, 0x00b6_5c00);
                    let v582 = image_f32(mem, 0x00b6_5c20);
                    let v583 = image_f32(mem, 0x00b4_9980);
                    let v584 = image_f32(mem, 0x00b6_5c1c);
                    let v585 = image_f32(mem, 0x00b5_eea4);
                    let r586 = remap(mem, true, v585);
                    let v587 = image_f32(mem, 0x00b6_5c08);
                    let r588 = remap(mem, false, v587);
                    let v589 = image_f32(mem, 0x00b3_eb94);
                    let r590 = remap(mem, true, v589);
                    let x591: f64 = f64::from(r590) + f64::from(y);
                    let v592 = x591 as f32;
                    let v593 = image_f32(mem, 0x00b6_56d0);
                    let r594 = remap(mem, false, v593);
                    let x595: f64 = f64::from(r594) + f64::from(x);
                    let v596 = x595 as f32;
                    call(
                        mem,
                        0x0090_3ee0,
                        &[
                            0xffff_ffff,
                            v596.to_bits(),
                            v592.to_bits(),
                            r588.to_bits(),
                            r586.to_bits(),
                            v584.to_bits(),
                            v583.to_bits(),
                            v582.to_bits(),
                            v581.to_bits(),
                            0x4080_8080,
                            0x4080_8080,
                        ],
                    );
                    let v597 = image_f32(mem, 0x00b9_2268);
                    let r598 = remap(mem, true, v597);
                    let v599 = image_f32(mem, 0x00b4_1d8c);
                    let r600 = remap(mem, false, v599);
                    let v601 = 1.0f32;
                    let r602 = remap(mem, true, v601);
                    let x603: f64 = f64::from(r602) + f64::from(y);
                    let v604 = x603 as f32;
                    call(
                        mem,
                        0x008f_4b20,
                        &[
                            x.to_bits(),
                            v604.to_bits(),
                            r600.to_bits(),
                            r598.to_bits(),
                            0x00ff_ffff,
                            0x5aff_ffff,
                        ],
                    );
                    let v605 = image_f32(mem, 0x00b9_2268);
                    let r606 = remap(mem, true, v605);
                    let d607: f64 = f64::from(w);
                    let v608 = image_f32(mem, 0x00b5_f384);
                    let r609 = remap(mem, false, v608);
                    let x610: f64 = d607 - f64::from(r609);
                    let v611 = x610 as f32;
                    let v612 = 1.0f32;
                    let r613 = remap(mem, true, v612);
                    let x614: f64 = f64::from(r613) + f64::from(y);
                    let v615 = x614 as f32;
                    let v616 = image_f32(mem, 0x00b4_1d8c);
                    let r617 = remap(mem, false, v616);
                    let x618: f64 = f64::from(r617) + f64::from(x);
                    let v619 = x618 as f32;
                    call(
                        mem,
                        0x008f_4b20,
                        &[
                            v619.to_bits(),
                            v615.to_bits(),
                            v611.to_bits(),
                            r606.to_bits(),
                            0x5aff_ffff,
                            0x5aff_ffff,
                        ],
                    );
                    let v620 = image_f32(mem, 0x00b9_2268);
                    let r621 = remap(mem, true, v620);
                    let v622 = image_f32(mem, 0x00b4_1d8c);
                    let r623 = remap(mem, false, v622);
                    let v624 = 1.0f32;
                    let r625 = remap(mem, true, v624);
                    let x626: f64 = f64::from(r625) + f64::from(y);
                    let v627 = x626 as f32;
                    let v628 = image_f32(mem, 0x00b4_1d8c);
                    let r629 = remap(mem, false, v628);
                    let x630: f64 = x485 - f64::from(r629);
                    let v631 = x630 as f32;
                    call(
                        mem,
                        0x008f_4b20,
                        &[
                            v631.to_bits(),
                            v627.to_bits(),
                            r623.to_bits(),
                            r621.to_bits(),
                            0x5aff_ffff,
                            0x00ff_ffff,
                        ],
                    );
                    let v632 = image_f32(mem, 0x00b3_eb94);
                    let r633 = remap(mem, true, v632);
                    let v634 = image_f32(mem, 0x00b4_1d8c);
                    let r635 = remap(mem, false, v634);
                    let v636 = image_f32(mem, 0x00b3_eb94);
                    let r637 = remap(mem, true, v636);
                    let x638: f64 = x468 - f64::from(r637);
                    let v639 = x638 as f32;
                    call(
                        mem,
                        0x008f_4b20,
                        &[
                            x.to_bits(),
                            v639.to_bits(),
                            r635.to_bits(),
                            r633.to_bits(),
                            0x0000_0000,
                            0x8000_0000,
                        ],
                    );
                    let v640 = image_f32(mem, 0x00b3_eb94);
                    let r641 = remap(mem, true, v640);
                    let d642: f64 = f64::from(w);
                    let v643 = image_f32(mem, 0x00b5_f384);
                    let r644 = remap(mem, false, v643);
                    let x645: f64 = d642 - f64::from(r644);
                    let v646 = x645 as f32;
                    let v647 = image_f32(mem, 0x00b3_eb94);
                    let r648 = remap(mem, true, v647);
                    let x649: f64 = x468 - f64::from(r648);
                    let v650 = x649 as f32;
                    let v651 = image_f32(mem, 0x00b4_1d8c);
                    let r652 = remap(mem, false, v651);
                    let x653: f64 = f64::from(r652) + f64::from(x);
                    let v654 = x653 as f32;
                    call(
                        mem,
                        0x008f_4b20,
                        &[
                            v654.to_bits(),
                            v650.to_bits(),
                            v646.to_bits(),
                            r641.to_bits(),
                            0x8000_0000,
                            0x8000_0000,
                        ],
                    );
                    let v655 = image_f32(mem, 0x00b3_eb94);
                    let r656 = remap(mem, true, v655);
                    let v657 = image_f32(mem, 0x00b4_1d8c);
                    let r658 = remap(mem, false, v657);
                    let v659 = image_f32(mem, 0x00b3_eb94);
                    let r660 = remap(mem, true, v659);
                    let x661: f64 = x468 - f64::from(r660);
                    let v662 = x661 as f32;
                    let v663 = image_f32(mem, 0x00b4_1d8c);
                    let r664 = remap(mem, false, v663);
                    let x665: f64 = x485 - f64::from(r664);
                    let v666 = x665 as f32;
                    call(
                        mem,
                        0x008f_4b20,
                        &[
                            v666.to_bits(),
                            v662.to_bits(),
                            r658.to_bits(),
                            r656.to_bits(),
                            0x8000_0000,
                            0x0000_0000,
                        ],
                    );
                } else {
                    let v511 = image_f32(mem, 0x00b9_2268);
                    let r512 = remap(mem, true, v511);
                    let v513 = image_f32(mem, 0x00b4_1d8c);
                    let r514 = remap(mem, false, v513);
                    let v515 = 1.0f32;
                    let r516 = remap(mem, true, v515);
                    let x517: f64 = f64::from(r516) + f64::from(y);
                    let v518 = x517 as f32;
                    call(
                        mem,
                        0x008f_4b20,
                        &[
                            x.to_bits(),
                            v518.to_bits(),
                            r514.to_bits(),
                            r512.to_bits(),
                            0x00ff_ffff,
                            0x5aff_ffff,
                        ],
                    );
                    let v519 = image_f32(mem, 0x00b9_2268);
                    let r520 = remap(mem, true, v519);
                    let d521: f64 = f64::from(w);
                    let v522 = image_f32(mem, 0x00b5_f384);
                    let r523 = remap(mem, false, v522);
                    let x524: f64 = d521 - f64::from(r523);
                    let v525 = x524 as f32;
                    let v526 = 1.0f32;
                    let r527 = remap(mem, true, v526);
                    let x528: f64 = f64::from(r527) + f64::from(y);
                    let v529 = x528 as f32;
                    let v530 = image_f32(mem, 0x00b4_1d8c);
                    let r531 = remap(mem, false, v530);
                    let x532: f64 = f64::from(r531) + f64::from(x);
                    let v533 = x532 as f32;
                    call(
                        mem,
                        0x008f_4b20,
                        &[
                            v533.to_bits(),
                            v529.to_bits(),
                            v525.to_bits(),
                            r520.to_bits(),
                            0x5aff_ffff,
                            0x5aff_ffff,
                        ],
                    );
                    let v534 = image_f32(mem, 0x00b9_2268);
                    let r535 = remap(mem, true, v534);
                    let v536 = image_f32(mem, 0x00b4_1d8c);
                    let r537 = remap(mem, false, v536);
                    let v538 = 1.0f32;
                    let r539 = remap(mem, true, v538);
                    let x540: f64 = f64::from(r539) + f64::from(y);
                    let v541 = x540 as f32;
                    let v542 = image_f32(mem, 0x00b4_1d8c);
                    let r543 = remap(mem, false, v542);
                    let x544: f64 = x485 - f64::from(r543);
                    let v545 = x544 as f32;
                    call(
                        mem,
                        0x008f_4b20,
                        &[
                            v545.to_bits(),
                            v541.to_bits(),
                            r537.to_bits(),
                            r535.to_bits(),
                            0x5aff_ffff,
                            0x00ff_ffff,
                        ],
                    );
                    let v546 = image_f32(mem, 0x00b3_eb94);
                    let r547 = remap(mem, true, v546);
                    let v548 = image_f32(mem, 0x00b4_1d8c);
                    let r549 = remap(mem, false, v548);
                    let v550 = image_f32(mem, 0x00b3_eb94);
                    let r551 = remap(mem, true, v550);
                    let x552: f64 = x468 - f64::from(r551);
                    let v553 = x552 as f32;
                    call(
                        mem,
                        0x008f_4b20,
                        &[
                            x.to_bits(),
                            v553.to_bits(),
                            r549.to_bits(),
                            r547.to_bits(),
                            0x0000_0000,
                            0x8000_0000,
                        ],
                    );
                    let v554 = image_f32(mem, 0x00b3_eb94);
                    let r555 = remap(mem, true, v554);
                    let d556: f64 = f64::from(w);
                    let v557 = image_f32(mem, 0x00b5_f384);
                    let r558 = remap(mem, false, v557);
                    let x559: f64 = d556 - f64::from(r558);
                    let v560 = x559 as f32;
                    let v561 = image_f32(mem, 0x00b3_eb94);
                    let r562 = remap(mem, true, v561);
                    let x563: f64 = x468 - f64::from(r562);
                    let v564 = x563 as f32;
                    let v565 = image_f32(mem, 0x00b4_1d8c);
                    let r566 = remap(mem, false, v565);
                    let x567: f64 = f64::from(r566) + f64::from(x);
                    let v568 = x567 as f32;
                    call(
                        mem,
                        0x008f_4b20,
                        &[
                            v568.to_bits(),
                            v564.to_bits(),
                            v560.to_bits(),
                            r555.to_bits(),
                            0x8000_0000,
                            0x8000_0000,
                        ],
                    );
                    let v569 = image_f32(mem, 0x00b3_eb94);
                    let r570 = remap(mem, true, v569);
                    let v571 = image_f32(mem, 0x00b4_1d8c);
                    let r572 = remap(mem, false, v571);
                    let v573 = image_f32(mem, 0x00b3_eb94);
                    let r574 = remap(mem, true, v573);
                    let x575: f64 = x468 - f64::from(r574);
                    let v576 = x575 as f32;
                    let v577 = image_f32(mem, 0x00b4_1d8c);
                    let r578 = remap(mem, false, v577);
                    let x579: f64 = x485 - f64::from(r578);
                    let v580 = x579 as f32;
                    call(
                        mem,
                        0x008f_4b20,
                        &[
                            v580.to_bits(),
                            v576.to_bits(),
                            r572.to_bits(),
                            r570.to_bits(),
                            0x8000_0000,
                            0x0000_0000,
                        ],
                    );
                }
            }
            8 => {
                call(
                    mem,
                    0x008f_9c20,
                    &[
                        x.to_bits(),
                        y.to_bits(),
                        w.to_bits(),
                        h.to_bits(),
                        0x0000_0004,
                    ],
                );
                let v667 = image_f32(mem, 0x00b6_6ba8);
                let v668 = image_f32(mem, 0x00b6_6c2c);
                let v669 = image_f32(mem, 0x00b6_6b94);
                let v670 = image_f32(mem, 0x00b6_6bb4);
                let d671: f64 = f64::from(h);
                let v672 = image_f32(mem, 0x00b9_2318);
                let r673 = remap(mem, true, v672);
                let x674: f64 = d671 - f64::from(r673);
                let v675 = x674 as f32;
                let v676 = image_f32(mem, 0x00b6_6dcc);
                let r677 = remap(mem, false, v676);
                let v678 = image_f32(mem, 0x00b3_eb98);
                let r679 = remap(mem, true, v678);
                let x680: f64 = f64::from(r679) + f64::from(y);
                let v681 = x680 as f32;
                let v682 = image_f32(mem, 0x00b9_2308);
                let r683 = remap(mem, false, v682);
                let x684: f64 = f64::from(r683) + f64::from(x);
                let v685 = x684 as f32;
                call(
                    mem,
                    0x0090_3bb0,
                    &[
                        0x0000_00c8,
                        v685.to_bits(),
                        v681.to_bits(),
                        r677.to_bits(),
                        v675.to_bits(),
                        v670.to_bits(),
                        v669.to_bits(),
                        v668.to_bits(),
                        v667.to_bits(),
                    ],
                );
            }
            9 => {
                let x686: f64 = f64::from(h) * img64(mem, 0x00b4_1d58);
                let x687: f64 = x686 / img64(mem, 0x00b4_1d30);
                if kind == 9 {
                    let v931 = x687 as f32;
                    let x932: f64 = f64::from(w) * img64(mem, 0x00b4_1d68);
                    let x933: f64 = x932 * img64(mem, 0x00b4_1cc0);
                    let v934 = x933 as f32;
                    let v935 = 0.0f32;
                    let v936 = 0.0f32;
                    call(
                        mem,
                        0x0090_3ee0,
                        &[
                            0xffff_fffc,
                            x.to_bits(),
                            y.to_bits(),
                            w.to_bits(),
                            h.to_bits(),
                            v936.to_bits(),
                            v935.to_bits(),
                            v934.to_bits(),
                            v931.to_bits(),
                            0x8080_8080,
                            0x8080_8080,
                        ],
                    );
                    call(
                        mem,
                        0x008f_4df0,
                        &[
                            x.to_bits(),
                            y.to_bits(),
                            w.to_bits(),
                            h.to_bits(),
                            0x668f_4b2d,
                            0x6617_080a,
                        ],
                    );
                    let v937 = image_f32(mem, 0x00b6_6bc4);
                    let v938 = image_f32(mem, 0x00b6_6bb8);
                    let v939 = image_f32(mem, 0x00b6_6bc0);
                    let v940 = image_f32(mem, 0x00b6_6bb4);
                    let v941 = image_f32(mem, 0x00b5_f2e8);
                    let r942 = remap(mem, true, v941);
                    let v943 = image_f32(mem, 0x00b5_f2e8);
                    let r944 = remap(mem, false, v943);
                    call(
                        mem,
                        0x0090_3bb0,
                        &[
                            0x0000_00c8,
                            x.to_bits(),
                            y.to_bits(),
                            r944.to_bits(),
                            r942.to_bits(),
                            v940.to_bits(),
                            v939.to_bits(),
                            v938.to_bits(),
                            v937.to_bits(),
                        ],
                    );
                    let v945 = image_f32(mem, 0x00b6_6bd8);
                    let v946 = image_f32(mem, 0x00b6_6bb8);
                    let v947 = image_f32(mem, 0x00b6_6bc4);
                    let v948 = image_f32(mem, 0x00b6_6bb4);
                    let d949: f64 = f64::from(h);
                    let v950 = image_f32(mem, 0x00b5_4f84);
                    let r951 = remap(mem, true, v950);
                    let x952: f64 = d949 - f64::from(r951);
                    let v953 = x952 as f32;
                    let v954 = image_f32(mem, 0x00b5_f2e8);
                    let r955 = remap(mem, false, v954);
                    let v956 = image_f32(mem, 0x00b5_f2e8);
                    let r957 = remap(mem, true, v956);
                    let x958: f64 = f64::from(r957) + f64::from(y);
                    let v959 = x958 as f32;
                    call(
                        mem,
                        0x0090_3bb0,
                        &[
                            0x0000_00c8,
                            x.to_bits(),
                            v959.to_bits(),
                            r955.to_bits(),
                            v953.to_bits(),
                            v948.to_bits(),
                            v947.to_bits(),
                            v946.to_bits(),
                            v945.to_bits(),
                        ],
                    );
                    let x960: f64 = f64::from(y) + f64::from(h);
                    let v961 = image_f32(mem, 0x00b5_f0d8);
                    let v962 = image_f32(mem, 0x00b6_6bb8);
                    let v963 = image_f32(mem, 0x00b6_6bd8);
                    let v964 = image_f32(mem, 0x00b6_6bb4);
                    let v965 = image_f32(mem, 0x00b5_f2e8);
                    let r966 = remap(mem, true, v965);
                    let v967 = image_f32(mem, 0x00b5_f2e8);
                    let r968 = remap(mem, false, v967);
                    let v969 = image_f32(mem, 0x00b5_f2e8);
                    let r970 = remap(mem, true, v969);
                    let x971: f64 = x960 - f64::from(r970);
                    let v972 = x971 as f32;
                    call(
                        mem,
                        0x0090_3bb0,
                        &[
                            0x0000_00c8,
                            x.to_bits(),
                            v972.to_bits(),
                            r968.to_bits(),
                            r966.to_bits(),
                            v964.to_bits(),
                            v963.to_bits(),
                            v962.to_bits(),
                            v961.to_bits(),
                        ],
                    );
                    let v973 = image_f32(mem, 0x00b6_6bc4);
                    let v974 = image_f32(mem, 0x00b6_5820);
                    let v975 = image_f32(mem, 0x00b6_6bc0);
                    let v976 = image_f32(mem, 0x00b6_6bb8);
                    let v977 = image_f32(mem, 0x00b5_f2e8);
                    let r978 = remap(mem, true, v977);
                    let d979: f64 = f64::from(w);
                    let v980 = image_f32(mem, 0x00b5_4f84);
                    let r981 = remap(mem, false, v980);
                    let x982: f64 = d979 - f64::from(r981);
                    let v983 = x982 as f32;
                    let v984 = image_f32(mem, 0x00b5_f2e8);
                    let r985 = remap(mem, false, v984);
                    let x986: f64 = f64::from(r985) + f64::from(x);
                    let v987 = x986 as f32;
                    call(
                        mem,
                        0x0090_3bb0,
                        &[
                            0x0000_00c8,
                            v987.to_bits(),
                            y.to_bits(),
                            v983.to_bits(),
                            r978.to_bits(),
                            v976.to_bits(),
                            v975.to_bits(),
                            v974.to_bits(),
                            v973.to_bits(),
                        ],
                    );
                    let v988 = image_f32(mem, 0x00b5_f0d8);
                    let v989 = image_f32(mem, 0x00b6_5820);
                    let v990 = image_f32(mem, 0x00b6_6bd8);
                    let v991 = image_f32(mem, 0x00b6_6bb8);
                    let v992 = image_f32(mem, 0x00b5_f2e8);
                    let r993 = remap(mem, true, v992);
                    let d994: f64 = f64::from(w);
                    let v995 = image_f32(mem, 0x00b5_4f84);
                    let r996 = remap(mem, false, v995);
                    let x997: f64 = d994 - f64::from(r996);
                    let v998 = x997 as f32;
                    let v999 = image_f32(mem, 0x00b5_f2e8);
                    let r1000 = remap(mem, true, v999);
                    let x1001: f64 = x960 - f64::from(r1000);
                    let v1002 = x1001 as f32;
                    let v1003 = image_f32(mem, 0x00b5_f2e8);
                    let r1004 = remap(mem, false, v1003);
                    let x1005: f64 = f64::from(r1004) + f64::from(x);
                    let v1006 = x1005 as f32;
                    call(
                        mem,
                        0x0090_3bb0,
                        &[
                            0x0000_00c8,
                            v1006.to_bits(),
                            v1002.to_bits(),
                            v998.to_bits(),
                            r993.to_bits(),
                            v991.to_bits(),
                            v990.to_bits(),
                            v989.to_bits(),
                            v988.to_bits(),
                        ],
                    );
                    let x1007: f64 = f64::from(x) + f64::from(w);
                    let v1008 = image_f32(mem, 0x00b6_6bc4);
                    let v1009 = image_f32(mem, 0x00b6_6bd4);
                    let v1010 = image_f32(mem, 0x00b6_6bc0);
                    let v1011 = image_f32(mem, 0x00b6_5820);
                    let v1012 = image_f32(mem, 0x00b5_f2e8);
                    let r1013 = remap(mem, true, v1012);
                    let v1014 = image_f32(mem, 0x00b5_f2e8);
                    let r1015 = remap(mem, false, v1014);
                    let v1016 = image_f32(mem, 0x00b5_f2e8);
                    let r1017 = remap(mem, false, v1016);
                    let x1018: f64 = x1007 - f64::from(r1017);
                    let v1019 = x1018 as f32;
                    call(
                        mem,
                        0x0090_3bb0,
                        &[
                            0x0000_00c8,
                            v1019.to_bits(),
                            y.to_bits(),
                            r1015.to_bits(),
                            r1013.to_bits(),
                            v1011.to_bits(),
                            v1010.to_bits(),
                            v1009.to_bits(),
                            v1008.to_bits(),
                        ],
                    );
                    let v1020 = image_f32(mem, 0x00b6_6bd8);
                    let v1021 = image_f32(mem, 0x00b6_6bd4);
                    let v1022 = image_f32(mem, 0x00b6_6bc4);
                    let v1023 = image_f32(mem, 0x00b6_5820);
                    let d1024: f64 = f64::from(h);
                    let v1025 = image_f32(mem, 0x00b5_4f84);
                    let r1026 = remap(mem, true, v1025);
                    let x1027: f64 = d1024 - f64::from(r1026);
                    let v1028 = x1027 as f32;
                    let v1029 = image_f32(mem, 0x00b5_f2e8);
                    let r1030 = remap(mem, false, v1029);
                    let v1031 = image_f32(mem, 0x00b5_f2e8);
                    let r1032 = remap(mem, true, v1031);
                    let x1033: f64 = f64::from(r1032) + f64::from(y);
                    let v1034 = x1033 as f32;
                    let v1035 = image_f32(mem, 0x00b5_f2e8);
                    let r1036 = remap(mem, false, v1035);
                    let x1037: f64 = x1007 - f64::from(r1036);
                    let v1038 = x1037 as f32;
                    call(
                        mem,
                        0x0090_3bb0,
                        &[
                            0x0000_00c8,
                            v1038.to_bits(),
                            v1034.to_bits(),
                            r1030.to_bits(),
                            v1028.to_bits(),
                            v1023.to_bits(),
                            v1022.to_bits(),
                            v1021.to_bits(),
                            v1020.to_bits(),
                        ],
                    );
                    let v1039 = image_f32(mem, 0x00b5_f0d8);
                    let v1040 = image_f32(mem, 0x00b6_6bd4);
                    let v1041 = image_f32(mem, 0x00b6_6bd8);
                    let v1042 = image_f32(mem, 0x00b6_5820);
                    let v1043 = image_f32(mem, 0x00b5_f2e8);
                    let r1044 = remap(mem, true, v1043);
                    let v1045 = image_f32(mem, 0x00b5_f2e8);
                    let r1046 = remap(mem, false, v1045);
                    let v1047 = image_f32(mem, 0x00b5_f2e8);
                    let r1048 = remap(mem, true, v1047);
                    let x1049: f64 = x960 - f64::from(r1048);
                    let v1050 = x1049 as f32;
                    let v1051 = image_f32(mem, 0x00b5_f2e8);
                    let r1052 = remap(mem, false, v1051);
                    let x1053: f64 = x1007 - f64::from(r1052);
                    let v1054 = x1053 as f32;
                    call(
                        mem,
                        0x0090_3bb0,
                        &[
                            0x0000_00c8,
                            v1054.to_bits(),
                            v1050.to_bits(),
                            r1046.to_bits(),
                            r1044.to_bits(),
                            v1042.to_bits(),
                            v1041.to_bits(),
                            v1040.to_bits(),
                            v1039.to_bits(),
                        ],
                    );
                } else {
                    let v688 = x687 as f32;
                    let x689: f64 = f64::from(w) * img64(mem, 0x00b4_1d68);
                    let x690: f64 = x689 * img64(mem, 0x00b4_1cc0);
                    let v691 = x690 as f32;
                    let v692 = 0.0f32;
                    let v693 = 0.0f32;
                    call(
                        mem,
                        0x0090_3ee0,
                        &[
                            0xffff_fffc,
                            x.to_bits(),
                            y.to_bits(),
                            w.to_bits(),
                            h.to_bits(),
                            v693.to_bits(),
                            v692.to_bits(),
                            v691.to_bits(),
                            v688.to_bits(),
                            0x4080_8080,
                            0x4080_8080,
                        ],
                    );
                    let e694 = call(mem, 0x0064_1060, &[]);
                    if e694 == 0 {
                        call(
                            mem,
                            0x008f_4df0,
                            &[
                                x.to_bits(),
                                y.to_bits(),
                                w.to_bits(),
                                h.to_bits(),
                                0x608f_2c45,
                                0x6050_1826,
                            ],
                        );
                        let v813 = image_f32(mem, 0x00b6_6bc4);
                        let v814 = image_f32(mem, 0x00b6_6bb8);
                        let v815 = image_f32(mem, 0x00b6_6bc0);
                        let v816 = image_f32(mem, 0x00b6_6bb4);
                        let v817 = image_f32(mem, 0x00b5_f2e8);
                        let r818 = remap(mem, true, v817);
                        let v819 = image_f32(mem, 0x00b5_f2e8);
                        let r820 = remap(mem, false, v819);
                        call(
                            mem,
                            0x0090_3bb0,
                            &[
                                0x0000_00c8,
                                x.to_bits(),
                                y.to_bits(),
                                r820.to_bits(),
                                r818.to_bits(),
                                v816.to_bits(),
                                v815.to_bits(),
                                v814.to_bits(),
                                v813.to_bits(),
                            ],
                        );
                        let v821 = image_f32(mem, 0x00b6_6bd8);
                        let v822 = image_f32(mem, 0x00b6_6bb8);
                        let v823 = image_f32(mem, 0x00b6_6bc4);
                        let v824 = image_f32(mem, 0x00b6_6bb4);
                        let d825: f64 = f64::from(h);
                        let v826 = image_f32(mem, 0x00b5_4f84);
                        let r827 = remap(mem, true, v826);
                        let x828: f64 = d825 - f64::from(r827);
                        let v829 = x828 as f32;
                        let v830 = image_f32(mem, 0x00b5_f2e8);
                        let r831 = remap(mem, false, v830);
                        let v832 = image_f32(mem, 0x00b5_f2e8);
                        let r833 = remap(mem, true, v832);
                        let x834: f64 = f64::from(r833) + f64::from(y);
                        let v835 = x834 as f32;
                        call(
                            mem,
                            0x0090_3bb0,
                            &[
                                0x0000_00c8,
                                x.to_bits(),
                                v835.to_bits(),
                                r831.to_bits(),
                                v829.to_bits(),
                                v824.to_bits(),
                                v823.to_bits(),
                                v822.to_bits(),
                                v821.to_bits(),
                            ],
                        );
                        let x836: f64 = f64::from(y) + f64::from(h);
                        let v837 = image_f32(mem, 0x00b5_f0d8);
                        let v838 = image_f32(mem, 0x00b6_6bb8);
                        let v839 = image_f32(mem, 0x00b6_6bd8);
                        let v840 = image_f32(mem, 0x00b6_6bb4);
                        let v841 = image_f32(mem, 0x00b5_f2e8);
                        let r842 = remap(mem, true, v841);
                        let v843 = image_f32(mem, 0x00b5_f2e8);
                        let r844 = remap(mem, false, v843);
                        let v845 = image_f32(mem, 0x00b5_f2e8);
                        let r846 = remap(mem, true, v845);
                        let x847: f64 = x836 - f64::from(r846);
                        let v848 = x847 as f32;
                        call(
                            mem,
                            0x0090_3bb0,
                            &[
                                0x0000_00c8,
                                x.to_bits(),
                                v848.to_bits(),
                                r844.to_bits(),
                                r842.to_bits(),
                                v840.to_bits(),
                                v839.to_bits(),
                                v838.to_bits(),
                                v837.to_bits(),
                            ],
                        );
                        let v849 = image_f32(mem, 0x00b6_6bc4);
                        let v850 = image_f32(mem, 0x00b6_5820);
                        let v851 = image_f32(mem, 0x00b6_6bc0);
                        let v852 = image_f32(mem, 0x00b6_6bb8);
                        let v853 = image_f32(mem, 0x00b5_f2e8);
                        let r854 = remap(mem, true, v853);
                        let d855: f64 = f64::from(w);
                        let v856 = image_f32(mem, 0x00b5_4f84);
                        let r857 = remap(mem, false, v856);
                        let x858: f64 = d855 - f64::from(r857);
                        let v859 = x858 as f32;
                        let v860 = image_f32(mem, 0x00b5_f2e8);
                        let r861 = remap(mem, false, v860);
                        let x862: f64 = f64::from(r861) + f64::from(x);
                        let v863 = x862 as f32;
                        call(
                            mem,
                            0x0090_3bb0,
                            &[
                                0x0000_00c8,
                                v863.to_bits(),
                                y.to_bits(),
                                v859.to_bits(),
                                r854.to_bits(),
                                v852.to_bits(),
                                v851.to_bits(),
                                v850.to_bits(),
                                v849.to_bits(),
                            ],
                        );
                        let v864 = image_f32(mem, 0x00b5_f0d8);
                        let v865 = image_f32(mem, 0x00b6_5820);
                        let v866 = image_f32(mem, 0x00b6_6bd8);
                        let v867 = image_f32(mem, 0x00b6_6bb8);
                        let v868 = image_f32(mem, 0x00b5_f2e8);
                        let r869 = remap(mem, true, v868);
                        let d870: f64 = f64::from(w);
                        let v871 = image_f32(mem, 0x00b5_4f84);
                        let r872 = remap(mem, false, v871);
                        let x873: f64 = d870 - f64::from(r872);
                        let v874 = x873 as f32;
                        let v875 = image_f32(mem, 0x00b5_f2e8);
                        let r876 = remap(mem, true, v875);
                        let x877: f64 = x836 - f64::from(r876);
                        let v878 = x877 as f32;
                        let v879 = image_f32(mem, 0x00b5_f2e8);
                        let r880 = remap(mem, false, v879);
                        let x881: f64 = f64::from(r880) + f64::from(x);
                        let v882 = x881 as f32;
                        call(
                            mem,
                            0x0090_3bb0,
                            &[
                                0x0000_00c8,
                                v882.to_bits(),
                                v878.to_bits(),
                                v874.to_bits(),
                                r869.to_bits(),
                                v867.to_bits(),
                                v866.to_bits(),
                                v865.to_bits(),
                                v864.to_bits(),
                            ],
                        );
                        let x883: f64 = f64::from(x) + f64::from(w);
                        let v884 = image_f32(mem, 0x00b6_6bc4);
                        let v885 = image_f32(mem, 0x00b6_6bd4);
                        let v886 = image_f32(mem, 0x00b6_6bc0);
                        let v887 = image_f32(mem, 0x00b6_5820);
                        let v888 = image_f32(mem, 0x00b5_f2e8);
                        let r889 = remap(mem, true, v888);
                        let v890 = image_f32(mem, 0x00b5_f2e8);
                        let r891 = remap(mem, false, v890);
                        let v892 = image_f32(mem, 0x00b5_f2e8);
                        let r893 = remap(mem, false, v892);
                        let x894: f64 = x883 - f64::from(r893);
                        let v895 = x894 as f32;
                        call(
                            mem,
                            0x0090_3bb0,
                            &[
                                0x0000_00c8,
                                v895.to_bits(),
                                y.to_bits(),
                                r891.to_bits(),
                                r889.to_bits(),
                                v887.to_bits(),
                                v886.to_bits(),
                                v885.to_bits(),
                                v884.to_bits(),
                            ],
                        );
                        let v896 = image_f32(mem, 0x00b6_6bd8);
                        let v897 = image_f32(mem, 0x00b6_6bd4);
                        let v898 = image_f32(mem, 0x00b6_6bc4);
                        let v899 = image_f32(mem, 0x00b6_5820);
                        let d900: f64 = f64::from(h);
                        let v901 = image_f32(mem, 0x00b5_4f84);
                        let r902 = remap(mem, true, v901);
                        let x903: f64 = d900 - f64::from(r902);
                        let v904 = x903 as f32;
                        let v905 = image_f32(mem, 0x00b5_f2e8);
                        let r906 = remap(mem, false, v905);
                        let v907 = image_f32(mem, 0x00b5_f2e8);
                        let r908 = remap(mem, true, v907);
                        let x909: f64 = f64::from(r908) + f64::from(y);
                        let v910 = x909 as f32;
                        let v911 = image_f32(mem, 0x00b5_f2e8);
                        let r912 = remap(mem, false, v911);
                        let x913: f64 = x883 - f64::from(r912);
                        let v914 = x913 as f32;
                        call(
                            mem,
                            0x0090_3bb0,
                            &[
                                0x0000_00c8,
                                v914.to_bits(),
                                v910.to_bits(),
                                r906.to_bits(),
                                v904.to_bits(),
                                v899.to_bits(),
                                v898.to_bits(),
                                v897.to_bits(),
                                v896.to_bits(),
                            ],
                        );
                        let v915 = image_f32(mem, 0x00b5_f0d8);
                        let v916 = image_f32(mem, 0x00b6_6bd4);
                        let v917 = image_f32(mem, 0x00b6_6bd8);
                        let v918 = image_f32(mem, 0x00b6_5820);
                        let v919 = image_f32(mem, 0x00b5_f2e8);
                        let r920 = remap(mem, true, v919);
                        let v921 = image_f32(mem, 0x00b5_f2e8);
                        let r922 = remap(mem, false, v921);
                        let v923 = image_f32(mem, 0x00b5_f2e8);
                        let r924 = remap(mem, true, v923);
                        let x925: f64 = x836 - f64::from(r924);
                        let v926 = x925 as f32;
                        let v927 = image_f32(mem, 0x00b5_f2e8);
                        let r928 = remap(mem, false, v927);
                        let x929: f64 = x883 - f64::from(r928);
                        let v930 = x929 as f32;
                        call(
                            mem,
                            0x0090_3bb0,
                            &[
                                0x0000_00c8,
                                v930.to_bits(),
                                v926.to_bits(),
                                r922.to_bits(),
                                r920.to_bits(),
                                v918.to_bits(),
                                v917.to_bits(),
                                v916.to_bits(),
                                v915.to_bits(),
                            ],
                        );
                    } else {
                        call(
                            mem,
                            0x008f_4df0,
                            &[
                                x.to_bits(),
                                y.to_bits(),
                                w.to_bits(),
                                h.to_bits(),
                                0x6034_0614,
                                0x6030_0410,
                            ],
                        );
                        let v695 = image_f32(mem, 0x00b6_6bc4);
                        let v696 = image_f32(mem, 0x00b6_6bb8);
                        let v697 = image_f32(mem, 0x00b6_6bc0);
                        let v698 = image_f32(mem, 0x00b6_6bb4);
                        let v699 = image_f32(mem, 0x00b5_f2e8);
                        let r700 = remap(mem, true, v699);
                        let v701 = image_f32(mem, 0x00b5_f2e8);
                        let r702 = remap(mem, false, v701);
                        call(
                            mem,
                            0x0090_3bb0,
                            &[
                                0x0000_00c8,
                                x.to_bits(),
                                y.to_bits(),
                                r702.to_bits(),
                                r700.to_bits(),
                                v698.to_bits(),
                                v697.to_bits(),
                                v696.to_bits(),
                                v695.to_bits(),
                            ],
                        );
                        let v703 = image_f32(mem, 0x00b6_6bd8);
                        let v704 = image_f32(mem, 0x00b6_6bb8);
                        let v705 = image_f32(mem, 0x00b6_6bc4);
                        let v706 = image_f32(mem, 0x00b6_6bb4);
                        let d707: f64 = f64::from(h);
                        let v708 = image_f32(mem, 0x00b5_4f84);
                        let r709 = remap(mem, true, v708);
                        let x710: f64 = d707 - f64::from(r709);
                        let v711 = x710 as f32;
                        let v712 = image_f32(mem, 0x00b5_f2e8);
                        let r713 = remap(mem, false, v712);
                        let v714 = image_f32(mem, 0x00b5_f2e8);
                        let r715 = remap(mem, true, v714);
                        let x716: f64 = f64::from(r715) + f64::from(y);
                        let v717 = x716 as f32;
                        call(
                            mem,
                            0x0090_3bb0,
                            &[
                                0x0000_00c8,
                                x.to_bits(),
                                v717.to_bits(),
                                r713.to_bits(),
                                v711.to_bits(),
                                v706.to_bits(),
                                v705.to_bits(),
                                v704.to_bits(),
                                v703.to_bits(),
                            ],
                        );
                        let x718: f64 = f64::from(y) + f64::from(h);
                        let v719 = image_f32(mem, 0x00b5_f0d8);
                        let v720 = image_f32(mem, 0x00b6_6bb8);
                        let v721 = image_f32(mem, 0x00b6_6bd8);
                        let v722 = image_f32(mem, 0x00b6_6bb4);
                        let v723 = image_f32(mem, 0x00b5_f2e8);
                        let r724 = remap(mem, true, v723);
                        let v725 = image_f32(mem, 0x00b5_f2e8);
                        let r726 = remap(mem, false, v725);
                        let v727 = image_f32(mem, 0x00b5_f2e8);
                        let r728 = remap(mem, true, v727);
                        let x729: f64 = x718 - f64::from(r728);
                        let v730 = x729 as f32;
                        call(
                            mem,
                            0x0090_3bb0,
                            &[
                                0x0000_00c8,
                                x.to_bits(),
                                v730.to_bits(),
                                r726.to_bits(),
                                r724.to_bits(),
                                v722.to_bits(),
                                v721.to_bits(),
                                v720.to_bits(),
                                v719.to_bits(),
                            ],
                        );
                        let v731 = image_f32(mem, 0x00b6_6bc4);
                        let v732 = image_f32(mem, 0x00b6_5820);
                        let v733 = image_f32(mem, 0x00b6_6bc0);
                        let v734 = image_f32(mem, 0x00b6_6bb8);
                        let v735 = image_f32(mem, 0x00b5_f2e8);
                        let r736 = remap(mem, true, v735);
                        let d737: f64 = f64::from(w);
                        let v738 = image_f32(mem, 0x00b5_4f84);
                        let r739 = remap(mem, false, v738);
                        let x740: f64 = d737 - f64::from(r739);
                        let v741 = x740 as f32;
                        let v742 = image_f32(mem, 0x00b5_f2e8);
                        let r743 = remap(mem, false, v742);
                        let x744: f64 = f64::from(r743) + f64::from(x);
                        let v745 = x744 as f32;
                        call(
                            mem,
                            0x0090_3bb0,
                            &[
                                0x0000_00c8,
                                v745.to_bits(),
                                y.to_bits(),
                                v741.to_bits(),
                                r736.to_bits(),
                                v734.to_bits(),
                                v733.to_bits(),
                                v732.to_bits(),
                                v731.to_bits(),
                            ],
                        );
                        let v746 = image_f32(mem, 0x00b5_f0d8);
                        let v747 = image_f32(mem, 0x00b6_5820);
                        let v748 = image_f32(mem, 0x00b6_6bd8);
                        let v749 = image_f32(mem, 0x00b6_6bb8);
                        let v750 = image_f32(mem, 0x00b5_f2e8);
                        let r751 = remap(mem, true, v750);
                        let d752: f64 = f64::from(w);
                        let v753 = image_f32(mem, 0x00b5_4f84);
                        let r754 = remap(mem, false, v753);
                        let x755: f64 = d752 - f64::from(r754);
                        let v756 = x755 as f32;
                        let v757 = image_f32(mem, 0x00b5_f2e8);
                        let r758 = remap(mem, true, v757);
                        let x759: f64 = x718 - f64::from(r758);
                        let v760 = x759 as f32;
                        let v761 = image_f32(mem, 0x00b5_f2e8);
                        let r762 = remap(mem, false, v761);
                        let x763: f64 = f64::from(r762) + f64::from(x);
                        let v764 = x763 as f32;
                        call(
                            mem,
                            0x0090_3bb0,
                            &[
                                0x0000_00c8,
                                v764.to_bits(),
                                v760.to_bits(),
                                v756.to_bits(),
                                r751.to_bits(),
                                v749.to_bits(),
                                v748.to_bits(),
                                v747.to_bits(),
                                v746.to_bits(),
                            ],
                        );
                        let x765: f64 = f64::from(x) + f64::from(w);
                        let v766 = image_f32(mem, 0x00b6_6bc4);
                        let v767 = image_f32(mem, 0x00b6_6bd4);
                        let v768 = image_f32(mem, 0x00b6_6bc0);
                        let v769 = image_f32(mem, 0x00b6_5820);
                        let v770 = image_f32(mem, 0x00b5_f2e8);
                        let r771 = remap(mem, true, v770);
                        let v772 = image_f32(mem, 0x00b5_f2e8);
                        let r773 = remap(mem, false, v772);
                        let v774 = image_f32(mem, 0x00b5_f2e8);
                        let r775 = remap(mem, false, v774);
                        let x776: f64 = x765 - f64::from(r775);
                        let v777 = x776 as f32;
                        call(
                            mem,
                            0x0090_3bb0,
                            &[
                                0x0000_00c8,
                                v777.to_bits(),
                                y.to_bits(),
                                r773.to_bits(),
                                r771.to_bits(),
                                v769.to_bits(),
                                v768.to_bits(),
                                v767.to_bits(),
                                v766.to_bits(),
                            ],
                        );
                        let v778 = image_f32(mem, 0x00b6_6bd8);
                        let v779 = image_f32(mem, 0x00b6_6bd4);
                        let v780 = image_f32(mem, 0x00b6_6bc4);
                        let v781 = image_f32(mem, 0x00b6_5820);
                        let d782: f64 = f64::from(h);
                        let v783 = image_f32(mem, 0x00b5_4f84);
                        let r784 = remap(mem, true, v783);
                        let x785: f64 = d782 - f64::from(r784);
                        let v786 = x785 as f32;
                        let v787 = image_f32(mem, 0x00b5_f2e8);
                        let r788 = remap(mem, false, v787);
                        let v789 = image_f32(mem, 0x00b5_f2e8);
                        let r790 = remap(mem, true, v789);
                        let x791: f64 = f64::from(r790) + f64::from(y);
                        let v792 = x791 as f32;
                        let v793 = image_f32(mem, 0x00b5_f2e8);
                        let r794 = remap(mem, false, v793);
                        let x795: f64 = x765 - f64::from(r794);
                        let v796 = x795 as f32;
                        call(
                            mem,
                            0x0090_3bb0,
                            &[
                                0x0000_00c8,
                                v796.to_bits(),
                                v792.to_bits(),
                                r788.to_bits(),
                                v786.to_bits(),
                                v781.to_bits(),
                                v780.to_bits(),
                                v779.to_bits(),
                                v778.to_bits(),
                            ],
                        );
                        let v797 = image_f32(mem, 0x00b5_f0d8);
                        let v798 = image_f32(mem, 0x00b6_6bd4);
                        let v799 = image_f32(mem, 0x00b6_6bd8);
                        let v800 = image_f32(mem, 0x00b6_5820);
                        let v801 = image_f32(mem, 0x00b5_f2e8);
                        let r802 = remap(mem, true, v801);
                        let v803 = image_f32(mem, 0x00b5_f2e8);
                        let r804 = remap(mem, false, v803);
                        let v805 = image_f32(mem, 0x00b5_f2e8);
                        let r806 = remap(mem, true, v805);
                        let x807: f64 = x718 - f64::from(r806);
                        let v808 = x807 as f32;
                        let v809 = image_f32(mem, 0x00b5_f2e8);
                        let r810 = remap(mem, false, v809);
                        let x811: f64 = x765 - f64::from(r810);
                        let v812 = x811 as f32;
                        call(
                            mem,
                            0x0090_3bb0,
                            &[
                                0x0000_00c8,
                                v812.to_bits(),
                                v808.to_bits(),
                                r804.to_bits(),
                                r802.to_bits(),
                                v800.to_bits(),
                                v799.to_bits(),
                                v798.to_bits(),
                                v797.to_bits(),
                            ],
                        );
                    }
                }
            }
            10 => {
                let d1055: f64 = f64::from(w);
                let v1056 = image_f32(mem, 0x00b5_4f84);
                let r1057 = remap(mem, false, v1056);
                if (f64::from(r1057)) <= (d1055) || (f64::from(r1057)).is_nan() || (d1055).is_nan()
                {
                    let v1058 = image_f32(mem, 0x00b9_2308);
                    let r1059 = remap(mem, true, v1058);
                    let x1060: f64 = f64::from(r1059) + f64::from(y);
                    let v1061 = x1060 as f32;
                    let v1062 = image_f32(mem, 0x00b9_2308);
                    let r1063 = remap(mem, false, v1062);
                    let x1064: f64 = f64::from(r1063) + f64::from(x);
                    let v1065 = x1064 as f32;
                    call(
                        mem,
                        0x008f_4b20,
                        &[
                            v1065.to_bits(),
                            v1061.to_bits(),
                            w.to_bits(),
                            h.to_bits(),
                            0x401a_1a1a,
                            0x401a_1a1a,
                        ],
                    );
                    let x1066: f64 = f64::from(h) * img64(mem, 0x00b4_1d58);
                    let x1067: f64 = x1066 / img64(mem, 0x00b4_1d30);
                    if kind == 9 {
                        let v1311 = x1067 as f32;
                        let x1312: f64 = f64::from(w) * img64(mem, 0x00b4_1d68);
                        let x1313: f64 = x1312 * img64(mem, 0x00b4_1cc0);
                        let v1314 = x1313 as f32;
                        let v1315 = 0.0f32;
                        let v1316 = 0.0f32;
                        call(
                            mem,
                            0x0090_3ee0,
                            &[
                                0xffff_fffc,
                                x.to_bits(),
                                y.to_bits(),
                                w.to_bits(),
                                h.to_bits(),
                                v1316.to_bits(),
                                v1315.to_bits(),
                                v1314.to_bits(),
                                v1311.to_bits(),
                                0x8080_8080,
                                0x8080_8080,
                            ],
                        );
                        call(
                            mem,
                            0x008f_4df0,
                            &[
                                x.to_bits(),
                                y.to_bits(),
                                w.to_bits(),
                                h.to_bits(),
                                0x668f_4b2d,
                                0x6617_080a,
                            ],
                        );
                        let v1317 = image_f32(mem, 0x00b6_6bc4);
                        let v1318 = image_f32(mem, 0x00b6_6bb8);
                        let v1319 = image_f32(mem, 0x00b6_6bc0);
                        let v1320 = image_f32(mem, 0x00b6_6bb4);
                        let v1321 = image_f32(mem, 0x00b5_f2e8);
                        let r1322 = remap(mem, true, v1321);
                        let v1323 = image_f32(mem, 0x00b5_f2e8);
                        let r1324 = remap(mem, false, v1323);
                        call(
                            mem,
                            0x0090_3bb0,
                            &[
                                0x0000_00c8,
                                x.to_bits(),
                                y.to_bits(),
                                r1324.to_bits(),
                                r1322.to_bits(),
                                v1320.to_bits(),
                                v1319.to_bits(),
                                v1318.to_bits(),
                                v1317.to_bits(),
                            ],
                        );
                        let v1325 = image_f32(mem, 0x00b6_6bd8);
                        let v1326 = image_f32(mem, 0x00b6_6bb8);
                        let v1327 = image_f32(mem, 0x00b6_6bc4);
                        let v1328 = image_f32(mem, 0x00b6_6bb4);
                        let d1329: f64 = f64::from(h);
                        let v1330 = image_f32(mem, 0x00b5_4f84);
                        let r1331 = remap(mem, true, v1330);
                        let x1332: f64 = d1329 - f64::from(r1331);
                        let v1333 = x1332 as f32;
                        let v1334 = image_f32(mem, 0x00b5_f2e8);
                        let r1335 = remap(mem, false, v1334);
                        let v1336 = image_f32(mem, 0x00b5_f2e8);
                        let r1337 = remap(mem, true, v1336);
                        let x1338: f64 = f64::from(r1337) + f64::from(y);
                        let v1339 = x1338 as f32;
                        call(
                            mem,
                            0x0090_3bb0,
                            &[
                                0x0000_00c8,
                                x.to_bits(),
                                v1339.to_bits(),
                                r1335.to_bits(),
                                v1333.to_bits(),
                                v1328.to_bits(),
                                v1327.to_bits(),
                                v1326.to_bits(),
                                v1325.to_bits(),
                            ],
                        );
                        let x1340: f64 = f64::from(y) + f64::from(h);
                        let v1341 = image_f32(mem, 0x00b5_f0d8);
                        let v1342 = image_f32(mem, 0x00b6_6bb8);
                        let v1343 = image_f32(mem, 0x00b6_6bd8);
                        let v1344 = image_f32(mem, 0x00b6_6bb4);
                        let v1345 = image_f32(mem, 0x00b5_f2e8);
                        let r1346 = remap(mem, true, v1345);
                        let v1347 = image_f32(mem, 0x00b5_f2e8);
                        let r1348 = remap(mem, false, v1347);
                        let v1349 = image_f32(mem, 0x00b5_f2e8);
                        let r1350 = remap(mem, true, v1349);
                        let x1351: f64 = x1340 - f64::from(r1350);
                        let v1352 = x1351 as f32;
                        call(
                            mem,
                            0x0090_3bb0,
                            &[
                                0x0000_00c8,
                                x.to_bits(),
                                v1352.to_bits(),
                                r1348.to_bits(),
                                r1346.to_bits(),
                                v1344.to_bits(),
                                v1343.to_bits(),
                                v1342.to_bits(),
                                v1341.to_bits(),
                            ],
                        );
                        let v1353 = image_f32(mem, 0x00b6_6bc4);
                        let v1354 = image_f32(mem, 0x00b6_5820);
                        let v1355 = image_f32(mem, 0x00b6_6bc0);
                        let v1356 = image_f32(mem, 0x00b6_6bb8);
                        let v1357 = image_f32(mem, 0x00b5_f2e8);
                        let r1358 = remap(mem, true, v1357);
                        let d1359: f64 = f64::from(w);
                        let v1360 = image_f32(mem, 0x00b5_4f84);
                        let r1361 = remap(mem, false, v1360);
                        let x1362: f64 = d1359 - f64::from(r1361);
                        let v1363 = x1362 as f32;
                        let v1364 = image_f32(mem, 0x00b5_f2e8);
                        let r1365 = remap(mem, false, v1364);
                        let x1366: f64 = f64::from(r1365) + f64::from(x);
                        let v1367 = x1366 as f32;
                        call(
                            mem,
                            0x0090_3bb0,
                            &[
                                0x0000_00c8,
                                v1367.to_bits(),
                                y.to_bits(),
                                v1363.to_bits(),
                                r1358.to_bits(),
                                v1356.to_bits(),
                                v1355.to_bits(),
                                v1354.to_bits(),
                                v1353.to_bits(),
                            ],
                        );
                        let v1368 = image_f32(mem, 0x00b5_f0d8);
                        let v1369 = image_f32(mem, 0x00b6_5820);
                        let v1370 = image_f32(mem, 0x00b6_6bd8);
                        let v1371 = image_f32(mem, 0x00b6_6bb8);
                        let v1372 = image_f32(mem, 0x00b5_f2e8);
                        let r1373 = remap(mem, true, v1372);
                        let d1374: f64 = f64::from(w);
                        let v1375 = image_f32(mem, 0x00b5_4f84);
                        let r1376 = remap(mem, false, v1375);
                        let x1377: f64 = d1374 - f64::from(r1376);
                        let v1378 = x1377 as f32;
                        let v1379 = image_f32(mem, 0x00b5_f2e8);
                        let r1380 = remap(mem, true, v1379);
                        let x1381: f64 = x1340 - f64::from(r1380);
                        let v1382 = x1381 as f32;
                        let v1383 = image_f32(mem, 0x00b5_f2e8);
                        let r1384 = remap(mem, false, v1383);
                        let x1385: f64 = f64::from(r1384) + f64::from(x);
                        let v1386 = x1385 as f32;
                        call(
                            mem,
                            0x0090_3bb0,
                            &[
                                0x0000_00c8,
                                v1386.to_bits(),
                                v1382.to_bits(),
                                v1378.to_bits(),
                                r1373.to_bits(),
                                v1371.to_bits(),
                                v1370.to_bits(),
                                v1369.to_bits(),
                                v1368.to_bits(),
                            ],
                        );
                        let x1387: f64 = f64::from(x) + f64::from(w);
                        let v1388 = image_f32(mem, 0x00b6_6bc4);
                        let v1389 = image_f32(mem, 0x00b6_6bd4);
                        let v1390 = image_f32(mem, 0x00b6_6bc0);
                        let v1391 = image_f32(mem, 0x00b6_5820);
                        let v1392 = image_f32(mem, 0x00b5_f2e8);
                        let r1393 = remap(mem, true, v1392);
                        let v1394 = image_f32(mem, 0x00b5_f2e8);
                        let r1395 = remap(mem, false, v1394);
                        let v1396 = image_f32(mem, 0x00b5_f2e8);
                        let r1397 = remap(mem, false, v1396);
                        let x1398: f64 = x1387 - f64::from(r1397);
                        let v1399 = x1398 as f32;
                        call(
                            mem,
                            0x0090_3bb0,
                            &[
                                0x0000_00c8,
                                v1399.to_bits(),
                                y.to_bits(),
                                r1395.to_bits(),
                                r1393.to_bits(),
                                v1391.to_bits(),
                                v1390.to_bits(),
                                v1389.to_bits(),
                                v1388.to_bits(),
                            ],
                        );
                        let v1400 = image_f32(mem, 0x00b6_6bd8);
                        let v1401 = image_f32(mem, 0x00b6_6bd4);
                        let v1402 = image_f32(mem, 0x00b6_6bc4);
                        let v1403 = image_f32(mem, 0x00b6_5820);
                        let d1404: f64 = f64::from(h);
                        let v1405 = image_f32(mem, 0x00b5_4f84);
                        let r1406 = remap(mem, true, v1405);
                        let x1407: f64 = d1404 - f64::from(r1406);
                        let v1408 = x1407 as f32;
                        let v1409 = image_f32(mem, 0x00b5_f2e8);
                        let r1410 = remap(mem, false, v1409);
                        let v1411 = image_f32(mem, 0x00b5_f2e8);
                        let r1412 = remap(mem, true, v1411);
                        let x1413: f64 = f64::from(r1412) + f64::from(y);
                        let v1414 = x1413 as f32;
                        let v1415 = image_f32(mem, 0x00b5_f2e8);
                        let r1416 = remap(mem, false, v1415);
                        let x1417: f64 = x1387 - f64::from(r1416);
                        let v1418 = x1417 as f32;
                        call(
                            mem,
                            0x0090_3bb0,
                            &[
                                0x0000_00c8,
                                v1418.to_bits(),
                                v1414.to_bits(),
                                r1410.to_bits(),
                                v1408.to_bits(),
                                v1403.to_bits(),
                                v1402.to_bits(),
                                v1401.to_bits(),
                                v1400.to_bits(),
                            ],
                        );
                        let v1419 = image_f32(mem, 0x00b5_f0d8);
                        let v1420 = image_f32(mem, 0x00b6_6bd4);
                        let v1421 = image_f32(mem, 0x00b6_6bd8);
                        let v1422 = image_f32(mem, 0x00b6_5820);
                        let v1423 = image_f32(mem, 0x00b5_f2e8);
                        let r1424 = remap(mem, true, v1423);
                        let v1425 = image_f32(mem, 0x00b5_f2e8);
                        let r1426 = remap(mem, false, v1425);
                        let v1427 = image_f32(mem, 0x00b5_f2e8);
                        let r1428 = remap(mem, true, v1427);
                        let x1429: f64 = x1340 - f64::from(r1428);
                        let v1430 = x1429 as f32;
                        let v1431 = image_f32(mem, 0x00b5_f2e8);
                        let r1432 = remap(mem, false, v1431);
                        let x1433: f64 = x1387 - f64::from(r1432);
                        let v1434 = x1433 as f32;
                        call(
                            mem,
                            0x0090_3bb0,
                            &[
                                0x0000_00c8,
                                v1434.to_bits(),
                                v1430.to_bits(),
                                r1426.to_bits(),
                                r1424.to_bits(),
                                v1422.to_bits(),
                                v1421.to_bits(),
                                v1420.to_bits(),
                                v1419.to_bits(),
                            ],
                        );
                    } else {
                        let v1068 = x1067 as f32;
                        let x1069: f64 = f64::from(w) * img64(mem, 0x00b4_1d68);
                        let x1070: f64 = x1069 * img64(mem, 0x00b4_1cc0);
                        let v1071 = x1070 as f32;
                        let v1072 = 0.0f32;
                        let v1073 = 0.0f32;
                        call(
                            mem,
                            0x0090_3ee0,
                            &[
                                0xffff_fffc,
                                x.to_bits(),
                                y.to_bits(),
                                w.to_bits(),
                                h.to_bits(),
                                v1073.to_bits(),
                                v1072.to_bits(),
                                v1071.to_bits(),
                                v1068.to_bits(),
                                0x4080_8080,
                                0x4080_8080,
                            ],
                        );
                        let e1074 = call(mem, 0x0064_1060, &[]);
                        if e1074 == 0 {
                            call(
                                mem,
                                0x008f_4df0,
                                &[
                                    x.to_bits(),
                                    y.to_bits(),
                                    w.to_bits(),
                                    h.to_bits(),
                                    0x608f_2c45,
                                    0x6050_1826,
                                ],
                            );
                            let v1193 = image_f32(mem, 0x00b6_6bc4);
                            let v1194 = image_f32(mem, 0x00b6_6bb8);
                            let v1195 = image_f32(mem, 0x00b6_6bc0);
                            let v1196 = image_f32(mem, 0x00b6_6bb4);
                            let v1197 = image_f32(mem, 0x00b5_f2e8);
                            let r1198 = remap(mem, true, v1197);
                            let v1199 = image_f32(mem, 0x00b5_f2e8);
                            let r1200 = remap(mem, false, v1199);
                            call(
                                mem,
                                0x0090_3bb0,
                                &[
                                    0x0000_00c8,
                                    x.to_bits(),
                                    y.to_bits(),
                                    r1200.to_bits(),
                                    r1198.to_bits(),
                                    v1196.to_bits(),
                                    v1195.to_bits(),
                                    v1194.to_bits(),
                                    v1193.to_bits(),
                                ],
                            );
                            let v1201 = image_f32(mem, 0x00b6_6bd8);
                            let v1202 = image_f32(mem, 0x00b6_6bb8);
                            let v1203 = image_f32(mem, 0x00b6_6bc4);
                            let v1204 = image_f32(mem, 0x00b6_6bb4);
                            let d1205: f64 = f64::from(h);
                            let v1206 = image_f32(mem, 0x00b5_4f84);
                            let r1207 = remap(mem, true, v1206);
                            let x1208: f64 = d1205 - f64::from(r1207);
                            let v1209 = x1208 as f32;
                            let v1210 = image_f32(mem, 0x00b5_f2e8);
                            let r1211 = remap(mem, false, v1210);
                            let v1212 = image_f32(mem, 0x00b5_f2e8);
                            let r1213 = remap(mem, true, v1212);
                            let x1214: f64 = f64::from(r1213) + f64::from(y);
                            let v1215 = x1214 as f32;
                            call(
                                mem,
                                0x0090_3bb0,
                                &[
                                    0x0000_00c8,
                                    x.to_bits(),
                                    v1215.to_bits(),
                                    r1211.to_bits(),
                                    v1209.to_bits(),
                                    v1204.to_bits(),
                                    v1203.to_bits(),
                                    v1202.to_bits(),
                                    v1201.to_bits(),
                                ],
                            );
                            let x1216: f64 = f64::from(y) + f64::from(h);
                            let v1217 = image_f32(mem, 0x00b5_f0d8);
                            let v1218 = image_f32(mem, 0x00b6_6bb8);
                            let v1219 = image_f32(mem, 0x00b6_6bd8);
                            let v1220 = image_f32(mem, 0x00b6_6bb4);
                            let v1221 = image_f32(mem, 0x00b5_f2e8);
                            let r1222 = remap(mem, true, v1221);
                            let v1223 = image_f32(mem, 0x00b5_f2e8);
                            let r1224 = remap(mem, false, v1223);
                            let v1225 = image_f32(mem, 0x00b5_f2e8);
                            let r1226 = remap(mem, true, v1225);
                            let x1227: f64 = x1216 - f64::from(r1226);
                            let v1228 = x1227 as f32;
                            call(
                                mem,
                                0x0090_3bb0,
                                &[
                                    0x0000_00c8,
                                    x.to_bits(),
                                    v1228.to_bits(),
                                    r1224.to_bits(),
                                    r1222.to_bits(),
                                    v1220.to_bits(),
                                    v1219.to_bits(),
                                    v1218.to_bits(),
                                    v1217.to_bits(),
                                ],
                            );
                            let v1229 = image_f32(mem, 0x00b6_6bc4);
                            let v1230 = image_f32(mem, 0x00b6_5820);
                            let v1231 = image_f32(mem, 0x00b6_6bc0);
                            let v1232 = image_f32(mem, 0x00b6_6bb8);
                            let v1233 = image_f32(mem, 0x00b5_f2e8);
                            let r1234 = remap(mem, true, v1233);
                            let d1235: f64 = f64::from(w);
                            let v1236 = image_f32(mem, 0x00b5_4f84);
                            let r1237 = remap(mem, false, v1236);
                            let x1238: f64 = d1235 - f64::from(r1237);
                            let v1239 = x1238 as f32;
                            let v1240 = image_f32(mem, 0x00b5_f2e8);
                            let r1241 = remap(mem, false, v1240);
                            let x1242: f64 = f64::from(r1241) + f64::from(x);
                            let v1243 = x1242 as f32;
                            call(
                                mem,
                                0x0090_3bb0,
                                &[
                                    0x0000_00c8,
                                    v1243.to_bits(),
                                    y.to_bits(),
                                    v1239.to_bits(),
                                    r1234.to_bits(),
                                    v1232.to_bits(),
                                    v1231.to_bits(),
                                    v1230.to_bits(),
                                    v1229.to_bits(),
                                ],
                            );
                            let v1244 = image_f32(mem, 0x00b5_f0d8);
                            let v1245 = image_f32(mem, 0x00b6_5820);
                            let v1246 = image_f32(mem, 0x00b6_6bd8);
                            let v1247 = image_f32(mem, 0x00b6_6bb8);
                            let v1248 = image_f32(mem, 0x00b5_f2e8);
                            let r1249 = remap(mem, true, v1248);
                            let d1250: f64 = f64::from(w);
                            let v1251 = image_f32(mem, 0x00b5_4f84);
                            let r1252 = remap(mem, false, v1251);
                            let x1253: f64 = d1250 - f64::from(r1252);
                            let v1254 = x1253 as f32;
                            let v1255 = image_f32(mem, 0x00b5_f2e8);
                            let r1256 = remap(mem, true, v1255);
                            let x1257: f64 = x1216 - f64::from(r1256);
                            let v1258 = x1257 as f32;
                            let v1259 = image_f32(mem, 0x00b5_f2e8);
                            let r1260 = remap(mem, false, v1259);
                            let x1261: f64 = f64::from(r1260) + f64::from(x);
                            let v1262 = x1261 as f32;
                            call(
                                mem,
                                0x0090_3bb0,
                                &[
                                    0x0000_00c8,
                                    v1262.to_bits(),
                                    v1258.to_bits(),
                                    v1254.to_bits(),
                                    r1249.to_bits(),
                                    v1247.to_bits(),
                                    v1246.to_bits(),
                                    v1245.to_bits(),
                                    v1244.to_bits(),
                                ],
                            );
                            let x1263: f64 = f64::from(x) + f64::from(w);
                            let v1264 = image_f32(mem, 0x00b6_6bc4);
                            let v1265 = image_f32(mem, 0x00b6_6bd4);
                            let v1266 = image_f32(mem, 0x00b6_6bc0);
                            let v1267 = image_f32(mem, 0x00b6_5820);
                            let v1268 = image_f32(mem, 0x00b5_f2e8);
                            let r1269 = remap(mem, true, v1268);
                            let v1270 = image_f32(mem, 0x00b5_f2e8);
                            let r1271 = remap(mem, false, v1270);
                            let v1272 = image_f32(mem, 0x00b5_f2e8);
                            let r1273 = remap(mem, false, v1272);
                            let x1274: f64 = x1263 - f64::from(r1273);
                            let v1275 = x1274 as f32;
                            call(
                                mem,
                                0x0090_3bb0,
                                &[
                                    0x0000_00c8,
                                    v1275.to_bits(),
                                    y.to_bits(),
                                    r1271.to_bits(),
                                    r1269.to_bits(),
                                    v1267.to_bits(),
                                    v1266.to_bits(),
                                    v1265.to_bits(),
                                    v1264.to_bits(),
                                ],
                            );
                            let v1276 = image_f32(mem, 0x00b6_6bd8);
                            let v1277 = image_f32(mem, 0x00b6_6bd4);
                            let v1278 = image_f32(mem, 0x00b6_6bc4);
                            let v1279 = image_f32(mem, 0x00b6_5820);
                            let d1280: f64 = f64::from(h);
                            let v1281 = image_f32(mem, 0x00b5_4f84);
                            let r1282 = remap(mem, true, v1281);
                            let x1283: f64 = d1280 - f64::from(r1282);
                            let v1284 = x1283 as f32;
                            let v1285 = image_f32(mem, 0x00b5_f2e8);
                            let r1286 = remap(mem, false, v1285);
                            let v1287 = image_f32(mem, 0x00b5_f2e8);
                            let r1288 = remap(mem, true, v1287);
                            let x1289: f64 = f64::from(r1288) + f64::from(y);
                            let v1290 = x1289 as f32;
                            let v1291 = image_f32(mem, 0x00b5_f2e8);
                            let r1292 = remap(mem, false, v1291);
                            let x1293: f64 = x1263 - f64::from(r1292);
                            let v1294 = x1293 as f32;
                            call(
                                mem,
                                0x0090_3bb0,
                                &[
                                    0x0000_00c8,
                                    v1294.to_bits(),
                                    v1290.to_bits(),
                                    r1286.to_bits(),
                                    v1284.to_bits(),
                                    v1279.to_bits(),
                                    v1278.to_bits(),
                                    v1277.to_bits(),
                                    v1276.to_bits(),
                                ],
                            );
                            let v1295 = image_f32(mem, 0x00b5_f0d8);
                            let v1296 = image_f32(mem, 0x00b6_6bd4);
                            let v1297 = image_f32(mem, 0x00b6_6bd8);
                            let v1298 = image_f32(mem, 0x00b6_5820);
                            let v1299 = image_f32(mem, 0x00b5_f2e8);
                            let r1300 = remap(mem, true, v1299);
                            let v1301 = image_f32(mem, 0x00b5_f2e8);
                            let r1302 = remap(mem, false, v1301);
                            let v1303 = image_f32(mem, 0x00b5_f2e8);
                            let r1304 = remap(mem, true, v1303);
                            let x1305: f64 = x1216 - f64::from(r1304);
                            let v1306 = x1305 as f32;
                            let v1307 = image_f32(mem, 0x00b5_f2e8);
                            let r1308 = remap(mem, false, v1307);
                            let x1309: f64 = x1263 - f64::from(r1308);
                            let v1310 = x1309 as f32;
                            call(
                                mem,
                                0x0090_3bb0,
                                &[
                                    0x0000_00c8,
                                    v1310.to_bits(),
                                    v1306.to_bits(),
                                    r1302.to_bits(),
                                    r1300.to_bits(),
                                    v1298.to_bits(),
                                    v1297.to_bits(),
                                    v1296.to_bits(),
                                    v1295.to_bits(),
                                ],
                            );
                        } else {
                            call(
                                mem,
                                0x008f_4df0,
                                &[
                                    x.to_bits(),
                                    y.to_bits(),
                                    w.to_bits(),
                                    h.to_bits(),
                                    0x6034_0614,
                                    0x6030_0410,
                                ],
                            );
                            let v1075 = image_f32(mem, 0x00b6_6bc4);
                            let v1076 = image_f32(mem, 0x00b6_6bb8);
                            let v1077 = image_f32(mem, 0x00b6_6bc0);
                            let v1078 = image_f32(mem, 0x00b6_6bb4);
                            let v1079 = image_f32(mem, 0x00b5_f2e8);
                            let r1080 = remap(mem, true, v1079);
                            let v1081 = image_f32(mem, 0x00b5_f2e8);
                            let r1082 = remap(mem, false, v1081);
                            call(
                                mem,
                                0x0090_3bb0,
                                &[
                                    0x0000_00c8,
                                    x.to_bits(),
                                    y.to_bits(),
                                    r1082.to_bits(),
                                    r1080.to_bits(),
                                    v1078.to_bits(),
                                    v1077.to_bits(),
                                    v1076.to_bits(),
                                    v1075.to_bits(),
                                ],
                            );
                            let v1083 = image_f32(mem, 0x00b6_6bd8);
                            let v1084 = image_f32(mem, 0x00b6_6bb8);
                            let v1085 = image_f32(mem, 0x00b6_6bc4);
                            let v1086 = image_f32(mem, 0x00b6_6bb4);
                            let d1087: f64 = f64::from(h);
                            let v1088 = image_f32(mem, 0x00b5_4f84);
                            let r1089 = remap(mem, true, v1088);
                            let x1090: f64 = d1087 - f64::from(r1089);
                            let v1091 = x1090 as f32;
                            let v1092 = image_f32(mem, 0x00b5_f2e8);
                            let r1093 = remap(mem, false, v1092);
                            let v1094 = image_f32(mem, 0x00b5_f2e8);
                            let r1095 = remap(mem, true, v1094);
                            let x1096: f64 = f64::from(r1095) + f64::from(y);
                            let v1097 = x1096 as f32;
                            call(
                                mem,
                                0x0090_3bb0,
                                &[
                                    0x0000_00c8,
                                    x.to_bits(),
                                    v1097.to_bits(),
                                    r1093.to_bits(),
                                    v1091.to_bits(),
                                    v1086.to_bits(),
                                    v1085.to_bits(),
                                    v1084.to_bits(),
                                    v1083.to_bits(),
                                ],
                            );
                            let x1098: f64 = f64::from(y) + f64::from(h);
                            let v1099 = image_f32(mem, 0x00b5_f0d8);
                            let v1100 = image_f32(mem, 0x00b6_6bb8);
                            let v1101 = image_f32(mem, 0x00b6_6bd8);
                            let v1102 = image_f32(mem, 0x00b6_6bb4);
                            let v1103 = image_f32(mem, 0x00b5_f2e8);
                            let r1104 = remap(mem, true, v1103);
                            let v1105 = image_f32(mem, 0x00b5_f2e8);
                            let r1106 = remap(mem, false, v1105);
                            let v1107 = image_f32(mem, 0x00b5_f2e8);
                            let r1108 = remap(mem, true, v1107);
                            let x1109: f64 = x1098 - f64::from(r1108);
                            let v1110 = x1109 as f32;
                            call(
                                mem,
                                0x0090_3bb0,
                                &[
                                    0x0000_00c8,
                                    x.to_bits(),
                                    v1110.to_bits(),
                                    r1106.to_bits(),
                                    r1104.to_bits(),
                                    v1102.to_bits(),
                                    v1101.to_bits(),
                                    v1100.to_bits(),
                                    v1099.to_bits(),
                                ],
                            );
                            let v1111 = image_f32(mem, 0x00b6_6bc4);
                            let v1112 = image_f32(mem, 0x00b6_5820);
                            let v1113 = image_f32(mem, 0x00b6_6bc0);
                            let v1114 = image_f32(mem, 0x00b6_6bb8);
                            let v1115 = image_f32(mem, 0x00b5_f2e8);
                            let r1116 = remap(mem, true, v1115);
                            let d1117: f64 = f64::from(w);
                            let v1118 = image_f32(mem, 0x00b5_4f84);
                            let r1119 = remap(mem, false, v1118);
                            let x1120: f64 = d1117 - f64::from(r1119);
                            let v1121 = x1120 as f32;
                            let v1122 = image_f32(mem, 0x00b5_f2e8);
                            let r1123 = remap(mem, false, v1122);
                            let x1124: f64 = f64::from(r1123) + f64::from(x);
                            let v1125 = x1124 as f32;
                            call(
                                mem,
                                0x0090_3bb0,
                                &[
                                    0x0000_00c8,
                                    v1125.to_bits(),
                                    y.to_bits(),
                                    v1121.to_bits(),
                                    r1116.to_bits(),
                                    v1114.to_bits(),
                                    v1113.to_bits(),
                                    v1112.to_bits(),
                                    v1111.to_bits(),
                                ],
                            );
                            let v1126 = image_f32(mem, 0x00b5_f0d8);
                            let v1127 = image_f32(mem, 0x00b6_5820);
                            let v1128 = image_f32(mem, 0x00b6_6bd8);
                            let v1129 = image_f32(mem, 0x00b6_6bb8);
                            let v1130 = image_f32(mem, 0x00b5_f2e8);
                            let r1131 = remap(mem, true, v1130);
                            let d1132: f64 = f64::from(w);
                            let v1133 = image_f32(mem, 0x00b5_4f84);
                            let r1134 = remap(mem, false, v1133);
                            let x1135: f64 = d1132 - f64::from(r1134);
                            let v1136 = x1135 as f32;
                            let v1137 = image_f32(mem, 0x00b5_f2e8);
                            let r1138 = remap(mem, true, v1137);
                            let x1139: f64 = x1098 - f64::from(r1138);
                            let v1140 = x1139 as f32;
                            let v1141 = image_f32(mem, 0x00b5_f2e8);
                            let r1142 = remap(mem, false, v1141);
                            let x1143: f64 = f64::from(r1142) + f64::from(x);
                            let v1144 = x1143 as f32;
                            call(
                                mem,
                                0x0090_3bb0,
                                &[
                                    0x0000_00c8,
                                    v1144.to_bits(),
                                    v1140.to_bits(),
                                    v1136.to_bits(),
                                    r1131.to_bits(),
                                    v1129.to_bits(),
                                    v1128.to_bits(),
                                    v1127.to_bits(),
                                    v1126.to_bits(),
                                ],
                            );
                            let x1145: f64 = f64::from(x) + f64::from(w);
                            let v1146 = image_f32(mem, 0x00b6_6bc4);
                            let v1147 = image_f32(mem, 0x00b6_6bd4);
                            let v1148 = image_f32(mem, 0x00b6_6bc0);
                            let v1149 = image_f32(mem, 0x00b6_5820);
                            let v1150 = image_f32(mem, 0x00b5_f2e8);
                            let r1151 = remap(mem, true, v1150);
                            let v1152 = image_f32(mem, 0x00b5_f2e8);
                            let r1153 = remap(mem, false, v1152);
                            let v1154 = image_f32(mem, 0x00b5_f2e8);
                            let r1155 = remap(mem, false, v1154);
                            let x1156: f64 = x1145 - f64::from(r1155);
                            let v1157 = x1156 as f32;
                            call(
                                mem,
                                0x0090_3bb0,
                                &[
                                    0x0000_00c8,
                                    v1157.to_bits(),
                                    y.to_bits(),
                                    r1153.to_bits(),
                                    r1151.to_bits(),
                                    v1149.to_bits(),
                                    v1148.to_bits(),
                                    v1147.to_bits(),
                                    v1146.to_bits(),
                                ],
                            );
                            let v1158 = image_f32(mem, 0x00b6_6bd8);
                            let v1159 = image_f32(mem, 0x00b6_6bd4);
                            let v1160 = image_f32(mem, 0x00b6_6bc4);
                            let v1161 = image_f32(mem, 0x00b6_5820);
                            let d1162: f64 = f64::from(h);
                            let v1163 = image_f32(mem, 0x00b5_4f84);
                            let r1164 = remap(mem, true, v1163);
                            let x1165: f64 = d1162 - f64::from(r1164);
                            let v1166 = x1165 as f32;
                            let v1167 = image_f32(mem, 0x00b5_f2e8);
                            let r1168 = remap(mem, false, v1167);
                            let v1169 = image_f32(mem, 0x00b5_f2e8);
                            let r1170 = remap(mem, true, v1169);
                            let x1171: f64 = f64::from(r1170) + f64::from(y);
                            let v1172 = x1171 as f32;
                            let v1173 = image_f32(mem, 0x00b5_f2e8);
                            let r1174 = remap(mem, false, v1173);
                            let x1175: f64 = x1145 - f64::from(r1174);
                            let v1176 = x1175 as f32;
                            call(
                                mem,
                                0x0090_3bb0,
                                &[
                                    0x0000_00c8,
                                    v1176.to_bits(),
                                    v1172.to_bits(),
                                    r1168.to_bits(),
                                    v1166.to_bits(),
                                    v1161.to_bits(),
                                    v1160.to_bits(),
                                    v1159.to_bits(),
                                    v1158.to_bits(),
                                ],
                            );
                            let v1177 = image_f32(mem, 0x00b5_f0d8);
                            let v1178 = image_f32(mem, 0x00b6_6bd4);
                            let v1179 = image_f32(mem, 0x00b6_6bd8);
                            let v1180 = image_f32(mem, 0x00b6_5820);
                            let v1181 = image_f32(mem, 0x00b5_f2e8);
                            let r1182 = remap(mem, true, v1181);
                            let v1183 = image_f32(mem, 0x00b5_f2e8);
                            let r1184 = remap(mem, false, v1183);
                            let v1185 = image_f32(mem, 0x00b5_f2e8);
                            let r1186 = remap(mem, true, v1185);
                            let x1187: f64 = x1098 - f64::from(r1186);
                            let v1188 = x1187 as f32;
                            let v1189 = image_f32(mem, 0x00b5_f2e8);
                            let r1190 = remap(mem, false, v1189);
                            let x1191: f64 = x1145 - f64::from(r1190);
                            let v1192 = x1191 as f32;
                            call(
                                mem,
                                0x0090_3bb0,
                                &[
                                    0x0000_00c8,
                                    v1192.to_bits(),
                                    v1188.to_bits(),
                                    r1184.to_bits(),
                                    r1182.to_bits(),
                                    v1180.to_bits(),
                                    v1179.to_bits(),
                                    v1178.to_bits(),
                                    v1177.to_bits(),
                                ],
                            );
                        }
                    }
                }
            }
            11 => {
                let x1435: f64 = f64::from(y) + f64::from(h);
                let v1436 = x1435 as f32;
                let v1437 = image_f32(mem, 0x00b9_22f8);
                let r1438 = remap(mem, true, v1437);
                let v1439 = image_f32(mem, 0x00b4_1d8c);
                let r1440 = remap(mem, false, v1439);
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        x.to_bits(),
                        v1436.to_bits(),
                        r1440.to_bits(),
                        r1438.to_bits(),
                        0x0000_0000,
                        0x2600_0000,
                    ],
                );
                let v1441 = image_f32(mem, 0x00b9_22f8);
                let r1442 = remap(mem, true, v1441);
                let d1443: f64 = f64::from(w);
                let v1444 = image_f32(mem, 0x00b5_f384);
                let r1445 = remap(mem, false, v1444);
                let x1446: f64 = d1443 - f64::from(r1445);
                let v1447 = x1446 as f32;
                let v1448 = image_f32(mem, 0x00b4_1d8c);
                let r1449 = remap(mem, false, v1448);
                let x1450: f64 = f64::from(r1449) + f64::from(x);
                let v1451 = x1450 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        v1451.to_bits(),
                        v1436.to_bits(),
                        v1447.to_bits(),
                        r1442.to_bits(),
                        0x2600_0000,
                        0x2600_0000,
                    ],
                );
                let x1452: f64 = f64::from(x) + f64::from(w);
                let v1453 = image_f32(mem, 0x00b9_22f8);
                let r1454 = remap(mem, true, v1453);
                let v1455 = image_f32(mem, 0x00b4_1d8c);
                let r1456 = remap(mem, false, v1455);
                let v1457 = image_f32(mem, 0x00b4_1d8c);
                let r1458 = remap(mem, false, v1457);
                let x1459: f64 = x1452 - f64::from(r1458);
                let v1460 = x1459 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        v1460.to_bits(),
                        v1436.to_bits(),
                        r1456.to_bits(),
                        r1454.to_bits(),
                        0x2600_0000,
                        0x0000_0000,
                    ],
                );
                let v1461 = image_f32(mem, 0x00b4_1d8c);
                let r1462 = remap(mem, false, v1461);
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        x.to_bits(),
                        y.to_bits(),
                        r1462.to_bits(),
                        h.to_bits(),
                        0x0070_3140,
                        0x8070_3140,
                    ],
                );
                let d1463: f64 = f64::from(w);
                let v1464 = image_f32(mem, 0x00b5_f384);
                let r1465 = remap(mem, false, v1464);
                let x1466: f64 = d1463 - f64::from(r1465);
                let v1467 = x1466 as f32;
                let v1468 = image_f32(mem, 0x00b4_1d8c);
                let r1469 = remap(mem, false, v1468);
                let x1470: f64 = f64::from(r1469) + f64::from(x);
                let v1471 = x1470 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        v1471.to_bits(),
                        y.to_bits(),
                        v1467.to_bits(),
                        h.to_bits(),
                        0x8070_3140,
                        0x8070_3140,
                    ],
                );
                let v1472 = image_f32(mem, 0x00b4_1d8c);
                let r1473 = remap(mem, false, v1472);
                let v1474 = image_f32(mem, 0x00b4_1d8c);
                let r1475 = remap(mem, false, v1474);
                let x1476: f64 = x1452 - f64::from(r1475);
                let v1477 = x1476 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        v1477.to_bits(),
                        y.to_bits(),
                        r1473.to_bits(),
                        h.to_bits(),
                        0x8070_3140,
                        0x0070_3140,
                    ],
                );
                let v1478 = image_f32(mem, 0x00b6_5c00);
                let v1479 = image_f32(mem, 0x00b6_5c20);
                let v1480 = image_f32(mem, 0x00b4_9980);
                let v1481 = image_f32(mem, 0x00b6_5c1c);
                let v1482 = image_f32(mem, 0x00b5_eea4);
                let r1483 = remap(mem, true, v1482);
                let v1484 = image_f32(mem, 0x00b6_5b18);
                let r1485 = remap(mem, false, v1484);
                let v1486 = image_f32(mem, 0x00b3_eb94);
                let r1487 = remap(mem, true, v1486);
                let x1488: f64 = f64::from(r1487) + f64::from(y);
                let v1489 = x1488 as f32;
                let v1490 = image_f32(mem, 0x00b6_6dc4);
                let r1491 = remap(mem, false, v1490);
                let x1492: f64 = f64::from(r1491) + f64::from(x);
                let v1493 = x1492 as f32;
                call(
                    mem,
                    0x0090_3ee0,
                    &[
                        0xffff_ffff,
                        v1493.to_bits(),
                        v1489.to_bits(),
                        r1485.to_bits(),
                        r1483.to_bits(),
                        v1481.to_bits(),
                        v1480.to_bits(),
                        v1479.to_bits(),
                        v1478.to_bits(),
                        0x8080_8080,
                        0x8080_8080,
                    ],
                );
                let v1494 = image_f32(mem, 0x00b9_2268);
                let r1495 = remap(mem, true, v1494);
                let v1496 = image_f32(mem, 0x00b4_1d8c);
                let r1497 = remap(mem, false, v1496);
                let v1498 = 1.0f32;
                let r1499 = remap(mem, true, v1498);
                let x1500: f64 = f64::from(r1499) + f64::from(y);
                let v1501 = x1500 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        x.to_bits(),
                        v1501.to_bits(),
                        r1497.to_bits(),
                        r1495.to_bits(),
                        0x00ff_ffff,
                        0x5aff_ffff,
                    ],
                );
                let v1502 = image_f32(mem, 0x00b9_2268);
                let r1503 = remap(mem, true, v1502);
                let d1504: f64 = f64::from(w);
                let v1505 = image_f32(mem, 0x00b5_f384);
                let r1506 = remap(mem, false, v1505);
                let x1507: f64 = d1504 - f64::from(r1506);
                let v1508 = x1507 as f32;
                let v1509 = 1.0f32;
                let r1510 = remap(mem, true, v1509);
                let x1511: f64 = f64::from(r1510) + f64::from(y);
                let v1512 = x1511 as f32;
                let v1513 = image_f32(mem, 0x00b4_1d8c);
                let r1514 = remap(mem, false, v1513);
                let x1515: f64 = f64::from(r1514) + f64::from(x);
                let v1516 = x1515 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        v1516.to_bits(),
                        v1512.to_bits(),
                        v1508.to_bits(),
                        r1503.to_bits(),
                        0x5aff_ffff,
                        0x5aff_ffff,
                    ],
                );
                let v1517 = image_f32(mem, 0x00b9_2268);
                let r1518 = remap(mem, true, v1517);
                let v1519 = image_f32(mem, 0x00b4_1d8c);
                let r1520 = remap(mem, false, v1519);
                let v1521 = 1.0f32;
                let r1522 = remap(mem, true, v1521);
                let x1523: f64 = f64::from(r1522) + f64::from(y);
                let v1524 = x1523 as f32;
                let v1525 = image_f32(mem, 0x00b4_1d8c);
                let r1526 = remap(mem, false, v1525);
                let x1527: f64 = x1452 - f64::from(r1526);
                let v1528 = x1527 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        v1528.to_bits(),
                        v1524.to_bits(),
                        r1520.to_bits(),
                        r1518.to_bits(),
                        0x5aff_ffff,
                        0x00ff_ffff,
                    ],
                );
                let v1529 = image_f32(mem, 0x00b3_eb94);
                let r1530 = remap(mem, true, v1529);
                let v1531 = image_f32(mem, 0x00b4_1d8c);
                let r1532 = remap(mem, false, v1531);
                let v1533 = image_f32(mem, 0x00b3_eb94);
                let r1534 = remap(mem, true, v1533);
                let x1535: f64 = x1435 - f64::from(r1534);
                let v1536 = x1535 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        x.to_bits(),
                        v1536.to_bits(),
                        r1532.to_bits(),
                        r1530.to_bits(),
                        0x0000_0000,
                        0x8000_0000,
                    ],
                );
                let v1537 = image_f32(mem, 0x00b3_eb94);
                let r1538 = remap(mem, true, v1537);
                let d1539: f64 = f64::from(w);
                let v1540 = image_f32(mem, 0x00b5_f384);
                let r1541 = remap(mem, false, v1540);
                let x1542: f64 = d1539 - f64::from(r1541);
                let v1543 = x1542 as f32;
                let v1544 = image_f32(mem, 0x00b3_eb94);
                let r1545 = remap(mem, true, v1544);
                let x1546: f64 = x1435 - f64::from(r1545);
                let v1547 = x1546 as f32;
                let v1548 = image_f32(mem, 0x00b4_1d8c);
                let r1549 = remap(mem, false, v1548);
                let x1550: f64 = f64::from(r1549) + f64::from(x);
                let v1551 = x1550 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        v1551.to_bits(),
                        v1547.to_bits(),
                        v1543.to_bits(),
                        r1538.to_bits(),
                        0x8000_0000,
                        0x8000_0000,
                    ],
                );
                let v1552 = image_f32(mem, 0x00b3_eb94);
                let r1553 = remap(mem, true, v1552);
                let v1554 = image_f32(mem, 0x00b4_1d8c);
                let r1555 = remap(mem, false, v1554);
                let v1556 = image_f32(mem, 0x00b3_eb94);
                let r1557 = remap(mem, true, v1556);
                let x1558: f64 = x1435 - f64::from(r1557);
                let v1559 = x1558 as f32;
                let v1560 = image_f32(mem, 0x00b4_1d8c);
                let r1561 = remap(mem, false, v1560);
                let x1562: f64 = x1452 - f64::from(r1561);
                let v1563 = x1562 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        v1563.to_bits(),
                        v1559.to_bits(),
                        r1555.to_bits(),
                        r1553.to_bits(),
                        0x8000_0000,
                        0x0000_0000,
                    ],
                );
            }
            12 => {
                let v1564 = image_f32(mem, 0x00b6_6bc4);
                let v1565 = image_f32(mem, 0x00b6_6bb8);
                let v1566 = image_f32(mem, 0x00b6_6bc0);
                let v1567 = image_f32(mem, 0x00b6_6bb4);
                let v1568 = image_f32(mem, 0x00b5_f2e8);
                let r1569 = remap(mem, true, v1568);
                let v1570 = image_f32(mem, 0x00b5_f2e8);
                let r1571 = remap(mem, false, v1570);
                call(
                    mem,
                    0x0090_3bb0,
                    &[
                        0x0000_00c8,
                        x.to_bits(),
                        y.to_bits(),
                        r1571.to_bits(),
                        r1569.to_bits(),
                        v1567.to_bits(),
                        v1566.to_bits(),
                        v1565.to_bits(),
                        v1564.to_bits(),
                    ],
                );
                let v1572 = image_f32(mem, 0x00b6_6bd8);
                let v1573 = image_f32(mem, 0x00b6_6bb8);
                let v1574 = image_f32(mem, 0x00b6_6bc4);
                let v1575 = image_f32(mem, 0x00b6_6bb4);
                let d1576: f64 = f64::from(h);
                let v1577 = image_f32(mem, 0x00b5_4f84);
                let r1578 = remap(mem, true, v1577);
                let x1579: f64 = d1576 - f64::from(r1578);
                let v1580 = x1579 as f32;
                let v1581 = image_f32(mem, 0x00b5_f2e8);
                let r1582 = remap(mem, false, v1581);
                let v1583 = image_f32(mem, 0x00b5_f2e8);
                let r1584 = remap(mem, true, v1583);
                let x1585: f64 = f64::from(r1584) + f64::from(y);
                let v1586 = x1585 as f32;
                call(
                    mem,
                    0x0090_3bb0,
                    &[
                        0x0000_00c8,
                        x.to_bits(),
                        v1586.to_bits(),
                        r1582.to_bits(),
                        v1580.to_bits(),
                        v1575.to_bits(),
                        v1574.to_bits(),
                        v1573.to_bits(),
                        v1572.to_bits(),
                    ],
                );
                let x1587: f64 = f64::from(y) + f64::from(h);
                let v1588 = image_f32(mem, 0x00b5_f0d8);
                let v1589 = image_f32(mem, 0x00b6_6bb8);
                let v1590 = image_f32(mem, 0x00b6_6bd8);
                let v1591 = image_f32(mem, 0x00b6_6bb4);
                let v1592 = image_f32(mem, 0x00b5_f2e8);
                let r1593 = remap(mem, true, v1592);
                let v1594 = image_f32(mem, 0x00b5_f2e8);
                let r1595 = remap(mem, false, v1594);
                let v1596 = image_f32(mem, 0x00b5_f2e8);
                let r1597 = remap(mem, true, v1596);
                let x1598: f64 = x1587 - f64::from(r1597);
                let v1599 = x1598 as f32;
                call(
                    mem,
                    0x0090_3bb0,
                    &[
                        0x0000_00c8,
                        x.to_bits(),
                        v1599.to_bits(),
                        r1595.to_bits(),
                        r1593.to_bits(),
                        v1591.to_bits(),
                        v1590.to_bits(),
                        v1589.to_bits(),
                        v1588.to_bits(),
                    ],
                );
                let v1600 = image_f32(mem, 0x00b6_6bc4);
                let v1601 = image_f32(mem, 0x00b6_5820);
                let v1602 = image_f32(mem, 0x00b6_6bc0);
                let v1603 = image_f32(mem, 0x00b6_6bb8);
                let v1604 = image_f32(mem, 0x00b5_f2e8);
                let r1605 = remap(mem, true, v1604);
                let d1606: f64 = f64::from(w);
                let v1607 = image_f32(mem, 0x00b5_4f84);
                let r1608 = remap(mem, false, v1607);
                let x1609: f64 = d1606 - f64::from(r1608);
                let v1610 = x1609 as f32;
                let v1611 = image_f32(mem, 0x00b5_f2e8);
                let r1612 = remap(mem, false, v1611);
                let x1613: f64 = f64::from(r1612) + f64::from(x);
                let v1614 = x1613 as f32;
                call(
                    mem,
                    0x0090_3bb0,
                    &[
                        0x0000_00c8,
                        v1614.to_bits(),
                        y.to_bits(),
                        v1610.to_bits(),
                        r1605.to_bits(),
                        v1603.to_bits(),
                        v1602.to_bits(),
                        v1601.to_bits(),
                        v1600.to_bits(),
                    ],
                );
                let v1615 = image_f32(mem, 0x00b5_f0d8);
                let v1616 = image_f32(mem, 0x00b6_5820);
                let v1617 = image_f32(mem, 0x00b6_6bd8);
                let v1618 = image_f32(mem, 0x00b6_6bb8);
                let v1619 = image_f32(mem, 0x00b5_f2e8);
                let r1620 = remap(mem, true, v1619);
                let d1621: f64 = f64::from(w);
                let v1622 = image_f32(mem, 0x00b5_4f84);
                let r1623 = remap(mem, false, v1622);
                let x1624: f64 = d1621 - f64::from(r1623);
                let v1625 = x1624 as f32;
                let v1626 = image_f32(mem, 0x00b5_f2e8);
                let r1627 = remap(mem, true, v1626);
                let x1628: f64 = x1587 - f64::from(r1627);
                let v1629 = x1628 as f32;
                let v1630 = image_f32(mem, 0x00b5_f2e8);
                let r1631 = remap(mem, false, v1630);
                let x1632: f64 = f64::from(r1631) + f64::from(x);
                let v1633 = x1632 as f32;
                call(
                    mem,
                    0x0090_3bb0,
                    &[
                        0x0000_00c8,
                        v1633.to_bits(),
                        v1629.to_bits(),
                        v1625.to_bits(),
                        r1620.to_bits(),
                        v1618.to_bits(),
                        v1617.to_bits(),
                        v1616.to_bits(),
                        v1615.to_bits(),
                    ],
                );
                let x1634: f64 = f64::from(x) + f64::from(w);
                let v1635 = image_f32(mem, 0x00b6_6bc4);
                let v1636 = image_f32(mem, 0x00b6_6bd4);
                let v1637 = image_f32(mem, 0x00b6_6bc0);
                let v1638 = image_f32(mem, 0x00b6_5820);
                let v1639 = image_f32(mem, 0x00b5_f2e8);
                let r1640 = remap(mem, true, v1639);
                let v1641 = image_f32(mem, 0x00b5_f2e8);
                let r1642 = remap(mem, false, v1641);
                let v1643 = image_f32(mem, 0x00b5_f2e8);
                let r1644 = remap(mem, false, v1643);
                let x1645: f64 = x1634 - f64::from(r1644);
                let v1646 = x1645 as f32;
                call(
                    mem,
                    0x0090_3bb0,
                    &[
                        0x0000_00c8,
                        v1646.to_bits(),
                        y.to_bits(),
                        r1642.to_bits(),
                        r1640.to_bits(),
                        v1638.to_bits(),
                        v1637.to_bits(),
                        v1636.to_bits(),
                        v1635.to_bits(),
                    ],
                );
                let v1647 = image_f32(mem, 0x00b6_6bd8);
                let v1648 = image_f32(mem, 0x00b6_6bd4);
                let v1649 = image_f32(mem, 0x00b6_6bc4);
                let v1650 = image_f32(mem, 0x00b6_5820);
                let d1651: f64 = f64::from(h);
                let v1652 = image_f32(mem, 0x00b5_4f84);
                let r1653 = remap(mem, true, v1652);
                let x1654: f64 = d1651 - f64::from(r1653);
                let v1655 = x1654 as f32;
                let v1656 = image_f32(mem, 0x00b5_f2e8);
                let r1657 = remap(mem, false, v1656);
                let v1658 = image_f32(mem, 0x00b5_f2e8);
                let r1659 = remap(mem, true, v1658);
                let x1660: f64 = f64::from(r1659) + f64::from(y);
                let v1661 = x1660 as f32;
                let v1662 = image_f32(mem, 0x00b5_f2e8);
                let r1663 = remap(mem, false, v1662);
                let x1664: f64 = x1634 - f64::from(r1663);
                let v1665 = x1664 as f32;
                call(
                    mem,
                    0x0090_3bb0,
                    &[
                        0x0000_00c8,
                        v1665.to_bits(),
                        v1661.to_bits(),
                        r1657.to_bits(),
                        v1655.to_bits(),
                        v1650.to_bits(),
                        v1649.to_bits(),
                        v1648.to_bits(),
                        v1647.to_bits(),
                    ],
                );
                let v1666 = image_f32(mem, 0x00b5_f0d8);
                let v1667 = image_f32(mem, 0x00b6_6bd4);
                let v1668 = image_f32(mem, 0x00b6_6bd8);
                let v1669 = image_f32(mem, 0x00b6_5820);
                let v1670 = image_f32(mem, 0x00b5_f2e8);
                let r1671 = remap(mem, true, v1670);
                let v1672 = image_f32(mem, 0x00b5_f2e8);
                let r1673 = remap(mem, false, v1672);
                let v1674 = image_f32(mem, 0x00b5_f2e8);
                let r1675 = remap(mem, true, v1674);
                let x1676: f64 = x1587 - f64::from(r1675);
                let v1677 = x1676 as f32;
                let v1678 = image_f32(mem, 0x00b5_f2e8);
                let r1679 = remap(mem, false, v1678);
                let x1680: f64 = x1634 - f64::from(r1679);
                let v1681 = x1680 as f32;
                call(
                    mem,
                    0x0090_3bb0,
                    &[
                        0x0000_00c8,
                        v1681.to_bits(),
                        v1677.to_bits(),
                        r1673.to_bits(),
                        r1671.to_bits(),
                        v1669.to_bits(),
                        v1668.to_bits(),
                        v1667.to_bits(),
                        v1666.to_bits(),
                    ],
                );
            }
            13 => {
                let x1682: f64 = f64::from(y) + f64::from(h);
                let v1683 = x1682 as f32;
                let v1684 = image_f32(mem, 0x00b9_22f8);
                let r1685 = remap(mem, true, v1684);
                let v1686 = image_f32(mem, 0x00b6_59bc);
                let r1687 = remap(mem, false, v1686);
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        x.to_bits(),
                        v1683.to_bits(),
                        r1687.to_bits(),
                        r1685.to_bits(),
                        0x0000_0000,
                        0x2600_0000,
                    ],
                );
                let v1688 = image_f32(mem, 0x00b9_22f8);
                let r1689 = remap(mem, true, v1688);
                let d1690: f64 = f64::from(w);
                let v1691 = image_f32(mem, 0x00b5_8cdc);
                let r1692 = remap(mem, false, v1691);
                let x1693: f64 = d1690 - f64::from(r1692);
                let v1694 = x1693 as f32;
                let v1695 = image_f32(mem, 0x00b6_59bc);
                let r1696 = remap(mem, false, v1695);
                let x1697: f64 = f64::from(r1696) + f64::from(x);
                let v1698 = x1697 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        v1698.to_bits(),
                        v1683.to_bits(),
                        v1694.to_bits(),
                        r1689.to_bits(),
                        0x2600_0000,
                        0x2600_0000,
                    ],
                );
                let x1699: f64 = f64::from(x) + f64::from(w);
                let v1700 = image_f32(mem, 0x00b9_22f8);
                let r1701 = remap(mem, true, v1700);
                let v1702 = image_f32(mem, 0x00b6_59bc);
                let r1703 = remap(mem, false, v1702);
                let v1704 = image_f32(mem, 0x00b6_59bc);
                let r1705 = remap(mem, false, v1704);
                let x1706: f64 = x1699 - f64::from(r1705);
                let v1707 = x1706 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        v1707.to_bits(),
                        v1683.to_bits(),
                        r1703.to_bits(),
                        r1701.to_bits(),
                        0x2600_0000,
                        0x0000_0000,
                    ],
                );
                let v1708 = image_f32(mem, 0x00b6_59bc);
                let r1709 = remap(mem, false, v1708);
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        x.to_bits(),
                        y.to_bits(),
                        r1709.to_bits(),
                        h.to_bits(),
                        0x0063_2d0d,
                        0x4063_2d0d,
                    ],
                );
                let d1710: f64 = f64::from(w);
                let v1711 = image_f32(mem, 0x00b5_8cdc);
                let r1712 = remap(mem, false, v1711);
                let x1713: f64 = d1710 - f64::from(r1712);
                let v1714 = x1713 as f32;
                let v1715 = image_f32(mem, 0x00b6_59bc);
                let r1716 = remap(mem, false, v1715);
                let x1717: f64 = f64::from(r1716) + f64::from(x);
                let v1718 = x1717 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        v1718.to_bits(),
                        y.to_bits(),
                        v1714.to_bits(),
                        h.to_bits(),
                        0x4063_2d0d,
                        0x4063_2d0d,
                    ],
                );
                let v1719 = image_f32(mem, 0x00b6_59bc);
                let r1720 = remap(mem, false, v1719);
                let v1721 = image_f32(mem, 0x00b6_59bc);
                let r1722 = remap(mem, false, v1721);
                let x1723: f64 = x1699 - f64::from(r1722);
                let v1724 = x1723 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        v1724.to_bits(),
                        y.to_bits(),
                        r1720.to_bits(),
                        h.to_bits(),
                        0x4063_2d0d,
                        0x0063_2d0d,
                    ],
                );
                let v1725 = image_f32(mem, 0x00b9_2268);
                let r1726 = remap(mem, true, v1725);
                let v1727 = image_f32(mem, 0x00b6_59bc);
                let r1728 = remap(mem, false, v1727);
                let v1729 = 1.0f32;
                let r1730 = remap(mem, true, v1729);
                let x1731: f64 = f64::from(r1730) + f64::from(y);
                let v1732 = x1731 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        x.to_bits(),
                        v1732.to_bits(),
                        r1728.to_bits(),
                        r1726.to_bits(),
                        0x00ff_ffff,
                        0x5aff_ffff,
                    ],
                );
                let v1733 = image_f32(mem, 0x00b9_2268);
                let r1734 = remap(mem, true, v1733);
                let d1735: f64 = f64::from(w);
                let v1736 = image_f32(mem, 0x00b5_8cdc);
                let r1737 = remap(mem, false, v1736);
                let x1738: f64 = d1735 - f64::from(r1737);
                let v1739 = x1738 as f32;
                let v1740 = 1.0f32;
                let r1741 = remap(mem, true, v1740);
                let x1742: f64 = f64::from(r1741) + f64::from(y);
                let v1743 = x1742 as f32;
                let v1744 = image_f32(mem, 0x00b6_59bc);
                let r1745 = remap(mem, false, v1744);
                let x1746: f64 = f64::from(r1745) + f64::from(x);
                let v1747 = x1746 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        v1747.to_bits(),
                        v1743.to_bits(),
                        v1739.to_bits(),
                        r1734.to_bits(),
                        0x5aff_ffff,
                        0x5aff_ffff,
                    ],
                );
                let v1748 = image_f32(mem, 0x00b9_2268);
                let r1749 = remap(mem, true, v1748);
                let v1750 = image_f32(mem, 0x00b6_59bc);
                let r1751 = remap(mem, false, v1750);
                let v1752 = 1.0f32;
                let r1753 = remap(mem, true, v1752);
                let x1754: f64 = f64::from(r1753) + f64::from(y);
                let v1755 = x1754 as f32;
                let v1756 = image_f32(mem, 0x00b6_59bc);
                let r1757 = remap(mem, false, v1756);
                let x1758: f64 = x1699 - f64::from(r1757);
                let v1759 = x1758 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        v1759.to_bits(),
                        v1755.to_bits(),
                        r1751.to_bits(),
                        r1749.to_bits(),
                        0x5aff_ffff,
                        0x00ff_ffff,
                    ],
                );
                let v1760 = image_f32(mem, 0x00b3_eb94);
                let r1761 = remap(mem, true, v1760);
                let v1762 = image_f32(mem, 0x00b6_59bc);
                let r1763 = remap(mem, false, v1762);
                let v1764 = image_f32(mem, 0x00b3_eb94);
                let r1765 = remap(mem, true, v1764);
                let x1766: f64 = x1682 - f64::from(r1765);
                let v1767 = x1766 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        x.to_bits(),
                        v1767.to_bits(),
                        r1763.to_bits(),
                        r1761.to_bits(),
                        0x0000_0000,
                        0x8000_0000,
                    ],
                );
                let v1768 = image_f32(mem, 0x00b3_eb94);
                let r1769 = remap(mem, true, v1768);
                let d1770: f64 = f64::from(w);
                let v1771 = image_f32(mem, 0x00b5_8cdc);
                let r1772 = remap(mem, false, v1771);
                let x1773: f64 = d1770 - f64::from(r1772);
                let v1774 = x1773 as f32;
                let v1775 = image_f32(mem, 0x00b3_eb94);
                let r1776 = remap(mem, true, v1775);
                let x1777: f64 = x1682 - f64::from(r1776);
                let v1778 = x1777 as f32;
                let v1779 = image_f32(mem, 0x00b6_59bc);
                let r1780 = remap(mem, false, v1779);
                let x1781: f64 = f64::from(r1780) + f64::from(x);
                let v1782 = x1781 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        v1782.to_bits(),
                        v1778.to_bits(),
                        v1774.to_bits(),
                        r1769.to_bits(),
                        0x8000_0000,
                        0x8000_0000,
                    ],
                );
                let v1783 = image_f32(mem, 0x00b3_eb94);
                let r1784 = remap(mem, true, v1783);
                let v1785 = image_f32(mem, 0x00b6_59bc);
                let r1786 = remap(mem, false, v1785);
                let v1787 = image_f32(mem, 0x00b3_eb94);
                let r1788 = remap(mem, true, v1787);
                let x1789: f64 = x1682 - f64::from(r1788);
                let v1790 = x1789 as f32;
                let v1791 = image_f32(mem, 0x00b6_59bc);
                let r1792 = remap(mem, false, v1791);
                let x1793: f64 = x1699 - f64::from(r1792);
                let v1794 = x1793 as f32;
                call(
                    mem,
                    0x008f_4b20,
                    &[
                        v1794.to_bits(),
                        v1790.to_bits(),
                        r1786.to_bits(),
                        r1784.to_bits(),
                        0x8000_0000,
                        0x0000_0000,
                    ],
                );
            }
            _ => {}
        }
    }
}
