//! A script block: the part of a monster file (section 0) or event file
//! (`EV01` section 0) the interpreter runs.
//!
//! Recovered from `Script_Run` (`0x008641e0`) and its helpers, and checked
//! against all 758 shipped scripts (see `docs/formats/script.md`):
//!
//! ```text
//! block +0x00  u32   code length
//! block +0x10  u32   block length
//! block +0x30  u32   code offset (the interpreter's only use of the block header)
//! block +0x34  u16   worker count
//! block +0x38  u32*  worker header offsets
//!
//! worker +0x00  u16  type
//!        +0x02  u16  variable count      +0x14  u32  variable table (8 bytes each)
//!        +0x04  u16  int constant count  +0x18  u32  int constants
//!        +0x06  u16  float count         +0x1c  u32  float constants
//!        +0x08  u16  entry count         +0x20  u32  entry points (u32 code offsets)
//!        +0x0a  u16  label count         +0x24  u32  jump labels (u32 code offsets)
//! ```
//!
//! Every worker's entries and labels lie in its own contiguous code range;
//! in shipped scripts the ranges follow worker order.

use crate::{Error, Result};

const WORKER_HEADER_LEN: usize = 0x34;

/// One worker: a script object with its own entry points and jump labels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Worker {
    /// Offset of the worker header in the block.
    pub header: usize,
    pub kind: u16,
    /// Entry points, as code offsets.
    pub entries: Vec<u32>,
    /// Jump labels, as code offsets.
    pub labels: Vec<u32>,
    /// The worker's code range, as code offsets.
    pub range: std::ops::Range<u32>,
}

/// One decoded instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Instr {
    /// Code offset.
    pub offset: u32,
    pub opcode: u8,
    pub operand: Option<u16>,
}

impl Instr {
    #[must_use]
    pub fn len(&self) -> u32 {
        if self.operand.is_some() { 3 } else { 1 }
    }

    /// Never true; present for clippy's `len_without_is_empty`.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        false
    }

    /// Appends the instruction's bytes.
    pub fn encode(&self, out: &mut Vec<u8>) {
        match self.operand {
            None => out.push(self.opcode),
            Some(v) => {
                out.push(self.opcode | 0x80);
                out.extend(v.to_le_bytes());
            }
        }
    }
}

/// A parsed script block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Script {
    bytes: Vec<u8>,
    code: usize,
    code_len: usize,
    workers: Vec<Worker>,
}

fn u16_at(b: &[u8], o: usize) -> Result<u16> {
    b.get(o..o + 2)
        .map(|s| u16::from_le_bytes([s[0], s[1]]))
        .ok_or(Error::Malformed("truncated script"))
}

fn u32_at(b: &[u8], o: usize) -> Result<u32> {
    b.get(o..o + 4)
        .map(|s| u32::from_le_bytes(s.try_into().expect("4 bytes")))
        .ok_or(Error::Malformed("truncated script"))
}

fn put_u32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}

