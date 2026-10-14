//! Motion groups (`.mgrp`): the clips and motion scripts of one or more
//! characters (`SG_AddMotionGroup`, `0x00836c90`; relocated by
//! `SG_RelocateGroup`, `0x00836f70`).
//!
//! ```text
//! group     +0x00 u32 0 (its own address once loaded)
//!           +0x04 u32 sequence count
//!           +0x08 u32 0
//!           +0x0c u32 offset of the sequence table (at the end of the file)
//! sequence  +0x00 u32 flags
//!           +0x04 u16 character (the high half of a motion id)
//!           +0x06 u16 0
//!           +0x08 u16 motion count, +0x0a u16 clip count
//!           +0x0c u32 offset of the motions (16 bytes each)
//!           +0x10 u32 offset of the clips (16 bytes each)
//! clip      +0x00 u32 0, +0x04 u32 mark count
//!           +0x08 u32 offset of the marks (u16 frames, then the clip header
//!                     8-byte aligned), +0x0c u32 offset of the clip header
//! motion    +0x00 u16 number (the low half of a motion id)
//!           +0x02 u16, +0x04 u16 label count, +0x06 u16 script length
//!           +0x08 u32 offset of the labels (u16 each), +0x0c u32 of the script
//! ```
//!
//! A sequence's data is laid out as its clips (marks, then the clip), then
//! its clip table, its labels and scripts, and its motion table. Padding is
//! `0x77`.
//!
//! A motion's script (`SG_SeqStep`, `0x00837900`) plays clips; opcode 1
//! (`u16` arguments, unaligned) is `clip, loops, start mark, end mark`.

use crate::clip::Clip;
use crate::{Error, PAD, Result};

/// A clip with its marks: frames the motion scripts start and stop at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipEntry {
    pub word0: u32,
    pub marks: Vec<u16>,
    pub clip: Clip,
}

/// A motion: a script over the sequence's clips.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Motion {
    pub number: u16,
    pub word2: u16,
    /// Jump targets, as offsets into the script.
    pub labels: Vec<u16>,
    pub script: Vec<u8>,
}

/// The motions of one character.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sequence {
    pub flags: u32,
    pub character: u16,
    pub word6: u16,
    pub motions: Vec<Motion>,
    pub clips: Vec<ClipEntry>,
}

/// A parsed motion group.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MotionGroup {
    pub sequences: Vec<Sequence>,
}

fn u16_at(b: &[u8], o: usize) -> Result<u16> {
    b.get(o..o + 2)
        .map(|s| u16::from_le_bytes([s[0], s[1]]))
        .ok_or(Error::Truncated(o))
}

fn u32_at(b: &[u8], o: usize) -> Result<u32> {
    b.get(o..o + 4)
        .map(|s| u32::from_le_bytes(s.try_into().expect("4 bytes")))
        .ok_or(Error::Truncated(o))
}

fn pad(out: &mut Vec<u8>, align: usize) {
    out.resize(out.len().next_multiple_of(align), PAD);
}

fn put16(out: &mut Vec<u8>, v: usize) {
    out.extend(u16::try_from(v).expect("16-bit field").to_le_bytes());
}

fn put32(out: &mut Vec<u8>, v: usize) {
    out.extend(u32::try_from(v).expect("32-bit field").to_le_bytes());
}

