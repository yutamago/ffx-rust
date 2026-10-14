//! Round-trip tests against archives built in memory, so they run without
//! game data.

#![allow(clippy::cast_possible_truncation)] // test archives are tiny

use std::io::Write;
use std::path::PathBuf;

use ffx_vbf::{Archive, BLOCK_SIZE, Error, MAGIC};
use md5::{Digest, Md5};

/// Builds an archive the way the shipped ones are laid out: zlib blocks when
/// that saves space, raw blocks otherwise, and `0` for a raw full block.
fn build(files: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut files: Vec<_> = files
        .iter()
        .map(|(path, data)| (<[u8; 16]>::from(Md5::digest(path.as_bytes())), *path, data))
        .collect();
    files.sort_by_key(|f| f.0);

    let mut paths = Vec::new();
    let mut path_offsets = Vec::new();
    for (_, path, _) in &files {
        path_offsets.push(paths.len() as u64);
        paths.extend_from_slice(path.as_bytes());
        paths.push(0);
    }

    let mut block_sizes: Vec<u16> = Vec::new();
    let mut blobs = Vec::new();
    let mut first_blocks = Vec::new();
    for (_, _, data) in &files {
        first_blocks.push(block_sizes.len() as u32);
        let mut blob = Vec::new();
        for chunk in data.chunks(BLOCK_SIZE) {
            let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::best());
            z.write_all(chunk).unwrap();
            let z = z.finish().unwrap();
            let stored: &[u8] = if z.len() < chunk.len() { &z } else { chunk };
            block_sizes.push(if stored.len() == BLOCK_SIZE {
                0
            } else {
                stored.len() as u16
            });
            blob.extend_from_slice(stored);
        }
        blobs.push(blob);
    }

    let n = files.len();
    let header_len = 0x10 + n * 16 + n * 32 + 4 + paths.len() + block_sizes.len() * 2;
    let mut out = Vec::new();
    out.extend_from_slice(&MAGIC.to_le_bytes());
    out.extend_from_slice(&(header_len as u32).to_le_bytes());
    out.extend_from_slice(&(n as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    for (digest, _, _) in &files {
        out.extend_from_slice(digest);
    }
    let mut offset = header_len as u64;
    for (i, (_, _, data)) in files.iter().enumerate() {
        out.extend_from_slice(&first_blocks[i].to_le_bytes());
        out.extend_from_slice(&first_blocks[i].to_le_bytes());
        out.extend_from_slice(&(data.len() as u64).to_le_bytes());
        out.extend_from_slice(&offset.to_le_bytes());
        out.extend_from_slice(&path_offsets[i].to_le_bytes());
        offset += blobs[i].len() as u64;
    }
    out.extend_from_slice(&((paths.len() + 4) as u32).to_le_bytes());
    out.extend_from_slice(&paths);
    for size in &block_sizes {
        out.extend_from_slice(&size.to_le_bytes());
    }
    assert_eq!(out.len(), header_len);
    let digest = Md5::digest(&out);
    for blob in &blobs {
        out.extend_from_slice(blob);
    }
    out.extend_from_slice(&digest);
    out
}

fn write_temp(name: &str, bytes: &[u8]) -> PathBuf {
    let path = std::env::temp_dir().join(format!("ffx-vbf-{}-{name}.vbf", std::process::id()));
    std::fs::write(&path, bytes).unwrap();
    path
}

/// Deterministic noise that zlib cannot shrink.
fn noise(len: usize, mut seed: u32) -> Vec<u8> {
    (0..len)
        .map(|_| {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed as u8
        })
        .collect()
}

#[test]
fn round_trips_raw_compressed_and_mixed_files() {
    let files = vec![
        ("data/empty.bin", Vec::new()),
        ("data/small.txt", b"hello vbf".to_vec()),
        ("data/zeros.bin", vec![0u8; BLOCK_SIZE * 3 + 17]),
        ("data/noise.bin", noise(BLOCK_SIZE * 2 + 5, 7)),
        ("data/exact.bin", noise(BLOCK_SIZE, 9)),
    ];
    let path = write_temp("roundtrip", &build(&files));
    let archive = Archive::open(&path).unwrap();
    assert_eq!(archive.entries().len(), files.len());
    for (name, data) in &files {
        let entry = archive
            .find(name)
            .unwrap_or_else(|| panic!("{name} missing"));
        assert_eq!(&entry.path, name);
        assert_eq!(&archive.read(entry).unwrap(), data, "{name}");
    }
    assert!(archive.find("data/missing.bin").is_none());
    // The game lowercases ASCII and turns `\` into `/` before hashing.
    assert_eq!(
        archive.find("DATA\\Small.TXT").map(|e| e.path.as_str()),
        Some("data/small.txt")
    );
    std::fs::remove_file(path).unwrap();
}

#[test]
fn rejects_wrong_magic() {
    let mut bytes = build(&[("a", b"x".to_vec())]);
    bytes[0] = b'X';
    let path = write_temp("magic", &bytes);
    assert!(matches!(Archive::open(&path), Err(Error::BadMagic(_))));
    std::fs::remove_file(path).unwrap();
}

#[test]
fn rejects_tampered_header() {
    let mut bytes = build(&[("a", b"x".to_vec())]);
    bytes[0x10] ^= 0xff;
    let path = write_temp("digest", &bytes);
    assert!(matches!(
        Archive::open(&path),
        Err(Error::DigestMismatch { .. })
    ));
    std::fs::remove_file(path).unwrap();
}

#[test]
fn pack_reproduces_and_edits() {
    let files = vec![
        ("data/small.txt", b"hello vbf".to_vec()),
        ("data/empty.bin", Vec::new()),
        ("data/zeros.bin", vec![0u8; BLOCK_SIZE * 2 + 3]),
        ("data/noise.bin", noise(BLOCK_SIZE + 9, 3)),
    ];
    let original = build(&files);
    let path = write_temp("pack-base", &original);
    let base = Archive::open(&path).unwrap();

    let mut same = Vec::new();
    let stats = ffx_vbf::pack(&base, &std::collections::BTreeMap::new(), &mut same).unwrap();
    assert_eq!(same, original, "packing without changes is the identity");
    assert_eq!((stats.files, stats.replaced, stats.added), (4, 0, 0));

    let mut changes = std::collections::BTreeMap::new();
    changes.insert(
        "DATA\\Small.TXT".to_string(),
        b"changed and longer than before".to_vec(),
    );
    changes.insert("data/new.bin".to_string(), noise(BLOCK_SIZE * 3, 5));
    let mut edited = Vec::new();
    let stats = ffx_vbf::pack(&base, &changes, &mut edited).unwrap();
    assert_eq!((stats.files, stats.replaced, stats.added), (5, 1, 1));
    let out = write_temp("pack-edited", &edited);
    let archive = Archive::open(&out).unwrap();
    let read = |p: &str| archive.read(archive.find(p).unwrap()).unwrap();
    assert_eq!(read("data/small.txt"), b"changed and longer than before");
    assert_eq!(read("data/new.bin"), noise(BLOCK_SIZE * 3, 5));
    assert_eq!(read("data/zeros.bin"), vec![0u8; BLOCK_SIZE * 2 + 3]);
    assert_eq!(read("data/noise.bin"), noise(BLOCK_SIZE + 9, 3));
    std::fs::remove_file(path).unwrap();
    std::fs::remove_file(out).unwrap();
}
