//! The field's particle effects ("FP", `pppFp*` and `Fp_*`).
//!
//! `g_FpData` points at the current field's FP table: a `u16` count at
//! `+4`, and 0x50-byte entries whose `u16` ID is at `+0x64` (entry `i`'s
//! at `+0x64 + 0x50 i`). Each entry has two runtime records:
//! - `g_FpRun` (0x80 bytes each): the frame counter at `+0` (`-0x1000`
//!   while not started) and the status byte at `+0x1d` (0 running, 1
//!   ended, 2 restarted in mode 2);
//! - `g_FpDisp` (0x4c bytes each): flags at `+0` (0x10 hidden, 2 stuck to
//!   a map object, 8 stuck to a display object, the low four bits the
//!   sticking), the stick target at `+4` and a word at `+8` that sticking
//!   clears.
//!
//! The functions taking an `i16` ID change every entry with that ID; the
//! ones taking a full word compare it with the zero-extended ID and change
//! the first match.

/// `pppFpSaveStat`'s packing: two bits per entry with a nonzero ID, four
/// entries a byte from bit 0 (bit 0 running, bit 1 hidden). The byte after
/// a full one is started (cleared) even when no entry follows; `out`
/// receives every byte written.
pub fn save_stat(entries: impl IntoIterator<Item = (bool, bool)>, out: &mut Vec<u8>) {
    out.clear();
    out.push(0);
    let mut bit = 0;
    for (running, hidden) in entries {
        let last = out.len() - 1;
        if running {
            out[last] |= 1 << bit;
        }
        if hidden {
            out[last] |= 1 << (bit + 1);
        }
        bit += 2;
        if bit > 7 {
            bit = 0;
            out.push(0);
        }
    }
}

/// The FP tables where the game keeps them.
pub mod bridge {
    use ffx_bridge::{Global, Mem, Ptr};

    /// `g_FpData`: the current field's FP table (0 when none).
    pub const DATA: Global<u32> = Global::new(0x00d2_c258);
    /// `g_FpRun`: 0x80-byte run records.
    pub const RUN: Global<[u8; 0x80]> = Global::new(0x00d2_c260);
    /// `g_FpDisp`: 0x4c-byte display records.
    pub const DISP: Global<[u8; 0x4c]> = Global::new(0x00d3_0260);
    /// `g_pppStopStatus` (see [`crate::ppp::bridge::STOP`]).
    pub const STOP: Global<u32> = crate::ppp::bridge::STOP;
    /// `g_FpPointLightA`: cleared by `pppFpPointLightInit` and `pppFpLoop`,
    /// read by `pppFpPointLightProc`.
    pub const POINT_LIGHT_A: Global<u32> = Global::new(0x00ce_c1f4);
    /// `g_FpPointLightB`: likewise.
    pub const POINT_LIGHT_B: Global<u32> = Global::new(0x00ce_c1fc);

    const NOT_STARTED: u32 = 0xffff_f000;

    /// The table's entry count and the address of entry 0's ID.
    fn table(mem: &impl Mem) -> Option<(u32, u32)> {
        let t = DATA.get(mem);
        if t == 0 {
            return None;
        }
        Some((u32::from(mem.read::<u16>(t + 4)), t + 0x64))
    }

    fn id(mem: &impl Mem, ids: u32, i: u32) -> u16 {
        mem.read(ids.wrapping_add(0x50 * i))
    }

    fn run(mem: &impl Mem, i: u32) -> u32 {
        RUN.ptr(mem).at(i).addr
    }

    fn disp(mem: &impl Mem, i: u32) -> u32 {
        DISP.ptr(mem).at(i).addr
    }

    /// Every entry whose ID is the low 16 bits of `id16`.
    fn matching(mem: &impl Mem, id16: u32) -> Vec<u32> {
        let Some((n, ids)) = table(mem) else {
            return Vec::new();
        };
        #[allow(clippy::cast_possible_truncation)] // the original compares 16 bits
        let want = id16 as u16;
        (0..n).filter(|&i| id(mem, ids, i) == want).collect()
    }

    fn first(mem: &impl Mem, id32: u32) -> Option<u32> {
        let (n, ids) = table(mem)?;
        (0..n).find(|&i| u32::from(id(mem, ids, i)) == id32)
    }

    fn or_flags(mem: &mut impl Mem, at: u32, set: u32, keep: u32) {
        let p = Ptr::<u32>::new(at);
        let v = (p.get(mem) & keep) | set;
        p.set(mem, v);
    }

    fn restart(mem: &mut impl Mem, id: u32, status: u8) {
        for i in matching(mem, id) {
            let r = run(mem, i);
            mem.write(r, 0u32);
            mem.write(r + 0x1d, status);
        }
    }

    /// `Fp_Restart(id)` (`0x00729700`): counter 0, status running.
    pub fn fp_restart(mem: &mut impl Mem, id: u32) {
        restart(mem, id, 0);
    }

    /// `Fp_RestartMode2(id)` (`0x00729810`): counter 0, status 2.
    pub fn fp_restart_mode2(mem: &mut impl Mem, id: u32) {
        restart(mem, id, 2);
    }

    /// Every entry whose `u16` at `+0x66` (after the ID) is the low 16
    /// bits of `group`.
    fn in_group(mem: &impl Mem, group: u32) -> Vec<u32> {
        let Some((n, ids)) = table(mem) else {
            return Vec::new();
        };
        #[allow(clippy::cast_possible_truncation)] // the original compares 16 bits
        let want = group as u16;
        (0..n)
            .filter(|&i| mem.read::<u16>(ids.wrapping_add(2 + 0x50 * i)) == want)
            .collect()
    }

