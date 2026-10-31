//! M5 `system-a` (round 3): the save machine's file scan
//! (`SaveDataMachine_ScanFiles`: which of the 200 save files exist and
//! their local modification times) and the list rows from the files'
//! headers (`SaveDataMachine_ReadHeaders`).

/// The originals over game memory and the original callees.
pub mod bridge {
    use crate::m5a_savedata::{
        COUNT, EXISTS, FILES, FULL, LIST_LEN, NAME_FORMAT, SAVE_DIR, STATE, USER_ID,
    };
    use crate::m5a_savedata2::FOLDER_SET;
    use ffx_graphic::host::{Conv, Host};

    /// KERNEL32 imports (stdcall): `GetTimeZoneInformation`,
    /// `CreateFileW`, `GetFileTime`, `FileTimeToSystemTime`,
    /// `SystemTimeToTzSpecificLocalTime`, `CloseHandle`; USER32
    /// `wsprintfW` (cdecl).
    pub const GET_TIME_ZONE_IAT: u32 = 0x00b0_c084;
    pub const CREATE_FILE_IAT: u32 = 0x00b0_c138;
    pub const GET_FILE_TIME_IAT: u32 = 0x00b0_c090;
    pub const FILE_TIME_TO_SYSTEM_TIME_IAT: u32 = 0x00b0_c088;
    pub const TO_LOCAL_TIME_IAT: u32 = 0x00b0_c08c;
    pub const CLOSE_HANDLE_IAT: u32 = 0x00b0_c0dc;
    pub const WSPRINTFW_IAT: u32 = 0x00b0_c674;

