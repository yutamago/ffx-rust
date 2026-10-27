//! The Iggy call log the hook writes (`crates/ffx-hook/src/iggy_log.rs`),
//! read back and turned into what a SWF player needs: movies, AS3 calls
//! with their arguments (string arrays rebuilt from the value-path calls),
//! ticks, draws and screenshots, in the order they happened.

use std::collections::HashMap;

use anyhow::{Context, Result};
use serde_json::Value as Json;

/// An argument or result as the game passed it (`IggyDataValue`).
#[derive(Debug, Clone, PartialEq)]
pub enum Arg {
    Undefined,
    Bool(bool),
    Number(f64),
    String(String),
    /// An AS3 array of strings the game built (`IggyValueRefCreateArray`,
    /// `IggyValuePathMakeArrayRef`, `IggyValueSetStringUTF8RS`).
    Strings(Vec<String>),
    /// A type the pilot does not convert (the raw type).
    Other(u32),
}

/// One thing that happened, in log order.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// `IggyPlayerCreateFromMemory`: a movie of `size` bytes (FNV-1a 64
    /// `hash`) became player `player`.
    Create {
        player: u32,
        size: u32,
        hash: u64,
    },
    Destroy {
        player: u32,
    },
    /// `IggyPlayerSetDisplaySize`.
    DisplaySize {
        player: u32,
        w: u32,
        h: u32,
    },
    /// `IggyPlayerInitializeAndTickRS` (`init`) or `IggyPlayerTickRS`: one
    /// Flash frame; `update` is the update whose flip followed.
    Tick {
        player: u32,
        init: bool,
        update: Option<u32>,
    },
    /// `IggyPlayerDraw` for the image of `update`.
    Draw {
        player: u32,
        update: Option<u32>,
    },
    /// `IggyPlayerCallFunctionRS`: an AS3 function (an
    /// `ExternalInterface.addCallback` name) called by the game in update
    /// `frame`.
    Call {
        player: u32,
        name: String,
        args: Vec<Arg>,
        frame: u32,
    },
    /// An AS3 `ExternalInterface.call` into the game, with what the game
    /// put in the result path (`None`: nothing, the AS3 side sees
    /// `undefined`).
    Callback {
        player: u32,
        name: String,
        args: Vec<Arg>,
        answer: Option<String>,
    },
    /// `IggyPlayerDispatchEventRS` of a mouse move (`IggyMakeEventMouseMove`,
    /// event type 7) to display pixel `x`, `y`.
    MouseMove {
        player: u32,
        x: i32,
        y: i32,
    },
    /// A harness screenshot of update `update`.
    Shot {
        path: String,
        update: Option<u32>,
    },
}

fn hex(v: &Json) -> u32 {
    v.as_str()
        .and_then(|s| u32::from_str_radix(s, 16).ok())
        .unwrap_or(0)
}

fn word(line: &Json, i: usize) -> u32 {
    line["a"].get(i).map_or(0, hex)
}

fn label(v: &Json) -> Option<u32> {
    v.as_i64().and_then(|l| u32::try_from(l).ok())
}

/// Reads a log (one JSON object per line; lines are sorted by `seq`).
///
/// # Errors
/// A line that is not JSON.
pub fn parse(text: &str) -> Result<Vec<Event>> {
    let mut lines = Vec::new();
    for (i, l) in text.lines().enumerate() {
        if !l.starts_with("{\"seq\"") {
            continue;
        }
        let v: Json = serde_json::from_str(l).with_context(|| format!("line {}", i + 1))?;
        lines.push(v);
    }
    lines.sort_by_key(|v| v["seq"].as_u64().unwrap_or(0));
    Ok(convert(&lines))
}

/// Value paths: path address -> (array ref, index).
#[derive(Default)]
struct Arrays {
    paths: HashMap<u32, (u32, u32)>,
    arrays: HashMap<u32, Vec<String>>,
}

