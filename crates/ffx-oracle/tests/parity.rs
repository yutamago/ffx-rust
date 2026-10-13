//! Checks `ffx-core` reimplementations against the original machine code in
//! `FFX.exe`, run under emulation. Needs `FFX_GAME_DIR`; skipped otherwise.

use std::io::Write;
use std::path::PathBuf;

use ffx_oracle::Oracle;

const PATH_TO_LOWER_ASCII: u32 = 0x0060_7d50;
const ZLIB_UNCOMPRESS: u32 = 0x0040_5e20;

fn game_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("FFX_GAME_DIR").map(PathBuf::from);
    if dir.is_none() {
        eprintln!("FFX_GAME_DIR is not set; skipping");
    }
    dir
}

fn oracle() -> Option<Oracle> {
    Some(Oracle::load(game_dir()?.join("FFX.exe")).expect("load FFX.exe"))
}

/// Deterministic xorshift stream for generating inputs.
struct Rng(u32);

impl Rng {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }

    fn below(&mut self, n: u32) -> u32 {
        self.next() % n
    }

    #[allow(clippy::cast_possible_truncation)] // taking the low byte
    fn bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.next() as u8).collect()
    }
}

#[test]
fn path_to_lower_ascii_matches_original() {
    let Some(mut o) = oracle() else { return };

    // NULL is returned unchanged.
    assert_eq!(o.call_cdecl(PATH_TO_LOWER_ASCII, &[0]).unwrap(), 0);

    let mut rng = Rng(0x1234_5678);
    let mut cases: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"/FFX_Data/GameData/PS3Data/menu/D3D11/skip_US.dds.phyre".to_vec(),
        (1..=255u8).collect(),
    ];
    for _ in 0..2000 {
        let len = rng.below(80) as usize;
        cases.push(rng.bytes(len).into_iter().map(|b| b.max(1)).collect());
    }
    for case in cases {
        o.reset_heap();
        let mut buf = case.clone();
        buf.push(0);
        let ptr = o.alloc_bytes(&buf).unwrap();
        let ret = o.call_cdecl(PATH_TO_LOWER_ASCII, &[ptr]).unwrap();
        let original = o.read(ptr, buf.len()).unwrap();

        let mut ours = case.clone();
        ffx_core::path::to_lower_ascii(&mut ours);
        ours.push(0);
        assert_eq!(original, ours, "input {case:02x?}");
        // EAX ends on the terminator (or stays put for an empty string).
        assert_eq!(
            ret,
            ptr + u32::try_from(case.len()).unwrap(),
            "return value for {case:02x?}"
        );
    }
}

/// Calls the original `uncompress` and returns (code, `dst_len`, dst bytes).
fn original_uncompress(o: &mut Oracle, src: &[u8], dst_capacity: usize) -> (i32, u32, Vec<u8>) {
    o.reset_heap();
    let src_ptr = o.alloc_bytes(src).unwrap();
    let dst_ptr = o.alloc(dst_capacity).unwrap();
    let len_ptr = o
        .alloc_bytes(&u32::try_from(dst_capacity).unwrap().to_le_bytes())
        .unwrap();
    let args = [dst_ptr, len_ptr, src_ptr, u32::try_from(src.len()).unwrap()];
    let code = o.call_cdecl(ZLIB_UNCOMPRESS, &args).unwrap().cast_signed();
    let len = u32::from_le_bytes(o.read(len_ptr, 4).unwrap().try_into().unwrap());
    let dst = o.read(dst_ptr, dst_capacity).unwrap();
    (code, len, dst)
}

/// Compares the original and ours on one input; returns the shared result code.
fn check_uncompress(o: &mut Oracle, src: &[u8], dst_capacity: usize) -> i32 {
    let (code, len, dst) = original_uncompress(o, src, dst_capacity);
    let mut ours = vec![0u8; dst_capacity];
    match ffx_core::zlib::uncompress(&mut ours, src) {
        Ok(n) => {
            assert_eq!(code, 0, "original failed with {code}, ours succeeded");
            assert_eq!(len as usize, n, "output length");
            assert_eq!(&dst[..n], &ours[..n], "output bytes");
        }
        Err(ours_code) => {
            assert_eq!(
                code,
                ours_code,
                "error code (src {} bytes, dst {dst_capacity})",
                src.len()
            );
            // The original leaves dst_len alone on failure.
            assert_eq!(len as usize, dst_capacity, "dst_len changed on failure");
        }
    }
    code
}

fn zlib(data: &[u8], level: u32) -> Vec<u8> {
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::new(level));
    z.write_all(data).unwrap();
    z.finish().unwrap()
}

#[test]
fn zlib_uncompress_matches_original_on_synthetic_streams() {
    let Some(mut o) = oracle() else { return };
    let mut rng = Rng(0x9e37_79b9);
    let mut seen = std::collections::BTreeMap::<i32, u32>::new();
    let mut check = |o: &mut Oracle, src: &[u8], cap: usize| {
        *seen.entry(check_uncompress(o, src, cap)).or_default() += 1;
    };
    for i in 0..300 {
        // Mix of compressible text-like data and noise, 0..=64 KiB.
        let len = rng.below(0x1_0001) as usize;
        let data: Vec<u8> = if i % 2 == 0 {
            rng.bytes(len)
        } else {
            (0..len)
                .map(|j| b"abcdefgh"[(j / 7 + rng.below(2) as usize) % 8])
                .collect()
        };
        let packed = zlib(&data, rng.below(10));

        check(&mut o, &packed, len); // exact fit
        check(&mut o, &packed, len + 100); // room to spare
        if len > 0 {
            let small = rng.below(u32::try_from(len).unwrap()) as usize;
            check(&mut o, &packed, small); // too small
        }
        let cut = rng.below(u32::try_from(packed.len()).unwrap()) as usize;
        check(&mut o, &packed[..cut], len); // truncated

        let mut corrupt = packed.clone();
        let at = rng.below(u32::try_from(corrupt.len()).unwrap()) as usize;
        corrupt[at] ^= 1 << rng.below(8);
        check(&mut o, &corrupt, len); // one flipped bit
    }
    // Guard against a vacuous pass: every outcome must actually occur.
    for code in [0, ffx_core::zlib::Z_DATA_ERROR, ffx_core::zlib::Z_BUF_ERROR] {
        let count = seen.get(&code).copied().unwrap_or(0);
        assert!(count > 10, "code {code} seen only {count} times: {seen:?}");
    }
    eprintln!("result codes: {seen:?}");
}

/// Every compressed block of `metamenu.vbf`, as the game reads it.
#[test]
fn zlib_uncompress_matches_original_on_game_blocks() {
    let Some(dir) = game_dir() else { return };
    let mut o = oracle().unwrap();
    let archive = ffx_vbf::Archive::open(dir.join("data/metamenu.vbf")).unwrap();
    let raw = std::fs::read(dir.join("data/metamenu.vbf")).unwrap();
    let mut blocks = 0;
    for entry in archive.entries() {
        for block in archive.blocks(entry).unwrap() {
            if block.is_compressed() {
                let offset = usize::try_from(block.offset).unwrap();
                let code =
                    check_uncompress(&mut o, &raw[offset..offset + block.stored], block.size);
                assert_eq!(code, 0, "{} block {}", entry.path, block.index);
                blocks += 1;
            }
        }
    }
    assert!(blocks > 100, "only {blocks} compressed blocks found");
}
