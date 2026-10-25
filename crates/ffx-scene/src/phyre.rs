//! A cluster's objects as a graph: fields by class and member name
//! (through the namespace, base classes included), pointers through the
//! pointer fixups and strings and byte arrays through the array fixups
//! (`docs/formats/phyre.md`, "Pointer fixups", "Array fixups").

use ffx_phyre::cluster::Cluster;
use ffx_phyre::fixup::{ArrayFixup, PointerFixup};

/// One object: its list, its index in the list and its file offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Obj {
    pub list: usize,
    pub index: u32,
    pub at: usize,
}

/// A pointer field's target: `count` objects from `first` of `list`
/// (`offset` bytes into the first), or a user fixup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    pub list: usize,
    pub first: u32,
    pub count: u32,
    pub offset: u32,
    pub user: Option<u32>,
}

/// A parsed cluster with its fixups.
pub struct Graph {
    pub c: Cluster,
    pub bytes: Vec<u8>,
    pub pointers: Vec<Vec<PointerFixup>>,
    pub arrays: Vec<Vec<ArrayFixup>>,
}

impl std::fmt::Debug for Graph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Graph({} bytes)", self.bytes.len())
    }
}

impl Graph {
    /// # Errors
    /// When the cluster or its fixups don't parse.
    pub fn parse(bytes: Vec<u8>) -> ffx_phyre::Result<Self> {
        let c = Cluster::parse(&bytes)?;
        let pointers = c.pointer_fixups(&bytes)?;
        let arrays = c.array_fixups(&bytes)?;
        Ok(Self {
            c,
            bytes,
            pointers,
            arrays,
        })
    }

    /// The class name of a list.
    #[must_use]
    pub fn class_of(&self, list: usize) -> &str {
        self.c
            .lists
            .get(list)
            .and_then(|l| self.c.classes.get(l.class))
            .map_or("", |c| c.name.as_str())
    }

    /// Every list of a class.
    pub fn lists_of<'a>(&'a self, class: &'a str) -> impl Iterator<Item = usize> + 'a {
        (0..self.c.lists.len()).filter(move |&l| self.class_of(l) == class)
    }

    /// The objects of every list of a class, in list order.
    #[must_use]
    pub fn objects(&self, class: &str) -> Vec<Obj> {
        self.lists_of(class)
            .flat_map(|l| (0..self.c.lists[l].count).map(move |i| (l, i)))
            .map(|(l, i)| self.obj(l, i))
            .collect()
    }

    #[must_use]
    pub fn obj(&self, list: usize, index: u32) -> Obj {
        Obj {
            list,
            index,
            at: self.c.lists[list].object(index),
        }
    }

    /// The offset of `member` in objects of `class` (base classes too).
    #[must_use]
    pub fn offset(&self, class: &str, member: &str) -> Option<u32> {
        self.c.member_offset(class, member)
    }

    fn is_field(&self, word: u32, class: &str, member: &str, inner: u32) -> bool {
        if word & 0x8000_0000 != 0 {
            return self
                .offset(class, member)
                .is_some_and(|o| o + inner == word & 0x7fff_ffff);
        }
        // A namespace member index names the member itself; it can be a
        // base class's member (the index is global).
        inner == 0
            && self
                .c
                .member_by_index(word)
                .is_some_and(|(_, m)| m.name == member)
    }

    /// The pointer of `o`'s `class.member` (`inner`: bytes into an
    /// embedded member, 4 for a `PArray`'s `m_els`).
    #[must_use]
    pub fn ptr(&self, o: Obj, class: &str, member: &str, inner: u32) -> Option<Target> {
        let f =
            self.pointers.get(o.list)?.iter().find(|f| {
                f.object() == o.index && self.is_field(f.member(), class, member, inner)
            })?;
        Some(Self::target(f))
    }

    /// The pointer at byte `offset` of `o`.
    #[must_use]
    pub fn ptr_at(&self, o: Obj, offset: u32) -> Option<Target> {
        let f = self
            .pointers
            .get(o.list)?
            .iter()
            .find(|f| f.object() == o.index && f.member() == 0x8000_0000 | offset)?;
        Some(Self::target(f))
    }

