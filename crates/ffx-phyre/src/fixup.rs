//! Packed pointer fixups: which object each pointer in the instance data
//! points to. Reimplements the unpacker at `0x60db20` / `0x60a750` (parity
//! test in `ffx-oracle`).
//!
//! Each instance list's `m_pointerFixupCount` records are packed as groups.
//! A group starts with a byte (mode in bits 0–2, flags in bits 3–7) and a
//! LEB128 member word, rotated right by one bit. The group then lists the
//! objects it applies to. The modes are:
//!
//! - 0: every object;
//! - 1: subgroups, each with its own kind byte;
//! - 2: listed objects;
//! - 3: all objects but the listed ones;
//! - 4: a bitmask;
//! - 5: explicit records;
//! - 6: an arithmetic run.
//!
//! The per-record values are LEB128 integers, and the flags say which of
//! them are shared by the group and therefore omitted.

use crate::cluster::Cluster;
use crate::{Error, Result};

/// One unpacked pointer fixup (the engine's 28-byte record).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PointerFixup(pub [u32; 7]);

impl PointerFixup {
    /// The field: with bit 31 set, the byte offset of the pointer within
    /// the object (used for fields inside embedded members). Otherwise the
    /// member's index in the namespace ([`Cluster::member_by_index`]); the
    /// loader looks it up in the same way (`0x57a280`).
    #[must_use]
    pub fn member(&self) -> u32 {
        self.0[0]
    }
    /// The object holding the pointer, as an index into its list.
    #[must_use]
    pub fn object(&self) -> u32 {
        self.0[1]
    }
    /// The first target object, as an index into the target list.
    #[must_use]
    pub fn target(&self) -> u32 {
        self.0[2]
    }
    /// The target instance list.
    #[must_use]
    pub fn target_list(&self) -> u32 {
        self.0[3]
    }
    /// Byte offset added to the target object's address.
    #[must_use]
    pub fn target_offset(&self) -> u32 {
        self.0[4]
    }
    /// The number of consecutive targets (array pointers).
    #[must_use]
    pub fn count(&self) -> u32 {
        self.0[5]
    }
    /// The user fixup this field takes its value from.
    #[must_use]
    pub fn user_fixup(&self) -> Option<u32> {
        (self.0[6] != u32::MAX).then_some(self.0[6])
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn byte(&mut self) -> Result<u8> {
        let b = *self
            .bytes
            .get(self.at)
            .ok_or(Error::Malformed("truncated pointer fixups"))?;
        self.at += 1;
        Ok(b)
    }

    /// LEB128, accumulated in 64 bits and truncated like the original.
    #[allow(clippy::cast_possible_truncation)]
    fn leb(&mut self) -> Result<u32> {
        let (mut v, mut shift) = (0u64, 0u32);
        loop {
            let b = self.byte()?;
            if shift < 64 {
                v |= u64::from(b & 0x7f) << shift;
            }
            shift += 7;
            if b & 0x80 == 0 {
                return Ok(v as u32);
            }
        }
    }

    /// An object index: a byte for lists under 256 objects, else LEB128.
    fn index(&mut self, objects: u32) -> Result<u32> {
        if objects < 0x100 {
            self.byte().map(u32::from)
        } else {
            self.leb()
        }
    }

    /// The per-record values a group's flags leave out of its template:
    /// `Fixup_ReadRecord` (`0x60e730`) for pointers, `0x60ae50` for arrays.
    fn rest(&mut self, kind: Kind, rec: &mut [u32; 7], flags: u32) -> Result<()> {
        if kind == Kind::Array {
            if flags & 8 == 0 {
                rec[2] = self.leb()?;
            }
            rec[3] = self.leb()?;
            return Ok(());
        }
        let mut targets = true;
        if flags & 0x10 == 0 {
            let user = self.leb()?;
            rec[6] = user.wrapping_sub(1);
            targets = user == 0;
        }
        if targets {
            rec[2] = self.leb()?;
            if flags & 0x20 == 0 {
                rec[3] = self.leb()?;
            }
            if flags & 0x40 == 0 {
                rec[4] = self.leb()?;
            }
        }
        if flags & 8 == 0 {
            rec[5] = self.leb()?;
        }
        Ok(())
    }
}

/// The two record formats sharing the group encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// 28-byte records (`Fixup_UnpackList`, `0x60a750`).
    Pointer,
    /// 16-byte records (`Fixup_UnpackArrayList`, `0x60a340`).
    Array,
}