fn arg(v: &Json, arrays: &Arrays) -> Arg {
    match v["t"].as_u64().unwrap_or(0) {
        1 | 2 => Arg::Undefined,
        3 => Arg::Bool(v["b"].as_bool().unwrap_or(false)),
        4 => Arg::Number(v["n"].as_f64().unwrap_or(f64::NAN)),
        5 | 6 => Arg::String(v["s"].as_str().unwrap_or("").to_owned()),
        9 => {
            let r = v["w"].get(2).map_or(0, hex);
            Arg::Strings(arrays.arrays.get(&r).cloned().unwrap_or_default())
        }
        t => Arg::Other(u32::try_from(t).unwrap_or(u32::MAX)),
    }
}

fn args(v: &Json, arrays: &Arrays) -> Vec<Arg> {
    v.as_array()
        .map(|a| a.iter().map(|x| arg(x, arrays)).collect())
        .unwrap_or_default()
}

#[allow(clippy::too_many_lines)]
fn convert(lines: &[Json]) -> Vec<Event> {
    let mut out = Vec::new();
    let mut names: HashMap<u32, String> = HashMap::new();
    let mut arrays = Arrays::default();
    // open callbacks: seq -> index in `out`; result paths of the open one
    let mut open: Vec<(u64, usize)> = Vec::new();
    let mut result_paths: Vec<u32> = Vec::new();
    for v in lines {
        let f = v["f"].as_str().unwrap_or("");
        let p = word(v, 0);
        match f {
            "CreateFromMemory" => out.push(Event::Create {
                player: hex(&v["r"]),
                size: u32::try_from(v["size"].as_u64().unwrap_or(0)).unwrap_or(0),
                hash: v["hash"]
                    .as_str()
                    .and_then(|s| u64::from_str_radix(s, 16).ok())
                    .unwrap_or(0),
            }),
            "Destroy" => out.push(Event::Destroy { player: p }),
            "SetDisplaySize" => out.push(Event::DisplaySize {
                player: p,
                w: word(v, 1),
                h: word(v, 2),
            }),
            "InitializeAndTickRS" | "TickRS" => out.push(Event::Tick {
                player: p,
                init: f == "InitializeAndTickRS",
                update: label(&v["rl"]),
            }),
            "Draw" => out.push(Event::Draw {
                player: p,
                update: label(&v["rl"]),
            }),
            "DispatchEventRS" => {
                let ev: Vec<u32> = v["ev"]
                    .as_array()
                    .map(|a| a.iter().map(hex).collect())
                    .unwrap_or_default();
                if ev.len() >= 4 && ev[0] == 7 {
                    out.push(Event::MouseMove {
                        player: p,
                        x: ev[2].cast_signed(),
                        y: ev[3].cast_signed(),
                    });
                }
            }
            "CreateFastNameUTF8" => {
                names.insert(hex(&v["r"]), v["s"].as_str().unwrap_or("").to_owned());
            }
            "CallFunctionRS" => out.push(Event::Call {
                player: p,
                name: names.get(&word(v, 2)).cloned().unwrap_or_default(),
                args: args(&v["args"], &arrays),
                frame: u32::try_from(v["fr"].as_u64().unwrap_or(0)).unwrap_or(0),
            }),
            "ValueRefCreateArray" => {
                arrays.arrays.insert(hex(&v["r"]), Vec::new());
            }
            "ValuePathFromRef" => {
                arrays.paths.insert(p, (word(v, 2), 0));
            }
            "ValuePathMakeArrayRef" => {
                let parent = arrays.paths.get(&word(v, 1)).map_or(0, |x| x.0);
                arrays.paths.insert(p, (parent, word(v, 2)));
            }
            "ValuePathSetArrayIndex" => {
                if let Some(x) = arrays.paths.get_mut(&p) {
                    x.1 = word(v, 1);
                }
            }
            "CallbackResultPath" => result_paths.push(hex(&v["r"])),
            "ValueSetStringUTF8RS" => {
                let s = v["s"].as_str().unwrap_or("").to_owned();
                if result_paths.contains(&p) {
                    if let Some(&(_, i)) = open.last()
                        && let Event::Callback { answer, .. } = &mut out[i]
                    {
                        *answer = Some(s);
                    }
                } else if let Some(&(r, i)) = arrays.paths.get(&p)
                    && let Some(a) = arrays.arrays.get_mut(&r)
                {
                    let i = i as usize;
                    if a.len() <= i {
                        a.resize(i + 1, String::new());
                    }
                    a[i] = s;
                }
            }
            "cb" => {
                open.push((v["seq"].as_u64().unwrap_or(0), out.len()));
                result_paths.clear();
                out.push(Event::Callback {
                    player: v["player"].as_str().map_or(0, |_| hex(&v["player"])),
                    name: v["name"].as_str().unwrap_or("").to_owned(),
                    args: args(&v["args"], &arrays),
                    answer: None,
                });
            }
            "cbret" => {
                open.pop();
                result_paths.clear();
            }
            "shot" => out.push(Event::Shot {
                path: v["path"].as_str().unwrap_or("").to_owned(),
                update: label(&v["label"]),
            }),
            _ => {}
        }
    }
    out
}

