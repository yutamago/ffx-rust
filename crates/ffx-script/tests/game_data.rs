//! Round-trips every shipped script. Needs `FFX_GAME_DIR`; skipped otherwise.

use std::path::PathBuf;

use ffx_script::{Container, Kind, assemble, disassemble};
use ffx_vbf::Archive;

fn archive() -> Option<Archive> {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return None;
    };
    Some(Archive::open(PathBuf::from(dir).join("data/FFX_Data.vbf")).unwrap())
}

fn script_paths(archive: &Archive) -> Vec<String> {
    let mut paths: Vec<String> = archive
        .entries()
        .iter()
        .map(|e| e.path.clone())
        .filter(|p| {
            p.ends_with(".ebp")
                || (p.starts_with("ffx_ps2/ffx/master/jppc/battle/mon/") && p.ends_with(".bin"))
        })
        .collect();
    paths.sort();
    paths
}

#[test]
fn every_script_disassembles_and_reassembles_exactly() {
    let Some(archive) = archive() else { return };
    let (mut monsters, mut events, mut instructions) = (0, 0, 0);
    for path in script_paths(&archive) {
        let original = archive.read(archive.find(&path).unwrap()).unwrap();
        let file = Container::parse(original.clone()).unwrap_or_else(|e| panic!("{path}: {e}"));
        let text = disassemble(&file.script).unwrap();
        let rebuilt = assemble(&file.script, &text).unwrap_or_else(|e| panic!("{path}: {e}"));
        assert_eq!(
            rebuilt.bytes(),
            file.script.bytes(),
            "{path}: script round trip"
        );
        let repacked = file.replace_script(&rebuilt).unwrap();
        assert_eq!(
            repacked.bytes(),
            &original[..],
            "{path}: container round trip"
        );
        instructions += text.lines().filter(|l| l.starts_with("    ")).count();
        match file.kind {
            Kind::Monster => monsters += 1,
            Kind::Event => events += 1,
        }
    }
    assert_eq!((monsters, events), (361, 397));
    eprintln!(
        "{monsters} monster and {events} event scripts, {instructions} instructions round-tripped"
    );
}

#[test]
fn a_grown_worker_moves_to_the_end_and_keeps_working() {
    let Some(archive) = archive() else { return };
    let path = "ffx_ps2/ffx/master/jppc/battle/mon/_m001/m001.bin";
    let original = archive.read(archive.find(path).unwrap()).unwrap();
    let file = Container::parse(original.clone()).unwrap();
    let text = disassemble(&file.script).unwrap();

    // Insert 600 bytes of no-ops at the start of worker 1: far more than fits.
    let marker = "\nworker 1 type 2\ne0:\n";
    assert!(text.contains(marker), "{}", &text[..300]);
    let edited = text.replacen(marker, &format!("{marker}{}", "    nop\n".repeat(600)), 1);
    let grown = assemble(&file.script, &edited).unwrap();
    assert!(grown.bytes().len() > file.script.bytes().len());
    // Worker 1 now starts past the original code and still decodes.
    let w1 = &grown.workers()[1];
    assert!(w1.range.start >= file.script.workers()[2].range.end);
    // Its code is the inserted no-ops followed by its original code.
    let worker_text = |t: &str| {
        let start = t.find("\nworker 1 ").unwrap();
        let end = t[start + 1..]
            .find("\nworker ")
            .map_or(t.len(), |e| start + 1 + e);
        t[start..end].to_string()
    };
    assert_eq!(
        worker_text(&disassemble(&grown).unwrap()),
        worker_text(&edited)
    );

    // The container grows and later sections keep their contents.
    let repacked = file.replace_script(&grown).unwrap();
    // Read back, the section also holds the zero padding up to the next one.
    let back = repacked.script.bytes();
    assert!(back.starts_with(grown.bytes()) && back[grown.bytes().len()..].iter().all(|&b| b == 0));
    let section = |bytes: &[u8], i: usize| {
        let off =
            |k: usize| u32::from_le_bytes(bytes[4 + 4 * k..8 + 4 * k].try_into().unwrap()) as usize;
        let start = off(i);
        let end = (i + 1..8)
            .map(off)
            .find(|&o| o > start)
            .unwrap_or(bytes.len());
        bytes[start..end].to_vec()
    };
    for i in [1, 2, 4, 5, 6] {
        assert_eq!(
            section(repacked.bytes(), i),
            section(&original, i),
            "section {i}"
        );
    }
    // And it is stable: disassembling and reassembling the grown file changes nothing.
    let again = assemble(&repacked.script, &disassemble(&repacked.script).unwrap()).unwrap();
    assert_eq!(again.bytes(), repacked.script.bytes());
}