/// How the objects of a run are chosen; shared by group modes and the
/// kinds of mode-1 subgroups. The array variants (`0x60ca10`, `0x60ce70`,
/// `0x60cc20`, `0x60cac0`, `0x60d000`) differ only in the record size.
fn run(
    r: &mut Reader,
    kind: u8,
    objects: u32,
    mut emit: impl FnMut(&mut Reader, u32) -> Result<()>,
) -> Result<u32> {
    match kind {
        // Every object (`0x60ca70`).
        0 => {
            for i in 0..objects {
                emit(r, i)?;
            }
            Ok(objects)
        }
        // Listed objects (`0x60cf40`).
        2 => {
            let n = r.leb()?;
            for _ in 0..n {
                let i = r.index(objects)?;
                emit(r, i)?;
            }
            Ok(n)
        }
        // All but the listed objects (`0x60cd50`); returns how many were left out.
        3 => {
            let n = r.leb()?;
            let mut i = 0;
            for _ in 0..n {
                let skip = r.index(objects)?;
                while i < skip {
                    emit(r, i)?;
                    i += 1;
                }
                i = skip + 1;
            }
            while i < objects {
                emit(r, i)?;
                i += 1;
            }
            Ok(n)
        }
        // A bitmask over the objects, then the records (`0x60cb70`).
        4 => {
            let mask_len = objects.div_ceil(8) as usize;
            let mask = r.at;
            r.at += mask_len;
            if r.at > r.bytes.len() {
                return Err(Error::Malformed("truncated fixups"));
            }
            for i in 0..objects {
                if r.bytes[mask + (i / 8) as usize] & (1 << (i % 8)) != 0 {
                    emit(r, i)?;
                }
            }
            Ok(0)
        }
        // An arithmetic run (`0x60d080`).
        6 => {
            let (start, step, n) = (r.leb()?, r.leb()?, r.leb()?);
            let mut i = start;
            for _ in 0..n {
                emit(r, i)?;
                i = i.wrapping_add(step);
            }
            Ok(n)
        }
        _ => Err(Error::Malformed("unknown fixup group")),
    }
}

/// Unpacks one list's records.
fn unpack_list(
    r: &mut Reader,
    kind: Kind,
    out: &mut Vec<[u32; 7]>,
    count: u32,
    objects: u32,
) -> Result<()> {
    let end = out.len() + count as usize;
    while out.len() < end {
        let head = r.byte()?;
        let mode = head & 7;
        let mut flags = u32::from(head & 0xf9) | 1;
        if objects == 1 {
            flags |= 2;
        }
        let mut template = [
            0,
            0,
            0,
            0,
            0,
            0,
            if kind == Kind::Pointer { u32::MAX } else { 0 },
        ];
        template[0] = r.leb()?.rotate_right(1);
        if kind == Kind::Pointer && flags & 0x20 != 0 {
            template[3] = r.leb()?;
        }
        let push = |out: &mut Vec<[u32; 7]>, t: [u32; 7], i: u32| {
            let mut rec = t;
            rec[1] = i;
            out.push(rec);
        };
        match mode {
            0 | 4 | 6 => {
                run(r, mode, objects, |r, i| {
                    let mut rec = template;
                    rec[1] = i;
                    r.rest(kind, &mut rec, flags)?;
                    out.push(rec);
                    Ok(())
                })?;
            }
            // Indices first, then each record's values.
            2 | 3 => {
                let start = out.len();
                run(r, mode, objects, |_, i| {
                    push(out, template, i);
                    Ok(())
                })?;
                for rec in &mut out[start..] {
                    r.rest(kind, rec, flags)?;
                }
            }
            // Subgroups: a kind byte, the values shared by the subgroup,
            // then its objects.
            1 => {
                let mut t = template;
                let group_end = out.len() + objects as usize;
                while out.len() < group_end {
                    let sub = r.byte()?;
                    r.rest(kind, &mut t, flags)?;
                    if !matches!(sub, 2 | 3 | 4 | 6) {
                        return Err(Error::Malformed("unknown fixup subgroup"));
                    }
                    run(r, sub, objects, |_, i| {
                        push(out, t, i);
                        Ok(())
                    })?;
                }
            }
            5 => {
                let n = r.leb()?;
                for _ in 0..n {
                    let mut rec = template;
                    if flags & 1 == 0 {
                        rec[0] = r.leb()?.rotate_right(1);
                    }
                    if flags & 2 == 0 {
                        rec[1] = r.leb()?;
                    }
                    r.rest(kind, &mut rec, flags)?;
                    out.push(rec);
                }
            }
            _ => return Err(Error::Malformed("unknown fixup group")),
        }
    }
    if out.len() != end {
        return Err(Error::Malformed("fixup group overruns its list"));
    }
    Ok(())
}