/// The file name of a screenshot path (Windows or WSL), without `.bmp`.
#[must_use]
pub fn shot_name(path: &str) -> &str {
    let name = path.rsplit(['\\', '/']).next().unwrap_or(path);
    name.strip_suffix(".bmp").unwrap_or(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rebuilds_string_arrays_and_names() {
        let log = r#"{"iggylog":1}
{"seq":0,"f":"ValueRefCreateArray","a":["p","2"],"r":"r1"}
{"seq":1,"f":"ValuePathFromRef","a":["a0","p","r1"],"r":"a0"}
{"seq":2,"f":"ValuePathMakeArrayRef","a":["b0","a0","0"],"r":"b0"}
{"seq":3,"f":"ValueSetStringUTF8RS","a":["b0","0","0","0","3"],"r":"1","s":"Off"}
{"seq":4,"f":"ValuePathSetArrayIndex","a":["b0","1"],"r":"b0"}
{"seq":5,"f":"ValueSetStringUTF8RS","a":["b0","0","0","0","2"],"r":"1","s":"On"}
{"seq":6,"f":"CreateFastNameUTF8","a":["p","0","3"],"r":"77","s":"add"}
{"seq":7,"f":"CallFunctionRS","fr":5,"a":["p","0","77","2","0"],"r":"0","args":[{"t":4,"n":1.0},{"t":9,"w":["9","0","r1","0"]}]}
{"seq":9,"f":"cb","player":"p","name":"getLocByStr","args":[{"t":5,"s":"x"}]}
{"seq":10,"f":"CallbackResultPath","a":["p"],"r":"c0"}
{"seq":11,"f":"ValueSetStringUTF8RS","a":["c0","0","0","0","5"],"r":"1","s":"Hello"}
{"seq":12,"f":"cbret","of":9,"r":"1"}
{"seq":13,"f":"shot","path":"C:\\x\\pilot-a.bmp","label":7}
"#;
        let ev = parse(log).unwrap();
        assert_eq!(
            ev[0],
            Event::Call {
                player: 0,
                name: "add".into(),
                args: vec![
                    Arg::Number(1.0),
                    Arg::Strings(vec!["Off".into(), "On".into()])
                ],
                frame: 5
            }
        );
        let Event::Callback { name, answer, .. } = &ev[1] else {
            panic!("{:?}", ev[1]);
        };
        assert_eq!(
            (name.as_str(), answer.as_deref()),
            ("getLocByStr", Some("Hello"))
        );
        let Event::Shot { path, update } = &ev[2] else {
            panic!()
        };
        assert_eq!((shot_name(path), *update), ("pilot-a", Some(7)));
    }
}
