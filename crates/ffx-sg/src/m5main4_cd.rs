//! M5 stage 1, list `main`, round 4: the PC CD emulation's channel reads
//! (`sa_HddExFileRead` for every channel, `FUN_00a43740` for one): a
//! channel's file read from the HDD install's ex directory, from the
//! sector image in HDD mode (whole sectors through the read buffer, the
//! 5-byte header of packed files stripped, the decode step), or from its
//! own file name.
//!
//! A channel (`g_CdChannels`, 0x38 bytes from `0x01127c80`): `+0` the
//! byte count, `+4` the buffer, `+8` the archive, `+0x1c` the sector,
//! `+0x20` the callback, `+0x28` the file, `+0x2c` the file size, bytes
//! `+0x31` active, `+0x32` busy, `+0x33` cancelled, `+0x34` packed.

/// The originals over game memory.
#[allow(clippy::many_single_char_names, clippy::verbose_bit_mask)] // registers; the original's test
pub mod bridge {
    use ffx_bridge::Mem;
    use ffx_graphic::host::{Conv, Host};

    fn at(p: u32, o: u32) -> u32 {
        p.wrapping_add(o)
    }

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// `GetCD()`, `get_read_buffer()`, `cd_channel_clear(i)`,
    /// `hdd_install_find(file, archive)`, `Sa_CdRead_task(sector, n, buf,
    /// &status, file)`, `TkGetFFXCofingAdrs()`, `TkSignalSemaBGThread()`,
    /// `cd_decode_data(ch)`, `sa_read_callback(ch)`,
    /// `graphicCharacterLoadingPush(ch)`, `FUN_00a42f50(ok, name)` (the
    /// open check) (cdecl).
    pub const GET_CD: u32 = 0x0076_d0d0;
    pub const GET_READ_BUFFER: u32 = 0x0076_dc50;
    pub const CHANNEL_CLEAR: u32 = 0x0076_d530;
    pub const HDD_INSTALL_FIND: u32 = 0x0076_d600;
    pub const CD_READ_TASK: u32 = 0x0076_d200;
    pub const GET_CONFIG: u32 = 0x0088_d5e0;
    pub const SIGNAL_BG: u32 = 0x0088_8820;
    pub const DECODE_DATA: u32 = 0x0076_d440;
    pub const READ_CALLBACK: u32 = 0x0076_e640;
    pub const LOADING_PUSH: u32 = 0x0064_2980;
    pub const OPEN_CHECK: u32 = 0x00a4_2f50;
    /// `sceOpen`, `sceClose`, `sceRead`, `sceLseek`, `sceIoctl`,
    /// `FlushCache`, `dbgPrintf`, `task_wait` (cdecl); `memcpy`'s thunk;
    /// `sprintf`'s import slot.
    pub const SCE_OPEN: u32 = 0x0062_fa30;
    pub const SCE_CLOSE: u32 = 0x0062_f610;
    pub const SCE_READ: u32 = 0x0062_fc00;
    pub const SCE_LSEEK: u32 = 0x0062_f8e0;
    pub const SCE_IOCTL: u32 = 0x0062_f8a0;
    pub const FLUSH_CACHE: u32 = 0x0062_f4c0;
    pub const DBG_PRINTF: u32 = 0x0062_f500;
    pub const TASK_WAIT: u32 = 0x0077_a4e0;
    pub const MEMCPY: u32 = 0x0094_9256;
    pub const SPRINTF: u32 = 0x00b0_c3dc;
    /// The channels, their names (0x80 bytes each), `g_FFXHddMode`, the
    /// background decode's busy byte and channel, the ex path buffer and
    /// its format and error message.
    pub const CHANNELS: u32 = 0x0112_7c80;
    pub const NAMES: u32 = 0x022f_b6c0;
    pub const HDD_MODE: u32 = 0x0133_c913;
    pub const BG_BUSY: u32 = 0x0112_8aad;
    pub const BG_CHANNEL: u32 = 0x0231_0c2c;
    pub const EX_PATH: u32 = 0x01a8_5e30;
    pub const EX_FMT: u32 = 0x00b5_1ee0;
    pub const EX_OPEN_ERR: u32 = 0x00b7_ae5c;