    fn target(f: &PointerFixup) -> Target {
        Target {
            list: f.target_list() as usize,
            first: f.target(),
            count: f.count(),
            offset: f.target_offset(),
            user: f.user_fixup(),
        }
    }

    /// The objects a pointer field points to (a `PArray`'s elements with
    /// `inner` 4).
    #[must_use]
    pub fn targets(&self, o: Obj, class: &str, member: &str, inner: u32) -> Vec<Obj> {
        self.ptr(o, class, member, inner)
            .filter(|t| t.user.is_none() && t.list < self.c.lists.len())
            .map(|t| {
                (t.first..t.first + t.count.max(1))
                    .map(|i| self.obj(t.list, i))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The single object a pointer field points to.
    #[must_use]
    pub fn target_obj(&self, o: Obj, class: &str, member: &str) -> Option<Obj> {
        let t = self.ptr(o, class, member, 0)?;
        (t.user.is_none() && t.list < self.c.lists.len()).then(|| self.obj(t.list, t.first))
    }

    /// The text of the user fixup a field points to (enums such as
    /// `PSceneRenderPassType`, class names).
    #[must_use]
    pub fn user_text(&self, o: Obj, class: &str, member: &str) -> Option<&str> {
        let u = self.ptr(o, class, member, 0)?.user?;
        self.c.user_fixups.get(u as usize).map(|u| u.text.as_str())
    }

    /// The data of the user fixup a pointer refers to.
    #[must_use]
    pub fn user_data(&self, t: Target) -> Option<&[u8]> {
        self.c
            .user_fixups
            .get(t.user? as usize)
            .map(|u| u.data.as_slice())
    }

    /// The bytes of an array field held in the list's arrays region (a
    /// `PString`'s characters, a `PArray<PUInt8>`'s bytes): from the array
    /// fixup's offset, `len` bytes.
    #[must_use]
    pub fn array_bytes(&self, o: Obj, class: &str, member: &str, inner: u32) -> Option<&[u8]> {
        let f =
            self.arrays.get(o.list)?.iter().find(|f| {
                f.object() == o.index && self.is_field(f.member(), class, member, inner)
            })?;
        let range = self.c.lists[o.list].arrays();
        self.bytes.get(range.start + f.0[3] as usize..range.end)
    }

    /// A NUL-terminated string field.
    #[must_use]
    pub fn string(&self, o: Obj, class: &str, member: &str) -> Option<String> {
        let tail = self.array_bytes(o, class, member, 0)?;
        let end = tail.iter().position(|&b| b == 0).unwrap_or(tail.len());
        Some(String::from_utf8_lossy(&tail[..end]).into_owned())
    }

    /// `len` bytes of a field.
    #[must_use]
    pub fn field(&self, o: Obj, class: &str, member: &str, len: usize) -> Option<&[u8]> {
        let at = o.at + self.offset(class, member)? as usize;
        self.bytes.get(at..at + len)
    }

    #[must_use]
    pub fn u8(&self, o: Obj, class: &str, member: &str) -> Option<u8> {
        self.field(o, class, member, 1).map(|b| b[0])
    }

    #[must_use]
    pub fn u16(&self, o: Obj, class: &str, member: &str) -> Option<u16> {
        self.field(o, class, member, 2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
    }

    #[must_use]
    pub fn u32(&self, o: Obj, class: &str, member: &str) -> Option<u32> {
        self.field(o, class, member, 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// A word at a byte offset of an object.
    #[must_use]
    pub fn word_at(&self, at: usize) -> Option<u32> {
        self.bytes
            .get(at..at + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// The import paths of `PAssetReferenceImport`, by index.
    #[must_use]
    pub fn imports(&self) -> Vec<String> {
        self.objects("PAssetReferenceImport")
            .into_iter()
            .map(|o| {
                self.string(o, "PAssetReferenceImport", "m_id")
                    .unwrap_or_default()
            })
            .collect()
    }

    /// The import a pointer's user fixup names (two bytes, a big-endian
    /// import index).
    #[must_use]
    pub fn import_index(&self, t: Target) -> Option<usize> {
        match self.user_data(t)? {
            [hi, lo] => Some(usize::from(u16::from_be_bytes([*hi, *lo]))),
            _ => None,
        }
    }
}
