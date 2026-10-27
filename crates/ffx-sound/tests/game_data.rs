//! Every bank of the game parses, its samples fit their data, and decoded
//! audio is smooth (a wrong layout decodes to noise). Needs `FFX_GAME_DIR`;
//! skipped otherwise. Slow in debug builds: run with `--release`.

use std::path::PathBuf;

use ffx_sound::fev::{Group, Project};
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

/// Every FMOD Designer project of the game parses to the end of its `LGCY`
/// chunk, and the facts `docs/formats/sound.md` states hold for all of them.
#[test]
fn every_fev_parses() {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return;
    };
    let archive = ffx_vbf::Archive::open(PathBuf::from(dir).join("data/FFX_Data.vbf")).unwrap();
    let (mut files, mut events, mut oneshot) = (0, 0, 0);
    for e in archive
        .entries()
        .iter()
        .filter(|e| e.path.ends_with(".fev"))
    {
        let bytes = archive.read(e).unwrap();
        let p = Project::parse(&bytes).unwrap_or_else(|err| panic!("{}: {err}", e.path));
        assert_eq!(p.version, 0x0045_0000, "{}", e.path);
        assert_eq!(p.banks.len(), 1, "{}", e.path);
        let sfx = e.path.contains("/sfx/");
        assert_eq!(
            p.banks[0].mode,
            if sfx { 0x200 } else { 0x80 },
            "{}",
            e.path
        );
        assert_eq!(p.banks[0].name, format!("{}_bank00", p.name), "{}", e.path);
        assert_eq!(p.configs.len(), 1, "{}", e.path);
        let mut stack: Vec<&Group> = p.groups.iter().collect();
        while let Some(g) = stack.pop() {
            stack.extend(g.groups.iter());
            for ev in &g.events {
                assert_eq!(ev.layers.len(), 1, "{} {}", e.path, ev.name);
                let (wave, once) = p.wave(ev).unwrap();
                assert_eq!(wave.bank, 0);
                assert_eq!(wave.weight, 100);
                assert_eq!([ev.pitch, ev.pitch_randomization].map(f32::to_bits), [0, 0]);
                assert_eq!(ev.volume_randomization.to_bits(), 0);
                assert_eq!(ev.flags, 0x0018_0008);
                let mode = match g.name.as_str() {
                    "loop" => 0x100,
                    "nonloop" => 0x200,
                    _ => 0x80,
                };
                assert_eq!(ev.mode, mode, "{} {}", e.path, ev.name);
                if g.name == "nonloop" {
                    assert!(once, "{} {}", e.path, ev.name);
                    let want = match ev.name.as_str() {
                        "1" => 1,
                        "114" => 30,
                        _ => 5,
                    };
                    assert_eq!(ev.max_playbacks, want, "{} {}", e.path, ev.name);
                } else {
                    assert_eq!(ev.max_playbacks, 1, "{} {}", e.path, ev.name);
                }
                if g.name == "music" {
                    assert_eq!(once, ev.name.ends_with("_oneshot"), "{}", ev.name);
                }
                if sfx {
                    assert_eq!(ev.volume.to_bits(), 1f32.to_bits());
                }
                events += 1;
                oneshot += usize::from(once);
            }
        }
        files += 1;
    }
    assert_eq!(files, 1260);
    eprintln!("{files} projects, {events} events, {oneshot} play once");
}