    /// `sceIoctl(fd, 1, status)` until the status word is 0
    /// (`task_wait(1)` between polls).
    fn wait_io(h: &mut impl Host, fd: u32, status: u32) {
        cdecl(h, SCE_IOCTL, &[fd, 1, status]);
        while h.read::<u32>(status) != 0 {
            cdecl(h, TASK_WAIT, &[1]);
            cdecl(h, SCE_IOCTL, &[fd, 1, status]);
        }
    }

    /// The same with the descriptor read from the CD state each time.
    fn wait_io_cd(h: &mut impl Host, cd: u32, status: u32) {
        let fd: u32 = h.read(at(cd, 0x5c));
        cdecl(h, SCE_IOCTL, &[fd, 1, status]);
        while h.read::<u32>(status) != 0 {
            cdecl(h, TASK_WAIT, &[1]);
            let fd: u32 = h.read(at(cd, 0x5c));
            cdecl(h, SCE_IOCTL, &[fd, 1, status]);
        }
    }

    fn flush(h: &mut impl Host) {
        cdecl(h, FLUSH_CACHE, &[0]);
    }

    /// `(x + 0x7ff) / 0x800`-style signed sector count: `x / 0x800`
    /// toward zero.
    fn sectors(x: u32) -> u32 {
        (x.cast_signed() / 0x800).cast_unsigned()
    }

    /// The frame words the original hands its callees: the I/O status
    /// (`ebp - 4`) and the CD read status.
    #[derive(Clone, Copy, Debug)]
    pub struct Frame {
        pub io: u32,
        pub cd: u32,
    }

    /// Channel `i`'s read, up to the common tail. `false` when the channel
    /// is idle or busy (nothing done) or was cancelled
    /// (`cd_channel_clear`), `true` after a read.
    fn read_channel(h: &mut impl Host, cd: u32, buffer: u32, i: u32, f: Frame) -> bool {
        let ch = at(h.rebase(CHANNELS), i.wrapping_mul(0x38));
        if h.read::<u8>(at(ch, 0x31)) == 0 || h.read::<u8>(at(ch, 0x32)) != 0 {
            return false;
        }
        h.write(at(ch, 0x32), 1u8);
        if h.read::<u8>(at(ch, 0x33)) != 0 {
            cdecl(h, CHANNEL_CLEAR, &[i]);
            return false;
        }
        h.write(at(cd, 0x74), i);
        h.write(at(cd, 0x70), 1u32);
        let (file, archive): (u32, u32) = (h.read(at(ch, 0x28)), h.read(at(ch, 8)));
        let e = cdecl(h, HDD_INSTALL_FIND, &[file, archive]);
        if e != 0 {
            let path = h.rebase(EX_PATH);
            let fmt = h.rebase(EX_FMT);
            let sprintf: u32 = h.read(h.rebase(SPRINTF));
            h.call(sprintf, Conv::Cdecl, &[path, fmt, at(e, 0x24)]);
            let fd = cdecl(h, SCE_OPEN, &[path, 0x8001]);
            if fd == 0 {
                let m = h.rebase(EX_OPEN_ERR);
                cdecl(h, DBG_PRINTF, &[m, path]);
            }
            wait_io(h, fd, f.io);
            let off: u32 = h.read(at(e, 0xc));
            cdecl(h, SCE_LSEEK, &[fd, off, 0]);
            wait_io(h, fd, f.io);
            flush(h);
            let (buf, n): (u32, u32) = (h.read(at(ch, 4)), h.read(at(e, 0x10)));
            cdecl(h, SCE_READ, &[fd, buf, n]);
            wait_io(h, fd, f.io);
            cdecl(h, SCE_CLOSE, &[fd]);
            flush(h);
            return true;
        }
        if h.read::<u8>(h.rebase(HDD_MODE)) != 0 {
            read_sectors(h, ch, buffer, f.cd);
        } else {
            read_named(h, cd, ch, i, f.io);
        }
        true
    }