    /// `Fp_EndGroup(group)` (`0x007294f0`): as `Fp_End` for every entry
    /// of the group (`+0x66`).
    pub fn fp_end_group(mem: &mut impl Mem, group: u32) {
        for i in in_group(mem, group) {
            let r = run(mem, i);
            mem.write(r + 0x1d, 1u8);
        }
    }

    /// `Fp_RestartGroup(group)` (`0x00729530`): as `Fp_Restart` for every
    /// entry of the group.
    pub fn fp_restart_group(mem: &mut impl Mem, group: u32) {
        for i in in_group(mem, group) {
            let r = run(mem, i);
            mem.write(r, 0u32);
            mem.write(r + 0x1d, 0u8);
        }
    }

    /// `Fp_End(id)` (`0x00729660`): status ended.
    pub fn fp_end(mem: &mut impl Mem, id: u32) {
        for i in matching(mem, id) {
            let r = run(mem, i);
            mem.write(r + 0x1d, 1u8);
        }
    }

    /// `pppFpIDdispOff(id)` (`0x00729570`): hidden.
    pub fn ppp_fp_id_disp_off(mem: &mut impl Mem, id: u32) {
        for i in matching(mem, id) {
            let d = disp(mem, i);
            or_flags(mem, d, 0x10, !0);
        }
    }

    /// `pppFpIDdispOn(id)` (`0x007295b0`): shown.
    pub fn ppp_fp_id_disp_on(mem: &mut impl Mem, id: u32) {
        for i in matching(mem, id) {
            let d = disp(mem, i);
            or_flags(mem, d, 0, !0x10);
        }
    }

    /// `pppFpGetMngPtr(id, &run, &disp)` (`0x007292f0`): the first entry's
    /// records, each output cleared first and set when found (outputs may
    /// be null).
    pub fn ppp_fp_get_mng_ptr(mem: &mut impl Mem, id: u32, run_out: u32, disp_out: u32) {
        if run_out != 0 {
            mem.write(run_out, 0u32);
        }
        if disp_out != 0 {
            mem.write(disp_out, 0u32);
        }
        let Some(i) = first(mem, id) else { return };
        if run_out != 0 {
            let r = run(mem, i);
            mem.write(run_out, r);
        }
        if disp_out != 0 {
            let d = disp(mem, i);
            mem.write(disp_out, d);
        }
    }

    fn stick(mem: &mut impl Mem, id: u32, set: u32, keep: u32, target: u32) {
        let Some(i) = first(mem, id) else { return };
        let d = disp(mem, i);
        or_flags(mem, d, set, keep);
        mem.write(d + 4, target);
        mem.write(d + 8, 0u32);
    }

    /// `pppFpResetStick(id)` (`0x0072a120`): no sticking.
    pub fn ppp_fp_reset_stick(mem: &mut impl Mem, id: u32) {
        stick(mem, id, 0, !0xf, 0);
    }

    /// `pppFpSetStickMap(id, target)` (`0x0072a590`).
    pub fn ppp_fp_set_stick_map(mem: &mut impl Mem, id: u32, target: u32) {
        stick(mem, id, 2, !0, target);
    }

    /// `pppFpSetStickMapDObj(id, target)` (`0x0072a5e0`).
    pub fn ppp_fp_set_stick_map_dobj(mem: &mut impl Mem, id: u32, target: u32) {
        stick(mem, id, 8, !0, target);
    }

    /// `pppFpStopStatus(stop)` (`0x0072a750`).
    pub fn ppp_fp_stop_status(mem: &mut impl Mem, stop: u32) {
        STOP.set(mem, stop);
    }

    /// `pppFpPointLightInit()` (`0x00729f70`).
    pub fn ppp_fp_point_light_init(mem: &mut impl Mem) {
        POINT_LIGHT_A.set(mem, 0);
        POINT_LIGHT_B.set(mem, 0);
    }

    /// `pppFpSaveStat(out)` (`0x0072a180`): every entry's state as two
    /// bits ([`super::save_stat`]); running is the first entry with the
    /// same ID started and not ended.
    pub fn ppp_fp_save_stat(mem: &mut impl Mem, out: u32) {
        if out == 0 {
            return;
        }
        let Some((n, ids)) = table(mem) else { return };
        let mut entries = Vec::new();
        for i in 0..n {
            let me = id(mem, ids, i);
            if me == 0 {
                continue;
            }
            let running = (0..n).find(|&j| id(mem, ids, j) == me).is_some_and(|j| {
                let r = run(mem, j);
                mem.read::<u32>(r) != NOT_STARTED && mem.read::<u8>(r + 0x1d) == 0
            });
            let hidden = mem.read::<u32>(disp(mem, i)) & 0x10 != 0;
            entries.push((running, hidden));
        }
        let mut bytes = Vec::new();
        super::save_stat(entries, &mut bytes);
        mem.write_bytes(out, &bytes);
    }

    /// `g_pppFpDisp`: the field effects' display switch (`pppFpSetDisp`,
    /// the field effect debug listing reads it).
    pub const FP_DISP: Global<u32> = Global::new(0x00ce_c1f0);

    /// `pppFpSetDisp(on)` (`0x00729050`): returns `on` (left in `eax`).
    pub fn ppp_fp_set_disp(mem: &mut impl Mem, on: u32) -> u32 {
        FP_DISP.set(mem, on);
        on
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn packs_four_a_byte() {
        let mut out = Vec::new();
        super::save_stat(
            [(true, false), (false, true), (true, true), (false, false)],
            &mut out,
        );
        assert_eq!(out, [0b0011_1001, 0]);
        super::save_stat([], &mut out);
        assert_eq!(out, [0]);
    }
}
