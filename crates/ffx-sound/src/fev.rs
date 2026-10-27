//! FMOD Designer event projects (`sound_pc/**/*.fev`): the events the game
//! creates by group and index, with the properties that shape their sound
//! (`docs/formats/sound.md`, "FMOD Designer event projects").
//!
//! The layout is derived from the data: every `.fev` of `FFX_Data.vbf`
//! (1,260: SFX, voice and music, US and JP) parses to the end of its `LGCY`
//! chunk (`tests/game_data.rs`). Fields that hold one value in every file
//! are kept as numbers and named only where the value says what they are;
//! the rest are documented as constants in `sound.md`.

use std::fmt;

/// Errors reading a project.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("not a RIFF FEV file")]
    Magic,
    #[error("chunk {0} is missing")]
    Missing(&'static str),
    #[error("truncated at {0:#x}")]
    Truncated(usize),
    #[error("string index {0} out of range")]
    Name(u32),
    #[error("{what} at {at:#x}: {value:#x}")]
    Unexpected {
        what: &'static str,
        at: usize,
        value: u32,
    },
}

/// A wave bank the project plays from (`<name>.fsb` next to the `.fev`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bank {
    pub name: String,
    /// How FMOD opens the bank, as `FMOD_MODE` bits: 0x80 streamed (music,
    /// voice), 0x200 a compressed sample in memory (SFX).
    pub mode: u32,
    /// 10 in every project.
    pub max_streams: u32,
}

impl Bank {
    /// The bank is streamed from the file, not loaded.
    #[must_use]
    pub fn streamed(&self) -> bool {
        self.mode & 0x80 != 0
    }
}

/// A category (`master` and its children).
#[derive(Debug, Clone, PartialEq)]
pub struct Category {
    pub name: String,
    pub volume: f32,
    pub pitch: f32,
    pub children: Vec<Category>,
}

/// An event's one layer: which sound definition it plays and how.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layer {
    /// Index into [`Project::sound_defs`].
    pub sound_def: u16,
    /// Plays its wave once; false loops it (between the FSB loop points).
    pub oneshot: bool,
}

/// An event (every event of the game is a simple event, type 0x10, with
/// one layer and one sound definition of one wave).
#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub name: String,
    pub guid: [u8; 16],
    /// Linear gain (dB-rounded values: 0.5012 is -6 dB). 1 for every SFX.
    pub volume: f32,
    /// Pitch, pitch and volume randomisation: 0 in every event.
    pub pitch: f32,
    pub pitch_randomization: f32,
    pub volume_randomization: f32,
    /// 0x80 music and voice, 0x100 SFX group `loop`, 0x200 SFX group
    /// `nonloop` (the bank mode bits again, per event).
    pub mode: u32,
    /// Instances that may play at once: 5 for most SFX, 30 for sequence
    /// 114, 1 for sequence 1, every looping SFX, music and voice.
    pub max_playbacks: u32,
    /// `0x0018_0008` in every event.
    pub flags: u32,
    pub layers: Vec<Layer>,
    pub category: String,
    /// The fixed words between the volume and the layer list, in file order
    /// (`sound.md`; constant over the game's data).
    pub raw: [u32; 36],
}

/// An event group (`loop`, `nonloop`, `music`, `voice`).
#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    pub name: String,
    pub groups: Vec<Group>,
    pub events: Vec<Event>,
}

/// One wave of a sound definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wave {
    /// The designer's file name (`0303/9003.wav`).
    pub file: String,
    /// Index into [`Project::banks`].
    pub bank: u32,
    /// Sample index in that bank.
    pub index: u32,
    /// Length in milliseconds.
    pub length_ms: u32,
    /// 100 in every wave.
    pub weight: u32,
}

/// A sound definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoundDef {
    pub name: String,
    pub config: u32,
    pub waves: Vec<Wave>,
}

/// A project.
#[derive(Debug, Clone, PartialEq)]
pub struct Project {
    pub name: String,
    pub version: u32,
    pub banks: Vec<Bank>,
    pub category: Category,
    pub groups: Vec<Group>,
    /// Sound definition configurations: one per project, every word the
    /// same in all of them (`sound.md`).
    pub configs: Vec<[u32; 18]>,
    pub sound_defs: Vec<SoundDef>,
}

impl fmt::Display for Project {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} banks, {} groups, {} sound defs",
            self.name,
            self.banks.len(),
            self.groups.len(),
            self.sound_defs.len()
        )
    }
}

struct Reader<'a> {
    b: &'a [u8],
    p: usize,
    names: &'a [String],
}