impl Script {
    /// Parses a script block.
    ///
    /// # Errors
    /// Fails if a table lies outside the block, or the entries and labels
    /// break the per-worker range rule every shipped script follows.
    pub fn parse(bytes: Vec<u8>) -> Result<Self> {
        let code = u32_at(&bytes, 0x30)? as usize;
        let code_len = u32_at(&bytes, 0x00)? as usize;
        if code
            .checked_add(code_len)
            .is_none_or(|end| end > bytes.len())
        {
            return Err(Error::Malformed("code outside the block"));
        }
        let count = usize::from(u16_at(&bytes, 0x34)?);
        let mut workers = Vec::with_capacity(count);
        for w in 0..count {
            let header = u32_at(&bytes, 0x38 + 4 * w)? as usize;
            if header + WORKER_HEADER_LEN > bytes.len() {
                return Err(Error::Malformed("worker header outside the block"));
            }
            let table = |count_at: usize, offset_at: usize| -> Result<Vec<u32>> {
                let n = usize::from(u16_at(&bytes, header + count_at)?);
                let at = u32_at(&bytes, header + offset_at)? as usize;
                (0..n).map(|i| u32_at(&bytes, at + 4 * i)).collect()
            };
            workers.push(Worker {
                header,
                kind: u16_at(&bytes, header)?,
                entries: table(0x08, 0x20)?,
                labels: table(0x0a, 0x24)?,
                range: 0..0,
            });
        }
        // Each worker runs from its first entry to the next one in code
        // order. Shipped scripts are in worker order; rebuilt ones may have
        // workers appended past the original code, up to the block end.
        let starts: Vec<u32> = workers
            .iter()
            .map(|w| {
                w.entries
                    .iter()
                    .copied()
                    .min()
                    .ok_or(Error::Malformed("worker without entries"))
            })
            .collect::<Result<_>>()?;
        let mut order: Vec<usize> = (0..workers.len()).collect();
        order.sort_by_key(|&i| starts[i]);
        if order.windows(2).any(|p| starts[p[0]] == starts[p[1]]) {
            return Err(Error::Malformed("two workers start at the same offset"));
        }
        let code_len32 = u32::try_from(code_len).map_err(|_| Error::Malformed("code too large"))?;
        let block_end =
            u32::try_from(bytes.len() - code).map_err(|_| Error::Malformed("block too large"))?;
        for (k, &i) in order.iter().enumerate() {
            let end = match order.get(k + 1) {
                Some(&next) => starts[next],
                None if starts[i] >= code_len32 => block_end,
                None => code_len32,
            };
            workers[i].range = starts[i]..end;
            let w = &workers[i];
            if w.entries
                .iter()
                .chain(&w.labels)
                .any(|t| !w.range.contains(t))
            {
                return Err(Error::Malformed("entry or label outside its worker"));
            }
        }
        let script = Self {
            bytes,
            code,
            code_len,
            workers,
        };
        for w in &script.workers {
            let starts: std::collections::HashSet<u32> = script
                .decode(w.range.clone())?
                .iter()
                .map(|i| i.offset)
                .collect();
            if w.entries
                .iter()
                .chain(&w.labels)
                .any(|t| !starts.contains(t))
            {
                return Err(Error::Malformed("entry or label inside an instruction"));
            }
        }
        Ok(script)
    }

    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    #[must_use]
    pub fn workers(&self) -> &[Worker] {
        &self.workers
    }

    /// Decodes the instructions in a code range.
    ///
    /// # Errors
    /// Fails if an instruction runs past the range.
    pub fn decode(&self, range: std::ops::Range<u32>) -> Result<Vec<Instr>> {
        let mut out = Vec::new();
        let mut pc = range.start;
        while pc < range.end {
            let at = self.code + pc as usize;
            let b = self.bytes[at];
            let operand = if b & 0x80 == 0 {
                None
            } else {
                let bytes = self
                    .bytes
                    .get(at + 1..at + 3)
                    .ok_or(Error::Malformed("truncated instruction"))?;
                Some(u16::from_le_bytes([bytes[0], bytes[1]]))
            };
            let instr = Instr {
                offset: pc,
                opcode: b & 0x7f,
                operand,
            };
            pc += instr.len();
            if pc > range.end {
                return Err(Error::Malformed("instruction runs past its worker"));
            }
            out.push(instr);
        }
        Ok(out)
    }

    /// Integer constant `i` of a worker.
    #[must_use]
    pub fn int_const(&self, worker: usize, i: u16) -> Option<i32> {
        let h = self.workers.get(worker)?.header;
        (i < u16_at(&self.bytes, h + 4).ok()?)
            .then(|| {
                u32_at(
                    &self.bytes,
                    u32_at(&self.bytes, h + 0x18).ok()? as usize + 4 * usize::from(i),
                )
                .ok()
            })
            .flatten()
            .map(u32::cast_signed)
    }