impl MotionGroup {
    /// Parses a motion group. Empty (16-byte) files have no sequences.
    ///
    /// # Errors
    /// Fails on truncated or inconsistent tables.
    pub fn parse(b: &[u8]) -> Result<Self> {
        let count = u32_at(b, 4)? as usize;
        let table = u32_at(b, 0xc)? as usize;
        let sequences = (0..count)
            .map(|s| {
                let at = table + 0x14 * s;
                let motion_count = usize::from(u16_at(b, at + 8)?);
                let clip_count = usize::from(u16_at(b, at + 0xa)?);
                let motions_at = u32_at(b, at + 0xc)? as usize;
                let clips_at = u32_at(b, at + 0x10)? as usize;
                let motions = (0..motion_count)
                    .map(|m| {
                        let e = motions_at + 16 * m;
                        let labels = usize::from(u16_at(b, e + 4)?);
                        let script_len = usize::from(u16_at(b, e + 6)?);
                        let labels_at = u32_at(b, e + 8)? as usize;
                        let script_at = u32_at(b, e + 0xc)? as usize;
                        Ok(Motion {
                            number: u16_at(b, e)?,
                            word2: u16_at(b, e + 2)?,
                            labels: (0..labels)
                                .map(|l| u16_at(b, labels_at + 2 * l))
                                .collect::<Result<_>>()?,
                            script: b
                                .get(script_at..script_at + script_len)
                                .ok_or(Error::Truncated(script_at))?
                                .to_vec(),
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                let clips = (0..clip_count)
                    .map(|c| {
                        let e = clips_at + 16 * c;
                        let marks = u32_at(b, e + 4)? as usize;
                        let marks_at = u32_at(b, e + 8)? as usize;
                        let header_at = u32_at(b, e + 0xc)? as usize;
                        Ok(ClipEntry {
                            word0: u32_at(b, e)?,
                            marks: (0..marks)
                                .map(|k| u16_at(b, marks_at + 2 * k))
                                .collect::<Result<_>>()?,
                            clip: Clip::parse(b, header_at)?.0,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                Ok(Sequence {
                    flags: u32_at(b, at)?,
                    character: u16_at(b, at + 4)?,
                    word6: u16_at(b, at + 6)?,
                    motions,
                    clips,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self { sequences })
    }

    /// Writes the group in the shipped layout.
    ///
    /// # Errors
    /// Fails when a clip cannot be encoded.
    pub fn write(&self) -> Result<Vec<u8>> {
        let mut out = vec![0u8; 16];
        let mut tables = Vec::new();
        for s in &self.sequences {
            let mut clip_offsets = Vec::new();
            for c in &s.clips {
                pad(&mut out, 4);
                let marks = out.len();
                for m in &c.marks {
                    out.extend(m.to_le_bytes());
                }
                pad(&mut out, 8);
                let header = out.len();
                c.clip.write(&mut out)?;
                clip_offsets.push((marks, header));
            }
            pad(&mut out, 4);
            let clips_at = out.len();
            for (c, (marks, header)) in s.clips.iter().zip(&clip_offsets) {
                out.extend(c.word0.to_le_bytes());
                put32(&mut out, c.marks.len());
                put32(&mut out, *marks);
                put32(&mut out, *header);
            }
            let mut motion_offsets = Vec::new();
            for m in &s.motions {
                pad(&mut out, 4);
                let labels = out.len();
                for l in &m.labels {
                    out.extend(l.to_le_bytes());
                }
                let script = out.len();
                out.extend(&m.script);
                motion_offsets.push((labels, script));
            }
            pad(&mut out, 4);
            let motions_at = out.len();
            for (m, (labels, script)) in s.motions.iter().zip(&motion_offsets) {
                out.extend(m.number.to_le_bytes());
                out.extend(m.word2.to_le_bytes());
                put16(&mut out, m.labels.len());
                put16(&mut out, m.script.len());
                put32(&mut out, *labels);
                put32(&mut out, *script);
            }
            tables.push((motions_at, clips_at));
        }
        pad(&mut out, 4);
        let table = out.len();
        for (s, (motions_at, clips_at)) in self.sequences.iter().zip(tables) {
            out.extend(s.flags.to_le_bytes());
            out.extend(s.character.to_le_bytes());
            out.extend(s.word6.to_le_bytes());
            put16(&mut out, s.motions.len());
            put16(&mut out, s.clips.len());
            put32(&mut out, motions_at);
            put32(&mut out, clips_at);
        }
        out[4..8].copy_from_slice(
            &u32::try_from(self.sequences.len())
                .expect("count")
                .to_le_bytes(),
        );
        out[0xc..0x10].copy_from_slice(&u32::try_from(table).expect("small").to_le_bytes());
        Ok(out)
    }
}
