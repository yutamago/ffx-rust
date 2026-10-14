//! The generic parts of a cluster: the packed namespace (class
//! descriptors), the instance lists and the user fixup strings.
//!
//! ```text
//! namespace +0x00  u32  0x01020304
//!           +0x04  u32  size
//!           +0x08  u32  type count T      (plain types: PUInt32, float, enums…)
//!           +0x0c  u32  class count C
//!           +0x10  u32  member count M
//!           +0x14  u32  string table size (the table ends the namespace)
//!           +0x20  u32 × T          type name offsets
//!           …      36 bytes × C     classes: base (1-based class, 0 = none),
//!                                   0x20000000 | size, name, member count, …
//!           …      24 bytes × M     members, consecutive per class: name,
//!                                   type, offset, size, flags, …
//! ```
//!
//! Then `0x24` bytes per instance list (class, count, total size, objects
//! size, arrays size, then array, pointer, and pointer-array fixup counts).
//! Next comes the instance data (each list's objects, then its arrays).
//! After it come the user fixup strings and their records
//! `(type, length, offset)`. Then the header class counts (`u32` each) and
//! the header class fixups (16 bytes each). Last are the packed fixups:
//! pointer-array fixups, pointer fixups ([`crate::fixup`]), and array fixups.

use crate::{Error, HEADER_LEN, Header, Result};

/// A class from the namespace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Class {
    pub name: String,
    /// Index of the base class in [`Cluster::classes`].
    pub base: Option<usize>,
    pub size: u32,
    pub members: Vec<Member>,
}

/// A data member of a class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    pub name: String,
    /// Index into the namespace's types, then (past them, 1-based) classes.
    pub type_index: u32,
    pub offset: u32,
    pub size: u32,
}

/// The objects of one class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceList {
    pub class: usize,
    pub count: u32,
    pub objects_size: u32,
    pub arrays_size: u32,
    /// Number of pointer fixups in this list's objects.
    pub pointer_fixups: u32,
    /// File offset of the first object; arrays follow the objects.
    pub offset: usize,
}

impl InstanceList {
    /// Size of one object.
    #[must_use]
    pub fn object_size(&self) -> usize {
        self.objects_size.checked_div(self.count).unwrap_or(0) as usize
    }

    /// File offset of object `i`.
    #[must_use]
    pub fn object(&self, i: u32) -> usize {
        self.offset + self.object_size() * i as usize
    }

    /// File range of the list's arrays.
    #[must_use]
    pub fn arrays(&self) -> std::ops::Range<usize> {
        let start = self.offset + self.objects_size as usize;
        start..start + self.arrays_size as usize
    }
}

/// A user fixup: a string the loader resolves (class names, enum values…).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserFixup {
    pub type_index: u32,
    pub text: String,
}

/// A parsed cluster.
#[derive(Debug, Clone)]
pub struct Cluster {
    pub header: Header,
    pub types: Vec<String>,
    pub classes: Vec<Class>,
    pub lists: Vec<InstanceList>,
    pub user_fixups: Vec<UserFixup>,
    /// File offset of the packed fixups: pointer array fixups, then pointer
    /// fixups, then array fixups.
    pub fixups_offset: usize,
}

fn u32_at(b: &[u8], o: usize) -> Result<u32> {
    b.get(o..o + 4)
        .map(|s| u32::from_le_bytes(s.try_into().expect("4 bytes")))
        .ok_or(Error::Malformed("truncated cluster"))
}