fn unpack(packed: &[u8], kind: Kind, lists: &[(u32, u32)]) -> Result<(Vec<[u32; 7]>, usize)> {
    let mut r = Reader {
        bytes: packed,
        at: 0,
    };
    let mut out = Vec::new();
    for &(count, objects) in lists {
        unpack_list(&mut r, kind, &mut out, count, objects)?;
    }
    Ok((out, r.at))
}

/// Unpacks the packed pointer fixups of `lists` (pairs of fixup count and
/// object count, one per instance list), like `Cluster_UnpackPointerFixups`
/// (`0x60db20`). Returns the records in list order and the number of bytes
/// read.
///
/// # Errors
/// Fails on truncated data and unknown group modes.
pub fn unpack_pointer_fixups(
    packed: &[u8],
    lists: &[(u32, u32)],
) -> Result<(Vec<PointerFixup>, usize)> {
    let (recs, used) = unpack(packed, Kind::Pointer, lists)?;
    Ok((recs.into_iter().map(PointerFixup).collect(), used))
}

/// One unpacked array fixup (the engine's 16-byte record).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArrayFixup(pub [u32; 4]);

impl ArrayFixup {
    /// The field, as in [`PointerFixup::member`].
    #[must_use]
    pub fn member(&self) -> u32 {
        self.0[0]
    }
    /// The object holding the array, as an index into its list.
    #[must_use]
    pub fn object(&self) -> u32 {
        self.0[1]
    }
}

/// Unpacks packed array (or pointer-array) fixups, like
/// `Cluster_UnpackArrayFixups` (`0x60dac0`). `lists` pairs each list's
/// array (or pointer-array) fixup count with its object count.
///
/// # Errors
/// Fails on truncated data and unknown group modes.
pub fn unpack_array_fixups(
    packed: &[u8],
    lists: &[(u32, u32)],
) -> Result<(Vec<ArrayFixup>, usize)> {
    let (recs, used) = unpack(packed, Kind::Array, lists)?;
    Ok((
        recs.into_iter()
            .map(|r| ArrayFixup([r[0], r[1], r[2], r[3]]))
            .collect(),
        used,
    ))
}

impl Cluster {
    /// The pointer fixups, grouped by instance list.
    ///
    /// # Errors
    /// Fails on truncated or malformed fixup data.
    pub fn pointer_fixups(&self, bytes: &[u8]) -> Result<Vec<Vec<PointerFixup>>> {
        let h = &self.header;
        let at = self.fixups_offset + h.pointer_array_fixup_size as usize;
        let packed = bytes
            .get(at..at + h.pointer_fixup_size as usize)
            .ok_or(Error::Malformed("pointer fixups outside the file"))?;
        let lists: Vec<(u32, u32)> = self
            .lists
            .iter()
            .map(|l| (l.pointer_fixups, l.count))
            .collect();
        let (flat, used) = unpack_pointer_fixups(packed, &lists)?;
        if used != packed.len() || flat.len() != h.pointer_fixup_count as usize {
            return Err(Error::Malformed("pointer fixups do not add up"));
        }
        let mut flat = flat.into_iter();
        Ok(lists
            .iter()
            .map(|&(n, _)| flat.by_ref().take(n as usize).collect())
            .collect())
    }

    /// The array fixups, grouped by instance list.
    ///
    /// # Errors
    /// Fails on truncated or malformed fixup data.
    pub fn array_fixups(&self, bytes: &[u8]) -> Result<Vec<Vec<ArrayFixup>>> {
        let h = &self.header;
        let at = self.fixups_offset
            + h.pointer_array_fixup_size as usize
            + h.pointer_fixup_size as usize;
        let packed = bytes
            .get(at..at + h.array_fixup_size as usize)
            .ok_or(Error::Malformed("array fixups outside the file"))?;
        let lists: Vec<(u32, u32)> = self
            .lists
            .iter()
            .map(|l| (l.array_fixups, l.count))
            .collect();
        let (flat, used) = unpack_array_fixups(packed, &lists)?;
        if used != packed.len() || flat.len() != h.array_fixup_count as usize {
            return Err(Error::Malformed("array fixups do not add up"));
        }
        let mut flat = flat.into_iter();
        Ok(lists
            .iter()
            .map(|&(n, _)| flat.by_ref().take(n as usize).collect())
            .collect())
    }
}
