//! M5 `system-b` (round 2): the movie player's frame queue
//! (`CDataFrameQueue`) and its `.vbf` Matroska reader (`VbfMkvReader`).
//!
//! `CDataFrameQueue` is a ring of size-prefixed records between the
//! decoder thread and the player: `+4` the buffer, `+8` the read index,
//! `+0xc` the write index, `+0x10` the write index after the record
//! being reserved, `+0x14` the read index after the record at the front,
//! `+0x18` open (byte), `+0x1c` the "space freed" event and `+0x20` its
//! "initialised" byte, `+0x24` the "record committed" event and `+0x28`
//! its byte, `+0x2c` running (byte), `+0x30` the ring size (the
//! capacity + 8), `+0x34` records popped, `+0x38` records committed,
//! `+0x3c` the free function. The events are `Phyre::PEventFlagWin32`
//! (one handle word).

/// `CDataFrameQueue_vftable`, `VbfMkvReader_vftable`.
pub const DATA_FRAME_QUEUE_VTABLE: u32 = 0x00b7_a9e4;
pub const VBF_MKV_READER_VTABLE: u32 = 0x00b7_a9fc;
/// `Phyre::PEventFlagWin32`: the constructor (thiscall, returns `this`),
/// `initialize(this; flags)` (`ret 4`), `signal(this)`, `wait(this)`
/// and `0x00430240(this)` (`CloseHandle`), all returning 0.
pub const EVENT_CTOR: u32 = 0x0042_fc80;
pub const EVENT_INITIALIZE: u32 = 0x0043_0050;
pub const EVENT_SIGNAL: u32 = 0x0043_0200;
pub const EVENT_WAIT: u32 = 0x0043_02a0;
pub const EVENT_CLOSE: u32 = 0x0043_0240;
/// `Phyre_PhyreMalloc`, `Phyre_PhyreFree` (cdecl): the defaults when
/// `Init` gets no allocator.
pub const PHYRE_MALLOC: u32 = 0x0042_fb40;
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// `operator_new`, `operator_delete` (cdecl).
pub const OPERATOR_NEW: u32 = 0x0063_0310;
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
/// `0x00607c10`: the stream file's destructor (a bare `ret`, thiscall);
/// `0x0041e6a0`: the reader base's destructor (thiscall: its vtable).
pub const STREAM_FILE_DTOR: u32 = 0x0060_7c10;
pub const MKV_READER_BASE_DTOR: u32 = 0x0041_e6a0;
/// `Phyre_PSerialization_PStreamFile_ctor(this; name, a, b, c, d)`
/// (thiscall, `ret 0x14`) and `..._getFileSize(this)` (thiscall).
pub const STREAM_FILE_CTOR: u32 = 0x0060_7bc0;
pub const STREAM_FILE_SIZE: u32 = 0x0060_7dc0;

/// The originals over game memory.
pub mod bridge {
    use super::{
        DATA_FRAME_QUEUE_VTABLE, EVENT_CLOSE, EVENT_CTOR, EVENT_INITIALIZE, EVENT_SIGNAL,
        EVENT_WAIT, MKV_READER_BASE_DTOR, OPERATOR_DELETE, OPERATOR_NEW, PHYRE_FREE, PHYRE_MALLOC,
        STREAM_FILE_CTOR, STREAM_FILE_DTOR, STREAM_FILE_SIZE, VBF_MKV_READER_VTABLE,
    };
    use ffx_graphic::host::{Conv, Host};

    fn byte(h: &impl Host, at: u32) -> u8 {
        h.read(at)
    }

    /// `SetFields_00a3dd00(this; a, b, c, d)` (thiscall, `ret 0x10`):
    /// `+0xc` = `c`, `+0x10` = `d`, `+0x14` = `a`, `+0x18` = `b`; returns
    /// `b`.
    pub fn set_fields(h: &mut impl Host, this: u32, words: [u32; 4]) -> u32 {
        let [first, second, third, fourth] = words;
        h.write(this.wrapping_add(0xc), third);
        h.write(this.wrapping_add(0x10), fourth);
        h.write(this.wrapping_add(0x14), first);
        h.write(this.wrapping_add(0x18), second);
        second
    }