impl Reader<'_> {
    fn bytes(&mut self, n: usize) -> Result<&[u8], Error> {
        let s = self
            .b
            .get(self.p..self.p + n)
            .ok_or(Error::Truncated(self.p))?;
        self.p += n;
        Ok(s)
    }
    fn u32(&mut self) -> Result<u32, Error> {
        let s = self.bytes(4)?;
        Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }
    fn u16(&mut self) -> Result<u16, Error> {
        let s = self.bytes(2)?;
        Ok(u16::from_le_bytes([s[0], s[1]]))
    }
    fn f32(&mut self) -> Result<f32, Error> {
        self.u32().map(f32::from_bits)
    }
    /// `u32 length` (with the NUL), the bytes.
    fn string(&mut self) -> Result<String, Error> {
        let n = self.u32()? as usize;
        let s = self.bytes(n)?;
        let s = s.strip_suffix(&[0]).unwrap_or(s);
        Ok(String::from_utf8_lossy(s).into_owned())
    }
    /// A `u32` index into the `STRR` table.
    fn name(&mut self) -> Result<String, Error> {
        let i = self.u32()?;
        self.names.get(i as usize).cloned().ok_or(Error::Name(i))
    }
    fn expect(&mut self, what: &'static str, want: u32) -> Result<(), Error> {
        let at = self.p;
        let value = self.u32()?;
        if value == want {
            Ok(())
        } else {
            Err(Error::Unexpected { what, at, value })
        }
    }

    fn category(&mut self) -> Result<Category, Error> {
        let name = self.string()?;
        let volume = self.f32()?;
        let pitch = self.f32()?;
        self.u32()?;
        self.u32()?;
        let n = self.u32()?;
        let children = (0..n).map(|_| self.category()).collect::<Result<_, _>>()?;
        Ok(Category {
            name,
            volume,
            pitch,
            children,
        })
    }

    fn group(&mut self) -> Result<Group, Error> {
        let name = self.name()?;
        self.expect("group user properties", 0)?;
        let groups = self.u32()?;
        let events = self.u32()?;
        let groups = (0..groups)
            .map(|_| self.group())
            .collect::<Result<_, _>>()?;
        let events = (0..events)
            .map(|_| self.event())
            .collect::<Result<_, _>>()?;
        Ok(Group {
            name,
            groups,
            events,
        })
    }

    fn event(&mut self) -> Result<Event, Error> {
        self.expect("event type", 0x10)?;
        let name = self.name()?;
        let mut guid = [0; 16];
        guid.copy_from_slice(self.bytes(16)?);
        let mut raw = [0u32; 36];
        for w in &mut raw {
            *w = self.u32()?;
        }
        let layers = self.u32()?;
        let layers = (0..layers)
            .map(|_| {
                let sound_def = self.u16()?;
                let mut w = [0u32; 14];
                for x in &mut w {
                    *x = self.u32()?;
                }
                Ok(Layer {
                    sound_def,
                    oneshot: w[3] != 0,
                })
            })
            .collect::<Result<_, Error>>()?;
        self.expect("event categories", 1)?;
        let category = self.string()?;
        Ok(Event {
            name,
            guid,
            volume: f32::from_bits(raw[0]),
            pitch: f32::from_bits(raw[1]),
            pitch_randomization: f32::from_bits(raw[2]),
            volume_randomization: f32::from_bits(raw[3]),
            mode: raw[4],
            max_playbacks: raw[5],
            flags: raw[7],
            layers,
            category,
            raw,
        })
    }

    fn sound_def(&mut self) -> Result<SoundDef, Error> {
        let name = self.name()?;
        let config = self.u32()?;
        let n = self.u32()?;
        let waves = (0..n)
            .map(|_| {
                self.expect("wave type", 0)?;
                let weight = self.u32()?;
                let file = self.string()?;
                Ok(Wave {
                    file,
                    bank: self.u32()?,
                    index: self.u32()?,
                    length_ms: self.u32()?,
                    weight,
                })
            })
            .collect::<Result<_, Error>>()?;
        Ok(SoundDef {
            name,
            config,
            waves,
        })
    }
}

/// The RIFF chunks of a `.fev`, by tag (padded to even sizes; `LIST`
/// entered).
type Chunk<'a> = ([u8; 4], &'a [u8]);

