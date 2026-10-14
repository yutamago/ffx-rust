//! JSON form of a [`MotionGroup`], for editing: every field of the file,
//! with each bone's nine channels as `"zero"`, `"one"`, a constant, or an
//! array of per-frame keys, and scripts as hex.

use serde_json::{Value, json};

use crate::clip::{Channel, Clip};
use crate::group::{ClipEntry, Motion, MotionGroup, Sequence};
use crate::{Error, Result};

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut s, b| {
        write!(s, "{b:02x}").expect("string");
        s
    })
}

fn unhex(s: &str) -> Result<Vec<u8>> {
    let s: String = s.split_whitespace().collect();
    if s.len() % 2 == 1 {
        return Err(Error::Json("odd hex length".into()));
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| Error::Json(e.to_string())))
        .collect()
}

fn channel_json(c: &Channel) -> Value {
    match c {
        Channel::Zero => json!("zero"),
        Channel::One => json!("one"),
        Channel::Constant(v) => json!(v),
        Channel::Keys(k) => json!(k),
    }
}

fn channel(v: &Value) -> Result<Channel> {
    Ok(match v {
        Value::String(s) if s == "zero" => Channel::Zero,
        Value::String(s) if s == "one" => Channel::One,
        Value::Number(_) => Channel::Constant(int(v, "constant")?),
        Value::Array(keys) => Channel::Keys(
            keys.iter()
                .map(|k| int(k, "key"))
                .collect::<Result<Vec<_>>>()?,
        ),
        _ => return Err(Error::Json(format!("bad channel {v}"))),
    })
}

fn int<T: TryFrom<i64>>(v: &Value, what: &str) -> Result<T> {
    v.as_i64()
        .and_then(|n| T::try_from(n).ok())
        .ok_or_else(|| Error::Json(format!("{what}: expected an integer in range, got {v}")))
}

fn field<'a>(v: &'a Value, key: &str) -> Result<&'a Value> {
    v.get(key)
        .ok_or_else(|| Error::Json(format!("missing \"{key}\"")))
}

fn list<'a>(v: &'a Value, key: &str) -> Result<&'a [Value]> {
    field(v, key)?
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| Error::Json(format!("\"{key}\" is not an array")))
}

impl MotionGroup {
    /// The group as JSON.
    #[must_use]
    pub fn to_json(&self) -> Value {
        let sequences: Vec<Value> = self
            .sequences
            .iter()
            .map(|s| {
                let motions: Vec<Value> = s
                    .motions
                    .iter()
                    .map(|m| {
                        json!({"number": m.number, "word2": m.word2, "labels": m.labels,
                               "script": hex(&m.script)})
                    })
                    .collect();
                let clips: Vec<Value> = s
                    .clips
                    .iter()
                    .map(|c| {
                        let clip = &c.clip;
                        let bones: Vec<Value> = clip
                            .channels
                            .chunks(9)
                            .map(|b| Value::Array(b.iter().map(channel_json).collect()))
                            .collect();
                        json!({
                            "word0": c.word0, "marks": c.marks,
                            "frames": clip.frames, "rate": clip.rate,
                            "header_len": clip.header_len, "word14": clip.word14,
                            "raw_rate": clip.raw_rate, "events": clip.events,
                            "bones": bones,
                        })
                    })
                    .collect();
                json!({"flags": s.flags, "character": s.character, "word6": s.word6,
                       "motions": motions, "clips": clips})
            })
            .collect();
        json!({"sequences": sequences})
    }

    /// A group from [`MotionGroup::to_json`]'s form.
    ///
    /// # Errors
    /// Fails on missing or out-of-range fields, or keys whose count is not
    /// the frame count.
    pub fn from_json(v: &Value) -> Result<Self> {
        let sequences = list(v, "sequences")?
            .iter()
            .map(|s| {
                let motions = list(s, "motions")?
                    .iter()
                    .map(|m| {
                        Ok(Motion {
                            number: int(field(m, "number")?, "number")?,
                            word2: int(field(m, "word2")?, "word2")?,
                            labels: list(m, "labels")?
                                .iter()
                                .map(|l| int(l, "label"))
                                .collect::<Result<_>>()?,
                            script: unhex(field(m, "script")?.as_str().unwrap_or_default())?,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                let clips = list(s, "clips")?
                    .iter()
                    .map(|c| {
                        let frames: u16 = int(field(c, "frames")?, "frames")?;
                        let mut channels = Vec::new();
                        for bone in list(c, "bones")? {
                            let bone = bone
                                .as_array()
                                .filter(|b| b.len() == 9)
                                .ok_or_else(|| Error::Json("a bone has nine channels".into()))?;
                            for ch in bone {
                                let ch = channel(ch)?;
                                if let Channel::Keys(k) = &ch
                                    && k.len() != usize::from(frames)
                                {
                                    return Err(Error::Json(format!(
                                        "{} keys for {frames} frames",
                                        k.len()
                                    )));
                                }
                                channels.push(ch);
                            }
                        }
                        let raw_rate = match field(c, "raw_rate")? {
                            Value::Null => None,
                            r => Some(int(r, "raw_rate")?),
                        };
                        if raw_rate.is_some()
                            && channels.iter().any(|c| !matches!(c, Channel::Keys(_)))
                        {
                            return Err(Error::Json("raw clips have only keys".into()));
                        }
                        let header_len: u32 = int(field(c, "header_len")?, "header_len")?;
                        let has_events = !list(c, "events")?.is_empty();
                        if !matches!(header_len, 0x10 | 0x18) || (has_events && header_len != 0x18)
                        {
                            return Err(Error::Json(
                                "header_len is 16 or 24 (24 with events)".into(),
                            ));
                        }
                        Ok(ClipEntry {
                            word0: int(field(c, "word0")?, "word0")?,
                            marks: list(c, "marks")?
                                .iter()
                                .map(|m| int(m, "mark"))
                                .collect::<Result<_>>()?,
                            clip: Clip {
                                frames,
                                rate: int(field(c, "rate")?, "rate")?,
                                channels,
                                events: list(c, "events")?
                                    .iter()
                                    .map(|e| {
                                        let e = e.as_array().filter(|e| e.len() == 4).ok_or_else(
                                            || Error::Json("an event has four words".into()),
                                        )?;
                                        Ok([
                                            int(&e[0], "event")?,
                                            int(&e[1], "event")?,
                                            int(&e[2], "event")?,
                                            int(&e[3], "event")?,
                                        ])
                                    })
                                    .collect::<Result<_>>()?,
                                header_len,
                                word14: int(field(c, "word14")?, "word14")?,
                                raw_rate,
                            },
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                Ok(Sequence {
                    flags: int(field(s, "flags")?, "flags")?,
                    character: int(field(s, "character")?, "character")?,
                    word6: int(field(s, "word6")?, "word6")?,
                    motions,
                    clips,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self { sequences })
    }
}