    /// `CDataFrameQueue_ctor(this)` (thiscall): the vtable, closed, both
    /// events constructed and uninitialised; returns `this`.
    pub fn queue_construct(h: &mut impl Host, this: u32) -> u32 {
        let vt = h.rebase(DATA_FRAME_QUEUE_VTABLE);
        h.write(this, vt);
        h.write(this.wrapping_add(0x18), 0u8);
        h.call_sym(EVENT_CTOR, Conv::Thiscall(this.wrapping_add(0x1c)), &[]);
        h.write(this.wrapping_add(0x20), 0u8);
        h.call_sym(EVENT_CTOR, Conv::Thiscall(this.wrapping_add(0x24)), &[]);
        h.write(this.wrapping_add(0x28), 0u8);
        this
    }

    /// The deleting destructor `0x00a3dd60(this; flags)` (thiscall,
    /// `ret 4`): the vtable; deleted when bit 0 is set. Returns `this`.
    pub fn queue_deleting_dtor(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        let vt = h.rebase(DATA_FRAME_QUEUE_VTABLE);
        h.write(this, vt);
        if flags & 1 != 0 {
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[this]);
        }
        this
    }

    /// `CDataFrameQueue_Init(this; capacity, extra, alloc, free)`
    /// (thiscall, `ret 0x10`): the events initialised (flags 3 and 1),
    /// the ring size `capacity + 8`, both counters 0, the free function
    /// (`Phyre_PhyreFree` when null); the buffer from `alloc` (or
    /// `Phyre_PhyreMalloc`) of `extra + 3 + size` bytes, read index
    /// `size - 4`, write index 0, the word at the read index cleared
    /// (written even when the allocation failed, as the original does);
    /// running and open. Returns whether the buffer was allocated.
    pub fn queue_init(
        h: &mut impl Host,
        this: u32,
        capacity: u32,
        extra: u32,
        alloc: u32,
        free: u32,
    ) -> u32 {
        let at = |off: u32| this.wrapping_add(off);
        h.call_sym(EVENT_INITIALIZE, Conv::Thiscall(at(0x1c)), &[3]);
        h.write(at(0x20), 1u8);
        h.call_sym(EVENT_INITIALIZE, Conv::Thiscall(at(0x24)), &[1]);
        h.write(at(0x28), 1u8);
        let size = capacity.wrapping_add(8);
        h.write(at(0x30), size);
        h.write(at(0x34), 0u32);
        h.write(at(0x38), 0u32);
        let free = if free == 0 {
            h.rebase(PHYRE_FREE)
        } else {
            free
        };
        h.write(at(0x3c), free);
        let n = extra.wrapping_add(3).wrapping_add(size);
        let buffer = if alloc == 0 {
            h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[n])
        } else {
            h.call(alloc, Conv::Cdecl, &[n])
        };
        h.write(at(4), buffer);
        let read = h.read::<u32>(at(0x30)).wrapping_sub(4);
        h.write(at(0xc), 0u32);
        h.write(at(8), read);
        h.write(read.wrapping_add(buffer), 0u32);
        let ok = h.read::<u32>(at(4)) != 0;
        h.write(at(0x2c), 1u8);
        h.write(at(0x18), 1u8);
        u32::from(ok)
    }

    /// `0x00a3dfb0(this; out, start, len, limit, event)` (thiscall, `ret
    /// 0x14`): `*out` = `start + len + 4`; while it is not below the
    /// other side's index (`*limit`, plus the ring size when `start` is
    /// past it; unsigned) and the queue runs, `wait(event)`. Stopped:
    /// returns 0 in `al`. Else `*out` wraps by the ring size when it
    /// reached it, and `al` = 1. The rest of `eax` is the last limit.
    pub fn queue_wait_space(
        h: &mut impl Host,
        this: u32,
        out: u32,
        start: u32,
        len: u32,
        limit: u32,
        event: u32,
    ) -> u32 {
        h.write(out, len.wrapping_add(4).wrapping_add(start));
        loop {
            let mut end: u32 = h.read(limit);
            if start > end {
                end = end.wrapping_add(h.read(this.wrapping_add(0x30)));
            }
            let next: u32 = h.read(out);
            if next < end {
                let size: u32 = h.read(this.wrapping_add(0x30));
                if next >= size {
                    h.write(out, next.wrapping_sub(size));
                }
                return (end & !0xff) | 1;
            }
            if byte(h, this.wrapping_add(0x2c)) == 0 {
                return end & !0xff;
            }
            h.call_sym(EVENT_WAIT, Conv::Thiscall(event), &[]);
        }
    }

    /// `CDataFrameQueue_Front(this)` (thiscall): the record at the read
    /// index (its size word there) waited for ([`queue_wait_space`] into
    /// `+0x14` against the write index `+0xc`, event `+0x24`); returns
    /// its address (`buffer + +0x14`), or 0 when the queue stopped.
    pub fn queue_front(h: &mut impl Host, this: u32) -> u32 {
        let at = |off: u32| this.wrapping_add(off);
        let read: u32 = h.read(at(8));
        let buffer: u32 = h.read(at(4));
        let len: u32 = h.read(buffer.wrapping_add(read));
        let r = queue_wait_space(h, this, at(0x14), read, len, at(0xc), at(0x24));
        if r.to_le_bytes()[0] == 0 {
            return 0;
        }
        let next: u32 = h.read(at(0x14));
        next.wrapping_add(h.read(at(4)))
    }

    /// `CDataFrameQueue_Pop(this)` (thiscall): one popped (`+0x34`), the
    /// read index = `+0x14`; when the reserve end `+0x10` is below it
    /// (plus the ring size when the write index is past it, unsigned) the
    /// writer is woken (`signal(+0x1c)`: `Some` of its `eax`). Else the
    /// original returns with `eax` untouched (`None`).
    pub fn queue_pop(h: &mut impl Host, this: u32) -> Option<u32> {
        let at = |off: u32| this.wrapping_add(off);
        let mut next: u32 = h.read(at(0x14));
        let popped: u32 = h.read(at(0x34));
        h.write(at(0x34), popped.wrapping_add(1));
        h.write(at(8), next);
        if h.read::<u32>(at(0xc)) > next {
            next = next.wrapping_add(h.read(at(0x30)));
        }
        if h.read::<u32>(at(0x10)) >= next {
            return None;
        }
        Some(h.call_sym(EVENT_SIGNAL, Conv::Thiscall(at(0x1c)), &[]))
    }

    /// `CDataFrameQueue_Count(this)` (thiscall): committed minus popped
    /// while open, else 0.
    pub fn queue_count(h: &mut impl Host, this: u32) -> u32 {
        if byte(h, this.wrapping_add(0x18)) == 0 {
            return 0;
        }
        let committed: u32 = h.read(this.wrapping_add(0x38));
        committed.wrapping_sub(h.read(this.wrapping_add(0x34)))
    }

    /// `CDataFrameQueue_Close(this)` (thiscall): stopped, both events
    /// signalled (the second a tail jump: its `eax`).
    pub fn queue_close(h: &mut impl Host, this: u32) -> u32 {
        h.write(this.wrapping_add(0x2c), 0u8);
        h.call_sym(EVENT_SIGNAL, Conv::Thiscall(this.wrapping_add(0x1c)), &[]);
        h.call_sym(EVENT_SIGNAL, Conv::Thiscall(this.wrapping_add(0x24)), &[])
    }

    /// `CDataFrameQueue_Reserve(this; len)` (thiscall, `ret 4`): room for
    /// a record of `len` bytes at the write index waited for
    /// ([`queue_wait_space`] into `+0x10` against the read index `+8`,
    /// event `+0x1c`); its size word written; returns its address, or 0
    /// when the queue stopped.
    pub fn queue_reserve(h: &mut impl Host, this: u32, len: u32) -> u32 {
        let at = |off: u32| this.wrapping_add(off);
        let write: u32 = h.read(at(0xc));
        let r = queue_wait_space(h, this, at(0x10), write, len, at(8), at(0x1c));
        if r.to_le_bytes()[0] == 0 {
            return 0;
        }
        let rec = h.read::<u32>(at(4)).wrapping_add(h.read::<u32>(at(0xc)));
        h.write(rec, len);
        rec
    }

    /// `CDataFrameQueue_Commit(this)` (thiscall): the write index = the
    /// reserve end, one committed, the reader woken (tail jump to
    /// `signal(+0x24)`: its `eax`).
    pub fn queue_commit(h: &mut impl Host, this: u32) -> u32 {
        let at = |off: u32| this.wrapping_add(off);
        let end: u32 = h.read(at(0x10));
        let committed: u32 = h.read(at(0x38));
        h.write(at(0x38), committed.wrapping_add(1));
        h.write(at(0xc), end);
        h.call_sym(EVENT_SIGNAL, Conv::Thiscall(at(0x24)), &[])
    }

    /// `CDataFrameQueue_Destroy(this)` (thiscall): when open, the buffer
    /// freed (`+0x3c`, cdecl), both events closed and marked
    /// uninitialised, closed. Returns the last close's `eax` (`Some`);
    /// closed already, the original leaves `eax` untouched (`None`).
    pub fn queue_destroy(h: &mut impl Host, this: u32) -> Option<u32> {
        let at = |off: u32| this.wrapping_add(off);
        if byte(h, at(0x18)) == 0 {
            return None;
        }
        let free: u32 = h.read(at(0x3c));
        let buffer: u32 = h.read(at(4));
        h.call(free, Conv::Cdecl, &[buffer]);
        h.call_sym(EVENT_CLOSE, Conv::Thiscall(at(0x1c)), &[]);
        h.write(at(0x20), 0u8);
        let r = h.call_sym(EVENT_CLOSE, Conv::Thiscall(at(0x24)), &[]);
        h.write(at(0x28), 0u8);
        h.write(at(0x18), 0u8);
        Some(r)
    }

    /// `VbfMkvReader_ctor(this)` (thiscall): the vtable, no file; returns
    /// `this`.
    pub fn reader_construct(h: &mut impl Host, this: u32) -> u32 {
        let vt = h.rebase(VBF_MKV_READER_VTABLE);
        h.write(this, vt);
        h.write(this.wrapping_add(0x10), 0u32);
        this
    }

    /// The file at `+0x10`, when set, destroyed (its bare-`ret`
    /// destructor) and deleted, and cleared.
    fn close_file(h: &mut impl Host, this: u32) {
        let file: u32 = h.read(this.wrapping_add(0x10));
        if file != 0 {
            h.call_sym(STREAM_FILE_DTOR, Conv::Thiscall(file), &[]);
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[file]);
            h.write(this.wrapping_add(0x10), 0u32);
        }
    }

    /// `VbfMkvReader_Close(this)` (thiscall): the file closed. `eax` junk
    /// to the callers (the reader's own methods).
    pub fn reader_close(h: &mut impl Host, this: u32) {
        close_file(h, this);
    }

    /// The deleting destructor `0x00a3e0c0(this; flags)` (thiscall, `ret
    /// 4`): the vtable, the file closed, the base destructor
    /// (`0x0041e6a0`), deleted when bit 0 is set. Returns `this`.
    pub fn reader_deleting_dtor(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        let vt = h.rebase(VBF_MKV_READER_VTABLE);
        h.write(this, vt);
        close_file(h, this);
        h.call_sym(MKV_READER_BASE_DTOR, Conv::Thiscall(this), &[]);
        if flags & 1 != 0 {
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[this]);
        }
        this
    }

    /// `VbfMkvReader_Length(this; total, available)` (thiscall, `ret
    /// 8`): without a file -1; else the size (`+8`, 64 bits) into each
    /// pointer that is set, and 0.
    pub fn reader_length(h: &mut impl Host, this: u32, total: u32, available: u32) -> u32 {
        if h.read::<u32>(this.wrapping_add(0x10)) == 0 {
            return u32::MAX;
        }
        for out in [total, available] {
            if out != 0 {
                let lo: u32 = h.read(this.wrapping_add(8));
                h.write(out, lo);
                let hi: u32 = h.read(this.wrapping_add(0xc));
                h.write(out.wrapping_add(4), hi);
            }
        }
        0
    }

    /// `VbfMkvReader_Open(this; name)` (thiscall, `ret 4`): -1 without a
    /// name or with a file open; else a stream file (`new(8)`, then
    /// `PStreamFile(name, 1, 0, 0, 1)`) at `+0x10`: 1 when the
    /// allocation failed, else its size at `+8` (high word 0) and 0.
    pub fn reader_open(h: &mut impl Host, this: u32, name: u32) -> u32 {
        if name == 0 || h.read::<u32>(this.wrapping_add(0x10)) != 0 {
            return u32::MAX;
        }
        let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[8]);
        let file = if p == 0 {
            0
        } else {
            h.call_sym(STREAM_FILE_CTOR, Conv::Thiscall(p), &[name, 1, 0, 0, 1])
        };
        h.write(this.wrapping_add(0x10), file);
        if file == 0 {
            return 1;
        }
        let size = h.call_sym(STREAM_FILE_SIZE, Conv::Thiscall(file), &[]);
        h.write(this.wrapping_add(8), size);
        h.write(this.wrapping_add(0xc), 0u32);
        0
    }
}