impl Cluster {
    /// Parses the header, namespace, instance lists and user fixups.
    ///
    /// # Errors
    /// Fails on truncated or inconsistent tables.
    #[allow(clippy::too_many_lines)] // one pass over the cluster's tables
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let header = Header::parse(bytes)?;
        let ns = HEADER_LEN;
        let w = |o: usize| u32_at(bytes, ns + o);
        if w(0)? != 0x0102_0304 {
            return Err(Error::Malformed("namespace marker"));
        }
        let (n_types, n_classes, n_members, strings_size) = (
            w(8)? as usize,
            w(12)? as usize,
            w(16)? as usize,
            w(20)? as usize,
        );
        let ns_size = header.namespace_size as usize;
        let strings_at = ns
            + ns_size
                .checked_sub(strings_size)
                .ok_or(Error::Malformed("string table"))?;
        let strings = bytes
            .get(strings_at..ns + ns_size)
            .ok_or(Error::Malformed("string table"))?;
        let name = |off: u32| -> Result<String> {
            let rest = strings
                .get(off as usize..)
                .ok_or(Error::Malformed("name outside the string table"))?;
            let end = rest.iter().position(|&b| b == 0).unwrap_or(rest.len());
            Ok(String::from_utf8_lossy(&rest[..end]).into_owned())
        };
        let types = (0..n_types)
            .map(|i| name(w(0x20 + 4 * i)?))
            .collect::<Result<Vec<_>>>()?;
        let classes_at = 0x20 + 4 * n_types;
        let members_at = classes_at + 36 * n_classes;
        if ns + members_at + 24 * n_members > strings_at {
            return Err(Error::Malformed("namespace tables overlap the strings"));
        }
        let mut classes = Vec::with_capacity(n_classes);
        let mut next_member = 0;
        for k in 0..n_classes {
            let c = classes_at + 36 * k;
            let count = w(c + 12)? as usize;
            let members = (next_member..next_member + count)
                .map(|m| {
                    let at = members_at + 24 * m;
                    Ok(Member {
                        name: name(w(at)?)?,
                        type_index: w(at + 4)?,
                        offset: w(at + 8)?,
                        size: w(at + 12)?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            next_member += count;
            let base = w(c)?;
            classes.push(Class {
                name: name(w(c + 8)?)?,
                base: base.checked_sub(1).map(|b| b as usize),
                size: w(c + 4)? & 0x00ff_ffff,
                members,
            });
        }
        if next_member != n_members {
            return Err(Error::Malformed("member count"));
        }

        let lists_at = ns + ns_size;
        let mut offset = lists_at + 0x24 * header.instance_list_count as usize;
        let mut lists = Vec::new();
        for i in 0..header.instance_list_count as usize {
            let at = lists_at + 0x24 * i;
            let class = u32_at(bytes, at)?
                .checked_sub(1)
                .ok_or(Error::Malformed("instance list class"))? as usize;
            if class >= classes.len() {
                return Err(Error::Malformed("instance list class"));
            }
            let list = InstanceList {
                class,
                count: u32_at(bytes, at + 4)?,
                objects_size: u32_at(bytes, at + 12)?,
                arrays_size: u32_at(bytes, at + 16)?,
                pointer_fixups: u32_at(bytes, at + 0x1c)?,
                offset,
            };
            offset += (list.objects_size + list.arrays_size) as usize;
            lists.push(list);
        }
        if offset - (lists_at + 0x24 * lists.len()) != header.total_data_size as usize {
            return Err(Error::Malformed(
                "instance lists do not add up to the data size",
            ));
        }

        let data_at = offset;
        let records_at = data_at + header.user_fixup_data_size as usize;
        let user_fixups = (0..header.user_fixup_count as usize)
            .map(|i| {
                let r = records_at + 12 * i;
                let (type_index, len, off) = (
                    u32_at(bytes, r)?,
                    u32_at(bytes, r + 4)? as usize,
                    u32_at(bytes, r + 8)? as usize,
                );
                let raw = bytes
                    .get(data_at + off..data_at + off + len)
                    .ok_or(Error::Malformed("user fixup string"))?;
                let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
                Ok(UserFixup {
                    type_index,
                    text: String::from_utf8_lossy(&raw[..end]).into_owned(),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let fixups_offset = records_at
            + 12 * header.user_fixup_count as usize
            + 4 * header.header_class_count as usize
            + 16 * header.header_class_fixup_count as usize;
        Ok(Self {
            header,
            types,
            classes,
            lists,
            user_fixups,
            fixups_offset,
        })
    }

    /// Offset of a member within objects of `class`, searching base classes.
    #[must_use]
    pub fn member_offset(&self, class: &str, member: &str) -> Option<u32> {
        let mut c = self.classes.iter().position(|c| c.name == class)?;
        loop {
            if let Some(m) = self.classes[c].members.iter().find(|m| m.name == member) {
                return Some(m.offset);
            }
            c = self.classes[c].base?;
        }
    }

    /// The first instance list of a class, by name.
    #[must_use]
    pub fn list(&self, class: &str) -> Option<&InstanceList> {
        self.lists
            .iter()
            .find(|l| self.classes[l.class].name == class)
    }

    /// The user fixup strings of one plain type, by type name.
    #[must_use]
    pub fn user_strings(&self, type_name: &str) -> Vec<&str> {
        let Some(t) = self.types.iter().position(|n| n == type_name) else {
            return Vec::new();
        };
        self.user_fixups
            .iter()
            .filter(|f| f.type_index as usize == t)
            .map(|f| f.text.as_str())
            .collect()
    }
}