    /// The HDD mode read from the sector image.
    fn read_sectors(h: &mut impl Host, ch: u32, buffer: u32, status: u32) {
        let size: u32 = h.read(at(ch, 0x2c));
        let file: u32 = h.read(at(ch, 0x28));
        let sector: u32 = h.read(at(ch, 0x1c));
        let packed = h.read::<u8>(at(ch, 0x34)) != 0;
        let count: u32 = h.read(ch);
        if packed && count < size {
            // the 5-byte header is read with the data and dropped
            let n5 = count.wrapping_add(5);
            let secs = sectors(n5);
            if secs != 0 && n5.cast_signed() > 0 {
                let buf: u32 = h.read(at(ch, 4));
                cdecl(h, CD_READ_TASK, &[sector, n5, buf, status, file]);
                flush(h);
                let buf: u32 = h.read(at(ch, 4));
                let whole = secs << 11;
                cdecl(h, MEMCPY, &[buf, at(buf, 5), whole]);
                let (sector, file): (u32, u32) = (h.read(at(ch, 0x1c)), h.read(at(ch, 0x28)));
                cdecl(
                    h,
                    CD_READ_TASK,
                    &[sector.wrapping_add(secs), 0x800, buffer, status, file],
                );
                flush(h);
                let buf: u32 = h.read(at(ch, 4));
                cdecl(
                    h,
                    MEMCPY,
                    &[
                        at(buf, whole),
                        buffer,
                        n5.wrapping_sub(whole).wrapping_sub(5),
                    ],
                );
            } else {
                cdecl(h, CD_READ_TASK, &[sector, 0x800, buffer, status, file]);
                flush(h);
                let buf: u32 = h.read(at(ch, 4));
                cdecl(h, MEMCPY, &[buf, at(buffer, 5), count]);
            }
            return;
        }
        let secs = sectors(size);
        if secs != 0 {
            let buf: u32 = h.read(at(ch, 4));
            cdecl(h, CD_READ_TASK, &[sector, size, buf, status, file]);
            if size & 0x7ff == 0 {
                flush(h);
            } else {
                let (sector, file): (u32, u32) = (h.read(at(ch, 0x1c)), h.read(at(ch, 0x28)));
                cdecl(
                    h,
                    CD_READ_TASK,
                    &[sector.wrapping_add(secs), 0x800, buffer, status, file],
                );
                tail_copy(h, ch, buffer, size);
            }
        } else {
            cdecl(h, CD_READ_TASK, &[sector, 0x800, buffer, status, file]);
            tail_copy(h, ch, buffer, size);
        }
        if h.read::<u8>(at(ch, 0x34)) == 0 {
            return;
        }
        let cfg = cdecl(h, GET_CONFIG, &[]);
        if h.read::<u8>(at(cfg, 0x10)) == 0 {
            cdecl(h, DECODE_DATA, &[ch]);
            return;
        }
        let busy = h.rebase(BG_BUSY);
        while h.read::<u8>(busy) != 0 {
            cdecl(h, TASK_WAIT, &[1]);
        }
        h.write(busy, 1u8);
        h.write(h.rebase(BG_CHANNEL), ch);
        cdecl(h, SIGNAL_BG, &[]);
        while h.read::<u8>(busy) != 0 {
            cdecl(h, TASK_WAIT, &[1]);
        }
    }

    /// The last partial sector copied from the read buffer.
    fn tail_copy(h: &mut impl Host, ch: u32, buffer: u32, size: u32) {
        flush(h);
        let rem = size & 0x7ff;
        let buf: u32 = h.read(at(ch, 4));
        cdecl(
            h,
            MEMCPY,
            &[buf.wrapping_sub(rem).wrapping_add(size), buffer, rem],
        );
    }

