//! Lists a capture's items as drawdiff sees them: `items <capture> [n]`
//! prints one line per item (index, scene flag, match key, fingerprint,
//! label); with `n`, every field and used constant of item `n`.
fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: items <capture> [item]");
    let one: Option<usize> = args.next().and_then(|n| n.parse().ok());
    let c = ffx_drawdiff::capture::Capture::read(std::path::Path::new(&path))
        .unwrap_or_else(|e| panic!("{e}"));
    for (f, frame) in c.frames.iter().enumerate() {
        let whole = frame
            .items
            .iter()
            .flat_map(|i| &i.constants)
            .filter(|k| k.vars.len() == 1 && k.vars[0].name.starts_with("cb"))
            .count();
        println!(
            "frame {f} (scene {}): {} items; {whole} constant windows without reflection",
            frame.scene,
            frame.items.len()
        );
        for (n, it) in frame.items.iter().enumerate() {
            if one.is_some_and(|o| o != n) {
                continue;
            }
            println!(
                "{n:04} {} {:016x} {:016x} {}",
                if it.scene { "S" } else { "-" },
                it.match_key,
                it.fingerprint,
                it.label
            );
            if one.is_some() {
                for fl in &it.fields {
                    println!("  {:<8} {} = {}", fl.class.name(), fl.name, fl.value);
                }
                for k in &it.constants {
                    for v in &k.vars {
                        for (w, (bits, float)) in v.words.iter().enumerate() {
                            let value = match (bits, float) {
                                (Some(b), true) => ffx_drawdiff::Value::F(*b),
                                (Some(b), false) => ffx_drawdiff::Value::U(*b),
                                (None, _) => ffx_drawdiff::Value::Absent,
                            };
                            println!(
                                "  constant {}.cb{}.{} = {value}",
                                k.stage,
                                k.slot,
                                v.word_name(w)
                            );
                        }
                    }
                }
            }
        }
    }
}
