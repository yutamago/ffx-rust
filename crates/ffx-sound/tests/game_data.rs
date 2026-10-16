//! Every bank of the game parses, its samples fit their data, and decoded
//! audio is smooth (a wrong layout decodes to noise). Needs `FFX_GAME_DIR`;
//! skipped otherwise. Slow in debug builds: run with `--release`.

use std::path::PathBuf;

use ffx_sound::{fsb::Bank, ima};

#[test]
fn every_bank_reads_and_decodes() {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return;
    };
    let archive = ffx_vbf::Archive::open(PathBuf::from(dir).join("data/FFX_Data.vbf")).unwrap();
    let mut banks = 0;
    let mut samples = 0;
    let mut checked = 0;
    for e in archive
        .entries()
        .iter()
        .filter(|e| e.path.ends_with(".fsb"))
    {
        let bytes = archive.read(e).unwrap();
        let bank = Bank::parse(&bytes).unwrap_or_else(|err| panic!("{}: {err}", e.path));
        assert_eq!(bank.codec, 7, "{}", e.path);
        for s in &bank.samples {
            assert!(s.data.end <= bytes.len(), "{} {}", e.path, s.name);
            // 64 frames per block of 0x24 bytes a channel
            let blocks = (s.frames as usize).div_ceil(ima::BLOCK_SAMPLES);
            assert!(
                blocks * ima::BLOCK * usize::from(s.channels) <= s.data.len(),
                "{} {}: data too short",
                e.path,
                s.name
            );
            if checked < 400 && s.frames > 1000 {
                let pcm = ima::decode(&bytes[s.data.clone()], s.channels, s.frames);
                let ch = usize::from(s.channels);
                #[allow(clippy::cast_precision_loss)] // a mean
                let rough = pcm
                    .windows(ch + 1)
                    .map(|w| f64::from((i32::from(w[ch]) - i32::from(w[0])).unsigned_abs()))
                    .sum::<f64>()
                    / pcm.len() as f64;
                assert!(rough < 2500.0, "{} {}: mean step {rough}", e.path, s.name);
                checked += 1;
            }
        }
        banks += 1;
        samples += bank.samples.len();
    }
    assert!(banks > 1000, "{banks} banks");
    eprintln!("{banks} banks, {samples} samples, {checked} decoded");
}