fn chunks(b: &[u8]) -> Result<Vec<Chunk<'_>>, Error> {
    fn walk<'a>(b: &'a [u8], mut p: usize, end: usize, out: &mut Vec<Chunk<'a>>) {
        while p + 8 <= end {
            let tag = [b[p], b[p + 1], b[p + 2], b[p + 3]];
            let n = u32::from_le_bytes([b[p + 4], b[p + 5], b[p + 6], b[p + 7]]) as usize;
            let body = (p + 8).min(end)..(p + 8 + n).min(end);
            if &tag == b"LIST" {
                walk(b, p + 12, body.end, out);
            } else {
                out.push((tag, &b[body]));
            }
            p += 8 + n + (n & 1);
        }
    }
    if b.len() < 12 || &b[..4] != b"RIFF" || &b[8..12] != b"FEV " {
        return Err(Error::Magic);
    }
    let mut out = Vec::new();
    walk(b, 12, b.len(), &mut out);
    Ok(out)
}

/// The `STRR` table: `u32 count, u32 offset[count]`, NUL-terminated strings.
fn strings(d: &[u8]) -> Result<Vec<String>, Error> {
    let word = |at: usize| {
        d.get(at..at + 4)
            .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]) as usize)
            .ok_or(Error::Truncated(at))
    };
    let n = word(0)?;
    let base = 4 + 4 * n;
    (0..n)
        .map(|i| {
            let start = base + word(4 + 4 * i)?;
            let rest = d.get(start..).ok_or(Error::Truncated(start))?;
            let end = rest.iter().position(|&c| c == 0).unwrap_or(rest.len());
            Ok(String::from_utf8_lossy(&rest[..end]).into_owned())
        })
        .collect()
}

impl Project {
    /// Reads a project.
    ///
    /// # Errors
    /// A file that is not RIFF `FEV `, lacks `FMT `/`LGCY`/`STRR`, or whose
    /// `LGCY` body does not parse to its end.
    pub fn parse(b: &[u8]) -> Result<Self, Error> {
        let chunks = chunks(b)?;
        let find = |tag: &[u8; 4], name: &'static str| {
            chunks
                .iter()
                .find(|(t, _)| t == tag)
                .map(|(_, d)| *d)
                .ok_or(Error::Missing(name))
        };
        let fmt = find(b"FMT ", "FMT")?;
        let version = fmt
            .get(..4)
            .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
            .ok_or(Error::Truncated(0))?;
        let names = strings(find(b"STRR", "STRR")?)?;
        let body = find(b"LGCY", "LGCY")?;
        let mut r = Reader {
            b: body,
            p: 0,
            names: &names,
        };
        // 24 and 32 bytes per event: sizes, not read further
        r.u32()?;
        r.u32()?;
        let name = r.string()?;
        r.expect("project word", 1)?;
        let n = r.u32()?;
        let banks = (0..n)
            .map(|_| {
                let mode = r.u32()?;
                let max_streams = r.u32()?;
                r.bytes(8)?; // a hash
                r.expect("bank word", 0)?;
                Ok(Bank {
                    name: r.string()?,
                    mode,
                    max_streams,
                })
            })
            .collect::<Result<_, Error>>()?;
        let category = r.category()?;
        let n = r.u32()?;
        let groups = (0..n).map(|_| r.group()).collect::<Result<_, _>>()?;
        let n = r.u32()?;
        let configs = (0..n)
            .map(|_| {
                let mut w = [0u32; 18];
                for x in &mut w {
                    *x = r.u32()?;
                }
                r.u16()?;
                Ok(w)
            })
            .collect::<Result<_, Error>>()?;
        let n = r.u32()?;
        let sound_defs = (0..n).map(|_| r.sound_def()).collect::<Result<_, _>>()?;
        r.expect("count before comp", 0)?;
        // the closing `comp` record: u32 size (8, itself and the tag), "comp"
        r.expect("comp size", 8)?;
        let at = r.p;
        if r.bytes(4)? != b"comp" || r.p != body.len() {
            return Err(Error::Unexpected {
                what: "end of LGCY",
                at,
                value: u32::try_from(body.len()).unwrap_or(u32::MAX),
            });
        }
        Ok(Self {
            name,
            version,
            banks,
            category,
            groups,
            configs,
            sound_defs,
        })
    }

    /// A top-level group by name.
    #[must_use]
    pub fn group(&self, name: &str) -> Option<&Group> {
        self.groups.iter().find(|g| g.name == name)
    }

    /// The one wave an event plays and whether it plays once.
    #[must_use]
    pub fn wave(&self, event: &Event) -> Option<(&Wave, bool)> {
        let layer = event.layers.first()?;
        let def = self.sound_defs.get(usize::from(layer.sound_def))?;
        Some((def.waves.first()?, layer.oneshot))
    }
}