    /// Variable `i` of a worker: its location word (type in bits 28–31,
    /// storage in bits 25–27, offset in bits 0–23; `AtelGetRealVarAdrs`,
    /// `0x0086c2e0`) and array length.
    #[must_use]
    pub fn variable(&self, worker: usize, i: u16) -> Option<(u32, u16)> {
        let h = self.workers.get(worker)?.header;
        if i >= u16_at(&self.bytes, h + 2).ok()? {
            return None;
        }
        let at = u32_at(&self.bytes, h + 0x14).ok()? as usize + 8 * usize::from(i);
        Some((
            u32_at(&self.bytes, at).ok()?,
            u16_at(&self.bytes, at + 4).ok()?,
        ))
    }

    /// Float constant `i` of a worker.
    #[must_use]
    pub fn float_const(&self, worker: usize, i: u16) -> Option<f32> {
        let h = self.workers.get(worker)?.header;
        (i < u16_at(&self.bytes, h + 6).ok()?)
            .then(|| {
                u32_at(
                    &self.bytes,
                    u32_at(&self.bytes, h + 0x1c).ok()? as usize + 4 * usize::from(i),
                )
                .ok()
            })
            .flatten()
            .map(f32::from_bits)
    }

    /// Replaces each worker's code, entry points and labels.
    ///
    /// `code[w]` is worker `w`'s new code; `entries[w]` and `labels[w]` are
    /// offsets into it and must keep the original counts. Code that fits in
    /// the worker's original range is written there, padded with `nop`;
    /// longer code is appended to the end of the block, which the
    /// interpreter allows (it only uses the code offset at `+0x30`). Nothing
    /// else in the block moves.
    ///
    /// # Errors
    /// Fails on count mismatches and offsets outside the new code.
    pub fn rebuild(
        &self,
        code: &[Vec<u8>],
        entries: &[Vec<u32>],
        labels: &[Vec<u32>],
    ) -> Result<Self> {
        if code.len() != self.workers.len()
            || entries.len() != code.len()
            || labels.len() != code.len()
        {
            return Err(Error::Shape(
                "one code, entry and label list per worker".into(),
            ));
        }
        let mut bytes = self.bytes.clone();
        for (w, worker) in self.workers.iter().enumerate() {
            if entries[w].len() != worker.entries.len() || labels[w].len() != worker.labels.len() {
                return Err(Error::Shape(format!(
                    "worker {w} must keep {} entries and {} labels",
                    worker.entries.len(),
                    worker.labels.len()
                )));
            }
            let len =
                u32::try_from(code[w].len()).map_err(|_| Error::Shape("code too large".into()))?;
            if entries[w]
                .iter()
                .chain(&labels[w])
                .any(|&t| t >= len.max(1))
            {
                return Err(Error::Shape(format!(
                    "worker {w}: an entry or label points past its code"
                )));
            }
            let original = worker.range.end - worker.range.start;
            let base = if len <= original {
                let at = self.code + worker.range.start as usize;
                bytes[at..at + original as usize].fill(0);
                bytes[at..at + code[w].len()].copy_from_slice(&code[w]);
                worker.range.start
            } else {
                while !bytes.len().is_multiple_of(4) {
                    bytes.push(0);
                }
                let start = u32::try_from(bytes.len() - self.code)
                    .map_err(|_| Error::Shape("block too large".into()))?;
                bytes.extend(&code[w]);
                start
            };
            for (table_at, values) in [(0x20, &entries[w]), (0x24, &labels[w])] {
                let table = u32_at(&bytes, worker.header + table_at)? as usize;
                for (i, v) in values.iter().enumerate() {
                    put_u32(&mut bytes, table + 4 * i, base + v);
                }
            }
        }
        if bytes.len() != self.bytes.len() {
            let len =
                u32::try_from(bytes.len()).map_err(|_| Error::Shape("block too large".into()))?;
            put_u32(&mut bytes, 0x10, len);
        }
        Self::parse(bytes)
    }
}