    /// The read of the channel's own file name (no HDD mode).
    fn read_named(h: &mut impl Host, cd: u32, ch: u32, i: u32, status: u32) {
        let path = at(h.rebase(NAMES), i << 7);
        let sector: u32 = h.read(at(ch, 0x1c));
        let mode = if sector == 0 { 0x8001 } else { 1 };
        let fd = cdecl(h, SCE_OPEN, &[path, mode]);
        h.write(at(cd, 0x5c), fd);
        cdecl(h, OPEN_CHECK, &[fd, path]);
        if sector != 0 {
            let fd: u32 = h.read(at(cd, 0x5c));
            let sector: u32 = h.read(at(ch, 0x1c));
            cdecl(h, SCE_LSEEK, &[fd, sector << 11, 0]);
        }
        flush(h);
        let (fd, buf, n): (u32, u32, u32) = (h.read(at(cd, 0x5c)), h.read(at(ch, 4)), h.read(ch));
        cdecl(h, SCE_READ, &[fd, buf, n]);
        if sector == 0 {
            wait_io_cd(h, cd, status);
        }
        let fd: u32 = h.read(at(cd, 0x5c));
        cdecl(h, SCE_CLOSE, &[fd]);
        flush(h);
    }

    /// The CD state after a read: no channel, no descriptor.
    fn idle(h: &mut impl Host, cd: u32) {
        h.write(at(cd, 0x74), u32::MAX);
        h.write(at(cd, 0x70), 0u32);
        h.write(at(cd, 0x5c), u32::MAX);
    }

    /// `sa_HddExFileRead_00a432a0()` (cdecl): every one of the 64 channels
    /// read; after each read the callback (`sa_read_callback`) when set,
    /// the CD state idle, the channel's active and busy bytes cleared.
    /// `f` is the original's frame words (`ebp - 4`, `ebp - 0x10`).
    /// Returns 64 (the loop count).
    pub fn hdd_ex_file_read(h: &mut impl Host, f: Frame) -> u32 {
        let cd = cdecl(h, GET_CD, &[]);
        let buffer = cdecl(h, GET_READ_BUFFER, &[]);
        h.write(f.io, 0u32);
        for i in 0..64 {
            if !read_channel(h, cd, buffer, i, f) {
                continue;
            }
            let ch = at(h.rebase(CHANNELS), i * 0x38);
            if h.read::<u32>(at(ch, 0x20)) != 0 {
                cdecl(h, READ_CALLBACK, &[ch]);
            }
            idle(h, cd);
            h.write(at(ch, 0x31), 0u16);
        }
        64
    }

    /// A host that remembers the last call's `eax`: what the original's
    /// `eax` is when it returns after a run of calls.
    struct Track<'a, H: Host> {
        h: &'a mut H,
        last: u32,
    }

    impl<H: Host> Mem for Track<'_, H> {
        fn read_bytes(&self, addr: u32, out: &mut [u8]) {
            self.h.read_bytes(addr, out);
        }
        fn write_bytes(&mut self, addr: u32, data: &[u8]) {
            self.h.write_bytes(addr, data);
        }
        fn image_base(&self) -> u32 {
            self.h.image_base()
        }
    }

    impl<H: Host> Host for Track<'_, H> {
        fn call(&mut self, addr: u32, conv: Conv, args: &[u32]) -> u32 {
            self.last = self.h.call(addr, conv, args);
            self.last
        }
        fn scratch(&mut self) -> u32 {
            self.h.scratch()
        }
    }

    /// `FUN_00a43740(i)` (cdecl): channel `i` read (the async task the CD
    /// queue registers); after the read the callback queue
    /// (`graphicCharacterLoadingPush`) when the channel has a callback,
    /// else its active and busy bytes cleared; the CD state idle. `f` is
    /// the original's frame words (`ebp - 4`, `ebp - 0xc`). Returns the
    /// last call's `eax`, as the original leaves it.
    pub fn channel_task(h: &mut impl Host, i: u32, f: Frame) -> u32 {
        let mut t = Track { h, last: 0 };
        let h = &mut t;
        let cd = cdecl(h, GET_CD, &[]);
        let buffer = cdecl(h, GET_READ_BUFFER, &[]);
        h.write(f.io, 0u32);
        if !read_channel(h, cd, buffer, i, f) {
            return h.last;
        }
        let ch = at(h.rebase(CHANNELS), i.wrapping_mul(0x38));
        if h.read::<u32>(at(ch, 0x20)) == 0 {
            h.write(at(ch, 0x31), 0u16);
        } else {
            cdecl(h, LOADING_PUSH, &[ch]);
        }
        idle(h, cd);
        h.last
    }
}