    fn import(h: &mut impl Host, slot: u32, conv: Conv, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, conv, args)
    }

    fn rd(h: &impl Host, sym: u32) -> u32 {
        h.read(h.rebase(sym))
    }

    fn wr(h: &mut impl Host, sym: u32, v: u32) {
        let at = h.rebase(sym);
        h.write(at, v);
    }

    /// The frame `SaveDataMachine_ScanFiles` needs: the file time at
    /// `ebp - 0x4dc`, the slot-0 flag at `ebp - 0x4d4`, the time zone at
    /// `ebp - 0x4d0`, the UTC and local `SYSTEMTIME`s at `ebp - 0x424`
    /// and `ebp - 0x414`, the path at `ebp - 0x404`.
    pub const SCAN_FRAME: u32 = 0x4dc;

    /// `SaveDataMachine_ScanFiles()` (cdecl): with the save folder set
    /// (else the original's deliberate store to address 0), each of the
    /// 200 files opened for reading: a missing one marks its row -1
    /// (`EXISTS` and the three `FILES` words); an existing one records
    /// its number, its local modification date (`(year - 2000) * 100 +
    /// month) * 100 + day`) and time of day in seconds, and counts. The
    /// count goes to `COUNT`, whether the list is full (199 or more
    /// besides file 0) to `FULL`; the list length (`LIST_LEN`) is the
    /// count, or in the save state (1) the count besides file 0, plus a
    /// new-file row unless full. Returns the count besides file 0 (plus
    /// that row in the save state).
    ///
    /// junk read: `[ebp-0x414..-0x408]` (the local time) is read for an
    /// open file whose time `GetFileTime` doesn't give; on the first such
    /// file before any time was converted it was never written; 0 (brief
    /// 2026-10-24): the port's frame is zeroed.
    pub fn scan_files(h: &mut impl Host, ebp: u32) -> u32 {
        let at = |off: u32| ebp.wrapping_sub(off);
        let (ft, slot0, tz, utc, local, path) = (
            at(0x4dc),
            at(0x4d4),
            at(0x4d0),
            at(0x424),
            at(0x414),
            at(0x404),
        );
        h.write(slot0, 0u32);
        import(h, GET_TIME_ZONE_IAT, Conv::Stdcall, &[tz]);
        if rd(h, FOLDER_SET) == 0 {
            // the original's assertion: a store to address 0
            h.write(0, 1u32);
        }
        let mut count = 0u32;
        for i in 0..200u32 {
            let fmt = h.rebase(NAME_FORMAT);
            let dir = h.rebase(SAVE_DIR);
            let user = rd(h, USER_ID);
            import(h, WSPRINTFW_IAT, Conv::Cdecl, &[path, fmt, dir, user, i]);
            let file = import(
                h,
                CREATE_FILE_IAT,
                Conv::Stdcall,
                &[path, 0x8000_0000, 1, 0, 3, 0, 0],
            );
            let row = h.rebase(FILES).wrapping_add(12 * i);
            let exists = h.rebase(EXISTS).wrapping_add(4 * i);
            if file == u32::MAX {
                h.write(exists, u32::MAX);
                for k in 0..3 {
                    h.write(row.wrapping_add(4 * k), u32::MAX);
                }
                continue;
            }
            if import(h, GET_FILE_TIME_IAT, Conv::Stdcall, &[file, 0, 0, ft]) != 0 {
                import(h, FILE_TIME_TO_SYSTEM_TIME_IAT, Conv::Stdcall, &[ft, utc]);
                import(h, TO_LOCAL_TIME_IAT, Conv::Stdcall, &[tz, utc, local]);
            }
            let mut st = [0u32; 7];
            for (k, v) in (0u32..).zip(st.iter_mut()) {
                *v = u32::from(h.read::<u16>(local.wrapping_add(2 * k)));
            }
            let [year, month, _, day, hour, minute, second] = st;
            let date = year
                .wrapping_sub(2000)
                .wrapping_mul(100)
                .wrapping_add(month)
                .wrapping_mul(100)
                .wrapping_add(day);
            let time = (hour * 60 + minute) * 60 + second;
            h.write(row, i);
            h.write(row.wrapping_add(4), date);
            h.write(row.wrapping_add(8), time);
            h.write(exists, 1u32);
            import(h, CLOSE_HANDLE_IAT, Conv::Stdcall, &[file]);
            if i == 0 {
                h.write(slot0, 1u32);
            }
            count += 1;
        }
        let mut others = count.wrapping_sub(h.read(slot0));
        let full = u32::from(others.cast_signed() >= 199);
        wr(h, COUNT, count);
        wr(h, FULL, full);
        if rd(h, STATE) == 1 {
            if full != 1 {
                others = others.wrapping_add(1);
            }
            wr(h, LIST_LEN, others);
        } else {
            wr(h, LIST_LEN, count);
        }
        others
    }

    /// `_wfopen`, `fread`, `fclose`, `sprintf` (cdecl).
    pub const WFOPEN_IAT: u32 = 0x00b0_c47c;
    pub const FREAD_IAT: u32 = 0x00b0_c3f4;
    pub const FCLOSE_IAT: u32 = 0x00b0_c3ec;
    pub const SPRINTF_IAT: u32 = 0x00b0_c3dc;
    /// The read mode (`"rb"`, UTF-16).
    pub const MODE_READ: u32 = 0x00b7_ca84;
    /// The callees (cdecl): `SaveDataMachine_newFileRow_00702be0`,
    /// `SaveDataMachine_fileRow_00702d90` (12 words), `AtelGetSaveDicName`,
    /// `ConvertGameCodeToUTF8`, `SaveDataGetLoc_00647f30`, `fix_mappic`,
    /// `isNeedShowJapanLogo`.
    pub const NEW_FILE_ROW: u32 = 0x0070_2be0;
    pub const FILE_ROW: u32 = 0x0070_2d90;
    pub const SAVE_DIC_NAME: u32 = 0x0086_c430;
    pub const TO_UTF8: u32 = 0x0064_5f30;
    pub const GET_LOC: u32 = 0x0064_7f30;
    pub const FIX_MAPPIC: u32 = 0x006e_f6f0;
    pub const JAPAN_LOGO: u32 = 0x0078_7390;
    /// The row's text fields (globals): the number, the place (UTF-8,
    /// its game-code name at `0x00ce7fe0`), the date, the name (game code,
    /// then UTF-8), the play-time label (game code, UTF-8), the play time,
    /// the three party bytes, the picture.
    pub const T_NUMBER: u32 = 0x00ce_7f98;
    pub const T_PLACE: u32 = 0x00ce_7fa0;
    pub const T_PLACE_NAME: u32 = 0x00ce_7fe0;
    pub const T_DATE: u32 = 0x00ce_7fe8;
    pub const T_NAME_CODE: u32 = 0x00ce_8028;
    pub const T_NAME: u32 = 0x00ce_8048;
    pub const T_LABEL_CODE: u32 = 0x00ce_8068;
    pub const T_LABEL: u32 = 0x00ce_80a8;
    pub const T_TIME: u32 = 0x00ce_80e8;
    pub const T_PARTY: [u32; 3] = [0x00ce_8168, 0x00ce_816c, 0x00ce_8170];
    pub const T_PICTURE: u32 = 0x00ce_8174;
    /// The formats: the number, the date, `"%s"`, the play time, a party
    /// byte, the picture, the picture above 1000.
    pub const F_NUMBER: u32 = 0x00b0_f604;
    pub const F_DATE: u32 = 0x00b4_b6a8;
    pub const F_STRING: u32 = 0x00b0_e160;
    pub const F_TIME: u32 = 0x00b4_b6c0;
    pub const F_PARTY: u32 = 0x00b4_b6d4;
    pub const F_PICTURE: u32 = 0x00b4_b6e0;
    pub const F_PICTURE_HIGH: u32 = 0x00b4_b6d8;
    /// The frame `SaveDataMachine_ReadHeaders` needs: the header at
    /// `ebp - 0x444` (0x40 bytes), the path at `ebp - 0x404`.
    pub const HEADERS_FRAME: u32 = 0x444;

    fn sprintf(h: &mut impl Host, out: u32, fmt: u32, args: &[u32]) {
        let mut words = vec![h.rebase(out), h.rebase(fmt)];
        words.extend_from_slice(args);
        // call width: 8 (the date's six values)
        import(h, SPRINTF_IAT, Conv::Cdecl, &words);
    }

    /// `SaveDataMachine_ReadHeaders(first, end)` (cdecl; its callers drop
    /// `eax`): the rows `first..end` of the sorted file list (with the
    /// save folder set, else the store to address 0): in the save state
    /// a list from row 0 starts with the new-file row (unless full) and
    /// skips file 0. Each file that opens has its 0x40-byte header read
    /// into the frame and its row's texts formatted into the globals (the
    /// number, the place by its map's save name, the date and time, the
    /// name, the play time with its label, the three party bytes plus 1,
    /// the picture: `fix_mappic` of the map, 999 for 0 with the Japanese
    /// logo, less 1000 above it), then the row added
    /// (`SaveDataMachine_fileRow`; file 0's only in the load states 0xb
    /// and 0x15, with its own flag). The row number counts the opened
    /// files.
    ///
    /// junk read: a file shorter than 0x40 bytes leaves the header's rest
    /// as the frame held it (an earlier file's, or never written: 0 in
    /// the port's zeroed frame, brief 2026-10-24).
    #[allow(clippy::too_many_lines)] // the row's ten texts
    pub fn read_headers(h: &mut impl Host, ebp: u32, first: u32, end: u32) {
        let hdr = ebp.wrapping_sub(HEADERS_FRAME);
        let path = ebp.wrapping_sub(0x404);
        let mut row = first;
        if rd(h, FOLDER_SET) == 0 {
            h.write(0, 1u32);
        }
        let mut i = first;
        if i == 0 && rd(h, STATE) == 1 {
            if rd(h, FULL) == 0 {
                h.call_sym(NEW_FILE_ROW, Conv::Cdecl, &[]);
                row = 1;
            }
            i = 1;
        }
        while i.cast_signed() < end.cast_signed() {
            let files = h.rebase(FILES).wrapping_add(12 * i);
            let number: u32 = h.read(files);
            let fmt = h.rebase(NAME_FORMAT);
            let dir = h.rebase(SAVE_DIR);
            let user = rd(h, USER_ID);
            import(
                h,
                WSPRINTFW_IAT,
                Conv::Cdecl,
                &[path, fmt, dir, user, number],
            );
            let mode = h.rebase(MODE_READ);
            let f = import(h, WFOPEN_IAT, Conv::Cdecl, &[path, mode]);
            if f == 0 {
                i = i.wrapping_add(1);
                continue;
            }
            import(h, FREAD_IAT, Conv::Cdecl, &[hdr, 1, 0x40, f]);
            import(h, FCLOSE_IAT, Conv::Cdecl, &[f]);
            let number: u32 = h.read(files);
            sprintf(h, T_NUMBER, F_NUMBER, &[number]);
            let map = u32::from(h.read::<u16>(hdr.wrapping_add(0x18)));
            if map == 0 {
                let at = h.rebase(T_PLACE);
                h.write(at, 0u8);
            } else {
                let name = h.call_sym(SAVE_DIC_NAME, Conv::Cdecl, &[map, 0]);
                let at = h.rebase(T_PLACE_NAME);
                h.write(at, name);
                let out = h.rebase(T_PLACE);
                h.call_sym(TO_UTF8, Conv::Cdecl, &[name, out]);
            }
            let time = h.read::<u32>(files.wrapping_add(8)).cast_signed();
            let date = h.read::<u32>(files.wrapping_add(4)).cast_signed();
            let (hours, rest) = (time.wrapping_div(3600), time.wrapping_rem(3600));
            let (year, month_day) = (date.wrapping_div(10000), date.wrapping_rem(10000));
            let seconds = time.wrapping_rem(60);
            let minutes = rest.wrapping_div(60);
            let day = date.wrapping_rem(100);
            let month = month_day.wrapping_div(100);
            let date_args = [year.wrapping_add(2000), month, day, hours, minutes, seconds]
                .map(i32::cast_unsigned);
            sprintf(h, T_DATE, F_DATE, &date_args);
            let name = hdr.wrapping_add(0x20);
            sprintf(h, T_NAME_CODE, F_STRING, &[name]);
            let (code, out) = (h.rebase(T_NAME_CODE), h.rebase(T_NAME));
            h.call_sym(TO_UTF8, Conv::Cdecl, &[code, out]);
            let play = h.read::<u32>(hdr.wrapping_add(0x10)).cast_signed();
            let label = h.rebase(T_LABEL_CODE);
            h.call_sym(GET_LOC, Conv::Cdecl, &[0x52, label]);
            let out = h.rebase(T_LABEL);
            h.call_sym(TO_UTF8, Conv::Cdecl, &[label, out]);
            let total_minutes = play.wrapping_div(60);
            let time_args = [
                h.rebase(T_LABEL),
                total_minutes.wrapping_div(60).cast_unsigned(),
                total_minutes.wrapping_rem(60).cast_unsigned(),
                play.wrapping_sub(total_minutes.wrapping_mul(60))
                    .cast_unsigned(),
            ];
            sprintf(h, T_TIME, F_TIME, &time_args);
            for (k, out) in (5u32..).zip(T_PARTY) {
                let b = i32::from(h.read::<i8>(hdr.wrapping_add(k)));
                sprintf(h, out, F_PARTY, &[b.wrapping_add(1).cast_unsigned()]);
            }
            let mut picture = h.call_sym(FIX_MAPPIC, Conv::Cdecl, &[map]);
            if picture == 0 && h.call_sym(JAPAN_LOGO, Conv::Cdecl, &[]) != 0 {
                picture = 999;
            }
            if picture.cast_signed() > 1000 {
                sprintf(h, T_PICTURE, F_PICTURE_HIGH, &[picture.wrapping_sub(1000)]);
            } else {
                sprintf(h, T_PICTURE, F_PICTURE, &[picture]);
            }
            let flags = if i != 0 {
                Some(0)
            } else if matches!(rd(h, STATE), 0xb | 0x15) {
                Some(1)
            } else {
                None
            };
            if let Some(flag) = flags {
                let texts = [
                    T_NUMBER, T_PLACE, T_DATE, T_NAME, T_TIME, T_PARTY[0], T_PARTY[1], T_PARTY[2],
                    T_PICTURE,
                ];
                let mut args = [0u32; 12];
                args[0] = row;
                for (slot, t) in args[1..10].iter_mut().zip(texts) {
                    *slot = h.rebase(t);
                }
                args[10] = flag;
                h.call_sym(FILE_ROW, Conv::Cdecl, &args);
            }
            row = row.wrapping_add(1);
            i = i.wrapping_add(1);
        }
    }
}
