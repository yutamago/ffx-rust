//! The run's log: accesses outside the arena's regions, counted and the
//! first few kept (a port that reads past a mapping gets zeros, as
//! [`crate::arena::ArenaMem`] promises, and the report names it).

use std::sync::Mutex;

/// A bad access: address, length, whether a write.
pub type Access = (u32, usize, bool);

/// The count and the first bad accesses.
static BAD: Mutex<(u64, Vec<Access>)> = Mutex::new((0, Vec::new()));

/// Records an access outside the mapped regions.
pub fn bad_access(addr: u32, len: usize, write: bool) {
    if let Ok(mut b) = BAD.lock() {
        b.0 += 1;
        if b.1.len() < 16 {
            b.1.push((addr, len, write));
        }
    }
}

/// How many bad accesses, and the first ones.
#[must_use]
pub fn bad_accesses() -> (u64, Vec<Access>) {
    BAD.lock().map(|b| b.clone()).unwrap_or_default()
}
