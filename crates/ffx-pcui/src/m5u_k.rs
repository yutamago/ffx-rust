//! M5 stage 1, list `render-pcui` (round 3), batch 6: the page heap's
//! bucket table setup, hash table constructor and hash table release.

/// The originals over game memory.
#[allow(
    clippy::cast_possible_wrap, // the originals' signed arithmetic
    clippy::cast_sign_loss
)]
pub mod bridge {
    use ffx_bridge::Mem;
    use ffx_graphic::host::{Conv, Host};

    pub const PAGE_BUCKET_FILL: u32 = 0x0094_6b20;
    /// The bucket table's `(size, align)` pairs, in the original's order.
    pub const BUCKETS: [(u32, u32); 13] = [
        (4, 4),
        (8, 8),
        (0xc, 0xc),
        (0x10, 0x10),
        (0x14, 0x14),
        (0x18, 0x18),
        (0x1c, 0x1c),
        (0x20, 0x10),
        (0x24, 0x24),
        (0x28, 0x28),
        (0x2c, 0x2c),
        (0x30, 0x10),
        (0x40, 0x10),
    ];
    pub const RESERVE: u32 = 0x0094_39a0;
    pub const COMMIT: u32 = 0x0094_38b0;
    pub const DECOMMIT: u32 = 0x0094_38e0;
    pub const RELEASE: u32 = 0x0094_3940;
    pub const MEMSET_THUNK: u32 = 0x0094_964c;

    fn word(h: &impl Mem, at: u32) -> u32 {
        h.read(at)
    }

    fn cdecl(h: &mut impl Host, f: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Cdecl, args)
    }

    /// `PageHeap_initBuckets_00946060(heap)`: `FUN_00946b20(heap + 0xd0;
    /// size, align)` for the 13 buckets -> the last `eax`.
    pub fn init_buckets(h: &mut impl Host, this: u32) -> u32 {
        let mut r = 0;
        for (size, align) in BUCKETS {
            r = h.call_sym(
                PAGE_BUCKET_FILL,
                Conv::Thiscall(this + 0xd0),
                &[size, align],
            );
        }
        r
    }

    /// Whole pages for `bytes`.
    fn pages(bytes: u32) -> u32 {
        bytes.wrapping_add(0xfff) & 0xffff_f000
    }

    /// `PageHash_ctor_00945630(this; n, owner)` (`ret 8`): the seven words
    /// cleared (`+0xc` owner); with `n`, the slot count is the power of
    /// two from 2 reaching `n * 256 / 102` (signed), its table reserved
    /// and committed in whole pages (counted in `+0x18`), `+4` the table,
    /// cleared -> `this`.
    pub fn hash_ctor(h: &mut impl Host, this: u32, n: u32, owner: u32) -> u32 {
        for k in 0..7 {
            h.write(this + 4 * k, if k == 3 { owner } else { 0 });
        }
        if n == 0 {
            return this;
        }
        let want = ((n << 8) as i32) / 0x66;
        let mut count = 2_i32;
        if want > count {
            while count < want {
                count = count.wrapping_add(count);
            }
        }
        h.write(this, count as u32);
        if count <= 0 {
            return this;
        }
        let bytes = (count as u32).wrapping_shl(2);
        let size = pages(bytes);
        let table = cdecl(h, RESERVE, &[size]);
        cdecl(h, COMMIT, &[table, size]);
        let used = word(h, this + 0x18);
        h.write(this + 0x18, used.wrapping_add(size));
        h.write(this + 4, table);
        cdecl(h, MEMSET_THUNK, &[table, 0, bytes]);
        this
    }

    /// `PageHash_release_009474f0(this)`: the table (`+4`, `+0` slots in
    /// whole pages) decommitted and released, then each overflow page of
    /// the chain `+0x10` (0x1000 bytes); `+0x18` counts them down; the
    /// words `+0..+8` and `+0x10`, `+0x14` cleared.
    pub fn hash_release(h: &mut impl Host, this: u32) {
        let size = pages(word(h, this).wrapping_mul(4));
        let table = word(h, this + 4);
        cdecl(h, DECOMMIT, &[table, size]);
        cdecl(h, RELEASE, &[table, size]);
        let used = word(h, this + 0x18);
        h.write(this + 0x18, used.wrapping_sub(size));
        let mut page = word(h, this + 0x10);
        h.write(this, 0_u32);
        h.write(this + 4, 0_u32);
        h.write(this + 8, 0_u32);
        while page != 0 {
            let next = word(h, page);
            cdecl(h, DECOMMIT, &[page, 0x1000]);
            cdecl(h, RELEASE, &[page, 0x1000]);
            let used = word(h, this + 0x18);
            h.write(this + 0x18, used.wrapping_sub(0x1000));
            page = next;
        }
        h.write(this + 0x14, 0_u32);
        h.write(this + 0x10, 0_u32);
    }
}
