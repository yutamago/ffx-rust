//! A single-producer, single-consumer byte ring in shared memory: the game
//! writes encoded messages, the renderer reads them.
//!
//! Layout: a 64-byte header (write position, read position, capacity, then
//! padding), then `capacity` bytes. Positions only grow; the byte at
//! position `p` lives at `p % capacity`. Messages may wrap around the end.

// The mapping is page-aligned, and the capacity fits the address space of
// both sides, so offsets inside it fit `usize`.
#![allow(clippy::cast_ptr_alignment, clippy::cast_possible_truncation)]

use std::sync::atomic::{AtomicU64, Ordering};

pub const HEADER: usize = 64;

/// A view of a ring in memory the caller owns (a file mapping).
pub struct Ring {
    base: *mut u8,
    capacity: u64,
}

// SAFETY: the ring is shared between processes by design; each side only
// writes its own position and the bytes the other side has released.
unsafe impl Send for Ring {}

impl Ring {
    /// Sets up a ring in `len` bytes at `base` (zeroed by the caller), for
    /// the side that creates the mapping.
    ///
    /// # Safety
    /// `base` must point to `len` writable bytes that outlive the ring.
    #[must_use]
    pub unsafe fn create(base: *mut u8, len: usize) -> Self {
        let capacity = (len - HEADER) as u64;
        // SAFETY: the header is inside the mapping.
        unsafe {
            (*base.cast::<AtomicU64>()).store(0, Ordering::Release);
            (*base.add(8).cast::<AtomicU64>()).store(0, Ordering::Release);
            base.add(16).cast::<u64>().write(capacity);
        }
        Self { base, capacity }
    }

    /// A ring another side created.
    ///
    /// # Safety
    /// As for [`Ring::create`], and the header must be initialised.
    #[must_use]
    pub unsafe fn open(base: *mut u8) -> Self {
        // SAFETY: the creator wrote the capacity.
        let capacity = unsafe { base.add(16).cast::<u64>().read() };
        Self { base, capacity }
    }

    fn write_pos(&self) -> &AtomicU64 {
        // SAFETY: inside the header, 8-aligned (the mapping is page-aligned).
        unsafe { &*self.base.cast::<AtomicU64>() }
    }

    fn read_pos(&self) -> &AtomicU64 {
        // SAFETY: as above.
        unsafe { &*self.base.add(8).cast::<AtomicU64>() }
    }

    #[must_use]
    pub fn capacity(&self) -> u64 {
        self.capacity
    }

    /// Bytes written and not yet read.
    #[must_use]
    pub fn pending(&self) -> u64 {
        self.write_pos().load(Ordering::Acquire) - self.read_pos().load(Ordering::Acquire)
    }

    fn copy_in(&self, at: u64, data: &[u8]) {
        let start = (at % self.capacity) as usize;
        let first = data.len().min(self.capacity as usize - start);
        // SAFETY: both ranges are inside the data area.
        unsafe {
            let area = self.base.add(HEADER);
            std::ptr::copy_nonoverlapping(data.as_ptr(), area.add(start), first);
            std::ptr::copy_nonoverlapping(data.as_ptr().add(first), area, data.len() - first);
        }
    }

    fn copy_out(&self, at: u64, out: &mut [u8]) {
        let start = (at % self.capacity) as usize;
        let first = out.len().min(self.capacity as usize - start);
        // SAFETY: both ranges are inside the data area.
        unsafe {
            let area = self.base.add(HEADER);
            std::ptr::copy_nonoverlapping(area.add(start), out.as_mut_ptr(), first);
            std::ptr::copy_nonoverlapping(area, out.as_mut_ptr().add(first), out.len() - first);
        }
    }

    /// Writes `data` whole, waiting (with `wait`) while the reader makes
    /// room. Data larger than the ring is written in pieces as the reader
    /// takes them.
    pub fn write(&self, data: &[u8], mut wait: impl FnMut()) {
        let mut rest = data;
        while !rest.is_empty() {
            let w = self.write_pos().load(Ordering::Relaxed);
            let free = self.capacity - (w - self.read_pos().load(Ordering::Acquire));
            if free == 0 {
                wait();
                continue;
            }
            let n = rest.len().min(free as usize);
            self.copy_in(w, &rest[..n]);
            self.write_pos().store(w + n as u64, Ordering::Release);
            rest = &rest[n..];
        }
    }

    /// Reads exactly `out.len()` bytes, waiting (with `wait`, which returns
    /// `false` to give up) for the writer.
    pub fn read(&self, out: &mut [u8], mut wait: impl FnMut() -> bool) -> bool {
        let mut done = 0;
        while done < out.len() {
            let r = self.read_pos().load(Ordering::Relaxed);
            let avail = self.write_pos().load(Ordering::Acquire) - r;
            if avail == 0 {
                if !wait() {
                    return false;
                }
                continue;
            }
            let n = (out.len() - done).min(avail as usize);
            self.copy_out(r, &mut out[done..done + n]);
            self.read_pos().store(r + n as u64, Ordering::Release);
            done += n;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_and_streams_more_than_capacity() {
        let mut mem = vec![0u64; (HEADER + 64) / 8];
        let base = mem.as_mut_ptr().cast::<u8>();
        // SAFETY: the buffer outlives both views.
        let (w, r) = unsafe { (Ring::create(base, HEADER + 64), Ring::open(base)) };
        let data: Vec<u8> = (0..200u8).collect();
        let mut got = vec![0u8; 200];
        // Interleave: write 50, read 50.
        for k in 0..4 {
            w.write(&data[k * 50..(k + 1) * 50], || panic!("full"));
            assert!(r.read(&mut got[k * 50..(k + 1) * 50], || false));
        }
        assert_eq!(got, data);
        assert_eq!(r.pending(), 0);
    }
}
