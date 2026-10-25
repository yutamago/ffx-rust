//! The FMOD call log `ffx-hook` writes with `FFX_FMOD_LOG`
//! (`crates/ffx-hook/src/fmod_log.rs` documents the format).

use std::collections::BTreeMap;

/// A logged function: an import of `FFX.exe` or a vtable entry of
/// `fmod_event.dll`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    pub index: u32,
    pub dll: String,
    /// `import`, or `vtable@<module base>+<slot offset>`.
    pub site: String,
    pub target: u32,
    /// The decorated name.
    pub name: String,
}

impl Function {
    #[must_use]
    pub fn is_import(&self) -> bool {
        self.site == "import"
    }
}

/// One call from `FFX.exe`.
#[derive(Debug, Clone, PartialEq)]
pub struct Call {
    pub seq: u64,
    pub ms: f64,
    pub frame: u32,
    pub thread: u32,
    /// The `FFX.exe` address the call returns to.
    pub caller: u32,
    pub function: u32,
    /// Stack words the callee popped (`this` included).
    pub words: u32,
    pub args: Vec<u32>,
    pub result: u32,
    /// Argument index -> the word it pointed at after the call.
    pub outs: BTreeMap<usize, u32>,
    /// Argument index -> the string it pointed at before the call.
    pub strings: BTreeMap<usize, String>,
}

impl Call {
    /// The address of the `call` instruction when it is a direct one
    /// (`E8 rel32`, 5 bytes before the return): every import call.
    #[must_use]
    pub fn direct_site(&self) -> u32 {
        self.caller.wrapping_sub(5)
    }
}

/// A whole log.
#[derive(Debug, Clone, Default)]
pub struct Log {
    pub functions: Vec<Function>,
    /// Thread id -> start address (an `FFX.exe` address or `module+offset`).
    pub threads: BTreeMap<u32, String>,
    pub calls: Vec<Call>,
}

/// A line the parser could not read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub text: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: cannot read {:?}", self.line, self.text)
    }
}

impl std::error::Error for ParseError {}

fn hex(s: &str) -> Option<u32> {
    u32::from_str_radix(s, 16).ok()
}

impl Log {
    /// Parses a log. A last line cut short (the game was ended mid-write)
    /// is dropped.
    ///
    /// # Errors
    /// A line that is neither a header nor a call.
    pub fn parse(text: &str) -> Result<Self, ParseError> {
        let mut log = Self::default();
        let lines: Vec<&str> = text.lines().collect();
        let complete = if text.ends_with('\n') {
            lines.len()
        } else {
            lines.len().saturating_sub(1)
        };
        for (i, line) in lines[..complete].iter().enumerate() {
            let bad = || ParseError {
                line: i + 1,
                text: (*line).to_owned(),
            };
            if let Some(rest) = line.strip_prefix("# fn ") {
                let w: Vec<&str> = rest.splitn(5, ' ').collect();
                if w.len() != 5 {
                    return Err(bad());
                }
                log.functions.push(Function {
                    index: w[0].parse().map_err(|_| bad())?,
                    dll: w[1].to_owned(),
                    site: w[2].to_owned(),
                    target: hex(w[3]).ok_or_else(bad)?,
                    name: w[4].to_owned(),
                });
            } else if let Some(rest) = line.strip_prefix("# thread ") {
                let (tid, start) = rest.split_once(' ').ok_or_else(bad)?;
                log.threads
                    .insert(tid.parse().map_err(|_| bad())?, start.to_owned());
            } else if line.starts_with('#') || line.is_empty() {
            } else {
                log.calls.push(parse_call(line).ok_or_else(bad)?);
            }
        }
        Ok(log)
    }

    /// The function a call went to.
    #[must_use]
    pub fn function(&self, call: &Call) -> Option<&Function> {
        self.functions.get(call.function as usize)
    }
}

fn parse_call(line: &str) -> Option<Call> {
    let mut words_in = line.split(' ');
    let mut field = || words_in.next();
    let seq = field()?.parse().ok()?;
    let ms = field()?.parse().ok()?;
    let frame = field()?.parse().ok()?;
    let thread = field()?.parse().ok()?;
    let caller = hex(field()?)?;
    let function = field()?.parse().ok()?;
    let words = field()?.parse().ok()?;
    let args = match field()? {
        "-" => Vec::new(),
        list => list.split(',').map(hex).collect::<Option<Vec<_>>>()?,
    };
    let result = hex(field()?)?;
    let mut outs = BTreeMap::new();
    let mut strings = BTreeMap::new();
    // the rest: o<i>=<hex> and s<i>="<text>" (text may hold spaces)
    let rest: Vec<&str> = words_in.collect();
    let rest = rest.join(" ");
    let mut s = rest.as_str();
    while !s.is_empty() {
        s = s.trim_start();
        if let Some(r) = s.strip_prefix('o') {
            let (i, r) = r.split_once('=')?;
            let end = r.find(' ').unwrap_or(r.len());
            outs.insert(i.parse().ok()?, hex(&r[..end])?);
            s = &r[end..];
        } else {
            let (i, r) = s.strip_prefix('s')?.split_once("=\"")?;
            let mut text = String::new();
            let mut chars = r.char_indices();
            let mut end = None;
            while let Some((at, c)) = chars.next() {
                match c {
                    '\\' => text.push(chars.next()?.1),
                    '"' => {
                        end = Some(at + 1);
                        break;
                    }
                    c => text.push(c),
                }
            }
            strings.insert(i.parse().ok()?, text);
            s = &r[end?..];
        }
    }
    Some(Call {
        seq,
        ms,
        frame,
        thread,
        caller,
        function,
        words,
        args,
        result,
        outs,
        strings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "# fmodlog 1
# fn 0 fmod_event.dll import 10012340 ?start@Event@FMOD@@QAG?AW4FMOD_RESULT@@XZ
# fn 1 fmod_event.dll vtable@10000000+a1b2c 1002b8b0 ?getGroup@EventProjectI@FMOD@@UAG?AW4FMOD_RESULT@@PBD_NPAPAVEventGroup@2@@Z
# thread 4242 006fa700
1 12.500 7 4242 0070931c 0 1 2a3b4c 0
2 13.000 7 4242 00708dfc 1 4 5000,b4a0f0,0,19ff00 0 o3=6000 s1=\"mu \\\"sic\\\"\"
3 13.";

    #[test]
    fn reads_a_log() {
        let log = Log::parse(SAMPLE).unwrap();
        assert_eq!(log.functions.len(), 2);
        assert!(log.functions[0].is_import());
        assert_eq!(log.threads[&4242], "006fa700");
        assert_eq!(log.calls.len(), 2, "the cut last line is dropped");
        let c = &log.calls[1];
        assert_eq!(c.caller, 0x0070_8dfc);
        assert_eq!(c.direct_site(), 0x0070_8df7);
        assert_eq!(c.args, [0x5000, 0xb4_a0f0, 0, 0x19_ff00]);
        assert_eq!(c.outs[&3], 0x6000);
        assert_eq!(c.strings[&1], "mu \"sic\"");
        assert_eq!(log.function(c).unwrap().index, 1);
    }
}
